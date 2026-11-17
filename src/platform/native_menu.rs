// SPDX-License-Identifier: GPL-3.0-or-later
//! macOS menu bar adapter. All actions still flow through `State::update`.

use crate::actions::MenuAction;
use crate::config::{AppearanceChoice, FontChoice};
use crate::domain::keymap::{self, Command, KeyName};
use crate::domain::recent::RecentItem;
use muda::accelerator::{Accelerator, CMD_OR_CTRL, Code, Modifiers};
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use std::cell::{Cell, RefCell};

struct NativeMenu {
    _menu: Menu,
    recent: Submenu,
    recent_items: Vec<MenuItem>,
}

thread_local! {
    static MENU: RefCell<Option<NativeMenu>> = const { RefCell::new(None) };
    static ATTEMPTED: Cell<bool> = const { Cell::new(false) };
}

fn item(id: &'static str, label: &'static str) -> MenuItem {
    MenuItem::with_id(id, label, true, None)
}

/// A menu item with its accelerator taken from `command`'s keymap chord,
/// when it has one that a menu accelerator can express (a single printable
/// key, as today's commands all use).
fn item_for(id: &'static str, label: &'static str, command: Command) -> MenuItem {
    MenuItem::with_id(id, label, true, accelerator_for(command))
}

fn accelerator_for(command: Command) -> Option<Accelerator> {
    let chord = keymap::chord_for(command)?;
    let KeyName::Char(c) = chord.key else {
        return None;
    };
    let code = match c {
        'a'..='z' => {
            const LETTERS: [Code; 26] = [
                Code::KeyA,
                Code::KeyB,
                Code::KeyC,
                Code::KeyD,
                Code::KeyE,
                Code::KeyF,
                Code::KeyG,
                Code::KeyH,
                Code::KeyI,
                Code::KeyJ,
                Code::KeyK,
                Code::KeyL,
                Code::KeyM,
                Code::KeyN,
                Code::KeyO,
                Code::KeyP,
                Code::KeyQ,
                Code::KeyR,
                Code::KeyS,
                Code::KeyT,
                Code::KeyU,
                Code::KeyV,
                Code::KeyW,
                Code::KeyX,
                Code::KeyY,
                Code::KeyZ,
            ];
            LETTERS[(c as u8 - b'a') as usize]
        }
        '0'..='9' => {
            const DIGITS: [Code; 10] = [
                Code::Digit0,
                Code::Digit1,
                Code::Digit2,
                Code::Digit3,
                Code::Digit4,
                Code::Digit5,
                Code::Digit6,
                Code::Digit7,
                Code::Digit8,
                Code::Digit9,
            ];
            DIGITS[(c as u8 - b'0') as usize]
        }
        '/' => Code::Slash,
        '=' | '+' => Code::Equal,
        '-' => Code::Minus,
        _ => return None,
    };
    let mut mods = Modifiers::empty();
    if chord.primary {
        mods |= CMD_OR_CTRL;
    }
    if chord.shift {
        mods |= Modifiers::SHIFT;
    }
    if chord.alt {
        mods |= Modifiers::ALT;
    }
    Some(Accelerator::new(mods, code))
}

