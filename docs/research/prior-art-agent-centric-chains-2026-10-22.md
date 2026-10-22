# Prior art for personal chains: Holochain, Secure Scuttlebutt, and neighbors

**In one line: the personal chain and the rival / non-rival split are prior art; pricing contributions and paying
witnesses are not.**

2026-10-22. Research for `the-node-and-the-network-2026-10-22.md`, the "one chain per person, Noesis is the node"
framing. Most claims come from primary sources: official docs, whitepapers, specs, source code, and maintainers'
posts. Anything taken from press coverage is marked as such, and "Method and limits" at the end lists what could
not be checked. Sources are listed at the end of each section. Web pages were read on 2026-10-22.

Status words used throughout:
- **shipped**: in a released version and on by default.
- **behind a flag**: the code exists but is off by default or marked unstable.
- **designed**: written in a whitepaper or spec, not built.
- **proposed**: an issue, open PR, or discussion.
- **unclear**: sources disagree.

---

## Short answer

1. **Holochain is the closest prior art, and it already draws our rival / non-rival line.** Its 2024 whitepaper
   says nearly what our note says: treating non-rival data as rival makes coordination needlessly expensive, and a
   double-spend is an agent forking its own chain. It catches forks by sending every action an agent takes to that
   agent's DHT neighbors, who can see two actions with the same parent. But in Holochain's own docs: countersigning
   "is not for preventing double-spends"; chain-fork warrants "aren't operational yet"; countersigning has been
   unstable and off by default since 0.4 (Dec 2024); and DHT sharding is off by default. HoloFuel (its money) was
   designed in 2017 and is still not live. The HOT migration is promised "by November 1st, 2026". The protocol has
   no notion of the value of a contribution.
2. **Scuttlebutt showed that union replication of per-person signed logs works for social data at small scale.** It
   chose not to do consensus or money. A fork is caught only by a peer that already holds the other branch. The
   feed then dies, for honest users who restored from a backup too. A "proof of fork" that would spread was proposed
   in 2017 and was still listed as open future work in the 2019 paper. Multi-device use and backup restores were
   among its worst UX problems. Adoption stalled: Planetary moved to Nostr in 2023, and Manyverse's lead developer
   left in 2024.
3. **The theory backs our first hypothesis exactly.** Kleppmann and Howard (2020) prove that a peer-to-peer database
   can tolerate any number of Byzantine peers if and only if every operation is invariant-confluent, meaning merges
   can never break an application rule. Guerraoui et al. (2019) show that transfers out of single-owner accounts need
   **no** consensus at all, only that the owner cannot equivocate. Accounts shared by k owners need consensus among
   those k. That sharpens "consensus among the parties" and supports "witnesses, not consensus" for personal chains.
4. **None of the agent-centric systems studied (Holochain, SSB, Hypercore, Ceramic, Nostr) pays anyone per record
   to witness, store, or validate, and none prices contributions.** Holo is designed to pay hosts for serving web
   users, today in fiat. Linera's validators would have earned like any proof-of-stake chain's. Neither pays per
   witnessed record. Our third and fourth hypotheses (every overlap record pays both parties; payouts scale with
   value created) have no prior art in this family. They are what Noesis adds, and they are the least tested part.

Where that leaves Noesis: the personal chain itself is not new. Holochain, SSB, Hypercore, and Linera all built
one, and the rival / non-rival split is already in the literature. What would be new is pricing contributions
across chains and paying the witnesses. Our claims should lead with those two, not with the chain. One caveat: this
survey did not cover prior art on pricing contributions (for example SourceCred or retroactive public-goods
funding). That has to be checked before we claim novelty there.

---

## 1. Holochain

### (a) Identity and the per-agent chain

- An agent is a keypair. Its public key is the agent's address in the same hash space as the data, so "nearness" to
  an address decides who stores what (whitepaper 2.0). **Shipped.**
- Each agent has a **source chain** for each app (DNA): "A hash chain of records committed by an agent" (glossary).
  Every action is signed by its author. **Shipped.**
- **Membranes**: the first records on a chain include a membrane proof, and a validation function decides who may
  join the network. **Shipped**, with a known gap: as of 0.6, membrane proofs are "only enforced via normal
  validation, not during handshaking." An unauthorized agent can join briefly before it is warranted and blocked
  (Dev Pulse 153, Dec 2025).
- Key continuity across devices (DPKI / DeepKey) is **behind a flag** (`unstable-dpki`, 0.4 upgrade guide).

### (b) Rival data: double-spends and exclusive claims

