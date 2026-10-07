//! Editor de texto `CobraTUI`.
use crate::OxideEngine::oxide::Buffer;
use crate::highlight::{self, Kind, Lang};
use crate::themes::{self, Theme};
use crate::util::centered_rect;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};
use std::time::{Duration, Instant};

pub struct Editor {
    pub buffer: Buffer,
    pub current_path: Option<std::path::PathBuf>,
    dirty: bool,
    theme: usize,
    scroll_y: usize,
    /// Primera columna visible (scroll horizontal en chars).
    scroll_x: usize,
    pub notification: Option<String>,
    pub notification_expires: Option<Instant>,
    cursor_visible: bool,
    last_blink: Instant,
}

/// Intervalo del parpadeo suave del cursor propio (ms).
const BLINK_MS: u64 = 530;

impl Editor {
    pub fn new() -> Self {
        Self {
            buffer: Buffer::new(""),
            current_path: None,
            dirty: false,
            theme: themes::DEFAULT_THEME,
            scroll_y: 0,
            scroll_x: 0,
            notification: None,
            notification_expires: None,
            cursor_visible: true,
            last_blink: Instant::now(),
        }
    }

    pub fn theme(&self) -> Theme {
        themes::get(self.theme)
    }

    pub fn theme_name(&self) -> &'static str {
        self.theme().name
    }

    /// Idioma para resaltado segun la extension del archivo actual.
    /// Sin archivo (o extension desconocida) cae a Rust.
    fn lang(&self) -> Lang {
        self.current_path
            .as_deref()
            .map_or(Lang::Rust, highlight::detect)
    }

    /// Cambia el tema por indice validado. Devuelve `false` si es invalido.
    pub fn set_theme(&mut self, index: usize) -> bool {
        if index < themes::THEMES.len() {
            self.theme = index;
            true
        } else {
            false
        }
    }

    /// Carga el tema persistido (si existe y es válido). Se llama una
    /// vez al arrancar; `set_theme` no persiste solo para no tocar
    /// disco en tests ni en cada preview.
    pub fn load_persisted_theme(&mut self) {
        if let Some(index) = themes::load_theme_index() {
            self.theme = index;
        }
    }

    /// Muestra el cursor solido y reinicia el ciclo de parpadeo.
    /// Se llama con cada tecla para no parpadear mientras se escribe.
    fn touch_cursor(&mut self) {
        self.cursor_visible = true;
        self.last_blink = Instant::now();
    }

    pub fn is_dirty(&self) -> bool {
        self.current_path.is_some() && self.dirty
    }

    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    fn mark_dirty(&mut self) {
        if self.current_path.is_some() {
            self.dirty = true;
        }
    }

    /// Guarda en el archivo actual y actualiza notificacion + marca limpia.
    /// Devuelve Ok(path) si guardo, Err si fallo o no hay archivo.
    pub fn save_current(&mut self) -> anyhow::Result<std::path::PathBuf> {
        let path = self
            .current_path
            .clone()
            .ok_or_else(|| anyhow::anyhow!("sin archivo abierto"))?;
        let path_str = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("ruta no UTF-8"))?;
        self.buffer.save(path_str)?;
        self.mark_saved();
        let abs = std::env::current_dir().map_or_else(
            |_| path.display().to_string(),
            |d| d.join(&path).display().to_string(),
        );
        self.notification = Some(format!("guardado con exito en: {abs}"));
        self.notification_expires =
            Some(std::time::Instant::now() + std::time::Duration::from_secs(2));
        Ok(path)
    }

    pub fn open_file(&mut self, path: std::path::PathBuf) -> anyhow::Result<()> {
        let path_str = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("ruta no UTF-8"))?;
        self.buffer = Buffer::from_file(path_str)?;
        self.current_path = Some(path);
        self.mark_saved();
        self.scroll_y = 0;
        self.scroll_x = 0;
        self.notification = None;
        self.notification_expires = None;
        self.touch_cursor();
        Ok(())
    }

    pub fn open_buffer(&mut self, buf: Buffer, path: std::path::PathBuf) {
        self.buffer = buf;
        self.current_path = Some(path);
        self.mark_saved();
        self.scroll_y = 0;
        self.scroll_x = 0;
        self.notification = None;
        self.notification_expires = None;
        self.touch_cursor();
    }

    /// Crea un archivo vacio en `path`. Con la misma guardia que `save`:
    /// rechaza symlinks y destinos no regulares (evita truncar el destino
    /// de un enlace plantado tras el modal de confirmacion) y propaga el
    /// error de creacion en vez de fijar un estado fantasma.
    pub fn new_file(&mut self, path: std::path::PathBuf) -> anyhow::Result<()> {
        match std::fs::symlink_metadata(&path) {
            Ok(m) => {
                let ft = m.file_type();
                if ft.is_symlink() || !ft.is_file() {
                    return Err(anyhow::anyhow!("destino no es un archivo regular"));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        std::fs::write(&path, "")?;
        self.buffer = Buffer::new("");
        self.current_path = Some(path);
        self.mark_saved();
        self.scroll_y = 0;
        self.scroll_x = 0;
        self.touch_cursor();
        Ok(())
    }

    fn edit_insert(&mut self, ch: char) {
        // Auto-cierre: `(`, `[`, `{` insertan su pareja y el cursor
        // queda en el medio. `"` y backtick igual, salvo overtype
        // (si el cierre ya esta a la derecha, se salta).
        // `'` no se empareja: rompería lifetimes (`'a`) y runas.
        const PAIRS: &[(char, char)] = &[('(', ')'), ('[', ']'), ('{', '}')];
        if let Some(&(_, close)) = PAIRS.iter().find(|(o, _)| *o == ch) {
            self.buffer.insert_char(ch);
            self.buffer.insert_char(close);
            self.buffer.move_cursor(-1, 0);
            self.mark_dirty();
            return;
        }
        if matches!(ch, ')' | ']' | '}' | '"' | '\'' | '`')
            && self.buffer.char_after_cursor() == Some(ch)
        {
            self.buffer.move_cursor(1, 0);
            return;
        }
        if ch == '"' || ch == '`' {
            self.buffer.insert_char(ch);
            self.buffer.insert_char(ch);
            self.buffer.move_cursor(-1, 0);
            self.mark_dirty();
            return;
        }
        self.buffer.insert_char(ch);
        self.mark_dirty();
    }

    fn edit_delete(&mut self) {
        // Si el cursor esta entre un par vacio (`(|)`), Backspace
        // borra ambos de una vez.
        const PAIRS: &[(char, char)] = &[
            ('(', ')'),
            ('[', ']'),
            ('{', '}'),
            ('"', '"'),
            ('\'', '\''),
            ('`', '`'),
        ];
        if let (Some(o), Some(c)) = (
            self.buffer.char_before_cursor(),
            self.buffer.char_after_cursor(),
        ) && PAIRS.contains(&(o, c))
        {
            self.buffer.move_cursor(1, 0);
            self.buffer.delete_char();
            self.buffer.delete_char();
            self.mark_dirty();
            return;
        }
        self.buffer.delete_char();
        self.mark_dirty();
    }

    fn edit_newline(&mut self) {
        self.buffer.insert_newline();
        self.mark_dirty();
    }

    fn ensure_visible(&mut self, cursor_y: usize, visible_h: usize) {
        if cursor_y < self.scroll_y {
            self.scroll_y = cursor_y;
        } else if cursor_y >= self.scroll_y + visible_h {
            self.scroll_y = cursor_y - visible_h + 1;
        }
        self.scroll_y = self
            .scroll_y
            .min(self.buffer.lines().len().saturating_sub(1));
    }

    fn ensure_visible_x(&mut self, cx: usize, vis_w: usize) {
        if vis_w == 0 || cx < self.scroll_x {
            self.scroll_x = cx;
        } else if cx >= self.scroll_x + vis_w {
            self.scroll_x = cx - vis_w + 1;
        }
    }

    /// Construye las lineas visibles con gutter, resaltado, sanitizado,
    /// ventana horizontal y cursor propio. Sin allocs salvo lineas con
    /// controles (caso raro, ver `sanitize_owned`).
    fn text_lines(&self, theme: &Theme, ctx: &ViewCtx) -> Vec<Line<'_>> {
        let is_blank = self.buffer.lines().len() == 1 && self.buffer.lines()[0].is_empty();
        self.buffer
            .lines()
            .iter()
            .enumerate()
            .skip(ctx.scroll_y)
            .take(ctx.visible_h)
            .map(|(i, l)| match sanitize_owned(l) {
                None => render_text_line(i, l, theme, ctx, is_blank),
                Some(owned) => own_line(render_text_line(i, &owned, theme, ctx, is_blank)),
            })
            .collect()
    }

    /// Avanza el ciclo de parpadeo suave si paso `BLINK_MS`.
    fn tick_blink(&mut self) {
        if Instant::now().duration_since(self.last_blink).as_millis() >= u128::from(BLINK_MS) {
            self.cursor_visible = !self.cursor_visible;
            self.last_blink = Instant::now();
        }
    }

    pub fn draw(&mut self, f: &mut Frame, area: Rect, focused: bool) {
        let theme = self.theme();
        let title_name = self
            .current_path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map_or_else(|| "Untitled".to_string(), crate::util::sanitize);
        let dirty_mark = if self.is_dirty() { " ●" } else { "" };
        let block = Block::default()
            .title(format!(" {title_name}{dirty_mark} "))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if focused {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.border_dim)
            });
        let inner = block.inner(area);

        let visible_h = inner.height as usize;
        let (_, cursor_y) = self.buffer.cursor();
        self.ensure_visible(cursor_y, visible_h);

        // Parpadeo suave con bloque propio (sin cursor nativo del
        // terminal). Fase apagada = tenue, nunca invisible del todo.
        self.tick_blink();
        let show_block = focused && self.cursor_visible;

        let (cx, _) = self.buffer.cursor();
        // Ancho visible del texto: resta el gutter `"NNN │ "` (6 celdas).
        // El viewport horizontal sigue al cursor al moverse a los costados.
        let vis_w = (inner.width as usize).saturating_sub(6);
        self.ensure_visible_x(cx, vis_w);
        let ctx = ViewCtx {
            cursor_y,
            cx,
            visible_h,
            vis_w,
            scroll_x: self.scroll_x,
            scroll_y: self.scroll_y,
            focused,
            show_block,
            lang: self.lang(),
        };
        let lines = self.text_lines(&theme, &ctx);
        let paragraph = Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);

        if let (Some(msg), Some(expires)) = (&self.notification, &self.notification_expires)
            && Instant::now() < *expires
        {
            draw_notification(f, area, msg, theme.accent);
        }

        // Sin cursor nativo: el bloque dibujado arriba ya marca la posicion.
        // No se llama a `f.set_cursor` para evitar el parpadeo del terminal.
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.kind != KeyEventKind::Press {
            return true;
        }
        // Cada tecla muestra el cursor solido: no parpadea mientras se escribe.
        self.touch_cursor();
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
            if self.current_path.is_none() {
                self.notification = Some("nada que guardar: abri un archivo primero".to_string());
                self.notification_expires = Some(Instant::now() + Duration::from_secs(2));
                return true;
            }
            if let Err(e) = self.save_current() {
                self.notification = Some(format!("falla al guardar: {e}"));
                self.notification_expires = Some(Instant::now() + Duration::from_secs(3));
            }
            return true;
        }
        if let Some(expires) = self.notification_expires
            && Instant::now() >= expires
        {
            self.notification = None;
            self.notification_expires = None;
        }
        match key.code {
            KeyCode::Enter => self.edit_newline(),
            KeyCode::Backspace => self.edit_delete(),
            KeyCode::Char(c) => self.edit_insert(c),
            KeyCode::Left => self.buffer.move_cursor(-1, 0),
            KeyCode::Right => self.buffer.move_cursor(1, 0),
            KeyCode::Up => self.buffer.move_cursor(0, -1),
            KeyCode::Down => self.buffer.move_cursor(0, 1),
            KeyCode::Esc => return false,
            KeyCode::Tab => {
                for _ in 0..4 {
                    self.edit_insert(' ');
                }
            }
            _ => {}
        }
        true
    }
}

