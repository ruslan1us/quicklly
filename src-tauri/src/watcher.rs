//! Watches the notes folder, so that open windows follow changes made outside the app: a note
//! saved through the command line or a `quicklly://` link (written by a separate process, see
//! `cli`) or a notes file edited in another editor. A change to a notes file sends the windows
//! `notes-changed`, as the app's own changes do; those are seen here too, which only makes the
//! windows reload once more.

use std::path::{Component, Path};
use std::sync::Mutex;
use std::time::Duration;

use notify_debouncer_mini::notify::{self, RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use tauri::{AppHandle, Manager};

use crate::{images, settings};

/// How long a file has to stay unchanged before the windows are told, so that a burst of
/// writes (a temporary file renamed over a notes file, several files archived) gives one event.
const DEBOUNCE: Duration = Duration::from_millis(300);

/// The watcher of the notes folder; dropping it stops watching.
#[derive(Default)]
pub struct Watcher(Mutex<Option<Debouncer<RecommendedWatcher>>>);

/// Starts watching the notes folder, at startup and again whenever another folder is chosen in
/// settings, in place of the one watched before. A folder that can't be watched is only logged:
/// the windows then follow the app's own changes, as before.
pub fn start(app: &AppHandle) {
    let state = app.state::<Watcher>();
    let mut watcher = state.0.lock().unwrap();
    // The old folder is no longer watched, even if the new one can't be.
    *watcher = None;
    let result = settings::notes_dir(app)
        .map_err(|e| e.to_string())
        .and_then(|dir| {
            let app = app.clone();
            watch(&dir, move || crate::notes_changed(&app))
                .map_err(|e| format!("{}: {e}", dir.display()))
        });
    match result {
        Ok(debouncer) => *watcher = Some(debouncer),
        Err(e) => eprintln!("Failed to watch the notes folder: {e}"),
    }
}

/// Watches `dir` and everything in it, and calls `changed` once a burst of changes to its notes
/// files is over. The folder is made if it isn't there yet (it would be with the first note), so
/// that it can be watched from the start.
fn watch(
    dir: &Path,
    changed: impl Fn() + Send + 'static,
) -> notify::Result<Debouncer<RecommendedWatcher>> {
    std::fs::create_dir_all(dir)?;
    let watched = dir.to_path_buf();
    let mut debouncer = new_debouncer(DEBOUNCE, move |result: DebounceEventResult| match result {
        Ok(events) => {
            if events
                .iter()
                .any(|event| is_notes_change(&watched, &event.path))
            {
                changed();
            }
        }
        Err(e) => eprintln!("Error watching the notes folder: {e}"),
    })?;
    debouncer.watcher().watch(dir, RecursiveMode::Recursive)?;
    Ok(debouncer)
}

/// Whether a change at `path` in the notes folder `dir` is one to a notes file: a Markdown file
/// anywhere in it, tag folders and the archive included, but not in the images folder or a
/// dot-folder (like Obsidian's `.obsidian` and `.trash`). The app's temporary `*.md.tmp` files
/// don't count; renaming one over a notes file does, as a change to that file.
fn is_notes_change(dir: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(dir) else {
        return false;
    };
    let mut names = Vec::new();
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return false;
        };
        names.push(name.to_string_lossy());
    }
    let markdown = relative.extension().is_some_and(|ext| ext == "md");
    let in_images = names
        .first()
        .is_some_and(|first| first.eq_ignore_ascii_case(images::IMAGES_DIR));
    markdown && !in_images && !names.iter().any(|name| name.starts_with('.'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::mpsc;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("quicklly-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// `base` with `names` joined to it, one path component each.
    fn path_in(base: &Path, names: &[&str]) -> PathBuf {
        names
            .iter()
            .fold(base.to_path_buf(), |path, name| path.join(name))
    }

    #[test]
    fn counts_changes_to_notes_files_only() {
        let root = std::env::temp_dir();
        let dir = root.join("Notes");
        let counts = |names: &[&str]| is_notes_change(&dir, &path_in(&dir, names));
        assert!(counts(&["inbox.md"]));
        assert!(counts(&["2026-10-14.md"]));
        assert!(counts(&["work", "quicklly.md"]));
        assert!(counts(&["archive", "work", "quicklly.md"]));
        // A tag folder may be called images below the top level.
        assert!(counts(&["work", "images", "x.md"]));
        assert!(!counts(&["inbox.md.tmp"]));
        assert!(!counts(&["work", "quicklly.md.tmp"]));
        assert!(!counts(&["images", "2026-10-14_09-00-00.png"]));
        assert!(!counts(&["Images", "notes.md"]));
        assert!(!counts(&[".obsidian", "workspace.json"]));
        assert!(!counts(&[".trash", "old.md"]));
        assert!(!counts(&["work"]));
        assert!(!counts(&["notes.txt"]));
        assert!(!is_notes_change(
            &dir,
            &path_in(&root, &["Other", "inbox.md"])
        ));
    }

    #[test]
    fn reports_a_notes_file_renamed_into_place_but_not_images_or_temporary_files() {
        let dir = temp_dir("watcher");
        let (tx, rx) = mpsc::channel();
        let watcher = watch(&dir, move || {
            let _ = tx.send(());
        })
        .unwrap();
        fs::create_dir_all(dir.join("images")).unwrap();
        fs::write(dir.join("images").join("a.png"), b"png").unwrap();
        fs::write(dir.join("inbox.md.tmp"), "- 09:00 note\n").unwrap();
        assert!(rx.recv_timeout(Duration::from_secs(1)).is_err());
        fs::rename(dir.join("inbox.md.tmp"), dir.join("inbox.md")).unwrap();
        assert!(rx.recv_timeout(Duration::from_secs(5)).is_ok());
        drop(watcher);
        fs::remove_dir_all(&dir).unwrap();
    }
}
