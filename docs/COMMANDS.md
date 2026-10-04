# Paleta de comandos

`Ctrl+P` abre la entrada de texto arriba del todo (solo en modo edición).
`Esc` o `Ctrl+P` la cierran. Mientras está abierta, el editor y el picker
quedan desenfocados y todas las teclas van a la paleta.

## Comandos

| Comando   | Acción                          |
|-----------|---------------------------------|
| `/themes` | Lista los temas para elegir     |
| `/save`   | Guarda el archivo actual        |
| `/open`   | Abrir archivo (diálogo)         |
| `/folder` | Abrir carpeta (diálogo)         |
| `/new`    | Nuevo archivo (diálogo)          |
| `/quit`   | Salir                           |

Escribí para filtrar (`/sa` → `/save`). `↑↓` navegan sugerencias,
`Tab` autocompleta con la seleccionada, `Enter` ejecuta, comando
desconocido avisa sin hacer nada.

## `/themes`

Escribí `/themes` y filtrá por nombre (`/themes dra` → `dracula`).
`Enter` aplica el tema y muestra `theme: <nombre>`. Con `/themes`
pelado, `Enter` no hace nada (solo abre el modo lista).

## Seguridad

- Entrada acotada a 64 caracteres, máximo 6 sugerencias.
- Los comandos son literales estrictos: no hay eval ni ejecución.
- Cambiar de archivo con cambios sin guardar pasa por el modal
  de confirmación (`[S]` guardar, `[D]` descartar, `[Esc]` cancelar).
