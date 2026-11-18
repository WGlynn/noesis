# Noesis testnet: status, joining, and reset policy

Status words: built, designed, open. Never rounded up. Every protocol number below points at the line of code it comes from.

## Where it stands (2026-11-17)

**Devnet stage, running the testnet genesis.** One node, run by the project on its own machine,
using the testnet chain spec (`ChainSpec::testnet()`, `node/src/chainspec.rs:164`):

- chain id `0x7e57` (`chainspec.rs:172`). The dev chain has a different id (`0xde0`, `chainspec.rs:92`). Every block carries its chain id and the rulebook refuses another network's block, so a block or a chain log of one network is refused by the other (since 2026-11-18; before, blocks named no chain and a dev log loaded on the testnet genesis);
- proof of work enforced in consensus (`pow_enforced: true`, `chainspec.rs:168`), so the testnet tests the real issuance model;
- difficulty `0x1f00_ffff` (`pow_bits`, `chainspec.rs:177`), which every block's seal must claim (no retarget, so every height uses it; since 2026-11-18, before which a seal was checked only against the bits it named). About 1 hash in 65,536 meets it: a placeholder that is harder than dev, not a measured mainnet difficulty;
- the identity issuer's authority cell, present from genesis (`chainspec.rs:59`). Identities minted under it get no PoM standing and no finality weight. A test checks this: `identity_mints_carry_zero_pom_and_zero_finality_weight`;
- control binding on (`control_binding: true`, `chainspec.rs:171`): every spend of a cell must carry its owner's signature;
- the block reward paid to the faucet, not to the placeholder validator key (`reward_to_faucet: true`, `chainspec.rs:201`). Each reward is locked by the faucet's own one-time key for that block. Who receives the reward changes no standing and no finality weight (test `the_reward_recipient_and_faucet_payouts_move_no_standing_and_no_finality_weight`).

Test JUL is worthless by construction: it mints from trivial CPU work, on a chain of its own.

A testnet is a network other people join. This is not one yet. What is missing, easiest first:

| # | Step | Status |
|---|------|--------|
| 1 | This page: status and reset policy | built (this file) |
| 2 | A guide to run and join a node (`docs/RUN-A-NODE.md`); a tagged release | guide built; release held until sharing |
| 3 | A faucet, so newcomers hold test JUL | built, as designed (app layer, a funded wallet): `POST /faucet`, 10 test JUL per address per day, see below. The running node gets it when it restarts with the faucet seed |
| 4 | An explorer for blocks, cells and identities | built, basic: `GET /blocks` and the page show the newest blocks, the contribution graph and identities |
| 5 | A stable public address, TLS and connection caps | open (a temporary tunnel URL works today) |
| 6 | One process serving the wallet and peers | built: `--serve-api <http> <store> <peers>`, a one-time sync per joiner, read fresh for each |
| 7 | Live sync, so a follower tracks new blocks as they land | built, one producer: `noesisd --follow <producer-url> <http> <store>` pulls `GET /blocks?since=N` over HTTP, re-validates every block, and stops on a rewrite (see below). Tested on local processes; not yet run against the public tunnel |
| 8 | Contributors as finality voters | open |

Control binding is on for the testnet, in the reference node: a block that spends a cell without its
owner's signature is invalid, for the producer and for every joiner that re-validates it
(`Constitution::control_binding`, `node/src/runtime.rs:197`; test
`testnet_refuses_an_unsigned_spend_that_dev_accepts`). It is off on the dev chain. The four on-VM binding
switches in `onchain/*` are still off. Turning them on is the go-live milestone and is decided by the
maintainer, not by this list. Until then the on-chain rules are checked by the reference node, not yet
enforced by the on-VM scripts.

## The faucet (step 3)

Each block reward goes to the faucet. A newcomer asks for test JUL by address:

```
curl -X POST http://<node>/faucet -d '{"address":"<64 hex characters>"}'
curl http://<node>/faucet                      # what it holds, the drip, the limits
curl http://<node>/balance/<64 hex characters> # JUL held under that address
```

