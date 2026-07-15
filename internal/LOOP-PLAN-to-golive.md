# Noesis — Loop Plan to Go-Live (the discrete, loop-sequenced path)

> **Purpose (Will 2026-07-13):** turn *"I don't know when it'll happen"* into *"N loops, here's the DAG,
> here are the walls."* Each **LOOP** = plan (2 planners) → build (lean, RED-first) → **Council** (adversarial
> personas) → **Pragma** (code≡docs confluence) → commit+push. The plan gives a countable structure people
> can be told; it does not invent wall-clock dates it cannot honor.
>
> **Grounded in** (read these, cite by row): `internal/MVP-SCOPE-JULY-2026.md`, `internal/SYSTEM-MAP.md`,
> `internal/STATUS-LEDGER.md`, `ROADMAP-2026-2028.md`. **Honest labels (NO ROUND-UP):** ✅ built ·
> 🟡 designed · 🔬 open-research · ⛔ blocked · 🔌 deploy-coupled · ⚑ Will-gated decision.
>
> **Stamp:** first written 2026-07-13 (HEAD `bf781fc`, lib 323 green). **RECONCILED 2026-07-15 (HEAD `34ee644`,
> lib 328 green) — 40 commits since the stamp drained almost every decision-unblocked cold build.** The
> spine + money track are essentially BUILT at the reference/on-VM layer; what remains is the **deploy pole**
> (5 deploy-coupled binding flips, all still `false`) + a short ⚑-economics sitting + the genesis/P2P engineering
> pole + the undated research moat. The 07-13 counts below ("~4-5 loops" / "~10 loops") are SUPERSEDED by the
> reconciled ledger — read the RECONCILED rows, not the original Status column.

---

## The two "done"s — never conflate (MVP-SCOPE §0)

