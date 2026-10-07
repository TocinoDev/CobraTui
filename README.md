# 🐍 CobraTUI

[![Release](https://img.shields.io/github/v/release/TocinoDev/CobraTui)](https://github.com/TocinoDev/CobraTui/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE-MIT)
[![Built with Rust](https://img.shields.io/badge/Built_with-Rust-orange?logo=rust)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-blue)](https://github.com/TocinoDev/CobraTui)

Editor de texto en terminal: ligero, rápido y sin distracciones. Escrito en Rust con `ratatui` + `crossterm`, con estética tipo VSCode — explorador lateral, paleta de comandos, temas y resaltado de sintaxis.

> **Aviso legal:** software experimental proporcionado "tal cual", sin garantías de ningún tipo. Lee la sección [Descargo de responsabilidad](#descargo-de-responsabilidad) antes de usarlo.

## ✨ Características

- **Edición fluida**: escribir, borrar, nueva línea y cursor con memoria de columna.
- **Explorador lateral**: navegación de carpetas (`..`, `Backspace` para subir).
- **Paleta de comandos** (`Ctrl+P`): `/themes`, `/save`, `/open`, `/folder`, `/new`, `/quit`.
- **4 temas + resaltado**: `cobra-dark`, `dracula`, `monokai`, `ocean`; keywords, funciones, tipos, constantes, strings, números, comentarios y lifetimes.
- **Guardado atómico**: temporal + `fsync` + `rename`; aviso de cambios sin guardar; scroll vertical y horizontal; cursor con parpadeo suave.
- **Privacidad por diseño**: sin red, sin telemetría, sin cuentas, sin secretos en el repo.

## 📋 Requisitos

- Rust estable reciente (edition 2024) — solo si compilás desde fuente.
- Windows, Linux o macOS con terminal que soporte Unicode.

## 📦 Instalación

**Opción A — zip listo (Windows 64-bit):** descargá `CobraTUI-v0.1.0-win64.zip` de
[Releases](https://github.com/TocinoDev/CobraTui/releases), descomprimí y ejecutá `CobraTUI.exe`.

**Opción B — compilar desde fuente** (recomendado si desconfiás del binario: así verificás vos el código):

```sh
git clone https://github.com/TocinoDev/CobraTui.git
cd CobraTUI
cargo build --release
```

El binario queda en `target/release/CobraTUI` (`CobraTUI.exe` en Windows).

## 🚀 Uso

```sh
cargo run
# o
./target/release/CobraTUI
```

Al abrir verás el menú principal. Abrí una carpeta o archivo para entrar al editor.

### ⌨️ Atajos

| Atajo              | Acción                              |
|--------------------|-------------------------------------|
| `Ctrl+P`           | Paleta de comandos                  |
| `Ctrl+K`           | Abrir carpeta                       |
| `Ctrl+O`           | Abrir archivo                       |
| `Ctrl+N`           | Nuevo archivo (pide confirmación si pisa uno existente) |
| `Ctrl+S`           | Guardar (atómico)                   |
| `Ctrl+Q`           | Salir (avisa si hay cambios sin guardar) |
| `Tab`              | Cambiar foco explorer ↔ editor      |
| `Ctrl+A`           | Nuevo archivo en el explorer (pide nombre) |
| `Ctrl+D`           | Borrar seleccionado (pide confirmación) |
| `↑↓` / `Enter`     | Navegar y abrir en el explorer      |
| `Backspace`        | Subir carpeta (en explorer)         |
| `Esc`              | Volver al menú                      |

### 🎨 Paleta de comandos

`Ctrl+P` abre la entrada arriba del todo. Escribí para filtrar, `↑↓` para elegir, `Enter` para ejecutar:

`/themes` (elegir tema, ej. `/themes dra`), `/save`, `/open`, `/folder`, `/new`, `/quit`.

## 🗂️ Estructura

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

## 🔒 Seguridad

- Archivos de más de 10 MiB se rechazan antes de leerlos.
- Rutas remotas (UNC) y dispositivos reservados (`CON`, `NUL`…) se rechazan.
- Sin `unwrap()`/`expect()` en el código propio fuera de tests; errores con notificación, sin pánicos.
- En Windows se quita el directorio actual del orden de búsqueda de DLLs.
- Detalle completo y auditorías en [`docs/SECURITY.md`](docs/SECURITY.md).

## ✅ Verificar la descarga

Cada release incluye `SHA256SUMS.txt` junto al zip. En Windows:

```sh
certutil -hashfile CobraTUI-v0.1.0-win64.zip SHA256
```

El hash debe coincidir con el publicado. Si no coincide, no lo ejecutes.

## 📌 Estado

Proyecto en desarrollo activo. Funciona para edición diaria de archivos de texto y código. Ver [`docs/`](docs/) para arquitectura y decisiones.

## 🤖 Transparencia sobre IA

Este proyecto se desarrolla con asistencia de IA (generación de código,
documentación y revisión), siempre con supervisión y decisión humana en
cada cambio. Todo el código incluido es original del proyecto o de sus
dependencias open-source debidamente licenciadas (ver `Cargo.lock`).

## ⚠️ Descargo de responsabilidad

- **Uso bajo tu propio riesgo.** Este software se distribuye "TAL CUAL", sin garantías expresas ni implícitas, incluyendo —sin limitación— garantías de comerciabilidad, idoneidad para un fin particular o ausencia de errores.
- **Pérdida de datos.** Es un editor con capacidad de crear, modificar y sobrescribir archivos. Aunque el guardado es atómico y pide confirmación antes de descartar cambios o sobrescribir, **vos sos responsable de tus copias de seguridad**. El autor no responde por archivos perdidos, dañados o modificados por el uso (o mal uso) del programa.
- **Uso indebido.** El autor no se hace responsable por daños directos o indirectos derivados del uso indebido del software: edición o borrado de archivos del sistema, apertura de contenido malicioso, ejecución en entornos críticos o cualquier uso contrario a la ley.
- **Sin afiliación.** Proyecto personal independiente, sin relación con los autores de `ratatui`, `crossterm` ni otras dependencias.
- Al descargar, compilar o ejecutar CobraTUI aceptás estas condiciones. Si no estás de acuerdo, no lo uses.

## 📄 Licencia

MIT — ver [`LICENSE-MIT`](LICENSE-MIT). Gratis para uso personal y
comercial: podés usar, copiar, modificar y distribuir el software con
la única condición de conservar el aviso de copyright.
