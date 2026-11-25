//! Selector interactivo de `list`: el orden de la lista final es el orden en
//! que se van marcando, y en migraciones `-` cambia un elemento a down.

use std::io::IsTerminal;

use anyhow::bail;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph};
use ratatui::{DefaultTerminal, Frame, TerminalOptions, Viewport};

use crate::migrate::{MigrationRow, SeedRow};

/// Elige migraciones. Devuelve `m<id>` (up) o `-m<id>` (down) en el orden
/// marcado; `None` si se canceló.
pub fn pick_migrations(rows: &[MigrationRow]) -> anyhow::Result<Option<Vec<String>>> {
    pick("Migraciones", migration_items(rows), true)
}

/// Elige seeds. Devuelve `s<id>` en el orden marcado; `None` si se canceló.
pub fn pick_seeds(rows: &[SeedRow]) -> anyhow::Result<Option<Vec<String>>> {
    pick("Seeds", seed_items(rows), false)
}

/// Items del selector de migraciones (solo las que están en el código).
pub fn migration_items(rows: &[MigrationRow]) -> Vec<Item> {
    rows.iter()
        .filter(|m| m.in_code)
        .map(|m| {
            let state = match (m.applied, m.last_op.as_deref()) {
                (true, _) => format!("ejecutada · batch {}", m.batch.unwrap_or_default()),
                (false, Some("down")) => "revertida".to_string(),
                _ => "pendiente".to_string(),
            };
            Item {
                id: m.id.clone(),
                info: format!("{} · {state}", m.collection.as_deref().unwrap_or("?")),
                applied: m.applied,
            }
        })
        .collect()
}

