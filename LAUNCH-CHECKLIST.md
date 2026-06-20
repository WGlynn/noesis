# LAUNCH-CHECKLIST — Noesis (open-source release)

> 2026-06-20. **Starting point — expect to adjust/correct.** Decisions baked in this session:
> - **Open-source, forego patent** (Will). The moat is STRUCTURAL — an attribution network can only
>   be joined, not stolen ([[attribution-network-cannot-be-stolen-only-joined]]) — so no patent/stealth
>   is needed. Open-source is the *natural* form, and faster to launch. (One-way door: publishing
>   forecloses patentability — chosen deliberately.)
> - Two tracks: **A — RELEASE** (make it public) and **B — BUILD-OUT** (post-release; "come join us"
>   IS the launch mechanism). Tags: ✅ demonstrated · 🟡 designed · 🔴 unbuilt.

## The one substance gate (both tracks depend on it)
- [ ] 🟡→✅ **THE MOAT — un-gameable `v(S)` on REAL labels.** Seam is wired end-to-end
  (`load_prefs → train → v_outcome_floored → seed`); runs on SYNTHETIC labels today. Real closure =
  the DeepFunding-distill-over-sets outcome-label pull. **Data-blocked**, the one genuine long pole.
  ROADMAP's own verdict until it lands: "everything above is a reputation system."
  **Recommended sequencing: open-source NOW with honest demonstrated-vs-designed framing, invite
  collaborators (Tom/Bernhard) to help close it** — the structural moat means there's nothing to lose
  by being early, and "come join us" wants you public before it's finished.

## Track A — Open-source RELEASE (the actual launch)
- [ ] 🟡 **Whitepaper finalize** — v3.2 → ≤12pp; finish the plain-down; name-lock + formalize the
  Glynn equilibrium; move core/nucleolus + hybrid-finalization math to an appendix.
- [ ] 🔴 **LICENSE** — recommend Apache-2.0 (its patent-grant clause protects contributors; permissive
  is safe since fork-resistance is structural, not legal). CC0 also viable (matches the CMD repo).
- [ ] 🔴 **Public README** + build/run instructions (the node already runs + 2-node converges).
- [ ] 🔴 **`internal/`-vs-public triage** — which design notes ship vs stay internal.
- [ ] 🔴 **Delete stale `docs/WHITEPAPER.md`** (orphaned v0.1; competes with the `.tex`).
- [ ] 🔴 **Retire the stealth scaffolding** once public — private-repo, leak-gate, and the
  substrate-sync "scrub noesis" patterns become unnecessary (keep until the repo actually flips).
- [ ] ⚠ **(irreversible — Will only)** flip the repo public.

## Track B — BUILD-OUT (post-release; finite or v2, NOT launch blockers)
### Adversarial moat (mostly done; finite tail)
- [x] ✅ collusion detectors — circulation (bb) + Helmholtz–Hodge cycle-energy (cc).
- [ ] 🟡 collusion-slash wiring (dd, decided) — detection → economic penalty (`collusion_residual_by_identity`).

### Money layer (JUL) — a PORT, not an invention
- [ ] 🟡 **Port the Trinomial Stability System** (Will's theorem; `Joule.sol` + `ECONOMITRA_V1.2 §8.3`)
  → JUL money cell (Rust/CKB-VM): Ergon proportional anchor + elastic rebase + PI controller. Carry the
  **MED-6 audit fixes** (rebase-scalar cap, PI-output cap, multi-oracle median); add the **firewall
  invariant** (JUL mint ≠ PoM/standing) + **oracle-as-bonded-verified-compute**. See
  `internal/DESIGN-elastic-pow-money.md`. *De-risked: the mechanism + stability theorem already exist.*

### Deploy-coupled crypto (reference → live chain)
- [ ] 🟡 lock-sig DEPLOY half (`verify_sig` + ed25519/PQ suite).
- [ ] 🟡 on-VM single-use (k) + token-state.
- [ ] 🟡 on-VM finalization program (header-`now` sourcing + fixtures).
- [ ] 🟡 T7 #4 second half (witness syscalls + on-VM e2e).
- [ ] 🔴 genesis / chain-spec / P2P (today: reference runtime + tested 2-node convergence, not a network).

### Consensus productionization
- [ ] 🟡 stability LP / iterated-LP solver at scale (core/nucleolus reference exists).
- [ ] 🟡 on-chain slashing accounting + dispute window.
- [ ] ⚖ **(Will)** the 3 open consensus rulings — log₂ cleanup (live path already linear),
  `finalizes_pos_pom` wiring (T3 PoW-out-of-finality, currently stranded), and the `pow=0.10` vote
  question (#5; TSS framing suggests: JUL doesn't vote, the `pow` dimension is the liveness floor).

### Phase 4 — v2 (do NOT block launch)
- [ ] 🔴 training-signal export (value-weighted dataset from high-PoM blocks).
- [ ] 🔬 open-weight fine-tune loop (depends on the open-weights migration).

## The honest long poles
1. **Real-label `v(S)` moat** — data-blocked; the one credibility gate.
2. **Deploy crypto + genesis** — bounded, known engineering for a live chain.

Everything else is finite, a faithful port, or v2. The money layer and Phase 4 are the two things
outside readers will assume are done that aren't — be precise about that in any raise/announcement.
