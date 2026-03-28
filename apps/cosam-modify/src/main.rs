/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Utc, Weekday};
use schedule_core::ScheduleFile;
use schedule_core::data::Schedule;
use schedule_core::data::panel::ExtraValue;
use schedule_core::data::time;
use schedule_core::edit::{EditContext, PanelField, SessionScheduleState};
use schedule_core::xlsx::{XlsxImportOptions, canonical_header};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Human,
    Json,
}

struct CliArgs {
    file: PathBuf,
    format: OutputFormat,
    import_options: XlsxImportOptions,
    stages: Vec<Stage>,
}

#[derive(Debug, Clone, Default)]
struct Selectors {
    queries: Vec<SelectQuery>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectScope {
    Panel,
    Presenter,
    Room,
    Type,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectField {
    Day,
    Start,
    Duration,
    Room,
    Presenter,
    Type,
    Id,
    Name,
    Prefix,
    Kind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectRelationship {
    Smart,
    LessThan,
    LessThanOrEqual,
    Equal,
    GreaterThanOrEqual,
    GreaterThan,
    NotEqual,
    Contains,
}

#[derive(Debug, Clone)]
struct SelectClause {
    relationship: SelectRelationship,
    value: String,
}

#[derive(Debug, Clone)]
struct SelectQuery {
    scope: SelectScope,
    field: Option<SelectField>,
    clauses: Vec<SelectClause>,
}

#[derive(Debug, Clone)]
struct Stage {
    selectors: Selectors,
    command: Command,
}

#[derive(Debug, Clone)]
enum ListTarget {
    Panels,
    Rooms,
    Presenters,
}

#[derive(Debug, Clone)]
enum SetField {
    Description,
    Note,
    AvNote,
}

#[derive(Debug, Clone)]
enum Command {
    List(ListTarget),
    Set {
        field: SetField,
        value: String,
    },
    AddPresenter {
        name: String,
    },
    RemovePresenter {
        name: String,
    },
    Reschedule {
        room_name: Option<String>,
        day: Option<String>,
        start_time: Option<String>,
        end_time: Option<String>,
        duration: Option<String>,
    },
    Cancel,
    RemoveSession,
    UpdateFromJson {
        path: PathBuf,
    },
    Query {
        fields: Vec<String>,
    },
    SetMetadata {
        key: String,
        value: String,
    },
    ClearMetadata {
        key: String,
    },
    Undo,
    Redo,
    ShowHistory,
}

#[derive(Debug, Clone)]
struct SessionRef {
    panel_id: String,
}

#[derive(Debug, Clone, Default)]
struct SelectionResult {
    panel_ids: BTreeSet<String>,
    sessions: Vec<SessionRef>,
    room_ids: Vec<u32>,
    panel_type_prefixes: Vec<String>,
    explicit_session_selector: bool,
}

#[derive(Debug, Serialize)]
struct PanelListRow {
    id: String,
    name: String,
    panel_type: Option<String>,
    room_names: Vec<String>,
    start_times: Vec<String>,
    presenters: Vec<String>,
    session_count: usize,
}

#[derive(Debug, Serialize)]
struct RoomListRow {
    short_name: String,
    long_name: String,
    hotel_room: String,
    sort_key: u32,
}

#[derive(Debug, Serialize)]
struct PresenterListRow {
    name: String,
    rank: String,
    is_group: bool,
}

fn print_usage() {
    eprintln!(
        "Usage: cosam-modify --file <schedule.xlsx|schedule.json> [global options] <stage> [-- <stage> ...]\n\
         \n\
         Stage format:\n\
         \x20 [selectors...] <command> [command args]\n\
         \n\
         Global options:\n\
         \x20 --file, -f <path>                 Input/output schedule file (.xlsx or .json)\n\
         \x20 --format <human|json>             List output format (default: human)\n\
         \x20 --schedule-table <name>           Table name for schedule data (default: Schedule)\n\
         \x20 --roommap-table <name>            Table name for room mapping (default: RoomMap)\n\
         \x20 --prefix-table <name>             Table name for panel types (default: Prefix)\n\
         \x20 --presenter-table <name>          Table name for presenters (default: Presenters)\n\
         \x20 --title <string>                  Import title override (default: Event Schedule)\n\
         \x20 --help, -h                        Show this help message\n\
         \n\
         Selectors (AND semantics):\n\
         \x20 --select [panel|presenter|room|type] [<field>:][relationship:]<value>[,[relationship:]<value>...]\n\
         \n\
         Relationships: <, <=, =, =>, >, !, ~ (optional trailing colon)\n\
         Panel fields: day, start|begin, dur|duration|length, room, presenter, type, id, name\n\
         Room fields:  id, name\n\
         Type fields:  prefix|id, kind|name\n\
         Wildcard: use * or all to select all entities of a scope\n\
         \n\
         Commands:\n\
         \x20 list [panels|rooms|presenters]\n\
         \x20 query <field>[,<field>...]  (fields: id,name,description,note,av-note,room,start,end,duration,type,cost,capacity,difficulty,prereq,presenters,credited,uncredited,metadata,change-state,all)\n\
         \x20 set description|note|av-note <text>\n\
         \x20 set-metadata <key> <value>\n\
         \x20 clear-metadata <key>\n\
         \x20 add-presenter <name>\n\
         \x20 remove-presenter <name>\n\
         \x20 reschedule [-room|--room <name>] [--day <weekday>] [--start <hh>:<mm>] [--duration <min|hour:min>] [--end <hh>:<mm>]\n\
         \x20 cancel\n\
         \x20 remove-session\n\
         \x20 update-from-json <path.json>\n\
         \n\
         Examples:\n\
         \x20 cosam-modify --file test.xlsx --select panel name:\"Armor 101\" set note \"Bring foam\"\n\
         \x20 cosam-modify --file test.xlsx --select room name:\"Main Events\" list panels\n\
         \x20 cosam-modify --file test.xlsx --select panel id:panel-10-1 remove-session\n\
         \x20 cosam-modify --file test.xlsx --select presenter name:\"Yaya Han\" add-presenter \"Guest Host\" -- --select panel id:panel-20 cancel\n\
         \x20 cosam-modify --file test.xlsx --select type prefix:GP set-metadata ThemeColor '#FF8800'\n\
         \x20 cosam-modify --file test.xlsx --select room name:'Workshop 1' set-metadata Notes 'Has sewing machines'\n\
         \x20 cosam-modify --file test.xlsx --select type '*' query prefix,kind,metadata\n\
         \x20 cosam-modify --file test.xlsx --select room '*' query name,metadata"
    );
}

fn parse_args() -> Result<CliArgs> {
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.len() == 1 {
        anyhow::bail!("Missing arguments");
    }

    let mut file: Option<PathBuf> = None;
    let mut format = OutputFormat::Human;
    let mut import_options = XlsxImportOptions::default();

    let mut index = 1;
    while index < arguments.len() {
        if arguments[index] == "--" {
            break;
        }
        if !arguments[index].starts_with('-') {
            break;
        }

        match arguments[index].as_str() {
            "--file" | "-f" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --file");
                }
                file = Some(PathBuf::from(&arguments[index]));
            }
            "--format" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --format");
                }
                format = match arguments[index].to_lowercase().as_str() {
                    "human" => OutputFormat::Human,
                    "json" => OutputFormat::Json,
                    other => anyhow::bail!("Unsupported format: {other}"),
                };
            }
            "--schedule-table" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --schedule-table");
                }
                import_options.schedule_table = arguments[index].clone();
            }
            "--roommap-table" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --roommap-table");
                }
                import_options.rooms_table = arguments[index].clone();
            }
            "--prefix-table" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --prefix-table");
                }
                import_options.panel_types_table = arguments[index].clone();
            }
            "--presenter-table" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --presenter-table");
                }
                import_options.people_table = arguments[index].clone();
            }
            "--title" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --title");
                }
                import_options.title = arguments[index].clone();
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            "--select" => {
                break;
            }
            "list" | "set" | "add-presenter" | "remove-presenter" | "reschedule" | "cancel"
            | "remove-session" | "update-from-json" | "query" | "set-metadata"
            | "clear-metadata" => {
                break;
            }
            other => anyhow::bail!("Unknown argument: {other}"),
        }

        index += 1;
    }

    let Some(file) = file else {
        anyhow::bail!("--file is required");
    };

    if index >= arguments.len() {
        anyhow::bail!("At least one stage is required");
    }

    let raw_stage_args = &arguments[index..];
    let mut raw_stages: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();

    for token in raw_stage_args {
        if token == "--" {
            if current.is_empty() {
                anyhow::bail!("Empty stage before '--'");
            }
            raw_stages.push(std::mem::take(&mut current));
            continue;
        }
        current.push(token.clone());
    }

    if current.is_empty() {
        anyhow::bail!("Trailing '--' without a stage");
    }
    raw_stages.push(current);

    let stages = raw_stages
        .iter()
        .map(|tokens| parse_stage(tokens))
        .collect::<Result<Vec<_>>>()?;

    Ok(CliArgs {
        file,
        format,
        import_options,
        stages,
    })
}

