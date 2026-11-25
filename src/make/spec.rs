//! Campos de un modelo escritos con la misma sintaxis del Blueprint de las
//! migraciones, una llamada por línea (sin `col.` y sin `;`):
//!
//! ```text
//! id()
//! string("name").len(120).index()
//! string("email").unique(UniqueType::Normal)
//! int("age").len(16).unsigned().optional()
//! enum_("type_document", &["CC", "CE", "NIT"])
//! timestamps()
//! ```
//!
//! Cada línea se copia tal cual a la migración; de ella también sale el
//! campo del modelo (tipo de Rust según el tipo y `Option` con
//! `nullable()`/`optional()`).

use std::collections::BTreeSet;

/// Una línea de la especificación ya separada en partes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldLine {
    /// Número de línea (desde 1) en el texto original.
    pub line: usize,
    /// El método del Blueprint: `string`, `timestamps`, `index`...
    pub method: String,
    /// Argumentos del método, tal como se escribieron.
    pub args: Vec<String>,
    /// Modificadores encadenados: (`len`, ["120"]), (`index`, []).
    pub mods: Vec<(String, Vec<String>)>,
    /// La llamada normalizada, lista para `col.{code};`.
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecError {
    pub line: usize,
    pub message: String,
}

/// Métodos de campo que van al modelo.
pub const FIELD_METHODS: &[&str] = &[
    "id",
    "string",
    "char",
    "text",
    "int",
    "float",
    "double",
    "decimal",
    "boolean",
    "date",
    "date_time",
    "timestamp",
    "uuid",
    "binary",
    "json",
    "object",
    "array",
    "array_of",
    "point",
    "enum_",
    "foreign_id",
    "morphs",
    "timestamps",
    "soft_deletes",
];

/// Índices a nivel colección: van a la migración pero no al modelo.
pub const INDEX_METHODS: &[&str] = &["index", "unique", "full_text", "hashed", "geo", "ttl", "wildcard"];

/// Métodos sin nombre de campo.
const NO_NAME: &[&str] = &["id", "timestamps", "soft_deletes"];

/// Pasa los `;` de primer nivel a saltos de línea (para `--fields "a; b"`),
/// sin tocar los que están dentro de strings, paréntesis o closures.
pub fn split_statements(text: &str) -> String {
    let mut out = String::new();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut escaped = false;
    for c in text.chars() {
        match c {
            _ if escaped => escaped = false,
            '\\' if in_str => escaped = true,
            '"' => in_str = !in_str,
            '(' | '[' | '{' if !in_str => depth += 1,
            ')' | ']' | '}' if !in_str => depth -= 1,
            ';' if !in_str && depth == 0 => {
                out.push('\n');
                continue;
            }
            _ => {}
        }
        out.push(c);
    }
    out
}

pub fn parse(text: &str) -> Result<Vec<FieldLine>, Vec<SpecError>> {
    let mut lines = Vec::new();
    let mut errors = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = i + 1;
        let t = raw.trim();
        if t.is_empty() || t.starts_with("//") || t.starts_with('#') {
            continue;
        }
        match parse_line(t, line) {
            Ok(f) => lines.push(f),
            Err(message) => errors.push(SpecError { line, message }),
        }
    }
    if errors.is_empty() { Ok(lines) } else { Err(errors) }
}

