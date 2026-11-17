// SPDX-License-Identifier: GPL-3.0-or-later
//! Wraps one rendered `rich_text` span run with mouse-driven text selection.
//!
//! `rich_text`'s own `Paragraph` is private, so hit-testing a drag gesture
//! into a character offset needs a second `Paragraph`, built from the same
//! spans, kept purely for `hit_test` (mirroring how `iced_widget`'s `Rich`
//! widget builds and caches its own). Painting the highlight is *not* this
//! widget's job: the caller (`ui::find_viewer`) slices the span backgrounds
//! before the inner `rich_text` ever sees them, the same way find-match
//! highlighting already works, so a plain `rich_text` renders it for free.
//!
//! Structurally this is `ui::press_area::PressArea` with an extra shadow
//! paragraph: layout, draw, children, operate, and overlay all delegate to
//! the wrapped `content`.

use crate::domain::selection::{BlockId, word_at};
use iced::advanced::text::{self as adv_text, Paragraph as _, Renderer as _};
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::widget::markdown::Uri;
use iced::{Element, Event, Length, Pixels, Rectangle, Size, Vector, keyboard, window};

type Paragraph = <iced::Renderer as adv_text::Renderer>::Paragraph;

/// A selection gesture reported by a [`SelectableText`] leaf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SelectionEvent {
    /// The pointer was pressed at this character offset within `BlockId`.
    Start(BlockId, usize),
    /// The pointer, already dragging, moved over this character offset.
    Extend(BlockId, usize),
    /// Shift+click: move the selection's head here, keeping its anchor.
    ExtendTo(BlockId, usize),
    /// Double- or triple-click selected this byte range of the leaf.
    Range(BlockId, usize, usize),
    /// The mouse button was released.
    End,
    /// The right button was pressed over this link; a context menu opens next.
    ContextLink(Uri),
}

/// The document-wide drag, threaded into every leaf.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Drag {
    /// The button is down after pressing on text: pointer moves extend.
    pub(crate) selecting: bool,
    /// The preview is auto-scrolling under a stationary pointer, so the head
    /// is re-resolved every frame.
    pub(crate) autoscrolling: bool,
}

pub(crate) struct SelectableText<'a, Message> {
    content: Element<'a, Message>,
    spans: Vec<adv_text::Span<'static, Uri>>,
    block: BlockId,
    size: Pixels,
    font: Option<iced::Font>,
    line_height: adv_text::LineHeight,
    drag: Drag,
    on_event: Box<dyn Fn(SelectionEvent) -> Message + 'a>,
}

impl<'a, Message> SelectableText<'a, Message> {
    pub(crate) fn new(
        content: impl Into<Element<'a, Message>>,
        spans: Vec<adv_text::Span<'static, Uri>>,
        block: BlockId,
        size: Pixels,
        font: Option<iced::Font>,
        drag: Drag,
        on_event: impl Fn(SelectionEvent) -> Message + 'a,
    ) -> Self {
        Self {
            content: content.into(),
            spans,
            block,
            size,
            font,
            line_height: adv_text::LineHeight::default(),
            drag,
            on_event: Box::new(on_event),
        }
    }

    /// Sets the line height; it must match the wrapped text's so hit-testing
    /// lands on the line the user sees.
    pub(crate) fn line_height(mut self, line_height: adv_text::LineHeight) -> Self {
        self.line_height = line_height;
        self
    }

    fn text(
        &self,
        renderer: &iced::Renderer,
        bounds: Size,
    ) -> adv_text::Text<&[adv_text::Span<'static, Uri>]> {
        adv_text::Text {
            content: self.spans.as_slice(),
            bounds,
            size: self.size,
            line_height: self.line_height,
            font: self.font.unwrap_or_else(|| renderer.default_font()),
            align_x: adv_text::Alignment::Default,
            align_y: iced::alignment::Vertical::Top,
            shaping: adv_text::Shaping::Advanced,
            wrapping: adv_text::Wrapping::default(),
        }
    }

    fn plain_text(&self) -> String {
        self.spans.iter().map(|span| span.text.as_ref()).collect()
    }
}

#[derive(Default)]
struct State {
    spans: Vec<adv_text::Span<'static, Uri>>,
    paragraph: Paragraph,
    modifiers: keyboard::Modifiers,
    /// The last left press, for counting double and triple clicks.
    last_click: Option<mouse::Click>,
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for SelectableText<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let node = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);

        let state = tree.state.downcast_mut::<State>();
        let bounds = limits.max();
        let text = self.text(renderer, bounds);

        if state.spans == self.spans {
            match state.paragraph.compare(adv_text::Text {
                content: (),
                bounds,
                size: text.size,
                line_height: text.line_height,
                font: text.font,
                align_x: text.align_x,
                align_y: text.align_y,
                shaping: text.shaping,
                wrapping: text.wrapping,
            }) {
                adv_text::Difference::None => {}
                adv_text::Difference::Bounds => state.paragraph.resize(bounds),
                adv_text::Difference::Shape => state.paragraph = Paragraph::with_spans(text),
            }
        } else {
            state.paragraph = Paragraph::with_spans(text);
            state.spans.clone_from(&self.spans);
        }

        node
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
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
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let state = tree.state.downcast_mut::<State>();
        let hit = |cursor: mouse::Cursor| {
            let position = cursor.position_in(layout.bounds())?;
            let adv_text::Hit::CharOffset(offset) = state.paragraph.hit_test(position)?;
            Some((position, offset))
        };
        // Outside the scrollable the cursor arrives levitating; while dragging,
        // land it so the head follows the pointer above or below the viewport.
        let dragged = cursor.land();
        let event = match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.modifiers = *modifiers;
                None
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                hit(cursor).map(|(position, offset)| {
                    let click = mouse::Click::new(position, mouse::Button::Left, state.last_click);
                    state.last_click = Some(click);
                    shell.capture_event();
                    if state.modifiers.shift() {
                        return SelectionEvent::ExtendTo(self.block, offset);
                    }
                    match click.kind() {
                        mouse::click::Kind::Single => SelectionEvent::Start(self.block, offset),
                        mouse::click::Kind::Double => {
                            let word = word_at(&self.plain_text(), offset);
                            SelectionEvent::Range(self.block, word.start, word.end)
                        }
                        mouse::click::Kind::Triple => {
                            SelectionEvent::Range(self.block, 0, self.plain_text().len())
                        }
                    }
                })
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) if self.drag.selecting => {
                hit(dragged).map(|(_, offset)| SelectionEvent::Extend(self.block, offset))
            }
            Event::Window(window::Event::RedrawRequested(_)) if self.drag.autoscrolling => {
                hit(dragged).map(|(_, offset)| SelectionEvent::Extend(self.block, offset))
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if self.drag.selecting =>
            {
                Some(SelectionEvent::End)
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => hit(cursor)
                .and_then(|(position, _)| {
                    let index = state.paragraph.hit_span(position)?;
                    Some(SelectionEvent::ContextLink(
                        self.spans.get(index)?.link.clone()?,
                    ))
                }),
            _ => None,
        };
        if let Some(event) = event {
            shell.publish((self.on_event)(event));
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let inner = self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        );
        if matches!(inner, mouse::Interaction::None) && cursor.is_over(layout.bounds()) {
            mouse::Interaction::Text
        } else {
            inner
        }
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
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, iced::Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: 'a> From<SelectableText<'a, Message>> for Element<'a, Message> {
    fn from(text: SelectableText<'a, Message>) -> Self {
        Element::new(text)
    }
}
