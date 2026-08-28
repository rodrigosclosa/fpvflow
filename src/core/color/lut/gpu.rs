// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2026 Rodrigo Sclosa

//! Preparing a [`Lut`] for the GPU.
//!
//! Two layouts, because the backends do not agree on 3D textures:
//!
//! - **native 3D** (`N×N×N`, `Rgba32Float`): wgpu creates and samples this with
//!   a linear sampler, which is trilinear interpolation for free.
//! - **tiled 2D** (`N×N` cells laid out in a row): the fallback for anything
//!   that cannot sample a 3D texture. Interpolation along the third axis has to
//!   be done by hand in the shader, mixing two adjacent slices.
//!
//! Both are built from the same data, so a backend choosing one over the other
//! does not change the result - only how the shader reads it.

use super::{Lut, LutData};

/// A LUT flattened for upload, in whichever layout the backend can sample.
#[derive(Debug, Clone)]
pub struct LutTexture {
    /// Cube edge length.
    pub size: u32,
    /// Texel data, `RGBA32F`, row-major.
    pub data: Vec<f32>,
    /// Layout of `data`.
    pub layout: LutLayout,
}

/// How the texels are arranged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LutLayout {
    /// `size × size × size`, sampled as a 3D texture.
    Volume3D,
    /// `(size * size) × size`, blue slices laid side by side.
    Tiled2D,
}

impl LutTexture {
    /// Width in texels, for the texture descriptor.
    pub fn width(&self) -> u32 {
        match self.layout {
            LutLayout::Volume3D => self.size,
            LutLayout::Tiled2D => self.size * self.size,
        }
    }

    /// Height in texels.
    pub fn height(&self) -> u32 {
        self.size
    }

    /// Depth in texels - always 1 for the tiled layout.
    pub fn depth(&self) -> u32 {
        match self.layout {
            LutLayout::Volume3D => self.size,
            LutLayout::Tiled2D => 1,
        }
    }

    /// Bytes per row, which the upload APIs ask for separately.
    pub fn bytes_per_row(&self) -> u32 {
        self.width() * 4 * std::mem::size_of::<f32>() as u32
    }

    /// Total size in bytes.
    pub fn byte_size(&self) -> usize {
        self.data.len() * std::mem::size_of::<f32>()
    }
}

/// Builds a native 3D texture payload.
///
/// The `.cube` order - red fastest, then green, then blue - is already the
/// row-major order a 3D texture expects, so this is a straight copy with alpha
/// padded in. Getting that wrong would mirror the color channels.
pub fn build_volume(lut: &Lut) -> Option<LutTexture> {
    let LutData::Lut3D { size, .. } = &lut.data else { return None };
    Some(LutTexture {
        size: *size as u32,
        data: lut.to_rgba_f32(),
        layout: LutLayout::Volume3D,
    })
}

/// Builds a tiled 2D payload, for backends without 3D sampling.
///
/// Slice `b` of the cube occupies the horizontal band `[b*N, (b+1)*N)`, so a
/// shader reads `(r + b*N, g)` and mixes two neighbouring slices to interpolate
/// along blue.
pub fn build_tiled(lut: &Lut) -> Option<LutTexture> {
    let LutData::Lut3D { size, data } = &lut.data else { return None };
    let n = *size;
    let mut out = vec![0.0f32; n * n * n * 4];

    for b in 0..n {
        for g in 0..n {
            for r in 0..n {
                let src = data[r + g * n + b * n * n];
                // Row `g` of the tiled image, column `r` inside slice `b`.
                let dst = (g * (n * n) + b * n + r) * 4;
                out[dst] = src[0];
                out[dst + 1] = src[1];
                out[dst + 2] = src[2];
                out[dst + 3] = 1.0;
            }
        }
    }

    Some(LutTexture { size: n as u32, data: out, layout: LutLayout::Tiled2D })
}

