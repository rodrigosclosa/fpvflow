// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2026 Rodrigo Sclosa

//! Decodificação do arquivo de áudio externo.
//!
//! # Por que symphonia e não o ffmpeg do projeto
//!
//! O Gyroflow já linka ffmpeg, mas apenas no crate raiz: `gyroflow-core` não tem
//! `ffmpeg-next` entre suas dependências e a intenção do projeto é que continue
//! assim — o core é lógica pura, compilável e testável sem a toolchain de vídeo.
//! Como a especificação pede o decode dentro de `src/core/audio/`, usar ffmpeg
//! obrigaria a arrastar essa dependência inteira para o core.
//!
//! `symphonia` resolve o caso sem esse custo: é Rust puro, já estava no
//! `Cargo.lock` (transitivamente, via rodio) e cobre WAV/PCM incluindo **IEEE
//! float 32-bit**, que é o formato do DJI Mic e o requisito central desta
//! feature. As features habilitadas cobrem ainda AAC/MP4, MP3 e FLAC, de modo que
//! o usuário possa importar também o `.m4a` que alguns gravadores produzem.

use symphonia::core::audio::{AudioBufferRef, SampleBuffer};
use symphonia::core::codecs::{CodecType, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::sample::SampleFormat;

use super::{AudioTrack, SourceFormat};

/// Adapta o [`crate::filesystem::FileWrapper`] à interface que o symphonia espera.
///
/// O `FileWrapper` já oferece `Read + Seek` e conhece o tamanho do arquivo; falta
/// apenas declarar essas capacidades no formato do trait `MediaSource`. Passar
/// pelo wrapper (em vez de abrir um `std::fs::File`) é o que faz a importação
/// funcionar no Android, onde os arquivos chegam como `content://`.
struct FileWrapperSource(crate::filesystem::FileWrapper);

impl std::io::Read for FileWrapperSource {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        std::io::Read::read(&mut self.0, buf)
    }
}
impl std::io::Seek for FileWrapperSource {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        std::io::Seek::seek(&mut self.0, pos)
    }
}
impl MediaSource for FileWrapperSource {
    fn is_seekable(&self) -> bool {
        true
    }
    fn byte_len(&self) -> Option<u64> {
        Some(self.0.size as u64)
    }
}

/// Falhas na importação de um arquivo de áudio.
///
/// As mensagens são escritas para serem exibidas ao usuário sem tradução extra.
#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("Não foi possível abrir o arquivo de áudio: {0}")]
    Open(String),

    #[error("Formato de áudio não reconhecido ou arquivo corrompido: {0}")]
    UnsupportedFormat(String),

    #[error("O arquivo não contém nenhuma trilha de áudio decodificável")]
    NoAudioTrack,

    #[error("O arquivo não informa o sample rate da trilha de áudio")]
    MissingSampleRate,

    #[error("Falha ao decodificar o áudio: {0}")]
    Decode(String),

    #[error("A trilha de áudio está vazia")]
    Empty,
}

/// Traduz o formato de origem a partir do **tipo de codec**.
///
/// Para PCM, o symphonia não preenche `sample_format` nos parâmetros do stream —
/// a informação está no `CodecType`. Ignorar isso faria um WAV 32-bit float ser
/// classificado como `Other` e exportado com perda, violando a decisão 7 da
/// especificação. Este foi um bug real, pego pelos testes com arquivo de
/// verdade.
fn map_codec_type(codec: CodecType) -> Option<SourceFormat> {
    use symphonia::core::codecs::*;
    match codec {
        CODEC_TYPE_PCM_F32LE | CODEC_TYPE_PCM_F32BE | CODEC_TYPE_PCM_F64LE | CODEC_TYPE_PCM_F64BE
        | CODEC_TYPE_PCM_F32LE_PLANAR | CODEC_TYPE_PCM_F32BE_PLANAR
        | CODEC_TYPE_PCM_F64LE_PLANAR | CODEC_TYPE_PCM_F64BE_PLANAR => Some(SourceFormat::F32),

        CODEC_TYPE_PCM_S32LE | CODEC_TYPE_PCM_S32BE | CODEC_TYPE_PCM_U32LE | CODEC_TYPE_PCM_U32BE
        | CODEC_TYPE_PCM_S32LE_PLANAR | CODEC_TYPE_PCM_S32BE_PLANAR
        | CODEC_TYPE_PCM_U32LE_PLANAR | CODEC_TYPE_PCM_U32BE_PLANAR => Some(SourceFormat::S32),

        CODEC_TYPE_PCM_S24LE | CODEC_TYPE_PCM_S24BE | CODEC_TYPE_PCM_U24LE | CODEC_TYPE_PCM_U24BE
        | CODEC_TYPE_PCM_S24LE_PLANAR | CODEC_TYPE_PCM_S24BE_PLANAR
        | CODEC_TYPE_PCM_U24LE_PLANAR | CODEC_TYPE_PCM_U24BE_PLANAR => Some(SourceFormat::S24),

        CODEC_TYPE_PCM_S16LE | CODEC_TYPE_PCM_S16BE | CODEC_TYPE_PCM_U16LE | CODEC_TYPE_PCM_U16BE
        | CODEC_TYPE_PCM_S16LE_PLANAR | CODEC_TYPE_PCM_S16BE_PLANAR
        | CODEC_TYPE_PCM_U16LE_PLANAR | CODEC_TYPE_PCM_U16BE_PLANAR => Some(SourceFormat::S16),

        CODEC_TYPE_PCM_S8 | CODEC_TYPE_PCM_U8
        | CODEC_TYPE_PCM_S8_PLANAR | CODEC_TYPE_PCM_U8_PLANAR => Some(SourceFormat::U8),

        // Não é PCM: provavelmente um codec com perda, tratado pelo chamador.
        _ => None,
    }
}