fn parse_line(text: &str, line: usize) -> Result<FieldLine, String> {
    let t = text.trim_end_matches(';').trim();
    let t = t.strip_prefix("col.").unwrap_or(t);
    let calls = split_chain(t)?;
    let mut calls = calls.into_iter();
    let Some((mut method, args)) = calls.next() else {
        return Err("línea vacía".into());
    };
    if method == "enum" {
        method = "enum_".into(); // `enum` es palabra reservada en Rust
    }
    if !FIELD_METHODS.contains(&method.as_str()) && !INDEX_METHODS.contains(&method.as_str()) {
        let hint = closest(&method).map(|m| format!(" (¿{m}?)")).unwrap_or_default();
        return Err(format!("tipo desconocido: {method}{hint}"));
    }
    if NO_NAME.contains(&method.as_str()) {
        if !args.is_empty() {
            return Err(format!("{method}() no lleva argumentos"));
        }
    } else {
        match args.first() {
            Some(a) if string_literal(a).is_some() => {}
            Some(a) => {
                return Err(format!(
                    "el primer argumento de {method} va entre comillas: {method}(\"{a}\")"
                ));
            }
            None => return Err(format!("falta el nombre del campo: {method}(\"nombre\")")),
        }
    }
    if let Some(name) = args.first().and_then(|a| string_literal(a))
        && FIELD_METHODS.contains(&method.as_str())
        && !valid_ident(&name)
    {
        return Err(format!("«{name}» no sirve como nombre de campo (letras, números y _)"));
    }
    let needs_second = matches!(method.as_str(), "object" | "array" | "array_of" | "enum_");
    if needs_second && args.len() < 2 {
        let example = match method.as_str() {
            "object" => "object(\"perfil\", |p| { p.string(\"ciudad\"); })",
            "array" => "array(\"tags\", Item::String)",
            "array_of" => "array_of(\"items\", |i| { i.string(\"sku\"); })",
            _ => "enum_(\"tipo\", &[\"A\", \"B\"])",
        };
        return Err(format!("a {method} le falta el segundo argumento: {example}"));
    }
    let mods: Vec<(String, Vec<String>)> = calls.collect();
    let mut code = format!("{method}({})", args.join(", "));
    for (m, a) in &mods {
        code.push_str(&format!(".{m}({})", a.join(", ")));
    }
    Ok(FieldLine {
        line,
        method,
        args,
        mods,
        code,
    })
}

/// `a(x).b().c(1, 2)` → [(a, [x]), (b, []), (c, [1, 2])].
fn split_chain(t: &str) -> Result<Vec<(String, Vec<String>)>, String> {
    let chars: Vec<char> = t.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    loop {
        let start = i;
        while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
            i += 1;
        }
        let name: String = chars[start..i].iter().collect();
        if name.is_empty() {
            let rest: String = chars[i..].iter().collect();
            return Err(format!("se esperaba un método y vino «{rest}»"));
        }
        if chars.get(i) != Some(&'(') {
            return Err(format!("falta ( después de {name}"));
        }
        let (args, end) = split_args(&chars, i + 1)?;
        out.push((name, args));
        i = end + 1;
        match chars.get(i) {
            None => return Ok(out),
            Some('.') => i += 1,
            Some(c) => {
                return Err(format!(
                    "sobra «{c}…» después de {})",
                    out.last().map_or("", |c| c.0.as_str())
                ));
            }
        }
    }
}

/// Argumentos desde `start` (justo después de `(`) hasta su `)`. Respeta
/// strings y anidamiento de (), [] y {}. Devuelve también la posición de `)`.
fn split_args(chars: &[char], start: usize) -> Result<(Vec<String>, usize), String> {
    let mut args = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut i = start;
    while i < chars.len() {
        let c = chars[i];
        if in_str {
            cur.push(c);
            if c == '\\' {
                if let Some(&n) = chars.get(i + 1) {
                    cur.push(n);
                    i += 1;
                }
            } else if c == '"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match c {
            '"' => {
                in_str = true;
                cur.push(c);
            }
            '(' | '[' | '{' => {
                depth += 1;
                cur.push(c);
            }
            ')' if depth == 0 => {
                let last = cur.trim().to_string();
                if !last.is_empty() {
                    args.push(last);
                } else if !args.is_empty() {
                    return Err("sobra una coma antes de )".into());
                }
                return Ok((args, i));
            }
            ')' | ']' | '}' => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 => {
                args.push(cur.trim().to_string());
                cur.clear();
            }
            c => cur.push(c),
        }
        i += 1;
    }
    if in_str {
        Err("falta cerrar las comillas".into())
    } else {
        Err("paréntesis sin cerrar (escribe cada campo completo en una línea)".into())
    }
}

/// El contenido de `"texto"`, si el argumento es un string literal simple.
pub fn string_literal(arg: &str) -> Option<String> {
    let inner = arg.strip_prefix('"')?.strip_suffix('"')?;
    (!inner.contains('"')).then(|| inner.to_string())
}

fn valid_ident(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// El método conocido más parecido (por prefijo común), para sugerirlo.
fn closest(method: &str) -> Option<&'static str> {
    FIELD_METHODS
        .iter()
        .chain(INDEX_METHODS)
        .copied()
        .map(|m| (m.chars().zip(method.chars()).take_while(|(a, b)| a == b).count(), m))
        .filter(|(n, _)| *n >= 2)
        .max_by_key(|(n, _)| *n)
        .map(|(_, m)| m)
}

// ---------------------------------------------------------------------------
// Modelo
// ---------------------------------------------------------------------------

