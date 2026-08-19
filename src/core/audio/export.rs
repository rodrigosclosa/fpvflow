// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2026 Rodrigo Sclosa

//! Montagem do buffer de áudio que será embutido no vídeo exportado.
//!
//! Este arquivo contém **apenas lógica pura**: recorte, deslocamento e
//! preenchimento com silêncio, tudo em `f32` e sem nenhuma dependência de
//! ffmpeg. O encode e o mux vivem em `src/rendering/audio_export.rs`, no crate
//! raiz, porque `gyroflow-core` não depende de ffmpeg e não deve passar a
//! depender.
//!
//! # Regra central
//!
//! `t_audio = t_video + offset`. Um trecho de vídeo `[t_in, t_out]` corresponde
//! ao trecho de áudio `[t_in + offset, t_out + offset]`.
//!
//! O buffer devolvido **sempre** cobre exatamente a duração do trecho de vídeo
//! pedido. Onde o áudio não alcança — antes do início ou depois do fim da
//! trilha —, o espaço é preenchido com silêncio. Devolver um buffer mais curto
//! desalinharia o mux.
//!
//! # Preservação de formato
//!
//! Nada aqui quantiza: as amostras seguem em `f32` do decode até o encode, que
//! é o único ponto onde a conversão para o formato de saída acontece. Ver
//! [`SourceFormat`](super::SourceFormat) e [`recommended_codec`].

use super::{AudioTrack, SourceFormat};

/// Codec de saída recomendado para preservar um formato de origem.
///
/// Os nomes são exatamente os aceitos pelo seletor de codec em
/// `src/rendering/mod.rs:250-258`.
pub fn recommended_codec(format: SourceFormat) -> &'static str {
    match format {
        // Float precisa de float: converter para inteiro perde a faixa acima de
        // 0 dBFS que o material de 32-bit float carrega, e é justamente por isso
        // que gravadores como o DJI Mic usam esse formato.
        SourceFormat::F32 => "PCM (f32le)",
        SourceFormat::S32 | SourceFormat::S24 => "PCM (s24le)",
        SourceFormat::S16 => "PCM (s16le)",
        SourceFormat::U8 => "PCM (s16le)",
        // A origem já é comprimida com perda; não há o que preservar bit a bit.
        SourceFormat::Other => "AAC",
    }
}

/// Se um container aceita o codec de áudio indicado.
///
/// A checagem existe para impedir a conversão silenciosa proibida pela decisão 7
/// da especificação: quando float não cabe no container escolhido, o usuário
/// precisa ser avisado e decidir, em vez de receber um arquivo rebaixado.
///
/// `extension` é comparada sem o ponto e sem diferenciar maiúsculas.
pub fn container_supports_codec(extension: &str, codec: &str) -> bool {
    let ext = extension.trim_start_matches('.').to_ascii_lowercase();
    match codec {
        // PCM float existe em MOV e MKV. MP4 não o suporta na prática: o padrão
        // não define mapeamento para pcm_f32le e os players não o reproduzem.
        "PCM (f32le)" => matches!(ext.as_str(), "mov" | "mkv"),
        // PCM inteiro é aceito em MOV/MKV; em MP4 é possível mas raro e mal
        // suportado, então tratamos como incompatível para não gerar arquivos
        // que o usuário não consegue abrir.
        "PCM (s16le)" | "PCM (s16be)" | "PCM (s24le)" | "PCM (s24be)" => {
            matches!(ext.as_str(), "mov" | "mkv" | "wav")
        }
        // AAC cabe em qualquer um dos containers que o Gyroflow exporta.
        "AAC" => matches!(ext.as_str(), "mp4" | "mov" | "mkv" | "m4a"),
        _ => false,
    }
}

/// Resultado da checagem de compatibilidade entre origem, codec e container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatCompatibility {
    /// O formato de origem será preservado sem perda.
    Preserved {
        /// Codec que será usado na saída.
        codec: &'static str,
    },
    /// O container escolhido não comporta o formato de origem.
    ///
    /// **Não converta silenciosamente.** Apresente as opções ao usuário: trocar
    /// o container ou aceitar explicitamente a perda.
    ContainerMismatch {
        /// Codec que preservaria o material.
        wanted_codec: &'static str,
        /// Extensão escolhida para o vídeo, que não o comporta.
        extension: String,
        /// Extensão sugerida como alternativa.
        suggested_extension: &'static str,
    },
    /// O usuário desligou a preservação de forma explícita.
    DowngradeAccepted {
        /// Codec padrão do container.
        codec: &'static str,
    },
}

impl FormatCompatibility {
    /// Se a exportação pode prosseguir sem intervenção do usuário.
    pub fn can_proceed(&self) -> bool {
        !matches!(self, Self::ContainerMismatch { .. })
    }
}

