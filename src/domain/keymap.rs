// SPDX-License-Identifier: GPL-3.0-or-later
//! The single table of keybindings: what each chord means, in which scope,
//! and how to display it. Dispatch (`src/app/update/keyboard.rs`), tooltips,
//! menu hints, the Help dialog, and the README shortcut table all read this
//! table instead of carrying their own copy.

/// Something a chord can trigger. The app layer maps each variant onto a
/// `Message` or a direct `State` method; this module only decides which
/// `Command`, if any, a chord means in a given scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    OpenFile,
    OpenFolder,
    CloseCurrent,
    Quit,
    FindInDocument,
    FindNext,
    FindPrevious,
    FocusFolderSearch,
    ToggleFiles,
    ToggleRightGutter,
    ToggleMinimap,
    ToggleSourceView,
    ShowOutline,
    ShowDetails,
    CycleRightTab,
    CycleLeftTab,
    /// Move focus to the next / previous control inside the open dialog.
    DialogFocusNext,
    DialogFocusPrevious,
    /// Activate the open dialog's safe default button (`iced` buttons
    /// aren't focusable, so Tab can't reach them; this stands in for
    /// clicking whichever button plain Enter would hit in a native dialog).
    DialogDefaultAction,
    NavUp,
    NavDown,
    NavExpand,
    NavCollapse,
    NavToggleFolder,
    ShowShortcuts,
    ReloadTheme,
    /// Rename the file or folder highlighted in the nav.
    RenameItem,
    /// Move the file or folder highlighted in the nav to the Trash.
    DeleteItem,
    /// Close the topmost menu or dialog.
    Dismiss,
    /// Close the Find bar.
    CloseFind,
    /// Clear the folder search while it holds text. (Escape then leaves the field, which the
    /// field does itself, so this binds only the first press.)
    ClearSearch,
    Copy,
    SelectAll,
    FindSelection,
    HistoryBack,
    HistoryForward,
    ScrollPageUp,
    ScrollPageDown,
    /// Grow the preview/source text one step (issue #76).
    ZoomIn,
    /// Shrink the preview/source text one step.
    ZoomOut,
    /// Return the preview/source text to its default size.
    ZoomReset,
}

/// The active input surfaces, innermost first. `State::input_scopes` builds
/// the stack; `resolve` walks it looking for the first scope that binds the
/// chord pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Every mode: quit, open, gutter toggles, help.
    Global,
    /// Reading mode: nav arrows, Tab to fold folders, selection shortcuts.
    View,
    /// The Find bar is open.
    Find,
    /// The folder search input is focused (`State::search_focused`).
    Search,
    /// An in-window menu, the native menu, or the right-click context menu is open.
    Menu,
    /// The About or Shortcuts dialog is open.
    Dialog,
}

/// Display/Help-dialog sectioning, independent of dispatch `Scope`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    App,
    Files,
    Panels,
    Find,
    Navigation,
}

impl Group {
    pub fn label(self) -> &'static str {
        match self {
            Self::App => "App",
            Self::Files => "Files",
            Self::Panels => "Panels",
            Self::Find => "Find",
            Self::Navigation => "Navigation",
        }
    }
}

/// A key with no modifiers attached yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyName {
    Char(char),
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Tab,
    Escape,
    Enter,
    PageUp,
    PageDown,
    F2,
    Delete,
    Backspace,
}

