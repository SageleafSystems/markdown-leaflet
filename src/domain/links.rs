// SPDX-License-Identifier: GPL-3.0-or-later
//! Decides whether a clicked link points at a Markdown file inside the open
//! folder, so the app can show it itself instead of handing it to the OS.

use crate::domain::walk::{is_markdown_path, is_text_path};
use std::path::{Component, Path, PathBuf};

/// The path a link names when it is local: relative to `doc_dir`, absolute, or `file://`, with
/// any `#fragment` or `?query` dropped and `.`/`..` resolved. Web, `mailto:` and other scheme
/// links, anchors within the same page, and undecodable paths return `None`.
fn local_path(uri: &str, doc_dir: &Path) -> Option<PathBuf> {
    let uri = uri.trim();
    let local = match uri.strip_prefix("file://") {
        Some(rest) => rest.strip_prefix("localhost").unwrap_or(rest),
        None if has_scheme(uri) => return None,
        None => uri,
    };
    let local = local.split(['#', '?']).next().unwrap_or_default();
    if local.is_empty() {
        return None;
    }
    let decoded = percent_decode(local)?;
    Some(normalize(&doc_dir.join(decoded)))
}

fn is_markdown(path: &Path) -> bool {
    is_markdown_path(path)
}

/// The Markdown file `uri` names, if it is a local path (relative to
/// `doc_dir`, absolute, or `file://`) to an existing Markdown file under
/// `root` (or, with `include_text`, a plain-text file, issue #79). Any `#fragment` or `?query` is ignored. Web, `mailto:` and other
/// scheme links, anchors within the same page, and files outside `root`
/// return `None`.
pub fn local_markdown_target(
    uri: &str,
    doc_dir: &Path,
    root: &Path,
    include_text: bool,
) -> Option<PathBuf> {
    let path = local_path(uri, doc_dir)?;
    let shown = is_markdown(&path) || (include_text && is_text_path(&path));
    (shown && is_inside(&path, root) && path.is_file()).then_some(path)
}

/// The existing Markdown file `uri` names when it lies **outside** `root`, as its real path
/// (symlinks resolved, so a link inside the folder that leads out is reported where the file
/// actually is). Everything `local_markdown_target` accepts, and every non-Markdown or
/// non-local link, returns `None`.
pub fn external_markdown_target(uri: &str, doc_dir: &Path, root: &Path) -> Option<PathBuf> {
    let path = local_path(uri, doc_dir)?;
    if !is_markdown(&path) || !path.is_file() || is_inside(&path, root) {
        return None;
    }
    std::fs::canonicalize(path).ok()
}

/// Whether `path` really lies inside `root` once symlinks are resolved. The lexical `normalize`
/// check alone lets a symlink inside the folder reach files outside it. Both must exist.
pub(crate) fn is_inside(path: &Path, root: &Path) -> bool {
    match (std::fs::canonicalize(path), std::fs::canonicalize(root)) {
        (Ok(path), Ok(root)) => path.starts_with(root),
        _ => false,
    }
}

/// Link schemes the app hands to the operating system.
const OPENABLE_SCHEMES: &[&str] = &["http", "https", "mailto"];

/// Document types a local link may open with the default application. Everything else (apps,
/// scripts, installers, shortcuts, extensionless files) is refused, so a document cannot launch
/// a program with one click.
const OPENABLE_EXTENSIONS: &[&str] = &[
    "pdf", "png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "txt", "csv", "json", "toml", "yaml",
    "yml", "rtf", "html", "htm", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "ods", "odp",
    "mp3", "wav", "mp4", "mov", "md", "markdown", "mdown", "mdx",
];

/// Whether a clicked link that is not a Markdown page may be passed to the operating system:
/// a web or `mailto:` link, or a local document of a safe type inside `root`.
pub fn may_open_externally(uri: &str, doc_dir: &Path, root: &Path) -> bool {
    let uri = uri.trim();
    if let Some((scheme, _)) = uri.split_once(':')
        && has_scheme(uri)
        && !scheme.eq_ignore_ascii_case("file")
    {
        return OPENABLE_SCHEMES
            .iter()
            .any(|s| s.eq_ignore_ascii_case(scheme));
    }
    let local = match uri.strip_prefix("file://") {
        Some(rest) => rest.strip_prefix("localhost").unwrap_or(rest),
        None => uri,
    };
    let local = local.split(['#', '?']).next().unwrap_or_default();
    let Some(decoded) = percent_decode(local).filter(|d| !d.is_empty()) else {
        return false;
    };
    let path = normalize(&doc_dir.join(decoded));
    let safe_type = path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            OPENABLE_EXTENSIONS
                .iter()
                .any(|k| k.eq_ignore_ascii_case(ext))
        });
    safe_type && path.is_file() && is_inside(&path, root)
}

