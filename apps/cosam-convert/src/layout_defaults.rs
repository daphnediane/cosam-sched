/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Layout defaults configuration for per-paper and per-format settings.
//!
//! Loaded from `config/layout.toml` (optional). Allows customizing default
//! font sizes and other layout parameters without modifying code.
//!
//! # Presets and Imports
//!
//! Named presets can be defined using `[presets.name]` sections:
//!
//! ```toml
//! [presets.workshop_base]
//! content = "description_only"
//! split = "none"
//! panel_filter = "workshops"
//! cards = true
//!
//! [presets.large_print]
//! paper = "tabloid"
//! base_font_pt = "14pt"
//!
//! [[jobs]]
//! import = ["workshop_base", "large_print"]
//! stem = "workshops-large"
//! orientation = "portrait"
//! ```
//!
//! Jobs and presets can import one or more presets using the `import` field.
//! Later imports override earlier ones, and job-specific settings override
//! all imported values.
//!
//! # Custom Brand Configuration
//!
//! Individual jobs can specify a different brand.toml file:
//!
//! ```toml
//! [[jobs]]
//! stem = "special-edition"
//! brand_config = "config/brand-special.toml"
//! paper = "tabloid"
//! ```

use std::collections::HashMap;
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
    #[error("Preset cycle detected: {0}")]
    PresetCycle(String),
    #[error("Unknown preset: {0}")]
    UnknownPreset(String),
}

/// Layout defaults configuration loaded from `config/layout.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutDefaults {
    /// Layout jobs to generate. If empty, uses hardcoded defaults.
    pub jobs: Vec<JobConfig>,
    /// Named presets that can be imported by jobs or other presets.
    pub presets: HashMap<String, JobConfig>,
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
    /// Page background color: hex (`"#f2f2f2"`), `"luma(95%)"`, or a named color.
    pub page_fill: Option<String>,
    /// Fill for empty grid cells (keeps them from blending into a tinted page).
    pub empty_grid_fill: Option<String>,
    /// Render description panels as bordered cards instead of the left-bar style.
    pub cards: Option<bool>,
    /// Card background color when `cards` is set (defaults to white).
    pub card_fill: Option<String>,
    /// Override the gutter between body-text columns (e.g. `"0.25in"`).
    pub column_gap: Option<String>,
    /// Gap between cards (with `cards`); `"column"` means "match the column gutter".
    pub card_gap: Option<String>,
    /// Presets to import and merge into this job. Later imports override earlier ones.
    pub import: Vec<String>,
    /// Custom brand.toml path for this job. If not set, uses the global brand config.
    pub brand_config: Option<String>,
}

impl JobConfig {
    /// Merge another JobConfig into this one.
    /// The other config's values only override if they are explicitly set (not empty/default).
    fn merge_from(&mut self, other: &JobConfig) {
        if !other.paper.is_empty() {
            self.paper = other.paper.clone();
        }
        if other.content.is_some() {
            self.content = other.content.clone();
        }
        if !other.split.is_empty() {
            self.split = other.split.clone();
        }
        if !other.orientation.is_empty() {
            self.orientation = other.orientation.clone();
        }
        if !other.stem.is_empty() {
            self.stem = other.stem.clone();
        }
        if other.panel_filter.is_some() {
            self.panel_filter = other.panel_filter.clone();
        }
        if other.color_mode.is_some() {
            self.color_mode = other.color_mode.clone();
        }
        if other.footer.is_some() {
            self.footer = other.footer.clone();
        }
        if other.double_sided.is_some() {
            self.double_sided = other.double_sided;
        }
        if other.header_text.is_some() {
            self.header_text = other.header_text.clone();
        }
        if other.columns.is_some() {
            self.columns = other.columns;
        }
        if other.base_font_pt.is_some() {
            self.base_font_pt = other.base_font_pt.clone();
        }
        if other.grid_font_pt.is_some() {
            self.grid_font_pt = other.grid_font_pt.clone();
        }
        if other.page_fill.is_some() {
            self.page_fill = other.page_fill.clone();
        }
        if other.empty_grid_fill.is_some() {
            self.empty_grid_fill = other.empty_grid_fill.clone();
        }
        if other.cards.is_some() {
            self.cards = other.cards;
        }
        if other.card_fill.is_some() {
            self.card_fill = other.card_fill.clone();
        }
        if other.column_gap.is_some() {
            self.column_gap = other.column_gap.clone();
        }
        if other.card_gap.is_some() {
            self.card_gap = other.card_gap.clone();
        }
        if other.brand_config.is_some() {
            self.brand_config = other.brand_config.clone();
        }
    }

