# Noesis — architecture reference (canonical, code-grounded)

> **Why this file exists.** The load-bearing facts about Noesis's consensus, finality,
> tokens, and value measure are easy to misremember (the *overall consensus mix* vs the
> *finality mix* is a known trap — they are different). This is the single place that
> states them with a pointer to the **constant in code** that defines each one. Treat the
> code pointer as ground truth.
>
> **DISCIPLINE (load-bearing):** never assert a Noesis protocol number — consensus mix,
> finality weights, thresholds, anti-concentration floor, MIN_STAKE, tokenomics — from
> memory. Open the cited `file:line` and read the constant first. Numbers below are stamped
> from code on 2026-06-29; verify before quoting in any partner-facing artifact.

---

## Repo layout

- `node/src/lib.rs` — the reference node: novelty/PoM (`temporal_novelty`, `pom_scores`),
  consensus (`mod consensus`), the value stack (`mod value`: `value_v5..v8`), dispute/slash,
  outcome model, attribution/collusion detectors. Large; grep by symbol.
- `node/src/runtime.rs` — `Node`/`Ledger`/`Block`, the `Constitution`, token txs
  (`TokenTx`, `mod tokens`), the **finality gadget** (`mod finality`), lock-sig verifier.
- `node/src/tokens.rs` — ERC-analog conservation (Fungible/Nft/Multi) and the soulbound identity
  rule (`tokens::soulbound`, 2026-11-17).
- `node/src/identity_mint.rs` + `node/src/bin/identity_mint.rs` — the soulbound issuer's mint tool
  (off-consensus; it reads the issuer's seed file, which the node and the bot never need; the dev
  seed alone is public by design).
- `onchain/noesis-core` — `no_std` verify-side core shared with the on-VM type-scripts
  (Q32.32 mirrors: `finalizes_pos_pom_fixed`, `lamport`, `tx`). Single-sourced with the node.
- `onchain/{pom,finalization,commit-order,locksig}-typescript` — RISC-V CKB-VM scripts
  (excluded from the workspace; built standalone for `riscv64imac`).
- `SECURITY.md` — the attack-class defense matrix (gameability / DoS / double-spend /
  rollback) with honest built-vs-designed status. **Read before any security claim.**
- `ROADMAP.md` — adversarial-loop log (newest first); `internal/CONTINUE.md` — handoff.
- `docs/` — canonical longform: `TOKENOMICS.md`, `POM-CONSENSUS.md`,
  `POM-FINALITY-TEMPORALITY.md`, `RESOURCE-DOS-BOUNDING.md`, `ISOMORPHISM-INVARIANCE-VS.md`,
  `ON-VM-FINALIZATION.md`, the whitepaper, accessible tier.

## Consensus vs. Finality — THE distinction (do not conflate)

Two different weightings. Quoting one for the other is the 2026-06-29 hallucination.

- **Overall consensus mix (NCI):** `pow 0.10 / pos 0.30 / pom 0.60`.
  → `consensus::NCI` at `node/src/lib.rs:4052`. Supermajority `TWO_THIRDS_BPS = 6667`
  (`lib.rs:4054`). This is the *whole-system* weight (production + ordering + finality inputs).
- **Finality mix — PoW EXCLUDED:** `pow 0.0 / pos 1/3 / pom 2/3`, renormalized so PoS+PoM
  sum to 1. → `finality::FINALITY_MIX` at `node/src/runtime.rs:1725`.
  - **Why PoW is out of finality:** PoW is probabilistic and reorgeable, so putting it on
    the *safety* path would itself be the hazard. PoW secures block production, ordering, and
    Sybil-cost (and is the money layer, JUL) — not safety. Finality routes through
    `finality::finalizes_pos_pom` (`runtime.rs:1791`).
  - **2/3 bar is over the PoS+PoM fast-final set**, not a global total.
  - **Each side is measured as a share of its own total before mixing** (2026-10-21,
    `finality::side_shares`, `runtime.rs:1758`; on-VM mirror `finalizes_pos_pom_fixed` in
    `onchain/noesis-core`). So capital holds exactly 1/3 of the finality vote once any contribution
    weight exists, and all of it at genesis when the contribution side is empty
    (`finality::capital_share`, `runtime.rs:1773`), at any denomination of stake or standing. With
    both sides present and no decay, a checkpoint finalizes when `a + 2b` clears `3 x 0.6667` and
    `a, b >= 1/2`, where `a` and `b` are the voting fractions of stake and of standing.
    Note (2026-10-21): before this, the mix weighed raw stake against raw novelty counts, so the
    balance between the sides depended on their units. With the test numbers (stake 1000.0 per genesis
    validator, standing in novelty counts) contribution swamps stake as data accumulates and capital's
    share of the 2/3 bar runs toward 0; with stake counted in a smallest unit it would flip and capital
    would own the bar. The per-side floor never depended on units (rescaling a side multiplies both
    terms of its comparison by the same factor). Found while answering a reviewer's (Boardy's) request
    to publish capital's share as a falsifier.
  - **Cost, stated plainly:** shares make the genesis handoff a step, not a slope. The first vested
    contribution weight, however small, carries the full 2/3 at once, so whoever holds a tiny early
    contribution side plus half of the stake can finalize (with raw units and stake dominant, that took
    about 2/3 of the stake). The per-side floor already gave a tiny contribution side its veto before
    this change. The intended fix is a ramp that phases the contribution side's weight in as it grows,
    keyed to something costly to fake (lineage diversity, not a date that can be front-run or a head
    count that extra accounts can pad). It is not designed yet.
