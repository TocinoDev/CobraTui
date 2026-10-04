use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

/// Menú principal como pantalla de bienvenida a pantalla completa,
/// no popup. Se ve como el editor pero con mensaje centrado.
pub struct Menu;

/// Los metodos toman `self` por consistencia con `Editor`/`Picker`
/// aunque `Menu` aun no guarda estado.
#[allow(clippy::unused_self)]
impl Menu {
    pub fn new() -> Self {
        Self
    }

    /// Menu principal a pantalla completa, sin picker.
    pub fn draw(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" CobraTUI v0.1.0 ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let title = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let key_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let desc_style = Style::default().fg(Color::Gray);
        let dim = Style::default().fg(Color::DarkGray);

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled("CobraTUI", title)),
            Line::from(Span::styled("Fast terminal text editor", dim)),
            Line::from(""),
            Line::from(vec![
                Span::styled("Ctrl+K  ", key_style),
                Span::styled("Abrir carpeta", desc_style),
            ]),
            Line::from(vec![
                Span::styled("Ctrl+O  ", key_style),
                Span::styled("Abrir archivo", desc_style),
            ]),
            Line::from(vec![
                Span::styled("Ctrl+N  ", key_style),
                Span::styled("Nuevo archivo", desc_style),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Enter   ", key_style),
                Span::styled("Explorar workspace", desc_style),
            ]),
            Line::from(vec![
                Span::styled("Esc     ", key_style),
                Span::styled("Salir", desc_style),
            ]),
        ];

        let p = Paragraph::new(lines)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White));
        f.render_widget(p, inner);
    }

    /// Panel derecho cuando hay proyecto abierto pero ningun archivo seleccionado.
    /// Mismas guias adaptadas al explorer.
    pub fn draw_panel(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Bienvenido ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::White));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let title = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let key_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let desc_style = Style::default().fg(Color::Gray);
        let dim = Style::default().fg(Color::DarkGray);

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled("CobraTUI", title)),
            Line::from(Span::styled("Selecciona un archivo para empezar", dim)),
            Line::from(""),
            Line::from(vec![
                Span::styled("↑↓      ", key_style),
                Span::styled("Navegar explorer", desc_style),
            ]),
            Line::from(vec![
                Span::styled("Enter   ", key_style),
                Span::styled("Abrir archivo o carpeta", desc_style),
            ]),
            Line::from(vec![
                Span::styled("Backsp  ", key_style),
                Span::styled("Subir carpeta", desc_style),
            ]),
            Line::from(vec![
                Span::styled("Tab     ", key_style),
                Span::styled("Cambiar foco", desc_style),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Ctrl+K  ", key_style),
                Span::styled("Abrir otra carpeta", desc_style),
            ]),
            Line::from(vec![
                Span::styled("Ctrl+O  ", key_style),
                Span::styled("Abrir archivo", desc_style),
            ]),
            Line::from(vec![
                Span::styled("Ctrl+N  ", key_style),
                Span::styled("Nuevo archivo", desc_style),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Esc     ", key_style),
                Span::styled("Volver al menu", desc_style),
            ]),
        ];

        let p = Paragraph::new(lines)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White));
        f.render_widget(p, inner);
    }

    /// Enter va al editor, Esc sale.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<bool> {
        if key.kind != KeyEventKind::Press {
            return None;
        }
        match key.code {
            KeyCode::Enter => Some(true),
            KeyCode::Esc => Some(false),
            _ => None,
        }
    }
}
