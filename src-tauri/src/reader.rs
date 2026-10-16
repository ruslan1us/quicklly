use std::cmp::Ordering;
use std::fs;
use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::archive::ARCHIVE_DIR;
use crate::images::{self, IMAGES_DIR};
use crate::notes::{first_tag, format_note, note_lines, INBOX_FILE};
use crate::settings::DateFormat;

/// A notes file as listed in the reader.
#[derive(Debug, PartialEq, Serialize)]
pub struct NoteFile {
    /// The path from the notes folder, with `/` between folders: `work/quicklly.md`.
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

/// Lists the Markdown files in `dir` and its folders (but not the archive, the images or
/// dot-folders) by name, as a tree: in each folder `inbox.md` first, then tag files and any other
/// `.md` files by name, then daily files, newest first, and then its folders by name. (The
/// reader can instead sort them by when they last changed.) A missing folder just has no notes
/// yet; a file that can't be read is listed with no notes.
pub fn list_files(dir: &Path) -> io::Result<Vec<NoteFile>> {
    let mut files = Vec::new();
    add_files(dir, "", &mut files)?;
    files.sort_by(|a, b| tree_order(&a.name, &b.name));
    Ok(files)
}

/// Adds the notes files in the folder `prefix` (`""` or `work/`) of `dir` to `files`, and those
/// of the folders in it.
fn add_files(dir: &Path, prefix: &str, files: &mut Vec<NoteFile>) -> io::Result<()> {
    let entries = match fs::read_dir(path_of(dir, prefix)) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    for entry in entries {
        let entry = entry?;
        let name = format!("{prefix}{}", entry.file_name().to_string_lossy());
        let kind = entry.file_type()?;
        // A folder notes files can be in.
        if kind.is_dir() && is_notes_file(&format!("{name}/x.md")) {
            add_files(dir, &format!("{name}/"), files)?;
        }
        if !kind.is_file() || !is_notes_file(&name) {
            continue;
        }
        let changed = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|modified| modified.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map_or(0, |since| since.as_millis() as u64);
        // A file that can't be read (not UTF-8, say) is still listed, with no notes, so the
        // reader shows it and says why when it is opened.
        let (notes, done) =
            fs::read_to_string(entry.path()).map_or((0, 0), |content| count_notes(&content));
        files.push(NoteFile {
            name,
            notes,
            done,
            changed,
        });
    }
    Ok(())
}

/// The order of two names in the file list (see [`list_files`]).
fn tree_order(a: &str, b: &str) -> Ordering {
    let mut a_parts = a.split('/').peekable();
    let mut b_parts = b.split('/').peekable();
    loop {
        let (a_part, b_part) = (a_parts.next().unwrap_or(""), b_parts.next().unwrap_or(""));
        match (a_parts.peek().is_some(), b_parts.peek().is_some()) {
            // Both in a folder: the same one goes on to what is in it.
            (true, true) if a_part == b_part => continue,
            (true, true) => return a_part.cmp(b_part),
            // A file comes before the folders next to it.
            (false, true) => return Ordering::Less,
            (true, false) => return Ordering::Greater,
            (false, false) => return file_order(a_part, b_part),
        }
    }
}

/// The order of two files in the same folder: the inbox, other files by name, days newest first.
fn file_order(a: &str, b: &str) -> Ordering {
    let group = |name: &str| match name {
        INBOX_FILE => 0,
        _ if daily_date(name).is_none() => 1,
        _ => 2,
    };
    group(a).cmp(&group(b)).then_with(|| {
        // Daily names are ISO dates, so reverse name order is newest first.
        if group(a) == 2 {
            b.cmp(a)
        } else {
            a.cmp(b)
        }
    })
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
    /// The note's day: the daily file's date, or the `## YYYY-MM-DD` heading above it; none
    /// under any other heading or no heading.
    pub day: Option<String>,
    pub time: String,
    pub done: bool,
    pub text: String,
}

/// Finds the notes in all notes files that contain `query`, ignoring case, in their text as the
/// windows show it (see [`shown_text`]), with dates in `format`; newest first, at most `limit`
/// of them. Files that can't be read are left out.
pub fn search(dir: &Path, query: &str, format: DateFormat, limit: usize) -> io::Result<Vec<Hit>> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let mut hits = Vec::new();
    for file in list_files(dir)? {
        // Notes in other files take their day from the `## YYYY-MM-DD` heading above them; a
        // heading that is not a date, like `## Ideas`, gives the notes under it none.
        let mut day = daily_date(&file.name).map(String::from);
        let Ok(content) = fs::read_to_string(path_of(dir, &file.name)) else {
            continue;
        };
        for item in parse(&content) {
            match item {
                Item::Day { date } => day = iso_date(&date).map(|_| date),
                Item::Note {
                    line,
                    time,
                    done,
                    text,
                } if shown_text(&text, format).to_lowercase().contains(&query) => hits.push(Hit {
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
    // Notes with a day come first, newest first; those without one follow in the order of the
    // files and of the notes in them (a stable sort).
    hits.sort_by(|a, b| match (&a.day, &b.day) {
        (Some(a_day), Some(b_day)) => (b_day, &b.time).cmp(&(a_day, &a.time)),
        _ => a.day.is_none().cmp(&b.day.is_none()),
    });
    hits.truncate(limit);
    Ok(hits)
}

/// A note's text as the windows show it: without the day it was done, its images as their
/// labels `[Image #N]` and its dates (`📅` or `✅`) in `format`.
fn shown_text(text: &str, format: DateFormat) -> String {
    let (first, rest) = text.split_at(text.find('\n').unwrap_or(text.len()));
    let text = images::labels(&format!("{}{rest}", split_done_date(first).0));
    let mut shown = String::with_capacity(text.len());
    let mut from = 0;
    for (at, mark) in text.match_indices(['📅', '✅']) {
        let start = at + mark.len() + 1;
        let Some(date) = text[at + mark.len()..]
            .strip_prefix(' ')
            .and_then(stored_date)
        else {
            continue;
        };
        shown.push_str(&text[from..start]);
        shown.push_str(&format.format(date));
        from = start + 10;
    }
    shown.push_str(&text[from..]);
    shown
}

/// A note that is due, or due ahead, found by [`due`].
#[derive(Debug, Serialize)]
pub struct Due {
    pub file: String,
    pub note: NoteRef,
    /// The note's date, `YYYY-MM-DD`.
    pub date: String,
    /// How many days ago the date was; 0 when it is today, less than 0 when it is ahead.
    pub overdue: i64,
}

/// Finds the notes in all notes files that are not done and have a date (`📅 YYYY-MM-DD`, the
/// first one if there are several) of `today` or earlier, or with `ahead` after `today`: the
/// oldest date first, notes of the same date in the order of the files and of the notes in them.
/// Files that can't be read are left out.
pub fn due(dir: &Path, today: NaiveDate, ahead: bool) -> io::Result<Vec<Due>> {
    let mut found = Vec::new();
    for file in list_files(dir)? {
        let Ok(content) = fs::read_to_string(path_of(dir, &file.name)) else {
            continue;
        };
        for item in parse(&content) {
            let Item::Note {
                line,
                time,
                done: false,
                text,
            } = item
            else {
                continue;
            };
            let Some(date) = first_date(&text).filter(|date| (*date > today) == ahead) else {
                continue;
            };
            found.push(Due {
                file: file.name.clone(),
                note: NoteRef {
                    line,
                    time,
                    done: false,
                    text,
                },
                date: date.format("%Y-%m-%d").to_string(),
                overdue: (today - date).num_days(),
            });
        }
    }
    // A stable sort, so notes of the same date keep their order.
    found.sort_by(|a, b| a.date.cmp(&b.date));
    Ok(found)
}

/// A done note found by [`done_since`].
#[derive(Debug, PartialEq, Serialize)]
pub struct Done {
    /// The notes file it is in; `archive/…` once archived.
    pub file: String,
    /// Its text, without the day it was done.
    pub text: String,
    /// The day it was done, `YYYY-MM-DD`.
    pub date: String,
}

/// Finds the notes in all notes files and in the archive that were done (`✅ YYYY-MM-DD`) on
/// `from` or later: the latest day first, notes of the same day in the order of the files (the
/// archive last) and of the notes in them. Notes done without a date, and files that can't be
/// read, are left out.
pub fn done_since(dir: &Path, from: NaiveDate) -> io::Result<Vec<Done>> {
    let mut found = Vec::new();
    for (folder, prefix) in [(dir.to_path_buf(), ""), (dir.join(ARCHIVE_DIR), "archive/")] {
        for file in list_files(&folder)? {
            let Ok(content) = fs::read_to_string(path_of(&folder, &file.name)) else {
                continue;
            };
            for item in parse(&content) {
                let Item::Note {
                    done: true, text, ..
                } = item
                else {
                    continue;
                };
                let (first, rest) = text.split_at(text.find('\n').unwrap_or(text.len()));
                let (first, Some(date)) = split_done_date(first) else {
                    continue;
                };
                if date < from {
                    continue;
                }
                found.push(Done {
                    file: format!("{prefix}{}", file.name),
                    text: format!("{first}{rest}"),
                    date: date.format("%Y-%m-%d").to_string(),
                });
            }
        }
    }
    // A stable sort, so notes of the same day keep their order.
    found.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(found)
}

/// The first real date stored in a note's text as `📅 YYYY-MM-DD`.
fn first_date(text: &str) -> Option<NaiveDate> {
    first_date_at(text).map(|(_, date)| date)
}

/// [`first_date`] with where its `YYYY-MM-DD` starts in `text`.
fn first_date_at(text: &str) -> Option<(usize, NaiveDate)> {
    text.match_indices("📅 ").find_map(|(at, mark)| {
        let start = at + mark.len();
        stored_date(&text[start..]).map(|date| (start, date))
    })
}

/// The real date `YYYY-MM-DD` that `text` starts with, as a date is stored after its mark.
fn stored_date(text: &str) -> Option<NaiveDate> {
    // Like the frontend, a longer number is no date.
    if text.get(10..)?.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    iso_date(&text[..10])
}

/// A real date written exactly as `YYYY-MM-DD`.
fn iso_date(date: &str) -> Option<NaiveDate> {
    let digits = date.len() == 10
        && date.bytes().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                b == b'-'
            } else {
                b.is_ascii_digit()
            }
        });
    digits
        .then(|| NaiveDate::parse_from_str(date, "%Y-%m-%d").ok())
        .flatten()
}

/// Marks the day a note was done, at the end of its first line, like Obsidian Tasks.
const DONE_MARK: &str = "✅ ";

/// Splits a note's first line into its text and the date it was done, from a trailing
/// `✅ YYYY-MM-DD`; the line as it is and `None` when it has none.
pub(crate) fn split_done_date(line: &str) -> (&str, Option<NaiveDate>) {
    let line = line.trim_end();
    let Some((before, date)) = line.rsplit_once(DONE_MARK) else {
        return (line, None);
    };
    // The mark follows the text after a space, or is all there is.
    let before = match before.strip_suffix(' ') {
        Some(before) => before,
        None if before.is_empty() => before,
        None => return (line, None),
    };
    match iso_date(date) {
        Some(date) => (before, Some(date)),
        None => (line, None),
    }
}

/// Reads and parses one notes file from `dir`; only notes files can be read.
pub fn read_file(dir: &Path, name: &str) -> io::Result<Vec<Item>> {
    Ok(parse(&fs::read_to_string(notes_file(dir, name)?)?))
}

/// Moves one notes file from `dir` to the Recycle Bin, so a file deleted by mistake can be
/// restored; only notes files can be deleted. The folders it was in go too once empty.
pub fn trash_file(dir: &Path, name: &str) -> io::Result<()> {
    let path = notes_file(dir, name)?;
    trash::delete(&path).map_err(io::Error::other)?;
    for folder in path.ancestors().skip(1).take(name.matches('/').count()) {
        // Fails, and so stops, at a folder with something still in it.
        if fs::remove_dir(folder).is_err() {
            break;
        }
    }
    Ok(())
}

/// The file of the notes file `name` (a path from `dir`, like `work/quicklly.md`); an error for
/// anything that is no notes file, so nothing outside the notes folder is ever touched.
pub(crate) fn notes_file(dir: &Path, name: &str) -> io::Result<PathBuf> {
    if !is_notes_file(name) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{name} is not a notes file"),
        ));
    }
    Ok(path_of(dir, name))
}

