//! A character grid, painted to the client one cell at a time.
//!
//! The grid exists so the reveal can be genuinely progressive: cells are
//! emitted left to right, top to bottom, the way a Minitel filled its screen
//! from a 1200 baud line. Building a styled string instead would make that
//! awkward, because you cannot cut an ANSI escape in half.

pub const BLACK: u8 = 0;
pub const RED: u8 = 1;
pub const GREEN: u8 = 2;
pub const YELLOW: u8 = 3;
pub const BLUE: u8 = 4;
pub const MAGENTA: u8 = 5;
pub const CYAN: u8 = 6;
pub const WHITE: u8 = 7;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Style {
    pub fg: u8,
    pub bg: u8,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl Default for Style {
    fn default() -> Self {
        Style { fg: WHITE, bg: BLACK, bold: false, italic: false, underline: false }
    }
}

impl Style {
    pub fn fg(fg: u8) -> Self {
        Style { fg, ..Default::default() }
    }
    pub fn bold(fg: u8) -> Self {
        Style { fg, bold: true, ..Default::default() }
    }
    pub fn italic(fg: u8) -> Self {
        Style { fg, italic: true, ..Default::default() }
    }
    pub fn link(fg: u8) -> Self {
        Style { fg, underline: true, ..Default::default() }
    }
    pub fn bar(fg: u8, bg: u8) -> Self {
        Style { fg, bg, ..Default::default() }
    }
}

#[derive(Clone, Copy)]
pub struct Cell {
    pub ch: char,
    pub style: Style,
}

impl Default for Cell {
    fn default() -> Self {
        Cell { ch: ' ', style: Style::default() }
    }
}

pub struct Screen {
    pub w: usize,
    pub h: usize,
    pub cells: Vec<Cell>,
}

impl Screen {
    pub fn new(w: usize, h: usize) -> Self {
        Screen { w, h, cells: vec![Cell::default(); w * h] }
    }

    /// Nothing painted here is ever a control character, so anything that
    /// looks like one is text from somewhere else wearing a disguise: a
    /// README, a feed, a name a visitor chose. Left alone it would reach the
    /// reader's terminal as an escape sequence rather than as writing, which
    /// is a way to set their title, rewrite the screen, or load their
    /// clipboard for the next time they paste. It gets a visible mark instead.
    pub fn put(&mut self, x: usize, y: usize, ch: char, style: Style) {
        if x < self.w && y < self.h {
            let ch = if ch.is_control() { '\u{fffd}' } else { ch };
            self.cells[y * self.w + x] = Cell { ch, style };
        }
    }

    /// Writes `s` at (x, y), clipped to the screen. Returns the column after
    /// the last character written, so callers can chain styled runs on a line.
    pub fn text(&mut self, x: usize, y: usize, s: &str, style: Style) -> usize {
        let mut cx = x;
        for ch in s.chars() {
            if cx >= self.w {
                break;
            }
            self.put(cx, y, ch, style);
            cx += 1;
        }
        cx
    }

    pub fn fill_row(&mut self, y: usize, style: Style) {
        for x in 0..self.w {
            self.put(x, y, ' ', style);
        }
    }

    /// Centre `s` on row `y`.
    pub fn center(&mut self, y: usize, s: &str, style: Style) {
        let len = s.chars().count();
        let x = self.w.saturating_sub(len) / 2;
        self.text(x, y, s, style);
    }

    /// Serialise the whole grid into positioned, styled chunks.
    ///
    /// One chunk per cell would be correct but hopelessly chatty, so runs of
    /// identical style on the same row are merged. The reveal still paces
    /// itself per character, by splitting these runs again as it writes.
    pub fn runs(&self) -> Vec<(usize, usize, Style, String)> {
        let mut out = Vec::new();
        for y in 0..self.h {
            let mut x = 0;
            while x < self.w {
                let style = self.cells[y * self.w + x].style;
                let mut s = String::new();
                let start = x;
                while x < self.w && self.cells[y * self.w + x].style == style {
                    s.push(self.cells[y * self.w + x].ch);
                    x += 1;
                }
                // Every cell is emitted, trailing blanks included, so the
                // background is painted edge to edge. Skipping them would leave
                // the terminal's own background showing through, which looks
                // like a half-drawn screen on anything but a black theme — and
                // a Minitel filled its whole screen.
                out.push((start, y, style, s));
            }
        }
        out
    }
}

pub fn sgr(style: Style) -> String {
    let mut s = String::from("\x1b[0");
    if style.bold {
        s.push_str(";1");
    }
    if style.italic {
        s.push_str(";3");
    }
    if style.underline {
        s.push_str(";4");
    }
    s.push_str(&format!(";{}", 30 + style.fg));
    s.push_str(&format!(";{}", 40 + style.bg));
    s.push('m');
    s
}

pub fn goto(x: usize, y: usize) -> String {
    format!("\x1b[{};{}H", y + 1, x + 1)
}

pub const CLEAR: &str = "\x1b[2J\x1b[H";
pub const HIDE_CURSOR: &str = "\x1b[?25l";
pub const SHOW_CURSOR: &str = "\x1b[?25h";
pub const RESET: &str = "\x1b[0m";

// ── Themes ───────────────────────────────────────────────────────────────────

/// Box-drawing characters: h, v, tl, tr, bl, br, tee-left, tee-right,
/// tee-down, tee-up.
pub struct Border {
    pub h: char,
    pub v: char,
    pub tl: char,
    pub tr: char,
    pub bl: char,
    pub br: char,
    pub tl_t: char,
    pub tr_t: char,
    pub td: char,
    pub tu: char,
}

pub const SINGLE: Border = Border {
    h: '─', v: '│', tl: '┌', tr: '┐', bl: '└', br: '┘',
    tl_t: '├', tr_t: '┤', td: '┬', tu: '┴',
};

/// The DOS look: double bars, as every text-mode utility drew them.
pub const DOUBLE: Border = Border {
    h: '═', v: '║', tl: '╔', tr: '╗', bl: '╚', br: '╝',
    tl_t: '╠', tr_t: '╣', td: '╦', tu: '╩',
};

#[derive(Clone, Copy, PartialEq)]
pub enum ThemeKind {
    Dark,
    Dos,
}

pub struct Theme {
    pub kind: ThemeKind,
    pub bg: u8,
    pub fg: u8,
    pub dim: u8,
    pub accent: u8,
    pub heading: u8,
    pub code: u8,
    pub link: u8,
    pub bar_fg: u8,
    pub bar_bg: u8,
    pub sel_fg: u8,
    pub sel_bg: u8,
    pub border: &'static Border,
}

impl Theme {
    pub fn get(kind: ThemeKind) -> Theme {
        match kind {
            ThemeKind::Dark => Theme {
                kind, bg: BLACK, fg: WHITE, dim: BLUE, accent: CYAN,
                heading: YELLOW, code: GREEN, link: MAGENTA,
                bar_fg: BLACK, bar_bg: CYAN, sel_fg: BLACK, sel_bg: YELLOW,
                border: &SINGLE,
            },
            // White here is the terminal's light grey, which is exactly the
            // DOS text-mode background everyone remembers.
            ThemeKind::Dos => Theme {
                kind, bg: WHITE, fg: BLACK, dim: BLUE, accent: BLUE,
                heading: RED, code: MAGENTA, link: BLUE,
                bar_fg: WHITE, bar_bg: BLUE, sel_fg: WHITE, sel_bg: BLUE,
                border: &DOUBLE,
            },
        }
    }

    pub fn base(&self) -> Style {
        Style { fg: self.fg, bg: self.bg, ..Default::default() }
    }
    pub fn on(&self, fg: u8) -> Style {
        Style { fg, bg: self.bg, ..Default::default() }
    }
    pub fn strong(&self, fg: u8) -> Style {
        Style { fg, bg: self.bg, bold: true, ..Default::default() }
    }
    pub fn bar(&self) -> Style {
        Style { fg: self.bar_fg, bg: self.bar_bg, bold: true, ..Default::default() }
    }
    pub fn sel(&self) -> Style {
        Style { fg: self.sel_fg, bg: self.sel_bg, bold: true, ..Default::default() }
    }
}

impl Screen {
    pub fn clear_to(&mut self, style: Style) {
        for c in self.cells.iter_mut() {
            *c = Cell { ch: ' ', style };
        }
    }

    /// Frame with an optional title in the top edge.
    pub fn frame(&mut self, x: usize, y: usize, w: usize, h: usize, b: &Border, style: Style, title: Option<(&str, Style)>) {
        if w < 2 || h < 2 {
            return;
        }
        let (x2, y2) = (x + w - 1, y + h - 1);
        self.put(x, y, b.tl, style);
        self.put(x2, y, b.tr, style);
        self.put(x, y2, b.bl, style);
        self.put(x2, y2, b.br, style);
        for i in x + 1..x2 {
            self.put(i, y, b.h, style);
            self.put(i, y2, b.h, style);
        }
        for j in y + 1..y2 {
            self.put(x, j, b.v, style);
            self.put(x2, j, b.v, style);
        }
        if let Some((t, ts)) = title {
            let t = format!(" {t} ");
            if t.chars().count() + 2 < w {
                self.text(x + 2, y, &t, ts);
            }
        }
    }
}
