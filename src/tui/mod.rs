//! `blacksmith ui`: interfaz con paneles (menú, ayuda, salida) y una línea de
//! comandos que acepta lo mismo que el CLI, sin escribir `blacksmith`.
//!
//! Los comandos corren en segundo plano: su salida aparece en el panel
//! Salida (una pestaña por proceso) y se pueden tener varios a la vez.

mod ansi;
mod app;
mod cmdline;
mod designer;
mod draw;
mod git;
mod jobs;

use std::io::{self, IsTerminal};
use std::process::ExitCode;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::bail;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};

use app::{Action, App};
use jobs::Msg;

/// Cada cuánto se revisan mensajes y procesos terminados sin eventos.
const TICK: Duration = Duration::from_millis(50);

pub fn run() -> anyhow::Result<ExitCode> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        bail!("ui necesita una terminal interactiva");
    }
    let project = crate::project::root().map_err(|e| format!("{e:#}"));
    let (tx, rx) = mpsc::channel();
    let mut app = App::new(project, tx);

    let mut terminal = ratatui::try_init()?;
    install_mouse_panic_hook();
    execute!(io::stdout(), EnableMouseCapture)?;
    let result = event_loop(&mut terminal, &mut app, &rx);
    app.kill_all();
    let _ = execute!(io::stdout(), DisableMouseCapture);
    ratatui::restore();
    result.map(|()| ExitCode::SUCCESS)
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App, rx: &mpsc::Receiver<Msg>) -> anyhow::Result<()> {
    let mut dirty = true;
    loop {
        if dirty {
            app.refresh_completion();
            terminal.draw(|f| draw::draw(f, app))?;
            dirty = false;
        }
        if event::poll(TICK)? {
            let action = match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => app.on_key(key),
                Event::Mouse(m) => app.on_mouse(m),
                _ => Action::None,
            };
            match action {
                Action::None => {}
                Action::Quit => return Ok(()),
                Action::Suspend(mut cmd, name) => {
                    suspend()?;
                    let result = cmd.status();
                    resume(terminal)?;
                    app.resumed(&name, result);
                }
            }
            dirty = true;
        }
        while let Ok(msg) = rx.try_recv() {
            app.on_msg(msg);
            dirty = true;
        }
        dirty |= app.tick();
    }
}

/// Deja la terminal normal para que otro programa la use.
fn suspend() -> anyhow::Result<()> {
    execute!(io::stdout(), DisableMouseCapture, LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

fn resume(terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    terminal.clear()?;
    Ok(())
}

/// `ratatui::try_init` ya restaura la terminal si hay panic; esto además
/// apaga la captura del mouse para no dejar la terminal mandando escapes.
fn install_mouse_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(io::stdout(), DisableMouseCapture);
        prev(info);
    }));
}
