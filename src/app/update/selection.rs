// SPDX-License-Identifier: GPL-3.0-or-later
//! Drag-selecting rendered preview text and copying it.

use crate::app::{FIND_INPUT, Message, State};
use crate::domain::selection::{Position, Selection};
use crate::ui::locate_heading::DOC_SCROLL;
use iced::Task;

/// Longest query Find Selection puts in the find box.
const FIND_SELECTION_MAX_CHARS: usize = 200;

impl State {
    pub(crate) fn update_selection(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectionStarted(block, offset) => {
                self.selection = Some(Selection::at(Position { block, offset }));
                self.selecting = true;
                Task::none()
            }
            Message::SelectionExtended(block, offset) => {
                if self.selecting
                    && let Some(selection) = &mut self.selection
                {
                    selection.extend(Position { block, offset });
                }
                Task::none()
            }
            Message::SelectionExtendedTo(block, offset) => {
                let to = Position { block, offset };
                match &mut self.selection {
                    Some(selection) => selection.extend(to),
                    None => self.selection = Some(Selection::at(to)),
                }
                self.selecting = true;
                Task::none()
            }
            Message::SelectionRange(block, start, end) => {
                self.selection = Some(Selection {
                    anchor: Position {
                        block,
                        offset: start,
                    },
                    head: Position { block, offset: end },
                });
                self.selecting = false;
                self.write_primary()
            }
            Message::SelectionEnded => {
                self.selecting = false;
                self.autoscroll = 0.0;
                self.write_primary()
            }
            Message::SelectionCleared => {
                self.selection = None;
                self.selecting = false;
                self.autoscroll = 0.0;
                Task::none()
            }
            Message::SelectAll => {
                self.selection = Selection::all(
                    self.leaves
                        .iter()
                        .map(|(block, text)| (*block, text.as_str())),
                );
                self.selecting = false;
                self.write_primary()
            }
            Message::SelectionAutoScroll(speed) => {
                self.autoscroll = if self.selecting { speed } else { 0.0 };
                Task::none()
            }
            Message::AutoScrollTick => {
                if !self.selecting || self.autoscroll == 0.0 {
                    return Task::none();
                }
                iced::widget::operation::scroll_by(
                    DOC_SCROLL,
                    iced::widget::operation::AbsoluteOffset {
                        x: 0.0,
                        y: self.autoscroll,
                    },
                )
            }
            Message::CopySelection => match self.selected_text() {
                Some(text) => iced::clipboard::write(text),
                None => Task::none(),
            },
            Message::FindSelection => {
                let Some(query) = self.selected_text().and_then(|text| {
                    let line = text.lines().map(str::trim).find(|line| !line.is_empty())?;
                    Some(
                        line.chars()
                            .take(FIND_SELECTION_MAX_CHARS)
                            .collect::<String>(),
                    )
                }) else {
                    return Task::none();
                };
                self.find.query = query;
                self.find.visible = true;
                self.refresh_find();
                Task::batch([
                    iced::widget::operation::focus(FIND_INPUT),
                    self.locate_find(),
                ])
            }
            _ => unreachable!("selection messages are routed by State::update"),
        }
    }

    /// Mirrors the selection into the X11/Wayland primary selection, so a
    /// middle-click pastes it into other apps.
    #[cfg(all(unix, not(target_os = "macos")))]
    fn write_primary(&self) -> Task<Message> {
        self.selected_text()
            .map_or_else(Task::none, iced::clipboard::write_primary)
    }

    #[cfg(not(all(unix, not(target_os = "macos"))))]
    #[allow(clippy::unused_self)] // Same signature as the Linux version, which reads the selection.
    fn write_primary(&self) -> Task<Message> {
        Task::none()
    }

    /// The selected preview text, leaves joined by newlines; `None` when
    /// nothing is selected.
    pub(crate) fn selected_text(&self) -> Option<String> {
        let selection = self.selection.filter(|selection| !selection.is_empty())?;
        let mut leaves: Vec<_> = self
            .leaves
            .iter()
            .filter(|(block, _)| selection.touches(*block))
            .collect();
        leaves.sort_by_key(|(block, _)| *block);
        let text = leaves
            .into_iter()
            .filter_map(|(block, text)| {
                selection
                    .range_in(*block, text.len())
                    .map(|range| &text[range])
            })
            .collect::<Vec<_>>()
            .join("\n");
        (!text.is_empty()).then_some(text)
    }
}
