# Markdown Leaflet

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/markdown-leaflet-logo-dark.svg">
  <img src="assets/brand/markdown-leaflet-logo.svg" alt="Markdown Leaflet logo" width="640">
</picture>

[![CI](https://github.com/SageleafSystems/markdown-leaflet/actions/workflows/ci.yml/badge.svg)](https://github.com/SageleafSystems/markdown-leaflet/actions/workflows/ci.yml)
[![License: GPL v3 or later](https://img.shields.io/badge/license-GPL--3.0--or--later-blue.svg)](LICENSE)
![Platforms: Linux | macOS](https://img.shields.io/badge/platform-Linux%20%7C%20macOS-lightgrey.svg)
![Rust: 1.93+](https://img.shields.io/badge/rust-1.93%2B-orange.svg)

A native desktop viewer for reading Markdown files and browsing folders of Markdown. Open a file, follow its heading outline, or search a folder without moving into an editor.

The app can be built from source on Linux and macOS. There are no downloadable binaries yet, and Windows has not been verified. The project is preparing for a public GitHub release.

This application is inspired by the most excellent Markdown Preview for MacOS https://github.com/pluk-inc/markdown-preview.

## Build and run

Install a current Rust toolchain with Cargo, then run:

```sh
cargo run --locked
```

Pass a Markdown file or folder to open it at launch:

```sh
cargo run --locked -- path/to/notes
cargo run --locked -- path/to/notes/readme.md
```

To install a user-local executable in `~/.local/bin`, run:

```sh
./scripts/install.sh
```

The script builds a release binary and replaces any previous copy installed at that location. Run `./scripts/install.sh remove` to remove it; `uninstall` is also accepted. Use `PREFIX=/some/path ./scripts/install.sh` to choose another prefix (the binary goes in its `bin/` subdirectory); use the same `PREFIX` when removing it. Ensure the chosen `bin/` directory is on your `PATH` to launch `markdown-leaflet` by name.

To install it system-wide instead, for every user on the machine, run `./scripts/install.sh global`. It builds as you, then uses `sudo` to install into `/usr/local` (prompting for your password); `./scripts/install.sh global-remove` (or `global-uninstall`) removes it. `PREFIX` overrides `/usr/local` here too.

On macOS, `./scripts/bundle-macos.sh` builds `target/macos/Markdown Leaflet.app` (with the app icon, signed ad hoc), and `./scripts/bundle-macos.sh dmg` also makes a `.dmg` with an Applications shortcut. Drag the app to `/Applications` to install it; it then appears under Open With for Markdown files and folders, and opens them from Finder or the Dock icon. Set `CODESIGN_IDENTITY` to sign with a Developer ID; notarization steps are in the script's header. An ad-hoc-signed app opens on the Mac that built it; on another Mac, Gatekeeper asks you to approve it in System Settings → Privacy & Security.

To build without installing, run `cargo build --release --locked`; the binary is `target/release/markdown-leaflet`. Linux builds need the native libraries used by `iced` and the file dialogs; the CI workflow is the reference for Ubuntu packages. The macOS CI job checks that the source builds there.

## Use

| Action                      | Shortcut                  |
| --------------------------- | -------------------------- |
| Open file                   | Ctrl/⌘ O                   |
| Open folder                 | Ctrl/⌘ Shift O             |
| Close current               | Ctrl/⌘ W                   |
| Toggle Files sidebar        | Ctrl/⌘ B                   |
| Toggle right gutter         | Ctrl/⌘ Alt B                |
| Toggle minimap              | Ctrl/⌘ Shift M              |
| View source                 | Ctrl/⌘ Alt S                |
| Zoom in / out / reset       | Ctrl/⌘ = / Ctrl/⌘ - / Ctrl/⌘ 0 |
| Show Outline                | Ctrl/⌘ Alt 1                |
| Show Details                | Ctrl/⌘ Alt 2                |
| Find in document            | Ctrl/⌘ F                   |
| Next / previous match       | Ctrl/⌘ G / Ctrl/⌘ Shift G   |
| Focus folder search         | Ctrl/⌘ Shift F              |
| Move through visible files  | ↑ / ↓                       |
| Rename selected file or folder | F2                      |
| Move selected file or folder to Trash | Delete or Ctrl/⌘ Backspace |
| Scroll preview a page       | Page Up / Page Down         |
| Select all preview text     | Ctrl/⌘ A                   |
| Copy selected text          | Ctrl/⌘ C or Ctrl Insert     |
| Find the selected text      | Ctrl/⌘ E                   |
| Show keyboard shortcuts     | Ctrl/⌘ /                   |
| Reload theme                | Ctrl/⌘ Shift R              |
| Quit                        | Ctrl/⌘ Q                   |

Drag across the preview to select text, shift+click to extend a selection, double-click for a word, and triple-click for a paragraph. Right-click the preview for copy, select, find, and link actions. On Linux a selection also fills the primary selection, so middle-click pastes it elsewhere.

Folders start collapsed. Folder search matches filenames or relative paths. Selecting a search result expands its ancestor folders. Document Find highlights matches in rendered text and wraps when moving between results; Escape closes its bar. Open documents are checked for edits about every 600 ms, and open folders are rescanned about every 3 seconds. Recent paths are stored locally in the OS configuration directory.

### Configuration and themes

The app skips common dependency, cache, and build folders such as `.git`, `node_modules`, and `target`. To include one of these, put this in the platform configuration directory at `markdown-leaflet/config.toml`:

```toml
include = ["node_modules"]
```

The same file controls appearance and semantic colors:

```toml
[theme]
appearance = "system" # system, slate, harbor, nightfall, linen, or paper

[theme.slate]
accent = "#30D158"
file_icon = "#81B8EB"
folder_icon = "#EDBB67"
utility_icon = "#9FC4AE"

[theme.paper]
accent = "#2F6FED"
```

The left and right gutter cards share an accent gradient you can adjust under `[theme.gradient]`, then apply with **View → Reload Theme**:

```toml
[theme.gradient]
style = "border" # "border" (default) fades the card's border; "fill" fades its fill instead
angle = 45       # degrees of tilt from straight down
opacity = 0.35   # how far the gradient leans toward the accent, 0 to 1
glow = 0.5       # accent glow around the card, 0 (off) to 1
```

Every color is optional and accepts `#RRGGBB`. Available roles are `background`, `surface`, `surface_alt`, `border`, `text`, `text_muted`, `accent`, `on_accent`, `danger`, `warning`, `resize`, `file_icon`, `folder_icon`, and `utility_icon`. Missing roles use the built-in palette. The app loads the file at startup; **View → Reload Theme** applies theme edits while it runs. The `include` scan rules continue to apply from startup. The `[preview]` table sets the body font of the rendered Markdown: `font = "system"` (default), `"sans"`, `"serif"`, or `"mono"`. Everything but `system` is a bundled face (Source Sans 3, Source Serif 4, Source Code Pro; SIL OFL, see `assets/fonts/`), so it looks the same on every OS; code blocks stay monospace. View → Preview Font (the in-window menu lists *System/Sans/Serif/Mono Font*) writes the choice back to the file, and Reload Theme re-reads it. The `[links]` table has one key, `external_markdown`, for what a link to a Markdown file outside the open folder does: `"ask"` (default) shows a dialog, `"folder"` opens that file's folder, and `"document"` opens the file alone while keeping the current folder. Tick "Always do this" in the dialog to set it, and choose View → Ask About External Links to go back to asking. The `[files]` table has one key, `show_text` (default `false`); set it, or choose **Show Text Files (.txt)** (checked, and labelled **Hide Text Files (.txt)**, while on) in any of the Files list's right-click menus, to list `.txt` files, which open as monospace text. The `[animation]` table has one key, `enabled` (default `true`); set it to `false` to make gutters open and close instantly instead of sliding. The Appearance menu writes the selected mode back to the file while preserving comments and other settings. Invalid theme values show an error and leave the last valid appearance active. See [src/domain/walk.rs](src/domain/walk.rs) for the default ignored directories.

The app menu includes About and Quit; File has open, recent, and close actions; View has panel and appearance controls; Help lists shortcuts. macOS uses the native menu bar, while other platforms show these menus in the window.

## Project status

CI is configured to check formatting, Clippy, tests, and source builds on Linux and macOS; it has not run on GitHub yet. Visual flows still need a real display check before the first release. The app reads local Markdown files and opens links through the OS; it does not edit documents. Folder search currently scans the in-memory file list for each query, which may slow down in very large trees.

See [ROADMAP.md](docs/ROADMAP.md) for planned validation and improvements. [Architecture](docs/ARCHITECTURE.md) explains the code. [Desktop release check](docs/MANUAL_TEST.md) lists the display checks still needed. [Contributing](CONTRIBUTING.md) has the development workflow. Security reports should follow [SECURITY.md](SECURITY.md).

The [logo and app icon](assets/brand/README.md) are available as SVG and PNG, with editable VectorCraft source files.

## License

Copyright (C) 2026 Sageleaf Systems. Licensed under the [GNU General Public License v3.0 or later](LICENSE) (`GPL-3.0-or-later`).
