//! Diseñador de `make model`: los campos se escriben con la sintaxis del
//! Blueprint (una llamada por línea) y al lado se ve la migración y el modelo
//! que se van a crear. Ctrl+S los crea.

use std::path::PathBuf;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::make::{self, ModelPlan, spec};

/// Texto de varias líneas con cursor (fila y columna en caracteres).
#[derive(Debug, Default)]
pub struct TextArea {
    pub lines: Vec<String>,
    pub row: usize,
    pub col: usize,
    /// Primera fila visible.
    pub top: usize,
}

impl TextArea {
    pub fn new(text: &str) -> Self {
        let lines: Vec<String> = text.lines().map(str::to_string).collect();
        let row = lines.len().saturating_sub(1);
        let col = lines.last().map_or(0, |l| l.chars().count());
        Self {
            lines: if lines.is_empty() { vec![String::new()] } else { lines },
            row,
            col,
            top: 0,
        }
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    fn line_len(&self) -> usize {
        self.lines[self.row].chars().count()
    }

    fn byte(&self, col: usize) -> usize {
        let l = &self.lines[self.row];
        l.char_indices().nth(col).map_or(l.len(), |(b, _)| b)
    }

    pub fn insert_str(&mut self, s: &str) {
        for c in s.chars() {
            self.insert(c);
        }
    }

    pub fn insert(&mut self, c: char) {
        if c == '\n' {
            return self.newline();
        }
        let at = self.byte(self.col);
        self.lines[self.row].insert(at, c);
        self.col += 1;
    }

    pub fn newline(&mut self) {
        let at = self.byte(self.col);
        let rest = self.lines[self.row].split_off(at);
        self.row += 1;
        self.lines.insert(self.row, rest);
        self.col = 0;
    }

    pub fn backspace(&mut self) {
        if self.col > 0 {
            self.col -= 1;
            let at = self.byte(self.col);
            self.lines[self.row].remove(at);
        } else if self.row > 0 {
            let line = self.lines.remove(self.row);
            self.row -= 1;
            self.col = self.line_len();
            self.lines[self.row].push_str(&line);
        }
    }

    pub fn delete(&mut self) {
        if self.col < self.line_len() {
            let at = self.byte(self.col);
            self.lines[self.row].remove(at);
        } else if self.row + 1 < self.lines.len() {
            let next = self.lines.remove(self.row + 1);
            self.lines[self.row].push_str(&next);
        }
    }

    pub fn left(&mut self) {
        if self.col > 0 {
            self.col -= 1;
        } else if self.row > 0 {
            self.row -= 1;
            self.col = self.line_len();
        }
    }

    pub fn right(&mut self) {
        if self.col < self.line_len() {
            self.col += 1;
        } else if self.row + 1 < self.lines.len() {
            self.row += 1;
            self.col = 0;
        }
    }

    pub fn up(&mut self) {
        if self.row > 0 {
            self.row -= 1;
            self.col = self.col.min(self.line_len());
        }
    }

    pub fn down(&mut self) {
        if self.row + 1 < self.lines.len() {
            self.row += 1;
            self.col = self.col.min(self.line_len());
        }
    }

    pub fn home(&mut self) {
        self.col = 0;
    }

    pub fn end(&mut self) {
        self.col = self.line_len();
    }

    /// Ajusta `top` para que el cursor quede a la vista en `height` filas.
    pub fn scroll_to_cursor(&mut self, height: usize) {
        if self.row < self.top {
            self.top = self.row;
        } else if height > 0 && self.row >= self.top + height {
            self.top = self.row + 1 - height;
        }
    }

    /// Lo escrito desde el inicio de la línea (sin espacios ni `col.`) hasta
    /// el cursor, si es una palabra: lo que Tab puede completar.
    fn word_before_cursor(&self) -> Option<(usize, String)> {
        let before: String = self.lines[self.row].chars().take(self.col).collect();
        let indent = before.chars().take_while(|c| c.is_whitespace()).count();
        let mut start = indent;
        let mut word: String = before.chars().skip(indent).collect();
        if let Some(w) = word.strip_prefix("col.") {
            word = w.to_string();
            start += 4;
        }
        word.chars()
            .all(|c| c.is_ascii_lowercase() || c == '_')
            .then_some((start, word))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Migration,
    Model,
}

/// Qué pasó con una tecla.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Continue,
    Save,
    Cancel,
}

pub struct Designer {
    root: PathBuf,
    pub model: String,
    pub collection: Option<String>,
    pub editor: TextArea,
    pub view: View,
    pub preview_scroll: u16,
    pub plan: Result<ModelPlan, Vec<String>>,
    /// Opciones de Tab cuando hay varias, para mostrarlas.
    pub completions: Vec<&'static str>,
    /// Esc con cambios: la segunda vez descarta.
    pub confirm_cancel: bool,
    original: String,
}

pub const TEMPLATE: &str = "id()\nstring(\"name\")\ntimestamps()";

impl Designer {
    pub fn new(root: PathBuf, model: String, collection: Option<String>) -> Self {
        let mut d = Self {
            root,
            model,
            collection,
            editor: TextArea::new(TEMPLATE),
            view: View::Migration,
            preview_scroll: 0,
            plan: Err(Vec::new()),
            completions: Vec::new(),
            confirm_cancel: false,
            original: TEMPLATE.to_string(),
        };
        d.recompute();
        d
    }

