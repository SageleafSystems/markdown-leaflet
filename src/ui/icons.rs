// SPDX-License-Identifier: GPL-3.0-or-later
//! A tiny, hand-authored set of line icons (one consistent 1.75px stroke,
//! round joins) used by the toolbar and lists instead of emoji. Each is an
//! inline SVG string rendered through [`iced::widget::svg`](mod@iced::widget::svg), which lets a
//! single shape be recolored per call via [`svg::Style::color`] — so the
//! same handle serves both the light and dark palette.
use iced::widget::svg;
use iced::{Color, Element, Length};

const STROKE_HEAD: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">"##;

/// Sidebar-toggle glyph: an outlined panel with a divider near its
/// leading edge, echoing macOS's `sidebar.leading` symbol.
pub const SIDEBAR: &str = r#"<rect x="3" y="4" width="18" height="16" rx="2.5"/><line x1="9.5" y1="4" x2="9.5" y2="20"/>"#;

/// Trailing panel glyph for the right gutter reveal button.
pub const SIDEBAR_RIGHT: &str = r#"<rect x="3" y="4" width="18" height="16" rx="2.5"/><line x1="14.5" y1="4" x2="14.5" y2="20"/>"#;

/// A heading and indented lines for the Outline tab.
pub const OUTLINE: &str = r#"<line x1="4" y1="5" x2="20" y2="5"/><line x1="4" y1="10" x2="14" y2="10"/><line x1="7" y1="15" x2="20" y2="15"/><line x1="7" y1="20" x2="16" y2="20"/>"#;

/// Magnifying glass for the search field.
pub const SEARCH: &str =
    r#"<circle cx="10.2" cy="10.2" r="6.2"/><line x1="19.5" y1="19.5" x2="15" y2="15"/>"#;

/// Circled "i" for the inspector toggle.
pub const INFO: &str = r##"<circle cx="12" cy="12" r="9"/><line x1="12" y1="11" x2="12" y2="16.5"/><circle cx="12" cy="7.6" r="0.9" fill="#000" stroke="none"/>"##;

/// Triangle exclamation, for the footer's frontmatter-parse-error note.
pub const WARNING: &str = r##"<path d="M12 3.5 21.5 20H2.5Z"/><line x1="12" y1="9.5" x2="12" y2="14.5"/><circle cx="12" cy="17.2" r="0.9" fill="#000" stroke="none"/>"##;

/// Square-with-arrow "share" glyph.
pub const SHARE: &str = r#"<path d="M5 13v4a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2v-4"/><polyline points="8 7 12 3 16 7"/><line x1="12" y1="3" x2="12" y2="15"/>"#;

/// Pencil glyph for the "Open With" editor switcher.
pub const EDIT: &str =
    r#"<path d="M12 20h9"/><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4Z"/>"#;

/// Page/file glyph for list rows (outline entries, file-list rows).
pub const FILE: &str = r#"<path d="M14 2.5H7a2 2 0 0 0-2 2v15a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8Z"/><polyline points="14 2.5 14 8 19.5 8"/>"#;

/// Folder and disclosure glyphs for the file tree.
pub const FOLDER: &str =
    r#"<path d="M3 6a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z"/>"#;
/// Clock face with its sweep drawn as a counter-clockwise arrow, for the Recent tab in the left
/// gutter: a clock face reads as "time", but the backward arrow is what reads as "recent" rather
/// than, say, a timer or an alarm.
pub const RECENT: &str = r#"<circle cx="12" cy="12" r="9"/><path d="M12 6.5a5.5 5.5 0 1 0 4.1 1.85"/><polyline points="17.2 4.8 16.9 8.6 13.1 8"/>"#;

/// A capital T on a baseline, for the preview font menu.
pub const FONT: &str = r#"<polyline points="5 8 5 5 19 5 19 8"/><line x1="12" y1="5" x2="12" y2="19"/><line x1="9" y1="19" x2="15" y2="19"/>"#;

/// A page outline with a bar and a viewport box beside it, for the minimap toggle.
pub const MINIMAP: &str = r##"<rect x="3" y="4" width="18" height="16" rx="2"/><line x1="15" y1="4" x2="15" y2="20"/><line x1="6" y1="9" x2="12" y2="9"/><line x1="6" y1="13" x2="11" y2="13"/><rect x="17" y="8" width="2" height="5" fill="#000" stroke="none"/>"##;

/// A circle half filled, for the theme cycle button.
pub const THEME: &str =
    r##"<circle cx="12" cy="12" r="9"/><path d="M12 3a9 9 0 0 1 0 18Z" fill="#000"/>"##;

/// A small filled corner triangle, marking a button whose right-click opens a dropdown.
pub const CORNER_MARK: &str = r##"<path d="M22 8V22H8Z" fill="#000" stroke="none"/>"##;

pub const CHEVRON_LEFT: &str = r#"<polyline points="15 6 9 12 15 18"/>"#;
pub const CHEVRON_RIGHT: &str = r#"<polyline points="9 6 15 12 9 18"/>"#;
pub const CHEVRON_DOWN: &str = r#"<polyline points="6 9 12 15 18 9"/>"#;
pub const CHEVRON_UP: &str = r#"<polyline points="6 15 12 9 18 15"/>"#;
pub const CLOSE: &str =
    r#"<line x1="6" y1="6" x2="18" y2="18"/><line x1="18" y1="6" x2="6" y2="18"/>"#;

/// A plus, for the nav header's New button.
pub const ADD: &str =
    r#"<line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>"#;

/// Two overlapping sheets, for Copy.
pub const COPY: &str =
    r#"<rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15V6a2 2 0 0 1 2-2h8"/>"#;

/// A dashed frame with a check corner, for Select All.
pub const SELECT_ALL: &str = r#"<rect x="4" y="4" width="16" height="16" rx="2" stroke-dasharray="3 3"/><polyline points="8.5 12.5 11 15 15.5 9.5"/>"#;

/// Box with an arrow leaving its corner, for Open Link.
pub const EXTERNAL: &str = r#"<path d="M18 13v5a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h5"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/>"#;

/// Two chain links, for Copy Link Address.
pub const LINK: &str = r#"<path d="M10 14a4.5 4.5 0 0 0 6.4 0l3-3a4.5 4.5 0 0 0-6.4-6.4l-1 1"/><path d="M14 10a4.5 4.5 0 0 0-6.4 0l-3 3a4.5 4.5 0 0 0 6.4 6.4l1-1"/>"#;

/// Picture frame with a sun and hills, for Open Image.
pub const IMAGE: &str = r#"<rect x="3" y="4" width="18" height="16" rx="2.5"/><circle cx="8.5" cy="9.5" r="1.5"/><polyline points="21 16 15.5 11 6 20"/>"#;

/// Angle brackets, for Copy Markdown Source.
pub const CODE: &str = r#"<polyline points="8 7 3 12 8 17"/><polyline points="16 7 21 12 16 17"/>"#;

/// Builds a themed, fixed-size icon element from one of the fragments above.
pub fn svg_icon<'a, Message: 'a>(fragment: &str, color: Color, size: f32) -> Element<'a, Message> {
    let markup = format!("{STROKE_HEAD}{fragment}</svg>");
    svg(svg::Handle::from_memory(markup.into_bytes()))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(move |_theme, _status| svg::Style { color: Some(color) })
        .into()
}

/// A trash can, for moving a file to the Trash.
pub const TRASH: &str = r#"<polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6"/><line x1="10" y1="11" x2="10" y2="17"/><line x1="14" y1="11" x2="14" y2="17"/><path d="M9 6V4a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2"/>"#;