/// Byte index del `char_idx`-esimo char (frontera UTF-8 segura).
fn byte_idx(s: &str, ci: usize) -> usize {
    s.char_indices().nth(ci).map_or(s.len(), |(i, _)| i)
}

/// Parametros del viewport para construir las lineas visibles.
struct ViewCtx {
    cursor_y: usize,
    cx: usize,
    visible_h: usize,
    vis_w: usize,
    scroll_x: usize,
    scroll_y: usize,
    focused: bool,
    show_block: bool,
    lang: Lang,
}

/// Sanitizado solo para display con `util::sanitize` (preserva `\t`,
/// resto de controles a U+FFFD, 1:1 en chars). Devuelve `None` si no
/// hay nada que sanitizar (caso comun, sin alloc). El buffer (y lo que
/// se guarda) queda intacto.
fn sanitize_owned(l: &str) -> Option<String> {
    if l.chars().any(|c| c.is_control() && c != '\t') {
        Some(crate::util::sanitize(l))
    } else {
        None
    }
}

/// Construye una linea visible (gutter + texto resaltado + cursor).
/// El texto se presta de `line`.
fn render_text_line<'a>(
    i: usize,
    line: &'a str,
    theme: &Theme,
    ctx: &ViewCtx,
    is_blank: bool,
) -> Line<'a> {
    let num_style = Style::default().fg(theme.border_dim);
    let num_active = Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(theme.text);
    let hint_style = Style::default().fg(theme.border_dim);
    let cursor_on = Style::default()
        .bg(Color::White)
        .fg(Color::Black)
        .add_modifier(Modifier::BOLD);
    let cursor_off = Style::default().bg(Color::DarkGray).fg(Color::Gray);

    let current = i == ctx.cursor_y;
    let gutter = if current { num_active } else { num_style };
    let prefix = Span::styled(format!("{:>3} │ ", i + 1), gutter);
    // Ventana visible en bytes (fronteras UTF-8 seguras).
    let vb0 = byte_idx(line, ctx.scroll_x);
    let vb1 = byte_idx(line, ctx.scroll_x + ctx.vis_w);
    let view = (vb0, vb1);
    if line.is_empty() && is_blank {
        if current && ctx.focused {
            let style = if ctx.show_block {
                cursor_on
            } else {
                cursor_off
            };
            return Line::from(vec![
                prefix,
                Span::styled(" ", style),
                Span::styled(" Start typing…", hint_style),
            ]);
        }
        return Line::from(vec![prefix, Span::styled("Start typing…", hint_style)]);
    }
    if current && ctx.focused {
        let style = if ctx.show_block {
            cursor_on
        } else {
            cursor_off
        };
        let nchars = line.chars().count();
        if ctx.cx >= nchars {
            let mut spans = code_spans_lang(line, theme, text_style, None, view, ctx.lang);
            spans.insert(0, prefix);
            if ctx.vis_w > 0 {
                spans.push(Span::styled(" ", style));
            }
            return Line::from(spans);
        }
        let b0 = byte_idx(line, ctx.cx);
        let b1 = byte_idx(line, ctx.cx + 1);
        let mut spans = code_spans_lang(
            line,
            theme,
            text_style,
            Some((b0, b1, style)),
            view,
            ctx.lang,
        );
        spans.insert(0, prefix);
        return Line::from(spans);
    }
    let mut spans = code_spans_lang(line, theme, text_style, None, view, ctx.lang);
    spans.insert(0, prefix);
    Line::from(spans)
}

