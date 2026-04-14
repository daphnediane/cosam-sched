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

use crate::entity::PresenterId;
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
#[default_resolver]
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

    /// Backing storage for event room relationships (owned forward side).
    /// Panels can have multiple event rooms (though rare).
    pub event_room_ids: Vec<crate::entity::EventRoomId>,

    /// Backing storage for panel type relationships (owned forward side).
    /// Panels can have multiple panel types.
    pub panel_type_ids: Vec<crate::entity::PanelTypeId>,

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
    #[read(|schedule: &crate::schedule::Schedule, entity: &PanelData| {
        use crate::entity::InternalData;
        let panel_id = entity.id();
        let ids = PanelEntityType::presenters_of(&schedule.entities, panel_id);
        Some(crate::field::FieldValue::presenter_list(ids))
    })]
    #[write(|schedule: &mut crate::schedule::Schedule, entity: &mut PanelData, value: crate::field::FieldValue| {
        use crate::entity::InternalData;
        let panel_id = entity.id();
        let presenter_ids = PresenterId::from_field_values(value, schedule)?;
        PanelEntityType::set_presenters(&mut schedule.entities, panel_id, presenter_ids)
    })]
    pub presenters: Vec<crate::entity::PresenterId>,

    /// Add individual presenters to this panel without replacing existing ones.
    /// Write-only computed field that accepts a single UUID/string or list of UUIDs/strings.
    /// String values are resolved via tagged lookup (e.g., "G:Alice", "presenter-`<uuid>`").
    #[computed_field(
        display = "Add Presenters",
        description = "Add presenters to this panel (append mode)"
    )]
    #[write(|schedule: &mut crate::schedule::Schedule, entity: &mut PanelData, value: crate::field::FieldValue| {
        use crate::entity::InternalData;
        let panel_id = entity.id();
        let presenter_ids = PresenterId::from_field_values(value, schedule)?;
        PanelEntityType::add_presenters(&mut schedule.entities, panel_id, presenter_ids);
        Ok(())
    })]
    pub add_presenters: Vec<crate::entity::PresenterId>,

    /// Remove individual presenters from this panel.
    /// Write-only computed field that accepts a single UUID/string or list of UUIDs/strings.
    /// String values are resolved via tagged lookup (e.g., "presenter-`<uuid>`").
    #[computed_field(
        display = "Remove Presenters",
        description = "Remove presenters from this panel"
    )]
    #[write(|schedule: &mut crate::schedule::Schedule, entity: &mut PanelData, value: crate::field::FieldValue| {
        use crate::entity::InternalData;
        let panel_id = entity.id();
        let presenter_ids = PresenterId::from_field_values(value, schedule)?;
        PanelEntityType::remove_presenters(&mut schedule.entities, panel_id, presenter_ids);
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
        use std::collections::{HashSet, VecDeque};

        let direct = entity.presenter_ids.clone();

        let mut result = Vec::new();
        let mut seen = HashSet::new();

        for presenter_id in direct {
            let presenter_uuid = presenter_id.non_nil_uuid();

            // Add the direct presenter
            if seen.insert(presenter_uuid) {
                result.push(crate::field::FieldValue::NonNilUuid(presenter_uuid));
            }

            // Upward: add all groups this presenter belongs to (transitive via group_ids)
            let mut up_queue: VecDeque<uuid::NonNilUuid> = VecDeque::new();
            if let Some(data) = schedule.entities.presenters.get(PresenterId::from_uuid(presenter_uuid)) {
                for gid in &data.group_ids {
                    up_queue.push_back(gid.non_nil_uuid());
                }
            }
            while let Some(group_uuid) = up_queue.pop_front() {
                if seen.insert(group_uuid) {
                    result.push(crate::field::FieldValue::NonNilUuid(group_uuid));
                    if let Some(data) = schedule.entities.presenters.get(PresenterId::from_uuid(group_uuid)) {
                        for gid in &data.group_ids {
                            up_queue.push_back(gid.non_nil_uuid());
                        }
                    }
                }
            }

            // Downward: if this presenter is a group, add its members (transitive)
            let is_group = schedule.entities.presenters.get(PresenterId::from_uuid(presenter_uuid))
                .is_some_and(|d| d.is_explicit_group)
                || !schedule.entities.presenter_group_members
                    .by_left(&PresenterId::from_uuid(presenter_uuid)).is_empty();
            if is_group {
                let mut down_queue: VecDeque<uuid::NonNilUuid> = VecDeque::new();
                for m in schedule.entities.presenter_group_members
                    .by_left(&PresenterId::from_uuid(presenter_uuid)) {
                    down_queue.push_back(m.non_nil_uuid());
                }
                while let Some(m_uuid) = down_queue.pop_front() {
                    if seen.insert(m_uuid) {
                        result.push(crate::field::FieldValue::NonNilUuid(m_uuid));
                        for sm in schedule.entities.presenter_group_members
                            .by_left(&PresenterId::from_uuid(m_uuid)) {
                            down_queue.push_back(sm.non_nil_uuid());
                        }
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
        display = "Event Rooms",
        description = "Rooms where this panel takes place"
    )]
    #[alias("rooms", "event_rooms")]
    #[read(|_schedule: &crate::schedule::Schedule, entity: &PanelData| {
        Some(crate::field::FieldValue::event_room_list(entity.event_room_ids.clone()))
    })]
    #[write(|schedule: &mut crate::schedule::Schedule, entity: &mut PanelData, value: crate::field::FieldValue| {
        use crate::entity::InternalData;
        let panel_id = entity.id();
        let event_room_ids = crate::entity::EventRoomId::from_field_values(value, schedule)?;
        crate::entity::PanelEntityType::set_event_rooms(&mut schedule.entities, panel_id, event_room_ids)
    })]
    pub event_rooms: Vec<crate::entity::EventRoomId>,

    // @todo - panels can have only one type!
    #[computed_field(
        display = "Panel Types",
        description = "Types / categories of this panel (e.g. \"Guest Panel\", \"Workshop\")"
    )]
    #[alias("panel_types", "kinds", "types")]
    #[read(|_schedule: &crate::schedule::Schedule, entity: &PanelData| {
        Some(crate::field::FieldValue::panel_type_list(entity.panel_type_ids.clone()))
    })]
    #[write(|schedule: &mut crate::schedule::Schedule, entity: &mut PanelData, value: crate::field::FieldValue| {
        use crate::entity::InternalData;
        let panel_id = entity.id();
        let panel_type_ids = crate::entity::PanelTypeId::from_field_values(value, schedule)?;
        crate::entity::PanelEntityType::set_panel_types(&mut schedule.entities, panel_id, panel_type_ids)
    })]
    pub panel_types: Vec<crate::entity::PanelTypeId>,
}

