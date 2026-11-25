//! Dibujo de la interfaz. Guarda en `app.hits` las zonas clickeables.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, List, ListItem, ListState, Paragraph, Wrap};

use super::app::{App, Catalog, Focus, Hits, LogKind, MENU, Panel};
use super::designer::View;
use super::jobs::JobState;
use crate::grammar::{self, Topic};
use crate::make;
use crate::select;

const DIM: Style = Style::new().fg(Color::DarkGray);
const ACCENT: Color = Color::Cyan;
const MENU_WIDTH: u16 = 28;
/// Ancho de la columna izquierda con el panel Git abierto.
const GIT_WIDTH: u16 = 38;
/// Largo máximo del título de un proceso en su pestaña.
const TAB_TITLE: usize = 22;

pub fn draw(f: &mut Frame, app: &mut App) {
    app.hits = Hits::default();
    let [tabs, body, input, keys] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .areas(f.area());

    draw_tabs(f, app, tabs);
    draw_body(f, app, body);
    draw_input(f, app, input);
    draw_suggestions(f, app);
    let hint = if app.designer.is_some() {
        "diseñador abierto · esc cancela"
    } else if app.picker.is_some() {
        "selector abierto · esc cancela"
    } else {
        "tab completa · shift+tab menú/git · F1-F4 paneles · alt+←→ pestañas · ctrl+c corta · ctrl+q sale"
    };
    f.render_widget(Paragraph::new(hint).style(DIM), keys);
    if app.picker.is_some() {
        draw_picker(f, app);
    }
    if app.designer.is_some() {
        draw_designer(f, app);
    }
}

/// Resalta una línea de código: comentarios en gris, strings en verde y, si
/// `method_first`, la primera palabra (el tipo) en cian.
fn highlight(line: &str, method_first: bool) -> Line<'static> {
    if line.trim_start().starts_with("//") {
        return Line::styled(line.to_string(), DIM);
    }
    let mut spans = Vec::new();
    let mut cur = String::new();
    let mut in_str = false;
    let mut first_word = method_first;
    for c in line.chars() {
        if in_str {
            cur.push(c);
            if c == '"' {
                spans.push(Span::styled(std::mem::take(&mut cur), Style::new().fg(Color::Green)));
                in_str = false;
            }
            continue;
        }
        if c == '"' {
            spans.push(Span::raw(std::mem::take(&mut cur)));
            cur.push(c);
            in_str = true;
        } else if first_word && c == '(' {
            let word = std::mem::take(&mut cur);
            let (indent, name) = word.split_at(word.len() - word.trim_start().len());
            let (prefix, name) = name.split_at(if name.starts_with("col.") { 4 } else { 0 });
            spans.push(Span::raw(format!("{indent}{prefix}")));
            spans.push(Span::styled(name.to_string(), Style::new().fg(ACCENT).bold()));
            cur.push(c);
            first_word = false;
        } else {
            cur.push(c);
        }
    }
    let style = if in_str {
        Style::new().fg(Color::Green)
    } else {
        Style::new()
    };
    spans.push(Span::styled(cur, style));
    Line::from(spans)
}

