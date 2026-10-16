use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::ops::Range;
use std::path::Path;

use chrono::{Local, NaiveDate};
use tauri::AppHandle;

use crate::images::IMAGES_DIR;
use crate::reader::{self, Item};
use crate::settings;

/// Folder in the notes folder that done notes are moved to, a file for each notes file.
pub const ARCHIVE_DIR: &str = "archive";

/// A done note to move to the archive.
#[derive(Debug, PartialEq)]
struct Moved {
    /// The `## YYYY-MM-DD` heading it is under, if any.
    day: Option<String>,
    /// Its lines in the file.
    lines: Range<usize>,
}

/// The notes in `content` that were done (`✅ YYYY-MM-DD`) at least `days` days before
/// `today`, in file order. Notes done without a date are left alone.
fn due_for_archive(content: &str, today: NaiveDate, days: i64) -> Vec<Moved> {
    let mut day = None;
    let mut due = Vec::new();
    for item in reader::parse(content) {
        match item {
            Item::Day { date } => day = Some(date),
            Item::Note {
                line,
                done: true,
                text,
                ..
            } => {
                let (_, done_on) = reader::split_done_date(text.lines().next().unwrap_or(""));
                if done_on.is_some_and(|done_on| (today - done_on).num_days() >= days) {
                    due.push(Moved {
                        day: day.clone(),
                        lines: reader::note_span(line, &text),
                    });
                }
            }
            Item::Note { .. } => {}
        }
    }
    due
}

/// What to append to an archive file that has `existing` in it, to add `notes` (each its day
/// and its lines): a new file starts with `title`, and notes go under a `## YYYY-MM-DD` heading
/// like in the inbox, a new one unless the last heading is already their day.
fn archive_entry(
    existing: &str,
    title: &str,
    notes: &[(Option<String>, Vec<String>)],
    newline: &str,
) -> String {
    let mut lines: Vec<String> = Vec::new();
    // The file may have been edited by hand without a trailing newline.
    if !existing.is_empty() && !existing.ends_with('\n') {
        lines.push(String::new());
    }
    if existing.is_empty() {
        lines.extend([title.to_string(), String::new()]);
    }
    let mut last_day = existing
        .lines()
        .rev()
        .find(|line| line.starts_with("## "))
        .map(|line| line.trim_end().to_string());
    let mut fresh = existing.is_empty();
    for (day, note) in notes {
        if let Some(day) = day {
            let heading = format!("## {day}");
            if last_day.as_ref() != Some(&heading) {
                if !fresh {
                    lines.push(String::new());
                }
                lines.extend([heading.clone(), String::new()]);
                last_day = Some(heading);
            }
        }
        lines.extend(note.iter().cloned());
        fresh = false;
    }
    let mut entry = lines.join(newline);
    entry.push_str(newline);
    entry
}

/// A note's line from the notes file `name` as it goes to the archive: its images are linked
/// from one folder further down.
fn archived_line(line: &str, name: &str) -> String {
    let up = "../".repeat(name.matches('/').count());
    line.replace(
        &format!("]({up}{IMAGES_DIR}/"),
        &format!("](../{up}{IMAGES_DIR}/"),
    )
}

