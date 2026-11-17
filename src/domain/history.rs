// SPDX-License-Identifier: GPL-3.0-or-later
//! Browser-style back/forward history of the documents shown this session.
//! It knows nothing about rendering or the filesystem; the app decides when a
//! navigation is a new visit and when it is a step through existing entries.

use std::path::PathBuf;

/// One visited document and where the reader was in it.
#[derive(Debug, Clone)]
pub struct Entry {
    pub path: PathBuf,
    /// Vertical scroll offset when the reader last left this document.
    pub scroll: f32,
}

#[derive(Debug, Default)]
pub struct History {
    entries: Vec<Entry>,
    /// Position of the document on screen; meaningful only when `entries` is non-empty.
    index: usize,
}

impl History {
    /// Forgets every entry, as when a different workspace opens.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.index = 0;
    }

    /// Records a navigation to `path`. Revisiting the document already on
    /// screen adds nothing; otherwise any forward entries are dropped first.
    pub fn visit(&mut self, path: PathBuf) {
        if self.current().is_some_and(|entry| entry.path == path) {
            return;
        }
        if !self.entries.is_empty() {
            self.entries.truncate(self.index + 1);
        }
        self.entries.push(Entry { path, scroll: 0.0 });
        self.index = self.entries.len() - 1;
    }

    /// Repoints every entry at or inside `from` to the same place under `to`, after a file or
    /// folder was renamed.
    pub fn rename(&mut self, from: &std::path::Path, to: &std::path::Path) {
        for entry in &mut self.entries {
            if let Some(moved) = crate::domain::create::rebase(&entry.path, from, to) {
                entry.path = moved;
            }
        }
    }

    pub fn current(&self) -> Option<&Entry> {
        self.entries.get(self.index)
    }

    /// Stores the reading position of the document on screen.
    pub fn record_scroll(&mut self, scroll: f32) {
        if let Some(entry) = self.entries.get_mut(self.index) {
            entry.scroll = scroll;
        }
    }

    pub fn can_go_back(&self) -> bool {
        self.index > 0 && !self.entries.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        self.index + 1 < self.entries.len()
    }

    /// The index `steps` away (negative is back), if there is one.
    pub fn step_target(&self, steps: isize) -> Option<usize> {
        let target = self.index.checked_add_signed(steps)?;
        (!self.entries.is_empty() && target < self.entries.len() && target != self.index)
            .then_some(target)
    }

    pub fn entry(&self, index: usize) -> Option<&Entry> {
        self.entries.get(index)
    }

    /// Makes `index` the current entry; out-of-range indices are ignored.
    pub fn go(&mut self, index: usize) {
        if index < self.entries.len() {
            self.index = index;
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn renaming_a_file_repoints_its_entries() {
        let mut history = History::default();
        history.visit(PathBuf::from("/a.md"));
        history.visit(PathBuf::from("/b.md"));
        history.rename(std::path::Path::new("/a.md"), std::path::Path::new("/c.md"));
        assert_eq!(history.entry(0).unwrap().path, PathBuf::from("/c.md"));
        assert_eq!(history.entry(1).unwrap().path, PathBuf::from("/b.md"));
    }

    use super::*;

    fn history(names: &[&str]) -> History {
        let mut history = History::default();
        for name in names {
            history.visit(PathBuf::from(name));
        }
        history
    }

    #[test]
    fn starts_empty_with_nowhere_to_go() {
        let history = History::default();
        assert!(!history.can_go_back() && !history.can_go_forward());
        assert_eq!(history.step_target(-1), None);
        assert!(history.current().is_none());
    }

    #[test]
    fn visiting_the_current_document_adds_no_entry() {
        let mut history = history(&["a", "b", "b"]);
        assert_eq!(history.entries.len(), 2);
        history.go(0);
        history.visit("a".into());
        assert_eq!(history.entries.len(), 2);
    }

    #[test]
    fn steps_back_and_forward_within_bounds() {
        let mut history = history(&["a", "b", "c"]);
        assert!(history.can_go_back() && !history.can_go_forward());
        assert_eq!(history.step_target(-1), Some(1));
        history.go(1);
        assert!(history.can_go_back() && history.can_go_forward());
        history.go(0);
        assert_eq!(history.step_target(-1), None);
        assert_eq!(history.step_target(1), Some(1));
        assert_eq!(history.current().unwrap().path, PathBuf::from("a"));
    }

    #[test]
    fn navigating_after_going_back_discards_forward_entries() {
        let mut history = history(&["a", "b", "c"]);
        history.go(1);
        history.visit("d".into());
        let paths: Vec<_> = history.entries.iter().map(|e| e.path.clone()).collect();
        assert_eq!(
            paths,
            vec![PathBuf::from("a"), "b".into(), "d".into()],
            "A → B → C, back to B, then D leaves A → B → D"
        );
        assert!(!history.can_go_forward());
    }

    #[test]
    fn scroll_is_stored_on_the_current_entry() {
        let mut history = history(&["a", "b"]);
        history.record_scroll(120.0);
        history.go(0);
        history.record_scroll(40.0);
        assert_eq!(history.entry(0).unwrap().scroll, 40.0);
        assert_eq!(history.entry(1).unwrap().scroll, 120.0);
    }

    #[test]
    fn clear_forgets_everything() {
        let mut history = history(&["a", "b"]);
        history.clear();
        assert!(history.current().is_none() && !history.can_go_back());
    }
}
