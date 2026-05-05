/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! HTML widget embedding for cosam-convert.
//!
//! Produces self-contained HTML files (embed or full test page) by inlining
//! the widget CSS, JS, and schedule JSON. Assets are compiled-in by default;
//! callers can override via `--widget-css`, `--widget-js`, `--test-template`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

const BUILTIN_CSS: &str = include_str!("../../../widget/cosam-calendar.css");
const BUILTIN_JS: &str = include_str!("../../../widget/cosam-calendar.js");
const BUILTIN_TEMPLATE: &str = include_str!("../../../widget/square-template.html");

const COPYRIGHT_COMMENT: &str = "\
<!-- CosAm Calendar Widget | Copyright (c) 2026 Daphne Pfister | BSD-2-Clause | \
https://github.com/daphnediane/cosam-sched -->";

// ── WidgetSources ─────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct WidgetSources {
    pub css: String,
    pub js: String,
    pub template: String,
}

impl Default for WidgetSources {
    fn default() -> Self {
        Self {
            css: BUILTIN_CSS.to_string(),
            js: BUILTIN_JS.to_string(),
            template: BUILTIN_TEMPLATE.to_string(),
        }
    }
}

fn resolve_asset_path(value: &str, extension: &str, default_basename: &str) -> Result<PathBuf> {
    let path = Path::new(value);

    if path.is_file() {
        return Ok(path.to_path_buf());
    }

    let with_ext = PathBuf::from(format!("{value}.{extension}"));
    if with_ext.is_file() {
        return Ok(with_ext);
    }

    if path.is_dir() {
        let in_dir = path.join(format!("{default_basename}.{extension}"));
        if in_dir.is_file() {
            return Ok(in_dir);
        }
        anyhow::bail!(
            "Directory '{}' does not contain {}.{}",
            value,
            default_basename,
            extension
        );
    }

    anyhow::bail!("Cannot find {extension} file for '{value}'");
}

impl WidgetSources {
    /// Build sources, overriding builtins with any caller-specified paths.
    pub fn resolve(
        widget_css: Option<&str>,
        widget_js: Option<&str>,
        test_template: Option<&str>,
    ) -> Result<Self> {
        let mut sources = Self::default();

        if let Some(css_val) = widget_css {
            let css_path = resolve_asset_path(css_val, "css", "cosam-calendar")?;
            sources.css = std::fs::read_to_string(&css_path)
                .with_context(|| format!("Failed to read CSS: {}", css_path.display()))?;
        }

        if let Some(js_val) = widget_js {
            let js_path = resolve_asset_path(js_val, "js", "cosam-calendar")?;
            sources.js = std::fs::read_to_string(&js_path)
                .with_context(|| format!("Failed to read JS: {}", js_path.display()))?;
        }

        if let Some(tmpl_val) = test_template {
            let tmpl_path = Path::new(tmpl_val);
            sources.template = std::fs::read_to_string(tmpl_path)
                .with_context(|| format!("Failed to read template: {}", tmpl_path.display()))?;
        }

        Ok(sources)
    }
}

// ── HTML generation ───────────────────────────────────────────────────────────

/// Generate embeddable HTML snippet (no outer `<html>` or `<body>`).
///
/// All CSS, JS, and schedule JSON are inlined. The result can be pasted
/// into a Squarespace Code Block or any page that supports raw HTML.
pub fn generate_embed_html(
    json_data: &str,
    sources: &WidgetSources,
    minified: bool,
    style_page: Option<bool>,
) -> Result<String> {
    let style_page_line = match style_page {
        Some(true) => "\n            stylePageBody: true,",
        Some(false) => "\n            stylePageBody: false,",
        None => "",
    };
    let raw = format!(
        r#"{COPYRIGHT_COMMENT}
<style>
{css}
</style>
<div id="cosam-calendar-root"></div>
<script>
// CosAm Calendar Widget - Embeddable Version
// Copyright (c) 2026 Daphne Pfister
// SPDX-License-Identifier: BSD-2-Clause
// Project: https://github.com/daphnediane/cosam-sched

// Schedule data
window.cosamScheduleData = {json_data};

// Widget code
{js}

// Initialize widget
(function() {{
    if (typeof CosAmCalendar !== 'undefined' && window.cosamScheduleData) {{
        CosAmCalendar.init({{
            el: document.getElementById('cosam-calendar-root'),
            data: window.cosamScheduleData,{style_page_line}
        }});
    }}
}})();
</script>"#,
        css = sources.css,
        js = sources.js,
    );

    if minified {
        minify_html_content(&raw)
    } else {
        Ok(raw)
    }
}

/// Generate a standalone test page with the widget embedded in the template.
///
/// The template must contain `{WIDGET_BLOCK}` and `{TITLE}` placeholders.
pub fn generate_test_html(
    json_data: &str,
    title: &str,
    sources: &WidgetSources,
    minified: bool,
    style_page: Option<bool>,
) -> Result<String> {
    // Generate the embed block unminified; the outer minification covers it.
    let embed_block = generate_embed_html(json_data, sources, false, style_page)?;

    let raw = sources
        .template
        .replace("{WIDGET_BLOCK}", &embed_block)
        .replace("{TITLE}", title);

    if minified {
        minify_html_content(&raw)
    } else {
        Ok(raw)
    }
}

// ── File writers ──────────────────────────────────────────────────────────────

pub fn write_embed_html(
    path: &Path,
    json_data: &str,
    sources: &WidgetSources,
    minified: bool,
    style_page: Option<bool>,
) -> Result<()> {
    let html = generate_embed_html(json_data, sources, minified, style_page)?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }
    }
    std::fs::write(path, html.as_bytes())
        .with_context(|| format!("Failed to write embed HTML: {}", path.display()))?;
    eprintln!("Written: {} ({})", path.display(), format_size(html.len()));
    Ok(())
}

pub fn write_test_html(
    path: &Path,
    json_data: &str,
    title: &str,
    sources: &WidgetSources,
    minified: bool,
    style_page: Option<bool>,
) -> Result<()> {
    let html = generate_test_html(json_data, title, sources, minified, style_page)?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }
    }
    std::fs::write(path, html.as_bytes())
        .with_context(|| format!("Failed to write test HTML: {}", path.display()))?;
    eprintln!("Written: {} ({})", path.display(), format_size(html.len()));
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn minify_html_content(html: &str) -> Result<String> {
    let cfg = minify_html::Cfg {
        minify_css: true,
        minify_js: true,
        ..Default::default()
    };
    let minified = minify_html::minify(html.as_bytes(), &cfg);
    String::from_utf8(minified).context("Minified HTML contained invalid UTF-8")
}

fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
