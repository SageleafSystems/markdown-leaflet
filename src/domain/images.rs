// SPDX-License-Identifier: GPL-3.0-or-later
//! Local images named by a document: which `![alt](url)` targets can be shown, where they live,
//! and decoding them within safe limits. Nothing here touches `iced` widgets.

use crate::domain::links::{has_scheme, is_inside, normalize, percent_decode};
use iced::widget::markdown;
use std::path::{Path, PathBuf};

/// Extensions the preview can draw. SVG is rasterized by the renderer; the rest are decoded here.
const RASTER_EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "gif", "webp", "bmp"];
const SVG_EXTENSION: &str = "svg";

/// Larger files are refused before they are read.
pub const MAX_FILE_BYTES: u64 = 32 * 1024 * 1024;
/// SVG source larger than this is refused. Vector art is small; a big file is an attack or a
/// mistake, and the renderer parses it on the interface thread.
const MAX_SVG_BYTES: u64 = 4 * 1024 * 1024;
/// Deepest element nesting accepted in an SVG.
const MAX_SVG_DEPTH: usize = 128;
/// A decoded side longer than this is refused, which bounds the pixel buffer.
const MAX_SIDE: u32 = 12_000;
/// Decoder allocation budget, in bytes.
const MAX_ALLOC: u64 = 384 * 1024 * 1024;

/// An image ready to hand to the renderer.
#[derive(Debug, Clone)]
pub enum Decoded {
    /// Straight RGBA, 8 bits per channel. Animated formats keep their first frame.
    Raster {
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    },
    /// Raw SVG source.
    Svg(Vec<u8>),
}

