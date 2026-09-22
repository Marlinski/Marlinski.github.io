//! Panes, navigation and chrome.
//!
//! One state, three projections. The same selection model is drawn as three
//! panes on a wide terminal, two when it is narrower, and one on a phone — the
//! layout changes, the navigation does not.

use std::sync::Arc;

use crate::feed::{md_lines, post_lines, Feed, Span};
use crate::store::Message;
use crate::screen::*;

#[derive(Clone, Copy, PartialEq)]
pub enum Section {
    Writing,
    Projects,
    Public,
    Message,
    Inbox,
    About,
}

impl Section {
    fn label(self) -> &'static str {
        match self {
            Section::Writing => "WRITING",
            Section::Projects => "PROJECTS",
            Section::Public => "PUBLIC",
            Section::Message => "MESSAGE",
            Section::Inbox => "INBOX",
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
    /// True when the connecting key matched MINITEL_ADMIN_KEY.
    pub admin: bool,
    /// Whatever name they used: ssh alice@minitel.marlinski.org
    pub user: String,
    pub compose: String,
    pub email: String,
    /// 0 = email, 1 = message.
    pub field: usize,
    pub pubkey: String,
    pub notice: Option<String>,
    /// Set to a message id after the first D, cleared by anything else.
    pub confirm_delete: Option<i64>,
    pub want_delete: Option<i64>,
    pub inbox: Vec<Message>,
    pub want_send: Option<String>,
    pub want_inbox: bool,
    pub want_mark_read: Option<i64>,
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
            admin: false,
            user: String::new(),
            compose: String::new(),
            email: String::new(),
            field: 0,
            pubkey: String::new(),
            notice: None,
            confirm_delete: None,
            want_delete: None,
            inbox: Vec::new(),
            want_send: None,
            want_inbox: false,
            want_mark_read: None,
            quit: false,
            w,
            h,
        };
        a.sync_body();
        a
    }

    /// INBOX only exists for the owner.
    pub fn sections(&self) -> Vec<Section> {
        let mut v = vec![
            Section::About,
            Section::Writing,
            Section::Projects,
            Section::Public,
            Section::Message,
        ];
        if self.admin {
            v.push(Section::Inbox);
        }
        v
    }

    /// True while the compose box has the keyboard: raw characters then, not
    /// navigation keys.
    pub fn editing(&self) -> bool {
        self.section == Section::Message && self.focus == Focus::Content
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
            Section::Public => self.feed.public.len(),
            Section::Inbox => self.inbox.len(),
            Section::Message | Section::About => 0,
        }
    }

    /// (x, width) for nav, list and content. Width 0 means "not at this size".
    ///
    /// About has no list of its own, so its list pane folds into the content
    /// rather than sitting there empty.
    fn layout(&self) -> [(usize, usize); 3] {
        let inner = self.w;
        let flat = matches!(self.section, Section::About | Section::Message);
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
                let list = (inner * 40 / 100).clamp(24, 40);
                if self.focus == Focus::Nav {
                    return [(0, list), (0, 0), (list, inner - list)];
                }
                if flat {
                    return [(0, 0), (0, 0), (0, inner)];
                }
                [(0, 0), (0, list), (list, inner - list)]
            }
            _ => match self.focus {
                Focus::Nav => [(0, inner), (0, 0), (0, 0)],
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
            Section::Public => {
                if let Some(e) = self.feed.public.get(self.item) {
                    let key = format!("pub:{}:{w}", e.title);
                    if key != self.body_key {
                        self.body_key = key;
                        self.scroll = 0;
                        let mut b: Vec<Vec<Span>> = Vec::new();
                        for l in wrap(&e.title, w) {
                            b.push(vec![(l, th.strong(th.heading))]);
                        }
                        let mut meta = e.date.clone();
                        if !e.where_at.is_empty() {
                            meta.push_str("  ");
                            meta.push_str(&e.where_at);
                        }
                        b.push(vec![(meta, th.on(th.dim))]);
                        b.push(vec![(String::new(), th.base())]);
                        b.push(vec![("FILES".into(), th.strong(th.code))]);
                        for f in &e.files {
                            let mut row = vec![(format!("  {}", f.name), th.base())];
                            if !f.desc.is_empty() {
                                row.push((format!("  {}", f.desc), th.on(th.dim)));
                            }
                            b.push(row);
                            b.push(vec![(
                                format!("    {}", f.url),
                                Style { underline: true, ..th.on(th.link) },
                            )]);
                        }
                        self.body = b;
                    }
                }
            }
            Section::Message => {
                // Drawn directly by form(), so there is no body to build.
                self.body_key = "msg".into();
                self.body.clear();
            }
            Section::Inbox => {
                if self.inbox.is_empty() {
                    self.body_key = "inbox:empty".into();
                    self.body = vec![vec![("no messages yet".into(), th.on(th.dim))]];
                } else if let Some(m) = self.inbox.get(self.item) {
                    let key = format!("inbox:{}:{w}:{}", m.id, m.read);
                    if key != self.body_key {
                        self.body_key = key;
                        self.scroll = 0;
                        let mut b: Vec<Vec<Span>> = Vec::new();
                        b.push(vec![(
                            format!("from {}", if m.who.is_empty() { "anonymous" } else { &m.who }),
                            th.strong(th.heading),
                        )]);
                        b.push(vec![(format!("{} UTC", m.at), th.on(th.dim))]);
                        if !m.email.is_empty() {
                            b.push(vec![
                                ("email  ".into(), th.on(th.dim)),
                                (m.email.clone(), th.on(th.link)),
                            ]);
                        }
                        if !m.ip.is_empty() {
                            b.push(vec![
                                ("from   ".into(), th.on(th.dim)),
                                (m.ip.clone(), th.on(th.accent)),
                            ]);
                        }
                        if m.pubkey.is_empty() {
                            b.push(vec![("key    none offered".into(), th.on(th.dim))]);
                        } else {
                            for (i, l) in wrap(&m.pubkey, w.saturating_sub(7)).iter().enumerate() {
                                let label = if i == 0 { "key    " } else { "       " };
                                b.push(vec![
                                    (label.to_string(), th.on(th.dim)),
                                    (l.clone(), th.on(th.accent)),
                                ]);
                            }
                        }
                        b.push(vec![(String::new(), th.base())]);
                        for l in wrap(&m.body, w) {
                            b.push(vec![(l, th.base())]);
                        }
                        if !m.read {
                            self.want_mark_read = Some(m.id);
                        }
                        self.body = b;
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

    pub fn set_inbox(&mut self, msgs: Vec<Message>) {
        self.inbox = msgs;
        if self.item >= self.inbox.len() {
            self.item = 0;
        }
        self.body_key.clear();
        self.sync_body();
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
            Key::Delete => {
                if self.admin && self.section == Section::Inbox {
                    if let Some(m) = self.inbox.get(self.item) {
                        if self.confirm_delete == Some(m.id) {
                            self.want_delete = Some(m.id);
                            self.confirm_delete = None;
                            self.notice = Some("deleted".into());
                        } else {
                            // Two presses, because there is no undo.
                            self.confirm_delete = Some(m.id);
                            self.notice = Some("press D again to delete".into());
                        }
                        self.body_key.clear();
                        self.sync_body();
                        return true;
                    }
                }
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
                self.focus = Focus::Nav;
                true
            }
            Key::Back => {
                // Walks left through the panes and stops. Leaving is Q or
                // Ctrl-C, never an extra Escape — losing the session because
                // you pressed Escape once too often is infuriating.
                self.focus = match self.focus {
                    Focus::Content => Focus::List,
                    Focus::List => Focus::Nav,
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
                    Focus::Content => Focus::Nav,
                    Focus::List => Focus::Nav,
                    f => f,
                };
                true
            }
            Key::Tab => {
                let has_list = self.list_len() > 0;
                self.focus = match self.focus {
                    Focus::Nav if has_list => Focus::List,
                    Focus::Nav => Focus::Content,
                    Focus::List => Focus::Content,
                    Focus::Content => Focus::Nav,
                };
                true
            }
            Key::Up => {
                match self.focus {
                    Focus::Nav => {
                        let i = self.sections().iter().position(|s| *s == self.section).unwrap_or(0);
                        let secs = self.sections();
                        self.section = secs[(i + secs.len() - 1) % secs.len()];
                        self.item = 0;
                        self.body_key.clear();
                        if self.section == Section::Inbox {
                            self.want_inbox = true;
                        }
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
                        let i = self.sections().iter().position(|s| *s == self.section).unwrap_or(0);
                        let secs = self.sections();
                        self.section = secs[(i + 1) % secs.len()];
                        self.item = 0;
                        self.body_key.clear();
                        if self.section == Section::Inbox {
                            self.want_inbox = true;
                        }
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

    fn field_mut(&mut self) -> &mut String {
        if self.field == 0 {
            &mut self.email
        } else {
            &mut self.compose
        }
    }

    /// Raw bytes while the compose box has the keyboard.
    ///
    /// Navigation keys are deliberately not decoded here — someone typing a
    /// message should be able to write "q" without hanging up.
    pub fn input(&mut self, data: &[u8]) -> bool {
        let mut dirty = false;

        // Strip CSI sequences first. Otherwise an arrow key arrives as
        // ESC [ A: the ESC leaves the field and "[A" gets typed into it.
        let mut clean: Vec<u8> = Vec::with_capacity(data.len());
        let mut i = 0;
        while i < data.len() {
            if data[i] == 0x1b && i + 1 < data.len() && data[i + 1] == b'[' {
                i += 2;
                while i < data.len() && !(0x40..=0x7e).contains(&data[i]) {
                    i += 1;
                }
                i += 1; // the final byte
                continue;
            }
            clean.push(data[i]);
            i += 1;
        }

        let text = String::from_utf8_lossy(&clean);
        for ch in text.chars() {
            match ch {
                '\t' => {
                    self.field = 1 - self.field;
                    dirty = true;
                }
                '\r' | '\n' => {
                    if self.field == 0 {
                        // Enter in the email field just moves on.
                        self.field = 1;
                    } else {
                        let body = self.compose.trim().to_string();
                        let email = self.email.trim().to_string();
                        if !looks_like_email(&email) {
                            self.notice = Some("an email is required to send".into());
                            self.field = 0;
                        } else if body.is_empty() {
                            self.notice = Some("nothing to send".into());
                        } else {
                            self.want_send = Some(body);
                            self.compose.clear();
                            self.notice = Some("sent — thank you".into());
                            self.focus = Focus::List;
                        }
                    }
                    dirty = true;
                }
                '\u{1b}' => {
                    self.focus = Focus::List;
                    dirty = true;
                }
                '\u{15}' => {
                    // Ctrl-U, as a shell would.
                    self.field_mut().clear();
                    dirty = true;
                }
                '\u{7f}' | '\u{8}' => {
                    self.field_mut().pop();
                    dirty = true;
                }
                '\u{3}' | '\u{4}' => {
                    self.quit = true;
                    return false;
                }
                c if !c.is_control() => {
                    let limit = if self.field == 0 { 120 } else { 500 };
                    let f = self.field_mut();
                    if f.chars().count() < limit {
                        f.push(c);
                        self.notice = None;
                        dirty = true;
                    }
                }
                _ => {}
            }
        }
        if dirty {
            self.body_key.clear();
            self.sync_body();
        }
        dirty
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
        }
        if two && nw == 0 {
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
        for sec in self.sections() {
            let st = if sec == self.section { th.sel() } else { th.on(th.dim) };
            x = s.text(x, 1, &format!(" {} ", sec.label()), st) + 1;
        }
    }

    fn nav_pane(&self, s: &mut Screen, th: &Theme, x: usize, y: usize, w: usize, h: usize) {
        let focused = self.focus == Focus::Nav;
        let (bs, ts) = pane_style(th, focused);
        s.frame(x, y, w, h, th.border, bs, Some(("MENU", ts)));

        let mut ly = y + 2;
        for sec in self.sections() {
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
                Section::Public => self.feed.public.len(),
                Section::Inbox => self.inbox.len(),
                _ => 0,
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
            Section::Public => {
                let top = self.item.saturating_sub(rows.saturating_sub(1));
                for (i, e) in self.feed.public.iter().enumerate().skip(top) {
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
                    let mark = if e.kind == "teaching" { '§' } else { '▸' };
                    s.put(x + 2, ly, mark, if on { st } else { th.on(th.dim) });
                    s.text(x + 4, ly, &truncate(&e.title, iw.saturating_sub(2)), st);
                    ly += 1;
                }
            }
            Section::Inbox => {
                let top = self.item.saturating_sub(rows.saturating_sub(1));
                for (i, m) in self.inbox.iter().enumerate().skip(top) {
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
                    let (mark, mst) = if m.read {
                        ('○', th.on(th.dim))
                    } else {
                        ('●', th.strong(th.code))
                    };
                    s.put(x + 2, ly, mark, if on { st } else { mst });
                    let who = if m.who.is_empty() { "anonymous" } else { &m.who };
                    s.text(x + 4, ly, &truncate(who, 12), st);
                    let preview = m.body.replace('\n', " ");
                    s.text(x + 17, ly, &truncate(&preview, iw.saturating_sub(15)), if on { st } else { th.on(th.dim) });
                    ly += 1;
                }
            }
            Section::Message | Section::About => {
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
            Section::Public => match self.feed.public.get(self.item) {
                Some(e) => (e.title.clone(), e.url.clone()),
                None => (String::new(), String::new()),
            },
            Section::Message => ("LEAVE A MESSAGE".to_string(), String::new()),
            Section::Inbox => match self.inbox.get(self.item) {
                Some(m) => (
                    format!("from {}", if m.who.is_empty() { "anonymous" } else { &m.who }),
                    format!("{} UTC", m.at),
                ),
                None => ("INBOX".to_string(), String::new()),
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

        if self.section == Section::Message {
            self.form(s, th, x, ly, w, y + h - ly);
            return;
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


    /// The compose form: a one-line email box and a multi-line message box.
    ///
    /// Drawn rather than composed from body lines, because an input needs a
    /// frame, a caret in the right cell, and a focus colour — none of which a
    /// list of styled strings gives you.
    fn form(&self, s: &mut Screen, th: &Theme, x: usize, y: usize, w: usize, h: usize) {
        let iw = w.saturating_sub(4);
        let mut ly = y;

        for l in wrap(
            "Leave a message. It lands in a mailbox only Marlinski can read.",
            iw,
        ) {
            s.text(x + 2, ly, &l, th.base());
            ly += 1;
        }
        ly += 1;

        if let Some(n) = &self.notice {
            s.text(x + 2, ly, &truncate(n, iw), th.strong(th.code));
        }
        ly += 2;

        let editing = self.editing();
        let bw = iw;

        // ── email, one line ──
        let on_email = editing && self.field == 0;
        s.text(x + 2, ly, "EMAIL", if on_email { th.strong(th.code) } else { th.on(th.dim) });
        ly += 1;
        let ebs = if on_email { th.strong(th.accent) } else { th.on(th.dim) };
        s.frame(x + 2, ly, bw, 3, th.border, ebs, None);
        let evis: String = self.email.chars().rev().take(bw - 4).collect::<Vec<_>>().into_iter().rev().collect();
        let ex = s.text(x + 4, ly + 1, &evis, th.base());
        if on_email {
            s.put(ex, ly + 1, '\u{2588}', th.strong(th.code));
        }
        ly += 4;

        // ── message, as many lines as fit ──
        let on_body = editing && self.field == 1;
        s.text(x + 2, ly, "MESSAGE", if on_body { th.strong(th.code) } else { th.on(th.dim) });
        ly += 1;
        let rows_left = (y + h).saturating_sub(ly + 3).max(3);
        let bh = rows_left.min(8).max(3);
        let bbs = if on_body { th.strong(th.accent) } else { th.on(th.dim) };
        s.frame(x + 2, ly, bw, bh, th.border, bbs, None);

        let inner = bw.saturating_sub(4);
        let lines = wrap(&self.compose, inner);
        let visible = bh.saturating_sub(2);
        let skip = lines.len().saturating_sub(visible);
        let mut ty = ly + 1;
        let mut last_end = x + 4;
        for l in lines.iter().skip(skip) {
            if ty >= ly + bh - 1 {
                break;
            }
            last_end = s.text(x + 4, ty, l, th.base());
            ty += 1;
        }
        if on_body {
            let (cx, cy) = if lines.is_empty() {
                (x + 4, ly + 1)
            } else {
                (last_end, ty.saturating_sub(1))
            };
            s.put(cx, cy, '\u{2588}', th.strong(th.code));
        }
        ly += bh + 1;

        // ── footer ──
        if ly < y + h {
            let hint = if editing {
                "TAB next field   ENTER send   ESC leave"
            } else {
                "ENTER to start typing"
            };
            s.text(x + 2, ly, &truncate(hint, iw.saturating_sub(12)), th.on(th.dim));
            let count = format!("{}/500", self.compose.chars().count());
            let cx = x + w - 2 - count.chars().count();
            s.text(cx, ly, &count, th.on(th.dim));
        }
    }

    fn status_bar(&self, s: &mut Screen, th: &Theme) {
        let fy = self.h.saturating_sub(1);
        s.fill_row(fy, th.bar());
        let keys = if self.editing() {
            " typing…  ENTER send  ESC leave  CTRL-U clear  CTRL-C quit "
        } else {
            match self.focus {
            Focus::Content => " ^v scroll  SPACE page  TAB pane  B back  F baud  T theme  Q quit ",
            _ => " ^v move  ENTER open  TAB pane  1-9 jump  F baud  T theme  Q quit ",
            }
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
    Delete,
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
            b'd' | b'D' => out.push(Key::Delete),
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

/// Deliberately loose: enough to catch a typo, not a validator.
fn looks_like_email(s: &str) -> bool {
    let at = s.find('@');
    match at {
        Some(i) => i > 0 && s[i + 1..].contains('.') && !s.ends_with('.') && !s.contains(' '),
        None => false,
    }
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
