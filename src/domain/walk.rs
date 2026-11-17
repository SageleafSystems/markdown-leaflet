// SPDX-License-Identifier: GPL-3.0-or-later
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Directory names skipped by default when walking a folder — the usual
/// dependency/cache/build junk nobody wants scanned for Markdown. A user
/// can un-skip any of these via `include` in the config file.
const DEFAULT_IGNORED_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".hg",
    ".svn",
    "target",
    "dist",
    "build",
    "out",
    ".next",
    ".nuxt",
    ".cache",
    "__pycache__",
    ".venv",
    "venv",
    "vendor",
    ".idea",
    ".vscode",
    "coverage",
    ".pytest_cache",
    ".mypy_cache",
    ".tox",
    ".gradle",
    ".terraform",
];

/// Which directories a folder scan skips.
#[derive(Debug, Clone, Default)]
pub struct ScanRules {
    /// Directory names to NOT ignore, overriding the default ignore list.
    include: Vec<String>,
    /// Whether plain-text (`.txt`) files are listed alongside Markdown (issue #79).
    include_text: bool,
}

impl ScanRules {
    pub fn new(include: Vec<String>) -> Self {
        Self {
            include,
            include_text: false,
        }
    }

    /// These rules, listing `.txt` files too when `show` is true.
    #[must_use]
    pub fn with_text(mut self, show: bool) -> Self {
        self.include_text = show;
        self
    }

    /// Whether a scan lists `path`: a Markdown file always, a plain-text file only when the
    /// rules include text.
    pub fn lists(&self, path: &Path) -> bool {
        is_markdown_path(path) || (self.include_text && is_text_path(path))
    }

    pub fn ignores(&self, dir_name: &str) -> bool {
        DEFAULT_IGNORED_DIRS.contains(&dir_name) && !self.include.iter().any(|i| i == dir_name)
    }
}

pub const MARKDOWN_EXTENSIONS: &[&str] = &["md", "markdown", "mdown", "mdx"];

/// Plain-text extensions listed when the "Show Text Files" setting is on (issue #79).
pub const TEXT_EXTENSIONS: &[&str] = &["txt"];

fn has_extension(path: &Path, known: &[&str]) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| known.iter().any(|k| k.eq_ignore_ascii_case(ext)))
}

/// Whether `path` has a Markdown extension (case-insensitive).
pub fn is_markdown_path(path: &Path) -> bool {
    has_extension(path, MARKDOWN_EXTENSIONS)
}

/// Whether `path` has a plain-text extension (case-insensitive).
pub fn is_text_path(path: &Path) -> bool {
    has_extension(path, TEXT_EXTENSIONS)
}

/// Deepest folder level the scan descends to. Real note trees are far shallower; the cap keeps a
/// pathological or hostile tree from exhausting the stack.
const MAX_DEPTH: usize = 64;

/// How long to wait before the next folder rescan, given how long the last one took. A scan of a
/// small tree is instant and keeps `base`; a huge tree backs off to ten times its scan time (so
/// scanning uses about a tenth of one core at most), never less than `base` or more than a minute.
pub fn poll_interval(
    last_scan: std::time::Duration,
    base: std::time::Duration,
) -> std::time::Duration {
    const CEILING: std::time::Duration = std::time::Duration::from_secs(60);
    last_scan.saturating_mul(10).clamp(base, CEILING.max(base))
}

/// The result of scanning a folder.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scan {
    /// Markdown files found, sorted.
    pub files: Vec<PathBuf>,
    /// Folders that exist but could not be listed (for example, no permission). Their contents
    /// are missing from `files`.
    pub unreadable: Vec<PathBuf>,
}

pub fn collect_markdown_files(root: &Path, rules: &ScanRules) -> Vec<PathBuf> {
    scan_markdown_files(root, rules).files
}

/// Like [`collect_markdown_files`], but also reports the folders it could not read, so the app
/// can say that part of the tree is missing instead of silently showing less.
pub fn scan_markdown_files(root: &Path, rules: &ScanRules) -> Scan {
    let mut scan = Scan::default();
    let mut visited = HashSet::new();
    collect_markdown_files_into(root, rules, 0, &mut visited, &mut scan);
    scan.files.sort();
    scan.unreadable.sort();
    scan
}

