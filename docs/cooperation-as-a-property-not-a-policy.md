# Cooperation as a Property, Not a Policy

*Why crypto's cooperative governance kept being captured, and what building cooperation into the smallest unit changes*

Will Glynn, `noesis`. October 2026. [PDF](cooperation-as-a-property-not-a-policy.pdf) | [LaTeX source](cooperation-as-a-property-not-a-policy.tex)

## Abstract

Crypto’s experiments in cooperation mostly took one form: a vote, held at one layer, over a shared pool of funds. That form was captured again and again, and almost always by a neighboring layer that did not share the cooperative rule. Capital above the vote bought, borrowed or rented it. The price and ordering layers below the vote moved the outcome before the vote could matter. We argue that these failures have one cause. Cooperation was a policy adopted at one layer, not a property of the units that every layer is built from. Capture happens at seams: the points where a property stops holding and someone else decides. We then describe three things. VibeSwap moved cooperation into its batch auction. It could not remove the seams of the chain it runs on. Noesis moves the property into the substrate: standing cannot be transferred, finality needs both capital and contribution, there is no fee and no treasury, and a majority cannot vote the rules away. We also state what remains open, chief among it cooperation that is faked from the inside. Noesis is not live, and its code is cited here as the current sketch, not as settled law.

## 1. Introduction

Most crypto projects that called themselves cooperative were cooperative at exactly one layer. Members pooled funds or tokens and voted on how to use them. Everything around that layer stayed competitive: the votes were transferable tokens, transactions were ordered by whoever built the block, and prices came from outside feeds. The cooperative layer was surrounded by layers that did not share its rule, and each of those layers could reach in.

Our claim is that cooperation is legitimate only when it holds in the smallest unit of the system. Every layer built from those units then inherits it, to the extent that the way they are combined preserves it. We call a *seam* any point where a property stops holding and some other party decides. Capture happens at seams. A design that starts from the bottom does not remove every seam, but it leaves fewer, and it lets us name the ones that remain.

## 2. The record

Each event below is grouped by the direction from which the cooperative layer was captured. Figures are the commonly reported ones; sources are listed at the end.

### 2.1 From above: capital bought, borrowed or rented the vote

- **Beanstalk, April 2022.** About $1 billion of flash-loaned assets gave an attacker roughly 70% of voting weight. That cleared the two-thirds bar of an emergency path that read voting power at that instant, so the vote and its execution happened in one transaction. About $182 million was lost \[2, 3\].

- **Compound, proposal 289, July 2024.** A large holder passed a proposal moving 499,000 COMP, about $24 million, from the treasury into a vault the holder would control. It was withdrawn in exchange for a staking program paying 30% of market reserves \[4\]. The capture was settled by paying the captor.

- **The Curve wars, 2021 to 2022.** Votes that directed CRV emissions became a market, and a marketplace arose for selling them for bribes \[5\]. The cooperative vote became a price.

### 2.2 From inside the vote: the rules of the vote were captured

- **Tornado Cash, May 2023.** A proposal copied an earlier passed one but hid a self-destruct path. After it passed, the code at that address was replaced by code that granted the attacker about 1.2 million votes, against roughly 700,000 legitimate ones \[6\].

- **Solend, June 2022.** A vote, in which one wallet held about 90% of the votes cast, gave the team power to take over a single member’s account. A second vote reversed it within a day \[7\]. The majority had voted itself power over one member’s property.

### 2.3 From below: the layer under the cooperation moved the outcome

- **Mango Markets, October 2022.** The attacker raised the oracle price of MNGO, borrowed against it and drained roughly $110 million. A governance vote then let him keep $47 million as a “bounty,” with votes reportedly cast from tokens taken in the exploit \[8\]. The price layer took the funds and the vote layer approved it afterward. His 2024 conviction was vacated in May 2025 \[9\].

