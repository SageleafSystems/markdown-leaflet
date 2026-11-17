// SPDX-License-Identifier: GPL-3.0-or-later
//! Receives "open these files" requests from Finder (double-click, Open With, a file or folder
//! dropped on the Dock icon). macOS delivers them to the application delegate as
//! `application:openURLs:`, not as command-line arguments, and winit's delegate does not
//! implement it, so the request would be dropped. This adds that method to the delegate's class
//! and queues the paths for the app to open on its next tick.
//!
//! This is the only module that talks to Objective-C directly, which needs `unsafe`.
#![allow(unsafe_code)]

use objc2::ffi;
use objc2::runtime::{AnyClass, AnyObject, Imp, Sel};
use objc2::{class, msg_send, sel};
use objc2_foundation::{NSArray, NSURL};
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

static PENDING: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
static INSTALLED: AtomicBool = AtomicBool::new(false);

/// The method `AppKit` calls on the delegate for each open request, cold launch or running.
/// `-[NSApplicationDelegate application:openURLs:]`.
unsafe extern "C-unwind" fn application_open_urls(
    _this: *mut AnyObject,
    _cmd: Sel,
    _application: *mut AnyObject,
    urls: *mut NSArray<NSURL>,
) {
    // SAFETY: AppKit passes a valid `NSArray` of `NSURL`s (or nil, handled by `as_ref`).
    let Some(urls) = (unsafe { urls.as_ref() }) else {
        return;
    };
    let mut queue = lock();
    for url in urls {
        if let Some(path) = url.path() {
            enqueue(&mut queue, PathBuf::from(path.to_string()));
        }
    }
}

fn lock() -> std::sync::MutexGuard<'static, Vec<PathBuf>> {
    PENDING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Adds `path` unless it is already waiting, so a repeated event opens the file once.
fn enqueue(queue: &mut Vec<PathBuf>, path: PathBuf) {
    if !queue.contains(&path) {
        queue.push(path);
    }
}

/// Teaches the application delegate to receive open requests. Safe to call repeatedly; it does
/// nothing once installed, or while the delegate does not exist yet. Call it on the main thread,
/// as early as the delegate exists: a file opened from Finder at launch is delivered as soon as
/// the app finishes launching.
pub fn install() {
    if INSTALLED.load(Ordering::Acquire) {
        return;
    }
    // SAFETY: plain Objective-C messages to the shared application and its delegate, from the
    // main thread; the delegate pointer is only used to read its class.
    unsafe {
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let delegate: *mut AnyObject = msg_send![app, delegate];
        let Some(delegate) = delegate.as_ref() else {
            return;
        };
        let class: *const AnyClass = delegate.class();
        // `v@:@@`: returns void; takes self, the selector, the application and the URL array.
        let imp: Imp = std::mem::transmute::<
            unsafe extern "C-unwind" fn(*mut AnyObject, Sel, *mut AnyObject, *mut NSArray<NSURL>),
            Imp,
        >(application_open_urls);
        // Fails, harmlessly, if the class already has the method.
        ffi::class_addMethod(
            class.cast_mut(),
            sel!(application:openURLs:),
            imp,
            c"v@:@@".as_ptr(),
        );
    }
    INSTALLED.store(true, Ordering::Release);
}

/// The paths Finder asked to open since the last call, oldest first.
pub fn take() -> Vec<PathBuf> {
    std::mem::take(&mut *lock())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_repeated_request_is_queued_once_and_order_is_kept() {
        let mut queue = Vec::new();
        enqueue(&mut queue, PathBuf::from("/a.md"));
        enqueue(&mut queue, PathBuf::from("/b.md"));
        enqueue(&mut queue, PathBuf::from("/a.md"));
        assert_eq!(queue, [PathBuf::from("/a.md"), PathBuf::from("/b.md")]);
    }
}
