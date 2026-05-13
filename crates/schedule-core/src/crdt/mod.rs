/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! CRDT system for conflict-free replicated data types.
//!
//! This module provides the CRDT document structure and edge-specific CRDT operations
//! for the schedule system.

use automerge::transaction::Transactable;
use automerge::{AutoCommit, ObjId, ObjType, ReadDoc, ScalarValue, Value, ROOT};
use chrono::{DateTime, Duration, TimeZone, Utc};
use thiserror::Error;
use uuid::{NonNilUuid, Uuid};

use crate::edit::builder::{build_entity, BuildError, EntityBuildable};
use crate::entity::{EntityTyped, EntityUuid, RuntimeEntityId, UuidPreference};
use crate::field::set::{FieldRef, FieldUpdate};
use crate::field::NamedField;
use crate::schedule::Schedule;
use crate::value::{FieldTypeItem, FieldValue, FieldValueItem};

/// Top-level key for the entities sub-map in the document.
pub const ENTITIES_KEY: &str = "entities";

/// Soft-delete flag stored as a boolean scalar inside each entity map.
pub const DELETED_KEY: &str = "__deleted";

// ── Error ──────────────────────────────────────────────────────────────────

/// Errors raised by the CRDT mirror layer.
///
/// These wrap the underlying [`automerge::AutomergeError`] plus any shape
/// mismatches discovered while reading or writing the document.
#[derive(Debug, Error)]
pub enum CrdtError {
    /// Underlying automerge error.
    #[error("automerge error: {0}")]
    Automerge(#[from] automerge::AutomergeError),

    /// A value in the document did not match the expected shape.
    #[error("type mismatch: {0}")]
    TypeMismatch(String),

    /// Save/load bytes could not be decoded.
    #[error("codec error: {0}")]
    Codec(String),

    /// `FieldValue` + `CrdtFieldType` combination is not supported.
    #[error("unsupported: {0}")]
    Unsupported(String),
}

/// Shorthand result type for the mirror layer.
pub type CrdtResult<T> = Result<T, CrdtError>;

// ── CrdtFieldType ─────────────────────────────────────────────────────────────

/// How a field maps to CRDT storage in Phase 4.
///
/// Annotations are baked in from Phase 2 so no entity structs need changing
/// when automerge integration lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrdtFieldType {
    /// Last-write-wins scalar via `put` / `get` (automerge LWW).
    Scalar,
    /// Prose RGA text via `splice_text` / `text` (automerge RGA).
    Text,
    /// OR-Set equivalent list via `insert` / `delete` / `list` (automerge list).
    List,
    /// Computed from relationships; not stored in CRDT — lives only in RAM.
    Derived,
}

// ── Path helpers ───────────────────────────────────────────────────────────

/// Get-or-create a Map child named `key` on `parent`.
pub fn ensure_map(doc: &mut AutoCommit, parent: &ObjId, key: &str) -> CrdtResult<ObjId> {
    match doc.get(parent, key)? {
        Some((Value::Object(ObjType::Map), id)) => Ok(id),
        Some((other, _)) => Err(CrdtError::TypeMismatch(format!(
            "{key}: expected Map, got {other:?}"
        ))),
        None => Ok(doc.put_object(parent, key, ObjType::Map)?),
    }
}

/// Resolve the map for one entity, creating any missing intermediate maps.
pub fn ensure_entity_map(
    doc: &mut AutoCommit,
    type_name: &str,
    uuid: NonNilUuid,
) -> CrdtResult<ObjId> {
    let entities = ensure_map(doc, &ROOT, ENTITIES_KEY)?;
    let type_map = ensure_map(doc, &entities, type_name)?;
    ensure_map(doc, &type_map, &uuid.to_string())
}

/// Read-only version of [`ensure_entity_map`] — returns `None` if any level
/// of the path is missing.
#[must_use]
pub fn get_entity_map(doc: &AutoCommit, type_name: &str, uuid: NonNilUuid) -> Option<ObjId> {
    let entities = match doc.get(&ROOT, ENTITIES_KEY).ok()?? {
        (Value::Object(ObjType::Map), id) => id,
        _ => return None,
    };
    let type_map = match doc.get(&entities, type_name).ok()?? {
        (Value::Object(ObjType::Map), id) => id,
        _ => return None,
    };
    match doc.get(&type_map, uuid.to_string()).ok()?? {
        (Value::Object(ObjType::Map), id) => Some(id),
        _ => None,
    }
}

