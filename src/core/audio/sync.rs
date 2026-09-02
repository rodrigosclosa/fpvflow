// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2026 Rodrigo Sclosa

//! Cross-correlation between the audio and gyroscope vibration envelopes.
//!
//! Takes the two intensity curves produced by [`super::features`] and returns the
//! time shift that best aligns them, along with a confidence score.
//!
//! The correlation goes through an FFT because the direct version is O(n^2): a
//! few minutes of envelope at 15 Hz is tens of thousands of points, and the
//! difference between O(n^2) and O(n log n) is the difference between seconds and
//! milliseconds.
//!
//! The logic here is independent of UI and files: `(audio_env, gyro_env,
//! env_rate_hz)` in, `(offset_seconds, confidence)` out.

use rustfft::{num_complex::Complex, FftPlanner};

/// Result of the automatic alignment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SyncResult {
    /// Estimated shift in seconds, following `t_audio = t_video + offset`.
    pub offset_seconds: f64,
    /// Match quality, from 0 to 1.
    ///
    /// The normalized correlation coefficient at the peak. Low values mean the
    /// two signals have no clear common signature - for example when the gimbal
    /// isolates the blade vibration, or when the audio was recorded far from the
    /// drone.
    pub confidence: f32,
}

/// Normalizes an envelope to zero mean and unit standard deviation.
///
/// Without this the correlation would be dominated by the absolute amplitude of
/// each signal, which is unrelated between a microphone and a gyroscope.
fn normalize(signal: &[f32]) -> Vec<f32> {
    if signal.is_empty() {
        return Vec::new();
    }
    let mean = signal.iter().sum::<f32>() / signal.len() as f32;
    let variance = signal.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / signal.len() as f32;
    let std_dev = variance.sqrt();

    if std_dev < 1e-9 {
        // Constant signal: carries no alignment information.
        return vec![0.0; signal.len()];
    }
    signal.iter().map(|v| (v - mean) / std_dev).collect()
}

/// Cross-correlates two signals via FFT.
///
/// Returns the full correlation vector, with lags from `-(b.len()-1)` to
/// `+(a.len()-1)`, where index `i` maps to lag `i - (b.len() - 1)`.
fn cross_correlate_fft(a: &[f32], b: &[f32]) -> Vec<f32> {
    let result_len = a.len() + b.len() - 1;
    // The FFT works best with powers of two, and the padding does not change the
    // linear correlation result.
    let fft_len = result_len.next_power_of_two();

    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(fft_len);
    let ifft = planner.plan_fft_inverse(fft_len);

    let mut buf_a = vec![Complex::new(0.0f32, 0.0f32); fft_len];
    let mut buf_b = vec![Complex::new(0.0f32, 0.0f32); fft_len];

    for (i, v) in a.iter().enumerate() {
        buf_a[i] = Complex::new(*v, 0.0);
    }
    // `b` goes in reversed: correlation is convolution with the mirrored signal.
    for (i, v) in b.iter().enumerate() {
        buf_b[b.len() - 1 - i] = Complex::new(*v, 0.0);
    }

    fft.process(&mut buf_a);
    fft.process(&mut buf_b);

    // Multiplication in the frequency domain is convolution in the time domain.
    for (x, y) in buf_a.iter_mut().zip(buf_b.iter()) {
        *x *= *y;
    }

    ifft.process(&mut buf_a);

    let scale = 1.0 / fft_len as f32;
    buf_a[..result_len].iter().map(|c| c.re * scale).collect()
}

