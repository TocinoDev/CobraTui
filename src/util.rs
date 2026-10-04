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
    use super::sanitize;

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
}