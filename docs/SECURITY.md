# Seguridad

CobraTUI es un editor local monousuario sin red, auth ni secretos.
Mitigaciones aplicadas en el código:

- **Orden de búsqueda de DLLs (Windows):** `harden_dll_search_order()`
  en `src/main.rs` llama a `SetDllDirectoryW("")` al arrancar para
  quitar el directorio actual de la búsqueda. Protege si se lanza el
  editor desde una carpeta no confiable y alguna dependencia (actual o
  futura) carga una DLL por nombre relativo. Sin dependencias nuevas
  (FFI directo a `kernel32`, bloque `unsafe` mínimo y documentado).
- **Tope de tamaño al abrir:** `Buffer::from_file` rechaza archivos de
  más de `MAX_FILE_BYTES` (10 MiB) antes de leerlos.
- **Guardado atómico:** `Buffer::save` escribe a `*.tmp-cobra` + `fsync`
  + `rename`. Si el proceso muere a mitad de escritura, el original
  sigue intacto (sin truncado).
- **Sin truncado silencioso:** `Ctrl+N` sobre un archivo existente no
  vacío pide confirmación (`Pending::OverwriteNew`, solo `S` confirma).
- **Rutas UTF-8:** las rutas no UTF-8 se rechazan con notificación en
  vez de `unwrap()` (sin pánicos en producción; `unwrap` solo en tests).
- **CRLF normalizado:** `\r\n`/`\r` a `\n` al abrir para no corromper
  el render del terminal.
- **Render sanitizado:** los controles (`\t` → espacio, resto → `�`)
  solo en display; el buffer y lo guardado quedan intactos.
- **Terminal siempre restaurada:** hook de panic + restore best-effort
  en todos los caminos de `main` (raw mode + alternate screen).
- **Sin secretos:** no hay `.env`, claves ni tokens en el repo;
  `.gitignore` excluye `.env*`, `*.pem`, `*.key`, `credentials*`.
- **Paleta sin eval:** los comandos son literales estrictos
  (`/themes`, `/save`, …); entrada acotada a 64 caracteres.

## Dependencias (auditoría por lectura, sin red)

4 directas (`anyhow 1.0.104`, `crossterm 0.27.0`, `ratatui 0.24.0`,
`rfd 0.14.1`), 200 paquetes fijados en `Cargo.lock`. Duplicados solo
por majors distintos (`getrandom` 0.2/0.4, `hashbrown` 0.15/0.17,
`syn` 2/3, `windows-sys` 0.48–0.61) o por target Windows: normal, sin
acción. `build.rs` revisados: `anyhow` (probe de features con `rustc`
local) y `rfd` (solo flags de link por OS, en Windows no hace nada):
benignos. Sin scripts postinstall (Cargo no los tiene). Pendiente con
red: `cargo audit` para advisories (SA-001 del informe).

Ver informe completo de la última auditoría (solo lectura) en el
historial del proyecto: 0 confirmados, 1 pendiente (`cargo audit`
de dependencias, requiere red).

## Tests de seguridad (`cargo test`)

- `from_file` inexistente/gigante: falla con `Err`, sin pánico.
- `save_current` sin archivo y `open_file` inexistente: `Err` sin pánico.
- `needs_overwrite_confirm`: solo `true` si existe y no está vacío
  (inexistente y vacío dan `false`).
- Sanitizado: sin controles no aloca; `\t`→espacio, resto→`�`, 1:1
  en chars; vacía sigue vacía.
- Dirty tracking: limpio al abrir/guardar, sucio al editar.
