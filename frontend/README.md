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
  the origin rule, and the migration of old plain-text records.

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
`pendingKeyOp`, a signed rotation or cancel kept until the chain shows it so it is re-sent byte for byte,
never signed twice. The counter belongs to key `keyIndex`; the pair only ever goes up. A recovery made in
the browser waits under `noesis_wallet_v2_recovery` (sealed the same way, with its signed request) until the
chain puts its key in effect, then becomes the wallet; a wallet of the same address already there is kept
under `noesis_wallet_v2_retired`. The cold key is never stored.

Either wrap alone unlocks. The seed is in memory only while unlocked; Lock, or leaving the page, wipes it
(best effort: JavaScript cannot promise freed bytes are gone). The cold recovery key is shown once at
creation and never stored. A wallet stored in plain text by an older version of the page
(`noesis_wallet_v2`) is encrypted in place on first load: the encrypted copy is opened again and must
rebuild the same address before the plain record is removed; if anything fails, the plain record stays.

## Keys: rotate and recover (decisions A1 and A7)

- The wallet shows which key of its series signs and how many of its 65,536 one-time keys are left (the last
  is kept for a rotation).
- Rotate (unlocked): the current key signs a rotation to the next key of the series, at a one-time index
  spent before signing. It rides the next contribution block, and the page signs with the new key from then
  on. The seed, and so the backup, does not change.
- A red banner says when a recovery request against the address is pending (its block and countdown), with
  a button to cancel it while unlocked.
- RECOVER: paste the address and the cold key, choose a passphrase for a new hot key made here, send. The
  page checks the cold key against the address first, signs once in memory and clears the field. While the
  chain holds no key record for the address, the request must also name its first hot key root. The status
  shows the countdown; once the key is in effect, "Use the recovered key" makes it this browser's wallet.

## Tests

```bash
node frontend/parity-test.mjs   # crypto.js matches the Rust byte for byte
node frontend/wallet-test.mjs   # wallet.js, and the page's own script in a fake browser (a minute or two)
```

`wallet-test.mjs` stubs WebAuthn with a simulated authenticator, so passkeys still need a manual check in
real browsers at `http://localhost:9955`.

## Honest scope

A testnet page. Still open: a malicious page or extension running while the wallet is unlocked can read
the key; an encrypted backup file is designed, not built; rotation goes only to the next key derived from
the same seed (after a suspected seed leak a fresh seed is needed, which the chain accepts but the page does
not offer); recovery helps when the hot key is lost, not when it is stolen (the current hot key can cancel
it); two tabs signing at the same moment are not yet serialized (the chain's high-water mark is the
backstop).
