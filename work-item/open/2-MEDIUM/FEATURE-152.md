# FEATURE-152: WASM Typst PDF export plugin for the widget

## Summary

Compile `schedule-layout` + an in-process Typst engine to WebAssembly and expose
it as a lazy-loaded widget plugin, so the widget can produce the real Typst
house-style PDF in the browser without paying the wasm cost during normal use.

## Status

Open

## Priority

Medium

## Description

The widget's browser-print path (FEATURE-151) reimplements the Typst house style
in CSS and can only approximate it — pagination, running footers with page
numbers, and full panel metadata are hard or impossible in native
`window.print()`. The authoritative layout lives in `schedule-layout`
(`document::generate` → Typst `.typ`), today compiled to PDF only by shelling out
to the `typst` binary in `cosam-convert`.

This feature brings that real pipeline into the browser: a WebAssembly module
that reuses `schedule-layout`'s `.typ` generation and then compiles it to a PDF
**in-process** via the `typst` crate (the same engine the CLI uses, no external
binary). The widget gets a "Download Typst PDF" action that produces the
house-style output, not a CSS approximation.

Because the wasm blob is large (the Typst engine + std), it must **not** load
during normal widget usage. It ships as a separate, lazy-loaded plugin file
(mirroring the existing opt-in widget loaders) that imports the wasm only when
the user invokes the PDF action.

This is explicitly a **rough first pass**: prove the pipeline end-to-end
(schedule JSON in → house-style PDF out, in the browser), accept known fidelity
gaps, and iterate afterward.

## Goals

- A wasm module that reuses `schedule-layout`'s pure `.typ` generation and
  compiles it to PDF bytes in-process, with no external `typst` binary.
- A lazy-loaded widget plugin that pulls in the wasm only on demand, so core
  widget usage loads none of it.
- Reuse the FEATURE-151 brand web-font mapping: the plugin fetches the Google
  Font files at runtime and feeds the bytes to the in-process engine, keeping
  font binaries out of the wasm blob.
- The heavy Typst dependency tree stays out of the normal `cargo build` /
  `cargo test` path (e.g. a workspace-excluded crate) so normal usage pays
  nothing at the toolchain level.
- A user action in the widget downloads a real Typst house-style PDF.

## First-pass scope and known gaps

- No virtual filesystem in the wasm world, so logos are suppressed in v1.
- Font-family fallback is accepted if a requested family name doesn't match a
  loaded face (the PDF still renders).
- Timestamps come from the schedule data, rendered deterministically.

Both the logo suppression and font-matching are fidelity follow-ups, not v1
blockers. (Implementation specifics — crate name, the wasm-bindgen entry point,
the Typst `World` wiring, and dependency versions — are deferred to the
implementation branch.)

## Acceptance Criteria

- [ ] The wasm module turns a widget schedule JSON + config into PDF bytes
      in-process (no external `typst` binary), reusing `schedule-layout`'s `.typ`
      generation.
- [ ] The Typst dependency tree is excluded from the normal workspace build.
- [ ] The plugin lazy-loads the wasm only on demand; core widget usage loads
      none of it.
- [ ] The plugin fetches brand Google web-font bytes and feeds them to the
      engine.
- [ ] A user action in the widget downloads a real Typst house-style PDF.

## Notes

- Risk areas: the Typst `World` API surface on `wasm32-unknown-unknown`, wasm
  blob size, and font-family matching. The `.typ` generation path is pure/IO-free
  and ports trivially.
- Relationship to FEATURE-151: that feature is the CSS-print approximation; this
  one is the authoritative in-browser PDF. They coexist — print for quick output,
  Typst PDF for fidelity.
- Build/test prerequisites absent from some environments: network (to fetch the
  Typst crate tree on first build) and the `wasm32-unknown-unknown` target.

## Blocked By

- None (the FEATURE-151 brand/web-font bridge already provides the font mapping
  this reuses).