/// A chord: a key plus the modifiers held with it. `primary` is Ctrl on
/// Linux/Windows and ⌘ on macOS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chord {
    pub key: KeyName,
    pub primary: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Chord {
    const fn new(key: KeyName) -> Self {
        Self {
            key,
            primary: false,
            shift: false,
            alt: false,
        }
    }

    const fn primary(key: KeyName) -> Self {
        Self::new(key).with_primary()
    }

    const fn with_primary(mut self) -> Self {
        self.primary = true;
        self
    }

    const fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }

    const fn with_alt(mut self) -> Self {
        self.alt = true;
        self
    }

    /// Renders the chord for display, e.g. `⌘⇧O` on macOS, `Ctrl+Shift+O`
    /// elsewhere.
    pub fn display(self, mac: bool) -> String {
        let mut out = String::new();
        if mac {
            if self.primary {
                out.push('⌘');
            }
            if self.shift {
                out.push('⇧');
            }
            if self.alt {
                out.push('⌥');
            }
            out.push_str(&self.key_label(mac));
        } else {
            let mut parts = Vec::new();
            if self.primary {
                parts.push("Ctrl".to_string());
            }
            if self.shift {
                parts.push("Shift".to_string());
            }
            if self.alt {
                parts.push("Alt".to_string());
            }
            parts.push(self.key_label(mac));
            out.push_str(&parts.join("+"));
        }
        out
    }

    /// Platform-agnostic chord text for prose read by both audiences at
    /// once, e.g. `Ctrl/⌘ Shift O` — the README's existing style. Only the
    /// README drift test below reads this; the UI shows `display` instead.
    #[cfg(test)]
    pub fn display_combined(self) -> String {
        let mut parts = Vec::new();
        if self.primary {
            parts.push("Ctrl/⌘".to_string());
        }
        if self.shift {
            parts.push("Shift".to_string());
        }
        if self.alt {
            parts.push("Alt".to_string());
        }
        parts.push(self.key_label(false));
        parts.join(" ")
    }

    fn key_label(self, mac: bool) -> String {
        match self.key {
            KeyName::Char(c) => c.to_ascii_uppercase().to_string(),
            KeyName::ArrowUp => "↑".to_string(),
            KeyName::ArrowDown => "↓".to_string(),
            KeyName::ArrowLeft => "←".to_string(),
            KeyName::ArrowRight => "→".to_string(),
            KeyName::Tab => "Tab".to_string(),
            KeyName::Escape => "Escape".to_string(),
            KeyName::Enter => {
                if mac {
                    "↩".to_string()
                } else {
                    "Enter".to_string()
                }
            }
            KeyName::PageUp => "Page Up".to_string(),
            KeyName::PageDown => "Page Down".to_string(),
            KeyName::F2 => "F2".to_string(),
            KeyName::Delete => "Delete".to_string(),
            KeyName::Backspace => {
                if mac {
                    "⌫".to_string()
                } else {
                    "Backspace".to_string()
                }
            }
        }
    }
}

/// One entry of the keymap table.
#[derive(Debug, Clone, Copy)]
pub struct Binding {
    pub command: Command,
    pub chord: Chord,
    pub scope: Scope,
    pub group: Group,
    pub label: &'static str,
}

const fn binding(
    command: Command,
    chord: Chord,
    scope: Scope,
    group: Group,
    label: &'static str,
) -> Binding {
    Binding {
        command,
        chord,
        scope,
        group,
        label,
    }
}

