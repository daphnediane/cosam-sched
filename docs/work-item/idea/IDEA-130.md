# IDEA-130: Collaborative undo via inverse writes past sync horizon

## Summary

When a local undo crosses the last save/sync point, write inverse changes as new
automerge ops instead of using `fork_at`, so the undo propagates to peers on merge.

## Status

Open

## Priority

Low

## Description

### Background

The current undo system (FEATURE-129) uses `Schedule::fork_at_heads(pre_heads)` to
implement undo. This is a local document fork: the automerge DAG is not modified —
instead a new `AutoCommit` is created that only knows the subset of changes up to
`pre_heads`. Because no new changes are added to the DAG, peers never observe the undo
when they merge.

This is acceptable within a single editing session (the user undoes, then saves, and
the saved file reflects the undone state), but it breaks down once a schedule has been
shared: if Alice syncs her changes to Bob and then undoes, Bob will get Alice's original
changes on the next merge, not the undone result.

### Proposed Approach

`EditContext` already tracks `clean_heads` (set by `mark_clean()` at save/sync time).
When an undo is requested and `entry.pre_heads` is a strict ancestor of `clean_heads`
(i.e. the undo would cross the sync horizon), use inverse writes instead of `fork_at`:

1. Compute the diff between the current document state and the target `pre_heads`
   state using `doc.diff(current_heads, pre_heads)` or the automerge `*_at()` read
   APIs to inspect each changed path at the target snapshot.
2. For each changed field, write the `pre_heads` value as a new automerge operation
   using the normal write path (LWW put, text splice, list replace, etc.).
3. The inverse writes are new entries in the CRDT DAG — they propagate to peers via
   normal sync and arrive as legitimate last-write-wins overwrites.

For undos that stay within the unsaved session (i.e. `pre_heads` is a descendant of
`clean_heads`), `fork_at` remains the right tool: it is cheaper, exact, and the user
has not yet shared those changes.

### Hybrid Strategy

```text
            clean_heads        current_heads
                |                    |
 ... ──────────[S]────[A]────[B]────[C]
                 ↑
                last save/sync

undo to [A]: pre_heads=[A] is descendant of clean_heads → use fork_at (local, fast)
undo to [*]: pre_heads before [S] → use inverse writes (new ops in DAG, visible to peers)
```

### Open Questions

- The `diff()` / `*_at()` automerge APIs expose low-level `Prop` paths, not typed
  `FieldDescriptor` values. A mapping layer is needed to translate automerge diffs
  back to schedule write calls. Complexity is non-trivial.
- List fields (edges, `List` CRDT fields): inverse writes would replace the list
  contents, which is LWW-on-the-whole-list rather than add-wins. Concurrent adds
  from other peers during the window between the original write and the inverse write
  would be lost. Whether this is acceptable depends on the use case.
- Text fields (RGA): inverse writes would replace the full text, losing concurrent
  character-level edits from peers. Same trade-off as list fields.
- Should the undo label in the menu indicate whether the undo will be collaborative
  (visible to peers) or local only?
- Does "past the sync horizon" need a more nuanced definition when multiple sync
  targets exist (e.g. synced to Bob but not Carol)?

### Relationship to Existing Work

- Depends on FEATURE-129 (heads-based undo/redo infrastructure, completed).
- The `clean_heads` field already exists in `EditContext`; the horizon check is
  one comparison away.
- FEATURE-099 (undo history persistence) should be designed with this in mind —
  persisted undo entries that cross the sync horizon would need to know which undo
  strategy to use on replay.
