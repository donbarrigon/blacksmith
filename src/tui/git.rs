//! Panel de git: estado del repositorio (rama y archivos), stage/unstage y
//! diffs. Los comandos largos (commit, push...) corren como procesos de la
//! interfaz con `git_command`.

use std::path::Path;
use std::process::Command;

/// Estado de `git status`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GitStatus {
    pub branch: String,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub files: Vec<FileEntry>,
}

/// Un archivo cambiado. `x` = estado en el índice (staged), `y` = en el
/// directorio de trabajo, como en `git status --short`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub x: char,
    pub y: char,
    pub path: String,
}

impl FileEntry {
    pub fn untracked(&self) -> bool {
        self.x == '?'
    }

    pub fn staged(&self) -> bool {
        !matches!(self.x, ' ' | '?' | '!')
    }

    pub fn unstaged(&self) -> bool {
        self.untracked() || self.y != ' '
    }
}

/// `git` con lo necesario para correr dentro de la interfaz: colores aunque
/// la salida no sea una terminal, sin pager, sin editor (un commit sin -m
/// falla en vez de abrir uno) y sin pedir credenciales por la terminal.
pub fn git_command(dir: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(dir)
        .args(["-c", "color.ui=always"])
        .env("GIT_PAGER", "cat")
        .env("GIT_EDITOR", "true")
        .env("GIT_TERMINAL_PROMPT", "0");
    cmd
}

