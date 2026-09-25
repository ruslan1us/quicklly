use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use chrono::NaiveDateTime;

/// Appends a note as `- HH:MM text` to the daily file `YYYY-MM-DD.md` in `dir`.
///
/// The file (and `dir`) are created on first write, starting with a `# YYYY-MM-DD` heading.
/// Line breaks in `text` are folded into spaces so a note always stays one list item.
/// Returns the path of the file written to, or `None` if the note is blank.
pub fn append_note(dir: &Path, now: NaiveDateTime, text: &str) -> io::Result<Option<PathBuf>> {
    let text = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if text.is_empty() {
        return Ok(None);
    }

    fs::create_dir_all(dir)?;
    let date = now.format("%Y-%m-%d").to_string();
    let path = dir.join(format!("{date}.md"));
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;

    let mut entry = String::new();
    if file.metadata()?.len() == 0 {
        entry.push_str(&format!("# {date}\n\n"));
    }
    entry.push_str(&format!("- {} {text}\n", now.format("%H:%M")));
    file.write_all(entry.as_bytes())?;

    Ok(Some(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn at(h: u32, m: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 9, 26)
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
    fn writes_heading_once_then_appends() {
        let dir = temp_dir("append");
        append_note(&dir, at(14, 32), "first").unwrap();
        let path = append_note(&dir, at(15, 5), "second").unwrap().unwrap();

        assert_eq!(path, dir.join("2026-09-26.md"));
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# 2026-09-26\n\n- 14:32 first\n- 15:05 second\n"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn folds_line_breaks_and_skips_blank_notes() {
        let dir = temp_dir("fold");
        assert_eq!(append_note(&dir, at(9, 0), "  \r\n ").unwrap(), None);
        assert!(!dir.exists());

        let path = append_note(&dir, at(9, 0), " one\r\n\ntwo ")
            .unwrap()
            .unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# 2026-09-26\n\n- 09:00 one two\n"
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
