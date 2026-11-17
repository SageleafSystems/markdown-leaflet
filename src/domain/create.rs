// SPDX-License-Identifier: GPL-3.0-or-later
//! Naming and creating new Markdown files and folders from the file list.
//!
//! Nothing touches the disk until the user commits a name. The nav lists only folders that hold
//! Markdown, so a new folder is created with a blank `Untitled.md` inside it; otherwise it would
//! not appear.

use crate::domain::walk::{MARKDOWN_EXTENSIONS, TEXT_EXTENSIONS, is_markdown_path, is_text_path};
use std::path::{Path, PathBuf};

const FILE_STEM: &str = "Untitled";
const FOLDER_NAME: &str = "New Folder";
const DEFAULT_EXT: &str = "md";
/// The longest file name most filesystems accept, in bytes.
const MAX_NAME_BYTES: usize = 255;
const FORBIDDEN: &[char] = &['/', '\\', '<', '>', ':', '"', '|', '?', '*'];
const WINDOWS_RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ItemKind {
    File,
    Folder,
}

/// A new item being named: nothing exists on disk yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewItem {
    pub(crate) kind: ItemKind,
    pub(crate) parent: PathBuf,
    pub(crate) text: String,
    /// Why the last attempt failed on disk, shown until the text changes.
    pub(crate) failure: Option<String>,
    /// The existing file being renamed, when this draft renames instead of creating.
    pub(crate) rename: Option<PathBuf>,
}

impl NewItem {
    /// A draft in `parent`, named with the first free default.
    pub(crate) fn new(kind: ItemKind, parent: PathBuf) -> Self {
        let text = default_text(&parent, kind);
        Self {
            kind,
            parent,
            text,
            failure: None,
            rename: None,
        }
    }

    /// A draft that renames the existing file or folder `path`, starting from its current name.
    pub(crate) fn rename(path: PathBuf) -> Self {
        let kind = if path.is_dir() {
            ItemKind::Folder
        } else {
            ItemKind::File
        };
        Self {
            kind,
            parent: path.parent().map(Path::to_path_buf).unwrap_or_default(),
            text: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            failure: None,
            rename: Some(path),
        }
    }

    /// How many characters of the starting name precede its extension, so renaming can select
    /// just the stem.
    pub(crate) fn stem_len(&self) -> usize {
        if self.kind == ItemKind::Folder {
            return self.text.chars().count();
        }
        Path::new(&self.text)
            .file_stem()
            .map_or(0, |stem| stem.to_string_lossy().chars().count())
    }

    /// Whether the draft renames a plain-text file, which keeps `.txt` rather than gaining `.md`.
    fn is_text_rename(&self) -> bool {
        self.rename.as_deref().is_some_and(is_text_path)
    }

    /// The extension that will be added to the typed name (`md`, or `txt` when renaming a text
    /// file), so the field can hint at it; `None` when the name already has one.
    pub(crate) fn added_extension(&self) -> Option<&'static str> {
        (self.kind == ItemKind::File && Path::new(self.text.trim()).extension().is_none())
            .then_some(if self.is_text_rename() {
                "txt"
            } else {
                DEFAULT_EXT
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NameError {
    Empty,
    InvalidChars,
    Reserved,
    TooLong,
    NotMarkdown,
    Exists(String),
    Io(String),
}

impl std::fmt::Display for NameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => f.write_str("Enter a name."),
            Self::InvalidChars => {
                f.write_str("Names can't contain / \\ : * ? \" < > | or end in a dot or space.")
            }
            Self::Reserved => f.write_str("That name is reserved by the system."),
            Self::TooLong => f.write_str("That name is too long."),
            Self::NotMarkdown => f.write_str("Name must end in .md (or leave the extension off)."),
            Self::Exists(name) => write!(f, "“{name}” already exists here."),
            Self::Io(error) => write!(f, "Couldn't create it: {error}"),
        }
    }
}

/// The first of `stem`, `stem 2`, `stem 3`… that is free in `dir`, counting `ext` for a file.
fn free_stem(dir: &Path, stem: &str, ext: Option<&str>) -> String {
    (1..=10_000_u32)
        .map(|n| {
            if n == 1 {
                stem.to_owned()
            } else {
                format!("{stem} {n}")
            }
        })
        .find(|candidate| {
            let name = match ext {
                Some(ext) => format!("{candidate}.{ext}"),
                None => candidate.clone(),
            };
            dir.join(name).symlink_metadata().is_err()
        })
        // A folder with ten thousand numbered copies is not real; `validate` reports a clash.
        .unwrap_or_else(|| stem.to_owned())
}

