// SPDX-License-Identifier: GPL-3.0-or-later
//! Folder tree selection, search, and keyboard navigation.

use crate::app::{Message, Mode, NAV_DEBOUNCE, SEARCH_INPUT, State};
use crate::domain::folder::{FolderNode, NavEntry, matching_files, selection_target};
use crate::gutter::GutterSide;
use crate::ui::nav_scroll::{NAV_SCROLL, RevealCursor};
use iced::Task;

impl State {
    pub(crate) fn update_navigation(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::NavCommit(generation, path) => {
                if generation == self.nav.generation && self.nav.cursor.as_ref() == Some(&path) {
                    self.update(Message::FileSelected(path))
                } else {
                    Task::none()
                }
            }
            Message::NavCursorLocated(Some(offset)) => iced::widget::operation::scroll_to(
                NAV_SCROLL,
                iced::widget::operation::AbsoluteOffset {
                    x: None,
                    y: Some(offset),
                },
            ),
            Message::NavCursorLocated(None) => Task::none(),
            Message::ToggleFolder(path) => {
                self.nav.cursor = Some(path.clone());
                self.nav.invalidate_pending();
                if !self.nav.expanded.insert(path.clone()) {
                    self.nav.expanded.remove(&path);
                }
                Task::none()
            }
            Message::SearchChanged(new_query) => {
                if let Mode::Folder { query, .. } = &mut self.mode {
                    // Only the first character reveals Files; later keystrokes must not
                    // pull it back over a panel the user has since switched to.
                    let starting = query.trim().is_empty() && !new_query.trim().is_empty();
                    *query = new_query;
                    if starting {
                        self.gutters.reveal(GutterSide::Left);
                    }
                    self.nav.cursor = None;
                    self.nav.invalidate_pending();
                }
                Task::none()
            }
            Message::SearchCleared => {
                let cleared = self.update(Message::SearchChanged(String::new()));
                // Clicking the × would otherwise leave the field unfocused; Escape keeps it.
                Task::batch([cleared, iced::widget::operation::focus(SEARCH_INPUT)])
            }
            _ => unreachable!("navigation messages are routed by State::update"),
        }
    }

    pub(crate) fn move_nav_selection(&mut self, direction: isize) -> Task<Message> {
        let Mode::Folder {
            root,
            files,
            query,
            selected,
        } = &self.mode
        else {
            return Task::none();
        };
        let matches: Vec<NavEntry> = if query.trim().is_empty() {
            let mut visible = Vec::new();
            FolderNode::from_files(root, files).visible_entries(
                root,
                &self.nav.expanded,
                &mut visible,
            );
            visible
        } else {
            matching_files(root, files, query)
                .into_iter()
                .cloned()
                .map(NavEntry::File)
                .collect()
        };
        let selected_path = self
            .nav
            .cursor
            .as_deref()
            .or_else(|| selected.as_ref().map(|doc| doc.path.as_path()));
        let Some(next) = selection_target(&matches, selected_path, direction).cloned() else {
            return Task::none();
        };
        self.nav.cursor = Some(next.path().to_path_buf());
        self.nav.invalidate_pending();
        let reveal =
            iced::advanced::widget::operate(RevealCursor::new()).map(Message::NavCursorLocated);
        match next {
            NavEntry::Folder(_) => reveal,
            NavEntry::File(path) => {
                let generation = self.nav.generation;
                let commit = Task::perform(
                    async move {
                        tokio::time::sleep(NAV_DEBOUNCE).await;
                        (generation, path)
                    },
                    |(generation, path)| Message::NavCommit(generation, path),
                );
                Task::batch([reveal, commit])
            }
        }
    }

    pub(crate) fn navigate_folder(&mut self, right: bool) -> Task<Message> {
        let Mode::Folder {
            root, files, query, ..
        } = &self.mode
        else {
            return Task::none();
        };
        if !query.trim().is_empty() {
            return Task::none();
        }
        let Some(cursor) = self.nav.cursor.clone() else {
            return Task::none();
        };
        let tree = FolderNode::from_files(root, files);
        let mut entries = Vec::new();
        tree.visible_entries(root, &self.nav.expanded, &mut entries);
        let entry = entries.iter().find(|entry| entry.path() == cursor);
        match entry {
            Some(NavEntry::Folder(path)) if right => {
                self.nav.expanded.insert(path.clone());
            }
            Some(NavEntry::Folder(path)) if self.nav.expanded.remove(path) => {}
            Some(_) => {
                let parent = cursor.parent().filter(|path| {
                    *path != root
                        && entries.iter().any(
                            |entry| matches!(entry, NavEntry::Folder(folder) if folder == *path),
                        )
                });
                if !right && let Some(parent) = parent {
                    self.nav.cursor = Some(parent.to_path_buf());
                    self.nav.invalidate_pending();
                }
            }
            None => {}
        }
        Task::none()
    }

    pub(crate) fn toggle_nav_folder(&mut self) -> Task<Message> {
        let Mode::Folder {
            root, files, query, ..
        } = &self.mode
        else {
            return Task::none();
        };
        if !query.trim().is_empty() {
            return Task::none();
        }
        let Some(cursor) = self.nav.cursor.clone() else {
            return Task::none();
        };
        let mut entries = Vec::new();
        FolderNode::from_files(root, files).visible_entries(root, &self.nav.expanded, &mut entries);
        if entries
            .iter()
            .any(|entry| matches!(entry, NavEntry::Folder(path) if path == &cursor))
        {
            self.update(Message::ToggleFolder(cursor))
        } else {
            Task::none()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::Viewport;
    use std::path::PathBuf;

    fn folder_state() -> State {
        let mut state = State::blank();
        state.viewport = Viewport::new(900.0);
        state.mode = Mode::Folder {
            root: PathBuf::from("/notes"),
            files: Vec::new(),
            query: String::new(),
            selected: None,
        };
        state.gutters.sidebar_visible = true;
        state.gutters.inspector_visible = true;
        state.gutters.active = GutterSide::Right;
        state
    }

    #[test]
    fn the_first_character_reveals_files() {
        let mut state = folder_state();
        let _ = state.update_navigation(Message::SearchChanged("a".into()));
        assert_eq!(state.gutters.active, GutterSide::Left);
    }

    #[test]
    fn later_keystrokes_do_not_pull_files_back_over_another_panel() {
        let mut state = folder_state();
        let _ = state.update_navigation(Message::SearchChanged("a".into()));
        state.gutters.reveal(GutterSide::Right);
        let _ = state.update_navigation(Message::SearchChanged("ab".into()));
        assert_eq!(state.gutters.active, GutterSide::Right);
    }

    #[test]
    fn clearing_the_search_does_not_reveal_files() {
        let mut state = folder_state();
        let _ = state.update_navigation(Message::SearchChanged("a".into()));
        state.gutters.reveal(GutterSide::Right);
        let _ = state.update_navigation(Message::SearchChanged(String::new()));
        assert_eq!(state.gutters.active, GutterSide::Right);
    }

    #[test]
    fn the_clear_button_empties_the_query_without_changing_the_active_panel() {
        let mut state = folder_state();
        let _ = state.update_navigation(Message::SearchChanged("abc".into()));
        state.gutters.reveal(GutterSide::Right);
        let _ = state.update_navigation(Message::SearchCleared);
        let Mode::Folder { query, .. } = &state.mode else {
            panic!("folder mode");
        };
        assert!(query.is_empty());
        assert_eq!(state.gutters.active, GutterSide::Right);
    }
}
