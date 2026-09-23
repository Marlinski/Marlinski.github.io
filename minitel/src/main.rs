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
mod proxy;
mod screen;
mod store;
mod web;

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
use store::Store;
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

    let db_path = std::env::var("MINITEL_DB").unwrap_or_else(|_| "/data/minitel.sqlite3".to_string());
    let store = Store::open(&db_path)?;
    eprintln!("minitel: mailbox at {db_path}");

    // The owner's public key, in authorized_keys form. Public by definition, so
    // it lives in the manifest rather than the Secret.
    let admin_key = std::env::var("MINITEL_ADMIN_KEY")
        .ok()
        .and_then(|v| parse_admin_key(&v));
    if admin_key.is_some() {
        eprintln!("minitel: admin key configured");
    }

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

    // The landing page rides along in this process: one binary, one pod, and
    // a second listener rather than a whole deployment to serve one page.
    let http_port: u16 = std::env::var("MINITEL_HTTP_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8080);
    tokio::spawn(web::serve(http_port));

    // Set when the router in front of this pod speaks PROXY protocol. Without
    // it every visitor's address is the router's own, which is worth recording
    // exactly never.
    let proxied = std::env::var("MINITEL_PROXY_PROTOCOL")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let mut server = Server { cache, store, admin_key: admin_key.map(Arc::new) };
    let socket = TcpListener::bind(("0.0.0.0", port)).await?;
    eprintln!(
        "minitel: listening on 0.0.0.0:{port}, feed {feed_url}{}",
        if proxied { ", behind PROXY protocol" } else { "" }
    );

    // Hand-rolled accept loop rather than run_on_socket, because the PROXY
    // header has to come off the stream before russh sees it.
    loop {
        let (mut stream, addr) = match socket.accept().await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("minitel: accept failed: {e}");
                continue;
            }
        };
        let config = config.clone();
        let handler = {
            let peer = if proxied { None } else { Some(addr) };
            server.new_client(peer)
        };
        tokio::spawn(async move {
            let mut handler = handler;
            if proxied {
                match proxy::read_header(&mut stream).await {
                    Ok(peer) => handler.peer = peer,
                    Err(e) => {
                        eprintln!("minitel: PROXY header from {addr}: {e}");
                        return;
                    }
                }
            }
            if let Err(e) = russh::server::run_stream(config, stream, handler).await {
                if !benign(&e) {
                    eprintln!("minitel: session error: {e}");
                }
            }
        });
    }
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

/// Strips a visitor-supplied name down to something safe to print.
///
/// The username arrives from the client and is stored, logged and shown in
/// the inbox. OpenSSH's own client refuses to send a name containing control
/// characters, but the protocol allows it and a hand-written client will:
/// left as-is, `ssh $'\e]0;...'@host` puts an escape sequence in the owner's
/// terminal the next time they read their mail. Length is capped for the same
/// reason a field is — it is displayed, not parsed.
fn clean_name(user: &str) -> String {
    user.chars().filter(|c| !c.is_control()).take(64).collect()
}

/// Reduces an authorized_keys line to the comparable key material.
fn parse_admin_key(line: &str) -> Option<russh::keys::ssh_key::PublicKey> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    match russh::keys::ssh_key::PublicKey::from_openssh(line) {
        Ok(k) => Some(k),
        Err(e) => {
            eprintln!("minitel: MINITEL_ADMIN_KEY unreadable: {e}");
            None
        }
    }
}

/// A connection that ends by going away. The health probes open a socket and
/// drop it on a timer, and clients close mid-paint all the time; neither is
/// worth a line in the log.
fn benign(e: &russh::Error) -> bool {
    let msg = format!("{e}");
    msg.contains("EOF")
        || msg.contains("early eof")
        || msg.contains("Disconnected")
        || msg.contains("reset by peer")
        || msg.contains("Broken pipe")
        || msg.contains("Connection closed")
}

#[derive(Clone)]
struct Server {
    cache: Arc<FeedCache>,
    store: Store,
    admin_key: Option<Arc<russh::keys::ssh_key::PublicKey>>,
}

impl russh::server::Server for Server {
    type Handler = Client;
    fn new_client(&mut self, peer: Option<std::net::SocketAddr>) -> Client {
        Client {
            peer,
            cache: self.cache.clone(),
            store: self.store.clone(),
            admin_key: self.admin_key.clone(),
            admin: Arc::new(AtomicBool::new(false)),
            user: Arc::new(Mutex::new(String::new())),
            pubkey: Arc::new(Mutex::new(String::new())),
            app: Arc::new(Mutex::new(None)),
            size: Arc::new(Mutex::new((80, 24))),
            pending: Arc::new(Mutex::new(Vec::new())),
            pty: Arc::new(AtomicBool::new(false)),
            generation: Arc::new(AtomicU64::new(0)),
            skip: Arc::new(AtomicBool::new(false)),
        }
    }
    fn handle_session_error(&mut self, error: russh::Error) {
        if !benign(&error) {
            eprintln!("minitel: session error: {error}");
        }
    }
}

struct Client {
    /// Where the visitor is connecting from, once the router has told us.
    peer: Option<std::net::SocketAddr>,
    cache: Arc<FeedCache>,
    store: Store,
    admin_key: Option<Arc<russh::keys::ssh_key::PublicKey>>,
    admin: Arc<AtomicBool>,
    user: Arc<Mutex<String>>,
    /// The key the visitor authenticated with, in authorized_keys form.
    pubkey: Arc<Mutex<String>>,
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

impl Client {
    fn hang_up(&self, channel: ChannelId, session: &mut Session) -> Result<(), russh::Error> {
        let bye = format!("{RESET}{SHOW_CURSOR}\r\n  goodbye — https://marlinski.org\r\n\r\n");
        let _ = session.data(channel, bye.into_bytes());
        session.close(channel)
    }

