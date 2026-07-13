# Noesis — System Map (the living connection graph)

> **Why this exists.** Will 2026-07-13: *"connecting all the puzzle pieces is something you need to
> internalize and instantiate yourself. I can't be here forever connecting things."* This is that
> instantiation: how every piece connects, its honest status, and what "start a chain" actually
> requires — so the integration lives in an artifact, not in one person's head. **Maintain it every
> build step:** before a slice, check what it connects to / depends on / blocks; after, propagate what
> it just enabled. Honest labels: ✅ built · 🟡 designed · 🔬 open · ⛔ blocked · 🔌 deploy-coupled.

## Layers

| # | Layer | What | Status | Connects to |
|---|---|---|---|---|
| 1 | **Rulebook** | UTXO invariants: value-conservation, no-double-spend, no-spend-of-nonexistent — the rules "easy to STATE" | ✅ | 2, 4, 5, 6(sync re-validates) |
| 2 | **State machine** | `Node::apply` / `Ledger` / `state_digest` — replay canonical blocks ⇒ deterministic state | ✅ | 1, 3, 6 |
| 3 | **Consensus** | NCI mix (PoW/PoS/PoM); finality = PoS+PoM, PoW-excluded, anti-concentration floor; equivocation slash-before-count (live) | ✅ | 7(standing=weight), 8(v(S)→PoM) |
| 4 | **Crypto** | **Lamport** PQ hash-based one-time sig. PK = 32-byte blake2b root in `lock.args`; verifier linked (`spend_is_authorized`→`lamport::verify`, `noesis-core`, no_std/riscv). Only primitive = blake2b (no ECC). | ✅ ref · 🔌 on-VM port | 1(spend rule), 5 |
| 5 | **Verification** | FOLLOWED = `validate`/replay + Phase-0/1 pure rulebook ✅ · RIGHT = Phase-4 Isabelle FV ✅ (machine-checked) · CHEAP-STATELESS = Phase-2 UTXO commitment ✅ + Phase-3 zkVM recursion 🔌(Linux) | mixed | 1, 6(sync), 8 |
| 6 | **Networking (T1)** | slice-1 persist+codec ✅ · slice-2 transport ✅ · slice-3 gossip ✅ · slice-4 sync (join, re-validates) ✅ · slice-5 noesisd wire + 2-node demo 🟡 | 4/5 ✅ | 1(re-validate), 2(replay), 5 |
| 7 | **Tokenomics** | standing (soulbound franchise = consensus weight, earned) ✅ref · state-bytes (capital, 1 PoM = 1 byte, mint↔decay) ✅ref · **JUL** (money, PoW energy-peg) 🟡 · **VIBE** (governance) 🟡 | mixed | 3(standing=weight), 8(earned via v(S)), 9 |
| 8 | **AI value-oracle** | `v(S)` seam ✅ (the plug) · `NoveltyOracleV0` ✅ (heuristic, designed-not-learned) · learned-v(S) 🔬 (**the moat**; first real-data test inconclusive) · CRPC fuzzy meta-consensus 🟡 (Tim Cotten's spec) | ✅ seam / 🔬 moat | 3, 7(standing), 9(amend canonical v(S)) |
| 9 | **Governance** | Constitution 3-layer (physics near-immutable / constitutional verifier-gated / weights bounded); amendment coherence socket ✅ · dimension-set matrix ⛔ (deferred to the confluence engine — partner, terms-first) | ✅ socket | 7(VIBE), 8(canonical-oracle version) |

## The load-bearing cross-connections (the "puzzle")

- **standing = consensus-weight = earned-via-v(S).** One object seen three ways: tokenomics calls it the
  soulbound franchise (7), consensus calls it PoM finality weight (3), the value-oracle is *how* it's
  earned (8). Touching any one implicates the other two.
- **The v(S) seam (8) is the plug for tokenomics' #1 open item.** The learned value-oracle is BOTH the
  tokenomics moat (7's "value-oracle is the core open problem") AND the AI axis. Today's seam is the
  socket it drops into — no rebuild. Governance (9) picks the *canonical* v(S) version by amendment.
- **sync (6) trusts the RULES, not the peer.** It re-runs the rulebook (1, FOLLOWED); the rules being
  RIGHT is FV (5); a *cheap* proof-carrying join (verify a zkVM recursion proof instead of replaying)
  is the Phase-3 endgame (5, Linux-blocked). Crypto (4) `spend_is_authorized` is IN the rulebook sync
  re-validates.
- **Lamport ⊗ UTXO = free one-time safety.** A cell is consumed once ⇒ its lock key signs once ⇒ exactly
  Lamport's requirement. Hash-only ⇒ post-quantum AND "no hardcoded curve," AND it ports to the on-VM
  type-script (no_std).

## Genesis-readiness — what "starting a chain" actually requires

- ✅ **Required and present (design/reference layer):** honest empty genesis (PoM earned, never
  pre-minted), standing-mint + state-byte rules, consensus mix + finality gate, Lamport lock verifier,
  the rulebook + `validate`. `noesisd` already boots this honest genesis.
- ⚠ **OPEN — the genesis bootstrap paradox (Will 2026-07-13; corrects an earlier "no PoW" claim):**
  PoS needs stake (= state-bytes, minted by PoM); PoM = 0 at genesis; no ICO / no pre-mint ⇒ nothing
  is allocated ⇒ neither PoS nor PoM can secure or mint block zero. The **only allocation-free**
  primitive is **PoW** (energy → blocks + coins, permissionless, Bitcoin-style), so PoW is the likely
  genesis bootstrap scaffold: it secures the earliest blocks while the first contributions finalize →
  first PoM → first state-bytes → PoS becomes possible → finality shifts to PoS+PoM → PoW recedes to
  production + the JUL money layer. The mix is **dynamic** (PoW-heavy @ genesis → PoM-dominant steady
  state). "No PoW" holds only for *steady-state finality*, NOT genesis. **A genuine open founding
  decision, not pinned in docs:** PoW-bootstrap (ethos-pure) vs a founding bonded-set (small
  pre-allocation). JUL/VIBE remain designed, not required for the steady state.
- 🔬 **The real gate is QUALITY, not presence:** the learned value-oracle (8). A chain can *start* on
  the honest heuristic v0; the un-gameable measure is the moat and is open (inconclusive first test).
- 🔌 **Deploy-coupled before a public testnet:** the on-VM type-script ports (Lamport, finalization,
  UTXO retirement), a real zkVM receipt (Linux), the persistent validator registry (T1), and T1 slice-5
  + a hosted seed node.

**Synthesis:** the *steady-state* design is largely built at the reference layer, but **genesis is NOT
fully pinned** — the bootstrap decision above (PoW-scaffold vs founding bonded-set) is an open founding
call, and without it "who wins block zero" has no answer. Distance to a *real* launch: **(0) pin the
genesis bootstrap**, (a) on-VM/deploy ports, (b) JUL/VIBE integration, (c) the value-oracle quality (the
moat), (d) T1 slice-5 + a seed node.
