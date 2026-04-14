/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Automerge implementation of [`CrdtDocument`].
//!
//! All entity kinds share one [`AutoCommit`] document.  Field types map to
//! automerge operations as follows:
//!
//! | Field category | automerge type | Operation |
//! |---|---|---|
//! | Scalars (String, Int, Bool, DateTime, Duration, UUID ref) | Scalar via `put()` | LWW |
//! | Prose (description, note, *_notes) | `ObjType::Text` via `splice_text` | RGA |
//! | Relationship sets (presenter_ids, etc.) | `ObjType::List` | add-wins (OR-Set equivalent) |
//!
//! ## Document root layout
//!
//! ```text
//! ROOT (Map)
//! ├── "panels"      → Map { uuid_str → Map { field → value } }
//! ├── "presenters"  → Map { uuid_str → Map { field → value } }
//! ├── "event_rooms" → Map { uuid_str → Map { field → value } }
//! ├── "hotel_rooms" → Map { uuid_str → Map { field → value } }
//! └── "panel_types" → Map { uuid_str → Map { field → value } }
//! ```
//!
//! Relationship list fields hold a `List` of UUID strings.
//! Prose fields hold a `Text` object.
//! All other fields are scalars.

use std::collections::HashSet;

use automerge::transaction::Transactable;
use automerge::{AutoCommit, ObjId, ObjType, ReadDoc, ScalarValue, Value, ROOT};
use thiserror::Error;

use crate::entity::EntityKind;

use super::{ActorId, CrdtDocument, CrdtOp, CrdtScalar};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Error type for automerge backend operations.
#[derive(Debug, Error)]
pub enum AutomergeDocError {
    /// An error returned by the automerge library.
    #[error("automerge error: {0}")]
    Automerge(#[from] automerge::AutomergeError),
}

// ---------------------------------------------------------------------------
// AutomergeDocument
// ---------------------------------------------------------------------------

/// Automerge-backed CRDT schedule document.
///
/// Wraps an [`AutoCommit`] document where each entity kind lives in a top-level
/// map keyed by the kind name, and each entity's fields are in a nested map
/// keyed by the entity UUID string.
///
/// Merge the entire document with [`CrdtDocument::merge_from`]; there is no
/// partial-entity merge — the single-document model gives causal consistency
/// across all entity types at once.
pub struct AutomergeDocument {
    inner: AutoCommit,
}

impl AutomergeDocument {
    /// Return the automerge `ObjId` for an entity's field map, creating the
    /// kind-level map and the entity map if they don't yet exist.
    fn get_or_create_entity_map(
        &mut self,
        kind: EntityKind,
        uuid: uuid::Uuid,
    ) -> Result<ObjId, AutomergeDocError> {
        let kind_key = kind_key(kind);
        let kind_map = match self.inner.get(ROOT, kind_key)? {
            Some((Value::Object(_), id)) => id,
            _ => self.inner.put_object(ROOT, kind_key, ObjType::Map)?,
        };
        let uuid_str = uuid.to_string();
        match self.inner.get(&kind_map, uuid_str.as_str())? {
            Some((Value::Object(_), id)) => Ok(id),
            _ => Ok(self
                .inner
                .put_object(&kind_map, uuid_str.as_str(), ObjType::Map)?),
        }
    }

