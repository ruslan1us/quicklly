//! Watches the notes folder, so that open windows follow changes made outside the app: a note
//! saved through the command line or a `quicklly://` link (written by a separate process, see
//! `cli`) or a notes file edited in another editor. A change to a notes file sends the windows
//! `notes-changed`, as the app's own changes do; those are seen here too, which only makes the
//! windows reload once more.

use std::path::{Component, Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher as _};
use tauri::{AppHandle, Manager};

use crate::{images, settings};

/// How long the notes files have to stay unchanged before the windows are told, so that a burst of
/// writes (a temporary file renamed over a notes file, several files archived) gives one event.
const DEBOUNCE: Duration = Duration::from_millis(300);

/// The watcher of the notes folder; dropping it stops watching.
#[derive(Default)]
pub struct Watcher(Mutex<Option<RecommendedWatcher>>);

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
        Ok(watching) => *watcher = Some(watching),
        Err(e) => eprintln!("Failed to watch the notes folder: {e}"),
    }
}

/// Watches `dir` and everything in it, and calls `changed` once a burst of changes to its notes
/// files is over. The folder is made if it isn't there yet (it would be with the first note), so
/// that it can be watched from the start.
fn watch(dir: &Path, changed: impl Fn() + Send + 'static) -> notify::Result<RecommendedWatcher> {
    std::fs::create_dir_all(dir)?;
    // FSEvents (macOS) reports paths with symlinks resolved, `/private/var/...` for `/var/...`;
    // Windows and inotify report them under the folder as given. A change counts under either.
    // On Windows the resolved form is a `\\?\` path, which reported paths never start with.
    let mut folders = vec![dir.to_path_buf()];
    folders.extend(dir.canonicalize().ok());
    let (sender, events) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(sender)?;
    watcher.watch(dir, RecursiveMode::Recursive)?;
    std::thread::spawn(move || debounce(events, &folders, changed));
    Ok(watcher)
}

