/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Schedule configuration: branding and print-format presets.
//!
//! This is presentation configuration, not schedule data. It's kept separate
//! from the schedule itself so the same schedule can be displayed with
//! different branding or print formats without modifying the core data.

use serde::{Deserialize, Serialize};

/// Schedule configuration: branding and print-format presets.
///
/// This is presentation configuration, not schedule data. It's kept separate
/// from the schedule itself so the same schedule can be displayed with
/// different branding or print formats without modifying the core data.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleConfig {
    /// Configuration format version. Consumers branch on this to handle
    /// structural changes to the config format.
    pub version: i32,
    /// Brand palette, logos, and print-font substitutes. Populated by
    /// `cosam-convert` from `config/brand.toml`; absent when no brand config is
    /// available. See `docs/widget-json-format.md`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brand: Option<ScheduleBrand>,
    /// Shipped default print formats seeding the widget's print dropdown.
    /// Omitted when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub print_formats: Vec<SchedulePrintFormat>,
}

impl ScheduleConfig {
    /// Parse from a config JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Load from a config JSON file.
    pub fn load(path: &std::path::Path) -> Result<Self, std::io::Error> {
        let json = std::fs::read_to_string(path)?;
        Self::from_json(&json).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

/// Brand palette, logos, and print-font substitutes carried to the widget so
/// its print formats can match the printed/PDF house style. Populated by
/// `cosam-convert` from `config/brand.toml`; absent when no brand config is
/// available. See `docs/widget-json-format.md`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleBrand {
    pub colors: ScheduleBrandColors,
    /// Logo alias (e.g. `"brand"`, `"small"`) → URL usable in `<img src>`:
    /// either a `data:` URL (base64-embedded file) or an `http(s)` URL.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub logos: std::collections::HashMap<String, String>,
    /// Web-equivalent fonts for print, one entry per configured role.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fonts: Vec<SchedulePrintFont>,
    pub meta: ScheduleBrandMeta,
}

/// Brand color palette mirrored from `brand.toml [colors]`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleBrandColors {
    pub primary: String,
    pub black: String,
    pub dark_grey: String,
    pub white: String,
}

/// One web-equivalent print font (a role's browser substitute).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SchedulePrintFont {
    /// `"heading"`, `"banner"`, `"subheading"`, or `"body"`.
    pub role: String,
    /// CSS font-family to apply.
    pub family: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// Google Fonts stylesheet URL the print window loads via `<link>`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_url: Option<String>,
}

/// Brand metadata mirrored from `brand.toml [meta]`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleBrandMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_url: Option<String>,
}

/// A shipped default print format the widget seeds its print-format dropdown
/// with. Authored in `config/widget-default.toml` (overridable by
/// `config/widget.toml`) and resolved by `cosam-convert`. References brand by
/// alias (`logo`) and role (`fonts.*`) so `brand.toml` stays the identity
/// source. Empty-string fields mean "use the widget default".
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SchedulePrintFormat {
    pub name: String,
    /// `"both"`, `"gridOnly"`, `"descriptionOnly"`, or `"panelList"`.
    pub content_mode: String,
    /// `"color"` or `"bw"`.
    pub color_mode: String,
    /// Column count for description/list regions; `0` = per-mode auto default.
    #[serde(default)]
    pub columns: u32,
    #[serde(default)]
    pub header_text: String,
    #[serde(default)]
    pub footer_text: String,
    /// `"full"`, `"timestamp"`, or `"none"`.
    pub footer_mode: String,
    /// Brand logo alias (`"brand"`, `"small"`) or `"none"`.
    pub logo: String,
    /// Page background fill (CSS color); empty = white.
    #[serde(default)]
    pub page_fill: String,
    #[serde(default)]
    pub cards: bool,
    /// `"all"`, `"workshops"`, or `"premium"`.
    pub panel_filter: String,
    /// Per-role brand font references (each names a `brand.fonts` role or "").
    #[serde(default)]
    pub fonts: SchedulePrintFonts,
    /// Per-role point sizes (e.g. `"9pt"`); empty = widget default.
    #[serde(default)]
    pub font_sizes: SchedulePrintFontSizes,
}

/// Brand font-role references for a print format (each value is a role key into
/// `brand.fonts`, e.g. `"heading"`, or empty for the widget default font).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SchedulePrintFonts {
    #[serde(default)]
    pub heading: String,
    #[serde(default)]
    pub banner: String,
    #[serde(default)]
    pub subheading: String,
    #[serde(default)]
    pub body: String,
}

/// Point-size overrides for a print format (empty = widget default).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SchedulePrintFontSizes {
    #[serde(default)]
    pub base: String,
    #[serde(default)]
    pub grid: String,
    #[serde(default)]
    pub banner: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_version() {
        let config = ScheduleConfig {
            version: 1,
            ..Default::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: ScheduleConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.version, 1);
    }

    #[test]
    fn test_config_from_json() {
        let json = r#"{"version":1,"brand":null,"printFormats":[]}"#;
        let config = ScheduleConfig::from_json(json).unwrap();
        assert_eq!(config.version, 1);
        assert!(config.brand.is_none());
        assert!(config.print_formats.is_empty());
    }
}
