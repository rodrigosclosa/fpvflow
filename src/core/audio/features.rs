// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2026 Rodrigo Sclosa

//! Envelopes de vibração usados pelo auto-sync.
//!
//! A ideia: as hélices produzem uma vibração que aparece **nos dois sinais** —
//! como som captado pelo microfone e como oscilação lida pelo giroscópio.
//! Quando a rotação muda, a intensidade dessa vibração muda junto nos dois.
//! Correlacionar essas duas curvas de intensidade revela o deslocamento
//! temporal entre eles.
//!
//! # Por que envelopes de energia, e não os espectros
//!
//! O giroscópio amostra a uma taxa `R` relativamente baixa (200–1000 Hz típicos),
//! então só enxerga vibração até `R/2` — o limite de Nyquist. A frequência de
//! passagem das pás costuma estar acima disso, e o que o gyro registra é o
//! **aliasing** dela, numa frequência aparente diferente da real.
//!
//! Comparar os espectros diretamente, portanto, não funcionaria: as frequências
//! não batem. Mas a *intensidade* da vibração ao longo do tempo sobrevive ao
//! aliasing — quando as hélices aceleram, a energia sobe nos dois sinais, cada um
//! na sua banda. É por isso que correlacionamos envelopes de energia.

use std::f32::consts::PI;

use rustfft::{num_complex::Complex, FftPlanner};

/// Tamanho da janela da STFT, em amostras.
pub const STFT_WINDOW: usize = 2048;
/// Passo entre janelas consecutivas.
pub const STFT_HOP: usize = 512;

/// Banda de passagem das pás usada por padrão, em Hz.
///
/// Cobre a frequência fundamental e os primeiros harmônicos da maioria dos
/// drones de consumo.
pub const DEFAULT_BAND_LO_HZ: f32 = 150.0;
pub const DEFAULT_BAND_HI_HZ: f32 = 900.0;

/// Corte do passa-alta aplicado ao gyro, em Hz.
///
/// Remove o movimento intencional (pan, tilt, curvas) e o nível DC, deixando
/// apenas a vibração.
pub const DEFAULT_HIGHPASS_HZ: f32 = 30.0;

/// Abaixo disto o espectro é ignorado na busca automática de banda: é a faixa do
/// vento e do ruído de manuseio, que não carregam a assinatura das pás.
const AUTO_BAND_MIN_HZ: f32 = 80.0;

/// Largura relativa da banda em torno do pico, na detecção automática.
const AUTO_BAND_WIDTH: f32 = 0.4;

/// Parâmetros da extração de envelopes.
///
/// Expostos no `.gyroflow` para que drones com frequência incomum, ou gyros de
/// taxa baixa, sejam ajustados sem recompilar.
#[derive(Debug, Clone, Copy)]
pub struct FeatureParams {
    /// Início da banda fixa, em Hz.
    pub band_lo_hz: f32,
    /// Fim da banda fixa, em Hz.
    pub band_hi_hz: f32,
    /// Se `true`, a banda é detectada a partir do próprio sinal e os limites
    /// acima servem só de fallback.
    pub auto_band: bool,
    /// Corte do passa-alta aplicado ao gyro, em Hz.
    pub highpass_hz: f32,
}

impl Default for FeatureParams {
    fn default() -> Self {
        Self {
            band_lo_hz: DEFAULT_BAND_LO_HZ,
            band_hi_hz: DEFAULT_BAND_HI_HZ,
            auto_band: true,
            highpass_hz: DEFAULT_HIGHPASS_HZ,
        }
    }
}

/// Janela de Hann de `n` pontos.
///
/// Suaviza as bordas de cada janela da STFT, evitando que a descontinuidade
/// artificial do recorte espalhe energia por todo o espectro.
fn hann_window(n: usize) -> Vec<f32> {
    (0..n).map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / n as f32).cos())).collect()
}

/// Compressão logarítmica.
///
/// A energia da vibração varia por ordens de grandeza; o log aproxima as escalas
/// e impede que um único pico domine a correlação.
fn log_compress(value: f32) -> f32 {
    (value + 1e-9).ln()
}

