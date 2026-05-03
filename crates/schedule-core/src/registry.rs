/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Global lookup caches for entity types and named fields.
//!
//! Provides O(1) lookups via [`LazyLock`]-backed [`HashMap`]s built once from
//! the global `inventory` registries. Complements the iterator-based
//! [`registered_entity_types`] and [`all_named_fields`] functions with direct
//! name-based lookups.
//!
//! # Lookup functions
//!
//! - [`get_entity_type`] — look up a [`RegisteredEntityType`] by type name.
//! - [`get_named_field`] — look up a [`NamedField`] by entity type name + field name/alias.
//! - [`get_full_edge_by_owner`] — look up a [`FullEdge`] by its owner half-edge key.
//!
//! # Macros
//!
//! [`get_named_field!`] and [`get_entity_type!`] wrap the free functions with a
//! per-call-site [`OnceLock`] cache when called with string literals, giving
//! effectively zero-cost repeated lookups on hot paths.
//!
//! [`registered_entity_types`]: crate::entity::registered_entity_types
//! [`all_named_fields`]: crate::field::all_named_fields
//! [`LazyLock`]: std::sync::LazyLock
//! [`HashMap`]: std::collections::HashMap
//! [`OnceLock`]: std::sync::OnceLock

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::edge::{EdgeKind, FullEdge};
use crate::entity::RegisteredEntityType;
use crate::field::{all_named_fields, NamedField};

// ── Entity registry ───────────────────────────────────────────────────────────

static ENTITY_REGISTRY: LazyLock<HashMap<&'static str, &'static RegisteredEntityType>> =
    LazyLock::new(|| {
        crate::entity::registered_entity_types()
            .map(|r| (r.type_name, r))
            .collect()
    });

/// Look up a registered entity type by its canonical type name.
///
/// Returns `None` if no entity type with that name is registered.
#[must_use]
pub fn get_entity_type(name: &str) -> Option<&'static RegisteredEntityType> {
    ENTITY_REGISTRY.get(name).copied()
}

// ── Field index ───────────────────────────────────────────────────────────────

static FIELD_INDEX: LazyLock<HashMap<String, HashMap<String, &'static dyn NamedField>>> =
    LazyLock::new(|| {
        let mut outer: HashMap<String, HashMap<String, &'static dyn NamedField>> = HashMap::new();
        for field in all_named_fields() {
            let inner = outer
                .entry(field.entity_type_name().to_string())
                .or_default();
            inner.insert(field.name().to_string(), field);
            for alias in field.aliases() {
                inner.insert(alias.to_string(), field);
            }
        }
        outer
    });

/// Look up a named field by entity type name and field name or alias.
///
/// Returns `None` if no matching field is found.
#[must_use]
pub fn get_named_field(entity: &str, field: &str) -> Option<&'static dyn NamedField> {
    FIELD_INDEX.get(entity).and_then(|m| m.get(field).copied())
}

// ── FullEdge index ────────────────────────────────────────────────────────────

static FULL_EDGE_INDEX: LazyLock<HashMap<String, FullEdge>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for field in all_named_fields() {
        if let Some(he) = field.try_as_half_edge() {
            if let EdgeKind::Owner { target_field, .. } = he.edge_kind() {
                let owner = he.edge_id();
                let edge = FullEdge {
                    near: owner,
                    far: *target_field,
                };
                map.insert(field.field_key(), edge);
            }
        }
    }
    map
});

/// Look up a [`FullEdge`] by its owner half-edge's `"entity_type:field_name"` key.
///
/// Returns the canonical orientation with `near = owner` and `far = target`.
/// Call [`.flip()`](FullEdge::flip) on the result to get the reversed orientation.
///
/// Returns `None` if no owner edge with that key is registered.
#[must_use]
pub fn get_full_edge_by_owner(owner_key: &str) -> Option<FullEdge> {
    FULL_EDGE_INDEX.get(owner_key).copied()
}

// ── Macros ────────────────────────────────────────────────────────────────────

