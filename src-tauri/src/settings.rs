use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
use tauri_plugin_store::StoreExt;

use crate::hotkey;
use crate::notes::Mode;

/// Settings file, stored in the app data directory.
pub const STORE_FILE: &str = "settings.json";
const NOTES_DIR_KEY: &str = "notesDir";
const MODE_KEY: &str = "mode";
const HOTKEY_KEY: &str = "hotkey";
const THEME_KEY: &str = "theme";
const AUTO_UPDATE_KEY: &str = "autoUpdate";

const HOTKEY_TARGET_KEY: &str = "hotkeyOpens";
const PAD_POSITION_KEY: &str = "padPosition";
const TRANSPARENCY_KEY: &str = "transparency";
const SCALE_KEY: &str = "scale";
const DATE_FORMAT_KEY: &str = "dateFormat";
const ARCHIVE_AFTER_KEY: &str = "archiveAfter";

/// What the global hotkey opens.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HotkeyTarget {
    /// The one-line input window.
    #[default]
    Input,
    /// The Pad, for longer notes.
    Pad,
}

/// Where an unpinned Pad pops up.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PadPosition {
    #[default]
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
    Center,
}

/// Colour theme of all windows.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    /// Neutral dark.
    #[default]
    Default,
    /// Dark purple, in the colours of the app icon.
    DefaultPlus,
    /// Light lavender.
    Light,
}

/// How dates in notes are shown and typed; files always keep them as `📅 YYYY-MM-DD`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DateFormat {
    #[default]
    #[serde(rename = "DD/MM/YYYY")]
    DayMonthYear,
    #[serde(rename = "MM/DD/YYYY")]
    MonthDayYear,
    #[serde(rename = "YYYY-MM-DD")]
    YearMonthDay,
    #[serde(rename = "DD.MM.YYYY")]
    DayMonthYearDots,
}

impl DateFormat {
    /// `date` as the windows show it in this format, like `formatDate` in `dates.ts`.
    pub fn format(self, date: NaiveDate) -> String {
        let pattern = match self {
            DateFormat::DayMonthYear => "%d/%m/%Y",
            DateFormat::MonthDayYear => "%m/%d/%Y",
            DateFormat::YearMonthDay => "%Y-%m-%d",
            DateFormat::DayMonthYearDots => "%d.%m.%Y",
        };
        date.format(pattern).to_string()
    }
}

/// How long after a note is done it is moved to the archive.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArchiveAfter {
    Never,
    OneDay,
    ThreeDays,
    #[default]
    SevenDays,
    FourteenDays,
    ThirtyDays,
}

impl ArchiveAfter {
    /// The number of days, or `None` when done notes are never archived.
    pub fn days(self) -> Option<i64> {
        match self {
            ArchiveAfter::Never => None,
            ArchiveAfter::OneDay => Some(1),
            ArchiveAfter::ThreeDays => Some(3),
            ArchiveAfter::SevenDays => Some(7),
            ArchiveAfter::FourteenDays => Some(14),
            ArchiveAfter::ThirtyDays => Some(30),
        }
    }
}

/// Settings as shown in the settings window.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    notes_dir: String,
    notes_dir_is_default: bool,
    mode: Mode,
    hotkey: String,
    hotkey_is_default: bool,
    theme: Theme,
    hotkey_target: HotkeyTarget,
    pad_position: PadPosition,
    autostart: bool,
    auto_update: bool,
    transparency: u32,
    scale: u32,
    date_format: DateFormat,
    archive_after: ArchiveAfter,
}

/// How every window looks: its colours, how much of the blurred desktop shows through its
/// background (in percent), and how big it is drawn (in percent).
#[derive(Clone, Serialize)]
pub struct Appearance {
    theme: Theme,
    transparency: u32,
    scale: u32,
}

/// The default notes folder: `Quicklly` in the Documents folder, or in the home folder where
/// there is none (Linux without `XDG_DOCUMENTS_DIR`). Works before Tauri starts (see `cli`).
pub fn default_notes_dir() -> Option<PathBuf> {
    default_notes_dir_in(dirs::document_dir(), dirs::home_dir())
}