// ── Entity lifecycle ───────────────────────────────────────────────────────

/// Ensure the entity's field map exists without writing any fields.
pub fn touch_entity(doc: &mut AutoCommit, type_name: &str, uuid: NonNilUuid) -> CrdtResult<()> {
    ensure_entity_map(doc, type_name, uuid)?;
    Ok(())
}

/// Set (or clear) the `__deleted` soft-delete flag on an entity.
pub fn put_deleted(
    doc: &mut AutoCommit,
    type_name: &str,
    uuid: NonNilUuid,
    flag: bool,
) -> CrdtResult<()> {
    let entity = ensure_entity_map(doc, type_name, uuid)?;
    doc.put(&entity, DELETED_KEY, ScalarValue::Boolean(flag))?;
    Ok(())
}

/// Check whether an entity has its `__deleted` flag set to true.
///
/// Returns `false` if the entity or flag is absent.
#[must_use]
pub fn is_deleted(doc: &AutoCommit, type_name: &str, uuid: NonNilUuid) -> bool {
    let Some(entity) = get_entity_map(doc, type_name, uuid) else {
        return false;
    };
    matches!(
        doc.get(&entity, DELETED_KEY).ok().flatten(),
        Some((Value::Scalar(sv), _)) if matches!(sv.as_ref(), ScalarValue::Boolean(true))
    )
}

/// Iterate the UUIDs stored for a given entity type, including soft-deleted.
#[must_use]
pub fn list_all_uuids(doc: &AutoCommit, type_name: &str) -> Vec<NonNilUuid> {
    let Some((Value::Object(ObjType::Map), entities)) = doc.get(&ROOT, ENTITIES_KEY).ok().flatten()
    else {
        return Vec::new();
    };
    let Some((Value::Object(ObjType::Map), type_map)) =
        doc.get(&entities, type_name).ok().flatten()
    else {
        return Vec::new();
    };
    doc.keys(&type_map)
        .filter_map(|k| Uuid::parse_str(&k).ok())
        .filter_map(NonNilUuid::new)
        .collect()
}

// ── Field write ────────────────────────────────────────────────────────────

/// Mirror a field's current value into the document, routed by `crdt_type`.
///
/// `Derived` fields are skipped silently. Other combinations of
/// `(crdt_type, value)` that don't match are reported as
/// [`CrdtError::Unsupported`].
pub fn write_field(
    doc: &mut AutoCommit,
    type_name: &str,
    uuid: NonNilUuid,
    field_name: &str,
    crdt_type: CrdtFieldType,
    value: &FieldValue,
) -> CrdtResult<()> {
    if matches!(crdt_type, CrdtFieldType::Derived) {
        return Ok(());
    }
    let entity = ensure_entity_map(doc, type_name, uuid)?;
    match (crdt_type, value) {
        (CrdtFieldType::Scalar, FieldValue::Single(item)) => {
            doc.put(&entity, field_name, item_to_scalar(item)?)?;
            Ok(())
        }
        (CrdtFieldType::Scalar, FieldValue::List(_)) => Err(CrdtError::Unsupported(format!(
            "field `{field_name}`: Scalar CrdtFieldType requires FieldValue::Single"
        ))),
        (CrdtFieldType::Text, FieldValue::Single(FieldValueItem::Text(s))) => {
            write_text(doc, &entity, field_name, s)
        }
        (CrdtFieldType::Text, _) => Err(CrdtError::Unsupported(format!(
            "field `{field_name}`: Text CrdtFieldType requires FieldValue::Single(Text)"
        ))),
        (CrdtFieldType::List, FieldValue::List(items)) => {
            write_list(doc, &entity, field_name, items)
        }
        (CrdtFieldType::List, FieldValue::Single(_)) => Err(CrdtError::Unsupported(format!(
            "field `{field_name}`: List CrdtFieldType requires FieldValue::List"
        ))),
        (CrdtFieldType::Derived, _) => unreachable!("handled above"),
    }
}

