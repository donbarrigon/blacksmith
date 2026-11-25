//! Convierte texto con códigos ANSI (colores de cargo, etc.) en una `Line`
//! de ratatui. Solo entiende SGR (`ESC[...m`); el resto de secuencias se
//! descartan. `\r` borra la línea (las barras de progreso se reescriben así).

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

pub fn to_line(text: &str) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut style = Style::new();
    let mut cur = String::new();
    let mut chars = text.chars().peekable();

    let flush = |cur: &mut String, spans: &mut Vec<Span<'static>>, style: Style| {
        if !cur.is_empty() {
            spans.push(Span::styled(std::mem::take(cur), style));
        }
    };

    while let Some(c) = chars.next() {
        match c {
            '\x1b' => {
                if chars.peek() != Some(&'[') {
                    chars.next(); // ESC + un carácter: se descarta
                    continue;
                }
                chars.next();
                let mut params = String::new();
                let mut fin = None;
                for c in chars.by_ref() {
                    if ('\x40'..='\x7e').contains(&c) {
                        fin = Some(c);
                        break;
                    }
                    params.push(c);
                }
                if fin == Some('m') {
                    flush(&mut cur, &mut spans, style);
                    style = apply_sgr(style, &params);
                }
            }
            '\r' => {
                cur.clear();
                spans.clear();
            }
            '\t' => cur.push_str("    "),
            c if c.is_control() => {}
            c => cur.push(c),
        }
    }
    flush(&mut cur, &mut spans, style);
    Line::from(spans)
}

fn apply_sgr(mut style: Style, params: &str) -> Style {
    let codes: Vec<u16> = if params.is_empty() {
        vec![0]
    } else {
        params.split(';').map(|p| p.parse().unwrap_or(0)).collect()
    };
    let mut it = codes.into_iter();
    while let Some(code) = it.next() {
        style = match code {
            0 => Style::new(),
            1 => style.add_modifier(Modifier::BOLD),
            2 => style.add_modifier(Modifier::DIM),
            3 => style.add_modifier(Modifier::ITALIC),
            4 => style.add_modifier(Modifier::UNDERLINED),
            22 => style.remove_modifier(Modifier::BOLD | Modifier::DIM),
            23 => style.remove_modifier(Modifier::ITALIC),
            24 => style.remove_modifier(Modifier::UNDERLINED),
            30..=37 => style.fg(basic(code - 30, false)),
            90..=97 => style.fg(basic(code - 90, true)),
            40..=47 => style.bg(basic(code - 40, false)),
            100..=107 => style.bg(basic(code - 100, true)),
            39 => style.fg(Color::Reset),
            49 => style.bg(Color::Reset),
            38 | 48 => match extended(&mut it) {
                Some(color) if code == 38 => style.fg(color),
                Some(color) => style.bg(color),
                None => style,
            },
            _ => style,
        };
    }
    style
}

/// `5;n` (256 colores) o `2;r;g;b`.
fn extended(it: &mut impl Iterator<Item = u16>) -> Option<Color> {
    match it.next()? {
        5 => Some(Color::Indexed(it.next()? as u8)),
        2 => Some(Color::Rgb(it.next()? as u8, it.next()? as u8, it.next()? as u8)),
        _ => None,
    }
}

fn basic(n: u16, bright: bool) -> Color {
    match (n, bright) {
        (0, false) => Color::Black,
        (1, false) => Color::Red,
        (2, false) => Color::Green,
        (3, false) => Color::Yellow,
        (4, false) => Color::Blue,
        (5, false) => Color::Magenta,
        (6, false) => Color::Cyan,
        (7, false) => Color::Gray,
        (0, true) => Color::DarkGray,
        (1, true) => Color::LightRed,
        (2, true) => Color::LightGreen,
        (3, true) => Color::LightYellow,
        (4, true) => Color::LightBlue,
        (5, true) => Color::LightMagenta,
        (6, true) => Color::LightCyan,
        _ => Color::White,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colores_y_secuencias() {
        let line = to_line("\x1b[1m\x1b[32m   Compiling\x1b[0m app v0.1.0");
        assert_eq!(line.spans.len(), 2);
        assert_eq!(line.spans[0].content, "   Compiling");
        assert_eq!(
            line.spans[0].style,
            Style::new().fg(Color::Green).add_modifier(Modifier::BOLD)
        );
        assert_eq!(line.spans[1].content, " app v0.1.0");
        assert_eq!(line.spans[1].style, Style::new());

        assert_eq!(to_line("\x1b[38;5;208mx").spans[0].style.fg, Some(Color::Indexed(208)));
        assert_eq!(to_line("\x1b[2Kbarra\rfinal").to_string(), "final");
    }
}
