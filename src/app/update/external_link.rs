// SPDX-License-Identifier: GPL-3.0-or-later
//! A link to a Markdown file outside the open folder: ask what to do with it, or apply the
//! remembered choice.

use crate::app::{Dialog, ErrorNotice, Message, Mode, State};
use crate::config::{Config, ExternalLinkAction};
use crate::domain::links::external_markdown_target;
use crate::domain::workspace::OpenRequest;
use iced::Task;
use std::path::PathBuf;

impl State {
    /// The file a clicked link names, when it is a Markdown file outside the open folder.
    pub(crate) fn external_markdown_link(&self, uri: &str) -> Option<PathBuf> {
        let Mode::Folder { root, selected, .. } = &self.mode else {
            return None;
        };
        let doc_dir = selected.as_ref()?.path.parent()?;
        external_markdown_target(uri, doc_dir, root)
    }

    /// Opens `path` as the remembered choice says, or asks.
    pub(crate) fn open_external_markdown(&mut self, path: PathBuf) -> Task<Message> {
        match self.config.links.external_markdown {
            ExternalLinkAction::Ask => {
                self.nav.pending_external = Some(path);
                self.nav.external_always = false;
                self.open_dialog(Dialog::ExternalLink);
                Task::none()
            }
            choice @ (ExternalLinkAction::Folder | ExternalLinkAction::Document) => {
                self.apply_external(path, choice)
            }
        }
    }

    /// Carries out a choice. `Ask` does nothing.
    fn apply_external(&mut self, path: PathBuf, choice: ExternalLinkAction) -> Task<Message> {
        match choice {
            // The file's folder becomes the workspace, which also resets history as any
            // workspace change does.
            ExternalLinkAction::Folder => self.request_open(OpenRequest::file(&path)),
            // Load the file alone; the open folder and its file list stay as they are.
            ExternalLinkAction::Document => self.update(Message::FileSelected(path)),
            ExternalLinkAction::Ask => Task::none(),
        }
    }

