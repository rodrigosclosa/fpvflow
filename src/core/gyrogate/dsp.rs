// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2026 Rodrigo Sclosa

//! Signal processing of the Gyrogate repair, ported from o4fix `correction.py` and `quat.py`.
//!
//! The port follows the numpy/scipy operations step by step (including numpy's pairwise
//! summation and scipy's running mean), so results match o4fix to rounding.

use super::{ Burst, Correction, Quat4, Settings };

const DEG_PER_RAD: f64 = 180.0 / std::f64::consts::PI; // np.degrees
const RAD_PER_DEG: f64 = std::f64::consts::PI / 180.0; // np.radians

// remove_impulses() keyword defaults of o4fix.
const PURITY: f64 = 0.6;
const MEDIAN_WINDOW_MS: f64 = 61.0;
const PADDING_SAMPLES: usize = 3;
const MIN_WIDTH_MS: f64 = 2.0;
const MAX_WIDTH_MS: f64 = 25.0;
const MIN_AREA_DEG: f64 = 0.25;
const MERGE_GAP_MS: f64 = 80.0;
const NOISY_MERGE_GAP_MS: f64 = 120.0;
const EDGE_WINDOW_MS: f64 = 15.0;
// adaptive_lowpass() keyword defaults of o4fix.
const BUSY_LEVEL_DPS: f64 = 20.0;
const SMOOTHING_S: f64 = 0.5;
// Single-axis tremor (FPVFlow, not in o4fix): dense runs of single-axis deviations too small or
// too close together for remove_impulses. Tuned on a 2026 O4 Pro clip, where they sit on the roll
// axis, 20–56 °/s and 0.2–0.35° each, many per second, while calm stretches have none.
const TREMOR_FLOOR_RATIO: f64 = 0.25; // of the impulse threshold: 15 °/s at the default 60
const TREMOR_WINDOW_S: f64 = 0.5;
const TREMOR_DENSITY_START: f64 = 10.0; // pure samples per second where the low-pass starts...
const TREMOR_DENSITY_FULL: f64 = 40.0; // ...and where it fully applies

/// Body angular rate (deg/s) between consecutive samples: `n` quaternions -> `n - 1` rates.
pub fn body_rates(quats: &[Quat4], rate_hz: f64) -> Vec<[f64; 3]> {
    quats.windows(2).map(|w| {
        let r = to_rotvec(&qmul(&conjugate(&w[0]), &w[1]));
        [r[0] * DEG_PER_RAD * rate_hz, r[1] * DEG_PER_RAD * rate_hz, r[2] * DEG_PER_RAD * rate_hz]
    }).collect()
}

/// Integrate body rates (deg/s) from `q0`: `n` rates -> `n + 1` quaternions. Inverse of `body_rates`.
pub fn integrate(q0: Quat4, rates: &[[f64; 3]], rate_hz: f64) -> Vec<Quat4> {
    let mut out = Vec::with_capacity(rates.len() + 1);
    let mut q = normalized(&q0);
    out.push(q);
    for r in rates {
        let step = from_rotvec(&[r[0] * RAD_PER_DEG / rate_hz, r[1] * RAD_PER_DEG / rate_hz, r[2] * RAD_PER_DEG / rate_hz]);
        q = normalized(&qmul(&q, &step));
        out.push(q);
    }
    out
}

/// Collapse consecutive stored duplicates (o4fix `AttitudeTrack.unique()`): returns the unique
/// values and, for every entry, the index of its unique value. Unlike o4fix, an entry equal to the
/// negation of the previous one also counts as a duplicate (same rotation).
pub fn dedup(quats: &[Quat4]) -> (Vec<Quat4>, Vec<usize>) {
    let mut unique = Vec::new();
    let mut index = Vec::with_capacity(quats.len());
    for (i, q) in quats.iter().enumerate() {
        let duplicate = i > 0 && {
            let p = &quats[i - 1];
            q == p || q.iter().zip(p).all(|(a, b)| *a == -*b)
        };
        if !duplicate {
            unique.push(*q);
        }
        index.push(unique.len() - 1);
    }
    (unique, index)
}

