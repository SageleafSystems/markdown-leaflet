// SPDX-License-Identifier: GPL-3.0-or-later
//! The toolbar's title and search field, laid out together in the one slot between the left
//! cluster and the right actions. The title sits at the slot's start and the search at its end.
//! At rest the search keeps its width and the title takes what is left (up to a maximum); as
//! `expand` goes from 0 to 1 the title shrinks to a third of its width and the search grows into
//! the room it leaves, so the two share one slot and can never overlap or leave a jump.

use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{
    Clipboard, Layout, Shell, Widget, layout, mouse, overlay as adv_overlay, renderer,
};
use iced::{Element, Event, Length, Point, Rectangle, Size, Vector};

/// How much of its width the title keeps when the search is fully expanded.
pub const TITLE_SHRUNK_TO: f32 = 1.0 / 3.0;

/// The two widths for a slot `available` wide.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Split {
    pub title: f32,
    pub search: f32,
}

/// Splits `available` between a title whose natural width is `title_natural` (capped at
/// `title_max`) and a search field that rests at `search_rest`, `gap` apart, with the search
/// `expand` of the way to filling everything the shrinking title leaves.
pub fn split(
    available: f32,
    title_natural: f32,
    expand: f32,
    search_rest: f32,
    title_max: f32,
    gap: f32,
) -> Split {
    let expand = expand.clamp(0.0, 1.0);
    let rest = search_rest.min(available).max(0.0);
    // At rest the title may take only what the search leaves it.
    let room = (available - rest - gap).max(0.0);
    let full = title_natural.min(title_max).min(room);
    let title = full * (1.0 - (1.0 - TITLE_SHRUNK_TO) * expand);
    let gap = if title > 0.0 { gap } else { 0.0 };
    let grown = (available - title - gap).max(rest);
    Split {
        title,
        search: rest + (grown - rest) * expand,
    }
}

pub struct TitleSearch<'a, Message> {
    /// `[title, search]`.
    children: [Element<'a, Message>; 2],
    expand: f32,
    search_rest: f32,
    title_max: f32,
    gap: f32,
}

impl<'a, Message> TitleSearch<'a, Message> {
    pub fn new(
        title: impl Into<Element<'a, Message>>,
        search: impl Into<Element<'a, Message>>,
        expand: f32,
        search_rest: f32,
        title_max: f32,
        gap: f32,
    ) -> Self {
        Self {
            children: [title.into(), search.into()],
            expand,
            search_rest,
            title_max,
            gap,
        }
    }
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for TitleSearch<'_, Message> {
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let max = limits.max();
        let available = if max.width.is_finite() {
            max.width
        } else {
            self.search_rest + self.title_max + self.gap
        };
        let [title, search] = &mut self.children;
        let (title_tree, search_tree) = tree.children.split_at_mut(1);

        // The title's natural width is what it takes when offered the most it could use.
        let offered = Size::new(self.title_max.min(available), max.height);
        let natural = title
            .as_widget_mut()
            .layout(
                &mut title_tree[0],
                renderer,
                &layout::Limits::new(Size::ZERO, offered),
            )
            .size()
            .width;
        let widths = split(
            available,
            natural,
            self.expand,
            self.search_rest,
            self.title_max,
            self.gap,
        );
        let title_node = title.as_widget_mut().layout(
            &mut title_tree[0],
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(widths.title, max.height)),
        );
        let search_node = search.as_widget_mut().layout(
            &mut search_tree[0],
            renderer,
            &layout::Limits::new(
                Size::new(widths.search, 0.0),
                Size::new(widths.search, max.height),
            ),
        );

        let height = title_node.size().height.max(search_node.size().height);
        let centered = |node: layout::Node, x: f32| {
            let y = (height - node.size().height) / 2.0;
            node.move_to(Point::new(x, y))
        };
        let search_x = available - search_node.size().width;
        layout::Node::with_children(
            Size::new(available, height),
            vec![centered(title_node, 0.0), centered(search_node, search_x)],
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        for ((child, state), layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child
                .as_widget_mut()
                .operate(state, layout, renderer, operation);
        }
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
        for ((child, state), layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child.as_widget_mut().update(
                state, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
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
        self.children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((child, state), layout)| {
                child
                    .as_widget()
                    .mouse_interaction(state, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
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
        for ((child, state), layout) in self
            .children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
        {
            child
                .as_widget()
                .draw(state, renderer, theme, style, layout, cursor, viewport);
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<adv_overlay::Element<'b, Message, iced::Theme, iced::Renderer>> {
        adv_overlay::from_children(
            &mut self.children,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: 'a> From<TitleSearch<'a, Message>> for Element<'a, Message> {
    fn from(widget: TitleSearch<'a, Message>) -> Self {
        Element::new(widget)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REST: f32 = 220.0;
    const MAX: f32 = 240.0;
    const GAP: f32 = 8.0;

    fn at(available: f32, natural: f32, expand: f32) -> Split {
        split(available, natural, expand, REST, MAX, GAP)
    }

    #[test]
    fn at_rest_the_search_keeps_its_width_and_the_title_its_own() {
        let split = at(800.0, 150.0, 0.0);
        assert_eq!(split.search, REST);
        assert_eq!(split.title, 150.0);
    }

    #[test]
    fn a_long_title_stops_at_its_maximum() {
        assert_eq!(at(900.0, 600.0, 0.0).title, MAX);
    }

    #[test]
    fn at_rest_a_title_never_pushes_the_search_out_of_the_slot() {
        // Only 300 left: the search keeps 220 and the title gets what remains after the gap.
        let split = at(300.0, 240.0, 0.0);
        assert_eq!(split.search, REST);
        assert_eq!(split.title, 300.0 - REST - GAP);
        assert!(split.title + GAP + split.search <= 300.0);
    }

    #[test]
    fn expanding_shrinks_the_title_to_a_third_and_the_search_fills_the_rest() {
        let split = at(800.0, 240.0, 1.0);
        assert!((split.title - 240.0 * TITLE_SHRUNK_TO).abs() < 1e-3);
        assert!((split.search - (800.0 - split.title - GAP)).abs() < 1e-3);
    }

    #[test]
    fn the_two_never_overlap_at_any_point_of_the_animation() {
        for available in [260.0, 400.0, 800.0, 1400.0] {
            for step in 0..=20 {
                let expand = step as f32 / 20.0;
                let split = at(available, 240.0, expand);
                let gap = if split.title > 0.0 { GAP } else { 0.0 };
                assert!(
                    split.title + gap + split.search <= available + 1e-3,
                    "{available} at {expand}: {split:?}"
                );
                assert!(split.search >= REST.min(available));
            }
        }
    }

    #[test]
    fn the_search_grows_and_the_title_shrinks_monotonically() {
        let mut last = at(800.0, 240.0, 0.0);
        for step in 1..=20 {
            let split = at(800.0, 240.0, step as f32 / 20.0);
            assert!(split.search >= last.search);
            assert!(split.title <= last.title);
            last = split;
        }
    }

    #[test]
    fn with_no_title_the_search_takes_the_whole_slot_without_a_stray_gap() {
        let split = at(800.0, 0.0, 1.0);
        assert_eq!(split.title, 0.0);
        assert_eq!(split.search, 800.0);
        assert_eq!(at(800.0, 0.0, 0.0).search, REST);
    }

    #[test]
    fn a_slot_narrower_than_the_search_just_gives_it_everything() {
        let split = at(150.0, 200.0, 0.0);
        assert_eq!(split.search, 150.0);
        assert_eq!(split.title, 0.0);
    }
}
