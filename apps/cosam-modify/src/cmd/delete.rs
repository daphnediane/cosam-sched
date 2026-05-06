/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! `delete` command — remove an entity. (CLI-095)

use anyhow::{bail, Result};
use schedule_core::edit::context::EditContext;
use schedule_core::entity::{EntityType, RuntimeEntityId};
use schedule_core::query::{lookup_single, EntityScannable};
use schedule_core::tables::{
    EventRoomEntityType, HotelRoomEntityType, PanelEntityType, PanelTypeEntityType,
    PresenterEntityType,
};

use crate::args::{EntityTypeName, Stage};

pub fn run(ctx: &mut EditContext, stage: &Stage, query: &str) -> Result<()> {
    // Disallow bulk wildcard delete without --force (not yet implemented).
    if query == "*" || query.to_lowercase() == "all" {
        bail!("'delete *' is not allowed without --force; specify an entity by name or UUID");
    }

    match stage.entity_type {
        EntityTypeName::Panel => run_for_type::<PanelEntityType>(ctx, query),
        EntityTypeName::Presenter => run_for_type::<PresenterEntityType>(ctx, query),
        EntityTypeName::EventRoom => run_for_type::<EventRoomEntityType>(ctx, query),
        EntityTypeName::HotelRoom => run_for_type::<HotelRoomEntityType>(ctx, query),
        EntityTypeName::PanelType => run_for_type::<PanelTypeEntityType>(ctx, query),
    }
}

fn run_for_type<E: EntityType + EntityScannable>(ctx: &mut EditContext, query: &str) -> Result<()> {
    let id = lookup_single::<E>(ctx.schedule(), query)?;
    let cmd = ctx.remove_entity_cmd(RuntimeEntityId::from_dynamic(id))?;
    ctx.apply(cmd)?;
    Ok(())
}
