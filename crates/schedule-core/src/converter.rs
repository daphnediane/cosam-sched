/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Type-safe conversion from [`FieldValue`] to typed Rust outputs via a work-queue
//! iteration pattern.
//!
//! This module implements the generic conversion system needed by the import
//! pipeline (e.g., tagged presenter `"P:Name"` → `EntityId<PresenterEntityType>` with
//! rank assignment).
//!
//! ## Core traits
//!
//! - [`FieldTypeMapping`] — maps a marker type to a Rust output type
//! - [`FieldValueConverter`] — converts individual items with entity resolution support
//!
//! ## Driver functions
//!
//! Six driver functions expand `FieldValue::List` as a work queue:
//! - Read-only: [`lookup_one`], [`lookup_optional`], [`lookup_many`]
//! - Mutable: [`resolve_one`], [`resolve_optional`], [`resolve_many`]

use crate::entity::{EntityId, EntityType, RuntimeEntityId};
use crate::lookup::EntityMatcher;
use crate::schedule::Schedule;
use crate::value::{ConversionError, FieldValue, FieldValueItem};

/// Wrapper for FieldValue with schedule context for conversions.
///
/// This enum provides a single interface for both read-only and mutable conversions,
/// preventing bugs where the wrong conversion mode is used.
pub enum FieldValueForSchedule<'a> {
    /// Read-only lookup mode (requires &Schedule)
    Lookup(&'a Schedule, FieldValue),
    /// Create-or-resolve mode (requires &mut Schedule)
    LookupOrCreate(&'a mut Schedule, FieldValue),
}

impl<'a> FieldValueForSchedule<'a> {
    /// Type-inferred conversion to a specific marker type.
    ///
    /// The target type is inferred from context. Dispatches to lookup_one or resolve_one
    /// based on the variant (Lookup vs LookupOrCreate).
    pub fn into<M: FieldTypeMapping, C: FieldValueConverter<M>>(
        self,
        converter: &C,
    ) -> Result<M::Output, ConversionError> {
        match self {
            Self::Lookup(schedule, value) => {
                lookup_optional(converter, schedule, value)?.ok_or(ConversionError::ParseError {
                    message: format!(
                        "No value found for conversion to {:?}",
                        M::field_type_item()
                    ),
                })
            }
            Self::LookupOrCreate(schedule, value) => resolve_optional(converter, schedule, value)?
                .ok_or(ConversionError::ParseError {
                    message: format!(
                        "No value found for conversion to {:?}",
                        M::field_type_item()
                    ),
                }),
        }
    }

    /// Convert to a specific marker type using the appropriate mode.
    ///
    /// Alias for `into()` for clarity when explicit converter is provided.
    pub fn convert<M: FieldTypeMapping, C: FieldValueConverter<M>>(
        self,
        converter: &C,
    ) -> Result<M::Output, ConversionError> {
        self.into(converter)
    }

    /// Convert to EntityId using EntityNameLookup converter.
    pub fn into_entity_id<E: EntityType + crate::converter::EntityStringResolver>(
        self,
    ) -> Result<EntityId<E>, ConversionError> {
        // EntityNameLookup is defined in tests, so we need to provide it as a parameter
        // For now, return an error - this will be implemented when EntityNameLookup is moved out of tests
        Err(ConversionError::ParseError {
            message: "into_entity_id requires explicit converter parameter for now".to_string(),
        })
    }
}

/// Maps a marker type to a Rust output type for field value conversion.
///
/// Implement this trait for marker types that specify how to convert a
/// `FieldValueItem` into a typed Rust value.
pub trait FieldTypeMapping: 'static {
    /// The Rust output type after conversion.
    type Output;

    /// The `FieldTypeItem` variant this mapping produces (available in const
    /// contexts so it can be baked into `FieldDescriptor` statics).
    const FIELD_TYPE_ITEM: crate::value::FieldTypeItem;

    /// CRDT annotation for fields backed by this mapping. Defaults to
    /// [`CrdtFieldType::Scalar`]; `AsText` overrides to
    /// [`CrdtFieldType::Text`].
    const CRDT_TYPE: crate::value::CrdtFieldType = crate::value::CrdtFieldType::Scalar;

    /// Returns the `FieldTypeItem` that this mapping expects.
    #[must_use]
    fn field_type_item() -> crate::value::FieldTypeItem {
        Self::FIELD_TYPE_ITEM
    }

    /// Convert a `FieldValueItem` into the output type.
    ///
    /// Returns an error if the item variant does not match the expected type.
    fn from_field_value_item(item: FieldValueItem) -> Result<Self::Output, ConversionError>;

    /// Convert the output type back into a `FieldValueItem`.
    fn to_field_value_item(output: Self::Output) -> FieldValueItem;
}

// ── Standard marker types ─────────────────────────────────────────────────────

/// Marker type for converting to `String`.
pub struct AsString;

impl FieldTypeMapping for AsString {
    type Output = String;
    const FIELD_TYPE_ITEM: crate::value::FieldTypeItem = crate::value::FieldTypeItem::String;

