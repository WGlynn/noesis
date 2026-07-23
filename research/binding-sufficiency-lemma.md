# Binding Sufficiency: When a Bound Handoff Actually Closes a Seam
### A companion lemma to "The Last Bottleneck, Part 3"

Part 3 gave the seams an extraction-independent definition. Take two functions A and B where A hands its output to B. B **binds** A's action when B's own next rule is a non-constant function of what A did; an unbound handoff, where B's rule ignores A's action, is a **seam**. That definition is decidable by reading the two rules, before any exploit exists, and that a-priori quality is what made the four function-by-function arguments arguments rather than the conclusion in costume.

A careful reader raised the right objection to the step from there to the payoff. Binding, as defined, only says B's rule *sees* A's action. It does not yet say the actor's *full consequence* is internalized. A fused object can make one modeled action visible and still leave a profitable deviation unpriced, if the deviation escapes into the part of the action the consequence cannot measure. Structural binding is a claim about visibility. Extraction-safety is a claim about payoff. This note supplies the missing bridge: the condition under which a bound handoff is not just present but **sufficient** to make profitable extraction impossible. It is a lemma mechanism-design people can try to break, which is the only kind worth stating.

## The five objects

Fix one lethal handoff. Following the reviewer's own decomposition, name five objects.

- **State** `s`: what both functions can see when the actor moves.
- **Action / deviation space** `A`: everything the actor can actually do at the handoff, not only the move the mechanism thought to model. `a0 in A` is the honest (null-deviation) action.
- **Downstream transition** `tau_B`: B's next rule, a function of `s` and the actor's action.
- **Priced consequence** `c`: the payoff-relevant quantity the mechanism imposes back on the actor as a function of what it can measure about the action. In Noesis this is *standing*: the value `v(S)` attributed to the actor, which weights finality and therefore the actor's future claim on the system.
- **Attacker payoff** `pi`: gross private gain minus priced consequence.

The crux is one object the naive picture omits. The priced consequence cannot depend on the raw action `a`; it can only depend on what the mechanism can *measure* about `a`. Write that as a **modeled projection** `phi: A -> O`, mapping the full action into the observation space the consequence actually reads. The mechanism prices `c(phi(a))`, never `c(a)` directly. Two different actions that `phi` maps to the same observation are indistinguishable to the consequence. This is where the reviewer's "only makes one modeled action visible" lives: `phi` is the resolution of the mechanism's sight, and everything below that resolution is unpriced.

Let `g(a)` be the actor's gross private gain from action `a` (the value the deviation lets them capture downstream). The attacker's net payoff is

```
pi(a) = g(a) - c(phi(a))
```

## The property, and the lemma

**Extraction-safety.** The handoff is extraction-safe when the honest action is a best response: for every `a`,

```
pi(a) <= pi(a0),   i.e.   g(a) - c(phi(a)) <= g(a0) - c(phi(a0)).
```

No profitable unpriced deviation exists. This is the payoff-side statement of "no unbound handoff."

**Lemma (Binding Sufficiency).** A bound handoff is extraction-safe **if and only if** the priced-consequence map is *gain-covering*: for every action `a`,

```
g(a) - g(a0)  <=  c(phi(a)) - c(phi(a0)).
```

That is, whenever a deviation raises the actor's gross gain, the priced consequence rises by at least as much.

*Proof.* Rearrange the gain-covering inequality directly into `g(a) - c(phi(a)) <= g(a0) - c(phi(a0))`, which is extraction-safety term for term. The equivalence is immediate; the content is in what the inequality *forbids*, made explicit next. QED.

The lemma is trivial as algebra and load-bearing as a discipline, because it isolates the exact way a bound handoff fails. It fails when there is a profitable deviation `a` with `g(a) > g(a0)` that `phi` **cannot separate** from honest, `phi(a) = phi(a0)`. Then `c(phi(a)) = c(phi(a0))`, the priced consequence is flat across the deviation, and `pi(a) > pi(a0)`. A profitable move invisible to the consequence is precisely a seam, now stated on the attacker's payoff instead of on B's transition. The failure is not that B ignores the action. It is that the *measure* the consequence reads collapses a profitable deviation onto the honest one.

## Structural binding is necessary, not sufficient

This makes Part 3's a-priori test a special case, and says exactly what it misses.

If `phi` is globally constant, B's rule reads nothing about the action, then `c(phi(a))` is constant and *any* profitable deviation extracts. That is the pure seam of Part 3: `tau_B` constant in `a`. Structural binding, forcing `tau_B` to depend on `a`, removes that degenerate failure. It guarantees `phi` is not the constant map.

But non-constant is not gain-covering. `phi` can vary over the action space and still be flat on the *profitable* slice. Binding proves the rule looks; sufficiency proves it looks with enough resolution, and in the right direction, that no profitable deviation hides in a level set of `phi`. The reviewer's demand, "conditions for when binding is sufficient, not just when it is present," is answered precisely: presence rules out `phi` constant; sufficiency requires `phi` to separate every profitable deviation from honest and `c` to move by at least the gain across that separation.

