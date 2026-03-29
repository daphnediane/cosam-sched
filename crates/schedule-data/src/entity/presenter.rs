/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Presenter entity implementation

use crate::entity::EntityType;
use crate::field::{FieldDescriptor, ValidationError};
use std::fmt;

/// Presenter ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PresenterId(u64);

impl fmt::Display for PresenterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "presenter-{}", self.0)
    }
}

/// Presenter entity
#[derive(Debug, Clone)]
pub struct Presenter {
    pub uid: String,
    pub name: String,
    pub rank: Option<String>,
    pub sort_rank_column: Option<i64>,
    pub sort_rank_row: Option<i64>,
    pub sort_rank_member: Option<i64>,
    pub always_shown: bool,
    pub bio: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub is_group: bool,
    pub member_uids: Vec<String>,
    pub group_uids: Vec<String>,
    pub always_grouped: bool,
    pub pronouns: Option<String>,
    pub website: Option<String>,
}

/// Field constants for Presenter
pub mod presenter_fields {
    use super::Presenter;
    use crate::field::*;

    fn uid_accessor(p: &Presenter) -> Option<FieldValue> {
        Some(FieldValue::String(p.uid.clone()))
    }

    fn name_accessor(p: &Presenter) -> Option<FieldValue> {
        Some(FieldValue::String(p.name.clone()))
    }

    fn rank_accessor(p: &Presenter) -> Option<FieldValue> {
        p.rank.as_ref().map(|v| FieldValue::String(v.clone()))
    }

    fn is_group_accessor(p: &Presenter) -> Option<FieldValue> {
        Some(FieldValue::Boolean(p.is_group))
    }

    fn always_grouped_accessor(p: &Presenter) -> Option<FieldValue> {
        Some(FieldValue::Boolean(p.always_grouped))
    }

    fn always_shown_accessor(p: &Presenter) -> Option<FieldValue> {
        Some(FieldValue::Boolean(p.always_shown))
    }

    fn bio_accessor(p: &Presenter) -> Option<FieldValue> {
        p.bio.as_ref().map(|v| FieldValue::String(v.clone()))
    }

    fn email_accessor(p: &Presenter) -> Option<FieldValue> {
        p.email.as_ref().map(|v| FieldValue::String(v.clone()))
    }

    fn phone_accessor(p: &Presenter) -> Option<FieldValue> {
        p.phone.as_ref().map(|v| FieldValue::String(v.clone()))
    }

    fn pronouns_accessor(p: &Presenter) -> Option<FieldValue> {
        p.pronouns.as_ref().map(|v| FieldValue::String(v.clone()))
    }

    fn website_accessor(p: &Presenter) -> Option<FieldValue> {
        p.website.as_ref().map(|v| FieldValue::String(v.clone()))
    }

    fn sort_rank_column_accessor(p: &Presenter) -> Option<FieldValue> {
        p.sort_rank_column.map(FieldValue::Integer)
    }

    fn sort_rank_row_accessor(p: &Presenter) -> Option<FieldValue> {
        p.sort_rank_row.map(FieldValue::Integer)
    }

    fn sort_rank_member_accessor(p: &Presenter) -> Option<FieldValue> {
        p.sort_rank_member.map(FieldValue::Integer)
    }