/// Decide o codec de saída, ou detecta que o container não comporta o material.
///
/// `extension` é a extensão do arquivo de saída escolhida para o **vídeo** — é
/// ela que manda, já que o áudio é embutido nele.
///
/// A sugestão de alternativa é sempre `.mov`: é o container já usado por
/// ProRes, DNxHD e CineForm no seletor de exportação, e é o que a própria
/// interface do Gyroflow recomenda quando codec e container não batem
/// (`src/ui/App.qml:729`).
pub fn check_format_compatibility(
    source_format: SourceFormat,
    extension: &str,
    preserve_original_format: bool,
) -> FormatCompatibility {
    if !preserve_original_format {
        return FormatCompatibility::DowngradeAccepted { codec: "AAC" };
    }

    let wanted = recommended_codec(source_format);
    if container_supports_codec(extension, wanted) {
        FormatCompatibility::Preserved { codec: wanted }
    } else {
        FormatCompatibility::ContainerMismatch {
            wanted_codec: wanted,
            extension: extension.trim_start_matches('.').to_ascii_lowercase(),
            suggested_extension: "mov",
        }
    }
}

/// Converte um instante de vídeo para o índice de frame de áudio correspondente.
///
/// O arredondamento acontece **uma única vez**, aqui, sobre o tempo já somado ao
/// offset. Converter em duas etapas (tempo→frame de vídeo→sample) acumularia
/// erro de meio-frame, que a 30 fps chega a 16 ms — audível.
///
/// O resultado pode ser negativo: significa que o trecho de vídeo começa antes
/// do início do áudio, e a diferença vira silêncio.
fn video_time_to_audio_frame(t_video_s: f64, offset_s: f64, sample_rate: u32) -> i64 {
    ((t_video_s + offset_s) * sample_rate as f64).round() as i64
}

/// Monta o buffer de áudio para um trecho de vídeo.
///
/// - `track` é a trilha importada, com os canais originais preservados.
/// - `t_in_s` / `t_out_s` delimitam o trecho **de vídeo**, em segundos.
///
/// Devolve amostras `f32` intercaladas cobrindo exatamente `t_out_s - t_in_s`
/// segundos, com silêncio onde a trilha não alcança.
pub fn build_segment(track: &AudioTrack, t_in_s: f64, t_out_s: f64) -> Vec<f32> {
    let channels = track.channels as usize;
    if channels == 0 || track.sample_rate == 0 || t_out_s <= t_in_s {
        return Vec::new();
    }

    let sample_rate = track.sample_rate;
    let offset = track.offset_seconds;

    // Quantidade de frames que o trecho de vídeo exige. É este número que o
    // buffer terá, independentemente do que a trilha cobre.
    let wanted_frames = (((t_out_s - t_in_s) * sample_rate as f64).round() as i64).max(0) as usize;
    if wanted_frames == 0 {
        return Vec::new();
    }

    let start_frame = video_time_to_audio_frame(t_in_s, offset, sample_rate);
    let available_frames = track.frame_count() as i64;

    let mut out = vec![0.0f32; wanted_frames * channels];

    // Interseção entre o trecho pedido e o que a trilha realmente cobre.
    let copy_from = start_frame.max(0);
    let copy_to = (start_frame + wanted_frames as i64).min(available_frames);
    if copy_to <= copy_from {
        // Nenhuma sobreposição: o trecho inteiro é silêncio.
        return out;
    }

    // Onde, dentro do buffer de saída, o trecho copiado começa. É positivo
    // quando o áudio entra depois do início do vídeo (offset positivo).
    let dest_frame_offset = (copy_from - start_frame).max(0) as usize;

    let src_start = copy_from as usize * channels;
    let src_end = copy_to as usize * channels;
    let dst_start = dest_frame_offset * channels;
    let dst_end = dst_start + (src_end - src_start);

    out[dst_start..dst_end].copy_from_slice(&track.samples[src_start..src_end]);
    out
}

