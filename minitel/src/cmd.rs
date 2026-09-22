//! Non-interactive mode: `ssh marlinski.org blog`, `ssh marlinski.org help`.
//!
//! Output is plain lines, so it pipes. Colour is only emitted when the client
//! asked for a PTY (`ssh -t`), because nobody wants escape codes in a grep.

use std::sync::Arc;

use crate::feed::{post_lines, Feed};

pub struct Out {
    lines: Vec<String>,
    color: bool,
}

impl Out {
    fn new(color: bool) -> Self {
        Out { lines: Vec::new(), color }
    }
    fn raw(&mut self, s: impl Into<String>) {
        self.lines.push(s.into());
    }
    fn c(&mut self, code: &str, s: &str) {
        if self.color {
            self.lines.push(format!("\x1b[{code}m{s}\x1b[0m"));
        } else {
            self.lines.push(s.to_string());
        }
    }
    fn blank(&mut self) {
        self.lines.push(String::new());
    }
    /// SSH channels are not terminals in cooked mode, so lines need CRLF.
    pub fn finish(self) -> String {
        let mut s = self.lines.join("\r\n");
        s.push_str("\r\n");
        s
    }
}

pub fn run(feed: &Arc<Feed>, cmdline: &str, color: bool, width: usize) -> String {
    let mut parts = cmdline.split_whitespace();
    let verb = parts.next().unwrap_or("help").to_lowercase();
    let arg = parts.next().unwrap_or("");
    let mut o = Out::new(color);

    match verb.as_str() {
        "blog" | "posts" | "writing" if arg.is_empty() => list_posts(feed, &mut o),
        "blog" | "posts" | "writing" | "read" | "post" | "cat" => {
            match resolve(feed, arg) {
                Some(i) => show_post(feed, i, &mut o, width),
                None => {
                    o.c("31", &format!("no such post: {arg}"));
                    o.blank();
                    list_posts(feed, &mut o);
                }
            }
        }
        "projects" | "proj" => list_projects(feed, &mut o),
        "about" | "whoami" => about(feed, &mut o),
        "now" => o.raw(feed.now.clone()),
        "feed" | "json" => o.raw("https://marlinski.org/minitel.json"),
        "help" | "-h" | "--help" | "?" => help(&mut o),
        other => {
            o.c("31", &format!("unknown command: {other}"));
            o.blank();
            help(&mut o);
        }
    }
    o.finish()
}

/// Accepts either a 1-based index or a substring of the title/slug.
fn resolve(feed: &Arc<Feed>, arg: &str) -> Option<usize> {
    if arg.is_empty() {
        return None;
    }
    if let Ok(n) = arg.parse::<usize>() {
        if n >= 1 && n <= feed.posts.len() {
            return Some(n - 1);
        }
        return None;
    }
    let needle = arg.to_lowercase();
    feed.posts
        .iter()
        .position(|p| p.title.to_lowercase().contains(&needle) || p.url.to_lowercase().contains(&needle))
}

fn list_posts(feed: &Arc<Feed>, o: &mut Out) {
    o.c("1;33", "WRITING");
    o.blank();
    for (i, p) in feed.posts.iter().enumerate() {
        o.raw(format!("  {:>2}  {}  {}", i + 1, p.date, p.title));
        if !p.blurb.is_empty() {
            o.c("34", &format!("      {}", p.blurb));
        }
    }
    o.blank();
    o.c("34", "  ssh minitel.marlinski.org blog <n>   to read one");
}

fn show_post(feed: &Arc<Feed>, idx: usize, o: &mut Out, width: usize) {
    let p = &feed.posts[idx];
    o.c("1;33", &p.title);
    let tags = p.tags.iter().map(|t| format!("#{t}")).collect::<Vec<_>>().join(" ");
    o.c("34", &format!("{}  {}", p.date, tags));
    o.blank();
    // Exec output stays plain: the styling belongs to the interactive screen,
    // and this is meant to survive a pipe.
    // Exec output is flattened to plain text, so the theme here is irrelevant.
    let th = crate::screen::Theme::get(crate::screen::ThemeKind::Dark);
    for line in post_lines(&p.html, width, &th) {
        let text: String = line.iter().map(|(t, _)| t.as_str()).collect();
        o.raw(text.trim_end().to_string());
    }
    o.blank();
    o.c("35", &p.url);
}

fn list_projects(feed: &Arc<Feed>, o: &mut Out) {
    o.c("1;33", "PROJECTS");
    o.blank();
    for p in &feed.projects {
        let mark = match p.status.as_str() {
            "active" => "*",
            "idle" => "-",
            "archived" => "o",
            _ => " ",
        };
        let years = if p.years.is_empty() { String::new() } else { format!(" ({})", p.years) };
        o.raw(format!("  {}  {:<16}{}{}", mark, p.name, p.blurb, years));
        if !p.url.is_empty() {
            o.c("34", &format!("     {}", p.url));
        }
    }
    o.blank();
    o.c("34", "  * active   - idle   o archived");
}

fn about(feed: &Arc<Feed>, o: &mut Out) {
    o.c("1;33", &feed.title.to_uppercase());
    o.raw(feed.tagline.clone());
    o.blank();
    if !feed.now.is_empty() {
        o.c("32", &format!("now  {}", feed.now));
        o.blank();
    }
    o.raw("web   https://marlinski.org".to_string());
    o.raw("code  https://github.com/Marlinski".to_string());
}

fn help(o: &mut Out) {
    o.c("1;33", "3615 MARLINSKI");
    o.c("34", "a Minitel for marlinski.org, served over SSH");
    o.blank();
    o.raw("  ssh minitel.marlinski.org         the Minitel itself, interactive");
    o.blank();
    o.c("1", "  commands");
    o.raw("  blog                              list the posts");
    o.raw("  blog <n|word>                     read one, by number or title");
    o.raw("  projects                          list the projects");
    o.raw("  about                             who, where, what");
    o.raw("  now                               what I am working on");
    o.raw("  feed                              the JSON feed this reads");
    o.raw("  help                              this screen");
    o.blank();
    o.c("34", "  ssh -t minitel.marlinski.org blog  add -t for colour");
}
