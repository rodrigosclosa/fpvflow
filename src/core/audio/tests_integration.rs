// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2026 Rodrigo Sclosa

//! Testes de integração do módulo de áudio com arquivos reais.
//!
//! Diferente dos testes unitários dos outros arquivos, que usam sinais
//! sintéticos em memória, estes escrevem um WAV de verdade em disco e o passam
//! pelo decodificador completo. É o que garante que o caminho
//! arquivo → `AudioTrack` → buffer de export funciona ponta a ponta, incluindo o
//! reconhecimento do formato IEEE float (format tag 3).

use std::io::Write;

use super::decode::decode_file;
use super::export::{build_from_trim_ranges, check_format_compatibility, FormatCompatibility};
use super::SourceFormat;

/// Escreve um WAV 32-bit float (IEEE, format tag 3) num arquivo temporário.
///
/// Devolve o caminho. O conteúdo é uma senoide por canal, com um transiente
/// curto em `click_at_s` — o transiente serve de referência visível quando o
/// recorte é conferido amostra a amostra.
fn write_float_wav(name: &str, sample_rate: u32, channels: u16, duration_s: f32, click_at_s: f32) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(name);

    let frames = (sample_rate as f32 * duration_s) as usize;
    let click_frame = (sample_rate as f32 * click_at_s) as usize;

    let mut samples: Vec<f32> = Vec::with_capacity(frames * channels as usize);
    for i in 0..frames {
        let t = i as f32 / sample_rate as f32;
        for ch in 0..channels {
            let freq = 440.0 + 220.0 * ch as f32;
            let mut value = 0.5 * (2.0 * std::f32::consts::PI * freq * t).sin();
            if i >= click_frame && i - click_frame < 200 {
                value += 0.9;
            }
            samples.push(value);
        }
    }

    let bits: u16 = 32;
    let block_align = channels * bits / 8;
    let byte_rate = sample_rate * block_align as u32;
    let data_len = (samples.len() * 4) as u32;

    let mut file = std::fs::File::create(&path).expect("criar WAV de teste");
    file.write_all(b"RIFF").unwrap();
    file.write_all(&(36 + data_len).to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    // 3 = WAVE_FORMAT_IEEE_FLOAT. É este campo que marca o arquivo como float.
    file.write_all(&3u16.to_le_bytes()).unwrap();
    file.write_all(&channels.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    file.write_all(&byte_rate.to_le_bytes()).unwrap();
    file.write_all(&block_align.to_le_bytes()).unwrap();
    file.write_all(&bits.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_len.to_le_bytes()).unwrap();
    for s in &samples {
        file.write_all(&s.to_le_bytes()).unwrap();
    }

    path
}

/// Converte um caminho local para a URL que o `filesystem` do core espera.
fn as_url(path: &std::path::Path) -> String {
    crate::filesystem::path_to_url(&path.to_string_lossy())
}

#[test]
fn decodifica_wav_float_preservando_o_formato_de_origem() {
    let path = write_float_wav("gyroflow_test_f32.wav", 48000, 2, 1.0, 0.5);
    let track = decode_file(&as_url(&path)).expect("decodificar o WAV float");

    // O ponto central da feature: float precisa ser reconhecido como float.
    assert_eq!(track.source_format, SourceFormat::F32);
    assert!(track.source_format.is_float());
    assert_eq!(track.sample_rate, 48000);
    assert_eq!(track.channels, 2, "os canais originais têm que ser preservados");
    assert_eq!(track.frame_count(), 48000);
    assert!((track.duration_seconds() - 1.0).abs() < 0.001);

    // O mono de análise existe, é separado, e tem um valor por frame.
    assert_eq!(track.mono_analysis.len(), 48000);
    assert_ne!(track.mono_analysis.len(), track.samples.len(), "o mono não pode ser o buffer de export");

    // Preservação ligada por default.
    assert!(track.preserve_original_format);

    let _ = std::fs::remove_file(path);
}

/// Regressão: o formato de PCM vem do `CodecType`, não do `sample_format`.
///
/// O symphonia deixa `sample_format` vazio para streams PCM. A primeira versão
/// deste decodificador só olhava esse campo, então um WAV 32-bit float era
/// classificado como `Other` — e teria sido exportado como AAC, silenciosamente,
/// que é exatamente o que a decisão 7 da especificação proíbe.
#[test]
fn wav_float_nao_pode_ser_classificado_como_comprimido() {
    let path = write_float_wav("gyroflow_test_regressao_float.wav", 48000, 2, 0.2, 0.1);
    let track = decode_file(&as_url(&path)).expect("decodificar");

    assert_ne!(
        track.source_format,
        SourceFormat::Other,
        "WAV float classificado como comprimido: seria exportado com perda"
    );
    assert!(track.source_format.is_float());

    // E a consequência prática: o codec recomendado tem que ser float.
    assert_eq!(
        super::export::recommended_codec(track.source_format),
        "PCM (f32le)"
    );

    let _ = std::fs::remove_file(path);
}

#[test]
fn decode_de_mono_tambem_funciona() {
    let path = write_float_wav("gyroflow_test_f32_mono.wav", 44100, 1, 0.5, 0.25);
    let track = decode_file(&as_url(&path)).expect("decodificar WAV mono");

    assert_eq!(track.channels, 1);
    assert_eq!(track.sample_rate, 44100);
    assert_eq!(track.source_format, SourceFormat::F32);

    let _ = std::fs::remove_file(path);
}

#[test]
fn arquivo_inexistente_devolve_erro_util() {
    let err = decode_file("file:///caminho/que/nao/existe.wav").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("Não foi possível abrir"), "mensagem inesperada: {msg}");
}

