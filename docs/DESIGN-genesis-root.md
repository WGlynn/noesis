# DESIGN — the genesis root: how a contribution chain starts — ✅ RATIFIED (Will 2026-07-19)

> Status: **design ratified, mainnet-not-yet-built.** Answers "how should a contribution chain start?"
> (Will 2026-07-19). The testnet already shows a foundational root (`/contributions` id 1, `parent=null`);
> this doc fixes what that root *should* be and, crucially, what it must **score**. Companions:
> `first-citizens-ai-genesis-contributors.md`, `DESIGN-bootstrap-admission.md`, `[[project_noesis-genesis-bootstrap-decision]]`.

## 0. Two genesis, not one

The question "what is the first contribution?" conflates two different roots. Separate them and it resolves:

1. **Consensus genesis — the rules.** `chain_id`, the consensus mix, the finality rule, the bonded
   validator set: the `ChainSpec` / `Constitution` every node must agree on to compare state at all.
   This is the true block zero and it **already exists and is correct** (`node/src/chainspec.rs`).
2. **Value-DAG genesis — the first *contribution* others build on.** The origin of the provenance forest
   (`parent` links, `rpc.rs:231`). This is the "empty vs seeded" question the DAG view surfaces.

Candidates for (2), decided:
- **DeFi tools / applications** — ✗ downstream *leaves*, never the root.
- **Empty / nothing** — honest pure fair-launch (Bitcoin's instinct), but leaves the value DAG *rootless*:
  every early contribution is an orphan and the downstream-flow measure has nothing to price. Defensible;
  cold-start.
- **The constitution / a single commit (the codebase)** — ✅ these are the *same* answer. The codebase is
  the constitution made concrete: "here are the rules by which contribution is measured, and here is the
  mechanism that measures it." It is the root everything genuinely descends from.

## 1. The load-bearing theorem — the root must be UNSCORED

Noesis's value function measures **downstream realized flow**: you are worth what later builds on you
(`value_v5` / `flow_gate`; a parent link only ever CREDITS the ancestor, `rpc.rs:239-240`).

Ask what *everything* builds on: the constitution — every contribution is made *under* those rules.

**⇒ If the genesis root is scored, it becomes the ancestor of the entire DAG, and a downstream-flow
measure routes ALL value back to it. The authors of the constitution capture everything. That is a
premine in disguise, and it breaks fair-launch.**

Therefore the root must be **excluded from the measure**: it sits at the DAG origin, `parent = null`,
**standing 0** — a founding *record*, not a scored play. It is the **axiom**; you don't pay the axiom,
you prove theorems on it.

This is the *same structural move Noesis already makes one layer down*: **PoW is excluded from the
finality mix** (`FINALITY_MIX`, PoW is the reorgeable base) exactly as **the constitution is excluded
from value scoring** (it is the frame). The consistency is how we know it's right, not arbitrary.

## 2. "OG = 0" — what it does and does not mean

- **0 is the reference point, not a nothing.** Like the origin on a number line: everything is measured
  *from* it, so it cannot also be a point *on* the scale.
- **The founders do not get nothing.** They earn standing the *same way everyone else does* — by making
  real contributions *on top of* the frame (ideas, code, work others build on), which **are** scored.
  You are paid for what you build within the rules, never for having authored the rules. No rent accrues
  to protocol-authorship; founders compete on the same field as newcomers.
- **Precedent:** Satoshi's genesis-block coinbase (50 BTC) is **unspendable by construction**. Bitcoin's
  origin is a marker, not a payout. Noesis makes the same instinct a rule.

**One line:** *A contribution chain starts with the constitution — the shared rules that make
contribution measurable — recorded as an unscored foundational root, so provenance has an origin but no
one is paid for authoring the frame.*

## 3. Does *what* the genesis is affect growth rates?

**In the scoring math: no.** The root is 0, so its content never enters anyone's value calculation.
Growth is priced by the downstream flow of what's built *on top*, not by the root's nature.

**In the growth *dynamics*: yes — three channels, one of which is a real rate lever:**

1. **Presence ≫ content (the dominant effect).** Whether a genesis *exists* matters far more than what
   it is. With a root, contribution #1 can build on it ⇒ provenance depth ≥ 1 and the downstream-flow
   mechanism is live from the first move; with an empty genesis everything orphans until a chain
   organically forms (cold-start drag). *Having* a root accelerates growth; its content barely enters here.

2. **Novelty-space occupancy — the one hard rule.** Noesis scores novelty as distance from the
   already-committed **seen-set** (temporal-novelty + the θ_sim similarity floor over committed shingles).
   The genesis content seeds that set. A **fat genesis** — dumping the whole codebase as `data` —
   pre-claims a large slice of novelty space, so every later contribution resembling it scores **0**:
   you would suppress the exact "build on the protocol" contributions you want. A **thin genesis** (a
   commit *hash* / source-tree Merkle root — a marker, not a content dump) claims ~nothing and leaves the
   field open. **⇒ RULE: keep the genesis thin.** This is why §4 specifies `data = commit hash`, not the
   codebase text. (The ~250-char description seeded on the testnet is borderline-fine; a hash is strictly
   better for growth.)

3. **Semantic attractor + legitimacy (direction, not rate).** The genesis is the focal point of the whole
   tree — it orients *what* people build — and signals credible-neutrality ("no one was pre-paid"). A
   direction-and-trust lever on adoption, not a throughput knob.

**Monetary growth is orthogonal:** JUL emission is driven by PoW difficulty (`genesis_bits`) and
`reward_for_work` — the genesis *contribution* content has zero effect on issuance.

**Net:** genesis content is a *weak* rate lever, a *strong* direction/legitimacy lever, and carries
exactly one hard rule — keep it thin (a hash), or it eats novelty space and throttles early growth.

## 4. Implementation (next session — mainnet)

- Bake a genesis contribution cell into `ChainSpec::testnet()`/mainnet genesis ledger: `data` = the repo
  commit hash / source-tree Merkle root (the codebase snapshot), contributor = a genesis soulbound
  identity (`chainspec:38` already pairs validators with contributor keys), `parent = None`, **PoM
  standing 0** (genesis `pom = 0` already; keep the cell out of the scoring path so it is a visible DAG
  anchor that earns nothing directly).
- This is a consensus change (alters the block-zero state digest) ⇒ RED-first + parity, not a mid-session
  flip.
- Depends on identity-durability (chain-of-roots) so the genesis contributor's soulbound key outlives its
  256-signature tree — see the CONTINUE.md identity-durability gap.
- **Testnet caveat (honest):** the live testnet's foundational contribution (`/contributions` id 1) earned
  ~265 *novelty* standing because the v0 testnet scores every contribution by novelty-at-submission. That
  is the *right root with the wrong number* — a demo artifact. The genesis-baked mainnet root is the
  standing-0 design above.