- **Anti-concentration floor:** each dimension (PoS and PoM) must *independently* supply
  ≥ **50%** of its own dimension's total to finalize. → `MIN_DIM_BPS = 5000` (`runtime.rs:1737`),
  `dim_ok` (`runtime.rs:1739`). This is what makes neither axis sufficient alone:
  - a gamed PoM (the 2/3 share) cannot finalize without the PoS dimension also clearing 50%;
  - capital holding *all* stake controls only the PoS dimension, and the unbuyable PoM
    dimension must independently clear 50% — **capital cannot finalize without contribution's
    consent.** This is the anti-plutocracy property; it is a per-dimension floor, not a cap.
- **WEIGHT ≠ EFFECTIVE POWER (do not conflate).** `60/30/10` (and finality's `1/3:2/3`) is how much
  each axis *weighs* in the combined vote (`base_weight`/`effective_weight`, `lib.rs:4070/4084`). It is
  NOT how much each axis *rules*: the anti-concentration floor + AND-composition make NCI a
  **separation-of-powers** (rock-paper-scissors), where each axis is *independently necessary* and none
  finalizes alone (`docs/CONSENSUS-REVIEW.md`, `docs/COHERENCE-LAWS.md`: "no dimension finalizes alone").
  So effective finalization power is far more balanced than the headline weights imply. `60/30/10` is the
  **voting-weight mix, not a reward distribution** — there is no separate `60/30/10` emission/reward
  constant; rewards come from PoM-standing-via-`v(S)`, JUL-via-PoW, and PoS returns.
- `MIN_STAKE = 100.0` bounds validator Sybils (`lib.rs:4160`).
- **Finality liveness under withholding: missed-finality-vote decay** (built in the reference, inert by
  default, not on-VM; pointers re-stamped 2026-10-21). A group holding more than a third of the combined
  weight that stays online and withholds stalls finality on the 2/3 bar; block production continues
  through any finality stall. The decay (`finality::VoteLeak`, `finalizes_pos_pom_leaking`,
  `runtime.rs:1921/1990`; `Node::checkpoint_round`, `runtime.rs:1153`; shares are taken against the
  unleaked totals, then leaked) runs only during a finality
  stall, decays the 2/3-bar weight of whoever does not vote for this chain's checkpoint (never stake or
  credit), and keeps the quorum floor on the registered base. Ruling (Will 2026-10-20, provisional
  pending peer feedback): it NEVER reaches the per-dimension floor, so a refusal by more than half of
  either dimension keeps its veto (the protocol can't tell an honest veto from a captor's stall); that
  case is answered by the social fork. Off while `Constitution.leak_horizon == 0` (`runtime.rs:177`).
  Design, partition risk, and the fork-as-exit remedy ladder: `SECURITY.md` section 5.
- **DECISION — PoM stays coupled to finality (ruled by Will, 2026-06-29; security-expert call).** The
  built design stands: PoM is load-bearing in finality, bounded by the anti-concentration floor.
  Rationale: full-decouple (PoS-only safety) would forfeit anti-plutocracy, which is the thesis. The
  floor converts the welding risk from "gaming OR capital" into "gaming AND capital" (a conjunction).
  - **The two real protections are the anti-concentration floor + the un-gameability moat — nothing else.**
  - **Standing-slash is NOT a finality-safety property (rejected as circular):** slashing gamed PoM
    standing is slashing collateral the attacker gamed into existence; and an honest staker who co-signs
    a gamed checkpoint committed no *objective* fault (unlike equivocation), so can't be fairly slashed.
    Slashing deters equivocation, not the gaming case. Do not claim accountable-safety-via-standing-slash.
  - Forward: keep PoW excluded from finality (done); consider whether `MIN_DIM_BPS` (50%) is conservative
    enough for the safety path specifically; the residual (severe undetected gaming + cleared floor) is
    exactly what the moat closes — coupling is the bet that the moat wins.

## Three tokens (orthogonal roles) — see `docs/TOKENOMICS.md`

| Role | Token | Axis | Status |
|---|---|---|---|
| Contribution / consensus franchise (soulbound standing) | PoM standing | PoM (60% overall, 2/3 of finality) | ✅ built (reference) |
| Bonded capital / stake | CKB-native (state-rent) | PoS (30% / 1/3 of finality) | ✅ built (reference) |
| Money / medium of exchange (energy-pegged, Ergon-style) | **JUL** | PoW (10%, excluded from finality) | issuance + coinbase built (reference; mints on the reference node, worthless at its test difficulty; on the testnet the block reward funds the faucet, see below); stability reserve built but off and unwired; economics unset (`docs/DESIGN-jul-money-layer.md` section 4) |
| Governance (separate instrument, orthogonal to the capture cycle) | **VIBE** | — | per TOKENOMICS.md |

