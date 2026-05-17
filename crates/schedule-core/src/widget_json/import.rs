/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Widget JSON import functionality.
//!
//! This module provides functions for importing widget JSON data into a Schedule,
//! with best-effort reconstruction of entities and relationships.

use std::path::Path;

use super::export::WidgetJsonError;
use super::types::WidgetExport;

/// Import widget JSON from a file.
pub fn load_from_file(path: &Path) -> Result<WidgetExport, WidgetJsonError> {
    let json = std::fs::read_to_string(path)?;
    load_from_json(&json)
}

/// Import widget JSON from a string.
pub fn load_from_json(json: &str) -> Result<WidgetExport, WidgetJsonError> {
    Ok(serde_json::from_str(json)?)
}

/// Import widget JSON from a URL by extracting embedded gzip+base64 data.
///
/// Fetches the webpage, finds the `<script type="application/json" id="cosam-schedule-data">`
/// tag, extracts the base64-encoded gzip data, decompresses it, and parses the JSON.
pub fn load_from_url(url: &str) -> Result<WidgetExport, WidgetJsonError> {
    // Fetch the webpage
    let response = reqwest::blocking::get(url)?;
    let html = response.text()?;

    // Parse HTML and extract the embedded data
    let document = scraper::Html::parse_document(&html);
    let selector =
        scraper::Selector::parse(r#"script[type="application/json"][id="cosam-schedule-data"]"#)
            .map_err(|e| WidgetJsonError::DataExtraction(format!("Invalid selector: {}", e)))?;

    let script_element = document.select(&selector).next().ok_or_else(|| {
        WidgetJsonError::DataExtraction(
            "No script tag with id='cosam-schedule-data' found in webpage".to_string(),
        )
    })?;

    let encoded_data = script_element
        .text()
        .collect::<Vec<_>>()
        .join("")
        .trim()
        .to_string();

    if encoded_data.is_empty() {
        return Err(WidgetJsonError::DataExtraction(
            "Script tag is empty".to_string(),
        ));
    }

    // Decode and decompress the data
    let json_data = decode_gzip_base64(&encoded_data)?;

    // Parse the JSON
    load_from_json(&json_data)
}

/// Decode gzip+base64 encoded data to a JSON string.
///
/// Handles both gzip-compressed base64 data (detected by "H4sI" prefix)
/// and plain base64-encoded JSON.
fn decode_gzip_base64(encoded: &str) -> Result<String, WidgetJsonError> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use flate2::read::GzDecoder;
    use std::io::Read as _;

    let encoded = encoded.trim();

    // Decode base64
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|e| WidgetJsonError::Base64Decode(format!("{}", e)))?;

    // Check if it's gzip-compressed (H4sI is the gzip magic number in base64)
    let json_string = if encoded.starts_with("H4sI") {
        // Decompress gzip
        let mut decoder = GzDecoder::new(&bytes[..]);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| WidgetJsonError::GzipDecompress(format!("{}", e)))?;
        String::from_utf8(decompressed)
            .map_err(|e| WidgetJsonError::GzipDecompress(format!("Invalid UTF-8: {}", e)))?
    } else {
        // Plain base64-encoded JSON
        String::from_utf8(bytes)
            .map_err(|e| WidgetJsonError::Base64Decode(format!("Invalid UTF-8: {}", e)))?
    };

    Ok(json_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_decode_gzip_base64_plain() {
        // Test plain base64-encoded JSON (not gzip)
        let json = r#"{"test":"value"}"#;
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        let encoded = STANDARD.encode(json.as_bytes());
        let decoded = decode_gzip_base64(&encoded).unwrap();
        assert_eq!(decoded, json);
    }

    #[test]
    fn test_decode_gzip_base64_compressed() {
        // Test gzip+base64 encoded JSON
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write as _;

        let json = r#"{"test":"value","nested":{"key":"data"}}"#;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(json.as_bytes()).unwrap();
        let compressed = encoder.finish().unwrap();
        let encoded = STANDARD.encode(&compressed);

        // Should start with H4sI (gzip magic number in base64)
        assert!(encoded.starts_with("H4sI"));

        let decoded = decode_gzip_base64(&encoded).unwrap();
        assert_eq!(decoded, json);
    }

    #[test]
    fn test_decode_gzip_base64_invalid_base64() {
        let result = decode_gzip_base64("invalid!base64");
        assert!(result.is_err());
        matches!(result.unwrap_err(), WidgetJsonError::Base64Decode(_));
    }
}
