# Design note — Infrastructure incentives (who is paid to run the network, and why)

> **STATUS: DESIGN ONLY — nothing in this note is built.** Grounded 2026-07-13 against
> `node/src/` (HEAD `8695d65`). The mechanism CHOICE (§5) is a ⚑ Will/M3 decision; this note
> frames the option space and the constraint it puts on M3, it does not pin numbers or lock a
> mechanism. Honest labels: ✅ built · 🟡 designed · 🔬 open · ⚑ Will-gated · ❌ absent.
>
> **Why now (Will 2026-07-13):** *"Ethereum's biggest problem has always been: good with
> consensus protocol-level incentives, horrible at infrastructure incentives."* This must be
> settled as a DESIGN before M3 (`LOOP-PLAN-to-golive.md`), because M3 sets the JUL issuance
> split and the state-rent routing — the two money flows an infra subsidy would draw from.
> Retrofitting an infra subsidy *after* monetary policy is locked is precisely Ethereum's trap
> (EIP-4444 history-expiry is that retrofit, done painfully and late).

## 1. The real diagnosis — it is a public-goods problem, not an oversight

Ethereum did not forget to pay infrastructure. It priced the wrong *kind* of good.

- **Block production** is a **rival, excludable** good: one proposer wins the slot and captures
  issuance + priority fees + MEV. Markets provision rival goods well (often over-provision them
  — the MEV arms race).
- **Serving, storing, and relaying** are **non-excludable public goods**: serve a block once and
  everyone free-rides; store history and others depend on it for free. Markets systematically
  *under*-provide public goods (free-rider problem).

So Ethereum's fee market prices **inclusion**, never **availability**. The predictable result:
production is over-provisioned, while availability collapses onto a few off-protocol SaaS
providers (Infura/Alchemy became the real default RPC), and history storage became an unfunded
liability. The lesson for Noesis: **pricing production (PoW/JUL) and finality (PoS+PoM) does not,
by itself, price the availability layer** — and a *value chain* whose data no one is paid to serve
is not usable by the minds that give it value.

## 2. The 2×2 for a Noesis node operator (grounded, honestly marked)

