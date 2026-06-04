/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Brand configuration: colors, fonts, logos, and site metadata.
//!
//! Load from `config/brand.toml` (gitignored); use `config/brand.sample.toml`
//! as the committed template.
//!
//! ## Logo lookup
//!
//! Logos are configured in the `[logos]` section.  `logo_dir` sets the
//! directory (default: auto-detect `brand/logos` then `brand/logo` relative to
//! the config file).  Named aliases map short names to filenames within that
//! directory:
//!
//! ```toml
//! [logos]
//! logo_dir = "brand/logo"
//! brand    = "COSLogoAltWhite2026.svg"
//! small    = "COSLogoAltWhite.svg"
//! ```
//!
//! A layout job may then set `logo = "small"` to use the alias, or
//! `logo = "COSLogoAltBlue.svg"` to use a bare filename directly.
//! `logo = "none"` (or omitting the field) suppresses the logo entirely.
//!
//! Resolution order for a name `N`:
//! 1. Named alias in `[logos]` → `logo_dir / filename`
//! 2. Bare filename → `logo_dir / N`
//! 3. Neither found → `None` + warning printed to stderr.

use std::collections::HashMap;
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
    #[serde(default)]
    pub colors: BrandColors,
    #[serde(default)]
    pub fonts: BrandFonts,
    #[serde(default)]
    pub logos: BrandLogos,
    #[serde(default)]
    pub meta: BrandMeta,
}