/// The default notes folder, `Quicklly` in the Documents folder `documents`, else in `home`.
fn default_notes_dir_in(documents: Option<PathBuf>, home: Option<PathBuf>) -> Option<PathBuf> {
    Some(documents.or(home)?.join("Quicklly"))
}

fn custom_notes_dir(app: &AppHandle) -> Option<PathBuf> {
    let store = app.store(STORE_FILE).ok()?;
    chosen_dir(&store.get(NOTES_DIR_KEY)?)
}

/// The notes folder chosen in settings, from its stored value.
fn chosen_dir(value: &serde_json::Value) -> Option<PathBuf> {
    value.as_str().filter(|s| !s.is_empty()).map(PathBuf::from)
}

/// The notes folder chosen in settings (`None` for the default one) and the mode, read straight
/// from the settings file `file`, to save a note without starting the app (see `cli`).
pub fn notes_settings_in(file: &Path) -> (Option<PathBuf>, Mode) {
    let settings: serde_json::Map<String, serde_json::Value> = std::fs::read(file)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    let dir = settings.get(NOTES_DIR_KEY).and_then(chosen_dir);
    let mode = settings
        .get(MODE_KEY)
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .unwrap_or_default();
    (dir, mode)
}

/// Folder where notes are written: the one chosen in settings, or the default one.
pub fn notes_dir(app: &AppHandle) -> tauri::Result<PathBuf> {
    match custom_notes_dir(app) {
        Some(dir) => Ok(dir),
        None => default_notes_dir().ok_or(tauri::Error::UnknownPath),
    }
}

/// Changes the notes folder (`None` for the default one), and watches the new one instead.
fn set_custom_notes_dir(app: &AppHandle, dir: Option<PathBuf>) -> Result<(), String> {
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    match dir {
        Some(dir) => store.set(NOTES_DIR_KEY, dir.to_string_lossy().into_owned()),
        None => {
            store.delete(NOTES_DIR_KEY);
        }
    }
    store.save().map_err(|e| e.to_string())?;
    crate::watcher::start(app);
    Ok(())
}

/// Whether notes go to a file per day or a single inbox; daily unless changed in settings.
pub fn mode(app: &AppHandle) -> Mode {
    stored(app, MODE_KEY)
}

pub fn hotkey_target(app: &AppHandle) -> HotkeyTarget {
    stored(app, HOTKEY_TARGET_KEY)
}

pub fn pad_position(app: &AppHandle) -> PadPosition {
    stored(app, PAD_POSITION_KEY)
}

/// See-through background, in percent: 0 is solid.
pub fn transparency(app: &AppHandle) -> u32 {
    stored::<Option<u32>>(app, TRANSPARENCY_KEY).unwrap_or(DEFAULT_TRANSPARENCY)
}

const DEFAULT_TRANSPARENCY: u32 = 30;

fn scale_percent(app: &AppHandle) -> u32 {
    stored::<Option<u32>>(app, SCALE_KEY).unwrap_or(100)
}

/// How big the windows are drawn: 1.0 is normal size.
pub fn scale(app: &AppHandle) -> f64 {
    f64::from(scale_percent(app)) / 100.0
}

fn appearance(app: &AppHandle) -> Appearance {
    Appearance {
        theme: theme(app),
        transparency: transparency(app),
        scale: scale_percent(app),
    }
}

/// Tells every window to redraw with the new appearance.
fn appearance_changed(app: &AppHandle) -> Result<(), String> {
    app.emit("appearance-changed", appearance(app))
        .map_err(|e| e.to_string())
}

/// A setting saved under `key`, or its default.
fn stored<T: DeserializeOwned + Default>(app: &AppHandle, key: &str) -> T {
    app.store(STORE_FILE)
        .ok()
        .and_then(|store| store.get(key))
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}

