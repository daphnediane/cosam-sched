/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! CLI argument parsing for `cosam-modify`.

use std::path::PathBuf;

use anyhow::{bail, Result};

// ── OutputFormat ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
    Toml,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Text
    }
}

// ── EntityTypeName ────────────────────────────────────────────────────────────

/// The entity type named by `--select <type>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityTypeName {
    Panel,
    Presenter,
    EventRoom,
    HotelRoom,
    PanelType,
}

impl Default for EntityTypeName {
    fn default() -> Self {
        Self::Panel
    }
}

impl EntityTypeName {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "panel" | "panels" => Some(Self::Panel),
            "presenter" | "presenters" => Some(Self::Presenter),
            "event_room" | "room" | "rooms" | "event_rooms" => Some(Self::EventRoom),
            "hotel_room" | "hotel_rooms" => Some(Self::HotelRoom),
            "panel_type" | "type" | "types" | "panel_types" => Some(Self::PanelType),
            _ => None,
        }
    }

    pub fn type_name(self) -> &'static str {
        match self {
            Self::Panel => "panel",
            Self::Presenter => "presenter",
            Self::EventRoom => "event_room",
            Self::HotelRoom => "hotel_room",
            Self::PanelType => "panel_type",
        }
    }
}

// ── StageCommand ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum StageCommand {
    List,
    Get { query: String },
    Set { field: String, value: String },
    Create { fields: Vec<(String, String)> },
    Delete { query: String },
    AddEdge { edge_field: String, value: String },
    RemoveEdge { edge_field: String, value: String },
    Undo,
    Redo,
    ShowHistory,
}

// ── Stage ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Stage {
    pub entity_type: EntityTypeName,
    /// `None` or `"*"` / `"all"` = all entities of the type.
    pub entity_query: Option<String>,
    pub command: StageCommand,
}

// ── CliArgs ───────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct CliArgs {
    pub file: PathBuf,
    pub format: OutputFormat,
    pub create_new: bool,
    pub stages: Vec<Stage>,
}

// ── Parsing ───────────────────────────────────────────────────────────────────