/// Transfiere una linea a spans propios (para texto sanitizado local).
fn own_line(line: Line<'_>) -> Line<'static> {
    Line::from(
        line.spans
            .into_iter()
            .map(|sp| Span {
                content: std::borrow::Cow::Owned(sp.content.into_owned()),
                style: sp.style,
            })
            .collect::<Vec<_>>(),
    )
}

fn style_for(kind: Kind, theme: &Theme) -> Style {
    match kind {
        Kind::Keyword => Style::default()
            .fg(theme.keyword)
            .add_modifier(Modifier::BOLD),
        Kind::String => Style::default().fg(theme.string),
        Kind::Number => Style::default().fg(theme.number),
        Kind::Comment => Style::default().fg(theme.comment),
        Kind::Function => Style::default().fg(theme.function),
        Kind::Type => Style::default()
            .fg(theme.type_)
            .add_modifier(Modifier::BOLD),
        Kind::Constant => Style::default()
            .fg(theme.constant)
            .add_modifier(Modifier::BOLD),
        Kind::Lifetime => Style::default().fg(theme.lifetime),
    }
}

/// Emite los `Span`s de una linea con resaltado en Rust.
/// Solo se usa en tests; el render usa `code_spans_lang` con el idioma
/// del archivo.
#[cfg(test)]
fn code_spans<'a>(
    line: &'a str,
    theme: &Theme,
    text_style: Style,
    cursor: Option<(usize, usize, Style)>,
    view: (usize, usize),
) -> Vec<Span<'a>> {
    code_spans_lang(line, theme, text_style, cursor, view, Lang::Rust)
}

