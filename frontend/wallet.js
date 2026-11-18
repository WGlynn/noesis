// Noesis wallet storage: the one door. This is the only code that reads or writes the wallet in browser
// storage, encrypts or decrypts the hot seed, or moves the one-time-key counter (wallet design D1 and D2,
// Will's decisions A4 and A5, 2026-11-17). The page (index.html) calls it and never touches storage itself.
//
// At rest, under localStorage key noesis_wallet_v2_sealed:
//   in plain text: v, sealed, address, hot_root, cold_commit, nextIndex (all public; the page shows the
//                  wallet and its keys left while locked)
//   seed_ct:       the 32-byte hot seed, AES-256-GCM under a random 32-byte data key. The associated data
//                  binds it to the address, hot root and cold commitment, so a swapped field fails to open.
//   pass:          the data key, AES-256-GCM under PBKDF2-SHA256(passphrase, random 16-byte salt, 600,000)
//   passkey:       optional: the data key, AES-256-GCM under HKDF-SHA256(the passkey's WebAuthn PRF output)
// Either wrap alone opens the data key. The seed is never written in plain text.
//
// Why the counter stays outside the ciphertext: it must be saved, synchronously, before every signature,
// and raised from the chain while the wallet is locked. Inside the ciphertext every save would need the
// data key and an async re-encryption, which opens a window where a signature exists and its counter is
// not yet saved. Sealing it would not stop a rollback either, because whoever can write storage can put
// back an old ciphertext. The counter is public anyway (every signature carries its index), and the
// chain floor (GET /index/<address>) is what catches a rollback.
//
// Why a new storage key: the previous page reads noesis_wallet_v2 and, finding no seed there, would offer
// to generate a new wallet and overwrite it. Under its own key the sealed record is invisible to an old
// page still open in another tab, so that tab cannot destroy it.
//
// WebCrypto (crypto.subtle) and WebAuthn only, no dependency. Both need a secure context (HTTPS or
// localhost). Runs in Node too, for frontend/wallet-test.mjs. Needs crypto.js loaded first.
;(function (root) {
  'use strict';
  var C = root.NoesisCrypto;

  // A5: one stable origin. Storage is bound to the exact origin (scheme, host, port) and a passkey to the
  // host name (WebAuthn refuses a bare IP address), so the wallet lives at one address. A domain comes
  // later; until then it is the user's own node.
  var CANONICAL_ORIGIN = 'http://localhost:9955';
  var KEY_SEALED = 'noesis_wallet_v2_sealed';
  var KEY_PLAIN = 'noesis_wallet_v2';   // the old plain-text record: read only to migrate it, then removed
  var KEY_V1 = 'noesis_wallet_v1';      // the old v1 record: read for its address only, never changed
  // 600,000 rounds: OWASP's figure for PBKDF2-HMAC-SHA256 (Password Storage Cheat Sheet, 2023 and after),
  // and the figure VibeSwap's password fallback used. About half a second per unlock on a desktop. The
  // count is stored with each wrap, so it can be raised later without breaking old records.
  var PBKDF2_ITER = 600000;
  var MIN_PASS = 12;
  var HEX64 = /^[0-9a-f]{64}$/;
  var TE = new TextEncoder();

  function originOk(origin) { return origin === CANONICAL_ORIGIN; }

  function subtle() {
    var c = root.crypto || globalThis.crypto;
    if (!c || !c.subtle) throw new Error('this browser offers no WebCrypto at this address (it needs HTTPS or localhost); open ' + CANONICAL_ORIGIN);
    return c.subtle;
  }
  function random(n) {
    var b = new Uint8Array(n);
    (root.crypto || globalThis.crypto).getRandomValues(b);
    return b;
  }

  // A problem with a new passphrase, or null. `again` is the second copy the user typed.
  function checkPass(pass, again) {
    if (typeof pass !== 'string' || Array.from(pass.normalize('NFC')).length < MIN_PASS)
      return 'The passphrase must be at least ' + MIN_PASS + ' characters (four or more random words is a good shape).';
    if (pass !== again) return 'The two passphrases do not match.';
    return null;
  }

  // ============ AES-GCM boxes ============
  async function aesKey(raw) { return subtle().importKey('raw', raw, 'AES-GCM', false, ['encrypt', 'decrypt']); }
  async function encrypt(key, plain, aad) {
    var iv = random(12);
    var ct = await subtle().encrypt({ name: 'AES-GCM', iv: iv, additionalData: TE.encode(aad) }, key, plain);
    return { iv: C.hexEncode(iv), ct: C.hexEncode(new Uint8Array(ct)) };
  }
  async function decrypt(key, box, aad) {
    var pt = await subtle().decrypt({ name: 'AES-GCM', iv: C.hexDecode(box.iv), additionalData: TE.encode(aad) }, key, C.hexDecode(box.ct));
    return new Uint8Array(pt);
  }
  function seedAad(r) { return 'noesis-wallet-seed-v1|' + r.address + '|' + r.hot_root + '|' + r.cold_commit; }
  function wrapAad(kind, address) { return 'noesis-wallet-wrap-v1|' + kind + '|' + address; }

  // The passphrase's wrapping key. A new wrap gets a fresh random salt; an unlock passes the stored one.
  async function passKek(pass, saltHex, iter) {
    var salt = saltHex ? C.hexDecode(saltHex) : random(16);
    iter = iter || PBKDF2_ITER;
    var base = await subtle().importKey('raw', TE.encode(String(pass).normalize('NFC')), 'PBKDF2', false, ['deriveKey']);
    var key = await subtle().deriveKey({ name: 'PBKDF2', hash: 'SHA-256', salt: salt, iterations: iter },
      base, { name: 'AES-GCM', length: 256 }, false, ['encrypt', 'decrypt']);
    return { key: key, salt: C.hexEncode(salt), iter: iter };
  }
  // The passkey's wrapping key: HKDF over the 32-byte PRF output the authenticator computes.
  async function prfKek(first, saltHex) {
    var base = await subtle().importKey('raw', first, 'HKDF', false, ['deriveKey']);
    return subtle().deriveKey({ name: 'HKDF', hash: 'SHA-256', salt: C.hexDecode(saltHex), info: TE.encode('noesis-wallet-passkey-v1') },
      base, { name: 'AES-GCM', length: 256 }, false, ['encrypt', 'decrypt']);
  }

  // ============ seal and open ============
  function checkPublic(r) {
    if (!r || !HEX64.test(r.address) || !HEX64.test(r.hot_root) || !HEX64.test(r.cold_commit) ||
        !Number.isSafeInteger(r.nextIndex) || r.nextIndex < 0) throw new Error('malformed wallet record');
  }
  // Encrypt `seed` under a new random data key, wrapped by the passphrase key `kek` (from passKek).
  async function seal(seed, pub, kek) {
    var r = { v: 2, sealed: 1, address: pub.address, hot_root: pub.hot_root, cold_commit: pub.cold_commit, nextIndex: pub.nextIndex };
    checkPublic(r);
    var dataKey = random(32);
    r.seed_ct = await encrypt(await aesKey(dataKey), seed, seedAad(r));
    var w = await encrypt(kek.key, dataKey, wrapAad('pass', r.address));
    r.pass = { kdf: 'PBKDF2-SHA256', iter: kek.iter, salt: kek.salt, iv: w.iv, ct: w.ct };
    r.passkey = null;
    return { record: r, dataKey: dataKey };
  }
  async function openSeed(r, dataKey) {
    try { return await decrypt(await aesKey(dataKey), r.seed_ct, seedAad(r)); }
    catch (e) { throw new Error('the encrypted hot key does not open (the stored record was changed or damaged)'); }
  }
  async function unlockPass(r, pass) {
    var kek = await passKek(pass, r.pass.salt, r.pass.iter);
    var dataKey;
    try { dataKey = await decrypt(kek.key, r.pass, wrapAad('pass', r.address)); }
    catch (e) { throw new Error('wrong passphrase'); }
    return { seed: await openSeed(r, dataKey), dataKey: dataKey };
  }
  async function unlockPrf(r, first) {
    if (!r.passkey) throw new Error('no passkey is set for this wallet');
    var dataKey;
    try { dataKey = await decrypt(await prfKek(first, r.passkey.salt), r.passkey, wrapAad('passkey', r.address)); }
    catch (e) { throw new Error('this passkey does not open this wallet'); }
    return { seed: await openSeed(r, dataKey), dataKey: dataKey };
  }
  // A copy of `r` with a passkey wrap of the open `dataKey`, kept only once it opens again.
  async function addPasskey(r, dataKey, pk) {
    var w = await encrypt(await prfKek(pk.first, pk.salt), dataKey, wrapAad('passkey', r.address));
    var out = JSON.parse(JSON.stringify(r));
    out.passkey = { cred: pk.cred, salt: pk.salt, iv: w.iv, ct: w.ct };
    var back = await unlockPrf(out, pk.first);
    var same = C.bytesEqual(back.dataKey, dataKey);
    back.seed.fill(0); back.dataKey.fill(0);
    if (!same) throw new Error('the passkey wrap did not open to the same key; no passkey was added');
    return out;
  }

  // ============ storage ============
  function parse(storage, key) {
    var raw = storage.getItem(key);
    return raw == null ? null : JSON.parse(raw);
  }
  // What this browser holds. Never throws and never writes: a damaged record is reported, left as it is.
  function read(storage) {
    var out = { sealed: null, plain: null, v1Address: null, error: null };
    try {
      var s = parse(storage, KEY_SEALED);
      if (s) { checkPublic(s); if (!s.seed_ct || !s.pass) throw new Error('malformed'); out.sealed = s; }
    } catch (e) {
      out.error = 'The encrypted wallet record in this browser is damaged. This page leaves it untouched and will not sign with it.';
    }
    try {
      var p = parse(storage, KEY_PLAIN);
      if (p && p.v === 2) { if (!HEX64.test(p.seed)) throw new Error('bad seed'); checkPublic(p); out.plain = p; }
    } catch (e) {
      out.error = (out.error ? out.error + '\n' : '') + 'The plain-text wallet record in this browser is damaged. This page leaves it untouched.';
    }
    try { out.v1Address = (parse(storage, KEY_V1) || {}).address || null; } catch (e) { out.v1Address = null; }
    return out;
  }
  function sealedFor(storage, address) {
    var s = parse(storage, KEY_SEALED);
    if (!s || s.address !== address || !Number.isSafeInteger(s.nextIndex))
      throw new Error('the stored wallet changed (in another tab?), so this page will not sign; reload it');
    return s;
  }
  // The counter as stored now (another tab may have raised it).
  function storedNext(storage, address) { return sealedFor(storage, address).nextIndex; }
  // Raise the stored counter to at least `next`, synchronously, and never lower it. Returns the stored value.
  function saveCounter(storage, address, next) {
    var s = sealedFor(storage, address);
    if (next > s.nextIndex) {
      s.nextIndex = next;
      var text = JSON.stringify(s);
      storage.setItem(KEY_SEALED, text);
      if (storage.getItem(KEY_SEALED) !== text) throw new Error('the one-time-key counter did not save');
    }
    return s.nextIndex;
  }
  // Write a sealed record for its own address, keeping the higher counter if one is already stored.
  function store(storage, r) {
    checkPublic(r);
    var cur = parse(storage, KEY_SEALED);
    if (cur && cur.address !== r.address) throw new Error('this browser already holds a different encrypted wallet (' + cur.address + ')');
    if (cur && Number.isSafeInteger(cur.nextIndex) && cur.nextIndex > r.nextIndex) r = Object.assign({}, r, { nextIndex: cur.nextIndex });
    var text = JSON.stringify(r);
    storage.setItem(KEY_SEALED, text);
    if (storage.getItem(KEY_SEALED) !== text) throw new Error('the wallet did not save');
    return r;
  }
  // A new wallet: refused if any wallet is stored, so creation can never overwrite one.
  function storeNew(storage, r) {
    if (storage.getItem(KEY_SEALED) != null || storage.getItem(KEY_PLAIN) != null)
      throw new Error('a wallet is already stored in this browser, and this page never overwrites one');
    return store(storage, r);
  }
  function forget(storage) { storage.removeItem(KEY_SEALED); }

  // ============ migration of the old plain-text record (A4) ============
  // Order: the plain record must rebuild its own address; seal it; open the exact text that will be stored
  // with the passphrase and rebuild the address from the decrypted seed; check nobody changed the plain
  // record meanwhile; write the sealed record and read it back; only then remove the plain record. Any
  // failure before that last step leaves the plain record exactly as it was. The seed, address and roots
  // are carried byte for byte; the counter is carried as is (or the higher stored one, if an interrupted
  // migration already left a sealed copy of the same wallet). Returns the open session.
  async function migrate(storage, pass) {
    var before = storage.getItem(KEY_PLAIN);
    var p = before == null ? null : JSON.parse(before);
    if (!p || p.v !== 2 || !HEX64.test(p.seed)) throw new Error('no plain-text v2 wallet to encrypt');
    checkPublic(p);
    var rebuilds = function (seed) {
      var cache = C.mtCache(seed);
      var ok = C.hexEncode(cache.root) === p.hot_root &&
               C.hexEncode(C.identity(cache.root, C.hexDecode(p.cold_commit))) === p.address;
      return ok ? cache : null;
    };
    var seed = C.hexDecode(p.seed);
    if (!rebuilds(seed)) throw new Error('the plain-text wallet does not rebuild its own address, so it was not touched');
    var cur = parse(storage, KEY_SEALED);
    if (cur && cur.address !== p.address) throw new Error('this browser already holds a different encrypted wallet (' + cur.address + ')');
    var next = Math.max(p.nextIndex, cur && Number.isSafeInteger(cur.nextIndex) ? cur.nextIndex : 0);
    var made = await seal(seed, { address: p.address, hot_root: p.hot_root, cold_commit: p.cold_commit, nextIndex: next }, await passKek(pass));
    made.dataKey.fill(0);
    var text = JSON.stringify(made.record);
    var r = JSON.parse(text);
    var opened = await unlockPass(r, pass);
    if (!C.bytesEqual(opened.seed, seed)) throw new Error('the encrypted copy did not decrypt to the same key');
    var cache = rebuilds(opened.seed);
    if (!cache) throw new Error('the encrypted copy did not rebuild the same address');
    if (r.address !== p.address || r.hot_root !== p.hot_root || r.cold_commit !== p.cold_commit || r.nextIndex < p.nextIndex)
      throw new Error('the encrypted copy does not carry the same address and counter');
    if (storage.getItem(KEY_PLAIN) !== before)
      throw new Error('the wallet changed in another tab during the migration; close the other tabs of this page and try again');
    storage.setItem(KEY_SEALED, text); // throws when storage is full; the plain record is still untouched
    if (storage.getItem(KEY_SEALED) !== text) throw new Error('the encrypted copy did not save');
    storage.removeItem(KEY_PLAIN);
    seed.fill(0); // best effort; the session keeps opened.seed
    return { record: r, seed: opened.seed, dataKey: opened.dataKey, cache: cache };
  }

  // ============ passkeys (WebAuthn PRF) ============
  // The passkey holds a key, not a login: the authenticator computes PRF(credential secret, salt) only
  // after user verification, and that output (through HKDF) is the only thing that opens the passkey
  // wrap. Nothing that sits in storage opens it, unlike VibeSwap's passkey, which gated only the UI.
  // No server checks the assertion; it is not what protects the key.
  var NO_WEBAUTHN = 'This browser has no passkey support (WebAuthn). The passphrase still unlocks this wallet.';
  var NO_PRF_ADD = 'This browser or passkey does not support the WebAuthn PRF extension, which lets a passkey hold a key and not just log in. No passkey was added, and the passphrase still unlocks this wallet. (The authenticator may still list a "Noesis wallet" passkey; it opens nothing and can be deleted.)';
  var NO_PRF_UNLOCK = 'This browser did not return the passkey secret (the WebAuthn PRF extension), so the passkey cannot unlock here. Use the passphrase.';
  function prfFirst(cred) {
    var ext = cred && typeof cred.getClientExtensionResults === 'function' ? cred.getClientExtensionResults() : null;
    var f = ext && ext.prf && ext.prf.results && ext.prf.results.first;
    if (!f) return null;
    return ArrayBuffer.isView(f) ? new Uint8Array(f.buffer.slice(f.byteOffset, f.byteOffset + f.byteLength)) : new Uint8Array(f);
  }
  function ceremonyError(e) {
    if (e && e.name === 'NotAllowedError') return new Error('The passkey prompt was cancelled or timed out. Nothing changed.');
    if (e && e.name === 'SecurityError') return new Error('The browser refused passkeys at this address (a passkey needs a domain name such as localhost, not an IP address). Nothing changed.');
    return e instanceof Error ? e : new Error(String(e));
  }
  // Ask the authenticator for its PRF output over the wrap's salt (one user-verified prompt).
  async function passkeyEval(creds, wrap, adding) {
    if (!creds || typeof creds.get !== 'function') throw new Error(NO_WEBAUTHN);
    var got;
    try {
      got = await creds.get({ publicKey: {
        challenge: random(32),
        allowCredentials: [{ type: 'public-key', id: C.hexDecode(wrap.cred) }],
        // the PRF output differs with and without user verification, so every ceremony requires it
        userVerification: 'required',
        timeout: 120000,
        extensions: { prf: { eval: { first: C.hexDecode(wrap.salt) } } }
      } });
    } catch (e) { throw ceremonyError(e); }
    var first = prfFirst(got);
    if (!first) throw new Error(adding ? NO_PRF_ADD : NO_PRF_UNLOCK);
    if (first.length !== 32) throw new Error('This passkey returned a non-standard PRF result (' + first.length + ' bytes, not 32), so it is not used.');
    return first;
  }
  // Make a passkey for this wallet and read its PRF output once, so the unlock path is proven before
  // the wrap is kept. Two prompts: create, then confirm.
  async function passkeyCreate(creds, address) {
    if (!creds || typeof creds.create !== 'function' || typeof creds.get !== 'function') throw new Error(NO_WEBAUTHN);
    var salt = random(32), made;
    try {
      made = await creds.create({ publicKey: {
        rp: { name: 'Noesis wallet' }, // the RP ID defaults to this page's host: localhost
        user: { id: random(16), name: 'noesis-' + address.slice(0, 12), displayName: 'Noesis wallet ' + address.slice(0, 12) },
        challenge: random(32), // nothing checks it: the unlock secret is the PRF output, not the assertion
        pubKeyCredParams: [{ type: 'public-key', alg: -7 }, { type: 'public-key', alg: -257 }],
        authenticatorSelection: { userVerification: 'required', residentKey: 'preferred' },
        timeout: 120000,
        extensions: { prf: { eval: { first: salt } } }
      } });
    } catch (e) { throw ceremonyError(e); }
    var ext = made && typeof made.getClientExtensionResults === 'function' ? made.getClientExtensionResults() : {};
    if (!ext || !ext.prf || ext.prf.enabled === false) throw new Error(NO_PRF_ADD);
    var wrap = { cred: C.hexEncode(new Uint8Array(made.rawId)), salt: C.hexEncode(salt) };
    var first = await passkeyEval(creds, wrap, true);
    var atCreate = prfFirst(made);
    if (atCreate && !C.bytesEqual(atCreate, first))
      throw new Error('This passkey gave two different secrets for the same input (a non-standard PRF), so no passkey was added.');
    return { cred: wrap.cred, salt: wrap.salt, first: first };
  }

  root.NoesisWallet = {
    CANONICAL_ORIGIN: CANONICAL_ORIGIN, KEY_SEALED: KEY_SEALED, KEY_PLAIN: KEY_PLAIN, KEY_V1: KEY_V1,
    PBKDF2_ITER: PBKDF2_ITER, MIN_PASS: MIN_PASS,
    originOk: originOk, checkPass: checkPass, passKek: passKek, seal: seal,
    unlockPass: unlockPass, unlockPrf: unlockPrf, addPasskey: addPasskey,
    read: read, storedNext: storedNext, saveCounter: saveCounter, store: store, storeNew: storeNew, forget: forget,
    migrate: migrate, passkeyCreate: passkeyCreate, passkeyEval: passkeyEval
  };
})(typeof window !== 'undefined' ? window : globalThis);
