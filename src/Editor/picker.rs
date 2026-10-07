use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
};
use std::time::{Duration, Instant};

struct Entry {
    name: String,
    is_dir: bool,
}

/// Maximo de entradas listadas: colectar+ordenar un directorio gigante
/// en el hilo de eventos congelaria la UI. Muy por encima de cualquier
/// carpeta normal (System32 ronda 5k).
const MAX_ENTRIES: usize = 10_000;

/// Maximo de caracteres del nombre al crear un archivo.
const CREATE_MAX: usize = 64;

/// Cada cuanto se revisa el directorio en busca de cambios externos (ms).
const POLL_MS: u64 = 500;

/// Resultado de una tecla en modo crear-archivo.
pub enum CreateResult {
    /// Sigue editando el nombre.
    Pending,
    /// Nombre invalido (vacio no: eso cancela; separadores o `.`/`..`):
    /// sigue editando, el llamador avisa.
    Invalid,
    /// Nombre valido: ruta completa lista para crear.
    Confirmed(std::path::PathBuf),
    /// Cancelado (Esc o Enter con nombre vacio).
    Cancelled,
}

pub struct Picker {
    files: Vec<Entry>,
    selected: usize,
    pub current_dir: std::path::PathBuf,
    /// Entrada de nombre activa (Ctrl+A).
    pub creating: bool,
    create_input: String,
    create_cursor: usize,
    last_poll: Instant,
    last_mtime: Option<std::time::SystemTime>,
}