/// Espectro médio do sinal ao longo do tempo.
///
/// Usado pela detecção automática de banda: a assinatura das pás é persistente,
/// então aparece na média mesmo quando eventos isolados são mais intensos.
fn average_spectrum(samples: &[f32], sample_rate: u32) -> Vec<f32> {
    if samples.len() < STFT_WINDOW {
        return Vec::new();
    }

    let window = hann_window(STFT_WINDOW);
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(STFT_WINDOW);

    let bins = STFT_WINDOW / 2;
    let mut accum = vec![0.0f32; bins];
    let mut frames = 0usize;

    let mut buffer = vec![Complex::new(0.0f32, 0.0f32); STFT_WINDOW];

    let mut pos = 0;
    while pos + STFT_WINDOW <= samples.len() {
        for i in 0..STFT_WINDOW {
            buffer[i] = Complex::new(samples[pos + i] * window[i], 0.0);
        }
        fft.process(&mut buffer);

        for (bin, acc) in accum.iter_mut().enumerate() {
            *acc += buffer[bin].norm_sqr();
        }
        frames += 1;
        pos += STFT_HOP;
    }

    let _ = sample_rate;
    if frames > 0 {
        for value in &mut accum {
            *value /= frames as f32;
        }
    }
    accum
}

/// Descobre a banda dominante da vibração a partir do próprio sinal.
///
/// Devolve `(lo_hz, hi_hz)` em torno do pico do espectro médio, ignorando o que
/// está abaixo de [`AUTO_BAND_MIN_HZ`] — sem assumir nenhum RPM conhecido.
///
/// Devolve `None` quando o sinal é curto demais para uma estimativa confiável;
/// o chamador então usa a banda fixa.
pub fn detect_band(samples: &[f32], sample_rate: u32) -> Option<(f32, f32)> {
    let spectrum = average_spectrum(samples, sample_rate);
    if spectrum.is_empty() {
        return None;
    }

    let hz_per_bin = sample_rate as f32 / STFT_WINDOW as f32;
    let min_bin = (AUTO_BAND_MIN_HZ / hz_per_bin).ceil() as usize;
    if min_bin >= spectrum.len() {
        return None;
    }

    let (peak_bin, _) = spectrum[min_bin..]
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))?;

    let peak_hz = (min_bin + peak_bin) as f32 * hz_per_bin;
    if peak_hz <= 0.0 {
        return None;
    }

    Some((peak_hz * (1.0 - AUTO_BAND_WIDTH), peak_hz * (1.0 + AUTO_BAND_WIDTH)))
}

/// Envelope de energia do áudio na banda das pás.
///
/// Opera sobre o downmix mono de análise — nunca sobre o áudio de export.
/// Devolve o envelope e a taxa dele em Hz (`sample_rate / STFT_HOP`).
pub fn audio_envelope(mono: &[f32], sample_rate: u32, params: &FeatureParams) -> (Vec<f32>, f32) {
    if mono.len() < STFT_WINDOW || sample_rate == 0 {
        return (Vec::new(), 0.0);
    }

    // A banda automática cai para a fixa quando o sinal não permite estimar —
    // é o fallback que o prompt pede quando o auto-band pega vento.
    let (band_lo, band_hi) = if params.auto_band {
        detect_band(mono, sample_rate).unwrap_or((params.band_lo_hz, params.band_hi_hz))
    } else {
        (params.band_lo_hz, params.band_hi_hz)
    };

    let hz_per_bin = sample_rate as f32 / STFT_WINDOW as f32;
    let bin_lo = ((band_lo / hz_per_bin).floor() as usize).max(1);
    let bin_hi = ((band_hi / hz_per_bin).ceil() as usize).min(STFT_WINDOW / 2);
    if bin_hi <= bin_lo {
        return (Vec::new(), 0.0);
    }

    let window = hann_window(STFT_WINDOW);
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(STFT_WINDOW);

    let mut envelope = Vec::with_capacity(mono.len() / STFT_HOP + 1);
    let mut buffer = vec![Complex::new(0.0f32, 0.0f32); STFT_WINDOW];

    let mut pos = 0;
    while pos + STFT_WINDOW <= mono.len() {
        for i in 0..STFT_WINDOW {
            buffer[i] = Complex::new(mono[pos + i] * window[i], 0.0);
        }
        fft.process(&mut buffer);

        let energy: f32 = buffer[bin_lo..bin_hi].iter().map(|c| c.norm_sqr()).sum();
        envelope.push(log_compress(energy));

        pos += STFT_HOP;
    }

    (envelope, sample_rate as f32 / STFT_HOP as f32)
}

