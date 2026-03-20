/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::data::Schedule;

const WIDGET_CSS: &str = include_str!("../../../../widget/cosam-calendar.css");
const WIDGET_JS: &str = include_str!("../../../../widget/cosam-calendar.js");

pub fn generate_preview_html(schedule: &Schedule) -> Result<String> {
    let json_data = schedule.export_public_json_string()?;
    let title = &schedule.meta.title;
    let generation = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    Ok(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta name="cosam-generation" content="{generation}">
  <title>{title} — Preview</title>
  <style>
    body {{
      margin: 0;
      padding: 20px;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      background: #f5f5f5;
    }}
    .page-header {{
      text-align: center;
      padding: 20px;
      margin-bottom: 20px;
    }}
    .page-header h1 {{
      margin: 0 0 8px 0;
      font-size: 28px;
      color: #1f2937;
    }}
    .page-header p {{
      margin: 0;
      color: #6b7280;
      font-size: 14px;
    }}
{WIDGET_CSS}
  </style>
</head>
<body>
  <div class="page-header">
    <h1>{title}</h1>
    <p>Editor Preview — auto-refreshes when data changes</p>
  </div>
  <div id="cosam-calendar"></div>
  <script>
{WIDGET_JS}
  </script>
  <script>
    CosAmCalendar.init({{
      el: '#cosam-calendar',
      data: {json_data},
      watchForChanges: true
    }});
  </script>
</body>
</html>"#
    ))
}

pub fn preview_file_path() -> PathBuf {
    std::env::temp_dir().join("cosam-preview.html")
}

pub fn write_preview(schedule: &Schedule) -> Result<PathBuf> {
    let html = generate_preview_html(schedule)?;
    let path = preview_file_path();
    std::fs::write(&path, html.as_bytes())
        .with_context(|| format!("Failed to write preview to {}", path.display()))?;
    Ok(path)
}

pub fn open_preview_in_browser(path: &Path) -> Result<()> {
    open::that(path).with_context(|| format!("Failed to open {} in browser", path.display()))
}

pub fn cleanup_preview() {
    let path = preview_file_path();
    let _ = std::fs::remove_file(path);
}