/// Items del selector de seeds (solo los que están en el código).
pub fn seed_items(rows: &[SeedRow]) -> Vec<Item> {
    rows.iter()
        .filter(|s| s.in_code)
        .map(|s| {
            let state = match s.status.as_deref() {
                Some("ok") => "ejecutado",
                Some("error") => "error",
                _ => "pendiente",
            };
            let dev = if s.only_dev { " · solo dev" } else { "" };
            Item {
                id: s.id.clone(),
                info: format!("{state} · {} intentos{dev}", s.attempts),
                applied: s.status.as_deref() == Some("ok"),
            }
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct Item {
    id: String,
    info: String,
    /// Migración ejecutada / seed ok.
    applied: bool,
}

/// Resultado de una tecla en el selector.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Continue,
    /// Enter: la lista elegida (puede estar vacía).
    Done(Vec<String>),
    Cancel,
}

/// Estado del selector, sin terminal.
pub struct Picker {
    pub items: Vec<Item>,
    allow_down: bool,
    cursor: usize,
    /// (índice del item, down) en el orden en que se marcaron.
    order: Vec<(usize, bool)>,
}

impl Picker {
    pub fn new(items: Vec<Item>, allow_down: bool) -> Self {
        Self {
            items,
            allow_down,
            cursor: 0,
            order: Vec::new(),
        }
    }

    pub fn up(&mut self) {
        self.cursor = self.cursor.checked_sub(1).unwrap_or(self.items.len() - 1);
    }

    pub fn down(&mut self) {
        self.cursor = (self.cursor + 1) % self.items.len();
    }

    /// Mueve el cursor a un item (clic) y lo marca o desmarca.
    pub fn click(&mut self, i: usize) {
        if i < self.items.len() {
            self.cursor = i;
            self.toggle();
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) -> Outcome {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.up(),
            KeyCode::Down | KeyCode::Char('j') => self.down(),
            KeyCode::Char(' ') => self.toggle(),
            KeyCode::Char('-') => self.flip(),
            KeyCode::Enter => return Outcome::Done(self.result()),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Outcome::Cancel,
            KeyCode::Esc | KeyCode::Char('q') => return Outcome::Cancel,
            _ => {}
        }
        Outcome::Continue
    }

    /// Espacio: marca o desmarca. Una migración ejecutada se marca como down.
    fn toggle(&mut self) {
        match self.order.iter().position(|(i, _)| *i == self.cursor) {
            Some(p) => {
                self.order.remove(p);
            }
            None => {
                let down = self.allow_down && self.items[self.cursor].applied;
                self.order.push((self.cursor, down));
            }
        }
    }

    /// `-`: cambia entre up y down (si no estaba marcada, la marca como down).
    fn flip(&mut self) {
        if !self.allow_down {
            return;
        }
        match self.order.iter_mut().find(|(i, _)| *i == self.cursor) {
            Some((_, down)) => *down = !*down,
            None => self.order.push((self.cursor, true)),
        }
    }

    /// (posición 1.., down) si el item está marcado.
    fn mark(&self, i: usize) -> Option<(usize, bool)> {
        self.order
            .iter()
            .position(|(idx, _)| *idx == i)
            .map(|p| (p + 1, self.order[p].1))
    }

    /// Marcado en una dirección que no corresponde a su estado (el binario
    /// `migrate` lo rechazaría salvo que otro paso lo deje listo).
    fn suspicious(&self, i: usize) -> bool {
        self.allow_down && self.mark(i).is_some_and(|(_, down)| down != self.items[i].applied)
    }

    pub fn result(&self) -> Vec<String> {
        self.order
            .iter()
            .map(|(i, down)| {
                let id = &self.items[*i].id;
                if *down { format!("-{id}") } else { id.clone() }
            })
            .collect()
    }
}

fn pick(title: &str, items: Vec<Item>, allow_down: bool) -> anyhow::Result<Option<Vec<String>>> {
    if items.is_empty() {
        bail!("no hay {} en el proyecto", title.to_lowercase());
    }
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        bail!("list necesita una terminal interactiva");
    }
    let (_, rows) = ratatui::crossterm::terminal::size()?;
    let height = (items.len() as u16 + 4).min(rows.saturating_sub(1)).max(6);
    let mut terminal = ratatui::try_init_with_options(TerminalOptions {
        viewport: Viewport::Inline(height),
    })?;
    let mut picker = Picker::new(items, allow_down);
    let result = event_loop(&mut terminal, &mut picker, title);
    let _ = terminal.clear();
    ratatui::restore();
    result
}

fn event_loop(terminal: &mut DefaultTerminal, picker: &mut Picker, title: &str) -> anyhow::Result<Option<Vec<String>>> {
    let mut state = ListState::default();
    loop {
        terminal.draw(|f| {
            render(f, f.area(), picker, title, &mut state);
        })?;
        let Event::Key(key) = event::read()? else { continue };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match picker.on_key(key) {
            Outcome::Continue => {}
            Outcome::Done(list) => return Ok(Some(list)),
            Outcome::Cancel => return Ok(None),
        }
    }
}

/// Dibuja el selector en `area` y devuelve el área de la lista (para clics).
pub fn render(f: &mut Frame, area: Rect, picker: &Picker, title: &str, state: &mut ListState) -> Rect {
    state.select(Some(picker.cursor));
    let [head, list, preview, help] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(area);

    let marked = picker.order.len();
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(title, Style::new().add_modifier(Modifier::BOLD)),
            Span::styled(format!(" · {marked} marcadas"), Style::new().fg(Color::DarkGray)),
        ])),
        head,
    );

    let rows: Vec<ListItem> = picker
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let (num, dir, color) = match picker.mark(i) {
                Some((n, true)) => (format!("{n:>3}"), "↓", Color::Red),
                Some((n, false)) => (format!("{n:>3}"), "↑", Color::Green),
                None => ("   ".to_string(), " ", Color::Reset),
            };
            let warn = if picker.suspicious(i) { " ⚠" } else { "" };
            let prefix = if dir == "↓" { "-" } else { " " };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{num} {dir} "), Style::new().fg(color)),
                Span::styled(format!("{prefix}{}", item.id), Style::new().fg(color)),
                Span::styled(format!("  {}", item.info), Style::new().fg(Color::DarkGray)),
                Span::styled(warn, Style::new().fg(Color::Yellow)),
            ]))
        })
        .collect();
    f.render_stateful_widget(
        List::new(rows)
            .highlight_symbol("› ")
            .highlight_style(Style::new().add_modifier(Modifier::BOLD)),
        list,
        state,
    );

    let result = picker.result();
    let text = if result.is_empty() {
        "(nada marcado)".to_string()
    } else {
        format!("[{}]", result.join(", "))
    };
    f.render_widget(Paragraph::new(text).style(Style::new().fg(Color::Cyan)), preview);

    let keys = if picker.allow_down {
        "↑↓ moverse · espacio marcar · - up/down · enter correr · esc cancelar"
    } else {
        "↑↓ moverse · espacio marcar · enter correr · esc cancelar"
    };
    f.render_widget(Paragraph::new(keys).style(Style::new().fg(Color::DarkGray)), help);
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Vec<Item> {
        ["ma", "mb", "mc"]
            .iter()
            .enumerate()
            .map(|(i, id)| Item {
                id: id.to_string(),
                info: String::new(),
                applied: i == 0,
            })
            .collect()
    }

    #[test]
    fn orden_de_seleccion_y_down() {
        let mut p = Picker::new(items(), true);
        p.cursor = 2;
        p.toggle(); // mc pendiente → up
        p.cursor = 0;
        p.toggle(); // ma ejecutada → down
        p.cursor = 1;
        p.flip(); // mb sin marcar → down
        assert_eq!(p.result(), ["mc", "-ma", "-mb"]);
        assert!(p.suspicious(1)); // mb pendiente marcada como down
        assert!(!p.suspicious(0));

        p.flip(); // mb → up
        p.cursor = 2;
        p.toggle(); // desmarca mc
        assert_eq!(p.result(), ["-ma", "mb"]);
        assert_eq!(p.mark(0), Some((1, true)));
        assert_eq!(p.mark(2), None);
    }

    #[test]
    fn seeds_sin_down() {
        let mut p = Picker::new(items(), false);
        p.toggle(); // ma "ok" → igual es up
        p.cursor = 1;
        p.flip(); // ignorado
        assert_eq!(p.result(), ["ma"]);
        assert!(!p.suspicious(0));
    }

    #[test]
    fn cursor_circular() {
        let mut p = Picker::new(items(), true);
        p.up();
        assert_eq!(p.cursor, 2);
        p.down();
        assert_eq!(p.cursor, 0);
    }
}
