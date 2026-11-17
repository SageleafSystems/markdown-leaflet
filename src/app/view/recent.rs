// SPDX-License-Identifier: GPL-3.0-or-later
//! The left gutter's Recent tab: every remembered file or folder as a sidebar row, newest first
//! (`self.recents` is already kept in that order; see `domain::recent::remember`).
use crate::app::{Message, State, chrome_size};
use crate::domain::recent::{RecentItem, RecentKind};
use crate::layout::spacing;
use crate::ui::icons::{self, svg_icon};
use crate::ui::theme;
use iced::widget::{button, center, column, container, row, scrollable, text};
use iced::{Alignment, Element, Fill};
use std::path::Path;

impl State {
    fn recent_row<'a>(&self, item: &RecentItem) -> Element<'a, Message> {
        let colors = theme::colors(self.appearance);
        let icon = match item.kind {
            RecentKind::File => icons::FILE,
            RecentKind::Folder => icons::FOLDER,
        };
        let name = item
            .path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let parent = item.path.parent().unwrap_or_else(|| Path::new(""));
        let mut label = column![text(name).size(chrome_size(13.0))]
            .spacing(spacing::list::ROW_GAP)
            .width(Fill);
        if !parent.as_os_str().is_empty() {
            label = label.push(
                text(parent.display().to_string())
                    .size(chrome_size(11.0))
                    .color(colors.text_muted),
            );
        }
        let row_icon = container(svg_icon(icon, colors.file_icon, 14.0))
            .width(spacing::list::ICON_SLOT)
            .align_x(Alignment::Center);
        let content = row![row_icon, label]
            .spacing(spacing::GAP)
            .align_y(Alignment::Center);
        button(content)
            .padding(spacing::list::FILE_ROW_PAD)
            .width(Fill)
            .style(theme::sidebar_row(self.appearance, false))
            .on_press(Message::OpenRecent(item.clone()))
            .into()
    }

    /// The left gutter's Recent tab content: a plain "No recent items" in place of an empty
    /// list, mirroring `file_list`'s "No Markdown files" treatment.
    pub(crate) fn recent_panel(&self) -> Element<'_, Message> {
        if self.recents.is_empty() {
            return center(
                text("No recent items").color(theme::colors(self.appearance).text_muted),
            )
            .into();
        }
        let mut list = column![].spacing(spacing::list::TREE_ROW_GAP);
        for item in &self.recents {
            list = list.push(self.recent_row(item));
        }
        scrollable(list).width(Fill).height(Fill).into()
    }
}
