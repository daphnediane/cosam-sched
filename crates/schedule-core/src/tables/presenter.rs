/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Presenter entity — represents a guest, panelist, staff member, or group.
//!
//! Three structs define the Presenter entity:
//!
//! - [`PresenterCommonData`] — user-facing fields (name, rank, bio, group flags)
//! - [`PresenterInternalData`] — `EntityType::InternalData`
//! - [`PresenterData`] — export/API view including flattened edge relationships
//!
//! Groups and group membership are modeled as edges between presenters,
//! accessed through `Schedule::edges_from` / `Schedule::edges_to`.
//! Tagged credit-string resolution (`[Kind:]Name[=Group]`) is implemented
//! by `find_tagged_presenter` and `find_or_create_tagged_presenter`.

use crate::accessor_field_properties;
use crate::callback_field_properties;
use crate::crdt::CrdtFieldType;
use crate::entity::{EntityId, EntityType, EntityUuid, FieldSet};
use crate::field::{CollectedField, CollectedHalfEdge, FieldDescriptor, NamedField};
use crate::field_value;
use crate::query::converter::EntityStringResolver;
use crate::query::lookup::{EntityMatcher, MatchPriority};
use crate::schedule::Schedule;
use crate::tables::panel::{self, PanelEntityType, PanelId};
use crate::value::{ConversionError, FieldValue, ValidationError};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::LazyLock;

// ── Type aliases ──────────────────────────────────────────────────────────────

/// Type-safe identifier for Presenter entities.
pub type PresenterId = EntityId<PresenterEntityType>;

// ── PresenterRank ─────────────────────────────────────────────────────────────

/// Presenter classification tier, used for both display credits and import
/// column layout.
///
/// `InvitedGuest` carries an optional custom display label: `None` serializes
/// as `"invited_panelist"`; `Some(label)` serializes as the label string
/// directly (e.g. `"Sponsor"`, `"105th"`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PresenterRank {
    Guest,
    Judge,
    Staff,
    InvitedGuest(Option<String>),
    /// Presenter who is a regular panelist (prefix `P`).
    #[default]
    Panelist,
    /// Fan panelist (prefix `F`).
    FanPanelist,
}

impl PresenterRank {
    /// Canonical lower-case string representation used in JSON and the field
    /// system. `InvitedGuest(Some(label))` preserves the custom label verbatim.
    pub fn as_str(&self) -> &str {
        match self {
            PresenterRank::Guest => "guest",
            PresenterRank::Judge => "judge",
            PresenterRank::Staff => "staff",
            PresenterRank::InvitedGuest(None) => "invited_panelist",
            PresenterRank::InvitedGuest(Some(s)) => s.as_str(),
            PresenterRank::Panelist => "panelist",
            PresenterRank::FanPanelist => "fan_panelist",
        }
    }

    /// Numeric priority: lower value = higher rank tier. Used to resolve
    /// conflicts between schedule-prefix rank and People-sheet classification.
    #[must_use]
    pub fn priority(&self) -> u8 {
        match self {
            PresenterRank::Guest => 0,
            PresenterRank::Judge => 1,
            PresenterRank::Staff => 2,
            PresenterRank::InvitedGuest(_) => 3,
            PresenterRank::Panelist => 4,
            PresenterRank::FanPanelist => 5,
        }
    }

    /// Parse a canonical rank string. Unknown values are preserved as
    /// `InvitedGuest(Some(label))` so custom labels round-trip intact.
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "guest" => PresenterRank::Guest,
            "judge" => PresenterRank::Judge,
            "staff" => PresenterRank::Staff,
            "invited_guest" | "invited_panelist" | "invitedpanelist" => {
                PresenterRank::InvitedGuest(None)
            }
            "panelist" => PresenterRank::Panelist,
            "fan_panelist" | "fanpanelist" => PresenterRank::FanPanelist,
            _ => PresenterRank::InvitedGuest(Some(s.to_string())),
        }
    }

    /// Single-character column prefix used in XLSX presenter column headers.
    #[must_use]
    pub fn prefix_char(&self) -> char {
        match self {
            PresenterRank::Guest => 'G',
            PresenterRank::Judge => 'J',
            PresenterRank::Staff => 'S',
            PresenterRank::InvitedGuest(_) => 'I',
            PresenterRank::Panelist => 'P',
            PresenterRank::FanPanelist => 'F',
        }
    }

    /// Map the single-character column prefix back to a rank. Case-insensitive.
    ///
    /// | Char    | Rank                   |
    /// |---------|------------------------|
    /// | `G`/`g` | `Guest`                |
    /// | `J`/`j` | `Judge`                |
    /// | `S`/`s` | `Staff`                |
    /// | `I`/`i` | `InvitedGuest(None)`   |
    /// | `P`/`p` | `Panelist`             |
    /// | `F`/`f` | `FanPanelist`          |
    #[must_use]
    pub fn from_prefix_char(c: char) -> Option<Self> {
        match c.to_ascii_uppercase() {
            'G' => Some(PresenterRank::Guest),
            'J' => Some(PresenterRank::Judge),
            'S' => Some(PresenterRank::Staff),
            'I' => Some(PresenterRank::InvitedGuest(None)),
            'P' => Some(PresenterRank::Panelist),
            'F' => Some(PresenterRank::FanPanelist),
            _ => None,
        }
    }

    /// All standard ranks in priority order, used for column layout.
    /// `InvitedGuest(None)` represents the entire invited tier.
    #[must_use]
    pub fn standard_ranks() -> &'static [PresenterRank] {
        &[
            PresenterRank::Guest,
            PresenterRank::Judge,
            PresenterRank::Staff,
            PresenterRank::InvitedGuest(None),
            PresenterRank::Panelist,
            PresenterRank::FanPanelist,
        ]
    }
}

impl std::fmt::Display for PresenterRank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Serialize for PresenterRank {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for PresenterRank {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(PresenterRank::parse(&s))
    }
}

// ── PresenterSortRank ─────────────────────────────────────────────────────────

/// Ordering key for a presenter, recording where it was first defined during
/// import.
///
/// - `column_index`: 0 for the People table, schedule column number otherwise.
/// - `row_index`: row in the People table, or position in a comma-separated
///   presenter list on the schedule sheet.
/// - `member_index`: position within a group's member list (0 for the group
///   itself or for standalone presenters; 1+ for individual members).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresenterSortRank {
    pub column_index: u32,
    pub row_index: u32,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub member_index: u32,
}

fn is_zero_u32(v: &u32) -> bool {
    *v == 0
}

// ── PresenterCommonData ───────────────────────────────────────────────────────

/// User-facing presenter fields. Serializable and represents the data as
/// stored/imported from the People sheet.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresenterCommonData {
    /// Full display name (required, indexed).
    pub name: String,

    /// Presenter classification tier.
    #[serde(default)]
    pub rank: PresenterRank,

    /// Biography or description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,

    /// Marks this entity as an explicit group (vs. an individual presenter).
    /// The computed `is_group` field also returns `true` when the presenter has
    /// members via edge-backed membership (checked via `edges_to`).
    #[serde(default)]
    pub is_explicit_group: bool,

    /// Always display this member under its group name, never individually.
    #[serde(default)]
    pub always_grouped: bool,

    /// Always show the group name even with partial member attendance.
    #[serde(default)]
    pub always_shown_in_group: bool,

    /// Import ordering key (column/row/member index).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_rank: Option<PresenterSortRank>,
}

impl PresenterCommonData {
    fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if self.name.is_empty() {
            errors.push(ValidationError::Required { field: "name" });
        }
        errors
    }
}

// ── PresenterInternalData ─────────────────────────────────────────────────────

/// Runtime storage struct; the field system operates on this.
#[derive(Debug, Clone)]
pub struct PresenterInternalData {
    pub id: PresenterId,
    pub data: PresenterCommonData,
}

// ── PresenterData ─────────────────────────────────────────────────────────────

/// Export/API view produced by [`PresenterEntityType::export`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresenterData {
    #[serde(flatten)]
    pub data: PresenterCommonData,
    /// Groups this presenter belongs to — from edge maps.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub group_ids: Vec<PresenterId>,
    /// Panels this presenter is on — from edge maps.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub panels: Vec<PanelId>,
}

// ── PresenterEntityType ───────────────────────────────────────────────────────

/// Singleton type representing the Presenter entity kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PresenterEntityType;

impl EntityType for PresenterEntityType {
    type InternalData = PresenterInternalData;
    type Data = PresenterData;

    const TYPE_NAME: &'static str = "presenter";

    fn uuid_namespace() -> &'static uuid::Uuid {
        static NS: LazyLock<uuid::Uuid> =
            LazyLock::new(|| uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, b"presenter"));
        &NS
    }

    fn field_set() -> &'static FieldSet<Self> {
        &PRESENTER_FIELD_SET
    }

    fn export(internal: &Self::InternalData) -> Self::Data {
        PresenterData {
            data: internal.data.clone(),
            group_ids: Vec::new(),
            panels: Vec::new(),
        }
    }

    fn validate(internal: &Self::InternalData) -> Vec<ValidationError> {
        internal.data.validate()
    }
}

impl PresenterEntityType {
    /// Find the best-matching presenter by name.
    ///
    /// Uses `match_entity` against all stored presenters.
    /// Returns the presenter with the highest `MatchPriority`, or `None` if
    /// no presenter matches.
    pub fn find_by_name(schedule: &crate::schedule::Schedule, name: &str) -> Option<PresenterId> {
        let mut best: Option<(PresenterId, MatchPriority)> = None;
        for (id, data) in schedule.iter_entities::<Self>() {
            if let Some(priority) = Self::match_entity(name, data) {
                let is_better = match &best {
                    None => true,
                    Some((_, best_p)) => priority > *best_p,
                };
                if is_better {
                    best = Some((id, priority));
                }
            }
        }
        best.map(|(id, _)| id)
    }

