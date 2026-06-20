/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! HTML widget embedding for cosam-convert.
//!
//! Produces self-contained HTML files (embed or full test page) by inlining
//! the widget CSS, JS, and schedule data. Two formats are supported:
//! - Widget-html format (default): structural data is a compact JSON block;
//!   panels are semantic HTML outside `#cosam-calendar-root`. The inlined
//!   `load-html-embed.min.js` reads both and hands them to
//!   `CosAmCalendar.HtmlEmbedLoader`.
//! - JSON format (`--embed-as-json`): schedule data is gzip+base64-encoded
//!   JSON. The inlined `load-json-embed.min.js` reads `#cosam-schedule-data`
//!   and hands it to `CosAmCalendar.JsonEmbedLoader`.
//!
//! Assets are compiled-in by default; callers can override via
//! `--widget-css`, `--widget-js`, `--test-template`.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use schedule_core::widget_json::{ScheduleConfig, WidgetExport};

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

/// Build the widget bootstrap that mounts `#cosam-calendar-root`.
///
/// A bare one-shot `init()` only fires on a full page parse. Squarespace 7.0
/// (and other Ajax-navigation hosts) swap page content over XHR without firing
/// `DOMContentLoaded`, so a one-shot init never re-runs and the widget stays
/// blank when the page is reached via the site nav. This bootstrap mounts on
/// first parse and re-mounts on every signal the active template might emit:
/// `DOMContentLoaded`/`load`, Squarespace's `mercury:load` event, the official
/// `Squarespace.onInitialize` hook, and a `MutationObserver` fallback. A
/// `data-cosam-mounted` marker keeps it idempotent across all of them.
///
/// Init line offsetting sticky headers below a host fixed top bar. Squarespace's
/// mobile nav (`[data-nc-base="mobile-bar"]` / `.Mobile-bar--top`) is
/// `position: fixed; top: 0` under its mobile breakpoint, so without this the
/// widget's sticky day/time headers pin behind it. The selector is harmless on
/// non-Squarespace hosts (it simply matches nothing → 0 offset).
const STICKY_OFFSET_LINE: &str =
    "\n            stickyOffsetSelector: '[data-nc-base=\"mobile-bar\"], .Mobile-bar--top',";

/// `loader_expr` is the JS expression producing the loader (e.g.
/// `CosAmCalendar.HtmlEmbedLoader()`); `style_page_line` is the optional
/// `stylePageBody` opt line (already indented, or empty).
fn init_bootstrap(loader_expr: &str, style_page_line: &str) -> String {
    format!(
        r#"// Initialize widget — resilient to Squarespace 7.0 Ajax navigation.
// Direct loads parse this inline, but in-site nav swaps content via XHR without
// firing DOMContentLoaded, so we (re)mount on every signal the template emits.
(function () {{
    function mount() {{
        if (!window.CosAmCalendar) return;
        var el = document.getElementById('cosam-calendar-root');
        if (!el || el.getAttribute('data-cosam-mounted') === '1') return;
        el.setAttribute('data-cosam-mounted', '1');
        CosAmCalendar.init({{
            el: el,
            loader: {loader_expr},{STICKY_OFFSET_LINE}{style_page_line}
        }});
    }}
    document.addEventListener('DOMContentLoaded', mount);
    window.addEventListener('load', mount);
    // Squarespace 7.0 fires mercury:load after each Ajax page transition.
    window.addEventListener('mercury:load', mount);
    // Official Squarespace hook — runs on initial load and after every Ajax
    // transition. Needs the YUI instance, which may not be in scope; guard it.
    try {{
        if (window.Squarespace && typeof window.Squarespace.onInitialize === 'function') {{
            window.Squarespace.onInitialize(typeof Y !== 'undefined' ? Y : window.Y, mount);
        }}
    }} catch (e) {{ /* Y unavailable — the other triggers cover it */ }}
    // Fallback: mount as soon as the root lands in the DOM, regardless of which
    // (if any) navigation event the active template dispatches.
    if (window.MutationObserver) {{
        new MutationObserver(mount).observe(document.documentElement, {{ childList: true, subtree: true }});
    }}
    mount();
}})();"#
    )
}

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