/// Moves the notes of the notes file `name` in `dir` that were done at least `days` days
/// before `today` to `archive/<name>` (in the same folders), keeping their lines and their day.
/// Returns how many. A notes file or archive file that can't be read (not UTF-8, say) is left
/// as it is, and so is the other one.
fn archive_file(dir: &Path, name: &str, today: NaiveDate, days: i64) -> io::Result<usize> {
    let path = reader::notes_file(dir, name)?;
    let Ok(content) = fs::read_to_string(&path) else {
        return Ok(0);
    };
    let due = due_for_archive(&content, today, days);
    if due.is_empty() {
        return Ok(0);
    }
    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    let notes: Vec<_> = due
        .iter()
        .map(|moved| {
            let note = lines[moved.lines.clone()]
                .iter()
                .map(|line| archived_line(line, name))
                .collect();
            (moved.day.clone(), note)
        })
        .collect();

    // The archive is written first: if the notes file can't be written after it, the notes
    // are in both rather than lost.
    let archive = reader::path_of(&dir.join(ARCHIVE_DIR), name);
    fs::create_dir_all(archive.parent().unwrap_or(dir))?;
    let existing = match fs::read_to_string(&archive) {
        Ok(existing) => existing,
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        Err(_) => return Ok(0),
    };
    let title = lines
        .iter()
        .find(|line| line.starts_with("# "))
        .map(|line| line.trim_end().to_string())
        .unwrap_or_else(|| format!("# {}", name.trim_end_matches(".md")));
    let entry = archive_entry(&existing, &title, &notes, reader::newline_of(&content));
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(&archive)?
        .write_all(entry.as_bytes())?;

    for moved in due.iter().rev() {
        lines.drain(moved.lines.clone());
    }
    reader::remove_empty_days(&mut lines);
    reader::write_lines(&path, &content, &lines)?;
    Ok(due.len())
}

/// Moves the notes done at least `days` days before `today` out of every notes file in `dir`
/// into the archive. Returns how many were moved.
pub fn archive_all(dir: &Path, today: NaiveDate, days: i64) -> io::Result<usize> {
    let mut moved = 0;
    for file in reader::list_names(dir)? {
        moved += archive_file(dir, &file, today, days)?;
    }
    Ok(moved)
}

