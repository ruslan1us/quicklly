use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::settings;

/// How often a running app looks for a new release.
const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);

/// The version of a newer release, once one has been found and downloaded.
#[derive(Default)]
pub struct AvailableUpdate(Mutex<Option<String>>);

/// Where a downloaded (and already signature-checked) release waits for the next start.
fn download_path(app: &AppHandle, version: &str) -> Option<PathBuf> {
    let dir = app.path().app_cache_dir().ok()?;
    Some(dir.join(format!("update-{version}.bin")))
}

/// Looks for updates now and then every few hours, in the background, unless automatic
/// updates are turned off in settings.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let mut at_startup = true;
        loop {
            if settings::auto_update(&app) {
                run_check(&app, at_startup);
            }
            at_startup = false;
            std::thread::sleep(CHECK_EVERY);
        }
    });
}

fn run_check(app: &AppHandle, at_startup: bool) {
    if let Err(e) = tauri::async_runtime::block_on(check(app, at_startup)) {
        eprintln!("Update check failed: {e}");
    }
}

/// Turning automatic updates on looks for one right away; turning them off stops announcing
/// a release found earlier.
pub fn auto_update_changed(app: &AppHandle, enabled: bool) {
    if enabled {
        let app = app.clone();
        std::thread::spawn(move || run_check(&app, false));
    } else {
        *app.state::<AvailableUpdate>().0.lock().unwrap() = None;
        let _ = app.emit("update-available", None::<String>);
    }
}

/// Downloads a newer release for the next start. At startup, a release downloaded earlier is
/// installed right away instead (on Windows this exits and restarts the app).
async fn check(app: &AppHandle, at_startup: bool) -> Result<(), String> {
    let Some(update) = find(app).await? else {
        return Ok(());
    };
    let path = download_path(app, &update.version);
    match path.as_ref().and_then(|path| fs::read(path).ok()) {
        Some(bytes) if at_startup => return install(app, &update, bytes),
        Some(_) => {}
        None => {
            let bytes = download(&update).await?;
            if let Some(path) = &path {
                if let Some(dir) = path.parent() {
                    let _ = fs::create_dir_all(dir);
                }
                let _ = fs::write(path, bytes);
            }
        }
    }
    *app.state::<AvailableUpdate>().0.lock().unwrap() = Some(update.version.clone());
    let _ = app.emit("update-available", &update.version);
    Ok(())
}

async fn find(app: &AppHandle) -> Result<Option<Update>, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    updater.check().await.map_err(|e| e.to_string())
}

/// Downloads a release and checks its signature.
async fn download(update: &Update) -> Result<Vec<u8>, String> {
    update
        .download(|_, _| {}, || {})
        .await
        .map_err(|e| e.to_string())
}

fn install(app: &AppHandle, update: &Update, bytes: Vec<u8>) -> Result<(), String> {
    if let Some(path) = download_path(app, &update.version) {
        let _ = fs::remove_file(path);
    }
    update.install(bytes).map_err(|e| e.to_string())
}

/// The version of the newer release waiting to be installed, if any.
#[tauri::command]
pub fn get_update(app: AppHandle) -> Option<String> {
    app.state::<AvailableUpdate>().0.lock().unwrap().clone()
}

/// Installs the newer release now (the `/update` command); the app restarts into it.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    let update = find(&app).await?.ok_or("Quicklly is already up to date.")?;
    let downloaded = download_path(&app, &update.version).and_then(|path| fs::read(path).ok());
    let bytes = match downloaded {
        Some(bytes) => bytes,
        None => download(&update).await?,
    };
    install(&app, &update, bytes)
}
