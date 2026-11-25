//! Sintaxis de los comandos de blacksmith (la misma que el binario `migrate`
//! de ironforge, más `list`). De aquí salen el autocompletado, la ayuda según
//! lo que se está escribiendo y la referencia completa.

use crate::migrate::Status;

/// Un tema de ayuda: un comando y lo que hace.
#[derive(Debug, PartialEq, Eq)]
pub struct Topic {
    pub syntax: &'static str,
    pub desc: &'static str,
    pub notes: &'static [&'static str],
}

const FORCE_NOTE: &str = "force: no pide confirmación cuando app.debug es false.";

pub const RUN: Topic = Topic {
    syntax: "run <migration | seed | dev | módulo> ...",
    desc: "Corre algo del proyecto.",
    notes: &["dev y los módulos todavía no están implementados."],
};
pub const MIGRATE: Topic = Topic {
    syntax: "run migration [seed ...]",
    desc: "Ejecuta las migraciones pendientes.",
    notes: &["Con `seed` al final corre después los seeds pendientes."],
};
pub const ROLLBACK: Topic = Topic {
    syntax: "run migration rollback [step N]",
    desc: "Revierte el último batch, o las últimas N migraciones con step N.",
    notes: &[],
};
pub const RESET: Topic = Topic {
    syntax: "run migration reset [force]",
    desc: "Revierte todas las migraciones.",
    notes: &[FORCE_NOTE],
};
pub const REFRESH: Topic = Topic {
    syntax: "run migration refresh [force] [seed ...]",
    desc: "Revierte todas las migraciones y las vuelve a ejecutar.",
    notes: &[FORCE_NOTE],
};
pub const FRESH: Topic = Topic {
    syntax: "run migration fresh [force] [seed ...]",
    desc: "Borra todas las colecciones de la base y ejecuta todas las migraciones.",
    notes: &["Cuidado: borra los datos.", FORCE_NOTE],
};
pub const STATUS: Topic = Topic {
    syntax: "run migration status [json]",
    desc: "Muestra el estado de migraciones y seeds.",
    notes: &[],
};
pub const UNLOCK: Topic = Topic {
    syntax: "run migration unlock",
    desc: "Libera el lock de migraciones si quedó tomado (por ejemplo, tras cortar una).",
    notes: &[],
};
pub const MIGRATION_IDS: Topic = Topic {
    syntax: "run migration [m<id>, -m<id>, ...] [seed ...]",
    desc: "Corre solo esas migraciones, en ese orden, en un batch nuevo.",
    notes: &["m<id> = up (tiene que estar pendiente) · -m<id> = down (tiene que estar ejecutada)."],
};
pub const MIGRATION_PICK: Topic = Topic {
    syntax: "run migration list [seed ...]",
    desc: "Elige de una lista qué migraciones correr.",
    notes: &["En el selector: espacio marca, - cambia up/down, enter corre, esc cancela."],
};
pub const SEED_TAIL: Topic = Topic {
    syntax: "... seed [[s<id>, ...] | list]",
    desc: "Al final de migration, refresh, fresh, [...] o list: corre seeds después.",
    notes: &["Sin lista corre los pendientes."],
};
pub const SEED: Topic = Topic {
    syntax: "run seed",
    desc: "Corre los seeds nuevos y los que fallaron.",
    notes: &["Con app.debug false los seeds solo dev no corren."],
};
pub const SEED_IDS: Topic = Topic {
    syntax: "run seed [s<id>, ...] [force]",
    desc: "Corre solo esos seeds, en ese orden, aunque ya hayan corrido.",
    notes: &[FORCE_NOTE],
};
pub const SEED_PICK: Topic = Topic {
    syntax: "run seed list [force]",
    desc: "Elige de una lista qué seeds correr.",
    notes: &[FORCE_NOTE],
};
pub const GIT: Topic = Topic {
    syntax: "git <comando> ...",
    desc: "Corre git en la carpeta del proyecto; la salida va a una pestaña de Salida.",
    notes: &[
        "Sin editor: usa git commit -m \"mensaje\".",
        "No pide usuario/contraseña por la terminal: usa ssh o un credential helper.",
    ],
};
pub const LAZYGIT: Topic = Topic {
    syntax: "lazygit",
    desc: "Abre lazygit pausando la interfaz; al salir de lazygit se vuelve aquí.",
    notes: &[],
};
pub const MAKE_MIGRATION: Topic = Topic {
    syntax: "make migration <nombre> [colección]",
    desc: "Crea una migración en src/bin/migrate/migrations y la registra en su mod.rs.",
    notes: &[
        "create_<colección> crea la colección; otro nombre deja una plantilla de alter.",
        "La colección se deduce de create_x o de ..._to_x / _from_x / _in_x / _on_x.",
    ],
};
pub const MAKE_SEED: Topic = Topic {
    syntax: "make seed <nombre>",
    desc: "Crea un seed en src/bin/migrate/seeds y lo registra en su mod.rs.",
    notes: &[],
};
pub const MAKE_MODEL: Topic = Topic {
    syntax: "make model <Nombre> [colección] [--fields \"...\"]",
    desc: "Crea el modelo (src/models) y su migración create_<colección> con los mismos campos.",
    notes: &[
        "En la interfaz, sin --fields abre el diseñador: campos con la sintaxis del Blueprint y vista previa.",
        "La colección por defecto es el plural: User → users, Category → categories.",
    ],
};
pub const EDIT: Topic = Topic {
    syntax: "editar <archivo>",
    desc: "Abre el archivo en $VISUAL / $EDITOR (o nano) pausando la interfaz.",
    notes: &["La ruta es relativa a la raíz del proyecto. En el panel Git, la tecla e abre el archivo elegido."],
};
pub const UI_COMMANDS: Topic = Topic {
    syntax: "ayuda [tema] · limpiar · salir",
    desc: "Comandos de la interfaz.",
    notes: &["ayuda rollback muestra solo los temas que mencionan rollback."],
};