/// The file at `name`, a path from `dir` with `/` between folders, with Windows separators.
pub(crate) fn path_of(dir: &Path, name: &str) -> PathBuf {
    name.split('/')
        .filter(|part| !part.is_empty())
        .fold(dir.to_path_buf(), |path, part| path.join(part))
}

/// Reads the file at `name` (see [`path_of`]); an error names the file, so it can be found
/// among all the others.
pub(crate) fn read_named(dir: &Path, name: &str) -> io::Result<String> {
    fs::read_to_string(path_of(dir, name))
        .map_err(|e| io::Error::new(e.kind(), format!("couldn't read {name}: {e}")))
}

/// Any Markdown file in the notes folder or a folder in it: `name.md` or `work/quicklly.md`,
/// with `/` between folders. Nothing that could lead outside the notes folder (`..`, absolute
/// paths, drives, `\`), nor anything in the archive, the images or a dot-folder.
fn is_notes_file(name: &str) -> bool {
    let parts: Vec<&str> = name.split('/').collect();
    let plain = parts.iter().all(|part| {
        !part.is_empty()
            && !part.starts_with('.')
            && !part.contains(['\\', ':'])
            && Path::new(part).file_name().and_then(|n| n.to_str()) == Some(*part)
    });
    let reserved = parts.len() > 1
        && [ARCHIVE_DIR, IMAGES_DIR]
            .iter()
            .any(|folder| parts[0].eq_ignore_ascii_case(folder));
    name.ends_with(".md") && plain && !reserved
}

