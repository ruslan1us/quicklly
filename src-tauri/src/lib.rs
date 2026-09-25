mod notes;

use tauri::{AppHandle, Manager, WindowEvent};

/// Appends `text` to today's note file in `Documents\Quicklly`.
#[tauri::command]
fn save_note(app: AppHandle, text: String) -> Result<(), String> {
    let dir = app
        .path()
        .document_dir()
        .map_err(|e| e.to_string())?
        .join("Quicklly");
    notes::append_note(&dir, chrono::Local::now().naive_local(), &text)
        .map_err(|e| format!("Failed to save note in {}: {e}", dir.display()))?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|window, event| {
            // Clicking elsewhere dismisses the input window, keeping the typed draft.
            if let WindowEvent::Focused(false) = event {
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![save_note])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
