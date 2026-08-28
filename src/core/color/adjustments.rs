// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2026 Rodrigo Sclosa

//! Per-pixel color adjustments, applied after the LUT.
//!
//! # The order is part of the result
//!
//! Color operations do not commute. The order below is fixed and mirrored
//! verbatim in the three shaders:
//!
//! ```text
//! exposure -> luminance -> contrast -> blacks -> shadows -> highlights
//!   -> whites -> temperature -> tint -> saturation -> vibrance -> clamp
//! ```
//!
//! Sharpness and vignette are not here: they need neighbouring pixels or the
//! output position, so they run as a separate stage after this one.
//!
//! # Working space
//!
//! Everything runs on Rec.709 **display-encoded** values, which is what the LUT
//! produced and what matches the user's expectation when dragging a slider.
//! Exposure is the one exception: it linearizes, applies the gain and encodes
//! back, because exposure is light and light multiplies in linear space. That
//! difference is also what keeps exposure and luminance from collapsing into the
//! same control.
//!
//! # Four copies, one reference
//!
//! `wgpu_undistort.wgsl`, `opencl_undistort.cl` and `qt_gpu/undistort.frag` each
//! carry a hand-written copy, because this project has no shader code sharing
//! (see `stabilization/mod.rs:102`). This module is the one the tests exercise,
//! so it is the reference the other three must match.

use nalgebra::Vector4;

/// Calibration constants.
///
/// Kept together and named rather than scattered as literals in four files: when
/// a control feels too weak or too strong, this is the only block to touch, and
/// the shaders can be checked against it by eye.
pub mod k {
    /// Rec.709 luma weights - the space the LUT converts into. Used by
    /// saturation, vibrance and every tonal mask, so they stay consistent.
    pub const LUMA: [f32; 3] = [0.2126, 0.7152, 0.0722];

    /// Exposure travel at the ends of the slider, in stops.
    pub const EXPOSURE_STOPS: f32 = 2.0;
    /// Luminance gain at the ends, as a fraction.
    pub const LUMINANCE_GAIN: f32 = 0.5;
    /// Pivot contrast rotates around, in display space. Middle grey encoded.
    pub const CONTRAST_PIVOT: f32 = 0.5;
    /// How far the four tonal bands can push their region.
    pub const TONAL_STRENGTH: f32 = 0.5;
    /// Channel tilt at the ends for white balance.
    pub const WB_STRENGTH: f32 = 0.2;
    /// Radius where the vignette starts falling off, and where it ends.
    pub const VIGNETTE_INNER: f32 = 0.3;
    pub const VIGNETTE_OUTER: f32 = 1.0;

    /// Mask edges for the four tonal bands. Overlapping on purpose: the bands
    /// have to cover the tonal range without gaps, and hard edges would band.
    pub const BLACKS_EDGE: (f32, f32) = (0.0, 0.25);
    pub const SHADOWS_EDGE: (f32, f32) = (0.0, 0.5);
    pub const HIGHLIGHTS_EDGE: (f32, f32) = (0.5, 1.0);
    pub const WHITES_EDGE: (f32, f32) = (0.75, 1.0);
}

/// Every adjustment, as the user sees it: -100..100 (0..100 for sharpness),
/// neutral at 0.
///
/// The core stores UI values and normalizes at the shader boundary, so there is
/// exactly one place where the /100 happens.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ColorAdjustments {
    // --- Light ---
    pub exposure: f32,
    pub luminance: f32,
    pub contrast: f32,
    pub highlights: f32,
    pub shadows: f32,
    pub whites: f32,
    pub blacks: f32,
    // --- Color ---
    pub temperature: f32,
    pub tint: f32,
    pub saturation: f32,
    pub vibrance: f32,
    // --- Effect ---
    /// 0..100, neutral 0. Runs in the spatial stage, not the per-pixel one.
    pub sharpness: f32,
    /// -100..100, neutral 0. Negative darkens the corners, positive lightens.
    pub vignette: f32,
}

impl Default for ColorAdjustments {
    fn default() -> Self {
        Self {
            exposure: 0.0, luminance: 0.0, contrast: 0.0,
            highlights: 0.0, shadows: 0.0, whites: 0.0, blacks: 0.0,
            temperature: 0.0, tint: 0.0, saturation: 0.0, vibrance: 0.0,
            sharpness: 0.0, vignette: 0.0,
        }
    }
}

impl ColorAdjustments {
    /// Whether nothing would change, so the whole stage can be skipped.
    pub fn is_identity(&self) -> bool { *self == Self::default() }

