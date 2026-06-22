/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Optional QR-code page decoration.
//!
//! When a layout job sets [`LayoutConfig::qr_url`](crate::config::LayoutConfig::qr_url),
//! the URL is encoded as a QR code and placed in the bottom-right corner of every
//! page via a `#set page(foreground: …)` directive. The QR is rendered as an
//! inline SVG (no external asset file) and is always black-on-white for
//! scannability, independent of the job's color mode.

use std::fmt::Write as _;

use qrcode::types::Color;
use qrcode::QrCode;

/// Default QR box size (a Typst length) when the job doesn't override it.
pub const DEFAULT_QR_SIZE: &str = "0.75in";
/// Default caption (above-QR) text size (a Typst length).
pub const DEFAULT_CAPTION_SIZE: &str = "9pt";
/// Default URL (below-QR) text size (a Typst length).
pub const DEFAULT_URL_SIZE: &str = "7pt";
/// Quiet-zone width in modules baked into the rendered SVG. Kept small (1) so
/// the caption/URL sit close to the code; the surrounding white card adds the
/// rest of the visual margin.
const QUIET_ZONE_MODULES: usize = 1;

/// Render `url` as a compact SVG QR-code string, or `None` when it can't be
/// encoded (e.g. the URL is longer than the largest QR version can hold).
///
/// Built by hand from the module matrix (rather than the crate's SVG renderer)
/// so the quiet zone is a tight [`QUIET_ZONE_MODULES`] instead of the standard 4,
/// which otherwise leaves a large gap between the code and the caption/URL.
fn qr_svg(url: &str) -> Option<String> {
    let code = QrCode::new(url.as_bytes()).ok()?;
    let n = code.width();
    let colors = code.to_colors();
    let qz = QUIET_ZONE_MODULES;
    let dim = n + 2 * qz;

    let mut path = String::new();
    for y in 0..n {
        for x in 0..n {
            if colors[y * n + x] == Color::Dark {
                // A 1×1 module square at the quiet-zone-offset position.
                let _ = write!(path, "M{} {}h1v1h-1z", x + qz, y + qz);
            }
        }
    }

    Some(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {dim} {dim}\" \
         shape-rendering=\"crispEdges\">\
         <rect width=\"{dim}\" height=\"{dim}\" fill=\"#ffffff\"/>\
         <path d=\"{path}\" fill=\"#000000\"/></svg>",
    ))
}

/// Build the `#set page(foreground: …)` directive that places a QR code for
/// `url` in the bottom-right corner, or `None` when the URL can't be encoded.
///
/// The corner element is a centered white card stacking, top to bottom: the
/// optional `msg` caption (e.g. "Register Here") in the heading font, the QR
/// code, and the `url` in small text. The card sits a page-edge margin in from
/// the right and clears the footer band; the white background keeps it legible
/// over page content.
///
/// `caption_size`/`url_size` are Typst lengths for the caption and URL text
/// (e.g. `"12.5pt"`). `size_expr` is a Typst length used for the QR's
/// width/height and the column the caption/URL wrap within. Geometry `#let`s
/// (`_page-edge`, `_footer-bottom`) and `_heading-font` come from the preamble,
/// so this must be emitted after the font/geometry lets.
pub(crate) fn qr_page_foreground(
    url: &str,
    msg: Option<&str>,
    caption_size: &str,
    url_size: &str,
    size_expr: &str,
) -> Option<String> {
    use crate::typst_gen::escape_typst;

    let svg = qr_svg(url)?;
    // Escape for embedding inside a Typst double-quoted string. SVG never
    // contains backslashes, but escape them too for safety.
    let escaped = svg.replace('\\', "\\\\").replace('"', "\\\"");

    // Caption cell above the QR (only when a message is set), in the heading font.
    let caption_cell = msg
        .filter(|m| !m.is_empty())
        .map(|m| {
            format!(
                "[#text(.._heading-font, size: {caption_size})[{}]],\n      ",
                escape_typst(m),
            )
        })
        .unwrap_or_default();

    // Tight row gutter so the caption/URL hug the code; the QR image and the URL
    // share the column width so they stay aligned and the URL wraps within it.
    Some(format!(
        "#set page(foreground: place(bottom + right, dx: -_page-edge, dy: -_footer-bottom, \
         block(fill: white, inset: 5pt)[\n    \
           #grid(columns: ({size},), row-gutter: 1pt, align: center,\n      \
             {caption}[#image(bytes(\"{svg}\"), format: \"svg\", width: {size}, height: {size})],\n      \
             [#text(size: {url_size}, fill: luma(80))[{url}]],\n    \
           )\n  \
         ]))\n",
        size = size_expr,
        caption = caption_cell,
        svg = escaped,
        url_size = url_size,
        url = escape_typst(url),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qr_svg_encodes_url() {
        let svg = qr_svg("https://cosplayamerica.com").expect("encodable");
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn test_qr_foreground_directive() {
        let out = qr_page_foreground(
            "https://example.com",
            None,
            DEFAULT_CAPTION_SIZE,
            DEFAULT_URL_SIZE,
            "0.75in",
        )
        .unwrap();
        assert!(out.contains("#set page(foreground:"));
        assert!(out.contains("bottom + right"));
        assert!(out.contains("width: 0.75in"));
        assert!(out.contains("format: \"svg\""));
        // The embedded SVG quotes must be escaped for the Typst string literal.
        assert!(out.contains("\\\""));
        // The URL is shown below the code, at the default size.
        assert!(out.contains("https://example.com"));
        assert!(out.contains("size: 7pt"));
    }

    #[test]
    fn test_qr_foreground_caption_and_sizes() {
        let with = qr_page_foreground(
            "https://example.com",
            Some("Register Here"),
            "12.5pt",
            "10pt",
            "0.75in",
        )
        .unwrap();
        assert!(with.contains("Register Here"));
        assert!(with.contains("size: 12.5pt")); // caption size override
        assert!(with.contains("size: 10pt")); // url size override
                                              // No caption cell when the message is absent.
        let without =
            qr_page_foreground("https://example.com", None, "9pt", "7pt", "0.75in").unwrap();
        assert!(!without.contains("_heading-font"));
    }
}