/// Todos los temas, agrupados, para la referencia completa.
pub const REFERENCE: &[(&str, &[&Topic])] = &[
    (
        "Migraciones",
        &[
            &MIGRATE,
            &ROLLBACK,
            &RESET,
            &REFRESH,
            &FRESH,
            &MIGRATION_IDS,
            &MIGRATION_PICK,
            &STATUS,
            &UNLOCK,
            &SEED_TAIL,
        ],
    ),
    ("Seeds", &[&SEED, &SEED_IDS, &SEED_PICK]),
    ("Crear archivos", &[&MAKE_MODEL, &MAKE_MIGRATION, &MAKE_SEED, &EDIT]),
    ("Git", &[&GIT, &LAZYGIT]),
    ("Interfaz", &[&UI_COMMANDS]),
];

/// Temas que mencionan `query` (en la sintaxis o la descripción).
pub fn search(query: &str) -> Vec<&'static Topic> {
    let q = query.to_lowercase();
    REFERENCE
        .iter()
        .flat_map(|(_, topics)| topics.iter().copied())
        .filter(|t| t.syntax.to_lowercase().contains(&q) || t.desc.to_lowercase().contains(&q))
        .collect()
}

/// Una opción para completar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    /// Texto que reemplaza desde `from` hasta el cursor.
    pub insert: String,
    pub label: String,
    pub desc: String,
    /// Posición (en caracteres) desde donde se reemplaza.
    pub from: usize,
}

/// Lo que se sabe de la línea hasta el cursor.
#[derive(Debug, Default)]
pub struct Analysis {
    /// El comando que se está escribiendo.
    pub topic: Option<&'static Topic>,
    pub suggestions: Vec<Suggestion>,
    /// Lo escrito ya no es válido (el mismo error que daría `migrate`).
    pub error: Option<String>,
    /// Qué va a continuación cuando no hay opciones fijas (ej. un número).
    pub expecting: Option<String>,
}

