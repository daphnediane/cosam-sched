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
//! Groups and group membership are modeled as edges between presenters; the
//! edge-backed computed fields (`groups`, `members`, `inclusive_*`, `panels`)
//! are stubs here and fully wired in FEATURE-018.

use crate::entity::{EntityId, EntityType, FieldSet};
use crate::field::{FieldDescriptor, ReadFn, WriteFn};
use crate::field_macros::{
    bool_field, edge_list_field, edge_list_field_rw, edge_mutator_field, opt_text_field,
    req_string_field,
};
use crate::panel::PanelId;
use crate::value::{CrdtFieldType, ValidationError};
use crate::{field_boolean, field_string};
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
    /// The computed `is_group` field may also reflect edge-backed membership
    /// once FEATURE-018 lands.
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
    /// Groups this presenter belongs to — from edge maps (FEATURE-018).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub group_ids: Vec<PresenterId>,
    /// Panels this presenter is on — from edge maps (FEATURE-018).
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

// ── Stored field descriptors ──────────────────────────────────────────────────

req_string_field!(FIELD_NAME, PresenterEntityType, PresenterInternalData, name,
    name: "name", display: "Name",
    desc: "Presenter or group display name.",
    aliases: &["presenter_name", "display_name"],
    example: "Alice Example");

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
    example: "guest",
    read_fn: Some(ReadFn::Bare(|d: &PresenterInternalData| {
        Some(field_string!(d.data.rank.as_str()))
    })),
    write_fn: Some(WriteFn::Bare(|d: &mut PresenterInternalData, v| {
        d.data.rank = PresenterRank::parse(&v.into_string()?);
        Ok(())
    })),
    index_fn: None,
    verify_fn: None,
};

opt_text_field!(FIELD_BIO, PresenterEntityType, PresenterInternalData, bio,
    name: "bio", display: "Bio",
    desc: "Biography or description.",
    aliases: &["biography", "description"],
    example: "Long-time guest.");

bool_field!(FIELD_IS_EXPLICIT_GROUP, PresenterEntityType, PresenterInternalData, is_explicit_group,
    name: "is_explicit_group", display: "Is Explicit Group",
    desc: "Marks this presenter entity as an explicit group.",
    aliases: &["explicit_group"],
    example: "false");

bool_field!(FIELD_ALWAYS_GROUPED, PresenterEntityType, PresenterInternalData, always_grouped,
    name: "always_grouped", display: "Always Grouped",
    desc: "Always display this member under its group name.",
    aliases: &[],
    example: "false");

bool_field!(FIELD_ALWAYS_SHOWN_IN_GROUP, PresenterEntityType, PresenterInternalData, always_shown_in_group,
    name: "always_shown_in_group", display: "Always Shown In Group",
    desc: "Always show group name even with partial member attendance.",
    aliases: &["always_shown"],
    example: "false");

// ── Computed / edge-backed field stubs (full wiring in FEATURE-018) ───────────

/// `is_group` — `Derived` bool that currently mirrors `is_explicit_group`; once
/// FEATURE-018 adds edge-backed membership, this also returns `true` when edge
/// storage records any members.
static FIELD_IS_GROUP: FieldDescriptor<PresenterEntityType> = FieldDescriptor {
    name: "is_group",
    display: "Is Group",
    description: "Whether this entity represents a group (explicit or by membership).",
    aliases: &["group"],
    required: false,
    crdt_type: CrdtFieldType::Derived,
    example: "false",
    read_fn: Some(ReadFn::Bare(|d: &PresenterInternalData| {
        Some(field_boolean!(d.data.is_explicit_group))
    })),
    write_fn: None,
    index_fn: None,
    verify_fn: None,
};

edge_list_field_rw!(FIELD_GROUPS, PresenterEntityType, PresenterInternalData,
    name: "groups", display: "Groups",
    desc: "Groups this presenter belongs to.",
    aliases: &["group_memberships"],
    example: "[]");

edge_list_field_rw!(FIELD_MEMBERS, PresenterEntityType, PresenterInternalData,
    name: "members", display: "Members",
    desc: "Members of this group (empty for individuals).",
    aliases: &["group_members"],
    example: "[]");

