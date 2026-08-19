// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2026 Rodrigo Sclosa

//! Sincronização de áudio externo com o giroscópio.
//!
//! Este módulo importa uma trilha de áudio gravada por um dispositivo separado
//! (DJI Mic e similares), permite alinhá-la ao vídeo e prepara o buffer que será
//! embutido na exportação.
//!
//! # Distinção central: áudio de análise ≠ áudio de export
//!
//! Duas representações do mesmo arquivo convivem em [`AudioTrack`], e confundi-las
//! é o erro mais caro possível aqui:
//!
//! - [`AudioTrack::samples`] é o **áudio de export**: todos os canais originais,
//!   sample rate original, em `f32` para não quantizar em estágios intermediários.
//!   O formato de origem fica registrado em [`AudioTrack::source_format`] para que o
//!   encode final reconstrua o arquivo sem perda.
//! - [`AudioTrack::mono_analysis`] é o **áudio de análise**: um downmix mono
//!   descartável que existe apenas para a correlação cruzada do auto-sync.
//!   **Nunca** deve ser exportado.
//!
//! O pipeline inteiro (offset, silêncio, trim) opera em `f32`; a conversão para o
//! formato de saída acontece uma única vez, no encode.

pub mod decode;
pub mod export;
pub mod waveform;

use std::fmt;

/// Formato do arquivo de origem, preservado para reencodar sem perda.
///
/// O áudio é mantido em `f32` na memória independentemente disto; este enum
/// registra **de onde ele veio**, para que a exportação escolha um codec que
/// não degrade o material (float continua float, e a profundidade de bits de um
/// PCM inteiro nunca é rebaixada).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SourceFormat {
    /// IEEE float 32-bit (WAV format tag 3). É o formato do DJI Mic.
    ///
    /// É também o default: uma [`AudioTrack`] vazia não deve sugerir que há um
    /// formato de menor precisão a preservar.
    #[default]
    F32,
    /// PCM inteiro com sinal, 32 bits.
    S32,
    /// PCM inteiro com sinal, 24 bits.
    S24,
    /// PCM inteiro com sinal, 16 bits.
    S16,
    /// PCM inteiro sem sinal, 8 bits.
    U8,
    /// Origem comprimida com perda (AAC, MP3...) ou formato não mapeado.
    ///
    /// Não há "formato original" a preservar bit a bit; a exportação pode
    /// escolher livremente, já que a perda ocorreu antes de chegarmos ao arquivo.
    Other,
}

impl SourceFormat {
    /// Profundidade nominal em bits do formato de origem.
    pub fn bits(self) -> u32 {
        match self {
            Self::F32 | Self::S32 => 32,
            Self::S24 => 24,
            Self::S16 => 16,
            Self::U8 => 8,
            Self::Other => 0,
        }
    }

    /// Se a origem é ponto flutuante.
    ///
    /// É esta resposta que dispara a exigência de `pcm_f32le` na exportação:
    /// converter float para inteiro ou para um codec com perda sem avisar o
    /// usuário viola a decisão 7 da especificação.
    pub fn is_float(self) -> bool {
        matches!(self, Self::F32)
    }

    /// Rótulo curto para exibir na interface.
    pub fn label(self) -> &'static str {
        match self {
            Self::F32 => "32-bit float",
            Self::S32 => "32-bit int",
            Self::S24 => "24-bit int",
            Self::S16 => "16-bit int",
            Self::U8 => "8-bit",
            Self::Other => "comprimido",
        }
    }
}

impl fmt::Display for SourceFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Uma trilha de áudio externa importada pelo usuário.
///
/// Ver a documentação do módulo para a distinção entre os campos de export e os
/// de análise.
#[derive(Debug, Clone, Default)]
pub struct AudioTrack {
    /// Caminho do arquivo de origem, no formato de URL do
    /// [`crate::filesystem`] — é o que será serializado no `.gyroflow` e usado
    /// para recarregar o áudio ao reabrir o projeto.
    pub path: String,

    // ---- Dados para EXPORT (formato original preservado) ----
    /// Amostras de **todos os canais, intercaladas** (`[c0_s0, c1_s0, c0_s1, ...]`),
    /// mantidas em `f32`.
    ///
    /// A intercalação segue a convenção do próprio symphonia e é a mesma que o
    /// encoder de saída espera, evitando uma transposição desnecessária.
    pub samples: Vec<f32>,
    /// Número de canais do arquivo original (não é 1 quando a origem é estéreo).
    pub channels: u16,
    /// Sample rate do arquivo original, em Hz.
    pub sample_rate: u32,
    /// Formato de origem, para reencodar sem perda.
    pub source_format: SourceFormat,

    // ---- Dados para ANÁLISE/SYNC (descartável) ----
    /// Downmix mono usado **exclusivamente** pela correlação do auto-sync.
    ///
    /// Nunca deve ser escrito no arquivo exportado.
    pub mono_analysis: Vec<f32>,

    /// Deslocamento aplicado ao áudio, em segundos: `t_audio = t_video + offset`.
    ///
    /// A Fase 1 mantém sempre `0.0`; o ajuste manual entra na Fase 2 e o cálculo
    /// automático na Fase 5.
    pub offset_seconds: f64,

    /// Se o formato de origem deve ser preservado bit a bit na exportação.
    ///
    /// Default `true` (decisão 7). Só o usuário pode desligar isto, e apenas de
    /// forma explícita, ciente da perda.
    pub preserve_original_format: bool,
}

impl AudioTrack {
    /// Número de frames de áudio (um frame contém uma amostra por canal).
    pub fn frame_count(&self) -> usize {
        if self.channels == 0 {
            return 0;
        }
        self.samples.len() / self.channels as usize
    }

    /// Duração da trilha em segundos.
    pub fn duration_seconds(&self) -> f64 {
        if self.sample_rate == 0 {
            return 0.0;
        }
        self.frame_count() as f64 / self.sample_rate as f64
    }

    /// Se a trilha não tem áudio carregado.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Resumo legível do formato, para log e para a interface.
    ///
    /// Exemplo: `"48000 Hz, 2 ch, 32-bit float"`.
    pub fn format_summary(&self) -> String {
        format!("{} Hz, {} ch, {}", self.sample_rate, self.channels, self.source_format)
    }
}
