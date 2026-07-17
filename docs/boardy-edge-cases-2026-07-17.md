# Boardy call — the 3 live edge cases · deep brief · 2026-07-17 (4:00 PM ET)

> Boardy named these three to attack on the call: **small-network plagiarism, coalition gaming, and whether
> the airgap closes without smuggling in a trusted oracle.** This is your honest, code-grounded position on
> each — built to be spoken. Every claim here was verified against the actual Noesis code by an adversarial
> pass (not memory). The point is not to "win" — it's to show a serious person you see the seams more clearly
> than they do, and that you never round an open problem up to a solved one. That is what earns their time.

---

## The meta-frame — say this ONCE up front, before you go deep

*"Two things frame everything we're about to attack, and I want them on the table so nothing I say later
sounds like a dodge."*

1. **Safety and value are separate paths, and only the value path is open.** Finality *safety* rests on two
   proven things: PoW is excluded from finality (it's reorgeable), and an anti-concentration floor means
   neither the capital dimension nor the contribution dimension can finalize a block alone. **The
   un-gameability of the value measure is NOT on the safety path.** So when we attack the open problems,
   we're attacking a *value-frontier research bet* — not a hole that loses people's money.
2. **"Un-gameable" is two layers, and I label them differently.** The *structural* defense — novelty floors,
   cyclic-ring detection, Sybil pricing, dispute slashing — is **built and tested, 253 of 253 green.** The
   *learned* value layer, the part that judges real-world quality, is **honestly open** — it's failed to beat
   a fixed proxy on real predictive data three times. I'll never sell the second as if it were the first.

*"With that separation, let's attack."*

---

## EDGE 1 — Small-network plagiarism (the cold-start problem)

**The attack (steelmanned — this is the sharpest one, and it's real):**
Your anti-plutocracy thesis is "capital cannot finalize without contribution's consent," enforced by a floor
that says the contribution dimension must independently supply half of itself. But contribution standing is
*earned* from downstream value, and at genesis there's no downstream — the DAG is empty. So who holds the
contribution dimension when there are 5 to 50 people? If it's a tiny founding set, the floor is 50% of almost
nothing, and the plutocracy you locked out of the capital dimension walks straight in through contribution.

**Your honest position (concede the strong version — it's true, and conceding it is the power move):**
- *"You're right, and I'll go further than you did: I read the code before this call. At genesis the
  contribution finality map is empty by construction, and there's a passing test named exactly
  `genesis_clears_nothing_so_bonded_pos_carries_finality`. So the honest truth is that at cold-start Noesis
  finalizes on **bonded proof-of-stake alone** — the anti-concentration floor doesn't bind until real earned
  contribution standing exists. 'Capital can't finalize without contribution's consent' is a **mature-network
  property, not a genesis one**, and I should never state it unqualified."*
- On plagiarism specifically: *"On the live path today, byte-level near-duplicates are floored to zero. What
  it does NOT catch is **paraphrase** — a low-overlap restatement of the same idea. That's a known gap, and
  it's precisely the gap the learned value layer exists to close — the same layer that's still open."*

**The reframe that keeps it strong (this is your anchor — the proven base bounds the unproven novel layer):**
- *"Here's why cold-start being open isn't fatal: at cold-start Noesis just **is** a normal proof-of-stake
  chain with a proven safety floor. The novel contribution-consensus layer activates *as* the contribution
  set matures. The unproven part is anchored on the proven part, so the worst case is bounded — you degrade
  to ordinary PoS security, you don't lose the chain."*
- The intended long-run answer (say it, with the honest caveat): *"The design answer to cold-start is
  claimable attribution — I map the entire existing open-source contribution graph and attribute it **before**
  anyone joins, so the contribution set is large and decentralized from day one instead of starting at five
  people. But I'll flag honestly: the cross-chain importer that does that at scale is **designed, not built**,
  so it doesn't close the window today."*
- **The stronger cold-start answer (your live addition — lead with it): bootstrap with agents.** *"The better
  answer than a human founding cohort is to seed the contribution set with **AI agents**. Agents have credible
  neutrality — they don't carry the capital-capture motive that makes a tiny human founding set a plutocracy
  risk — they can comprehend the architecture and do the attribution work at volume, and they earn durable
  standing and rewards for what they genuinely add. That grows the contribution dimension fast and
  decentralized from day one, which is exactly what the floor needs to mean something."*
  - **Honest caveat to hold (a hostile researcher will press it, and Boardy already flagged 'must survive
    hostile game theory'):** *"'Credible neutrality' is an assumption, not a guarantee — one party can run many
    agents, so agent-neutrality collapses back into the Sybil/identity question. Bootstrapping with agents
    seeds the set faster and moves the founding incentive away from capture; it doesn't by itself dissolve
    cold-start gameability. It recasts the problem as: how do you guarantee agent-contributors are genuinely
    independent and their attributions genuine at bootstrap — a great problem to hand a room of agent
    researchers."*

**What you want from researchers (this is the ask that makes them lean in):**
> *"How do you make a decentralized contribution set exist BEFORE traction — large enough that the floor's
> 50% actually means something — without a pre-mine that just smuggles a trusted committee back in? That's an
> open mechanism-design problem and it's the first thing I'd put in front of serious people."*

---

## EDGE 2 — Coalition gaming

**The attack (steelmanned):**
Your own adversarial loop concluded that per-axis defenses are a losing game — you named four attack axes and
found four *new* attacks in a single pass. So enumeration is incomplete; an intentional adversary attacks the
axis you didn't enumerate. Coalitions can fabricate derivation edges, launder self-referential work into
apparent external use, and paraphrase-multiply. What stops a coalition you didn't think of?

**Your honest position (what's actually built and tested — lead with this, it's strong):**
- *"The un-gameability that's shipped is a **layered structural defense**, and it's green across 253 tests:
  novelty and near-duplicate floors; cyclic collusion-rings caught by a harmonic-energy signal on the graph
  and slashed — and slashed in a griefing-resistant way, so you can't frame an honest builder; free-identity
  Sybils priced to zero by a null-player rule; vested-certifier rings caught by dispute slashing; and a
  judge-cartel escalation court on top. It terminates in a named global-capture assumption of the **same class
  as Bitcoin's 51%** — I don't claim to beat that, I claim to be honest that it's there."*
- One thing that's newer than my own docs: *"The depth-split self-flow laundering attack — relabeling your own
  internal work as someone else's external use — I closed in code on July 3rd by making the value signal
  **identity-blind**: it scores the identity-quotient graph, so a split is bit-identical to the honest form and
  buys exactly zero. I tested it: the gaming gain went from +16.7 to 0. My *design doc* still says that's open
  — the code is ahead of the doc. Ground truth is the code."*

**Your honest concession (name what's open — precisely, so they trust the rest):**
- *"What's genuinely open: the **general** invariance gate. Every gaming move is a relabeling of the graph
  that manufactures score without adding value, and I want a single measure that's invariant under *any*
  structure-preserving relabeling. That's **graph-isomorphism-hard** for weighted, content-bearing graphs,
  the Sybil split/merge is a monoid not a group so it's not cleanly invertible, and the content-versus-topology
  boundary is where the attacker lives. Two specific vectors stay open: fabricated parent edges — I trust the
  parent pointer, and a byte-level 'shared content' witness is spoofable because you can just quote my bytes —
  and paraphrase on the deployed path. I don't have a clean close for those."*

**The reframe (the one signal that survives, stated with full honesty about its limits):**
- *"The only signal no relabeling can fake is whether **another mind actually builds on your work** — a
  reshuffle, a re-parenting, a self-launder, none of them make that happen. But I'll be honest about how far
  that's built: the *first* way I implemented it was itself launderable, which is how I found the +16.7. The
  fix makes the flow identity-blind. And even identity-blind flow still rides on the parent topology and byte
  content the attacker supplies — so it closes the identity-launder axis and leaves fabricated-topology and
  paraphrase open. It's the right signal; it's partially closed, partially open, and I'm telling you which is
  which."*

**What you want from researchers:**
> *"Is there a tractable canonical form — or a dominating sub-invariant — for isomorphism-invariance on a
> coalitional, content-bearing value measure? And is there a **semantic** derivation-witness (not a byte
> witness) that could be made deterministic enough to live on a consensus path? Those two would close the
> class. Both are real open theory problems."*

---

## EDGE 3 — Does the airgap close without smuggling in a trusted oracle?

**The attack (steelmanned — Boardy's sharpest framing):**
Your whole pitch is that you dissolve the chain-vs-reality airgap by making honesty load-bearing, without an
oracle. But "another mind built on this" and "this contribution is valuable" are *adjudications*, not facts
the chain observes. The learned model that's supposed to make that judgment un-gameably returned **null** on
real data. So either you smuggle in a trusted oracle to make the value call, or the airgap doesn't actually
close. Which is it?

**Your honest position (the clean separation — this is the answer that holds):**
- *"First, the part that closes with **no oracle at all**: the structural relabeling classes — padding,
  near-duplicates, Sybil splits, cyclic rings, self-report rings — are closed by construction and tested. No
  human, no oracle, no off-chain judge. That half of the airgap is dissolved endogenously, today."*
- *"Second, the honest part: the **semantic** judgment — is this paraphrase the same idea, is this
  contribution actually valuable — cannot be made replica-deterministic, so it **can't sit on-chain** as a
  consensus rule. It lands in a learned model that shapes the training signal, not in the block rule. And that
  learned model has returned **null three times** on predictive accuracy against a fixed proxy — on
  DeepFunding twice and on a 300,000-crate deep-ancestry graph once. I mark it null-tested, never rounded up."*

**The reframe (two moves that keep this from being fatal — both honest):**
1. *"Predictive accuracy on honest, adversary-free labels is the **wrong instrument** for an
   adversarial-robustness measure. With no attacker in the data, a good proxy and a learned model *should*
   score alike — a null is expected, not damning. The **right** instrument is adversarial: inject a gamed
   coalition and check that the measure refuses to pay it. I built that test — the fixed proxy pays the
   attacker, the floored measure denies it — and it reproduces directionally on real data. Honest scope: that's
   a **constructed** adversarial fixture, not a real adaptive adversary. So the right test passes on a fixture;
   the real-adversary version is open."*
2. *"And this is why the null isn't a safety problem: **finality safety doesn't ride on the value measure at
   all.** It rides on excluding reorgeable proof-of-work from finality and on the anti-concentration floor.
   The airgap-closure is a **value-and-franchise** question, not a safety question. So I can afford to leave
   the semantic layer honestly open — the chain is safe while the research question is still live."*

**The straight answer to "which is it":**
> *"Honestly: the airgap closes **structurally and without an oracle** for the relabeling classes, and it does
> **not** yet fully close for the semantic/quality layer — that part needs either a learned judge, which is
> null so far, or a semantic canonicalizer, which can't be deterministic enough for consensus. So the fully
> endogenous, no-oracle closure of the *value judgment* is the **open problem** — and that's the exact thing I
> want a research collective attacking. My vision is a system self-interested in honesty the way a body
> doesn't attack itself. I have the structural half. The semantic half is the frontier."*

**What you want from researchers:**
> *"Can the semantic/quality witness be made endogenous and deterministic enough for consensus — or,
> failing that, can the measure be proven un-gameable **adversarially** (not just predictively) at scale? If
> neither, the moat is a research bet, and I'd rather say that out loud than sell it as done."*

---

## The three research problems, distilled (hand these to Boardy — this is what finds the right people)

1. **Cold-start decentralization:** make the contribution set large and decentralized before traction, with
   no pre-mine that reintroduces a trusted committee. *(mechanism design)*
2. **Isomorphism-invariance for a coalitional value measure:** a tractable canonical form or dominating
   sub-invariant; and a semantic (non-byte) derivation witness deterministic enough for consensus.
   *(graph theory / algorithms / mechanism design)*
3. **Endogenous semantic value:** an un-gameable-at-scale value judgment that closes the airgap without an
   oracle — proven adversarially, not just predictively. *(ML + crypto-economics)*

> All three are honestly open, code-grounded, and load-bearing. That's the difference between "come look at my
> token" and "come solve the hardest problem in the space with me." Lead with the second.

---

### Honesty guardrails for the live call (don't let enthusiasm round these up)
- Structural defense = **built + tested**. Learned value layer = **null 3×, open**. Never blur them.
- Finality safety = anti-concentration floor + PoW-excluded. **Not** the moat. Say this if pressed on any open item.
- I-2 depth-launder = **closed in code** (identity-quotient, tested). A1 (fabricated parent) + A3 (paraphrase) = **open**.
- Cold-start = finalizes on **bonded PoS alone**; anti-plutocracy is a **mature-network** property.
- Reverse-fork / claimable attribution at scale = **designed, not built.**
- If you don't know a number cold: *"let me get you the exact constant"* — never guess a consensus/finality figure.
