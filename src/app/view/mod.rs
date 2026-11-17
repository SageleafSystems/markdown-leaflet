// SPDX-License-Identifier: GPL-3.0-or-later
//! Rendering of the current state; no transitions live here.

use crate::app::view::toolbar::tip;
use crate::app::{Dialog, Message, Mode, State, chrome_size};
use crate::domain::folder::matching_files;
use crate::domain::workspace::OpenRequest;
use crate::layout::spacing;
use crate::ui::icons::{self, svg_icon};
use crate::ui::theme;
#[cfg(not(target_os = "macos"))]
use iced::Padding;
#[cfg(not(target_os = "macos"))]
use iced::widget::mouse_area;
use iced::widget::{Space, Stack, button, center, column, container, row, svg, text, tooltip};
use iced::{Alignment, Element, Fill};

mod context_menu;
mod dialog;
mod document;
mod external_link;
mod folder_panes;
mod gutters;
#[cfg(not(target_os = "macos"))]
mod menu;
mod menu_list;
mod new_item;
mod notifications;
mod panels;
mod recent;
mod toolbar;

pub(crate) use document::{PREVIEW_INSET, TextSetting, render_markdown};
pub(crate) use folder_panes::search_bar;
pub(crate) use gutters::{docked_margin, gutter_card, overlay_margin};
pub(crate) use panels::{inspector_panel, outline_sidebar};

/// Width of the logo on the welcome screen; its height keeps the artwork's 1100 × 300 ratio.
const WELCOME_LOGO_WIDTH: f32 = 320.0;

