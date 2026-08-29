// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2026 Rodrigo Sclosa

//! Reader for the Adobe Cube (`.cube`) text format.
//!
//! The format is a header of keywords followed by whitespace-separated RGB
//! triplets. What it does not carry is any indication of the color space it
//! expects on the input: a table built for DLog-M produces nonsense when fed
//! Rec.709. That is the user's call, not something this parser can check.

use super::{Lut, LutData};
use std::fmt;

/// Why a `.cube` file could not be read.
///
/// Every variant carries enough context to be shown in the interface - the user
/// picked a file and deserves to know what is wrong with it.
#[derive(Debug, Clone, PartialEq)]
pub enum LutParseError {
    /// The file could not be opened or read.
    Io(String),
    /// A `LUT_*_SIZE` line was present but the value made no sense.
    InvalidSize {
        /// What the file declared.
        found: String,
    },
    /// Neither `LUT_1D_SIZE` nor `LUT_3D_SIZE` was found.
    MissingSize,
    /// Both `LUT_1D_SIZE` and `LUT_3D_SIZE` were declared.
    ConflictingSize,
    /// A data line did not hold three parseable floats.
    InvalidData {
        /// 1-based line number, to point the user at it.
        line: usize,
        /// The offending text.
        text: String,
    },
    /// The number of data rows does not match the declared size.
    ///
    /// This is the check that catches a truncated download, which is the most
    /// common way a `.cube` goes wrong in practice.
    SizeMismatch {
        /// Rows the header implies.
        expected: usize,
        /// Rows actually found.
        found: usize,
    },
    /// A `DOMAIN_MIN`/`DOMAIN_MAX` line was malformed.
    InvalidDomain {
        /// 1-based line number.
        line: usize,
    },
}

impl fmt::Display for LutParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "Could not read the file: {e}"),
            Self::InvalidSize { found } => write!(f, "Invalid LUT size: {found}"),
            Self::MissingSize => write!(f, "No LUT_1D_SIZE or LUT_3D_SIZE in the file"),
            Self::ConflictingSize => write!(f, "The file declares both LUT_1D_SIZE and LUT_3D_SIZE"),
            Self::InvalidData { line, text } => write!(f, "Line {line} is not three numbers: {text}"),
            Self::SizeMismatch { expected, found } => {
                write!(f, "Expected {expected} entries, found {found} - the file looks truncated")
            }
            Self::InvalidDomain { line } => write!(f, "Malformed DOMAIN on line {line}"),
        }
    }
}

impl std::error::Error for LutParseError {}

/// Reads a `.cube` file from disk.
///
/// Goes through [`filesystem::open_file`](fpvflow_core::filesystem) so it works
/// on Android too, where a path is not necessarily a path.
pub fn parse_cube(url: &str) -> Result<Lut, LutParseError> {
    let mut file =
        crate::filesystem::open_file(url, false, false).map_err(|e| LutParseError::Io(e.to_string()))?;
    let size = file.size;
    let text = {
        use std::io::Read;
        let mut buf = String::with_capacity(size);
        file.get_file()
            .read_to_string(&mut buf)
            .map_err(|e| LutParseError::Io(e.to_string()))?;
        buf
    };
    let mut lut = parse_cube_str(&text)?;
    lut.path = url.to_string();
    Ok(lut)
}

