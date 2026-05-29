/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! HTML widget embedding for cosam-convert.
//!
//! Produces self-contained HTML files (embed or full test page) by inlining
//! the widget CSS, JS, and schedule data. Two formats are supported:
//! - JSON format (default): schedule data is gzip+base64-encoded JSON. The
//!   inlined `load-json-embed.min.js` reads `#cosam-schedule-data` and hands
//!   it to `CosAmCalendar.JsonEmbedLoader`.
//! - Widget-html format (`--embed-as-html`): structural data is a compact JSON
//!   block; panels are semantic HTML outside `#cosam-calendar-root`. The
//!   inlined `load-html-embed.min.js` reads both and hands them to
//!   `CosAmCalendar.HtmlEmbedLoader`.
//!
//! Assets are compiled-in by default; callers can override via
//! `--widget-css`, `--widget-js`, `--test-template`.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use schedule_core::widget_json::WidgetExport;

use crate::static_html;

// Pre-minified by esbuild (via `npm run build` / build.rs).
// Using the esbuild output avoids minify-html's JS minifier, which
// double-escapes \uXXXX sequences in string literals.
const BUILTIN_CSS: &str = include_str!("../../../widget/cosam-calendar.min.css");
const BUILTIN_JS: &str = include_str!("../../../widget/cosam-calendar.min.js");
const BUILTIN_JSON_EMBED_LOADER: &str = include_str!("../../../widget/load-json-embed.min.js");
const BUILTIN_HTML_EMBED_LOADER: &str = include_str!("../../../widget/load-html-embed.min.js");
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

// ── Data compression (JSON format) ────────────────────────────────────────────

/// Compact JSON whitespace, gzip-compress, and base64-encode for embedding.
///
/// Returns the base64 string. The caller embeds it in a
/// `<script type="application/json" id="cosam-schedule-data">` tag so the
/// HTML/JS minifier never touches the data, and the browser decompresses it
/// with the native `DecompressionStream` API before handing it to the widget.
fn compress_and_encode(json_data: &str) -> Result<String> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use flate2::{write::GzEncoder, Compression};

    let value: serde_json::Value =
        serde_json::from_str(json_data).context("Failed to parse JSON for embedding")?;
    let compact = serde_json::to_string(&value).context("Failed to compact JSON")?;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder
        .write_all(compact.as_bytes())
        .context("Failed to compress schedule data")?;
    let compressed = encoder
        .finish()
        .context("Failed to finalize gzip compression")?;

    Ok(STANDARD.encode(compressed))
}

// ── HTML generation (JSON format) ─────────────────────────────────────────────

/// Generate embeddable HTML snippet using the gzip+base64 JSON format (default).
///
/// Schedule data is compressed and embedded in a `<script>` tag; the widget
/// decompresses it at runtime via `dataEl`. All CSS, JS, and data are inlined.
/// The result can be pasted into a Squarespace Code Block or any raw-HTML page.
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
    let encoded_data = compress_and_encode(json_data)?;
    let json_loader = BUILTIN_JSON_EMBED_LOADER;
    let raw = format!(
        r#"{COPYRIGHT_COMMENT}
<style>
{css}
</style>
<div id="cosam-calendar-root"><p style="padding:40px 20px;text-align:center">Schedule failed to load. Please enable JavaScript and reload the page.</p></div>
<script type="application/json" id="cosam-schedule-data">
{encoded_data}
</script>
<script>
// CosAm Calendar Widget - Embeddable Version
// Copyright (c) 2026 Daphne Pfister
// SPDX-License-Identifier: BSD-2-Clause
// Project: https://github.com/daphnediane/cosam-sched
// Includes: qrcode (MIT) https://github.com/soldair/node-qrcode

// Widget code
{js}

// JSON embed loader
{json_loader}

// Initialize widget
CosAmCalendar.init({{
    el: document.getElementById('cosam-calendar-root'),
    loader: CosAmCalendar.JsonEmbedLoader(),{style_page_line}
}});
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

/// Generate a standalone test page with the widget embedded in the template
/// using the gzip+base64 JSON format.
///
/// The template must contain `{WIDGET_BLOCK}` and `{TITLE}` placeholders.
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

// ── HTML generation (widget-html format) ──────────────────────────────────────

/// Generate embeddable HTML snippet using the widget-html format.
///
/// Schedule data is rendered as a compact JSON block (structural data) plus
/// semantic `<article>` elements (panels). Requires the widget-html JS parser
/// (Phase 4). All CSS, JS, and data are inlined.
pub fn generate_embed_html_widget_html(
    export: &WidgetExport,
    sources: &WidgetSources,
    minified: bool,
    style_page: Option<bool>,
) -> Result<String> {
    let style_page_line = match style_page {
        Some(true) => "\n    stylePageBody: true,",
        Some(false) => "\n    stylePageBody: false,",
        None => "",
    };
    let schedule_html = static_html::generate_static_schedule_html(export)?;
    let html_loader = BUILTIN_HTML_EMBED_LOADER;
    let raw = format!(
        r#"{COPYRIGHT_COMMENT}
<style>
{css}
</style>
<div id="cosam-calendar-root"></div>
{schedule_html}
<script>
// CosAm Calendar Widget - Embeddable Version
// Copyright (c) 2026 Daphne Pfister
// SPDX-License-Identifier: BSD-2-Clause
// Project: https://github.com/daphnediane/cosam-sched
// Includes: qrcode (MIT) https://github.com/soldair/node-qrcode

// Widget code
{js}

// HTML embed loader
{html_loader}

// Initialize widget
CosAmCalendar.init({{
    el: document.getElementById('cosam-calendar-root'),
    loader: CosAmCalendar.HtmlEmbedLoader(),{style_page_line}
}});
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

/// Generate a standalone test page using the widget-html format.
///
/// The template must contain `{WIDGET_BLOCK}` and `{TITLE}` placeholders.
pub fn generate_test_html_widget_html(
    export: &WidgetExport,
    title: &str,
    sources: &WidgetSources,
    minified: bool,
    style_page: Option<bool>,
) -> Result<String> {
    let embed_block = generate_embed_html_widget_html(export, sources, false, style_page)?;

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
    write_html_file(path, &html, "embed HTML")
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
    write_html_file(path, &html, "test HTML")
}

pub fn write_embed_html_widget_html(
    path: &Path,
    export: &WidgetExport,
    sources: &WidgetSources,
    minified: bool,
    style_page: Option<bool>,
) -> Result<()> {
    let html = generate_embed_html_widget_html(export, sources, minified, style_page)?;
    write_html_file(path, &html, "embed HTML (widget-html)")
}

pub fn write_test_html_widget_html(
    path: &Path,
    export: &WidgetExport,
    title: &str,
    sources: &WidgetSources,
    minified: bool,
    style_page: Option<bool>,
) -> Result<()> {
    let html = generate_test_html_widget_html(export, title, sources, minified, style_page)?;
    write_html_file(path, &html, "test HTML (widget-html)")
}

fn write_html_file(path: &Path, html: &str, label: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }
    }
    std::fs::write(path, html.as_bytes())
        .with_context(|| format!("Failed to write {label}: {}", path.display()))?;
    eprintln!("Written: {} ({})", path.display(), format_size(html.len()));
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn minify_html_content(html: &str) -> Result<String> {
    let cfg = minify_html::Cfg {
        // JS and CSS are pre-minified by esbuild; only strip HTML whitespace here.
        minify_css: false,
        minify_js: false,
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
