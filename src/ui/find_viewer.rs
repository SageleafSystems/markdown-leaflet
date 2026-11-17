// SPDX-License-Identifier: GPL-3.0-or-later
//! A markdown viewer that highlights in-document find matches and makes
//! every rendered heading, paragraph, code line, and image caption
//! selectable (see `ui::selectable_text`).

use crate::app::TaskToggleEvent;
use crate::domain::selection::{BlockId, Selection};
use crate::domain::tasks::{self, TaskId};
use crate::layout::typography;
use crate::ui::context_area::ContextArea;
use crate::ui::images::{ImageMenu, ImageStore, Loaded, Slot};
use crate::ui::selectable_text::{Drag, SelectableText, SelectionEvent};
use crate::ui::syntax;
use crate::ui::theme::Appearance;
use iced::widget::text::LineHeight;
use iced::widget::{
    checkbox, column, container, image, markdown, progress_bar, rich_text, row, scrollable, svg,
    text,
};
use iced::{ContentFit, Element, Fill, Length, Padding, Pixels, alignment};
use std::cell::{Cell, RefCell};

pub struct FindViewer<'a> {
    query: &'a str,
    active: Option<usize>,
    appearance: Appearance,
    count: Cell<usize>,
    item_index: usize,
    next_leaf: Cell<usize>,
    /// How many `Bullet::Task`s have been rendered under this item so far, in document order
    /// (both `unordered_list` and `ordered_list` increment it), matching `domain::tasks`'
    /// counting.
    next_task: Cell<usize>,
    /// Plain text of each leaf rendered so far, in render order; read back
    /// as `leaf_texts` once rendering of this item is done.
    log: RefCell<Vec<String>>,
    selection: Option<Selection>,
    /// The document-wide drag, threaded down so leaves know whether to
    /// report pointer moves as an extend.
    drag: Drag,
    /// Relative line height of body text; headings and code derive theirs
    /// from it (see `layout::typography`).
    body_ratio: f32,
    /// Font of body text, which selection hit-testing measures with when a leaf names none.
    body_font: iced::Font,
    /// The document's loaded images; without it every image shows its alt text.
    images: Option<&'a ImageStore>,
}

impl<'a> FindViewer<'a> {
    /// Draws the document's local images from `images`.
    pub fn with_images(mut self, images: &'a ImageStore) -> Self {
        self.images = Some(images);
        self
    }

    /// Sets the body line height, as a multiple of the font size.
    pub fn with_body_ratio(mut self, ratio: f32) -> Self {
        self.body_ratio = ratio;
        self
    }

    /// Sets the font body text is set in.
    pub fn with_font(mut self, font: iced::Font) -> Self {
        self.body_font = font;
        self
    }

    fn body_height(&self) -> LineHeight {
        LineHeight::Relative(self.body_ratio)
    }

    fn tight_height(&self) -> LineHeight {
        LineHeight::Relative(typography::heading_ratio(self.body_ratio))
    }

    /// Matches seen so far across the spans rendered through this viewer.
    pub fn count(&self) -> usize {
        self.count.get()
    }

    pub fn new(
        query: &'a str,
        active: Option<usize>,
        appearance: Appearance,
        item_index: usize,
        selection: Option<Selection>,
        drag: Drag,
    ) -> Self {
        Self {
            query,
            active,
            appearance,
            count: Cell::new(0),
            item_index,
            next_leaf: Cell::new(0),
            next_task: Cell::new(0),
            log: RefCell::new(Vec::new()),
            selection,
            drag,
            body_ratio: 1.3,
            body_font: iced::Font::DEFAULT,
            images: None,
        }
    }

    /// The plain text of every leaf rendered through this viewer so far, in
    /// render order, paired with the `BlockId` it was rendered under.
    pub fn leaf_texts(&self) -> Vec<(BlockId, String)> {
        self.log
            .borrow()
            .iter()
            .enumerate()
            .map(|(leaf, text)| {
                (
                    BlockId {
                        item: self.item_index,
                        leaf,
                    },
                    text.clone(),
                )
            })
            .collect()
    }

