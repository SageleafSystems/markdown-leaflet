// SPDX-License-Identifier: GPL-3.0-or-later
use crate::app::{FOLDER_POLL_INTERVAL, Message, Mode, RELOAD_POLL_INTERVAL, State};
use crate::domain::walk::poll_interval;
use crate::gutter;

/// One frame of drag auto-scroll, about 60 per second.
const AUTOSCROLL_FRAME: std::time::Duration = std::time::Duration::from_millis(16);

impl State {
    /// Polls for on-disk changes to whichever document is currently shown,
    /// so edits made in another editor show up without reopening the file.
    /// Only active while a document is actually open, so the app is fully
    /// idle on the welcome screen and folder-listing-without-selection.
    pub(crate) fn subscription(&self) -> iced::Subscription<Message> {
        let has_open_document = self.current_document().is_some();

        let reload_poll = if has_open_document {
            iced::time::every(RELOAD_POLL_INTERVAL).map(|_| Message::ReloadPoll)
        } else {
            iced::Subscription::none()
        };

        let folder_poll = if matches!(self.mode, Mode::Folder { .. }) {
            // Back off on a tree that is slow to scan, so a huge folder is not rescanned flat out.
            let interval = poll_interval(self.nav.last_scan, FOLDER_POLL_INTERVAL);
            iced::time::every(interval).map(|_| Message::FolderPoll)
        } else {
            iced::Subscription::none()
        };

        let keyboard = iced::event::listen_with(|event, status, _window| match event {
            iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                Some(Message::KeyPressed(
                    key,
                    modifiers,
                    status == iced::event::Status::Captured,
                ))
            }
            _ => None,
        });

        let history_buttons = iced::event::listen_with(|event, _status, _window| match event {
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Back)) => {
                Some(Message::HistoryBack)
            }
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Forward)) => {
                Some(Message::HistoryForward)
            }
            _ => None,
        });

        // While a new item is named inline, any left press (the field's own is told apart by
        // its hover flag) finishes the edit.
        let draft_blur = if self.nav.draft.is_some() && self.draft_inline() {
            iced::event::listen_with(|event, _status, _window| match event {
                iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
                    iced::mouse::Button::Left,
                )) => Some(Message::NewItemBlur),
                _ => None,
            })
        } else {
            iced::Subscription::none()
        };

        // Only live while a drag is in progress. Global move and release
        // events keep tracking after the cursor leaves the divider target.
        let sidebar_drag = if self.gutters.drag.is_some() {
            iced::event::listen_with(|event, _status, _window| match event {
                iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                    Some(Message::SidebarDragged(position.x))
                }
                iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                    iced::mouse::Button::Left,
                )) => Some(Message::SidebarDragEnd),
                _ => None,
            })
        } else {
            iced::Subscription::none()
        };

        let animation = if self.gutters.is_animating()
            || self.search_expand.is_animating()
            || !self.copy_flash.is_empty()
        {
            iced::time::every(gutter::ANIMATION_FRAME).map(|_| Message::AnimationTick)
        } else {
            iced::Subscription::none()
        };

        let autoscroll = if self.selecting && self.autoscroll != 0.0 {
            iced::time::every(AUTOSCROLL_FRAME).map(|_| Message::AutoScrollTick)
        } else {
            iced::Subscription::none()
        };

        let window_resize =
            iced::window::resize_events().map(|(_id, size)| Message::WindowResized(size));

        let subscriptions = [
            window_resize,
            reload_poll,
            folder_poll,
            keyboard,
            history_buttons,
            draft_blur,
            sidebar_drag,
            animation,
            autoscroll,
        ];
        #[cfg(target_os = "macos")]
        return iced::Subscription::batch([
            iced::Subscription::batch(subscriptions),
            iced::time::every(std::time::Duration::from_millis(100))
                .map(|_| Message::NativeMenuTick),
        ]);
        #[cfg(not(target_os = "macos"))]
        iced::Subscription::batch(subscriptions)
    }
}
