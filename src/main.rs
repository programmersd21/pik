use crossterm::{
    cursor,
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
        MouseButton, MouseEvent,
    },
    execute, queue,
    style::{self, Color},
    terminal::{self, ClearType},
};
use rand::{thread_rng, Rng};
use std::io::{self, BufRead, IsTerminal, Write};
use std::time::{Duration, Instant};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const EXIT_SUCCESS: i32 = 0;
const EXIT_CANCELLED: i32 = 130;
const EXIT_ERROR: i32 = 1;

const MIN_BOX_W: usize = 40;
const BOX_MARGIN: usize = 4;
const MIN_TERM_COLS: usize = MIN_BOX_W + BOX_MARGIN;
const MIN_TERM_ROWS: usize = 8;

// Color System

#[derive(Clone, Copy, PartialEq, Eq)]
enum ColorTier {
    TrueColor,
    Color256,
    Ansi,
    None,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct CellStyle {
    fg: Option<Color>,
    bold: bool,
    dim: bool,
    reverse: bool,
}

impl CellStyle {
    const fn plain() -> Self {
        Self {
            fg: None,
            bold: false,
            dim: false,
            reverse: false,
        }
    }
}

fn detect_color_tier() -> ColorTier {
    if std::env::var("NO_COLOR").is_ok() {
        return ColorTier::None;
    }
    if !io::stdout().is_terminal() {
        return ColorTier::None;
    }
    if let Ok(v) = std::env::var("COLORTERM") {
        if v.contains("truecolor") || v.contains("24bit") {
            return ColorTier::TrueColor;
        }
    }
    if let Ok(v) = std::env::var("TERM") {
        if v.contains("256") {
            return ColorTier::Color256;
        }
    }
    ColorTier::Ansi
}

fn pick_accent(tier: ColorTier) -> Color {
    let mut rng = thread_rng();
    match tier {
        ColorTier::TrueColor => {
            const PALETTE: [(u8, u8, u8); 8] = [
                (137, 180, 250), // blue
                (203, 166, 247), // mauve
                (148, 226, 213), // teal
                (166, 227, 161), // green
                (250, 179, 135), // peach
                (243, 139, 168), // rose
                (137, 220, 235), // sky
                (245, 194, 231), // pink
            ];
            let (r, g, b) = PALETTE[rng.gen_range(0..PALETTE.len())];
            Color::Rgb { r, g, b }
        }
        ColorTier::Color256 => {
            let indices = [111, 141, 116, 151, 180, 175, 117, 183];
            Color::AnsiValue(indices[rng.gen_range(0..indices.len())])
        }
        ColorTier::Ansi => {
            let colors = [Color::Cyan, Color::Magenta, Color::Blue, Color::Green];
            colors[rng.gen_range(0..colors.len())]
        }
        ColorTier::None => Color::Reset,
    }
}

// Screen Buffer

#[derive(Clone, PartialEq, Eq, Debug)]
struct Cell {
    ch: char,
    style: CellStyle,
}

struct ScreenBuffer {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
}

impl ScreenBuffer {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![
                Cell {
                    ch: ' ',
                    style: CellStyle::plain()
                };
                width * height
            ],
        }
    }

    fn set(&mut self, col: usize, row: usize, ch: char, style: CellStyle) {
        if row < self.height && col < self.width {
            self.cells[row * self.width + col] = Cell { ch, style };
        }
    }

    fn put(&mut self, col: usize, row: usize, s: &str, style: CellStyle) {
        let mut x = col;
        for ch in s.chars() {
            let w = ch.width().unwrap_or(0);
            if w == 0 {
                continue;
            }
            if x >= self.width {
                break;
            }
            self.set(x, row, ch, style);
            x += 1;
            for _ in 1..w {
                if x >= self.width {
                    break;
                }
                self.set(x, row, '\0', style);
                x += 1;
            }
        }
    }

    fn fill(&mut self, col: usize, row: usize, ch: char, len: usize, style: CellStyle) {
        for i in 0..len {
            self.set(col + i, row, ch, style);
        }
    }
}

