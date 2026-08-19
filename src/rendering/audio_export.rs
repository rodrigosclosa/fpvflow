// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2026 Rodrigo Sclosa

//! Encode e mux da trilha de áudio externa.
//!
//! O restante do pipeline de áudio do Gyroflow ([`super::ffmpeg_audio`])
//! transcodifica streams que **já existem** no arquivo de entrada: o
//! `AudioTranscoder` recebe um `format::stream::Stream` e copia dele o sample
//! rate, o layout de canais e o formato. Aqui não há stream de entrada — o
//! áudio vem de outro arquivo, já decodificado em memória —, então os
//! parâmetros vêm da [`AudioTrack`].
//!
//! Isso **não** exigiu modificar o fork do `ffmpeg-next`: `add_stream`,
//! `avcodec_alloc_context3` e `encoder().audio()` são as mesmas APIs públicas
//! que o `AudioTranscoder` já usa (ver `ffmpeg_audio.rs:17-46`).
//!
//! # Preservação de formato
//!
//! As amostras chegam em `f32` de [`gyroflow_core::audio::export`] e só são
//! convertidas aqui, uma única vez, para o formato que o encoder pede. Quando o
//! codec escolhido é `pcm_f32le` e o encoder aceita `F32`, não há conversão
//! nenhuma: os bits do arquivo original chegam intactos à saída.

use ffmpeg_next::{ codec, encoder, format, frame, Error, Rational };
use ffmpeg_next::channel_layout::ChannelLayout;
use ffmpeg_next::format::context::Output;

use gyroflow_core::audio::AudioTrack;

use super::audio_resampler::AudioResampler;

/// Escreve um buffer de áudio próprio como novo stream do arquivo de saída.
pub struct ExternalAudioEncoder {
    /// Índice do stream de áudio no arquivo de saída.
    ost_index: usize,
    encoder: encoder::Audio,
    /// Conversor de `f32` intercalado para o formato do encoder.
    ///
    /// `None` quando o encoder já aceita `f32` intercalado — o caso do
    /// `pcm_f32le`, em que qualquer conversão seria perda gratuita.
    resampler: Option<AudioResampler>,
    /// Quantas amostras por canal cada frame leva ao encoder.
    frame_size: usize,
    sample_rate: u32,
    channels: u16,
    /// Contador de frames já enviados, para calcular o PTS.
    frames_written: i64,
}

impl ExternalAudioEncoder {
    /// Cria o stream de saída e configura o encoder a partir da trilha.
    ///
    /// `codec_id` deve vir de
    /// [`gyroflow_core::audio::export::recommended_codec`], já validado contra o
    /// container pela checagem de compatibilidade — chegar aqui com um codec que
    /// o container não aceita produziria um arquivo ilegível.
    pub fn new(codec_id: codec::Id, track: &AudioTrack, octx: &mut Output, ost_index: usize) -> Result<Self, Error> {
        let codec = encoder::find(codec_id).ok_or(Error::EncoderNotFound)?.audio()?;
        let global = octx.format().flags().contains(format::flag::Flags::GLOBAL_HEADER);

        let channels = track.channels.max(1) as i32;
        let channel_layout = codec
            .channel_layouts()
            .map_or(ChannelLayout::default(channels), |cls| cls.best(channels));

        let mut output = octx.add_stream(codec)?;
        let ctx = unsafe {
            codec::context::Context::wrap(ffmpeg_next::ffi::avcodec_alloc_context3(codec.as_ptr()), None)
        };
        let mut encoder = ctx.encoder().audio()?;

        if global {
            encoder.set_flags(codec::flag::Flags::GLOBAL_HEADER);
        }

        // Sample rate e canais vêm do arquivo original, sem reamostragem: o
        // requisito é preservar o material como ele foi gravado.
        encoder.set_rate(track.sample_rate as i32);
        encoder.set_channel_layout(channel_layout);

        // Entre os formatos que o codec aceita, escolhe F32 quando disponível —
        // é o que evita conversão para o áudio float.
        let target_format = codec
            .formats()
            .and_then(|mut formats| {
                let all: Vec<_> = formats.by_ref().collect();
                all.iter()
                    .find(|f| matches!(f, format::Sample::F32(_)))
                    .copied()
                    .or_else(|| all.first().copied())
            })
            .ok_or(Error::EncoderNotFound)?;
        encoder.set_format(target_format);

        encoder.set_time_base((1, track.sample_rate as i32));
        output.set_time_base((1, track.sample_rate as i32));

        let encoder = encoder.open_as(codec)?;
        output.set_parameters(&encoder);

        // PCM aceita qualquer tamanho de frame e informa 0; nesse caso usamos um
        // bloco fixo para não emitir milhares de pacotes minúsculos.
        let frame_size = if encoder.frame_size() > 0 { encoder.frame_size() as usize } else { 1024 };

        // O resampler só entra quando o encoder não aceita f32 intercalado.
        let source_format = format::Sample::F32(format::sample::Type::Packed);
        let resampler = if encoder.format() == source_format && encoder.rate() == track.sample_rate {
            None
        } else {
            Some(AudioResampler::new(
                (source_format, channel_layout, track.sample_rate),
                (encoder.format(), encoder.channel_layout(), encoder.rate()),
                frame_size,
            )?)
        };

        Ok(Self {
            ost_index,
            encoder,
            resampler,
            frame_size,
            sample_rate: track.sample_rate,
            channels: track.channels.max(1),
            frames_written: 0,
        })
    }

