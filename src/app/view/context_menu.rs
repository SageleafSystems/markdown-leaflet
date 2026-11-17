// SPDX-License-Identifier: GPL-3.0-or-later
//! The open right-click menu, placed at the click and kept inside the window.

use crate::app::view::menu_list::{MENU_WIDTH, menu_divider, menu_panel, menu_row_with_info};
use crate::app::{ContextEntry, ContextMenu, Message, State};
use crate::layout::spacing;
use iced::widget::{Space, Stack, column, container, mouse_area, pin};
use iced::{Fill, Point};

/// Approximate rendered heights, for placing the popup before it is laid out.
const ROW_HEIGHT: f32 = 28.0;
const DIVIDER_HEIGHT: f32 = 9.0;

impl State {
    /// `root` with the context menu and its click-away layer stacked on top.
    pub(crate) fn with_context_menu<'a>(
        &'a self,
        root: Stack<'a, Message>,
        menu: &'a ContextMenu,
    ) -> Stack<'a, Message> {
        let appearance = self.appearance;
        let mut rows = column![].spacing(spacing::menu::ROW_GAP);
        for (index, entry) in menu.entries.iter().enumerate() {
            rows = rows.push(match entry {
                ContextEntry::Item {
                    label,
                    icon,
                    shortcut,
                    checked,
                    message,
                    info,
                } => menu_row_with_info(
                    label,
                    *icon,
                    *shortcut,
                    *checked,
                    message.as_ref().map(|_| Message::ContextMenuPick(index)),
                    info.as_deref(),
                    appearance,
                ),
                ContextEntry::Divider => menu_divider(appearance),
            });
        }
        let height = menu
            .entries
            .iter()
            .map(|entry| match entry {
                ContextEntry::Item { .. } => ROW_HEIGHT,
                ContextEntry::Divider => DIVIDER_HEIGHT,
            })
            .sum::<f32>()
            + spacing::menu::PANEL_PAD * 2.0;
        let at = place(
            menu.position,
            menu.anchor_right,
            (MENU_WIDTH, height),
            (self.viewport.width, self.viewport.height),
        );
        root.push(
            mouse_area(container(Space::new()).width(Fill).height(Fill))
                .on_press(Message::DismissContextMenu)
                .on_right_press(Message::DismissContextMenu),
        )
        .push(
            pin(mouse_area(menu_panel(rows, appearance))
                .on_press(Message::ContextMenuInert)
                .on_right_press(Message::ContextMenuInert))
            .position(at),
        )
    }
}

/// Where a `size` popup opened at `click` goes: at the click, flipped left
/// or up when it would cross the window's edge, and never above or left of
/// the window.
fn place(
    click: Point,
    anchor_right: Option<f32>,
    (width, height): (f32, f32),
    (window_w, window_h): (f32, f32),
) -> Point {
    let x = if click.x + width <= window_w {
        click.x
    } else {
        // A dropdown stays under its control, right edges aligned; a right-click menu flips.
        anchor_right.unwrap_or(click.x) - width
    };
    let y = if click.y + height > window_h {
        click.y - height
    } else {
        click.y
    };
    Point::new(x.max(0.0), y.max(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_at_the_click_when_it_fits() {
        assert_eq!(
            place(Point::new(10.0, 20.0), None, (200.0, 100.0), (800.0, 600.0)),
            Point::new(10.0, 20.0)
        );
    }

    #[test]
    fn flips_away_from_the_right_and_bottom_edges() {
        assert_eq!(
            place(
                Point::new(700.0, 550.0),
                None,
                (200.0, 100.0),
                (800.0, 600.0)
            ),
            Point::new(500.0, 450.0)
        );
    }

    #[test]
    fn a_dropdown_that_does_not_fit_right_aligns_under_its_control() {
        // A 32px control spanning x 740..772 in an 800px window, opened with a 230px menu.
        assert_eq!(
            place(
                Point::new(740.0, 40.0),
                Some(772.0),
                (230.0, 100.0),
                (800.0, 600.0)
            ),
            Point::new(542.0, 40.0)
        );
        // When it fits it stays flush with the control's left edge.
        assert_eq!(
            place(
                Point::new(100.0, 40.0),
                Some(132.0),
                (230.0, 100.0),
                (800.0, 600.0)
            ),
            Point::new(100.0, 40.0)
        );
    }

    #[test]
    fn stays_inside_a_window_smaller_than_the_menu() {
        assert_eq!(
            place(Point::new(50.0, 40.0), None, (200.0, 100.0), (120.0, 90.0)),
            Point::new(0.0, 0.0)
        );
    }
}
