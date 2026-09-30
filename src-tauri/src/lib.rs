mod hotkey;
mod monitor;
mod notes;
mod pad;
mod reader;
mod reader_window;
mod settings;
mod updater;
mod window_bounds;

use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

use serde::{Deserialize, Serialize};
use tauri::utils::config::WindowEffectsConfig;
use tauri::window::{Effect, EffectsBuilder};
use tauri::{App, AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_global_shortcut::ShortcutState;
use tauri_plugin_opener::OpenerExt;

use reader_window::READER_WINDOW;

const MAIN_WINDOW: &str = "main";
const SETTINGS_WINDOW: &str = "settings";
const HELP_WINDOW: &str = "help";

/// Appends `text` to today's note file in the notes folder.
#[tauri::command]
fn save_note(app: AppHandle, text: String) -> Result<(), String> {
    let dir = settings::notes_dir(&app).map_err(|e| e.to_string())?;
    let mode = settings::mode(&app);
    notes::append_note(&dir, mode, chrono::Local::now().naive_local(), &text)
        .map_err(|e| format!("Failed to save note in {}: {e}", dir.display()))?;
    notes_changed(&app);
    Ok(())
}

/// Tells the reader that a notes file changed, so a pinned reader shows it right away.
fn notes_changed(app: &AppHandle) {
    let _ = app.emit_to(READER_WINDOW, "notes-changed", ());
}

fn show_input(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        monitor::center_on_active(&window);
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn open_notes_folder(app: &AppHandle) {
    let result = settings::notes_dir(app)
        .map_err(|e| e.to_string())
        .and_then(|dir| {
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            app.opener()
                .open_path(dir.to_string_lossy(), None::<&str>)
                .map_err(|e| e.to_string())
        });
    if let Err(e) = result {
        eprintln!("Failed to open notes folder: {e}");
    }
}

/// Opens settings from the input window (the `/config` command).
///
/// Async on purpose: creating a window from a sync command deadlocks on Windows.
#[tauri::command]
async fn open_settings(app: AppHandle) {
    show_settings(&app);
}

/// Quits the app from the input window (the `/exit` command), like "Quit" in the tray.
#[tauri::command]
async fn exit_app(app: AppHandle) {
    app.exit(0);
}

/// Shows a terminal-style popup window, creating it on first use so it costs nothing until
/// opened. The page sizes the window to its content and then shows it, so it never flickers.
fn show_popup(app: &AppHandle, label: &str, page: &str, title: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let zoom = settings::scale(app);
    let result = WebviewWindowBuilder::new(app, label, WebviewUrl::App(page.into()))
        .title(title)
        .inner_size(POPUP_WIDTH * zoom, 200.0)
        .visible(false)
        // The page shows and focuses the window once it is ready.
        .focused(false)
        // A frameless popup like the input window; it is dragged by its header.
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .transparent(true)
        .effects(blur_behind())
        .build();
    match result {
        Ok(window) => {
            let _ = window.set_zoom(zoom);
            if label == READER_WINDOW {
                reader_window::arrange(&window);
            }
        }
        Err(e) => eprintln!("Failed to open {label} window: {e}"),
    }
}

/// Width of the input window and the popups at normal size, in logical pixels.
const POPUP_WIDTH: f64 = 640.0;

/// The blurred desktop behind every window, which shows through as much as the
/// transparency setting lets the background through.
pub(crate) fn blur_behind() -> WindowEffectsConfig {
    EffectsBuilder::new().effect(Effect::Acrylic).build()
}

/// Zooms every window to the scale from settings. The input window and the popups then size
/// themselves to their content at the new scale; the Pad is sized here.
pub(crate) fn apply_scale(app: &AppHandle) {
    let zoom = settings::scale(app);
    for (label, window) in app.webview_windows() {
        let _ = window.set_zoom(zoom);
        if label == pad::PAD_WINDOW {
            pad::rescale(&window);
        }
    }
}

fn show_settings(app: &AppHandle) {
    show_popup(app, SETTINGS_WINDOW, "settings.html", "Quicklly Settings");
}

/// Opens the help from the input window (the `/help` command).
///
/// Async on purpose: creating a window from a sync command deadlocks on Windows.
#[tauri::command]
async fn open_help(app: AppHandle) {
    show_popup(&app, HELP_WINDOW, "help.html", "Quicklly Help");
}

/// Opens the notes reader from the input window (← in an empty note).
///
/// Async on purpose: creating a window from a sync command deadlocks on Windows.
#[tauri::command]
async fn open_reader(app: AppHandle) {
    show_popup(&app, READER_WINDOW, "reader.html", "Quicklly Notes");
}

/// Reads one notes file for the reader.
#[tauri::command]
fn read_note_file(app: AppHandle, name: String) -> Result<Vec<reader::Item>, String> {
    let dir = settings::notes_dir(&app).map_err(|e| e.to_string())?;
    reader::read_file(&dir, &name).map_err(|e| format!("Failed to read {name}: {e}"))
}

/// Marks a note in a notes file as done or not done.
#[tauri::command]
fn set_note_done(
    app: AppHandle,
    name: String,
    note: reader::NoteRef,
    done: bool,
) -> Result<(), String> {
    let dir = settings::notes_dir(&app).map_err(|e| e.to_string())?;
    reader::set_done(&dir, &name, &note, done)
        .map_err(|e| format!("Couldn't update {name}: {e}"))?;
    notes_changed(&app);
    Ok(())
}

/// Deletes a note from a notes file.
#[tauri::command]
fn delete_note(app: AppHandle, name: String, note: reader::NoteRef) -> Result<(), String> {
    let dir = settings::notes_dir(&app).map_err(|e| e.to_string())?;
    reader::delete(&dir, &name, &note).map_err(|e| format!("Couldn't update {name}: {e}"))?;
    notes_changed(&app);
    Ok(())
}

/// Replaces the text of a note in a notes file.
#[tauri::command]
fn edit_note(
    app: AppHandle,
    name: String,
    note: reader::NoteRef,
    text: String,
) -> Result<(), String> {
    let dir = settings::notes_dir(&app).map_err(|e| e.to_string())?;
    reader::set_text(&dir, &name, &note, &text)
        .map_err(|e| format!("Couldn't update {name}: {e}"))?;
    notes_changed(&app);
    Ok(())
}

/// Finds notes containing `query` in all notes files, for search in the input window.
#[tauri::command]
fn search_notes(app: AppHandle, query: String) -> Result<Vec<reader::Hit>, String> {
    let dir = settings::notes_dir(&app).map_err(|e| e.to_string())?;
    reader::search(&dir, &query, 50).map_err(|e| format!("Search failed: {e}"))
}

/// A note for the reader to open, e.g. a search result.
#[derive(Clone, Serialize, Deserialize)]
struct ReaderTarget {
    file: String,
    line: usize,
}

/// The note the reader should show next; taken by the reader once it is ready.
#[derive(Default)]
struct PendingReaderTarget(Mutex<Option<ReaderTarget>>);

/// Opens the reader on a note (Enter on a search result).
///
/// Async on purpose: creating a window from a sync command deadlocks on Windows.
#[tauri::command]
async fn open_note(app: AppHandle, file: String, line: usize) {
    *app.state::<PendingReaderTarget>().0.lock().unwrap() = Some(ReaderTarget { file, line });
    if app.get_webview_window(READER_WINDOW).is_some() {
        let _ = app.emit_to(READER_WINDOW, "reader-target", ());
    }
    show_popup(&app, READER_WINDOW, "reader.html", "Quicklly Notes");
}

/// Hands the pending note, if any, to the reader.
#[tauri::command]
fn take_reader_target(app: AppHandle) -> Option<ReaderTarget> {
    app.state::<PendingReaderTarget>().0.lock().unwrap().take()
}

/// Goes back from the reader to the note input (→ in the file list).
#[tauri::command]
fn open_input(app: AppHandle) {
    show_input(&app);
}

/// Lists the notes files for the reader.
#[tauri::command]
fn list_note_files(app: AppHandle) -> Result<Vec<reader::NoteFile>, String> {
    let dir = settings::notes_dir(&app).map_err(|e| e.to_string())?;
    reader::list_files(&dir).map_err(|e| format!("Failed to read {}: {e}", dir.display()))
}

fn setup_tray(app: &App) -> tauri::Result<()> {
    let new_note = MenuItem::with_id(app, "new_note", "New note", true, None::<&str>)?;
    let open_folder =
        MenuItem::with_id(app, "open_folder", "Open notes folder", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &new_note,
            &open_folder,
            &settings,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("Quicklly")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "new_note" => show_input(app),
            "open_folder" => open_notes_folder(app),
            "settings" => show_settings(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_input(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be registered first: a second launch exits here and opens the input
        // window of the already running instance instead.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_input(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                // The only registered shortcut is the new-note hotkey.
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        match settings::hotkey_target(app) {
                            settings::HotkeyTarget::Input => show_input(app),
                            settings::HotkeyTarget::Pad => pad::show(app),
                        }
                    }
                })
                .build(),
        )
        .manage(PendingReaderTarget::default())
        .manage(updater::AvailableUpdate::default())
        .manage(pad::Pinned::default())
        .manage(reader_window::Pinned::default())
        .manage(pad::PendingEdit::default())
        .setup(|app| {
            setup_tray(app)?;
            settings::register_hotkey(app.handle());
            updater::start(app.handle());
            pad::load(app.handle());
            reader_window::load(app.handle());
            if let Some(input) = app.get_webview_window(MAIN_WINDOW) {
                let _ = input.set_effects(blur_behind());
            }
            apply_scale(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == READER_WINDOW {
                let pinned = reader_window::is_pinned(window.app_handle());
                match event {
                    // Clicking elsewhere closes an unpinned reader; not while it is still being
                    // set up hidden, when Windows may already report it losing the focus.
                    WindowEvent::Focused(false)
                        if !pinned && window.is_visible().unwrap_or(false) =>
                    {
                        let _ = window.close();
                    }
                    WindowEvent::Moved(_) | WindowEvent::Resized(_) if pinned => {
                        reader_window::remember_bounds(window);
                    }
                    _ => {}
                }
                return;
            }
            // The input window and the Pad live hidden; other windows close normally.
            if window.label() != MAIN_WINDOW && window.label() != pad::PAD_WINDOW {
                return;
            }
            let pinned_pad =
                window.label() == pad::PAD_WINDOW && pad::is_pinned(window.app_handle());
            match event {
                // Clicking elsewhere dismisses the window, keeping the typed draft; a pinned Pad
                // stays open.
                WindowEvent::Focused(false) if !pinned_pad => {
                    let _ = window.hide();
                }
                WindowEvent::Moved(_) | WindowEvent::Resized(_) if pinned_pad => {
                    pad::remember_bounds(window);
                }
                // Alt+F4 only hides the window; the app keeps living in the tray until "Quit".
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = window.hide();
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            save_note,
            open_settings,
            exit_app,
            open_reader,
            open_help,
            reader_window::get_reader_pin,
            reader_window::get_reader_sort,
            reader_window::set_reader_sort,
            monitor::center_window,
            reader_window::set_reader_pin,
            open_input,
            list_note_files,
            read_note_file,
            set_note_done,
            delete_note,
            edit_note,
            search_notes,
            open_note,
            take_reader_target,
            updater::get_update,
            pad::open_pad,
            pad::get_pad_draft,
            pad::set_pad_draft,
            pad::get_pad_pinned,
            pad::set_pad_pinned,
            pad::expand_to_pad,
            pad::edit_in_pad,
            pad::take_pad_edit,
            settings::set_hotkey_target,
            settings::set_pad_position,
            updater::install_update,
            settings::get_settings,
            settings::pick_notes_dir,
            settings::reset_notes_dir,
            settings::set_mode,
            settings::set_hotkey,
            settings::reset_hotkey,
            settings::set_autostart,
            settings::set_auto_update,
            settings::get_appearance,
            settings::set_transparency,
            settings::set_scale,
            settings::set_theme,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
