// SPDX-License-Identifier: GPL-3.0-or-later
//! Width arithmetic for gutters: limits, chrome overhead, and what still fits. Gaps come from
//! the spacing scale in [`super::spacing`].

use super::spacing::{HALF, UNIT, units};
use crate::gutter::GutterSide;

/// Generic lower bound exercised by the resize-math tests below (`settle`, `drag_width`,
/// `effective_width`); every real gutter now has its own floor (`MIN_LEFT_WIDTH`,
/// `MIN_RIGHT_WIDTH`), since each one's header needs a different minimum to avoid clipping.
#[cfg(test)]
pub const MIN_WIDTH: f32 = 180.0;
/// The left gutter's floor: its header (toggle, Back, Forward, the combined Open button, New,
/// and the icon-only Files/Recent tabs) needs more room than a bare 180px gives it without
/// clipping or wrapping.
pub const MIN_LEFT_WIDTH: f32 = 300.0;
/// The right gutter's floor: its header (Outline and Details tabs, copy-path and toggle buttons)
/// has to fit without clipping, which a bare 180px is too narrow for.
pub const MIN_RIGHT_WIDTH: f32 = 340.0;
pub const MAX_WIDTH: f32 = 480.0;
pub const MIN_PREVIEW_WIDTH: f32 = 420.0;
/// The gap kept between a docked gutter's card and the window's own edge: just enough that the
/// card plainly floats clear of it rather than butting up against the chrome.
pub const OUTER_MARGIN: f32 = UNIT;
/// The gap kept on the edge that carries the resize handle: just wide enough for the handle's
/// hit zone, which can still overlap into the card itself (see [`TARGET_WIDTH`]).
pub const INNER_MARGIN: f32 = UNIT;
/// The full-height resize target laid over a gutter card's inner edge. It costs no layout width.
pub const TARGET_WIDTH: f32 = UNIT;
/// The resize cue's line: drawn over the card's border, on the edge that faces the preview.
pub const LINE_WIDTH: f32 = 2.0;
/// Space left uncovered beside an overlay gutter.
pub const OVERLAY_MARGIN: f32 = units(2.5);
/// Space between the window or a gutter card and the preview's text.
pub const TEXT_INSET: f32 = UNIT + HALF;
/// Space kept at the preview text's right edge, clear of its scrollbar.
pub const SCROLLBAR_CLEARANCE: f32 = units(2.0);
/// Horizontal space between the window or gutters and the preview's text: the 12 px text inset
/// on the left and the 16 px beside the scrollbar on the right.
pub const PREVIEW_PADDING: f32 = TEXT_INSET + SCROLLBAR_CLEARANCE;

/// Layout width a docked gutter takes beyond its card: the outer margin plus the inner,
/// resize-handle margin.
pub fn gutter_overhead() -> f32 {
    OUTER_MARGIN + INNER_MARGIN
}

/// A horizontal range, in pixels from the left of a gutter's footprint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub start: f32,
    pub end: f32,
}

impl Span {
    pub fn width(self) -> f32 {
        self.end - self.start
    }
}

/// Where a docked gutter's card, resize target, and resize line sit across its footprint (the
/// card's `width` plus [`gutter_overhead`]). The view places the strip from these spans, so what
/// is drawn and what is tested cannot drift apart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeSpans {
    /// The footprint: the card and the margins around it.
    pub footprint: Span,
    /// The card itself, border included.
    pub card: Span,
    /// The press target: [`TARGET_WIDTH`] on the card's inner padding, against its edge.
    pub target: Span,
    /// The cue line: [`LINE_WIDTH`], over the card's border.
    pub line: Span,
}

