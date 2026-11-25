//! `blacksmith make ...`: crea migraciones, seeds y modelos con el formato
//! de ironforge (`.claude/rules/migraciones.md` en ironforge):
//!
//! - id `yymmdd_hhNN_nombre` (NN = consecutivo dentro de la hora)
//! - archivos `m<id>.rs` / `s<id>.rs` en `src/bin/migrate/{migrations,seeds}/`
//! - registrados en el `mod.rs` de cada carpeta (los mantiene blacksmith)
//! - modelos en `src/models/<nombre>.rs`

pub mod registry;
pub mod render;
pub mod spec;

use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use chrono::Local;

use crate::project::MIGRATE_DIR;
use registry::List;

/// Lo que se creó, para mostrarlo.
#[derive(Debug, Default)]
pub struct Created {
    /// Rutas relativas a la raíz del proyecto.
    pub files: Vec<PathBuf>,
    pub notes: Vec<String>,
}

/// Lo que se va a crear con `make model`, para mostrarlo antes de crearlo.
pub struct ModelPlan {
    pub migration_id: String,
    pub migration_module: String,
    pub migration_path: PathBuf,
    pub migration_code: String,
    pub model_path: PathBuf,
    pub model_code: String,
    pub notes: Vec<String>,
}

// ---------------------------------------------------------------------------
// Nombres
// ---------------------------------------------------------------------------

/// `Crear Users` / `create-users` → `create_users`. Error si queda vacío o
/// no empieza con letra.
pub fn snake_name(name: &str) -> anyhow::Result<String> {
    let mut out = String::new();
    let mut prev_lower = false;
    for c in name.trim().chars() {
        if c.is_ascii_uppercase() {
            if prev_lower {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
            prev_lower = false;
        } else if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
        } else if matches!(c, '_' | '-' | ' ') {
            if !out.ends_with('_') && !out.is_empty() {
                out.push('_');
            }
            prev_lower = false;
        } else {
            bail!("«{name}» tiene caracteres no permitidos (usa letras, números y _)");
        }
    }
    let out = out.trim_end_matches('_').to_string();
    if !out.starts_with(|c: char| c.is_ascii_lowercase()) {
        bail!("el nombre tiene que empezar con una letra: «{name}»");
    }
    Ok(out)
}

/// `create_users` → `CreateUsers`.
pub fn pascal(snake: &str) -> String {
    snake
        .split('_')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut c = p.chars();
            c.next()
                .map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
                .unwrap_or_default()
        })
        .collect()
}

/// Plural simple en inglés para el nombre de la colección: `user` → `users`,
/// `category` → `categories`, `box` → `boxes`.
pub fn plural(snake: &str) -> String {
    let vowel_before_y = snake.len() >= 2 && "aeiou".contains(&snake[snake.len() - 2..snake.len() - 1]);
    if snake.ends_with('y') && !vowel_before_y {
        format!("{}ies", &snake[..snake.len() - 1])
    } else if ["s", "x", "z", "ch", "sh"].iter().any(|e| snake.ends_with(e)) {
        format!("{snake}es")
    } else {
        format!("{snake}s")
    }
}

