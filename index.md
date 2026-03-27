---
layout: default
---

Research engineer. Currently building [SHAI](https://github.com/ovh/shai) at OVHcloud — an AI coding agent that lives in your terminal.
Working at the intersection of AI, blockchain, cryptography, and decentralized networks.
International background: France, Singapore, US, Canada.

## recent posts

<ul class="sq-list">
{% for post in site.posts limit:3 %}
  <li><a href="{{ post.url | relative_url }}">{{ post.title }}</a> <span class="meta">{{ post.date | date: "%Y-%m-%d" }}</span></li>
{% endfor %}
</ul>

## projects

<ul class="proj-list">
  <li><span class="proj-title"><a href="https://github.com/ovh/shai">SHAI</a></span><span class="proj-desc">AI coding agent and pair-programming buddy that lives in the terminal.</span><span class="proj-tags">#ai #cli #rust</span></li>
  <li><span class="proj-title"><a href="https://github.com/Marlinski/airc">AIRC</a></span><span class="proj-desc">IRC platform where AI agents and humans connect, discover capabilities, and collaborate.</span><span class="proj-tags">#ai #irc #rust</span></li>
  <li><span class="proj-title"><a href="https://lookwhatidid.xyz">lwid</a> <a href="https://github.com/Marlinski/lwid" class="proj-gh">[gh]</a></span><span class="proj-desc">Encrypted, zero-knowledge app-sharing platform. Pastebin for small web apps, with client-side encryption.</span><span class="proj-tags">#crypto #web #cli</span></li>
  <li><span class="proj-title"><a href="https://github.com/Marlinski/plan">plan</a></span><span class="proj-desc">Lightweight CLI task tracker for AI agents and humans. Persistent, cross-session, no server.</span><span class="proj-tags">#ai #cli #productivity</span></li>
  <li><span class="proj-title"><a href="https://github.com/Marlinski/Rumble">Rumble</a></span><span class="proj-desc">Off-the-grid micro-blogging app. Think Twitter, but no internet required.</span><span class="proj-tags">#p2p #mobile #android</span></li>
  <li><span class="proj-title"><a href="https://github.com/NodleCode/substrate-client-kotlin">substrate-rpc</a></span><span class="proj-desc">RPC client for Parity Substrate / Polkadot chains.</span><span class="proj-tags">#blockchain #polkadot #kotlin</span></li>
  <li><span class="proj-title"><a href="https://github.com/Marlinski/Terra">Terra</a></span><span class="proj-desc">Lightweight extensible DTN Bundle Protocol library.</span><span class="proj-tags">#dtn #networking #java</span></li>
</ul>

## elsewhere

All projects on [GitHub](https://github.com/Marlinski).
