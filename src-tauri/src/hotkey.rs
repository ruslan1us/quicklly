use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

/// Default global hotkey: Ctrl+Alt+N.
pub fn default() -> Shortcut {
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyN)
}

/// Parses a hotkey such as `Ctrl+Alt+KeyN` or just `F9` (optional modifiers plus a
/// `KeyboardEvent.code`).
pub fn parse(hotkey: &str) -> Result<Shortcut, String> {
    hotkey
        .parse()
        .map_err(|_| format!("\"{hotkey}\" is not a supported hotkey."))
}

/// The names of Ctrl, Alt, Shift and Super in a label, as the system calls them, and what goes
/// between the parts: `Ctrl+Alt+N`, or `⌃⌥N` on macOS.
#[cfg(windows)]
const MODIFIER_NAMES: [&str; 4] = ["Ctrl", "Alt", "Shift", "Win"];
#[cfg(target_os = "macos")]
const MODIFIER_NAMES: [&str; 4] = ["⌃", "⌥", "⇧", "⌘"];
#[cfg(not(any(windows, target_os = "macos")))]
const MODIFIER_NAMES: [&str; 4] = ["Ctrl", "Alt", "Shift", "Super"];
const SEPARATOR: &str = if cfg!(target_os = "macos") { "" } else { "+" };

/// Human-readable form, e.g. `Ctrl+Alt+N` (`⌃⌥N` on macOS).
pub fn label(shortcut: &Shortcut) -> String {
    let mut parts = Vec::new();
    let modifiers = [
        Modifiers::CONTROL,
        Modifiers::ALT,
        Modifiers::SHIFT,
        Modifiers::SUPER,
    ];
    for (modifier, name) in modifiers.into_iter().zip(MODIFIER_NAMES) {
        if shortcut.mods.contains(modifier) {
            parts.push(name.to_string());
        }
    }
    parts.push(key_label(shortcut.key));
    parts.join(SEPARATOR)
}

fn key_label(key: Code) -> String {
    let symbol = match key {
        Code::Backquote => "`",
        Code::Minus => "-",
        Code::Equal => "=",
        Code::BracketLeft => "[",
        Code::BracketRight => "]",
        Code::Backslash => "\\",
        Code::Semicolon => ";",
        Code::Quote => "'",
        Code::Comma => ",",
        Code::Period => ".",
        Code::Slash => "/",
        _ => "",
    };
    if !symbol.is_empty() {
        return symbol.into();
    }
    let name = key.to_string();
    for prefix in ["Key", "Digit", "Arrow"] {
        if let Some(rest) = name.strip_prefix(prefix) {
            return rest.into();
        }
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hotkeys in their stored form and their labels on this system.
    #[cfg(windows)]
    const LABELS: [(&str, &str); 6] = [
        ("Ctrl+Alt+KeyN", "Ctrl+Alt+N"),
        ("Shift+Super+Digit1", "Shift+Win+1"),
        ("Ctrl+Slash", "Ctrl+/"),
        ("Alt+F5", "Alt+F5"),
        ("Ctrl+ArrowUp", "Ctrl+Up"),
        ("Shift+KeyN", "Shift+N"),
    ];
    #[cfg(target_os = "macos")]
    const LABELS: [(&str, &str); 6] = [
        ("Ctrl+Alt+KeyN", "⌃⌥N"),
        ("Shift+Super+Digit1", "⇧⌘1"),
        ("Ctrl+Slash", "⌃/"),
        ("Alt+F5", "⌥F5"),
        ("Ctrl+ArrowUp", "⌃Up"),
        ("Shift+KeyN", "⇧N"),
    ];
    #[cfg(not(any(windows, target_os = "macos")))]
    const LABELS: [(&str, &str); 6] = [
        ("Ctrl+Alt+KeyN", "Ctrl+Alt+N"),
        ("Shift+Super+Digit1", "Shift+Super+1"),
        ("Ctrl+Slash", "Ctrl+/"),
        ("Alt+F5", "Alt+F5"),
        ("Ctrl+ArrowUp", "Ctrl+Up"),
        ("Shift+KeyN", "Shift+N"),
    ];

    #[test]
    fn parses_and_labels_hotkeys() {
        assert_eq!(parse("Ctrl+Alt+KeyN").unwrap(), default());
        for (hotkey, shown) in LABELS {
            assert_eq!(label(&parse(hotkey).unwrap()), shown, "{hotkey}");
        }
    }

    #[test]
    fn stored_form_round_trips() {
        let shortcut = parse("Ctrl+Shift+Super+KeyQ").unwrap();
        assert_eq!(parse(&shortcut.into_string()).unwrap(), shortcut);
    }

    #[test]
    fn accepts_any_supported_key() {
        assert_eq!(label(&parse("F9").unwrap()), "F9");
        assert_eq!(label(&parse("KeyN").unwrap()), "N");
        assert!(parse("Ctrl+Nope").is_err());
    }
}