    /// Reads one value by the name the UI and the project file use.
    pub fn get(&self, name: &str) -> f32 {
        match name {
            "exposure" => self.exposure, "luminance" => self.luminance,
            "contrast" => self.contrast, "highlights" => self.highlights,
            "shadows" => self.shadows, "whites" => self.whites,
            "blacks" => self.blacks, "temperature" => self.temperature,
            "tint" => self.tint, "saturation" => self.saturation,
            "vibrance" => self.vibrance, "sharpness" => self.sharpness,
            "vignette" => self.vignette,
            _ => 0.0
        }
    }

    /// Writes one value by name, clamped to its range. Unknown names are
    /// ignored so a project written by a newer version still loads.
    pub fn set(&mut self, name: &str, v: f32) {
        // Sharpness is the only one-sided control besides being 0..100.
        let v = if name == "sharpness" { v.clamp(0.0, 100.0) } else { v.clamp(-100.0, 100.0) };
        match name {
            "exposure" => self.exposure = v, "luminance" => self.luminance = v,
            "contrast" => self.contrast = v, "highlights" => self.highlights = v,
            "shadows" => self.shadows = v, "whites" => self.whites = v,
            "blacks" => self.blacks = v, "temperature" => self.temperature = v,
            "tint" => self.tint = v, "saturation" => self.saturation = v,
            "vibrance" => self.vibrance = v, "sharpness" => self.sharpness = v,
            "vignette" => self.vignette = v,
            _ => log::warn!("Unknown color adjustment: {name}")
        }
    }

    /// The names in pipeline order, for the UI and for iterating.
    pub const NAMES: [&'static str; 13] = [
        "exposure", "luminance", "contrast", "highlights", "shadows", "whites", "blacks",
        "temperature", "tint", "saturation", "vibrance", "sharpness", "vignette"
    ];

    /// Packs into the four `vec4`s `KernelParams` carries, normalized to -1..1.
    ///
    /// This is the single UI -> shader conversion point the spec asks for.
    pub fn to_gpu(&self) -> [[f32; 4]; 4] {
        let n = |v: f32| v / 100.0;
        [
            [n(self.exposure), n(self.luminance), n(self.contrast), n(self.highlights)],
            [n(self.shadows), n(self.whites), n(self.blacks), n(self.temperature)],
            [n(self.tint), n(self.saturation), n(self.vibrance), n(self.vignette)],
            // Sharpness is consumed by the spatial stage; the fourth slot keeps
            // the block 16-byte aligned and leaves room for the next control.
            [n(self.sharpness), 0.0, 0.0, 0.0],
        ]
    }
}

/// Rec.709 EOTF - display-encoded to linear light.
///
/// Exposure needs this: doubling light is a multiply in linear space, and doing
/// it on gamma-encoded values would brighten midtones far more than highlights.
#[inline]
fn eotf_r709(v: f32) -> f32 {
    if v < 0.081 { v / 4.5 } else { ((v + 0.099) / 1.099).powf(1.0 / 0.45) }
}

/// Rec.709 OETF - linear light back to display-encoded.
#[inline]
fn oetf_r709(v: f32) -> f32 {
    if v < 0.018 { v * 4.5 } else { 1.099 * v.powf(0.45) - 0.099 }
}

#[inline]
fn luma(c: &[f32; 3]) -> f32 { c[0] * k::LUMA[0] + c[1] * k::LUMA[1] + c[2] * k::LUMA[2] }