/// Emite los `Span`s de una linea con resaltado. `cursor` es el rango de
/// bytes del caracter bajo el cursor con su estilo: lo parte del tramo
/// que lo contenga. `view` recorta a la ventana horizontal visible.
/// Todos los cortes son fronteras UTF-8 (vienen del escaner o de
/// `char_indices`).
fn code_spans_lang<'a>(
    line: &'a str,
    theme: &Theme,
    text_style: Style,
    cursor: Option<(usize, usize, Style)>,
    view: (usize, usize),
    lang: Lang,
) -> Vec<Span<'a>> {
    fn push_range<'a>(
        spans: &mut Vec<Span<'a>>,
        line: &'a str,
        start: usize,
        end: usize,
        style: Style,
        cursor: Option<(usize, usize, Style)>,
    ) {
        if start >= end {
            return;
        }
        match cursor {
            None => spans.push(Span::styled(&line[start..end], style)),
            Some((cb0, cb1, cs)) => {
                if cb1 <= start || cb0 >= end {
                    spans.push(Span::styled(&line[start..end], style));
                } else {
                    if start < cb0 {
                        spans.push(Span::styled(&line[start..cb0], style));
                    }
                    spans.push(Span::styled(&line[cb0.max(start)..cb1.min(end)], cs));
                    if cb1 < end {
                        spans.push(Span::styled(&line[cb1..end], style));
                    }
                }
            }
        }
    }

    let (vs, ve) = view;
    let mut spans = Vec::new();
    let mut pos = vs;
    for t in highlight::highlight_line_lang(line, lang) {
        if t.end <= vs {
            continue;
        }
        if t.start >= ve {
            break;
        }
        let s = t.start.max(vs);
        let e = t.end.min(ve);
        push_range(&mut spans, line, pos, s, text_style, cursor);
        push_range(&mut spans, line, s, e, style_for(t.kind, theme), cursor);
        pos = e;
    }
    push_range(&mut spans, line, pos, ve, text_style, cursor);
    spans
}

