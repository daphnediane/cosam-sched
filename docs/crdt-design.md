# CRDT Design for Offline Collaborative Editing

**Status:** Spike findings (META-027 Step 2)  
**Spike crate:** `crates/crdt-spike`  
**Related work items:** META-027, FEATURE-011, FEATURE-012, FEATURE-013

---

## Problem Statement

The scheduling application is used by multiple operators, sometimes offline
(e.g., at the convention itself where network access is unreliable).  When two
operators edit the same entity and later sync, changes must merge without
silent data loss.

A simple "last writer wins at the whole-entity level" model is unacceptable
because unrelated edits (different fields, or different parts of the same prose
field) would be silently discarded.

---

## Field Classification

Not all fields need the same CRDT type.  This classification drives the design:

### Structured scalar fields

Single values where the most-recent write is authoritative:

| Field | Examples |
|---|---|
| String scalars | `name`, `rank`, `badge_name`, `badge_name_short` |
| Integer scalars | `duration`, `min_age`, `max_age` |
| Boolean scalars | `invite_only`, `gophers_only`, `at_capacity` |
| UUID references | `event_room_id`, `panel_type_id` |
| Timestamp scalars | `start_time`, `end_time` |

**CRDT type:** `LWWReg<V, (u64, ActorId)>` from the `crdts` crate.

The marker is a `(logical_time, actor_id)` pair.  Concurrent writes at the same
logical time are deterministically broken by actor ID (higher wins).  This is
acceptable for scalar fields because the operators, or the sync protocol, are
responsible for advancing the logical clock.

### Relationship set fields

Ordered or unordered collections of UUID references:

| Field | Examples |
|---|---|
| Presenter sets | `presenter_ids` on Panel |
| Room assignments | `event_room_ids` on Panel |
| Tags/types | (future) |

**CRDT type:** `Orswot<Uuid, ActorId>` from the `crdts` crate.

OR-Set semantics: concurrent adds both survive (set union); a remove only
cancels tokens the removing actor has already observed — an unobserved
concurrent add wins over the remove.  This matches user intuition: if two
operators independently add different presenters to a panel, both additions
should survive the merge.

### Prose / long-text fields

Fields where multiple operators may edit different parts of the same text:

| Field | Notes |
|---|---|
| `description` | Most commonly edited; copy-paste from prior years is common |
| `note` | Shared notes; rare concurrent writes but possible |
| `notes_non_printing` | Same as note |
| `workshop_notes` | Typically one author, but consistency matters |
| `av_notes` | Same as workshop_notes |

**CRDT type:** `automerge::Text` (RGA — Replicated Growable Array).

**Why LWW is insufficient for prose:**  
A find-replace operation on a guest name (e.g., "Cosplay Ant" → "Cosplay Aunt"
across all descriptions) touches the start of the text.  A concurrent operator
editing the end of the same description to remove stale computed data would lose
their change under LWW.  RGA merges at character granularity: edits to
non-overlapping positions both survive; simultaneous inserts at the same
position receive a deterministic total order (no data loss).

---

## Library Evaluation

### `crdts` crate (v7.3.2)

**Evaluated types:** `LWWReg`, `Orswot`, `Map`

**Strengths:**
- Lightweight; no async runtime dependency
- `LWWReg` is a simple struct — trivially serialisable, embeddable in JSON
- `Orswot` implements OR-Set semantics correctly (add-wins vs unobserved remove)
- `Map` supports composing nested CRDTs (e.g., entity fields as a CRDT map)
- All types implement `CvRDT` (state-based merge) and `CmRDT` (op-based apply)

**Limitations:**
- No character-level text CRDT — `LWWReg<String, _>` discards entire concurrent
  prose edits
- `LWWReg` marker is caller-managed (monotonic counter or `(time, actor)` tuple)
  — the application must maintain logical clocks per field per actor