fn flush_diff(out: &mut impl Write, old: &ScreenBuffer, new: &ScreenBuffer) -> io::Result<()> {
    let mut cur_style = CellStyle::plain();
    apply_style(out, &cur_style)?;

    for r in 0..new.height {
        let mut c = 0;
        while c < new.width {
            let idx = r * new.width + c;
            if idx >= old.cells.len() || new.cells[idx] != old.cells[idx] {
                queue!(out, cursor::MoveTo(c as u16, r as u16))?;
                while c < new.width {
                    let i = r * new.width + c;
                    if i < old.cells.len() && new.cells[i] == old.cells[i] {
                        break;
                    }
                    let cell = &new.cells[i];
                    if cell.style != cur_style {
                        apply_style(out, &cell.style)?;
                        cur_style = cell.style;
                    }
                    if cell.ch != '\0' {
                        queue!(out, style::Print(cell.ch))?;
                    }
                    c += 1;
                }
            } else {
                c += 1;
            }
        }
    }
    apply_style(out, &CellStyle::plain())?;
    Ok(())
}

fn apply_style(out: &mut impl Write, s: &CellStyle) -> io::Result<()> {
    queue!(
        out,
        style::ResetColor,
        style::SetAttribute(style::Attribute::Reset)
    )?;
    if let Some(fg) = s.fg {
        queue!(out, style::SetForegroundColor(fg))?;
    }
    if s.bold {
        queue!(out, style::SetAttribute(style::Attribute::Bold))?;
    }
    if s.dim {
        queue!(out, style::SetAttribute(style::Attribute::Dim))?;
    }
    if s.reverse {
        queue!(out, style::SetAttribute(style::Attribute::Reverse))?;
    }
    Ok(())
}

// Text Helpers

fn truncate_to_width(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let w = UnicodeWidthStr::width(s);
    if w <= max {
        return s.to_owned();
    }
    let target = max.saturating_sub(1);
    let mut used = 0;
    let mut out = String::new();
    for ch in s.chars() {
        let cw = ch.width().unwrap_or(0);
        if used + cw > target {
            break;
        }
        used += cw;
        out.push(ch);
    }
    out.push('~');
    out
}

// Picker

struct Config {
    prompt: Option<String>,
    file: Option<String>,
}

struct LayoutInfo {
    box_x: usize,
    box_y: usize,
    box_w: usize,
    box_h: usize,
    vis: usize,
}

struct Picker {
    items: Vec<String>,
    cursor: usize,
    viewport_top: usize,
    config: Config,
    startup: Instant,
    use_color: bool,
    accent: Color,
    content_w: usize,
    last_size: Option<(u16, u16)>,
    prev_buf: ScreenBuffer,
    last_click: Option<(Instant, u16, u16)>,
}

impl Picker {
    fn new(items: Vec<String>, config: Config) -> Self {
        let tier = detect_color_tier();
        let use_color = !matches!(tier, ColorTier::None);
        let accent = pick_accent(tier);
        let content_w = items
            .iter()
            .map(|s| UnicodeWidthStr::width(s.as_str()))
            .max()
            .unwrap_or(0)
            .max(
                config
                    .prompt
                    .as_deref()
                    .map(UnicodeWidthStr::width)
                    .unwrap_or(0),
            );
        Self {
            items,
            cursor: 0,
            viewport_top: 0,
            config,
            startup: Instant::now(),
            use_color,
            accent,
            content_w,
            last_size: None,
            prev_buf: ScreenBuffer::new(0, 0),
            last_click: None,
        }
    }

    fn selected(&self) -> Option<&str> {
        self.items.get(self.cursor).map(String::as_str)
    }

    fn move_cursor(&mut self, new: usize) {
        self.cursor = new;
    }

    fn move_up(&mut self) {
        if self.cursor > 0 {
            self.move_cursor(self.cursor - 1);
        }
    }

    fn move_down(&mut self) {
        if self.cursor + 1 < self.items.len() {
            self.move_cursor(self.cursor + 1);
        }
    }

    fn jump_first(&mut self) {
        self.move_cursor(0);
    }

    fn jump_last(&mut self) {
        self.move_cursor(self.items.len().saturating_sub(1));
    }

    fn page_up(&mut self, n: usize) {
        self.move_cursor(self.cursor.saturating_sub(n));
    }

