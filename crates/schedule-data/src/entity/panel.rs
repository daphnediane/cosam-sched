/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Panel entity — one event on the schedule.
//!
//! Fields map directly to spreadsheet columns from the **Schedule** sheet.
//! Time information is held in a [`TimeRange`] backing field (`time_slot`)
//! and exposed through computed `start_time`, `end_time`, and `duration`
//! fields.  Relationship data (presenters, room, panel type) is available
//! through schedule-aware computed fields backed by edges (FEATURE-007).
//!
//! [`TimeRange`]: crate::time::TimeRange

use crate::EntityFields;

/// A panel (event) on the schedule.
///
/// ## Stored fields
///
/// All stored fields correspond directly to **Schedule** sheet columns.  The
/// raw `uid` string is also stored parsed in `parsed_uid` (set during import).
/// Timing is held as a [`TimeRange`] in `time_slot` and projected as
/// `start_time`, `end_time`, and `duration` computed fields.
///
/// ## Cost field
///
/// The `cost` field stores the raw spreadsheet value (e.g. `"$35"`, `"Free"`,
/// `"Kids"`, `"*"`).  The `is_free` and `is_kids` booleans are set during
/// import by parsing the cost string and are stored independently.
///
/// [`TimeRange`]: crate::time::TimeRange
#[derive(EntityFields, Debug, Clone)]
#[entity_kind(Panel)]
pub struct Panel {
    // --- Raw spreadsheet columns -------------------------------------------
    #[field(
        display = "Uniq ID",
        description = "Panel identifier from the spreadsheet"
    )]
    #[alias("uid", "id", "uniq_id")]
    #[required]
    #[indexable(priority = 220)]
    pub uid: String,

    #[field(display = "Name", description = "Panel name / title")]
    #[alias("name", "title", "panel_name")]
    #[required]
    #[indexable(priority = 210)]
    pub name: String,

    #[field(
        display = "Description",
        description = "Event description shown to attendees"
    )]
    #[alias("description", "desc")]
    pub description: Option<String>,

    #[field(display = "Note", description = "Extra note displayed verbatim")]
    #[alias("note")]
    pub note: Option<String>,

    #[field(
        display = "Notes (Non Printing)",
        description = "Internal notes not shown to the public"
    )]
    #[alias("notes_non_printing", "internal_notes")]
    pub notes_non_printing: Option<String>,

    #[field(display = "Workshop Notes", description = "Notes for workshop staff")]
    #[alias("workshop_notes")]
    pub workshop_notes: Option<String>,

    #[field(
        display = "Power Needs",
        description = "Power / electrical requirements"
    )]
    #[alias("power_needs", "power")]
    pub power_needs: Option<String>,

    #[field(
        display = "Sewing Machines",
        description = "Whether sewing machines are required"
    )]
    #[alias("sewing_machines", "sewing")]
    pub sewing_machines: bool,

    #[field(display = "AV Notes", description = "Audio/visual setup notes")]
    #[alias("av_notes", "av")]
    pub av_notes: Option<String>,

    #[field(
        display = "Difficulty",
        description = "Skill-level indicator (free text)"
    )]
    #[alias("difficulty")]
    pub difficulty: Option<String>,

    #[field(
        display = "Prerequisites",
        description = "Comma-separated prerequisite Uniq IDs"
    )]
    #[alias("prereq", "prerequisites")]
    pub prereq: Option<String>,

    #[field(
        display = "Cost",
        description = "Raw cost value (e.g. \"$35\", \"Free\", \"Kids\", \"*\")"
    )]
    #[alias("cost")]
    pub cost: Option<String>,

    #[field(
        display = "Is Free",
        description = "True when cost is blank, \"Free\", \"$0\", or \"N/A\" (set during import)"
    )]
    #[alias("is_free", "free")]
    pub is_free: bool,

    #[field(
        display = "Is Kids",
        description = "True when cost is \"Kids\" (set during import)"
    )]
    #[alias("is_kids", "kids")]
    pub is_kids: bool,

    #[field(
        display = "Full",
        description = "Non-blank value means the event is at capacity"
    )]
    #[alias("is_full", "full")]
    pub is_full: bool,

    #[field(display = "Capacity", description = "Total seats available")]
    #[alias("capacity")]
    pub capacity: Option<i64>,

    #[field(
        display = "Seats Sold",
        description = "Number of seats pre-sold or reserved via ticketing"
    )]
    #[alias("seats_sold")]
    pub seats_sold: Option<i64>,

    #[field(
        display = "Pre-reg Max",
        description = "Maximum seats available for pre-registration"
    )]
    #[alias("pre_reg_max", "prereg_max")]
    pub pre_reg_max: Option<i64>,

    #[field(
        display = "Ticket URL",
        description = "URL for purchasing tickets (\"Ticket Sale\" / \"Ticket URL\" columns)"
    )]
    #[alias("ticket_url", "ticket_sale")]
    pub ticket_url: Option<String>,

    #[field(
        display = "Have Ticket Image",
        description = "Whether a ticket/flyer image has been received and uploaded"
    )]
    #[alias("have_ticket_image")]
    pub have_ticket_image: bool,

    #[field(
        display = "SimpleTix Event",
        description = "Link to the SimpleTix admin portal for this event"
    )]
    #[alias("simpletix_event", "simpletix")]
    pub simpletix_event: Option<String>,

    #[field(
        display = "Hide Panelist",
        description = "Non-blank to suppress presenter credits"
    )]
    #[alias("hide_panelist")]
    pub hide_panelist: bool,

    #[field(
        display = "Alt Panelist",
        description = "Override text for the presenter credits line"
    )]
    #[alias("alt_panelist")]
    pub alt_panelist: Option<String>,

    // --- Internal storage (no field-system exposure) ------------------------
    /// Parsed components of [`uid`](Panel::uid); populated during import.
    pub parsed_uid: Option<crate::entity::PanelUniqId>,

    /// Canonical time-slot storage; exposed through computed fields below.
    pub time_slot: crate::time::TimeRange,

    /// Backing storage for presenter relationships (owned forward side).
    /// Updated by the `presenters` computed field write closure and
    /// `PanelEntityType` presenter helpers.
    pub presenter_ids: Vec<crate::entity::PresenterId>,

    /// Backing storage for the event room relationship (owned forward side).
    /// Updated by the `event_room` computed field write closure.
    pub event_room_id: Option<crate::entity::EventRoomId>,

    /// Backing storage for the panel type relationship (owned forward side).
    /// Updated by the `panel_type` computed field write closure.
    pub panel_type_id: Option<crate::entity::PanelTypeId>,

    // --- Computed: time_slot projections ------------------------------------
    #[computed_field(display = "Start Time", description = "Panel start time (ISO-8601)")]
    #[alias("start_time")]
    #[read(|entity: &PanelData| {
        entity.time_slot.start_time().map(|dt| {
            crate::field::FieldValue::String(crate::time::format_storage(dt))
        })
    })]
    #[write(|entity: &mut PanelData, value: crate::field::FieldValue| {
        match value {
            crate::field::FieldValue::String(s) => {
                match crate::time::parse_datetime(&s) {
                    Some(dt) => { entity.time_slot.add_start_time(dt); Ok(()) }
                    None => Err(crate::field::FieldError::from(
                        crate::field::validation::ConversionError::InvalidTimestamp
                    )),
                }
            }
            _ => Err(crate::field::FieldError::from(
                crate::field::validation::ConversionError::UnsupportedType
            )),
        }
    })]
    pub start_time: Option<String>,

    #[computed_field(display = "End Time", description = "Panel end time (ISO-8601)")]
    #[alias("end_time")]
    #[read(|entity: &PanelData| {
        entity.time_slot.end_time().map(|dt| {
            crate::field::FieldValue::String(crate::time::format_storage(dt))
        })
    })]
    #[write(|entity: &mut PanelData, value: crate::field::FieldValue| {
        match value {
            crate::field::FieldValue::String(s) => {
                match crate::time::parse_datetime(&s) {
                    Some(dt) => { entity.time_slot.add_end_time(dt); Ok(()) }
                    None => Err(crate::field::FieldError::from(
                        crate::field::validation::ConversionError::InvalidTimestamp
                    )),
                }
            }
            _ => Err(crate::field::FieldError::from(
                crate::field::validation::ConversionError::UnsupportedType
            )),
        }
    })]
    pub end_time: Option<String>,

    #[computed_field(display = "Duration", description = "Panel duration in whole minutes")]
    #[alias("duration")]
    #[read(|entity: &PanelData| {
        entity.time_slot.duration().map(|d| {
            crate::field::FieldValue::Integer(d.num_minutes())
        })
    })]
    #[write(|entity: &mut PanelData, value: crate::field::FieldValue| {
        let minutes = match value {
            crate::field::FieldValue::Integer(m) => Some(m),
            crate::field::FieldValue::String(ref s) => {
                crate::time::parse_duration(s).map(|d| d.num_minutes())
            }
            _ => None,
        };
        match minutes {
            Some(m) => {
                entity.time_slot.add_duration(chrono::Duration::minutes(m));
                Ok(())
            }
            None => Err(crate::field::FieldError::from(
                crate::field::validation::ConversionError::InvalidFormat
            )),
        }
    })]
    pub duration: Option<i64>,

    // --- Computed: schedule-aware (edge-based) --------------------------------
    #[computed_field(
        display = "Presenters",
        description = "All presenters credited for this panel"
    )]
    #[alias("presenters", "panelists")]
    #[read(|_schedule: &crate::schedule::Schedule, entity: &PanelData| {
        if entity.presenter_ids.is_empty() {
            None
        } else {
            Some(crate::field::FieldValue::List(
                entity.presenter_ids.iter()
                    .map(|id| crate::field::FieldValue::NonNilUuid(id.non_nil_uuid()))
                    .collect(),
            ))
        }
    })]
    #[write(|schedule: &mut crate::schedule::Schedule, entity: &mut PanelData, value: crate::field::FieldValue| {
        use crate::entity::{InternalData, PanelToPresenterEntityType, PresenterId};
        let panel_uuid = entity.uuid();
        let new_presenter_uuids: Vec<uuid::NonNilUuid> = match value {
            crate::field::FieldValue::List(items) => items
                .into_iter()
                .filter_map(|v| if let crate::field::FieldValue::NonNilUuid(u) = v { Some(u) } else { None })
                .collect(),
            crate::field::FieldValue::NonNilUuid(u) => vec![u],
            _ => return Err(crate::field::FieldError::ConversionError(
                crate::field::validation::ConversionError::InvalidFormat,
            )),
        };
        entity.presenter_ids = new_presenter_uuids
            .iter()
            .map(|&u| PresenterId::from_uuid(u))
            .collect();
        PanelToPresenterEntityType::set_presenters(&mut schedule.entities, panel_uuid, &new_presenter_uuids)
            .map_err(|_| crate::field::FieldError::ConversionError(
                crate::field::validation::ConversionError::InvalidFormat,
            ))
    })]
    pub presenters: Vec<crate::entity::PresenterId>,

    /// Add individual presenters to this panel without replacing existing ones.
    /// Write-only computed field that accepts a single UUID/string or list of UUIDs/strings.
    /// String values are resolved via tagged lookup (e.g., "G:Alice", "presenter-<uuid>").
    #[computed_field(
        display = "Add Presenters",
        description = "Add presenters to this panel (append mode)"
    )]
    #[write(|schedule: &mut crate::schedule::Schedule, entity: &mut PanelData, value: crate::field::FieldValue| {
        use crate::entity::{InternalData, PanelToPresenterEntityType, PresenterEntityType};
        let panel_uuid = entity.uuid();
        let values: Vec<crate::field::FieldValue> = match value {
            crate::field::FieldValue::List(items) => items,
            single => vec![single],
        };
        for v in values {
            if let Ok(presenter_id) = PresenterEntityType::resolve_field_value(&mut schedule.entities, v) {
                if !entity.presenter_ids.contains(&presenter_id) {
                    entity.presenter_ids.push(presenter_id);
                }
                PanelToPresenterEntityType::add_presenters(
                    &mut schedule.entities,
                    panel_uuid,
                    &[presenter_id],
                );
            }
        }
        Ok(())
    })]
    pub add_presenters: Vec<crate::entity::PresenterId>,

    /// Remove individual presenters from this panel.
    /// Write-only computed field that accepts a single UUID/string or list of UUIDs/strings.
    /// String values are resolved via tagged lookup (e.g., "presenter-<uuid>").
    #[computed_field(
        display = "Remove Presenters",
        description = "Remove presenters from this panel"
    )]
    #[write(|schedule: &mut crate::schedule::Schedule, entity: &mut PanelData, value: crate::field::FieldValue| {
        use crate::entity::{InternalData, PanelToPresenterEntityType, PresenterEntityType};
        let panel_uuid = entity.uuid();
        let values: Vec<crate::field::FieldValue> = match value {
            crate::field::FieldValue::List(items) => items,
            single => vec![single],
        };
        let mut to_remove = Vec::new();
        for v in values {
            if let Ok(presenter_id) = PresenterEntityType::resolve_field_value(&mut schedule.entities, v) {
                entity.presenter_ids.retain(|id| id != &presenter_id);
                to_remove.push(presenter_id);
            }
        }
        PanelToPresenterEntityType::remove_presenters(&mut schedule.entities, panel_uuid, &to_remove);
        Ok(())
    })]
    pub remove_presenters: Vec<crate::entity::PresenterId>,

    /// Transitive closure of all presenters for this panel.
    ///
    /// Includes: direct presenters + their groups (upward) + members of groups (downward).
    /// This is the full set used for conflict checking and credit display.
    #[computed_field(
        display = "Inclusive Presenters",
        description = "Transitive closure: direct presenters + their groups + group members"
    )]
    #[alias("inclusive_presenter")]
    #[read(|schedule: &crate::schedule::Schedule, entity: &PanelData| {
        use crate::entity::{PresenterEntityType, PresenterToGroupEntityType};
        use std::collections::HashSet;

        let direct = entity.presenter_ids.clone();

        let mut result = Vec::new();
        let mut seen = HashSet::new();

        for presenter_id in direct {
            let presenter_uuid = presenter_id.non_nil_uuid();

            // Add the direct presenter
            if seen.insert(presenter_uuid) {
                result.push(crate::field::FieldValue::NonNilUuid(presenter_uuid));
            }

            // Upward: add all groups this presenter belongs to (transitive)
            for group_id in PresenterToGroupEntityType::inclusive_groups_of(&schedule.entities, presenter_uuid) {
                let group_uuid = group_id.non_nil_uuid();
                if seen.insert(group_uuid) {
                    result.push(crate::field::FieldValue::NonNilUuid(group_uuid));
                }
            }

            // Downward: if this presenter is a group, add its members (transitive)
            if PresenterEntityType::is_group(&schedule.entities, presenter_uuid) {
                for member_id in PresenterToGroupEntityType::inclusive_members_of(&schedule.entities, presenter_uuid) {
                    let member_uuid = member_id.non_nil_uuid();
                    if seen.insert(member_uuid) {
                        result.push(crate::field::FieldValue::NonNilUuid(member_uuid));
                    }
                }
            }
        }
        if result.is_empty() {
            None
        } else {
            Some(crate::field::FieldValue::List(result))
        }
    })]
    pub inclusive_presenters: Vec<crate::entity::PresenterId>,

    #[computed_field(
        display = "Event Room",
        description = "Room where this panel takes place"
    )]
    #[alias("room", "event_room")]
    #[read(|_schedule: &crate::schedule::Schedule, entity: &PanelData| {
        entity.event_room_id.map(|id| crate::field::FieldValue::NonNilUuid(id.non_nil_uuid()))
    })]
    #[write(|schedule: &mut crate::schedule::Schedule, entity: &mut PanelData, value: crate::field::FieldValue| {
        use crate::entity::{EventRoomId, InternalData, PanelToEventRoomEntityType};
        let panel_uuid = entity.uuid();
        let event_room_uuid = match value {
            crate::field::FieldValue::NonNilUuid(u) => u,
            _ => return Err(crate::field::FieldError::ConversionError(
                crate::field::validation::ConversionError::InvalidFormat,
            )),
        };
        entity.event_room_id = Some(EventRoomId::from_uuid(event_room_uuid));
        PanelToEventRoomEntityType::set_event_room(&mut schedule.entities, panel_uuid, event_room_uuid)
            .map_err(|_| crate::field::FieldError::ConversionError(
                crate::field::validation::ConversionError::InvalidFormat,
            ))
    })]
    pub event_room: Option<String>,

    #[computed_field(
        display = "Panel Type",
        description = "Type / category of this panel (e.g. \"Guest Panel\", \"Workshop\")"
    )]
    #[alias("panel_type", "kind", "type")]
    #[read(|_schedule: &crate::schedule::Schedule, entity: &PanelData| {
        entity.panel_type_id.map(|id| crate::field::FieldValue::NonNilUuid(id.non_nil_uuid()))
    })]
    #[write(|schedule: &mut crate::schedule::Schedule, entity: &mut PanelData, value: crate::field::FieldValue| {
        use crate::entity::{InternalData, PanelToPanelTypeEntityType, PanelTypeId};
        let panel_uuid = entity.uuid();
        let panel_type_uuid = match value {
            crate::field::FieldValue::NonNilUuid(u) => u,
            _ => return Err(crate::field::FieldError::ConversionError(
                crate::field::validation::ConversionError::InvalidFormat,
            )),
        };
        entity.panel_type_id = Some(PanelTypeId::from_uuid(panel_type_uuid));
        PanelToPanelTypeEntityType::set_panel_type(&mut schedule.entities, panel_uuid, panel_type_uuid)
            .map(|_| ())
            .map_err(|_| crate::field::FieldError::ConversionError(
                crate::field::validation::ConversionError::InvalidFormat,
            ))
    })]
    pub panel_type: Option<String>,
}