/// What a new item's field starts with: `Untitled` or `New Folder`, numbered if taken. A file's
/// `.md` is left off; [`validate`] adds it.
pub(crate) fn default_text(dir: &Path, kind: ItemKind) -> String {
    match kind {
        ItemKind::File => free_stem(dir, FILE_STEM, Some(DEFAULT_EXT)),
        ItemKind::Folder => free_stem(dir, FOLDER_NAME, None),
    }
}

/// The path `text` would create in `parent`, or why it can't be used.
pub(crate) fn validate(parent: &Path, kind: ItemKind, text: &str) -> Result<PathBuf, NameError> {
    validate_as(parent, kind, text, false)
}

/// [`validate`], where `text_file` says the name is for a plain-text file being renamed: it
/// keeps `.txt` (or any Markdown extension) instead of gaining `.md` (issue #79).
fn validate_as(
    parent: &Path,
    kind: ItemKind,
    text: &str,
    text_file: bool,
) -> Result<PathBuf, NameError> {
    let name = text.trim();
    if name.is_empty() {
        return Err(NameError::Empty);
    }
    if name == "."
        || name == ".."
        || name.contains(FORBIDDEN)
        || name.chars().any(char::is_control)
        || name.ends_with('.')
    {
        return Err(NameError::InvalidChars);
    }
    let base = name.split('.').next().unwrap_or(name).to_uppercase();
    if WINDOWS_RESERVED.contains(&base.as_str()) {
        return Err(NameError::Reserved);
    }
    let name = match kind {
        ItemKind::Folder => name.to_owned(),
        ItemKind::File => match Path::new(name).extension().and_then(|ext| ext.to_str()) {
            None if text_file => format!("{name}.{}", TEXT_EXTENSIONS[0]),
            None => format!("{name}.{DEFAULT_EXT}"),
            Some(ext)
                if MARKDOWN_EXTENSIONS.contains(&ext.to_lowercase().as_str())
                    || (text_file && TEXT_EXTENSIONS.contains(&ext.to_lowercase().as_str())) =>
            {
                name.to_owned()
            }
            Some(_) => return Err(NameError::NotMarkdown),
        },
    };
    if name.len() > MAX_NAME_BYTES {
        return Err(NameError::TooLong);
    }
    let path = parent.join(&name);
    if path.symlink_metadata().is_ok() {
        return Err(NameError::Exists(name));
    }
    Ok(path)
}

/// Files an OS drops into folders on its own; they don't make a folder "not just Markdown".
const JUNK: &[&str] = &[".DS_Store", "Thumbs.db", "desktop.ini"];

/// A file that may sit in a folder being renamed or trashed: Markdown, or plain text (issue #79).
fn is_listable(path: &Path) -> bool {
    is_markdown_path(path) || is_text_path(path)
}

/// The first entry under `dir` that is not a Markdown file or a folder (junk files aside).
/// Anything unreadable counts, since it can't be shown safe.
fn first_foreign_entry(dir: &Path) -> Option<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Some(dir.to_path_buf());
    };
    for entry in entries {
        let Ok(entry) = entry else {
            return Some(dir.to_path_buf());
        };
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            return Some(path);
        };
        if kind.is_dir() {
            if let Some(found) = first_foreign_entry(&path) {
                return Some(found);
            }
        } else if !(kind.is_file() && is_listable(&path)) {
            let junk = path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| JUNK.contains(&name));
            if !junk {
                return Some(path);
            }
        }
    }
    None
}

/// Why `path` can't be renamed or deleted from here, or `None` if it can. Files always can; a
/// folder only if everything inside it is Markdown, so an app for reading Markdown never
/// touches other work.
pub(crate) fn folder_blocker(path: &Path) -> Option<String> {
    if !path.is_dir() {
        return None;
    }
    let foreign = first_foreign_entry(path)?;
    let name = foreign.file_name().unwrap_or_default().to_string_lossy();
    Some(format!(
        "This folder contains “{name}”, which isn't a Markdown file. Only folders holding \
         nothing but Markdown can be renamed or deleted here."
    ))
}

