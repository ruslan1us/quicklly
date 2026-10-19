mod archive;
mod cli;
mod hotkey;
mod images;
mod monitor;
mod notes;
mod pad;
mod reader;
mod reader_window;
mod reminders;
mod settings;
mod updater;
mod watcher;
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
const CHANGELOG_WINDOW: &str = "changelog";
const DONE_WINDOW: &str = "done";

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

/// Saves the notes of `quicklly://add?text=…` links, which macOS hands to the running app, as
/// the link path of the CLI does elsewhere; other links are ignored. Whoever opened the link gets
/// no answer, so a failure is only logged.
#[cfg(target_os = "macos")]
fn save_links(app: &AppHandle, urls: Vec<tauri::Url>) {
    for url in urls {
        if let Some(text) = cli::link_text(url.as_str()) {
            if let Err(e) = save_note(app.clone(), text) {
                eprintln!("Failed to save the note of a quicklly:// link: {e}");
            }
        }
    }
    // macOS made Quicklly the active app to open the link.
    give_focus_back(app);
}

/// Saves an image pasted into the input window or the Pad in the notes folder's `images/`, and
/// returns its path relative to the notes folder for the note to link to.
///
/// The image comes as the raw request body, its MIME type in the `Image-Type` header.
#[tauri::command]
fn save_image(app: AppHandle, request: tauri::ipc::Request<'_>) -> Result<String, String> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err("No image to save".into());
    };
    let mime = request
        .headers()
        .get("Image-Type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let dir = settings::notes_dir(&app).map_err(|e| e.to_string())?;
    images::save(&dir, chrono::Local::now().naive_local(), mime, bytes)
        .map_err(|e| format!("Failed to save the image in {}: {e}", dir.display()))
}

/// Moves images pasted into the input window or the Pad (`images/…`) that the window let go of
/// without saving them, when its text started over, to the Recycle Bin, unless a notes file, the
/// Pad draft or `kept` (texts the window still holds) mentions them. A tidy-up nobody waits
/// for: a failure is only logged, and leaves the images where they are.
#[tauri::command]
fn trash_unsaved_images(app: AppHandle, paths: Vec<String>, mut kept: Vec<String>) {
    let result = pad::draft(&app).and_then(|draft| {
        kept.push(draft);
        let dir = settings::notes_dir(&app).map_err(|e| e.to_string())?;
        images::trash_unsaved(&dir, &paths, &kept).map_err(|e| e.to_string())
    });
    if let Err(e) = result {
        eprintln!("Failed to tidy up pasted images: {e}");
    }
}

