/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Panel entity implementation

use crate::EntityFields;
use std::fmt;

/// Panel ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PanelId(u64);

impl fmt::Display for PanelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "panel-{}", self.0)
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
    #[alias("description", "desc", "details")]
    pub description: Option<String>,

    #[field(display = "Note", description = "General notes about the panel")]
    #[alias("note", "notes", "general_notes")]
    pub note: Option<String>,

    #[field(display = "Prerequisites", description = "Panel prerequisites")]
    #[alias("prereq", "prerequisites", "requirements")]
    pub prereq: Option<String>,

    // TODO: Add computed fields
    pub time_range: crate::time::TimeRange,

    // room_uids removed — now handled via Edge relationships
    #[field(display = "Cost", description = "Panel cost")]
    #[alias("cost", "price", "fee")]
    pub cost: Option<String>,

    #[field(display = "Capacity", description = "Panel capacity")]
    #[alias("capacity", "max_capacity", "seats")]
    pub capacity: Option<String>,

    #[field(display = "Pre-reg Max", description = "Maximum pre-registrations")]
    #[alias("pre_reg_max", "pre_reg", "pre_registration")]
    pub pre_reg_max: Option<String>,

    #[field(display = "Difficulty", description = "Panel difficulty level")]
    #[alias("difficulty", "level", "skill_level")]
    pub difficulty: Option<String>,

    #[field(display = "Ticket URL", description = "URL for ticket purchases")]
    #[alias("ticket_url", "tickets", "registration_url")]
    pub ticket_url: Option<String>,

    #[field(
        display = "SimpleTix Event",
        description = "SimpleTix event identifier"
    )]
    #[alias("simple_tix_event", "simpletix", "event_id")]
    pub simple_tix_event: Option<String>,

    #[field(
        display = "Have Ticket Image",
        description = "Whether ticket image is available"
    )]
    #[alias("have_ticket_image", "ticket_image", "has_ticket_image")]
    pub have_ticket_image: Option<bool>,

    #[field(display = "Is Free", description = "Whether the panel is free")]
    #[alias("is_free", "free", "no_cost")]
    pub is_free: bool,

    #[field(display = "Is Kids", description = "Whether this is a kids program")]
    #[alias("is_kids", "kids", "children")]
    pub is_kids: bool,

    #[field(display = "Is Full", description = "Whether the panel is at capacity")]
    #[alias("is_full", "full", "at_capacity")]
    pub is_full: bool,

    #[field(
        display = "Hide Panelist",
        description = "Whether to hide panelist names"
    )]
    #[alias("hide_panelist", "hide_panelists", "anonymous")]
    pub hide_panelist: bool,

    #[field(
        display = "Sewing Machines",
        description = "Whether sewing machines are needed"
    )]
    #[alias("sewing_machines", "sewing", "equipment")]
    pub sewing_machines: bool,

    #[field(
        display = "Alt Panelist",
        description = "Alternative panelist information"
    )]
    #[alias("alt_panelist", "alternate_panelist", "substitute")]
    pub alt_panelist: Option<String>,

    #[field(display = "Seats Sold", description = "Number of seats sold")]
    #[alias("seats_sold", "sold", "attendance")]
    pub seats_sold: Option<i64>,

    #[field(display = "Notes Non-Printing", description = "Non-printing notes")]
    #[alias("notes_non_printing", "internal_notes", "admin_notes")]
    pub notes_non_printing: Option<String>,

    #[field(display = "Workshop Notes", description = "Workshop-specific notes")]
    #[alias("workshop_notes", "workshop", "hands_on_notes")]
    pub workshop_notes: Option<String>,

    #[field(display = "Power Needs", description = "Power requirements")]
    #[alias("power_needs", "power", "electricity")]
    pub power_needs: Option<String>,

    #[field(display = "AV Notes", description = "Audio/visual requirements")]
    #[alias("av_notes", "av", "audio_visual")]
    pub av_notes: Option<String>,
    // TODO: Implement use proper syntax and edges to support adding presenters etc...
    /*
    #[computed_field(display = "Add Presenter", description = "Add a presenter to this panel")]
    #[alias("add_presenter")]
    #[write]
    fn add_presenter(&self, schedule: &mut Schedule) -> Result<(), FieldError> {
        unimplemented!()
    }
    */
}
