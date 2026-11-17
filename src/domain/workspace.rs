// SPDX-License-Identifier: GPL-3.0-or-later
//! Opening a file or folder into a browsable workspace, without any UI.

use super::document::{Document, DocumentError};
use super::recent;
use super::walk::{Scan, ScanRules, scan_markdown_files};
use std::path::{Path, PathBuf};

/// A scanned folder and, when one could be restored, the open document.
#[derive(Debug)]
pub struct Workspace {
    pub root: PathBuf,
    pub files: Vec<PathBuf>,
    pub selected: Option<Document>,
    /// Why a requested selection could not be restored.
    pub issue: Option<SelectionIssue>,
    /// Folders in the tree that could not be listed, so their files are missing.
    pub unreadable: Vec<PathBuf>,
}

#[derive(Debug, thiserror::Error)]
pub enum SelectionIssue {
    /// The file is no longer in the folder.
    #[error(
        "Previously selected file is unavailable: {}. Choose another file from the sidebar.",
        .0.display()
    )]
    Unavailable(PathBuf),
    #[error(transparent)]
    Unreadable(DocumentError),
}

#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    #[error("Folder is unavailable: {}", .0.display())]
    FolderUnavailable(PathBuf),
    #[error(transparent)]
    Unreadable(#[from] DocumentError),
}

/// What to open: a file in its folder, or a folder with an optional file to select. Describes the
/// work without doing it, so the scan and the document read can run off the interface thread and
/// the same request can be retried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenRequest {
    File(PathBuf),
    Folder {
        root: PathBuf,
        selected: Option<PathBuf>,
    },
}

impl OpenRequest {
    /// Opens `path` in its folder. The path is made absolute so a retry does not depend on the
    /// working directory.
    pub fn file(path: &Path) -> Self {
        Self::File(recent::absolute(path))
    }

    pub fn folder(root: PathBuf, selected: Option<PathBuf>) -> Self {
        Self::Folder { root, selected }
    }

    /// The request a recent item stands for.
    pub fn from_recent(item: &recent::RecentItem) -> Self {
        match item.kind {
            recent::RecentKind::File => Self::file(&item.path),
            recent::RecentKind::Folder => Self::folder(item.path.clone(), item.selected.clone()),
        }
    }

    /// The name shown while it is opening.
    pub fn label(&self) -> String {
        let path = match self {
            Self::File(path) | Self::Folder { root: path, .. } => path,
        };
        path.file_name()
            .unwrap_or(path.as_os_str())
            .to_string_lossy()
            .into_owned()
    }

    /// Scans and reads. This is the slow part; call it on a worker.
    pub fn run(&self, rules: &ScanRules) -> Result<Workspace, OpenError> {
        match self {
            Self::File(path) => Workspace::open_file(path.clone(), rules),
            Self::Folder { root, selected } => {
                Workspace::open_folder(root.clone(), selected.clone(), rules)
            }
        }
    }
}

impl Workspace {
    /// A scanned folder with nothing selected.
    pub fn new(root: PathBuf, files: Vec<PathBuf>) -> Self {
        Self {
            root,
            files,
            selected: None,
            issue: None,
            unreadable: Vec::new(),
        }
    }

    /// A scanned folder with nothing selected, keeping what the scan could not read.
    pub fn from_scan(root: PathBuf, scan: Scan) -> Self {
        Self {
            unreadable: scan.unreadable,
            ..Self::new(root, scan.files)
        }
    }

    /// Opens `path` inside its parent folder. The file is listed even when
    /// the scan rules would exclude it.
    pub fn open_file(path: PathBuf, rules: &ScanRules) -> Result<Self, OpenError> {
        let path = recent::absolute(&path);
        let doc = Document::open(path.clone())?;
        let root = doc
            .path
            .parent()
            .map(Path::to_path_buf)
            .ok_or(OpenError::FolderUnavailable(path))?;
        let scan = scan_markdown_files(&root, rules);
        let files = include_selected(scan.files, Some(&doc.path));
        Ok(Self {
            root,
            files,
            selected: Some(doc),
            issue: None,
            unreadable: scan.unreadable,
        })
    }