/// The date of a daily file name `YYYY-MM-DD.md`, in any folder.
fn daily_date(name: &str) -> Option<&str> {
    name.rsplit('/')
        .next()?
        .strip_suffix(".md")
        .filter(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").is_ok())
}

/// The checkbox of a note line: `[x] `, `[X] `, `[ ] ` or none (`""`).
fn checkbox(line: &str) -> &str {
    line.strip_prefix("- ")
        .and_then(|rest| rest.get(..4))
        .filter(|mark| matches!(*mark, "[x] " | "[X] " | "[ ] "))
        .unwrap_or("")
}

/// Splits `- HH:MM text`, `- [ ] HH:MM text` or `- [x] HH:MM text` into (done, time, text).
fn parse_note_line(line: &str) -> Option<(bool, &str, &str)> {
    let mark = checkbox(line);
    let rest = line.strip_prefix("- ")?.get(mark.len()..)?;
    let done = matches!(mark, "[x] " | "[X] ");
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

/// A notes file before and after a change made in the reader, so the reader can undo it.
#[derive(Debug, Serialize, Deserialize)]
pub struct Change {
    pub before: String,
    pub after: String,
    /// The images the change moved to the Recycle Bin, to bring back with it (see
    /// [`images::trash_unused`]).
    pub images: Vec<PathBuf>,
}

/// Marks a note as done on `today` (`- [x] HH:MM text ✅ YYYY-MM-DD`) or not done
/// (`- [ ] HH:MM text`, without the done date). A note done and then not done again is an open
/// task, as in Obsidian: the plain `- HH:MM` it may have had is not remembered.
pub fn set_done(
    dir: &Path,
    name: &str,
    note: &NoteRef,
    done: bool,
    today: NaiveDate,
) -> io::Result<Change> {
    edit_note(dir, name, note, |lines, span| {
        let (first, _) = split_done_date(note.text.lines().next().unwrap_or(""));
        let first = match (done, first.is_empty()) {
            (false, _) => first.to_string(),
            (true, true) => format!("{DONE_MARK}{}", today.format("%Y-%m-%d")),
            (true, false) => format!("{first} {DONE_MARK}{}", today.format("%Y-%m-%d")),
        };
        let mark = if done { "[x] " } else { "[ ] " };
        lines[span.start] = note_line(mark, &note.time, &first);
    })
}

/// Moves a note's date (the one [`due`] goes by) to `date`, e.g. to be reminded again tomorrow.
pub fn snooze(dir: &Path, name: &str, note: &NoteRef, date: NaiveDate) -> io::Result<Change> {
    edit_note(dir, name, note, |lines, span| {
        move_date(&mut lines[span], date)
    })
}

/// Replaces the first date in a note's lines with `date`, leaving the rest as it is.
fn move_date(lines: &mut [String], date: NaiveDate) {
    for line in lines {
        if let Some((at, _)) = first_date_at(line) {
            line.replace_range(at..at + 10, &date.format("%Y-%m-%d").to_string());
            return;
        }
    }
}

/// Removes a note, including its extra lines, from a notes file.
pub fn delete(dir: &Path, name: &str, note: &NoteRef) -> io::Result<Change> {
    edit_note(dir, name, note, |lines, span| {
        lines.drain(span);
        remove_empty_days(lines);
    })
}

/// Drops the `## YYYY-MM-DD` headings that have nothing but blank lines under them any more,
/// and blank lines left at the end. Text written under a heading by hand keeps it, and other
/// headings always stay.
pub(crate) fn remove_empty_days(lines: &mut Vec<String>) {
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
        let day = iso_date(lines[i][3..].trim_end()).is_some();
        if day && lines[i + 1..end].iter().all(|line| line.trim().is_empty()) {
            lines.drain(i..end);
        } else {
            i = end;
        }
    }
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
}

/// Replaces the text of a note, keeping its time and checkbox. Extra lines are indented
/// under the first one, like new notes; a blank text is refused (Del deletes a note).
pub fn set_text(dir: &Path, name: &str, note: &NoteRef, text: &str) -> io::Result<Change> {
    let new_lines = note_lines(text);
    let Some((first, rest)) = new_lines.split_first() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "a note can't be empty",
        ));
    };
    // The text is edited without the day it was done (the reader doesn't show it), so that
    // stays as it was.
    let old_first = note.text.lines().next().unwrap_or("");
    let first = match (split_done_date(old_first).1, split_done_date(first).1) {
        (Some(done_on), None) if note.done => {
            let date = format!("{DONE_MARK}{}", done_on.format("%Y-%m-%d"));
            if first.is_empty() {
                date
            } else {
                format!("{first} {date}")
            }
        }
        _ => first.to_string(),
    };
    // New images are saved as `images/…`, so a file in a folder links them from there.
    let first = images::link_from(&first, name);
    let rest: Vec<String> = rest
        .iter()
        .map(|line| images::link_from(line, name))
        .collect();
    let rest: Vec<&str> = rest.iter().map(String::as_str).collect();
    edit_note(dir, name, note, |lines, span| {
        let first = note_line(checkbox(&lines[span.start]), &note.time, &first);
        let rest = format_note(&[&[""], rest.as_slice()].concat());
        // `rest` starts with the empty first line, so skip it.
        let rest = rest.lines().skip(1).map(String::from);
        lines.splice(span, std::iter::once(first).chain(rest));
    })
}

