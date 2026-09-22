//! Pages, navigation and the Minitel chrome.

use std::sync::Arc;

use crate::feed::{post_text, Feed};
use crate::screen::*;

#[derive(Clone, PartialEq)]
pub enum Page {
    Sommaire,
    Articles,
    Article(usize),
    Projets,
    Apropos,
}

pub struct App {
    pub feed: Arc<Feed>,
    pub page: Page,
    pub stack: Vec<Page>,
    pub sel: usize,
    pub scroll: usize,
    /// Lines of the article being read, wrapped to the current width.
    pub body: Vec<String>,
    pub fast: bool,
    pub quit: bool,
    pub w: usize,
    pub h: usize,
}

impl App {
    pub fn new(feed: Arc<Feed>, w: usize, h: usize) -> Self {
        App {
            feed,
            page: Page::Sommaire,
            stack: Vec::new(),
            sel: 0,
            scroll: 0,
            body: Vec::new(),
            fast: false,
            quit: false,
            w,
            h,
        }
    }

    fn inner_w(&self) -> usize {
        self.w.saturating_sub(4).max(20)
    }

    /// Rows available between the header and the footer.
    fn body_h(&self) -> usize {
        self.h.saturating_sub(6).max(3)
    }

    fn menu_len(&self) -> usize {
        match self.page {
            Page::Sommaire => 4,
            Page::Articles => self.feed.posts.len(),
            Page::Projets => self.feed.projects.len(),
            _ => 0,
        }
    }

    fn goto(&mut self, page: Page) {
        self.stack.push(self.page.clone());
        self.page = page;
        self.sel = 0;
        self.scroll = 0;
    }

    pub fn back(&mut self) {
        if let Some(p) = self.stack.pop() {
            self.page = p;
            self.sel = 0;
            self.scroll = 0;
        }
    }

    fn open_selected(&mut self) {
        match self.page {
            Page::Sommaire => match self.sel {
                0 => self.goto(Page::Articles),
                1 => self.goto(Page::Projets),
                2 => self.goto(Page::Apropos),
                _ => self.quit = true,
            },
            Page::Articles => {
                if self.sel < self.feed.posts.len() {
                    let idx = self.sel;
                    let w = self.inner_w();
                    self.body = post_text(&self.feed.posts[idx].html, w);
                    self.goto(Page::Article(idx));
                }
            }
            _ => {}
        }
    }

