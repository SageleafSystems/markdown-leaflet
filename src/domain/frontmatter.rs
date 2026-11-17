// SPDX-License-Identifier: GPL-3.0-or-later
//! YAML frontmatter: the `---`-delimited metadata block some Markdown tools (Jekyll, Hugo,
//! Obsidian, Zettlr) put at the very start of a file. `split` pulls it off the raw source so
//! `markdown::parse` and the outline never see it; `parse` turns it into a [`Frontmatter`] for
//! the Details panel. Only YAML is recognized (see issue #65); a `+++` (TOML) or other block is
//! left as ordinary Markdown.

use std::fmt;

/// One frontmatter value, shaped for display rather than for round-tripping YAML. `Map` and
/// `List` preserve source order so the panel reads top-to-bottom the way the file does.
#[derive(Debug, Clone, PartialEq)]
pub enum FrontmatterValue {
    /// A string, number, bool, or null, already formatted for display. Null renders as an
    /// empty string rather than the word "null", since `key:` with nothing after it is how
    /// most of these tools write "no value".
    Scalar(String),
    List(Vec<FrontmatterValue>),
    Map(Vec<(String, FrontmatterValue)>),
}

/// A parsed frontmatter block: its exact source text (for a future "view raw" affordance) and
/// its top-level fields in file order.
#[derive(Debug, Clone, PartialEq)]
pub struct Frontmatter {
    pub raw: String,
    pub fields: Vec<(String, FrontmatterValue)>,
}

impl Frontmatter {
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// The first top-level field named `title`, if it is a non-empty scalar. Used to override
    /// the file name in the window title and toolbar (issue #65).
    pub fn title(&self) -> Option<&str> {
        self.fields.iter().find_map(|(key, value)| match value {
            FrontmatterValue::Scalar(title) if key == "title" && !title.is_empty() => {
                Some(title.as_str())
            }
            _ => None,
        })
    }
}