    /// Assigns the next selectable leaf's id and records its plain text,
    /// recovered from `styled` the same way `find::match_ranges` needs it:
    /// as the concatenation of every span's text.
    fn next_leaf(&self, visible: &str) -> BlockId {
        let leaf = self.next_leaf.get();
        self.next_leaf.set(leaf + 1);
        self.log.borrow_mut().push(visible.to_string());
        BlockId {
            item: self.item_index,
            leaf,
        }
    }

    fn spans(
        &self,
        content: &markdown::Text,
        style: markdown::Style,
    ) -> (
        BlockId,
        Vec<iced::widget::text::Span<'static, markdown::Uri>>,
    ) {
        let styled = content.spans(style);
        let visible = styled
            .iter()
            .map(|span| span.text.as_ref())
            .collect::<String>();
        let block = self.next_leaf(&visible);
        (block, self.sliced(block, &visible, styled.to_vec()))
    }

    /// Slices `styled` (spans whose concatenated text is `visible`) at find-match and selection
    /// boundaries; see `ui::syntax::slice`, which does the actual work and is shared with
    /// `ui::source_view`. Used by `spans`, above, for parsed Markdown spans, and `code_block`,
    /// below, for syntax-highlighted code-block lines, whose "source" spans carry a language's
    /// colors instead of a Markdown style's.
    fn sliced(
        &self,
        block: BlockId,
        visible: &str,
        styled: Vec<iced::widget::text::Span<'static, markdown::Uri>>,
    ) -> Vec<iced::widget::text::Span<'static, markdown::Uri>> {
        let selected = self
            .selection
            .and_then(|selection| selection.range_in(block, visible.len()));
        let mut count = self.count.get();
        let output = syntax::slice(
            styled,
            visible,
            self.query,
            self.active,
            &mut count,
            selected,
            self.appearance,
        );
        self.count.set(count);
        output
    }

    /// Wraps a rendered leaf so it participates in drag selection.
    fn selectable<M: From<SelectionEvent> + 'a>(
        &self,
        block: BlockId,
        spans: Vec<iced::widget::text::Span<'static, markdown::Uri>>,
        size: iced::Pixels,
        font: Option<iced::Font>,
        line_height: LineHeight,
        inner: impl Into<Element<'a, M>>,
    ) -> Element<'a, M> {
        SelectableText::new(
            inner,
            spans,
            block,
            size,
            font.or(Some(self.body_font)),
            self.drag,
            M::from,
        )
        .line_height(line_height)
        .into()
    }

    /// The marker beside a list item, in the accent color and sharing the
    /// body line height so it sits on the item's first baseline.
    fn marker(&self, label: String, size: Pixels) -> text::Text<'a> {
        text(label)
            .size(size)
            .line_height(self.body_height())
            .color(self.appearance.colors.accent)
    }

    /// Assigns the next checkbox's `TaskId`, in the same document order `domain::tasks` counts
    /// them in.
    fn next_task(&self) -> TaskId {
        let task = self.next_task.get();
        self.next_task.set(task + 1);
        TaskId {
            item: self.item_index,
            task,
        }
    }

    /// A task bullet's interactive checkbox: clicking it reports `TaskToggleEvent` rather than
    /// changing anything itself (`app::update::tasks` owns the actual toggle and write).
    fn task_checkbox<M: From<TaskToggleEvent> + 'a>(
        &self,
        done: bool,
        settings: markdown::Settings,
    ) -> Element<'a, M> {
        let id = self.next_task();
        container(
            checkbox(done)
                .size(settings.text_size)
                .on_toggle(move |_| M::from(TaskToggleEvent(id))),
        )
        .center_y(self.body_height().to_absolute(settings.text_size))
        .into()
    }

    /// A list's progress row, shown only when `bullets` holds at least one task (issue #68):
    /// the completed/total count, which doubles as the accessible text equivalent, beside a
    /// compact bar.
    fn progress_row<M: 'a>(
        &self,
        bullets: &[markdown::Bullet],
        size: Pixels,
    ) -> Option<Element<'a, M>> {
        let progress = tasks::progress_of(bullets)?;
        Some(
            row![
                text(format!("{}/{} completed", progress.done, progress.total))
                    .size(size * 0.85)
                    .color(self.appearance.colors.text_muted),
                progress_bar(0.0..=100.0, progress.percent())
                    .length(Length::Fixed(crate::layout::spacing::units(10.0)))
                    .girth(Length::Fixed(crate::layout::spacing::HALF)),
            ]
            .spacing(crate::layout::spacing::UNIT)
            .align_y(alignment::Vertical::Center)
            .into(),
        )
    }
}

