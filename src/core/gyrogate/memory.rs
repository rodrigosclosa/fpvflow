// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2026 Rodrigo Sclosa

//! The repair applied to the quaternions loaded in memory, for the stabilization.
//!
//! telemetry-parser hands over every stored value of the O4 attitude track, each duplicate with
//! its own timestamp, already rotated into Gyroflow's frame: `q = (0,0,1,0) ⊗ q_raw ⊗ M`.
//! The repair works on body rates, which that constant mount rotation `M` only permutes, so
//! the same bursts are found as in the file and the result matches the corrected copy.

use super::{ dsp, Quat4, Settings, UNIQUE_RATE_HZ };
use crate::gyro_source::{ Quat64, TimeQuat };
use nalgebra::Quaternion;

/// What the detector finds in a clip, for the UI.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct Summary {
    pub bursts: usize,
    /// Sum of the absolute rotation of every burst, degrees.
    pub total_deg: f64,
    pub largest_deg: f64,
    /// Total length of the bursts, seconds.
    pub seconds: f64,
}

/// The unique attitude samples of `data`, and for every entry the index of its unique sample.
/// `None` when the track is not the 1 kHz attitude the repair is tuned for.
fn unique_samples(data: &TimeQuat) -> Option<(Vec<Quat4>, Vec<usize>)> {
    let stored: Vec<Quat4> = data.values().map(|q| [q.w, q.i, q.j, q.k]).collect();
    let (unique, index) = dsp::dedup(&stored);
    let (first, last) = (*data.keys().next()? as f64, *data.keys().next_back()? as f64);
    let span_s = (last - first) / 1_000_000.0;
    if unique.len() < 8 || span_s <= 0.0 {
        return None;
    }
    let rate = (unique.len() - 1) as f64 / span_s;
    if (rate / UNIQUE_RATE_HZ - 1.0).abs() > 0.1 {
        log::warn!("Gyrogate: attitude at {rate:.0} Hz, expected {UNIQUE_RATE_HZ} Hz; not repairing");
        return None;
    }
    Some((unique, index))
}

/// Count the false bursts without changing anything.
pub fn analyze(data: &TimeQuat, settings: &Settings) -> Option<Summary> {
    let (unique, _) = unique_samples(data)?;
    let bursts = dsp::detect(&unique, UNIQUE_RATE_HZ, settings);
    Some(Summary {
        bursts: bursts.len(),
        total_deg: bursts.iter().map(|b| b.area_deg.abs()).sum(),
        largest_deg: bursts.iter().map(|b| b.area_deg.abs()).fold(0.0, f64::max),
        seconds: bursts.iter().map(|b| (b.end_sample - b.start_sample + 1) as f64).sum::<f64>() / UNIQUE_RATE_HZ,
    })
}