/// Opens a note's images (paths relative to the folder of its notes file `name`) in the default
/// viewer, from the reader. Only pasted images, right in `images/`: a link to anything else, a
/// program say, opens nothing, not even the note's other images.
#[tauri::command]
fn open_images(app: AppHandle, name: String, paths: Vec<String>) -> Result<(), String> {
    let dir = settings::notes_dir(&app).map_err(|e| e.to_string())?;
    let files = paths
        .iter()
        .map(|path| {
            images::pasted_image(&dir, &name, path)
                .ok_or_else(|| format!("Not opened: {path} is not an image in the images folder"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    for (path, file) in paths.iter().zip(files) {
        app.opener()
            .open_path(file.to_string_lossy(), None::<&str>)
            .map_err(|e| format!("Couldn't open {path}: {e}"))?;
    }
    Ok(())
}

/// Tells the reader, the reminders and the done journal that a notes file changed, so a pinned
/// reader shows it right away and the others stay up to date.
pub(crate) fn notes_changed(app: &AppHandle) {
    let _ = app.emit_to(READER_WINDOW, "notes-changed", ());
    let _ = app.emit_to(reminders::REMINDERS_WINDOW, "notes-changed", ());
    let _ = app.emit_to(DONE_WINDOW, "notes-changed", ());
}

/// Brings Quicklly forward before one of its windows shows with the focus. On macOS it unhides
/// the app, which `give_focus_back` may have hidden: the windows of a hidden app don't show.
#[cfg(target_os = "macos")]
pub(crate) fn come_forward(app: &AppHandle) {
    let _ = app.show();
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn come_forward(_app: &AppHandle) {}

/// Hands the keys back to the app in use before, once none of Quicklly's windows is on screen.
/// Windows does that by itself when a window hides; macOS leaves Quicklly the active app with no
/// window, until the app hides itself. Not while a popup is still being set up hidden, as it is
/// about to show: popups are closed when they are done, while the input window and the Pad live
/// hidden.
#[cfg(target_os = "macos")]
fn give_focus_back(app: &AppHandle) {
    let showing = app.webview_windows().into_iter().any(|(label, window)| {
        (label != MAIN_WINDOW && label != pad::PAD_WINDOW) || window.is_visible().unwrap_or(false)
    });
    if !showing {
        let _ = app.hide();
    }
}

fn show_input(app: &AppHandle) {
    come_forward(app);
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

/// The page of a window: each one is `src/windows/<name>/index.html`, built to `dist/windows/`.
pub(crate) fn page(name: &str) -> WebviewUrl {
    WebviewUrl::App(format!("windows/{name}/index.html").into())
}

/// Shows a terminal-style popup window, creating it on first use so it costs nothing until
/// opened. The page sizes the window to its content and then shows it, so it never flickers.
fn show_popup(app: &AppHandle, label: &str, name: &str, title: &str) {
    come_forward(app);
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let zoom = settings::scale(app);
    let result = WebviewWindowBuilder::new(app, label, page(name))
        .title(title)
        .inner_size(POPUP_WIDTH * zoom, 200.0)
        .visible(false)
        // The page shows and focuses the window once it is ready.
        .focused(false)
        // A frameless popup like the input window; it is dragged by its header.
        .decorations(false)
        .always_on_top(true)
        .visible_on_all_workspaces(ON_EVERY_SPACE)
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

/// Whether the windows are on every Space (desktop), on macOS: a window shows on the Space in
/// use, and a pinned one comes along to the next, instead of macOS switching to the Space where
/// it was. Only there: Windows has no such option and Linux would stick the windows to every
/// workspace.
pub(crate) const ON_EVERY_SPACE: bool = cfg!(target_os = "macos");

/// The blurred desktop behind every window, which shows through as much as the
/// transparency setting lets the background through.
#[cfg(not(target_os = "macos"))]
pub(crate) fn blur_behind() -> WindowEffectsConfig {
    EffectsBuilder::new().effect(Effect::Acrylic).build()
}

/// The blurred desktop behind every window on macOS, where Acrylic doesn't exist: the dark HUD
/// vibrancy. It stays on while the window doesn't have the keys, as the reminders shown without
/// the focus or a pinned Pad in the background; by default it would look grey then.
#[cfg(target_os = "macos")]
pub(crate) fn blur_behind() -> WindowEffectsConfig {
    EffectsBuilder::new()
        .effect(Effect::HudWindow)
        .state(tauri::window::EffectState::Active)
        .build()
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
    show_popup(app, SETTINGS_WINDOW, "settings", "Quicklly Settings");
}

/// Opens the help from the input window (the `/help` command).
///
/// Async on purpose: creating a window from a sync command deadlocks on Windows.
#[tauri::command]
async fn open_help(app: AppHandle) {
    show_popup(&app, HELP_WINDOW, "help", "Quicklly Help");
}

/// Opens the changelog from the input window (the `/changelog` command).
///
/// Async on purpose: creating a window from a sync command deadlocks on Windows.
#[tauri::command]
async fn open_changelog(app: AppHandle) {
    show_popup(&app, CHANGELOG_WINDOW, "changelog", "Quicklly Changelog");
}

/// Opens the journal of the notes done lately from the input window (the `/done` command).
///
/// Async on purpose: creating a window from a sync command deadlocks on Windows.
#[tauri::command]
async fn open_done(app: AppHandle) {
    show_popup(&app, DONE_WINDOW, "done", "Quicklly Done");
}

/// Runs `read`, a command's work that reads every notes file and so may take a while in a big
/// notes folder, on a thread of its own, so the windows, the tray and the hotkey don't wait for it.
///
/// Only for commands that read: those that write stay sync, on the main thread, so they never run
/// at the same time as each other.
pub(crate) async fn in_background<T: Send + 'static>(
    read: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(read)
        .await
        .map_err(|e| e.to_string())?
}

/// Lists the notes done in the last `days` days, today included, archived ones too, for the
/// journal.
#[tauri::command]
async fn list_done(app: AppHandle, days: u64) -> Result<Vec<reader::Done>, String> {
    in_background(move || {
        let dir = settings::notes_dir(&app).map_err(|e| e.to_string())?;
        let from = chrono::Local::now().date_naive() - chrono::Days::new(days - 1);
        reader::done_since(&dir, from).map_err(|e| format!("Failed to read {}: {e}", dir.display()))
    })
    .await
}

/// Opens the notes reader from the input window (← in an empty note).
///
/// Async on purpose: creating a window from a sync command deadlocks on Windows.
#[tauri::command]
async fn open_reader(app: AppHandle) {
    show_popup(&app, READER_WINDOW, "reader", "Quicklly Notes");
}

/// Reads one notes file for the reader.
#[tauri::command]
fn read_note_file(app: AppHandle, name: String) -> Result<Vec<reader::Item>, String> {
    let dir = settings::notes_dir(&app).map_err(|e| e.to_string())?;
    reader::read_file(&dir, &name).map_err(|e| format!("Failed to read {name}: {e}"))
}

/// Moves a notes file to the Recycle Bin, from the reader's file list, with its images.
#[tauri::command]
fn delete_note_file(app: AppHandle, name: String) -> Result<(), String> {
    let dir = settings::notes_dir(&app).map_err(|e| e.to_string())?;
    let content = reader::notes_file(&dir, &name)
        .and_then(std::fs::read_to_string)
        .unwrap_or_default();
    reader::trash_file(&dir, &name).map_err(|e| format!("Couldn't delete {name}: {e}"))?;
    notes_changed(&app);
    trash_images(&dir, &name, images::paths_in(&content));
    Ok(())
}

/// Moves images that notes no longer link to to the Recycle Bin; `paths` as the notes file
/// `name` links to them. Returns where the images were, to bring them back on undo. Called once
/// the notes file is written, so a failure is only logged: the change itself is made, and the
/// images that couldn't be moved just stay in `images/`.
fn trash_images(dir: &std::path::Path, name: &str, paths: Vec<String>) -> Vec<std::path::PathBuf> {
    images::trash_unused(dir, name, &paths).unwrap_or_else(|e| {
        eprintln!("Failed to delete the images of {name}: {e}");
        Vec::new()
    })
}

/// Marks a note in a notes file as done or not done; returns the change, for the reader to undo.
#[tauri::command]
fn set_note_done(
    app: AppHandle,
    name: String,
    note: reader::NoteRef,
    done: bool,
) -> Result<reader::Change, String> {
    let dir = settings::notes_dir(&app).map_err(|e| e.to_string())?;
    let change = reader::set_done(&dir, &name, &note, done, chrono::Local::now().date_naive())
        .map_err(|e| format!("Couldn't update {name}: {e}"))?;
    notes_changed(&app);
    Ok(change)
}

/// Moves a note's date to `days` from today, from the reminders.
#[tauri::command]
fn snooze_note(
    app: AppHandle,
    name: String,
    note: reader::NoteRef,
    days: u64,
) -> Result<(), String> {
    let dir = settings::notes_dir(&app).map_err(|e| e.to_string())?;
    let date = chrono::Local::now().date_naive() + chrono::Days::new(days);
    reader::snooze(&dir, &name, &note, date).map_err(|e| format!("Couldn't update {name}: {e}"))?;
    notes_changed(&app);
    Ok(())
}

/// Deletes a note from a notes file, with its images; returns the change, for the reader to undo.
#[tauri::command]
fn delete_note(
    app: AppHandle,
    name: String,
    note: reader::NoteRef,
) -> Result<reader::Change, String> {
    let dir = settings::notes_dir(&app).map_err(|e| e.to_string())?;
    let mut change =
        reader::delete(&dir, &name, &note).map_err(|e| format!("Couldn't update {name}: {e}"))?;
    notes_changed(&app);
    change.images = trash_images(&dir, &name, images::paths_in(&note.text));
    Ok(change)
}

/// Replaces the text of a note in a notes file; images taken out of it are deleted. Returns the
/// change, for the reader to undo.
#[tauri::command]
fn edit_note(
    app: AppHandle,
    name: String,
    note: reader::NoteRef,
    text: String,
) -> Result<reader::Change, String> {
    let dir = settings::notes_dir(&app).map_err(|e| e.to_string())?;
    let mut change = reader::set_text(&dir, &name, &note, &text)
        .map_err(|e| format!("Couldn't update {name}: {e}"))?;
    notes_changed(&app);
    change.images = trash_images(&dir, &name, images::paths_in(&note.text));
    Ok(change)
}

/// Undoes a change made in the reader (Ctrl+Z) and brings back the images it deleted; returns
/// how many of them couldn't be brought back.
#[tauri::command]
fn undo_change(app: AppHandle, name: String, change: reader::Change) -> Result<usize, String> {
    let dir = settings::notes_dir(&app).map_err(|e| e.to_string())?;
    reader::undo(&dir, &name, &change).map_err(|e| format!("Couldn't undo: {e}"))?;
    notes_changed(&app);
    Ok(images::restore(&change.images))
}

/// Finds notes containing `query`, as they are shown, in all notes files, for search in the
/// input window.
#[tauri::command]
async fn search_notes(app: AppHandle, query: String) -> Result<Vec<reader::Hit>, String> {
    in_background(move || {
        let dir = settings::notes_dir(&app).map_err(|e| e.to_string())?;
        let format = settings::date_format(&app);
        reader::search(&dir, &query, format, 50).map_err(|e| format!("Search failed: {e}"))
    })
    .await
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
    show_popup(&app, READER_WINDOW, "reader", "Quicklly Notes");
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
async fn list_note_files(app: AppHandle) -> Result<Vec<reader::NoteFile>, String> {
    in_background(move || {
        let dir = settings::notes_dir(&app).map_err(|e| e.to_string())?;
        reader::list_files(&dir).map_err(|e| format!("Failed to read {}: {e}", dir.display()))
    })
    .await
}

/// Lists the tags that have their own file, to suggest them while a `#tag` is typed.
#[tauri::command]
async fn list_tags(app: AppHandle) -> Result<Vec<String>, String> {
    in_background(move || {
        let dir = settings::notes_dir(&app).map_err(|e| e.to_string())?;
        reader::list_tags(&dir).map_err(|e| format!("Failed to read {}: {e}", dir.display()))
    })
    .await
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
    // The macOS menu bar draws a template icon in its own colour, light or dark like the bar,
    // from the icon's shape (its alpha) alone.
    #[cfg(target_os = "macos")]
    {
        tray = tray.icon_as_template(true);
    }
    tray.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let context = tauri::generate_context!();
    // A note from a script or a `quicklly://` link is saved by the process it starts, which
    // then quits without starting the app (or reaching the running one). On macOS a link starts
    // no process: the running app gets it (see `save_links`).
    let args: Vec<String> = std::env::args().collect();
    if let Some(text) = cli::note_request(&args) {
        std::process::exit(cli::save(&context.config().identifier, &text));
    }
    tauri::Builder::default()
        // Must be registered first: a second launch exits here and opens the input
        // window of the already running instance instead.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_input(app);
        }))
        .plugin(tauri_plugin_deep_link::init())
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
        .manage(reminders::FocusOnShow::default())
        .manage(watcher::Watcher::default())
        .setup(|app| {
            // A menu-bar app: no Dock icon and not in ⌘-Tab, as Quicklly is not in the taskbar
            // on Windows.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            // Windows installers register `quicklly://` links; a dev build registers itself.
            // On Linux the app always registers itself: an AppImage registers nothing, and the
            // bundled `.desktop` file starts the app without the link. On macOS the bundle's
            // Info.plist declares the scheme and the plugin can't register it.
            #[cfg(any(all(debug_assertions, windows), target_os = "linux"))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                if let Err(e) = app.deep_link().register_all() {
                    eprintln!("Failed to register quicklly:// links: {e}");
                }
            }
            // macOS hands links to the running app, starting it if needed, and never to a new
            // process, so the app saves them itself; a link that started it came before this.
            #[cfg(target_os = "macos")]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                let handle = app.handle().clone();
                app.deep_link()
                    .on_open_url(move |event| save_links(&handle, event.urls()));
                if let Ok(Some(urls)) = app.deep_link().get_current() {
                    save_links(app.handle(), urls);
                }
            }
            setup_tray(app)?;
            settings::register_hotkey(app.handle());
            updater::start(app.handle());
            pad::load(app.handle());
            reader_window::load(app.handle());
            if let Some(input) = app.get_webview_window(MAIN_WINDOW) {
                let _ = input.set_effects(blur_behind());
                #[cfg(target_os = "macos")]
                let _ = input.set_visible_on_all_workspaces(ON_EVERY_SPACE);
            }
            apply_scale(app.handle());
            reminders::start(app.handle());
            watcher::start(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
            // A window that hid or closed may have been the last one on screen.
            #[cfg(target_os = "macos")]
            if matches!(event, WindowEvent::Focused(false) | WindowEvent::Destroyed) {
                give_focus_back(window.app_handle());
            }
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
            save_image,
            trash_unsaved_images,
            open_images,
            open_settings,
            exit_app,
            open_reader,
            open_help,
            open_changelog,
            open_done,
            list_done,
            reminders::open_reminders,
            reminders::list_due,
            reminders::reveal_reminders,
            reminders::place_reminders,
            reader_window::get_reader_pin,
            reader_window::get_reader_sort,
            reader_window::set_reader_sort,
            monitor::center_window,
            reader_window::set_reader_pin,
            open_input,
            list_note_files,
            list_tags,
            read_note_file,
            delete_note_file,
            set_note_done,
            snooze_note,
            delete_note,
            edit_note,
            undo_change,
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
            settings::get_date_format,
            settings::set_date_format,
            settings::set_archive_after,
            settings::get_archive_days,
        ])
        .build(context)
        .expect("error while running tauri application")
        .run(on_run_event);
}

/// Opening Quicklly in Finder while it runs starts no second process on macOS: the running app
/// is told to reopen instead, and shows its input window, as a second launch does elsewhere.
#[cfg(target_os = "macos")]
fn on_run_event(app: &AppHandle, event: tauri::RunEvent) {
    if let tauri::RunEvent::Reopen { .. } = event {
        show_input(app);
    }
}

#[cfg(not(target_os = "macos"))]
fn on_run_event(_app: &AppHandle, _event: tauri::RunEvent) {}
