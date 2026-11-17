// SPDX-License-Identifier: GPL-3.0-or-later
//! Keyboard shortcuts: translates an `iced` key press into a `keymap::Chord`,
//! resolves it against the active scopes, and dispatches the resulting
//! `Command`. The keymap table itself lives in `domain::keymap`; this module
//! is the only place an `iced` key is touched.

use crate::actions::MenuAction;
use crate::app::{FIND_INPUT, Message, Mode, SEARCH_INPUT, State};
use crate::domain::keymap::{self, Chord, Command, KeyName, Scope};
use crate::gutter::{GutterSide, LeftTab, RightTab};
use crate::platform::dialogs::{pick_file, pick_folder};
use crate::ui::locate_heading::PageScroll;
use iced::Task;

/// Translates an `iced` key press into a chord the keymap understands, or
/// `None` for keys the keymap has no opinion about.
fn chord_from(key: &iced::keyboard::Key, modifiers: iced::keyboard::Modifiers) -> Option<Chord> {
    use iced::keyboard::Key;
    use iced::keyboard::key::Named;
    let key = match key.as_ref() {
        Key::Character(text) => KeyName::Char(text.chars().next()?.to_ascii_lowercase()),
        Key::Named(Named::ArrowUp) => KeyName::ArrowUp,
        Key::Named(Named::ArrowDown) => KeyName::ArrowDown,
        Key::Named(Named::ArrowLeft) => KeyName::ArrowLeft,
        Key::Named(Named::ArrowRight) => KeyName::ArrowRight,
        Key::Named(Named::Tab) => KeyName::Tab,
        Key::Named(Named::Escape) => KeyName::Escape,
        Key::Named(Named::Enter) => KeyName::Enter,
        Key::Named(Named::PageUp) => KeyName::PageUp,
        Key::Named(Named::PageDown) => KeyName::PageDown,
        Key::Named(Named::F2) => KeyName::F2,
        Key::Named(Named::Delete) => KeyName::Delete,
        Key::Named(Named::Backspace) => KeyName::Backspace,
        _ => return None,
    };
    Some(Chord {
        key,
        primary: modifiers.control() || modifiers.command(),
        shift: modifiers.shift(),
        alt: modifiers.alt(),
    })
}

