# Temas y resaltado

Ubicación: `src/themes.rs`, `src/highlight.rs`.

## Temas

| Nombre       | Acento   | Uso                          |
|--------------|----------|------------------------------|
| `cobra-dark` | Cyan     | Por defecto                  |
| `dracula`    | Magenta  | Alternativo cálido           |
| `monokai`    | Amarillo | Contraste alto               |
| `ocean`      | Azul     | Frío                         |

Cada tema define 8 colores de sintaxis: `keyword`, `string`, `number`,
`comment`, `function`, `type_`, `constant` y `lifetime`. El tema tiñe
bordes enfocados, números de línea activos, teclas de la paleta, popup
de aviso y las categorías del resaltado. El índice se valida
(`set_theme` ignora índices inválidos, `get` usa fallback).

## Persistencia

El tema elegido se guarda en `%APPDATA%\CobraTUI\theme` (Windows) o
`$XDG_CONFIG_HOME/CobraTUI/theme` (`~/.config`, resto) y se carga al
arrancar. Nombre de archivo fijo, contenido acotado a 64 chars y
valores inválidos ignorados. Solo tu usuario accede a esa carpeta.

## Resaltado (`highlight_line`)

Escáner lineal O(n) sin regex y sin I/O, solo sobre líneas visibles
y 512 caracteres por línea como máximo. Todas las palabras tienen color:

- `//` → comentario hasta fin de línea
- `"…"` → string con escapes (`\"`), tolera sin cerrar
- `'x'` → char literal
- `'a` → lifetime (los labels `'loop:` también)
- `#[...]` → atributo (color de keyword)
- Dígitos → número (con sufijos tipo `42u32`)
- `foo(`, `obj.m()` → función (macros `vec![` incluidas)
- `String`, `Foo` → tipo; `MAX_SIZE` → constante
- Resto de identificadores contra lista de keywords (`binary_search`)

Todos los rangos son fronteras UTF-8 (verificado en tests con emoji).
Sin archivo abierto no hay resaltado (se muestra el panel de bienvenida).
