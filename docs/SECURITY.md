# Seguridad de CobraTUI

## Modelo de amenazas

CobraTUI es un editor de texto local y monousuario: sin red, sin
autenticación, sin multiusuario y sin secretos. El atacante relevante
**no** es remoto; son los datos que el usuario abre o el entorno desde
el que lo lanza:

1. **Archivos/carpetas no confiables** que el usuario abre (contenido
   con secuencias de escape, nombres con controles, symlinks,
   archivos gigantes o especiales).
2. **Directorio de lanzamiento hostil** (DLL planting si se ejecuta
   desde una carpeta controlada por un tercero).
3. **Rutas remotas (UNC)** que cuelgan la app o exfiltran NTLM.
4. **Fallo a mitad de escritura** (corte de luz, kill) que corrompa
   el archivo, y **pérdida de datos** (truncados silenciosos).
5. **Terminal rota** tras un panic (raw mode + alternate screen).

Fuera de alcance: red, sandboxing del SO, vulnerabilidades del
terminal o del sistema operativo en sí.

## Mitigaciones aplicadas

| Amenaza | Mitigación | Ubicación |
|---|---|---|
| DLL planting (CWD) | `SetDllDirectoryW("")` al arrancar | `src/main.rs` |
| DLL planting (link) | `+crt-static` (sin VCRUNTIME140), `/DEPENDENTLOADFLAG:0x800`, Control Flow Guard | `.cargo/config.toml` |
| Truncado por crash | Guardado atómico: temporal `create_new` + `fsync` + `rename`, con reintentos y limpieza | `src/OxideEngine/oxide.rs` (`atomic_write`) |
| Symlink/junction destino | Rechazo vía `symlink_metadata` (sin seguir enlaces) | `oxide.rs` (`save`) |
| Permisos alterados | Se copian los del original al temporal | `oxide.rs` (`save`) |
| Escape injection en terminal | Sanitizado solo-display (`util::sanitize`, `\t` preservado) en editor, explorer, notificaciones y paleta | `src/util.rs`, renders |
| Rutas UNC/remotas | `util::is_remote` (UNC, VerbatimUNC, DeviceNS); rechazo con aviso en diálogos, atajos y explorer | `src/util.rs`, `src/main.rs` |
| Archivo gigante (DoS memoria) | Tope de 10 MiB con lectura acotada (`take`), sin chequeo previo separado | `oxide.rs` (`from_file`) |
| Dispositivos `CON`/`NUL`/… | Rechazo por nombre reservado + exigencia de archivo regular | `util.rs`, `oxide.rs` |
| Truncado silencioso (Ctrl+N) | Confirmación obligatoria si existe y no está vacío | `src/main.rs` (`OverwriteNew`) |
| Cambios sin guardar | Modal `[S]/[D]/[Esc]` + marca `●` en el título | `src/main.rs`, `Editor` |
| Terminal rota | Hook de panic + restore best-effort en todos los caminos | `src/main.rs` |
| Preferencias corruptas | Tema con límite de 64 bytes, lista cerrada y guardado atómico | `src/themes.rs` |
| Secretos en repo | Sin `.env`/claves/tokens; `.gitignore` excluye `.env*`, `*.pem`, `*.key`, `credentials*` | `.gitignore` |

Sin `unwrap()`/`expect()` en el código propio fuera de tests (solo en
`#[cfg(test)]`); errores propagados con `anyhow` y notificados en UI.

## Verificación del binario (build limpio `cargo clean && cargo build --release`)

- `DependentLoadFlags=0x800` (solo System32) y `GuardFlags=0x10017500`
  (CFG) en el Load Config del PE.
- Imports solo de sistema (`kernel32`, `user32`, `ole32`, `shell32`,
  `ntdll`, …): sin `VCRUNTIME140.dll` ni `api-ms-win-crt-*`.

## Dependencias (auditoría por lectura, sin red)

4 directas (`anyhow 1.0.104`, `crossterm 0.27.0`, `ratatui 0.24.0`,
`rfd 0.14.1`), ~200 paquetes fijados en `Cargo.lock`. Duplicados solo
por majors distintos o targets Windows: normal. `build.rs` revisados
(`anyhow`: probe local de features; `rfd`: flags de link, en Windows
no-op): benignos. Reglas en `deny.toml` (requiere red para
`cargo deny check`). Pendiente con red: `cargo audit` de advisories.

Riesgo aceptado (Dependabot #1, low): RUSTSEC-2026-0002 en `lru`
0.12.5 (vía `ratatui 0.24`, fijado a `^0.12`; parche en 0.16.3).
Soundness teórico en `IterMut` (Stacked Borrows, sin CVE, CVSS 2.7),
sin uso directo propio y sin exploit conocido. Fix real = subir
`ratatui`+`crossterm` (cambio mayor, pendiente).

## Tests de seguridad (`cargo test`)

- `from_file` inexistente/gigante/directorio/reservado: `Err` sin pánico.
- `save_current` sin archivo y `open_file` inexistente: `Err` sin pánico.
- `needs_overwrite_confirm`: solo `true` si existe y no está vacío.
- Guardado atómico: contenido exacto, sin temporales, colisión salta
  al siguiente, permisos conservados.
- Sanitizado: sin controles no aloca; `\t` se preserva, resto a U+FFFD.
- `is_remote` / reservados: UNC/DeviceNS `true`, locales `false`.
- Tema: roundtrip, inválido/sobretamaño/binario ignorados, sin restos.
- Paleta acotada (64 chars, 6 sugerencias), dirty tracking, UTF-8.

## Reportar una vulnerabilidad

No abrir un issue público con detalles. Usar **Private vulnerability
reporting** del repo en GitHub (*Security → Report a vulnerability*),
describiendo versión afectada, pasos para reproducir e impacto. Se
responde publicando el fix y rotando lo expuesto si aplica.
