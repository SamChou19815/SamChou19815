//! Tiny terminal-chart toolkit: horizontal bars, column charts, and ANSI color.
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

/// Width of the terminal in columns. Prefers the actual TTY size, honors an
/// explicit `COLUMNS` override, and falls back to 80 when neither is available
/// (e.g. piped output) so charts stay a sensible fixed size.
pub fn term_width() -> usize {
    terminal_size::terminal_size()
        .map(|(terminal_size::Width(w), _)| w as usize)
        .or_else(|| std::env::var("COLUMNS").ok().and_then(|c| c.parse().ok()))
        .filter(|&w| w > 0)
        .unwrap_or(80)
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

/// A multi-row column chart of `values`, `width` columns by `height` rows,
/// scaled between the series min and max. Values are resampled to `width`
/// columns via linear interpolation, and each column is drawn as a vertical bar
/// from the baseline up with eighth-of-a-row resolution. Returns `height` lines,
/// top row first; lines are right-trimmed so they carry no trailing blanks.
pub fn chart(values: &[f64], width: usize, height: usize) -> Vec<String> {
    if values.is_empty() || width == 0 || height == 0 {
        return Vec::new();
    }
    // Resample to `width` columns (linear interpolation between data points).
    let n = values.len();
    let denom = (width.saturating_sub(1)).max(1) as f64;
    let cols: Vec<f64> = (0..width)
        .map(|i| {
            if n == 1 {
                return values[0];
            }
            let pos = i as f64 * (n - 1) as f64 / denom;
            let lo = pos.floor() as usize;
            let hi = (lo + 1).min(n - 1);
            let frac = pos - lo as f64;
            values[lo] * (1.0 - frac) + values[hi] * frac
        })
        .collect();

    let finite: Vec<f64> = cols.iter().copied().filter(|v| v.is_finite()).collect();
    let min = finite.iter().copied().fold(f64::INFINITY, f64::min);
    let max = finite.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = max - min;
    // Height of each column expressed in eighths of a row (0..=height*8).
    let total = (height * 8) as f64;
    let eighths: Vec<usize> = cols
        .iter()
        .map(|&v| {
            if !v.is_finite() {
                0
            } else if span <= 0.0 {
                // Flat series: draw a single baseline row.
                8
            } else {
                (((v - min) / span) * total).round() as usize
            }
        })
        .collect();

    (0..height)
        .map(|out_row| {
            // Rows render top-down; distance from the baseline (bottom) row.
            let from_bottom = height - 1 - out_row;
            let base = from_bottom * 8;
            let line: String = eighths
                .iter()
                .map(|&e| {
                    let filled = e.saturating_sub(base);
                    if filled >= 8 {
                        '█'
                    } else if filled == 0 {
                        ' '
                    } else {
                        EIGHTHS[filled - 1]
                    }
                })
                .collect();
            line.trim_end().to_string()
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
    fn chart_has_height_rows_and_fills_bottom_up() {
        // Rising series: bottom row full across, top row only under the peak.
        let rows = chart(&[0.0, 1.0, 2.0, 3.0], 8, 3);
        assert_eq!(rows.len(), 3);
        // Last column is the max → full height, so every row ends in a block.
        for r in &rows {
            assert!(r.chars().last().is_some_and(|c| c == '█'));
        }
        // Bottom row spans the whole width; the top row is shorter (peak only).
        assert!(rows[2].chars().count() >= rows[0].chars().count());
        assert_eq!(chart(&[], 8, 3), Vec::<String>::new());
    }

    #[test]
    fn chart_flat_series_draws_a_baseline() {
        let rows = chart(&[5.0, 5.0, 5.0], 4, 3);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[2], "████"); // baseline row filled
        assert_eq!(rows[0], ""); // upper rows empty (trimmed)
    }
}
