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
#[derive(Debug, Deserialize)]
pub struct Status {
    pub migrations: Vec<MigrationRow>,
    pub seeds: Vec<SeedRow>,
}

#[derive(Debug, Deserialize)]
pub struct MigrationRow {
    /// Con prefijo: `m261122_1401_create_users`.
    pub id: String,
    pub collection: Option<String>,
    pub applied: bool,
    pub last_op: Option<String>,
    pub batch: Option<i64>,
    pub in_code: bool,
}

#[derive(Debug, Deserialize)]
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
    let status = migrate_cmd(root, false)
        .arg(target)
        .args(&resolved)
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

/// Reemplaza cada `list` (justo después de `migration` o de `seed`) por la
/// lista `[...]` elegida en el selector. `None` = el usuario canceló.
fn resolve_lists(root: &Path, target: &str, args: &[String]) -> anyhow::Result<Option<Vec<String>>> {
    let mut status: Option<Status> = None;
    let mut out = Vec::with_capacity(args.len());
    for (i, arg) in args.iter().enumerate() {
        if arg != "list" {
            out.push(arg.clone());
            continue;
        }
        let prev = if i == 0 { target } else { args[i - 1].as_str() };
        if prev != "migration" && prev != "seed" {
            bail!("list va justo después de migration o de seed");
        }
        if status.is_none() {
            status = Some(load_status(root)?);
        }
        let Some(st) = status.as_ref() else { unreachable!() };
        let picked = if prev == "migration" {
            select::pick_migrations(&st.migrations)?
        } else {
            select::pick_seeds(&st.seeds)?
        };
        match picked {
            Some(list) if !list.is_empty() => out.push(format!("[{}]", list.join(", "))),
            _ => return Ok(None),
        }
    }
    Ok(Some(out))
}

/// Corre `migrate status json` y lee la última línea JSON de la salida.
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
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_status(&stdout)
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
}
