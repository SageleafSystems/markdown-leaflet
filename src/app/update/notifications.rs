// SPDX-License-Identifier: GPL-3.0-or-later
//! Transient toasts (issue #43). Pushing one is a plain `State::notify` call from
//! wherever has news to report; this module only handles dismissing one.

use crate::app::{Message, State};
use iced::Task;

impl State {
    pub(crate) fn update_notifications(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::DismissNotification(id) => {
                self.notifications
                    .retain(|notification| notification.id != id);
                Task::none()
            }
            _ => unreachable!("notification messages are routed by State::update"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::{Message, State};

    #[test]
    fn notify_assigns_increasing_ids_and_dismiss_removes_only_that_one() {
        let mut state = State::blank();
        state.notify("Theme reloaded.", None);
        state.notify("Saved.", Some(("Undo", Message::DismissError)));
        assert_eq!(state.notifications.len(), 2);
        assert_eq!(state.notifications[0].id, 1);
        assert_eq!(state.notifications[1].id, 2);

        let _ = state.update(Message::DismissNotification(1));
        assert_eq!(state.notifications.len(), 1);
        assert_eq!(state.notifications[0].id, 2);
    }
}
