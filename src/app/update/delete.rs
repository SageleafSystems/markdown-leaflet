// SPDX-License-Identifier: GPL-3.0-or-later
//! Moving a file to the Trash, after the user confirms.

use crate::app::{Dialog, ErrorNotice, FindState, Message, Mode, State};
use crate::domain::create;
use iced::Task;
use std::path::Path;

impl State {
    pub(crate) fn update_delete(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::DeleteItem(path) => {
                self.menus.context = None;
                if let Some(reason) = create::folder_blocker(&path) {
                    self.error = Some(ErrorNotice::new(reason, None));
                    return Task::none();
                }
                self.nav.pending_delete = Some(path);
                self.open_dialog(Dialog::ConfirmDelete);
                Task::none()
            }
            Message::CancelDelete => {
                self.nav.pending_delete = None;
                self.close_dialog()
            }
            Message::ConfirmDelete => {
                let restore = self.close_dialog();
                let Some(path) = self.nav.pending_delete.take() else {
                    return restore;
                };
                // The folder may have changed since the dialog opened.
                if let Some(reason) = create::folder_blocker(&path) {
                    self.error = Some(ErrorNotice::new(reason, None));
                    return restore;
                }
                let work = match self.opener.trash(&path) {
                    Ok(()) => self.forget_path(&path),
                    Err(error) => {
                        self.error = Some(ErrorNotice::new(
                            format!("Couldn't move {} to the Trash: {error}", path.display()),
                            Some(Message::DeleteItem(path)),
                        ));
                        Task::none()
                    }
                };
                Task::batch([restore, work])
            }
            _ => unreachable!("delete messages are routed by State::update"),
        }
    }

    /// Drops a deleted file, or a folder and everything in it, from the list, and from the
    /// preview if the open document was inside it.
    fn forget_path(&mut self, path: &Path) -> Task<Message> {
        if let Mode::Folder {
            files, selected, ..
        } = &mut self.mode
        {
            files.retain(|file| !file.starts_with(path));
            if selected
                .as_ref()
                .is_some_and(|doc| doc.path.starts_with(path))
            {
                *selected = None;
                self.find = FindState::default();
                self.selection = None;
                self.selecting = false;
                self.leaves.clear();
            }
        }
        if self
            .nav
            .cursor
            .as_deref()
            .is_some_and(|c| c.starts_with(path))
        {
            self.nav.cursor = None;
            self.nav.invalidate_pending();
        }
        self.nav.expanded.retain(|folder| !folder.starts_with(path));
        self.load_images()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn state_with(name: &str) -> (State, crate::testutil::ScratchDir, PathBuf) {
        let root = crate::testutil::scratch_dir(&format!("delete-{name}"));
        let file = root.join("a.md");
        std::fs::write(&file, "# a").unwrap();
        let mut state = State::blank();
        state.opener = Box::new(crate::testutil::RecordingOpener::deleting());
        state.mode = Mode::Folder {
            root: root.to_path_buf(),
            files: vec![file.clone()],
            query: String::new(),
            selected: None,
        };
        (state, root, file)
    }

    #[test]
    fn delete_asks_first_and_cancel_keeps_the_file() {
        let (mut state, _root, file) = state_with("cancel");
        let _ = state.update(Message::DeleteItem(file.clone()));
        assert!(matches!(state.menus.dialog, Some(Dialog::ConfirmDelete)));
        assert!(file.exists());
        let _ = state.update(Message::CancelDelete);
        assert!(state.menus.dialog.is_none() && state.nav.pending_delete.is_none());
        assert!(file.exists());
    }

    #[test]
    fn enter_on_the_confirm_delete_dialog_confirms_it() {
        let (mut state, _root, file) = state_with("enter-confirms");
        let _ = state.update(Message::DeleteItem(file.clone()));
        let _ = state.dialog_default_action();
        assert!(!file.exists());
        assert!(state.menus.dialog.is_none());
    }

    #[test]
    fn confirming_removes_the_file_from_disk_list_and_cursor() {
        let (mut state, _root, file) = state_with("confirm");
        state.nav.cursor = Some(file.clone());
        let _ = state.update(Message::DeleteItem(file.clone()));
        let _ = state.update(Message::ConfirmDelete);
        assert!(!file.exists());
        assert!(state.menus.dialog.is_none());
        assert!(state.nav.cursor.is_none());
        let Mode::Folder { files, .. } = &state.mode else {
            panic!("folder mode");
        };
        assert!(files.is_empty());
    }

    #[test]
    fn a_failed_delete_shows_the_error_with_a_retry() {
        let (mut state, _root, file) = state_with("fail");
        let _ = state.update(Message::DeleteItem(file.clone()));
        std::fs::remove_file(&file).unwrap();
        let _ = state.update(Message::ConfirmDelete);
        assert!(state.error.is_some());
    }

    #[test]
    fn deleting_a_folder_of_markdown_drops_everything_under_it() {
        let (mut state, root, file) = state_with("folder");
        let docs = root.join("docs");
        std::fs::create_dir(&docs).unwrap();
        let inner = docs.join("x.md");
        std::fs::write(&inner, "x").unwrap();
        if let Mode::Folder { files, .. } = &mut state.mode {
            files.push(inner.clone());
        }
        state.nav.cursor = Some(inner);
        state.nav.expanded.insert(docs.clone());
        let _ = state.update(Message::DeleteItem(docs.clone()));
        let _ = state.update(Message::ConfirmDelete);
        assert!(!docs.exists() && file.exists());
        let Mode::Folder { files, .. } = &state.mode else {
            panic!("folder mode");
        };
        assert_eq!(files, &vec![file]);
        assert!(state.nav.cursor.is_none() && state.nav.expanded.is_empty());
    }

    #[test]
    fn a_folder_with_other_files_cannot_be_deleted() {
        let (mut state, root, _file) = state_with("blocked");
        let docs = root.join("docs");
        std::fs::create_dir(&docs).unwrap();
        std::fs::write(docs.join("pic.png"), "").unwrap();
        let _ = state.update(Message::DeleteItem(docs.clone()));
        assert!(state.menus.dialog.is_none() && state.error.is_some());
        assert!(docs.join("pic.png").exists());
    }
}