/// A note's first line, with `checkbox` (`[x] `, `[ ] ` or `""`) before the time.
fn note_line(checkbox: &str, time: &str, text: &str) -> String {
    if text.is_empty() {
        format!("- {checkbox}{time}")
    } else {
        format!("- {checkbox}{time} {text}")
    }
}

/// Applies `change` to the lines of `note` in a notes file, leaving everything else as is, and
/// returns the file before and after it. Fails without writing if the file no longer has that
/// note where the reader saw it.
fn edit_note(
    dir: &Path,
    name: &str,
    note: &NoteRef,
    change: impl FnOnce(&mut Vec<String>, Range<usize>),
) -> io::Result<Change> {
    let path = notes_file(dir, name)?;
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

    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    change(&mut lines, note_span(note.line, &note.text));
    let after = write_lines(&path, &content, &lines)?;
    Ok(Change {
        before: content,
        after,
        images: Vec::new(),
    })
}

/// What to write back to undo `change` to a file that now has `current`: what it had before,
/// if nothing else has changed it since the change; `None` otherwise.
fn undone<'a>(change: &'a Change, current: &str) -> Option<&'a str> {
    (current == change.after).then_some(change.before.as_str())
}

/// Undoes a change made in the reader to the notes file `name`, putting back exactly what the
/// file had before it. Fails without writing if the file has changed since, e.g. a note added
/// from the input window or moved to the archive.
pub fn undo(dir: &Path, name: &str, change: &Change) -> io::Result<()> {
    let path = notes_file(dir, name)?;
    let current = fs::read_to_string(&path)?;
    let before = undone(change, &current)
        .ok_or_else(|| io::Error::other(format!("{name} has changed since")))?;
    write_file(&path, before)
}

/// The lines of the file a note takes up: its first line and its extra lines.
pub(crate) fn note_span(line: usize, text: &str) -> Range<usize> {
    line..line + text.lines().count().max(1)
}

/// The line ending a file uses: Windows' if it has any, otherwise `\n`.
pub(crate) fn newline_of(content: &str) -> &'static str {
    if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

/// Writes `lines` as the new content of the notes file at `path`, which had `content`, with
/// its line endings and its final new line. Returns the new content.
pub(crate) fn write_lines(path: &Path, content: &str, lines: &[String]) -> io::Result<String> {
    let newline = newline_of(content);
    let mut updated = lines.join(newline);
    if content.ends_with('\n') && !lines.is_empty() {
        updated.push_str(newline);
    }
    write_file(path, &updated)?;
    Ok(updated)
}

