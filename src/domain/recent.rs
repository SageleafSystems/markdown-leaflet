// SPDX-License-Identifier: GPL-3.0-or-later
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const MAX_RECENT: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecentKind {
    File,
    Folder,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentItem {
    pub kind: RecentKind,
    pub path: PathBuf,
    #[serde(default)]
    pub selected: Option<PathBuf>,
}

impl RecentItem {
    /// Used only by the macOS native Recent menu (`platform::native_menu`); the in-window Recent
    /// panel (`app::view::recent`) renders its rows directly instead.
    #[cfg(target_os = "macos")]
    pub fn label(&self) -> String {
        let name = self.path.file_name().unwrap_or_default().to_string_lossy();
        let parent = self.path.parent().unwrap_or(Path::new(""));
        let kind = match self.kind {
            RecentKind::File => "File",
            RecentKind::Folder => "Folder",
        };
        format!("{kind}: {name} — {}", parent.display())
    }

    /// Keep recent entries identifiable without letting a long path size every menu.
    pub fn compact_label(&self) -> String {
        let name = self.path.file_name().unwrap_or_default().to_string_lossy();
        let kind = match self.kind {
            RecentKind::File => "File",
            RecentKind::Folder => "Folder",
        };
        let mut short = name.chars().take(13).collect::<String>();
        if name.chars().count() > 13 {
            short.push('…');
        }
        format!("{kind}: {short}")
    }
}

#[derive(Default, Deserialize, Serialize)]
struct RecentStore {
    items: Vec<RecentItem>,
}

fn store_path() -> Option<PathBuf> {
    // Tests build a `State` and open files, which saves the list. They must never read or
    // overwrite the developer's real Recent list.
    if cfg!(test) {
        return None;
    }
    Some(dirs::config_dir()?.join("markdown-leaflet/recent.toml"))
}

pub fn load() -> Vec<RecentItem> {
    let Some(path) = store_path() else {
        return Vec::new();
    };
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| toml::from_str::<RecentStore>(&raw).ok())
        .unwrap_or_default()
        .items
        .into_iter()
        .filter(|item| item.path.exists())
        .take(MAX_RECENT)
        .collect()
}

pub fn remember(items: &mut Vec<RecentItem>, item: RecentItem) {
    items.retain(|previous| previous.path != item.path);
    items.insert(0, item);
    items.truncate(MAX_RECENT);
}

pub fn save(items: &[RecentItem]) {
    let Some(path) = store_path() else {
        return;
    };
    let Some(parent) = path.parent() else {
        return;
    };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }
    if let Ok(raw) = toml::to_string(&RecentStore {
        items: items.to_vec(),
    }) {
        let _ = std::fs::write(path, raw);
    }
}

pub fn absolute(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_default().join(path)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn most_recent_path_replaces_older_entry_and_keeps_folder_selection() {
        let mut items = vec![RecentItem {
            kind: RecentKind::Folder,
            path: PathBuf::from("/notes"),
            selected: None,
        }];
        remember(
            &mut items,
            RecentItem {
                kind: RecentKind::Folder,
                path: PathBuf::from("/notes"),
                selected: Some(PathBuf::from("/notes/today.md")),
            },
        );
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].selected, Some(PathBuf::from("/notes/today.md")));
    }
}