/// The keymap: every default binding, in no particular dispatch order.
// The table below reads far better with bare variant names than with 200 prefixes.
#[allow(clippy::enum_glob_use)]
pub fn defaults() -> &'static [Binding] {
    use Command::*;
    use KeyName::*;
    use Scope::*;

    const DEFAULTS: &[Binding] = &[
        binding(
            OpenFile,
            Chord::primary(Char('o')),
            Global,
            Group::Files,
            "Open file",
        ),
        binding(
            OpenFolder,
            Chord::primary(Char('o')).with_shift(),
            Global,
            Group::Files,
            "Open folder",
        ),
        binding(
            CloseCurrent,
            Chord::primary(Char('w')),
            Global,
            Group::Files,
            "Close current",
        ),
        binding(Quit, Chord::primary(Char('q')), Global, Group::App, "Quit"),
        binding(
            ToggleFiles,
            Chord::primary(Char('b')),
            Global,
            Group::Panels,
            "Toggle Files sidebar",
        ),
        binding(
            ToggleRightGutter,
            Chord::primary(Char('b')).with_alt(),
            Global,
            Group::Panels,
            "Toggle right gutter",
        ),
        binding(
            ToggleMinimap,
            Chord::primary(Char('m')).with_shift(),
            Global,
            Group::Panels,
            "Toggle minimap",
        ),
        binding(
            ToggleSourceView,
            Chord::primary(Char('s')).with_alt(),
            Global,
            Group::Panels,
            "View source",
        ),
        binding(
            ShowOutline,
            Chord::primary(Char('1')).with_alt(),
            Global,
            Group::Panels,
            "Show Outline",
        ),
        binding(
            ShowDetails,
            Chord::primary(Char('2')).with_alt(),
            Global,
            Group::Panels,
            "Show Details",
        ),
        binding(
            CycleRightTab,
            Chord::new(PageUp).with_primary(),
            Global,
            Group::Panels,
            "Cycle right tab",
        ),
        binding(
            CycleRightTab,
            Chord::new(PageDown).with_primary(),
            Global,
            Group::Panels,
            "Cycle right tab",
        ),
        binding(
            CycleLeftTab,
            Chord::new(PageUp).with_primary().with_shift(),
            Global,
            Group::Panels,
            "Cycle left tab",
        ),
        binding(
            CycleLeftTab,
            Chord::new(PageDown).with_primary().with_shift(),
            Global,
            Group::Panels,
            "Cycle left tab",
        ),
        binding(
            ShowShortcuts,
            Chord::primary(Char('/')),
            Global,
            Group::App,
            "Show shortcuts",
        ),
        binding(
            ReloadTheme,
            Chord::primary(Char('r')).with_shift(),
            Global,
            Group::App,
            "Reload theme",
        ),
        binding(
            FindInDocument,
            Chord::primary(Char('f')),
            View,
            Group::Find,
            "Find in document",
        ),
        binding(
            FindNext,
            Chord::primary(Char('g')),
            View,
            Group::Find,
            "Next match",
        ),
        binding(
            FindPrevious,
            Chord::primary(Char('g')).with_shift(),
            View,
            Group::Find,
            "Previous match",
        ),
        binding(
            FindNext,
            Chord::primary(Char('g')),
            Find,
            Group::Find,
            "Next match",
        ),
        binding(
            FindPrevious,
            Chord::primary(Char('g')).with_shift(),
            Find,
            Group::Find,
            "Previous match",
        ),
        binding(FindNext, Chord::new(Enter), Find, Group::Find, "Next match"),
        binding(
            FindPrevious,
            Chord::new(Enter).with_shift(),
            Find,
            Group::Find,
            "Previous match",
        ),
        binding(
            CloseFind,
            Chord::new(Escape),
            Find,
            Group::Find,
            "Close Find",
        ),
        binding(
            ClearSearch,
            Chord::new(Escape),
            Search,
            Group::Files,
            "Clear search (again to leave it)",
        ),
        binding(
            Dismiss,
            Chord::new(Escape),
            Menu,
            Group::App,
            "Dismiss menu or dialog",
        ),
        binding(
            DialogFocusNext,
            Chord::new(Tab),
            Dialog,
            Group::App,
            "Next control in dialog",
        ),
        binding(
            DialogFocusPrevious,
            Chord::new(Tab).with_shift(),
            Dialog,
            Group::App,
            "Previous control in dialog",
        ),
        binding(
            Dismiss,
            Chord::new(Escape),
            Dialog,
            Group::App,
            "Dismiss menu or dialog",
        ),
        binding(
            DialogDefaultAction,
            Chord::new(Enter),
            Dialog,
            Group::App,
            "Activate dialog's default button",
        ),
        binding(
            Dismiss,
            Chord::new(Escape),
            Global,
            Group::App,
            "Dismiss menu or dialog",
        ),
        binding(
            FocusFolderSearch,
            Chord::primary(Char('f')).with_shift(),
            View,
            Group::Files,
            "Focus folder search",
        ),
        binding(
            RenameItem,
            Chord::new(F2),
            View,
            Group::Files,
            "Rename selected file or folder",
        ),
        binding(
            DeleteItem,
            Chord::new(Delete),
            View,
            Group::Files,
            "Move selected file or folder to Trash",
        ),
        binding(
            DeleteItem,
            Chord::primary(Backspace),
            View,
            Group::Files,
            "Move selected file or folder to Trash",
        ),
        binding(
            NavDown,
            Chord::new(ArrowDown),
            View,
            Group::Navigation,
            "Move through files",
        ),
        binding(
            NavUp,
            Chord::new(ArrowUp),
            View,
            Group::Navigation,
            "Move through files",
        ),
        binding(
            NavExpand,
            Chord::new(ArrowRight),
            View,
            Group::Navigation,
            "Expand",
        ),
        binding(
            NavCollapse,
            Chord::new(ArrowLeft),
            View,
            Group::Navigation,
            "Collapse",
        ),
        binding(
            NavToggleFolder,
            Chord::new(Tab),
            View,
            Group::Navigation,
            "Toggle folder",
        ),
        binding(
            Copy,
            Chord::primary(Char('c')),
            View,
            Group::Navigation,
            "Copy selected text",
        ),
        binding(
            SelectAll,
            Chord::primary(Char('a')),
            View,
            Group::Navigation,
            "Select all preview text",
        ),
        binding(
            FindSelection,
            Chord::primary(Char('e')),
            View,
            Group::Navigation,
            "Find the selected text",
        ),
        binding(
            HistoryBack,
            Chord::new(ArrowLeft).with_alt(),
            View,
            Group::Navigation,
            "Back",
        ),
        binding(
            HistoryForward,
            Chord::new(ArrowRight).with_alt(),
            View,
            Group::Navigation,
            "Forward",
        ),
        binding(
            HistoryBack,
            Chord::primary(Char('[')),
            View,
            Group::Navigation,
            "Back",
        ),
        binding(
            HistoryForward,
            Chord::primary(Char(']')),
            View,
            Group::Navigation,
            "Forward",
        ),
        binding(
            ScrollPageUp,
            Chord::new(PageUp),
            View,
            Group::Navigation,
            "Scroll up a page",
        ),
        binding(
            ScrollPageDown,
            Chord::new(PageDown),
            View,
            Group::Navigation,
            "Scroll down a page",
        ),
        binding(
            ZoomIn,
            Chord::primary(Char('=')),
            Global,
            Group::Panels,
            "Zoom in",
        ),
        binding(
            ZoomIn,
            Chord::primary(Char('+')).with_shift(),
            Global,
            Group::Panels,
            "Zoom in",
        ),
        binding(
            ZoomOut,
            Chord::primary(Char('-')),
            Global,
            Group::Panels,
            "Zoom out",
        ),
        binding(
            ZoomReset,
            Chord::primary(Char('0')),
            Global,
            Group::Panels,
            "Reset zoom",
        ),
    ];
    DEFAULTS
}

