// SPDX-License-Identifier: GPL-3.0-or-later
//! Right-click menus: what each target offers, and dispatching a choice.
//!
//! A new menu is a `ContextTarget` variant plus a builder below; views open
//! it by wrapping an element in `ui::context_area::ContextArea`.

use crate::actions::MenuAction;
use crate::app::{ContextEntry, ContextMenu, ContextTarget, Message, State};
use crate::config::{AppearanceChoice, FontChoice};
use crate::domain::create;
use crate::domain::editors::EDITORS;
use crate::domain::stats::StatsFormat;
use crate::ui::{anchor_area, icons};
use iced::Task;
use iced::widget::markdown;
use std::path::PathBuf;

impl State {
    pub(crate) fn update_context_menu(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ContextLinkPressed(uri) => {
                self.menus.context_link = Some(uri);
                Task::none()
            }
            Message::ContextImagePressed(path) => {
                self.menus.context_image = Some(path);
                Task::none()
            }
            Message::OpenContextMenu(target, position) => {
                let link = self.menus.context_link.take();
                let image = self.menus.context_image.take();
                let root = self.root().map(PathBuf::from);
                let in_nav = matches!(
                    target,
                    ContextTarget::NavFile(_)
                        | ContextTarget::NavFolder(_)
                        | ContextTarget::NavBlank
                );
                let mut entries = match (target, image) {
                    (ContextTarget::NavFile(path), _) => file_entries(path),
                    (ContextTarget::NavFolder(path), _) => {
                        let blocker = create::folder_blocker(&path);
                        folder_entries(path, blocker)
                    }
                    (ContextTarget::NavBlank, _) => match root {
                        Some(root) => new_entries(root),
                        None => return Task::none(),
                    },
                    (ContextTarget::Preview, Some(path)) => image_entries(path),
                    (ContextTarget::Preview, None) => preview_entries(
                        link,
                        self.selected_text().is_some(),
                        !self.leaves.is_empty(),
                    ),
                };
                // Every menu in the file nav can turn text files on or off (issue #79).
                if in_nav {
                    entries.push(ContextEntry::Divider);
                    entries.push(show_text_entry(self.config.files.show_text));
                }
                self.menus.context = Some(ContextMenu {
                    position,
                    anchor_right: None,
                    entries,
                });
                Task::none()
            }
            Message::OpenFileOrFolderDropdown(bounds) => {
                self.menus.context = Some(dropdown(bounds, open_entries()));
                Task::none()
            }
            Message::OpenNewDropdown(bounds) => {
                if let Some(root) = self.root().map(PathBuf::from) {
                    self.menus.context = Some(dropdown(bounds, new_entries(root)));
                }
                Task::none()
            }
            Message::OpenFontDropdown(bounds) => {
                self.menus.context = Some(dropdown(bounds, font_entries(self.config.preview.font)));
                Task::none()
            }
            Message::OpenThemeDropdown(bounds) => {
                self.menus.context = Some(dropdown(
                    bounds,
                    theme_entries(self.config.theme.appearance),
                ));
                Task::none()
            }
            Message::OpenEditorDropdown(bounds) => {
                self.menus.context = Some(dropdown(bounds, editor_entries()));
                Task::none()
            }
            Message::OpenMetricsFormatDropdown(bounds) => {
                self.menus.context = Some(dropdown(
                    bounds,
                    metrics_format_entries(self.menus.stats_format),
                ));
                Task::none()
            }
            Message::ContextMenuPick(index) => {
                let chosen = self.menus.context.take().and_then(|menu| {
                    match menu.entries.into_iter().nth(index)? {
                        ContextEntry::Item { message, .. } => message,
                        ContextEntry::Divider => None,
                    }
                });
                match chosen {
                    Some(message) => self.update(message),
                    None => Task::none(),
                }
            }
            Message::ContextMenuInert => Task::none(),
            Message::DismissContextMenu => {
                self.menus.context = None;
                self.menus.context_link = None;
                self.menus.context_image = None;
                Task::none()
            }
            _ => unreachable!("context menu messages are routed by State::update"),
        }
    }
}

fn item(
    label: &'static str,
    icon: &'static str,
    shortcut: Option<&'static str>,
    message: Option<Message>,
) -> ContextEntry {
    ContextEntry::Item {
        label: label.into(),
        icon: Some(icon),
        shortcut,
        checked: false,
        message,
        info: None,
    }
}