fn parse_stage(tokens: &[String]) -> Result<Stage> {
    let mut selectors = Selectors::default();
    let mut index = 0;

    while index < tokens.len() && tokens[index] == "--select" {
        match tokens[index].as_str() {
            "--select" => {
                index += 1;
                let first = tokens
                    .get(index)
                    .cloned()
                    .context("Missing value for --select")?;
                let (scope, expression) = if let Some(scope) = parse_select_scope(&first) {
                    index += 1;
                    let expression = tokens
                        .get(index)
                        .cloned()
                        .context("Missing selector expression for --select")?;
                    (scope, expression)
                } else {
                    (SelectScope::Panel, first)
                };

                selectors
                    .queries
                    .push(parse_select_query(scope, &expression)?);
            }
            other => anyhow::bail!("Unknown selector option: {other}"),
        }
        index += 1;
    }

    let Some(command_name) = tokens.get(index).map(String::as_str) else {
        anyhow::bail!("Stage is missing a command");
    };

    let command = match command_name {
        "list" => {
            index += 1;
            let target = match tokens.get(index).map(String::as_str) {
                None => ListTarget::Panels,
                Some("panels") => {
                    index += 1;
                    ListTarget::Panels
                }
                Some("rooms") => {
                    index += 1;
                    ListTarget::Rooms
                }
                Some("presenters") => {
                    index += 1;
                    ListTarget::Presenters
                }
                Some(other) => anyhow::bail!("Unknown list target: {other}"),
            };
            Command::List(target)
        }
        "set" => {
            index += 1;
            let field = match tokens
                .get(index)
                .map(String::as_str)
                .context("Missing field for set command")?
            {
                "description" => SetField::Description,
                "note" => SetField::Note,
                "av-note" | "av-notes" => SetField::AvNote,
                other => anyhow::bail!("Unsupported set field: {other}"),
            };
            index += 1;
            let value = tokens
                .get(index)
                .cloned()
                .context("Missing value for set command")?;
            index += 1;
            Command::Set { field, value }
        }
        "add-presenter" => {
            index += 1;
            let name = tokens
                .get(index)
                .cloned()
                .context("Missing presenter name for add-presenter")?;
            index += 1;
            Command::AddPresenter { name }
        }
        "remove-presenter" => {
            index += 1;
            let name = tokens
                .get(index)
                .cloned()
                .context("Missing presenter name for remove-presenter")?;
            index += 1;
            Command::RemovePresenter { name }
        }
        "reschedule" => {
            index += 1;
            let mut room_name: Option<String> = None;
            let mut day: Option<String> = None;
            let mut start_time: Option<String> = None;
            let mut end_time: Option<String> = None;
            let mut duration: Option<String> = None;

            while index < tokens.len() {
                match tokens[index].as_str() {
                    "--room" | "-room" => {
                        index += 1;
                        room_name = Some(
                            tokens
                                .get(index)
                                .cloned()
                                .context("Missing value for --room")?,
                        );
                    }
                    "--day" => {
                        index += 1;
                        day = Some(
                            tokens
                                .get(index)
                                .cloned()
                                .context("Missing value for --day")?,
                        );
                    }
                    "--start" => {
                        index += 1;
                        start_time = Some(
                            tokens
                                .get(index)
                                .cloned()
                                .context("Missing value for --start")?,
                        );
                    }
                    "--end" => {
                        index += 1;
                        end_time = Some(
                            tokens
                                .get(index)
                                .cloned()
                                .context("Missing value for --end")?,
                        );
                    }
                    "--duration" => {
                        index += 1;
                        duration = Some(
                            tokens
                                .get(index)
                                .cloned()
                                .context("Missing value for --duration")?,
                        );
                    }
                    other => anyhow::bail!("Unknown reschedule option: {other}"),
                }
                index += 1;
            }

            if room_name.is_none()
                && day.is_none()
                && start_time.is_none()
                && end_time.is_none()
                && duration.is_none()
            {
                anyhow::bail!(
                    "reschedule requires at least one option (--room, --day, --start, --end, --duration)"
                );
            }

            if end_time.is_some() && duration.is_some() {
                anyhow::bail!("reschedule accepts either --end or --duration, not both");
            }

            Command::Reschedule {
                room_name,
                day,
                start_time,
                end_time,
                duration,
            }
        }
        "cancel" => {
            index += 1;
            Command::Cancel
        }
        "remove-session" => {
            index += 1;
            Command::RemoveSession
        }
        "update-from-json" => {
            index += 1;
            let path = PathBuf::from(
                tokens
                    .get(index)
                    .cloned()
                    .context("Missing path for update-from-json")?,
            );
            index += 1;
            Command::UpdateFromJson { path }
        }
        "query" => {
            index += 1;
            let spec = tokens
                .get(index)
                .cloned()
                .context("query requires at least one field name")?;
            index += 1;
            let fields: Vec<String> = spec
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect();
            if fields.is_empty() {
                anyhow::bail!("query requires at least one field name");
            }
            Command::Query { fields }
        }
        "set-metadata" => {
            index += 1;
            let key = tokens
                .get(index)
                .cloned()
                .context("set-metadata requires a key")?;
            index += 1;
            let value = tokens
                .get(index)
                .cloned()
                .context("set-metadata requires a value")?;
            index += 1;
            Command::SetMetadata { key, value }
        }
        "clear-metadata" => {
            index += 1;
            let key = tokens
                .get(index)
                .cloned()
                .context("clear-metadata requires a key")?;
            index += 1;
            Command::ClearMetadata { key }
        }
        "undo" => {
            index += 1;
            Command::Undo
        }
        "redo" => {
            index += 1;
            Command::Redo
        }
        "show-history" => {
            index += 1;
            Command::ShowHistory
        }
        other => anyhow::bail!("Unknown command: {other}"),
    };

    if index != tokens.len() {
        anyhow::bail!("Unexpected trailing arguments in stage");
    }

    Ok(Stage { selectors, command })
}

fn parse_select_scope(value: &str) -> Option<SelectScope> {
    match value.to_ascii_lowercase().as_str() {
        "panel" => Some(SelectScope::Panel),
        "presenter" => Some(SelectScope::Presenter),
        "room" => Some(SelectScope::Room),
        "type" | "panel-type" | "panel_type" => Some(SelectScope::Type),
        _ => None,
    }
}

fn normalize_field_name(s: &str) -> String {
    canonical_header(s)
        .map(|k| k.to_lowercase())
        .unwrap_or_else(|| s.to_lowercase().replace('-', "_"))
}

fn parse_select_field(value: &str) -> Option<SelectField> {
    match normalize_field_name(value).as_str() {
        "day" => Some(SelectField::Day),
        "start" | "begin" => Some(SelectField::Start),
        "dur" | "duration" | "length" => Some(SelectField::Duration),
        "room" => Some(SelectField::Room),
        "presenter" => Some(SelectField::Presenter),
        "type" => Some(SelectField::Type),
        "id" => Some(SelectField::Id),
        "name" => Some(SelectField::Name),
        "prefix" => Some(SelectField::Prefix),
        "kind" => Some(SelectField::Kind),
        _ => None,
    }
}

fn parse_select_query(scope: SelectScope, expression: &str) -> Result<SelectQuery> {
    let (field, remaining) = if let Some((prefix, rest)) = expression.split_once(':') {
        if let Some(field) = parse_select_field(prefix) {
            (Some(field), rest)
        } else {
            (None, expression)
        }
    } else {
        (None, expression)
    };

    let clauses = remaining
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(parse_select_clause)
        .collect::<Result<Vec<_>>>()?;

    if clauses.is_empty() {
        anyhow::bail!("Selector expression is empty");
    }

    Ok(SelectQuery {
        scope,
        field,
        clauses,
    })
}

fn parse_select_clause(value: &str) -> Result<SelectClause> {
    let tokens = [
        ("<=", SelectRelationship::LessThanOrEqual),
        (">=", SelectRelationship::GreaterThanOrEqual),
        ("=>", SelectRelationship::GreaterThanOrEqual),
        ("<", SelectRelationship::LessThan),
        (">", SelectRelationship::GreaterThan),
        ("=", SelectRelationship::Equal),
        ("!", SelectRelationship::NotEqual),
        ("~", SelectRelationship::Contains),
    ];

    for (prefix, relationship) in tokens {
        if let Some(stripped) = value.strip_prefix(prefix) {
            let stripped = stripped.strip_prefix(':').unwrap_or(stripped).trim();
            if stripped.is_empty() {
                anyhow::bail!("Missing selector value after relationship '{prefix}'");
            }
            return Ok(SelectClause {
                relationship,
                value: stripped.to_string(),
            });
        }
    }

    Ok(SelectClause {
        relationship: SelectRelationship::Smart,
        value: value.trim().to_string(),
    })
}