    /// Scans `root` and tries to restore `selected` within it.
    pub fn open_folder(
        root: PathBuf,
        selected: Option<PathBuf>,
        rules: &ScanRules,
    ) -> Result<Self, OpenError> {
        if !root.is_dir() {
            return Err(OpenError::FolderUnavailable(root));
        }
        let root = recent::absolute(&root);
        let Scan { files, unreadable } = scan_markdown_files(&root, rules);
        let (selected, issue) = match selected {
            Some(path) if files.contains(&path) => match Document::open(path.clone()) {
                Ok(doc) => (Some(doc), None),
                Err(error) => (None, Some(SelectionIssue::Unreadable(error))),
            },
            Some(path) => (None, Some(SelectionIssue::Unavailable(path))),
            None => match default_selection(&root, &files) {
                Some(path) => match Document::open(path) {
                    Ok(doc) => (Some(doc), None),
                    Err(_) => (None, None),
                },
                None => (None, None),
            },
        };
        Ok(Self {
            root,
            files,
            selected,
            issue,
            unreadable,
        })
    }
}

/// The file to open automatically when a folder is opened with nothing selected: a `README.md`
/// (any case) directly in `root`, or the only file in `files` when there is exactly one.
fn default_selection(root: &Path, files: &[PathBuf]) -> Option<PathBuf> {
    files
        .iter()
        .find(|path| {
            path.parent() == Some(root)
                && path
                    .file_name()
                    .is_some_and(|name| name.eq_ignore_ascii_case("README.md"))
        })
        .or(match files {
            [only] => Some(only),
            _ => None,
        })
        .cloned()
}

