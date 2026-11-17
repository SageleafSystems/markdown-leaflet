// SPDX-License-Identifier: GPL-3.0-or-later
//! The minimap strip: one thin bar per rendered block and a slider over the
//! part of the document in view. Geometry comes from `domain::minimap`.

use crate::domain::minimap::{Bar, BlockKind, LINE_PITCH, Marks, line_count, line_length};
use crate::ui::theme::Colors;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::{Border, Color, Element, Event, Length, Rectangle, Shadow, Size, Vector, mouse};

/// Width of the strip, in pixels.
pub const WIDTH: f32 = 72.0;

/// Width of the preview's scrollbar, which the strip sits just inside.
pub const SCROLLBAR_WIDTH: f32 = 10.0;

/// Gap kept at the strip's edges so bars do not touch the scrollbar or the pane.
const MARGIN: f32 = 6.0;

pub struct Minimap<'a, Message> {
    bars: Vec<Bar>,
    slider: (f32, f32),
    colors: Colors,
    marks: Marks,
    /// Called with a y in the strip's own coordinates when the reader presses
    /// or drags on it.
    on_seek: Box<dyn Fn(f32) -> Message + 'a>,
}

/// Whether a press on the strip is still being dragged.
#[derive(Default)]
struct Dragging(bool);

impl<'a, Message> Minimap<'a, Message> {
    pub fn new(
        bars: Vec<Bar>,
        slider: (f32, f32),
        colors: Colors,
        on_seek: impl Fn(f32) -> Message + 'a,
    ) -> Self {
        Self {
            bars,
            slider,
            colors,
            marks: Marks::default(),
            on_seek: Box::new(on_seek),
        }
    }

    /// Highlights find matches and the heading the reader is in.
    pub fn with_marks(mut self, marks: Marks) -> Self {
        self.marks = marks;
        self
    }

    /// Marks a block that holds a find match: a tick in the strip's right margin, wider
    /// and brighter for the active match.
    fn draw_find_mark<Renderer: iced::advanced::Renderer>(
        &self,
        renderer: &mut Renderer,
        bounds: Rectangle,
        bar: &Bar,
    ) {
        if !self.marks.is_found(bar.item) {
            return;
        }
        let active = self.marks.active == Some(bar.item);
        let (width, alpha) = if active { (6.0, 1.0) } else { (4.0, 0.8) };
        let height = bar.height.max(if active { 4.0 } else { 3.0 });
        fill(
            renderer,
            Rectangle {
                x: bounds.x + bounds.width - width,
                y: bounds.y + bar.y,
                width,
                height,
            },
            Color {
                a: alpha,
                ..self.colors.warning
            },
        );
    }

    /// Draws one block's bar, with the detail its kind has room for.
    fn draw_bar<Renderer: iced::advanced::Renderer>(
        &self,
        renderer: &mut Renderer,
        area: Rectangle,
        kind: BlockKind,
        seed: usize,
        current: bool,
    ) {
        let colors = &self.colors;
        let muted = |a: f32| Color {
            a,
            ..colors.text_muted
        };
        let accent = |a: f32| Color { a, ..colors.accent };
        let mut fill = |x: f32, y: f32, width: f32, height: f32, color: Color| {
            fill(
                renderer,
                Rectangle {
                    x,
                    y,
                    width,
                    height,
                },
                color,
            );
        };
        // A block too short for lines is one solid bar.
        let lines = line_count(area.height);
        let tall = area.height >= LINE_PITCH * 2.0;
        let line = |i: usize| area.y + i as f32 * LINE_PITCH;
        match kind {
            BlockKind::Heading(level) => {
                let width = area.width * (1.0 - f32::from(level - 1) * 0.08);
                // The heading the reader is in stands out from the other accent bars.
                let color = if current { colors.text } else { accent(1.0) };
                fill(area.x, area.y, width, area.height.max(2.0), color);
            }
            BlockKind::Rule => fill(area.x, area.y, area.width, 1.0, colors.border),
            _ if !tall => {
                let color = match kind {
                    BlockKind::Image => accent(0.4),
                    BlockKind::Code => muted(0.3),
                    _ => muted(0.55),
                };
                fill(area.x, area.y, area.width, area.height, color);
            }
            BlockKind::Paragraph => {
                for i in 0..lines {
                    let length = line_length(seed, i, lines);
                    fill(area.x, line(i), area.width * length, 2.0, muted(0.55));
                }
            }
            BlockKind::List => {
                for i in 0..lines {
                    let length = line_length(seed, i, lines);
                    fill(area.x + 2.0, line(i), 2.0, 2.0, accent(0.9));
                    fill(
                        area.x + 7.0,
                        line(i),
                        (area.width - 7.0) * length,
                        2.0,
                        muted(0.55),
                    );
                }
            }
            BlockKind::Quote => {
                fill(area.x, area.y, 2.0, area.height, accent(0.8));
                for i in 0..lines {
                    let length = line_length(seed, i, lines);
                    fill(
                        area.x + 6.0,
                        line(i),
                        (area.width - 6.0) * length,
                        2.0,
                        muted(0.4),
                    );
                }
            }
            BlockKind::Code => {
                fill(area.x, area.y, area.width, area.height, muted(0.14));
                for i in 0..lines {
                    // Code is indented in steps, unlike prose.
                    let indent = 3.0 + f32::from((seed + i) as u8 % 3) * 3.0;
                    let length = line_length(seed, i, lines) * 0.8;
                    let width = ((area.width - indent - 3.0) * length).max(2.0);
                    fill(area.x + indent, line(i) + 0.5, width, 2.0, accent(0.6));
                }
            }
            BlockKind::Table => {
                fill(area.x, area.y, area.width, area.height, muted(0.1));
                let cell = (area.width - 4.0) / 3.0;
                for i in 0..lines {
                    // The header row is brighter than the body rows.
                    let color = if i == 0 { accent(0.8) } else { muted(0.5) };
                    for column in 0..3 {
                        let x = area.x + 1.0 + column as f32 * (cell + 1.0);
                        fill(x, line(i), cell - 1.0, 2.0, color);
                    }
                }
            }
            BlockKind::Image => {
                fill(area.x, area.y, area.width, area.height, accent(0.18));
                // A sun and a hill: the usual picture glyph.
                fill(area.x + 4.0, area.y + 3.0, 3.0, 3.0, accent(0.7));
                let hill = (area.height * 0.4).max(3.0);
                fill(
                    area.x + area.width * 0.3,
                    area.y + area.height - hill,
                    area.width * 0.55,
                    hill,
                    accent(0.5),
                );
            }
        }
    }
}