/// Filtro passa-alta de primeira ordem.
///
/// Simples de propósito: o objetivo é remover a componente lenta do movimento
/// intencional, não construir um filtro com resposta precisa. O que importa para
/// a correlação é o envelope resultante, não a fidelidade da banda.
fn highpass(signal: &[f32], sample_rate: f32, cutoff_hz: f32) -> Vec<f32> {
    if signal.is_empty() || sample_rate <= 0.0 {
        return Vec::new();
    }
    let rc = 1.0 / (2.0 * PI * cutoff_hz);
    let dt = 1.0 / sample_rate;
    let alpha = rc / (rc + dt);

    let mut out = Vec::with_capacity(signal.len());
    let mut prev_in = signal[0];
    let mut prev_out = 0.0;
    out.push(0.0);
    for &sample in &signal[1..] {
        let value = alpha * (prev_out + sample - prev_in);
        out.push(value);
        prev_out = value;
        prev_in = sample;
    }
    out
}

/// Envelope de energia da vibração lida pelo giroscópio.
///
/// - `gyro` são as amostras `(timestamp_ms, [x, y, z])` em `f64`, como o
///   `telemetry-parser` as entrega.
/// - `target_rate_hz` é a taxa do envelope de áudio, para que as duas curvas
///   fiquem na mesma base de tempo e possam ser correlacionadas.
///
/// O sinal usado é a magnitude `sqrt(x² + y² + z²)`, que independe da orientação
/// do drone.
pub fn gyro_envelope(gyro: &[(f64, [f64; 3])], target_rate_hz: f32, params: &FeatureParams) -> Vec<f32> {
    if gyro.len() < 2 || target_rate_hz <= 0.0 {
        return Vec::new();
    }

    let first_ms = gyro[0].0;
    let last_ms = gyro[gyro.len() - 1].0;
    let duration_s = (last_ms - first_ms) / 1000.0;
    if duration_s <= 0.0 {
        return Vec::new();
    }

    // Taxa real de amostragem do gyro, derivada dos timestamps: não existe campo
    // pronto para isso no core (ver docs/audio-sync/recon.md, seção 2).
    let gyro_rate = (gyro.len() as f64 / duration_s) as f32;

    let magnitude: Vec<f32> = gyro
        .iter()
        .map(|(_, v)| ((v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()) as f32)
        .collect();

    let filtered = highpass(&magnitude, gyro_rate, params.highpass_hz);

    // Energia por janela, com a janela dimensionada para que o envelope saia
    // exatamente na taxa do envelope de áudio.
    let samples_per_bucket = (gyro_rate / target_rate_hz).round().max(1.0) as usize;
    let bucket_count = filtered.len() / samples_per_bucket;

    let mut envelope = Vec::with_capacity(bucket_count);
    for bucket in 0..bucket_count {
        let from = bucket * samples_per_bucket;
        let to = (from + samples_per_bucket).min(filtered.len());
        let energy: f32 = filtered[from..to].iter().map(|v| v * v).sum();
        envelope.push(log_compress(energy));
    }

    envelope
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Seno de `freq_hz` com amplitude variável ao longo do tempo.
    fn tone(freq_hz: f32, sample_rate: u32, seconds: f32, amplitude: impl Fn(f32) -> f32) -> Vec<f32> {
        let n = (sample_rate as f32 * seconds) as usize;
        (0..n)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                amplitude(t) * (2.0 * PI * freq_hz * t).sin()
            })
            .collect()
    }

    #[test]
    fn envelope_acompanha_a_variacao_de_intensidade() {
        // Tom de 400 Hz que fica mais forte na segunda metade.
        let sr = 8000;
        let signal = tone(400.0, sr, 2.0, |t| if t < 1.0 { 0.1 } else { 1.0 });
        let params = FeatureParams { auto_band: false, ..Default::default() };
        let (env, rate) = audio_envelope(&signal, sr, &params);

        assert!(!env.is_empty());
        assert!((rate - sr as f32 / STFT_HOP as f32).abs() < 0.01);

        // A energia na segunda metade tem que ser claramente maior.
        let mid = env.len() / 2;
        let first_half: f32 = env[..mid].iter().sum::<f32>() / mid as f32;
        let second_half: f32 = env[mid..].iter().sum::<f32>() / (env.len() - mid) as f32;
        assert!(second_half > first_half + 1.0, "primeira={first_half}, segunda={second_half}");
    }

    #[test]
    fn deteccao_automatica_encontra_o_tom_dominante() {
        let sr = 8000;
        let signal = tone(500.0, sr, 2.0, |_| 1.0);
        let (lo, hi) = detect_band(&signal, sr).expect("deveria detectar a banda");
        // A banda tem que conter os 500 Hz reais.
        assert!(lo < 500.0 && hi > 500.0, "banda detectada: {lo}..{hi}");
    }

    #[test]
    fn banda_ignora_a_faixa_do_vento() {
        let sr = 8000;
        // Rumble forte em 30 Hz + assinatura em 600 Hz, mais fraca.
        let mut signal = tone(30.0, sr, 2.0, |_| 3.0);
        let blades = tone(600.0, sr, 2.0, |_| 1.0);
        for (s, b) in signal.iter_mut().zip(blades.iter()) {
            *s += *b;
        }
        let (lo, hi) = detect_band(&signal, sr).expect("deveria detectar a banda");
        // Apesar de o rumble ser mais forte, a banda tem que cair nos 600 Hz.
        assert!(lo > AUTO_BAND_MIN_HZ, "banda começou dentro do vento: {lo}");
        assert!(lo < 600.0 && hi > 600.0, "banda detectada: {lo}..{hi}");
    }

    #[test]
    fn sinal_curto_nao_gera_envelope() {
        let (env, rate) = audio_envelope(&[0.0; 100], 8000, &FeatureParams::default());
        assert!(env.is_empty());
        assert_eq!(rate, 0.0);
    }

    #[test]
    fn envelope_do_gyro_sai_na_taxa_pedida() {
        // 10 s de gyro a 500 Hz, com vibração que dobra na metade.
        let gyro_rate = 500.0;
        let n = 5000;
        let gyro: Vec<(f64, [f64; 3])> = (0..n)
            .map(|i| {
                let t = i as f64 / gyro_rate;
                let amp = if t < 5.0 { 0.1 } else { 1.0 };
                let v = amp * (2.0 * std::f64::consts::PI * 80.0 * t).sin();
                (t * 1000.0, [v, 0.0, 0.0])
            })
            .collect();

        let target_rate = 15.625; // 8000/512
        let env = gyro_envelope(&gyro, target_rate, &FeatureParams::default());

        assert!(!env.is_empty());
        // ~10 s a 15.625 Hz ≈ 156 pontos, com folga para o arredondamento.
        assert!(env.len() > 100 && env.len() < 200, "tamanho={}", env.len());

        let mid = env.len() / 2;
        let first: f32 = env[..mid].iter().sum::<f32>() / mid as f32;
        let second: f32 = env[mid..].iter().sum::<f32>() / (env.len() - mid) as f32;
        assert!(second > first, "primeira={first}, segunda={second}");
    }

    #[test]
    fn passa_alta_remove_o_nivel_dc() {
        // Sinal constante: só DC, nada de vibração.
        let dc = vec![5.0f32; 1000];
        let filtered = highpass(&dc, 500.0, 30.0);
        // Depois do transiente inicial o resultado tem que ficar perto de zero.
        assert!(filtered[500..].iter().all(|v| v.abs() < 0.1));
    }
}
