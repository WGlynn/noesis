# 2026-07-18 — 15-loop TRP/RSI on the node repo: 10 real bugs fixed, 2 correctly refused

Plain-English recap of an autonomous adversarial-audit run you asked for ("TRP RSI 15 loops of the
code repo"). It ran ~3.4 hours in the background: 70 sub-agents, 15 sequential loops, each one
auditing a different part of the node, verifying findings against the real code, and only fixing what
survived. Every fix had to keep the test suite green or it was reverted.

## The headline
Started at 548 passing tests, ended at **556 passing / 0 failing** (I re-ran it myself at the final
commit, not trusting the workflow's word). **10 genuine bugs fixed, committed, and pushed** to public
`master` (HEAD `bfdf796`). Two more were found and deliberately NOT fixed because the "fix" would have
been worse than the bug — the honest-defer rail worked.

## The 10 fixes (all cargo-green, each with a regression test)
Two rated critical, eight high. In plain terms:
- **Bond single-use** (runtime.rs): the "you can't spend the same bond twice" check ignored a cell's
  data field, so two genuinely-different cells looked identical and a valid block got rejected. Now
  uses the full cell identity, matching the authoritative ledger check.
- **Refuted-set slash ordering** (lib.rs): slashes were emitted in input order, so two honest nodes
  could hash the same settlement differently — a consensus split. Now sorted canonically.
- **Anti-plutocracy axiom** (amendment.rs): the "capital may never outweigh contribution" guard on
  governance mix-amendments allowed capital to exceed contribution by a hair (a 1e-9 tolerance). Now
  strict. This guards *amendment proposals*, not per-block validation, so it can't reject real blocks.
- **Coinbase id collision** (jul.rs): at astronomically high block heights (≥ 2^62, unreachable for
  ages) a coinbase id could collide with a split-slice id. Masked off. No effect on any real chain.
- **Sub-block ordering** (subblock.rs) — critical: the provisional "what's confirmed so far" view
  folded sub-blocks in receipt order while the real absorb step sorts by sequence. A test proved this
  let a double-spend through when sub-blocks arrived shuffled. Now both sort identically.
- **Four resource/DoS bounds** (wire.rs, rpc.rs, screen.rs): an attacker-supplied block log, HTTP
  header flood, or a single huge submission could exhaust memory. All now bounded. Node-local
  liveness only, no consensus change.
- **Duplicate-key and overflow guards** (utxo_commitment.rs, tokens.rs): a cell appearing as both
  spent and created in one block now fails loud instead of producing a contradictory proof; the
  supply-total sum now saturates instead of silently wrapping (which could have forged supply).

## The 2 it refused to fix (this is the system working, needs your call)
- **Loop 2 — false alarm, correctly rejected.** An agent claimed a similarity-floor rounding bug
  "breaks consensus determinism." A second agent read the actual code and refuted it: the value is a
  governance parameter (not a computed rounding), the "diverging" number appears nowhere, and the only
  divergence is the deliberately-documented fixed-point-vs-f64-prototype gap (every node disagrees with
  the prototype identically = still deterministic). No change made. Good catch — a worse system would
  have "fixed" correct code.
- **Loop 10 — real bug, but the obvious fix weakens safety. Your design call.** If you ever flip
  `clock_enforced` from false→true on an existing chain, replaying the old (timestamp-less) blocks
  fails. The tempting one-liner (accept missing timestamps whenever enforcement is on) would
  permanently disable the "every block must carry a timestamp" rule for the whole chain — trading a
  live-safety guarantee for a migration edge case. The agent declined and recommended the right shape:
  either forbid activating the flag on a chain that started without it (fail-loud at genesis, like the
  existing pow_enforced assert), or add an explicit activation-height. Both are schema changes worth a
  human review.

## Backlog it surfaced but did not fix (one fix per loop by design)
~11 additional real-looking findings, mostly >4GB-input truncation edges (unreachable in practice) and
overflow-hardening. **The one worth a look:** a *negative-alpha* path in the refutation slash math
(`resolve_refuted` / `resolve_refuted_guarded`) that could produce negative slash amounts and a
settlement imbalance. It was found but not the top-severity item in its loop, so it is unfixed. A
follow-up loop (or a targeted fix) should close it.

## Honest scope
- All 10 fixes are minimal, test-gated, and reversible (`git revert <hash>`). Nothing swept the
  do-not-sweep set. Identity is your own (Will Glynn), no AI trailer.
- I did NOT hand-audit all 10 diffs line-by-line beyond the anti-plutocracy one; I verified the whole
  suite is green and spot-checked the consensus-touching changes. The three I'd want your eyes on are
  the anti-plutocracy tightening (amendment.rs), the bond single-use widening (runtime.rs), and the new
  duplicate-key assert (utxo_commitment.rs) — all consensus-adjacent, all defensible, all reversible.

Commits: `ed31d38` `697c999` `b715870` `db1924d` `1e67893` `699a1e6` `3d64c50` `6f35643` `9d9cc5e`
`bfdf796` (TRP loops 1,3,4,5,6,9,11,12,14,15). HEAD == origin/master.
