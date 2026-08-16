//! Serializes a rendered ratatui buffer into a byte stream for terminals the
//! app does not own directly: xterm.js on the web (via the wasm FFI).
//!
//! Layout of the stream:
//!
//! ```text
//! [ANSI frame bytes …][0x00][u32 LE link count]
//!   { u16 LE x, y, w, h, u32 LE url length, url bytes } × count
//! ```

use crate::LinkRegion;
use ratatui_core::buffer::Buffer;
use ratatui_core::style::{Color, Modifier, Style};

pub fn serialize_frame(buffer: &Buffer, links: &[LinkRegion]) -> Vec<u8> {
    let mut out = String::with_capacity((buffer.area.area() as usize * 6).max(1024));
    out.push_str("\x1b[?25l"); // hide the cursor; the app never moves it
    let area = buffer.area;
    let width = area.width as usize;
    for y in 0..area.height as usize {
        out.push_str(&format!(
            "\x1b[{};{}H",
            area.y as usize + y + 1,
            area.x as usize + 1
        ));
        let row = &buffer.content[y * width..(y + 1) * width];
        let mut last_style: Option<Style> = None;
        for cell in row {
            let style = cell.style();
            if last_style != Some(style) {
                push_sgr(&mut out, style);
                last_style = Some(style);
            }
            out.push_str(cell.symbol());
        }
        out.push_str("\x1b[m\x1b[K");
    }
    out.push_str("\x1b[m");

    let mut bytes = out.into_bytes();
    bytes.push(0);
    let (count, link_bytes) = serialize_links(links);
    bytes.extend_from_slice(&count.to_le_bytes());
    bytes.extend_from_slice(&link_bytes);
    bytes
}

fn serialize_links(links: &[LinkRegion]) -> (u32, Vec<u8>) {
    let mut out = Vec::new();
    for link in links {
        let url = link.url.as_bytes();
        out.extend_from_slice(&link.rect.x.to_le_bytes());
        out.extend_from_slice(&link.rect.y.to_le_bytes());
        out.extend_from_slice(&link.rect.width.to_le_bytes());
        out.extend_from_slice(&link.rect.height.to_le_bytes());
        out.extend_from_slice(&(url.len() as u32).to_le_bytes());
        out.extend_from_slice(url);
    }
    (links.len() as u32, out)
}

/// One full SGR escape for a style (reset + fg + bg + modifiers); used by the
/// shell, which emits styled strings directly instead of cell buffers.
pub(crate) fn style_sgr(style: Style) -> String {
    let mut out = String::new();
    push_sgr(&mut out, style);
    out
}

fn push_sgr(out: &mut String, style: Style) {
    let mut parts = vec![String::from("0")];
    if let Some(fg) = style.fg {
        parts.push(color_code(fg, 30));
    }
    if let Some(bg) = style.bg {
        parts.push(color_code(bg, 40));
    }
    let modifiers = style.add_modifier;
    if modifiers.intersects(Modifier::BOLD) {
        parts.push(String::from("1"));
    }
    if modifiers.intersects(Modifier::DIM) {
        parts.push(String::from("2"));
    }
    if modifiers.intersects(Modifier::ITALIC) {
        parts.push(String::from("3"));
    }
    if modifiers.intersects(Modifier::UNDERLINED) {
        parts.push(String::from("4"));
    }
    if modifiers.intersects(Modifier::REVERSED) {
        parts.push(String::from("7"));
    }
    if modifiers.intersects(Modifier::CROSSED_OUT) {
        parts.push(String::from("9"));
    }
    out.push_str("\x1b[");
    out.push_str(&parts.join(";"));
    out.push('m');
}

fn color_code(color: Color, base: u8) -> String {
    let layer = if base == 40 { 48 } else { 38 };
    let named = match color {
        Color::Black => 0,
        Color::Red => 1,
        Color::Green => 2,
        Color::Yellow => 3,
        Color::Blue => 4,
        Color::Magenta => 5,
        Color::Cyan => 6,
        Color::Gray => 7,
        Color::DarkGray => 60,
        Color::LightRed => 61,
        Color::LightGreen => 62,
        Color::LightYellow => 63,
        Color::LightBlue => 64,
        Color::LightMagenta => 65,
        Color::LightCyan => 66,
        Color::White => 67,
        Color::Indexed(index) => return format!("{layer};5;{index}"),
        Color::Rgb(r, g, b) => return format!("{layer};2;{r};{g};{b}"),
        Color::Reset => return String::from(if base == 40 { "49" } else { "39" }),
    };
    (base + named).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui_core::buffer::Cell;
    use ratatui_core::layout::Rect;

    #[test]
    fn frame_contains_cursor_moves_and_link_table() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 2));
        buffer.content[0] = Cell::new("H");
        buffer.content[1] = Cell::new("i");
        let links = vec![LinkRegion {
            rect: Rect::new(2, 1, 4, 1),
            url: String::from("https://developersam.com"),
        }];
        let bytes = serialize_frame(&buffer, &links);
        let split = bytes.split(|byte| *byte == 0).next().unwrap().to_vec();
        let frame = String::from_utf8(split).unwrap();
        assert!(frame.contains("\x1b[1;1H"));
        assert!(frame.contains("Hi"));
        assert!(frame.contains("\x1b[K"));

        let separator = bytes.iter().position(|byte| *byte == 0).unwrap();
        let count = u32::from_le_bytes(bytes[separator + 1..separator + 5].try_into().unwrap());
        assert_eq!(count, 1);
        let url_len = u32::from_le_bytes(bytes[separator + 13..separator + 17].try_into().unwrap());
        assert_eq!(url_len as usize, "https://developersam.com".len());
        let url = &bytes[separator + 17..separator + 17 + url_len as usize];
        assert_eq!(url, b"https://developersam.com");
    }
}