    /// Índice do stream de áudio no arquivo de saída.
    ///
    /// Usado para consultar a time base do stream no momento da escrita: o vetor
    /// `ost_time_bases` do processador é dimensionado pelos streams de entrada e
    /// não comporta este índice, que não corresponde a nenhum.
    pub fn stream_index(&self) -> usize {
        self.ost_index
    }

    /// Codifica todo o buffer e escreve os pacotes no arquivo de saída.
    ///
    /// `samples` são amostras `f32` intercaladas, já recortadas e alinhadas por
    /// [`gyroflow_core::audio::export::build_segment`].
    pub fn write_all(&mut self, samples: &[f32], octx: &mut Output, ost_time_base: Rational) -> Result<(), Error> {
        let channels = self.channels as usize;
        if channels == 0 || samples.is_empty() {
            return Ok(());
        }

        let total_frames = samples.len() / channels;
        let mut pos = 0usize;

        while pos < total_frames {
            let this_frame = (total_frames - pos).min(self.frame_size);

            let mut input = frame::Audio::new(
                format::Sample::F32(format::sample::Type::Packed),
                this_frame,
                self.encoder.channel_layout(),
            );
            input.set_rate(self.sample_rate);
            input.set_pts(Some(self.frames_written));

            // `frame::Audio` com formato Packed guarda todos os canais no plano
            // 0, intercalados. Mas `plane_mut::<f32>(0)` devolve uma fatia
            // dimensionada em *frames*, não em amostras — em estéreo ela tem
            // metade dos valores que precisamos escrever. Por isso o acesso é
            // feito pelo buffer de dados bruto.
            let src = &samples[pos * channels..(pos + this_frame) * channels];
            let dst = input.data_mut(0);
            let byte_len = std::mem::size_of_val(src);
            debug_assert!(dst.len() >= byte_len, "plano menor que o esperado: {} < {byte_len}", dst.len());
            dst[..byte_len].copy_from_slice(unsafe {
                // `f32` não tem padding nem invariantes: reinterpretar como
                // bytes é seguro, e é o que o ffmpeg espera receber.
                std::slice::from_raw_parts(src.as_ptr() as *const u8, byte_len)
            });

            match &mut self.resampler {
                Some(resampler) => {
                    resampler.new_frame(&mut input)?;
                    while let Some(out_frame) = resampler.run() {
                        self.encoder.send_frame(out_frame)?;
                    }
                }
                // Caminho sem perda: o buffer vai direto ao encoder.
                None => {
                    self.encoder.send_frame(&input)?;
                }
            }

            self.receive_packets(octx, ost_time_base)?;

            self.frames_written += this_frame as i64;
            pos += this_frame;
        }

        Ok(())
    }

    /// Esvazia o encoder ao fim da exportação.
    pub fn finish(&mut self, octx: &mut Output, ost_time_base: Rational) -> Result<(), Error> {
        // Drena o que ficou no resampler antes de fechar o encoder.
        if let Some(resampler) = &mut self.resampler {
            if let Some(out_frame) = resampler.flush() {
                self.encoder.send_frame(out_frame)?;
            }
        }
        self.encoder.send_eof()?;
        self.receive_packets(octx, ost_time_base)
    }

    /// Drena os pacotes prontos e os escreve com o timestamp correto.
    fn receive_packets(&mut self, octx: &mut Output, ost_time_base: Rational) -> Result<(), Error> {
        let mut packet = ffmpeg_next::Packet::empty();
        while self.encoder.receive_packet(&mut packet).is_ok() {
            packet.set_stream(self.ost_index);
            packet.rescale_ts(self.encoder.time_base(), ost_time_base);
            packet.write_interleaved(octx)?;
        }
        Ok(())
    }
}