    fn page_down(&mut self, n: usize) {
        self.move_cursor((self.cursor + n).min(self.items.len().saturating_sub(1)));
    }

    fn scroll(&mut self, delta: isize) {
        let new = if delta < 0 {
            self.cursor.saturating_sub((-delta) as usize)
        } else {
            (self.cursor + delta as usize).min(self.items.len().saturating_sub(1))
        };
        self.move_cursor(new);
    }

    fn cut_animation(&mut self) {
        self.startup = Instant::now() - Duration::from_secs(1);
    }

    // clamp bounds are safe: max_box_w >= MIN_BOX_W regardless of terminal width
    fn compute_layout(&self, cols: usize, rows: usize) -> LayoutInfo {
        let max_box_w = cols.saturating_sub(BOX_MARGIN).max(MIN_BOX_W);
        let box_w = (self.content_w + 10).clamp(MIN_BOX_W, max_box_w);
        let box_x = (cols.saturating_sub(box_w)) / 2;
        let vis = rows.saturating_sub(6).min(self.items.len()).max(1);
        let box_h = vis + 2;
        let box_y = (rows.saturating_sub(box_h + 2)) / 2;
        LayoutInfo {
            box_x,
            box_y,
            box_w,
            box_h,
            vis,
        }
    }

    fn handle_mouse_click(&self, col: u16, row: u16, tcols: u16, trows: u16) -> Option<usize> {
        let cols = tcols as usize;
        let rows = trows as usize;
        if cols < MIN_TERM_COLS || rows < MIN_TERM_ROWS {
            return None;
        }
        let layout = self.compute_layout(cols, rows);
        let r = row as usize;
        let c = col as usize;
        if c < layout.box_x || c >= layout.box_x + layout.box_w {
            return None;
        }
        let item_start = layout.box_y + 1;
        let item_end = layout.box_y + layout.box_h - 1;
        if !(item_start..item_end).contains(&r) {
            return None;
        }
        let idx = self.viewport_top + (r - item_start);
        if idx >= self.items.len() {
            return None;
        }
        Some(idx)
    }

    fn startup_progress(&self) -> f32 {
        if !self.use_color {
            return 1.0;
        }
        (self.startup.elapsed().as_millis() as f32 / 120.0).min(1.0)
    }

    fn is_animating(&self) -> bool {
        self.startup_progress() < 1.0
    }

