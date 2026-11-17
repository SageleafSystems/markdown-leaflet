// SPDX-License-Identifier: GPL-3.0-or-later
use crate::actions::MenuAction;
#[cfg(not(target_os = "macos"))]
use crate::app::AppMenu;
use crate::domain::document::{self, Document, DocumentError, SaveError};
use crate::domain::editors::EditorOption;
use crate::domain::recent::RecentItem;
use crate::domain::selection::BlockId;
use crate::domain::tasks::TaskId;
use crate::domain::walk::ScanRules;
use crate::domain::workspace::{OpenError, OpenRequest, Workspace};
use crate::gutter::{GutterSide, LeftTab, RightTab};
use crate::ui::images::{ImageMenu, Loaded};
use crate::ui::selectable_text::SelectionEvent;
use iced::widget::markdown;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// A document read and parsed off the UI thread. `Message` must be `Clone`, so the result sits
/// behind a shared slot that the handler takes it from once.
#[derive(Debug, Clone)]
pub(crate) struct LoadedDocument(Arc<Mutex<Option<Result<Document, DocumentError>>>>);

impl LoadedDocument {
    /// Reads and parses `path` on a blocking thread.
    pub(crate) async fn load(path: PathBuf) -> Self {
        let result = tokio::task::spawn_blocking({
            let path = path.clone();
            move || Document::open(path)
        })
        .await
        .unwrap_or_else(|error| {
            Err(DocumentError {
                path,
                source: std::io::Error::other(error),
            })
        });
        Self(Arc::new(Mutex::new(Some(result))))
    }

    pub(crate) fn take(&self) -> Option<Result<Document, DocumentError>> {
        self.0.lock().ok()?.take()
    }
}

/// A workspace opened off the interface thread: the scan and the document read can take long on a
/// big folder. Like [`LoadedDocument`], the result sits behind a shared slot because `Message` must
/// be `Clone`.
#[derive(Debug, Clone)]
pub(crate) struct LoadedWorkspace(Arc<Mutex<Option<Result<Workspace, OpenError>>>>);

impl LoadedWorkspace {
    /// Runs `request` on a blocking thread.
    pub(crate) async fn load(request: OpenRequest, rules: ScanRules) -> Self {
        let target = request.clone();
        let result = tokio::task::spawn_blocking(move || request.run(&rules))
            .await
            .unwrap_or_else(|error| {
                Err(OpenError::Unreadable(DocumentError {
                    path: PathBuf::from(target.label()),
                    source: std::io::Error::other(error),
                }))
            });
        Self::ready(result)
    }

    /// A result that is already in hand.
    pub(crate) fn ready(result: Result<Workspace, OpenError>) -> Self {
        Self(Arc::new(Mutex::new(Some(result))))
    }

    pub(crate) fn take(&self) -> Option<Result<Workspace, OpenError>> {
        self.0.lock().ok()?.take()
    }
}

/// A checkbox write finished (or failed) on a blocking thread. Like [`LoadedDocument`], the
/// result sits behind a shared slot because `Message` must be `Clone`.
#[derive(Debug, Clone)]
pub(crate) struct SavedDocument(Arc<Mutex<Option<Result<SystemTime, SaveError>>>>);

impl SavedDocument {
    /// Writes `raw` to `path` on a blocking thread (see `domain::document::save`).
    pub(crate) async fn save(
        path: PathBuf,
        raw: String,
        expected_mtime: Option<SystemTime>,
    ) -> Self {
        let result =
            tokio::task::spawn_blocking(move || document::save(&path, &raw, expected_mtime))
                .await
                .unwrap_or_else(|error| Err(SaveError::Io(std::io::Error::other(error))));
        Self::ready(result)
    }

    /// A result that is already in hand, for tests.
    pub(crate) fn ready(result: Result<SystemTime, SaveError>) -> Self {
        Self(Arc::new(Mutex::new(Some(result))))
    }

    pub(crate) fn take(&self) -> Option<Result<SystemTime, SaveError>> {
        self.0.lock().ok()?.take()
    }
}

/// A checkbox was clicked; `FindViewer` fires this instead of toggling anything itself.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TaskToggleEvent(pub(crate) TaskId);