/// Diseñador de `make model`: campos a la izquierda, código a la derecha.
fn draw_designer(f: &mut Frame, app: &mut App) {
    let Some(d) = &mut app.designer else { return };
    let screen = f.area();
    // Ocupa todo menos la barra de atajos de abajo.
    let area = Rect::new(screen.x, screen.y, screen.width, screen.height.saturating_sub(1));
    let coll = d.collection_name();
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(ACCENT))
        .title(Span::styled(
            format!(" Nuevo modelo: {} → colección {coll} ", d.model),
            Style::new().bold(),
        ));
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);
    if inner.height < 6 || inner.width < 30 {
        return;
    }
    let [files, body, status, keys] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(2),
        Constraint::Length(1),
    ])
    .areas(inner);

    // Archivos que se van a crear.
    let files_line = match &d.plan {
        Ok(p) => Line::from(vec![
            Span::styled("Crea: ", DIM),
            Span::raw(p.model_path.display().to_string()),
            Span::styled(" · ", DIM),
            Span::raw(p.migration_path.display().to_string()),
            Span::styled(" · y los registra en sus mod.rs", DIM),
        ]),
        Err(_) => Line::styled(
            "Corrige los errores para ver qué se crea.",
            Style::new().fg(Color::Yellow),
        ),
    };
    f.render_widget(Paragraph::new(files_line), files);

    let [left, right] = Layout::horizontal([Constraint::Percentage(45), Constraint::Min(20)]).areas(body);

    // Editor con número de línea; las líneas con error, en rojo.
    let ed_block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(ACCENT))
        .title(Span::styled(" Campos (sintaxis del Blueprint) ", Style::new().bold()))
        .title_bottom(Line::from(Span::styled(" tab completa el tipo ", DIM)).right_aligned());
    let ed = ed_block.inner(left);
    f.render_widget(ed_block, left);
    let errors = d.error_lines();
    d.editor.scroll_to_cursor(ed.height as usize);
    let gutter = 4u16;
    // Desplazamiento horizontal: todas las líneas se corren igual para que
    // el cursor quede a la vista.
    let text_w = ed.width.saturating_sub(gutter) as usize;
    let left = d.editor.col.saturating_sub(text_w.saturating_sub(1));
    let lines: Vec<Line> = d
        .editor
        .lines
        .iter()
        .enumerate()
        .skip(d.editor.top)
        .take(ed.height as usize)
        .map(|(i, l)| {
            let n_style = if errors.contains(&(i + 1)) {
                Style::new().fg(Color::Red).bold()
            } else {
                DIM
            };
            let visible: String = l.chars().skip(left).collect();
            let mut line = highlight(&visible, left == 0);
            line.spans.insert(0, Span::styled(format!("{:>3} ", i + 1), n_style));
            line
        })
        .collect();
    f.render_widget(Paragraph::new(lines), ed);
    let cx = ed.x + gutter + (d.editor.col - left) as u16;
    let cy = ed.y + (d.editor.row - d.editor.top) as u16;
    if cx < ed.right() && cy < ed.bottom() {
        f.set_cursor_position((cx, cy));
    }

    // Vista previa del código generado.
    let (title, code) = match (&d.plan, d.view) {
        (Ok(p), View::Migration) => (" Migración ", p.migration_code.as_str()),
        (Ok(p), View::Model) => (" Modelo ", p.model_code.as_str()),
        (Err(_), _) => (" Errores ", ""),
    };
    let pv_block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(DIM)
        .title(Span::styled(title, Style::new().bold()))
        .title_bottom(Line::from(Span::styled(" ctrl+t migración/modelo · PgUp/PgDn ", DIM)).right_aligned());
    let preview: Vec<Line> = match &d.plan {
        Ok(_) => code.lines().map(|l| highlight(l, false)).collect(),
        Err(errs) => errs
            .iter()
            .map(|e| Line::styled(format!("✗ {e}"), Style::new().fg(Color::Red)))
            .collect(),
    };
    let max = preview.len().saturating_sub(1) as u16;
    d.preview_scroll = d.preview_scroll.min(max);
    f.render_widget(
        Paragraph::new(preview).block(pv_block).scroll((d.preview_scroll, 0)),
        right,
    );

    // Estado: opciones de tab, avisos, o la lista de tipos.
    let status_lines = if !d.completions.is_empty() {
        vec![Line::from(vec![
            Span::styled("Tab: ", DIM),
            Span::styled(d.completions.join("  "), Style::new().fg(ACCENT)),
        ])]
    } else if d.confirm_cancel {
        vec![Line::styled(
            "Hay cambios sin crear: esc otra vez los descarta.",
            Style::new().fg(Color::Yellow),
        )]
    } else if let Some(p) = d.plan.as_ref().ok().filter(|p| !p.notes.is_empty()) {
        p.notes
            .iter()
            .map(|n| Line::styled(format!("! {n}"), Style::new().fg(Color::Yellow)))
            .collect()
    } else {
        vec![
            Line::from(vec![
                Span::styled("Tipos: ", DIM),
                Span::raw(make::spec::FIELD_METHODS.join(" ")),
            ]),
            Line::from(vec![
                Span::styled("Índices de colección: ", DIM),
                Span::raw(make::spec::INDEX_METHODS.join(" ")),
            ]),
        ]
    };
    f.render_widget(Paragraph::new(status_lines).wrap(Wrap { trim: true }), status);
    f.render_widget(
        Paragraph::new("ctrl+s crea los archivos · enter nueva línea · esc cancela").style(DIM),
        keys,
    );
}

