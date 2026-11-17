// SPDX-License-Identifier: GPL-3.0-or-later
//! Shared syntax-highlighting plumbing over `iced::highlighter` (syntect via two-face).
//!
//! Two callers share this: `ui::find_viewer::FindViewer::code_block`, which replaces the
//! Markdown widget's own built-in code-block highlighting — fixed to a single dark theme
//! regardless of the app's appearance — with a theme matched to it; and `ui::source_view`,
//! which colors the raw Markdown source itself the same way, using `two-face`'s bundled
//! Markdown grammar (token `"md"`) instead of a code fence's language.

use crate::domain::find;
use crate::ui::theme::{self, Appearance};
use iced::advanced::text::Span;
use iced::widget::markdown::Uri;
use iced::{Color, Font, highlighter};
use std::ops::Range;

/// The syntect theme to color `token` with, matched to the app's light/dark appearance so
/// highlighted text stays legible against the panel behind it.
fn theme_for(appearance: Appearance) -> highlighter::Theme {
    if theme::is_dark(appearance) {
        highlighter::Theme::Base16Ocean
    } else {
        highlighter::Theme::InspiredGitHub
    }
}

/// Highlights `lines` of `token` syntax (a language name or extension such as `"rust"`, or
/// `"md"` for Markdown itself), one output line of spans per input line, in the same order.
/// A `token` the bundled grammars don't recognize highlights as plain text: every span then
/// carries `default_color` and `default_font` unchanged, so callers need no fallback of their
/// own for an unknown language.
pub(crate) fn highlight_lines<'a>(
    token: &str,
    lines: impl Iterator<Item = &'a str>,
    appearance: Appearance,
    default_font: Font,
    default_color: Color,
) -> Vec<Vec<Span<'static, Uri>>> {
    let mut stream = highlighter::Stream::new(&highlighter::Settings {
        theme: theme_for(appearance),
        token: token.to_string(),
    });
    lines
        .map(|line| {
            let mut spans: Vec<_> = stream
                .highlight_line(line)
                .map(|(range, highlight)| {
                    Span::new(line[range].to_string())
                        .color(highlight.color().unwrap_or(default_color))
                        .font(highlight.font().unwrap_or(default_font))
                })
                .collect();
            stream.commit();
            // A blank line highlights to no ranges at all; keep one empty span rather than none,
            // so it still renders as a line (and still gets a `BlockId` from the caller's
            // `next_leaf`) instead of collapsing.
            if spans.is_empty() {
                spans.push(
                    Span::new(String::new())
                        .color(default_color)
                        .font(default_font),
                );
            }
            spans
        })
        .collect()
}

