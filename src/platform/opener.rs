// SPDX-License-Identifier: GPL-3.0-or-later
//! Hands links and files to the operating system or another program.

use std::path::Path;

pub trait Opener {
    /// Opens a link in the default handler.
    fn open_link(&self, uri: &str) -> std::io::Result<()>;
    /// Opens a file with the program launched by `command`.
    fn open_with(&self, path: &Path, command: &str) -> std::io::Result<()>;
    /// Shows `path` in the system file manager, selected where the platform can.
    fn reveal(&self, path: &Path) -> std::io::Result<()>;
    /// Moves `path` to the system Trash, where it can be restored.
    fn trash(&self, path: &Path) -> std::io::Result<()>;
    /// Starts another instance of this app on `path`.
    fn open_new_window(&self, path: &Path) -> std::io::Result<()>;
}

/// Delegates to the `open` crate.
pub struct SystemOpener;

impl Opener for SystemOpener {
    fn open_link(&self, uri: &str) -> std::io::Result<()> {
        open::that(uri)
    }

    fn open_with(&self, path: &Path, command: &str) -> std::io::Result<()> {
        open::with(path, command)
    }

    fn reveal(&self, path: &Path) -> std::io::Result<()> {
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg("-R")
                .arg(path)
                .spawn()
                .map(drop)
        }
        #[cfg(target_os = "windows")]
        {
            let mut select = std::ffi::OsString::from("/select,");
            select.push(path);
            std::process::Command::new("explorer")
                .arg(select)
                .spawn()
                .map(drop)
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            // No portable "select this file": show its folder (or the folder itself).
            let target = if path.is_dir() {
                Some(path)
            } else {
                path.parent()
            };
            open::that(target.unwrap_or(path))
        }
    }

    fn trash(&self, path: &Path) -> std::io::Result<()> {
        trash::delete(path).map_err(std::io::Error::other)
    }

    fn open_new_window(&self, path: &Path) -> std::io::Result<()> {
        std::process::Command::new(std::env::current_exe()?)
            .arg(path)
            .spawn()
            .map(drop)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_system_opener_never_deletes_files_itself() {
        // Delete goes to the Trash so it can be undone. Guard the production code (the part
        // above the test module) against a direct, permanent delete creeping in.
        let source = include_str!("opener.rs");
        let production = source.split("#[cfg(test)]").next().unwrap();
        for forbidden in [["remove", "_dir_all"], ["remove", "_file"]] {
            let call = forbidden.concat();
            assert!(
                !production.contains(&call),
                "opener.rs must not call {call}"
            );
        }
    }
}
