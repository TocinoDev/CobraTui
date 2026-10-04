# Build

## Requisitos
- Rust stable 1.95+ (`rustup`), Windows PowerShell

## Comandos
```powershell
cargo check          # 0 warnings (allow snake_case para Editor/OxideEngine)
cargo test -- --nocapture  # test_save_crea_archivo
cargo run            # inicia TUI (requiere TTY real, no panel de salida del IDE)
cargo build          # debug en target/debug/CobraTUI.exe
```

## Windows `Acceso denegado (os error 5)`
`target/debug/CobraTUI.exe` bloqueado porque sigue corriendo.
- Cierra con `Esc` o `Ctrl+Q` antes de recompilar
- O `taskkill /F /IM CobraTUI.exe`

## Estructura `Cargo.toml`
```toml
[package]
name = "CobraTUI"
edition = "2024"
[dependencies]
crossterm = "0.27"
ratatui = "0.24"
anyhow = "1"
```

## Tests
`src/OxideEngine/oxide.rs:95-108` — `test_save_crea_archivo` verifica `save`.