#[test]
fn arquivo_invalido_e_rejeitado() {
    let path = std::env::temp_dir().join("gyroflow_test_lixo.wav");
    std::fs::write(&path, b"isto nao e um arquivo de audio").unwrap();

    assert!(decode_file(&as_url(&path)).is_err());

    let _ = std::fs::remove_file(path);
}

#[test]
fn caminho_completo_do_arquivo_ao_buffer_de_export() {
    // Cenário realista: WAV float de 5 s, vídeo de 3 s, offset de +0,5 s e um
    // corte no meio.
    let path = write_float_wav("gyroflow_test_pipeline.wav", 48000, 2, 5.0, 2.0);
    let mut track = decode_file(&as_url(&path)).expect("decodificar");
    track.offset_seconds = 0.5;

    // Sem corte: o buffer cobre exatamente os 3 s de vídeo.
    let full = build_from_trim_ranges(&track, &[], 3.0);
    assert_eq!(full.len(), (3.0 * 48000.0) as usize * 2, "buffer tem que cobrir o vídeo inteiro");

    // Com corte em [1/3, 2/3] do vídeo de 3 s = 1 s de áudio.
    let trimmed = build_from_trim_ranges(&track, &[(1.0 / 3.0, 2.0 / 3.0)], 3.0);
    assert_eq!(trimmed.len(), 48000 * 2, "corte de 1 s a 48 kHz estéreo");

    // O formato continua sendo float e o codec recomendado, pcm_f32le em MOV.
    let compat = check_format_compatibility(track.source_format, "mov", true);
    assert_eq!(compat, FormatCompatibility::Preserved { codec: "PCM (f32le)" });

    // E o mesmo material em MP4 tem que ser barrado, não convertido.
    let compat_mp4 = check_format_compatibility(track.source_format, "mp4", true);
    assert!(!compat_mp4.can_proceed(), "float em MP4 tem que exigir decisão do usuário");

    let _ = std::fs::remove_file(path);
}

#[test]
fn waveform_de_arquivo_real_tem_picos_coerentes() {
    let path = write_float_wav("gyroflow_test_waveform.wav", 48000, 2, 1.0, 0.5);
    let track = decode_file(&as_url(&path)).expect("decodificar");

    let peaks = track.peaks_for_width(100);
    assert!(peaks.len() <= 100 && !peaks.is_empty());

    // O transiente em t=0,5 s tem que aparecer como um pico maior que a senoide
    // ao redor: é metade do arquivo, então perto do bucket 50.
    let middle = &peaks[45..55];
    let max_middle = middle.iter().map(|(_, max)| *max).fold(f32::MIN, f32::max);
    let max_start = peaks[..10].iter().map(|(_, max)| *max).fold(f32::MIN, f32::max);
    assert!(max_middle > max_start, "o transiente deveria produzir um pico maior: {max_middle} vs {max_start}");

    let _ = std::fs::remove_file(path);
}
