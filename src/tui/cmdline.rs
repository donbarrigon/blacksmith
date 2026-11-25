//! Línea de comandos de la interfaz: texto editable, separación en palabras
//! y parseo con el mismo `Cli` de clap (sin escribir `blacksmith`).

use clap::Parser;

use crate::{Cli, Command};

/// Texto editable con cursor (en caracteres, no bytes).
#[derive(Debug, Default)]
pub struct Input {
    text: String,
    cursor: usize,
}

impl Input {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn set(&mut self, text: &str) {
        self.text = text.to_string();
        self.cursor = self.len();
    }

    /// Reemplaza el texto y deja el cursor en `cursor` (en caracteres).
    pub fn set_with_cursor(&mut self, text: String, cursor: usize) {
        self.text = text;
        self.cursor = cursor.min(self.len());
    }

    /// El texto hasta el cursor.
    pub fn before_cursor(&self) -> String {
        self.text.chars().take(self.cursor).collect()
    }

    pub fn take(&mut self) -> String {
        self.cursor = 0;
        std::mem::take(&mut self.text)
    }

    fn len(&self) -> usize {
        self.text.chars().count()
    }

    fn byte(&self, char_idx: usize) -> usize {
        self.text
            .char_indices()
            .nth(char_idx)
            .map_or(self.text.len(), |(b, _)| b)
    }

    pub fn insert(&mut self, c: char) {
        let at = self.byte(self.cursor);
        self.text.insert(at, c);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            let at = self.byte(self.cursor);
            self.text.remove(at);
        }
    }

    pub fn delete(&mut self) {
        if self.cursor < self.len() {
            let at = self.byte(self.cursor);
            self.text.remove(at);
        }
    }

    pub fn left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.len());
    }

    pub fn home(&mut self) {
        self.cursor = 0;
    }

    pub fn end(&mut self) {
        self.cursor = self.len();
    }
}

/// Separa la línea en palabras. Las comillas ('...' o "...") agrupan y se
/// quitan; una lista `[a, b]` queda como una sola palabra (así no hace falta
/// ponerla entre comillas como en zsh).
pub fn split_words(line: &str) -> Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut cur = String::new();
    let mut in_word = false;
    let mut quote: Option<char> = None;
    let mut bracket = false;

    for c in line.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => cur.push(c),
            (None, '\'' | '"') => {
                quote = Some(c);
                in_word = true;
            }
            (None, '[') if !bracket => {
                bracket = true;
                in_word = true;
                cur.push(c);
            }
            (None, ']') if bracket => {
                bracket = false;
                cur.push(c);
            }
            (None, c) if c.is_whitespace() && !bracket => {
                if in_word {
                    words.push(std::mem::take(&mut cur));
                    in_word = false;
                }
            }
            (None, c) => {
                in_word = true;
                cur.push(c);
            }
        }
    }
    if let Some(q) = quote {
        return Err(format!("falta cerrar la comilla {q}"));
    }
    if bracket {
        return Err("falta cerrar el ]".to_string());
    }
    if in_word {
        words.push(cur);
    }
    Ok(words)
}

/// Qué hacer con una línea escrita.
#[derive(Debug)]
pub enum Parsed {
    /// Línea vacía.
    Empty,
    /// Comando de la propia interfaz.
    Builtin(Builtin),
    /// Comando de blacksmith para ejecutar.
    Run(Command),
    /// `git <args>`: se corre git en el proyecto.
    Git(Vec<String>),
    /// Error o texto de ayuda de clap / del separador, para mostrar.
    Message { text: String, is_error: bool },
}

#[derive(Debug, PartialEq, Eq)]
pub enum Builtin {
    Quit,
    Clear,
    /// `ayuda [tema]`: abre la ayuda, filtrada si hay tema.
    Help(Option<String>),
    /// Abre lazygit pausando la interfaz.
    Lazygit,
    /// `editar <archivo>`: lo abre en $EDITOR pausando la interfaz.
    Edit(String),
}