    fn from_field_value_item(item: FieldValueItem) -> Result<Self::Output, ConversionError> {
        match item {
            FieldValueItem::String(s) => Ok(s),
            FieldValueItem::Text(s) => Ok(s),
            FieldValueItem::Integer(n) => Ok(n.to_string()),
            FieldValueItem::Float(v) => Ok(v.to_string()),
            FieldValueItem::Boolean(b) => Ok(b.to_string()),
            FieldValueItem::DateTime(dt) => Ok(format!("{}", dt.format("%a %I:%M %p"))),
            FieldValueItem::Duration(d) => {
                let total_minutes = d.num_minutes();
                if total_minutes >= 60 {
                    let hours = total_minutes / 60;
                    let mins = total_minutes % 60;
                    Ok(format!("{}:{:02}", hours, mins))
                } else {
                    Ok(total_minutes.to_string())
                }
            }
            FieldValueItem::EntityIdentifier(ei) => Ok(ei.to_string()),
        }
    }

    fn to_field_value_item(output: Self::Output) -> FieldValueItem {
        FieldValueItem::String(output)
    }
}

/// Marker type for converting to `String` (long prose).
pub struct AsText;

impl FieldTypeMapping for AsText {
    type Output = String;
    const FIELD_TYPE_ITEM: crate::value::FieldTypeItem = crate::value::FieldTypeItem::Text;
    const CRDT_TYPE: crate::value::CrdtFieldType = crate::value::CrdtFieldType::Text;

    fn from_field_value_item(item: FieldValueItem) -> Result<Self::Output, ConversionError> {
        // Same conversions as AsString, but Text stays as Text variant
        match item {
            FieldValueItem::Text(s) => Ok(s),
            FieldValueItem::String(s) => Ok(s),
            FieldValueItem::Integer(n) => Ok(n.to_string()),
            FieldValueItem::Float(v) => Ok(v.to_string()),
            FieldValueItem::Boolean(b) => Ok(b.to_string()),
            FieldValueItem::DateTime(dt) => Ok(format!("{}", dt.format("%a %I:%M %p"))),
            FieldValueItem::Duration(d) => {
                let total_minutes = d.num_minutes();
                if total_minutes >= 60 {
                    let hours = total_minutes / 60;
                    let mins = total_minutes % 60;
                    Ok(format!("{}:{:02}", hours, mins))
                } else {
                    Ok(total_minutes.to_string())
                }
            }
            FieldValueItem::EntityIdentifier(ei) => Ok(ei.to_string()),
        }
    }

    fn to_field_value_item(output: Self::Output) -> FieldValueItem {
        FieldValueItem::Text(output)
    }
}

/// Marker type for converting to `i64`.
pub struct AsInteger;

impl FieldTypeMapping for AsInteger {
    type Output = i64;
    const FIELD_TYPE_ITEM: crate::value::FieldTypeItem = crate::value::FieldTypeItem::Integer;

    fn from_field_value_item(item: FieldValueItem) -> Result<Self::Output, ConversionError> {
        match item {
            FieldValueItem::Integer(n) => Ok(n),
            FieldValueItem::String(s) => s.parse().map_err(|_| ConversionError::ParseError {
                message: format!("Cannot parse '{}' as integer", s),
            }),
            FieldValueItem::Float(v) => {
                if v.fract() == 0.0 && v >= i64::MIN as f64 && v <= i64::MAX as f64 {
                    Ok(v as i64)
                } else {
                    Err(ConversionError::ParseError {
                        message: format!("Float {} is not a whole number", v),
                    })
                }
            }
            FieldValueItem::Duration(d) => Ok(d.num_minutes()),
            _ => Err(ConversionError::ParseError {
                message: "Cannot convert to Integer from this type".to_string(),
            }),
        }
    }

    fn to_field_value_item(output: Self::Output) -> FieldValueItem {
        FieldValueItem::Integer(output)
    }
}

/// Marker type for converting to `f64`.
pub struct AsFloat;

impl FieldTypeMapping for AsFloat {
    type Output = f64;
    const FIELD_TYPE_ITEM: crate::value::FieldTypeItem = crate::value::FieldTypeItem::Float;

    fn from_field_value_item(item: FieldValueItem) -> Result<Self::Output, ConversionError> {
        match item {
            FieldValueItem::Float(v) => Ok(v),
            FieldValueItem::String(s) => s.parse().map_err(|_| ConversionError::ParseError {
                message: format!("Cannot parse '{}' as float", s),
            }),
            FieldValueItem::Integer(n) => Ok(n as f64),
            FieldValueItem::Duration(d) => Ok(d.num_minutes() as f64),
            _ => Err(ConversionError::ParseError {
                message: "Cannot convert to Float from this type".to_string(),
            }),
        }
    }

    fn to_field_value_item(output: Self::Output) -> FieldValueItem {
        FieldValueItem::Float(output)
    }
}

/// Marker type for converting to `bool`.
pub struct AsBoolean;

impl FieldTypeMapping for AsBoolean {
    type Output = bool;
    const FIELD_TYPE_ITEM: crate::value::FieldTypeItem = crate::value::FieldTypeItem::Boolean;

