//! Paleta de comandos de `CobraTUI` (`Ctrl+P`).
//!
//! Entrada de texto con sugerencias filtradas. Los comandos son literales
//! estrictos (sin eval ni I/O): `/themes`, `/save`, `/open`, `/folder`,
//! `/new`, `/quit`. La entrada esta acotada a `MAX_INPUT` caracteres y
//! las sugerencias a `MAX_SUGGESTIONS`.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

/// Maximo de caracteres de la entrada (seguridad + rendimiento).
pub const MAX_INPUT: usize = 64;
/// Maximo de sugerencias visibles.
pub const MAX_SUGGESTIONS: usize = 6;

/// Accion que `main` debe ejecutar tras la paleta.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaletteAction {
    None,
    Close,
    Unknown,
    ApplyTheme(usize),
    Save,
    PickFile,
    PickFolder,
    NewFile,
    Quit,
}

struct Command {
    name: &'static str,
    desc: &'static str,
    action: PaletteAction,
}

const COMMANDS: &[Command] = &[
    Command {
        name: "/themes",
        desc: "Choose color theme",
        action: PaletteAction::ApplyTheme(usize::MAX),
    },
    Command {
        name: "/save",
        desc: "Save current file",
        action: PaletteAction::Save,
    },
    Command {
        name: "/open",
        desc: "Open file",
        action: PaletteAction::PickFile,
    },
    Command {
        name: "/folder",
        desc: "Open folder",
        action: PaletteAction::PickFolder,
    },
    Command {
        name: "/new",
        desc: "New file",
        action: PaletteAction::NewFile,
    },
    Command {
        name: "/quit",
        desc: "Quit",
        action: PaletteAction::Quit,
    },
];

/// Sugerencia visible con su accion.
#[derive(Clone, Debug)]
pub struct Suggestion {
    pub label: String,
    pub hint: &'static str,
    pub action: PaletteAction,
}

pub struct Palette {
    pub open: bool,
    input: String,
    cursor: usize,
    selected: usize,
}

