// SPDX-License-Identifier: GPL-3.0-or-later
//! The breakpoint table: every window width at which the layout changes shape.
//! Add or move a threshold here, never as a literal in a view.

/// Below this width a gutter overlays the preview instead of docking beside it.
pub const OVERLAY_BELOW: f32 = 720.0;
/// From this width two gutters may dock at once, if the preview still fits.
pub const TWO_GUTTERS_FROM: f32 = 1120.0;
/// Once two gutters are docked they stay docked until the window is this much
/// narrower than [`TWO_GUTTERS_FROM`], so a window edge hovering at the
/// threshold does not make the second gutter flicker in and out. The preview's
/// minimum width is still enforced separately by the gutter budget.
pub const TWO_GUTTERS_HOLD: f32 = 32.0;
/// Below this width the toolbar stacks search and Open With on a second row.
pub const TOOLBAR_STACKED_BELOW: f32 = 560.0;
/// Below this width the toolbar moves search onto its own row.
pub const TOOLBAR_TWO_ROW_BELOW: f32 = 900.0;
/// Below this width the Open Recent submenu replaces the File menu.
pub const MENU_REPLACE_BELOW: f32 = 490.0;

/// Window size in logical pixels, as last reported by the window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub width: f32,
    pub height: f32,
    two_gutters: bool,
}

impl Viewport {
    /// A viewport with no history: two gutters are allowed from [`TWO_GUTTERS_FROM`].
    pub const fn new(width: f32) -> Self {
        Self {
            width,
            height: DEFAULT_HEIGHT,
            two_gutters: width >= TWO_GUTTERS_FROM,
        }
    }

    /// This viewport after the window reports `width`. Two gutters are allowed
    /// from [`TWO_GUTTERS_FROM`] upward, and keep being allowed down to
    /// [`TWO_GUTTERS_HOLD`] below it when they already were.
    pub fn resized(self, width: f32) -> Self {
        let held = self.two_gutters && width >= TWO_GUTTERS_FROM - TWO_GUTTERS_HOLD;
        Self {
            width,
            two_gutters: width >= TWO_GUTTERS_FROM || held,
            height: self.height,
        }
    }

    /// This viewport with the window height set; width rules are unaffected.
    pub fn with_height(self, height: f32) -> Self {
        Self { height, ..self }
    }

    /// Whether the window is wide enough to dock two gutters at once.
    pub fn allows_two_gutters(&self) -> bool {
        self.two_gutters
    }
}

/// The `iced` default window height, used until the window reports its size.
const DEFAULT_HEIGHT: f32 = 768.0;

impl Default for Viewport {
    /// The `iced` default window width, used until the window reports its size.
    fn default() -> Self {
        Self::new(1024.0)
    }
}

/// How gutters may be arranged at a width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Breakpoint {
    /// One gutter at most, overlaying the preview.
    Compact,
    /// One docked gutter at most.
    Medium,
    /// Two docked gutters when the preview still fits.
    Wide,
}

impl Breakpoint {
    pub fn from_width(width: f32) -> Self {
        if width < OVERLAY_BELOW {
            Self::Compact
        } else if width < TWO_GUTTERS_FROM {
            Self::Medium
        } else {
            Self::Wide
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarDensity {
    /// One row, with a fixed-width search field.
    Full,
    /// Search on a second row; actions stay in the first.
    TwoRow,
    /// Search and Open With share the second row.
    Stacked,
}

impl ToolbarDensity {
    pub fn from_width(width: f32) -> Self {
        if width < TOOLBAR_STACKED_BELOW {
            Self::Stacked
        } else if width < TOOLBAR_TWO_ROW_BELOW {
            Self::TwoRow
        } else {
            Self::Full
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuLayout {
    /// The Open Recent submenu opens beside the File menu.
    Cascade,
    /// The Open Recent submenu replaces the File menu.
    Replace,
}

impl MenuLayout {
    pub fn from_width(width: f32) -> Self {
        if width < MENU_REPLACE_BELOW {
            Self::Replace
        } else {
            Self::Cascade
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholds_are_lower_inclusive() {
        assert_eq!(Breakpoint::from_width(719.0), Breakpoint::Compact);
        assert_eq!(Breakpoint::from_width(720.0), Breakpoint::Medium);
        assert_eq!(Breakpoint::from_width(1119.0), Breakpoint::Medium);
        assert_eq!(Breakpoint::from_width(1120.0), Breakpoint::Wide);
        assert_eq!(ToolbarDensity::from_width(559.0), ToolbarDensity::Stacked);
        assert_eq!(ToolbarDensity::from_width(560.0), ToolbarDensity::TwoRow);
        assert_eq!(ToolbarDensity::from_width(899.0), ToolbarDensity::TwoRow);
        assert_eq!(ToolbarDensity::from_width(900.0), ToolbarDensity::Full);
        assert_eq!(MenuLayout::from_width(489.0), MenuLayout::Replace);
        assert_eq!(MenuLayout::from_width(490.0), MenuLayout::Cascade);
    }

    #[test]
    fn two_gutters_hold_below_the_threshold_once_shown() {
        let wide = Viewport::new(1200.0);
        assert!(wide.allows_two_gutters());
        let held = wide.resized(1100.0);
        assert!(held.allows_two_gutters());
        assert!(held.resized(1120.0 - TWO_GUTTERS_HOLD).allows_two_gutters());
        assert!(!held.resized(1119.0 - TWO_GUTTERS_HOLD).allows_two_gutters());
    }

    #[test]
    fn two_gutters_need_the_full_threshold_to_appear() {
        let narrow = Viewport::new(1000.0);
        assert!(!narrow.allows_two_gutters());
        assert!(!narrow.resized(1119.0).allows_two_gutters());
        assert!(narrow.resized(1120.0).allows_two_gutters());
    }

    #[test]
    fn a_window_hovering_at_the_threshold_does_not_flap() {
        let mut viewport = Viewport::new(1130.0);
        for width in [1118.0, 1122.0, 1116.0, 1124.0, 1110.0] {
            viewport = viewport.resized(width);
            assert!(viewport.allows_two_gutters(), "flapped at {width}");
        }
    }

    #[test]
    fn default_viewport_is_the_window_default() {
        assert_eq!(Viewport::default().width, 1024.0);
    }
}