/// Un campo del struct del modelo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelField {
    /// Nombre en Mongo.
    pub name: String,
    /// Tipo de Rust (ya con `Option<..>` si corresponde).
    pub ty: String,
    /// Atributos `#[serde(..)]` y comentarios, una línea cada uno.
    pub attrs: Vec<String>,
}

/// Campos del modelo e imports de `mongodb::bson` que usan.
pub struct Model {
    pub fields: Vec<ModelField>,
    pub bson: BTreeSet<&'static str>,
    /// Avisos para mostrar (ej. se agregó `id`).
    pub notes: Vec<String>,
}

pub fn model(lines: &[FieldLine]) -> Model {
    let mut m = Model {
        fields: Vec::new(),
        bson: BTreeSet::new(),
        notes: Vec::new(),
    };
    if !lines.iter().any(|l| l.method == "id") {
        m.notes.push("no hay id(): el modelo lleva igual el campo _id".into());
    }
    m.bson.insert("oid::ObjectId");
    m.fields.push(ModelField {
        name: "id".into(),
        ty: "Option<ObjectId>".into(),
        attrs: vec!["#[serde(rename = \"_id\", skip_serializing_if = \"Option::is_none\")]".into()],
    });

    for l in lines {
        let name = l.args.first().and_then(|a| string_literal(a)).unwrap_or_default();
        let has = |m: &str| l.mods.iter().any(|(n, _)| n == m);
        let optional = has("optional");
        let nullable = has("nullable");
        let date = |m: &mut Model| {
            m.bson.insert("DateTime");
            "DateTime".to_string()
        };
        let ty = match l.method.as_str() {
            "id" => continue,
            "timestamps" => {
                let t = date(&mut m);
                for n in ["created_at", "updated_at"] {
                    m.fields.push(ModelField {
                        name: n.into(),
                        ty: t.clone(),
                        attrs: vec![],
                    });
                }
                continue;
            }
            "soft_deletes" => {
                date(&mut m);
                m.fields.push(ModelField {
                    name: "deleted_at".into(),
                    ty: "Option<DateTime>".into(),
                    attrs: vec!["#[serde(default, skip_serializing_if = \"Option::is_none\")]".into()],
                });
                continue;
            }
            "morphs" => {
                m.bson.insert("oid::ObjectId");
                m.fields.push(ModelField {
                    name: format!("{name}_id"),
                    ty: "ObjectId".into(),
                    attrs: vec![],
                });
                m.fields.push(ModelField {
                    name: format!("{name}_type"),
                    ty: "String".into(),
                    attrs: vec![],
                });
                continue;
            }
            "string" | "char" | "text" | "enum_" => "String".to_string(),
            "int" => int_type(l),
            "float" | "double" => "f64".into(),
            "decimal" => {
                m.bson.insert("Decimal128");
                "Decimal128".into()
            }
            "boolean" => "bool".into(),
            "date" | "date_time" | "timestamp" => date(&mut m),
            "uuid" => {
                m.bson.insert("Uuid");
                "Uuid".into()
            }
            "binary" => {
                m.bson.insert("Binary");
                "Binary".into()
            }
            "json" | "object" | "point" => {
                m.bson.insert("Document");
                "Document".into()
            }
            "foreign_id" => "ObjectId".into(),
            "array" => {
                let item = l.args.get(1).map(String::as_str).unwrap_or_default();
                let (ty, import) = item_type(item);
                if let Some(i) = import {
                    m.bson.insert(i);
                }
                format!("Vec<{ty}>")
            }
            "array_of" => {
                m.bson.insert("Document");
                "Vec<Document>".into()
            }
            _ => continue, // índices de colección
        };
        let mut attrs = Vec::new();
        if l.method == "object" || l.method == "array_of" {
            attrs.push("// Sub-documento: se puede cambiar por un struct propio con Serialize/Deserialize.".into());
        }
        let ty = if optional {
            attrs.push("#[serde(default, skip_serializing_if = \"Option::is_none\")]".into());
            format!("Option<{ty}>")
        } else if nullable {
            format!("Option<{ty}>")
        } else {
            ty
        };
        m.fields.push(ModelField { name, ty, attrs });
    }
    m
}

