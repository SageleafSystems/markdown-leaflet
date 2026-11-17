// SPDX-License-Identifier: GPL-3.0-or-later
//! Naming a new file or folder in the nav: the draft, committing it, and discarding it.

use crate::app::{ErrorNotice, Message, Mode, NEW_ITEM_INPUT, State};
use crate::domain::create::{self, ItemKind, NewItem};
use crate::domain::folder::expand_file_ancestors;
use crate::gutter::{GutterSide, LeftTab};
use iced::Task;
use std::path::PathBuf;

impl State {
    pub(crate) fn update_new_item(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::NewMarkdownFile(dir) => self.begin_draft(ItemKind::File, dir),
            Message::NewFolder(dir) => self.begin_draft(ItemKind::Folder, dir),
            Message::RenameItem(path) => self.begin_rename(path),
            Message::NewItemChanged(text) => {
                if let Some(draft) = &mut self.nav.draft {
                    draft.text = text;
                    draft.failure = None;
                }
                Task::none()
            }
            Message::NewItemSubmit => self.commit_draft(true),
            Message::NewItemBlur => {
                if self.nav.draft_hover {
                    return Task::none();
                }
                let valid = self
                    .nav
                    .draft
                    .as_ref()
                    .is_some_and(|d| create::validate_item(d).is_ok());
                if valid {
                    self.commit_draft(false)
                } else {
                    self.cancel_draft();
                    Task::none()
                }
            }
            Message::NewItemCancel => {
                self.cancel_draft();
                Task::none()
            }
            Message::NewItemHover(hovered) => {
                self.nav.draft_hover = hovered;
                Task::none()
            }
            _ => unreachable!("new-item messages are routed by State::update"),
        }
    }

    pub(crate) fn cancel_draft(&mut self) {
        self.nav.draft = None;
        self.nav.draft_hover = false;
    }

    /// Opens the name field for a new item in `dir`, with its default name selected.
    fn begin_draft(&mut self, kind: ItemKind, dir: PathBuf) -> Task<Message> {
        if self.root().is_none() {
            return Task::none();
        }
        self.menus.context = None;
        // Switch back to Files so the field this opens is where it can be seen and used.
        self.gutters.left_tab = LeftTab::Files;
        // Open the folder so the new file appears in it once created.
        self.nav.expanded.insert(dir.clone());
        self.start_draft(NewItem::new(kind, dir), None)
    }

    /// Opens the name field on an existing file, with its name (minus the extension) selected.
    fn begin_rename(&mut self, path: PathBuf) -> Task<Message> {
        if self.root().is_none() {
            return Task::none();
        }
        self.menus.context = None;
        if let Some(reason) = create::folder_blocker(&path) {
            self.error = Some(ErrorNotice::new(reason, None));
            return Task::none();
        }
        let draft = NewItem::rename(path);
        let stem = draft.stem_len();
        self.start_draft(draft, Some(stem))
    }

    fn start_draft(&mut self, draft: NewItem, select_to: Option<usize>) -> Task<Message> {
        self.gutters.reveal(GutterSide::Left);
        self.nav.draft_hover = false;
        self.nav.draft = Some(draft);
        let select = match select_to {
            Some(end) => iced::widget::operation::select_range(NEW_ITEM_INPUT, 0, end),
            None => iced::widget::operation::select_all(NEW_ITEM_INPUT),
        };
        Task::batch([iced::widget::operation::focus(NEW_ITEM_INPUT), select])
    }

    /// Creates or renames to the draft's name. On failure the draft stays open with the reason;
    /// on success the list updates at once (the next folder scan confirms it) and, for Enter, a
    /// new file opens.
    fn commit_draft(&mut self, open: bool) -> Task<Message> {
        let Some(draft) = self.nav.draft.clone() else {
            return Task::none();
        };
        let path = match create::commit_item(&draft) {
            Ok(path) => path,
            Err(error) => {
                if let Some(draft) = &mut self.nav.draft {
                    draft.failure = Some(error.to_string());
                }
                return Task::none();
            }
        };
        self.cancel_draft();
        if let Some(source) = draft.rename {
            self.apply_rename(&source, path)
        } else {
            if let Mode::Folder { root, files, .. } = &mut self.mode {
                if !files.contains(&path) {
                    files.push(path.clone());
                }
                expand_file_ancestors(root, &path, &mut self.nav.expanded);
            }
            if open {
                self.update(Message::FileSelected(path))
            } else {
                self.nav.cursor = Some(path);
                Task::none()
            }
        }
    }

    /// Repoints the list, cursor, expanded folders, history, and open document from `source` (a
    /// file, or a folder with everything in it) to `path`.
    fn apply_rename(&mut self, source: &std::path::Path, path: PathBuf) -> Task<Message> {
        if source == path {
            return Task::none();
        }
        let rebase = |old: &std::path::Path| create::rebase(old, source, &path);
        let open_document = self.current_document().and_then(|doc| rebase(&doc.path));
        if let Mode::Folder { files, .. } = &mut self.mode {
            for file in files.iter_mut() {
                if let Some(moved) = rebase(file) {
                    *file = moved;
                }
            }
        }
        if let Some(moved) = self.nav.cursor.as_deref().and_then(rebase) {
            self.nav.cursor = Some(moved);
        }
        self.nav.expanded = self
            .nav
            .expanded
            .iter()
            .map(|folder| rebase(folder).unwrap_or_else(|| folder.clone()))
            .collect();
        self.nav.history.rename(source, &path);
        match open_document {
            // The same document under a new name: reopen it there. History already points at
            // the new path, so this adds no entry.
            Some(moved) => self.update(Message::FileSelected(moved)),
            None => Task::none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A state showing a scratch folder that holds one Markdown file.
    fn open_scratch(name: &str) -> (State, crate::testutil::ScratchDir) {
        let root = crate::testutil::scratch_dir(&format!("draft-{name}"));
        let existing = root.join("a.md");
        std::fs::write(&existing, "# a").unwrap();
        let mut state = State::blank();
        state.mode = Mode::Folder {
            root: root.to_path_buf(),
            files: vec![existing],
            query: String::new(),
            selected: None,
        };
        (state, root)
    }

    fn type_name(state: &mut State, text: &str) {
        let _ = state.update(Message::NewItemChanged(text.into()));
    }

    #[test]
    fn starting_a_draft_defaults_the_name_and_writes_nothing() {
        let (mut state, root) = open_scratch("start");
        let _ = state.update(Message::NewMarkdownFile(root.to_path_buf()));
        let draft = state.nav.draft.as_ref().unwrap();
        assert_eq!(
            (draft.text.as_str(), draft.parent.clone()),
            ("Untitled", root.to_path_buf())
        );
        assert!(!root.join("Untitled.md").exists());
    }

    #[test]
    fn starting_a_draft_in_a_folder_expands_it() {
        let (mut state, root) = open_scratch("expand");
        let sub = root.join("docs");
        std::fs::create_dir(&sub).unwrap();
        let _ = state.update(Message::NewFolder(sub.clone()));
        assert!(state.nav.expanded.contains(&sub));
    }

    #[test]
    fn escape_style_cancel_discards_without_touching_disk() {
        let (mut state, root) = open_scratch("cancel");
        let _ = state.update(Message::NewMarkdownFile(root.to_path_buf()));
        type_name(&mut state, "plan");
        let _ = state.update(Message::NewItemCancel);
        assert!(state.nav.draft.is_none());
        assert!(!root.join("plan.md").exists());
    }

    #[test]
    fn enter_on_a_valid_name_creates_the_file_and_lists_it() {
        let (mut state, root) = open_scratch("submit");
        let _ = state.update(Message::NewMarkdownFile(root.to_path_buf()));
        type_name(&mut state, "plan");
        let _ = state.update(Message::NewItemSubmit);
        assert!(state.nav.draft.is_none());
        assert!(root.join("plan.md").is_file());
        let Mode::Folder { files, .. } = &state.mode else {
            panic!("folder mode");
        };
        assert!(files.contains(&root.join("plan.md")));
        assert_eq!(state.nav.cursor, Some(root.join("plan.md")));
    }

    #[test]
    fn enter_on_an_invalid_name_keeps_the_draft_open() {
        let (mut state, root) = open_scratch("invalid");
        let _ = state.update(Message::NewMarkdownFile(root.to_path_buf()));
        type_name(&mut state, "a");
        // `a.md` exists already.
        let _ = state.update(Message::NewItemSubmit);
        assert!(state.nav.draft.is_some());
        type_name(&mut state, "notes.txt");
        let _ = state.update(Message::NewItemSubmit);
        assert!(state.nav.draft.is_some());
        assert!(!root.join("notes.txt").exists());
    }

    #[test]
    fn clicking_away_commits_a_valid_name_without_opening_it() {
        let (mut state, root) = open_scratch("blur-valid");
        let _ = state.update(Message::NewFolder(root.to_path_buf()));
        type_name(&mut state, "Specs");
        let _ = state.update(Message::NewItemBlur);
        assert!(state.nav.draft.is_none());
        assert!(root.join("Specs").join("Untitled.md").is_file());
    }

    #[test]
    fn clicking_away_discards_an_invalid_name() {
        let (mut state, root) = open_scratch("blur-invalid");
        let _ = state.update(Message::NewMarkdownFile(root.to_path_buf()));
        type_name(&mut state, "bad/name");
        let _ = state.update(Message::NewItemBlur);
        assert!(state.nav.draft.is_none());
    }

    #[test]
    fn a_press_on_the_field_itself_is_not_clicking_away() {
        let (mut state, root) = open_scratch("hover");
        let _ = state.update(Message::NewMarkdownFile(root.to_path_buf()));
        let _ = state.update(Message::NewItemHover(true));
        let _ = state.update(Message::NewItemBlur);
        assert!(state.nav.draft.is_some());
    }

    #[test]
    fn a_failed_create_stays_open_with_the_reason() {
        let (mut state, root) = open_scratch("fail");
        let _ = state.update(Message::NewMarkdownFile(root.to_path_buf()));
        // Remove the folder under the draft so that creating the file fails.
        std::fs::remove_dir_all(&root).unwrap();
        let _ = state.update(Message::NewItemSubmit);
        assert!(state.nav.draft.as_ref().unwrap().failure.is_some());
    }

    #[test]
    fn opening_another_folder_discards_the_draft() {
        let (mut state, root) = open_scratch("workspace");
        let _ = state.update(Message::NewMarkdownFile(root.to_path_buf()));
        state.show_workspace(crate::domain::workspace::Workspace::new(
            root.to_path_buf(),
            vec![],
        ));
        assert!(state.nav.draft.is_none());
    }

    #[test]
    fn renaming_updates_the_list_cursor_and_history() {
        let (mut state, root) = open_scratch("rename");
        let old = root.join("a.md");
        state.nav.cursor = Some(old.clone());
        state.nav.history.visit(old.clone());
        let _ = state.update(Message::RenameItem(old.clone()));
        assert_eq!(state.nav.draft.as_ref().unwrap().text, "a.md");
        type_name(&mut state, "b");
        let _ = state.update(Message::NewItemSubmit);
        let new = root.join("b.md");
        assert!(new.is_file() && !old.exists());
        let Mode::Folder { files, .. } = &state.mode else {
            panic!("folder mode");
        };
        assert_eq!(files, &vec![new.clone()]);
        assert_eq!(state.nav.cursor, Some(new.clone()));
        assert_eq!(state.nav.history.current().unwrap().path, new);
    }

    #[test]
    fn renaming_a_folder_moves_its_files_and_expansion() {
        let (mut state, root) = open_scratch("rename-folder");
        let docs = root.join("docs");
        std::fs::create_dir(&docs).unwrap();
        let inner = docs.join("x.md");
        std::fs::write(&inner, "x").unwrap();
        if let Mode::Folder { files, .. } = &mut state.mode {
            files.push(inner.clone());
        }
        state.nav.expanded.insert(docs.clone());
        state.nav.cursor = Some(inner.clone());
        let _ = state.update(Message::RenameItem(docs.clone()));
        type_name(&mut state, "guides");
        let _ = state.update(Message::NewItemSubmit);
        let guides = root.join("guides");
        assert!(guides.join("x.md").is_file() && !docs.exists());
        let Mode::Folder { files, .. } = &state.mode else {
            panic!("folder mode");
        };
        assert!(files.contains(&guides.join("x.md")));
        assert!(state.nav.expanded.contains(&guides));
        assert_eq!(state.nav.cursor, Some(guides.join("x.md")));
    }

    #[test]
    fn a_folder_with_other_files_will_not_start_renaming() {
        let (mut state, root) = open_scratch("rename-blocked");
        let docs = root.join("docs");
        std::fs::create_dir(&docs).unwrap();
        std::fs::write(docs.join("pic.png"), "").unwrap();
        let _ = state.update(Message::RenameItem(docs));
        assert!(state.nav.draft.is_none());
        assert!(state.error.is_some());
    }

    #[test]
    fn clicking_away_from_an_unchanged_rename_just_closes_it() {
        let (mut state, root) = open_scratch("rename-same");
        let old = root.join("a.md");
        let _ = state.update(Message::RenameItem(old.clone()));
        let _ = state.update(Message::NewItemBlur);
        assert!(state.nav.draft.is_none());
        assert!(old.is_file());
    }
}