fn eq_ignore_case(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

fn cmp_string_ignore_case(left: &str, right: &str) -> std::cmp::Ordering {
    left.to_ascii_lowercase().cmp(&right.to_ascii_lowercase())
}

fn session_presenters(panel: &schedule_core::data::Panel) -> Vec<String> {
    panel.credited_presenters.clone()
}

fn parse_time_selector_value(value: &str) -> Option<u32> {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized == "noon" {
        return Some(12 * 60);
    }
    if normalized == "midnight" {
        return Some(0);
    }

    let compact = normalized.replace(' ', "");
    let formats = [
        "%H:%M", "%H", "%I:%M%P", "%I%P", "%I:%M%p", "%I%p", "%I:%M %p", "%I %p",
    ];

    for format in formats {
        if let Ok(parsed) = NaiveTime::parse_from_str(&compact, format) {
            return Some(parsed.hour() * 60 + parsed.minute());
        }
        if let Ok(parsed) = NaiveTime::parse_from_str(&normalized, format) {
            return Some(parsed.hour() * 60 + parsed.minute());
        }
    }

    None
}

fn text_clause_matches(candidates: &[String], clause: &SelectClause) -> bool {
    // Wildcard: * or "all" with Smart relationship matches everything.
    if clause.relationship == SelectRelationship::Smart
        && (clause.value == "*" || clause.value.eq_ignore_ascii_case("all"))
    {
        return true;
    }
    match clause.relationship {
        SelectRelationship::Smart => {
            if candidates
                .iter()
                .any(|candidate| eq_ignore_case(candidate, &clause.value))
            {
                true
            } else {
                candidates
                    .iter()
                    .any(|candidate| contains_ignore_case(candidate, &clause.value))
            }
        }
        SelectRelationship::Contains => candidates
            .iter()
            .any(|candidate| contains_ignore_case(candidate, &clause.value)),
        SelectRelationship::Equal => candidates
            .iter()
            .any(|candidate| eq_ignore_case(candidate, &clause.value)),
        SelectRelationship::NotEqual => candidates
            .iter()
            .all(|candidate| !eq_ignore_case(candidate, &clause.value)),
        SelectRelationship::LessThan => candidates
            .iter()
            .any(|candidate| cmp_string_ignore_case(candidate, &clause.value).is_lt()),
        SelectRelationship::LessThanOrEqual => candidates
            .iter()
            .any(|candidate| !cmp_string_ignore_case(candidate, &clause.value).is_gt()),
        SelectRelationship::GreaterThan => candidates
            .iter()
            .any(|candidate| cmp_string_ignore_case(candidate, &clause.value).is_gt()),
        SelectRelationship::GreaterThanOrEqual => candidates
            .iter()
            .any(|candidate| !cmp_string_ignore_case(candidate, &clause.value).is_lt()),
    }
}

fn compare_u32(left: u32, right: u32, relationship: SelectRelationship) -> bool {
    match relationship {
        SelectRelationship::Smart | SelectRelationship::Equal => left == right,
        SelectRelationship::NotEqual => left != right,
        SelectRelationship::LessThan => left < right,
        SelectRelationship::LessThanOrEqual => left <= right,
        SelectRelationship::GreaterThan => left > right,
        SelectRelationship::GreaterThanOrEqual => left >= right,
        SelectRelationship::Contains => left.to_string().contains(&right.to_string()),
    }
}

fn compare_u32_option(left: Option<u32>, right: u32, relationship: SelectRelationship) -> bool {
    if let Some(left_val) = left {
        compare_u32(left_val, right, relationship)
    } else {
        false
    }
}

fn query_matches_session(
    schedule: &Schedule,
    panel: &schedule_core::data::Panel,
    query: &SelectQuery,
) -> bool {
    query
        .clauses
        .iter()
        .any(|clause| query_clause_matches_session(schedule, panel, query, clause))
}

fn query_clause_matches_session(
    schedule: &Schedule,
    panel: &schedule_core::data::Panel,
    query: &SelectQuery,
    clause: &SelectClause,
) -> bool {
    match query.field {
        Some(SelectField::Day) => {
            let Some(start_text) = panel.start_time_str() else {
                return false;
            };
            let Ok(start_time) = parse_timestamp(&start_text) else {
                return false;
            };

            if let Ok(target_date) = NaiveDate::parse_from_str(&clause.value, "%Y-%m-%d") {
                match clause.relationship {
                    SelectRelationship::Smart | SelectRelationship::Equal => {
                        start_time.date() == target_date
                    }
                    SelectRelationship::NotEqual => start_time.date() != target_date,
                    SelectRelationship::LessThan => start_time.date() < target_date,
                    SelectRelationship::LessThanOrEqual => start_time.date() <= target_date,
                    SelectRelationship::GreaterThan => start_time.date() > target_date,
                    SelectRelationship::GreaterThanOrEqual => start_time.date() >= target_date,
                    SelectRelationship::Contains => {
                        contains_ignore_case(&start_time.date().to_string(), &clause.value)
                    }
                }
            } else if let Ok(target_weekday) = parse_weekday(&clause.value) {
                match clause.relationship {
                    SelectRelationship::Smart
                    | SelectRelationship::Equal
                    | SelectRelationship::Contains => start_time.date().weekday() == target_weekday,
                    SelectRelationship::NotEqual => start_time.date().weekday() != target_weekday,
                    _ => false,
                }
            } else {
                let date_text = start_time.date().to_string();
                text_clause_matches(&[date_text], clause)
            }
        }
        Some(SelectField::Start) => {
            let Some(start_text) = panel.start_time_str() else {
                return false;
            };
            let session_minutes = parse_timestamp(&start_text)
                .ok()
                .map(|value| value.time().hour() * 60 + value.time().minute());
            let target_minutes = parse_time_selector_value(&clause.value);

            if let (Some(left), Some(right)) = (session_minutes, target_minutes) {
                compare_u32(left, right, clause.relationship)
            } else {
                text_clause_matches(&[start_text.to_string()], clause)
            }
        }
        Some(SelectField::Duration) => {
            let target = parse_duration_spec(&clause.value).ok();
            if let Some(target) = target {
                compare_u32_option(
                    panel.effective_duration_minutes(),
                    target,
                    clause.relationship,
                )
            } else {
                text_clause_matches(
                    &[panel.effective_duration_minutes().unwrap_or(0).to_string()],
                    clause,
                )
            }
        }
        Some(SelectField::Room) => {
            let mut room_names = Vec::new();
            for room_id in &panel.room_ids {
                if let Some(room) = schedule.room_by_id(*room_id) {
                    room_names.push(room.short_name.clone());
                    room_names.push(room.long_name.clone());
                    room_names.push(room.hotel_room.clone());
                }
            }
            text_clause_matches(&room_names, clause)
        }
        Some(SelectField::Presenter) => {
            let names = session_presenters(panel);
            text_clause_matches(&names, clause)
        }
        Some(SelectField::Type) => {
            let mut values = Vec::new();
            if let Some(ref panel_type_id) = panel.panel_type {
                values.push(panel_type_id.clone());
                if let Some(panel_type) = schedule.panel_types.get(panel_type_id) {
                    values.push(panel_type.kind.clone());
                    values.push(panel_type.prefix.clone());
                }
            }
            text_clause_matches(&values, clause)
        }
        Some(SelectField::Id) => text_clause_matches(&[panel.id.clone()], clause),
        Some(SelectField::Name) => match query.scope {
            SelectScope::Presenter => {
                let names = session_presenters(panel);
                text_clause_matches(&names, clause)
            }
            SelectScope::Room => {
                let mut room_names = Vec::new();
                for room_id in &panel.room_ids {
                    if let Some(room) = schedule.room_by_id(*room_id) {
                        room_names.push(room.short_name.clone());
                        room_names.push(room.long_name.clone());
                    }
                }
                text_clause_matches(&room_names, clause)
            }
            SelectScope::Panel => text_clause_matches(&[panel.name.clone()], clause),
            SelectScope::Type => false,
        },
        Some(SelectField::Prefix) | Some(SelectField::Kind) => false,
        None => match query.scope {
            SelectScope::Panel => {
                let mut values = vec![panel.id.clone(), panel.name.clone()];
                if let Some(ref panel_type_id) = panel.panel_type {
                    values.push(panel_type_id.clone());
                    if let Some(panel_type) = schedule.panel_types.get(panel_type_id) {
                        values.push(panel_type.kind.clone());
                        values.push(panel_type.prefix.clone());
                    }
                }
                if let Some(start) = panel.start_time_str() {
                    values.push(start);
                }
                if let Some(end) = panel.end_time_str() {
                    values.push(end);
                }
                values.push(panel.effective_duration_minutes().unwrap_or(0).to_string());
                for room_id in &panel.room_ids {
                    if let Some(room) = schedule.room_by_id(*room_id) {
                        values.push(room.short_name.clone());
                        values.push(room.long_name.clone());
                        values.push(room.hotel_room.clone());
                    }
                }
                values.extend(session_presenters(panel));
                text_clause_matches(&values, clause)
            }
            SelectScope::Presenter => {
                let names = session_presenters(panel);
                text_clause_matches(&names, clause)
            }
            SelectScope::Room => {
                let mut values = Vec::new();
                for room_id in &panel.room_ids {
                    if let Some(room) = schedule.room_by_id(*room_id) {
                        values.push(room.short_name.clone());
                        values.push(room.long_name.clone());
                        values.push(room.hotel_room.clone());
                    }
                }
                text_clause_matches(&values, clause)
            }
            SelectScope::Type => false,
        },
    }
}

fn query_matches_panel_only(
    schedule: &Schedule,
    panel: &schedule_core::data::Panel,
    query: &SelectQuery,
) -> bool {
    if query.scope != SelectScope::Panel {
        return false;
    }

    query.clauses.iter().any(|clause| match query.field {
        Some(SelectField::Id) => text_clause_matches(&[panel.id.clone()], clause),
        Some(SelectField::Name) => text_clause_matches(&[panel.name.clone()], clause),
        Some(SelectField::Type) => {
            let mut values = Vec::new();
            if let Some(ref panel_type_id) = panel.panel_type {
                values.push(panel_type_id.clone());
                if let Some(panel_type) = schedule.panel_types.get(panel_type_id) {
                    values.push(panel_type.kind.clone());
                    values.push(panel_type.prefix.clone());
                }
            }
            text_clause_matches(&values, clause)
        }
        Some(SelectField::Day)
        | Some(SelectField::Start)
        | Some(SelectField::Duration)
        | Some(SelectField::Room)
        | Some(SelectField::Presenter)
        | Some(SelectField::Prefix)
        | Some(SelectField::Kind) => false,
        None => {
            let mut values = vec![panel.id.clone(), panel.name.clone()];
            if let Some(ref panel_type_id) = panel.panel_type {
                values.push(panel_type_id.clone());
                if let Some(panel_type) = schedule.panel_types.get(panel_type_id) {
                    values.push(panel_type.kind.clone());
                    values.push(panel_type.prefix.clone());
                }
            }
            text_clause_matches(&values, clause)
        }
    })
}

fn session_matches(
    schedule: &Schedule,
    panel: &schedule_core::data::Panel,
    selectors: &Selectors,
) -> bool {
    selectors
        .queries
        .iter()
        .all(|query| query_matches_session(schedule, panel, query))
}

fn selectors_have_explicit_session_filter(selectors: &Selectors) -> bool {
    selectors.queries.iter().any(|query| {
        matches!(query.scope, SelectScope::Presenter | SelectScope::Room)
            || matches!(
                query.field,
                Some(SelectField::Day)
                    | Some(SelectField::Start)
                    | Some(SelectField::Duration)
                    | Some(SelectField::Room)
                    | Some(SelectField::Presenter)
                    | Some(SelectField::Id)
            )
    })
}

fn panel_type_matches_selectors(
    panel_type: &schedule_core::data::PanelType,
    selectors: &Selectors,
) -> bool {
    selectors.queries.iter().all(|query| {
        if query.scope != SelectScope::Type {
            return true;
        }
        query.clauses.iter().any(|clause| {
            let values: Vec<String> = match query.field {
                Some(SelectField::Prefix) | Some(SelectField::Id) => {
                    vec![panel_type.prefix.clone()]
                }
                Some(SelectField::Kind) | Some(SelectField::Name) => {
                    vec![panel_type.kind.clone()]
                }
                None => vec![panel_type.prefix.clone(), panel_type.kind.clone()],
                _ => return false,
            };
            text_clause_matches(&values, clause)
        })
    })
}

fn room_matches_selectors(room: &schedule_core::data::Room, selectors: &Selectors) -> bool {
    selectors.queries.iter().all(|query| {
        if query.scope != SelectScope::Room {
            return true;
        }
        query.clauses.iter().any(|clause| {
            let values: Vec<String> = match query.field {
                Some(SelectField::Name) | None => vec![
                    room.short_name.clone(),
                    room.long_name.clone(),
                    room.hotel_room.clone(),
                ],
                Some(SelectField::Id) => vec![room.uid.to_string()],
                _ => return false,
            };
            text_clause_matches(&values, clause)
        })
    })
}

fn collect_selection(schedule: &Schedule, selectors: &Selectors) -> SelectionResult {
    let mut result = SelectionResult {
        explicit_session_selector: selectors_have_explicit_session_filter(selectors),
        ..SelectionResult::default()
    };

    let no_filters = selectors.queries.is_empty();
    let has_room_queries = selectors
        .queries
        .iter()
        .any(|q| q.scope == SelectScope::Room);
    let has_type_queries = selectors
        .queries
        .iter()
        .any(|q| q.scope == SelectScope::Type);
    let has_panel_queries = selectors
        .queries
        .iter()
        .any(|q| !matches!(q.scope, SelectScope::Room | SelectScope::Type));
    let only_non_panel = !no_filters && !has_panel_queries;

    // Collect rooms only when explicitly selected or no filters at all.
    if no_filters || has_room_queries {
        for room in &schedule.rooms {
            if no_filters || room_matches_selectors(room, selectors) {
                result.room_ids.push(room.uid);
            }
        }
    }

    // Collect panel types only when explicitly selected or no filters at all.
    if no_filters || has_type_queries {
        for (_prefix, panel_type) in &schedule.panel_types {
            if no_filters || panel_type_matches_selectors(panel_type, selectors) {
                result.panel_type_prefixes.push(panel_type.prefix.clone());
            }
        }
    }

    // Skip panel/session collection when the selector is only room/type scoped.
    if only_non_panel {
        return result;
    }

    for ps in schedule.panel_sets.values() {
        for panel in &ps.panels {
            if no_filters {
                result.panel_ids.insert(panel.id.clone());
                result.sessions.push(SessionRef {
                    panel_id: panel.id.clone(),
                });
                continue;
            }

            if session_matches(schedule, panel, selectors) {
                result.panel_ids.insert(panel.id.clone());
                result.sessions.push(SessionRef {
                    panel_id: panel.id.clone(),
                });
            } else {
                let query_match = !selectors.queries.is_empty()
                    && selectors
                        .queries
                        .iter()
                        .all(|query| query_matches_panel_only(schedule, panel, query));
                if query_match {
                    result.panel_ids.insert(panel.id.clone());
                }
            }
        }
    }

    result
}

fn find_room_id(schedule: &Schedule, room_name: &str) -> Result<u32> {
    schedule
        .rooms
        .iter()
        .find(|room| {
            eq_ignore_case(&room.short_name, room_name)
                || eq_ignore_case(&room.long_name, room_name)
                || eq_ignore_case(&room.hotel_room, room_name)
        })
        .map(|room| room.uid)
        .with_context(|| format!("Room not found: {room_name}"))
}

fn room_name_for_output(schedule: &Schedule, room_id: u32) -> Option<String> {
    schedule
        .room_by_id(room_id)
        .map(|room| room.long_name.clone())
}

fn parse_timestamp(value: &str) -> Result<NaiveDateTime> {
    time::parse_storage(value)
        .with_context(|| format!("Invalid timestamp '{value}', expected YYYY-MM-DDTHH:MM:SS"))
}

fn parse_clock_time(value: &str) -> Result<(i64, NaiveTime)> {
    let mut parts = value.split(':');
    let hour_raw = parts
        .next()
        .context("Missing hour in time value")?
        .parse::<u32>()
        .with_context(|| format!("Invalid hour in '{value}'"))?;
    let minute = parts
        .next()
        .context("Missing minute in time value")?
        .parse::<u32>()
        .with_context(|| format!("Invalid minute in '{value}'"))?;
    if parts.next().is_some() {
        anyhow::bail!("Invalid time '{value}', expected HH:MM");
    }
    if minute > 59 {
        anyhow::bail!("Invalid time '{value}', minute must be 0..59");
    }
    let day_offset = (hour_raw / 24) as i64;
    let hour = hour_raw % 24;
    let time = NaiveTime::from_hms_opt(hour, minute, 0)
        .with_context(|| format!("Invalid time '{value}', expected HH:MM"))?;
    Ok((day_offset, time))
}

fn parse_duration_spec(value: &str) -> Result<u32> {
    if value.contains(':') {
        let (day_offset, time) = parse_clock_time(value)?;
        Ok((day_offset * 24 * 60 + i64::from(time.hour()) * 60 + i64::from(time.minute())) as u32)
    } else {
        value
            .parse::<u32>()
            .with_context(|| format!("Invalid duration '{value}', expected minutes or H:MM"))
    }
}

fn parse_weekday(value: &str) -> Result<Weekday> {
    match value.to_lowercase().as_str() {
        "mon" | "monday" => Ok(Weekday::Mon),
        "tue" | "tues" | "tuesday" => Ok(Weekday::Tue),
        "wed" | "wednesday" => Ok(Weekday::Wed),
        "thu" | "thurs" | "thursday" => Ok(Weekday::Thu),
        "fri" | "friday" => Ok(Weekday::Fri),
        "sat" | "saturday" => Ok(Weekday::Sat),
        "sun" | "sunday" => Ok(Weekday::Sun),
        _ => anyhow::bail!("Invalid weekday '{value}'"),
    }
}

fn resolve_target_date(
    day_option: Option<&str>,
    existing_start: Option<NaiveDateTime>,
    schedule_days: &[NaiveDate],
) -> Result<NaiveDate> {
    if let Some(day_text) = day_option {
        let weekday = parse_weekday(day_text)?;
        if let Some(date) = schedule_days.iter().find(|date| date.weekday() == weekday) {
            return Ok(*date);
        }
        if let Some(existing) = existing_start
            && existing.date().weekday() == weekday
        {
            return Ok(existing.date());
        }
        anyhow::bail!("No schedule date matches weekday '{day_text}'");
    }

    if let Some(existing) = existing_start {
        return Ok(existing.date());
    }

    anyhow::bail!(
        "Cannot determine target day; provide --day or ensure selected sessions have start_time"
    )
}

fn list_panels(
    schedule: &Schedule,
    selection: &SelectionResult,
    format: OutputFormat,
) -> Result<()> {
    let rows: Vec<PanelListRow> = schedule
        .panel_sets
        .values()
        .flat_map(|ps| ps.panels.iter())
        .filter(|panel| selection.panel_ids.contains(&panel.id))
        .map(|panel| {
            let mut room_names: BTreeSet<String> = BTreeSet::new();
            let mut start_times: BTreeSet<String> = BTreeSet::new();
            let mut presenters: BTreeSet<String> = BTreeSet::new();

            for room_id in &panel.room_ids {
                if let Some(name) = room_name_for_output(schedule, *room_id) {
                    room_names.insert(name);
                }
            }
            if let Some(start) = panel.start_time_str() {
                start_times.insert(start);
            }
            for name in &panel.credited_presenters {
                presenters.insert(name.clone());
            }

            PanelListRow {
                id: panel.id.clone(),
                name: panel.name.clone(),
                panel_type: panel.panel_type.clone(),
                room_names: room_names.into_iter().collect(),
                start_times: start_times.into_iter().collect(),
                presenters: presenters.into_iter().collect(),
                session_count: 1,
            }
        })
        .collect();

    match format {
        OutputFormat::Human => {
            println!("Panels: {}", rows.len());
            for row in rows {
                let room_label = if row.room_names.is_empty() {
                    "—".to_string()
                } else {
                    row.room_names.join(", ")
                };
                println!(
                    "- {} | {} | sessions={} | rooms={} ",
                    row.id, row.name, row.session_count, room_label
                );
            }
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&rows)?);
        }
    }

    Ok(())
}

