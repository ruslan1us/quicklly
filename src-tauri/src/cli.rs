//! Notes saved from outside the app: `quicklly.exe add <text>` from scripts, and
//! `quicklly://add?text=<text>` links, which Windows hands to the exe as its argument.
//!
//! The process started for it saves the note itself, with the settings read straight from the
//! settings file, prints where it went and quits, whether or not the app is running.

use std::io::Write;
use std::path::Path;

use tauri::Url;

use crate::{notes, settings};

/// Saves `text` as a note and prints `Saved to <file>` or `Error: …`; returns the exit code.
/// `identifier` is the app's, which names its data folder with the settings file.
pub fn save(identifier: &str, text: &str) -> i32 {
    attach_console();
    // Written without `println!`, which panics when nobody reads the output any more.
    match save_note(identifier, text) {
        Ok(name) => {
            let _ = writeln!(std::io::stdout(), "Saved to {name}");
            0
        }
        Err(e) => {
            let _ = writeln!(std::io::stderr(), "Error: {e}");
            1
        }
    }
}

/// Saves the note as the input window does and returns its file, relative to the notes folder.
fn save_note(identifier: &str, text: &str) -> Result<String, String> {
    let data_dir = dirs::data_dir().ok_or("Couldn't find the AppData folder")?;
    let (dir, mode) =
        settings::notes_settings_in(&data_dir.join(identifier).join(settings::STORE_FILE));
    let dir = match dir {
        Some(dir) => dir,
        None => settings::default_notes_dir_in(
            &dirs::document_dir().ok_or("Couldn't find the Documents folder")?,
        ),
    };
    let path = notes::append_note(&dir, mode, chrono::Local::now().naive_local(), text)
        .map_err(|e| format!("Failed to save note in {}: {e}", dir.display()))?
        .ok_or("The note is empty")?;
    Ok(file_name(&dir, &path))
}

/// The notes file `path` as the reader names it: relative to the notes folder `dir`, with `/`.
fn file_name(dir: &Path, path: &Path) -> String {
    path.strip_prefix(dir)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Lets the output reach the console the command was typed in. The release exe is a windowed
/// app, so it has no console of its own and only gets output handles when redirected.
#[cfg(windows)]
fn attach_console() {
    use windows::Win32::System::Console::{
        AttachConsole, GetStdHandle, ATTACH_PARENT_PROCESS, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE,
    };
    let missing = |handle| unsafe { GetStdHandle(handle) }.map_or(true, |h| h.is_invalid());
    if missing(STD_OUTPUT_HANDLE) || missing(STD_ERROR_HANDLE) {
        let _ = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
    }
}

#[cfg(not(windows))]
fn attach_console() {}

/// The note a command line asks to save: the words after `add`, joined with spaces, or the
/// `text` of a `quicklly://add?text=…` link. `args[0]` is the exe path. `None` for anything
/// else, which starts the app. A blank note is still a request, so that it fails with an error
/// instead of starting the app under a script that waits for it.
pub fn note_request(args: &[String]) -> Option<String> {
    match args.get(1..)? {
        [command, words @ ..] if command == "add" => Some(words.join(" ")),
        [link] => link_text(link),
        _ => None,
    }
}

/// The decoded `text` of a `quicklly://add?text=…` link (`+` is a space, as in a form), empty
/// without one.
fn link_text(link: &str) -> Option<String> {
    let url = Url::parse(link).ok()?;
    let is_add = url.scheme() == "quicklly"
        && url
            .host_str()
            .is_some_and(|host| host.eq_ignore_ascii_case("add"))
        && matches!(url.path(), "" | "/");
    if !is_add {
        return None;
    }
    let text = url.query_pairs().find(|(key, _)| key == "text");
    Some(text.map(|(_, text)| text.into_owned()).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(args: &[&str]) -> Option<String> {
        let args: Vec<String> = ["quicklly.exe"]
            .iter()
            .chain(args)
            .map(|arg| arg.to_string())
            .collect();
        note_request(&args)
    }

    #[test]
    fn file_names_are_relative_to_the_notes_folder() {
        let dir = Path::new(r"C:\Notes");
        assert_eq!(file_name(dir, &dir.join("todo.md")), "todo.md");
        assert_eq!(
            file_name(dir, &dir.join("work/quicklly.md")),
            "work/quicklly.md"
        );
    }

    #[test]
    fn add_takes_the_text() {
        assert_eq!(
            request(&["add", "buy milk #todo"]).unwrap(),
            "buy milk #todo"
        );
    }

    #[test]
    fn add_joins_several_words() {
        assert_eq!(
            request(&["add", "buy", "milk", "#todo"]).unwrap(),
            "buy milk #todo"
        );
    }

    #[test]
    fn add_without_text_is_a_blank_note() {
        assert_eq!(request(&["add"]).unwrap(), "");
        assert_eq!(request(&["add", "  ", ""]).unwrap(), "   ");
    }

    #[test]
    fn no_or_unknown_arguments_are_nothing() {
        assert_eq!(note_request(&[]), None);
        assert_eq!(request(&[]), None);
        assert_eq!(request(&["remove", "x"]), None);
        assert_eq!(request(&["--flag"]), None);
        assert_eq!(request(&["buy milk"]), None);
    }

    #[test]
    fn link_is_decoded() {
        assert_eq!(
            request(&["quicklly://add?text=buy%20milk%20%23todo"]).unwrap(),
            "buy milk #todo"
        );
        assert_eq!(
            request(&["quicklly://add?text=buy+milk+%23todo"]).unwrap(),
            "buy milk #todo"
        );
        assert_eq!(
            request(&["quicklly://add?text=%D0%BF%D1%80%D0%B8%D0%B2%D0%B5%D1%82"]).unwrap(),
            "привет"
        );
        assert_eq!(request(&["quicklly://add?text=a%2Bb"]).unwrap(), "a+b");
    }

    #[test]
    fn link_tolerates_slash_case_and_other_params() {
        assert_eq!(request(&["quicklly://add/?text=x"]).unwrap(), "x");
        assert_eq!(request(&["QUICKLLY://Add?text=x"]).unwrap(), "x");
        assert_eq!(request(&["quicklly://add?from=ahk&text=x"]).unwrap(), "x");
    }

    #[test]
    fn link_without_text_is_a_blank_note() {
        assert_eq!(request(&["quicklly://add"]).unwrap(), "");
        assert_eq!(request(&["quicklly://add?note=x"]).unwrap(), "");
        assert_eq!(request(&["quicklly://add?text="]).unwrap(), "");
        assert_eq!(request(&["quicklly://add?text=+%20"]).unwrap(), "  ");
    }

    #[test]
    fn other_links_are_nothing() {
        assert_eq!(request(&["quicklly://open?text=x"]), None);
        assert_eq!(request(&["quicklly://add/more?text=x"]), None);
        assert_eq!(request(&["https://add?text=x"]), None);
        assert_eq!(request(&["quicklly:add?text=x"]), None);
        assert_eq!(request(&["quicklly://add?text=x", "y"]), None);
        assert_eq!(request(&["%%%://"]), None);
    }
}
