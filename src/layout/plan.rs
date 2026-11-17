// SPDX-License-Identifier: GPL-3.0-or-later
//! The layout plan: what the user asked for, resolved against the window width.

use super::breakpoint::{Breakpoint, MenuLayout, ToolbarDensity, Viewport};
use super::budget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GutterSide {
    Left,
    Right,
}

impl GutterSide {
    pub fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
        }
    }

    pub fn other(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

/// What the user asked for. It is remembered across window resizes and says
/// nothing about what is visible at the current width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanelIntent {
    pub left_open: bool,
    pub right_open: bool,
    /// The gutter that wins when both are open but only one fits.
    pub active: GutterSide,
    pub left_width: f32,
    pub right_width: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    Hidden,
    /// Beside the preview, taking width from it.
    Docked,
    /// On top of the preview.
    Overlay,
    /// A compact bar above the preview, in place of a side panel. Only the
    /// left (Files) gutter takes this form, and only in `Compact` windows.
    Strip,
}

/// Where each panel sits at one window width. Derived on demand; never stored.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutPlan {
    pub viewport: Viewport,
    pub breakpoint: Breakpoint,
    pub left: Placement,
    pub right: Placement,
    /// Width to render the left gutter at, after fitting the window.
    pub left_width: f32,
    pub right_width: f32,
    pub toolbar: ToolbarDensity,
    pub menu: MenuLayout,
}

impl LayoutPlan {
    pub fn placement(&self, side: GutterSide) -> Placement {
        match side {
            GutterSide::Left => self.left,
            GutterSide::Right => self.right,
        }
    }

    pub fn width(&self, side: GutterSide) -> f32 {
        match side {
            GutterSide::Left => self.left_width,
            GutterSide::Right => self.right_width,
        }
    }

    /// Whether the gutter is on screen at this width, docked or overlaid.
    pub fn shows(&self, side: GutterSide) -> bool {
        self.placement(side) != Placement::Hidden
    }

    pub fn both_docked(&self) -> bool {
        self.left == Placement::Docked && self.right == Placement::Docked
    }

    /// Width of the preview's text column: the window less every docked
    /// gutter and the preview's own padding. Strips and overlays take none.
    pub fn text_measure(&self) -> f32 {
        let docked: f32 = [GutterSide::Left, GutterSide::Right]
            .into_iter()
            .filter(|side| self.placement(*side) == Placement::Docked)
            .map(|side| self.width(side) + budget::gutter_overhead())
            .sum();
        (self.viewport.width - docked - budget::PREVIEW_PADDING).max(0.0)
    }

    /// The widest `side` may be dragged while the preview keeps its minimum.
    pub fn drag_max(&self, side: GutterSide) -> f32 {
        let other = self.both_docked().then(|| self.width(side.other()));
        budget::feasible_max(self.viewport.width, other)
    }
}