    fn render(&mut self, stdout: &mut impl Write) -> io::Result<()> {
        let (tcols, trows) = terminal::size()?;
        let cols = tcols as usize;
        let rows = trows as usize;

        let mut cmd = Vec::new();
        if self.last_size != Some((tcols, trows)) {
            queue!(&mut cmd, terminal::Clear(ClearType::All))?;
            self.last_size = Some((tcols, trows));
            self.prev_buf = ScreenBuffer::new(cols, rows);
        }

        let mut buf = ScreenBuffer::new(cols, rows);
        if cols < MIN_TERM_COLS || rows < MIN_TERM_ROWS {
            let msg = "terminal too small - resize to continue";
            let mw = UnicodeWidthStr::width(msg);
            let mx = cols.saturating_sub(mw) / 2;
            let my = rows / 2;
            buf.put(
                mx,
                my,
                msg,
                CellStyle {
                    dim: true,
                    ..CellStyle::plain()
                },
            );
            return self.finalize(&mut cmd, buf, stdout);
        }

        let sp = self.startup_progress();
        let accent_dim = CellStyle {
            fg: if self.use_color {
                Some(self.accent)
            } else {
                None
            },
            dim: true,
            ..CellStyle::plain()
        };
        let ts = CellStyle {
            fg: if self.use_color {
                Some(Color::White)
            } else {
                None
            },
            bold: true,
            dim: sp < 0.3,
            ..CellStyle::plain()
        };
        let row_style = CellStyle {
            fg: if self.use_color {
                Some(self.accent)
            } else {
                None
            },
            bold: true,
            reverse: true,
            ..CellStyle::plain()
        };

        let layout = self.compute_layout(cols, rows);
        let box_x = layout.box_x;
        let box_y = layout.box_y;
        let box_w = layout.box_w;
        let box_h = layout.box_h;
        let vis = layout.vis;

        if self.cursor < self.viewport_top {
            self.viewport_top = self.cursor;
        } else if self.cursor >= self.viewport_top + vis {
            self.viewport_top = self.cursor - vis + 1;
        }
        let slice_end = (self.viewport_top + vis).min(self.items.len());

        // Draw top border
        {
            let mut x = box_x;
            buf.put(x, box_y, "╭", accent_dim);
            x += 1;
            if let Some(ref prompt) = self.config.prompt {
                buf.put(x, box_y, "─ ", accent_dim);
                x += 2;
                buf.put(x, box_y, prompt, ts);
                x += UnicodeWidthStr::width(prompt.as_str());
                buf.put(x, box_y, " ", accent_dim);
                x += 1;
            }
            let remaining = (box_x + box_w - 1).saturating_sub(x);
            buf.fill(x, box_y, '─', remaining, accent_dim);
            buf.put(box_x + box_w - 1, box_y, "╮", accent_dim);
        }

        // Draw items
        let inner_w = box_w.saturating_sub(2);
        let text_max = inner_w.saturating_sub(4);
        for (slot, item) in self.items[self.viewport_top..slice_end].iter().enumerate() {
            let row_idx = self.viewport_top + slot;
            let is_sel = row_idx == self.cursor;
            let iy = box_y + 1 + slot;
            buf.put(box_x, iy, "│", accent_dim);
            buf.put(box_x + box_w - 1, iy, "│", accent_dim);

            let text = truncate_to_width(item, text_max);
            if is_sel {
                buf.fill(box_x + 1, iy, ' ', inner_w, row_style);
                buf.put(box_x + 2, iy, ">", row_style);
                buf.put(box_x + 4, iy, &text, row_style);
            } else {
                let dim_style = CellStyle {
                    dim: sp < 0.9,
                    ..CellStyle::plain()
                };
                buf.put(box_x + 4, iy, &text, dim_style);
            }
        }

        // Draw bottom border
        {
            let counter = format!(" {}/{} ", self.cursor + 1, self.items.len());
            let counter_w = UnicodeWidthStr::width(counter.as_str());
            let by = box_y + box_h - 1;
            buf.put(box_x, by, "╰", accent_dim);
            let total_dashes = box_w.saturating_sub(counter_w + 2);
            let left = total_dashes * 4 / 5;
            buf.fill(box_x + 1, by, '─', left, accent_dim);
            buf.put(
                box_x + 1 + left,
                by,
                &counter,
                CellStyle {
                    dim: true,
                    ..CellStyle::plain()
                },
            );
            buf.fill(
                box_x + 1 + left + counter_w,
                by,
                '─',
                total_dashes - left,
                accent_dim,
            );
            buf.put(box_x + box_w - 1, by, "╯", accent_dim);
        }

        // Help bar - width is measured dynamically so it stays centered
        // regardless of hints content.
        {
            let hy = box_y + box_h + 1;
            let hints = [
                ("↵", "select"),
                ("↑↓", "move"),
                ("g/G", "jump"),
                ("q", "quit"),
            ];
            let sep = " | ";
            let sep_w = UnicodeWidthStr::width(sep);
            let mut total_w = 0usize;
            for (i, (k, l)) in hints.iter().enumerate() {
                if i > 0 {
                    total_w += sep_w;
                }
                total_w += UnicodeWidthStr::width(*k) + 1 + UnicodeWidthStr::width(*l);
            }
            let mut hx = box_x + (box_w.saturating_sub(total_w)) / 2;
            for (i, (k, l)) in hints.iter().enumerate() {
                if i > 0 {
                    buf.put(hx, hy, sep, accent_dim);
                    hx += sep_w;
                }
                buf.put(hx, hy, k, ts);
                hx += UnicodeWidthStr::width(*k) + 1;
                buf.put(
                    hx,
                    hy,
                    l,
                    CellStyle {
                        dim: true,
                        ..CellStyle::plain()
                    },
                );
                hx += UnicodeWidthStr::width(*l);
            }
        }

        self.finalize(&mut cmd, buf, stdout)
    }

