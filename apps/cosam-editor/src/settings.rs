/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportSettings {
    pub minified: bool,
    pub widget_css_path: Option<PathBuf>,
    pub widget_js_path: Option<PathBuf>,
    pub test_template_path: Option<PathBuf>,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            minified: true,
            widget_css_path: None,
            widget_js_path: None,
            test_template_path: None,
        }
    }
}

pub struct SettingsManager;

impl SettingsManager {
    pub fn config_dir() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not find config directory")?
            .join("cosam-editor");

        std::fs::create_dir_all(&config_dir).with_context(|| {
            format!(
                "Failed to create config directory: {}",
                config_dir.display()
            )
        })?;

        Ok(config_dir)
    }

    pub fn settings_file() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("settings.json"))
    }

    pub fn load_settings() -> Result<ExportSettings> {
        let settings_file = Self::settings_file()?;

        if settings_file.exists() {
            let content = std::fs::read_to_string(&settings_file).with_context(|| {
                format!("Failed to read settings file: {}", settings_file.display())
            })?;

            serde_json::from_str(&content).with_context(|| {
                format!("Failed to parse settings file: {}", settings_file.display())
            })
        } else {
            Ok(ExportSettings::default())
        }
    }

    pub fn save_settings(settings: &ExportSettings) -> Result<()> {
        let settings_file = Self::settings_file()?;

        let content =
            serde_json::to_string_pretty(settings).context("Failed to serialize settings")?;

        std::fs::write(&settings_file, content).with_context(|| {
            format!("Failed to write settings file: {}", settings_file.display())
        })?;

        Ok(())
    }

    pub fn set_minified(minified: bool) -> Result<()> {
        let mut settings = Self::load_settings()?;
        settings.minified = minified;
        Self::save_settings(&settings)
    }

    pub fn set_widget_css_path(path: Option<PathBuf>) -> Result<()> {
        let mut settings = Self::load_settings()?;
        settings.widget_css_path = path;
        Self::save_settings(&settings)
    }

    pub fn set_widget_js_path(path: Option<PathBuf>) -> Result<()> {
        let mut settings = Self::load_settings()?;
        settings.widget_js_path = path;
        Self::save_settings(&settings)
    }

    pub fn set_test_template_path(path: Option<PathBuf>) -> Result<()> {
        let mut settings = Self::load_settings()?;
        settings.test_template_path = path;
        Self::save_settings(&settings)
    }
}
