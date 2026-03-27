---
layout: post
title: "lwid: Encrypted Static App Hosting for Vibecoded Dashboards"
date: 2026-03-27
tags: [ai, tools, crypto, web, cli]
---

Around me, a bunch of non-technical colleagues have been vibecoding dashboards. Small static HTML apps — a bit of JavaScript, some charts, maybe a table pulling from a CSV. They know exactly how they want their data to look, and AI agents are surprisingly good at building exactly that.

But they all hit the same wall: **how do you share it?**

The options aren't great. Email the HTML file? Fragile, can't update it. Push to GitHub Pages? Requires a GitHub account and some git literacy. Spin up a container or a VPS? Massively overkill for a 200-line HTML file. Upload to Netlify? Yet another account, yet another dashboard to manage.

I wanted something simpler. Give it to the agent, get a URL back, share that URL with your colleagues. That's it.

So I built [lwid](https://lookwhatidid.xyz) (https://lookwhatidid.xyz).

![lwid](/images/posts/lwid/screenshot.png)

## What it is

Think [plik](https://github.com/root-gg/plik) but for static sites instead of files. You push a directory, you get a URL. Anyone with the URL can view it. You can update it. It expires in 7 days by default.

The twist: **everything is encrypted client-side before it ever leaves your machine**.

## Why encryption matters here

Vibecoded dashboards often contain sensitive data. Sales numbers, internal metrics, customer lists — the kind of stuff you wouldn't want sitting in plaintext on a random server.

Most hosting services store your files in the clear. That's fine for a public portfolio, but not great for internal tooling. With lwid, the server is just a dumb blob store. It never sees your content.

Here's how it works:

- Your files are encrypted with **AES-256-GCM** in the browser (or CLI) before upload
- The server stores opaque, content-addressed blobs — it has no idea what it's hosting
- The <span class="key-read">decryption key</span> lives in the **URL fragment** (`#key`), which browsers never send to the server
- Write access is gated by an <span class="key-write">Ed25519 signature</span>, so the server can verify authorship without knowing the content

The URL looks like this:

<pre class="post-pre"><span class="sh-cmd">https://lookwhatidid.xyz/p/{project-id}#<span class="key-read">{read-key}</span>:<span class="key-write">{write-key}</span></span></pre>

On first load, the <span class="key-write">write key</span> is automatically saved to localStorage and stripped from the URL — so you can't accidentally leak write access by sharing your address bar. The share button defaults to a <span class="key-read">read-only</span> URL, but can optionally include the <span class="key-write">write key</span> if you want to collaborate.

## Agentic first

The whole thing is designed to be driven by an AI agent. There's a [`SKILL.md`](https://lookwhatidid.xyz/SKILL.md) — a plain-text file that tells the agent exactly what lwid is, how to install it, and how to use it. Give the skill to your agent and it handles the rest: encrypt, upload, return a URL.

<pre class="post-pre"><span class="sh-comment"># The agent does this:</span>
<span class="sh-cmd">lwid push --server https://lookwhatidid.xyz</span>
<span class="sh-comment"># → https://lookwhatidid.xyz/p/abc123#<span class="key-read">readkey</span>:<span class="key-write">writekey</span></span></pre>

No account creation, no OAuth flow, no config files to explain.

## Stateful webapps

Static doesn't have to mean stateless. lwid ships a JS library that exposes a **key-value store** and a **blob store** backed by encrypted S3 objects. Your vibecoded app can persist data across sessions — still zero-knowledge, still no plaintext on the server.

```js
import { kv } from "https://lookwhatidid.xyz/sdk.js";

await kv.set("last_run", new Date().toISOString());
const last = await kv.get("last_run");
```

## Under the hood

The backend is a small Rust server (Axum) fronting S3-compatible storage. The shell SPA is vanilla JS — no framework. A Service Worker intercepts navigation and decrypts content on the fly inside a sandboxed iframe, which means the decryption key never touches the server even during rendering.

The CLI is also Rust, distributed as a single static binary.

```
lwid-common/   Crypto primitives (AES-256-GCM, Ed25519), CID utilities
lwid-server/   Axum HTTP server, blob storage, shell SPA
lwid-cli/      CLI binary — push, pull, clone, kv, blob
```

## Try it

- Live: [lookwhatidid.xyz](https://lookwhatidid.xyz)
- Skill: [lookwhatidid.xyz/SKILL.md](https://lookwhatidid.xyz/SKILL.md)
- Source: [github.com/Marlinski/lwid](https://github.com/Marlinski/lwid)

Install the CLI:

```sh
curl -fsSL https://raw.githubusercontent.com/Marlinski/lwid/main/install.sh | sh
```

Or just hand the skill URL to your agent and let it figure out the rest.