/// The spans for `side`'s card at `width`. The edge that faces the preview is the card's right
/// edge on the left gutter and its left edge on the right gutter; the target and line lie inside
/// the card there, the line flush with the card's outer edge so it covers the border.
pub fn edge_spans(side: GutterSide, width: f32) -> EdgeSpans {
    let footprint = Span {
        start: 0.0,
        end: width + gutter_overhead(),
    };
    let (outer, inner) = match side {
        GutterSide::Left => (OUTER_MARGIN, INNER_MARGIN),
        GutterSide::Right => (INNER_MARGIN, OUTER_MARGIN),
    };
    let card = Span {
        start: outer,
        end: footprint.end - inner,
    };
    let (target, line) = match side {
        GutterSide::Left => (
            Span {
                start: card.end - TARGET_WIDTH,
                end: card.end,
            },
            Span {
                start: card.end - LINE_WIDTH,
                end: card.end,
            },
        ),
        GutterSide::Right => (
            Span {
                start: card.start,
                end: card.start + TARGET_WIDTH,
            },
            Span {
                start: card.start,
                end: card.start + LINE_WIDTH,
            },
        ),
    };
    EdgeSpans {
        footprint,
        card,
        target,
        line,
    }
}

/// The toolbar's padding on the window edge a docked card slides in from, `progress` of the way
/// in (`0.0` away, `1.0` in place). Away it is `closed`, the toolbar's own edge padding. In place
/// it puts the toolbar's content `gap` clear of the card's visible border, which lies
/// [`INNER_MARGIN`] inside the card's footprint on either side. Between, it moves with the card.
pub fn toolbar_padding(closed: f32, card_width: f32, gap: f32, progress: f32) -> f32 {
    let beside_card = card_width + gutter_overhead() - INNER_MARGIN + gap;
    closed + (beside_card - closed) * progress
}

/// Whether two docked gutters leave the preview its minimum width, in a window
/// that allows two gutters at all.
pub fn both_fit(two_gutters_allowed: bool, window: f32, left: f32, right: f32) -> bool {
    two_gutters_allowed && window >= left + right + 2.0 * gutter_overhead() + MIN_PREVIEW_WIDTH
}

/// The widest a docked gutter may be; `other` is the second docked gutter's width.
pub fn feasible_max(window: f32, other: Option<f32>) -> f32 {
    let overhead = if other.is_some() { 2.0 } else { 1.0 } * gutter_overhead();
    (window - MIN_PREVIEW_WIDTH - overhead - other.unwrap_or(0.0)).min(MAX_WIDTH)
}

/// The narrowest `side`'s card may be.
pub fn min_width(side: GutterSide) -> f32 {
    match side {
        GutterSide::Left => MIN_LEFT_WIDTH,
        GutterSide::Right => MIN_RIGHT_WIDTH,
    }
}

