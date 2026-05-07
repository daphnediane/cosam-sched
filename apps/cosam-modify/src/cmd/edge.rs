/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! `add-edge` and `remove-edge` commands. (CLI-096)

use anyhow::{anyhow, bail, Result};
use schedule_core::edge::{EdgeKind, FullEdge};
use schedule_core::edit::context::EditContext;
use schedule_core::entity::{EntityType, RuntimeEntityId};
use schedule_core::field::NamedField;
use schedule_core::query::{lookup_single, EntityScannable};
use schedule_core::tables::{
    EventRoomEntityType, HotelRoomEntityType, PanelEntityType, PanelTypeEntityType,
    PresenterEntityType,
};
use schedule_core::value::{FieldTypeItem, FieldValue, FieldValueItem};

use crate::args::{EntityTypeName, Stage};

pub fn run_add(ctx: &mut EditContext, stage: &Stage, edge_field: &str, value: &str) -> Result<()> {
    match stage.entity_type {
        EntityTypeName::Panel => {
            run_edge_op::<PanelEntityType>(ctx, stage, edge_field, value, true)
        }
        EntityTypeName::Presenter => {
            run_edge_op::<PresenterEntityType>(ctx, stage, edge_field, value, true)
        }
        EntityTypeName::EventRoom => {
            run_edge_op::<EventRoomEntityType>(ctx, stage, edge_field, value, true)
        }
        EntityTypeName::HotelRoom => {
            run_edge_op::<HotelRoomEntityType>(ctx, stage, edge_field, value, true)
        }
        EntityTypeName::PanelType => {
            run_edge_op::<PanelTypeEntityType>(ctx, stage, edge_field, value, true)
        }
    }
}

pub fn run_remove(
    ctx: &mut EditContext,
    stage: &Stage,
    edge_field: &str,
    value: &str,
) -> Result<()> {
    match stage.entity_type {
        EntityTypeName::Panel => {
            run_edge_op::<PanelEntityType>(ctx, stage, edge_field, value, false)
        }
        EntityTypeName::Presenter => {
            run_edge_op::<PresenterEntityType>(ctx, stage, edge_field, value, false)
        }
        EntityTypeName::EventRoom => {
            run_edge_op::<EventRoomEntityType>(ctx, stage, edge_field, value, false)
        }
        EntityTypeName::HotelRoom => {
            run_edge_op::<HotelRoomEntityType>(ctx, stage, edge_field, value, false)
        }
        EntityTypeName::PanelType => {
            run_edge_op::<PanelTypeEntityType>(ctx, stage, edge_field, value, false)
        }
    }
}

fn run_edge_op<E: EntityType + EntityScannable>(
    ctx: &mut EditContext,
    stage: &Stage,
    edge_field_name: &str,
    target_query: &str,
    is_add: bool,
) -> Result<()> {
    // Resolve the half-edge by name.
    let he = E::field_set()
        .half_edges()
        .find(|d| d.matches_name(edge_field_name))
        .ok_or_else(|| anyhow!("unknown edge field '{edge_field_name}' on {}", E::TYPE_NAME))?;

    // Only Owner half-edges support direct add/remove.
    let target_he = match he.edge_kind {
        EdgeKind::Owner { target_field, .. } => target_field,
        EdgeKind::Target { .. } => bail!(
            "edge field '{}' is a read-only (target/inverse) edge; \
             add/remove via the owning entity type instead",
            edge_field_name
        ),
    };

    let full_edge = FullEdge {
        near: he,
        far: target_he,
    };

    // Determine target entity type from the field type.
    let target_type_name = match he.field_type().item_type() {
        FieldTypeItem::EntityIdentifier(name) => name,
        other => bail!(
            "edge field '{}' has unexpected item type '{other}'; expected EntityIdentifier",
            edge_field_name
        ),
    };

    // Require a specific near entity query — wildcards not supported for edge ops.
    let near_query = stage
        .entity_query
        .as_deref()
        .ok_or_else(|| anyhow!("add-edge/remove-edge requires a specific entity query"))?;
    let near_id = lookup_single::<E>(ctx.schedule(), near_query)?;

    // Look up the target entity by dispatching on its type name.
    let target_runtime_id = lookup_target(ctx, target_type_name, target_query)?;

    let items = FieldValue::List(vec![FieldValueItem::EntityIdentifier(target_runtime_id)]);

    let cmd = if is_add {
        ctx.add_to_field_cmd(near_id, full_edge, items)
    } else {
        ctx.remove_from_field_cmd(near_id, full_edge, items)
    };
    ctx.apply(cmd)?;
    Ok(())
}

/// Look up a target entity by type name and query, returning a [`RuntimeEntityId`].
fn lookup_target(ctx: &EditContext, type_name: &str, query: &str) -> Result<RuntimeEntityId> {
    match type_name {
        "panel" => {
            let id = lookup_single::<PanelEntityType>(ctx.schedule(), query)?;
            Ok(RuntimeEntityId::from_dynamic(id))
        }
        "presenter" => {
            let id = lookup_single::<PresenterEntityType>(ctx.schedule(), query)?;
            Ok(RuntimeEntityId::from_dynamic(id))
        }
        "event_room" => {
            let id = lookup_single::<EventRoomEntityType>(ctx.schedule(), query)?;
            Ok(RuntimeEntityId::from_dynamic(id))
        }
        "hotel_room" => {
            let id = lookup_single::<HotelRoomEntityType>(ctx.schedule(), query)?;
            Ok(RuntimeEntityId::from_dynamic(id))
        }
        "panel_type" => {
            let id = lookup_single::<PanelTypeEntityType>(ctx.schedule(), query)?;
            Ok(RuntimeEntityId::from_dynamic(id))
        }
        _ => bail!("unsupported target entity type '{type_name}'"),
    }
}