    /// Find all presenters matching a name, with their priorities.
    pub fn find_all_by_name(
        schedule: &crate::schedule::Schedule,
        name: &str,
    ) -> Vec<(PresenterId, MatchPriority)> {
        let mut results = Vec::new();
        for (id, data) in schedule.iter_entities::<Self>() {
            if let Some(priority) = Self::match_entity(name, data) {
                results.push((id, priority));
            }
        }
        results.sort_by_key(|b| std::cmp::Reverse(b.1));
        results
    }
}

inventory::submit! {
    crate::entity::RegisteredEntityType {
        type_name: PresenterEntityType::TYPE_NAME,
        uuid_namespace: PresenterEntityType::uuid_namespace,
        type_id: || std::any::TypeId::of::<PresenterInternalData>(),
        read_field_fn: |schedule, uuid, field_name| {
            // SAFETY: uuid came from an existing PresenterEntityType entity.
            let id = unsafe { crate::entity::EntityId::<PresenterEntityType>::new_unchecked(uuid) };
            PresenterEntityType::field_set().read_field_value(field_name, id, schedule)
        },
        write_field_fn: |schedule, uuid, field_name, value| {
            // SAFETY: uuid came from an existing PresenterEntityType entity.
            let id = unsafe { crate::entity::EntityId::<PresenterEntityType>::new_unchecked(uuid) };
            PresenterEntityType::field_set().write_field_value(field_name, id, schedule, value)
        },
        build_fn: |schedule, uuid, fields| {
            crate::edit::builder::build_entity::<PresenterEntityType>(
                schedule,
                crate::entity::UuidPreference::Exact(uuid),
                fields
                    .iter()
                    .map(|(n, v)| crate::field::set::FieldUpdate {
                        op: crate::field::set::FieldOp::Set,
                        field: crate::field::set::FieldRef::Name(n),
                        value: v.clone(),
                    })
                    .collect(),
            )
            .map(|id| id.entity_uuid())
        },
        snapshot_fn: |schedule, uuid| {
            // SAFETY: uuid came from an existing PresenterEntityType entity.
            let id = unsafe { crate::entity::EntityId::<PresenterEntityType>::new_unchecked(uuid) };
            PresenterEntityType::field_set()
                .fields()
                .filter(|d| d.cb.read_fn.is_some() && d.cb.write_fn.is_some())
                .filter_map(|d| {
                    d.read(id, schedule).ok().flatten().map(|v| (d.name(), v))
                })
                .collect()
        },
        remove_fn: |schedule, uuid| {
            // SAFETY: uuid came from an existing PresenterEntityType entity.
            let id = unsafe { crate::entity::EntityId::<PresenterEntityType>::new_unchecked(uuid) };
            schedule.remove_entity::<PresenterEntityType>(id);
        },
        rehydrate_fn: |schedule, uuid| {
            crate::crdt::rehydrate_entity::<PresenterEntityType>(schedule, uuid)
        },
    }
}

// ── EntityBuildable ─────────────────────────────────────────────────────────────

impl crate::edit::builder::EntityBuildable for PresenterEntityType {
    fn default_data(id: EntityId<Self>) -> Self::InternalData {
        PresenterInternalData {
            id,
            data: PresenterCommonData::default(),
        }
    }
}

// ── Tagged presenter lookup functions ─────────────────────────────────────────

/// Parsed representation of a tagged presenter credit string.
///
/// Format: `[Kind:][ < ]Name[ = [ = ]Group]`
///
/// - `Kind:` — one or more rank prefix chars (`G`/`J`/`S`/`I`/`P`/`F`);
///   highest-priority rank among them is used.
/// - `<Name` — sets `always_grouped = true` on the member.
/// - `=Group` — links member to a group; group becomes `is_explicit_group`.
/// - `==Group` — same, and also sets `always_shown_in_group = true` on the group.
/// - Empty name or name == group (case-insensitive) → group-only form; returns
///   the group's `PresenterId` rather than a member.
struct ParsedTag<'a> {
    required_rank: Option<PresenterRank>,
    name: &'a str,
    group_name: Option<&'a str>,
    always_grouped: bool,
    always_shown: bool,
}

impl<'a> ParsedTag<'a> {
    fn is_group_only(&self) -> bool {
        self.name.is_empty()
            || self
                .group_name
                .is_some_and(|g| g.eq_ignore_ascii_case(self.name))
    }
}

fn parse_tag(input: &str) -> ParsedTag<'_> {
    // Parse optional Kind: prefix (one or more alpha chars followed by ':')
    let (required_rank, rest) = match input.find(':') {
        Some(colon) if colon > 0 && input[..colon].chars().all(|c| c.is_alphabetic()) => {
            let flag_str = &input[..colon];
            let mut best: Option<PresenterRank> = None;
            let mut valid = true;
            for c in flag_str.chars() {
                match PresenterRank::from_prefix_char(c) {
                    Some(rank) => {
                        best = Some(match best {
                            None => rank,
                            Some(b) if rank.priority() < b.priority() => rank,
                            Some(b) => b,
                        });
                    }
                    None => {
                        valid = false;
                        break;
                    }
                }
            }
            if valid {
                (best, input[colon + 1..].trim())
            } else {
                (None, input)
            }
        }
        _ => (None, input),
    };

    // Split on first '=' to get name_raw and group_part
    let (name_raw, group_part) = match rest.find('=') {
        Some(eq) => (&rest[..eq], Some(&rest[eq + 1..])),
        None => (rest, None),
    };

    // Strip '<' from name → always_grouped
    let (name, always_grouped) = match name_raw.trim().strip_prefix('<') {
        Some(stripped) => (stripped.trim(), true),
        None => (name_raw.trim(), false),
    };

    // Strip leading '=' from group_part → always_shown
    let (group_name, always_shown) = match group_part {
        None => (None, false),
        Some(g) => match g.strip_prefix('=') {
            Some(stripped) => {
                let gn = stripped.trim();
                ((!gn.is_empty()).then_some(gn), true)
            }
            None => {
                let gn = g.trim();
                ((!gn.is_empty()).then_some(gn), false)
            }
        },
    };

    ParsedTag {
        required_rank,
        name,
        group_name,
        always_grouped,
        always_shown,
    }
}

/// Return `true` if `id` acts as a group: either `is_explicit_group` flag is set
/// or the presenter has at least one member via the homogeneous edge map.
fn is_group_entity(schedule: &crate::schedule::Schedule, id: PresenterId) -> bool {
    schedule
        .get_internal::<PresenterEntityType>(id)
        .is_some_and(|d| d.data.is_explicit_group)
        || {
            // Check if this presenter has any members (edges from FIELD_MEMBERS pointing to any members)
            !schedule.connected_field_nodes(id, EDGE_MEMBERS).is_empty()
        }
}

/// Find a group presenter matching `name`, using `is_group_entity` as the filter.
fn find_group_by_name(schedule: &crate::schedule::Schedule, name: &str) -> Option<PresenterId> {
    PresenterEntityType::find_all_by_name(schedule, name)
        .into_iter()
        .find_map(|(id, _)| is_group_entity(schedule, id).then_some(id))
}

/// Find a presenter by tagged credit string; does not create entities.
///
/// Does not handle UUID strings — callers should resolve UUIDs before calling
/// (see `lookup` in the `lookup` module).
///
/// Returns `None` when:
/// - The tagged string is empty.
/// - No matching presenter / group is found.
/// - The found presenter's rank is strictly lower (higher priority number) than
///   the required `Kind:` prefix rank.
/// - A `=Group` suffix is given but the found presenter is not a member.
pub fn find_tagged_presenter(
    schedule: &crate::schedule::Schedule,
    tagged: &str,
) -> Option<PresenterId> {
    let tagged = tagged.trim();
    if tagged.is_empty() {
        return None;
    }

    let parsed = parse_tag(tagged);

    let found_id = if parsed.is_group_only() {
        let group_name = parsed
            .group_name
            .or((!parsed.name.is_empty()).then_some(parsed.name))?;
        find_group_by_name(schedule, group_name)?
    } else {
        let id = PresenterEntityType::find_by_name(schedule, parsed.name)?;
        // Verify group membership if a group suffix is given
        if let Some(group_name) = parsed.group_name {
            let in_group = schedule
                .connected_field_nodes(id, EDGE_GROUPS)
                .into_iter()
                .map(|e| unsafe { PresenterId::new_unchecked(e.entity_uuid()) })
                .any(|gid| {
                    schedule
                        .get_internal::<PresenterEntityType>(gid)
                        .is_some_and(|d| d.data.name.eq_ignore_ascii_case(group_name))
                });
            if !in_group {
                return None;
            }
        }
        id
    };

    // Rank gate: found rank must be at least as high as required (lower priority number)
    if let Some(ref req) = parsed.required_rank {
        let found_priority = schedule
            .get_internal::<PresenterEntityType>(found_id)
            .map_or(u8::MAX, |d| d.data.rank.priority());
        if found_priority > req.priority() {
            return None;
        }
    }

    Some(found_id)
}

