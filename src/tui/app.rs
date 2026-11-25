//! Estado de la interfaz y manejo de teclado/mouse/mensajes, sin terminal.

use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Position, Rect};
use ratatui::widgets::ListState;

use super::cmdline::{Builtin, Input, Parsed, parse};
use super::designer::{Designer, Outcome as DesignerOutcome};
use super::git::{self, FileEntry, GitStatus};
use super::jobs::{Job, JobState, Msg};
use crate::Command;
use crate::grammar::{self, Analysis, Suggestion};
use crate::migrate::{self, ListKind, Status};
use crate::select::{self, Outcome, Picker};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Menu,
    Help,
    Output,
    Git,
}

impl Panel {
    pub const ALL: [Panel; 4] = [Panel::Menu, Panel::Help, Panel::Output, Panel::Git];

    pub fn title(self) -> &'static str {
        match self {
            Panel::Menu => "Menú",
            Panel::Help => "Ayuda",
            Panel::Output => "Salida",
            Panel::Git => "Git",
        }
    }

    /// Tecla de función que lo abre/minimiza (F1, F2, ...).
    pub fn key(self) -> u8 {
        self as u8 + 1
    }

    fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Input,
    Menu,
    Git,
}

/// Una entrada del menú: texto y comando. `direct` = se ejecuta al elegirla;
/// si no, solo se escribe en la línea de comandos para confirmarla con enter.
pub struct MenuItem {
    pub label: &'static str,
    pub command: &'static str,
    pub direct: bool,
}