fn list_rooms(
    schedule: &Schedule,
    selection: &SelectionResult,
    format: OutputFormat,
) -> Result<()> {
    let room_filter: Option<BTreeSet<u32>> = if selection.sessions.is_empty() {
        None
    } else {
        let mut ids = BTreeSet::new();
        for reference in &selection.sessions {
            if let Some(panel) = schedule
                .panel_sets
                .values()
                .flat_map(|ps| ps.panels.iter())
                .find(|p| p.id == reference.panel_id)
            {
                for room_id in &panel.room_ids {
                    ids.insert(*room_id);
                }
            }
        }
        Some(ids)
    };

    let rows: Vec<RoomListRow> = schedule
        .sorted_rooms()
        .iter()
        .filter(|room| {
            room_filter
                .as_ref()
                .is_none_or(|ids| ids.contains(&room.uid))
        })
        .map(|room| RoomListRow {
            short_name: room.short_name.clone(),
            long_name: room.long_name.clone(),
            hotel_room: room.hotel_room.clone(),
            sort_key: room.sort_key,
        })
        .collect();

    match format {
        OutputFormat::Human => {
            println!("Rooms: {}", rows.len());
            for row in rows {
                println!(
                    "- {} ({}) [{}]",
                    row.long_name, row.short_name, row.hotel_room
                );
            }
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&rows)?);
        }
    }

    Ok(())
}

fn list_presenters(
    schedule: &Schedule,
    selection: &SelectionResult,
    format: OutputFormat,
) -> Result<()> {
    let name_filter: Option<BTreeSet<String>> = if selection.sessions.is_empty() {
        None
    } else {
        let mut names = BTreeSet::new();
        for reference in &selection.sessions {
            if let Some(panel) = schedule
                .panel_sets
                .values()
                .flat_map(|ps| ps.panels.iter())
                .find(|p| p.id == reference.panel_id)
            {
                for name in &panel.credited_presenters {
                    names.insert(name.clone());
                }
            }
        }
        Some(names)
    };

    let rows: Vec<PresenterListRow> = schedule
        .presenters
        .iter()
        .filter(|presenter| {
            name_filter
                .as_ref()
                .is_none_or(|names| names.iter().any(|n| eq_ignore_case(n, &presenter.name)))
        })
        .map(|presenter| PresenterListRow {
            name: presenter.name.clone(),
            rank: presenter.rank.as_str().to_string(),
            is_group: schedule.relationships.is_group(&presenter.name),
        })
        .collect();

    match format {
        OutputFormat::Human => {
            println!("Presenters: {}", rows.len());
            for row in rows {
                println!("- {} [{}]", row.name, row.rank);
            }
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&rows)?);
        }
    }

    Ok(())
}

fn execute_set(
    sf: &mut ScheduleFile,
    selection: &SelectionResult,
    field: &SetField,
    value: &str,
) -> Result<()> {
    let mut ctx = sf.edit_context();
    match field {
        SetField::Description | SetField::Note => {
            if selection.panel_ids.is_empty() {
                anyhow::bail!("No panels matched selectors");
            }
            let panel_field = match field {
                SetField::Description => PanelField::Description,
                SetField::Note => PanelField::Note,
                SetField::AvNote => unreachable!(),
            };
            for panel_id in &selection.panel_ids {
                ctx.set_panel_field(panel_id, panel_field.clone(), Some(value.to_string()));
            }
        }
        SetField::AvNote => {
            if selection.sessions.is_empty() {
                anyhow::bail!("set av-note requires session matches");
            }
            for reference in &selection.sessions {
                ctx.set_panel_field(
                    &reference.panel_id,
                    PanelField::AvNotes,
                    Some(value.to_string()),
                );
            }
        }
    }

    Ok(())
}

fn apply_presenter_change(
    sf: &mut ScheduleFile,
    selection: &SelectionResult,
    presenter_name: &str,
    add: bool,
) -> Result<()> {
    if selection.sessions.is_empty() {
        anyhow::bail!("No sessions matched selectors for presenter update");
    }

    let mut ctx = sf.edit_context();
    for reference in &selection.sessions {
        if add {
            ctx.add_presenter_to_panel(&reference.panel_id, presenter_name);
        } else {
            ctx.remove_presenter_from_panel(&reference.panel_id, presenter_name);
        }
    }

    Ok(())
}

fn execute_reschedule(
    sf: &mut ScheduleFile,
    selection: &SelectionResult,
    room_name: Option<&str>,
    day: Option<&str>,
    start_time: Option<&str>,
    end_time: Option<&str>,
    duration: Option<&str>,
) -> Result<()> {
    if selection.sessions.is_empty() {
        anyhow::bail!("reschedule requires at least one selected session");
    }

    let room_id = if let Some(room_name) = room_name {
        Some(find_room_id(&sf.schedule, room_name)?)
    } else {
        None
    };
    let duration_override = if let Some(spec) = duration {
        Some(parse_duration_spec(spec)?)
    } else {
        None
    };
    let end_clock = if let Some(end_text) = end_time {
        Some(parse_clock_time(end_text)?)
    } else {
        None
    };
    let start_clock = if let Some(start_text) = start_time {
        Some(parse_clock_time(start_text)?)
    } else {
        None
    };
    let schedule_days = sf.schedule.days();

    // Pre-compute the new state for each session before borrowing mutably.
    let mut updates: Vec<(usize, SessionScheduleState)> = Vec::new();
    for (idx, reference) in selection.sessions.iter().enumerate() {
        let Some(panel) = sf
            .schedule
            .panel_sets
            .values()
            .flat_map(|ps| ps.panels.iter())
            .find(|p| p.id == reference.panel_id)
        else {
            continue;
        };

        let existing_start = panel
            .start_time_str()
            .map(|s| parse_timestamp(&s))
            .transpose()?;
        let base_date = resolve_target_date(day, existing_start, &schedule_days)?;

        let (start_day_offset, start_clock_time) = if let Some((offset, time)) = start_clock {
            (offset, time)
        } else if let Some(existing) = existing_start {
            (0, existing.time())
        } else {
            anyhow::bail!(
                "Selected panel {} has no start_time; provide --start",
                panel.id
            );
        };

        let start_date = base_date
            .checked_add_signed(Duration::days(start_day_offset))
            .context("Invalid date while applying --start")?;
        let new_start = NaiveDateTime::new(start_date, start_clock_time);

        let new_duration = if let Some(minutes) = duration_override {
            minutes
        } else if let Some((end_day_offset, end_clock_time)) = end_clock {
            let end_date = base_date
                .checked_add_signed(Duration::days(end_day_offset))
                .context("Invalid date while applying --end")?;
            let new_end = NaiveDateTime::new(end_date, end_clock_time);
            let delta = (new_end - new_start).num_minutes();
            if delta <= 0 {
                anyhow::bail!("--end must be after start time");
            }
            delta as u32
        } else {
            panel.effective_duration_minutes().unwrap_or(60)
        };

        let timing = schedule_core::data::time::TimeRange::new_scheduled(
            new_start,
            chrono::Duration::minutes(new_duration as i64),
        )
        .unwrap_or_else(|_| schedule_core::data::time::TimeRange::Unspecified);

        let new_room_ids = if let Some(rid) = room_id {
            vec![rid]
        } else {
            panel.room_ids.clone()
        };

        updates.push((
            idx,
            SessionScheduleState {
                room_ids: new_room_ids,
                timing,
            },
        ));
    }

    let mut ctx = sf.edit_context();
    for (idx, new_state) in updates {
        let reference = &selection.sessions[idx];
        ctx.reschedule_panel(&reference.panel_id, new_state);
    }

    Ok(())
}