/// Estimates the shift between the audio and gyroscope envelopes.
///
/// `env_rate_hz` is the common rate of both envelopes, in Hz - it converts the
/// lag in samples into seconds.
///
/// The offset convention matches the rest of the module: `t_audio = t_video +
/// offset`. A positive offset means the event appears later in the audio than in
/// the video.
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

    // Index 0 corresponds to the most negative lag possible.
    let lag_samples = peak_index as i64 - (g.len() as i64 - 1);
    let offset_seconds = lag_samples as f64 / env_rate_hz as f64;

    // Normalizing by the number of overlapping points turns the raw sum into the
    // correlation coefficient, which stays in 0..1 and is comparable across clips
    // of different durations.
    let overlap = (a.len().min(g.len())) as f32;
    let confidence = (peak_value / overlap).clamp(0.0, 1.0);

    SyncResult { offset_seconds, confidence }
}

/// Index of the FIRST sustained rise in `envelope`, or `None` if there is none.
///
/// Take-off is the *first* time the propellers spin up, not the loudest moment of
/// the clip. Looking for the maximum rise fails on real footage: in a recording
/// made at an event, the crowd and the music produce bigger energy jumps later on,
/// and on a 185s test clip the largest jump landed at 140.8s while the drone left
/// the ground at 37s. Taking the first rise above a fraction of the largest one
/// puts it at 37.05s - the difference between unusable and frame accurate.
///
/// `smooth_window` is how many samples are averaged on each side of a candidate.
/// Wider windows ignore short bursts (a shout, a door) at the cost of blurring the
/// exact instant; around one second works well.
///
/// `threshold_fraction` is measured against the largest rise in this same clip,
/// not against an absolute level, so it adapts to the microphone gain and to how
/// far away the drone was.
pub fn first_sustained_rise(envelope: &[f32], smooth_window: usize, threshold_fraction: f32) -> Option<usize> {
    let k = smooth_window.max(1);
    if envelope.len() < k * 2 + 1 {
        return None;
    }

    // Rise at each point: how much the average energy grows from the window
    // before it to the window after it.
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

    let threshold = max_rise * threshold_fraction.clamp(0.0, 1.0);
    rise.iter().position(|&v| v >= threshold)
}

/// Index where the aircraft leaves the ground, read from its angular speed in
/// degrees per second, or `None` if it never does.
///
/// On the ground the orientation only drifts: across six real DJI O4P clips the
/// speed sat at 1-2 deg/s until the take-off, then ramped to 30-50 deg/s within
/// a second. That floor is what everything is measured against, as a multiple,
/// so the rule does not depend on the camera's noise level.
///
/// Three things have to be true of the instant reported:
///
/// - **It leaves the floor for good.** Throttling up rocks the aircraft on the
///   ground before it lifts: on the reference clip a bump to 17 deg/s fell
///   straight back, one second before the real lift-off. A rise that returns
///   to the floor within [`LIFT_OFF_STAY_S`] is that nudge.
/// - **It is sustained**, like the audio rule in `takeoff`: the median speed
///   over the following [`LIFT_OFF_HOLD_S`] stays above the floor.
/// - **It is the start of the motion**, not the sample that crossed the
///   threshold: the index walks back to where the speed left the floor.
///
/// Earlier versions read one quaternion component as "attitude Z" and looked
/// for its first drift, its first bend, or the top of its first fall. None of
/// those is a physical quantity, and all of them fired seconds late on the
/// reference clip.
pub fn lift_off(speed: &[f32], rate: f32) -> Option<usize> {
    let n = speed.len();
    let stay = (LIFT_OFF_STAY_S * rate) as usize;
    let hold = (LIFT_OFF_HOLD_S * rate) as usize;
    if stay == 0 || n < hold.max(stay) {
        return None;
    }

    let median = |v: &[f32]| -> f32 {
        let mut sorted: Vec<f32> = v.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        sorted[sorted.len() / 2]
    };
    let floor = {
        let mut sorted: Vec<f32> = speed.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        sorted[n / 10].max(1e-3)
    };
    let rise = floor * LIFT_OFF_RISE;
    let moving = floor * LIFT_OFF_MOVING;
    let sustained = floor * LIFT_OFF_SUSTAIN;

    for j in 0..n - stay {
        if speed[j] < rise {
            continue;
        }
        if speed[j..j + stay].iter().any(|&v| v < moving) {
            continue;
        }
        if median(&speed[j..(j + hold).min(n)]) < sustained {
            continue;
        }
        let mut k = j;
        while k > 0 && speed[k - 1] >= moving {
            k -= 1;
        }
        return Some(k);
    }
    None
}