/// Corre git y devuelve stdout; el error trae stderr.
fn run(dir: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .map_err(|e| format!("no se pudo ejecutar git: {e}"))?;
    if out.status.success() {
        Ok(out.stdout)
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn status(dir: &Path) -> Result<GitStatus, String> {
    let out = run(dir, &["status", "--porcelain=v1", "-b", "-z", "--untracked-files=all"])?;
    Ok(parse_status(&String::from_utf8_lossy(&out)))
}

/// Lee la salida de `git status --porcelain=v1 -b -z`.
pub fn parse_status(out: &str) -> GitStatus {
    let mut st = GitStatus::default();
    let mut entries = out.split('\0').filter(|e| !e.is_empty());
    while let Some(entry) = entries.next() {
        if let Some(head) = entry.strip_prefix("## ") {
            parse_branch(&mut st, head);
            continue;
        }
        let mut chars = entry.chars();
        let (Some(x), Some(y)) = (chars.next(), chars.next()) else {
            continue;
        };
        let path = entry.get(3..).unwrap_or_default().to_string();
        if matches!(x, 'R' | 'C') {
            entries.next(); // el nombre anterior del archivo renombrado
        }
        st.files.push(FileEntry { x, y, path });
    }
    st
}

/// `master...origin/master [ahead 1, behind 2]`, `No commits yet on main`,
/// `HEAD (no branch)`.
fn parse_branch(st: &mut GitStatus, head: &str) {
    let (names, counts) = match head.split_once(" [") {
        Some((n, c)) => (n, c.trim_end_matches(']')),
        None => (head, ""),
    };
    let names = names.strip_prefix("No commits yet on ").unwrap_or(names);
    match names.split_once("...") {
        Some((branch, upstream)) => {
            st.branch = branch.to_string();
            st.upstream = Some(upstream.to_string());
        }
        None => st.branch = names.to_string(),
    }
    for part in counts.split(", ") {
        if let Some(n) = part.strip_prefix("ahead ") {
            st.ahead = n.parse().unwrap_or(0);
        } else if let Some(n) = part.strip_prefix("behind ") {
            st.behind = n.parse().unwrap_or(0);
        }
    }
}

/// Espacio: si tiene cambios sin preparar los agrega; si no, los saca del
/// índice. Devuelve lo que hizo, para el registro.
pub fn toggle_stage(dir: &Path, f: &FileEntry) -> Result<String, String> {
    if f.unstaged() {
        run(dir, &["add", "--", &f.path])?;
        Ok(format!("git add {}", f.path))
    } else {
        // `restore --staged` falla sin commits; `rm --cached` sirve ahí.
        match run(dir, &["restore", "--staged", "--", &f.path]) {
            Ok(_) => Ok(format!("git restore --staged {}", f.path)),
            Err(_) => {
                run(dir, &["rm", "--cached", "--quiet", "--", &f.path])?;
                Ok(format!("git rm --cached {}", f.path))
            }
        }
    }
}

pub fn stage_all(dir: &Path) -> Result<(), String> {
    run(dir, &["add", "-A"]).map(|_| ())
}

/// Diff de un archivo, con colores: lo preparado y lo sin preparar por
/// separado; un archivo nuevo se muestra entero como agregado.
pub fn diff(dir: &Path, f: &FileEntry) -> Result<Vec<u8>, String> {
    let color = ["-c", "color.ui=always"];
    if f.untracked() {
        // --no-index termina con código 1 cuando hay diferencias: es normal.
        let out = Command::new("git")
            .current_dir(dir)
            .args(color)
            .args(["diff", "--no-index", "--", "/dev/null", &f.path])
            .output()
            .map_err(|e| format!("no se pudo ejecutar git: {e}"))?;
        return Ok(with_header("archivo nuevo (sin seguimiento)", out.stdout));
    }
    let mut out = Vec::new();
    if f.staged() {
        let staged = run(dir, &[color[0], color[1], "diff", "--cached", "--", &f.path])?;
        out.extend(with_header("preparado (staged)", staged));
    }
    if f.y != ' ' {
        let unstaged = run(dir, &[color[0], color[1], "diff", "--", &f.path])?;
        out.extend(with_header("sin preparar", unstaged));
    }
    Ok(out)
}

fn with_header(title: &str, body: Vec<u8>) -> Vec<u8> {
    let mut out = format!("\x1b[1;36m── {title} ──\x1b[0m\n").into_bytes();
    if body.is_empty() {
        out.extend_from_slice(b"(sin cambios de contenido)\n");
    }
    out.extend(body);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_rama_y_archivos() {
        let out = "## master...origin/master [ahead 2, behind 1]\0 M src/main.rs\0M  src/a.rs\0\
                   MM src/b.rs\0?? nuevo archivo.rs\0R  nuevo.rs\0viejo.rs\0";
        let st = parse_status(out);
        assert_eq!(st.branch, "master");
        assert_eq!(st.upstream.as_deref(), Some("origin/master"));
        assert_eq!((st.ahead, st.behind), (2, 1));
        let paths: Vec<&str> = st.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(
            paths,
            ["src/main.rs", "src/a.rs", "src/b.rs", "nuevo archivo.rs", "nuevo.rs"]
        );
        let f = &st.files;
        assert!(!f[0].staged() && f[0].unstaged());
        assert!(f[1].staged() && !f[1].unstaged());
        assert!(f[2].staged() && f[2].unstaged());
        assert!(f[3].untracked() && f[3].unstaged() && !f[3].staged());

        let st = parse_status("## No commits yet on main\0");
        assert_eq!(st.branch, "main");
        assert_eq!(st.upstream, None);
    }

    /// Repo temporal real: stage, unstage y diff.
    #[test]
    fn stage_unstage_y_diff_en_un_repo() {
        let dir = std::env::temp_dir().join(format!("blacksmith_git_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        run(&dir, &["init", "-q"]).unwrap();
        std::fs::write(dir.join("a.txt"), "hola\n").unwrap();

        let st = status(&dir).unwrap();
        assert_eq!(st.files.len(), 1);
        assert!(st.files[0].untracked());
        let d = String::from_utf8(diff(&dir, &st.files[0]).unwrap()).unwrap();
        assert!(d.contains("hola"), "{d}");

        assert_eq!(toggle_stage(&dir, &st.files[0]).unwrap(), "git add a.txt");
        let st = status(&dir).unwrap();
        assert!(st.files[0].staged() && !st.files[0].unstaged());
        // Sin commits todavía: se saca del índice con rm --cached.
        toggle_stage(&dir, &st.files[0]).unwrap();
        assert!(status(&dir).unwrap().files[0].untracked());

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
