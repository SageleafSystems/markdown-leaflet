// SPDX-License-Identifier: GPL-3.0-or-later
//! Task-list checkbox toggles and the write they persist to disk (issue #68). Writes are
//! serialized per document: a toggle that arrives while one is already in flight only marks it
//! dirty, and the in-flight write's completion re-fires with the latest text, so rapid toggles
//! are coalesced rather than lost or racing each other.

use crate::app::message::SavedDocument;
use crate::app::{Message, Mode, SaveStatus, State};
use crate::domain::document::SaveError;
use crate::domain::tasks::TaskId;
use iced::Task;
use std::path::PathBuf;
use std::time::SystemTime;

impl State {
    pub(crate) fn update_tasks(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TaskToggled(id) => self.task_toggled(id),
            Message::DocumentSaved(path, saved) => self.document_saved(path, saved),
            _ => Task::none(),
        }
    }

    /// A checkbox was clicked: flip it in memory immediately, then write the file unless a
    /// write for this document is already running.
    fn task_toggled(&mut self, id: TaskId) -> Task<Message> {
        let Mode::Folder {
            selected: Some(doc),
            ..
        } = &mut self.mode
        else {
            return Task::none();
        };
        if doc.read_only {
            self.save_status = Some(SaveStatus::ReadOnly);
            return Task::none();
        }
        if !doc.toggle_task(id) {
            return Task::none();
        }
        if self.nav.write_in_flight {
            self.nav.write_dirty = true;
            return Task::none();
        }
        let (path, raw, mtime) = (doc.path.clone(), doc.raw.clone(), doc.mtime());
        self.spawn_save(path, raw, mtime)
    }

    fn spawn_save(
        &mut self,
        path: PathBuf,
        raw: String,
        mtime: Option<SystemTime>,
    ) -> Task<Message> {
        self.nav.write_in_flight = true;
        self.save_status = Some(SaveStatus::Saving);
        let future = SavedDocument::save(path.clone(), raw, mtime);
        Task::perform(future, move |saved| {
            Message::DocumentSaved(path.clone(), saved)
        })
    }

    /// A write finished (or failed). Ignored if the open document changed mid-write, or if a
    /// newer write for the same document already landed first.
    fn document_saved(&mut self, path: PathBuf, saved: SavedDocument) -> Task<Message> {
        let Some(result) = saved.take() else {
            return Task::none();
        };
        let Mode::Folder {
            selected: Some(doc),
            ..
        } = &mut self.mode
        else {
            return Task::none();
        };
        if doc.path != path {
            return Task::none();
        }
        self.nav.write_in_flight = false;
        let retry = match result {
            Ok(mtime) => {
                doc.note_written(mtime);
                if self.nav.write_dirty {
                    self.nav.write_dirty = false;
                    Some((doc.path.clone(), doc.raw.clone(), doc.mtime()))
                } else {
                    self.save_status = Some(SaveStatus::Saved(filename(&doc.path)));
                    None
                }
            }
            Err(SaveError::ReadOnly) => {
                self.save_status = Some(SaveStatus::ReadOnly);
                self.nav.write_dirty = false;
                None
            }
            // The next `reload_poll` tick will notice the file's real mtime no longer matches
            // ours and replace the document with the on-disk content; don't retry the write.
            Err(SaveError::ExternalChange) => {
                self.save_status = Some(SaveStatus::ExternalChange);
                self.nav.write_dirty = false;
                None
            }
            Err(SaveError::Io(err)) => {
                self.save_status = Some(SaveStatus::Failed(err.to_string()));
                None
            }
        };
        match retry {
            Some((path, raw, mtime)) => self.spawn_save(path, raw, mtime),
            None => Task::none(),
        }
    }
}

