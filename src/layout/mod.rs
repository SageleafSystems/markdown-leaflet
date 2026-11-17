// SPDX-License-Identifier: GPL-3.0-or-later
//! Where panels sit at a given window width. Pure rules with no `iced`
//! dependency: [`plan`] turns a [`Viewport`] and the user's [`PanelIntent`]
//! into a [`LayoutPlan`] that the toolbar, menus, and panes all render from.

pub mod breakpoint;
pub mod budget;
pub mod plan;
pub mod spacing;
pub mod typography;

pub use breakpoint::{ToolbarDensity, Viewport};
pub use plan::{GutterSide, LayoutPlan, PanelIntent, Placement, plan};
