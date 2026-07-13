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
> **Stamp:** first written 2026-07-13 (HEAD `bf781fc`, lib suite 323 green). Loop-counts firm up after L0.

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
- **🔬 RESEARCH — undatable (MVP-SCOPE §2, STATUS-LEDGER).** Learned-`v(S)` moat (data-blocked: needs a
  deep-ancestry outcome-labelled dataset; the adversarial instrument is ✅ but real-outcome data is open)
  and HCE M2/C4 theorems (+ M3 `p`-supplier, M4 symmetric-lie elimination). Run as **background tracks**;
  never place them on the critical path or a schedule.
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

| Loop | Piece (SYSTEM-MAP layer) | Tier | Depends on | Status @ 2026-07-13 |
|---|---|---|---|---|
| **L0** | **Reconcile MVP-SCOPE §1 vs HEAD** (re-verify every file:line pin; the doc is 10 days stale) | DI | — | **do first** — prerequisite for firm counts |
| **⚑-D** | **Will decision packet** — W + PoM-finality input + bridge shape + θ_sim + MIN_DIM_BPS + I-2 | ⚑ | L0 | design inputs partly shipped (`DESIGN-vesting-W-and-standing-bridge.md`) |
| **L1** | Finalization ELF twin-update → `finalizes_pos_pom_fixed` (5) — kills the wrong-rule-at-deploy landmine | COLD | L0 (decision-unblocked; starts before ⚑-D) | 🟡, in-progress-dirty in tree |
| **L2** | Circularity fix / **vesting window `W`** (7,3) — fresh standing can't vote finality inside the window | COLD | ⚑-D | 🟡 designed |
| **L3** | `Standing.pom`→`Validator.pom` **production bridge** (7→3) — the finality PoM input becomes real | COLD | L2 | 🟡 designed |
| **L4** | Invariant pins + doc-coherence (E) — slash-burns-never-transfers test, zero-fee pin, β-bounty fence, re-stamp drift | DI/WARM | L0 | 🟡 cheap, load-bearing for "honest" |
| **L5** | Bound B commit-deposit (liveness) — refund-on-contribution / forfeit-BURNS-only, 12-item rubric before merge | COLD | L4 | 🟡 designed, rubric-gated |
| **L6** | On-VM enforcement parity (4,5,7) — validator-registry binding, lock-sig GO-LIVE flip, on-VM soulbound+similarity syscalls, double-spend crypto | DEPLOY | L1,L3 | 🟡/🔌 |
| **⚑-G** | **Genesis bootstrap decision** — PoW-scaffold vs founding bonded-set | ⚑ | — | open founding call |
| **L7** | Genesis / chain-spec / P2P + T1 slice-5 live + hosted seed node (6) — **the long pole** | DEPLOY | L6, ⚑-G | 🔬/🔌 (2-node convergence ✅, not a network) |
| **R1** | Learned-`v(S)` moat on a deep-ancestry outcome-labelled dataset (8) | RESEARCH | data hunt | 🔬 NULL twice; **undated** |
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

Built so far: inc-1 ✅ (`e51e164`) issuance core · inc-2 ✅ (`56d506f`) coinbase settlement · inc-3 ✅
(`bf781fc`) counter-cyclical reserve (shadow).

| Loop | Piece | Tier | Depends on | Status |
|---|---|---|---|---|
| **M1** | JUL inc-3b — reserve consensus wiring: skim/top-up at the coinbase-mint site + protocol-spend-only reserve cell | COLD | signal in block stream; `CONTROL_BINDING_ACTIVE` flip (runtime.rs:411) | 🟡 designed (inc-3 seams) |
| **M2** | PoW + genesis issuance — `Block::difficulty` + real `block_work` + issue JUL from block 0 on the PoW path, JUL out of `FINALITY_MIX` (inc-4 genesis wiring) | COLD | ⚑-G (PoW starts genesis) | 🟡 designed |
| **M3** | JUL economics live — governable `Constitution` JUL params + Lever-A difficulty-retarget (Ergon fidelity) + reserve activation numbers + the miner-reflexivity game-theory pass (the inc-3 Council gate) | COLD/⚑ | M1, M2; ⚑ numbers | 🟡 designed / ⚑ |

> Lever A (the production-cost anchor) is economically INERT until M2 makes `block_work` return real mined
> difficulty (jul.rs:6-9) — so "JUL live as money" genuinely requires the PoW layer, which is why M2 is
> coupled to the genesis bootstrap decision ⚑-G.

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

**Honest count to a floor public testnet:** **L0 + ⚑-D + consensus spine { L1, L2, L3, L4, L5, L6 } +
money track { M1, M2, M3 } + ⚑-G + L7** — i.e. **~10 build loops across two parallel tracks behind two
Will-decision gates (⚑-D, ⚑-G), with L7 (genesis/P2P) the long engineering pole.** Each build loop is a
session; the pacing constraint is Will's decision/review cadence on the COLD loops, not build throughput.

## The one sentence to tell people (honest, no round-up)

> *"The reference node is built and green, and JUL — the e-cash — is three-quarters built (issuance,
> settlement, reserve done; genesis-issuance and economics still to wire). The launchable **floor**
> (contribution ledger + capital-orthogonal PoS+PoM finality + JUL money live) is about ten cold-build
> loops across two parallel tracks, behind two decision sittings and a genesis/P2P engineering pole —
> automatable in sessions, paced by review, not effort. The un-gameability **moat** is honest open research
> we won't date, and our launch never claims it."*

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
