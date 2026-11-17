// SPDX-License-Identifier: GPL-3.0-or-later
//! Rules that run without an `iced` window: documents, folders, scanning, and recents.
//! `document` and `outline` use `iced::widget::markdown` item types by decision.

pub mod create;
pub mod document;
pub mod editors;
pub mod find;
pub mod folder;
pub mod frontmatter;
pub mod history;
pub mod images;
pub mod keymap;
pub mod links;
pub mod minimap;
pub mod outline;
pub mod recent;
pub mod selection;
pub mod stats;
pub mod tasks;
pub mod walk;
pub mod workspace;
