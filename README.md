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

The app can be built from source on Linux and macOS. There are no downloadable binaries yet, and Windows has not been verified. This is an early alpha: expect rough edges.

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

The script builds a release binary and replaces any previous copy installed at that location.

| Task | Command |
| --- | --- |
| Remove the user-local install | `./scripts/install.sh remove` (or `uninstall`) |
| Install to a different prefix | `PREFIX=/some/path ./scripts/install.sh` — binary lands in its `bin/` subdirectory; use the same `PREFIX` to remove it |
| Install system-wide (all users) | `./scripts/install.sh global` — builds as you, then uses `sudo` to install into `/usr/local` |
| Remove the system-wide install | `./scripts/install.sh global-remove` (or `global-uninstall`) |

Put the chosen `bin/` directory on your `PATH` to launch `markdown-leaflet` by name.

**macOS app bundle:**

- `./scripts/bundle-macos.sh` builds `target/macos/Markdown Leaflet.app`, ad-hoc signed.
- `./scripts/bundle-macos.sh dmg` also makes a `.dmg` with an Applications shortcut.
- Drag the app to `/Applications` to install it. It then appears under Open With for Markdown files and folders, and opens from Finder or the Dock icon.
- Set `CODESIGN_IDENTITY` to sign with a Developer ID (notarization steps are in the script's header).
- An ad-hoc-signed app opens on the Mac that built it. On another Mac, approve it first in System Settings → Privacy & Security.

**Build without installing:** `cargo build --release --locked` — binary at `target/release/markdown-leaflet`. Linux needs the native libraries `iced` and the file dialogs use; see the CI workflow for the Ubuntu package list.

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

**Selecting text:** drag to select, shift+click to extend, double-click for a word, triple-click for a paragraph. Right-click for copy, select, find, and link actions. On Linux, a selection also fills the primary selection — middle-click pastes it elsewhere.

**Other behavior:**

- Folders start collapsed.
- Folder search matches filenames or relative paths; selecting a result expands its ancestor folders.
- Find highlights matches in rendered text, wraps at the ends, and closes with Escape.
- Open documents are checked for edits every ~600 ms; open folders are rescanned every ~3 s.
- Recent paths are stored locally in the OS configuration directory.

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

Colors are optional and accept `#RRGGBB`; missing roles fall back to the built-in palette. Available roles: `background`, `surface`, `surface_alt`, `border`, `text`, `text_muted`, `accent`, `on_accent`, `danger`, `warning`, `resize`, `file_icon`, `folder_icon`, `utility_icon`.

The app loads `config.toml` at startup. **View → Reload Theme** re-reads it without restarting; the `include` scan rules still apply from startup only. Invalid theme values show an error and keep the last valid appearance active. See [src/domain/walk.rs](src/domain/walk.rs) for the default ignored directories.

Other `config.toml` settings:

| Table | Key | Values | Default | Effect |
| --- | --- | --- | --- | --- |
| `[preview]` | `font` | `system`, `sans`, `serif`, `mono` | `system` | Body font of rendered Markdown. Non-system fonts are bundled (Source Sans 3, Source Serif 4, Source Code Pro; SIL OFL, see `assets/fonts/`), so they render identically on every OS; code blocks always stay monospace. Also settable from View → Preview Font. |
| `[links]` | `external_markdown` | `ask`, `folder`, `document` | `ask` | What happens when you follow a link to a Markdown file outside the open folder: ask via dialog, open that file's folder, or open just the file. Tick "Always do this" in the dialog to set it; View → Ask About External Links resets to `ask`. |
| `[files]` | `show_text` | `true`, `false` | `false` | List `.txt` files (opened as monospace text). Also toggled via **Show Text Files (.txt)** in any Files-list right-click menu. |
| `[animation]` | `enabled` | `true`, `false` | `true` | `false` makes gutters open/close instantly instead of sliding. |

The View → Appearance menu writes the selected mode back to `config.toml`, preserving comments and other settings.

The app menu includes About and Quit; File has open, recent, and close actions; View has panel and appearance controls; Help lists shortcuts. macOS uses the native menu bar, while other platforms show these menus in the window.

## Project status

- CI checks formatting, Clippy, tests, and source builds on Linux and macOS.
- The app reads local Markdown files and opens links through the OS; it does not edit documents.
- Folder search scans the in-memory file list per query, which may slow down in very large trees.

[Architecture](docs/ARCHITECTURE.md) explains the code. [Contributing](CONTRIBUTING.md) has the development workflow. Security reports should follow [SECURITY.md](SECURITY.md).

The [logo and app icon](assets/brand/README.md) are available as SVG and PNG, with editable VectorCraft source files.

## License

Copyright (C) 2026 Sageleaf Systems. Licensed under the [GNU General Public License v3.0 or later](LICENSE) (`GPL-3.0-or-later`).
