# Main

Ubicación: `src/main.rs:1-119`

## Setup
```rust
mod Editor; mod OxideEngine;
use Editor::editor::Editor as CobraEditor;
use Editor::picker::Picker;
use OxideEngine::oxide::Buffer;
enum Focus { Picker, Editor }
```

## `main() -> Result<()>`
`enable_raw_mode` → `EnterAlternateScreen` → `CrosstermBackend(stdout)` → `Terminal::new` → `editor=Editor::new()` con `buffer=Buffer::new("")` vacío → `picker=Picker::new()` → `focus=Editor` → `run_app` → `disable_raw_mode` + `LeaveAlternateScreen` + `show_cursor`.

## `run_app<B: Backend>(terminal, editor, picker, focus) -> Result<()>`
Loop 60fps:
```rust
terminal.draw(|f| {
    let chunks = Layout::horizontal([30%, 70%]).split(f.size());
    picker.draw(f, chunks[0], *focus==Picker);
    editor.draw(f, chunks[1]);
})?;
if poll(16ms) { if Event::Key(key) {
    if key.kind != Press { continue; }
    if Ctrl+Q { return Ok(()); }
    if Tab { *focus = toggle; continue; }
    if Picker { Up/Down/Enter/Esc }
    else { if !editor.handle_key(key) { return Ok(()); } }
}}
```

## Eventos
- `Tab` siempre cambia `Focus`.
- `Ctrl+Q` sale desde cualquier foco.
- `Esc` en `Picker` sale, en `Editor` vacío volvería a menú (ahora directo sale, antes volvía a `AppMode::Menu`).
- `Enter` en `Picker` intenta `Buffer::from_file(&path)` y si `Ok` lo asigna a `editor.buffer`.
