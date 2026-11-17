// SPDX-License-Identifier: GPL-3.0-or-later
//! Drag and snap rules for the gutter dividers. Window-width budgets live in
//! `layout::budget`.

use crate::layout::budget::effective_width;

pub const LEFT_DEFAULT: f32 = 300.0;
pub const RIGHT_DEFAULT: f32 = 360.0;
const SNAP_RADIUS: f32 = 14.0;
const ANCHORS: [f32; 7] = [180.0, 220.0, 260.0, 320.0, 360.0, 400.0, 480.0];

pub fn drag_width(
    initial: f32,
    origin_x: f32,
    cursor_x: f32,
    direction: f32,
    max: f32,
    min: f32,
) -> f32 {
    effective_width(initial + direction * (cursor_x - origin_x), max, min)
}

pub fn settle(width: f32, max: f32, min: f32) -> f32 {
    let width = effective_width(width, max, min);
    ANCHORS
        .into_iter()
        .filter(|anchor| *anchor >= min && *anchor <= max && (*anchor - width).abs() <= SNAP_RADIUS)
        .min_by(|a, b| {
            (a - width)
                .abs()
                .total_cmp(&(b - width).abs())
                .then_with(|| a.total_cmp(b))
        })
        .unwrap_or(width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::budget::{MAX_WIDTH, MIN_WIDTH, feasible_max};

    #[test]
    fn snapping_and_free_widths() {
        assert_eq!(settle(252.0, MAX_WIDTH, MIN_WIDTH), 260.0);
        assert_eq!(settle(228.0, MAX_WIDTH, MIN_WIDTH), 220.0);
        assert_eq!(settle(240.0, MAX_WIDTH, MIN_WIDTH), 240.0);
        assert_eq!(settle(340.0, MAX_WIDTH, MIN_WIDTH), 340.0);
    }

    #[test]
    fn drag_direction_and_bounds() {
        assert_eq!(
            drag_width(260.0, 500.0, 530.0, 1.0, 480.0, MIN_WIDTH),
            290.0
        );
        assert_eq!(
            drag_width(220.0, 500.0, 470.0, -1.0, 480.0, MIN_WIDTH),
            250.0
        );
        assert_eq!(drag_width(260.0, 500.0, 0.0, 1.0, 480.0, MIN_WIDTH), 180.0);
        assert_eq!(
            drag_width(260.0, 500.0, 1000.0, 1.0, 480.0, MIN_WIDTH),
            480.0
        );
        assert_eq!(
            drag_width(260.0, 500.0, 500.0, 1.0, 480.0, MIN_WIDTH),
            260.0
        );
    }

    #[test]
    fn snapping_respects_the_preview_budget() {
        let max = feasible_max(1120.0, Some(260.0));
        assert_eq!(settle(380.0, max, MIN_WIDTH), 380.0); // 400 would collapse the preview
    }
}
