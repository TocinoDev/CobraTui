//! Resaltado liviano linea por linea para `CobraTUI`, multilenguaje.
//!
//! Escaner lineal O(n) sin regex y sin I/O: comentarios, strings con
//! escapes, chars, numeros y keywords. El idioma sale de la extension
//! del archivo (`detect`); cada idioma aporta su tablita (`LangSpec`).
//! Solo se escanean las lineas visibles y como maximo `MAX_SCAN_CHARS`
//! caracteres por linea. Todos los rangos son fronteras UTF-8 validas.
//!
//! Limitacion conocida: sin estado entre lineas. Un `/*` (o `"""`) que
//! no cierre en la misma linea pinta hasta fin de linea y la siguiente
//! linea se escanea como si nada (puede pintar de mas, nunca paniquea).

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

/// Keywords de Rust, ordenadas para `binary_search`.
static KEYWORDS: &[&str] = &[
    "Self", "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "static", "struct", "trait", "true", "type", "use", "where", "while",
];

/// `true` si `word` es keyword de Rust (busqueda binaria, sin allocs).
/// Solo se usa en tests; el render productivo usa `LangSpec`.
#[cfg(test)]
pub fn is_keyword(word: &str) -> bool {
    is_word_in(KEYWORDS, word)
}

fn is_word_in(list: &[&str], word: &str) -> bool {
    list.binary_search(&word).is_ok()
}

/// Keywords de Go, ordenadas para `binary_search`.
static GO_KEYWORDS: &[&str] = &[
    "any",
    "break",
    "case",
    "chan",
    "comparable",
    "const",
    "continue",
    "default",
    "defer",
    "else",
    "fallthrough",
    "for",
    "func",
    "go",
    "goto",
    "if",
    "import",
    "interface",
    "map",
    "package",
    "range",
    "return",
    "select",
    "struct",
    "switch",
    "type",
    "var",
];

/// Constantes predefinidas de Go (`true`/`false` no son keywords).
static GO_CONSTANTS: &[&str] = &["false", "iota", "nil", "true"];

/// Keywords de Python, ordenadas para `binary_search`.
static PY_KEYWORDS: &[&str] = &[
    "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "elif",
    "else", "except", "finally", "for", "from", "global", "if", "import", "in", "is", "lambda",
    "nonlocal", "not", "or", "pass", "raise", "return", "try", "while", "with", "yield",
];

/// `True`/`False`/`None` se pintan como constantes, no como tipos.
static PY_CONSTANTS: &[&str] = &["False", "None", "True"];

/// Keywords de JavaScript/TypeScript, ordenadas para `binary_search`.
static JS_KEYWORDS: &[&str] = &[
    "async",
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "finally",
    "for",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "let",
    "new",
    "return",
    "static",
    "super",
    "switch",
    "this",
    "throw",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
    "yield",
];

/// Constantes de JS/TS (algunas capitalizadas: no las cubre la regla de tipos).
static JS_CONSTANTS: &[&str] = &["Infinity", "NaN", "false", "null", "true", "undefined"];

/// Keywords de C, ordenadas para `binary_search`.
static C_KEYWORDS: &[&str] = &[
    "auto", "break", "case", "char", "const", "continue", "default", "do", "double", "else",
    "enum", "extern", "float", "for", "goto", "if", "inline", "int", "long", "register",
    "restrict", "return", "short", "signed", "sizeof", "static", "struct", "switch", "typedef",
    "union", "unsigned", "void", "volatile", "while",
];

static C_CONSTANTS: &[&str] = &["false", "true"];

/// Tablita de sintaxis por idioma. Las listas van ordenadas para
/// `binary_search` (hay un test que lo exige al agregar idiomas).
pub struct LangSpec {
    pub keywords: &'static [&'static str],
    pub constants: &'static [&'static str],
    pub slash_comment: bool,
    pub block_comment: bool,
    pub strings: &'static [char],
    pub triple_strings: bool,
    pub raw_tick: bool,
    pub char_literal: bool,
    pub lifetime: bool,
    pub hash_attr: bool,
    pub hash_line: bool,
    pub hash_comment: bool,
    pub decorator: bool,
    pub bang_macro: bool,
}