/// The "Show Text Files" row at the foot of the nav menus (issue #79): checked, and reading
/// "Hide Text Files", while text files are listed.
fn show_text_entry(shown: bool) -> ContextEntry {
    ContextEntry::Item {
        label: if shown {
            "Hide Text Files (.txt)"
        } else {
            "Show Text Files (.txt)"
        }
        .into(),
        // No icon: a row only draws its checkmark in the icon's place.
        icon: None,
        shortcut: None,
        checked: shown,
        message: Some(Message::ToggleShowTextFiles),
        info: None,
    }
}

/// Rows that create something new in `dir`.
fn new_entries(dir: PathBuf) -> Vec<ContextEntry> {
    vec![
        item(
            "New Markdown File",
            icons::FILE,
            None,
            Some(Message::NewMarkdownFile(dir.clone())),
        ),
        item(
            "New Folder",
            icons::FOLDER,
            None,
            Some(Message::NewFolder(dir)),
        ),
    ]
}

fn reveal_label() -> &'static str {
    if cfg!(target_os = "macos") {
        "Show in Finder"
    } else if cfg!(target_os = "windows") {
        "Show in Explorer"
    } else {
        "Show in File Manager"
    }
}

/// A Rename or Trash row that is disabled, with an explanation, when `blocker` holds one.
fn change_item(
    label: &'static str,
    icon: &'static str,
    shortcut: &'static str,
    message: Message,
    blocker: Option<&String>,
) -> ContextEntry {
    ContextEntry::Item {
        label: label.into(),
        icon: Some(icon),
        shortcut: Some(shortcut),
        checked: false,
        message: blocker.is_none().then_some(message),
        info: blocker.cloned().map(Into::into),
    }
}

/// The menu for a right-clicked folder row. Renaming and trashing it are offered only when it
/// holds nothing but Markdown; `blocker` says why not.
fn folder_entries(path: PathBuf, blocker: Option<String>) -> Vec<ContextEntry> {
    let mut entries = new_entries(path.clone());
    entries.extend([
        ContextEntry::Divider,
        item(
            reveal_label(),
            icons::FOLDER,
            None,
            Some(Message::RevealInFileManager(path.clone())),
        ),
        item(
            "Copy Path",
            icons::COPY,
            None,
            Some(Message::CopyPath(path.clone())),
        ),
        ContextEntry::Divider,
        change_item(
            "Rename…",
            icons::EDIT,
            "F2",
            Message::RenameItem(path.clone()),
            blocker.as_ref(),
        ),
        change_item(
            "Move to Trash",
            icons::TRASH,
            "Delete",
            Message::DeleteItem(path),
            blocker.as_ref(),
        ),
    ]);
    entries
}

/// The menu for a right-clicked file row.
fn file_entries(path: PathBuf) -> Vec<ContextEntry> {
    vec![
        item(
            reveal_label(),
            icons::FOLDER,
            None,
            Some(Message::RevealInFileManager(path.clone())),
        ),
        ContextEntry::Divider,
        item(
            "Open in New Window",
            icons::SIDEBAR,
            None,
            Some(Message::OpenInNewWindow(path.clone())),
        ),
        item(
            "Open with Default App",
            icons::EXTERNAL,
            None,
            Some(Message::OpenExternally(path.clone())),
        ),
        ContextEntry::Divider,
        item(
            "Rename…",
            icons::EDIT,
            Some("F2"),
            Some(Message::RenameItem(path.clone())),
        ),
        item(
            "Copy Path",
            icons::COPY,
            None,
            Some(Message::CopyPath(path.clone())),
        ),
        ContextEntry::Divider,
        item(
            "Move to Trash",
            icons::TRASH,
            Some("Delete"),
            Some(Message::DeleteItem(path)),
        ),
    ]
}

/// The combined Open button's right-click dropdown: the two choices its left-click collapses.
fn open_entries() -> Vec<ContextEntry> {
    vec![
        ContextEntry::Item {
            label: "Open File…".into(),
            icon: Some(icons::FILE),
            shortcut: None,
            checked: false,
            message: Some(Message::OpenFileClicked),
            info: None,
        },
        ContextEntry::Item {
            label: "Open Folder…".into(),
            icon: Some(icons::FOLDER),
            shortcut: None,
            checked: false,
            message: Some(Message::OpenFolderClicked),
            info: None,
        },
    ]
}