/// Find or create a presenter by tagged credit string.
///
/// Creates entities as needed. Existing presenter ranks are upgraded when the
/// `Kind:` prefix specifies a higher rank (lower priority number); they are
/// never downgraded, and bare-name (no `Kind:`) calls never change rank.
///
/// Does not handle UUID strings — callers should resolve UUIDs before calling
/// (see `lookup_or_create` in the `lookup` module).
pub fn find_or_create_tagged_presenter(
    schedule: &mut crate::schedule::Schedule,
    tagged: &str,
) -> Result<PresenterId, ConversionError> {
    let tagged = tagged.trim();
    if tagged.is_empty() {
        return Err(ConversionError::ParseError {
            message: "empty presenter string".to_string(),
        });
    }

    let parsed = parse_tag(tagged);

    if parsed.is_group_only() {
        let group_name = parsed
            .group_name
            .or((!parsed.name.is_empty()).then_some(parsed.name))
            .ok_or_else(|| ConversionError::ParseError {
                message: "empty group name".to_string(),
            })?;
        let gid =
            find_or_create_presenter_by_name(schedule, group_name, parsed.required_rank.as_ref());
        if let Some(d) = schedule.get_internal_mut::<PresenterEntityType>(gid) {
            d.data.is_explicit_group = true;
            if parsed.always_shown {
                d.data.always_shown_in_group = true;
            }
        }
        return Ok(gid);
    }

    let pres_id =
        find_or_create_presenter_by_name(schedule, parsed.name, parsed.required_rank.as_ref());
    if parsed.always_grouped {
        if let Some(d) = schedule.get_internal_mut::<PresenterEntityType>(pres_id) {
            d.data.always_grouped = true;
        }
    }

    if let Some(group_name) = parsed.group_name {
        let gid =
            find_or_create_presenter_by_name(schedule, group_name, parsed.required_rank.as_ref());
        if let Some(gd) = schedule.get_internal_mut::<PresenterEntityType>(gid) {
            gd.data.is_explicit_group = true;
            if parsed.always_shown {
                gd.data.always_shown_in_group = true;
            }
        }
        let already_in_group = {
            schedule
                .connected_field_nodes(pres_id, EDGE_GROUPS)
                .into_iter()
                .map(|e| unsafe { PresenterId::new_unchecked(e.entity_uuid()) })
                .collect::<Vec<PresenterId>>()
                .contains(&gid)
        };
        if !already_in_group {
            schedule
                .edge_add(pres_id, EDGE_GROUPS, std::iter::once(gid))
                .expect("edge type validation failed");
        }
    }

    Ok(pres_id)
}

/// Case-insensitive exact name lookup; creates with `effective_rank` if not found.
/// Upgrades rank only when `rank` is `Some` and is higher (lower priority number).
fn find_or_create_presenter_by_name(
    schedule: &mut crate::schedule::Schedule,
    name: &str,
    rank: Option<&PresenterRank>,
) -> PresenterId {
    let existing = schedule
        .iter_entities::<PresenterEntityType>()
        .find_map(|(id, d)| d.data.name.eq_ignore_ascii_case(name).then_some(id));

    if let Some(id) = existing {
        if let Some(new_rank) = rank {
            if let Some(d) = schedule.get_internal_mut::<PresenterEntityType>(id) {
                if new_rank.priority() < d.data.rank.priority() {
                    d.data.rank = new_rank.clone();
                }
            }
        }
        return id;
    }

    let effective_rank = rank.cloned().unwrap_or(PresenterRank::Panelist);
    let id = EntityId::generate();
    schedule.insert(
        id,
        PresenterInternalData {
            id,
            data: PresenterCommonData {
                name: name.to_string(),
                rank: effective_rank,
                ..Default::default()
            },
        },
    );
    id
}

// ── EntityStringResolver implementation ──────────────────────────────────────

impl EntityStringResolver for PresenterEntityType {
    fn entity_to_string(schedule: &crate::schedule::Schedule, id: EntityId<Self>) -> String {
        schedule
            .get_internal(id)
            .map(|data| data.data.name.clone())
            .unwrap_or_else(|| id.to_string())
    }
}

// ── Stored field descriptors ──────────────────────────────────────────────────

pub static FIELD_NAME: FieldDescriptor<PresenterEntityType> = {
    let (data, crdt_type, cb) = accessor_field_properties! {
        PresenterEntityType,
        name,
        name: "name",
        display: "Name",
        description: "Presenter or group display name.",
        aliases: &["presenter_name", "display_name"],
        cardinality: Single,
        item: String,
        example: "Alice Example",
        order: 0,
    };
    FieldDescriptor {
        data,
        crdt_type,
        required: true,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_NAME) }

/// Presenter rank — stored as `PresenterRank`, exposed as `FieldValue::String`
/// using the canonical tag (`guest`, `judge`, `staff`, `invited_panelist`,
/// `fan_panelist`, or a custom invited-guest label).
pub static FIELD_RANK: FieldDescriptor<PresenterEntityType> = {
    let (data, crdt_type, cb) = callback_field_properties! {
        PresenterEntityType,
        name: "rank",
        display: "Rank",
        description: "Presenter classification tier.",
        aliases: &["classification"],
        cardinality: Optional,
        item: String,
        example: "guest",
        order: 100,
        read: |d: &PresenterInternalData| {
            Some(field_value!(d.data.rank.as_str()))
        },
        write: |d: &mut PresenterInternalData, v: FieldValue| {
            d.data.rank = PresenterRank::parse(&v.into_string()?);
            Ok(())
        }
    };
    FieldDescriptor {
        data,
        crdt_type,
        required: false,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_RANK) }

pub static FIELD_BIO: FieldDescriptor<PresenterEntityType> = {
    let (data, crdt_type, cb) = accessor_field_properties! {
        PresenterEntityType,
        bio,
        name: "bio",
        display: "Bio",
        description: "Biography or description.",
        aliases: &["biography", "description"],
        cardinality: Optional,
        item: Text,
        example: "Long-time guest.",
        order: 200,
    };
    FieldDescriptor {
        data,
        crdt_type,
        required: false,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_BIO) }

pub static FIELD_IS_EXPLICIT_GROUP: FieldDescriptor<PresenterEntityType> = {
    let (data, crdt_type, cb) = accessor_field_properties! {
        PresenterEntityType,
        is_explicit_group,
        name: "is_explicit_group",
        display: "Is Explicit Group",
        description: "Marks this presenter entity as an explicit group.",
        aliases: &["explicit_group"],
        cardinality: Single,
        item: Boolean,
        example: "false",
        order: 300,
        required: false,
    };
    FieldDescriptor {
        data,
        crdt_type,
        required: false,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_IS_EXPLICIT_GROUP) }

pub static FIELD_ALWAYS_GROUPED: FieldDescriptor<PresenterEntityType> = {
    let (data, crdt_type, cb) = accessor_field_properties! {
        PresenterEntityType,
        always_grouped,
        name: "always_grouped",
        display: "Always Grouped",
        description: "Always display this member under its group name.",
        aliases: &[],
        cardinality: Single,
        item: Boolean,
        example: "false",
        order: 400,
        required: false,
    };
    FieldDescriptor {
        data,
        crdt_type,
        required: false,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_ALWAYS_GROUPED) }

pub static FIELD_ALWAYS_SHOWN_IN_GROUP: FieldDescriptor<PresenterEntityType> = {
    let (data, crdt_type, cb) = accessor_field_properties! {
        PresenterEntityType,
        always_shown_in_group,
        name: "always_shown_in_group",
        display: "Always Shown In Group",
        description: "Always show group name even with partial member attendance.",
        aliases: &["always_shown"],
        cardinality: Single,
        item: Boolean,
        example: "false",
        order: 500,
        required: false,
    };
    FieldDescriptor {
        data,
        crdt_type,
        required: false,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_ALWAYS_SHOWN_IN_GROUP) }

// ── Computed / edge-backed fields ─────────────────────────────────────────────

/// `is_group` — `true` if `is_explicit_group` is set OR this presenter has
/// any members (edge-based membership).
pub static FIELD_IS_GROUP: FieldDescriptor<PresenterEntityType> = {
    let (data, _, cb) = callback_field_properties! {
        PresenterEntityType,
        name: "is_group",
        display: "Is Group",
        description: "Whether this entity represents a group (explicit flag or has members).",
        aliases: &["group"],
        cardinality: Single,
        item: Boolean,
        example: "false",
        order: 600,
        read: |sched: &Schedule, id: EntityId<PresenterEntityType>| {
            let explicit = sched
                .get_internal::<PresenterEntityType>(id)
                .is_some_and(|d| d.data.is_explicit_group);
            // Edge convention: a field name points at the far side of the edge.
            // `FIELD_MEMBERS` on `id` therefore points at id's members
            // (id playing the group role); querying it toward far-side
            // `FIELD_GROUPS` returns those member entities.
            let has_members = !sched
                .connected_field_nodes(id, EDGE_MEMBERS)
                .is_empty();
            Some(field_value!(explicit || has_members))
        }
    };
    FieldDescriptor {
        data,
        crdt_type: CrdtFieldType::Derived,
        required: false,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_IS_GROUP) }

pub static HALF_EDGE_MEMBERS: crate::edge::HalfEdgeDescriptor = {
    crate::edge::HalfEdgeDescriptor {
        data: crate::field::CommonFieldData {
            name: "members",
            display: "Members",
            description: "Members of this group (empty for individuals).",
            aliases: &["group_members"],
            field_type: crate::value::FieldType(
                crate::value::FieldCardinality::List,
                crate::value::FieldTypeItem::EntityIdentifier(PresenterEntityType::TYPE_NAME),
            ),
            example: "[]",
            order: 800,
        },
        edge_kind: crate::edge::EdgeKind::Owner {
            target_field: &HALF_EDGE_GROUPS,
            exclusive_with: None,
        },
        entity_name: PresenterEntityType::TYPE_NAME,
    }
};
inventory::submit! { CollectedHalfEdge(&HALF_EDGE_MEMBERS) }

