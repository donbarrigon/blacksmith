//! Ubicación del proyecto ironforge.

use std::path::{Path, PathBuf};

use anyhow::{Context, bail};

/// Carpeta del binario de migraciones dentro del proyecto.
pub const MIGRATE_DIR: &str = "src/bin/migrate";

/// Raíz del proyecto: la primera carpeta, desde la actual hacia arriba, que
/// tiene `Cargo.toml` y `src/bin/migrate`.
pub fn root() -> anyhow::Result<PathBuf> {
    let cwd = std::env::current_dir().context("no se pudo leer la carpeta actual")?;
    find_root(&cwd)
}

fn find_root(start: &Path) -> anyhow::Result<PathBuf> {
    for dir in start.ancestors() {
        if dir.join("Cargo.toml").is_file() && dir.join(MIGRATE_DIR).is_dir() {
            return Ok(dir.to_path_buf());
        }
    }
    bail!(
        "no estás dentro de un proyecto ironforge (no se encontró Cargo.toml con {MIGRATE_DIR} desde {})",
        start.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encuentra_la_raiz_desde_una_subcarpeta() {
        let tmp = std::env::temp_dir().join(format!("blacksmith_root_{}", std::process::id()));
        let sub = tmp.join("src/controllers");
        std::fs::create_dir_all(tmp.join(MIGRATE_DIR)).unwrap();
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(tmp.join("Cargo.toml"), "").unwrap();

        assert_eq!(find_root(&sub).unwrap(), tmp);
        std::fs::remove_dir_all(&tmp).unwrap();
    }
}