fn execute_cancel(sf: &mut ScheduleFile, selection: &SelectionResult) -> Result<()> {
    if selection.sessions.is_empty() {
        anyhow::bail!("cancel requires at least one selected session");
    }

    let mut ctx = sf.edit_context();
    for reference in &selection.sessions {
        ctx.unschedule_panel(&reference.panel_id);
    }

    Ok(())
}

fn execute_remove_session(sf: &mut ScheduleFile, selection: &SelectionResult) -> Result<()> {
    if !selection.explicit_session_selector {
        anyhow::bail!(
            "remove-session requires a session-targeting --select (for example: --select panel id:<session-id>)"
        );
    }
    if selection.sessions.is_empty() {
        anyhow::bail!("No sessions matched --select");
    }

    let mut ctx = sf.edit_context();
    for reference in &selection.sessions {
        // Soft-delete: mark Deleted so update_xlsx writes the * prefix to the XLSX row.
        // post_save_cleanup removes Deleted panels from memory after the file is saved.
        ctx.soft_delete_panel(&reference.panel_id);
    }

    Ok(())
}

fn string_array(value: &Value, key: &str) -> Option<Vec<String>> {
    value.get(key).and_then(Value::as_array).map(|items| {
        items
            .iter()
            .filter_map(Value::as_str)
            .map(ToString::to_string)
            .collect()
    })
}

fn apply_panel_json_merge(ctx: &mut EditContext<'_>, panel_id: &str, patch: &Value) {
    if let Some(name) = patch.get("name").and_then(Value::as_str) {
        ctx.set_panel_name(panel_id, name);
    }
    if let Some(description) = patch.get("description").and_then(Value::as_str) {
        ctx.set_panel_field(
            panel_id,
            PanelField::Description,
            Some(description.to_string()),
        );
    }
    if let Some(note) = patch.get("note").and_then(Value::as_str) {
        ctx.set_panel_field(panel_id, PanelField::Note, Some(note.to_string()));
    }
    if let Some(cost) = patch.get("cost").and_then(Value::as_str) {
        ctx.set_panel_field(panel_id, PanelField::Cost, Some(cost.to_string()));
    }
    if let Some(capacity) = patch.get("capacity").and_then(Value::as_str) {
        ctx.set_panel_field(panel_id, PanelField::Capacity, Some(capacity.to_string()));
    }
    if let Some(presenters) = string_array(patch, "presenters") {
        ctx.set_panel_presenters(panel_id, presenters);
    }
}

fn apply_session_json_merge(
    ctx: &mut EditContext<'_>,
    panel_id: &str,
    patch: &Value,
    room_id_override: Option<u32>,
    null_room: bool,
) {
    if let Some(description) = patch.get("description").and_then(Value::as_str) {
        ctx.set_panel_field(
            panel_id,
            PanelField::Description,
            Some(description.to_string()),
        );
    }
    if let Some(note) = patch.get("note").and_then(Value::as_str) {
        ctx.set_panel_field(panel_id, PanelField::Note, Some(note.to_string()));
    }
    if let Some(av_note) = patch
        .get("avNote")
        .and_then(Value::as_str)
        .or_else(|| patch.get("avNotes").and_then(Value::as_str))
    {
        ctx.set_panel_field(panel_id, PanelField::AvNotes, Some(av_note.to_string()));
    }
    if patch.get("startTime").is_some() {
        ctx.set_panel_field(
            panel_id,
            PanelField::StartTime,
            patch
                .get("startTime")
                .and_then(Value::as_str)
                .map(ToString::to_string),
        );
    }
    if patch.get("endTime").is_some() {
        ctx.set_panel_field(
            panel_id,
            PanelField::EndTime,
            patch
                .get("endTime")
                .and_then(Value::as_str)
                .map(ToString::to_string),
        );
    }
    if let Some(duration) = patch.get("duration").and_then(Value::as_u64) {
        ctx.set_panel_duration(panel_id, duration as u32);
    }
    if let Some(room_id) = room_id_override {
        let state = ctx
            .schedule
            .panel_sets
            .values()
            .flat_map(|ps| ps.panels.iter())
            .find(|p| p.id == panel_id)
            .map(|p| SessionScheduleState {
                room_ids: vec![room_id],
                timing: p.timing.clone(),
            });
        if let Some(s) = state {
            ctx.reschedule_panel(panel_id, s);
        }
    } else if null_room {
        ctx.unschedule_panel(panel_id);
    }
    if let Some(presenters) = string_array(patch, "presenters") {
        ctx.set_panel_presenters(panel_id, presenters);
    }
    if let Some(add_presenters) = string_array(patch, "addPresenters") {
        for presenter_name in &add_presenters {
            ctx.add_presenter_to_panel(panel_id, presenter_name);
        }
    }
    if let Some(remove_presenters) = string_array(patch, "removePresenters") {
        for presenter_name in &remove_presenters {
            ctx.remove_presenter_from_panel(panel_id, presenter_name);
        }
    }
}

fn execute_update_from_json(
    sf: &mut ScheduleFile,
    selection: &SelectionResult,
    patch_path: &Path,
) -> Result<()> {
    if selection.panel_ids.is_empty() {
        anyhow::bail!("No selected records for update-from-json");
    }

    let patch_text = std::fs::read_to_string(patch_path)
        .with_context(|| format!("Failed to read patch file {}", patch_path.display()))?;
    let patch: Value = serde_json::from_str(&patch_text)
        .with_context(|| format!("Invalid JSON patch in {}", patch_path.display()))?;

    // Pre-compute room ID override before creating the mutable context.
    let room_id_override = patch
        .get("roomName")
        .and_then(Value::as_str)
        .or_else(|| patch.get("room").and_then(Value::as_str))
        .map(|name| find_room_id(&sf.schedule, name))
        .transpose()?;
    let null_room = patch.get("roomName").is_some_and(Value::is_null);

    let panel_ids: Vec<String> = selection.panel_ids.iter().cloned().collect();
    let sessions: Vec<SessionRef> = selection.sessions.clone();

    let mut ctx = sf.edit_context();
    for panel_id in &panel_ids {
        apply_panel_json_merge(&mut ctx, panel_id, &patch);
    }
    for reference in &sessions {
        apply_session_json_merge(
            &mut ctx,
            &reference.panel_id,
            &patch,
            room_id_override,
            null_room,
        );
    }

    Ok(())
}

