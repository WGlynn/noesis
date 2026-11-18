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

On the testnet every block reward goes to the node's faucet, so a testnet node needs a faucet seed and
refuses to start without one: a file of 64 hex characters made from real randomness, kept outside the
repository, at `~/.noesis/noesis-faucet-testnet.seed` (or the path in `NOESIS_FAUCET_SEED`). Make one per
chain (a new one after a reset, since the faucet's one-time keys derive from it) and back it up offline;
anyone who reads it can spend what the faucet holds. For example:

```
python3 -c "import secrets; print(secrets.token_hex(32))" > ~/.noesis/noesis-faucet-testnet.seed
```

(Windows: `python -c "import secrets; print(secrets.token_hex(32))" > %USERPROFILE%\.noesis\noesis-faucet-testnet.seed`.)
The dev chain needs no seed and has no faucet.

Linux or macOS:
```
NOESIS_NET=testnet ./target/release/noesisd --serve-api 127.0.0.1:9955 ~/.noesis/testnet-chain.log
```

Windows (cmd):
```
set NOESIS_NET=testnet
target\release\noesisd.exe --serve-api 127.0.0.1:9955 %USERPROFILE%\.noesis\testnet-chain.log
```

Open http://localhost:9955/ in a browser, that exact address. From the page you can:
- generate a wallet: a post-quantum hot key with 65,536 one-time keys, kept in that browser encrypted
  under a passphrase you choose (a passkey can be added as a second way to unlock), and a cold recovery
  key shown once for you to store offline (recovery itself is designed, not built);
- sign and submit a contribution, which the node mines into a block at once;
- see the chain, the contribution graph, and the soulbound identities.

One origin for the wallet: http://localhost:9955. A browser keeps storage separately for every exact
origin (scheme, host and port) and binds a passkey to the host name, so http://127.0.0.1:9955, another
port, or a tunnel URL has its own store, which does not hold the wallet made at localhost; a quick-tunnel
URL also changes from run to run, and a passkey cannot be made on a bare IP address at all. At any other address the page still shows the chain but will
not create a wallet, sign, or add a passkey, and it says why. The node still binds 127.0.0.1:9955;
`localhost` reaches it.

On the testnet, `POST /faucet {"address": "<64 hex>"}` asks the node's faucet for 10 test JUL; see
`docs/TESTNET.md` (the faucet) for the limits and what the address must be.

The chain log survives a restart: start the node again with the same file and it resumes the same chain.
If the node stopped in the middle of writing a block (a crash or a power cut), the next start moves the
half-written record to a file next to the log, `<log name>.torn-at-<byte offset>-<unix seconds>`, cuts
the log back to its last whole block, and says so in one line. That file is never deleted; keep it or
remove it yourself. If the log is damaged any other way (for example, whole blocks after a broken record),
the node refuses to start, says at which byte, and leaves the file as it is: restore it from a copy, or
start the node on a new chain log and keep the old one. A follower's chain log works the same way.

## 3. Join another node (one-time sync)

A seed serves its saved chain. A joiner downloads it and re-validates each block before replaying it
(`sync_from`, called from `run_connect` in `node/src/bin/noesisd.rs`):

```
NOESIS_NET=testnet ./target/release/noesisd --listen 0.0.0.0:9944 ~/.noesis/testnet-chain.log   # seed
NOESIS_NET=testnet ./target/release/noesisd --connect <seed-host>:9944                          # joiner
```

Both print a `DIGEST` line. Matching lines mean the two nodes hold the same state.

Or run one process for the page AND joiners: add a peers address after the chain log.

```
NOESIS_NET=testnet ./target/release/noesisd --serve-api 127.0.0.1:9955 ~/.noesis/testnet-chain.log 0.0.0.0:9944
```

It prints `PEERS LISTENING <addr>`, and it reads the chain log fresh for each joiner, so a joiner gets
the chain as it stands when it connects.

Both sides must use the same `NOESIS_NET`. If they differ, the joiner starts from a different genesis and
the sync fails. Before commit 23d1d4e3 (2026-11-17) the joiner booted the dev genesis whatever
`NOESIS_NET` said, so a testnet join failed this way; the test
`testnet_joiner_boots_the_testnet_genesis_and_converges` covers it now.

## 4. Follow the producer live (a replica)

A follower keeps its own copy of the chain in step with the producer as blocks land (testnet step 7, built;
see `docs/TESTNET.md`, live sync). It serves its own page and API, and produces no blocks. All three
arguments are required: the producer's URL, the follower's own HTTP address, and the follower's own chain
log (never the producer's).

```
NOESIS_NET=testnet ./target/release/noesisd --follow https://<producer-url> 127.0.0.1:9956 ~/.noesis/follower-chain.log
```

Windows (cmd):
```
set NOESIS_NET=testnet
target\release\noesisd.exe --follow https://<producer-url> 127.0.0.1:9956 %USERPROFILE%\.noesis\follower-chain.log
```

- It prints `FOLLOWING <url> from height <h>`, then a line for each batch of blocks it takes.
- The wallet works at one origin only, http://localhost:9955 (section 2). Port 9956 above lets a follower run
  beside a producer on one machine; its page then shows the chain but will not sign. On a machine that runs
  only a follower, give it `127.0.0.1:9955` to use the wallet there.
- A follower needs no faucet seed, even on the testnet. Submitting or asking the faucet at a follower returns
  HTTP 409 with the producer's URL; the page sends your signed contribution there for you.
- An `https://` producer URL (a tunnel) is fetched with the system's `curl` program (shipped with Windows 10
  and later, macOS and most Linux systems); an `http://` URL needs nothing extra.
- Restart it with the same chain log and it carries on from its own height. If the producer's tunnel URL
  changed, give the new URL.
- If it prints `FOLLOWING STOPPED`, it has refused something: a block that breaks the rules, a producer whose
  chain is shorter than its own or whose history differs, or another network. It keeps serving the chain it
  has and does not try again by itself. If the chain was reset on purpose (`docs/TESTNET.md`, reset policy),
  start the follower on a new chain log; keep the old one if you want the old chain.
- `GET /state` on a follower says `"role": "replica"` and, under `replica`, the producer, whether it is still
  following, and why it stopped.

## What does not work yet (open)

- **More than one producer.** One node makes every block of a chain, for now. If it stops, the chain stops;
  a follower keeps serving what it has but cannot make blocks. A follower checks every block against the
  rulebook (the proof of work against the target the block names), but it trusts the producer's URL (and
  TLS, for https) for who made the chain, and it cannot re-check who signed a contribution, because the
  block does not carry the wallet signature. `--connect` (section 3) still syncs once and exits.
