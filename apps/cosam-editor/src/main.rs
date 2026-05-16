/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

mod menu;
mod ui;

use dioxus_desktop::{Config, WindowBuilder};
use ui::App;

fn main() {
    let app_menu = menu::build_app_menu();

    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            Config::new()
                .with_window(
                    WindowBuilder::new()
                        .with_title("cosam-editor")
                        .with_inner_size(dioxus_desktop::tao::dpi::LogicalSize::new(
                            1200.0_f64, 800.0_f64,
                        )),
                )
                .with_menu(app_menu),
        )
        .launch(App);
}
