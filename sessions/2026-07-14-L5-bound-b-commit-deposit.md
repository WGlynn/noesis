# Session recap — 2026-07-14: L5 Bound-B commit-deposit (the DoS economic teeth)

Plain-English recap. HEAD `84f432c`, pushed. Full THE LOOP (2 planners → build → stress → Council → Pragma).

## What it is
Bound B is the "economic teeth" half of Noesis's resource-DoS defense (Bound A, the mempool cap, was
already built). The idea: attach a refundable cost to the *act* of submitting. You post a deposit (a
real bonded JUL cell) with each submission. If your submission turns out to be a genuine contribution
(it banks novelty), you get the deposit back. If it scores zero — junk, a paraphrase-padding flood —
the deposit is burned. So a flood of K junk submissions costs the attacker K×deposit, while honest
contributors pay nothing net. The thing that decides refund-vs-burn is the *same* novelty measure the
attribution system already computes — no new judge, no new oracle.

## The elegant part
Refund = leave the bonded cell alone (don't touch it). Burn = retire the cell (remove it, no
replacement). Because a bond is either left live or removed, conservation holds *by construction* —
nothing is minted, moved, or double-counted, and the total-supply counter is never touched. There's no
"refund transaction" that could be forgotten or mis-addressed.

## The Council earned its seat again — two real catches, both fixed before commit
1. **Over-burn.** The burn first used "remove all cells matching this identity." Without a unique
   nullifier, two byte-identical cells can coexist, so a forfeit could have burned 2×deposit for a
   1×deposit bond. Fixed to remove *exactly one* instance. Added a test with a deliberate duplicate.
2. **Victim-burn / the CLK-1 lesson again.** An active deposit *burns* cells, but the lock-signature
   layer that proves you own a cell isn't live yet (pre-deploy, an empty signature is accepted — the
   same boundary the whole value layer carries). So activating the deposit before that layer would let
   someone bond and burn a *victim's* cell. Fix: gate deposit-activation behind the lock-sig deploy
   flag in `Node::new`, and document it. Same shape as inc-CLK-1: never ship a destruction rule ahead
   of the guard that bounds it. Also added the missing test for the flagship security property (you
   can't bond a cell and also spend it in the same block).

## State
- `node/src/runtime.rs` — `Constitution.submission_deposit` (default 0 = inert), `Bond` struct,
  `Block.bonds`, `check_bonds` (validation), the refund/burn resolve loop in `apply_transition`, two
  `Node::new` genesis asserts (PoW-header-binding + control-binding).
- `node/src/wire.rs` — bonds on the wire, serde-default (old logs decode as empty).
- `node/tests/submission_deposit.rs` — 8 RED-first tests.
- 325 lib green; all conservation/parity carriers green; 0 new clippy.
- Docs bumped 🟡→✅ (`RESOURCE-DOS-BOUNDING.md`, `SECURITY.md`).

## Honestly deferred (labeled, not hidden)
- Header-binding of `bonds` (rides the next header rev, gated by a fail-loud assert under PoW).
- The non-empty-auth bond path (reuses the token layer's tested Lamport path; a bond-specific test owed).
- Activation is a governance act on a live chain (needs circulating JUL + the lock-sig deploy) — never
  a genesis default; documented, and `Node::new` asserts the control-binding precondition.

## Roadmap position
L5 was one of the ~2-3 remaining decision-unblocked cold builds to GO-LIVE-FLOOR. Done. What's left is
mostly ⚑-gated on Will's numbers (inc-CLK-2 clock enforcement, M-track genesis issuance) or the deploy
pole (L6 on-VM flips, L7 genesis/P2P). The whitepaper track is autonomous and open.