impl BrandConfig {
    /// Load from a TOML file. Missing keys fall back to defaults.
    pub fn load(path: &Path) -> Result<Self, BrandError> {
        let text = std::fs::read_to_string(path)?;
        let mut config: BrandConfig = toml::from_str(&text)?;
        // Resolve relative paths against the config file's directory.
        // Canonicalize first so that relative config paths (e.g. `config/brand.toml`)
        // produce absolute asset paths in the output.
        let abs_path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        if let Some(dir) = abs_path.parent() {
            config.logos.resolve_paths(dir);
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

// ---------------------------------------------------------------------------
// Logo configuration
// ---------------------------------------------------------------------------

/// Logo registry: a directory of logo files with optional named aliases.
///
/// Deserialized from the `[logos]` section of `brand.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BrandLogos {
    /// Directory containing logo files. Relative paths are resolved against
    /// the brand.toml file's directory at load time. When absent, the loader
    /// auto-detects `brand/logos` then `brand/logo` relative to brand.toml.
    pub logo_dir: Option<PathBuf>,

    /// Named logo aliases, e.g. `brand = "COSLogoAltWhite2026.svg"`.
    /// Any key not listed here can still be used as a bare filename.
    #[serde(flatten)]
    pub aliases: HashMap<String, String>,
}

/// Candidate subdirectory names to probe when `logo_dir` is not explicit.
const LOGO_DIR_CANDIDATES: &[&str] = &["brand/logos", "brand/logo"];

impl BrandLogos {
    /// Resolve `logo_dir` and alias paths to absolute paths.
    ///
    /// Called by [`BrandConfig::load`] after deserialization.
    pub(crate) fn resolve_paths(&mut self, config_dir: &Path) {
        if let Some(dir) = &self.logo_dir {
            if !dir.is_absolute() {
                self.logo_dir = Some(config_dir.join(dir));
            }
        } else {
            // Auto-detect: probe candidates relative to the config file dir.
            for candidate in LOGO_DIR_CANDIDATES {
                let p = config_dir.join(candidate);
                if p.exists() {
                    self.logo_dir = Some(p);
                    break;
                }
            }
        }
    }

    /// Resolve a logo name to an absolute path, or `None` if unresolvable.
    ///
    /// `name` is checked against the named aliases first, then used as a
    /// bare filename within `logo_dir`.  A warning is printed to stderr when
    /// the name cannot be resolved.
    ///
    /// Returns `None` when `logo_dir` is unset or the file does not exist.
    pub fn resolve_logo(&self, name: &str) -> Option<PathBuf> {
        let dir = match &self.logo_dir {
            Some(d) => d,
            None => {
                eprintln!(
                    "warning: logo \"{name}\" requested but no logo_dir is configured \
                     (add [logos] logo_dir to brand.toml)"
                );
                return None;
            }
        };

        // 1. Named alias → filename in logo_dir.
        // 2. Bare filename → directly in logo_dir.
        let filename = self.aliases.get(name).map(String::as_str).unwrap_or(name);
        let path = dir.join(filename);

        if path.exists() {
            Some(path)
        } else {
            eprintln!("warning: logo \"{name}\" not found at {}", path.display());
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Colors
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Fonts
// ---------------------------------------------------------------------------

/// Font configuration. Paths are optional; if absent, Typst uses system fonts.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct BrandFonts {
    /// Heading font family name (e.g. `"Trend Sans"`).
    pub heading: Option<String>,
    /// Heading font style (e.g. `"normal"`, `"italic"`, `"oblique"`). If None, uses family default.
    pub heading_style: Option<String>,
    /// Heading font weight (e.g., `"regular"`, `"bold"`, or numeric like `"500"`).
    /// For Trend Sans: "200"=One, "300"=Two, "400"=Three, "500"=Four, "700"=Five
    pub heading_weight: Option<String>,
    /// Banner font family (page-header bars). Falls back to `heading`.
    pub banner: Option<String>,
    /// Banner font style. Falls back to `heading_style`.
    pub banner_style: Option<String>,
    /// Banner font weight. Falls back to `heading_weight`, then `"bold"`.
    pub banner_weight: Option<String>,
    /// Subheading font family name (e.g. `"Bebas Neue"`).
    pub subheading: Option<String>,
    /// Body font family name (e.g. `"Avenir Next"`).
    pub body: Option<String>,
    /// Body font style (e.g. `"normal"`, `"italic"`, `"oblique"`). If None, uses family default.
    pub body_style: Option<String>,
    /// Body font weight (e.g., `"regular"`, `"bold"`, or numeric like `"400"`).
    pub body_weight: Option<String>,
    /// Path to a directory containing font files (TTF/OTF).
    pub font_dir: Option<PathBuf>,
}

impl BrandFonts {
    pub(crate) fn resolve_paths(&mut self, base: &Path) {
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

    /// Heading font style, if specified.
    pub fn heading_style(&self) -> Option<&str> {
        self.heading_style.as_deref()
    }

    /// Heading font weight, if specified.
    pub fn heading_weight(&self) -> Option<&str> {
        self.heading_weight.as_deref()
    }

    /// Banner font family, falling back to `heading_or_default()`.
    pub fn banner_or_default(&self) -> &str {
        self.banner
            .as_deref()
            .unwrap_or_else(|| self.heading_or_default())
    }

    /// Banner font style, falling back to `heading_style`.
    pub fn banner_style(&self) -> Option<&str> {
        self.banner_style
            .as_deref()
            .or(self.heading_style.as_deref())
    }

    /// Banner font weight, falling back to `heading_weight`, then `"bold"`.
    pub fn banner_weight_or_default(&self) -> &str {
        self.banner_weight
            .as_deref()
            .or(self.heading_weight.as_deref())
            .unwrap_or("bold")
    }

    /// Subheading font, falling back to `"Liberation Sans"`.
    pub fn subheading_or_default(&self) -> &str {
        self.subheading.as_deref().unwrap_or("Liberation Sans")
    }

    /// Body font, falling back to `"Liberation Sans"`.
    pub fn body_or_default(&self) -> &str {
        self.body.as_deref().unwrap_or("Liberation Sans")
    }

    /// Body font style, if specified.
    pub fn body_style(&self) -> Option<&str> {
        self.body_style.as_deref()
    }

    /// Body font weight, if specified.
    pub fn body_weight(&self) -> Option<&str> {
        self.body_weight.as_deref()
    }
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

/// Brand metadata: organization name and site URL.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct BrandMeta {
    /// Organization name (e.g. `"Cosplay America"`).
    pub name: Option<String>,
    /// Public site URL (e.g. `"https://cosplayamerica.com"`).
    pub site_url: Option<String>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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

    #[test]
    fn test_logos_resolve_missing_dir() {
        // No logo_dir set and no auto-detect path available → None + warning.
        let logos = BrandLogos::default();
        assert!(logos.resolve_logo("brand").is_none());
    }

    #[test]
    fn test_logos_alias_resolution() {
        use std::collections::HashMap;
        let mut aliases = HashMap::new();
        aliases.insert("brand".to_string(), "logo.svg".to_string());
        let logos = BrandLogos {
            // Use a tempdir so the file "exists" check can pass.
            logo_dir: Some(std::env::temp_dir()),
            aliases,
        };
        // The alias resolves to temp_dir/logo.svg; file won't exist so None + warning.
        // Just confirm we don't panic and the path composition is attempted.
        let _ = logos.resolve_logo("brand");
    }

    #[test]
    fn test_logos_parse_from_toml() {
        let toml = r#"
[logos]
brand = "logo-white.svg"
small = "logo-small.svg"
"#;
        let config: BrandConfig = toml::from_str(toml).unwrap();
        assert_eq!(
            config.logos.aliases.get("brand").map(String::as_str),
            Some("logo-white.svg")
        );
        assert_eq!(
            config.logos.aliases.get("small").map(String::as_str),
            Some("logo-small.svg")
        );
        assert!(config.logos.logo_dir.is_none());
    }
}