/// The block failed to parse as YAML, or parsed to something other than a mapping (frontmatter
/// is always `key: value` pairs, never a bare list or scalar).
#[derive(Debug, thiserror::Error)]
pub enum FrontmatterError {
    #[error("{0}")]
    Yaml(#[from] serde_saphyr::Error),
    #[error("frontmatter must be a mapping of keys to values, not a {0}")]
    NotAMapping(&'static str),
}

/// Splits a leading `---`/`---` (or `---`/`...`) YAML block off `source`, if one is present.
/// Returns `(Some(raw_block), body)` when found — `raw_block` is the exact text between the
/// delimiters (not yet parsed; it may still be invalid YAML) and `body` is everything after the
/// closing delimiter's line, which is what `markdown::parse` should see. Returns `(None,
/// source)` when `source` doesn't open with a bare `---` line, or that opening line is never
/// closed, so today's rendering (the `---` as an ordinary thematic break) is unaffected.
///
/// Detection is byte-0 only (after an optional UTF-8 BOM): a `---` anywhere else in the document
/// is ordinary Markdown and is never touched.
pub fn split(source: &str) -> (Option<&str>, &str) {
    let scanned = source.strip_prefix('\u{FEFF}').unwrap_or(source);
    let Some(after_open_dashes) = scanned.strip_prefix("---") else {
        return (None, source);
    };
    // The opening line must be exactly `---`, plus only trailing whitespace, then a newline.
    let Some(newline) = after_open_dashes.find('\n') else {
        return (None, source);
    };
    let (opening_rest, after_open_line) = after_open_dashes.split_at(newline + 1);
    if !opening_rest.trim_end().is_empty() {
        return (None, source);
    }

    let mut offset = 0;
    for line in after_open_line.split_inclusive('\n') {
        if matches!(line.trim_end(), "---" | "...") {
            let raw = &after_open_line[..offset];
            let body = &after_open_line[offset + line.len()..];
            return (Some(raw), body);
        }
        offset += line.len();
    }
    (None, source)
}

/// Parses a frontmatter block's raw text (as returned by [`split`]) into its fields. A blank
/// block (`---`/`---` with nothing between) is valid frontmatter with no fields, not an error.
pub fn parse(raw: &str) -> Result<Frontmatter, FrontmatterError> {
    if raw.trim().is_empty() {
        return Ok(Frontmatter {
            raw: raw.to_string(),
            fields: Vec::new(),
        });
    }
    let fields = match serde_saphyr::from_str::<FrontmatterValue>(raw)? {
        FrontmatterValue::Map(fields) => fields,
        FrontmatterValue::List(_) => return Err(FrontmatterError::NotAMapping("list")),
        FrontmatterValue::Scalar(_) => return Err(FrontmatterError::NotAMapping("scalar")),
    };
    Ok(Frontmatter {
        raw: raw.to_string(),
        fields,
    })
}

impl<'de> serde::de::Deserialize<'de> for FrontmatterValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}

struct ValueVisitor;

impl<'de> serde::de::Visitor<'de> for ValueVisitor {
    type Value = FrontmatterValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a YAML scalar, sequence, or mapping")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E> {
        Ok(FrontmatterValue::Scalar(v.to_string()))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
        Ok(FrontmatterValue::Scalar(v.to_string()))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
        Ok(FrontmatterValue::Scalar(v.to_string()))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E> {
        Ok(FrontmatterValue::Scalar(v.to_string()))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
        Ok(FrontmatterValue::Scalar(v.to_string()))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E> {
        Ok(FrontmatterValue::Scalar(v))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(FrontmatterValue::Scalar(String::new()))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(FrontmatterValue::Scalar(String::new()))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element()? {
            items.push(item);
        }
        Ok(FrontmatterValue::List(items))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut fields = Vec::new();
        while let Some((key, value)) = map.next_entry::<String, FrontmatterValue>()? {
            fields.push((key, value));
        }
        Ok(FrontmatterValue::Map(fields))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scalar(s: &str) -> FrontmatterValue {
        FrontmatterValue::Scalar(s.to_string())
    }

    #[test]
    fn no_delimiter_is_not_frontmatter() {
        let source = "# Title\n\nBody.\n";
        assert_eq!(split(source), (None, source));
    }

    #[test]
    fn simple_block_is_split_off() {
        let source = "---\ntitle: Example\n---\n\n# Body\n";
        let (raw, body) = split(source);
        assert_eq!(raw, Some("title: Example\n"));
        assert_eq!(body, "\n# Body\n");
    }

    #[test]
    fn closed_with_ellipsis() {
        let source = "---\ntitle: Example\n...\nBody\n";
        let (raw, body) = split(source);
        assert_eq!(raw, Some("title: Example\n"));
        assert_eq!(body, "Body\n");
    }

    #[test]
    fn empty_block_has_empty_raw() {
        let source = "---\n---\nBody\n";
        let (raw, body) = split(source);
        assert_eq!(raw, Some(""));
        assert_eq!(body, "Body\n");
    }

    #[test]
    fn unterminated_block_is_not_frontmatter() {
        let source = "---\ntitle: Example\n\n# Body with no closing delimiter\n";
        assert_eq!(split(source), (None, source));
    }

    #[test]
    fn four_dashes_is_not_a_delimiter() {
        let source = "----\nnot frontmatter\n----\n";
        assert_eq!(split(source), (None, source));
    }

    #[test]
    fn trailing_whitespace_on_opening_line_is_allowed() {
        let source = "---   \ntitle: Example\n---\nBody\n";
        let (raw, body) = split(source);
        assert_eq!(raw, Some("title: Example\n"));
        assert_eq!(body, "Body\n");
    }

    #[test]
    fn crlf_line_endings_are_handled() {
        let source = "---\r\ntitle: Example\r\n---\r\nBody\r\n";
        let (raw, body) = split(source);
        assert_eq!(raw, Some("title: Example\r\n"));
        assert_eq!(body, "Body\r\n");
    }

    #[test]
    fn leading_bom_is_skipped_for_detection() {
        let source = "\u{FEFF}---\ntitle: Example\n---\nBody\n";
        let (raw, body) = split(source);
        assert_eq!(raw, Some("title: Example\n"));
        assert_eq!(body, "Body\n");
    }

    #[test]
    fn a_rule_elsewhere_in_the_document_is_untouched() {
        let source = "# Title\n\n---\n\nMore.\n";
        assert_eq!(split(source), (None, source));
    }

    #[test]
    fn parses_scalars_lists_and_nested_maps_in_order() {
        let fm = parse(
            "title: Example\ntags: [one, two]\nauthor:\n  name: Ada\n  email: ada@example.com\n",
        )
        .unwrap();
        assert_eq!(
            fm.fields,
            vec![
                ("title".to_string(), scalar("Example")),
                (
                    "tags".to_string(),
                    FrontmatterValue::List(vec![scalar("one"), scalar("two")])
                ),
                (
                    "author".to_string(),
                    FrontmatterValue::Map(vec![
                        ("name".to_string(), scalar("Ada")),
                        ("email".to_string(), scalar("ada@example.com")),
                    ])
                ),
            ]
        );
    }

    #[test]
    fn blank_block_is_empty_frontmatter_not_an_error() {
        let fm = parse("").unwrap();
        assert!(fm.is_empty());
        let fm = parse("   \n").unwrap();
        assert!(fm.is_empty());
    }

    #[test]
    fn malformed_yaml_is_an_error() {
        let err = parse("tags: [one, two\n").unwrap_err();
        assert!(matches!(err, FrontmatterError::Yaml(_)));
    }

    #[test]
    fn a_bare_list_is_not_a_mapping() {
        let err = parse("- one\n- two\n").unwrap_err();
        assert!(matches!(err, FrontmatterError::NotAMapping("list")));
    }

    #[test]
    fn a_bare_scalar_is_not_a_mapping() {
        let err = parse("just text\n").unwrap_err();
        assert!(matches!(err, FrontmatterError::NotAMapping("scalar")));
    }

    #[test]
    fn title_reads_a_non_empty_top_level_scalar() {
        let fm = parse("title: Example\n").unwrap();
        assert_eq!(fm.title(), Some("Example"));
    }

    #[test]
    fn title_ignores_a_null_or_missing_value() {
        assert_eq!(parse("date: 2026-01-15\n").unwrap().title(), None);
        assert_eq!(parse("title:\n").unwrap().title(), None);
    }
}