/// Calls `changed` once the notes files in `folders` have stayed unchanged for `DEBOUNCE` after a
/// change. Returns when `events` closes, which dropping the watcher does.
fn debounce(events: Receiver<notify::Result<Event>>, folders: &[PathBuf], changed: impl Fn()) {
    let mut deadline: Option<Instant> = None;
    loop {
        let received = match deadline {
            Some(at) => events.recv_timeout(at.saturating_duration_since(Instant::now())),
            None => events.recv().map_err(RecvTimeoutError::from),
        };
        match received {
            Ok(Ok(event)) => {
                if is_change(folders, &event) {
                    deadline = Some(Instant::now() + DEBOUNCE);
                }
            }
            Ok(Err(e)) => eprintln!("Error watching the notes folder: {e}"),
            Err(RecvTimeoutError::Timeout) => {
                deadline = None;
                changed();
            }
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

/// Whether `event` changes a notes file. An access doesn't: inotify reports every file opened and
/// closed, so the windows reloading on it would read the files and report that, without end.
fn is_change(folders: &[PathBuf], event: &Event) -> bool {
    !event.kind.is_access()
        && event
            .paths
            .iter()
            .any(|path| is_notes_change(folders, path))
}

/// Whether a change at `path` in the notes folder, given as its `folders` (the path as set and
/// as resolved), is one to a notes file: a Markdown file anywhere in it, tag folders and the
/// archive included, but not in the images folder or a dot-folder (like Obsidian's `.obsidian`
/// and `.trash`). The app's temporary `*.md.tmp` files don't count; renaming one over a notes
/// file does, as a change to that file.
fn is_notes_change(folders: &[PathBuf], path: &Path) -> bool {
    let Some(relative) = folders.iter().find_map(|dir| path.strip_prefix(dir).ok()) else {
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
    use notify::event::{AccessKind, AccessMode, CreateKind, DataChange, ModifyKind, RemoveKind};
    use notify::EventKind;
    use std::fs;

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
        let folders = [dir.clone()];
        let counts = |names: &[&str]| is_notes_change(&folders, &path_in(&dir, names));
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
            &folders,
            &path_in(&root, &["Other", "inbox.md"])
        ));
    }

    #[test]
    fn counts_changes_under_either_form_of_the_folder() {
        let root = std::env::temp_dir();
        let folders = [root.join("Linked notes"), root.join("Notes")];
        let counts = |path: PathBuf| is_notes_change(&folders, &path);
        assert!(counts(path_in(&folders[0], &["inbox.md"])));
        assert!(counts(path_in(&folders[1], &["work", "quicklly.md"])));
        assert!(!counts(path_in(&folders[1], &["images", "notes.md"])));
        assert!(!counts(path_in(&root, &["Other", "inbox.md"])));
    }

    /// An event of `kind` for `path`.
    fn event(kind: EventKind, path: &Path) -> Event {
        Event::new(kind).add_path(path.to_path_buf())
    }

    #[test]
    fn counts_changes_but_not_accesses() {
        let dir = std::env::temp_dir().join("Notes");
        let folders = [dir.clone()];
        let inbox = dir.join("inbox.md");
        let counts = |kind| is_change(&folders, &event(kind, &inbox));
        assert!(!counts(EventKind::Access(AccessKind::Open(
            AccessMode::Any
        ))));
        assert!(!counts(EventKind::Access(AccessKind::Close(
            AccessMode::Read
        ))));
        assert!(!counts(EventKind::Access(AccessKind::Close(
            AccessMode::Write
        ))));
        let modify = EventKind::Modify(ModifyKind::Data(DataChange::Any));
        assert!(counts(modify));
        assert!(counts(EventKind::Create(CreateKind::File)));
        assert!(counts(EventKind::Remove(RemoveKind::File)));
        // A change elsewhere in the folder doesn't count.
        assert!(!is_change(
            &folders,
            &event(modify, &path_in(&dir, &["images", "a.png"]))
        ));
        // A rename can report both names, and only one of them need be a notes file.
        let rename = Event::new(modify)
            .add_path(dir.join("inbox.md.tmp"))
            .add_path(inbox.clone());
        assert!(is_change(&folders, &rename));
    }

    #[test]
    fn reports_a_burst_of_changes_once_and_ends_with_its_events() {
        let dir = std::env::temp_dir().join("Notes");
        let inbox = dir.join("inbox.md");
        let (sender, events) = mpsc::channel();
        let (told, reports) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            debounce(events, &[dir], move || told.send(()).unwrap());
        });
        let open = EventKind::Access(AccessKind::Open(AccessMode::Any));
        sender.send(Ok(event(open, &inbox))).unwrap();
        assert!(reports.recv_timeout(DEBOUNCE * 2).is_err());
        let modify = EventKind::Modify(ModifyKind::Data(DataChange::Any));
        for _ in 0..3 {
            sender.send(Ok(event(modify, &inbox))).unwrap();
        }
        assert!(reports.recv_timeout(Duration::from_secs(5)).is_ok());
        assert!(reports.recv_timeout(DEBOUNCE * 2).is_err());
        drop(sender);
        thread.join().unwrap();
    }

    #[test]
    fn stops_when_the_watcher_is_dropped() {
        let dir = temp_dir("watcher-drop");
        let (tx, rx) = mpsc::channel::<()>();
        let watcher = watch(&dir, move || {
            let _ = tx.send(());
        })
        .unwrap();
        drop(watcher);
        // The thread owns the callback and with it `tx`.
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(5)),
            Err(RecvTimeoutError::Disconnected)
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    /// Not on macOS: FSEvents can report a file made just before the watch started.
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn does_not_report_a_notes_file_being_read() {
        let dir = temp_dir("watcher-read");
        fs::write(dir.join("inbox.md"), "- 09:00 note\n").unwrap();
        let (tx, rx) = mpsc::channel();
        let watcher = watch(&dir, move || {
            let _ = tx.send(());
        })
        .unwrap();
        fs::read_to_string(dir.join("inbox.md")).unwrap();
        assert!(rx.recv_timeout(Duration::from_secs(1)).is_err());
        drop(watcher);
        fs::remove_dir_all(&dir).unwrap();
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