    /// Resolve this job configuration by applying all imported presets.
    pub fn resolve(
        &self,
        presets: &HashMap<String, JobConfig>,
        resolution_stack: &mut Vec<String>,
    ) -> Result<JobConfig, LayoutDefaultsError> {
        let mut resolved = JobConfig::default();

        // Apply imported presets in order
        for preset_name in &self.import {
            if resolution_stack.contains(preset_name) {
                return Err(LayoutDefaultsError::PresetCycle(format!(
                    "Cycle detected: {} -> {}",
                    resolution_stack.join(" -> "),
                    preset_name
                )));
            }

            let preset = presets
                .get(preset_name)
                .ok_or_else(|| LayoutDefaultsError::UnknownPreset(preset_name.clone()))?;

            resolution_stack.push(preset_name.clone());
            let resolved_preset = preset.resolve(presets, resolution_stack)?;
            resolution_stack.pop();

            resolved.merge_from(&resolved_preset);
        }

        // Finally, apply the job's own settings (which override everything)
        resolved.merge_from(self);

        Ok(resolved)
    }
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

    /// Resolve all jobs, applying preset imports.
    /// Returns a Vec of (resolved_job, maybe_brand_config_path).
    pub fn resolve_jobs(&self) -> Result<Vec<(JobConfig, Option<String>)>, LayoutDefaultsError> {
        let mut resolved = Vec::new();

        for job in &self.jobs {
            let mut resolution_stack = Vec::new();
            let resolved_job = job.resolve(&self.presets, &mut resolution_stack)?;
            let brand_path = resolved_job.brand_config.clone();
            resolved.push((resolved_job, brand_path));
        }

        Ok(resolved)
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

    #[test]
    fn test_preset_parsing() {
        let toml = r#"
[presets.workshop_base]
content = "description_only"
split = "none"
panel_filter = "workshops"
cards = true
paper = "letter"
orientation = "portrait"
stem = "workshop"

[[jobs]]
import = ["workshop_base"]
stem = "my-workshops"
paper = "tabloid"
"#;
        let defaults = LayoutDefaults::from_str(toml).unwrap();
        assert_eq!(defaults.presets.len(), 1);
        assert!(defaults.presets.contains_key("workshop_base"));
        assert_eq!(defaults.jobs[0].import, vec!["workshop_base"]);
    }

    #[test]
    fn test_preset_resolution() {
        let toml = r#"
[presets.base]
paper = "letter"
orientation = "portrait"
stem = "base"
split = "day"
content = "both"

[presets.large]
paper = "tabloid"
base_font_pt = "14pt"

[[jobs]]
import = ["base", "large"]
stem = "final"
orientation = "landscape"
"#;
        let defaults = LayoutDefaults::from_str(toml).unwrap();
        let resolved = defaults.resolve_jobs().unwrap();

        assert_eq!(resolved.len(), 1);
        let (job, _) = &resolved[0];
        // From base: split, content
        assert_eq!(job.split, "day");
        assert_eq!(job.content, Some("both".to_string()));
        // From large (overrides base): paper, base_font_pt
        assert_eq!(job.paper, "tabloid");
        assert_eq!(job.base_font_pt, Some("14pt".to_string()));
        // From job itself (overrides all): stem, orientation
        assert_eq!(job.stem, "final");
        assert_eq!(job.orientation, "landscape");
    }

    #[test]
    fn test_preset_cycle_detection() {
        let toml = r#"
[presets.a]
import = ["b"]
paper = "letter"

[presets.b]
import = ["a"]
paper = "tabloid"

[[jobs]]
import = ["a"]
stem = "test"
"#;
        let defaults = LayoutDefaults::from_str(toml).unwrap();
        let result = defaults.resolve_jobs();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cycle detected"));
    }

    #[test]
    fn test_unknown_preset() {
        let toml = r#"
[[jobs]]
import = ["nonexistent"]
stem = "test"
"#;
        let defaults = LayoutDefaults::from_str(toml).unwrap();
        let result = defaults.resolve_jobs();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown preset"));
    }

    #[test]
    fn test_brand_config_in_job() {
        let toml = r#"
[[jobs]]
stem = "special"
paper = "tabloid"
brand_config = "config/brand-special.toml"
"#;
        let defaults = LayoutDefaults::from_str(toml).unwrap();
        let resolved = defaults.resolve_jobs().unwrap();
        assert_eq!(
            resolved[0].0.brand_config,
            Some("config/brand-special.toml".to_string())
        );
    }

    #[test]
    fn test_preset_with_brand_config() {
        let toml = r#"
[presets.special_brand]
brand_config = "config/brand-special.toml"
cards = true

[[jobs]]
import = ["special_brand"]
stem = "my-job"
paper = "letter"
"#;
        let defaults = LayoutDefaults::from_str(toml).unwrap();
        let resolved = defaults.resolve_jobs().unwrap();
        let (job, brand) = &resolved[0];
        assert_eq!(*brand, Some("config/brand-special.toml".to_string()));
        assert_eq!(
            job.brand_config,
            Some("config/brand-special.toml".to_string())
        );
        assert_eq!(job.cards, Some(true));
    }
}
