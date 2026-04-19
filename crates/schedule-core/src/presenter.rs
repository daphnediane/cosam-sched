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

use crate::converter::EntityStringResolver;
use crate::entity::{EntityId, EntityType, FieldSet, UuidPreference};
use crate::field::{FieldDescriptor, ReadFn, WriteFn};
use crate::field_macros::{
    bool_field, define_field, edge_add_field, edge_list_field_rw, edge_list_field_to_rw,
    edge_remove_field, opt_text_field, req_string_field,
};
use crate::field_value;
use crate::panel::PanelEntityType;
use crate::panel::PanelId;
use crate::value::ConversionError;
use crate::value::{CrdtFieldType, FieldType, FieldTypeItem, ValidationError};
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

inventory::submit! {
    crate::entity::RegisteredEntityType {
        type_name: PresenterEntityType::TYPE_NAME,
        uuid_namespace: PresenterEntityType::uuid_namespace,
        type_id: || std::any::TypeId::of::<PresenterInternalData>(),
    }
}
inventory::collect!(crate::entity::CollectedField<PresenterEntityType>);

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
/// or the presenter has at least one member via the homo edge map.
fn is_group_entity(schedule: &crate::schedule::Schedule, id: PresenterId) -> bool {
    schedule
        .get_internal::<PresenterEntityType>(id)
        .is_some_and(|d| d.data.is_explicit_group)
        || !schedule
            .edges_to::<PresenterEntityType, PresenterEntityType>(id)
            .is_empty()
}

/// Find a group presenter matching `name`, using `is_group_entity` as the filter.
fn find_group_by_name(schedule: &crate::schedule::Schedule, name: &str) -> Option<PresenterId> {
    schedule
        .find::<PresenterEntityType>(name)
        .into_iter()
        .find_map(|(id, _)| is_group_entity(schedule, id).then_some(id))
}

