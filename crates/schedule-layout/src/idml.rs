/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Adobe IDML (InDesign Markup Language) package generation.
//!
//! [`generate_idml`] turns a [`ScheduleData`] into the bytes of a `.idml`
//! package — a ZIP of XML parts that Adobe InDesign can open and hand-edit. It is
//! an alternative output to the Typst/PDF pipeline, selected by
//! [`LayoutFormat::Idml`](crate::config::LayoutFormat::Idml).
//!
//! # Scope (v1)
//!
//! v1 renders a **threaded text listing**: panels grouped by day → time slot,
//! flowed through a chain of linked text frames across as many spreads as needed.
//! Text detail follows [`ContentMode`]:
//!
//! - [`ContentMode::PanelList`] → compact (name, time, room).
//! - everything else → full (name, time · room(s) · presenters, description).
//!
//! The schedule **grid** (a [`ContentMode::GridOnly`]/[`ContentMode::Both`]
//! feature) is not yet rendered — those modes emit only their text portion and a
//! one-line notice. The grid maps onto an InDesign `<Table>` built from
//! [`GridLayout`](crate::timegrid::GridLayout) and is planned as a follow-up.
//!
//! # Structure
//!
//! The emitted package mirrors what InDesign itself writes: `mimetype` (stored,
//! first), `designmap.xml`, `META-INF/`, `Resources/` (Fonts/Styles/Graphic/
//! Preferences), one `MasterSpreads/` part, N `Spreads/`, one `Stories/` part,
//! and `XML/` (BackingStory/Tags). All ids are deterministic so output is
//! byte-stable across runs.

use std::io::Write;

use thiserror::Error;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::brand::BrandConfig;
use crate::config::{ContentMode, LayoutConfig, Orientation, PanelFilter};
use crate::model::{Panel, ScheduleData};
use crate::time_fmt;

/// IDML DOM version targeted. 8.0 (InDesign CS6) maximizes forward compatibility:
/// newer InDesign opens older IDML without complaint.
const DOM_VERSION: &str = "8.0";

/// `idPkg` packaging namespace used on every part's root element.
const IDPKG_NS: &str = "http://ns.adobe.com/AdobeInDesign/idml/1.0/packaging";

/// Page margin, in points (0.5 inch).
const MARGIN_PT: f64 = 36.0;

/// Errors produced while building an IDML package.
#[derive(Debug, Error)]
pub enum IdmlError {
    #[error("IDML zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("IDML I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("no panels to render")]
    Empty,
}

/// A named paragraph style applied to a generated paragraph.
#[derive(Debug, Clone, Copy)]
enum ParaStyle {
    Day,
    Slot,
    Title,
    Meta,
    Body,
}

impl ParaStyle {
    /// Style name, used both as the `ParagraphStyle/<name>` id and display name.
    fn name(self) -> &'static str {
        match self {
            ParaStyle::Day => "Day",
            ParaStyle::Slot => "Slot",
            ParaStyle::Title => "Title",
            ParaStyle::Meta => "Meta",
            ParaStyle::Body => "Body",
        }
    }

    /// Point size as a multiple of the body base size.
    fn size(self, base: f64) -> f64 {
        match self {
            ParaStyle::Day => base * 2.0,
            ParaStyle::Slot => base * 1.55,
            ParaStyle::Title => base * 1.2,
            ParaStyle::Meta | ParaStyle::Body => base,
        }
    }

    /// Space-before, in points, as a multiple of the body base size.
    fn space_before(self, base: f64) -> f64 {
        match self {
            ParaStyle::Day => base * 1.2,
            ParaStyle::Slot => base * 0.8,
            ParaStyle::Title => base * 0.5,
            ParaStyle::Meta | ParaStyle::Body => 0.0,
        }
    }
}

/// One paragraph in the generated story.
struct Para {
    style: ParaStyle,
    text: String,
}

/// Fully-resolved paragraph style: which font and InDesign `FontStyle` name to
/// request, plus point size and space-before. Built from the brand config so the
/// requested style names match the brand's actual weights (e.g. Trend Sans's
/// light weight, not a fabricated "Bold").
struct StyleSpec {
    style: ParaStyle,
    size: f64,
    space_before: f64,
    /// Font family name.
    font: String,
    /// InDesign style name (e.g. `"Regular"`, `"Light"`, `"Italic"`).
    font_style: String,
}

/// Resolve every paragraph style from the brand config and body base size.
///
/// Headings (Day/Slot/Title) use the heading typeface at its configured weight;
/// hierarchy comes from point size, not a forced bold. Meta is the body face in
/// italic; Body is the body face at its configured weight.
fn style_specs(brand: &BrandConfig, base: f64) -> Vec<StyleSpec> {
    let heading = brand.fonts.heading_or_default().to_string();
    let body = brand.fonts.body_or_default().to_string();
    // Prefer an explicit IDML style name (e.g. Trend Sans's "One") when the brand
    // provides one; otherwise map the numeric Typst weight to a standard name.
    let heading_style = brand
        .fonts
        .heading_idml_style()
        .map(str::to_string)
        .unwrap_or_else(|| {
            indesign_font_style(brand.fonts.heading_weight(), brand.fonts.heading_style())
        });
    let body_style = brand
        .fonts
        .body_idml_style()
        .map(str::to_string)
        .unwrap_or_else(|| {
            indesign_font_style(brand.fonts.body_weight(), brand.fonts.body_style())
        });
    // Meta: body face, forced italic (regardless of the configured body style).
    let meta_style = indesign_font_style(brand.fonts.body_weight(), Some("italic"));

    [
        ParaStyle::Day,
        ParaStyle::Slot,
        ParaStyle::Title,
        ParaStyle::Meta,
        ParaStyle::Body,
    ]
    .into_iter()
    .map(|style| {
        let (font, font_style) = match style {
            ParaStyle::Day | ParaStyle::Slot | ParaStyle::Title => {
                (heading.clone(), heading_style.clone())
            }
            ParaStyle::Meta => (body.clone(), meta_style.clone()),
            ParaStyle::Body => (body.clone(), body_style.clone()),
        };
        StyleSpec {
            style,
            size: style.size(base),
            space_before: style.space_before(base),
            font,
            font_style,
        }
    })
    .collect()
}