/// A menu hung beneath the control at `bounds`.
fn dropdown(bounds: iced::Rectangle, entries: Vec<ContextEntry>) -> ContextMenu {
    ContextMenu {
        position: anchor_area::dropdown_origin(bounds),
        anchor_right: Some(bounds.x + bounds.width),
        entries,
    }
}

/// The toolbar's Open With dropdown: one row per editor.
fn editor_entries() -> Vec<ContextEntry> {
    EDITORS
        .iter()
        .map(|editor| ContextEntry::Item {
            label: editor.name.into(),
            icon: None,
            shortcut: None,
            checked: false,
            message: Some(Message::OpenWith(editor.clone())),
            info: None,
        })
        .collect()
}

/// The toolbar's Font dropdown: one row per preview font, the current one checked.
fn font_entries(current: FontChoice) -> Vec<ContextEntry> {
    FontChoice::ALL
        .into_iter()
        .map(|choice| ContextEntry::Item {
            label: format!("{} · {}", choice.label(), choice.family()).into(),
            icon: None,
            shortcut: None,
            checked: choice == current,
            message: Some(Message::MenuAction(MenuAction::PreviewFont(choice))),
            info: None,
        })
        .collect()
}

/// The toolbar's Theme dropdown: one row per appearance, the current one checked.
fn theme_entries(current: AppearanceChoice) -> Vec<ContextEntry> {
    AppearanceChoice::ALL
        .into_iter()
        .map(|choice| ContextEntry::Item {
            label: choice.label().into(),
            icon: None,
            shortcut: None,
            checked: choice == current,
            message: Some(Message::MenuAction(MenuAction::Appearance(choice))),
            info: None,
        })
        .collect()
}

/// The details panel's "Copy All" dropdown: one row per shape the metrics can be copied as, the
/// current one checked, matching the Theme and Font dropdowns.
fn metrics_format_entries(current: StatsFormat) -> Vec<ContextEntry> {
    StatsFormat::ALL
        .into_iter()
        .map(|format| ContextEntry::Item {
            label: format.label().into(),
            icon: Some(icons::CODE),
            shortcut: None,
            checked: format == current,
            message: Some(Message::CopyAllStats(format)),
            info: None,
        })
        .collect()
}

/// The menu for a right-clicked image.
fn image_entries(path: PathBuf) -> Vec<ContextEntry> {
    vec![
        item(
            "Open Image",
            icons::IMAGE,
            None,
            Some(Message::OpenImage(path.clone())),
        ),
        item(
            "Copy Image Path",
            icons::COPY,
            None,
            Some(Message::CopyImagePath(path)),
        ),
        ContextEntry::Divider,
        item(
            "Copy Markdown Source",
            icons::CODE,
            None,
            Some(Message::CopySource),
        ),
    ]
}

