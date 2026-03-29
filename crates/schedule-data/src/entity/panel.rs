/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Panel entity implementation

use std::fmt;

use crate::entity::EntityType;
use crate::field::{
    DateTimeFieldType, DurationFieldType, FieldDescriptor, FieldError, FieldType, FieldValue,
    IntegerFieldType, StringFieldType, ValidationError,
};

/// Panel ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PanelId(u64);

impl fmt::Display for PanelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "panel-{}", self.0)
    }
}

/// Panel entity
#[derive(Debug, Clone)]
pub struct Panel {
    pub uid: String,
    pub base_uid: Option<String>,
    pub part_num: Option<i64>,
    pub session_num: Option<i64>,
    pub name: String,
    pub panel_type_uid: Option<String>,
    pub description: Option<String>,
    pub note: Option<String>,
    pub prereq: Option<String>,
    pub time_range: crate::time::TimeRange,
    pub room_uids: Vec<String>,
    pub presenter_uids: Vec<String>,
    pub cost: Option<String>,
    pub capacity: Option<String>,
    pub pre_reg_max: Option<String>,
    pub difficulty: Option<String>,
    pub ticket_url: Option<String>,
    pub simple_tix_event: Option<String>,
    pub have_ticket_image: Option<bool>,
    pub is_free: bool,
    pub is_kids: bool,
    pub is_full: bool,
    pub hide_panelist: bool,
    pub sewing_machines: bool,
    pub alt_panelist: Option<String>,
    pub seats_sold: Option<i64>,
    pub notes_non_printing: Option<String>,
    pub workshop_notes: Option<String>,
    pub power_needs: Option<String>,
    pub av_notes: Option<String>,
}

/// Field constants for Panel
pub mod panel_fields {
    use super::Panel;
    use crate::field::*;

    fn validate_time_range(panel: &Panel, _value: &FieldValue) -> Result<(), ValidationError> {
        panel
            .time_range
            .validate()
            .map_err(|reason| ValidationError::ValidationFailed {
                field: "time_range".to_string(),
                reason,
            })
    }

    fn validate_required_uid(panel: &Panel, _value: &FieldValue) -> Result<(), ValidationError> {
        if panel.uid.is_empty() {
            return Err(ValidationError::RequiredFieldMissing {
                field: "uid".to_string(),
            });
        }
        Ok(())
    }

    fn validate_required_name(panel: &Panel, _value: &FieldValue) -> Result<(), ValidationError> {
        if panel.name.is_empty() {
            return Err(ValidationError::RequiredFieldMissing {
                field: "name".to_string(),
            });
        }
        Ok(())
    }

