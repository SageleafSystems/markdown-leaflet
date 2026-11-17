// SPDX-License-Identifier: GPL-3.0-or-later
//! The images of the open document: which URL names which file, and the load state of each file.
//! The preview reads it with [`ImageStore::slot`]; `app::update::images` fills it.

use crate::domain::images;
use iced::widget::{image, markdown, svg};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// A right-click on a drawn image: the file, for the menu the surrounding area opens.
#[derive(Debug, Clone)]
pub struct ImageMenu {
    pub path: PathBuf,
}

/// A decoded image, ready to draw.
#[derive(Debug, Clone)]
pub enum Loaded {
    Raster(image::Handle),
    Svg(svg::Handle),
}

/// What the preview shows for one image.
#[derive(Debug, Clone)]
pub enum Slot {
    Loading,
    Ready(Loaded),
    Failed(String),
}

#[derive(Debug)]
struct Entry {
    /// The file's modification time when its load began; a different time means a reload.
    mtime: Option<SystemTime>,
    slot: Slot,
}

/// A file whose load must start, with the modification time it was planned under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingLoad {
    pub path: PathBuf,
    pub mtime: Option<SystemTime>,
}

#[derive(Debug, Default)]
pub struct ImageStore {
    /// Each document URL that names a local image, and the file it resolves to.
    urls: HashMap<String, PathBuf>,
    entries: HashMap<PathBuf, Entry>,
}

impl ImageStore {
    /// Resolves every image of `items` against the document at `doc_path` inside `root`, keeps
    /// the entries that are still current, and returns the files that need loading. Entries no
    /// longer referenced are dropped.
    pub fn plan(
        &mut self,
        items: &[markdown::Item],
        doc_path: &Path,
        root: &Path,
    ) -> Vec<PendingLoad> {
        let doc_dir = doc_path.parent().unwrap_or(root);
        self.urls = images::image_urls(items)
            .into_iter()
            .filter_map(|url| Some((url.to_string(), images::resolve(url, doc_dir, root)?)))
            .collect();
        let mut kept = HashMap::new();
        let mut pending = Vec::new();
        for path in self.urls.values() {
            if kept.contains_key(path) {
                continue;
            }
            let mtime = std::fs::metadata(path)
                .and_then(|meta| meta.modified())
                .ok();
            let entry = match self.entries.remove(path) {
                Some(entry) if entry.mtime == mtime => entry,
                _ => {
                    pending.push(PendingLoad {
                        path: path.clone(),
                        mtime,
                    });
                    Entry {
                        mtime,
                        slot: Slot::Loading,
                    }
                }
            };
            kept.insert(path.clone(), entry);
        }
        self.entries = kept;
        pending
    }

    /// Records a finished load, unless the file changed or left the document since it began.
    pub fn finish(
        &mut self,
        path: &Path,
        mtime: Option<SystemTime>,
        result: Result<Loaded, String>,
    ) {
        if let Some(entry) = self.entries.get_mut(path)
            && entry.mtime == mtime
        {
            entry.slot = match result {
                Ok(loaded) => Slot::Ready(loaded),
                Err(reason) => Slot::Failed(reason),
            };
        }
    }

    /// The state of the image `url` names, or `None` when it is not a local image the document
    /// can show (remote, outside the folder, missing, or an unsupported type).
    pub fn slot(&self, url: &str) -> Option<&Slot> {
        self.entries
            .get(self.urls.get(url)?)
            .map(|entry| &entry.slot)
    }

    /// The file `url` resolves to, if it names a local image.
    pub fn path(&self, url: &str) -> Option<&Path> {
        self.urls.get(url).map(PathBuf::as_path)
    }

    pub fn clear(&mut self) {
        self.urls.clear();
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(source: &str) -> Vec<markdown::Item> {
        markdown::parse(source).collect()
    }

    fn folder(name: &str) -> crate::testutil::ScratchDir {
        crate::testutil::scratch_dir(&format!("store-{name}"))
    }

    #[test]
    fn plans_each_local_image_once_and_keeps_current_entries() {
        let root = folder("plan");
        ::image::RgbaImage::new(1, 1)
            .save(root.join("a.png"))
            .unwrap();
        let doc = root.join("doc.md");
        let items = items("![a](a.png)\n\n![again](a.png)\n\n![r](https://x.test/r.png)");
        let mut store = ImageStore::default();
        let pending = store.plan(&items, &doc, &root);
        assert_eq!(pending.len(), 1);
        assert!(matches!(store.slot("a.png"), Some(Slot::Loading)));
        assert!(store.slot("https://x.test/r.png").is_none());

        let handle = image::Handle::from_rgba(1, 1, vec![0; 4]);
        store.finish(
            &pending[0].path,
            pending[0].mtime,
            Ok(Loaded::Raster(handle)),
        );
        assert!(matches!(store.slot("a.png"), Some(Slot::Ready(_))));
        // A reload with the file unchanged neither reloads nor flickers back to loading.
        assert!(store.plan(&items, &doc, &root).is_empty());
        assert!(matches!(store.slot("a.png"), Some(Slot::Ready(_))));
    }

    #[test]
    fn ignores_a_result_for_a_changed_file_and_drops_unused_entries() {
        let root = folder("stale");
        ::image::RgbaImage::new(1, 1)
            .save(root.join("a.png"))
            .unwrap();
        let doc = root.join("doc.md");
        let mut store = ImageStore::default();
        let pending = store.plan(&items("![a](a.png)"), &doc, &root);
        store.finish(
            &pending[0].path,
            Some(SystemTime::UNIX_EPOCH),
            Err("old".into()),
        );
        assert!(matches!(store.slot("a.png"), Some(Slot::Loading)));
        store.plan(&items("no images"), &doc, &root);
        assert!(store.slot("a.png").is_none());
    }
}