pub const MENU: &[MenuItem] = &[
    MenuItem {
        label: "Estado",
        command: "run migration status",
        direct: true,
    },
    MenuItem {
        label: "Correr pendientes",
        command: "run migration",
        direct: false,
    },
    MenuItem {
        label: "Elegir migraciones",
        command: "run migration list",
        direct: true,
    },
    MenuItem {
        label: "Rollback último batch",
        command: "run migration rollback",
        direct: false,
    },
    MenuItem {
        label: "Desbloquear",
        command: "run migration unlock",
        direct: false,
    },
    MenuItem {
        label: "Seeds pendientes",
        command: "run seed",
        direct: false,
    },
    MenuItem {
        label: "Elegir seeds",
        command: "run seed list",
        direct: true,
    },
    MenuItem {
        label: "Nuevo modelo",
        command: "make model ",
        direct: false,
    },
    MenuItem {
        label: "Nueva migración",
        command: "make migration ",
        direct: false,
    },
    MenuItem {
        label: "Nuevo seed",
        command: "make seed ",
        direct: false,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogKind {
    Command,
    Ok,
    Error,
    Info,
}

#[derive(Debug, Clone)]
pub struct LogLine {
    pub kind: LogKind,
    pub text: String,
}

/// Zonas clickeables del último dibujo.
#[derive(Debug, Default)]
pub struct Hits {
    pub tabs: Vec<(Rect, Panel)>,
    pub close: Vec<(Rect, Panel)>,
    pub panels: Vec<(Rect, Panel)>,
    /// Área interior de la lista del menú (una fila por item).
    pub menu_list: Rect,
    pub input: Rect,
    /// Pestañas del panel de salida: 0 = registro, n = jobs[n - 1].
    pub out_tabs: Vec<(Rect, usize)>,
    /// Lista del selector emergente.
    pub picker_list: Rect,
    /// Lista de sugerencias del autocompletado.
    pub suggestions: Rect,
    pub help: Rect,
    pub git_list: Rect,
}

/// Estado de los IDs para el autocompletado.
#[derive(Debug)]
pub enum Catalog {
    NoProject,
    Loading,
    Ready(Status),
    /// El error quedó en el registro.
    Failed,
}

impl Catalog {
    pub fn status(&self) -> Option<&Status> {
        match self {
            Catalog::Ready(st) => Some(st),
            _ => None,
        }
    }
}

/// Cada cuánto se relee `git status` con el panel Git abierto.
const GIT_REFRESH: Duration = Duration::from_secs(2);

/// Lo que el bucle principal tiene que hacer después de un evento.
pub enum Action {
    None,
    Quit,
    /// Pausar la interfaz y correr este programa en la terminal (lazygit).
    Suspend(Box<std::process::Command>, String),
}

/// Un `list` esperando el estado de las migraciones o la elección.
struct PendingList {
    line: String,
    root: PathBuf,
    target: String,
    args: Vec<String>,
    slots: Vec<(usize, ListKind)>,
    next: usize,
    status: Option<Status>,
}

/// Selector emergente abierto.
pub struct PickerModal {
    pub picker: Picker,
    pub title: &'static str,
    pub state: ListState,
}

pub struct App {
    /// Raíz del proyecto o el motivo por el que no se encontró.
    pub project: Result<PathBuf, String>,
    tx: Sender<Msg>,
    visible: [bool; 4],
    pub focus: Focus,
    pub input: Input,
    history: Vec<String>,
    /// Posición al recorrer el historial con ↑↓.
    hist_pos: Option<usize>,
    pub menu_sel: usize,
    pub log: Vec<LogLine>,
    /// Líneas desplazadas hacia arriba desde el final del registro.
    pub scroll: usize,
    pub jobs: Vec<Job>,
    next_job: usize,
    /// Pestaña del panel de salida: 0 = registro, n = jobs[n - 1].
    pub out_tab: usize,
    pending: Option<PendingList>,
    pub picker: Option<PickerModal>,
    /// Diseñador de `make model` abierto.
    pub designer: Option<Designer>,
    pub hits: Hits,
    /// Carpeta donde corre git: la raíz del proyecto o la actual.
    git_dir: PathBuf,
    pub git: Result<GitStatus, String>,
    pub git_state: ListState,
    git_checked: Instant,
    pub catalog: Catalog,
    /// Análisis de la línea hasta el cursor (sugerencias y ayuda).
    pub analysis: Analysis,
    /// Para qué texto se calculó `analysis`.
    analyzed: Option<String>,
    pub sugg_state: ListState,
    /// Esc cerró las sugerencias (vuelven al cambiar el texto).
    sugg_hidden: bool,
    /// Tab con la línea vacía: mostrar sugerencias igual.
    sugg_forced: bool,
    /// `ayuda <tema>`.
    pub help_query: Option<String>,
    pub help_scroll: u16,
}

impl App {
    pub fn new(project: Result<PathBuf, String>, tx: Sender<Msg>) -> Self {
        let mut app = Self {
            project,
            tx,
            visible: [true; 4],
            focus: Focus::Input,
            input: Input::default(),
            history: Vec::new(),
            hist_pos: None,
            menu_sel: 0,
            log: Vec::new(),
            scroll: 0,
            jobs: Vec::new(),
            next_job: 1,
            out_tab: 0,
            pending: None,
            picker: None,
            designer: None,
            hits: Hits::default(),
            git_dir: PathBuf::from("."),
            git: Ok(GitStatus::default()),
            git_state: ListState::default(),
            git_checked: Instant::now(),
            catalog: Catalog::NoProject,
            analysis: Analysis::default(),
            analyzed: None,
            sugg_state: ListState::default(),
            sugg_hidden: false,
            sugg_forced: false,
            help_query: None,
            help_scroll: 0,
        };
        app.log(
            LogKind::Info,
            "Escribe un comando sin `blacksmith` (ej. run migration status) o elige uno del menú.",
        );
        if let Err(e) = &app.project {
            let e = e.clone();
            app.log(LogKind::Error, e);
        }
        app.git_dir = match &app.project {
            Ok(root) => root.clone(),
            Err(_) => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        };
        app.refresh_git();
        app.load_catalog();
        app.refresh_completion();
        app
    }

    pub fn is_visible(&self, p: Panel) -> bool {
        self.visible[p.index()]
    }

    pub fn toggle(&mut self, p: Panel) {
        self.visible[p.index()] = !self.visible[p.index()];
        let focused_here = matches!((p, self.focus), (Panel::Menu, Focus::Menu) | (Panel::Git, Focus::Git));
        if focused_here && !self.is_visible(p) {
            self.focus = Focus::Input;
        }
    }

    fn show(&mut self, p: Panel) {
        self.visible[p.index()] = true;
    }

    pub fn log(&mut self, kind: LogKind, text: impl Into<String>) {
        for line in text.into().lines() {
            self.log.push(LogLine {
                kind,
                text: line.to_string(),
            });
        }
        self.scroll = 0;
    }

    /// El job de la pestaña de salida elegida.
    pub fn current_job(&self) -> Option<&Job> {
        self.out_tab.checked_sub(1).and_then(|i| self.jobs.get(i))
    }

    fn current_job_mut(&mut self) -> Option<&mut Job> {
        self.out_tab.checked_sub(1).and_then(|i| self.jobs.get_mut(i))
    }

    /// El job al que va lo que se escribe (si está esperando una respuesta).
    pub fn input_target(&self) -> Option<&Job> {
        self.current_job().filter(|j| j.awaiting_input())
    }

    fn scroll_mut(&mut self) -> &mut usize {
        match self.out_tab.checked_sub(1) {
            Some(i) if i < self.jobs.len() => &mut self.jobs[i].scroll,
            _ => &mut self.scroll,
        }
    }

    pub fn running_jobs(&self) -> usize {
        self.jobs.iter().filter(|j| j.is_running()).count()
    }

    // ---------------------------------------------------------------
    // Eventos
    // ---------------------------------------------------------------

    pub fn on_key(&mut self, key: KeyEvent) -> Action {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        match key.code {
            KeyCode::Char('q' | 'd') if ctrl => return Action::Quit,
            KeyCode::Char('c') if ctrl && self.picker.is_none() && self.designer.is_none() => {
                return match self.current_job_mut() {
                    Some(job) if job.is_running() => {
                        job.kill();
                        Action::None
                    }
                    _ => Action::Quit,
                };
            }
            _ => {}
        }
        if self.picker.is_some() {
            self.on_picker_key(key);
            return Action::None;
        }
        if self.designer.is_some() {
            self.on_designer_key(key);
            return Action::None;
        }
        match key.code {
            KeyCode::Char('l') if ctrl => {
                self.log.clear();
                return Action::None;
            }
            KeyCode::Char('w') if ctrl => {
                self.close_tab();
                return Action::None;
            }
            KeyCode::Left if alt => {
                self.out_tab = self.out_tab.checked_sub(1).unwrap_or(self.jobs.len());
                return Action::None;
            }
            KeyCode::Right if alt => {
                self.out_tab = (self.out_tab + 1) % (self.jobs.len() + 1);
                return Action::None;
            }
            KeyCode::F(n @ 1..=4) => {
                self.toggle(Panel::ALL[usize::from(n - 1)]);
                return Action::None;
            }
            KeyCode::Tab if self.focus == Focus::Input => {
                self.tab_complete();
                return Action::None;
            }
            KeyCode::Tab | KeyCode::BackTab => {
                self.focus = self.next_focus();
                return Action::None;
            }
            KeyCode::PageUp => {
                *self.scroll_mut() += 10;
                return Action::None;
            }
            KeyCode::PageDown => {
                let s = self.scroll_mut();
                *s = s.saturating_sub(10);
                return Action::None;
            }
            _ => {}
        }
        match self.focus {
            Focus::Input => self.on_input_key(key),
            Focus::Menu => self.on_menu_key(key),
            Focus::Git => self.on_git_key(key),
        }
    }

    /// Shift+tab: comando → menú → git → comando (saltando los ocultos).
    fn next_focus(&self) -> Focus {
        let order = [Focus::Input, Focus::Menu, Focus::Git];
        let start = order.iter().position(|f| *f == self.focus).unwrap_or(0);
        (1..=order.len())
            .map(|i| order[(start + i) % order.len()])
            .find(|f| match f {
                Focus::Input => true,
                Focus::Menu => self.is_visible(Panel::Menu),
                Focus::Git => self.is_visible(Panel::Git),
            })
            .unwrap_or(Focus::Input)
    }

    fn on_input_key(&mut self, key: KeyEvent) -> Action {
        if self.suggestions_visible() {
            let n = self.analysis.suggestions.len();
            let sel = self.sugg_state.selected().unwrap_or(0);
            match key.code {
                KeyCode::Up => return self.select_suggestion((sel + n - 1) % n),
                KeyCode::Down => return self.select_suggestion((sel + 1) % n),
                KeyCode::Esc => {
                    self.sugg_hidden = true;
                    self.sugg_forced = false;
                    return Action::None;
                }
                _ => {}
            }
        }
        match key.code {
            KeyCode::Enter => return self.submit(),
            KeyCode::Esc => {
                self.input.take();
                self.hist_pos = None;
            }
            KeyCode::Char(c) => {
                self.input.insert(c);
                self.hist_pos = None;
            }
            KeyCode::Backspace => {
                self.input.backspace();
                self.hist_pos = None;
            }
            KeyCode::Delete => {
                self.input.delete();
                self.hist_pos = None;
            }
            KeyCode::Left => self.input.left(),
            KeyCode::Right => self.input.right(),
            KeyCode::Home => self.input.home(),
            KeyCode::End => self.input.end(),
            KeyCode::Up => self.history_prev(),
            KeyCode::Down => self.history_next(),
            _ => {}
        }
        Action::None
    }

    fn on_menu_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.menu_sel = self.menu_sel.checked_sub(1).unwrap_or(MENU.len() - 1);
            }
            KeyCode::Down | KeyCode::Char('j') => self.menu_sel = (self.menu_sel + 1) % MENU.len(),
            KeyCode::Enter => return self.activate_menu(self.menu_sel),
            KeyCode::Esc => self.focus = Focus::Input,
            _ => {}
        }
        Action::None
    }

    fn on_picker_key(&mut self, key: KeyEvent) {
        let Some(modal) = &mut self.picker else { return };
        match modal.picker.on_key(key) {
            Outcome::Continue => {}
            Outcome::Done(list) => self.picked(Some(list)),
            Outcome::Cancel => self.picked(None),
        }
    }

    pub fn on_mouse(&mut self, ev: MouseEvent) -> Action {
        let pos = Position::new(ev.column, ev.row);
        if let Some(d) = &mut self.designer {
            match ev.kind {
                MouseEventKind::ScrollUp => d.preview_scroll = d.preview_scroll.saturating_sub(3),
                MouseEventKind::ScrollDown => d.preview_scroll += 3,
                _ => {}
            }
            return Action::None;
        }
        if let Some(modal) = &mut self.picker {
            let list = self.hits.picker_list;
            match ev.kind {
                MouseEventKind::Down(MouseButton::Left) if list.contains(pos) => {
                    modal.picker.click(modal.state.offset() + usize::from(pos.y - list.y));
                }
                MouseEventKind::ScrollUp => modal.picker.up(),
                MouseEventKind::ScrollDown => modal.picker.down(),
                _ => {}
            }
            return Action::None;
        }
        let on_output = hit(&self.hits.panels, pos) == Some(Panel::Output);
        match ev.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some(p) = hit(&self.hits.tabs, pos).or_else(|| hit(&self.hits.close, pos)) {
                    self.toggle(p);
                } else if let Some((_, tab)) = self.hits.out_tabs.iter().find(|(r, _)| r.contains(pos)) {
                    self.out_tab = *tab;
                } else if self.suggestions_visible() && self.hits.suggestions.contains(pos) {
                    let i = self.sugg_state.offset() + usize::from(pos.y - self.hits.suggestions.y);
                    if i < self.analysis.suggestions.len() {
                        self.accept(i);
                    }
                } else if self.hits.git_list.contains(pos) {
                    let i = self.git_state.offset() + usize::from(pos.y - self.hits.git_list.y);
                    if self.git.as_ref().is_ok_and(|g| i < g.files.len()) {
                        self.focus = Focus::Git;
                        self.git_state.select(Some(i));
                        self.show_diff();
                    }
                } else if self.hits.input.contains(pos) {
                    self.focus = Focus::Input;
                } else if self.hits.menu_list.contains(pos) {
                    let i = usize::from(pos.y - self.hits.menu_list.y);
                    if i < MENU.len() {
                        self.focus = Focus::Menu;
                        self.menu_sel = i;
                        return self.activate_menu(i);
                    }
                }
            }
            MouseEventKind::ScrollUp if self.hits.help.contains(pos) => {
                self.help_scroll = self.help_scroll.saturating_sub(3);
            }
            MouseEventKind::ScrollDown if self.hits.help.contains(pos) => self.help_scroll += 3,
            MouseEventKind::ScrollUp if on_output => *self.scroll_mut() += 3,
            MouseEventKind::ScrollDown if on_output => {
                let s = self.scroll_mut();
                *s = s.saturating_sub(3);
            }
            _ => {}
        }
        Action::None
    }

    /// Mensaje de un hilo de fondo.
    pub fn on_msg(&mut self, msg: Msg) {
        match msg {
            Msg::Output { job, stream, bytes } => {
                let tab = self.out_tab;
                if let Some((i, j)) = self.jobs.iter_mut().enumerate().find(|(_, j)| j.id == job) {
                    j.push_bytes(stream, &bytes);
                    if tab == i + 1 && j.awaiting_input() {
                        self.focus = Focus::Input;
                    }
                }
            }
            Msg::Status(result) => self.status_loaded(result),
            Msg::Catalog(result) => {
                self.catalog = match result {
                    Ok(st) => Catalog::Ready(st),
                    Err(e) => {
                        self.log(
                            LogKind::Error,
                            format!("✗ no se pudieron leer los IDs para autocompletar: {e}"),
                        );
                        Catalog::Failed
                    }
                };
                self.analyzed = None; // recalcular sugerencias con los IDs nuevos
            }
        }
    }

    /// Revisa qué procesos terminaron. `true` si algo cambió.
    pub fn tick(&mut self) -> bool {
        let mut finished = Vec::new();
        for job in &mut self.jobs {
            if job.poll_exit() {
                finished.push((job.id, job.title.clone(), job.state));
            }
        }
        let changed = !finished.is_empty();
        if changed {
            // Una migración pudo cambiar qué está ejecutado; un git, el repo.
            self.load_catalog();
            self.refresh_git();
        } else if self.git_checked.elapsed() >= GIT_REFRESH && self.is_visible(Panel::Git) {
            let before = self.git.clone();
            self.refresh_git();
            return self.git != before;
        }
        for (id, title, state) in finished {
            match state {
                JobState::Exited(Some(0)) => self.log(LogKind::Ok, format!("✓ #{id} {title}")),
                JobState::Exited(Some(code)) => self.log(LogKind::Error, format!("✗ #{id} {title} (código {code})")),
                _ => self.log(LogKind::Error, format!("✗ #{id} {title} (terminó sin código)")),
            }
        }
        changed
    }

    /// Mata todos los procesos (al salir).
    pub fn kill_all(&mut self) {
        for job in &mut self.jobs {
            job.kill();
        }
    }

    // ---------------------------------------------------------------
    // Comandos
    // ---------------------------------------------------------------

    fn activate_menu(&mut self, i: usize) -> Action {
        let item = &MENU[i];
        self.input.set(item.command);
        if item.direct {
            self.submit()
        } else {
            self.focus = Focus::Input;
            Action::None
        }
    }

    /// Procesa la línea escrita: respuesta a un proceso o comando nuevo.
    fn submit(&mut self) -> Action {
        if self.input_target().is_some() {
            let answer = self.input.take();
            let Some(job) = self.current_job_mut() else {
                return Action::None;
            };
            if let Err(e) = job.send_line(&answer) {
                self.log(LogKind::Error, format!("no se pudo responder al proceso: {e}"));
            }
            return Action::None;
        }

        let line = self.input.take().trim().to_string();
        self.hist_pos = None;
        if line.is_empty() {
            return Action::None;
        }
        if self.history.last() != Some(&line) {
            self.history.push(line.clone());
        }
        match parse(&line) {
            Parsed::Empty => Action::None,
            Parsed::Builtin(Builtin::Quit) => Action::Quit,
            Parsed::Builtin(Builtin::Clear) => {
                self.log.clear();
                Action::None
            }
            Parsed::Builtin(Builtin::Help(query)) => {
                self.show(Panel::Help);
                self.help_query = query;
                self.help_scroll = 0;
                Action::None
            }
            Parsed::Builtin(Builtin::Lazygit) => self.lazygit(),
            Parsed::Builtin(Builtin::Edit(file)) => self.edit(&file),
            Parsed::Git(args) => {
                self.log(LogKind::Command, format!("› {line}"));
                self.show(Panel::Output);
                let mut cmd = git::git_command(&self.git_dir);
                cmd.args(&args);
                self.spawn(&line, cmd);
                Action::None
            }
            Parsed::Run(Command::Make { what }) => {
                self.log(LogKind::Command, format!("› {line}"));
                self.make(what);
                Action::None
            }
            Parsed::Run(Command::Ui) => {
                self.log(LogKind::Info, "Ya estás en la interfaz.");
                Action::None
            }
            Parsed::Run(Command::Run { target, args }) => {
                self.log(LogKind::Command, format!("› {line}"));
                self.start(line, target, args);
                Action::None
            }
            Parsed::Message { text, is_error } => {
                self.log(LogKind::Command, format!("› {line}"));
                self.log(if is_error { LogKind::Error } else { LogKind::Info }, text);
                self.show(Panel::Output);
                self.out_tab = 0;
                Action::None
            }
        }
    }

    fn start(&mut self, line: String, target: String, args: Vec<String>) {
        self.show(Panel::Output);
        self.out_tab = 0;
        let root = match &self.project {
            Ok(root) => root.clone(),
            Err(e) => {
                let e = e.clone();
                return self.log(LogKind::Error, e);
            }
        };
        if !matches!(target.as_str(), "migration" | "seed") {
            return self.log(LogKind::Error, format!("✗ `run {target}` todavía no está implementado"));
        }
        let slots = match migrate::list_slots(&target, &args) {
            Ok(s) => s,
            Err(e) => return self.log(LogKind::Error, format!("✗ {e:#}")),
        };
        if slots.is_empty() {
            let cmd = migrate::command(&root, &target, &args);
            return self.spawn(&line, cmd);
        }
        if self.pending.is_some() {
            return self.log(LogKind::Error, "✗ ya hay una selección en curso");
        }
        self.log(
            LogKind::Info,
            "Leyendo el estado de las migraciones... (tarda más si cargo tiene que compilar)",
        );
        let tx = self.tx.clone();
        let fetch_root = root.clone();
        std::thread::spawn(move || {
            let result = migrate::fetch_status(&fetch_root).map_err(|e| format!("{e:#}"));
            let _ = tx.send(Msg::Status(result));
        });
        self.pending = Some(PendingList {
            line,
            root,
            target,
            args,
            slots,
            next: 0,
            status: None,
        });
    }

    fn spawn(&mut self, line: &str, cmd: std::process::Command) {
        let id = self.next_job;
        let title = line.strip_prefix("run ").unwrap_or(line).to_string();
        match Job::spawn(id, title, cmd, &self.tx) {
            Ok(job) => {
                self.next_job += 1;
                self.jobs.push(job);
                self.out_tab = self.jobs.len();
            }
            Err(e) => self.log(LogKind::Error, format!("✗ no se pudo ejecutar {line}: {e}")),
        }
    }

    // ---------------------------------------------------------------
    // Git
    // ---------------------------------------------------------------

    pub fn refresh_git(&mut self) {
        self.git = git::status(&self.git_dir);
        self.git_checked = Instant::now();
        let n = self.git.as_ref().map_or(0, |g| g.files.len());
        let sel = self.git_state.selected().unwrap_or(0);
        self.git_state.select((n > 0).then(|| sel.min(n - 1)));
    }

    fn selected_file(&self) -> Option<FileEntry> {
        let g = self.git.as_ref().ok()?;
        g.files.get(self.git_state.selected()?).cloned()
    }

    fn on_git_key(&mut self, key: KeyEvent) -> Action {
        let n = self.git.as_ref().map_or(0, |g| g.files.len());
        let sel = self.git_state.selected().unwrap_or(0);
        match key.code {
            KeyCode::Up | KeyCode::Char('k') if n > 0 => self.git_state.select(Some((sel + n - 1) % n)),
            KeyCode::Down | KeyCode::Char('j') if n > 0 => self.git_state.select(Some((sel + 1) % n)),
            KeyCode::Char(' ') => {
                if let Some(f) = self.selected_file() {
                    match git::toggle_stage(&self.git_dir, &f) {
                        Ok(done) => self.log(LogKind::Info, done),
                        Err(e) => self.log(LogKind::Error, format!("✗ {e}")),
                    }
                    self.refresh_git();
                }
            }
            KeyCode::Enter | KeyCode::Char('d') => self.show_diff(),
            KeyCode::Char('a') => {
                match git::stage_all(&self.git_dir) {
                    Ok(()) => self.log(LogKind::Info, "git add -A"),
                    Err(e) => self.log(LogKind::Error, format!("✗ {e}")),
                }
                self.refresh_git();
            }
            KeyCode::Char('c') => self.prompt("git commit -m \"\"", 1),
            KeyCode::Char('p') => self.prompt("git push", 0),
            KeyCode::Char('f') => self.prompt("git pull", 0),
            KeyCode::Char('r') => self.refresh_git(),
            KeyCode::Char('L') => return self.lazygit(),
            KeyCode::Char('e') => {
                if let Some(f) = self.selected_file() {
                    return self.edit(&f.path);
                }
            }
            KeyCode::Esc => self.focus = Focus::Input,
            _ => {}
        }
        Action::None
    }

    /// Escribe un comando en la línea para completarlo y confirmarlo con
    /// enter; el cursor queda `back` caracteres antes del final.
    fn prompt(&mut self, text: &str, back: usize) {
        let len = text.chars().count();
        self.input.set_with_cursor(text.to_string(), len - back);
        self.focus = Focus::Input;
        self.hist_pos = None;
    }

    /// Muestra el diff del archivo elegido en una pestaña de Salida (la
    /// reutiliza si ya había una del mismo archivo).
    fn show_diff(&mut self) {
        let Some(f) = self.selected_file() else { return };
        let bytes = match git::diff(&self.git_dir, &f) {
            Ok(b) => b,
            Err(e) => return self.log(LogKind::Error, format!("✗ {e}")),
        };
        let title = format!("diff {}", f.path);
        self.show(Panel::Output);
        if let Some(i) = self.jobs.iter().position(|j| j.title == title) {
            let id = self.jobs[i].id;
            self.jobs[i] = Job::finished(id, title, &bytes);
            self.out_tab = i + 1;
        } else {
            let id = self.next_job;
            self.next_job += 1;
            self.jobs.push(Job::finished(id, title, &bytes));
            self.out_tab = self.jobs.len();
        }
    }

    fn lazygit(&mut self) -> Action {
        let mut cmd = std::process::Command::new("lazygit");
        cmd.current_dir(&self.git_dir);
        Action::Suspend(Box::new(cmd), "lazygit".into())
    }

    /// Abre `file` (relativo al proyecto) en $VISUAL / $EDITOR / nano.
    fn edit(&mut self, file: &str) -> Action {
        let editor = std::env::var("VISUAL")
            .or_else(|_| std::env::var("EDITOR"))
            .ok()
            .filter(|e| !e.trim().is_empty())
            .unwrap_or_else(|| "nano".into());
        let mut parts = editor.split_whitespace();
        let program = parts.next().unwrap_or("nano").to_string();
        let mut cmd = std::process::Command::new(&program);
        cmd.args(parts).arg(file).current_dir(&self.git_dir);
        self.log(LogKind::Command, format!("› {program} {file}"));
        Action::Suspend(Box::new(cmd), program)
    }

    /// Volvió el programa que pausó la interfaz.
    pub fn resumed(&mut self, name: &str, result: std::io::Result<std::process::ExitStatus>) {
        match result {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                self.log(LogKind::Error, format!("✗ {name} no está instalado"));
            }
            Err(e) => self.log(LogKind::Error, format!("✗ no se pudo abrir {name}: {e}")),
        }
        self.refresh_git();
    }

    /// `make ...`: crea los archivos y muestra cuáles.
    fn make(&mut self, what: crate::Make) {
        self.show(Panel::Output);
        self.out_tab = 0;
        let root = match &self.project {
            Ok(root) => root.clone(),
            Err(e) => {
                let e = e.clone();
                return self.log(LogKind::Error, e);
            }
        };
        if let crate::Make::Model {
            name,
            collection,
            fields: None,
        } = what
        {
            // Sin --fields: se escriben los campos en el diseñador.
            self.designer = Some(Designer::new(root, name, collection));
            return;
        }
        match crate::run_make(&root, what) {
            Ok(created) => self.created(&created),
            Err(e) => self.log(LogKind::Error, format!("✗ {e:#}")),
        }
    }

    fn on_designer_key(&mut self, key: KeyEvent) {
        let Some(d) = &mut self.designer else { return };
        match d.on_key(key) {
            DesignerOutcome::Continue => {}
            DesignerOutcome::Cancel => {
                self.designer = None;
                self.log(LogKind::Info, "Diseñador cerrado sin crear nada.");
            }
            DesignerOutcome::Save => match d.save() {
                Ok(created) => {
                    self.designer = None;
                    self.created(&created);
                }
                Err(e) => self.log(LogKind::Error, format!("✗ {e:#}")),
            },
        }
    }

    /// Muestra lo creado por `make` y relee git y el estado.
    fn created(&mut self, created: &crate::make::Created) {
        for f in &created.files {
            self.log(LogKind::Ok, format!("✓ {}", f.display()));
        }
        for n in &created.notes {
            self.log(LogKind::Error, format!("! {n}"));
        }
        if let Some(first) = created.files.first() {
            self.log(LogKind::Info, format!("Para abrirlo: editar {}", first.display()));
        }
        self.refresh_git();
        self.load_catalog();
    }

    fn status_loaded(&mut self, result: Result<Status, String>) {
        let Some(pending) = &mut self.pending else { return };
        match result {
            Ok(st) => {
                pending.status = Some(st.clone());
                self.catalog = Catalog::Ready(st);
                self.analyzed = None;
                self.open_next_picker();
            }
            Err(e) => {
                self.pending = None;
                self.log(LogKind::Error, format!("✗ {e}"));
            }
        }
    }

    /// Abre el selector del siguiente `list`, o lanza el comando si no quedan.
    fn open_next_picker(&mut self) {
        let Some(p) = &self.pending else { return };
        let Some(st) = &p.status else { return };
        let Some(&(_, kind)) = p.slots.get(p.next) else {
            let Some(p) = self.pending.take() else { return };
            self.log(LogKind::Info, format!("→ {} {}", p.target, p.args.join(" ")));
            let cmd = migrate::command(&p.root, &p.target, &p.args);
            return self.spawn(&p.line, cmd);
        };
        let (items, title, allow_down) = match kind {
            ListKind::Migrations => (select::migration_items(&st.migrations), "Migraciones", true),
            ListKind::Seeds => (select::seed_items(&st.seeds), "Seeds", false),
        };
        if items.is_empty() {
            self.pending = None;
            return self.log(
                LogKind::Error,
                format!("✗ no hay {} en el proyecto", title.to_lowercase()),
            );
        }
        self.picker = Some(PickerModal {
            picker: Picker::new(items, allow_down),
            title,
            state: ListState::default(),
        });
    }

    /// El usuario cerró el selector: `None` o lista vacía = cancelar.
    fn picked(&mut self, list: Option<Vec<String>>) {
        self.picker = None;
        let Some(p) = &mut self.pending else { return };
        match list {
            Some(ids) if !ids.is_empty() => {
                let (i, _) = p.slots[p.next];
                p.args[i] = migrate::format_list(&ids);
                p.next += 1;
                self.open_next_picker();
            }
            _ => {
                self.pending = None;
                self.log(LogKind::Info, "Cancelado.");
            }
        }
    }

    // ---------------------------------------------------------------
    // Autocompletado
    // ---------------------------------------------------------------

    /// Pide en segundo plano el estado del proyecto para sugerir IDs.
    fn load_catalog(&mut self) {
        let Ok(root) = &self.project else { return };
        if matches!(self.catalog, Catalog::Loading) {
            return;
        }
        if !matches!(self.catalog, Catalog::Ready(_)) {
            self.catalog = Catalog::Loading;
        }
        let tx = self.tx.clone();
        let root = root.clone();
        std::thread::spawn(move || {
            let result = migrate::fetch_status(&root).map_err(|e| format!("{e:#}"));
            let _ = tx.send(Msg::Catalog(result));
        });
    }

    /// Recalcula sugerencias y ayuda si cambió el texto hasta el cursor.
    pub fn refresh_completion(&mut self) {
        let before = self.input.before_cursor();
        if self.analyzed.as_deref() == Some(before.as_str()) {
            return;
        }
        let text_changed = self.analyzed.as_ref().is_some_and(|a| *a != before);
        self.analysis = grammar::analyze(&before, self.catalog.status());
        if text_changed || self.analyzed.is_none() {
            self.sugg_hidden = false;
        }
        if text_changed {
            self.sugg_forced = false;
            self.help_query = None;
        }
        let sel = self.sugg_state.selected().unwrap_or(0);
        let n = self.analysis.suggestions.len();
        self.sugg_state =
            ListState::default().with_selected((n > 0).then(|| if text_changed { 0 } else { sel.min(n - 1) }));
        self.analyzed = Some(before);
    }

    pub fn suggestions_visible(&self) -> bool {
        self.focus == Focus::Input
            && self.picker.is_none()
            && self.designer.is_none()
            && self.input_target().is_none()
            && self.hist_pos.is_none()
            && !self.sugg_hidden
            && !self.analysis.suggestions.is_empty()
            && (!self.input.text().is_empty() || self.sugg_forced)
    }

    pub fn selected_suggestion(&self) -> Option<&Suggestion> {
        self.analysis.suggestions.get(self.sugg_state.selected()?)
    }

    fn select_suggestion(&mut self, i: usize) -> Action {
        self.sugg_state.select(Some(i));
        Action::None
    }

    /// Tab: completa la sugerencia elegida, o muestra las sugerencias.
    fn tab_complete(&mut self) {
        self.refresh_completion();
        if self.suggestions_visible() || self.analysis.suggestions.len() == 1 {
            self.accept(self.sugg_state.selected().unwrap_or(0));
        } else {
            self.sugg_hidden = false;
            self.sugg_forced = true;
            self.hist_pos = None;
        }
    }

    fn accept(&mut self, i: usize) {
        let Some(s) = self.analysis.suggestions.get(i) else {
            return;
        };
        let chars: Vec<char> = self.input.text().chars().collect();
        let cursor = self.input.cursor();
        let from = s.from.min(cursor);
        let mut text: String = chars[..from].iter().collect();
        text.push_str(&s.insert);
        text.extend(&chars[cursor..]);
        let new_cursor = from + s.insert.chars().count();
        self.input.set_with_cursor(text, new_cursor);
        self.hist_pos = None;
        self.refresh_completion();
        // Seguir mostrando lo siguiente (Tab, Tab, Tab...).
        self.sugg_forced = true;
    }

    fn close_tab(&mut self) {
        let Some(i) = self.out_tab.checked_sub(1) else { return };
        if i < self.jobs.len() {
            self.jobs.remove(i); // Drop mata el proceso si seguía corriendo
            self.out_tab = self.out_tab.min(self.jobs.len());
        }
    }

    fn history_prev(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let pos = self.hist_pos.map_or(self.history.len() - 1, |p| p.saturating_sub(1));
        self.hist_pos = Some(pos);
        self.input.set(&self.history[pos]);
    }

    fn history_next(&mut self) {
        match self.hist_pos {
            Some(p) if p + 1 < self.history.len() => {
                self.hist_pos = Some(p + 1);
                self.input.set(&self.history[p + 1]);
            }
            Some(_) => {
                self.hist_pos = None;
                self.input.take();
            }
            None => {}
        }
    }
}

