---
description: Testing print layouts (schedule-layout / Typst → PDF)
trigger: glob
globs: crates/schedule-layout/**/*.rs,apps/cosam-layout/**/*.rs,config/layout*.toml
---

# Print Layout Testing

The `schedule-layout` crate turns a schedule into Typst `.typ` source, compiled
to PDF. One configurable builder (`document::generate`) produces every artifact;
see `docs/layout-formats.md` for the content/split model.

Requires the `typst` binary on `PATH` to compile PDFs.

## Quick start

Drive the builder through `cosam-convert` with a layout-config TOML. Put
throwaway job configs **and all output under `scratch/`** (gitignored) — never
`output/`.

```bash
cargo run --release -p cosam-convert -- \
  --input "input/2026 Schedule.xlsx" \
  --layout-config scratch/layout-modes.toml \
  --stable-timestamps \
  --export-layout scratch/layout-after
```

- Build `--release`: `cosam-convert` is very slow in debug.
- `--stable-timestamps` pins the generated time to the (stable) modified time so
  the footer — and the rendered output — is reproducible across runs. Settings
  must precede `--export-layout`.
- `--export-layout <dir>` writes PDFs into per-paper-size subdirs and all `.typ`
  under `<dir>/typ/`. The `.typ` is written even if `typst` is missing.
- A scratch job TOML mirrors `config/layout-default.toml`: one `[[jobs]]` per
  mode (`content` + `split` + `paper`). `scratch/layout-modes.toml` exercises
  every content mode (both / grid_only / description_only / panel_list).

## Verifying a change preserves output

PDFs embed a creation timestamp, so compare **rendered PNGs**, not PDFs. Render
each `.typ` to per-page PNGs and diff page hashes against a baseline render:

```bash
for t in scratch/layout-after/typ/*.typ; do
  stem=$(basename "$t" .typ)
  typst compile --root / "$t" "scratch/render-after/${stem}-{0p}.png"
done
# shasum each page vs scratch/render-baseline/*.png — identical ⇒ same rendering
```

Identical hashes mean a refactor changed the `.typ` text (e.g. new `#let`
variables) but not the rendering. Capture the baseline from the pre-change build
*before* editing. If a footer timestamp leaks in, normalize it (or use
`--stable-timestamps` on both runs).
