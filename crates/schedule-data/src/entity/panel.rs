/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Panel entity implementation

use crate::EntityFields;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Panel ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PanelId(Uuid);

impl fmt::Display for PanelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "panel-{}", self.0)
    }
}

impl From<Uuid> for PanelId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<PanelId> for Uuid {
    fn from(id: PanelId) -> Uuid {
        id.0
    }
}

/// Panel entity with EntityFields derive macro
#[derive(EntityFields, Debug, Clone)]
pub struct Panel {
    #[field(display = "Uniq UID", description = "Unique identifier for the panel")]
    #[alias("uid", "id")]
    #[required]
    #[indexable(priority = 220)]
    pub uid: String,

    #[field(display = "Base UID", description = "Base UID for multi-part sessions")]
    #[alias("base_uid", "base", "session_base")]
    pub base_uid: Option<String>,

    #[field(
        display = "Part Number",
        description = "Part number for multi-part sessions"
    )]
    #[alias("part_num", "part", "part_number")]
    pub part_num: Option<i64>,

    #[field(display = "Session Number", description = "Session number within part")]
    #[alias("session_num", "session", "session_number")]
    pub session_num: Option<i64>,

    #[field(display = "Name", description = "Panel name/title")]
    #[alias("name", "title", "panel_name")]
    #[required]
    #[indexable(priority = 210, |entity: &Panel, query: &str| {
        let query_lower = query.to_lowercase();
        let name_lower = entity.name.to_lowercase();
        if query.is_empty() { None }
        else if name_lower == query_lower { 
            Some(scaled_exact)
        }
        else if name_lower.starts_with(&query_lower) {
            Some(scaled_strong)
        }
        else if regex::Regex::new(&format!(r"\b{}", regex::escape(query_lower)))
            .unwrap()
            .is_match(&name_lower) {
            Some(scaled_average)
        }
        else if name_lower.contains(&query_lower) {
            Some(scaled_weak)
        }
        else { None }
    })]
    pub name: String,

    #[field(display = "Panel Type UID", description = "UID of the panel type")]
    #[alias("panel_type_uid", "type_uid", "category")]
    pub panel_type_uid: Option<String>,

    #[field(display = "Description", description = "Panel description")]
    #[alias("description", "Description", "desc", "details")]
    pub description: Option<String>,

    #[field(display = "Note", description = "General notes about the panel")]
    #[alias("note", "Note", "notes", "general_notes")]
    pub note: Option<String>,

    #[field(display = "Prerequisites", description = "Panel prerequisites")]
    #[alias("prereq", "Prereq", "prerequisites", "requirements")]
    pub prereq: Option<String>,

    // TODO: Add computed fields
    pub time_range: crate::time::TimeRange,

    // room_uids removed — now handled via Edge relationships
    #[field(display = "Cost", description = "Panel cost")]
    #[alias("cost", "Cost", "price", "fee")]
    pub cost: Option<String>,

    #[field(display = "Capacity", description = "Panel capacity")]
    #[alias("capacity", "Capacity", "max_capacity", "seats")]
    pub capacity: Option<String>,

    #[field(display = "Pre-reg Max", description = "Maximum pre-registrations")]
    #[alias("pre_reg_max", "Prereg_Max", "pre_reg", "pre_registration")]
    pub pre_reg_max: Option<String>,

    #[field(display = "Difficulty", description = "Panel difficulty level")]
    #[alias("difficulty", "Difficulty", "level", "skill_level")]
    pub difficulty: Option<String>,

    #[field(display = "Ticket URL", description = "URL for ticket purchases")]
    #[alias("ticket_url", "Ticket_URL", "tickets", "registration_url")]
    pub ticket_url: Option<String>,

    #[field(
        display = "SimpleTix Event",
        description = "SimpleTix event identifier"
    )]
    #[alias("simple_tix_event", "Simple_Tix_Event", "simpletix", "event_id")]
    pub simple_tix_event: Option<String>,

    #[field(
        display = "Have Ticket Image",
        description = "Whether ticket image is available"
    )]
    #[alias("have_ticket_image", "Have_Ticket_Image", "ticket_image", "has_ticket_image")]
    pub have_ticket_image: Option<bool>,

    #[field(display = "Is Free", description = "Whether the panel is free")]
    #[alias("is_free", "Is_Free", "free", "no_cost")]
    pub is_free: bool,

    #[field(display = "Is Kids", description = "Whether this is a kids program")]
    #[alias("is_kids", "Is_Kids", "kids", "children")]
    pub is_kids: bool,

    #[field(display = "Is Full", description = "Whether the panel is at capacity")]
    #[alias("is_full", "Full", "full", "at_capacity")]
    pub is_full: bool,

    #[field(
        display = "Hide Panelist",
        description = "Whether to hide panelist names"
    )]
    #[alias("hide_panelist", "Hide_Panelist", "hide_panelists", "anonymous")]
    pub hide_panelist: bool,

    #[field(
        display = "Sewing Machines",
        description = "Whether sewing machines are needed"
    )]
    #[alias("sewing_machines", "Sewing_Machines", "sewing", "equipment")]
    pub sewing_machines: bool,

    #[field(
        display = "Alt Panelist",
        description = "Alternative panelist information"
    )]
    #[alias("alt_panelist", "Alt_Panelist", "alternate_panelist", "substitute")]
    pub alt_panelist: Option<String>,

    #[field(display = "Seats Sold", description = "Number of seats sold")]
    #[alias("seats_sold", "Seats_Sold", "sold", "attendance")]
    pub seats_sold: Option<i64>,

    #[field(display = "Notes Non-Printing", description = "Non-printing notes")]
    #[alias("notes_non_printing", "Notes_Non_Printing", "internal_notes", "admin_notes")]
    pub notes_non_printing: Option<String>,

    #[field(display = "Workshop Notes", description = "Workshop-specific notes")]
    #[alias("workshop_notes", "Workshop_Notes", "workshop", "hands_on_notes")]
    pub workshop_notes: Option<String>,

    #[field(display = "Power Needs", description = "Power requirements")]
    #[alias("power_needs", "Power_Needs", "power", "electricity")]
    pub power_needs: Option<String>,

    #[field(display = "AV Notes", description = "Audio/visual requirements")]
    #[alias("av_notes", "AV_Notes", "av", "audio_visual")]
    pub av_notes: Option<String>,

    #[computed_field(
        name = "presenters",
        display = "Presenters",
        description = "All presenters for this panel"
    )]
    #[alias("presenter_list", "panelists")]
    #[read(|schedule: &crate::schedule::Schedule, entity: &PanelData| {
        let presenter_ids = schedule.get_panel_presenters(PanelId(entity.entity_uuid));
        Some(crate::field::FieldValue::List(
            schedule.get_entity_names::<crate::entity::PresenterEntityType>(&presenter_ids)
                .into_iter()
                .map(crate::field::FieldValue::String)
                .collect()
        ))
    })]
    pub presenters: Vec<crate::entity::PresenterId>,

    #[computed_field(
        name = "event_room",
        display = "Event Room",
        description = "Primary event room for this panel"
    )]
    #[alias("room", "location", "event_room_name")]
    #[read(|schedule: &crate::schedule::Schedule, entity: &PanelData| {
        if let Some(room_id) = schedule.get_panel_event_room(PanelId(entity.entity_uuid)) {
            if let Some(room) = schedule.get_entity::<crate::entity::EventRoomEntityType>(room_id) {
                return Some(crate::field::FieldValue::String(room.long_name.clone()));
            }
        }
        None
    })]
    pub event_room: Option<String>,

    #[computed_field(
        name = "panel_type",
        display = "Panel Type",
        description = "Type/category of this panel"
    )]
    #[alias("type", "category", "panel_category")]
    #[read(|schedule: &crate::schedule::Schedule, entity: &PanelData| {
        if let Some(type_id) = schedule.get_panel_type(PanelId(entity.entity_uuid)) {
            if let Some(panel_type) = schedule.get_entity::<crate::entity::PanelTypeEntityType>(type_id) {
                return Some(crate::field::FieldValue::String(panel_type.prefix.clone()));
            }
        }
        None
    })]
    pub panel_type: Option<String>,
}

impl crate::entity::SchedulableEntity for PanelEntityType {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_id_from_uuid() {
        let uuid = Uuid::nil();
        let id = PanelId::from(uuid);
        assert_eq!(Uuid::from(id), uuid);
    }

    #[test]
    fn panel_id_display() {
        let id = PanelId::from(Uuid::nil());
        assert_eq!(id.to_string(), "panel-00000000-0000-0000-0000-000000000000");
    }

    #[test]
    fn panel_id_serde_round_trip() {
        let id = PanelId::from(Uuid::nil());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"00000000-0000-0000-0000-000000000000\"");
        let back: PanelId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }
}
