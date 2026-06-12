# PROVISIONAL PATENT APPLICATION (DRAFT — PRIVATE, attorney review required)

> Status: Jarvis-drafted spec + first claim set for inventor and patent counsel.
> NOT filed. NOT legal advice. Inventor: Will Glynn. File the provisional to lock the
> priority date BEFORE any third-party (including any external LLM) review. Keep in the
> private repo only.

## TITLE

Systems and methods for a verifiable contribution ledger with strategyproof valuation, non-transferable consensus standing, and eclipse-resistant weighted finalization

## FIELD

The disclosure relates to distributed ledger systems, and more specifically to computer-implemented methods for recording units of contribution as ledger state, assigning value to those units in a manner resistant to manipulation, deriving a consensus weight from accumulated value, and finalizing blocks under a weighted vote whose participation weight decays with time.

## BACKGROUND

Existing distributed ledgers secure transaction ordering by proof of expended energy or by staked capital. In such systems the recorded state is possession of a token, the work performed is decoupled from any useful output, and the value of a unit is priced by an external market rather than measured on the ledger. Systems that attempt to measure contribution on-chain face three unsolved problems: (i) valuation can be gamed by duplicating, padding, or collusively recombining contributions; (ii) a consensus weight earned from contribution becomes purchasable if it is transferable, collapsing the system to a stake-weighted one; and (iii) aligning vote weight with live participation, by decaying the weight of inactive validators, destabilizes finalization. The present disclosure addresses all three with concrete technical mechanisms.

## SUMMARY

Disclosed is a ledger in which a unit of contribution is recorded as a state cell having a lock script that defines a transferable owner and a type script that encodes state-transition rules. The type script enforces a strategyproof valuation rule and a non-transferability invariant. Value is assigned by a temporal-novelty rule keyed to a commit-reveal ordering, such that only content not previously covered earns value, rendering duplication, subset padding, and collusive recombination valueless by construction. Verified value mints two cells: a transferable capacity cell representing a unit of storage, and a non-transferable standing cell whose type script admits only identity-preserving successor transitions and rejects any reassignment of owner or contributor, so that consensus standing is earned and cannot be bought. Consensus finalizes blocks under a weighted vote in which each validator's weight decays symmetrically across all weighted dimensions with time since a last heartbeat, and in which the supermajority threshold is measured against a finalization basis equal to the greater of the present effective weight and a quorum floor that is a fixed fraction of base weight, preventing a denominator-shrinking eclipse from enabling a minority to finalize.

## BRIEF DESCRIPTION OF THE FIGURES

- Figure 1: the pipeline from a unit of contribution to consensus weight and tradable state.
- Figure 2: the two-cell mint separating non-transferable standing from transferable capacity.
- Figure 3: the temporal-novelty valuation over a commit-reveal order.
- Figure 4: the finalization basis decision among base, effective, and hybrid bases.
- Figure 5: the two-level recursive attribution over a provenance graph.

## DETAILED DESCRIPTION

### 1. State cell and scripts

State is partitioned into independent cells. Each cell comprises an identifier, a lock script, a type script, an optional reference to a parent cell, a commit-order timestamp, and a data payload. The lock script holds a current owner identifier and authorizes transfer of the cell; the type script holds a contributor identifier and a program hash, and is executed on every state transition that creates or consumes the cell. A transition is valid only if the type script program returns success. Because cells are independent and self-validating via their scripts, state partitions and validates in parallel.

### 2. Provenance and un-front-runnable authorship

A cell records the inputs from which its output was produced, so that the production is reproducible rather than merely possessed. Authorship is bound by commit-reveal: a producer publishes a commitment comprising a cryptographic hash of the contribution concatenated with a secret, together with a signature and a timestamp, before revealing the contribution. Failure to reveal a valid contribution matching a published commitment is a slashable event. The commit-reveal procedure binds authorship and ordering before disclosure and supplies a canonical order used by the valuation rule.