impl Palette {
    pub fn new() -> Self {
        Self {
            open: false,
            input: String::new(),
            cursor: 0,
            selected: 0,
        }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            self.input.clear();
            self.cursor = 0;
            self.selected = 0;
        }
    }

    pub fn close(&mut self) {
        self.open = false;
        self.input.clear();
        self.cursor = 0;
        self.selected = 0;
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    fn byte_idx(&self) -> usize {
        self.input
            .char_indices()
            .nth(self.cursor)
            .map_or(self.input.len(), |(i, _)| i)
    }

    /// Sugerencias filtradas segun la entrada actual.
    pub fn suggestions(&self) -> Vec<Suggestion> {
        let q = self.input.trim().to_lowercase();
        // Modo /themes: lista temas filtrados por el texto posterior.
        if q == "/themes" || q.starts_with("/themes ") {
            let query = q["/themes".len()..].trim();
            let mut out = Vec::new();
            for (i, t) in crate::themes::THEMES.iter().enumerate() {
                if query.is_empty() || t.name.contains(query) {
                    out.push(Suggestion {
                        label: t.name.to_string(),
                        hint: "Theme",
                        action: PaletteAction::ApplyTheme(i),
                    });
                    if out.len() >= MAX_SUGGESTIONS {
                        break;
                    }
                }
            }
            return out;
        }
        let mut out = Vec::new();
        for c in COMMANDS {
            let name = c.name.to_lowercase();
            let bare = name.trim_start_matches('/');
            if q.is_empty()
                || q == "/"
                || name.starts_with(&q)
                || bare.starts_with(q.trim_start_matches('/'))
            {
                out.push(Suggestion {
                    label: c.name.to_string(),
                    hint: c.desc,
                    action: c.action.clone(),
                });
                if out.len() >= MAX_SUGGESTIONS {
                    break;
                }
            }
        }
        out
    }

    /// Altura total de la caja (input + sugerencias).
    pub fn height(&self) -> u16 {
        let n = self.suggestions().len().min(MAX_SUGGESTIONS) as u16;
        3 + n
    }

    pub fn draw(&self, f: &mut Frame, area: Rect, accent: Color) {
        let block = Block::default()
            .title(" Command — Ctrl+P to close ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(accent));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let rows = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(1),
                ratatui::layout::Constraint::Min(0),
            ])
            .split(inner);

        // Entrada con bloque de cursor propio (sin cursor nativo).
        let bi = self.byte_idx();
        let before = &self.input[..bi];
        let cur = self.input[bi..]
            .chars()
            .next()
            .map_or_else(|| " ".to_string(), |c| c.to_string());
        let after_b = self.input[bi..]
            .char_indices()
            .nth(1)
            .map_or(self.input.len(), |(i, _)| bi + i);
        let after = &self.input[after_b..];
        let prompt = Span::styled(
            "> ",
            Style::default()
                .fg(accent)
                .add_modifier(Modifier::BOLD),
        );
        let cursor_style = Style::default().bg(Color::White).fg(Color::Black);
        let input_line = Line::from(vec![
            prompt,
            Span::raw(before),
            Span::styled(cur, cursor_style),
            Span::raw(after),
        ]);
        f.render_widget(Paragraph::new(input_line), rows[0]);

        let sugg = self.suggestions();
        let items: Vec<ListItem> = sugg
            .iter()
            .map(|s| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:<10} ", s.label),
                        Style::default()
                            .fg(accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(s.hint, Style::default().fg(Color::DarkGray)),
                ]))
            })
            .collect();
        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("› ");
        let mut state = ListState::default();
        if !sugg.is_empty() {
            state.select(Some(self.selected.min(sugg.len() - 1)));
        }
        f.render_stateful_widget(list, rows[1], &mut state);
    }

    /// Completa la entrada con la sugerencia seleccionada. Sin efecto
    /// si no hay sugerencias.
    fn autocomplete(&mut self) {
        let list = self.suggestions();
        if list.is_empty() {
            return;
        }
        let idx = self.selected.min(list.len().saturating_sub(1));
        let Some(sel) = list.into_iter().nth(idx) else {
            return;
        };
        if sel.label == "/themes" {
            self.input = "/themes ".to_string();
        } else if matches!(sel.action, PaletteAction::ApplyTheme(_)) {
            self.input = format!("/themes {}", sel.label);
        } else {
            self.input = sel.label;
        }
        self.cursor = self.input.chars().count();
        self.selected = 0;
    }

    /// Maneja una tecla. Solo `Press` sin `Ctrl`/`Alt` edita el texto.
    pub fn handle_key(&mut self, key: KeyEvent) -> PaletteAction {
        if key.kind != KeyEventKind::Press {
            return PaletteAction::None;
        }
        match key.code {
            KeyCode::Esc => {
                self.close();
                return PaletteAction::Close;
            }
            KeyCode::Enter => {
                // `/themes` solo abre el modo lista: sin filtro no aplica nada.
                if self.input.trim().eq_ignore_ascii_case("/themes") {
                    return PaletteAction::None;
                }
                let list = self.suggestions();
                if list.is_empty() {
                    return PaletteAction::Unknown;
                }
                let action = list
                    .get(self.selected.min(list.len().saturating_sub(1)))
                    .map_or(PaletteAction::Unknown, |s| s.action.clone());
                if action == PaletteAction::ApplyTheme(usize::MAX) {
                    return PaletteAction::None;
                }
                self.close();
                return action;
            }
            KeyCode::Tab => {
                self.autocomplete();
                return PaletteAction::None;
            }
            KeyCode::Up => {
                let n = self.suggestions().len();
                if n > 0 {
                    self.selected = self.selected.saturating_sub(1);
                }
                return PaletteAction::None;
            }
            KeyCode::Down => {
                let n = self.suggestions().len();
                if n > 0 && self.selected + 1 < n {
                    self.selected += 1;
                }
                return PaletteAction::None;
            }
            KeyCode::Left => {
                self.cursor = self.cursor.saturating_sub(1);
                return PaletteAction::None;
            }
            KeyCode::Right => {
                let max = self.input.chars().count();
                if self.cursor < max {
                    self.cursor += 1;
                }
                return PaletteAction::None;
            }
            KeyCode::Backspace => {
                if self.cursor > 0 && !self.input.is_empty() {
                    let bi = self.byte_idx();
                    let prev = self.input[..bi]
                        .char_indices()
                        .next_back()
                        .map_or(0, |(i, _)| i);
                    self.input.drain(prev..bi);
                    self.cursor -= 1;
                    self.selected = 0;
                }
                return PaletteAction::None;
            }
            KeyCode::Char(c) => {
                if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
                    || key.modifiers.contains(crossterm::event::KeyModifiers::ALT)
                {
                    return PaletteAction::None;
                }
                if self.input.chars().count() >= MAX_INPUT {
                    return PaletteAction::None;
                }
                let bi = self.byte_idx();
                self.input.insert(bi, c);
                self.cursor += 1;
                self.selected = 0;
                return PaletteAction::None;
            }
            _ => {}
        }
        PaletteAction::None
    }
}

