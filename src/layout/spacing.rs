// SPDX-License-Identifier: GPL-3.0-or-later
//! The spacing scale. Every gap, margin, and inset in the layout is a multiple of [`HALF`]
//! (4 px), and the structural gaps (the well around a gutter card, the space between the
//! preview's scrollbar and a card) are one [`UNIT`] (8 px). Name a size here instead of writing a
//! pixel literal, so two gaps that should match cannot drift apart.
//!
//! Menus ([`menu`]) and file/heading lists ([`list`]) are the two exceptions: their rows are
//! deliberately dense, so each keeps its own named set of sizes, off the grid where the tighter
//! rhythm reads better, and every menu or list takes its spacing from that set.

/// The base spacing unit, in logical pixels.
pub const UNIT: f32 = 8.0;
/// Half a unit: the finest step on the grid.
pub const HALF: f32 = UNIT / 2.0;

/// `count` base units.
pub const fn units(count: f32) -> f32 {
    UNIT * count
}

/// Gap between sibling controls, and between a control and its icon or label.
pub const GAP: f32 = UNIT;
/// Gap between tightly related controls, such as the Back and Forward buttons.
pub const GAP_TIGHT: f32 = HALF;
/// Gap between groups of related content, such as the rows of the inspector.
pub const GAP_SECTION: f32 = units(2.0);
/// Vertical padding inside a toolbar-height control: an icon button, pill, or tab.
pub const CONTROL_PAD_V: f32 = UNIT;
/// Horizontal padding inside a toolbar-height control with a text label.
pub const CONTROL_PAD_H: f32 = units(1.5);
/// Border width of the well behind the toolbar's left icon cluster.
pub const WELL_BORDER: f32 = 1.0;
/// Padding between that well's border and its buttons: deliberately tight, so off the grid.
pub const WELL_PAD: f32 = 2.0;
/// How far the well extends beyond its buttons on every side. Callers shave this much off the
/// padding around the well so the buttons themselves stay put.
pub const WELL_INSET: f32 = WELL_BORDER + WELL_PAD;
/// The toolbar's search field at rest, in a full-width toolbar.
pub const SEARCH_WIDTH: f32 = units(27.5);
/// The widest the toolbar's document title grows.
pub const TITLE_MAX_WIDTH: f32 = units(30.0);
/// Space between a tooltip and the control it describes.
pub const TOOLTIP_GAP: f32 = UNIT;
/// Padding inside a gutter card.
pub const CARD_PAD: f32 = units(1.5);
/// Padding around the footer's labels.
pub const FOOTER_PAD: [f32; 2] = [HALF, units(2.0)];

/// Sizes for in-window menus, their bar, and context menus. These are tighter than the grid.
pub mod menu {
    /// Gap between rows in a menu panel.
    pub const ROW_GAP: f32 = 1.0;
    /// Padding inside a menu row: vertical, horizontal.
    pub const ROW_PAD: [f32; 2] = [6.0, 8.0];
    /// Padding between a menu panel's edge and its rows.
    pub const PANEL_PAD: f32 = 5.0;
    /// Padding around a divider line between menu groups.
    pub const DIVIDER_PAD: [f32; 2] = [4.0, 8.0];
    // The in-window menu bar exists only off macOS, which has a native one.

    /// Gap between the menu bar's triggers.
    #[cfg(not(target_os = "macos"))]
    pub const BAR_GAP: f32 = 2.0;
    /// Padding inside one menu bar trigger.
    #[cfg(not(target_os = "macos"))]
    pub const TRIGGER_PAD: [f32; 2] = [5.0, 10.0];
    /// Padding around the menu bar.
    #[cfg(not(target_os = "macos"))]
    pub const BAR_PAD: [f32; 2] = [6.0, 12.0];
}

/// Sizes for the file tree, outline, and file chips. These are tighter than the grid.
pub mod list {
    /// Padding between a list card's edge and its rows. Rows carry their own 8 px, so this keeps
    /// the highlight one unit from the card and the content two.
    pub const CARD_PAD: f32 = super::UNIT;
    /// Gap between rows.
    pub const ROW_GAP: f32 = 2.0;
    /// Padding inside a file or folder row: vertical, horizontal.
    pub const FILE_ROW_PAD: [f32; 2] = [4.0, 8.0];
    /// Padding inside an outline heading row.
    pub const HEADING_ROW_PAD: [f32; 2] = [7.0, 8.0];
    /// Indent per folder level in the file tree.
    pub const TREE_INDENT: f32 = super::units(2.0);
    /// Gap between rows of the file tree, so neighbouring highlights stay distinct.
    pub const TREE_ROW_GAP: f32 = super::HALF;
    /// Width of a folder's chevron, and the blank slot that keeps a file's icon in line with
    /// the folder icons beside it.
    pub const CHEVRON_SLOT: f32 = 12.0;
    /// Width of the file or folder icon column; the file icon is centred in it.
    pub const ICON_SLOT: f32 = super::units(2.0);
    /// Padding on each side of an outline nesting guide, so a level indents by this twice plus
    /// the guide's 1 px line.
    pub const HEADING_GUIDE_PAD: f32 = 6.0;
    /// Padding inside one chip in the compact file strip.
    pub const CHIP_PAD: [f32; 2] = [5.0, 10.0];
    /// Padding around the compact file strip.
    pub const STRIP_PAD: [f32; 2] = [6.0, 12.0];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scale_is_built_on_a_four_pixel_grid() {
        assert_eq!(UNIT, 8.0);
        assert_eq!(HALF, 4.0);
        assert_eq!(units(1.5), 12.0);
        assert_eq!(units(2.0), 16.0);
    }

    #[test]
    fn every_non_exempt_size_is_a_multiple_of_the_half_unit() {
        let sizes = [
            GAP,
            GAP_TIGHT,
            GAP_SECTION,
            CONTROL_PAD_V,
            CONTROL_PAD_H,
            TOOLTIP_GAP,
            CARD_PAD,
            FOOTER_PAD[0],
            FOOTER_PAD[1],
        ];
        for size in sizes {
            assert_eq!(size % HALF, 0.0, "{size} is off the 4 px grid");
        }
    }
}