| | **Explicit** (protocol pays directly) | **Implicit** (indirect benefit) |
|---|---|---|
| **Extrinsic** (money) | ✅ Production → JUL coinbase (`jul.rs`, coinbase mint `runtime.rs:822`). ✅ Finality/contribution → PoM franchise + state-bytes. ❌ **Serving / storage / RPC / bandwidth → nothing** (`gossip.rs`, `net.rs`, `sync::serve` are capabilities, not paid roles). | ◐ Liveness-decay: exercise your franchise or weight decays (`last_heartbeat` / franchise-decay `horizon`, `runtime.rs:47`; **default `horizon = 0` ⇒ decay OFF** today). ◐ Validators run infra for *themselves*, not the public. MEV subsidy deliberately absent (commit-reveal lineage). |
| **Intrinsic** (identity/mission) | ◐ **Soulbound standing as recognition** — *if* infra-provision earned standing (not wired). | ◐ "Proof-of-Mind" ethos / belonging. Real for early believers; does not scale (Ethereum's "run a node for decentralization" had this and it was not enough). |

**The one box that decides the network's fate is extrinsic + explicit for serving/storage, and
today it is the same empty box Ethereum has (❌).**

## 3. Where Noesis is *structurally better positioned* than Ethereum

These are latent advantages — real, but **none is wired to infrastructure today.**

1. **The funding source already exists: state-rent.** Ethereum charges one-time gas then stores
   state free forever (the bloat externality, no revenue to pay storers). The Nervos-CKB lineage
   Noesis inherits prices state *occupation* as an ongoing rent (state-bytes, mint↔decay,
   `SYSTEM-MAP.md` layer 7). That means the money to pay storage providers is *already flowing in
   the system*; the missing half is **routing** rent to the actual storers. Ethereum cannot do
   this cleanly because it never metered occupation. (Mechanism-level claim; specific CKB
   secondary-issuance parameters are not asserted here — read them before quoting any number.)
2. **Infra-provision can be a first-class contribution type.** The whole thesis is *measure and
   reward contribution*, and running reliable infrastructure **is** a contribution. Ethereum's
   value function only scores "did you attest correctly"; Noesis's `v(S)` value-oracle
   (`SYSTEM-MAP.md` layer 8) is general and could score "did you serve / store / stay available,"
   paid in **soulbound standing**, not only money. This engages an intrinsic reputational motive
   crypto usually cannot offer (staked capital is fungible and identity-free). This is *on-thesis*,
   not a bolt-on. (Caveat: it inherits the same un-gameability problem as all PoM — a claim of
   serving must be *proven*, see §5.)
3. **Stateless verification dissolves part of the problem instead of subsidizing it.** The
   verification track (Phase-2 UTXO commitment ✅; Phase-3 zkVM recursion 🔌 Linux-blocked,
   `SYSTEM-MAP.md` layer 5) lets a light client *verify a proof* rather than needing a full node
   to serve it state. That shrinks the serving burden structurally ([[bottleneck-dissolution]]):
   dissolve where you can, then subsidize only the irreducible remainder (raw data availability /
   history, which proofs cannot conjure from nothing).
4. **Absence is already self-slashing (partially).** The heartbeat/franchise-decay
   (`runtime.rs:47`) gives finality participation a built implicit incentive (stay online or lose
   weight), where Ethereum needs explicit attestation rewards *plus* slashing. It needs turning on
   (`horizon > 0`) and calibrating (a ⚑ number), but the mechanism exists.

## 4. The honest gaps (real risks, not "good enough")

- **Standing ≠ online.** A high-PoM contributor who does not run a node contributes zero finality
  weight. Decay *punishes* absence; nothing *pays* the non-validating, public-serving role (the
  RPC / archive / relay layer that makes the chain usable by others).
- **Serving / RPC / archive / bandwidth are unpriced** (§2, ❌). Same trap as Ethereum. If M3
  locks the JUL issuance split and rent routing without carving an infra subsidy, we inherit the
  retrofit problem.
- **Seed-node centralization** (`LOOP-PLAN` L7). The hosted seed node is founder infra; who runs
  seeds/bootstrap afterward, and why?

## 5. The design — meter + fund + dissolve (⚑ mechanism choice for Will/M3)

Three levers; Noesis can uniquely combine them. **The choice among the metering mechanisms below
is ⚑ (Will/M3); this note recommends, it does not decide.**

- **METER it** (make the public good verifiable/excludable). Options, all with mature prior art —
  do NOT reinvent:
  - proof-of-storage / proof-of-spacetime (Filecoin lineage) — prove you *still hold* the data;
  - proof-of-serving / proof-of-access (Arweave lineage) — prove you *served* a request;
  - availability sampling — cheap probabilistic checks that data is retrievable.
- **FUND it.** Route a slice of the **state-rent stream** (the natural, occupation-priced source)
  and/or a small **JUL issuance carve-out** to *proven* providers. Rent is preferred (it is
  demand-funded and does not dilute), issuance is the fallback for the bootstrap phase before rent
  volume exists.
- **PAY it in standing (optionally).** Route the payout — or a portion — through PoM as an
  infra-contribution type, so reliable infra earns **soulbound reputation**, not just money
  (§3.2). This is the Noesis-native move; it also means the un-gameability moat must cover
  infra-claims (a claimed-but-not-performed service must be refutable, same dispute/slash machinery
  as `dispute::resolve_refuted`).
- **DISSOLVE first.** Use cheap stateless verification (§3.3) to shrink the serving burden, then
  subsidize only the remainder. The cheapest infra subsidy is the one you do not need to pay
  because verification made a full serving node unnecessary.

**Recommended shape (for the M3 loop to gate against, not a locked decision):** rent-funded +
proof-of-storage-metered + payout routed through PoM standing, with stateless-verification
shrinking the metered surface over time. This keeps the funding demand-priced, makes infra an
honest contribution, and reuses the existing dispute/slash machinery for the un-gameability edge.

## 6. The constraint this puts on M3 (the actionable output)

M3 (`LOOP-PLAN` — JUL economics: issuance split, rent routing, retarget numbers) **must reserve a
routable slice for the infra subsidy** rather than allocating 100% of issuance to production
(miners) and 100% of rent to the treasury/decay sink. Concretely, before M3 pins numbers:
1. decide whether the infra subsidy draws from rent, issuance, or both (§5 FUND);
2. leave a governable `Constitution` parameter for the split, defaulting to a safe/inert value
   (⚑ number — do not invent it here);
3. do not hard-wire a two-way (miners / treasury) split that would have to be reopened to insert a
   third (infra) recipient.

## 7. Suggested loop-plan slot

Add an **`L-INFRA`** row to `internal/LOOP-PLAN-to-golive.md` (I did not edit that file — it has
uncommitted working changes; add the row when it is clean):

> `L-INFRA` | Infrastructure incentive layer — meter (proof-of-storage/serving) + fund (rent
> and/or JUL carve-out) + pay-in-standing, dissolve-via-statelessness | DESIGN→COLD | gates M3 |
> 🟡 designed (this note); mechanism choice ⚑

Nothing here is built. The next concrete step is the ⚑ mechanism decision (§5); until then M2 (the
PoW work dimension) proceeds independently — it does not depend on this note.