/// What was right-clicked, which decides the entries of the context menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ContextTarget {
    Preview,
    /// A file row in the nav.
    NavFile(PathBuf),
    /// A folder row in the nav.
    NavFolder(PathBuf),
    /// Empty space in the nav, which acts on the open folder's root.
    NavBlank,
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    OpenFileClicked,
    OpenFolderClicked,
    OpenRecent(RecentItem),
    FileOpened(Option<PathBuf>),
    FolderOpened(Option<(PathBuf, crate::domain::walk::Scan)>),
    FolderPoll,
    /// The nav menu's "Show Text Files" row: list `.txt` files alongside Markdown, or stop.
    ToggleShowTextFiles,
    FolderScanned(PathBuf, Vec<PathBuf>, std::time::Duration),
    /// A folder or file finished opening in the background. The number identifies the request, so
    /// a slower earlier open cannot replace a newer one.
    WorkspaceOpened(u64, OpenRequest, LoadedWorkspace),
    FileSelected(PathBuf),
    /// A document requested by `FileSelected` finished loading; the number is its request.
    DocumentLoaded(u64, LoadedDocument),
    /// The open document was re-read after changing on disk.
    DocumentReloaded(LoadedDocument),
    NavCommit(u64, PathBuf),
    ToggleFolder(PathBuf),
    NavCursorLocated(Option<f32>),
    SearchChanged(String),
    FindChanged(String),
    FindNext,
    FindPrevious,
    CloseFind,
    FindLocated(Option<f32>),
    JumpToHeading(usize),
    HeadingLocated(Option<f32>),
    LinkClicked(markdown::Uri),
    /// An image was right-clicked; its menu opens at the position.
    ContextImagePressed(PathBuf),
    /// Open an image file with the system's default program.
    OpenImage(PathBuf),
    CopyImagePath(PathBuf),
    /// A local image finished decoding (or failed); the time identifies the file version it read.
    ImageLoaded(
        PathBuf,
        Option<std::time::SystemTime>,
        Result<Loaded, String>,
    ),
    /// Back in the session history.
    HistoryBack,
    /// Forward in the session history.
    HistoryForward,
    /// The preview's scroll offset, read just before a navigation.
    ScrollRead(Option<f32>),
    /// The preview scrolled, resized, or its content changed height.
    PreviewScrolled(iced::widget::scrollable::Viewport),
    /// The reader pressed or dragged the minimap at this y, in its own pixels.
    MinimapSeek(f32),
    /// Item index, top and height of each rendered block, read for the minimap.
    BlocksMeasured(crate::ui::measure_blocks::Measurement),
    /// A debounced re-measure of the preview came due; the number is the latest request.
    MeasureDue(u64),
    ReloadPoll,
    /// A task-list checkbox was clicked.
    TaskToggled(TaskId),
    /// A checkbox write finished; the path identifies which document, so a save that outlasts
    /// a document switch is ignored instead of touching the wrong one.
    DocumentSaved(PathBuf, SavedDocument),
    KeyPressed(iced::keyboard::Key, iced::keyboard::Modifiers, bool),
    TogglePanel(GutterSide),
    SelectRightTab(RightTab),
    SelectLeftTab(LeftTab),
    OpenWith(EditorOption),
    CopySource,
    SelectionStarted(BlockId, usize),
    SelectionExtended(BlockId, usize),
    /// Shift+click: move the selection's head, keeping its anchor.
    SelectionExtendedTo(BlockId, usize),
    /// Double- or triple-click selected this byte range of one leaf.
    SelectionRange(BlockId, usize, usize),
    SelectionEnded,
    SelectionCleared,
    /// Signed pixels per frame to scroll while a drag holds the pointer near
    /// or past the preview's top or bottom edge; 0 stops.
    SelectionAutoScroll(f32),
    AutoScrollTick,
    CopySelection,
    SelectAll,
    FindSelection,
    CopyLink(markdown::Uri),
    /// A preview leaf was right-clicked over this link; the menu opens next.
    ContextLinkPressed(markdown::Uri),
    OpenContextMenu(ContextTarget, iced::Point),
    /// An entry of the open context menu was chosen, by index.
    ContextMenuPick(usize),
    DismissContextMenu,
    /// A press on the menu panel itself (a disabled row or padding): swallowed so the
    /// click-away layer underneath does not close the menu.
    ContextMenuInert,
    /// The combined Open button was right-clicked; the rectangle is the button's bounds.
    OpenFileOrFolderDropdown(iced::Rectangle),
    /// The toolbar's Theme button was right-clicked; the rectangle is the button's bounds.
    OpenThemeDropdown(iced::Rectangle),
    /// The toolbar's Font button was pressed; the rectangle is the button's bounds.
    OpenFontDropdown(iced::Rectangle),
    /// The nav's New button was pressed; the rectangle is the button's bounds.
    OpenNewDropdown(iced::Rectangle),
    /// Start naming a new Markdown file in this folder.
    NewMarkdownFile(PathBuf),
    /// Start naming a new folder (created with a blank Markdown file) in this folder.
    NewFolder(PathBuf),
    /// Start renaming this file or folder in place.
    RenameItem(PathBuf),
    /// Ask whether to move this file or folder to the Trash.
    DeleteItem(PathBuf),
    /// The delete dialog's confirm button.
    ConfirmDelete,
    CancelDelete,
    /// The external-link dialog's "Always do this" checkbox.
    ExternalLinkAlways(bool),
    /// Open the linked file's folder as the workspace.
    ExternalLinkOpenFolder,
    /// Open the linked file alone, leaving the open folder as it is.
    ExternalLinkOpenDocument,
    /// Open the linked file in another editor.
    ExternalLinkOpenWith(EditorOption),
    CancelExternalLink,
    NewItemChanged(String),
    /// Enter: create the item and open it.
    NewItemSubmit,
    /// A press outside the field: create the item if its name is valid, else discard it.
    NewItemBlur,
    /// Escape or Cancel: discard the draft.
    NewItemCancel,
    NewItemHover(bool),
    /// Show this path in the system file manager.
    RevealInFileManager(PathBuf),
    /// Open this file in a new app window.
    OpenInNewWindow(PathBuf),
    /// Open this file with the system's default program.
    OpenExternally(PathBuf),
    CopyPath(PathBuf),
    /// The toolbar's Open With button was pressed; the rectangle is the button's bounds.
    OpenEditorDropdown(iced::Rectangle),
    /// A details-panel metric card was pressed anywhere on it: copies its "Label: value" pair
    /// and flashes its value toward the accent color. The label also keys the flash in
    /// `State::copy_flash`.
    CopyMetric(&'static str, String),
    /// Copies every metric in this shape, from the details panel's "Copy All" button; also
    /// becomes its new type indicator and default for the next press.
    CopyAllStats(crate::domain::stats::StatsFormat),
    /// The details panel's "Copy All" button was right-clicked; the rectangle is the button's
    /// bounds.
    OpenMetricsFormatDropdown(iced::Rectangle),
    ResizeDragStart(GutterSide, f32),
    WindowResized(iced::Size),
    SidebarDragged(f32),
    SidebarDragEnd,
    ResizeZoneEntered(GutterSide),
    ResizeZoneExited(GutterSide),
    AnimationTick,
    /// The toolbar search field gained or lost keyboard focus.
    SearchFocused(bool),
    /// Clear the toolbar search and keep keyboard focus in it (the × button, or Escape with text).
    SearchCleared,
    DismissError,
    /// Dismiss the toast with this id (issue #43).
    DismissNotification(u64),
    MenuAction(MenuAction),
    #[cfg(not(target_os = "macos"))]
    ToggleMenu(AppMenu),
    #[cfg(not(target_os = "macos"))]
    ActivateMenu(AppMenu),
    #[cfg(not(target_os = "macos"))]
    OpenRecentMenu,
    #[cfg(not(target_os = "macos"))]
    CloseRecentMenu,
    #[cfg(not(target_os = "macos"))]
    DismissMenu,
    DismissDialog,
    #[cfg(target_os = "macos")]
    NativeMenuTick,
}

impl From<markdown::Uri> for Message {
    fn from(uri: markdown::Uri) -> Self {
        Self::LinkClicked(uri)
    }
}

impl From<TaskToggleEvent> for Message {
    fn from(event: TaskToggleEvent) -> Self {
        Self::TaskToggled(event.0)
    }
}

impl From<ImageMenu> for Message {
    fn from(menu: ImageMenu) -> Self {
        Self::ContextImagePressed(menu.path)
    }
}

impl From<SelectionEvent> for Message {
    fn from(event: SelectionEvent) -> Self {
        match event {
            SelectionEvent::Start(block, offset) => Self::SelectionStarted(block, offset),
            SelectionEvent::Extend(block, offset) => Self::SelectionExtended(block, offset),
            SelectionEvent::ExtendTo(block, offset) => Self::SelectionExtendedTo(block, offset),
            SelectionEvent::Range(block, start, end) => Self::SelectionRange(block, start, end),
            SelectionEvent::End => Self::SelectionEnded,
            SelectionEvent::ContextLink(uri) => Self::ContextLinkPressed(uri),
        }
    }
}
