# RESEARCH — networking + AI-native consensus (PRIVATE, stealth)

> Captured 2026-06-16 from parallel web-research (full-auto Noesis session). Decision-oriented;
> raw sources in the citations. Threads T1/T2 landed; T3 (PoW finality-lag), T9 (Ergo sub-blocks),
> T10 (Constellation DAG gossip) pending — appended on arrival.

## T1 — peer-discovery transport  (CKB-shape COMMITTED; transport open)

**Recommendation: build on `rust-libp2p`, LEAN profile** — QUIC + GossipSub v1.2 + Identify +
a **custom Bitcoin-style addr-gossip discovery** (replicate CKB RFC0012 as a `NetworkBehaviour`)
INSTEAD of enabling the Kademlia DHT. Gets modern QUIC transport (encryption+mux+no-HoL in one
handshake) and the most-hardened gossip mesh in the ecosystem, while pruning DHT weight that a flat
UTXO/cell chain does not need. Price: rust-libp2p is pre-1.0 (API churn, thin upstream — pin + budget
migrations).

Ranked:
1. **rust-libp2p (lean)** — QUIC/GossipSub-v1.2/DCUtR-NAT; modular ⇒ opt out of DHT. Heaviest deps but prunable.
2. **tentacle (CKB-native)** — lightest, mainnet-proven, RFC0012 addr-gossip = literally the
   Bitcoin-simplicity ideal; BUT TCP-only (no QUIC), near-solo maintenance, dated secio handshake.
   Defensible #2 if native-alignment + min-deps outweigh transport modernity.