PoM standing is **soulbound** (the `type_script.args` contributor identity, never reassigned
on transfer); the transferable byte (`lock.args`) carries ownership. Consensus franchise rides
the soulbound side, never the transferable byte.

## Soulbound identities: token cells with zero standing (built 2026-11-17, reference node)

Not PoM standing, and not a fourth token in the table above: an application credential (first
collection `vibeswap-identity-v1`, `node/src/identity_mint.rs:33`, the Vibeswap chat's opt-in
identity with a member tier). Standard `TokenStandard::Soulbound` (`node/src/runtime.rs:355`; wire,
digest and on-VM tag 3), rule `tokens::soulbound` (`node/src/tokens.rs:202`, `valid` at `:330`). One
cell per serial in `Ledger.token_cells`; `data` = version, serial, tier (identity / member /
revoked), the holder's 32-byte address. The issuer holds every cell. The type rule lets a tx mint,
re-tier or burn only if it spends the issuer's authority cell, and never changes a live cell's holder
(ERC-5484 "IssuerOnly" in cell form). Serial uniqueness is checked against live cells only, so the
issuer could burn a serial and later mint it again for another address; the mint tool has no burn
action. Who can spend that authority cell is a signature question,
which consensus checks only once control binding is on (see the honest floor below).

- **Zero PoM standing and zero finality weight, from the code.** Token outputs are appended to
  `token_cells` only (`runtime.rs:1551`); PoM is recomputed over `state.cells` only (`runtime.rs:1629`,
  keyed on `type_script.args`, `lib.rs:329`); `finality_pom_weight` filters `ledger.cells`
  (`runtime.rs:1232-1239`); PoS weight comes from the chain spec's bonded validators, not from cells.
  The only outside path into `cells` is `POST /submit`; identities enter through `POST /token-tx`,
  which takes soulbound txs only. Confinement (`runtime.rs:401`): a soulbound cell moves only in a
  soulbound tx of its own collection, and a soulbound tx carries nothing else. The test is
  `identity_mints_carry_zero_pom_and_zero_finality_weight` (`runtime.rs:3548`): six blocks with and
  without mints (holders equal to the contributor's own key, vesting W = 2) give identical cells, index
  root, PoM, work clock, `finality_pom_weight()` and finality decision; folding soulbound outputs into
  `cells` breaks it. That is a test over the reference node, not a proof.
- **Issuer authority is a linear cell, so minting is not one-shot.** Fungible/Nft take mint authority
  from an input whose `lock.args` equals the token args (`runtime.rs:422`). Only a 32-byte one-time
  Lamport root can sign, so the authority must come back under the same key and a second mint reuses
  it; from two such signatures a third party assembles a valid signature over a digest the issuer
  never signed (`one_mint_limit_of_the_lock_args_rule_is_real`, `runtime.rs:3632`). Still true for
  Fungible/Nft (open). Soulbound authority works differently. Each collection has exactly one
  authority cell. Genesis creates it (`chainspec.rs:45`, `:213`); no tx can create another. Every spend
  must re-create it with its generation number one higher, under any lock. So the tool locks
  generation `g` with a key derived from (issuer seed, g) (`identity_mint.rs:56`), and each key signs
  once. Identity cells get their own per-(g, serial) keys (`:61`).
  Test: `soulbound_key_chain_mints_two_and_three_times_without_reusing_a_key` (`runtime.rs:3715`).
- **Mint path.** A batch file (the bot's format) -> `identity-mint mint` reads `GET /tokens`
  (`rpc.rs:912`) -> one tx, signed -> `POST /token-tx` (`rpc.rs:931`; node-local policy: soulbound
  only, every input signed and verified, valid with all pending txs, queue of 16, `rpc.rs:94`) -> the
  next contribution block carries it (`chainspec.rs:236`). A token tx never makes a block alone
  (`EmptyBlock`, `runtime.rs:1381`, unchanged). Per tx at most 64 mints and 2 re-tiers
  (`identity_mint.rs:42-43`) so the signed body fits `MAX_BODY` = 160 KiB (`rpc.rs:52`). The dev spec's
  authority uses a public dev seed; the testnet's root is `TESTNET_IDENTITY_AUTHORITY_ROOT`
  (`chainspec.rs:59`), its seed kept outside the repo. On-VM, the lock script maps the code hash to
  tag 3 (`onchain/locksig-typescript/src/main.rs:60-72`; `node/tests/ckb_vm_locksig.rs`).
- **Honest floor.** Spend signatures are checked in the reference node's consensus on the testnet
  only: control binding (`Constitution::control_binding`, `runtime.rs:197`) is on in
  `ChainSpec::testnet()` (`chainspec.rs:171`) and off on dev. On the testnet a block that spends the
  authority or re-tiers an identity without the owner's signature is invalid for the producer and for
  every joiner or follower that re-validates (`testnet_refuses_an_unsigned_spend_that_dev_accepts`,
  `chainspec.rs:419`); on dev an empty signature still spends. The on-VM lock script's own switch
  (`CONTROL_ENFORCED`) is off, so this is the reference node's check, not yet the on-VM scripts'.
  The off-chain registry stays the authority. The holder address is pasted, not proven; the cell grants nothing on
  chain. Pending txs and inclusion records are node-local memory. On a quiet chain a mint waits for a
  contribution.

## Testnet faucet and control binding (built 2026-11-17, reference node)

Testnet step 3 (`docs/TESTNET.md`), built as designed there: app layer, a funded wallet.

- **Control binding is a chain setting.** `Constitution::control_binding` (`runtime.rs:197`) replaced the
  hard-coded `CONTROL_BINDING_ACTIVE = false`. It reaches `TokenTx::spend_is_authorized`
  (`runtime.rs:604`) through `is_valid_in_ledger` (`:513`) for token-tx inputs and through `check_bonds`
  (`:1421`) for Bound-B deposits, and the sub-block tier gets it too. With it on, an empty `auth` no
  longer authorizes a spend; a presented one is verified the same either way. Contribution cells and the
  coinbase are not spends, so `POST /submit` is unchanged
  (`a_v2_submit_lands_on_the_testnet_with_control_binding_on_and_pays_the_faucet`, `rpc.rs:1424`, on the
  real testnet spec). On in `ChainSpec::testnet()` only.
- **The reward funds the faucet.** `ChainSpec::reward_to_faucet` (`chainspec.rs:52`, on for the testnet)
  makes the producing node pay each block reward to the faucet (`produce_block_paying`,
  `chainspec.rs:248`; `rpc.rs:855`). Consensus fixes the amount and never checks the recipient. A chain
  that pays the faucet produces nothing without the faucet seed: `--serve-api` refuses to start
  (`rpc.rs:1118`) and an in-memory node refuses the block (`rpc.rs:793`). A replica (`--follow`, below)
  produces nothing and needs no seed: it validates a reward paid to the faucet without the faucet's key.
- **A key per cell.** A lock is a Lamport one-time key, so one fixed faucet address would sign every payout
  with one key. Each reward is locked by its own key derived from (faucet seed, height)
  (`faucet.rs:83`); the change goes to a purse whose key is derived from (seed, generation) (`:88`), and
  each payout spends purse `g` and writes `g + 1`. A pending payout's signatures leave the node only in
  the finalized block that spends its cells, so the faucet re-signs it freely until then.
- **Route and limits.** `POST /faucet {address}` (`rpc.rs:976`), `GET /faucet`, `GET /balance/<address>`.
  10 JUL per payout (`DRIP`, `faucet.rs:58`), one per address per 24 hours (`:60`), 30 per hour overall
  (`:62-64`), 8 waiting per block (`:66`), node-local and reset by a restart. A payout lands with the
  next contribution block.
- **Zero standing, zero finality weight.** JUL cells sit in `token_cells`, which PoM and finality never
  read. Test: `the_reward_recipient_and_faucet_payouts_move_no_standing_and_no_finality_weight`
  (`faucet.rs:492`): two replicas, rewards to the placeholder vs to the faucet plus payouts (one to the
  contributor's own key bytes), with vesting W = 0 and W > 0, give identical cells, index root, PoM,
  work clock, finality weight and finality decision; folding JUL into `pom` breaks it (checked).
- **Honest floor.** The seed is a plaintext file outside the repo (`~/.noesis/noesis-faucet-testnet.seed`
  or `NOESIS_FAUCET_SEED`, `faucet.rs:116`), like the identity issuer's; encrypting it is open. Anyone can
  name any 32 bytes as the address; only the holder of the one-time key whose Lamport root is those bytes
  can spend the payout, and the web wallet has no such key yet, so wallet holders can hold and see test
  JUL but not move it. The limits are per address and per window, so many addresses can still take the
  window's 30 payouts. Testnet JUL is worthless by design.

## Live sync: one producer, followers (built 2026-11-17, reference node)

Testnet step 7, topology A of `docs/DESIGN-live-gossip.md` (Will 2026-11-17: "one producer, for now").

- **Roles.** The producer is `noesisd --serve-api`. A follower is `noesisd --follow <producer-url> <http>
  <store>` (`rpc.rs:1103`, loop `follow.rs:221`): its own page and read API over its own durable store. It
  produces nothing: `POST /submit`, `/faucet` and `/token-tx` answer 409 with the producer's URL
  (`rpc.rs:617`), and the page re-sends the same signed body there once.
- **Route.** `GET /blocks?since=N[&limit=M]` (`rpc.rs:915`, `blocks_page` `:518`): the full finalized blocks
  after height N, oldest first, at most 32 a page (`PAGE_MAX_BLOCKS`, `follow.rs:48`) and about 1 MiB of
  block bytes (`PAGE_MAX_BYTES`, `:52`; at least one block), with `tip`, `more`, and the chain link at N as
  `anchor`. Blocks are read back from the store by frame offset (`store.rs:59`, `:131`). Only blocks in the
  durable log are served (written and flushed to disk first, see the chain log bullet below), and a
  block is appended only after it finalized, so a pending faucet payout's
  signatures do not leave the producer early (`blocks_since_serves_only_finalized_blocks_and_never_a_pending_payout`,
  `rpc.rs:2175`). A follower polls every 2 s (`POLL`, `follow.rs:55`), backing off to 30 s on errors; no long-poll.
- **Validation.** Every pulled block goes through `ChainSpec::accept_block` (`chainspec.rs:282`), the
  producer's own path: `validate_block` (the block's chain id, the PoW seal at the chain's own bits, token
  conservation, spend signatures under control binding; see the next section), the finality gate, then
  apply. A refused block stops following and the follower keeps its
  chain (`a_follower_refuses_a_tampered_block_and_keeps_its_chain`, `rpc.rs:1974`;
  `a_follower_refuses_a_payout_whose_signature_was_tampered`, `:2016`). Standing on a follower is what its
  own apply derives from the blocks.
- **Rewrite detection.** `chain_link` (`follow.rs:66`) chains every block's header digest from genesis
  (`header_digest` binds the chain id, cells, coords, token txs with their signatures, coinbase, seal and
  timestamp; not the Bound-B bonds, which must be empty while `submission_deposit` is 0, as on the testnet).
  Before applying a page (`apply_page`, `:102`) the follower compares the producer's link at its own height,
  and stops if they differ, if the producer's height is below its own, or if the page's chain id differs
  (the producer's word, an early stop; each block's own chain id is the rulebook's check, next section);
  `GET /state` then says why (`replica.stopped`). Tests: `a_follower_stops_on_a_rewritten_history_even_one_that_keeps_the_later_blocks`
  (`rpc.rs:2069`; blocks name no parent hash, so the tip's digest alone would miss that rewrite) and
  `a_follower_stops_on_a_producer_behind_it_and_on_another_network` (`:2101`). The link is node-local, not consensus.
- **Restart.** A follower replays its own store and asks only for what it lacks
  (`a_follower_resumes_from_its_own_store_after_a_restart`, `rpc.rs:2044`; two real processes in
  `node/tests/follow_live.rs`).
- **The chain log across a crash (built 2026-11-18, producer and follower alike).** A block's record goes
  out in one write call and is flushed to disk (`sync_all`) before the append returns
  (`append_block_at`, `store.rs:59`); only then is it served by `GET /blocks` or reported final by
  `POST /submit` (`persist`, `rpc.rs:484`). If a write fails, the log is cut back to its last whole record,
  the block is neither served nor reported final (503), and the node takes no more blocks until it is
  restarted, since a later block written without it would leave a gap no boot can replay
  (`a_block_the_log_cannot_take_is_not_reported_final_or_served_and_production_stops`, `rpc.rs:2435`).
  At boot (`load_for_append`, `store.rs:142`, called by `with_store`, `rpc.rs:341`): if the bytes after
  the last good record are what a crash mid-write leaves (part of one record with no whole block record
  among them, or all zeros, `torn_tail`, `store.rs:281`), they are copied to
  `<log name>.torn-at-<byte offset>-<unix seconds>` next to the log (never deleted), the log is cut back
  to the last good record, both are flushed, one line on stderr says so, and only then does the node
  append. Before this fix the torn bytes stayed, new blocks went after them, and the next boot stopped at
  the torn record and dropped every later block, so the producer fell below its followers and finalized
  blocks were lost (`a_torn_tail_is_set_aside_at_boot_and_every_later_block_survives`, `rpc.rs:2290`;
  follower, `:2321`; real process, `node/tests/listen_serves_durable_store.rs`). Anything else (good
  records after a bad one, a whole record that is not a block, a block the rules refuse) is damage, not a
  crash: the node refuses to start, says where, and leaves the file as it is (`rpc.rs:2358`). A log of
  another network is neither: the boot stops at its first block with both chain ids named (next section).
  A read-only load (`--listen <store>`, the peers seed) never changes the file, and judges only the bytes it saw when its
  replay began, so a record the writer finishes meanwhile is not mistaken for damage. Honest floor: no power-cut test was run;
  on Windows the directory entry of a newly created log is not flushed (std cannot open a directory
  there); a log an older node already damaged this way (whole blocks after a torn record) now stops the
  boot instead of booting short, and repairing one is by hand.
- **Honest floor.** One producer: if it stops, the chain stops (no failover, no leader election). A follower
  checks every rule the rulebook covers, but trusts the producer URL (and TLS, for https) for who made the
  chain: a block carries no producer signature, and a contribution cell does not carry its wallet signature
  (the producer checks it at `POST /submit`), so a follower cannot re-check who wrote a contribution.
  An https producer is fetched through the system's `curl`, since the node links no TLS library.

## Every block names its chain and claims its chain's difficulty (built 2026-11-18, reference node)

Two block-acceptance gaps, closed before the testnet's first block (it was at height 0 with no chain log,
so the format change cost nothing).

- **Difficulty.** Before, `pow_check` checked a seal only against the bits the seal itself named, and
  nothing compared those bits with the spec's. A producer (or anyone a follower trusted) could serve blocks
  mined at a trivial target, and `block_work`, which pays JUL and advances the work clock, was computed from
  the bits the producer chose. Now the bits are a rulebook value, `Constitution::pow_bits`
  (`runtime.rs:100`), and under `pow_enforced` a seal must claim exactly them before its hash is checked
  against them (`pow_check`, `runtime.rs:978`, the bits check at `:984`; refusal `Violation::PowBitsWrong`). No retarget is wired:
  `noesis_core::pow::next_target` (ASERT) and `liveness::liveness_bits` are pure and called from no
  consensus path. So every height uses one value, dev `0x2000_ffff` (`chainspec.rs:95`, about 1 hash in 256
  meets it) and testnet `0x1f00_ffff` (`chainspec.rs:177`, about 1 in 65,536). Harder bits are refused too:
  the rule is equality, as in Bitcoin, where a block's bits must equal what the schedule requires and a hash
  lower than required is always fine. `Node::new` refuses a PoW-enforced chain whose bits decode to no
  target (`runtime.rs:1063`).
- **Chain id.** Before, `ChainSpec::chain_id` was read only by the follower's page check (`apply_page`, the
  producer's own word) and by messages: blocks did not carry it and validation never read it, so a dev chain
  log replayed onto the testnet genesis, and a block mined for one network was valid on any other with the
  same rules. Now every block carries `chain_id` (`Block::chain_id`, `runtime.rs:667`, stamped by
  `Node::propose`, `runtime.rs:1129`),
  `validate_block` refuses a block whose id is not `Constitution::chain_id` before any other check (`runtime.rs:1371`; `Violation::WrongChain`), with or without
  PoW, and `header_digest` binds the id first (`runtime.rs:882`, domain tag `noesis-pow-hdr-v2`), so a
  block relabeled for another chain no longer matches the digest its seal was mined over. Dev is `0xde0`
  (`chainspec.rs:92`), testnet `0x7e57` (`chainspec.rs:172`).
- **Why in the rulebook, not only the node.** Block validity is the one pure function every replica runs
  (`validate_block`). The producer and a follower reach it through `ChainSpec::accept_block`, but a node
  booting from its chain log, a joiner syncing from a seed (`sync.rs`), the reorg path and `apply_block` call
  `validate_block` directly. A rule kept only in `accept_block` would let a replica that boots from a log
  accept a block that a follower refuses. The bits also set consensus state (`block_work` feeds the coinbase
  amount and `Ledger::work`, which is in the state digest). Both values moved from `ChainSpec` fields into
  the `Constitution`, which the spec builds, so the bits a node mines at (`ChainSpec::genesis_bits()`) and
  the bits it requires are one value.
- **Boot and join.** A chain log of another network stops the boot at its first block with both chain ids
  named, and the file is left as it is (`store.rs:228`); a joiner that meets another network's seed says the
  same (`sync.rs`). A log written before this change names no chain id and its seals were mined over the v1
  digest: it does not decode, and the node refuses to start on it, as for damage. `chain_id` is a required
  wire field (`wire.rs:70`).
- **Tests.** Producer path:
  `a_seal_claiming_easier_bits_than_the_spec_is_refused_even_when_valid_for_them` (`chainspec.rs:496`) and
  `a_block_mined_for_one_chain_is_refused_on_another_with_the_same_rules` (`chainspec.rs:529`). Follower path:
  `a_follower_refuses_a_block_mined_at_easier_bits_than_the_spec` (`rpc.rs:2132`) and
  `a_follower_refuses_blocks_of_another_network_even_when_the_page_claims_its_chain_id` (`rpc.rs:2157`). Boot:
  `a_dev_log_refuses_to_load_as_testnet_naming_both_chain_ids` (`store.rs:455`). Join:
  `a_joiner_on_another_network_refuses_the_seeds_blocks_naming_both_chains` (`node/tests/sync_join.rs`).
  Rulebook: `a_seal_must_claim_the_chains_own_bits`, `a_block_names_its_chain_and_the_header_binds_it`
  (`node/tests/pow_enforcement.rs`). The producer, follower and boot tests were run on the code before the
  fix first and failed there (the easier-bits block and the other network's block were accepted, the dev
  log loaded as testnet); the rulebook and join tests use the new fields and were written with the fix.
- **Honest floor.** Both rules are the reference node's: no on-VM script reads a block header or its seal
  (the header digest is built only in the node; `noesis-core` holds the hash and the compact arithmetic),
  so nothing on-VM changed and nothing on-VM checks either rule. A chain id separates networks only if their
  ids differ: anyone can start a chain from the same spec, and its blocks are valid on ours. The difficulty
  is still a placeholder (cheap blocks, low Sybil cost, expected for a testnet); the measured mainnet value
  and a retarget are open.

## Value measure `v(S)` — the moat (re-stamped from code and the research record, 2026-10-21)

> Read with `docs/research/WASH-PROBLEM-INDEX.md` (every approach, with status) and the recent recaps in
> `sessions/`. This section lagged the September-October research once and misled a reader; when the
> research moves, update it here. Code pointers are the current sketch, not law.

**1. What the sketch computes (built).** In `node/src/lib.rs`: `temporal_novelty` (`:187`) -> `pom_scores`
(`:260`). The value-oracle seam: trait `ValueOracle` (`:293`), the built scorer `NoveltyOracleV0` (`:303`),
the aggregator `pom_scores_with_oracle` (`:314`); `pom_scores_with_similarity_floor_q16` (`:341`) is the live
consensus PoM gate (near-duplicate floor `Constitution.theta_sim_q16`, default 0.95). The value stack is
`mod value` (`:1007`): `value_v5` (`:1189`) gates novelty by realized downstream use through `flow_gate`
(`:1157`), which counts use and so is an upper bound on dependence; `value_v6` (`:1239`) prices identity
through standing-gated seeds; `value_v7` (`:1306`) adds a semantic/entropy floor; `value_v8` (`:1391`) adds
a learned-outcome gate (Bradley-Terry over coalition features; its weights are passed in, and nothing yet
decides when it retrains). Interior anti-gaming: within-identity, cross-identity and joint damping,
`attribution_circulation` (`:373`) and `attribution_cycle_energy` (`:426`, Helmholtz-Hodge) feeding
`collusion_slash` (`:5010`); one invariant behind them in `docs/ISOMORPHISM-INVARIANCE-VS.md`.

**2. The fence: finality reads structural novelty only (built, 2026-10-05).** `Node::finality_pom_weight`
(`node/src/runtime.rs:1198`) reads novelty past the vesting window, not the value stack, and `value_v5..v8`
have no caller in the runtime or the RPC. The test
`standing_and_finality_are_structural_novelty_only_no_value_layer_leak` (`runtime.rs:3270`) fails if the
value layer reaches standing. So nothing that judges worth, including a learned or brain-based judge,
touches what gets finalized. Why split them: if finality and reward read one gameable signal, one wash
attack is a value, governance and consensus exploit at once (`docs/DESIGN-novelty-split-structural-vs-value.md`).

**3. The open problem, stated precisely.** Interior signals are worth-blind. A competent acyclic wash
(distinct keys, novel content, honest topology) is structurally identical to genuine collaboration: 0.0%
separation in `node/examples/wash_sim.rs`. Wash (fake value from real, distinct actors) is not Sybil (fake
identity), so identity proofs don't reach it. Closing it needs two things from outside the graph:
independence that costs something to fake, and a worth test the contributor cannot move. Already handled:
cyclic self-citation is caught by built interior defenses, and so is relabeling on the demonstrated axes
(`node/examples/adaptive_sim.rs`, `g = 0`; paraphrase is the one live content-proxy gap). On real crates.io
data the independence gate's logic zeroes closed self-owned dependency rings (78.8% of reuse crates; they vest 0.00
against 69.36 for genuine ones), so the machinery works given a clean independence signal
(`docs/research/WASH-PROBLEM-INDEX.md`, sections 0 and 6).

**4. The current approach (designed; the measuring part partly measured).** Four parts, from the
September-October 2026 research:

- **Measured transfer instead of declared citation.** Make the graph's dependencies learners (Cadence
  brains): an edge a -> b means b took in a and b's measured behavior changed, so the graph records who
  measurably learned from whom instead of who cited whom. The receiving brain is the instrument and stands in
  for a user: worth is how much a copy fed the contribution improves over its untaught twin (a byte-identical
  snapshot) on a held-out task the contributor doesn't control. Contributions come from anyone; the
  receiving brains hold no standing, cast no votes and vouch for nothing. They are costly to fool (a fake
  that moves an honest task had to carry real task information, so faking collapses into contributing) and
  cheap to copy, which is what makes the twin comparison and many-judge checks affordable. Measured in the
  public lab `WGlynn/cadence-teachers` (ledger `VS-TRANSFER-PROGRAM.md`):
  - On a bandit task: separation from surface-matched fakes (M1); it survives a fake matched on every
    interior feature, additively and for two forms of synergy (M2); collusion among peers buys nothing given
    an honest task pool, by construction, since the measure never reads peer agreement (M4). Isomorphism
    invariance is empirical only (M3).
  - On a richer chain task: transfer prices skill on a brain that takes in content (a know-1..4 ladder,
    ordered, replicated on four disjoint student sets, fakes at most about 4% of pay; EXP-62 to EXP-62e) and
    is blind on a brain that doesn't (M1b). The zero point is the unsettled part.
  - The price depends on three outside ingredients: the task, the instrument (which brain) and the null
    (what "no contribution" means). On every brain tested the order of worth held; the exchange rate between
    kinds of contribution moved with the brain, the zero and how often lessons reached late stages
    (EXP-62b, EXP-62h, EXP-62i).
  - Boundary (M5, `M5-EXOGENOUS-ANCHOR.md`): whoever supplies a majority of the scoring tasks wins (the
    crossover is half the task pool; about a third against an adversary who knows the task, derived, not
    measured). Transfer moves the honest-majority assumption from peers to tasks. That is progress, because a
    task has an answer that can be checked and a peer report doesn't, but it is not an escape.
- **Judges out of the contributor's reach** (`sessions/2026-10-19-judges-out-of-reach.md`). Law below state:
  a cheater can buy state (tokens, an aged account) but not law, so fix everything that judges (model, task,
  baseline, rules) before the work it judges arrives. Adopted 2026-10-21: a frozen judge pins its whole setup
  (library commit, every setting, its zero, its curriculum; `docs/DESIGN-value-oracle-seam.md`). Designed:
  grow each round's judge from settled work and freeze it before the round opens; commit-reveal the test from
  a public random beacon. Not built: no judge-version field in `Constitution`, no retrain schedule, no
  amendment activation point (`node/src/amendment.rs` rejects the class as not built).
- **Layered anchors; "outside" means too costly for this attacker to move**
  (`docs/research/layered-anchors-2026-10-16.md`; designed, not measured). Stack anchors rooted in different
  scarce resources (capital, energy, time) so a ring has to look independent in every layer at once; the
  faking cost multiplies only if the roots can't be bought together. Task labels come from realized use one
  layer up; proof of work anchors independence and bootstrap, not worth; revenue needs an oracle. Brains
  that disagree on prices give a signal, not noise (OPH-style local consistency; Glynn 2026, *Value Disputes
  Are Missing Dimensions*): a fake that fools one judge but not the others should
  leave a price inconsistency (proposed test, not run). Built half: the independence gate's logic; the
  capital-cluster oracle that would feed it is not built. Honest floor: for an attacker with capital, locked
  money, bought energy and bought time largely convert into one another, so the moat is the conversion cost,
  and an aged, funded account buys several layers at once (`docs/REVIEW-2026-10-19-ledger-explainer.md`,
  principle 4).
- **Pay the honest outcome instead of detecting the fake.** Value vests on realized use by capital-independent
  minds, and a carrying cost plus slash-at-risk makes patient wash negative-EV, which prices a wash ring to a
  51%-class residual rather than excluding it (the periphery, `docs/DESIGN-periphery-solution.md`; simulated
  in `node/examples/periphery_sim.rs` under design parameters). Credit has no clock (built for `value_v5`; test
  `credit_has_no_clock_late_use_pays_early_work_the_same`, `lib.rs:1634`). A refundable submission deposit is
  built and inert by default (`submission_deposit`, `check_bonds`); it refunds on novelty today, and the
  designed change is to refund on realized use and feed kept deposits to the judge as failure labels. Rule
  for any such cost (Will, 2026-08-05): it lands on the fakeness, not on entry. If a funded ring can afford
  it and a poor honest newcomer can't, it filters poverty, not fakes.

**5. Scope.** The measured scope starts where the counterfactual can be run: code first, then models,
prompts and datasets, and written know-how an agent consumes under a frozen judge
(`docs/research/sage-validity-layer-and-the-memory-counterfactual.md`). It widens as AI makes
counterfactuals cheaper. Less measurable value earns under stricter terms (larger deposit, longer vesting,
lower cap), never zero (Will, 2026-10-19). Designed, not built: `submission_deposit` and `vesting_w` are
single global settings in `Constitution`.

**6. A learned judge on real labels (open).** A rich-feature judge on 1,965 usable Deep Funding jury pairs
generalizes at about 0.60 on a repo-disjoint split, mostly from popularity; graph-topology features alone
were null at about 0.54 (`data/deepfunding/RESULTS-RICH-JUDGE.md`). A modest real signal, and a live risk of
learning attention instead of dependence. Prediction on honest labels is not adversarial un-gameability.

**Success measure (decided 2026-10-27).** Noesis is judged by calibration: how well its valuations today predict independent use later, the way a planet is judged by homeostasis. Independence is treated as a cost to fake, not a yes-or-no fact. First measured 2026-10-27 on crates.io (experiments C1 and C2). The measurement is an offline script in the ETM repo; no Noesis code computes it. A plain count of independent users forecast next year's independent adoption slightly better than `value_v5`'s damped flow. See `docs/research/calibration-is-homeostasis.md`.

**Outside evidence: SAGE outcome receipts (built 2026-11-01, off-consensus).** A receipt is SAGE's signed record of what one public memory changed on a set of tasks: paired runs with and without it, under a pinned judge. `interop/sage-receipts` checks a receipt the way SAGE's own code does, stricter in four documented places, and writes each valid one as a label. It is tested on 40 vectors generated by SAGE's code. Nothing in `node` depends on it. No real pilot receipts yet, and no written evaluator policy decides on the independence facts a label records. It came from the first outside contribution, issue #2.

**7. Still open.** The capital-cluster independence oracle (designed, not built); an honest task source
(M5); isomorphism invariance as a proof (M3; `WASH-PROBLEM-INDEX.md` row 9); transfer beyond small brains
on toy tasks (no instrument yet takes in ordinary code or prose); and the review's list: genesis prior,
composition, the patience filter, frame underprovision, capital buying variance, survival as lock-in,
counterfactuals that don't climb to roots (`docs/REVIEW-2026-10-19-ledger-explainer.md`). Un-gameability is
claimed for demonstrated vectors only, never as a finished proof.

## Security model — see `SECURITY.md` (read it; don't paraphrase from here)

Five attack classes, honest status: (1) `v(S)` gameability ✅ built (reference+runtime),
isomorphism-invariance open; (2) DoS/spam — economic ✅, **resource-DoS Bound A (mempool cap)
✅, Bound B commit-deposit ✅ (reference layer, inert by default)**; (3) double-spend ✅ reference / 🟡 deploy-coupled crypto;
(4) rollback/finality ✅ reference / 🟡 on-VM mirror; (5) capture of the standing set and fork as exit
(credit vs franchise), with the missed-finality-vote decay (2/3 bar only) built in the reference and inert by default. Structural maxim: *don't let the attacker
choose a security-critical input* (every consensus value re-derived on-VM, rejected if not
reconstructable).

## Status discipline

✅ built & tested · 🟡 designed, not built · 🔬 open problem. Never round designed up to built.
Test count + green status: `node/README.md` (doc-coherence hook stamps it against code).
