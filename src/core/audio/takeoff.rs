// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2026 Rodrigo Sclosa

//! Finds the moment the propellers spin up in a recording.
//!
//! This is what anchors an external microphone to the video: the take-off is the
//! one event both the audio and the gyroscope witness. Everything here was
//! calibrated against real recordings made at an event, with a crowd, music and
//! voices in the background - the conditions that broke every simpler approach.
//!
//! Three decisions carry the accuracy, and each replaced something that failed on
//! that material:
//!
//! 1. **The FIRST rise, not the largest one.** Take-off is the first time the
//!    motors spin up. On a 185s clip the largest energy jump was at 140.8s (the
//!    crowd), while the drone left the ground at 37s.
//!
//! 2. **The band is searched, not assumed.** Blade noise sits on harmonics whose
//!    frequency depends on propeller size and RPM: a 2.5" prop and a 5" one do not
//!    share a band. Fixing the band on one recording put the detection 20s off on
//!    another from the same drone. Each octave band is scored on its own.
//!
//! 3. **Sustain tells a flight from a noise.** Propellers keep turning; a shout or
//!    a door does not. Comparing the level after the rise against the level before
//!    it separates the two: across the test recordings, flights scored 5.5x to
//!    9.7x while a microphone left on a table scored 1.2x.

/// A detected take-off.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Takeoff {
    /// Seconds from the start of the audio.
    pub time_seconds: f64,
    /// How much the level holds after the rise, as a ratio against the level
    /// before it. Flights land well above [`MIN_SUSTAIN`]; ambient noise does not.
    pub sustain: f32,
    /// Low edge of the band the detection came from, in Hz. Useful in logs when a
    /// result looks wrong.
    pub band_lo_hz: f32,
}

/// Below this, the rise is not treated as a take-off.
///
/// Measured on six real recordings: flights scored 5.5x-9.7x, a mic left on a
/// table 1.2x. The gap is wide, so the exact cut is not delicate - but it is not
/// perfect either, and one no-flight recording still reached 3.35x, which is why
/// the caller is expected to surface a weak result rather than trust it blindly.
pub const MIN_SUSTAIN: f32 = 4.5;

/// Octave bands searched for the blade signature, as low edges in Hz.
///
/// Each band spans one octave (`lo..lo*2`). The range covers small props, whose
/// harmonics sit high, down to large ones - on the test set the winning band
/// ranged from 1800 Hz to 7000 Hz depending on the recording.
const BAND_EDGES: [f32; 8] = [300.0, 700.0, 1200.0, 1800.0, 2500.0, 3500.0, 5000.0, 7000.0];

/// Envelope resolution. 20 Hz is fine enough to place the take-off within a
/// couple of frames and coarse enough to keep the search cheap.
const ENVELOPE_RATE_HZ: f32 = 20.0;

/// Seconds averaged on each side of a candidate when measuring a rise.
///
/// One second ignores short bursts without blurring the instant: on the test set,
/// 0.3s and 1.0s windows agreed to within 0.2s.
const RISE_WINDOW_S: f32 = 1.0;

/// A rise counts once it reaches this fraction of the largest rise in the clip.
///
/// Relative, not absolute, so it adapts to microphone gain and distance. Values
/// from 0.15 to 0.25 gave the same answer on the test set.
const RISE_FRACTION: f32 = 0.20;

/// Seconds of audio after the rise used to measure sustain.
const SUSTAIN_WINDOW_S: f32 = 10.0;

