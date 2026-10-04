//! Temas de color de `CobraTUI`.
//!
//! Paletas estaticas `Copy`: sin allocs, sin I/O, solo `Color`s de `ratatui`.
//! El resaltado mapea sus categorias a estos colores (ver `highlight`).

use ratatui::style::Color;

/// Paleta de un tema. Todos los campos son `Color` (baratos de copiar).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    pub name: &'static str,
    pub accent: Color,
    pub border_dim: Color,
    pub text: Color,
    pub comment: Color,
    pub string: Color,
    pub number: Color,
    pub keyword: Color,
}

/// Tema por defecto: cian sobre fondo del terminal.
pub const COBRA_DARK: Theme = Theme {
    name: "cobra-dark",
    accent: Color::Cyan,
    border_dim: Color::DarkGray,
    text: Color::Gray,
    comment: Color::DarkGray,
    string: Color::Green,
    number: Color::Yellow,
    keyword: Color::Cyan,
};

/// Acento magenta estilo Dracula.
pub const DRACULA: Theme = Theme {
    name: "dracula",
    accent: Color::Magenta,
    border_dim: Color::DarkGray,
    text: Color::Gray,
    comment: Color::Gray,
    string: Color::Yellow,
    number: Color::LightMagenta,
    keyword: Color::LightMagenta,
};

/// Acento calido estilo Monokai.
pub const MONOKAI: Theme = Theme {
    name: "monokai",
    accent: Color::Yellow,
    border_dim: Color::DarkGray,
    text: Color::Gray,
    comment: Color::Gray,
    string: Color::Yellow,
    number: Color::LightMagenta,
    keyword: Color::LightRed,
};

/// Acento azul oceano.
pub const OCEAN: Theme = Theme {
    name: "ocean",
    accent: Color::Blue,
    border_dim: Color::DarkGray,
    text: Color::Gray,
    comment: Color::Gray,
    string: Color::Green,
    number: Color::Cyan,
    keyword: Color::LightBlue,
};

/// Registro de temas disponibles. El primero es el por defecto.
pub const THEMES: &[Theme] = &[COBRA_DARK, DRACULA, MONOKAI, OCEAN];

/// Indice del tema por defecto.
pub const DEFAULT_THEME: usize = 0;

/// Busca tema por nombre (insensible a mayusculas). Devuelve su indice.
pub fn by_name(name: &str) -> Option<usize> {
    THEMES
        .iter()
        .position(|t| t.name.eq_ignore_ascii_case(name.trim()))
}

/// Acceso seguro con fallback al por defecto si el indice es invalido.
pub fn get(index: usize) -> Theme {
    THEMES.get(index).copied().unwrap_or(COBRA_DARK)
}

#[cfg(test)]
mod tests {
    use super::{by_name, get, DEFAULT_THEME, THEMES};

    #[test]
    fn test_lookup_insensible_a_mayusculas() {
        assert_eq!(by_name("dracula"), Some(1));
        assert_eq!(by_name("DRACULA"), Some(1));
        assert_eq!(by_name("  monokai  "), Some(2));
        assert_eq!(by_name("no-existe"), None);
        assert_eq!(by_name(""), None);
    }

    #[test]
    fn test_get_con_fallback() {
        assert_eq!(get(DEFAULT_THEME).name, "cobra-dark");
        assert_eq!(get(999).name, "cobra-dark");
    }

    #[test]
    fn test_nombres_unicos() {
        let mut names: Vec<&str> = THEMES.iter().map(|t| t.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), THEMES.len());
        assert!(!THEMES.is_empty());
    }
}