/// A scheme such as `https:` or `mailto:`. A single letter is a Windows
/// drive (`C:\notes.md`), not a scheme.
pub(crate) fn has_scheme(uri: &str) -> bool {
    let Some((scheme, _)) = uri.split_once(':') else {
        return false;
    };
    scheme.len() > 1
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

pub(crate) fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = text.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Resolves `.` and `..` without touching the filesystem.
pub(crate) fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tree() -> tempdir::Dir {
        tempdir::Dir::new()
    }

    /// A throwaway folder under the system temp directory, removed on drop.
    mod tempdir {
        use std::path::Path;
        pub struct Dir(crate::testutil::ScratchDir);
        impl Dir {
            pub fn new() -> Self {
                Self(crate::testutil::scratch_dir("links"))
            }
            pub fn path(&self) -> &Path {
                self.0.path()
            }
        }
    }

    #[test]
    fn resolves_relative_links_to_markdown_in_the_folder() {
        let dir = tree();
        let root = dir.path();
        fs::create_dir_all(root.join("docs/deep")).unwrap();
        fs::write(root.join("docs/a.md"), "").unwrap();
        fs::write(root.join("docs/deep/b.MD"), "").unwrap();
        fs::write(root.join("docs/my notes.md"), "").unwrap();
        fs::write(root.join("top.md"), "").unwrap();
        let docs = root.join("docs");

        let target = |uri| local_markdown_target(uri, &docs, root, false);
        assert_eq!(target("a.md"), Some(docs.join("a.md")));
        assert_eq!(target("./a.md#intro"), Some(docs.join("a.md")));
        assert_eq!(target("deep/b.MD?x=1"), Some(docs.join("deep/b.MD")));
        assert_eq!(target("../top.md"), Some(root.join("top.md")));
        assert_eq!(target("my%20notes.md"), Some(docs.join("my notes.md")));
    }

    #[test]
    fn leaves_everything_else_to_the_operating_system() {
        let dir = tree();
        let root = dir.path();
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::write(root.join("sub/a.md"), "").unwrap();
        fs::write(root.join("sub/pic.png"), "").unwrap();
        let sub = root.join("sub");
        let outside = root.parent().unwrap().join("outside.md");

        let target = |uri| local_markdown_target(uri, &sub, &sub, false);
        assert_eq!(target("https://example.com/a.md"), None);
        assert_eq!(target("mailto:me@example.com"), None);
        assert_eq!(target("#section"), None);
        assert_eq!(target("pic.png"), None);
        assert_eq!(target("missing.md"), None);
        assert_eq!(target("../sub/../outside.md"), None);
        let outside_uri = format!("file://{}", outside.display());
        assert_eq!(target(&outside_uri), None);
        assert_eq!(target("bad%zz.md"), None);
        let inside_uri = format!("file://{}", sub.join("a.md").display());
        assert_eq!(target(&inside_uri), Some(sub.join("a.md")));
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_cannot_lead_out_of_the_root() {
        let dir = tree();
        let root = dir.path().join("root");
        let outside = dir.path().join("outside");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("secret.md"), "# s").unwrap();
        std::os::unix::fs::symlink(&outside, root.join("link")).unwrap();
        assert_eq!(
            local_markdown_target("link/secret.md", &root, &root, false),
            None
        );
        assert!(!is_inside(&root.join("link/secret.md"), &root));
        assert!(is_inside(&root, &root));
    }

    #[test]
    fn only_web_mail_and_safe_local_documents_open_externally() {
        let dir = tree();
        let root = dir.path();
        fs::write(root.join("a.pdf"), "x").unwrap();
        fs::write(root.join("run.command"), "x").unwrap();
        fs::write(root.join("noext"), "x").unwrap();
        let ok = |uri: &str| may_open_externally(uri, root, root);
        assert!(ok("https://example.com"));
        assert!(ok("HTTP://example.com"));
        assert!(ok("mailto:a@b.c"));
        assert!(ok("a.pdf"));
        assert!(ok("a.pdf#page=2"));
        assert!(!ok("run.command"));
        assert!(!ok("noext"));
        assert!(!ok("missing.pdf"));
        assert!(!ok("../outside.pdf"));
        assert!(!ok("ssh://host"));
        assert!(!ok("smb://host/share"));
        assert!(!ok("x-apple.systempreferences:"));
        assert!(!ok("javascript:alert(1)"));
        assert!(!ok(&format!(
            "file://{}",
            root.join("run.command").display()
        )));
        assert!(!ok("%zz"));
    }

    #[test]
    fn a_markdown_file_outside_the_root_is_external_and_one_inside_is_not() {
        let dir = tree();
        let root = dir.path().join("root");
        let outside = dir.path().join("elsewhere");
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(root.join("in.md"), "# in").unwrap();
        fs::write(outside.join("out.md"), "# out").unwrap();
        fs::write(outside.join("notes.txt"), "x").unwrap();
        let doc_dir = root.join("sub");
        let real = fs::canonicalize(outside.join("out.md")).unwrap();
        let external = |uri: &str| external_markdown_target(uri, &doc_dir, &root);
        assert_eq!(external("../../elsewhere/out.md#intro"), Some(real.clone()));
        assert_eq!(
            external(&format!("file://{}", outside.join("out.md").display())),
            Some(real)
        );
        assert_eq!(external("../in.md"), None, "inside the root is local");
        assert_eq!(external("../../elsewhere/notes.txt"), None, "not Markdown");
        assert_eq!(external("../../elsewhere/missing.md"), None, "must exist");
        assert_eq!(external("https://example.com/a.md"), None);
        assert_eq!(external("#heading"), None);
        assert_eq!(external("mailto:a@b.c"), None);
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_out_of_the_root_is_external_at_its_real_location() {
        let dir = tree();
        let root = dir.path().join("root");
        let outside = dir.path().join("outside");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("secret.md"), "# s").unwrap();
        std::os::unix::fs::symlink(&outside, root.join("link")).unwrap();
        let real = fs::canonicalize(outside.join("secret.md")).unwrap();
        assert_eq!(
            external_markdown_target("link/secret.md", &root, &root),
            Some(real)
        );
    }

    /// Property tests: link handling takes arbitrary text from untrusted documents.
    mod properties {
        use super::super::*;
        use proptest::prelude::*;
        use std::path::Component;

        fn percent_encode_all(text: &str) -> String {
            use std::fmt::Write;
            text.bytes().fold(String::new(), |mut out, byte| {
                let _ = write!(out, "%{byte:02X}");
                out
            })
        }

        proptest! {
            #[test]
            fn percent_decode_never_panics_and_inverts_full_encoding(text in "\\PC*") {
                let _ = percent_decode(&text);
                prop_assert_eq!(percent_decode(&percent_encode_all(&text)), Some(text));
            }

            #[test]
            fn percent_decode_rejects_a_truncated_or_non_hex_escape(
                prefix in "[a-z]{0,5}", bad in "(%|%[0-9a-fA-F]|%[g-z]{2})"
            ) {
                prop_assert_eq!(percent_decode(&format!("{prefix}{bad}")), None);
            }

            #[test]
            fn normalize_is_idempotent_and_leaves_no_dot_components(
                parts in proptest::collection::vec(
                    prop_oneof![Just("a"), Just("b"), Just(".."), Just("."), Just("")], 0..12)
            ) {
                let path = PathBuf::from(format!("/{}", parts.join("/")));
                let once = normalize(&path);
                prop_assert_eq!(normalize(&once), once.clone());
                prop_assert!(once.components().all(|c| !matches!(c, Component::CurDir | Component::ParentDir)));
            }

            #[test]
            fn scheme_detection_and_the_open_policy_never_panic(uri in "\\PC{0,60}") {
                let root = Path::new("/nonexistent-root");
                let _ = has_scheme(&uri);
                let _ = local_markdown_target(&uri, root, root, false);
                let _ = external_markdown_target(&uri, root, root);
                let _ = may_open_externally(&uri, root, root);
            }

            #[test]
            fn only_web_and_mail_schemes_ever_reach_the_os(
                scheme in "[a-z][a-z0-9+.-]{1,10}", rest in "[ -~]{0,30}"
            ) {
                prop_assume!(!["http", "https", "mailto", "file"].contains(&scheme.as_str()));
                let root = Path::new("/nonexistent-root");
                let uri = format!("{scheme}:{rest}");
                prop_assert!(!may_open_externally(&uri, root, root));
            }
        }

        #[test]
        fn a_link_is_never_both_local_and_external() {
            let dir = super::tree();
            let root = dir.path().join("root");
            let outside = dir.path().join("elsewhere");
            std::fs::create_dir_all(root.join("sub")).unwrap();
            std::fs::create_dir_all(&outside).unwrap();
            std::fs::write(root.join("in.md"), "x").unwrap();
            std::fs::write(outside.join("out.md"), "x").unwrap();
            let doc_dir = root.join("sub");
            let pieces = [
                "../",
                "../../",
                "sub/",
                "elsewhere/",
                "root/",
                "in.md",
                "out.md",
                "#x",
                "?q",
                "%2E%2E/",
                "./",
            ];
            let strategy =
                proptest::collection::vec(proptest::sample::select(pieces.to_vec()), 1..7)
                    .prop_map(|parts| parts.concat());
            let mut runner =
                proptest::test_runner::TestRunner::new(ProptestConfig::with_cases(300));
            runner
                .run(&strategy, |uri| {
                    let local = local_markdown_target(&uri, &doc_dir, &root, false);
                    let external = external_markdown_target(&uri, &doc_dir, &root);
                    prop_assert!(
                        local.is_none() || external.is_none(),
                        "{uri}: {local:?} {external:?}"
                    );
                    if let Some(path) = local {
                        prop_assert!(is_inside(&path, &root));
                    }
                    if let Some(path) = external {
                        prop_assert!(!is_inside(&path, &root) && path.is_file());
                    }
                    Ok(())
                })
                .unwrap();
        }
    }
}
