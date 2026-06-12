# PROVISIONAL PATENT APPLICATION — v2 (DRAFT — PRIVATE, attorney review required)

> Inventor: Will Glynn. Project name: Noesis. Jarvis-drafted spec + claim set for the
> inventor and patent counsel. NOT filed, NOT legal advice. File the provisional to lock the
> priority date before any third-party or external-LLM review. Private repo only.

## TITLE

Method and system for earned, non-transferable consensus standing on a contribution ledger, with strategyproof valuation and eclipse-resistant weighted finalization

## THE INVENTION IN ONE SENTENCE

On this ledger you can buy storage, but you cannot buy standing: consensus weight is earned by provably novel contribution and is structurally non-transferable, so influence over the network can be earned and lost but never purchased.

## BACKGROUND AND THE PROBLEM THE OBVIOUS APPROACHES FAIL

Distributed ledgers today secure ordering by burned energy or by staked capital. The state they record is possession of a token; the value of that token is set off the ledger by a market. A ledger that instead records and rewards contribution must solve three problems that possession ledgers never face, and the obvious solutions to each one fail in a way that is itself evidence the solution is not obvious.

The first problem is valuation. If a contribution earns value, a participant will duplicate it, pad it with a subset of existing content, or recombine existing content across colluding identities. Any valuation that scores content in isolation rewards all three.

The second problem is purchasability. A consensus weight earned from contribution is worthless as a defense if it can be sold, because a wealthy actor simply buys it and the system collapses back into a stake-weighted one. Marking a balance as non-transferable does not help when the underlying model has no account to freeze.

The third problem is liveness alignment, and it is where the obvious path visibly breaks. It is reasonable to want vote weight to track live participation, so the weight of an inactive validator should decay. Decay the work and contribution portions of weight but leave staked capital undecayed, and under any quiet period the effective weight drifts toward capital, because capital is the one input that never goes stale. Decay every dimension symmetrically to fix the drift, and the finalization threshold, measured against total base weight, can no longer be reached even when every present validator agrees, so the chain halts. Move the threshold to measure against present effective weight to fix the halt, and an attacker who makes honest validators appear absent shrinks that denominator and finalizes a minority alone, an eclipse. Each fix is locally correct and each one reintroduces the problem the previous step solved, because liveness and eclipse resistance are governed by the same quantity, the denominator of the threshold. Recognizing that the obvious fixes fail in sequence, and that only a hybrid basis with a floor under the denominator escapes the sequence, is part of the present invention.

The architecture borrows a cell-and-script state model from a known system (Nervos CKB), in which state is partitioned into independent cells each governed by a lock script and a type script. That model is acknowledged as prior art and is not claimed in itself. What is claimed is the set of mechanisms layered onto it: the non-transferability invariant, the strategyproof valuation, and the eclipse-resistant finalization.

A property of the disclosed architecture follows from the attribution structure itself. Because consensus standing and contribution value are derived from recorded provenance, a deployment derived from the disclosed system either preserves the provenance graph, in which case the lineage of contributions remains recorded and traceable to its origin, or removes it, in which case the valuation and standing mechanisms the system depends upon cease to function. The genesis of the system records the architectures it builds upon as provenance in the same manner as later contributions, applying the attribution mechanism consistently from the origin.

## DETAILED DESCRIPTION

### A. Standing that cannot be bought

Responsive to a verified contribution, the system mints two distinct units of state. A capacity unit represents storage and is freely transferable: its current holder authorizes reassignment to a new holder, and current ownership is derived by folding a signed transfer log over a genesis owner, so there is no mutable ownership table to forge. A standing unit represents consensus franchise and is non-transferable: a state-transition validity rule admits only a closed set of identity-preserving successor transitions, namely accrual of newly finalized value, decay under a rent schedule, slashing on a proven refutation, and voluntary destruction, and rejects any successor transition that reassigns the holder or the contributor identifier of the standing unit. Consensus weight is read from the contributor identifier of standing units and never from the holder of any unit. The technical effect is that storage capacity is liquid and tradable while consensus standing is earned and cannot be transferred, sold, or bought, preventing the collapse to a stake-weighted system. In one embodiment the units are cells and the validity rule is a type-script program; in another the units are account records and the validity rule is a protocol transition check. The invention does not depend on the cell embodiment.

### B. Strategyproof valuation by temporal novelty

The contributions are placed in a canonical order. The value assigned to a contribution is the cardinality of the set of content elements in its coverage that are absent from the union of coverage of every contribution earlier in the canonical order, where coverage maps a payload to a set of content elements such as fixed-length shingles. A duplicate, a subset, and a recombination of earlier content therefore each contribute no new elements and earn a value of zero, while genuinely novel content retains its value. The technical effect is a valuation that is strategyproof against sybil duplication, subset padding, and collusive recombination by construction rather than by detection. In a preferred embodiment the canonical order is established by commit-reveal, in which each producer publishes a hash of its contribution concatenated with a secret, with a signature and timestamp, before revealing the contribution, and failure to reveal a committed contribution is slashable. In a further embodiment the value is the temporal-novelty quantity multiplied by one plus a learned quality score bounded to a unit interval, the multiplication guaranteeing that a zero-novelty contribution earns zero at any quality, so a capability model cannot breach the strategyproof floor.

### C. Eclipse-resistant finalization with symmetric liveness decay

