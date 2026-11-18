# Noesis testnet: status, joining, and reset policy

Status words: built, designed, open. Never rounded up. Every protocol number below points at the line of code it comes from.

## Where it stands (2026-11-17)

**Devnet stage, running the testnet genesis.** One node, run by the project on its own machine,
using the testnet chain spec (`ChainSpec::testnet()`, `node/src/chainspec.rs:132`):

- chain id `0x7e57` (`chainspec.rs:148`). The dev chain has a different id (`0xde0`, `chainspec.rs:84`), so the two networks are told apart;
- proof of work enforced in consensus (`pow_enforced: true`, `chainspec.rs:133`), so the testnet tests the real issuance model;
- `genesis_bits` `0x1f00_ffff` (`chainspec.rs:154`), a placeholder that is harder than dev, not a measured mainnet difficulty;
- the identity issuer's authority cell, present from genesis (`chainspec.rs:48-51`). Identities minted under it get no PoM standing and no finality weight. A test checks this: `identity_mints_carry_zero_pom_and_zero_finality_weight`.

Test JUL is worthless by construction: it mints from trivial CPU work, on a chain of its own.

A testnet is a network other people join. This is not one yet. What is missing, easiest first:

| # | Step | Status |
|---|------|--------|
| 1 | This page: status and reset policy | built (this file) |
| 2 | A guide to run and join a node (`docs/RUN-A-NODE.md`); a tagged release | guide built; release held until sharing |
| 3 | A faucet, so newcomers hold test JUL | open (designed: app layer, a funded wallet) |
| 4 | An explorer for blocks, cells and identities | built, basic: `GET /blocks` and the page show the newest blocks, the contribution graph and identities |
| 5 | A stable public address, TLS and connection caps | open (a temporary tunnel URL works today) |
| 6 | One process serving the wallet and peers | open (`--listen` already serves the durable log) |
| 7 | Live gossip, so a follower tracks new blocks as they land | open (a joiner replays history once; new blocks do not propagate yet) |
| 8 | Contributors as finality voters | open |

Separately, the five on-VM binding switches (`CONTROL_BINDING_ACTIVE` and four in `onchain/*`) are all off.
Turning them on is the go-live milestone and is decided by the maintainer, not by this list. Until then the
on-chain rules are checked by the reference node, not yet enforced by the on-VM scripts.

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
