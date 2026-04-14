/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Spike: structured field CRDT scenarios using the `crdts` crate.
//!
//! Evaluates `LWWReg` (Last-Write-Wins register) for scalar fields and
//! `Orswot` (Observed-Remove Set Without Tombstones) for relationship fields,
//! against six scenarios from the META-027 spike plan.
//!
//! ## crdts 7.x API notes
//!
//! - `LWWReg<V, M>` is a bare struct `{ val: V, marker: M }`.
//!   The marker is the caller's responsibility to keep monotonic.
//!   We use `(logical_time: u64, actor: ActorId)` so that concurrent writes
//!   at the same logical time are broken by actor ID.
//! - `Orswot<M, A>` (note order: element type first, actor type second)
//!   uses `add(member, AddCtx)` / `rm(member, RmCtx)` / `read()`.
//!   `iter()` returns `ReadCtx<&M, A>` wrappers; use `.read().val` to get
//!   the full `HashSet<M>`.
//!
//! ## Model
//!
//! ```text
//! ActorId          = u64          (each peer has a unique ID)
//! ScalarMarker     = (u64, ActorId)  (logical clock + actor for LWW)
//! ScalarField<V>   = LWWReg<V, ScalarMarker>
//! SetField         = Orswot<Uuid, ActorId>
//! CrdtEntity       = { scalars: HashMap<name, ScalarField>,
//!                      sets:    HashMap<name, SetField> }
//! CrdtEntityStore  = { present: Orswot<Uuid, ActorId>,
//!                      data:    HashMap<Uuid, CrdtEntity> }
//! ```

use crdts::{CmRDT, CvRDT, LWWReg, Orswot};
use uuid::Uuid;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Minimal actor model
// ---------------------------------------------------------------------------

type ActorId = u64;

const ACTOR_A: ActorId = 1;
const ACTOR_B: ActorId = 2;

// ---------------------------------------------------------------------------
// Entity field model
// ---------------------------------------------------------------------------

/// A simple scalar value for spike purposes.
/// (Production would use FieldValue; it would need Ord + Clone.)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum ScalarVal {
    Str(String),
    Int(i64),
    Bool(bool),
}

/// LWW marker: (logical_time, actor_id).
/// Concurrent writes at equal logical time are resolved by actor ID.
type ScalarMarker = (u64, ActorId);

/// Sentinel zero-valued marker for newly inserted LWWReg entries.
const MARKER_ZERO: ScalarMarker = (0, 0);

type ScalarField = LWWReg<ScalarVal, ScalarMarker>;
type SetField    = Orswot<Uuid, ActorId>;

/// A sentinel ScalarVal used only when initialising a new register slot.
/// It will be immediately overwritten by the first real `write_scalar` call.
fn sentinel() -> ScalarVal {
    ScalarVal::Bool(false)
}

/// One entity's CRDT state.
#[derive(Clone, Default)]
struct CrdtEntity {
    scalars: HashMap<String, ScalarField>,
    sets:    HashMap<String, SetField>,
}

impl CrdtEntity {
    /// Write a scalar field.
    ///
    /// `marker` is a `(logical_time, actor_id)` pair that must be monotonically
    /// increasing across all writes to this field on this replica.
    fn write_scalar(&mut self, field: &str, val: ScalarVal, marker: ScalarMarker) {
        let reg = self.scalars
            .entry(field.to_string())
            .or_insert_with(|| LWWReg { val: sentinel(), marker: MARKER_ZERO });
        reg.update(val, marker);
    }

    /// Add an element to a set field for a given actor.
    fn add_to_set(&mut self, field: &str, elem: Uuid, actor: ActorId) {
        let set = self.sets.entry(field.to_string()).or_default();
        let add_ctx = set.read_ctx().derive_add_ctx(actor);
        let op = set.add(elem, add_ctx);
        set.apply(op);
    }