- **Ordering extraction (MEV), ongoing.** On a public mempool, searchers and block builders order other users’ trades and insert their own around them. Regulators now treat this as a market-integrity question \[10\]. However cooperative an application is, the ordering layer beneath it can extract from it.

- **The DAO, June 2016.** A reentrancy bug in the code beneath the vote drained about 3.64 million ETH. The remedy came from the social layer above: a hard fork that edited balances, with the holdouts continuing as Ethereum Classic \[1\].

### 2.4 From beside: operators acted before the vote

- **Arbitrum, AIP-1, March to April 2023.** The foundation had already moved the 750 million ARB that the proposal was meant to approve, and had lent or sold part of it. It described the vote as a “ratification.” The community vote ran 77% against \[11\].

### 2.5 Cooperation as narrative

- **Olympus, 2021 to 2022.** Staking was presented as the cooperative strategy, through a payoff matrix popularized as “(3,3).” Nothing in the mechanism made it an equilibrium. When selling paid better, holders sold, and OHM fell about 90% from its peak \[12\].

## 3. The common cause

In every case cooperation was a decision taken at one layer: participants pooled something and voted on its use. The neighboring layers played a competitive game, and each could reach into the cooperative one.

1.  Votes were objects, so they could be bought, borrowed, rented or forged.

2.  The pool was a target, so capturing the vote paid.

3.  The majority was sovereign over the members, so it could seize from them or approve a theft.

4.  The ordering and prices underneath belonged to someone else, so the outcome could be set before cooperation had a say.

5.  Exit meant confiscation or abandonment.

Bad intent was not what failed. The cooperation sat in a layer whose neighbors did not share it, and it was captured at the seams between them.

## 4. Four tests for legitimate cooperation

1.  **It is an equilibrium.** Each participant does best by cooperating without relying on anyone else’s restraint. We do not claim it is a dominant strategy; that would require cooperating to be best whatever others do, which we have not shown.

2.  **The surplus goes to whoever created it, by a rule fixed before the game**, not to wherever a vote points afterward.

3.  **No neighboring layer can buy, borrow, reorder or vote it away**, because it holds in the unit itself.

4.  **Anyone can leave with what they earned.** Exit keeps credit, so leaving is not confiscation.

The DAO experiments above fail all four.

## 5. VibeSwap: cooperation in the batch

VibeSwap is a decentralized exchange built around a batch auction. It puts the cooperative property into the batch. References are to the public repository at commit `243280eb`.

- **Intent cannot be front-run.** Orders are committed as sealed hashes for 8 seconds and revealed in a 2-second window (`CommitRevealAuction.sol:108,114`). An invalid reveal forfeits half of its deposit (`:140`).

- **No order in a batch can be sandwiched.** Every order in the batch clears at one uniform price computed from the batch (`CommitRevealAuction.sol:792`).

- **No single party sets the order.** The shuffle seed combines every participant’s secret with the hash of the block that ends the reveal window. Settlement must wait at least one more block, so the last revealer cannot also settle in the block it proposes and predict the order (`CommitRevealAuction.sol:809-832`).

- **The surplus follows a rule fixed in advance.** Rewards are split by the Shapley value, enforcing efficiency, symmetry, the null player and pairwise proportionality, with a 1% floor (`ShapleyDistributor.sol:30-33,98`).

- **The single-block attack is blocked at the pool.** A guard rejects two interactions by the same party in one block (`VibeAMM.sol:411-415`).

- **Governance is fenced.** Changes pass a 48-hour timelock and a fairness veto. Upgrades of the reward contract must also report the same axiom version. (See `GovernanceGuard.sol` and `ShapleyDistributor.sol:2126`.)

### 5.1 The seams VibeSwap could not remove

VibeSwap runs on a chain it does not control, so it keeps seams on both sides.

- **Below.** Block builders still choose which commitments are included. They cannot reorder within a batch, but they can leave orders out. Part of the shuffle’s entropy also comes from the host chain: whoever produces the block that ends the reveal window can still bias its hash, at the cost of the blocks it discards. Prices still come from an outside feed.

