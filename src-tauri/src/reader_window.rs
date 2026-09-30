use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use tauri::{AppHandle, Manager, WebviewWindow, Window};
use tauri_plugin_store::StoreExt;

use crate::settings::STORE_FILE;
use crate::window_bounds;

pub const READER_WINDOW: &str = "reader";

/// Smallest size of a pinned reader, in logical pixels at normal scale.
const MIN_SIZE: (f64, f64) = (440.0, 200.0);
const PINNED_KEY: &str = "readerPinned";
const BOUNDS_KEY: &str = "readerBounds";
const FILE_KEY: &str = "readerFile";

/// Whether the reader is pinned: then it stays open while other windows are used, can be
/// resized, keeps its place and reopens on the same file. Unpinned, it closes when another
/// window is clicked.
#[derive(Default)]
pub struct Pinned(AtomicBool);

/// Reads the saved pin state; called once at startup.
pub fn load(app: &AppHandle) {
    let pinned = app
        .store(STORE_FILE)
        .ok()
        .and_then(|store| store.get(PINNED_KEY))
        .and_then(|pinned| pinned.as_bool())
        .unwrap_or(false);
    app.state::<Pinned>().0.store(pinned, Ordering::Relaxed);
}

pub fn is_pinned(app: &AppHandle) -> bool {
    app.state::<Pinned>().0.load(Ordering::Relaxed)
}

/// A new pinned reader goes back where it was left; an unpinned one is placed by its page.
pub fn arrange(window: &WebviewWindow) {
    if is_pinned(window.app_handle()) {
        window_bounds::set_resizable(window, true, MIN_SIZE);
        window_bounds::restore(window, BOUNDS_KEY);
    }
}

/// Remembers where a pinned reader is, after it was moved or resized.
pub fn remember_bounds(window: &Window) {
    if is_pinned(window.app_handle()) {
        window_bounds::save(window, BOUNDS_KEY);
    }
}

/// The pin state as the reader page needs it: pinned, and the file it was showing.
#[derive(Serialize)]
pub struct ReaderPin {
    pinned: bool,
    file: Option<String>,
}

#[tauri::command]
pub fn get_reader_pin(app: AppHandle) -> ReaderPin {
    let file = app
        .store(STORE_FILE)
        .ok()
        .and_then(|store| store.get(FILE_KEY))
        .and_then(|file| file.as_str().map(String::from));
    ReaderPin {
        pinned: is_pinned(&app),
        file,
    }
}

/// Pins or unpins the reader (Ctrl+P), and remembers the file it shows so a pinned reader
/// reopens on it.
#[tauri::command]
pub fn set_reader_pin(app: AppHandle, pinned: bool, file: Option<String>) -> Result<(), String> {
    app.state::<Pinned>().0.store(pinned, Ordering::Relaxed);
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    store.set(PINNED_KEY, pinned);
    match file {
        Some(file) => store.set(FILE_KEY, file),
        None => {
            store.delete(FILE_KEY);
        }
    }
    if let Some(window) = app.get_webview_window(READER_WINDOW) {
        window_bounds::set_resizable(&window, pinned, MIN_SIZE);
        if pinned {
            window_bounds::save_webview(&window, BOUNDS_KEY);
        }
    }
    store.save().map_err(|e| e.to_string())
}