    fn finalize(
        &mut self,
        cmd: &mut Vec<u8>,
        buf: ScreenBuffer,
        stdout: &mut impl Write,
    ) -> io::Result<()> {
        queue!(cmd, style::Print("\x1b[?2026h"))?;
        flush_diff(cmd, &self.prev_buf, &buf)?;
        queue!(cmd, style::Print("\x1b[?2026l"))?;
        self.prev_buf = buf;
        stdout.write_all(cmd)?;
        stdout.flush()
    }
}

// Main Loop

fn run_picker(items: Vec<String>, config: Config) -> io::Result<Option<String>> {
    let mut picker = Picker::new(items, config);
    let _guard = TerminalGuard::new()?;
    let mut stdout = io::stdout();

    picker.render(&mut stdout)?;
    stdout.flush()?;

    std::thread::sleep(Duration::from_millis(50));
    while event::poll(Duration::from_millis(0))? {
        let _ = event::read()?;
    }

    let _mouse_guard = MouseGuard::enable(&mut stdout)?;

    std::thread::sleep(Duration::from_millis(50));
    while event::poll(Duration::from_millis(0))? {
        let _ = event::read()?;
    }

    let mut last_move = Instant::now();

    loop {
        let timeout = if picker.is_animating() {
            Duration::from_millis(16)
        } else {
            Duration::from_secs(86400)
        };

        if event::poll(timeout)? {
            let ev = event::read()?;
            picker.cut_animation();

            match ev {
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    ..
                })
                | Event::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    ..
                })
                | Event::Key(KeyEvent {
                    code: KeyCode::Esc, ..
                }) => return Ok(None),

                Event::Key(KeyEvent {
                    code: KeyCode::Up, ..
                })
                | Event::Key(KeyEvent {
                    code: KeyCode::Char('k'),
                    ..
                }) if last_move.elapsed() >= Duration::from_millis(30) => {
                    picker.move_up();
                    last_move = Instant::now();
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Down,
                    ..
                })
                | Event::Key(KeyEvent {
                    code: KeyCode::Char('j'),
                    ..
                }) if last_move.elapsed() >= Duration::from_millis(30) => {
                    picker.move_down();
                    last_move = Instant::now();
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('g'),
                    ..
                })
                | Event::Key(KeyEvent {
                    code: KeyCode::Home,
                    ..
                }) => picker.jump_first(),
                Event::Key(KeyEvent {
                    code: KeyCode::Char('G'),
                    ..
                })
                | Event::Key(KeyEvent {
                    code: KeyCode::End, ..
                }) => picker.jump_last(),
                Event::Key(KeyEvent {
                    code: KeyCode::PageUp,
                    ..
                })
                | Event::Key(KeyEvent {
                    code: KeyCode::Char('u'),
                    modifiers: KeyModifiers::CONTROL,
                    ..
                }) => {
                    let size = terminal::size()?.1 as usize;
                    picker.page_up(size);
                }
                Event::Key(KeyEvent {
                    code: KeyCode::PageDown,
                    ..
                })
                | Event::Key(KeyEvent {
                    code: KeyCode::Char('d'),
                    modifiers: KeyModifiers::CONTROL,
                    ..
                }) => {
                    let size = terminal::size()?.1 as usize;
                    picker.page_down(size);
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    ..
                }) => return Ok(picker.selected().map(|s| s.to_string())),

                Event::Mouse(MouseEvent {
                    column,
                    row,
                    kind: event::MouseEventKind::Down(MouseButton::Left),
                    ..
                }) => {
                    let (tcols, trows) = terminal::size()?;
                    if let Some(idx) = picker.handle_mouse_click(column, row, tcols, trows) {
                        let is_double = picker.last_click.is_some_and(|(t, c, r)| {
                            t.elapsed() < Duration::from_millis(300) && c == column && r == row
                        });
                        picker.last_click = Some((Instant::now(), column, row));
                        if is_double {
                            return Ok(picker.selected().map(|s| s.to_string()));
                        }
                        picker.move_cursor(idx);
                    }
                }
                Event::Mouse(MouseEvent {
                    kind: event::MouseEventKind::ScrollUp,
                    ..
                }) => picker.scroll(-1),
                Event::Mouse(MouseEvent {
                    kind: event::MouseEventKind::ScrollDown,
                    ..
                }) => picker.scroll(1),

                Event::Resize(_, _) => picker.last_size = None,
                _ => {}
            }
            picker.render(&mut stdout)?;
        } else if picker.is_animating() {
            picker.render(&mut stdout)?;
        }
    }
}