fn draw_notification(f: &mut Frame, area: Rect, msg: &str, accent: Color) {
    // Sanitiza antes de medir/dibujar: un \n en el mensaje no debe
    // romper el layout del popup.
    let msg = crate::util::sanitize(msg);
    let popup_width = u16::try_from(msg.chars().count())
        .unwrap_or(u16::MAX)
        .saturating_add(4)
        .min(area.width.saturating_sub(4));
    let popup = centered_rect(popup_width, 3, area);

    f.render_widget(Clear, popup);
    let block = Block::default()
        .title(" Notice ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent));
    f.render_widget(block, popup);

    let inner_popup = Rect {
        x: popup.x + 1,
        y: popup.y + 1,
        width: popup.width.saturating_sub(2),
        height: popup.height.saturating_sub(2),
    };
    let style = if msg.starts_with("falla") {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::White)
    };
    f.render_widget(Paragraph::new(msg.to_string()).style(style), inner_popup);
}

#[cfg(test)]
mod tests {
    use super::{Editor, code_spans, sanitize_owned};
    use ratatui::style::{Color, Style};

    #[test]
    fn test_sanitize_solo_display() {
        // Sin controles (o solo tabs): None, sin alloc.
        assert_eq!(sanitize_owned("hola"), None);
        assert_eq!(sanitize_owned(""), None);
        assert_eq!(sanitize_owned("a\tb"), None);
        // Otros controles -> U+FFFD via util::sanitize, 1:1 en chars.
        let clean = sanitize_owned("a\x07c").unwrap();
        assert_eq!(clean, "a�c");
        assert_eq!(clean.chars().count(), 3);
    }