/// Clear a field from the document. Used when a field's value becomes `None`
/// (unset optional field).
pub fn clear_field(
    doc: &mut AutoCommit,
    type_name: &str,
    uuid: NonNilUuid,
    field_name: &str,
) -> CrdtResult<()> {
    let Some(entity) = get_entity_map(doc, type_name, uuid) else {
        return Ok(());
    };
    // `delete` on a missing key is an error; probe first.
    if doc.get(&entity, field_name)?.is_some() {
        doc.delete(&entity, field_name)?;
    }
    Ok(())
}

// ── Field read ─────────────────────────────────────────────────────────────

/// Read a field back from the document as a `FieldValue`, shaped by
/// `item_type` and routed by `crdt_type`. Returns `None` if the field is
/// absent.
pub fn read_field(
    doc: &AutoCommit,
    type_name: &str,
    uuid: NonNilUuid,
    field_name: &str,
    item_type: FieldTypeItem,
    crdt_type: CrdtFieldType,
) -> CrdtResult<Option<FieldValue>> {
    let Some(entity) = get_entity_map(doc, type_name, uuid) else {
        return Ok(None);
    };
    match crdt_type {
        CrdtFieldType::Derived => Ok(None),
        CrdtFieldType::Scalar => match doc.get(&entity, field_name)? {
            Some((Value::Scalar(sv), _)) => Ok(Some(FieldValue::Single(scalar_to_item(
                sv.as_ref(),
                item_type,
            )?))),
            Some((other, _)) => Err(CrdtError::TypeMismatch(format!(
                "{field_name}: expected Scalar, got {other:?}"
            ))),
            None => Ok(None),
        },
        CrdtFieldType::Text => match doc.get(&entity, field_name)? {
            Some((Value::Object(ObjType::Text), id)) => {
                let s = doc.text(&id)?;
                Ok(Some(FieldValue::Single(FieldValueItem::Text(s))))
            }
            Some((other, _)) => Err(CrdtError::TypeMismatch(format!(
                "{field_name}: expected Text, got {other:?}"
            ))),
            None => Ok(None),
        },
        CrdtFieldType::List => match doc.get(&entity, field_name)? {
            Some((Value::Object(ObjType::List), list)) => {
                let len = doc.length(&list);
                let mut out = Vec::with_capacity(len);
                for i in 0..len {
                    match doc.get(&list, i)? {
                        Some((Value::Scalar(sv), _)) => {
                            out.push(scalar_to_item(sv.as_ref(), item_type)?);
                        }
                        Some((other, _)) => {
                            return Err(CrdtError::TypeMismatch(format!(
                                "{field_name}[{i}]: expected Scalar, got {other:?}"
                            )));
                        }
                        None => {}
                    }
                }
                Ok(Some(FieldValue::List(out)))
            }
            Some((other, _)) => Err(CrdtError::TypeMismatch(format!(
                "{field_name}: expected List, got {other:?}"
            ))),
            None => Ok(None),
        },
    }
}

// ── ScalarValue conversions ────────────────────────────────────────────────

pub(crate) fn item_to_scalar(item: &FieldValueItem) -> CrdtResult<ScalarValue> {
    Ok(match item {
        FieldValueItem::String(s) | FieldValueItem::Text(s) => ScalarValue::Str(s.clone().into()),
        FieldValueItem::Integer(n) => ScalarValue::Int(*n),
        FieldValueItem::Float(v) => ScalarValue::F64(*v),
        FieldValueItem::Boolean(b) => ScalarValue::Boolean(*b),
        FieldValueItem::DateTime(dt) => {
            let millis = Utc.from_utc_datetime(dt).timestamp_millis();
            ScalarValue::Timestamp(millis)
        }
        FieldValueItem::Duration(d) => ScalarValue::Int(d.num_milliseconds()),
        FieldValueItem::EntityIdentifier(rid) => {
            ScalarValue::Str(format!("{}:{}", rid.entity_type_name(), rid.entity_uuid()).into())
        }
        FieldValueItem::AdditionalCost(c) => ScalarValue::Str(c.to_string().into()),
    })
}

