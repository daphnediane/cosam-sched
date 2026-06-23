# Page Geometry (`schedule-layout`)

How `schedule-layout` reserves the page margins and places the banner and footer.
This is the conceptual map; the **canonical values live in the source** and are
referenced here so they cannot drift:

- `crates/schedule-layout/src/geometry.rs` — every dimensional constant (banner
  height, footer height, gaps, insets, logo height) and the `typst_lets` emitter
  that turns them into preamble `#let`s. Read the module rustdoc first.
- `crates/schedule-layout/src/blocks/banner.rs` — the banner bar and the three
  footer variants (`footer_context` does the vertical placement).
- `crates/schedule-layout/src/document.rs` — the `#set page(margin, …)` directive
  that ties the constants to the page.
- `crates/schedule-layout/src/fonts.rs` — banner/footer text-size defaults.

## The page model

Every dimensional literal is emitted once as a preamble `#let` (e.g. `_page-edge`,
`_banner-height`, `_footer-bottom`) and referenced by name, so the page setup and
the generators always agree. Margins are:

```text
top    = _content-top = _page-edge + _banner-height + _banner-gap
bottom = _footer-bottom        (reserved strip for the footer)
left   = right = _page-edge
```

The top margin is **not** a free value — it is exactly the page edge plus the
banner bar plus the gap to the body. Change the banner height and the body moves
with it.

## Banner

The banner is a **fixed-height block** (`_banner-height`) filled with the brand
color, with its content vertically centered. It used to be content-driven (the bar
grew to fit its text), so the reserved margin and the visible bar disagreed; making
it a fixed height keeps them in sync. The logo height is emitted as
`calc.min(<nominal>, _banner-height - 2·inset)` so it always fits the bar.

## Footer

The footer occupies the reserved bottom margin (`_footer-bottom`) and is built by
`footer_context`:

- The horizontal rule sits `_footer-line-gap` below the body bottom (its
  historical position).
- The text is **vertically centered** in the space below the rule (an inner
  `1fr`-height block with `align(horizon)`), with a minimum `_footer-rule-gap`
  below the rule.
- `_footer-descent` is reserved as a real **bottom margin** below the text.

### Gotcha: `footer-descent` is a body gap, not an edge gap

Typst's page `footer-descent` is the gap between the **body and the footer top**,
not between the footer and the page edge. So the page directive pins
`footer-descent: 0pt` (footer starts at the body bottom) and the footer block
reserves its own bottom margin via its height (`_footer-bottom - _footer-descent`,
bottom-aligned). Setting a non-zero page `footer-descent` pushes the whole footer
down until the text overruns the page edge — that is the bug this structure avoids.

Likewise, a trailing `v(1fr)` is collapsed at the end of a block, so the footer
centers with an explicit sized inner block rather than a trailing flexible spacer.

## Chrome sizing (compact / full / explicit)

Banner and footer heights are selected **independently** per job by `banner_size`
and `footer_size` (see [layout-formats.md](layout-formats.md)):

- `auto` — compact on [`PaperSize::is_compact`] papers (4×6 postcard, quarter
  letter), full-size otherwise. Compact uses proportionally thinner bars and
  smaller banner/footer text defaults so the chrome stays in scale on a ~6in page.
- `compact` / `full` — force either set regardless of paper.
- a length (`0.5in`) or a `%` of page height (`4%`) — an explicit bar height;
  percentages resolve against the page height in the current orientation.

A job can therefore opt a small-paper output back into the large banner (e.g.
guest-itinerary postcards use `banner_size = "full"`).

## Header room names

Grid room-header short names shrink to stay on a single line: each name is measured
at the nominal header size against the column width the layout offers and scaled
down by the overflow ratio when needed (`blocks/grid.rs::fit_header_name`). Names
that already fit render unchanged. The optional hotel-room line stays in the same
paragraph (a `\` linebreak) so the name→hotel spacing is the tight leading, not
block spacing.

[`PaperSize::is_compact`]: ../crates/schedule-layout/src/config.rs
