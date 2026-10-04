use ratatui::layout::Rect;

/// Rect centrado de `w` x `h` dentro de `area`.
pub fn centered_rect(w: u16, h: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    Rect {
        x,
        y,
        width: w.min(area.width),
        height: h.min(area.height),
    }
}

/// `true` si la ruta es remota (UNC): `\\servidor\recurso`,
/// `\\?\UNC\...` o `\\.\...` (DeviceNS). En el resto de plataformas
/// estos prefijos no existen y siempre es `false`.
pub fn is_remote(path: &std::path::Path) -> bool {
    use std::path::{Component, Prefix};
    path.components().any(|c| {
        matches!(
            c,
            Component::Prefix(p) if matches!(
                p.kind(),
                Prefix::UNC(..) | Prefix::VerbatimUNC(..) | Prefix::DeviceNS(..)
            )
        )
    })
}

/// `true` si el nombre final de `path` es un dispositivo reservado de
/// Windows (`CON`, `PRN`, `AUX`, `NUL`, `COM1`-`COM9`, `LPT1`-`LPT9`),
/// con o sin extensión (`NUL.txt` también está reservado). Insensible
/// a mayúsculas. Evita abrir dispositivos al leer archivos.
pub fn is_reserved_device_name(path: &std::path::Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let stem = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    matches!(
        stem.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

/// Sanitizado solo para mostrar: sustituye todo carácter de control
/// (incluido `\x1b` y C1) excepto `\t` por U+FFFD. Preserva la cantidad
/// de chars (1:1) para no romper mapeos byte<->char. El contenido
/// original nunca se modifica, solo lo renderizado.
pub fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c == '\t' || !c.is_control() {
                c
            } else {
                '\u{FFFD}'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{is_remote, is_reserved_device_name, sanitize};

    #[test]
    fn test_reservados_windows() {
        use std::path::Path;
        for name in [
            "CON", "con", "NUL", "nul.txt", "COM1", "com9", "LPT1", "lpt9.dat", "PRN", "AUX",
        ] {
            assert!(is_reserved_device_name(Path::new(name)), "{name}");
            assert!(is_reserved_device_name(&Path::new("dir").join(name)));
        }
        for name in ["console.txt", "null", "COM10", "notes", "a.txt", ""] {
            assert!(!is_reserved_device_name(Path::new(name)), "{name}");
        }
    }

    #[test]
    fn test_sanitize_reemplaza_controles() {
        // ESC, C1 (U+0085), BEL y DEL van a U+FFFD (escapes explicitos,
        // sin controles crudos en el fuente).
        assert_eq!(sanitize("a\x1b[2Jb"), "a�[2Jb");
        assert_eq!(sanitize("a\u{85}b"), "a�b");
        assert_eq!(sanitize("a\x07b"), "a�b");
        assert_eq!(sanitize("a\x7fb"), "a�b");
    }

    #[test]
    fn test_sanitize_sin_controles_identico() {
        assert_eq!(sanitize("hola mundo"), "hola mundo");
        assert_eq!(sanitize(""), "");
        assert_eq!(sanitize("áé"), "áé");
    }

    #[test]
    fn test_sanitize_preserva_cantidad_chars() {
        let s = "a\tb\x1bc";
        assert_eq!(s.chars().count(), sanitize(s).chars().count());
    }

    #[test]
    fn test_is_remote_local_no() {
        use std::path::Path;
        assert!(!is_remote(Path::new("relativo/archivo.txt")));
        assert!(!is_remote(Path::new(".")));
    }

    #[cfg(windows)]
    #[test]
    fn test_is_remote_unc_si() {
        use std::path::Path;
        assert!(is_remote(Path::new("\\\\servidor\\recurso")));
        assert!(is_remote(Path::new("\\\\servidor\\recurso\\dir\\a.txt")));
        assert!(is_remote(Path::new("\\\\?\\UNC\\servidor\\recurso")));
        assert!(is_remote(Path::new("\\\\.\\COM1")));
        // Disco local y verbatim de disco no son remotos.
        assert!(!is_remote(Path::new("C:\\dir\\a.txt")));
        assert!(!is_remote(Path::new("\\\\?\\C:\\dir")));
    }

    #[cfg(not(windows))]
    #[test]
    fn test_is_remote_solo_windows() {
        use std::path::Path;
        assert!(!is_remote(Path::new("/mnt/datos/a.txt")));
    }
}
