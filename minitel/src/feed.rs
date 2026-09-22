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
pub struct Feed {
    pub title: String,
    pub tagline: String,
    pub now: String,
    pub projects: Vec<Project>,
    pub posts: Vec<Post>,
}

impl Feed {
    /// Placeholder shown only if the very first fetch fails, before any good
    /// copy exists.
    fn unavailable() -> Self {
        Feed {
            title: "MARLINSKI".into(),
            tagline: "service temporairement indisponible".into(),
            now: String::new(),
            projects: Vec::new(),
            posts: Vec::new(),
        }
    }
}

pub struct FeedCache {
    url: String,
    ttl: Duration,
    inner: RwLock<(Arc<Feed>, Option<Instant>)>,
    client: reqwest::Client,
}

impl FeedCache {
    pub fn new(url: String, ttl: Duration) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("minitel.marlinski.org")
            .build()
            .expect("http client");
        FeedCache {
            url,
            ttl,
            inner: RwLock::new((Arc::new(Feed::unavailable()), None)),
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

/// Rendered post HTML into wrapped plain text.
pub fn post_text(html: &str, width: usize) -> Vec<String> {
    let width = width.clamp(20, 200);
    let text = html2text::from_read(html.as_bytes(), width)
        .unwrap_or_else(|_| "[contenu illisible]".to_string());
    text.lines().map(|l| l.trim_end().to_string()).collect()
}
