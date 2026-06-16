/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! CRDT save/load/merge operations for [`Schedule`].

use crate::crdt::CrdtFieldType;
use crate::crdt::{self, CrdtError};
use crate::edge::{EdgeKind, FullEdge, RawEdgeMap};
use crate::entity::{registered_entity_types, EntityType, EntityUuid};
use crate::field::NamedField;
use crate::value::{FieldError, FieldValue};
use crate::EntityId;
use automerge::transaction::CommitOptions;
use automerge::AutoCommit;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::NonNilUuid;

use super::Schedule;

/// Errors returned by [`Schedule::load`] and [`Schedule::load_from_file`].
#[derive(Debug, Error)]
pub enum LoadError {
    /// The file header is missing, corrupted, or an unsupported format version.
    ///
    /// Only returned by [`Schedule::load_from_file`].
    #[error("invalid file format: {0}")]
    Format(String),

    /// The automerge byte blob could not be decoded.
    #[error("failed to decode automerge document: {0}")]
    Codec(String),

    /// Rebuilding a specific entity from the document failed — most commonly
    /// because a required field is missing after a schema migration.
    #[error("failed to rehydrate {type_name}:{uuid}: {detail}")]
    Rehydrate {
        type_name: &'static str,
        uuid: NonNilUuid,
        detail: String,
    },
}

// ── File format constants ─────────────────────────────────────────────────────

/// Magic bytes at the start of every native schedule file.
pub const FILE_MAGIC: &[u8; 6] = b"COSAM\x00";

/// Current native file format version.
pub const FILE_FORMAT_VERSION: u16 = 1;

// ── ChangeLogEntry ────────────────────────────────────────────────────────────

/// Summary of a single CRDT commit returned by [`Schedule::change_log`].
#[derive(Debug, Clone)]
pub struct ChangeLogEntry {
    /// First 8 hex characters of the change hash.
    pub hash_short: String,
    /// First 8 hex characters of the actor ID (identifies the writing process).
    pub actor_short: String,
    /// Unix timestamp in seconds as stored in the change.
    /// `0` means the committer did not set a timestamp.
    pub timestamp_secs: i64,
    /// Number of CRDT operations in this change (`0` for marker commits).
    pub ops: usize,
    /// Commit message, if any.  Marker commits written by [`Schedule::commit_marker`]
    /// always have a message; raw field-write commits generally do not.
    pub message: Option<String>,
    /// Detailed operation descriptions when using change_log_detailed().
    pub detailed_ops: Option<Vec<String>>,
}

// ── ScheduleMetadata ──────────────────────────────────────────────────────────

/// Top-level schedule identity and provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleMetadata {
    /// Globally unique schedule identity (v7, generated at [`Schedule::new`]).
    pub schedule_id: NonNilUuid,
    /// When this schedule was created (by our tooling).
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the underlying source data was last modified.
    ///
    /// For XLSX imports this is the `dcterms:modified` property from the file's
    /// `docProps/core.xml`, with a fallback to the file-system mtime.  For
    /// native `.schedule` files it is carried forward from prior saves.
    /// `None` means the timestamp was not determinable at import time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<chrono::DateTime<chrono::Utc>>,
    /// IANA timezone name (e.g. `"America/New_York"`) the schedule's naive
    /// wall-clock timestamps are expressed in.
    ///
    /// Any timestamp in the schedule that lacks an explicit zone is interpreted
    /// as being in this timezone.  `None` means no timezone has been resolved
    /// yet; tooling fills it in at import time (source meta → CLI default →
    /// system local).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    /// Optional schedule-wide start of the event window.
    ///
    /// Acts as a default lower bound that real panels can extend earlier.
    /// `None` falls back to the earliest scheduled panel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_time: Option<chrono::NaiveDateTime>,
    /// Optional schedule-wide end of the event window.
    ///
    /// Acts as a default upper bound that real panels can extend later.
    /// `None` falls back to the latest scheduled panel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_time: Option<chrono::NaiveDateTime>,
    /// Human-readable generator identifier (e.g. `"cosam-convert 0.1"`).
    pub generator: String,
    /// Monotonically increasing edit version counter.
    pub version: u32,
}

// ── CRDT save/load implementation ─────────────────────────────────────────────

impl Schedule {
    // ── CRDT save/load ────────────────────────────────────────────────────────

    /// Serialize the entire authoritative CRDT document to a compact byte
    /// blob suitable for on-disk persistence or transport.
    ///
    /// This is a pure pass-through to [`AutoCommit::save`]; the in-memory
    /// cache contributes nothing — it can be fully rebuilt from the bytes
    /// via [`Self::load`].
    pub fn save(&mut self) -> Vec<u8> {
        self.doc.save()
    }