/// Map a Typst-style weight token and optional style to an InDesign `FontStyle`
/// name. Numeric weights map to the standard CSS-ish names; a non-numeric weight
/// passes through (capitalized) so a brand may name a font's exact style. An
/// italic/oblique style appends `" Italic"` (or yields `"Italic"` for Regular).
fn indesign_font_style(weight: Option<&str>, style: Option<&str>) -> String {
    let base = match weight.map(|w| w.trim()).filter(|w| !w.is_empty()) {
        None => "Regular".to_string(),
        Some(w) => match w.to_ascii_lowercase().as_str() {
            "100" => "Thin".to_string(),
            "200" | "300" | "light" => "Light".to_string(),
            "400" | "regular" | "normal" => "Regular".to_string(),
            "500" | "medium" => "Medium".to_string(),
            "600" | "semibold" => "Semibold".to_string(),
            "700" | "bold" => "Bold".to_string(),
            "800" | "900" | "black" => "Black".to_string(),
            _ => capitalize(w),
        },
    };
    let italic = matches!(
        style.map(|s| s.trim().to_ascii_lowercase()).as_deref(),
        Some("italic") | Some("oblique")
    );
    match (base.as_str(), italic) {
        ("Regular", true) => "Italic".to_string(),
        (_, true) => format!("{base} Italic"),
        (_, false) => base,
    }
}

/// Capitalize the first character, lowercasing the rest.
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first
            .to_uppercase()
            .chain(chars.flat_map(char::to_lowercase))
            .collect(),
    }
}

/// Generate the bytes of an `.idml` package for the given schedule.
///
/// The result is a complete ZIP ready to write to a `.idml` file. Returns
/// [`IdmlError::Empty`] when no panels match the configured filter.
pub fn generate_idml(
    data: &ScheduleData,
    brand: &BrandConfig,
    config: &LayoutConfig,
) -> Result<Vec<u8>, IdmlError> {
    let paras = build_paragraphs(data, config);
    if paras.is_empty() {
        return Err(IdmlError::Empty);
    }

    let base = config.base_font_value();
    let specs = style_specs(brand, base);
    let (page_w, page_h) = page_size_pt(config);
    let frame_w = page_w - 2.0 * MARGIN_PT;
    let frame_h = page_h - 2.0 * MARGIN_PT;
    let page_count = estimate_pages(&paras, frame_w, frame_h, base);

    // --- Deterministic ids ---------------------------------------------------
    let story_id = "story1";
    let master_id = "umaster";
    let layer_id = "ulayer1";
    let frame_ids: Vec<String> = (0..page_count).map(|i| format!("uframe{i}")).collect();
    let spread_ids: Vec<String> = (0..page_count).map(|i| format!("uspread{i}")).collect();

    // --- Assemble parts ------------------------------------------------------
    let mut parts: Vec<(String, Vec<u8>)> = Vec::new();
    let push = |parts: &mut Vec<(String, Vec<u8>)>, name: &str, s: String| {
        parts.push((name.to_string(), s.into_bytes()));
    };

    push(&mut parts, "META-INF/container.xml", container_xml());
    push(&mut parts, "META-INF/metadata.xml", metadata_xml(data));
    push(&mut parts, "Resources/Fonts.xml", fonts_xml(&specs));
    push(&mut parts, "Resources/Styles.xml", styles_xml(&specs));
    push(&mut parts, "Resources/Graphic.xml", graphic_xml(brand));
    push(
        &mut parts,
        "Resources/Preferences.xml",
        preferences_xml(page_w, page_h),
    );
    push(&mut parts, "XML/Tags.xml", tags_xml());
    push(&mut parts, "XML/BackingStory.xml", backing_story_xml());
    push(
        &mut parts,
        &format!("MasterSpreads/MasterSpread_{master_id}.xml"),
        master_spread_xml(master_id, page_w, page_h),
    );
    for i in 0..page_count {
        let prev = if i == 0 {
            "n".to_string()
        } else {
            frame_ids[i - 1].clone()
        };
        let next = if i + 1 == page_count {
            "n".to_string()
        } else {
            frame_ids[i + 1].clone()
        };
        push(
            &mut parts,
            &format!("Spreads/Spread_{}.xml", spread_ids[i]),
            spread_xml(
                &spread_ids[i],
                &frame_ids[i],
                master_id,
                layer_id,
                story_id,
                &prev,
                &next,
                i + 1,
                page_w,
                page_h,
            ),
        );
    }
    push(
        &mut parts,
        &format!("Stories/Story_{story_id}.xml"),
        story_xml(story_id, &paras),
    );

    let doc_name = if data.meta.title.trim().is_empty() {
        "schedule"
    } else {
        data.meta.title.trim()
    };
    let designmap = designmap_xml(story_id, layer_id, master_id, &spread_ids, doc_name);

    // --- Package as ZIP (mimetype first, stored) -----------------------------
    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buf));
        let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        zip.start_file("mimetype", stored)?;
        zip.write_all(b"application/vnd.adobe.indesign-idml-package")?;

        zip.start_file("designmap.xml", deflated)?;
        zip.write_all(designmap.as_bytes())?;

        for (name, bytes) in &parts {
            zip.start_file(name, deflated)?;
            zip.write_all(bytes)?;
        }
        zip.finish()?;
    }
    Ok(buf)
}

// ---------------------------------------------------------------------------
// Content model
// ---------------------------------------------------------------------------

