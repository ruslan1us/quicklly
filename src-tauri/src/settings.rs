use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_store::StoreExt;

/// Settings file, stored in the app data directory.
const STORE_FILE: &str = "settings.json";
const NOTES_DIR_KEY: &str = "notesDir";

/// Settings as shown in the settings window.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    notes_dir: String,
    notes_dir_is_default: bool,
}

fn default_notes_dir(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(app.path().document_dir()?.join("Quicklly"))
}

fn custom_notes_dir(app: &AppHandle) -> Option<PathBuf> {
    let store = app.store(STORE_FILE).ok()?;
    let dir = store.get(NOTES_DIR_KEY)?;
    dir.as_str().filter(|s| !s.is_empty()).map(PathBuf::from)
}

/// Folder where notes are written: the one chosen in settings, or `Documents\Quicklly`.
pub fn notes_dir(app: &AppHandle) -> tauri::Result<PathBuf> {
    match custom_notes_dir(app) {
        Some(dir) => Ok(dir),
        None => default_notes_dir(app),
    }
}

fn set_custom_notes_dir(app: &AppHandle, dir: Option<PathBuf>) -> Result<(), String> {
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    match dir {
        Some(dir) => store.set(NOTES_DIR_KEY, dir.to_string_lossy().into_owned()),
        None => {
            store.delete(NOTES_DIR_KEY);
        }
    }
    store.save().map_err(|e| e.to_string())
}

fn view(app: &AppHandle) -> Result<SettingsView, String> {
    let notes_dir = notes_dir(app).map_err(|e| e.to_string())?;
    Ok(SettingsView {
        notes_dir: notes_dir.to_string_lossy().into_owned(),
        notes_dir_is_default: custom_notes_dir(app).is_none(),
    })
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Result<SettingsView, String> {
    view(&app)
}

/// Lets the user pick a new notes folder; keeps the current one if the dialog is cancelled.
#[tauri::command]
pub async fn pick_notes_dir(app: AppHandle) -> Result<SettingsView, String> {
    let mut dialog = app.dialog().file().set_title("Choose notes folder");
    if let Ok(current) = notes_dir(&app) {
        dialog = dialog.set_directory(current);
    }
    if let Some(window) = app.get_webview_window(crate::SETTINGS_WINDOW) {
        dialog = dialog.set_parent(&window);
    }
    if let Some(picked) = dialog.blocking_pick_folder() {
        let dir = picked.into_path().map_err(|e| e.to_string())?;
        set_custom_notes_dir(&app, Some(dir))?;
    }
    view(&app)
}

#[tauri::command]
pub fn reset_notes_dir(app: AppHandle) -> Result<SettingsView, String> {
    set_custom_notes_dir(&app, None)?;
    view(&app)
}