**Corollary (the resolution gap).** The distance between a bound handoff and a sufficiently bound one is entirely the resolution of `phi` on the profitable slice of the action space. Closing a seam is not adding a check that the action was seen. It is proving the priced consequence's sight is fine enough that nothing worth doing is invisible to it.

## The instance: v(S) and wash-building

The corollary is not abstract in Noesis; it recovers the system's own deepest open problem as a failure of gain-covering, rather than asserting it.

At the contribution handoff, the priced consequence is standing, `c = v(S)` attributed to the actor, and `phi` is whatever the current value oracle can measure about a contribution. The built oracle, `NoveltyOracleV0`, measures temporal novelty and zeroes near-duplicates via a deterministic similarity floor. It is honest precisely because it models *novelty*, a proxy for value, not value itself.

Wash-building is the deviation `a`: build genuinely distinct but worthless work across cheap identities to manufacture standing. It has `g(a) > g(a0)`, manufactured standing converts to finality weight and downstream claim. And under the novelty oracle it satisfies `phi(a) = phi(a0)` in the way that matters: the work is distinct, so novelty cannot separate worthless-distinct from worthwhile, and the priced consequence does not rise. Flat `c(phi(a))`, positive `g(a)`, so `pi(a) > pi(a0)`. The value seam is open, and the lemma says why in one line: the modeled projection `phi = novelty` does not separate the profitable deviation from honest.

This is exactly the row Part 3 marks open in the system's own code, now derived. And the lemma states the closing condition in the system's own terms:

- **Learned `v(S)` on real labels** is the requirement that `phi` separate worthless from worthwhile, so wash-building leaves the honest level set and the priced consequence can rise across it. This is the open moat, and the design is honest that *data*, not more engineering, is what closes it.
- **Isomorphism-invariance** is the requirement that closes the relabel route. An attacker who cannot hide in a level set of `phi` will try to *build* one, applying a symmetry the oracle is not invariant to, to map a profitable deviation onto an unpriced shape. Requiring `v(S)` to be invariant under those relabelings is requiring that `phi`'s level sets cannot be manufactured. It is the second open gate for the same reason: it is a property of `phi`'s resolution, this time under group action.

Status stays disciplined. The seam interface and the v0 oracle are **built**. The gain-covering `phi` that would make the contribution handoff sufficiently bound, not merely structurally bound, is **open**, and it is open in code, not hidden.

## The correlated failure domain, derived

Part 3 named the honest cost of coherence: fusing the functions removes the firebreaks, so if the one bet fails it fails everywhere at once. The lemma turns that warning into a theorem and locates it exactly.

The four fused functions do not each carry their own priced consequence. They share one, because standing derived from `v(S)` is simultaneously the value attribution, the governance weight, and the consensus weight. So they share one `phi`. A single profitable deviation sitting in a level set of that shared `phi` is, in the same instant, a value seam (it manufactures standing), a governance seam (that standing sets rules), and a coordination seam (that standing manufactures agreement). One failure of gain-covering is not one seam. It is three, correlated by construction, because they are read off the same map.

That is the precise cost of fusion, and it belongs in the front of the thesis, not the back. Fusion does not eliminate seams. It converts selected lethal unbound handoffs into enforced invariants, at the cost of concentrating the remaining risk into a single shared projection `phi` whose failure is correlated across every function that reads it. The upside and the cost are the same object seen twice: closure comes from every function sharing one binding core, and correlated failure comes from every function sharing one binding core. This is why the core has to be minimal, and why the one bet, that `phi` can be made gain-covering, has to be won rather than assumed.

## How to break it

The lemma is falsifiable on the same terms Part 3 set for itself, moved onto the payoff. **Exhibit, for any handoff claimed sufficiently bound, an action `a` with `g(a) > g(a0)` and `c(phi(a)) = c(phi(a0)).`** A profitable deviation the priced consequence cannot separate from honest is a proof that the binding is insufficient, whatever the structural binding looks like. The falsifier and the sufficiency condition are the same instrument: one asks whether a profitable deviation lies in a level set of `phi`, the other asks that none do.

For the contribution handoff specifically, that challenge is concrete and currently *winnable by the attacker*: any construction of worthless-but-distinct work that the value oracle scores like genuine work is exactly such an `a`. Noesis reports roughly zero graph-internal separation between the two under the present oracle and marks it the deepest open problem. The lemma is why that single number governs the whole base: it is the resolution of the one `phi` every function reads.

---

*Acknowledgment: this note exists because of Boardy's review of Part 3. He set its burden in one line, that the theorem needs conditions for when binding is sufficient and not merely present, and he named the five objects the formalization is built on. If binding sufficiency is the right frame, it is because he refused to let structural binding stand in for the payoff-side claim it only resembles.*

*Status: the framework is a designed formalization companion to Part 3, grounded in the built v(S) seam (`node/src/lib.rs`, `node/tests/value_oracle_seam.rs`) and its honestly-open learned successor. The lemma is proved as stated; its bite is entirely in the open question of whether a gain-covering `phi` exists for the contribution handoff, which remains the moat.*