/// Repair `data` in place. Returns the number of bursts removed; when none is found the data
/// is left untouched.
pub fn repair(data: &mut TimeQuat, settings: &Settings) -> usize {
    let Some((unique, index)) = unique_samples(data) else { return 0; };
    let Some(correction) = dsp::repair(&unique, UNIQUE_RATE_HZ, settings) else { return 0; };
    for (q, &i) in data.values_mut().zip(index.iter()) {
        let [w, x, y, z] = correction.quats[i];
        let mut new = Quaternion::new(w, x, y, z);
        // Keep each entry's sign, which telemetry-parser flips to keep the track continuous.
        if new.dot(q.quaternion()) < 0.0 {
            new = -new;
        }
        *q = Quat64::from_quaternion(new);
    }
    correction.bursts.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gyro_source::{ FileLoadOptions, GyroSource };
    use std::sync::{ Arc, atomic::AtomicBool };

    /// telemetry-parser's frame change: `(0,0,1,0) ⊗ q ⊗ (0.5,-0.5,-0.5,0.5)`.
    fn to_gyroflow(q: &Quat4) -> Quat64 {
        let q = Quaternion::new(q[0], q[1], q[2], q[3]);
        Quat64::from_quaternion(Quaternion::new(0.0, 0.0, 1.0, 0.0) * q * Quaternion::new(0.5, -0.5, -0.5, 0.5))
    }
    fn angle_deg(a: &Quat64, b: &Quat64) -> f64 {
        a.angle_to(b).to_degrees()
    }

    /// Attitude at 1 kHz with a smooth motion and one false 15 ms burst on the body Z axis.
    fn attitude_with_burst() -> Vec<Quat4> {
        let rates: Vec<[f64; 3]> = (0..3000).map(|i| {
            let t = i as f64 / 1000.0;
            let burst = if (1500..1515).contains(&i) { 150.0 * (std::f64::consts::PI * (i - 1500) as f64 / 15.0).sin() } else { 0.0 };
            [25.0 * (4.4 * t).sin(), -18.0 * (2.5 * t).sin(), 12.0 * (5.6 * t).cos() + burst]
        }).collect();
        dsp::integrate([0.9, 0.1, -0.3, 0.2], &rates, UNIQUE_RATE_HZ)
    }

    /// Every value stored twice, as the O4 does, at telemetry-parser's evenly spaced timestamps.
    fn stored_twice(unique: &[Quat4]) -> TimeQuat {
        unique.iter().enumerate().flat_map(|(i, q)| [(i as i64 * 1000, to_gyroflow(q)), (i as i64 * 1000 + 500, to_gyroflow(q))]).collect()
    }

    #[test]
    fn the_repair_does_not_depend_on_the_camera_frame() {
        let unique = attitude_with_burst();
        let expected = dsp::repair(&unique, UNIQUE_RATE_HZ, &Settings::default()).expect("the burst is found");
        let mut data = stored_twice(&unique);
        let summary = analyze(&data, &Settings::default()).unwrap();
        assert_eq!(summary.bursts, expected.bursts.len());
        assert_eq!(repair(&mut data, &Settings::default()), expected.bursts.len());
        let worst = data.values().enumerate().map(|(i, q)| angle_deg(q, &to_gyroflow(&expected.quats[i / 2]))).fold(0.0, f64::max);
        assert!(worst < 1e-9, "in-memory repair differs from the raw-frame repair by {worst}°");
    }

    #[test]
    fn clean_attitude_is_left_untouched() {
        let rates = vec![[20.0, -10.0, 5.0]; 3000];
        let mut data = stored_twice(&dsp::integrate([1.0, 0.0, 0.0, 0.0], &rates, UNIQUE_RATE_HZ));
        let before = data.clone();
        assert_eq!(analyze(&data, &Settings::default()).unwrap().bursts, 0);
        assert_eq!(repair(&mut data, &Settings::default()), 0);
        assert!(data == before);
    }

    #[test]
    fn other_rates_are_not_repaired() {
        let unique = attitude_with_burst();
        // 200 Hz: every 5th sample of the 1 kHz track.
        let mut data: TimeQuat = unique.iter().step_by(5).enumerate().map(|(i, q)| (i as i64 * 5000, to_gyroflow(q))).collect();
        assert!(analyze(&data, &Settings::default()).is_none());
        assert_eq!(repair(&mut data, &Settings::default()), 0);
    }

    #[test]
    fn a_new_or_cleared_gyro_source_has_the_default_sensitivity() {
        // The UI restores the slider from the project, and a fresh project must not say 0%.
        let mut gyro = GyroSource::new();
        assert_eq!(gyro.imu_transforms.gyrogate_strength, 50.0);
        gyro.imu_transforms.gyrogate_strength = 10.0;
        gyro.clear();
        assert_eq!(gyro.imu_transforms.gyrogate_strength, 50.0);
    }

    fn load(path: &std::path::Path) -> TimeQuat {
        let mut file = std::fs::File::open(path).unwrap();
        let size = file.metadata().unwrap().len() as usize;
        let md = GyroSource::parse_telemetry_file(&mut file, size, path, &FileLoadOptions::default(), (3840, 2160), 50.0, |_| (), Arc::new(AtomicBool::new(false))).unwrap();
        md.quaternions
    }

    /// The stabilization sees the same attitude with the in-memory repair as with the corrected
    /// copy. Run with `FPVFLOW_GYROGATE_DIR=<folder with o4lite_53-58s.MP4> cargo test -p fpvflow-core
    /// --release memory_repair_matches -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn memory_repair_matches_the_corrected_copy() {
        let Ok(dir) = std::env::var("FPVFLOW_GYROGATE_DIR") else { return; };
        let src = std::path::Path::new(&dir).join("o4lite_53-58s.MP4");
        let dst = std::env::temp_dir().join("fpvflow_memory_repair_o4lite_gyrofix.MP4");
        let report = super::super::file::repair_file(&src, &dst, &Settings::default(), |_| (), Arc::new(AtomicBool::new(false))).unwrap();
        assert!(report.written);

        let mut in_memory = load(&src);
        assert_eq!(repair(&mut in_memory, &Settings::default()), 3);
        let copy = load(&dst);
        let _ = std::fs::remove_file(&dst);
        assert_eq!(in_memory.len(), copy.len());
        let worst = in_memory.iter().zip(copy.iter()).map(|((ta, a), (tb, b))| {
            assert_eq!(ta, tb);
            angle_deg(a, b)
        }).fold(0.0, f64::max);
        println!("in-memory repair vs corrected copy: {} values, worst {worst:.2e}°", in_memory.len());
        // The copy is stored as f32.
        assert!(worst < 1e-4);
    }
}
