// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2026 Rodrigo Sclosa

//! Color conversion LUTs in the Adobe Cube (`.cube`) format.
//!
//! The target use is a conversion LUT - log to display, such as the DLog-M to
//! Rec.709 table DJI ships - applied during processing so the exported file is
//! already graded.
//!
//! # Sampling happens in float
//!
//! A log to Rec.709 conversion stretches the dynamic range non-linearly and
//! shows banding at once when applied at 8 or 10 bit, precisely in the skies and
//! gradients this kind of footage is full of. Everything here works in `f32`;
//! quantization back to the output bit depth belongs at the very end of the
//! pipeline, after the LUT and every adjustment.
//!
//! # Data layout
//!
//! In a `.cube` file the **red index varies fastest**, then green, then blue.
//! Reading it in any other order mirrors the color channels, which goes
//! unnoticed on neutral images and is obvious on anything saturated - hence
//! [`Lut::sample`] and the identity test that pins the convention down.

pub mod gpu;
pub mod parser;

pub use gpu::{build_tiled, build_volume, LutLayout, LutTexture};
pub use parser::{parse_cube, parse_cube_str, LutParseError};

/// Table payload, either a per-channel curve or a full color cube.
#[derive(Debug, Clone, PartialEq)]
pub enum LutData {
    /// Per-channel curve: `size` entries, each mapping one input level.
    Lut1D {
        /// Number of entries.
        size: usize,
        /// `size` RGB triplets.
        data: Vec<[f32; 3]>,
    },
    /// Color cube: `size³` entries, red index varying fastest.
    Lut3D {
        /// Cube edge length (17, 33 and 65 are the usual values).
        size: usize,
        /// `size³` RGB triplets, indexed `r + g * size + b * size²`.
        data: Vec<[f32; 3]>,
    },
}

/// A parsed `.cube` file.
#[derive(Debug, Clone, PartialEq)]
pub struct Lut {
    /// Where it was loaded from, so a project can reload it later.
    pub path: String,
    /// `TITLE` field, when the file declares one.
    pub title: Option<String>,
    /// Lower bound of the input domain, per channel.
    pub domain_min: [f32; 3],
    /// Upper bound of the input domain, per channel.
    pub domain_max: [f32; 3],
    /// The table itself.
    pub data: LutData,
}

impl Lut {
    /// Cube edge length for a 3D table, or entry count for a 1D one.
    pub fn size(&self) -> usize {
        match &self.data {
            LutData::Lut1D { size, .. } | LutData::Lut3D { size, .. } => *size,
        }
    }

    /// Whether this is a 3D table - the case the GPU path targets.
    pub fn is_3d(&self) -> bool {
        matches!(self.data, LutData::Lut3D { .. })
    }

    /// Maps a color into the declared domain, as a 0..1 position in the table.
    ///
    /// A LUT is not required to cover 0..1: `DOMAIN_MIN`/`DOMAIN_MAX` may narrow
    /// or widen it, and ignoring them yields a subtly wrong result - the kind
    /// that survives review because the image still looks plausible.
    fn normalize(&self, rgb: [f32; 3]) -> [f32; 3] {
        let mut out = [0.0f32; 3];
        for i in 0..3 {
            let span = self.domain_max[i] - self.domain_min[i];
            out[i] = if span.abs() < f32::EPSILON {
                0.0
            } else {
                ((rgb[i] - self.domain_min[i]) / span).clamp(0.0, 1.0)
            };
        }
        out
    }

    /// Samples the table with linear interpolation.
    ///
    /// This mirrors what the GPU does with a linear sampler on a 3D texture, and
    /// exists so the conversion can be tested - and the identity LUT proven to
    /// be an identity - without a GPU.
    pub fn sample(&self, rgb: [f32; 3]) -> [f32; 3] {
        let p = self.normalize(rgb);
        match &self.data {
            LutData::Lut1D { size, data } => {
                let mut out = [0.0f32; 3];
                for ch in 0..3 {
                    let x = p[ch] * (*size as f32 - 1.0);
                    let i0 = (x.floor() as usize).min(size - 1);
                    let i1 = (i0 + 1).min(size - 1);
                    let f = x - i0 as f32;
                    out[ch] = data[i0][ch] * (1.0 - f) + data[i1][ch] * f;
                }
                out
            }
            LutData::Lut3D { size, data } => {
                let n = *size;
                let last = n - 1;
                // Position in cell units, then the surrounding cell corners.
                let x = p[0] * last as f32;
                let y = p[1] * last as f32;
                let z = p[2] * last as f32;

                let (x0, y0, z0) = (
                    (x.floor() as usize).min(last),
                    (y.floor() as usize).min(last),
                    (z.floor() as usize).min(last),
                );
                let (x1, y1, z1) = ((x0 + 1).min(last), (y0 + 1).min(last), (z0 + 1).min(last));
                let (fx, fy, fz) = (x - x0 as f32, y - y0 as f32, z - z0 as f32);

                // Red fastest, then green, then blue.
                let at = |r: usize, g: usize, b: usize| -> [f32; 3] { data[r + g * n + b * n * n] };

                let mut out = [0.0f32; 3];
                for ch in 0..3 {
                    let c000 = at(x0, y0, z0)[ch];
                    let c100 = at(x1, y0, z0)[ch];
                    let c010 = at(x0, y1, z0)[ch];
                    let c110 = at(x1, y1, z0)[ch];
                    let c001 = at(x0, y0, z1)[ch];
                    let c101 = at(x1, y0, z1)[ch];
                    let c011 = at(x0, y1, z1)[ch];
                    let c111 = at(x1, y1, z1)[ch];

                    let c00 = c000 * (1.0 - fx) + c100 * fx;
                    let c10 = c010 * (1.0 - fx) + c110 * fx;
                    let c01 = c001 * (1.0 - fx) + c101 * fx;
                    let c11 = c011 * (1.0 - fx) + c111 * fx;

                    let c0 = c00 * (1.0 - fy) + c10 * fy;
                    let c1 = c01 * (1.0 - fy) + c11 * fy;

                    out[ch] = c0 * (1.0 - fz) + c1 * fz;
                }
                out
            }
        }
    }

    /// Flattens the table into `RGBA32F` texels for upload as a 3D texture.
    ///
    /// Alpha is padded to 1.0 because the GPU backends have no three-channel
    /// float format in common.
    pub fn to_rgba_f32(&self) -> Vec<f32> {
        let data = match &self.data {
            LutData::Lut1D { data, .. } | LutData::Lut3D { data, .. } => data,
        };
        let mut out = Vec::with_capacity(data.len() * 4);
        for texel in data {
            out.extend_from_slice(&[texel[0], texel[1], texel[2], 1.0]);
        }
        out
    }
}