/// A stored width limited to `max` but never below `min`, rounded to a whole pixel.
/// Every width this module hands back for rendering goes through here (a drag reads the
/// pointer's own fractional position, and a fitted width divides a fractional window size), so
/// rounding once, at the source, is what keeps a gutter's card and the toolbar's reserved
/// padding — two independently laid-out trees fed the same logical width — from rounding to
/// different device pixels and drifting apart by a pixel or two at the seam between them.
pub fn effective_width(stored: f32, max: f32, min: f32) -> f32 {
    stored.clamp(min, max.max(min)).round()
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Span {
        fn contains(self, inner: Span) -> bool {
            self.start <= inner.start && inner.end <= self.end
        }
    }

    #[test]
    fn the_resize_line_sits_on_the_cards_own_border_not_in_the_margin_beside_it() {
        for side in [GutterSide::Left, GutterSide::Right] {
            for width in [180.0, 260.0, 480.0] {
                let spans = edge_spans(side, width);
                assert_eq!(spans.card.width(), width, "{side:?} card is its width");
                assert!(spans.card.contains(spans.target), "{side:?} target in card");
                assert!(spans.target.contains(spans.line), "{side:?} line in target");
                assert_eq!(spans.line.width(), LINE_WIDTH);
                assert_eq!(spans.target.width(), TARGET_WIDTH);
                // The line's outer edge is the card's outer edge, so it covers the border:
                // nothing, least of all the 8px margin, lies between them.
                let (line_edge, card_edge) = match side {
                    GutterSide::Left => (spans.line.end, spans.card.end),
                    GutterSide::Right => (spans.line.start, spans.card.start),
                };
                assert_eq!(line_edge, card_edge, "{side:?} line is on the card edge");
            }
        }
    }

    #[test]
    fn the_spans_agree_with_the_footprint_the_layout_reserves() {
        for side in [GutterSide::Left, GutterSide::Right] {
            let spans = edge_spans(side, 300.0);
            assert_eq!(spans.footprint.width(), 300.0 + gutter_overhead());
            // The two margins around the card are exactly the overhead every width budget uses.
            let margins =
                spans.card.start - spans.footprint.start + (spans.footprint.end - spans.card.end);
            assert_eq!(margins, gutter_overhead());
        }
        // The margin facing the window is the outer one; the one facing the preview, the inner.
        let left = edge_spans(GutterSide::Left, 300.0);
        assert_eq!(left.card.start, OUTER_MARGIN);
        assert_eq!(left.footprint.end - left.card.end, INNER_MARGIN);
        let right = edge_spans(GutterSide::Right, 300.0);
        assert_eq!(right.card.start, INNER_MARGIN);
        assert_eq!(right.footprint.end - right.card.end, OUTER_MARGIN);
    }

    #[test]
    fn toolbar_content_rests_the_gap_from_the_cards_visible_border_on_both_sides() {
        let gap = UNIT;
        for side in [GutterSide::Left, GutterSide::Right] {
            let spans = edge_spans(side, 300.0);
            // How far the card's border on the preview side is from the window edge it hugs.
            let border_from_window_edge = match side {
                GutterSide::Left => spans.card.end,
                GutterSide::Right => spans.footprint.end - spans.card.start,
            };
            assert_eq!(
                toolbar_padding(16.0, 300.0, gap, 1.0),
                border_from_window_edge + gap,
                "{side:?}: content starts exactly one gap past the border, not past the margin"
            );
        }
    }

    #[test]
    fn toolbar_padding_moves_from_its_resting_edge_to_the_card_without_jumping() {
        assert_eq!(toolbar_padding(16.0, 300.0, 8.0, 0.0), 16.0);
        let mut last = 16.0;
        for step in 1..=10 {
            let padding = toolbar_padding(16.0, 300.0, 8.0, step as f32 / 10.0);
            assert!(padding >= last);
            last = padding;
        }
        assert_eq!(last, 300.0 + gutter_overhead() - INNER_MARGIN + 8.0);
    }

    #[test]
    fn preview_and_two_gutter_budget() {
        let max = feasible_max(1120.0, Some(260.0));
        assert_eq!(max, 408.0);
        assert!(both_fit(true, 1120.0, 260.0, 408.0));
        assert!(!both_fit(true, 1120.0, 260.0, 424.0));
        assert!(!both_fit(false, 1400.0, 260.0, 220.0));
        assert_eq!(feasible_max(900.0, None), 464.0);
    }

    #[test]
    fn effective_width_never_drops_below_the_minimum() {
        assert_eq!(effective_width(300.0, 250.0, MIN_WIDTH), 250.0);
        assert_eq!(effective_width(300.0, 100.0, MIN_WIDTH), MIN_WIDTH);
        assert_eq!(effective_width(100.0, 480.0, MIN_WIDTH), MIN_WIDTH);
        assert_eq!(effective_width(260.0, 480.0, MIN_WIDTH), 260.0);
        assert_eq!(
            effective_width(200.0, 480.0, min_width(GutterSide::Right)),
            MIN_RIGHT_WIDTH
        );
        assert_eq!(
            effective_width(200.0, 480.0, min_width(GutterSide::Left)),
            MIN_LEFT_WIDTH
        );
    }
}