    /// Decode an automerge document from `bytes` and rebuild a `Schedule`
    /// from it: the HashMap cache is rehydrated by replaying every
    /// non-deleted entity through its registered
    /// [`crate::entity::RegisteredEntityType::rehydrate_fn`].
    ///
    /// Metadata (`schedule_id`, `created_at`, etc.) is re-initialized for
    /// the loading process; only entity field data is round-tripped.
    /// Edge state (`RawEdgeMap`) is *not* rehydrated from the doc in this
    /// phase — see FEATURE-023 for owner-list edge storage.
    ///
    /// # Errors
    /// Returns [`LoadError::Codec`] if the bytes do not parse, or
    /// [`LoadError::Rehydrate`] if any entity fails to rebuild (typically
    /// a missing required field after a migration).
    pub fn load(bytes: &[u8]) -> Result<Self, LoadError> {
        let doc = AutoCommit::load(bytes).map_err(|e| LoadError::Codec(e.to_string()))?;
        let mut sched = Self::new();
        sched.doc = doc;
        sched.rebuild_cache_from_doc()?;
        sched.sidecar.clear();
        sched.change_tracker.clear();
        Ok(sched)
    }

    /// Serialize this schedule to the versioned native file format.
    ///
    /// The format is a binary envelope:
    ///
    /// | Offset   | Width  | Description                              |
    /// |----------|--------|------------------------------------------|
    /// | 0        | 6      | Magic: `COSAM\x00`                       |
    /// | 6        | 2      | Format version: `u16` LE (currently `1`) |
    /// | 8        | 4      | Metadata JSON length: `u32` LE           |
    /// | 12       | N      | Metadata: JSON-encoded [`ScheduleMetadata`] |
    /// | 12+N     | …      | Automerge binary document                |
    ///
    /// Metadata (schedule UUID, creation timestamp, generator, edit version)
    /// is embedded in the envelope so that [`Self::load_from_file`] can
    /// restore it exactly; this is the primary difference from the raw
    /// [`Self::save`] / [`Self::load`] pair used for CRDT sync.
    ///
    /// # Panics
    /// Panics if `ScheduleMetadata` cannot be serialized to JSON (this cannot
    /// happen in practice — all field types are always serializable).
    pub fn save_to_file(&mut self) -> Vec<u8> {
        let meta_json = serde_json::to_vec(&self.metadata)
            .expect("ScheduleMetadata serialization is infallible");
        let automerge_bytes = self.doc.save();

        let meta_len = u32::try_from(meta_json.len()).expect("metadata JSON exceeds 4 GiB");
        let mut out =
            Vec::with_capacity(FILE_MAGIC.len() + 2 + 4 + meta_json.len() + automerge_bytes.len());
        out.extend_from_slice(FILE_MAGIC);
        out.extend_from_slice(&FILE_FORMAT_VERSION.to_le_bytes());
        out.extend_from_slice(&meta_len.to_le_bytes());
        out.extend_from_slice(&meta_json);
        out.extend_from_slice(&automerge_bytes);

        // Clear ephemeral state: change tracker is reset after each successful
        // save so the next update_xlsx only sees post-save mutations.
        // The sidecar is NOT cleared here — it remains valid for update_xlsx
        // calls in the same session immediately after save.
        self.change_tracker.clear();

        out
    }

    /// Decode a schedule from the native file format, restoring both entity
    /// data (including CRDT history) and schedule metadata.
    ///
    /// This is the counterpart to [`Self::save_to_file`].  Use
    /// [`Self::load`] instead when you have raw automerge bytes from a
    /// sync operation (no metadata envelope).
    ///
    /// # Errors
    /// - [`LoadError::Format`] — bad magic, unsupported version, or
    ///   truncated / invalid metadata JSON.
    /// - [`LoadError::Codec`] — the embedded automerge document cannot be
    ///   decoded.
    /// - [`LoadError::Rehydrate`] — an entity failed to rebuild from the
    ///   document (typically a missing required field after migration).
    pub fn load_from_file(bytes: &[u8]) -> Result<Self, LoadError> {
        const HEADER_SIZE: usize = 6 + 2 + 4; // magic + version + meta_len

        if bytes.len() < HEADER_SIZE {
            return Err(LoadError::Format(
                "file too short to contain a valid header".into(),
            ));
        }
        if &bytes[..6] != FILE_MAGIC {
            return Err(LoadError::Format(
                "unrecognized file magic — not a cosam schedule file".into(),
            ));
        }
        let version = u16::from_le_bytes([bytes[6], bytes[7]]);
        if version != FILE_FORMAT_VERSION {
            return Err(LoadError::Format(format!(
                "unsupported format version {version} (this build supports version {FILE_FORMAT_VERSION})"
            )));
        }
        let meta_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
        let meta_end = HEADER_SIZE + meta_len;
        if bytes.len() < meta_end {
            return Err(LoadError::Format("metadata section is truncated".into()));
        }
        let metadata: ScheduleMetadata = serde_json::from_slice(&bytes[HEADER_SIZE..meta_end])
            .map_err(|e| LoadError::Format(format!("invalid metadata JSON: {e}")))?;

        let doc =
            AutoCommit::load(&bytes[meta_end..]).map_err(|e| LoadError::Codec(e.to_string()))?;

        let mut sched = Self::new();
        sched.doc = doc;
        sched.metadata = metadata;
        sched.rebuild_cache_from_doc()?;
        // Sidecar and change tracker start empty after load — they are
        // ephemeral and not stored in the file.
        sched.sidecar.clear();
        sched.change_tracker.clear();
        Ok(sched)
    }

