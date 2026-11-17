// SPDX-License-Identifier: GPL-3.0-or-later
use crate::actions::MenuAction;
use crate::app::{LoadedWorkspace, Message};
use crate::config::{Config, ConfigError};
use crate::domain::create::NewItem;
use crate::domain::document::Document;
use crate::domain::folder::expand_file_ancestors;
use crate::domain::history::History;
use crate::domain::minimap::{Block, Geometry};
use crate::domain::recent;
use crate::domain::recent::{RecentItem, RecentKind};
use crate::domain::selection::{BlockId, Selection};
use crate::domain::workspace::{OpenError, OpenRequest, SelectionIssue, Workspace};
use crate::gutter::motion::{Eased, Tween};
use crate::gutter::{GutterSide, GutterState};
use crate::layout::{self, LayoutPlan, Viewport};
use crate::platform::opener::{Opener, SystemOpener};
use crate::ui::find_viewer;
use crate::ui::images::ImageStore;
use crate::ui::theme::{Appearance, AppearanceKind};
use iced::widget::markdown;
use iced::{Point, Task};
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

// `Mode` is one long-lived value in `State`, never stored in bulk, so the Folder
// variant's size costs nothing; boxing it would only add an indirection. Windows
// lays `Document` out just over clippy's 200-byte threshold.
#[allow(clippy::large_enum_variant)]
pub(crate) enum Mode {
    Welcome,
    Folder {
        root: PathBuf,
        files: Vec<PathBuf>,
        query: String,
        selected: Option<Document>,
    },
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Dialog {
    About,
    Shortcuts,
    /// Names the draft in `NavState::draft`; used when the tree has no room for an inline field.
    NewItem,
    /// Asks before moving `NavState::pending_delete` to the Trash.
    ConfirmDelete,
    /// Asks what to do with the linked file in `NavState::pending_external`.
    ExternalLink,
}

#[cfg(not(target_os = "macos"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AppMenu {
    File,
    View,
    Help,
}

pub(crate) struct State {
    pub(crate) mode: Mode,
    pub(crate) find: FindState,
    pub(crate) error: Option<ErrorNotice>,
    pub(crate) config: Config,
    pub(crate) opener: Box<dyn Opener>,
    pub(crate) appearance: Appearance,
    pub(crate) gutters: GutterState,
    /// Whether the toolbar search field has keyboard focus, as last reported by `FocusWatch`.
    pub(crate) search_focused: bool,
    /// The folder search had focus when the open dialog appeared; closing returns it there.
    pub(crate) dialog_restores_search: bool,
    /// How far the search field has expanded over the title: 0 at rest, 1 when focused.
    pub(crate) search_expand: Eased,
    pub(crate) viewport: Viewport,
    pub(crate) nav: NavState,
    pub(crate) minimap: MinimapState,
    pub(crate) recents: Vec<RecentItem>,
    pub(crate) menus: MenuState,
    /// The current drag-selected range over the rendered preview, if any;
    /// persists after mouse-up so it stays visible and copyable.
    pub(crate) selection: Option<Selection>,
    /// Whether the mouse button is currently down over a selectable leaf,
    /// i.e. whether pointer moves should extend `selection`.
    pub(crate) selecting: bool,
    /// Pixels per frame the preview scrolls during a drag near its edge.
    pub(crate) autoscroll: f32,
    /// The plain text of every selectable leaf in the current document, by
    /// `BlockId`, refreshed whenever the document (re)loads; read back by
    /// `Message::CopySelection`.
    pub(crate) leaves: Vec<(BlockId, String)>,
    /// The local images of the open document and their load states.
    pub(crate) images: ImageStore,
    /// The footer's checkbox-save status, if any task-list write is in flight or just finished
    /// or failed (issue #68). Cleared whenever the open document changes, so a message from a
    /// previous file never lingers; otherwise only replaced by the next save event, so an error
    /// stays visible until superseded, per the issue.
    pub(crate) save_status: Option<SaveStatus>,
    /// Whether the open document shows as the rendered preview or its raw Markdown source
    /// (issue #71). Resets to `Preview` whenever a different document is opened, but not when
    /// the open one reloads, so watching a file edit in Source view stays put.
    pub(crate) document_view: DocumentView,
    /// Transient toasts stacked bottom-right (issue #43): confirmations and other
    /// non-blocking, non-error news, as opposed to `error`'s full-width banner for
    /// something that needs a retry. Oldest first; newest toast lands at the bottom
    /// of the stack, closest to its corner.
    pub(crate) notifications: Vec<Notification>,
    /// The next id `State::notify` hands out, so `Message::DismissNotification`
    /// can name a specific toast even as others come and go around it.
    pub(crate) notification_seq: u64,
    /// Multiplies the preview and source text size (issue #76). 1.0 is the
    /// default; `zoom_in`/`zoom_out`/`reset_zoom` keep it within `ZOOM_MIN`/`ZOOM_MAX`.
    pub(crate) zoom: f32,
    /// A metric card's value flashes toward the accent color, then eases back, right after it
    /// is copied; keyed by the metric's label. A finished tween is pruned by `AnimationTick`
    /// rather than left to sit at rest, so this stays empty whenever nothing just flashed.
    pub(crate) copy_flash: HashMap<&'static str, Tween>,
}

