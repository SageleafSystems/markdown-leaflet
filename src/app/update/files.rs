// SPDX-License-Identifier: GPL-3.0-or-later
//! Opening files and folders, rescans, reloads, and external programs.

use crate::app::message::LoadedDocument;
use crate::app::{DocumentView, ErrorNotice, FindState, Message, Mode, State};
use crate::config::Config;
use crate::domain::document::DocumentKind;
use crate::domain::folder::expand_file_ancestors;
use crate::domain::links::{local_markdown_target, may_open_externally};
use crate::domain::recent::{RecentItem, RecentKind};
use crate::domain::walk::collect_markdown_files;
use crate::domain::workspace::{OpenRequest, Workspace, include_selected};
use crate::gutter::motion::{Easing, Tween};
use crate::platform::dialogs::{pick_file, pick_folder};
use iced::Task;
use iced::widget::markdown;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// How long the "Copied to clipboard." toast stays up before dismissing itself.
const COPY_TOAST_DURATION: Duration = Duration::from_millis(1600);

/// How long a metric card's value takes to fade from the accent color back to normal after a
/// copy.
const METRIC_FLASH_DURATION: Duration = Duration::from_millis(500);

/// A tween that starts at full accent color (1.0) and eases back to normal (0.0) right away:
/// a quick flash rather than a fade-in-then-out, so the copy reads as instant feedback.
fn metric_flash() -> Tween {
    Tween::new(1.0, 0.0, Instant::now(), METRIC_FLASH_DURATION, Easing::Out)
}

impl State {
    /// The Markdown file a clicked link names, when it lives in the open
    /// folder and should replace the current document instead of leaving
    /// the app.
    fn local_markdown_link(&self, uri: &str) -> Option<std::path::PathBuf> {
        let Mode::Folder { root, selected, .. } = &self.mode else {
            return None;
        };
        let doc_dir = selected.as_ref()?.path.parent()?;
        local_markdown_target(uri, doc_dir, root, self.config.files.show_text)
    }

    /// Whether a clicked link may go to the operating system. In single-file mode the document's
    /// own folder is the boundary for local links.
    fn may_open_link(&self, uri: &str) -> bool {
        let (doc_dir, root) = match &self.mode {
            Mode::Folder { root, selected, .. } => {
                let dir = selected
                    .as_ref()
                    .and_then(|s| s.path.parent())
                    .unwrap_or(root);
                (dir.to_path_buf(), root.clone())
            }
            Mode::Welcome => (std::path::PathBuf::new(), std::path::PathBuf::new()),
        };
        may_open_externally(uri, &doc_dir, &root)
    }

    /// A background rescan finished: refresh the list, and drop a selection or folder that is gone.
    fn folder_scanned(
        &mut self,
        scanned_root: PathBuf,
        scanned_files: Vec<PathBuf>,
        took: std::time::Duration,
    ) -> Task<Message> {
        self.nav.scan_in_flight = false;
        self.nav.last_scan = took;
        let Mode::Folder {
            root,
            files,
            selected,
            ..
        } = &mut self.mode
        else {
            return Task::none();
        };
        if *root != scanned_root {
            return Task::none();
        }
        if self
            .nav
            .draft
            .as_ref()
            .is_some_and(|draft| !draft.parent.is_dir())
        {
            self.nav.draft = None;
        }
        if !scanned_root.is_dir() {
            self.error = Some(ErrorNotice::new(
                format!("Folder is unavailable: {}", scanned_root.display()),
                Some(Message::OpenRecent(RecentItem {
                    kind: RecentKind::Folder,
                    path: scanned_root.clone(),
                    selected: None,
                })),
            ));
            return Task::none();
        }
        let scanned_files = include_selected(
            scanned_files,
            selected.as_ref().map(|doc| doc.path.as_path()),
        );
        let removed_selection = selected
            .as_ref()
            .is_some_and(|doc| !scanned_files.contains(&doc.path));
        if removed_selection {
            *selected = None;
            self.find = FindState::default();
            self.selection = None;
            self.selecting = false;
            self.leaves.clear();
            self.error = Some(ErrorNotice::new(
                "The selected file was removed from this folder. Choose another file from the sidebar.",
                None,
            ));
        }
        if *files != scanned_files {
            *files = scanned_files;
        }
        if self
            .nav
            .cursor
            .as_ref()
            .is_some_and(|cursor| !cursor.exists())
        {
            self.nav.cursor = None;
            self.nav.invalidate_pending();
        }
        if removed_selection {
            self.remember(RecentKind::Folder, scanned_root, None);
        }
        Task::none()
    }