    /// Remove an element from a set field.
    ///
    /// Only removes elements the caller has already observed (OR-Set semantics):
    /// the `RmCtx` is derived from the *current* clock of the set on this replica.
    fn remove_from_set(&mut self, field: &str, elem: Uuid) {
        let set = self.sets.entry(field.to_string()).or_default();
        let rm_ctx = set.read_ctx().derive_rm_ctx();
        let rm = set.rm(elem, rm_ctx);
        set.apply(rm);
    }

    /// Merge another entity's CRDT state into this one.
    fn merge(&mut self, other: &CrdtEntity) {
        // Merge scalar fields (LWWReg CvRDT merge)
        for (name, other_reg) in &other.scalars {
            let reg: &mut ScalarField = self.scalars
                .entry(name.clone())
                .or_insert_with(|| LWWReg { val: sentinel(), marker: MARKER_ZERO });
            reg.merge(other_reg.clone());
        }
        // Merge set fields (Orswot CvRDT merge)
        for (name, other_set) in &other.sets {
            let set: &mut SetField = self.sets.entry(name.clone()).or_default();
            set.merge(other_set.clone());
        }
    }

    fn read_scalar(&self, field: &str) -> Option<&ScalarVal> {
        self.scalars.get(field).map(|r| &r.val)
    }

    fn read_set(&self, field: &str) -> Vec<Uuid> {
        self.sets
            .get(field)
            .map(|s| s.read().val.into_iter().collect())
            .unwrap_or_default()
    }
}

/// The full entity store: tracks which entity UUIDs exist (OR-Set) plus
/// their CRDT field data.
#[derive(Clone, Default)]
struct CrdtStore {
    present: Orswot<Uuid, ActorId>,
    data:    HashMap<Uuid, CrdtEntity>,
}

impl CrdtStore {
    fn create_entity(&mut self, uuid: Uuid, actor: ActorId) {
        let add_ctx = self.present.read_ctx().derive_add_ctx(actor);
        let op = self.present.add(uuid, add_ctx);
        self.present.apply(op);
        self.data.entry(uuid).or_default();
    }

    fn remove_entity(&mut self, uuid: Uuid) {
        let rm_ctx = self.present.read_ctx().derive_rm_ctx();
        let rm = self.present.rm(uuid, rm_ctx);
        self.present.apply(rm);
        self.data.remove(&uuid);
    }

    fn entity_mut(&mut self, uuid: Uuid) -> Option<&mut CrdtEntity> {
        if self.present.contains(&uuid).val {
            self.data.get_mut(&uuid)
        } else {
            None
        }
    }

    fn entity_exists(&self, uuid: Uuid) -> bool {
        self.present.contains(&uuid).val
    }

    fn merge(&mut self, other: &CrdtStore) {
        // Merge entity-presence set
        self.present.merge(other.present.clone());

        // Merge per-entity field data
        for (uuid, other_entity) in &other.data {
            self.data.entry(*uuid).or_default().merge(other_entity);
        }
    }
}

// ---------------------------------------------------------------------------
// Scenario 1 — Two actors create entities with different UUIDs
// ---------------------------------------------------------------------------

/// After independent creates and a merge, both entities appear.
#[test]
fn scenario_1_both_creates_survive_merge() {
    let uuid_e1 = Uuid::new_v4();
    let uuid_e2 = Uuid::new_v4();

    let mut store_a = CrdtStore::default();
    store_a.create_entity(uuid_e1, ACTOR_A);

    let mut store_b = CrdtStore::default();
    store_b.create_entity(uuid_e2, ACTOR_B);

    // Merge A ← B and verify both entities exist
    store_a.merge(&store_b);

    assert!(store_a.entity_exists(uuid_e1), "E1 created by A should survive");
    assert!(store_a.entity_exists(uuid_e2), "E2 created by B should survive");
}

// ---------------------------------------------------------------------------
// Scenario 2 — Two actors edit different fields on the same entity
// ---------------------------------------------------------------------------