/// Build the ordered paragraph list: panels grouped by day → time slot.
fn build_paragraphs(data: &ScheduleData, config: &LayoutConfig) -> Vec<Para> {
    let compact = matches!(config.content, ContentMode::PanelList { .. });

    let tz = data.meta.timezone.as_str();
    let mut panels: Vec<&Panel> = filter_panels(data, config.panel_filter);
    panels.sort_by(|a, b| {
        a.start_epoch
            .cmp(&b.start_epoch)
            .then_with(|| a.name.cmp(&b.name))
    });

    let mut out: Vec<Para> = Vec::new();
    let mut cur_day: Option<String> = None;
    let mut cur_slot: Option<String> = None;

    for p in panels {
        let start = crate::model::panel_start_iso(p, tz).unwrap_or_default();
        let day_key = start.get(..10).unwrap_or("").to_string();
        let slot_key = start.get(..16).unwrap_or("").to_string();

        if cur_day.as_deref() != Some(day_key.as_str()) {
            out.push(Para {
                style: ParaStyle::Day,
                text: day_heading(&day_key),
            });
            cur_day = Some(day_key);
            cur_slot = None;
        }
        if cur_slot.as_deref() != Some(slot_key.as_str()) {
            let label = time_fmt::format_time(&slot_key);
            if !label.is_empty() {
                out.push(Para {
                    style: ParaStyle::Slot,
                    text: label,
                });
            }
            cur_slot = Some(slot_key);
        }

        out.push(Para {
            style: ParaStyle::Title,
            text: p.name.clone(),
        });
        let meta = panel_meta(data, p, compact);
        if !meta.is_empty() {
            out.push(Para {
                style: ParaStyle::Meta,
                text: meta,
            });
        }
        if !compact {
            if let Some(body) = panel_body(p) {
                out.push(Para {
                    style: ParaStyle::Body,
                    text: body,
                });
            }
        }
    }
    out
}

/// Apply the [`PanelFilter`], mirroring `document::filter_panels`.
fn filter_panels(data: &ScheduleData, filter: PanelFilter) -> Vec<&Panel> {
    let panels = data.scheduled_panels();
    match filter {
        PanelFilter::All => panels,
        PanelFilter::Workshops => panels
            .into_iter()
            .filter(|p| {
                p.panel_type
                    .as_ref()
                    .and_then(|pt| data.panel_types.get(pt.as_str()))
                    .is_some_and(|pt| pt.is_workshop)
            })
            .collect(),
        PanelFilter::Premium => panels.into_iter().filter(|p| p.is_premium).collect(),
    }
}

/// Day heading like `"Friday, June 26"`, falling back to the raw date.
fn day_heading(date: &str) -> String {
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map(|d| d.format("%A, %B %-d").to_string())
        .unwrap_or_else(|_| date.to_string())
}

/// The meta line: `time · room(s)` (compact) or `time · room(s) · presenters`.
fn panel_meta(data: &ScheduleData, panel: &Panel, compact: bool) -> String {
    let mut parts: Vec<String> = Vec::new();
    let tz = data.meta.timezone.as_str();
    let time = time_fmt::format_time_range(
        crate::model::panel_start_iso(panel, tz).as_deref(),
        crate::model::panel_end_iso(panel, tz).as_deref(),
    );
    if !time.is_empty() {
        parts.push(time);
    }
    let rooms = room_names(data, panel);
    if !rooms.is_empty() {
        parts.push(rooms);
    }
    if !compact && !panel.presenters.is_empty() {
        parts.push(panel.presenters.join(", "));
    }
    parts.join(" \u{00b7} ")
}