    /// Apply a batch of encoded automerge changes, then rebuild the
    /// in-memory cache in full so it reflects the post-merge state.
    ///
    /// Typical usage is receiving sync bytes from a peer's
    /// [`Self::get_changes_since`] and calling `apply_changes` to integrate
    /// them into the local replica.
    ///
    /// # Errors
    /// Returns [`LoadError::Codec`] if any byte slice fails to decode as an
    /// automerge `Change`, or [`LoadError::Rehydrate`] if the post-merge
    /// cache rebuild cannot recover an entity (e.g. a required field went
    /// missing under concurrent deletes).
    pub fn apply_changes(&mut self, changes: &[Vec<u8>]) -> Result<(), LoadError> {
        let mut decoded: Vec<automerge::Change> = Vec::with_capacity(changes.len());
        for bytes in changes {
            decoded.push(
                automerge::Change::try_from(bytes.as_slice())
                    .map_err(|e| LoadError::Codec(e.to_string()))?,
            );
        }
        self.doc
            .apply_changes(decoded)
            .map_err(|e| LoadError::Codec(e.to_string()))?;
        self.rebuild_cache_from_doc()
    }

    /// The set of change hashes identifying the current head(s) of the CRDT
    /// document.
    ///
    /// Takes `&mut self` because [`AutoCommit`] flushes pending ops before
    /// reporting heads.  Callers pass the returned slice back later to
    /// [`Self::get_changes_since`] to ask "what have you observed since
    /// this snapshot?" for delta sync.
    pub fn get_heads(&mut self) -> Vec<automerge::ChangeHash> {
        self.doc.get_heads()
    }

    /// Encode every change in the doc's history as bytes.  Useful for a
    /// fresh replica bootstrap (equivalent to `save()`, but without the
    /// compressed document-level framing).
    pub fn get_changes(&mut self) -> Vec<Vec<u8>> {
        self.doc
            .get_changes(&[])
            .into_iter()
            .map(|c| c.raw_bytes().to_vec())
            .collect()
    }

    /// Encode every change the doc has observed that is *not* reachable
    /// from `have_deps`.  Used by sync-pull: the requester sends its
    /// [`Self::get_heads`] and the responder returns the delta.
    pub fn get_changes_since(&mut self, have_deps: &[automerge::ChangeHash]) -> Vec<Vec<u8>> {
        self.doc
            .get_changes(have_deps)
            .into_iter()
            .map(|c| c.raw_bytes().to_vec())
            .collect()
    }

    /// Write an empty automerge commit with `message` and the current wall-clock
    /// time as a human-readable bookmark in the change history.
    ///
    /// This is used by [`crate::xlsx::update_schedule_from_xlsx`] to record
    /// which XLSX file was imported, making the history visible via
    /// `cosam-modify log`.  The commit contains no data operations — it only
    /// serves as a labelled milestone.
    pub fn commit_marker(&mut self, message: &str) {
        let now_secs = chrono::Utc::now().timestamp();
        self.doc.empty_change(
            CommitOptions::default()
                .with_message(message)
                .with_time(now_secs),
        );
    }

    /// Return a summary of every CRDT commit in the document's history,
    /// oldest first.
    ///
    /// Entries with `ops == 0` and a `message` are marker commits written by
    /// [`Self::commit_marker`]; entries without a message are raw field-write
    /// commits from import or edit operations.
    pub fn change_log(&mut self) -> Vec<ChangeLogEntry> {
        self.doc
            .get_changes(&[])
            .into_iter()
            .map(|c| {
                let hash_bytes = c.hash().0;
                let hash_short = hash_bytes[..4].iter().map(|b| format!("{b:02x}")).collect();
                let actor_full = c.actor_id().to_string();
                let actor_short = actor_full.chars().take(8).collect();
                ChangeLogEntry {
                    hash_short,
                    actor_short,
                    timestamp_secs: c.timestamp(),
                    ops: c.len(),
                    message: c.message().cloned(),
                    detailed_ops: None,
                }
            })
            .collect()
    }

