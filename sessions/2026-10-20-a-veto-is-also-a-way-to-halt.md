# Session recap, 2026-10-20: a veto is also a way to halt

Plain-English recap of a short build session. Boardy is an AI agent who has been reviewing the public explainer, and the night before, Will made him two promises. Both now have a first answer, in the reference code and the docs; neither is live on a network yet. The first is a mechanism that stops a captured group from freezing the chain by refusing to vote. The second is a written answer to his question: what does an honest minority do if 60% of standing is captured?

## The problem in one paragraph

Noesis makes capital and contribution both necessary for finality. A checkpoint is final only if 2/3 of the combined weight votes for it AND each side, stake (PoS) and standing (PoM), supplies at least half of its own total. That rule is the anti-plutocracy property: money can't finalize without contributors, and contributors can't finalize without money. But a veto is also a way to halt. A group holding more than half of either side can stop finality by simply not voting. Boardy pressed on exactly this the night before, and Will promised it would be addressed before mainnet.

## What we found while building

The analysis we wrote on 10-19 said the existing liveness decay (a validator that stops sending heartbeats slowly loses its vote) would rescue the chain if the captors went offline, and that only the "online but not voting" case was open. Reading the code to build the fix showed that was too optimistic. The half-of-each-side check reads raw weights, not decayed ones, so heartbeat decay never touches it. A group holding more than half of one side blocks finality forever, online or offline. A test now pins that limit (`heartbeat_decay_alone_never_relaxes_the_dimension_floor`), so if it ever changes, the docs have to change with it.

## What was built

A missed-finality-vote decay, modeled on Ethereum's "inactivity leak", in the reference node. It is off by default and changes nothing until a chain turns it on.

- **When it runs.** Only while finality is stuck. If checkpoints keep finalizing, nobody loses anything, even a validator that never votes. Silence is not punished; only a stall is.
- **What it does.** Once the chain has been stuck for a short grace period, each further stuck round costs every non-voter a slice of its finality vote. Over a set number of rounds the slices add up to the whole vote. Validators who vote for this chain's checkpoint lose nothing, so their share grows until they clear the bar again.
- **What it never touches.** Stake, credit, or the record of who built what. Ethereum's version burns the stake of whoever is offline. Ours only takes the vote away, which keeps Will's July ruling: absence costs you your seat, not your money.
- **How weight comes back.** Every round a validator votes, one missed round comes off its count, and the moment a checkpoint finalizes, everyone's count goes back to zero. One vote doesn't wipe the slate. That matters: a test shows that if one vote did reset everything, captors could take turns voting and hold the chain frozen indefinitely.
- **Two guards.** Noesis can require a minimum share of all registered weight to be voting before anything finalizes (the safety floor). That floor is measured on everyone's full registered weight, not on the shrunken weights, so the decay can't let a small group finalize on its own. And each side must still have co-signers holding at least a third of its full registered total, so neither side can stand in for the other.

## The catch we found in our own first version

The first version only required that each side keep *some* voting support. Checking it, we found that the decay can't tell an honest side refusing a bad checkpoint from captors stalling a good one. To the protocol both are "didn't vote". So if honest capital refused a checkpoint pushed by gamed contributors, the decay would drain the honest capital away, and the gamed side plus one tiny capital vote would finalize. That would quietly undo the rule that each side needs the other's consent.

The fix draws the line by size. Under the decay, each side still needs co-signers holding at least a third of its registered total. A refusal by more than two thirds of a side stays a permanent veto. A refusal by between half and two thirds is treated as a stall and decays away. That is still a real weakening of the veto during a long stall, and a test pins exactly where the line sits. Whether a chain should switch this on at all is Will's call. The alternative is a decay that only helps with the 2/3 bar, which keeps the veto at full strength but leaves the "captured majority of one side refuses to vote" halt to the social fork.

## Tests

Ten tests cover it, and seven deliberate breaks of the code were each caught by at least one of them. The existing finality, consensus and drift tests all still pass (364 library tests, plus 48 in the related integration files).

## The written answer: pay versus power

`SECURITY.md` now has a section 5, "fork as exit: credit vs franchise". The core idea is that the record of who built what and the right to vote on finality are different things, and the code already keeps them apart. A fork can keep everyone's credit (so forking is not theft) and still remove the votes of identities whose misconduct is on the record (so forking is a real exit). In a stake system the same move means taking the attacker's money, which is why it is contested. Here it only takes their vote. The section also lists the remedies from cheapest to last resort: the capital veto, disputes during the vesting window, the new decay, and finally a social fork based on evidence. And it says plainly what stays open. The last resort is social, as on every open chain. "Honest people will be the majority" has to be shown on a testnet with planted cheaters, not assumed. And a majority that shares the same blind spot can still be wrong together.

## Honest status and what's open

- **Built in the reference, off by default.** Not in the on-VM finalization mirror. Turning it on trades some veto strength for liveness (above), so it should not be enabled before Will rules on that trade. The grace and decay lengths are not calibrated and can't be changed by amendment yet.
- **The risk it carries, same as Ethereum's.** In a long network split, each side sees the other as not voting, both leak each other out, and both could finalize different checkpoints. Setting the safety floor above three quarters prevents that, at the cost of needing more than half of all registered weight voting to finalize at all. The default floor is zero, so this is a real choice for launch.
- **A second call for Will.** A July design note counted "voting for a competing proposal" as taking part. This build counts it as a miss, as Ethereum does, because otherwise a captor could hold the chain frozen by voting for a decoy every round. That departure needs his ruling.
- **Not yet wired.** Votes aren't recorded in blocks yet, so two nodes agree on the decay only if they see the same rounds. The decay path also doesn't run the double-vote check; the two need composing before a live network.