/// Generate embeddable HTML snippet using the gzip+base64 JSON format (`--embed-as-json`).
///
/// Schedule data is compressed and embedded in a `<script>` tag; the widget
/// decompresses it at runtime. All CSS, JS, and data are inlined.
/// The result can be pasted into a Squarespace Code Block or any raw-HTML page.
pub fn generate_embed_html(
    json_data: &str,
    config: Option<&ScheduleConfig>,
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
    // Optional presentation config (branding + print-format defaults), emitted as
    // its own ScheduleConfig <script> the default EmbeddedConfigLoader reads.
    // Absent when no brand/widget config is available.
    let config_html = match config {
        Some(cfg) => static_html::generate_config_html(cfg)?,
        None => String::new(),
    };
    let json_loader = BUILTIN_JSON_EMBED_LOADER;
    let raw = format!(
        r#"{COPYRIGHT_COMMENT}
<style>
{css}
</style>
<div id="cosam-calendar-root"><p style="padding:40px 20px;text-align:center">Schedule failed to load. Please enable JavaScript and reload the page.</p></div>
{config_html}
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

{bootstrap}
</script>"#,
        css = sources.css,
        js = sources.js,
        bootstrap = init_bootstrap("CosAmCalendar.JsonEmbedLoader()", style_page_line),
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
    config: Option<&ScheduleConfig>,
    title: &str,
    sources: &WidgetSources,
    minified: bool,
    style_page: Option<bool>,
) -> Result<String> {
    let embed_block = generate_embed_html(json_data, config, sources, false, style_page)?;

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

/// Generate embeddable HTML snippet using the widget-html format (default).
///
/// Schedule data is rendered as a compact JSON block (structural data) plus
/// semantic `<article>` elements (panels). All CSS, JS, and data are inlined.
pub fn generate_embed_html_widget_html(
    export: &WidgetExport,
    config: Option<&ScheduleConfig>,
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
    // Optional presentation config (branding + print-format defaults), emitted as
    // its own ScheduleConfig <script> so the embedded widget can match the
    // printed house style. Absent when no brand/widget config is available.
    let config_html = match config {
        Some(cfg) => static_html::generate_config_html(cfg)?,
        None => String::new(),
    };
    let html_loader = BUILTIN_HTML_EMBED_LOADER;
    let raw = format!(
        r#"{COPYRIGHT_COMMENT}
<style>
{css}
</style>
<div id="cosam-calendar-root"></div>
{config_html}
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

{bootstrap}
</script>"#,
        css = sources.css,
        js = sources.js,
        bootstrap = init_bootstrap("CosAmCalendar.HtmlEmbedLoader()", style_page_line),
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
    config: Option<&ScheduleConfig>,
    title: &str,
    sources: &WidgetSources,
    minified: bool,
    style_page: Option<bool>,
) -> Result<String> {
    let embed_block = generate_embed_html_widget_html(export, config, sources, false, style_page)?;

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

// ── Split embed (head engine + page content) ──────────────────────────────────
//
// Squarespace 7.0 (Mercury) and similar Ajax-navigation hosts load pages over
// XHR and do NOT execute inline `<script>` in injected page content — so a
// single all-in-one embed mounts only on a direct hit and stays blank when the
// page is reached via in-site navigation. The split form fixes this:
// - The **head engine** (CSS + widget JS + loader + resident bootstrap) goes in
//   site-wide **Code Injection → Header**. It runs once on the first full page
//   load and stays resident in `window` across every Ajax transition.
// - The **page content** (root div + schedule data + panels) goes in the page's
//   Code Block. Mercury reloads this region, so the data is always present.
//
// The resident bootstrap mounts idempotently and re-mounts after each navigation
// via events, `Squarespace.onInitialize`, and a MutationObserver fallback.

/// Build the resident bootstrap for the head engine.
///
/// Unlike [`init_bootstrap`], this lives in the page `<head>` and tolerates the
/// page content arriving later (initial load) or being swapped in by an Ajax
/// navigation. It waits until the page data is fully present (`ready_expr` — a
/// JS expression truthy only once the loader has everything it needs), guards
/// against double-mount, debounces so a multi-step content insertion settles
/// before reading it, and listens on every plausible signal.
///
/// `ready_expr` must be format-specific: the widget-html loader reads separate
/// `<article>` panels that Mercury inserts after the structural data script, so
/// mounting on the data script alone yields an empty schedule — gate on the
/// panels instead.
fn resident_bootstrap(loader_expr: &str, style_page_line: &str, ready_expr: &str) -> String {
    format!(
        r#"// Resident widget bootstrap — site-wide Code Injection (Header).
// The host swaps page content over XHR without re-running inline scripts in the
// code block, so the engine loads once here and stays resident. mount() is
// idempotent (data-cosam-mounted guard), waits for the page data to be fully
// present, and fires on first load and after every Ajax transition via
// navigation events, Squarespace.onInitialize, and a MutationObserver fallback.
(function () {{
    var pending = false;
    // Truthy only once the loader has everything it needs in the DOM. Mercury
    // inserts content in steps, so mounting too early reads an empty schedule.
    function dataReady() {{ return {ready_expr}; }}
    function mount() {{
        if (!window.CosAmCalendar) return;
        var el = document.getElementById('cosam-calendar-root');
        if (!el || el.getAttribute('data-cosam-mounted') === '1') return;
        if (!dataReady()) return;
        el.setAttribute('data-cosam-mounted', '1');
        CosAmCalendar.init({{
            el: el,
            loader: {loader_expr},{STICKY_OFFSET_LINE}{style_page_line}
        }});
    }}
    // Debounce: let an in-flight content insertion settle, then try to mount.
    // If it is still not ready, mount() no-ops and a later signal retries.
    function schedule() {{
        if (pending) return;
        pending = true;
        setTimeout(function () {{ pending = false; mount(); }}, 60);
    }}
    document.addEventListener('DOMContentLoaded', schedule);
    window.addEventListener('load', schedule);
    // Squarespace 7.0 fires mercury:load after each Ajax page transition.
    window.addEventListener('mercury:load', schedule);
    // Official Squarespace hook — runs on initial load and after every Ajax
    // transition. Needs the YUI instance, which may not be in scope; guard it.
    try {{
        if (window.Squarespace && typeof window.Squarespace.onInitialize === 'function') {{
            window.Squarespace.onInitialize(typeof Y !== 'undefined' ? Y : window.Y, schedule);
        }}
    }} catch (e) {{ /* Y unavailable — the other triggers cover it */ }}
    // Fallback: re-check whenever the DOM changes, regardless of which (if any)
    // navigation event the active template dispatches.
    if (window.MutationObserver) {{
        new MutationObserver(schedule).observe(document.documentElement, {{ childList: true, subtree: true }});
    }}
    schedule();
}})();"#
    )
}

/// Generate the head engine snippet for the widget-html format.
///
/// Contains the CSS, widget JS, HTML embed loader, and resident bootstrap — but
/// no schedule data. Paste once into site-wide Code Injection → Header.
pub fn generate_embed_head_widget_html(
    config: Option<&ScheduleConfig>,
    sources: &WidgetSources,
    minified: bool,
    style_page: Option<bool>,
) -> Result<String> {
    let style_page_line = match style_page {
        Some(true) => "\n            stylePageBody: true,",
        Some(false) => "\n            stylePageBody: false,",
        None => "",
    };
    let config_html = if let Some(cfg) = config {
        static_html::generate_config_html(cfg)?
    } else {
        String::new()
    };
    let html_loader = BUILTIN_HTML_EMBED_LOADER;
    let raw = format!(
        r#"{COPYRIGHT_COMMENT}
<style>
{css}
</style>
{config_html}
<script>
// CosAm Calendar Widget - Engine (site-wide Code Injection: Header)
// Copyright (c) 2026 Daphne Pfister
// SPDX-License-Identifier: BSD-2-Clause
// Project: https://github.com/daphnediane/cosam-sched
// Includes: qrcode (MIT) https://github.com/soldair/node-qrcode

// Widget code
{js}

// HTML embed loader
{html_loader}

{bootstrap}
</script>"#,
        css = sources.css,
        js = sources.js,
        bootstrap = resident_bootstrap(
            "CosAmCalendar.HtmlEmbedLoader()",
            style_page_line,
            // Panels are separate <article> elements inserted after the data
            // script; wait for at least one so we never mount an empty schedule.
            "!!document.querySelector('.cosam-static-schedule article.cosam-panel')",
        ),
    );

    if minified {
        minify_html_content(&raw)
    } else {
        Ok(raw)
    }
}

/// Generate the page content snippet for the widget-html format.
///
/// Contains the root div, schedule data, and static panels — but no CSS or JS.
/// Paste into the page's Code Block; pairs with [`generate_embed_head_widget_html`].
pub fn generate_embed_body_widget_html(export: &WidgetExport, minified: bool) -> Result<String> {
    let schedule_html = static_html::generate_static_schedule_html(export)?;
    let raw = format!(
        r#"{COPYRIGHT_COMMENT}
<div id="cosam-calendar-root"></div>
{schedule_html}"#
    );

    if minified {
        minify_html_content(&raw)
    } else {
        Ok(raw)
    }
}

/// Generate the head engine snippet for the gzip+base64 JSON format.
pub fn generate_embed_head_json(
    config: Option<&ScheduleConfig>,
    sources: &WidgetSources,
    minified: bool,
    style_page: Option<bool>,
) -> Result<String> {
    let style_page_line = match style_page {
        Some(true) => "\n            stylePageBody: true,",
        Some(false) => "\n            stylePageBody: false,",
        None => "",
    };
    // Presentation config (branding + print-format defaults) ships in the head
    // engine so the resident widget has it before page content arrives.
    let config_html = match config {
        Some(cfg) => static_html::generate_config_html(cfg)?,
        None => String::new(),
    };
    let json_loader = BUILTIN_JSON_EMBED_LOADER;
    let raw = format!(
        r#"{COPYRIGHT_COMMENT}
<style>
{css}
</style>
{config_html}
<script>
// CosAm Calendar Widget - Engine (site-wide Code Injection: Header)
// Copyright (c) 2026 Daphne Pfister
// SPDX-License-Identifier: BSD-2-Clause
// Project: https://github.com/daphnediane/cosam-sched
// Includes: qrcode (MIT) https://github.com/soldair/node-qrcode

// Widget code
{js}

// JSON embed loader
{json_loader}

{bootstrap}
</script>"#,
        css = sources.css,
        js = sources.js,
        bootstrap = resident_bootstrap(
            "CosAmCalendar.JsonEmbedLoader()",
            style_page_line,
            // All data (including panels) lives in the single base64 script tag.
            "!!(document.getElementById('cosam-schedule-data') && document.getElementById('cosam-schedule-data').textContent.trim())",
        ),
    );

    if minified {
        minify_html_content(&raw)
    } else {
        Ok(raw)
    }
}

/// Generate the page content snippet for the gzip+base64 JSON format.
pub fn generate_embed_body_json(json_data: &str, minified: bool) -> Result<String> {
    let encoded_data = compress_and_encode(json_data)?;
    let raw = format!(
        r#"{COPYRIGHT_COMMENT}
<div id="cosam-calendar-root"><p style="padding:40px 20px;text-align:center">Schedule failed to load. Please enable JavaScript and reload the page.</p></div>
<script type="application/json" id="cosam-schedule-data">
{encoded_data}
</script>"#
    );

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
    config: Option<&ScheduleConfig>,
    sources: &WidgetSources,
    minified: bool,
    style_page: Option<bool>,
) -> Result<()> {
    let html = generate_embed_html(json_data, config, sources, minified, style_page)?;
    write_html_file(path, &html, "embed HTML")
}

pub fn write_test_html(
    path: &Path,
    json_data: &str,
    config: Option<&ScheduleConfig>,
    title: &str,
    sources: &WidgetSources,
    minified: bool,
    style_page: Option<bool>,
) -> Result<()> {
    let html = generate_test_html(json_data, config, title, sources, minified, style_page)?;
    write_html_file(path, &html, "test HTML")
}

pub fn write_embed_html_widget_html(
    path: &Path,
    export: &WidgetExport,
    config: Option<&ScheduleConfig>,
    sources: &WidgetSources,
    minified: bool,
    style_page: Option<bool>,
) -> Result<()> {
    let html = generate_embed_html_widget_html(export, config, sources, minified, style_page)?;
    write_html_file(path, &html, "embed HTML (widget-html)")
}

pub fn write_test_html_widget_html(
    path: &Path,
    export: &WidgetExport,
    config: Option<&ScheduleConfig>,
    title: &str,
    sources: &WidgetSources,
    minified: bool,
    style_page: Option<bool>,
) -> Result<()> {
    let html = generate_test_html_widget_html(export, config, title, sources, minified, style_page)?;
    write_html_file(path, &html, "test HTML (widget-html)")
}

pub fn write_embed_head_widget_html(
    path: &Path,
    config: Option<&ScheduleConfig>,
    sources: &WidgetSources,
    minified: bool,
    style_page: Option<bool>,
) -> Result<()> {
    let html = generate_embed_head_widget_html(config, sources, minified, style_page)?;
    write_html_file(path, &html, "embed head engine (widget-html)")
}

pub fn write_embed_body_widget_html(
    path: &Path,
    export: &WidgetExport,
    minified: bool,
) -> Result<()> {
    let html = generate_embed_body_widget_html(export, minified)?;
    write_html_file(path, &html, "embed page content (widget-html)")
}

pub fn write_embed_head_json(
    path: &Path,
    config: Option<&ScheduleConfig>,
    sources: &WidgetSources,
    minified: bool,
    style_page: Option<bool>,
) -> Result<()> {
    let html = generate_embed_head_json(config, sources, minified, style_page)?;
    write_html_file(path, &html, "embed head engine (json)")
}

pub fn write_embed_body_json(path: &Path, json_data: &str, minified: bool) -> Result<()> {
    let html = generate_embed_body_json(json_data, minified)?;
    write_html_file(path, &html, "embed page content (json)")
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
