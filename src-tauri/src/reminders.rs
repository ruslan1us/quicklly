use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use chrono::{Local, NaiveDate};
use tauri::{
    AppHandle, Emitter, Manager, Monitor, PhysicalPosition, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use crate::{archive, monitor, pad, reader, settings};

/// The reminders: the notes with a date of today or earlier that are not done yet.
pub const REMINDERS_WINDOW: &str = "reminders";

/// How often the date is looked at, to remind of the new day's notes.
const CHECK_EVERY: Duration = Duration::from_secs(60);

/// Whether the reminders window takes the focus when it shows: only when it was asked for
/// (`/reminders`), not when it pops up by itself while something else is being typed in.
#[derive(Default)]
pub struct FocusOnShow(AtomicBool);

/// The notes due today or earlier.
fn due_now(app: &AppHandle) -> Result<Vec<reader::Due>, String> {
    let dir = settings::notes_dir(app).map_err(|e| e.to_string())?;
    reader::due(&dir, Local::now().date_naive())
        .map_err(|e| format!("Failed to read {}: {e}", dir.display()))
}

/// The daily round, at startup and then whenever a new day starts (at midnight, or when the
/// computer wakes up on a later day): first moves the notes done long enough ago to the archive,
/// then shows the reminders if anything is due.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let mut last: Option<NaiveDate> = None;
        loop {
            let today = Local::now().date_naive();
            if last != Some(today) {
                last = Some(today);
                archive::archive_done(&app);
                match due_now(&app) {
                    Ok(due) if !due.is_empty() => show(&app, false),
                    Ok(_) => {}
                    Err(e) => eprintln!("Failed to look for reminders: {e}"),
                }
            }
            std::thread::sleep(CHECK_EVERY);
        }
    });
}

/// Shows the reminders, creating the window if needed; the page sizes it to its content and
/// then shows it. Unless `focus`, it shows without taking the focus. An open window is only
/// told to reload, as days have moved on.
fn show(app: &AppHandle, focus: bool) {
    if let Some(window) = app.get_webview_window(REMINDERS_WINDOW) {
        if focus {
            let _ = window.show();
            let _ = window.set_focus();
        } else {
            let _ = app.emit_to(REMINDERS_WINDOW, "notes-changed", ());
        }
        return;
    }
    app.state::<FocusOnShow>().0.store(focus, Ordering::Relaxed);
    let zoom = settings::scale(app);
    let result = WebviewWindowBuilder::new(
        app,
        REMINDERS_WINDOW,
        WebviewUrl::App("reminders.html".into()),
    )
    .title("Quicklly Reminders")
    .inner_size(crate::POPUP_WIDTH * zoom, 200.0)
    .visible(false)
    // Shown without taking the focus (see `reveal_reminders`); a click on it gives it the keys.
    .focused(false)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .transparent(true)
    .effects(crate::blur_behind())
    .build();
    match result {
        Ok(window) => {
            let _ = window.set_zoom(zoom);
        }
        Err(e) => eprintln!("Failed to open the reminders: {e}"),
    }
}

/// Puts the window in the bottom right corner of `monitor`'s work area.
fn place(window: &WebviewWindow, monitor: &Monitor) {
    // Twice: moving to a screen with another scaling resizes the window.
    for _ in 0..2 {
        let Ok(size) = window.outer_size() else {
            return;
        };
        let area = monitor.work_area();
        let margin = (pad::MARGIN * monitor.scale_factor()) as i32;
        let x = area.position.x + area.size.width as i32 - size.width as i32 - margin;
        let y = area.position.y + area.size.height as i32 - size.height as i32 - margin;
        let _ = window.set_position(PhysicalPosition::new(x, y));
    }
}

/// Opens the reminders from the input window (the `/reminders` command), with the focus.
/// Returns false, and opens nothing, when nothing is due.
///
/// Async on purpose: creating a window from a sync command deadlocks on Windows.
#[tauri::command]
pub async fn open_reminders(app: AppHandle) -> Result<bool, String> {
    if due_now(&app)?.is_empty() {
        return Ok(false);
    }
    show(&app, true);
    Ok(true)
}

/// Lists the notes due today or earlier, for the reminders window.
#[tauri::command]
pub fn list_due(app: AppHandle) -> Result<Vec<reader::Due>, String> {
    due_now(&app)
}

/// Shows the reminders window, once the page is ready, in the corner of the screen in use. It
/// takes the focus only when it was asked for: a window created unfocused shows without
/// activating it.
#[tauri::command]
pub fn reveal_reminders(window: WebviewWindow) {
    if let Some(monitor) = monitor::active(window.app_handle()) {
        place(&window, &monitor);
    }
    let _ = window.show();
    let focus = window
        .state::<FocusOnShow>()
        .0
        .swap(false, Ordering::Relaxed);
    if focus {
        let _ = window.set_focus();
    }
}

/// Keeps the reminders window in its corner after it was resized to its content.
#[tauri::command]
pub fn place_reminders(window: WebviewWindow) {
    if let Ok(Some(monitor)) = window.current_monitor() {
        place(&window, &monitor);
    }
}
