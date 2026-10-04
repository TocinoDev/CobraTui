//! Resaltado liviano linea por linea para `CobraTUI`.
//!
//! Escaner lineal O(n) sin regex y sin I/O: `//` comentarios, `"…"`
//! strings con escapes, `'x'` chars, numeros y keywords. Solo se
//! escanean las lineas visibles y como maximo `MAX_SCAN_CHARS`
//! caracteres por linea. Todos los rangos son fronteras UTF-8 validas.

/// Categoria de un tramo resaltado.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Keyword,
    String,
    Number,
    Comment,
}

/// Tramo resaltado con indices de byte (`start..end`, `end` exclusivo).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    pub start: usize,
    pub end: usize,
    pub kind: Kind,
}

/// Maximo de caracteres escaneados por linea (rendimiento acotado).
pub const MAX_SCAN_CHARS: usize = 512;

/// Keywords ordenadas para `binary_search`.
static KEYWORDS: &[&str] = &[
    "Self", "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else",
    "enum", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move",
    "mut", "pub", "ref", "return", "self", "static", "struct", "trait", "true", "type", "use",
    "where", "while",
];

/// `true` si `word` es keyword (busqueda binaria, sin allocs).
pub fn is_keyword(word: &str) -> bool {
    KEYWORDS.binary_search(&word).is_ok()
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Resalta una linea. Nunca paniquea: todos los cortes son
/// fronteras de char y el trabajo esta acotado por `MAX_SCAN_CHARS`.
pub fn highlight_line(line: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut it = line.char_indices().peekable();
    let mut scanned: usize = 0;
    let mut prev_ident = false;

    while let Some((b, c)) = it.next() {
        if scanned >= MAX_SCAN_CHARS {
            break;
        }
        scanned += 1;
        match c {
            '/' => {
                if matches!(it.peek(), Some((_, '/'))) {
                    out.push(Token {
                        start: b,
                        end: line.len(),
                        kind: Kind::Comment,
                    });
                    break;
                }
                prev_ident = false;
            }
            '"' => {
                let mut end = line.len();
                let mut escaped = false;
                for (b2, c2) in it.by_ref() {
                    if scanned >= MAX_SCAN_CHARS {
                        break;
                    }
                    scanned += 1;
                    if escaped {
                        escaped = false;
                    } else if c2 == '\\' {
                        escaped = true;
                    } else if c2 == '"' {
                        end = b2 + c2.len_utf8();
                        break;
                    }
                }
                out.push(Token {
                    start: b,
                    end,
                    kind: Kind::String,
                });
                prev_ident = false;
            }
            '\'' => {
                // Solo char literal corto ('x', '\n'): lifetimes como
                // `'a` no cierran comilla y quedan como texto normal.
                let mut probe = it.clone();
                let mut found: Option<usize> = None;
                match probe.next() {
                    Some((_, '\\')) => {
                        for _ in 0..6 {
                            match probe.next() {
                                Some((be, '\'')) => {
                                    found = Some(be + 1);
                                    break;
                                }
                                Some(_) => {}
                                None => break,
                            }
                        }
                    }
                    Some(_) => {
                        if let Some((be, '\'')) = probe.next() {
                            found = Some(be + 1);
                        }
                    }
                    None => {}
                }
                if let Some(end) = found {
                    for (b2, c2) in it.by_ref() {
                        scanned += 1;
                        if b2 + c2.len_utf8() >= end {
                            break;
                        }
                    }
                    out.push(Token {
                        start: b,
                        end,
                        kind: Kind::String,
                    });
                    prev_ident = false;
                } else {
                    prev_ident = false;
                }
            }
            c if c.is_ascii_digit() && !prev_ident => {
                let mut end = b + c.len_utf8();
                loop {
                    match it.peek() {
                        Some((b2, c2))
                            if c2.is_ascii_alphanumeric() || *c2 == '_' || *c2 == '.' =>
                        {
                            end = b2 + c2.len_utf8();
                            scanned += 1;
                            if scanned >= MAX_SCAN_CHARS {
                                it.next();
                                break;
                            }
                            it.next();
                        }
                        _ => break,
                    }
                }
                out.push(Token {
                    start: b,
                    end,
                    kind: Kind::Number,
                });
                prev_ident = true;
            }
            c if is_ident_start(c) => {
                let mut end = b + c.len_utf8();
                loop {
                    match it.peek() {
                        Some((b2, c2)) if is_ident_char(*c2) => {
                            end = b2 + c2.len_utf8();
                            scanned += 1;
                            if scanned >= MAX_SCAN_CHARS {
                                it.next();
                                break;
                            }
                            it.next();
                        }
                        _ => break,
                    }
                }
                if is_keyword(&line[b..end]) {
                    out.push(Token {
                        start: b,
                        end,
                        kind: Kind::Keyword,
                    });
                }
                prev_ident = true;
            }
            _ => {
                prev_ident = is_ident_char(c);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{highlight_line, is_keyword, Kind, MAX_SCAN_CHARS};

    fn kinds(line: &str) -> Vec<(String, Kind)> {
        highlight_line(line)
            .into_iter()
            .map(|t| (line[t.start..t.end].to_string(), t.kind))
            .collect()
    }

    fn assert_boundaries(line: &str) {
        for t in highlight_line(line) {
            assert!(line.is_char_boundary(t.start), "start {}", t.start);
            assert!(line.is_char_boundary(t.end), "end {}", t.end);
            assert!(t.start < t.end);
        }
    }

    #[test]
    fn test_keywords() {
        let k = kinds("fn main() { let mut x = 1; }");
        assert!(k.contains(&("fn".to_string(), Kind::Keyword)));
        assert!(k.contains(&("let".to_string(), Kind::Keyword)));
        assert!(k.contains(&("mut".to_string(), Kind::Keyword)));
        assert!(!k.iter().any(|(w, _)| w == "main" || w == "x"));
        assert!(!is_keyword("main"));
        assert!(is_keyword("return"));
    }

    #[test]
    fn test_string_con_escape() {
        let k = kinds(r#"let s = "a\"b";"#);
        assert_eq!(k.len(), 2); // let + string
        assert!(k.iter().any(|(w, kind)| *kind == Kind::String && w.starts_with('"')));
    }

    #[test]
    fn test_string_sin_cerrar() {
        let k = kinds(r#"let s = "abc"#);
        assert!(k.iter().any(|(_, kind)| *kind == Kind::String));
    }

    #[test]
    fn test_comentario() {
        let k = kinds("// todo: fix");
        assert_eq!(k.len(), 1);
        assert_eq!(k[0].1, Kind::Comment);
        let k2 = kinds("let x = 1; // fin");
        assert!(k2.iter().any(|(_, kind)| *kind == Kind::Comment));
    }

    #[test]
    fn test_numero_y_sufijo() {
        let k = kinds("let x = 42u32;");
        assert!(k.iter().any(|(w, kind)| *kind == Kind::Number && w == "42u32"));
        // Identificador con digitos no es numero
        let k2 = kinds("abc123");
        assert!(!k2.iter().any(|(_, kind)| *kind == Kind::Number));
    }

    #[test]
    fn test_char_vs_lifetime() {
        let k = kinds("let c = 'a';");
        assert!(k.iter().any(|(_, kind)| *kind == Kind::String));
        let k2 = kinds("fn f<'a>(x: &'a str) {}");
        assert!(!k2.iter().any(|(_, kind)| *kind == Kind::String));
    }

    #[test]
    fn test_unicode_sin_panico() {
        let line = "let emoji = \"😀\"; // hola 🌍";
        assert_boundaries(line);
        let k = kinds(line);
        assert!(k.iter().any(|(_, kind)| *kind == Kind::String));
        assert!(k.iter().any(|(_, kind)| *kind == Kind::Comment));
        assert_boundaries("");
        assert_boundaries("   ");
        assert!(highlight_line("").is_empty());
    }

    #[test]
    fn test_trabajo_acotado() {
        let long: String = "a".repeat(MAX_SCAN_CHARS + 500);
        let tokens = highlight_line(&long);
        // Un ident larguisimo como mucho genera 0-1 tokens: nunca explota
        assert!(tokens.len() <= 2);
    }
}
