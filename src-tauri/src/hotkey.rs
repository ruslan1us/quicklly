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

/// Human-readable form, e.g. `Ctrl+Alt+N`.
pub fn label(shortcut: &Shortcut) -> String {
    let mut parts = Vec::new();
    for (modifier, name) in [
        (Modifiers::CONTROL, "Ctrl"),
        (Modifiers::ALT, "Alt"),
        (Modifiers::SHIFT, "Shift"),
        (Modifiers::SUPER, "Win"),
    ] {
        if shortcut.mods.contains(modifier) {
            parts.push(name.to_string());
        }
    }
    parts.push(key_label(shortcut.key));
    parts.join("+")
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

    #[test]
    fn parses_and_labels_hotkeys() {
        assert_eq!(parse("Ctrl+Alt+KeyN").unwrap(), default());
        assert_eq!(label(&default()), "Ctrl+Alt+N");
        assert_eq!(label(&parse("Shift+Super+Digit1").unwrap()), "Shift+Win+1");
        assert_eq!(label(&parse("Ctrl+Slash").unwrap()), "Ctrl+/");
        assert_eq!(label(&parse("Alt+F5").unwrap()), "Alt+F5");
        assert_eq!(label(&parse("Ctrl+ArrowUp").unwrap()), "Ctrl+Up");
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
        assert_eq!(label(&parse("Shift+KeyN").unwrap()), "Shift+N");
        assert!(parse("Ctrl+Nope").is_err());
    }
}