fn draw_tabs(f: &mut Frame, app: &mut App, area: Rect) {
    let mut spans = vec![Span::styled(
        " blacksmith ",
        Style::new().fg(Color::Black).bg(ACCENT).bold(),
    )];
    let mut x = area.x + 12;
    for p in Panel::ALL {
        let label = format!(" F{} {} ", p.key(), p.title());
        let width = label.chars().count() as u16;
        let style = if app.is_visible(p) { Style::new().bold() } else { DIM };
        x += 1; // separador
        app.hits.tabs.push((Rect::new(x, area.y, width, 1), p));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(label, style));
        x += width;
    }
    let running = app.running_jobs();
    if running > 0 {
        spans.push(Span::styled(
            format!("  ⟳ {running} corriendo"),
            Style::new().fg(Color::Yellow),
        ));
    }
    spans.push(match &app.project {
        Ok(root) => Span::styled(format!("  {}", root.display()), DIM),
        Err(_) => Span::styled("  sin proyecto ironforge", Style::new().fg(Color::Yellow)),
    });
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_body(f: &mut Frame, app: &mut App, area: Rect) {
    let (menu, git) = (app.is_visible(Panel::Menu), app.is_visible(Panel::Git));
    let right = if menu || git {
        let width = if git { GIT_WIDTH } else { MENU_WIDTH };
        let [left, right] = Layout::horizontal([Constraint::Length(width), Constraint::Min(10)]).areas(area);
        match (menu, git) {
            (true, true) => {
                let menu_h = MENU.len() as u16 + 2;
                let [m, g] = Layout::vertical([Constraint::Length(menu_h), Constraint::Min(4)]).areas(left);
                draw_menu(f, app, m);
                draw_git(f, app, g);
            }
            (true, false) => draw_menu(f, app, left),
            _ => draw_git(f, app, left),
        }
        right
    } else {
        area
    };

    match (app.is_visible(Panel::Help), app.is_visible(Panel::Output)) {
        (true, true) => {
            let [help, output] = Layout::vertical([Constraint::Percentage(35), Constraint::Min(3)]).areas(right);
            draw_help(f, app, help);
            draw_output(f, app, output);
        }
        (true, false) => draw_help(f, app, right),
        (false, true) => draw_output(f, app, right),
        (false, false) => f.render_widget(
            Paragraph::new("Paneles minimizados: F2/F3 o clic en la barra de arriba para abrirlos.").style(DIM),
            right,
        ),
    }
}

/// Marco del panel con botón × para minimizarlo.
fn panel_block(app: &mut App, p: Panel, area: Rect, focused: bool) -> Block<'static> {
    app.hits.panels.push((area, p));
    if area.width >= 6 {
        app.hits.close.push((Rect::new(area.right() - 4, area.y, 3, 1), p));
    }
    let border = if focused { Style::new().fg(ACCENT) } else { DIM };
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(border)
        .title(Span::styled(format!(" {} ", p.title()), Style::new().bold()))
        .title(Line::from(Span::styled(" × ", DIM)).right_aligned())
}

fn draw_menu(f: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == Focus::Menu;
    let block = panel_block(app, Panel::Menu, area, focused);
    let inner = block.inner(area);
    app.hits.menu_list = inner;
    let items: Vec<ListItem> = MENU
        .iter()
        .map(|m| {
            let mark = if m.direct { " " } else { "…" };
            ListItem::new(Line::from(vec![Span::raw(m.label), Span::styled(mark, DIM)]))
        })
        .collect();
    let mut state = ListState::default().with_selected(Some(app.menu_sel));
    let highlight = if focused {
        Style::new().fg(ACCENT).bold()
    } else {
        Style::new().bold()
    };
    f.render_widget(block, area);
    f.render_stateful_widget(
        List::new(items).highlight_symbol("› ").highlight_style(highlight),
        inner,
        &mut state,
    );
}