fn execute_query(
    schedule: &Schedule,
    selection: &SelectionResult,
    fields: &[String],
    format: OutputFormat,
) -> Result<()> {
    let want_all = fields.iter().any(|f| f == "all");
    let want = |name: &str| -> bool {
        let norm_name = normalize_field_name(name);
        want_all || fields.iter().any(|f| normalize_field_name(f) == norm_name)
    };

    let room_lookup: HashMap<u32, String> = schedule
        .rooms
        .iter()
        .map(|r| (r.uid, r.short_name.clone()))
        .collect();

    let mut rows: Vec<serde_json::Map<String, Value>> = Vec::new();

    for reference in &selection.sessions {
        let Some(panel) = schedule
            .panel_sets
            .values()
            .flat_map(|ps| ps.panels.iter())
            .find(|p| p.id == reference.panel_id)
        else {
            continue;
        };

        let mut obj = serde_json::Map::new();

        if want("id") {
            obj.insert("id".to_string(), Value::String(panel.id.clone()));
        }
        if want("name") {
            obj.insert("name".to_string(), Value::String(panel.name.clone()));
        }
        if want("description") {
            let text = panel.description.as_deref().unwrap_or("").to_string();
            obj.insert(
                "description".to_string(),
                if text.is_empty() {
                    Value::Null
                } else {
                    Value::String(text)
                },
            );
        }
        if want("note") {
            let text = panel.note.as_deref().unwrap_or("").to_string();
            obj.insert(
                "note".to_string(),
                if text.is_empty() {
                    Value::Null
                } else {
                    Value::String(text)
                },
            );
        }
        if want("av-note") || want("av_note") {
            obj.insert(
                "av-note".to_string(),
                panel
                    .av_notes
                    .as_deref()
                    .map(|s| Value::String(s.to_string()))
                    .unwrap_or(Value::Null),
            );
        }
        if want("prereq") {
            let text = panel.prereq.as_deref().unwrap_or("").to_string();
            obj.insert(
                "prereq".to_string(),
                if text.is_empty() {
                    Value::Null
                } else {
                    Value::String(text)
                },
            );
        }
        if want("room") {
            let rooms: Vec<Value> = panel
                .room_ids
                .iter()
                .map(|&id| {
                    room_lookup
                        .get(&id)
                        .map(|n| Value::String(n.clone()))
                        .unwrap_or_else(|| Value::Number(id.into()))
                })
                .collect();
            obj.insert("room".to_string(), Value::Array(rooms));
        }
        if want("start") {
            obj.insert(
                "start".to_string(),
                panel
                    .start_time_str()
                    .map(Value::String)
                    .unwrap_or(Value::Null),
            );
        }
        if want("end") {
            obj.insert(
                "end".to_string(),
                panel
                    .end_time_str()
                    .map(Value::String)
                    .unwrap_or(Value::Null),
            );
        }
        if want("duration") {
            obj.insert(
                "duration".to_string(),
                Value::Number(panel.effective_duration_minutes().unwrap_or(0).into()),
            );
        }
        if want("type") {
            obj.insert(
                "type".to_string(),
                panel
                    .panel_type
                    .as_deref()
                    .map(|s| Value::String(s.to_string()))
                    .unwrap_or(Value::Null),
            );
        }
        if want("cost") {
            obj.insert(
                "cost".to_string(),
                panel
                    .cost
                    .as_deref()
                    .map(|s| Value::String(s.to_string()))
                    .unwrap_or(Value::Null),
            );
        }
        if want("capacity") {
            obj.insert(
                "capacity".to_string(),
                panel
                    .capacity
                    .as_deref()
                    .map(|s| Value::String(s.to_string()))
                    .unwrap_or(Value::Null),
            );
        }
        if want("difficulty") {
            obj.insert(
                "difficulty".to_string(),
                panel
                    .difficulty
                    .as_deref()
                    .map(|s| Value::String(s.to_string()))
                    .unwrap_or(Value::Null),
            );
        }
        if want("presenters") {
            let mut all: Vec<String> = Vec::new();
            for name in &panel.credited_presenters {
                if !all.contains(name) {
                    all.push(name.clone());
                }
            }
            for name in &panel.uncredited_presenters {
                let tagged = format!("{name}(*)");
                if !all.iter().any(|a| a == name || a == &tagged) {
                    all.push(tagged);
                }
            }
            obj.insert(
                "presenters".to_string(),
                Value::Array(all.into_iter().map(Value::String).collect()),
            );
        }
        if want("credited") {
            obj.insert(
                "credited".to_string(),
                Value::Array(
                    panel
                        .credited_presenters
                        .iter()
                        .cloned()
                        .map(Value::String)
                        .collect(),
                ),
            );
        }
        if want("uncredited") {
            obj.insert(
                "uncredited".to_string(),
                Value::Array(
                    panel
                        .uncredited_presenters
                        .iter()
                        .cloned()
                        .map(Value::String)
                        .collect(),
                ),
            );
        }
        if want("metadata") {
            let mut meta_obj = serde_json::Map::new();
            for (k, v) in &panel.metadata {
                let json_val = match v {
                    ExtraValue::String(s) => Value::String(s.clone()),
                    ExtraValue::Formula(fv) => {
                        let mut m = serde_json::Map::new();
                        m.insert("formula".to_string(), Value::String(fv.formula.clone()));
                        m.insert("value".to_string(), Value::String(fv.value.clone()));
                        Value::Object(m)
                    }
                };
                meta_obj.insert(k.clone(), json_val);
            }
            obj.insert("metadata".to_string(), Value::Object(meta_obj));
        }
        if want("change-state") || want("change_state") {
            obj.insert(
                "change-state".to_string(),
                Value::String(format!("{:?}", panel.change_state)),
            );
        }

        rows.push(obj);
    }

    // Rooms
    for &room_id in &selection.room_ids {
        let Some(room) = schedule.rooms.iter().find(|r| r.uid == room_id) else {
            continue;
        };
        let mut obj = serde_json::Map::new();
        obj.insert("_entity".to_string(), Value::String("room".to_string()));
        if want("id") {
            obj.insert("id".to_string(), Value::Number(room.uid.into()));
        }
        if want("name") || want_all {
            obj.insert("name".to_string(), Value::String(room.short_name.clone()));
        }
        if want("long-name") || want_all {
            obj.insert(
                "long-name".to_string(),
                if room.long_name.is_empty() {
                    Value::Null
                } else {
                    Value::String(room.long_name.clone())
                },
            );
        }
        if want("hotel-room") || want_all {
            obj.insert(
                "hotel-room".to_string(),
                if room.hotel_room.is_empty() {
                    Value::Null
                } else {
                    Value::String(room.hotel_room.clone())
                },
            );
        }
        if want("sort-key") || want_all {
            obj.insert("sort-key".to_string(), Value::Number(room.sort_key.into()));
        }
        if want("metadata") {
            let mut meta_obj = serde_json::Map::new();
            if let Some(meta) = &room.metadata {
                for (k, v) in meta.iter() {
                    let json_val = match v {
                        ExtraValue::String(s) => Value::String(s.clone()),
                        ExtraValue::Formula(fv) => {
                            let mut m = serde_json::Map::new();
                            m.insert("formula".to_string(), Value::String(fv.formula.clone()));
                            m.insert("value".to_string(), Value::String(fv.value.clone()));
                            Value::Object(m)
                        }
                    };
                    meta_obj.insert(k.clone(), json_val);
                }
            }
            obj.insert("metadata".to_string(), Value::Object(meta_obj));
        }
        if want("change-state") || want("change_state") {
            obj.insert(
                "change-state".to_string(),
                Value::String(format!("{:?}", room.change_state)),
            );
        }
        rows.push(obj);
    }

    // Panel types
    for prefix in &selection.panel_type_prefixes {
        let Some(pt) = schedule.panel_types.get(prefix) else {
            continue;
        };
        let mut obj = serde_json::Map::new();
        obj.insert(
            "_entity".to_string(),
            Value::String("panel-type".to_string()),
        );
        if want("id") || want("prefix") {
            obj.insert("prefix".to_string(), Value::String(pt.prefix.clone()));
        }
        if want("name") || want("kind") || want_all {
            obj.insert("kind".to_string(), Value::String(pt.kind.clone()));
        }
        if want("color") || want_all {
            obj.insert(
                "color".to_string(),
                pt.color()
                    .map(|s| Value::String(s.to_string()))
                    .unwrap_or(Value::Null),
            );
        }
        if want("bw-color") || want("bw_color") || want_all {
            obj.insert(
                "bw-color".to_string(),
                pt.bw_color()
                    .map(|s| Value::String(s.to_string()))
                    .unwrap_or(Value::Null),
            );
        }
        if want("metadata") {
            let mut meta_obj = serde_json::Map::new();
            if let Some(meta) = &pt.metadata {
                for (k, v) in meta.iter() {
                    let json_val = match v {
                        ExtraValue::String(s) => Value::String(s.clone()),
                        ExtraValue::Formula(fv) => {
                            let mut m = serde_json::Map::new();
                            m.insert("formula".to_string(), Value::String(fv.formula.clone()));
                            m.insert("value".to_string(), Value::String(fv.value.clone()));
                            Value::Object(m)
                        }
                    };
                    meta_obj.insert(k.clone(), json_val);
                }
            }
            obj.insert("metadata".to_string(), Value::Object(meta_obj));
        }
        if want("change-state") || want("change_state") {
            obj.insert(
                "change-state".to_string(),
                Value::String(format!("{:?}", pt.change_state)),
            );
        }
        rows.push(obj);
    }

    match format {
        OutputFormat::Human => {
            if rows.is_empty() {
                println!("(no matches)");
            }
            for row in &rows {
                let entity = row
                    .get("_entity")
                    .and_then(Value::as_str)
                    .unwrap_or("session");
                let header = match entity {
                    "room" => {
                        let name = row.get("name").and_then(Value::as_str).unwrap_or("?");
                        format!("[room] {name}")
                    }
                    "panel-type" => {
                        let prefix = row.get("prefix").and_then(Value::as_str).unwrap_or("?");
                        let kind = row.get("kind").and_then(Value::as_str).unwrap_or("");
                        if kind.is_empty() {
                            format!("[type] {prefix}")
                        } else {
                            format!("[type] {prefix} | {kind}")
                        }
                    }
                    _ => {
                        let id = row.get("id").and_then(Value::as_str).unwrap_or("?");
                        let name = row.get("name").and_then(Value::as_str).unwrap_or("");
                        if name.is_empty() {
                            id.to_string()
                        } else {
                            format!("{id} | {name}")
                        }
                    }
                };
                println!("{header}");
                for (k, v) in row {
                    if matches!(k.as_str(), "id" | "name" | "prefix" | "kind" | "_entity") {
                        continue;
                    }
                    match v {
                        Value::Null => println!("  {k}: <null>"),
                        Value::Array(arr) => {
                            if arr.is_empty() {
                                println!("  {k}: []");
                            } else {
                                let items: Vec<String> = arr
                                    .iter()
                                    .map(|v| {
                                        v.as_str()
                                            .map(String::from)
                                            .unwrap_or_else(|| v.to_string())
                                    })
                                    .collect();
                                println!("  {k}: {}", items.join(", "));
                            }
                        }
                        Value::Object(map) => {
                            if map.is_empty() {
                                println!("  {k}: {{}}");
                            } else {
                                println!("  {k}:");
                                for (mk, mv) in map {
                                    match mv {
                                        Value::Object(inner) => {
                                            let formula = inner
                                                .get("formula")
                                                .and_then(Value::as_str)
                                                .unwrap_or("");
                                            let val = inner
                                                .get("value")
                                                .and_then(Value::as_str)
                                                .unwrap_or("");
                                            println!("    {mk}: ={formula} [→ {val}]");
                                        }
                                        other => {
                                            let s = other
                                                .as_str()
                                                .map(String::from)
                                                .unwrap_or_else(|| other.to_string());
                                            println!("    {mk}: {s}");
                                        }
                                    }
                                }
                            }
                        }
                        other => {
                            let s = other
                                .as_str()
                                .map(String::from)
                                .unwrap_or_else(|| other.to_string());
                            println!("  {k}: {s}");
                        }
                    }
                }
            }
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&rows)?);
        }
    }

    Ok(())
}

fn execute_set_metadata(
    sf: &mut ScheduleFile,
    selection: &SelectionResult,
    key: &str,
    value: &str,
) -> Result<()> {
    let has_targets = !selection.sessions.is_empty()
        || !selection.room_ids.is_empty()
        || !selection.panel_type_prefixes.is_empty();
    if !has_targets {
        anyhow::bail!("No entities matched for set-metadata");
    }
    let mut ctx = sf.edit_context();
    for reference in &selection.sessions {
        ctx.set_panel_metadata(
            &reference.panel_id,
            key,
            ExtraValue::String(value.to_string()),
        );
    }
    for &room_id in &selection.room_ids {
        ctx.set_room_metadata(room_id, key, ExtraValue::String(value.to_string()));
    }
    for prefix in &selection.panel_type_prefixes {
        ctx.set_panel_type_metadata(prefix, key, ExtraValue::String(value.to_string()));
    }
    Ok(())
}

fn execute_clear_metadata(
    sf: &mut ScheduleFile,
    selection: &SelectionResult,
    key: &str,
) -> Result<()> {
    let has_targets = !selection.sessions.is_empty()
        || !selection.room_ids.is_empty()
        || !selection.panel_type_prefixes.is_empty();
    if !has_targets {
        anyhow::bail!("No entities matched for clear-metadata");
    }
    let mut ctx = sf.edit_context();
    for reference in &selection.sessions {
        ctx.clear_panel_metadata(&reference.panel_id, key);
    }
    for &room_id in &selection.room_ids {
        ctx.clear_room_metadata(room_id, key);
    }
    for prefix in &selection.panel_type_prefixes {
        ctx.clear_panel_type_metadata(prefix, key);
    }
    Ok(())
}

fn execute_undo(sf: &mut ScheduleFile) -> Result<()> {
    if sf.edit_context().undo() {
        eprintln!("Undo applied.");
    } else {
        eprintln!("Nothing to undo.");
    }
    Ok(())
}

