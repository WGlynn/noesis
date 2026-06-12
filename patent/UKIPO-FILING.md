# UKIPO PATENT APPLICATION — description for filing (DRAFT, PRIVATE)

> Filing strategy (per Rodney): UK description-only filing for a filing date and "patent
> pending", no claims / no abstract / no search request (Form 9A), Pay Later. Deadlines: pay
> the application fee within 2 months of filing; file claims, abstract, and the Form 9A search
> request within 12 months. The filing date protects only what this description discloses, and
> claims added later must be supported by it, so the description below is complete and broad.
> This UK filing date can anchor a later US or PCT priority claim within 12 months (Paris
> Convention). Inventor: Will Glynn. Applicant / address for service: to be set (Rodney as
> applicant and representative is fine; inventorship remains Will alone). NOT legal advice.

## INVENTION TITLE

Method and system for earned, non-transferable consensus standing on a distributed contribution ledger, with strategyproof valuation and eclipse-resistant weighted finalization

## BACKGROUND

Distributed ledger systems today secure transaction ordering by proof of expended energy or by staked capital. The state such a ledger records is possession of a token, and the value of that token is set off the ledger by an external market; the computational work performed is decoupled from any useful output. A ledger that instead records and rewards contribution faces three technical problems that possession ledgers never face, and the straightforward solution to each one fails in a manner that motivates the present invention.

First, valuation can be gamed. If a contribution earns value, a participant can duplicate it, pad it with a subset of existing content, or recombine existing content across colluding identities; any valuation that scores content in isolation rewards all three.

Second, an earned consensus weight is purchasable. A consensus weight derived from contribution provides no defence if it can be sold, because a capital-rich actor buys it and the system reverts to a stake-weighted one. Marking a balance non-transferable does not solve this where the state model has no account to freeze.

Third, aligning vote weight with live participation destabilises finalisation, and the obvious fixes fail in sequence. It is desirable for an inactive validator's weight to decay so that influence tracks live participation. If the work and contribution portions of weight decay but staked capital does not, then under any quiet period the effective weight drifts toward capital, the one input that never goes stale. Decaying all dimensions symmetrically removes that drift, but a finalisation threshold measured against total base weight can then no longer be reached even when every present validator agrees, halting the chain. Measuring the threshold against present effective weight removes the halt, but an attacker who causes honest validators to appear absent shrinks that quantity and finalises with a minority, an eclipse. Each fix is locally correct and reintroduces the problem the previous step solved, because liveness and eclipse-resistance are both governed by the same quantity, the denominator of the threshold.

The architecture builds on a known cell-and-script state model (Nervos CKB), in which state is partitioned into independent cells each governed by a lock script and a type script, and on known cooperative-game attribution methods (the Shapley value, graph-restricted Shapley, and Data-Shapley sampling) as used for dependency-credit distillation (Ethereum Deep Funding). These are prior art and are not the invention. The invention is the set of mechanisms described below that are layered onto them. The genesis of the system records the architectures it builds upon as provenance in the same manner as later contributions, so the attribution mechanism is applied consistently from the origin; a deployment derived from the system either preserves that provenance graph, keeping the lineage of contributions traceable, or removes it, in which case the valuation and standing mechanisms cease to function.

## TECHNICAL DESCRIPTION

### State cells and scripts

Ledger state is partitioned into independent cells. Each cell comprises an identifier, a lock script defining a current transferable holder, a type script encoding state-transition rules and holding a contributor identifier and a program hash, an optional reference to a parent cell, a commit-order timestamp, and a data payload. A transition that creates or consumes a cell is valid only if the type script program returns success. Because cells are independent and self-validating through their scripts, state partitions by cell and validates in parallel. In an alternative embodiment the units of state are account records and the transition rules are protocol-level checks rather than per-cell scripts; the mechanisms below do not depend on the cell embodiment.

### Provenance and un-front-runnable authorship

A cell records the inputs from which its output was produced, so production is reproducible rather than merely possessed. Authorship is bound by commit-reveal: a producer publishes a cryptographic hash of the contribution concatenated with a secret, together with a signature and a timestamp, before revealing the contribution. Failure to reveal a valid contribution matching a published commitment is a slashable event. The commit-reveal procedure binds authorship and ordering before disclosure and supplies a canonical order used by the valuation.

### Strategyproof valuation by temporal novelty