    /// A document finished loading: show it, unless a newer open has superseded it.
    fn document_loaded(&mut self, generation: u64, loaded: LoadedDocument) -> Task<Message> {
        if generation != self.nav.load_generation {
            return Task::none();
        }
        let Some(result) = loaded.take() else {
            return Task::none();
        };
        let mut recent_folder = None;
        let mut opened = None;
        let mut scroll = Task::none();
        let mut images = Task::none();
        let mut remeasure = Task::none();
        if let Mode::Folder { selected, .. } = &mut self.mode {
            match result {
                Ok(doc) => {
                    opened = Some(doc.path.clone());
                    // Plain text has no rendered form, so it is always shown as typed.
                    self.document_view = if doc.kind == DocumentKind::Text {
                        DocumentView::Source
                    } else {
                        DocumentView::Preview
                    };
                    *selected = Some(doc);
                    self.find = FindState::default();
                    self.error = None;
                    self.save_status = None;
                    self.nav.write_in_flight = false;
                    self.nav.write_dirty = false;
                    if let Mode::Folder { root, .. } = &self.mode {
                        recent_folder = Some(root.clone());
                    }
                    self.refresh_selection();
                    self.clear_minimap();
                    if let Some(path) = &opened {
                        scroll = self.finish_load(path.clone());
                    }
                    images = self.load_images();
                    remeasure = self.measure_preview_soon();
                }
                Err(err) => {
                    self.fail_load();
                    let path = err.path.clone();
                    self.error = Some(ErrorNotice::new(err, Some(Message::FileSelected(path))));
                }
            }
        }
        if let (Some(root), Some(path)) = (recent_folder, opened) {
            expand_file_ancestors(&root, &path, &mut self.nav.expanded);
            self.remember(RecentKind::Folder, root, Some(path));
        }
        Task::batch([scroll, images, remeasure])
    }

    /// A link in the preview was clicked: open it here, ask about it, or hand it to the OS.
    fn link_clicked(&mut self, uri: markdown::Uri) -> Task<Message> {
        if let Some(path) = self.local_markdown_link(&uri) {
            return self.update(Message::FileSelected(path));
        }
        if let Some(path) = self.external_markdown_link(&uri) {
            return self.open_external_markdown(path);
        }
        if !self.may_open_link(&uri) {
            self.error = Some(ErrorNotice::new(
                format!("Blocked link {uri}: only web, email and local document links open."),
                None,
            ));
            return Task::none();
        }
        if let Err(err) = self.opener.open_link(&uri) {
            self.error = Some(ErrorNotice::new(
                format!("Couldn't open link {uri}: {err}"),
                Some(Message::LinkClicked(uri)),
            ));
        }
        Task::none()
    }

    /// Checks whether the shown document changed on disk and reloads it in the background.
    fn reload_poll(&mut self) -> Task<Message> {
        if self.nav.reload_in_flight || self.nav.write_in_flight {
            return Task::none();
        }
        let Mode::Folder {
            selected: Some(doc),
            ..
        } = &self.mode
        else {
            return Task::none();
        };
        if !doc.is_stale() {
            return Task::none();
        }
        self.nav.reload_in_flight = true;
        Task::perform(
            LoadedDocument::load(doc.path.clone()),
            Message::DocumentReloaded,
        )
    }

    /// Starts a background rescan of the open folder unless one is already running.
    fn folder_poll(&mut self) -> Task<Message> {
        if self.nav.scan_in_flight {
            return Task::none();
        }
        let Mode::Folder { root, .. } = &self.mode else {
            return Task::none();
        };
        let root = root.clone();
        let config = self.config.clone();
        self.nav.scan_in_flight = true;
        Task::perform(
            async move {
                let started = std::time::Instant::now();
                let files = collect_markdown_files(&root, &config.scan_rules());
                (root, files, started.elapsed())
            },
            |(root, files, took)| Message::FolderScanned(root, files, took),
        )
    }

    /// A background reload finished: swap in the new contents if it is still the shown file.
    fn document_reloaded(&mut self, loaded: LoadedDocument) -> Task<Message> {
        self.nav.reload_in_flight = false;
        // An unreadable file is retried on the next tick, as when it was mid-write.
        let Some(Ok(reloaded)) = loaded.take() else {
            return Task::none();
        };
        if let Mode::Folder {
            selected: Some(doc),
            ..
        } = &mut self.mode
            && doc.path == reloaded.path
        {
            *doc = reloaded;
            self.refresh_find();
            self.refresh_selection();
        }
        Task::batch([self.load_images(), self.measure_preview_soon()])
    }