    #[test]
    fn test_code_spans_mixto_con_cursor() {
        let theme = crate::themes::get(0);
        let line = "fn foo(x: String) -> u32 {";
        let full = (0, line.len());
        let plain = code_spans(line, &theme, Style::default(), None, full);
        assert!(plain.len() > 3);
        // Cursor sobre la 'f' de `foo` (byte 3..4): no paniquea y parte el tramo.
        let cur = Style::default().bg(Color::White);
        let with_cur = code_spans(line, &theme, Style::default(), Some((3, 4, cur)), full);
        assert!(with_cur.len() >= plain.len());
        // Linea con emoji + cursor al final: fronteras validas.
        let uni = "let e = \"😀\";";
        let spans = code_spans(
            uni,
            &theme,
            Style::default(),
            Some((uni.len(), uni.len(), cur)),
            (0, uni.len()),
        );
        assert!(!spans.is_empty());
    }

    #[test]
    fn test_code_spans_ventana_horizontal() {
        let theme = crate::themes::get(0);
        let line = "fn foo(x: String) -> u32 {";
        // Ventana de los primeros 6 bytes: solo `fn foo` parcial.
        let spans = code_spans(line, &theme, Style::default(), None, (0, 6));
        let text: String = spans.iter().map(|s| s.content.to_string()).collect();
        assert_eq!(text, "fn foo");
        // Ventana vacia no paniquea ni emite nada.
        let empty = code_spans(line, &theme, Style::default(), None, (5, 5));
        assert!(empty.is_empty());
    }

    #[test]
    fn test_set_theme_valida_indice() {
        let mut ed = Editor::new();
        assert_eq!(ed.theme_name(), "cobra-dark");
        assert!(ed.set_theme(1));
        assert_eq!(ed.theme_name(), "dracula");
        assert!(!ed.set_theme(999));
        assert_eq!(ed.theme_name(), "dracula");
    }

    #[test]
    fn test_save_sin_archivo_falla_sin_panico() {
        let mut ed = Editor::new();
        assert!(ed.save_current().is_err());
        assert!(!ed.is_dirty());
    }

    #[test]
    fn test_open_inexistente_falla_sin_panico() {
        let mut ed = Editor::new();
        let r = ed.open_file(std::path::PathBuf::from("no_existe_cobra_sec_unit.txt"));
        assert!(r.is_err());
        assert!(ed.current_path.is_none());
    }