fn draw_git(f: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == Focus::Git;
    let block = panel_block(app, Panel::Git, area, focused)
        .title_bottom(Line::from(Span::styled(" ␣ stage · ⏎ diff · c commit ", DIM)).right_aligned());
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height < 2 {
        return;
    }
    let st = match &app.git {
        Ok(st) => st,
        Err(e) => {
            let msg = if e.contains("not a git repository") {
                "No es un repositorio git."
            } else {
                e.as_str()
            };
            f.render_widget(
                Paragraph::new(msg.to_string()).style(DIM).wrap(Wrap { trim: true }),
                inner,
            );
            return;
        }
    };
    let [head, list] = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(inner);

    let mut spans = vec![Span::styled(
        format!(" {}", st.branch),
        Style::new().fg(Color::Magenta).bold(),
    )];
    if st.ahead > 0 {
        spans.push(Span::styled(format!(" ↑{}", st.ahead), Style::new().fg(Color::Green)));
    }
    if st.behind > 0 {
        spans.push(Span::styled(format!(" ↓{}", st.behind), Style::new().fg(Color::Yellow)));
    }
    match &st.upstream {
        Some(up) => spans.push(Span::styled(format!(" → {up}"), DIM)),
        None => spans.push(Span::styled(" (sin upstream)", DIM)),
    }
    f.render_widget(Paragraph::new(Line::from(spans)), head);

    app.hits.git_list = list;
    if st.files.is_empty() {
        f.render_widget(
            Paragraph::new(" ✓ sin cambios").style(Style::new().fg(Color::Green)),
            list,
        );
        return;
    }
    let width = list.width.saturating_sub(5) as usize;
    let items: Vec<ListItem> = st
        .files
        .iter()
        .map(|file| {
            let (x, y) = if file.untracked() { ('?', '?') } else { (file.x, file.y) };
            let x_style = if file.untracked() {
                Style::new().fg(Color::Red)
            } else {
                Style::new().fg(Color::Green)
            };
            // Si no entra, se ve el final de la ruta (el nombre del archivo).
            let n = file.path.chars().count();
            let path = if n > width {
                format!("…{}", file.path.chars().skip(n + 1 - width).collect::<String>())
            } else {
                file.path.clone()
            };
            ListItem::new(Line::from(vec![
                Span::styled(x.to_string(), x_style),
                Span::styled(y.to_string(), Style::new().fg(Color::Red)),
                Span::raw(format!(" {path}")),
            ]))
        })
        .collect();
    let highlight = if focused {
        Style::new().fg(ACCENT).bold()
    } else {
        Style::new().bold()
    };
    f.render_stateful_widget(
        List::new(items).highlight_symbol("›").highlight_style(highlight),
        list,
        &mut app.git_state,
    );
}

fn draw_help(f: &mut Frame, app: &mut App, area: Rect) {
    let block = panel_block(app, Panel::Help, area, false);
    app.hits.help = area;
    let catalog = match &app.catalog {
        Catalog::NoProject => Span::styled(" sin proyecto: no hay IDs ", DIM),
        Catalog::Loading => Span::styled(" leyendo IDs del proyecto… ", Style::new().fg(Color::Yellow)),
        Catalog::Ready(st) => Span::styled(
            format!(" IDs: {} migraciones · {} seeds ", st.migrations.len(), st.seeds.len()),
            DIM,
        ),
        Catalog::Failed => Span::styled(
            " no se pudo leer el estado (ver Registro) ",
            Style::new().fg(Color::Red),
        ),
    };
    let block = block.title_bottom(Line::from(catalog).right_aligned());

    let lines = if !app.input.text().trim().is_empty() && app.input_target().is_none() {
        context_help(app)
    } else if let Some(q) = &app.help_query {
        query_help(q)
    } else {
        reference_help()
    };
    let max_scroll = lines.len().saturating_sub(1) as u16;
    app.help_scroll = app.help_scroll.min(max_scroll);
    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((app.help_scroll, 0))
            .block(block),
        area,
    );
}

/// Ayuda de lo que se está escribiendo.
fn context_help(app: &App) -> Vec<Line<'static>> {
    let a = &app.analysis;
    let mut lines = Vec::new();
    match a.topic {
        Some(t) => topic_lines(&mut lines, t),
        None => lines.push(Line::styled("Comandos", Style::new().bold())),
    }
    lines.push(Line::raw(""));
    if let Some(e) = &a.error {
        lines.push(Line::styled(format!("✗ {e}"), Style::new().fg(Color::Red).bold()));
    }
    if let Some(x) = &a.expecting {
        lines.push(Line::styled(format!("→ {x}"), Style::new().fg(Color::Yellow)));
    }
    if !a.suggestions.is_empty() {
        lines.push(Line::styled("Puede seguir (tab completa):", DIM));
        let width = a
            .suggestions
            .iter()
            .map(|s| s.label.chars().count())
            .max()
            .unwrap_or(0)
            .min(40);
        for s in &a.suggestions {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<width$}  ", s.label), Style::new().fg(ACCENT)),
                Span::styled(s.desc.clone(), DIM),
            ]));
        }
    } else if a.error.is_none() && a.expecting.is_none() {
        lines.push(Line::styled(
            "✓ completo: enter lo ejecuta",
            Style::new().fg(Color::Green),
        ));
    }
    lines
}