    /// Get change log with detailed operation information for each change.
    /// Shows the actual Automerge operations (Put, Insert, Delete, etc.).
    pub fn change_log_detailed(&mut self) -> Vec<ChangeLogEntry> {
        // Get all changes with their dependency hashes
        let changes: Vec<_> = self.doc.get_changes(&[]).into_iter().collect();

        // Collect all change hashes for dependency tracking
        let change_hashes: std::collections::HashSet<_> =
            changes.iter().map(|c| c.hash()).collect();

        let mut result = Vec::with_capacity(changes.len());

        for c in changes {
            let hash_bytes = c.hash().0;
            let hash_short = hash_bytes[..4].iter().map(|b| format!("{b:02x}")).collect();
            let actor_full = c.actor_id().to_string();
            let actor_short = actor_full.chars().take(8).collect();

            // Compute detailed field-level changes by comparing states
            let detailed_ops = if !c.is_empty() {
                // Get the dependencies of this change (heads before this change)
                let deps: Vec<_> = c.deps().to_vec();

                // Build diff by comparing entity states
                let diffs = self.compute_diff_for_change(&deps, c.hash(), &change_hashes);

                if diffs.is_empty() {
                    Some(vec![format!(
                        "{} operations (details unavailable)",
                        c.len()
                    )])
                } else {
                    Some(diffs)
                }
            } else {
                None
            };

            result.push(ChangeLogEntry {
                hash_short,
                actor_short,
                timestamp_secs: c.timestamp(),
                ops: c.len(),
                message: c.message().cloned(),
                detailed_ops,
            });
        }

        result
    }

    /// Compute field-level differences introduced by a specific change.
    /// Compares document state at `before_heads` with state after including this change.
    fn compute_diff_for_change(
        &mut self,
        before_heads: &[automerge::ChangeHash],
        _change_hash: automerge::ChangeHash,
        all_hashes: &std::collections::HashSet<automerge::ChangeHash>,
    ) -> Vec<String> {
        // Fork at the before state
        let Ok(before_sched) = self.fork_at_heads(before_heads) else {
            return Vec::new();
        };

        // Build current heads (before_heads + this change)
        // The after state is: all deps that aren't in before_heads, plus any new heads
        let after_heads: Vec<_> = self
            .doc
            .get_heads()
            .into_iter()
            .filter(|h| !before_heads.contains(h) || all_hashes.contains(h))
            .collect();

        let Ok(after_sched) = self.fork_at_heads(&after_heads) else {
            return Vec::new();
        };

        // Compare and generate diffs
        self.diff_schedules(&before_sched, &after_sched)
    }

    /// Compare two schedule states and generate human-readable differences.
    fn diff_schedules(&self, before: &Schedule, after: &Schedule) -> Vec<String> {
        let mut diffs = Vec::new();

        // For each registered entity type, compare entities
        for reg in crate::entity::registered_entity_types() {
            let type_name = reg.type_name;

            // Get all UUIDs from both states
            let before_uuids: std::collections::HashSet<_> =
                crdt::list_all_uuids(&before.doc, type_name)
                    .into_iter()
                    .filter(|u| !crdt::is_deleted(&before.doc, type_name, *u))
                    .collect();
            let after_uuids: std::collections::HashSet<_> =
                crdt::list_all_uuids(&after.doc, type_name)
                    .into_iter()
                    .filter(|u| !crdt::is_deleted(&after.doc, type_name, *u))
                    .collect();

            // Find added entities - show as JSON with field values
            for uuid in after_uuids.difference(&before_uuids) {
                if let Some(json) = format_entity_as_json(type_name, *uuid, after) {
                    diffs.push(format!(
                        "{type_name}[{}]: created = {json}",
                        &uuid.to_string()[..8]
                    ));
                } else {
                    diffs.push(format!(
                        "{type_name}[{}]: (created)",
                        &uuid.to_string()[..8]
                    ));
                }
            }

            // Find deleted entities - show prior contents as JSON
            for uuid in before_uuids.difference(&after_uuids) {
                if let Some(json) = format_entity_as_json(type_name, *uuid, before) {
                    diffs.push(format!(
                        "{type_name}[{}]: deleted = {json}",
                        &uuid.to_string()[..8]
                    ));
                } else {
                    diffs.push(format!(
                        "{type_name}[{}]: (deleted)",
                        &uuid.to_string()[..8]
                    ));
                }
            }

            // Find modified entities - compare field values
            for uuid in before_uuids.intersection(&after_uuids) {
                if let Some(entity_diffs) = self.diff_entity_fields(type_name, *uuid, before, after)
                {
                    diffs.extend(entity_diffs);
                }
            }
        }

        diffs.truncate(50); // Limit to avoid overwhelming output
        diffs
    }