/// Traduz o formato relatado pelo symphonia para o enum que guardamos.
///
/// `bits_per_sample` é consultado porque um WAV de 24 bits normalmente é
/// entregue como `S32` com `bits_per_sample = 24`: preservar essa distinção evita
/// exportar como 32 bits um material que nasceu com 24.
fn map_source_format(sample_format: Option<SampleFormat>, bits_per_sample: Option<u32>) -> SourceFormat {
    match sample_format {
        Some(SampleFormat::F32) | Some(SampleFormat::F64) => SourceFormat::F32,
        Some(SampleFormat::S32) | Some(SampleFormat::U32) => match bits_per_sample {
            Some(24) => SourceFormat::S24,
            _ => SourceFormat::S32,
        },
        Some(SampleFormat::S24) | Some(SampleFormat::U24) => SourceFormat::S24,
        Some(SampleFormat::S16) | Some(SampleFormat::U16) => SourceFormat::S16,
        Some(SampleFormat::S8) | Some(SampleFormat::U8) => SourceFormat::U8,
        // Codecs com perda (AAC, MP3) não expõem sample_format: a perda já
        // aconteceu antes de chegarmos ao arquivo, então não há formato de
        // origem a preservar bit a bit.
        None => SourceFormat::Other,
    }
}

/// Gera o downmix mono usado **apenas** pela análise de sincronismo.
///
/// A média simples entre canais basta aqui: o auto-sync correlaciona envelopes de
/// energia, que são insensíveis a fase e a ganho absoluto.
///
/// Este buffer nunca deve ser exportado — ver a documentação do módulo.
fn downmix_to_mono(interleaved: &[f32], channels: u16) -> Vec<f32> {
    if channels <= 1 {
        return interleaved.to_vec();
    }
    let channels = channels as usize;
    let frames = interleaved.len() / channels;
    let mut mono = Vec::with_capacity(frames);
    for frame in 0..frames {
        let base = frame * channels;
        let sum: f32 = interleaved[base..base + channels].iter().sum();
        mono.push(sum / channels as f32);
    }
    mono
}