    pub(crate) fn update_files(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenRecent(item) => self.request_open(OpenRequest::from_recent(&item)),
            Message::WorkspaceOpened(generation, request, loaded) => {
                // A newer open has started since; its result is the one to show.
                if generation == self.nav.open_generation
                    && let Some(result) = loaded.take()
                {
                    self.finish_open(request, result);
                }
                Task::none()
            }
            Message::OpenFileClicked => Task::perform(pick_file(), Message::FileOpened),
            Message::OpenFolderClicked => {
                Task::perform(pick_folder(self.config.clone()), Message::FolderOpened)
            }
            Message::FileOpened(Some(path)) => self.request_open(OpenRequest::file(&path)),
            Message::FileOpened(None) | Message::FolderOpened(None) => Task::none(),
            Message::FolderOpened(Some((root, scan))) => {
                self.show_workspace(Workspace::from_scan(root, scan));
                Task::none()
            }
            Message::FolderPoll => self.folder_poll(),
            Message::ToggleShowTextFiles => {
                let shown = !self.config.files.show_text;
                match Config::save_show_text(shown) {
                    Ok(()) => {
                        self.config.files.show_text = shown;
                        self.error = None;
                    }
                    Err(error) => {
                        self.error = Some(ErrorNotice::new(error, None));
                        return Task::none();
                    }
                }
                // Rescan now under the new rules; a scan already running used the old ones.
                self.nav.scan_in_flight = false;
                self.folder_poll()
            }
            Message::FolderScanned(scanned_root, scanned_files, took) => {
                self.folder_scanned(scanned_root, scanned_files, took)
            }
            Message::FileSelected(path) => {
                self.nav.cursor = Some(path.clone());
                self.nav.invalidate_pending();
                if !matches!(self.mode, Mode::Folder { .. }) {
                    return Task::none();
                }
                self.nav.history_target = None;
                self.load_after_reading_scroll(path)
            }
            Message::DocumentLoaded(generation, loaded) => self.document_loaded(generation, loaded),
            Message::LinkClicked(uri) => self.link_clicked(uri),
            Message::ReloadPoll => self.reload_poll(),
            Message::DocumentReloaded(loaded) => self.document_reloaded(loaded),
            Message::OpenWith(editor) => {
                if let Some(doc) = self.current_document() {
                    if let Err(err) = self.opener.open_with(&doc.path, editor.command) {
                        self.error = Some(ErrorNotice::new(
                            format!(
                                "Couldn't open {} with {}: {err}",
                                doc.path.display(),
                                editor.name
                            ),
                            Some(Message::OpenWith(editor)),
                        ));
                    } else {
                        self.error = None;
                    }
                }
                Task::none()
            }
            Message::RevealInFileManager(path) => {
                let result = self.opener.reveal(&path);
                self.report_failure(
                    result,
                    "show",
                    &path,
                    Message::RevealInFileManager(path.clone()),
                )
            }
            Message::OpenInNewWindow(path) => {
                let result = self.opener.open_new_window(&path);
                self.report_failure(
                    result,
                    "open in a new window",
                    &path,
                    Message::OpenInNewWindow(path.clone()),
                )
            }
            Message::OpenExternally(path) => {
                let result = self.opener.open_link(&path.to_string_lossy());
                self.report_failure(result, "open", &path, Message::OpenExternally(path.clone()))
            }
            Message::CopyPath(path) => iced::clipboard::write(path.display().to_string()),
            Message::CopyMetric(label, value) => {
                self.copy_flash.insert(label, metric_flash());
                Task::batch([
                    iced::clipboard::write(format!("{label}: {value}")),
                    self.notify_expiring("Copied to clipboard.", COPY_TOAST_DURATION),
                ])
            }
            Message::CopyAllStats(format) => {
                self.menus.stats_format = format;
                match self.current_document() {
                    Some(doc) => Task::batch([
                        iced::clipboard::write(doc.stats.format(doc.outline.len(), format)),
                        self.notify_expiring("Copied to clipboard.", COPY_TOAST_DURATION),
                    ]),
                    None => Task::none(),
                }
            }
            Message::DismissError => {
                self.error = None;
                Task::none()
            }
            Message::CopyLink(uri) => iced::clipboard::write(uri),
            Message::CopySource => {
                if let Some(doc) = self.current_document() {
                    return iced::clipboard::write(doc.raw.clone());
                }
                Task::none()
            }
            _ => unreachable!("files messages are routed by State::update"),
        }
    }
}