- No built-in serialisation feature flag (`features = ["serde"]` does not exist;
  the crate unconditionally depends on `serde`)

**Spike results:** 7 scenarios pass (structured_fields.rs):
- Two actors create entities with different UUIDs → both survive merge
- Non-overlapping field edits → both preserved
- Concurrent scalar edits → LWW converges (actor-ID tiebreak)
- Concurrent set adds → both survive (OR-Set union)
- Unobserved concurrent remove → add wins
- Idempotent merge (X ∪ X = X)
- Remove-entity vs concurrent field-edit → OR-Set presence decides

### `automerge` crate (v0.5.12)

**Evaluated type:** `AutoCommit` document with `ObjType::Text`

**Strengths:**
- Text CRDT (RGA) handles character-level concurrent edits correctly
- `fork()` / `merge()` API is ergonomic for offline-then-sync workflows
- Full document model: scalars, maps, lists, and text are all composable
- Active development; well-tested in production

**Limitations:**
- Heavier dependency than `crdts`; binary size is larger
- The document model is holistic — integrating it field-by-field into the
  existing `EntityStorage` / field system requires an adapter layer
- Logical clock and actor identity are internal to the document — the
  application cannot directly inspect them for debugging

**Spike results:** 5 scenarios pass (prose_fields.rs):
- Edits at different character positions both survive
- Global find-replace + concurrent description edit both survive
- Concurrent inserts at the same position: deterministic order, no data loss
- Idempotent merge (X ∪ X = X)
- Trim stale computed data + concurrent name fix both survive

---

## Recommendation

Use a **two-library approach** matching field classification to CRDT type:

| Field category | Library | Type |
|---|---|---|
| Structured scalars | `crdts` | `LWWReg<V, (u64, ActorId)>` |
| Relationship sets | `crdts` | `Orswot<Uuid, ActorId>` |
| Prose / long text | `automerge` | Document `Text` object |

For the initial implementation (FEATURE-011 through FEATURE-013), a practical
split is:

1. **Phase 1:** All fields use `LWWReg` / `Orswot` from `crdts` — covers the
   majority of fields and unblocks offline sync for structured data.
2. **Phase 2:** Prose fields (`description`, `note`, etc.) are stored as
   `automerge` Text objects, serialised alongside the structured CRDT state.

This phased approach keeps scope manageable and lets the sync protocol be
designed around `crdts` first before adding the `automerge` document layer.

---

## Open Questions

1. **Actor identity** — How are actor IDs assigned and persisted?  Per-device?
   Per-user?  The spike uses `u64` constants; production needs a stable,
   globally unique ID (UUID or device fingerprint).

2. **Logical clock management** — `LWWReg` requires a monotonic marker.  The
   application must maintain a per-actor, per-field logical clock, or use a
   hybrid logical clock (HLC) for wall-clock-aligned ordering.

3. **Serialisation format** — Both libraries support `serde`.  The sync wire
   format needs to be defined (full state merge vs op log replay).  `crdts`
   state can be serialised to JSON; `automerge` has its own binary format with
   optional `serde_json` round-trip.

4. **Prose field integration** — The existing field system (`FieldDef`,
   `FieldValue`) has no `Text` variant.  Either add `FieldValue::Text(String)`
   as the read-only view while CRDT state is stored separately, or extend
   `FieldValue` with an opaque CRDT handle.

5. **Conflict notification** — LWW silently picks a winner.  Should the UI
   surface scalar conflicts to the user (e.g., two operators set different start
   times)?  The `crdts` `MVReg` (Multi-Value Register) can preserve all
   concurrent values for user review; this may be preferable for high-stakes
   fields like `start_time`.

6. **`crdts::Map` vs flat HashMap** — The spike models entity fields as a
   `HashMap<String, LWWReg>`.  The `crdts::Map` type supports reset-remove and
   observed-remove semantics for nested values, which may simplify entity
   deletion semantics.  Worth a follow-up spike for entity lifecycle.