fn query_help(q: &str) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::styled(format!("Ayuda sobre «{q}»"), Style::new().bold()),
        Line::raw(""),
    ];
    let found = grammar::search(q);
    if found.is_empty() {
        lines.push(Line::styled("Nada coincide. `ayuda` sola muestra todo.", DIM));
    }
    for t in found {
        topic_lines(&mut lines, t);
        lines.push(Line::raw(""));
    }
    lines
}

fn reference_help() -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::styled(
            "Escribe los comandos sin `blacksmith`; tab completa y la ayuda sigue lo que escribes.",
            DIM,
        ),
        Line::styled(
            "Las listas [m1, m2] no necesitan comillas. `ayuda <tema>` busca (ej. ayuda force).",
            DIM,
        ),
    ];
    for (group, topics) in grammar::REFERENCE {
        lines.push(Line::raw(""));
        lines.push(Line::styled(group.to_string(), Style::new().bold().underlined()));
        for t in *topics {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<46}", t.syntax), Style::new().fg(ACCENT)),
                Span::raw(t.desc),
            ]));
        }
    }
    shortcut_lines(&mut lines, "Atajos", SHORTCUTS);
    shortcut_lines(&mut lines, "Panel Git", GIT_SHORTCUTS);
    lines
}

const SHORTCUTS: &[(&str, &str)] = &[
    ("tab", "completa / muestra sugerencias"),
    ("↑ ↓", "elige sugerencia, o recorre el historial"),
    ("esc", "cierra sugerencias; otra vez borra la línea"),
    ("shift+tab", "pasa del comando al menú, a git y vuelve"),
    ("F1 … F4", "abre o minimiza Menú, Ayuda, Salida y Git"),
    ("alt+← →", "cambia de pestaña en Salida"),
    ("PgUp PgDn", "desplaza la salida (o la rueda del mouse)"),
    ("ctrl+c", "corta el proceso de la pestaña; sin proceso, sale"),
    ("ctrl+w", "cierra la pestaña del proceso"),
    ("ctrl+l", "limpia el registro"),
    ("ctrl+q", "sale (corta todos los procesos)"),
];

/// Teclas con el panel Git enfocado (shift+tab hasta llegar a él).
const GIT_SHORTCUTS: &[(&str, &str)] = &[
    ("↑ ↓", "elige archivo (clic también, y muestra el diff)"),
    ("espacio", "prepara / saca del commit (git add / restore --staged)"),
    ("enter · d", "diff del archivo en Salida"),
    ("a", "prepara todo (git add -A)"),
    ("c", "escribe git commit -m \"\" para poner el mensaje"),
    ("p · f", "escribe git push / git pull para confirmar con enter"),
    ("e", "abre el archivo en $EDITOR (pausa la interfaz)"),
    ("L", "abre lazygit (pausa la interfaz)"),
    ("r", "relee el estado (se relee solo cada 2 s)"),
];

fn shortcut_lines(lines: &mut Vec<Line<'static>>, title: &str, keys: &[(&str, &'static str)]) {
    lines.push(Line::raw(""));
    lines.push(Line::styled(title.to_string(), Style::new().bold().underlined()));
    for (k, d) in keys {
        lines.push(Line::from(vec![
            Span::styled(format!("  {k:<16}"), Style::new().fg(ACCENT)),
            Span::raw(*d),
        ]));
    }
}

fn topic_lines(lines: &mut Vec<Line<'static>>, t: &Topic) {
    lines.push(Line::styled(t.syntax, Style::new().fg(ACCENT).bold()));
    lines.push(Line::raw(t.desc));
    for n in t.notes {
        lines.push(Line::styled(format!("· {n}"), DIM));
    }
}

