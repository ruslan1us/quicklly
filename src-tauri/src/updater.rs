use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use tauri::utils::{config::BundleType, platform::bundle_type};
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
    Some(dir.join(download_name(version)))
}

fn download_name(version: &str) -> String {
    format!("update-{version}.bin")
}

/// The downloads among `names` that can never be installed: every `update-<version>.bin` except
/// the one for `keep`, the latest release (none when the running version is the latest).
fn stale_downloads(names: Vec<String>, keep: Option<&str>) -> Vec<String> {
    let kept = keep.map(download_name);
    names
        .into_iter()
        .filter(|name| {
            let is_download = name
                .strip_prefix("update-")
                .and_then(|rest| rest.strip_suffix(".bin"))
                .is_some_and(|version| !version.is_empty());
            is_download && kept.as_ref() != Some(name)
        })
        .collect()
}

/// Removes the stale downloads from `dir`; see `stale_downloads`.
fn remove_stale_downloads_in(dir: &Path, keep: Option<&str>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return,
        Err(e) => {
            eprintln!("Could not read the update cache {}: {e}", dir.display());
            return;
        }
    };
    let names = entries
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .collect();
    for name in stale_downloads(names, keep) {
        let path = dir.join(name);
        if let Err(e) = fs::remove_file(&path) {
            eprintln!("Could not remove the old update {}: {e}", path.display());
        }
    }
}

/// Removes downloads of releases that were superseded or already installed, keeping only the one
/// for `keep`, the latest release.
fn remove_stale_downloads(app: &AppHandle, keep: Option<&str>) {
    if let Ok(dir) = app.path().app_cache_dir() {
        remove_stale_downloads_in(&dir, keep);
    }
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
/// installed right away instead, and the app restarts into it; a deb or rpm install only
/// announces it (see `installs_at_startup`).
async fn check(app: &AppHandle, at_startup: bool) -> Result<(), String> {
    let Some(update) = find(app).await? else {
        return Ok(());
    };
    let path = download_path(app, &update.version);
    match path.as_ref().and_then(|path| fs::read(path).ok()) {
        Some(bytes) if at_startup && installs_at_startup() => return install(app, &update, bytes),
        Some(_) => {}
        None => {
            let bytes = download(&update).await?;
            let path = path.ok_or("No cache folder to save the update in")?;
            save_download(&path, &bytes)
                .map_err(|e| format!("Could not save the update to {}: {e}", path.display()))?;
        }
    }
    *app.state::<AvailableUpdate>().0.lock().unwrap() = Some(update.version.clone());
    let _ = app.emit("update-available", &update.version);
    Ok(())
}

/// Writes a download to `path` through `<path>.tmp` renamed into place, so `path` holds either a
/// complete download or nothing: the next start would run whatever is there. A failed write
/// removes the temporary file.
fn save_download(path: &Path, bytes: &[u8]) -> io::Result<()> {
    // Two checks can run at once (turning automatic updates on starts one); they take turns, so
    // neither renames the temporary file while the other is still writing it.
    static SAVING: Mutex<()> = Mutex::new(());
    let _saving = SAVING.lock().unwrap();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("bin.tmp");
    let saved = fs::write(&tmp, bytes).and_then(|()| fs::rename(&tmp, path));
    if saved.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    saved
}

/// Looks for a newer release. Only the latest release can be installed, so downloads of any other
/// release (superseded, or the running one, already installed) are removed from the cache.
async fn find(app: &AppHandle) -> Result<Option<Update>, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater.check().await.map_err(|e| e.to_string())?;
    remove_stale_downloads(app, update.as_ref().map(|update| update.version.as_str()));
    Ok(update)
}

/// Downloads a release and checks its signature.
async fn download(update: &Update) -> Result<Vec<u8>, String> {
    update
        .download(|_, _| {}, || {})
        .await
        .map_err(|e| e.to_string())
}