impl crate::entity::SchedulableEntity for PanelEntityType {}

// ---------------------------------------------------------------------------
// PanelEntityType relationship management methods
// ---------------------------------------------------------------------------

impl PanelEntityType {
    /// Get all presenters for this panel.
    pub fn presenters_of(
        storage: &crate::schedule::EntityStorage,
        panel_id: PanelId,
    ) -> Vec<crate::entity::PresenterId> {
        storage
            .panels
            .get(panel_id)
            .map(|d| d.presenter_ids.clone())
            .unwrap_or_default()
    }

    /// Set the presenters for this panel.
    ///
    /// Updates both the forward backing field and the reverse index.
    pub fn set_presenters(
        storage: &mut crate::schedule::EntityStorage,
        panel_id: PanelId,
        presenter_ids: Vec<crate::entity::PresenterId>,
    ) -> Result<(), crate::field::FieldError> {
        let entity =
            storage
                .panels
                .get_mut(panel_id)
                .ok_or(crate::field::FieldError::ConversionError(
                    crate::field::validation::ConversionError::InvalidFormat,
                ))?;

        // Update reverse index and forward backing field
        for old_id in &entity.presenter_ids.clone() {
            storage.panels_by_presenter.remove(old_id, &panel_id);
        }
        for new_id in &presenter_ids {
            storage.panels_by_presenter.add(*new_id, panel_id);
        }
        entity.presenter_ids = presenter_ids;

        Ok(())
    }