### 3. Strategyproof temporal-novelty valuation

A coverage function maps a contribution's payload to a set of content elements (in one embodiment, a set of fixed-length shingles of the payload). The value of a contribution is the count of content elements in its coverage that are not present in the union of coverage of all earlier-committed contributions, as established by the commit-reveal order. Consequently a contribution that duplicates earlier content, that is a subset of earlier content, or that collusively recombines earlier content earns zero value, while a genuinely novel contribution retains its value. In a further embodiment the value is the temporal-novelty quantity multiplied by one plus a learned quality score in a bounded interval, the multiplication ensuring that a zero-novelty contribution earns zero regardless of quality, so that a capability model cannot breach the strategyproof floor.

### 4. Two-cell mint and the non-transferability invariant

Responsive to a verified contribution, the system mints two cells. A capacity cell is transferable: its lock script permits the current owner to sign a reassignment to a new owner, and current ownership is derived by folding a signed transfer log over a genesis owner. A standing cell is non-transferable: its type script admits only a closed set of identity-preserving successor transitions, namely accrual of newly finalized value, decay according to a rent schedule, slashing on a proven refutation, and voluntary destruction, and rejects any successor that reassigns the owner lock or the contributor identifier. Consensus weight is read from the contributor identifier of standing cells and never from the owner of capacity cells, so that storage capacity may be bought and sold while consensus standing may only be earned.

### 5. Proof-of-mind weight and two-level recursive attribution

A participant's consensus weight is the accumulated value of the verified, provenance-complete cells attributed to it. Where a cell has multiple contributors, the cell's value is divided among contributors by a cooperative-game value computed one level below, over the contributors as players, using a graph-restricted Shapley (Myerson) value estimated by permutation sampling. The same machinery computes inter-cell attribution over the provenance graph and intra-cell attribution over a cell's contributors, so that the economy is two-level recursive. Value propagates backward along provenance edges with a damping factor strictly less than one, which both converges and bounds self-referential attribution loops.

### 6. Eclipse-resistant weighted finalization with liveness decay

Validators vote on a proposed block, each contributing a weight that is a weighted sum across a plurality of dimensions. To align weight with live participation, the weight of each validator is multiplied by a retention factor that decreases with the time since the validator's last heartbeat, and the decay is applied symmetrically across all weighted dimensions so that the effective mixture of dimensions is invariant to staleness. The decay reduces vote weight (the franchise) but not the underlying staked balance. A proposal finalizes when the supporting weight reaches a supermajority fraction of a finalization basis, where the finalization basis is computed as the greater of (a) the total effective decayed weight of validators and (b) a quorum floor equal to a fixed fraction of total base weight. Measuring the threshold against this hybrid basis closes the liveness halt that arises when the basis is total base weight, while the quorum floor prevents an attacker who causes honest validators to appear absent from shrinking the denominator and thereby enabling a sub-floor coalition to finalize.

### 7. Composition, stability, and slashing

The plurality of weighted dimensions is composed conjunctively, such that defeating consensus requires defeating each independent dimension, rather than as a single substitutable weighted vote; a dimension whose weight is below the supermajority threshold cannot finalize alone. A stability constraint requires the weight allocation to lie in the core of the validator coalition game, or, where the core is empty, to minimize the maximum coalition excess (the nucleolus), so that no validator coalition profits by deviating. Slashable events include failure to reveal a committed contribution, a refuted attestation, a refuted value claim resolved through a commit-dispute-finalize window, and equivocation by voting for two distinct proposals in one epoch; slashing remains possible regardless of how decayed a validator's franchise is.

### 8. Backwards-enforcement of the producing model

Because each cell is provenance-complete, owner-authenticated, and value-weighted, the ledger constitutes a value-weighted dataset. The system uses high-value verified cells as positive signal and refuted contributions as negative signal to constrain or update the model that produces contributions, such that the verified record disciplines the producing model, which in turn produces contributions that the record verifies.