/// Look up a [`NamedField`] by entity type and field name.
///
/// # Call styles
///
/// ```ignore
/// // Combined literal — per-call-site OnceLock (zero-cost on repeated calls)
/// let field = get_named_field!("panel:panel_type");
///
/// // Two literals — per-call-site OnceLock
/// let field = get_named_field!("panel", "panel_type");
///
/// // Runtime expressions — direct function call each time
/// let field = get_named_field!(entity_str, field_str);
/// ```
///
/// [`NamedField`]: crate::field::NamedField
#[macro_export]
macro_rules! get_named_field {
    ($combined:literal) => {{
        static FIELD: ::std::sync::OnceLock<Option<&'static dyn $crate::field::NamedField>> =
            ::std::sync::OnceLock::new();
        *FIELD.get_or_init(|| match ($combined as &str).split_once(':') {
            Some((entity, field)) => $crate::registry::get_named_field(entity, field),
            None => None,
        })
    }};
    ($entity:literal, $field:literal) => {{
        static FIELD: ::std::sync::OnceLock<Option<&'static dyn $crate::field::NamedField>> =
            ::std::sync::OnceLock::new();
        *FIELD.get_or_init(|| $crate::registry::get_named_field($entity, $field))
    }};
    ($entity:expr, $field:expr) => {
        $crate::registry::get_named_field($entity, $field)
    };
}

/// Look up a registered entity type by name.
///
/// # Call styles
///
/// ```ignore
/// // Literal — per-call-site OnceLock (zero-cost on repeated calls)
/// let et = get_entity_type!("panel");
///
/// // Runtime expression — direct function call each time
/// let et = get_entity_type!(name_str);
/// ```
#[macro_export]
macro_rules! get_entity_type {
    ($name:literal) => {{
        static ET: ::std::sync::OnceLock<Option<&'static $crate::entity::RegisteredEntityType>> =
            ::std::sync::OnceLock::new();
        *ET.get_or_init(|| $crate::registry::get_entity_type($name))
    }};
    ($name:expr) => {
        $crate::registry::get_entity_type($name)
    };
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_entity_type_known() {
        let reg = get_entity_type("panel");
        assert!(reg.is_some());
        assert_eq!(reg.unwrap().type_name, "panel");
    }

    #[test]
    fn test_get_entity_type_unknown() {
        assert!(get_entity_type("no_such_entity").is_none());
    }

    #[test]
    fn test_get_named_field_canonical() {
        let field = get_named_field("panel", "panel_type");
        assert!(field.is_some());
        assert_eq!(field.unwrap().name(), "panel_type");
        assert_eq!(field.unwrap().entity_type_name(), "panel");
    }

    #[test]
    fn test_get_named_field_unknown_entity() {
        assert!(get_named_field("no_such_entity", "panel_type").is_none());
    }

    #[test]
    fn test_get_named_field_unknown_field() {
        assert!(get_named_field("panel", "no_such_field").is_none());
    }

    #[test]
    fn test_get_full_edge_by_owner_known() {
        let edge = get_full_edge_by_owner("panel:panel_type");
        assert!(edge.is_some());
        let edge = edge.unwrap();
        assert_eq!(edge.near.entity_type_name(), "panel");
        assert_eq!(edge.near.name(), "panel_type");
        assert_eq!(edge.far.entity_type_name(), "panel_type");
        assert_eq!(edge.far.name(), "panels");
    }

    #[test]
    fn test_get_full_edge_by_owner_target_is_not_owner() {
        assert!(get_full_edge_by_owner("panel_type:panels").is_none());
    }

    #[test]
    fn test_get_full_edge_by_owner_unknown() {
        assert!(get_full_edge_by_owner("panel:no_such_field").is_none());
    }

    #[test]
    fn test_get_full_edge_flip() {
        let edge = get_full_edge_by_owner("panel:panel_type").unwrap();
        let flipped = edge.flip();
        assert_eq!(flipped.near.entity_type_name(), "panel_type");
        assert_eq!(flipped.near.name(), "panels");
        assert_eq!(flipped.far.entity_type_name(), "panel");
        assert_eq!(flipped.far.name(), "panel_type");
    }

    #[test]
    fn test_all_owner_edges_indexed() {
        let owners = [
            "panel:panel_type",
            "panel:credited_presenters",
            "panel:uncredited_presenters",
            "panel:event_rooms",
            "event_room:hotel_rooms",
            "presenter:members",
        ];
        for key in owners {
            assert!(
                get_full_edge_by_owner(key).is_some(),
                "Expected owner edge not found: {key}"
            );
        }
    }
}