fn fill<Renderer: iced::advanced::Renderer>(
    renderer: &mut Renderer,
    bounds: Rectangle,
    color: Color,
) {
    renderer.fill_quad(
        renderer::Quad {
            bounds,
            ..renderer::Quad::default()
        },
        color,
    );
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Minimap<'_, Message>
where
    Renderer: iced::advanced::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Dragging>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Dragging::default())
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let dragging = &mut tree.state.downcast_mut::<Dragging>().0;
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if !self.bars.is_empty() =>
            {
                if let Some(position) = cursor.position_in(bounds) {
                    *dragging = true;
                    shell.publish((self.on_seek)(position.y));
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) if *dragging => {
                shell.publish((self.on_seek)(
                    (position.y - bounds.y).clamp(0.0, bounds.height),
                ));
                shell.capture_event();
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if *dragging => {
                *dragging = false;
                shell.capture_event();
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if tree.state.downcast_ref::<Dragging>().0
            || (!self.bars.is_empty() && cursor.is_over(layout.bounds()))
        {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::None
        }
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(WIDTH), Length::Fill)
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, Length::Fixed(WIDTH), Length::Fill)
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let inner = bounds.width - MARGIN * 2.0;
        if !self.bars.is_empty() {
            // The strip is a raised panel: its shadow falls on the preview to its left.
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    shadow: Shadow {
                        color: Color {
                            a: 0.28,
                            ..Color::BLACK
                        },
                        offset: Vector::new(-2.0, 0.0),
                        blur_radius: 12.0,
                    },
                    ..renderer::Quad::default()
                },
                self.colors.surface,
            );
        }
        for (seed, bar) in self.bars.iter().enumerate() {
            if bar.y > bounds.height {
                break;
            }
            // Leave a hairline between neighbouring bars so each block reads separately.
            let area = Rectangle {
                x: bounds.x + MARGIN,
                y: bounds.y + bar.y,
                width: inner,
                height: (bar.height - 1.0).max(1.0),
            };
            let current = self.marks.heading == Some(bar.item);
            self.draw_bar(renderer, area, bar.kind, seed, current);
            self.draw_find_mark(renderer, bounds, bar);
        }
        let (y, height) = self.slider;
        if height <= 0.0 {
            return;
        }
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: bounds.x,
                    y: bounds.y + y,
                    width: bounds.width,
                    height,
                },
                border: Border {
                    color: Color {
                        a: 0.35,
                        ..self.colors.text_muted
                    },
                    width: 1.0,
                    radius: 2.0.into(),
                },
                ..renderer::Quad::default()
            },
            Color {
                a: 0.12,
                ..self.colors.text
            },
        );
    }
}

impl<'a, Message: 'a, Theme: 'a, Renderer: iced::advanced::Renderer + 'a> From<Minimap<'a, Message>>
    for Element<'a, Message, Theme, Renderer>
{
    fn from(minimap: Minimap<'a, Message>) -> Self {
        Element::new(minimap)
    }
}
