// SPDX-License-Identifier: GPL-3.0-or-later
//! The dialog shown when a link points at a Markdown file outside the open folder.

use crate::app::{Message, State, chrome_size};
use crate::domain::editors::EDITORS;
use crate::layout::spacing;
use crate::ui::theme;
use iced::Element;
use iced::widget::{button, checkbox, column, container, row, text};

impl State {
    pub(crate) fn external_link_dialog(&self) -> Element<'_, Message> {
        let appearance = self.appearance;
        let path = self.nav.pending_external.as_deref();
        let name = path
            .and_then(|path| path.file_name())
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let location = path
            .map(|path| path.display().to_string())
            .unwrap_or_default();
        let editors = EDITORS
            .iter()
            .fold(row![].spacing(spacing::GAP), |editors, editor| {
                editors.push(
                    button(text(editor.name).size(chrome_size(13.0)))
                        .padding([spacing::units(0.5), spacing::units(1.5)])
                        .style(theme::dialog_button_secondary(appearance))
                        .on_press(Message::ExternalLinkOpenWith(editor.clone())),
                )
            });
        let choice_button = |label: &'static str| {
            button(text(label).size(chrome_size(13.0)))
                .padding([spacing::units(0.75), spacing::units(2.0)])
                .style(theme::dialog_button_primary(appearance))
        };
        let content = column![
            text("Open linked file?").size(chrome_size(19.0)),
            text(format!("“{name}” is outside the folder you have open.")).size(chrome_size(13.0)),
            container(text(location).size(chrome_size(12.0)))
                .padding(spacing::units(1.0))
                .width(iced::Length::Fill)
                .style(move |_| theme::dialog_well(appearance)),
            column![
                choice_button("Open Containing Folder").on_press(Message::ExternalLinkOpenFolder),
                text("Show that file's folder in the sidebar. Your current folder is closed.")
                    .size(chrome_size(12.0))
                    .color(theme::colors(appearance).text_muted),
            ]
            .spacing(spacing::units(0.5)),
            column![
                choice_button("Open as External Document")
                    .on_press(Message::ExternalLinkOpenDocument),
                text("Read just this file. Your current folder stays open.")
                    .size(chrome_size(12.0))
                    .color(theme::colors(appearance).text_muted),
            ]
            .spacing(spacing::units(0.5)),
            column![
                text("Open with another editor").size(chrome_size(13.0)),
                editors.wrap(),
            ]
            .spacing(spacing::units(0.5)),
            checkbox(self.nav.external_always)
                .label("Always do this (Open Containing Folder or Open as External Document)")
                .on_toggle(Message::ExternalLinkAlways)
                .size(chrome_size(15.0))
                .text_size(chrome_size(13.0)),
            button(text("Cancel").size(chrome_size(13.0)))
                .padding([spacing::units(0.75), spacing::units(2.0)])
                .style(theme::dialog_button_secondary(appearance))
                .on_press(Message::CancelExternalLink),
        ]
        .spacing(spacing::units(1.5));
        container(content)
            .width(iced::Length::Fixed(460.0))
            .padding(spacing::units(3.0))
            .style(move |_| theme::dialog_panel(appearance))
            .into()
    }
}
