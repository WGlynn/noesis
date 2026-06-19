# DESIGN — lock-sig binding (existence → control)

> pom-roadmap-advance design tick 2026-06-19 (v). No code (PCP-gate: lock-sig touches the
> spend-validation path = high blast radius ⇒ Rust in fresh low-context). Advances the #1
> named frontier from *named/deploy-coupled* → **DECIDED + reference-scaffold contract**.
> Lean-like-Bitcoin: one field + one inert call-site, no new crate, no sig type until deploy.

## The gap (grounded)
`is_valid_in_ledger` (runtime.rs) proves a consumed input EXISTS as a finalized cell (identity
`id + lock + type_script`), and (o) bound `data` so the amount can't be forged. It does NOT prove
the spender CONTROLS the cell. `lock.args` carries the current owner pubkey hash and the program
is documented to "authorize the spend" (lib.rs:41-42), but pre-deploy nothing checks that the
party presenting the input actually holds the owner key. Anyone who can name a real cell's identity
can spend it. **EXISTENCE ≠ CONTROL.**

## Why control has NO meaningful pure-reference model (the honest core)
Existence was reference-checkable because a finalized cell cannot be fabricated from public state —
the check reads only consensus-known state. **Control is different in kind:** it requires proof of a
SECRET (the owner's signing key) that lives outside the finalized state. A pure-reference predicate
`spender == owner` is vacuous — the producer just sets `spender = owner`. There is no consensus-only
value to re-derive, because the discriminating input (a signature over the tx) is irreducibly
cryptographic. This is precisely why prior ticks ((g)/(h)/(o)) correctly punted control to
"deploy-coupled" — not laziness, a category fact. Control is the FIRST frontier item that is
genuinely crypto-irreducible (unlike index-dep / header-`now`, which were consensus-sourced and so
modelable inert at reference layer).

## DECISION — scaffold the STRUCTURE now, gate the VERIFICATION inert until deploy
Mirror the index-dep / header-`now` pattern (sentinel-gated-inert): land the SHAPE the crypto plugs
into, so deploy is a drop-in, not a refactor of the spend path.

1. **Authorization field on the spend.** Each token/cell input in a tx carries an `auth` (a byte
   blob: at deploy = a signature over the canonical tx digest by the input's owner key; pre-deploy =
   an empty/sentinel value). NOT a producer-set boolean — there is no `authorized: bool` channel
   ([P·dont-let-attacker-choose-critical-input]); the only input is the opaque `auth` the verifier
   interprets.
2. **One verification call-site, sentinel-gated-inert.** `is_valid_in_ledger` gains a
   `spend_is_authorized(input, auth, tx_digest)` call. Pre-deploy it is INERT (sentinel `auth` ⇒
   returns true, with an explicit `// CONTROL DEPLOY-COUPLED` marker) so all current honest tests
   stay green and no behavior changes. At deploy the body becomes `verify_sig(owner_pubkey =
   input.lock.args, msg = tx_digest, sig = auth)` — owner sourced from the FINALIZED cell's `lock`,
   never producer-asserted; tx_digest canonical/deterministic.
3. **Canonical tx digest now (deploy-independent, the one BUILDABLE grain).** Define the
   deterministic byte-serialization of a tx that the signature will cover (inputs identities +
   outputs + nonce, canonical order — reuse the (u) flatten/sort discipline). This is pure
   consensus-state serialization, has NO crypto dependency, and is needed by BOTH the eventual
   sig-verify AND replica determinism. It can be built + tested at the reference layer this frontier,
   ahead of the sig itself. ⇒ the lock-sig mile is no longer fully blocked; its deterministic-digest
   prerequisite is deploy-independent.
4. **Crypto suite choice deferred but constrained:** per [P·quantum-proficiency], the deploy sig is
   post-quantum capable (Lamport/hash-based as the floor; the lock program may offer a classical
   fast-path + PQ). Decide at deploy; the `auth` blob is suite-agnostic by being opaque bytes.

## BUILD CONTRACT (next, ordered)
- **Deploy-INDEPENDENT, buildable now (fresh low-context):** the canonical `tx_digest`
  serializer + determinism test (×N bit-identical; digest changes iff a value input changes) + the
  inert `spend_is_authorized` call-site (sentinel ⇒ true; one regression pinning that a NON-sentinel
  malformed `auth` is rejected even pre-deploy, so the gate isn't dead code). Honest-INERT: all v5–v8
  + token tests green (sentinel path).
- **Deploy-COUPLED (named, not built):** swap the inert body for `verify_sig` over `tx_digest`;
  regression — spend of alice's cell with bob's sig (or no sig) REJECTED, alice's own sig ACCEPTED,
  fabricated-owner REJECTED. This is the existence→control closure end-to-end.
- **Anti-theater (deploy):** flip to an always-true `verify_sig` ⇒ the control regression must go RED
  (the sig is load-bearing), same break-on-purpose discipline as the (u)/T3 ρ:=1.0 check
  ([[vibe-coding-confidence-loop]] keystone).

## Honest residual
This tick decides the shape + extracts the one deploy-independent grain (the digest). Control itself
stays 🔬 deploy-coupled (crypto-irreducible) — correctly, and now with a de-risked drop-in path
rather than an open hand-wave. NEXT after the digest grain: on-VM single-use per (k); then the
learned-v(S)-on-real-labels mile (THE moat).