/// Archives the done notes as the settings say, and has open windows reload if any moved.
pub fn archive_done(app: &AppHandle) {
    let Some(days) = settings::archive_after(app).days() else {
        return;
    };
    let result = settings::notes_dir(app)
        .map_err(|e| e.to_string())
        .and_then(|dir| {
            archive_all(&dir, Local::now().date_naive(), days)
                .map_err(|e| format!("in {}: {e}", dir.display()))
        });
    match result {
        Ok(0) => {}
        Ok(_) => crate::notes_changed(app),
        Err(e) => eprintln!("Failed to archive done notes {e}"),
    }
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

    fn read(path: &Path) -> String {
        fs::read_to_string(path).unwrap()
    }

    #[test]
    fn picks_notes_done_long_enough_ago() {
        let content = [
            "# todo",
            "",
            "## 2026-10-01",
            "",
            "- [x] 09:00 a week ago ✅ 2026-10-06",
            "- [x] 09:30 six days ago ✅ 2026-10-07",
            "- [x] 10:00 done without a date",
            "- 10:30 not done ✅ 2026-10-01",
            "- [x] 11:00 date not at the end ✅ 2026-10-01 later",
            "",
            "## 2026-10-02",
            "",
            "- [x] 12:00 long ago ✅ 2026-09-01",
            "  second line",
            "",
            "  third line",
        ]
        .join("\n");
        assert_eq!(
            due_for_archive(&content, today(), 7),
            [
                Moved {
                    day: Some("2026-10-01".into()),
                    lines: 4..5
                },
                Moved {
                    day: Some("2026-10-02".into()),
                    lines: 12..16
                },
            ]
        );
        assert_eq!(due_for_archive(&content, today(), 30).len(), 1);
        assert_eq!(due_for_archive(&content, today(), 1).len(), 3);
    }

    #[test]
    fn archives_a_daily_file_like_a_daily_file() {
        let dir = temp_dir("archive-daily");
        let path = dir.join("2026-10-08.md");
        fs::write(
            &path,
            "# 2026-10-08\n\n- [x] 09:00 old ✅ 2026-10-01\n- 10:00 open\n\
             - [x] 11:00 ![Image #1](images/a.png)\n  more ✅ 2026-10-01\n\
             - [x] 12:00 with image ![Image #1](images/b.png) ✅ 2026-10-02\n  more\n",
        )
        .unwrap();

        assert_eq!(archive_all(&dir, today(), 7).unwrap(), 2);
        assert_eq!(
            read(&path),
            "# 2026-10-08\n\n- 10:00 open\n- [x] 11:00 ![Image #1](images/a.png)\n  more ✅ 2026-10-01\n"
        );
        assert_eq!(
            read(&dir.join("archive").join("2026-10-08.md")),
            "# 2026-10-08\n\n- [x] 09:00 old ✅ 2026-10-01\n\
             - [x] 12:00 with image ![Image #1](../images/b.png) ✅ 2026-10-02\n  more\n"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn archives_grouped_files_by_day_into_an_existing_archive() {
        let dir = temp_dir("archive-inbox");
        let path = dir.join("inbox.md");
        fs::write(
            &path,
            "# Inbox\r\n\r\n## 2026-10-01\r\n\r\n- [x] 09:00 a ✅ 2026-10-02\r\n\r\n\
             ## 2026-10-02\r\n\r\n- [x] 09:00 b ✅ 2026-10-02\r\n- 10:00 c\r\n\r\n\
             ## 2026-10-03\r\n\r\n- [x] 09:00 d ✅ 2026-10-03\r\n",
        )
        .unwrap();
        fs::create_dir_all(dir.join("archive")).unwrap();
        let archive = dir.join("archive").join("inbox.md");
        fs::write(
            &archive,
            "# Inbox\r\n\r\n## 2026-10-01\r\n\r\n- [x] 08:00 earlier ✅ 2026-10-01",
        )
        .unwrap();

        assert_eq!(archive_all(&dir, today(), 7).unwrap(), 3);
        assert_eq!(
            read(&path),
            "# Inbox\r\n\r\n## 2026-10-02\r\n\r\n- 10:00 c\r\n"
        );
        assert_eq!(
            read(&archive),
            "# Inbox\r\n\r\n## 2026-10-01\r\n\r\n- [x] 08:00 earlier ✅ 2026-10-01\r\n\
             - [x] 09:00 a ✅ 2026-10-02\r\n\r\n\
             ## 2026-10-02\r\n\r\n- [x] 09:00 b ✅ 2026-10-02\r\n\r\n\
             ## 2026-10-03\r\n\r\n- [x] 09:00 d ✅ 2026-10-03\r\n"
        );

        // Nothing more is due: nothing changes.
        assert_eq!(archive_all(&dir, today(), 7).unwrap(), 0);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn archives_tag_files_and_files_without_days() {
        let dir = temp_dir("archive-tag");
        fs::write(
            dir.join("todo.md"),
            "# todo\n\n## 2026-10-01\n\n- [x] 09:00 milk #todo ✅ 2026-10-05\n",
        )
        .unwrap();
        fs::write(
            dir.join("My Notes.md"),
            "Written by hand\n\n- [x] 09:00 one ✅ 2026-10-05\n- [x] 10:00 two ✅ 2026-10-06\n",
        )
        .unwrap();

        assert_eq!(archive_all(&dir, today(), 7).unwrap(), 3);
        assert_eq!(read(&dir.join("todo.md")), "# todo\n");
        assert_eq!(
            read(&dir.join("archive").join("todo.md")),
            "# todo\n\n## 2026-10-01\n\n- [x] 09:00 milk #todo ✅ 2026-10-05\n"
        );
        assert_eq!(read(&dir.join("My Notes.md")), "Written by hand\n");
        assert_eq!(
            read(&dir.join("archive").join("My Notes.md")),
            "# My Notes\n\n- [x] 09:00 one ✅ 2026-10-05\n- [x] 10:00 two ✅ 2026-10-06\n"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn archiving_keeps_empty_headings_that_are_no_day() {
        let dir = temp_dir("archive-other-headings");
        let path = dir.join("plan.md");
        fs::write(
            &path,
            "# plan\n\n## Ideas\n\n## 2026-10-01\n\n- [x] 09:00 a ✅ 2026-10-02\n\n## Done\n",
        )
        .unwrap();

        assert_eq!(archive_all(&dir, today(), 7).unwrap(), 1);
        assert_eq!(read(&path), "# plan\n\n## Ideas\n\n## Done\n");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn archives_a_nested_file_into_the_same_folders() {
        let dir = temp_dir("archive-nested");
        fs::create_dir_all(dir.join("work")).unwrap();
        let path = dir.join("work").join("quicklly.md");
        fs::write(
            &path,
            "# work/quicklly\n\n## 2026-10-01\n\n\
             - [x] 09:00 ship ![Image #1](../images/a.png) ✅ 2026-10-05\n- 10:00 open\n",
        )
        .unwrap();

        assert_eq!(archive_all(&dir, today(), 7).unwrap(), 1);
        assert_eq!(
            read(&path),
            "# work/quicklly\n\n## 2026-10-01\n\n- 10:00 open\n"
        );
        assert_eq!(
            read(&dir.join("archive").join("work").join("quicklly.md")),
            "# work/quicklly\n\n## 2026-10-01\n\n\
             - [x] 09:00 ship ![Image #1](../../images/a.png) ✅ 2026-10-05\n"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn appends_after_a_hand_edited_archive() {
        let notes = [(Some("2026-10-02".to_string()), vec!["- [x] 09:00 b".into()])];
        assert_eq!(
            archive_entry(
                "# todo\n\n## 2026-10-01\n\n- [x] 08:00 a",
                "# todo",
                &notes,
                "\n"
            ),
            "\n\n## 2026-10-02\n\n- [x] 09:00 b\n"
        );
        assert_eq!(
            archive_entry("", "# todo", &notes, "\r\n"),
            "# todo\r\n\r\n## 2026-10-02\r\n\r\n- [x] 09:00 b\r\n"
        );
    }

    #[test]
    fn leaves_files_without_due_notes_alone() {
        let dir = temp_dir("archive-none");
        let content = "# todo\n\n- [x] 09:00 no date\n- 10:00 open ✅ 2026-01-01\n";
        fs::write(dir.join("todo.md"), content).unwrap();

        assert_eq!(archive_all(&dir, today(), 1).unwrap(), 0);
        assert_eq!(read(&dir.join("todo.md")), content);
        assert!(!dir.join("archive").exists());
        assert_eq!(archive_all(&dir.join("missing"), today(), 1).unwrap(), 0);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn leaves_files_it_cant_read_alone() {
        let dir = temp_dir("archive-unreadable");
        fs::create_dir_all(dir.join("archive")).unwrap();
        // Saved by hand in Windows-1251, say: not UTF-8.
        let ansi: &[u8] =
            b"# todo\n\n- [x] 09:00 \xec\xee\xeb\xee\xea\xee \xe2\x9c\x85 2026-10-01\n";
        fs::write(dir.join("todo.md"), ansi).unwrap();
        // An archive file that can't be read keeps its notes file as it is too.
        let ideas = "# ideas\n\n- [x] 09:00 a ✅ 2026-10-01\n";
        fs::write(dir.join("ideas.md"), ideas).unwrap();
        fs::write(dir.join("archive").join("ideas.md"), ansi).unwrap();
        fs::write(
            dir.join("inbox.md"),
            "# Inbox\n\n- [x] 09:00 b ✅ 2026-10-01\n",
        )
        .unwrap();

        assert_eq!(archive_all(&dir, today(), 7).unwrap(), 1);
        assert_eq!(fs::read(dir.join("todo.md")).unwrap(), ansi);
        assert!(!dir.join("archive").join("todo.md").exists());
        assert_eq!(read(&dir.join("ideas.md")), ideas);
        assert_eq!(
            fs::read(dir.join("archive").join("ideas.md")).unwrap(),
            ansi
        );
        assert_eq!(read(&dir.join("inbox.md")), "# Inbox\n");
        assert_eq!(
            read(&dir.join("archive").join("inbox.md")),
            "# Inbox\n\n- [x] 09:00 b ✅ 2026-10-01\n"
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