    #[test]
    fn test_dirty_tracking() {
        let _guard = crate::OxideEngine::oxide::SAVE_TEST_LOCK.lock().unwrap();
        let path = std::path::PathBuf::from("test_dirty_unit.txt");
        let _ = std::fs::remove_file(&path);
        let mut ed = Editor::new();
        ed.new_file(path.clone()).unwrap();
        assert!(!ed.is_dirty());
        ed.edit_insert('x');
        assert!(ed.is_dirty());
        ed.save_current().unwrap();
        assert!(!ed.is_dirty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_new_file_rechaza_symlink_y_falla_limpio() {
        // Un symlink en la ruta no se trunca: se rechaza y el destino
        // queda intacto. Sin privilegios para symlinks (Windows), se omite.
        let mut dir = std::env::temp_dir();
        dir.push(format!("cobra_newlink_unit_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let real = dir.join("destino.txt");
        std::fs::write(&real, "intacto").unwrap();
        let link = dir.join("enlace.txt");
        let _ = std::fs::remove_file(&link);
        #[cfg(windows)]
        let made = std::os::windows::fs::symlink_file(&real, &link).is_ok();
        #[cfg(not(windows))]
        let made = std::os::unix::fs::symlink(&real, &link).is_ok();
        if !made {
            return;
        }
        let mut ed = Editor::new();
        assert!(ed.new_file(link.clone()).is_err());
        assert!(ed.current_path.is_none(), "sin estado fantasma");
        assert_eq!(std::fs::read_to_string(&real).unwrap(), "intacto");
        let _ = std::fs::remove_file(&link);
        let _ = std::fs::remove_file(&real);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn test_autocierre_pares_y_overtype() {
        let mut ed = Editor::new();
        ed.edit_insert('(');
        assert_eq!(ed.buffer.to_string(), "()");
        assert_eq!(ed.buffer.cursor(), (1, 0));
        // Overtype: el cierre ya esta a la derecha, se salta sin duplicar.
        ed.edit_insert(')');
        assert_eq!(ed.buffer.to_string(), "()");
        assert_eq!(ed.buffer.cursor(), (2, 0));
        // Corchetes y llaves igual.
        let mut ed2 = Editor::new();
        ed2.edit_insert('[');
        ed2.edit_insert('{');
        assert_eq!(ed2.buffer.to_string(), "[{}]");
        assert_eq!(ed2.buffer.cursor(), (2, 0));
    }

    #[test]
    fn test_comillas_par_y_simple() {
        let mut ed = Editor::new();
        ed.edit_insert('"');
        assert_eq!(ed.buffer.to_string(), "\"\"");
        assert_eq!(ed.buffer.cursor(), (1, 0));
        ed.edit_insert('"'); // overtype
        assert_eq!(ed.buffer.to_string(), "\"\"");
        assert_eq!(ed.buffer.cursor(), (2, 0));
        ed.edit_insert('`');
        assert_eq!(ed.buffer.to_string(), "\"\"``");
        // Comilla simple NO se empareja (lifetimes `'a`, runas).
        let mut ed2 = Editor::new();
        for c in "'a".chars() {
            ed2.edit_insert(c);
        }
        assert_eq!(ed2.buffer.to_string(), "'a");
        ed2.edit_insert('\'');
        assert_eq!(ed2.buffer.to_string(), "'a'");
    }

    #[test]
    fn test_backspace_borra_par_vacio() {
        let mut ed = Editor::new();
        ed.edit_insert('(');
        ed.edit_delete();
        assert_eq!(ed.buffer.to_string(), "");
        assert_eq!(ed.buffer.cursor(), (0, 0));
        // Par no vacio: borra normal, de a un char.
        let mut ed2 = Editor::new();
        for c in "(x)".chars() {
            ed2.edit_insert(c);
        }
        assert_eq!(ed2.buffer.to_string(), "(x)");
        ed2.edit_delete();
        assert_eq!(ed2.buffer.to_string(), "(x");
    }
}
