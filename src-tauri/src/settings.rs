use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
use tauri_plugin_store::StoreExt;

use crate::hotkey;
use crate::notes::Mode;

/// Settings file, stored in the app data directory.
const STORE_FILE: &str = "settings.json";
const NOTES_DIR_KEY: &str = "notesDir";
const MODE_KEY: &str = "mode";
const HOTKEY_KEY: &str = "hotkey";

/// Settings as shown in the settings window.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    notes_dir: String,
    notes_dir_is_default: bool,
    mode: Mode,
    hotkey: String,
    hotkey_is_default: bool,
    autostart: bool,
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

/// Whether notes go to a file per day or a single inbox; daily unless changed in settings.
pub fn mode(app: &AppHandle) -> Mode {
    app.store(STORE_FILE)
        .ok()
        .and_then(|store| store.get(MODE_KEY))
        .and_then(|mode| serde_json::from_value(mode).ok())
        .unwrap_or_default()
}

/// Global hotkey that opens the input window; Ctrl+Alt+N unless changed in settings.
pub fn hotkey(app: &AppHandle) -> Shortcut {
    app.store(STORE_FILE)
        .ok()
        .and_then(|store| store.get(HOTKEY_KEY))
        .and_then(|hotkey| hotkey.as_str().and_then(|s| hotkey::parse(s).ok()))
        .unwrap_or_else(hotkey::default)
}

/// Registers the saved hotkey at startup.
pub fn register_hotkey(app: &AppHandle) {
    let shortcut = hotkey(app);
    // A hotkey that can't be registered must not stop the app: the tray still works.
    if let Err(e) = app.global_shortcut().register(shortcut) {
        eprintln!(
            "Failed to register global hotkey {}: {e}",
            hotkey::label(&shortcut)
        );
    }
}

/// Switches to a new hotkey, keeping the old one if the new one can't be registered.
fn change_hotkey(app: &AppHandle, new: Shortcut) -> Result<(), String> {
    let old = hotkey(app);
    let shortcuts = app.global_shortcut();
    let _ = shortcuts.unregister(old);
    if let Err(e) = shortcuts.register(new) {
        let _ = shortcuts.register(old);
        return Err(format!("Couldn't set {}: {e}", hotkey::label(&new)));
    }
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    if new == hotkey::default() {
        store.delete(HOTKEY_KEY);
    } else {
        store.set(HOTKEY_KEY, new.into_string());
    }
    store.save().map_err(|e| e.to_string())
}

fn view(app: &AppHandle) -> Result<SettingsView, String> {
    let notes_dir = notes_dir(app).map_err(|e| e.to_string())?;
    let hotkey = hotkey(app);
    Ok(SettingsView {
        notes_dir: notes_dir.to_string_lossy().into_owned(),
        notes_dir_is_default: custom_notes_dir(app).is_none(),
        mode: mode(app),
        hotkey: hotkey::label(&hotkey),
        hotkey_is_default: hotkey == hotkey::default(),
        autostart: app.autolaunch().is_enabled().map_err(|e| e.to_string())?,
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

#[tauri::command]
pub fn set_mode(app: AppHandle, mode: Mode) -> Result<SettingsView, String> {
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    store.set(
        MODE_KEY,
        serde_json::to_value(mode).map_err(|e| e.to_string())?,
    );
    store.save().map_err(|e| e.to_string())?;
    view(&app)
}

/// Sets the global hotkey from a string like `Ctrl+Alt+KeyN`.
#[tauri::command]
pub fn set_hotkey(app: AppHandle, hotkey: String) -> Result<SettingsView, String> {
    change_hotkey(&app, hotkey::parse(&hotkey)?)?;
    view(&app)
}

#[tauri::command]
pub fn reset_hotkey(app: AppHandle) -> Result<SettingsView, String> {
    change_hotkey(&app, hotkey::default())?;
    view(&app)
}

/// Turns launching at Windows sign-in on or off (a `Run` entry in the user registry).
#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<SettingsView, String> {
    let autolaunch = app.autolaunch();
    let result = if enabled {
        autolaunch.enable()
    } else {
        autolaunch.disable()
    };
    result.map_err(|e| e.to_string())?;
    view(&app)
}
