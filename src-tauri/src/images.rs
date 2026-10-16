use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use chrono::NaiveDateTime;
use trash::TrashItem;

use crate::archive::ARCHIVE_DIR;
use crate::reader;

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

/// `text` with its images, saved as `images/…` from the notes folder, linked from the notes
/// file `name` instead: one `../` for each folder it is in, so `work/quicklly.md` links
/// `../images/…`, as Obsidian expects.
pub fn link_from(text: &str, name: &str) -> String {
    let up = "../".repeat(name.matches('/').count());
    text.replace(&format!("]({IMAGES_DIR}/"), &format!("]({up}{IMAGES_DIR}/"))
}

/// The image that a note in the notes file `file` (a path from the notes folder) links to as
/// `path` (from the file's folder), as the parts of its path from the notes folder; `None` for
/// a path that could lead outside it (absolute, or with too many `..`).
fn from_notes_folder(file: &str, path: &str) -> Option<Vec<String>> {
    let mut parts: Vec<String> = Vec::new();
    for c in Path::new(file).parent()?.components() {
        let Component::Normal(part) = c else {
            return None;
        };
        parts.push(part.to_string_lossy().into_owned());
    }
    for c in Path::new(path).components() {
        match c {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::ParentDir => {
                parts.pop()?;
            }
            _ => return None,
        }
    }
    // An image, not the folder it is in.
    let names_a_file = matches!(
        Path::new(path).components().next_back(),
        Some(Component::Normal(_))
    );
    names_a_file.then_some(parts)
}

/// The file an image path from a note in the notes file `file` points to, inside `dir`; `None`
/// for a path that could lead outside it. Joined part by part, so the path gets Windows
/// separators.
pub fn resolve(dir: &Path, file: &str, path: &str) -> Option<PathBuf> {
    let parts = from_notes_folder(file, path)?;
    Some(
        parts
            .iter()
            .fold(dir.to_path_buf(), |dir, part| dir.join(part)),
    )
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

/// A note's text with each image as notes store it, `![Image #N](path)`, as its label
/// `[Image #N]`, the way the windows show it (`markdownNodes` in `images.ts`).
pub fn labels(text: &str) -> String {
    let mut shown = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("![Image #") {
        let after = &rest[start + "![Image #".len()..];
        let digits = after.len() - after.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        let path_end = after[digits..].strip_prefix("](").and_then(|path| {
            let end = path.find(')')?;
            let plain = end > 0 && !path[..end].contains(|c: char| c == '(' || c.is_whitespace());
            plain.then_some(end)
        });
        match path_end {
            Some(end) if digits > 0 => {
                shown.push_str(&rest[..start]);
                shown.push_str(&format!("[Image #{}]", &after[..digits]));
                rest = &after[digits + "](".len() + end + ")".len()..];
            }
            // Not an image: keep the `!` and look further.
            _ => {
                shown.push_str(&rest[..=start]);
                rest = &rest[start + 1..];
            }
        }
    }
    shown.push_str(rest);
    shown
}

/// The image files at `paths` (linked from the notes file `file`) that no notes file in `dir`
/// or its archive, in any folder, links to any more; missing ones are left out. Fails, naming
/// the file, when a notes file can't be read.
fn unused(dir: &Path, file: &str, paths: &[String]) -> io::Result<Vec<PathBuf>> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let mut used = Vec::new();
    // Each link is resolved from the folder of the file it is in.
    let archive = (dir.join(ARCHIVE_DIR), format!("{ARCHIVE_DIR}/"));
    for (folder, prefix) in [(dir.to_path_buf(), String::new()), archive] {
        for note_file in reader::list_files(&folder)? {
            let name = format!("{prefix}{}", note_file.name);
            // A file that can't be read might link to the images: fail rather than skip it.
            let content = reader::read_named(dir, &name)?;
            used.extend(
                paths_in(&content)
                    .iter()
                    .filter_map(|path| from_notes_folder(&name, path)),
            );
        }
    }
    Ok(paths
        .iter()
        .filter_map(|path| from_notes_folder(file, path))
        // Only pasted images, right in `images/`: a hand-written link to anything else, a
        // notes file say, never sends it to the Recycle Bin.
        .filter(|parts| parts.len() == 2 && parts[0] == IMAGES_DIR)
        .filter(|parts| !used.contains(parts))
        .map(|parts| {
            parts
                .iter()
                .fold(dir.to_path_buf(), |dir, part| dir.join(part))
        })
        .filter(|image| image.is_file())
        .collect())
}

