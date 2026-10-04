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
    pub function: Color,
    pub type_: Color,
    pub constant: Color,
    pub lifetime: Color,
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
    function: Color::LightGreen,
    type_: Color::LightCyan,
    constant: Color::LightMagenta,
    lifetime: Color::Magenta,
};

/// Acento magenta estilo Dracula.
pub const DRACULA: Theme = Theme {
    name: "dracula",
    accent: Color::Magenta,
    border_dim: Color::DarkGray,
    text: Color::Gray,
    comment: Color::Blue,
    string: Color::Yellow,
    number: Color::Magenta,
    keyword: Color::LightMagenta,
    function: Color::Green,
    type_: Color::LightCyan,
    constant: Color::LightRed,
    lifetime: Color::Cyan,
};

/// Acento calido estilo Monokai.
pub const MONOKAI: Theme = Theme {
    name: "monokai",
    accent: Color::Yellow,
    border_dim: Color::DarkGray,
    text: Color::Gray,
    comment: Color::DarkGray,
    string: Color::Yellow,
    number: Color::LightMagenta,
    keyword: Color::LightRed,
    function: Color::Green,
    type_: Color::LightCyan,
    constant: Color::Magenta,
    lifetime: Color::Cyan,
};

/// Acento azul oceano.
pub const OCEAN: Theme = Theme {
    name: "ocean",
    accent: Color::Blue,
    border_dim: Color::DarkGray,
    text: Color::Gray,
    comment: Color::DarkGray,
    string: Color::Green,
    number: Color::Cyan,
    keyword: Color::LightBlue,
    function: Color::LightGreen,
    type_: Color::White,
    constant: Color::Yellow,
    lifetime: Color::Magenta,
};

/// Registro de temas disponibles. El primero es el por defecto.
pub const THEMES: &[Theme] = &[COBRA_DARK, DRACULA, MONOKAI, OCEAN];

/// Indice del tema por defecto.
pub const DEFAULT_THEME: usize = 0;

/// Acceso seguro con fallback al por defecto si el indice es invalido.
pub fn get(index: usize) -> Theme {
    THEMES.get(index).copied().unwrap_or(COBRA_DARK)
}

/// Busca tema por nombre (insensible a mayusculas). Devuelve su indice.
pub fn by_name(name: &str) -> Option<usize> {
    THEMES
        .iter()
        .position(|t| t.name.eq_ignore_ascii_case(name.trim()))
}

/// Archivo donde se persiste el tema elegido. Nombre fijo (sin input
/// del usuario en la ruta: no hay traversal posible).
/// Windows: `%APPDATA%\CobraTUI\theme`. Resto: `$XDG_CONFIG_HOME` o
/// `~/.config`, subcarpeta `CobraTUI`.
pub fn config_file() -> Option<std::path::PathBuf> {
    #[cfg(windows)]
    let base = std::env::var_os("APPDATA").map(std::path::PathBuf::from);
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| {
                let mut p = std::path::PathBuf::from(h);
                p.push(".config");
                p
            })
        });
    base.map(|mut d| {
        d.push("CobraTUI");
        d.push("theme");
        d
    })
}

/// Lee el indice persistido desde `path` con límite de 64 bytes y lo
/// valida contra la lista cerrada de temas. Contenido inválido,
/// sobredimensionado o no UTF-8 se ignora (`None` → tema por defecto),
/// sin fallar nunca.
pub fn load_theme_from(path: &std::path::Path) -> Option<usize> {
    use std::io::Read as _;
    let mut file = std::fs::File::open(path).ok()?;
    let mut buf = Vec::new();
    file.by_ref().take(65).read_to_end(&mut buf).ok()?;
    if buf.len() > 64 {
        return None;
    }
    let text = std::str::from_utf8(&buf).ok()?;
    by_name(text)
}

/// Persiste el nombre del tema con el mismo guardado atómico del
/// editor (best-effort: los errores se ignoran, no vale romper el
/// editor por no poder guardar una preferencia).
pub fn persist_theme_to(path: &std::path::Path, name: &str) {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
    let _ = crate::OxideEngine::oxide::atomic_write(path, format!("{name}\n").as_bytes(), None);
}

/// Lee el tema persistido en la ubicación estándar, si existe y es válido.
pub fn load_theme_index() -> Option<usize> {
    config_file().and_then(|p| load_theme_from(&p))
}

/// Persiste el tema en la ubicación estándar (best-effort).
pub fn persist_theme(name: &str) {
    if let Some(path) = config_file() {
        persist_theme_to(&path, name);
    }
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_THEME, THEMES, by_name, get, load_theme_from, persist_theme_to};

    #[test]
    fn test_lookup_insensible_a_mayusculas() {
        assert_eq!(by_name("dracula"), Some(1));
        assert_eq!(by_name("DRACULA"), Some(1));
        assert_eq!(by_name("  monokai  "), Some(2));
        assert_eq!(by_name("no-existe"), None);
        assert_eq!(by_name(""), None);
    }

    #[test]
    fn test_persist_y_load_roundtrip() {
        let _guard = crate::OxideEngine::oxide::SAVE_TEST_LOCK
            .lock()
            .unwrap();
        let mut path = std::env::temp_dir();
        path.push("cobra_theme_unit_test");
        let _ = std::fs::remove_file(&path);
        assert_eq!(load_theme_from(&path), None);
        persist_theme_to(&path, "ocean");
        assert_eq!(load_theme_from(&path), Some(3));
        // Contenido inválido se ignora sin pánico.
        std::fs::write(&path, "no-existe\n").unwrap();
        assert_eq!(load_theme_from(&path), None);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_load_rechaza_sobretamano_y_binario() {
        let mut path = std::env::temp_dir();
        path.push("cobra_theme_big_unit");
        // 100 bytes > límite de 64: se ignora aunque empiece válido.
        std::fs::write(&path, "ocean".to_string() + &"x".repeat(100)).unwrap();
        assert_eq!(load_theme_from(&path), None);
        // Bytes no UTF-8: se ignoran sin pánico.
        std::fs::write(&path, [0xff, 0xfe, 0x00]).unwrap();
        assert_eq!(load_theme_from(&path), None);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_persist_no_deja_temporales() {
        // Serializado con los otros tests que guardan: comparten el
        // espacio de nombres de temporales del proceso.
        let _guard = crate::OxideEngine::oxide::SAVE_TEST_LOCK
            .lock()
            .unwrap();
        let mut path = std::env::temp_dir();
        path.push("cobra_theme_atomic_unit");
        let _ = std::fs::remove_file(&path);
        persist_theme_to(&path, "dracula");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "dracula\n");
        let prefix = format!(".tmp-cobra-{}-", std::process::id());
        let leftovers: Vec<String> = std::fs::read_dir(path.parent().unwrap())
            .map(|rd| {
                rd.filter_map(Result::ok)
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .filter(|n| n.starts_with(&prefix))
                    .collect()
            })
            .unwrap_or_default();
        assert!(leftovers.is_empty(), "restos: {leftovers:?}");
        let _ = std::fs::remove_file(&path);
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
