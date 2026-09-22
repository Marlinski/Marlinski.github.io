//! Reading the client's real address from a PROXY protocol header.
//!
//! Traefik terminates the TCP connection before it reaches this pod, so the
//! address the socket reports is Traefik's, the same one for every visitor.
//! PROXY protocol is how the router passes the original address along: a short
//! header in front of the stream, before any SSH bytes.
//!
//! Only read when MINITEL_PROXY_PROTOCOL is set, because the header is then
//! guaranteed to be there. Reading speculatively would mean blocking on bytes
//! that a direct client has no reason to send until it has seen our banner.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt};

const SIG_V2: [u8; 12] = [0x0D, 0x0A, 0x0D, 0x0A, 0x00, 0x0D, 0x0A, 0x51, 0x55, 0x49, 0x54, 0x0A];

/// Consumes the header and returns the address it carries.
///
/// `Ok(None)` means the header was well formed but carried no address — a
/// health check from the router itself, which announces LOCAL rather than a
/// connection it is forwarding.
pub async fn read_header<R: AsyncRead + Unpin>(
    stream: &mut R,
) -> std::io::Result<Option<SocketAddr>> {
    // A router that has accepted the connection sends this immediately; if it
    // does not arrive, nothing further on this stream will parse either.
    tokio::time::timeout(Duration::from_secs(5), parse(stream))
        .await
        .unwrap_or_else(|_| {
            Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "no PROXY header",
            ))
        })
}

async fn parse<R: AsyncRead + Unpin>(stream: &mut R) -> std::io::Result<Option<SocketAddr>> {
    let mut sig = [0u8; 12];
    stream.read_exact(&mut sig).await?;

    if sig != SIG_V2 {
        // v1 is the text form: "PROXY TCP4 1.2.3.4 5.6.7.8 1111 22\r\n".
        // Those 12 bytes are already part of the line, so finish reading it.
        if sig.starts_with(b"PROXY ") {
            return parse_v1(stream, &sig).await;
        }
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "not a PROXY header",
        ));
    }

    let mut meta = [0u8; 4];
    stream.read_exact(&mut meta).await?;
    let ver_cmd = meta[0];
    let family = meta[1];
    let len = u16::from_be_bytes([meta[2], meta[3]]) as usize;

    let mut rest = vec![0u8; len];
    stream.read_exact(&mut rest).await?;

    // High nibble 0x2 is version 2; low nibble 0x0 is LOCAL, the router's own
    // health check, which carries no address to report.
    if ver_cmd >> 4 != 2 || ver_cmd & 0x0F != 1 {
        return Ok(None);
    }

    Ok(match family {
        // TCP over IPv4: 4 + 4 + 2 + 2.
        0x11 if rest.len() >= 12 => {
            let ip = Ipv4Addr::new(rest[0], rest[1], rest[2], rest[3]);
            let port = u16::from_be_bytes([rest[8], rest[9]]);
            Some(SocketAddr::new(IpAddr::V4(ip), port))
        }
        // TCP over IPv6: 16 + 16 + 2 + 2.
        0x21 if rest.len() >= 36 => {
            let mut o = [0u8; 16];
            o.copy_from_slice(&rest[0..16]);
            let port = u16::from_be_bytes([rest[32], rest[33]]);
            Some(SocketAddr::new(IpAddr::V6(Ipv6Addr::from(o)), port))
        }
        _ => None,
    })
}

async fn parse_v1<R: AsyncRead + Unpin>(
    stream: &mut R,
    head: &[u8],
) -> std::io::Result<Option<SocketAddr>> {
    let mut line = head.to_vec();
    // Short and terminated by \n; a byte at a time avoids reading into the SSH
    // stream that follows it.
    loop {
        let mut b = [0u8; 1];
        stream.read_exact(&mut b).await?;
        line.push(b[0]);
        if b[0] == b'\n' || line.len() > 107 {
            break;
        }
    }
    let line = String::from_utf8_lossy(&line);
    let f: Vec<&str> = line.trim_end().split(' ').collect();
    if f.len() < 6 || f[1] == "UNKNOWN" {
        return Ok(None);
    }
    match (f[2].parse::<IpAddr>(), f[4].parse::<u16>()) {
        (Ok(ip), Ok(port)) => Ok(Some(SocketAddr::new(ip, port))),
        _ => Ok(None),
    }
}