impl State {
    /// The welcome screen's centered content: the logo, the open buttons and the shortcut hint.
    /// Recent items live in the left panel now (`State::welcome_panes`), not inline here.
    fn welcome_view(&self) -> Element<'_, Message> {
        let logo: &'static [u8] = if theme::is_dark(self.appearance) {
            include_bytes!("../../../assets/brand/markdown-leaflet-logo-dark.svg")
        } else {
            include_bytes!("../../../assets/brand/markdown-leaflet-logo.svg")
        };
        let welcome = column![
            svg(svg::Handle::from_memory(logo))
                .width(WELCOME_LOGO_WIDTH)
                .height(WELCOME_LOGO_WIDTH * 300.0 / 1100.0),
            text("Open a Markdown file or folder to get started.").size(chrome_size(18.0)),
            row![
                button(text("Open File…").size(chrome_size(13.0)))
                    .on_press(Message::OpenFileClicked),
                button(text("Open Folder…").size(chrome_size(13.0)))
                    .on_press(Message::OpenFolderClicked),
            ]
            .spacing(spacing::units(1.5)),
            text("Ctrl/⌘O file · Ctrl/⌘Shift+O folder · Ctrl/⌘F find")
                .size(chrome_size(11.0))
                .color(theme::colors(self.appearance).text_muted),
        ]
        .spacing(spacing::GAP_SECTION)
        .align_x(Alignment::Center);
        center(welcome).into()
    }

    /// `body`, under a dismissible banner when there is an error to show.
    fn with_error_banner<'a>(&'a self, body: Element<'a, Message>) -> Element<'a, Message> {
        if let Some(error) = &self.error {
            let mut banner = row![text(error.message.clone()).width(Fill)]
                .spacing(8)
                .align_y(Alignment::Center);
            if let Some(retry) = &error.retry {
                banner = banner
                    .push(button(text("Retry").size(chrome_size(13.0))).on_press(retry.clone()));
            }
            banner = banner.push(
                button(text("Dismiss").size(chrome_size(13.0))).on_press(Message::DismissError),
            );
            column![
                container(banner)
                    .padding(8)
                    .style(move |_| theme::error_banner(self.appearance)),
                body,
            ]
            .into()
        } else {
            body
        }
    }

    /// The footer strip: file and heading counts on the left, the version on the right.
    fn footer(&self) -> Element<'_, Message> {
        let status = match &self.mode {
            _ if self.save_status.is_some() => self
                .save_status
                .as_ref()
                .map_or_else(String::new, crate::app::SaveStatus::label),
            _ if self.nav.opening.is_some() => {
                format!(
                    "Opening “{}”…",
                    self.nav
                        .opening
                        .as_ref()
                        .map_or_else(String::new, OpenRequest::label)
                )
            }
            Mode::Welcome => String::from("Ready"),
            Mode::Folder {
                root,
                files,
                query,
                selected,
            } => {
                let count = if query.trim().is_empty() {
                    format!("{} files", files.len())
                } else {
                    format!(
                        "{} of {} files",
                        matching_files(root, files, query).len(),
                        files.len()
                    )
                };
                if let Some(doc) = selected {
                    format!("{count} · {} headings", doc.outline.len())
                } else {
                    count
                }
            }
        };
        let footer_label = |value| {
            text(value)
                .size(chrome_size(10.0))
                .color(theme::colors(self.appearance).text_muted)
        };
        let mut left = row![footer_label(status).width(Fill)];
        if let Some(note) = self.frontmatter_status() {
            left = left.push(note);
        }
        container(column![
            container(Space::new().height(1).width(Fill))
                .style(move |_| theme::footer_divider(self.appearance)),
            row![
                left.width(Fill),
                footer_label(format!(
                    "Markdown Leaflet · v{} ({})",
                    env!("CARGO_PKG_VERSION"),
                    env!("MP_BUILD_REV")
                )),
            ]
            .spacing(spacing::units(1.5))
            .align_y(Alignment::Center)
            .padding(spacing::FOOTER_PAD),
        ])
        .width(Fill)
        .style(move |_| theme::footer_panel(self.appearance))
        .into()
    }

    /// A small warning glyph with a tooltip explaining why, shown in the footer's status area
    /// when the open document's frontmatter block failed to parse (issue #65). The document
    /// still renders — this is a note, not an error banner.
    fn frontmatter_status(&self) -> Option<Element<'_, Message>> {
        let doc = self.current_document()?;
        let message = doc.frontmatter_error.as_ref()?;
        Some(
            tooltip(
                svg_icon(icons::WARNING, theme::colors(self.appearance).warning, 12.0),
                tip(
                    format!("Frontmatter couldn't be parsed: {message}"),
                    None,
                    self.appearance,
                ),
                tooltip::Position::Top,
            )
            .gap(spacing::TOOLTIP_GAP)
            .into(),
        )
    }

    pub(crate) fn view(&self) -> Element<'_, Message> {
        let toolbar = self.toolbar();

        let (body, left_overlay, right_overlay): (
            Element<'_, Message>,
            Option<Element<'_, Message>>,
            Option<Element<'_, Message>>,
        ) = match &self.mode {
            Mode::Welcome => {
                let panes = self.welcome_panes();
                (panes.content, panes.left_overlay, panes.right_overlay)
            }
            Mode::Folder {
                root,
                files,
                query,
                selected,
            } => {
                let folder = self.folder_panes(root, files, query, selected.as_ref());
                (folder.content, folder.left_overlay, folder.right_overlay)
            }
        };

        let content = self.with_error_banner(body);

        let footer = self.footer();
        // The toolbar and content form the "chrome" block that the open gutters' floating cards
        // are stacked over: this is what lets a card's top reach above the toolbar and bump into
        // it, while its bottom still lands exactly at the content's bottom (flush with the
        // footer), with no separate math for "how tall is the footer." The in-window menu row
        // (non-macOS) sits outside this stack, always visible and clickable, above it — it is
        // chrome in its own right, the same way the native macOS menu bar is, not a surface the
        // nav should visually float over.
        let chrome = column![toolbar, container(content).height(Fill)].height(Fill);
        let mut upper = Stack::new().push(chrome).width(Fill).height(Fill);
        // Always one ghost layer, so the cards after it keep their place in the stack.
        upper = upper.push(self.toolbar_ghost());
        // Each card arrives already pinned at its slid position (`State::slid_card`).
        if let Some(card) = left_overlay {
            upper = upper.push(card);
        }
        if let Some(card) = right_overlay {
            upper = upper.push(card);
        }
        #[cfg(not(target_os = "macos"))]
        let layout = column![self.menu_bar(), upper, footer];
        #[cfg(target_os = "macos")]
        let layout = column![upper, footer];
        let main = container(layout).style(move |_| container::Style {
            background: Some(iced::Background::Color(
                theme::colors(self.appearance).background,
            )),
            ..container::Style::default()
        });
        // The root is always a `Stack` with `main` first, so opening a menu or dialog adds layers
        // without changing the tree's shape; otherwise iced rebuilds it and the preview's scroll
        // offset is lost.
        let root = Stack::new().push(main);
        #[cfg(not(target_os = "macos"))]
        let root = if self.menus.active.is_some() {
            root.push(
                container(
                    mouse_area(container(Space::new()).width(Fill).height(Fill))
                        .on_press(Message::DismissMenu),
                )
                .width(Fill)
                .height(Fill)
                .padding(Padding {
                    top: menu::MENU_BAR_HEIGHT,
                    right: 0.0,
                    bottom: 0.0,
                    left: 0.0,
                }),
            )
            .push(self.menu_popup())
        } else {
            root
        };
        let root = match &self.menus.context {
            Some(menu) => self.with_context_menu(root, menu),
            None => root,
        };
        // A draft that has no room inline is named in a dialog instead.
        let dialog = self.menus.dialog.or_else(|| {
            (self.nav.draft.is_some() && !self.draft_inline()).then_some(Dialog::NewItem)
        });
        let root = if let Some(dialog) = dialog {
            root.push(center(self.dialog_view(dialog)).width(Fill).height(Fill))
        } else {
            root
        };
        match self.notification_stack() {
            Some(toasts) => root.push(toasts),
            None => root,
        }
        .into()
    }
}
