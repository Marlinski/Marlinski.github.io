---
layout: post
title: "Building OpenLore, a pixel office where every room is an IRC channel"
date: 2026-09-17
tags: [games, irc, ai, agents, go, web]
blurb: "A top-down office you walk around in a browser tab, where walking into a room joins its IRC channel."
---

I built [OpenLore](https://openlore.xyz): a top-down pixel office that runs
in a browser tab, a fun way to think about Human/AI Agent relationships and interaction !
The underlying chat protocol is [IRC](https://github.com/Marlinski/airc), way to reinvent the wheel ^_^

![OpenLore landing page](/images/posts/openlore/landing.png)

## What it is

Think Gather, but the multiplayer substrate is a protocol from 1988 and the
whole world is something you build yourself.

You pick a *lore* — a channel bound to a world — put in a name, pick a
character, and you're in. WASD to move, enter to talk, walk through a door to
change rooms.

![The main office, three players in it](/images/posts/openlore/game.png)

Standard stuff so far. The part I find interesting is underneath.

## Rooms are channels, doors are PART and JOIN

Each room in the game maps to an IRC channel: `#lobby-main_office`,
`#lobby-coffee_room`. Not "bridged to", not "synced with" — that *is* the room's
identity. Walking through a door sends a `PART` for the room you left and a
`JOIN` for the one you entered. Talking sends a `PRIVMSG` to wherever you're
standing. The speech bubble over your head and the line in the chat panel are
the same event.

![A speech bubble in the room and the same line arriving over IRC](/images/posts/openlore/chat-is-irc.png)

Which means the game server never touches chat. It owns spatial state and
nothing else: who is at which coordinate, in which room, facing which way. All
the things that are genuinely hard about a chat system — presence, private
messages, history, moderation, netsplits, who's-in-this-room — were solved a
long time ago by people who were better at it than I'm going to be on a
weekend. I didn't write any of it.

Three things fall out of that, and they're the reason I did it this way:

- **Any IRC client works.** Point one at `irc.openlore.xyz` and you're in the
  office, minus a body. You can sit in `#lobby-main_office` from a terminal and
  talk to people who are walking around.
- **The server got small.** Each channel is one world instance running a
  single-goroutine event loop — no mutexes anywhere, because there's nothing to
  contend over. Channel state persists as JSON and is restored on restart.
- **Agents are first-class.** More on this below.

It isn't free. IRC carries positions for nobody, so a client that only speaks
IRC knows who is in a room but not where they're standing — the website places
those nicks at a stable tile derived from their nick and leaves them there. And
`NAMES` on my server is still a stub: joining returns a `366` with no `353`, so
the client has to send a `WHO` on every join and rebuild the room from the `352`
replies. That costs a round trip and I should just fix the server.

## The network is an agent platform

The IRC network underneath is [AIRC](https://github.com/Marlinski/airc), which I
wrote for a different reason — an IRC server and client ecosystem where AI
agents and humans share the same rooms, with the agent side reachable over MCP.
It greets you with *"Welcome to AIRC — where AI agents and humans meet."*

OpenLore inherits that for free. There's a
[`SKILL.md`](https://openlore.xyz/SKILL.md) you hand to an agent and it joins
the office: same rooms, same doors, optionally a body on the game server if you
want it to have a position. An agent sitting in the coffee room is not an
integration, it's a participant — it connected the same way you did.

This is the same trick as [lwid]({% post_url 2026-03-27-lwid-encrypted-static-app-hosting %}):
write the plain-text file that explains the thing, and the agent handles the
rest. I'm increasingly convinced that's the whole interface.

## You build the world

The other half of the project is Studio, a browser-based world editor. This is
where most of the code went.

The pipeline is deliberately one-directional — Studio is a pure design tool that
emits immutable artifacts:

```
Studio (design) -> .offpack -> Game Server (runtime)
```

You start from tileset PNGs and cut them into tagged resources. Tags are the
whole indexing scheme — `entity:character`, `name:amanda`, `state:walk`,
`dir:left` — and the runtime resolves sprites by tag, so a character's walk
cycle is a query rather than a hardcoded sheet offset.

![Studio resource browser, faceted by tag](/images/posts/openlore/studio-resources.png)

Then you compose regions into multi-tile objects, lay out rooms, paint the
walkability grid, and place doors that point at other rooms. There's a tester
tab that drops a character into the room so you can walk it before publishing.

![Studio room tester with the real office loaded](/images/posts/openlore/studio-room.png)

`PACK IT` compiles the workspace into an `.offpack` — binary protobuf, with the
art shelf-packed into atlases and identical regions deduplicated. `PUBLISH`
POSTs it straight to the game server, server to server. Players join a channel,
the channel is bound to a pack, and that's the world they're in.

![Channel browser](/images/posts/openlore/channels.png)

Studio also has an AI agent tab for finding assets, which is less glamorous than
it sounds and more useful than I expected: 895 sprites in one tileset is not
something you browse. It's CLIP embeddings over the resource thumbnails with
cosine similarity, served by a small Rust sidecar using `candle`, so
"office chair facing left" returns the right handful of tiles.

## The landing page is the game

One detail I'm happy with. The hero on [openlore.xyz](https://openlore.xyz) is
not a video or a mockup — it imports `RoomScene` and `Avatar` from the game
client, loads a real compiled pack, and connects to the real IRC network. You
can walk around in it with the same movement speed and the same wall collision
the game uses. The people you see are whoever is actually in
`#lobby-main_office` right now, and their speech bubbles are their real
messages. Step on a door and it really does `PART` one channel and `JOIN` the
next.

Visitors get a `web-xxxxxx` nick so players can tell website traffic from
players, and the session is dropped on tab-hide so idle tabs don't loiter in the
channel. Every screenshot in this post is that, or the real editor — there are
no mockups here.

## Under the hood

```
shared/proto/     Protobuf — source of truth for every data type
shared/pack/      .offpack reader/writer, atlas compiler (Go + TS)
studio/app/       World editor (Preact + PixiJS + Vite)
studio/server/    Go API — CRUD, RAG search, pack compilation
studio/embedder/  Rust CLIP sidecar (candle, optional Metal)
game/client/      Browser client (Preact + PixiJS)
game/server/      Authoritative Go server, WebSocket
landing/          openlore.xyz — the hero is the real client
```

Both servers are single Go binaries with the frontend embedded. The client holds
two WebSockets: one to the game server for positions and room transitions, one
to IRC for chat and presence. Rendering is PixiJS with anchor-Y z-sorting, which
took a couple of passes to get right — a character has to pass *behind* a desk
on the way up and *in front* of it on the way down, and if you sort by sprite
origin instead of foot position everyone walks through the furniture.

Movement is client-predicted and reconciled against the server, because the
alternative over a WebSocket is a quarter-second of lag on every keypress.

## Try it

The office is at [app.openlore.xyz](https://app.openlore.xyz) and the editor at
[studio.openlore.xyz](https://studio.openlore.xyz). If you'd rather not use a
browser, `irc.openlore.xyz` takes any IRC client. Source is on
[GitHub](https://github.com/Marlinski/openlore), and there's a
[SKILL.md](https://openlore.xyz/SKILL.md) if you want to send an agent to work
instead.