pub const RUST: LangSpec = LangSpec {
    keywords: KEYWORDS,
    constants: &[],
    slash_comment: true,
    block_comment: false,
    strings: &['"'],
    triple_strings: false,
    raw_tick: false,
    char_literal: true,
    lifetime: true,
    hash_attr: true,
    hash_line: false,
    hash_comment: false,
    decorator: false,
    bang_macro: true,
};

pub const GO: LangSpec = LangSpec {
    keywords: GO_KEYWORDS,
    constants: GO_CONSTANTS,
    slash_comment: true,
    block_comment: true,
    strings: &['"', '`'],
    triple_strings: false,
    raw_tick: true,
    char_literal: true,
    lifetime: false,
    hash_attr: false,
    hash_line: false,
    hash_comment: false,
    decorator: false,
    bang_macro: false,
};

pub const PYTHON: LangSpec = LangSpec {
    keywords: PY_KEYWORDS,
    constants: PY_CONSTANTS,
    slash_comment: false,
    block_comment: false,
    strings: &['"', '\''],
    triple_strings: true,
    raw_tick: false,
    char_literal: false,
    lifetime: false,
    hash_attr: false,
    hash_line: false,
    hash_comment: true,
    decorator: true,
    bang_macro: false,
};

pub const JAVASCRIPT: LangSpec = LangSpec {
    keywords: JS_KEYWORDS,
    constants: JS_CONSTANTS,
    slash_comment: true,
    block_comment: true,
    strings: &['"', '\'', '`'],
    triple_strings: false,
    raw_tick: false,
    char_literal: false,
    lifetime: false,
    hash_attr: false,
    hash_line: false,
    hash_comment: false,
    decorator: false,
    bang_macro: false,
};

pub const C: LangSpec = LangSpec {
    keywords: C_KEYWORDS,
    constants: C_CONSTANTS,
    slash_comment: true,
    block_comment: true,
    strings: &['"'],
    triple_strings: false,
    raw_tick: false,
    char_literal: true,
    lifetime: false,
    hash_attr: false,
    hash_line: true,
    hash_comment: false,
    decorator: false,
    bang_macro: false,
};

/// Idiomas con resaltado. Lo desconocido cae a Rust (comportamiento
/// historico: sin regresion para archivos sin extension).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Rust,
    Go,
    Python,
    JavaScript,
    C,
}

/// Idioma segun la extension del archivo (insensible a mayusculas).
pub fn detect(path: &std::path::Path) -> Lang {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "go" => Lang::Go,
        "py" | "pyw" => Lang::Python,
        "js" | "mjs" | "cjs" | "jsx" | "ts" | "mts" | "cts" | "tsx" => Lang::JavaScript,
        "c" | "h" => Lang::C,
        _ => Lang::Rust,
    }
}

/// Tablita del idioma.
pub fn spec(lang: Lang) -> &'static LangSpec {
    match lang {
        Lang::Rust => &RUST,
        Lang::Go => &GO,
        Lang::Python => &PYTHON,
        Lang::JavaScript => &JAVASCRIPT,
        Lang::C => &C,
    }
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

/// Avanza el iterador hasta el byte `end` (exclusivo), contando escaneo.
fn skip_to(
    it: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    scanned: &mut usize,
    end: usize,
) {
    while let Some((b2, _)) = it.peek() {
        if *b2 >= end || *scanned >= MAX_SCAN_CHARS {
            break;
        }
        *scanned += 1;
        it.next();
    }
}

/// Fin del string que abre con `close`: respeta escapes, tolera sin cerrar.
fn scan_string(
    it: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    scanned: &mut usize,
    line_len: usize,
    close: char,
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
        } else if c2 == close {
            end = b2 + c2.len_utf8();
            break;
        }
    }
    end
}

