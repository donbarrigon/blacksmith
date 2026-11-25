//! `blacksmith run migration ...` y `blacksmith run seed ...`: pasa los
//! argumentos tal cual al binario `migrate` del proyecto (que tiene la misma
//! sintaxis) y resuelve `list` con un selector interactivo.

use std::path::Path;
use std::process::{Command, ExitCode, ExitStatus, Stdio};

use anyhow::{Context, bail};
use owo_colors::OwoColorize;
use serde::Deserialize;

use crate::select;

pub const HELP: &str = "\
Migraciones:
  run migration                          Ejecuta las pendientes
  run migration rollback [step N]        Revierte el último batch o las últimas N
  run migration reset [force]            Revierte todas
  run migration refresh [force]          Revierte todas y las vuelve a ejecutar
  run migration fresh [force]            Borra todas las colecciones y ejecuta todas
  run migration [m<id>, -m<id>, ...]     Solo esas, en ese orden (- = down)
  run migration list                     Elige de una lista cuáles correr (- = down)
  run migration status | unlock

  Al final de migration, refresh, fresh, [...] o list:
  ... seed                               Corre después los seeds pendientes
  ... seed [s<id>, ...] | seed list      Corre después solo esos seeds

Seeds:
  run seed                               Los nuevos y los que fallaron
  run seed [s<id>, ...] [force]          Solo esos, aunque ya hayan corrido
  run seed list [force]                  Elige de una lista cuáles correr

En zsh pon la lista entre comillas: '[m261122_1401_create_users]'";

/// Estado que devuelve `migrate status json`.
#[derive(Debug, Clone, Deserialize)]
pub struct Status {
    pub migrations: Vec<MigrationRow>,
    pub seeds: Vec<SeedRow>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MigrationRow {
    /// Con prefijo: `m261122_1401_create_users`.
    pub id: String,
    pub collection: Option<String>,
    pub applied: bool,
    pub last_op: Option<String>,
    pub batch: Option<i64>,
    pub in_code: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SeedRow {
    /// Con prefijo: `s261122_1501_users_admin`.
    pub id: String,
    pub status: Option<String>,
    pub attempts: i64,
    pub only_dev: bool,
    pub in_code: bool,
}

pub fn run(root: &Path, target: &str, args: Vec<String>) -> anyhow::Result<ExitCode> {
    let Some(resolved) = resolve_lists(root, target, &args)? else {
        println!("Cancelado.");
        return Ok(ExitCode::SUCCESS);
    };
    if resolved != args {
        println!("{} {target} {}", "→".cyan(), resolved.join(" "));
    }
    let status = command(root, target, &resolved)
        .status()
        .context("no se pudo ejecutar cargo")?;
    Ok(exit_code(status))
}

/// `cargo run --bin migrate --` en la raíz del proyecto. `quiet` oculta la
/// salida de compilación de cargo (los errores igual se ven).
fn migrate_cmd(root: &Path, quiet: bool) -> Command {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(root).args(["run", "--bin", "migrate"]);
    if quiet {
        cmd.arg("--quiet");
    }
    cmd.arg("--");
    cmd
}

/// Qué lista abre un `list`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKind {
    Migrations,
    Seeds,
}

/// Posiciones de cada `list` en `args` y qué lista elige. Error si un `list`
/// no va justo después de `migration` o de `seed`.
pub fn list_slots(target: &str, args: &[String]) -> anyhow::Result<Vec<(usize, ListKind)>> {
    let mut slots = Vec::new();
    for (i, arg) in args.iter().enumerate() {
        if arg != "list" {
            continue;
        }
        let prev = if i == 0 { target } else { args[i - 1].as_str() };
        match prev {
            "migration" => slots.push((i, ListKind::Migrations)),
            "seed" => slots.push((i, ListKind::Seeds)),
            _ => bail!("list va justo después de migration o de seed"),
        }
    }
    Ok(slots)
}

/// Reemplaza cada `list` (justo después de `migration` o de `seed`) por la
/// lista `[...]` elegida en el selector. `None` = el usuario canceló.
fn resolve_lists(root: &Path, target: &str, args: &[String]) -> anyhow::Result<Option<Vec<String>>> {
    let slots = list_slots(target, args)?;
    if slots.is_empty() {
        return Ok(Some(args.to_vec()));
    }
    let st = load_status(root)?;
    let mut out = args.to_vec();
    for (i, kind) in slots {
        let picked = match kind {
            ListKind::Migrations => select::pick_migrations(&st.migrations)?,
            ListKind::Seeds => select::pick_seeds(&st.seeds)?,
        };
        match picked {
            Some(list) if !list.is_empty() => out[i] = format_list(&list),
            _ => return Ok(None),
        }
    }
    Ok(Some(out))
}

/// `[a, b, c]`, como lo espera el binario `migrate`.
pub fn format_list(ids: &[String]) -> String {
    format!("[{}]", ids.join(", "))
}

/// El comando que ejecuta `migrate <target> <args>` (args ya sin `list`).
pub fn command(root: &Path, target: &str, args: &[String]) -> Command {
    let mut cmd = migrate_cmd(root, false);
    cmd.arg(target).args(args);
    cmd
}

/// Corre `migrate status json` y lee la última línea JSON de la salida.
/// Para el CLI: avisa y deja ver la compilación de cargo si falla.
pub fn load_status(root: &Path) -> anyhow::Result<Status> {
    println!("{}", "Leyendo el estado de las migraciones...".dimmed());
    let output = migrate_cmd(root, true)
        .args(["status", "json"])
        .stderr(Stdio::inherit())
        .output()
        .context("no se pudo ejecutar cargo")?;
    if !output.status.success() {
        bail!("`migrate status json` terminó con error");
    }
    parse_status(&String::from_utf8_lossy(&output.stdout))
}

/// Como `load_status` pero sin escribir en la terminal (para la interfaz):
/// si falla, el error incluye el final de lo que cargo escribió en stderr.
pub fn fetch_status(root: &Path) -> anyhow::Result<Status> {
    let output = migrate_cmd(root, true)
        .args(["status", "json"])
        .stdin(Stdio::null())
        .output()
        .context("no se pudo ejecutar cargo")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tail: Vec<&str> = stderr.lines().rev().take(15).collect();
        let tail: Vec<&str> = tail.into_iter().rev().collect();
        bail!("`migrate status json` terminó con error:\n{}", tail.join("\n"));
    }
    parse_status(&String::from_utf8_lossy(&output.stdout))
}

