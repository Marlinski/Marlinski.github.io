//! Panes, navigation and chrome.
//!
//! One state, three projections. The same selection model is drawn as three
//! panes on a wide terminal, two when it is narrower, and one on a phone — the
//! layout changes, the navigation does not.

use std::sync::Arc;

use crate::feed::{md_lines, post_lines, Feed, Span};
use crate::screen::*;

#[derive(Clone, Copy, PartialEq)]
pub enum Section {
    Writing,
    Projects,
    About,
}

impl Section {
    const ALL: [Section; 3] = [Section::About, Section::Writing, Section::Projects];
    fn label(self) -> &'static str {
        match self {
            Section::Writing => "WRITING",
            Section::Projects => "PROJECTS",
            Section::About => "ABOUT",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Focus {
    Nav,
    List,
    Content,
}

pub struct App {
    pub feed: Arc<Feed>,
    pub section: Section,
    pub item: usize,
    pub focus: Focus,
    pub scroll: usize,
    pub body: Vec<Vec<Span>>,
    /// What `body` currently holds, so re-selecting does not rebuild it.
    pub body_key: String,
    pub want_readme: Option<(usize, Vec<String>)>,
    pub slow: bool,
    pub theme: ThemeKind,
    pub quit: bool,
    pub w: usize,
    pub h: usize,
}

impl App {
    pub fn new(feed: Arc<Feed>, w: usize, h: usize) -> Self {
        let mut a = App {
            feed,
            section: Section::About,
            item: 0,
            // Start on the menu; one pane wide there is no menu, so the list
            // is the first thing.
            focus: if w >= 110 { Focus::Nav } else { Focus::List },
            scroll: 0,
            body: Vec::new(),
            body_key: String::new(),
            want_readme: None,
            slow: false,
            theme: ThemeKind::Dark,
            quit: false,
            w,
            h,
        };
        a.sync_body();
        a
    }

    fn th(&self) -> Theme {
        Theme::get(self.theme)
    }

    /// 3 panes when there is room, 2 when there is less, 1 on a phone.
    fn panes(&self) -> usize {
        if self.w >= 110 {
            3
        } else if self.w >= 76 {
            2
        } else {
            1
        }
    }

    fn list_len(&self) -> usize {
        match self.section {
            Section::Writing => self.feed.posts.len(),
            Section::Projects => self.feed.projects.len(),
            Section::About => 0,
        }
    }

    /// (x, width) for nav, list and content. Width 0 means "not at this size".
    ///
    /// About has no list of its own, so its list pane folds into the content
    /// rather than sitting there empty.
    fn layout(&self) -> [(usize, usize); 3] {
        let inner = self.w;
        let flat = self.section == Section::About;
        match self.panes() {
            3 => {
                let nav = 20;
                if flat {
                    return [(0, nav), (0, 0), (nav, inner - nav)];
                }
                let list = ((inner - nav) * 42 / 100).max(26);
                let content = inner - nav - list;
                [(0, nav), (nav, list), (nav + list, content)]
            }
            2 => {
                if flat {
                    return [(0, 0), (0, 0), (0, inner)];
                }
                let list = (inner * 40 / 100).clamp(24, 40);
                [(0, 0), (0, list), (list, inner - list)]
            }
            _ => match self.focus {
                Focus::Content => [(0, 0), (0, 0), (0, inner)],
                _ => [(0, 0), (0, inner), (0, 0)],
            },
        }
    }

    fn content_h(&self) -> usize {
        self.h.saturating_sub(6).max(3)
    }

    fn content_w(&self) -> usize {
        let [_, _, (_, cw)] = self.layout();
        (if cw == 0 { self.w } else { cw }).saturating_sub(4).max(20)
    }

    // ── content ──────────────────────────────────────────────────────────

    /// Rebuilds the content pane for whatever is selected.
    pub fn sync_body(&mut self) {
        let w = self.content_w();
        let th = self.th();
        match self.section {
            Section::Writing => {
                if let Some(p) = self.feed.posts.get(self.item) {
                    let key = format!("post:{}:{w}", p.url);
                    if key != self.body_key {
                        self.body = post_lines(&p.html, w, &th);
                        self.body_key = key;
                        self.scroll = 0;
                    }
                } else {
                    self.body.clear();
                }
            }
            Section::Projects => {
                if let Some(p) = self.feed.projects.get(self.item) {
                    let key = format!("proj:{}:{w}", p.name);
                    if key != self.body_key {
                        self.body_key = key;
                        self.scroll = 0;
                        let urls = p.readme_urls();
                        match urls.is_empty() {
                            false => {
                                self.body =
                                    vec![vec![("fetching README…".into(), th.on(th.dim))]];
                                self.want_readme = Some((self.item, urls));
                            }
                            true => {
                                self.body = vec![
                                    vec![(p.blurb.clone(), th.base())],
                                    vec![(String::new(), th.base())],
                                    vec![("no public repository".into(), th.on(th.dim))],
                                ];
                            }
                        }
                    }
                }
            }
            Section::About => {
                let key = format!("about:{w}");
                if key != self.body_key {
                    self.body_key = key;
                    self.scroll = 0;
                    let mut b: Vec<Vec<Span>> = Vec::new();
                    for l in wrap(&self.feed.tagline, w) {
                        b.push(vec![(l, th.base())]);
                    }
                    b.push(vec![(String::new(), th.base())]);
                    if !self.feed.now.is_empty() {
                        b.push(vec![("NOW".into(), th.strong(th.code))]);
                        for l in wrap(&self.feed.now, w) {
                            b.push(vec![(l, th.base())]);
                        }
                        b.push(vec![(String::new(), th.base())]);
                    }
                    for (k, v) in [
                        ("web", "https://marlinski.org"),
                        ("code", "https://github.com/Marlinski"),
                        ("ssh", "ssh minitel.marlinski.org"),
                    ] {
                        b.push(vec![
                            (format!("{k:<6}"), th.on(th.dim)),
                            (
                                v.to_string(),
                                Style { underline: true, ..th.on(th.link) },
                            ),
                        ]);
                    }
                    self.body = b;
                }
            }
        }
    }

    pub fn set_readme(&mut self, md: Option<String>) {
        let w = self.content_w();
        let th = self.th();
        self.body = match md {
            Some(t) => md_lines(&t, w, &th),
            None => vec![vec![("README unavailable".into(), th.on(RED))]],
        };
        self.scroll = 0;
    }

    // ── input ────────────────────────────────────────────────────────────

    pub fn key(&mut self, k: Key) -> bool {
        match k {
            Key::Quit => {
                self.quit = true;
                false
            }
            Key::Theme => {
                self.theme = match self.theme {
                    ThemeKind::Dark => ThemeKind::Dos,
                    ThemeKind::Dos => ThemeKind::Dark,
                };
                // Styles are baked into the body spans, so it has to be rebuilt.
                self.body_key.clear();
                self.sync_body();
                true
            }
            Key::Baud => {
                self.slow = !self.slow;
                true
            }
            Key::Index => {
                self.focus = if self.panes() == 3 { Focus::Nav } else { Focus::List };
                true
            }
            Key::Back => {
                // Walks left through the panes and stops. Leaving is Q or
                // Ctrl-C, never an extra Escape — losing the session because
                // you pressed Escape once too often is infuriating.
                self.focus = match self.focus {
                    Focus::Content => Focus::List,
                    Focus::List if self.panes() == 3 => Focus::Nav,
                    f => f,
                };
                true
            }
            Key::Enter | Key::Right => {
                self.focus = match self.focus {
                    Focus::Nav if self.list_len() > 0 => Focus::List,
                    _ => Focus::Content,
                };
                true
            }
            Key::Left => {
                self.focus = match self.focus {
                    Focus::Content if self.list_len() > 0 => Focus::List,
                    Focus::Content if self.panes() == 3 => Focus::Nav,
                    Focus::List if self.panes() == 3 => Focus::Nav,
                    f => f,
                };
                true
            }
            Key::Tab => {
                let three = self.panes() == 3;
                let has_list = self.list_len() > 0;
                self.focus = match self.focus {
                    Focus::Nav if has_list => Focus::List,
                    Focus::Nav => Focus::Content,
                    Focus::List => Focus::Content,
                    Focus::Content => {
                        if three {
                            Focus::Nav
                        } else if has_list {
                            Focus::List
                        } else {
                            Focus::Content
                        }
                    }
                };
                true
            }
            Key::Up => {
                match self.focus {
                    Focus::Nav => {
                        let i = Section::ALL.iter().position(|s| *s == self.section).unwrap_or(0);
                        self.section = Section::ALL[(i + 2) % 3];
                        self.item = 0;
                        self.body_key.clear();
                        self.sync_body();
                    }
                    Focus::List => {
                        let n = self.list_len();
                        if n > 0 {
                            self.item = (self.item + n - 1) % n;
                            self.sync_body();
                        }
                    }
                    Focus::Content => self.scroll = self.scroll.saturating_sub(1),
                }
                true
            }
            Key::Down => {
                match self.focus {
                    Focus::Nav => {
                        let i = Section::ALL.iter().position(|s| *s == self.section).unwrap_or(0);
                        self.section = Section::ALL[(i + 1) % 3];
                        self.item = 0;
                        self.body_key.clear();
                        self.sync_body();
                    }
                    Focus::List => {
                        let n = self.list_len();
                        if n > 0 {
                            self.item = (self.item + 1) % n;
                            self.sync_body();
                        }
                    }
                    Focus::Content => {
                        let max = self.body.len().saturating_sub(self.content_h());
                        self.scroll = (self.scroll + 1).min(max);
                    }
                }
                true
            }
            Key::Next => {
                let max = self.body.len().saturating_sub(self.content_h());
                self.scroll = (self.scroll + self.content_h()).min(max);
                true
            }
            Key::Prev => {
                self.scroll = self.scroll.saturating_sub(self.content_h());
                true
            }
            Key::Digit(d) => {
                if d >= 1 && d <= self.list_len() {
                    self.item = d - 1;
                    self.focus = Focus::Content;
                    self.sync_body();
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
        self.body_key.clear();
        self.sync_body();
    }

    // ── rendering ────────────────────────────────────────────────────────

    pub fn render(&self) -> Screen {
        let th = self.th();
        let mut s = Screen::new(self.w, self.h);
        s.clear_to(th.base());

        self.title_bar(&mut s, &th);
        let [(nx, nw), (lx, lw), (cx, cw)] = self.layout();
        let two = self.panes() == 2;
        let top = if two { 2 } else { 1 };
        let height = self.h.saturating_sub(top + 1);

        if nw > 0 {
            self.nav_pane(&mut s, &th, nx, top, nw, height);
        } else if two {
            self.tab_strip(&mut s, &th);
        }
        if lw > 0 {
            self.list_pane(&mut s, &th, lx, top, lw, height);
        }
        if cw > 0 {
            self.content_pane(&mut s, &th, cx, top, cw, height);
        }
        self.status_bar(&mut s, &th);
        s
    }

    fn title_bar(&self, s: &mut Screen, th: &Theme) {
        s.fill_row(0, th.bar());
        s.text(1, 0, "3615 MARLINSKI", th.bar());
        if self.w > 64 {
            s.center(0, "MINITEL SERVICE", th.bar());
        }
        let right = format!(
            "{}{}",
            if self.slow { "1200 BAUD  " } else { "" },
            match self.theme {
                ThemeKind::Dark => "DARK",
                ThemeKind::Dos => "DOS",
            }
        );
        let x = self.w.saturating_sub(right.chars().count() + 1);
        s.text(x, 0, &right, th.bar());
    }

    fn tab_strip(&self, s: &mut Screen, th: &Theme) {
        let mut x = 1;
        for sec in Section::ALL {
            let st = if sec == self.section { th.sel() } else { th.on(th.dim) };
            x = s.text(x, 1, &format!(" {} ", sec.label()), st) + 1;
        }
    }

    fn nav_pane(&self, s: &mut Screen, th: &Theme, x: usize, y: usize, w: usize, h: usize) {
        let focused = self.focus == Focus::Nav;
        let (bs, ts) = pane_style(th, focused);
        s.frame(x, y, w, h, th.border, bs, Some(("MENU", ts)));

        let mut ly = y + 2;
        for sec in Section::ALL {
            let on = sec == self.section;
            let st = if on && focused {
                th.sel()
            } else if on {
                th.strong(th.heading)
            } else {
                th.base()
            };
            let n = match sec {
                Section::Writing => self.feed.posts.len(),
                Section::Projects => self.feed.projects.len(),
                Section::About => 0,
            };
            let label = if n > 0 {
                format!(" {:<9}{:>3} ", sec.label(), n)
            } else {
                format!(" {:<12} ", sec.label())
            };
            s.text(x + 1, ly, &label, st);
            ly += 1;
        }

        // A little more life than an empty box: the place, in numbers.
        ly += 1;
        for i in x + 1..x + w.saturating_sub(1) {
            s.put(i, ly, th.border.h, th.on(th.dim));
        }
        ly += 1;
        let active = self.feed.projects.iter().filter(|p| p.status == "active").count();
        for (k, v) in [
            ("posts", format!("{}", self.feed.posts.len())),
            ("projects", format!("{}", self.feed.projects.len())),
            ("active", format!("{active}")),
        ] {
            if ly >= y + h - 1 {
                break;
            }
            s.text(x + 2, ly, k, th.on(th.dim));
            let vx = x + w - 2 - v.chars().count();
            s.text(vx, ly, &v, th.strong(th.accent));
            ly += 1;
        }

        ly += 1;
        for l in wrap(&self.feed.now, w.saturating_sub(4)) {
            if ly >= y + h - 1 {
                break;
            }
            s.text(x + 2, ly, &l, th.on(th.code));
            ly += 1;
        }
    }

    fn list_pane(&self, s: &mut Screen, th: &Theme, x: usize, y: usize, w: usize, h: usize) {
        let focused = self.focus == Focus::List;
        let (bs, ts) = pane_style(th, focused);
        s.frame(x, y, w, h, th.border, bs, Some((self.section.label(), ts)));

        let rows = h.saturating_sub(2);
        let mut ly = y + 1;
        let iw = w.saturating_sub(4);

        match self.section {
            Section::Writing => {
                // Titles wrap rather than getting cut off — a list of posts is
                // no use if you cannot read which post is which. Continuations
                // align under the title, clear of the date column.
                let indent = 11usize;
                let tw = iw.saturating_sub(indent).max(12);
                let mut entries: Vec<(usize, Vec<String>)> = Vec::new();
                for (i, p) in self.feed.posts.iter().enumerate() {
                    entries.push((i, wrap(&p.title, tw)));
                }
                // Scroll by entry, keeping the selected one on screen.
                let mut top = 0usize;
                loop {
                    let used: usize = entries[top..]
                        .iter()
                        .take_while(|(i, _)| *i <= self.item)
                        .map(|(_, l)| l.len())
                        .sum();
                    if used <= rows || top >= self.item {
                        break;
                    }
                    top += 1;
                }

                for (i, lines) in entries.iter().skip(top) {
                    if ly >= y + h - 1 {
                        break;
                    }
                    let on = *i == self.item;
                    let st = if on && focused {
                        th.sel()
                    } else if on {
                        th.strong(th.heading)
                    } else {
                        th.base()
                    };
                    for (n, line) in lines.iter().enumerate() {
                        if ly >= y + h - 1 {
                            break;
                        }
                        if on {
                            for k in x + 1..x + w - 1 {
                                s.put(k, ly, ' ', st);
                            }
                        }
                        if n == 0 {
                            let p = &self.feed.posts[*i];
                            s.text(x + 2, ly, &p.date, if on { st } else { th.on(th.dim) });
                        }
                        s.text(x + 2 + indent, ly, line, st);
                        ly += 1;
                    }
                }
            }
            Section::Projects => {
                let top = self.item.saturating_sub(rows.saturating_sub(1));
                for (i, p) in self.feed.projects.iter().enumerate().skip(top) {
                    if ly >= y + h - 1 {
                        break;
                    }
                    let on = i == self.item;
                    let st = if on && focused {
                        th.sel()
                    } else if on {
                        th.strong(th.heading)
                    } else {
                        th.base()
                    };
                    if on {
                        for k in x + 1..x + w - 1 {
                            s.put(k, ly, ' ', st);
                        }
                    }
                    let (mark, mst) = match p.status.as_str() {
                        "active" => ('■', th.strong(th.code)),
                        "idle" => ('■', th.on(th.dim)),
                        _ => ('□', th.on(th.dim)),
                    };
                    s.put(x + 2, ly, mark, if on { st } else { mst });
                    s.text(x + 4, ly, &truncate(&p.name, iw.saturating_sub(2)), st);
                    ly += 1;
                }
            }
            Section::About => {
                for l in wrap(&self.feed.tagline, iw) {
                    if ly >= y + h - 1 {
                        break;
                    }
                    s.text(x + 2, ly, &l, th.base());
                    ly += 1;
                }
            }
        }
    }

    fn content_pane(&self, s: &mut Screen, th: &Theme, x: usize, y: usize, w: usize, h: usize) {
        let focused = self.focus == Focus::Content;
        let (bs, ts) = pane_style(th, focused);

        // Header inside the frame: what this is, and where it came from. A
        // README's source URL belongs at the top, not buried at the end.
        let (title, subtitle) = match self.section {
            Section::Writing => match self.feed.posts.get(self.item) {
                Some(p) => (p.title.clone(), p.url.clone()),
                None => (String::new(), String::new()),
            },
            Section::Projects => match self.feed.projects.get(self.item) {
                Some(p) => (p.name.clone(), if p.has_repo() { p.gh.clone() } else { p.url.clone() }),
                None => (String::new(), String::new()),
            },
            Section::About => ("ABOUT".to_string(), "marlinski.org".to_string()),
        };

        s.frame(
            x,
            y,
            w,
            h,
            th.border,
            bs,
            Some((&truncate(&title, w.saturating_sub(8)), ts)),
        );

        let iw = w.saturating_sub(4);
        let mut ly = y + 1;
        if !subtitle.is_empty() && h > 6 {
            s.text(
                x + 2,
                ly,
                &truncate(&subtitle, iw),
                Style { underline: true, ..th.on(th.link) },
            );
            ly += 1;
            for k in x + 1..x + w - 1 {
                s.put(k, ly, th.border.h, th.on(th.dim));
            }
            ly += 1;
        }

        let body_top = ly;
        for line in self.body.iter().skip(self.scroll) {
            if ly >= y + h - 1 {
                break;
            }
            let mut cx = x + 2;
            let limit = x + w - 2;
            for (text, style) in line {
                if cx >= limit {
                    break;
                }
                let clipped: String = text.chars().take(limit - cx).collect();
                cx = s.text(cx, ly, &clipped, *style);
            }
            ly += 1;
        }

        let total = self.body.len().max(1);
        let shown = (self.scroll + ly.saturating_sub(body_top)).min(total);
        let t = format!(" {:>3}% ", shown * 100 / total);
        let tx = x + w - 1 - t.chars().count();
        s.text(tx, y + h - 1, &t, th.on(th.dim));
    }

    fn status_bar(&self, s: &mut Screen, th: &Theme) {
        let fy = self.h.saturating_sub(1);
        s.fill_row(fy, th.bar());
        let keys = match self.focus {
            Focus::Content => " ^v scroll  SPACE page  TAB pane  B back  F baud  T theme  Q quit ",
            _ => " ^v move  ENTER open  TAB pane  1-9 jump  F baud  T theme  Q quit ",
        };
        s.text(1, fy, keys, th.bar());
    }
}

/// Border and title styling for a pane, so which one has focus is obvious at
/// a glance: the active frame is drawn in the accent colour with its title
/// reversed out.
fn pane_style(th: &Theme, focused: bool) -> (Style, Style) {
    if focused {
        (th.strong(th.accent), th.sel())
    } else {
        (th.on(th.dim), th.on(th.dim))
    }
}

// ── keys ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Tab,
    Digit(usize),
    Back,
    Index,
    Next,
    Prev,
    Baud,
    Theme,
    Quit,
}

/// Decodes keys, returning any trailing bytes that are the start of an escape
/// sequence but not yet a whole one.
///
/// SSH gives no guarantee that a three-byte arrow key arrives in one packet, so
/// a partial sequence is handed back and prepended to the next read rather than
/// being misread as a bare Escape followed by junk.
pub fn decode(data: &[u8]) -> (Vec<Key>, Vec<u8>) {
    let mut out = Vec::new();
    let mut i = 0;
    while i < data.len() {
        match data[i] {
            // Incomplete CSI at the end of the buffer: keep it for next time.
            0x1b if data.len() - i < 3 && (data.len() - i == 1 || data[i + 1] == b'[') => {
                return (out, data[i..].to_vec());
            }
            0x1b if data[i + 1] == b'[' => {
                match data[i + 2] {
                    b'A' => out.push(Key::Up),
                    b'B' => out.push(Key::Down),
                    b'C' => out.push(Key::Right),
                    b'D' => out.push(Key::Left),
                    b'5' => out.push(Key::Prev),
                    b'6' => out.push(Key::Next),
                    _ => {}
                }
                i += 3;
                continue;
            }
            0x1b => out.push(Key::Back),
            b'\t' => out.push(Key::Tab),
            b'\r' | b'\n' => out.push(Key::Enter),
            b' ' => out.push(Key::Next),
            0x7f | 0x08 => out.push(Key::Back),
            0x03 | 0x04 => out.push(Key::Quit),
            b'q' | b'Q' => out.push(Key::Quit),
            b'b' | b'B' | b'r' | b'R' => out.push(Key::Back),
            b'i' | b'I' | b's' | b'S' => out.push(Key::Index),
            b'f' | b'F' => out.push(Key::Baud),
            b't' | b'T' => out.push(Key::Theme),
            b'k' => out.push(Key::Up),
            b'j' => out.push(Key::Down),
            b'h' => out.push(Key::Left),
            b'l' => out.push(Key::Right),
            c @ b'1'..=b'9' => out.push(Key::Digit((c - b'0') as usize)),
            _ => {}
        }
        i += 1;
    }
    (out, Vec::new())
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
    let width = width.max(8);
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
