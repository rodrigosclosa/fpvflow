// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2026 Rodrigo Sclosa

//! Per-pixel color adjustments, applied after the LUT.
//!
//! # The order is part of the result
//!
//! Exposure, then white balance, then tone (contrast, highlights, shadows), then
//! saturation last so it acts on the tones the earlier steps produced. Swapping
//! any two changes the image, so the order is fixed here and mirrored verbatim in
//! the three shaders.
//!
//! # Four copies, one reference
//!
//! `wgpu_undistort.wgsl`, `opencl_undistort.cl` and `qt_gpu/undistort.frag` each
//! carry a hand-written copy, because this project has no shader code sharing
//! (see `stabilization/mod.rs:102`). This module is the one the tests exercise,
//! so it is the reference the other three must match.

use nalgebra::Vector4;

/// Rec.709 luma weights - the space the LUT converts into.
const LUMA: [f32; 3] = [0.2126, 0.7152, 0.0722];

/// Middle grey, the pivot contrast rotates around so the image does not also
/// brighten when contrast is raised.
const MID_GREY: f32 = 0.18;

/// Applies the adjustments in place.
///
/// `adjust1` is `[exposure_ev, contrast, saturation, temperature]` and `adjust2`
/// is `[tint, highlights, shadows, vignette]`, packed that way because
/// `KernelParams` passes them as two `vec4`s.
///
/// `px` is in the pipeline's scale (0..`max_pixel_value`), not 0..1.
pub fn apply(px: &mut Vector4<f32>, out_pos: (f32, f32), adjust1: [f32; 4], adjust2: [f32; 4], max_pixel_value: f32, output_size: (f32, f32)) {
    let [exposure_ev, contrast, saturation, temperature] = adjust1;
    let [tint, highlights, shadows, vignette] = adjust2;

    // The common case is every slider at its default, so this costs one branch
    // rather than the whole chain.
    if exposure_ev == 0.0 && contrast == 0.0 && saturation == 0.0 && temperature == 0.0
        && tint == 0.0 && highlights == 0.0 && shadows == 0.0 && vignette == 0.0 {
        return;
    }

    let mut c = [px[0] / max_pixel_value, px[1] / max_pixel_value, px[2] / max_pixel_value];

    // Exposure in stops, which is what the number means to a photographer.
    if exposure_ev != 0.0 {
        let gain = 2.0f32.powf(exposure_ev);
        for ch in &mut c { *ch *= gain; }
    }

    // White balance as a channel tilt: warm pushes red and pulls blue, tint
    // trades green against magenta. Not a chromatic adaptation transform, but
    // predictable and monotonic, which is what a slider needs.
    if temperature != 0.0 || tint != 0.0 {
        c[0] += temperature * 0.2;
        c[2] -= temperature * 0.2;
        c[1] += tint * 0.2;
    }

    if contrast != 0.0 {
        for ch in &mut c { *ch = (*ch - MID_GREY) * (1.0 + contrast) + MID_GREY; }
    }

    // Smooth masks rather than hard thresholds, so a moving edge does not band
    // as the mask flips.
    if highlights != 0.0 || shadows != 0.0 {
        let l = luma(&c);
        let hi = smoothstep(0.5, 1.0, l);
        let lo = 1.0 - smoothstep(0.0, 0.5, l);
        for ch in &mut c { *ch += highlights * hi * 0.5 + shadows * lo * 0.5; }
    }

    // Last, so it acts on the tones the steps above produced.
    if saturation != 0.0 {
        let l = luma(&c);
        for ch in &mut c { *ch = l + (*ch - l) * (1.0 + saturation); }
    }

    // Darkens toward the corners, aspect-corrected so a 16:9 frame gets a circle
    // rather than an ellipse.
    if vignette != 0.0 {
        let (w, h) = (output_size.0, output_size.1);
        let aspect = w / h.max(1.0);
        let dx = (out_pos.0 / w.max(1.0) - 0.5) * aspect;
        let dy = out_pos.1 / h.max(1.0) - 0.5;
        // Normalized so the corner reaches 1.0 whatever the aspect.
        let corner = (0.5f32 * aspect).hypot(0.5);
        let r = dx.hypot(dy) / corner.max(0.0001);
        let factor = 1.0 - vignette * smoothstep(0.3, 1.0, r);
        for ch in &mut c { *ch *= factor; }
    }

    for i in 0..3 { px[i] = c[i].clamp(0.0, 1.0) * max_pixel_value; }
}

