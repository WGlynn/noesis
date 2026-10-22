# Ruling: a finality stall over a wrong finalization (2026-10-21)

Linked from SECURITY.md section 5. Code cited is the current sketch, not law; pointers checked 2026-10-21 at `720a5843`. Plain-English recap: `sessions/2026-10-21-stall-over-hack-and-stale-binaries.md`.

## The ruling

Will, 2026-10-21: "this is the way". A finality stall is preferred over a wrong finalization: a stall over a hack.

The choice comes down to one setting: how long a side has to stay silent before its silence stops counting as a "no" on the half-of-each-side rule (`MIN_DIM_BPS = 5000`, `node/src/runtime.rs:1676`). It is set to forever for now. That keeps the 2026-10-20 ruling (5b2bd81c) unchanged. The missed-vote decay can relax the two-thirds bar. It never relaxes the per-side rule (pinned by `a_refusal_by_more_than_half_of_a_dimension_is_a_permanent_veto` in `node/tests/vote_leak.rs`). If more than half of one side withholds, finality stalls until the social fork in SECURITY.md section 5 resolves it. Testnet measures how often absence, as opposed to deliberate refusal, actually stalls finality. The window gets shortened only if that data says it's worth it.

Blocks keep coming during any finality stall. Only finality waits.

## Why: the reasoning Will and Jarvis worked through

1. **No design gets both.** When enough participants go quiet, a protocol has to either stop finalizing or risk finalizing something wrong (Fischer, Lynch and Paterson, 1985). Every chain has chosen. Bitcoin chose liveness: it keeps producing blocks however many miners leave, and no block is ever final, only harder to reverse the deeper it sits. Cosmos-style chains chose safety: they stop when more than a third goes quiet. Ethereum is a hybrid, and its inactivity leak accepts that in a long network split both sides can finalize different histories.

2. **A stall can be paid back and a wrong finalization can't.** During a stall nothing wrong has been finalized, so every option stays open: resume, fork, compensate. The unfinalized tip can still be reorganized, which is why the offsets below cap what can move during a stall. A wrong finalization is permanent at the block layer, and undoing it means a fork anyway, after the damage. Scores live a layer up and stay correctable. When the right answer is unclear, pick the failure that can still be undone.

3. **The per-side rule is the anti-plutocracy property.** It is what stops capital from finalizing without contributors, and contributors without capital. An expiring floor (proposed by Boardy, 2026-10-21) would put a timer on it: "money can't buy finality" would become "money can't buy finality until the timer runs out". The patch that lets a bonded objection hold the rule protects only whoever can afford the bond, and capital has money where contributors often don't. It would also hand the hardest judgment, whether a side is gamed, to the dispute layer, which is the unsolved part of the system.

4. **We've already seen it fail.** An early build let the missed-vote decay reach the per-side rule. In a test, honest stakers refused to finalize a gamed contribution side. Their refusal looked like silence, so their weight decayed away. The gamed side then finalized with a dust amount of stake. The protocol can't tell refusing from being absent, because both look like "didn't vote" (SECURITY.md section 5).

5. **It isn't a binary.** Forever is one end of the dial. A finite window gives the third option: silence decays, but a signed "no" holds. That heals outages and apathy while keeping the veto for anyone who actively refuses. Its weakness is that an attacker who can censor the honest side's "no" votes for the whole window, by controlling block production or knocking those voters offline, makes them look absent. So we start at the safe end and let measurements move it.

6. **Most real stalls are bugs and absence, not malice.** Ethereum lost finality twice in May 2023, for about 25 minutes and about an hour, from client bugs, and blocks kept coming. In this sketch the likeliest stall is contributors being offline: the per-side rule counts an absent contributor as a "no", contributors aren't professional operators, and nobody pays them to vote. And today a stall costs nobody income. Miners keep mining, and contribution rewards aren't tied to finality ("vesting gates FINALITY weight only, not reward", `runtime.rs:3081`). The incentive against stalling has to be built.

7. **Bitcoin's security comes from its size, not from choosing liveness.** Rewriting Bitcoin takes a majority of the largest hashpower in the world. The same design on a small chain gets 51%-attacked: Ethereum Classic and Bitcoin Gold were both reorganized more than once. Our proof of work is small at launch, so it can't carry safety on its own, and the finality layer has to.

8. **What's left is mostly the time a fork takes to coordinate, and that can improve.** Two more costs come with it: a fork can split the community (Ethereum and Ethereum Classic in 2016, Bitcoin and Bitcoin Cash in 2017), and whoever chooses can choose wrong, for example by mistaking honest refusers for captors. A signed public record of who withheld shrinks both. Bitcoin's own history gives the range. Forks with clear evidence coordinated in hours: the 2010 value-overflow bug in about five hours, the 2013 accidental split in about six. A contested question of values took about two years and ended in a split.

## The offsets (designed, not built)

These make stalls rarer, cheaper, shorter and insured:

1. **Rarer.** Make voting effortless: a contributor's node votes automatically, or the contributor can hand their vote (never their standing) to a node they pick and take it back at any time. Handing votes over concentrates power and has to be measured.
2. **Cheaper.** Small payments keep going on the proof-of-work tip ("Bitcoin mode", heaviest work over the last finalized checkpoint, `node/src/reorg.rs`), capped by value, while large exits wait for finality. On-chain deadlines (loans, liquidations, expiries) run on finalized time, so nobody gets liquidated or misses a deadline because of a stall. The lesson here is Black Thursday, 2020: Ethereum was so congested that MakerDAO vaults were auctioned for zero and about $8M was lost.
3. **Shorter.** Rewards earned during a stall stay locked until finality returns, like Bitcoin's 100-block maturity on mined coins. That way everyone pays for every hour of a stall, including whoever is causing it. The fork comes ready-made, carrying the signed record of who withheld.
4. **Insured.** A small slice of fees goes into a shared pool that compensates whoever a stall still hurts. It's the same idea as VibeSwap's insurance pools: spread the cost of the risk nobody can remove.

## When to revisit

On a testnet where contributors run voting nodes (the single-node devnet can't show this: its placeholder validators never withhold), record every finality stall: its cause (absence, signed refusal, or a bug), its length, and who it hurt. Shorten the silence window only if absence stalls turn out frequent or costly enough, and only if the censorship needed to exploit the shorter window would cost an attacker more than it could gain.
