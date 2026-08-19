// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2026 Rodrigo Sclosa

//! Testes do encoder de áudio externo com arquivos reais.
//!
//! Diferente dos testes do core, que são lógica pura, estes exercitam o ffmpeg
//! de verdade: criam um arquivo de saída, escrevem um stream de áudio e
//! verificam o resultado relendo o arquivo. É o que prova que o mux funciona sem
//! nenhuma modificação no fork.

use ffmpeg_next::codec;
use gyroflow_core::audio::{AudioTrack, SourceFormat};

use super::audio_export::ExternalAudioEncoder;

/// Trilha sintética estéreo em 48 kHz.
fn make_track(seconds: f32, sample_rate: u32, channels: u16) -> AudioTrack {
    let frames = (sample_rate as f32 * seconds) as usize;
    let mut samples = Vec::with_capacity(frames * channels as usize);
    for i in 0..frames {
        let t = i as f32 / sample_rate as f32;
        for ch in 0..channels {
            let freq = 440.0 + 220.0 * ch as f32;
            samples.push(0.5 * (2.0 * std::f32::consts::PI * freq * t).sin());
        }
    }
    AudioTrack {
        path: String::new(),
        samples,
        channels,
        sample_rate,
        source_format: SourceFormat::F32,
        mono_analysis: Vec::new(),
        offset_seconds: 0.0,
        preserve_original_format: true,
    }
}

/// Escreve a trilha num arquivo e devolve o caminho.
fn write_audio_file(name: &str, track: &AudioTrack, codec_id: codec::Id) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(name);
    let path_str = path.to_string_lossy().to_string();

    let mut octx = ffmpeg_next::format::output(&path_str).expect("criar arquivo de saída");

    let mut encoder = ExternalAudioEncoder::new(codec_id, track, &mut octx, 0).expect("criar encoder");

    octx.write_header().expect("escrever header");

    let time_base = octx.stream(0).map(|s| s.time_base()).expect("time base do stream");

    encoder.write_all(&track.samples, &mut octx, time_base).expect("escrever amostras");
    encoder.finish(&mut octx, time_base).expect("finalizar");

    octx.write_trailer().expect("escrever trailer");

    path
}

/// Lê de volta os parâmetros do stream de áudio do arquivo gerado.
fn probe_audio(path: &std::path::Path) -> (codec::Id, u32, u16, f64) {
    let ictx = ffmpeg_next::format::input(&path.to_string_lossy().to_string()).expect("abrir arquivo gerado");
    let stream = ictx
        .streams()
        .best(ffmpeg_next::media::Type::Audio)
        .expect("o arquivo deveria ter um stream de áudio");

    let params = stream.parameters();
    let ctx = codec::context::Context::from_parameters(params).expect("parâmetros");
    let audio = ctx.decoder().audio().expect("decoder de áudio");

    let duration_s = stream.duration() as f64 * f64::from(stream.time_base());

    (audio.codec().map(|c| c.id()).unwrap_or(codec::Id::None), audio.rate(), audio.channels(), duration_s)
}

#[test]
fn escreve_pcm_f32le_preservando_taxa_e_canais() {
    let track = make_track(1.0, 48000, 2);
    let path = write_audio_file("gyroflow_test_out_f32.mov", &track, codec::Id::PCM_F32LE);

    assert!(path.exists(), "o arquivo de saída não foi criado");
    let size = std::fs::metadata(&path).unwrap().len();
    // 1 s estéreo em float são ~384 KB de dados; com folga para o container.
    assert!(size > 300_000, "arquivo pequeno demais ({size} bytes): o áudio não foi escrito");

    let (codec_id, rate, channels, duration) = probe_audio(&path);

    // O ponto central: float entra, float sai, sem conversão.
    assert_eq!(codec_id, codec::Id::PCM_F32LE, "o codec de saída não é pcm_f32le");
    assert_eq!(rate, 48000, "o sample rate original não foi preservado");
    assert_eq!(channels, 2, "os canais originais não foram preservados");
    assert!((duration - 1.0).abs() < 0.05, "duração inesperada: {duration}s");

    let _ = std::fs::remove_file(path);
}

