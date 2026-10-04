# CobraTUI — Documentación

Editor TUI ligero en Rust (`ratatui` + `crossterm`) con picker lateral estilo VSCode.

## Estructura
```
src/
  main.rs                 # Arranque, restore y loop 60fps + Focus Picker/Editor
  OxideEngine/oxide.rs    # Motor puro Buffer (sin TUI)
  Editor/editor.rs        # Editor literal + bienvenida + popup guardado + scroll
  Editor/picker.rs        # Picker lateral 30% (lista archivos, resalta selección)
docs/
  README.md               # Este archivo
  ARCHITECTURE.md         # Arquitectura y flujo
  OXIDE.md                # Motor Buffer
  EDITOR.md               # Editor y bienvenida
  PICKER.md               # Picker
  MAIN.md                 # Main y loop
  USAGE.md                # Uso y atajos
  BUILD.md                # Build, run y tests
```

## Stack
- `crossterm 0.27` — raw mode, alternate screen, eventos
- `ratatui 0.24` — widgets `Block`, `Paragraph`, `List`, `Layout`
- `anyhow 1` — errores

## Flujo
`main::main` → `enable_raw_mode` + `EnterAlternateScreen` → `run_app` → `terminal.draw` → `picker.draw` + `editor.draw` → `event::poll(16ms)` → `handle_key` → `disable_raw_mode` + `LeaveAlternateScreen`

## Estado
- Editor literal funciona: inserta, borra, newline, mueve, Tab=4 espacios, scroll vertical, `Ctrl+S` guarda `test.txt` con popup.
- Picker lista `read_dir(".")`, `Up/Down` navega, `Enter` abre con `Buffer::from_file`.
- Bienvenida VSCode cuando buffer vacío (cobra ASCII + `CobraTUI` + atajos) en el 70% junto al picker.
