//! `CobraTUI` - binario principal.
//! Menu principal sin picker. El picker 30/70 aparece solo al abrir proyecto/archivo.
//!
//! Nota: las conversiones `usize -> u16` usan `try_from` con saturacion:
//! los conteos pueden superar 65535 y en release el overflow es abort.

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
use Editor::picker::{CreateResult, Picker};
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
    /// Sobrescribir este archivo existente con uno vacio (Ctrl+N).
    OverwriteNew(std::path::PathBuf),
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
            .map_or_else(|| "archivo actual".to_string(), util::sanitize);
        match self {
            Pending::OpenPath(p) => {
                let target = p
                    .file_name()
                    .and_then(std::ffi::OsStr::to_str)
                    .map_or_else(|| "archivo".to_string(), util::sanitize);
                format!("'{name}' tiene cambios sin guardar. Abrir '{target}' los descarta.")
            }
            Pending::PickFile => {
                format!("'{name}' tiene cambios sin guardar. Elegir otro archivo los descarta.")
            }
            Pending::NewFile => {
                format!("'{name}' tiene cambios sin guardar. Crear uno nuevo los descarta.")
            }
            Pending::OverwriteNew(p) => {
                let target = p
                    .file_name()
                    .and_then(std::ffi::OsStr::to_str)
                    .map_or_else(|| "archivo".to_string(), util::sanitize);
                format!("'{target}' ya existe y no esta vacio. Sobrescribirlo lo vacia.")
            }
            Pending::ToMenu => {
                format!("'{name}' tiene cambios sin guardar. Volver al menu los descarta.")
            }
            Pending::Quit => {
                format!("'{name}' tiene cambios sin guardar. Salir los descarta.")
            }
        }
    }

    /// Botones del modal según la acción: sobrescribir solo se confirma con S.
    fn hint(&self) -> &'static str {
        match self {
            Pending::OverwriteNew(_) => "[S] Overwrite   [D/Esc] Cancel",
            _ => "[S] Save   [D] Discard   [Esc] Cancel",
        }
    }
}

use util::centered_rect;

/// Endurece el orden de búsqueda de DLLs en Windows: quita el directorio
/// actual para mitigar DLL search-order hijacking si se lanza el editor
/// desde una carpeta no confiable. Solo afecta a cargas por nombre
/// relativo; las DLLs de sistema se siguen resolviendo igual.
#[cfg(windows)]
fn harden_dll_search_order() {
    unsafe extern "system" {
        fn SetDllDirectoryW(path: *const u16) -> i32;
    }
    // Cadena vacía = quita el CWD del orden de búsqueda (NULL lo restauraría).
    let empty: [u16; 1] = [0];
    // SAFETY: puntero a buffer válido de 1 u16 nulo; la API solo lo lee.
    unsafe {
        SetDllDirectoryW(empty.as_ptr());
    }
}

fn main() -> Result<()> {
    #[cfg(windows)]
    harden_dll_search_order();
    // Si algo paniquea con raw mode + alternate screen, la terminal
    // quedaria rota: el hook la restaura antes del hook por defecto.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        disable_raw_mode().ok();
        execute!(io::stdout(), LeaveAlternateScreen).ok();
        default_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    // Si falla algo del setup, restaurar lo ya activado antes de salir.
    if execute!(stdout, EnterAlternateScreen).is_err() {
        disable_raw_mode().ok();
        return Err(anyhow::anyhow!("no se pudo entrar a alternate screen"));
    }
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match Terminal::new(backend) {
        Ok(t) => t,
        Err(e) => {
            disable_raw_mode().ok();
            execute!(io::stdout(), LeaveAlternateScreen).ok();
            return Err(e.into());
        }
    };
    // Cursor propio dibujado: se oculta el nativo para evitar su parpadeo.
    terminal.hide_cursor().ok();

    let mut editor = CobraEditor::new();
    editor.load_persisted_theme();
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

    // Restore siempre, incluso si run_app devolvio Err (best-effort).
    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();

    if let Err(e) = res {
        eprintln!("Error: {e:?}");
    }
    Ok(())
}

