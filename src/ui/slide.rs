// SPDX-License-Identifier: GPL-3.0-or-later
//! Two layout wrappers for animated chrome. Neither changes what its child draws; they only
//! change where it sits and how much room it takes.
//!
//! - [`SlideLayer`] places a card partway between off the window edge and in place. iced's own
//!   `pin` hands its child `max - position` as available space, so a card sliding toward the
//!   right edge is squeezed narrower as it goes and its text rewraps; this one always lays the
//!   child out at its natural size and only then moves it.
//! - [`Collapse`] reports a fraction of its child's width, so a control can shrink out of a row
//!   (and the controls beside it move smoothly) instead of vanishing and making them jump.

use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{
    Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
};
use iced::{Element, Event, Length, Point, Rectangle, Size, Vector};

/// The window edge a card slides to and from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Start,
    End,
}

/// Where a card's left edge sits, as `x` within a layer `layer_width` wide, when it is
/// `progress` of the way in (`0.0` just off its edge, `1.0` in place) and `card_width` wide.
pub fn slide_x(edge: Edge, layer_width: f32, card_width: f32, progress: f32) -> f32 {
    match edge {
        Edge::Start => -(1.0 - progress) * card_width,
        Edge::End => layer_width - progress * card_width,
    }
}

/// A full-size layer holding one card at its slid position.
pub struct SlideLayer<'a, Message> {
    content: Element<'a, Message>,
    edge: Edge,
    progress: f32,
}

impl<'a, Message> SlideLayer<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>, edge: Edge, progress: f32) -> Self {
        Self {
            content: content.into(),
            edge,
            progress: progress.clamp(0.0, 1.0),
        }
    }
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for SlideLayer<'_, Message> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let size = limits.width(Length::Fill).height(Length::Fill).max();
        // The card gets the whole layer to size itself in, wherever it is moved to afterwards.
        let card = self.content.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &layout::Limits::new(Size::ZERO, size),
        );
        let x = slide_x(self.edge, size.width, card.size().width, self.progress);
        layout::Node::with_children(size, vec![card.move_to(Point::new(x, 0.0))])
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            only_child(layout),
            renderer,
            operation,
        );
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
            only_child(layout),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
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
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            only_child(layout),
            cursor,
            viewport,
            renderer,
        )
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
        let Some(visible) = layout.bounds().intersection(viewport) else {
            return;
        };
        renderer.with_layer(visible, |renderer| {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                only_child(layout),
                cursor,
                &visible,
            );
        });
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
            only_child(layout),
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: 'a> From<SlideLayer<'a, Message>> for Element<'a, Message> {
    fn from(layer: SlideLayer<'a, Message>) -> Self {
        Element::new(layer)
    }
}

/// Takes `factor` of its child's natural width and clips the rest. At `1.0` it is the child
/// exactly; below that the child is inert (a control mid-collapse is not meant to be clicked).
pub struct Collapse<'a, Message> {
    content: Element<'a, Message>,
    factor: f32,
}

impl<'a, Message> Collapse<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>, factor: f32) -> Self {
        Self {
            content: content.into(),
            factor: factor.clamp(0.0, 1.0),
        }
    }

    fn whole(&self) -> bool {
        self.factor >= 1.0
    }
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for Collapse<'_, Message> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, self.content.as_widget().size().height)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let child =
            self.content
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, &limits.loose());
        let size = Size::new(child.size().width * self.factor, child.size().height);
        layout::Node::with_children(size, vec![child])
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            only_child(layout),
            renderer,
            operation,
        );
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
        if !self.whole() {
            return;
        }
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            only_child(layout),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
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
        if !self.whole() {
            return mouse::Interaction::None;
        }
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            only_child(layout),
            cursor,
            viewport,
            renderer,
        )
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
        let child = only_child(layout);
        if self.whole() {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                child,
                cursor,
                viewport,
            );
        } else if let Some(visible) = layout.bounds().intersection(viewport) {
            renderer.with_layer(visible, |renderer| {
                self.content.as_widget().draw(
                    &tree.children[0],
                    renderer,
                    theme,
                    style,
                    child,
                    mouse::Cursor::Unavailable,
                    &visible,
                );
            });
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, iced::Theme, iced::Renderer>> {
        if !self.whole() {
            return None;
        }
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            only_child(layout),
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: 'a> From<Collapse<'a, Message>> for Element<'a, Message> {
    fn from(collapse: Collapse<'a, Message>) -> Self {
        Element::new(collapse)
    }
}

/// The one child a wrapper widget lays out; its `layout` always builds exactly one.
#[allow(clippy::expect_used)]
fn only_child(layout: Layout<'_>) -> Layout<'_> {
    layout
        .children()
        .next()
        .expect("a slide widget always lays out one child")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_start_card_enters_from_beyond_the_left_edge() {
        assert_eq!(slide_x(Edge::Start, 1000.0, 300.0, 0.0), -300.0);
        assert_eq!(slide_x(Edge::Start, 1000.0, 300.0, 0.5), -150.0);
        assert_eq!(slide_x(Edge::Start, 1000.0, 300.0, 1.0), 0.0);
    }

    #[test]
    fn an_end_card_enters_from_beyond_the_right_edge() {
        assert_eq!(slide_x(Edge::End, 1000.0, 300.0, 0.0), 1000.0);
        assert_eq!(slide_x(Edge::End, 1000.0, 300.0, 0.5), 850.0);
        assert_eq!(slide_x(Edge::End, 1000.0, 300.0, 1.0), 700.0);
    }

    #[test]
    fn the_end_position_depends_on_the_layer_not_the_card() {
        // A wider card at rest still ends flush with the right edge, whatever the window.
        for width in [320.0, 900.0] {
            assert_eq!(slide_x(Edge::End, width, 300.0, 1.0) + 300.0, width);
        }
    }
}
