use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

/// Menú principal como pantalla de bienvenida a pantalla completa,
/// no popup. Se ve como el editor pero con mensaje centrado.
pub struct Menu;

/// Logo COBRATUI en bloques (7 filas). Fuente 5x7: mas alta que ancha
/// porque las celdas del terminal son ~2:1. Todas las filas se rellenan
/// al mismo ancho para que el centrado no desplace el medio (filas de
/// 45 vs 47 se veian corridas a la derecha).
fn logo(accent: Style) -> Vec<Line<'static>> {
    const ROWS: [&str; 7] = [
        " ████  ███  ████  ████    █   █████ █   █ █████",
        "█     █   █ █   █ █   █  █ █    █   █   █   █",
        "█     █   █ █   █ █   █ █   █   █   █   █   █",
        "█     █   █ ████  ████  █████   █   █   █   █",
        "█     █   █ █   █ █ █   █   █   █   █   █   █",
        "█     █   █ █   █ █  █  █   █   █   █   █   █",
        " ████  ███  ████  █   █ █   █   █    ███  █████",
    ];
    let width = ROWS.iter().map(|r| r.chars().count()).max().unwrap_or(0);
    ROWS.iter()
        .map(|r| {
            let mut s = String::from(*r);
            while s.chars().count() < width {
                s.push(' ');
            }
            Line::from(Span::styled(s, accent))
        })
        .collect()
}

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
            .title(" CobraTUI v0.1.1 ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let logo_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let key_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let desc_style = Style::default().fg(Color::Gray);
        let dim = Style::default().fg(Color::DarkGray);

        let mut lines = vec![Line::from("")];
        lines.extend(logo(logo_style));
        lines.extend(vec![
            Line::from(""),
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
        ]);

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
            Line::from(vec![
                Span::styled("Ctrl+P  ", key_style),
                Span::styled("Comandos (/themes…)", desc_style),
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

#[cfg(test)]
mod tests {
    use super::logo;
    use ratatui::style::{Color, Modifier, Style};

    #[test]
    fn test_logo_bloques_proporcionado() {
        let style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let lines = logo(style);
        assert_eq!(lines.len(), 7);
        // Todas las filas con el mismo ancho: si no, el centrado
        // desplaza las mas angostas (bug visto abajo a la derecha).
        let widths: Vec<usize> = lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.chars().count()).sum())
            .collect();
        assert!((30..=48).contains(&widths[0]), "ancho logo: {}", widths[0]);
        assert!(
            widths.iter().all(|w| *w == widths[0]),
            "filas desparejas: {widths:?}"
        );
        for line in &lines {
            for s in &line.spans {
                assert!(
                    s.content.chars().all(|c| c == ' ' || c == '█'),
                    "solo bloques y espacios: {}",
                    s.content
                );
            }
        }
    }
}