#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    #[error("file is {} (limit {})", crate::domain::document::human_size(*.0), crate::domain::document::human_size(MAX_FILE_BYTES))]
    TooLarge(u64),
    #[error("couldn't read the file: {0}")]
    Io(#[from] std::io::Error),
    #[error("the SVG was not drawn: {0}")]
    UnsafeSvg(&'static str),
    #[error("couldn't decode the image: {0}")]
    Decode(#[from] image::ImageError),
}

/// The file `uri` names, if it is a supported image that exists inside `root`. Relative paths
/// resolve against `doc_dir`; absolute paths and `file://` are accepted. Web and other scheme
/// URLs, `data:` URIs, and files outside `root` return `None`. A `#fragment` or `?query` is
/// ignored.
pub fn resolve(uri: &str, doc_dir: &Path, root: &Path) -> Option<PathBuf> {
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
    let path = normalize(&doc_dir.join(percent_decode(local)?));
    (is_supported(&path) && is_inside(&path, root) && path.is_file()).then_some(path)
}

/// Whether `path` has an extension the preview can draw.
pub fn is_supported(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
        return false;
    };
    ext.eq_ignore_ascii_case(SVG_EXTENSION)
        || RASTER_EXTENSIONS
            .iter()
            .any(|known| known.eq_ignore_ascii_case(ext))
}

/// Every image URL in `items`, in document order, looking inside lists and quotes. Images inside
/// table cells are not reachable from outside `iced`'s markdown module and are not listed.
pub fn image_urls(items: &[markdown::Item]) -> Vec<&str> {
    let mut urls = Vec::new();
    collect(items, &mut urls);
    urls
}

fn collect<'a>(items: &'a [markdown::Item], urls: &mut Vec<&'a str>) {
    for item in items {
        match item {
            markdown::Item::Image { url, .. } => urls.push(url),
            markdown::Item::Quote(inner) => collect(inner, urls),
            markdown::Item::List { bullets, .. } => {
                for bullet in bullets {
                    match bullet {
                        markdown::Bullet::Point { items }
                        | markdown::Bullet::Task { items, .. } => {
                            collect(items, urls);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Refuses SVG source the preview should not hand to the renderer: entity declarations (which can
/// expand into enormous documents), references to anything but the same file or an inline
/// `data:` URI (the renderer would otherwise read local files the SVG names), `@import`, and
/// absurd nesting. Plain drawings pass. The reason is shown to the user.
fn check_svg(bytes: &[u8]) -> Result<(), &'static str> {
    let text = String::from_utf8_lossy(bytes).to_ascii_lowercase();
    if text.contains("<!entity") {
        return Err("it declares entities");
    }
    if text.contains("@import") {
        return Err("it imports a stylesheet");
    }
    let mut rest = text.as_str();
    while let Some(at) = rest.find("href") {
        rest = &rest[at + 4..];
        let value = rest
            .trim_start()
            .strip_prefix('=')
            .map(str::trim_start)
            .and_then(|value| {
                let quote = value.chars().next().filter(|c| matches!(c, '"' | '\''))?;
                value[1..].split(quote).next()
            });
        if let Some(value) = value
            && !(value.starts_with('#') || value.starts_with("data:"))
        {
            return Err("it refers to another file or address");
        }
    }
    if text.contains("url(") {
        let mut rest = text.as_str();
        while let Some(at) = rest.find("url(") {
            rest = &rest[at + 4..];
            let target = rest.trim_start().trim_start_matches(['"', '\'']);
            if !(target.starts_with('#') || target.starts_with("data:")) {
                return Err("it refers to another file or address");
            }
        }
    }
    let mut depth = 0_usize;
    let mut tags = text.split('<').skip(1);
    for tag in &mut tags {
        if tag.starts_with('/') {
            depth = depth.saturating_sub(1);
        } else if !(tag.starts_with('?') || tag.starts_with('!')) {
            let end = tag.find('>').unwrap_or(tag.len());
            if !tag[..end].trim_end().ends_with('/') {
                depth += 1;
                if depth > MAX_SVG_DEPTH {
                    return Err("its elements are nested too deeply");
                }
            }
        }
    }
    Ok(())
}

/// Reads and decodes `path`, refusing oversized files and images.
pub fn decode(path: &Path) -> Result<Decoded, ImageError> {
    let size = std::fs::metadata(path)?.len();
    if size > MAX_FILE_BYTES {
        return Err(ImageError::TooLarge(size));
    }
    let is_svg = path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case(SVG_EXTENSION));
    if is_svg {
        if size > MAX_SVG_BYTES {
            return Err(ImageError::UnsafeSvg("it is larger than 4 MB"));
        }
        let bytes = std::fs::read(path)?;
        check_svg(&bytes).map_err(ImageError::UnsafeSvg)?;
        return Ok(Decoded::Svg(bytes));
    }
    let mut reader = image::ImageReader::open(path)?.with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    limits.max_alloc = Some(MAX_ALLOC);
    reader.limits(limits);
    let rgba = reader.decode()?.into_rgba8();
    Ok(Decoded::Raster {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A throwaway folder under the system temp directory, removed on drop.
    struct Dir(crate::testutil::ScratchDir);

    impl Dir {
        fn new(name: &str) -> Self {
            Self(crate::testutil::scratch_dir(&format!("images-{name}")))
        }
    }

    fn write_png(path: &Path, width: u32, height: u32) {
        image::RgbaImage::new(width, height).save(path).unwrap();
    }

    #[test]
    fn resolves_relative_and_nested_images_inside_the_root() {
        let dir = Dir::new("resolve");
        let root = &dir.0;
        fs::create_dir_all(root.join("docs/img")).unwrap();
        write_png(&root.join("docs/img/a b.png"), 1, 1);
        write_png(&root.join("top.PNG"), 1, 1);
        let doc_dir = root.join("docs");
        assert_eq!(
            resolve("img/a%20b.png#frag", &doc_dir, root),
            Some(root.join("docs/img/a b.png"))
        );
        assert_eq!(
            resolve("../top.PNG", &doc_dir, root),
            Some(root.join("top.PNG"))
        );
        let file_uri = format!("file://{}", root.join("top.PNG").display());
        assert_eq!(
            resolve(&file_uri, &doc_dir, root),
            Some(root.join("top.PNG"))
        );
    }

    #[test]
    fn refuses_remote_outside_missing_and_unsupported() {
        let dir = Dir::new("refuse");
        let root = dir.0.join("site");
        fs::create_dir_all(&root).unwrap();
        write_png(&dir.0.join("outside.png"), 1, 1);
        write_png(&root.join("in.png"), 1, 1);
        fs::write(root.join("notes.txt"), "x").unwrap();
        assert_eq!(resolve("https://example.com/in.png", &root, &root), None);
        assert_eq!(resolve("data:image/png;base64,AAAA", &root, &root), None);
        assert_eq!(resolve("../outside.png", &root, &root), None);
        assert_eq!(resolve("missing.png", &root, &root), None);
        assert_eq!(resolve("notes.txt", &root, &root), None);
        assert_eq!(resolve("", &root, &root), None);
        assert!(resolve("in.png", &root, &root).is_some());
    }

    #[test]
    fn lists_images_inside_lists_and_quotes_in_order() {
        let source = "![a](a.png)\n\n- ![b](b.png)\n\n> ![c](c.png)\n\n![d](d.png)";
        let items: Vec<_> = markdown::parse(source).collect();
        assert_eq!(image_urls(&items), vec!["a.png", "b.png", "c.png", "d.png"]);
    }

    #[test]
    fn decodes_raster_and_svg() {
        let dir = Dir::new("decode");
        write_png(&dir.0.join("p.png"), 3, 2);
        fs::write(
            dir.0.join("s.svg"),
            "<svg xmlns='http://www.w3.org/2000/svg'/>",
        )
        .unwrap();
        match decode(&dir.0.join("p.png")).unwrap() {
            Decoded::Raster {
                width,
                height,
                rgba,
            } => {
                assert_eq!((width, height), (3, 2));
                assert_eq!(rgba.len(), 3 * 2 * 4);
            }
            other @ Decoded::Svg(_) => panic!("expected raster, got {other:?}"),
        }
        assert!(matches!(
            decode(&dir.0.join("s.svg")).unwrap(),
            Decoded::Svg(_)
        ));
    }

    #[test]
    fn rejects_corrupt_and_oversized_images() {
        let dir = Dir::new("reject");
        fs::write(dir.0.join("bad.png"), b"not a png").unwrap();
        assert!(matches!(
            decode(&dir.0.join("bad.png")),
            Err(ImageError::Decode(_))
        ));
        assert!(matches!(
            decode(&dir.0.join("absent.png")),
            Err(ImageError::Io(_))
        ));
        write_png(&dir.0.join("wide.png"), MAX_SIDE + 1, 1);
        assert!(matches!(
            decode(&dir.0.join("wide.png")),
            Err(ImageError::Decode(_))
        ));
    }

    /// The demo document stays testable: opened from the repository root, every image it shows
    /// resolves, and the failure examples do not.
    #[test]
    fn the_demo_document_resolves_its_images_from_the_repo_root() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let demo = root.join("tests/fixtures/images/demo.md");
        let text = fs::read_to_string(&demo).unwrap();
        let items: Vec<_> = markdown::parse(&text).collect();
        let doc_dir = demo.parent().unwrap();
        let (shown, failed): (Vec<_>, Vec<_>) = image_urls(&items)
            .into_iter()
            .partition(|url| resolve(url, doc_dir, root).is_some());
        assert_eq!(shown.len(), 9, "shown: {shown:?}");
        assert_eq!(
            failed,
            vec![
                "img/does-not-exist.png",
                "https://example.com/logo.png",
                "img/notes.txt"
            ]
        );
    }

    #[test]
    fn plain_svg_passes_the_guard() {
        let fixture = include_bytes!("../../tests/fixtures/images/img/shapes.svg");
        assert_eq!(check_svg(fixture), Ok(()));
        let inline =
            br##"<svg xmlns="http://www.w3.org/2000/svg"><defs><linearGradient id="g"/></defs>
            <rect fill="url(#g)" style="fill:url('#g')"/><use href="#g"/>
            <image href="data:image/png;base64,AAAA"/></svg>"##;
        assert_eq!(check_svg(inline), Ok(()));
    }

    #[test]
    fn svg_that_reaches_outside_itself_is_refused() {
        let refuse = |svg: &str| check_svg(svg.as_bytes()).is_err();
        assert!(refuse(r#"<svg><image href="/etc/hosts.png"/></svg>"#));
        assert!(refuse(r#"<svg><image xlink:href="file:///x.png"/></svg>"#));
        assert!(refuse(
            r"<svg><image HREF = 'https://example.com/a.png'/></svg>"
        ));
        assert!(refuse(r#"<svg><use href="other.svg#a"/></svg>"#));
        assert!(refuse(
            r#"<svg><rect fill="url(http://example.com/x)"/></svg>"#
        ));
        assert!(refuse(r"<svg><style>@import url(x.css);</style></svg>"));
        assert!(refuse(
            r#"<!DOCTYPE svg [<!ENTITY a "aaaa"><!ENTITY b "&a;&a;">]><svg>&b;</svg>"#
        ));
    }

    #[test]
    fn svg_nested_absurdly_deep_is_refused() {
        let ok = format!("<svg>{}{}</svg>", "<g>".repeat(100), "</g>".repeat(100));
        assert_eq!(check_svg(ok.as_bytes()), Ok(()));
        let deep = format!("<svg>{}{}</svg>", "<g>".repeat(500), "</g>".repeat(500));
        assert!(check_svg(deep.as_bytes()).is_err());
        let flat = format!("<svg>{}</svg>", "<rect/>".repeat(10_000));
        assert_eq!(check_svg(flat.as_bytes()), Ok(()));
    }

    #[test]
    fn decode_refuses_an_unsafe_or_oversized_svg_file() {
        let dir = Dir::new("svg-guard");
        fs::write(
            dir.0.join("bad.svg"),
            r#"<svg><image href="/etc/hosts.png"/></svg>"#,
        )
        .unwrap();
        assert!(matches!(
            decode(&dir.0.join("bad.svg")),
            Err(ImageError::UnsafeSvg(_))
        ));
        let big = fs::File::create(dir.0.join("big.svg")).unwrap();
        big.set_len(MAX_SVG_BYTES + 1).unwrap();
        assert!(matches!(
            decode(&dir.0.join("big.svg")),
            Err(ImageError::UnsafeSvg(_))
        ));
    }

    mod properties {
        use super::super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn the_svg_guard_never_panics_on_arbitrary_bytes(bytes in proptest::collection::vec(any::<u8>(), 0..400)) {
                let _ = check_svg(&bytes);
            }

            #[test]
            fn an_href_to_anything_but_a_fragment_or_data_uri_is_refused(
                target in "[a-z/.:]{1,12}", quote in prop_oneof![Just('"'), Just('\'')]
            ) {
                prop_assume!(!target.starts_with('#') && !target.starts_with("data:"));
                let svg = format!("<svg><image href={quote}{target}{quote}/></svg>");
                prop_assert!(check_svg(svg.as_bytes()).is_err());
            }

            #[test]
            fn resolving_an_image_uri_never_panics(uri in "\\PC{0,60}") {
                let root = Path::new("/nonexistent-root");
                let _ = resolve(&uri, root, root);
            }
        }
    }
}