/// Edits to non-overlapping fields are both preserved after merge.
#[test]
fn scenario_2_different_fields_both_preserved() {
    let uuid = Uuid::new_v4();

    // Both actors start with the same entity
    let mut store_a = CrdtStore::default();
    store_a.create_entity(uuid, ACTOR_A);

    let mut store_b = store_a.clone();

    // A edits `name`, B edits `rank` — each at logical time 1 for their actor
    store_a.entity_mut(uuid).unwrap()
        .write_scalar("name", ScalarVal::Str("Alice".into()), (1, ACTOR_A));
    store_b.entity_mut(uuid).unwrap()
        .write_scalar("rank", ScalarVal::Str("Panelist".into()), (1, ACTOR_B));

    store_a.merge(&store_b);

    let e = store_a.data.get(&uuid).unwrap();
    assert_eq!(e.read_scalar("name"), Some(&ScalarVal::Str("Alice".into())));
    assert_eq!(e.read_scalar("rank"), Some(&ScalarVal::Str("Panelist".into())));
}

// ---------------------------------------------------------------------------
// Scenario 3 — Two actors edit the same scalar field concurrently → LWW
// ---------------------------------------------------------------------------

/// When two actors write the same scalar field at the same logical time, the
/// `(logical_time, actor_id)` marker breaks ties deterministically.
/// ACTOR_B (id=2) > ACTOR_A (id=1), so B's value wins.
#[test]
fn scenario_3_same_scalar_field_lww_resolution() {
    let uuid = Uuid::new_v4();

    let mut store_a = CrdtStore::default();
    store_a.create_entity(uuid, ACTOR_A);

    let mut store_b = store_a.clone();

    // Concurrent writes at the same logical time → actor ID is tiebreaker
    store_a.entity_mut(uuid).unwrap()
        .write_scalar("name", ScalarVal::Str("Alice A".into()), (1, ACTOR_A));
    store_b.entity_mut(uuid).unwrap()
        .write_scalar("name", ScalarVal::Str("Alice B".into()), (1, ACTOR_B));

    // Merge both directions
    let mut merged_a = store_a.clone();
    merged_a.merge(&store_b);

    let mut merged_b = store_b.clone();
    merged_b.merge(&store_a);

    // Both replicas must converge to the same value
    let val_a = merged_a.data[&uuid].read_scalar("name").unwrap().clone();
    let val_b = merged_b.data[&uuid].read_scalar("name").unwrap().clone();
    assert_eq!(val_a, val_b, "LWW must converge: both replicas must agree");

    // ACTOR_B=2 > ACTOR_A=1, so (1, ACTOR_B) > (1, ACTOR_A) → B wins
    assert_eq!(
        val_a,
        ScalarVal::Str("Alice B".into()),
        "ACTOR_B's write should win (higher actor ID tiebreaker)"
    );
}

// ---------------------------------------------------------------------------
// Scenario 4 — Two actors add different presenters to the same panel
// ---------------------------------------------------------------------------

/// Both adds must survive merge (set-union / OR-Set semantics).
#[test]
fn scenario_4_concurrent_adds_both_survive() {
    let panel_uuid  = Uuid::new_v4();
    let presenter_1 = Uuid::new_v4();
    let presenter_2 = Uuid::new_v4();

    let mut store_a = CrdtStore::default();
    store_a.create_entity(panel_uuid, ACTOR_A);

    let mut store_b = store_a.clone();

    store_a.entity_mut(panel_uuid).unwrap()
        .add_to_set("presenter_ids", presenter_1, ACTOR_A);
    store_b.entity_mut(panel_uuid).unwrap()
        .add_to_set("presenter_ids", presenter_2, ACTOR_B);

    store_a.merge(&store_b);

    let ids = store_a.data[&panel_uuid].read_set("presenter_ids");
    assert!(ids.contains(&presenter_1), "Presenter 1 (added by A) must survive");
    assert!(ids.contains(&presenter_2), "Presenter 2 (added by B) must survive");
}

// ---------------------------------------------------------------------------
// Scenario 5 — Actor A adds presenter X, Actor B removes it concurrently
// ---------------------------------------------------------------------------

