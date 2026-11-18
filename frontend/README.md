# Noēsis frontend

A single self-contained page (`index.html`, no build step, no framework, no third-party script) that talks
to a running node's HTTP API (`node/src/rpc.rs`): make a wallet, sign and submit a contribution, and watch
the chain, the contribution graph and per-contributor PoM standing update live.

## Files

- `index.html`: the page. Its script never touches browser storage itself.
- `crypto.js`: the post-quantum keys (Lamport, XMSS, XMSS^MT, wallet format v2), a byte-for-byte port of
  the Rust `noesis_core`, checked by `parity-test.mjs`.
- `wallet.js`: the one door to the stored wallet: the encrypted hot key, the passphrase and passkey
  unlock paths, the one-time-key counter, the key series (which key signs now), a recovery in progress,
  the origin rule, and the migration of old plain-text records. The cold key never passes through it: the
  page uses it in memory for one action and wipes it.

All three are compiled into the node binary (`include_str!` in `node/src/rpc.rs`) and served at `/`,
`/crypto.js` and `/wallet.js`. A change here reaches a running node only after the node is rebuilt.

## Run it

```bash
# from the repo root: the node serves the page itself
cargo run -p noesis --bin noesisd -- --serve-api 127.0.0.1:9955
#   then open http://localhost:9955/
```

The wallet works only at `http://localhost:9955` (decision A5): browser storage is kept per exact origin and
a passkey is bound to the host name, so any other address (127.0.0.1, another port, a tunnel URL, `file://`)
has a separate store. There the page still reads the chain, but it will not create a wallet, sign or add a
passkey, and it says why.

## The wallet at rest (decision A4)

Stored under `noesis_wallet_v2_sealed`:

- in plain text: the address, the hot root, the cold-key commitment and the one-time-key counter. They are
  public, and the counter must be saved, synchronously and without a key, before every signature;
- `seed_ct`: the hot seed, AES-256-GCM under a random 32-byte data key, bound to the public fields;
- `pass`: the data key wrapped under PBKDF2-SHA256 (600,000 rounds, a random 16-byte salt per wallet) of a
  passphrase of 12 characters or more;
- `passkey` (optional): the data key wrapped under HKDF-SHA256 of the passkey's WebAuthn PRF output. If the
  browser or authenticator has no PRF support, the page says so and the passphrase stays the only path.

Since the key series (wallet decision A1, 2026-11-18) the plain-text part also holds `keyIndex` and
`keyRoot`, the number and root of the hot key that signs now (absent: key 0, the seed itself), and
`pendingKeyOp`, a signed rotation or cancel kept until the chain shows it (a rotation: until it takes effect
after the chain's delay, or is cancelled) so it is re-sent byte for byte, never signed twice. The counter belongs to key `keyIndex`; the pair only ever goes up. A recovery made in
the browser waits under `noesis_wallet_v2_recovery` (sealed the same way, with its signed request) until the
chain puts its key in effect, then becomes the wallet; a wallet of the same address already there is kept
under `noesis_wallet_v2_retired`. The cold key is never stored.

Either wrap alone unlocks. The seed is in memory only while unlocked; Lock, or leaving the page, wipes it
(best effort: JavaScript cannot promise freed bytes are gone). The cold recovery key is shown once at
creation and never stored. A wallet stored in plain text by an older version of the page
(`noesis_wallet_v2`) is encrypted in place on first load: the encrypted copy is opened again and must
rebuild the same address before the plain record is removed; if anything fails, the plain record stays.

## Keys: rotate, freeze and recover (decisions A1 and A7)

The cold key is the final authority over an address: it can freeze it at once, cancel a rotation, unfreeze,
and replace the hot key. Whoever holds it can do all of that, so it lives offline, on paper.

- The wallet shows which key of its series signs and how many of its 65,536 one-time keys are left. The last
  is kept for a rotation: the page never signs a contribution or a cancel with it, and the node refuses one.
- Rotate (unlocked): the current key signs a rotation to the next key of the series, at a one-time index
  spent before signing. It rides the next contribution block and takes effect the chain's delay after that
  block (3 blocks on dev, 100 on the testnet); a note under the buttons says so. Until then the current key
  keeps signing, the banner shows the countdown, and the cold key can cancel the rotation. The page moves to
  the new key only when the chain shows it in effect; if the cold key cancels it, the page drops the signed
  rotation and says so. The seed, and so the backup, does not change.
- The red banner shows the address's key state: frozen by the cold key (the page then refuses to sign or
  rotate); this wallet's own pending rotation; a rotation it did not start (someone holds the hot key: use
  the cold key); a recovery request, with a cancel button while unlocked and not frozen.
- COLD KEY: paste the address and the cold key for each action. The page checks the cold key against the
  address, signs once in memory, clears the field and stores nothing. Freeze (from the next block the hot key
  signs nothing), Cancel the pending rotation, Unfreeze (refused while a rotation is pending), and Recover
  (choose a passphrase for a new hot key made here). While the chain holds no key record for the address, the
  first operation must also name its first hot key root. The section shows the address's state and
  countdowns; once a recovered key is in effect, "Use the recovered key" makes it this browser's wallet.
- The cold key's one-time index is the chain's floor (or this page's last use, if higher) plus a spread of 0
  to 255, so two browsers that cannot see each other rarely pick the same one; rarely, not never.

## Tests

```bash
node frontend/parity-test.mjs   # crypto.js matches the Rust byte for byte
node frontend/wallet-test.mjs   # wallet.js, and the page's own script in a fake browser (a minute or two)
```

`wallet-test.mjs` stubs WebAuthn with a simulated authenticator, so passkeys still need a manual check in
real browsers at `http://localhost:9955`. Its key-lane node is a fake that mirrors the rule (the rule itself
is tested in Rust): it checks every signature the page makes with the real verifier, but the page has not
been run against a real node or in a real browser for rotate, freeze, cancel, unfreeze or recover.

## Honest scope

A testnet page. Still open: a malicious page or extension running while the wallet is unlocked can read
the key; an encrypted backup file is designed, not built; rotation goes only to the next key derived from
the same seed, so after a suspected seed leak the answer is a freeze and a recovery (which makes a fresh
seed); a stolen cold key loses the address (it can freeze first, and then the hot key cannot cancel its
recovery); two tabs signing at the same moment are not yet serialized (the chain's high-water mark is the
backstop).
