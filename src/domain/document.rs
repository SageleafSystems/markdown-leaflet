// SPDX-License-Identifier: GPL-3.0-or-later
use crate::domain::frontmatter::{self, Frontmatter};
use crate::domain::outline::{OutlineEntry, extract_outline};
use crate::domain::stats::DocumentStats;
use crate::domain::tasks::{self, TaskId};
use crate::domain::walk::is_text_path;
use iced::widget::markdown;
use std::path::{Path, PathBuf};

/// The file could not be read.
#[derive(Debug, thiserror::Error)]
#[error("couldn't read {}: {source}", path.display())]
pub struct DocumentError {
    pub path: PathBuf,
    pub source: std::io::Error,
}

/// Formats a byte count for the inspector, e.g. `1.5 KB`.
pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

/// How a file is shown: Markdown is parsed and rendered, plain text is shown as typed in a
/// monospace face (issue #79).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentKind {
    Markdown,
    Text,
}

impl DocumentKind {
    pub fn of(path: &Path) -> Self {
        if is_text_path(path) {
            Self::Text
        } else {
            Self::Markdown
        }
    }
}

#[derive(Debug)]
pub struct Document {
    pub path: PathBuf,
    pub kind: DocumentKind,
    pub items: Vec<markdown::Item>,
    pub outline: Vec<OutlineEntry>,
    /// The raw source text, kept around for the share/copy action, byte-for-byte including any
    /// frontmatter block: Select All and Copy reproduce the file as it is on disk.
    pub raw: String,
    /// File size at last load/reload, for the inspector panel.
    pub size: u64,
    /// Word, character, line, link and image counts of the document body, excluding any
    /// frontmatter block (issue #65) — a frontmatter key isn't prose.
    pub stats: DocumentStats,
    /// The leading `---`/`---` YAML block, if the file opens with one and it parses. `None`
    /// both when there is no such block and when there is one but it failed to parse (see
    /// `frontmatter_error`); either way the block still renders as ordinary Markdown, unchanged.
    pub frontmatter: Option<Frontmatter>,
    /// Set when the file opens with a `---`/`---` block that failed to parse as YAML, for the
    /// footer's status note. `None` once the user fixes the file and it reloads cleanly.
    pub frontmatter_error: Option<String>,
    /// Byte length of the frontmatter block (and its delimiters) stripped from `raw` before
    /// parsing, so a byte range into the parsed body maps back into `raw` as `body_offset + i`.
    /// Zero when there is no frontmatter block.
    body_offset: usize,
    /// Whether the file's permissions refuse writes, read at open/reload time so a checkbox
    /// click can be rejected up front rather than failing after the fact.
    pub read_only: bool,
    mtime: Option<std::time::SystemTime>,
}

/// Largest Markdown file the viewer will load. Larger files would stall parsing and layout and
/// can exhaust memory, so they are refused with an explanation.
pub const MAX_DOCUMENT_BYTES: u64 = 64 * 1024 * 1024;