/// Decodifica um arquivo de áudio para uma [`AudioTrack`].
///
/// `url` é uma URL no formato do [`crate::filesystem`] — não um caminho cru —,
/// para que a importação funcione também no Android, onde o acesso se dá por
/// `content://` e não pelo sistema de arquivos.
///
/// As amostras são mantidas em `f32` intercalado, preservando todos os canais
/// originais, e o formato de origem é registrado para que a exportação
/// reconstrua o arquivo sem perda.
pub fn decode_file(url: &str) -> Result<AudioTrack, DecodeError> {
    // `FileWrapper` implementa Read + Seek e cuida do ciclo de vida do
    // descritor no Android (ver filesystem/mod.rs:119-144).
    let file = crate::filesystem::open_file(url, false, false)
        .map_err(|e| DecodeError::Open(e.to_string()))?;
    let stream = MediaSourceStream::new(Box::new(FileWrapperSource(file)), Default::default());

    // A extensão é só uma dica; o symphonia confirma pelo conteúdo real.
    let filename = crate::filesystem::get_filename(url);
    let mut hint = Hint::new();
    if let Some(ext) = std::path::Path::new(&filename).extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, stream, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| DecodeError::UnsupportedFormat(e.to_string()))?;

    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or(DecodeError::NoAudioTrack)?;

    let track_id = track.id;
    let params = track.codec_params.clone();

    let sample_rate = params.sample_rate.ok_or(DecodeError::MissingSampleRate)?;

    // Para PCM o formato vem do tipo de codec;  costuma estar
    // vazio. Só quando o codec não é PCM (AAC, MP3...) caímos no sample_format.
    let source_format = map_codec_type(params.codec)
        .unwrap_or_else(|| map_source_format(params.sample_format, params.bits_per_sample));

    let mut decoder = symphonia::default::get_codecs()
        .make(&params, &DecoderOptions::default())
        .map_err(|e| DecodeError::UnsupportedFormat(e.to_string()))?;

    // Reserva o buffer com base em `n_frames` quando o container informa a
    // duração, evitando dezenas de realocações num arquivo longo.
    let mut samples: Vec<f32> = Vec::with_capacity(
        params.n_frames.unwrap_or(0).saturating_mul(params.channels.map_or(2, |c| c.count() as u64)) as usize,
    );
    let mut channels: u16 = params.channels.map_or(0, |c| c.count() as u16);
    let mut sample_buf: Option<SampleBuffer<f32>> = None;

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            // Fim do stream: `ResetRequired` e o IO `UnexpectedEof` são as duas
            // formas normais de terminar a leitura.
            Err(SymphoniaError::ResetRequired) => break,
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(DecodeError::Decode(e.to_string())),
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                append_samples(&decoded, &mut sample_buf, &mut samples, &mut channels);
            }
            // Pacotes corrompidos isolados não invalidam o arquivo inteiro;
            // o symphonia recomenda seguir em frente nestes dois casos.
            Err(SymphoniaError::DecodeError(_)) | Err(SymphoniaError::IoError(_)) => continue,
            Err(e) => return Err(DecodeError::Decode(e.to_string())),
        }
    }

    if samples.is_empty() || channels == 0 {
        return Err(DecodeError::Empty);
    }

    let mono_analysis = downmix_to_mono(&samples, channels);

    Ok(AudioTrack {
        path: url.to_string(),
        samples,
        channels,
        sample_rate,
        source_format,
        mono_analysis,
        offset_seconds: 0.0,
        preserve_original_format: true,
    })
}

/// Converte um buffer decodificado para `f32` intercalado e o acumula.
///
/// `SampleBuffer::copy_interleaved_ref` trata todas as variantes de
/// [`AudioBufferRef`], então não precisamos de um match de dez braços — e a
/// conversão para `f32` é a mesma que usaríamos manualmente.
fn append_samples(
    decoded: &AudioBufferRef,
    sample_buf: &mut Option<SampleBuffer<f32>>,
    out: &mut Vec<f32>,
    channels: &mut u16,
) {
    let spec = *decoded.spec();
    if *channels == 0 {
        *channels = spec.channels.count() as u16;
    }

    // O buffer é recriado quando a capacidade não basta: alguns containers
    // entregam o primeiro pacote menor que os seguintes.
    let needs_new = match sample_buf {
        Some(buf) => buf.capacity() < decoded.capacity(),
        None => true,
    };
    if needs_new {
        *sample_buf = Some(SampleBuffer::<f32>::new(decoded.capacity() as u64, spec));
    }

    if let Some(buf) = sample_buf {
        buf.copy_interleaved_ref(decoded.clone());
        out.extend_from_slice(buf.samples());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downmix_media_os_canais() {
        // Dois canais intercalados: [L=1.0, R=0.0] e [L=0.0, R=1.0].
        let stereo = vec![1.0, 0.0, 0.0, 1.0];
        let mono = downmix_to_mono(&stereo, 2);
        assert_eq!(mono, vec![0.5, 0.5]);
    }

    #[test]
    fn downmix_de_mono_e_identidade() {
        let mono_in = vec![0.1, -0.2, 0.3];
        assert_eq!(downmix_to_mono(&mono_in, 1), mono_in);
    }

    #[test]
    fn float_e_reconhecido_como_float() {
        assert!(map_source_format(Some(SampleFormat::F32), Some(32)).is_float());
        assert!(map_source_format(Some(SampleFormat::F64), Some(64)).is_float());
    }

    #[test]
    fn wav_24_bits_nao_vira_32() {
        // WAV de 24 bits costuma ser entregue como S32 com bits_per_sample=24.
        assert_eq!(map_source_format(Some(SampleFormat::S32), Some(24)), SourceFormat::S24);
        assert_eq!(map_source_format(Some(SampleFormat::S32), Some(32)), SourceFormat::S32);
    }

    #[test]
    fn codec_com_perda_nao_tem_formato_a_preservar() {
        assert_eq!(map_source_format(None, None), SourceFormat::Other);
        assert!(!map_source_format(None, None).is_float());
    }
}