    fn from_field_value_item(item: FieldValueItem) -> Result<Self::Output, ConversionError> {
        match item {
            FieldValueItem::Boolean(b) => Ok(b),
            FieldValueItem::String(s) => {
                let lower = s.to_lowercase();
                match lower.as_str() {
                    "true" | "yes" | "1" | "on" => Ok(true),
                    "false" | "no" | "0" | "off" => Ok(false),
                    "" => Err(ConversionError::ParseError {
                        message: "Empty string cannot be converted to boolean".to_string(),
                    }),
                    _ => Err(ConversionError::ParseError {
                        message: format!("Cannot parse '{}' as boolean", s),
                    }),
                }
            }
            FieldValueItem::Integer(n) => Ok(n != 0),
            FieldValueItem::Float(v) => Ok(v != 0.0),
            _ => Err(ConversionError::ParseError {
                message: "Cannot convert to Boolean from this type".to_string(),
            }),
        }
    }

    fn to_field_value_item(output: Self::Output) -> FieldValueItem {
        FieldValueItem::Boolean(output)
    }
}

/// Marker type for converting to `chrono::NaiveDateTime`.
pub struct AsDateTime;

impl FieldTypeMapping for AsDateTime {
    type Output = chrono::NaiveDateTime;
    const FIELD_TYPE_ITEM: crate::value::FieldTypeItem = crate::value::FieldTypeItem::DateTime;

    fn from_field_value_item(item: FieldValueItem) -> Result<Self::Output, ConversionError> {
        match item {
            FieldValueItem::DateTime(dt) => Ok(dt),
            FieldValueItem::String(s) => {
                // Try parsing as ISO-8601 first
                chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S")
                    .or_else(|_| chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S"))
                    .map_err(|_| ConversionError::ParseError {
                        message: format!("Cannot parse '{}' as datetime", s),
                    })
            }
            _ => Err(ConversionError::ParseError {
                message: "Cannot convert to DateTime from this type".to_string(),
            }),
        }
    }

    fn to_field_value_item(output: Self::Output) -> FieldValueItem {
        FieldValueItem::DateTime(output)
    }
}

/// Marker type for converting to `chrono::Duration`.
pub struct AsDuration;

impl FieldTypeMapping for AsDuration {
    type Output = chrono::Duration;
    const FIELD_TYPE_ITEM: crate::value::FieldTypeItem = crate::value::FieldTypeItem::Duration;

    fn from_field_value_item(item: FieldValueItem) -> Result<Self::Output, ConversionError> {
        match item {
            FieldValueItem::Duration(d) => Ok(d),
            FieldValueItem::String(s) => {
                // Try parsing as "HH:MM" format
                let parts: Vec<&str> = s.split(':').collect();
                if parts.len() == 2 {
                    let hours: i64 = parts[0].parse().map_err(|_| ConversionError::ParseError {
                        message: format!("Cannot parse '{}' as duration", s),
                    })?;
                    let minutes: i64 =
                        parts[1].parse().map_err(|_| ConversionError::ParseError {
                            message: format!("Cannot parse '{}' as duration", s),
                        })?;
                    chrono::Duration::try_minutes(hours * 60 + minutes).ok_or_else(|| {
                        ConversionError::ParseError {
                            message: "Duration overflow".to_string(),
                        }
                    })
                } else {
                    // Try parsing as plain minutes
                    let minutes: i64 = s.parse().map_err(|_| ConversionError::ParseError {
                        message: format!("Cannot parse '{}' as duration", s),
                    })?;
                    chrono::Duration::try_minutes(minutes).ok_or_else(|| {
                        ConversionError::ParseError {
                            message: "Duration overflow".to_string(),
                        }
                    })
                }
            }
            FieldValueItem::Integer(n) => {
                chrono::Duration::try_minutes(n).ok_or_else(|| ConversionError::ParseError {
                    message: "Duration overflow".to_string(),
                })
            }
            FieldValueItem::Float(v) => {
                chrono::Duration::try_minutes(v as i64).ok_or_else(|| ConversionError::ParseError {
                    message: "Duration overflow".to_string(),
                })
            }
            _ => Err(ConversionError::ParseError {
                message: "Cannot convert to Duration from this type".to_string(),
            }),
        }
    }

    fn to_field_value_item(output: Self::Output) -> FieldValueItem {
        FieldValueItem::Duration(output)
    }
}

/// Trait for customizable entity string resolution.
///
/// Entity types can implement this to provide custom logic for converting
/// string references (e.g., "P:Name" for presenters, panel codes) to EntityIds,
/// and for entity-specific string formatting (e.g., panel code:name).
///
/// Note: The lookup methods have been moved to the lookup module. Use
/// `lookup::lookup_single` and `lookup::lookup_or_create_single` instead.
pub trait EntityStringResolver: EntityMatcher {
    /// Convert an EntityId to a string with entity-specific formatting.
    ///
    /// Examples:
    /// - Panels: `<code>: <name>` (e.g., "GP: Cosplay Foam Armor 101")
    /// - Presenters: name (e.g., "John Smith")
    /// - Event rooms and hotel rooms: room name (e.g., "Ballroom East")
    ///
    /// Default implementation returns the UUID string.
    fn entity_to_string(_schedule: &Schedule, id: EntityId<Self>) -> String {
        id.to_string()
    }