    fn write_uid(presenter: &mut Presenter, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            presenter.uid = v;
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_uid(presenter: &Presenter, _value: &FieldValue) -> Result<(), ValidationError> {
        if presenter.uid.is_empty() {
            return Err(ValidationError::RequiredFieldMissing {
                field: "uid".to_string(),
            });
        }
        Ok(())
    }

    fn write_name(presenter: &mut Presenter, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            presenter.name = v;
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_name(presenter: &Presenter, _value: &FieldValue) -> Result<(), ValidationError> {
        if presenter.name.is_empty() {
            return Err(ValidationError::RequiredFieldMissing {
                field: "name".to_string(),
            });
        }
        Ok(())
    }

    fn write_rank(presenter: &mut Presenter, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            presenter.rank = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_rank(presenter: &Presenter, _value: &FieldValue) -> Result<(), ValidationError> {
        if let Some(v) = &presenter.rank {
            StringFieldType::validate(v)?;
        }
        Ok(())
    }

    fn write_is_group(presenter: &mut Presenter, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::Boolean(v) = value {
            presenter.is_group = v;
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn write_always_grouped(
        presenter: &mut Presenter,
        value: FieldValue,
    ) -> Result<(), FieldError> {
        if let FieldValue::Boolean(v) = value {
            presenter.always_grouped = v;
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn write_always_shown(presenter: &mut Presenter, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::Boolean(v) = value {
            presenter.always_shown = v;
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_bool_field(
        _presenter: &Presenter,
        value: &FieldValue,
    ) -> Result<(), ValidationError> {
        if let FieldValue::Boolean(v) = value {
            BooleanFieldType::validate(v)?;
        }
        Ok(())
    }

    fn write_bio(presenter: &mut Presenter, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            presenter.bio = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_bio(presenter: &Presenter, _value: &FieldValue) -> Result<(), ValidationError> {
        if let Some(v) = &presenter.bio {
            StringFieldType::validate(v)?;
        }
        Ok(())
    }

    fn write_email(presenter: &mut Presenter, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            presenter.email = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_email(presenter: &Presenter, _value: &FieldValue) -> Result<(), ValidationError> {
        if let Some(v) = &presenter.email {
            StringFieldType::validate(v)?;
        }
        Ok(())
    }

    fn write_phone(presenter: &mut Presenter, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            presenter.phone = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_phone(presenter: &Presenter, _value: &FieldValue) -> Result<(), ValidationError> {
        if let Some(v) = &presenter.phone {
            StringFieldType::validate(v)?;
        }
        Ok(())
    }

    fn write_pronouns(presenter: &mut Presenter, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            presenter.pronouns = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_pronouns(
        presenter: &Presenter,
        _value: &FieldValue,
    ) -> Result<(), ValidationError> {
        if let Some(v) = &presenter.pronouns {
            StringFieldType::validate(v)?;
        }
        Ok(())
    }

    fn write_website(presenter: &mut Presenter, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            presenter.website = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_website(presenter: &Presenter, _value: &FieldValue) -> Result<(), ValidationError> {
        if let Some(v) = &presenter.website {
            StringFieldType::validate(v)?;
        }
        Ok(())
    }

    fn write_sort_rank_column(
        presenter: &mut Presenter,
        value: FieldValue,
    ) -> Result<(), FieldError> {
        if let FieldValue::Integer(v) = value {
            presenter.sort_rank_column = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_sort_rank_column(
        presenter: &Presenter,
        _value: &FieldValue,
    ) -> Result<(), ValidationError> {
        if let Some(v) = presenter.sort_rank_column {
            IntegerFieldType::validate(&v)?;
        }
        Ok(())
    }

    fn write_sort_rank_row(presenter: &mut Presenter, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::Integer(v) = value {
            presenter.sort_rank_row = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_sort_rank_row(
        presenter: &Presenter,
        _value: &FieldValue,
    ) -> Result<(), ValidationError> {
        if let Some(v) = presenter.sort_rank_row {
            IntegerFieldType::validate(&v)?;
        }
        Ok(())
    }

    fn write_sort_rank_member(
        presenter: &mut Presenter,
        value: FieldValue,
    ) -> Result<(), FieldError> {
        if let FieldValue::Integer(v) = value {
            presenter.sort_rank_member = Some(v);
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_sort_rank_member(
        presenter: &Presenter,
        _value: &FieldValue,
    ) -> Result<(), ValidationError> {
        if let Some(v) = presenter.sort_rank_member {
            IntegerFieldType::validate(&v)?;
        }
        Ok(())
    }

    pub static UID: FieldDescriptor<Presenter> = FieldDescriptor {
        name: "uid",
        display_name: "UID",
        description: "Unique identifier for the presenter",
        required: true,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(uid_accessor),
        writer: Some(write_uid),
        validator: Some(validate_uid),
    };

    pub static NAME: FieldDescriptor<Presenter> = FieldDescriptor {
        name: "name",
        display_name: "Name",
        description: "Presenter name",
        required: true,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(name_accessor),
        writer: Some(write_name),
        validator: Some(validate_name),
    };

    pub static RANK: FieldDescriptor<Presenter> = FieldDescriptor {
        name: "rank",
        display_name: "Rank",
        description: "Presenter classification rank",
        required: false,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(rank_accessor),
        writer: Some(write_rank),
        validator: Some(validate_rank),
    };

    pub static IS_GROUP: FieldDescriptor<Presenter> = FieldDescriptor {
        name: "is_group",
        display_name: "Is Group",
        description: "Whether presenter is a group",
        required: true,
        field_type: FieldTypeEnum::Boolean(BooleanFieldType),
        reader: FieldReader::Direct(is_group_accessor),
        writer: Some(write_is_group),
        validator: Some(validate_bool_field),
    };

    pub static ALWAYS_GROUPED: FieldDescriptor<Presenter> = FieldDescriptor {
        name: "always_grouped",
        display_name: "Always Grouped",
        description: "Whether presenter should always be grouped",
        required: true,
        field_type: FieldTypeEnum::Boolean(BooleanFieldType),
        reader: FieldReader::Direct(always_grouped_accessor),
        writer: Some(write_always_grouped),
        validator: Some(validate_bool_field),
    };

    pub static ALWAYS_SHOWN: FieldDescriptor<Presenter> = FieldDescriptor {
        name: "always_shown",
        display_name: "Always Shown",
        description: "Whether presenter should always be shown",
        required: true,
        field_type: FieldTypeEnum::Boolean(BooleanFieldType),
        reader: FieldReader::Direct(always_shown_accessor),
        writer: Some(write_always_shown),
        validator: Some(validate_bool_field),
    };

    pub static BIO: FieldDescriptor<Presenter> = FieldDescriptor {
        name: "bio",
        display_name: "Bio",
        description: "Presenter biography",
        required: false,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(bio_accessor),
        writer: Some(write_bio),
        validator: Some(validate_bio),
    };

    pub static EMAIL: FieldDescriptor<Presenter> = FieldDescriptor {
        name: "email",
        display_name: "Email",
        description: "Presenter email",
        required: false,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(email_accessor),
        writer: Some(write_email),
        validator: Some(validate_email),
    };

    pub static PHONE: FieldDescriptor<Presenter> = FieldDescriptor {
        name: "phone",
        display_name: "Phone",
        description: "Presenter phone",
        required: false,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(phone_accessor),
        writer: Some(write_phone),
        validator: Some(validate_phone),
    };

    pub static PRONOUNS: FieldDescriptor<Presenter> = FieldDescriptor {
        name: "pronouns",
        display_name: "Pronouns",
        description: "Presenter pronouns",
        required: false,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(pronouns_accessor),
        writer: Some(write_pronouns),
        validator: Some(validate_pronouns),
    };

    pub static WEBSITE: FieldDescriptor<Presenter> = FieldDescriptor {
        name: "website",
        display_name: "Website",
        description: "Presenter website",
        required: false,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(website_accessor),
        writer: Some(write_website),
        validator: Some(validate_website),
    };

    pub static SORT_RANK_COLUMN: FieldDescriptor<Presenter> = FieldDescriptor {
        name: "sort_rank_column",
        display_name: "Sort Rank Column",
        description: "Presenter sort rank column index",
        required: false,
        field_type: FieldTypeEnum::Integer(IntegerFieldType),
        reader: FieldReader::Direct(sort_rank_column_accessor),
        writer: Some(write_sort_rank_column),
        validator: Some(validate_sort_rank_column),
    };

    pub static SORT_RANK_ROW: FieldDescriptor<Presenter> = FieldDescriptor {
        name: "sort_rank_row",
        display_name: "Sort Rank Row",
        description: "Presenter sort rank row index",
        required: false,
        field_type: FieldTypeEnum::Integer(IntegerFieldType),
        reader: FieldReader::Direct(sort_rank_row_accessor),
        writer: Some(write_sort_rank_row),
        validator: Some(validate_sort_rank_row),
    };

    pub static SORT_RANK_MEMBER: FieldDescriptor<Presenter> = FieldDescriptor {
        name: "sort_rank_member",
        display_name: "Sort Rank Member",
        description: "Presenter sort rank member index",
        required: false,
        field_type: FieldTypeEnum::Integer(IntegerFieldType),
        reader: FieldReader::Direct(sort_rank_member_accessor),
        writer: Some(write_sort_rank_member),
        validator: Some(validate_sort_rank_member),
    };
}

impl Presenter {
    pub fn all_fields() -> &'static [FieldDescriptor<Presenter>] {
        use std::sync::LazyLock;

        static FIELDS: LazyLock<Vec<FieldDescriptor<Presenter>>> = LazyLock::new(|| {
            vec![
                presenter_fields::UID,
                presenter_fields::NAME,
                presenter_fields::RANK,
                presenter_fields::IS_GROUP,
                presenter_fields::ALWAYS_GROUPED,
                presenter_fields::ALWAYS_SHOWN,
                presenter_fields::BIO,
                presenter_fields::EMAIL,
                presenter_fields::PHONE,
                presenter_fields::PRONOUNS,
                presenter_fields::WEBSITE,
                presenter_fields::SORT_RANK_COLUMN,
                presenter_fields::SORT_RANK_ROW,
                presenter_fields::SORT_RANK_MEMBER,
            ]
        });

        FIELDS.as_slice()
    }
}

impl EntityType for Presenter {
    type Data = Presenter;

    const TYPE_NAME: &'static str = "presenter";

    fn field_set() -> &'static crate::field::field_set::FieldSet<Self> {
        use crate::entity::macros::field_set;
        use std::sync::LazyLock;

        static FIELD_SET: LazyLock<crate::field::field_set::FieldSet<Presenter>> = field_set!(Presenter, {
            fields: [
                &presenter_fields::UID,
                &presenter_fields::NAME,
                &presenter_fields::RANK,
                &presenter_fields::IS_GROUP,
                &presenter_fields::ALWAYS_GROUPED,
                &presenter_fields::ALWAYS_SHOWN,
                &presenter_fields::BIO,
                &presenter_fields::EMAIL,
                &presenter_fields::PHONE,
                &presenter_fields::PRONOUNS,
                &presenter_fields::WEBSITE,
                &presenter_fields::SORT_RANK_COLUMN,
                &presenter_fields::SORT_RANK_ROW,
                &presenter_fields::SORT_RANK_MEMBER
            ],
            required: ["uid", "name"]
        });

        &FIELD_SET
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
        Ok(())
    }
}