- **Above.** The contracts are upgradeable by an owner (`CommitRevealAuction.sol:1644`). The axiom-version check compares a label that the new implementation reports about itself. It stops an honest or mislabelled change, not a malicious implementation that reports the current label. Priority bids are forwarded to a DAO treasury (`VibeSwapCore.sol:1215,1591`), which is the vote-over-a-pool shape described above, defended but present.

Until this note was written, the contracts’ own documentation described governance capture as structurally impossible. We corrected those comments (VibeSwap commits `f941c523` and `243280eb`), because the accurate description is that capture is slower and harder. The stronger remedy on a general-purpose chain is to remove the upgrade path. Once the batch, shuffle and reward contracts are audited, they can be deployed so that no one can change them. Their rules then cannot be voted on. Changing them means a new deployment, which users must choose to move to.

This is the reason VibeSwap led to Noesis. Cooperation can be fenced on top of an extractive base layer, but it cannot be made a property while the layers beneath and above it belong to someone else.

## 6. Noesis: cooperation in the substrate

Noesis is a chain in which the cooperative property is part of the state and the consensus rule rather than an application on top. The table below maps each failure shape to its Noesis mechanism. References are to the public repository at commit `0862b85f`. “Reference” means built and tested in the reference implementation; none of it has faced a live adversary.

| Failure shape | Noesis mechanism | Status |
|:---|:---|:---|
| Votes bought, borrowed or rented | Standing is bound to the contributor at mint and never reassigned on transfer (`node/src/lib.rs:144`); a transition that reassigns it is rejected (`lib.rs:656`, tests at `750, 762`). Storage can be bought; a say cannot. There is no vote object to lend. | Built (reference) |
| Capital captures the cooperative layer, or the reverse | Finality needs two thirds of combined stake and standing weight, and each of stake and standing must supply at least half of its own total (`runtime.rs:1661`, `1673`). That floor is a constitutional constant, not a governance parameter. | Built (reference) |
| The pool is captured | There is no pool. The node code contains no fee and no treasury. Penalties are burned, paid to no one. The only producer payment is new money minted in proportion to work (`chainspec.rs:52`). | Built, by absence |
| A majority seizes or rewrites | Finality chooses only among blocks that already satisfy the rules (`runtime.rs:1252`). A majority can stall or censor; it cannot vote a theft through. Changing the rules is a fork. | Built (reference) |
| Outside prices move the outcome | No oracle layer: token validity is a pure function of the transaction (`tokens.rs:10-13`). | Built (reference) |
| Ordering extracts from users | A block is a commit-reveal batch in canonical order with a consensus-sourced shuffle; a producer-favorable reorder is rejected. | Reorder: built. Inclusion: open |
| Exit means confiscation | A fork keeps everyone’s credit and can revoke the franchise of identities whose misconduct is on the record (`SECURITY.md`, section 5). | Designed |
| Cooperation as narrative | Value vests only when minds whose capital is independent of the contributor’s build on the work (`lib.rs:7384`). It pays for what others used, not for staying in. | Gate built; capital signal open |

*Table 1. Failure shapes from Section 2 and the Noesis mechanism that addresses each.*


## 7. What remains open

- **Cooperation faked from the inside.** Building from the bottom addresses capture from above and below. The remaining attack comes from inside: genuinely distinct minds who vouch only for one another. No check that looks only inside the contribution graph separates such a ring from genuine work; our own simulation measured no separation. The OPH program has the same shape. Its compact paper gives a countermodel in which two disconnected components each satisfy every variational identity with different constants \[13\], so a region cut off from the rest settles into its own consistent world. Bernhard Mueller suggested, in conversation, reading this through black holes: a black hole as a quarantined set of observers. In our terms, every new set of minds would start quarantined, and its prices would count outside only once outsiders use the work. That places the remedy in the unit, since each contribution vests on outside use, rather than in a committee that judges rings. A genuine new group is treated the same way at first, which is fair: it is not penalized, only not yet paid by the outside. The boundary still needs a measure of capital independence that is not yet built.