/// Remove false single-axis impulse bursts from body rates. Returns the cleaned rates and the bursts.
pub fn remove_impulses(rates: &[[f64; 3]], rate_hz: f64, threshold_dps: f64) -> (Vec<[f64; 3]>, Vec<Burst>) {
    let mut cleaned = rates.to_vec();
    let mut bursts = Vec::new();
    let n = rates.len();
    if n == 0 {
        return (cleaned, bursts);
    }
    let window = (MEDIAN_WINDOW_MS * rate_hz / 1000.0) as usize | 1;
    let cols: [Vec<f64>; 3] = std::array::from_fn(|a| rates.iter().map(|r| r[a]).collect());
    let baseline: [Vec<f64>; 3] = std::array::from_fn(|a| running_median(&cols[a], window));
    let deviation: [Vec<f64>; 3] = std::array::from_fn(|a| cols[a].iter().zip(&baseline[a]).map(|(x, b)| (x - b).abs()).collect());
    let merge_gap = (MERGE_GAP_MS * rate_hz / 1000.0) as isize;
    let noisy_merge_gap = (NOISY_MERGE_GAP_MS * rate_hz / 1000.0) as isize;
    let edge = ((EDGE_WINDOW_MS * rate_hz / 1000.0) as usize).max(1);
    let last = n - 1;

    for axis in 0..3 {
        let (col, base, dev) = (&cols[axis], &baseline[axis], &deviation[axis]);
        let (o1, o2) = ((axis + 1) % 3, (axis + 2) % 3);

        // Runs of impulse samples, split where the index gap exceeds 2.
        let mut runs: Vec<(usize, usize)> = Vec::new();
        for i in 0..n {
            let other = deviation[o1][i].max(deviation[o2][i]);
            if dev[i] > threshold_dps && other < PURITY * dev[i] {
                match runs.last_mut() {
                    Some(run) if i - run.1 <= 2 => run.1 = i,
                    _ => runs.push((i, i)),
                }
            }
        }

        let mut spans: Vec<(usize, usize)> = Vec::new();
        for (mut start, mut end) in runs {
            while start > 0 && dev[start - 1] > threshold_dps / 5.0 {
                start -= 1;
            }
            while end < last && dev[end + 1] > threshold_dps / 5.0 {
                end += 1;
            }
            let width_ms = (end - start + 1) as f64 * 1000.0 / rate_hz;
            let area_deg = pairwise_sum(&(start..=end).map(|i| col[i] - base[i]).collect::<Vec<_>>()) / rate_hz;
            if MIN_WIDTH_MS <= width_ms && width_ms <= MAX_WIDTH_MS && area_deg.abs() >= MIN_AREA_DEG {
                spans.push((start.saturating_sub(PADDING_SAMPLES), (end + PADDING_SAMPLES).min(last)));
            }
        }
        spans.sort();

        let mut merged: Vec<(usize, usize)> = Vec::new();
        for (start, end) in spans {
            if let Some(m) = merged.last_mut() {
                // rates[merged_end : start + 1], empty when the spans overlap.
                let gap_rates = if start + 1 > m.1 { &col[m.1..=start] } else { &col[0..0] };
                if mergeable(gap_rates, merge_gap, noisy_merge_gap, threshold_dps) {
                    m.1 = m.1.max(end);
                    continue;
                }
            }
            merged.push((start, end));
        }

        for (start, end) in merged {
            let rate_before = median(&col[start.saturating_sub(edge)..start]);
            let rate_after = median(&col[(end + 1).min(n)..(end + 1 + edge).min(n)]);
            let (rate_before, rate_after) = match (rate_before, rate_after) {
                (None, None) => continue,
                (Some(b), None) => (b, b),
                (None, Some(a)) => (a, a),
                (Some(b), Some(a)) => (b, a),
            };
            let replacement = linspace(rate_before, rate_after, end - start + 1);
            let removed: Vec<f64> = (start..=end).map(|i| col[i] - replacement[i - start]).collect();
            bursts.push(Burst { start_sample: start, end_sample: end, axis, area_deg: pairwise_sum(&removed) / rate_hz });
            for (i, v) in (start..=end).zip(replacement) {
                cleaned[i][axis] = v;
            }
        }
    }
    (cleaned, bursts)
}

/// Bursts found by `remove_impulses` alone, for a summary at load time.
pub fn detect(quats: &[Quat4], rate_hz: f64, settings: &Settings) -> Vec<Burst> {
    if quats.len() < 2 || !valid(quats) {
        return Vec::new();
    }
    remove_impulses(&body_rates(quats, rate_hz), rate_hz, settings.impulse_threshold_dps).1
}

/// Seconds of single-axis tremor that `repair` would low-pass, for a summary at load time.
pub fn detect_tremor(quats: &[Quat4], rate_hz: f64, settings: &Settings) -> f64 {
    if quats.len() < 2 || !valid(quats) {
        return 0.0;
    }
    tremor_weights(&body_rates(quats, rate_hz), rate_hz, settings).map_or(0.0, |w| tremor_seconds(&w, rate_hz))
}

/// o4fix `correct()`: the whole pipeline, applied even when no burst is found.
/// Fewer than 2 samples are returned unchanged (o4fix raises).
pub fn correct(quats: &[Quat4], rate_hz: f64, settings: &Settings) -> Correction {
    if quats.len() < 2 {
        return Correction { quats: quats.to_vec(), bursts: Vec::new(), tremor_seconds: 0.0 };
    }
    let rates = body_rates(quats, rate_hz);
    let (cleaned, bursts) = remove_impulses(&rates, rate_hz, settings.impulse_threshold_dps);
    let filtered = finish(&rates, cleaned, rate_hz, settings);
    Correction { quats: integrate(quats[0], &filtered, rate_hz), bursts, tremor_seconds: 0.0 }
}

/// FPVFlow policy: `None` when neither a burst nor tremor is found, so clean footage stays
/// bit-for-bit untouched (o4fix low-passes every clip). With bursts, the same result as
/// `correct()`; then the tremor, if any, is low-passed on its axis. `None` too when the track
/// holds an invalid quaternion.
pub fn repair(quats: &[Quat4], rate_hz: f64, settings: &Settings) -> Option<Correction> {
    if quats.len() < 2 || !valid(quats) {
        return None;
    }
    let rates = body_rates(quats, rate_hz);
    let (cleaned, bursts) = remove_impulses(&rates, rate_hz, settings.impulse_threshold_dps);
    let tremor = tremor_weights(&rates, rate_hz, settings);
    if bursts.is_empty() && tremor.is_none() {
        return None;
    }
    let mut filtered = if bursts.is_empty() { rates } else { finish(&rates, cleaned, rate_hz, settings) };
    let mut tremor_s = 0.0;
    if let Some(weights) = tremor {
        tremor_s = tremor_seconds(&weights, rate_hz);
        filtered = remove_tremor(&filtered, &weights, rate_hz, settings.tremor_lowpass_hz);
    }
    Some(Correction { quats: integrate(quats[0], &filtered, rate_hz), bursts, tremor_seconds: tremor_s })
}

