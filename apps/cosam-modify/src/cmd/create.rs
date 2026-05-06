/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! `create` command — add a new entity. (CLI-094)

use anyhow::{anyhow, Result};
use schedule_core::edit::context::EditContext;
use schedule_core::edit::EditCommand;
use schedule_core::entity::{EntityId, EntityType, RuntimeEntityId};
use schedule_core::query::EntityScannable;
use schedule_core::tables::{
    EventRoomEntityType, HotelRoomEntityType, PanelEntityType, PanelTypeEntityType,
    PresenterEntityType,
};
use schedule_core::value::{FieldValue, FieldValueItem};

use crate::args::{EntityTypeName, Stage};

pub fn run(ctx: &mut EditContext, stage: &Stage, fields: &[(String, String)]) -> Result<()> {
    match stage.entity_type {
        EntityTypeName::Panel => run_for_type::<PanelEntityType>(ctx, fields),
        EntityTypeName::Presenter => run_for_type::<PresenterEntityType>(ctx, fields),
        EntityTypeName::EventRoom => run_for_type::<EventRoomEntityType>(ctx, fields),
        EntityTypeName::HotelRoom => run_for_type::<HotelRoomEntityType>(ctx, fields),
        EntityTypeName::PanelType => run_for_type::<PanelTypeEntityType>(ctx, fields),
    }
}

fn run_for_type<E: EntityType + EntityScannable>(
    ctx: &mut EditContext,
    cli_fields: &[(String, String)],
) -> Result<()> {
    let field_set = E::field_set();
    let fields: Vec<(&'static str, FieldValue)> = cli_fields
        .iter()
        .map(|(name, val)| {
            let resolved = field_set
                .get_by_name(name)
                .ok_or_else(|| anyhow!("unknown field '{name}'"))?;
            Ok((
                resolved.name(),
                FieldValue::Single(FieldValueItem::String(val.clone())),
            ))
        })
        .collect::<Result<_>>()?;

    let id: EntityId<E> = EntityId::generate();
    let runtime_id = RuntimeEntityId::from_dynamic(id);
    let cmd = EditCommand::AddEntity {
        entity: runtime_id,
        fields,
    };
    ctx.apply(cmd)?;
    Ok(())
}
