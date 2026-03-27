---
layout: post
title: "Open Sourcing our Substrate Client for Android (Kotlin)"
date: 2020-11-09
tags: [kotlin, blockchain, android, substrate]
canonical: https://medium.com/nodle-io/open-sourcing-our-substrate-client-for-android-kotlin-5558be84c7fd
---

![Substrate Client for Android](/images/posts/substrate-client/substrate-client-android.png)

In May of this year, my team at Nodle [launched](https://medium.com/@nodle/announcing-nodles-arcadia-blockchain-testnet-bringing-you-a-more-secure-and-faster-blockchain-faf67801b687) our own [Blockchain](https://nodlecode.github.io/polkadot-js-apps/?rpc=wss%3A%2F%2Fmain0.nodleprotocol.io#/explorer) based on Parity [Substrate](https://substrate.dev/) to support our Nodle Cash micro-transactions. As known, [Nodle CashApp](https://nodle.io/cash) is our cryptocurrency wallet that generates Nodle Cash (NODL). The app thus requires the ability to interact with our blockchain to query the balance of an account or send transactions.

Each blockchain built with Substrate exposes an RPC API that can be used to perform the various calls. The RPC is exposed over an HTTP or WebSocket connection. The main client implementation is [polkadot-js](https://github.com/polkadot-js/api) but [other clients implementations](https://substrate.dev/docs/en/knowledgebase/integrate/libraries) exist for different languages such as Go, C++, Rust or Python. In our case, we wanted a native client implementation for our mobile apps but were unable to find any suitable. Substrate is also fast-moving and we did not want to depend on a client that wouldn't be up to date with the latest version since some updates do in fact break API compatibility.

Thus, we developed an Android client library to interact with a substrate-based chain:

**[https://github.com/NodleCode/substrate-client-kotlin](https://github.com/NodleCode/substrate-client-kotlin)**

This implementation has been made in pure Kotlin so it can even be used for any Java project, not just Android. Unlike polkadot-js, the client is simple and mostly optimized for a wallet use case. As of today, it offers the following functionalities:

- compatible with substrate 2.0
- Ed25519 wallet
- get account info (balance)
- sign and send extrinsic
- estimate fee

We are planning to bring support to Schnorrkel/Ristretto signatures (SR25519) soon and try to keep it current with the latest Substrate upgrade.

Our team is truly excited to be an active part of this community and hope that by open-sourcing and sharing this project we will be able to help other teams contribute and grow this ecosystem.

---

*Originally published on [Medium]({{ page.canonical }}).*
