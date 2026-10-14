// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2026 Rodrigo Sclosa

//! Repair of the DJI O4 "Gyrogate" defect.
//!
//! DJI O4 Air Unit Pro units built since about February 2026, and every O4 Lite, use the
//! TDK ICM-40609-D gyro. Under motor vibration it records short false rotations on a single
//! gyro axis, and the camera's attitude fusion then pulls each false step back over about
//! 12.5 s. The 1 kHz attitude quaternions stored in the MP4 carry both, so the stabilization
//! faithfully "corrects" rotations that never happened.
//!
//! The correction is a port of o4fix (https://github.com/vadimbryl/o4fix, MIT, by vadimbryl):
//! remove the single-axis impulse bursts, add back the fusion's pull-back of the removed steps,
//! then low-pass. `dsp` holds the signal processing, `file` reads the attitude track with the
//! byte offset of every value and writes a corrected copy of the MP4, `memory` repairs the
//! quaternions loaded for the stabilization and `unit` tells which gyro a unit likely has.

pub mod dsp;
pub mod file;
pub mod memory;
pub mod unit;

/// Unit quaternion as `[w, x, y, z]`.
pub type Quat4 = [f64; 4];

/// The attitude rate of the O4 track once each stored value's duplicate is collapsed.
pub const UNIQUE_RATE_HZ: f64 = 1000.0;

/// Appended to the file stem of the corrected copy: `<name>_gyrofix.MP4`.
pub const COPY_SUFFIX: &str = "_gyrofix";

/// Correction parameters. `pullback_tau_s` or `lowpass_hz` of 0 skips that step, and
/// `busy_lowpass_hz` of 0 low-passes everywhere at `lowpass_hz`. `impulse_threshold_dps` must be
/// above 0: at 0 every single-axis deviation would count as a burst.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub impulse_threshold_dps: f64,
    pub lowpass_hz: f64,
    pub busy_lowpass_hz: f64,
    pub pullback_tau_s: f64,
}
impl Default for Settings {
    fn default() -> Self {
        Self { impulse_threshold_dps: 60.0, lowpass_hz: 20.0, busy_lowpass_hz: 3.0, pullback_tau_s: 12.5 }
    }
}
impl Settings {
    /// Settings for a UI sensitivity in percent. 50 is o4fix's default (60 °/s impulse
    /// threshold); 0 doubles the threshold and 100 halves it.
    pub fn from_strength(strength_pct: f64) -> Self {
        let s = strength_pct.clamp(0.0, 100.0);
        Self { impulse_threshold_dps: 60.0 * 2f64.powf((50.0 - s) / 50.0), ..Default::default() }
    }
}

/// A removed false burst: inclusive range of rate samples, body axis (0, 1, 2) and the
/// rotation removed in degrees.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct Burst {
    pub start_sample: usize,
    pub end_sample: usize,
    pub axis: usize,
    pub area_deg: f64,
}

/// Corrected unique attitude samples and what was removed.
#[derive(Clone, Debug)]
pub struct Correction {
    pub quats: Vec<Quat4>,
    pub bursts: Vec<Burst>,
}