/// Installs the menu bar once. Call it after the event loop has launched:
/// winit replaces the main menu with its own default when launching, so a
/// menu installed during startup is lost.
pub fn install(recents: &[RecentItem]) {
    if ATTEMPTED.with(|attempted| attempted.replace(true)) {
        return;
    }
    let result = (|| -> muda::Result<NativeMenu> {
        let menu = Menu::new();
        let app = Submenu::new("Markdown Leaflet", true);
        app.append(&item("about", "About Markdown Leaflet"))?;
        app.append(&PredefinedMenuItem::separator())?;
        app.append(&item_for("quit", "Quit Markdown Leaflet", Command::Quit))?;
        menu.append(&app)?;

        let file = Submenu::new("File", true);
        file.append(&item_for("open_file", "Open File…", Command::OpenFile))?;
        file.append(&item_for(
            "open_folder",
            "Open Folder…",
            Command::OpenFolder,
        ))?;
        let recent = Submenu::new("Open Recent", true);
        file.append(&recent)?;
        file.append(&PredefinedMenuItem::separator())?;
        file.append(&item_for(
            "close_current",
            "Close Current",
            Command::CloseCurrent,
        ))?;
        menu.append(&file)?;

        let view = Submenu::new("View", true);
        view.append(&item_for("show_files", "Show Files", Command::ToggleFiles))?;
        view.append(&item_for(
            "show_outline",
            "Show Outline",
            Command::ShowOutline,
        ))?;
        view.append(&item_for(
            "show_details",
            "Show Details",
            Command::ShowDetails,
        ))?;
        view.append(&item_for(
            "toggle_minimap",
            "Show Minimap",
            Command::ToggleMinimap,
        ))?;
        view.append(&item_for(
            "view_source",
            "View Source",
            Command::ToggleSourceView,
        ))?;
        view.append(&PredefinedMenuItem::separator())?;
        view.append(&item_for("zoom_in", "Zoom In", Command::ZoomIn))?;
        view.append(&item_for("zoom_out", "Zoom Out", Command::ZoomOut))?;
        view.append(&item_for("zoom_reset", "Reset Zoom", Command::ZoomReset))?;
        view.append(&PredefinedMenuItem::separator())?;
        let appearance = Submenu::new("Appearance", true);
        appearance.append(&item("appearance_system", "System"))?;
        appearance.append(&item("appearance_slate", "Slate"))?;
        appearance.append(&item("appearance_harbor", "Harbor"))?;
        appearance.append(&item("appearance_nightfall", "Nightfall"))?;
        appearance.append(&item("appearance_linen", "Linen"))?;
        appearance.append(&item("appearance_paper", "Paper"))?;
        view.append(&appearance)?;
        let font = Submenu::new("Preview Font", true);
        font.append(&item("font_system", "System"))?;
        font.append(&item("font_sans", "Sans"))?;
        font.append(&item("font_serif", "Serif"))?;
        font.append(&item("font_mono", "Mono"))?;
        view.append(&font)?;
        view.append(&item_for(
            "reload_theme",
            "Reload Theme",
            Command::ReloadTheme,
        ))?;
        view.append(&PredefinedMenuItem::separator())?;
        view.append(&item("ask_external_links", "Ask About External Links"))?;
        menu.append(&view)?;

        let help = Submenu::new("Help", true);
        help.append(&item_for(
            "shortcuts",
            "Keyboard Shortcuts",
            Command::ShowShortcuts,
        ))?;
        menu.append(&help)?;
        menu.init_for_nsapp();
        Ok(NativeMenu {
            _menu: menu,
            recent,
            recent_items: Vec::new(),
        })
    })();
    match result {
        Ok(native) => {
            MENU.with(|slot| *slot.borrow_mut() = Some(native));
            refresh(recents);
        }
        Err(error) => eprintln!("Could not install macOS menu: {error}"),
    }
}

pub fn refresh(recents: &[RecentItem]) {
    MENU.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(native) = slot.as_mut() else { return };
        for item in native.recent_items.drain(..) {
            if let Err(error) = native.recent.remove(&item) {
                eprintln!("Could not update Recent menu: {error}");
            }
        }
        for (index, recent) in recents.iter().enumerate() {
            let item = MenuItem::with_id(format!("recent:{index}"), recent.label(), true, None);
            match native.recent.append(&item) {
                Ok(()) => native.recent_items.push(item),
                Err(error) => eprintln!("Could not update Recent menu: {error}"),
            }
        }
    });
}

pub fn next_action(recents: &[RecentItem]) -> Option<MenuAction> {
    let event = MenuEvent::receiver().try_recv().ok()?;
    let id = event.id.as_ref();
    let action = match id {
        "about" => MenuAction::About,
        "quit" => MenuAction::Quit,
        "open_file" => MenuAction::OpenFile,
        "open_folder" => MenuAction::OpenFolder,
        "close_current" => MenuAction::CloseCurrent,
        "show_files" => MenuAction::ShowFiles,
        "show_outline" => MenuAction::ShowOutline,
        "show_details" => MenuAction::ShowDetails,
        "toggle_minimap" => MenuAction::ToggleMinimap,
        "view_source" => MenuAction::ToggleSourceView,
        "appearance_system" => MenuAction::Appearance(AppearanceChoice::System),
        "appearance_slate" => MenuAction::Appearance(AppearanceChoice::Slate),
        "appearance_harbor" => MenuAction::Appearance(AppearanceChoice::Harbor),
        "appearance_nightfall" => MenuAction::Appearance(AppearanceChoice::Nightfall),
        "appearance_linen" => MenuAction::Appearance(AppearanceChoice::Linen),
        "appearance_paper" => MenuAction::Appearance(AppearanceChoice::Paper),
        "font_system" => MenuAction::PreviewFont(FontChoice::System),
        "font_sans" => MenuAction::PreviewFont(FontChoice::Sans),
        "font_serif" => MenuAction::PreviewFont(FontChoice::Serif),
        "font_mono" => MenuAction::PreviewFont(FontChoice::Mono),
        "zoom_in" => MenuAction::ZoomIn,
        "zoom_out" => MenuAction::ZoomOut,
        "zoom_reset" => MenuAction::ResetZoom,
        "reload_theme" => MenuAction::ReloadTheme,
        "ask_external_links" => MenuAction::AskAboutExternalLinks,
        "shortcuts" => MenuAction::Shortcuts,
        _ => {
            let index = id.strip_prefix("recent:")?.parse::<usize>().ok()?;
            MenuAction::Recent(recents.get(index)?.clone())
        }
    };
    Some(action)
}