    /// Get the event rooms for this panel.
    pub fn event_rooms_of(
        storage: &crate::schedule::EntityStorage,
        panel_id: PanelId,
    ) -> Vec<crate::entity::EventRoomId> {
        storage
            .panels
            .get(panel_id)
            .map(|d| d.event_room_ids.clone())
            .unwrap_or_default()
    }

    /// Set the event rooms for this panel.
    ///
    /// Updates both the forward backing field and the reverse index.
    pub fn set_event_rooms(
        storage: &mut crate::schedule::EntityStorage,
        panel_id: PanelId,
        event_room_ids: Vec<crate::entity::EventRoomId>,
    ) -> Result<(), crate::field::FieldError> {
        let entity =
            storage
                .panels
                .get_mut(panel_id)
                .ok_or(crate::field::FieldError::ConversionError(
                    crate::field::validation::ConversionError::InvalidFormat,
                ))?;

        // Update reverse index and forward backing field
        for old_id in &entity.event_room_ids.clone() {
            storage.panels_by_event_room.remove(old_id, &panel_id);
        }
        for new_id in &event_room_ids {
            storage.panels_by_event_room.add(*new_id, panel_id);
        }
        entity.event_room_ids = event_room_ids;

        Ok(())
    }

    /// Get the panel types for this panel.
    pub fn panel_types_of(
        storage: &crate::schedule::EntityStorage,
        panel_id: PanelId,
    ) -> Vec<crate::entity::PanelTypeId> {
        storage
            .panels
            .get(panel_id)
            .map(|d| d.panel_type_ids.clone())
            .unwrap_or_default()
    }

    /// Set the panel types for this panel.
    ///
    /// Updates both the forward backing field and the reverse index.
    pub fn set_panel_types(
        storage: &mut crate::schedule::EntityStorage,
        panel_id: PanelId,
        panel_type_ids: Vec<crate::entity::PanelTypeId>,
    ) -> Result<(), crate::field::FieldError> {
        let entity =
            storage
                .panels
                .get_mut(panel_id)
                .ok_or(crate::field::FieldError::ConversionError(
                    crate::field::validation::ConversionError::InvalidFormat,
                ))?;

        // Update reverse index and forward backing field
        for old_id in &entity.panel_type_ids.clone() {
            storage.panels_by_panel_type.remove(old_id, &panel_id);
        }
        for new_id in &panel_type_ids {
            storage.panels_by_panel_type.add(*new_id, panel_id);
        }
        entity.panel_type_ids = panel_type_ids;

        Ok(())
    }

    /// Get panels that a presenter is assigned to (reverse lookup).
    pub fn panels_of_presenter(
        storage: &crate::schedule::EntityStorage,
        presenter_id: PresenterId,
    ) -> Vec<PanelId> {
        storage.panels_by_presenter.by_left(&presenter_id).to_vec()
    }

    /// Set the panels that a presenter is assigned to.
    ///
    /// Updates both the forward backing field and the reverse index.
    pub fn set_panels_of_presenter(
        storage: &mut crate::schedule::EntityStorage,
        presenter_id: PresenterId,
        panel_ids: Vec<PanelId>,
    ) -> Result<(), crate::field::FieldError> {
        // Get old panels and update reverse index
        let old_panel_ids: Vec<PanelId> =
            storage.panels_by_presenter.by_left(&presenter_id).to_vec();

        // Remove presenter from old panels' presenter_ids
        for old_panel_id in &old_panel_ids {
            if !panel_ids.contains(old_panel_id) {
                if let Some(panel_data) = storage.panels.get_mut(*old_panel_id) {
                    panel_data.presenter_ids.retain(|id| *id != presenter_id);
                }
            }
        }

        // Update reverse index
        storage
            .panels_by_presenter
            .update_by_left(presenter_id, &panel_ids);

        // Add presenter to new panels' presenter_ids
        for new_panel_id in &panel_ids {
            if let Some(panel_data) = storage.panels.get_mut(*new_panel_id) {
                if !panel_data.presenter_ids.contains(&presenter_id) {
                    panel_data.presenter_ids.push(presenter_id);
                }
            }
        }

        Ok(())
    }

