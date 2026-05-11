# FEATURE-106: Add crates/schedule-layout shared crate

## Summary

New shared Rust crate providing layout engine, brand config, Typst codegen, and in-process PDF compilation for print output formats.

## Status

Open

## Priority

High

## Blocked By

- FEATURE-105: Widget print work can proceed in parallel, but crate is needed before FEATURE-107

## Description

Create `crates/schedule-layout` as a workspace member. This crate is reusable by `cosam-layout`, future `cosam-editor` WYSIWYG grid calculations, and any other tool needing layout logic.

## Implementation Details

### Modules

- `model` — wraps widget JSON types from `schedule-core`; adds half-day splits, per-room and per-guest filtered views
- `grid` — time-grid layout computation: time slots, room columns, event cell spans, row heights (kept stable/`pub` for editor reuse)
- `brand` — `BrandConfig` struct; loads `config/brand.toml`; `BrandConfig::sample()` for `--dump-sample-brand`
- `color` — `ColorMode` enum (`Color`/`Bw`); grayscale derivation via ITU-R BT.601 luminance when `colors.bw` is absent
- `typst_gen` — Typst `.typ` source generation; panel-type colors resolved at build time
- `compile` — wraps `typst` Rust crate for in-process PDF compilation; feature-gated behind `compile` feature flag
- `formats/` — one submodule per output format: `schedule`, `workshop_poster`, `room_signs`, `guest_postcards`, `descriptions`

### Key types

```rust
pub struct LayoutConfig {
    pub paper: PaperSize,
    pub format: LayoutFormat,
    pub split_by: SplitMode,
    pub color_mode: ColorMode,
    pub filter: LayoutFilter,
}
pub enum PaperSize { Legal, Tabloid, SuperB, Postcard4x6 }
pub enum LayoutFormat { Schedule, WorkshopPoster, RoomSigns, GuestPostcards, Descriptions }
pub enum ColorMode { Color, Bw }
```

### BrandConfig (config/brand.toml)

- `BrandColors`: primary, black, dark_grey, white
- `BrandFonts`: heading, subheading, body (optional paths)
- `BrandMeta`: name, site_url, logo_path (optional)
- Resolved relative to config file directory
- SVG logo preferred; PNG accepted

### Dependencies

- `serde`, `serde_json` — deserialize schedule.json
- `toml` — load brand.toml
- `typst` — in-process compilation (`compile` feature)
- `qrcode` + `image` — QR codes for workshop-poster format
- `thiserror` — library errors

### .gitignore / config/

- Add `/config/` and `!/config/brand.sample.toml` to root `.gitignore`
- Commit `config/brand.sample.toml` with placeholder values and font path documentation

## Acceptance Criteria

- `cargo test -p schedule-layout` passes
- `BrandConfig::load()` reads TOML; missing fields use defaults
- `BrandConfig::sample()` returns defaults matching `brand.sample.toml`
- Grayscale derivation: `#E2F9D7` → ~`#E8E8E8`
- `compile` feature gate compiles with and without `typst` dependency
