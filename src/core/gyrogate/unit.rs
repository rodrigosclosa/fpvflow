// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2026 Rodrigo Sclosa

//! Which gyro a DJI O4 unit most likely has, from the clip header.
//!
//! The file does not name the gyro chip. DJI switched the O4 Air Unit Pro from the MP66 to the
//! ICM-40609-D in February 2026, and the O4 (Lite) has had the ICM-40609-D since launch. The
//! serial number encodes the production date: after the 4-character prefix come the year
//! (N = 2025, P = 2026) and the month (1-9, A, B, C), e.g. `9F2KN8W…` is August 2025.
//! Sources: Oscar Liang's article on the O4 Pro gyro change and the DJIFix notes, which call
//! January 2026 (`9F2KP1…`) a grey zone.

/// Likely gyro of the unit that recorded a clip.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Chip {
    /// O4 (Lite): always the ICM-40609-D.
    Icm40609d,
    /// O4 Pro built in February 2026 or later.
    Icm40609dLikely,
    /// O4 Pro built in January 2026, when both gyros were shipping.
    Unsure,
    /// O4 Pro built before 2026: the old MP66 gyro.
    Mp66,
    /// O4 Pro whose serial number could not be decoded.
    Unknown,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct UnitInfo {
    /// `O4` or `O4P`.
    pub product: String,
    pub serial: Option<String>,
    pub firmware: Option<String>,
    /// Production year and month decoded from the serial number.
    pub built: Option<(u16, u8)>,
    pub chip: Chip,
}

/// The unit behind a DJI O4 / O4 Pro clip, from `FileMetadata::additional_data`
/// (`clip_meta_header.product_name`, `product_sn`, `product_firmware_version`).
/// `None` for anything else.
pub fn classify(additional_data: &serde_json::Value) -> Option<UnitInfo> {
    let header = additional_data.get("clip_meta_header")?;
    let text = |key: &str| header.get(key).and_then(|v| v.as_str()).map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned);
    let product = text("product_name")?;
    let product = product.strip_prefix("DJI ").unwrap_or(&product).trim().to_owned();
    if product != "O4" && product != "O4P" {
        return None;
    }
    let serial = text("product_sn");
    let built = serial.as_deref().and_then(production_date);
    let chip = if product == "O4" {
        Chip::Icm40609d
    } else {
        match built {
            Some((y, m)) if (y, m) >= (2026, 2) => Chip::Icm40609dLikely,
            Some((2026, 1)) => Chip::Unsure,
            Some(_) => Chip::Mp66,
            None => Chip::Unknown,
        }
    };
    Some(UnitInfo { product, serial, firmware: text("product_firmware_version"), built, chip })
}

/// Production year and month from an O4 Pro serial number, e.g. `9F2KN8W01100R32` -> (2025, 8).
pub fn production_date(serial: &str) -> Option<(u16, u8)> {
    let bytes = serial.as_bytes();
    if bytes.len() < 7 {
        return None;
    }
    // The year letters skip I and O, like the rest of DJI's serial alphabet.
    let year = match bytes[4] {
        b'L' => 2023, b'M' => 2024, b'N' => 2025, b'P' => 2026, b'Q' => 2027, b'R' => 2028, b'S' => 2029, b'T' => 2030,
        _ => return None,
    };
    let month = match bytes[5] {
        c @ b'1'..=b'9' => c - b'0',
        b'A' => 10, b'B' => 11, b'C' => 12,
        _ => return None,
    };
    Some((year, month))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(name: &str, sn: &str) -> serde_json::Value {
        serde_json::json!({ "clip_meta_header": { "product_name": name, "product_sn": sn, "product_firmware_version": "01.00.06.00" } })
    }

    #[test]
    fn serial_numbers_decode_to_the_production_month() {
        assert_eq!(production_date("9F2KN8W01100R32"), Some((2025, 8)));
        assert_eq!(production_date("9F2KN37001021C2"), Some((2025, 3)));
        assert_eq!(production_date("9F2KP2A00000000"), Some((2026, 2)));
        assert_eq!(production_date("9F2KPC100000000"), Some((2026, 12)));
        assert_eq!(production_date("9F2KX1100000000"), None);
        assert_eq!(production_date("9F2K"), None);
    }

    #[test]
    fn units_are_classified_by_product_and_date() {
        assert_eq!(classify(&header("DJI O4P", "9F2KN8W01100R32")).unwrap().chip, Chip::Mp66);
        assert_eq!(classify(&header("DJI O4P", "9F2KP1R00000000")).unwrap().chip, Chip::Unsure);
        assert_eq!(classify(&header("DJI O4P", "9F2KP2100000000")).unwrap().chip, Chip::Icm40609dLikely);
        assert_eq!(classify(&header("DJI O4P", "")).unwrap().chip, Chip::Unknown);
        let lite = classify(&header("DJI O4", "")).unwrap();
        assert_eq!((lite.product.as_str(), lite.chip, lite.serial), ("O4", Chip::Icm40609d, None));
        assert!(classify(&header("DJI Avata 2", "9F2KP2100000000")).is_none());
        assert!(classify(&serde_json::json!({})).is_none());
        assert!(classify(&serde_json::json!({ "clip_meta_header": null })).is_none());
    }
}
