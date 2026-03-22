/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

const BUILTIN_CSS: &str = include_str!("../../../../widget/cosam-calendar.css");
const BUILTIN_JS: &str = include_str!("../../../../widget/cosam-calendar.js");
const BUILTIN_TEMPLATE: &str = include_str!("../../../../widget/square-template.html");

const COPYRIGHT_COMMENT: &str = "\
<!-- CosAm Calendar Widget | Copyright (c) 2026 Daphne Pfister | BSD-2-Clause | https://github.com/daphnediane/cosam-sched -->";

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

pub fn generate_test_html(
    json_data: &str,
    title: &str,
    sources: &WidgetSources,
    minified: bool,
    style_page: Option<bool>,
) -> Result<String> {
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

pub fn generate_preview_html(
    json_data: &str,
    title: &str,
    sources: &WidgetSources,
    generation: u64,
) -> Result<String> {
    Ok(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta name="cosam-generation" content="{generation}">
  <title>{title} — Preview</title>
  <style>
    body {{
      margin: 0;
      padding: 20px;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      background: #f5f5f5;
    }}
    .page-header {{
      text-align: center;
      padding: 20px;
      margin-bottom: 20px;
    }}
    .page-header h1 {{
      margin: 0 0 8px 0;
      font-size: 28px;
      color: #1f2937;
    }}
    .page-header p {{
      margin: 0;
      color: #6b7280;
      font-size: 14px;
    }}
{css}
  </style>
</head>
<body>
  <div class="page-header">
    <h1>{title}</h1>
    <p>Editor Preview — auto-refreshes when data changes</p>
  </div>
  <div id="cosam-calendar"></div>
  <script>
{js}
  </script>
  <script>
    CosAmCalendar.init({{
      el: '#cosam-calendar',
      data: {json_data},
      watchForChanges: true
    }});
  </script>
</body>
</html>"#,
        css = sources.css,
        js = sources.js,
    ))
}

fn minify_html_content(html: &str) -> Result<String> {
    let cfg = minify_html::Cfg {
        minify_css: true,
        minify_js: true,
        ..Default::default()
    };

    let minified_bytes = minify_html::minify(html.as_bytes(), &cfg);
    String::from_utf8(minified_bytes).context("Minified HTML contained invalid UTF-8")
}

pub fn write_embed_html(
    path: &Path,
    json_data: &str,
    sources: &WidgetSources,
    minified: bool,
    style_page: Option<bool>,
) -> Result<()> {
    let html = generate_embed_html(json_data, sources, minified, style_page)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }
    std::fs::write(path, html.as_bytes())
        .with_context(|| format!("Failed to write embed HTML: {}", path.display()))?;

    let size = html.len();
    eprintln!("Embed HTML: {} ({})", path.display(), format_size(size));
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
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }
    std::fs::write(path, html.as_bytes())
        .with_context(|| format!("Failed to write test HTML: {}", path.display()))?;

    let size = html.len();
    eprintln!("Test HTML: {} ({})", path.display(), format_size(size));
    Ok(())
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