pub(crate) fn scalar_to_item(
    sv: &ScalarValue,
    expected: FieldTypeItem,
) -> CrdtResult<FieldValueItem> {
    match (sv, expected) {
        (ScalarValue::Str(s), FieldTypeItem::String) => Ok(FieldValueItem::String(s.to_string())),
        (ScalarValue::Str(s), FieldTypeItem::Text) => Ok(FieldValueItem::Text(s.to_string())),
        (ScalarValue::Int(n), FieldTypeItem::Integer) => Ok(FieldValueItem::Integer(*n)),
        (ScalarValue::Uint(n), FieldTypeItem::Integer) => {
            Ok(FieldValueItem::Integer(i64::try_from(*n).map_err(|e| {
                CrdtError::TypeMismatch(format!("uint {n} does not fit in i64: {e}"))
            })?))
        }
        (ScalarValue::F64(v), FieldTypeItem::Float) => Ok(FieldValueItem::Float(*v)),
        (ScalarValue::Boolean(b), FieldTypeItem::Boolean) => Ok(FieldValueItem::Boolean(*b)),
        (ScalarValue::Timestamp(ms), FieldTypeItem::DateTime) => {
            let dt: DateTime<Utc> = Utc
                .timestamp_millis_opt(*ms)
                .single()
                .ok_or_else(|| CrdtError::TypeMismatch(format!("bad timestamp: {ms}")))?;
            Ok(FieldValueItem::DateTime(dt.naive_utc()))
        }
        (ScalarValue::Int(ms), FieldTypeItem::Duration) => {
            Ok(FieldValueItem::Duration(Duration::milliseconds(*ms)))
        }
        (ScalarValue::Str(s), FieldTypeItem::AdditionalCost) => {
            let cost = s
                .as_str()
                .parse::<crate::value::AdditionalCost>()
                .map_err(CrdtError::TypeMismatch)?;
            Ok(FieldValueItem::AdditionalCost(cost))
        }
        (ScalarValue::Str(s), FieldTypeItem::EntityIdentifier(type_name)) => {
            let (got_type, uuid_part) = s
                .as_str()
                .split_once(':')
                .ok_or_else(|| CrdtError::TypeMismatch(format!("bad entity id: {s}")))?;
            if got_type != type_name {
                return Err(CrdtError::TypeMismatch(format!(
                    "entity id type mismatch: got {got_type}, expected {type_name}"
                )));
            }
            let u = Uuid::parse_str(uuid_part)
                .map_err(|e| CrdtError::TypeMismatch(format!("bad uuid in entity id: {e}")))?;
            let nn = NonNilUuid::new(u)
                .ok_or_else(|| CrdtError::TypeMismatch("entity id is nil UUID".into()))?;
            // SAFETY: type_name comes from the field descriptor's declared
            // target type and the UUID was just validated as non-nil.
            let rid = unsafe { RuntimeEntityId::new_unchecked(nn, type_name) };
            Ok(FieldValueItem::EntityIdentifier(rid))
        }
        (sv, item) => Err(CrdtError::TypeMismatch(format!(
            "scalar {sv:?} does not match FieldTypeItem::{item}"
        ))),
    }
}

// ── Text + List write helpers ──────────────────────────────────────────────

fn write_text(
    doc: &mut AutoCommit,
    parent: &ObjId,
    field_name: &str,
    text: &str,
) -> CrdtResult<()> {
    // Replace-style bulk write. For character-granular concurrent edits,
    // callers must reach into `splice_text` directly (the edit system can do
    // so once it grows text-diff awareness).
    let obj = match doc.get(parent, field_name)? {
        Some((Value::Object(ObjType::Text), id)) => id,
        Some((other, _)) => {
            return Err(CrdtError::TypeMismatch(format!(
                "{field_name}: expected Text, got {other:?}"
            )))
        }
        None => doc.put_object(parent, field_name, ObjType::Text)?,
    };
    let current_len = doc.length(&obj);
    if current_len > 0 {
        doc.splice_text(&obj, 0, current_len as isize, "")?;
    }
    if !text.is_empty() {
        doc.splice_text(&obj, 0, 0, text)?;
    }
    Ok(())
}

