# Uso

## Atajos
- **Escribir** — inserta `Char` en cursor
- **Enter** — `insert_newline` (corta línea)
- **Backspace** — borra anterior o une líneas si `x==0`
- **Flechas** — `move_cursor` con `clamp`
- **Tab** — 4 espacios en `Editor`; `Tab` en `main` cambia foco `Picker↔Editor`
- **Esc** — sale (en `Editor` vacío antes volvía al menú)
- **Ctrl+S** — guarda `test.txt` en `current_dir` + popup `guardado con exito en: {abs}` 2s (negro borde blanco, sin fondo, centrado) o `falla…` rojo 3s
- **Ctrl+Q** — sale desde cualquier foco

## Picker 30%
- `Up/Down` navega, `Enter` abre archivo con `Buffer::from_file`, borde `Cyan` si enfocado.

## Bienvenida 70%
- Si `buffer` vacío (`[""]`) muestra cobra ASCII pequeña + `CobraTUI` + atajos `Enter Abrir │ Ctrl+S Guardar │ Tab …` centrados. Al escribir o abrir archivo desaparece y muestra `{:>3} | line` con scroll.

## Guardado
`test.txt` se crea al lado de `Cargo.toml` (donde lanzas `cargo run`). Verifica con `Get-Content test.txt` o `cat test.txt`.