/// OR-Set semantics: if B never observed A's add of X, B's remove has no
/// observed token to cancel.  After merge, X must still be present
/// (add wins over concurrent unobserved remove).
#[test]
fn scenario_5_add_wins_over_unobserved_concurrent_remove() {
    let panel_uuid  = Uuid::new_v4();
    let presenter_x = Uuid::new_v4();

    // Both actors start from the same empty state
    let mut store_a = CrdtStore::default();
    store_a.create_entity(panel_uuid, ACTOR_A);
    let mut store_b = store_a.clone(); // B has not observed any adds yet

    // A adds presenter X
    store_a.entity_mut(panel_uuid).unwrap()
        .add_to_set("presenter_ids", presenter_x, ACTOR_A);

    // B (from its own view where X was never added) attempts to remove X.
    // Because B's set is empty, this remove has no observed tokens to cancel
    // — it is effectively a no-op against A's add.
    store_b.entity_mut(panel_uuid).unwrap()
        .remove_from_set("presenter_ids", presenter_x);

    // Merge A ← B
    store_a.merge(&store_b);

    let ids = store_a.data[&panel_uuid].read_set("presenter_ids");
    assert!(
        ids.contains(&presenter_x),
        "Add by A must win over B's unobserved-remove (OR-Set semantics)"
    );
}

// ---------------------------------------------------------------------------
// Scenario 6 — Merge idempotency (identity law: X ∪ X = X)
// ---------------------------------------------------------------------------

/// Merging a state with itself must be idempotent.
/// (Stands in for a full serialise/deserialise round-trip test;
/// serde support for crdts requires the serde feature which this crate
/// does not enable — tested separately if needed.)
#[test]
fn scenario_6_merge_identity_convergence() {
    let panel_uuid  = Uuid::new_v4();
    let presenter_1 = Uuid::new_v4();
    let presenter_2 = Uuid::new_v4();

    let mut store = CrdtStore::default();
    store.create_entity(panel_uuid, ACTOR_A);
    store.entity_mut(panel_uuid).unwrap()
        .add_to_set("presenter_ids", presenter_1, ACTOR_A);
    store.entity_mut(panel_uuid).unwrap()
        .add_to_set("presenter_ids", presenter_2, ACTOR_A);

    // Merging a clone into itself must be idempotent (X ∪ X = X)
    let clone = store.clone();
    store.merge(&clone);

    let ids = store.data[&panel_uuid].read_set("presenter_ids");
    assert_eq!(ids.len(), 2, "Idempotent merge must not duplicate elements");
    assert!(ids.contains(&presenter_1));
    assert!(ids.contains(&presenter_2));
}

// ---------------------------------------------------------------------------
// Bonus: entity remove vs concurrent field edit
// ---------------------------------------------------------------------------

/// If A removes an entity and B concurrently edits a field on it, the
/// OR-Set presence wins for whichever operation carries the observed token.
/// Here A removes from its own observation; B's edit keeps the entity alive
/// in B's view.  After merge, entity presence is determined by Orswot.
#[test]
fn bonus_remove_vs_concurrent_edit() {
    let uuid = Uuid::new_v4();

    let mut store_a = CrdtStore::default();
    store_a.create_entity(uuid, ACTOR_A);
    let mut store_b = store_a.clone();

    // A removes the entity
    store_a.remove_entity(uuid);
    assert!(!store_a.entity_exists(uuid));

    // B edits a field on the entity (B still thinks it exists)
    store_b.entity_mut(uuid).unwrap()
        .write_scalar("name", ScalarVal::Str("Still Here".into()), (1, ACTOR_B));

    // After merge: OR-Set semantics mean A's remove cancels only the tokens
    // it observed (the original create).  B did not add a new token, so
    // the entity is gone in the merged state.
    store_a.merge(&store_b);

    // The entity was created once (by A), A observed and removed that token,
    // B had no concurrent create — so entity is removed.
    assert!(
        !store_a.entity_exists(uuid),
        "Remove wins when B made no concurrent create"
    );
}
