// SPDX-License-Identifier: GPL-3.0-or-later
//! Keeps the minimap's scroll geometry and block positions current.

use crate::app::state::MinimapState;
use crate::app::{Message, State};
use crate::domain::minimap::{Block, BlockKind};
use crate::ui::locate_heading::DOC_SCROLL;
use crate::ui::measure_blocks::MeasureBlocks;
use iced::Task;
use std::time::Duration;

/// Wait before re-measuring after a load or resize, so the new layout has been drawn
/// and a burst of resizes measures once.
const MEASURE_DEBOUNCE: Duration = Duration::from_millis(80);

impl State {
    pub(crate) fn update_minimap(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PreviewScrolled(viewport) => {
                let content = viewport.content_bounds().height;
                let geometry = &mut self.minimap.geometry;
                geometry.offset = viewport.absolute_offset().y;
                geometry.viewport_height = viewport.bounds().height;
                geometry.content_height = content;
                geometry.track_height = viewport.bounds().height;
                if self.minimap.measured_height == Some(content) {
                    return Task::none();
                }
                self.minimap.measured_height = Some(content);
                self.measure_preview()
            }
            Message::MeasureDue(generation) => {
                if generation == self.minimap.measure_generation {
                    self.measure_preview()
                } else {
                    Task::none()
                }
            }
            Message::MinimapSeek(y) => {
                let offset = self.minimap.geometry.offset_for_click(y);
                self.minimap.geometry.offset = offset;
                iced::widget::operation::scroll_to(
                    DOC_SCROLL,
                    iced::widget::operation::AbsoluteOffset {
                        x: None,
                        y: Some(offset),
                    },
                )
            }
            Message::BlocksMeasured(measurement) => {
                let Some(document) = self.current_document() else {
                    return Task::none();
                };
                let mut blocks: Vec<Block> = measurement
                    .blocks
                    .into_iter()
                    .filter_map(|(index, y, height)| {
                        Some(Block {
                            item: index,
                            kind: BlockKind::of(document.items.get(index)?),
                            y,
                            height,
                        })
                    })
                    .collect();
                blocks.sort_by(|a, b| a.y.total_cmp(&b.y));
                let geometry = &mut self.minimap.geometry;
                geometry.viewport_height = measurement.viewport_height;
                geometry.track_height = measurement.viewport_height;
                geometry.content_height = measurement.content_height;
                geometry.offset = measurement.offset;
                self.minimap.measured_height = Some(measurement.content_height);
                self.minimap.blocks = blocks;
                Task::none()
            }
            _ => unreachable!("minimap messages are routed by State::update"),
        }
    }

    /// Reads the preview's geometry and blocks now.
    fn measure_preview(&self) -> Task<Message> {
        let count = self.current_document().map_or(0, |doc| doc.items.len());
        iced::advanced::widget::operate(MeasureBlocks::new(count)).map(Message::BlocksMeasured)
    }

    /// Re-measures the preview shortly, once its new layout exists. The scrollable
    /// reports nothing while a document fits the window, so a load, reload, or
    /// resize cannot rely on `PreviewScrolled` to say the minimap is stale.
    pub(crate) fn measure_preview_soon(&mut self) -> Task<Message> {
        self.minimap.measure_generation = self.minimap.measure_generation.wrapping_add(1);
        let generation = self.minimap.measure_generation;
        Task::perform(
            async move {
                tokio::time::sleep(MEASURE_DEBOUNCE).await;
                generation
            },
            Message::MeasureDue,
        )
    }

    /// Forgets the minimap's geometry and blocks, for a document that is gone or replaced.
    pub(crate) fn clear_minimap(&mut self) {
        self.minimap = MinimapState {
            measure_generation: self.minimap.measure_generation.wrapping_add(1),
            ..MinimapState::default()
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::minimap::Geometry;

    fn scrolled_state() -> State {
        let mut state = State::blank();
        state.minimap.geometry = Geometry {
            content_height: 5_000.0,
            viewport_height: 800.0,
            offset: 100.0,
            track_height: 800.0,
        };
        state.minimap.blocks = vec![Block {
            item: 0,
            kind: BlockKind::Paragraph,
            y: 0.0,
            height: 40.0,
        }];
        state.minimap.measured_height = Some(5_000.0);
        state
    }

    #[test]
    fn closing_or_replacing_a_document_clears_the_minimap() {
        let mut state = scrolled_state();
        state.clear_minimap();
        assert!(state.minimap.blocks.is_empty());
        assert_eq!(state.minimap.geometry, Geometry::default());
        assert_eq!(state.minimap.measured_height, None);
    }

    #[test]
    fn only_the_latest_debounced_measure_runs() {
        let mut state = State::blank();
        let _ = state.measure_preview_soon();
        let first = state.minimap.measure_generation;
        let _ = state.measure_preview_soon();
        assert_ne!(first, state.minimap.measure_generation);
        state.minimap.measured_height = Some(1.0);
        let _ = state.update_minimap(Message::MeasureDue(first));
        assert_eq!(state.minimap.measured_height, Some(1.0));
    }

    #[test]
    fn clearing_invalidates_a_pending_measure() {
        let mut state = State::blank();
        let _ = state.measure_preview_soon();
        let pending = state.minimap.measure_generation;
        state.clear_minimap();
        assert_ne!(pending, state.minimap.measure_generation);
    }
}
