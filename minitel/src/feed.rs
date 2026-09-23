//! Reads marlinski.org/minitel.json and keeps a cached copy.
//!
//! The service holds no content of its own: publish the blog and the Minitel
//! picks it up on the next refresh. If the fetch fails we keep serving the last
//! good copy rather than showing an error, because a blog outage should not
//! take the terminal down with it.

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Deserialize;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Deserialize)]
pub struct Project {
    pub name: String,
    pub blurb: String,
    pub tags: Vec<String>,
    pub status: String,
    pub years: String,
    pub url: String,
    /// The GitHub repo, where there is one. Falls back to `url` in the feed.
    pub gh: String,
}

impl Project {
    /// Candidate raw README URLs for a github.com project, most likely first.
    ///
    /// HEAD rather than a branch name, so it works whether the default branch
    /// is main or master. Several filenames because raw.githubusercontent.com
    /// is case-sensitive and not everyone shouts: Rumble's is `Readme.md`.
    pub fn readme_urls(&self) -> Vec<String> {
        let Some(rest) = self.gh.strip_prefix("https://github.com/") else {
            return Vec::new();
        };
        let mut it = rest.trim_end_matches('/').split('/');
        let (Some(owner), Some(repo)) = (it.next(), it.next()) else {
            return Vec::new();
        };
        if owner.is_empty() || repo.is_empty() {
            return Vec::new();
        }
        ["README.md", "Readme.md", "readme.md", "README.markdown", "README"]
            .iter()
            .map(|f| format!("https://raw.githubusercontent.com/{owner}/{repo}/HEAD/{f}"))
            .collect()
    }