fn draw_output(f: &mut Frame, app: &mut App, area: Rect) {
    let block = panel_block(app, Panel::Output, area, false);
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height < 2 {
        return;
    }
    let [tabs, body] = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(inner);
    draw_output_tabs(f, app, tabs);

    let height = body.height as usize;
    let (lines, scroll): (Vec<Line>, usize) = match app.out_tab.checked_sub(1) {
        Some(i) if i < app.jobs.len() => {
            let job = &mut app.jobs[i];
            let total = job.view_len();
            job.scroll = job.scroll.min(total.saturating_sub(height));
            let top = total.saturating_sub(height) - job.scroll;
            let mut lines: Vec<Line> = job.view().skip(top).take(height).collect();
            if total == 0 {
                lines.push(Line::styled("(sin salida todavía)", DIM));
            }
            (lines, job.scroll)
        }
        _ => {
            let total = app.log.len();
            app.scroll = app.scroll.min(total.saturating_sub(height));
            let top = total.saturating_sub(height) - app.scroll;
            let lines = app.log[top..]
                .iter()
                .take(height)
                .map(|l| {
                    let style = match l.kind {
                        LogKind::Command => Style::new().fg(ACCENT).add_modifier(Modifier::BOLD),
                        LogKind::Ok => Style::new().fg(Color::Green),
                        LogKind::Error => Style::new().fg(Color::Red),
                        LogKind::Info => Style::new(),
                    };
                    Line::styled(l.text.clone(), style)
                })
                .collect();
            (lines, app.scroll)
        }
    };
    f.render_widget(Paragraph::new(lines), body);
    if scroll > 0 {
        let note = format!(" ↓ {scroll} líneas más ");
        let w = note.chars().count() as u16;
        if body.width > w {
            let r = Rect::new(body.right() - w, body.bottom() - 1, w, 1);
            f.render_widget(
                Paragraph::new(note).style(Style::new().fg(Color::Black).bg(Color::DarkGray)),
                r,
            );
        }
    }
}