fn filename(path: &std::path::Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::document::Document;
    use std::fs;

    fn open_doc(dir: &std::path::Path, name: &str, body: &str) -> (State, PathBuf) {
        let path = dir.join(name);
        fs::write(&path, body).unwrap();
        let mut state = State::blank();
        let doc = Document::open(path.clone()).unwrap();
        state.mode = Mode::Folder {
            root: dir.to_path_buf(),
            files: vec![path.clone()],
            query: String::new(),
            selected: Some(doc),
        };
        (state, path)
    }

    fn doc_raw(state: &State) -> String {
        let Mode::Folder {
            selected: Some(doc),
            ..
        } = &state.mode
        else {
            panic!("expected a selected document");
        };
        doc.raw.clone()
    }

    #[test]
    fn toggling_writes_the_file_and_reports_saved() {
        let dir = crate::testutil::scratch_dir("tasks-save");
        let (mut state, path) = open_doc(&dir, "a.md", "- [ ] a\n");
        let _ = state.update(Message::TaskToggled(TaskId { item: 0, task: 0 }));
        assert!(state.nav.write_in_flight);
        assert_eq!(state.save_status, Some(SaveStatus::Saving));
        let mtime = fs::metadata(&path).unwrap().modified().unwrap();
        let _ = state.update(Message::DocumentSaved(
            path.clone(),
            SavedDocument::ready(Ok(mtime)),
        ));
        assert!(!state.nav.write_in_flight);
        assert_eq!(state.save_status, Some(SaveStatus::Saved("a.md".into())));
        // The actual `fs::write` is exercised by `domain::document::save`'s own tests; `Task`s
        // never run in a unit test (see `docs/ARCHITECTURE.md`), so here it's the in-memory
        // toggle and the `DocumentSaved` orchestration above that matter.
        assert!(doc_raw(&state).contains("[x]"));
    }

    #[test]
    fn a_read_only_file_is_rejected_without_writing() {
        let dir = crate::testutil::scratch_dir("tasks-readonly");
        let path = dir.join("a.md");
        fs::write(&path, "- [ ] a\n").unwrap();
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&path, perms).unwrap();
        // `read_only` is read at open time, so the file must already be read-only here.
        let doc = Document::open(path.clone()).unwrap();
        assert!(doc.read_only);
        let mut state = State::blank();
        state.mode = Mode::Folder {
            root: dir.to_path_buf(),
            files: vec![path.clone()],
            query: String::new(),
            selected: Some(doc),
        };
        let before = doc_raw(&state);
        let _ = state.update(Message::TaskToggled(TaskId { item: 0, task: 0 }));
        assert_eq!(state.save_status, Some(SaveStatus::ReadOnly));
        assert!(!state.nav.write_in_flight);
        assert_eq!(doc_raw(&state), before, "the checkbox did not flip");
    }

    #[test]
    fn a_toggle_while_a_write_is_in_flight_is_coalesced_not_lost() {
        let dir = crate::testutil::scratch_dir("tasks-coalesce");
        let (mut state, path) = open_doc(&dir, "a.md", "- [ ] a\n- [ ] b\n");
        let _ = state.update(Message::TaskToggled(TaskId { item: 0, task: 0 }));
        assert!(state.nav.write_in_flight);
        let _ = state.update(Message::TaskToggled(TaskId { item: 0, task: 1 }));
        assert!(state.nav.write_dirty, "the second toggle only marks dirty");
        assert!(doc_raw(&state).contains("- [x] a"));
        assert!(doc_raw(&state).contains("- [x] b"), "applied in memory");
        let mtime = fs::metadata(&path).unwrap().modified().unwrap();
        let _ = state.update(Message::DocumentSaved(
            path.clone(),
            SavedDocument::ready(Ok(mtime)),
        ));
        assert!(
            state.nav.write_in_flight,
            "the dirty toggle fires a follow-up write"
        );
        assert!(!state.nav.write_dirty);
    }

    #[test]
    fn an_external_change_sets_status_and_does_not_retry() {
        let dir = crate::testutil::scratch_dir("tasks-external");
        let (mut state, path) = open_doc(&dir, "a.md", "- [ ] a\n");
        let _ = state.update(Message::TaskToggled(TaskId { item: 0, task: 0 }));
        let _ = state.update(Message::DocumentSaved(
            path.clone(),
            SavedDocument::ready(Err(crate::domain::document::SaveError::ExternalChange)),
        ));
        assert_eq!(state.save_status, Some(SaveStatus::ExternalChange));
        assert!(!state.nav.write_in_flight);
        assert!(!state.nav.write_dirty);
    }

    #[test]
    fn a_save_for_a_document_that_is_no_longer_open_is_ignored() {
        let dir = crate::testutil::scratch_dir("tasks-stale");
        let (mut state, _path) = open_doc(&dir, "a.md", "- [ ] a\n");
        let other = dir.join("other.md");
        fs::write(&other, "# other").unwrap();
        let _ = state.update(Message::DocumentSaved(
            other,
            SavedDocument::ready(Ok(SystemTime::now())),
        ));
        assert!(state.save_status.is_none());
    }
}