impl Picker {
    pub fn new() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let last_mtime = std::fs::metadata(&current_dir)
            .and_then(|m| m.modified())
            .ok();
        let mut picker = Self {
            files: Vec::new(),
            selected: 0,
            current_dir,
            creating: false,
            create_input: String::new(),
            create_cursor: 0,
            last_poll: Instant::now(),
            last_mtime,
        };
        picker.reload();
        picker
    }
    pub fn draw(&self, f: &mut Frame, area: Rect, focused: bool, theme: &crate::themes::Theme) {
        // Con la entrada de nombre activa, la lista cede 3 filas abajo.
        let (list_area, input_area) = if self.creating {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(3)])
                .split(area);
            (rows[0], Some(rows[1]))
        } else {
            (area, None)
        };
        // Etiquetas precomputadas en `reload`: sin syscalls por frame.
        let items: Vec<ListItem> = self
            .files
            .iter()
            .map(|e| {
                // Nombres sanitizados: en Linux un nombre puede traer ESC.
                let name = crate::util::sanitize(&e.name);
                let label = if name == ".." {
                    "  ..".to_string()
                } else if e.is_dir {
                    format!("▸ {name}/")
                } else {
                    format!("  {name}")
                };
                ListItem::new(label)
            })
            .collect();
        let folder = self
            .current_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Explorer");
        let block = Block::default()
            .title(format!(" Explorer · {folder} "))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if focused {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.border_dim)
            });
        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("› ");
        let mut state = ListState::default();
        state.select(Some(self.selected));
        f.render_stateful_widget(list, list_area, &mut state);

        if let Some(ia) = input_area {
            let iblock = Block::default()
                .title(" Nuevo archivo — Enter crea, Esc cancela ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.accent));
            let inner = iblock.inner(ia);
            f.render_widget(iblock, ia);
            // Entrada sanitizada para display; indice sobre el texto
            // sanitizado (1:1 en chars, igual que la paleta).
            let shown = crate::util::sanitize(&self.create_input);
            let bi = shown
                .char_indices()
                .nth(self.create_cursor)
                .map_or(shown.len(), |(i, _)| i);
            let before = &shown[..bi];
            let cur = shown[bi..]
                .chars()
                .next()
                .map_or_else(|| " ".to_string(), |c| c.to_string());
            let after_b = shown[bi..]
                .char_indices()
                .nth(1)
                .map_or(shown.len(), |(i, _)| bi + i);
            let after = &shown[after_b..];
            let line = Line::from(vec![
                Span::styled(
                    "> ",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(before),
                Span::styled(cur, Style::default().bg(Color::White).fg(Color::Black)),
                Span::raw(after),
            ]);
            f.render_widget(Paragraph::new(line), inner);
        }
    }
    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }
    pub fn move_down(&mut self) {
        if self.selected + 1 < self.files.len() {
            self.selected += 1;
        }
    }
    pub fn selected_file(&self) -> Option<&String> {
        self.files.get(self.selected).map(|e| &e.name)
    }
    /// Mueve la seleccion a `name` si existe (tras crear un archivo).
    pub fn select_name(&mut self, name: &str) {
        if let Some(i) = self.files.iter().position(|e| e.name == name) {
            self.selected = i;
        }
    }
    /// Cambia de directorio: recarga y resincroniza la baseline del
    /// poll. Usar SIEMPRE en vez de asignar `current_dir` a mano: la
    /// baseline pertenece al directorio vigilado, no al anterior
    /// (si coinciden en el mismo tick del reloj, el poll no detecta
    /// nada y la lista queda stale).
    pub fn set_dir(&mut self, dir: std::path::PathBuf) {
        self.current_dir = dir;
        self.last_mtime = std::fs::metadata(&self.current_dir)
            .and_then(|m| m.modified())
            .ok();
        self.last_poll = Instant::now();
        self.reload();
    }

    pub fn reload(&mut self) {
        // Sin unwrap: si el dir falla (permiso/eliminado) queda lista con "..".
        self.files = self.read_entries().unwrap_or_else(|| {
            let mut files = Vec::new();
            if self.current_dir.parent().is_some() {
                files.push(Entry {
                    name: "..".to_string(),
                    is_dir: true,
                });
            }
            files
        });
        self.selected = 0;
    }

    /// Lee las entradas ordenadas (dirs primero) o `None` si falla.
    fn read_entries(&self) -> Option<Vec<Entry>> {
        let mut files = Vec::new();
        // Añade ".." para volver atrás si no estamos en raíz
        if self.current_dir.parent().is_some() {
            files.push(Entry {
                name: "..".to_string(),
                is_dir: true,
            });
        }
        let rd = std::fs::read_dir(&self.current_dir).ok()?;
        let mut rest: Vec<Entry> = rd
            .filter_map(Result::ok)
            .map(|e| {
                let is_dir = e.file_type().is_ok_and(|t| t.is_dir());
                Entry {
                    name: e.file_name().to_string_lossy().to_string(),
                    is_dir,
                }
            })
            .collect();
        // Directorios primero, luego alfabetico insensible a mayusculas.
        rest.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        rest.truncate(MAX_ENTRIES);
        files.extend(rest);
        Some(files)
    }

    /// Revisa el directorio cada `POLL_MS` y recarga si cambio por fuera
    /// (otro programa, el propio editor). Conserva la seleccion por nombre.
    /// Barato: un `metadata` por intervalo, sin syscalls por frame.
    /// Si la relectura falla (transitorio bajo carga), conserva la lista
    /// y la baseline para reintentar en el proximo intervalo.
    pub fn poll_refresh(&mut self) {
        if self.last_poll.elapsed() < Duration::from_millis(POLL_MS) {
            return;
        }
        self.last_poll = Instant::now();
        let mtime = std::fs::metadata(&self.current_dir)
            .and_then(|m| m.modified())
            .ok();
        if mtime == self.last_mtime {
            return;
        }
        let sel = self.selected_file().cloned();
        let Some(files) = self.read_entries() else {
            return;
        };
        self.last_mtime = mtime;
        self.files = files;
        self.selected = 0;
        if let Some(name) = sel {
            self.select_name(&name);
        }
    }

    /// Empieza la entrada de nombre para crear un archivo (Ctrl+A).
    pub fn begin_create(&mut self) {
        self.creating = true;
        self.create_input.clear();
        self.create_cursor = 0;
    }

    fn create_byte_idx(&self) -> usize {
        self.create_input
            .char_indices()
            .nth(self.create_cursor)
            .map_or(self.create_input.len(), |(i, _)| i)
    }

    /// Una tecla en modo crear-archivo. Solo acepta texto sin Ctrl/Alt;
    /// Enter confirma (o cancela si esta vacio), Esc cancela.
    pub fn create_key(&mut self, key: KeyEvent) -> CreateResult {
        if key.kind != KeyEventKind::Press {
            return CreateResult::Pending;
        }
        match key.code {
            KeyCode::Esc => {
                self.creating = false;
                self.create_input.clear();
                self.create_cursor = 0;
                CreateResult::Cancelled
            }
            KeyCode::Enter => {
                let name = self.create_input.trim().to_string();
                if name.is_empty() {
                    self.creating = false;
                    self.create_input.clear();
                    self.create_cursor = 0;
                    return CreateResult::Cancelled;
                }
                // Solo nombre plano: sin separadores ni fugas con `..`.
                if name == "." || name == ".." || name.contains('/') || name.contains('\\') {
                    return CreateResult::Invalid;
                }
                self.creating = false;
                let path = self.current_dir.join(&name);
                self.create_input.clear();
                self.create_cursor = 0;
                CreateResult::Confirmed(path)
            }
            KeyCode::Backspace => {
                if self.create_cursor > 0 && !self.create_input.is_empty() {
                    let bi = self.create_byte_idx();
                    let prev = self.create_input[..bi]
                        .char_indices()
                        .next_back()
                        .map_or(0, |(i, _)| i);
                    self.create_input.drain(prev..bi);
                    self.create_cursor -= 1;
                }
                CreateResult::Pending
            }
            KeyCode::Left => {
                self.create_cursor = self.create_cursor.saturating_sub(1);
                CreateResult::Pending
            }
            KeyCode::Right => {
                let max = self.create_input.chars().count();
                if self.create_cursor < max {
                    self.create_cursor += 1;
                }
                CreateResult::Pending
            }
            KeyCode::Char(c) => {
                if !key.modifiers.is_empty() {
                    return CreateResult::Pending;
                }
                if self.create_input.chars().count() >= CREATE_MAX {
                    return CreateResult::Pending;
                }
                let bi = self.create_byte_idx();
                self.create_input.insert(bi, c);
                self.create_cursor += 1;
                CreateResult::Pending
            }
            _ => CreateResult::Pending,
        }
    }

    /// Borra la entrada seleccionada: archivos siempre, carpetas solo si
    /// estan vacias. Devuelve la ruta borrada y deja la seleccion valida.
    /// `".."` se rechaza (el llamador lo filtra en silencio).
    pub fn remove_selected(&mut self) -> Result<std::path::PathBuf, String> {
        let name = self
            .selected_file()
            .cloned()
            .ok_or_else(|| "nada seleccionado".to_string())?;
        if name == ".." {
            return Err(".. no se borra".to_string());
        }
        let full = self.current_dir.join(&name);
        if full.is_dir() {
            std::fs::remove_dir(&full).map_err(|e| {
                if e.kind() == std::io::ErrorKind::DirectoryNotEmpty {
                    "carpeta no vacía: vaciala primero".to_string()
                } else {
                    format!("no se pudo borrar: {e}")
                }
            })?;
        } else {
            std::fs::remove_file(&full).map_err(|e| format!("no se pudo borrar: {e}"))?;
        }
        let idx = self.selected;
        self.reload();
        self.selected = idx.min(self.files.len().saturating_sub(1));
        Ok(full)
    }
}

