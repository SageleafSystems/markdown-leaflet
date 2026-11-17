// SPDX-License-Identifier: GPL-3.0-or-later
//! Stamps the build with a short git revision (and a `-dirty` suffix when
//! the tree has uncommitted changes) so the footer and About dialog can
//! show which commit a running binary actually came from. Without this,
//! every build shows the same `Cargo.toml` version regardless of what
//! source changed, which makes a stale install indistinguishable from a
//! fresh one.
//!
//! Emits no `rerun-if-changed` directives on purpose: Cargo's default of
//! rerunning a build script when any file in the crate changes is exactly
//! the freshness check wanted here.

use std::process::Command;

fn main() {
    println!("cargo:rustc-env=MP_BUILD_REV={}", build_rev());
}

fn build_rev() -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    let Ok(output) = Command::new("git")
        .args(["rev-parse", "--short=7", "HEAD"])
        .current_dir(dir)
        .output()
    else {
        return "unknown".to_string();
    };
    if !output.status.success() {
        return "unknown".to_string();
    }

    let mut rev = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(dir)
        .output()
        .is_ok_and(|out| out.status.success() && !out.stdout.is_empty());
    if dirty {
        rev.push_str("-dirty");
    }
    rev
}
