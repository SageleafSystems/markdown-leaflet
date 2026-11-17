// SPDX-License-Identifier: GPL-3.0-or-later
//! Helpers shared by unit tests.

use crate::platform::opener::Opener;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

static NEXT: AtomicU32 = AtomicU32::new(0);

/// A new, empty folder under the system temp directory that is removed, with everything in it,
/// when the value is dropped, even if the test fails. Its name is unique to this process and call,
/// so parallel tests and other users' leftovers cannot collide with it or pre-create it.
#[derive(Debug)]
pub(crate) struct ScratchDir(PathBuf);

impl ScratchDir {
    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

impl Deref for ScratchDir {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.0
    }
}

impl AsRef<Path> for ScratchDir {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        // Restore any permissions a test removed, or the folder could not be deleted.
        let _ = restore_access(&self.0);
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(unix)]
fn restore_access(dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    for entry in std::fs::read_dir(dir)?.flatten() {
        let path = entry.path();
        if path.is_dir() && !path.is_symlink() {
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755));
            let _ = restore_access(&path);
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn restore_access(_: &Path) -> std::io::Result<()> {
    Ok(())
}

/// Creates a [`ScratchDir`] whose name starts with `mdp-<prefix>`.
pub(crate) fn scratch_dir(prefix: &str) -> ScratchDir {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("mdp-{prefix}-{}-{n}", std::process::id()));
    // `create_dir` (not `_all`) fails if the name already exists, rather than adopting it.
    std::fs::create_dir(&dir).expect("create a unique scratch folder");
    ScratchDir(dir)
}

/// What an [`Opener`] was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OpenerCall {
    Link(String),
    With(PathBuf, String),
    Reveal(PathBuf),
    Trash(PathBuf),
    NewWindow(PathBuf),
}

/// An [`Opener`] for tests: it records every request and never reaches the operating system, so a
/// test cannot launch a browser or editor, open a window, or touch the Trash. Clone it to keep a
/// handle after boxing it into a `State`.
#[derive(Debug, Clone, Default)]
pub(crate) struct RecordingOpener {
    calls: Arc<Mutex<Vec<OpenerCall>>>,
    fail: bool,
    delete_for_real: bool,
}

impl RecordingOpener {
    /// Every request returns an error.
    pub(crate) fn failing() -> Self {
        Self {
            fail: true,
            ..Self::default()
        }
    }

    /// `trash` deletes permanently instead of recording only, for tests of what a delete leaves
    /// behind. Use it only on a [`ScratchDir`].
    pub(crate) fn deleting() -> Self {
        Self {
            delete_for_real: true,
            ..Self::default()
        }
    }

    pub(crate) fn calls(&self) -> Vec<OpenerCall> {
        self.calls.lock().unwrap().clone()
    }

    fn record(&self, call: OpenerCall) -> std::io::Result<()> {
        self.calls.lock().unwrap().push(call);
        if self.fail {
            Err(std::io::Error::other("refused by the test opener"))
        } else {
            Ok(())
        }
    }
}

impl Opener for RecordingOpener {
    fn open_link(&self, uri: &str) -> std::io::Result<()> {
        self.record(OpenerCall::Link(uri.to_string()))
    }

    fn open_with(&self, path: &Path, command: &str) -> std::io::Result<()> {
        self.record(OpenerCall::With(path.to_path_buf(), command.to_string()))
    }

    fn reveal(&self, path: &Path) -> std::io::Result<()> {
        self.record(OpenerCall::Reveal(path.to_path_buf()))
    }

    fn trash(&self, path: &Path) -> std::io::Result<()> {
        self.record(OpenerCall::Trash(path.to_path_buf()))?;
        if self.delete_for_real {
            if path.is_dir() {
                std::fs::remove_dir_all(path)?;
            } else {
                std::fs::remove_file(path)?;
            }
        }
        Ok(())
    }

    fn open_new_window(&self, path: &Path) -> std::io::Result<()> {
        self.record(OpenerCall::NewWindow(path.to_path_buf()))
    }
}
