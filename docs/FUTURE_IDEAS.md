# Future Ideas and Design Notes

Updated on: Sat Apr 11 20:04:17 2026

Open design questions, unexplored alternatives, and deferred ideas.
An IDEA item can be promoted to a work item by renaming it to another prefix
(e.g. `IDEA-033.md` → `REFACTOR-033.md`) while keeping the same number.

## Open Ideas

### [IDEA-033] DirectedEdge: endpoint_uuids() tuple accessor and #[endpoint] attribute rename

**Summary:** Deferred design idea: add an `endpoint_uuids()` tuple method to `DirectedEdge`
and optionally rename `#[edge_from]`/`#[edge_to]` to `#[endpoint]`.

**Description:** After renaming `from`/`to` → `left`/`right` on `DirectedEdge` (REFACTOR-032),
two further refinements were considered but deferred:

---

### [IDEA-039] Per-Membership Edge Flags (always_grouped / always_shown_in_group)

**Summary:** Explore restoring per-membership granularity for `always_grouped` and
`always_shown_in_group` if entity-level flags prove insufficient.

**Description:** Currently `always_grouped` and `always_shown_in_group` are entity-level fields
on `Presenter`, meaning they apply to **all** of a presenter's group memberships
equally.  This matches the old `schedule-to-html` Perl implementation behavior.

The old `PresenterToGroup` edge stored these as per-edge flags, allowing a
presenter to be `always_grouped` with respect to Group A but not Group B.  This
distinction was not actually used in the spreadsheet data, but the model
supported it.

---

## Next Available IDs

IDs are shared with the main work item pool.
Rename `IDEA-###.md` to another prefix to promote an idea.

**Available:** 040, 041, 042, 043, 044, 045, 046, 047, 048, 049

**Highest used:** 39

---

[IDEA-033]: work-item/idea/IDEA-033.md
[IDEA-039]: work-item/idea/IDEA-039.md
