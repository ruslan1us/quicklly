use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use chrono::NaiveDateTime;

/// Folder in the notes folder that pasted images are saved to.
pub const IMAGES_DIR: &str = "images";

/// File extension for an image of the given MIME type; PNG when it is not one we know.
fn extension(mime: &str) -> &'static str {
    match mime {
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/bmp" => "bmp",
        _ => "png",
    }
}

/// Saves a pasted image as `images/YYYY-MM-DD_HH-MM-SS.<ext>` in `dir`, creating the folder on
/// first use. A second image in the same second gets `_2`, then `_3`, and so on.
/// Returns the path relative to `dir`, with forward slashes, as notes link to it.
pub fn save(dir: &Path, now: NaiveDateTime, mime: &str, bytes: &[u8]) -> io::Result<String> {
    let folder = dir.join(IMAGES_DIR);
    fs::create_dir_all(&folder)?;
    let stem = now.format("%Y-%m-%d_%H-%M-%S").to_string();
    let ext = extension(mime);
    let mut n = 1;
    loop {
        let name = if n == 1 {
            format!("{stem}.{ext}")
        } else {
            format!("{stem}_{n}.{ext}")
        };
        // `create_new` never overwrites: a taken name moves on to the next number.
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(folder.join(&name))
        {
            Ok(mut file) => {
                file.write_all(bytes)?;
                return Ok(format!("{IMAGES_DIR}/{name}"));
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => n += 1,
            Err(e) => return Err(e),
        }
    }
}

/// The file an image path from a note points to, inside `dir`; `None` for a path that could
/// lead outside it (absolute, or with `..`). Joined part by part, so the path gets Windows
/// separators.
pub fn resolve(dir: &Path, path: &str) -> Option<PathBuf> {
    let path = Path::new(path);
    let inside = path.components().next().is_some()
        && path.components().all(|c| matches!(c, Component::Normal(_)));
    inside.then(|| {
        path.components()
            .fold(dir.to_path_buf(), |file, c| file.join(c))
    })
}

/// The image paths in a note's text, `![Image #N](path)` as notes store them, each once.
pub fn paths_in(text: &str) -> Vec<String> {
    let mut paths: Vec<String> = Vec::new();
    for (start, _) in text.match_indices("![Image #") {
        let rest = &text[start + "![Image #".len()..];
        let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        let Some(rest) = rest[digits..].strip_prefix("](") else {
            continue;
        };
        let Some(end) = rest.find(')') else {
            continue;
        };
        let path = &rest[..end];
        let plain = !path.is_empty() && !path.contains(|c: char| c == '(' || c.is_whitespace());
        if digits > 0 && plain && !paths.iter().any(|p| p == path) {
            paths.push(path.to_string());
        }
    }
    paths
}

/// The image files at `paths` that no notes file in `dir` links to any more; missing ones are
/// left out.
fn unused(dir: &Path, paths: &[String]) -> io::Result<Vec<PathBuf>> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let mut used = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == "md") && path.is_file() {
            used.extend(paths_in(&fs::read_to_string(path)?));
        }
    }
    Ok(paths
        .iter()
        .filter(|path| !used.contains(path))
        .filter_map(|path| resolve(dir, path))
        .filter(|file| file.is_file())
        .collect())
}

/// Moves the images at `paths` (from a deleted or edited note, or a deleted file) to the Recycle
/// Bin, unless a notes file in `dir` still links to them, like a note saved twice from the
/// input window's history.
pub fn trash_unused(dir: &Path, paths: &[String]) -> io::Result<()> {
    for file in unused(dir, paths)? {
        trash::delete(&file).map_err(io::Error::other)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn now() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 10, 7)
            .unwrap()
            .and_hms_opt(14, 32, 5)
            .unwrap()
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("quicklly-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn names_images_by_time_and_numbers_the_same_second() {
        let dir = temp_dir("images");
        assert_eq!(
            save(&dir, now(), "image/png", b"one").unwrap(),
            "images/2026-10-07_14-32-05.png"
        );
        assert_eq!(
            save(&dir, now(), "image/png", b"two").unwrap(),
            "images/2026-10-07_14-32-05_2.png"
        );
        assert_eq!(
            save(&dir, now(), "image/png", b"three").unwrap(),
            "images/2026-10-07_14-32-05_3.png"
        );
        let first = dir.join("images").join("2026-10-07_14-32-05.png");
        assert_eq!(fs::read(first).unwrap(), b"one");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn picks_the_extension_from_the_type() {
        let dir = temp_dir("image-types");
        assert_eq!(
            save(&dir, now(), "image/jpeg", b"").unwrap(),
            "images/2026-10-07_14-32-05.jpg"
        );
        // Another extension is another name, so no number is needed.
        assert_eq!(
            save(&dir, now(), "image/webp", b"").unwrap(),
            "images/2026-10-07_14-32-05.webp"
        );
        assert_eq!(
            save(&dir, now(), "image/x-unknown", b"").unwrap(),
            "images/2026-10-07_14-32-05.png"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn finds_the_image_paths_in_a_note() {
        assert_eq!(
            paths_in(
                "a ![Image #1](images/a.png) b ![Image #2](images/b.jpg) ![Image #1](images/a.png)"
            ),
            ["images/a.png", "images/b.jpg"]
        );
        assert!(
            paths_in("![Image #](images/a.png) ![Image #1]() [Image #1] ![Image #1](a b)")
                .is_empty()
        );
    }

    #[test]
    fn finds_images_no_note_links_to() {
        let dir = temp_dir("unused-images");
        let kept = save(&dir, now(), "image/png", b"kept").unwrap();
        let free = save(&dir, now(), "image/png", b"free").unwrap();
        fs::write(
            dir.join("todo.md"),
            format!("- 10:00 still here ![Image #1]({kept})\n"),
        )
        .unwrap();
        assert_eq!(
            unused(&dir, &[kept, free.clone(), "images/gone.png".into()]).unwrap(),
            [resolve(&dir, &free).unwrap()]
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn resolves_only_paths_inside_the_notes_folder() {
        let dir = Path::new("notes");
        assert_eq!(
            resolve(dir, "images/a.png"),
            Some(dir.join("images").join("a.png"))
        );
        assert_eq!(resolve(dir, "../secret.png"), None);
        assert_eq!(resolve(dir, "images/../../secret.png"), None);
        assert_eq!(resolve(dir, "./images/a.png"), None);
        assert_eq!(resolve(dir, "C:/Windows/a.png"), None);
        assert_eq!(resolve(dir, "/a.png"), None);
        assert_eq!(resolve(dir, ""), None);
    }
}
