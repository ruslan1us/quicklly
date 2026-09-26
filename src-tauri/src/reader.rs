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

/// One entry of a notes file, as shown in the reader.
#[derive(Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Item {
    /// A `## YYYY-MM-DD` day heading, as in the inbox.
    Day { date: String },
    /// A note: its first line in the file (0-based), time, done mark and text,
    /// with any extra lines joined by new lines.
    Note {
        line: usize,
        time: String,
        done: bool,
        text: String,
    },
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
        if !is_notes_file(&name) || !entry.file_type()?.is_file() {
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

/// Reads and parses one notes file from `dir`; only notes files can be read.
pub fn read_file(dir: &Path, name: &str) -> io::Result<Vec<Item>> {
    if !is_notes_file(name) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{name} is not a notes file"),
        ));
    }
    Ok(parse(&fs::read_to_string(dir.join(name))?))
}

/// `inbox.md` or a daily `YYYY-MM-DD.md`.
fn is_notes_file(name: &str) -> bool {
    name == INBOX_FILE
        || name
            .strip_suffix(".md")
            .is_some_and(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").is_ok())
}

/// Splits `- HH:MM text` or `- [x] HH:MM text` into (done, time, text).
fn parse_note_line(line: &str) -> Option<(bool, &str, &str)> {
    let rest = line.strip_prefix("- ")?;
    let (done, rest) = match rest.get(..4) {
        Some("[x] " | "[X] ") => (true, &rest[4..]),
        Some("[ ] ") => (false, &rest[4..]),
        _ => (false, rest),
    };
    let bytes = rest.as_bytes();
    let is_time = bytes.len() >= 5
        && bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2] == b':'
        && bytes[3].is_ascii_digit()
        && bytes[4].is_ascii_digit();
    if !is_time {
        return None;
    }
    let (time, after) = rest.split_at(5);
    match after.strip_prefix(' ') {
        Some(text) => Some((done, time, text)),
        None if after.is_empty() => Some((done, time, "")),
        None => None,
    }
}

fn count_notes(content: &str) -> usize {
    content
        .lines()
        .filter(|line| parse_note_line(line).is_some())
        .count()
}

/// Parses a notes file into day headings and notes; everything else is skipped.
/// A note's extra lines are the non-empty lines indented by two spaces right after it.
pub fn parse(content: &str) -> Vec<Item> {
    let lines: Vec<&str> = content.lines().collect();
    let mut items = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if let Some(date) = line.strip_prefix("## ") {
            items.push(Item::Day {
                date: date.trim().to_string(),
            });
        } else if let Some((done, time, first)) = parse_note_line(line) {
            let start = i;
            let mut text = first.to_string();
            while let Some(more) = lines.get(i + 1).and_then(|l| l.strip_prefix("  ")) {
                if more.trim().is_empty() {
                    break;
                }
                text.push('\n');
                text.push_str(more);
                i += 1;
            }
            items.push(Item::Note {
                line: start,
                time: time.to_string(),
                done,
                text,
            });
        }
        i += 1;
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(line: usize, time: &str, done: bool, text: &str) -> Item {
        Item::Note {
            line,
            time: time.into(),
            done,
            text: text.into(),
        }
    }

    fn day(date: &str) -> Item {
        Item::Day { date: date.into() }
    }

    #[test]
    fn recognises_note_lines() {
        assert_eq!(
            parse_note_line("- 14:32 text"),
            Some((false, "14:32", "text"))
        );
        assert_eq!(
            parse_note_line("- [x] 14:32 text"),
            Some((true, "14:32", "text"))
        );
        assert_eq!(parse_note_line("- 14:32"), Some((false, "14:32", "")));
        assert_eq!(parse_note_line("  second line of a note"), None);
        assert_eq!(parse_note_line("# 2026-09-26"), None);
        assert_eq!(parse_note_line("- a list item written by hand"), None);
        assert_eq!(parse_note_line("- 14:321 not a time"), None);
    }

    #[test]
    fn parses_days_and_multi_line_notes() {
        let content = [
            "# Inbox",
            "",
            "## 2026-09-26",
            "",
            "- 14:32 one",
            "  two",
            "  three",
            "- [x] 15:00 done",
            "",
            "some text by hand",
            "",
            "## 2026-09-27",
            "",
            "- 09:00 next",
        ]
        .join("\n");
        assert_eq!(
            parse(&content),
            [
                day("2026-09-26"),
                note(4, "14:32", false, "one\ntwo\nthree"),
                note(7, "15:00", true, "done"),
                day("2026-09-27"),
                note(13, "09:00", false, "next"),
            ]
        );
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
        assert!(read_file(&dir, "todo.md").is_err());
        assert!(read_file(&dir, "../2026-09-26.md").is_err());
        fs::remove_dir_all(&dir).unwrap();
    }
}