#[inline]
fn luma(c: &[f32; 3]) -> f32 { c[0] * LUMA[0] + c[1] * LUMA[1] + c[2] * LUMA[2] }

/// Same curve as GLSL/WGSL/OpenCL `smoothstep`, so the four copies agree.
#[inline]
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE1: [f32; 4] = [0.0; 4];
    const NONE2: [f32; 4] = [0.0; 4];
    const SIZE: (f32, f32) = (1920.0, 1080.0);
    /// Frame centre, where the vignette must not darken anything.
    const CENTER: (f32, f32) = (960.0, 540.0);

    fn px(r: f32, g: f32, b: f32) -> Vector4<f32> { Vector4::new(r, g, b, 1.0) }

    #[test]
    fn all_defaults_leave_the_pixel_untouched() {
        let mut p = px(0.3, 0.5, 0.7);
        let before = p;
        apply(&mut p, CENTER, NONE1, NONE2, 1.0, SIZE);
        assert_eq!(p, before, "every slider at default must be a no-op");
    }

    #[test]
    fn exposure_is_measured_in_stops() {
        // +1 EV doubles, -1 EV halves - that is what the unit promises.
        let mut p = px(0.2, 0.2, 0.2);
        apply(&mut p, CENTER, [1.0, 0.0, 0.0, 0.0], NONE2, 1.0, SIZE);
        assert!((p[0] - 0.4).abs() < 1e-5, "+1 EV must double, got {}", p[0]);

        let mut p = px(0.4, 0.4, 0.4);
        apply(&mut p, CENTER, [-1.0, 0.0, 0.0, 0.0], NONE2, 1.0, SIZE);
        assert!((p[0] - 0.2).abs() < 1e-5, "-1 EV must halve, got {}", p[0]);
    }

    #[test]
    fn contrast_pivots_around_middle_grey() {
        // A pixel already at the pivot must not move, which is what makes
        // contrast a rotation rather than a brightness change.
        let mut p = px(MID_GREY, MID_GREY, MID_GREY);
        apply(&mut p, CENTER, [0.0, 0.5, 0.0, 0.0], NONE2, 1.0, SIZE);
        for ch in 0..3 {
            assert!((p[ch] - MID_GREY).abs() < 1e-5, "middle grey must stay put, got {}", p[ch]);
        }

        // Above the pivot moves up, below moves down.
        let mut hi = px(0.5, 0.5, 0.5);
        apply(&mut hi, CENTER, [0.0, 0.5, 0.0, 0.0], NONE2, 1.0, SIZE);
        assert!(hi[0] > 0.5, "above the pivot must brighten");

        let mut lo = px(0.05, 0.05, 0.05);
        apply(&mut lo, CENTER, [0.0, 0.5, 0.0, 0.0], NONE2, 1.0, SIZE);
        assert!(lo[0] < 0.05, "below the pivot must darken");
    }

    #[test]
    fn full_desaturation_lands_on_luma() {
        let mut p = px(0.8, 0.4, 0.2);
        let expected = 0.8 * LUMA[0] + 0.4 * LUMA[1] + 0.2 * LUMA[2];
        apply(&mut p, CENTER, [0.0, 0.0, -1.0, 0.0], NONE2, 1.0, SIZE);
        for ch in 0..3 {
            assert!((p[ch] - expected).abs() < 1e-5,
                "saturation -1 must collapse to luma {expected}, got {}", p[ch]);
        }
    }

    #[test]
    fn a_grey_pixel_stays_grey_when_desaturated() {
        // Guards the luma weights: if they did not sum to 1, grey would drift.
        let mut p = px(0.5, 0.5, 0.5);
        apply(&mut p, CENTER, [0.0, 0.0, -1.0, 0.0], NONE2, 1.0, SIZE);
        for ch in 0..3 { assert!((p[ch] - 0.5).abs() < 1e-5, "grey drifted to {}", p[ch]); }
    }

    #[test]
    fn temperature_warms_and_cools_in_opposite_directions() {
        let mut warm = px(0.5, 0.5, 0.5);
        apply(&mut warm, CENTER, [0.0, 0.0, 0.0, 0.5], NONE2, 1.0, SIZE);
        assert!(warm[0] > 0.5 && warm[2] < 0.5, "warm must raise red and lower blue");

        let mut cool = px(0.5, 0.5, 0.5);
        apply(&mut cool, CENTER, [0.0, 0.0, 0.0, -0.5], NONE2, 1.0, SIZE);
        assert!(cool[0] < 0.5 && cool[2] > 0.5, "cool must lower red and raise blue");
    }

    #[test]
    fn the_vignette_spares_the_centre_and_darkens_the_corner() {
        let strength = [0.0, 0.0, 0.0, 0.8];

        let mut center = px(1.0, 1.0, 1.0);
        apply(&mut center, CENTER, NONE1, strength, 1.0, SIZE);
        assert!((center[0] - 1.0).abs() < 1e-5, "the centre must not darken, got {}", center[0]);

        let mut corner = px(1.0, 1.0, 1.0);
        apply(&mut corner, (0.0, 0.0), NONE1, strength, 1.0, SIZE);
        assert!(corner[0] < 0.5, "the corner must darken clearly, got {}", corner[0]);
    }

    #[test]
    fn the_vignette_is_round_not_elliptical() {
        // The aspect correction is the whole point: without it a 16:9 frame gets
        // an ellipse, and the left edge would darken differently from the top.
        let strength = [0.0, 0.0, 0.0, 0.8];
        let (w, h) = SIZE;

        // Two points at the same corrected radius, one horizontal, one vertical.
        let r = 0.4f32;
        let aspect = w / h;
        let mut horizontal = px(1.0, 1.0, 1.0);
        apply(&mut horizontal, (w * (0.5 + r / aspect), h * 0.5), NONE1, strength, 1.0, SIZE);
        let mut vertical = px(1.0, 1.0, 1.0);
        apply(&mut vertical, (w * 0.5, h * (0.5 + r)), NONE1, strength, 1.0, SIZE);

        assert!((horizontal[0] - vertical[0]).abs() < 1e-4,
            "equal radii must darken equally: {} vs {}", horizontal[0], vertical[0]);
    }

    #[test]
    fn the_scale_round_trips_at_every_bit_depth() {
        // Each backend divides by max_pixel_value and multiplies back; getting
        // that wrong yields an image that is merely a bit off.
        for max in [255.0f32, 1023.0, 65535.0] {
            let mut p = Vector4::new(0.5 * max, 0.5 * max, 0.5 * max, max);
            apply(&mut p, CENTER, [1.0, 0.0, 0.0, 0.0], NONE2, max, SIZE);
            // +1 EV on 0.5 saturates to 1.0 after the clamp.
            assert!((p[0] - max).abs() < 0.01, "at scale {max} expected {max}, got {}", p[0]);
        }
    }

    #[test]
    fn values_are_clamped_into_range() {
        let mut p = px(0.9, 0.9, 0.9);
        apply(&mut p, CENTER, [4.0, 0.0, 0.0, 0.0], NONE2, 1.0, SIZE);
        for ch in 0..3 { assert!(p[ch] <= 1.0, "must not exceed 1.0, got {}", p[ch]); }

        let mut p = px(0.1, 0.1, 0.1);
        apply(&mut p, CENTER, [-6.0, 0.0, 0.0, 0.0], NONE2, 1.0, SIZE);
        for ch in 0..3 { assert!(p[ch] >= 0.0, "must not go below 0.0, got {}", p[ch]); }
    }

    #[test]
    fn alpha_is_never_touched() {
        let mut p = Vector4::new(0.5, 0.5, 0.5, 0.25);
        apply(&mut p, CENTER, [1.0, 0.5, 0.5, 0.5], [0.5, 0.5, 0.5, 0.5], 1.0, SIZE);
        assert_eq!(p[3], 0.25, "alpha must survive every adjustment");
    }
}
