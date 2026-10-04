# Arquitectura

## Principio
`Oxide` es puro y testeable sin terminal. `Editor` envuelve `Buffer` y lo dibuja. `main` solo orquesta terminal y foco.

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│ OxideEngine │◄────│   Editor     │◄────│    main     │
│  Buffer     │     │  Editor      │     │  Terminal   │
│  lines      │     │  scroll_y    │     │  Focus      │
│  cursor_x/y │     │  status      │     │  Layout     │
└─────────────┘     └──────────────┘     └─────────────┘
       ▲                    ▲                   │
       │ from_file/save     │ draw/handle_key   │ poll/draw
       └────────────────────┴───────────────────┘
```

## Módulos
- `OxideEngine::oxide::Buffer` — `Vec<String>` por líneas, `cursor_x/y` en `chars`, `clamp` seguro, `insert/remove` por `Vec<char>`.
- `Editor::editor::Editor` — `buffer: Buffer + scroll_y + notification`, `draw(&mut self, f, area)` con `Block`/`Paragraph`/`List` y `set_cursor`, `handle_key` filtra `Press`.
- `Editor::picker::Picker` — `files: Vec<String>, selected`, `read_dir`, `ListState`, `move_up/down`, `selected_file`.
- `main` — `Focus {Picker, Editor}`, `Layout 30/70`, `run_app` loop 60fps.

## Decisiones
- `Vec<String>` vs `Rope`: `Vec` simple para <10k líneas; migrar a `ropey` luego sin tocar `Editor` si se extrae trait `TextBuffer`.
- Snapshot undo (100) pospuesto: `Buffer` ya guarda `lines+cursor`, falta `history: Vec<State>`.
- Tab híbrido: en `Editor` inserta 4 espacios, `Tab` en `main` cambia `Focus`.
