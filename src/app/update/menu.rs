// SPDX-License-Identifier: GPL-3.0-or-later
//! Menus and dialogs.

use crate::actions::MenuAction;
use crate::app::{Dialog, ErrorNotice, FindState, Message, Mode, State, detect_appearance};
use crate::config::{Config, ConfigError, ExternalLinkAction};
use crate::gutter::{GutterSide, RightTab};
use crate::ui::theme::Appearance;
use iced::Task;

impl State {
    /// Shows `dialog`, remembering whether the folder search had focus so closing can return it.
    pub(crate) fn open_dialog(&mut self, dialog: Dialog) {
        self.dialog_restores_search = self.search_focused;
        self.menus.dialog = Some(dialog);
    }

    /// Activates whichever button Enter should hit in the open dialog, since
    /// `iced` buttons can't take keyboard focus for Tab/Space to reach. Only
    /// a dialog with one unambiguous, non-destructive-by-default action gets
    /// a binding; `ExternalLink` offers several open choices and no single
    /// default, so it falls through to Cancel via Escape instead.
    pub(crate) fn dialog_default_action(&mut self) -> Task<Message> {
        match self.menus.dialog {
            Some(Dialog::About | Dialog::Shortcuts) => self.update(Message::DismissDialog),
            Some(Dialog::ConfirmDelete) => self.update(Message::ConfirmDelete),
            Some(Dialog::NewItem | Dialog::ExternalLink) | None => Task::none(),
        }
    }

    /// Hides the dialog and hands focus back to whatever held it before it opened.
    pub(crate) fn close_dialog(&mut self) -> Task<Message> {
        self.menus.dialog = None;
        // Dismissing by Escape leaves nothing pending for the next dialog to act on.
        self.nav.pending_external = None;
        if std::mem::take(&mut self.dialog_restores_search) {
            iced::widget::operation::focus(crate::app::SEARCH_INPUT)
        } else {
            Task::none()
        }
    }

    /// Carries out a command chosen from the native or in-window menu.
    fn menu_action(&mut self, action: MenuAction) -> Task<Message> {
        #[cfg(not(target_os = "macos"))]
        {
            self.menus.active = None;
            self.menus.recent_open = false;
        }
        match action {
            MenuAction::About => {
                self.open_dialog(Dialog::About);
                Task::none()
            }
            MenuAction::Shortcuts => {
                self.open_dialog(Dialog::Shortcuts);
                Task::none()
            }
            MenuAction::Quit => iced::exit(),
            MenuAction::OpenFile => self.update(Message::OpenFileClicked),
            MenuAction::OpenFolder => self.update(Message::OpenFolderClicked),
            MenuAction::Recent(item) => self.update(Message::OpenRecent(item)),
            MenuAction::CloseCurrent => {
                self.mode = Mode::Welcome;
                self.nav.history.clear();
                self.nav.history_target = None;
                self.find = FindState::default();
                self.nav.cursor = None;
                self.nav.invalidate_pending();
                self.clear_minimap();
                Task::none()
            }
            MenuAction::ShowFiles => {
                self.gutters.reveal(GutterSide::Left);
                Task::none()
            }
            MenuAction::ShowOutline => self.update(Message::SelectRightTab(RightTab::Outline)),
            MenuAction::ShowDetails => self.update(Message::SelectRightTab(RightTab::Details)),
            MenuAction::ToggleMinimap => {
                let shown = !self.config.preview.minimap;
                match Config::save_minimap(shown) {
                    Ok(()) => {
                        self.config.preview.minimap = shown;
                        self.error = None;
                    }
                    Err(error) => self.error = Some(ErrorNotice::new(error, None)),
                }
                Task::none()
            }
            MenuAction::ToggleSourceView => {
                self.toggle_document_view();
                Task::none()
            }
            MenuAction::Appearance(choice) => {
                let result = Config::load().and_then(|disk| {
                    let mut next = self.config.clone();
                    next.theme = disk.theme;
                    next.theme.appearance = choice;
                    let appearance = Appearance::from_config(&next, detect_appearance())?;
                    Config::save_appearance(choice)?;
                    Ok((next, appearance))
                });
                match result {
                    Ok((next, appearance)) => {
                        self.config = next;
                        self.appearance = appearance;
                        self.error = None;
                    }
                    Err(error) => self.error = Some(ErrorNotice::new(error, None)),
                }
                Task::none()
            }
            MenuAction::PreviewFont(choice) => {
                match Config::save_font(choice) {
                    Ok(()) => {
                        self.config.preview.font = choice;
                        self.error = None;
                    }
                    Err(error) => self.error = Some(ErrorNotice::new(error, None)),
                }
                Task::none()
            }
            MenuAction::AskAboutExternalLinks => {
                self.remember_external_choice(ExternalLinkAction::Ask);
                Task::none()
            }
            MenuAction::ZoomIn => {
                self.zoom_in();
                Task::none()
            }
            MenuAction::ZoomOut => {
                self.zoom_out();
                Task::none()
            }
            MenuAction::ResetZoom => {
                self.reset_zoom();
                Task::none()
            }
            MenuAction::ReloadTheme => {
                match Config::load().and_then(|config| {
                    Appearance::from_config(&config, detect_appearance())
                        .map(|appearance| (config, appearance))
                        .map_err(ConfigError::from)
                }) {
                    Ok((config, appearance)) => {
                        self.config.theme = config.theme;
                        self.config.preview = config.preview;
                        self.config.links = config.links;
                        self.appearance = appearance;
                        self.error = None;
                        self.notify("Theme reloaded.", None);
                    }
                    Err(error) => {
                        self.error = Some(ErrorNotice::new(
                            error,
                            Some(Message::MenuAction(MenuAction::ReloadTheme)),
                        ));
                    }
                }
                Task::none()
            }
        }
    }