fn execute_redo(sf: &mut ScheduleFile) -> Result<()> {
    if sf.edit_context().redo() {
        eprintln!("Redo applied.");
    } else {
        eprintln!("Nothing to redo.");
    }
    Ok(())
}

fn execute_show_history(sf: &ScheduleFile, format: OutputFormat) {
    let undo_count = sf.history.undo_count();
    let redo_count = sf.history.redo_count();
    match format {
        OutputFormat::Json => {
            let obj = serde_json::json!({
                "undoCount": undo_count,
                "redoCount": redo_count,
            });
            println!("{}", serde_json::to_string_pretty(&obj).unwrap());
        }
        OutputFormat::Human => {
            println!("Undo stack: {undo_count} item(s)");
            println!("Redo stack: {redo_count} item(s)");
        }
    }
}

fn execute_stage(sf: &mut ScheduleFile, stage: &Stage, format: OutputFormat) -> Result<()> {
    let selection = collect_selection(&sf.schedule, &stage.selectors);

    match &stage.command {
        Command::List(target) => match target {
            ListTarget::Panels => list_panels(&sf.schedule, &selection, format)?,
            ListTarget::Rooms => list_rooms(&sf.schedule, &selection, format)?,
            ListTarget::Presenters => list_presenters(&sf.schedule, &selection, format)?,
        },
        Command::Set { field, value } => execute_set(sf, &selection, field, value)?,
        Command::AddPresenter { name } => apply_presenter_change(sf, &selection, name, true)?,
        Command::RemovePresenter { name } => apply_presenter_change(sf, &selection, name, false)?,
        Command::Reschedule {
            room_name,
            day,
            start_time,
            end_time,
            duration,
        } => execute_reschedule(
            sf,
            &selection,
            room_name.as_deref(),
            day.as_deref(),
            start_time.as_deref(),
            end_time.as_deref(),
            duration.as_deref(),
        )?,
        Command::Cancel => execute_cancel(sf, &selection)?,
        Command::RemoveSession => execute_remove_session(sf, &selection)?,
        Command::UpdateFromJson { path } => execute_update_from_json(sf, &selection, path)?,
        Command::Query { fields } => execute_query(&sf.schedule, &selection, fields, format)?,
        Command::SetMetadata { key, value } => execute_set_metadata(sf, &selection, key, value)?,
        Command::ClearMetadata { key } => execute_clear_metadata(sf, &selection, key)?,
        Command::Undo => execute_undo(sf)?,
        Command::Redo => execute_redo(sf)?,
        Command::ShowHistory => execute_show_history(sf, format),
    }

    Ok(())
}

fn apply_modification_metadata(schedule: &mut Schedule) {
    let now = time::format_storage_ts(Utc::now());
    let username = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| "Unknown User".to_string());

    schedule.meta.modified = Some(now);
    schedule.meta.last_modified_by = Some(username);
    schedule.meta.generator = Some(format!("cosam-modify {}", env!("CARGO_PKG_VERSION")));
}

