# The transparency<->privacy factoring in Noesis

Status: design note, 2026-07-12. States the structural claim that the transparency/privacy tradeoff
is **factored** in Noesis rather than balanced, and pins each half to a concrete artifact. Grounded
to `file:line`; verify before claiming. Nothing here changes protocol behaviour — it names a
property the existing pieces already have and points at the two PoCs that demonstrate it.

## The claim in one line

> **Transparency belongs to the rule; privacy belongs to the inputs; ZK proves the transparent rule
> ran on the private inputs.** There is no global transparency/privacy dial — each fact is placed on
> the axis independently, and the placement is verifiable either way.

The usual framing treats transparency and privacy as opposite ends of one slider: reveal more to be
auditable, reveal less to be private. Noesis does not sit on that slider. It splits the single axis
into two orthogonal ones and puts each concern where it has no cost.

## Two orthogonal axes

### Axis A — rule transparency (no tradeoff; pinned at "fully public")

This axis decomposes into three separable guarantees:

1. **Validity** — did this block follow the rulebook. (zk proofs / light clients / fraud proofs.)
2. **Veracity of the frozen rulebook** — is the rule itself sound (conserves value, forbids double
   spends, deterministic) over *every* execution. In Noesis this is a machine-checked Isabelle proof
   (`internal/fv/Noesis_Rulebook.thy` — `conservation` + `no_double_spend` + `determinism`, 0 sorry).
3. **Veracity under change** — does a governance amendment stay confluent AND preserve the axioms.
   This is the amendment-court surface: Noesis's own constitutional-cell + verifier-gate governance
   (`node/src/runtime.rs`), which makes each amendment an explicit, inspectable rule-set mutation
   with its axiom obligations stated. The amendment rules are currently a `pending` rule-set.

Every guarantee on Axis A *wants* maximum transparency. A private rulebook is just an unaudited
rulebook; a private amendment is an unchecked amendment. So this axis is pinned at "fully public,"
and more transparency here is strictly better — there is nothing to trade away.

### Axis B — input privacy (the real tradeoff; ZK resolves it)

This is the axis the ZK spec lays out as the four fits (`docs/ZK-SPEC.md`,
`docs/ZK-INTEGRATION.md`). It is about *what the rule runs on*, not the rule:

| Fit | Private input | Public | Axis-B position |
|---|---|---|---|
| 1 — finality verdict | none | verdict | fully transparent (succinctness only) |
| 2 — contribution scoring | content + proofs | `scored >= V`, floor-pass | private inputs |
| 3 — novelty check | content + corpus | overlap `< theta` vs committed root | private inputs |
| 4 — provenance / account-link | handle<->key link + work | binding holds | private inputs |

Here transparency *does* have a cost: fully public inputs foreclose proprietary/pre-publication
work, pseudonymous standing, and private dispute. This is where the genuine tradeoff lives — and ZK
dissolves it by proving a fact about hidden data instead of forcing "reveal or don't."

## The seam: Fit 1 is the Axis-A validity mechanism AND the Axis-B bridge

The two axes are not merely adjacent; they meet at one mechanism. Fit 1
(`onchain/zk-finalize/`) proves `finalizes_pos_pom_fixed` — *the exact code the node runs*, because
`noesis-core` is `no_std` + `riscv64imac`, so a RISC-V zkVM proves the canonical rule with no circuit
re-derivation. That property does double duty:

- On **Axis A** it is the succinctness the *validity* guarantee needs for outside verifiers (a
  proof, not an attestation).
- On **Axis B** it is what makes hiding the inputs *safe*: because the proof attests the
  **canonical, court-approved rule** was applied, a verifier can trust a private-input verdict (Fit
  2) without trusting the prover or seeing the data.

So the factoring composes into a single sentence:

> Axis A keeps the **rule** transparent and provably sound-under-amendment. Axis B (ZK) keeps the
> rule **correctly applied even when the data it ran on is hidden**. Public rule, private inputs,
> verifiable either way.

## The two artifacts that demonstrate it

- **`onchain/zk-finalize/`** — Fit 1. Public inputs, public rule, succinct public verdict. *Axis-A
  transparency pole.* Parity harness GREEN; risc0 guest/host written; **no receipt yet** (needs
  Linux/WSL2).
- **`onchain/zk-score/`** — Fit 2. Content + proofs as **private witness**, the *same* canonical rule
  (`zk_score_eval` over `proven_floored_novelty_q16`), journal reveals only
  `(public_digest, nullifier, accepted, value>=V_FLOOR)`. *Axis-B privacy pole.* Parity harness GREEN
  incl. forgery-caught fixtures; risc0 guest/host written; **no receipt yet**.

**The privacy pole carries a verifier obligation** (do not skip it when reasoning about Axis B). A
Fit-2 receipt proves "some hidden content scored `>= V_FLOOR` against root R" — it does **not** prove R
is the real corpus. `root` is a prover-chosen public input, so the consumer MUST reject any receipt
whose `public_digest != zk_score_public_digest(canonical_root, policy_thetas)` and MUST verify against
the image id (which bakes `V_FLOOR`, so the bar is not prover-chosen). The bound `V_FLOOR` is baked, not
supplied, so the exact score cannot be binary-searched across receipts; a `nullifier` binds the content
so one contribution earns standing once. In factoring terms: hiding the *input* is safe only because the
*rule and its public parameters* are pinned by the verifier — transparency of the rule is what licenses
privacy of the input.

The pair is the minimal demonstration of the factoring: identical rule, opposite input-visibility,
both verdicts publicly checkable.

## Why this doesn't contradict the amendment-corpus argument

The amendment court needs a **public, labelled corpus of rule-mutation events** (amendments tagged by
which axiom they touch). That is an *Axis-A* asset and stays fully transparent even when *contribution
content* (an Axis-B input) goes private under Fit 2. Amendments public, contributions optionally dark
— no conflict, because they live on different axes.

## Honest status (do not round up)

- Axis A is materially more mature than Axis B: the veracity proof is machine-checked (Isabelle,
  0 sorry); the amendment-court surface itself is still a `pending` rule-set (constitutional-cell +
  verifier-gate) awaiting its checking engine.
- Axis B is entirely 🟡 designed / 🔬 unbuilt at the receipt level. Fit 1 & Fit 2 have GREEN parity
  harnesses but **no STARK receipt** (no prover on the dev box). Fits 3 & 4 additionally depend on
  unbuilt Merkle-corpus plumbing (Fit 4 waits on Fit 3).
- Therefore this note grounds the factoring as **design**, demonstrated by two host-verified PoCs —
  not as a shipped end-to-end private-scoring system. The claim is "the architecture factors the
  tradeoff and here are the two artifacts that show the seam," not "private scoring ships today."

## References

- `docs/ZK-SPEC.md`, `docs/ZK-INTEGRATION.md`, `docs/phase3-zk-plan.md` — the four fits + build order.
- `onchain/noesis-core/src/lib.rs:215` (`proven_floored_novelty_q16`), `:481`
  (`finalizes_pos_pom_fixed`) — the two canonical rules the PoCs prove.
- `internal/fv/Noesis_Rulebook.thy` — the machine-checked veracity proof (Axis A, layer 2).
- `node/src/runtime.rs` — the Constitution / amendment-court governance surface (Axis A, layer 3;
  amendment rules currently `pending`).