/// Colección deducida del nombre de la migración, estilo Laravel:
/// `create_users` → users, `add_phone_to_users` → users.
pub fn collection_from_name(name: &str) -> Option<String> {
    if let Some(c) = name.strip_prefix("create_") {
        return Some(c.to_string());
    }
    for sep in ["_to_", "_from_", "_in_", "_on_"] {
        if let Some((_, c)) = name.rsplit_once(sep) {
            return Some(c.to_string());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Ids
// ---------------------------------------------------------------------------

/// Siguiente id para `name` en `dir` (archivos `{prefix}<id>.rs`), con la hora
/// local: `yymmdd_hhNN_name`.
fn next_id(dir: &Path, prefix: char, name: &str) -> String {
    let stamp = Local::now().format("%y%m%d_%H").to_string();
    let existing: Vec<String> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter_map(|e| e.file_name().into_string().ok())
                .filter_map(|f| f.strip_prefix(prefix).map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    next_id_from(&existing, &stamp, name)
}

fn next_id_from(existing: &[String], stamp: &str, name: &str) -> String {
    let max = existing
        .iter()
        .filter_map(|f| f.strip_prefix(stamp))
        .filter_map(|rest| rest.get(..2)?.parse::<u32>().ok())
        .max()
        .unwrap_or(0);
    format!("{stamp}{:02}_{name}", max + 1)
}

fn migrations_dir(root: &Path) -> PathBuf {
    root.join(MIGRATE_DIR).join("migrations")
}

fn seeds_dir(root: &Path) -> PathBuf {
    root.join(MIGRATE_DIR).join("seeds")
}

fn relative(root: &Path, p: &Path) -> PathBuf {
    p.strip_prefix(root).unwrap_or(p).to_path_buf()
}

fn write_new(path: &Path, content: &str) -> anyhow::Result<()> {
    if path.exists() {
        bail!("{} ya existe", path.display());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("no se pudo crear {}", dir.display()))?;
    }
    std::fs::write(path, content).with_context(|| format!("no se pudo escribir {}", path.display()))
}

// ---------------------------------------------------------------------------
// Comandos
// ---------------------------------------------------------------------------

/// `make migration <nombre> [colección]`. Si el nombre es `create_x` crea la
/// colección `x`; si no, deja una plantilla de `alter_collection`.
pub fn migration(root: &Path, name: &str, collection: Option<&str>) -> anyhow::Result<Created> {
    let name = snake_name(name)?;
    let coll = match collection {
        Some(c) => snake_name(c)?,
        None => collection_from_name(&name)
            .with_context(|| format!("no pude deducir la colección de «{name}»: make migration {name} <colección>"))?,
    };
    let dir = migrations_dir(root);
    let id = next_id(&dir, 'm', &name);
    let module = format!("m{id}");
    let struct_name = pascal(&name);
    let code = if name.starts_with("create_") {
        render::migration_create(&id, &struct_name, &coll, &[])
    } else {
        render::migration_alter(&id, &struct_name, &coll)
    };
    let path = dir.join(format!("{module}.rs"));
    write_new(&path, &code)?;
    registry::register(&dir.join("mod.rs"), List::Migrations, &module, &struct_name)?;
    Ok(Created {
        files: vec![relative(root, &path), relative(root, &dir.join("mod.rs"))],
        notes: Vec::new(),
    })
}

/// `make seed <nombre>`.
pub fn seed(root: &Path, name: &str) -> anyhow::Result<Created> {
    let name = snake_name(name)?;
    let dir = seeds_dir(root);
    let id = next_id(&dir, 's', &name);
    let module = format!("s{id}");
    let struct_name = pascal(&name);
    let path = dir.join(format!("{module}.rs"));
    write_new(&path, &render::seed(&id, &struct_name))?;
    registry::register(&dir.join("mod.rs"), List::Seeds, &module, &struct_name)?;
    Ok(Created {
        files: vec![relative(root, &path), relative(root, &dir.join("mod.rs"))],
        notes: Vec::new(),
    })
}

/// Arma (sin escribir nada) la migración y el modelo de `make model`.
/// `fields` usa la sintaxis de [`spec`]; vacío = `id()` + `timestamps()`.
pub fn plan_model(root: &Path, model: &str, collection: Option<&str>, fields: &str) -> Result<ModelPlan, Vec<String>> {
    let one = |e: anyhow::Error| vec![format!("{e:#}")];
    let snake = snake_name(model).map_err(one)?;
    let struct_name = pascal(&snake);
    let coll = match collection {
        Some(c) => snake_name(c).map_err(one)?,
        None => plural(&snake),
    };
    let fields = if fields.trim().is_empty() {
        "id()\ntimestamps()"
    } else {
        fields
    };
    let lines = spec::parse(fields).map_err(|errs| {
        errs.into_iter()
            .map(|e| format!("línea {}: {}", e.line, e.message))
            .collect::<Vec<_>>()
    })?;
    let model_spec = spec::model(&lines);

    let mig_name = format!("create_{coll}");
    let dir = migrations_dir(root);
    let id = next_id(&dir, 'm', &mig_name);
    let module = format!("m{id}");
    let model_path = root.join("src/models").join(format!("{snake}.rs"));
    let mut notes = model_spec.notes.clone();
    if model_path.exists() {
        return Err(vec![format!("{} ya existe", relative(root, &model_path).display())]);
    }
    if !root.join(MIGRATE_DIR).is_dir() {
        notes.push(format!("no existe {MIGRATE_DIR}: se crea la carpeta, falta su main.rs"));
    }
    Ok(ModelPlan {
        migration_code: render::migration_create(&id, &pascal(&mig_name), &coll, &lines),
        model_code: render::model(&struct_name, &coll, &module, &model_spec),
        migration_path: relative(root, &dir.join(format!("{module}.rs"))),
        migration_id: id,
        migration_module: module,
        model_path: relative(root, &model_path),
        notes,
    })
}

/// Escribe lo planeado por [`plan_model`].
pub fn write_model(root: &Path, plan: &ModelPlan) -> anyhow::Result<Created> {
    let mig_path = root.join(&plan.migration_path);
    write_new(&root.join(&plan.model_path), &plan.model_code)?;
    write_new(&mig_path, &plan.migration_code)?;
    let struct_name = pascal(plan.migration_id.splitn(3, '_').nth(2).unwrap_or_default());
    let mod_rs = migrations_dir(root).join("mod.rs");
    registry::register(&mod_rs, List::Migrations, &plan.migration_module, &struct_name)?;
    let model_module = plan.model_path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
    let mut notes = plan.notes.clone();
    notes.extend(registry::register_model(root, model_module)?);
    Ok(Created {
        files: vec![
            plan.model_path.clone(),
            PathBuf::from("src/models/mod.rs"),
            plan.migration_path.clone(),
            relative(root, &mod_rs),
        ],
        notes,
    })
}

/// `make model` completo (CLI).
pub fn model(root: &Path, model: &str, collection: Option<&str>, fields: &str) -> anyhow::Result<Created> {
    let plan = plan_model(root, model, collection, fields).map_err(|errs| anyhow::anyhow!(errs.join("\n")))?;
    write_model(root, &plan)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nombres() {
        assert_eq!(snake_name("CreateUsers").unwrap(), "create_users");
        assert_eq!(snake_name("add phone-to users").unwrap(), "add_phone_to_users");
        assert_eq!(snake_name("BlogPost").unwrap(), "blog_post");
        assert!(snake_name("1abc").is_err());
        assert!(snake_name("a.b").is_err());
        assert_eq!(pascal("create_users"), "CreateUsers");
        assert_eq!(plural("user"), "users");
        assert_eq!(plural("category"), "categories");
        assert_eq!(plural("day"), "days");
        assert_eq!(plural("box"), "boxes");
        assert_eq!(collection_from_name("create_users").as_deref(), Some("users"));
        assert_eq!(collection_from_name("add_phone_to_users").as_deref(), Some("users"));
        assert_eq!(collection_from_name("algo"), None);
    }

    #[test]
    fn ids_consecutivos_por_hora() {
        let existing = ["261125_1401_a.rs", "261125_1402_b.rs", "261125_1301_c.rs", "mod.rs"].map(String::from);
        assert_eq!(next_id_from(&existing, "261125_14", "x"), "261125_1403_x");
        assert_eq!(next_id_from(&existing, "261125_15", "x"), "261125_1501_x");
    }

    /// Proyecto temporal: migración, seed y modelo, y sus mod.rs.
    #[test]
    fn crea_archivos_en_un_proyecto() {
        let root = std::env::temp_dir().join(format!("blacksmith_make_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(MIGRATE_DIR)).unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/lib.rs"), "pub mod models;\n").unwrap();

        let c = model(&root, "BlogPost", None, "id()\nstring(\"title\").index()\ntimestamps()").unwrap();
        assert!(c.notes.is_empty(), "{:?}", c.notes);
        let model_rs = std::fs::read_to_string(root.join("src/models/blog_post.rs")).unwrap();
        assert!(model_rs.contains("pub struct BlogPost"));
        assert!(model_rs.contains("\"blog_posts\""));
        let mods = std::fs::read_to_string(root.join("src/models/mod.rs")).unwrap();
        assert_eq!(mods, "pub mod blog_post;\n");

        let c2 = migration(&root, "add_slug_to_blog_posts", None).unwrap();
        let mig_mod = std::fs::read_to_string(migrations_dir(&root).join("mod.rs")).unwrap();
        let first = c.files[2].file_stem().unwrap().to_str().unwrap().to_string();
        let second = c2.files[0].file_stem().unwrap().to_str().unwrap().to_string();
        assert!(first < second, "{first} {second}");
        assert!(
            mig_mod.contains(&format!("Box::new({first}::CreateBlogPosts),")),
            "{mig_mod}"
        );
        assert!(
            mig_mod.contains(&format!("Box::new({second}::AddSlugToBlogPosts),")),
            "{mig_mod}"
        );
        let alter = std::fs::read_to_string(root.join(&c2.files[0])).unwrap();
        assert!(alter.contains("Schema::alter_collection") && alter.contains("\"blog_posts\""));

        seed(&root, "admin_user").unwrap();
        let seeds = std::fs::read_to_string(seeds_dir(&root).join("mod.rs")).unwrap();
        assert!(seeds.contains("use ironforge::migration::DynSeeder;") && seeds.contains("::AdminUser),"));

        // El mismo modelo dos veces: error, sin tocar nada.
        assert!(model(&root, "BlogPost", None, "").is_err());
        // Sin colección deducible.
        assert!(migration(&root, "algo_raro", None).is_err());
        std::fs::remove_dir_all(&root).unwrap();
    }
}