    fn write_uid(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            panel.uid = v;
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn write_name(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            panel.name = v;
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn write_description(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            panel.description = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_description(panel: &Panel, _value: &FieldValue) -> Result<(), ValidationError> {
        if let Some(v) = &panel.description {
            StringFieldType::validate(v)?;
        }
        Ok(())
    }

    fn write_base_uid(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            panel.base_uid = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_base_uid(panel: &Panel, _value: &FieldValue) -> Result<(), ValidationError> {
        if let Some(v) = &panel.base_uid {
            StringFieldType::validate(v)?;
        }
        Ok(())
    }

    fn write_part_num(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::Integer(v) = value {
            panel.part_num = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_part_num(panel: &Panel, _value: &FieldValue) -> Result<(), ValidationError> {
        if let Some(v) = panel.part_num {
            IntegerFieldType::validate(&v)?;
        }
        Ok(())
    }

    fn write_session_num(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::Integer(v) = value {
            panel.session_num = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_session_num(panel: &Panel, _value: &FieldValue) -> Result<(), ValidationError> {
        if let Some(v) = panel.session_num {
            IntegerFieldType::validate(&v)?;
        }
        Ok(())
    }

    fn write_panel_type_uid(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            panel.panel_type_uid = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_panel_type_uid(panel: &Panel, _value: &FieldValue) -> Result<(), ValidationError> {
        if let Some(v) = &panel.panel_type_uid {
            StringFieldType::validate(v)?;
        }
        Ok(())
    }

    fn write_note(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            panel.note = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_note(panel: &Panel, _value: &FieldValue) -> Result<(), ValidationError> {
        if let Some(v) = &panel.note {
            StringFieldType::validate(v)?;
        }
        Ok(())
    }

    fn write_prereq(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            panel.prereq = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_prereq(panel: &Panel, _value: &FieldValue) -> Result<(), ValidationError> {
        if let Some(v) = &panel.prereq {
            StringFieldType::validate(v)?;
        }
        Ok(())
    }

    fn write_cost(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            panel.cost = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_cost(panel: &Panel, _value: &FieldValue) -> Result<(), ValidationError> {
        if let Some(v) = &panel.cost {
            StringFieldType::validate(v)?;
        }
        Ok(())
    }

    fn write_capacity(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            panel.capacity = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_capacity(panel: &Panel, _value: &FieldValue) -> Result<(), ValidationError> {
        if let Some(v) = &panel.capacity {
            StringFieldType::validate(v)?;
        }
        Ok(())
    }

    fn write_pre_reg_max(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            panel.pre_reg_max = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_pre_reg_max(panel: &Panel, _value: &FieldValue) -> Result<(), ValidationError> {
        if let Some(v) = &panel.pre_reg_max {
            StringFieldType::validate(v)?;
        }
        Ok(())
    }

    fn write_difficulty(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            panel.difficulty = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_difficulty(panel: &Panel, _value: &FieldValue) -> Result<(), ValidationError> {
        if let Some(v) = &panel.difficulty {
            StringFieldType::validate(v)?;
        }
        Ok(())
    }

    fn write_ticket_url(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            panel.ticket_url = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_ticket_url(panel: &Panel, _value: &FieldValue) -> Result<(), ValidationError> {
        if let Some(v) = &panel.ticket_url {
            StringFieldType::validate(v)?;
        }
        Ok(())
    }

    fn write_simple_tix_event(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            panel.simple_tix_event = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_simple_tix_event(
        panel: &Panel,
        _value: &FieldValue,
    ) -> Result<(), ValidationError> {
        if let Some(v) = &panel.simple_tix_event {
            StringFieldType::validate(v)?;
        }
        Ok(())
    }

    fn write_have_ticket_image(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::Boolean(v) = value {
            panel.have_ticket_image = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_have_ticket_image(
        _panel: &Panel,
        _value: &FieldValue,
    ) -> Result<(), ValidationError> {
        Ok(())
    }

    fn write_is_free(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::Boolean(v) = value {
            panel.is_free = v;
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn write_is_kids(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::Boolean(v) = value {
            panel.is_kids = v;
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn write_is_full(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::Boolean(v) = value {
            panel.is_full = v;
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn write_hide_panelist(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::Boolean(v) = value {
            panel.hide_panelist = v;
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn write_sewing_machines(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::Boolean(v) = value {
            panel.sewing_machines = v;
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_bool_field(_panel: &Panel, _value: &FieldValue) -> Result<(), ValidationError> {
        Ok(())
    }

    fn write_alt_panelist(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            panel.alt_panelist = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_alt_panelist(panel: &Panel, _value: &FieldValue) -> Result<(), ValidationError> {
        if let Some(v) = &panel.alt_panelist {
            StringFieldType::validate(v)?;
        }
        Ok(())
    }

    fn write_seats_sold(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::Integer(v) = value {
            panel.seats_sold = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_seats_sold(panel: &Panel, _value: &FieldValue) -> Result<(), ValidationError> {
        if let Some(v) = panel.seats_sold {
            IntegerFieldType::validate(&v)?;
        }
        Ok(())
    }

    fn write_notes_non_printing(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            panel.notes_non_printing = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_notes_non_printing(
        panel: &Panel,
        _value: &FieldValue,
    ) -> Result<(), ValidationError> {
        if let Some(v) = &panel.notes_non_printing {
            StringFieldType::validate(v)?;
        }
        Ok(())
    }

    fn write_workshop_notes(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            panel.workshop_notes = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_workshop_notes(panel: &Panel, _value: &FieldValue) -> Result<(), ValidationError> {
        if let Some(v) = &panel.workshop_notes {
            StringFieldType::validate(v)?;
        }
        Ok(())
    }

    fn write_power_needs(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            panel.power_needs = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_power_needs(panel: &Panel, _value: &FieldValue) -> Result<(), ValidationError> {
        if let Some(v) = &panel.power_needs {
            StringFieldType::validate(v)?;
        }
        Ok(())
    }

    fn write_av_notes(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            panel.av_notes = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_av_notes(panel: &Panel, _value: &FieldValue) -> Result<(), ValidationError> {
        if let Some(v) = &panel.av_notes {
            StringFieldType::validate(v)?;
        }
        Ok(())
    }

    fn write_start_time(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::DateTime(v) = value {
            panel.time_range.add_start_time(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn write_end_time(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::DateTime(v) = value {
            panel.time_range.add_end_time(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn write_duration(panel: &mut Panel, value: FieldValue) -> Result<(), FieldError> {
        match value {
            FieldValue::Duration(v) => {
                panel.time_range.add_duration(v);
                Ok(())
            }
            FieldValue::Integer(v) => {
                panel
                    .time_range
                    .add_duration(chrono::Duration::minutes(v.max(0)));
                Ok(())
            }
            _ => Err(FieldError::CannotStoreComputedField),
        }
    }

    fn validate_start_time(panel: &Panel, value: &FieldValue) -> Result<(), ValidationError> {
        if let Some(v) = panel.time_range.start_time() {
            DateTimeFieldType::validate(&v)?;
        }
        validate_time_range(panel, value)
    }

    fn validate_end_time(panel: &Panel, value: &FieldValue) -> Result<(), ValidationError> {
        if let Some(v) = panel.time_range.end_time() {
            DateTimeFieldType::validate(&v)?;
        }
        validate_time_range(panel, value)
    }

    fn validate_duration(panel: &Panel, value: &FieldValue) -> Result<(), ValidationError> {
        if let Some(v) = panel.time_range.duration() {
            DurationFieldType::validate(&v)?;
        }
        validate_time_range(panel, value)
    }

    fn uid_accessor(panel: &Panel) -> Option<FieldValue> {
        Some(FieldValue::String(panel.uid.clone()))
    }

    fn name_accessor(panel: &Panel) -> Option<FieldValue> {
        Some(FieldValue::String(panel.name.clone()))
    }

    fn description_accessor(panel: &Panel) -> Option<FieldValue> {
        panel
            .description
            .as_ref()
            .map(|d| FieldValue::String(d.clone()))
    }

    fn base_uid_accessor(panel: &Panel) -> Option<FieldValue> {
        panel
            .base_uid
            .as_ref()
            .map(|v| FieldValue::String(v.clone()))
    }

    fn part_num_accessor(panel: &Panel) -> Option<FieldValue> {
        panel.part_num.map(FieldValue::Integer)
    }

    fn session_num_accessor(panel: &Panel) -> Option<FieldValue> {
        panel.session_num.map(FieldValue::Integer)
    }

    fn panel_type_uid_accessor(panel: &Panel) -> Option<FieldValue> {
        panel
            .panel_type_uid
            .as_ref()
            .map(|v| FieldValue::String(v.clone()))
    }

    fn note_accessor(panel: &Panel) -> Option<FieldValue> {
        panel.note.as_ref().map(|v| FieldValue::String(v.clone()))
    }

    fn prereq_accessor(panel: &Panel) -> Option<FieldValue> {
        panel.prereq.as_ref().map(|v| FieldValue::String(v.clone()))
    }

    fn cost_accessor(panel: &Panel) -> Option<FieldValue> {
        panel.cost.as_ref().map(|v| FieldValue::String(v.clone()))
    }

    fn capacity_accessor(panel: &Panel) -> Option<FieldValue> {
        panel
            .capacity
            .as_ref()
            .map(|v| FieldValue::String(v.clone()))
    }

    fn pre_reg_max_accessor(panel: &Panel) -> Option<FieldValue> {
        panel
            .pre_reg_max
            .as_ref()
            .map(|v| FieldValue::String(v.clone()))
    }

    fn difficulty_accessor(panel: &Panel) -> Option<FieldValue> {
        panel
            .difficulty
            .as_ref()
            .map(|v| FieldValue::String(v.clone()))
    }

    fn ticket_url_accessor(panel: &Panel) -> Option<FieldValue> {
        panel
            .ticket_url
            .as_ref()
            .map(|v| FieldValue::String(v.clone()))
    }

    fn simple_tix_event_accessor(panel: &Panel) -> Option<FieldValue> {
        panel
            .simple_tix_event
            .as_ref()
            .map(|v| FieldValue::String(v.clone()))
    }

    fn have_ticket_image_accessor(panel: &Panel) -> Option<FieldValue> {
        panel.have_ticket_image.map(FieldValue::Boolean)
    }

    fn is_free_accessor(panel: &Panel) -> Option<FieldValue> {
        Some(FieldValue::Boolean(panel.is_free))
    }

    fn is_kids_accessor(panel: &Panel) -> Option<FieldValue> {
        Some(FieldValue::Boolean(panel.is_kids))
    }

    fn is_full_accessor(panel: &Panel) -> Option<FieldValue> {
        Some(FieldValue::Boolean(panel.is_full))
    }

    fn hide_panelist_accessor(panel: &Panel) -> Option<FieldValue> {
        Some(FieldValue::Boolean(panel.hide_panelist))
    }

    fn sewing_machines_accessor(panel: &Panel) -> Option<FieldValue> {
        Some(FieldValue::Boolean(panel.sewing_machines))
    }

    fn alt_panelist_accessor(panel: &Panel) -> Option<FieldValue> {
        panel
            .alt_panelist
            .as_ref()
            .map(|v| FieldValue::String(v.clone()))
    }

    fn seats_sold_accessor(panel: &Panel) -> Option<FieldValue> {
        panel.seats_sold.map(FieldValue::Integer)
    }

    fn notes_non_printing_accessor(panel: &Panel) -> Option<FieldValue> {
        panel
            .notes_non_printing
            .as_ref()
            .map(|v| FieldValue::String(v.clone()))
    }

    fn workshop_notes_accessor(panel: &Panel) -> Option<FieldValue> {
        panel
            .workshop_notes
            .as_ref()
            .map(|v| FieldValue::String(v.clone()))
    }

    fn power_needs_accessor(panel: &Panel) -> Option<FieldValue> {
        panel
            .power_needs
            .as_ref()
            .map(|v| FieldValue::String(v.clone()))
    }

    fn av_notes_accessor(panel: &Panel) -> Option<FieldValue> {
        panel
            .av_notes
            .as_ref()
            .map(|v| FieldValue::String(v.clone()))
    }

    fn start_time_accessor(panel: &Panel) -> Option<FieldValue> {
        panel.time_range.start_time().map(FieldValue::DateTime)
    }

    fn end_time_accessor(panel: &Panel) -> Option<FieldValue> {
        panel.time_range.end_time().map(FieldValue::DateTime)
    }

    fn duration_accessor(panel: &Panel) -> Option<FieldValue> {
        panel.time_range.duration().map(FieldValue::Duration)
    }

    pub static UID: FieldDescriptor<Panel> = FieldDescriptor {
        name: "uid",
        display_name: "UID",
        description: "Unique identifier for the panel",
        required: true,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(uid_accessor),
        writer: Some(write_uid),
        validator: Some(validate_required_uid),
    };

    pub static NAME: FieldDescriptor<Panel> = FieldDescriptor {
        name: "name",
        display_name: "Name",
        description: "Panel name",
        required: true,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(name_accessor),
        writer: Some(write_name),
        validator: Some(validate_required_name),
    };

    pub static DESCRIPTION: FieldDescriptor<Panel> = FieldDescriptor {
        name: "description",
        display_name: "Description",
        description: "Panel description",
        required: false,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(description_accessor),
        writer: Some(write_description),
        validator: Some(validate_description),
    };

    pub static BASE_UID: FieldDescriptor<Panel> = FieldDescriptor {
        name: "base_uid",
        display_name: "Base UID",
        description: "Base panel-set UID",
        required: false,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(base_uid_accessor),
        writer: Some(write_base_uid),
        validator: Some(validate_base_uid),
    };

    pub static PART_NUM: FieldDescriptor<Panel> = FieldDescriptor {
        name: "part_num",
        display_name: "Part Number",
        description: "Panel part number",
        required: false,
        field_type: FieldTypeEnum::Integer(IntegerFieldType),
        reader: FieldReader::Direct(part_num_accessor),
        writer: Some(write_part_num),
        validator: Some(validate_part_num),
    };

    pub static SESSION_NUM: FieldDescriptor<Panel> = FieldDescriptor {
        name: "session_num",
        display_name: "Session Number",
        description: "Panel session number",
        required: false,
        field_type: FieldTypeEnum::Integer(IntegerFieldType),
        reader: FieldReader::Direct(session_num_accessor),
        writer: Some(write_session_num),
        validator: Some(validate_session_num),
    };

    pub static PANEL_TYPE_UID: FieldDescriptor<Panel> = FieldDescriptor {
        name: "panel_type_uid",
        display_name: "Panel Type",
        description: "Panel type UID",
        required: false,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(panel_type_uid_accessor),
        writer: Some(write_panel_type_uid),
        validator: Some(validate_panel_type_uid),
    };

    pub static NOTE: FieldDescriptor<Panel> = FieldDescriptor {
        name: "note",
        display_name: "Note",
        description: "Panel note",
        required: false,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(note_accessor),
        writer: Some(write_note),
        validator: Some(validate_note),
    };

    pub static PREREQ: FieldDescriptor<Panel> = FieldDescriptor {
        name: "prereq",
        display_name: "Prereq",
        description: "Panel prerequisites",
        required: false,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(prereq_accessor),
        writer: Some(write_prereq),
        validator: Some(validate_prereq),
    };

    pub static COST: FieldDescriptor<Panel> = FieldDescriptor {
        name: "cost",
        display_name: "Cost",
        description: "Panel cost",
        required: false,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(cost_accessor),
        writer: Some(write_cost),
        validator: Some(validate_cost),
    };

    pub static CAPACITY: FieldDescriptor<Panel> = FieldDescriptor {
        name: "capacity",
        display_name: "Capacity",
        description: "Panel capacity",
        required: false,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(capacity_accessor),
        writer: Some(write_capacity),
        validator: Some(validate_capacity),
    };

    pub static PRE_REG_MAX: FieldDescriptor<Panel> = FieldDescriptor {
        name: "pre_reg_max",
        display_name: "Prereg Max",
        description: "Panel preregistration max",
        required: false,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(pre_reg_max_accessor),
        writer: Some(write_pre_reg_max),
        validator: Some(validate_pre_reg_max),
    };

    pub static DIFFICULTY: FieldDescriptor<Panel> = FieldDescriptor {
        name: "difficulty",
        display_name: "Difficulty",
        description: "Panel difficulty",
        required: false,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(difficulty_accessor),
        writer: Some(write_difficulty),
        validator: Some(validate_difficulty),
    };

    pub static TICKET_URL: FieldDescriptor<Panel> = FieldDescriptor {
        name: "ticket_url",
        display_name: "Ticket URL",
        description: "Panel ticket URL",
        required: false,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(ticket_url_accessor),
        writer: Some(write_ticket_url),
        validator: Some(validate_ticket_url),
    };

    pub static SIMPLE_TIX_EVENT: FieldDescriptor<Panel> = FieldDescriptor {
        name: "simple_tix_event",
        display_name: "SimpleTix Event",
        description: "SimpleTix event identifier",
        required: false,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(simple_tix_event_accessor),
        writer: Some(write_simple_tix_event),
        validator: Some(validate_simple_tix_event),
    };

    pub static HAVE_TICKET_IMAGE: FieldDescriptor<Panel> = FieldDescriptor {
        name: "have_ticket_image",
        display_name: "Have Ticket Image",
        description: "Whether panel has a ticket image",
        required: false,
        field_type: FieldTypeEnum::Boolean(BooleanFieldType),
        reader: FieldReader::Direct(have_ticket_image_accessor),
        writer: Some(write_have_ticket_image),
        validator: Some(validate_have_ticket_image),
    };

    pub static IS_FREE: FieldDescriptor<Panel> = FieldDescriptor {
        name: "is_free",
        display_name: "Is Free",
        description: "Whether panel is free",
        required: true,
        field_type: FieldTypeEnum::Boolean(BooleanFieldType),
        reader: FieldReader::Direct(is_free_accessor),
        writer: Some(write_is_free),
        validator: Some(validate_bool_field),
    };

    pub static IS_KIDS: FieldDescriptor<Panel> = FieldDescriptor {
        name: "is_kids",
        display_name: "Is Kids",
        description: "Whether panel is kids focused",
        required: true,
        field_type: FieldTypeEnum::Boolean(BooleanFieldType),
        reader: FieldReader::Direct(is_kids_accessor),
        writer: Some(write_is_kids),
        validator: Some(validate_bool_field),
    };

    pub static IS_FULL: FieldDescriptor<Panel> = FieldDescriptor {
        name: "is_full",
        display_name: "Full",
        description: "Whether panel is full",
        required: true,
        field_type: FieldTypeEnum::Boolean(BooleanFieldType),
        reader: FieldReader::Direct(is_full_accessor),
        writer: Some(write_is_full),
        validator: Some(validate_bool_field),
    };

    pub static HIDE_PANELIST: FieldDescriptor<Panel> = FieldDescriptor {
        name: "hide_panelist",
        display_name: "Hide Panelist",
        description: "Whether panelists should be hidden",
        required: true,
        field_type: FieldTypeEnum::Boolean(BooleanFieldType),
        reader: FieldReader::Direct(hide_panelist_accessor),
        writer: Some(write_hide_panelist),
        validator: Some(validate_bool_field),
    };

    pub static SEWING_MACHINES: FieldDescriptor<Panel> = FieldDescriptor {
        name: "sewing_machines",
        display_name: "Sewing Machines",
        description: "Whether sewing machines are needed",
        required: true,
        field_type: FieldTypeEnum::Boolean(BooleanFieldType),
        reader: FieldReader::Direct(sewing_machines_accessor),
        writer: Some(write_sewing_machines),
        validator: Some(validate_bool_field),
    };

    pub static ALT_PANELIST: FieldDescriptor<Panel> = FieldDescriptor {
        name: "alt_panelist",
        display_name: "Alt Panelist",
        description: "Alternative panelist display text",
        required: false,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(alt_panelist_accessor),
        writer: Some(write_alt_panelist),
        validator: Some(validate_alt_panelist),
    };

    pub static SEATS_SOLD: FieldDescriptor<Panel> = FieldDescriptor {
        name: "seats_sold",
        display_name: "Seats Sold",
        description: "Number of seats sold",
        required: false,
        field_type: FieldTypeEnum::Integer(IntegerFieldType),
        reader: FieldReader::Direct(seats_sold_accessor),
        writer: Some(write_seats_sold),
        validator: Some(validate_seats_sold),
    };

    pub static NOTES_NON_PRINTING: FieldDescriptor<Panel> = FieldDescriptor {
        name: "notes_non_printing",
        display_name: "Notes (Non Printing)",
        description: "Non-printing notes",
        required: false,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(notes_non_printing_accessor),
        writer: Some(write_notes_non_printing),
        validator: Some(validate_notes_non_printing),
    };

    pub static WORKSHOP_NOTES: FieldDescriptor<Panel> = FieldDescriptor {
        name: "workshop_notes",
        display_name: "Workshop Notes",
        description: "Workshop notes",
        required: false,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(workshop_notes_accessor),
        writer: Some(write_workshop_notes),
        validator: Some(validate_workshop_notes),
    };

    pub static POWER_NEEDS: FieldDescriptor<Panel> = FieldDescriptor {
        name: "power_needs",
        display_name: "Power Needs",
        description: "Power requirements",
        required: false,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(power_needs_accessor),
        writer: Some(write_power_needs),
        validator: Some(validate_power_needs),
    };

    pub static AV_NOTES: FieldDescriptor<Panel> = FieldDescriptor {
        name: "av_notes",
        display_name: "AV Notes",
        description: "Audio/visual notes",
        required: false,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(av_notes_accessor),
        writer: Some(write_av_notes),
        validator: Some(validate_av_notes),
    };

    pub static START_TIME: FieldDescriptor<Panel> = FieldDescriptor {
        name: "start_time",
        display_name: "Start Time",
        description: "Computed start time from time range",
        required: false,
        field_type: FieldTypeEnum::DateTime(DateTimeFieldType),
        reader: FieldReader::Computed(start_time_accessor),
        writer: Some(write_start_time),
        validator: Some(validate_start_time),
    };

    pub static END_TIME: FieldDescriptor<Panel> = FieldDescriptor {
        name: "end_time",
        display_name: "End Time",
        description: "Computed end time from time range",
        required: false,
        field_type: FieldTypeEnum::DateTime(DateTimeFieldType),
        reader: FieldReader::Computed(end_time_accessor),
        writer: Some(write_end_time),
        validator: Some(validate_end_time),
    };

    pub static DURATION: FieldDescriptor<Panel> = FieldDescriptor {
        name: "duration",
        display_name: "Duration",
        description: "Computed duration from time range",
        required: false,
        field_type: FieldTypeEnum::Duration(DurationFieldType),
        reader: FieldReader::Computed(duration_accessor),
        writer: Some(write_duration),
        validator: Some(validate_duration),
    };
}

impl Panel {
    pub fn all_fields() -> &'static [FieldDescriptor<Panel>] {
        use std::sync::LazyLock;

        static FIELDS: LazyLock<Vec<FieldDescriptor<Panel>>> = LazyLock::new(|| {
            vec![
                panel_fields::UID,
                panel_fields::NAME,
                panel_fields::DESCRIPTION,
                panel_fields::BASE_UID,
                panel_fields::PART_NUM,
                panel_fields::SESSION_NUM,
                panel_fields::PANEL_TYPE_UID,
                panel_fields::NOTE,
                panel_fields::PREREQ,
                panel_fields::COST,
                panel_fields::CAPACITY,
                panel_fields::PRE_REG_MAX,
                panel_fields::DIFFICULTY,
                panel_fields::TICKET_URL,
                panel_fields::SIMPLE_TIX_EVENT,
                panel_fields::HAVE_TICKET_IMAGE,
                panel_fields::IS_FREE,
                panel_fields::IS_KIDS,
                panel_fields::IS_FULL,
                panel_fields::HIDE_PANELIST,
                panel_fields::SEWING_MACHINES,
                panel_fields::ALT_PANELIST,
                panel_fields::SEATS_SOLD,
                panel_fields::NOTES_NON_PRINTING,
                panel_fields::WORKSHOP_NOTES,
                panel_fields::POWER_NEEDS,
                panel_fields::AV_NOTES,
                panel_fields::START_TIME,
                panel_fields::END_TIME,
                panel_fields::DURATION,
            ]
        });

        FIELDS.as_slice()
    }
}

impl EntityType for Panel {
    type Id = PanelId;
    type Data = Panel;

    const TYPE_NAME: &'static str = "panel";

    fn entity_id(data: &Self::Data) -> Self::Id {
        PanelId(crate::simple_hash(&data.uid))
    }

    fn fields() -> &'static [FieldDescriptor<Self>] {
        Self::all_fields()
    }

    fn validate(data: &Self::Data) -> Result<(), ValidationError> {
        if data.uid.is_empty() {
            return Err(ValidationError::RequiredFieldMissing {
                field: "uid".to_string(),
            });
        }
        if data.name.is_empty() {
            return Err(ValidationError::RequiredFieldMissing {
                field: "name".to_string(),
            });
        }
        if let Err(reason) = data.time_range.validate() {
            return Err(ValidationError::ValidationFailed {
                field: "time_range".to_string(),
                reason,
            });
        }
        Ok(())
    }
}
