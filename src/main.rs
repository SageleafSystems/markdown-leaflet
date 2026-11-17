// SPDX-License-Identifier: GPL-3.0-or-later
mod actions;
mod app;
mod config;
mod domain;
mod gutter;
mod layout;
mod platform;
#[cfg(test)]
mod testutil;
mod ui;

fn main() -> iced::Result {
    app::run()
}
