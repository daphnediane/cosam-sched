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

use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

// Pre-minified by esbuild (via `npm run build` / build.rs).
// Using the esbuild output avoids minify-html's JS minifier, which
// double-escapes \uXXXX sequences in string literals.
const BUILTIN_CSS: &str = include_str!("../../../widget/cosam-calendar.min.css");
const BUILTIN_JS: &str = include_str!("../../../widget/cosam-calendar.min.js");
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

// ── Data compression ─────────────────────────────────────────────────────────

/// Compact JSON whitespace, gzip-compress, and base64-encode for embedding.
///
/// Returns the base64 string. The caller embeds it in a
/// `<script type="application/json" data-encoding="gzip-base64">` tag so the
/// HTML/JS minifier never touches the data, and the browser decompresses it
/// with the native `DecompressionStream` API before handing it to the widget.
fn compress_and_encode(json_data: &str) -> Result<String> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use flate2::{write::GzEncoder, Compression};

    // Re-serialize without whitespace to maximize compression ratio.
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
    let encoded_data = compress_and_encode(json_data)?;
    let raw = format!(
        r#"{COPYRIGHT_COMMENT}
<style>
{css}
</style>
<div id="cosam-calendar-root"></div>
<script type="application/json" id="cosam-schedule-data">
{encoded_data}
</script>
<script>
// CosAm Calendar Widget - Embeddable Version
// Copyright (c) 2026 Daphne Pfister
// SPDX-License-Identifier: BSD-2-Clause
// Project: https://github.com/daphnediane/cosam-sched

// Widget code
{js}

// Initialize widget — data is gzip+base64 (detected by H4sI signature)
(function() {{
    var dataEl = document.getElementById('cosam-schedule-data');
    if (!dataEl || typeof CosAmCalendar === 'undefined') return;
    var raw = dataEl.textContent.trim();
    if (raw.substring(0, 4) === 'H4sI') {{
        var bytes = Uint8Array.from(atob(raw), function(c) {{ return c.charCodeAt(0); }});
        var ds = new DecompressionStream('gzip');
        var writer = ds.writable.getWriter();
        writer.write(bytes);
        writer.close();
        new Response(ds.readable).arrayBuffer().then(function(buf) {{
            CosAmCalendar.init({{
                el: document.getElementById('cosam-calendar-root'),
                data: JSON.parse(new TextDecoder().decode(buf)),{style_page_line}
            }});
        }});
    }} else {{
        CosAmCalendar.init({{
            el: document.getElementById('cosam-calendar-root'),
            data: JSON.parse(raw),{style_page_line}
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