- 10 test JUL per payout, one payout per address per 24 hours, 30 payouts per hour across all addresses,
  8 waiting at once (`node/src/faucet.rs:58-66`). The limits live in the node's memory and reset when it
  restarts. A refusal for a limit is HTTP 429; an empty faucet is 503.
- A payout lands with the next contribution block, like every token movement.
- The address is the 32-byte lock of the payout cell. Only the holder of the one-time key whose Lamport
  root is that address can spend it. The web wallet has no such key yet (open), so JUL paid to a wallet
  address can be held and seen but not moved. No route moves anyone's JUL yet either: `POST /token-tx`
  takes soulbound identity txs only.
- The faucet's key: a seed file kept outside the repository, read once when the node starts
  (`~/.noesis/noesis-faucet-testnet.seed`, or the path in `NOESIS_FAUCET_SEED`). A testnet node refuses to
  start without it, so the reward never falls back to the placeholder. Each faucet cell has its own
  one-time key derived from that seed, because a one-time key that signs two different messages can be
  forged. For the same reason a reset needs a new faucet seed: the keys derive from the seed and the
  block height, so the old seed on a new chain would sign a second message with each key.

## Live sync (step 7)

One producer, for now (decided 2026-11-17): the project's node makes every block of this chain, and a node
that joins it is a follower (a replica). Leader election and several producers are not built and not planned
for this stage.

- A follower asks the producer for `GET /blocks?since=<its height>` on the producer's one public port, so it
  works through the tunnel. A page holds at most 32 full blocks and about 1 MiB of block data
  (`node/src/follow.rs:48-52`), and says whether more are waiting. An up-to-date follower asks again every
  2 seconds (`follow.rs:55`), backing off to 30 seconds while the producer cannot be reached.
- It runs every block through the producer's own acceptance path (`ChainSpec::accept_block`,
  `node/src/chainspec.rs:282`): the block's chain id, the proof-of-work seal at the testnet's own difficulty,
  token conservation, every spend's signature (control binding), the finality gate. Standing on a follower is whatever its own
  replay of the blocks gives; it is not taken on the producer's word.
- It stops, keeps its chain, and says why (on its console and in `GET /state` under `replica.stopped`) when
  the producer serves a block it refuses, when the producer's chain is shorter than its own, when the chain
  id on the page differs (each block's own chain id is checked as well, by the rulebook), or when the producer's history differs from its own (a running hash over every block from
  genesis, compared before each page). It never switches to the other history by itself.
- A follower that restarts replays its own chain log and asks only for the blocks after it.
- Submitting a contribution or asking the faucet at a follower returns HTTP 409 with the producer's URL. The
  page sends the same signed contribution to the producer once.
- A follower needs no faucet seed: the chain fixes the reward amount and never checks who receives it, so a
  follower checks a block that pays the faucet without the faucet's key.
- Only finalized blocks are served, so a faucet payout's signatures do not leave the producer before the
  block that spends its cells is final.

What a follower cannot check: who made the chain (blocks carry no producer signature, so it trusts the URL,
and TLS when the URL is https); who signed each contribution (the producer checks the wallet signature when
the contribution arrives, and the block does not carry it). If the producer stops, the chain stops.

## Joining today

You can open the node's page (the URL is shared by the maintainer; a temporary tunnel URL changes when it
restarts) and read the chain state. You can also run a follower that stays in sync (`docs/RUN-A-NODE.md`,
section 4). When the producer's tunnel URL changes, restart the follower with the new URL and the same chain
log; it carries on from where it was.

## Reset policy

A reset starts a new genesis and discards the chain. The rules:

1. **Before anyone outside the project joins,** the chain may be reset whenever a fix needs a new genesis.
   Each reset is still announced in the repository, with a line in `ROADMAP.md` saying when and why.
2. **After outside joiners,** a reset happens only for a consensus-breaking fix or a genesis change, and is
   announced at least 7 days ahead, with the reason.
3. **The old chain is kept.** Its chain log is archived and published, so anything that happened on it stays
   checkable.
4. **What carries over:** soulbound identities are re-minted from the issuer's records, with the same serials
   and holders. Contributions can be resubmitted by their authors. Test JUL does not carry over: it is
   worthless by design.
5. **No silent edits.** A genesis never changes without a reset, and a reset never happens without the steps
   above.
