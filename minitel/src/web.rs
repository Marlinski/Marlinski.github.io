//! A one-page website whose only job is to tell you to leave it.
//!
//! Served by the same binary as the SSH front end, on a second port, because a
//! separate pod to return 900 bytes of static HTML would be silly. Hand-rolled
//! HTTP/1.1 for the same reason: one route, one response, no framework.

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const PAGE: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>minitel.marlinski.org</title>
<meta name="description" content="marlinski.org, served over SSH as a Minitel.">
<style>
  :root {
    --bg: #0d0d0d; --fg: #d4d4d4; --dim: #808080; --faint: #4a4a4a;
    --accent: #e8e8e8; --live: #4ee99a; --border: #2a2a2a; --panel: #111;
    --font: "Courier New", Courier, monospace;
  }
  * { box-sizing: border-box; margin: 0; padding: 0; }
  html, body {
    background: var(--bg); color: var(--fg); font-family: var(--font);
    font-size: 14px; line-height: 1.7; min-height: 100vh;
  }
  body {
    display: flex; flex-direction: column; align-items: center;
    justify-content: center; gap: 1.6rem; padding: 2rem 1.5rem; text-align: center;
  }
  h1 {
    font-size: 0.75rem; font-weight: normal; letter-spacing: 0.18em;
    text-transform: uppercase; color: var(--dim);
  }
  .box {
    border: 1px solid var(--border); background: var(--panel);
    padding: 1.1rem 1.6rem; font-size: 1rem;
    display: inline-flex; align-items: baseline; gap: 0.6rem;
    transition: border-color 0.15s;
  }
  .box:hover { border-color: var(--live); }
  .prompt { color: var(--live); }
  .cmd { color: var(--accent); user-select: all; }
  .caret {
    color: var(--live); animation: blink 1.1s step-end infinite;
  }
  @keyframes blink { 0%,49% { opacity: 1 } 50%,100% { opacity: 0 } }
  @media (prefers-reduced-motion: reduce) { .caret { animation: none } }
  p { color: var(--dim); font-size: 0.85rem; }
  a { color: var(--dim); }
  a:hover { color: #fff; }
  @media (max-width: 480px) {
    .box { font-size: 0.85rem; padding: 0.9rem 1rem; flex-wrap: wrap; gap: 0.4rem; }
  }
</style>
</head>
<body>
  <h1>3615 marlinski</h1>

  <div class="box">
    <span class="prompt">$</span><span class="cmd">ssh minitel.marlinski.org</span><span class="caret">&#9612;</span>
  </div>

  <p><a href="https://marlinski.org">&larr; marlinski.org</a></p>
</body>
</html>
"#;

pub async fn serve(port: u16) {
    let listener = match TcpListener::bind(("0.0.0.0", port)).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("minitel: http listener failed on {port}: {e}");
            return;
        }
    };
    eprintln!("minitel: http on 0.0.0.0:{port}");

    loop {
        let Ok((mut sock, _)) = listener.accept().await else {
            continue;
        };
        tokio::spawn(async move {
            // One small read is enough: everything here answers the same way,
            // so the request only has to be recognised, not parsed.
            let mut buf = [0u8; 1024];
            let n = match tokio::time::timeout(Duration::from_secs(5), sock.read(&mut buf)).await {
                Ok(Ok(n)) if n > 0 => n,
                _ => return,
            };
            let req = String::from_utf8_lossy(&buf[..n]);
            let head = req.starts_with("HEAD ");
            let health = req.starts_with("GET /healthz");

            let (status, ctype, body) = if health {
                ("200 OK", "text/plain; charset=utf-8", "ok\n")
            } else {
                ("200 OK", "text/html; charset=utf-8", PAGE)
            };

            let headers = format!(
                "HTTP/1.1 {status}\r\n\
                 Content-Type: {ctype}\r\n\
                 Content-Length: {}\r\n\
                 Cache-Control: public, max-age=300\r\n\
                 Connection: close\r\n\r\n",
                body.len()
            );
            let _ = sock.write_all(headers.as_bytes()).await;
            if !head {
                let _ = sock.write_all(body.as_bytes()).await;
            }
            let _ = sock.shutdown().await;
        });
    }
}