impl Document {
    pub fn open(path: PathBuf) -> Result<Self, DocumentError> {
        if let Ok(meta) = std::fs::metadata(&path)
            && meta.len() > MAX_DOCUMENT_BYTES
        {
            let source = std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "file is {} (the viewer opens files up to {})",
                    human_size(meta.len()),
                    human_size(MAX_DOCUMENT_BYTES)
                ),
            );
            return Err(DocumentError { path, source });
        }
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(source) => return Err(DocumentError { path, source }),
        };
        let kind = DocumentKind::of(&path);
        if kind == DocumentKind::Text {
            return Ok(Self::text(path, text));
        }
        let (frontmatter_block, body) = frontmatter::split(&text);
        let (frontmatter, frontmatter_error) = match frontmatter_block.map(frontmatter::parse) {
            Some(Ok(frontmatter)) => (Some(frontmatter), None),
            Some(Err(error)) => (None, Some(error.to_string())),
            None => (None, None),
        };
        // A block that failed to parse is left in `body` too (`body` is only ever the text
        // after a *successfully split* block — `split` already returns the whole source as
        // `body` when it finds no closing delimiter, and parsing only runs on a block `split`
        // did find), so a malformed block still renders exactly as it does without this
        // feature: as a thematic break and a stray paragraph, not swallowed.
        let body = if frontmatter.is_some() { body } else { &text };
        let body_offset = text.len() - body.len();
        let items: Vec<_> = markdown::parse(body).collect();
        let outline = extract_outline(&items);
        let metadata = std::fs::metadata(&path).ok();
        let mtime = metadata.as_ref().and_then(|m| m.modified().ok());
        let read_only = metadata
            .as_ref()
            .is_some_and(|m| m.permissions().readonly());
        let size = metadata.map_or(0, |m| m.len());
        let stats = DocumentStats::of(body);
        Ok(Self {
            path,
            kind,
            items,
            outline,
            raw: text,
            size,
            stats,
            frontmatter,
            frontmatter_error,
            body_offset,
            read_only,
            mtime,
        })
    }

    /// A plain-text document: no parsing, no outline, no frontmatter; `raw` is what is shown.
    fn text(path: PathBuf, text: String) -> Self {
        let metadata = std::fs::metadata(&path).ok();
        Self {
            kind: DocumentKind::Text,
            items: Vec::new(),
            outline: Vec::new(),
            size: metadata.as_ref().map_or(0, std::fs::Metadata::len),
            stats: DocumentStats::of(&text),
            frontmatter: None,
            frontmatter_error: None,
            body_offset: 0,
            read_only: metadata
                .as_ref()
                .is_some_and(|m| m.permissions().readonly()),
            mtime: metadata.and_then(|m| m.modified().ok()),
            raw: text,
            path,
        }
    }

    /// Flips the checked state of the `id`th checkbox in place: one byte in `raw` and the
    /// matching `Bullet::Task::done` in `items`, so the rendered checkbox and progress
    /// indicators update without a reparse. Returns `false` (nothing changed) for a `TaskId`
    /// that no longer exists, e.g. a reload raced the click.
    pub fn toggle_task(&mut self, id: TaskId) -> bool {
        let offsets = tasks::task_offsets(&self.items);
        let Some(&item_offset) = offsets.get(id.item) else {
            return false;
        };
        let index = item_offset + id.task;
        let body = &self.raw[self.body_offset..];
        let Some(range) = tasks::marker_ranges(body).get(index).cloned() else {
            return false;
        };
        if !tasks::toggle(&mut self.items, index) {
            return false;
        }
        let at = self.body_offset + range.start;
        let flipped = if &self.raw[at..at + 1] == " " {
            "x"
        } else {
            " "
        };
        self.raw.replace_range(at..at + 1, flipped);
        true
    }

    /// The mtime this document was last read at, so a write can refuse to overwrite a file that
    /// changed on disk since (`app::update::tasks`).
    pub fn mtime(&self) -> Option<std::time::SystemTime> {
        self.mtime
    }

    /// Records the mtime a successful write produced, so the live-reload poll does not mistake
    /// our own write for an external change (see `app::update::tasks`).
    pub fn note_written(&mut self, mtime: std::time::SystemTime) {
        self.mtime = Some(mtime);
    }

    /// The name to show for this document: a frontmatter `title` field (issue #65) if the file
    /// has one, otherwise the file name.
    pub fn display_title(&self) -> String {
        self.frontmatter
            .as_ref()
            .and_then(Frontmatter::title)
            .map_or_else(
                || {
                    self.path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned()
                },
                ToString::to_string,
            )
    }

    /// Time since this document's last on-disk modification, for the
    /// inspector panel. `None` if the mtime couldn't be read.
    pub fn modified_ago(&self) -> Option<String> {
        let elapsed = self.mtime?.elapsed().ok()?;
        let secs = elapsed.as_secs();
        Some(if secs < 60 {
            "just now".to_string()
        } else if secs < 3600 {
            format!("{}m ago", secs / 60)
        } else if secs < 86400 {
            format!("{}h ago", secs / 3600)
        } else {
            format!("{}d ago", secs / 86400)
        })
    }

    /// The last on-disk modification in local time, e.g. `Nov 14, 2026 at 7:33 PM`.
    pub fn modified_label(&self) -> Option<String> {
        let local: chrono::DateTime<chrono::Local> = self.mtime?.into();
        Some(local.format("%b %-d, %Y at %-I:%M %p").to_string())
    }

    /// Whether the file's mtime has moved since it was last loaded, for the live-reload poll.
    /// A missing or briefly unreadable file (e.g. mid-write by an editor) counts as unchanged;
    /// the next tick will just try again.
    pub fn is_stale(&self) -> bool {
        std::fs::metadata(&self.path)
            .and_then(|m| m.modified())
            .is_ok_and(|modified| self.mtime != Some(modified))
    }
}