/// Slices `styled` (spans whose concatenated text is `visible`) at find-match and `selected`
/// boundaries, overlaying a highlight background — and, for a find match, a contrasting
/// foreground — while leaving each source span's own font and color alone elsewhere. `active`
/// names which match on this line, if any, is the globally active one; `count` accumulates
/// matches found on earlier lines and is advanced by this line's own, mirroring how
/// `ui::find_viewer::FindViewer::count` accumulates matches across a document's leaves.
///
/// Shared by `FindViewer::sliced` (parsed Markdown spans, and this module's syntax-highlighted
/// code-block lines) and `ui::source_view` (this module's syntax-highlighted Markdown-source
/// lines) — the two colorizations in issue #74 need the exact same overlay, just starting from
/// different "source" spans.
#[allow(clippy::too_many_arguments)]
pub(crate) fn slice(
    styled: Vec<Span<'static, Uri>>,
    visible: &str,
    query: &str,
    active: Option<usize>,
    count: &mut usize,
    selected: Option<Range<usize>>,
    appearance: Appearance,
) -> Vec<Span<'static, Uri>> {
    if query.is_empty() && selected.is_none() {
        return styled;
    }

    let ranges = find::match_ranges(visible, query);
    let first = *count;
    *count += ranges.len();
    let mut output = Vec::new();
    let mut span_start = 0;
    for source in &styled {
        let value = source.text.as_ref();
        let span_end = span_start + value.len();
        let mut boundaries = vec![span_start, span_end];
        for range in &ranges {
            if range.start > span_start && range.start < span_end {
                boundaries.push(range.start);
            }
            if range.end > span_start && range.end < span_end {
                boundaries.push(range.end);
            }
        }
        if let Some(selected) = &selected {
            if selected.start > span_start && selected.start < span_end {
                boundaries.push(selected.start);
            }
            if selected.end > span_start && selected.end < span_end {
                boundaries.push(selected.end);
            }
        }
        boundaries.sort_unstable();
        boundaries.dedup();
        for pair in boundaries.windows(2) {
            let start = pair[0];
            let end = pair[1];
            let mut part = source.clone();
            part.text = value[start - span_start..end - span_start]
                .to_string()
                .into();
            let in_selection = selected
                .as_ref()
                .is_some_and(|range| range.start <= start && end <= range.end);
            if in_selection {
                part = part.background(theme::selection_background(appearance));
            } else if let Some((index, _)) = ranges
                .iter()
                .enumerate()
                .find(|(_, range)| range.start <= start && end <= range.end)
            {
                let color = if active == Some(first + index) {
                    appearance.colors.accent
                } else {
                    appearance.colors.warning
                };
                part = part.background(color).color(appearance.colors.on_accent);
            }
            output.push(part);
        }
        if value.is_empty() {
            let mut part = source.clone();
            part.text = String::new().into();
            output.push(part);
        }
        span_start = span_end;
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::Appearance;

    const DEFAULT_FONT: Font = Font::MONOSPACE;
    const DEFAULT_COLOR: Color = Color::WHITE;

    fn highlight(token: &str, source: &str) -> Vec<Vec<Span<'static, Uri>>> {
        highlight_lines(
            token,
            source.lines(),
            Appearance::SLATE,
            DEFAULT_FONT,
            DEFAULT_COLOR,
        )
    }

    #[test]
    fn a_recognized_language_colors_at_least_one_span_apart_from_the_default() {
        let lines = highlight("rust", "fn main() {}");
        let colored = lines
            .iter()
            .flatten()
            .any(|span| span.color != Some(DEFAULT_COLOR));
        assert!(colored, "expected a keyword to get its own color");
    }

    #[test]
    fn an_unrecognized_token_falls_back_to_the_defaults_throughout() {
        let lines = highlight("not-a-real-language", "fn main() {}");
        for span in lines.iter().flatten() {
            assert_eq!(span.color, Some(DEFAULT_COLOR));
            assert_eq!(span.font, Some(DEFAULT_FONT));
        }
    }

    #[test]
    fn a_blank_line_stays_one_empty_span_instead_of_none() {
        let lines = highlight("rust", "fn main() {\n\n}");
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[1].len(), 1);
        assert_eq!(lines[1][0].text.as_ref(), "");
    }

    #[test]
    fn markdown_is_a_recognized_token() {
        let lines = highlight("md", "# Heading\n\nA paragraph with `code`.");
        let colored = lines
            .iter()
            .flatten()
            .any(|span| span.color != Some(DEFAULT_COLOR));
        assert!(colored, "expected the bundled Markdown grammar to be found");
    }

    #[test]
    fn with_no_query_or_selection_slicing_is_a_no_op() {
        let styled = vec![Span::new("hello world".to_string()).color(DEFAULT_COLOR)];
        let mut count = 0;
        let sliced = slice(
            styled.clone(),
            "hello world",
            "",
            None,
            &mut count,
            None,
            Appearance::SLATE,
        );
        assert_eq!(sliced.len(), styled.len());
        assert_eq!(sliced[0].text.as_ref(), "hello world");
        assert_eq!(count, 0);
    }

    #[test]
    fn a_selection_backgrounds_only_its_own_range_and_keeps_the_source_color() {
        let styled = vec![Span::new("hello world".to_string()).color(DEFAULT_COLOR)];
        let mut count = 0;
        let sliced = slice(
            styled,
            "hello world",
            "",
            None,
            &mut count,
            Some(0..5),
            Appearance::SLATE,
        );
        let texts: Vec<&str> = sliced.iter().map(|span| span.text.as_ref()).collect();
        assert_eq!(texts, vec!["hello", " world"]);
        assert!(sliced[0].highlight.is_some());
        assert_eq!(sliced[0].color, Some(DEFAULT_COLOR));
        assert!(sliced[1].highlight.is_none());
    }

    #[test]
    fn a_find_match_overrides_color_for_contrast_and_counts_it() {
        let styled = vec![Span::new("find it here".to_string()).color(DEFAULT_COLOR)];
        let mut count = 0;
        let sliced = slice(
            styled,
            "find it here",
            "find",
            Some(0),
            &mut count,
            None,
            Appearance::SLATE,
        );
        assert_eq!(count, 1);
        let texts: Vec<&str> = sliced.iter().map(|span| span.text.as_ref()).collect();
        assert_eq!(texts, vec!["find", " it here"]);
        assert_eq!(sliced[0].color, Some(Appearance::SLATE.colors.on_accent));
        assert!(sliced[0].highlight.is_some());
    }
}
