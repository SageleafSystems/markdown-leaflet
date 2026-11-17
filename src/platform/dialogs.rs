// SPDX-License-Identifier: GPL-3.0-or-later
//! Native file and folder pickers.

use crate::config::Config;
use crate::domain::walk::{MARKDOWN_EXTENSIONS, Scan, scan_markdown_files};
use std::path::PathBuf;

pub async fn pick_file() -> Option<PathBuf> {
    let handle = rfd::AsyncFileDialog::new()
        .add_filter("Markdown", MARKDOWN_EXTENSIONS)
        .pick_file()
        .await?;

    Some(handle.path().to_path_buf())
}

pub async fn pick_folder(config: Config) -> Option<(PathBuf, Scan)> {
    let handle = rfd::AsyncFileDialog::new().pick_folder().await?;

    let path = handle.path().to_path_buf();
    let scan = scan_markdown_files(&path, &config.scan_rules());
    Some((path, scan))
}