/// A checkbox write could not be committed to disk.
#[derive(Debug, thiserror::Error)]
pub enum SaveError {
    #[error("file is read-only")]
    ReadOnly,
    #[error("file changed externally")]
    ExternalChange,
    #[error("{0}")]
    Io(std::io::Error),
}

/// Writes `raw` to `path`, refusing when the file is read-only or its on-disk mtime no longer
/// matches `expected_mtime` (someone else changed it since it was read, so overwriting it would
/// silently discard that change). Returns the file's new mtime on success, recorded by the
/// caller so the live-reload poll does not mistake this write for an external one.
pub fn save(
    path: &std::path::Path,
    raw: &str,
    expected_mtime: Option<std::time::SystemTime>,
) -> Result<std::time::SystemTime, SaveError> {
    let metadata = std::fs::metadata(path).map_err(SaveError::Io)?;
    if metadata.permissions().readonly() {
        return Err(SaveError::ReadOnly);
    }
    let on_disk = metadata.modified().map_err(SaveError::Io)?;
    if Some(on_disk) != expected_mtime {
        return Err(SaveError::ExternalChange);
    }
    std::fs::write(path, raw).map_err(SaveError::Io)?;
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map_err(SaveError::Io)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/frontmatter")
            .join(name)
    }

    fn tasks_fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/tasks")
            .join(name)
    }

    #[test]
    fn a_simple_block_is_parsed_the_title_overrides_and_stats_exclude_it() {
        let doc = Document::open(fixture("simple.md")).unwrap();
        let fm = doc.frontmatter.as_ref().expect("frontmatter");
        assert!(doc.frontmatter_error.is_none());
        assert_eq!(fm.title(), Some("A Simple Frontmatter Example"));
        assert_eq!(doc.display_title(), "A Simple Frontmatter Example");
        assert_eq!(fm.fields.len(), 4);
        // `raw` keeps the whole file (frontmatter included) for Copy/Select All, but the
        // frontmatter block itself must not count toward the body's stats.
        assert!(doc.raw.contains("title: A Simple Frontmatter Example"));
        assert!(doc.stats.words < doc.raw.split_whitespace().count());
    }

    #[test]
    fn no_frontmatter_block_leaves_the_document_unchanged() {
        let doc = Document::open(fixture("no-frontmatter.md")).unwrap();
        assert!(doc.frontmatter.is_none());
        assert!(doc.frontmatter_error.is_none());
        assert_eq!(doc.display_title(), "no-frontmatter.md");
    }

    #[test]
    fn an_empty_block_has_no_fields_and_is_not_an_error() {
        let doc = Document::open(fixture("empty.md")).unwrap();
        let fm = doc.frontmatter.as_ref().expect("frontmatter");
        assert!(fm.is_empty());
        assert!(doc.frontmatter_error.is_none());
    }

    #[test]
    fn a_malformed_block_falls_back_to_rendering_the_file_unchanged() {
        let doc = Document::open(fixture("malformed.md")).unwrap();
        assert!(doc.frontmatter.is_none());
        assert!(doc.frontmatter_error.is_some());
        // Nothing was stripped: the stats cover the whole file, dashes and all.
        assert_eq!(doc.stats.words, DocumentStats::of(&doc.raw).words);
    }

    #[test]
    fn an_unterminated_block_is_not_frontmatter() {
        let doc = Document::open(fixture("unterminated.md")).unwrap();
        assert!(doc.frontmatter.is_none());
        assert!(doc.frontmatter_error.is_none());
        assert_eq!(doc.stats.words, DocumentStats::of(&doc.raw).words);
    }

    #[test]
    fn nested_maps_and_lists_parse_in_order() {
        let doc = Document::open(fixture("nested.md")).unwrap();
        let fm = doc.frontmatter.as_ref().expect("frontmatter");
        let names: Vec<_> = fm.fields.iter().map(|(key, _)| key.as_str()).collect();
        assert_eq!(names, ["title", "tags", "author", "contributors"]);
        let crate::domain::frontmatter::FrontmatterValue::Map(author) = &fm.fields[2].1 else {
            panic!("author should be a mapping");
        };
        assert_eq!(author[0].0, "name");
        let crate::domain::frontmatter::FrontmatterValue::List(contributors) = &fm.fields[3].1
        else {
            panic!("contributors should be a list");
        };
        assert_eq!(contributors.len(), 2);
    }

    #[test]
    fn a_toml_style_block_is_not_recognized() {
        let doc = Document::open(fixture("toml-style.md")).unwrap();
        assert!(doc.frontmatter.is_none());
        assert!(doc.frontmatter_error.is_none());
    }

    #[test]
    fn toggle_task_flips_one_byte_and_leaves_everything_else_untouched() {
        let mut doc = Document::open(tasks_fixture("nested.md")).unwrap();
        let before = doc.raw.clone();
        assert!(
            doc.toggle_task(TaskId { item: 1, task: 1 }),
            "Design interface"
        );
        assert_ne!(doc.raw, before);
        assert_eq!(doc.raw.len(), before.len());
        let diff: Vec<_> = before
            .bytes()
            .zip(doc.raw.bytes())
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .collect();
        assert_eq!(diff.len(), 1, "only the targeted marker byte changed");
        assert!(doc.raw.contains("- [x] Design interface"));
        assert!(doc.raw.contains("- [x] Define requirements"), "untouched");
        assert!(doc.raw.contains("- [ ] Ship release"), "untouched");
    }

    #[test]
    fn toggle_task_updates_the_rendered_item_tree_too() {
        let mut doc = Document::open(tasks_fixture("nested.md")).unwrap();
        assert!(doc.toggle_task(TaskId { item: 1, task: 0 }), "first task");
        let iced::widget::markdown::Item::List { bullets, .. } = &doc.items[1] else {
            panic!("expected the unordered list");
        };
        let iced::widget::markdown::Bullet::Task { done, .. } = &bullets[0] else {
            panic!("expected a task");
        };
        assert!(!done, "was checked, now unchecked");
    }

    #[test]
    fn toggle_task_reaches_an_ordered_list() {
        let mut doc = Document::open(tasks_fixture("nested.md")).unwrap();
        assert!(
            doc.toggle_task(TaskId { item: 3, task: 0 }),
            "ordered list, First"
        );
        assert!(doc.raw.contains("1. [x] First"));
        assert!(doc.raw.contains("2. [x] Second"), "untouched");
    }

    #[test]
    fn toggle_task_with_an_unknown_id_changes_nothing() {
        let mut doc = Document::open(tasks_fixture("nested.md")).unwrap();
        let before = doc.raw.clone();
        assert!(!doc.toggle_task(TaskId { item: 1, task: 99 }));
        assert_eq!(doc.raw, before);
        assert!(!doc.toggle_task(TaskId { item: 99, task: 0 }));
        assert_eq!(doc.raw, before);
    }

    #[test]
    fn save_writes_the_file_and_returns_its_new_mtime() {
        let dir = crate::testutil::scratch_dir("save-ok");
        let path = dir.join("a.md");
        std::fs::write(&path, "- [ ] a\n").unwrap();
        let expected = std::fs::metadata(&path).unwrap().modified().unwrap();
        let new_mtime = save(&path, "- [x] a\n", Some(expected)).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "- [x] a\n");
        assert_eq!(
            std::fs::metadata(&path).unwrap().modified().unwrap(),
            new_mtime
        );
    }

    #[test]
    fn save_refuses_a_read_only_file() {
        let dir = crate::testutil::scratch_dir("save-readonly");
        let path = dir.join("a.md");
        std::fs::write(&path, "- [ ] a\n").unwrap();
        let mtime = std::fs::metadata(&path).unwrap().modified().unwrap();
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&path, perms).unwrap();
        let err = save(&path, "- [x] a\n", Some(mtime)).unwrap_err();
        assert!(matches!(err, SaveError::ReadOnly));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "- [ ] a\n");
    }

    #[test]
    fn save_refuses_a_file_that_changed_on_disk_since_it_was_read() {
        let dir = crate::testutil::scratch_dir("save-external");
        let path = dir.join("a.md");
        std::fs::write(&path, "- [ ] a\n").unwrap();
        let stale_mtime = std::fs::metadata(&path).unwrap().modified().unwrap();
        // A change on disk the caller never read.
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&path, "- [ ] a\n- [ ] b\n").unwrap();
        let err = save(&path, "- [x] a\n", Some(stale_mtime)).unwrap_err();
        assert!(matches!(err, SaveError::ExternalChange));
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "- [ ] a\n- [ ] b\n",
            "the external change was not overwritten"
        );
    }

    #[test]
    fn human_size_scales_units() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(1023), "1023 B");
        assert_eq!(human_size(1536), "1.5 KB");
        assert_eq!(human_size(5 * 1024 * 1024), "5.0 MB");
    }

    #[test]
    fn a_file_over_the_cap_is_refused_with_its_size() {
        let dir = crate::testutil::scratch_dir("big");
        let path = dir.join("big.md");
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(MAX_DOCUMENT_BYTES + 1).unwrap();
        let Err(err) = Document::open(path.clone()) else {
            panic!("an oversized file must be refused");
        };
        assert_eq!(err.source.kind(), std::io::ErrorKind::InvalidData);
        assert!(
            err.source
                .to_string()
                .contains("the viewer opens files up to")
        );
    }

    #[test]
    fn a_text_file_is_shown_as_typed_with_no_outline_or_frontmatter() {
        let dir = crate::testutil::scratch_dir("doc-text");
        let path = dir.join("notes.txt");
        std::fs::write(&path, "---\ntitle: x\n---\n# not a heading\n* not a list\n").unwrap();
        let doc = Document::open(path).unwrap();
        assert_eq!(doc.kind, DocumentKind::Text);
        assert!(doc.items.is_empty() && doc.outline.is_empty());
        assert!(doc.frontmatter.is_none());
        assert!(doc.raw.starts_with("---\ntitle: x"));
        assert_eq!(doc.display_title(), "notes.txt");
    }

    #[test]
    fn a_markdown_file_stays_markdown() {
        assert_eq!(
            Document::open(fixture("simple.md")).unwrap().kind,
            DocumentKind::Markdown
        );
    }

    #[test]
    fn the_text_fixtures_open_as_literal_text() {
        let text = |name: &str| {
            Document::open(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures/text")
                    .join(name),
            )
            .unwrap()
        };
        let plain = text("plain.txt");
        assert_eq!(plain.kind, DocumentKind::Text);
        assert!(plain.raw.contains("# Not a heading") && plain.raw.contains("\ta leading tab"));
        assert!(plain.items.is_empty() && plain.frontmatter.is_none());
        assert!(text("unicode.txt").raw.contains("日本語"));
        assert!(text("long-lines.txt").stats.lines > 40);
        let empty = text("empty.txt");
        assert!(empty.raw.is_empty() && empty.outline.is_empty());
        assert_eq!(text("SHOUTING.TXT").kind, DocumentKind::Text);
    }
}