    /// Compare field values for a specific entity between two schedules.
    fn diff_entity_fields(
        &self,
        type_name: &'static str,
        uuid: NonNilUuid,
        before: &Schedule,
        after: &Schedule,
    ) -> Option<Vec<String>> {
        use automerge::{ReadDoc, Value};

        // Get entity maps from both docs
        let before_map = crdt::get_entity_map(&before.doc, type_name, uuid)?;
        let after_map = crdt::get_entity_map(&after.doc, type_name, uuid)?;

        let mut changes = Vec::new();

        // Get all keys from both entity maps
        let before_keys: std::collections::HashSet<String> = before.doc.keys(&before_map).collect();
        let after_keys: std::collections::HashSet<String> = after.doc.keys(&after_map).collect();

        // Check all unique keys from both maps
        let all_keys: std::collections::HashSet<_> =
            before_keys.union(&after_keys).cloned().collect();

        for key in all_keys {
            // Skip internal CRDT fields
            if key.starts_with("__") {
                continue;
            }

            let before_val = before.doc.get(&before_map, &key).ok().flatten();
            let after_val = after.doc.get(&after_map, &key).ok().flatten();

            match (before_val, after_val) {
                (None, Some((val, _))) => {
                    changes.push(format!(
                        "{type_name}[{:.8}].{key}: (none) → {}",
                        uuid,
                        format_value(&val)
                    ));
                }
                (Some((Value::Scalar(old), _)), Some((Value::Scalar(new), _))) if old != new => {
                    changes.push(format!(
                        "{type_name}[{:.8}].{key}: {} → {}",
                        uuid,
                        format_scalar(old.as_ref()),
                        format_scalar(new.as_ref())
                    ));
                }
                (Some((Value::Scalar(old), _)), Some((Value::Scalar(new), _))) if old == new => {}
                (Some((old, _)), Some((new, _))) if old != new => {
                    changes.push(format!(
                        "{type_name}[{:.8}].{key}: {} → {}",
                        uuid,
                        format_value(&old),
                        format_value(&new)
                    ));
                }
                (Some(_), Some(_)) => {}
                (Some((old, _)), None) => {
                    changes.push(format!(
                        "{type_name}[{:.8}].{key}: {} → (deleted)",
                        uuid,
                        format_value(&old)
                    ));
                }
                _ => {}
            }
        }

        if changes.is_empty() {
            None
        } else {
            Some(changes)
        }
    }
}

/// Format an Automerge scalar value for display.
fn format_scalar(sv: &automerge::ScalarValue) -> String {
    match sv {
        automerge::ScalarValue::Str(s) => {
            if s.len() > 40 {
                format!("\"{}...\"", &s[..40])
            } else {
                format!("\"{}\"", s)
            }
        }
        automerge::ScalarValue::Int(i) => i.to_string(),
        automerge::ScalarValue::Uint(u) => u.to_string(),
        automerge::ScalarValue::F64(f) => f.to_string(),
        automerge::ScalarValue::Boolean(b) => b.to_string(),
        automerge::ScalarValue::Null => "null".to_string(),
        automerge::ScalarValue::Timestamp(t) => format!("@{}", t),
        automerge::ScalarValue::Counter(c) => c.to_string(),
        _ => format!("{:?}", sv),
    }
}

/// Format any Automerge value for display.
fn format_value(val: &automerge::Value) -> String {
    match val {
        automerge::Value::Scalar(sv) => format_scalar(sv.as_ref()),
        automerge::Value::Object(_) => "(object)".to_string(),
    }
}

/// Format entity fields as a compact JSON representation.
fn format_entity_as_json(type_name: &str, uuid: NonNilUuid, schedule: &Schedule) -> Option<String> {
    use automerge::ReadDoc;

    let entity_map = crdt::get_entity_map(&schedule.doc, type_name, uuid)?;

    // Collect field names and values
    let mut fields: std::collections::HashMap<String, serde_json::Value> =
        std::collections::HashMap::new();

    for key in schedule.doc.keys(&entity_map) {
        // Skip internal CRDT fields
        if key.starts_with("__") {
            continue;
        }

        if let Ok(Some((val, _))) = schedule.doc.get(&entity_map, &key) {
            let json_val = automerge_value_to_json(&val);
            fields.insert(key, json_val);
        }
    }

    if fields.is_empty() {
        None
    } else {
        serde_json::to_string(&fields).ok()
    }
}

/// Convert an Automerge Value to a serde_json::Value.
fn automerge_value_to_json(val: &automerge::Value) -> serde_json::Value {
    match val {
        automerge::Value::Scalar(sv) => match sv.as_ref() {
            automerge::ScalarValue::Str(s) => serde_json::Value::String(s.to_string()),
            automerge::ScalarValue::Int(i) => serde_json::Value::Number((*i).into()),
            automerge::ScalarValue::Uint(u) => serde_json::Value::Number((*u).into()),
            automerge::ScalarValue::F64(f) => serde_json::json!(*f),
            automerge::ScalarValue::Boolean(b) => serde_json::Value::Bool(*b),
            automerge::ScalarValue::Null => serde_json::Value::Null,
            automerge::ScalarValue::Bytes(b) => serde_json::json!(b),
            automerge::ScalarValue::Timestamp(t) => serde_json::json!(t),
            automerge::ScalarValue::Counter(c) => serde_json::json!(c),
            _ => serde_json::json!(format!("{:?}", sv)),
        },
        automerge::Value::Object(_) => serde_json::Value::String("(object)".to_string()),
    }
}

