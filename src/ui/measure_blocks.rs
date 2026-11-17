// SPDX-License-Identifier: GPL-3.0-or-later
//! A scrollable operation that reads where each rendered block sits in the
//! preview, for the minimap.

use crate::ui::locate_heading::DOC_SCROLL;
use iced::advanced::widget::Id;
use iced::advanced::widget::operation::{Outcome, Scrollable};
use std::collections::HashMap;

/// A block's item index with its top and height in content coordinates.
pub type Measured = (usize, f32, f32);

/// What the preview looks like right now: its scroll geometry and each block's position.
#[derive(Debug, Clone, Default)]
pub struct Measurement {
    pub viewport_height: f32,
    pub content_height: f32,
    pub offset: f32,
    pub blocks: Vec<Measured>,
}

pub struct MeasureBlocks {
    ids: HashMap<Id, usize>,
    content_y: Option<f32>,
    found: Vec<Measured>,
    geometry: Option<(f32, f32, f32)>,
}

impl MeasureBlocks {
    pub fn new(item_count: usize) -> Self {
        Self {
            ids: (0..item_count)
                .map(|index| (Id::from(format!("document-find-{index}")), index))
                .collect(),
            content_y: None,
            found: Vec::new(),
            geometry: None,
        }
    }
}

impl iced::advanced::widget::Operation<Measurement> for MeasureBlocks {
    fn traverse(
        &mut self,
        operate: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation<Measurement>),
    ) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: iced::Rectangle,
        content_bounds: iced::Rectangle,
        translation: iced::Vector,
        _state: &mut dyn Scrollable,
    ) {
        if id == Some(&Id::new(DOC_SCROLL)) {
            self.content_y = Some(content_bounds.y);
            self.geometry = Some((bounds.height, content_bounds.height, translation.y));
        }
    }

    fn container(&mut self, id: Option<&Id>, bounds: iced::Rectangle) {
        let (Some(origin), Some(index)) = (self.content_y, id.and_then(|id| self.ids.get(id)))
        else {
            return;
        };
        self.found.push((*index, bounds.y - origin, bounds.height));
    }

    fn finish(&self) -> Outcome<Measurement> {
        let (viewport_height, content_height, offset) = self.geometry.unwrap_or_default();
        Outcome::Some(Measurement {
            viewport_height,
            content_height,
            offset,
            blocks: self.found.clone(),
        })
    }
}
