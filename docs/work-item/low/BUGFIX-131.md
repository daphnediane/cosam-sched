# BUGFIX-131: PanelUniqId::parse silently truncates prefixes longer than 2 chars

## Summary

`PanelUniqId::parse("SPLIT001")` normalizes the prefix to `"SP"` and returns
`full_id()` = `"SP001"`, discarding the original `"SPLIT001"` string. The raw
form typed in the spreadsheet should be preserved.

## Status

Open

## Priority

Low

## Blocked By (optional)

## Description

`PanelUniqId::parse` normalizes any prefix longer than 2 characters to its
first two letters (e.g. `"SPLIT"` → `"SP"`, `"BREAK"` → `"BR"`). This means
`full_id()` never returns the original string — `"SPLIT001"` becomes `"SP001"`.

Two consequences:

1. The panel/timeline is stored and displayed under the shorter code, which may
   confuse coordinators who typed the long form.
2. Any lookup or export that compares against the raw XLSX value will disagree
   with what is stored (worked around in FEATURE-127 by normalizing the upsert
   key via `full_id()`, but the display value is still wrong).

The correct behavior: the stored `code` should round-trip to whatever normalized
form the parser produces, AND `full_id()` should reflect that canonical form.
The question is whether `"SPLIT001"` should be stored as `"SP001"` (current) or
`"SPLIT001"` (raw). Given that the prefix drives panel-type lookup (by 2-char
prefix), the normalized 2-char form is probably right — but it should at least
be surfaced as a warning/note rather than silently discarded.

## How Found

During FEATURE-127 idempotency debugging: `find_by_code("SPLIT001")` always
returned empty because stored `full_id()` = `"SP001"` never matched the raw
upsert key `"SPLIT001"`. Traced via `eprintln!` debug output.

## Reproduction

```rust
let id = PanelUniqId::parse("SPLIT001").unwrap();
assert_eq!(id.full_id(), "SPLIT001"); // fails — returns "SP001"
```

**Expected:** `full_id()` returns `"SP001"` (normalized canonical form) and
the import logs a note that `"SPLIT001"` was normalized to `"SP001"`.

**Actual:** Silently truncates; no warning; round-trip from raw XLSX value is
lost.

## Steps to Fix

Option A (minimal): emit a warning/log entry when a prefix is truncated during
parse, so coordinators know their code was normalized.

Option B (preserve raw): store both the raw string and the parsed form;
`full_id()` returns the normalized form, but the original is available for
display or round-trip export.

Option C (reject long prefixes): return `None` from `parse` for prefixes longer
than 2 chars, requiring the spreadsheet to use correct 2-char prefixes.

Option A is the lowest-risk fix; Option C enforces data quality at import time.

## Testing

- Unit test: `PanelUniqId::parse("SPLIT001")` → `full_id()` = `"SP001"`,
  warning emitted (Option A) or `None` returned (Option C).
- Integration: re-import with a `"SPLIT001"` code row; verify idempotency and
  that the normalized form is stable across imports.
