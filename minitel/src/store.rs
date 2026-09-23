//! The mailbox: visitors leave a message, the owner reads it.
//!
//! SQLite on a small volume. Every call goes through spawn_blocking because
//! rusqlite is synchronous and this runs on an async runtime — the writes are
//! microseconds, but blocking the reactor on disk is the kind of thing that
//! only hurts once everyone is connected at the same time.

use std::sync::{Arc, Mutex};

use rusqlite::{params, Connection};

/// Roughly 15MB of messages on a 100MB volume. The mailbox filling up should
/// be something the owner notices, not something a script decides.
const MAX_MESSAGES: i64 = 20_000;

#[derive(Debug, Clone)]
pub struct Message {
    pub id: i64,
    pub who: String,
    pub email: String,
    /// The public key the sender authenticated with, if any. Not identity, but
    /// it does tell two messages apart and links repeat visitors.
    pub pubkey: String,
    /// Where the message was sent from. Empty when the router did not say,
    /// which is every message sent before this was wired up.
    pub ip: String,
    pub body: String,
    pub at: String,
    pub read: bool,
}

#[derive(Clone)]
pub struct Store {
    conn: Arc<Mutex<Connection>>,
}

impl Store {
    pub fn open(path: &str) -> anyhow::Result<Store> {
        if let Some(dir) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let conn = Connection::open(path)?;
        // WAL so a reader never blocks the writer; this is a guestbook, but the
        // default journal mode would serialise the whole thing on one lock.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS messages (
                 id    INTEGER PRIMARY KEY AUTOINCREMENT,
                 who    TEXT NOT NULL,
                 email  TEXT NOT NULL DEFAULT '',
                 pubkey TEXT NOT NULL DEFAULT '',
                 body   TEXT NOT NULL,
                 at     TEXT NOT NULL,
                 read   INTEGER NOT NULL DEFAULT 0
             );",
        )?;
        // Added after the table already existed on the volume; the error when
        // the column is already there is the expected outcome on every start
        // but the first.
        let _ = conn.execute("ALTER TABLE messages ADD COLUMN ip TEXT NOT NULL DEFAULT ''", []);
        Ok(Store { conn: Arc::new(Mutex::new(conn)) })
    }

    pub async fn add(
        &self,
        who: String,
        email: String,
        pubkey: String,
        ip: String,
        body: String,
    ) -> anyhow::Result<()> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let c = conn.lock().unwrap();
            let n: i64 = c.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))?;
            if n >= MAX_MESSAGES {
                anyhow::bail!("mailbox full");
            }
            c.execute(
                "INSERT INTO messages (who, email, pubkey, ip, body, at)
                 VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))",
                params![who, email, pubkey, ip, body],
            )?;
            Ok(())
        })
        .await??;
        Ok(())
    }

    pub async fn delete(&self, id: i64) {
        let conn = self.conn.clone();
        let _ = tokio::task::spawn_blocking(move || {
            if let Ok(c) = conn.lock() {
                let _ = c.execute("DELETE FROM messages WHERE id = ?1", params![id]);
            }
        })
        .await;
    }

    /// Newest first, unread before read, so the inbox opens on what matters.
    pub async fn list(&self) -> Vec<Message> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let c = match conn.lock() {
                Ok(c) => c,
                Err(_) => return Vec::new(),
            };
            let mut stmt = match c.prepare(
                "SELECT id, who, email, pubkey, ip, body, at, read FROM messages
                 ORDER BY read ASC, id DESC LIMIT 500",
            ) {
                Ok(s) => s,
                Err(_) => return Vec::new(),
            };
            let rows = stmt.query_map([], |r| {
                Ok(Message {
                    id: r.get(0)?,
                    who: r.get(1)?,
                    email: r.get(2)?,
                    pubkey: r.get(3)?,
                    ip: r.get(4)?,
                    body: r.get(5)?,
                    at: r.get(6)?,
                    read: r.get::<_, i64>(7)? != 0,
                })
            });
            match rows {
                Ok(it) => it.filter_map(|r| r.ok()).collect(),
                Err(_) => Vec::new(),
            }
        })
        .await
        .unwrap_or_default()
    }

    pub async fn mark_read(&self, id: i64) {
        let conn = self.conn.clone();
        let _ = tokio::task::spawn_blocking(move || {
            if let Ok(c) = conn.lock() {
                let _ = c.execute("UPDATE messages SET read = 1 WHERE id = ?1", params![id]);
            }
        })
        .await;
    }

    pub async fn unread(&self) -> usize {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            conn.lock()
                .ok()
                .and_then(|c| {
                    c.query_row("SELECT COUNT(*) FROM messages WHERE read = 0", [], |r| {
                        r.get::<_, i64>(0)
                    })
                    .ok()
                })
                .unwrap_or(0) as usize
        })
        .await
        .unwrap_or(0)
    }
}