    pub(crate) fn update_menu(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::MenuAction(action) => self.menu_action(action),
            #[cfg(not(target_os = "macos"))]
            Message::ToggleMenu(menu) => {
                self.menus.active = (self.menus.active != Some(menu)).then_some(menu);
                self.menus.recent_open = false;
                Task::none()
            }
            #[cfg(not(target_os = "macos"))]
            Message::ActivateMenu(menu) => {
                if self.menus.active != Some(menu) {
                    self.menus.active = Some(menu);
                    self.menus.recent_open = false;
                }
                Task::none()
            }
            #[cfg(not(target_os = "macos"))]
            Message::OpenRecentMenu => {
                self.menus.recent_open = true;
                Task::none()
            }
            #[cfg(not(target_os = "macos"))]
            Message::CloseRecentMenu => {
                self.menus.recent_open = false;
                Task::none()
            }
            #[cfg(not(target_os = "macos"))]
            Message::DismissMenu => {
                self.menus.active = None;
                self.menus.recent_open = false;
                Task::none()
            }
            Message::DismissDialog => self.close_dialog(),
            #[cfg(target_os = "macos")]
            Message::NativeMenuTick => {
                crate::platform::native_menu::install(&self.recents);
                crate::platform::macos_open::install();
                // Files Finder asked to open (double-click, Open With, Dock drop). Each is opened
                // in turn, so the last stays on screen and all land in Recent; one that no
                // longer exists is reported.
                let mut tasks = Vec::new();
                for path in crate::platform::macos_open::take() {
                    match State::recent_for_path(path.clone()) {
                        Some(item) => tasks.push(self.update(Message::OpenRecent(item))),
                        None => {
                            self.error = Some(ErrorNotice::new(
                                format!("Couldn't open {}: it no longer exists.", path.display()),
                                None,
                            ));
                        }
                    }
                }
                if !tasks.is_empty() {
                    return Task::batch(tasks);
                }
                if let Some(action) = crate::platform::native_menu::next_action(&self.recents) {
                    return self.update(Message::MenuAction(action));
                }
                Task::none()
            }
            _ => unreachable!("menu messages are routed by State::update"),
        }
    }
}