/// The preview's menu. Selection items are disabled without a selection;
/// link items appear only when a link was right-clicked.
fn preview_entries(
    link: Option<markdown::Uri>,
    has_selection: bool,
    has_text: bool,
) -> Vec<ContextEntry> {
    let mut entries = vec![
        item(
            "Copy",
            icons::COPY,
            Some("Ctrl/⌘C"),
            has_selection.then_some(Message::CopySelection),
        ),
        item(
            "Select All",
            icons::SELECT_ALL,
            Some("Ctrl/⌘A"),
            has_text.then_some(Message::SelectAll),
        ),
        item(
            "Find Selection",
            icons::SEARCH,
            Some("Ctrl/⌘E"),
            has_selection.then_some(Message::FindSelection),
        ),
    ];
    if let Some(link) = link {
        entries.extend([
            ContextEntry::Divider,
            item(
                "Open Link",
                icons::EXTERNAL,
                None,
                Some(Message::LinkClicked(link.clone())),
            ),
            item(
                "Copy Link Address",
                icons::LINK,
                None,
                Some(Message::CopyLink(link)),
            ),
        ]);
    }
    entries.extend([
        ContextEntry::Divider,
        item(
            "Copy Markdown Source",
            icons::CODE,
            None,
            Some(Message::CopySource),
        ),
    ]);
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nav_menus_offer_the_issue_38_actions() {
        let dir = || PathBuf::from("/a/docs");
        assert_eq!(
            labels(&new_entries(dir())),
            vec![("New Markdown File", true), ("New Folder", true)]
        );
        let folder_menu = folder_entries(dir(), None);
        let folder = labels(&folder_menu);
        assert_eq!(folder.len(), 8);
        assert_eq!(folder[0].0, "New Markdown File");
        assert_eq!(folder[4], ("Copy Path", true));
        assert_eq!(folder[6], ("Rename…", true));
        assert_eq!(folder[7], ("Move to Trash", true));
        let file_menu = file_entries(PathBuf::from("/a/docs/x.md"));
        let file = labels(&file_menu);
        assert_eq!(
            file.iter().map(|(label, _)| *label).collect::<Vec<_>>(),
            [
                reveal_label(),
                "—",
                "Open in New Window",
                "Open with Default App",
                "—",
                "Rename…",
                "Copy Path",
                "—",
                "Move to Trash"
            ]
        );
    }

    #[test]
    fn a_folder_with_other_files_gets_disabled_rename_and_trash_with_an_explanation() {
        let menu = folder_entries(PathBuf::from("/a/docs"), Some("because".into()));
        let labelled = labels(&menu);
        assert_eq!(labelled[6], ("Rename…", false));
        assert_eq!(labelled[7], ("Move to Trash", false));
        for entry in &menu[6..] {
            assert!(matches!(
                entry,
                ContextEntry::Item { info: Some(text), .. } if text == "because"
            ));
        }
        // Creating and revealing stay available.
        assert_eq!(labelled[0], ("New Markdown File", true));
        assert!(labelled[3].1);
    }

    #[test]
    fn right_clicking_nav_blank_space_needs_an_open_folder() {
        let mut state = State::blank();
        let _ = state.update(Message::OpenContextMenu(
            ContextTarget::NavBlank,
            iced::Point::ORIGIN,
        ));
        assert!(state.menus.context.is_none());
    }

    #[test]
    fn font_dropdown_lists_every_font_and_checks_only_the_current_one() {
        let entries = font_entries(FontChoice::Serif);
        assert_eq!(entries.len(), FontChoice::ALL.len());
        let checked: Vec<_> = entries
            .iter()
            .filter_map(|entry| match entry {
                ContextEntry::Item { label, checked, .. } if *checked => Some(label.as_ref()),
                _ => None,
            })
            .collect();
        assert_eq!(checked, ["Serif · Source Serif 4"]);
    }

    #[test]
    fn theme_dropdown_lists_every_theme_and_checks_only_the_current_one() {
        let entries = theme_entries(AppearanceChoice::Nightfall);
        assert_eq!(entries.len(), AppearanceChoice::ALL.len());
        let checked: Vec<_> = entries
            .iter()
            .filter_map(|entry| match entry {
                ContextEntry::Item { label, checked, .. } if *checked => Some(label.as_ref()),
                _ => None,
            })
            .collect();
        assert_eq!(checked, ["Nightfall"]);
    }

    #[test]
    fn metrics_format_menu_lists_every_shape_and_checks_only_the_current_one() {
        let entries = metrics_format_entries(StatsFormat::Yaml);
        assert_eq!(entries.len(), StatsFormat::ALL.len());
        let checked: Vec<_> = entries
            .iter()
            .filter_map(|entry| match entry {
                ContextEntry::Item { label, checked, .. } if *checked => Some(label.as_ref()),
                _ => None,
            })
            .collect();
        assert_eq!(checked, ["YAML"]);
        assert!(matches!(
            &entries[0],
            ContextEntry::Item {
                message: Some(Message::CopyAllStats(StatsFormat::PlainText)),
                ..
            }
        ));
    }

    fn labels(entries: &[ContextEntry]) -> Vec<(&str, bool)> {
        entries
            .iter()
            .map(|entry| match entry {
                ContextEntry::Item { label, message, .. } => (label.as_ref(), message.is_some()),
                ContextEntry::Divider => ("—", false),
            })
            .collect()
    }

    #[test]
    fn image_menu_opens_and_copies_the_path() {
        let entries = image_entries(PathBuf::from("/tmp/a.png"));
        assert_eq!(
            labels(&entries),
            vec![
                ("Open Image", true),
                ("Copy Image Path", true),
                ("—", false),
                ("Copy Markdown Source", true),
            ]
        );
    }

    #[test]
    fn preview_menu_disables_selection_items_without_a_selection() {
        assert_eq!(
            labels(&preview_entries(None, false, true)),
            vec![
                ("Copy", false),
                ("Select All", true),
                ("Find Selection", false),
                ("—", false),
                ("Copy Markdown Source", true),
            ]
        );
    }

    #[test]
    fn preview_menu_adds_link_items_for_a_link() {
        let entries = preview_entries(Some("https://example.com".into()), true, true);
        assert_eq!(
            labels(&entries)[..6],
            [
                ("Copy", true),
                ("Select All", true),
                ("Find Selection", true),
                ("—", false),
                ("Open Link", true),
                ("Copy Link Address", true),
            ]
        );
    }

    #[test]
    fn picking_an_entry_closes_the_menu_and_runs_it() {
        let mut state = State::blank();
        let _ = state.update(Message::OpenContextMenu(
            ContextTarget::Preview,
            iced::Point::ORIGIN,
        ));
        assert!(state.menus.context.is_some());
        let _ = state.update(Message::ContextMenuPick(0));
        assert!(state.menus.context.is_none());
    }

    #[test]
    fn pressing_the_menu_panel_keeps_it_open() {
        let mut state = State::blank();
        let _ = state.update(Message::OpenContextMenu(
            ContextTarget::Preview,
            iced::Point::ORIGIN,
        ));
        let _ = state.update(Message::ContextMenuInert);
        assert!(state.menus.context.is_some());
    }

    #[test]
    fn a_right_clicked_image_opens_the_image_menu_once() {
        let mut state = State::blank();
        let _ = state.update(Message::ContextImagePressed(PathBuf::from("/tmp/a.png")));
        let _ = state.update(Message::OpenContextMenu(
            ContextTarget::Preview,
            iced::Point::ORIGIN,
        ));
        assert!(state.menus.context_image.is_none());
        let menu = state.menus.context.as_ref().unwrap();
        assert_eq!(labels(&menu.entries)[0], ("Open Image", true));
    }

    #[test]
    fn a_right_clicked_link_is_used_once() {
        let mut state = State::blank();
        let _ = state.update(Message::ContextLinkPressed("https://example.com".into()));
        let _ = state.update(Message::OpenContextMenu(
            ContextTarget::Preview,
            iced::Point::ORIGIN,
        ));
        assert!(state.menus.context_link.is_none());
        let menu = state.menus.context.as_ref().unwrap();
        assert!(menu.entries.iter().any(|entry| matches!(
            entry,
            ContextEntry::Item { label, .. } if label == "Open Link"
        )));
    }

    #[test]
    fn every_nav_menu_ends_with_a_show_or_hide_text_files_row() {
        let mut state = State::blank();
        state.mode = crate::app::Mode::Folder {
            root: PathBuf::from("/a"),
            files: vec![],
            query: String::new(),
            selected: None,
        };
        let targets = || {
            [
                ContextTarget::NavBlank,
                ContextTarget::NavFolder(PathBuf::from("/a/docs")),
                ContextTarget::NavFile(PathBuf::from("/a/x.md")),
            ]
        };
        for (shown, target) in [false, true]
            .into_iter()
            .flat_map(|shown| targets().map(move |target| (shown, target)))
        {
            state.config.files.show_text = shown;
            let _ = state.update(Message::OpenContextMenu(target, iced::Point::ORIGIN));
            let menu = state.menus.context.take().expect("menu opens");
            assert!(matches!(
                menu.entries.last(),
                Some(ContextEntry::Item { label, icon: None, checked, message: Some(Message::ToggleShowTextFiles), .. })
                    if label.as_ref() == if shown { "Hide Text Files (.txt)" } else { "Show Text Files (.txt)" }
                        && *checked == shown
            ));
            assert!(matches!(
                menu.entries[menu.entries.len() - 2],
                ContextEntry::Divider
            ));
        }
        // The New dropdown does not carry it, and neither does the preview's menu.
        assert_eq!(new_entries(PathBuf::from("/a")).len(), 2);
        let _ = state.update(Message::OpenContextMenu(
            ContextTarget::Preview,
            iced::Point::ORIGIN,
        ));
        let menu = state.menus.context.take().expect("menu opens");
        assert!(!menu.entries.iter().any(|entry| matches!(
            entry,
            ContextEntry::Item {
                message: Some(Message::ToggleShowTextFiles),
                ..
            }
        )));
    }
}