pub fn parse(line: &str) -> Parsed {
    let mut words = match split_words(line) {
        Ok(w) => w,
        Err(e) => {
            return Parsed::Message {
                text: e,
                is_error: true,
            };
        }
    };
    // Por costumbre se puede escribir igual `blacksmith run ...`.
    if words.first().is_some_and(|w| w == "blacksmith") {
        words.remove(0);
    }
    match words.first().map(String::as_str) {
        None => return Parsed::Empty,
        Some("exit" | "quit" | "salir" | "q") if words.len() == 1 => return Parsed::Builtin(Builtin::Quit),
        Some("clear" | "limpiar") if words.len() == 1 => return Parsed::Builtin(Builtin::Clear),
        Some("git") => return Parsed::Git(words[1..].to_vec()),
        Some("lazygit") if words.len() == 1 => return Parsed::Builtin(Builtin::Lazygit),
        Some("editar" | "edit") if words.len() == 2 => return Parsed::Builtin(Builtin::Edit(words[1].clone())),
        Some("editar" | "edit") => {
            return Parsed::Message {
                text: "uso: editar <archivo>".into(),
                is_error: true,
            };
        }
        Some("ayuda" | "help" | "?") => {
            let query = (words.len() > 1).then(|| words[1..].join(" "));
            return Parsed::Builtin(Builtin::Help(query));
        }
        _ => {}
    }
    match Cli::try_parse_from(std::iter::once("blacksmith".to_string()).chain(words)) {
        Ok(cli) => Parsed::Run(cli.command),
        Err(e) => Parsed::Message {
            text: e.render().to_string().trim_end().to_string(),
            is_error: e.use_stderr(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separa_palabras_con_listas_y_comillas() {
        assert_eq!(
            split_words("run migration [m1, -m2]  seed").unwrap(),
            ["run", "migration", "[m1, -m2]", "seed"]
        );
        assert_eq!(split_words("run 'a b' \"c\"").unwrap(), ["run", "a b", "c"]);
        assert_eq!(split_words("  ").unwrap(), Vec::<String>::new());
        assert!(split_words("run [m1").is_err());
        assert!(split_words("run 'a").is_err());
    }

    #[test]
    fn parsea_sin_blacksmith() {
        match parse("run migration rollback step 2") {
            Parsed::Run(Command::Run { target, args }) => {
                assert_eq!(target, "migration");
                assert_eq!(args, ["rollback", "step", "2"]);
            }
            other => panic!("{other:?}"),
        }
        assert!(matches!(parse("blacksmith run seed"), Parsed::Run(Command::Run { .. })));
        assert!(matches!(parse("ui"), Parsed::Run(Command::Ui)));
        assert!(matches!(parse(""), Parsed::Empty));
        assert!(matches!(parse("salir"), Parsed::Builtin(Builtin::Quit)));
        assert!(matches!(parse("nada"), Parsed::Message { is_error: true, .. }));
        assert!(matches!(parse("help"), Parsed::Builtin(Builtin::Help(None))));
        assert!(matches!(parse("ayuda step"), Parsed::Builtin(Builtin::Help(Some(q))) if q == "step"));
        assert!(matches!(parse("run --help"), Parsed::Message { is_error: false, .. }));
        assert!(matches!(parse("git commit -m 'hola mundo'"), Parsed::Git(a) if a == ["commit", "-m", "hola mundo"]));
        assert!(matches!(parse("lazygit"), Parsed::Builtin(Builtin::Lazygit)));
    }

    #[test]
    fn edita_con_acentos() {
        let mut i = Input::default();
        for c in "migración".chars() {
            i.insert(c);
        }
        i.left();
        i.backspace(); // borra la ó
        assert_eq!(i.text(), "migracin");
        i.home();
        i.delete();
        assert_eq!(i.text(), "igracin");
        i.end();
        assert_eq!(i.cursor(), 7);
    }
}