A coverage function maps a contribution payload to a set of content elements, in one embodiment a set of fixed-length shingles of the payload. The value assigned to a contribution is the count of content elements in its coverage that are absent from the union of coverage of all contributions earlier in the canonical order. Consequently a contribution that duplicates earlier content, that is a subset of earlier content, or that recombines earlier content across identities contributes no new elements and is assigned zero value, while genuinely novel content retains its value. In a further embodiment the value is this temporal-novelty quantity multiplied by one plus a learned quality score bounded to a unit interval, so that a contribution of zero novelty earns zero at any quality score and a capability model cannot breach the strategyproof floor. In a further embodiment a coverage-similarity floor assigns a value of zero to a contribution whose coverage overlaps the union of the coverage of earlier-ordered contributions above a threshold, treating it as a near-duplicate, so that a contribution that alters only a few content elements of an earlier contribution to escape exact-subset detection still earns nothing.

### Earned, non-transferable standing by a two-unit mint

Responsive to a verified contribution the system mints two units of state. A capacity unit represents storage and is freely transferable: its current holder authorises reassignment to a new holder, and current ownership is derived by folding a signed transfer log over a genesis owner, so there is no mutable ownership table to forge. A standing unit represents consensus franchise and is non-transferable: a state-transition validity rule admits only a closed set of identity-preserving successor transitions, namely accrual of newly finalised value, decay under a rent schedule, slashing on a proven refutation, and voluntary destruction, and rejects any successor transition that reassigns the holder or the contributor identifier of the standing unit. Consensus weight is read from the contributor identifier of standing units and never from the holder of any unit, so storage capacity is liquid and tradable while consensus standing is earned and cannot be transferred, sold, or bought. In the cell embodiment the validity rule is a type-script program and the standing unit is a cell whose type script admits only the listed transitions.

### Eclipse-resistant weighted finalisation with symmetric liveness decay

Validators vote on a proposed block, each contributing a base weight that is a weighted sum across a plurality of dimensions. The base weight is multiplied by a retention factor that decreases with the time since the validator's last heartbeat, the retention factor applied symmetrically across all dimensions so that the effective mixture of dimensions is invariant to staleness and no single undecayed dimension can come to dominate. The retention factor reduces vote weight, the franchise, but not the underlying staked balance, and a validator remains slashable regardless of its retention factor. A proposal finalises when the supporting weight reaches a supermajority fraction of a finalisation basis, where the finalisation basis is computed as the greater of the total effective weight of the validators and a quorum floor equal to a fixed fraction of the total base weight. Measuring against this hybrid basis removes the liveness halt produced by a base-weight basis, while the quorum floor prevents an attacker who shrinks the apparent set of present validators from reaching the supermajority with a sub-floor coalition.

### Conjunctive composition, recursive attribution, stability, enforcement, launch

The plurality of dimensions is composed conjunctively, so that defeating consensus requires defeating each independent dimension rather than accumulating a single substitutable weighted vote, and no single dimension whose weight is below the supermajority fraction finalises alone. Where a contribution has a plurality of contributors, its value is divided among them by a graph-restricted cooperative-game value computed over the contributors one level below the inter-contribution attribution, with value propagating backward along provenance edges under a damping factor strictly less than one that both converges and bounds self-referential attribution loops. A stability constraint requires the validator weight allocation to lie in the core of the validator coalition game, or to minimise the maximum coalition excess when the core is empty, so that no validator coalition profits by deviating. The value-weighted, signed record of contributions is used as a training signal, contributions of value above a threshold as positive signal and refuted contributions as negative signal, to constrain or update the model that produces contributions. At a designated launch height the consensus standing and state value of all units recorded before the launch height are programmatically reduced to zero while the units are preserved as auditable history, neutralising any creator pre-launch advantage in a manner verifiable on the ledger.

## TECHNICAL EFFECT

The described mechanisms solve structural problems of a distributed computing network rather than an abstract economic scheme.

The temporal-novelty valuation makes contribution value strategyproof against sybil duplication, subset padding, and collusive recombination by construction rather than by detection, removing a class of manipulation that isolated-content valuation admits, and the commit-reveal procedure that supplies its canonical order makes authorship and ordering un-front-runnable.

The two-unit mint with a non-transferability invariant enforced by a state-transition validity rule keeps storage capacity tradable while making consensus standing earned and non-purchasable, which prevents the network's security from collapsing to a capital-weighted one.

The symmetric liveness decay keeps the effective mixture of consensus dimensions invariant to validator inactivity, preventing an undecayed dimension from coming to dominate, and the hybrid finalisation basis simultaneously keeps consensus tracking live participation, prevents the network halting under low participation, and resists an eclipse that shrinks the apparent validator set, three properties that no single basis provides.

The conjunctive composition prevents any single consensus dimension below the supermajority threshold from finalising alone, and the core or nucleolus stability constraint makes finalisation defection-proof against validator coalitions. Together these produce a distributed ledger whose security, liveness, and resistance to manipulation are properties of its construction.