/// `path` with its leading `from` replaced by `to`, if it is `from` or inside it.
pub(crate) fn rebase(path: &Path, from: &Path, to: &Path) -> Option<PathBuf> {
    let rest = path.strip_prefix(from).ok()?;
    Some(if rest.as_os_str().is_empty() {
        to.to_path_buf()
    } else {
        to.join(rest)
    })
}

/// What the draft would create or rename to, or why it can't. A rename may keep its own name
/// (or change only its case), which is not a clash with itself.
pub(crate) fn validate_item(item: &NewItem) -> Result<PathBuf, NameError> {
    let result = validate_as(&item.parent, item.kind, &item.text, item.is_text_rename());
    match (&item.rename, result) {
        (Some(source), Err(NameError::Exists(name))) => {
            let path = item.parent.join(&name);
            match (std::fs::canonicalize(&path), std::fs::canonicalize(source)) {
                (Ok(a), Ok(b)) if a == b => Ok(path),
                _ => Err(NameError::Exists(name)),
            }
        }
        (_, result) => result,
    }
}

/// Creates or renames to the draft's name. For a new item this returns the Markdown file to open
/// (see [`commit`]); for a rename, the file's new path, which is its old one if the name is
/// unchanged.
pub(crate) fn commit_item(item: &NewItem) -> Result<PathBuf, NameError> {
    let Some(source) = &item.rename else {
        return commit(&item.parent, item.kind, &item.text);
    };
    let path = validate_item(item)?;
    if path != *source {
        if let Some(reason) = folder_blocker(source) {
            return Err(NameError::Io(reason));
        }
        std::fs::rename(source, &path).map_err(|error| NameError::Io(error.to_string()))?;
    }
    Ok(path)
}

fn create_blank(path: &Path) -> Result<(), NameError> {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map(drop)
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::AlreadyExists => NameError::Exists(
                path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
            ),
            _ => NameError::Io(error.to_string()),
        })
}

