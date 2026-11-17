// SPDX-License-Identifier: GPL-3.0-or-later
//! Operating-system integration: dialogs, opening links, the native menu bar.

pub mod dialogs;
#[cfg(target_os = "macos")]
pub mod macos_open;
#[cfg(target_os = "macos")]
pub mod native_menu;
pub mod opener;