#[cfg(test)]
mod tests {
    use super::{CreateResult, Picker};
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    fn tmpdir(tag: &str) -> std::path::PathBuf {
        let mut d = std::env::temp_dir();
        d.push(format!("cobra_picker_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn test_create_confirma_nombre() {
        let mut p = Picker::new();
        p.current_dir = tmpdir("create");
        p.begin_create();
        assert!(p.creating);
        for c in "hola.txt".chars() {
            assert!(matches!(
                p.create_key(key(KeyCode::Char(c))),
                CreateResult::Pending
            ));
        }
        match p.create_key(key(KeyCode::Enter)) {
            CreateResult::Confirmed(path) => {
                assert_eq!(path.file_name().unwrap(), "hola.txt")
            }
            _ => panic!("debia confirmar"),
        }
        assert!(!p.creating);
        let _ = std::fs::remove_dir_all(p.current_dir.clone());
    }

    #[test]
    fn test_create_rechaza_separadores_y_cancela() {
        let mut p = Picker::new();
        p.begin_create();
        for c in "a/b".chars() {
            p.create_key(key(KeyCode::Char(c)));
        }
        assert!(matches!(
            p.create_key(key(KeyCode::Enter)),
            CreateResult::Invalid
        ));
        assert!(p.creating, "sigue editando tras invalido");
        assert!(matches!(
            p.create_key(key(KeyCode::Esc)),
            CreateResult::Cancelled
        ));
        assert!(!p.creating);
        // Enter vacio tambien cancela en silencio.
        p.begin_create();
        assert!(matches!(
            p.create_key(key(KeyCode::Enter)),
            CreateResult::Cancelled
        ));
    }

    #[test]
    fn test_remove_selected_archivo() {
        let mut p = Picker::new();
        let dir = tmpdir("rm");
        std::fs::write(dir.join("borrar.txt"), "x").unwrap();
        std::fs::write(dir.join("quedate.txt"), "y").unwrap();
        p.set_dir(dir.clone());
        p.select_name("borrar.txt");
        let gone = p.remove_selected().unwrap();
        assert_eq!(gone, dir.join("borrar.txt"));
        assert!(!dir.join("borrar.txt").exists());
        assert!(dir.join("quedate.txt").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_remove_selected_carpeta_no_vacia_falla() {
        let mut p = Picker::new();
        let dir = tmpdir("rmdir");
        let sub = dir.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("dentro.txt"), "x").unwrap();
        p.set_dir(dir.clone());
        p.select_name("sub");
        assert!(p.remove_selected().is_err());
        assert!(sub.exists(), "no se toco");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_poll_detecta_cambios_externos() {
        let mut p = Picker::new();
        let dir = tmpdir("poll");
        p.set_dir(dir.clone());
        // El reloj del SO avanza a saltos (~15.6ms): separar la baseline
        // de la escritura para que el mtime caiga en otro tick seguro.
        std::thread::sleep(std::time::Duration::from_millis(25));
        std::fs::write(dir.join("nuevo.txt"), "x").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(600));
        p.poll_refresh();
        assert!(
            p.files.iter().any(|e| e.name == "nuevo.txt"),
            "debio recargar"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