/// Parses `.cube` content already in memory.
///
/// Split out from [`parse_cube`] so the format can be tested without touching
/// the filesystem.
pub fn parse_cube_str(text: &str) -> Result<Lut, LutParseError> {
    let mut title = None;
    let mut size_1d: Option<usize> = None;
    let mut size_3d: Option<usize> = None;
    let mut domain_min = [0.0f32; 3];
    let mut domain_max = [1.0f32; 3];
    let mut data: Vec<[f32; 3]> = Vec::new();

    for (i, raw) in text.lines().enumerate() {
        let line_no = i + 1;
        // Comments run to the end of the line, and a stray BOM would otherwise
        // poison the first keyword.
        let line = raw.trim_start_matches('\u{feff}').trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split_whitespace();
        let keyword = parts.next().unwrap_or_default();

        match keyword {
            "TITLE" => {
                // TITLE "Some name" - quotes are conventional but not required.
                let rest = line["TITLE".len()..].trim();
                title = Some(rest.trim_matches('"').to_string());
            }
            "LUT_1D_SIZE" | "LUT_3D_SIZE" => {
                let value = parts.next().unwrap_or_default();
                let n: usize = value.parse().map_err(|_| LutParseError::InvalidSize {
                    found: value.to_string(),
                })?;
                // A single-entry table cannot interpolate anything, and the
                // upper bound keeps a bogus header from asking for terabytes.
                if n < 2 || n > 256 {
                    return Err(LutParseError::InvalidSize { found: n.to_string() });
                }
                if keyword == "LUT_1D_SIZE" {
                    size_1d = Some(n);
                } else {
                    size_3d = Some(n);
                }
            }
            "DOMAIN_MIN" | "DOMAIN_MAX" => {
                let values: Vec<f32> = parts.filter_map(|v| v.parse().ok()).collect();
                if values.len() != 3 {
                    return Err(LutParseError::InvalidDomain { line: line_no });
                }
                let target = if keyword == "DOMAIN_MIN" { &mut domain_min } else { &mut domain_max };
                target.copy_from_slice(&values);
            }
            // Anything else is either a data row or a keyword we do not use
            // (LUT_3D_INPUT_RANGE and friends). Numbers start with a digit,
            // a sign or a dot; keywords do not.
            _ => {
                let first = keyword.as_bytes()[0];
                if !(first.is_ascii_digit() || first == b'-' || first == b'+' || first == b'.') {
                    continue;
                }
                let values: Vec<f32> = line.split_whitespace().filter_map(|v| v.parse().ok()).collect();
                if values.len() != 3 {
                    return Err(LutParseError::InvalidData {
                        line: line_no,
                        text: line.to_string(),
                    });
                }
                data.push([values[0], values[1], values[2]]);
            }
        }
    }

    let lut_data = match (size_1d, size_3d) {
        (Some(_), Some(_)) => return Err(LutParseError::ConflictingSize),
        (None, None) => return Err(LutParseError::MissingSize),
        (Some(n), None) => {
            if data.len() != n {
                return Err(LutParseError::SizeMismatch { expected: n, found: data.len() });
            }
            LutData::Lut1D { size: n, data }
        }
        (None, Some(n)) => {
            let expected = n * n * n;
            if data.len() != expected {
                return Err(LutParseError::SizeMismatch { expected, found: data.len() });
            }
            LutData::Lut3D { size: n, data }
        }
    };

    Ok(Lut {
        path: String::new(),
        title,
        domain_min,
        domain_max,
        data: lut_data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smallest possible 3D table that is an exact identity: the eight corners
    /// of the unit cube, in the order the format mandates - red fastest.
    const IDENTITY_2: &str = "\
TITLE \"identity\"
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
    fn identity_lut_returns_the_input_untouched() {
        let lut = parse_cube_str(IDENTITY_2).expect("parse");
        assert_eq!(lut.size(), 2);
        assert!(lut.is_3d());
        assert_eq!(lut.title.as_deref(), Some("identity"));

        // The anchor test of the whole feature: if this drifts, the LUT pass
        // will be tinting images that should come out untouched.
        for &c in &[
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
            [0.5, 0.25, 0.75],
            [0.1, 0.9, 0.4],
        ] {
            let out = lut.sample(c);
            for ch in 0..3 {
                assert!(
                    (out[ch] - c[ch]).abs() < 1e-5,
                    "channel {ch} of {c:?} came back as {out:?}"
                );
            }
        }
    }

    /// Swapping the scan order mirrors the image, and stays invisible on grey.
    /// This pins the convention down with a table that is asymmetric on purpose.
    #[test]
    fn red_varies_fastest() {
        let lut = parse_cube_str(IDENTITY_2).expect("parse");
        let LutData::Lut3D { data, .. } = &lut.data else { panic!("expected 3D") };

        // Index 1 is (r=1, g=0, b=0). Read in the wrong order it would be blue.
        assert_eq!(data[1], [1.0, 0.0, 0.0], "index 1 must be pure red");
        assert_eq!(data[2], [0.0, 1.0, 0.0], "index 2 must be pure green");
        assert_eq!(data[4], [0.0, 0.0, 1.0], "index 4 must be pure blue");

        // And through sampling: pure red in, pure red out.
        let out = lut.sample([1.0, 0.0, 0.0]);
        assert!((out[0] - 1.0).abs() < 1e-5 && out[1] < 1e-5 && out[2] < 1e-5, "got {out:?}");
    }

    #[test]
    fn domain_is_respected() {
        // Same identity cube, but declaring that the input runs 0..0.5. A value
        // of 0.5 therefore lands at the top of the table, not the middle.
        let text = IDENTITY_2.replace(
            "LUT_3D_SIZE 2",
            "DOMAIN_MIN 0.0 0.0 0.0\nDOMAIN_MAX 0.5 0.5 0.5\nLUT_3D_SIZE 2",
        );
        let lut = parse_cube_str(&text).expect("parse");
        assert_eq!(lut.domain_max, [0.5, 0.5, 0.5]);

        let out = lut.sample([0.5, 0.5, 0.5]);
        for ch in 0..3 {
            assert!((out[ch] - 1.0).abs() < 1e-5, "0.5 should map to the top, got {out:?}");
        }

        // Above the domain the value clamps instead of extrapolating.
        let out = lut.sample([0.9, 0.9, 0.9]);
        for ch in 0..3 {
            assert!((out[ch] - 1.0).abs() < 1e-5, "expected clamping, got {out:?}");
        }
    }

    #[test]
    fn one_dimensional_lut_parses() {
        let text = "LUT_1D_SIZE 3\n0.0 0.0 0.0\n0.5 0.5 0.5\n1.0 1.0 1.0\n";
        let lut = parse_cube_str(text).expect("parse");
        assert_eq!(lut.size(), 3);
        assert!(!lut.is_3d());

        let out = lut.sample([0.5, 0.5, 0.5]);
        for ch in 0..3 {
            assert!((out[ch] - 0.5).abs() < 1e-5, "got {out:?}");
        }
    }

    #[test]
    fn truncated_file_is_rejected() {
        // Seven rows where the header promises eight - what a partial download
        // looks like.
        let text = IDENTITY_2.rsplit_once("1.0 1.0 1.0\n").unwrap().0.to_string();
        match parse_cube_str(&text) {
            Err(LutParseError::SizeMismatch { expected, found }) => {
                assert_eq!(expected, 8);
                assert_eq!(found, 7);
            }
            other => panic!("expected SizeMismatch, got {other:?}"),
        }
    }

    #[test]
    fn missing_size_is_rejected() {
        let text = "TITLE \"no size\"\n0.0 0.0 0.0\n1.0 1.0 1.0\n";
        assert_eq!(parse_cube_str(text), Err(LutParseError::MissingSize));
    }

    #[test]
    fn garbage_data_line_is_rejected() {
        let text = "LUT_3D_SIZE 2\n0.0 0.0 0.0\n1.0 oops 0.0\n";
        match parse_cube_str(text) {
            Err(LutParseError::InvalidData { line, .. }) => assert_eq!(line, 3),
            other => panic!("expected InvalidData, got {other:?}"),
        }
    }

    #[test]
    fn comments_and_blank_lines_are_skipped() {
        let text = format!("# a comment\n\n{IDENTITY_2}\n# trailing comment\n");
        let lut = parse_cube_str(&text).expect("parse");
        assert_eq!(lut.size(), 2);
    }

    #[test]
    fn rgba_upload_pads_alpha() {
        let lut = parse_cube_str(IDENTITY_2).expect("parse");
        let texels = lut.to_rgba_f32();
        assert_eq!(texels.len(), 8 * 4, "eight RGBA texels");
        assert_eq!(&texels[0..4], &[0.0, 0.0, 0.0, 1.0]);
        assert_eq!(&texels[4..8], &[1.0, 0.0, 0.0, 1.0]);
    }
}
