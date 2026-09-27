use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, Window,
};
use tauri_plugin_store::StoreExt;

use crate::settings::STORE_FILE;

/// The Pad: a bigger editor for longer notes.
pub const PAD_WINDOW: &str = "pad";

/// Distance from the screen edges, in logical pixels.
const MARGIN: f64 = 16.0;

/// The unsaved Pad text, kept on disk so it survives closing the Pad or the app.
fn draft_path(app: &AppHandle) -> Option<PathBuf> {
    Some(app.path().app_data_dir().ok()?.join("pad-draft.md"))
}

#[tauri::command]
pub fn get_pad_draft(app: AppHandle) -> String {
    draft_path(&app)
        .and_then(|path| fs::read_to_string(path).ok())
        .unwrap_or_default()
}

#[tauri::command]
pub fn set_pad_draft(app: AppHandle, text: String) -> Result<(), String> {
    let path = draft_path(&app).ok_or("No app data folder")?;
    if text.is_empty() {
        if path.exists() {
            fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    fs::write(&path, text).map_err(|e| e.to_string())
}

/// Puts the Pad in the top right corner of the screen it is on.
fn place(window: &WebviewWindow) {
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten());
    let (Some(monitor), Ok(size)) = (monitor, window.outer_size()) else {
        return;
    };
    let area = monitor.work_area();
    let margin = (MARGIN * monitor.scale_factor()) as i32;
    let x = area.position.x + area.size.width as i32 - size.width as i32 - margin;
    let y = area.position.y + margin;
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

/// Shows the Pad, creating it on first use. The page shows a new window once it has loaded
/// the draft, so it never flickers.
pub fn show(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(PAD_WINDOW) {
        arrange(&window);
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let result = WebviewWindowBuilder::new(app, PAD_WINDOW, WebviewUrl::App("pad.html".into()))
        .title("Quicklly Pad")
        .inner_size(WIDTH, HEIGHT)
        .visible(false)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(is_pinned(app))
        .build();
    match result {
        Ok(window) => arrange(&window),
        Err(e) => eprintln!("Failed to open the Pad: {e}"),
    }
}

/// Default size of the Pad, in logical pixels.
const WIDTH: f64 = 560.0;
const HEIGHT: f64 = 380.0;
const PINNED_KEY: &str = "padPinned";
const BOUNDS_KEY: &str = "padBounds";

/// Whether the Pad is pinned: then it stays open while other windows are used, can be moved
/// and resized, and keeps its place. Unpinned, it pops up in its default place and hides
/// like the input window.
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

/// Where and how big a pinned Pad was, in physical pixels.
#[derive(Serialize, Deserialize)]
struct Bounds {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

/// A pinned Pad goes back where it was left; an unpinned one gets its default size and place.
fn arrange(window: &WebviewWindow) {
    let app = window.app_handle();
    let saved = app
        .store(STORE_FILE)
        .ok()
        .and_then(|store| store.get(BOUNDS_KEY))
        .and_then(|bounds| serde_json::from_value::<Bounds>(bounds).ok());
    match saved {
        Some(bounds) if is_pinned(app) => {
            let _ = window.set_size(PhysicalSize::new(bounds.width, bounds.height));
            let _ = window.set_position(PhysicalPosition::new(bounds.x, bounds.y));
        }
        _ => {
            let _ = window.set_size(LogicalSize::new(WIDTH, HEIGHT));
            place(window);
        }
    }
}

/// Remembers where a pinned Pad is, after it was moved or resized.
pub fn remember_bounds(window: &Window) {
    if let (Ok(position), Ok(size)) = (window.outer_position(), window.inner_size()) {
        save_bounds(window.app_handle(), position, size);
    }
}

fn save_bounds(app: &AppHandle, position: PhysicalPosition<i32>, size: PhysicalSize<u32>) {
    if !is_pinned(app) {
        return;
    }
    let bounds = Bounds {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    };
    if let (Ok(store), Ok(bounds)) = (app.store(STORE_FILE), serde_json::to_value(bounds)) {
        store.set(BOUNDS_KEY, bounds);
    }
}

#[tauri::command]
pub fn get_pad_pinned(app: AppHandle) -> bool {
    is_pinned(&app)
}

/// Pins or unpins the Pad (Ctrl+P).
#[tauri::command]
pub fn set_pad_pinned(app: AppHandle, pinned: bool) -> Result<(), String> {
    app.state::<Pinned>().0.store(pinned, Ordering::Relaxed);
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    store.set(PINNED_KEY, pinned);
    if let Some(window) = app.get_webview_window(PAD_WINDOW) {
        let _ = window.set_resizable(pinned);
        if pinned {
            if let (Ok(position), Ok(size)) = (window.outer_position(), window.inner_size()) {
                save_bounds(&app, position, size);
            }
        } else {
            arrange(&window);
        }
    }
    store.save().map_err(|e| e.to_string())
}

/// Opens the Pad from the input window (→ in an empty note, or `/pad`).
///
/// Async on purpose: creating a window from a sync command deadlocks on Windows.
#[tauri::command]
pub async fn open_pad(app: AppHandle) {
    show(&app);
}
