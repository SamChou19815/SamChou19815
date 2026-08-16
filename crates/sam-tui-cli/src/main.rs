//! Native front-end for the Developer Sam TUI. Runs the same
//! backend-agnostic [`sam_tui::App`] that powers developersam.com/terminal,
//! so behavior can be exercised in a real terminal.

use anyhow::Result;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    MouseButton as CrosstermButton, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::tty::IsTty;
use ratatui::DefaultTerminal;
use sam_tui::{Action, App, Input, Key, Mods, MouseButton, MouseEv, MouseKind};

fn main() -> Result<()> {
    if !std::io::stdout().is_tty() {
        eprintln!("sam-tui: refusing to start because stdout is not a tty");
        return Ok(());
    }
    let mut terminal = ratatui::init();
    // Best-effort: the TUI is fully keyboard-driven if capture is rejected.
    let _ = execute!(std::io::stdout(), EnableMouseCapture);
    let result = run(&mut terminal);
    let _ = execute!(std::io::stdout(), DisableMouseCapture);
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal) -> Result<()> {
    let size = terminal.size()?;
    let mut app = App::new();
    app.resize(size.width, size.height);
    while !app.quit {
        terminal.draw(|frame| app.draw(frame))?;
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                let mods = Mods {
                    ctrl: key.modifiers.contains(KeyModifiers::CONTROL),
                    shift: key.modifiers.contains(KeyModifiers::SHIFT),
                    alt: key.modifiers.contains(KeyModifiers::ALT),
                };
                if let Some(key) = convert_key(key.code) {
                    app.handle(Input::Key { key, mods });
                }
            }
            Event::Mouse(mouse) => {
                if let Some(mouse) = convert_mouse(mouse.kind, mouse.column, mouse.row) {
                    app.handle(Input::Mouse(mouse));
                }
            }
            Event::Resize(width, height) => app.resize(width, height),
            _ => {}
        }
        for action in app.take_actions() {
            let Action::OpenUrl(url) = action;
            open_url(&url);
        }
    }
    Ok(())
}

fn convert_key(code: KeyCode) -> Option<Key> {
    let key = match code {
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Enter => Key::Enter,
        KeyCode::Esc => Key::Esc,
        KeyCode::Tab => Key::Tab,
        KeyCode::BackTab => Key::BackTab,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::Delete => Key::Delete,
        KeyCode::Char(character) => Key::Char(character),
        _ => return None,
    };
    Some(key)
}

fn convert_mouse(kind: MouseEventKind, col: u16, row: u16) -> Option<MouseEv> {
    let map_button = |button: CrosstermButton| match button {
        CrosstermButton::Left => MouseButton::Left,
        CrosstermButton::Middle => MouseButton::Middle,
        CrosstermButton::Right => MouseButton::Right,
    };
    let kind = match kind {
        MouseEventKind::Down(button) => MouseKind::Press(map_button(button)),
        MouseEventKind::Up(button) => MouseKind::Release(map_button(button)),
        MouseEventKind::ScrollUp => MouseKind::ScrollUp,
        MouseEventKind::ScrollDown => MouseKind::ScrollDown,
        MouseEventKind::Drag(_)
        | MouseEventKind::Moved
        | MouseEventKind::ScrollLeft
        | MouseEventKind::ScrollRight => {
            return None;
        }
    };
    Some(MouseEv { kind, col, row })
}

fn open_url(url: &str) {
    let result = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).spawn()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .args(["/c", "start", "", url])
            .spawn()
    } else {
        std::process::Command::new("xdg-open").arg(url).spawn()
    };
    if result.is_err() {
        // Still useful in a terminal: the user can copy the URL.
        eprintln!("sam-tui: could not open {url}");
    }
}