pub fn plan(viewport: Viewport, intent: PanelIntent) -> LayoutPlan {
    let width = viewport.width;
    let breakpoint = Breakpoint::from_width(width);
    let PanelIntent {
        left_open,
        right_open,
        active,
        left_width,
        right_width,
    } = intent;
    // A stored width below a side's floor (an old setting, or the default) is raised to it
    // before anything is budgeted, so the fit rules see the width the card will really take.
    let left_width = left_width.max(budget::min_width(GutterSide::Left));
    let right_width = right_width.max(budget::min_width(GutterSide::Right));

    // In a compact window Files becomes a strip that takes no width from the
    // preview, so it no longer competes with the right gutter for the space.
    let compact = breakpoint == Breakpoint::Compact;
    let (show_left, show_right) = if !compact
        && left_open
        && right_open
        && !budget::both_fit(
            viewport.allows_two_gutters(),
            width,
            left_width,
            right_width,
        ) {
        (active == GutterSide::Left, active == GutterSide::Right)
    } else {
        (left_open, right_open)
    };
    let left = match (show_left, breakpoint) {
        (false, _) => Placement::Hidden,
        (true, Breakpoint::Compact) => Placement::Strip,
        (true, _) => Placement::Docked,
    };
    let right = match (show_right, breakpoint) {
        (false, _) => Placement::Hidden,
        (true, Breakpoint::Compact) => Placement::Overlay,
        (true, _) => Placement::Docked,
    };

    // Rounded to a whole pixel even where nothing clamps it (both gutters docked, or an
    // overlay): a card and the toolbar's reserved padding are two independently laid-out trees
    // fed this same width, and letting a fractional one (from a drag, or a fractional window
    // size) reach them lets each round to a different device pixel at the seam between them.
    let fit = |stored: f32, min: f32, placement: Placement| match placement {
        Placement::Hidden | Placement::Strip => stored,
        Placement::Overlay => stored
            .min((width - budget::OVERLAY_MARGIN).max(0.0))
            .round(),
        Placement::Docked if show_left && show_right => stored.round(),
        Placement::Docked => {
            budget::effective_width(stored, budget::feasible_max(width, None), min)
        }
    };

    LayoutPlan {
        viewport,
        breakpoint,
        left,
        right,
        left_width: fit(left_width, budget::MIN_LEFT_WIDTH, left),
        right_width: fit(right_width, budget::MIN_RIGHT_WIDTH, right),
        toolbar: ToolbarDensity::from_width(width),
        menu: MenuLayout::from_width(width),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intent(left_open: bool, right_open: bool, active: GutterSide) -> PanelIntent {
        PanelIntent {
            left_open,
            right_open,
            active,
            left_width: budget::MIN_LEFT_WIDTH,
            right_width: 360.0,
        }
    }

    fn at(width: f32, intent: PanelIntent) -> LayoutPlan {
        plan(Viewport::new(width), intent)
    }

    #[test]
    fn text_measure_excludes_docked_gutters_only() {
        let both = at(1400.0, intent(true, true, GutterSide::Right));
        assert_eq!(
            both.text_measure(),
            1400.0 - budget::MIN_LEFT_WIDTH - 360.0 - 32.0 - 28.0
        );
        let none = at(1400.0, intent(false, false, GutterSide::Left));
        assert_eq!(none.text_measure(), 1372.0);
        let strip = at(600.0, intent(true, false, GutterSide::Left));
        assert_eq!(strip.left, Placement::Strip);
        assert_eq!(strip.text_measure(), 572.0);
    }

    #[test]
    fn wide_windows_dock_both_gutters() {
        let plan = at(1400.0, intent(true, true, GutterSide::Right));
        assert_eq!(
            (plan.left, plan.right),
            (Placement::Docked, Placement::Docked)
        );
        assert!(plan.both_docked());
    }

    #[test]
    fn two_gutters_stay_docked_just_below_the_threshold_after_a_resize() {
        // 1118 sits in the hold window (>= 1120 - 32) and still clears both_fit's own floor
        // (300 + 360 + 32 + 420 = 1112) with both gutters at their default widths.
        let held = Viewport::new(1200.0).resized(1118.0);
        let plan = super::plan(held, intent(true, true, GutterSide::Left));
        assert!(plan.both_docked());
        // The stateless viewport at that width has no history, so only one fits.
        assert!(!at(1118.0, intent(true, true, GutterSide::Left)).both_docked());
    }

    #[test]
    fn a_held_viewport_still_protects_the_preview() {
        let held = Viewport::new(1200.0).resized(1100.0);
        let mut wide = intent(true, true, GutterSide::Left);
        wide.right_width = 420.0;
        assert!(!super::plan(held, wide).both_docked());
    }

    #[test]
    fn medium_windows_dock_only_the_active_gutter() {
        let left = at(900.0, intent(true, true, GutterSide::Left));
        assert_eq!(
            (left.left, left.right),
            (Placement::Docked, Placement::Hidden)
        );
        let right = at(900.0, intent(true, true, GutterSide::Right));
        assert_eq!(
            (right.left, right.right),
            (Placement::Hidden, Placement::Docked)
        );
    }

    #[test]
    fn a_lone_open_gutter_shows_whichever_side_is_active() {
        let plan = at(900.0, intent(true, false, GutterSide::Right));
        assert_eq!(
            (plan.left, plan.right),
            (Placement::Docked, Placement::Hidden)
        );
        let plan = at(900.0, intent(false, true, GutterSide::Left));
        assert_eq!(
            (plan.left, plan.right),
            (Placement::Hidden, Placement::Docked)
        );
    }

    #[test]
    fn compact_windows_strip_files_and_overlay_the_right_gutter() {
        let both = at(600.0, intent(true, true, GutterSide::Right));
        assert_eq!(
            (both.left, both.right),
            (Placement::Strip, Placement::Overlay)
        );
        assert_eq!(both.right_width, 360.0);
        let files = at(600.0, intent(true, false, GutterSide::Right));
        assert_eq!(
            (files.left, files.right),
            (Placement::Strip, Placement::Hidden)
        );
        let right = at(200.0, intent(false, true, GutterSide::Left));
        assert_eq!(
            (right.left, right.right),
            (Placement::Hidden, Placement::Overlay)
        );
        assert_eq!(right.right_width, 180.0);
    }

    #[test]
    fn nothing_open_means_nothing_shown() {
        for width in [300.0, 900.0, 1400.0] {
            let plan = at(width, intent(false, false, GutterSide::Left));
            assert!(!plan.shows(GutterSide::Left) && !plan.shows(GutterSide::Right));
        }
    }

    #[test]
    fn wide_gutters_that_would_crush_the_preview_fall_back_to_one() {
        let mut wide = intent(true, true, GutterSide::Left);
        wide.right_width = 440.0;
        let plan = at(1120.0, wide);
        assert_eq!(
            (plan.left, plan.right),
            (Placement::Docked, Placement::Hidden)
        );
    }

    #[test]
    fn a_stored_width_shrinks_with_the_window_and_returns_when_it_grows() {
        let mut big = intent(true, false, GutterSide::Left);
        big.left_width = 480.0;
        let narrow = at(800.0, big);
        assert_eq!(narrow.left_width, 364.0); // 800 - 420 preview - 16 chrome
        assert_eq!(at(1400.0, big).left_width, 480.0);
    }

    #[test]
    fn drag_max_accounts_for_a_second_docked_gutter() {
        let plan = at(1120.0, intent(true, true, GutterSide::Left));
        assert_eq!(plan.drag_max(GutterSide::Right), 368.0); // 1120 - 420 preview - 32 chrome - 300 left
        let single = at(900.0, intent(true, true, GutterSide::Left));
        assert_eq!(single.drag_max(GutterSide::Left), 464.0);
    }

    /// Every docked gutter leaves the preview its minimum width.
    #[test]
    fn docked_layouts_always_preserve_the_preview() {
        let sides = [GutterSide::Left, GutterSide::Right];
        for width in (0..2000).step_by(10).map(|w| w as f32) {
            for (l, r) in [(true, true), (true, false), (false, true)] {
                for active in sides {
                    for (lw, rw) in [(180.0, 180.0), (260.0, 220.0), (480.0, 480.0)] {
                        let plan = at(
                            width,
                            PanelIntent {
                                left_open: l,
                                right_open: r,
                                active,
                                left_width: lw,
                                right_width: rw,
                            },
                        );
                        if plan.breakpoint == Breakpoint::Compact {
                            assert!(plan.left != Placement::Docked);
                            assert!(plan.right != Placement::Docked);
                            continue;
                        }
                        let docked: Vec<_> = sides
                            .into_iter()
                            .filter(|s| plan.placement(*s) == Placement::Docked)
                            .collect();
                        let used: f32 = docked
                            .iter()
                            .map(|s| plan.width(*s) + budget::gutter_overhead())
                            .sum();
                        // Gutters never fall below their minimum, so the guarantee only
                        // holds once the window can fit the wider one at its minimum.
                        if width
                            >= budget::MIN_PREVIEW_WIDTH
                                + budget::gutter_overhead()
                                + budget::MIN_RIGHT_WIDTH
                        {
                            assert!(
                                width - used >= budget::MIN_PREVIEW_WIDTH,
                                "width {width}, plan {plan:?}"
                            );
                        }
                    }
                }
            }
        }
    }
}
