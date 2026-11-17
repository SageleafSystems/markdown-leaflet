// SPDX-License-Identifier: GPL-3.0-or-later
//! Window geometry: tracks the viewport and keeps in-flight drags within budget.

use crate::app::{Message, State};
use iced::Task;

impl State {
    pub(crate) fn update_layout(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WindowResized(size) => {
                self.viewport = self.viewport.resized(size.width).with_height(size.height);
                self.menus.context = None;
                // A window edge being dragged must never wait on a slide.
                self.gutters.snap_slides();
                if let Some(side) = self.gutters.drag.map(|drag| drag.side) {
                    let max_width = self.layout_plan().drag_max(side);
                    if let Some(drag) = &mut self.gutters.drag {
                        drag.max_width = max_width;
                    }
                }
                self.measure_preview_soon()
            }
            _ => unreachable!("layout messages are routed by State::update"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gutter::{GutterSide, ResizeDrag};

    #[test]
    fn resizing_updates_the_viewport() {
        let mut state = State::blank();
        let _ = state.update_layout(Message::WindowResized(iced::Size::new(640.0, 480.0)));
        assert_eq!(state.viewport.width, 640.0);
    }

    #[test]
    fn resizing_ends_a_running_slide() {
        let mut state = State::blank();
        let _ = state.update_gutters(Message::TogglePanel(GutterSide::Left));
        assert!(state.gutters.is_animating());
        let _ = state.update_layout(Message::WindowResized(iced::Size::new(900.0, 600.0)));
        assert!(!state.gutters.is_animating());
    }

    #[test]
    fn resizing_during_a_drag_refreshes_its_budget() {
        let mut state = State::blank();
        state.gutters.drag = Some(ResizeDrag {
            side: GutterSide::Left,
            origin_x: 0.0,
            initial_width: 260.0,
            max_width: 480.0,
        });
        let _ = state.update_layout(Message::WindowResized(iced::Size::new(800.0, 600.0)));
        assert_eq!(state.gutters.drag.map(|d| d.max_width), Some(364.0));
    }
}