/// `State::zoom`'s bounds and step, shared by the keyboard, menu, and tests.
pub(crate) const ZOOM_MIN: f32 = 0.5;
pub(crate) const ZOOM_MAX: f32 = 2.0;
pub(crate) const ZOOM_STEP: f32 = 0.1;

/// A single toast: a message, and an optional action button alongside the always-present
/// dismiss button.
#[derive(Debug, Clone)]
pub(crate) struct Notification {
    pub(crate) id: u64,
    pub(crate) message: String,
    pub(crate) action: Option<(&'static str, Message)>,
}

/// Which way the open document is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum DocumentView {
    #[default]
    Preview,
    Source,
}

/// The footer's checkbox-save status (issue #68), generalized so future document-write features
/// (issue #69's plugin interactions) can reuse the same slot instead of inventing another one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SaveStatus {
    Saving,
    Saved(String),
    Failed(String),
    ReadOnly,
    ExternalChange,
}

impl SaveStatus {
    pub(crate) fn label(&self) -> String {
        match self {
            Self::Saving => "Saving…".to_string(),
            Self::Saved(name) => format!("Saved {name}"),
            Self::Failed(reason) => format!("Save failed: {reason}"),
            Self::ReadOnly => "File is read-only".to_string(),
            Self::ExternalChange => "File changed externally".to_string(),
        }
    }
}

/// What the minimap draws: the preview's scroll geometry and its blocks.
#[derive(Default)]
pub(crate) struct MinimapState {
    pub(crate) geometry: Geometry,
    pub(crate) blocks: Vec<Block>,
    /// The content height the blocks were measured at; a change re-measures.
    pub(crate) measured_height: Option<f32>,
    /// Identifies the latest re-measure request, so earlier debounced ones are dropped.
    pub(crate) measure_generation: u64,
}

/// Folder-tree cursor, expansion, and rescan bookkeeping.
#[derive(Default)]
pub(crate) struct NavState {
    pub(crate) cursor: Option<PathBuf>,
    /// Identifies the latest pending cursor move, so a superseded delayed open is discarded.
    pub(crate) generation: u64,
    pub(crate) scan_in_flight: bool,
    /// How long the last background folder rescan took; sets how often the next one runs.
    pub(crate) last_scan: std::time::Duration,
    /// Identifies the latest open of a folder or file, so a slower earlier one cannot replace it.
    pub(crate) open_generation: u64,
    /// The folder or file being opened in the background, shown in the footer until it is ready.
    pub(crate) opening: Option<OpenRequest>,
    /// Identifies the latest file open, so a slower earlier load cannot replace a newer one.
    pub(crate) load_generation: u64,
    pub(crate) reload_in_flight: bool,
    /// Whether a checkbox-toggle write to the open document is in flight (issue #68); the
    /// live-reload poll skips a document while this is set, just like `reload_in_flight`.
    pub(crate) write_in_flight: bool,
    /// Set when a toggle arrives while `write_in_flight`; the in-flight write's completion
    /// handler notices this and immediately fires another write with the latest text, so rapid
    /// toggles are coalesced rather than lost or overwritten.
    pub(crate) write_dirty: bool,
    pub(crate) expanded: BTreeSet<PathBuf>,
    /// Documents visited this session, for Back and Forward.
    pub(crate) history: History,
    /// The history index a pending Back/Forward load will land on.
    pub(crate) history_target: Option<usize>,
    /// The preview's last read vertical scroll offset, read just before a
    /// navigation so the document being left can be restored later.
    pub(crate) scroll_y: f32,
    /// A new file or folder being named; nothing exists on disk until it is committed.
    pub(crate) draft: Option<NewItem>,
    /// Whether the pointer is over the draft's field, so a press elsewhere can be told apart.
    pub(crate) draft_hover: bool,
    /// The file the delete dialog is asking about.
    pub(crate) pending_delete: Option<PathBuf>,
    /// The linked Markdown file, outside the open folder, that the external-link dialog is about.
    pub(crate) pending_external: Option<PathBuf>,
    /// The external-link dialog's "Always do this" checkbox.
    pub(crate) external_always: bool,
}

