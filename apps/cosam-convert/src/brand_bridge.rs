/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Branding bridge: convert the layout [`BrandConfig`] (colors, logos, fonts
//! from `config/brand.toml`) into the [`WidgetBrand`] payload carried to the
//! widget so its print formats can match the printed/PDF house style.
//!
//! This module is only compiled with the `layout` feature, since `BrandConfig`
//! lives in the optional `schedule-layout` crate. When the feature is off, the
//! embed path simply leaves [`WidgetExport::brand`] as `None`.

use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use schedule_core::widget_json::{
    ScheduleBrand, ScheduleBrandColors, ScheduleBrandMeta, SchedulePrintFont,
};
use schedule_layout::brand::BrandConfig;

/// Default brand config path, matching the layout export.
const DEFAULT_BRAND_PATH: &str = "config/brand.toml";

/// Build a [`ScheduleBrand`] from the configured brand file.
///
/// `brand_config` is the explicit `--brand-config` path if any; otherwise the
/// default `config/brand.toml` is tried. Returns `None` (no branding embedded)
/// when the file is missing or unparseable — branding is purely additive.
#[must_use]
pub fn load_widget_brand(brand_config: Option<&Path>) -> Option<ScheduleBrand> {
    let brand_path = brand_config
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_BRAND_PATH));

    let brand = match BrandConfig::load(&brand_path) {
        Ok(b) => b,
        Err(e) => {
            // Only warn when the user explicitly asked for a brand file; the
            // default path being absent is a normal "no branding" case.
            if brand_config.is_some() {
                eprintln!("warning: brand config {brand_path:?}: {e}; widget branding omitted");
            }
            return None;
        }
    };

    Some(widget_brand_from_config(&brand))
}

/// Convert a loaded [`BrandConfig`] into a [`ScheduleBrand`].
fn widget_brand_from_config(brand: &BrandConfig) -> ScheduleBrand {
    let colors = ScheduleBrandColors {
        primary: brand.colors.primary.clone(),
        black: brand.colors.black.clone(),
        dark_grey: brand.colors.dark_grey.clone(),
        white: brand.colors.white.clone(),
    };

    // Each logo alias becomes an `<img src>`-ready URL: passthrough for
    // http(s) URLs (e.g. a Squarespace-hosted asset), else a base64 data URL.
    let mut logos = std::collections::HashMap::new();
    for (name, value) in &brand.logos.aliases {
        if let Some(url) = logo_url(brand, name, value) {
            logos.insert(name.clone(), url);
        }
    }

    let fonts = brand
        .fonts
        .web_font_specs()
        .into_iter()
        .map(|s| SchedulePrintFont {
            role: s.role,
            family: s.family,
            weight: s.weight,
            style: s.style,
            google_url: s.google_url,
        })
        .collect();

    let meta = ScheduleBrandMeta {
        name: brand.meta.name.clone(),
        site_url: brand.meta.site_url.clone(),
    };

    ScheduleBrand {
        colors,
        logos,
        fonts,
        meta,
    }
}

/// Resolve a single logo alias to a URL usable in `<img src>`.
///
/// A `value` that is already an `http(s)` URL is passed through verbatim.
/// Otherwise the alias is resolved to a file (via `BrandLogos`) and embedded as
/// a base64 `data:` URL with a MIME type inferred from the file extension.
fn logo_url(brand: &BrandConfig, name: &str, value: &str) -> Option<String> {
    if is_http_url(value) {
        return Some(value.to_string());
    }

    let path = brand.logos.resolve_logo(name)?;
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("warning: reading logo {path:?}: {e}; skipped");
            return None;
        }
    };
    let mime = logo_mime(&path);
    Some(format!("data:{mime};base64,{}", STANDARD.encode(&bytes)))
}

fn is_http_url(value: &str) -> bool {
    let v = value.trim_start();
    v.starts_with("http://") || v.starts_with("https://")
}

/// Infer an image MIME type from a logo file's extension.
fn logo_mime(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_http_url() {
        assert!(is_http_url("https://example.com/a.png"));
        assert!(is_http_url("http://example.com/a.png"));
        assert!(!is_http_url("logo.svg"));
        assert!(!is_http_url("brand/logo/logo.png"));
    }

    #[test]
    fn test_logo_mime() {
        assert_eq!(logo_mime(Path::new("a.svg")), "image/svg+xml");
        assert_eq!(logo_mime(Path::new("a.PNG")), "image/png");
        assert_eq!(logo_mime(Path::new("a.jpeg")), "image/jpeg");
        assert_eq!(
            logo_mime(Path::new("a.unknown")),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_widget_brand_from_config_defaults() {
        let brand = BrandConfig::default();
        let wb = widget_brand_from_config(&brand);
        assert_eq!(wb.colors.primary, "#00BCDD");
        assert!(wb.logos.is_empty());
        assert!(wb.fonts.is_empty());
    }

    #[test]
    fn test_widget_brand_passthrough_url_logo() {
        let mut brand = BrandConfig::default();
        brand.logos.aliases.insert(
            "brand".to_string(),
            "https://static1.squarespace.com/logo.png".to_string(),
        );
        let wb = widget_brand_from_config(&brand);
        assert_eq!(
            wb.logos.get("brand").map(String::as_str),
            Some("https://static1.squarespace.com/logo.png")
        );
    }

    #[test]
    fn test_widget_brand_web_fonts() {
        let mut brand = BrandConfig::default();
        brand.fonts.heading_web = Some("Montserrat".to_string());
        brand.fonts.heading_web_weight = Some("600".to_string());
        let wb = widget_brand_from_config(&brand);
        // heading + banner (banner falls back to heading web font).
        let heading = wb.fonts.iter().find(|f| f.role == "heading").unwrap();
        assert_eq!(heading.family, "Montserrat");
        assert_eq!(heading.weight.as_deref(), Some("600"));
        assert!(wb.fonts.iter().any(|f| f.role == "banner"));
    }
}
