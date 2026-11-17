// SPDX-License-Identifier: GPL-3.0-or-later
//! Reports when a text field inside it gains or loses focus, and can claim Escape. iced's `text_input` publishes no
//! focus event, so after each event this asks the field itself (through a widget `Operation`)
//! and publishes only when the answer differs from `focused`, the state the app last recorded.

use iced::advanced::widget::operation::{Focusable, Outcome};
use iced::advanced::widget::{Id, Operation, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Size, Vector};

/// Records whether the first focusable widget it meets is focused.
#[derive(Default)]
struct Probe {
    focused: Option<bool>,
}

impl Operation for Probe {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
        self.focused.get_or_insert(state.is_focused());
    }

    fn finish(&self) -> Outcome<()> {
        Outcome::None
    }
}

pub struct FocusWatch<'a, Message> {
    content: Element<'a, Message>,
    focused: bool,
    on_change: Box<dyn Fn(bool) -> Message + 'a>,
    on_escape: Option<Box<dyn Fn() -> Message + 'a>>,
}

impl<'a, Message> FocusWatch<'a, Message> {
    pub fn new(
        content: impl Into<Element<'a, Message>>,
        focused: bool,
        on_change: impl Fn(bool) -> Message + 'a,
    ) -> Self {
        Self {
            content: content.into(),
            focused,
            on_change: Box::new(on_change),
            on_escape: None,
        }
    }

    /// While the field is focused, Escape publishes `message` instead of reaching it, so the
    /// field keeps its focus. (Left alone, a `text_input` blurs itself on Escape.) Pass this only
    /// when there is something for Escape to do first, such as text to clear; with no handler,
    /// Escape passes through and blurs the field as usual.
    pub fn on_escape(mut self, message: impl Fn() -> Message + 'a) -> Self {
        self.on_escape = Some(Box::new(message));
        self
    }

    fn probe(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
    ) -> Option<bool> {
        let mut probe = Probe::default();
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, &mut probe);
        probe.focused
    }
}

/// Whether `event` is an Escape key press.
fn is_escape(event: &Event) -> bool {
    matches!(
        event,
        Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
            ..
        })
    )
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for FocusWatch<'_, Message> {
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
        if self.on_escape.is_some()
            && is_escape(event)
            && self.probe(tree, layout, renderer) == Some(true)
            && let Some(on_escape) = &self.on_escape
        {
            shell.publish(on_escape());
            shell.capture_event();
            return;
        }
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
        if let Some(now) = self.probe(tree, layout, renderer)
            && now != self.focused
        {
            shell.publish((self.on_change)(now));
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

impl<'a, Message: 'a> From<FocusWatch<'a, Message>> for Element<'a, Message> {
    fn from(watch: FocusWatch<'a, Message>) -> Self {
        Element::new(watch)
    }
}