impl Schedule {
    /// Surface every concurrent value for a scalar field on `id`.
    ///
    /// Returns **all** concurrent writers' values when two or more
    /// replicas wrote different scalars without either observing the
    /// other; the primary read (via `field_set`) continues to return
    /// automerge's deterministically-selected LWW winner.
    ///
    /// Only scalar fields are supported; derived, text, and list fields
    /// yield an empty vec (they have their own per-character or
    /// per-item conflict semantics).
    pub fn conflicts_for<E: EntityType>(
        &self,
        id: EntityId<E>,
        field_name: &'static str,
    ) -> Vec<FieldValue> {
        use automerge::{ReadDoc, Value};

        let Some(desc) = E::field_set().get_by_name(field_name) else {
            return Vec::new();
        };
        if !matches!(desc.crdt_type(), CrdtFieldType::Scalar) {
            return Vec::new();
        }
        let Some(entity_map) = crdt::get_entity_map(&self.doc, E::TYPE_NAME, id.entity_uuid())
        else {
            return Vec::new();
        };
        let Ok(values) = self.doc.get_all(&entity_map, desc.name()) else {
            return Vec::new();
        };
        let item_type = desc.field_type().item_type();
        values
            .into_iter()
            .filter_map(|(value, _obj_id)| match value {
                Value::Scalar(sv) => crdt::scalar_to_item(&sv, item_type)
                    .ok()
                    .map(FieldValue::Single),
                _ => None,
            })
            .collect()
    }

    /// Create a new `Schedule` whose CRDT document is forked at `heads`.
    ///
    /// The returned schedule represents the state the document was in when
    /// `heads` were the current tips — all changes after `heads` are absent.
    /// The in-memory cache is fully rebuilt from the forked document.
    ///
    /// Callers should ensure any pending auto-commit ops are flushed (e.g. by
    /// calling [`Self::get_heads`]) before invoking this so the fork sees a
    /// consistent snapshot.
    ///
    /// # Errors
    /// Returns [`LoadError::Codec`] if `heads` are not reachable in the
    /// document, or [`LoadError::Rehydrate`] if cache rebuild fails.
    pub fn fork_at_heads(&mut self, heads: &[automerge::ChangeHash]) -> Result<Self, LoadError> {
        // Flush any pending auto-commit ops so the fork is consistent.
        let _ = self.doc.get_heads();
        let forked_doc = self
            .doc
            .fork_at(heads)
            .map_err(|e| LoadError::Codec(e.to_string()))?;
        let mut sched = Self::new();
        sched.doc = forked_doc;
        sched.metadata = self.metadata.clone();
        sched.rebuild_cache_from_doc()?;
        // Ephemeral state is not meaningful at a prior point in time.
        sched.sidecar.clear();
        sched.change_tracker.clear();
        Ok(sched)
    }

    /// Merge `other`'s automerge document into this one and rebuild the
    /// cache to the unified state.  Both replicas remain usable — this is
    /// a symmetric join, not a move.
    ///
    /// # Errors
    /// Returns [`LoadError::Codec`] from automerge merge, or
    /// [`LoadError::Rehydrate`] from the post-merge cache rebuild.
    pub fn merge(&mut self, other: &mut Self) -> Result<(), LoadError> {
        self.doc
            .merge(&mut other.doc)
            .map_err(|e| LoadError::Codec(e.to_string()))?;
        self.rebuild_cache_from_doc()
    }