### 9. Provable fair launch

At a designated launch height the system programmatically reduces to zero the consensus standing and state value of all cells produced before the launch height, while preserving those cells as auditable history, so that any pre-launch advantage of the creator is neutralized in a manner that is verifiable on-chain rather than asserted.

## CLAIMS

1. A distributed ledger system comprising a processor and memory storing instructions that, when executed, partition ledger state into independent cells, each cell comprising a lock script defining a transferable owner and a type script encoding state-transition rules, wherein a transition that creates or consumes a cell is valid only if the type script returns success, and wherein the type script of a standing cell admits only identity-preserving successor transitions and rejects any transition that reassigns the owner or a contributor identifier of the standing cell.

2. The system of claim 1, wherein consensus weight is read from the contributor identifier of standing cells and not from the owner of any cell, such that a transferable capacity cell representing storage may change owners while the consensus standing of a contributor cannot be transferred.

3. A computer-implemented method comprising: receiving a plurality of commitments, each commitment comprising a hash of a contribution concatenated with a secret; establishing a canonical order of the contributions from the revealed commitments; and assigning to each contribution a value equal to a measure of the contribution's content that is not present in a union of the content of all contributions earlier in the canonical order; whereby a contribution duplicating, contained within, or recombined from earlier content is assigned zero value.

4. The method of claim 3, wherein the assigned value is multiplied by a quantity comprising one plus a learned quality score in a bounded interval, such that a contribution of zero novelty is assigned zero value irrespective of the quality score.

5. A computer-implemented method for finalizing blocks in a weighted-vote consensus, comprising: computing for each validator an effective weight by multiplying a base weight, comprising a weighted sum over a plurality of dimensions, by a retention factor that decreases with time since the validator's last heartbeat, the retention factor applied symmetrically across the plurality of dimensions; computing a finalization basis as the greater of a total effective weight of the validators and a quorum floor equal to a fixed fraction of a total base weight of the validators; and finalizing a proposal when supporting weight reaches a supermajority fraction of the finalization basis.

6. The method of claim 5, wherein the retention factor reduces a validator's vote weight without reducing a staked balance of the validator, and wherein a validator remains subject to slashing irrespective of the value of its retention factor.

7. The method of claim 5, wherein the plurality of dimensions is composed conjunctively such that finalization requires support across each dimension, and wherein no single dimension whose weight is below the supermajority fraction can finalize a proposal alone.

8. The method of claim 5, further comprising constraining the validator weight allocation to lie in the core of a validator coalition game, or, when the core is empty, to minimize the maximum coalition excess, such that no coalition of validators profits by deviating.

9. A computer-implemented method comprising attributing a value of an outcome among contributing cells of a provenance graph by a graph-restricted cooperative-game value estimated by permutation sampling, and dividing a value of a cell having a plurality of contributors among the contributors by the same graph-restricted cooperative-game value computed over the contributors, wherein value propagates backward along provenance edges with a damping factor strictly less than one.

10. The method of any preceding claim, further comprising, at a designated launch height, programmatically reducing to zero the consensus standing and state value of all cells produced before the launch height while preserving those cells as auditable history.

11. The system of claim 1, further comprising using cells of value above a threshold as positive training signal and refuted contributions as negative training signal to constrain or update a model that produces the contributions recorded as cells.

## ABSTRACT

A distributed ledger records units of contribution as independent state cells, each governed by a type script. Contribution value is assigned by a temporal-novelty rule over a commit-reveal order, so that duplicated, subset, or recombined content earns nothing. Verified value mints a transferable capacity cell and a non-transferable standing cell whose type script rejects reassignment, so storage is purchasable but consensus standing is only earned. Blocks finalize under a weighted vote whose participation weight decays symmetrically with staleness and whose supermajority threshold is measured against the greater of present effective weight and a quorum floor, closing both the liveness halt and the eclipse that a single basis admits.
