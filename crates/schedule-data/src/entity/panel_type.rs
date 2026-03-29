/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Panel type entity implementation

use crate::entity::EntityType;
use crate::field::{FieldDescriptor, ValidationError};
use std::fmt;

/// Panel type ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PanelTypeId(u64);

impl fmt::Display for PanelTypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "panel-type-{}", self.0)
    }
}

/// Panel type entity
#[derive(Debug, Clone)]
pub struct PanelType {
    pub uid: String,
    pub kind: String,
    pub color: Option<String>,
    pub is_break: bool,
    pub is_cafe: bool,
    pub is_workshop: bool,
    pub is_hidden: bool,
    pub is_room_hours: bool,
    pub is_timeline: bool,
    pub is_private: bool,
    pub bw_color: Option<String>,
}

/// Field constants for PanelType
pub mod panel_type_fields {
    use super::PanelType;
    use crate::field::*;

    fn uid_accessor(pt: &PanelType) -> Option<FieldValue> {
        Some(FieldValue::String(pt.uid.clone()))
    }

    fn kind_accessor(pt: &PanelType) -> Option<FieldValue> {
        Some(FieldValue::String(pt.kind.clone()))
    }

    fn color_accessor(pt: &PanelType) -> Option<FieldValue> {
        pt.color.as_ref().map(|v| FieldValue::String(v.clone()))
    }

    fn bw_color_accessor(pt: &PanelType) -> Option<FieldValue> {
        pt.bw_color.as_ref().map(|v| FieldValue::String(v.clone()))
    }

    fn is_break_accessor(pt: &PanelType) -> Option<FieldValue> {
        Some(FieldValue::Boolean(pt.is_break))
    }

    fn is_cafe_accessor(pt: &PanelType) -> Option<FieldValue> {
        Some(FieldValue::Boolean(pt.is_cafe))
    }

    fn is_workshop_accessor(pt: &PanelType) -> Option<FieldValue> {
        Some(FieldValue::Boolean(pt.is_workshop))
    }

    fn is_hidden_accessor(pt: &PanelType) -> Option<FieldValue> {
        Some(FieldValue::Boolean(pt.is_hidden))
    }

    fn is_room_hours_accessor(pt: &PanelType) -> Option<FieldValue> {
        Some(FieldValue::Boolean(pt.is_room_hours))
    }

    fn is_timeline_accessor(pt: &PanelType) -> Option<FieldValue> {
        Some(FieldValue::Boolean(pt.is_timeline))
    }

    fn is_private_accessor(pt: &PanelType) -> Option<FieldValue> {
        Some(FieldValue::Boolean(pt.is_private))
    }

