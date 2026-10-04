//! `CobraTUI` - binario principal.
//! Menu principal sin picker. El picker 30/70 aparece solo al abrir proyecto/archivo.
//!
//! Nota: los casts `usize as u16` son seguros aqui porque las dimensiones
//! del terminal ya vienen como `u16` en `crossterm`/`ratatui`.
#![allow(clippy::cast_possible_truncation)]

#[allow(non_snake_case)]
mod Editor;
#[allow(non_snake_case)]
mod Menu;
#[allow(non_snake_case)]
mod OxideEngine;
mod highlight;
mod palette;
mod themes;
mod util;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};
use std::io;

use Editor::editor::Editor as CobraEditor;
use Editor::picker::Picker;
use Menu::menu::Menu as CobraMenu;
use OxideEngine::oxide::Buffer;
use palette::{Palette, PaletteAction};

#[derive(PartialEq)]
enum Focus {
    Picker,
    Editor,
}

enum AppMode {
    Menu,
    Editing,
}

/// Accion en espera mientras el modal de cambios sin guardar esta abierto.
enum Pending {
    /// Abrir este archivo (viene del picker Enter).
    OpenPath(std::path::PathBuf),
    /// Abrir archivo con dialogo (Ctrl+O).
    PickFile,
    /// Crear archivo con dialogo (Ctrl+N).
    NewFile,
    /// Volver al menu (Esc).
    ToMenu,
    /// Salir de la app (Ctrl+Q o Esc final).
    Quit,
}

impl Pending {
    fn describe(&self, editor: &CobraEditor) -> String {
        let name = editor
            .current_path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("archivo actual");
        match self {
            Pending::OpenPath(p) => {
                let target = p
                    .file_name()
                    .and_then(std::ffi::OsStr::to_str)
                    .unwrap_or("archivo");
                format!(
                    "'{name}' tiene cambios sin guardar. Abrir '{target}' los descarta."
                )
            }
            Pending::PickFile => format!(
                "'{name}' tiene cambios sin guardar. Elegir otro archivo los descarta."
            ),
            Pending::NewFile => format!(
                "'{name}' tiene cambios sin guardar. Crear uno nuevo los descarta."
            ),
            Pending::ToMenu => format!(
                "'{name}' tiene cambios sin guardar. Volver al menu los descarta."
            ),
            Pending::Quit => {
                format!("'{name}' tiene cambios sin guardar. Salir los descarta.")
            }
        }
    }
}

use util::centered_rect;

fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    // Cursor propio dibujado: se oculta el nativo para evitar su parpadeo.
    terminal.hide_cursor().ok();

    let mut editor = CobraEditor::new();
    editor.buffer = Buffer::new("");
    let mut picker = Picker::new();
    let mut menu = CobraMenu::new();
    let mut palette = Palette::new();
    let mut focus = Focus::Editor;
    let mut mode = AppMode::Menu;

    let res = run_app(
        &mut terminal,
        &mut editor,
        &mut picker,
        &mut menu,
        &mut palette,
        &mut focus,
        &mut mode,
    );

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = res {
        eprintln!("Error: {e:?}");
    }
    Ok(())
}

fn open_folder_dialog(picker: &mut Picker) -> bool {
    disable_raw_mode().ok();
    let folder = rfd::FileDialog::new().pick_folder();
    enable_raw_mode().ok();
    if let Some(path) = folder {
        picker.current_dir = path;
        picker.reload();
        return true;
    }
    false
}

fn open_file_dialog(editor: &mut CobraEditor) -> bool {
    disable_raw_mode().ok();
    let file = rfd::FileDialog::new().pick_file();
    enable_raw_mode().ok();
    if let Some(path) = file
        && editor.open_file(path).is_ok()
    {
        return true;
    }
    false
}

fn notify_open_error(editor: &mut CobraEditor, e: impl std::fmt::Display) {
    editor.notification = Some(format!("falla al abrir: {e}"));
    editor.notification_expires = Some(
        std::time::Instant::now() + std::time::Duration::from_secs(2),
    );
}

/// Abre `path` en el editor. Devuelve `true` si se abrio.
fn open_buffer_at(editor: &mut CobraEditor, path: std::path::PathBuf) -> bool {
    let Some(path_str) = path.to_str().map(str::to_owned) else {
        notify_open_error(editor, "ruta no UTF-8");
        return false;
    };
    match Buffer::from_file(&path_str) {
        Ok(buf) => {
            editor.open_buffer(buf, path);
            true
        }
        Err(e) => {
            notify_open_error(editor, e);
            false
        }
    }
}

