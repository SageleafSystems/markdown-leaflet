// SPDX-License-Identifier: GPL-3.0-or-later
//! A scrollable operation that finds the offset of a heading or find match.

/// Id of the scrollable that holds the rendered document.
pub const DOC_SCROLL: &str = "document-scroll";

pub struct LocateHeading {
    target: iced::advanced::widget::Id,
    content_y: Option<f32>,
    position: Option<f32>,
}

impl LocateHeading {
    pub fn new(index: usize) -> Self {
        Self::for_id(format!("document-heading-{index}"))
    }

    pub fn for_id(id: String) -> Self {
        Self {
            target: id.into(),
            content_y: None,
            position: None,
        }
    }
}

impl iced::advanced::widget::Operation<Option<f32>> for LocateHeading {
    fn traverse(
        &mut self,
        operate: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation<Option<f32>>),
    ) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        id: Option<&iced::advanced::widget::Id>,
        _bounds: iced::Rectangle,
        content_bounds: iced::Rectangle,
        _translation: iced::Vector,
        _state: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        if id == Some(&iced::advanced::widget::Id::new(DOC_SCROLL)) {
            self.content_y = Some(content_bounds.y);
        }
    }

    fn container(&mut self, id: Option<&iced::advanced::widget::Id>, bounds: iced::Rectangle) {
        if id == Some(&self.target) {
            self.position = self.content_y.map(|origin| bounds.y - origin);
        }
    }

    fn finish(&self) -> iced::advanced::widget::operation::Outcome<Option<f32>> {
        iced::advanced::widget::operation::Outcome::Some(self.position)
    }
}

/// Fraction of the viewport height a page scroll moves, leaving a sliver of
/// overlap so the reader keeps their place (the usual PageUp/PageDown feel).
const PAGE_OVERLAP: f32 = 0.9;

/// Scrolls the preview up or down by about one page, measured against its
/// own viewport height rather than a fixed pixel count.
pub struct PageScroll {
    direction: f32,
}

impl PageScroll {
    pub fn down() -> Self {
        Self { direction: 1.0 }
    }

    pub fn up() -> Self {
        Self { direction: -1.0 }
    }
}

impl iced::advanced::widget::Operation for PageScroll {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        id: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        content_bounds: iced::Rectangle,
        _translation: iced::Vector,
        state: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        if id == Some(&iced::advanced::widget::Id::new(DOC_SCROLL)) {
            state.scroll_by(
                iced::advanced::widget::operation::scrollable::AbsoluteOffset {
                    x: 0.0,
                    y: self.direction * bounds.height * PAGE_OVERLAP,
                },
                bounds,
                content_bounds,
            );
        }
    }
}

/// Reads the preview's current vertical scroll offset.
pub struct ReadScroll {
    offset: Option<f32>,
}

impl ReadScroll {
    pub fn new() -> Self {
        Self { offset: None }
    }
}

impl iced::advanced::widget::Operation<Option<f32>> for ReadScroll {
    fn traverse(
        &mut self,
        operate: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation<Option<f32>>),
    ) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        id: Option<&iced::advanced::widget::Id>,
        _bounds: iced::Rectangle,
        _content_bounds: iced::Rectangle,
        translation: iced::Vector,
        _state: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        if id == Some(&iced::advanced::widget::Id::new(DOC_SCROLL)) {
            self.offset = Some(translation.y);
        }
    }

    fn finish(&self) -> iced::advanced::widget::operation::Outcome<Option<f32>> {
        iced::advanced::widget::operation::Outcome::Some(self.offset)
    }
}