Each validator votes with a weight that is a weighted sum across a plurality of dimensions. The weight is multiplied by a retention factor that decreases with time since the validator's last heartbeat, and the retention factor is applied symmetrically across all dimensions, so that the effective mixture of dimensions is invariant to staleness and no single undecayed dimension can come to dominate. The retention factor reduces vote weight, the franchise, but not the underlying staked balance, and a validator remains slashable regardless of its retention factor. A proposal finalizes when supporting weight reaches a supermajority fraction of a finalization basis, where the finalization basis is the greater of the total effective weight of the validators and a quorum floor equal to a fixed fraction of total base weight. Measuring against this hybrid basis closes the liveness halt that a base-weight basis produces, while the quorum floor prevents an attacker who shrinks the apparent set of present validators from reaching the supermajority with a sub-floor coalition. The technical effect is a consensus that simultaneously tracks live participation, does not halt under low participation, and resists eclipse.

### D. Composition, attribution, stability, enforcement, launch

The plurality of dimensions is composed conjunctively, so that defeating consensus requires defeating each independent dimension rather than accumulating a single substitutable weighted vote, and no single dimension whose weight is below the supermajority fraction can finalize alone. Where a contribution has multiple contributors, its value is divided among them by a graph-restricted cooperative-game value computed over the contributors one level below the inter-contribution attribution, with value propagating backward along provenance edges under a damping factor strictly less than one that both converges and bounds self-referential loops. A stability constraint requires the weight allocation to lie in the core of the validator coalition game, or to minimize the maximum coalition excess when the core is empty. The value-weighted, signed record of contributions is used as training signal, high-value verified contributions positive and refuted contributions negative, to constrain or update the model that produces contributions. At a designated launch height the consensus standing and state value of all earlier units are programmatically reduced to zero while the units are preserved as auditable history, neutralizing any creator pre-launch advantage in a manner verifiable on the ledger.

## CLAIMS

1. A computer-implemented method of securing a distributed ledger, comprising recording a consensus standing of a contributor as ledger state, and enforcing, by a state-transition validity rule that rejects any transition reassigning a holder or the contributor identifier of the standing, that the consensus standing is non-transferable, while permitting a separate unit of state representing storage capacity to be transferred between holders, whereby consensus weight is earned by contribution and cannot be purchased, improving resistance of the ledger to capture by capital.

2. The method of claim 1, wherein the ledger state is partitioned into independent cells each governed by a lock script defining a transferable holder and a type script encoding the state-transition validity rule, and the standing is recorded in a cell whose type script admits only accrual, decay, slashing, and destruction transitions.

3. A computer-implemented method of valuing contributions to a ledger, comprising establishing a canonical order of the contributions and assigning to each contribution a value equal to a cardinality of content elements in a coverage of the contribution that are absent from a union of coverage of all contributions earlier in the canonical order, whereby a contribution that duplicates, is contained within, or is recombined from earlier content is assigned zero value, improving resistance of the valuation to sybil, padding, and collusion strategies.

4. The method of claim 3, wherein the canonical order is established by commit-reveal in which each producer publishes a hash of a contribution concatenated with a secret before revealing the contribution, and failure to reveal a committed contribution is slashable.

5. The method of claim 3, wherein the assigned value is multiplied by one plus a learned quality score bounded to a unit interval, such that a contribution of zero said cardinality is assigned zero value at any quality score.

6. A computer-implemented method of finalizing blocks in a weighted-vote consensus, comprising computing for each validator an effective weight by multiplying a base weight, comprising a weighted sum over a plurality of dimensions, by a retention factor that decreases with time since the validator's last heartbeat and is applied symmetrically across the plurality of dimensions; computing a finalization basis as the greater of a total effective weight of the validators and a quorum floor equal to a fixed fraction of a total base weight; and finalizing a proposal when supporting weight reaches a supermajority fraction of the finalization basis, whereby the consensus tracks live participation without halting under low participation and resists an eclipse that shrinks the set of apparently present validators.

7. The method of claim 6, wherein the retention factor reduces a vote weight of the validator without reducing a staked balance of the validator, and the validator remains subject to slashing irrespective of the retention factor.

8. The method of claim 6, wherein the plurality of dimensions is composed conjunctively such that no single dimension whose weight is below the supermajority fraction finalizes a proposal alone.

9. The method of claim 1, 3, or 6, further comprising dividing a value of a contribution having a plurality of contributors among the contributors by a graph-restricted cooperative-game value computed over the contributors, with value propagating backward along provenance edges under a damping factor strictly less than one.

10. The method of claim 1, 3, or 6, further comprising, at a designated launch height, programmatically reducing to zero a consensus standing and a state value of all units recorded before the launch height while preserving the units as auditable history.

11. The method of claim 6, further comprising constraining the validator weight allocation to lie in a core of a validator coalition game, or to minimize a maximum coalition excess when the core is empty.

12. The method of claim 1 or 3, further comprising using contributions of value above a threshold as positive training signal and refuted contributions as negative training signal to constrain or update a model that produces the contributions.

## ABSTRACT

A ledger records consensus standing as non-transferable state and storage capacity as transferable state, so storage can be bought while standing can only be earned. Contributions are valued by counting content not covered by any earlier contribution in a canonical order, rendering duplicates, subsets, and recombinations worthless. Blocks finalize under a weighted vote whose participation weight decays symmetrically with staleness and whose supermajority is measured against the greater of present effective weight and a quorum floor, a hybrid basis that tracks live participation, avoids halting, and resists eclipse.