fn draw_output_tabs(f: &mut Frame, app: &mut App, area: Rect) {
    let mut spans = Vec::new();
    let mut x = area.x;
    let mut push = |app: &mut App, spans: &mut Vec<Span<'static>>, idx: usize, label: String, color: Color| {
        let selected = app.out_tab == idx;
        let style = if selected {
            Style::new().fg(Color::Black).bg(color).bold()
        } else {
            Style::new().fg(color)
        };
        let w = label.chars().count() as u16;
        if x + w <= area.right() {
            app.hits.out_tabs.push((Rect::new(x, area.y, w, 1), idx));
            spans.push(Span::styled(label, style));
            spans.push(Span::raw(" "));
        }
        x += w + 1;
    };
    push(app, &mut spans, 0, " Registro ".into(), Color::Gray);
    for i in 0..app.jobs.len() {
        let job = &app.jobs[i];
        let (icon, color) = match job.state {
            JobState::Running if job.awaiting_input() => ("?", Color::Magenta),
            JobState::Running => ("⟳", Color::Yellow),
            JobState::Exited(Some(0)) => ("✓", Color::Green),
            JobState::Exited(_) => ("✗", Color::Red),
            JobState::Killed => ("■", Color::Red),
        };
        let mut title: String = job.title.chars().take(TAB_TITLE).collect();
        if job.title.chars().count() > TAB_TITLE {
            title.push('…');
        }
        let label = format!(" {icon} #{} {title} ", job.id);
        push(app, &mut spans, i + 1, label, color);
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Cuántos caracteres del inicio se ocultan para que el cursor quepa.
fn input_skip(cursor: usize, avail: usize) -> usize {
    cursor.saturating_sub(avail.saturating_sub(1))
}

fn input_inner(area: Rect) -> Rect {
    Block::bordered().inner(area)
}

fn draw_input(f: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == Focus::Input && app.picker.is_none() && app.designer.is_none();
    let (title, color) = match app.input_target() {
        Some(job) => (format!(" Respuesta para #{} (enter la envía) ", job.id), Color::Magenta),
        None => (" Comando ".to_string(), ACCENT),
    };
    let border = if focused { Style::new().fg(color) } else { DIM };
    let mut block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(border)
        .title(Span::styled(title, Style::new().bold()));
    if let Some(e) = app.analysis.error.as_ref().filter(|_| app.input_target().is_none()) {
        block = block.title(Line::from(Span::styled(format!(" ✗ {e} "), Style::new().fg(Color::Red))).right_aligned());
    }
    let inner = input_inner(area);
    app.hits.input = area;

    // Desplaza el texto si el cursor se pasa del ancho.
    let avail = inner.width.saturating_sub(2) as usize;
    let cursor = app.input.cursor();
    let skip = input_skip(cursor, avail);
    let visible: String = app.input.text().chars().skip(skip).take(avail).collect();
    let mut spans = vec![Span::styled("› ", Style::new().fg(color)), Span::raw(visible.clone())];
    if let Some(ghost) = ghost_text(app) {
        let room = avail.saturating_sub(visible.chars().count());
        spans.push(Span::styled(ghost.chars().take(room).collect::<String>(), DIM));
    }
    f.render_widget(Paragraph::new(Line::from(spans)).block(block), area);
    if focused {
        f.set_cursor_position((inner.x + 2 + (cursor - skip) as u16, inner.y));
    }
}

/// Lo que falta de la sugerencia elegida, en gris tras el cursor (solo si el
/// cursor está al final y la sugerencia sigue lo escrito).
fn ghost_text(app: &App) -> Option<String> {
    if !app.suggestions_visible() || app.input.cursor() != app.input.text().chars().count() {
        return None;
    }
    let s = app.selected_suggestion()?;
    let typed: String = app.input.text().chars().skip(s.from).collect();
    let insert = s.insert.trim_end();
    insert
        .starts_with(&typed)
        .then(|| insert[typed.len()..].to_string())
        .filter(|g| !g.is_empty())
}

/// Ventana de sugerencias, justo encima de la línea de comandos.
fn draw_suggestions(f: &mut Frame, app: &mut App) {
    if !app.suggestions_visible() {
        return;
    }
    let input = app.hits.input;
    let inner = input_inner(input);
    let avail = inner.width.saturating_sub(2) as usize;
    let skip = input_skip(app.input.cursor(), avail);
    let suggestions = &app.analysis.suggestions;
    let from = suggestions.first().map_or(0, |s| s.from);

    let label_w = suggestions.iter().map(|s| s.label.chars().count()).max().unwrap_or(0);
    let desc_w = suggestions.iter().map(|s| s.desc.chars().count()).max().unwrap_or(0);
    let screen = f.area();
    let width = ((label_w + desc_w + 7) as u16)
        .min(screen.width.saturating_sub(2))
        .max(12);
    let rows = suggestions.len().min(8) as u16;
    let height = (rows + 2).min(input.y.saturating_sub(screen.y));
    if height < 3 {
        return;
    }
    // El texto empieza en inner.x + 2 (tras "› "); la etiqueta, 3 columnas
    // después del borde de la ventana (borde + "› ").
    let word_x = inner.x + 2 + from.saturating_sub(skip) as u16;
    let x = word_x
        .saturating_sub(3)
        .max(screen.x)
        .min(screen.right().saturating_sub(width));
    let area = Rect::new(x, input.y - height, width, height);

    let items: Vec<ListItem> = suggestions
        .iter()
        .map(|s| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<label_w$}", s.label), Style::new().fg(ACCENT)),
                Span::styled(format!("  {}", s.desc), DIM),
            ]))
        })
        .collect();
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(ACCENT))
        .title_bottom(Line::from(Span::styled(" tab completa · esc cierra ", DIM)).right_aligned());
    app.hits.suggestions = block.inner(area);
    f.render_widget(Clear, area);
    f.render_stateful_widget(
        List::new(items)
            .block(block)
            .highlight_symbol("› ")
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED)),
        area,
        &mut app.sugg_state,
    );
}

