/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! `get` command — display a single entity. (CLI-092)

use anyhow::Result;
use schedule_core::edit::context::EditContext;
use schedule_core::entity::EntityType;
use schedule_core::field::NamedField;
use schedule_core::query::{lookup_single, EntityScannable};
use schedule_core::tables::{
    EventRoomEntityType, HotelRoomEntityType, PanelEntityType, PanelTypeEntityType,
    PresenterEntityType,
};

use crate::args::{EntityTypeName, OutputFormat, Stage};

pub fn run(ctx: &mut EditContext, stage: &Stage, query: &str, format: &OutputFormat) -> Result<()> {
    match stage.entity_type {
        EntityTypeName::Panel => run_for_type::<PanelEntityType>(ctx, query, format),
        EntityTypeName::Presenter => run_for_type::<PresenterEntityType>(ctx, query, format),
        EntityTypeName::EventRoom => run_for_type::<EventRoomEntityType>(ctx, query, format),
        EntityTypeName::HotelRoom => run_for_type::<HotelRoomEntityType>(ctx, query, format),
        EntityTypeName::PanelType => run_for_type::<PanelTypeEntityType>(ctx, query, format),
    }
}

fn run_for_type<E: EntityType + EntityScannable>(
    ctx: &mut EditContext,
    query: &str,
    format: &OutputFormat,
) -> Result<()> {
    let schedule = ctx.schedule();
    let id = lookup_single::<E>(schedule, query)?;

    let field_set = E::field_set();
    let field_names: Vec<&'static str> = field_set
        .readable_fields()
        .map(|d| d.name())
        .chain(field_set.half_edges().map(|e| e.name()))
        .collect();

    let id_str = id.to_string();
    let fields: Vec<(&'static str, String)> = field_names
        .iter()
        .filter_map(|&name| {
            field_set
                .read_field_value(name, id, schedule)
                .ok()
                .flatten()
                .map(|v| (name, v.to_string()))
        })
        .collect();

    match format {
        OutputFormat::Text => {
            println!("{id_str}");
            for (name, val) in &fields {
                println!("  {name}: {val}");
            }
        }
        OutputFormat::Json => {
            let mut obj = serde_json::Map::new();
            obj.insert("id".to_string(), serde_json::Value::String(id_str));
            for (name, val) in &fields {
                obj.insert(name.to_string(), serde_json::Value::String(val.clone()));
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::Value::Object(obj))?
            );
        }
        OutputFormat::Toml => {
            let escape = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
            println!("[[entities]]");
            println!("id = \"{}\"", escape(&id_str));
            for (name, val) in &fields {
                println!("{name} = \"{}\"", escape(val));
            }
        }
    }

    Ok(())
}
