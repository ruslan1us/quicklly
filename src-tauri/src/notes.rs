use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// File that collects all notes in inbox mode.
pub const INBOX_FILE: &str = "inbox.md";

/// Which file notes go to.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    /// One file per day, `YYYY-MM-DD.md`.
    #[default]
    Daily,
    /// A single `inbox.md`, grouped under a `## YYYY-MM-DD` heading per day.
    Inbox,
}

/// The lines of a note as they are stored: trimmed, without blank lines.
pub fn note_lines(text: &str) -> Vec<&str> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect()
}

/// The first `#tag` in `text`, lowercased: a `#` at the start of a word, then a letter, then
/// letters, digits, `_` or `-`. So `#1`, `#2026-09-26` and `C#` are not tags.
pub fn first_tag(text: &str) -> Option<String> {
    let mut previous = ' ';
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        let starts_tag = c == '#'
            && previous.is_whitespace()
            && chars.peek().is_some_and(|&(_, next)| next.is_alphabetic());
        previous = c;
        if !starts_tag {
            continue;
        }
        let tag: String = text[i + 1..]
            .chars()
            .take_while(|&c| c.is_alphanumeric() || c == '_' || c == '-')
            .collect();
        return Some(tag.to_lowercase());
    }
    None
}

/// Appends a note as `- HH:MM text` to its notes file in `dir`: the file of its first
/// `#tag` (`tag.md`), otherwise the daily file or the inbox, depending on `mode`.
///
/// The file (and `dir`) are created on first write. A daily file starts with a
/// `# YYYY-MM-DD` heading; the inbox and tag files start with `# Inbox` / `# tag` and get a
/// `## YYYY-MM-DD` heading whenever the first note of a new day is added.
/// A multi-line note stays one list item: further lines are indented under the first one,
/// and blank lines are dropped.
/// Returns the path of the file written to, or `None` if the note is blank.
pub fn append_note(
    dir: &Path,
    mode: Mode,
    now: NaiveDateTime,
    text: &str,
) -> io::Result<Option<PathBuf>> {
    let text = note_lines(text).join("\n  ");
    if text.is_empty() {
        return Ok(None);
    }

    fs::create_dir_all(dir)?;
    let date = now.format("%Y-%m-%d").to_string();
    // A grouped file has a title and a heading per day; a daily file needs neither.
    let (path, grouped_title) = match (first_tag(&text), mode) {
        (Some(tag), _) => (dir.join(format!("{tag}.md")), Some(tag)),
        (None, Mode::Inbox) => (dir.join(INBOX_FILE), Some("Inbox".to_string())),
        (None, Mode::Daily) => (dir.join(format!("{date}.md")), None),
    };
    let existing = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e),
    };

    let mut entry = String::new();
    // The file may have been edited by hand without a trailing newline.
    if !existing.is_empty() && !existing.ends_with('\n') {
        entry.push('\n');
    }
    match grouped_title {
        None => {
            if existing.is_empty() {
                entry.push_str(&format!("# {date}\n\n"));
            }
        }
        Some(title) => {
            if existing.is_empty() {
                entry.push_str(&format!("# {title}\n\n"));
            }
            let heading = format!("## {date}");
            // Notes are appended, so the last day heading in the file is the current one.
            let last_day = existing.lines().rev().find(|line| line.starts_with("## "));
            if last_day.map(str::trim_end) != Some(heading.as_str()) {
                if !existing.is_empty() {
                    entry.push('\n');
                }
                entry.push_str(&format!("{heading}\n\n"));
            }
        }
    }
    entry.push_str(&format!("- {} {text}\n", now.format("%H:%M")));

    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    file.write_all(entry.as_bytes())?;

    Ok(Some(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn at(day: u32, h: u32, m: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 9, day)
            .unwrap()
            .and_hms_opt(h, m, 0)
            .unwrap()
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("quicklly-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn daily_writes_heading_once_then_appends() {
        let dir = temp_dir("daily");
        append_note(&dir, Mode::Daily, at(26, 14, 32), "first").unwrap();
        let path = append_note(&dir, Mode::Daily, at(26, 15, 5), "second")
            .unwrap()
            .unwrap();

        assert_eq!(path, dir.join("2026-09-26.md"));
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# 2026-09-26\n\n- 14:32 first\n- 15:05 second\n"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn inbox_groups_notes_by_day() {
        let dir = temp_dir("inbox");
        append_note(&dir, Mode::Inbox, at(26, 14, 32), "first").unwrap();
        append_note(&dir, Mode::Inbox, at(26, 15, 5), "second").unwrap();
        let path = append_note(&dir, Mode::Inbox, at(27, 9, 0), "next day")
            .unwrap()
            .unwrap();

        assert_eq!(path, dir.join("inbox.md"));
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# Inbox\n\n\
             ## 2026-09-26\n\n- 14:32 first\n- 15:05 second\n\n\
             ## 2026-09-27\n\n- 09:00 next day\n"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn finds_the_first_tag() {
        assert_eq!(first_tag("buy milk #Todo #work"), Some("todo".into()));
        assert_eq!(first_tag("#идея: трекер кофе"), Some("идея".into()));
        assert_eq!(first_tag("line one\n  #work-2 next"), Some("work-2".into()));
        assert_eq!(first_tag("learning C# and #1 of #2026-09-26"), None);
        assert_eq!(first_tag("a#b and # alone"), None);
    }

    #[test]
    fn tagged_notes_go_to_the_tag_file_grouped_by_day() {
        let dir = temp_dir("tag");
        append_note(&dir, Mode::Daily, at(26, 9, 0), "untagged").unwrap();
        append_note(&dir, Mode::Daily, at(26, 10, 0), "buy milk #Todo #work").unwrap();
        let path = append_note(&dir, Mode::Inbox, at(27, 11, 0), "call the bank #todo")
            .unwrap()
            .unwrap();

        assert_eq!(path, dir.join("todo.md"));
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# todo\n\n\
             ## 2026-09-26\n\n- 10:00 buy milk #Todo #work\n\n\
             ## 2026-09-27\n\n- 11:00 call the bank #todo\n"
        );
        assert_eq!(
            fs::read_to_string(dir.join("2026-09-26.md")).unwrap(),
            "# 2026-09-26\n\n- 09:00 untagged\n"
        );
        assert!(!dir.join("work.md").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn adds_missing_trailing_newline() {
        let dir = temp_dir("newline");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("inbox.md");
        fs::write(&path, "# Inbox\n\n## 2026-09-26\n\n- 14:32 edited by hand").unwrap();

        append_note(&dir, Mode::Inbox, at(26, 15, 0), "more").unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# Inbox\n\n## 2026-09-26\n\n- 14:32 edited by hand\n- 15:00 more\n"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn indents_extra_lines_and_skips_blank_notes() {
        let dir = temp_dir("lines");
        assert_eq!(
            append_note(&dir, Mode::Daily, at(26, 9, 0), "  \r\n ").unwrap(),
            None
        );
        assert!(!dir.exists());

        let path = append_note(&dir, Mode::Daily, at(26, 9, 0), " one\r\n\n two \nthree")
            .unwrap()
            .unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# 2026-09-26\n\n- 09:00 one\n  two\n  three\n"
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
