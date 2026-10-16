use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewWindow, Window,
};
use tauri_plugin_store::StoreExt;

use crate::settings::{self, STORE_FILE};

/// Where and how big a pinned window was, in physical pixels.
#[derive(Serialize, Deserialize)]
struct Bounds {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

/// Puts a window back where it was saved under `key`; returns false if nothing was saved, or if
/// the saved place is on no screen now (a screen was disconnected): then only the size is put
/// back and the window is left where it is.
pub fn restore(window: &WebviewWindow, key: &str) -> bool {
    let saved = window
        .app_handle()
        .store(STORE_FILE)
        .ok()
        .and_then(|store| store.get(key))
        .and_then(|bounds| serde_json::from_value::<Bounds>(bounds).ok());
    let Some(bounds) = saved else {
        return false;
    };
    let _ = window.set_size(PhysicalSize::new(bounds.width, bounds.height));
    if let Ok(monitors) = window.available_monitors() {
        let screens: Vec<Bounds> = monitors
            .iter()
            .map(|monitor| Bounds {
                x: monitor.position().x,
                y: monitor.position().y,
                width: monitor.size().width,
                height: monitor.size().height,
            })
            .collect();
        if !on_screen(&bounds, &screens) {
            return false;
        }
    }
    let _ = window.set_position(PhysicalPosition::new(bounds.x, bounds.y));
    true
}

/// How much of the top of a window, where it is dragged, has to be on a screen, in physical
/// pixels.
const GRAB_HEIGHT: u32 = 32;

/// Whether the top of a window is on one of the screens, so it can be seen and dragged.
fn on_screen(window: &Bounds, screens: &[Bounds]) -> bool {
    let (left, top) = (i64::from(window.x), i64::from(window.y));
    let right = left + i64::from(window.width);
    let bottom = top + i64::from(window.height.min(GRAB_HEIGHT));
    screens.iter().any(|screen| {
        let (screen_left, screen_top) = (i64::from(screen.x), i64::from(screen.y));
        left < screen_left + i64::from(screen.width)
            && screen_left < right
            && top < screen_top + i64::from(screen.height)
            && screen_top < bottom
    })
}

/// Saves where a window is and how big it is under `key`.
pub fn save(window: &Window, key: &str) {
    if let (Ok(position), Ok(size)) = (window.outer_position(), window.inner_size()) {
        store(window.app_handle(), key, position, size);
    }
}

/// [`save`] for a window with a webview.
pub fn save_webview(window: &WebviewWindow, key: &str) {
    if let (Ok(position), Ok(size)) = (window.outer_position(), window.inner_size()) {
        store(window.app_handle(), key, position, size);
    }
}

fn store(app: &AppHandle, key: &str, position: PhysicalPosition<i32>, size: PhysicalSize<u32>) {
    let bounds = Bounds {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    };
    if let (Ok(store), Ok(bounds)) = (app.store(STORE_FILE), serde_json::to_value(bounds)) {
        store.set(key, bounds);
    }
}

/// Lets a pinned window be resized, but not below `min` (logical pixels at normal scale), so
/// it can't be squeezed into a sliver; an unpinned window keeps the size its page gives it.
pub fn set_resizable(window: &WebviewWindow, resizable: bool, min: (f64, f64)) {
    let _ = window.set_resizable(resizable);
    let zoom = settings::scale(window.app_handle());
    let min_size = resizable.then(|| LogicalSize::new(min.0 * zoom, min.1 * zoom));
    let _ = window.set_min_size(min_size);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, width: u32, height: u32) -> Bounds {
        Bounds {
            x,
            y,
            width,
            height,
        }
    }

    /// A 1920×1080 primary screen and a 1280×1024 one to its left.
    fn screens() -> Vec<Bounds> {
        vec![rect(0, 0, 1920, 1080), rect(-1280, 0, 1280, 1024)]
    }

    #[test]
    fn a_window_fully_on_a_screen_is_on_screen() {
        assert!(on_screen(&rect(100, 100, 560, 380), &screens()));
    }

    #[test]
    fn a_window_partly_on_a_screen_is_on_screen() {
        assert!(on_screen(&rect(1700, 900, 560, 380), &screens()));
        assert!(on_screen(&rect(-1500, 200, 560, 380), &screens()));
    }

    #[test]
    fn a_window_on_the_screen_left_of_the_primary_one_is_on_screen() {
        assert!(on_screen(&rect(-1000, 300, 560, 380), &screens()));
    }

    #[test]
    fn a_window_off_every_screen_is_not_on_screen() {
        // Right of the primary screen, where a disconnected second screen was.
        assert!(!on_screen(&rect(2200, 100, 560, 380), &screens()));
        // Left of the left screen.
        assert!(!on_screen(&rect(-2000, 100, 560, 380), &screens()));
        // Left of the primary screen once the left screen is gone.
        assert!(!on_screen(&rect(-1000, 300, 560, 380), &screens()[..1]));
    }

    #[test]
    fn a_window_with_only_its_bottom_on_a_screen_is_not_on_screen() {
        assert!(!on_screen(&rect(100, -300, 560, 380), &screens()));
    }

    #[test]
    fn a_window_touching_a_screen_edge_is_not_on_screen() {
        assert!(!on_screen(&rect(1920, 100, 560, 380), &screens()));
    }
}
