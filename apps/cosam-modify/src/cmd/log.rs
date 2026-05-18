/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! `log` command — show CRDT commit history.

use anyhow::Result;
use schedule_core::edit::context::EditContext;
use schedule_core::schedule::ChangeLogEntry;

use crate::args::OutputFormat;

pub fn run(ctx: &mut EditContext, format: &OutputFormat, verbose: bool) -> Result<()> {
    // Use detailed log when verbose mode is enabled
    let entries = if verbose {
        ctx.schedule_mut().change_log_detailed()
    } else {
        ctx.schedule_mut().change_log()
    };

    match format {
        OutputFormat::Json => print_json(&entries),
        OutputFormat::Toml => print_toml(&entries),
        OutputFormat::Text => print_text(&entries, verbose),
    }

    Ok(())
}

// ── Text output ───────────────────────────────────────────────────────────────

fn print_text(entries: &[ChangeLogEntry], verbose: bool) {
    if entries.is_empty() {
        println!("No commits in document history.");
        return;
    }

    // Group: accumulate raw (message-less) commits until a marker appears,
    // then print the marker with the accumulated op count.  A trailing group
    // of raw commits (no marker yet) is printed as an "uncommitted ops" line.
    // In verbose mode, show each raw commit individually with its details.
    let mut pending_ops: usize = 0;
    let mut pending_commits: usize = 0;

    for entry in entries {
        match &entry.message {
            Some(msg) => {
                // Flush any pending raw commits as context for this marker.
                if pending_commits > 0 {
                    println!(
                        "  ({pending_commits} commit{}, {pending_ops} op{})",
                        if pending_commits == 1 { "" } else { "s" },
                        if pending_ops == 1 { "" } else { "s" },
                    );
                    pending_ops = 0;
                    pending_commits = 0;
                }
                let ts = format_timestamp(entry.timestamp_secs);
                println!("commit {}  {}  {}", entry.hash_short, ts, msg);
                if verbose {
                    print_change_details(entry);
                }
            }
            None => {
                if verbose {
                    // In verbose mode, print each raw commit individually
                    let ts = format_timestamp(entry.timestamp_secs);
                    println!("commit {}  {}  ({} ops)", entry.hash_short, ts, entry.ops);
                    print_change_details(entry);
                } else {
                    // In normal mode, accumulate
                    pending_ops += entry.ops;
                    pending_commits += 1;
                }
            }
        }
    }

    // Any remaining raw commits after the last marker.
    if pending_commits > 0 {
        println!(
            "  ({pending_commits} commit{}, {pending_ops} op{} — no marker yet)",
            if pending_commits == 1 { "" } else { "s" },
            if pending_ops == 1 { "" } else { "s" },
        );
    }
}

/// Print detailed change information for verbose mode.
fn print_change_details(entry: &ChangeLogEntry) {
    if entry.ops == 0 {
        println!("    (marker commit, no ops)");
        return;
    }

    // Show detailed operations if available
    if let Some(ops) = &entry.detailed_ops {
        for op in ops.iter().take(20) {
            // Limit to 20 operations to avoid overwhelming output
            println!("    - {}", op);
        }
        if ops.len() > 20 {
            println!("    ... and {} more operations", ops.len() - 20);
        }
    } else {
        println!("    ({} ops - use --verbose for details)", entry.ops);
    }
}

fn format_timestamp(secs: i64) -> String {
    if secs <= 0 {
        return "(no timestamp)".to_string();
    }
    match chrono::DateTime::from_timestamp(secs, 0) {
        Some(dt) => dt.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        None => format!("(ts={secs})"),
    }
}

// ── JSON output ───────────────────────────────────────────────────────────────

fn print_json(entries: &[ChangeLogEntry]) {
    // Only emit marker commits (those with messages) as structured JSON.
    let markers: Vec<_> = entries.iter().filter(|e| e.message.is_some()).collect();

    print!("[");
    for (i, e) in markers.iter().enumerate() {
        if i > 0 {
            print!(",");
        }
        let ts = if e.timestamp_secs > 0 {
            format!("\"{}\"", format_timestamp(e.timestamp_secs))
        } else {
            "null".to_string()
        };
        print!(
            "{{\"hash\":\"{}\",\"timestamp\":{},\"message\":{}}}",
            e.hash_short,
            ts,
            serde_json::to_string(e.message.as_deref().unwrap_or("")).unwrap_or_default(),
        );
    }
    println!("]");
}

// ── TOML output ───────────────────────────────────────────────────────────────

fn print_toml(entries: &[ChangeLogEntry]) {
    let markers: Vec<_> = entries.iter().filter(|e| e.message.is_some()).collect();

    for (i, e) in markers.iter().enumerate() {
        if i > 0 {
            println!();
        }
        println!("[[commit]]");
        println!("hash = \"{}\"", e.hash_short);
        if e.timestamp_secs > 0 {
            println!("timestamp = \"{}\"", format_timestamp(e.timestamp_secs));
        }
        if let Some(msg) = &e.message {
            println!(
                "message = \"{}\"",
                msg.replace('\\', "\\\\").replace('"', "\\\"")
            );
        }
    }
}
