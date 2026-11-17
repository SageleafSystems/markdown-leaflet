// SPDX-License-Identifier: GPL-3.0-or-later
//! Folder browsing rules shared by the sidebar and keyboard navigation.

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(crate) struct FolderNode {
    pub(crate) folders: BTreeMap<String, FolderNode>,
    pub(crate) files: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NavEntry {
    Folder(PathBuf),
    File(PathBuf),
}

impl NavEntry {
    pub(crate) fn path(&self) -> &Path {
        match self {
            Self::Folder(path) | Self::File(path) => path,
        }
    }
}

impl FolderNode {
    pub(crate) fn from_files(root: &Path, files: &[PathBuf]) -> Self {
        let mut tree = Self::default();
        for file in files {
            let Ok(relative) = file.strip_prefix(root) else {
                continue;
            };
            let mut node = &mut tree;
            if let Some(parent) = relative.parent() {
                for part in parent.components() {
                    node = node
                        .folders
                        .entry(part.as_os_str().to_string_lossy().into_owned())
                        .or_default();
                }
            }
            node.files.push(file.clone());
        }
        tree
    }

    pub(crate) fn visible_entries(
        &self,
        root: &Path,
        expanded: &BTreeSet<PathBuf>,
        output: &mut Vec<NavEntry>,
    ) {
        for (name, child) in &self.folders {
            let path = root.join(name);
            output.push(NavEntry::Folder(path.clone()));
            if expanded.contains(&path) {
                child.visible_entries(&path, expanded, output);
            }
        }
        output.extend(self.files.iter().cloned().map(NavEntry::File));
    }
}

pub(crate) fn parent_key(root: &Path, path: &Path) -> PathBuf {
    path.parent()
        .and_then(|parent| parent.strip_prefix(root).ok())
        .unwrap_or(Path::new(""))
        .to_path_buf()
}

pub(crate) fn expand_file_ancestors(root: &Path, file: &Path, expanded: &mut BTreeSet<PathBuf>) {
    let mut folder = file.parent();
    while let Some(path) = folder.filter(|path| *path != root) {
        expanded.insert(path.to_path_buf());
        folder = path.parent();
    }
}

pub(crate) fn matching_files<'a>(
    root: &Path,
    files: &'a [PathBuf],
    query: &str,
) -> Vec<&'a PathBuf> {
    if query.trim().is_empty() {
        return files.iter().collect();
    }
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, &PathBuf)> = files
        .iter()
        .filter_map(|path| {
            let name = path.file_name()?.to_string_lossy();
            let relative = path.strip_prefix(root).ok()?.to_string_lossy();
            matcher
                .fuzzy_match(&name, query)
                .or_else(|| matcher.fuzzy_match(&relative, query))
                .map(|score| (score, path))
        })
        .collect();
    scored.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
    scored.into_iter().map(|(_, path)| path).collect()
}

/// Returns the next visible row; arrow keys stop at either end.
pub(crate) fn selection_target<'a>(
    entries: &'a [NavEntry],
    selected: Option<&Path>,
    direction: isize,
) -> Option<&'a NavEntry> {
    if entries.is_empty() {
        return None;
    }
    let current = selected.and_then(|path| {
        entries
            .iter()
            .position(|candidate| candidate.path() == path)
    });
    let next = match (current, direction) {
        (Some(index), n) if n > 0 => (index + 1).min(entries.len() - 1),
        (Some(index), _) => index.saturating_sub(1),
        (None, n) if n < 0 => entries.len() - 1,
        (None, _) => 0,
    };
    Some(&entries[next])
}

/// Whether `path`'s row is highlighted: the keyboard cursor wins when set,
/// otherwise the open document's row is.
pub(crate) fn is_highlighted(cursor: Option<&Path>, open: Option<&Path>, path: &Path) -> bool {
    cursor.map_or_else(|| open == Some(path), |cursor| cursor == path)
}

#[cfg(test)]
mod tests {
    use super::{
        FolderNode, NavEntry, expand_file_ancestors, is_highlighted, matching_files,
        selection_target,
    };
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    #[test]
    fn folder_tree_starts_closed_and_reveals_nested_files_in_display_order() {
        let root = Path::new("/notes");
        let files = [
            "/notes/readme.md",
            "/notes/a/one.md",
            "/notes/a/b/two.md",
            "/notes/z/three.md",
        ]
        .map(PathBuf::from);
        let tree = FolderNode::from_files(root, &files);
        assert_eq!(
            tree.folders.keys().map(String::as_str).collect::<Vec<_>>(),
            ["a", "z"]
        );

        let mut expanded = BTreeSet::new();
        let mut visible = Vec::new();
        tree.visible_entries(root, &expanded, &mut visible);
        assert_eq!(
            visible,
            [
                NavEntry::Folder("/notes/a".into()),
                NavEntry::Folder("/notes/z".into()),
                NavEntry::File("/notes/readme.md".into())
            ]
        );

        expand_file_ancestors(root, Path::new("/notes/a/b/two.md"), &mut expanded);
        visible.clear();
        tree.visible_entries(root, &expanded, &mut visible);
        assert_eq!(
            visible,
            [
                NavEntry::Folder("/notes/a".into()),
                NavEntry::Folder("/notes/a/b".into()),
                NavEntry::File("/notes/a/b/two.md".into()),
                NavEntry::File("/notes/a/one.md".into()),
                NavEntry::Folder("/notes/z".into()),
                NavEntry::File("/notes/readme.md".into()),
            ]
        );
    }

    #[test]
    fn keyboard_selection_follows_filtered_results_and_stops_at_ends() {
        let root = Path::new("/notes");
        let files =
            ["/notes/other.md", "/notes/a/guide.md", "/notes/b/guide.md"].map(PathBuf::from);
        let matches: Vec<NavEntry> = matching_files(root, &files, "guide")
            .into_iter()
            .cloned()
            .map(NavEntry::File)
            .collect();
        assert_eq!(matches.len(), 2);
        assert_eq!(selection_target(&matches, None, 1), Some(&matches[0]));
        assert_eq!(selection_target(&matches, None, -1), Some(&matches[1]));
        assert_eq!(
            selection_target(&matches, Some(matches[1].path()), 1),
            Some(&matches[1])
        );
        assert_eq!(
            selection_target(&matches, Some(matches[0].path()), -1),
            Some(&matches[0])
        );
    }

    #[test]
    fn the_cursor_overrides_the_open_document_highlight() {
        let a = Path::new("/n/a.md");
        let b = Path::new("/n/b.md");
        assert!(is_highlighted(None, Some(a), a));
        assert!(!is_highlighted(None, Some(a), b));
        assert!(is_highlighted(Some(b), Some(a), b));
        assert!(!is_highlighted(Some(b), Some(a), a));
    }
}
