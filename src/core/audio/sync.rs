// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2026 Rodrigo Sclosa

//! Correlação cruzada entre os envelopes de vibração do áudio e do giroscópio.
//!
//! Recebe as duas curvas de intensidade produzidas por [`super::features`] e
//! devolve o deslocamento temporal que melhor as alinha, junto com um score de
//! confiança.
//!
//! A correlação é feita via FFT porque a versão direta é O(n²): com envelopes de
//! alguns minutos a 15 Hz, são dezenas de milhares de pontos, e a diferença entre
//! O(n²) e O(n log n) é a diferença entre segundos e milissegundos.
//!
//! A lógica aqui é independente de UI e de arquivo: entra `(audio_env, gyro_env,
//! env_rate_hz)`, sai `(offset_seconds, confidence)`.

use rustfft::{num_complex::Complex, FftPlanner};

/// Resultado do alinhamento automático.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SyncResult {
    /// Deslocamento estimado, em segundos, na convenção
    /// `t_audio = t_video + offset`.
    pub offset_seconds: f64,
    /// Qualidade do casamento, de 0 a 1.
    ///
    /// É o coeficiente de correlação normalizado no pico. Valores baixos
    /// significam que os dois sinais não têm uma assinatura comum clara — por
    /// exemplo quando o gimbal isola a vibração das pás, ou quando o áudio foi
    /// gravado longe do drone.
    pub confidence: f32,
}

/// Normaliza um envelope: média zero, desvio padrão um.
///
/// Sem isso a correlação seria dominada pela amplitude absoluta de cada sinal,
/// que não tem relação nenhuma entre microfone e giroscópio.
fn normalize(signal: &[f32]) -> Vec<f32> {
    if signal.is_empty() {
        return Vec::new();
    }
    let mean = signal.iter().sum::<f32>() / signal.len() as f32;
    let variance = signal.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / signal.len() as f32;
    let std_dev = variance.sqrt();

    if std_dev < 1e-9 {
        // Sinal constante: não há informação de alinhamento nele.
        return vec![0.0; signal.len()];
    }
    signal.iter().map(|v| (v - mean) / std_dev).collect()
}

/// Correlação cruzada de dois sinais via FFT.
///
/// Devolve o vetor de correlação completo, com deslocamentos de
/// `-(b.len()-1)` a `+(a.len()-1)`, onde o índice `i` corresponde ao
/// deslocamento `i - (b.len() - 1)`.
fn cross_correlate_fft(a: &[f32], b: &[f32]) -> Vec<f32> {
    let result_len = a.len() + b.len() - 1;
    // A FFT trabalha melhor com potências de dois, e o padding não altera o
    // resultado da correlação linear.
    let fft_len = result_len.next_power_of_two();

    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(fft_len);
    let ifft = planner.plan_fft_inverse(fft_len);

    let mut buf_a = vec![Complex::new(0.0f32, 0.0f32); fft_len];
    let mut buf_b = vec![Complex::new(0.0f32, 0.0f32); fft_len];

    for (i, v) in a.iter().enumerate() {
        buf_a[i] = Complex::new(*v, 0.0);
    }
    // `b` entra invertido: correlação é convolução com o sinal espelhado.
    for (i, v) in b.iter().enumerate() {
        buf_b[b.len() - 1 - i] = Complex::new(*v, 0.0);
    }

    fft.process(&mut buf_a);
    fft.process(&mut buf_b);

    // Multiplicação no domínio da frequência = convolução no domínio do tempo.
    for (x, y) in buf_a.iter_mut().zip(buf_b.iter()) {
        *x *= *y;
    }

    ifft.process(&mut buf_a);

    let scale = 1.0 / fft_len as f32;
    buf_a[..result_len].iter().map(|c| c.re * scale).collect()
}