fn hit(zones: &[(Rect, Panel)], pos: Position) -> Option<Panel> {
    zones.iter().find(|(r, _)| r.contains(pos)).map(|(_, p)| *p)
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use ratatui::crossterm::event::{KeyEventKind, KeyEventState};

    use super::*;
    use crate::migrate::{MigrationRow, SeedRow};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn type_line(app: &mut App, s: &str) -> Action {
        for c in s.chars() {
            app.on_key(key(KeyCode::Char(c)));
        }
        app.on_key(key(KeyCode::Enter))
    }

    /// App sin proyecto: los comandos `run` no lanzan procesos.
    fn app() -> App {
        let (tx, _rx) = mpsc::channel();
        App::new(Err("sin proyecto".into()), tx)
    }

    #[test]
    fn historial_y_errores() {
        let mut app = app();
        type_line(&mut app, "run seed");
        assert_eq!(app.log.last().unwrap().text, "sin proyecto");
        type_line(&mut app, "nada");
        assert_eq!(app.log.last().unwrap().kind, LogKind::Error);
        app.on_key(key(KeyCode::Up));
        assert_eq!(app.input.text(), "nada");
        app.on_key(key(KeyCode::Up));
        assert_eq!(app.input.text(), "run seed");
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Down));
        assert_eq!(app.input.text(), "");
        assert!(matches!(type_line(&mut app, "salir"), Action::Quit));
    }

    #[test]
    fn paneles_y_foco() {
        let mut app = app();
        app.on_key(key(KeyCode::BackTab));
        assert_eq!(app.focus, Focus::Menu);
        app.on_key(key(KeyCode::BackTab));
        assert_eq!(app.focus, Focus::Git);
        app.on_key(key(KeyCode::BackTab));
        assert_eq!(app.focus, Focus::Input);
        app.on_key(key(KeyCode::BackTab));
        app.on_key(key(KeyCode::F(1))); // minimiza el menú → foco vuelve al input
        assert!(!app.is_visible(Panel::Menu));
        assert_eq!(app.focus, Focus::Input);
        app.on_key(key(KeyCode::F(4))); // y sin git tampoco hay a dónde ir
        app.on_key(key(KeyCode::BackTab));
        assert_eq!(app.focus, Focus::Input);
    }

    #[test]
    fn menu_a_confirmar_solo_escribe() {
        let mut app = app();
        assert!(matches!(app.activate_menu(3), Action::None));
        assert_eq!(app.input.text(), "run migration rollback");
    }

    #[test]
    fn list_abre_selectores_en_orden() {
        let mut app = app();
        app.pending = Some(PendingList {
            line: "run migration list seed list".into(),
            root: PathBuf::from("/no/existe"),
            target: "migration".into(),
            args: ["list", "seed", "list"].map(String::from).into(),
            slots: vec![(0, ListKind::Migrations), (2, ListKind::Seeds)],
            next: 0,
            status: None,
        });
        let status = Status {
            migrations: vec![MigrationRow {
                id: "m1".into(),
                collection: Some("users".into()),
                applied: false,
                last_op: None,
                batch: None,
                in_code: true,
            }],
            seeds: vec![SeedRow {
                id: "s1".into(),
                status: None,
                attempts: 0,
                only_dev: false,
                in_code: true,
            }],
        };
        app.on_msg(Msg::Status(Ok(status)));
        assert_eq!(app.picker.as_ref().unwrap().title, "Migraciones");
        app.on_key(key(KeyCode::Char(' ')));
        app.on_key(key(KeyCode::Enter));
        assert_eq!(app.picker.as_ref().unwrap().title, "Seeds");
        assert_eq!(app.pending.as_ref().unwrap().args[0], "[m1]");
        app.on_key(key(KeyCode::Esc)); // cancela todo
        assert!(app.picker.is_none() && app.pending.is_none());
        assert_eq!(app.log.last().unwrap().text, "Cancelado.");
    }

    fn status() -> Status {
        Status {
            migrations: vec![
                MigrationRow {
                    id: "m261122_1401_create_users".into(),
                    collection: Some("users".into()),
                    applied: true,
                    last_op: Some("up".into()),
                    batch: Some(1),
                    in_code: true,
                },
                MigrationRow {
                    id: "m261123_0900_add_posts".into(),
                    collection: Some("posts".into()),
                    applied: false,
                    last_op: None,
                    batch: None,
                    in_code: true,
                },
            ],
            seeds: vec![],
        }
    }

    fn type_text(app: &mut App, s: &str) {
        for c in s.chars() {
            app.on_key(key(KeyCode::Char(c)));
            app.refresh_completion();
        }
    }

    fn tab(app: &mut App) {
        app.on_key(key(KeyCode::Tab));
        app.refresh_completion();
    }

    #[test]
    fn tab_completa_paso_a_paso() {
        let mut app = app();
        app.on_msg(Msg::Catalog(Ok(status())));
        type_text(&mut app, "run mig");
        assert!(app.suggestions_visible());
        tab(&mut app);
        assert_eq!(app.input.text(), "run migration ");
        assert!(app.suggestions_visible()); // sigue mostrando lo siguiente
        // Elige `[` con flechas y Tab.
        let i = app.analysis.suggestions.iter().position(|s| s.label == "[").unwrap();
        for _ in 0..i {
            app.on_key(key(KeyCode::Down));
        }
        tab(&mut app);
        assert_eq!(app.input.text(), "run migration [");
        type_text(&mut app, "post");
        tab(&mut app);
        assert_eq!(app.input.text(), "run migration [m261123_0900_add_posts, ");
        // Lo siguiente es cerrar la lista o la otra migración (como down).
        let labels: Vec<&str> = app.analysis.suggestions.iter().map(|s| s.label.as_str()).collect();
        assert_eq!(labels, ["]", "-m261122_1401_create_users"]);
        tab(&mut app);
        assert_eq!(app.input.text(), "run migration [m261123_0900_add_posts] ");
        assert_eq!(app.analysis.suggestions[0].label, "seed");
        // Esc cierra las sugerencias; escribir las vuelve a abrir.
        app.on_key(key(KeyCode::Esc));
        assert!(!app.suggestions_visible());
        assert_eq!(app.input.text(), "run migration [m261123_0900_add_posts] ");
        type_text(&mut app, "s");
        assert!(app.suggestions_visible());
    }

    #[test]
    fn historial_sin_sugerencias_y_tab_en_vacio() {
        let mut app = app();
        tab(&mut app); // línea vacía: muestra los comandos
        assert!(app.suggestions_visible());
        assert_eq!(app.analysis.suggestions[0].label, "run");
        app.on_key(key(KeyCode::Esc));
        type_line(&mut app, "ayuda force");
        assert_eq!(app.help_query.as_deref(), Some("force"));
        app.refresh_completion();
        app.on_key(key(KeyCode::Up)); // línea vacía: historial
        app.refresh_completion();
        assert_eq!(app.input.text(), "ayuda force");
        assert!(!app.suggestions_visible());
    }

    #[test]
    fn panel_git_en_un_repo_real() {
        let dir = std::env::temp_dir().join(format!("blacksmith_app_git_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let git = |args: &[&str]| {
            let ok = std::process::Command::new("git")
                .current_dir(&dir)
                .args(args)
                .status()
                .unwrap();
            assert!(ok.success());
        };
        git(&["init", "-q"]);
        std::fs::write(dir.join("nuevo.rs"), "fn main() {}\n").unwrap();

        let (tx, rx) = mpsc::channel();
        let mut app = App::new(Err("sin proyecto".into()), tx);
        app.git_dir = dir.clone();
        app.refresh_git();
        assert_eq!(app.git.as_ref().unwrap().files.len(), 1);

        app.focus = Focus::Git;
        app.on_key(key(KeyCode::Char(' '))); // git add
        assert!(app.git.as_ref().unwrap().files[0].staged());
        app.on_key(key(KeyCode::Enter)); // diff en Salida
        let job = app.current_job().unwrap();
        assert_eq!(job.title, "diff nuevo.rs");
        assert!(job.view().any(|l| l.to_string().contains("fn main")));

        // c: deja el commit escrito con el cursor entre las comillas.
        app.on_key(key(KeyCode::Char('c')));
        assert_eq!(app.focus, Focus::Input);
        assert_eq!(app.input.before_cursor(), "git commit -m \"");
        type_text(&mut app, "primero");
        assert_eq!(app.input.text(), "git commit -m \"primero\"");
        app.on_key(key(KeyCode::Enter)); // corre como proceso
        let id = app.jobs.last().unwrap().id;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while app.jobs.last().unwrap().is_running() {
            assert!(std::time::Instant::now() < deadline);
            while let Ok(msg) = rx.try_recv() {
                app.on_msg(msg);
            }
            app.tick();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let job = app.jobs.iter().find(|j| j.id == id).unwrap();
        // Sin user.name configurado el commit puede fallar; lo que importa es
        // que corrió git con el mensaje y terminó (y el estado se releyó).
        let out: Vec<String> = job.view().map(|l| l.to_string()).collect();
        if job.state == JobState::Exited(Some(0)) {
            assert!(out.iter().any(|l| l.contains("primero")), "{out:?}");
            assert!(app.git.as_ref().unwrap().files.is_empty());
        } else {
            assert!(
                out.iter().any(|l| l.contains("user") || l.contains("identity")),
                "{out:?}"
            );
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn make_model_con_el_disenador() {
        let root = std::env::temp_dir().join(format!("blacksmith_app_make_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(crate::project::MIGRATE_DIR)).unwrap();
        std::fs::write(root.join("Cargo.toml"), "").unwrap();
        let (tx, _rx) = mpsc::channel();
        let mut app = App::new(Ok(root.clone()), tx);

        type_line(&mut app, "make model Post");
        assert!(app.designer.is_some());
        assert!(!app.suggestions_visible());
        // Agrega un campo con tab y lo crea.
        app.on_key(key(KeyCode::Enter));
        type_text(&mut app, "boo");
        app.on_key(key(KeyCode::Tab));
        type_text(&mut app, "published");
        let ctrl_s = KeyEvent {
            modifiers: KeyModifiers::CONTROL,
            ..key(KeyCode::Char('s'))
        };
        app.on_key(ctrl_s);
        assert!(app.designer.is_none(), "{:?}", app.log.last());

        let model = std::fs::read_to_string(root.join("src/models/post.rs")).unwrap();
        assert!(model.contains("pub published: bool,"), "{model}");
        let logs: Vec<&str> = app.log.iter().map(|l| l.text.as_str()).collect();
        assert!(logs.iter().any(|l| l.starts_with("✓ src/models/post.rs")), "{logs:?}");
        assert!(logs.iter().any(|l| l.contains("pub mod models;")), "{logs:?}"); // aviso: falta declarar models

        // make migration directo (sin diseñador).
        type_line(&mut app, "make migration add_slug_to_posts");
        assert!(app.log.iter().any(|l| l.text.contains("_add_slug_to_posts.rs")));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn pestañas_de_salida_y_respuesta() {
        let mut app = app();
        app.jobs.push(Job::fake(1, "migration reset"));
        app.jobs.push(Job::fake(2, "seed"));
        app.on_key(KeyEvent {
            modifiers: KeyModifiers::ALT,
            ..key(KeyCode::Right)
        });
        assert_eq!(app.out_tab, 1);
        app.on_key(KeyEvent {
            modifiers: KeyModifiers::ALT,
            ..key(KeyCode::Left)
        });
        app.on_key(KeyEvent {
            modifiers: KeyModifiers::ALT,
            ..key(KeyCode::Left)
        });
        assert_eq!(app.out_tab, 2);
        app.on_key(KeyEvent {
            modifiers: KeyModifiers::CONTROL,
            ..key(KeyCode::Char('w'))
        });
        assert_eq!(app.jobs.len(), 1);
        assert_eq!(app.out_tab, 1);
    }
}