pub static HALF_EDGE_GROUPS: crate::edge::HalfEdgeDescriptor = {
    crate::edge::HalfEdgeDescriptor {
        data: crate::field::CommonFieldData {
            name: "groups",
            display: "Groups",
            description: "Groups this presenter belongs to.",
            aliases: &["group_memberships"],
            field_type: crate::value::FieldType(
                crate::value::FieldCardinality::List,
                crate::value::FieldTypeItem::EntityIdentifier(PresenterEntityType::TYPE_NAME),
            ),
            example: "[]",
            order: 700,
        },
        edge_kind: crate::edge::EdgeKind::Target {
            source_fields: &[&HALF_EDGE_MEMBERS],
        },
        entity_name: PresenterEntityType::TYPE_NAME,
    }
};
inventory::submit! { CollectedHalfEdge(&HALF_EDGE_GROUPS) }

/// Static edge from groups field to members field (for querying a presenter's groups)
pub const EDGE_GROUPS: crate::edge::FullEdge = crate::edge::FullEdge {
    near: &HALF_EDGE_GROUPS,
    far: &HALF_EDGE_MEMBERS,
};

/// Static edge from members field to groups field (for querying a group's members)
pub const EDGE_MEMBERS: crate::edge::FullEdge = crate::edge::FullEdge {
    near: &HALF_EDGE_MEMBERS,
    far: &HALF_EDGE_GROUPS,
};

/// Inclusive groups — all groups this presenter belongs to, transitively.
///
/// Follows forward homogeneous edges upward: `presenter → group → parent_group → …`.
/// Does not include the presenter itself.
pub static FIELD_INCLUSIVE_GROUPS: FieldDescriptor<PresenterEntityType> = {
    let (data, _, cb) = callback_field_properties! {
        PresenterEntityType,
        name: "inclusive_groups",
        display: "Inclusive Groups",
        description: "Transitive closure of groups this presenter appears in.",
        aliases: &[],
        cardinality: List,
        item: EntityIdentifier,
        item_entity: PresenterEntityType,
        example: "[]",
        order: 900,
        read: |sched: &Schedule, id: EntityId<PresenterEntityType>| {
            let ids = sched.inclusive_edges::<PresenterEntityType, PresenterEntityType>(id, EDGE_GROUPS);
            Some(crate::schedule::entity_ids_to_field_value(ids))
        }
    };
    FieldDescriptor {
        data,
        crdt_type: CrdtFieldType::Derived,
        required: false,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_INCLUSIVE_GROUPS) }

/// Inclusive members — all members of this group, transitively.
///
/// Follows reverse homogeneous edges downward: `group ← member ← sub_member ← …`.
/// Does not include the group itself.
pub static FIELD_INCLUSIVE_MEMBERS: FieldDescriptor<PresenterEntityType> = {
    let (data, _, cb) = callback_field_properties! {
        PresenterEntityType,
        name: "inclusive_members",
        display: "Inclusive Members",
        description: "Transitive closure of members for this group.",
        aliases: &[],
        cardinality: List,
        item: EntityIdentifier,
        item_entity: PresenterEntityType,
        example: "[]",
        order: 1000,
        read: |sched: &Schedule, id: EntityId<PresenterEntityType>| {
            let ids = sched.inclusive_edges::<PresenterEntityType, PresenterEntityType>(id, EDGE_MEMBERS);
            Some(crate::schedule::entity_ids_to_field_value(ids))
        }
    };
    FieldDescriptor {
        data,
        crdt_type: CrdtFieldType::Derived,
        required: false,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_INCLUSIVE_MEMBERS) }

/// All panels this presenter is scheduled on (credited and uncredited).
///
/// Target edge that combines credited and uncredited panel edges.
pub static HALF_EDGE_PANELS: crate::edge::HalfEdgeDescriptor = {
    crate::edge::HalfEdgeDescriptor {
        data: crate::field::CommonFieldData {
            name: "panels",
            display: "Panels",
            description: "Panels this presenter is scheduled on (credited and uncredited).",
            aliases: &["panel"],
            field_type: crate::value::FieldType(
                crate::value::FieldCardinality::List,
                crate::value::FieldTypeItem::EntityIdentifier(PanelEntityType::TYPE_NAME),
            ),
            example: "[]",
            order: 1100,
        },
        edge_kind: crate::edge::EdgeKind::Target {
            source_fields: &[
                &panel::HALF_EDGE_CREDITED_PRESENTERS,
                &panel::HALF_EDGE_UNCREDITED_PRESENTERS,
            ],
        },
        entity_name: PresenterEntityType::TYPE_NAME,
    }
};
inventory::submit! { CollectedHalfEdge(&HALF_EDGE_PANELS) }

/// Full edge from panel credited presenters to presenter panels
pub const EDGE_CREDITED_PANELS: crate::edge::FullEdge = crate::edge::FullEdge {
    near: &HALF_EDGE_PANELS,
    far: &panel::HALF_EDGE_CREDITED_PRESENTERS,
};

/// Full edge from panel uncredited presenters to presenter panels
pub const EDGE_UNCREDITED_PANELS: crate::edge::FullEdge = crate::edge::FullEdge {
    near: &HALF_EDGE_PANELS,
    far: &panel::HALF_EDGE_UNCREDITED_PRESENTERS,
};

/// Credited panels for this presenter.
///
/// Read/write field for panels where this presenter is credited.
pub static FIELD_CREDITED_PANELS: FieldDescriptor<PresenterEntityType> = {
    let (data, _, cb) = callback_field_properties! {
        PresenterEntityType,
        name: "credited_panels",
        display: "Credited Panels",
        description: "Panels where this presenter is credited.",
        aliases: &["credited_panel"],
        cardinality: List,
        item: EntityIdentifier,
        item_entity: PanelEntityType,
        example: "[panel_id]",
        order: 1200,
        read: |sched: &Schedule, id: PresenterId| {
            let ids: Vec<PanelId> = sched
                .connected_field_nodes(id, EDGE_CREDITED_PANELS)
                .into_iter()
                .map(|e| unsafe { PanelId::new_unchecked(e.entity_uuid()) })
                .collect();
            Some(crate::schedule::entity_ids_to_field_value(ids))
        },
        write: |sched: &mut Schedule, presenter_id: PresenterId, val: FieldValue| {
            let ids = crate::schedule::field_value_to_entity_ids::<PanelEntityType>(val)?;
            let far_type = EDGE_UNCREDITED_PANELS.far.entity_type_name();
            let (added, _ ) = sched.edge_set(presenter_id, EDGE_CREDITED_PANELS, ids)?;
            // SAFETY: The added UUIDs are already validated to be panel::PanelEntityType::TYPE_NAME().
            let added_runtime: Vec<crate::entity::RuntimeEntityId> = added
                .into_iter()
                .map(|uuid| unsafe {
                    crate::entity::RuntimeEntityId::new_unchecked(uuid, far_type)
                })
                .collect();
            sched.edge_remove(presenter_id, EDGE_UNCREDITED_PANELS, added_runtime);
            Ok(())
        },
        add: |sched: &mut Schedule, presenter_id: PresenterId, val: FieldValue| {
            let ids = crate::schedule::field_value_to_entity_ids::<PanelEntityType>(val)?;
            let far_type = EDGE_UNCREDITED_PANELS.far.entity_type_name();
            let added = sched.edge_add(presenter_id, EDGE_CREDITED_PANELS, ids)?;
            // SAFETY: The added UUIDs are already validated to be panel::PanelEntityType::TYPE_NAME().
            let added_runtime: Vec<crate::entity::RuntimeEntityId> = added
                .into_iter()
                .map(|uuid| unsafe {
                    crate::entity::RuntimeEntityId::new_unchecked(uuid, far_type)
                })
                .collect();
            sched.edge_remove(presenter_id, EDGE_UNCREDITED_PANELS, added_runtime);
            Ok(())
        }
    };
    FieldDescriptor {
        data,
        crdt_type: crate::crdt::CrdtFieldType::Derived,
        required: false,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_CREDITED_PANELS) }