/// The first scope in `scopes` (innermost first) that binds `chord` wins;
/// a narrower scope can shadow a wider one without removing it.
pub fn resolve(scopes: &[Scope], chord: Chord) -> Option<Command> {
    scopes.iter().find_map(|scope| {
        defaults()
            .iter()
            .find(|binding| binding.scope == *scope && binding.chord == chord)
            .map(|binding| binding.command)
    })
}

/// `command`'s first bound chord, if it has one.
pub fn chord_for(command: Command) -> Option<Chord> {
    defaults()
        .iter()
        .find(|binding| binding.command == command)
        .map(|binding| binding.chord)
}

/// The display chord for `command`'s first binding, if it has one — used by
/// tooltips and menu rows.
pub fn label_for(command: Command, mac: bool) -> Option<String> {
    chord_for(command).map(|chord| chord.display(mac))
}

/// Every binding, sectioned by `Group` in a fixed, stable order, one line
/// per distinct chord+label within a section. Shared by the Help dialog and
/// the README drift test so both read the same table.
pub fn help_sections(mac: bool) -> Vec<(&'static str, Vec<String>)> {
    const ORDER: [Group; 5] = [
        Group::App,
        Group::Files,
        Group::Panels,
        Group::Find,
        Group::Navigation,
    ];
    ORDER
        .into_iter()
        .map(|group| {
            let mut lines: Vec<String> = defaults()
                .iter()
                .filter(|binding| binding.group == group)
                .map(|binding| format!("{}  {}", binding.chord.display(mac), binding.label))
                .collect();
            lines.dedup();
            (group.label(), lines)
        })
        .collect()
}

