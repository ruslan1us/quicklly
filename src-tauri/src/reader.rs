use std::fs;
use std::io;
use std::ops::Range;
use std::path::Path;
use std::time::SystemTime;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::notes::{first_tag, format_note, note_lines, INBOX_FILE};

/// A notes file as listed in the reader.
#[derive(Debug, PartialEq, Serialize)]
pub struct NoteFile {
    pub name: String,
    pub notes: usize,
    /// How many of the notes are marked done.
    pub done: usize,
    /// When the file was last changed, in milliseconds since 1970.
    pub changed: u64,
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

/// Lists the Markdown files in `dir` by name: `inbox.md` first, then tag files and any other
/// `.md` files by name, then daily files, newest first. (The reader can instead sort them by
/// when they last changed.) A missing folder just has no notes yet.
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
        let changed = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|modified| modified.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map_or(0, |since| since.as_millis() as u64);
        let (notes, done) = count_notes(&fs::read_to_string(entry.path())?);
        files.push(NoteFile {
            name,
            notes,
            done,
            changed,
        });
    }
    let group = |name: &str| match name {
        INBOX_FILE => 0,
        _ if daily_date(name).is_none() => 1,
        _ => 2,
    };
    files.sort_by(|a, b| {
        group(&a.name).cmp(&group(&b.name)).then_with(|| {
            // Daily names are ISO dates, so reverse name order is newest first.
            if group(&a.name) == 2 {
                b.name.cmp(&a.name)
            } else {
                a.name.cmp(&b.name)
            }
        })
    });
    Ok(files)
}

/// The tags that have a file of their own: the names, without `.md`, of the notes files that
/// are neither the inbox nor daily files, sorted. Suggested while a `#tag` is typed.
pub fn list_tags(dir: &Path) -> io::Result<Vec<String>> {
    let mut tags: Vec<String> = list_files(dir)?
        .into_iter()
        .filter(|file| file.name != INBOX_FILE && daily_date(&file.name).is_none())
        .filter_map(|file| file.name.strip_suffix(".md").map(String::from))
        // Only names a note's `#tag` can lead to: `My Notes.md` is no tag file.
        .filter(|name| first_tag(&format!("#{name}")).as_deref() == Some(name.as_str()))
        .collect();
    tags.sort();
    Ok(tags)
}

/// A note found by [`search`].
#[derive(Debug, PartialEq, Serialize)]
pub struct Hit {
    pub file: String,
    /// First line of the note in the file (0-based), to open it in the reader.
    pub line: usize,
    /// The note's day: the daily file's date, or the `## YYYY-MM-DD` heading above it.
    pub day: String,
    pub time: String,
    pub done: bool,
    pub text: String,
}

/// Finds the notes in all notes files that contain `query`, ignoring case; newest first,
/// at most `limit` of them.
pub fn search(dir: &Path, query: &str, limit: usize) -> io::Result<Vec<Hit>> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let mut hits = Vec::new();
    for file in list_files(dir)? {
        // Notes in other files take their day from the `## YYYY-MM-DD` heading above them.
        let mut day = daily_date(&file.name).unwrap_or_default().to_string();
        for item in parse(&fs::read_to_string(dir.join(&file.name))?) {
            match item {
                Item::Day { date } => day = date,
                Item::Note {
                    line,
                    time,
                    done,
                    text,
                } if text.to_lowercase().contains(&query) => hits.push(Hit {
                    file: file.name.clone(),
                    line,
                    day: day.clone(),
                    time,
                    done,
                    text,
                }),
                Item::Note { .. } => {}
            }
        }
    }
    hits.sort_by(|a, b| (&b.day, &b.time).cmp(&(&a.day, &a.time)));
    hits.truncate(limit);
    Ok(hits)
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

/// Moves one notes file from `dir` to the Recycle Bin, so a file deleted by mistake can be
/// restored; only notes files can be deleted.
pub fn trash_file(dir: &Path, name: &str) -> io::Result<()> {
    if !is_notes_file(name) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{name} is not a notes file"),
        ));
    }
    trash::delete(dir.join(name)).map_err(io::Error::other)
}

/// Any Markdown file directly in the notes folder: a plain `name.md`, no paths.
fn is_notes_file(name: &str) -> bool {
    name.len() > ".md".len()
        && name.ends_with(".md")
        && Path::new(name).file_name().and_then(|n| n.to_str()) == Some(name)
        && !name.starts_with('.')
}

