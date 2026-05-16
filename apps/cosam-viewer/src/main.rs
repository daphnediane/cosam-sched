/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

mod data;
mod state;
mod ui;

use ui::App;

fn main() {
    #[cfg(feature = "desktop")]
    {
        use dioxus_desktop::{Config, WindowBuilder};

        dioxus::LaunchBuilder::desktop()
            .with_cfg(
                Config::new().with_window(
                    WindowBuilder::new()
                        .with_title("cosam Schedule Viewer")
                        .with_inner_size(dioxus_desktop::tao::dpi::LogicalSize::new(
                            1100.0_f64,
                            800.0_f64,
                        )),
                ),
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
