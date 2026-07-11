# PIN — quorum-floor Q on the live finality path (RESOLVED 2026-07-11)

**Status: RESOLVED — Will ratified the recommendation ("go with yours"; frame = thousand-year
play, kernel-first). WIRED: `finalizes_pos_pom` accepts `quorum_floor_bps`; `finalizes` passes
`c.quorum_floor_bps`; stale T3 comment corrected; all call sites pass 0; on-VM drift-guard passes 0
to stay faithful. Default stays 0 ⇒ NON-BREAKING (lib suite 277→278 green, integration all green,
0 new clippy). New RED→GREEN test `runtime::tests::quorum_floor_prevents_minority_finalization_safe_halt`:
Q=0 lets a lone live validator (50% of each dim, other half decayed) finalize a minority checkpoint;
a high Q anchors the denominator to the full registered base ⇒ safe halt; full participation still
finalizes under the floor. Dynamic-Q decision: NO self-regulation on the safety path (a floor that
reacts to participation is reflexively gameable) — keep it a governed constant like MIN_DIM_BPS;
defensible future refinement = slow self-SCALING to Sybil-discounted soulbound-PoM network size (v2).
NOT committed/pushed — Will's call.**

---
_Original held finding below (kept as record)._

## Verified finding (in code, not from memory)
`runtime::finality::finalizes_pos_pom` hardcodes the `finalizes_hybrid` quorum-floor arg to `0`, and
does NOT accept a quorum param. So `Constitution.quorum_floor_bps` (field exists, default 0) is a
**dead wire on the finality path**. The ratified deactivation→safe-halt composition
(DESIGN-block-logistics-mechanism.md §123: withhold → stall ≤ grace → deactivated → resume on live
set iff ≥ floor, else safe halt) is therefore currently UNENFORCEABLE. Semantics of Q in
`finalizes_hybrid` (lib.rs:3844): `floor = base_total·Q/BPS; basis = max(eff_total, floor);
finalize ⇔ weight_for ≥ basis·⅔` — a nonzero Q stops a thinned live set from finalizing a minority.

## Tension (why this is a Will-decision, not an auto-fix)
Old T3 doc-comment (runtime.rs:621, 2026-06-20) deliberately scoped `quorum_floor_bps` to the
PRODUCTION/ordering path, NOT the fast-final gate. New block-logistics design (2026-07-10 §123) needs
it ON the finality path. Live contradiction in the repo's own docs, on the consensus safety path.

## Recommendation (Jarvis, on the merits — elegant-kernel dead-wire fix)
**Wire `c.quorum_floor_bps` through `finalizes_pos_pom`, keep constitutional default 0.**
NON-BREAKING (Q=0 ≡ today's rule exactly). Makes the ratified safety property enforceable on opt-in;
adds no magic constant. Alternatives: (2) nonzero default now = unearned constant / machine-drift;
(3) leave dead, enforce §123 elsewhere.

## Exact edit map (ready to apply on ratify)
- `finalizes_pos_pom` (runtime.rs:666): add `quorum_floor_bps: u64` param; pass it instead of `0`.
- `finalizes` wrapper (runtime.rs:626): pass `c.quorum_floor_bps`.
- Update stale doc-comment (runtime.rs:620-624).
- Call sites to append `, 0`: runtime.rs 1545, 1677, 1681, 1692-block, 1701; lib.rs 8710 (float_pp),
  8736, 8741.
- On-VM mirror `finalizes_pos_pom_fixed` (lib.rs): Q-port is a FOLLOW-UP; drift-guard stays valid
  while production Q=0 (pass Q=0 in the drift comparison at lib.rs:8710).
- Add RED→GREEN test: nonzero Q, thinned live set clears ⅔-of-active but < Q ⇒ rejected (safe halt);
  control Q=0 ⇒ same set finalizes.