    /// Carries out whatever the state machine asked for: fetch a README, save a
    /// message, load the mailbox. It is split out because App::key is sync and
    /// these all need to await.
    async fn pump(&self, channel: ChannelId, session: &mut Session) {
        // Paint first so "fetching" is visible, then do the slow thing.
        let (readme, send, inbox, mark, del, email) = {
            let mut guard = self.app.lock().await;
            match guard.as_mut() {
                Some(a) => (
                    a.want_readme.take(),
                    a.want_send.take(),
                    std::mem::replace(&mut a.want_inbox, false),
                    a.want_mark_read.take(),
                    a.want_delete.take(),
                    a.email.trim().to_string(),
                ),
                None => (None, None, false, None, None, String::new()),
            }
        };

        if let Some((_idx, urls)) = readme {
            let md = self.cache.readme(&urls).await;
            if let Some(a) = self.app.lock().await.as_mut() {
                a.set_readme(md);
            }
            self.repaint(session.handle(), channel).await;
        }

        if let Some(body) = send {
            let who = self.user.lock().await.clone();
            let key = self.pubkey.lock().await.clone();
            let ip = self.peer.map(|a| a.ip().to_string()).unwrap_or_default();
            if let Err(e) = self.store.add(who, email, key, ip, body).await {
                eprintln!("minitel: could not save message: {e}");
            }
        }

        if let Some(id) = del {
            self.store.delete(id).await;
            let msgs = self.store.list().await;
            if let Some(a) = self.app.lock().await.as_mut() {
                a.set_inbox(msgs);
            }
            self.repaint(session.handle(), channel).await;
        }

        if let Some(id) = mark {
            self.store.mark_read(id).await;
        }

        if inbox {
            let msgs = self.store.list().await;
            if let Some(a) = self.app.lock().await.as_mut() {
                a.set_inbox(msgs);
            }
            self.repaint(session.handle(), channel).await;
        }
    }
}

impl Handler for Client {
    type Error = russh::Error;

    /// One line per visitor who gets as far as opening a session. Logged here
    /// rather than on accept, because the probes that keep this pod alive open
    /// a TCP connection every few seconds and never say anything.
    async fn channel_open_session(
        &mut self,
        _channel: Channel<Msg>,
        reply: russh::server::ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        let who = self.user.lock().await.clone();
        println!(
            "minitel: session from {} as {}{}",
            self.peer.map(|a| a.ip().to_string()).unwrap_or_else(|| "unknown".into()),
            if who.is_empty() { "-" } else { &who },
            if self.admin.load(Ordering::SeqCst) { " (owner)" } else { "" }
        );
        reply.accept().await;
        Ok(())
    }

    // Open house: no credentials, any key, anyone.
    /// Anyone may enter, but the owner has to be recognised, and a client that
    /// gets `none` accepted immediately never offers a key. So the first `none`
    /// is rejected asking for publickey; if the visitor has no key their client
    /// comes back to `none` and is let in.
    async fn auth_none(&mut self, user: &str) -> Result<Auth, Self::Error> {
        *self.user.lock().await = clean_name(user);
        // OpenSSH will not retry a method it has already failed, so the fallback
        // cannot be `none` again: it is keyboard-interactive, answered with zero
        // prompts, which the client completes without asking the visitor
        // anything.
        Ok(Auth::Reject {
            proceed_with_methods: Some(russh::MethodSet::from(
                &[
                    russh::MethodKind::PublicKey,
                    russh::MethodKind::KeyboardInteractive,
                ][..],
            )),
            partial_success: false,
        })
    }

    async fn auth_publickey(
        &mut self,
        user: &str,
        key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<Auth, Self::Error> {
        *self.user.lock().await = clean_name(user);
        if let Ok(line) = key.to_openssh() {
            *self.pubkey.lock().await = line;
        }
        if let Some(admin) = &self.admin_key {
            if key.key_data() == admin.key_data() {
                self.admin.store(true, Ordering::SeqCst);
                eprintln!("minitel: owner connected as {}", clean_name(user));
            }
        }
        Ok(Auth::Accept)
    }

    /// The keyless path in: no prompts, so nothing is asked of the visitor.
    async fn auth_keyboard_interactive(
        &mut self,
        user: &str,
        _submethods: &str,
        _response: Option<russh::server::Response<'_>>,
    ) -> Result<Auth, Self::Error> {
        *self.user.lock().await = clean_name(user);
        Ok(Auth::Accept)
    }

    async fn auth_password(&mut self, user: &str, _pw: &str) -> Result<Auth, Self::Error> {
        *self.user.lock().await = clean_name(user);
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
        let mut app = App::new(feed, w, h);
        app.admin = self.admin.load(Ordering::SeqCst);
        app.user = self.user.lock().await.clone();
        app.pubkey = self.pubkey.lock().await.clone();
        *self.app.lock().await = Some(app);
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
        // While the compose box has the keyboard, bytes are text, not commands.
        let editing = {
            let guard = self.app.lock().await;
            guard.as_ref().map(|a| a.editing()).unwrap_or(false)
        };
        if editing {
            let (dirty, quit) = {
                let mut guard = self.app.lock().await;
                match guard.as_mut() {
                    Some(a) => (a.input(data), a.quit),
                    None => (false, false),
                }
            };
            if quit {
                return self.hang_up(channel, session);
            }
            if dirty {
                self.repaint(session.handle(), channel).await;
            }
            self.pump(channel, session).await;
            return Ok(());
        }

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
            return self.hang_up(channel, session);
        }
        if dirty {
            self.repaint(session.handle(), channel).await;
        }
        self.pump(channel, session).await;
        Ok(())
    }
}
