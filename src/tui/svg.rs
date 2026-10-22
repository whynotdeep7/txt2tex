//! Renders a terminal buffer as an SVG "screenshot".
//!
//! This is how the README's picture of the editor is made: draw a real frame
//! to the in-memory backend, then write it out cell by cell. The image can
//! never drift from the program, because it *is* the program.
//!
//! Regenerate it with:
//!
//! ```text
//! cargo test readme_screenshot -- --ignored
//! ```

use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};
use std::fmt::Write;

/// One Dark, roughly — a palette most people's terminals resemble.
const BACKGROUND: &str = "#282c34";
const FOREGROUND: &str = "#d7dae0";
const BORDER: &str = "#4b5263";

const CELL_W: f32 = 9.0;
const CELL_H: f32 = 20.0;
const FONT_SIZE: f32 = 15.0;
/// Room above the grid for the window's title bar.
const CHROME: f32 = 36.0;
const PAD: f32 = 14.0;

fn colour(c: Color) -> Option<&'static str> {
    Some(match c {
        Color::Reset => return None,
        Color::Black => BACKGROUND,
        Color::Red | Color::LightRed => "#e06c75",
        Color::Green | Color::LightGreen => "#98c379",
        Color::Yellow | Color::LightYellow => "#e5c07b",
        Color::Blue | Color::LightBlue => "#61afef",
        Color::Magenta | Color::LightMagenta => "#c678dd",
        Color::Cyan | Color::LightCyan => "#56b6c2",
        Color::Gray => "#abb2bf",
        Color::DarkGray => "#5c6370",
        Color::White => "#ffffff",
        _ => FOREGROUND,
    })
}

/// The colours a cell actually shows, with `REVERSED` applied.
fn cell_colours(cell: &ratatui::buffer::Cell) -> (&'static str, Option<&'static str>) {
    let fg = colour(cell.fg).unwrap_or(FOREGROUND);
    let bg = colour(cell.bg);
    if cell.modifier.contains(Modifier::REVERSED) {
        (bg.unwrap_or(BACKGROUND), Some(fg))
    } else {
        (fg, bg)
    }
}

/// Box-drawing characters become real lines. Drawn as glyphs they leave gaps
/// between rows and depend on whichever font the viewer happens to have.
fn border(symbol: &str) -> Option<(bool, bool, bool, bool)> {
    // (up, down, left, right)
    Some(match symbol {
        "─" => (false, false, true, true),
        "│" => (true, true, false, false),
        "┌" => (false, true, false, true),
        "┐" => (false, true, true, false),
        "└" => (true, false, false, true),
        "┘" => (true, false, true, false),
        _ => return None,
    })
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub fn render(buffer: &Buffer, title: &str) -> String {
    let area = buffer.area;
    let width = PAD * 2.0 + area.width as f32 * CELL_W;
    let height = CHROME + PAD + area.height as f32 * CELL_H;
    let x0 = PAD;
    let y0 = CHROME;

    let mut out = String::new();
    let _ = writeln!(
        out,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-label="{}">"##,
        escape(title)
    );
    let _ = writeln!(
        out,
        r##"<rect width="{width}" height="{height}" rx="10" fill="{BACKGROUND}"/><rect x="0.5" y="0.5" width="{}" height="{}" rx="10" fill="none" stroke="#3e4451"/>"##,
        width - 1.0,
        height - 1.0
    );
    out.push_str(r##"<circle cx="20" cy="18" r="6" fill="#ff5f57"/><circle cx="40" cy="18" r="6" fill="#febc2e"/><circle cx="60" cy="18" r="6" fill="#28c840"/>"##);
    let _ = writeln!(
        out,
        r##"<text x="{}" y="23" text-anchor="middle" font-family="-apple-system, 'Segoe UI', sans-serif" font-size="12" fill="#7f848e">{}</text>"##,
        width / 2.0,
        escape(title)
    );

    // Backgrounds first, so text and borders sit on top of them.
    for y in 0..area.height {
        for x in 0..area.width {
            if let (_, Some(bg)) = cell_colours(&buffer[(x, y)]) {
                let _ = write!(
                    out,
                    r#"<rect x="{}" y="{}" width="{}" height="{CELL_H}" fill="{bg}"/>"#,
                    x0 + x as f32 * CELL_W,
                    y0 + y as f32 * CELL_H,
                    CELL_W + 0.3
                );
            }
        }
    }
    out.push('\n');

    // Borders as strokes from each cell's centre towards its open sides.
    for y in 0..area.height {
        for x in 0..area.width {
            let cell = &buffer[(x, y)];
            let Some((up, down, left, right)) = border(cell.symbol()) else {
                continue;
            };
            let colour = colour(cell.fg).unwrap_or(BORDER);
            let cx = x0 + x as f32 * CELL_W + CELL_W / 2.0;
            let cy = y0 + y as f32 * CELL_H + CELL_H / 2.0;
            let mut seg = |x2: f32, y2: f32| {
                let _ = write!(
                    out,
                    r#"<line x1="{cx}" y1="{cy}" x2="{x2}" y2="{y2}" stroke="{colour}" stroke-width="1.2" stroke-linecap="square"/>"#
                );
            };
            if up {
                seg(cx, cy - CELL_H / 2.0);
            }
            if down {
                seg(cx, cy + CELL_H / 2.0);
            }
            if left {
                seg(cx - CELL_W / 2.0, cy);
            }
            if right {
                seg(cx + CELL_W / 2.0, cy);
            }
        }
    }
    out.push('\n');

    // Text, in runs of identical style. Each run is pinned to its grid
    // columns with textLength, so the layout holds in any monospace font.
    let _ = writeln!(
        out,
        r#"<g font-family="'JetBrains Mono', Menlo, Consolas, 'DejaVu Sans Mono', monospace" font-size="{FONT_SIZE}" xml:space="preserve">"#
    );
    for y in 0..area.height {
        let mut x = 0;
        while x < area.width {
            let cell = &buffer[(x, y)];
            let style = (cell_colours(cell).0, cell.modifier.contains(Modifier::BOLD));
            let start = x;
            let mut text = String::new();
            while x < area.width {
                let next = &buffer[(x, y)];
                let next_style = (cell_colours(next).0, next.modifier.contains(Modifier::BOLD));
                if next_style != style || border(next.symbol()).is_some() {
                    break;
                }
                text.push_str(next.symbol());
                x += 1;
            }
            if x == start {
                // A border cell: already drawn as a line.
                x += 1;
                continue;
            }
            if text.trim().is_empty() {
                continue;
            }
            let _ = write!(
                out,
                r#"<text x="{}" y="{}" fill="{}"{} textLength="{}" lengthAdjust="spacingAndGlyphs">{}</text>"#,
                x0 + start as f32 * CELL_W,
                y0 + y as f32 * CELL_H + CELL_H * 0.72,
                style.0,
                if style.1 {
                    r#" font-weight="bold""#
                } else {
                    ""
                },
                (x - start) as f32 * CELL_W,
                escape(&text)
            );
        }
        out.push('\n');
    }
    out.push_str("</g>\n</svg>\n");
    out
}
