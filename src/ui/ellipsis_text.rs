// SPDX-License-Identifier: GPL-3.0-or-later
//! Single-line text that ends in an ellipsis when it does not fit its width.
//!
//! `iced` clips or wraps overflowing text but cannot elide it, so this widget
//! shortens the string during layout using the width it is actually given.

use iced::advanced::text::{self as adv_text, Renderer as _};
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::widget::{text as core_text, tooltip};
use iced::{Color, Element, Event, Length, Pixels, Rectangle, Size, Vector};
use std::time::Duration;

type Paragraph = <iced::Renderer as adv_text::Renderer>::Paragraph;

const ELLIPSIS: char = '…';

/// Shortens `text` to at most `max_chars` characters by cutting out its middle and
/// splicing in an ellipsis, keeping the start and end (where a name's distinguishing
/// part usually is) rather than iced's default end truncation. Used for fixed-size
/// chrome, such as a tooltip line, that cannot measure the text it is given the way
/// [`EllipsisText`] does.
pub(crate) fn truncate_middle(text: &str, max_chars: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_chars || max_chars < 3 {
        return text.to_string();
    }
    let keep = max_chars - 1;
    let head = keep.div_ceil(2);
    let tail = keep - head;
    let mut result: String = chars[..head].iter().collect();
    result.push(ELLIPSIS);
    result.extend(&chars[chars.len() - tail..]);
    result
}

pub struct EllipsisText {
    content: String,
    format: core_text::Format<iced::Font>,
    color: Option<Color>,
}

impl EllipsisText {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            format: core_text::Format {
                width: Length::Fill,
                wrapping: adv_text::Wrapping::None,
                ..core_text::Format::default()
            },
            color: None,
        }
    }

    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.format.size = Some(size.into());
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn font(mut self, font: iced::Font) -> Self {
        self.format.font = Some(font);
        self
    }

    /// Takes the text's own width (up to what it is offered) instead of all of it.
    pub fn shrink(mut self) -> Self {
        self.format.width = Length::Shrink;
        self
    }
}

#[derive(Default)]
struct State {
    shown: core_text::State<Paragraph>,
    scratch: core_text::State<Paragraph>,
    /// The width and content `shown_text` was fitted for.
    fitted: Option<(f32, String)>,
    shown_text: String,
    truncated: bool,
}

impl State {
    /// The text to display: `content` whole when it fits `available`,
    /// otherwise its longest prefix that still fits once `…` is appended.
    fn fit(
        &mut self,
        renderer: &iced::Renderer,
        content: &str,
        format: core_text::Format<iced::Font>,
        available: f32,
    ) -> &str {
        if self
            .fitted
            .as_ref()
            .is_some_and(|(width, cached)| *width == available && cached == content)
        {
            return &self.shown_text;
        }
        let size = format.size.unwrap_or_else(|| renderer.default_size());
        let font = format.font.unwrap_or_else(|| renderer.default_font());
        let mut width_of = |candidate: &str| {
            let _ = self.scratch.update(adv_text::Text {
                content: candidate,
                bounds: Size::INFINITE,
                size,
                line_height: format.line_height,
                font,
                align_x: format.align_x,
                align_y: format.align_y,
                shaping: format.shaping,
                wrapping: adv_text::Wrapping::None,
            });
            self.scratch.min_width()
        };
        let shown = if !available.is_finite() || width_of(content) <= available {
            content.to_owned()
        } else {
            let ends: Vec<usize> = content
                .char_indices()
                .map(|(index, _)| index)
                .chain(std::iter::once(content.len()))
                .collect();
            // Largest prefix length whose elided form fits; zero keeps just `…`.
            let (mut low, mut high) = (0, ends.len() - 1);
            while low < high {
                let mid = (low + high).div_ceil(2);
                let candidate = format!("{}{ELLIPSIS}", &content[..ends[mid]]);
                if width_of(&candidate) <= available {
                    low = mid;
                } else {
                    high = mid - 1;
                }
            }
            format!("{}{ELLIPSIS}", content[..ends[low]].trim_end())
        };
        self.truncated = shown != content;
        self.shown_text = shown;
        self.fitted = Some((available, content.to_owned()));
        &self.shown_text
    }
}

impl<Message, Theme> Widget<Message, Theme, iced::Renderer> for EllipsisText {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.format.width,
            height: self.format.height,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<State>();
        let available = limits.max().width;
        let shown = state
            .fit(renderer, &self.content, self.format, available)
            .to_owned();
        core_text::layout(&mut state.shown, renderer, limits, &shown, self.format)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &Theme,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        core_text::draw(
            renderer,
            defaults,
            layout.bounds(),
            state.shown.raw(),
            core_text::Style { color: self.color },
            viewport,
        );
    }
}

impl<'a, Message: 'a, Theme: 'a> From<EllipsisText>
    for Element<'a, Message, Theme, iced::Renderer>
{
    fn from(text: EllipsisText) -> Self {
        Element::new(text)
    }
}

/// Pause before the full text appears, so sweeping the pointer down the list stays quiet.
const TOOLTIP_DELAY: Duration = Duration::from_millis(500);

/// An [`EllipsisText`] that shows `full` in a tooltip, but only while it is elided.
pub struct ElidedTooltip<'a, Message> {
    inner: Element<'a, Message>,
}

impl<'a, Message: 'a> ElidedTooltip<'a, Message> {
    pub fn new(text: EllipsisText, full: impl Into<Element<'a, Message>>) -> Self {
        let inner = tooltip(text, full, tooltip::Position::Bottom)
            .gap(4)
            .delay(TOOLTIP_DELAY)
            .into();
        Self { inner }
    }
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for ElidedTooltip<'_, Message> {
    fn tag(&self) -> tree::Tag {
        self.inner.as_widget().tag()
    }
    fn state(&self) -> tree::State {
        self.inner.as_widget().state()
    }
    fn children(&self) -> Vec<Tree> {
        self.inner.as_widget().children()
    }
    fn diff(&self, tree: &mut Tree) {
        self.inner.as_widget().diff(tree);
    }
    fn size(&self) -> Size<Length> {
        self.inner.as_widget().size()
    }
    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.inner.as_widget_mut().layout(tree, renderer, limits)
    }
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.inner
            .as_widget_mut()
            .operate(tree, layout, renderer, operation);
    }
    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.inner.as_widget_mut().update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
    }
    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.inner
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }
    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.inner
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }
    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, iced::Theme, iced::Renderer>> {
        // The tooltip's tree holds the elided text as its first child.
        let truncated = tree
            .children
            .first()
            .is_some_and(|text| text.state.downcast_ref::<State>().truncated);
        if !truncated {
            return None;
        }
        self.inner
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
    }
}

impl<'a, Message: 'a> From<ElidedTooltip<'a, Message>> for Element<'a, Message> {
    fn from(tooltip: ElidedTooltip<'a, Message>) -> Self {
        Element::new(tooltip)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_middle_leaves_short_text_alone() {
        assert_eq!(truncate_middle("Source Code Pro", 20), "Source Code Pro");
    }

    #[test]
    fn truncate_middle_cuts_out_the_center() {
        let truncated = truncate_middle("Source Sans Three Narrow Condensed", 12);
        assert_eq!(truncated.chars().count(), 12);
        assert!(truncated.starts_with("Source"));
        assert!(truncated.ends_with("ensed"));
        assert!(truncated.contains(ELLIPSIS));
    }
}
