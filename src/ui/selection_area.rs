// SPDX-License-Identifier: GPL-3.0-or-later
//! Wraps the preview's scrollable with the selection behavior that spans
//! the whole viewport rather than one text leaf: a press on blank space
//! clears the selection, and a drag held near (or past) the top or bottom
//! edge scrolls the preview.
//!
//! A press on text is captured by its `SelectableText` leaf first, so an
//! uncaptured press here means it landed between or beside the text.

use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Size, Vector};

/// How far inside the edge the auto-scroll band reaches.
const EDGE_BAND: f32 = 32.0;
/// Pixels per frame at the edge; it ramps up from zero across the band, and
/// keeps growing past the edge up to [`MAX_SPEED`].
const EDGE_SPEED: f32 = 12.0;
const MAX_SPEED: f32 = 40.0;

pub(crate) struct SelectionArea<'a, Message> {
    content: Element<'a, Message>,
    selecting: bool,
    on_blank_press: Message,
    on_autoscroll: Box<dyn Fn(f32) -> Message + 'a>,
}

impl<'a, Message> SelectionArea<'a, Message> {
    pub(crate) fn new(
        content: impl Into<Element<'a, Message>>,
        selecting: bool,
        on_blank_press: Message,
        on_autoscroll: impl Fn(f32) -> Message + 'a,
    ) -> Self {
        Self {
            content: content.into(),
            selecting,
            on_blank_press,
            on_autoscroll: Box::new(on_autoscroll),
        }
    }
}

/// Pixels per frame to scroll with the pointer at `y`, for a viewport
/// spanning `top..bottom`: negative scrolls up, 0 outside the edge bands.
fn edge_speed(y: f32, top: f32, bottom: f32) -> f32 {
    let ramp = |depth: f32| (depth / EDGE_BAND * EDGE_SPEED).min(MAX_SPEED);
    if y < top + EDGE_BAND {
        -ramp(top + EDGE_BAND - y)
    } else if y > bottom - EDGE_BAND {
        ramp(y - (bottom - EDGE_BAND))
    } else {
        0.0
    }
}

#[derive(Default)]
struct State {
    /// The speed last reported, so a move within the same speed stays quiet.
    speed: f32,
}

impl<Message: Clone> Widget<Message, iced::Theme, iced::Renderer> for SelectionArea<'_, Message> {
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
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
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
        let bounds = layout.bounds();
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if !shell.is_event_captured() && cursor.is_over(bounds) =>
            {
                shell.publish(self.on_blank_press.clone());
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) if self.selecting => {
                let speed = edge_speed(position.y, bounds.y, bounds.y + bounds.height);
                if speed != state.speed {
                    state.speed = speed;
                    shell.publish((self.on_autoscroll)(speed));
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.speed = 0.0;
            }
            _ => {}
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
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
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

impl<'a, Message: Clone + 'a> From<SelectionArea<'a, Message>> for Element<'a, Message> {
    fn from(area: SelectionArea<'a, Message>) -> Self {
        Element::new(area)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_is_zero_away_from_the_edges() {
        assert_eq!(edge_speed(300.0, 100.0, 600.0), 0.0);
    }

    #[test]
    fn speed_ramps_up_toward_and_past_each_edge() {
        let near_top = edge_speed(120.0, 100.0, 600.0);
        let at_top = edge_speed(100.0, 100.0, 600.0);
        let past_top = edge_speed(40.0, 100.0, 600.0);
        assert!(near_top < 0.0 && at_top < near_top && past_top < at_top);
        assert_eq!(at_top, -EDGE_SPEED);
        assert!(edge_speed(590.0, 100.0, 600.0) > 0.0);
        assert_eq!(edge_speed(5000.0, 100.0, 600.0), MAX_SPEED);
    }
}
