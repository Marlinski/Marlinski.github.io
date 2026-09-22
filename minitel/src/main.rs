//! marlinski.org, served over SSH as a Minitel.
//!
//! This is not a shell and not a VM: it speaks the SSH protocol and paints a
//! terminal UI, nothing more. It also owns no content — everything comes from
//! marlinski.org/minitel.json, which Jekyll generates from the same markdown
//! the website is built from. Publish a post and it appears here on the next
//! refresh, with no redeploy.

mod app;
mod cmd;
mod feed;
mod screen;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use russh::keys::PrivateKey;
use russh::server::{Auth, Handler, Msg, Server as _, Session};
use russh::{Channel, ChannelId};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

use app::{decode, App};
use feed::FeedCache;
use screen::*;

/// Characters per second while painting. A Minitel's 1200 baud line managed
/// about 120, which is authentic and unbearable; this is the same idea with
/// the patience dialled out.
const CPS: u64 = 1400;
/// Cells per write. One write per character would be faithful and would also
/// bury the network in tiny packets.
const CHUNK: usize = 6;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port: u16 = std::env::var("MINITEL_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2222);
    let feed_url = std::env::var("MINITEL_FEED")
        .unwrap_or_else(|_| "https://marlinski.org/minitel.json".to_string());
    let key_path =
        std::env::var("MINITEL_HOST_KEY").unwrap_or_else(|_| "/data/host_key".to_string());
    // Supplied by a Kubernetes Secret. Keeping the key out of the image means
    // the image can be public; keeping it in the environment rather than on a
    // volume means the fingerprint is the same wherever this runs.
    let key_pem = std::env::var("MINITEL_HOST_KEY_PEM").ok().filter(|v| !v.trim().is_empty());

    let cache = Arc::new(FeedCache::new(feed_url.clone(), Duration::from_secs(300)));
    // Warm the cache so the first visitor does not wait on an HTTP round trip.
    let _ = cache.get().await;

    let key = match key_pem {
        Some(pem) => {
            eprintln!("minitel: host key from MINITEL_HOST_KEY_PEM");
            PrivateKey::from_openssh(pem.as_bytes())?
        }
        None => load_or_create_host_key(&key_path).await?,
    };

    let config = Arc::new(russh::server::Config {
        inactivity_timeout: Some(Duration::from_secs(600)),
        auth_rejection_time: Duration::from_secs(1),
        keys: vec![key],
        ..Default::default()
    });

    let mut server = Server { cache };
    let socket = TcpListener::bind(("0.0.0.0", port)).await?;
    eprintln!("minitel: listening on 0.0.0.0:{port}, feed {feed_url}");
    server.run_on_socket(config, &socket).await?;
    Ok(())
}

/// Fallback when MINITEL_HOST_KEY_PEM is not set: keep a key on disk. Either
/// way the point is that the fingerprint never changes, because a new one on
/// every restart means every visitor gets a man-in-the-middle warning from
/// their own known_hosts.
async fn load_or_create_host_key(path: &str) -> anyhow::Result<PrivateKey> {
    match tokio::fs::read_to_string(path).await {
        Ok(pem) => {
            eprintln!("minitel: host key loaded from {path}");
            Ok(PrivateKey::from_openssh(pem.as_bytes())?)
        }
        Err(_) => {
            let key = PrivateKey::random(&mut rand::rng(), russh::keys::Algorithm::Ed25519)?;
            let pem = key.to_openssh(russh::keys::ssh_key::LineEnding::LF)?;
            if let Some(dir) = std::path::Path::new(path).parent() {
                let _ = tokio::fs::create_dir_all(dir).await;
            }
            match tokio::fs::write(path, pem.as_bytes()).await {
                Ok(_) => eprintln!("minitel: new host key written to {path}"),
                Err(e) => eprintln!("minitel: WARNING could not persist host key ({e}); it will change on restart"),
            }
            Ok(key)
        }
    }
}

#[derive(Clone)]
struct Server {
    cache: Arc<FeedCache>,
}

impl russh::server::Server for Server {
    type Handler = Client;
    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Client {
        Client {
            cache: self.cache.clone(),
            app: Arc::new(Mutex::new(None)),
            size: Arc::new(Mutex::new((80, 24))),
            pending: Arc::new(Mutex::new(Vec::new())),
            pty: Arc::new(AtomicBool::new(false)),
            generation: Arc::new(AtomicU64::new(0)),
            skip: Arc::new(AtomicBool::new(false)),
        }
    }
    fn handle_session_error(&mut self, error: russh::Error) {
        // Clients hanging up mid-paint is the normal way a session ends.
        let msg = format!("{error}");
        if !msg.contains("EOF") && !msg.contains("early eof") {
            eprintln!("minitel: session error: {msg}");
        }
    }
}

struct Client {
    cache: Arc<FeedCache>,
    app: Arc<Mutex<Option<App>>>,
    size: Arc<Mutex<(usize, usize)>>,
    /// Bytes of a half-delivered escape sequence, waiting for the rest.
    pending: Arc<Mutex<Vec<u8>>>,
    pty: Arc<AtomicBool>,
    generation: Arc<AtomicU64>,
    skip: Arc<AtomicBool>,
}