- **Inclusion.** A producer cannot reorder a block but can still choose what enters it.

- **Mutualized risk.** Of the three elements of cooperative capitalism (mutualized risk, competition on value, fair distribution) the first is the thinnest in the code. Every risk instrument is an individual bond, penalties are burned, and there is no shared risk pool.

- **Off-chain bribery.** Paying a participant outside the chain remains open here, as on every chain.

- **Live conditions.** None of this has faced a real adversary with real money at stake.

## 8. Conclusion

Crypto tried cooperation as a vote over a shared pool, and each well-known failure was a neighboring layer reaching in. Capital bought or borrowed the vote. The ordering or price layer underneath moved the outcome first. Or the majority voted itself the pool. We took the other route and put the fairness in the smallest unit: each batch in VibeSwap, and each cell and finality vote in Noesis. Standing cannot be bought because it is not transferable. Capital cannot finalize without contributors, nor contributors without capital. There is no treasury to capture. What remains is cooperation that is faked from the inside, and that problem is being worked on in public.

## References

1.  “The DAO.” Wikipedia. <https://en.wikipedia.org/wiki/The_DAO>

2.  “Beanstalk DeFi platform loses $182 million in flash-loan attack.” BleepingComputer, April 2022. <https://www.bleepingcomputer.com/news/security/beanstalk-defi-platform-loses-182-million-in-flash-loan-attack/>

3.  “Flash loan governance vulnerability: Beanstalk.” Veridise. <https://veridise.com/blog/audit-insights/flash_loan_governance_vulnerability_beanstalk_182m/>

4.  “The Humpy Dance.” Rekt News, July 2024. <https://rekt.news/the-humpy-dance>

5.  “The Curve Wars.” tokenbrice. <https://tokenbrice.xyz/crv-wars/>

6.  “Attacker hijacks Tornado Cash governance via malicious proposal.” Cointelegraph, May 2023. <https://cointelegraph.com/news/attacker-hijacks-tornado-cash-governance-via-malicious-proposal>

7.  “DeFi protocol Solend passes governance vote to reverse emergency powers.” CoinDesk, June 2022. <https://www.coindesk.com/business/2022/06/20/defi-protocol-solend-passes-governance-vote-to-reverse-emergency-powers>

8.  “Mango Markets exploiter seeks to keep disputed funds paid as bug bounty.” Cointelegraph, October 2022. <https://cointelegraph.com/news/mango-markets-exploiter-seeks-to-keep-disputed-funds-paid-as-bug-bounty>

9.  “U.S. judge overturns fraud convictions of Mango Markets exploiter Eisenberg.” The Block, May 2025. <https://www.theblock.co/post/355666/u-s-judge-overturns-fraud-convictions-of-mango-markets-exploiter-eisenberg-determining-improper-venue>

10. European Securities and Markets Authority. “Maximal Extractable Value: implications for crypto markets.” July 2025. <https://www.esma.europa.eu/sites/default/files/2025-07/ESMA50-481369926-29744_Maximal_Extractable_Value_Implications_for_crypto_markets.pdf>

11. “Contentious Arbitrum vote over $1B in tokens a ‘ratification,’ not request, says foundation.” CoinDesk, April 2023. <https://www.coindesk.com/business/2023/04/02/contentious-arbitrum-vote-over-1b-in-tokens-ratification-not-request-says-foundation>

12. “OlympusDAO’s slide continues, OHM down 90% from all-time high.” The Block. <https://www.theblock.co/post/130597/olympusdaos-slide-continues-ohm-down-90-from-all-time-high>

13. B. Mueller. *Recovering relativity and Standard Model structure from observer-overlap consistency* (compact paper), disconnected-region countermodel. Observer Patch Holography repository. <https://github.com/FloatingPragma/observer-patch-holography>
