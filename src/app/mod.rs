// SPDX-License-Identifier: GPL-3.0-or-later
//! The `iced` application: state, messages, transitions (`update`), and rendering (`view`).

use crate::layout::spacing;
use crate::ui::theme;

const SEARCH_INPUT: &str = "folder-search";
const FIND_INPUT: &str = "document-find";
const NEW_ITEM_INPUT: &str = "new-item-name";
const RELOAD_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(600);
const FOLDER_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(3);
const NAV_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(160);
const ICON_SIZE: f32 = 16.0;
/// Multiplier for toolbar, navigation, menus, and panel text. Markdown content keeps its own scale.
const CHROME_FONT_SCALE: f32 = 1.12;
const fn chrome_size(base: f32) -> f32 {
    base * CHROME_FONT_SCALE
}

/// Height every control in the toolbar's primary row, and in a gutter's header row, is fixed to
/// — an icon button's own height. Controls there share `CONTROL_PAD_V`, but the ones with
/// taller intrinsic content (the search field, a `pick_list`) would otherwise settle to their
/// own, taller natural height, which is what let the search pill poke past the toolbar's border
/// and look vertically off against its neighbors. Fixing every control to this one height,
/// rather than leaving each to its own tallest content, is also what keeps a control that moves
/// between the toolbar and a header (the gutter toggle) landing at the identical pixel either
/// way: the two rows can use `Alignment::Start` (see `State::toolbar`) and still match exactly,
/// because nothing in either is ever taller than this.
const TOOLBAR_CONTROL_HEIGHT: f32 = ICON_SIZE + spacing::CONTROL_PAD_V * 2.0;

/// How long the search field takes to expand over the title, and to settle back.
const SEARCH_EXPAND_DURATION: std::time::Duration = std::time::Duration::from_millis(180);
const SEARCH_COLLAPSE_DURATION: std::time::Duration = std::time::Duration::from_millis(180);

mod message;
mod state;
mod subscription;
mod update;
mod view;

pub(crate) use message::*;
pub(crate) use state::*;

pub fn run() -> iced::Result {
    let icon = app_icon();

    let mut app = iced::application(boot, State::update, State::view)
        .title(State::title)
        .theme(|state: &State| theme::theme(state.appearance))
        .subscription(State::subscription)
        .window(window_settings(icon));
    for face in crate::ui::fonts::BUNDLED {
        app = app.font(face);
    }
    app.run()
}

/// Wayland has no window-icon API (winit only applies `icon` on X11), so
/// compositors there resolve the taskbar icon by matching an app id against
/// an installed `.desktop` file's basename instead. `PlatformSpecific` only
/// carries an `application_id` field on Linux, hence the `cfg`.
fn window_settings(icon: iced::window::Icon) -> iced::window::Settings {
    let settings = iced::window::Settings {
        icon: Some(icon),
        ..Default::default()
    };
    #[cfg(target_os = "linux")]
    let settings = iced::window::Settings {
        platform_specific: iced::window::settings::PlatformSpecific {
            application_id: "markdown-leaflet".to_string(),
            ..settings.platform_specific
        },
        ..settings
    };
    settings
}

/// Builds the initial state and asks the window for its size, which `iced`
/// does not report until the first resize.
fn boot() -> (State, iced::Task<Message>) {
    let mut state = State::new();
    let open = State::startup_request()
        .map_or_else(iced::Task::none, |request| state.request_open(request));
    let size = iced::window::oldest()
        .and_then(iced::window::size)
        .map(Message::WindowResized);
    (state, iced::Task::batch([open, size]))
}

// The icon is a compile-time asset; a decode failure is a build defect, caught by the test below.
#[allow(clippy::expect_used)]
fn app_icon() -> iced::window::Icon {
    let png = include_bytes!("../../assets/brand/markdown-leaflet-icon.png");
    let decoder = png::Decoder::new(std::io::Cursor::new(png));
    let mut reader = decoder
        .read_info()
        .expect("embedded app icon should decode");
    let mut pixels = vec![
        0;
        reader
            .output_buffer_size()
            .expect("embedded app icon should report an output buffer size")
    ];
    let frame = reader
        .next_frame(&mut pixels)
        .expect("embedded app icon should have a frame");
    assert_eq!(frame.color_type, png::ColorType::Rgba);
    assert_eq!(frame.bit_depth, png::BitDepth::Eight);
    iced::window::icon::from_rgba(
        pixels[..frame.buffer_size()].to_vec(),
        frame.width,
        frame.height,
    )
    .expect("embedded app icon should contain valid RGBA pixels")
}

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_app_icon_decodes() {
        let (pixels, size) = super::app_icon().into_raw();
        assert_eq!((size.width, size.height), (1024, 1024));
        assert_eq!(pixels.len(), 1024 * 1024 * 4);
    }
}