#[cfg(test)]
mod tests {
    use super::{Palette, PaletteAction, MAX_INPUT};

    #[test]
    fn test_filtra_comandos() {
        let mut p = Palette::new();
        p.open = true;
        for c in "/sa".chars() {
            p.input.push(c);
        }
        p.cursor = 3;
        let labels: Vec<String> = p.suggestions().iter().map(|s| s.label.clone()).collect();
        assert_eq!(labels, vec!["/save".to_string()]);
    }

    #[test]
    fn test_themes_lista_todo_y_filtra() {
        let mut p = Palette::new();
        p.open = true;
        p.input = "/themes".to_string();
        p.cursor = 7;
        assert_eq!(p.suggestions().len(), crate::themes::THEMES.len());
        p.input = "/themes dra".to_string();
        p.cursor = p.input.chars().count();
        let labels: Vec<String> = p.suggestions().iter().map(|s| s.label.clone()).collect();
        assert_eq!(labels, vec!["dracula".to_string()]);
    }

    #[test]
    fn test_enter_en_themes_sin_filtro_no_aplica() {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState};
        let mut p = Palette::new();
        p.open = true;
        p.input = "/themes".to_string();
        p.cursor = 7;
        let key = KeyEvent {
            code: KeyCode::Enter,
            modifiers: crossterm::event::KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        assert_eq!(p.handle_key(key), PaletteAction::None);
        assert!(p.open);
    }

    #[test]
    fn test_enter_aplica_tema_filtrado() {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState};
        let mut p = Palette::new();
        p.open = true;
        p.input = "/themes monokai".to_string();
        p.cursor = p.input.chars().count();
        let enter = KeyEvent {
            code: KeyCode::Enter,
            modifiers: crossterm::event::KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        assert_eq!(p.handle_key(enter), PaletteAction::ApplyTheme(2));
        assert!(!p.open);
    }

    #[test]
    fn test_tab_autocompleta_comando() {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState};
        let mut p = Palette::new();
        p.open = true;
        p.input = "/sa".to_string();
        p.cursor = 3;
        let tab = KeyEvent {
            code: KeyCode::Tab,
            modifiers: crossterm::event::KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        assert_eq!(p.handle_key(tab), PaletteAction::None);
        assert_eq!(p.input, "/save");
        assert!(p.open);
    }

    #[test]
    fn test_tab_autocompleta_tema() {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState};
        let mut p = Palette::new();
        p.open = true;
        p.input = "/themes".to_string();
        p.cursor = 7;
        let tab = KeyEvent {
            code: KeyCode::Tab,
            modifiers: crossterm::event::KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        assert_eq!(p.handle_key(tab), PaletteAction::None);
        assert_eq!(p.input, "/themes cobra-dark");
        // Y luego Enter lo aplica.
        let enter = KeyEvent {
            code: KeyCode::Enter,
            modifiers: crossterm::event::KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        assert_eq!(p.handle_key(enter), PaletteAction::ApplyTheme(0));
    }

    #[test]
    fn test_tab_sin_sugerencias_no_hace_nada() {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState};
        let mut p = Palette::new();
        p.open = true;
        p.input = "/zzz".to_string();
        p.cursor = 4;
        let tab = KeyEvent {
            code: KeyCode::Tab,
            modifiers: crossterm::event::KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        assert_eq!(p.handle_key(tab), PaletteAction::None);
        assert_eq!(p.input, "/zzz");
    }

    #[test]
    fn test_entrada_acotada_y_backspace() {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState};
        let mut p = Palette::new();
        p.open = true;
        let key_c = |c: char| KeyEvent {
            code: KeyCode::Char(c),
            modifiers: crossterm::event::KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        for _ in 0..MAX_INPUT + 20 {
            p.handle_key(key_c('a'));
        }
        assert_eq!(p.input.chars().count(), MAX_INPUT);
        let back = KeyEvent {
            code: KeyCode::Backspace,
            modifiers: crossterm::event::KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        p.handle_key(back);
        assert_eq!(p.input.chars().count(), MAX_INPUT - 1);
    }
}