    fn write_uid(panel_type: &mut PanelType, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            panel_type.uid = v;
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_uid(panel_type: &PanelType, _value: &FieldValue) -> Result<(), ValidationError> {
        if panel_type.uid.is_empty() {
            return Err(ValidationError::RequiredFieldMissing {
                field: "uid".to_string(),
            });
        }
        Ok(())
    }

    fn write_kind(panel_type: &mut PanelType, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            panel_type.kind = v;
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_kind(panel_type: &PanelType, _value: &FieldValue) -> Result<(), ValidationError> {
        if panel_type.kind.is_empty() {
            return Err(ValidationError::RequiredFieldMissing {
                field: "kind".to_string(),
            });
        }
        Ok(())
    }

    fn write_color(panel_type: &mut PanelType, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            panel_type.color = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_color(panel_type: &PanelType, _value: &FieldValue) -> Result<(), ValidationError> {
        if let Some(v) = &panel_type.color {
            StringFieldType::validate(v)?;
        }
        Ok(())
    }

    fn write_bw_color(panel_type: &mut PanelType, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            panel_type.bw_color = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_bw_color(
        panel_type: &PanelType,
        _value: &FieldValue,
    ) -> Result<(), ValidationError> {
        if let Some(v) = &panel_type.bw_color {
            StringFieldType::validate(v)?;
        }
        Ok(())
    }

    fn write_is_break(panel_type: &mut PanelType, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::Boolean(v) = value {
            panel_type.is_break = v;
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn write_is_cafe(panel_type: &mut PanelType, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::Boolean(v) = value {
            panel_type.is_cafe = v;
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn write_is_workshop(panel_type: &mut PanelType, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::Boolean(v) = value {
            panel_type.is_workshop = v;
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn write_is_hidden(panel_type: &mut PanelType, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::Boolean(v) = value {
            panel_type.is_hidden = v;
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn write_is_room_hours(
        panel_type: &mut PanelType,
        value: FieldValue,
    ) -> Result<(), FieldError> {
        if let FieldValue::Boolean(v) = value {
            panel_type.is_room_hours = v;
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn write_is_timeline(panel_type: &mut PanelType, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::Boolean(v) = value {
            panel_type.is_timeline = v;
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn write_is_private(panel_type: &mut PanelType, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::Boolean(v) = value {
            panel_type.is_private = v;
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_bool_field(
        _panel_type: &PanelType,
        value: &FieldValue,
    ) -> Result<(), ValidationError> {
        if let FieldValue::Boolean(v) = value {
            BooleanFieldType::validate(v)?;
        }
        Ok(())
    }

    pub static UID: FieldDescriptor<PanelType> = FieldDescriptor {
        name: "uid",
        display_name: "UID",
        description: "Unique identifier for the panel type",
        required: true,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(uid_accessor),
        writer: Some(write_uid),
        validator: Some(validate_uid),
    };

    pub static KIND: FieldDescriptor<PanelType> = FieldDescriptor {
        name: "kind",
        display_name: "Panel Kind",
        description: "Panel type display kind",
        required: true,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(kind_accessor),
        writer: Some(write_kind),
        validator: Some(validate_kind),
    };

    pub static COLOR: FieldDescriptor<PanelType> = FieldDescriptor {
        name: "color",
        display_name: "Color",
        description: "Primary panel type color",
        required: false,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(color_accessor),
        writer: Some(write_color),
        validator: Some(validate_color),
    };

    pub static BW_COLOR: FieldDescriptor<PanelType> = FieldDescriptor {
        name: "bw_color",
        display_name: "BW",
        description: "Black and white panel type color",
        required: false,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(bw_color_accessor),
        writer: Some(write_bw_color),
        validator: Some(validate_bw_color),
    };

    pub static IS_BREAK: FieldDescriptor<PanelType> = FieldDescriptor {
        name: "is_break",
        display_name: "Is Break",
        description: "Whether this panel type represents breaks",
        required: true,
        field_type: FieldTypeEnum::Boolean(BooleanFieldType),
        reader: FieldReader::Direct(is_break_accessor),
        writer: Some(write_is_break),
        validator: Some(validate_bool_field),
    };

    pub static IS_CAFE: FieldDescriptor<PanelType> = FieldDescriptor {
        name: "is_cafe",
        display_name: "Is Café",
        description: "Whether this panel type represents cafe events",
        required: true,
        field_type: FieldTypeEnum::Boolean(BooleanFieldType),
        reader: FieldReader::Direct(is_cafe_accessor),
        writer: Some(write_is_cafe),
        validator: Some(validate_bool_field),
    };

    pub static IS_WORKSHOP: FieldDescriptor<PanelType> = FieldDescriptor {
        name: "is_workshop",
        display_name: "Is Workshop",
        description: "Whether this panel type represents workshops",
        required: true,
        field_type: FieldTypeEnum::Boolean(BooleanFieldType),
        reader: FieldReader::Direct(is_workshop_accessor),
        writer: Some(write_is_workshop),
        validator: Some(validate_bool_field),
    };

    pub static IS_HIDDEN: FieldDescriptor<PanelType> = FieldDescriptor {
        name: "is_hidden",
        display_name: "Hidden",
        description: "Whether this panel type is hidden",
        required: true,
        field_type: FieldTypeEnum::Boolean(BooleanFieldType),
        reader: FieldReader::Direct(is_hidden_accessor),
        writer: Some(write_is_hidden),
        validator: Some(validate_bool_field),
    };

    pub static IS_ROOM_HOURS: FieldDescriptor<PanelType> = FieldDescriptor {
        name: "is_room_hours",
        display_name: "Is Room Hours",
        description: "Whether this panel type represents room-hours entries",
        required: true,
        field_type: FieldTypeEnum::Boolean(BooleanFieldType),
        reader: FieldReader::Direct(is_room_hours_accessor),
        writer: Some(write_is_room_hours),
        validator: Some(validate_bool_field),
    };

    pub static IS_TIMELINE: FieldDescriptor<PanelType> = FieldDescriptor {
        name: "is_timeline",
        display_name: "Is Timeline",
        description: "Whether this panel type represents timeline entries",
        required: true,
        field_type: FieldTypeEnum::Boolean(BooleanFieldType),
        reader: FieldReader::Direct(is_timeline_accessor),
        writer: Some(write_is_timeline),
        validator: Some(validate_bool_field),
    };

    pub static IS_PRIVATE: FieldDescriptor<PanelType> = FieldDescriptor {
        name: "is_private",
        display_name: "Is Private",
        description: "Whether this panel type is private",
        required: true,
        field_type: FieldTypeEnum::Boolean(BooleanFieldType),
        reader: FieldReader::Direct(is_private_accessor),
        writer: Some(write_is_private),
        validator: Some(validate_bool_field),
    };
}

impl PanelType {
    pub fn all_fields() -> &'static [FieldDescriptor<PanelType>] {
        use std::sync::LazyLock;

        static FIELDS: LazyLock<Vec<FieldDescriptor<PanelType>>> = LazyLock::new(|| {
            vec![
                panel_type_fields::UID,
                panel_type_fields::KIND,
                panel_type_fields::COLOR,
                panel_type_fields::BW_COLOR,
                panel_type_fields::IS_BREAK,
                panel_type_fields::IS_CAFE,
                panel_type_fields::IS_WORKSHOP,
                panel_type_fields::IS_HIDDEN,
                panel_type_fields::IS_ROOM_HOURS,
                panel_type_fields::IS_TIMELINE,
                panel_type_fields::IS_PRIVATE,
            ]
        });

        FIELDS.as_slice()
    }
}

impl EntityType for PanelType {
    type Data = PanelType;

    const TYPE_NAME: &'static str = "panel_type";

    fn field_set() -> &'static crate::field::field_set::FieldSet<Self> {
        use crate::entity::macros::field_set;
        use std::sync::LazyLock;

        static FIELD_SET: LazyLock<crate::field::field_set::FieldSet<PanelType>> = field_set!(PanelType, {
            fields: [
                &panel_type_fields::UID,
                &panel_type_fields::KIND,
                &panel_type_fields::COLOR,
                &panel_type_fields::BW_COLOR,
                &panel_type_fields::IS_BREAK,
                &panel_type_fields::IS_CAFE,
                &panel_type_fields::IS_WORKSHOP,
                &panel_type_fields::IS_HIDDEN,
                &panel_type_fields::IS_ROOM_HOURS,
                &panel_type_fields::IS_TIMELINE,
                &panel_type_fields::IS_PRIVATE
            ],
            required: ["uid", "kind"]
        });

        &FIELD_SET
    }

    fn validate(data: &Self::Data) -> Result<(), ValidationError> {
        if data.uid.is_empty() {
            return Err(ValidationError::RequiredFieldMissing {
                field: "uid".to_string(),
            });
        }
        if data.kind.is_empty() {
            return Err(ValidationError::RequiredFieldMissing {
                field: "kind".to_string(),
            });
        }
        Ok(())
    }
}