fn parse_status(stdout: &str) -> anyhow::Result<Status> {
    let line = stdout.lines().rev().find(|l| l.trim_start().starts_with('{')).context(
        "`migrate status json` no devolvió JSON; ¿el proyecto usa una versión de ironforge con status json?",
    )?;
    serde_json::from_str(line).context("no se pudo leer el JSON de `migrate status json`")
}

fn exit_code(status: ExitStatus) -> ExitCode {
    match status.code() {
        Some(0) => ExitCode::SUCCESS,
        Some(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
        None => ExitCode::FAILURE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_la_ultima_linea_json() {
        let out = "log cualquiera\n{\"migrations\":[{\"id\":\"m261122_1401_create_users\",\"collection\":\"users\",\
                   \"applied\":true,\"last_op\":\"up\",\"batch\":1,\"ran_at\":null,\"in_code\":true}],\
                   \"seeds\":[{\"id\":\"s261122_1501_users_admin\",\"status\":null,\"error\":null,\
                   \"attempts\":0,\"only_dev\":false,\"in_code\":true}]}\n";
        let st = parse_status(out).unwrap();
        assert_eq!(st.migrations[0].id, "m261122_1401_create_users");
        assert!(st.migrations[0].applied);
        assert_eq!(st.seeds[0].attempts, 0);
        assert!(parse_status("sin json").is_err());
    }

    #[test]
    fn encuentra_cada_list() {
        let args: Vec<String> = ["list", "seed", "list"].map(String::from).into();
        assert_eq!(
            list_slots("migration", &args).unwrap(),
            [(0, ListKind::Migrations), (2, ListKind::Seeds)]
        );
        assert!(list_slots("migration", &["step".into(), "list".into()]).is_err());
        assert!(list_slots("seed", &[]).unwrap().is_empty());
    }
}
