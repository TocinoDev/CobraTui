/// Motor de texto puro de `CobraTUI` (sin dependencias de TUI).
/// Gestiona líneas, cursor y edición. 100% testeable sin terminal.
#[derive(Debug)]
pub struct Buffer {
    /// Cada entrada es una línea sin `\n` (el `\n` se añade en `to_string`).
    lines: Vec<String>,
    /// Columna del cursor en `chars` (no bytes), `0..=line_len`.
    cursor_x: usize,
    /// Fila del cursor, 0..=lines.len()-1.
    cursor_y: usize,
    /// Columna deseada al moverse verticalmente (memoria de columna).
    goal_x: usize,
}

impl Buffer {
    /// Getters para que `editor.rs` pueda renderizar sin exponer `Vec` mutable.
    pub fn lines(&self) -> &[String] {
        &self.lines
    }
    pub fn cursor(&self) -> (usize, usize) {
        (self.cursor_x, self.cursor_y)
    }

    /// Crea un buffer desde texto. Normaliza `\r\n` y `\r` a `\n`. `""` -> `[""]`.
    pub fn new(text: &str) -> Self {
        // Los dotfiles (.gitignore, etc.) en Windows suelen traer CRLF.
        // Un `\r` suelto dentro de una linea mueve el cursor al inicio
        // de la fila en el terminal y corrompe el render.
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        let lines: Vec<String> = normalized
            .split('\n')
            .map(std::string::ToString::to_string)
            .collect();
        Self {
            lines,
            cursor_x: 0,
            cursor_y: 0,
            goal_x: 0,
        }
    }

    /// Mueve el cursor relativo con clamp seguro (no paniquea en líneas vacías).
    /// `dx`/`dy` pueden ser negativos; el cursor se limita a `[0, len]`.
    /// Movimiento vertical conserva la columna deseada (`goal_x`): si la
    /// linea nueva es mas corta queda al final, y al volver a una mas
    /// larga recupera la misma posicion.
    pub fn move_cursor(&mut self, dx: i32, dy: i32) {
        if dy != 0 {
            let new_y = self.cursor_y.saturating_add_signed(dy as isize);
            self.cursor_y = new_y.clamp(0, self.lines.len().saturating_sub(1));
            let line_len = self.lines[self.cursor_y].chars().count();
            // Si ademas hay dx (diagonal), primero actualiza la meta.
            if dx != 0 {
                self.goal_x = self.goal_x.saturating_add_signed(dx as isize);
            }
            self.cursor_x = self.goal_x.min(line_len);
        } else {
            let line_len = self.lines[self.cursor_y].chars().count();
            let new_x = self.cursor_x.saturating_add_signed(dx as isize);
            self.cursor_x = new_x.clamp(0, line_len);
            self.goal_x = self.cursor_x;
        }
    }

    /// Byte index del `char_idx`-esimo char (frontera UTF-8 segura).
    fn byte_idx(line: &str, char_idx: usize) -> usize {
        line.char_indices()
            .nth(char_idx)
            .map_or(line.len(), |(i, _)| i)
    }

    /// Inserta un `char` en la posición del cursor y avanza `cursor_x`.
    /// Opera in-place sobre el `String` (sin clonar la linea).
    pub fn insert_char(&mut self, ch: char) {
        let idx = Self::byte_idx(&self.lines[self.cursor_y], self.cursor_x);
        self.lines[self.cursor_y].insert(idx, ch);
        self.cursor_x += 1;
        self.goal_x = self.cursor_x;
    }
    /// Backspace: borra el char anterior o une líneas si `x==0 && y>0`.
    pub fn delete_char(&mut self) {
        if self.cursor_x > 0 {
            let idx = Self::byte_idx(&self.lines[self.cursor_y], self.cursor_x - 1);
            self.lines[self.cursor_y].remove(idx);
            self.cursor_x -= 1;
        } else if self.cursor_x == 0 && self.cursor_y > 0 {
            let current = self.lines.remove(self.cursor_y);
            self.cursor_y -= 1;
            self.cursor_x = self.lines[self.cursor_y].chars().count();
            self.lines[self.cursor_y].push_str(&current);
        }
        self.goal_x = self.cursor_x;
    }

