/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Brand configuration: colors, fonts, logo path, and site URL.
//!
//! Load from `config/brand.toml` (gitignored); use `config/brand.sample.toml`
//! as the committed template.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BrandError {
    #[error("I/O error reading brand config: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
}

/// Brand configuration loaded from `config/brand.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BrandConfig {
    pub colors: BrandColors,
    pub fonts: BrandFonts,
    pub meta: BrandMeta,
}

impl BrandConfig {
    /// Load from a TOML file. Missing keys fall back to defaults.
    pub fn load(path: &Path) -> Result<Self, BrandError> {
        let text = std::fs::read_to_string(path)?;
        let mut config: BrandConfig = toml::from_str(&text)?;
        // Resolve relative paths against the config file's directory
        if let Some(dir) = path.parent() {
            if let Some(logo) = &config.meta.logo_path {
                if !logo.is_absolute() {
                    config.meta.logo_path = Some(dir.join(logo));
                }
            }
            config.fonts.resolve_paths(dir);
        }
        Ok(config)
    }

    /// Return the built-in defaults (matches `config/brand.sample.toml`).
    pub fn sample() -> Self {
        Self::default()
    }

    /// Serialize to TOML string (for `--dump-sample-brand`).
    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).unwrap_or_default()
    }
}

/// Brand color palette.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct BrandColors {
    /// Primary accent color (default: CosAm blue `#00BCDD`).
    pub primary: String,
    /// Black (default: `#000000`).
    pub black: String,
    /// Dark grey (default: `#18191C`).
    pub dark_grey: String,
    /// White (default: `#FFFFFF`).
    pub white: String,
}

impl Default for BrandColors {
    fn default() -> Self {
        Self {
            primary: "#00BCDD".to_string(),
            black: "#000000".to_string(),
            dark_grey: "#18191C".to_string(),
            white: "#FFFFFF".to_string(),
        }
    }
}

/// Font configuration. Paths are optional; if absent, Typst uses system fonts.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct BrandFonts {
    /// Heading font family name (e.g. `"Trend Sans One"`).
    pub heading: Option<String>,
    /// Subheading font family name (e.g. `"Bebas Neue"`).
    pub subheading: Option<String>,
    /// Body font family name (e.g. `"Avenir Next"`).
    pub body: Option<String>,
    /// Path to a directory containing font files (TTF/OTF).
    pub font_dir: Option<PathBuf>,
}

impl BrandFonts {
    fn resolve_paths(&mut self, base: &Path) {
        if let Some(dir) = &self.font_dir {
            if !dir.is_absolute() {
                self.font_dir = Some(base.join(dir));
            }
        }
    }

    /// Heading font, falling back to `"Liberation Sans"`.
    pub fn heading_or_default(&self) -> &str {
        self.heading.as_deref().unwrap_or("Liberation Sans")
    }

    /// Subheading font, falling back to `"Liberation Sans"`.
    pub fn subheading_or_default(&self) -> &str {
        self.subheading.as_deref().unwrap_or("Liberation Sans")
    }

    /// Body font, falling back to `"Liberation Sans"`.
    pub fn body_or_default(&self) -> &str {
        self.body.as_deref().unwrap_or("Liberation Sans")
    }
}

/// Brand metadata: organization name, site URL, optional logo.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct BrandMeta {
    /// Organization name (e.g. `"Cosplay America"`).
    pub name: Option<String>,
    /// Public site URL (e.g. `"https://cosplay-america.com"`).
    pub site_url: Option<String>,
    /// Path to logo image (SVG or PNG). Resolved relative to config file.
    pub logo_path: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brand_defaults() {
        let b = BrandConfig::default();
        assert_eq!(b.colors.primary, "#00BCDD");
        assert_eq!(b.colors.black, "#000000");
        assert_eq!(b.colors.dark_grey, "#18191C");
        assert_eq!(b.colors.white, "#FFFFFF");
    }

    #[test]
    fn test_brand_to_toml_roundtrip() {
        let b = BrandConfig::default();
        let toml_str = b.to_toml();
        let b2: BrandConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(b2.colors.primary, b.colors.primary);
    }

    #[test]
    fn test_sample_matches_defaults() {
        let sample = BrandConfig::sample();
        assert_eq!(sample.colors.primary, BrandColors::default().primary);
    }

    #[test]
    fn test_font_fallbacks() {
        let fonts = BrandFonts::default();
        assert_eq!(fonts.heading_or_default(), "Liberation Sans");
        assert_eq!(fonts.body_or_default(), "Liberation Sans");
    }
}
