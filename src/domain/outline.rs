// SPDX-License-Identifier: GPL-3.0-or-later
use iced::widget::markdown;

/// A heading in the rendered document, numbered in visual order.
#[derive(Debug)]
pub struct OutlineEntry {
    pub level: u8,
    pub title: String,
    pub index: usize,
}

pub fn extract_outline(items: &[markdown::Item]) -> Vec<OutlineEntry> {
    let style = markdown::Style::from_palette(iced::Theme::Light.palette());
    items
        .iter()
        .filter_map(|item| {
            let markdown::Item::Heading(level, text) = item else {
                return None;
            };
            let title = text
                .spans(style)
                .iter()
                .map(|span| span.text.as_ref())
                .collect::<String>();
            Some(OutlineEntry {
                level: heading_depth(*level),
                title,
                index: 0,
            })
        })
        .enumerate()
        .map(|(index, mut entry)| {
            entry.index = index;
            entry
        })
        .collect()
}

fn heading_depth(level: markdown::HeadingLevel) -> u8 {
    match level {
        markdown::HeadingLevel::H1 => 1,
        markdown::HeadingLevel::H2 => 2,
        markdown::HeadingLevel::H3 => 3,
        markdown::HeadingLevel::H4 => 4,
        markdown::HeadingLevel::H5 => 5,
        markdown::HeadingLevel::H6 => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_indices_follow_rendered_blocks_not_source_length() {
        let items: Vec<_> =
            markdown::parse("# First\n\n```\n# not a heading\n```\n\n## Second\n\n# First")
                .collect();
        let outline = extract_outline(&items);
        assert_eq!(
            outline
                .iter()
                .map(|entry| (entry.index, entry.title.as_str()))
                .collect::<Vec<_>>(),
            vec![(0, "First"), (1, "Second"), (2, "First")]
        );
    }
}