/// `int` según `.len(n)` y `.unsigned()` (ver migraciones.md: 8/16 = int con
/// límites, 32 = int, 64 = long; 32 unsigned se guarda como long).
fn int_type(l: &FieldLine) -> String {
    let len = l
        .mods
        .iter()
        .find(|(n, _)| n == "len")
        .and_then(|(_, a)| a.first())
        .and_then(|a| a.parse::<u8>().ok())
        .unwrap_or(32);
    let unsigned = l.mods.iter().any(|(n, _)| n == "unsigned");
    match (len, unsigned) {
        (8, false) => "i8",
        (16, false) => "i16",
        (8, true) => "u8",
        (16, true) => "u16",
        (32, true) => "u32",
        (64, _) => "i64",
        _ => "i32",
    }
    .into()
}

fn item_type(item: &str) -> (&'static str, Option<&'static str>) {
    match item.trim_start_matches("Item::") {
        "Int" => ("i32", None),
        "Long" => ("i64", None),
        "Double" => ("f64", None),
        "Decimal" => ("Decimal128", Some("Decimal128")),
        "Boolean" => ("bool", None),
        "Date" => ("DateTime", Some("DateTime")),
        "ObjectId" => ("ObjectId", None),
        "Uuid" => ("Uuid", Some("Uuid")),
        "Json" => ("Document", Some("Document")),
        _ => ("String", None),
    }
}

/// Nombre de campo de Rust: las palabras reservadas van como `r#tipo`
/// (serde las serializa sin el `r#`).
pub fn rust_ident(name: &str) -> String {
    const KEYWORDS: &[&str] = &[
        "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn",
        "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "static",
        "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while", "yield",
    ];
    if KEYWORDS.contains(&name) {
        format!("r#{name}")
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: &str = r#"
        id()
        string("name").len(120).index()
        col.string("email").unique(UniqueType::Normal);
        string("phone").optional().unique(UniqueType::Sparse)
        enum("type", &["CC", "CE", "NIT"])
        int("age").len(16).unsigned().nullable()
        array("tags", Item::String)
        object("profile", |p| { p.string("city").optional(); })
        foreign_id("company_id").index()
        unique("type,email", UniqueType::Normal)
        // comentario
        timestamps()
    "#;

    #[test]
    fn separa_cada_linea() {
        let lines = parse(SPEC).unwrap();
        assert_eq!(lines.len(), 11);
        assert_eq!(lines[2].code, "string(\"email\").unique(UniqueType::Normal)");
        assert_eq!(lines[4].method, "enum_");
        assert_eq!(lines[4].code, "enum_(\"type\", &[\"CC\", \"CE\", \"NIT\"])");
        assert_eq!(lines[7].args[1], "|p| { p.string(\"city\").optional(); }");
        assert_eq!(lines[5].mods[0], ("len".to_string(), vec!["16".to_string()]));
    }

    #[test]
    fn campos_del_modelo() {
        let m = model(&parse(SPEC).unwrap());
        let f: Vec<(String, String)> = m.fields.iter().map(|f| (f.name.clone(), f.ty.clone())).collect();
        let f: Vec<(&str, &str)> = f.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        assert_eq!(
            f,
            [
                ("id", "Option<ObjectId>"),
                ("name", "String"),
                ("email", "String"),
                ("phone", "Option<String>"),
                ("type", "String"),
                ("age", "Option<u16>"),
                ("tags", "Vec<String>"),
                ("profile", "Document"),
                ("company_id", "ObjectId"),
                ("created_at", "DateTime"),
                ("updated_at", "DateTime"),
            ]
        );
        assert!(m.bson.contains("DateTime") && m.bson.contains("Document"));
        assert!(m.notes.is_empty());
        assert_eq!(rust_ident("type"), "r#type");
    }

    #[test]
    fn separa_por_punto_y_coma_de_primer_nivel() {
        let s = split_statements(r#"string("a;b"); object("p", |p| { p.string("c"); }); id()"#);
        assert_eq!(s, "string(\"a;b\")\n object(\"p\", |p| { p.string(\"c\"); })\n id()");
    }

    #[test]
    fn errores_con_linea() {
        let errs = parse("id()\nstrng(\"x\")\nstring(name)\nint(\"a\"\nstring(\"a b\")\narray(\"t\")").unwrap_err();
        let msgs: Vec<(usize, &str)> = errs.iter().map(|e| (e.line, e.message.as_str())).collect();
        assert_eq!(msgs[0], (2, "tipo desconocido: strng (¿string?)"));
        assert_eq!(
            msgs[1],
            (3, "el primer argumento de string va entre comillas: string(\"name\")")
        );
        assert!(msgs[2].1.starts_with("paréntesis sin cerrar"));
        assert!(msgs[3].1.contains("no sirve como nombre"));
        assert!(msgs[4].1.contains("segundo argumento"));
    }
}