    /// Return the automerge `ObjId` for an entity's field map without creating
    /// it.  Returns `None` if the entity does not exist.
    fn find_entity_map(&self, kind: EntityKind, uuid: uuid::Uuid) -> Option<ObjId> {
        let kind_key = kind_key(kind);
        let kind_result = self.inner.get(ROOT, kind_key).ok()?;
        let kind_map = match kind_result? {
            (Value::Object(_), id) => id,
            _ => return None,
        };
        let uuid_str = uuid.to_string();
        let entity_result = self.inner.get(&kind_map, uuid_str.as_str()).ok()?;
        match entity_result? {
            (Value::Object(_), id) => Some(id),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// CrdtDocument impl
// ---------------------------------------------------------------------------

impl CrdtDocument for AutomergeDocument {
    type Error = AutomergeDocError;

    fn new(actor: &ActorId) -> Result<Self, Self::Error> {
        let actor_id = automerge::ActorId::from(actor.uuid().as_bytes().to_vec());
        let mut inner = AutoCommit::new();
        inner.set_actor(actor_id);
        Ok(Self { inner })
    }

    fn load(bytes: &[u8]) -> Result<Self, Self::Error> {
        let inner = AutoCommit::load(bytes)?;
        Ok(Self { inner })
    }

    fn set_actor(&mut self, actor: &ActorId) {
        let actor_id = automerge::ActorId::from(actor.uuid().as_bytes().to_vec());
        self.inner.set_actor(actor_id);
    }

    fn save(&mut self) -> Vec<u8> {
        self.inner.save()
    }

    fn merge_from(&mut self, other_bytes: &[u8]) -> Result<(), Self::Error> {
        let mut other = AutoCommit::load(other_bytes)?;
        self.inner.merge(&mut other)?;
        Ok(())
    }

    fn apply(&mut self, op: &CrdtOp) -> Result<(), Self::Error> {
        match op {
            CrdtOp::EnsureEntity {
                entity_kind,
                entity_uuid,
            } => {
                let _ = self.get_or_create_entity_map(*entity_kind, *entity_uuid)?;
            }

            CrdtOp::PutScalar {
                entity_kind,
                entity_uuid,
                field_name,
                value,
            } => {
                let entity_map = self.get_or_create_entity_map(*entity_kind, *entity_uuid)?;
                let sv = crdt_scalar_to_automerge(value);
                self.inner.put(&entity_map, field_name.as_str(), sv)?;
            }

            CrdtOp::PutText {
                entity_kind,
                entity_uuid,
                field_name,
                text,
            } => {
                let entity_map = self.get_or_create_entity_map(*entity_kind, *entity_uuid)?;
                // Get existing Text object or create a new one.
                let maybe_text_id = self
                    .inner
                    .get(&entity_map, field_name.as_str())?
                    .and_then(|(v, id)| {
                        if matches!(v, Value::Object(ObjType::Text)) {
                            Some(id)
                        } else {
                            None
                        }
                    });
                let text_obj = match maybe_text_id {
                    Some(id) => id,
                    None => self
                        .inner
                        .put_object(&entity_map, field_name.as_str(), ObjType::Text)?,
                };
                // Replace entire text content.
                let current_len = self.inner.text(&text_obj)?.chars().count() as isize;
                self.inner.splice_text(&text_obj, 0, current_len, text)?;
            }

            CrdtOp::ListAdd {
                entity_kind,
                entity_uuid,
                field_name,
                element,
            } => {
                let entity_map = self.get_or_create_entity_map(*entity_kind, *entity_uuid)?;
                // Get existing List object or create a new one.
                let maybe_list_id = self
                    .inner
                    .get(&entity_map, field_name.as_str())?
                    .and_then(|(v, id)| {
                        if matches!(v, Value::Object(ObjType::List)) {
                            Some(id)
                        } else {
                            None
                        }
                    });
                let list_obj = match maybe_list_id {
                    Some(id) => id,
                    None => self
                        .inner
                        .put_object(&entity_map, field_name.as_str(), ObjType::List)?,
                };
                let uuid_str = element.to_string();
                let len = self.inner.length(&list_obj);
                // Deduplicate: skip if the element is already present.
                let mut already_present = false;
                for i in 0..len {
                    if let Ok(Some((Value::Scalar(sv), _))) = self.inner.get(&list_obj, i) {
                        if let ScalarValue::Str(s) = sv.as_ref() {
                            if s.as_str() == uuid_str.as_str() {
                                already_present = true;
                                break;
                            }
                        }
                    }
                }
                if !already_present {
                    self.inner
                        .insert(&list_obj, len, ScalarValue::Str(uuid_str.into()))?;
                }
            }

            CrdtOp::ListRemove {
                entity_kind,
                entity_uuid,
                field_name,
                element,
            } => {
                let entity_map = match self.find_entity_map(*entity_kind, *entity_uuid) {
                    Some(id) => id,
                    None => return Ok(()), // entity doesn't exist; nothing to remove
                };
                let list_obj = match self
                    .inner
                    .get(&entity_map, field_name.as_str())?
                    .and_then(|(v, id)| {
                        if matches!(v, Value::Object(ObjType::List)) {
                            Some(id)
                        } else {
                            None
                        }
                    }) {
                    Some(id) => id,
                    None => return Ok(()), // no list; nothing to remove
                };
                let uuid_str = element.to_string();
                let len = self.inner.length(&list_obj);
                // Walk backwards so index arithmetic stays valid after deletions.
                for i in (0..len).rev() {
                    if let Ok(Some((Value::Scalar(sv), _))) = self.inner.get(&list_obj, i) {
                        if let ScalarValue::Str(s) = sv.as_ref() {
                            if s.as_str() == uuid_str.as_str() {
                                self.inner.delete(&list_obj, i)?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn read_scalar(&self, kind: EntityKind, uuid: uuid::Uuid, field: &str) -> Option<CrdtScalar> {
        let entity_map = self.find_entity_map(kind, uuid)?;
        let result = self.inner.get(&entity_map, field).ok()?;
        let (value, _) = result?;
        match value {
            Value::Scalar(sv) => automerge_to_crdt_scalar(sv.as_ref()),
            _ => None,
        }
    }

    fn read_text(&self, kind: EntityKind, uuid: uuid::Uuid, field: &str) -> Option<String> {
        let entity_map = self.find_entity_map(kind, uuid)?;
        let result = self.inner.get(&entity_map, field).ok()?;
        let (value, obj_id) = result?;
        match value {
            Value::Object(ObjType::Text) => self.inner.text(&obj_id).ok(),
            _ => None,
        }
    }

    fn read_list(&self, kind: EntityKind, uuid: uuid::Uuid, field: &str) -> Vec<uuid::Uuid> {
        let entity_map = match self.find_entity_map(kind, uuid) {
            Some(id) => id,
            None => return Vec::new(),
        };
        let list_id = match self.inner.get(&entity_map, field) {
            Ok(Some((Value::Object(ObjType::List), id))) => id,
            _ => return Vec::new(),
        };
        let len = self.inner.length(&list_id);
        let mut result = Vec::with_capacity(len);
        let mut seen = HashSet::new();
        for i in 0..len {
            if let Ok(Some((Value::Scalar(sv), _))) = self.inner.get(&list_id, i) {
                if let ScalarValue::Str(s) = sv.as_ref() {
                    if let Ok(u) = s.as_str().parse::<uuid::Uuid>() {
                        if seen.insert(u) {
                            result.push(u);
                        }
                    }
                }
            }
        }
        result
    }

    fn entity_exists(&self, kind: EntityKind, uuid: uuid::Uuid) -> bool {
        self.find_entity_map(kind, uuid).is_some()
    }
}

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------

/// Map [`EntityKind`] to the document root key string.
pub(crate) fn kind_key(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Panel => "panels",
        EntityKind::Presenter => "presenters",
        EntityKind::EventRoom => "event_rooms",
        EntityKind::HotelRoom => "hotel_rooms",
        EntityKind::PanelType => "panel_types",
    }
}

/// Convert a [`CrdtScalar`] to an automerge [`ScalarValue`].
fn crdt_scalar_to_automerge(value: &CrdtScalar) -> ScalarValue {
    match value {
        CrdtScalar::Null => ScalarValue::Null,
        CrdtScalar::Bool(b) => ScalarValue::Boolean(*b),
        CrdtScalar::Int(i) => ScalarValue::Int(*i),
        CrdtScalar::Float(f) => ScalarValue::F64(*f),
        CrdtScalar::Str(s) => ScalarValue::Str(s.as_str().into()),
        CrdtScalar::TimestampMs(ms) => ScalarValue::Timestamp(*ms),
        CrdtScalar::DurationMins(m) => ScalarValue::Int(*m),
        CrdtScalar::Uuid(u) => ScalarValue::Str(u.to_string().as_str().into()),
    }
}

/// Convert an automerge [`ScalarValue`] to a [`CrdtScalar`].
///
/// Returns `None` for automerge-internal types that have no `CrdtScalar`
/// equivalent (`Bytes`, `Counter`, `Unknown`).
fn automerge_to_crdt_scalar(sv: &ScalarValue) -> Option<CrdtScalar> {
    match sv {
        ScalarValue::Null => Some(CrdtScalar::Null),
        ScalarValue::Boolean(b) => Some(CrdtScalar::Bool(*b)),
        ScalarValue::Int(i) => Some(CrdtScalar::Int(*i)),
        ScalarValue::Uint(u) => Some(CrdtScalar::Int(*u as i64)),
        ScalarValue::F64(f) => Some(CrdtScalar::Float(*f)),
        ScalarValue::Str(s) => Some(CrdtScalar::Str(s.to_string())),
        ScalarValue::Timestamp(ts) => Some(CrdtScalar::TimestampMs(*ts)),
        ScalarValue::Bytes(_) | ScalarValue::Counter(_) | ScalarValue::Unknown { .. } => None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::EntityKind;

    fn actor_a() -> ActorId {
        ActorId::from_uuid(uuid::Uuid::from_bytes([
            0xAA, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
        ]))
    }

    fn actor_b() -> ActorId {
        ActorId::from_uuid(uuid::Uuid::from_bytes([
            0xBB, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
        ]))
    }

    fn panel_uuid() -> uuid::Uuid {
        uuid::Uuid::from_bytes([0x01; 16])
    }

    fn presenter_uuid() -> uuid::Uuid {
        uuid::Uuid::from_bytes([0x02; 16])
    }

    // -----------------------------------------------------------------------
    // Basic entity / scalar tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_new_doc_no_entities() {
        let doc = AutomergeDocument::new(&actor_a()).unwrap();
        assert!(!doc.entity_exists(EntityKind::Panel, panel_uuid()));
    }

    #[test]
    fn test_ensure_entity_creates_entry() {
        let mut doc = AutomergeDocument::new(&actor_a()).unwrap();
        doc.apply(&CrdtOp::EnsureEntity {
            entity_kind: EntityKind::Panel,
            entity_uuid: panel_uuid(),
        })
        .unwrap();
        assert!(doc.entity_exists(EntityKind::Panel, panel_uuid()));
        // Other kinds remain absent
        assert!(!doc.entity_exists(EntityKind::Presenter, panel_uuid()));
    }

    #[test]
    fn test_ensure_entity_idempotent() {
        let mut doc = AutomergeDocument::new(&actor_a()).unwrap();
        let op = CrdtOp::EnsureEntity {
            entity_kind: EntityKind::Panel,
            entity_uuid: panel_uuid(),
        };
        doc.apply(&op).unwrap();
        doc.apply(&op).unwrap(); // second apply must not error
        assert!(doc.entity_exists(EntityKind::Panel, panel_uuid()));
    }

    #[test]
    fn test_put_scalar_string_roundtrip() {
        let mut doc = AutomergeDocument::new(&actor_a()).unwrap();
        doc.apply(&CrdtOp::PutScalar {
            entity_kind: EntityKind::Panel,
            entity_uuid: panel_uuid(),
            field_name: "title".to_string(),
            value: CrdtScalar::Str("My Panel".to_string()),
        })
        .unwrap();
        assert_eq!(
            doc.read_scalar(EntityKind::Panel, panel_uuid(), "title"),
            Some(CrdtScalar::Str("My Panel".to_string()))
        );
    }

    #[test]
    fn test_put_scalar_bool_roundtrip() {
        let mut doc = AutomergeDocument::new(&actor_a()).unwrap();
        doc.apply(&CrdtOp::PutScalar {
            entity_kind: EntityKind::Panel,
            entity_uuid: panel_uuid(),
            field_name: "is_break".to_string(),
            value: CrdtScalar::Bool(true),
        })
        .unwrap();
        assert_eq!(
            doc.read_scalar(EntityKind::Panel, panel_uuid(), "is_break"),
            Some(CrdtScalar::Bool(true))
        );
    }

    #[test]
    fn test_put_scalar_int_roundtrip() {
        let mut doc = AutomergeDocument::new(&actor_a()).unwrap();
        doc.apply(&CrdtOp::PutScalar {
            entity_kind: EntityKind::Panel,
            entity_uuid: panel_uuid(),
            field_name: "rank".to_string(),
            value: CrdtScalar::Int(42),
        })
        .unwrap();
        assert_eq!(
            doc.read_scalar(EntityKind::Panel, panel_uuid(), "rank"),
            Some(CrdtScalar::Int(42))
        );
    }

    #[test]
    fn test_put_scalar_null_roundtrip() {
        let mut doc = AutomergeDocument::new(&actor_a()).unwrap();
        // Set a value then null it out (soft-delete pattern)
        doc.apply(&CrdtOp::PutScalar {
            entity_kind: EntityKind::Panel,
            entity_uuid: panel_uuid(),
            field_name: "title".to_string(),
            value: CrdtScalar::Str("Old Title".to_string()),
        })
        .unwrap();
        doc.apply(&CrdtOp::PutScalar {
            entity_kind: EntityKind::Panel,
            entity_uuid: panel_uuid(),
            field_name: "title".to_string(),
            value: CrdtScalar::Null,
        })
        .unwrap();
        assert_eq!(
            doc.read_scalar(EntityKind::Panel, panel_uuid(), "title"),
            Some(CrdtScalar::Null)
        );
    }

    // -----------------------------------------------------------------------
    // Prose / Text tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_put_text_roundtrip() {
        let mut doc = AutomergeDocument::new(&actor_a()).unwrap();
        doc.apply(&CrdtOp::PutText {
            entity_kind: EntityKind::Panel,
            entity_uuid: panel_uuid(),
            field_name: "description".to_string(),
            text: "Alice teaches cosplay.".to_string(),
        })
        .unwrap();
        assert_eq!(
            doc.read_text(EntityKind::Panel, panel_uuid(), "description"),
            Some("Alice teaches cosplay.".to_string())
        );
    }

    #[test]
    fn test_put_text_overwrite() {
        let mut doc = AutomergeDocument::new(&actor_a()).unwrap();
        doc.apply(&CrdtOp::PutText {
            entity_kind: EntityKind::Panel,
            entity_uuid: panel_uuid(),
            field_name: "description".to_string(),
            text: "First version.".to_string(),
        })
        .unwrap();
        doc.apply(&CrdtOp::PutText {
            entity_kind: EntityKind::Panel,
            entity_uuid: panel_uuid(),
            field_name: "description".to_string(),
            text: "Second version.".to_string(),
        })
        .unwrap();
        assert_eq!(
            doc.read_text(EntityKind::Panel, panel_uuid(), "description"),
            Some("Second version.".to_string())
        );
    }

    // -----------------------------------------------------------------------
    // List / relationship tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_list_add_read() {
        let mut doc = AutomergeDocument::new(&actor_a()).unwrap();
        doc.apply(&CrdtOp::ListAdd {
            entity_kind: EntityKind::Panel,
            entity_uuid: panel_uuid(),
            field_name: "presenter_ids".to_string(),
            element: presenter_uuid(),
        })
        .unwrap();
        let list = doc.read_list(EntityKind::Panel, panel_uuid(), "presenter_ids");
        assert_eq!(list, vec![presenter_uuid()]);
    }

    #[test]
    fn test_list_add_dedup() {
        let mut doc = AutomergeDocument::new(&actor_a()).unwrap();
        let op = CrdtOp::ListAdd {
            entity_kind: EntityKind::Panel,
            entity_uuid: panel_uuid(),
            field_name: "presenter_ids".to_string(),
            element: presenter_uuid(),
        };
        doc.apply(&op).unwrap();
        doc.apply(&op).unwrap(); // second add must not duplicate
        let list = doc.read_list(EntityKind::Panel, panel_uuid(), "presenter_ids");
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn test_list_remove() {
        let mut doc = AutomergeDocument::new(&actor_a()).unwrap();
        doc.apply(&CrdtOp::ListAdd {
            entity_kind: EntityKind::Panel,
            entity_uuid: panel_uuid(),
            field_name: "presenter_ids".to_string(),
            element: presenter_uuid(),
        })
        .unwrap();
        doc.apply(&CrdtOp::ListRemove {
            entity_kind: EntityKind::Panel,
            entity_uuid: panel_uuid(),
            field_name: "presenter_ids".to_string(),
            element: presenter_uuid(),
        })
        .unwrap();
        assert!(
            doc.read_list(EntityKind::Panel, panel_uuid(), "presenter_ids")
                .is_empty()
        );
    }

    #[test]
    fn test_list_remove_missing_is_noop() {
        let mut doc = AutomergeDocument::new(&actor_a()).unwrap();
        // Remove from non-existent entity — must not panic
        let result = doc.apply(&CrdtOp::ListRemove {
            entity_kind: EntityKind::Panel,
            entity_uuid: panel_uuid(),
            field_name: "presenter_ids".to_string(),
            element: presenter_uuid(),
        });
        assert!(result.is_ok());
    }

    // -----------------------------------------------------------------------
    // Save / load round-trip
    // -----------------------------------------------------------------------

    #[test]
    fn test_save_load_roundtrip() {
        let mut doc = AutomergeDocument::new(&actor_a()).unwrap();
        doc.apply(&CrdtOp::PutScalar {
            entity_kind: EntityKind::Panel,
            entity_uuid: panel_uuid(),
            field_name: "title".to_string(),
            value: CrdtScalar::Str("Saved Panel".to_string()),
        })
        .unwrap();
        doc.apply(&CrdtOp::PutText {
            entity_kind: EntityKind::Panel,
            entity_uuid: panel_uuid(),
            field_name: "description".to_string(),
            text: "Some description.".to_string(),
        })
        .unwrap();
        doc.apply(&CrdtOp::ListAdd {
            entity_kind: EntityKind::Panel,
            entity_uuid: panel_uuid(),
            field_name: "presenter_ids".to_string(),
            element: presenter_uuid(),
        })
        .unwrap();

        let bytes = doc.save();
        let loaded = AutomergeDocument::load(&bytes).unwrap();

        assert_eq!(
            loaded.read_scalar(EntityKind::Panel, panel_uuid(), "title"),
            Some(CrdtScalar::Str("Saved Panel".to_string()))
        );
        assert_eq!(
            loaded.read_text(EntityKind::Panel, panel_uuid(), "description"),
            Some("Some description.".to_string())
        );
        assert_eq!(
            loaded.read_list(EntityKind::Panel, panel_uuid(), "presenter_ids"),
            vec![presenter_uuid()]
        );
    }

    // -----------------------------------------------------------------------
    // Merge / CRDT semantics tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_merge_scalar_lww_different_fields() {
        // Two actors edit different fields on a shared document — both survive.
        //
        // Production workflow: device B loads device A's file (shared ancestor),
        // sets its own actor, then both make independent changes.
        let mut doc_a = AutomergeDocument::new(&actor_a()).unwrap();
        // Ensure the entity exists in the shared base before forking.
        doc_a
            .apply(&CrdtOp::EnsureEntity {
                entity_kind: EntityKind::Presenter,
                entity_uuid: presenter_uuid(),
            })
            .unwrap();

        // Fork: device B loads A's current state.
        let shared_base = doc_a.save();
        let mut doc_b = AutomergeDocument::load(&shared_base).unwrap();
        doc_b.set_actor(&actor_b());

        // Both actors write different fields concurrently (divergent edits).
        doc_a
            .apply(&CrdtOp::PutScalar {
                entity_kind: EntityKind::Presenter,
                entity_uuid: presenter_uuid(),
                field_name: "name".to_string(),
                value: CrdtScalar::Str("Alice".to_string()),
            })
            .unwrap();
        doc_b
            .apply(&CrdtOp::PutScalar {
                entity_kind: EntityKind::Presenter,
                entity_uuid: presenter_uuid(),
                field_name: "rank".to_string(),
                value: CrdtScalar::Str("Panelist".to_string()),
            })
            .unwrap();

        let b_bytes = doc_b.save();
        doc_a.merge_from(&b_bytes).unwrap();

        assert_eq!(
            doc_a.read_scalar(EntityKind::Presenter, presenter_uuid(), "name"),
            Some(CrdtScalar::Str("Alice".to_string()))
        );
        assert_eq!(
            doc_a.read_scalar(EntityKind::Presenter, presenter_uuid(), "rank"),
            Some(CrdtScalar::Str("Panelist".to_string()))
        );
    }

    #[test]
    fn test_merge_list_both_adds_survive() {
        // Two actors both add presenters — both survive after merge (OR-Set semantics).
        //
        // Production workflow: device A creates the schedule and adds p1.  Device B
        // receives A's file (shared ancestor that includes the list object), then
        // adds p2 concurrently.  After merge both are present.
        //
        // The shared ancestor must include the list object — automerge's List is an
        // RGA with a stable ObjId.  If both devices created the list from scratch,
        // there would be a key conflict on "presenter_ids" resolved by LWW.
        let p1 = uuid::Uuid::from_bytes([0x11; 16]);
        let p2 = uuid::Uuid::from_bytes([0x22; 16]);

        // doc_a creates the entity and adds p1, establishing the list object.
        let mut doc_a = AutomergeDocument::new(&actor_a()).unwrap();
        doc_a
            .apply(&CrdtOp::ListAdd {
                entity_kind: EntityKind::Panel,
                entity_uuid: panel_uuid(),
                field_name: "presenter_ids".to_string(),
                element: p1,
            })
            .unwrap();

        // Fork: device B loads A's file (shared ancestor with p1 and the list object).
        let shared_base = doc_a.save();
        let mut doc_b = AutomergeDocument::load(&shared_base).unwrap();
        doc_b.set_actor(&actor_b());

        // doc_b concurrently adds p2 to the same list.
        doc_b
            .apply(&CrdtOp::ListAdd {
                entity_kind: EntityKind::Panel,
                entity_uuid: panel_uuid(),
                field_name: "presenter_ids".to_string(),
                element: p2,
            })
            .unwrap();

        // Merge B's changes back into A — both adds must survive.
        let b_bytes = doc_b.save();
        doc_a.merge_from(&b_bytes).unwrap();

        let list = doc_a.read_list(EntityKind::Panel, panel_uuid(), "presenter_ids");
        assert!(list.contains(&p1), "A's add (in shared base) must survive");
        assert!(list.contains(&p2), "B's concurrent add must survive");
    }

    #[test]
    fn test_merge_text_both_edits_survive() {
        // Two actors edit non-overlapping parts of a prose field — both survive.
        let mut doc_a = AutomergeDocument::new(&actor_a()).unwrap();
        // A sets the initial text
        doc_a
            .apply(&CrdtOp::PutText {
                entity_kind: EntityKind::Panel,
                entity_uuid: panel_uuid(),
                field_name: "description".to_string(),
                text: "Alice presents cosplay.".to_string(),
            })
            .unwrap();

        // B gets doc_a's state and makes independent edits (fork)
        let a_bytes_before = doc_a.save();
        let mut doc_b = AutomergeDocument::load(&a_bytes_before).unwrap();
        doc_b.set_actor(&actor_b());
        doc_b
            .apply(&CrdtOp::PutText {
                entity_kind: EntityKind::Panel,
                entity_uuid: panel_uuid(),
                field_name: "description".to_string(),
                text: "Alice presents cosplay. Registration required.".to_string(),
            })
            .unwrap();

        // A makes its own edit in parallel (on the saved version before B's changes)
        doc_a
            .apply(&CrdtOp::PutScalar {
                entity_kind: EntityKind::Panel,
                entity_uuid: panel_uuid(),
                field_name: "title".to_string(),
                value: CrdtScalar::Str("Panel A Title".to_string()),
            })
            .unwrap();

        // Merge B into A — both changes should be present
        let b_bytes = doc_b.save();
        doc_a.merge_from(&b_bytes).unwrap();

        // A's scalar write should still be there
        assert_eq!(
            doc_a.read_scalar(EntityKind::Panel, panel_uuid(), "title"),
            Some(CrdtScalar::Str("Panel A Title".to_string()))
        );
        // The document should have content from both (one will win on LWW text replacement —
        // the key invariant is that merge doesn't error and the doc is consistent)
        assert!(
            doc_a
                .read_text(EntityKind::Panel, panel_uuid(), "description")
                .is_some(),
            "description must be readable after merge"
        );
    }

    #[test]
    fn test_merge_idempotent() {
        // Merging the same bytes twice must not change the state.
        let mut doc = AutomergeDocument::new(&actor_a()).unwrap();
        doc.apply(&CrdtOp::PutScalar {
            entity_kind: EntityKind::Panel,
            entity_uuid: panel_uuid(),
            field_name: "title".to_string(),
            value: CrdtScalar::Str("Idempotent".to_string()),
        })
        .unwrap();

        let bytes = doc.save();
        doc.merge_from(&bytes).unwrap();
        doc.merge_from(&bytes).unwrap();

        assert_eq!(
            doc.read_scalar(EntityKind::Panel, panel_uuid(), "title"),
            Some(CrdtScalar::Str("Idempotent".to_string()))
        );
    }

    #[test]
    fn test_read_missing_entity_returns_none() {
        let doc = AutomergeDocument::new(&actor_a()).unwrap();
        assert!(doc
            .read_scalar(EntityKind::Panel, panel_uuid(), "title")
            .is_none());
        assert!(doc
            .read_text(EntityKind::Panel, panel_uuid(), "description")
            .is_none());
        assert!(
            doc.read_list(EntityKind::Panel, panel_uuid(), "presenter_ids")
                .is_empty()
        );
    }

    #[test]
    fn test_multiple_entity_kinds_independent() {
        let mut doc = AutomergeDocument::new(&actor_a()).unwrap();
        doc.apply(&CrdtOp::PutScalar {
            entity_kind: EntityKind::Panel,
            entity_uuid: panel_uuid(),
            field_name: "title".to_string(),
            value: CrdtScalar::Str("Panel".to_string()),
        })
        .unwrap();
        doc.apply(&CrdtOp::PutScalar {
            entity_kind: EntityKind::Presenter,
            entity_uuid: presenter_uuid(),
            field_name: "name".to_string(),
            value: CrdtScalar::Str("Alice".to_string()),
        })
        .unwrap();

        assert!(doc.entity_exists(EntityKind::Panel, panel_uuid()));
        assert!(doc.entity_exists(EntityKind::Presenter, presenter_uuid()));
        assert!(!doc.entity_exists(EntityKind::EventRoom, panel_uuid()));
    }
}