/// o4fix `dji.read_track` rejects a track with a non-finite quaternion or one of norm < 0.5; one
/// such value would otherwise turn the whole filtered output into NaN.
fn valid(quats: &[Quat4]) -> bool {
    quats.iter().all(|q| q.iter().all(|v| v.is_finite()) && q.iter().map(|v| v * v).sum::<f64>() >= 0.25)
}

/// The rest of `correct()` after impulse removal, up to the filtered rates.
fn finish(rates: &[[f64; 3]], mut cleaned: Vec<[f64; 3]>, rate_hz: f64, settings: &Settings) -> Vec<[f64; 3]> {
    let removed: Vec<[f64; 3]> = rates.iter().zip(&cleaned).map(|(r, c)| [r[0] - c[0], r[1] - c[1], r[2] - c[2]]).collect();
    if settings.pullback_tau_s != 0.0 {
        cleaned = remove_pullback(&cleaned, &removed, rate_hz, settings.pullback_tau_s);
    }
    if settings.lowpass_hz != 0.0 && settings.busy_lowpass_hz != 0.0 {
        cleaned = adaptive_lowpass(&cleaned, &removed, rate_hz, settings.lowpass_hz, settings.busy_lowpass_hz);
    } else {
        cleaned = lowpass(&cleaned, rate_hz, settings.lowpass_hz);
    }
    cleaned
}

/// Per axis, how much of the tremor low-pass applies to each sample (0 to 1), or `None` when no
/// axis reaches half weight anywhere. A sample is "pure" when its axis deviates from the 61 ms
/// running median by more than the floor while the other two stay under `PURITY` of it, as in
/// `remove_impulses`; the weight ramps with how many pure samples fall in a 0.5 s window.
fn tremor_weights(rates: &[[f64; 3]], rate_hz: f64, settings: &Settings) -> Option<[Vec<f64>; 3]> {
    let window = (TREMOR_WINDOW_S * rate_hz) as usize;
    if settings.tremor_lowpass_hz == 0.0 || window == 0 || rates.len() < window {
        return None;
    }
    let floor = settings.impulse_threshold_dps * TREMOR_FLOOR_RATIO;
    let median_window = (MEDIAN_WINDOW_MS * rate_hz / 1000.0) as usize | 1;
    let deviation: [Vec<f64>; 3] = std::array::from_fn(|a| {
        let col: Vec<f64> = rates.iter().map(|r| r[a]).collect();
        col.iter().zip(running_median(&col, median_window)).map(|(x, b)| (x - b).abs()).collect()
    });
    let weights: [Vec<f64>; 3] = std::array::from_fn(|a| {
        let (dev, o1, o2) = (&deviation[a], &deviation[(a + 1) % 3], &deviation[(a + 2) % 3]);
        let pure: Vec<f64> = (0..rates.len()).map(|i| (dev[i] > floor && o1[i].max(o2[i]) < PURITY * dev[i]) as u8 as f64).collect();
        let ramp: Vec<f64> = uniform_filter1d(&pure, window).into_iter()
            .map(|share| ((share * rate_hz - TREMOR_DENSITY_START) / (TREMOR_DENSITY_FULL - TREMOR_DENSITY_START)).clamp(0.0, 1.0))
            .collect();
        uniform_filter1d(&ramp, window)
    });
    weights.iter().flatten().any(|&w| w >= 0.5).then_some(weights)
}

/// Seconds where the tremor low-pass applies at least halfway on some axis.
fn tremor_seconds(weights: &[Vec<f64>; 3], rate_hz: f64) -> f64 {
    (0..weights[0].len()).filter(|&i| weights.iter().any(|w| w[i] >= 0.5)).count() as f64 / rate_hz
}

/// Blend each axis towards its low-pass at `cutoff_hz` by the tremor weight.
fn remove_tremor(rates: &[[f64; 3]], weights: &[Vec<f64>; 3], rate_hz: f64, cutoff_hz: f64) -> Vec<[f64; 3]> {
    let sos = butter4_sos(cutoff_hz / (rate_hz / 2.0));
    let mut out = rates.to_vec();
    for a in 0..3 {
        if weights[a].iter().all(|&w| w == 0.0) {
            continue;
        }
        let col: Vec<f64> = rates.iter().map(|r| r[a]).collect();
        for ((o, smooth), w) in out.iter_mut().zip(sosfiltfilt(&sos, &col)).zip(&weights[a]) {
            o[a] = (1.0 - w) * o[a] + w * smooth;
        }
    }
    out
}

/// Bursts merge across short gaps, and across longer ones only when the gap jitters sample to sample.
fn mergeable(gap_rates: &[f64], merge_gap: isize, noisy_merge_gap: isize, threshold_dps: f64) -> bool {
    let gap = gap_rates.len() as isize - 1;
    if gap <= merge_gap {
        return true;
    }
    let diffs: Vec<f64> = gap_rates.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
    gap <= noisy_merge_gap && pairwise_sum(&diffs) / diffs.len() as f64 > threshold_dps / 8.0
}

