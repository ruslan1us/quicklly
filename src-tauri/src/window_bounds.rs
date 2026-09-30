use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewWindow, Window,
};
use tauri_plugin_store::StoreExt;

use crate::settings::{self, STORE_FILE};

/// Where and how big a pinned window was, in physical pixels.
#[derive(Serialize, Deserialize)]
struct Bounds {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

/// Puts a window back where it was saved under `key`; returns false if nothing was saved.
pub fn restore(window: &WebviewWindow, key: &str) -> bool {
    let saved = window
        .app_handle()
        .store(STORE_FILE)
        .ok()
        .and_then(|store| store.get(key))
        .and_then(|bounds| serde_json::from_value::<Bounds>(bounds).ok());
    let Some(bounds) = saved else {
        return false;
    };
    let _ = window.set_size(PhysicalSize::new(bounds.width, bounds.height));
    let _ = window.set_position(PhysicalPosition::new(bounds.x, bounds.y));
    true
}

/// Saves where a window is and how big it is under `key`.
pub fn save(window: &Window, key: &str) {
    if let (Ok(position), Ok(size)) = (window.outer_position(), window.inner_size()) {
        store(window.app_handle(), key, position, size);
    }
}

/// [`save`] for a window with a webview.
pub fn save_webview(window: &WebviewWindow, key: &str) {
    if let (Ok(position), Ok(size)) = (window.outer_position(), window.inner_size()) {
        store(window.app_handle(), key, position, size);
    }
}

fn store(app: &AppHandle, key: &str, position: PhysicalPosition<i32>, size: PhysicalSize<u32>) {
    let bounds = Bounds {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    };
    if let (Ok(store), Ok(bounds)) = (app.store(STORE_FILE), serde_json::to_value(bounds)) {
        store.set(key, bounds);
    }
}

/// Lets a pinned window be resized, but not below `min` (logical pixels at normal scale), so
/// it can't be squeezed into a sliver; an unpinned window keeps the size its page gives it.
pub fn set_resizable(window: &WebviewWindow, resizable: bool, min: (f64, f64)) {
    let _ = window.set_resizable(resizable);
    let zoom = settings::scale(window.app_handle());
    let min_size = resizable.then(|| LogicalSize::new(min.0 * zoom, min.1 * zoom));
    let _ = window.set_min_size(min_size);
}