/// Same curve as GLSL/WGSL/OpenCL `smoothstep`, so the four copies agree.
#[inline]
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Applies the per-pixel adjustments in place.
///
/// `adjust` holds the four normalized blocks from [`ColorAdjustments::to_gpu`].
/// `px` is in the pipeline's scale (0..`max_pixel_value`), not 0..1.
///
/// Vignette is included here because it only needs the output position, which
/// this stage already has. Sharpness is not - it needs neighbours.
pub fn apply(px: &mut Vector4<f32>, out_pos: (f32, f32), adjust: &[[f32; 4]; 4], max_pixel_value: f32, output_size: (f32, f32)) {
    let [exposure, luminance, contrast, highlights] = adjust[0];
    let [shadows, whites, blacks, temperature] = adjust[1];
    let [tint, saturation, vibrance, vignette] = adjust[2];

    // The common case is every slider at neutral, so this costs one branch
    // rather than the whole chain.
    if exposure == 0.0 && luminance == 0.0 && contrast == 0.0 && highlights == 0.0
        && shadows == 0.0 && whites == 0.0 && blacks == 0.0 && temperature == 0.0
        && tint == 0.0 && saturation == 0.0 && vibrance == 0.0 && vignette == 0.0 {
        return;
    }

    let mut c = [px[0] / max_pixel_value, px[1] / max_pixel_value, px[2] / max_pixel_value];

    // 1. Exposure, in stops, in linear light.
    if exposure != 0.0 {
        let gain = (exposure * k::EXPOSURE_STOPS).exp2();
        for ch in &mut c { *ch = oetf_r709(eotf_r709(ch.max(0.0)) * gain); }
    }

    // 2. Luminance: a plain gain in display space. Deliberately not exposure -
    //    same direction, different curve, which is why both controls exist.
    if luminance != 0.0 {
        let gain = 1.0 + luminance * k::LUMINANCE_GAIN;
        for ch in &mut c { *ch *= gain; }
    }

    // 3. Contrast, pivoting on middle grey so the image does not also brighten.
    if contrast != 0.0 {
        let gain = 1.0 + contrast;
        for ch in &mut c { *ch = (*ch - k::CONTRAST_PIVOT) * gain + k::CONTRAST_PIVOT; }
    }

    // 4-7. The four tonal bands, darkest first. Each has its own mask, and each
    //      mixes toward white or toward its own luma rather than adding a raw
    //      offset - that keeps the ends from clipping abruptly.
    if blacks != 0.0 || shadows != 0.0 || highlights != 0.0 || whites != 0.0 {
        for (amount, edge) in [
            (blacks,     k::BLACKS_EDGE),
            (shadows,    k::SHADOWS_EDGE),
            (highlights, k::HIGHLIGHTS_EDGE),
            (whites,     k::WHITES_EDGE),
        ] {
            if amount == 0.0 { continue; }
            let l = luma(&c);
            // The dark bands invert the mask so they act near black.
            let mask = if edge.1 <= 0.5 || edge == k::SHADOWS_EDGE {
                1.0 - smoothstep(edge.0, edge.1, l)
            } else {
                smoothstep(edge.0, edge.1, l)
            };
            let w = amount.abs() * mask * k::TONAL_STRENGTH;
            let target = if amount > 0.0 { 1.0 } else { 0.0 };
            for ch in &mut c { *ch += (target - *ch) * w; }
        }
    }

    // 8-9. White balance: warm pushes red and pulls blue; tint trades green
    //      against magenta, splitting the counter-move across red and blue so it
    //      does not just shift green.
    if temperature != 0.0 || tint != 0.0 {
        let t = temperature * k::WB_STRENGTH;
        c[0] += t;
        c[2] -= t;
        let g = tint * k::WB_STRENGTH;
        c[1] -= g;
        c[0] += g * 0.5;
        c[2] += g * 0.5;
    }

    // 10. Saturation, after the tonal work so it acts on the final tones.
    if saturation != 0.0 {
        let l = luma(&c);
        let gain = 1.0 + saturation;
        for ch in &mut c { *ch = l + (*ch - l) * gain; }
    }

    // 11. Vibrance: saturation that backs off where the pixel is already
    //     saturated, so skies and skin do not blow out first.
    if vibrance != 0.0 {
        let l = luma(&c);
        let mx = c[0].max(c[1]).max(c[2]);
        let mn = c[0].min(c[1]).min(c[2]);
        let gain = 1.0 + vibrance * (1.0 - (mx - mn).clamp(0.0, 1.0));
        for ch in &mut c { *ch = l + (*ch - l) * gain; }
    }

    // 12. Vignette. Bipolar: negative darkens the corners, positive lifts them.
    //     The radius is aspect-corrected, so a 16:9 frame gets a circle.
    if vignette != 0.0 {
        let (w, h) = (output_size.0.max(1.0), output_size.1.max(1.0));
        let aspect = w / h;
        let dx = (out_pos.0 / w - 0.5) * aspect;
        let dy = out_pos.1 / h - 0.5;
        // v = 1 at the centre, falling to 0 at the corner, so (1 - v) is how far
        // into the vignette this pixel is. Negative darkens, positive lightens,
        // and the centre is untouched either way because there 1 - v == 0.
        let v = smoothstep(k::VIGNETTE_OUTER, k::VIGNETTE_INNER, dx.hypot(dy) / (0.5f32 * aspect).hypot(0.5));
        let factor = 1.0 + vignette * (1.0 - v);
        for ch in &mut c { *ch *= factor; }
    }

    // Safety clamp before the spatial stage, so out-of-range values do not
    // propagate into the sharpening neighbourhood.
    for i in 0..3 { px[i] = c[i].clamp(0.0, 1.0) * max_pixel_value; }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: (f32, f32) = (1920.0, 1080.0);
    /// Frame centre, where the vignette must not change anything.
    const CENTER: (f32, f32) = (960.0, 540.0);

    fn px(r: f32, g: f32, b: f32) -> Vector4<f32> { Vector4::new(r, g, b, 1.0) }

    /// Builds the packed block with one control set, everything else neutral.
    fn one(name: &str, ui_value: f32) -> [[f32; 4]; 4] {
        let mut a = ColorAdjustments::default();
        a.set(name, ui_value);
        a.to_gpu()
    }

    fn apply1(p: &mut Vector4<f32>, name: &str, v: f32) {
        apply(p, CENTER, &one(name, v), 1.0, SIZE);
    }

    /// Criterion 1 of the spec, for every control: neutral is identity.
    #[test]
    fn every_control_is_identity_at_neutral() {
        for name in ColorAdjustments::NAMES {
            let mut p = px(0.3, 0.5, 0.7);
            let before = p;
            apply1(&mut p, name, 0.0);
            assert_eq!(p, before, "{name} at 0 must not change the pixel");
        }
        assert!(ColorAdjustments::default().is_identity());
    }

    /// Criterion 2: each control moves the image the way its name promises.
    #[test]
    fn every_control_moves_in_the_expected_direction() {
        // Each band is tested inside its own mask: whites only bites above 0.75,
        // so probing it at 0.6 would test nothing.
        for (name, probe) in [
            ("exposure", 0.6f32), ("luminance", 0.6), ("contrast", 0.8),
            ("highlights", 0.8), ("whites", 0.9),
            ("shadows", 0.2), ("blacks", 0.05),
        ] {
            let mut up = px(probe, probe, probe);
            apply1(&mut up, name, 60.0);
            assert!(up[0] > probe, "{name} +60 must brighten {probe}, got {}", up[0]);

            let mut down = px(probe, probe, probe);
            apply1(&mut down, name, -60.0);
            assert!(down[0] < probe, "{name} -60 must darken {probe}, got {}", down[0]);
        }

        let mut warm = px(0.5, 0.5, 0.5);
        apply1(&mut warm, "temperature", 50.0);
        assert!(warm[0] > 0.5 && warm[2] < 0.5, "warm must raise red and lower blue");

        let mut sat = px(0.8, 0.4, 0.2);
        apply1(&mut sat, "saturation", 50.0);
        assert!(sat[0] > 0.8, "more saturation must push the dominant channel out");
    }

    /// Criterion 4: controls that should differ, differ. The spec calls this out
    /// twice, because collapsing them would silently reduce 13 controls to 11.
    #[test]
    fn exposure_and_luminance_are_not_the_same_control() {
        let mut a = px(0.25, 0.25, 0.25);
        let mut b = px(0.25, 0.25, 0.25);
        apply1(&mut a, "exposure", 50.0);
        apply1(&mut b, "luminance", 50.0);
        assert!((a[0] - b[0]).abs() > 0.01,
            "exposure and luminance must differ: {} vs {}", a[0], b[0]);
    }

    #[test]
    fn shadows_and_blacks_use_different_masks() {
        // A pixel in the shadow region but above the blacks band: shadows should
        // move it clearly more than blacks does.
        let mut s = px(0.35, 0.35, 0.35);
        let mut b = px(0.35, 0.35, 0.35);
        apply1(&mut s, "shadows", 80.0);
        apply1(&mut b, "blacks", 80.0);
        assert!((s[0] - b[0]).abs() > 0.01,
            "shadows and blacks must have distinct masks: {} vs {}", s[0], b[0]);
    }

    #[test]
    fn highlights_and_whites_use_different_masks() {
        let mut h = px(0.6, 0.6, 0.6);
        let mut w = px(0.6, 0.6, 0.6);
        apply1(&mut h, "highlights", 80.0);
        apply1(&mut w, "whites", 80.0);
        assert!((h[0] - w[0]).abs() > 0.01,
            "highlights and whites must have distinct masks: {} vs {}", h[0], w[0]);
    }

    /// Criterion 5: bipolar controls are symmetric around neutral.
    #[test]
    fn bipolar_controls_reverse_around_neutral() {
        for name in ["exposure", "luminance", "contrast", "saturation", "temperature", "tint"] {
            let base = 0.5f32;
            let mut up = px(base, base, base);
            let mut down = px(base, base, base);
            apply1(&mut up, name, 40.0);
            apply1(&mut down, name, -40.0);
            // One goes up and the other down, or (for contrast at the pivot)
            // both stay - what must not happen is both moving the same way.
            let du = up[0] - base;
            let dd = down[0] - base;
            assert!(du * dd <= 1e-6, "{name}: +40 and -40 must not move the same way ({du} vs {dd})");
        }
    }

    #[test]
    fn contrast_pivots_at_the_spec_value() {
        // A pixel exactly at the pivot must not move - that is what makes
        // contrast a rotation rather than a brightness change.
        let mut p = px(k::CONTRAST_PIVOT, k::CONTRAST_PIVOT, k::CONTRAST_PIVOT);
        apply1(&mut p, "contrast", 50.0);
        for ch in 0..3 {
            assert!((p[ch] - k::CONTRAST_PIVOT).abs() < 1e-5,
                "the pivot must stay put, got {}", p[ch]);
        }

        let mut hi = px(0.7, 0.7, 0.7);
        apply1(&mut hi, "contrast", 50.0);
        assert!(hi[0] > 0.7, "above the pivot must brighten");

        let mut lo = px(0.3, 0.3, 0.3);
        apply1(&mut lo, "contrast", 50.0);
        assert!(lo[0] < 0.3, "below the pivot must darken");
    }

    #[test]
    fn exposure_doubles_light_per_stop() {
        // +100 is EXPOSURE_STOPS, so half of it is one stop: the linear light
        // must double. This is what linearizing buys, and testing it in linear
        // space is the only way to see it.
        let start = 0.2f32;
        let mut p = px(start, start, start);
        apply1(&mut p, "exposure", 100.0 / k::EXPOSURE_STOPS);
        let got = eotf_r709(p[0]);
        let want = eotf_r709(start) * 2.0;
        assert!((got - want).abs() < 1e-4,
            "one stop must double linear light: {got} vs {want}");
    }

    #[test]
    fn full_desaturation_lands_on_luma() {
        let mut p = px(0.8, 0.4, 0.2);
        let expected = luma(&[0.8, 0.4, 0.2]);
        apply1(&mut p, "saturation", -100.0);
        for ch in 0..3 {
            assert!((p[ch] - expected).abs() < 1e-5,
                "saturation -100 must collapse to luma {expected}, got {}", p[ch]);
        }
    }

    #[test]
    fn a_grey_pixel_stays_grey_when_desaturated() {
        // Guards the luma weights: if they did not sum to 1, grey would drift.
        let mut p = px(0.5, 0.5, 0.5);
        apply1(&mut p, "saturation", -100.0);
        for ch in 0..3 { assert!((p[ch] - 0.5).abs() < 1e-5, "grey drifted to {}", p[ch]); }
    }

    /// Vibrance is saturation that protects what is already saturated - if it
    /// behaved like plain saturation, it would be a duplicate control.
    #[test]
    fn vibrance_protects_already_saturated_pixels() {
        let vivid = [0.95, 0.05, 0.05];
        let muted = [0.55, 0.45, 0.45];

        let gain_of = |c: [f32; 3]| {
            let mut p = px(c[0], c[1], c[2]);
            apply1(&mut p, "vibrance", 80.0);
            let l = luma(&c);
            // How much the channel spread grew.
            (p[0] - l).abs() / (c[0] - l).abs().max(1e-6)
        };

        assert!(gain_of(muted) > gain_of(vivid),
            "vibrance must push muted pixels harder than vivid ones");
    }

    #[test]
    fn the_vignette_is_bipolar_and_spares_the_centre() {
        let mut center = px(0.8, 0.8, 0.8);
        apply1(&mut center, "vignette", -80.0);
        assert!((center[0] - 0.8).abs() < 1e-5, "the centre must not change, got {}", center[0]);

        let mut dark = px(0.8, 0.8, 0.8);
        apply(&mut dark, (0.0, 0.0), &one("vignette", -80.0), 1.0, SIZE);
        assert!(dark[0] < 0.5, "negative must darken the corner, got {}", dark[0]);

        let mut light = px(0.4, 0.4, 0.4);
        apply(&mut light, (0.0, 0.0), &one("vignette", 80.0), 1.0, SIZE);
        assert!(light[0] > 0.4, "positive must lighten the corner, got {}", light[0]);
    }

    #[test]
    fn the_vignette_is_round_not_elliptical() {
        // Without the aspect correction a 16:9 frame gets an ellipse, and the
        // left edge would darken differently from the top.
        let (w, h) = SIZE;
        let r = 0.4f32;
        let aspect = w / h;

        let mut horizontal = px(0.8, 0.8, 0.8);
        apply(&mut horizontal, (w * (0.5 + r / aspect), h * 0.5), &one("vignette", -80.0), 1.0, SIZE);
        let mut vertical = px(0.8, 0.8, 0.8);
        apply(&mut vertical, (w * 0.5, h * (0.5 + r)), &one("vignette", -80.0), 1.0, SIZE);

        assert!((horizontal[0] - vertical[0]).abs() < 1e-4,
            "equal radii must darken equally: {} vs {}", horizontal[0], vertical[0]);
    }

    /// Criterion 3: no banding. A synthetic gradient pushed hard must stay
    /// monotonic with no repeated steps - proof the work is in float.
    #[test]
    fn a_gradient_survives_extreme_settings_without_banding() {
        let mut a = ColorAdjustments::default();
        a.set("contrast", 100.0);
        a.set("saturation", 80.0);
        a.set("shadows", -60.0);
        let packed = a.to_gpu();

        const N: usize = 512;
        let mut last = -1.0f32;
        let mut flat_runs = 0;
        for i in 0..N {
            let v = i as f32 / (N - 1) as f32;
            let mut p = px(v, v, v);
            apply(&mut p, CENTER, &packed, 1.0, SIZE);
            assert!(p[0] >= last - 1e-6, "the gradient must stay monotonic at {v}");
            // Count consecutive identical outputs away from the clamped ends.
            if (p[0] - last).abs() < 1e-9 && p[0] > 0.001 && p[0] < 0.999 { flat_runs += 1; }
            last = p[0];
        }
        assert!(flat_runs < N / 8, "too many repeated values ({flat_runs}) - that is banding");
    }

    #[test]
    fn the_scale_round_trips_at_every_bit_depth() {
        // Each backend divides by max_pixel_value and multiplies back; getting
        // that wrong yields an image that is merely a bit off.
        for max in [255.0f32, 1023.0, 65535.0] {
            let mut p = Vector4::new(0.5 * max, 0.5 * max, 0.5 * max, max);
            apply(&mut p, CENTER, &one("luminance", 100.0), max, SIZE);
            let expected = (0.5 * (1.0 + k::LUMINANCE_GAIN)).clamp(0.0, 1.0) * max;
            assert!((p[0] - expected).abs() < max * 0.001,
                "at scale {max} expected {expected}, got {}", p[0]);
        }
    }

    #[test]
    fn values_are_clamped_into_range() {
        let mut p = px(0.9, 0.9, 0.9);
        apply1(&mut p, "exposure", 100.0);
        for ch in 0..3 { assert!(p[ch] <= 1.0, "must not exceed 1.0, got {}", p[ch]); }

        let mut p = px(0.1, 0.1, 0.1);
        apply1(&mut p, "exposure", -100.0);
        for ch in 0..3 { assert!(p[ch] >= 0.0, "must not go below 0.0, got {}", p[ch]); }
    }

    #[test]
    fn alpha_is_never_touched() {
        let mut a = ColorAdjustments::default();
        for name in ColorAdjustments::NAMES { a.set(name, 50.0); }
        let mut p = Vector4::new(0.5, 0.5, 0.5, 0.25);
        apply(&mut p, CENTER, &a.to_gpu(), 1.0, SIZE);
        assert_eq!(p[3], 0.25, "alpha must survive every adjustment");
    }

    #[test]
    fn the_transfer_functions_are_inverses() {
        for i in 0..=100 {
            let v = i as f32 / 100.0;
            assert!((oetf_r709(eotf_r709(v)) - v).abs() < 1e-4,
                "eotf/oetf must round-trip at {v}");
        }
    }

    #[test]
    fn set_clamps_to_the_documented_ranges() {
        let mut a = ColorAdjustments::default();
        a.set("exposure", 500.0);
        assert_eq!(a.exposure, 100.0);
        a.set("exposure", -500.0);
        assert_eq!(a.exposure, -100.0);
        // Sharpness is one-sided.
        a.set("sharpness", -50.0);
        assert_eq!(a.sharpness, 0.0);
        // An unknown name must not panic or corrupt anything.
        a.set("nonsense", 42.0);
        assert_eq!(a.get("nonsense"), 0.0);
    }
}