// Entry

fn main() {
    install_panic_hook();
    let config = match parse_args() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(EXIT_ERROR);
        }
    };
    let items = match read_input_lines(&config) {
        Ok(i) if !i.is_empty() => i,
        Ok(_) => {
            print_usage();
            std::process::exit(EXIT_ERROR);
        }
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(EXIT_ERROR);
        }
    };
    match run_picker(items, config) {
        Ok(Some(s)) => {
            println!("{}", s);
            std::process::exit(EXIT_SUCCESS);
        }
        Ok(None) => std::process::exit(EXIT_CANCELLED),
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(EXIT_ERROR);
        }
    }
}

fn read_stdin_lines() -> io::Result<Vec<String>> {
    let stdin = io::stdin();
    if stdin.is_terminal() {
        return Ok(vec![]);
    }
    stdin.lock().lines().collect()
}

fn read_file_lines(path: &str) -> io::Result<Vec<String>> {
    let f = std::fs::File::open(path)?;
    io::BufReader::new(f).lines().collect()
}

fn read_input_lines(config: &Config) -> io::Result<Vec<String>> {
    if let Some(ref p) = config.file {
        read_file_lines(p)
    } else {
        read_stdin_lines()
    }
}

fn parse_args() -> Result<Config, String> {
    let mut args = std::env::args().skip(1);
    let mut prompt = None;
    let mut file = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_usage();
                std::process::exit(EXIT_SUCCESS);
            }
            "-V" | "--version" => {
                println!("pik {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(EXIT_SUCCESS);
            }
            "-p" | "--prompt" => prompt = Some(args.next().ok_or("missing prompt")?),
            "-f" | "--file" => file = Some(args.next().ok_or("missing file path")?),
            _ => return Err(format!("unknown arg: {}", arg)),
        }
    }
    Ok(Config { prompt, file })
}

fn print_usage() {
    let v = env!("CARGO_PKG_VERSION");
    eprintln!("pik v{}\nMinimal interactive line picker\n\nusage:\n  pik [options]\n\nOptions:\n  -p, --prompt <txt>  Header text\n  -f, --file <path>   Read from file\n  -V, --version       Show version\n  -h, --help          Show help", v);
}

fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(io::stdout(), cursor::Show, terminal::LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
        prev(info);
    }));
}

struct TerminalGuard {
    active: bool,
}

struct MouseGuard;

impl MouseGuard {
    fn enable(stdout: &mut impl Write) -> io::Result<Self> {
        execute!(stdout, EnableMouseCapture)?;
        stdout.flush()?;
        Ok(Self)
    }
}

impl Drop for MouseGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), DisableMouseCapture);
    }
}

impl TerminalGuard {
    fn new() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(io::stdout(), terminal::EnterAlternateScreen, cursor::Hide)?;
        Ok(Self { active: true })
    }
    fn cleanup(&mut self) -> io::Result<()> {
        if self.active {
            execute!(io::stdout(), cursor::Show, terminal::LeaveAlternateScreen)?;
            terminal::disable_raw_mode()?;
            self.active = false;
        }
        Ok(())
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        assert_eq!(truncate_to_width("hello", 10), "hello");
        assert_eq!(truncate_to_width("hello world", 7), "hello ~");
    }

    #[test]
    fn layout_never_panics_across_widths() {
        let items: Vec<String> = vec!["alpha".into(), "beta".into(), "gamma".into()];
        let config = Config {
            prompt: Some("pick one".into()),
            file: None,
        };
        let picker = Picker::new(items, config);
        for cols in 0..(MIN_TERM_COLS + 20) {
            for rows in [MIN_TERM_ROWS, MIN_TERM_ROWS + 5, 3] {
                let _ = picker.compute_layout(cols, rows);
            }
        }
    }
}