/// Uncredited panels for this presenter.
///
/// Read/write field for panels where this presenter is uncredited.
pub static FIELD_UNCREDITED_PANELS: FieldDescriptor<PresenterEntityType> = {
    let (data, _, cb) = callback_field_properties! {
        PresenterEntityType,
        name: "uncredited_panels",
        display: "Uncredited Panels",
        description: "Panels where this presenter is uncredited.",
        aliases: &["uncredited_panel"],
        cardinality: List,
        item: EntityIdentifier,
        item_entity: PanelEntityType,
        example: "[panel_id]",
        order: 1210,
        read: |sched: &Schedule, id: PresenterId| {
            let edge = EDGE_UNCREDITED_PANELS;
            let ids: Vec<PanelId> = sched
                .connected_field_nodes(id, edge)
                .into_iter()
                .map(|e| unsafe { PanelId::new_unchecked(e.entity_uuid()) })
                .collect();
            Some(crate::schedule::entity_ids_to_field_value(ids))
        },
        write: |sched: &mut Schedule, presenter_id: PresenterId, val: FieldValue| {
            let ids = crate::schedule::field_value_to_entity_ids::<PanelEntityType>(val)?;
            let far_type = EDGE_CREDITED_PANELS.far.entity_type_name();
            let (added, _) = sched.edge_set(presenter_id, EDGE_UNCREDITED_PANELS, ids)?;
            // SAFETY: The added UUIDs are already validated to be panel::PanelEntityType::TYPE_NAME().
            let added_runtime: Vec<crate::entity::RuntimeEntityId> = added
                .into_iter()
                .map(|uuid| unsafe {
                    crate::entity::RuntimeEntityId::new_unchecked(uuid, far_type)
                })
                .collect();
            sched.edge_remove(presenter_id, EDGE_CREDITED_PANELS, added_runtime);
            Ok(())
        },
        add: |sched: &mut Schedule, presenter_id: PresenterId, val: FieldValue| {
            let ids = crate::schedule::field_value_to_entity_ids::<PanelEntityType>(val)?;
            let far_type = EDGE_CREDITED_PANELS.far.entity_type_name();
            let added = sched.edge_add(presenter_id, EDGE_UNCREDITED_PANELS, ids)?;
            // SAFETY: The added UUIDs are already validated to be panel::PanelEntityType::TYPE_NAME().
            let added_runtime: Vec<crate::entity::RuntimeEntityId> = added
                .into_iter()
                .map(|uuid| unsafe {
                    crate::entity::RuntimeEntityId::new_unchecked(uuid, far_type)
                })
                .collect();
            sched.edge_remove(presenter_id, EDGE_CREDITED_PANELS, added_runtime);
            Ok(())
        }
    };
    FieldDescriptor {
        data,
        crdt_type: CrdtFieldType::Derived,
        required: false,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_UNCREDITED_PANELS) }

/// Inclusive panels for a presenter.
///
/// Union of:
/// - Direct panels of this presenter.
/// - Panels of all transitive groups (following forward homogeneous edges upward).
/// - Panels of all transitive members (following reverse homogeneous edges downward).
///
/// This is symmetric with `FIELD_INCLUSIVE_PRESENTERS` on panels: if a panel
/// lists Team A, then all of Team A's inclusive presenters see that panel in
/// their inclusive panels.
pub static FIELD_INCLUSIVE_PANELS: FieldDescriptor<PresenterEntityType> = {
    let (data, _, cb) = callback_field_properties! {
        PresenterEntityType,
        name: "inclusive_panels",
        display: "Inclusive Panels",
        description: "Panels of this presenter plus panels of its transitive groups and members.",
        aliases: &[],
        cardinality: List,
        item: EntityIdentifier,
        item_entity: PanelEntityType,
        example: "[]",
        order: 1400,
        read: |sched: &Schedule, id: EntityId<PresenterEntityType>| {
            use std::collections::HashSet;
            let mut panel_set: HashSet<PanelId> = HashSet::new();
            // Direct panels of this presenter
            let credited_ids: Vec<PanelId> = sched
                .connected_field_nodes(id, EDGE_CREDITED_PANELS)
                .into_iter()
                .map(|e| unsafe { PanelId::new_unchecked(e.entity_uuid()) })
                .collect();
            let uncredited_ids: Vec<PanelId> = sched
                .connected_field_nodes(id, EDGE_UNCREDITED_PANELS)
                .into_iter()
                .map(|e| unsafe { PanelId::new_unchecked(e.entity_uuid()) })
                .collect();
            for p in credited_ids.into_iter().chain(uncredited_ids) {
                panel_set.insert(p);
            }
            // Panels of all transitive groups (upward)
            for g in sched.inclusive_edges::<PresenterEntityType, PresenterEntityType>(id, EDGE_GROUPS) {
                let credited_ids: Vec<PanelId> = sched
                    .connected_field_nodes(g, EDGE_CREDITED_PANELS)
                    .into_iter()
                    .map(|e| unsafe { PanelId::new_unchecked(e.entity_uuid()) })
                    .collect();
                let uncredited_ids: Vec<PanelId> = sched
                    .connected_field_nodes(g, EDGE_UNCREDITED_PANELS)
                    .into_iter()
                    .map(|e| unsafe { PanelId::new_unchecked(e.entity_uuid()) })
                    .collect();
                for p in credited_ids.into_iter().chain(uncredited_ids) {
                    panel_set.insert(p);
                }
            }
            // Panels of all transitive members (downward)
            for m in sched.inclusive_edges::<PresenterEntityType, PresenterEntityType>(id, EDGE_MEMBERS) {
                let credited_ids: Vec<PanelId> = sched
                    .connected_field_nodes(m, EDGE_CREDITED_PANELS)
                    .into_iter()
                    .map(|e| unsafe { PanelId::new_unchecked(e.entity_uuid()) })
                    .collect();
                let uncredited_ids: Vec<PanelId> = sched
                    .connected_field_nodes(m, EDGE_UNCREDITED_PANELS)
                    .into_iter()
                    .map(|e| unsafe { PanelId::new_unchecked(e.entity_uuid()) })
                    .collect();
                for p in credited_ids.into_iter().chain(uncredited_ids) {
                    panel_set.insert(p);
                }
            }
            let ids: Vec<PanelId> = panel_set.into_iter().collect();
            Some(crate::schedule::entity_ids_to_field_value(ids))
        }
    };
    FieldDescriptor {
        data,
        crdt_type: CrdtFieldType::Derived,
        required: false,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_INCLUSIVE_PANELS) }

// ── FieldSet ──────────────────────────────────────────────────────────────────

static PRESENTER_FIELD_SET: LazyLock<FieldSet<PresenterEntityType>> =
    LazyLock::new(FieldSet::from_inventory);

// ── Builder ───────────────────────────────────────────────────────────────────

crate::field::macros::define_entity_builder! {
    /// Typed builder for [`PresenterEntityType`] entities.
    PresenterBuilder for PresenterEntityType {
        /// Set the presenter or group display name.  Required.
        with_name                  => FIELD_NAME,
        /// Set the presenter rank (canonical tag: `guest`, `judge`, `staff`,
        /// `invited_panelist`, `fan_panelist`, or a custom invited-guest label).
        with_rank                  => FIELD_RANK,
        /// Set the biography or description.
        with_bio                   => FIELD_BIO,
        /// Mark this entity as an explicit group (vs. an individual).
        with_is_explicit_group     => FIELD_IS_EXPLICIT_GROUP,
        /// Always display this member under its group name.
        with_always_grouped        => FIELD_ALWAYS_GROUPED,
        /// Always show the group name, even with partial member attendance.
        with_always_shown_in_group => FIELD_ALWAYS_SHOWN_IN_GROUP,
        /// Set the groups this presenter belongs to.
        with_groups                => HALF_EDGE_GROUPS,
        /// Set the members of this group (empty for individuals).
        with_members               => HALF_EDGE_MEMBERS,
        /// Replace the set of panels this presenter is scheduled on.
        with_panels                => HALF_EDGE_PANELS,
    }
}

// ── EntityMatcher ─────────────────────────────────────────────────────────────

/// Extract the bare presenter name from a potentially tagged credit string.
///
/// Strips the optional rank prefix (`"G:"`, `"P:"`, `"GOH:"`, etc.), the
/// always-grouped marker (`"<"`), and the group suffix (`"=Group"`).
/// For the group-only form (`"=TeamA"` / `"==TeamA"`), returns the group name.
fn extract_presenter_match_name(query: &str) -> &str {
    let s = query.trim();
    // Strip optional rank prefix: alphabetic chars before ':'
    let s = if let Some(colon) = s.find(':') {
        let prefix = &s[..colon];
        if !prefix.is_empty() && prefix.chars().all(|c| c.is_alphabetic()) {
            s[colon + 1..].trim()
        } else {
            s
        }
    } else {
        s
    };
    // Strip always-grouped marker '<'
    let s = s.strip_prefix('<').map(str::trim).unwrap_or(s);
    // Handle group suffix "=…": "Name=Group" → "Name"; "=Group" → "Group"
    match s.find('=') {
        Some(eq) => {
            let before = s[..eq].trim();
            if before.is_empty() {
                // Group-only form: "=TeamA" or "==TeamA"
                s[eq + 1..]
                    .trim()
                    .strip_prefix('=')
                    .map(str::trim)
                    .unwrap_or(s[eq + 1..].trim())
            } else {
                before
            }
        }
        None => s,
    }
}

impl crate::query::lookup::EntityScannable for PresenterEntityType {
    /// Tagged-presenter-aware scan.
    ///
    /// The tagged credit syntax (`"Kind:Name=Group"`) is always per-token —
    /// it never spans a `,` / `;` separator — so we only consult the
    /// `partial` slice.  `find_tagged_presenter` handles prefix rank gates,
    /// group-only forms (`"=Band"` / `"==Band"`), and `=Group` membership
    /// verification, so we use it directly instead of the default
    /// linear-scan + `extract_presenter_match_name` path.
    ///
    /// On miss we defer to [`PresenterEntityType::can_create`]; its
    /// `CanCreate::Yes` hint drives whether the loop queues the whole
    /// remaining query or just the current token.  Actual creation runs
    /// through [`crate::query::lookup::EntityCreatable::create_from_string`], which in turn calls
    /// [`find_or_create_tagged_presenter`] — so group membership and rank
    /// promotion are honoured on the create path too.
    fn scan_entity(
        full: &str,
        partial: &str,
        schedule: &crate::schedule::Schedule,
    ) -> Result<crate::query::lookup::ScanResult<Self>, crate::query::lookup::LookupError> {
        use crate::query::lookup::{
            CanCreate, EntityMatcher, LookupError, MatchConsumed, ScanFound, ScanResult,
        };

        let consumed = if full == partial {
            MatchConsumed::Full
        } else {
            MatchConsumed::Partial
        };

        if let Some(id) = find_tagged_presenter(schedule, partial) {
            return Ok(ScanResult(consumed, ScanFound::Entity(id)));
        }

        match Self::can_create(full, partial) {
            CanCreate::No => Err(LookupError::NotFound {
                query: full.to_string(),
            }),
            CanCreate::Yes(c) => Ok(ScanResult(c, ScanFound::CanCreate)),
        }
    }
}

