#[derive(Clone, Copy)]
pub(crate) enum Key {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Esc,
    Tab,
    BackTab,
    Backspace,
    PageUp,
    PageDown,
    Home,
    End,
    Delete,
    Char(char),
}

#[derive(Clone, Copy)]
pub(crate) struct Mods {
    pub(crate) ctrl: bool,
    pub(crate) shift: bool,
    pub(crate) alt: bool,
}

pub(crate) fn map_key(event: &web_sys::KeyboardEvent) -> Option<(Key, Mods)> {
    let mods = Mods {
        ctrl: event.ctrl_key(),
        shift: event.shift_key(),
        alt: event.alt_key(),
    };
    let key = match event.key().as_str() {
        "ArrowUp" => Key::Up,
        "ArrowDown" => Key::Down,
        "ArrowLeft" => Key::Left,
        "ArrowRight" => Key::Right,
        "Enter" => Key::Enter,
        "Escape" => Key::Esc,
        "Tab" if mods.shift => Key::BackTab,
        "Tab" => Key::Tab,
        "Backspace" => Key::Backspace,
        "Delete" => Key::Delete,
        "PageUp" => Key::PageUp,
        "PageDown" => Key::PageDown,
        "Home" => Key::Home,
        "End" => Key::End,
        other => {
            let mut chars = other.chars();
            let only = chars.next()?;
            if chars.next().is_some() {
                return None;
            }
            Key::Char(only)
        }
    };
    Some((key, mods))
}

#[derive(Clone, Copy)]
pub(crate) enum Keymap {
    Prompt,
    App,
}

/// Only claim the Ctrl combos we handle, so browser shortcuts (reload, find, ...) keep working.
pub(crate) fn claims(keymap: Keymap, key: Key, mods: Mods) -> bool {
    if mods.alt {
        return false;
    }
    if !mods.ctrl {
        return true;
    }
    match keymap {
        Keymap::Prompt => matches!(key, Key::Char('c' | 'l' | 'a' | 'e' | 'u')),
        Keymap::App => matches!(key, Key::Char('c' | 'd')),
    }
}
