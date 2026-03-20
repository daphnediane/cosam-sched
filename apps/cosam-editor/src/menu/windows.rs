/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use gpui::prelude::*;
use gpui::{
    Context, Menu, OwnedMenu, OwnedMenuItem, SharedString, Window, anchored, deferred, div, px, rgb,
};

pub(super) fn menus() -> Vec<Menu> {
    vec![
        Menu {
            name: "File".into(),
            items: super::file_menu_items(true),
        },
        Menu {
            name: "Edit".into(),
            items: super::edit_menu_items(),
        },
    ]
}

pub struct WindowsMenuBar {
    menus: Vec<OwnedMenu>,
    open_menu_index: Option<usize>,
    open_submenu_index: Option<usize>,
    submenu_rect: Option<gpui::Bounds<gpui::Pixels>>,
}

impl WindowsMenuBar {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let menus = cx.get_menus().unwrap_or_default();
        Self {
            menus,
            open_menu_index: None,
            open_submenu_index: None,
            submenu_rect: None,
        }
    }

    fn render_menu_popup(
        &self,
        menu: &OwnedMenu,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut popup = div()
            .id("menu-popup")
            .min_w(px(200.0))
            .py(px(4.0))
            .bg(rgb(0xFFFFFF))
            .border_1()
            .border_color(rgb(0xD1D5DB))
            .rounded_md()
            .shadow_lg()
            .text_sm()
            .occlude();

        for (i, item) in menu.items.iter().enumerate() {
            match item {
                OwnedMenuItem::Separator => {
                    popup = popup.child(div().h(px(1.0)).my(px(4.0)).bg(rgb(0xE5E7EB)));
                }
                OwnedMenuItem::Action { name, action, .. } => {
                    let shortcut = window.keystroke_text_for(action.as_ref());
                    let action = action.boxed_clone();
                    let mut row = div()
                        .id(SharedString::from(format!("menu-item-{i}")))
                        .flex()
                        .items_center()
                        .justify_between()
                        .px(px(12.0))
                        .py(px(6.0))
                        .mx(px(4.0))
                        .rounded_sm()
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(0xE5E7EB)))
                        .child(SharedString::from(name.clone()));

                    if !shortcut.is_empty() {
                        row = row.child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x9CA3AF))
                                .ml(px(24.0))
                                .child(SharedString::from(shortcut)),
                        );
                    }

                    popup = popup.child(row.on_click(cx.listener(move |this, _, window, cx| {
                        this.open_menu_index = None;
                        this.open_submenu_index = None;
                        cx.notify();
                        window.dispatch_action(action.boxed_clone(), cx);
                    })));
                }
                OwnedMenuItem::Submenu(submenu) => {
                    let submenu_name = submenu.name.clone();
                    let submenu_items = submenu.items.clone();
                    let mut row = div()
                        .id(SharedString::from(format!("menu-item-{i}")))
                        .flex()
                        .items_center()
                        .justify_between()
                        .px(px(12.0))
                        .py(px(6.0))
                        .mx(px(4.0))
                        .rounded_sm()
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(0xE5E7EB)))
                        .child(SharedString::from(submenu_name.clone()))
                        .child(div().text_color(rgb(0x9CA3AF)).child("▶"));

                    popup =
                        popup.child(row.on_click(cx.listener(move |this, event, window, cx| {
                            this.open_submenu_index = Some(i);
                            cx.notify();
                        })));
                }
                OwnedMenuItem::SystemMenu(_) => {}
            }
        }

        popup
    }

    fn render_submenu_popup(
        &self,
        submenu: &OwnedMenu,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut popup = div()
            .id("submenu-popup")
            .min_w(px(200.0))
            .py(px(4.0))
            .bg(rgb(0xFFFFFF))
            .border_1()
            .border_color(rgb(0xD1D5DB))
            .rounded_md()
            .shadow_lg()
            .text_sm()
            .occlude();

        for (i, item) in submenu.items.iter().enumerate() {
            match item {
                OwnedMenuItem::Separator => {
                    popup = popup.child(div().h(px(1.0)).my(px(4.0)).bg(rgb(0xE5E7EB)));
                }
                OwnedMenuItem::Action { name, action, .. } => {
                    let shortcut = window.keystroke_text_for(action.as_ref());
                    let action = action.boxed_clone();
                    let mut row = div()
                        .id(SharedString::from(format!("submenu-item-{i}")))
                        .flex()
                        .items_center()
                        .justify_between()
                        .px(px(12.0))
                        .py(px(6.0))
                        .mx(px(4.0))
                        .rounded_sm()
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(0xE5E7EB)))
                        .child(SharedString::from(name.clone()));

                    if !shortcut.is_empty() {
                        row = row.child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x9CA3AF))
                                .ml(px(24.0))
                                .child(SharedString::from(shortcut)),
                        );
                    }

                    popup = popup.child(row.on_click(cx.listener(move |this, _, window, cx| {
                        this.open_menu_index = None;
                        this.open_submenu_index = None;
                        cx.notify();
                        window.dispatch_action(action.boxed_clone(), cx);
                    })));
                }
                OwnedMenuItem::Submenu(_) | OwnedMenuItem::SystemMenu(_) => {}
            }
        }

        popup
    }
}

impl Render for WindowsMenuBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut bar = div()
            .flex()
            .items_center()
            .px(px(4.0))
            .border_b_1()
            .border_color(rgb(0xE5E7EB))
            .bg(rgb(0xF9FAFB))
            .text_sm();

        for (idx, menu) in self.menus.iter().enumerate() {
            let name = menu.name.clone();
            let is_open = self.open_menu_index == Some(idx);

            bar = bar.child(
                div()
                    .id(SharedString::from(format!("menu-trigger-{idx}")))
                    .relative()
                    .px(px(8.0))
                    .py(px(4.0))
                    .rounded_sm()
                    .cursor_pointer()
                    .text_color(rgb(0x111827))
                    .hover(|s| s.bg(rgb(0xE5E7EB)))
                    .when(is_open, |el| el.bg(rgb(0xE5E7EB)))
                    .child(name)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.open_menu_index = if this.open_menu_index == Some(idx) {
                            None
                        } else {
                            Some(idx)
                        };
                        cx.notify();
                    })),
            );
        }

        let mut root = div().child(bar);

        if let Some(idx) = self.open_menu_index {
            if let Some(menu) = self.menus.get(idx) {
                let popup = self.render_menu_popup(menu, window, cx);
                root = root.child(deferred(anchored().child(popup)).with_priority(1));
            }
        }

        root
    }
}