    pub(crate) fn update_external_link(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ExternalLinkAlways(on) => {
                self.nav.external_always = on;
                Task::none()
            }
            Message::ExternalLinkOpenFolder => self.choose_external(ExternalLinkAction::Folder),
            Message::ExternalLinkOpenDocument => self.choose_external(ExternalLinkAction::Document),
            Message::ExternalLinkOpenWith(editor) => {
                let path = self.nav.pending_external.take();
                let restore = self.close_dialog();
                if let Some(path) = path
                    && let Err(error) = self.opener.open_with(&path, editor.command)
                {
                    self.error = Some(ErrorNotice::new(
                        format!(
                            "Couldn't open {} with {}: {error}",
                            path.display(),
                            editor.name
                        ),
                        None,
                    ));
                }
                restore
            }
            Message::CancelExternalLink => {
                self.nav.pending_external = None;
                self.close_dialog()
            }
            _ => unreachable!("external link messages are routed by State::update"),
        }
    }

    /// The dialog's Folder or Document button: remember the choice if asked, then apply it.
    fn choose_external(&mut self, choice: ExternalLinkAction) -> Task<Message> {
        let path = self.nav.pending_external.take();
        let remember = std::mem::take(&mut self.nav.external_always);
        let restore = self.close_dialog();
        let Some(path) = path else {
            return restore;
        };
        let work = self.apply_external(path, choice);
        if remember {
            self.remember_external_choice(choice);
        }
        Task::batch([restore, work])
    }

    /// Saves `choice` as the default for future external links. A failure is reported but does
    /// not undo the open that just happened. Called after the open, which clears `error`.
    pub(crate) fn remember_external_choice(&mut self, choice: ExternalLinkAction) {
        match Config::save_external_link_action(choice) {
            Ok(()) => self.config.links.external_markdown = choice,
            Err(error) => self.error = Some(ErrorNotice::new(error, None)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::document::Document;

    /// A state showing `root/a.md`, with `outside/o.md` beside the root.
    fn state_with_outside_file(
        name: &str,
    ) -> (State, crate::testutil::ScratchDir, PathBuf, PathBuf) {
        let base = crate::testutil::scratch_dir(&format!("extlink-{name}"));
        let root = base.join("root");
        let outside = base.join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(root.join("a.md"), "# a").unwrap();
        std::fs::write(outside.join("o.md"), "# o").unwrap();
        let mut state = State::blank();
        state.mode = Mode::Folder {
            root: std::fs::canonicalize(&root).unwrap(),
            files: vec![root.join("a.md")],
            query: String::new(),
            selected: Document::open(root.join("a.md")).ok(),
        };
        (
            state,
            base,
            root,
            std::fs::canonicalize(outside.join("o.md")).unwrap(),
        )
    }

    fn root_of(state: &State) -> PathBuf {
        match &state.mode {
            Mode::Folder { root, .. } => root.clone(),
            Mode::Welcome => panic!("a folder should be open"),
        }
    }

    #[test]
    fn a_link_outside_the_folder_asks_by_default_and_changes_nothing_yet() {
        let (mut state, _base, root, outside) = state_with_outside_file("ask");
        let before = root_of(&state);
        let uri = format!("file://{}", outside.display());
        let _ = state.update(Message::LinkClicked(uri));
        assert!(matches!(state.menus.dialog, Some(Dialog::ExternalLink)));
        assert_eq!(
            state.nav.pending_external.as_deref(),
            Some(outside.as_path())
        );
        assert_eq!(root_of(&state), before);
        assert!(state.error.is_none(), "no 'Blocked link' notice");
        let _ = root;
    }

    #[test]
    fn the_dialog_builds_with_and_without_a_pending_file() {
        let (mut state, _base, _, outside) = state_with_outside_file("view");
        let _ = state.external_link_dialog();
        state.nav.pending_external = Some(outside);
        state.nav.external_always = true;
        let _ = state.external_link_dialog();
    }

    #[test]
    fn a_link_inside_the_folder_or_to_the_web_never_asks() {
        let (mut state, _base, root, _) = state_with_outside_file("inside");
        std::fs::write(root.join("b.md"), "# b").unwrap();
        let _ = state.update(Message::LinkClicked("b.md".into()));
        assert!(state.menus.dialog.is_none());
        let _ = state.update(Message::LinkClicked("https://example.com/x.md".into()));
        assert!(state.menus.dialog.is_none());
    }

    #[test]
    fn open_containing_folder_makes_that_folder_the_workspace() {
        let (mut state, _base, _, outside) = state_with_outside_file("folder");
        let before = root_of(&state);
        let _ = state.update(Message::LinkClicked(outside.display().to_string()));
        let _ = state.update(Message::ExternalLinkOpenFolder);
        assert!(state.menus.dialog.is_none() && state.nav.pending_external.is_none());
        assert_eq!(root_of(&state), before, "the open runs in the background");
        assert!(state.nav.opening.is_some());
        state.complete_pending_open();
        assert!(state.nav.opening.is_none());
        assert_eq!(root_of(&state), outside.parent().unwrap());
        assert_eq!(
            state.config.links.external_markdown,
            ExternalLinkAction::Ask,
            "not remembered without the checkbox"
        );
    }

    #[test]
    fn open_as_external_document_keeps_the_open_folder() {
        let (mut state, _base, _, outside) = state_with_outside_file("document");
        let before = root_of(&state);
        let _ = state.update(Message::LinkClicked(outside.display().to_string()));
        let _ = state.update(Message::ExternalLinkOpenDocument);
        assert!(state.menus.dialog.is_none() && state.nav.pending_external.is_none());
        assert_eq!(root_of(&state), before);
        assert_eq!(state.nav.cursor.as_deref(), Some(outside.as_path()));
    }

    #[test]
    fn cancel_and_escape_leave_nothing_pending() {
        let (mut state, _base, _, outside) = state_with_outside_file("cancel");
        let before = root_of(&state);
        let _ = state.update(Message::LinkClicked(outside.display().to_string()));
        let _ = state.update(Message::CancelExternalLink);
        assert!(state.menus.dialog.is_none() && state.nav.pending_external.is_none());
        let _ = state.update(Message::LinkClicked(outside.display().to_string()));
        let _ = state.update(Message::DismissDialog);
        assert!(state.menus.dialog.is_none() && state.nav.pending_external.is_none());
        assert_eq!(root_of(&state), before);
    }

    #[test]
    fn always_remembers_the_choice_and_later_links_skip_the_dialog() {
        let (mut state, _base, _, outside) = state_with_outside_file("always");
        let _ = state.update(Message::LinkClicked(outside.display().to_string()));
        let _ = state.update(Message::ExternalLinkAlways(true));
        let _ = state.update(Message::ExternalLinkOpenDocument);
        assert_eq!(
            state.config.links.external_markdown,
            ExternalLinkAction::Document
        );
        let saved = Config::load().unwrap();
        assert_eq!(saved.links.external_markdown, ExternalLinkAction::Document);
        // The next external link opens without asking.
        let _ = state.update(Message::LinkClicked(outside.display().to_string()));
        assert!(state.menus.dialog.is_none());
        // Resetting from the menu brings the dialog back.
        let _ = state.update(Message::MenuAction(
            crate::actions::MenuAction::AskAboutExternalLinks,
        ));
        assert_eq!(
            state.config.links.external_markdown,
            ExternalLinkAction::Ask
        );
        let _ = state.update(Message::LinkClicked(outside.display().to_string()));
        assert!(matches!(state.menus.dialog, Some(Dialog::ExternalLink)));
    }

    #[test]
    fn a_remembered_folder_choice_opens_the_folder_without_asking() {
        let (mut state, _base, _, outside) = state_with_outside_file("remembered-folder");
        state.config.links.external_markdown = ExternalLinkAction::Folder;
        let _ = state.update(Message::LinkClicked(outside.display().to_string()));
        assert!(state.menus.dialog.is_none());
        state.complete_pending_open();
        assert_eq!(root_of(&state), outside.parent().unwrap());
    }

    use crate::testutil::{OpenerCall, RecordingOpener};

    /// A state with a recording opener, plus a handle to read what it was asked to do.
    fn with_opener(
        name: &str,
        opener: RecordingOpener,
    ) -> (State, crate::testutil::ScratchDir, PathBuf, RecordingOpener) {
        let (mut state, base, root, _) = state_with_outside_file(name);
        state.opener = Box::new(opener.clone());
        (state, base, root, opener)
    }

    #[test]
    fn only_safe_links_reach_the_operating_system() {
        let (mut state, _base, root, opener) = with_opener("policy", RecordingOpener::default());
        std::fs::write(root.join("paper.pdf"), "x").unwrap();
        std::fs::write(root.join("run.command"), "x").unwrap();
        for uri in ["https://example.com/a", "mailto:a@b.c", "paper.pdf"] {
            let _ = state.update(Message::LinkClicked(uri.into()));
        }
        assert_eq!(
            opener.calls(),
            [
                OpenerCall::Link("https://example.com/a".into()),
                OpenerCall::Link("mailto:a@b.c".into()),
                OpenerCall::Link("paper.pdf".into()),
            ]
        );
        assert!(state.error.is_none());
        for uri in [
            "run.command",
            "ssh://host",
            "smb://host/share",
            "javascript:alert(1)",
            "../../etc/hosts.pdf",
        ] {
            let _ = state.update(Message::LinkClicked(uri.into()));
            assert!(state.error.is_some(), "{uri} should be blocked");
            state.error = None;
        }
        assert_eq!(
            opener.calls().len(),
            3,
            "a blocked link never reaches the opener"
        );
    }

    #[test]
    fn a_failing_opener_shows_an_error_with_a_retry() {
        let (mut state, _base, _root, opener) = with_opener("fails", RecordingOpener::failing());
        let _ = state.update(Message::LinkClicked("https://example.com".into()));
        assert_eq!(opener.calls().len(), 1);
        let notice = state.error.as_ref().expect("an error notice");
        assert!(notice.message.contains("Couldn't open link"));
        assert!(notice.retry.is_some());
    }

    #[test]
    fn choosing_an_editor_hands_the_linked_file_to_that_editor_only() {
        // Keep every binding: a `_` pattern would drop the scratch folder at once.
        let (mut state, _base, _root, file) = state_with_outside_file("editor");
        let opener = RecordingOpener::default();
        state.opener = Box::new(opener.clone());
        let _ = state.update(Message::LinkClicked(file.display().to_string()));
        let editor = crate::domain::editors::EDITORS[0].clone();
        let _ = state.update(Message::ExternalLinkOpenWith(editor.clone()));
        assert_eq!(
            opener.calls(),
            [OpenerCall::With(file, editor.command.into())]
        );
        assert!(state.menus.dialog.is_none() && state.nav.pending_external.is_none());
    }
}
