# Noēsis frontend

A single self-contained page (`index.html`, no build step, no framework, no third-party script) that talks
to a running node's HTTP API (`node/src/rpc.rs`): make a wallet, sign and submit a contribution, and watch
the chain, the contribution graph and per-contributor PoM standing update live.

## Files

- `index.html`: the page. Its script never touches browser storage itself.
- `crypto.js`: the post-quantum keys (Lamport, XMSS, XMSS^MT, wallet format v2), a byte-for-byte port of
  the Rust `noesis_core`, checked by `parity-test.mjs`.
- `wallet.js`: the one door to the stored wallet: the encrypted hot key, the passphrase and passkey
  unlock paths, the one-time-key counter, the origin rule, and the migration of old plain-text records.

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

Either wrap alone unlocks. The seed is in memory only while unlocked; Lock, or leaving the page, wipes it
(best effort: JavaScript cannot promise freed bytes are gone). The cold recovery key is shown once at
creation and never stored. A wallet stored in plain text by an older version of the page
(`noesis_wallet_v2`) is encrypted in place on first load: the encrypted copy is opened again and must
rebuild the same address before the plain record is removed; if anything fails, the plain record stays.

## Tests

```bash
node frontend/parity-test.mjs   # crypto.js matches the Rust byte for byte
node frontend/wallet-test.mjs   # wallet.js, and the page's own script in a fake browser (a minute or two)
```

`wallet-test.mjs` stubs WebAuthn with a simulated authenticator, so passkeys still need a manual check in
real browsers at `http://localhost:9955`.

## Honest scope

A testnet page. Still open: a malicious page or extension running while the wallet is unlocked can read
the key; backups, key rotation and recovery with the cold key are designed, not built; two tabs signing at
the same moment are not yet serialized (the chain's high-water mark is the backstop).