fn main() {
    let cli = match parse_args() {
        Ok(args) => args,
        Err(error) => {
            eprintln!("{error}");
            print_usage();
            std::process::exit(1);
        }
    };

    let mut sf = match schedule_core::xlsx::load_auto(&cli.file, &cli.import_options) {
        Ok(sf) => sf,
        Err(error) => {
            eprintln!("Failed to load {}: {error}", cli.file.display());
            std::process::exit(1);
        }
    };

    for stage in &cli.stages {
        if let Err(error) = execute_stage(&mut sf, stage, cli.format) {
            eprintln!("Error executing stage: {error}");
            std::process::exit(1);
        }
    }

    apply_modification_metadata(&mut sf.schedule);
    if let Err(error) = schedule_core::xlsx::save_auto(&mut sf, &cli.file) {
        eprintln!("Failed to save {}: {error}", cli.file.display());
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_stage_with_selectors() {
        let tokens = vec![
            "--select".to_string(),
            "panel".to_string(),
            "name:Armor".to_string(),
            "set".to_string(),
            "note".to_string(),
            "Bring foam".to_string(),
        ];
        let stage = parse_stage(&tokens).expect("stage should parse");
        assert_eq!(stage.selectors.queries.len(), 1);
        assert_eq!(stage.selectors.queries[0].scope, SelectScope::Panel);
        assert_eq!(stage.selectors.queries[0].field, Some(SelectField::Name));
        assert_eq!(stage.selectors.queries[0].clauses.len(), 1);
        assert_eq!(stage.selectors.queries[0].clauses[0].value, "Armor");
        match stage.command {
            Command::Set {
                field: SetField::Note,
                value,
            } => assert_eq!(value, "Bring foam"),
            _ => panic!("expected set note command"),
        }
    }

    #[test]
    fn parse_stage_with_smart_and_panel_type_selectors() {
        let tokens = vec![
            "--select".to_string(),
            "Salon".to_string(),
            "--select".to_string(),
            "panel".to_string(),
            "type:workshop".to_string(),
            "list".to_string(),
            "panels".to_string(),
        ];
        let stage = parse_stage(&tokens).expect("stage should parse");
        assert_eq!(stage.selectors.queries.len(), 2);
        assert_eq!(stage.selectors.queries[0].scope, SelectScope::Panel);
        assert!(stage.selectors.queries[0].field.is_none());
        assert_eq!(stage.selectors.queries[0].clauses.len(), 1);
        assert_eq!(stage.selectors.queries[0].clauses[0].value, "Salon");
        assert_eq!(stage.selectors.queries[1].scope, SelectScope::Panel);
        assert_eq!(stage.selectors.queries[1].field, Some(SelectField::Type));
        assert_eq!(stage.selectors.queries[1].clauses[0].value, "workshop");
        match stage.command {
            Command::List(ListTarget::Panels) => {}
            _ => panic!("expected list panels command"),
        }
    }

    #[test]
    fn parse_stage_with_select_scope_field_and_relationships() {
        let tokens = vec![
            "--select".to_string(),
            "panel".to_string(),
            "room:~Salon,=Main Stage".to_string(),
            "list".to_string(),
            "panels".to_string(),
        ];
        let stage = parse_stage(&tokens).expect("stage should parse");
        assert_eq!(stage.selectors.queries.len(), 1);
        let query = &stage.selectors.queries[0];
        assert_eq!(query.scope, SelectScope::Panel);
        assert_eq!(query.field, Some(SelectField::Room));
        assert_eq!(query.clauses.len(), 2);
        assert_eq!(query.clauses[0].relationship, SelectRelationship::Contains);
        assert_eq!(query.clauses[0].value, "Salon");
        assert_eq!(query.clauses[1].relationship, SelectRelationship::Equal);
        assert_eq!(query.clauses[1].value, "Main Stage");
    }

    #[test]
    fn parse_stage_with_panel_presenter_field_and_default_list_target() {
        let tokens = vec![
            "--select".to_string(),
            "panel".to_string(),
            "presenter:Yaya".to_string(),
            "list".to_string(),
        ];
        let stage = parse_stage(&tokens).expect("stage should parse");
        assert_eq!(stage.selectors.queries.len(), 1);
        let query = &stage.selectors.queries[0];
        assert_eq!(query.scope, SelectScope::Panel);
        assert_eq!(query.field, Some(SelectField::Presenter));
        assert_eq!(query.clauses.len(), 1);
        assert_eq!(query.clauses[0].value, "Yaya");
        match stage.command {
            Command::List(ListTarget::Panels) => {}
            _ => panic!("expected list panels command"),
        }
    }

    #[test]
    fn parse_stage_with_presenter_scope_name_field_and_default_list_target() {
        let tokens = vec![
            "--select".to_string(),
            "presenter".to_string(),
            "name:Yaya".to_string(),
            "list".to_string(),
        ];
        let stage = parse_stage(&tokens).expect("stage should parse");
        assert_eq!(stage.selectors.queries.len(), 1);
        let query = &stage.selectors.queries[0];
        assert_eq!(query.scope, SelectScope::Presenter);
        assert_eq!(query.field, Some(SelectField::Name));
        assert_eq!(query.clauses.len(), 1);
        assert_eq!(query.clauses[0].value, "Yaya");
        match stage.command {
            Command::List(ListTarget::Panels) => {}
            _ => panic!("expected list panels command"),
        }
    }

    #[test]
    fn parse_reschedule_stage() {
        let tokens = vec![
            "--select".to_string(),
            "room".to_string(),
            "name:Main".to_string(),
            "reschedule".to_string(),
            "-room".to_string(),
            "Main Stage".to_string(),
            "--day".to_string(),
            "Friday".to_string(),
            "--start".to_string(),
            "10:00".to_string(),
            "--duration".to_string(),
            "1:15".to_string(),
        ];
        let stage = parse_stage(&tokens).expect("stage should parse");
        match stage.command {
            Command::Reschedule {
                room_name,
                day,
                start_time,
                end_time,
                duration,
            } => {
                assert_eq!(room_name.as_deref(), Some("Main Stage"));
                assert_eq!(day.as_deref(), Some("Friday"));
                assert_eq!(start_time.as_deref(), Some("10:00"));
                assert_eq!(end_time, None);
                assert_eq!(duration.as_deref(), Some("1:15"));
            }
            _ => panic!("expected reschedule command"),
        }
    }

    #[test]
    fn panel_json_merge_is_partial() {
        use schedule_core::data::panel_set::PanelSet;
        let mut schedule = Schedule::default();
        let mut panel = schedule_core::data::Panel::new("test-1", "test-1");
        panel.name = "Original Name".to_string();
        panel.note = Some("keep me".to_string());
        let mut ps = PanelSet::new("test-1");
        ps.panels.push(panel);
        schedule.panel_sets.insert("test-1".to_string(), ps);

        let patch: Value = serde_json::json!({
            "description": "new description"
        });

        {
            let mut ctx = EditContext::import(&mut schedule);
            apply_panel_json_merge(&mut ctx, "test-1", &patch);
        }

        let panel = schedule.panel_sets["test-1"].panels.first().unwrap();
        assert_eq!(panel.name, "Original Name");
        assert_eq!(panel.note.as_deref(), Some("keep me"));
        assert_eq!(panel.description.as_deref(), Some("new description"));
    }

    // --- helpers for undo/redo tests ---

    fn create_test_schedule_file() -> ScheduleFile {
        use indexmap::IndexMap;
        use schedule_core::data::panel::Panel;
        use schedule_core::data::panel_set::PanelSet;
        use schedule_core::data::relationship::RelationshipManager;
        use schedule_core::data::room::Room;
        use schedule_core::data::schedule::Meta;
        use schedule_core::data::source_info::ChangeState;

        let mut panel_sets: IndexMap<String, PanelSet> = IndexMap::new();
        let mut panel = Panel::new("test-panel-1", "test-panel-1");
        panel.name = "Armor 101".to_string();
        panel.description = Some("Original description".to_string());
        panel.note = Some("Original note".to_string());
        panel.session_num = Some(1);
        panel.room_ids = vec![10];
        panel.set_start_time_from_str("2026-07-10T10:00:00");
        panel.set_end_time_from_str("2026-07-10T11:00:00");
        panel.set_duration_minutes(60);
        panel.credited_presenters = vec!["Alice".to_string(), "Bob".to_string()];
        let mut ps = PanelSet::new("test-panel-1");
        ps.panels.push(panel);
        panel_sets.insert("test-panel-1".to_string(), ps);

        let rooms = vec![
            Room {
                uid: 10,
                short_name: "Main".to_string(),
                long_name: "Main Events".to_string(),
                hotel_room: "Salon F/G".to_string(),
                sort_key: 1,
                is_break: false,
                metadata: None,
                source: None,
                change_state: ChangeState::Unchanged,
            },
            Room {
                uid: 20,
                short_name: "Workshop 1".to_string(),
                long_name: "Workshop Room 1".to_string(),
                hotel_room: "Salon A".to_string(),
                sort_key: 2,
                is_break: false,
                metadata: None,
                source: None,
                change_state: ChangeState::Unchanged,
            },
        ];

        let schedule = Schedule {
            conflicts: Vec::new(),
            meta: Meta {
                title: "Test Schedule".to_string(),
                generated: "2026-01-01T00:00:00Z".to_string(),
                version: Some(8),
                variant: Some("full".to_string()),
                generator: None,
                start_time: None,
                end_time: None,
                next_presenter_id: None,
                creator: None,
                last_modified_by: None,
                modified: None,
            },
            timeline: Vec::new(),
            panel_sets,
            rooms,
            panel_types: IndexMap::new(),
            presenters: Vec::new(),
            imported_sheets: Default::default(),
            relationships: RelationshipManager::new(),
        };

        ScheduleFile::new(schedule)
    }

    fn select_panel(sf: &ScheduleFile, panel_id: &str) -> SelectionResult {
        let selectors = Selectors {
            queries: vec![SelectQuery {
                scope: SelectScope::Panel,
                field: Some(SelectField::Id),
                clauses: vec![SelectClause {
                    relationship: SelectRelationship::Equal,
                    value: panel_id.to_string(),
                }],
            }],
        };
        collect_selection(&sf.schedule, &selectors)
    }

    // --- undo/redo unit tests ---

    #[test]
    fn test_undo_no_history() {
        let mut sf = create_test_schedule_file();
        let result = sf.edit_context().undo();
        assert!(!result, "undo on empty history should return false");
    }

    #[test]
    fn test_redo_no_history() {
        let mut sf = create_test_schedule_file();
        let result = sf.edit_context().redo();
        assert!(!result, "redo on empty history should return false");
    }

    #[test]
    fn test_undo_single_set_description() {
        let mut sf = create_test_schedule_file();
        let selection = select_panel(&sf, "test-panel-1");

        execute_set(&mut sf, &selection, &SetField::Description, "New desc").unwrap();
        assert_eq!(
            sf.schedule.panel_sets["test-panel-1"].panels[0]
                .description
                .as_deref(),
            Some("New desc")
        );

        sf.edit_context().undo();
        assert_eq!(
            sf.schedule.panel_sets["test-panel-1"].panels[0]
                .description
                .as_deref(),
            Some("Original description")
        );
    }

    #[test]
    fn test_undo_multiple_commands() {
        let mut sf = create_test_schedule_file();
        let selection = select_panel(&sf, "test-panel-1");

        execute_set(&mut sf, &selection, &SetField::Description, "Desc 2").unwrap();
        execute_set(&mut sf, &selection, &SetField::Note, "Note 2").unwrap();

        assert_eq!(
            sf.schedule.panel_sets["test-panel-1"].panels[0]
                .description
                .as_deref(),
            Some("Desc 2")
        );
        assert_eq!(
            sf.schedule.panel_sets["test-panel-1"].panels[0]
                .note
                .as_deref(),
            Some("Note 2")
        );

        sf.edit_context().undo();
        assert_eq!(
            sf.schedule.panel_sets["test-panel-1"].panels[0]
                .note
                .as_deref(),
            Some("Original note"),
            "first undo should restore note"
        );
        assert_eq!(
            sf.schedule.panel_sets["test-panel-1"].panels[0]
                .description
                .as_deref(),
            Some("Desc 2"),
            "description should still be changed after undoing note"
        );

        sf.edit_context().undo();
        assert_eq!(
            sf.schedule.panel_sets["test-panel-1"].panels[0]
                .description
                .as_deref(),
            Some("Original description"),
            "second undo should restore description"
        );
    }

    #[test]
    fn test_redo_after_undo() {
        let mut sf = create_test_schedule_file();
        let selection = select_panel(&sf, "test-panel-1");

        execute_set(&mut sf, &selection, &SetField::Description, "Changed").unwrap();
        sf.edit_context().undo();
        assert_eq!(
            sf.schedule.panel_sets["test-panel-1"].panels[0]
                .description
                .as_deref(),
            Some("Original description")
        );

        sf.edit_context().redo();
        assert_eq!(
            sf.schedule.panel_sets["test-panel-1"].panels[0]
                .description
                .as_deref(),
            Some("Changed"),
            "redo should re-apply the change"
        );
    }

    #[test]
    fn test_show_history_empty() {
        let sf = create_test_schedule_file();
        assert_eq!(sf.history.undo_count(), 0);
        assert_eq!(sf.history.redo_count(), 0);
    }

    #[test]
    fn test_show_history_after_commands() {
        let mut sf = create_test_schedule_file();
        let selection = select_panel(&sf, "test-panel-1");

        execute_set(&mut sf, &selection, &SetField::Description, "D1").unwrap();
        execute_set(&mut sf, &selection, &SetField::Note, "N1").unwrap();
        assert_eq!(sf.history.undo_count(), 2);
        assert_eq!(sf.history.redo_count(), 0);

        sf.edit_context().undo();
        assert_eq!(sf.history.undo_count(), 1);
        assert_eq!(sf.history.redo_count(), 1);
    }

    #[test]
    fn test_undo_set_metadata() {
        let mut sf = create_test_schedule_file();
        let selection = select_panel(&sf, "test-panel-1");

        execute_set_metadata(&mut sf, &selection, "ThemeColor", "#FF0000").unwrap();
        {
            let session = &sf.schedule.panel_sets["test-panel-1"].panels[0];
            assert!(session.metadata.contains_key("ThemeColor"));
        }

        sf.edit_context().undo();
        {
            let session = &sf.schedule.panel_sets["test-panel-1"].panels[0];
            assert!(
                !session.metadata.contains_key("ThemeColor"),
                "undo should remove metadata key"
            );
        }
    }

    #[test]
    fn test_undo_clear_metadata() {
        let mut sf = create_test_schedule_file();
        let selection = select_panel(&sf, "test-panel-1");

        execute_set_metadata(&mut sf, &selection, "Color", "red").unwrap();
        execute_clear_metadata(&mut sf, &selection, "Color").unwrap();
        {
            let session = &sf.schedule.panel_sets["test-panel-1"].panels[0];
            assert!(!session.metadata.contains_key("Color"));
        }

        sf.edit_context().undo();
        {
            let session = &sf.schedule.panel_sets["test-panel-1"].panels[0];
            assert!(
                session.metadata.contains_key("Color"),
                "undo of clear-metadata should restore the key"
            );
        }
    }

    #[test]
    fn test_undo_add_presenter() {
        let mut sf = create_test_schedule_file();
        let selection = select_panel(&sf, "test-panel-1");

        assert_eq!(
            sf.schedule.panel_sets["test-panel-1"].panels[0]
                .credited_presenters
                .len(),
            2
        );

        apply_presenter_change(&mut sf, &selection, "Charlie", true).unwrap();
        assert_eq!(
            sf.schedule.panel_sets["test-panel-1"].panels[0]
                .credited_presenters
                .len(),
            3
        );

        sf.edit_context().undo();
        assert_eq!(
            sf.schedule.panel_sets["test-panel-1"].panels[0]
                .credited_presenters
                .len(),
            2,
            "undo should remove the added presenter"
        );
    }

    #[test]
    fn test_undo_remove_presenter() {
        let mut sf = create_test_schedule_file();
        let selection = select_panel(&sf, "test-panel-1");

        apply_presenter_change(&mut sf, &selection, "Alice", false).unwrap();
        assert_eq!(
            sf.schedule.panel_sets["test-panel-1"].panels[0]
                .credited_presenters
                .len(),
            1
        );

        sf.edit_context().undo();
        let presenters = &sf.schedule.panel_sets["test-panel-1"].panels[0].credited_presenters;
        assert_eq!(presenters.len(), 2, "undo should restore removed presenter");
        assert!(
            presenters.iter().any(|p| p == "Alice"),
            "Alice should be restored"
        );
    }

    #[test]
    fn test_undo_reschedule() {
        let mut sf = create_test_schedule_file();
        let selection = select_panel(&sf, "test-panel-1");

        let orig_start = sf.schedule.panel_sets["test-panel-1"].panels[0]
            .timing
            .start_time();
        let orig_room_ids = sf.schedule.panel_sets["test-panel-1"].panels[0]
            .room_ids
            .clone();

        execute_reschedule(
            &mut sf,
            &selection,
            Some("Workshop 1"),
            None,
            Some("14:00"),
            None,
            Some("90"),
        )
        .unwrap();

        let session = &sf.schedule.panel_sets["test-panel-1"].panels[0];
        assert_eq!(session.room_ids, vec![20]);
        assert_eq!(session.effective_duration_minutes().unwrap_or(0), 90);

        sf.edit_context().undo();
        let session = &sf.schedule.panel_sets["test-panel-1"].panels[0];
        assert_eq!(session.timing.start_time(), orig_start);
        assert_eq!(session.room_ids, orig_room_ids);
        assert_eq!(session.effective_duration_minutes().unwrap_or(0), 60);
    }

    #[test]
    fn test_undo_cancel() {
        let mut sf = create_test_schedule_file();
        let selection = select_panel(&sf, "test-panel-1");

        let orig_start = sf.schedule.panel_sets["test-panel-1"].panels[0]
            .timing
            .start_time();
        let orig_room_ids = sf.schedule.panel_sets["test-panel-1"].panels[0]
            .room_ids
            .clone();

        execute_cancel(&mut sf, &selection).unwrap();
        let session = &sf.schedule.panel_sets["test-panel-1"].panels[0];
        assert!(session.room_ids.is_empty());
        assert!(session.timing.start_time().is_none());

        sf.edit_context().undo();
        let session = &sf.schedule.panel_sets["test-panel-1"].panels[0];
        assert_eq!(session.timing.start_time(), orig_start);
        assert_eq!(session.room_ids, orig_room_ids);
    }

    #[test]
    fn test_undo_set_av_note() {
        let mut sf = create_test_schedule_file();
        let selection = select_panel(&sf, "test-panel-1");

        execute_set(&mut sf, &selection, &SetField::AvNote, "Needs mic").unwrap();
        assert_eq!(
            sf.schedule.panel_sets["test-panel-1"].panels[0]
                .av_notes
                .as_deref(),
            Some("Needs mic")
        );

        sf.edit_context().undo();
        assert_eq!(
            sf.schedule.panel_sets["test-panel-1"].panels[0].av_notes, None,
            "undo should restore original av_notes"
        );
    }
}