3. **discv5** — excellent discovery-ONLY component (Kademlia/ENR over UDP); DHT + topic-discovery is
   overkill for a flat UTXO chain (Ethereum needs it for subnets; we don't), and it's not a full stack.

Cross-cutting: GossipSub v1.2 **IDONTWANT** = big block-propagation bandwidth win. NAT traversal
(DCUtR/AutoNAT/Relay-v2) only exists in libp2p. Eclipse-resistance: IP-subnet limits raise cost but
don't prevent eclipses without identity/stake ⇒ add chain-specific peer-scoring + anchor peers
REGARDLESS of library. **PoM angle: we HAVE an identity/standing layer (soulbound) — peer scoring can
be standing-weighted, a real eclipse-resistance edge most chains lack.**

## T2 — ML-native / "intelligent" consensus

**The literature's single safety rule = the rule Noesis already follows:** the learned model tunes
PERFORMANCE/LIVENESS; cryptographic structure owns SAFETY/FINALITY ("model proposes, structure
disposes"). Every DEPLOYED system keeps the learned signal advisory; no production L1 lets an ML scalar
produce finality. This is exactly our Role-C `evaluator` (advance/evidence, never mint) + `value_v8`
(corrupt-outcome-model ≡ v7 exactly). **Validation, not redesign.**

Two unsolved problems if you cross the line (learned scalar gates finality/mint/quorum):
- **reward-model gaming** — "Strategyproof RLHF" (2503.09561): existing preference-learning is NOT
  strategyproof; ONE strategic actor → arbitrarily large misalignment; impossibility bound = any
  strategyproof algo is worst-case k× worse (k=#reporters). Mitigation = Pessimistic-Median-of-MLEs.
  ⇒ a BT score that directly sets consensus weight is a reward-hacking surface in the SAFETY layer.
- **non-determinism** — consensus must be bit-replayable; float inference is not. Fix: integer
  fixed-point (scale ≥ 1e12 ⇒ empirically zero bitwise loss), block-hash-seeded, re-executed +
  byte-equality-checked (EigenAI mainnet pattern, <2% overhead). We ALREADY do fixed-point
  (`value_fixed` Q16, `finalization_fixed`/`settlement_fixed` Q32.32) ⇒ the determinism stack exists.

**Safe ways to make PoM consensus "intelligent" (ranked safety×payoff):**
1. **Bounded deterministic weight MULTIPLIER on a floor it cannot breach** — `weight = base ×
   clamp(f(score),[0.5,1.5])`, finality still needs a stake/standing quorum the score can't manufacture;
   score run integer-fixed-point, seeded, re-executed, slash-on-mismatch; wrap score in
   Pessimistic-Median-of-MLEs vs gaming. Corrupt model ⇒ at most mis-weights within clamp, never forges
   quorum/mint. Clamp bounds = CONSTITUTIONAL constant (ties to the T4 matrix-governance: physics layer).
2. **Learned leader-candidate SHORTLIST + adaptive timing (liveness only)** — model orders a shortlist;
   final leader picked by VRF/sortition over a MIN-size set (anti-censorship); safety untouched
   (AdaptiveDAG: −34% latency, no quorum/cert/format change).
3. **Learned anomaly PRE-FILTER feeding evidence, not verdicts** — ML flags → inputs to the existing
   deterministic slashing/challenge process (which still requires re-executable proof). Model never slashes.

**DO-NOT:** (1) let the score gate finality/mint/quorum; (2) float inference anywhere on the consensus
path; (3) treat model-agreement as truth ("Consensus is Not Verification" 2026 — errors correlate across
model families ⇒ agreement grows faster than correctness; re-execute a COMMITTED model, don't vote
models); (4) unbounded learned objective without a strategyproofness firewall; (5) LLM-scale model on the
hot path (ZKML = days/inference for LLMs — keep the BT coalition-feature scorer small + integer-friendly);
(6) let the model narrow leader/validator sets without a cryptographic floor.

**Math worth noting:** BT extends to Plackett-Luce for ranking SETS (leaders/validators); Myerson value
= Shapley on a COMMUNICATION GRAPH = the right object for a P2P validator topology (we already use
Myerson in `synergy`/`flow`); eigenvector/PageRank value-flow composes with it.

## T3 — PoW finality-lag vs PoS/PoM   ‼ FOUND A LATENT BUG IN `finalizes_hybrid`

**The bug:** the current rule sums `0.10·pow + 0.30·pos + 0.60·pom` at FACE value and finalizes at
2/3. But PoW finality is probabilistic/lagging — a freshly-mined (reorgeable) PoW block counts toward
*irreversibility* the instant it's mined. This conflates fork-choice ("best branch now?") with finality
("beyond dispute?"). An attacker who can transiently reorg the PoW tip can flip whether a checkpoint
crosses 2/3 ⇒ **PoW lag is a finality-SAFETY vector, not just a liveness nuisance.**

**Universal production pattern:** decouple block production from finality; the probabilistic layer NEVER
enters the immediate finality weight. Casper-FFG (PoW proposes, PoS votes on epoch checkpoints),
GRANDPA+BABE (finality kept deliberately a few blocks behind the tip), Babylon (BTC PoW = anchoring
only, separate stake-signed finality round), Decred (stake tickets veto/ratify PoW blocks). The ONLY
family that merges resources into one weight is **Minotaur** — and it pays with epoch-sampling + a UC
proof that *combined adversary < 1/2*, not a naive "sum hits 2/3 now." Caveat: any design that keeps PoW
in finality requires honest supermajorities on miners AND stakers AND PoM simultaneously (non-fungible).

**RECOMMEND #1 (dissolves the bug): remove PoW from finality.** PoS+PoM become the finality gadget;
PoW stays as production / ordering / sybil-cost / liveness (sybil resistance lives in PRODUCTION, not the
finality sum — keep PoW gating block production and it's intact). Concrete `finalizes_hybrid` change:
- PoW removed from the finality sum (moves to fork-choice).
- threshold renormalized to **2/3 of the PoS+PoM set** (= 0.667 × 0.90 = 0.60 of global), not 2/3 of a
  mixed-confidence total.
- **finalize a LAGGING prefix, not the live tip**; never reorg a finalized prefix (weak-subjectivity anchor).
- **ANTI-CONCENTRATION rule: no single dimension may unilaterally finalize** — require BOTH `pos ≥
  MIN_DIM` and `pom ≥ MIN_DIM`, so PoM's 60% can't capture finality alone. ← the one new risk this
  architecture introduces, directly mitigated.
- accountable slashing on the PoS+PoM finality votes (Babylon EOTS-style double-sign slash) + a stall
  fallback (deterministic gadgets can halt — Casper May-2023).

**Cost to accept:** finality power concentrates in PoS+PoM; **PoM at 60% is the finality kingmaker.**
AUDIT PoM's validator/identity distribution before shipping — if PoM is more concentrated than the stake
set, finality is effectively PoM-controlled. The anti-concentration + diversity rule + weak-subjectivity
checkpoints are the defense. #2 transitional = depth-discounted PoW (`0.10 × confidence(depth)`,
confidence = 1 − P_reorg(depth)) — strictly more honest than today, but reintroduces PoW lag into finality
latency (contradicts the goal). #3 status-quo face-value sum = NOT recommended.

**Ties to T2/T4:** `MIN_DIM`, the renormalized threshold, and any T2 weight-multiplier clamp are
CONSTITUTIONAL constants (the physics/constitutional layer of the value-matrix governance — T4), not
governance-tunable. This is the same "structure owns safety" line as T2.

## T9 — Ergo-style sub-block scaling   ✅ REAL MERIT (best fit; adapt, don't copy)

Ergo "Matrix" (2025, soft-fork): **ordering blocks** = full-PoW (`H<T`, ~2 min) carry FINALITY;
**input/sub-blocks** = `H<T/64` (~2 s) carry fast SOFT confirmation. Finality stays entirely in
ordering blocks — sub-block confirms are explicitly revertible ("in the leader's working set," not
settled). Compact weak-ID propagation (6-byte tx ids) = bandwidth win. Honest tradeoff: buys UX/
failure-detection (~17× faster), NOT settlement.

**Maps onto our chain better than onto Ergo:** the two-tier split is exactly a fast-soft / slow-final
decomposition for a chain that ALREADY has a contribution-weighted finality gadget. Adopt:
1. two-tier block model — **ordering blocks = the unit the PoS+PoM finality gadget votes on** (ties to
   T3: ordering block = the lagging-prefix checkpoint); sub-blocks = optimistic, revertible, seconds.
2. **re-derive the sub-block GATE from contribution-weight, NOT PoW `T/64`** ([[primitive_cross-port-fn-var-audit]]
   — re-derive each fn from the destination substrate's physics): sub-block proposed by current leader-set
   or a low-threshold standing quorum.
3. adopt compact weak-ID propagation now (free bandwidth win, pairs with the T1 GossipSub layer).
4. borrow Ergo's honest UX contract VERBATIM: wallets may show "soft-confirmed," high-value flows MUST
   wait N ordering blocks — encode as a protocol-level confirmation-tier API (revertibility never hidden).
5. YAGNI for v1: the first-class/second-class (miner-affected) tx split — defer.
Risk: still R&D, never mainnet, known sync edge cases ⇒ adapt the STRUCTURE, prove our own gate.

## T10 — Constellation Hypergraph DAG gossip   ✖ MOSTLY HYPE (one borrowable idea)

HGTP/PRO ("Proof of Reputable Observation", reputation-weighted, DAG ledger, metagraph snapshots).
Live project (Tessellation V3, Kraken listing 2025) but adoption is narrow enterprise audit-trail, and
**zero independent security analysis of PRO in 8 years** — token-skepticism warranted; "Layer 0 /
infinite scalability" is unfalsifiable marketing. The DAG-concurrent-validation core **fights the
committed CKB cell/finality shape** (we WANT a canonical sequence) — do not adopt.
**The one borrowable idea — and it CONVERGES with T1:** topology/trust-weighted gossip peer selection.
Don't take HGTP; take the idea via the mainstream path — **feed PoM contribution/standing scores into
rust-libp2p GossipSub's peer-scoring** so high-contribution nodes are preferentially grafted into
propagation meshes. Audited, formally analyzed, Rust-ready; no $DAG baggage. (Same conclusion as T1's
"standing-weighted peer scoring" note — two independent agents converged here.)

## T11 — Solana-style PoS vs value-function-native PoS   ✅ CAPITAL-ORTHOGONALITY IS A FEATURE

**Two NOs and a structural reason.** (1) NOT Solana-style — PoH is a clock not consensus; deterministic
leader schedule = censorship/MEV-targetable; decentralization actively degrading (validators −68%,
Nakamoto coeff −35%, 88% one client, 7 halts). (2) **NOT value/intrinsic-weighted stake** — and this is
the load-bearing finding: because PoM (60%) already carries the subjective value signal, the PoS (30%)
dimension's JOB is to be what PoM is NOT — objective, capital-at-risk, slashable, sybil-costly.

Three structural facts (all map to Will's own primitives):
- **Buterin subjectivity spectrum**: PoW objective · PoS weakly-subjective · reputation/value-score FULLY
  subjective. PoM (learned value model) is the most expressive AND most gameable/opaque axis. A consensus
  needs an OBJECTIVE ANCHOR a fresh node can verify without trusting a model — PoS-as-pure-capital IS that
  anchor. Value-weighting stake deletes the only objective axis.
- **Minotaur fungibility (CCS 2022)**: multi-resource security comes from the axes being INDEPENDENT — an
  attacker must beat all simultaneously. `corr(PoM, PoS)→high` collapses 3 axes to ~2: gaming the value
  model would capture BOTH. ⇒ orthogonality is literally where the security lives. (= [[primitive_multi-axis-robustness-for-architectural-defense]].)
- **Filter-coincidence / skin-in-the-game** ([[primitive_filter-coincidence-as-structural-edge]]): PoM is
  backward-looking, model-scored, gameable, and NOT slashable-as-cost. Capital-at-risk is the forward-looking,
  objective, SLASHABLE cost-of-attack PoM structurally lacks. PoM ⟂ PoS = subjective-value ⟂ objective-cost.

**Empirical caution:** the designs that DID make consensus-weight = intrinsic-value (NEM Proof-of-Importance
via tx-graph PageRank; Token-Curated Registries) were deployed and effectively ABANDONED — subjective scores
are gameable without a clear Schelling point (the same TCR failure that threatens any learned v(S) carrying
hard security weight). Intrinsic value's safe home = PoM + reward distribution (our `ShapleyDistributor`
pattern), NEVER the sybil-resistance weight.

**RECOMMEND:** PoS = pure capital-at-risk **× time-lock** (vote-escrow; time is the only objective+slashable
enrichment), **VRF private leader selection** (Algorand/Ouroboros — no pre-targetable leader, unlike Solana),
**Phragmén-flatten** stake across validators (counter plutocracy without making weight subjective). Keep all
"intrinsic value" in PoM. **One-liner: for a chain where PoM already IS the value-function, the PoS dimension's
value is precisely that it is NOT one.**

## SHIPPED from the research — T3 finality fix (runtime level)
`runtime::finality::finalizes_pos_pom` (3 tests, suite 250 green): PoW removed from the finality mix
(`FINALITY_MIX = {pow:0, pos:1/3, pom:2/3}`), 2/3-of-the-fast-final-set, + **anti-concentration rule**
(`MIN_DIM_BPS`, each dim must independently clear its floor). Tests prove: both dims finalize; a PoM whale
clearing 2/3-of-set is REJECTED for zero capital participation (capital-orthogonality T11 enforced in code);
PoW giant contributes nothing to finality. Core `consensus::finalizes_hybrid` (235-test) left intact.
`MIN_DIM_BPS` + the renormalized threshold are CONSTITUTIONAL constants (T4 physics/constitutional layer).

## Convergence (the architecture the research points to)
- **Transport**: rust-libp2p (QUIC + GossipSub v1.2) + custom Bitcoin/RFC0012 addr-gossip discovery;
  GossipSub peer-scoring weighted by soulbound PoM standing (T1 ∩ T10).
- **Block/finality**: two-tier — sub-blocks (fast, revertible, contribution-gated) under ordering-block
  checkpoints; PoW OUT of finality; PoS+PoM finality gadget on the lagging ordering-prefix with an
  anti-concentration rule (no single dim ≥2/3) + accountable slashing + weak-subjectivity (T3 ∩ T9).
- **Intelligence**: learned v(S) stays role-bounded + deterministic (fixed-point, re-executed); at most a
  CLAMPED weight multiplier — clamp bounds are CONSTITUTIONAL constants (T2 ∩ T4).
- **Open audit before any finality ship**: PoM validator/identity distribution (T3's flagged risk —
  PoM at 60% is the finality kingmaker).