impl crate::query::lookup::EntityMatcher for PresenterEntityType {
    fn can_create(full: &str, partial: &str) -> crate::query::lookup::CanCreate {
        if partial.is_empty() {
            crate::query::lookup::CanCreate::No
        } else if full == partial {
            crate::query::lookup::CanCreate::Yes(crate::query::lookup::MatchConsumed::Full)
        } else {
            crate::query::lookup::CanCreate::Yes(crate::query::lookup::MatchConsumed::Partial)
        }
    }

    fn match_entity(
        query: &str,
        data: &PresenterInternalData,
    ) -> Option<crate::query::lookup::MatchPriority> {
        let name = extract_presenter_match_name(query);
        crate::query::lookup::string_match_priority(name, &data.data.name)
    }
}

// ── EntityCreatable ───────────────────────────────────────────────────────────

impl crate::query::lookup::EntityCreatable for PresenterEntityType {
    fn create_from_string(
        schedule: &mut crate::schedule::Schedule,
        s: &str,
    ) -> Result<EntityId<Self>, crate::query::lookup::LookupError> {
        find_or_create_tagged_presenter(schedule, s).map_err(|e| {
            crate::query::lookup::LookupError::CreateFailed {
                message: e.to_string(),
            }
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::lookup::{match_priority, EntityMatcher};
    use crate::schedule::Schedule;
    use crate::value::{FieldError, FieldValueItem};
    use uuid::Uuid;

    fn make_id() -> PresenterId {
        let uuid = Uuid::new_v4();
        let non_nil_uuid = unsafe { uuid::NonNilUuid::new_unchecked(uuid) };
        unsafe { PresenterId::new_unchecked(non_nil_uuid) }
    }

    fn make_internal() -> PresenterInternalData {
        PresenterInternalData {
            data: PresenterCommonData {
                name: "Alice Example".into(),
                rank: PresenterRank::Guest,
                bio: Some("Long-time guest.".into()),
                is_explicit_group: false,
                always_grouped: false,
                always_shown_in_group: false,
                sort_rank: Some(PresenterSortRank {
                    column_index: 0,
                    row_index: 3,
                    member_index: 0,
                }),
            },
            id: make_id(),
        }
    }

    fn schedule_with(id: PresenterId, data: PresenterInternalData) -> Schedule {
        let mut sched = Schedule::default();
        sched.insert(id, data);
        sched
    }

    #[test]
    fn test_rank_serde_roundtrip() {
        let values = [
            PresenterRank::Guest,
            PresenterRank::Judge,
            PresenterRank::Staff,
            PresenterRank::InvitedGuest(None),
            PresenterRank::InvitedGuest(Some("Sponsor".into())),
            PresenterRank::Panelist,
            PresenterRank::FanPanelist,
        ];
        for rank in &values {
            let json = serde_json::to_string(rank).unwrap();
            let back: PresenterRank = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, rank);
        }
    }

    #[test]
    fn test_rank_priority_ordering() {
        assert!(PresenterRank::Guest.priority() < PresenterRank::FanPanelist.priority());
        assert!(PresenterRank::Judge.priority() < PresenterRank::Staff.priority());
        assert!(PresenterRank::Panelist.priority() < PresenterRank::FanPanelist.priority());
    }

    #[test]
    fn test_rank_prefix_chars_distinguish_panelist_and_fan_panelist() {
        assert_eq!(PresenterRank::Panelist.prefix_char(), 'P');
        assert_eq!(PresenterRank::FanPanelist.prefix_char(), 'F');
        assert_eq!(
            PresenterRank::from_prefix_char('P'),
            Some(PresenterRank::Panelist)
        );
        assert_eq!(
            PresenterRank::from_prefix_char('f'),
            Some(PresenterRank::FanPanelist)
        );
    }

    #[test]
    fn test_rank_default_is_panelist() {
        assert_eq!(PresenterRank::default(), PresenterRank::Panelist);
    }

    #[test]
    fn test_field_set_count_and_required() {
        let fs = PresenterEntityType::field_set();
        assert_eq!(fs.fields().count(), 12);
        assert_eq!(fs.half_edges().count(), 3);
        let required: Vec<_> = fs.required_fields().map(|d| d.name()).collect();
        assert_eq!(required, vec!["name"]);
    }

    #[test]
    fn test_field_set_aliases() {
        let fs = PresenterEntityType::field_set();
        assert!(fs.get_by_name("classification").is_some()); // rank alias
        assert!(fs.get_by_name("biography").is_some()); // bio alias
        assert!(fs.get_by_name("always_shown").is_some()); // always_shown_in_group alias
    }

    #[test]
    fn test_read_name_and_rank() {
        let id = make_id();
        let sched = schedule_with(id, make_internal());
        let fs = PresenterEntityType::field_set();
        assert_eq!(
            fs.read_field_value("name", id, &sched).unwrap(),
            Some(field_value!("Alice Example"))
        );
        assert_eq!(
            fs.read_field_value("rank", id, &sched).unwrap(),
            Some(field_value!("guest"))
        );
    }

    #[test]
    fn test_write_rank_custom_invited() {
        let id = make_id();
        let mut sched = schedule_with(id, make_internal());
        let fs = PresenterEntityType::field_set();
        fs.write_field_value("rank", id, &mut sched, field_value!("Sponsor"))
            .unwrap();
        let value = fs.read_field_value("rank", id, &sched).unwrap();
        assert_eq!(value, Some(field_value!("Sponsor")));
    }

    #[test]
    fn test_is_group_mirrors_explicit_flag() {
        let id = make_id();
        let mut internal = make_internal();
        internal.data.is_explicit_group = true;
        let sched = schedule_with(id, internal);
        let fs = PresenterEntityType::field_set();
        assert_eq!(
            fs.read_field_value("is_group", id, &sched).unwrap(),
            Some(field_value!(true))
        );
    }

    #[test]
    fn test_is_group_false_without_explicit_or_members() {
        // Plain individual presenter with the explicit flag clear and no
        // edges should report `is_group: false`.
        let id = make_id();
        let sched = schedule_with(id, make_internal());
        let fs = PresenterEntityType::field_set();
        assert_eq!(
            fs.read_field_value("is_group", id, &sched).unwrap(),
            Some(field_value!(false))
        );
    }

    #[test]
    fn test_is_group_implicit_via_member_edge() {
        // `is_group` should become true when a presenter has a member, even
        // if `is_explicit_group` is false. Edge convention: each field name
        // points at the far side of the edge, so the group-side half-edge
        // sits on `FIELD_MEMBERS` (it points at members) and the member-side
        // sits on `FIELD_GROUPS` (it points at groups).
        let group_id = make_id();
        let member_id = make_id();
        let mut sched = Schedule::default();
        sched.insert(group_id, {
            let mut d = make_internal();
            d.id = group_id;
            d.data.name = "MyBand".into();
            d
        });
        sched.insert(member_id, {
            let mut d = make_internal();
            d.id = member_id;
            d.data.name = "Alice".into();
            d
        });

        // Add the group edge: member's GROUPS pointer → group
        sched
            .edge_add(member_id, EDGE_GROUPS, std::iter::once(group_id))
            .expect("edge type validation failed");

        let fs = PresenterEntityType::field_set();
        // The group reports is_group: true via the member edge.
        assert_eq!(
            fs.read_field_value("is_group", group_id, &sched).unwrap(),
            Some(field_value!(true))
        );
        // The member is not itself a group.
        assert_eq!(
            fs.read_field_value("is_group", member_id, &sched).unwrap(),
            Some(field_value!(false))
        );
    }

    #[test]
    fn test_is_group_is_read_only() {
        let id = make_id();
        let mut sched = schedule_with(id, make_internal());
        let fs = PresenterEntityType::field_set();
        let result = fs.write_field_value("is_group", id, &mut sched, field_value!(true));
        assert!(matches!(result, Err(FieldError::ReadOnly { .. })));
    }

    #[test]
    fn test_edge_fields_empty_without_edges() {
        let id = make_id();
        let sched = schedule_with(id, make_internal());
        let fs = PresenterEntityType::field_set();
        for name in [
            "groups",
            "members",
            "inclusive_groups",
            "inclusive_members",
            "panels",
            "inclusive_panels",
        ] {
            assert_eq!(
                fs.read_field_value(name, id, &sched).unwrap(),
                Some(field_value!(empty_list)),
                "field {name} should be empty without edges"
            );
        }
    }

    #[test]
    fn test_match_name_starts_with() {
        let data = make_internal();
        let priority = PresenterEntityType::match_entity("alice", &data);
        assert_eq!(priority, Some(match_priority::STRONG_MATCH));
    }

    #[test]
    fn test_common_data_serde_roundtrip() {
        let original = PresenterCommonData {
            name: "Group One".into(),
            rank: PresenterRank::InvitedGuest(Some("105th".into())),
            bio: None,
            is_explicit_group: true,
            always_grouped: false,
            always_shown_in_group: true,
            sort_rank: Some(PresenterSortRank {
                column_index: 5,
                row_index: 0,
                member_index: 0,
            }),
        };
        let json = serde_json::to_string(&original).unwrap();
        let back: PresenterCommonData = serde_json::from_str(&json).unwrap();
        assert_eq!(original, back);
    }

    #[test]
    fn test_validate_missing_name() {
        let data = PresenterCommonData::default();
        let errors = data.validate();
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], ValidationError::Required { field } if field == "name"));
    }

    #[test]
    fn test_sort_rank_member_index_omitted_when_zero() {
        let sr = PresenterSortRank {
            column_index: 1,
            row_index: 2,
            member_index: 0,
        };
        let json = serde_json::to_string(&sr).unwrap();
        assert!(!json.contains("memberIndex"));

        let sr2 = PresenterSortRank {
            column_index: 1,
            row_index: 2,
            member_index: 3,
        };
        let json2 = serde_json::to_string(&sr2).unwrap();
        assert!(json2.contains("memberIndex"));
    }

    #[test]
    fn test_entity_to_string_returns_name() {
        use crate::query::converter::EntityStringResolver;
        let id = make_id();
        let sched = schedule_with(id, make_internal());
        let s = PresenterEntityType::entity_to_string(&sched, id);
        assert_eq!(s, "Alice Example");
    }

    #[test]
    fn test_entity_to_string_fallback_to_uuid() {
        use crate::query::converter::EntityStringResolver;
        let id = make_id();
        let sched = Schedule::default();
        let s = PresenterEntityType::entity_to_string(&sched, id);
        assert_eq!(s, id.to_string());
    }

    // ── Tagged presenter tests ─────────────────────────────────────────────────

    fn make_presenter(name: &str, rank: PresenterRank) -> PresenterInternalData {
        let id = make_id();
        PresenterInternalData {
            id,
            data: PresenterCommonData {
                name: name.to_string(),
                rank,
                ..Default::default()
            },
        }
    }

    #[test]
    fn test_find_tagged_bare_name() {
        let id = make_id();
        let sched = schedule_with(id, make_internal());
        assert_eq!(find_tagged_presenter(&sched, "Alice Example"), Some(id));
    }

    #[test]
    fn test_find_tagged_empty_returns_none() {
        let sched = Schedule::default();
        assert_eq!(find_tagged_presenter(&sched, ""), None);
        assert_eq!(find_tagged_presenter(&sched, "  "), None);
    }

    #[test]
    fn test_find_tagged_kind_prefix_match() {
        let id = make_id();
        let mut internal = make_internal();
        internal.data.rank = PresenterRank::Guest;
        let sched = schedule_with(id, internal);
        // G: = Guest rank required; Alice is Guest → match
        assert_eq!(find_tagged_presenter(&sched, "G:Alice Example"), Some(id));
    }

    #[test]
    fn test_find_tagged_rank_gate_rejects_lower_rank() {
        let id = make_id();
        // Alice is Panelist (priority 4); G: requires Guest (priority 0) → reject
        let sched = schedule_with(id, make_internal()); // rank = Guest
                                                        // make_internal has rank=Guest, so G: passes
        assert_eq!(find_tagged_presenter(&sched, "G:Alice Example"), Some(id));
        // F: requires FanPanelist (priority 5); Guest (0) < 5 → passes (Guest is higher rank)
        assert_eq!(find_tagged_presenter(&sched, "F:Alice Example"), Some(id));

        // Create a FanPanelist — requesting G: (priority 0) should fail
        let mut sched2 = Schedule::default();
        let id2 = make_id();
        let mut fan = make_presenter("Bob", PresenterRank::FanPanelist);
        fan.id = id2;
        sched2.insert(id2, fan);
        assert_eq!(find_tagged_presenter(&sched2, "G:Bob"), None);
    }

    #[test]
    fn test_find_tagged_group_only_form() {
        let mut sched = Schedule::default();
        let group_id = make_id();
        let mut group = make_presenter("MyBand", PresenterRank::Panelist);
        group.id = group_id;
        group.data.is_explicit_group = true;
        sched.insert(group_id, group);

        assert_eq!(find_tagged_presenter(&sched, "=MyBand"), Some(group_id));
        assert_eq!(find_tagged_presenter(&sched, "==MyBand"), Some(group_id));
    }

    #[test]
    fn test_find_tagged_group_only_rejects_non_group() {
        let id = make_id();
        let sched = schedule_with(id, make_internal()); // is_explicit_group = false, no members
        assert_eq!(find_tagged_presenter(&sched, "=Alice Example"), None);
    }

    #[test]
    fn test_find_tagged_group_suffix_verifies_membership() {
        let mut sched = Schedule::default();
        let alice_id = make_id();
        let group_id = make_id();
        let mut alice = make_presenter("Alice", PresenterRank::Panelist);
        alice.id = alice_id;
        let mut group = make_presenter("MyBand", PresenterRank::Panelist);
        group.id = group_id;
        group.data.is_explicit_group = true;
        sched.insert(alice_id, alice);
        sched.insert(group_id, group);
        sched
            .edge_add(alice_id, EDGE_GROUPS, std::iter::once(group_id))
            .expect("edge type validation failed");

        assert_eq!(
            find_tagged_presenter(&sched, "Alice=MyBand"),
            Some(alice_id)
        );
        assert_eq!(find_tagged_presenter(&sched, "Alice=OtherGroup"), None);
    }

    #[test]
    fn test_find_or_create_bare_name_creates_panelist() {
        let mut sched = Schedule::default();
        let id = find_or_create_tagged_presenter(&mut sched, "Jane Doe").unwrap();
        let d = sched.get_internal::<PresenterEntityType>(id).unwrap();
        assert_eq!(d.data.name, "Jane Doe");
        assert_eq!(d.data.rank, PresenterRank::Panelist);
        assert!(!d.data.is_explicit_group);
    }

    #[test]
    fn test_find_or_create_idempotent() {
        let mut sched = Schedule::default();
        let id1 = find_or_create_tagged_presenter(&mut sched, "Alice").unwrap();
        let id2 = find_or_create_tagged_presenter(&mut sched, "Alice").unwrap();
        assert_eq!(id1, id2);
        assert_eq!(sched.entity_count::<PresenterEntityType>(), 1);
    }

    #[test]
    fn test_find_or_create_rank_upgrade() {
        let mut sched = Schedule::default();
        let id = find_or_create_tagged_presenter(&mut sched, "P:Alice").unwrap();
        assert_eq!(
            sched
                .get_internal::<PresenterEntityType>(id)
                .unwrap()
                .data
                .rank,
            PresenterRank::Panelist
        );
        // G: = Guest (priority 0 < 4) → upgrade
        find_or_create_tagged_presenter(&mut sched, "G:Alice").unwrap();
        assert_eq!(
            sched
                .get_internal::<PresenterEntityType>(id)
                .unwrap()
                .data
                .rank,
            PresenterRank::Guest
        );
    }

    #[test]
    fn test_find_or_create_no_downgrade() {
        let mut sched = Schedule::default();
        let id = find_or_create_tagged_presenter(&mut sched, "G:Alice").unwrap();
        // F: = FanPanelist (priority 5 > 0) → no downgrade
        find_or_create_tagged_presenter(&mut sched, "F:Alice").unwrap();
        assert_eq!(
            sched
                .get_internal::<PresenterEntityType>(id)
                .unwrap()
                .data
                .rank,
            PresenterRank::Guest
        );
        // Bare name also must not downgrade
        find_or_create_tagged_presenter(&mut sched, "Alice").unwrap();
        assert_eq!(
            sched
                .get_internal::<PresenterEntityType>(id)
                .unwrap()
                .data
                .rank,
            PresenterRank::Guest
        );
    }

    #[test]
    fn test_find_or_create_group_membership() {
        let mut sched = Schedule::default();
        let alice_id = find_or_create_tagged_presenter(&mut sched, "P:Alice=MyBand").unwrap();
        let alice = sched.get_internal::<PresenterEntityType>(alice_id).unwrap();
        assert_eq!(alice.data.name, "Alice");
        assert!(!alice.data.is_explicit_group);

        let groups = sched
            .connected_field_nodes(alice_id, EDGE_GROUPS)
            .into_iter()
            .map(|e| unsafe { PresenterId::new_unchecked(e.entity_uuid()) })
            .collect::<Vec<PresenterId>>();
        assert_eq!(groups.len(), 1);
        let group = sched
            .get_internal::<PresenterEntityType>(groups[0])
            .unwrap();
        assert_eq!(group.data.name, "MyBand");
        assert!(group.data.is_explicit_group);
        assert!(!group.data.always_shown_in_group);
    }

    #[test]
    fn test_find_or_create_double_equals_always_shown() {
        let mut sched = Schedule::default();
        let alice_id = find_or_create_tagged_presenter(&mut sched, "P:Alice==MyBand").unwrap();
        let groups = sched
            .connected_field_nodes(alice_id, EDGE_GROUPS)
            .into_iter()
            .map(|e| unsafe { PresenterId::new_unchecked(e.entity_uuid()) })
            .collect::<Vec<PresenterId>>();
        let group = sched
            .get_internal::<PresenterEntityType>(groups[0])
            .unwrap();
        assert!(group.data.always_shown_in_group);
    }

    #[test]
    fn test_find_or_create_less_than_always_grouped() {
        let mut sched = Schedule::default();
        let alice_id = find_or_create_tagged_presenter(&mut sched, "P:<Alice=MyBand").unwrap();
        let alice = sched.get_internal::<PresenterEntityType>(alice_id).unwrap();
        assert!(alice.data.always_grouped);
    }

    #[test]
    fn test_find_or_create_group_only_form() {
        let mut sched = Schedule::default();
        let gid = find_or_create_tagged_presenter(&mut sched, "P:==MyBand").unwrap();
        let group = sched.get_internal::<PresenterEntityType>(gid).unwrap();
        assert_eq!(group.data.name, "MyBand");
        assert!(group.data.is_explicit_group);
        assert!(group.data.always_shown_in_group);
        assert_eq!(sched.entity_count::<PresenterEntityType>(), 1);
    }

    #[test]
    fn test_find_or_create_untagged_double_equals_group_only() {
        let mut sched = Schedule::default();
        let gid = find_or_create_tagged_presenter(&mut sched, "==Troupe").unwrap();
        let g = sched.get_internal::<PresenterEntityType>(gid).unwrap();
        assert_eq!(g.data.name, "Troupe");
        assert!(g.data.is_explicit_group);
        assert!(g.data.always_shown_in_group);
    }

    #[test]
    fn test_find_or_create_name_equals_group_is_group_only() {
        let mut sched = Schedule::default();
        // "Alice=Alice" — name == group → group-only, creates group
        let gid = find_or_create_tagged_presenter(&mut sched, "Alice=Alice").unwrap();
        let g = sched.get_internal::<PresenterEntityType>(gid).unwrap();
        assert!(
            g.data.is_explicit_group,
            "should be marked as explicit group"
        );
        assert_eq!(sched.entity_count::<PresenterEntityType>(), 1);
    }

    #[test]
    fn test_find_or_create_empty_returns_error() {
        let mut sched = Schedule::default();
        assert!(find_or_create_tagged_presenter(&mut sched, "").is_err());
    }

    #[test]
    fn test_lookup_single_finds_by_tagged() {
        use crate::query::lookup::lookup_single;
        let mut sched = Schedule::default();
        let id = find_or_create_tagged_presenter(&mut sched, "G:Alice").unwrap();
        let found = lookup_single::<PresenterEntityType>(&sched, "Alice").unwrap();
        assert_eq!(found, id);
    }

    #[test]
    fn test_lookup_or_create_single_creates() {
        use crate::query::lookup::lookup_or_create_single;
        let mut sched = Schedule::default();
        let id = lookup_or_create_single::<PresenterEntityType>(&mut sched, "P:Bob=Crew").unwrap();
        assert_eq!(sched.entity_count::<PresenterEntityType>(), 2);
        let d = sched.get_internal::<PresenterEntityType>(id).unwrap();
        assert_eq!(d.data.name, "Bob");
    }

    #[test]
    fn test_is_group_implicit_via_members_edge() {
        let mut sched = Schedule::default();
        let group_id = find_or_create_tagged_presenter(&mut sched, "MyBand").unwrap();
        let member_id = find_or_create_tagged_presenter(&mut sched, "Alice").unwrap();
        // Manually add MyBand to Alice's group list
        sched
            .edge_add(member_id, EDGE_GROUPS, std::iter::once(group_id))
            .expect("edge type validation failed");

        // Debug: check what's in the edge map
        eprintln!(
            "FIELD_MEMBERS on group: {:?}",
            sched.connected_field_nodes(group_id, EDGE_MEMBERS).len()
        );
        eprintln!(
            "FIELD_GROUPS on group: {:?}",
            sched.connected_field_nodes(group_id, EDGE_GROUPS).len()
        );
        eprintln!(
            "FIELD_MEMBERS on member: {:?}",
            sched.connected_field_nodes(member_id, EDGE_MEMBERS).len()
        );

        // Now is_group_entity should return true via edges_to check
        assert!(is_group_entity(&sched, group_id));
        // And find_tagged for group-only should find it
        assert_eq!(find_tagged_presenter(&sched, "=MyBand"), Some(group_id));
    }

    // ── EntityCreatable ──────────────────────────────────────────────────────

    #[test]
    fn test_can_create_no_separator_returns_from_full() {
        use crate::query::lookup::{CanCreate, EntityMatcher};
        assert!(matches!(
            PresenterEntityType::can_create("G:Alice", "G:Alice"),
            CanCreate::Yes(crate::query::lookup::MatchConsumed::Full)
        ));
    }

    #[test]
    fn test_can_create_with_separator_returns_from_partial() {
        use crate::query::lookup::{CanCreate, EntityMatcher};
        assert!(matches!(
            PresenterEntityType::can_create("G:Alice, P:Bob", "G:Alice"),
            CanCreate::Yes(crate::query::lookup::MatchConsumed::Partial)
        ));
    }

    #[test]
    fn test_can_create_empty_partial_returns_no() {
        use crate::query::lookup::{CanCreate, EntityMatcher};
        assert!(matches!(
            PresenterEntityType::can_create("G:Alice", ""),
            CanCreate::No
        ));
    }

    #[test]
    fn test_create_from_string_creates_presenter() {
        use crate::query::lookup::EntityCreatable;
        let mut sched = Schedule::default();
        let id = PresenterEntityType::create_from_string(&mut sched, "G:Alice").unwrap();
        let data = sched.get_internal(id).unwrap();
        assert_eq!(data.data.name, "Alice");
        assert_eq!(data.data.rank, PresenterRank::Guest);
    }

    #[test]
    fn test_create_from_string_tagged_with_group() {
        use crate::query::lookup::EntityCreatable;
        let mut sched = Schedule::default();
        let id = PresenterEntityType::create_from_string(&mut sched, "P:Bob=Crew").unwrap();
        let data = sched.get_internal(id).unwrap();
        assert_eq!(data.data.name, "Bob");
        assert_eq!(sched.entity_count::<PresenterEntityType>(), 2);
    }

    #[test]
    fn test_field_inclusive_groups_leaf_member_returns_parent_group() {
        let mut sched = Schedule::default();
        let member_id = make_id();
        let group_id = make_id();

        sched.insert(member_id, {
            let mut d = make_internal();
            d.id = member_id;
            d.data.name = "Alice".into();
            d
        });
        sched.insert(group_id, {
            let mut d = make_internal();
            d.id = group_id;
            d.data.name = "MyBand".into();
            d.data.is_explicit_group = true;
            d
        });

        // Add member to group
        sched
            .edge_add(member_id, EDGE_GROUPS, std::iter::once(group_id))
            .expect("edge type validation failed");

        let fs = PresenterEntityType::field_set();
        let result = fs
            .read_field_value("inclusive_groups", member_id, &sched)
            .unwrap();
        assert_eq!(
            result,
            Some(crate::schedule::entity_ids_to_field_value(vec![group_id]))
        );
    }

    #[test]
    fn test_field_inclusive_members_group_returns_direct_members() {
        let mut sched = Schedule::default();
        let group_id = make_id();
        let member1_id = make_id();
        let member2_id = make_id();

        sched.insert(group_id, {
            let mut d = make_internal();
            d.id = group_id;
            d.data.name = "MyBand".into();
            d.data.is_explicit_group = true;
            d
        });
        sched.insert(member1_id, {
            let mut d = make_internal();
            d.id = member1_id;
            d.data.name = "Alice".into();
            d
        });
        sched.insert(member2_id, {
            let mut d = make_internal();
            d.id = member2_id;
            d.data.name = "Bob".into();
            d
        });

        // Add members to group
        sched
            .edge_add(member1_id, EDGE_GROUPS, std::iter::once(group_id))
            .expect("edge type validation failed");
        sched
            .edge_add(member2_id, EDGE_GROUPS, std::iter::once(group_id))
            .expect("edge type validation failed");

        let fs = PresenterEntityType::field_set();
        let result = fs
            .read_field_value("inclusive_members", group_id, &sched)
            .unwrap();

        let FieldValue::List(items) = result.unwrap() else {
            panic!("Expected List");
        };
        assert_eq!(items.len(), 2);
        let ids: Vec<PresenterId> = items
            .into_iter()
            .map(|item| {
                let FieldValueItem::EntityIdentifier(ei) = item else {
                    panic!("Expected EntityIdentifier");
                };
                unsafe { PresenterId::new_unchecked(ei.entity_uuid()) }
            })
            .collect();
        assert!(ids.contains(&member1_id));
        assert!(ids.contains(&member2_id));
    }

    #[test]
    fn test_edge_add_remove_symmetry_groups_and_members() {
        let mut sched = Schedule::default();
        let member_id = make_id();
        let group_id = make_id();

        sched.insert(member_id, {
            let mut d = make_internal();
            d.id = member_id;
            d.data.name = "Alice".into();
            d
        });
        sched.insert(group_id, {
            let mut d = make_internal();
            d.id = group_id;
            d.data.name = "MyBand".into();
            d.data.is_explicit_group = true;
            d
        });

        // Add edge: member → group
        sched
            .edge_add(member_id, EDGE_GROUPS, std::iter::once(group_id))
            .expect("edge type validation failed");

        // Verify member's groups contains group
        assert_eq!(
            sched.connected_field_nodes(member_id, EDGE_GROUPS),
            vec![group_id.into()]
        );
        // Verify group's members contains member
        assert_eq!(
            sched.connected_field_nodes(group_id, EDGE_MEMBERS),
            vec![member_id.into()]
        );

        // Remove edge
        sched.edge_remove(member_id, EDGE_GROUPS, std::iter::once(group_id));

        // Verify both directions are cleared
        assert!(sched
            .connected_field_nodes(member_id, EDGE_GROUPS)
            .is_empty());
        assert!(sched
            .connected_field_nodes(group_id, EDGE_MEMBERS)
            .is_empty());
    }
}