impl NavState {
    /// Makes any pending delayed open stale.
    pub(crate) fn invalidate_pending(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }
}

/// Open menus and dialogs.
#[derive(Default)]
pub(crate) struct MenuState {
    pub(crate) dialog: Option<Dialog>,
    #[cfg(not(target_os = "macos"))]
    pub(crate) active: Option<AppMenu>,
    #[cfg(not(target_os = "macos"))]
    pub(crate) recent_open: bool,
    pub(crate) context: Option<ContextMenu>,
    /// The link under a right-click, reported by the preview leaf just before
    /// the surrounding area opens the menu; consumed by that open.
    pub(crate) context_link: Option<markdown::Uri>,
    /// The drawn image under a right-click, reported the same way as `context_link`.
    pub(crate) context_image: Option<PathBuf>,
    /// The details panel's "Copy All" format, shown as its type indicator; changed by picking
    /// another format from its right-click dropdown.
    pub(crate) stats_format: crate::domain::stats::StatsFormat,
}

/// An open right-click menu: where it was opened and what it offers.
pub(crate) struct ContextMenu {
    pub(crate) position: Point,
    /// For a dropdown, the right edge of the control it hangs from: when the menu would cross
    /// the window's right edge it right-aligns to this instead of flipping a whole menu-width
    /// left of the control.
    pub(crate) anchor_right: Option<f32>,
    pub(crate) entries: Vec<ContextEntry>,
}

#[derive(Debug, Clone)]
pub(crate) enum ContextEntry {
    /// A row; a `None` message renders it disabled.
    Item {
        label: std::borrow::Cow<'static, str>,
        /// An `ui::icons` fragment drawn in the leading column.
        icon: Option<&'static str>,
        shortcut: Option<&'static str>,
        /// Draws a check in the leading column when there is no icon.
        checked: bool,
        message: Option<Message>,
        /// Why the row is disabled; drawn as an (i) icon whose tooltip says it.
        info: Option<std::borrow::Cow<'static, str>>,
    },
    Divider,
}

#[derive(Default)]
pub(crate) struct FindState {
    pub(crate) visible: bool,
    pub(crate) query: String,
    pub(crate) matches: Vec<usize>,
    pub(crate) active: usize,
}

pub(crate) struct ErrorNotice {
    pub(crate) message: String,
    pub(crate) retry: Option<Message>,
}

/// Says that part of the opened tree could not be read, naming the first folder.
fn unreadable_folders_notice(unreadable: &[PathBuf]) -> Option<ErrorNotice> {
    let first = unreadable.first()?;
    let name = first
        .file_name()
        .unwrap_or(first.as_os_str())
        .to_string_lossy();
    Some(ErrorNotice::new(
        match unreadable.len() {
            1 => format!("Couldn't read the folder “{name}”, so its files are not listed."),
            count => format!(
                "Couldn't read {count} folders, including “{name}”, so their files are not listed."
            ),
        },
        None,
    ))
}

impl ErrorNotice {
    pub(crate) fn new(message: impl std::fmt::Display, retry: Option<Message>) -> Self {
        Self {
            message: message.to_string(),
            retry,
        }
    }
}

