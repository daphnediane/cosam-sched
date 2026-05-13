/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Query and update system for entity field operations.
//!
//! This module provides field type mapping, entity matching, and export
//! functionality for the schedule system.

pub mod converter;
pub mod export;
pub mod lookup;

pub use converter::{
    convert_optional, convert_required, lookup_many, lookup_one, lookup_optional, resolve_many,
    resolve_one, resolve_optional, AsAdditionalCost, AsBoolean, AsDateTime, AsDuration, AsEntityId,
    AsFloat, AsInteger, AsString, AsText, EntityStringResolver, FieldTypeMapping,
    FieldValueConverter, FieldValueForSchedule,
};
pub use export::{
    export_to_widget_json, ExportError, WidgetExport, WidgetMeta, WidgetPanel, WidgetPanelType,
    WidgetPresenter, WidgetRoom, WidgetTimeline,
};
pub use lookup::{
    lookup, lookup_list, lookup_or_create, lookup_or_create_list, lookup_or_create_single,
    lookup_single, string_match_priority, CanCreate, EntityCreatable, EntityMatcher,
    EntityScannable, LookupError, MatchConsumed, MatchPriority, ScanFound, ScanResult,
};
