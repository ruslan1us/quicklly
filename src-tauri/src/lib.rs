mod hotkey;
mod notes;
mod reader;
mod settings;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_global_shortcut::ShortcutState;
use tauri_plugin_opener::OpenerExt;

const MAIN_WINDOW: &str = "main";
const SETTINGS_WINDOW: &str = "settings";
const READER_WINDOW: &str = "reader";

/// Appends `text` to today's note file in the notes folder.
#[tauri::command]
fn save_note(app: AppHandle, text: String) -> Result<(), String> {
    let dir = settings::notes_dir(&app).map_err(|e| e.to_string())?;
    let mode = settings::mode(&app);
    notes::append_note(&dir, mode, chrono::Local::now().naive_local(), &text)
        .map_err(|e| format!("Failed to save note in {}: {e}", dir.display()))?;
    Ok(())
}

fn show_input(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.center();
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
    let result = WebviewWindowBuilder::new(app, label, WebviewUrl::App(page.into()))
        .title(title)
        .inner_size(640.0, 200.0)
        .visible(false)
        // A frameless popup like the input window; it is dragged by its header.
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .build();
    if let Err(e) = result {
        eprintln!("Failed to open {label} window: {e}");
    }
}

fn show_settings(app: &AppHandle) {
    show_popup(app, SETTINGS_WINDOW, "settings.html", "Quicklly Settings");
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
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                // The only registered shortcut is the new-note hotkey.
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        show_input(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            setup_tray(app)?;
            settings::register_hotkey(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
            // Only the input window lives hidden; other windows close normally.
            if window.label() != MAIN_WINDOW {
                return;
            }
            match event {
                // Clicking elsewhere dismisses the input window, keeping the typed draft.
                WindowEvent::Focused(false) => {
                    let _ = window.hide();
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
            open_input,
            list_note_files,
            read_note_file,
            settings::get_settings,
            settings::pick_notes_dir,
            settings::reset_notes_dir,
            settings::set_mode,
            settings::set_hotkey,
            settings::reset_hotkey,
            settings::set_autostart,
            settings::get_theme,
            settings::set_theme,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
