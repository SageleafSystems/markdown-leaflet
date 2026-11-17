// SPDX-License-Identifier: GPL-3.0-or-later
//! Renders the open document's raw Markdown source (issue #71): one selectable line of
//! monospace text per line of the file, instead of the parsed-and-rendered preview, colored by
//! `ui::syntax`'s Markdown grammar (token `"md"`) the same way a code block's language colors it
//! (issue #74).
//!
//! Each line is its own leaf (`BlockId { item: line, leaf: 0 }`), so `domain::selection`'s
//! range math, `ui::selectable_text::SelectableText`'s drag handling, and `State::leaves`/
//! `State::selected_text` all work the same way they do over the preview's leaves — nothing
//! about them is preview-specific.

use crate::app::{ContextTarget, FindState, Message};
use crate::domain::selection::{BlockId, Selection};
use crate::layout::{budget, spacing, typography};
use crate::ui::context_area::ContextArea;
use crate::ui::fonts;
use crate::ui::locate_heading::DOC_SCROLL;
use crate::ui::selectable_text::{Drag, SelectableText};
use crate::ui::selection_area::SelectionArea;
use crate::ui::syntax;
use crate::ui::theme::{self, Appearance};
use iced::advanced::text::LineHeight;
use iced::widget::{Column, column, container, rich_text, row, scrollable, text};
use iced::{Element, Fill, Padding, Pixels};

/// The plain text of every line, paired with the `BlockId` it renders under — the Source-mode
/// equivalent of `ui::find_viewer::leaf_texts`, read back the same way for copy and Select All.
pub(crate) fn leaf_texts(raw: &str) -> Vec<(BlockId, String)> {
    raw.lines()
        .enumerate()
        .map(|(line, text)| {
            (
                BlockId {
                    item: line,
                    leaf: 0,
                },
                text.to_string(),
            )
        })
        .collect()
}

/// The column width to break at, whether the file is plain text rather than Markdown (issue
/// #79, which drops coloring and line numbers), and the zoom multiplier (issue #76).
#[derive(Clone, Copy)]
pub(crate) struct SourceSetting {
    pub measure: f32,
    pub plain: bool,
    pub zoom: f32,
}

/// Renders `raw` as selectable, find-highlighted monospace text, one leaf per line, wrapped the
/// same way `app::view::render_markdown` wraps the preview (scroll, drag-select, context menu).
pub(crate) fn render_source<'a>(
    raw: &'a str,
    appearance: Appearance,
    find: &'a FindState,
    selection: Option<Selection>,
    drag: Drag,
    setting: SourceSetting,
) -> Element<'a, Message> {
    let SourceSetting {
        measure,
        plain,
        zoom,
    } = setting;
    let font = fonts::body(crate::config::FontChoice::Mono);
    let size = Pixels(13.0 * zoom);
    let line_height = LineHeight::Relative(typography::heading_ratio(typography::body_ratio(
        size.0, measure,
    )));
    let total_lines = raw.lines().count().max(1);
    let number_width = total_lines.to_string().len();
    // Plain text (issue #79) is not Markdown: no coloring and no line numbers.
    let highlighted = syntax::highlight_lines(
        if plain { "" } else { "md" },
        raw.lines(),
        appearance,
        font,
        theme::colors(appearance).text,
    );
    let mut content: Column<'_, Message> = column![].spacing(0);
    let mut count = 0usize;
    for (index, (line, styled)) in raw.lines().zip(highlighted).enumerate() {
        let block = BlockId {
            item: index,
            leaf: 0,
        };
        let active = find.matches.get(find.active).and_then(|&matched| {
            (matched == index).then(|| {
                find.matches[..find.active]
                    .iter()
                    .filter(|&&i| i == index)
                    .count()
            })
        });
        let selected = selection.and_then(|selection| selection.range_in(block, line.len()));
        let spans = syntax::slice(
            styled,
            line,
            if find.visible { &find.query } else { "" },
            active,
            &mut count,
            selected,
            appearance,
        );
        let inner = rich_text(spans.clone())
            .font(font)
            .size(size)
            .line_height(line_height);
        let leaf = SelectableText::new(inner, spans, block, size, Some(font), drag, Message::from)
            .line_height(line_height);
        let numbered = if plain {
            row![Element::from(leaf)]
        } else {
            row![
                text(format!("{:>number_width$}", index + 1))
                    .font(font)
                    .size(size)
                    .line_height(line_height)
                    .color(theme::colors(appearance).text_muted),
                Element::from(leaf),
            ]
            .spacing(spacing::UNIT)
        };
        content = content.push(container(numbered).id(format!("document-find-{index}")));
    }
    let preview = scrollable(container(content).padding(Padding {
        top: spacing::units(2.0),
        right: budget::SCROLLBAR_CLEARANCE,
        bottom: spacing::units(2.0),
        left: budget::TEXT_INSET,
    }))
    .id(DOC_SCROLL)
    .on_scroll(Message::PreviewScrolled)
    .width(Fill)
    .height(Fill);
    let preview = SelectionArea::new(
        preview,
        drag.selecting,
        Message::SelectionCleared,
        Message::SelectionAutoScroll,
    );
    ContextArea::new(preview, |at| {
        Message::OpenContextMenu(ContextTarget::Preview, at)
    })
    .into()
}