/// Comma-joined room display names (long preferred), for a panel's rooms.
fn room_names(data: &ScheduleData, panel: &Panel) -> String {
    panel
        .room_ids
        .iter()
        .filter_map(|uid| data.rooms.iter().find(|r| r.uid == *uid))
        .map(|r| {
            if r.long_name.is_empty() {
                r.short_name.clone()
            } else {
                r.long_name.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Body text: description, then note, when present.
fn panel_body(panel: &Panel) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if let Some(d) = panel
        .description
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    {
        parts.push(d.trim().to_string());
    }
    if let Some(n) = panel.note.as_deref().filter(|s| !s.trim().is_empty()) {
        parts.push(n.trim().to_string());
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

// ---------------------------------------------------------------------------
// Geometry / pagination
// ---------------------------------------------------------------------------

/// Page size in points `(width, height)`, oriented per the config.
fn page_size_pt(config: &LayoutConfig) -> (f64, f64) {
    let (w_mm, h_mm) = config.paper.dimensions_mm();
    let to_pt = |mm: f64| mm * 72.0 / 25.4;
    let (w, h) = (to_pt(w_mm), to_pt(h_mm));
    if matches!(config.orientation, Orientation::Landscape) {
        (h.max(w), h.min(w))
    } else {
        (w.min(h), w.max(h))
    }
}

/// Estimate the number of pages needed to flow the paragraphs, with slack.
fn estimate_pages(paras: &[Para], frame_w: f64, frame_h: f64, base: f64) -> usize {
    let mut total_lines = 0.0_f64;
    for p in paras {
        let size = p.style.size(base);
        let char_w = (size * 0.5).max(1.0);
        let chars_per_line = (frame_w / char_w).max(1.0);
        let len = p.text.chars().count() as f64;
        let lines = (len / chars_per_line).ceil().max(1.0);
        // Approximate vertical cost in body-leading units.
        let leading = size * 1.2 + p.style.space_before(base);
        total_lines += lines * leading;
    }
    let page_capacity = frame_h.max(1.0);
    let pages = (total_lines / page_capacity * 1.15).ceil() as usize + 1;
    pages.clamp(1, 500)
}

// ---------------------------------------------------------------------------
// XML parts
// ---------------------------------------------------------------------------

/// Escape text for an XML element body or attribute value.
fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            // Strip control chars InDesign rejects (keep tab/newline out of attrs;
            // paragraph breaks are modelled with <Br/>, not literal newlines).
            c if (c as u32) < 0x20 => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// `<?xml ...?>` declaration shared by every part.
fn xml_decl() -> &'static str {
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n"
}

fn container_xml() -> String {
    format!(
        "{decl}<container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\">\n\
         \t<rootfiles>\n\
         \t\t<rootfile full-path=\"designmap.xml\" media-type=\"text/xml\"></rootfile>\n\
         \t</rootfiles>\n\
         </container>\n",
        decl = xml_decl()
    )
}

fn metadata_xml(data: &ScheduleData) -> String {
    // A minimal XMP packet; InDesign rewrites this on save.
    format!(
        "{decl}<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n\
         \t<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
         \t\t<rdf:Description xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n\
         \t\t\t<dc:title>{title}</dc:title>\n\
         \t\t</rdf:Description>\n\
         \t</rdf:RDF>\n\
         </x:xmpmeta>\n",
        decl = xml_decl(),
        title = xml_escape(&data.meta.title)
    )
}

fn tags_xml() -> String {
    format!(
        "{decl}<idPkg:Tags xmlns:idPkg=\"{ns}\" DOMVersion=\"{ver}\">\n\
         \t<XMLTag Self=\"XMLTag/Root\" Name=\"Root\">\n\
         \t\t<Properties>\n\
         \t\t\t<TagColor type=\"enumeration\">LightBlue</TagColor>\n\
         \t\t</Properties>\n\
         \t</XMLTag>\n\
         </idPkg:Tags>\n",
        decl = xml_decl(),
        ns = IDPKG_NS,
        ver = DOM_VERSION
    )
}

fn backing_story_xml() -> String {
    format!(
        "{decl}<idPkg:BackingStory xmlns:idPkg=\"{ns}\" DOMVersion=\"{ver}\">\n\
         \t<XmlStory Self=\"ubackingstory\" AppliedTOCStyle=\"n\" TrackChanges=\"false\" StoryTitle=\"$ID/\" AppliedNamedGrid=\"n\">\n\
         \t\t<ParagraphStyleRange AppliedParagraphStyle=\"ParagraphStyle/$ID/NormalParagraphStyle\">\n\
         \t\t\t<CharacterStyleRange AppliedCharacterStyle=\"CharacterStyle/$ID/[No character style]\">\n\
         \t\t\t\t<XMLElement Self=\"uroot\" MarkupTag=\"XMLTag/Root\" />\n\
         \t\t\t</CharacterStyleRange>\n\
         \t\t</ParagraphStyleRange>\n\
         \t</XmlStory>\n\
         </idPkg:BackingStory>\n",
        decl = xml_decl(),
        ns = IDPKG_NS,
        ver = DOM_VERSION
    )
}

/// Declare each font family and the exact set of styles the paragraph styles
/// request, so the `Fonts.xml` advertisement stays consistent with `Styles.xml`.
/// `Status="Substituted"` lets InDesign substitute when the font is not
/// installed (the schedule's brand fonts live in a font directory, not the
/// system), without flagging an inconsistent document.
fn fonts_xml(specs: &[StyleSpec]) -> String {
    // Distinct (family, style) pairs grouped by family, in first-seen order.
    let mut families: Vec<(String, Vec<String>)> = Vec::new();
    for spec in specs {
        let entry = match families.iter_mut().find(|(f, _)| f == &spec.font) {
            Some(e) => e,
            None => {
                families.push((spec.font.clone(), Vec::new()));
                families.last_mut().unwrap()
            }
        };
        if !entry.1.contains(&spec.font_style) {
            entry.1.push(spec.font_style.clone());
        }
    }

    let mut s = format!(
        "{decl}<idPkg:Fonts xmlns:idPkg=\"{ns}\" DOMVersion=\"{ver}\">\n",
        decl = xml_decl(),
        ns = IDPKG_NS,
        ver = DOM_VERSION
    );
    for (family, styles) in &families {
        let fam = xml_escape(family);
        let id = format!("font_{}", family.replace([' ', '"'], ""));
        s.push_str(&format!("\t<FontFamily Self=\"{id}\" Name=\"{fam}\">\n"));
        for style in styles {
            let st = xml_escape(style);
            let ps = format!("{}-{}", family.replace(' ', ""), style.replace(' ', ""));
            s.push_str(&format!(
                "\t\t<Font Self=\"{id}_{sid}\" FontFamily=\"{fam}\" Name=\"{fam} {st}\" \
                 PostScriptName=\"{ps}\" Status=\"Substituted\" FontStyleName=\"{st}\" \
                 FontType=\"OpenTypeCFF\" />\n",
                sid = style.replace(' ', ""),
                ps = xml_escape(&ps),
            ));
        }
        s.push_str("\t</FontFamily>\n");
    }
    s.push_str("</idPkg:Fonts>\n");
    s
}

fn graphic_xml(brand: &BrandConfig) -> String {
    let (r, g, b) = parse_hex_rgb(&brand.colors.primary).unwrap_or((0, 188, 221));
    format!(
        "{decl}<idPkg:Graphic xmlns:idPkg=\"{ns}\" DOMVersion=\"{ver}\">\n\
         \t<Color Self=\"Color/Black\" Model=\"Process\" Space=\"CMYK\" ColorValue=\"0 0 0 100\" ColorOverride=\"Specialblack\" Name=\"Black\" ColorEditable=\"false\" ColorRemovable=\"false\" Visible=\"true\" />\n\
         \t<Color Self=\"Color/Paper\" Model=\"Process\" Space=\"CMYK\" ColorValue=\"0 0 0 0\" ColorOverride=\"Specialpaper\" Name=\"Paper\" ColorEditable=\"true\" ColorRemovable=\"false\" Visible=\"true\" />\n\
         \t<Color Self=\"Color/BrandPrimary\" Model=\"Process\" Space=\"RGB\" ColorValue=\"{r} {g} {b}\" ColorOverride=\"Normal\" Name=\"BrandPrimary\" ColorEditable=\"true\" ColorRemovable=\"true\" Visible=\"true\" />\n\
         \t<Swatch Self=\"Swatch/None\" Name=\"None\" ColorEditable=\"false\" ColorRemovable=\"false\" Visible=\"true\" SwatchCreatorID=\"7937\" />\n\
         </idPkg:Graphic>\n",
        decl = xml_decl(),
        ns = IDPKG_NS,
        ver = DOM_VERSION,
    )
}

fn styles_xml(specs: &[StyleSpec]) -> String {
    let mut s = format!(
        "{decl}<idPkg:Styles xmlns:idPkg=\"{ns}\" DOMVersion=\"{ver}\">\n",
        decl = xml_decl(),
        ns = IDPKG_NS,
        ver = DOM_VERSION
    );

    // Required built-in character style.
    s.push_str("\t<RootCharacterStyleGroup Self=\"ucharroot\">\n");
    s.push_str("\t\t<CharacterStyle Self=\"CharacterStyle/$ID/[No character style]\" Name=\"$ID/[No character style]\" />\n");
    s.push_str("\t</RootCharacterStyleGroup>\n");

    // Required built-in paragraph styles, then the custom ones.
    s.push_str("\t<RootParagraphStyleGroup Self=\"upararoot\">\n");
    s.push_str("\t\t<ParagraphStyle Self=\"ParagraphStyle/$ID/[No paragraph style]\" Name=\"$ID/[No paragraph style]\" />\n");
    s.push_str("\t\t<ParagraphStyle Self=\"ParagraphStyle/$ID/NormalParagraphStyle\" Name=\"$ID/NormalParagraphStyle\" />\n");
    for spec in specs {
        s.push_str(&format!(
            "\t\t<ParagraphStyle Self=\"ParagraphStyle/{name}\" Name=\"{name}\" \
             FillColor=\"Color/Black\" FontStyle=\"{fstyle}\" PointSize=\"{size:.1}\" \
             SpaceBefore=\"{before:.1}\">\n\
             \t\t\t<Properties>\n\
             \t\t\t\t<BasedOn type=\"object\">ParagraphStyle/$ID/NormalParagraphStyle</BasedOn>\n\
             \t\t\t\t<AppliedFont type=\"string\">{font}</AppliedFont>\n\
             \t\t\t</Properties>\n\
             \t\t</ParagraphStyle>\n",
            name = spec.style.name(),
            fstyle = xml_escape(&spec.font_style),
            size = spec.size,
            before = spec.space_before,
            font = xml_escape(&spec.font),
        ));
    }
    s.push_str("\t</RootParagraphStyleGroup>\n");
    s.push_str("</idPkg:Styles>\n");
    s
}

fn preferences_xml(page_w: f64, page_h: f64) -> String {
    format!(
        "{decl}<idPkg:Preferences xmlns:idPkg=\"{ns}\" DOMVersion=\"{ver}\">\n\
         \t<DocumentPreference PageHeight=\"{h:.4}\" PageWidth=\"{w:.4}\" PagesPerDocument=\"1\" FacingPages=\"false\" DocumentBleedUniformSize=\"true\" PageBinding=\"LeftToRight\" ColumnDirection=\"Horizontal\" Intent=\"PrintIntent\" />\n\
         \t<MarginPreference ColumnCount=\"1\" ColumnGutter=\"12\" Top=\"{m}\" Bottom=\"{m}\" Left=\"{m}\" Right=\"{m}\" ColumnDirection=\"Horizontal\" />\n\
         </idPkg:Preferences>\n",
        decl = xml_decl(),
        ns = IDPKG_NS,
        ver = DOM_VERSION,
        w = page_w,
        h = page_h,
        m = MARGIN_PT as i64,
    )
}

fn master_spread_xml(master_id: &str, page_w: f64, page_h: f64) -> String {
    format!(
        "{decl}<idPkg:MasterSpread xmlns:idPkg=\"{ns}\" DOMVersion=\"{ver}\">\n\
         \t<MasterSpread Self=\"{mid}\" Name=\"A-Master\" NamePrefix=\"A\" BaseName=\"Master\" ShowMasterItems=\"true\" PageCount=\"1\" ItemTransform=\"1 0 0 1 0 0\">\n\
         \t\t<Page Self=\"{mid}page\" AppliedMaster=\"n\" Name=\"A\" GeometricBounds=\"0 0 {h:.4} {w:.4}\" ItemTransform=\"1 0 0 1 0 0\" UseMasterGrid=\"false\" GridStartingPoint=\"TopOutside\" OverrideList=\"\" TabOrder=\"\" AppliedTrapPreset=\"TrapPreset/$ID/kDefaultTrapStyleName\" AppliedAlternateLayout=\"n\" LayoutRule=\"Off\" OptionalPage=\"false\">\n\
         \t\t\t<MarginPreference ColumnCount=\"1\" ColumnGutter=\"12\" Top=\"{m}\" Bottom=\"{m}\" Left=\"{m}\" Right=\"{m}\" ColumnDirection=\"Horizontal\" />\n\
         \t\t</Page>\n\
         \t</MasterSpread>\n\
         </idPkg:MasterSpread>\n",
        decl = xml_decl(),
        ns = IDPKG_NS,
        ver = DOM_VERSION,
        mid = master_id,
        w = page_w,
        h = page_h,
        m = MARGIN_PT as i64,
    )
}

#[allow(clippy::too_many_arguments)]
fn spread_xml(
    spread_id: &str,
    frame_id: &str,
    master_id: &str,
    layer_id: &str,
    story_id: &str,
    prev_frame: &str,
    next_frame: &str,
    page_number: usize,
    page_w: f64,
    page_h: f64,
) -> String {
    // Frame is the margin box, centered on the page; its path is in frame-local
    // coordinates centered on the origin, with the frame translated to the page
    // center via ItemTransform.
    let half_w = (page_w - 2.0 * MARGIN_PT) / 2.0;
    let half_h = (page_h - 2.0 * MARGIN_PT) / 2.0;
    let cx = page_w / 2.0;
    let cy = page_h / 2.0;

    format!(
        "{decl}<idPkg:Spread xmlns:idPkg=\"{ns}\" DOMVersion=\"{ver}\">\n\
         \t<Spread Self=\"{sid}\" ShowMasterItems=\"true\" PageCount=\"1\" BindingLocation=\"0\" AllowPageShuffle=\"true\" ItemTransform=\"1 0 0 1 0 0\">\n\
         \t\t<Page Self=\"{sid}page\" AppliedMaster=\"{mid}\" Name=\"{pn}\" GeometricBounds=\"0 0 {h:.4} {w:.4}\" ItemTransform=\"1 0 0 1 0 0\" UseMasterGrid=\"false\" GridStartingPoint=\"TopOutside\" OverrideList=\"\" TabOrder=\"\" AppliedTrapPreset=\"TrapPreset/$ID/kDefaultTrapStyleName\" AppliedAlternateLayout=\"n\" LayoutRule=\"UseMaster\" OptionalPage=\"false\">\n\
         \t\t\t<MarginPreference ColumnCount=\"1\" ColumnGutter=\"12\" Top=\"{m}\" Bottom=\"{m}\" Left=\"{m}\" Right=\"{m}\" ColumnDirection=\"Horizontal\" />\n\
         \t\t</Page>\n\
         \t\t<TextFrame Self=\"{fid}\" ParentStory=\"{story}\" PreviousTextFrame=\"{prev}\" NextTextFrame=\"{next}\" ContentType=\"TextType\" Visible=\"true\" Name=\"$ID/\" ItemLayer=\"{layer}\" ItemTransform=\"1 0 0 1 {cx:.4} {cy:.4}\">\n\
         \t\t\t<Properties>\n\
         \t\t\t\t<PathGeometry>\n\
         \t\t\t\t\t<GeometryPathType PathOpen=\"false\">\n\
         \t\t\t\t\t\t<PathPointArray>\n\
         \t\t\t\t\t\t\t<PathPointType Anchor=\"{nx:.4} {ny:.4}\" LeftDirection=\"{nx:.4} {ny:.4}\" RightDirection=\"{nx:.4} {ny:.4}\" />\n\
         \t\t\t\t\t\t\t<PathPointType Anchor=\"{nx:.4} {py:.4}\" LeftDirection=\"{nx:.4} {py:.4}\" RightDirection=\"{nx:.4} {py:.4}\" />\n\
         \t\t\t\t\t\t\t<PathPointType Anchor=\"{px:.4} {py:.4}\" LeftDirection=\"{px:.4} {py:.4}\" RightDirection=\"{px:.4} {py:.4}\" />\n\
         \t\t\t\t\t\t\t<PathPointType Anchor=\"{px:.4} {ny:.4}\" LeftDirection=\"{px:.4} {ny:.4}\" RightDirection=\"{px:.4} {ny:.4}\" />\n\
         \t\t\t\t\t\t</PathPointArray>\n\
         \t\t\t\t\t</GeometryPathType>\n\
         \t\t\t\t</PathGeometry>\n\
         \t\t\t</Properties>\n\
         \t\t\t<TextFramePreference TextColumnCount=\"1\" TextColumnGutter=\"12\" />\n\
         \t\t</TextFrame>\n\
         \t</Spread>\n\
         </idPkg:Spread>\n",
        decl = xml_decl(),
        ns = IDPKG_NS,
        ver = DOM_VERSION,
        sid = spread_id,
        mid = master_id,
        fid = frame_id,
        story = story_id,
        prev = prev_frame,
        next = next_frame,
        layer = layer_id,
        pn = page_number,
        w = page_w,
        h = page_h,
        m = MARGIN_PT as i64,
        cx = cx,
        cy = cy,
        nx = -half_w,
        ny = -half_h,
        px = half_w,
        py = half_h,
    )
}

fn story_xml(story_id: &str, paras: &[Para]) -> String {
    let mut s = format!(
        "{decl}<idPkg:Story xmlns:idPkg=\"{ns}\" DOMVersion=\"{ver}\">\n\
         \t<Story Self=\"{sid}\" AppliedTOCStyle=\"n\" TrackChanges=\"false\" StoryTitle=\"$ID/\" AppliedNamedGrid=\"n\">\n\
         \t\t<StoryPreference OpticalMarginAlignment=\"false\" OpticalMarginSize=\"12\" FrameType=\"TextFrameType\" StoryOrientation=\"Horizontal\" StoryDirection=\"LeftToRightDirection\" />\n\
         \t\t<InCopyExportOption IncludeGraphicProxies=\"true\" IncludeAllResources=\"false\" />\n",
        decl = xml_decl(),
        ns = IDPKG_NS,
        ver = DOM_VERSION,
        sid = story_id
    );

    let last = paras.len().saturating_sub(1);
    for (i, p) in paras.iter().enumerate() {
        // A trailing <Br/> ends each paragraph; the final paragraph omits it.
        let br = if i == last { "" } else { "\n\t\t\t\t<Br />" };
        s.push_str(&format!(
            "\t\t<ParagraphStyleRange AppliedParagraphStyle=\"ParagraphStyle/{style}\">\n\
             \t\t\t<CharacterStyleRange AppliedCharacterStyle=\"CharacterStyle/$ID/[No character style]\">\n\
             \t\t\t\t<Content>{text}</Content>{br}\n\
             \t\t\t</CharacterStyleRange>\n\
             \t\t</ParagraphStyleRange>\n",
            style = p.style.name(),
            text = xml_escape(&p.text),
            br = br,
        ));
    }

    s.push_str("\t</Story>\n</idPkg:Story>\n");
    s
}

fn designmap_xml(
    story_id: &str,
    layer_id: &str,
    master_id: &str,
    spread_ids: &[String],
    title: &str,
) -> String {
    let mut s = String::new();
    s.push_str(xml_decl());
    s.push_str(&format!(
        "<?aid style=\"50\" type=\"document\" readerVersion=\"6.0\" featureSet=\"257\" product=\"{ver}(370)\" ?>\n",
        ver = DOM_VERSION
    ));
    s.push_str(&format!(
        "<Document xmlns:idPkg=\"{ns}\" DOMVersion=\"{ver}\" Self=\"d\" StoryList=\"{story}\" Name=\"{name}\" ZeroPoint=\"0 0\" ActiveLayer=\"{layer}\">\n",
        ns = IDPKG_NS,
        ver = DOM_VERSION,
        story = story_id,
        name = xml_escape(&format!("{title}.indd")),
        layer = layer_id,
    ));
    s.push_str("\t<idPkg:Graphic src=\"Resources/Graphic.xml\" />\n");
    s.push_str("\t<idPkg:Fonts src=\"Resources/Fonts.xml\" />\n");
    s.push_str("\t<idPkg:Styles src=\"Resources/Styles.xml\" />\n");
    s.push_str("\t<idPkg:Preferences src=\"Resources/Preferences.xml\" />\n");
    s.push_str("\t<idPkg:Tags src=\"XML/Tags.xml\" />\n");
    s.push_str(&format!(
        "\t<Layer Self=\"{layer}\" Name=\"Layer 1\" Visible=\"true\" Locked=\"false\" IgnoreWrap=\"false\" ShowGuides=\"true\" LockGuides=\"false\" UI=\"true\" Expendable=\"true\" Printable=\"true\" />\n",
        layer = layer_id
    ));
    s.push_str(&format!(
        "\t<idPkg:MasterSpread src=\"MasterSpreads/MasterSpread_{master_id}.xml\" />\n"
    ));
    for sid in spread_ids {
        s.push_str(&format!(
            "\t<idPkg:Spread src=\"Spreads/Spread_{sid}.xml\" />\n"
        ));
    }
    s.push_str("\t<idPkg:BackingStory src=\"XML/BackingStory.xml\" />\n");
    s.push_str(&format!(
        "\t<idPkg:Story src=\"Stories/Story_{story_id}.xml\" />\n"
    ));
    s.push_str("</Document>\n");
    s
}

/// Parse a `#rrggbb` (or `rrggbb`) hex color into `(r, g, b)`.
fn parse_hex_rgb(s: &str) -> Option<(u8, u8, u8)> {
    let h = s.trim().strip_prefix('#').unwrap_or(s.trim());
    if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()?;
    let g = u8::from_str_radix(&h[2..4], 16).ok()?;
    let b = u8::from_str_radix(&h[4..6], 16).ok()?;
    Some((r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ContentMode, PaperSize, TimeSplit};
    use crate::model::{Meta, Panel, Room, ScheduleData};
    use std::io::Read;

    fn schedule() -> ScheduleData {
        ScheduleData {
            meta: Meta {
                title: "Test & Demo".into(),
                ..Meta::default()
            },
            panels: vec![
                Panel {
                    id: "P1".into(),
                    name: "Opening <Ceremony>".into(),
                    room_ids: vec![1],
                    // 2026-06-26 09:00/10:00 UTC (empty meta timezone).
                    start_epoch: Some(1_782_810_000),
                    end_epoch: Some(1_782_813_600),
                    presenters: vec!["Ada".into()],
                    description: Some("Kick things off.".into()),
                    ..Panel::default()
                },
                Panel {
                    id: "P2".into(),
                    name: "Cosplay 101".into(),
                    room_ids: vec![2],
                    // 2026-06-27 14:00/15:00 UTC (empty meta timezone).
                    start_epoch: Some(1_782_914_400),
                    end_epoch: Some(1_782_918_000),
                    presenters: vec![],
                    ..Panel::default()
                },
            ],
            rooms: vec![
                Room {
                    uid: 1,
                    short_name: "A".into(),
                    long_name: "Salon A".into(),
                    sort_key: 0,
                    ..Room::default()
                },
                Room {
                    uid: 2,
                    short_name: "B".into(),
                    long_name: "Salon B".into(),
                    sort_key: 1,
                    ..Room::default()
                },
            ],
            panel_types: Default::default(),
            timeline: vec![],
            presenters: vec![],
        }
    }

    fn cfg(content: ContentMode) -> LayoutConfig {
        LayoutConfig {
            content,
            paper: PaperSize::Letter,
            ..LayoutConfig::default()
        }
    }

    /// Extract a part's text from the generated package.
    fn read_part(bytes: &[u8], name: &str) -> String {
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut f = zip.by_name(name).unwrap();
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();
        s
    }

    /// Lightweight well-formedness scan: declaration present and `<`/`>` balanced.
    fn well_formed(xml: &str) -> bool {
        xml.starts_with("<?xml")
            && xml.matches('<').count() == xml.matches('>').count()
            && xml.matches('<').count() > 0
    }

    #[test]
    fn test_empty_schedule_errors() {
        let mut data = schedule();
        data.panels.clear();
        let err = generate_idml(&data, &BrandConfig::default(), &LayoutConfig::default());
        assert!(matches!(err, Err(IdmlError::Empty)));
    }

    #[test]
    fn test_package_structure() {
        let data = schedule();
        let bytes = generate_idml(
            &data,
            &BrandConfig::default(),
            &cfg(ContentMode::DescriptionOnly {
                section: None,
                time: None,
            }),
        )
        .unwrap();
        assert!(!bytes.is_empty());

        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();

        // mimetype must be the first entry, stored (uncompressed), exact content.
        let first = zip.by_index(0).unwrap();
        assert_eq!(first.name(), "mimetype");
        assert_eq!(first.compression(), CompressionMethod::Stored);
        assert_eq!(first.size(), 43);
        drop(first);

        let names: Vec<String> = (0..zip.len())
            .map(|i| zip.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(names.contains(&"designmap.xml".to_string()));
        assert!(names.contains(&"META-INF/container.xml".to_string()));
        assert!(names.contains(&"Resources/Styles.xml".to_string()));
        assert!(names.iter().any(|n| n.starts_with("Stories/Story_")));
        assert!(names.iter().any(|n| n.starts_with("Spreads/Spread_")));
        assert!(names.iter().any(|n| n.starts_with("MasterSpreads/")));
    }

    #[test]
    fn test_story_contains_escaped_content() {
        let data = schedule();
        let bytes = generate_idml(
            &data,
            &BrandConfig::default(),
            &cfg(ContentMode::DescriptionOnly {
                section: None,
                time: None,
            }),
        )
        .unwrap();
        let story = read_part(&bytes, "Stories/Story_story1.xml");
        // Panel name with angle brackets must be escaped, not raw.
        assert!(story.contains("Opening &lt;Ceremony&gt;"));
        assert!(!story.contains("Opening <Ceremony>"));
        // Day heading and a presenter meta line are present.
        assert!(story.contains("ParagraphStyle/Day"));
        assert!(story.contains("Ada"));
        // Description body present in full mode.
        assert!(story.contains("Kick things off."));
    }

    #[test]
    fn test_panel_list_is_compact() {
        let data = schedule();
        let bytes = generate_idml(
            &data,
            &BrandConfig::default(),
            &cfg(ContentMode::PanelList {
                section: None,
                time: None,
            }),
        )
        .unwrap();
        let story = read_part(&bytes, "Stories/Story_story1.xml");
        // Compact mode omits presenters and descriptions.
        assert!(!story.contains("Ada"));
        assert!(!story.contains("Kick things off."));
    }

    #[test]
    fn test_all_parts_well_formed() {
        let data = schedule();
        let bytes = generate_idml(
            &data,
            &BrandConfig::default(),
            &cfg(ContentMode::Both {
                section: None,
                time: TimeSplit::Day,
            }),
        )
        .unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();
        for i in 0..zip.len() {
            let mut f = zip.by_index(i).unwrap();
            let name = f.name().to_string();
            if name == "mimetype" {
                continue;
            }
            let mut s = String::new();
            f.read_to_string(&mut s).unwrap();
            assert!(well_formed(&s), "part {name} not well-formed");
        }
    }

    #[test]
    fn test_deterministic_output() {
        let data = schedule();
        let c = cfg(ContentMode::DescriptionOnly {
            section: None,
            time: None,
        });
        let a = generate_idml(&data, &BrandConfig::default(), &c).unwrap();
        let b = generate_idml(&data, &BrandConfig::default(), &c).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn test_indesign_font_style_mapping() {
        assert_eq!(indesign_font_style(Some("200"), None), "Light");
        assert_eq!(indesign_font_style(Some("700"), None), "Bold");
        assert_eq!(indesign_font_style(Some("400"), Some("italic")), "Italic");
        assert_eq!(
            indesign_font_style(Some("700"), Some("italic")),
            "Bold Italic"
        );
        assert_eq!(indesign_font_style(None, None), "Regular");
        // A non-numeric weight passes through capitalized (font's own style name).
        assert_eq!(indesign_font_style(Some("One"), None), "One");
    }

    #[test]
    fn test_explicit_idml_style_overrides_weight() {
        let mut brand = BrandConfig::default();
        brand.fonts.heading = Some("Trend Sans".into());
        brand.fonts.heading_weight = Some("200".into()); // would map to "Light"
        brand.fonts.heading_idml_style = Some("One".into());
        let specs = style_specs(&brand, 9.0);
        let day = specs.iter().find(|s| s.style.name() == "Day").unwrap();
        assert_eq!(day.font, "Trend Sans");
        assert_eq!(day.font_style, "One"); // explicit override wins
    }

    #[test]
    fn test_fonts_xml_matches_requested_styles() {
        let mut brand = BrandConfig::default();
        brand.fonts.heading = Some("Trend Sans".into());
        brand.fonts.heading_idml_style = Some("One".into());
        let bytes = generate_idml(
            &schedule(),
            &brand,
            &cfg(ContentMode::DescriptionOnly {
                section: None,
                time: None,
            }),
        )
        .unwrap();
        let fonts = read_part(&bytes, "Resources/Fonts.xml");
        let styles = read_part(&bytes, "Resources/Styles.xml");
        // The heading style requested in Styles.xml is advertised in Fonts.xml.
        assert!(styles.contains("FontStyle=\"One\""));
        assert!(fonts.contains("FontStyleName=\"One\""));
        assert!(fonts.contains("Name=\"Trend Sans One\""));
        // No fabricated "Bold" for a font that has no such style.
        assert!(!styles.contains("FontStyle=\"Bold\""));
    }

    #[test]
    fn test_parse_hex_rgb() {
        assert_eq!(parse_hex_rgb("#00BCDD"), Some((0, 188, 221)));
        assert_eq!(parse_hex_rgb("ffffff"), Some((255, 255, 255)));
        assert_eq!(parse_hex_rgb("nope"), None);
    }
}
