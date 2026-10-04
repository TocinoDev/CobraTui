# Editor

Ubicación: `src/Editor/editor.rs:15-210`

## Estado
```rust
pub struct Editor {
    pub buffer: Buffer,
    scroll_y: usize,
    notification: Option<String>,
    notification_expires: Option<Instant>,
}
```

## `new()`
`Buffer::new("")` vacío para mostrar bienvenida VSCode.

## `draw(&mut self, f: &mut Frame, area: Rect)`
- Si `is_empty()` ( `lines==[""]` ) → bienvenida centrada: cobra ASCII pequeña (sin fondo `@`), `CobraTUI`, `Enter Abrir │ Ctrl+S Guardar │ Tab …` con `Block` borde + `Paragraph` `Alignment::Center`.
- Si no vacío → `visible_h = inner.height`, ajusta `scroll_y` para que `cursor_y` siempre visible, `skip(scroll_y).take(visible_h)` + `Paragraph` con números `{:>3} |`, `f.set_cursor(inner.x+6+cx, inner.y+visible_y)`.
- Popup notificación: si `notification.is_some() && now < expires`, `centered_rect(w=msg.len()+4, h=3)` + `Clear` + `Block` borde blanco (sin `bg` para fondo transparente) + `Paragraph` `White` o `Red` si falla.

## `handle_key(&mut self, key: KeyEvent) -> bool`
- Filtra `key.kind != Press` (evita duplicado Windows `Press+Release`).
- `Ctrl+S` → `buffer.save("test.txt")`, pone `notification = "guardado con exito en: {abs}"` 2s o `"falla…"` 3s.
- Limpia notificación expirada.
- `Enter` → `insert_newline`, `Backspace` → `delete_char`, `Char(c)` → `insert_char`, `Left/Right/Up/Down` → `move_cursor`, `Tab` → 4 espacios, `Esc` → `return false`.

## Scroll
`scroll_y` (vertical) y `scroll_x` (horizontal, en chars) se actualizan en `draw` (necesita `&mut`): la ventana sigue al cursor en ambas direcciones. Las líneas largas se recortan a la ventana visible por frontera UTF-8 (`byte_idx`), con resaltado y cursor superpuestos sin clonar de más.

## `centered_rect(w,h,area) -> Rect`
Centra popup: `x = area.x + (area.width-w)/2`, `y` igual.
