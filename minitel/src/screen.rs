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
}

impl Default for Style {
    fn default() -> Self {
        Style { fg: WHITE, bg: BLACK, bold: false }
    }
}

impl Style {
    pub fn fg(fg: u8) -> Self {
        Style { fg, ..Default::default() }
    }
    pub fn bold(fg: u8) -> Self {
        Style { fg, bg: BLACK, bold: true }
    }
    pub fn bar(fg: u8, bg: u8) -> Self {
        Style { fg, bg, bold: false }
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

    pub fn put(&mut self, x: usize, y: usize, ch: char, style: Style) {
        if x < self.w && y < self.h {
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