fn store_setting(app: &AppHandle, key: &str, value: impl Serialize) -> Result<(), String> {
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    store.set(key, serde_json::to_value(value).map_err(|e| e.to_string())?);
    store.save().map_err(|e| e.to_string())
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

fn theme(app: &AppHandle) -> Theme {
    stored(app, THEME_KEY)
}

/// How dates are shown; `DD/MM/YYYY` unless changed in settings.
pub fn date_format(app: &AppHandle) -> DateFormat {
    stored(app, DATE_FORMAT_KEY)
}

/// How long after a note is done it is archived; a week unless changed in settings.
pub fn archive_after(app: &AppHandle) -> ArchiveAfter {
    stored(app, ARCHIVE_AFTER_KEY)
}

/// How many days after a note is done it is archived, or null when it never is; the reader
/// shows how long a done note has left.
#[tauri::command]
pub fn get_archive_days(app: AppHandle) -> Option<i64> {
    archive_after(&app).days()
}

/// Whether new releases are looked for, downloaded and installed by themselves; on unless
/// turned off in settings. `/update` works either way.
pub fn auto_update(app: &AppHandle) -> bool {
    app.store(STORE_FILE)
        .ok()
        .and_then(|store| store.get(AUTO_UPDATE_KEY))
        .and_then(|enabled| enabled.as_bool())
        .unwrap_or(true)
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
        theme: theme(app),
        hotkey_target: hotkey_target(app),
        pad_position: pad_position(app),
        transparency: transparency(app),
        scale: scale_percent(app),
        date_format: date_format(app),
        archive_after: archive_after(app),
        autostart: app.autolaunch().is_enabled().map_err(|e| e.to_string())?,
        auto_update: auto_update(app),
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

#[tauri::command]
pub fn get_appearance(app: AppHandle) -> Appearance {
    appearance(&app)
}

#[tauri::command]
pub fn set_transparency(app: AppHandle, percent: u32) -> Result<SettingsView, String> {
    store_setting(&app, TRANSPARENCY_KEY, percent.min(90))?;
    appearance_changed(&app)?;
    view(&app)
}

#[tauri::command]
pub fn set_scale(app: AppHandle, percent: u32) -> Result<SettingsView, String> {
    store_setting(&app, SCALE_KEY, percent.clamp(50, 200))?;
    crate::apply_scale(&app);
    appearance_changed(&app)?;
    view(&app)
}

/// Saves the theme and tells every window to switch to it.
#[tauri::command]
pub fn set_theme(app: AppHandle, theme: Theme) -> Result<SettingsView, String> {
    store_setting(&app, THEME_KEY, theme)?;
    appearance_changed(&app)?;
    view(&app)
}

#[tauri::command]
pub fn get_date_format(app: AppHandle) -> DateFormat {
    date_format(&app)
}

/// Saves the date format and tells every window to show dates in it.
#[tauri::command]
pub fn set_date_format(app: AppHandle, format: DateFormat) -> Result<SettingsView, String> {
    store_setting(&app, DATE_FORMAT_KEY, format)?;
    app.emit("date-format-changed", format)
        .map_err(|e| e.to_string())?;
    view(&app)
}

/// Saves when done notes are archived, and archives the ones that are due by it now.
#[tauri::command]
pub fn set_archive_after(app: AppHandle, after: ArchiveAfter) -> Result<SettingsView, String> {
    store_setting(&app, ARCHIVE_AFTER_KEY, after)?;
    crate::archive::archive_done(&app);
    view(&app)
}

#[tauri::command]
pub fn set_auto_update(app: AppHandle, enabled: bool) -> Result<SettingsView, String> {
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    if enabled {
        store.delete(AUTO_UPDATE_KEY);
    } else {
        store.set(AUTO_UPDATE_KEY, false);
    }
    store.save().map_err(|e| e.to_string())?;
    crate::updater::auto_update_changed(&app, enabled);
    view(&app)
}

#[tauri::command]
pub fn set_hotkey_target(app: AppHandle, target: HotkeyTarget) -> Result<SettingsView, String> {
    store_setting(&app, HOTKEY_TARGET_KEY, target)?;
    view(&app)
}

#[tauri::command]
pub fn set_pad_position(app: AppHandle, position: PadPosition) -> Result<SettingsView, String> {
    store_setting(&app, PAD_POSITION_KEY, position)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_notes_folder_is_in_documents_else_in_home() {
        let documents = PathBuf::from("docs");
        let home = PathBuf::from("home");
        assert_eq!(
            default_notes_dir_in(Some(documents.clone()), Some(home.clone())),
            Some(documents.join("Quicklly"))
        );
        assert_eq!(
            default_notes_dir_in(None, Some(home.clone())),
            Some(home.join("Quicklly"))
        );
        assert_eq!(default_notes_dir_in(None, None), None);
    }

    #[test]
    fn date_formats_are_stored_as_the_frontend_writes_them() {
        for (format, name) in [
            (DateFormat::DayMonthYear, "DD/MM/YYYY"),
            (DateFormat::MonthDayYear, "MM/DD/YYYY"),
            (DateFormat::YearMonthDay, "YYYY-MM-DD"),
            (DateFormat::DayMonthYearDots, "DD.MM.YYYY"),
        ] {
            assert_eq!(serde_json::to_value(format).unwrap(), name);
            assert_eq!(
                serde_json::from_value::<DateFormat>(name.into()).unwrap(),
                format
            );
        }
        assert!(serde_json::from_value::<DateFormat>("D/M/Y".into()).is_err());
    }

    #[test]
    fn formats_dates_as_the_windows_show_them() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
        for (format, shown) in [
            (DateFormat::DayMonthYear, "05/10/2026"),
            (DateFormat::MonthDayYear, "10/05/2026"),
            (DateFormat::YearMonthDay, "2026-10-05"),
            (DateFormat::DayMonthYearDots, "05.10.2026"),
        ] {
            assert_eq!(format.format(date), shown);
        }
    }

    #[test]
    fn archive_after_is_stored_as_the_frontend_writes_it() {
        for (after, name, days) in [
            (ArchiveAfter::Never, "never", None),
            (ArchiveAfter::OneDay, "oneDay", Some(1)),
            (ArchiveAfter::ThreeDays, "threeDays", Some(3)),
            (ArchiveAfter::SevenDays, "sevenDays", Some(7)),
            (ArchiveAfter::FourteenDays, "fourteenDays", Some(14)),
            (ArchiveAfter::ThirtyDays, "thirtyDays", Some(30)),
        ] {
            assert_eq!(serde_json::to_value(after).unwrap(), name);
            assert_eq!(
                serde_json::from_value::<ArchiveAfter>(name.into()).unwrap(),
                after
            );
            assert_eq!(after.days(), days);
        }
        assert!(serde_json::from_value::<ArchiveAfter>("7".into()).is_err());
        assert_eq!(ArchiveAfter::default(), ArchiveAfter::SevenDays);
    }

    #[test]
    fn notes_settings_are_read_from_the_settings_file() {
        let dir =
            std::env::temp_dir().join(format!("quicklly-test-settings-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join(STORE_FILE);

        std::fs::write(
            &file,
            r#"{ "notesDir": "D:\\Notes", "mode": "inbox", "scale": 90 }"#,
        )
        .unwrap();
        assert_eq!(
            notes_settings_in(&file),
            (Some(PathBuf::from(r"D:\Notes")), Mode::Inbox)
        );

        // The default folder and mode when they were never changed or the file is unreadable.
        std::fs::write(&file, r#"{ "notesDir": "", "theme": "light" }"#).unwrap();
        assert_eq!(notes_settings_in(&file), (None, Mode::Daily));
        std::fs::write(&file, "not json").unwrap();
        assert_eq!(notes_settings_in(&file), (None, Mode::Daily));
        assert_eq!(
            notes_settings_in(&dir.join("missing.json")),
            (None, Mode::Daily)
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
