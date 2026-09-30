use tauri::{AppHandle, Manager, Monitor, PhysicalPosition, WebviewWindow};

/// The monitor to show a window on: the one with the window in use, so that on several
/// screens Quicklly opens where you are working. Falls back to the mouse, then the primary one.
pub fn active(app: &AppHandle) -> Option<Monitor> {
    #[cfg(windows)]
    if let Some((x, y)) = foreground_window_center() {
        if let Ok(Some(monitor)) = app.monitor_from_point(x, y) {
            return Some(monitor);
        }
    }
    let at_mouse = app
        .cursor_position()
        .ok()
        .and_then(|mouse| app.monitor_from_point(mouse.x, mouse.y).ok().flatten());
    at_mouse.or_else(|| app.primary_monitor().ok().flatten())
}

/// The middle of the window in use, in physical screen coordinates.
#[cfg(windows)]
fn foreground_window_center() -> Option<(f64, f64)> {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowRect};

    // SAFETY: both calls only read window information; the handle is checked before use.
    unsafe {
        let window = GetForegroundWindow();
        if window.is_invalid() {
            return None;
        }
        let mut rect = RECT::default();
        GetWindowRect(window, &mut rect).ok()?;
        Some((
            f64::from(rect.left + rect.right) / 2.0,
            f64::from(rect.top + rect.bottom) / 2.0,
        ))
    }
}

/// Centres `window` on the monitor in use.
pub fn center_on_active(window: &WebviewWindow) {
    let Some(monitor) = active(window.app_handle()) else {
        let _ = window.center();
        return;
    };
    // Twice: moving to a screen with another scaling resizes the window, which moves its centre.
    for _ in 0..2 {
        let Ok(size) = window.outer_size() else {
            return;
        };
        let area = monitor.work_area();
        let x = area.position.x + (area.size.width as i32 - size.width as i32) / 2;
        let y = area.position.y + (area.size.height as i32 - size.height as i32) / 2;
        let _ = window.set_position(PhysicalPosition::new(x, y));
    }
}

/// Centres the calling window on the monitor in use, as the popups do when they first show.
#[tauri::command]
pub fn center_window(window: WebviewWindow) {
    center_on_active(&window);
}