/// Detects the OS light/dark preference once at launch. Falls back to the
/// dark palette (the app's original look) when the platform can't say —
/// headless Linux/CI environments included.
pub(crate) fn detect_appearance() -> AppearanceKind {
    match dark_light::detect() {
        Ok(dark_light::Mode::Light) => AppearanceKind::Paper,
        _ => AppearanceKind::Slate,
    }
}

impl State {
    pub(crate) fn new() -> Self {
        let loaded = Config::load();
        let (config, config_error) = match loaded {
            Ok(config) => (config, None),
            Err(error) => (Config::default(), Some(error)),
        };
        let system = detect_appearance();
        let (appearance, config_error) = match Appearance::from_config(&config, system) {
            Ok(appearance) => (appearance, config_error),
            Err(error) => (
                match system {
                    AppearanceKind::Slate => Appearance::SLATE,
                    AppearanceKind::Harbor => Appearance::HARBOR,
                    AppearanceKind::Nightfall => Appearance::NIGHTFALL,
                    AppearanceKind::Linen => Appearance::LINEN,
                    AppearanceKind::Paper => Appearance::PAPER,
                },
                Some(ConfigError::from(error)),
            ),
        };
        let state = Self {
            mode: Mode::Welcome,
            find: FindState::default(),
            error: config_error.map(|error| {
                ErrorNotice::new(error, Some(Message::MenuAction(MenuAction::ReloadTheme)))
            }),
            config,
            opener: Box::new(SystemOpener),
            appearance,
            gutters: GutterState::default(),
            search_focused: false,
            dialog_restores_search: false,
            search_expand: Eased::default(),
            viewport: Viewport::default(),
            nav: NavState::default(),
            minimap: MinimapState::default(),
            recents: recent::load(),
            menus: MenuState::default(),
            selection: None,
            selecting: false,
            autoscroll: 0.0,
            leaves: Vec::new(),
            images: ImageStore::default(),
            save_status: None,
            document_view: DocumentView::default(),
            notifications: Vec::new(),
            notification_seq: 0,
            zoom: 1.0,
            copy_flash: HashMap::new(),
        };

        // Finder hands over files to open as an Apple Event, which may arrive before the first
        // tick; register for it as early as possible.
        #[cfg(target_os = "macos")]
        crate::platform::macos_open::install();

        state
    }

    /// What a path on the command line asks to open, if it names a file or folder. Opened from
    /// `boot`, after the window exists, so a large folder does not delay it.
    pub(crate) fn startup_request() -> Option<OpenRequest> {
        let item = Self::recent_for_path(PathBuf::from(std::env::args().nth(1)?))?;
        Some(OpenRequest::from_recent(&item))
    }

    /// The recent-item to open for `path`, a file or a folder; `None` if it is neither.
    pub(crate) fn recent_for_path(path: PathBuf) -> Option<RecentItem> {
        let kind = if path.is_dir() {
            RecentKind::Folder
        } else if path.is_file() {
            RecentKind::File
        } else {
            return None;
        };
        Some(RecentItem {
            kind,
            path,
            selected: None,
        })
    }

    /// A `State` with no disk access, for tests.
    #[cfg(test)]
    pub(crate) fn blank() -> Self {
        Self {
            mode: Mode::Welcome,
            find: FindState::default(),
            error: None,
            config: Config::default(),
            opener: Box::new(crate::testutil::RecordingOpener::default()),
            appearance: Appearance::SLATE,
            gutters: GutterState::default(),
            search_focused: false,
            dialog_restores_search: false,
            search_expand: Eased::default(),
            viewport: Viewport::default(),
            nav: NavState::default(),
            minimap: MinimapState::default(),
            recents: Vec::new(),
            menus: MenuState::default(),
            selection: None,
            selecting: false,
            autoscroll: 0.0,
            leaves: Vec::new(),
            images: ImageStore::default(),
            save_status: None,
            document_view: DocumentView::default(),
            notifications: Vec::new(),
            notification_seq: 0,
            zoom: 1.0,
            copy_flash: HashMap::new(),
        }
    }

