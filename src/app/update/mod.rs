// SPDX-License-Identifier: GPL-3.0-or-later
//! The single decision point for user actions and timer messages: routes each
//! `Message` to the handler for its area.

use crate::app::{Message, State};
use iced::Task;

mod context_menu;
mod delete;
mod external_link;
mod files;
mod find;
mod gutters;
mod history;
mod images;
mod keyboard;
mod layout;
mod menu;
mod minimap;
mod navigation;
mod new_item;
mod notifications;
mod selection;
mod tasks;

impl State {
    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::MenuAction(..) => self.update_menu(message),
            #[cfg(not(target_os = "macos"))]
            Message::ToggleMenu(..) => self.update_menu(message),
            #[cfg(not(target_os = "macos"))]
            Message::ActivateMenu(..) => self.update_menu(message),
            #[cfg(not(target_os = "macos"))]
            Message::OpenRecentMenu => self.update_menu(message),
            #[cfg(not(target_os = "macos"))]
            Message::CloseRecentMenu => self.update_menu(message),
            #[cfg(not(target_os = "macos"))]
            Message::DismissMenu => self.update_menu(message),
            Message::DismissDialog => self.update_menu(message),
            #[cfg(target_os = "macos")]
            Message::NativeMenuTick => self.update_menu(message),
            Message::OpenRecent(..) => self.update_files(message),
            Message::OpenFileClicked => self.update_files(message),
            Message::OpenFolderClicked => self.update_files(message),
            Message::FileOpened(..) => self.update_files(message),
            Message::FolderOpened(..) => self.update_files(message),
            Message::FolderPoll | Message::ToggleShowTextFiles => self.update_files(message),
            Message::FolderScanned(..) | Message::WorkspaceOpened(..) => self.update_files(message),
            Message::FileSelected(..) => self.update_files(message),
            Message::DocumentLoaded(..) => self.update_files(message),
            Message::DocumentReloaded(..) => self.update_files(message),
            Message::NavCommit(..) => self.update_navigation(message),
            Message::ToggleFolder(..) => self.update_navigation(message),
            Message::NavCursorLocated(..) => self.update_navigation(message),
            Message::SearchChanged(..) => self.update_navigation(message),
            Message::SearchCleared => self.update_navigation(message),
            Message::FindChanged(..) => self.update_find(message),
            Message::FindNext => self.update_find(message),
            Message::FindPrevious => self.update_find(message),
            Message::CloseFind => self.update_find(message),
            Message::FindLocated(..) => self.update_find(message),
            Message::JumpToHeading(..) => self.update_find(message),
            Message::HeadingLocated(..) => self.update_find(message),
            Message::LinkClicked(..) => self.update_files(message),
            Message::ImageLoaded(..) => self.update_images(message),
            Message::OpenImage(..) => self.update_images(message),
            Message::CopyImagePath(..) => self.update_images(message),
            Message::ContextImagePressed(..) => self.update_context_menu(message),
            Message::HistoryBack => self.update_history(message),
            Message::HistoryForward => self.update_history(message),
            Message::ScrollRead(..) => self.update_history(message),
            Message::PreviewScrolled(..)
            | Message::BlocksMeasured(..)
            | Message::MeasureDue(..)
            | Message::MinimapSeek(..) => self.update_minimap(message),
            Message::ReloadPoll => self.update_files(message),
            Message::TaskToggled(..) => self.update_tasks(message),
            Message::DocumentSaved(..) => self.update_tasks(message),
            Message::KeyPressed(..) => self.update_keyboard(message),
            Message::TogglePanel(..) => self.update_gutters(message),
            Message::SelectRightTab(..) => self.update_gutters(message),
            Message::SelectLeftTab(..) => self.update_gutters(message),
            Message::OpenWith(..) => self.update_files(message),
            Message::DismissError => self.update_files(message),
            Message::DismissNotification(..) => self.update_notifications(message),
            Message::CopySource => self.update_files(message),
            Message::ResizeDragStart(..) => self.update_gutters(message),
            Message::SidebarDragged(..) => self.update_gutters(message),
            Message::SidebarDragEnd => self.update_gutters(message),
            Message::ResizeZoneEntered(..) => self.update_gutters(message),
            Message::ResizeZoneExited(..) => self.update_gutters(message),
            Message::AnimationTick => self.update_gutters(message),
            Message::SearchFocused(..) => self.update_gutters(message),
            Message::WindowResized(..) => self.update_layout(message),
            Message::SelectionStarted(..) => self.update_selection(message),
            Message::SelectionExtended(..) => self.update_selection(message),
            Message::SelectionEnded => self.update_selection(message),
            Message::SelectionCleared => self.update_selection(message),
            Message::CopySelection => self.update_selection(message),
            Message::SelectAll => self.update_selection(message),
            Message::SelectionExtendedTo(..) => self.update_selection(message),
            Message::SelectionRange(..) => self.update_selection(message),
            Message::SelectionAutoScroll(..) => self.update_selection(message),
            Message::AutoScrollTick => self.update_selection(message),
            Message::FindSelection => self.update_selection(message),
            Message::CopyLink(..) => self.update_files(message),
            Message::ContextLinkPressed(..) => self.update_context_menu(message),
            Message::OpenContextMenu(..) => self.update_context_menu(message),
            Message::NewMarkdownFile(..)
            | Message::NewFolder(..)
            | Message::RenameItem(..)
            | Message::NewItemChanged(..)
            | Message::NewItemSubmit
            | Message::NewItemBlur
            | Message::NewItemCancel
            | Message::NewItemHover(..) => self.update_new_item(message),
            Message::DeleteItem(..) | Message::ConfirmDelete | Message::CancelDelete => {
                self.update_delete(message)
            }
            Message::ExternalLinkAlways(..)
            | Message::ExternalLinkOpenFolder
            | Message::ExternalLinkOpenDocument
            | Message::ExternalLinkOpenWith(..)
            | Message::CancelExternalLink => self.update_external_link(message),
            Message::RevealInFileManager(..)
            | Message::OpenInNewWindow(..)
            | Message::OpenExternally(..)
            | Message::CopyPath(..)
            | Message::CopyMetric(..)
            | Message::CopyAllStats(..) => self.update_files(message),
            Message::OpenNewDropdown(..)
            | Message::OpenFileOrFolderDropdown(..)
            | Message::OpenFontDropdown(..)
            | Message::OpenThemeDropdown(..)
            | Message::OpenEditorDropdown(..)
            | Message::OpenMetricsFormatDropdown(..) => self.update_context_menu(message),
            Message::ContextMenuPick(..) => self.update_context_menu(message),
            Message::DismissContextMenu | Message::ContextMenuInert => {
                self.update_context_menu(message)
            }
        }
    }
}