impl State {
    pub(crate) fn update_keyboard(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::KeyPressed(key, modifiers, captured) => {
                // Ctrl+Insert for copy doesn't fit the primary/shift/alt chord shape
                // (it's always the literal Ctrl key, not ⌘ on macOS), so it stays a
                // small special case rather than growing the schema for one binding.
                if !captured
                    && modifiers.control()
                    && matches!(
                        key.as_ref(),
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::Insert)
                    )
                    && self.selected_text().is_some()
                {
                    return self.update(Message::CopySelection);
                }

                // A draft being named owns the keyboard: Escape discards it, and only Global
                // bindings (quit, open…) still act, so arrows and Tab stay in the field.
                let drafting = self.nav.draft.is_some();
                if drafting
                    && matches!(
                        key.as_ref(),
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape)
                    )
                {
                    return self.update(Message::NewItemCancel);
                }
                let Some(chord) = chord_from(&key, modifiers) else {
                    return Task::none();
                };
                // A focused text input already claimed this exact key (arrows,
                // Ctrl+A/C, etc.); only Global bindings may still act on it.
                let scopes = if captured || drafting {
                    vec![Scope::Global]
                } else {
                    self.input_scopes()
                };
                let Some(command) = keymap::resolve(&scopes, chord) else {
                    return Task::none();
                };
                self.run_command(command)
            }
            _ => unreachable!("keyboard messages are routed by State::update"),
        }
    }

    /// The file or folder the nav highlights, for F2 and Delete; nothing while a menu or dialog
    /// is up, which owns the keyboard.
    fn keyboard_nav_target(&self) -> Option<std::path::PathBuf> {
        if self.menus.dialog.is_some() || self.menus.context.is_some() {
            return None;
        }
        self.root()?;
        self.nav
            .cursor
            .clone()
            .or_else(|| self.current_document().map(|doc| doc.path.clone()))
    }

    fn run_command(&mut self, command: Command) -> Task<Message> {
        match command {
            Command::OpenFile => Task::perform(pick_file(), Message::FileOpened),
            Command::OpenFolder => {
                Task::perform(pick_folder(self.config.clone()), Message::FolderOpened)
            }
            Command::CloseCurrent => self.update(Message::MenuAction(MenuAction::CloseCurrent)),
            Command::Quit => iced::exit(),
            Command::ToggleFiles => self.update(Message::TogglePanel(GutterSide::Left)),
            Command::ToggleRightGutter => self.update(Message::TogglePanel(GutterSide::Right)),
            Command::ShowOutline => self.update(Message::SelectRightTab(RightTab::Outline)),
            Command::ToggleMinimap => self.update(Message::MenuAction(MenuAction::ToggleMinimap)),
            Command::ToggleSourceView => {
                self.update(Message::MenuAction(MenuAction::ToggleSourceView))
            }
            Command::ShowDetails => self.update(Message::SelectRightTab(RightTab::Details)),
            Command::DialogFocusNext => iced::widget::operation::focus_next(),
            Command::DialogFocusPrevious => iced::widget::operation::focus_previous(),
            Command::DialogDefaultAction => self.dialog_default_action(),
            Command::CycleRightTab => {
                if !matches!(self.mode, Mode::Folder { .. }) {
                    return Task::none();
                }
                let next = match self.gutters.right_tab {
                    RightTab::Outline => RightTab::Details,
                    RightTab::Details => RightTab::Outline,
                };
                self.update(Message::SelectRightTab(next))
            }
            Command::CycleLeftTab => {
                if !matches!(self.mode, Mode::Folder { .. }) {
                    return Task::none();
                }
                let next = match self.gutters.left_tab {
                    LeftTab::Files => LeftTab::Recent,
                    LeftTab::Recent => LeftTab::Files,
                };
                self.update(Message::SelectLeftTab(next))
            }
            Command::ShowShortcuts => self.update(Message::MenuAction(MenuAction::Shortcuts)),
            Command::ReloadTheme => self.update(Message::MenuAction(MenuAction::ReloadTheme)),
            Command::FindInDocument => {
                if self.current_document().is_none() {
                    return Task::none();
                }
                self.find.visible = true;
                iced::widget::operation::focus(FIND_INPUT)
            }
            Command::FindNext => self.update(Message::FindNext),
            Command::FindPrevious => self.update(Message::FindPrevious),
            Command::CloseFind => self.update(Message::CloseFind),
            Command::ClearSearch => match &self.mode {
                Mode::Folder { query, .. } if !query.is_empty() => {
                    self.update(Message::SearchCleared)
                }
                _ => Task::none(),
            },
            Command::FocusFolderSearch => {
                if !matches!(self.mode, Mode::Folder { .. }) {
                    return Task::none();
                }
                self.gutters.reveal(GutterSide::Left);
                iced::widget::operation::focus(SEARCH_INPUT)
            }
            Command::NavUp => self.move_nav_selection(-1),
            Command::NavDown => self.move_nav_selection(1),
            Command::NavExpand => self.navigate_folder(true),
            Command::NavCollapse => self.navigate_folder(false),
            Command::NavToggleFolder => self.toggle_nav_folder(),
            Command::Copy => {
                if self.selected_text().is_some() {
                    self.update(Message::CopySelection)
                } else {
                    Task::none()
                }
            }
            Command::SelectAll => {
                if self.leaves.is_empty() {
                    Task::none()
                } else {
                    self.update(Message::SelectAll)
                }
            }
            Command::FindSelection => {
                if self.selected_text().is_some() {
                    self.update(Message::FindSelection)
                } else {
                    Task::none()
                }
            }
            Command::RenameItem => match self.keyboard_nav_target() {
                Some(path) => self.update(Message::RenameItem(path)),
                None => Task::none(),
            },
            Command::DeleteItem => match self.keyboard_nav_target() {
                Some(path) => self.update(Message::DeleteItem(path)),
                None => Task::none(),
            },
            Command::HistoryBack => self.update(Message::HistoryBack),
            Command::HistoryForward => self.update(Message::HistoryForward),
            Command::ScrollPageUp => iced::advanced::widget::operate(PageScroll::up()).discard(),
            Command::ScrollPageDown => {
                iced::advanced::widget::operate(PageScroll::down()).discard()
            }
            Command::ZoomIn => self.update(Message::MenuAction(MenuAction::ZoomIn)),
            Command::ZoomOut => self.update(Message::MenuAction(MenuAction::ZoomOut)),
            Command::ZoomReset => self.update(Message::MenuAction(MenuAction::ResetZoom)),
            Command::Dismiss => self.dismiss_topmost(),
        }
    }

    /// Closes whichever of the context menu, the in-window/native menu, the
    /// dialog, or a leftover selection is topmost, in that order — the same
    /// cascade Escape has always followed.
    fn dismiss_topmost(&mut self) -> Task<Message> {
        if self.menus.context.is_some() {
            return self.update(Message::DismissContextMenu);
        }
        #[cfg(not(target_os = "macos"))]
        if self.menus.active.is_some() || self.menus.recent_open {
            self.menus.active = None;
            self.menus.recent_open = false;
            return Task::none();
        }
        if self.menus.dialog.is_some() {
            return self.close_dialog();
        }
        if self.selection.is_some() {
            return self.update(Message::SelectionCleared);
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use crate::app::{ContextTarget, Message, State};
    use crate::domain::selection::BlockId;
    use crate::gutter::{LeftTab, RightTab};
    use iced::keyboard::{Key, Modifiers, key::Named};

    fn with_text() -> State {
        let mut state = State::blank();
        state.leaves = vec![
            (BlockId { item: 0, leaf: 0 }, "Title".into()),
            (BlockId { item: 1, leaf: 0 }, "Body text".into()),
        ];
        state
    }

    fn press(state: &mut State, key: Key, modifiers: Modifiers, captured: bool) {
        let _ = state.update(Message::KeyPressed(key, modifiers, captured));
    }

    fn browsing() -> State {
        let mut state = State::blank();
        state.mode = crate::app::Mode::Folder {
            root: "/notes".into(),
            files: Vec::new(),
            query: String::new(),
            selected: None,
        };
        state.nav.history.visit("/notes/a.md".into());
        state.nav.history.visit("/notes/b.md".into());
        state
    }

    #[test]
    fn alt_arrows_step_through_history() {
        let mut state = browsing();
        press(
            &mut state,
            Key::Named(Named::ArrowRight),
            Modifiers::ALT,
            false,
        );
        assert_eq!(state.nav.history_target, None, "nothing ahead yet");
        press(
            &mut state,
            Key::Named(Named::ArrowLeft),
            Modifiers::ALT,
            false,
        );
        assert_eq!(state.nav.history_target, Some(0));
        assert_eq!(state.nav.cursor.as_deref(), Some("/notes/a.md".as_ref()));
    }

    #[test]
    fn alt_arrows_leave_a_focused_text_box_alone() {
        let mut state = browsing();
        press(
            &mut state,
            Key::Named(Named::ArrowLeft),
            Modifiers::ALT,
            true,
        );
        assert_eq!(state.nav.history_target, None);
    }

    #[test]
    fn ctrl_a_selects_the_whole_preview() {
        let mut state = with_text();
        press(
            &mut state,
            Key::Character("a".into()),
            Modifiers::CTRL,
            false,
        );
        assert_eq!(state.selected_text().as_deref(), Some("Title\nBody text"));
    }

    #[test]
    fn shortcuts_leave_a_focused_text_box_alone() {
        let mut state = with_text();
        press(
            &mut state,
            Key::Character("a".into()),
            Modifiers::CTRL,
            true,
        );
        assert!(state.selection.is_none());
    }

    #[test]
    fn escape_closes_the_context_menu_before_clearing_the_selection() {
        let mut state = with_text();
        let _ = state.update(Message::SelectAll);
        let _ = state.update(Message::OpenContextMenu(
            ContextTarget::Preview,
            iced::Point::ORIGIN,
        ));
        press(
            &mut state,
            Key::Named(Named::Escape),
            Modifiers::empty(),
            false,
        );
        assert!(state.menus.context.is_none());
        assert!(state.selection.is_some());
        press(
            &mut state,
            Key::Named(Named::Escape),
            Modifiers::empty(),
            false,
        );
        assert!(state.selection.is_none());
    }

    #[test]
    fn shift_click_extends_from_the_anchor_in_either_direction() {
        let mut state = with_text();
        let body = BlockId { item: 1, leaf: 0 };
        let _ = state.update(Message::SelectionStarted(body, 4));
        let _ = state.update(Message::SelectionEnded);
        let _ = state.update(Message::SelectionExtendedTo(body, 9));
        assert_eq!(state.selected_text().as_deref(), Some(" text"));
        let _ = state.update(Message::SelectionExtendedTo(
            BlockId { item: 0, leaf: 0 },
            2,
        ));
        assert_eq!(state.selected_text().as_deref(), Some("tle\nBody"));
    }

    #[test]
    fn ctrl_alt_b_toggles_the_right_gutter() {
        let mut state = browsing();
        assert!(!state.gutters.inspector_visible);
        press(
            &mut state,
            Key::Character("b".into()),
            Modifiers::CTRL | Modifiers::ALT,
            false,
        );
        assert!(state.gutters.inspector_visible);
    }

    #[test]
    fn ctrl_alt_1_shows_the_outline() {
        let mut state = browsing();
        state.gutters.right_tab = RightTab::Details;
        press(
            &mut state,
            Key::Character("1".into()),
            Modifiers::CTRL | Modifiers::ALT,
            false,
        );
        assert_eq!(state.gutters.right_tab, RightTab::Outline);
        assert!(state.gutters.inspector_visible);
    }

    #[test]
    fn ctrl_shift_page_down_cycles_the_left_tab() {
        let mut state = browsing();
        assert_eq!(state.gutters.left_tab, LeftTab::Files);
        press(
            &mut state,
            Key::Named(Named::PageDown),
            Modifiers::CTRL | Modifiers::SHIFT,
            false,
        );
        assert_eq!(state.gutters.left_tab, LeftTab::Recent);
        assert!(state.gutters.sidebar_visible);
        press(
            &mut state,
            Key::Named(Named::PageDown),
            Modifiers::CTRL | Modifiers::SHIFT,
            false,
        );
        assert_eq!(state.gutters.left_tab, LeftTab::Files);
    }

    #[test]
    fn ctrl_slash_opens_the_shortcuts_dialog() {
        let mut state = State::blank();
        press(
            &mut state,
            Key::Character("/".into()),
            Modifiers::CTRL,
            false,
        );
        assert!(matches!(
            state.menus.dialog,
            Some(crate::app::Dialog::Shortcuts)
        ));
    }

    #[test]
    fn closing_a_dialog_returns_focus_to_the_search_it_interrupted() {
        let mut state = State::blank();
        state.search_focused = true;
        state.open_dialog(crate::app::Dialog::About);
        assert!(state.dialog_restores_search);
        let _ = state.close_dialog();
        assert!(state.menus.dialog.is_none() && !state.dialog_restores_search);
        // Without prior focus there is nothing to restore.
        state.search_focused = false;
        state.open_dialog(crate::app::Dialog::About);
        assert!(!state.dialog_restores_search);
    }

    #[test]
    fn ctrl_page_up_cycles_the_right_tab_in_folder_mode() {
        let mut state = browsing();
        let before = state.gutters.right_tab;
        press(
            &mut state,
            Key::Named(Named::PageUp),
            Modifiers::CTRL,
            false,
        );
        assert_ne!(state.gutters.right_tab, before);
    }

    #[test]
    fn ctrl_page_up_does_nothing_outside_folder_mode() {
        let mut state = State::blank();
        let before = state.gutters.right_tab;
        press(
            &mut state,
            Key::Named(Named::PageUp),
            Modifiers::CTRL,
            false,
        );
        assert_eq!(state.gutters.right_tab, before);
    }

    #[test]
    fn escape_in_a_focused_search_clears_it_and_otherwise_does_nothing_itself() {
        let mut state = browsing();
        state.search_focused = true;
        assert_eq!(
            state.input_scopes().first(),
            Some(&crate::domain::keymap::Scope::Search)
        );
        let _ = state.update(Message::SearchChanged("abc".into()));
        press(
            &mut state,
            Key::Named(Named::Escape),
            Modifiers::default(),
            false,
        );
        let crate::app::Mode::Folder { query, .. } = &state.mode else {
            panic!("folder mode");
        };
        assert!(query.is_empty(), "the first Escape clears the text");
        // With nothing to clear, the keymap leaves Escape to the field, which blurs itself.
        press(
            &mut state,
            Key::Named(Named::Escape),
            Modifiers::default(),
            false,
        );
        assert!(
            state.search_focused,
            "the app does not blur it behind the field's back"
        );
    }
}