impl crate::entity::SchedulableEntity for PanelEntityType {}

// ---------------------------------------------------------------------------
// PanelEntityType presenter management methods
// ---------------------------------------------------------------------------

impl PanelEntityType {
    /// Resolve a FieldValue to a PanelId.
    ///
    /// Supports:
    /// - `FieldValue::NonNilUuid(u)` -> lookup by UUID
    pub fn resolve_field_value(
        storage: &crate::schedule::EntityStorage,
        value: crate::field::FieldValue,
    ) -> Result<PanelId, crate::schedule::LookupError> {
        match value {
            crate::field::FieldValue::NonNilUuid(uuid) => {
                if storage.panels.contains_key(&uuid) {
                    Ok(PanelId::from_uuid(uuid))
                } else {
                    Err(crate::schedule::LookupError::UuidNotFound(uuid.into()))
                }
            }
            _ => Err(crate::schedule::LookupError::Empty),
        }
    }

    /// Add presenters to a panel by resolving FieldValues to presenter IDs.
    ///
    /// Each FieldValue can be either:
    /// - A UUID (`NonNilUuid`) for direct presenter reference
    /// - A string for tagged lookup (e.g., "G:Alice", "presenter-<uuid>")
    ///
    /// Returns the number of presenters successfully added. Errors for individual
    /// values are silently ignored.
    pub fn add_presenters(
        storage: &mut crate::schedule::EntityStorage,
        panel_uuid: uuid::NonNilUuid,
        values: Vec<crate::field::FieldValue>,
    ) -> usize {
        use crate::entity::{PanelToPresenterEntityType, PresenterEntityType};

        let mut added = 0;
        for value in values {
            match PresenterEntityType::resolve_field_value(storage, value) {
                Ok(presenter_id) => {
                    let count = PanelToPresenterEntityType::add_presenters(
                        storage,
                        panel_uuid,
                        &[presenter_id],
                    );
                    if count > 0 {
                        if let Some(panel_data) = storage.panels.get_mut(&panel_uuid) {
                            if !panel_data.presenter_ids.contains(&presenter_id) {
                                panel_data.presenter_ids.push(presenter_id);
                            }
                        }
                    }
                    added += count;
                }
                Err(_) => {
                    continue;
                }
            }
        }
        added
    }