pub fn parse_args() -> Result<CliArgs> {
    let raw: Vec<String> = std::env::args().collect();
    let args: Vec<&str> = raw.iter().map(|s| s.as_str()).collect();

    // Split on literal "--" tokens to get per-stage slices.
    // The prefix before the first stage (index 1 onward) holds global flags.
    let mut global_end = args.len();
    for (i, a) in args.iter().enumerate().skip(1) {
        if *a == "--" {
            global_end = i;
            break;
        }
        // Stop at the first command or stage-start word (single-stage invocation).
        if is_command_word(a) || *a == "--select" {
            global_end = i;
            break;
        }
    }

    let global_args = &args[1..global_end];
    let stage_args_raw = if global_end < args.len() {
        &args[global_end..]
    } else {
        &[]
    };

    // ── Parse global flags ────────────────────────────────────────────────────
    let mut file: Option<PathBuf> = None;
    let mut format = OutputFormat::default();
    let mut create_new = false;

    let mut i = 0;
    while i < global_args.len() {
        match global_args[i] {
            "--file" | "-f" => {
                i += 1;
                if i >= global_args.len() {
                    bail!("Missing value for --file");
                }
                file = Some(PathBuf::from(global_args[i]));
            }
            "--format" => {
                i += 1;
                if i >= global_args.len() {
                    bail!("Missing value for --format");
                }
                format = parse_format(global_args[i])?;
            }
            "--new" => {
                create_new = true;
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            other => {
                bail!("Unknown global option: {other}");
            }
        }
        i += 1;
    }

    let file = file.ok_or_else(|| anyhow::anyhow!("--file <path> is required"))?;

    // ── Split remaining args into stages separated by "--" ────────────────────
    let mut stage_chunks: Vec<&[&str]> = Vec::new();
    let mut chunk_start = 0;
    for (j, a) in stage_args_raw.iter().enumerate() {
        if *a == "--" {
            if j > chunk_start {
                stage_chunks.push(&stage_args_raw[chunk_start..j]);
            }
            chunk_start = j + 1;
        }
    }
    if chunk_start < stage_args_raw.len() {
        stage_chunks.push(&stage_args_raw[chunk_start..]);
    }

    // ── Parse each stage ──────────────────────────────────────────────────────
    let mut stages: Vec<Stage> = Vec::new();
    for chunk in stage_chunks {
        if chunk.is_empty() {
            continue;
        }
        stages.push(parse_stage(chunk)?);
    }

    Ok(CliArgs {
        file,
        format,
        create_new,
        stages,
    })
}

fn parse_format(s: &str) -> Result<OutputFormat> {
    match s.to_lowercase().as_str() {
        "text" => Ok(OutputFormat::Text),
        "json" => Ok(OutputFormat::Json),
        "toml" => Ok(OutputFormat::Toml),
        _ => bail!("Unknown format '{s}'; expected text, json, or toml"),
    }
}

fn is_command_word(s: &str) -> bool {
    matches!(
        s,
        "list"
            | "get"
            | "set"
            | "create"
            | "delete"
            | "add-edge"
            | "remove-edge"
            | "undo"
            | "redo"
            | "show-history"
    )
}

fn parse_stage(args: &[&str]) -> Result<Stage> {
    let mut entity_type = EntityTypeName::default();
    let mut entity_query: Option<String> = None;
    let mut i = 0;

    // Consume optional --select <type> [<query>]
    while i < args.len() && args[i] == "--select" {
        i += 1;
        if i >= args.len() {
            bail!("--select requires an entity type argument");
        }
        entity_type = EntityTypeName::from_str(args[i])
            .ok_or_else(|| anyhow::anyhow!("Unknown entity type '{}'; expected panel, presenter, event_room, hotel_room, or panel_type", args[i]))?;
        i += 1;
        // Optional query — anything that isn't another flag or command word.
        if i < args.len() && !args[i].starts_with("--") && !is_command_word(args[i]) {
            let q = args[i].to_string();
            entity_query = if q == "*" || q.to_lowercase() == "all" {
                None
            } else {
                Some(q)
            };
            i += 1;
        }
    }

    if i >= args.len() {
        bail!("Stage is missing a command");
    }

    let command = match args[i] {
        "list" => {
            i += 1;
            StageCommand::List
        }
        "get" => {
            i += 1;
            let query = next_arg(args, &mut i, "get requires a query argument")?;
            StageCommand::Get { query }
        }
        "set" => {
            i += 1;
            let field = next_arg(args, &mut i, "set requires a field name")?;
            let value = next_arg(args, &mut i, "set requires a value")?;
            StageCommand::Set { field, value }
        }
        "create" => {
            i += 1;
            let mut fields: Vec<(String, String)> = Vec::new();
            while i < args.len() {
                if args[i] == "--field" {
                    i += 1;
                    let name = next_arg(args, &mut i, "--field requires a name")?;
                    let value = next_arg(args, &mut i, "--field requires a value after the name")?;
                    fields.push((name, value));
                } else if let Some((name, value)) = args[i].split_once('=') {
                    fields.push((name.to_string(), value.to_string()));
                    i += 1;
                } else {
                    break;
                }
            }
            StageCommand::Create { fields }
        }
        "delete" => {
            i += 1;
            let query = next_arg(args, &mut i, "delete requires a query argument")?;
            StageCommand::Delete { query }
        }
        "add-edge" => {
            i += 1;
            let edge_field = next_arg(args, &mut i, "add-edge requires an edge field name")?;
            let value = next_arg(args, &mut i, "add-edge requires a target query")?;
            StageCommand::AddEdge { edge_field, value }
        }
        "remove-edge" => {
            i += 1;
            let edge_field = next_arg(args, &mut i, "remove-edge requires an edge field name")?;
            let value = next_arg(args, &mut i, "remove-edge requires a target query")?;
            StageCommand::RemoveEdge { edge_field, value }
        }
        "undo" => {
            i += 1;
            StageCommand::Undo
        }
        "redo" => {
            i += 1;
            StageCommand::Redo
        }
        "show-history" => {
            i += 1;
            StageCommand::ShowHistory
        }
        other => bail!("Unknown command '{other}'"),
    };

    if i < args.len() {
        bail!("Unexpected argument after command: '{}'", args[i]);
    }

    Ok(Stage {
        entity_type,
        entity_query,
        command,
    })
}

fn next_arg(args: &[&str], i: &mut usize, msg: &str) -> Result<String> {
    if *i >= args.len() {
        bail!("{msg}");
    }
    let val = args[*i].to_string();
    *i += 1;
    Ok(val)
}

fn print_usage() {
    eprintln!(
        "cosam-modify -- CLI editing tool for cosam schedules

USAGE:
    cosam-modify --file <path> [OPTIONS] [--select <type> [<query>]] <command> [args]
                 [-- [--select <type> [<query>]] <command> [args] ...]

GLOBAL OPTIONS:
    --file, -f <path>       Schedule file to modify (required)
    --new                   Create a new schedule if the file does not exist
    --format <fmt>          Output format: text (default), json, toml
    --help, -h              Show this help

ENTITY TYPES (for --select):
    panel           Schedule panel / session
    presenter       Presenter / speaker
    event_room      Event room
    hotel_room      Hotel room
    panel_type      Panel type / prefix

COMMANDS:
    list                    List all (or selected) entities
    get <query>             Show all fields of a single entity
    set <field> <value>     Update a field on the selected entity/entities
    create [field=value...] Create a new entity
    delete <query>          Delete an entity
    add-edge <edge> <target>    Add a relationship
    remove-edge <edge> <target> Remove a relationship
    undo                    Undo the most recent edit (in-memory)
    redo                    Redo the most recently undone edit (in-memory)
    show-history            Show undo/redo stack depth

EXAMPLES:
    cosam-modify --file sched.cosam --select panel list
    cosam-modify --file sched.cosam --select presenter get \"Jane Smith\"
    cosam-modify --file sched.cosam --select panel \"My Panel\" set note \"Updated\"
    cosam-modify --file sched.cosam --new --select panel_type create prefix=GP kind=\"Guest Panel\"
    cosam-modify --file sched.cosam --select panel \"My Panel\" set name \"New Name\" \\
      -- --select panel list"
    );
}
