// SPDX-License-Identifier: GPL-3.0-or-later
//! User-triggered menu commands shared by the in-window and native menus.

use crate::config::{AppearanceChoice, FontChoice};
use crate::domain::recent::RecentItem;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuAction {
    About,
    Quit,
    OpenFile,
    OpenFolder,
    Recent(RecentItem),
    CloseCurrent,
    ShowFiles,
    ShowOutline,
    ShowDetails,
    ToggleMinimap,
    ToggleSourceView,
    Appearance(AppearanceChoice),
    PreviewFont(FontChoice),
    ReloadTheme,
    /// Show the external-link dialog again instead of applying a remembered choice.
    AskAboutExternalLinks,
    Shortcuts,
    ZoomIn,
    ZoomOut,
    ResetZoom,
}

impl std::fmt::Display for MenuAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::About => "About Markdown Leaflet",
            Self::Quit => "Quit",
            Self::OpenFile => "Open File…",
            Self::OpenFolder => "Open Folder…",
            Self::Recent(item) => return f.write_str(&item.compact_label()),
            Self::CloseCurrent => "Close Current",
            Self::ShowFiles => "Show Files",
            Self::ShowOutline => "Show Outline",
            Self::ShowDetails => "Show Details",
            Self::ToggleMinimap => "Show Minimap",
            Self::ToggleSourceView => "View Source",
            Self::Appearance(AppearanceChoice::System) => "Appearance: System",
            Self::Appearance(AppearanceChoice::Slate) => "Appearance: Slate",
            Self::Appearance(AppearanceChoice::Harbor) => "Appearance: Harbor",
            Self::Appearance(AppearanceChoice::Nightfall) => "Appearance: Nightfall",
            Self::Appearance(AppearanceChoice::Linen) => "Appearance: Linen",
            Self::Appearance(AppearanceChoice::Paper) => "Appearance: Paper",
            Self::PreviewFont(FontChoice::System) => "Preview Font: System",
            Self::PreviewFont(FontChoice::Sans) => "Preview Font: Sans",
            Self::PreviewFont(FontChoice::Serif) => "Preview Font: Serif",
            Self::PreviewFont(FontChoice::Mono) => "Preview Font: Mono",
            Self::ReloadTheme => "Reload Theme",
            Self::AskAboutExternalLinks => "Ask About External Links",
            Self::Shortcuts => "Keyboard Shortcuts",
            Self::ZoomIn => "Zoom In",
            Self::ZoomOut => "Zoom Out",
            Self::ResetZoom => "Reset Zoom",
        };
        f.write_str(label)
    }
}