#[test]
fn escreve_em_44100_hz_sem_reamostrar() {
    // Sample rates diferentes de 48 kHz também têm que passar intactos.
    let track = make_track(0.5, 44100, 2);
    let path = write_audio_file("gyroflow_test_out_44k.mov", &track, codec::Id::PCM_F32LE);

    let (codec_id, rate, channels, _) = probe_audio(&path);
    assert_eq!(codec_id, codec::Id::PCM_F32LE);
    assert_eq!(rate, 44100, "o sample rate foi alterado");
    assert_eq!(channels, 2);

    let _ = std::fs::remove_file(path);
}

#[test]
fn escreve_mono() {
    let track = make_track(0.5, 48000, 1);
    let path = write_audio_file("gyroflow_test_out_mono.mov", &track, codec::Id::PCM_F32LE);

    let (_, rate, channels, _) = probe_audio(&path);
    assert_eq!(rate, 48000);
    assert_eq!(channels, 1, "mono virou outra coisa");

    let _ = std::fs::remove_file(path);
}

/// Drena os frames decodificados, convertendo para `f32` intercalado.
///
/// `frame.plane::<f32>(0)` é dimensionado em **frames**, não em amostras: em
/// estéreo ele devolve metade dos valores. Por isso os dados são lidos do buffer
/// bruto, como o encoder também faz.
fn collect_frames(decoder: &mut ffmpeg_next::decoder::Audio, out: &mut Vec<f32>) {
    let mut frame = ffmpeg_next::frame::Audio::empty();
    while decoder.receive_frame(&mut frame).is_ok() {
        let values = frame.samples() * frame.channels() as usize;
        let bytes = frame.data(0);
        let floats = unsafe { std::slice::from_raw_parts(bytes.as_ptr() as *const f32, values.min(bytes.len() / 4)) };
        out.extend_from_slice(floats);
    }
}

#[test]
fn conteudo_do_audio_sobrevive_ao_encode() {
    // Escreve uma trilha e a relê amostra a amostra: com pcm_f32le o caminho é
    // sem perda, então os valores têm que voltar praticamente idênticos.
    let track = make_track(0.2, 48000, 2);
    let path = write_audio_file("gyroflow_test_roundtrip.mov", &track, codec::Id::PCM_F32LE);

    let mut ictx = ffmpeg_next::format::input(&path.to_string_lossy().to_string()).expect("abrir");
    let stream_index = ictx
        .streams()
        .best(ffmpeg_next::media::Type::Audio)
        .expect("stream de áudio")
        .index();

    let params = ictx.stream(stream_index).unwrap().parameters();
    let ctx = codec::context::Context::from_parameters(params).unwrap();
    let mut decoder = ctx.decoder().audio().unwrap();

    let mut decoded: Vec<f32> = Vec::new();
    let packets: Vec<_> = ictx.packets().filter_map(|(s, p)| (s.index() == stream_index).then_some(p)).collect();
    for packet in packets {
        decoder.send_packet(&packet).unwrap();
        collect_frames(&mut decoder, &mut decoded);
    }
    decoder.send_eof().unwrap();
    collect_frames(&mut decoder, &mut decoded);

    assert!(!decoded.is_empty(), "nada foi decodificado de volta");

    // Compara o começo: se houvesse conversão para inteiro, o erro seria bem
    // maior que isto.
    let compare = decoded.len().min(track.samples.len()).min(4800);
    let max_error = (0..compare)
        .map(|i| (decoded[i] - track.samples[i]).abs())
        .fold(0.0f32, f32::max);
    assert!(max_error < 1e-6, "o áudio foi alterado no caminho: erro máximo {max_error}");

    let _ = std::fs::remove_file(path);
}