    /// Remove presenters from a panel by resolving FieldValues to presenter IDs.
    ///
    /// Each FieldValue can be either:
    /// - A UUID (`NonNilUuid`) for direct presenter reference
    /// - A string for tagged lookup (e.g., "presenter-<uuid>")
    ///
    /// Returns the number of presenters successfully removed.
    pub fn remove_presenters(
        storage: &mut crate::schedule::EntityStorage,
        panel_uuid: uuid::NonNilUuid,
        values: Vec<crate::field::FieldValue>,
    ) -> usize {
        use crate::entity::{PanelToPresenterEntityType, PresenterEntityType};

        let mut removed = 0;
        for value in values {
            match PresenterEntityType::resolve_field_value(storage, value) {
                Ok(presenter_id) => {
                    let count = PanelToPresenterEntityType::remove_presenters(
                        storage,
                        panel_uuid,
                        &[presenter_id],
                    );
                    if count > 0 {
                        if let Some(panel_data) = storage.panels.get_mut(&panel_uuid) {
                            panel_data.presenter_ids.retain(|id| id != &presenter_id);
                        }
                    }
                    removed += count;
                }
                Err(_) => {
                    continue;
                }
            }
        }
        removed
    }

    /// Add presenters to a panel by parsing tag strings.
    ///
    /// This is a convenience method for spreadsheet import that takes raw tag
    /// strings (e.g., "G:Alice=TeamA") and resolves them to presenters.
    ///
    /// Returns the number of presenters successfully added.
    pub fn add_presenters_tagged(
        storage: &mut crate::schedule::EntityStorage,
        panel_uuid: uuid::NonNilUuid,
        tags: &[&str],
    ) -> usize {
        use crate::entity::{PanelToPresenterEntityType, PresenterEntityType};

        let mut added = 0;
        for tag in tags {
            let tag = tag.trim();
            if tag.is_empty() {
                continue;
            }

            match PresenterEntityType::lookup_tagged(storage, tag) {
                Ok(presenter_id) => {
                    let count = PanelToPresenterEntityType::add_presenters(
                        storage,
                        panel_uuid,
                        &[presenter_id],
                    );
                    if count > 0 {
                        if let Some(panel_data) = storage.panels.get_mut(&panel_uuid) {
                            if !panel_data.presenter_ids.contains(&presenter_id) {
                                panel_data.presenter_ids.push(presenter_id);
                            }
                        }
                    }
                    added += count;
                }
                Err(_) => {
                    continue;
                }
            }
        }
        added
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::{NonNilUuid, Uuid};

    fn test_nn() -> NonNilUuid {
        unsafe {
            NonNilUuid::new_unchecked(Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
            ]))
        }
    }

    #[test]
    fn panel_id_from_uuid() {
        let nn = test_nn();
        let id = PanelId::from(nn);
        assert_eq!(NonNilUuid::from(id), nn);
    }

    #[test]
    fn panel_id_try_from_nil_returns_none() {
        assert!(PanelId::try_from_raw_uuid(Uuid::nil()).is_none());
    }

    #[test]
    fn panel_id_display() {
        let id = PanelId::from(test_nn());
        assert_eq!(id.to_string(), "panel-00000000-0000-0000-0000-000000000002");
    }

    #[test]
    fn panel_id_serde_round_trip() {
        let id = PanelId::from(test_nn());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"00000000-0000-0000-0000-000000000002\"");
        let back: PanelId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }
}
