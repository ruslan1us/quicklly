mod notes;
mod settings;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tauri_plugin_opener::OpenerExt;

const MAIN_WINDOW: &str = "main";
const SETTINGS_WINDOW: &str = "settings";

/// Global hotkey that brings up the input window: Ctrl+Alt+N.
fn new_note_shortcut() -> Shortcut {
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyN)
}

/// Appends `text` to today's note file in the notes folder.
#[tauri::command]
fn save_note(app: AppHandle, text: String) -> Result<(), String> {
    let dir = settings::notes_dir(&app).map_err(|e| e.to_string())?;
    notes::append_note(&dir, chrono::Local::now().naive_local(), &text)
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

/// Shows the settings window, creating it on first use so it costs nothing until opened.
fn show_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(SETTINGS_WINDOW) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let result = WebviewWindowBuilder::new(
        app,
        SETTINGS_WINDOW,
        WebviewUrl::App("settings.html".into()),
    )
    .title("Quicklly Settings")
    .inner_size(560.0, 280.0)
    .resizable(false)
    .maximizable(false)
    .minimizable(false)
    .center()
    .build();
    if let Err(e) = result {
        eprintln!("Failed to open settings window: {e}");
    }
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
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed && *shortcut == new_note_shortcut() {
                        show_input(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            setup_tray(app)?;
            // A taken hotkey must not stop the app: the tray still works without it.
            if let Err(e) = app.global_shortcut().register(new_note_shortcut()) {
                eprintln!("Failed to register global hotkey Ctrl+Alt+N: {e}");
            }
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
            settings::get_settings,
            settings::pick_notes_dir,
            settings::reset_notes_dir,
            settings::set_autostart,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
