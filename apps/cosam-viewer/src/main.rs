/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

mod data;
mod state;
mod ui;

use ui::App;

// ---------------------------------------------------------------------------
// Menu item IDs — matched in the muda event handler in app.rs
// ---------------------------------------------------------------------------

pub const ID_FILE_OPEN: &str = "viewer-open-file";
pub const ID_FILE_OPEN_FOLDER: &str = "viewer-open-folder";

// ---------------------------------------------------------------------------
// Window launch dimensions (logical pixels)
// ---------------------------------------------------------------------------

/// Default launch width of the application window, in logical pixels.
pub const WINDOW_LAUNCH_WIDTH_PX: f64 = 1100.0;

/// Default launch height of the application window, in logical pixels.
pub const WINDOW_LAUNCH_HEIGHT_PX: f64 = 800.0;

// ---------------------------------------------------------------------------

#[cfg(feature = "desktop")]
fn build_app_menu() -> dioxus_desktop::muda::Menu {
    use dioxus_desktop::muda::{Menu, MenuItem, PredefinedMenuItem, Submenu};

    let menu = Menu::new();

    // macOS: app menu (cosam-viewer, Hide, Quit…)
    #[cfg(target_os = "macos")]
    {
        let app_menu = Submenu::new("cosam-viewer", true);
        app_menu
            .append_items(&[
                &PredefinedMenuItem::services(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::hide(None),
                &PredefinedMenuItem::hide_others(None),
                &PredefinedMenuItem::show_all(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::quit(None),
            ])
            .ok();
        menu.append(&app_menu).ok();
    }

    // File menu
    let file_menu = Submenu::new("File", true);
    file_menu
        .append_items(&[
            &MenuItem::with_id(ID_FILE_OPEN, "Open File…", true, None),
            &MenuItem::with_id(ID_FILE_OPEN_FOLDER, "Open Folder…", true, None),
        ])
        .ok();

    #[cfg(not(target_os = "macos"))]
    {
        file_menu.append(&PredefinedMenuItem::separator()).ok();
        file_menu
            .append(&PredefinedMenuItem::quit(Some("Exit")))
            .ok();
    }

    // Edit menu — clipboard operations via predefined items
    let edit_menu = Submenu::new("Edit", true);
    edit_menu
        .append_items(&[
            &PredefinedMenuItem::copy(None),
            &PredefinedMenuItem::cut(None),
            &PredefinedMenuItem::paste(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::select_all(None),
        ])
        .ok();

    // Window menu (macOS standard)
    let window_menu = Submenu::new("Window", true);
    window_menu
        .append_items(&[
            &PredefinedMenuItem::minimize(None),
            &PredefinedMenuItem::maximize(None),
            &PredefinedMenuItem::close_window(None),
        ])
        .ok();

    menu.append_items(&[&file_menu, &edit_menu, &window_menu])
        .ok();

    menu
}

fn main() {
    #[cfg(feature = "desktop")]
    {
        use dioxus_desktop::{Config, WindowBuilder};

        dioxus::LaunchBuilder::desktop()
            .with_cfg(
                Config::new()
                    .with_window(
                        WindowBuilder::new()
                            .with_title("cosam Schedule Viewer")
                            .with_inner_size(dioxus_desktop::tao::dpi::LogicalSize::new(
                                WINDOW_LAUNCH_WIDTH_PX,
                                WINDOW_LAUNCH_HEIGHT_PX,
                            )),
                    )
                    .with_menu(build_app_menu()),
            )
            .launch(App);
    }

    #[cfg(feature = "mobile")]
    {
        dioxus::LaunchBuilder::mobile().launch(App);
    }

    #[cfg(not(any(feature = "desktop", feature = "mobile")))]
    compile_error!("Enable either the 'desktop' or 'mobile' feature");
}