/// Seconds the speed must stay off the ground floor for a rise to count.
pub const LIFT_OFF_STAY_S: f32 = 2.0;
/// Seconds over which the speed must remain elevated, as a median.
pub const LIFT_OFF_HOLD_S: f32 = 5.0;
/// Multiples of the ground floor that count as a rise, as still moving, and as
/// sustained flight. Ground drift is ~1 deg/s and flight is tens, so all three
/// sit well inside the gap.
const LIFT_OFF_RISE: f32 = 4.0;
const LIFT_OFF_MOVING: f32 = 2.0;
const LIFT_OFF_SUSTAIN: f32 = 3.0;

#[cfg(test)]
mod tests {
    use super::*;

    /// Angular speed of a clip at 20 Hz: `floor` on the ground, `flight` from
    /// `lift_off` on, with an optional half-second nudge that returns to the
    /// ground.
    fn flight(len: usize, floor: f32, lift_off: usize, flight: f32, nudge: Option<usize>) -> Vec<f32> {
        (0..len)
            .map(|i| {
                let mut v = floor + if i % 2 == 0 { 0.1 } else { -0.1 };
                if i >= lift_off {
                    v = flight;
                }
                if let Some(n) = nudge {
                    if i >= n && i < n + 10 {
                        v = flight * 0.4;
                    }
                }
                v
            })
            .collect()
    }

    #[test]
    fn lift_off_is_where_the_speed_leaves_the_ground() {
        let speed = flight(1200, 1.0, 400, 40.0, None);
        assert_eq!(lift_off(&speed, 20.0), Some(400));
    }

    /// The reference clip: the aircraft rocks on the ground a second before it
    /// actually leaves. That bump falls straight back and must be skipped.
    #[test]
    fn a_nudge_that_returns_to_the_ground_is_not_the_lift_off() {
        let speed = flight(1200, 1.0, 400, 40.0, Some(370));
        assert_eq!(lift_off(&speed, 20.0), Some(400));
    }

    #[test]
    fn a_grounded_clip_has_no_lift_off() {
        assert_eq!(lift_off(&flight(1200, 1.0, usize::MAX, 40.0, None), 20.0), None);
        assert_eq!(lift_off(&flight(1200, 1.0, usize::MAX, 40.0, Some(370)), 20.0), None);
    }

    /// The thresholds are multiples of the clip's own floor, so a camera with
    /// more drift is read the same way.
    #[test]
    fn a_noisier_camera_is_measured_against_its_own_floor() {
        let speed = flight(1200, 5.0, 400, 60.0, Some(370));
        assert_eq!(lift_off(&speed, 20.0), Some(400));
    }

    #[test]
    fn too_short_for_a_lift_off_is_not_a_crash() {
        assert_eq!(lift_off(&[1.0, 40.0, 40.0], 20.0), None);
        assert_eq!(lift_off(&[], 20.0), None);
    }
    /// Envelope that rises at `first` and rises much harder later, which is the
    /// shape that broke the previous "largest rise" logic: quiet, take-off, then
    /// something louder further in (crowd, music, a low pass).
    fn two_rises(len: usize, first: usize, second: usize) -> Vec<f32> {
        (0..len)
            .map(|i| {
                if i >= second { 1.0 }
                else if i >= first { 0.25 }
                else { 0.05 }
            })
            .collect()
    }

    #[test]
    fn first_rise_wins_over_the_largest_one() {
        let env = two_rises(600, 200, 400);
        let found = first_sustained_rise(&env, 20, 0.2).expect("a rise exists");
        // Near the first step, nowhere near the bigger one at 400.
        assert!((found as i64 - 200).abs() <= 25, "found {found}, expected ~200");
    }

