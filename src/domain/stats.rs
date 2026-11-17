// SPDX-License-Identifier: GPL-3.0-or-later
//! Counts shown in the inspector panel, computed once when a document loads.
use pulldown_cmark::{Event, Options, Parser, Tag};

/// Size of a document's text and how many links and images it holds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DocumentStats {
    pub words: usize,
    pub characters: usize,
    pub lines: usize,
    pub links: usize,
    pub images: usize,
}

/// A shape `DocumentStats::format` can render its six metrics into, for the details panel's
/// "Copy All" button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatsFormat {
    #[default]
    PlainText,
    Json,
    Yaml,
    Csv,
}

impl StatsFormat {
    pub const ALL: [Self; 4] = [Self::PlainText, Self::Json, Self::Yaml, Self::Csv];

    /// The short badge text shown as the "Copy All" button's type indicator.
    pub fn label(self) -> &'static str {
        match self {
            Self::PlainText => "TXT",
            Self::Json => "JSON",
            Self::Yaml => "YAML",
            Self::Csv => "CSV",
        }
    }
}

impl DocumentStats {
    /// The six metrics shown in the inspector, in display order. `headings` comes from the
    /// document's outline, which `DocumentStats` itself does not hold.
    fn entries(&self, headings: usize) -> [(&'static str, usize); 6] {
        [
            ("Words", self.words),
            ("Characters", self.characters),
            ("Lines", self.lines),
            ("Headings", headings),
            ("Links", self.links),
            ("Images", self.images),
        ]
    }

    /// Renders every metric as one block in `format`, for the "Copy All" button.
    pub fn format(&self, headings: usize, format: StatsFormat) -> String {
        let entries = self.entries(headings);
        match format {
            StatsFormat::PlainText => entries
                .iter()
                .map(|(label, value)| format!("{label}: {value}"))
                .collect::<Vec<_>>()
                .join("\n"),
            StatsFormat::Json => {
                let body = entries
                    .iter()
                    .map(|(label, value)| format!("  \"{}\": {value}", label.to_lowercase()))
                    .collect::<Vec<_>>()
                    .join(",\n");
                format!("{{\n{body}\n}}")
            }
            StatsFormat::Yaml => entries
                .iter()
                .map(|(label, value)| format!("{}: {value}", label.to_lowercase()))
                .collect::<Vec<_>>()
                .join("\n"),
            StatsFormat::Csv => {
                let rows = entries
                    .iter()
                    .map(|(label, value)| format!("{},{value}", label.to_lowercase()));
                std::iter::once("metric,value".to_string())
                    .chain(rows)
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
    }

    /// Measures the raw Markdown `source`. Links and images count inline and reference-style
    /// ones, including those inside tables, which the rendered item list does not expose.
    pub fn of(source: &str) -> Self {
        let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
        let (mut links, mut images) = (0, 0);
        for event in Parser::new_ext(source, options) {
            match event {
                Event::Start(Tag::Link { .. }) => links += 1,
                Event::Start(Tag::Image { .. }) => images += 1,
                _ => {}
            }
        }
        Self {
            words: source.split_whitespace().count(),
            characters: source.chars().count(),
            lines: source.lines().count(),
            links,
            images,
        }
    }
}

/// Groups digits in thousands, e.g. `4038` becomes `4,038`.
pub fn group_thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_text_size() {
        let stats = DocumentStats::of("# Title\n\nTwo words.\n");
        assert_eq!(stats.words, 4);
        assert_eq!(stats.characters, 20);
        assert_eq!(stats.lines, 3);
    }

    #[test]
    fn counts_links_and_images_including_tables_and_references() {
        let source = "[a](x.md) and ![i](p.png)\n\n[ref]: https://e.com\n\nsee [r][ref]\n\n\
                      | h |\n|---|\n| [t](y.md) ![j](q.png) |\n";
        let stats = DocumentStats::of(source);
        assert_eq!(stats.links, 3);
        assert_eq!(stats.images, 2);
    }

    #[test]
    fn ignores_links_in_code() {
        let stats = DocumentStats::of("`[a](b)`\n\n```\n![i](p.png)\n```\n");
        assert_eq!((stats.links, stats.images), (0, 0));
    }

    #[test]
    fn empty_source_is_all_zero() {
        assert_eq!(DocumentStats::of(""), DocumentStats::default());
    }

    #[test]
    fn formats_every_metric_in_each_shape() {
        let stats = DocumentStats {
            words: 4038,
            characters: 20,
            lines: 3,
            links: 1,
            images: 2,
        };
        assert_eq!(
            stats.format(5, StatsFormat::PlainText),
            "Words: 4038\nCharacters: 20\nLines: 3\nHeadings: 5\nLinks: 1\nImages: 2"
        );
        assert_eq!(
            stats.format(5, StatsFormat::Json),
            "{\n  \"words\": 4038,\n  \"characters\": 20,\n  \"lines\": 3,\n  \"headings\": 5,\n  \"links\": 1,\n  \"images\": 2\n}"
        );
        assert_eq!(
            stats.format(5, StatsFormat::Yaml),
            "words: 4038\ncharacters: 20\nlines: 3\nheadings: 5\nlinks: 1\nimages: 2"
        );
        assert_eq!(
            stats.format(5, StatsFormat::Csv),
            "metric,value\nwords,4038\ncharacters,20\nlines,3\nheadings,5\nlinks,1\nimages,2"
        );
    }

    #[test]
    fn groups_thousands() {
        assert_eq!(group_thousands(0), "0");
        assert_eq!(group_thousands(999), "999");
        assert_eq!(group_thousands(4038), "4,038");
        assert_eq!(group_thousands(1_234_567), "1,234,567");
    }
}
