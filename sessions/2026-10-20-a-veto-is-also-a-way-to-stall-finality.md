# Session recap, 2026-10-20: a veto is also a way to stall finality

Plain-English recap of a short build session. Boardy is an AI agent who has been reviewing the public explainer, and the night before, Will made him two promises. Both now have a first answer, in the reference code and the docs; neither is live on a network yet. The first is a mechanism that stops a captured group from freezing the chain by refusing to vote. The second is a written answer to his question: what does an honest minority do if 60% of standing is captured?

## The problem in one paragraph

Noesis makes capital and contribution both necessary for finality. A checkpoint is final only if 2/3 of the combined weight votes for it AND each side, stake (PoS) and standing (PoM), supplies at least half of its own total. That rule is the anti-plutocracy property: money can't finalize without contributors, and contributors can't finalize without money. But a veto is also a way to stall finality. A group holding more than half of either side can stop finality by simply not voting, and a group holding more than a third of the combined weight can do it too. Blocks keep being produced either way: in Noesis only finality ever stalls, never the chain. Boardy pressed on exactly this the night before, and Will promised it would be addressed before mainnet.

## What we found while building

The analysis we wrote on 10-19 said the existing liveness decay (a validator that stops sending heartbeats slowly loses its vote) would rescue the chain if the captors went offline, and that only the "online but not voting" case was open. Reading the code to build the fix showed that was too optimistic. The half-of-each-side check reads raw weights, not decayed ones, so heartbeat decay never touches it. A group holding more than half of one side blocks finality forever, online or offline. A test now pins that limit (`heartbeat_decay_alone_never_relaxes_the_dimension_floor`), so if it ever changes, the docs have to change with it.

## What was built

A missed-finality-vote decay, modeled on Ethereum's "inactivity leak", in the reference node. It is off by default and changes nothing until a chain turns it on.

- **When it runs.** Only while finality is stuck. If checkpoints keep finalizing, nobody loses anything, even a validator that never votes. Silence is not punished; only a stall is.
- **What it does.** Once finality has been stuck for a short grace period, each further stuck round costs every non-voter a slice of its weight toward the 2/3 bar. Over a set number of rounds the slices add up to the whole vote. Validators who vote for this chain's checkpoint lose nothing, so their share grows until they clear the bar again.
- **What it never touches.** Stake, credit, the record of who built what, or the half-of-each-side rule. Ethereum's version burns the stake of whoever is offline. Ours only shrinks the vote, which keeps Will's July ruling: absence costs you your seat, not your money.
- **How weight comes back.** Every round a validator votes, one missed round comes off its count, and the moment a checkpoint finalizes, everyone's count goes back to zero. One vote doesn't wipe the slate. That matters: a test shows that if one vote did reset everything, withholders could take turns voting and stall finality indefinitely.
- **The safety floor stays put.** Noesis can require a minimum share of all registered weight to be voting before anything finalizes. That floor is measured on everyone's full registered weight, not on the shrunken weights, so the decay can't let a small group finalize on its own.

## The catch we found in our own first version, and Will's ruling

The first version let the decay loosen the half-of-each-side rule too. Checking it, we found that the decay can't tell an honest side refusing a bad checkpoint from captors stalling a good one. To the protocol both are "didn't vote". So if honest capital refused a checkpoint pushed by gamed contributors, the decay would drain the honest capital away, and the gamed side plus one tiny capital vote would finalize. That would quietly undo the rule that each side needs the other's consent.

Will ruled (provisionally, open to change once peers weigh in): the decay helps only with the 2/3 bar and never touches the half-of-each-side rule. A refusal by more than half of either side stays a permanent veto. That means a captured majority of one side can still stall finality. Blocks keep coming while it does, and a test pins that. The answer to that case is the social fork described below, which keeps everyone's credit and takes away the captors' votes. The honest cost: that case gets a social answer, not an automatic one.

## Tests

Eleven tests cover it, and five deliberate breaks of the code were each caught by at least one of them. The existing finality, consensus and drift tests all still pass (364 library tests, plus 48 in the related integration files).

## The written answer: pay versus power

`SECURITY.md` now has a section 5, "fork as exit: credit vs franchise". The core idea is that the record of who built what and the right to vote on finality are different things, and the code already keeps them apart. A fork can keep everyone's credit (so forking is not theft) and still remove the votes of identities whose misconduct is on the record (so forking is a real exit). In a stake system the same move means taking the attacker's money, which is why it is contested. Here it only takes their vote. The section also lists the remedies from cheapest to last resort: the capital veto, disputes during the vesting window, the new decay, and finally a social fork based on evidence. And it says plainly what stays open. The last resort is social, as on every open chain. "Honest people will be the majority" has to be shown on a testnet with planted cheaters, not assumed. And a majority that shares the same blind spot can still be wrong together.

## Honest status and what's open

- **Built in the reference, off by default.** Not in the on-VM finalization mirror. The grace and decay lengths are not calibrated and can't be changed by amendment yet.
- **The risk it carries, narrowed by the ruling.** In a long network split, each side sees the other as not voting and decays it. Because each side still needs half of each dimension, both sides can finalize different checkpoints only if the split is exactly half and half on both sides; an uneven split finalizes on the bigger side only. Setting the safety floor above three quarters removes even that edge case, at the cost of needing more than half of all registered weight voting to finalize at all. The floor is zero today as a deliberate placeholder (a July note chose not to invent a number early), so picking it is still a real choice for launch.
- **A call for Will.** A July design note counted "voting for a competing proposal" as taking part. This build counts it as a miss, as Ethereum does, because otherwise a group could keep finality stalled by voting for a decoy every round. That departure needs his ruling.
- **Not yet wired.** Votes aren't recorded in blocks yet, so two nodes agree on the decay only if they see the same rounds. The decay path also doesn't run the double-vote check; the two need composing before a live network.