/// Keeps a still-existing selected file in a fresh scan so rules that
/// exclude it do not make it vanish from the list.
pub fn include_selected(mut scanned: Vec<PathBuf>, selected: Option<&Path>) -> Vec<PathBuf> {
    if let Some(path) = selected
        && path.exists()
        && !scanned.iter().any(|file| file == path)
    {
        scanned.push(path.to_path_buf());
        scanned.sort();
    }
    scanned
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(label: &str) -> (crate::testutil::ScratchDir, PathBuf) {
        let root = crate::testutil::scratch_dir(&format!("workspace-{label}"));
        std::fs::create_dir_all(root.join("node_modules")).unwrap();
        std::fs::write(root.join("a.md"), "# a").unwrap();
        std::fs::write(root.join("node_modules/ignored.md"), "# ignored").unwrap();
        // Paths are canonicalized on open (/var is /private/var on macOS).
        let real = std::fs::canonicalize(&*root).unwrap();
        (root, real)
    }

    #[test]
    fn open_file_lists_a_file_the_scan_rules_exclude() {
        let (_guard, root) = fixture("excluded-file");
        let path = root.join("node_modules/ignored.md");
        let ws = Workspace::open_file(path.clone(), &ScanRules::default()).unwrap();
        assert_eq!(ws.root, root.join("node_modules"));
        assert_eq!(ws.files, vec![path.clone()]);
        assert_eq!(ws.selected.unwrap().path, path);
    }

    #[test]
    fn open_file_reports_an_unreadable_file() {
        let (_guard, root) = fixture("missing-file");
        let result = Workspace::open_file(root.join("nope.md"), &ScanRules::default());
        assert!(matches!(result, Err(OpenError::Unreadable(_))));
    }

    #[test]
    fn open_folder_requires_a_directory() {
        let (_guard, root) = fixture("not-a-dir");
        let result = Workspace::open_folder(root.join("a.md"), None, &ScanRules::default());
        assert!(matches!(result, Err(OpenError::FolderUnavailable(_))));
    }

    #[test]
    fn open_folder_restores_or_explains_the_selection() {
        let (_guard, root) = fixture("restore");
        let rules = ScanRules::default();
        let ws = Workspace::open_folder(root.clone(), Some(root.join("a.md")), &rules).unwrap();
        assert_eq!(ws.files, vec![root.join("a.md")]);
        assert!(ws.selected.is_some() && ws.issue.is_none());

        let gone = root.join("gone.md");
        let ws = Workspace::open_folder(root.clone(), Some(gone.clone()), &rules).unwrap();
        assert!(ws.selected.is_none());
        assert!(matches!(ws.issue, Some(SelectionIssue::Unavailable(path)) if path == gone));
    }

    #[test]
    fn open_folder_auto_selects_a_readme_when_nothing_is_requested() {
        let root = crate::testutil::scratch_dir("workspace-readme");
        std::fs::write(root.join("README.md"), "# readme").unwrap();
        std::fs::write(root.join("other.md"), "# other").unwrap();
        let real = std::fs::canonicalize(&*root).unwrap();
        let ws = Workspace::open_folder(real.clone(), None, &ScanRules::default()).unwrap();
        assert_eq!(ws.selected.unwrap().path, real.join("README.md"));
    }

    #[test]
    fn open_folder_auto_selects_the_only_file_in_the_root() {
        let root = crate::testutil::scratch_dir("workspace-only-file");
        std::fs::write(root.join("notes.md"), "# notes").unwrap();
        let real = std::fs::canonicalize(&*root).unwrap();
        let ws = Workspace::open_folder(real.clone(), None, &ScanRules::default()).unwrap();
        assert_eq!(ws.selected.unwrap().path, real.join("notes.md"));
    }

    #[test]
    fn open_folder_leaves_nothing_selected_without_a_readme_or_single_file() {
        let (_guard, root) = fixture("ambiguous");
        std::fs::write(root.join("b.md"), "# b").unwrap();
        let ws = Workspace::open_folder(root.clone(), None, &ScanRules::default()).unwrap();
        assert!(ws.selected.is_none());
    }

    #[test]
    fn include_selected_keeps_an_existing_file_and_drops_a_missing_one() {
        let (_guard, root) = fixture("include");
        let hidden = root.join("node_modules/ignored.md");
        let kept = include_selected(vec![root.join("a.md")], Some(&hidden));
        assert_eq!(kept, vec![root.join("a.md"), hidden]);
        let gone = root.join("gone.md");
        let unchanged = include_selected(vec![root.join("a.md")], Some(&gone));
        assert_eq!(unchanged, vec![root.join("a.md")]);
    }

    #[cfg(unix)]
    #[test]
    fn open_folder_keeps_the_folders_it_could_not_read() {
        use std::os::unix::fs::PermissionsExt;
        let (_guard, root) = fixture("unreadable");
        std::fs::create_dir(root.join("private")).unwrap();
        std::fs::write(root.join("private/p.md"), "# p").unwrap();
        let locked = root.join("private");
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
        if std::fs::read_dir(&locked).is_ok() {
            return; // permissions are not enforced (running as root)
        }
        let ws = Workspace::open_folder(root.clone(), None, &ScanRules::default()).unwrap();
        assert_eq!(ws.unreadable, vec![locked]);
        assert_eq!(ws.files, vec![root.join("a.md")]);
    }

    #[test]
    fn an_open_request_names_itself_and_runs_to_the_same_workspace_as_a_direct_open() {
        let (_guard, root) = fixture("request");
        let rules = ScanRules::default();
        let folder = OpenRequest::folder(root.clone(), None);
        assert_eq!(folder.label(), root.file_name().unwrap().to_string_lossy());
        let ws = folder.run(&rules).unwrap();
        assert_eq!(ws.files, vec![root.join("a.md")]);
        let file = OpenRequest::file(&root.join("a.md"));
        assert_eq!(file.label(), "a.md");
        let ws = file.run(&rules).unwrap();
        assert_eq!(ws.selected.unwrap().path, root.join("a.md"));
        assert!(
            OpenRequest::folder(root.join("gone"), None)
                .run(&rules)
                .is_err()
        );
        let item = recent::RecentItem {
            kind: recent::RecentKind::Folder,
            path: root.clone(),
            selected: Some(root.join("a.md")),
        };
        assert_eq!(
            OpenRequest::from_recent(&item),
            OpenRequest::folder(root.clone(), Some(root.join("a.md")))
        );
    }
}
