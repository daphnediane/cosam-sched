mod data;
mod ui;

use std::path::PathBuf;

use anyhow::Result;
use gpui::prelude::*;
use gpui::{App, Application, Bounds, WindowBounds, WindowOptions, px, size};

use data::Schedule;
use ui::ScheduleEditor;

fn resolve_schedule_path() -> Result<PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        return Ok(PathBuf::from(&args[1]));
    }

    // Default: look for sample-data.json relative to the editor crate
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sample = manifest_dir
        .parent()
        .unwrap()
        .join("widget")
        .join("sample-data.json");
    if sample.exists() {
        return Ok(sample);
    }

    anyhow::bail!(
        "Usage: cosam-editor <schedule.json>\n\
         No file specified and sample-data.json not found."
    );
}

fn main() {
    let schedule_path = match resolve_schedule_path() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let schedule = match Schedule::load(&schedule_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error loading schedule: {e}");
            std::process::exit(1);
        }
    };

    let title = schedule.meta.title.clone();

    Application::new().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1200.), px(800.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_window, cx| cx.new(|cx| ScheduleEditor::new(schedule.clone(), cx)),
        )
        .expect("Failed to open window");

        // Set the window title if possible
        let _ = &title;
    });
}