/// Creates the validated item and returns the Markdown file to open: the file itself, or the
/// blank `Untitled.md` inside a new folder. A folder whose file can't be made is removed again.
pub(crate) fn commit(parent: &Path, kind: ItemKind, text: &str) -> Result<PathBuf, NameError> {
    let path = validate(parent, kind, text)?;
    match kind {
        ItemKind::File => {
            create_blank(&path)?;
            Ok(path)
        }
        ItemKind::Folder => {
            std::fs::create_dir(&path).map_err(|error| match error.kind() {
                std::io::ErrorKind::AlreadyExists => NameError::Exists(text.trim().to_owned()),
                _ => NameError::Io(error.to_string()),
            })?;
            let file = path.join(format!("{FILE_STEM}.{DEFAULT_EXT}"));
            if let Err(error) = create_blank(&file) {
                let _ = std::fs::remove_dir(&path);
                return Err(error);
            }
            Ok(file)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> crate::testutil::ScratchDir {
        crate::testutil::scratch_dir(&format!("create-{name}"))
    }

    #[test]
    fn defaults_take_the_next_free_name() {
        let dir = scratch("defaults");
        assert_eq!(default_text(&dir, ItemKind::File), "Untitled");
        assert_eq!(default_text(&dir, ItemKind::Folder), "New Folder");
        std::fs::write(dir.join("Untitled.md"), "").unwrap();
        std::fs::create_dir(dir.join("New Folder")).unwrap();
        assert_eq!(default_text(&dir, ItemKind::File), "Untitled 2");
        assert_eq!(default_text(&dir, ItemKind::Folder), "New Folder 2");
    }

    #[test]
    fn a_file_name_gains_md_unless_it_has_a_markdown_extension() {
        let dir = scratch("ext");
        let name = |text| validate(&dir, ItemKind::File, text).unwrap();
        assert_eq!(name("notes"), dir.join("notes.md"));
        assert_eq!(name("  notes  "), dir.join("notes.md"));
        assert_eq!(name("notes.md"), dir.join("notes.md"));
        assert_eq!(name("Notes.MARKDOWN"), dir.join("Notes.MARKDOWN"));
        assert_eq!(
            validate(&dir, ItemKind::File, "notes.txt"),
            Err(NameError::NotMarkdown)
        );
        assert_eq!(
            validate(&dir, ItemKind::File, "v1.2"),
            Err(NameError::NotMarkdown)
        );
    }

    #[test]
    fn a_folder_name_is_taken_as_typed() {
        let dir = scratch("folder-name");
        assert_eq!(
            validate(&dir, ItemKind::Folder, "v1.2 notes").unwrap(),
            dir.join("v1.2 notes")
        );
    }

    #[test]
    fn bad_names_are_rejected() {
        let dir = scratch("bad");
        let err = |text| validate(&dir, ItemKind::Folder, text).unwrap_err();
        assert_eq!(err(""), NameError::Empty);
        assert_eq!(err("   "), NameError::Empty);
        assert_eq!(err("a/b"), NameError::InvalidChars);
        assert_eq!(err("a\\b"), NameError::InvalidChars);
        assert_eq!(err("a:b"), NameError::InvalidChars);
        assert_eq!(err(".."), NameError::InvalidChars);
        assert_eq!(err("name."), NameError::InvalidChars);
        assert_eq!(err("tab\there"), NameError::InvalidChars);
        assert_eq!(err("nul"), NameError::Reserved);
        assert_eq!(err("Com1.txt"), NameError::Reserved);
        assert_eq!(err(&"x".repeat(256)), NameError::TooLong);
    }

    #[test]
    fn a_taken_name_is_rejected_by_its_final_name() {
        let dir = scratch("exists");
        std::fs::write(dir.join("notes.md"), "").unwrap();
        assert_eq!(
            validate(&dir, ItemKind::File, "notes"),
            Err(NameError::Exists("notes.md".into()))
        );
    }

    #[test]
    fn committing_a_file_creates_it_blank_and_never_overwrites() {
        let dir = scratch("commit-file");
        let path = commit(&dir, ItemKind::File, "plan").unwrap();
        assert_eq!(path, dir.join("plan.md"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "");
        std::fs::write(&path, "keep").unwrap();
        assert!(matches!(
            commit(&dir, ItemKind::File, "plan"),
            Err(NameError::Exists(_))
        ));
        assert_eq!(std::fs::read_to_string(path).unwrap(), "keep");
    }

    #[test]
    fn committing_a_folder_makes_it_with_a_blank_file_to_open() {
        let dir = scratch("commit-folder");
        let file = commit(&dir, ItemKind::Folder, "Specs").unwrap();
        assert_eq!(file, dir.join("Specs").join("Untitled.md"));
        assert!(file.is_file());
    }

    #[test]
    fn a_missing_parent_is_an_io_error() {
        let dir = scratch("missing").join("gone");
        assert!(matches!(
            commit(&dir, ItemKind::File, "a"),
            Err(NameError::Io(_))
        ));
        assert!(matches!(
            commit(&dir, ItemKind::Folder, "a"),
            Err(NameError::Io(_))
        ));
    }

    #[test]
    fn a_rename_may_keep_or_recase_its_own_name_but_not_take_anothers() {
        let dir = scratch("rename-validate");
        let a = dir.join("a.md");
        std::fs::write(&a, "x").unwrap();
        std::fs::write(dir.join("b.md"), "y").unwrap();
        let mut item = NewItem::rename(a.clone());
        assert_eq!(item.text, "a.md");
        assert_eq!(validate_item(&item), Ok(a.clone()));
        item.text = "b".into();
        assert_eq!(validate_item(&item), Err(NameError::Exists("b.md".into())));
        item.text = "c".into();
        assert_eq!(validate_item(&item), Ok(dir.join("c.md")));
    }

    #[test]
    fn committing_a_rename_moves_the_file_and_keeps_its_contents() {
        let dir = scratch("rename-commit");
        let a = dir.join("a.md");
        std::fs::write(&a, "body").unwrap();
        let mut item = NewItem::rename(a.clone());
        item.text = "renamed".into();
        let path = commit_item(&item).unwrap();
        assert_eq!(path, dir.join("renamed.md"));
        assert!(!a.exists());
        assert_eq!(std::fs::read_to_string(path).unwrap(), "body");
    }

    #[test]
    fn an_unchanged_rename_touches_nothing() {
        let dir = scratch("rename-same");
        let a = dir.join("a.md");
        std::fs::write(&a, "body").unwrap();
        assert_eq!(commit_item(&NewItem::rename(a.clone())), Ok(a.clone()));
        assert_eq!(std::fs::read_to_string(a).unwrap(), "body");
    }

    #[test]
    fn renaming_selects_only_the_stem() {
        assert_eq!(
            NewItem::rename(PathBuf::from("/d/notes.v2.md")).stem_len(),
            8
        );
    }

    #[test]
    fn only_folders_of_markdown_may_be_changed() {
        let dir = scratch("blocker");
        let docs = dir.join("docs");
        std::fs::create_dir_all(docs.join("deep")).unwrap();
        std::fs::write(docs.join("a.md"), "").unwrap();
        std::fs::write(docs.join("deep").join("B.MARKDOWN"), "").unwrap();
        std::fs::write(docs.join(".DS_Store"), "").unwrap();
        assert_eq!(folder_blocker(&docs), None);
        assert_eq!(folder_blocker(&docs.join("a.md")), None);
        std::fs::write(docs.join("deep").join("pic.png"), "").unwrap();
        let reason = folder_blocker(&docs).unwrap();
        assert!(reason.contains("pic.png"), "{reason}");
    }

    #[test]
    fn renaming_a_folder_validates_as_a_folder_and_moves_it_whole() {
        let dir = scratch("rename-folder");
        let old = dir.join("old");
        std::fs::create_dir(&old).unwrap();
        std::fs::write(old.join("a.md"), "x").unwrap();
        let mut item = NewItem::rename(old.clone());
        assert_eq!((item.kind, item.stem_len()), (ItemKind::Folder, 3));
        item.text = "v1.2".into();
        let path = commit_item(&item).unwrap();
        assert_eq!(path, dir.join("v1.2"));
        assert!(path.join("a.md").is_file() && !old.exists());
    }

    #[test]
    fn a_folder_with_other_files_will_not_rename() {
        let dir = scratch("rename-blocked");
        let old = dir.join("old");
        std::fs::create_dir(&old).unwrap();
        std::fs::write(old.join("x.png"), "x").unwrap();
        let mut item = NewItem::rename(old.clone());
        item.text = "new".into();
        assert!(matches!(commit_item(&item), Err(NameError::Io(_))));
        assert!(old.is_dir());
    }

    #[test]
    fn rebase_moves_a_path_and_everything_under_it() {
        let (from, to) = (Path::new("/r/a"), Path::new("/r/b"));
        assert_eq!(
            rebase(Path::new("/r/a"), from, to),
            Some(PathBuf::from("/r/b"))
        );
        assert_eq!(
            rebase(Path::new("/r/a/x/y.md"), from, to),
            Some(PathBuf::from("/r/b/x/y.md"))
        );
        assert_eq!(rebase(Path::new("/r/ab/y.md"), from, to), None);
    }

    #[test]
    fn a_new_item_hints_md_only_for_an_extensionless_file() {
        let dir = scratch("hint");
        let mut item = NewItem::new(ItemKind::File, dir.to_path_buf());
        assert_eq!(item.added_extension(), Some("md"));
        item.text = "a.md".into();
        assert_eq!(item.added_extension(), None);
        assert_eq!(
            NewItem::new(ItemKind::Folder, dir.to_path_buf()).added_extension(),
            None
        );
    }

    #[test]
    fn renaming_a_text_file_keeps_it_text() {
        let dir = scratch("rename-text");
        let a = dir.join("a.txt");
        std::fs::write(&a, "x").unwrap();
        let mut item = NewItem::rename(a);
        assert_eq!(item.added_extension(), None);
        item.text = "b".into();
        assert_eq!(item.added_extension(), Some("txt"));
        assert_eq!(validate_item(&item), Ok(dir.join("b.txt")));
        item.text = "b.md".into();
        assert_eq!(validate_item(&item), Ok(dir.join("b.md")));
        item.text = "b.csv".into();
        assert_eq!(validate_item(&item), Err(NameError::NotMarkdown));
        // A new file still may not be text.
        assert_eq!(
            validate(&dir, ItemKind::File, "n.txt"),
            Err(NameError::NotMarkdown)
        );
    }

    #[test]
    fn a_folder_holding_text_files_may_be_changed() {
        let dir = scratch("folder-text");
        let docs = dir.join("docs");
        std::fs::create_dir(&docs).unwrap();
        std::fs::write(docs.join("a.md"), "").unwrap();
        std::fs::write(docs.join("b.txt"), "").unwrap();
        assert_eq!(folder_blocker(&docs), None);
        std::fs::write(docs.join("c.png"), "").unwrap();
        assert!(folder_blocker(&docs).is_some());
    }
}
