# Oxide — Motor `Buffer`

Ubicación: `src/OxideEngine/oxide.rs:3-108`

## Estado
```rust
pub struct Buffer {
    lines: Vec<String>, // sin \n
    cursor_x: usize,    // chars, 0..=line_len
    cursor_y: usize,    // 0..=lines.len()-1
}
```

## API
- `new(text: &str) -> Self` — `split('\n')`, `cursor 0,0`. `""` → `[""]`.
- `lines(&self) -> &[String]` y `cursor() -> (usize,usize)` — getters para `Editor`.
- `move_cursor(&mut self, dx: i32, dy: i32)` — `clamp(0, len.saturating_sub(1))` para `y`, `clamp(0, line_len)` para `x`. No paniquea en líneas vacías.
- `insert_char(&mut self, ch: char)` — `Vec<char>` `insert(cursor_x)`, `cursor_x+=1`.
- `delete_char(&mut self)` — si `x>0` borra `x-1`; si `x==0 && y>0` une líneas (`remove(y)`, `y-=1`, `x=len(prev)`).
- `insert_newline(&mut self)` — corta `left=[..x]`, `right=[x..]`, `lines[y]=left`, `insert(y+1,right)`, `y+=1, x=0`.
- `Display` — serializa con `\n` sin intermediarios (`to_string()` via `ToString`).
- `byte_len(&self) -> usize` — longitud en bytes sin alocar (para la status bar 60fps).
- `from_file(path: &str) -> Result<Self>` — verifica `metadata` contra `MAX_FILE_BYTES` (10 MiB) y luego `read_to_string` + `new`. Archivos gigantes se rechazan con error en vez de cargarse enteros en memoria.
- `save(&self, path: &str) -> Result<()>` — `write(path, to_string)`.

## Detalles
- Todo en `chars` no `bytes` (utf8 seguro).
- `clamp` + `saturating_sub` evita overflow cuando `len==0`.
- 100% unit-testeable: `#[cfg(test)] test_save_crea_archivo` verifica `save`.

## Tests
`cargo test -- --nocapture` — crea `test_save_unit.txt`, verifica contenido, borra.