/// Selector de `list` como ventana centrada sobre el resto.
fn draw_picker(f: &mut Frame, app: &mut App) {
    let Some(modal) = &mut app.picker else { return };
    let screen = f.area();
    let width = screen.width.saturating_sub(8).min(110);
    let height = (modal.picker.items.len() as u16 + 6).min(screen.height.saturating_sub(4));
    let area = Rect::new(
        screen.x + (screen.width - width) / 2,
        screen.y + (screen.height - height) / 2,
        width,
        height,
    );
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(ACCENT))
        .title(Span::styled(" Elegir ", Style::new().bold()));
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);
    app.hits.picker_list = select::render(f, inner, &modal.picker, modal.title, &mut modal.state);
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;

    use super::super::jobs::{Job, Stream};
    use super::*;

    fn text_at(buf: &Buffer, r: Rect) -> String {
        (r.x..r.right()).map(|x| buf[(x, r.y)].symbol().to_string()).collect()
    }

    fn render(app: &mut App) -> Buffer {
        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        term.draw(|f| draw(f, app)).unwrap();
        let buf = term.backend().buffer().clone();
        if std::env::var("SHOW_UI").is_ok() {
            for y in 0..buf.area.height {
                println!("{}", text_at(&buf, Rect::new(0, y, buf.area.width, 1)));
            }
        }
        buf
    }

    #[test]
    fn sugerencias_y_ayuda_contextual() {
        let (tx, _rx) = mpsc::channel();
        let mut app = App::new(Ok("/proyecto".into()), tx);
        for c in "run migration rollback ".chars() {
            app.input.insert(c);
        }
        app.refresh_completion();
        let buf = render(&mut app);
        let all: String = (0..buf.area.height)
            .map(|y| text_at(&buf, Rect::new(0, y, buf.area.width, 1)))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            all.contains("run migration rollback [step N]"),
            "falta el tema en la ayuda"
        );
        assert!(all.contains("step  cuántas revertir"), "falta la sugerencia");
        // La ventana de sugerencias queda justo encima de la línea de comandos.
        assert_eq!(app.hits.suggestions.bottom() + 1, app.hits.input.y);
        // Y la etiqueta queda alineada con la palabra que completa.
        let label_x = app.hits.suggestions.x + 2;
        let word_x = app.hits.input.x + 1 + 2 + "run migration rollback ".len() as u16;
        assert_eq!(label_x, word_x);

        // Error de sintaxis: se ve en la línea de comandos antes de ejecutar.
        for c in "force ".chars() {
            app.input.insert(c);
        }
        app.refresh_completion();
        let buf = render(&mut app);
        let input_top = text_at(&buf, Rect::new(0, app.hits.input.y, buf.area.width, 1));
        assert!(
            input_top.contains("force solo aplica a reset, refresh y fresh"),
            "{input_top}"
        );
    }

    /// Con SHOW_UI=1 imprime la interfaz con el git de este repositorio.
    #[test]
    fn panel_git_se_dibuja() {
        let (tx, _rx) = mpsc::channel();
        let mut app = App::new(Err("sin proyecto".into()), tx);
        app.focus = Focus::Git;
        render(&mut app);
        if app.git.as_ref().is_ok_and(|g| !g.files.is_empty()) {
            assert!(app.hits.git_list.height > 0);
        }
    }

    #[test]
    fn disenador_se_dibuja() {
        let (tx, _rx) = mpsc::channel();
        let mut app = App::new(Err("sin proyecto".into()), tx);
        let mut d = super::super::designer::Designer::new(
            std::env::temp_dir().join("blacksmith_no_existe"),
            "User".into(),
            None,
        );
        for c in "\nstring(\"email\").unique(UniqueType::Normal)\nstrng(\"x\")".chars() {
            let code = if c == '\n' {
                ratatui::crossterm::event::KeyCode::Enter
            } else {
                ratatui::crossterm::event::KeyCode::Char(c)
            };
            d.on_key(ratatui::crossterm::event::KeyEvent::from(code));
        }
        app.designer = Some(d);
        let buf = render(&mut app);
        let all: String = (0..buf.area.height)
            .map(|y| text_at(&buf, Rect::new(0, y, buf.area.width, 1)))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(all.contains("Nuevo modelo: User → colección users"));
        assert!(all.contains("línea 5: tipo desconocido: strng (¿string?)"), "{all}");
    }

    #[test]
    fn zonas_clickeables_sobre_su_texto() {
        let (tx, _rx) = mpsc::channel();
        let mut app = App::new(Ok("/proyecto".into()), tx);
        let mut job = Job::fake(1, "migration reset");
        job.push_bytes(Stream::Err, b"\x1b[1;32m   Compiling\x1b[0m app\n");
        job.push_bytes(
            Stream::Out,
            "Esto revierte TODAS las migraciones. ¿Continuar? [s/N]: ".as_bytes(),
        );
        app.jobs.push(job);
        app.out_tab = 1;
        let buf = render(&mut app);

        let tabs: Vec<String> = app.hits.tabs.iter().map(|(r, _)| text_at(&buf, *r)).collect();
        assert_eq!(tabs, [" F1 Menú ", " F2 Ayuda ", " F3 Salida ", " F4 Git "]);
        for (r, _) in &app.hits.close {
            assert_eq!(text_at(&buf, *r), " × ");
        }
        let out_tabs: Vec<String> = app.hits.out_tabs.iter().map(|(r, _)| text_at(&buf, *r)).collect();
        assert_eq!(out_tabs, [" Registro ", " ⟳ #1 migration reset "]);
        let first = Rect::new(app.hits.menu_list.x, app.hits.menu_list.y, app.hits.menu_list.width, 1);
        assert!(text_at(&buf, first).contains(MENU[0].label));
    }
}