/// Cube edge the GPU texture is allocated at, regardless of the loaded LUT.
///
/// The bind group is built once, when the pipeline is created, and rebuilding it
/// later would mean retaining buffers this code drops on purpose (`buf_coeffs`,
/// `wgpu.rs:308`). Allocating for the largest LUT anyone ships instead makes a
/// LUT swap a plain texture write, with the binding untouched.
///
/// 65 is the largest of the three sizes in common use (17, 33, 65); at
/// `RGBA32F` the allocation is 65³·16 B ≈ 4.4 MB, negligible next to a 4K frame
/// buffer.
///
/// The parser accepts up to 256 (`parser.rs:131`), so a larger table is
/// **downsampled** here, and that direction does lose detail. It is a deliberate
/// trade: such files are rare, and the alternative - allocating 256³·16 B ≈
/// 268 MB for every session - is not worth paying on the common case.
pub const MAX_LUT_SIZE: usize = 65;

impl LutTexture {
    /// Resamples the payload to [`MAX_LUT_SIZE`], for the fixed GPU allocation.
    ///
    /// Upsampling a LUT is lossless in the only sense that matters: the result
    /// is trilinear interpolation of the source, which is exactly what the
    /// shader would have computed from the smaller table. A 33³ LUT resampled to
    /// 65³ and then sampled gives the same color as sampling the 33³ directly.
    pub fn resampled_to_max(lut: &Lut, layout: LutLayout) -> Option<Self> {
        if !lut.is_3d() { return None; }
        let n = MAX_LUT_SIZE;
        let last = (n - 1) as f32;
        let mut data = vec![0.0f32; n * n * n * 4];

        for b in 0..n {
            for g in 0..n {
                for r in 0..n {
                    // Sample the source at this cell's position in 0..1, mapped
                    // back through the domain so DOMAIN_MIN/MAX still apply.
                    let pos = [r as f32 / last, g as f32 / last, b as f32 / last];
                    let rgb = [
                        lut.domain_min[0] + pos[0] * (lut.domain_max[0] - lut.domain_min[0]),
                        lut.domain_min[1] + pos[1] * (lut.domain_max[1] - lut.domain_min[1]),
                        lut.domain_min[2] + pos[2] * (lut.domain_max[2] - lut.domain_min[2]),
                    ];
                    let out = lut.sample(rgb);
                    let dst = match layout {
                        LutLayout::Volume3D => (r + g * n + b * n * n) * 4,
                        LutLayout::Tiled2D  => (g * (n * n) + b * n + r) * 4,
                    };
                    data[dst]     = out[0];
                    data[dst + 1] = out[1];
                    data[dst + 2] = out[2];
                    data[dst + 3] = 1.0;
                }
            }
        }

        Some(Self { size: n as u32, data, layout })
    }

    /// An identity LUT at the full allocation size, for when none is loaded.
    pub fn identity_at_max(layout: LutLayout) -> Self {
        let n = MAX_LUT_SIZE;
        let last = (n - 1) as f32;
        let mut data = vec![0.0f32; n * n * n * 4];
        for b in 0..n {
            for g in 0..n {
                for r in 0..n {
                    let dst = match layout {
                        LutLayout::Volume3D => (r + g * n + b * n * n) * 4,
                        LutLayout::Tiled2D  => (g * (n * n) + b * n + r) * 4,
                    };
                    data[dst]     = r as f32 / last;
                    data[dst + 1] = g as f32 / last;
                    data[dst + 2] = b as f32 / last;
                    data[dst + 3] = 1.0;
                }
            }
        }
        Self { size: n as u32, data, layout }
    }

