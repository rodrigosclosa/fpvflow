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
    fn a_1d_lut_has_no_volume_form() {
        let lut = parse_cube_str("LUT_1D_SIZE 2\n0 0 0\n1 1 1\n").unwrap();
        assert!(build_volume(&lut).is_none());
        assert!(build_tiled(&lut).is_none());
    }
}
