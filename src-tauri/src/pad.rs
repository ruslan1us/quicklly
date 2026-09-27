use std::fs;
use std::path::PathBuf;

use tauri::{
    AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

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
        place(&window);
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let result = WebviewWindowBuilder::new(app, PAD_WINDOW, WebviewUrl::App("pad.html".into()))
        .title("Quicklly Pad")
        .inner_size(560.0, 380.0)
        .visible(false)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .build();
    match result {
        Ok(window) => place(&window),
        Err(e) => eprintln!("Failed to open the Pad: {e}"),
    }
}

/// Opens the Pad from the input window (→ in an empty note, or `/pad`).
///
/// Async on purpose: creating a window from a sync command deadlocks on Windows.
#[tauri::command]
pub async fn open_pad(app: AppHandle) {
    show(&app);
}