    /// Feeds one decoded key to the state machine. Returns true if the screen
    /// needs repainting.
    pub fn key(&mut self, k: Key) -> bool {
        match k {
            Key::Quit => {
                self.quit = true;
                false
            }
            Key::Sommaire => {
                self.stack.clear();
                self.page = Page::Sommaire;
                self.sel = 0;
                self.scroll = 0;
                true
            }
            Key::Retour => {
                if self.stack.is_empty() {
                    self.quit = true;
                    false
                } else {
                    self.back();
                    true
                }
            }
            Key::Fast => {
                self.fast = !self.fast;
                true
            }
            Key::Up => {
                if matches!(self.page, Page::Article(_)) {
                    self.scroll = self.scroll.saturating_sub(1);
                } else {
                    let n = self.menu_len();
                    if n > 0 {
                        self.sel = (self.sel + n - 1) % n;
                    }
                }
                true
            }
            Key::Down => {
                if matches!(self.page, Page::Article(_)) {
                    let max = self.body.len().saturating_sub(self.body_h());
                    self.scroll = (self.scroll + 1).min(max);
                } else {
                    let n = self.menu_len();
                    if n > 0 {
                        self.sel = (self.sel + 1) % n;
                    }
                }
                true
            }
            Key::Suite => {
                if matches!(self.page, Page::Article(_)) {
                    let max = self.body.len().saturating_sub(self.body_h());
                    self.scroll = (self.scroll + self.body_h()).min(max);
                    true
                } else {
                    false
                }
            }
            Key::Retourpage => {
                if matches!(self.page, Page::Article(_)) {
                    self.scroll = self.scroll.saturating_sub(self.body_h());
                    true
                } else {
                    false
                }
            }
            Key::Enter => {
                self.open_selected();
                true
            }
            Key::Digit(d) => {
                let n = self.menu_len();
                if d >= 1 && d <= n {
                    self.sel = d - 1;
                    self.open_selected();
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn resize(&mut self, w: usize, h: usize) {
        self.w = w;
        self.h = h;
        if let Page::Article(idx) = self.page {
            let iw = self.inner_w();
            self.body = post_text(&self.feed.posts[idx].html, iw);
            let max = self.body.len().saturating_sub(self.body_h());
            self.scroll = self.scroll.min(max);
        }
    }

    pub fn render(&self) -> Screen {
        let mut s = Screen::new(self.w, self.h);
        self.chrome(&mut s);
        match self.page {
            Page::Sommaire => self.sommaire(&mut s),
            Page::Articles => self.articles(&mut s),
            Page::Article(i) => self.article(&mut s, i),
            Page::Projets => self.projets(&mut s),
            Page::Apropos => self.apropos(&mut s),
        }
        s
    }

    fn chrome(&self, s: &mut Screen) {
        // Top bar: inverse video, the way a Minitel service announced itself.
        s.fill_row(0, Style::bar(BLACK, CYAN));
        s.text(1, 0, "MARLINSKI", Style { fg: BLACK, bg: CYAN, bold: true });
        let right = match self.page {
            Page::Sommaire => "SOMMAIRE",
            Page::Articles => "ARTICLES",
            Page::Article(_) => "ARTICLE",
            Page::Projets => "PROJETS",
            Page::Apropos => "A PROPOS",
        };
        let x = s.w.saturating_sub(right.chars().count() + 1);
        s.text(x, 0, right, Style { fg: BLACK, bg: CYAN, bold: true });

        // Footer: the Minitel function keys.
        let fy = s.h.saturating_sub(1);
        s.fill_row(fy, Style::bar(BLACK, WHITE));
        let keys = match self.page {
            Page::Article(_) => " ESPACE suite   ^v defiler   R retour   S sommaire   Q fin ",
            Page::Sommaire => " 1-4 choix   ^v deplacer   ENTREE valider   F vitesse   Q fin ",
            _ => " 1-9 choix   ^v deplacer   ENTREE ouvrir   R retour   S sommaire ",
        };
        s.text(1, fy, keys, Style::bar(BLACK, WHITE));
        if self.fast {
            let t = "RAPIDE";
            let x = s.w.saturating_sub(t.chars().count() + 1);
            s.text(x, fy, t, Style { fg: RED, bg: WHITE, bold: true });
        }
    }

    fn rule(&self, s: &mut Screen, y: usize) {
        for x in 1..s.w.saturating_sub(1) {
            s.put(x, y, '─', Style::fg(BLUE));
        }
    }

    fn sommaire(&self, s: &mut Screen) {
        s.center(2, &self.feed.title.to_uppercase(), Style::bold(YELLOW));
        let tag = truncate(&self.feed.tagline, self.inner_w());
        s.center(3, &tag, Style::fg(CYAN));
        self.rule(s, 4);

        if !self.feed.now.is_empty() {
            let now = truncate(&self.feed.now, self.inner_w().saturating_sub(6));
            s.text(2, 5, "NOW", Style::bold(GREEN));
            s.text(6, 5, &now, Style::fg(WHITE));
        }

        let items = [
            ("ARTICLES", format!("{} textes", self.feed.posts.len())),
            ("PROJETS", format!("{} projets", self.feed.projects.len())),
            ("A PROPOS", "qui, ou, quoi".to_string()),
            ("FIN", "raccrocher".to_string()),
        ];
        let mut y = 7;
        for (i, (label, sub)) in items.iter().enumerate() {
            let selected = i == self.sel;
            let marker = if selected { '>' } else { ' ' };
            s.put(2, y, marker, Style::bold(RED));
            s.text(4, y, &format!("{}", i + 1), Style::bold(CYAN));
            s.text(5, y, " ", Style::default());
            let st = if selected { Style::bold(YELLOW) } else { Style::fg(WHITE) };
            let after = s.text(6, y, label, st);
            s.text(after + 2, y, sub, Style::fg(BLUE));
            y += 2;
        }
    }

    fn articles(&self, s: &mut Screen) {
        s.text(2, 2, "ARTICLES", Style::bold(YELLOW));
        self.rule(s, 3);
        let top = self.sel.saturating_sub(self.body_h().saturating_sub(2));
        let mut y = 4;
        for (i, p) in self.feed.posts.iter().enumerate().skip(top) {
            if y >= s.h.saturating_sub(2) {
                break;
            }
            let selected = i == self.sel;
            s.put(2, y, if selected { '>' } else { ' ' }, Style::bold(RED));
            s.text(4, y, &format!("{:>2}", i + 1), Style::bold(CYAN));
            s.text(7, y, &p.date, Style::fg(BLUE));
            let st = if selected { Style::bold(YELLOW) } else { Style::fg(WHITE) };
            let room = s.w.saturating_sub(20);
            s.text(18, y, &truncate(&p.title, room), st);
            y += 1;
            if selected && !p.blurb.is_empty() && y < s.h.saturating_sub(2) {
                s.text(18, y, &truncate(&p.blurb, room), Style::fg(BLUE));
                y += 1;
            }
        }
    }

    fn article(&self, s: &mut Screen, idx: usize) {
        let post = &self.feed.posts[idx];
        s.text(2, 1, &truncate(&post.title, self.inner_w()), Style::bold(YELLOW));
        let tags = post
            .tags
            .iter()
            .map(|t| format!("#{t}"))
            .collect::<Vec<_>>()
            .join(" ");
        s.text(2, 2, &post.date, Style::fg(BLUE));
        s.text(14, 2, &truncate(&tags, s.w.saturating_sub(16)), Style::fg(MAGENTA));
        self.rule(s, 3);

        let mut y = 4;
        for line in self.body.iter().skip(self.scroll) {
            if y >= s.h.saturating_sub(2) {
                break;
            }
            // html2text marks headings with #, which doubles nicely as a cue
            // for where to put colour.
            let style = if line.starts_with('#') {
                Style::bold(GREEN)
            } else if line.starts_with("    ") || line.starts_with('\t') {
                Style::fg(CYAN)
            } else {
                Style::fg(WHITE)
            };
            s.text(2, y, &truncate(line, self.inner_w()), style);
            y += 1;
        }

        // Where to find the same thing on the web.
        if self.scroll + self.body_h() >= self.body.len() {
            let y = s.h.saturating_sub(2);
            s.text(2, y, &truncate(&post.url, s.w.saturating_sub(10)), Style::fg(MAGENTA));
        }

        // Scroll indicator, right edge.
        let total = self.body.len().max(1);
        let shown = (self.scroll + self.body_h()).min(total);
        let pct = shown * 100 / total;
        let t = format!("{pct:>3}%");
        let x = s.w.saturating_sub(6);
        s.text(x, s.h.saturating_sub(2), &t, Style::fg(BLUE));
    }

    fn projets(&self, s: &mut Screen) {
        s.text(2, 2, "PROJETS", Style::bold(YELLOW));
        self.rule(s, 3);
        let top = self.sel.saturating_sub(self.body_h().saturating_sub(2));
        let mut y = 4;
        for (i, p) in self.feed.projects.iter().enumerate().skip(top) {
            if y >= s.h.saturating_sub(2) {
                break;
            }
            let selected = i == self.sel;
            s.put(2, y, if selected { '>' } else { ' ' }, Style::bold(RED));
            // Same three states as the website's markers.
            let (mark, mstyle) = match p.status.as_str() {
                "active" => ('■', Style::bold(GREEN)),
                "idle" => ('■', Style::fg(WHITE)),
                "archived" => ('□', Style::fg(BLUE)),
                _ => (' ', Style::default()),
            };
            s.put(4, y, mark, mstyle);
            let st = if selected { Style::bold(YELLOW) } else { Style::fg(WHITE) };
            s.text(6, y, &truncate(&p.name, 16), st);
            let room = s.w.saturating_sub(26);
            s.text(24, y, &truncate(&p.blurb, room), Style::fg(BLUE));
            y += 1;
        }
    }

    fn apropos(&self, s: &mut Screen) {
        s.text(2, 2, "A PROPOS", Style::bold(YELLOW));
        self.rule(s, 3);
        let mut y = 5;
        for line in wrap(&self.feed.tagline, self.inner_w()) {
            s.text(2, y, &line, Style::fg(WHITE));
            y += 1;
        }
        y += 1;
        if !self.feed.now.is_empty() {
            s.text(2, y, "NOW", Style::bold(GREEN));
            y += 1;
            for line in wrap(&self.feed.now, self.inner_w()) {
                s.text(2, y, &line, Style::fg(WHITE));
                y += 1;
            }
            y += 1;
        }
        for (label, value) in [
            ("web", "https://marlinski.org"),
            ("code", "https://github.com/Marlinski"),
            ("ssh", "ssh minitel.marlinski.org"),
        ] {
            s.text(2, y, label, Style::fg(BLUE));
            s.text(8, y, value, Style::fg(CYAN));
            y += 1;
        }
    }
}

#[derive(Clone, Copy)]
pub enum Key {
    Up,
    Down,
    Enter,
    Digit(usize),
    Retour,
    Sommaire,
    Suite,
    Retourpage,
    Fast,
    Quit,
}

/// Decodes a byte slice from the client into keys. Handles the arrow-key CSI
/// sequences; anything unrecognised is dropped.
pub fn decode(data: &[u8]) -> Vec<Key> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < data.len() {
        match data[i] {
            0x1b if i + 2 < data.len() && data[i + 1] == b'[' => {
                match data[i + 2] {
                    b'A' => out.push(Key::Up),
                    b'B' => out.push(Key::Down),
                    b'C' => out.push(Key::Enter),
                    b'D' => out.push(Key::Retour),
                    b'5' => out.push(Key::Retourpage),
                    b'6' => out.push(Key::Suite),
                    _ => {}
                }
                i += 3;
                continue;
            }
            0x1b => out.push(Key::Retour),
            b'\r' | b'\n' => out.push(Key::Enter),
            b' ' => out.push(Key::Suite),
            0x7f | 0x08 => out.push(Key::Retour),
            0x03 | 0x04 => out.push(Key::Quit),
            b'q' | b'Q' => out.push(Key::Quit),
            b'r' | b'R' => out.push(Key::Retour),
            b's' | b'S' => out.push(Key::Sommaire),
            b'f' | b'F' => out.push(Key::Fast),
            b'k' => out.push(Key::Up),
            b'j' => out.push(Key::Down),
            c @ b'1'..=b'9' => out.push(Key::Digit((c - b'0') as usize)),
            _ => {}
        }
        i += 1;
    }
    out
}

fn truncate(s: &str, max: usize) -> String {
    let s = s.replace(['\n', '\r', '\t'], " ");
    if s.chars().count() <= max {
        s
    } else {
        s.chars().take(max.saturating_sub(1)).collect::<String>() + "…"
    }
}

fn wrap(s: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut line = String::new();
    for word in s.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
            out.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}