- The whitepaper (v2.0, Nov 2024) states our split directly. Its examples are unique usernames and a currency
  double-spend: "coordination overhead becomes unnecessarily high if all non-rivalrous data is treated as
  rivalrous." For a currency, "the double-spend problem presents differently. It involves Alice 'forking' her own
  hash chain": two payments that point to the same parent record.
- What Holochain offers for rival data, in its own words:
  1. **The DHT as an "ambient witness."** Anyone can look up whether an agent forked. An app on a well-connected
     network can make the recipient wait before accepting, to give a fork time to show up. **Shipped** (the lookup);
     the delay is up to each app.
  2. **Countersigning.** Two or more agents lock their chains, agree on one entry, sign it, and commit it to every
     chain. An optional **enzyme** (a trusted coordinator) and optional **M-of-N witnesses** (quorum over 50%) form
     what the whitepaper calls "micro-consensus." **Behind a flag**: "considered unstable and is disabled by default
     in Holochain 0.4 and newer. It may be removed or changed."
  3. **Social remedies.** In a high-trust setting, the forker can cancel one branch and pay again (whitepaper).
- **Key correction to our note.** The countersigning docs say outright that countersigning does not prevent
  double-spends: "The plain answer, though, is that it isn't." It is for "atomically synchronizing a single write to
  multiple source chains." Holochain "merely equips all peers to detect source chain forks" after the fact. The docs
  add that the threat of ejection may not deter forks "in low-trust settings, where fake agents are cheap." Stronger
  protection comes from enzymes or witnesses acting as notaries.
