---
layout: post
title: "Rumble, or what happens when you delete the server"
date: 2018-02-15
tags: [android, p2p, dtn, networking, mesh]
blurb: "Micro-blogging that spreads phone to phone. No towers, no accounts, no server."
---

Every messaging app you have ever used works the same way: your phone talks to
a server, the server talks to your friend's phone. Take the server away and
nothing moves. Take the *internet* away and nothing moves.

I wanted to know what was left if you deleted both. So a couple of years ago I
built [Rumble](https://github.com/Marlinski/Rumble): a micro-blogging app for
Android that has no server, no accounts, and never touches the internet.

I put it behind a project called Disrupted Systems — "a research laboratory on
disruptive communication", which was a grand name for a one-man project.
The site is gone now, but the
[Internet Archive kept it](https://web.archive.org/web/20180215121820/http://disruptedsystems.org/).

[![The Disrupted Systems site in 2018](/images/posts/rumble/disruptedsystems-site.jpg)](https://web.archive.org/web/20180215121820/http://disruptedsystems.org/)

## What it was

Think Twitter, except there is no Twitter. You write a message, it lands in a
database on your own phone, and that is the entire send operation. No request
goes out, because there is nobody to send it to yet.

The message moves later, when your phone meets another phone.

Whenever Rumble is running it scans for other Rumble devices over Bluetooth and
Wifi. When it finds one, the two devices open a channel, exchange their
preferences, and then start handing each other messages. Then they part ways
and do it again with the next device they meet.

Nobody routes anything. Messages spread because people move.

![The Rumble feed, showing messages in rumble.public](/images/posts/rumble/phone/feed.png)

Everyone starts in a default group called `rumble.public`, so a fresh install
has somewhere to talk. On the website's hero shot I had seeded it with Foucault,
Emma Goldman, Voltairine de Cleyre and Proudhon, which tells you roughly what I
thought the app was for.

## Store, carry, forward

The academic name for this is delay-tolerant networking, and the idea is older
than the app — it comes out of deep-space and disaster-response research, where
"the link is down" is the normal state rather than an error. RFC 5050 had
specified a Bundle Protocol for it back in 2007. What was new in 2015 was
that everyone was carrying a device with two radios and a battery, so the
"unreliable mobile node" the papers assumed was now just a person on a bus.

The hard part is not moving a message between two phones. It is deciding
*which* messages to move, because storage is finite and the number of messages
in the network is not. Rumble ranks what it carries — your hashtag
subscriptions, how widely replicated a message already is, how old it is — and
trades the most valuable things first. A message everyone already has is
worthless to forward. A message nobody has seen and somebody subscribed to is
worth a lot.

There was also a real-time mode: messages that go to the devices currently
around you and are deliberately *not* forwarded any further. Shouting across a
room instead of posting.

![The Neighborhood pane, listing devices discovered over Bluetooth and Wifi](/images/posts/rumble/phone/neighborhood.png)

That screen was my favourite part of the app, and the only one that felt like
something genuinely different: a list of the strangers currently in radio range
who happen to be carrying the same software.

## Private communities

The public group is open by definition, so Rumble also let you make private
ones, encrypted with AES-128 in CBC mode. A message in a private group only
propagates between members. You could not simply join one — a member had to
invite or vouch for you.

That constraint falls out of the architecture rather than from a policy
decision. There is no server to hold a membership list, so the only thing that
can admit you is a device that is already in the group.

![The Groups screen, showing rumble.public and a private group](/images/posts/rumble/phone/groups.png)

## What it actually cost

The permission list tells the story better than I can: `BLUETOOTH`,
`BLUETOOTH_ADMIN`, `ACCESS_WIFI_STATE`, `CHANGE_WIFI_STATE`,
`CHANGE_WIFI_MULTICAST_STATE`, `RECEIVE_BOOT_COMPLETED`. To do opportunistic
networking you need to drive both radios yourself, keep a service alive across
reboots, and do it on an OS that spent every release getting more hostile to
exactly that. Half the later commits are not features. They are Android
catching up with me: dynamic permissions for 6.0, a build-tools bump, a
`FloatingActionButton` that stopped scrolling properly when the design support
library moved.

The git history also contains the sentences "update submodule version, such
pain" and "damn submodule why so tricky", which I stand by.

And then the thing you cannot engineer around. An off-the-grid network is only
as good as its density. Two users in the same city who never stand near each
other are not a network, they are two databases. Rumble got somewhere between
100 and 500 installs on Google Play. That is enough people to prove the
protocol works and nowhere near enough for a message to reach anyone by
accident. The app worked exactly as designed, and the design needed a crowd I
did not have.

## Where it stands

Last commit was April 2017. It is GPLv3, the source is on
[GitHub](https://github.com/Marlinski/Rumble), and the APKs are somehow still
sitting on [F-Droid](https://f-droid.org/packages/org.disrupted.rumble/) —
version 1.0.2, `minSdkVersion 16`, which is to say it will still install on a
phone from 2012.

I do not think the idea was wrong. Every year or two something happens — a
protest, a hurricane, a government reaching for the off switch — and the same
apps get rediscovered and written about for a week. The problem was never the
protocol. It is that a network built out of proximity needs people to already
be nearby, and software cannot hand you that.

Still: it is a strange and good feeling to watch a message you wrote arrive on
a stranger's phone with no infrastructure in between. Worth doing once.
