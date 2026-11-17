// SPDX-License-Identifier: GPL-3.0-or-later
//! Opens a context menu on right-click anywhere over its content.
//!
//! Children see the press first, so a more specific target inside (a link
//! in the preview, say) can open its own menu and capture the event; this
//! area only reports presses nothing inside claimed. The position it reports
//! is window-absolute, ready for placing a popup.

use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::{Element, Event, Length, Point, Rectangle, Size, Vector};

pub struct ContextArea<'a, Message> {
    content: Element<'a, Message>,
    on_open: Box<dyn Fn(Point) -> Message + 'a>,
    /// Whether the press is consumed. A reporting area leaves it for an outer area to open
    /// the menu, which matters inside a scrollable, where cursor positions are scroll-shifted.
    capture: bool,
}

impl<'a, Message> ContextArea<'a, Message> {
    pub fn new(
        content: impl Into<Element<'a, Message>>,
        on_open: impl Fn(Point) -> Message + 'a,
    ) -> Self {
        Self {
            content: content.into(),
            on_open: Box::new(on_open),
            capture: true,
        }
    }

    /// Publishes `message` on a right-click over its content without claiming the press, so
    /// the surrounding area still opens the menu at a window-correct position.
    pub fn reporting(
        content: impl Into<Element<'a, Message>>,
        message: impl Fn() -> Message + 'a,
    ) -> Self {
        Self {
            content: content.into(),
            on_open: Box::new(move |_| message()),
            capture: false,
        }
    }
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for ContextArea<'_, Message> {
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
        if shell.is_event_captured() {
            return;
        }
        if matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))
        ) && let Some(position) = cursor.position_over(layout.bounds())
        {
            shell.publish((self.on_open)(position));
            if self.capture {
                shell.capture_event();
            }
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

impl<'a, Message: 'a> From<ContextArea<'a, Message>> for Element<'a, Message> {
    fn from(area: ContextArea<'a, Message>) -> Self {
        Element::new(area)
    }
}