/// Rechaza rutas remotas (UNC) con notificación clara. Devuelve `true`
/// si se rechazó (el llamador debe abortar la operación).
fn reject_remote(editor: &mut CobraEditor, path: &std::path::Path) -> bool {
    if util::is_remote(path) {
        editor.notification = Some("ruta remota no soportada".to_string());
        editor.notification_expires =
            Some(std::time::Instant::now() + std::time::Duration::from_secs(2));
        return true;
    }
    false
}

fn open_folder_dialog(picker: &mut Picker, editor: &mut CobraEditor) -> bool {
    disable_raw_mode().ok();
    let folder = rfd::FileDialog::new().pick_folder();
    enable_raw_mode().ok();
    if let Some(path) = folder {
        if reject_remote(editor, &path) {
            return false;
        }
        picker.set_dir(path);
        return true;
    }
    false
}

fn open_file_dialog(editor: &mut CobraEditor) -> bool {
    disable_raw_mode().ok();
    let file = rfd::FileDialog::new().pick_file();
    enable_raw_mode().ok();
    if let Some(path) = file {
        if reject_remote(editor, &path) {
            return false;
        }
        if editor.open_file(path).is_ok() {
            return true;
        }
    }
    false
}

fn notify_open_error(editor: &mut CobraEditor, e: impl std::fmt::Display) {
    editor.notification = Some(format!("falla al abrir: {e}"));
    editor.notification_expires =
        Some(std::time::Instant::now() + std::time::Duration::from_secs(2));
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

/// `true` si crear un archivo en `path` destruiría contenido: existe y
/// no esta vacio. Función pura de decisión (testeable sin diálogo).
fn needs_overwrite_confirm(path: &std::path::Path) -> bool {
    std::fs::metadata(path).is_ok_and(|m| m.len() > 0)
}

/// Diálogo para archivo nuevo. Si la ruta existe y no esta vacía, no la
/// trunca: deja un `Pending::OverwriteNew` en `slot` para confirmar.
/// Devuelve `true` si se creó el archivo.
fn request_new_file(editor: &mut CobraEditor, slot: &mut Option<Pending>) -> bool {
    disable_raw_mode().ok();
    let file = rfd::FileDialog::new().save_file();
    enable_raw_mode().ok();
    let Some(path) = file else {
        return false;
    };
    if reject_remote(editor, &path) {
        return false;
    }
    if needs_overwrite_confirm(&path) {
        *slot = Some(Pending::OverwriteNew(path));
        return false;
    }
    match editor.new_file(path) {
        Ok(()) => true,
        Err(e) => {
            editor.notification = Some(format!("falla al crear: {e}"));
            editor.notification_expires =
                Some(std::time::Instant::now() + std::time::Duration::from_secs(3));
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
    slot: &mut Option<Pending>,
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
            if request_new_file(editor, slot) {
                *mode = AppMode::Editing;
                *focus = Focus::Editor;
            }
        }
        Pending::OverwriteNew(path) => {
            // new_file revalida en el momento de escribir: la ventana
            // TOCTOU del modal queda en microsegundos.
            match editor.new_file(path) {
                Ok(()) => {
                    *mode = AppMode::Editing;
                    *focus = Focus::Editor;
                }
                Err(e) => {
                    editor.notification = Some(format!("falla al crear: {e}"));
                    editor.notification_expires =
                        Some(std::time::Instant::now() + std::time::Duration::from_secs(3));
                }
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
        // Refresco en tiempo real del explorer (barato: un metadata
        // cada 500ms; sin syscalls por frame).
        picker.poll_refresh();
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
                    let theme = editor.theme();
                    // Paleta arriba del todo cuando esta abierta.
                    let (work, palette_rect) = if palette.open {
                        let ph = palette.height().min(area.height.saturating_sub(4).max(4));
                        let rows = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Length(ph),
                                Constraint::Min(0),
                                Constraint::Length(3),
                            ])
                            .split(area);
                        (rows[1], Some(rows[0]))
                    } else {
                        (area, None)
                    };
                    let rows = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Min(0), Constraint::Length(3)])
                        .split(work);
                    let chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                        .split(rows[0]);
                    let picker_focus = *focus == Focus::Picker && !palette.open;
                    let editor_focus = *focus == Focus::Editor && !palette.open;
                    picker.draw(f, chunks[0], picker_focus, &theme);
                    if editor.current_path.is_none() {
                        menu.draw_panel(f, chunks[1]);
                    } else {
                        editor.draw(f, chunks[1], editor_focus);
                    }
                    if let Some(pr) = palette_rect {
                        palette.draw(f, pr, theme.accent);
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
                                util::sanitize,
                            );
                    let mode_label = if *focus == Focus::Picker {
                        "EXPLORER"
                    } else {
                        "EDITOR"
                    };
                    let mut status = format!(
                        " {} · {} · {} · {} lines · {} B · Ln {}, Col {} · {:.0} FPS · Ctrl+P Cmd · Ctrl+S Save · Tab Switch",
                        mode_label,
                        path,
                        editor.theme_name(),
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
                        let w = u16::try_from(msg.chars().count())
                            .unwrap_or(u16::MAX)
                            .saturating_add(6)
                            .min(area.width.saturating_sub(4));
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
                        let body = format!("{msg}\n\n{}", p.hint());
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
                                    && apply_pending(p, editor, focus, mode, &mut pending)
                                {
                                    return Ok(());
                                }
                            } else {
                                pending = None;
                            }
                        }
                        KeyCode::Char('d' | 'D') => {
                            // Sobrescribir solo con S explicita; D cancela.
                            if matches!(pending, Some(Pending::OverwriteNew(_))) {
                                pending = None;
                            } else if let Some(p) = pending.take()
                                && apply_pending(p, editor, focus, mode, &mut pending)
                            {
                                return Ok(());
                            }
                        }
                        _ => {}
                    }
                    continue;
                }
                if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    palette.close();
                    if editor.is_dirty() {
                        pending = Some(Pending::Quit);
                        continue;
                    }
                    return Ok(());
                }
                // Paleta abierta: captura todo menos Ctrl+Q (ya manejado).
                if palette.open {
                    // Ctrl+P tambien la cierra.
                    if key.code == KeyCode::Char('p')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        palette.close();
                        continue;
                    }
                    match palette.handle_key(key) {
                        PaletteAction::None | PaletteAction::Close => {}
                        PaletteAction::Unknown => {
                            editor.notification =
                                Some(format!("unknown command: {}", palette.input()));
                            editor.notification_expires =
                                Some(std::time::Instant::now() + std::time::Duration::from_secs(2));
                        }
                        PaletteAction::ApplyTheme(idx) => {
                            if editor.set_theme(idx) {
                                themes::persist_theme(editor.theme_name());
                                editor.notification =
                                    Some(format!("theme: {}", editor.theme_name()));
                                editor.notification_expires = Some(
                                    std::time::Instant::now() + std::time::Duration::from_secs(2),
                                );
                            }
                        }
                        PaletteAction::Save => {
                            if let Err(e) = editor.save_current() {
                                editor.notification = Some(format!("falla al guardar: {e}"));
                                editor.notification_expires = Some(
                                    std::time::Instant::now() + std::time::Duration::from_secs(3),
                                );
                            }
                        }
                        PaletteAction::PickFile => {
                            if editor.is_dirty() {
                                pending = Some(Pending::PickFile);
                            } else if open_file_dialog(editor) {
                                *focus = Focus::Editor;
                            }
                        }
                        PaletteAction::PickFolder => {
                            if open_folder_dialog(picker, editor) {
                                *focus = Focus::Picker;
                            }
                        }
                        PaletteAction::NewFile => {
                            if editor.is_dirty() {
                                pending = Some(Pending::NewFile);
                            } else if request_new_file(editor, &mut pending) {
                                *focus = Focus::Editor;
                            }
                        }
                        PaletteAction::Quit => {
                            if editor.is_dirty() {
                                pending = Some(Pending::Quit);
                            } else {
                                return Ok(());
                            }
                        }
                    }
                    continue;
                }

                match mode {
                    AppMode::Menu => {
                        if key.code == KeyCode::Char('k')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            if open_folder_dialog(picker, editor) {
                                *mode = AppMode::Editing;
                                *focus = Focus::Picker;
                            }
                            continue;
                        }
                        if (key.code == KeyCode::Char('o') || key.code == KeyCode::Char('0'))
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
                            if request_new_file(editor, &mut pending) {
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
                        // Entrada de nombre (Ctrl+A): captura todo hasta
                        // Enter/Esc, antes que cualquier otro atajo.
                        if picker.creating {
                            match picker.create_key(key) {
                                CreateResult::Pending => {}
                                CreateResult::Invalid => {
                                    editor.notification = Some(
                                        "nombre inválido (solo nombre, sin rutas)".to_string(),
                                    );
                                    editor.notification_expires = Some(
                                        std::time::Instant::now()
                                            + std::time::Duration::from_secs(2),
                                    );
                                }
                                CreateResult::Cancelled => {}
                                CreateResult::Confirmed(path) => {
                                    if needs_overwrite_confirm(&path) {
                                        pending = Some(Pending::OverwriteNew(path));
                                    } else {
                                        match editor.new_file(path.clone()) {
                                            Ok(()) => {
                                                picker.reload();
                                                if let Some(n) =
                                                    path.file_name().and_then(|n| n.to_str())
                                                {
                                                    picker.select_name(n);
                                                }
                                                *focus = Focus::Editor;
                                            }
                                            Err(e) => {
                                                editor.notification =
                                                    Some(format!("falla al crear: {e}"));
                                                editor.notification_expires = Some(
                                                    std::time::Instant::now()
                                                        + std::time::Duration::from_secs(3),
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            continue;
                        }
                        if key.code == KeyCode::Char('k')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            if open_folder_dialog(picker, editor) {
                                *focus = Focus::Picker;
                            }
                            continue;
                        }
                        if (key.code == KeyCode::Char('o') || key.code == KeyCode::Char('0'))
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
                        if key.code == KeyCode::Char('p')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            palette.toggle();
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
                            // Ctrl+D borra la entrada seleccionada
                            // (archivos siempre, carpetas solo vacias).
                            if matches!(key.code, KeyCode::Char('d' | 'D'))
                                && key.modifiers.contains(KeyModifiers::CONTROL)
                            {
                                if picker.selected_file().is_some_and(|n| n != "..") {
                                    match picker.remove_selected() {
                                        Ok(full) => {
                                            if editor.current_path.as_ref() == Some(&full) {
                                                editor.close_file();
                                            }
                                            let name = full
                                                .file_name()
                                                .and_then(|n| n.to_str())
                                                .unwrap_or("archivo");
                                            editor.notification =
                                                Some(format!("eliminado: {name}"));
                                            editor.notification_expires = Some(
                                                std::time::Instant::now()
                                                    + std::time::Duration::from_secs(2),
                                            );
                                        }
                                        Err(e) => {
                                            editor.notification = Some(e);
                                            editor.notification_expires = Some(
                                                std::time::Instant::now()
                                                    + std::time::Duration::from_secs(3),
                                            );
                                        }
                                    }
                                }
                                continue;
                            }
                            // Ctrl+A pide el nombre para crear un archivo.
                            if matches!(key.code, KeyCode::Char('a' | 'A'))
                                && key.modifiers.contains(KeyModifiers::CONTROL)
                            {
                                picker.begin_create();
                                continue;
                            }
                            match key.code {
                                KeyCode::Up => picker.move_up(),
                                KeyCode::Down => picker.move_down(),
                                KeyCode::Backspace => {
                                    if let Some(parent) = picker.current_dir.parent() {
                                        picker.set_dir(parent.to_path_buf());
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
                                        if reject_remote(editor, &full) {
                                            continue;
                                        }
                                        if full.is_dir() {
                                            picker.set_dir(full);
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
                                if request_new_file(editor, &mut pending) {
                                    *focus = Focus::Editor;
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

#[cfg(test)]
mod tests {
    use super::needs_overwrite_confirm;

    fn test_path(name: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(name);
        p
    }

    #[test]
    fn test_overwrite_solo_si_existe_y_no_vacio() {
        let missing = test_path("cobra_sec_missing_unit.txt");
        let _ = std::fs::remove_file(&missing);
        assert!(!needs_overwrite_confirm(&missing));

        let empty = test_path("cobra_sec_empty_unit.txt");
        std::fs::write(&empty, []).unwrap();
        assert!(!needs_overwrite_confirm(&empty));

        let full = test_path("cobra_sec_full_unit.txt");
        std::fs::write(&full, "contenido").unwrap();
        assert!(needs_overwrite_confirm(&full));

        let _ = std::fs::remove_file(&empty);
        let _ = std::fs::remove_file(&full);
    }
}