/// Find a presenter by tagged credit string; does not create entities.
///
/// Does not handle UUID strings — callers should resolve UUIDs before calling
/// (see [`EntityStringResolver::lookup_string`]).
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
        let id = schedule.find_first::<PresenterEntityType>(parsed.name)?;
        // Verify group membership if a group suffix is given
        if let Some(group_name) = parsed.group_name {
            let in_group = schedule
                .edges_from::<PresenterEntityType, PresenterEntityType>(id)
                .into_iter()
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
/// (see [`EntityStringResolver::lookup_or_create_string`]).
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
        let already_in_group = schedule
            .edges_from::<PresenterEntityType, PresenterEntityType>(pres_id)
            .contains(&gid);
        if !already_in_group {
            schedule.edge_add::<PresenterEntityType, PresenterEntityType>(pres_id, gid);
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
    let id = EntityId::from_preference(UuidPreference::GenerateNew);
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

    fn lookup_string(schedule: &crate::schedule::Schedule, s: &str) -> Option<EntityId<Self>> {
        Self::lookup_by_uuid_string(schedule, s).or_else(|| find_tagged_presenter(schedule, s))
    }

    fn lookup_or_create_string(
        schedule: &mut crate::schedule::Schedule,
        s: &str,
    ) -> Result<EntityId<Self>, ConversionError> {
        if let Some(id) = Self::lookup_by_uuid_string(schedule, s) {
            return Ok(id);
        }
        find_or_create_tagged_presenter(schedule, s)
    }
}

// ── Stored field descriptors ──────────────────────────────────────────────────

req_string_field!(FIELD_NAME, PresenterEntityType, PresenterInternalData, name,
    name: "name", display: "Name",
    desc: "Presenter or group display name.",
    aliases: &["presenter_name", "display_name"],
    example: "Alice Example",
    order: 0);

define_field!(
    /// Presenter rank — stored as `PresenterRank`, exposed as `FieldValue::String`
    /// using the canonical tag (`guest`, `judge`, `staff`, `invited_panelist`,
    /// `fan_panelist`, or a custom invited-guest label).
    static FIELD_RANK: FieldDescriptor<PresenterEntityType> = FieldDescriptor {
        name: "rank",
        display: "Rank",
        description: "Presenter classification tier.",
        aliases: &["classification"],
        required: false,
        crdt_type: CrdtFieldType::Scalar,
        field_type: FieldType::Optional(FieldTypeItem::String),
        example: "guest",
        order: 100,
        read_fn: Some(ReadFn::Bare(|d: &PresenterInternalData| {
            Some(field_value!(d.data.rank.as_str()))
        })),
        write_fn: Some(WriteFn::Bare(|d: &mut PresenterInternalData, v| {
            d.data.rank = PresenterRank::parse(&v.into_string()?);
            Ok(())
        })),
        index_fn: None,
        verify_fn: None,
    }
);

opt_text_field!(FIELD_BIO, PresenterEntityType, PresenterInternalData, bio,
    name: "bio", display: "Bio",
    desc: "Biography or description.",
    aliases: &["biography", "description"],
    example: "Long-time guest.",
    order: 200);

bool_field!(FIELD_IS_EXPLICIT_GROUP, PresenterEntityType, PresenterInternalData, is_explicit_group,
    name: "is_explicit_group", display: "Is Explicit Group",
    desc: "Marks this presenter entity as an explicit group.",
    aliases: &["explicit_group"],
    example: "false",
    order: 300);

bool_field!(FIELD_ALWAYS_GROUPED, PresenterEntityType, PresenterInternalData, always_grouped,
    name: "always_grouped", display: "Always Grouped",
    desc: "Always display this member under its group name.",
    aliases: &[],
    example: "false",
    order: 400);

bool_field!(FIELD_ALWAYS_SHOWN_IN_GROUP, PresenterEntityType, PresenterInternalData, always_shown_in_group,
    name: "always_shown_in_group", display: "Always Shown In Group",
    desc: "Always show group name even with partial member attendance.",
    aliases: &["always_shown"],
    example: "false",
    order: 500);

// ── Computed / edge-backed fields ─────────────────────────────────────────────

define_field!(
    /// `is_group` — `true` if `is_explicit_group` is set OR this presenter has
    /// any members (edge-based membership).
    static FIELD_IS_GROUP: FieldDescriptor<PresenterEntityType> = FieldDescriptor {
        name: "is_group",
        display: "Is Group",
        description: "Whether this entity represents a group (explicit flag or has members).",
        aliases: &["group"],
        required: false,
        crdt_type: CrdtFieldType::Derived,
        field_type: FieldType::Single(FieldTypeItem::Boolean),
        example: "false",
        order: 600,
        read_fn: Some(ReadFn::Schedule(|sched, id| {
            let explicit = sched
                .get_internal::<PresenterEntityType>(id)
                .is_some_and(|d| d.data.is_explicit_group);
            let has_members = !sched
                .edges_to::<PresenterEntityType, PresenterEntityType>(id)
                .is_empty();
            Some(field_value!(explicit || has_members))
        })),
        write_fn: None,
        index_fn: None,
        verify_fn: None,
    }
);

edge_list_field_rw!(FIELD_GROUPS, PresenterEntityType, PresenterInternalData, target: PresenterEntityType,
    name: "groups", display: "Groups",
    desc: "Groups this presenter belongs to.",
    aliases: &["group_memberships"],
    example: "[]",
    order: 700);

edge_list_field_to_rw!(FIELD_MEMBERS, PresenterEntityType, PresenterInternalData, source: PresenterEntityType,
    name: "members", display: "Members",
    desc: "Members of this group (empty for individuals).",
    aliases: &["group_members"],
    example: "[]",
    order: 800);

define_field!(
    /// Inclusive groups — BFS upward via homo forward edges (member → group).
    static FIELD_INCLUSIVE_GROUPS: FieldDescriptor<PresenterEntityType> = FieldDescriptor {
        name: "inclusive_groups",
        display: "Inclusive Groups",
        description: "Transitive closure of groups this presenter appears in.",
        aliases: &[],
        required: false,
        crdt_type: CrdtFieldType::Derived,
        field_type: FieldType::List(FieldTypeItem::EntityIdentifier(
            PresenterEntityType::TYPE_NAME,
        )),
        example: "[]",
        order: 900,
        read_fn: Some(ReadFn::Schedule(|sched, id| {
            use std::collections::HashSet;
            let mut visited: HashSet<PresenterId> = HashSet::new();
            let mut queue = vec![id];
            while let Some(curr) = queue.pop() {
                for g in sched.edges_from::<PresenterEntityType, PresenterEntityType>(curr) {
                    if visited.insert(g) {
                        queue.push(g);
                    }
                }
            }
            let ids: Vec<PresenterId> = visited.into_iter().collect();
            Some(crate::schedule::entity_ids_to_field_value(ids))
        })),
        write_fn: None,
        index_fn: None,
        verify_fn: None,
    }
);

define_field!(
    /// Inclusive members — BFS downward via homo reverse edges (group → member).
    static FIELD_INCLUSIVE_MEMBERS: FieldDescriptor<PresenterEntityType> = FieldDescriptor {
        name: "inclusive_members",
        display: "Inclusive Members",
        description: "Transitive closure of members for this group.",
        aliases: &[],
        required: false,
        crdt_type: CrdtFieldType::Derived,
        field_type: FieldType::List(FieldTypeItem::EntityIdentifier(
            PresenterEntityType::TYPE_NAME,
        )),
        example: "[]",
        order: 1000,
        read_fn: Some(ReadFn::Schedule(|sched, id| {
            use std::collections::HashSet;
            let mut visited: HashSet<PresenterId> = HashSet::new();
            let mut queue = vec![id];
            while let Some(curr) = queue.pop() {
                for m in sched.edges_to::<PresenterEntityType, PresenterEntityType>(curr) {
                    if visited.insert(m) {
                        queue.push(m);
                    }
                }
            }
            let ids: Vec<PresenterId> = visited.into_iter().collect();
            Some(crate::schedule::entity_ids_to_field_value(ids))
        })),
        write_fn: None,
        index_fn: None,
        verify_fn: None,
    }
);

edge_list_field_rw!(FIELD_PANELS, PresenterEntityType, PresenterInternalData, target: PanelEntityType,
    name: "panels", display: "Panels",
    desc: "Panels this presenter is scheduled on.",
    aliases: &["panel"],
    example: "[]",
    order: 1100);

edge_add_field!(FIELD_ADD_PANELS, PresenterEntityType, PresenterInternalData, target: PanelEntityType,
    name: "add_panels", display: "Add Panels",
    desc: "Append panels to this presenter.",
    aliases: &["add_panel"],
    example: "[panel_id]",
    order: 1200);

edge_remove_field!(FIELD_REMOVE_PANELS, PresenterEntityType, PresenterInternalData, target: PanelEntityType,
    name: "remove_panels", display: "Remove Panels",
    desc: "Remove panels from this presenter.",
    aliases: &["remove_panel"],
    example: "[panel_id]",
    order: 1300);

define_field!(
    /// Inclusive panels — direct panels + panels of all inclusive groups.
    static FIELD_INCLUSIVE_PANELS: FieldDescriptor<PresenterEntityType> = FieldDescriptor {
        name: "inclusive_panels",
        display: "Inclusive Panels",
        description: "Transitive closure: panels of this presenter and of its groups.",
        aliases: &[],
        required: false,
        crdt_type: CrdtFieldType::Derived,
        field_type: FieldType::List(FieldTypeItem::EntityIdentifier(PanelEntityType::TYPE_NAME)),
        example: "[]",
        order: 1400,
        read_fn: Some(ReadFn::Schedule(|sched, id| {
            use std::collections::HashSet;
            // Collect inclusive groups (BFS upward)
            let mut group_visited: HashSet<PresenterId> = HashSet::new();
            let mut queue = vec![id];
            while let Some(curr) = queue.pop() {
                for g in sched.edges_from::<PresenterEntityType, PresenterEntityType>(curr) {
                    if group_visited.insert(g) {
                        queue.push(g);
                    }
                }
            }
            // Union of direct panels + panels of each inclusive group
            let mut panel_set: HashSet<PanelId> = HashSet::new();
            for p in sched.edges_from::<PresenterEntityType, PanelEntityType>(id) {
                panel_set.insert(p);
            }
            for g in &group_visited {
                for p in sched.edges_from::<PresenterEntityType, PanelEntityType>(*g) {
                    panel_set.insert(p);
                }
            }
            let ids: Vec<PanelId> = panel_set.into_iter().collect();
            Some(crate::schedule::entity_ids_to_field_value(ids))
        })),
        write_fn: None,
        index_fn: None,
        verify_fn: None,
    }
);

// ── FieldSet ──────────────────────────────────────────────────────────────────

static PRESENTER_FIELD_SET: LazyLock<FieldSet<PresenterEntityType>> =
    LazyLock::new(FieldSet::from_inventory);

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::MatchPriority;
    use crate::field_value;
    use crate::schedule::Schedule;
    use crate::value::FieldError;
    use uuid::Uuid;

    fn make_id() -> PresenterId {
        PresenterId::new(Uuid::new_v4()).expect("v4 is never nil")
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
        assert_eq!(fs.fields().count(), 15);
        let required: Vec<_> = fs.required_fields().map(|d| d.name).collect();
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
    fn test_match_name_prefix() {
        let fs = PresenterEntityType::field_set();
        let data = make_internal();
        let priority = fs.match_index("alice", &data);
        assert_eq!(priority, Some(MatchPriority::Prefix));
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
        use crate::converter::EntityStringResolver;
        let id = make_id();
        let sched = schedule_with(id, make_internal());
        let s = PresenterEntityType::entity_to_string(&sched, id);
        assert_eq!(s, "Alice Example");
    }

    #[test]
    fn test_entity_to_string_fallback_to_uuid() {
        use crate::converter::EntityStringResolver;
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
        sched.edge_add::<PresenterEntityType, PresenterEntityType>(alice_id, group_id);

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

        let groups = sched.edges_from::<PresenterEntityType, PresenterEntityType>(alice_id);
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
        let groups = sched.edges_from::<PresenterEntityType, PresenterEntityType>(alice_id);
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
    fn test_lookup_string_finds_by_tagged() {
        use crate::converter::EntityStringResolver;
        let mut sched = Schedule::default();
        let id = find_or_create_tagged_presenter(&mut sched, "G:Alice").unwrap();
        let found = PresenterEntityType::lookup_string(&sched, "Alice");
        assert_eq!(found, Some(id));
    }

    #[test]
    fn test_lookup_or_create_string_creates() {
        use crate::converter::EntityStringResolver;
        let mut sched = Schedule::default();
        let id = PresenterEntityType::lookup_or_create_string(&mut sched, "P:Bob=Crew").unwrap();
        assert_eq!(sched.entity_count::<PresenterEntityType>(), 2);
        let d = sched.get_internal::<PresenterEntityType>(id).unwrap();
        assert_eq!(d.data.name, "Bob");
    }

    #[test]
    fn test_is_group_implicit_via_members_edge() {
        let mut sched = Schedule::default();
        let group_id = find_or_create_tagged_presenter(&mut sched, "MyBand").unwrap();
        let member_id = find_or_create_tagged_presenter(&mut sched, "Alice").unwrap();
        // Manually add member → group edge (group is NOT is_explicit_group yet)
        sched.edge_add::<PresenterEntityType, PresenterEntityType>(member_id, group_id);
        // Now is_group_entity should return true via edges_to check
        assert!(is_group_entity(&sched, group_id));
        // And find_tagged for group-only should find it
        assert_eq!(find_tagged_presenter(&sched, "=MyBand"), Some(group_id));
    }
}