- **Spending JUL from the wallet.** On the testnet each block's JUL reward goes to the faucet of the node
  that made the block, and the faucet pays small amounts on request (built). The web wallet has no key
  that can spend JUL yet, so a wallet address can hold test JUL but not move it. Test JUL is worthless by
  design.
- **On-chain enforcement.** The four on-VM binding switches are off, so the reference node checks the
  rules (on the testnet that includes every spend's signature) and the on-VM scripts do not yet enforce
  them. Turning them on is the go-live milestone.
- **Wallet safety.** The wallet's hot key is stored in the browser encrypted (built): AES-GCM under a
  random data key, which the passphrase (PBKDF2-SHA256, 600,000 rounds, a random salt per wallet) and an
  optional passkey (the WebAuthn PRF extension) each unlock. It is in memory only while unlocked; Lock,
  or leaving the page, wipes it. The one-time-key counter is stored beside it in plain text, so it can be
  saved before every signature. Still open: a malicious page or extension running while the wallet is
  unlocked can read the key, and clearing browser storage loses the wallet unless you kept the hot key
  (Show hot key, after unlock). Do not restore an old copy of a wallet that has signed since: reusing a
  one-time key weakens it. Backups, key rotation and recovery with the cold key are designed, not built.

## Contributions and standing

A contribution earns standing by how much of it is new: text already on the chain scores zero. On the
testnet, the chain's very first contribution earns no standing (`Constitution::genesis_unscored`), so
nobody holds standing from launch.