/// Looks for the take-off in `mono`, returning `None` when no band shows the
/// pattern - which is the right answer for a recording with no flight in it.
pub fn detect(mono: &[f32], sample_rate: u32) -> Option<Takeoff> {
    if mono.is_empty() || sample_rate == 0 {
        return None;
    }

    let nyquist = sample_rate as f32 / 2.0;
    let mut best: Option<Takeoff> = None;

    for &lo in BAND_EDGES.iter() {
        let hi = (lo * 2.0).min(nyquist * 0.9);
        if hi <= lo {
            continue;
        }
        let envelope = band_envelope(mono, sample_rate, lo, hi);
        if envelope.len() < 60 {
            continue;
        }
        let Some((index, sustain)) = first_rise_with_sustain(&envelope) else { continue };

        // Highest sustain wins: among the bands that see a rise, the one where
        // the level holds best is the one actually looking at the propellers.
        if best.map_or(true, |b| sustain > b.sustain) {
            best = Some(Takeoff {
                time_seconds: index as f64 / ENVELOPE_RATE_HZ as f64,
                sustain,
                band_lo_hz: lo,
            });
        }
    }

    best.filter(|t| t.sustain >= MIN_SUSTAIN)
}

/// Energy envelope of one frequency band, normalized to its own maximum.
///
/// Uses Goertzel per probe frequency instead of a full FFT: only a handful of
/// bins per window are needed, and this keeps the whole search over eight bands
/// affordable.
fn band_envelope(mono: &[f32], sample_rate: u32, f_lo: f32, f_hi: f32) -> Vec<f32> {
    const WINDOW: usize = 2048;
    let hop = (sample_rate as f32 / ENVELOPE_RATE_HZ).max(1.0) as usize;

    let probes: Vec<f32> = {
        let mut v = Vec::new();
        let mut f = f_lo;
        while f < f_hi {
            v.push(f);
            f += 200.0;
        }
        v
    };
    if probes.is_empty() {
        return Vec::new();
    }

    let mut envelope = Vec::new();
    let mut start = 0usize;
    while start + WINDOW < mono.len() {
        let window = &mono[start..start + WINDOW];
        let mut total = 0.0f32;
        for &f in &probes {
            total += goertzel(window, sample_rate as f32, f);
        }
        envelope.push(total / probes.len() as f32);
        start += hop;
    }

    let peak = envelope.iter().cloned().fold(0.0f32, f32::max);
    if peak > 1e-9 {
        for v in envelope.iter_mut() {
            *v /= peak;
        }
    }
    envelope
}