    /// Inserta un salto de línea literal en el cursor (Enter).
    /// Corta en el byte index del cursor: trunca + inserta (una sola alloc).
    pub fn insert_newline(&mut self) {
        let idx = Self::byte_idx(&self.lines[self.cursor_y], self.cursor_x);
        let right = self.lines[self.cursor_y][idx..].to_string();
        self.lines[self.cursor_y].truncate(idx);
        self.lines.insert(self.cursor_y + 1, right);
        self.cursor_y += 1;
        self.cursor_x = 0;
        self.goal_x = 0;
    }

    /// Tamaño máximo de archivo al abrir (`10 MiB`). Sin tope, un archivo
    /// gigante se cargaría entero en memoria (`DoS` de memoria).
    pub const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;

    /// Carga buffer desde archivo (para picker `Enter`).
    /// Apertura segura en un solo paso (sin chequeo previo separado):
    /// rechaza nombres de dispositivo reservados de Windows, abre el
    /// handle, exige archivo regular y lee como máximo `MAX_FILE_BYTES`
    /// + 1 (si hay un byte de más, es demasiado grande).
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        use std::io::Read as _;
        let fs_path = std::path::Path::new(path);
        if crate::util::is_reserved_device_name(fs_path) {
            return Err(anyhow::anyhow!("nombre de dispositivo reservado"));
        }
        let mut file = std::fs::File::open(fs_path)?;
        if !file.metadata()?.is_file() {
            return Err(anyhow::anyhow!("no es un archivo regular"));
        }
        let mut content = String::new();
        let n = file
            .by_ref()
            .take(Self::MAX_FILE_BYTES + 1)
            .read_to_string(&mut content)?;
        if n as u64 > Self::MAX_FILE_BYTES {
            return Err(anyhow::anyhow!(
                "archivo demasiado grande (más de {} B)",
                Self::MAX_FILE_BYTES
            ));
        }
        Ok(Self::new(&content))
    }

    /// Longitud en bytes del contenido serializado (sin alocarlo).
    /// Para la status bar 60fps: evita `join` por frame.
    pub fn byte_len(&self) -> usize {
        self.lines.iter().map(String::len).sum::<usize>() + self.lines.len().saturating_sub(1)
    }

    /// Intentos para crear el temporal antes de rendirse.
    const MAX_TMP_ATTEMPTS: u32 = 16;

    /// Guarda de forma atómica: crea un temporal con `create_new` en el
    /// mismo directorio (nombre `.tmp-cobra-<pid>-<n>`, hasta 16 intentos
    /// ante `AlreadyExists`), escribe, sincroniza, cierra y renombra.
    /// Si algo falla, borra el temporal. Copia los permisos del original
    /// para no alterar el modo del archivo. Rechaza symlinks, junctions
    /// y todo destino que no sea archivo regular.
    pub fn save(&self, path: &str) -> anyhow::Result<()> {
        use std::io::Write as _;
        let target = std::path::Path::new(path);
        // Sin seguir enlaces: un symlink/junction como destino se rechaza.
        // `NotFound` (archivo nuevo) sigue adelante.
        match std::fs::symlink_metadata(path) {
            Ok(m) => {
                let ft = m.file_type();
                if ft.is_symlink() || !ft.is_file() {
                    return Err(anyhow::anyhow!("destino no es un archivo regular"));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        let original_perms = std::fs::metadata(path).map(|m| m.permissions()).ok();
        let dir = target
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map_or_else(|| std::path::Path::new("."), |p| p);
        let pid = std::process::id();
        let mut slot: Option<(std::path::PathBuf, std::fs::File)> = None;
        for n in 0..Self::MAX_TMP_ATTEMPTS {
            let candidate = dir.join(format!(".tmp-cobra-{pid}-{n}"));
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&candidate)
            {
                Ok(f) => {
                    slot = Some((candidate, f));
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.into()),
            }
        }
        let (tmp_path, mut tmp) =
            slot.ok_or_else(|| anyhow::anyhow!("sin temporal libre tras 16 intentos"))?;
        // Toda falla a partir de aquí limpia el temporal antes de salir.
        if let Some(perms) = original_perms {
            if let Err(e) = tmp.set_permissions(perms) {
                let _ = std::fs::remove_file(&tmp_path);
                return Err(e.into());
            }
        }
        if let Err(e) = tmp.write_all(self.to_string().as_bytes()) {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(e.into());
        }
        if let Err(e) = tmp.sync_all() {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(e.into());
        }
        drop(tmp);
        if let Err(e) = std::fs::rename(&tmp_path, path) {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(e.into());
        }
        Ok(())
    }
}

impl std::fmt::Display for Buffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, line) in self.lines.iter().enumerate() {
            if i > 0 {
                f.write_str("\n")?;
            }
            f.write_str(line)?;
        }
        Ok(())
    }
}
/// Mutex para serializar los tests que guardan archivos: comparten el
/// espacio de nombres de temporales (mismo PID) y correrían en carrera.
#[cfg(test)]
pub(crate) static SAVE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::{Buffer, SAVE_TEST_LOCK};
    #[test]
    fn test_save_crea_archivo() {
        let _guard = SAVE_TEST_LOCK.lock().unwrap();
        let b = Buffer::new("hola test_save\nlinea2");
        let path = "test_save_unit.txt";
        b.save(path).unwrap();
        let content = std::fs::read_to_string(path).unwrap();
        assert_eq!(content, "hola test_save\nlinea2");
        std::fs::remove_file(path).unwrap();
        println!("test_save_unit OK - archivo creado correctamente");
    }

    #[test]
    fn test_crlf_no_deja_cr() {
        // Los dotfiles en Windows (.gitignore) suelen traer CRLF.
        // Un `\r` residual corrompe el render del terminal.
        let b = Buffer::new("linea1\r\nlinea2\r\n");
        assert!(!b.lines().iter().any(|l| l.contains('\r')));
        assert_eq!(
            b.lines(),
            &["linea1".to_string(), "linea2".to_string(), String::new()]
        );
        assert_eq!(b.to_string(), "linea1\nlinea2\n");
    }

    /// Temporales `.tmp-cobra-*` en el directorio actual. Los tests del
    /// mismo proceso comparten PID, así que se reintenta un momento por
    /// si otro test está guardando en ese instante.
    fn tmp_leftovers() -> Vec<String> {
        for _ in 0..40 {
            let left: Vec<String> = std::fs::read_dir(".")
                .map(|rd| {
                    rd.filter_map(Result::ok)
                        .map(|e| e.file_name().to_string_lossy().to_string())
                        .filter(|n| n.starts_with(".tmp-cobra-"))
                        .collect()
                })
                .unwrap_or_default();
            if left.is_empty() {
                return left;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        std::fs::read_dir(".")
            .map(|rd| {
                rd.filter_map(Result::ok)
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .filter(|n| n.starts_with(".tmp-cobra-"))
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn test_save_atomico_sin_restos() {
        let _guard = SAVE_TEST_LOCK.lock().unwrap();
        let path = "test_atomic_unit.txt";
        let _ = std::fs::remove_file(path);
        let b = Buffer::new("hola\natomico");
        b.save(path).unwrap();
        assert_eq!(std::fs::read_to_string(path).unwrap(), "hola\natomico");
        // Sobreescribir tambien es atomico y exacto.
        let b2 = Buffer::new("otro");
        b2.save(path).unwrap();
        assert_eq!(std::fs::read_to_string(path).unwrap(), "otro");
        assert!(tmp_leftovers().is_empty(), "no debe quedar el temporal");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_save_colision_temporal_usa_siguiente() {
        let _guard = SAVE_TEST_LOCK.lock().unwrap();
        // Pre-crea el primer candidato `.tmp-cobra-<pid>-0`: el save debe
        // saltarlo (create_new falla con AlreadyExists) sin tocarlo.
        let path = "test_collision_unit.txt";
        let first = format!(".tmp-cobra-{}-0", std::process::id());
        let _ = std::fs::remove_file(path);
        std::fs::write(&first, "marcador").unwrap();
        Buffer::new("nuevo").save(path).unwrap();
        assert_eq!(std::fs::read_to_string(path).unwrap(), "nuevo");
        assert_eq!(std::fs::read_to_string(&first).unwrap(), "marcador");
        let _ = std::fs::remove_file(&first);
        let _ = std::fs::remove_file(path);
        assert!(tmp_leftovers().is_empty());
    }

    #[test]
    fn test_save_conserva_permisos() {
        let _guard = SAVE_TEST_LOCK.lock().unwrap();
        let path = "test_perms_unit.txt";
        let _ = std::fs::remove_file(path);
        std::fs::write(path, "x").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mut perms = std::fs::metadata(path).unwrap().permissions();
            perms.set_mode(0o640);
            std::fs::set_permissions(path, perms).unwrap();
        }
        Buffer::new("y").save(path).unwrap();
        assert_eq!(std::fs::read_to_string(path).unwrap(), "y");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(
                std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o640
            );
        }
        #[cfg(windows)]
        {
            // En Windows solo se valida que el flag readonly sobrevive.
            assert!(!std::fs::metadata(path).unwrap().permissions().readonly());
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_from_file_inexistente_falla_sin_panico() {
        assert!(Buffer::from_file("no_existe_cobra_sec_unit.txt").is_err());
    }

    #[test]
    fn test_from_file_rechaza_directorio() {
        assert!(Buffer::from_file(".").is_err());
    }

    #[test]
    fn test_from_file_rechaza_reservados() {
        // Sin tocar disco en Windows: CON/NUL se rechazan por nombre.
        assert!(Buffer::from_file("CON").is_err());
        assert!(Buffer::from_file("nul.txt").is_err());
        assert!(Buffer::from_file("COM1").is_err());
    }

    #[test]
    fn test_from_file_rechaza_gigante() {
        let path = "test_big_unit.txt";
        let big = "a".repeat((Buffer::MAX_FILE_BYTES + 1) as usize);
        std::fs::write(path, big).unwrap();
        let err = Buffer::from_file(path).unwrap_err().to_string();
        assert!(err.contains("demasiado grande"), "error: {err}");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_from_file_dotfile_con_crlf() {
        let path = "test_dotfile_unit.gitignore";
        std::fs::write(path, "target/\r\n.env\r\n").unwrap();
        let b = Buffer::from_file(path).unwrap();
        assert!(!b.lines().iter().any(|l| l.contains('\r')));
        assert_eq!(b.to_string(), "target/\n.env\n");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_vertical_conserva_columna() {
        let mut b = Buffer::new("1234567890\nab\n1234567890");
        b.move_cursor(8, 0);
        assert_eq!(b.cursor(), (8, 0));
        // Baja a linea corta: queda al final (2)
        b.move_cursor(0, 1);
        assert_eq!(b.cursor(), (2, 1));
        // Baja a linea larga: recupera la columna 8
        b.move_cursor(0, 1);
        assert_eq!(b.cursor(), (8, 2));
        // Sube dos: corta al final, larga recupera 8
        b.move_cursor(0, -1);
        assert_eq!(b.cursor(), (2, 1));
        b.move_cursor(0, -1);
        assert_eq!(b.cursor(), (8, 0));
    }

    #[test]
    fn test_horizontal_resetea_meta() {
        let mut b = Buffer::new("1234567890\nab\n1234567890");
        b.move_cursor(8, 0);
        b.move_cursor(0, 1);
        assert_eq!(b.cursor(), (2, 1));
        // Moverse horizontalmente fija una nueva meta
        b.move_cursor(-1, 0);
        assert_eq!(b.cursor(), (1, 1));
        b.move_cursor(0, 1);
        assert_eq!(b.cursor(), (1, 2));
    }
}
