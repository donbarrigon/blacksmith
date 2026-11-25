//! Mantiene los `mod.rs` que listan migraciones y seeds
//! (`fn migrations() -> Vec<Box<dyn DynMigration>>`) y el de modelos.

use std::path::Path;

use anyhow::{Context, bail};

/// Qué lista se actualiza.
#[derive(Debug, Clone, Copy)]
pub enum List {
    Migrations,
    Seeds,
}

impl List {
    fn fn_name(self) -> &'static str {
        match self {
            List::Migrations => "migrations",
            List::Seeds => "seeds",
        }
    }

    fn header(self) -> String {
        let (what, cmd, ty) = match self {
            List::Migrations => ("migraciones", "make migration / make model", "DynMigration"),
            List::Seeds => ("seeds", "make seed", "DynSeeder"),
        };
        format!(
            "//! Lista de {what}. La mantiene blacksmith ({cmd}): cada archivo nuevo se\n\
             //! agrega como `mod` y en el `vec!`, en orden de id.\n\n\
             use ironforge::migration::{ty};\n"
        )
    }

    fn ty(self) -> &'static str {
        match self {
            List::Migrations => "DynMigration",
            List::Seeds => "DynSeeder",
        }
    }
}

/// Agrega `mod {module};` y `Box::new({module}::{struct_name})` al `mod.rs`.
/// Si no existe, lo crea.
pub fn register(mod_rs: &Path, list: List, module: &str, struct_name: &str) -> anyhow::Result<()> {
    let text = if mod_rs.exists() {
        std::fs::read_to_string(mod_rs).with_context(|| format!("no se pudo leer {}", mod_rs.display()))?
    } else {
        format!(
            "{}\npub fn {}() -> Vec<Box<dyn {}>> {{\n    vec![]\n}}\n",
            list.header(),
            list.fn_name(),
            list.ty()
        )
    };
    let updated = add_entry(&text, list, module, struct_name).with_context(|| {
        format!(
            "no pude actualizar {}; agrega a mano `mod {module};` y `Box::new({module}::{struct_name}),` en el vec!",
            mod_rs.display()
        )
    })?;
    std::fs::write(mod_rs, updated).with_context(|| format!("no se pudo escribir {}", mod_rs.display()))
}

fn add_entry(text: &str, list: List, module: &str, struct_name: &str) -> anyhow::Result<String> {
    let mod_line = format!("mod {module};");
    if text
        .lines()
        .any(|l| l.trim() == mod_line || l.trim() == format!("pub {mod_line}"))
    {
        bail!("{module} ya está registrado");
    }

    // 1) `mod ...;` después del último `mod`, o antes de `pub fn`.
    let lines: Vec<&str> = text.lines().collect();
    let last_mod = lines.iter().rposition(|l| {
        let t = l.trim_start().trim_start_matches("pub ");
        t.starts_with("mod ") && t.ends_with(';')
    });
    let fn_sig = format!("fn {}()", list.fn_name());
    let fn_line = lines
        .iter()
        .position(|l| l.contains(&fn_sig))
        .context("no encontré la función")?;
    let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    match last_mod {
        Some(i) => out.insert(i + 1, mod_line),
        None => {
            out.insert(fn_line, String::new());
            out.insert(fn_line, mod_line);
        }
    }
    let text = out.join("\n") + "\n";

    // 2) `Box::new(..)` antes del `]` que cierra el `vec![` de la función.
    let fn_at = text.find(&fn_sig).context("no encontré la función")?;
    let open = fn_at + text[fn_at..].find("vec![").context("no encontré vec![")? + "vec![".len();
    let mut depth = 0;
    let mut close = None;
    for (i, c) in text[open..].char_indices() {
        match c {
            '[' | '(' | '{' => depth += 1,
            ']' if depth == 0 => {
                close = Some(open + i);
                break;
            }
            ']' | ')' | '}' => depth -= 1,
            _ => {}
        }
    }
    let close = close.context("no encontré el ] del vec!")?;
    let inner = text[open..close].trim_end();
    let entry = format!("Box::new({module}::{struct_name}),");
    let new_inner = if inner.trim().is_empty() {
        format!("\n        {entry}\n    ")
    } else {
        let sep = if inner.ends_with(',') { "" } else { "," };
        format!("{inner}{sep}\n        {entry}\n    ")
    };
    Ok(format!("{}{}{}", &text[..open], new_inner, &text[close..]))
}

/// Agrega `pub mod {module};` a `src/models/mod.rs` (lo crea si falta).
/// Devuelve un aviso si el módulo `models` no está declarado en la app.
pub fn register_model(root: &Path, module: &str) -> anyhow::Result<Option<String>> {
    let dir = root.join("src/models");
    let mod_rs = dir.join("mod.rs");
    let mut text = std::fs::read_to_string(&mod_rs).unwrap_or_default();
    let line = format!("pub mod {module};");
    if !text.lines().any(|l| l.trim() == line) {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&line);
        text.push('\n');
        std::fs::write(&mod_rs, text).with_context(|| format!("no se pudo escribir {}", mod_rs.display()))?;
    }
    let declared = ["src/main.rs", "src/lib.rs"].iter().any(|f| {
        std::fs::read_to_string(root.join(f))
            .is_ok_and(|t| t.lines().any(|l| matches!(l.trim(), "mod models;" | "pub mod models;")))
    });
    Ok((!declared).then(|| "agrega `pub mod models;` en src/lib.rs (o `mod models;` en src/main.rs)".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crea_y_agrega() {
        let empty = format!(
            "{}\npub fn migrations() -> Vec<Box<dyn DynMigration>> {{\n    vec![]\n}}\n",
            List::Migrations.header()
        );
        let one = add_entry(&empty, List::Migrations, "m1_a", "A").unwrap();
        assert!(
            one.contains("use ironforge::migration::DynMigration;\n\nmod m1_a;\n\npub fn migrations()"),
            "{one}"
        );
        assert!(one.contains("    vec![\n        Box::new(m1_a::A),\n    ]\n"), "{one}");

        let two = add_entry(&one, List::Migrations, "m2_b", "B").unwrap();
        assert!(two.contains("mod m1_a;\nmod m2_b;\n"), "{two}");
        assert!(
            two.contains("        Box::new(m1_a::A),\n        Box::new(m2_b::B),\n    ]"),
            "{two}"
        );
        assert!(add_entry(&two, List::Migrations, "m2_b", "B").is_err());

        // Escrito a mano, sin coma final.
        let hand = "use x;\nmod s1_a;\npub fn seeds() -> Vec<Box<dyn DynSeeder>> {\n    vec![Box::new(s1_a::A)]\n}\n";
        let out = add_entry(hand, List::Seeds, "s2_b", "B").unwrap();
        assert!(
            out.contains("vec![Box::new(s1_a::A),\n        Box::new(s2_b::B),\n    ]"),
            "{out}"
        );
    }
}
