// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2026 Rodrigo Sclosa

//! Geração da forma de onda desenhada na timeline.
//!
//! A UI **não** recebe amostra por amostra: um minuto de áudio a 48 kHz estéreo
//! são 5,8 milhões de valores, e a timeline tem alguns milhares de pixels de
//! largura. Em vez disso, o Rust reduz o sinal a um par (mínimo, máximo) por
//! bucket — tipicamente um bucket por pixel —, que é o suficiente para desenhar
//! a silhueta clássica da waveform preservando os transientes.
//!
//! Os picos são recalculados quando o zoom muda, porque o número de amostras que
//! cabe em um pixel muda junto.

use super::AudioTrack;

/// Par (mínimo, máximo) das amostras contidas em um bucket.
pub type Peak = (f32, f32);

impl AudioTrack {
    /// Reduz a trilha a pares (mínimo, máximo), um por bucket.
    ///
    /// `samples_per_bucket` é quantos **frames** de áudio cada bucket cobre —
    /// normalmente `frames_visíveis / largura_em_pixels`. Valor 0 é tratado como
    /// 1 para não dividir por zero.
    ///
    /// O cálculo percorre os canais todos e toma o extremo entre eles, de modo
    /// que um transiente presente em apenas um canal continue visível.
    ///
    /// Usa [`AudioTrack::samples`] (áudio de export) porque é o que o usuário
    /// espera ver representado; o downmix de análise não é envolvido no desenho.
    pub fn peaks(&self, samples_per_bucket: usize) -> Vec<Peak> {
        let channels = self.channels as usize;
        if self.samples.is_empty() || channels == 0 {
            return Vec::new();
        }

        let samples_per_bucket = samples_per_bucket.max(1);
        let total_frames = self.samples.len() / channels;
        let bucket_count = total_frames.div_ceil(samples_per_bucket);

        let mut peaks = Vec::with_capacity(bucket_count);
        for bucket in 0..bucket_count {
            let first_frame = bucket * samples_per_bucket;
            let last_frame = ((bucket + 1) * samples_per_bucket).min(total_frames);

            let mut min = f32::MAX;
            let mut max = f32::MIN;
            for value in &self.samples[first_frame * channels..last_frame * channels] {
                if *value < min {
                    min = *value;
                }
                if *value > max {
                    max = *value;
                }
            }

            // Um bucket vazio só ocorre se `last_frame == first_frame`, o que a
            // aritmética acima impede; o guarda existe para não vazar sentinelas.
            if min > max {
                peaks.push((0.0, 0.0));
            } else {
                peaks.push((min, max));
            }
        }

        peaks
    }

    /// Picos calculados para uma largura de destino em pixels.
    ///
    /// Conveniência para a UI, que raciocina em pixels e não em amostras: divide
    /// a trilha inteira em `width_px` buckets.
    pub fn peaks_for_width(&self, width_px: usize) -> Vec<Peak> {
        let channels = self.channels as usize;
        if width_px == 0 || channels == 0 || self.samples.is_empty() {
            return Vec::new();
        }
        let total_frames = self.samples.len() / channels;
        self.peaks(total_frames.div_ceil(width_px))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::SourceFormat;

    /// Trilha sintética mono com valores conhecidos.
    fn track_mono(samples: Vec<f32>) -> AudioTrack {
        AudioTrack {
            path: String::new(),
            samples,
            channels: 1,
            sample_rate: 48000,
            source_format: SourceFormat::F32,
            mono_analysis: Vec::new(),
            offset_seconds: 0.0,
            preserve_original_format: true,
        }
    }

    #[test]
    fn extremos_por_bucket() {
        let track = track_mono(vec![0.0, 1.0, -1.0, 0.5]);
        // Dois buckets de dois frames: (0.0, 1.0) e (-1.0, 0.5).
        assert_eq!(track.peaks(2), vec![(0.0, 1.0), (-1.0, 0.5)]);
    }

    #[test]
    fn ultimo_bucket_parcial_nao_e_descartado() {
        let track = track_mono(vec![0.2, 0.4, 0.9]);
        let peaks = track.peaks(2);
        assert_eq!(peaks.len(), 2);
        assert_eq!(peaks[1], (0.9, 0.9));
    }

    #[test]
    fn bucket_zero_nao_divide_por_zero() {
        let track = track_mono(vec![0.1, 0.2]);
        assert_eq!(track.peaks(0).len(), 2);
    }

    #[test]
    fn estereo_considera_os_dois_canais() {
        let mut track = track_mono(vec![0.0, 1.0, -1.0, 0.0]);
        track.channels = 2;
        // Um único bucket cobrindo os dois frames: o extremo vem de canais
        // diferentes (máximo no canal direito, mínimo no esquerdo).
        assert_eq!(track.peaks(2), vec![(-1.0, 1.0)]);
    }

    #[test]
    fn trilha_vazia_nao_gera_picos() {
        assert!(track_mono(Vec::new()).peaks(10).is_empty());
    }

    #[test]
    fn largura_em_pixels_limita_a_quantidade() {
        let track = track_mono((0..1000).map(|i| i as f32 / 1000.0).collect());
        assert!(track.peaks_for_width(100).len() <= 100);
    }
}
