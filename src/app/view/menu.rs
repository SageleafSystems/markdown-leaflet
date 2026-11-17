// SPDX-License-Identifier: GPL-3.0-or-later
use crate::actions::MenuAction;
use crate::app::view::menu_list::{
    MENU_WIDTH, menu_divider, menu_panel, menu_row, menu_row_with_hover,
};
use crate::app::{AppMenu, DocumentView, Message, Mode, State, chrome_size};
use crate::config::{AppearanceChoice, ExternalLinkAction, FontChoice};
use crate::domain::keymap::{self, Command};
use crate::gutter::{GutterSide, RightTab};
use crate::layout::breakpoint::MenuLayout;
use crate::layout::spacing;
use crate::ui::theme;
use iced::widget::{Space, button, column, container, mouse_area, row, text};
use iced::{Alignment, Element, Fill, Padding};

/// Height of the menu bar; popups and the click-away overlay start below it.
#[cfg(not(target_os = "macos"))]
pub(crate) const MENU_BAR_HEIGHT: f32 = 40.0;

impl State {
    #[cfg(not(target_os = "macos"))]
    pub(crate) fn menu_bar(&self) -> Element<'_, Message> {
        let appearance = self.appearance;
        let mut bar = row![]
            .spacing(spacing::menu::BAR_GAP)
            .align_y(Alignment::Center);
        for (label, menu) in [
            ("File", AppMenu::File),
            ("View", AppMenu::View),
            ("Help", AppMenu::Help),
        ] {
            let active = self.menus.active == Some(menu);
            let trigger: iced::widget::Button<'_, Message> =
                button(text(label).size(chrome_size(12.0)))
                    .padding(spacing::menu::TRIGGER_PAD)
                    .style(move |t, s| theme::menu_bar_button(appearance, active)(t, s))
                    .on_press(Message::ToggleMenu(menu));
            let trigger: Element<'_, Message> = if self.menus.active.is_some() {
                mouse_area(trigger)
                    .on_enter(Message::ActivateMenu(menu))
                    .into()
            } else {
                trigger.into()
            };
            bar = bar.push(trigger);
        }
        container(bar)
            .width(Fill)
            .padding(spacing::menu::BAR_PAD)
            .style(move |_| theme::toolbar_panel(appearance))
            .into()
    }

    #[cfg(not(target_os = "macos"))]
    pub(crate) fn menu_popup(&self) -> Element<'_, Message> {
        let plan = self.layout_plan();
        let replace_file_menu = plan.menu == MenuLayout::Replace;
        let Some(menu) = self.menus.active else {
            return Space::new().into();
        };
        let appearance = self.appearance;
        // This module only builds on non-macOS targets (see the native menu
        // for macOS's own labels), so the keymap is always asked for them.
        let mac = false;
        let mut items = column![].spacing(spacing::menu::ROW_GAP);
        match menu {
            AppMenu::File => {
                items = items
                    .push(menu_row_with_hover(
                        "Open File…",
                        keymap::label_for(Command::OpenFile, mac).as_deref(),
                        false,
                        Message::MenuAction(MenuAction::OpenFile),
                        Message::CloseRecentMenu,
                        appearance,
                    ))
                    .push(menu_row_with_hover(
                        "Open Folder…",
                        keymap::label_for(Command::OpenFolder, mac).as_deref(),
                        false,
                        Message::MenuAction(MenuAction::OpenFolder),
                        Message::CloseRecentMenu,
                        appearance,
                    ));
                if !self.recents.is_empty() {
                    items = items
                        .push(menu_divider(appearance))
                        .push(menu_row_with_hover(
                            "Open Recent",
                            Some("›"),
                            false,
                            Message::OpenRecentMenu,
                            Message::OpenRecentMenu,
                            appearance,
                        ));
                }
                if matches!(self.mode, Mode::Folder { .. }) {
                    items = items.push(menu_row_with_hover(
                        "Close Current",
                        keymap::label_for(Command::CloseCurrent, mac).as_deref(),
                        false,
                        Message::MenuAction(MenuAction::CloseCurrent),
                        Message::CloseRecentMenu,
                        appearance,
                    ));
                }
                items = items
                    .push(menu_divider(appearance))
                    .push(menu_row_with_hover(
                        "Quit",
                        keymap::label_for(Command::Quit, mac).as_deref(),
                        false,
                        Message::MenuAction(MenuAction::Quit),
                        Message::CloseRecentMenu,
                        appearance,
                    ));
            }
            AppMenu::View => {
                if matches!(self.mode, Mode::Folder { .. }) {
                    items = items
                        .push(menu_row(
                            "Show Files",
                            keymap::label_for(Command::ToggleFiles, mac).as_deref(),
                            plan.shows(GutterSide::Left),
                            Some(Message::MenuAction(MenuAction::ShowFiles)),
                            appearance,
                        ))
                        .push(menu_row(
                            "Show Outline",
                            keymap::label_for(Command::ShowOutline, mac).as_deref(),
                            plan.shows(GutterSide::Right)
                                && self.gutters.right_tab == RightTab::Outline,
                            Some(Message::MenuAction(MenuAction::ShowOutline)),
                            appearance,
                        ))
                        .push(menu_row(
                            "Show Details",
                            keymap::label_for(Command::ShowDetails, mac).as_deref(),
                            plan.shows(GutterSide::Right)
                                && self.gutters.right_tab == RightTab::Details,
                            Some(Message::MenuAction(MenuAction::ShowDetails)),
                            appearance,
                        ))
                        .push(menu_row(
                            "Show Minimap",
                            keymap::label_for(Command::ToggleMinimap, mac).as_deref(),
                            self.config.preview.minimap,
                            Some(Message::MenuAction(MenuAction::ToggleMinimap)),
                            appearance,
                        ))
                        .push(menu_row(
                            "View Source",
                            keymap::label_for(Command::ToggleSourceView, mac).as_deref(),
                            self.document_view == DocumentView::Source,
                            Some(Message::MenuAction(MenuAction::ToggleSourceView)),
                            appearance,
                        ))
                        .push(menu_divider(appearance))
                        .push(menu_row(
                            "Zoom In",
                            keymap::label_for(Command::ZoomIn, mac).as_deref(),
                            false,
                            Some(Message::MenuAction(MenuAction::ZoomIn)),
                            appearance,
                        ))
                        .push(menu_row(
                            "Zoom Out",
                            keymap::label_for(Command::ZoomOut, mac).as_deref(),
                            false,
                            Some(Message::MenuAction(MenuAction::ZoomOut)),
                            appearance,
                        ))
                        .push(menu_row(
                            "Reset Zoom",
                            keymap::label_for(Command::ZoomReset, mac).as_deref(),
                            false,
                            Some(Message::MenuAction(MenuAction::ResetZoom)),
                            appearance,
                        ))
                        .push(menu_divider(appearance));
                }
                for (label, choice) in [
                    ("System Appearance", AppearanceChoice::System),
                    ("Slate Theme", AppearanceChoice::Slate),
                    ("Harbor Theme", AppearanceChoice::Harbor),
                    ("Nightfall Theme", AppearanceChoice::Nightfall),
                    ("Linen Theme", AppearanceChoice::Linen),
                    ("Paper Theme", AppearanceChoice::Paper),
                ] {
                    items = items.push(menu_row(
                        label,
                        None,
                        self.config.theme.appearance == choice,
                        Some(Message::MenuAction(MenuAction::Appearance(choice))),
                        appearance,
                    ));
                }
                items = items.push(menu_divider(appearance));
                for choice in FontChoice::ALL {
                    items = items.push(menu_row(
                        &format!("{} Font", choice.label()),
                        None,
                        self.config.preview.font == choice,
                        Some(Message::MenuAction(MenuAction::PreviewFont(choice))),
                        appearance,
                    ));
                }
                items = items.push(menu_divider(appearance)).push(menu_row(
                    "Ask About External Links",
                    None,
                    self.config.links.external_markdown == ExternalLinkAction::Ask,
                    Some(Message::MenuAction(MenuAction::AskAboutExternalLinks)),
                    appearance,
                ));
                items = items.push(menu_divider(appearance)).push(menu_row(
                    "Reload Theme",
                    keymap::label_for(Command::ReloadTheme, mac).as_deref(),
                    false,
                    Some(Message::MenuAction(MenuAction::ReloadTheme)),
                    appearance,
                ));
            }
            AppMenu::Help => {
                items = items
                    .push(menu_row(
                        "Keyboard Shortcuts",
                        keymap::label_for(Command::ShowShortcuts, mac).as_deref(),
                        false,
                        Some(Message::MenuAction(MenuAction::Shortcuts)),
                        appearance,
                    ))
                    .push(menu_divider(appearance))
                    .push(menu_row(
                        "About Markdown Leaflet",
                        None,
                        false,
                        Some(Message::MenuAction(MenuAction::About)),
                        appearance,
                    ));
            }
        }
        let panel = menu_panel(items, appearance);
        let popup: Element<'_, Message> = if menu == AppMenu::File && self.menus.recent_open {
            let mut recent = column![].spacing(spacing::menu::ROW_GAP);
            if replace_file_menu {
                recent = recent.push(menu_row(
                    "‹  File",
                    None,
                    false,
                    Some(Message::CloseRecentMenu),
                    appearance,
                ));
            }
            for item in &self.recents {
                let label = item.compact_label();
                recent = recent.push(menu_row(
                    &label,
                    None,
                    false,
                    Some(Message::MenuAction(MenuAction::Recent(item.clone()))),
                    appearance,
                ));
            }
            let recent_panel = menu_panel(recent, appearance);
            if replace_file_menu {
                recent_panel.into()
            } else {
                row![panel, recent_panel].spacing(4).into()
            }
        } else {
            panel.into()
        };
        let desired_x: f32 = match menu {
            AppMenu::File => 12.0,
            AppMenu::View => 60.0,
            AppMenu::Help => 108.0,
        };
        let popup_width = if menu == AppMenu::File && self.menus.recent_open && !replace_file_menu {
            MENU_WIDTH * 2.0 + 4.0
        } else {
            MENU_WIDTH
        };
        container(popup)
            .padding(Padding {
                top: MENU_BAR_HEIGHT,
                right: 0.0,
                bottom: 0.0,
                left: desired_x.min((plan.viewport.width - popup_width).max(0.0)),
            })
            .into()
    }
}
