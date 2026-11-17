// SPDX-License-Identifier: GPL-3.0-or-later
//! Loading the local images of the open document off the UI thread.

use crate::app::{ErrorNotice, Message, Mode, State};
use crate::domain::images::{self, Decoded};
use crate::ui::images::{Loaded, PendingLoad};
use iced::Task;
use iced::widget::{image, svg};

impl State {
    /// Resolves the open document's images and starts loading the ones not already current.
    /// Called whenever a document is opened or reloaded.
    pub(crate) fn load_images(&mut self) -> Task<Message> {
        let Mode::Folder {
            root,
            selected: Some(doc),
            ..
        } = &self.mode
        else {
            self.images.clear();
            return Task::none();
        };
        let pending = self.images.plan(&doc.items, &doc.path, root);
        Task::batch(pending.into_iter().map(load_one))
    }

    pub(crate) fn update_images(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ImageLoaded(path, mtime, result) => {
                self.images.finish(&path, mtime, result);
                Task::none()
            }
            Message::OpenImage(path) => {
                if let Err(err) = self.opener.open_link(&path.to_string_lossy()) {
                    self.error = Some(ErrorNotice::new(
                        format!("Couldn't open {}: {err}", path.display()),
                        Some(Message::OpenImage(path)),
                    ));
                }
                Task::none()
            }
            Message::CopyImagePath(path) => iced::clipboard::write(path.display().to_string()),
            _ => unreachable!("image messages are routed by State::update"),
        }
    }
}

fn load_one(PendingLoad { path, mtime }: PendingLoad) -> Task<Message> {
    Task::perform(
        {
            let path = path.clone();
            async move {
                tokio::task::spawn_blocking(move || {
                    images::decode(&path)
                        .map(|decoded| match decoded {
                            Decoded::Raster {
                                width,
                                height,
                                rgba,
                            } => Loaded::Raster(image::Handle::from_rgba(width, height, rgba)),
                            Decoded::Svg(bytes) => Loaded::Svg(svg::Handle::from_memory(bytes)),
                        })
                        .map_err(|error| error.to_string())
                })
                .await
                .unwrap_or_else(|error| Err(error.to_string()))
            }
        },
        move |result| Message::ImageLoaded(path.clone(), mtime, result),
    )
}
