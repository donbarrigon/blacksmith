//! blacksmith: CLI de desarrollo para proyectos ironforge.

mod migrate;
mod project;
mod select;

use std::process::ExitCode;

use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;

#[derive(Parser)]
#[command(name = "blacksmith", version, about = "CLI de desarrollo para proyectos ironforge")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
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
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Run { target, args } => run(&target, args),
    };
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