/// Walks `dir`, following symlinked folders but never entering the same real folder twice, so a
/// symlink loop (`a/b -> ..`) ends instead of recursing until the stack overflows.
fn collect_markdown_files_into(
    dir: &Path,
    rules: &ScanRules,
    depth: usize,
    visited: &mut HashSet<PathBuf>,
    scan: &mut Scan,
) {
    if depth > MAX_DEPTH {
        return;
    }
    let Ok(real) = std::fs::canonicalize(dir) else {
        return;
    };
    if !visited.insert(real) {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        scan.unreadable.push(dir.to_path_buf());
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if rules.ignores(name) {
                continue;
            }
            collect_markdown_files_into(&path, rules, depth + 1, visited, scan);
        } else if rules.lists(&path) {
            scan.files.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a throwaway fixture with one markdown file at the root, one
    /// nested under a non-ignored dir, and one under `node_modules/` —
    /// enough to exercise the default ignore list either way. Each test
    /// gets its own directory (keyed by `label`) so parallel runs don't
    /// collide.
    fn fixture(label: &str) -> crate::testutil::ScratchDir {
        let root = crate::testutil::scratch_dir(&format!("walk-{label}"));
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::create_dir_all(root.join("node_modules")).unwrap();
        std::fs::write(root.join("root.md"), "# root").unwrap();
        std::fs::write(root.join("docs/guide.md"), "# guide").unwrap();
        std::fs::write(root.join("node_modules/readme.md"), "# readme").unwrap();
        root
    }

    #[test]
    fn ignores_node_modules_by_default() {
        let root = fixture("default-ignore");
        let rules = ScanRules::default();
        let files = collect_markdown_files(&root, &rules);
        let names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(names.contains(&"guide.md".to_string()));
        assert!(names.contains(&"root.md".to_string()));
        assert!(!names.contains(&"readme.md".to_string()));
    }

    #[test]
    fn config_include_overrides_default_ignore() {
        let root = fixture("include-override");
        let rules = ScanRules::new(vec!["node_modules".to_string()]);
        let files = collect_markdown_files(&root, &rules);
        let names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(names.contains(&"readme.md".to_string()));
    }

    #[test]
    fn the_rescan_interval_backs_off_for_slow_trees_and_never_below_the_base() {
        use std::time::Duration;
        let base = Duration::from_secs(3);
        let ms = Duration::from_millis;
        assert_eq!(poll_interval(Duration::ZERO, base), base);
        assert_eq!(poll_interval(ms(20), base), base);
        assert_eq!(poll_interval(ms(300), base), base);
        assert_eq!(
            poll_interval(Duration::from_secs(1), base),
            Duration::from_secs(10)
        );
        assert_eq!(
            poll_interval(Duration::from_secs(30), base),
            Duration::from_secs(60)
        );
        assert_eq!(poll_interval(Duration::MAX, base), Duration::from_secs(60));
        // A base above the ceiling is respected.
        let long = Duration::from_secs(120);
        assert_eq!(poll_interval(Duration::from_secs(1), long), long);
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_loop_ends_instead_of_recursing_forever() {
        let root = fixture("symlink-loop");
        std::os::unix::fs::symlink("..", root.join("docs/up")).unwrap();
        let files = collect_markdown_files(&root, &ScanRules::default());
        let names: Vec<_> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(names.contains(&"root.md".to_string()));
        assert!(names.contains(&"guide.md".to_string()));
        assert!(files.len() < 10, "loop was followed: {files:?}");
    }

    #[test]
    fn a_very_deep_tree_is_cut_off_at_the_depth_cap() {
        let root = fixture("deep");
        let mut dir = root.to_path_buf();
        for _ in 0..(MAX_DEPTH + 5) {
            dir = dir.join("d");
        }
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("deep.md"), "# deep").unwrap();
        let files = collect_markdown_files(&root, &ScanRules::default());
        assert!(files.iter().all(|p| p.file_name().unwrap() != "deep.md"));
    }

    /// Locks `dir` so it cannot be listed. `None` if the permission change has no effect (for
    /// example when running as root), in which case the caller skips its assertions.
    #[cfg(unix)]
    fn lock(dir: &Path) -> Option<()> {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o000)).unwrap();
        std::fs::read_dir(dir).is_err().then_some(())
    }

    #[cfg(unix)]
    #[test]
    fn an_unreadable_folder_is_reported_instead_of_silently_skipped() {
        let root = fixture("unreadable");
        let locked = root.join("docs");
        if lock(&locked).is_none() {
            return;
        }
        let scan = scan_markdown_files(&root, &ScanRules::default());
        assert_eq!(scan.unreadable, vec![locked]);
        let names: Vec<_> = scan
            .files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            ["root.md"],
            "the locked folder's guide.md is missing"
        );
        // The plain listing is unchanged for callers that do not report it.
        assert_eq!(
            collect_markdown_files(&root, &ScanRules::default()),
            scan.files
        );
    }

    #[test]
    fn a_fully_readable_tree_reports_nothing_unreadable() {
        let root = fixture("readable");
        let scan = scan_markdown_files(&root, &ScanRules::default());
        assert!(scan.unreadable.is_empty() && scan.files.len() == 2);
    }

    #[test]
    fn text_files_are_listed_only_when_the_rules_include_them() {
        let root = fixture("text-files");
        std::fs::write(root.join("notes.txt"), "plain").unwrap();
        std::fs::write(root.join("docs/LOUD.TXT"), "plain").unwrap();
        std::fs::write(root.join("data.csv"), "a,b").unwrap();
        let names = |rules: &ScanRules| -> Vec<String> {
            collect_markdown_files(&root, rules)
                .iter()
                .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
                .collect()
        };
        let off = names(&ScanRules::default());
        assert!(off.contains(&"root.md".to_string()));
        assert!(!off.contains(&"notes.txt".to_string()));
        let on = names(&ScanRules::default().with_text(true));
        assert!(on.contains(&"notes.txt".to_string()));
        assert!(on.contains(&"LOUD.TXT".to_string()));
        assert!(!on.contains(&"data.csv".to_string()));
    }

    #[test]
    fn the_text_fixture_folder_lists_by_setting() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text");
        let names = |rules: &ScanRules| -> Vec<String> {
            collect_markdown_files(&root, rules)
                .iter()
                .map(|p| {
                    p.strip_prefix(&root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned()
                })
                .collect()
        };
        assert_eq!(names(&ScanRules::default()), ["notes.md"]);
        let on = names(&ScanRules::default().with_text(true));
        for expected in [
            "SHOUTING.TXT",
            "empty.txt",
            "only-text/readme.txt",
            "plain.txt",
        ] {
            assert!(
                on.iter().any(|n| n == expected),
                "{expected} missing: {on:?}"
            );
        }
    }
}
