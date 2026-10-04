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
    Function,
    Type,
    Constant,
    Lifetime,
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

/// `true` si viene `!` seguido de `(`, `[` o `{`: invocacion de macro.
fn is_macro_open(it: &std::iter::Peekable<std::str::CharIndices<'_>>) -> bool {
    let mut probe = it.clone();
    match probe.next() {
        Some((_, '!')) => matches!(probe.next(), Some((_, '(' | '[' | '{'))),
        _ => false,
    }
}

/// Clasifica un identificador no-keyword: MAYUSCULAS (len>1) es
/// constante, inicial mayuscula es tipo, resto texto normal (`None`).
fn classify_ident(word: &str) -> Option<Kind> {
    let mut chars = word.chars();
    match chars.next() {
        Some(c) if c.is_uppercase() => {
            if word.len() > 1
                && word
                    .chars()
                    .all(|d| d.is_uppercase() || d == '_' || d.is_numeric())
            {
                Some(Kind::Constant)
            } else {
                Some(Kind::Type)
            }
        }
        _ => None,
    }
}

/// Consume un run (`is_part`) desde el `peek`. Devuelve el byte final.
/// Actualiza `scanned`; corta al llegar a `MAX_SCAN_CHARS`.
fn scan_run(
    it: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    scanned: &mut usize,
    mut end: usize,
    is_part: impl Fn(char) -> bool,
) -> usize {
    loop {
        match it.peek() {
            Some((b2, c2)) if is_part(*c2) => {
                end = b2 + c2.len_utf8();
                *scanned += 1;
                if *scanned >= MAX_SCAN_CHARS {
                    it.next();
                    break;
                }
                it.next();
            }
            _ => break,
        }
    }
    end
}

/// Fin del string que abre en `b`: respeta escapes, tolera sin cerrar.
fn scan_string(
    it: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    scanned: &mut usize,
    line_len: usize,
) -> usize {
    let mut end = line_len;
    let mut escaped = false;
    for (b2, c2) in it.by_ref() {
        if *scanned >= MAX_SCAN_CHARS {
            break;
        }
        *scanned += 1;
        if escaped {
            escaped = false;
        } else if c2 == '\\' {
            escaped = true;
        } else if c2 == '"' {
            end = b2 + c2.len_utf8();
            break;
        }
    }
    end
}

/// Fin del atributo que abre en `#` (ya confirmado `[`): hasta `]`
/// en la misma linea, o fin de linea si no cierra.
fn scan_attribute(
    it: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    scanned: &mut usize,
    line_len: usize,
) -> usize {
    let mut end = line_len;
    for (b2, c2) in it.by_ref() {
        if *scanned >= MAX_SCAN_CHARS {
            break;
        }
        *scanned += 1;
        if c2 == ']' {
            end = b2 + 1;
            break;
        }
    }
    end
}

/// Fin del lifetime que abre en `'` (ya confirmado identificador
/// a continuacion), o `None`. Incluye labels (`'loop:`).
fn scan_lifetime(
    it: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    scanned: &mut usize,
) -> Option<usize> {
    let sb = match it.peek() {
        Some((bb, c)) if is_ident_start(*c) => *bb,
        _ => return None,
    };
    Some(scan_run(it, scanned, sb, is_ident_char))
}

/// Consume el identificador que abre en `b` y devuelve su token si
/// tiene color (`None` = texto normal). Orden: keyword, llamada o
/// macro (funcion), tipo o constante.
fn scan_ident(
    line: &str,
    it: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    scanned: &mut usize,
    b: usize,
    first: char,
) -> Option<Token> {
    let end = scan_run(it, scanned, b + first.len_utf8(), is_ident_char);
    let word = &line[b..end];
    if is_keyword(word) {
        Some(Token {
            start: b,
            end,
            kind: Kind::Keyword,
        })
    } else if matches!(it.peek(), Some((_, '('))) || is_macro_open(it) {
        Some(Token {
            start: b,
            end,
            kind: Kind::Function,
        })
    } else {
        classify_ident(word).map(|kind| Token { start: b, end, kind })
    }
}

/// Fin del char literal que abre en `b`, o `None` (ej. lifetime `'a`).
fn scan_char_end(
    it: &std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Option<usize> {
    let mut probe = it.clone();
    match probe.next() {
        Some((_, '\\')) => {
            for _ in 0..6 {
                match probe.next() {
                    Some((be, '\'')) => return Some(be + 1),
                    Some(_) => {}
                    None => return None,
                }
            }
            None
        }
        Some(_) => match probe.next() {
            Some((be, '\'')) => Some(be + 1),
            _ => None,
        },
        None => None,
    }
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
                let end = scan_string(&mut it, &mut scanned, line.len());
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
                if let Some(end) = scan_char_end(&it) {
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
                } else if let Some(end) = scan_lifetime(&mut it, &mut scanned) {
                    out.push(Token {
                        start: b,
                        end,
                        kind: Kind::Lifetime,
                    });
                    prev_ident = true;
                } else {
                    prev_ident = false;
                }
            }
            '#' => {
                // Atributo `#[...]`: usa color de keyword (ver THEMES.md).
                if matches!(it.peek(), Some((_, '['))) {
                    let end = scan_attribute(&mut it, &mut scanned, line.len());
                    out.push(Token {
                        start: b,
                        end,
                        kind: Kind::Keyword,
                    });
                }
                prev_ident = false;
            }
            c if c.is_ascii_digit() && !prev_ident => {
                let end = scan_run(&mut it, &mut scanned, b + c.len_utf8(), |d| {
                    d.is_ascii_alphanumeric() || d == '_' || d == '.'
                });
                out.push(Token {
                    start: b,
                    end,
                    kind: Kind::Number,
                });
                prev_ident = true;
            }
            c if is_ident_start(c) => {
                if let Some(tok) = scan_ident(line, &mut it, &mut scanned, b, c) {
                    out.push(tok);
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
        // `main(` es llamada: Function, `x` queda sin pintar
        assert!(k.contains(&("main".to_string(), Kind::Function)));
        assert!(!k.iter().any(|(w, _)| w == "x"));
        assert!(!is_keyword("main"));
        assert!(is_keyword("return"));
        // Keyword antes de `(` sigue siendo keyword
        let k2 = kinds("if (x) {}");
        assert!(k2.contains(&("if".to_string(), Kind::Keyword)));
    }

    #[test]
    fn test_funciones_y_macros() {
        let k = kinds("foo(bar); obj.method(x);");
        assert!(k.contains(&("foo".to_string(), Kind::Function)));
        assert!(k.contains(&("method".to_string(), Kind::Function)));
        let m = kinds("let v = vec![1, 2];");
        assert!(m.contains(&("vec".to_string(), Kind::Function)));
    }

    #[test]
    fn test_tipos_y_constantes() {
        let k = kinds("let x: String = MAX_SIZE;");
        assert!(k.contains(&("String".to_string(), Kind::Type)));
        assert!(k.contains(&("MAX_SIZE".to_string(), Kind::Constant)));
        let k2 = kinds("struct Foo;");
        assert!(k2.contains(&("Foo".to_string(), Kind::Type)));
    }

    #[test]
    fn test_lifetimes() {
        let k = kinds("fn f(x: &'a str) {}");
        assert!(k.iter().any(|(w, kind)| *kind == Kind::Lifetime && w == "'a"));
    }

    #[test]
    fn test_atributo() {
        let k = kinds("#[derive(Debug)]");
        assert!(k.iter().any(|(_, kind)| *kind == Kind::Keyword));
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