- **GO-LIVE-FLOOR** — a launchable network *as the floor*: a strategyproof contribution ledger with
  capital-orthogonal PoS+PoM finality, soulbound franchise, zero fees, burn-only slashing, **AND the JUL
  money layer live**. JUL is THE e-cash of the value chain (Will 2026-07-13: *"only consensus-deferrable,
  but it's THE e-cash of the system — we cannot launch without it"*); a *value chain* does not go live
  without its money. **Launch copy never claims the moat.** ← this is the **datable** destination.
- **FULL-THESIS** — floor **+** the learned-`v(S)` un-gameability moat **+** HCE's full three properties.
  ← **honestly undatable** (research-gated). The demonstrated-vs-designed release framing is what lets the
  floor ship while these stay open.

The distinction dissolves the apparent contradiction in the status maps: the moat (NULL twice on real
labels, STATUS-LEDGER MOAT-1) and HCE (M2/C4 open) block the **thesis claim**, not the **floor launch** —
provided copy never claims more than the floor.

## The walls a loop CANNOT bulldoze (this is why honesty beats a fake date)

- **⚑ WILL-PEN — the #1 blocker (MVP-SCOPE §1.A / §4).** One sitting, a handful of consensus-affecting
  decisions that every COLD build queues behind: the **vesting window `W`** + a **`v(S)`-independent
  PoM-finality input**, the **`Standing.pom`→`Validator.pom` bridge shape** (must be born `W`-discounted,
  never a raw `v(S)` pass-through), **θ_sim ratify-or-hold**, **MIN_DIM_BPS** rider, and **I-2 go/no-go**.
  Deciding late forces a bridge rebuild — so this gates the spine.
- **⚑ GENESIS BOOTSTRAP — an open founding call (SYSTEM-MAP genesis-readiness).** PoW-scaffold (ethos-pure)
  vs a founding bonded-set (small pre-allocation). Without it, "who wins block zero" has no answer. Not a
  build; a decision.
- **🔬 RESEARCH — undatable (MVP-SCOPE §2, STATUS-LEDGER).** Learned-`v(S)` moat — the deep-ancestry
  dataset is now ✅ **SUPPLIED** (2026-07-15, crates.io, `data/crates/RESULTS.md`) and the predictive half is
  NULL a 3rd time on non-degenerate data ⇒ the moat rests on the **structural defense**, not a predictive
  win; still open = a real *adaptive* adversary + the general graph-iso theorem — and HCE M2/C4 theorems
  (+ M3 `p`-supplier, M4 symmetric-lie elimination). Run as **background tracks**; never place them on the
  critical path or a schedule.
- **🔌 DEPLOY-INFRA.** A real zkVM recursion receipt needs **Linux/WSL2** (this box has none). Genesis /
  chain-spec / P2P is the **long engineering pole** after the cold builds.

## The reframe that answers "when" (load-bearing)

The reference node is built and green. With THE LOOP, a cold build is a **session, not a week** (JUL inc-3
— a full plan→build→Council→Pragma cycle — ran in one session). **So the schedule is no longer bounded by
build effort; it is bounded by (a) one Will-decision sitting and (b) Will's review cadence on the
consensus-affecting loops, plus the genesis/P2P engineering pole.** That is a countable, honest answer.

---

## Loop ledger — the discrete units

Tier: **DI** = deploy-independent, runnable now · **COLD** = consensus-affecting (⚑-gated, RED-first,
extraction-audit + Will-review before merge) · **DEPLOY** = 🔌 substrate · **RESEARCH** = 🔬 undatable.

| Loop | Piece (SYSTEM-MAP layer) | Tier | Depends on | Status @ RECONCILED 2026-07-15 |
|---|---|---|---|---|
| **L0** | **Reconcile MVP-SCOPE §1 vs HEAD** | DI | — | ✅ done 2026-07-13 (+ this 07-15 reconcile) |
| **⚑-D** | **Will decision packet** — W + PoM-finality input + bridge shape + θ_sim + MIN_DIM_BPS + I-2 | ⚑ | L0 | ✅ **RATIFIED 2026-07-11** (D1–D5, `DESIGN-vesting-W` §4); MIN_DIM_BPS deferred |
| **L1** | Finalization ELF twin-update → `finalizes_pos_pom_fixed` (5) | COLD | L0 | ✅ **SHIPPED** (`df8f05e`) |
| **L2** | Circularity fix / **vesting window `W`** (7,3) | COLD | ⚑-D | ✅ **SHIPPED** — Phase-3 refuted-set AND-gate (`11d5785`); inert at W=0 until ⚑ number |
| **L3** | `Standing.pom`→`Validator.pom` **production bridge** (7→3) | COLD | L2 | ✅ **SHIPPED** (`runtime.rs:626`) |
| **L4** | Invariant pins + doc-coherence (E) | DI/WARM | L0 | ✅ pins SHIPPED (`8695d65`); 🟡 residual doc-coherence tick owed (MVP-SCOPE §2 / TOKENOMICS JUL over-generalization) |
| **L5** | Bound B commit-deposit (liveness) — refund-on-contribution / forfeit-BURNS-only | COLD | L4 | ✅ **SHIPPED** (`84f432c`, 8 tests; Council caught 2 real bugs) |
| **L6** | On-VM enforcement parity (4,5,7) | DEPLOY | L1,L3 | ✅ **reference/on-VM BUILT** — lock-sig twin ✅, PoM intake floors ✅, similarity floor ✅, index root-transition ELF ✅ (`34ee644`); 🔌 remaining = 5 BINDING FLIPS (all `false`) + on-VM single-use/nullifier crypto |
| **⚑-G** | **Genesis bootstrap decision** — PoW-scaffold vs founding bonded-set | ⚑ | — | ✅ mostly SETTLED (PoW starts genesis + bonded PoS finalizes block-0, `DESIGN-vesting-W` §2.5); open sliver = money-in-genesis-set (rec: moneyless+decaying) |
| **L7** | Genesis / chain-spec / P2P + hosted seed node (6) — **the long pole** | DEPLOY | L6, ⚑-G | 🔌 (2-node convergence ✅, not yet a public network) |
| **R1** | Learned-`v(S)` moat on a deep-ancestry outcome-labelled dataset (8) | RESEARCH | data hunt | 🔬 **dataset DISCHARGED 2026-07-15** (crates.io, 299,775 crates, ancestor-singleton 0.18 vs DeepFunding 0.83; `data/crates/RESULTS.md`, `5d1a084`) ⇒ predictive half NULL a **3rd** time, now DECISIVE (topology excluded) ⇒ **reframe CONFIRMED: moat = structural defense, not the predictive win**. Iso-gate ran+passed on real data (caught+fixed a real B1-determinism bug). **Still undated:** real *adaptive* adversary + general graph-iso theorem |
| **R2** | HCE M2/C4 theorems + M3 `p`-supplier + M4 symmetric-lie elimination (3,8) | RESEARCH | — | 🔬 conjecture; **undated** |

### PoW-axis + money track (JUL — core consensus, launch-required; Will 2026-07-13)

**None of the three NCI axes is launch-deferrable.** The NCI *is* a 3-dimensional consensus — `pow 0.10 /
pos 0.30 / pom 0.60` (`lib.rs:3806`) — and you cannot launch a network with two axes and fork in the third
(Will 2026-07-13: *"what are we gonna do, launch with only 2 and add the 3rd through a fork? makes no
sense"*). PoW's layer **is** JUL, so JUL is core-consensus launch infrastructure, not a parallel money
nicety. At genesis the mix is designed **PoW-heavy and dynamic** — PoW starts the chain and issues the
first coins from energy with no premine (the only allocation-free block-zero primitive, SYSTEM-MAP genesis;
PoM = 0 and accrues → weight migrates toward the `0.10/0.30/0.60` steady state), so **PoW matters MORE at
launch than in steady state.**

**The ONLY precise "deferrability":** PoW is excluded from the LOCKED `FINALITY_MIX` sub-decision
(`runtime.rs:956-960` = `pow 0.0 / pos 1/3 / pom 2/3`) because PoW is reorgeable (`runtime.rs:944-949`), so
*steady-state finality-safety* runs on PoS+PoM. That is one sub-mix in steady state — NOT "JUL/PoW is
deferrable for launch." ⚠ **Doc-coherence fix owed (folds into L4):** MVP-SCOPE §2 "JUL deferrable" +
TOKENOMICS "core needs no PoW" over-generalize that narrow finality-safety claim into a launch-wide one —
correct them so a collaborator reading the chain isn't misled.

Built so far (RECONCILED 2026-07-15): inc-1 ✅ (`e51e164`) issuance · inc-2 ✅ (`56d506f`) coinbase settlement ·
inc-3 ✅ (`bf781fc`) counter-cyclical reserve (shadow) · **PoW work-dimension arithmetic + ENFORCEMENT ✅**
(`23a90f0`/`43921f9` — `block_work` returns real `work_from_target` under `pow_enforced`, `runtime.rs:703`; Lever-A
no longer a `=1` stub) · **M3 pow-arithmetic ✅** — ASERT retarget (`cc3254f`), work-clock ceiling (`62a0f4b`),
N-way coinbase split (`6df23e7`), `target_to_compact` (`cd915de`) · **committee-attested clock ✅** (`6184ac9`) +
clock ENFORCEMENT ✅ (CLK-1 `32b87b5`, CLK-2 `4e52ae6`) · **never-halt liveness detector ✅** (`697fc30`) ·
emission-ramp DROPPED (`4a8b6b9` — JUL elasticity dissolves deep-capital, no ramp needed).

| Loop | Piece | Tier | Depends on | Status @ RECONCILED 2026-07-15 |
|---|---|---|---|---|
| **M1** | JUL inc-3b — reserve consensus wiring: skim/top-up at the coinbase-mint site + protocol-spend-only reserve cell | COLD | `CONTROL_BINDING_ACTIVE` flip (runtime.rs:411) | 🟡 designed (inc-3 seams); the ONE remaining decision-unblocked-ish JUL build, but couples to the keyless-reserve-cell deploy flip |
| **M2** | PoW + genesis issuance — arithmetic + enforcement + retarget + clock DONE; remaining = issue JUL from block 0 in the genesis chain-spec | COLD/🔌 | ⚑-G (PoW starts genesis) | ✅ arithmetic+enforcement+retarget+clock SHIPPED; 🔌 genesis chain-spec issuance is deploy-coupled |
| **M3** | JUL economics live — plug the ⚑ numbers into the built machinery: genesis bits, retarget cadence, work-clock-ceiling `K`, reserve activation numbers + miner-reflexivity pass | ⚑ | M1, M2; ⚑ numbers | 🟡 mechanism BUILT (all M3 pow-arithmetic ✅); **⚑ numbers = the economics sitting** (Will owns) |

> Lever A (the production-cost anchor) is now LIVE-capable: `block_work` returns real mined difficulty under
> `pow_enforced` (`runtime.rs:703`). What's left for "JUL live as money" is the genesis chain-spec that turns
> `pow_enforced` on from block 0 (M2 deploy half) + the ⚑ economic numbers (M3) — not more mechanism.

### Deferrable, decision-unblocked (NOT launch-blocking)

| Loop | Piece | Status |
|---|---|---|
| **L-VIBE** | VIBE governance token | 🟡 designed; orthogonal to the capture-resistant cycle; naming-confirm only |

> Only VIBE (governance, not money) is genuinely launch-deferrable. JUL is NOT here — it is a core consensus
> axis (PoW) + the e-cash + the genesis money bootstrap; see the PoW-axis + money track above. The only
> precise sense in which PoW is "deferrable" is its exclusion from the `FINALITY_MIX` sub-decision
> (steady-state safety) — never launch-deferrable.

---

## The DAG + loop-count to GO-LIVE-FLOOR

```
L0 (reconcile)
  ├─→ ⚑-D (Will sitting) ──────────────┐
  ├─→ L1 finalization ELF ─────────────┤   (decision-unblocked; starts before ⚑-D)
  └─→ L4 pins + doc tick ──────────────┤
                                        ▼
  consensus spine:  L2 vesting-W → L3 bridge → L5 Bound B → L6 on-VM parity ──┐
                                                                              │
  money track:      M1 reserve-wiring ──────────────→ M3 JUL economics live ──┤
                              M2 PoW + genesis issuance ──┘                    │
                                        ▲                                      ▼
                            ⚑-G genesis bootstrap ─────────→ L7 genesis / P2P + seed node
                                                                              ▼
              GO-LIVE-FLOOR (public testnet: contribution ledger + PoS+PoM finality + JUL e-cash live)

Parallel, undated background tracks:  R1 moat data hunt · R2 HCE M2/C4
```

**Honest count — RECONCILED 2026-07-15 (HEAD `34ee644`, 40 commits since the 07-13 stamp):** the 07-13 count
was "~4-5 build loops"; those loops have now SHIPPED. The spine is done at the reference layer (L1 finalization
ELF ✅, L2 vesting-W Phase-3 ✅, L3 bridge ✅, θ_sim ✅, I-2 ✅), **L5 Bound-B commit-deposit ✅** (`84f432c`),
governance authority-tiering ✅, and the WHOLE money-track MECHANISM ✅ (PoW arithmetic + enforcement + ASERT
retarget + work-clock ceiling + committee clock + clock enforcement + never-halt). L6 on-VM parity is BUILT at
the reference/on-VM layer (lock-sig, PoM intake floors, similarity floor, index root-transition ELF `34ee644`).
**⇒ There are essentially NO decision-unblocked cold builds left** (CONTINUE.md 2026-07-14 PM verified this;
this reconcile confirms it). **What genuinely remains, in three honest buckets:**

1. **⚑ Economics sitting (Will owns, small):** M3 numbers — genesis bits, retarget cadence, work-clock-ceiling
   `K`, reserve activation numbers — plug into already-built machinery. Not a build; a ratification.
2. **🔌 Deploy pole (the real engineering, me to lead):** flip the **5 deploy-coupled binding sentinels** (all
   `false`: `CONTROL_BINDING_ACTIVE`, `COORDS_BOUND`, `REGISTRY_BINDING_ACTIVE`, `PROVENANCE_BOUND`,
   `CONTROL_ENFORCED`) so on-VM scripts source roots/coords/identity from real cells; on-VM single-use/nullifier
   crypto; the M2 genesis chain-spec that issues JUL from block 0; **L7 genesis / chain-spec / P2P + hosted seed
   node = the long pole**; and a **Linux/WSL2 box for the zkVM recursion receipt** (this machine has none).
3. **🔬 Research, undated (blocks the THESIS, never the FLOOR):** R1 learned-`v(S)` moat (data-blocked) · R2 HCE.

The two decision gates are all but CLOSED: **⚑-D RATIFIED 2026-07-11** (`DESIGN-vesting-W` §4) and **⚑-G
finality-genesis SETTLED** (bonded PoS carries block zero, §2.5; open sliver = money-in-genesis-set, rec:
moneyless+decaying). **So the schedule is no longer bounded by build effort or pending decisions — it is bounded
by the deploy/infra pole (a Linux box + genesis/P2P engineering) and one short economics sitting.**

Also decision-unblocked but NOT launch-gating (UX fast-follows / hardening): sub-block network half (gossip/DA
live propagation), equivocation-slashing wiring, apply-from-root rewire, single-sourcing the duplicated
`txs_conserve_and_single_use` / `commit_cell` helpers, Council pass on the sub-block tier, + the L4
doc-coherence tick.

## The one sentence to tell people (honest, no round-up)

> *"The reference node is built and green, and the money layer's mechanism is done — issuance, settlement,
> reserve, proof-of-work with real difficulty, difficulty retarget, and the chain's clock all built and tested.
> The consensus spine (a strategyproof contribution ledger with capital-orthogonal PoS+PoM finality) is
> complete, and every on-VM enforcement script is written. What stands between here and a public testnet is no
> longer building: it's the deploy pole — turning those enforcement scripts on against a real chain, wiring
> genesis and peer-to-peer into an actual network, a Linux box for one zero-knowledge receipt, and a short
> sitting to set the economic numbers. The un-gameability **moat** is honest open research we won't date, and
> our launch never claims it."*

---

## Loop 0 — reconcile (do this before quoting any count)

The MVP-SCOPE is 2026-07-03; HEAD is 2026-07-13. Known deltas already shipped since: JUL inc-1/2/3,
v(S) ValueOracle seam, equivocation live-path, T1 slices 1-5, the Pragma amendment socket,
`DESIGN-vesting-W-and-standing-bridge.md`, honesty pre-commit gate. **Before firming loop-counts, re-verify
each §1 item's file:line pin against HEAD** and re-mark ✅/🟡/🔬 — some Week-0 hygiene items and possibly
part of §1.A/§1.B may already be done. L0 is itself a DI loop (grep/read/test, no ⚑). Output: this ledger,
refreshed, with dead rows struck and live rows re-pinned.

> Noesis is PUBLIC (2026-06-29). This plan is a build-in-the-open roadmap; keep front-run-sensitive
> framing out, carry STATUS-LEDGER status-words verbatim, never claim the moat.