fn write_list(
    doc: &mut AutoCommit,
    parent: &ObjId,
    field_name: &str,
    items: &[FieldValueItem],
) -> CrdtResult<()> {
    // Replace-style bulk write. Fine-grained add/remove for relationship
    // lists is handled by dedicated edge helpers (FEATURE-023).
    if let Some((Value::Object(ObjType::List), id)) = doc.get(parent, field_name)? {
        let len = doc.length(&id);
        for i in (0..len).rev() {
            doc.delete(&id, i)?;
        }
        for (i, it) in items.iter().enumerate() {
            doc.insert(&id, i, item_to_scalar(it)?)?;
        }
        return Ok(());
    }
    let id = doc.put_object(parent, field_name, ObjType::List)?;
    for (i, it) in items.iter().enumerate() {
        doc.insert(&id, i, item_to_scalar(it)?)?;
    }
    Ok(())
}

// ── Rehydration (load path) ────────────────────────────────────────────────

/// Rebuild a single entity from the CRDT document into the cache.
///
/// This is the generic body shared by every entity type's
/// [`RegisteredEntityType::rehydrate_fn`].  It reads every non-derived
/// writable field for `E` out of `schedule.doc()` and hands the resulting
/// `(field_name, value)` batch to [`build_entity`] with
/// [`UuidPreference::Exact`], which in turn runs validation and registers
/// the entity in `schedule.entities`.
///
/// The CRDT mirror should be disabled around the call (see
/// [`Schedule::with_mirror_disabled`](crate::schedule::Schedule::with_mirror_disabled))
/// so we don't re-emit change records against the doc we just read from.
///
/// # Errors
/// Forwards [`BuildError`] from the underlying builder — typically
/// `BuildError::Validation` if a required field was missing from the doc.
pub fn rehydrate_entity<E: EntityBuildable>(
    schedule: &mut Schedule,
    uuid: NonNilUuid,
) -> Result<NonNilUuid, BuildError> {
    // Collect (name, value) pairs while holding `&schedule.doc()`; apply
    // them through the builder after the borrow is released.
    let mut updates: Vec<FieldUpdate<E>> = Vec::new();
    for desc in E::field_set().fields() {
        if matches!(desc.crdt_type, CrdtFieldType::Derived) {
            continue;
        }
        if desc.cb.write_fn.is_none() {
            continue;
        }
        let item_type = desc.field_type().item_type();
        match read_field(
            schedule.doc(),
            E::TYPE_NAME,
            uuid,
            desc.name(),
            item_type,
            desc.crdt_type,
        ) {
            Ok(Some(v)) => updates.push(FieldUpdate {
                op: crate::field::set::FieldOp::Set,
                field: FieldRef::Name(desc.name()),
                value: v,
            }),
            Ok(None) => {}
            // Treat a per-field shape mismatch as "field not present" during
            // rehydration — the builder's validation will catch any missing
            // required field.  This makes migrations across a schema change
            // forgiving instead of catastrophic.
            Err(_) => {}
        }
    }
    build_entity::<E>(schedule, UuidPreference::Exact(uuid), updates).map(|id| id.entity_uuid())
}

// ── Extra fields (__extra nested map) ─────────────────────────────────────────

/// Key used for the extra-fields child map inside each entity map.
pub const EXTRA_KEY: &str = "__extra";

/// Get-or-create the `__extra` map for an entity.
fn ensure_extra_map(doc: &mut AutoCommit, type_name: &str, uuid: NonNilUuid) -> CrdtResult<ObjId> {
    let entity = ensure_entity_map(doc, type_name, uuid)?;
    ensure_map(doc, &entity, EXTRA_KEY)
}

/// Read the `__extra` map for an entity without creating it.
fn get_extra_map(doc: &AutoCommit, type_name: &str, uuid: NonNilUuid) -> Option<ObjId> {
    let entity = get_entity_map(doc, type_name, uuid)?;
    match doc.get(&entity, EXTRA_KEY).ok()?? {
        (Value::Object(ObjType::Map), id) => Some(id),
        _ => None,
    }
}

/// Write a string value to an extra field, creating `__extra` if needed.
pub fn write_extra_field(
    doc: &mut AutoCommit,
    type_name: &str,
    uuid: NonNilUuid,
    key: &str,
    value: &str,
) -> CrdtResult<()> {
    let extra = ensure_extra_map(doc, type_name, uuid)?;
    doc.put(&extra, key, ScalarValue::Str(value.into()))?;
    Ok(())
}