    /// Queues a toast. `action`, if given, pairs a label with the message its button sends.
    pub(crate) fn notify(
        &mut self,
        message: impl Into<String>,
        action: Option<(&'static str, Message)>,
    ) {
        self.notification_seq += 1;
        self.notifications.push(Notification {
            id: self.notification_seq,
            message: message.into(),
            action,
        });
    }

    /// Queues a toast with no action button that dismisses itself after `duration`, for a quick
    /// confirmation (such as "Copied to clipboard.") that doesn't need the user to close it.
    pub(crate) fn notify_expiring(
        &mut self,
        message: impl Into<String>,
        duration: std::time::Duration,
    ) -> Task<Message> {
        self.notify(message, None);
        let id = self.notification_seq;
        Task::perform(
            async move { tokio::time::sleep(duration).await },
            move |()| Message::DismissNotification(id),
        )
    }

    pub(crate) fn zoom_in(&mut self) {
        self.zoom = Self::round_zoom((self.zoom + ZOOM_STEP).min(ZOOM_MAX));
    }

    pub(crate) fn zoom_out(&mut self) {
        self.zoom = Self::round_zoom((self.zoom - ZOOM_STEP).max(ZOOM_MIN));
    }

    /// Keeps repeated `zoom_in`/`zoom_out` presses on a clean step instead of drifting
    /// from `f32` rounding (1.0 + 0.1 + 0.1 + … is not exactly 1.3).
    fn round_zoom(zoom: f32) -> f32 {
        (zoom * 10.0).round() / 10.0
    }

    pub(crate) fn reset_zoom(&mut self) {
        self.zoom = 1.0;
    }

    pub(crate) fn remember(&mut self, kind: RecentKind, path: PathBuf, selected: Option<PathBuf>) {
        recent::remember(
            &mut self.recents,
            RecentItem {
                kind,
                path: recent::absolute(&path),
                selected: selected.map(|path| recent::absolute(&path)),
            },
        );
        recent::save(&self.recents);
        #[cfg(target_os = "macos")]
        crate::platform::native_menu::refresh(&self.recents);
    }

    /// Starts opening a folder or file on a worker, so a big tree does not freeze the window. The
    /// result arrives as `Message::WorkspaceOpened`; until then the current view stays and the
    /// footer says what is opening.
    pub(crate) fn request_open(&mut self, request: OpenRequest) -> Task<Message> {
        self.nav.open_generation = self.nav.open_generation.wrapping_add(1);
        let generation = self.nav.open_generation;
        self.nav.opening = Some(request.clone());
        let rules = self.config.scan_rules();
        Task::perform(
            LoadedWorkspace::load(request.clone(), rules),
            move |loaded| Message::WorkspaceOpened(generation, request, loaded),
        )
    }

    /// Shows the opened workspace, or reports why it could not be opened, with a way to retry.
    pub(crate) fn finish_open(
        &mut self,
        request: OpenRequest,
        result: Result<Workspace, OpenError>,
    ) {
        self.nav.opening = None;
        match result {
            Ok(workspace) => {
                let opened = match request {
                    OpenRequest::File(_) => workspace.selected.as_ref().map(|doc| doc.path.clone()),
                    OpenRequest::Folder { .. } => None,
                };
                self.show_workspace(workspace);
                if let Some(opened) = opened {
                    self.remember(RecentKind::File, opened, None);
                }
            }
            Err(err) => {
                let retry = match request {
                    OpenRequest::File(path) => Message::FileOpened(Some(path)),
                    OpenRequest::Folder { root, selected } => Message::OpenRecent(RecentItem {
                        kind: RecentKind::Folder,
                        path: root,
                        selected,
                    }),
                };
                self.error = Some(ErrorNotice::new(err.to_string(), Some(retry)));
            }
        }
    }

    /// Finishes the open in progress right away, for tests, which cannot run a `Task`.
    #[cfg(test)]
    pub(crate) fn complete_pending_open(&mut self) {
        let request = self.nav.opening.clone().expect("an open should be pending");
        let result = request.run(&self.config.scan_rules());
        let generation = self.nav.open_generation;
        let _ = self.update(Message::WorkspaceOpened(
            generation,
            request,
            LoadedWorkspace::ready(result),
        ));
    }

    pub(crate) fn show_workspace(&mut self, workspace: Workspace) {
        let Workspace {
            root,
            files,
            selected,
            issue,
            unreadable,
        } = workspace;
        self.find = FindState::default();
        let restore_error = issue.map(|issue| {
            let retry = match &issue {
                SelectionIssue::Unreadable(error) => {
                    Some(Message::FileSelected(error.path.clone()))
                }
                SelectionIssue::Unavailable(_) => None,
            };
            ErrorNotice::new(issue.to_string(), retry)
        });
        let restore_error = restore_error.or_else(|| unreadable_folders_notice(&unreadable));
        let selected_path = selected.as_ref().map(|doc| doc.path.clone());
        self.nav.expanded.clear();
        self.nav.draft = None;
        if let Some(doc) = &selected_path {
            expand_file_ancestors(&root, doc, &mut self.nav.expanded);
        }
        self.remember(RecentKind::Folder, root.clone(), selected_path.clone());
        self.mode = Mode::Folder {
            root,
            files,
            query: String::new(),
            selected,
        };
        self.gutters.reveal(GutterSide::Left);
        self.nav.history.clear();
        self.nav.history_target = None;
        self.nav.scroll_y = 0.0;
        if let Some(path) = &selected_path {
            self.nav.history.visit(path.clone());
        }
        self.nav.cursor = selected_path;
        self.nav.invalidate_pending();
        self.nav.scan_in_flight = false;
        self.nav.last_scan = std::time::Duration::ZERO;
        self.error = restore_error;
        self.refresh_selection();
    }

    /// Where each panel sits at the current window width.
    pub(crate) fn layout_plan(&self) -> LayoutPlan {
        layout::plan(self.viewport, self.gutters.intent())
    }

    pub(crate) fn title(&self) -> String {
        match &self.mode {
            Mode::Welcome => "Markdown Leaflet".to_string(),
            Mode::Folder {
                selected: Some(doc),
                ..
            } => format!("{} — Markdown Leaflet", doc.display_title()),
            Mode::Folder { root, .. } => format!(
                "{} — Markdown Leaflet",
                root.file_name().unwrap_or_default().to_string_lossy()
            ),
        }
    }

    /// The document currently on screen, regardless of mode — used by the
    /// toolbar actions (Open With, Share, Inspector) that don't care how
    /// the user got here.
    /// The open folder's root, when a folder (or a file's parent) is open.
    pub(crate) fn root(&self) -> Option<&std::path::Path> {
        match &self.mode {
            Mode::Welcome => None,
            Mode::Folder { root, .. } => Some(root),
        }
    }

    pub(crate) fn current_document(&self) -> Option<&Document> {
        match &self.mode {
            Mode::Welcome => None,
            Mode::Folder { selected, .. } => selected.as_ref(),
        }
    }

    /// The active input surfaces, innermost first, for `keymap::resolve`.
    /// Mirrors today's Escape precedence: a context/app menu closes before
    /// a dialog would, and either closes before the Find bar or a leftover
    /// selection. `View` and `Global` are always present.
    pub(crate) fn input_scopes(&self) -> Vec<crate::domain::keymap::Scope> {
        use crate::domain::keymap::Scope;
        let mut scopes = Vec::with_capacity(5);
        #[cfg(not(target_os = "macos"))]
        let menu_open =
            self.menus.context.is_some() || self.menus.active.is_some() || self.menus.recent_open;
        #[cfg(target_os = "macos")]
        let menu_open = self.menus.context.is_some();
        if menu_open {
            scopes.push(Scope::Menu);
        }
        if self.menus.dialog.is_some() {
            scopes.push(Scope::Dialog);
        }
        // The focused search field is the keyboard's innermost surface after a menu or dialog,
        // so it also outranks an open Find bar.
        if self.search_focused {
            scopes.push(Scope::Search);
        }
        if self.find.visible {
            scopes.push(Scope::Find);
        }
        scopes.push(Scope::View);
        scopes.push(Scope::Global);
        scopes
    }

    /// Clears any selection and recomputes `leaves` from the current
    /// document, called whenever it's replaced (opened or reloaded) since a
    /// stale selection or leaf text would no longer line up with it, or
    /// whenever `document_view` toggles, since Preview and Source don't
    /// share a leaf numbering.
    pub(crate) fn refresh_selection(&mut self) {
        self.menus.context = None;
        self.selection = None;
        self.selecting = false;
        self.autoscroll = 0.0;
        self.leaves = self
            .current_document()
            .map(|doc| match self.document_view {
                DocumentView::Preview => find_viewer::leaf_texts(&doc.items, self.appearance),
                DocumentView::Source => crate::ui::source_view::leaf_texts(&doc.raw),
            })
            .unwrap_or_default();
    }

    /// Whether the open document is plain text (issue #79), which has no rendered form to
    /// switch away from.
    pub(crate) fn is_text_document(&self) -> bool {
        self.current_document()
            .is_some_and(|doc| doc.kind == crate::domain::document::DocumentKind::Text)
    }

    /// Switches between the rendered preview and the raw Markdown source (issue #71), redoing
    /// selection and find state, since neither carries over between the two leaf numberings.
    pub(crate) fn toggle_document_view(&mut self) {
        if self.is_text_document() {
            return;
        }
        self.document_view = match self.document_view {
            DocumentView::Preview => DocumentView::Source,
            DocumentView::Source => DocumentView::Preview,
        };
        self.refresh_selection();
        self.refresh_find();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_steps_and_clamps_at_both_ends() {
        let mut state = State::blank();
        assert_eq!(state.zoom, 1.0);
        state.zoom_in();
        assert_eq!(state.zoom, 1.1);
        state.zoom_out();
        state.zoom_out();
        assert_eq!(state.zoom, 0.9);
        state.zoom = ZOOM_MAX;
        state.zoom_in();
        assert_eq!(state.zoom, ZOOM_MAX, "zoom in doesn't pass the ceiling");
        state.zoom = ZOOM_MIN;
        state.zoom_out();
        assert_eq!(state.zoom, ZOOM_MIN, "zoom out doesn't pass the floor");
        state.zoom = 1.6;
        state.reset_zoom();
        assert_eq!(state.zoom, 1.0);
    }

    #[test]
    fn invalidating_pending_changes_the_generation() {
        let mut nav = NavState::default();
        let before = nav.generation;
        nav.invalidate_pending();
        assert_ne!(nav.generation, before);
    }

    #[test]
    fn a_requested_path_opens_as_a_file_or_folder_and_a_missing_one_is_refused() {
        let dir = crate::testutil::scratch_dir("recent-for");
        let file = dir.join("a.md");
        std::fs::write(&file, "# a").unwrap();
        let folder = State::recent_for_path(dir.to_path_buf()).expect("a folder opens");
        assert_eq!(
            (folder.kind, folder.path),
            (RecentKind::Folder, dir.to_path_buf())
        );
        let item = State::recent_for_path(file.clone()).expect("a file opens");
        assert_eq!((item.kind, item.path), (RecentKind::File, file));
        assert!(State::recent_for_path(dir.join("gone.md")).is_none());
    }

    #[test]
    fn unreadable_folders_are_named_in_a_notice_and_a_clean_scan_has_none() {
        assert!(unreadable_folders_notice(&[]).is_none());
        let one = unreadable_folders_notice(&[PathBuf::from("/notes/private")]).unwrap();
        assert!(
            one.message.contains("“private”") && one.message.contains("Couldn't read the folder")
        );
        let many =
            unreadable_folders_notice(&[PathBuf::from("/a/x"), PathBuf::from("/a/y")]).unwrap();
        assert!(many.message.contains("2 folders") && many.message.contains("“x”"));
    }

    #[test]
    fn opening_a_tree_with_an_unreadable_folder_shows_the_notice() {
        let mut state = State::blank();
        let mut ws = crate::domain::workspace::Workspace::new(PathBuf::from("/notes"), vec![]);
        ws.unreadable = vec![PathBuf::from("/notes/private")];
        state.show_workspace(ws);
        assert!(state.error.as_ref().unwrap().message.contains("“private”"));
        state.show_workspace(crate::domain::workspace::Workspace::new(
            PathBuf::from("/notes"),
            vec![],
        ));
        assert!(state.error.is_none());
    }
}