    /// Convert multiple EntityIds to a comma-separated string with entity-specific formatting.
    ///
    /// Default implementation joins entity_to_string results with ", ".
    fn entity_to_string_many(schedule: &Schedule, ids: Vec<EntityId<Self>>) -> String {
        ids.iter()
            .map(|id| Self::entity_to_string(schedule, *id))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Lookup by UUID string (bare UUID or `type_name-<uuid>` prefixed form).
    ///
    /// Returns `Some(id)` only when the UUID is present in the schedule for this
    /// entity type; returns `None` when the string is not a UUID, the UUID is nil,
    /// or the entity is not found.
    fn lookup_by_uuid_string(schedule: &Schedule, s: &str) -> Option<EntityId<Self>> {
        let bare = s
            .strip_prefix(&format!("{}-", Self::TYPE_NAME))
            .unwrap_or(s);
        let uuid = uuid::Uuid::parse_str(bare).ok()?;
        let id = EntityId::<Self>::new(uuid)?;
        schedule.get_internal::<Self>(id).is_some().then_some(id)
    }
}

// Note: EntityStringResolver is implemented per-entity-type in each entity module
// to allow custom string resolution behavior (e.g., room_name for EventRoom)

/// Marker type for converting to `EntityId<E>` with entity type validation.
pub struct AsEntityId<E: EntityType>(std::marker::PhantomData<E>);

impl<E: EntityType> AsEntityId<E> {
    /// Create a new marker instance.
    #[must_use]
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<E: EntityType> Default for AsEntityId<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: EntityType> FieldTypeMapping for AsEntityId<E> {
    type Output = EntityId<E>;
    const FIELD_TYPE_ITEM: crate::value::FieldTypeItem =
        crate::value::FieldTypeItem::EntityIdentifier(E::TYPE_NAME);

    fn from_field_value_item(item: FieldValueItem) -> Result<Self::Output, ConversionError> {
        match item {
            FieldValueItem::EntityIdentifier(rid) => {
                rid.try_as_typed::<E>()
                    .ok_or(ConversionError::ParseError {
                        message: format!(
                            "Entity type mismatch: expected {}, got {}",
                            E::TYPE_NAME,
                            rid.type_name()
                        ),
                    })
            }
            FieldValueItem::String(_) => Err(ConversionError::ParseError {
                message: "String to EntityId conversion requires schedule context - use FieldValueConverter".to_string(),
            }),
            _ => Err(ConversionError::ParseError {
                message: "Cannot convert to EntityId from this type".to_string(),
            }),
        }
    }

    fn to_field_value_item(output: Self::Output) -> FieldValueItem {
        FieldValueItem::EntityIdentifier(RuntimeEntityId::from_typed(output))
    }
}

// ── FieldValueConverter trait ───────────────────────────────────────────────

/// Converts individual `FieldValueItem` values with optional entity resolution.
///
/// This trait is used by the driver functions to process each item in a
/// `FieldValue::List` work queue. The `lookup_next` method performs read-only
/// entity resolution, while `resolve_next` allows mutable operations.
pub trait FieldValueConverter<M: FieldTypeMapping> {
    /// Perform read-only entity resolution on a single item.
    ///
    /// Returns `None` if the item should be skipped (e.g., not found in a lookup).
    /// Returns `Some(Ok(...))` on success, `Some(Err(...))` on conversion failure.
    fn lookup_next(
        &self,
        _schedule: &Schedule,
        input: FieldValueItem,
    ) -> Option<Result<M::Output, ConversionError>> {
        // Default implementation: direct conversion without entity resolution
        Some(M::from_field_value_item(input))
    }

    /// Resolve with mutable schedule access for create-or-resolve operations.
    ///
    /// Default implementation delegates to read-only lookup (no creation).
    /// Implementations that support entity creation should override this.
    fn lookup_or_create_next(
        &self,
        schedule: &mut Schedule,
        input: FieldValueItem,
    ) -> Option<Result<M::Output, ConversionError>> {
        // Default: delegate to read-only lookup (no creation)
        self.lookup_next(schedule, input)
    }

