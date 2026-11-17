// SPDX-License-Identifier: GPL-3.0-or-later
//! Find in document.

use crate::app::{DocumentView, Message, State};
use crate::domain::find;
use crate::ui::find_viewer::FindViewer;
use crate::ui::locate_heading::{DOC_SCROLL, LocateHeading};
use crate::ui::selectable_text::Drag;
use crate::ui::theme;
use iced::widget::markdown;
use iced::{Element, Task};

impl State {
    pub(crate) fn update_find(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::FindChanged(query) => {
                self.find.query = query;
                self.refresh_find();
                self.locate_find()
            }
            Message::FindNext => self.step_find(1),
            Message::FindPrevious => self.step_find(-1),
            Message::CloseFind => {
                self.find.visible = false;
                Task::none()
            }
            Message::FindLocated(Some(position)) | Message::HeadingLocated(Some(position)) => {
                iced::widget::operation::scroll_to(
                    DOC_SCROLL,
                    iced::widget::operation::AbsoluteOffset {
                        x: None,
                        y: Some(position),
                    },
                )
            }
            Message::FindLocated(None) | Message::HeadingLocated(None) => Task::none(),
            Message::JumpToHeading(index) => {
                iced::advanced::widget::operate(LocateHeading::new(index))
                    .map(Message::HeadingLocated)
            }
            _ => unreachable!("find messages are routed by State::update"),
        }
    }

    pub(crate) fn refresh_find(&mut self) {
        self.find.active = 0;
        if self.find.query.is_empty() {
            self.find.matches.clear();
            return;
        }
        let mut matches = Vec::new();
        if let Some(doc) = self.current_document() {
            match self.document_view {
                DocumentView::Preview => {
                    let settings = markdown::Settings::with_style(markdown::Style::from_palette(
                        theme::palette(self.appearance),
                    ));
                    for (index, item) in doc.items.iter().enumerate() {
                        let viewer = FindViewer::new(
                            &self.find.query,
                            None,
                            self.appearance,
                            index,
                            None,
                            Drag::default(),
                        );
                        let _: Element<'_, Message> =
                            markdown::view_with([item], settings, &viewer);
                        matches.extend(std::iter::repeat_n(index, viewer.count()));
                    }
                }
                DocumentView::Source => {
                    for (index, line) in doc.raw.lines().enumerate() {
                        let count = find::match_ranges(line, &self.find.query).len();
                        matches.extend(std::iter::repeat_n(index, count));
                    }
                }
            }
        }
        self.find.matches = matches;
    }

    pub(crate) fn locate_find(&self) -> Task<Message> {
        self.find
            .matches
            .get(self.find.active)
            .map_or_else(Task::none, |index| {
                iced::advanced::widget::operate(LocateHeading::for_id(format!(
                    "document-find-{index}"
                )))
                .map(Message::FindLocated)
            })
    }

    pub(crate) fn step_find(&mut self, direction: isize) -> Task<Message> {
        if self.find.matches.is_empty() {
            return Task::none();
        }
        self.find.visible = true;
        let len = self.find.matches.len() as isize;
        self.find.active = (self.find.active as isize + direction).rem_euclid(len) as usize;
        self.locate_find()
    }
}