impl State {
    /// Surfaces a failed OS hand-off as the error banner, with a retry.
    fn report_failure(
        &mut self,
        result: std::io::Result<()>,
        verb: &str,
        path: &std::path::Path,
        retry: Message,
    ) -> Task<Message> {
        if let Err(err) = result {
            self.error = Some(ErrorNotice::new(
                format!("Couldn't {verb} {}: {err}", path.display()),
                Some(retry),
            ));
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::LoadedWorkspace;
    use crate::domain::document::Document;
    use crate::domain::walk::Scan;
    use crate::domain::workspace::OpenRequest;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    /// A folder with `a.md` and `b.md`, shown with `a.md` selected.
    fn open_folder_state(name: &str) -> (State, crate::testutil::ScratchDir) {
        let dir = crate::testutil::scratch_dir(&format!("files-{name}"));
        std::fs::write(dir.join("a.md"), "# a").unwrap();
        std::fs::write(dir.join("b.md"), "# b").unwrap();
        let mut state = State::blank();
        state.mode = Mode::Folder {
            root: dir.to_path_buf(),
            files: vec![dir.join("a.md"), dir.join("b.md")],
            query: String::new(),
            selected: Document::open(dir.join("a.md")).ok(),
        };
        (state, dir)
    }

    fn files_of(state: &State) -> Vec<PathBuf> {
        match &state.mode {
            Mode::Folder { files, .. } => files.clone(),
            Mode::Welcome => Vec::new(),
        }
    }

    fn selected_of(state: &State) -> Option<PathBuf> {
        match &state.mode {
            Mode::Folder { selected, .. } => selected.as_ref().map(|doc| doc.path.clone()),
            Mode::Welcome => None,
        }
    }

    fn folder_item(path: &Path) -> RecentItem {
        RecentItem {
            kind: RecentKind::Folder,
            path: path.to_path_buf(),
            selected: None,
        }
    }

    #[test]
    fn opening_a_recent_folder_lists_its_files_with_nothing_selected() {
        let (_state, dir) = open_folder_state("recent-folder");
        let mut state = State::blank();
        let _ = state.update(Message::OpenRecent(folder_item(&dir)));
        state.complete_pending_open();
        assert_eq!(files_of(&state).len(), 2);
        assert_eq!(selected_of(&state), None);
        assert!(state.error.is_none());
    }

    #[test]
    fn opening_a_recent_file_selects_it_in_its_folder() {
        let (_state, dir) = open_folder_state("recent-file");
        let mut state = State::blank();
        let item = RecentItem {
            kind: RecentKind::File,
            path: dir.join("b.md"),
            selected: None,
        };
        let _ = state.update(Message::OpenRecent(item));
        state.complete_pending_open();
        assert!(selected_of(&state).is_some_and(|path| path.ends_with("b.md")));
        assert_eq!(files_of(&state).len(), 2);
    }

    #[test]
    fn opening_a_missing_recent_folder_reports_it_with_a_retry() {
        let dir = crate::testutil::scratch_dir("files-missing");
        let mut state = State::blank();
        let _ = state.update(Message::OpenRecent(folder_item(&dir.join("gone"))));
        state.complete_pending_open();
        assert!(state.nav.opening.is_none(), "the progress text is cleared");
        assert!(
            matches!(state.mode, Mode::Welcome),
            "stays on the welcome screen"
        );
        let notice = state.error.as_ref().expect("an error notice");
        assert!(notice.retry.is_some());
    }

    #[test]
    fn a_picked_folder_opens_and_a_cancelled_pick_changes_nothing() {
        let (_state, dir) = open_folder_state("picked");
        let mut state = State::blank();
        let _ = state.update(Message::FolderOpened(None));
        assert!(matches!(state.mode, Mode::Welcome));
        let scan = Scan {
            files: vec![dir.join("a.md")],
            unreadable: vec![dir.join("locked")],
        };
        let _ = state.update(Message::FolderOpened(Some((dir.to_path_buf(), scan))));
        assert_eq!(files_of(&state), vec![dir.join("a.md")]);
        assert!(state.error.as_ref().unwrap().message.contains("“locked”"));
        let _ = state.update(Message::FileOpened(None));
        assert_eq!(
            files_of(&state).len(),
            1,
            "a cancelled file pick changes nothing"
        );
    }

    #[test]
    fn a_rescan_is_skipped_while_one_is_running_and_outside_a_folder() {
        let mut welcome = State::blank();
        let _ = welcome.update(Message::FolderPoll);
        assert!(
            !welcome.nav.scan_in_flight,
            "nothing to scan on the welcome screen"
        );
        let (mut state, _dir) = open_folder_state("poll");
        let _ = state.update(Message::FolderPoll);
        assert!(state.nav.scan_in_flight);
        let _ = state.update(Message::FolderPoll);
        assert!(
            state.nav.scan_in_flight,
            "a second poll does not start another scan"
        );
    }

    #[test]
    fn a_finished_rescan_updates_the_list_and_remembers_how_long_it_took() {
        let (mut state, dir) = open_folder_state("scanned");
        state.nav.scan_in_flight = true;
        std::fs::write(dir.join("c.md"), "# c").unwrap();
        let files = vec![dir.join("a.md"), dir.join("b.md"), dir.join("c.md")];
        let _ = state.update(Message::FolderScanned(
            dir.to_path_buf(),
            files.clone(),
            Duration::from_millis(40),
        ));
        assert_eq!(files_of(&state), files);
        assert!(!state.nav.scan_in_flight);
        assert_eq!(state.nav.last_scan, Duration::from_millis(40));
        assert!(
            selected_of(&state).is_some(),
            "a still-listed selection stays"
        );
    }

    #[test]
    fn a_rescan_that_drops_the_open_file_clears_it_and_says_so() {
        let (mut state, dir) = open_folder_state("removed");
        std::fs::remove_file(dir.join("a.md")).unwrap();
        let _ = state.update(Message::FolderScanned(
            dir.to_path_buf(),
            vec![dir.join("b.md")],
            Duration::ZERO,
        ));
        assert_eq!(selected_of(&state), None);
        assert_eq!(files_of(&state), vec![dir.join("b.md")]);
        assert!(
            state
                .error
                .as_ref()
                .unwrap()
                .message
                .contains("was removed")
        );
    }

    #[test]
    fn a_rescan_of_a_folder_that_is_no_longer_open_is_ignored() {
        let (mut state, dir) = open_folder_state("stale");
        let other = crate::testutil::scratch_dir("files-other");
        let before = files_of(&state);
        let _ = state.update(Message::FolderScanned(
            other.to_path_buf(),
            vec![other.join("x.md")],
            Duration::from_secs(2),
        ));
        assert_eq!(files_of(&state), before);
        assert!(state.error.is_none());
        let _ = dir;
    }

    #[test]
    fn a_folder_that_vanishes_is_reported_with_a_way_to_reopen_it() {
        let (mut state, dir) = open_folder_state("vanished");
        let root = dir.to_path_buf();
        std::fs::remove_dir_all(&root).unwrap();
        let _ = state.update(Message::FolderScanned(root, vec![], Duration::ZERO));
        let notice = state.error.as_ref().expect("an error notice");
        assert!(notice.message.contains("Folder is unavailable"));
        assert!(matches!(notice.retry, Some(Message::OpenRecent(_))));
    }

    #[test]
    fn the_current_view_stays_until_the_background_open_finishes() {
        let (mut state, _dir) = open_folder_state("stays");
        let other = crate::testutil::scratch_dir("files-next");
        std::fs::write(other.join("n.md"), "# n").unwrap();
        let before = files_of(&state);
        let _ = state.update(Message::OpenRecent(folder_item(&other)));
        assert_eq!(files_of(&state), before, "nothing changes while it opens");
        assert_eq!(
            state.nav.opening.as_ref().map(OpenRequest::label),
            other
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        );
        state.complete_pending_open();
        // Opening normalizes the path (macOS spells /var as /private/var).
        let real = std::fs::canonicalize(other.join("n.md")).unwrap();
        assert_eq!(files_of(&state), vec![real]);
        assert!(state.nav.opening.is_none());
    }

    #[test]
    fn a_slower_earlier_open_cannot_replace_a_newer_one() {
        let first = crate::testutil::scratch_dir("files-first");
        let second = crate::testutil::scratch_dir("files-second");
        std::fs::write(first.join("one.md"), "# 1").unwrap();
        std::fs::write(second.join("two.md"), "# 2").unwrap();
        let mut state = State::blank();
        let rules = state.config.scan_rules();
        let _ = state.update(Message::OpenRecent(folder_item(&first)));
        let earlier = (
            state.nav.open_generation,
            OpenRequest::from_recent(&folder_item(&first)),
        );
        let _ = state.update(Message::OpenRecent(folder_item(&second)));
        // The first result arrives late and is ignored; the second is still pending.
        let late = LoadedWorkspace::ready(earlier.1.run(&rules));
        let _ = state.update(Message::WorkspaceOpened(earlier.0, earlier.1, late));
        assert!(matches!(state.mode, Mode::Welcome));
        assert!(state.nav.opening.is_some());
        state.complete_pending_open();
        let real = std::fs::canonicalize(second.join("two.md")).unwrap();
        assert_eq!(files_of(&state), vec![real]);
    }
}