/// Delete an extra field if it exists.
pub fn delete_extra_field(
    doc: &mut AutoCommit,
    type_name: &str,
    uuid: NonNilUuid,
    key: &str,
) -> CrdtResult<()> {
    let Some(extra) = get_extra_map(doc, type_name, uuid) else {
        return Ok(());
    };
    if doc.get(&extra, key)?.is_some() {
        doc.delete(&extra, key)?;
    }
    Ok(())
}

/// Read one extra field value, or `None` if the key or map is absent.
#[must_use]
pub fn read_extra_field(
    doc: &AutoCommit,
    type_name: &str,
    uuid: NonNilUuid,
    key: &str,
) -> Option<String> {
    let extra = get_extra_map(doc, type_name, uuid)?;
    match doc.get(&extra, key).ok()?? {
        (Value::Scalar(sv), _) => match sv.as_ref() {
            ScalarValue::Str(s) => Some(s.to_string()),
            _ => None,
        },
        _ => None,
    }
}

/// Return all `(key, value)` pairs stored in the `__extra` map, or an empty
/// vec if the map does not exist.
#[must_use]
pub fn list_extra_fields(
    doc: &AutoCommit,
    type_name: &str,
    uuid: NonNilUuid,
) -> Vec<(String, String)> {
    let Some(extra) = get_extra_map(doc, type_name, uuid) else {
        return Vec::new();
    };
    doc.keys(&extra)
        .filter_map(|k| match doc.get(&extra, &k).ok()?? {
            (Value::Scalar(sv), _) => match sv.as_ref() {
                ScalarValue::Str(s) => Some((k, s.to_string())),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

pub mod edge;

pub use edge::{
    canonical_owner, ensure_owner_list, list_append_unique, list_remove_uuid, meta_field_name,
    read_edge_meta_bool, read_owner_list, write_edge_meta_bool, write_owner_list, CanonicalOwner,
};

#[cfg(test)]
mod tests {
    use super::*;

    fn test_uuid() -> NonNilUuid {
        let u = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440002").unwrap();
        NonNilUuid::new(u).unwrap()
    }

    #[test]
    fn test_crdt_field_type_variants() {
        let non_edge = [
            CrdtFieldType::Scalar,
            CrdtFieldType::Text,
            CrdtFieldType::List,
            CrdtFieldType::Derived,
        ];
        assert_eq!(non_edge.len(), 4);
    }

    #[test]
    fn test_extra_field_write_read() {
        let mut doc = AutoCommit::new();
        let uuid = test_uuid();
        write_extra_field(&mut doc, "panel", uuid, "Tech Notes", "bring extra cable").unwrap();
        assert_eq!(
            read_extra_field(&doc, "panel", uuid, "Tech Notes").as_deref(),
            Some("bring extra cable")
        );
    }

    #[test]
    fn test_extra_field_absent_returns_none() {
        let doc = AutoCommit::new();
        assert!(read_extra_field(&doc, "panel", test_uuid(), "missing").is_none());
    }

    #[test]
    fn test_extra_field_delete() {
        let mut doc = AutoCommit::new();
        let uuid = test_uuid();
        write_extra_field(&mut doc, "panel", uuid, "key", "val").unwrap();
        delete_extra_field(&mut doc, "panel", uuid, "key").unwrap();
        assert!(read_extra_field(&doc, "panel", uuid, "key").is_none());
    }

    #[test]
    fn test_extra_fields_list_and_roundtrip() {
        let mut doc = AutoCommit::new();
        let uuid = test_uuid();
        write_extra_field(&mut doc, "panel", uuid, "A", "1").unwrap();
        write_extra_field(&mut doc, "panel", uuid, "B", "2").unwrap();
        let fields = list_extra_fields(&doc, "panel", uuid);
        assert_eq!(fields.len(), 2);
        assert!(fields.iter().any(|(k, v)| k == "A" && v == "1"));
        assert!(fields.iter().any(|(k, v)| k == "B" && v == "2"));
    }

    #[test]
    fn test_extra_fields_survive_save_load() {
        let mut doc = AutoCommit::new();
        let uuid = test_uuid();
        write_extra_field(&mut doc, "panel", uuid, "Notes", "check mic").unwrap();
        let bytes = doc.save();
        let doc2 = AutoCommit::load(&bytes).unwrap();
        assert_eq!(
            read_extra_field(&doc2, "panel", uuid, "Notes").as_deref(),
            Some("check mic")
        );
    }
}
