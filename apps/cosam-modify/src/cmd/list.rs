/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! `list` command — display entities and their fields. (CLI-092)

use anyhow::Result;
use schedule_core::edit::context::EditContext;
use schedule_core::entity::{EntityId, EntityType};
use schedule_core::field::NamedField;
use schedule_core::query::{lookup_list, EntityScannable};
use schedule_core::tables::{
    EventRoomEntityType, HotelRoomEntityType, PanelEntityType, PanelTypeEntityType,
    PresenterEntityType,
};

use crate::args::{EntityTypeName, OutputFormat, Stage};

pub fn run(ctx: &mut EditContext, stage: &Stage, format: &OutputFormat) -> Result<()> {
    match stage.entity_type {
        EntityTypeName::Panel => run_for_type::<PanelEntityType>(ctx, stage, format),
        EntityTypeName::Presenter => run_for_type::<PresenterEntityType>(ctx, stage, format),
        EntityTypeName::EventRoom => run_for_type::<EventRoomEntityType>(ctx, stage, format),
        EntityTypeName::HotelRoom => run_for_type::<HotelRoomEntityType>(ctx, stage, format),
        EntityTypeName::PanelType => run_for_type::<PanelTypeEntityType>(ctx, stage, format),
    }
}

fn run_for_type<E: EntityType + EntityScannable>(
    ctx: &mut EditContext,
    stage: &Stage,
    format: &OutputFormat,
) -> Result<()> {
    let schedule = ctx.schedule();
    let ids: Vec<EntityId<E>> = match stage.entity_query.as_deref() {
        None | Some("*") | Some("all") => schedule.iter_entities::<E>().map(|(id, _)| id).collect(),
        Some(q) => lookup_list::<E>(schedule, q)?,
    };

    let field_set = E::field_set();
    let field_names: Vec<&'static str> = field_set
        .readable_fields()
        .map(|d| d.name())
        .chain(field_set.half_edges().map(|e| e.name()))
        .collect();

    // Collect entity records: (id_str, [(name, value_str)])
    let records: Vec<(String, Vec<(&'static str, String)>)> = ids
        .iter()
        .map(|id| {
            let fields: Vec<(&'static str, String)> = field_names
                .iter()
                .filter_map(|&name| {
                    field_set
                        .read_field_value(name, *id, schedule)
                        .ok()
                        .flatten()
                        .map(|v| (name, v.to_string()))
                })
                .collect();
            (id.to_string(), fields)
        })
        .collect();

    match format {
        OutputFormat::Text => {
            for (id_str, fields) in &records {
                println!("{id_str}");
                for (name, val) in fields {
                    println!("  {name}: {val}");
                }
            }
        }
        OutputFormat::Json => {
            let entries: Vec<serde_json::Value> = records
                .iter()
                .map(|(id_str, fields)| {
                    let mut obj = serde_json::Map::new();
                    obj.insert("id".to_string(), serde_json::Value::String(id_str.clone()));
                    for (name, val) in fields {
                        obj.insert(name.to_string(), serde_json::Value::String(val.clone()));
                    }
                    serde_json::Value::Object(obj)
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&entries)?);
        }
        OutputFormat::Toml => {
            for (id_str, fields) in &records {
                println!("[[entities]]");
                println!("id = \"{}\"", toml_escape(id_str));
                for (name, val) in fields {
                    println!("{name} = \"{}\"", toml_escape(val));
                }
                println!();
            }
        }
    }

    Ok(())
}

fn toml_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