    /// Discard the in-memory cache and fully reconstitute it from the
    /// current CRDT document.  Used by `load` / `apply_changes` / `merge`.
    ///
    /// Runs under [`Self::with_mirror_disabled`] so replayed entity and
    /// edge writes don't emit redundant changes against the doc we just
    /// read from.
    fn rebuild_cache_from_doc(&mut self) -> Result<(), LoadError> {
        // Wipe the cache — merge can resurrect soft-deleted uuids (add-wins
        // against a delete), retarget edges, and generally change which
        // entities exist.  Rebuilding from scratch is simpler and cheaper
        // than reconciling entry-by-entry.
        self.entities.clear();
        self.edges = RawEdgeMap::default();

        // Snapshot (type_name, rehydrate_fn, uuids) under an immutable
        // borrow of the doc, then apply each rehydrate with the mirror
        // disabled.  Collecting up-front avoids reborrowing the inventory
        // iterator while we mutate `self`.
        struct RehydrateWork {
            type_name: &'static str,
            rehydrate_fn: fn(
                &mut Schedule,
                NonNilUuid,
            ) -> Result<NonNilUuid, crate::edit::builder::BuildError>,
            uuids: Vec<NonNilUuid>,
        }
        let mut work: Vec<RehydrateWork> = Vec::new();
        for reg in registered_entity_types() {
            let uuids: Vec<NonNilUuid> = crdt::list_all_uuids(&self.doc, reg.type_name)
                .into_iter()
                .filter(|u| !crdt::is_deleted(&self.doc, reg.type_name, *u))
                .collect();
            if !uuids.is_empty() {
                work.push(RehydrateWork {
                    type_name: reg.type_name,
                    rehydrate_fn: reg.rehydrate_fn,
                    uuids,
                });
            }
        }

        self.with_mirror_disabled(|s| {
            for item in work {
                for uuid in item.uuids {
                    (item.rehydrate_fn)(s, uuid).map_err(|e| LoadError::Rehydrate {
                        type_name: item.type_name,
                        uuid,
                        detail: e.to_string(),
                    })?;
                }
            }
            s.rebuild_edges_from_doc();
            Ok(())
        })
    }

    /// Replay every canonical owner's relationship-list field into the
    /// in-memory [`RawEdgeMap`].
    ///
    /// Iterates every field in the inventory whose `edge_kind` is
    /// [`EdgeKind::Owner`], iterates every live owner uuid in the
    /// doc, reads the list, and `add_edge`s each endpoint pair into the cache.
    /// The caller is responsible for running this under
    /// [`Self::with_mirror_disabled`] — otherwise each replayed edge would
    /// re-write the same list back into the doc.
    fn rebuild_edges_from_doc(&mut self) {
        use crate::value::FieldTypeItem;

        // Snapshot the `(owner_uuid, target_uuids)` pairs while borrowing
        // `&self.doc`, then apply them under `&mut self`.
        struct EdgeBatch {
            owner_field: &'static crate::edge::HalfEdgeDescriptor,
            target_field: &'static crate::edge::HalfEdgeDescriptor,
            pairs: Vec<(NonNilUuid, Vec<NonNilUuid>)>,
        }
        let mut batches: Vec<EdgeBatch> = Vec::new();
        for collected in crate::field::all_named_fields() {
            let Some(owner_nf) = collected.try_as_half_edge() else {
                continue;
            };
            let EdgeKind::Owner {
                target_field: target_nf,
                ..
            } = owner_nf.edge_kind
            else {
                continue;
            };
            let owner_type = owner_nf.entity_type_name();
            let field_name = owner_nf.name();
            let target_type = target_nf.entity_type_name();
            let owner_field = owner_nf;
            let target_field = target_nf;
            let owner_uuids = crdt::list_all_uuids(&self.doc, owner_type);
            let mut pairs: Vec<(NonNilUuid, Vec<NonNilUuid>)> = Vec::new();
            for owner_uuid in owner_uuids {
                if crdt::is_deleted(&self.doc, owner_type, owner_uuid) {
                    continue;
                }
                let targets = crate::crdt::edge::read_owner_list(
                    &self.doc,
                    owner_type,
                    owner_uuid,
                    field_name,
                    FieldTypeItem::EntityIdentifier(target_type),
                );
                if !targets.is_empty() {
                    pairs.push((owner_uuid, targets));
                }
            }
            if !pairs.is_empty() {
                batches.push(EdgeBatch {
                    owner_field,
                    target_field,
                    pairs,
                });
            }
        }
        for batch in batches {
            for (owner_uuid, targets) in batch.pairs {
                // SAFETY: owner_uuid and target_uuid come from edge descriptors which
                // guarantee type compatibility with their respective fields.
                let owner_type = batch.owner_field.entity_type_name();
                let target_type = batch.target_field.entity_type_name();
                let from = unsafe {
                    crate::entity::RuntimeEntityId::new_unchecked(owner_uuid, owner_type)
                };
                let edge = FullEdge {
                    near: batch.owner_field,
                    far: batch.target_field,
                };
                let targets: Vec<crate::entity::RuntimeEntityId> = targets
                    .into_iter()
                    .map(|target_uuid| unsafe {
                        crate::entity::RuntimeEntityId::new_unchecked(target_uuid, target_type)
                    })
                    .collect();
                self.edges
                    .add_edge(from, edge, targets)
                    .expect("edge type validation failed");
            }
        }
    }

    // ── CRDT access ───────────────────────────────────────────────────────────