/// Moves the images at `paths` (from a deleted or edited note, or a deleted file, in the notes
/// file `file`) to the Recycle Bin, unless a notes file in `dir` still links to them, like a
/// note saved twice from the input window's history, or an archived note. Returns where the
/// images were, as the Recycle Bin remembers it, to bring them back with [`restore`]: only the
/// ones actually moved, as an image that can't be moved is only logged.
pub fn trash_unused(dir: &Path, file: &str, paths: &[String]) -> io::Result<Vec<PathBuf>> {
    Ok(trash_each(unused(dir, file, paths)?, to_recycle_bin))
}

/// The file name of an image pasted into a window, from its path `images/<name>` as the window
/// holds it; `None` for any other path, so nothing but a file right in `images/` is ever taken
/// for a pasted image.
fn pasted_name(path: &str) -> Option<&str> {
    let name = path.strip_prefix(IMAGES_DIR)?.strip_prefix('/')?;
    let plain = !name.is_empty() && !name.starts_with('.') && !name.contains(['/', '\\', ':']);
    plain.then_some(name)
}

/// Of the images pasted into a window at `paths` (`images/<name>`, from the notes folder), the
/// files that nothing mentions: no notes file in `dir` or its archive, in any folder, and none
/// of the texts in `kept` has the image's file name in it, ignoring case and however it links
/// to it. Missing images are left out. Fails, naming the file, when a notes file can't be read.
fn unsaved(dir: &Path, paths: &[String], kept: &[String]) -> io::Result<Vec<PathBuf>> {
    fn drop_mentioned(left: &mut Vec<&str>, text: &str) {
        let text = text.to_lowercase();
        left.retain(|name| !text.contains(&name.to_lowercase()));
    }
    let mut left: Vec<&str> = paths.iter().filter_map(|path| pasted_name(path)).collect();
    for text in kept {
        drop_mentioned(&mut left, text);
    }
    let archive = (dir.join(ARCHIVE_DIR), format!("{ARCHIVE_DIR}/"));
    for (folder, prefix) in [(dir.to_path_buf(), String::new()), archive] {
        for note_file in reader::list_files(&folder)? {
            if left.is_empty() {
                return Ok(Vec::new());
            }
            // A file that can't be read might mention the images: fail rather than skip it.
            let content = reader::read_named(dir, &format!("{prefix}{}", note_file.name))?;
            drop_mentioned(&mut left, &content);
        }
    }
    Ok(left
        .into_iter()
        .map(|name| dir.join(IMAGES_DIR).join(name))
        .filter(|image| image.is_file())
        .collect())
}

/// Moves the images pasted into a window at `paths` (`images/<name>`), which the window let go
/// of without saving them, to the Recycle Bin, unless a notes file in `dir` or one of the texts
/// in `kept` (the Pad draft, the input window's history) still mentions them.
pub fn trash_unsaved(dir: &Path, paths: &[String], kept: &[String]) -> io::Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    trash_each(unsaved(dir, paths, kept)?, to_recycle_bin);
    Ok(())
}

/// Moves a file to the Recycle Bin.
fn to_recycle_bin(path: &Path) -> io::Result<()> {
    trash::delete(path).map_err(io::Error::other)
}

/// Moves each of `images` away with `delete` (to the Recycle Bin), going on after one that
/// fails, which is only logged. Returns where the moved ones were, as [`full_path`] gives it.
fn trash_each(images: Vec<PathBuf>, delete: impl Fn(&Path) -> io::Result<()>) -> Vec<PathBuf> {
    let mut trashed = Vec::new();
    for image in images {
        match full_path(&image).and_then(|full| delete(&image).map(|()| full)) {
            Ok(full) => trashed.push(full),
            Err(e) => eprintln!("Couldn't delete {}: {e}", image.display()),
        }
    }
    trashed
}

