# Run a Noesis node (testnet stage)

What works today, and what does not yet. Status words: built, designed, open. See `docs/TESTNET.md` for
where the testnet stands and the reset policy.

## 1. Build

Needs a Rust toolchain (stable). From the repository root:

```
cargo build --release -p noesis --bin noesisd
```

The binary is `target/release/noesisd` (`noesisd.exe` on Windows).

## 2. Run your own node, with its page and wallet

The node serves an HTTP API and its own page from one address. `NOESIS_NET` picks the genesis:
`testnet` or `dev` (the default). Keep the chain log outside the repository.

Linux or macOS:
```
NOESIS_NET=testnet ./target/release/noesisd --serve-api 127.0.0.1:9955 ~/.noesis/testnet-chain.log
```

Windows (cmd):
```
set NOESIS_NET=testnet
target\release\noesisd.exe --serve-api 127.0.0.1:9955 %USERPROFILE%\.noesis\testnet-chain.log
```

Open http://127.0.0.1:9955/ in a browser. From the page you can:
- generate a wallet: a post-quantum keypair with 256 one-time keys, kept in that browser;
- sign and submit a contribution, which the node mines into a block at once;
- see the chain, the contribution graph, and the soulbound identities.

The chain log survives a restart: start the node again with the same file and it resumes the same chain.

## 3. Join another node (one-time sync)

A seed serves its saved chain. A joiner downloads it and re-validates each block before replaying it
(`sync_from`, called from `run_connect` in `node/src/bin/noesisd.rs`):

```
NOESIS_NET=testnet ./target/release/noesisd --listen 0.0.0.0:9944 ~/.noesis/testnet-chain.log   # seed
NOESIS_NET=testnet ./target/release/noesisd --connect <seed-host>:9944                          # joiner
```

Both print a `DIGEST` line. Matching lines mean the two nodes hold the same state.

Both sides must use the same `NOESIS_NET`. If they differ, the joiner starts from a different genesis and
the sync fails. Before commit 23d1d4e3 (2026-11-17) the joiner booted the dev genesis whatever
`NOESIS_NET` said, so a testnet join failed this way; the test
`testnet_joiner_boots_the_testnet_genesis_and_converges` covers it now.

## What does not work yet (open)

- **Live sync.** The seed serves the chain as it was when the seed started, and the joiner syncs once and
  exits. New blocks do not reach other nodes as they land. This is testnet step 7, the critical path.
- **One process for the page and peers.** `--serve-api` and `--listen` are separate modes (step 6).
- **Mining rewards for you.** Each block's JUL reward goes to the first validator in the chain spec, a
  placeholder key on the testnet, not to whoever runs the node. Test JUL is worthless by design, and a
  faucet is step 3.
- **On-chain enforcement.** The five on-VM binding switches are off, so the reference node checks the
  rules and the on-VM scripts do not yet enforce them. Turning them on is the go-live milestone.
- **Wallet safety.** The wallet's secret key and its one-time-key counter live in browser storage. Do not
  clear that storage, and do not restore an old copy of a wallet that has signed since: reusing a
  one-time key weakens it. A safer wallet is being designed.

## Contributions and standing

A contribution earns standing by how much of it is new: text already on the chain scores zero. On the
testnet, the chain's very first contribution earns no standing (`Constitution::genesis_unscored`), so
nobody holds standing from launch.
