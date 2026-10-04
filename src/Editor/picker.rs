use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
};

struct Entry {
    name: String,
    is_dir: bool,
}

pub struct Picker {
    files: Vec<Entry>,
    selected: usize,
    pub current_dir: std::path::PathBuf,
}

impl Picker {
    pub fn new() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let mut picker = Self {
            files: Vec::new(),
            selected: 0,
            current_dir,
        };
        picker.reload();
        picker
    }
    pub fn draw(&self, f: &mut Frame, area: Rect, focused: bool, theme: &crate::themes::Theme) {
        // Etiquetas precomputadas en `reload`: sin syscalls por frame.
        let items: Vec<ListItem> = self
            .files
            .iter()
            .map(|e| {
                let label = if e.name == ".." {
                    "  ..".to_string()
                } else if e.is_dir {
                    format!("▸ {}/", e.name)
                } else {
                    format!("  {}", e.name)
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
        f.render_stateful_widget(list, area, &mut state);
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
    pub fn reload(&mut self) {
        let mut files = Vec::new();
        // Añade ".." para volver atrás si no estamos en raíz
        if self.current_dir.parent().is_some() {
            files.push(Entry {
                name: "..".to_string(),
                is_dir: true,
            });
        }
        // Sin unwrap: si el dir falla (permiso/eliminado) queda lista con "..".
        if let Ok(rd) = std::fs::read_dir(&self.current_dir) {
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
            files.extend(rest);
        }
        self.files = files;
        self.selected = 0;
    }
}
