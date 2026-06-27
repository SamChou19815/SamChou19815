//! Tiny terminal-chart toolkit: horizontal bars, sparklines, and ANSI color.
//!
//! These render the same data the Budget web app charts, using Unicode block
//! elements instead of SVG. Color is emitted only when stdout is a TTY so piped
//! output stays clean.

use std::io::IsTerminal;
use std::sync::OnceLock;

/// ANSI foreground colors used to echo the web app's palette.
#[derive(Clone, Copy)]
pub enum Color {
    Green,
    Red,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Dim,
}

impl Color {
    fn code(self) -> &'static str {
        match self {
            Color::Green => "32",
            Color::Red => "31",
            Color::Yellow => "33",
            Color::Blue => "34",
            Color::Magenta => "35",
            Color::Cyan => "36",
            Color::Dim => "90",
        }
    }
}

fn color_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        // Respect the de-facto NO_COLOR convention.
        std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal()
    })
}

/// Wrap `text` in an ANSI color, or return it unchanged when color is disabled.
pub fn paint(text: &str, color: Color) -> String {
    if color_enabled() {
        format!("\x1b[{}m{text}\x1b[0m", color.code())
    } else {
        text.to_string()
    }
}

/// Render `text` bold (used for headings), no-op when color is disabled.
pub fn bold(text: &str) -> String {
    if color_enabled() {
        format!("\x1b[1m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

/// Color `text` green when `value` is positive, red when negative, plain at zero.
/// Handy for net / gain-loss figures.
pub fn paint_signed(text: &str, value: f64) -> String {
    if value > 0.0 {
        paint(text, Color::Green)
    } else if value < 0.0 {
        paint(text, Color::Red)
    } else {
        text.to_string()
    }
}

const EIGHTHS: [char; 8] = ['▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];

/// A horizontal bar `width` cells wide representing `value / max`, with
/// eighth-of-a-cell resolution. Returns an empty bar for non-positive `max`.
pub fn hbar(value: f64, max: f64, width: usize) -> String {
    if !value.is_finite() || max <= 0.0 {
        return String::new();
    }
    let units = (value.max(0.0) / max).clamp(0.0, 1.0) * width as f64;
    let full = units.floor() as usize;
    let mut bar = "█".repeat(full);
    if full < width {
        let rem = ((units - full as f64) * 8.0).round() as usize;
        if rem > 0 {
            bar.push(EIGHTHS[rem - 1]);
        }
    }
    bar
}

const SPARKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// A one-line sparkline of `values`, scaled between their min and max.
pub fn sparkline(values: &[f64]) -> String {
    if values.is_empty() {
        return String::new();
    }
    let finite: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    let min = finite.iter().copied().fold(f64::INFINITY, f64::min);
    let max = finite.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = max - min;
    values
        .iter()
        .map(|&v| {
            if !v.is_finite() {
                return ' ';
            }
            if span <= 0.0 {
                return SPARKS[0];
            }
            let idx = (((v - min) / span) * (SPARKS.len() - 1) as f64).round() as usize;
            SPARKS[idx.min(SPARKS.len() - 1)]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hbar_scales_and_clamps() {
        assert_eq!(hbar(0.0, 10.0, 8), "");
        assert_eq!(hbar(10.0, 10.0, 8), "████████");
        assert_eq!(hbar(20.0, 10.0, 8), "████████"); // clamped
        assert_eq!(hbar(5.0, 10.0, 8), "████"); // exactly half
        assert_eq!(hbar(1.0, 0.0, 8), ""); // non-positive max
    }

    #[test]
    fn sparkline_spans_low_to_high() {
        assert_eq!(sparkline(&[1.0, 2.0, 3.0]), "▁▅█");
        assert_eq!(sparkline(&[5.0, 5.0]), "▁▁"); // flat series
        assert_eq!(sparkline(&[]), "");
    }
}