    /// True when this project has a public repo we can read a README from.
    pub fn has_repo(&self) -> bool {
        !self.readme_urls().is_empty()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Post {
    pub title: String,
    pub date: String,
    pub tags: Vec<String>,
    pub blurb: String,
    pub url: String,
    pub html: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PublicFile {
    pub name: String,
    pub desc: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PublicItem {
    pub kind: String,
    pub title: String,
    #[serde(rename = "where")]
    pub where_at: String,
    pub date: String,
    pub url: String,
    pub files: Vec<PublicFile>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Feed {
    pub title: String,
    pub tagline: String,
    pub now: String,
    pub projects: Vec<Project>,
    pub posts: Vec<Post>,
    #[serde(default)]
    pub public: Vec<PublicItem>,
}

impl Feed {
    /// Placeholder shown only if the very first fetch fails, before any good
    /// copy exists.
    fn unavailable() -> Self {
        Feed {
            title: "MARLINSKI".into(),
            tagline: "service temporarily unavailable".into(),
            now: String::new(),
            projects: Vec::new(),
            posts: Vec::new(),
            public: Vec::new(),
        }
    }
}

/// Renders markdown into the same styled lines as a post.
///
/// A README is markdown, not HTML, so it goes through pulldown-cmark first and
/// then the identical path — one renderer, one look, whether the text came
/// from the blog or from GitHub.
pub fn md_lines(markdown: &str, width: usize, th: &crate::screen::Theme) -> Vec<Vec<Span>> {
    use pulldown_cmark::{html, Options, Parser};
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TABLES);
    let parser = Parser::new_ext(markdown, opts);
    let mut out = String::new();
    html::push_html(&mut out, parser);
    post_lines(&out, width, th)
}

pub struct FeedCache {
    url: String,
    ttl: Duration,
    inner: RwLock<(Arc<Feed>, Option<Instant>)>,
    readmes: RwLock<std::collections::HashMap<String, Option<String>>>,
    client: reqwest::Client,
}

impl FeedCache {
    pub fn new(url: String, ttl: Duration) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("3615.marlinski.org")
            .build()
            .expect("http client");
        FeedCache {
            url,
            ttl,
            inner: RwLock::new((Arc::new(Feed::unavailable()), None)),
            readmes: RwLock::new(std::collections::HashMap::new()),
            client,
        }
    }

    /// Cached feed, refreshed when the copy is older than the TTL. A failed
    /// refresh returns the stale copy instead of an error.
    pub async fn get(&self) -> Arc<Feed> {
        {
            let guard = self.inner.read().await;
            if let Some(at) = guard.1 {
                if at.elapsed() < self.ttl {
                    return guard.0.clone();
                }
            }
        }

        match self.fetch().await {
            Ok(feed) => {
                let feed = Arc::new(feed);
                let mut guard = self.inner.write().await;
                *guard = (feed.clone(), Some(Instant::now()));
                feed
            }
            Err(e) => {
                eprintln!("minitel: feed refresh failed: {e}");
                let mut guard = self.inner.write().await;
                // Back off, so a broken upstream is not hammered once per session.
                if guard.1.is_some() {
                    guard.1 = Some(Instant::now());
                }
                guard.0.clone()
            }
        }
    }

    /// Fetches a README, cached for the life of the process. Returns None on
    /// any failure — a private repo or a missing file should show a message,
    /// not an error screen.
    pub async fn readme(&self, urls: &[String]) -> Option<String> {
        let key = urls.first()?.clone();
        if let Some(hit) = self.readmes.read().await.get(&key) {
            return hit.clone();
        }
        let mut got = None;
        for url in urls {
            if let Ok(r) = self.client.get(url).send().await {
                if r.status().is_success() {
                    got = r.text().await.ok();
                    if got.is_some() {
                        break;
                    }
                }
            }
        }
        self.readmes.write().await.insert(key, got.clone());
        got
    }

    async fn fetch(&self) -> anyhow::Result<Feed> {
        let body = self
            .client
            .get(&self.url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        Ok(serde_json::from_str(&body)?)
    }
}

/// A run of text sharing one style, within a rendered line.
pub type Span = (String, crate::screen::Style);

/// Renders post HTML into wrapped, styled lines.
///
/// Jekyll hands us HTML rather than the original markdown, but the HTML is
/// still semantic, so html2text's rich mode gives back the emphasis, strong,
/// code and link spans and we colour them ourselves. Headings arrive as a
/// leading "#" run, which is the one thing the annotations do not carry.
pub fn post_lines(html: &str, width: usize, th: &crate::screen::Theme) -> Vec<Vec<Span>> {
    use crate::screen::*;
    use html2text::render::RichAnnotation as A;

    let width = width.clamp(20, 200);
    let lines = match html2text::from_read_rich(html.as_bytes(), width) {
        Ok(l) => l,
        Err(_) => return vec![vec![("[unreadable content]".to_string(), th.on(RED))]],
    };

    let mut out = Vec::new();
    for line in lines {
        let mut spans: Vec<Span> = Vec::new();
        for ts in line.tagged_strings() {
            let mut style = th.base();
            for ann in &ts.tag {
                style = match ann {
                    A::Strong => th.strong(th.fg),
                    A::Emphasis => Style { italic: true, ..th.on(th.accent) },
                    A::Code | A::Preformat(_) => th.on(th.code),
                    A::Link(_) => Style { underline: true, ..th.on(th.link) },
                    A::Strikeout => th.on(th.dim),
                    _ => style,
                };
            }
            spans.push((ts.s.clone(), style));
        }

        // Headings: html2text prefixes them with #, ##, ### and no annotation.
        // Colour the whole line by level and drop the markers, the way a
        // markdown viewer would.
        let flat: String = spans.iter().map(|(t, _)| t.as_str()).collect();
        let hashes = flat.chars().take_while(|c| *c == '#').count();
        if hashes > 0 && flat.chars().nth(hashes) == Some(' ') {
            let title = flat[hashes + 1..].to_string();
            let style = match hashes {
                1 => th.strong(th.heading),
                2 => th.strong(th.code),
                _ => th.strong(th.accent),
            };
            out.push(vec![(title, style)]);
            continue;
        }

        out.push(spans);
    }
    out
}