/// Fin del string crudo con backticks (Go): sin escapes, hasta el
/// siguiente backtick o fin de linea.
fn scan_raw_tick(
    it: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    scanned: &mut usize,
    line: &str,
    from: usize,
) -> usize {
    let end = line[from + 1..]
        .find('`')
        .map_or(line.len(), |i| from + 1 + i + 1);
    skip_to(it, scanned, end);
    end
}

/// Fin del string triple de Python (`"""` o `'''`): hasta el cierre
/// igual en la misma linea o fin de linea.
fn scan_triple(
    it: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    scanned: &mut usize,
    line: &str,
    from: usize,
) -> usize {
    let delim = &line[from..from + 3];
    let end = line[from + 3..]
        .find(delim)
        .map_or(line.len(), |i| from + 3 + i + 3);
    skip_to(it, scanned, end);
    end
}

/// Fin del comentario de bloque (`/* ... */`) en la misma linea,
/// o fin de linea si no cierra.
fn scan_block(
    it: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    scanned: &mut usize,
    line: &str,
    from: usize,
) -> usize {
    let end = line[from + 2..]
        .find("*/")
        .map_or(line.len(), |i| from + 2 + i + 2);
    skip_to(it, scanned, end);
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
/// tiene color (`None` = texto normal). Orden: keyword, constante,
/// llamada o macro (funcion), tipo o constante por forma.
fn scan_ident(
    line: &str,
    it: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    scanned: &mut usize,
    b: usize,
    first: char,
    spec: &LangSpec,
) -> Option<Token> {
    let end = scan_run(it, scanned, b + first.len_utf8(), is_ident_char);
    let word = &line[b..end];
    if is_word_in(spec.keywords, word) {
        Some(Token {
            start: b,
            end,
            kind: Kind::Keyword,
        })
    } else if is_word_in(spec.constants, word) {
        Some(Token {
            start: b,
            end,
            kind: Kind::Constant,
        })
    } else if matches!(it.peek(), Some((_, '('))) || (spec.bang_macro && is_macro_open(it)) {
        Some(Token {
            start: b,
            end,
            kind: Kind::Function,
        })
    } else {
        classify_ident(word).map(|kind| Token {
            start: b,
            end,
            kind,
        })
    }
}

/// Fin del char literal que abre en `b`, o `None` (ej. lifetime `'a`).
fn scan_char_end(it: &std::iter::Peekable<std::str::CharIndices<'_>>) -> Option<usize> {
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

/// Resalta una linea con la tablita de Rust.
/// Solo se usa en tests; el render productivo usa `highlight_line_lang`.
#[cfg(test)]
pub fn highlight_line(line: &str) -> Vec<Token> {
    highlight_line_lang(line, Lang::Rust)
}

/// Resalta una linea con la tablita de `lang`. Nunca paniquea: todos
/// los cortes son fronteras de char y el trabajo esta acotado por
/// `MAX_SCAN_CHARS`.
pub fn highlight_line_lang(line: &str, lang: Lang) -> Vec<Token> {
    let spec = spec(lang);
    let mut out = Vec::new();
    let mut it = line.char_indices().peekable();
    let mut scanned: usize = 0;
    let mut prev_ident = false;

    while let Some((b, c)) = it.next() {
        if scanned >= MAX_SCAN_CHARS {
            break;
        }
        scanned += 1;
        // Comentario de bloque en la misma linea (`/* ... */`).
        if spec.block_comment && line[b..].starts_with("/*") {
            let end = scan_block(&mut it, &mut scanned, line, b);
            out.push(Token {
                start: b,
                end,
                kind: Kind::Comment,
            });
            prev_ident = false;
            continue;
        }
        match c {
            '/' if spec.slash_comment => {
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
            c if spec.strings.contains(&c) => {
                let end = if spec.triple_strings
                    && (line[b..].starts_with("\"\"\"") || line[b..].starts_with("'''"))
                {
                    scan_triple(&mut it, &mut scanned, line, b)
                } else if spec.raw_tick && c == '`' {
                    scan_raw_tick(&mut it, &mut scanned, line, b)
                } else {
                    scan_string(&mut it, &mut scanned, line.len(), c)
                };
                out.push(Token {
                    start: b,
                    end,
                    kind: Kind::String,
                });
                prev_ident = false;
            }
            '\'' if spec.char_literal || spec.lifetime => {
                // Solo char literal corto ('x', '\n'): lifetimes como
                // `'a` no cierran comilla y quedan como texto normal.
                if spec.char_literal
                    && let Some(end) = scan_char_end(&it)
                {
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
                } else if spec.lifetime
                    && let Some(end) = scan_lifetime(&mut it, &mut scanned)
                {
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
            '#' if spec.hash_attr || spec.hash_line || spec.hash_comment => {
                if spec.hash_attr && matches!(it.peek(), Some((_, '['))) {
                    let end = scan_attribute(&mut it, &mut scanned, line.len());
                    out.push(Token {
                        start: b,
                        end,
                        kind: Kind::Keyword,
                    });
                } else if spec.hash_line {
                    // Preprocesador de C (`#include ...`): toda la linea.
                    out.push(Token {
                        start: b,
                        end: line.len(),
                        kind: Kind::Keyword,
                    });
                    break;
                } else if spec.hash_comment {
                    out.push(Token {
                        start: b,
                        end: line.len(),
                        kind: Kind::Comment,
                    });
                    break;
                }
                prev_ident = false;
            }
            '@' if spec.decorator => {
                // Decorador de Python (`@nombre`): funcion si hay
                // identificador; `@` solo queda como texto normal.
                let end = scan_run(&mut it, &mut scanned, b + c.len_utf8(), is_ident_char);
                if end > b + 1 {
                    out.push(Token {
                        start: b,
                        end,
                        kind: Kind::Function,
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
                if let Some(tok) = scan_ident(line, &mut it, &mut scanned, b, c, spec) {
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
    use super::{
        C, GO, JAVASCRIPT, Lang, PYTHON, RUST, detect, highlight_line, highlight_line_lang,
        is_keyword,
    };
    use super::{Kind, MAX_SCAN_CHARS};

    fn kinds(line: &str) -> Vec<(String, Kind)> {
        kinds_lang(line, Lang::Rust)
    }

    fn kinds_lang(line: &str, lang: Lang) -> Vec<(String, Kind)> {
        highlight_line_lang(line, lang)
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

    fn assert_sorted(list: &[&str], what: &str) {
        assert!(
            list.windows(2).all(|w| w[0] < w[1]),
            "lista desordenada en {what}"
        );
    }

    #[test]
    fn test_tablas_ordenadas_para_binary_search() {
        for (spec, name) in [
            (&RUST, "rust"),
            (&GO, "go"),
            (&PYTHON, "python"),
            (&JAVASCRIPT, "js"),
            (&C, "c"),
        ] {
            assert_sorted(spec.keywords, name);
            assert_sorted(spec.constants, name);
        }
    }

    #[test]
    fn test_detect_por_extension() {
        use std::path::Path;
        assert_eq!(detect(Path::new("main.go")), Lang::Go);
        assert_eq!(detect(Path::new("x.PY")), Lang::Python);
        assert_eq!(detect(Path::new("a.pyw")), Lang::Python);
        assert_eq!(detect(Path::new("a.ts")), Lang::JavaScript);
        assert_eq!(detect(Path::new("a.jsx")), Lang::JavaScript);
        assert_eq!(detect(Path::new("a.c")), Lang::C);
        assert_eq!(detect(Path::new("a.h")), Lang::C);
        assert_eq!(detect(Path::new("a.rs")), Lang::Rust);
        assert_eq!(detect(Path::new("Makefile")), Lang::Rust);
        assert_eq!(detect(Path::new("sin-extension")), Lang::Rust);
    }

    #[test]
    fn test_go_basico() {
        let k = kinds_lang("package main", Lang::Go);
        assert!(k.contains(&("package".to_string(), Kind::Keyword)));
        let k2 = kinds_lang("func main() {", Lang::Go);
        assert!(k2.contains(&("func".to_string(), Kind::Keyword)));
        assert!(k2.contains(&("main".to_string(), Kind::Function)));
        let c = kinds_lang("// hola", Lang::Go);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].1, Kind::Comment);
        // Backtick crudo y runa con comilla simple.
        let r = kinds_lang("s := `a\\nb`;", Lang::Go);
        assert!(r.iter().any(|(_, kind)| *kind == Kind::String));
        let ch = kinds_lang("x := 'r'", Lang::Go);
        assert!(ch.iter().any(|(_, kind)| *kind == Kind::String));
        // Constantes predefinidas, no keywords.
        let n = kinds_lang("var x = nil", Lang::Go);
        assert!(n.contains(&("nil".to_string(), Kind::Constant)));
    }

    #[test]
    fn test_python_basico() {
        let k = kinds_lang("def foo():", Lang::Python);
        assert!(k.contains(&("def".to_string(), Kind::Keyword)));
        assert!(k.contains(&("foo".to_string(), Kind::Function)));
        let c = kinds_lang("# hola", Lang::Python);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].1, Kind::Comment);
        // Comilla simple es string, no char.
        let s = kinds_lang("x = 'a'", Lang::Python);
        assert!(s.iter().any(|(_, kind)| *kind == Kind::String));
        let t = kinds_lang("d = \"\"\"doc\"\"\"", Lang::Python);
        assert!(t.iter().any(|(_, kind)| *kind == Kind::String));
        // Decorador y constantes capitalizadas.
        let d = kinds_lang("@property", Lang::Python);
        assert!(d.contains(&("@property".to_string(), Kind::Function)));
        let n = kinds_lang("x = None", Lang::Python);
        assert!(n.contains(&("None".to_string(), Kind::Constant)));
    }

    #[test]
    fn test_js_basico() {
        let k = kinds_lang("function f() {}", Lang::JavaScript);
        assert!(k.contains(&("function".to_string(), Kind::Keyword)));
        assert!(k.contains(&("f".to_string(), Kind::Function)));
        let t = kinds_lang("const s = `hola`;", Lang::JavaScript);
        assert!(t.iter().any(|(_, kind)| *kind == Kind::String));
        let c = kinds_lang("// hola", Lang::JavaScript);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].1, Kind::Comment);
        let b = kinds_lang("/* a */ let x = 1;", Lang::JavaScript);
        assert!(b.iter().any(|(_, kind)| *kind == Kind::Comment));
        assert!(b.contains(&("let".to_string(), Kind::Keyword)));
        let n = kinds_lang("let x = null;", Lang::JavaScript);
        assert!(n.contains(&("null".to_string(), Kind::Constant)));
    }

    #[test]
    fn test_c_basico() {
        let k = kinds_lang("#include <stdio.h>", Lang::C);
        assert!(k.iter().any(|(_, kind)| *kind == Kind::Keyword));
        let f = kinds_lang("int main() {", Lang::C);
        assert!(f.contains(&("int".to_string(), Kind::Keyword)));
        assert!(f.contains(&("main".to_string(), Kind::Function)));
        let b = kinds_lang("/* a */ int x;", Lang::C);
        assert!(b.iter().any(|(_, kind)| *kind == Kind::Comment));
        let ch = kinds_lang("char c = 'x';", Lang::C);
        assert!(ch.iter().any(|(_, kind)| *kind == Kind::String));
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
        assert!(
            k.iter()
                .any(|(w, kind)| *kind == Kind::Lifetime && w == "'a")
        );
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
        assert!(
            k.iter()
                .any(|(w, kind)| *kind == Kind::String && w.starts_with('"'))
        );
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
        assert!(
            k.iter()
                .any(|(w, kind)| *kind == Kind::Number && w == "42u32")
        );
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