impl Client {
    /// Repaints the screen, cancelling any paint still in flight.
    async fn repaint(&self, handle: russh::server::Handle, channel: ChannelId) {
        let screen = {
            let guard = self.app.lock().await;
            match guard.as_ref() {
                Some(a) => a.render(),
                None => return,
            }
        };
        let slow = {
            let guard = self.app.lock().await;
            guard.as_ref().map(|a| a.slow).unwrap_or(false)
        };

        let gen = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        self.skip.store(false, Ordering::SeqCst);
        let generation = self.generation.clone();
        let skip = self.skip.clone();

        tokio::spawn(async move {
            let mut out = String::from(CLEAR);
            out.push_str(HIDE_CURSOR);
            if handle
                .data(channel, out.clone().into_bytes())
                .await
                .is_err()
            {
                return;
            }

            let mut buf = String::new();
            let mut pending = 0usize;
            for (x, y, style, text) in screen.runs() {
                buf.push_str(&goto(x, y));
                buf.push_str(&sgr(style));
                for ch in text.chars() {
                    buf.push(ch);
                    pending += 1;
                    if pending >= CHUNK && slow && !skip.load(Ordering::Relaxed) {
                        if generation.load(Ordering::SeqCst) != gen {
                            return; // a newer paint took over
                        }
                        if handle
                            .data(channel, buf.clone().into_bytes())
                            .await
                            .is_err()
                        {
                            return;
                        }
                        buf.clear();
                        pending = 0;
                        tokio::time::sleep(Duration::from_micros(
                            1_000_000 * CHUNK as u64 / CPS,
                        ))
                        .await;
                    }
                }
            }
            if generation.load(Ordering::SeqCst) != gen {
                return;
            }
            buf.push_str(RESET);
            let _ = handle
                .data(channel, buf.clone().into_bytes())
                .await;
        });
    }
}

impl Handler for Client {
    type Error = russh::Error;

    async fn channel_open_session(
        &mut self,
        _channel: Channel<Msg>,
        reply: russh::server::ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        reply.accept().await;
        Ok(())
    }

    // Open house: no credentials, any key, anyone.
    async fn auth_none(&mut self, _user: &str) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn auth_publickey(
        &mut self,
        _user: &str,
        _key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn auth_password(&mut self, _user: &str, _pw: &str) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn pty_request(
        &mut self,
        _channel: ChannelId,
        _term: &str,
        col_width: u32,
        row_height: u32,
        _: u32,
        _: u32,
        _: &[(russh::Pty, u32)],
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        let w = (col_width as usize).clamp(40, 200);
        let h = (row_height as usize).clamp(12, 80);
        *self.size.lock().await = (w, h);
        self.pty.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn window_change_request(
        &mut self,
        channel: ChannelId,
        col_width: u32,
        row_height: u32,
        _: u32,
        _: u32,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let w = (col_width as usize).clamp(40, 200);
        let h = (row_height as usize).clamp(12, 80);
        *self.size.lock().await = (w, h);
        if let Some(a) = self.app.lock().await.as_mut() {
            a.resize(w, h);
        }
        self.repaint(session.handle(), channel).await;
        Ok(())
    }

    async fn shell_request(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let (w, h) = *self.size.lock().await;
        let feed = self.cache.get().await;
        *self.app.lock().await = Some(App::new(feed, w, h));
        self.repaint(session.handle(), channel).await;
        Ok(())
    }

    /// `ssh marlinski.org blog`, `ssh marlinski.org help`, and so on: answer
    /// on stdout and hang up, the way any other command would.
    async fn exec_request(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let line = String::from_utf8_lossy(data).to_string();
        let feed = self.cache.get().await;
        let color = self.pty.load(Ordering::SeqCst);
        let width = self.size.lock().await.0.clamp(40, 100);
        let out = cmd::run(&feed, &line, color, width.saturating_sub(4));
        session.data(channel, out.into_bytes())?;
        session.exit_status_request(channel, 0)?;
        session.eof(channel)?;
        session.close(channel)?;
        Ok(())
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let keys = {
            let mut pend = self.pending.lock().await;
            let mut buf = std::mem::take(&mut *pend);
            buf.extend_from_slice(data);
            let (keys, leftover) = decode(&buf);
            *pend = leftover;
            keys
        };
        if keys.is_empty() {
            return Ok(());
        }
        // Any key during a paint finishes it immediately — waiting out the
        // animation to press a key would be faithful and infuriating.
        self.skip.store(true, Ordering::SeqCst);

        let mut dirty = false;
        let mut quit = false;
        {
            let mut guard = self.app.lock().await;
            if let Some(a) = guard.as_mut() {
                for k in keys {
                    if a.key(k) {
                        dirty = true;
                    }
                    if a.quit {
                        quit = true;
                        break;
                    }
                }
            }
        }

        if quit {
            let bye = format!(
                "{RESET}{SHOW_CURSOR}\r\n  goodbye — https://marlinski.org\r\n\r\n"
            );
            let _ = session.data(channel, bye.into_bytes());
            session.close(channel)?;
            return Ok(());
        }
        if dirty {
            self.repaint(session.handle(), channel).await;
        }

        // A README was requested. Paint first so the "fetching" line shows,
        // then fetch and repaint — the alternative is a frozen screen for the
        // length of an HTTP round trip.
        let want = {
            let mut guard = self.app.lock().await;
            guard.as_mut().and_then(|a| a.want_readme.take())
        };
        if let Some((_idx, urls)) = want {
            let md = self.cache.readme(&urls).await;
            {
                let mut guard = self.app.lock().await;
                if let Some(a) = guard.as_mut() {
                    a.set_readme(md);
                }
            }
            self.repaint(session.handle(), channel).await;
        }
        Ok(())
    }
}
