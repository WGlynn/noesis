# Noesis and Pragma Coherence: the same primitive at two layers

The line from the consensus discussion that prompted this was: everything that is true overlaps with OPH. I want to make the overlap precise enough to be useful, not flattering. The test I am holding myself to is simple: if a sentence about Noesis stays true when you swap in any other chain's name, I have said nothing. So this is only the parts that do not survive that swap.

Noesis is public now, so its mechanism is fair to describe. I mark thesis versus built where it matters.

## The primitive, and the one axis that is not shared

Pragma certifies an agent's actions against a declared manifest from the outside. A Coherence License, an external verifier, an H1 obstruction over the action graph. The question it answers is: did the agent stay inside its declared intent. Proof of intended execution, externally attested.

Noesis enforces state transitions against declared rules from the inside of consensus. The gate refuses an incoherent transition, so an action is in bounds before it ever finalizes rather than certified as in-bounds afterward. Proof of intended execution, native. Same primitive, one layer in.

That much is a clean duality and it already fails the swap test for a generic chain, because a generic chain validates signatures and balances, not coherence against a declared intent manifest. But here is the axis Pragma does not have and no settlement chain has: Noesis prices which mind earns the credit. Coherence is the floor, the did-you-stay-in-bounds check. Proof of Mind is the layer on top that asks what was contributed and routes value to the contributor by provenance. Swap Ethereum into "prices which mind the value is owed to by tracing the contribution graph" and it is false. That sentence is the point of Noesis, and it is the half of the stack Pragma is not building.

## A receipt, so this is not vibes

An adversarial pass on the Noesis token layer found an action the gate would have wrongly accepted as coherent: spend 1000 while owning 6, because cell existence was bound to identity but not to amount. Caught, closed with a one-line fix, regression pinned. That is a coherence-RED on our own substrate, found by running intended-execution checks natively at the consensus gate. It is the exact failure class Pragma sells the external detection of, found internally before finalization. The two layers are not analogous by hand-waving; they catch the same bug from opposite sides.

## The math bridge, term for term

This is where the overlap stops being a slogan. The disagreement machinery lines up directly, and the pieces are standard enough (constraint codes, term rewriting, Lyapunov descent) that the correspondence is checkable, not asserted.

- A patch net is a graph where each node holds local state and each edge is an interface where neighbors compare. That is the Noesis replica graph, and one level down, the cell-provenance graph.
- An inconsistency potential sums weighted disagreement across every interface; the consistent states are exactly the zero set of that potential. That is the converged Noesis ledger: every replica's state digest agrees.
- A law is a local repair map that commits only if it strictly lowers the disagreement it touches. That is the Noesis apply step, the finalized deterministic version of the same monotone move; idempotent index insertion gives the same behavior.
- Asynchronous confluence to a unique normal form is Newman's lemma: local confluence plus termination yields a schedule-independent result. That is the Noesis convergence guarantee verbatim: two honest nodes applying the same finalized blocks reach an identical digest regardless of order.

One level up, selecting among laws by which ones let observers agree most efficiently is a replicator dynamic over the space of rules. That is mechanism selection, the same object as augmented mechanism design: a rule survives only if it makes coordination the surviving equilibrium. The observer-agreement requirement is the selector on both sides. That is the cleanest formal bridge between a physics-flavored consensus and a contribution-pricing chain, and it is why the convergence is not a coincidence.

One honest distinction worth keeping straight rather than blurring for effect: the OPH error model is redundancy-based, many observers redundantly recording the same fact so disagreement is correctable by code distance. The Noesis integrity model is cryptographic, a hash root that makes a record un-forgeable by collision resistance. Same goal, un-forgeable agreed state, different threat model. Naming the difference is the part that makes the rest credible.

## The composition and the endgame it serves

So the stack is not two overlapping products, it is two halves. Native coherence at the gate (Noesis) plus external certification any counterparty can verify in one call (Pragma) plus a value axis that prices contribution (Noesis PoM). The soulbound Coherence License and a soulbound PoM attestation are the same shape of object pointed at different questions, authorization versus credit.

The endgame this serves is concrete. A sovereign agent that runs as a Noesis-native subject has every state transition coherence-checked by the gate before it finalizes, carries an external Coherence License any third party verifies without trusting us, and earns PoM for what it produces. Pragma is the credential such an agent shows the world. That is a real reason the two systems want each other, not a partnership for its own sake.

## The open question that is worth a paper

Where does OPH's observer-overlap consistency formally meet Proof-of-Mind's contribution-overlap finalization? Both are fixed points of a disagreement-lowering dynamic over a partial order; if there is a single fixed-point theorem under which the observer case and the contribution case are instances, that is a joint paper.

The shared hard part under it is the ground-truth oracle. What grounds a coherence certificate, and what grounds Noesis's value function, is the same unsolved question in two costumes. Noesis's current answer is to source value from realized downstream flow rather than a declared proxy, and to treat the residual of that flow, the part no reordering of credit can explain away, as the certificate that is hard to game. Whether that residual maps onto how a coherence cert is grounded is the question I actually want answered, because if it does, the oracle problem has one solution and not two.

That is the overlap that survives the swap test. Same operator, two layers, and one axis, contribution pricing, that only one side is building.
