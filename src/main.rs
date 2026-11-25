//! blacksmith: CLI de desarrollo para proyectos ironforge.

mod grammar;
mod make;
mod migrate;
mod project;
mod select;
mod tui;

use std::process::ExitCode;

use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;

#[derive(Parser)]
#[command(name = "blacksmith", version, about = "CLI de desarrollo para proyectos ironforge")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Corre algo del proyecto: dev, un módulo (ej. mobile), migration o seed.
    #[command(after_help = migrate::HELP)]
    Run {
        /// dev | <módulo> | migration | seed
        target: String,
        /// Argumentos sin `--`: rollback, step 2, seed, [m<id>, -m<id>], list...
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Abre la interfaz interactiva (paneles, menú y línea de comandos).
    Ui,
    /// Crea archivos del proyecto: migraciones, seeds y modelos.
    Make {
        #[command(subcommand)]
        what: Make,
    },
}

#[derive(Debug, Subcommand)]
enum Make {
    /// Migración nueva: `create_<colección>` crea la colección; otro nombre
    /// deja una plantilla de alter (ej. add_phone_to_users).
    Migration {
        /// Nombre en snake_case: create_users, add_phone_to_users...
        name: String,
        /// Colección, si no se deduce del nombre.
        collection: Option<String>,
    },
    /// Seed nuevo.
    Seed {
        /// Nombre en snake_case: users_admin...
        name: String,
    },
    /// Modelo + su migración create_<colección>. En `blacksmith ui` abre un
    /// diseñador para escribir los campos con vista previa.
    Model {
        /// Nombre del modelo: User, BlogPost...
        name: String,
        /// Colección (por defecto, el plural: users, blog_posts).
        collection: Option<String>,
        /// Campos con la sintaxis del Blueprint, separados por `;` o saltos
        /// de línea: 'string("name").index(); int("age").optional()'.
        #[arg(long)]
        fields: Option<String>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    report(execute(cli.command))
}

/// Ejecuta un comando ya parseado. La interfaz lo usa para los comandos que
/// se escriben en su línea de comandos.
fn execute(command: Command) -> anyhow::Result<ExitCode> {
    match command {
        Command::Run { target, args } => run(&target, args),
        Command::Ui => tui::run(),
        Command::Make { what } => {
            let root = project::root()?;
            let created = run_make(&root, what)?;
            for f in &created.files {
                println!("{} {}", "✓".green(), f.display());
            }
            for n in &created.notes {
                println!("{} {n}", "!".yellow());
            }
            Ok(ExitCode::SUCCESS)
        }
    }
}

/// Ejecuta `make` (lo usan el CLI y la interfaz).
fn run_make(root: &std::path::Path, what: Make) -> anyhow::Result<make::Created> {
    match what {
        Make::Migration { name, collection } => make::migration(root, &name, collection.as_deref()),
        Make::Seed { name } => make::seed(root, &name),
        Make::Model {
            name,
            collection,
            fields,
        } => {
            let fields = make::spec::split_statements(&fields.unwrap_or_default());
            make::model(root, &name, collection.as_deref(), &fields)
        }
    }
}

/// Imprime el error (si lo hay) y lo convierte en código de salida.
fn report(result: anyhow::Result<ExitCode>) -> ExitCode {
    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{} {e:#}", "✗".red());
            ExitCode::FAILURE
        }
    }
}

fn run(target: &str, args: Vec<String>) -> anyhow::Result<ExitCode> {
    match target {
        "migration" | "seed" => {
            let root = project::root()?;
            migrate::run(&root, target, args)
        }
        "dev" => anyhow::bail!("`run dev` todavía no está implementado"),
        module => anyhow::bail!("`run {module}` todavía no está implementado"),
    }
}
