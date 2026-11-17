// SPDX-License-Identifier: GPL-3.0-or-later
//! Menu rows, dividers, and the popup panel shared by the in-window menu bar
//! and context menus, on every platform.

use crate::app::{Message, chrome_size};
use crate::layout::spacing;
use crate::ui::icons::{self, svg_icon};
use crate::ui::theme;
use crate::ui::theme::Appearance;
use iced::widget::{Space, button, container, row, text, tooltip};
use iced::{Alignment, Element, Fill};

/// Width of a menu popup panel.
pub(crate) const MENU_WIDTH: f32 = 230.0;

#[cfg(not(target_os = "macos"))]
pub(crate) fn menu_row_with_hover<'a>(
    label: &str,
    shortcut: Option<&str>,
    checked: bool,
    message: Message,
    on_hover: Message,
    appearance: Appearance,
) -> Element<'a, Message> {
    iced::widget::mouse_area(menu_row(
        label,
        shortcut,
        checked,
        Some(message),
        appearance,
    ))
    .on_enter(on_hover)
    .into()
}

/// One menu row. A `None` message renders it disabled.
#[cfg(not(target_os = "macos"))]
pub(crate) fn menu_row<'a>(
    label: &str,
    shortcut: Option<&str>,
    checked: bool,
    message: Option<Message>,
    appearance: Appearance,
) -> Element<'a, Message> {
    menu_row_with_info(label, None, shortcut, checked, message, None, appearance)
}

/// A menu row whose leading column shows an icon instead of a check mark. With `info`, an (i)
/// icon ends the row and hovering it shows the text, which says why the row is disabled.
pub(crate) fn menu_row_with_info<'a>(
    label: &str,
    icon: Option<&str>,
    shortcut: Option<&str>,
    checked: bool,
    message: Option<Message>,
    info: Option<&str>,
    appearance: Appearance,
) -> Element<'a, Message> {
    let colors = theme::colors(appearance);
    let label_color = if message.is_some() {
        colors.text
    } else {
        colors.text_muted
    };
    let lead: Element<'a, Message> = match icon {
        Some(fragment) => container(svg_icon(fragment, label_color, 14.0))
            .width(22)
            .into(),
        None => text(if checked { "✓" } else { " " })
            .size(chrome_size(12.0))
            .width(18)
            .into(),
    };
    let mut content = row![
        lead,
        text(label.to_owned())
            .size(chrome_size(12.0))
            .color(label_color)
            .wrapping(text::Wrapping::None)
            .width(Fill),
    ]
    .align_y(Alignment::Center);
    if let Some(shortcut) = shortcut {
        content = content.push(
            text(shortcut.to_owned())
                .size(chrome_size(10.0))
                .color(colors.text_muted),
        );
    }
    if let Some(info) = info {
        let note = container(text(info.to_owned()).size(chrome_size(11.0)).width(Fill))
            .max_width(280.0)
            .padding([spacing::HALF, spacing::UNIT])
            .style(move |_| theme::tooltip_panel(appearance));
        content = content.push(
            tooltip(
                svg_icon(icons::INFO, colors.text, 14.0),
                note,
                tooltip::Position::Right,
            )
            .gap(spacing::TOOLTIP_GAP),
        );
    }
    button(content)
        .width(Fill)
        .padding(spacing::menu::ROW_PAD)
        .style(move |t, s| theme::menu_item(appearance)(t, s))
        .on_press_maybe(message)
        .into()
}

pub(crate) fn menu_divider(appearance: Appearance) -> Element<'static, Message> {
    container(
        container(Space::new().height(1).width(Fill))
            .style(move |_| theme::footer_divider(appearance)),
    )
    .width(Fill)
    .padding(spacing::menu::DIVIDER_PAD)
    .into()
}

/// The rounded, shadowed panel a menu's rows sit in.
pub(crate) fn menu_panel<'a>(
    items: impl Into<Element<'a, Message>>,
    appearance: Appearance,
) -> container::Container<'a, Message> {
    container(items)
        .width(iced::Length::Fixed(MENU_WIDTH))
        .padding(spacing::menu::PANEL_PAD)
        .style(move |_| theme::menu_popup(appearance))
}