/// Ejecuta la accion pendiente tras confirmar (guardada o descartada).
/// Devuelve `true` si hay que salir de la app.
fn apply_pending(
    pending: Pending,
    editor: &mut CobraEditor,
    focus: &mut Focus,
    mode: &mut AppMode,
) -> bool {
    match pending {
        Pending::OpenPath(path) => {
            if open_buffer_at(editor, path) {
                *focus = Focus::Editor;
            }
            if matches!(mode, AppMode::Menu) {
                *mode = AppMode::Editing;
            }
        }
        Pending::PickFile => {
            if open_file_dialog(editor) {
                *mode = AppMode::Editing;
                *focus = Focus::Editor;
            }
        }
        Pending::NewFile => {
            disable_raw_mode().ok();
            let file = rfd::FileDialog::new().save_file();
            enable_raw_mode().ok();
            if let Some(path) = file {
                editor.new_file(path);
                *mode = AppMode::Editing;
                *focus = Focus::Editor;
            }
        }
        Pending::ToMenu => {
            *mode = AppMode::Menu;
        }
        Pending::Quit => {
            return true;
        }
    }
    false
}

/// Loop principal de eventos y render (60fps). Largo por naturaleza:
/// concentra draw + `poll` + `match` de modos en un solo lugar.
#[allow(clippy::too_many_lines)]
fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    editor: &mut CobraEditor,
    picker: &mut Picker,
    menu: &mut CobraMenu,
    palette: &mut Palette,
    focus: &mut Focus,
    mode: &mut AppMode,
) -> Result<()> {
    let mut last_frame = std::time::Instant::now();
    let mut fps: f64 = 60.0;
    let mut pending: Option<Pending> = None;
    loop {
        // FPS suavizado para la barra de estado
        let now = std::time::Instant::now();
        let dt = now.duration_since(last_frame).as_secs_f64();
        last_frame = now;
        if dt > 0.0 {
            let inst = 1.0 / dt;
            fps = fps * 0.9 + inst * 0.1;
        }
        match mode {
            AppMode::Menu => {
                terminal.draw(|f| {
                    let area = f.size();
                    f.render_widget(Clear, area);
                    menu.draw(f, area);
                })?;
            }
            AppMode::Editing => {
                terminal.draw(|f| {
                    let area = f.size();
                    f.render_widget(Clear, area);
                    let rows = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Min(0), Constraint::Length(3)])
                        .split(area);
                    let chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                        .split(rows[0]);
                    picker.draw(f, chunks[0], *focus == Focus::Picker);
                    if editor.current_path.is_none() {
                        menu.draw_panel(f, chunks[1]);
                    } else {
                        editor.draw(f, chunks[1], *focus == Focus::Editor);
                    }

                    let (cx, cy) = editor.buffer.cursor();
                    let nlines = editor.buffer.lines().len();
                    let nbytes = editor.buffer.byte_len();
                    let path = editor
                        .current_path
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .and_then(std::ffi::OsStr::to_str)
                        .map_or_else(
                            || {
                                picker
                                    .current_dir
                                    .file_name()
                                    .and_then(std::ffi::OsStr::to_str)
                                    .unwrap_or("No folder")
                                    .to_owned()
                            },
                            str::to_owned,
                        );
                    let mode_label = if *focus == Focus::Picker {
                        "EXPLORER"
                    } else {
                        "EDITOR"
                    };
                    let mut status = format!(
                        " {} · {} · {} lines · {} B · Ln {}, Col {} · {:.0} FPS · Ctrl+S Save · Tab Switch · Esc Menu",
                        mode_label,
                        path,
                        nlines,
                        nbytes,
                        cy + 1,
                        cx + 1,
                        fps.clamp(0.0, 999.0),
                    );
                    let bar_block = Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::White));
                    let bar_inner = bar_block.inner(rows[1]);
                    f.render_widget(bar_block, rows[1]);
                    // Trunca por chars (no por bytes) para no partir
                    // caracteres como `·` y nunca desbordar el render
                    let max_w = bar_inner.width as usize;
                    if status.chars().count() > max_w {
                        status = status.chars().take(max_w).collect();
                    }
                    let bar = Paragraph::new(status)
                        .style(Style::default().fg(Color::White));
                    f.render_widget(bar, bar_inner);

                    if let Some(p) = pending.as_ref() {
                        let msg = p.describe(editor);
                        let w = (msg.chars().count() as u16 + 6).min(area.width.saturating_sub(4));
                        let popup = centered_rect(w, 7, area);
                        f.render_widget(Clear, popup);
                        let block = Block::default()
                            .title(" Unsaved changes ")
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(Color::Yellow));
                        f.render_widget(block, popup);
                        let inner = ratatui::layout::Rect {
                            x: popup.x + 2,
                            y: popup.y + 1,
                            width: popup.width.saturating_sub(4),
                            height: popup.height.saturating_sub(2),
                        };
                        let body =
                        format!("{msg}\n\n[S] Save   [D] Discard   [Esc] Cancel");
                        let p = Paragraph::new(body)
                            .alignment(ratatui::layout::Alignment::Center)
                            .style(Style::default().fg(Color::White));
                        f.render_widget(p, inner);
                    }
                })?;
            }
        }

        // Intencionalmente anidado: `read` solo se evalua si `poll` dio true.
        #[allow(clippy::collapsible_if)]
        if event::poll(std::time::Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                // Modal de confirmacion: bloquea todo lo demas
                if pending.is_some() {
                    match key.code {
                        KeyCode::Esc => {
                            pending = None;
                        }
                        KeyCode::Char('s' | 'S') => {
                            if editor.save_current().is_ok() {
                                if let Some(p) = pending.take()
                                    && apply_pending(p, editor, focus, mode)
                                {
                                    return Ok(());
                                }
                            } else {
                                pending = None;
                            }
                        }
                        KeyCode::Char('d' | 'D') => {
                            if let Some(p) = pending.take()
                                && apply_pending(p, editor, focus, mode)
                            {
                                return Ok(());
                            }
                        }
                        _ => {}
                    }
                    continue;
                }
                if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    if editor.is_dirty() {
                        pending = Some(Pending::Quit);
                        continue;
                    }
                    return Ok(());
                }

                match mode {
                    AppMode::Menu => {
                        if key.code == KeyCode::Char('k')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            if open_folder_dialog(picker) {
                                *mode = AppMode::Editing;
                                *focus = Focus::Picker;
                            }
                            continue;
                        }
                        if (key.code == KeyCode::Char('o')
                            || key.code == KeyCode::Char('0'))
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            if open_file_dialog(editor) {
                                *mode = AppMode::Editing;
                                *focus = Focus::Editor;
                            }
                            continue;
                        }
                        if key.code == KeyCode::Char('n')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            disable_raw_mode().ok();
                            let file = rfd::FileDialog::new().save_file();
                            enable_raw_mode().ok();
                            if let Some(path) = file {
                                editor.new_file(path);
                                *mode = AppMode::Editing;
                                *focus = Focus::Editor;
                            }
                            continue;
                        }
                        if let Some(enter) = menu.handle_key(key) {
                            if enter {
                                *mode = AppMode::Editing;
                                *focus = Focus::Picker;
                            } else {
                                return Ok(());
                            }
                        }
                    }
                    AppMode::Editing => {
                        if key.code == KeyCode::Char('k')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            if open_folder_dialog(picker) {
                                *focus = Focus::Picker;
                            }
                            continue;
                        }
                        if (key.code == KeyCode::Char('o')
                            || key.code == KeyCode::Char('0'))
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            if editor.is_dirty() {
                                pending = Some(Pending::PickFile);
                                continue;
                            }
                            if open_file_dialog(editor) {
                                *focus = Focus::Editor;
                            }
                            continue;
                        }
                        if key.code == KeyCode::Tab {
                            *focus = if *focus == Focus::Picker {
                                Focus::Editor
                            } else {
                                Focus::Picker
                            };
                            continue;
                        }

                        if *focus == Focus::Picker {
                            match key.code {
                                KeyCode::Up => picker.move_up(),
                                KeyCode::Down => picker.move_down(),
                                KeyCode::Backspace => {
                                    if let Some(parent) = picker.current_dir.parent() {
                                        picker.current_dir = parent.to_path_buf();
                                        picker.reload();
                                    }
                                }
                                KeyCode::Enter => {
                                    if let Some(name) = picker.selected_file().cloned() {
                                        if name == ".." {
                                            if let Some(parent) = picker.current_dir.parent() {
                                                picker.current_dir = parent.to_path_buf();
                                                picker.reload();
                                            }
                                            continue;
                                        }
                                        let full = picker.current_dir.join(&name);
                                        if full.is_dir() {
                                            picker.current_dir = full;
                                            picker.reload();
                                        } else if editor.is_dirty() {
                                            pending = Some(Pending::OpenPath(full));
                                        } else if open_buffer_at(editor, full) {
                                            *focus = Focus::Editor;
                                        }
                                    }
                                }
                                KeyCode::Esc => {
                                    if editor.is_dirty() {
                                        pending = Some(Pending::ToMenu);
                                    } else {
                                        *mode = AppMode::Menu;
                                    }
                                }
                                _ => {}
                            }
                        } else {
                            if editor.current_path.is_none() {
                                if key.code == KeyCode::Esc {
                                    *mode = AppMode::Menu;
                                }
                                continue;
                            }
                            // Ctrl+N vive aca (no en editor) para salir de raw mode
                            // ante el dialogo y avisar si hay cambios sin guardar
                            if key.code == KeyCode::Char('n')
                                && key.modifiers.contains(KeyModifiers::CONTROL)
                            {
                                if editor.is_dirty() {
                                    pending = Some(Pending::NewFile);
                                    continue;
                                }
                                disable_raw_mode().ok();
                                let file = rfd::FileDialog::new().save_file();
                                enable_raw_mode().ok();
                                if let Some(path) = file {
                                    editor.new_file(path);
                                }
                                continue;
                            }
                            if !editor.handle_key(key) {
                                if editor.is_dirty() {
                                    pending = Some(Pending::ToMenu);
                                } else {
                                    *mode = AppMode::Menu;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