impl<
    'a,
    M: From<markdown::Uri> + From<SelectionEvent> + From<ImageMenu> + From<TaskToggleEvent> + 'a,
> markdown::Viewer<'a, M> for FindViewer<'a>
{
    fn on_link_click(url: markdown::Uri) -> M {
        M::from(url)
    }

    fn heading(
        &self,
        settings: markdown::Settings,
        level: &'a markdown::HeadingLevel,
        content: &'a markdown::Text,
        index: usize,
    ) -> Element<'a, M> {
        let size = match level {
            markdown::HeadingLevel::H1 => settings.h1_size,
            markdown::HeadingLevel::H2 => settings.h2_size,
            markdown::HeadingLevel::H3 => settings.h3_size,
            markdown::HeadingLevel::H4 => settings.h4_size,
            markdown::HeadingLevel::H5 => settings.h5_size,
            markdown::HeadingLevel::H6 => settings.h6_size,
        };
        let (block, mut spans) = self.spans(content, settings.style);
        let accent = self.appearance.colors.accent;
        for span in &mut spans {
            span.color = span.color.or(Some(accent));
        }
        let height = self.tight_height();
        let inner = rich_text(spans.clone())
            .on_link_click(Self::on_link_click)
            .line_height(height)
            .size(size);
        container(self.selectable(block, spans, size, None, height, inner))
            .padding(Padding {
                top: if index > 0 {
                    settings.text_size.0 / 2.0
                } else {
                    0.0
                },
                ..Padding::ZERO
            })
            .into()
    }

    fn paragraph(&self, settings: markdown::Settings, content: &markdown::Text) -> Element<'a, M> {
        let (block, spans) = self.spans(content, settings.style);
        let inner = rich_text(spans.clone())
            .size(settings.text_size)
            .line_height(self.body_height())
            .on_link_click(Self::on_link_click);
        self.selectable(
            block,
            spans,
            settings.text_size,
            None,
            self.body_height(),
            inner,
        )
    }

    fn code_block(
        &self,
        settings: markdown::Settings,
        language: Option<&'a str>,
        code: &'a str,
        _lines: &'a [markdown::Text],
    ) -> Element<'a, M> {
        // A fence's info string can carry more than the language (e.g. `rust,ignore`); only the
        // first word names it. Replaces the Markdown widget's own built-in highlighting (always
        // `Theme::Base16Ocean`, regardless of our appearance) with one matched to it (issue #74).
        let token = language
            .and_then(|language| language.split(',').next())
            .filter(|language| !language.is_empty())
            .unwrap_or("txt");
        let highlighted = syntax::highlight_lines(
            token,
            code.lines(),
            self.appearance,
            settings.style.code_block_font,
            self.appearance.colors.text,
        );
        container(
            scrollable(
                container(column(code.lines().zip(highlighted).map(
                    |(line, styled)| {
                        let block = self.next_leaf(line);
                        let spans = self.sliced(block, line, styled);
                        let inner = rich_text(spans.clone())
                            .font(settings.style.code_block_font)
                            .size(settings.code_size)
                            .line_height(self.tight_height())
                            .on_link_click(Self::on_link_click);
                        self.selectable(
                            block,
                            spans,
                            settings.code_size,
                            Some(settings.style.code_block_font),
                            self.tight_height(),
                            inner,
                        )
                    },
                )))
                .padding(settings.code_size),
            )
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::default()
                    .width(settings.code_size / 2)
                    .scroller_width(settings.code_size / 2),
            )),
        )
        .width(Fill)
        .padding(settings.code_size / 4)
        .class(<iced::Theme as markdown::Catalog>::code_block())
        .into()
    }

    fn image(
        &self,
        settings: markdown::Settings,
        url: &'a markdown::Uri,
        _title: &'a str,
        alt: &markdown::Text,
    ) -> Element<'a, M> {
        // The alt text is a leaf whether or not it is shown, so leaf ids do not shift as images
        // load; find and copy still reach it.
        let (block, spans) = self.spans(alt, settings.style);
        let colors = self.appearance.colors;
        let radius = settings.spacing.0 / 2.0;
        let path = self.images.and_then(|images| images.path(url));
        // A right-click on a drawn image opens its own menu.
        let with_menu = |drawn: Element<'a, M>| -> Element<'a, M> {
            match path {
                Some(path) => {
                    let path = path.to_path_buf();
                    ContextArea::reporting(drawn, move || M::from(ImageMenu { path: path.clone() }))
                        .into()
                }
                None => drawn,
            }
        };
        match self.images.and_then(|images| images.slot(url)) {
            Some(Slot::Ready(Loaded::Raster(handle))) => with_menu(
                container(
                    image(handle.clone())
                        .content_fit(ContentFit::Contain)
                        .border_radius(radius),
                )
                .into(),
            ),
            Some(Slot::Ready(Loaded::Svg(handle))) => with_menu(
                container(
                    svg(handle.clone())
                        .width(Length::Shrink)
                        .height(Length::Shrink)
                        .content_fit(ContentFit::Contain),
                )
                .into(),
            ),
            Some(Slot::Loading) => container(
                text("Loading image…")
                    .size(settings.text_size)
                    .color(colors.text_muted),
            )
            .width(Fill)
            .center_x(Fill)
            .padding(settings.spacing.0 * 3.0)
            .class(<iced::Theme as markdown::Catalog>::code_block())
            .into(),
            failed => {
                let inner = rich_text(spans.clone())
                    .line_height(self.body_height())
                    .on_link_click(Self::on_link_click);
                let mut body = column![self.selectable(
                    block,
                    spans,
                    settings.text_size,
                    None,
                    self.body_height(),
                    inner,
                )];
                let reason = match failed {
                    Some(Slot::Failed(reason)) => reason.as_str(),
                    _ => "Only images inside the open folder are shown",
                };
                body = body.push(
                    text(reason)
                        .size(settings.text_size * 0.85)
                        .color(colors.text_muted),
                );
                container(body)
                    .padding(settings.spacing.0)
                    .class(<iced::Theme as markdown::Catalog>::code_block())
                    .into()
            }
        }
    }

    fn unordered_list(
        &self,
        settings: markdown::Settings,
        bullets: &'a [markdown::Bullet],
    ) -> Element<'a, M> {
        let mut list = column(bullets.iter().map(|bullet| {
            let marker: Element<'a, M> = match bullet {
                markdown::Bullet::Point { .. } => {
                    self.marker("•".to_string(), settings.text_size).into()
                }
                markdown::Bullet::Task { done, .. } => self.task_checkbox(*done, settings),
            };
            row![
                marker,
                markdown::view_with(
                    bullet_items(bullet),
                    markdown::Settings {
                        spacing: settings.spacing * 0.6,
                        ..settings
                    },
                    self,
                )
            ]
            .spacing(settings.spacing)
            .into()
        }))
        .spacing(settings.spacing * 0.75)
        .padding([0.0, settings.spacing.0]);
        if let Some(progress) = self.progress_row(bullets, settings.text_size) {
            list = list.push(progress);
        }
        list.into()
    }

    fn ordered_list(
        &self,
        settings: markdown::Settings,
        start: u64,
        bullets: &'a [markdown::Bullet],
    ) -> Element<'a, M> {
        let digits = ((start + bullets.len() as u64).max(1) as f32)
            .log10()
            .ceil();
        let marker_width = settings.text_size * ((digits / 2.0).ceil() + 1.0);
        let mut list = column(bullets.iter().enumerate().map(|(i, bullet)| {
            let marker: Element<'a, M> = match bullet {
                markdown::Bullet::Point { .. } => self
                    .marker(format!("{}.", i as u64 + start), settings.text_size)
                    .align_x(alignment::Horizontal::Right)
                    .width(marker_width)
                    .into(),
                markdown::Bullet::Task { done, .. } => {
                    container(self.task_checkbox(*done, settings))
                        .width(marker_width)
                        .align_x(alignment::Horizontal::Right)
                        .into()
                }
            };
            row![
                marker,
                markdown::view_with(
                    bullet_items(bullet),
                    markdown::Settings {
                        spacing: settings.spacing * 0.6,
                        ..settings
                    },
                    self,
                )
            ]
            .spacing(settings.spacing)
            .into()
        }))
        .spacing(settings.spacing * 0.75);
        if let Some(progress) = self.progress_row(bullets, settings.text_size) {
            list = list.push(progress);
        }
        list.into()
    }
}