/// Analiza `before` (el texto hasta el cursor). `catalog` da los IDs reales
/// de migraciones y seeds; sin él no se sugieren IDs.
pub fn analyze(before: &str, catalog: Option<&Status>) -> Analysis {
    let lexed = lex(before);
    let exp = expect(&lexed.toks);
    let mut out = Analysis {
        topic: exp.topic,
        error: exp.error,
        expecting: exp.expecting.map(str::to_string),
        suggestions: Vec::new(),
    };
    if out.error.is_some() {
        return out;
    }
    match lexed.cur {
        Cur::Word { partial, from } => {
            out.suggestions = exp
                .opts
                .iter()
                .filter(|o| o.word.starts_with(partial.as_str()))
                .map(|o| Suggestion {
                    insert: if o.word == "[" {
                        "[".into()
                    } else {
                        format!("{} ", o.word)
                    },
                    label: o.word.into(),
                    desc: o.desc.into(),
                    from,
                })
                .collect();
            if out.suggestions.is_empty() && !partial.is_empty() && !exp.opts.is_empty() {
                out.expecting = Some(format!("nada empieza con «{partial}»"));
            }
        }
        Cur::List {
            items,
            partial,
            from,
            close_from,
        } => {
            let Some(kind) = exp.opts.iter().find_map(|o| o.list) else {
                out.error = Some("aquí no va una lista".into());
                return out;
            };
            out.topic = Some(match kind {
                IdKind::Migrations => &MIGRATION_IDS,
                IdKind::Seeds => &SEED_IDS,
            });
            if !items.is_empty() && partial.is_empty() {
                out.suggestions.push(Suggestion {
                    insert: "] ".into(),
                    label: "]".into(),
                    desc: "cerrar la lista".into(),
                    from: close_from,
                });
            }
            match catalog {
                Some(st) => out.suggestions.extend(id_suggestions(kind, st, &items, &partial, from)),
                None => out.expecting = Some("IDs: todavía no se leyó el estado del proyecto".into()),
            }
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IdKind {
    Migrations,
    Seeds,
}

fn id_suggestions(kind: IdKind, st: &Status, items: &[String], partial: &str, from: usize) -> Vec<Suggestion> {
    let used: Vec<&str> = items.iter().map(|i| i.trim_start_matches('-')).collect();
    let want_down = partial.starts_with('-');
    let needle = partial.trim_start_matches('-').to_lowercase();
    // (id con signo, descripción)
    let candidates: Vec<(String, String)> = match kind {
        IdKind::Migrations => st
            .migrations
            .iter()
            .filter(|m| m.in_code)
            .map(|m| {
                let down = m.applied || want_down;
                let state = if m.applied { "ejecutada" } else { "pendiente" };
                let dir = if down { "down" } else { "up" };
                let col = m.collection.as_deref().unwrap_or("?");
                let id = if down { format!("-{}", m.id) } else { m.id.clone() };
                (id, format!("{col} · {state} → {dir}"))
            })
            .collect(),
        IdKind::Seeds => st
            .seeds
            .iter()
            .filter(|s| s.in_code)
            .map(|s| {
                let state = match s.status.as_deref() {
                    Some("ok") => "ejecutado",
                    Some("error") => "con error",
                    _ => "pendiente",
                };
                let dev = if s.only_dev { " · solo dev" } else { "" };
                (s.id.clone(), format!("{state}{dev}"))
            })
            .collect(),
    };
    let mut out: Vec<(bool, Suggestion)> = candidates
        .into_iter()
        .filter(|(id, _)| !used.contains(&id.trim_start_matches('-')))
        .filter(|(id, _)| id.to_lowercase().contains(&needle))
        .map(|(id, desc)| {
            let prefix = id.trim_start_matches('-').to_lowercase().starts_with(&needle);
            let s = Suggestion {
                insert: format!("{id}, "),
                label: id,
                desc,
                from,
            };
            (prefix, s)
        })
        .collect();
    out.sort_by_key(|(prefix, _)| !prefix); // estable: primero los que empiezan igual
    out.into_iter().map(|(_, s)| s).collect()
}

// ---------------------------------------------------------------------------
// Separación en palabras, tolerante a una lista sin cerrar
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
enum Tok {
    Word(String),
    List(Vec<String>),
}

#[derive(Debug, PartialEq, Eq)]
enum Cur {
    Word {
        partial: String,
        from: usize,
    },
    List {
        items: Vec<String>,
        partial: String,
        from: usize,
        /// Desde dónde reemplazar al cerrar con `]` (se quita el `, ` final).
        close_from: usize,
    },
}

struct Lexed {
    toks: Vec<Tok>,
    cur: Cur,
}

fn lex(text: &str) -> Lexed {
    let mut toks = Vec::new();
    let mut word = String::new();
    let mut word_from = 0;
    let mut list: Option<Vec<String>> = None;
    let mut last_content = 0; // posición tras el último carácter útil de la lista

    let chars: Vec<char> = text.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        match (&mut list, c) {
            (None, '[') if word.is_empty() => {
                list = Some(Vec::new());
                last_content = i + 1;
            }
            (None, c) if c.is_whitespace() => {
                if !word.is_empty() {
                    toks.push(Tok::Word(std::mem::take(&mut word)));
                }
            }
            (None, c) => {
                if word.is_empty() {
                    word_from = i;
                }
                word.push(c);
            }
            (Some(items), ']') => {
                if !word.is_empty() {
                    items.push(std::mem::take(&mut word));
                }
                toks.push(Tok::List(std::mem::take(items)));
                list = None;
            }
            (Some(items), c) if c == ',' || c.is_whitespace() => {
                if !word.is_empty() {
                    items.push(std::mem::take(&mut word));
                }
            }
            (Some(_), c) => {
                if word.is_empty() {
                    word_from = i;
                }
                word.push(c);
                last_content = i + 1;
            }
        }
    }
    let end = chars.len();
    let from = if word.is_empty() { end } else { word_from };
    let cur = match list {
        Some(items) => Cur::List {
            items,
            partial: word,
            from,
            close_from: last_content,
        },
        None => Cur::Word { partial: word, from },
    };
    Lexed { toks, cur }
}

// ---------------------------------------------------------------------------
// Qué se espera después de las palabras completas
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Opt {
    word: &'static str,
    desc: &'static str,
    /// Para `[`: de qué son los IDs.
    list: Option<IdKind>,
}

const fn opt(word: &'static str, desc: &'static str) -> Opt {
    Opt { word, desc, list: None }
}

#[derive(Debug, Default)]
struct Expect {
    topic: Option<&'static Topic>,
    opts: Vec<Opt>,
    error: Option<String>,
    expecting: Option<&'static str>,
}

impl Expect {
    fn opts(topic: Option<&'static Topic>, opts: Vec<Opt>) -> Self {
        Self {
            topic,
            opts,
            ..Self::default()
        }
    }

    fn err(topic: Option<&'static Topic>, msg: impl Into<String>) -> Self {
        Self {
            topic,
            error: Some(msg.into()),
            ..Self::default()
        }
    }
}

fn word(t: &Tok) -> Option<&str> {
    match t {
        Tok::Word(w) => Some(w),
        Tok::List(_) => None,
    }
}

fn expect(toks: &[Tok]) -> Expect {
    let toks = match toks.first().and_then(word) {
        Some("blacksmith") => &toks[1..],
        _ => toks,
    };
    let Some(first) = toks.first() else {
        return Expect::opts(
            None,
            vec![
                opt("run", "corre migraciones, seeds, dev o un módulo"),
                opt("make", "crea un modelo, una migración o un seed"),
                opt("editar", "abre un archivo en tu editor"),
                opt("git", "corre git en el proyecto"),
                opt("lazygit", "abre lazygit"),
                opt("ayuda", "abre la ayuda (ayuda <tema> para buscar)"),
                opt("limpiar", "borra el registro"),
                opt("salir", "cierra la interfaz"),
            ],
        );
    };
    let rest = &toks[1..];
    match word(first) {
        Some("run") => expect_run(rest),
        Some("git") if rest.is_empty() => Expect::opts(
            Some(&GIT),
            vec![
                opt("status", "estado del repositorio"),
                opt("commit", "commit -m \"mensaje\""),
                opt("push", "sube los commits"),
                opt("pull", "trae los cambios"),
                opt("diff", "diferencias sin preparar"),
                opt("add", "prepara archivos"),
                opt("log", "historial: log --oneline -20"),
                opt("switch", "cambia de rama"),
                opt("stash", "guarda cambios aparte"),
            ],
        ),
        // Los argumentos de git no se validan: se pasan tal cual.
        Some("git") => Expect::opts(Some(&GIT), Vec::new()),
        Some("make") => expect_make(rest),
        Some("editar" | "edit") => match rest.len() {
            0 => Expect {
                topic: Some(&EDIT),
                expecting: Some("archivo: ruta desde la raíz del proyecto"),
                ..Expect::default()
            },
            1 => Expect::opts(Some(&EDIT), Vec::new()),
            _ => Expect::err(
                Some(&EDIT),
                "editar abre un solo archivo (usa comillas si la ruta tiene espacios)",
            ),
        },
        Some("lazygit") if rest.is_empty() => Expect::opts(Some(&LAZYGIT), Vec::new()),
        Some("lazygit") => Expect::err(Some(&LAZYGIT), "lazygit no lleva argumentos"),
        Some("ayuda" | "help" | "?") => Expect::opts(Some(&UI_COMMANDS), Vec::new()),
        Some("limpiar" | "clear" | "salir" | "exit" | "quit" | "q") if rest.is_empty() => {
            Expect::opts(Some(&UI_COMMANDS), Vec::new())
        }
        Some(w @ ("limpiar" | "clear" | "salir" | "exit" | "quit" | "q")) => {
            Expect::err(Some(&UI_COMMANDS), format!("{w} no lleva argumentos"))
        }
        Some("ui") => Expect::err(None, "ya estás en la interfaz"),
        Some(w) => Expect::err(None, format!("comando desconocido: {w}")),
        None => Expect::err(None, "la lista va después de migration o seed"),
    }
}

fn expect_make(rest: &[Tok]) -> Expect {
    let words: Vec<Option<&str>> = rest.iter().map(word).collect();
    if words.iter().any(Option::is_none) {
        return Expect::err(None, "make no lleva listas");
    }
    let words: Vec<&str> = words.into_iter().flatten().collect();
    let (topic, max, name_hint): (&'static Topic, usize, &'static str) = match words.first() {
        None => {
            return Expect::opts(
                None,
                vec![
                    opt("model", "modelo + su migración (abre el diseñador)"),
                    opt("migration", "migración nueva"),
                    opt("seed", "seed nuevo"),
                ],
            );
        }
        Some(&"migration") => (&MAKE_MIGRATION, 3, "nombre: create_users, add_phone_to_users..."),
        Some(&"seed") => (&MAKE_SEED, 2, "nombre: users_admin..."),
        Some(&"model") => (&MAKE_MODEL, 3, "Nombre del modelo: User, BlogPost..."),
        Some(other) => return Expect::err(None, format!("make crea model, migration o seed, no {other}")),
    };
    let topic = Some(topic);
    // --fields y su valor no cuentan como argumentos posicionales.
    let mut positional = 0;
    let mut i = 1;
    while i < words.len() {
        if words[i] == "--fields" && topic == Some(&MAKE_MODEL) {
            if i + 1 >= words.len() {
                return Expect {
                    topic,
                    expecting: Some("campos entre comillas: \"string(\\\"name\\\"); int(\\\"age\\\")\""),
                    ..Expect::default()
                };
            }
            i += 2;
            continue;
        }
        positional += 1;
        i += 1;
    }
    match positional {
        0 => Expect {
            topic,
            expecting: Some(name_hint),
            ..Expect::default()
        },
        n if n + 1 > max => Expect::err(topic, format!("sobra «{}»", words.last().unwrap_or(&""))),
        _ if topic == Some(&MAKE_SEED) => Expect::opts(topic, Vec::new()),
        _ => {
            let mut opts = Vec::new();
            if topic == Some(&MAKE_MODEL) && !words.contains(&"--fields") {
                opts.push(opt("--fields", "campos sin abrir el diseñador"));
            }
            Expect {
                topic,
                opts,
                expecting: Some("[colección] opcional · enter crea"),
                ..Expect::default()
            }
        }
    }
}

fn expect_run(rest: &[Tok]) -> Expect {
    match rest.first().map(word) {
        None => Expect::opts(
            Some(&RUN),
            vec![
                opt("migration", "migraciones de la base"),
                opt("seed", "seeds (datos iniciales)"),
                opt("dev", "servidor de desarrollo (todavía no)"),
            ],
        ),
        Some(Some("migration")) => expect_migration(&rest[1..]),
        Some(Some("seed")) => expect_seed(&rest[1..]),
        Some(Some(t)) => Expect::err(Some(&RUN), format!("`run {t}` todavía no está implementado")),
        Some(None) => Expect::err(Some(&RUN), "falta qué correr: migration o seed"),
    }
}

fn expect_migration(rest: &[Tok]) -> Expect {
    let (action, topic, start): (&str, &'static Topic, usize) = match rest.first() {
        None => {
            return Expect::opts(
                Some(&MIGRATE),
                vec![
                    opt("rollback", "revierte el último batch o las últimas N"),
                    opt("status", "estado de migraciones y seeds"),
                    opt("list", "elegir de una lista cuáles correr"),
                    Opt {
                        word: "[",
                        desc: "solo esas migraciones: [m<id>, -m<id>]",
                        list: Some(IdKind::Migrations),
                    },
                    opt("seed", "después corre los seeds pendientes"),
                    opt("refresh", "revierte todas y las vuelve a ejecutar"),
                    opt("reset", "revierte todas"),
                    opt("fresh", "BORRA las colecciones y ejecuta todas"),
                    opt("unlock", "libera el lock"),
                ],
            );
        }
        Some(Tok::List(_)) => ("[]", &MIGRATION_IDS, 1),
        Some(Tok::Word(w)) => match w.as_str() {
            "list" => ("list", &MIGRATION_PICK, 1),
            "seed" => ("", &MIGRATE, 0),
            "rollback" => ("rollback", &ROLLBACK, 1),
            "reset" => ("reset", &RESET, 1),
            "refresh" => ("refresh", &REFRESH, 1),
            "fresh" => ("fresh", &FRESH, 1),
            "status" => ("status", &STATUS, 1),
            "unlock" => ("unlock", &UNLOCK, 1),
            other => return Expect::err(Some(&MIGRATE), format!("acción desconocida: migration {other}")),
        },
    };
    let topic = Some(topic);
    let (mut force, mut step, mut json) = (false, false, false);
    let mut i = start;
    while i < rest.len() {
        let Some(w) = word(&rest[i]) else {
            return Expect::err(topic, "la lista va justo después de migration, o de seed");
        };
        match w {
            "force" if matches!(action, "reset" | "refresh" | "fresh") && !force => force = true,
            "force" if force => return Expect::err(topic, "force repetido"),
            "force" => return Expect::err(topic, "force solo aplica a reset, refresh y fresh"),
            "step" if action == "rollback" && !step => {
                step = true;
                match rest.get(i + 1) {
                    None => {
                        return Expect {
                            topic,
                            expecting: Some("N: cuántas migraciones revertir"),
                            ..Expect::default()
                        };
                    }
                    Some(Tok::Word(n)) if n.parse::<usize>().is_ok_and(|n| n > 0) => i += 1,
                    Some(Tok::Word(n)) => return Expect::err(topic, format!("step inválido: {n}")),
                    Some(Tok::List(_)) => return Expect::err(topic, "step requiere un número: rollback step 2"),
                }
            }
            "step" if step => return Expect::err(topic, "step repetido"),
            "step" => return Expect::err(topic, "step solo aplica a rollback"),
            "json" if action == "status" && !json => json = true,
            "json" if json => return Expect::err(topic, "json repetido"),
            "json" => return Expect::err(topic, "json solo aplica a status"),
            "seed" if matches!(action, "" | "refresh" | "fresh" | "[]" | "list") => {
                return expect_seed_tail(&rest[i + 1..]);
            }
            "seed" => return Expect::err(topic, format!("seed no aplica a {action}")),
            other if action.is_empty() => return Expect::err(topic, format!("opción desconocida: {other}")),
            other => return Expect::err(topic, format!("opción desconocida para {action}: {other}")),
        }
        i += 1;
    }

    let mut opts = Vec::new();
    match action {
        "rollback" if !step => opts.push(opt("step", "cuántas revertir: step 2")),
        "reset" | "refresh" | "fresh" if !force => opts.push(opt("force", "sin pedir confirmación")),
        "status" if !json => opts.push(opt("json", "salida en JSON")),
        _ => {}
    }
    if matches!(action, "" | "refresh" | "fresh" | "[]" | "list") {
        opts.push(opt("seed", "después corre seeds"));
    }
    Expect::opts(topic, opts)
}

fn expect_seed_tail(rest: &[Tok]) -> Expect {
    let topic = Some(&SEED_TAIL);
    match rest.first() {
        None => Expect::opts(
            topic,
            vec![
                Opt {
                    word: "[",
                    desc: "solo esos seeds: [s<id>, ...]",
                    list: Some(IdKind::Seeds),
                },
                opt("list", "elegir de una lista"),
            ],
        ),
        Some(Tok::List(_)) | Some(Tok::Word(_)) if rest.len() > 1 => Expect::err(topic, "seed va al final"),
        Some(Tok::List(_)) => Expect::opts(topic, Vec::new()),
        Some(Tok::Word(w)) if w == "list" => Expect::opts(topic, Vec::new()),
        Some(Tok::Word(w)) => Expect::err(topic, format!("después de seed solo va una lista [s<id>, ...], no {w}")),
    }
}

fn expect_seed(rest: &[Tok]) -> Expect {
    let (mut list, mut pick, mut force) = (false, false, false);
    for t in rest {
        match t {
            Tok::List(_) if !list => list = true,
            Tok::Word(w) if w == "list" && !list => {
                list = true;
                pick = true;
            }
            Tok::List(_) => return Expect::err(Some(&SEED_IDS), "seed admite una sola lista"),
            Tok::Word(w) if w == "list" => return Expect::err(Some(&SEED_PICK), "seed admite una sola lista"),
            Tok::Word(w) if w == "force" && !force => force = true,
            Tok::Word(w) if w == "force" => return Expect::err(Some(&SEED_IDS), "force repetido"),
            Tok::Word(w) => return Expect::err(Some(&SEED), format!("opción desconocida para seed: {w}")),
        }
    }
    let topic: &'static Topic = match (list, pick) {
        (false, _) => &SEED,
        (true, false) => &SEED_IDS,
        (true, true) => &SEED_PICK,
    };
    let mut out = Expect::opts(Some(topic), Vec::new());
    if !list {
        out.opts.push(Opt {
            word: "[",
            desc: "solo esos seeds: [s<id>, ...]",
            list: Some(IdKind::Seeds),
        });
        out.opts.push(opt("list", "elegir de una lista"));
        if force {
            out.expecting = Some("force solo aplica con una lista de seeds");
        }
    } else if !force {
        out.opts.push(opt("force", "sin pedir confirmación"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrate::{MigrationRow, SeedRow};

    fn catalog() -> Status {
        let m = |id: &str, applied: bool| MigrationRow {
            id: id.into(),
            collection: Some("users".into()),
            applied,
            last_op: None,
            batch: None,
            in_code: true,
        };
        Status {
            migrations: vec![m("m261122_1401_create_users", true), m("m261123_0900_add_posts", false)],
            seeds: vec![SeedRow {
                id: "s261122_1501_users_admin".into(),
                status: None,
                attempts: 0,
                only_dev: true,
                in_code: true,
            }],
        }
    }

    fn labels(a: &Analysis) -> Vec<&str> {
        a.suggestions.iter().map(|s| s.label.as_str()).collect()
    }

    #[test]
    fn palabras_segun_posicion() {
        assert_eq!(labels(&analyze("r", None)), ["run"]);
        assert_eq!(labels(&analyze("git p", None)), ["push", "pull"]);
        assert_eq!(labels(&analyze("make ", None)), ["model", "migration", "seed"]);
        let a = analyze("make migration ", None);
        assert_eq!(a.topic, Some(&MAKE_MIGRATION));
        assert!(a.expecting.unwrap().starts_with("nombre"));
        assert_eq!(labels(&analyze("make model User ", None)), ["--fields"]);
        assert_eq!(analyze("make seed a b ", None).error.as_deref(), Some("sobra «b»"));
        assert!(analyze("editar src/main.rs ", None).error.is_none());
        let a = analyze("git commit -m \"hola mundo\" ", None);
        assert!(a.error.is_none() && a.suggestions.is_empty());
        assert_eq!(a.topic, Some(&GIT));
        assert_eq!(labels(&analyze("run ", None)), ["migration", "seed", "dev"]);
        let a = analyze("run migration r", None);
        assert_eq!(labels(&a), ["rollback", "refresh", "reset"]);
        assert_eq!(a.topic, Some(&MIGRATE));
        assert_eq!(a.suggestions[0].from, 14);
        assert_eq!(a.suggestions[0].insert, "rollback ");

        let a = analyze("run migration rollback ", None);
        assert_eq!(a.topic, Some(&ROLLBACK));
        assert_eq!(labels(&a), ["step"]);
        let a = analyze("run migration rollback step ", None);
        assert!(a.suggestions.is_empty() && a.expecting.is_some());
        assert_eq!(labels(&analyze("run migration fresh force ", None)), ["seed"]);
        assert_eq!(labels(&analyze("run migration fresh seed ", None)), ["[", "list"]);
        assert_eq!(labels(&analyze("run seed list ", None)), ["force"]);
    }

    #[test]
    fn errores_como_migrate() {
        let err = |s: &str| analyze(s, None).error.unwrap_or_default();
        assert_eq!(
            err("run migration rollback force "),
            "force solo aplica a reset, refresh y fresh"
        );
        assert_eq!(err("run migration status seed "), "seed no aplica a status");
        assert_eq!(err("run migration rollback step x "), "step inválido: x");
        assert_eq!(err("run migration fresh seed list x "), "seed va al final");
        assert_eq!(err("run dev "), "`run dev` todavía no está implementado");
        assert_eq!(err("nada "), "comando desconocido: nada");
        // Una palabra a medias no es error todavía.
        assert!(analyze("run migration rollbac", None).error.is_none());
    }

    #[test]
    fn ids_reales_dentro_de_la_lista() {
        let st = catalog();
        // Ejecutada → se sugiere como down; pendiente → up.
        let a = analyze("run migration [", Some(&st));
        assert_eq!(labels(&a), ["-m261122_1401_create_users", "m261123_0900_add_posts"]);
        assert_eq!(a.topic, Some(&MIGRATION_IDS));
        // Busca por cualquier parte del id y no repite los ya puestos.
        let a = analyze("run migration [-m261122_1401_create_users, post", Some(&st));
        assert_eq!(labels(&a), ["m261123_0900_add_posts"]);
        assert_eq!(a.suggestions[0].from, 43);
        // Lista con algo: ofrece cerrar, quitando el ", " final.
        let text = "run migration [m261123_0900_add_posts, ";
        let a = analyze(text, Some(&st));
        assert_eq!(a.suggestions[0].label, "]");
        assert_eq!(&text[..a.suggestions[0].from], "run migration [m261123_0900_add_posts");
        // Seeds después de migration.
        assert_eq!(
            labels(&analyze("run migration seed [", Some(&st))),
            ["s261122_1501_users_admin"]
        );
        // Sin estado: no hay IDs pero se explica.
        assert!(analyze("run seed [", None).expecting.is_some());
        // Una lista donde no va.
        assert!(analyze("run migration rollback [", Some(&st)).error.is_some());
    }

    #[test]
    fn busca_temas() {
        assert_eq!(search("rollback"), [&ROLLBACK]);
        assert!(search("force").len() >= 4);
    }
}
