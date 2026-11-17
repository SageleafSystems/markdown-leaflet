// SPDX-License-Identifier: GPL-3.0-or-later
//! Back and Forward through the documents visited this session.

use crate::app::message::LoadedDocument;
use crate::app::{Message, Mode, State};
use crate::domain::history::History;
use crate::ui::locate_heading::{DOC_SCROLL, ReadScroll};
use iced::Task;
use std::path::PathBuf;

impl State {
    pub(crate) fn update_history(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::HistoryBack => self.history_step(-1),
            Message::HistoryForward => self.history_step(1),
            Message::ScrollRead(Some(offset)) => {
                self.nav.scroll_y = offset;
                Task::none()
            }
            Message::ScrollRead(None) => Task::none(),
            _ => unreachable!("history messages are routed by State::update"),
        }
    }

    pub(crate) fn can_go_back(&self) -> bool {
        matches!(self.mode, Mode::Folder { .. }) && self.nav.history.can_go_back()
    }

    pub(crate) fn can_go_forward(&self) -> bool {
        matches!(self.mode, Mode::Folder { .. }) && self.nav.history.can_go_forward()
    }

    /// Loads the history entry `steps` away. The index only moves once the
    /// document has loaded (see `finish_load`), so a missing file leaves the
    /// document on screen and the history intact.
    fn history_step(&mut self, steps: isize) -> Task<Message> {
        if !matches!(self.mode, Mode::Folder { .. }) {
            return Task::none();
        }
        let Some(target) = self.nav.history.step_target(steps) else {
            return Task::none();
        };
        let Some(path) = self.nav.history.entry(target).map(|e| e.path.clone()) else {
            return Task::none();
        };
        self.nav.cursor = Some(path.clone());
        self.nav.invalidate_pending();
        self.nav.history_target = Some(target);
        self.load_after_reading_scroll(path)
    }

    /// Reads the preview's scroll offset, then loads `path`, so the document
    /// being left can store where the reader was.
    pub(crate) fn load_after_reading_scroll(&mut self, path: PathBuf) -> Task<Message> {
        self.nav.load_generation = self.nav.load_generation.wrapping_add(1);
        let generation = self.nav.load_generation;
        iced::advanced::widget::operate(ReadScroll::new())
            .map(Message::ScrollRead)
            .chain(Task::perform(LoadedDocument::load(path), move |loaded| {
                Message::DocumentLoaded(generation, loaded)
            }))
    }

    /// Records a finished load in the history: a step lands on its target
    /// entry and restores that entry's scroll; a fresh navigation adds an
    /// entry and starts at the top. Returns the scroll to apply.
    pub(crate) fn finish_load(&mut self, path: PathBuf) -> Task<Message> {
        let history: &mut History = &mut self.nav.history;
        history.record_scroll(self.nav.scroll_y);
        match self.nav.history_target.take() {
            Some(index) => history.go(index),
            None => history.visit(path),
        }
        let offset = history.current().map_or(0.0, |entry| entry.scroll);
        self.nav.scroll_y = offset;
        scroll_preview_to(offset)
    }

    /// A load failed: a Back/Forward step still moves onto its entry, so the
    /// reader can keep stepping past a missing file.
    pub(crate) fn fail_load(&mut self) {
        let history = &mut self.nav.history;
        history.record_scroll(self.nav.scroll_y);
        if let Some(index) = self.nav.history_target.take() {
            history.go(index);
        }
    }
}

fn scroll_preview_to(y: f32) -> Task<Message> {
    iced::widget::operation::scroll_to(
        DOC_SCROLL,
        iced::widget::operation::AbsoluteOffset {
            x: None,
            y: Some(y),
        },
    )
}