fn bullet_items(bullet: &markdown::Bullet) -> &[markdown::Item] {
    match bullet {
        markdown::Bullet::Point { items } | markdown::Bullet::Task { items, .. } => items,
    }
}

/// Flattens `items` into the plain text of every selectable leaf, in render
/// order, for the clipboard copy to read back by `BlockId`. Runs the real
/// `Viewer` machinery once per item and discards the `Element`s, since a
/// `List`/`Quote`/`Table`'s nested leaves (table rows especially) have no
/// other way to be walked from outside `iced::widget::markdown`.
pub(crate) fn leaf_texts(
    items: &[markdown::Item],
    appearance: Appearance,
) -> Vec<(BlockId, String)> {
    let settings = markdown::Settings::with_style(markdown::Style::from_palette(
        crate::ui::theme::palette(appearance),
    ));
    items
        .iter()
        .enumerate()
        .flat_map(|(item_index, item)| {
            let viewer = FindViewer::new("", None, appearance, item_index, None, Drag::default());
            let _: Element<'_, crate::app::Message> =
                markdown::view_with([item], settings, &viewer);
            viewer.leaf_texts()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme;

    #[test]
    fn counts_rendered_matches_in_nested_markdown() {
        let source = "# Find\n\nA **fi**nd and FIND.\n\n- find in a list\n\n| Name |\n| --- |\n| find |\n\n```text\nfind\n```";
        let items: Vec<_> = markdown::parse(source).collect();
        let settings = markdown::Settings::with_style(markdown::Style::from_palette(
            theme::palette(Appearance::SLATE),
        ));
        let counts: Vec<_> = items
            .iter()
            .enumerate()
            .map(|(item_index, item)| {
                let viewer = FindViewer::new(
                    "find",
                    None,
                    Appearance::SLATE,
                    item_index,
                    None,
                    Drag::default(),
                );
                let _: Element<'_, crate::app::Message> =
                    markdown::view_with([item], settings, &viewer);
                viewer.count()
            })
            .collect();
        assert_eq!(counts.iter().sum::<usize>(), 6);
        assert_eq!(counts.iter().filter(|&&count| count > 0).count(), 5);
    }

    #[test]
    fn leaf_texts_cover_headings_paragraphs_lists_and_code() {
        let source = "# Title\n\nA paragraph.\n\n- one\n- two\n\n```text\nline one\nline two\n```";
        let items: Vec<_> = markdown::parse(source).collect();
        let leaves = leaf_texts(&items, Appearance::SLATE);
        let texts: Vec<&str> = leaves.iter().map(|(_, text)| text.as_str()).collect();
        assert_eq!(
            texts,
            vec![
                "Title",
                "A paragraph.",
                "one",
                "two",
                "line one",
                "line two"
            ]
        );
    }
}