/// Add back the fusion's pull-back of the removed steps: `y[n] = dt*x[n] + exp(-dt/tau)*y[n-1]`
/// over `removed`, then `rates + y / tau` (scipy `lfilter([dt], [1, -exp(-dt/tau)])`).
fn remove_pullback(rates: &[[f64; 3]], removed: &[[f64; 3]], rate_hz: f64, tau_s: f64) -> Vec<[f64; 3]> {
    let dt = 1.0 / rate_hz;
    let decay = (-dt / tau_s).exp();
    let mut y = [0.0f64; 3];
    rates.iter().zip(removed).map(|(r, x)| {
        let mut out = *r;
        for a in 0..3 {
            y[a] = x[a] * dt + decay * y[a];
            out[a] = r[a] + y[a] / tau_s;
        }
        out
    }).collect()
}

/// Zero-phase 4th-order Butterworth low-pass (scipy `sosfiltfilt`); no-op when `cutoff_hz` is 0.
fn lowpass(rates: &[[f64; 3]], rate_hz: f64, cutoff_hz: f64) -> Vec<[f64; 3]> {
    if cutoff_hz == 0.0 {
        return rates.to_vec();
    }
    let sos = butter4_sos(cutoff_hz / (rate_hz / 2.0));
    let mut out = rates.to_vec();
    for a in 0..3 {
        let col: Vec<f64> = rates.iter().map(|r| r[a]).collect();
        for (o, v) in out.iter_mut().zip(sosfiltfilt(&sos, &col)) {
            o[a] = v;
        }
    }
    out
}