    /// The smallest LUT that changes nothing: the eight corners of the RGB cube.
    ///
    /// A bind group layout is fixed when the pipeline is built, and the pipeline
    /// cache key does not include which LUT is loaded
    /// (`stabilization/mod.rs:355`). So the binding cannot appear and disappear
    /// with the feature - something must always occupy it. This is that
    /// something: trilinear interpolation over these eight corners reproduces
    /// the input exactly, so an unloaded LUT costs a texture fetch and nothing
    /// else.
    pub fn identity(layout: LutLayout) -> Self {
        let mut data = Vec::with_capacity(8 * 4);
        // The two layouts do not share a traversal order, not even at N=2: the
        // cube runs blue-major, while the tiled image runs green-major, because
        // one row of it holds the same green across every blue slice.
        match layout {
            LutLayout::Volume3D => {
                for b in 0..2u32 {
                    for g in 0..2u32 {
                        for r in 0..2u32 {
                            data.extend_from_slice(&[r as f32, g as f32, b as f32, 1.0]);
                        }
                    }
                }
            }
            LutLayout::Tiled2D => {
                for g in 0..2u32 {
                    for b in 0..2u32 {
                        for r in 0..2u32 {
                            data.extend_from_slice(&[r as f32, g as f32, b as f32, 1.0]);
                        }
                    }
                }
            }
        }
        Self { size: 2, data, layout }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::lut::parse_cube_str;

    const IDENTITY_2: &str = "\
LUT_3D_SIZE 2
0.0 0.0 0.0
1.0 0.0 0.0
0.0 1.0 0.0
1.0 1.0 0.0
0.0 0.0 1.0
1.0 0.0 1.0
0.0 1.0 1.0
1.0 1.0 1.0
";

    #[test]
    fn volume_keeps_the_cube_order() {
        let lut = parse_cube_str(IDENTITY_2).unwrap();
        let tex = build_volume(&lut).expect("3D lut");

        assert_eq!(tex.size, 2);
        assert_eq!((tex.width(), tex.height(), tex.depth()), (2, 2, 2));
        assert_eq!(tex.data.len(), 8 * 4);
        // Texel 1 is (r=1,g=0,b=0): red must stay red through the upload.
        assert_eq!(&tex.data[4..8], &[1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn tiled_lays_blue_slices_side_by_side() {
        let lut = parse_cube_str(IDENTITY_2).unwrap();
        let tex = build_tiled(&lut).expect("3D lut");

        assert_eq!((tex.width(), tex.height(), tex.depth()), (4, 2, 1));
        assert_eq!(tex.data.len(), 8 * 4);

        // Row 0 holds g=0: slice b=0 in columns 0..2, slice b=1 in columns 2..4.
        // Column 0 is (0,0,0) and column 2 is (0,0,1) - pure blue.
        assert_eq!(&tex.data[0..4], &[0.0, 0.0, 0.0, 1.0]);
        assert_eq!(&tex.data[8..12], &[0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn both_layouts_hold_the_same_texels() {
        let lut = parse_cube_str(IDENTITY_2).unwrap();
        let vol = build_volume(&lut).unwrap();
        let tiled = build_tiled(&lut).unwrap();
        assert_eq!(vol.data.len(), tiled.data.len(), "same payload, different order");
        assert_eq!(vol.byte_size(), tiled.byte_size());
    }

    #[test]
    fn the_identity_matches_what_the_parser_produces() {
        // The fallback LUT is hand-built, not parsed, so nothing guarantees it
        // agrees with the real thing except a test. If the channel order drifts,
        // "no LUT loaded" would silently mirror the colors.
        let parsed = parse_cube_str(IDENTITY_2).unwrap();

        let vol = LutTexture::identity(LutLayout::Volume3D);
        assert_eq!(vol.data, build_volume(&parsed).unwrap().data);

        let tiled = LutTexture::identity(LutLayout::Tiled2D);
        assert_eq!(tiled.data, build_tiled(&parsed).unwrap().data);
    }

    #[test]
    fn resampling_to_max_preserves_the_transform() {
        // The claim that upsampling costs nothing only holds because the source
        // is interpolated trilinearly and the shader would interpolate the same
        // way. If resampled_to_max ever indexes wrong, this catches it.
        let src = parse_cube_str(IDENTITY_2).unwrap();
        let tex = LutTexture::resampled_to_max(&src, LutLayout::Volume3D).unwrap();
        assert_eq!(tex.size as usize, MAX_LUT_SIZE);

        let n = MAX_LUT_SIZE;
        for &(r, g, b) in &[(0, 0, 0), (n - 1, 0, 0), (0, n - 1, 0), (0, 0, n - 1), (n / 2, n / 3, n - 1)] {
            let dst = (r + g * n + b * n * n) * 4;
            let last = (n - 1) as f32;
            // An identity source must map each cell back to its own position.
            for (ch, idx) in [(r, 0), (g, 1), (b, 2)] {
                assert!((tex.data[dst + idx] - ch as f32 / last).abs() < 1e-5,
                    "cell ({r},{g},{b}) channel {idx} drifted: {}", tex.data[dst + idx]);
            }
        }
    }

    #[test]
    fn the_max_size_identity_is_flat() {
        let tex = LutTexture::identity_at_max(LutLayout::Volume3D);
        let built = LutTexture::resampled_to_max(&parse_cube_str(IDENTITY_2).unwrap(), LutLayout::Volume3D).unwrap();
        assert_eq!(tex.data.len(), built.data.len());
        for (a, b) in tex.data.iter().zip(built.data.iter()) {
            assert!((a - b).abs() < 1e-5, "identity shortcut disagrees with the resampled identity");
        }
    }

    /// Every backend normalizes by `max_pixel_value` before indexing the LUT and
    /// scales back afterwards, because the pipeline works in 0..255 / 0..1023 /
    /// 0..65535 rather than 0..1. Four implementations of that step exist - WGSL,
    /// OpenCL C, the CPU path and this one - and getting it wrong yields an image
    /// that is merely a bit off, not obviously broken. This pins the contract.
    #[test]
    fn sampling_round_trips_through_the_pipeline_scale() {
        let lut = parse_cube_str(IDENTITY_2).unwrap();

        for max_pixel_value in [255.0f32, 1023.0, 65535.0] {
            for level in [0.0f32, 0.25, 0.5, 1.0] {
                let stored = level * max_pixel_value;
                let normalized = (stored / max_pixel_value).clamp(0.0, 1.0);
                let out = lut.sample([normalized, normalized, normalized]);
                let back = out[0] * max_pixel_value;
                assert!((back - stored).abs() < 0.01,
                    "identity LUT changed {stored} at scale {max_pixel_value} (got {back})");
            }
        }
    }

    /// An inverting LUT must actually invert - proof that `sample` applies the
    /// table rather than passing the input through, which an identity-only test
    /// cannot distinguish.
    #[test]
    fn an_inverting_lut_inverts() {
        let inverted = parse_cube_str("\
LUT_3D_SIZE 2
1.0 1.0 1.0
0.0 1.0 1.0
1.0 0.0 1.0
0.0 0.0 1.0
1.0 1.0 0.0
0.0 1.0 0.0
1.0 0.0 0.0
0.0 0.0 0.0
").unwrap();

        assert_eq!(inverted.sample([0.0, 0.0, 0.0]), [1.0, 1.0, 1.0]);
        assert_eq!(inverted.sample([1.0, 1.0, 1.0]), [0.0, 0.0, 0.0]);

        let mid = inverted.sample([0.25, 0.25, 0.25]);
        for ch in mid { assert!((ch - 0.75).abs() < 1e-5, "midtone should invert to 0.75, got {ch}"); }
    }

    /// The intensity slider blends the graded pixel against the original. Four
    /// backends implement that line, and the endpoints are what a user notices:
    /// 0 must be a true no-op, 1 must be the full grade.
    #[test]
    fn the_intensity_blend_hits_its_endpoints() {
        let inverted = parse_cube_str("\
LUT_3D_SIZE 2
1.0 1.0 1.0
0.0 1.0 1.0
1.0 0.0 1.0
0.0 0.0 1.0
1.0 1.0 0.0
0.0 1.0 0.0
1.0 0.0 0.0
0.0 0.0 0.0
").unwrap();

        // Mirrors what every shader does: mix(original, graded, amount).
        let blend = |original: f32, amount: f32| -> f32 {
            let graded = inverted.sample([original, original, original])[0];
            original + (graded - original) * amount
        };

        for &original in &[0.0f32, 0.25, 0.5, 1.0] {
            assert!((blend(original, 0.0) - original).abs() < 1e-6,
                "amount 0 must leave {original} untouched");
            assert!((blend(original, 1.0) - (1.0 - original)).abs() < 1e-5,
                "amount 1 must fully invert {original}");
            // Half strength lands halfway between the two.
            let expected = original + ((1.0 - original) - original) * 0.5;
            assert!((blend(original, 0.5) - expected).abs() < 1e-5,
                "amount 0.5 must be the midpoint for {original}");
        }
    }

    #[test]
    fn a_1d_lut_has_no_volume_form() {
        let lut = parse_cube_str("LUT_1D_SIZE 2\n0 0 0\n1 1 1\n").unwrap();
        assert!(build_volume(&lut).is_none());
        assert!(build_tiled(&lut).is_none());
    }
}