edge_list_field!(FIELD_INCLUSIVE_GROUPS, PresenterEntityType, PresenterInternalData,
    name: "inclusive_groups", display: "Inclusive Groups",
    desc: "Transitive closure of groups this presenter appears in.",
    aliases: &[],
    example: "[]");

edge_list_field!(FIELD_INCLUSIVE_MEMBERS, PresenterEntityType, PresenterInternalData,
    name: "inclusive_members", display: "Inclusive Members",
    desc: "Transitive closure of members for this group.",
    aliases: &[],
    example: "[]");

edge_list_field_rw!(FIELD_PANELS, PresenterEntityType, PresenterInternalData,
    name: "panels", display: "Panels",
    desc: "Panels this presenter is scheduled on.",
    aliases: &["panel"],
    example: "[]");

edge_mutator_field!(FIELD_ADD_PANELS, PresenterEntityType, PresenterInternalData,
    name: "add_panels", display: "Add Panels",
    desc: "Append panels to this presenter.",
    aliases: &["add_panel"],
    example: "[panel_id]");

edge_mutator_field!(FIELD_REMOVE_PANELS, PresenterEntityType, PresenterInternalData,
    name: "remove_panels", display: "Remove Panels",
    desc: "Remove panels from this presenter.",
    aliases: &["remove_panel"],
    example: "[panel_id]");

edge_list_field!(FIELD_INCLUSIVE_PANELS, PresenterEntityType, PresenterInternalData,
    name: "inclusive_panels", display: "Inclusive Panels",
    desc: "Transitive closure: panels of this presenter and of its groups.",
    aliases: &[],
    example: "[]");

// ── FieldSet ──────────────────────────────────────────────────────────────────

static PRESENTER_FIELD_SET: LazyLock<FieldSet<PresenterEntityType>> = LazyLock::new(|| {
    FieldSet::new(&[
        &FIELD_NAME,
        &FIELD_RANK,
        &FIELD_BIO,
        &FIELD_IS_EXPLICIT_GROUP,
        &FIELD_ALWAYS_GROUPED,
        &FIELD_ALWAYS_SHOWN_IN_GROUP,
        &FIELD_IS_GROUP,
        &FIELD_GROUPS,
        &FIELD_MEMBERS,
        &FIELD_INCLUSIVE_GROUPS,
        &FIELD_INCLUSIVE_MEMBERS,
        &FIELD_PANELS,
        &FIELD_ADD_PANELS,
        &FIELD_REMOVE_PANELS,
        &FIELD_INCLUSIVE_PANELS,
    ])
});

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::MatchPriority;
    use crate::schedule::Schedule;
    use crate::value::FieldError;
    use crate::{field_boolean, field_string, field_value};
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
            Some(field_string!("Alice Example"))
        );
        assert_eq!(
            fs.read_field_value("rank", id, &sched).unwrap(),
            Some(field_string!("guest"))
        );
    }

    #[test]
    fn test_write_rank_custom_invited() {
        let id = make_id();
        let mut sched = schedule_with(id, make_internal());
        let fs = PresenterEntityType::field_set();
        fs.write_field_value("rank", id, &mut sched, field_string!("Sponsor"))
            .unwrap();
        let value = fs.read_field_value("rank", id, &sched).unwrap();
        assert_eq!(value, Some(field_string!("Sponsor")));
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
            Some(field_boolean!(true))
        );
    }

    #[test]
    fn test_is_group_is_read_only() {
        let id = make_id();
        let mut sched = schedule_with(id, make_internal());
        let fs = PresenterEntityType::field_set();
        let result = fs.write_field_value("is_group", id, &mut sched, field_boolean!(true));
        assert!(matches!(result, Err(FieldError::ReadOnly { .. })));
    }

    #[test]
    fn test_edge_stubs_return_empty_list() {
        let id = make_id();
        let sched = schedule_with(id, make_internal());
        let fs = PresenterEntityType::field_set();
        for name in ["groups", "members", "inclusive_groups", "panels"] {
            assert_eq!(
                fs.read_field_value(name, id, &sched).unwrap(),
                Some(field_value!(empty_list))
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
}