/// Estima o deslocamento entre os envelopes de áudio e de giroscópio.
///
/// `env_rate_hz` é a taxa comum dos dois envelopes, em Hz — é ela que converte o
/// deslocamento em amostras para segundos.
///
/// A convenção do offset é a mesma do resto do módulo: `t_audio = t_video +
/// offset`. Um offset positivo significa que o evento aparece **mais tarde** no
/// áudio do que no vídeo.
pub fn cross_correlate(audio_env: &[f32], gyro_env: &[f32], env_rate_hz: f32) -> SyncResult {
    if audio_env.len() < 2 || gyro_env.len() < 2 || env_rate_hz <= 0.0 {
        return SyncResult { offset_seconds: 0.0, confidence: 0.0 };
    }

    let a = normalize(audio_env);
    let g = normalize(gyro_env);

    let correlation = cross_correlate_fft(&a, &g);
    if correlation.is_empty() {
        return SyncResult { offset_seconds: 0.0, confidence: 0.0 };
    }

    let (peak_index, peak_value) = correlation
        .iter()
        .enumerate()
        .max_by(|x, y| x.1.partial_cmp(y.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, v)| (i, *v))
        .unwrap_or((0, 0.0));

    // O índice 0 corresponde ao deslocamento mais negativo possível.
    let lag_samples = peak_index as i64 - (g.len() as i64 - 1);
    let offset_seconds = lag_samples as f64 / env_rate_hz as f64;

    // Normalizar pelo número de pontos sobrepostos transforma a soma bruta no
    // coeficiente de correlação, que fica em 0..1 e é comparável entre clipes de
    // durações diferentes.
    let overlap = (a.len().min(g.len())) as f32;
    let confidence = (peak_value / overlap).clamp(0.0, 1.0);

    SyncResult { offset_seconds, confidence }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Envelope sintético com um pulso gaussiano em `center`.
    ///
    /// Imita o que acontece quando as hélices aceleram: um aumento localizado de
    /// energia, que é exatamente o que a correlação procura.
    fn pulse(len: usize, center: f32, width: f32) -> Vec<f32> {
        (0..len)
            .map(|i| {
                let d = (i as f32 - center) / width;
                (-d * d).exp()
            })
            .collect()
    }

    #[test]
    fn recupera_offset_zero() {
        let signal = pulse(500, 250.0, 20.0);
        let r = cross_correlate(&signal, &signal, 100.0);
        assert!(r.offset_seconds.abs() < 0.02, "offset={}", r.offset_seconds);
        assert!(r.confidence > 0.5, "confiança={}", r.confidence);
    }

    #[test]
    fn recupera_offset_positivo_conhecido() {
        // O pulso do áudio acontece 50 amostras DEPOIS do pulso do gyro.
        // A 100 Hz de taxa de envelope, isso é +0.5 s.
        let audio = pulse(500, 300.0, 20.0);
        let gyro = pulse(500, 250.0, 20.0);
        let r = cross_correlate(&audio, &gyro, 100.0);
        assert!((r.offset_seconds - 0.5).abs() < 0.03, "offset={}", r.offset_seconds);
    }

    #[test]
    fn recupera_offset_negativo_conhecido() {
        // Agora o áudio vem 50 amostras ANTES: -0.5 s.
        let audio = pulse(500, 200.0, 20.0);
        let gyro = pulse(500, 250.0, 20.0);
        let r = cross_correlate(&audio, &gyro, 100.0);
        assert!((r.offset_seconds + 0.5).abs() < 0.03, "offset={}", r.offset_seconds);
    }

    #[test]
    fn confianca_cai_com_sinais_sem_relacao() {
        // Dois padrões diferentes, sem assinatura comum.
        let audio: Vec<f32> = (0..500).map(|i| ((i as f32) * 0.7).sin()).collect();
        let gyro: Vec<f32> = (0..500).map(|i| ((i as f32) * 0.013).cos()).collect();

        let matched = cross_correlate(&audio, &audio, 100.0);
        let unmatched = cross_correlate(&audio, &gyro, 100.0);

        assert!(
            unmatched.confidence < matched.confidence,
            "sem relação={}, casado={}",
            unmatched.confidence,
            matched.confidence
        );
    }

    #[test]
    fn sinal_constante_nao_gera_falso_alinhamento() {
        let flat = vec![1.0f32; 300];
        let signal = pulse(300, 150.0, 10.0);
        let r = cross_correlate(&signal, &flat, 100.0);
        // Sem variação no gyro não há o que casar.
        assert!(r.confidence < 0.1, "confiança={}", r.confidence);
    }

    #[test]
    fn entrada_curta_demais_e_rejeitada() {
        let r = cross_correlate(&[1.0], &[1.0], 100.0);
        assert_eq!(r.confidence, 0.0);
        assert_eq!(r.offset_seconds, 0.0);
    }

    #[test]
    fn taxa_de_envelope_converte_corretamente() {
        let audio = pulse(500, 300.0, 20.0);
        let gyro = pulse(500, 250.0, 20.0);

        // O mesmo deslocamento de 50 amostras vira offsets diferentes conforme
        // a taxa do envelope.
        let r100 = cross_correlate(&audio, &gyro, 100.0);
        let r50 = cross_correlate(&audio, &gyro, 50.0);
        assert!((r100.offset_seconds - 0.5).abs() < 0.03);
        assert!((r50.offset_seconds - 1.0).abs() < 0.06);
    }
}
