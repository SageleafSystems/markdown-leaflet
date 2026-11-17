// SPDX-License-Identifier: GPL-3.0-or-later
//! A scrollable operation that keeps the highlighted navigation row in view.

use iced::advanced::widget::operation::Outcome;
use iced::advanced::widget::{Id, Operation};

/// Id of the scrollable that holds the folder tree.
pub const NAV_SCROLL: &str = "nav-scroll";
/// Id carried by whichever row is currently highlighted.
pub const NAV_CURSOR: &str = "nav-cursor";

/// Finds the offset that brings the highlighted row fully into view, if it is not already.
#[derive(Default)]
pub struct RevealCursor {
    /// Viewport height, content origin, and current offset of the nav scrollable.
    viewport: Option<(f32, f32, f32)>,
    target: Option<f32>,
}

impl RevealCursor {
    pub fn new() -> Self {
        Self::default()
    }
}

/// The offset that shows `top..bottom` inside a viewport, or `None` when it already fits.
pub fn offset_to_reveal(top: f32, bottom: f32, offset: f32, height: f32) -> Option<f32> {
    if top < offset {
        Some(top.max(0.0))
    } else if bottom > offset + height {
        Some((bottom - height).max(0.0))
    } else {
        None
    }
}

impl Operation<Option<f32>> for RevealCursor {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Option<f32>>)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: iced::Rectangle,
        content_bounds: iced::Rectangle,
        translation: iced::Vector,
        _state: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        if id == Some(&Id::new(NAV_SCROLL)) {
            self.viewport = Some((bounds.height, content_bounds.y, translation.y));
        }
    }

    fn container(&mut self, id: Option<&Id>, bounds: iced::Rectangle) {
        if id != Some(&Id::new(NAV_CURSOR)) {
            return;
        }
        if let Some((height, origin, offset)) = self.viewport {
            let top = bounds.y - origin;
            self.target = offset_to_reveal(top, top + bounds.height, offset, height);
        }
    }

    fn finish(&self) -> Outcome<Option<f32>> {
        Outcome::Some(self.target)
    }
}

#[cfg(test)]
mod tests {
    use super::offset_to_reveal;

    #[test]
    fn visible_row_needs_no_scroll() {
        assert_eq!(offset_to_reveal(40.0, 70.0, 0.0, 200.0), None);
    }

    #[test]
    fn row_below_the_viewport_scrolls_down_to_its_bottom_edge() {
        assert_eq!(offset_to_reveal(210.0, 240.0, 0.0, 200.0), Some(40.0));
    }

    #[test]
    fn row_above_the_viewport_scrolls_up_to_its_top_edge() {
        assert_eq!(offset_to_reveal(20.0, 50.0, 100.0, 200.0), Some(20.0));
    }
}