/// Whether a downloaded release is installed at startup. A deb or rpm package is installed through
/// `pkexec`, which asks for the root password, so it waits for `/update` instead of popping that
/// prompt up at login.
fn installs_at_startup() -> bool {
    !matches!(bundle_type(), Some(BundleType::Deb | BundleType::Rpm))
}

/// Installs a release and restarts into it. On Windows the plugin has already exited into the
/// installer, which starts the app again; on macOS and Linux it only replaces the app and returns.
fn install(app: &AppHandle, update: &Update, bytes: Vec<u8>) -> Result<(), String> {
    if let Some(path) = download_path(app, &update.version) {
        let _ = fs::remove_file(path);
    }
    update.install(bytes).map_err(|e| e.to_string())?;
    app.restart()
}

/// The version of the newer release waiting to be installed, if any.
#[tauri::command]
pub fn get_update(app: AppHandle) -> Option<String> {
    app.state::<AvailableUpdate>().0.lock().unwrap().clone()
}

/// Installs the newer release now (the `/update` command); the app restarts into it.
/// Returns false when there is none.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<bool, String> {
    let Some(update) = find(&app).await? else {
        return Ok(false);
    };
    let downloaded = download_path(&app, &update.version).and_then(|path| fs::read(path).ok());
    let bytes = match downloaded {
        Some(bytes) => bytes,
        None => download(&update).await?,
    };
    install(&app, &update, bytes)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn keeps_only_the_latest_download() {
        let found = names(&["update-0.8.0.bin", "update-0.8.1.bin", "update-0.8.2.bin"]);
        assert_eq!(
            stale_downloads(found, Some("0.8.2")),
            names(&["update-0.8.0.bin", "update-0.8.1.bin"])
        );
    }

    #[test]
    fn removes_every_download_when_there_is_no_newer_release() {
        let found = names(&["update-0.8.1.bin", "update-0.8.2.bin"]);
        assert_eq!(stale_downloads(found.clone(), None), found);
    }

    #[test]
    fn leaves_other_files_alone() {
        let found = names(&[
            "update-.bin",
            "update-0.8.1.bin.tmp",
            "old-update-0.8.1.bin",
            "update-0.8.1.exe",
            "Update-0.8.1.bin",
            "notes.md",
        ]);
        assert!(stale_downloads(found, None).is_empty());
    }

    #[test]
    fn removes_stale_downloads_from_the_directory() {
        let dir =
            std::env::temp_dir().join(format!("quicklly-test-update-cache-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        for name in ["update-0.8.1.bin", "update-0.8.2.bin", "other.txt"] {
            fs::write(dir.join(name), b"").unwrap();
        }
        remove_stale_downloads_in(&dir, Some("0.8.2"));
        let mut left: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        left.sort();
        assert_eq!(left, names(&["other.txt", "update-0.8.2.bin"]));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn saves_a_download_through_a_temporary_file() {
        let dir =
            std::env::temp_dir().join(format!("quicklly-test-update-save-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join(download_name("0.8.2"));
        save_download(&path, b"first").unwrap();
        save_download(&path, b"second").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"second");
        let left: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(left, names(&["update-0.8.2.bin"]));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_failed_save_leaves_no_temporary_file() {
        let dir = std::env::temp_dir().join(format!(
            "quicklly-test-update-save-fails-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        // A folder in the download's place makes the rename fail.
        let path = dir.join(download_name("0.8.2"));
        fs::create_dir_all(&path).unwrap();
        assert!(save_download(&path, b"bytes").is_err());
        assert!(path.is_dir());
        assert!(!dir.join("update-0.8.2.bin.tmp").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_missing_cache_directory_is_not_an_error() {
        let dir = std::env::temp_dir().join(format!(
            "quicklly-test-no-update-cache-{}",
            std::process::id()
        ));
        remove_stale_downloads_in(&dir, None);
        assert!(!dir.exists());
    }
}