/// The date of a daily file name `YYYY-MM-DD.md`.
fn daily_date(name: &str) -> Option<&str> {
    name.strip_suffix(".md")
        .filter(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").is_ok())
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
            // Extra lines are indented by two spaces; blank lines belong to the note only
            // when another indented line follows them.
            loop {
                let mut next = i + 1;
                while lines.get(next).is_some_and(|l| l.trim().is_empty()) {
                    next += 1;
                }
                match lines.get(next).and_then(|l| l.strip_prefix("  ")) {
                    Some(more) if !more.trim().is_empty() => {
                        text.push_str(&"\n".repeat(next - i));
                        text.push_str(more);
                        i = next;
                    }
                    _ => break,
                }
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Removes a note, including its extra lines, from a notes file.
pub fn delete(dir: &Path, name: &str, note: &NoteRef) -> io::Result<()> {
    edit_note(dir, name, note, |lines, span| {
        lines.drain(span);
        remove_empty_days(lines);
    })
}

/// Drops the `## YYYY-MM-DD` headings that have nothing but blank lines under them any more,
/// and blank lines left at the end. Text written under a heading by hand keeps it.
fn remove_empty_days(lines: &mut Vec<String>) {
    let mut i = 0;
    while i < lines.len() {
        if !lines[i].starts_with("## ") {
            i += 1;
            continue;
        }
        let end = lines[i + 1..]
            .iter()
            .position(|line| line.starts_with("## "))
            .map_or(lines.len(), |next| i + 1 + next);
        if lines[i + 1..end].iter().all(|line| line.trim().is_empty()) {
            lines.drain(i..end);
        } else {
            i = end;
        }
    }
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
}

/// Replaces the text of a note, keeping its time and done mark. Extra lines are indented
/// under the first one, like new notes; a blank text is refused (Del deletes a note).
pub fn set_text(dir: &Path, name: &str, note: &NoteRef, text: &str) -> io::Result<()> {
    let new_lines = note_lines(text);
    let Some((first, rest)) = new_lines.split_first() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "a note can't be empty",
        ));
    };
    edit_note(dir, name, note, |lines, span| {
        let first = note_line(note.done, &note.time, first);
        let rest = format_note(&[&[""], rest].concat());
        // `rest` starts with the empty first line, so skip it.
        let rest = rest.lines().skip(1).map(String::from);
        lines.splice(span, std::iter::once(first).chain(rest));
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
    fn deletes_a_multi_line_note() {
        let dir = temp_dir("delete");
        let path = dir.join("inbox.md");
        fs::write(
            &path,
            "# Inbox\n\n## 2026-09-26\n\n- 14:32 one\n  two\n- 15:00 three\n",
        )
        .unwrap();

        delete(&dir, "inbox.md", &note_ref(4, "14:32", false, "one\ntwo")).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# Inbox\n\n## 2026-09-26\n\n- 15:00 three\n"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn deleting_the_last_note_of_a_day_drops_its_heading() {
        let dir = temp_dir("delete-day");
        let path = dir.join("idea.md");
        fs::write(
            &path,
            "# idea\n\n## 2026-09-26\n\n- 09:00 one\n\n## 2026-09-27\n\nby hand\n\n\
             ## 2026-09-28\n\n- 10:00 two\n",
        )
        .unwrap();

        delete(&dir, "idea.md", &note_ref(4, "09:00", false, "one")).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# idea\n\n## 2026-09-27\n\nby hand\n\n## 2026-09-28\n\n- 10:00 two\n"
        );
        delete(&dir, "idea.md", &note_ref(8, "10:00", false, "two")).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# idea\n\n## 2026-09-27\n\nby hand\n"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn edits_text_keeping_time_and_done_mark() {
        let dir = temp_dir("edit");
        let path = dir.join("2026-09-26.md");
        fs::write(&path, "# 2026-09-26\n\n- [x] 14:32 one\n- 15:00 two\n").unwrap();

        let note = note_ref(2, "14:32", true, "one");
        set_text(&dir, "2026-09-26.md", &note, " first \nsecond\n\n").unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# 2026-09-26\n\n- [x] 14:32 first\n  second\n- 15:00 two\n"
        );

        let note = note_ref(2, "14:32", true, "first\nsecond");
        assert!(set_text(&dir, "2026-09-26.md", &note, " \n ").is_err());
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
            "",
            "    - three",
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
                note(4, "14:32", false, "one\ntwo\n\n  - three"),
                note(8, "15:00", true, "done"),
                day("2026-09-27"),
                note(14, "09:00", false, "next"),
            ]
        );
    }

    #[test]
    fn edits_keep_blank_lines_and_indentation() {
        let dir = temp_dir("edit-blank");
        let path = dir.join("2026-09-26.md");
        fs::write(&path, "# 2026-09-26\n\n- 14:32 one\n- 15:00 two\n").unwrap();

        let old = note_ref(2, "14:32", false, "one");
        set_text(&dir, "2026-09-26.md", &old, "one\n\n  - sub\nend").unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(
            content,
            "# 2026-09-26\n\n- 14:32 one\n\n    - sub\n  end\n- 15:00 two\n"
        );
        assert_eq!(
            parse(&content)[0],
            note(2, "14:32", false, "one\n\n  - sub\nend")
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn searches_all_files_newest_first() {
        let dir = temp_dir("search");
        fs::write(
            dir.join("2026-09-25.md"),
            "# 2026-09-25\n\n- 09:00 Buy milk\n- 10:00 other\n",
        )
        .unwrap();
        fs::write(
            dir.join("inbox.md"),
            "# Inbox\n\n## 2026-09-26\n\n- 08:00 more milk\n  and bread\n- [x] 12:00 no\n",
        )
        .unwrap();

        let hits = search(&dir, " MILK ", 10).unwrap();
        let found: Vec<_> = hits
            .iter()
            .map(|h| (h.file.as_str(), h.line, h.day.as_str(), h.time.as_str()))
            .collect();
        assert_eq!(
            found,
            [
                ("inbox.md", 4, "2026-09-26", "08:00"),
                ("2026-09-25.md", 2, "2026-09-25", "09:00"),
            ]
        );
        assert_eq!(
            search(&dir, "bread", 10).unwrap()[0].text,
            "more milk\nand bread"
        );
        assert_eq!(search(&dir, "milk", 1).unwrap().len(), 1);
        assert!(search(&dir, "  ", 10).unwrap().is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn lists_inbox_then_tag_files_then_days_newest_first() {
        let dir = temp_dir("list");
        fs::write(dir.join("2026-09-25.md"), "# 2026-09-25\n\n- 09:00 a\n").unwrap();
        fs::write(
            dir.join("2026-09-26.md"),
            "# 2026-09-26\n\n- 10:00 b\n  more\n- [x] 11:00 c\n",
        )
        .unwrap();
        fs::write(dir.join("inbox.md"), "# Inbox\n").unwrap();
        fs::write(
            dir.join("todo.md"),
            "# todo\n\n## 2026-09-26\n\n- 12:00 d #todo\n",
        )
        .unwrap();
        fs::write(dir.join("ideas.md"), "# ideas\n").unwrap();
        fs::write(dir.join("notes.txt"), "- 12:00 not Markdown\n").unwrap();

        let base = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_800_000_000);
        for (i, name) in [
            "inbox.md",
            "ideas.md",
            "todo.md",
            "2026-09-26.md",
            "2026-09-25.md",
        ]
        .iter()
        .enumerate()
        {
            let file = fs::File::options()
                .write(true)
                .open(dir.join(name))
                .unwrap();
            file.set_modified(base + std::time::Duration::from_secs(i as u64))
                .unwrap();
        }

        let names = |files: Vec<NoteFile>| {
            files
                .into_iter()
                .map(|f| (f.name, f.notes, f.done, f.changed))
                .collect::<Vec<_>>()
        };
        let changed = |i: u64| (1_800_000_000 + i) * 1000;
        assert_eq!(
            names(list_files(&dir).unwrap()),
            [
                ("inbox.md".to_string(), 0, 0, changed(0)),
                ("ideas.md".to_string(), 0, 0, changed(1)),
                ("todo.md".to_string(), 1, 0, changed(2)),
                ("2026-09-26.md".to_string(), 2, 1, changed(3)),
                ("2026-09-25.md".to_string(), 1, 0, changed(4)),
            ]
        );
        assert!(list_files(&dir.join("missing")).unwrap().is_empty());
        assert!(read_file(&dir, "todo.md").is_ok());
        assert!(read_file(&dir, "notes.txt").is_err());
        assert!(read_file(&dir, "../2026-09-26.md").is_err());
        assert!(read_file(&dir, "..\\2026-09-26.md").is_err());
        assert_eq!(search(&dir, "#todo", 10).unwrap()[0].day, "2026-09-26");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn lists_tags_from_the_tag_files() {
        let dir = temp_dir("tags");
        for name in [
            "inbox.md",
            "2026-09-26.md",
            "todo.md",
            "ideas.md",
            "My Notes.md",
            "notes.txt",
        ] {
            fs::write(dir.join(name), "").unwrap();
        }
        fs::create_dir_all(dir.join("folder.md")).unwrap();

        assert_eq!(list_tags(&dir).unwrap(), ["ideas", "todo"]);
        assert!(list_tags(&dir.join("missing")).unwrap().is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn refuses_to_trash_anything_but_a_notes_file() {
        let dir = temp_dir("trash");
        let inner = dir.join("notes");
        fs::create_dir_all(&inner).unwrap();
        fs::write(dir.join("outside.md"), "# outside\n").unwrap();
        fs::write(inner.join("notes.txt"), "not Markdown\n").unwrap();

        for name in ["../outside.md", "..\\outside.md", "notes.txt", ".md", ""] {
            let err = trash_file(&inner, name).unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::InvalidInput, "{name}");
        }
        assert!(dir.join("outside.md").exists());
        assert!(inner.join("notes.txt").exists());
        fs::remove_dir_all(&dir).unwrap();
    }
}
