# Picker

Ubicación: `src/Editor/picker.rs:8-75`

## Estado
```rust
pub struct Picker {
    files: Vec<String>,
    selected: usize,
}
```

## `new() -> Self`
`std::fs::read_dir(".").unwrap()` → `entry.file_name().to_string_lossy()`, `selected=0`. No filtra dirs (muestra archivos y carpetas). Orden no garantizado (no `sort`).

## `draw(&self, f: &mut Frame, area: Rect, focused: bool)`
- `List::new(files.iter().map(|n| ListItem::new(n)))`
- `Block::title(folder)` donde `folder = current_dir().file_name()` o `"Archivos"`, `borders ALL`, `border_style Cyan` si `focused`, `White` si no.
- `ListState::default().select(Some(selected))`, `render_stateful_widget`.

## Navegación
- `move_up(&mut self)` — `if selected>0 { selected-=1 }`
- `move_down(&mut self)` — `if selected+1 < len { selected+=1 }`
- `selected_file(&self) -> Option<&String>` — `files.get(selected)`

## Uso en `main`
`*focus == Picker` → `Up/Down` mueve, `Enter` abre `Buffer::from_file(&path)` y cambia `focus=Editor`, `Esc` sale, `Tab` cambia foco.