    /// Add a panel to a presenter (append mode, avoiding duplicates).
    ///
    /// Returns the number of panels successfully added.
    pub fn add_panel_to_presenter(
        storage: &mut crate::schedule::EntityStorage,
        presenter_id: PresenterId,
        panel_id: PanelId,
    ) -> usize {
        let already = storage
            .panels
            .get(panel_id)
            .is_some_and(|d| d.presenter_ids.contains(&presenter_id));

        if !already {
            if let Some(panel_data) = storage.panels.get_mut(panel_id) {
                panel_data.presenter_ids.push(presenter_id);
            }
            storage.panels_by_presenter.add(presenter_id, panel_id);
            1
        } else {
            0
        }
    }

    /// Remove a panel from a presenter.
    ///
    /// Returns the number of panels successfully removed.
    pub fn remove_panel_from_presenter(
        storage: &mut crate::schedule::EntityStorage,
        presenter_id: PresenterId,
        panel_id: PanelId,
    ) -> usize {
        let had = storage
            .panels
            .get(panel_id)
            .is_some_and(|d| d.presenter_ids.contains(&presenter_id));

        if had {
            if let Some(panel_data) = storage.panels.get_mut(panel_id) {
                panel_data.presenter_ids.retain(|id| *id != presenter_id);
            }
            storage.panels_by_presenter.remove(&presenter_id, &panel_id);
            1
        } else {
            0
        }
    }
}

// ---------------------------------------------------------------------------
// PanelEntityType presenter management methods
// ---------------------------------------------------------------------------

impl PanelEntityType {
    /// Add presenters to a panel (append mode, avoiding duplicates).
    ///
    /// Returns the number of presenters successfully added.
    pub fn add_presenters(
        storage: &mut crate::schedule::EntityStorage,
        panel_id: PanelId,
        presenter_ids: Vec<PresenterId>,
    ) -> usize {
        let mut added = 0;
        for presenter_id in presenter_ids {
            let already = storage
                .panels
                .get(panel_id)
                .is_some_and(|d| d.presenter_ids.contains(&presenter_id));
            if !already {
                if let Some(panel_data) = storage.panels.get_mut(panel_id) {
                    panel_data.presenter_ids.push(presenter_id);
                }
                storage.panels_by_presenter.add(presenter_id, panel_id);
                added += 1;
            }
        }
        added
    }

    /// Remove presenters from a panel.
    ///
    /// Returns the number of presenters successfully removed.
    pub fn remove_presenters(
        storage: &mut crate::schedule::EntityStorage,
        panel_id: PanelId,
        presenter_ids: Vec<PresenterId>,
    ) -> usize {
        let mut removed = 0;
        for presenter_id in presenter_ids {
            let had = storage
                .panels
                .get(panel_id)
                .is_some_and(|d| d.presenter_ids.contains(&presenter_id));
            if had {
                if let Some(panel_data) = storage.panels.get_mut(panel_id) {
                    panel_data.presenter_ids.retain(|id| *id != presenter_id);
                }
                storage.panels_by_presenter.remove(&presenter_id, &panel_id);
                removed += 1;
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
        panel_id: PanelId,
        tags: &[&str],
    ) -> usize {
        use crate::entity::{EntityResolver, PresenterEntityType};

        let values: Vec<crate::field::FieldValue> = tags
            .iter()
            .map(|s| crate::field::FieldValue::String(s.trim().to_string()))
            .collect();
        let field_value = crate::field::FieldValue::List(values);
        let presenter_ids: Vec<PresenterId> =
            match PresenterEntityType::resolve_field_values(storage, field_value) {
                Ok(ids) => ids,
                Err(_) => return 0,
            };
        Self::add_presenters(storage, panel_id, presenter_ids)
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