/// Magnitude of one frequency bin, by the Goertzel recurrence.
fn goertzel(samples: &[f32], sample_rate: f32, freq: f32) -> f32 {
    let n = samples.len() as f32;
    let k = freq * n / sample_rate;
    let w = 2.0 * std::f32::consts::PI * k / n;
    let coeff = 2.0 * w.cos();
    let (mut s1, mut s2) = (0.0f32, 0.0f32);
    for &v in samples {
        let s0 = v + coeff * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    (s1 * s1 + s2 * s2 - coeff * s1 * s2).max(0.0).sqrt()
}

/// First rise in `envelope` that reaches [`RISE_FRACTION`] of the largest one,
/// with how much the level sustains afterwards.
fn first_rise_with_sustain(envelope: &[f32]) -> Option<(usize, f32)> {
    let k = (RISE_WINDOW_S * ENVELOPE_RATE_HZ) as usize;
    if envelope.len() < k * 2 + 1 {
        return None;
    }

    let rise: Vec<f32> = (0..envelope.len())
        .map(|i| {
            if i < k || i + k >= envelope.len() {
                return 0.0;
            }
            let before: f32 = envelope[i - k..i].iter().sum::<f32>() / k as f32;
            let after: f32 = envelope[i..i + k].iter().sum::<f32>() / k as f32;
            after - before
        })
        .collect();

    let max_rise = rise.iter().cloned().fold(0.0f32, f32::max);
    if max_rise <= 0.0 {
        return None;
    }
    let index = rise.iter().position(|&v| v >= max_rise * RISE_FRACTION)?;

    // Medians, not means: a single loud moment on either side would drag a mean
    // and turn a flight into noise or the other way round.
    let before_from = index.saturating_sub(k * 2);
    let level_before = median(&envelope[before_from..index.max(before_from + 1)]);
    let after_to = (index + (SUSTAIN_WINDOW_S * ENVELOPE_RATE_HZ) as usize).min(envelope.len());
    let level_after = median(&envelope[index..after_to.max(index + 1)]);

    let sustain = if level_before > 1e-6 { level_after / level_before } else { 0.0 };

    // `index` is where the one-second window ahead first looks louder than the
    // one behind, which is up to half a second BEFORE the sound actually rises.
    // The onset is where the level itself crosses 30% of that rise; three
    // samples together so a stray click does not count. On the real recordings
    // this moved the detections 0-0.45s later, within 0.3s of what a listener
    // picks; crossing halfway to the ten-second median instead overshot by up
    // to a second, because the level keeps climbing after the lift-off.
    let after: f32 = envelope[index..index + k].iter().sum::<f32>() / k as f32;
    let target = level_before + (after - level_before) * 0.3;
    let onset = (index..(index + k).min(envelope.len() - 2))
        .find(|&i| envelope[i..i + 3].iter().sum::<f32>() / 3.0 >= target)
        .unwrap_or(index);
    Some((onset, sustain))
}

fn median(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted: Vec<f32> = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    sorted[sorted.len() / 2]
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: u32 = 48000;

    /// Builds a signal: `noise_level` everywhere, plus a tone at `freq` between
    /// `from` and `to` seconds - a stand-in for the propellers running.
    fn synth(duration_s: f32, noise_level: f32, freq: f32, from: f32, to: f32, tone_level: f32) -> Vec<f32> {
        let n = (duration_s * SR as f32) as usize;
        // A fixed generator, so the test does not depend on a random crate and
        // cannot flake between runs.
        let mut state = 0x1234_5678u32;
        (0..n)
            .map(|i| {
                state = state.wrapping_mul(1_103_515_245).wrapping_add(12345);
                let noise = ((state >> 16) as f32 / 32768.0 - 1.0) * noise_level;
                let t = i as f32 / SR as f32;
                let tone = if t >= from && t < to {
                    (2.0 * std::f32::consts::PI * freq * t).sin() * tone_level
                } else {
                    0.0
                };
                noise + tone
            })
            .collect()
    }

    #[test]
    fn finds_the_takeoff_in_a_synthetic_flight() {
        let signal = synth(40.0, 0.02, 2600.0, 15.0, 35.0, 0.5);
        let found = detect(&signal, SR).expect("a flight is present");
        assert!((found.time_seconds - 15.0).abs() < 1.5, "found {:.2}s, expected ~15s", found.time_seconds);
        assert!(found.sustain >= MIN_SUSTAIN, "sustain {:.2} should clear the threshold", found.sustain);
    }

    #[test]
    fn plain_noise_is_not_a_takeoff() {
        let signal = synth(40.0, 0.05, 0.0, 0.0, 0.0, 0.0);
        assert_eq!(detect(&signal, SR), None);
    }

    /// The case that drove the design: a short burst is not a flight, because the
    /// level does not hold afterwards.
    #[test]
    fn a_short_burst_does_not_sustain() {
        let signal = synth(40.0, 0.02, 2600.0, 15.0, 15.5, 0.5);
        assert_eq!(detect(&signal, SR), None);
    }

    /// The band is searched rather than assumed, so a different propeller size -
    /// a much higher tone here - is found just as well.
    #[test]
    fn a_different_blade_frequency_is_still_found() {
        let signal = synth(40.0, 0.02, 6000.0, 12.0, 35.0, 0.5);
        let found = detect(&signal, SR).expect("a flight is present");
        assert!((found.time_seconds - 12.0).abs() < 1.5, "found {:.2}s, expected ~12s", found.time_seconds);
    }

    #[test]
    fn empty_input_is_not_a_crash() {
        assert_eq!(detect(&[], SR), None);
        assert_eq!(detect(&[0.1, 0.2], 0), None);
    }
}