- Countersigning is still being redesigned. An open PR (#5966, Aug 2026) argues "The existing protocol is already a
  two-phase commit, missing the durability and decision rules that make 2PC safe." It proposes commit and abort
  certificates, chain locks that never expire, and dropping M-of-N witnesses. **Proposed.**
- **HoloFuel (money).** The 2017 Holo currency paper describes mutual credit. Every transaction is a double-entry
  record countersigned by spender and receiver. Net supply is always zero. Credit limits are set from a host's
  hosting revenue over three months. Its defense against the agent-side double-spend (it calls this a Finney
  attack): above a threshold, the recipient waits about log2(amount) x 10 seconds, then asks the sender's DHT
  neighbors whether they hold headers the sender left out. **Designed** (2017). The current build is by Unyt, a
  Holochain Foundation subsidiary. The sender commits, the receiver accepts, and peers re-run the rules ("smart
  agreements"). Unyt's changelog says it now checks that an allocation's source is not reused. HoloFuel itself is
  **not launched**: hot.talk says HOT "becomes convertible to HoloFuel by: November 1st, 2026", and holo.host still
  prices hosting in national currencies.

### (c) Equivocation: catching an agent who forks its own chain, and the cost

- **Detection design.** Every action produces an "agent activity" operation addressed to the author's own public key.
  The author's DHT neighbors therefore hold the author's full action history. They check that sequence numbers and
  timestamps never go backwards, and they see a fork as two actions sharing one parent. This is a fixed meeting
  point for witnesses: everyone who wants to know about Alice asks the same neighborhood. **Shipped.**
  `get_agent_activity` returns forks, warrants, and invalid data.
- **Treatment.** "System-level validation considers this 'fork' a validation error, and applies it to the whole chain
  and its author." Authorities that have not seen the fork yet may disagree for a while.
- **Penalty.** Warrants are signed, self-proving claims, published to the offender's address. Peers who verify a
  warrant block the agent at the network level, which in effect ejects it from that app's network. Warrants plus
  blocking became **stable in 0.6** (released 19 Nov 2025). In 0.7 (30 Jul 2026), app-level `block_agent` was
  removed: "Blocking is a system-level behavior driven by warrants, not something an application decides." The
  2025 year-end post admits that until then "there were no real consequences when validation failed."
- **But forks specifically:** the current docs say "Chain fork warrants aren't operational yet. Warrants are only
  produced for operations that fail app or basic system validation rules. Chain forks will be warranted in the
  future." The code is moving: PR #6003 (merged Sep 2026 into develop-0.7) fixed a bug where a forged action "could
  be used as one half of a chain fork warrant against them." **Unclear**; treat fork penalties as not shipped until
  the release notes say they are.
- **Accidental forks** (restoring an old backup, two devices) are acknowledged: "We recognize that some source chain
  forks are accidental... We plan to address this scenario." The whitepaper's answer, "meta-data hardening," is
  **designed**, not built. Once gossip around an agent has been quiet for 5 to 15 minutes, peers would treat the
  state as settled and reject later conflicting changes. The whitepaper says "the initial beta version of Holochain
  does not offer fork finality protections."
- **What it costs the forker:** losing that identity in that app. There is no stake or slashing. The docs also warn
  that "The absence of a warrant or chain fork doesn't necessarily mean an agent is in good standing."

### (d) Value or worth of a contribution

- **None in the protocol.** Validation is yes/no.
- The nearest things are apps built on Holochain. hREA implements ValueFlows (resources, events, agents), which
  records who did what. Sensorica's "open value network" uses it to track contributions; it "attributes valuations
  to contributions through social processes" and "allocates shares of generated income based on these valuations."
  That is a split of realized income, with the weights set by people. HoloFuel credit limits follow hosting revenue,
  but that is payment for a service, not a valuation of contributions.

### (e) Incentives to store, validate, and witness

- Storing and validating is a **duty of membership**: agents whose addresses are near a piece of data must hold and
  validate it. No one is paid per operation. Authors keep republishing until enough authorities return signed
  validation receipts.
- Sharding has been **behind a flag** since 0.4 ("Omitting this flag ... force[s] you to choose either 'empty' or
  'full'"). As a result, today's networks are made of nodes that hold everything ("full arc") and nodes that hold
  nothing ("zero arc"). The roadmap lists sharding for 0.9 as **planned**, 0% complete.
- The paid layer is Holo Hosting: hosts serve web users who don't run a node, to be paid in HoloFuel. Today hosting
  is priced in USD or EUR. "Edge Node" (Oct 2025) is an open-source container image for always-on nodes.

### (f) Adoption history and known problems

- There have been three codebases: a Go prototype (2016, archived), `holochain-rust` (2018, "DEPRECATED"), and the
  current Rust codebase (2020 onward).
- Holo's 2018 token sale raised about 30,000 ETH, roughly $20M at the time (press figure). Consumer launch was
  promised for summer 2018. HoloPorts began shipping at the end of 2019. HoloFuel still isn't live eight years
  later (press coverage and holo.host).
- Oct 2025 strategic update: "We haven't gotten enough traction yet for this to be our sole strategy". Focus moved
  to Unyt payment rails and Edge Node.
- Dec 2025 retrospective: before the Kitsune2 network rewrite, DHT sync "could take 30 minutes or more," if it
  finished at all. Peer discovery and validation were unreliable.
- **Lesson:** the conceptual split was right early. The hard parts took 8+ years and are still partly behind flags:
  penalties for forks, multi-party rival writes, sharding, multi-device keys. Promises made against a token sale ran
  far ahead of the code.

**Sources (Holochain)**
- [Holochain whitepaper 2.0, "Distributed Coordination by Scaled Consent, not Global Consensus" (Harris-Braun, Brock, d'Aoust, 2024-11-08)](https://www.holochain.org/documents/holochain-white-paper-2.0.pdf)
- [Concepts: Validation](https://developer.holochain.org/concepts/7_validation/)
- [Concepts: Countersigning](https://developer.holochain.org/concepts/10_countersigning/)
- [Build guide: Getting an Agent's Status](https://developer.holochain.org/build/getting-an-agents-status/)
- [Glossary](https://developer.holochain.org/resources/glossary/)
- [Upgrade 0.3 to 0.4 (unstable flags: countersigning, DPKI, sharding)](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.4/)
- [Upgrade 0.6 to 0.7 (blocking is system-level, forked actions excluded)](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.7/)
- [Roadmap and release dates](https://www.holochain.org/roadmap/)
- [Dev Pulse 153: Holochain 0.6 Released with Immune System (2025-12-03)](https://blog.holochain.org/dev-pulse-153-holochain-0-6-released-with-immune-system/)
- [2025 at a Glance: Landing Reliability (2025-12-31)](https://blog.holochain.org/2025-at-a-glance-landing-reliability/)
- [Finding Our Edge: A Strategic Update (2025-10-02)](https://blog.holochain.org/finding-our-edge-a-strategic-update/)
- [PR #5966: countersigning design doc (open, Aug 2026)](https://github.com/holochain/holochain/pull/5966)
- [PR #6003: sign and verify every action with its author (merged Sep 2026)](https://github.com/holochain/holochain/pull/6003)
- [Holo currency whitepaper (Brock, Harris-Braun, 2017-11-28)](https://files.holo.host/2017/11/Holo-Currency-White-Paper_2017-11-28.pdf)
- [holo.host: Currency, pricing, payments and HoloFuel](https://holo.host/currency/)
- [hot.talk: HOT to HoloFuel migration](https://hot.talk/)
- [Unyt FAQ](https://unyt.co/faq/) and [Unyt changelog](https://github.com/unytco/unyt-infra-marketplace/blob/release/CHANGELOG.md)
- [Sensorica blog: Beyond the Ledger (2025-07)](https://sensoricablog.blogspot.com/2025/07/beyond-ledger-why-blockchain-and-daos.html)
- [holochain-proto (archived)](https://github.com/holochain/holochain-proto), [holochain-rust (deprecated)](https://github.com/holochain/holochain-rust)
- Press, for the 2018 raise and delays: [Crypto Briefing progress report](https://cryptobriefing.com/holochain-hot-token-progress-report/), [Holo starts shipping HoloPorts](https://medium.com/flipsidecrypto/holo-starts-shipping-holoports-globally-fb517c0a69d5)

---

## 2. Secure Scuttlebutt (SSB)

### (a) Identity and the per-agent chain

- An identity is an ed25519 keypair, and the public key is the feed ID. A feed is "an append-only log". Each message
  carries `previous` (the hash of the last message), `sequence` (1, 2, 3...), and the author's signature. Feeds are
  self-certifying, so anyone can serve them. **Shipped** (since about 2014).
- The recommended practice is one keypair per device, because two devices sharing a key will fork the feed.

### (b) Rival data

- **Not attempted, on purpose.** The 2019 paper says "the SSB network itself deliberately doesn't offer consensus
  services nor central content." There is no money and no payment in the protocol.
- Apps handle concurrency their own way. git-ssb resolves branch conflicts "through social coordination" (merge
  commits). ssb-chess avoids concurrency because players take turns. Gatherings used "most recent" by self-stated
  time, which the paper notes was fragile.

### (c) Equivocation: forks and their cost

- **Rule:** each relay checks that every message has "exactly zero or one incoming backlinks. If it has more than
  one, the log is considered forked. All messages from the point of the fork onwards are ignored, the log cannot be
  appended to anymore."
- **Detection is local only.** Dominic Tarr (2017): a fork is caught "if the network has already received the
  original n_0 message"; "If some nodes have not seen the original n_0 then they will accept the new one and your
  feed will be forked." His proposed fix was a fork proof "given out when someone requests the feed, killing that
  feed." **Proposed, never built**: the issue was closed by a stale-bot in 2019. The 2019 paper: "ideally, the
  information that a fork occurred should be propagated. We leave this open for future work." I found no evidence
  that later SSB code shipped fork-proof propagation.
- **Cost:** the identity dies. Honest users pay the same price. Manyverse's FAQ warns that using the recovery phrase
  on a second device can leave the account "irrecoverably stuck". Manyverse issue #111 shows that even the
  recovery flow itself could fork a feed.

### (d) Value of contributions

- None.

### (e) Incentives to store and replicate

- **Social, not paid.** You replicate the feeds you follow and a few hops further: "Messages from feeds 3 hops out
  are replicated to help keep them available for others."
- **Pubs** (always-on peers with stable IPs, reachable through invite codes) are run by volunteers. **Rooms** only
  help peers connect and store nothing.
- Planetary's founder praised the "mutual aid" hosting model, but called the pub and room workarounds a hack layered
  on a network meant to be flat.

### (f) Adoption and why it stalled

- At its peak it was "a community of multiple thousand users" (2019 paper).
- March 2023: Planetary, the best-funded SSB app, moved to Nostr. Its reasons: "The signing of the entire log makes
  everything harder"; it was designed for desktops, not phones; "there is no delete"; using one identity on several
  devices is "basically impossible"; onboarding was hard; offline-first was "rarely used." A developer's comparison
  added that Nostr had more users and was "much easier to understand and implement."
- July 2023: André Staltz (Manyverse) wrote that SSB "is becoming outdated", that "The multi-device story on SSB is
  also bad", and that invites only worked through pubs and rooms. His successor protocol, PPPPP, used a DAG, allowed
  deletion, and linked device keys.
- April 2024: Staltz stepped away after 6,400 hours. `ssb-server` has not been pushed since June 2022.
- **Lesson:** per-person logs plus social replication work. What hurt adoption was the whole-log, never-delete,
  one-device design: it made forks easy for honest users, made sync heavy on phones, and left hosting to volunteers.

**Sources (SSB)**
- [Tarr, Lavoie, Meyer, Tschudin, "Secure Scuttlebutt: An Identity-Centric Protocol for Subjective and Decentralized Applications," ACM ICN 2019](https://conferences.sigcomm.org/acm-icn/2019/proceedings/icn19-19.pdf)
- [Scuttlebutt Protocol Guide](https://ssbc.github.io/scuttlebutt-protocol-guide/)
- [SSB spec: Feeds and Messages](https://spec.scuttlebutt.nz/feed/messages.html)
- [ssb-db issue #157, "Network reaction to falsified ('forked') own feed?" (2017, replies by Dominic Tarr)](https://github.com/ssbc/ssb-db/issues/157)
- [Manyverse FAQ: account on many devices](https://www.manyver.se/faq/account-on-many-devices/), [Manyverse issue #111 (recovery forks feed)](https://github.com/staltz/manyverse/issues/111)
- [Manyverse July 2023 update (Staltz)](https://www.manyver.se/blog/2023-07-05/), [Manyverse "My last update" (2024-04-05)](https://www.manyver.se/blog/2024-04-05/)
- [Henshaw-Plath, "Pivoting Protocols, from SSB to Nostr" (2023-03-19)](https://relay.nos.social/blog/pivoting-from-ssb-to-nostr), [Lorentz, "Nostr v SSB" (2023-01-18)](https://mattlorentz.com/2023/01/18/nostr-v-ssb.html)
- [ssb-server repository (last push 2022-06)](https://github.com/ssbc/ssb-server)

---

## 3. Neighbors (short)

### Hypercore (Dat, now Holepunch)

Hypercore is a single-writer, signed Merkle log. The key derives from a "manifest", which can require a quorum of
signers. The core has no rival-data handling. For many writers, **Autobase** puts their causal graph into one order.
That order becomes final (a "signed length") only "so long as a majority set of indexers are writing messages," so
order-sensitive data gets a quorum of chosen indexers. **Forks:** the owner may openly truncate and rewrite; the
"fork id" increments and peers see a `truncate` event. The current source code also handles a hidden fork. If two
valid signatures exist for the same length and fork id, the core is marked frozen on disk, emits `conflict` with the
proof, closes all sessions ("Two conflicting signatures exist for length N"), and sends the proof to every connected
peer so they can check it themselves. This is **shipped** in the code (main branch, v11.39.0, Oct 2026), but the
public API docs don't list it. The old Dat spec already called such a feed "corrupt." There is no value notion and
no storage incentive; peers seed what they use. **Lesson:** a fork proof that spreads costs only a few dozen lines
once heads are signed; SSB never shipped one. Hypercore also shows that "append-only" is a policy: it allows signed
truncation.
Sources: [hypercore lib/core.js (checkConflict, _onconflict)](https://github.com/holepunchto/hypercore/blob/main/lib/core.js), [lib/replicator.js](https://github.com/holepunchto/hypercore/blob/main/lib/replicator.js), [Hypercore docs](https://docs.pears.com/building-blocks/hypercore/), [Autobase README](https://github.com/holepunchto/autobase), [DEP-0002 Hypercore](https://www.datprotocol.com/deps/0002-hypercore/)

### Ceramic

A Ceramic stream is a hash-linked log of commits controlled by one DID. Conflicting branches are settled by the
**earliest blockchain anchor** (Ethereum block height), so rival ordering was borrowed from a global chain. The
legacy spec names the flaw itself, a **data withholding attack**. The owner anchors update A in secret and publishes
the later-anchored B; everyone builds on B; then the owner reveals A, "the stream will fork back to this update and
all of the other updates ... will be invalidated." The spec calls this "essentially a *double spend attack*". It
argues that a single owner is "naturally disincentivized" because they would lose their own history. It says
organizations need "heavy anchors". Anchoring went through a company-run anchor service. 3Box Labs merged into
Textile (Feb 2025). In April 2025 js-ceramic and ComposeDB were deprecated, the anchor service was set to shut down,
and the team refocused on an AI-agent product (Recall). There is no value notion and no node incentive.
**Lesson for our priority rule:** "earliest commitment wins" lets a party commit in secret and reveal later to
rewrite priority. A commitment should count only once its content is published, or within a short, enforced reveal
window.
Sources: [Ceramic legacy specification (conflict resolution, data withholding)](https://github.com/ceramicnetwork/ceramic/blob/main/LEGACY_SPECIFICATION.md), [The Future of Ceramic: Focusing on Recall (2025-04-17)](https://blog.ceramic.network/the-future-of-ceramic-focusing-on-recall/), [Ceramic is joining Textile](https://blog.ceramic.network/ceramic-is-joining-textile/)

### Nostr

Nostr has no chains. Each event is signed and carries a self-declared `created_at`, and relays store events. For
"replaceable" events, relays keep only the latest per key and kind; ties go to the lowest id. So an owner can
publish conflicting versions freely, and the protocol has no idea of a fork. Money is off-protocol (Lightning
"zaps"), and the spec admits: "The `zap receipt` is not a proof of payment ... You are trusting the author." There is
no protocol incentive for relays. **Lesson:** Nostr won users SSB lost (multi-device, delete, simple) by giving up
order and fork evidence, which are exactly what rival facts need. Self-declared timestamps must never decide
priority.
Sources: [NIP-01](https://github.com/nostr-protocol/nips/blob/master/01.md), [NIP-57 Lightning Zaps](https://github.com/nostr-protocol/nips/blob/master/57.md)

### CRDT-style ledgers and the theory under them

- **Kleppmann and Howard (2020), Byzantine Eventual Consistency.** A peer-to-peer database tolerates any number of
  Byzantine peers (so it is immune to Sybil attacks) "if and only if all of the possible transactions are
  I-confluent". I-confluent means merges can never break an application rule. A cryptocurrency is not I-confluent,
  because two payments by one payer can together overdraw. "Uniqueness of an attribute" is unsafe for user-chosen
  values but "may be safe if the value is determined by the hash of the message." A materialized view (a derived
  result) is always safe. Consensus is needed "only ... for those transactions that are not I-confluent," for
  example an auction that collects bids freely and needs consensus only to pick the winner. The paper also shows
  that per-author counters (vector clocks) can be corrupted by a peer sending two messages with the same stamp;
  hash-linked graphs cannot.
- **Bailis et al. (VLDB 2015).** Invariant confluence is "a necessary and sufficient condition for safe,
  coordination-free execution" (non-Byzantine setting).
- **Guerraoui et al. (PODC 2019), The Consensus Number of a Cryptocurrency.** Transfers out of single-owner accounts
  have consensus number 1, so they need no consensus, only broadcast that stops the owner from equivocating.
  Accounts shared by k owners have consensus number k. Note that their Byzantine construction assumes a known set
  of processes with a bound on faults.
- **Adjacent, worth knowing:**
  - **SUNDR's fork consistency (2004).** A server that shows two users different histories must keep them apart
    forever, because either one "will detect the attack upon seeing a subsequent operation by the other". This is
    the formal basis of "witnesses who compare copies."
  - **C2SP transparency-log witnesses.** A witness cosigns a log's new checkpoint only if it is consistent with the
    last one it saw. A verifier that demands k cosignatures cannot be shown a split view unless witnesses collude,
    which turns detection into prevention.
  - **Linera.** One "microchain" per user on a shared validator set. Validators guarantee "at most one unique
    block" per height. Owners drive liveness. Single-owner chains use a light fast path, and multi-owner or public
    chains use rounds or BFT. This is the closest existing match to "one chain per person." Trade press reports
    that it shut down on 18 Sep 2026 before mainnet, for lack of funding; the original announcement was on Discord.

Sources: [Kleppmann and Howard, arXiv:2012.00472](https://arxiv.org/abs/2012.00472), [Bailis et al., arXiv:1402.2237](https://arxiv.org/abs/1402.2237), [Guerraoui et al., arXiv:1906.05574](https://arxiv.org/abs/1906.05574), [Li, Krohn, Mazieres, Shasha, SUNDR, OSDI 2004](https://www.usenix.org/legacy/event/osdi04/tech/full_papers/li_j/li_j.pdf), [Mahajan et al., Depot (fork-join-causal), OSDI 2010](https://www.usenix.org/legacy/events/osdi10/tech/full_papers/Mahajan.pdf), [C2SP tlog-witness](https://github.com/C2SP/C2SP/blob/main/tlog-witness.md), [Linera docs: Microchains](https://linera.dev/developers/core_concepts/microchains.html), press on Linera: [Crypto Times](https://www.cryptotimes.io/2026/09/19/a16z-backed-linera-ceases-operations-after-sonar-sale-falls-short/), [Bloomingbit](https://en.bloomingbit.io/feed/news/120649)

---

## 4. Mapping to our hypotheses

### H1. Non-rival facts merge by plain union in any order; rival facts need consensus among the parties

**Verdict: shown, in both theory and practice, with four sharpenings.**
- *Shown.* The Kleppmann-Howard theorem is the formal version of H1, and it holds even under Byzantine peers. The
  Holochain whitepaper makes the same split in words. SSB ran union replication of per-person logs in production for
  years.
- *Sharpening 1: "consensus among the parties" is right only for things held jointly.* Money out of a single-owner
  chain needs **no** consensus, only a guarantee that the owner cannot equivocate (Guerraoui et al.). Consensus
  among k parties is needed when k parties can spend the same thing. That matches our "two-key finality belongs to
  shared chains". For a payment between two personal chains, the job is no-equivocation plus the receiver's
  acceptance, not consensus.
- *Sharpening 2: rival facts need Sybil cost.* Kleppmann-Howard: anything not I-confluent (money, priority) cannot be
  immune to Sybil attacks. So whoever enforces no-equivocation for rival facts must be a set that is expensive to
  fake. For us that means PoS/PoM-weighted, not "any peer". Holochain leans on membranes, and SSB on the social
  graph.
- *Sharpening 3: H1 depends on content-addressed IDs.* Uniqueness becomes safe when the ID is the hash of the content
  (Kleppmann-Howard, Table 1). Standing as a derived view is a "materialized view", which is also safe. This matches
  the first change in our note's section 5. Today's local counter IDs would break H1.
- *Sharpening 4: priority and exclusive authorship are rival.* Neither is I-confluent. The options in the literature
  are trusted time (Holochain's whitepaper suggests Roughtime with earliest wins), M-of-N witnesses, or social
  resolution. Ceramic shows how earliest-anchor rules fail (section 3).

### H2. A single-owner chain needs witnesses who compare copies, not full consensus

**Verdict: shown for detection. Prevention and punishment are where the agent-centric systems fall short.**
Prevention does exist next door: transparency-log witnesses cosign heads before anyone accepts them, and Linera's
validators refuse a second block at the same height. Both rely on a set of witnesses that is fixed or expensive to
fake. The four gaps
below have known fixes: a meeting point for witnesses, cosigned heads, a Sybil-costly penalty, and an honest-fork
recovery path. They are items 3 and 4 in section 5, together with the Sybil cost of item 2.
- *Shown.* Holochain's agent-activity neighbors, Hypercore's spreading conflict proof, Linera's single-owner chains,
  and transparency-log witnesses are all "witnesses, not consensus" for single-writer logs.
- *Gap 1: the witnesses have to meet.* SSB's detection failed because a peer that never saw the first branch happily
  accepts the second. SUNDR proves a fork lasts as long as the victims don't compare notes. Holochain fixes this with
  one rendezvous: every action goes to the author's own DHT neighborhood. **In our design, overlap witnesses are
  scattered across many counterparties' chains, so a fork shown to two different counterparties may never be
  compared.** We need a rendezvous (publish chain heads to a known neighborhood or committee) or cosigned
  checkpoints before acceptance (C2SP style).
- *Gap 2: catching a fork afterwards does not prevent it.* For rival facts the recipient must wait and check (the
  Holo Finney delay) or require cosigned heads.
- *Gap 3: the penalty has to hurt.* Holochain's only penalty is ejection, and it admits this is weak where
  identities are cheap. Ceramic leans on "you'd lose your own history." Losing Noesis standing is a stronger
  penalty, but only if standing is costly to rebuild. That is the same Sybil question as sharpening 2.
- *Gap 4: honest forks happen.* Backup restores and second devices killed SSB identities. Holochain admits the case
  and its fix (meta-data hardening) is unbuilt.
- *New point the theory gives us:* for non-rival facts **a fork is harmless**. Both branches are valid signed facts,
  so just take the union of both (Kleppmann-Howard). The witness duty and the penalty only need to apply to forks
  that disagree on rival or order-dependent facts: balances, priority claims, finality votes. Punishing every fork
  punishes accidents and buys no safety.

### H3. Every overlap record should pay both parties

**Verdict: untested. No prior art here pays witnesses, validators, or storers per record.**
- What happens without pay: SSB's pubs ran on volunteers, and Planetary called them a hack. Every Holochain node
  holds and checks everything, unpaid (sharding is off). Ceramic's single anchoring service shut down along with the
  company's change of focus.
- The 2017 HoloFuel design counted on the *recipient* being "highly incentivized to publish the transaction". That is
  one-sided, which is exactly the fragility our note's second-pass argument flagged.
- *Risk to plan for:* paying per overlap record rewards fake overlaps. Holo's own credit-limit design needed
  "anti-gaming mechanisms to deter faking transactions." H3 has to be gated by the value layer's independence test,
  or it becomes a wash subsidy.

### H4. Payouts must scale with value created, not split a fixed pot

**Verdict: untested here; no protocol in this family prices contributions.**
- The nearest precedents: HoloFuel mutual credit (supply grows with real service, net zero, not launched) and
  Sensorica-style splits of realized income by social valuation (a pot, just a variable one). Nostr zaps are
  voluntary tips with receipts that are not proof of payment.

---

## 5. What Noesis would have to add (that these systems leave open)

1. A **value layer**: worth computed over reconciled facts. Nobody here has one.
2. **Paid witnessing** with Sybil cost, gated by an independence test (H3 plus anti-wash).
3. A **meeting point for chain heads** (a neighborhood, committee, or cosigning witnesses), so forks shown to
   different counterparties get compared.
4. **Fork penalties scoped to rival facts**, plus a recovery path for honest forks (a signed "abandon branch" record,
   or a settling window like Holochain's unbuilt meta-data hardening).
5. **Priority with published content**: commitments count only once their content is published, or within an
   enforced reveal window (the Ceramic lesson).
6. A **multi-party rival-write protocol with a real decision rule**. Holochain's open PR #5966 is a good starting
   point: commit and abort certificates, and no deciding on timers.
7. **Multi-device identity and backup restore from day one**, plus a story for deletion and compaction (SSB's and
   Planetary's top complaints).
8. **Sharding or partial replication**, and a cost model for it, before the network grows.

---

## 6. Design traps these projects hit

1. **Treating "we can detect it" as "we prevent it."** Holochain countersigning is not double-spend prevention, its
   fork warrants are not operational, and SSB's fork proofs were never built.
2. **No-equivocation with no meeting point.** SSB peers accepted whichever branch reached them first.
3. **Punishing honest forks the same as malicious ones.** Restoring from backup or adding a second device killed
   SSB identities ("irrecoverably stuck").
4. **Unpaid infrastructure roles.** Volunteer pubs, everyone-stores-everything validation, a single company-run
   anchor service.
5. **"Earliest anchor wins" without data availability.** This is Ceramic's data-withholding double-spend.
6. **Self-declared timestamps deciding anything rival** (Nostr `created_at`, SSB gatherings).
7. **Penalties that only bite if identity is expensive.** Ejection, or "you'd lose your own history."
8. **Core safety features parked behind flags for years**: countersigning, sharding, and DPKI, all behind flags
   since Dec 2024 or earlier.
9. **Token promises ahead of code.** HoloFuel was promised in 2018 and is still not live in Oct 2026. Linera
   reportedly ran out of money before mainnet. Ceramic's infrastructure was retired when the company changed focus.
10. **Whole-log, never-delete design on phones** (Planetary's first complaint).
11. **Per-author counters as identity.** Kleppmann-Howard show they can be corrupted; use hash links (our note's
    content-addressing item).

---

## 7. Corrections to our own note (`the-node-and-the-network-2026-10-22.md`)

1. Section 4 says Holochain and SSB "handle rival transactions by countersigning between the parties." **Partly
   wrong.** SSB doesn't handle rival transactions at all. Holochain says countersigning is not for double-spend
   prevention; its rival handling is fork detection plus ejection, or enzyme / M-of-N notaries. Countersigning has
   been off by default since 0.4.
2. "They have nothing like our value layer": **true at protocol level.** At app level, hREA/ValueFlows records
   contributions, and Sensorica assigns valuations by social process and splits income.
3. "How much of sections 3 to 5 Holochain already answers":
   - Section 3 (rival / non-rival split): answered in words by Holochain's 2024 whitepaper and formally by
     Kleppmann-Howard.
   - Section 4 (witnesses): Holochain's design is the reference, but its fork penalties are not shipped.
   - Section 5 (content-addressed records): Holochain already uses hash-addressed actions and entries.
   - Value: not answered at all.
4. "A chain with one owner doesn't need two-key finality": **supported** by Guerraoui et al. and by Linera's
   chain types. Add one condition. If a personal chain holds money or priority claims, the witnesses who check it
   for forks must be costly to fake, for example weighted by stake or standing. Otherwise an attacker can invent
   enough friendly witnesses to hide a fork. Weighting by standing is circular if a witness earns its standing from
   the very facts it witnesses. A standing snapshot frozen per epoch, or stake, avoids the loop.

---

## Method and limits

- Read on 2026-10-22: official docs, whitepapers (PDF text extracted locally), the Hypercore source (main branch),
  the Ceramic legacy spec, the NIPs, the C2SP witness spec, and arXiv papers. For the BEC paper I read the full
  text; for the others, the abstracts.
- From press only: Holo's 2018 raise figure and early delays, HoloPort shipping dates, and Linera's shutdown.
- **Not verifiable yet:** whether HoloFuel launches by 1 Nov 2026, and whether Holochain fork warrants ship in a
  0.7.x or 0.8 release. The code and the docs currently disagree on fork warrants.
- Sensorica's valuation process is described from Sensorica's own blog only. The peer-reviewed paper (MDPI
  Information 2020) blocked automated fetch.
