use std::fs;
use std::io;
use std::ops::Range;
use std::path::Path;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::notes::INBOX_FILE;

/// A notes file as listed in the reader.
#[derive(Debug, PartialEq, Serialize)]
pub struct NoteFile {
    pub name: String,
    pub notes: usize,
    /// How many of the notes are marked done.
    pub done: usize,
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
        let (notes, done) = count_notes(&fs::read_to_string(entry.path())?);
        files.push(NoteFile { name, notes, done });
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

/// Counts (all notes, done notes).
fn count_notes(content: &str) -> (usize, usize) {
    content
        .lines()
        .filter_map(parse_note_line)
        .fold((0, 0), |(notes, done), (is_done, _, _)| {
            (notes + 1, done + usize::from(is_done))
        })
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

/// A note as the reader last saw it, so a change is only made to that exact note.
#[derive(Debug, Deserialize)]
pub struct NoteRef {
    pub line: usize,
    pub time: String,
    pub done: bool,
    pub text: String,
}

/// Marks a note as done (`- [x] HH:MM text`) or not done (`- HH:MM text`).
pub fn set_done(dir: &Path, name: &str, note: &NoteRef, done: bool) -> io::Result<()> {
    edit_note(dir, name, note, |lines, span| {
        let first = note.text.lines().next().unwrap_or("");
        lines[span.start] = note_line(done, &note.time, first);
    })
}

fn note_line(done: bool, time: &str, text: &str) -> String {
    let checkbox = if done { "[x] " } else { "" };
    if text.is_empty() {
        format!("- {checkbox}{time}")
    } else {
        format!("- {checkbox}{time} {text}")
    }
}

/// Applies `change` to the lines of `note` in a notes file, leaving everything else as is.
/// Fails without writing if the file no longer has that note where the reader saw it.
fn edit_note(
    dir: &Path,
    name: &str,
    note: &NoteRef,
    change: impl FnOnce(&mut Vec<String>, Range<usize>),
) -> io::Result<()> {
    if !is_notes_file(name) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{name} is not a notes file"),
        ));
    }
    let path = dir.join(name);
    let content = fs::read_to_string(&path)?;
    let unchanged = parse(&content).iter().any(|item| {
        matches!(item, Item::Note { line, time, done, text }
            if *line == note.line && *time == note.time && *done == note.done && *text == note.text)
    });
    if !unchanged {
        return Err(io::Error::other(
            "the file has changed since it was shown, so it has been reloaded",
        ));
    }

    let newline = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    let span = note.line..note.line + note.text.lines().count().max(1);
    change(&mut lines, span);
    let mut updated = lines.join(newline);
    if content.ends_with('\n') && !lines.is_empty() {
        updated.push_str(newline);
    }
    // Write a copy and swap it in, so a failed write can't leave a half-written file.
    let tmp = path.with_extension("md.tmp");
    fs::write(&tmp, updated)?;
    fs::rename(&tmp, &path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("quicklly-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn note_ref(line: usize, time: &str, done: bool, text: &str) -> NoteRef {
        NoteRef {
            line,
            time: time.into(),
            done,
            text: text.into(),
        }
    }

    #[test]
    fn toggles_done_and_keeps_the_rest_of_the_file() {
        let dir = temp_dir("done");
        let path = dir.join("2026-09-26.md");
        fs::write(
            &path,
            "# 2026-09-26\r\n\r\n- 14:32 one\r\n  two\r\n- 15:00 three\r\n",
        )
        .unwrap();

        set_done(
            &dir,
            "2026-09-26.md",
            &note_ref(2, "14:32", false, "one\ntwo"),
            true,
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# 2026-09-26\r\n\r\n- [x] 14:32 one\r\n  two\r\n- 15:00 three\r\n"
        );

        set_done(
            &dir,
            "2026-09-26.md",
            &note_ref(2, "14:32", true, "one\ntwo"),
            false,
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# 2026-09-26\r\n\r\n- 14:32 one\r\n  two\r\n- 15:00 three\r\n"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn refuses_to_change_a_note_that_moved() {
        let dir = temp_dir("stale");
        let path = dir.join("2026-09-26.md");
        let content = "# 2026-09-26\n\n- 09:00 inserted meanwhile\n- 14:32 one\n";
        fs::write(&path, content).unwrap();

        assert!(set_done(
            &dir,
            "2026-09-26.md",
            &note_ref(2, "14:32", false, "one"),
            true
        )
        .is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), content);
        fs::remove_dir_all(&dir).unwrap();
    }

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
                .map(|f| (f.name, f.notes, f.done))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            names(list_files(&dir).unwrap()),
            [
                ("inbox.md".to_string(), 0, 0),
                ("2026-09-26.md".to_string(), 2, 1),
                ("2026-09-25.md".to_string(), 1, 0),
            ]
        );
        assert!(list_files(&dir.join("missing")).unwrap().is_empty());
        assert!(read_file(&dir, "todo.md").is_err());
        assert!(read_file(&dir, "../2026-09-26.md").is_err());
        fs::remove_dir_all(&dir).unwrap();
    }
}