/// The README's quick-reference rows, in display order — a curated subset
/// of `defaults()` (the Help dialog shows everything). The README drift
/// test below checks these against the checked-in README, so an edited
/// chord that isn't also updated there fails the build.
#[cfg(test)]
fn readme_rows() -> Vec<(&'static str, String)> {
    let chord = |command| chord_for(command).expect("README command must have a binding");
    vec![
        ("Open file", chord(Command::OpenFile).display_combined()),
        ("Open folder", chord(Command::OpenFolder).display_combined()),
        (
            "Close current",
            chord(Command::CloseCurrent).display_combined(),
        ),
        (
            "Toggle Files sidebar",
            chord(Command::ToggleFiles).display_combined(),
        ),
        (
            "Toggle right gutter",
            chord(Command::ToggleRightGutter).display_combined(),
        ),
        (
            "Toggle minimap",
            chord(Command::ToggleMinimap).display_combined(),
        ),
        (
            "View source",
            chord(Command::ToggleSourceView).display_combined(),
        ),
        (
            "Zoom in / out / reset",
            format!(
                "{} / {} / {}",
                chord(Command::ZoomIn).display_combined(),
                chord(Command::ZoomOut).display_combined(),
                chord(Command::ZoomReset).display_combined()
            ),
        ),
        (
            "Show Outline",
            chord(Command::ShowOutline).display_combined(),
        ),
        (
            "Show Details",
            chord(Command::ShowDetails).display_combined(),
        ),
        (
            "Find in document",
            chord(Command::FindInDocument).display_combined(),
        ),
        (
            "Next / previous match",
            format!(
                "{} / {}",
                chord(Command::FindNext).display_combined(),
                chord(Command::FindPrevious).display_combined()
            ),
        ),
        (
            "Focus folder search",
            chord(Command::FocusFolderSearch).display_combined(),
        ),
        ("Move through visible files", "↑ / ↓".to_string()),
        (
            "Rename selected file or folder",
            chord(Command::RenameItem).display_combined(),
        ),
        (
            "Move selected file or folder to Trash",
            format!(
                "{} or {}",
                chord(Command::DeleteItem).display_combined(),
                Chord::primary(KeyName::Backspace).display_combined()
            ),
        ),
        (
            "Scroll preview a page",
            format!(
                "{} / {}",
                chord(Command::ScrollPageUp).display_combined(),
                chord(Command::ScrollPageDown).display_combined()
            ),
        ),
        (
            "Select all preview text",
            chord(Command::SelectAll).display_combined(),
        ),
        (
            "Copy selected text",
            format!("{} or Ctrl Insert", chord(Command::Copy).display_combined()),
        ),
        (
            "Find the selected text",
            chord(Command::FindSelection).display_combined(),
        ),
        (
            "Show keyboard shortcuts",
            chord(Command::ShowShortcuts).display_combined(),
        ),
        (
            "Reload theme",
            chord(Command::ReloadTheme).display_combined(),
        ),
        ("Quit", chord(Command::Quit).display_combined()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_chord_is_bound_twice_within_one_scope() {
        for scope in [
            Scope::Global,
            Scope::View,
            Scope::Find,
            Scope::Search,
            Scope::Menu,
            Scope::Dialog,
        ] {
            let mut seen = Vec::new();
            for binding in defaults().iter().filter(|b| b.scope == scope) {
                assert!(
                    !seen.contains(&binding.chord),
                    "{:?} is bound twice in {scope:?}",
                    binding.chord
                );
                seen.push(binding.chord);
            }
        }
    }

    #[test]
    fn a_narrower_scope_shadows_a_wider_one() {
        let escape = Chord::new(KeyName::Escape);
        assert_eq!(
            resolve(&[Scope::Find, Scope::View, Scope::Global], escape),
            Some(Command::CloseFind),
            "Find's Escape must win over Global's Dismiss"
        );
        assert_eq!(
            resolve(&[Scope::View, Scope::Global], escape),
            Some(Command::Dismiss),
            "with no Find scope active, Escape falls through to Global"
        );
    }

    #[test]
    fn escape_in_the_search_field_clears_it_before_anything_wider_acts() {
        let escape = Chord::new(KeyName::Escape);
        assert_eq!(
            resolve(&[Scope::Search, Scope::View, Scope::Global], escape),
            Some(Command::ClearSearch)
        );
        // A menu or dialog above the field still takes Escape first.
        assert_eq!(
            resolve(&[Scope::Menu, Scope::Search, Scope::Global], escape),
            Some(Command::Dismiss)
        );
    }

    #[test]
    fn enter_in_a_dialog_activates_its_default_action() {
        let scopes = [Scope::Dialog, Scope::View, Scope::Global];
        assert_eq!(
            resolve(&scopes, Chord::new(KeyName::Enter)),
            Some(Command::DialogDefaultAction)
        );
    }

    #[test]
    fn tab_in_a_dialog_moves_dialog_focus_instead_of_cycling_the_right_tab() {
        let scopes = [Scope::Dialog, Scope::View, Scope::Global];
        assert_eq!(
            resolve(&scopes, Chord::new(KeyName::Tab)),
            Some(Command::DialogFocusNext)
        );
        assert_eq!(
            resolve(&scopes, Chord::new(KeyName::Tab).with_shift()),
            Some(Command::DialogFocusPrevious)
        );
    }

    #[test]
    fn page_keys_scroll_unmodified_but_cycle_tabs_with_primary() {
        let scopes = [Scope::View, Scope::Global];
        assert_eq!(
            resolve(&scopes, Chord::new(KeyName::PageDown)),
            Some(Command::ScrollPageDown)
        );
        assert_eq!(
            resolve(&scopes, Chord::primary(KeyName::PageDown)),
            Some(Command::CycleRightTab)
        );
    }

    #[test]
    fn zoom_chords_resolve_globally() {
        let scopes = [Scope::View, Scope::Global];
        assert_eq!(
            resolve(&scopes, Chord::primary(KeyName::Char('='))),
            Some(Command::ZoomIn)
        );
        assert_eq!(
            resolve(&scopes, Chord::primary(KeyName::Char('+')).with_shift()),
            Some(Command::ZoomIn)
        );
        assert_eq!(
            resolve(&scopes, Chord::primary(KeyName::Char('-'))),
            Some(Command::ZoomOut)
        );
        assert_eq!(
            resolve(&scopes, Chord::primary(KeyName::Char('0'))),
            Some(Command::ZoomReset)
        );
    }

    #[test]
    fn an_unbound_chord_resolves_to_nothing() {
        let chord = Chord::primary(KeyName::Char('z'));
        assert_eq!(resolve(&[Scope::Global, Scope::View], chord), None);
    }

    #[test]
    fn display_formats_the_primary_modifier_per_platform() {
        let chord = Chord::primary(KeyName::Char('o')).with_shift();
        assert_eq!(chord.display(true), "⌘⇧O");
        assert_eq!(chord.display(false), "Ctrl+Shift+O");
    }

    #[test]
    fn label_for_finds_a_commands_chord() {
        assert_eq!(
            label_for(Command::OpenFile, false).as_deref(),
            Some("Ctrl+O")
        );
        assert_eq!(label_for(Command::OpenFolder, true).as_deref(), Some("⌘⇧O"));
    }

    /// Catches a chord changed in `defaults()` without updating the
    /// README's shortcut table (or vice versa). Only cell content is
    /// compared, not column padding.
    #[test]
    fn readme_shortcut_table_matches_the_keymap() {
        let readme = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"));
        let table_start = readme
            .find("| Action")
            .expect("README must have a shortcut table");
        let mut lines = readme[table_start..].lines().skip(2);
        for (action, shortcut) in readme_rows() {
            let line = lines
                .next()
                .unwrap_or_else(|| panic!("README table is missing a row for {action:?}"));
            let mut cells = line.trim_matches('|').split('|').map(str::trim);
            assert_eq!(
                cells.next(),
                Some(action),
                "README's action text drifted from the keymap"
            );
            assert_eq!(
                cells.next(),
                Some(shortcut.as_str()),
                "README's shortcut drifted from the keymap for {action:?}"
            );
        }
    }
}