/// The whole path of an existing file, as `trash` gives it to the Recycle Bin: canonical, but
/// without the `\\?\` in front that Windows adds to it.
fn full_path(path: &Path) -> io::Result<PathBuf> {
    let full = fs::canonicalize(path)?;
    let full = full.to_string_lossy();
    Ok(PathBuf::from(full.strip_prefix(r"\\?\").unwrap_or(&full)))
}

/// Of the items in the Recycle Bin, the ones that bring back the images that were at `paths`:
/// for each, the one moved there from that path last. Images no longer in it are left out.
fn to_restore(bin: &[TrashItem], paths: &[PathBuf]) -> Vec<TrashItem> {
    paths
        .iter()
        .filter_map(|path| {
            bin.iter()
                .filter(|item| item.original_path() == *path)
                .max_by_key(|item| item.time_deleted)
                .cloned()
        })
        .collect()
}

/// Brings the images that were at `paths` back from the Recycle Bin, where [`trash_unused`]
/// moved them, to undo a change in the reader. Returns how many of them couldn't be brought
/// back: no longer in the Recycle Bin, or another file in their place.
pub fn restore(paths: &[PathBuf]) -> usize {
    if paths.is_empty() {
        return 0;
    }
    let Ok(bin) = trash::os_limited::list() else {
        return paths.len();
    };
    let restored = to_restore(&bin, paths)
        .into_iter()
        .map(|item| trash::os_limited::restore_all([item]))
        .filter(Result::is_ok)
        .count();
    paths.len() - restored
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
    fn shows_images_as_their_labels() {
        assert_eq!(
            labels("a ![Image #1](images/a.png) b\n![Image #12](../images/b.jpg)!"),
            "a [Image #1] b\n[Image #12]!"
        );
        let not_images =
            "![Image #](images/a.png) ![Image #1]() [Image #1] ![Image #1](a b) ![x](y)";
        assert_eq!(labels(not_images), not_images);
    }

    #[test]
    fn finds_images_no_note_links_to() {
        let dir = temp_dir("unused-images");
        let kept = save(&dir, now(), "image/png", b"kept").unwrap();
        let free = save(&dir, now(), "image/png", b"free").unwrap();
        fs::write(
            dir.join("todo.md"),
            format!(
                "- 10:00 still here ![Image #1]({kept})
"
            ),
        )
        .unwrap();
        assert_eq!(
            unused(
                &dir,
                "todo.md",
                &[kept, free.clone(), "images/gone.png".into()]
            )
            .unwrap(),
            [resolve(&dir, "todo.md", &free).unwrap()]
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn keeps_images_an_archived_note_links_to() {
        let dir = temp_dir("archived-images");
        let archived = save(&dir, now(), "image/png", b"archived").unwrap();
        let nested = save(&dir, now(), "image/png", b"nested").unwrap();
        let free = save(&dir, now(), "image/png", b"free").unwrap();
        fs::create_dir_all(dir.join("archive").join("work")).unwrap();
        fs::write(
            dir.join("archive").join("todo.md"),
            format!(
                "- [x] 10:00 old ![Image #1](../{archived}) ✅ 2026-10-01
"
            ),
        )
        .unwrap();
        fs::write(
            dir.join("archive").join("work").join("q.md"),
            format!(
                "- [x] 10:00 old ![Image #1](../../{nested}) ✅ 2026-10-01
"
            ),
        )
        .unwrap();
        assert_eq!(
            unused(&dir, "todo.md", &[archived, nested, free.clone()]).unwrap(),
            [resolve(&dir, "todo.md", &free).unwrap()]
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn keeps_images_a_nested_note_links_to() {
        let dir = temp_dir("nested-images");
        let kept = save(&dir, now(), "image/png", b"kept").unwrap();
        let free = save(&dir, now(), "image/png", b"free").unwrap();
        fs::create_dir_all(dir.join("work")).unwrap();
        fs::write(
            dir.join("work").join("quicklly.md"),
            format!(
                "- 10:00 here ![Image #1](../{kept})
"
            ),
        )
        .unwrap();
        // A note deleted from another nested file linked them from its own folder.
        let paths = [format!("../../{kept}"), format!("../../{free}")];
        assert_eq!(
            unused(&dir, "a/b/c.md", &paths).unwrap(),
            [dir.join("images").join(&free["images/".len()..])]
        );
        // A link to a notes file or to an image outside `images/` is never taken for one.
        fs::write(dir.join("inbox.md"), "").unwrap();
        fs::create_dir_all(dir.join("pics")).unwrap();
        fs::write(dir.join("pics").join("a.png"), "").unwrap();
        let other = ["inbox.md".to_string(), "pics/a.png".to_string()];
        assert!(unused(&dir, "todo.md", &other).unwrap().is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn finds_pasted_images_nothing_mentions() {
        let dir = temp_dir("unsaved-images");
        let saved = save(&dir, now(), "image/png", b"saved").unwrap();
        let tagged = save(&dir, now(), "image/png", b"tagged").unwrap();
        let archived = save(&dir, now(), "image/png", b"archived").unwrap();
        let embedded = save(&dir, now(), "image/png", b"embedded").unwrap();
        let drafted = save(&dir, now(), "image/png", b"drafted").unwrap();
        let free = save(&dir, now(), "image/png", b"free").unwrap();
        let name = |path: &str| path["images/".len()..].to_string();
        fs::create_dir_all(dir.join("work")).unwrap();
        fs::create_dir_all(dir.join("archive")).unwrap();
        fs::write(
            dir.join("2026-10-07.md"),
            format!("- 14:33 ![Image #1]({saved})\n"),
        )
        .unwrap();
        fs::write(
            dir.join("work").join("q.md"),
            format!("- 14:33 ![Image #1](../{tagged})\n"),
        )
        .unwrap();
        fs::write(
            dir.join("archive").join("todo.md"),
            format!("- [x] 14:33 ![Image #1](../{archived}) ✅ 2026-10-07\n"),
        )
        .unwrap();
        // Linked by hand, in another way, with the name in other case: still mentioned.
        let shouted = name(&embedded).to_uppercase();
        fs::write(dir.join("My Notes.md"), format!("![[{shouted}]]\n")).unwrap();
        let draft = format!("more ![Image #1]({drafted})");
        let paths = [
            saved,
            tagged,
            archived,
            embedded,
            drafted,
            free.clone(),
            "images/gone.png".into(),
        ];
        assert_eq!(
            unsaved(&dir, &paths, &[draft]).unwrap(),
            [dir.join("images").join(name(&free))]
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fails_naming_a_notes_file_it_cant_read() {
        let dir = temp_dir("images-unreadable");
        let image = save(&dir, now(), "image/png", b"").unwrap();
        fs::create_dir_all(dir.join("archive")).unwrap();
        fs::write(dir.join("todo.md"), "- 09:00 no images\n").unwrap();
        // Saved by hand in Windows-1251, say: not UTF-8, and it may link to the image.
        let ansi = [
            b"- [x] 09:00 \xec\xee\xeb\xee\xea\xee ".as_slice(),
            format!("![Image #1](../{image})\n").as_bytes(),
        ]
        .concat();
        fs::write(dir.join("archive").join("todo.md"), ansi).unwrap();

        let paths = [image];
        let err = unused(&dir, "todo.md", &paths).unwrap_err();
        assert!(
            err.to_string()
                .starts_with("couldn't read archive/todo.md: "),
            "{err}"
        );
        let err = unsaved(&dir, &paths, &[]).unwrap_err();
        assert!(
            err.to_string()
                .starts_with("couldn't read archive/todo.md: "),
            "{err}"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn takes_only_files_right_in_images_for_pasted_images() {
        let dir = temp_dir("pasted-only");
        fs::create_dir_all(dir.join("images").join("sub")).unwrap();
        for file in ["todo.md", "x.png", "images/sub/x.png", "images/.hidden.png"] {
            fs::write(dir.join(file), "").unwrap();
        }
        let paths = [
            "todo.md",
            "images/../todo.md",
            "images/../x.png",
            "../x.png",
            "images/sub/x.png",
            "images/sub",
            "images/.hidden.png",
            "images/",
            "images",
            "C:/x.png",
            "images\\..\\x.png",
            "imagesx.png",
        ]
        .map(String::from);
        assert!(unsaved(&dir, &paths, &[]).unwrap().is_empty());
        assert_eq!(
            pasted_name("images/2026-10-07_14-32-05_2.png"),
            Some("2026-10-07_14-32-05_2.png")
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn gives_the_whole_path_of_an_image() {
        let dir = temp_dir("full-path");
        let image = save(&dir, now(), "image/png", b"").unwrap();
        let full = full_path(&resolve(&dir, "todo.md", &image).unwrap()).unwrap();
        assert!(full.is_absolute());
        assert!(!full.to_string_lossy().starts_with(r"\\?\"));
        assert!(full.ends_with(&image));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn goes_on_deleting_images_after_one_fails() {
        let dir = temp_dir("trash-each");
        fs::create_dir_all(&dir).unwrap();
        let images: Vec<PathBuf> = ["a.png", "b.png", "c.png"]
            .iter()
            .map(|name| dir.join(name))
            .collect();
        for image in &images {
            fs::write(image, b"image").unwrap();
        }
        let missing = dir.join("missing.png");
        let tried = std::cell::RefCell::new(Vec::new());
        let trashed = trash_each(
            vec![
                images[0].clone(),
                images[1].clone(),
                missing,
                images[2].clone(),
            ],
            |image| {
                tried.borrow_mut().push(image.to_path_buf());
                if image.ends_with("b.png") {
                    Err(io::Error::other("locked"))
                } else {
                    Ok(())
                }
            },
        );
        assert_eq!(
            trashed,
            vec![
                full_path(&images[0]).unwrap(),
                full_path(&images[2]).unwrap()
            ]
        );
        assert_eq!(tried.into_inner(), images);
        fs::remove_dir_all(&dir).unwrap();
    }

    fn in_bin(id: &str, path: &str, time_deleted: i64) -> TrashItem {
        let path = Path::new(path);
        TrashItem {
            id: id.into(),
            name: path.file_name().unwrap().into(),
            original_parent: path.parent().unwrap().into(),
            time_deleted,
        }
    }

    #[test]
    fn finds_the_images_to_bring_back_in_the_recycle_bin() {
        let bin = [
            in_bin("old", "notes/images/a.png", 1),
            in_bin("new", "notes/images/a.png", 2),
            in_bin("elsewhere", "other/images/a.png", 3),
            in_bin("b", "notes/images/b.png", 1),
        ];
        let paths = [
            "notes/images/a.png",
            "notes/images/gone.png",
            "notes/images/b.png",
        ]
        .map(PathBuf::from);
        let found: Vec<_> = to_restore(&bin, &paths)
            .into_iter()
            .map(|item| item.id)
            .collect();
        assert_eq!(found, ["new", "b"]);
    }

    #[test]
    fn links_images_from_the_file_folder() {
        let text = "a ![Image #1](images/a.png) b ![Image #2](images/b.png)";
        assert_eq!(link_from(text, "todo.md"), text);
        assert_eq!(
            link_from(text, "work/quicklly.md"),
            "a ![Image #1](../images/a.png) b ![Image #2](../images/b.png)"
        );
        assert_eq!(
            link_from("![Image #1](images/a.png)", "a/b/c.md"),
            "![Image #1](../../images/a.png)"
        );
        // Already linked from the folder: left as it is.
        assert_eq!(
            link_from("![Image #1](../images/a.png)", "work/q.md"),
            "![Image #1](../images/a.png)"
        );
    }

    #[test]
    fn resolves_only_paths_inside_the_notes_folder() {
        let dir = Path::new("notes");
        let image = Some(dir.join("images").join("a.png"));
        assert_eq!(resolve(dir, "todo.md", "images/a.png"), image);
        assert_eq!(resolve(dir, "work/q.md", "../images/a.png"), image);
        assert_eq!(resolve(dir, "a/b/c.md", "../../images/a.png"), image);
        assert_eq!(
            resolve(dir, "archive/work/q.md", "../../images/a.png"),
            image
        );
        assert_eq!(
            resolve(dir, "work/q.md", "images/../../images/a.png"),
            image
        );
        assert_eq!(resolve(dir, "todo.md", "../secret.png"), None);
        assert_eq!(resolve(dir, "work/q.md", "../../secret.png"), None);
        assert_eq!(resolve(dir, "todo.md", "images/../../secret.png"), None);
        assert_eq!(resolve(dir, "../x/q.md", "images/a.png"), None);
        assert_eq!(resolve(dir, "C:/x/q.md", "images/a.png"), None);
        assert_eq!(resolve(dir, "todo.md", "./images/a.png"), None);
        assert_eq!(resolve(dir, "todo.md", "C:/Windows/a.png"), None);
        assert_eq!(resolve(dir, "todo.md", "/a.png"), None);
        assert_eq!(resolve(dir, "todo.md", ""), None);
        assert_eq!(resolve(dir, "work/q.md", ".."), None);
        assert_eq!(resolve(dir, "work/q.md", "a/.."), None);
    }
}
