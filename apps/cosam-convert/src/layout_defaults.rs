/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Layout defaults configuration for per-paper and per-format settings.
//!
//! Loaded from `config/layout.toml` (optional). Allows customizing default
//! font sizes and other layout parameters without modifying code.

use std::path::Path;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LayoutDefaultsError {
    #[error("I/O error reading layout defaults: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
}

/// Layout defaults configuration loaded from `config/layout.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutDefaults {
    /// Layout jobs to generate. If empty, uses hardcoded defaults.
    pub jobs: Vec<JobConfig>,
}

/// Layout job configuration for generating a specific output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct JobConfig {
    /// Paper size: "letter", "legal", "tabloid", "super_b", "poster", "postcard"
    pub paper: String,
    /// Content: "both" (default), "grid_only", "description_only", "panel_list"
    pub content: Option<String>,
    /// How to split: "none", "day", "half_day", "room", "room_day", "presenter", "presenter_day"
    #[serde(alias = "split_by")]
    pub split: String,
    /// Orientation: "portrait" or "landscape"
    pub orientation: String,
    /// File stem prefix (e.g., "schedule", "desc", "workshops")
    pub stem: String,
    /// Panel filter: "all" (default), "workshops", "premium". Optional.
    pub panel_filter: Option<String>,
    /// Color mode: "color" (default) or "bw". Optional.
    pub color_mode: Option<String>,
    /// Page footer: "full" (default), "timestamp_only", "none". Optional.
    pub footer: Option<String>,
    /// Insert blank pages so each section starts on an odd page. Optional.
    pub double_sided: Option<bool>,
    /// Header text (left for 1-D splits, right for "none"). Optional.
    pub header_text: Option<String>,
    /// Column-count override. If unset, the content/paper default is used.
    pub columns: Option<u32>,
    /// Optional font size override (e.g., "13.2pt"). Uses defaults if not specified.
    pub base_font_pt: Option<String>,
    /// Optional grid event text size override (e.g., "8pt"). If not set, uses base_font_pt.
    pub grid_font_pt: Option<String>,
}

impl LayoutDefaults {
    /// Load from a TOML file. Returns empty defaults if file doesn't exist.
    pub fn load(path: &Path) -> Result<Self, LayoutDefaultsError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)?;
        Self::from_str(&text)
    }

    /// Parse from a TOML string.
    pub fn from_str(text: &str) -> Result<Self, LayoutDefaultsError> {
        let defaults: LayoutDefaults = toml::from_str(text)?;
        Ok(defaults)
    }

    /// Return built-in default layout configuration.
    /// This is parsed from `config/layout-default.toml` at compile time and cached.
    pub fn default_layout() -> LayoutDefaults {
        static DEFAULT_LAYOUT: LazyLock<LayoutDefaults> = LazyLock::new(|| {
            let text = include_str!("../../../config/layout-default.toml");
            LayoutDefaults::from_str(text)
                .expect("embedded layout-default.toml should be valid TOML")
        });
        DEFAULT_LAYOUT.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_empty() {
        let defaults = LayoutDefaults::default();
        assert!(defaults.jobs.is_empty());
    }

    #[test]
    fn test_default_layout_has_jobs() {
        let defaults = LayoutDefaults::default_layout();
        assert!(!defaults.jobs.is_empty());
    }

    #[test]
    fn test_parse_toml_jobs() {
        let toml = r#"
[[jobs]]
content = "grid_only"
paper = "tabloid"
split_by = "half_day"
orientation = "landscape"
stem = "schedule"
panel_filter = "workshops"
color_mode = "bw"
double_sided = true
columns = 5
base_font_pt = "10pt"
"#;
        let defaults = LayoutDefaults::from_str(toml).unwrap();
        assert_eq!(defaults.jobs.len(), 1);
        assert_eq!(defaults.jobs[0].content, Some("grid_only".to_string()));
        // `split_by` is accepted as an alias for `split`.
        assert_eq!(defaults.jobs[0].split, "half_day");
        assert_eq!(defaults.jobs[0].panel_filter, Some("workshops".to_string()));
        assert_eq!(defaults.jobs[0].color_mode, Some("bw".to_string()));
        assert_eq!(defaults.jobs[0].double_sided, Some(true));
        assert_eq!(defaults.jobs[0].columns, Some(5));
        assert_eq!(defaults.jobs[0].base_font_pt, Some("10pt".to_string()));
    }
}