/// Low-pass at `cutoff_hz`, blending towards `busy_cutoff_hz` where impulse removal took out a lot.
fn adaptive_lowpass(rates: &[[f64; 3]], removed: &[[f64; 3]], rate_hz: f64, cutoff_hz: f64, busy_cutoff_hz: f64) -> Vec<[f64; 3]> {
    let window = ((SMOOTHING_S * rate_hz) as usize).max(1);
    let norm: Vec<f64> = removed.iter().map(|r| (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt()).collect();
    let weight: Vec<f64> = uniform_filter1d(&norm, window).into_iter()
        .map(|level| ((level - BUSY_LEVEL_DPS / 4.0) / (BUSY_LEVEL_DPS * 3.0 / 4.0)).clamp(0.0, 1.0))
        .collect();
    let weight = uniform_filter1d(&weight, window);
    let calm = lowpass(rates, rate_hz, cutoff_hz);
    let busy = lowpass(rates, rate_hz, busy_cutoff_hz);
    calm.iter().zip(&busy).zip(&weight).map(|((c, b), w)| {
        [(1.0 - w) * c[0] + w * b[0], (1.0 - w) * c[1] + w * b[1], (1.0 - w) * c[2] + w * b[2]]
    }).collect()
}

// ---------------------------------------------------------------------------------------------
// Quaternion helpers (o4fix quat.py)

fn qmul(a: &Quat4, b: &Quat4) -> Quat4 {
    let [w1, x1, y1, z1] = *a;
    let [w2, x2, y2, z2] = *b;
    [
        w1 * w2 - x1 * x2 - y1 * y2 - z1 * z2,
        w1 * x2 + x1 * w2 + y1 * z2 - z1 * y2,
        w1 * y2 - x1 * z2 + y1 * w2 + z1 * x2,
        w1 * z2 + x1 * y2 - y1 * x2 + z1 * w2,
    ]
}

fn conjugate(q: &Quat4) -> Quat4 {
    [q[0], -q[1], -q[2], -q[3]]
}

fn normalized(q: &Quat4) -> Quat4 {
    let n = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    [q[0] / n, q[1] / n, q[2] / n, q[3] / n]
}

/// Rotation vector (rad) of a unit quaternion.
fn to_rotvec(q: &Quat4) -> [f64; 3] {
    let w1 = q[0] + 1e-30;
    let sign = if w1 > 0.0 { 1.0 } else if w1 < 0.0 { -1.0 } else { 0.0 }; // np.sign
    let (w, v) = (q[0] * sign, [q[1] * sign, q[2] * sign, q[3] * sign]);
    let s = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    let angle = 2.0 * s.atan2(w);
    let m = s.max(1e-15);
    [v[0] / m * angle, v[1] / m * angle, v[2] / m * angle]
}

/// Unit quaternion from a rotation vector (rad).
fn from_rotvec(r: &[f64; 3]) -> Quat4 {
    let angle = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
    let k = if angle > 1e-15 { (angle / 2.0).sin() / angle.max(1e-15) } else { 0.5 };
    [(angle / 2.0).cos(), r[0] * k, r[1] * k, r[2] * k]
}

// ---------------------------------------------------------------------------------------------
// numpy / scipy equivalents

/// numpy's pairwise summation (`np.sum` of a contiguous float64 array), for bit-exact sums.
fn pairwise_sum(a: &[f64]) -> f64 {
    let n = a.len();
    if n < 8 {
        let mut res = 0.0;
        for v in a {
            res += v;
        }
        res
    } else if n <= 128 {
        let mut r = [0.0; 8];
        r.copy_from_slice(&a[..8]);
        let mut i = 8;
        while i < n - n % 8 {
            for j in 0..8 {
                r[j] += a[i + j];
            }
            i += 8;
        }
        let mut res = ((r[0] + r[1]) + (r[2] + r[3])) + ((r[4] + r[5]) + (r[6] + r[7]));
        for v in &a[i..] {
            res += v;
        }
        res
    } else {
        let n2 = n / 2;
        let n2 = n2 - n2 % 8;
        pairwise_sum(&a[..n2]) + pairwise_sum(&a[n2..])
    }
}

/// `np.median`; `None` for an empty slice.
fn median(a: &[f64]) -> Option<f64> {
    if a.is_empty() {
        return None;
    }
    let mut s = a.to_vec();
    s.sort_by(f64::total_cmp);
    let h = s.len() / 2;
    Some(if s.len() % 2 == 1 { s[h] } else { (s[h - 1] + s[h]) / 2.0 })
}

/// `np.linspace(start, stop, num)`.
fn linspace(start: f64, stop: f64, num: usize) -> Vec<f64> {
    if num < 2 {
        return vec![start; num];
    }
    let div = (num - 1) as f64;
    let delta = stop - start;
    let step = delta / div;
    let mut y: Vec<f64> = (0..num).map(|i| {
        let i = i as f64;
        if step == 0.0 { i / div * delta + start } else { i * step + start }
    }).collect();
    y[num - 1] = stop;
    y
}

/// scipy `median_filter(x, size=window, mode='nearest')` for an odd window, with a sorted
/// sliding window (O(n * window) moves, no allocation per sample).
fn running_median(x: &[f64], window: usize) -> Vec<f64> {
    let n = x.len() as isize;
    let h = (window / 2) as isize;
    let at = |i: isize| x[i.clamp(0, n - 1) as usize];
    let mut sorted: Vec<f64> = (-h..=h).map(at).collect();
    sorted.sort_by(f64::total_cmp);
    let mut out = Vec::with_capacity(x.len());
    for i in 0..n {
        out.push(sorted[h as usize]);
        let (old, new) = (at(i - h), at(i + h + 1));
        if i + 1 < n && old.to_bits() != new.to_bits() {
            if let Ok(p) = sorted.binary_search_by(|v| v.total_cmp(&old)) {
                sorted.remove(p);
            }
            let p = sorted.partition_point(|v| v.total_cmp(&new).is_lt());
            sorted.insert(p, new);
        }
    }
    out
}

/// scipy `uniform_filter1d(x, size, mode='nearest')`, with its running sum. For output `i` the
/// window covers `x[i - size/2 ..= i + size - size/2 - 1]` (checked bit-exact against scipy 1.18).
fn uniform_filter1d(x: &[f64], size: usize) -> Vec<f64> {
    let n = x.len() as isize;
    if n == 0 {
        return Vec::new();
    }
    let left = (size / 2) as isize;
    let at = |i: isize| x[(i - left).clamp(0, n - 1) as usize]; // index into the extended line
    let mut tmp = 0.0;
    for i in 0..size as isize {
        tmp += at(i);
    }
    let mut out = Vec::with_capacity(x.len());
    out.push(tmp / size as f64);
    for i in 1..n {
        tmp += at(i + size as isize - 1) - at(i - 1);
        out.push(tmp / size as f64);
    }
    out
}

type Sos = [[f64; 6]; 2];
type Complex = (f64, f64);

fn c_mul(a: Complex, b: Complex) -> Complex {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

/// numpy's complex division (Smith's algorithm).
fn c_div(a: Complex, b: Complex) -> Complex {
    if b.0.abs() >= b.1.abs() {
        let rat = b.1 / b.0;
        let scl = 1.0 / (b.0 + b.1 * rat);
        ((a.0 + a.1 * rat) * scl, (a.1 - a.0 * rat) * scl)
    } else {
        let rat = b.0 / b.1;
        let scl = 1.0 / (b.1 + b.0 * rat);
        ((a.0 * rat + a.1) * scl, (a.1 * rat - a.0) * scl)
    }
}

/// scipy `butter(4, wn, output='sos')`: analog prototype, prewarped bilinear transform, sections
/// ordered as `zpk2sos` does (lower-Q pole pair first, gain in the first section).
fn butter4_sos(wn: f64) -> Sos {
    let fs = 2.0;
    let warped = 2.0 * fs * (std::f64::consts::PI * wn / fs).tan();
    // Prototype poles -exp(j*pi*m/8), m = -3, -1, 1, 3, scaled to `warped` (lp2lp_zpk).
    let p: Vec<Complex> = [-3.0f64, -1.0, 1.0, 3.0].iter().map(|m| {
        let t = std::f64::consts::PI * m / 8.0;
        (-t.cos() * warped, -t.sin() * warped)
    }).collect();
    let k = warped.powf(4.0);
    // bilinear_zpk with fs = 2: four zeros at -1.
    let fs2 = 2.0 * fs;
    let mut denom = (1.0, 0.0);
    for &pi in &p {
        denom = c_mul(denom, (fs2 - pi.0, -pi.1));
    }
    let k_z = k * c_div((1.0, 0.0), denom).0;
    let pz: Vec<Complex> = p.iter().map(|&pi| c_div((fs2 + pi.0, pi.1), (fs2 - pi.0, -pi.1))).collect();
    let section = |z: Complex, gain: f64| [gain, 2.0 * gain, gain, 1.0, -(z.0 + z.0), z.0 * z.0 + z.1 * z.1];
    [section(pz[2], k_z), section(pz[3], 1.0)]
}

/// scipy `sosfilt_zi`: steady-state initial conditions of every section for a unit step.
fn sosfilt_zi(sos: &Sos) -> [[f64; 2]; 2] {
    let mut zi = [[0.0; 2]; 2];
    let mut scale = 1.0;
    for (s, z) in sos.iter().zip(zi.iter_mut()) {
        let (b, a) = (&s[..3], &s[3..]);
        // lfilter_zi: solve [[1 + a1, -1], [a2, 1]] zi = [b1 - a1*b0, b2 - a2*b0] by LU with partial pivoting.
        let m = [[1.0 + a[1], -1.0], [a[2], 1.0]];
        let rhs = [b[1] - a[1] * b[0], b[2] - a[2] * b[0]];
        let (r0, r1) = if m[1][0].abs() > m[0][0].abs() { (1, 0) } else { (0, 1) };
        let l = m[r1][0] * (1.0 / m[r0][0]);
        let u11 = m[r1][1] - l * m[r0][1];
        let y1 = rhs[r1] - l * rhs[r0];
        let x1 = y1 / u11;
        let x0 = (rhs[r0] - x1 * m[r0][1]) / m[r0][0];
        *z = [scale * x0, scale * x1];
        scale *= (b[0] + b[1] + b[2]) / (a[0] + a[1] + a[2]);
    }
    zi
}

/// scipy `sosfilt` in place (direct form II transposed, same operation order).
fn sosfilt(sos: &Sos, x: &mut [f64], mut zi: [[f64; 2]; 2]) {
    for v in x.iter_mut() {
        let mut cur = *v;
        for (s, z) in sos.iter().zip(zi.iter_mut()) {
            let new = s[0] * cur + z[0];
            z[0] = s[1] * cur - s[4] * new + z[1];
            z[1] = s[2] * cur - s[5] * new;
            cur = new;
        }
        *v = cur;
    }
}

/// scipy `sosfiltfilt(sos, x)` with its defaults: odd extension of 15 samples
/// (3 * (2 * sections + 1)) and `sosfilt_zi` initial conditions.
fn sosfiltfilt(sos: &Sos, x: &[f64]) -> Vec<f64> {
    let edge = 3 * (2 * sos.len() + 1);
    let n = x.len();
    if n <= edge {
        // scipy raises here; such short signals are returned unfiltered.
        return x.to_vec();
    }
    let mut ext = Vec::with_capacity(n + 2 * edge);
    ext.extend((1..=edge).rev().map(|i| 2.0 * x[0] - x[i]));
    ext.extend_from_slice(x);
    ext.extend((0..edge).map(|j| 2.0 * x[n - 1] - x[n - 2 - j]));

    let zi = sosfilt_zi(sos);
    let scaled = |v: f64| zi.map(|z| [z[0] * v, z[1] * v]);
    let zi0 = scaled(ext[0]);
    sosfilt(sos, &mut ext, zi0);
    ext.reverse();
    let zi0 = scaled(ext[0]);
    sosfilt(sos, &mut ext, zi0);
    ext.reverse();
    ext[edge..edge + n].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        bursts: Vec<Burst>,
        input: Vec<Quat4>,
        expected: Vec<Quat4>,
    }

    fn read_u32(d: &[u8], pos: &mut usize) -> u32 {
        let v = u32::from_le_bytes(d[*pos..*pos + 4].try_into().unwrap());
        *pos += 4;
        v
    }
    fn read_f64(d: &[u8], pos: &mut usize) -> f64 {
        let v = f64::from_le_bytes(d[*pos..*pos + 8].try_into().unwrap());
        *pos += 8;
        v
    }
    fn read_fixture(d: &[u8], pos: &mut usize) -> Fixture {
        let n = read_u32(d, pos) as usize;
        let nbursts = read_u32(d, pos) as usize;
        let bursts = (0..nbursts).map(|_| {
            let start_sample = read_u32(d, pos) as usize;
            let end_sample = read_u32(d, pos) as usize;
            let axis = read_u32(d, pos) as usize;
            Burst { start_sample, end_sample, axis, area_deg: read_f64(d, pos) }
        }).collect();
        let quats = |pos: &mut usize| (0..n).map(|_| [read_f64(d, pos), read_f64(d, pos), read_f64(d, pos), read_f64(d, pos)]).collect::<Vec<Quat4>>();
        let input = quats(pos);
        let expected = quats(pos);
        Fixture { bursts, input, expected }
    }

    fn assert_parity(f: &Fixture) {
        let c = correct(&f.input, 1000.0, &Settings::default());
        assert_eq!(c.bursts.len(), f.bursts.len(), "bursts: {:?}", c.bursts);
        for (b, e) in c.bursts.iter().zip(&f.bursts) {
            assert_eq!((b.start_sample, b.end_sample, b.axis), (e.start_sample, e.end_sample, e.axis));
            assert!((b.area_deg - e.area_deg).abs() < 1e-9, "{b:?} vs {e:?}");
        }
        assert_eq!(c.quats.len(), f.expected.len());
        let mut max_err = 0.0f64;
        for (q, e) in c.quats.iter().zip(&f.expected) {
            let sign = if q.iter().zip(e).map(|(a, b)| a * b).sum::<f64>() < 0.0 { -1.0 } else { 1.0 };
            for i in 0..4 {
                max_err = max_err.max((q[i] * sign - e[i]).abs());
            }
        }
        println!("n = {}, bursts = {}, max quaternion error = {max_err:e}", f.input.len(), c.bursts.len());
        assert!(max_err < 1e-9);
    }

    #[test]
    fn gyrogate_golden_o4fix_synthetic() {
        let d = include_bytes!("testdata/o4fix_synthetic.bin");
        let f = read_fixture(d, &mut 0);
        let expected = [(2397, 2420, 0, 2.2809435645855403), (1097, 1190, 1, -0.32785923975593334), (497, 512, 2, 0.9637562203291227)];
        assert_eq!(f.bursts.len(), expected.len());
        for (b, e) in f.bursts.iter().zip(expected) {
            assert_eq!((b.start_sample, b.end_sample, b.axis, b.area_deg), e);
        }
        assert_parity(&f);
    }

    #[test]
    fn gyrogate_golden_o4fix_edges() {
        let d = include_bytes!("testdata/o4fix_edges.bin");
        let mut pos = 0;
        let ncases = read_u32(d, &mut pos);
        assert_eq!(ncases, 2);
        for _ in 0..ncases {
            assert_parity(&read_fixture(d, &mut pos));
        }
        assert_eq!(pos, d.len());
    }

    /// Smooth motion with noise, no false bursts.
    fn clean_rates(n: usize) -> Vec<[f64; 3]> {
        let mut seed = 12345u64;
        let mut noise = move || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((seed >> 11) as f64 / (1u64 << 53) as f64 - 0.5) * 4.0
        };
        (0..n).map(|i| {
            let t = i as f64 / 1000.0;
            [
                30.0 * (2.0 * std::f64::consts::PI * 0.6 * t).sin() + noise(),
                -20.0 * (2.0 * std::f64::consts::PI * 0.3 * t + 1.0).cos() + noise(),
                10.0 * (2.0 * std::f64::consts::PI * 1.1 * t).sin() + noise(),
            ]
        }).collect()
    }

    #[test]
    fn gyrogate_clean_signal_untouched() {
        let quats = integrate([0.8, 0.2, -0.4, 0.4], &clean_rates(5000), 1000.0);
        let settings = Settings::default();
        assert!(detect(&quats, 1000.0, &settings).is_empty());
        assert_eq!(detect_tremor(&quats, 1000.0, &settings), 0.0);
        assert!(repair(&quats, 1000.0, &settings).is_none());
        // correct() still low-passes, as o4fix does.
        assert!(correct(&quats, 1000.0, &settings).bursts.is_empty());
        assert!(repair(&quats[..1], 1000.0, &settings).is_none());
        assert_eq!(correct(&quats[..1], 1000.0, &settings).quats, &quats[..1]);
    }

    /// `clean_rates` with a 2 s run of small pure spikes on axis 1 from sample 2000: half-sines
    /// of 35 °/s and 8 ms every 40 ms, alternating in sign, below the 60 °/s burst threshold.
    /// `axes` picks which axes carry the spikes.
    fn rates_with_tremor(axes: [bool; 3]) -> Vec<[f64; 3]> {
        let mut rates = clean_rates(6000);
        for (k, start) in (2000..4000).step_by(40).enumerate() {
            let sign = if k % 2 == 0 { 1.0 } else { -1.0 };
            for j in 0..8 {
                for a in (0..3).filter(|&a| axes[a]) {
                    rates[start + j][a] += sign * 35.0 * (std::f64::consts::PI * (j as f64 + 0.5) / 8.0).sin();
                }
            }
        }
        rates
    }

    #[test]
    fn gyrogate_tremor_is_low_passed_on_its_axis_only() {
        let truth = clean_rates(6000);
        let rates = rates_with_tremor([false, true, false]);
        let quats = integrate([0.8, 0.2, -0.4, 0.4], &rates, 1000.0);
        let settings = Settings::default();
        assert!(detect(&quats, 1000.0, &settings).is_empty(), "the spikes are below the burst threshold");
        let c = repair(&quats, 1000.0, &settings).expect("tremor found");
        assert!(c.bursts.is_empty());
        assert!((1.5..=2.5).contains(&c.tremor_seconds), "tremor seconds {}", c.tremor_seconds);
        assert_eq!(detect_tremor(&quats, 1000.0, &settings), c.tremor_seconds);

        let out = body_rates(&c.quats, 1000.0);
        let rms = |a: &[[f64; 3]], b: &[[f64; 3]], axis: usize, r: std::ops::Range<usize>| {
            (r.clone().map(|i| (a[i][axis] - b[i][axis]).powi(2)).sum::<f64>() / r.len() as f64).sqrt()
        };
        let (before, after) = (rms(&rates, &truth, 1, 2200..3800), rms(&out, &truth, 1, 2200..3800));
        println!("tremor axis error vs truth: {before:.2} -> {after:.2} °/s");
        assert!(after < 0.4 * before);
        // The other axes, and the tremor axis away from the tremor, are left as they were.
        for (axis, range) in [(0, 0..6000), (2, 0..6000), (1, 0..1000), (1, 5000..5999)] {
            let worst = range.clone().map(|i| (out[i][axis] - rates[i][axis]).abs()).fold(0.0, f64::max);
            assert!(worst < 1e-6, "axis {axis} {range:?} changed by {worst} °/s");
        }
    }

    #[test]
    fn gyrogate_tremor_needs_a_single_axis() {
        // The same spikes on all three axes look like real vibration, not the sensor defect.
        let quats = integrate([0.8, 0.2, -0.4, 0.4], &rates_with_tremor([true, true, true]), 1000.0);
        assert_eq!(detect_tremor(&quats, 1000.0, &Settings::default()), 0.0);
        assert!(repair(&quats, 1000.0, &Settings::default()).is_none());
    }

    #[test]
    fn gyrogate_tremor_can_be_turned_off() {
        let quats = integrate([0.8, 0.2, -0.4, 0.4], &rates_with_tremor([false, true, false]), 1000.0);
        let off = Settings { tremor_lowpass_hz: 0.0, ..Default::default() };
        assert_eq!(detect_tremor(&quats, 1000.0, &off), 0.0);
        assert!(repair(&quats, 1000.0, &off).is_none());
    }

    #[test]
    fn gyrogate_repair_matches_correct() {
        let f = read_fixture(include_bytes!("testdata/o4fix_synthetic.bin"), &mut 0);
        // The fixture's jittery 94 ms burst on axis 1 also counts as tremor: compare without it.
        let settings = Settings { tremor_lowpass_hz: 0.0, ..Default::default() };
        let c = correct(&f.input, 1000.0, &settings);
        let r = repair(&f.input, 1000.0, &settings).expect("bursts present");
        assert_eq!(r.tremor_seconds, 0.0);
        assert_eq!(r.quats, c.quats);
        assert_eq!(r.bursts, c.bursts);
        assert_eq!(detect(&f.input, 1000.0, &settings), c.bursts);
    }

    #[test]
    fn gyrogate_invalid_input_rejected() {
        let f = read_fixture(include_bytes!("testdata/o4fix_synthetic.bin"), &mut 0);
        let settings = Settings::default();
        for (i, bad) in [(100, [f64::NAN; 4]), (100, [f64::INFINITY, 0.0, 0.0, 0.0]), (0, [0.0; 4]), (100, [0.0; 4])] {
            let mut quats = f.input.clone();
            quats[i] = bad;
            assert!(repair(&quats, 1000.0, &settings).is_none(), "{bad:?} at {i}");
            assert!(detect(&quats, 1000.0, &settings).is_empty(), "{bad:?} at {i}");
        }
    }

    #[test]
    fn gyrogate_rates_round_trip() {
        let rates = clean_rates(3000);
        let quats = integrate([0.3, -0.5, 0.1, 0.8], &rates, 1000.0);
        let back = body_rates(&quats, 1000.0);
        assert_eq!(back.len(), rates.len());
        let max_err = rates.iter().zip(&back).flat_map(|(a, b)| (0..3).map(move |i| (a[i] - b[i]).abs())).fold(0.0, f64::max);
        assert!(max_err < 1e-12 * 1000.0 * DEG_PER_RAD, "max rate error {max_err:e}");
        let again = integrate(quats[0], &back, 1000.0);
        let max_q_err = quats.iter().zip(&again).flat_map(|(a, b)| (0..4).map(move |i| (a[i] - b[i]).abs())).fold(0.0, f64::max);
        assert!(max_q_err < 1e-12, "max quaternion error {max_q_err:e}");
    }

    #[test]
    fn gyrogate_dedup() {
        let a = [1.0, 0.0, 0.0, 0.0];
        let b = [0.5, 0.5, 0.5, 0.5];
        let nb = [-0.5, -0.5, -0.5, -0.5];
        let c = [0.0, 1.0, 0.0, 0.0];
        let (unique, index) = dedup(&[a, a, b, nb, b, c, c, a]);
        assert_eq!(unique, vec![a, b, c, a]);
        assert_eq!(index, vec![0, 0, 1, 1, 1, 2, 2, 3]);
        assert_eq!(dedup(&[]), (vec![], vec![]));
    }

    #[test]
    fn gyrogate_numpy_helpers() {
        assert_eq!(linspace(1.0, 3.0, 5), vec![1.0, 1.5, 2.0, 2.5, 3.0]);
        assert_eq!(median(&[3.0, 1.0, 2.0, 10.0]), Some(2.5));
        assert_eq!(median(&[]), None);
        assert_eq!(running_median(&[5.0, 1.0, 9.0, 2.0, 7.0], 3), vec![5.0, 5.0, 2.0, 7.0, 7.0]);
        // Even window: output i covers x[i - 2 ..= i + 1].
        let u = uniform_filter1d(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 4);
        assert_eq!(u, vec![1.25, 1.75, 2.5, 3.5, 4.5, 5.25]);
        // scipy butter(4, 20/500, output='sos')
        let sos = butter4_sos(20.0 / 500.0);
        let expected = [
            [1.32937288987529e-05, 2.65874577975058e-05, 1.32937288987529e-05, 1.0, -1.778313488139435, 0.7924474718329468],
            [1.0, 2.0, 1.0, 1.0, -1.8934156010225003, 0.9084644129492953],
        ];
        for (s, e) in sos.iter().zip(&expected) {
            for (a, b) in s.iter().zip(e) {
                assert!((a - b).abs() <= 1e-14 * b.abs(), "{sos:?}");
            }
        }
    }

    #[test]
    #[ignore]
    fn gyrogate_perf_20_min() {
        let n = 1_200_000;
        let mut rates = clean_rates(n);
        for i in (5000..n - 100).step_by(50_000) {
            for k in 0..10 {
                rates[i + k][i / 50_000 % 3] += 150.0 * (std::f64::consts::PI * (k as f64 + 0.5) / 10.0).sin();
            }
        }
        let quats = integrate([0.8, 0.2, -0.4, 0.4], &rates, 1000.0);
        let t = std::time::Instant::now();
        let c = correct(&quats, 1000.0, &Settings::default());
        println!("correct() on {n} samples: {:?}, {} bursts", t.elapsed(), c.bursts.len());
        assert_eq!(c.bursts.len(), 24);
    }
}
