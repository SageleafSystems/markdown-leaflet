// SPDX-License-Identifier: GPL-3.0-or-later
//! Toasts stacked over the bottom-right corner of the window (issue #43).

use crate::app::{Message, State, chrome_size};
use crate::layout::spacing;
use crate::ui::icons::{self, svg_icon};
use crate::ui::theme;
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Fill};

const TOAST_WIDTH: f32 = 340.0;

impl State {
    /// `None` while there is nothing to show, so callers can skip the layer entirely.
    pub(crate) fn notification_stack(&self) -> Option<Element<'_, Message>> {
        if self.notifications.is_empty() {
            return None;
        }
        let appearance = self.appearance;
        let colors = theme::colors(appearance);
        let mut stack = column![].spacing(spacing::units(1.0));
        for notification in &self.notifications {
            let mut content = row![
                text(notification.message.clone())
                    .size(chrome_size(13.0))
                    .width(Fill)
            ]
            .spacing(spacing::GAP)
            .align_y(Alignment::Center);
            if let Some((label, action)) = &notification.action {
                content = content.push(
                    button(text(*label).size(chrome_size(13.0)))
                        .padding([spacing::units(0.4), spacing::units(1.25)])
                        .style(theme::dialog_button_primary(appearance))
                        .on_press(action.clone()),
                );
            }
            let id = notification.id;
            content = content.push(
                button(svg_icon(icons::CLOSE, colors.text_muted, 14.0))
                    .padding(spacing::HALF)
                    .style(theme::search_clear_button(appearance))
                    .on_press(Message::DismissNotification(id)),
            );
            stack = stack.push(
                container(content)
                    .width(iced::Length::Fixed(TOAST_WIDTH))
                    .padding(spacing::units(1.25))
                    .style(move |_| theme::toast_panel(appearance)),
            );
        }
        Some(
            container(stack)
                .width(Fill)
                .height(Fill)
                .padding(spacing::units(2.0))
                .align_x(Alignment::End)
                .align_y(Alignment::End)
                .into(),
        )
    }
}