    /// Select a single result from multiple outputs.
    ///
    /// The default implementation returns the first item. Override this for
    /// custom selection logic (e.g., highest rank, most recent, etc.).
    fn select_one(&self, outputs: Vec<M::Output>) -> Result<Option<M::Output>, ConversionError> {
        Ok(outputs.into_iter().next())
    }
}

// ── Scalar write helpers ─────────────────────────────────────────────────────
//
// These are thin wrappers around `FieldTypeMapping::from_field_value_item` used
// by the `stored_field!` macro to implement `WriteFn::Bare` closures without
// duplicating per-type match arms.

/// Convert a scalar `FieldValue` into a required typed value.
///
/// Accepts `FieldValue::Single(item)` or a one-item `FieldValue::List`.
/// Returns an error if the value is empty or contains more than one item.
pub fn convert_required<M: FieldTypeMapping>(v: FieldValue) -> Result<M::Output, ConversionError> {
    let items = v.into_list()?;
    let mut iter = items.into_iter();
    let first = iter.next().ok_or(ConversionError::ParseError {
        message: format!(
            "Required field is empty (expected {:?})",
            M::FIELD_TYPE_ITEM
        ),
    })?;
    M::from_field_value_item(first)
}

/// Convert a scalar `FieldValue` into an optional typed value.
///
/// Empty input or a list whose only item is the empty string/text yields `None`.
/// A single present item is converted via `M::from_field_value_item`.
pub fn convert_optional<M: FieldTypeMapping>(
    v: FieldValue,
) -> Result<Option<M::Output>, ConversionError> {
    if v.is_empty() {
        return Ok(None);
    }
    let items = v.into_list()?;
    match items.into_iter().next() {
        None => Ok(None),
        Some(item) => M::from_field_value_item(item).map(Some),
    }
}

// ── Driver functions ─────────────────────────────────────────────────────────

/// Convert a single `FieldValue` to a typed output using read-only resolution.
///
/// Expands `FieldValue::List` as a work queue, calling `lookup_next` for each item
/// and selecting one result via `select_one`.
///
/// Returns `Ok(None)` if the input is empty or all items return `None` from `lookup_next`.
pub fn lookup_one<M: FieldTypeMapping, C: FieldValueConverter<M>>(
    converter: &C,
    schedule: &Schedule,
    input: FieldValue,
) -> Result<Option<M::Output>, ConversionError> {
    let items = input.into_list()?;
    let mut outputs = Vec::new();

    for item in items {
        if let Some(result) = converter.lookup_next(schedule, item) {
            outputs.push(result?);
        }
    }

    converter.select_one(outputs)
}

/// Convert a single `FieldValue` to a typed output using mutable resolution.
///
/// Iterates over list items and returns the first successful conversion.
///
/// Returns `Ok(None)` if the input is empty or all items return `None` from `lookup_or_create_next`.
pub fn resolve_one<M: FieldTypeMapping, C: FieldValueConverter<M>>(
    converter: &C,
    schedule: &mut Schedule,
    input: FieldValue,
) -> Result<Option<M::Output>, ConversionError> {
    let items = input.into_list()?;

    for item in items {
        if let Some(result) = converter.lookup_or_create_next(schedule, item) {
            return result.map(Some);
        }
    }

    Ok(None)
}

/// Convert a single `FieldValue` to an optional typed output using read-only resolution.
///
/// Similar to `lookup_one`, but explicitly returns `Ok(None)` for empty input
/// rather than treating it as a conversion error.
pub fn lookup_optional<M: FieldTypeMapping, C: FieldValueConverter<M>>(
    converter: &C,
    schedule: &Schedule,
    input: FieldValue,
) -> Result<Option<M::Output>, ConversionError> {
    if input.is_empty() {
        return Ok(None);
    }
    lookup_one(converter, schedule, input)
}

/// Convert a single `FieldValue` to an optional typed output using mutable resolution.
///
/// Similar to `resolve_one`, but explicitly returns `Ok(None)` for empty input
/// rather than treating it as a conversion error.
pub fn resolve_optional<M: FieldTypeMapping, C: FieldValueConverter<M>>(
    converter: &C,
    schedule: &mut Schedule,
    input: FieldValue,
) -> Result<Option<M::Output>, ConversionError> {
    if input.is_empty() {
        return Ok(None);
    }
    resolve_one(converter, schedule, input)
}

/// Convert a single `FieldValue` to a vector of typed outputs using read-only resolution.
///
/// Expands `FieldValue::List` as a work queue, calling `lookup_next` for each item
/// and collecting all successful results.
pub fn lookup_many<M: FieldTypeMapping, C: FieldValueConverter<M>>(
    converter: &C,
    schedule: &Schedule,
    input: FieldValue,
) -> Result<Vec<M::Output>, ConversionError> {
    let items = input.into_list()?;
    let mut outputs = Vec::new();

    for item in items {
        if let Some(result) = converter.lookup_next(schedule, item) {
            outputs.push(result?);
        }
    }

    Ok(outputs)
}

/// Convert a single `FieldValue` to a vector of typed outputs using mutable resolution.
///
/// Iterates over list items and collects all successful conversions.
pub fn resolve_many<M: FieldTypeMapping, C: FieldValueConverter<M>>(
    converter: &C,
    schedule: &mut Schedule,
    input: FieldValue,
) -> Result<Vec<M::Output>, ConversionError> {
    let items = input.into_list()?;
    let mut outputs = Vec::new();

    for item in items {
        if let Some(result) = converter.lookup_or_create_next(schedule, item) {
            outputs.push(result?);
        }
    }

    Ok(outputs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_value;
    use crate::lookup::{lookup_single, EntityScannable};
    use crate::value::FieldTypeItem;

    // Simple converter that uses default implementations
    struct SimpleConverter;

    impl FieldValueConverter<AsString> for SimpleConverter {}
    impl FieldValueConverter<AsText> for SimpleConverter {}
    impl FieldValueConverter<AsInteger> for SimpleConverter {}
    impl FieldValueConverter<AsFloat> for SimpleConverter {}
    impl FieldValueConverter<AsBoolean> for SimpleConverter {}
    impl FieldValueConverter<AsDateTime> for SimpleConverter {}
    impl FieldValueConverter<AsDuration> for SimpleConverter {}

    // Custom converter that parses duration strings (e.g., "1:30" → 90 minutes)
    struct DurationStringParser;

    impl FieldValueConverter<AsDuration> for DurationStringParser {
        fn lookup_next(
            &self,
            _schedule: &Schedule,
            input: FieldValueItem,
        ) -> Option<Result<chrono::Duration, ConversionError>> {
            match input {
                FieldValueItem::String(s) => {
                    // Parse "HH:MM" format to minutes
                    let parts: Vec<&str> = s.split(':').collect();
                    if parts.len() == 2 {
                        let hours: i64 = parts[0].parse().ok()?;
                        let minutes: i64 = parts[1].parse().ok()?;
                        chrono::Duration::try_minutes(hours * 60 + minutes).map(Ok)
                    } else {
                        None
                    }
                }
                FieldValueItem::Duration(d) => Some(Ok(d)),
                _ => Some(AsDuration::from_field_value_item(input)),
            }
        }
    }

    // Custom converter that does entity name lookups using lookup module
    struct EntityNameLookup;

    impl<E: EntityType + EntityScannable> FieldValueConverter<AsEntityId<E>> for EntityNameLookup {
        fn lookup_next(
            &self,
            schedule: &Schedule,
            input: FieldValueItem,
        ) -> Option<Result<EntityId<E>, ConversionError>> {
            match input {
                FieldValueItem::EntityIdentifier(rid) => {
                    // Direct entity ID - validate type
                    rid.try_as_typed::<E>().map(Ok).or_else(|| {
                        Some(Err(ConversionError::ParseError {
                            message: format!(
                                "Entity type mismatch: expected {}, got {}",
                                E::TYPE_NAME,
                                rid.type_name()
                            ),
                        }))
                    })
                }
                FieldValueItem::String(name) => {
                    // Name lookup via lookup module (read-only)
                    Some(lookup_single::<E>(schedule, &name).map_err(|e| {
                        ConversionError::ParseError {
                            message: e.to_string(),
                        }
                    }))
                }
                _ => Some(AsEntityId::<E>::from_field_value_item(input)),
            }
        }
    }

    #[test]
    fn test_as_text_field_type_item() {
        assert_eq!(AsText::field_type_item(), FieldTypeItem::Text);
    }

    #[test]
    fn test_as_integer_field_type_item() {
        assert_eq!(AsInteger::field_type_item(), FieldTypeItem::Integer);
    }

    #[test]
    fn test_as_float_field_type_item() {
        assert_eq!(AsFloat::field_type_item(), FieldTypeItem::Float);
    }

    #[test]
    fn test_as_boolean_field_type_item() {
        assert_eq!(AsBoolean::field_type_item(), FieldTypeItem::Boolean);
    }

    #[test]
    fn test_as_date_time_field_type_item() {
        assert_eq!(AsDateTime::field_type_item(), FieldTypeItem::DateTime);
    }

    #[test]
    fn test_as_duration_field_type_item() {
        assert_eq!(AsDuration::field_type_item(), FieldTypeItem::Duration);
    }

    #[test]
    fn test_as_string_from_field_value_item_ok() {
        let item = FieldValueItem::String("hello".to_owned());
        assert_eq!(AsString::from_field_value_item(item).unwrap(), "hello");
    }

    #[test]
    fn test_as_string_from_field_value_item_converts_integer() {
        let item = FieldValueItem::Integer(42);
        assert_eq!(AsString::from_field_value_item(item).unwrap(), "42");
    }

    #[test]
    #[allow(clippy::approx_constant)] // 3.14 is a sample float, not π
    fn test_as_string_from_field_value_item_converts_float() {
        let item = FieldValueItem::Float(3.14);
        assert_eq!(AsString::from_field_value_item(item).unwrap(), "3.14");
    }

    #[test]
    fn test_as_string_from_field_value_item_converts_boolean() {
        assert_eq!(
            AsString::from_field_value_item(FieldValueItem::Boolean(true)).unwrap(),
            "true"
        );
        assert_eq!(
            AsString::from_field_value_item(FieldValueItem::Boolean(false)).unwrap(),
            "false"
        );
    }

    #[test]
    fn test_as_string_from_field_value_item_converts_duration() {
        let d = chrono::Duration::try_minutes(90).unwrap();
        assert_eq!(
            AsString::from_field_value_item(FieldValueItem::Duration(d)).unwrap(),
            "1:30"
        );

        let d = chrono::Duration::try_minutes(45).unwrap();
        assert_eq!(
            AsString::from_field_value_item(FieldValueItem::Duration(d)).unwrap(),
            "45"
        );
    }

    #[test]
    fn test_as_string_to_field_value_item() {
        assert_eq!(
            AsString::to_field_value_item("test".to_owned()),
            FieldValueItem::String("test".to_owned())
        );
    }

    #[test]
    fn test_as_integer_from_field_value_item_ok() {
        let item = FieldValueItem::Integer(42);
        assert_eq!(AsInteger::from_field_value_item(item).unwrap(), 42);
    }

    #[test]
    fn test_as_integer_from_field_value_item_parses_string() {
        assert_eq!(
            AsInteger::from_field_value_item(FieldValueItem::String("42".to_owned())).unwrap(),
            42
        );
        assert!(AsInteger::from_field_value_item(FieldValueItem::String(
            "not a number".to_owned()
        ))
        .is_err());
    }

    #[test]
    fn test_as_integer_from_field_value_item_converts_whole_float() {
        assert_eq!(
            AsInteger::from_field_value_item(FieldValueItem::Float(42.0)).unwrap(),
            42
        );
        assert!(AsInteger::from_field_value_item(FieldValueItem::Float(42.5)).is_err());
    }

    #[test]
    fn test_as_integer_from_field_value_item_converts_duration() {
        let d = chrono::Duration::try_minutes(90).unwrap();
        assert_eq!(
            AsInteger::from_field_value_item(FieldValueItem::Duration(d)).unwrap(),
            90
        );
    }

    #[test]
    fn test_as_integer_to_field_value_item() {
        assert_eq!(
            AsInteger::to_field_value_item(99),
            FieldValueItem::Integer(99)
        );
    }

    #[test]
    fn test_as_boolean_from_field_value_item_ok() {
        let item = FieldValueItem::Boolean(true);
        assert!(AsBoolean::from_field_value_item(item).unwrap());
    }

    #[test]
    fn test_as_boolean_to_field_value_item() {
        assert_eq!(
            AsBoolean::to_field_value_item(false),
            FieldValueItem::Boolean(false)
        );
    }

    #[test]
    fn test_simple_converter_lookup_next_default() {
        let converter = SimpleConverter;
        let schedule = Schedule::default();
        let item = FieldValueItem::String("test".to_owned());

        let result = FieldValueConverter::<AsString>::lookup_next(&converter, &schedule, item);
        assert!(result.is_some());
        assert_eq!(result.unwrap().unwrap(), "test");
    }

    #[test]
    fn test_simple_converter_lookup_or_create_next_delegates_to_lookup() {
        let converter = SimpleConverter;
        let mut schedule = Schedule::default();
        let item = FieldValueItem::Integer(42);

        let result = FieldValueConverter::<AsInteger>::lookup_or_create_next(
            &converter,
            &mut schedule,
            item,
        );
        assert!(result.is_some());
        assert_eq!(result.unwrap().unwrap(), 42);
    }

    #[test]
    fn test_simple_converter_select_one_returns_first() {
        let converter = SimpleConverter;
        let outputs = vec!["first".to_owned(), "second".to_owned()];

        let result = FieldValueConverter::<AsString>::select_one(&converter, outputs).unwrap();
        assert_eq!(result, Some("first".to_owned()));
    }

    #[test]
    fn test_simple_converter_select_one_empty_returns_none() {
        let converter = SimpleConverter;
        let outputs: Vec<String> = vec![];

        let result = FieldValueConverter::<AsString>::select_one(&converter, outputs).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_lookup_one_single_value() {
        let converter = SimpleConverter;
        let schedule = Schedule::default();
        let input = field_value!("hello");

        let result = lookup_one::<AsString, _>(&converter, &schedule, input).unwrap();
        assert_eq!(result, Some("hello".to_owned()));
    }

    #[test]
    fn test_lookup_one_list_selects_first() {
        let converter = SimpleConverter;
        let schedule = Schedule::default();
        let input = FieldValue::List(vec![
            FieldValueItem::String("first".to_owned()),
            FieldValueItem::String("second".to_owned()),
        ]);

        let result = lookup_one::<AsString, _>(&converter, &schedule, input).unwrap();
        assert_eq!(result, Some("first".to_owned()));
    }

    #[test]
    fn test_lookup_one_empty_list_returns_none() {
        let converter = SimpleConverter;
        let schedule = Schedule::default();
        let input = FieldValue::List(vec![]);

        let result = lookup_one::<AsString, _>(&converter, &schedule, input).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_lookup_optional_empty_returns_none() {
        let converter = SimpleConverter;
        let schedule = Schedule::default();
        let input = FieldValue::List(vec![]);

        let result = lookup_optional::<AsString, _>(&converter, &schedule, input).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_lookup_many_collects_all() {
        let converter = SimpleConverter;
        let schedule = Schedule::default();
        let input = FieldValue::List(vec![
            FieldValueItem::Integer(1),
            FieldValueItem::Integer(2),
            FieldValueItem::Integer(3),
        ]);

        let result = lookup_many::<AsInteger, _>(&converter, &schedule, input).unwrap();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_lookup_many_empty_returns_empty() {
        let converter = SimpleConverter;
        let schedule = Schedule::default();
        let input = FieldValue::List(vec![]);

        let result = lookup_many::<AsInteger, _>(&converter, &schedule, input).unwrap();
        assert_eq!(result, Vec::<i64>::new());
    }

    #[test]
    fn test_resolve_one_single_value() {
        let converter = SimpleConverter;
        let mut schedule = Schedule::default();
        let input = field_value!(42i64);

        let result = resolve_one::<AsInteger, _>(&converter, &mut schedule, input).unwrap();
        assert_eq!(result, Some(42));
    }

    #[test]
    fn test_resolve_many_collects_all() {
        let converter = SimpleConverter;
        let mut schedule = Schedule::default();
        let input = FieldValue::List(vec![
            FieldValueItem::Boolean(true),
            FieldValueItem::Boolean(false),
        ]);

        let result = resolve_many::<AsBoolean, _>(&converter, &mut schedule, input).unwrap();
        assert_eq!(result, vec![true, false]);
    }

    #[test]
    fn test_resolve_optional_empty_returns_none() {
        let converter = SimpleConverter;
        let mut schedule = Schedule::default();
        let input = FieldValue::List(vec![]);

        let result = resolve_optional::<AsInteger, _>(&converter, &mut schedule, input).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_conversion_error_propagates() {
        let converter = SimpleConverter;
        let schedule = Schedule::default();
        let input = field_value!("not an integer");

        let result = lookup_one::<AsInteger, _>(&converter, &schedule, input);
        assert!(result.is_err());
    }

    // Custom converter tests

    #[test]
    fn test_duration_string_parser_parses_hhmm() {
        let converter = DurationStringParser;
        let schedule = Schedule::default();
        let input = field_value!("1:30");

        let result = lookup_one::<AsDuration, _>(&converter, &schedule, input).unwrap();
        assert_eq!(result.unwrap().num_minutes(), 90);
    }

    #[test]
    fn test_duration_string_parser_parses_2h45m() {
        let converter = DurationStringParser;
        let schedule = Schedule::default();
        let input = field_value!("2:45");

        let result = lookup_one::<AsDuration, _>(&converter, &schedule, input).unwrap();
        assert_eq!(result.unwrap().num_minutes(), 165);
    }

    #[test]
    fn test_duration_string_parser_passes_through_duration() {
        let converter = DurationStringParser;
        let schedule = Schedule::default();
        let duration = chrono::Duration::try_minutes(60).unwrap();
        let input = FieldValue::Single(FieldValueItem::Duration(duration));

        let result = lookup_one::<AsDuration, _>(&converter, &schedule, input).unwrap();
        assert_eq!(result.unwrap().num_minutes(), 60);
    }

    #[test]
    fn test_duration_string_parser_invalid_format_returns_none() {
        let converter = DurationStringParser;
        let schedule = Schedule::default();
        let input = field_value!("invalid");

        let result = lookup_one::<AsDuration, _>(&converter, &schedule, input).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_duration_string_parser_single_part_returns_none() {
        let converter = DurationStringParser;
        let schedule = Schedule::default();
        let input = field_value!("90");

        let result = lookup_one::<AsDuration, _>(&converter, &schedule, input).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_entity_name_lookup_validates_entity_type() {
        let converter = EntityNameLookup;
        let schedule = Schedule::default();

        // Create a RuntimeEntityId for a Panel
        use crate::entity::EntityId;
        use crate::panel::PanelEntityType;
        use uuid::Uuid;
        let uuid = Uuid::new_v4();
        let panel_id = EntityId::<PanelEntityType>::new(uuid).unwrap();
        let rid = RuntimeEntityId::from_typed(panel_id);

        let input = FieldValue::Single(FieldValueItem::EntityIdentifier(rid));
        let result =
            lookup_one::<AsEntityId<PanelEntityType>, _>(&converter, &schedule, input).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_entity_name_lookup_wrong_type_returns_error() {
        let converter = EntityNameLookup;
        let schedule = Schedule::default();

        // Create a RuntimeEntityId for a Presenter but try to convert as Panel
        use crate::entity::EntityId;
        use crate::presenter::PresenterEntityType;
        use uuid::Uuid;
        let uuid = Uuid::new_v4();
        let presenter_id = EntityId::<PresenterEntityType>::new(uuid).unwrap();
        let rid = RuntimeEntityId::from_typed(presenter_id);

        let input = FieldValue::Single(FieldValueItem::EntityIdentifier(rid));
        let result = lookup_one::<AsEntityId<crate::panel::PanelEntityType>, _>(
            &converter, &schedule, input,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_entity_name_lookup_string_returns_error_not_found() {
        let converter = EntityNameLookup;
        let schedule = Schedule::default();
        let input = field_value!("SomePresenter");

        // Name lookup returns error for not found (new lookup module returns Result)
        let result = lookup_one::<AsEntityId<crate::presenter::PresenterEntityType>, _>(
            &converter, &schedule, input,
        );
        assert!(result.is_err());
    }
}