/// Monta o buffer para vários trechos de vídeo, concatenados na ordem dada.
///
/// Usado quando a exportação tem múltiplos intervalos de corte: cada trecho é
/// mapeado independentemente e os buffers são emendados, espelhando o que o
/// vídeo faz.
pub fn build_segments(track: &AudioTrack, ranges: &[(f64, f64)]) -> Vec<f32> {
    if ranges.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (t_in, t_out) in ranges {
        out.extend_from_slice(&build_segment(track, *t_in, *t_out));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trilha sintética: `frames` frames, mono, com valor igual ao índice.
    ///
    /// Assim cada amostra identifica de qual instante veio, e um recorte errado
    /// aparece como valor deslocado em vez de passar despercebido.
    fn track(frames: usize, sample_rate: u32, offset_seconds: f64) -> AudioTrack {
        AudioTrack {
            path: String::new(),
            samples: (0..frames).map(|i| i as f32).collect(),
            channels: 1,
            sample_rate,
            source_format: SourceFormat::F32,
            mono_analysis: Vec::new(),
            offset_seconds,
            preserve_original_format: true,
        }
    }

    #[test]
    fn recorte_sem_offset_pega_o_trecho_certo() {
        // 100 frames a 100 Hz = 1 s. Trecho [0.1, 0.2] → frames 10..20.
        let t = track(100, 100, 0.0);
        let seg = build_segment(&t, 0.1, 0.2);
        assert_eq!(seg.len(), 10);
        assert_eq!(seg[0], 10.0);
        assert_eq!(seg[9], 19.0);
    }

    #[test]
    fn offset_positivo_adianta_o_audio() {
        // offset=+0.1 s: o trecho de vídeo [0.0, 0.1] busca o áudio em
        // [0.1, 0.2] → frames 10..20.
        let t = track(100, 100, 0.1);
        let seg = build_segment(&t, 0.0, 0.1);
        assert_eq!(seg[0], 10.0);
    }

    #[test]
    fn offset_negativo_preenche_com_silencio_no_inicio() {
        // offset=-0.05 s: o vídeo em [0.0, 0.1] pede áudio a partir de -0.05 s,
        // que não existe. Os 5 primeiros frames são silêncio.
        let t = track(100, 100, -0.05);
        let seg = build_segment(&t, 0.0, 0.1);
        assert_eq!(seg.len(), 10);
        assert_eq!(&seg[0..5], &[0.0, 0.0, 0.0, 0.0, 0.0]);
        assert_eq!(seg[5], 0.0); // primeiro frame real da trilha
        assert_eq!(seg[6], 1.0);
    }

    #[test]
    fn trecho_depois_do_fim_vira_silencio() {
        let t = track(100, 100, 0.0);
        // A trilha tem 1 s; pedir [1.5, 1.6] não tem sobreposição alguma.
        let seg = build_segment(&t, 1.5, 1.6);
        assert_eq!(seg.len(), 10);
        assert!(seg.iter().all(|s| *s == 0.0));
    }

    #[test]
    fn buffer_cobre_a_duracao_pedida_mesmo_estourando_a_trilha() {
        let t = track(100, 100, 0.0);
        // Pede 0.5 s começando em 0.8 s: a trilha acaba em 1.0 s.
        let seg = build_segment(&t, 0.8, 1.3);
        // O tamanho é o do trecho de vídeo, não o do que sobrou de áudio.
        assert_eq!(seg.len(), 50);
        assert_eq!(seg[0], 80.0);
        assert_eq!(seg[19], 99.0);
        assert!(seg[20..].iter().all(|s| *s == 0.0));
    }

    #[test]
    fn estereo_preserva_a_intercalacao() {
        let mut t = track(0, 100, 0.0);
        t.channels = 2;
        // [L0,R0, L1,R1, L2,R2] com L=índice e R=índice+100.
        t.samples = vec![0.0, 100.0, 1.0, 101.0, 2.0, 102.0];
        let seg = build_segment(&t, 0.01, 0.03);
        assert_eq!(seg, vec![1.0, 101.0, 2.0, 102.0]);
    }

    #[test]
    fn segmentos_multiplos_sao_concatenados_na_ordem() {
        let t = track(100, 100, 0.0);
        let seg = build_segments(&t, &[(0.0, 0.05), (0.5, 0.55)]);
        assert_eq!(seg.len(), 10);
        assert_eq!(seg[0], 0.0);
        assert_eq!(seg[5], 50.0);
    }

    #[test]
    fn float_exige_codec_float() {
        assert_eq!(recommended_codec(SourceFormat::F32), "PCM (f32le)");
    }

    #[test]
    fn float_nao_cabe_em_mp4() {
        assert!(!container_supports_codec("mp4", "PCM (f32le)"));
        assert!(container_supports_codec("mov", "PCM (f32le)"));
        assert!(container_supports_codec(".MOV", "PCM (f32le)"));
        assert!(container_supports_codec("mkv", "PCM (f32le)"));
    }

    #[test]
    fn conflito_float_mp4_e_sinalizado_e_nao_convertido() {
        let r = check_format_compatibility(SourceFormat::F32, "mp4", true);
        assert!(!r.can_proceed());
        match r {
            FormatCompatibility::ContainerMismatch { wanted_codec, suggested_extension, .. } => {
                assert_eq!(wanted_codec, "PCM (f32le)");
                assert_eq!(suggested_extension, "mov");
            }
            _ => panic!("float em MP4 tem que ser sinalizado como incompatível"),
        }
    }

    #[test]
    fn float_em_mov_e_preservado() {
        let r = check_format_compatibility(SourceFormat::F32, "mov", true);
        assert!(r.can_proceed());
        assert_eq!(r, FormatCompatibility::Preserved { codec: "PCM (f32le)" });
    }

    #[test]
    fn downgrade_explicito_e_respeitado() {
        let r = check_format_compatibility(SourceFormat::F32, "mp4", false);
        assert!(r.can_proceed());
        assert_eq!(r, FormatCompatibility::DowngradeAccepted { codec: "AAC" });
    }
}