    /// La colección que se va a crear (la dada o el plural del modelo).
    pub fn collection_name(&self) -> String {
        match (&self.collection, make::snake_name(&self.model)) {
            (Some(c), _) => c.clone(),
            (None, Ok(snake)) => make::plural(&snake),
            (None, Err(_)) => "?".into(),
        }
    }

    fn recompute(&mut self) {
        self.plan = make::plan_model(&self.root, &self.model, self.collection.as_deref(), &self.editor.text());
    }

    /// Líneas con error (números desde 1), para marcarlas.
    pub fn error_lines(&self) -> Vec<usize> {
        match spec::parse(&self.editor.text()) {
            Ok(_) => Vec::new(),
            Err(errs) => errs.iter().map(|e| e.line).collect(),
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) -> Outcome {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let before = self.editor.text();
        if key.code != KeyCode::Esc {
            self.confirm_cancel = false;
        }
        if key.code != KeyCode::Tab {
            self.completions.clear();
        }
        match key.code {
            KeyCode::Char('s') if ctrl => {
                return if self.plan.is_ok() {
                    Outcome::Save
                } else {
                    Outcome::Continue
                };
            }
            KeyCode::Char('t') if ctrl => {
                self.view = match self.view {
                    View::Migration => View::Model,
                    View::Model => View::Migration,
                };
                self.preview_scroll = 0;
            }
            KeyCode::Esc => {
                if self.editor.text() == self.original || self.confirm_cancel {
                    return Outcome::Cancel;
                }
                self.confirm_cancel = true;
            }
            KeyCode::PageUp => self.preview_scroll = self.preview_scroll.saturating_sub(10),
            KeyCode::PageDown => self.preview_scroll += 10,
            KeyCode::Tab => self.complete(),
            KeyCode::Enter => self.editor.newline(),
            KeyCode::Char(c) if !ctrl => self.editor.insert(c),
            KeyCode::Backspace => self.editor.backspace(),
            KeyCode::Delete => self.editor.delete(),
            KeyCode::Left => self.editor.left(),
            KeyCode::Right => self.editor.right(),
            KeyCode::Up => self.editor.up(),
            KeyCode::Down => self.editor.down(),
            KeyCode::Home => self.editor.home(),
            KeyCode::End => self.editor.end(),
            _ => {}
        }
        if self.editor.text() != before {
            self.recompute();
        }
        Outcome::Continue
    }

    /// Tab al inicio de una línea completa el tipo: `str` → `string("")`
    /// con el cursor entre las comillas.
    fn complete(&mut self) {
        let Some((start, word)) = self.editor.word_before_cursor() else {
            return;
        };
        let options: Vec<&'static str> = spec::FIELD_METHODS
            .iter()
            .chain(spec::INDEX_METHODS)
            .copied()
            .filter(|m| m.starts_with(&word))
            .collect();
        match options.as_slice() {
            [] => {}
            [one] => {
                // Reemplaza la palabra por la llamada completa.
                for _ in start..self.editor.col {
                    self.editor.backspace();
                }
                if matches!(*one, "id" | "timestamps" | "soft_deletes") {
                    self.editor.insert_str(&format!("{one}()"));
                } else {
                    self.editor.insert_str(&format!("{one}(\"\")"));
                    self.editor.left();
                    self.editor.left();
                }
            }
            many => {
                // Completa el prefijo común y muestra las opciones.
                let common = many.iter().skip(1).fold(many[0].to_string(), |acc, m| {
                    acc.chars()
                        .zip(m.chars())
                        .take_while(|(a, b)| a == b)
                        .map(|(a, _)| a)
                        .collect()
                });
                if common.len() > word.len() {
                    self.editor.insert_str(&common[word.len()..]);
                }
                self.completions = many.to_vec();
            }
        }
    }

    pub fn save(&self) -> anyhow::Result<make::Created> {
        match &self.plan {
            Ok(plan) => make::write_model(&self.root, plan),
            Err(errs) => anyhow::bail!(errs.join("; ")),
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::{KeyEventKind, KeyEventState};

    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn ctrl(c: char) -> KeyEvent {
        KeyEvent {
            modifiers: KeyModifiers::CONTROL,
            ..key(KeyCode::Char(c))
        }
    }

    fn designer() -> Designer {
        Designer::new(
            std::env::temp_dir().join("blacksmith_designer_no_existe"),
            "User".into(),
            None,
        )
    }

    #[test]
    fn editor_basico() {
        let mut t = TextArea::new("ab\ncd");
        assert_eq!((t.row, t.col), (1, 2));
        t.home();
        t.backspace(); // une con la anterior
        assert_eq!(t.text(), "abcd");
        assert_eq!((t.row, t.col), (0, 2));
        t.newline();
        t.insert_str("ñ");
        assert_eq!(t.text(), "ab\nñcd");
        t.up();
        t.end();
        t.delete(); // trae la siguiente
        assert_eq!(t.text(), "abñcd");
    }

    #[test]
    fn tab_completa_tipos() {
        let mut d = designer();
        d.on_key(key(KeyCode::Enter));
        for c in "str".chars() {
            d.on_key(key(KeyCode::Char(c)));
        }
        d.on_key(key(KeyCode::Tab));
        assert_eq!(d.editor.lines[3], "string(\"\")");
        for c in "email".chars() {
            d.on_key(key(KeyCode::Char(c)));
        }
        assert_eq!(d.editor.lines[3], "string(\"email\")");
        assert!(d.plan.is_ok());

        d.on_key(key(KeyCode::End));
        d.on_key(key(KeyCode::Enter));
        for c in "da".chars() {
            d.on_key(key(KeyCode::Char(c)));
        }
        d.on_key(key(KeyCode::Tab)); // date / date_time: completa "date" y muestra opciones
        assert_eq!(d.editor.lines[4], "date");
        assert_eq!(d.completions, ["date", "date_time"]);
    }

    #[test]
    fn errores_bloquean_guardar_y_esc_pide_confirmar() {
        let mut d = designer();
        assert_eq!(d.on_key(key(KeyCode::Esc)), Outcome::Cancel); // sin cambios

        let mut d = designer();
        d.on_key(key(KeyCode::Enter));
        for c in "strng(\"x\")".chars() {
            d.on_key(key(KeyCode::Char(c)));
        }
        assert_eq!(d.error_lines(), [4]);
        assert_eq!(d.on_key(ctrl('s')), Outcome::Continue);
        assert_eq!(d.on_key(key(KeyCode::Esc)), Outcome::Continue);
        assert!(d.confirm_cancel);
        assert_eq!(d.on_key(key(KeyCode::Esc)), Outcome::Cancel);
    }

    #[test]
    fn vista_previa() {
        let mut d = designer();
        let plan = d.plan.as_ref().unwrap();
        assert!(plan.migration_code.contains("col.string(\"name\");"));
        assert!(plan.model_code.contains("pub name: String,"));
        assert_eq!(plan.model_path, PathBuf::from("src/models/user.rs"));
        d.on_key(ctrl('t'));
        assert_eq!(d.view, View::Model);
    }
}
