use std::fs;
use std::io;
use std::path::Path;

use chrono::NaiveDate;
use serde::Serialize;

use crate::notes::INBOX_FILE;

/// A notes file as listed in the reader.
#[derive(Debug, PartialEq, Serialize)]
pub struct NoteFile {
    pub name: String,
    pub notes: usize,
}

/// Lists the notes files in `dir`: `inbox.md` first, then daily files, newest first.
/// Other files in the folder are ignored. A missing folder just has no notes yet.
pub fn list_files(dir: &Path) -> io::Result<Vec<NoteFile>> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !(is_daily_file(&name) || name == INBOX_FILE) || !entry.file_type()?.is_file() {
            continue;
        }
        let notes = count_notes(&fs::read_to_string(entry.path())?);
        files.push(NoteFile { name, notes });
    }
    // Daily names are ISO dates, so reverse name order is newest first.
    files.sort_by(|a, b| {
        (b.name == INBOX_FILE)
            .cmp(&(a.name == INBOX_FILE))
            .then_with(|| b.name.cmp(&a.name))
    });
    Ok(files)
}

fn is_daily_file(name: &str) -> bool {
    name.strip_suffix(".md")
        .is_some_and(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").is_ok())
}

/// A note starts with `- HH:MM` or, once done, `- [x] HH:MM`.
fn is_note_line(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("- ") else {
        return false;
    };
    let rest = ["[ ] ", "[x] ", "[X] "]
        .iter()
        .find_map(|checkbox| rest.strip_prefix(checkbox))
        .unwrap_or(rest);
    let bytes = rest.as_bytes();
    bytes.len() >= 5
        && bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2] == b':'
        && bytes[3].is_ascii_digit()
        && bytes[4].is_ascii_digit()
        && bytes.get(5).is_none_or(|&b| b == b' ')
}

fn count_notes(content: &str) -> usize {
    content.lines().filter(|line| is_note_line(line)).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_note_lines() {
        assert!(is_note_line("- 14:32 text"));
        assert!(is_note_line("- [x] 14:32 text"));
        assert!(is_note_line("- 14:32"));
        assert!(!is_note_line("  second line of a note"));
        assert!(!is_note_line("# 2026-09-26"));
        assert!(!is_note_line("- a list item written by hand"));
        assert!(!is_note_line("- 14:321 not a time"));
    }

    #[test]
    fn lists_notes_files_inbox_first_then_newest() {
        let dir = std::env::temp_dir().join(format!("quicklly-test-list-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("2026-09-25.md"), "# 2026-09-25\n\n- 09:00 a\n").unwrap();
        fs::write(
            dir.join("2026-09-26.md"),
            "# 2026-09-26\n\n- 10:00 b\n  more\n- [x] 11:00 c\n",
        )
        .unwrap();
        fs::write(dir.join("inbox.md"), "# Inbox\n").unwrap();
        fs::write(dir.join("todo.md"), "- 12:00 not a notes file\n").unwrap();

        let names = |files: Vec<NoteFile>| {
            files
                .into_iter()
                .map(|f| (f.name, f.notes))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            names(list_files(&dir).unwrap()),
            [
                ("inbox.md".to_string(), 0),
                ("2026-09-26.md".to_string(), 2),
                ("2026-09-25.md".to_string(), 1),
            ]
        );
        assert!(list_files(&dir.join("missing")).unwrap().is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }
}
