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

    // --- Computed: schedule-aware (edge-based, stubs until FEATURE-007) -----
    #[computed_field(
        display = "Presenters",
        description = "All presenters credited for this panel"
    )]
    #[alias("presenters", "panelists")]
    #[read(|_schedule: &crate::schedule::Schedule, _entity: &PanelData| {
        // @TODO FEATURE-007: populate via PanelToPresenter edges
        None
    })]
    pub presenters: Vec<crate::entity::PresenterId>,

    #[computed_field(
        display = "Event Room",
        description = "Room where this panel takes place"
    )]
    #[alias("room", "event_room")]
    #[read(|_schedule: &crate::schedule::Schedule, _entity: &PanelData| {
        // @TODO FEATURE-007: populate via PanelToEventRoom edge
        None
    })]
    pub event_room: Option<String>,

    #[computed_field(
        display = "Panel Type",
        description = "Type / category of this panel (e.g. \"Guest Panel\", \"Workshop\")"
    )]
    #[alias("panel_type", "kind", "type")]
    #[read(|_schedule: &crate::schedule::Schedule, _entity: &PanelData| {
        // @TODO FEATURE-007: populate via PanelToPanelType edge
        None
    })]
    pub panel_type: Option<String>,
}

impl crate::entity::SchedulableEntity for PanelEntityType {}

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