    /// Update the modification timestamp to the current UTC time.
    ///
    /// Called by [`crate::edit::context::EditContext`] after every successful
    /// [`apply`](crate::edit::context::EditContext::apply),
    /// [`undo`](crate::edit::context::EditContext::undo), and
    /// [`redo`](crate::edit::context::EditContext::redo).
    ///
    /// Import and CRDT-hydration paths bypass `EditContext` and must **not** call
    /// this — they manage `modified_at` directly from the source timestamp.
    pub fn touch_modified(&mut self) {
        self.metadata.modified_at = Some(chrono::Utc::now());
    }

    /// Borrow the underlying CRDT document (for change-tracking / save).
    #[must_use]
    pub fn doc(&self) -> &AutoCommit {
        &self.doc
    }

    /// Mutable access to the CRDT document — restricted to crate-internal
    /// helpers (edit commands, edge mirroring, load path).
    #[allow(dead_code)] // wired in FEATURE-022 part 2 (load/save) and FEATURE-023
    pub(crate) fn doc_mut(&mut self) -> &mut AutoCommit {
        &mut self.doc
    }

    /// Whether field writes currently mirror to the CRDT document.
    #[must_use]
    pub fn mirror_enabled(&self) -> bool {
        self.mirror_enabled
    }

    /// Run `f` with the CRDT mirror temporarily disabled. Used by the load
    /// path to rehydrate the cache without re-emitting CRDT operations.
    pub(crate) fn with_mirror_disabled<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let prev = self.mirror_enabled;
        self.mirror_enabled = false;
        let out = f(self);
        self.mirror_enabled = prev;
        out
    }

    /// Mirror every non-derived field of entity `id` into the CRDT document.
    ///
    /// Called by [`Self::insert`] immediately after the cache is populated.
    /// No-op when [`Self::mirror_enabled`] is false.
    pub(crate) fn mirror_entity_fields<E: EntityType>(
        &mut self,
        id: EntityId<E>,
    ) -> Result<(), CrdtError> {
        if !self.mirror_enabled {
            return Ok(());
        }
        let uuid = id.entity_uuid();
        let type_name = E::TYPE_NAME;
        crdt::touch_entity(&mut self.doc, type_name, uuid)?;
        // Collect (name, crdt_type, value) while holding `&self`, then apply
        // writes while holding `&mut self.doc`.
        let mut pending: Vec<(&'static str, CrdtFieldType, FieldValue)> = Vec::new();
        for desc in E::field_set().fields() {
            if !matches!(
                desc.crdt_type,
                CrdtFieldType::Scalar | CrdtFieldType::Text | CrdtFieldType::List
            ) {
                continue;
            }
            if let Ok(Some(v)) = desc.read(id, self) {
                pending.push((desc.name(), desc.crdt_type, v));
            }
        }
        for (name, crdt_type, v) in pending {
            crdt::write_field(&mut self.doc, type_name, uuid, name, crdt_type, &v)?;
        }
        // Ensure the entity is not marked deleted (idempotent on re-insert).
        crdt::put_deleted(&mut self.doc, type_name, uuid, false)?;
        // Pre-create every canonical owner-list field on this entity so
        // concurrent replicas share the same list `ObjId` and
        // `edge_add`/`edge_remove` can converge via incremental
        // `insert`/`delete` (FEATURE-023).
        //
        // We derive which fields own edges directly from the `Owner`
        // variant in each field descriptor, avoiding a global descriptor scan.
        for desc in E::field_set().half_edges() {
            if matches!(desc.edge_kind, EdgeKind::Owner { .. }) {
                crate::crdt::edge::ensure_owner_list(&mut self.doc, type_name, uuid, desc.name())?;
            }
        }
        Ok(())
    }

    /// Push a single post-write field value into the CRDT document.
    ///
    /// Invoked from [`crate::field::FieldDescriptor::write`] after the inner
    /// write succeeds.  `None` clears the field from the doc (corresponds to
    /// an unset optional field).
    ///
    /// # Errors
    /// Returns [`FieldError::Crdt`] if the mirror write fails.
    pub(crate) fn mirror_field_value<E: EntityType>(
        &mut self,
        id: EntityId<E>,
        name: &'static str,
        crdt_type: CrdtFieldType,
        value: Option<&FieldValue>,
    ) -> Result<(), FieldError> {
        if !self.mirror_enabled || matches!(crdt_type, CrdtFieldType::Derived) {
            return Ok(());
        }
        let uuid = id.entity_uuid();
        let type_name = E::TYPE_NAME;
        let res = match value {
            Some(v) => crdt::write_field(&mut self.doc, type_name, uuid, name, crdt_type, v),
            None => crdt::clear_field(&mut self.doc, type_name, uuid, name),
        };
        res.map_err(|e| FieldError::Crdt {
            name,
            detail: e.to_string(),
        })?;
        // Track that this entity was modified (unless it's already Added/Deleted).
        self.mark_entity_changed(uuid, crate::sidecar::ChangeState::Modified);
        Ok(())
    }
}
