//! What the Minitel tells Prometheus about itself.
//!
//! Counters only go up and gauges read the present, which is the whole
//! contract: the scraper takes a sample every thirty seconds and Grafana
//! works out the rates. Nothing here is labelled by address — a label per
//! visitor would multiply the series indefinitely, which is how a small
//! service quietly becomes a large Prometheus. Distinct visitors are counted
//! into a set instead and reported as one number.

use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Mutex;

/// Enough addresses for a day of being linked from somewhere busy, and a
/// ceiling so that being flooded costs memory once rather than without end.
const MAX_VISITORS: usize = 50_000;

#[derive(Default)]
pub struct Metrics {
    pub shells: AtomicU64,
    pub execs: AtomicU64,
    pub active: AtomicI64,
    pub messages: AtomicU64,
    pub readme_ok: AtomicU64,
    pub readme_fail: AtomicU64,
    pub feed_fail: AtomicU64,
    visitors: Mutex<Visitors>,
}

#[derive(Default)]
struct Visitors {
    /// UTC date the set belongs to, as YYYY-MM-DD.
    day: String,
    seen: HashSet<IpAddr>,
    /// Kept after the day rolls over, so a dashboard reading "yesterday" does
    /// not see the count fall to zero at midnight and call it an outage.
    yesterday: usize,
}

impl Metrics {
    /// Records a visitor, rolling the set over at UTC midnight.
    pub fn saw(&self, ip: Option<IpAddr>) {
        let Some(ip) = ip else { return };
        let today = utc_date();
        let Ok(mut v) = self.visitors.lock() else { return };
        if v.day != today {
            v.yesterday = v.seen.len();
            v.seen.clear();
            v.day = today;
        }
        if v.seen.len() < MAX_VISITORS {
            v.seen.insert(ip);
        }
    }

    fn visitor_counts(&self) -> (usize, usize) {
        match self.visitors.lock() {
            Ok(v) if v.day == utc_date() => (v.seen.len(), v.yesterday),
            // Past midnight with nobody having connected yet: today is zero
            // and what the set still holds belongs to yesterday.
            Ok(v) => (0, v.seen.len()),
            Err(_) => (0, 0),
        }
    }

    /// The Prometheus text exposition format, which is plain enough to write
    /// by hand and saves a dependency for the seven numbers this has.
    pub fn render(&self, mailbox: i64) -> String {
        let (today, yesterday) = self.visitor_counts();
        let mut s = String::new();

        s.push_str("# HELP minitel_sessions_total SSH sessions served, by kind.\n");
        s.push_str("# TYPE minitel_sessions_total counter\n");
        s.push_str(&format!(
            "minitel_sessions_total{{kind=\"interactive\"}} {}\n",
            self.shells.load(Ordering::Relaxed)
        ));
        s.push_str(&format!(
            "minitel_sessions_total{{kind=\"exec\"}} {}\n",
            self.execs.load(Ordering::Relaxed)
        ));

        s.push_str("# HELP minitel_sessions_active Interactive sessions connected right now.\n");
        s.push_str("# TYPE minitel_sessions_active gauge\n");
        s.push_str(&format!(
            "minitel_sessions_active {}\n",
            self.active.load(Ordering::Relaxed).max(0)
        ));

        s.push_str("# HELP minitel_visitors Distinct addresses seen, by UTC day.\n");
        s.push_str("# TYPE minitel_visitors gauge\n");
        s.push_str(&format!("minitel_visitors{{day=\"today\"}} {today}\n"));
        s.push_str(&format!("minitel_visitors{{day=\"yesterday\"}} {yesterday}\n"));

        s.push_str("# HELP minitel_messages_total Messages left in the mailbox.\n");
        s.push_str("# TYPE minitel_messages_total counter\n");
        s.push_str(&format!(
            "minitel_messages_total {}\n",
            self.messages.load(Ordering::Relaxed)
        ));

        s.push_str("# HELP minitel_mailbox_size Messages currently stored.\n");
        s.push_str("# TYPE minitel_mailbox_size gauge\n");
        s.push_str(&format!("minitel_mailbox_size {mailbox}\n"));

        s.push_str("# HELP minitel_fetch_total Outbound fetches, by what was asked for and how it went.\n");
        s.push_str("# TYPE minitel_fetch_total counter\n");
        s.push_str(&format!(
            "minitel_fetch_total{{what=\"readme\",result=\"ok\"}} {}\n",
            self.readme_ok.load(Ordering::Relaxed)
        ));
        s.push_str(&format!(
            "minitel_fetch_total{{what=\"readme\",result=\"error\"}} {}\n",
            self.readme_fail.load(Ordering::Relaxed)
        ));
        s.push_str(&format!(
            "minitel_fetch_total{{what=\"feed\",result=\"error\"}} {}\n",
            self.feed_fail.load(Ordering::Relaxed)
        ));
        s
    }
}

/// Today's UTC date, from the clock, without pulling in a date library for
/// one string. Days since the epoch through the civil-calendar conversion
/// Howard Hinnant's `civil_from_days` describes.
fn utc_date() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}
