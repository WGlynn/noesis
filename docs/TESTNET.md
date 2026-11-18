# Noesis testnet: status, joining, and reset policy

Status words: built, designed, open. Never rounded up. Every protocol number below points at the line of code it comes from.

## Where it stands (2026-11-17)

**Devnet stage, running the testnet genesis.** One node, run by the project on its own machine,
using the testnet chain spec (`ChainSpec::testnet()`, `node/src/chainspec.rs:146`):

- chain id `0x7e57` (`chainspec.rs:170`). The dev chain has a different id (`0xde0`, `chainspec.rs:91`), so the two networks are told apart;
- proof of work enforced in consensus (`pow_enforced: true`, `chainspec.rs:150`), so the testnet tests the real issuance model;
- `genesis_bits` `0x1f00_ffff` (`chainspec.rs:176`), a placeholder that is harder than dev, not a measured mainnet difficulty;
- the identity issuer's authority cell, present from genesis (`chainspec.rs:59`). Identities minted under it get no PoM standing and no finality weight. A test checks this: `identity_mints_carry_zero_pom_and_zero_finality_weight`;
- control binding on (`control_binding: true`, `chainspec.rs:153`): every spend of a cell must carry its owner's signature;
- the block reward paid to the faucet, not to the placeholder validator key (`reward_to_faucet: true`, `chainspec.rs:183`). Each reward is locked by the faucet's own one-time key for that block. Who receives the reward changes no standing and no finality weight (test `the_reward_recipient_and_faucet_payouts_move_no_standing_and_no_finality_weight`).

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
| 7 | Live gossip, so a follower tracks new blocks as they land | open (a joiner replays history once; new blocks do not propagate yet) |
| 8 | Contributors as finality voters | open |

Control binding is on for the testnet, in the reference node: a block that spends a cell without its
owner's signature is invalid, for the producer and for every joiner that re-validates it
(`Constitution::control_binding`, `node/src/runtime.rs:187`; test
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

## Joining today

You can open the node's page (the URL is shared by the maintainer; a temporary tunnel URL changes when it
restarts) and read the chain state. Running your own follower that stays in sync is step 7: not yet.

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