    #[test]
    fn flat_envelope_has_no_rise() {
        assert_eq!(first_sustained_rise(&vec![0.5; 300], 20, 0.2), None);
    }

    #[test]
    fn too_short_for_the_window_is_not_a_crash() {
        assert_eq!(first_sustained_rise(&[0.1, 0.2, 0.3], 20, 0.2), None);
    }

    /// Synthetic envelope with a gaussian pulse at `center`, imitating what
    /// happens when the propellers spin up: a localized energy increase, which is
    /// exactly what the correlation looks for.
    fn pulse(len: usize, center: f32, width: f32) -> Vec<f32> {
        (0..len)
            .map(|i| {
                let d = (i as f32 - center) / width;
                (-d * d).exp()
            })
            .collect()
    }

    #[test]
    fn recovers_zero_offset() {
        let signal = pulse(500, 250.0, 20.0);
        let r = cross_correlate(&signal, &signal, 100.0);
        assert!(r.offset_seconds.abs() < 0.02, "offset={}", r.offset_seconds);
        assert!(r.confidence > 0.5, "confidence={}", r.confidence);
    }

    #[test]
    fn recovers_known_positive_offset() {
        // The audio pulse happens 50 samples AFTER the gyro pulse. At a 100 Hz
        // envelope rate that is +0.5 s.
        let audio = pulse(500, 300.0, 20.0);
        let gyro = pulse(500, 250.0, 20.0);
        let r = cross_correlate(&audio, &gyro, 100.0);
        assert!((r.offset_seconds - 0.5).abs() < 0.03, "offset={}", r.offset_seconds);
    }

    #[test]
    fn recovers_known_negative_offset() {
        // Now the audio comes 50 samples BEFORE: -0.5 s.
        let audio = pulse(500, 200.0, 20.0);
        let gyro = pulse(500, 250.0, 20.0);
        let r = cross_correlate(&audio, &gyro, 100.0);
        assert!((r.offset_seconds + 0.5).abs() < 0.03, "offset={}", r.offset_seconds);
    }

    #[test]
    fn confidence_drops_for_unrelated_signals() {
        // Two different patterns, with no common signature.
        let audio: Vec<f32> = (0..500).map(|i| ((i as f32) * 0.7).sin()).collect();
        let gyro: Vec<f32> = (0..500).map(|i| ((i as f32) * 0.013).cos()).collect();

        let matched = cross_correlate(&audio, &audio, 100.0);
        let unmatched = cross_correlate(&audio, &gyro, 100.0);

        assert!(
            unmatched.confidence < matched.confidence,
            "unrelated={}, matched={}",
            unmatched.confidence,
            matched.confidence
        );
    }

    #[test]
    fn constant_signal_produces_no_false_alignment() {
        let flat = vec![1.0f32; 300];
        let signal = pulse(300, 150.0, 10.0);
        let r = cross_correlate(&signal, &flat, 100.0);
        // With no variation in the gyro there is nothing to match.
        assert!(r.confidence < 0.1, "confidence={}", r.confidence);
    }

    #[test]
    fn too_short_input_is_rejected() {
        let r = cross_correlate(&[1.0], &[1.0], 100.0);
        assert_eq!(r.confidence, 0.0);
        assert_eq!(r.offset_seconds, 0.0);
    }

    #[test]
    fn envelope_rate_converts_correctly() {
        let audio = pulse(500, 300.0, 20.0);
        let gyro = pulse(500, 250.0, 20.0);

        // The same 50-sample lag becomes a different offset depending on the
        // envelope rate.
        let r100 = cross_correlate(&audio, &gyro, 100.0);
        let r50 = cross_correlate(&audio, &gyro, 50.0);
        assert!((r100.offset_seconds - 0.5).abs() < 0.03);
        assert!((r50.offset_seconds - 1.0).abs() < 0.06);
    }
}
