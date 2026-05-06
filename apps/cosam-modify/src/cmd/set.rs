/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! `set` command — update a field on selected entities. (CLI-093)

use anyhow::{anyhow, Result};
use schedule_core::edit::context::EditContext;
use schedule_core::edit::EditCommand;
use schedule_core::entity::{EntityId, EntityType};
use schedule_core::query::{lookup_list, EntityScannable};
use schedule_core::tables::{
    EventRoomEntityType, HotelRoomEntityType, PanelEntityType, PanelTypeEntityType,
    PresenterEntityType,
};
use schedule_core::value::{FieldValue, FieldValueItem};

use crate::args::{EntityTypeName, Stage};

pub fn run(ctx: &mut EditContext, stage: &Stage, field: &str, value: &str) -> Result<()> {
    match stage.entity_type {
        EntityTypeName::Panel => run_for_type::<PanelEntityType>(ctx, stage, field, value),
        EntityTypeName::Presenter => run_for_type::<PresenterEntityType>(ctx, stage, field, value),
        EntityTypeName::EventRoom => run_for_type::<EventRoomEntityType>(ctx, stage, field, value),
        EntityTypeName::HotelRoom => run_for_type::<HotelRoomEntityType>(ctx, stage, field, value),
        EntityTypeName::PanelType => run_for_type::<PanelTypeEntityType>(ctx, stage, field, value),
    }
}

fn run_for_type<E: EntityType + EntityScannable>(
    ctx: &mut EditContext,
    stage: &Stage,
    field_name: &str,
    raw_value: &str,
) -> Result<()> {
    // Resolve runtime field name to the static &'static str the API requires.
    let static_name: &'static str = E::field_set()
        .get_by_name(field_name)
        .ok_or_else(|| anyhow!("unknown field '{field_name}'"))?
        .name();

    let new_value = FieldValue::Single(FieldValueItem::String(raw_value.to_owned()));

    let ids: Vec<EntityId<E>> = match stage.entity_query.as_deref() {
        None | Some("*") | Some("all") => ctx
            .schedule()
            .iter_entities::<E>()
            .map(|(id, _)| id)
            .collect(),
        Some(q) => lookup_list::<E>(ctx.schedule(), q)?,
    };

    if ids.is_empty() {
        return Ok(());
    }

    if ids.len() == 1 {
        let cmd = ctx.update_field_cmd(ids[0], static_name, new_value)?;
        ctx.apply(cmd)?;
    } else {
        let batch: Vec<EditCommand> = ids
            .iter()
            .map(|id| ctx.update_field_cmd(*id, static_name, new_value.clone()))
            .collect::<Result<_, _>>()?;
        ctx.apply(EditCommand::BatchEdit(batch))?;
    }

    Ok(())
}
