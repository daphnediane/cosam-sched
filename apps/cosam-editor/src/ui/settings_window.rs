/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::path::PathBuf;

use crate::settings::{ExportSettings, SettingsManager};
use gpui::prelude::*;
use gpui::{Action, App, FocusHandle, Focusable, SharedString, Window, actions, div, rgb};
use gpui_component::{
    button::Button,
    setting::{SettingField, SettingGroup, SettingItem, SettingPage, Settings},
};

actions!(
    settings_window,
    [
        BrowseCss,
        BrowseJs,
        BrowseTemplate,
        ClearCss,
        ClearJs,
        ClearTemplate,
    ]
);

pub struct SettingsWindow {
    focus_handle: FocusHandle,
    settings: ExportSettings,
}

impl SettingsWindow {
    pub fn new(cx: &mut gpui::Context<Self>) -> Self {
        let settings = match SettingsManager::load_settings() {
            Ok(settings) => settings,
            Err(error) => {
                eprintln!("Failed to load settings: {error}");
                ExportSettings::default()
            }
        };

        Self {
            focus_handle: cx.focus_handle(),
            settings,
        }
    }

    fn browse_css_file(
        &mut self,
        _: &BrowseCss,
        _window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("CSS files", &["css"])
            .add_filter("All files", &["*"])
            .pick_file()
        {
            self.settings.widget_css_path = Some(path);
            if let Err(error) =
                SettingsManager::set_widget_css_path(self.settings.widget_css_path.clone())
            {
                eprintln!("Failed to save CSS path setting: {error}");
            }
            cx.notify();
        }
    }

    fn browse_js_file(&mut self, _: &BrowseJs, _window: &mut Window, cx: &mut gpui::Context<Self>) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("JavaScript files", &["js"])
            .add_filter("All files", &["*"])
            .pick_file()
        {
            self.settings.widget_js_path = Some(path);
            if let Err(error) =
                SettingsManager::set_widget_js_path(self.settings.widget_js_path.clone())
            {
                eprintln!("Failed to save JS path setting: {error}");
            }
            cx.notify();
        }
    }

    fn browse_template_file(
        &mut self,
        _: &BrowseTemplate,
        _window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("HTML files", &["html", "htm"])
            .add_filter("All files", &["*"])
            .pick_file()
        {
            self.settings.test_template_path = Some(path);
            if let Err(error) =
                SettingsManager::set_test_template_path(self.settings.test_template_path.clone())
            {
                eprintln!("Failed to save template path setting: {error}");
            }
            cx.notify();
        }
    }

    fn clear_css_path(&mut self, _: &ClearCss, _window: &mut Window, cx: &mut gpui::Context<Self>) {
        self.settings.widget_css_path = None;
        if let Err(error) = SettingsManager::set_widget_css_path(None) {
            eprintln!("Failed to clear CSS path setting: {error}");
        }
        cx.notify();
    }

    fn clear_js_path(&mut self, _: &ClearJs, _window: &mut Window, cx: &mut gpui::Context<Self>) {
        self.settings.widget_js_path = None;
        if let Err(error) = SettingsManager::set_widget_js_path(None) {
            eprintln!("Failed to clear JS path setting: {error}");
        }
        cx.notify();
    }

    fn clear_template_path(
        &mut self,
        _: &ClearTemplate,
        _window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        self.settings.test_template_path = None;
        if let Err(error) = SettingsManager::set_test_template_path(None) {
            eprintln!("Failed to clear template path setting: {error}");
        }
        cx.notify();
    }

    fn toggle_minified(&mut self, value: bool, cx: &mut gpui::Context<Self>) {
        self.settings.minified = value;
        if let Err(error) = SettingsManager::set_minified(value) {
            eprintln!("Failed to save minified setting: {error}");
        }
        cx.notify();
    }
}

impl Focusable for SettingsWindow {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl gpui::Render for SettingsWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let minified = self.settings.minified;
        let css_path = self.settings.widget_css_path.clone();
        let js_path = self.settings.widget_js_path.clone();
        let template_path = self.settings.test_template_path.clone();

        Settings::new("app-settings")
            .pages(vec![
                SettingPage::new("Export")
                    .groups(vec![
                        SettingGroup::new()
                            .title("General")
                            .items(vec![
                                SettingItem::new(
                                    "Minified exports",
                                    SettingField::switch(
                                        move |_cx: &App| minified,
                                        move |val: bool, cx: &mut App| {
                                            // Handle value change - this will need to be connected to settings
                                        },
                                    )
                                )
                                .description("Minify HTML, CSS, and JavaScript in exported files."),
                            ]),
                        SettingGroup::new()
                            .title("Custom Assets")
                            .items(vec![
                                SettingItem::new(
                                    "Widget CSS",
                                    SettingField::render(move |_options, _window, cx| {
                                        let path_display = css_path
                                            .as_ref()
                                            .map(|p| p.display().to_string())
                                            .unwrap_or_else(|| "Built-in".to_string());

                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(rgb(0x374151))
                                                    .child("Widget CSS:")
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(rgb(0x6B7280))
                                                    .child(path_display)
                                            )
                                            .child(
                                                Button::new("browse-css")
                                                    .label("Browse...")
                                                    .on_click(|_event, _window, cx| {
                                                        cx.dispatch_action(&BrowseCss);
                                                    })
                                            )
                                            .child(
                                                Button::new("clear-css")
                                                    .label("Clear")
                                                    .on_click(|_event, _window, cx| {
                                                        cx.dispatch_action(&ClearCss);
                                                    })
                                            )
                                    })
                                ),
                                SettingItem::new(
                                    "Widget JavaScript",
                                    SettingField::render(move |_options, _window, cx| {
                                        let path_display = js_path
                                            .as_ref()
                                            .map(|p| p.display().to_string())
                                            .unwrap_or_else(|| "Built-in".to_string());

                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(rgb(0x374151))
                                                    .child("Widget JS:")
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(rgb(0x6B7280))
                                                    .child(path_display)
                                            )
                                            .child(
                                                Button::new("browse-js")
                                                    .label("Browse...")
                                                    .on_click(|_event, _window, cx| {
                                                        cx.dispatch_action(&BrowseJs);
                                                    })
                                            )
                                            .child(
                                                Button::new("clear-js")
                                                    .label("Clear")
                                                    .on_click(|_event, _window, cx| {
                                                        cx.dispatch_action(&ClearJs);
                                                    })
                                            )
                                    })
                                ),
                                SettingItem::new(
                                    "Test Template",
                                    SettingField::render(move |_options, _window, cx| {
                                        let path_display = template_path
                                            .as_ref()
                                            .map(|p| p.display().to_string())
                                            .unwrap_or_else(|| "Built-in".to_string());

                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(rgb(0x374151))
                                                    .child("Test Template:")
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(rgb(0x6B7280))
                                                    .child(path_display)
                                            )
                                            .child(
                                                Button::new("browse-template")
                                                    .label("Browse...")
                                                    .on_click(|_event, _window, cx| {
                                                        cx.dispatch_action(&BrowseTemplate);
                                                    })
                                            )
                                            .child(
                                                Button::new("clear-template")
                                                    .label("Clear")
                                                    .on_click(|_event, _window, cx| {
                                                        cx.dispatch_action(&ClearTemplate);
                                                    })
                                            )
                                    })
                                ),
                            ])
                            .description("Override built-in widget assets with custom files. Leave blank to use built-in versions."),
                    ]),
            ])
    }
}
