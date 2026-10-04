# 🐍 CobraTUI

Editor de texto en terminal, ligero y rápido, escrito en Rust con `ratatui` + `crossterm`. Estilo VSCode: explorador lateral, paleta de comandos, temas y resaltado de sintaxis.

## Características

- Edición literal (escribir, borrar, nueva línea, mover cursor con memoria de columna).
- Explorador lateral con navegación de carpetas (`..`, `Backspace` para subir).
- Paleta de comandos (`Ctrl+P`): `/themes`, `/save`, `/open`, `/folder`, `/new`, `/quit`.
- 4 temas (`cobra-dark`, `dracula`, `monokai`, `ocean`) + resaltado (keywords, funciones, tipos, constantes, strings, números, comentarios, lifetimes).
- Guardado atómico, aviso de cambios sin guardar, scroll vertical y horizontal, cursor con parpadeo suave.
- Sin red, sin telemetría, sin secretos en el repo.

## Requisitos

- Rust estable reciente (edition 2024).
- Windows, Linux o macOS con terminal que soporte Unicode.

## Instalación

```sh
git clone <tu-repo>
cd CobraTUI
cargo build --release
```

El binario queda en `target/release/CobraTUI` (`CobraTUI.exe` en Windows).

## Uso

```sh
cargo run
# o
./target/release/CobraTUI
```

Al abrir verás el menú principal. Abrí una carpeta o archivo para entrar al editor.

### Atajos

| Atajo              | Acción                              |
|--------------------|-------------------------------------|
| `Ctrl+P`           | Paleta de comandos                  |
| `Ctrl+K`           | Abrir carpeta                       |
| `Ctrl+O`           | Abrir archivo                       |
| `Ctrl+N`           | Nuevo archivo (pide confirmación si pisa uno existente) |
| `Ctrl+S`           | Guardar (atómico)                   |
| `Ctrl+Q`           | Salir (avisa si hay cambios sin guardar) |
| `Tab`              | Cambiar foco explorer ↔ editor      |
| `↑↓` / `Enter`     | Navegar y abrir en el explorer      |
| `Backspace`        | Subir carpeta (en explorer)         |
| `Esc`              | Volver al menú                      |

### Paleta de comandos

`Ctrl+P` abre la entrada arriba del todo. Escribí para filtrar, `↑↓` para elegir, `Enter` para ejecutar:

`/themes` (elegir tema, ej. `/themes dra`), `/save`, `/open`, `/folder`, `/new`, `/quit`.

## Estructura

```
src/
  main.rs              # Arranque, loop 60fps, modos Menu/Editing, diálogos
  OxideEngine/oxide.rs # Motor de texto puro (Buffer: líneas, cursor, I/O)
  Editor/editor.rs     # Editor, scroll, temas, notificaciones
  Editor/picker.rs     # Explorador lateral
  Menu/menu.rs         # Menú principal y panel de bienvenida
  themes.rs            # Paletas de color
  highlight.rs         # Resaltado lineal sin regex
  palette.rs           # Paleta de comandos
  util.rs              # Helpers compartidos
docs/                  # Documentación detallada (arquitectura, uso, seguridad…)
```

## Seguridad

- Archivos de más de 10 MiB se rechazan antes de leerlos.
- Sin `unwrap()` en rutas de producción; errores con notificación, sin pánicos.
- En Windows se quita el directorio actual del orden de búsqueda de DLLs.
- Detalle completo en [`docs/SECURITY.md`](docs/SECURITY.md).

## Estado

Proyecto en desarrollo activo. Funciona para edición diaria de archivos de texto y código. Ver [`docs/`](docs/) para arquitectura y decisiones.

## Transparencia sobre IA

Este proyecto se desarrolla con asistencia de IA (generación de código,
documentación y revisión), siempre con supervisión y decisión humana en
cada cambio. Todo el código incluido es original del proyecto o de sus
dependencias open-source debidamente licenciadas (ver `Cargo.lock`).

## Licencia

MIT — ver [`LICENSE-MIT`](LICENSE-MIT). Gratis para uso personal y
comercial: podés usar, copiar, modificar y distribuir el software con
la única condición de conservar el aviso de copyright.