/// Writes `content` to the notes file at `path`.
fn write_file(path: &Path, content: &str) -> io::Result<()> {
    // Write a copy and swap it in, so a failed write can't leave a half-written file.
    let tmp = path.with_extension("md.tmp");
    fs::write(&tmp, content)?;
    fs::rename(&tmp, path)
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

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 13).unwrap()
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
            today(),
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# 2026-09-26\r\n\r\n- [x] 14:32 one ✅ 2026-10-13\r\n  two\r\n- 15:00 three\r\n"
        );

        set_done(
            &dir,
            "2026-09-26.md",
            &note_ref(2, "14:32", true, "one ✅ 2026-10-13\ntwo"),
            false,
            today(),
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# 2026-09-26\r\n\r\n- [ ] 14:32 one\r\n  two\r\n- 15:00 three\r\n"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_open_task_stays_one_after_done_and_not_done() {
        let dir = temp_dir("open-task");
        let path = dir.join("todo.md");
        fs::write(&path, "# todo\n\n- [ ] 09:00 call the bank\n").unwrap();

        set_done(
            &dir,
            "todo.md",
            &note_ref(2, "09:00", false, "call the bank"),
            true,
            today(),
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# todo\n\n- [x] 09:00 call the bank ✅ 2026-10-13\n"
        );
        set_done(
            &dir,
            "todo.md",
            &note_ref(2, "09:00", true, "call the bank ✅ 2026-10-13"),
            false,
            today(),
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# todo\n\n- [ ] 09:00 call the bank\n"
        );

        // An edit keeps the checkbox too.
        let note = note_ref(2, "09:00", false, "call the bank");
        set_text(&dir, "todo.md", &note, "call the bank today").unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# todo\n\n- [ ] 09:00 call the bank today\n"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn done_date_goes_after_the_text_and_only_a_trailing_one_comes_off() {
        let dir = temp_dir("done-date");
        let path = dir.join("todo.md");
        fs::write(
            &path,
            "# todo\n\n- 09:00\n- [x] 10:00 ✅ 2026-10-01\n- [x] 11:00 a ✅ 2026-10-01 b\n",
        )
        .unwrap();

        set_done(
            &dir,
            "todo.md",
            &note_ref(2, "09:00", false, ""),
            true,
            today(),
        )
        .unwrap();
        set_done(
            &dir,
            "todo.md",
            &note_ref(3, "10:00", true, "✅ 2026-10-01"),
            false,
            today(),
        )
        .unwrap();
        set_done(
            &dir,
            "todo.md",
            &note_ref(4, "11:00", true, "a ✅ 2026-10-01 b"),
            false,
            today(),
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# todo\n\n- [x] 09:00 ✅ 2026-10-13\n- [ ] 10:00\n- [ ] 11:00 a ✅ 2026-10-01 b\n"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_edit_keeps_the_done_date() {
        let dir = temp_dir("edit-done-date");
        let path = dir.join("todo.md");
        fs::write(&path, "# todo\n\n- [x] 09:00 a ✅ 2026-10-01\n  more\n").unwrap();

        // The text comes without the date, as the reader shows it.
        let note = note_ref(2, "09:00", true, "a ✅ 2026-10-01\nmore");
        set_text(&dir, "todo.md", &note, "b\nmore").unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# todo\n\n- [x] 09:00 b ✅ 2026-10-01\n  more\n"
        );
        // A date typed in is kept as it is.
        let note = note_ref(2, "09:00", true, "b ✅ 2026-10-01\nmore");
        set_text(&dir, "todo.md", &note, "c ✅ 2026-10-05").unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# todo\n\n- [x] 09:00 c ✅ 2026-10-05\n"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn splits_off_the_done_date() {
        let date = |d| NaiveDate::from_ymd_opt(2026, 10, d);
        assert_eq!(
            split_done_date("buy milk ✅ 2026-10-13 "),
            ("buy milk", date(13))
        );
        assert_eq!(split_done_date("✅ 2026-10-01"), ("", date(1)));
        assert_eq!(split_done_date("buy milk"), ("buy milk", None));
        assert_eq!(split_done_date("a✅ 2026-10-13"), ("a✅ 2026-10-13", None));
        assert_eq!(split_done_date("a ✅ 2026-10-1"), ("a ✅ 2026-10-1", None));
        assert_eq!(
            split_done_date("a ✅ 2026-02-30"),
            ("a ✅ 2026-02-30", None)
        );
        assert_eq!(
            split_done_date("a 📅 2026-10-13"),
            ("a 📅 2026-10-13", None)
        );
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
    fn deleting_a_note_keeps_empty_headings_that_are_no_day() {
        let dir = temp_dir("delete-other-headings");
        let path = dir.join("plan.md");
        fs::write(
            &path,
            "# plan\n\n## Ideas\n\n## 2026-09-26\n\n- 09:00 one\n\n## Done\n\n\
             ## 2026-09-27\n\n## 2026-13-01\n",
        )
        .unwrap();

        delete(&dir, "plan.md", &note_ref(6, "09:00", false, "one")).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# plan\n\n## Ideas\n\n## Done\n\n## 2026-13-01\n"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn undoes_a_change_only_while_the_file_is_as_it_left_it() {
        let change = Change {
            before: "a\r\n".into(),
            after: "b\r\n".into(),
            images: Vec::new(),
        };
        assert_eq!(undone(&change, "b\r\n"), Some("a\r\n"));
        assert_eq!(undone(&change, "b\n"), None);
        assert_eq!(undone(&change, "b\r\n- 11:00 new\r\n"), None);
    }

    #[test]
    fn undo_puts_back_the_file_exactly_as_it_was() {
        let dir = temp_dir("undo");
        let path = dir.join("idea.md");
        let content = "# idea\r\n\r\n## 2026-09-26\r\n\r\n- [x] 09:00 one ✅ 2026-10-01\r\n\r\n\
                       ## 2026-09-28\r\n\r\n- 10:00 two\r\n";
        fs::write(&path, content).unwrap();
        let note = note_ref(4, "09:00", true, "one ✅ 2026-10-01");

        // The note comes back with its day heading.
        let change = delete(&dir, "idea.md", &note).unwrap();
        assert_eq!(change.before, content);
        assert_eq!(change.after, fs::read_to_string(&path).unwrap());
        undo(&dir, "idea.md", &change).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), content);

        // A note done again keeps the day it was first done.
        let change = set_done(&dir, "idea.md", &note, false, today()).unwrap();
        undo(&dir, "idea.md", &change).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), content);

        // Once something else has written to the file, nothing is undone.
        let change = set_text(&dir, "idea.md", &note, "edited").unwrap();
        let added = format!("{}- 11:00 new\r\n", change.after);
        fs::write(&path, &added).unwrap();
        let err = undo(&dir, "idea.md", &change).unwrap_err();
        assert_eq!(err.to_string(), "idea.md has changed since");
        assert_eq!(fs::read_to_string(&path).unwrap(), added);
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
            true,
            today()
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
        assert_eq!(
            parse_note_line("- [ ] 14:32 text"),
            Some((false, "14:32", "text"))
        );
        assert_eq!(parse_note_line("- [ ]14:32 text"), None);
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

        let hits = search(&dir, " MILK ", DateFormat::DayMonthYear, 10).unwrap();
        let found: Vec<_> = hits
            .iter()
            .map(|h| (h.file.as_str(), h.line, h.day.as_deref(), h.time.as_str()))
            .collect();
        assert_eq!(
            found,
            [
                ("inbox.md", 4, Some("2026-09-26"), "08:00"),
                ("2026-09-25.md", 2, Some("2026-09-25"), "09:00"),
            ]
        );
        assert_eq!(
            search(&dir, "bread", DateFormat::DayMonthYear, 10).unwrap()[0].text,
            "more milk\nand bread"
        );
        assert_eq!(
            search(&dir, "milk", DateFormat::DayMonthYear, 1)
                .unwrap()
                .len(),
            1
        );
        assert!(search(&dir, "  ", DateFormat::DayMonthYear, 10)
            .unwrap()
            .is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn search_takes_only_date_headings_as_days() {
        let dir = temp_dir("search-days");
        fs::write(
            dir.join("ideas.md"),
            "# ideas\n\n- 09:00 milk a\n\n## Ideas\n\n- 23:00 milk b\n\n\
             ## 2026-09-20\n\n- 10:00 milk c\n\n## 2026-02-30\n\n- 08:00 milk d\n",
        )
        .unwrap();
        fs::write(
            dir.join("2026-09-25.md"),
            "# 2026-09-25\n\n- 09:00 milk e\n",
        )
        .unwrap();

        let found = |limit| -> Vec<(usize, Option<String>)> {
            search(&dir, "milk", DateFormat::DayMonthYear, limit)
                .unwrap()
                .into_iter()
                .map(|hit| (hit.line, hit.day))
                .collect()
        };
        let day = |date: &str| Some(date.to_string());
        // Notes with a day newest first; then those under no heading, a heading that is not a
        // date or not a real date, in file order rather than by time.
        assert_eq!(
            found(10),
            [
                (2, day("2026-09-25")),
                (10, day("2026-09-20")),
                (2, None),
                (6, None),
                (14, None),
            ]
        );
        // The limit keeps the notes with a day.
        assert_eq!(found(2), [(2, day("2026-09-25")), (10, day("2026-09-20"))]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn searches_notes_as_they_are_shown() {
        let dir = temp_dir("search-shown");
        fs::write(
            dir.join("todo.md"),
            "# todo\n\n- 09:00 call the bank 📅 2026-10-15\n\
             - 10:00 receipt ![Image #1](images/2026-10-07_14-32-05.png)\n\
             - [x] 11:00 paid ✅ 2026-10-13\n",
        )
        .unwrap();
        let found = |query: &str, format| -> Vec<String> {
            search(&dir, query, format, 10)
                .unwrap()
                .into_iter()
                .map(|hit| hit.time)
                .collect()
        };
        let day_first = DateFormat::DayMonthYear;
        // Dates in the date format, not as stored.
        assert_eq!(found("📅 15/10/2026", day_first), ["09:00"]);
        assert!(found("2026-10-15", day_first).is_empty());
        assert_eq!(found("10/15/2026", DateFormat::MonthDayYear), ["09:00"]);
        assert_eq!(found("2026-10-15", DateFormat::YearMonthDay), ["09:00"]);
        // Images as their labels, not their paths.
        assert_eq!(found("[image #1]", day_first), ["10:00"]);
        assert!(found("images/", day_first).is_empty());
        assert!(found("png", day_first).is_empty());
        // The day a note was done is not shown.
        assert!(found("13/10/2026", day_first).is_empty());
        assert_eq!(found("paid", day_first), ["11:00"]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn shows_a_note_as_the_windows_do() {
        let format = DateFormat::DayMonthYear;
        assert_eq!(
            shown_text(
                "a 📅 2026-10-15 ✅ 2026-10-01 b ✅ 2026-10-13\n📅 2026-10-15",
                format
            ),
            "a 📅 15/10/2026 ✅ 01/10/2026 b\n📅 15/10/2026"
        );
        // Not dates: not real, part of a longer number, or without the space.
        let not_dates = "📅 2026-02-30 📅 2026-10-123 📅2026-10-15 📅 2026-10-1";
        assert_eq!(shown_text(not_dates, format), not_dates);
        assert_eq!(
            shown_text("![Image #2](../images/a.png) ✅ 2026-10-13", format),
            "[Image #2]"
        );
    }

    #[test]
    fn finds_due_notes_overdue_first() {
        let dir = temp_dir("due");
        fs::write(
            dir.join("2026-10-10.md"),
            [
                "# 2026-10-10",
                "",
                "- 09:00 today 📅 2026-10-13",
                "- 09:30 future 📅 2026-10-14",
                "- [x] 10:00 done 📅 2026-10-01",
                "- 10:30 no date",
                "- 11:00 overdue 📅 2026-10-11 then 📅 2026-10-20",
                "- 11:30 not a date 📅 2026-02-30 or 📅 2026-10-123",
            ]
            .join("\n"),
        )
        .unwrap();
        fs::write(
            dir.join("inbox.md"),
            [
                "# Inbox",
                "",
                "## 2026-10-01",
                "",
                "- 08:00 call back",
                "  about 📅 2026-10-12",
                "",
                "## 2026-10-02",
                "",
                "- 08:30 also today 📅 2026-10-13",
            ]
            .join("\n"),
        )
        .unwrap();

        let today = NaiveDate::from_ymd_opt(2026, 10, 13).unwrap();
        let found: Vec<_> = due(&dir, today, false)
            .unwrap()
            .into_iter()
            .map(|d| (d.file, d.note.line, d.note.text, d.date, d.overdue))
            .collect();
        let entry = |file: &str, line: usize, text: &str, date: &str, overdue: i64| {
            (
                String::from(file),
                line,
                String::from(text),
                String::from(date),
                overdue,
            )
        };
        assert_eq!(
            found,
            [
                entry(
                    "2026-10-10.md",
                    6,
                    "overdue 📅 2026-10-11 then 📅 2026-10-20",
                    "2026-10-11",
                    2
                ),
                entry(
                    "inbox.md",
                    4,
                    "call back\nabout 📅 2026-10-12",
                    "2026-10-12",
                    1
                ),
                entry("inbox.md", 9, "also today 📅 2026-10-13", "2026-10-13", 0),
                entry("2026-10-10.md", 2, "today 📅 2026-10-13", "2026-10-13", 0),
            ]
        );
        assert!(due(&dir.join("missing"), today, false).unwrap().is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn finds_notes_ahead_soonest_first() {
        let dir = temp_dir("ahead");
        fs::create_dir_all(dir.join("archive")).unwrap();
        fs::write(
            dir.join("todo.md"),
            [
                "# todo",
                "",
                "- 09:00 next week 📅 2026-10-20",
                "- 09:30 today 📅 2026-10-13",
                "- [x] 10:00 done 📅 2026-10-15",
                "- 10:30 overdue 📅 2026-10-12",
                "- 11:00 tomorrow 📅 2026-10-14",
            ]
            .join("\n"),
        )
        .unwrap();
        fs::write(
            dir.join("archive").join("todo.md"),
            "# todo\n\n- 09:00 archived 📅 2026-10-14\n",
        )
        .unwrap();

        let today = NaiveDate::from_ymd_opt(2026, 10, 13).unwrap();
        let found: Vec<_> = due(&dir, today, true)
            .unwrap()
            .into_iter()
            .map(|d| (d.note.line, d.date, d.overdue))
            .collect();
        assert_eq!(
            found,
            [
                (6, String::from("2026-10-14"), -1),
                (2, String::from("2026-10-20"), -7),
            ]
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn finds_notes_done_lately_latest_day_first() {
        let dir = temp_dir("done-since");
        fs::create_dir_all(dir.join("work")).unwrap();
        fs::create_dir_all(dir.join("archive")).unwrap();
        fs::write(
            dir.join("inbox.md"),
            [
                "# Inbox",
                "",
                "## 2026-10-01",
                "",
                "- [x] 08:00 six days ago ✅ 2026-10-07",
                "- [x] 08:30 a week ago ✅ 2026-10-06",
                "- [x] 09:00 today ✅ 2026-10-13",
                "  second line",
                "- 09:30 not done ✅ 2026-10-13",
                "- [x] 10:00 done without a date",
            ]
            .join("\n"),
        )
        .unwrap();
        fs::write(
            dir.join("work").join("quicklly.md"),
            "# work/quicklly\n\n- [x] 09:00 ship #work/quicklly ✅ 2026-10-13\n",
        )
        .unwrap();
        fs::write(
            dir.join("archive").join("todo.md"),
            "# todo\n\n- [x] 09:00 archived ✅ 2026-10-10\n- [x] 10:00 long ago ✅ 2026-09-01\n",
        )
        .unwrap();

        let from = today() - chrono::Days::new(6);
        let found: Vec<_> = done_since(&dir, from)
            .unwrap()
            .into_iter()
            .map(|d| (d.file, d.text, d.date))
            .collect();
        let entry = |file: &str, text: &str, date: &str| {
            (String::from(file), String::from(text), String::from(date))
        };
        assert_eq!(
            found,
            [
                entry("inbox.md", "today\nsecond line", "2026-10-13"),
                entry("work/quicklly.md", "ship #work/quicklly", "2026-10-13"),
                entry("archive/todo.md", "archived", "2026-10-10"),
                entry("inbox.md", "six days ago", "2026-10-07"),
            ]
        );
        assert!(done_since(&dir.join("missing"), from).unwrap().is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn moves_the_date_due_goes_by() {
        let moved = |lines: &[&str], date| {
            let mut lines: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
            move_date(&mut lines, date);
            lines
        };
        let tomorrow = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap();
        let next_week = NaiveDate::from_ymd_opt(2026, 10, 20).unwrap();
        // Overdue, to tomorrow; today, to a week on.
        assert_eq!(
            moved(&["- 09:00 a 📅 2026-10-01 b"], tomorrow),
            ["- 09:00 a 📅 2026-10-14 b"]
        );
        assert_eq!(
            moved(&["- 09:00 a 📅 2026-10-13"], next_week),
            ["- 09:00 a 📅 2026-10-20"]
        );
        // Only the first real date, wherever it is in the note; the done date stays.
        assert_eq!(
            moved(
                &[
                    "- [x] 09:00 a 📅 2026-02-30 📅 2026-10-123 ✅ 2026-10-02",
                    "  ![x](images/a.png)",
                    "",
                    "  b 📅 2026-10-11 then 📅 2026-10-12",
                ],
                tomorrow
            ),
            [
                "- [x] 09:00 a 📅 2026-02-30 📅 2026-10-123 ✅ 2026-10-02",
                "  ![x](images/a.png)",
                "",
                "  b 📅 2026-10-14 then 📅 2026-10-12",
            ]
        );
        assert_eq!(moved(&["- 09:00 no date"], tomorrow), ["- 09:00 no date"]);
    }

    #[test]
    fn snoozes_a_note_and_keeps_the_rest_of_the_file() {
        let dir = temp_dir("snooze");
        let path = dir.join("inbox.md");
        fs::write(
            &path,
            "# Inbox\r\n\r\n- 08:00 call back\r\n  about 📅 2026-10-12\r\n- 09:00 b 📅 2026-10-12\r\n",
        )
        .unwrap();

        let note = note_ref(2, "08:00", false, "call back\nabout 📅 2026-10-12");
        snooze(&dir, "inbox.md", &note, today().succ_opt().unwrap()).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# Inbox\r\n\r\n- 08:00 call back\r\n  about 📅 2026-10-14\r\n- 09:00 b 📅 2026-10-12\r\n"
        );
        // A note that moved since is not touched.
        assert!(snooze(&dir, "inbox.md", &note, today()).is_err());
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
        assert_eq!(
            search(&dir, "#todo", DateFormat::DayMonthYear, 10).unwrap()[0].day,
            Some("2026-09-26".to_string())
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn leaves_the_archive_out() {
        let dir = temp_dir("list-archive");
        fs::write(dir.join("todo.md"), "# todo\n\n- 09:00 milk\n").unwrap();
        fs::create_dir_all(dir.join("archive")).unwrap();
        fs::write(
            dir.join("archive").join("todo.md"),
            "# todo\n\n- [x] 08:00 old milk ✅ 2026-09-01\n",
        )
        .unwrap();

        let names: Vec<_> = list_files(&dir)
            .unwrap()
            .into_iter()
            .map(|f| f.name)
            .collect();
        assert_eq!(names, ["todo.md"]);
        assert_eq!(
            search(&dir, "milk", DateFormat::DayMonthYear, 10)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(list_tags(&dir).unwrap(), ["todo"]);
        assert!(read_file(&dir, "archive/todo.md").is_err());
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
    fn accepts_notes_files_in_folders_and_nothing_outside() {
        for name in [
            "todo.md",
            "a/b.md",
            "work/quicklly.md",
            "a/b/c.md",
            "archive.md",
        ] {
            assert!(is_notes_file(name), "{name}");
        }
        for name in [
            "../x.md",
            "a/../../x.md",
            "a/./b.md",
            "/x.md",
            "C:/x.md",
            "C:x.md",
            "a\\b.md",
            "a//b.md",
            "archive/x.md",
            "Archive/x.md",
            "images/x.md",
            ".obsidian/x.md",
            "a/.x.md",
            "a/b.txt",
            "a/",
            ".md",
            "",
        ] {
            assert!(!is_notes_file(name), "{name}");
        }
        let dir = Path::new("notes");
        assert_eq!(
            notes_file(dir, "work/quicklly.md").unwrap(),
            dir.join("work").join("quicklly.md")
        );
        assert_eq!(
            notes_file(dir, "../x.md").unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }

    #[test]
    fn lists_files_in_folders_as_a_tree() {
        let dir = temp_dir("list-tree");
        for folder in ["work/deep", "archive/work", "images", ".obsidian", "alpha"] {
            fs::create_dir_all(dir.join(folder)).unwrap();
        }
        for name in [
            "todo.md",
            "inbox.md",
            "2026-09-26.md",
            "work.md",
            "work/quicklly.md",
            "work/2026-09-25.md",
            "work/2026-09-26.md",
            "work/inbox.md",
            "work/deep/x.md",
            "work/deep/notes.txt",
            "alpha/b.md",
            "archive/work/old.md",
            "archive/old.md",
            "images/x.md",
            ".obsidian/x.md",
        ] {
            fs::write(path_of(&dir, name), "- 09:00 #work/quicklly milk\n").unwrap();
        }

        let names: Vec<_> = list_files(&dir)
            .unwrap()
            .into_iter()
            .map(|f| f.name)
            .collect();
        assert_eq!(
            names,
            [
                "inbox.md",
                "todo.md",
                "work.md",
                "2026-09-26.md",
                "alpha/b.md",
                "work/inbox.md",
                "work/quicklly.md",
                "work/2026-09-26.md",
                "work/2026-09-25.md",
                "work/deep/x.md",
            ]
        );
        assert_eq!(
            list_tags(&dir).unwrap(),
            [
                "alpha/b",
                "todo",
                "work",
                "work/deep/x",
                "work/inbox",
                "work/quicklly"
            ]
        );
        let hit = &search(&dir, "milk", DateFormat::DayMonthYear, 50).unwrap();
        assert_eq!(hit.len(), names.len());
        assert!(hit
            .iter()
            .any(|h| h.file == "work/2026-09-25.md" && h.day.as_deref() == Some("2026-09-25")));
        assert_eq!(read_file(&dir, "work/deep/x.md").unwrap().len(), 1);
        assert!(read_file(&dir, "archive/work/old.md").is_err());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn links_new_images_from_a_nested_file() {
        let dir = temp_dir("edit-nested");
        fs::create_dir_all(dir.join("work")).unwrap();
        let path = dir.join("work").join("q.md");
        fs::write(
            &path,
            "# work/q\n\n- 09:00 a ![Image #1](../images/a.png)\n",
        )
        .unwrap();

        let note = note_ref(2, "09:00", false, "a ![Image #1](../images/a.png)");
        set_text(
            &dir,
            "work/q.md",
            &note,
            "a ![Image #1](../images/a.png)\n![Image #2](images/b.png)",
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# work/q\n\n- 09:00 a ![Image #1](../images/a.png)\n  ![Image #2](../images/b.png)\n"
        );
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

    #[test]
    fn lists_a_file_it_cant_read_and_leaves_it_out_of_the_rest() {
        let dir = temp_dir("reader-unreadable");
        fs::create_dir_all(dir.join("archive")).unwrap();
        // Saved by hand in Windows-1251, say: not UTF-8, though most of it would be.
        let ansi: &[u8] =
            b"# todo\n\n- 09:00 milk \xec\xee\xeb\xee\xea\xee \xf0\x9f\x93\x85 2026-10-01\n\
                            - [x] 10:00 milk \xe2\x9c\x85 2026-10-13\n";
        fs::write(dir.join("todo.md"), ansi).unwrap();
        fs::write(dir.join("archive").join("todo.md"), ansi).unwrap();
        fs::write(
            dir.join("ideas.md"),
            "# ideas\n\n- 09:00 milk 📅 2026-10-01\n- [x] 10:00 milk ✅ 2026-10-13\n",
        )
        .unwrap();

        let files: Vec<_> = list_files(&dir)
            .unwrap()
            .into_iter()
            .map(|f| (f.name, f.notes, f.done))
            .collect();
        assert_eq!(
            files,
            [
                ("ideas.md".to_string(), 2, 1),
                ("todo.md".to_string(), 0, 0)
            ]
        );
        assert_eq!(list_tags(&dir).unwrap(), ["ideas", "todo"]);
        let hits: Vec<_> = search(&dir, "milk", DateFormat::DayMonthYear, 10)
            .unwrap()
            .into_iter()
            .map(|hit| hit.file)
            .collect();
        assert_eq!(hits, ["ideas.md", "ideas.md"]);
        let found: Vec<_> = due(&dir, today(), false)
            .unwrap()
            .into_iter()
            .map(|d| d.file)
            .collect();
        assert_eq!(found, ["ideas.md"]);
        let found: Vec<_> = done_since(&dir, today())
            .unwrap()
            .into_iter()
            .map(|d| d.file)
            .collect();
        assert_eq!(found, ["ideas.md"]);
        // Opening it says why; reading it by name says which file.
        let err = read_file(&dir, "todo.md").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        let err = read_named(&dir, "archive/todo.md").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(
            err.to_string()
                .starts_with("couldn't read archive/todo.md: "),
            "{err}"
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
