// Wallet test: frontend/wallet.js on its own (A4 sealing, A5 origin), then the page's OWN inline script
// run in a fake browser: stub elements parsed from index.html, a Map-backed localStorage, a fake node
// that checks every signature with the real verifier, and a simulated passkey.
// Run: `node frontend/wallet-test.mjs`  (exit 0 = all pass). It takes a minute or two: every wallet here
// builds real XMSS^MT trees, and every seal or unlock runs real PBKDF2 at 600,000 rounds.
//
// Also exercised here: the key series, the cold key's freeze, cancel and unfreeze, and recovery (A1/A7) on
// the page, against a fake node that keeps the identity-key rule's states and checks every signature with
// the real verifier; the rule itself is pinned in Rust (node/src/identity_key.rs and the rpc.rs tests).
//
// Only stub-tested: WebAuthn. The "authenticator" below is HMAC-SHA256(credential secret, "WebAuthn PRF"
// || 0x00 || salt), the shape of what a browser asks a CTAP2 hmac-secret authenticator for. No real
// browser, platform authenticator or security key took part, and no real browser storage either.
import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import vm from 'vm';
import { webcrypto, createHmac, randomBytes } from 'crypto';

const here = dirname(fileURLToPath(import.meta.url));
const src = (f) => readFileSync(join(here, f), 'utf8');
const CRYPTO = src('crypto.js'), WALLET = src('wallet.js'), HTML = src('index.html');
const INLINE = HTML.match(/<script>([\s\S]*?)<\/script>/)[1];

eval(CRYPTO);
eval(WALLET);
const C = globalThis.NoesisCrypto, W = globalThis.NoesisWallet;

let ok = true, total = 0, passed = 0;
const check = (name, pass, detail) => {
  total++; if (pass) passed++; else ok = false;
  console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}`);
  if (!pass && detail !== undefined) console.log('      ', detail);
};
const rejects = async (fn, re) => { try { await fn(); return false; } catch (e) { return re.test(e.message); } };

class FakeStorage {
  constructor(init) { this.m = new Map(Object.entries(init || {})); this.failSet = null; }
  getItem(k) { return this.m.has(k) ? this.m.get(k) : null; }
  setItem(k, v) {
    if (this.failSet && this.failSet(k)) { const e = new Error('The quota has been exceeded.'); e.name = 'QuotaExceededError'; throw e; }
    this.m.set(k, String(v));
  }
  removeItem(k) { this.m.delete(k); }
  dump() { return [...this.m.values()].join('\n'); }
}

// ============ fixture: the pinned parity wallet (hot seed [7;32], cold seed [8;32]) ============
const SEED7 = '07'.repeat(32);
const hot7 = C.hexDecode(SEED7);
const root7 = C.mtCache(hot7).root;
const commit8 = C.coldCommit(C.mtKeygenRoot(new Uint8Array(32).fill(8)));
const PUB = { address: C.hexEncode(C.identity(root7, commit8)), hot_root: C.hexEncode(root7), cold_commit: C.hexEncode(commit8), nextIndex: 41 };
check('fixture: the parity wallet has the pinned identity', PUB.address === 'af3b14f66bde666ba77b3f7486f678f88b39ece95dded520587d8c9ebcf4ebe7');
const UPASS = 'unit test passphrase one';

// ============ wallet.js on its own ============
check('passphrase: under 12 characters refused', /at least 12/.test(W.checkPass('elevenchars', 'elevenchars')));
check('passphrase: two copies that differ refused', /do not match/.test(W.checkPass('twelve chars!', 'twelve chars?')));
check('passphrase: 12 characters accepted', W.checkPass('twelve chars', 'twelve chars') === null);
check('origin: only http://localhost:9955 is the wallet origin',
  W.originOk('http://localhost:9955') && !W.originOk('http://127.0.0.1:9955') && !W.originOk('https://abc.trycloudflare.com') &&
  !W.originOk('http://localhost:9956') && !W.originOk('null'));

const sealA = await W.seal(hot7, PUB, await W.passKek(UPASS));
const sealB = await W.seal(hot7, PUB, await W.passKek(UPASS));
const recA = sealA.record;
check('seal: PBKDF2-SHA256, 600,000 rounds, a 16-byte salt stored with the wrap',
  recA.pass.kdf === 'PBKDF2-SHA256' && recA.pass.iter === 600000 && W.PBKDF2_ITER === 600000 && /^[0-9a-f]{32}$/.test(recA.pass.salt));
check('seal: no seed field, and the seed bytes appear nowhere in the record', !('seed' in recA) && !JSON.stringify(recA).includes(SEED7));
check('seal: a random salt, data key and nonce per wallet (two seals share nothing)',
  recA.pass.salt !== sealB.record.pass.salt && recA.seed_ct.ct !== sealB.record.seed_ct.ct && !C.bytesEqual(sealA.dataKey, sealB.dataKey));
const openA = await W.unlockPass(recA, UPASS);
check('unlock: the right passphrase returns the identical seed', C.hexEncode(openA.seed) === SEED7);
check('unlock: a wrong passphrase is refused', await rejects(() => W.unlockPass(recA, UPASS + 'x'), /wrong passphrase/));
const flip = (h) => (h[0] === '0' ? '1' : '0') + h.slice(1);
check('unlock: a changed ciphertext is refused',
  await rejects(() => W.unlockPass({ ...recA, seed_ct: { ...recA.seed_ct, ct: flip(recA.seed_ct.ct) } }, UPASS), /does not open/));
check('unlock: a swapped hot root is refused (the seed is bound to the public fields)',
  await rejects(() => W.unlockPass({ ...recA, hot_root: flip(recA.hot_root) }, UPASS), /does not open/));
const prfOut = new Uint8Array(randomBytes(32));
const recPk = await W.addPasskey(recA, openA.dataKey, { cred: 'ab'.repeat(16), salt: C.hexEncode(randomBytes(32)), first: prfOut });
const viaPrf = await W.unlockPrf(recPk, prfOut), viaPass = await W.unlockPass(recPk, UPASS);
check('passkey wrap: the PRF output alone opens the identical seed', C.hexEncode(viaPrf.seed) === SEED7);
check('passkey wrap: both wraps open the same data key', C.bytesEqual(viaPrf.dataKey, viaPass.dataKey));
check('passkey wrap: another PRF output is refused', await rejects(() => W.unlockPrf(recPk, new Uint8Array(randomBytes(32))), /does not open this wallet/));
{
  const st = new FakeStorage({ [W.KEY_SEALED]: JSON.stringify(recA) });
  const up = W.saveCounter(st, PUB.address, 50), down = W.saveCounter(st, PUB.address, 10);
  check('counter: raised synchronously, never lowered', up === 50 && down === 50 && W.storedNext(st, PUB.address) === 50);
  check('counter: refused for a different stored wallet', await rejects(() => W.saveCounter(st, 'cd'.repeat(32), 60), /stored wallet changed/));
  check('store: a new wallet never overwrites a stored one', await rejects(() => W.storeNew(st, sealB.record), /never overwrites/));
}
{ // another tab rewrites the plain record while the migration runs: nothing is written, the plain record stays
  const before = JSON.stringify({ v: 2, seed: SEED7, ...PUB });
  const st = new FakeStorage({ [W.KEY_PLAIN]: before });
  let reads = 0;
  const get = st.getItem.bind(st);
  st.getItem = (k) => (k === W.KEY_PLAIN && ++reads > 1 ? before.replace('"nextIndex":41', '"nextIndex":42') : get(k));
  check('migration: a plain record changed by another tab mid-migration aborts it',
    await rejects(() => W.migrate(st, UPASS), /changed in another tab/) && get(W.KEY_SEALED) === null && get(W.KEY_PLAIN) === before);
}
check('one door: the page script never touches localStorage itself', !/localStorage\.(get|set|remove)Item|localStorage\[/.test(INLINE));

// ============ the page in a fake browser ============
const unpart = (p, index) => ({ index, ots_root: C.hexDecode(p.ots_root), auth: p.auth.map((a) => C.hexDecode(a)), ots_sig: C.hexDecode(p.ots_sig) });
function fakeNode(storage) {
  const node = { highest: null, posts: [], reject: null, state: { height: 7, jul_issued: 0, contributors: {}, chain_id: 'test' } };
  node.fetch = async (url, opts = {}) => {
    const path = String(url).replace(/^[a-z]+:\/\/[^/]+/, '');
    const reply = (status, obj) => ({ ok: status === 200, status, json: async () => obj });
    if (path === '/state') return reply(200, node.state);
    if (path.startsWith('/index/')) return reply(200, { highest_used: node.highest });
    if (path === '/contributions') return reply(200, { contributions: [] });
    if (path === '/tokens') return reply(200, { cells: [] });
    if (path === '/blocks') return reply(200, { blocks: [] });
    if (path === '/submit' && opts.method === 'POST') {
      const body = JSON.parse(opts.body);
      const sealed = JSON.parse(storage.getItem(W.KEY_SEALED) || 'null');
      const sig = { g: body.index, bottom: unpart(body.bottom, body.index & 255), top: unpart(body.top, body.index >>> 8) };
      const msg = C.contributionDigest(C.hexDecode(body.identity), body.index, body.parent ?? null, C.utf8(body.data));
      const valid = C.mtVerify(C.hexDecode(body.hot_root), body.index, msg, sig) &&
        C.hexEncode(C.identity(C.hexDecode(body.hot_root), C.hexDecode(body.cold_commit))) === body.identity;
      const at = (String(url).match(/^[a-z]+:\/\/[^/]+/) || [''])[0];
      node.posts.push({ body, valid, storedNextAtSend: sealed && sealed.nextIndex, at });
      // a replica (testnet step 7) makes no blocks: 409 with the producer's URL, as rpc.rs answers
      if (node.replicaOf && at !== node.replicaOf) return reply(409, { error: 'this node is a replica', producer: node.replicaOf + '/' });
      if (node.reject) return reply(400, { error: node.reject });
      node.highest = body.index;
      node.state.height++;
      return reply(200, { block_height: node.state.height, contributors: { [body.identity]: 1 } });
    }
    return reply(404, { error: 'not found' });
  };
  return node;
}
// A passkey authenticator, simulated: one random secret per credential; PRF = HMAC over the salt.
function fakeAuthenticator({ prf = true } = {}) {
  const a = { prompts: 0, secrets: new Map() };
  a.create = async (opts) => {
    a.prompts++;
    if (opts.publicKey.authenticatorSelection.userVerification !== 'required') throw new Error('test: user verification must be required');
    const id = new Uint8Array(randomBytes(16));
    a.secrets.set(Buffer.from(id).toString('hex'), randomBytes(32));
    return { rawId: id.buffer, getClientExtensionResults: () => (prf ? { prf: { enabled: true } } : {}) };
  };
  a.get = async (opts) => {
    a.prompts++;
    const pk = opts.publicKey;
    if (pk.userVerification !== 'required') throw new Error('test: user verification must be required');
    const secret = a.secrets.get(Buffer.from(pk.allowCredentials[0].id).toString('hex'));
    if (!secret) { const e = new Error('no such credential'); e.name = 'NotAllowedError'; throw e; }
    const first = createHmac('sha256', secret).update(Buffer.concat([Buffer.from('WebAuthn PRF\0', 'latin1'), Buffer.from(pk.extensions.prf.eval.first)])).digest();
    return { getClientExtensionResults: () => (prf ? { prf: { results: { first: new Uint8Array(first).buffer } } } : {}) };
  };
  return a;
}
function openPage({ origin = 'http://localhost:9955', storage, node, creds = null, blockStorage = false }) {
  const els = {};
  for (const m of HTML.matchAll(/<([a-z]+)\b([^>]*)\bid="([^"]+)"([^>]*)>/g)) {
    const attrs = m[2] + m[4] + ' ';
    els[m[3]] = { id: m[3], value: (attrs.match(/\bvalue="([^"]*)"/) || [])[1] || '', textContent: '', innerHTML: '',
                  className: '', disabled: false, hidden: /\shidden\s/.test(attrs), files: [] };
  }
  const handlers = {};
  const u = new URL(origin);
  const win = {
    document: { getElementById: (id) => els[id] || null },
    localStorage: storage,
    location: { origin, protocol: u.protocol, hostname: u.hostname },
    navigator: { credentials: creds, clipboard: null },
    fetch: node.fetch, crypto: webcrypto, TextEncoder, TextDecoder, console, URL,
    setTimeout: (fn) => setTimeout(fn, 0), clearTimeout, setInterval: () => 0,
    confirm: () => true, FileReader: class {},
    addEventListener: (ev, fn) => { (handlers[ev] = handlers[ev] || []).push(fn); },
  };
  if (blockStorage) Object.defineProperty(win, 'localStorage', { get() { throw new Error('SecurityError: storage blocked'); } });
  win.window = win;
  vm.createContext(win);
  vm.runInContext(CRYPTO, win, { filename: 'crypto.js' });
  vm.runInContext(WALLET, win, { filename: 'wallet.js' });
  const signs = []; // every seed the page signs with, and the stored counter at that moment
  const realSign = win.NoesisCrypto.mtSign;
  win.NoesisCrypto.mtSign = (master, g, msg, cache) => {
    signs.push({ seed: master, g, storedNext: (JSON.parse(storage.getItem(W.KEY_SEALED)) || {}).nextIndex });
    return realSign(master, g, msg, cache);
  };
  vm.runInContext(INLINE, win, { filename: 'index.html' });
  const pg = {
    els, signs,
    text: (id) => els[id].textContent,
    set: (pairs) => { for (const [k, v] of Object.entries(pairs)) els[k].value = v; },
    click: async (id) => { await els[id].onclick(); await settle(); },
    fire: (ev) => (handlers[ev] || []).forEach((f) => f()),
    send: async (text) => { els.what.value = text; await pg.click('send'); },
    unlock: async (pass) => { els['u-pass'].value = pass; await pg.click('unlock-pass'); },
  };
  return pg;
}
const settle = async () => { for (let i = 0; i < 6; i++) await new Promise((r) => setTimeout(r, 2)); };
const zeroed = (b) => b.every((x) => x === 0);
const PASS = 'correct horse battery staple';

// ---- fresh create with a passphrase, sign, lock, reload, wrong and right passphrase, counter ----
{
  const storage = new FakeStorage(), node = fakeNode(storage);
  let pg = openPage({ storage, node });
  await settle();
  check('page: a fresh browser shows the create box and no origin warning', !pg.els['wallet-new'].hidden && pg.els['origin-note'].hidden);
  pg.set({ 'new-pass': 'too short', 'new-pass2': 'too short' });
  await pg.click('create-wallet');
  check('create: a short passphrase is refused before any key is made', /at least 12/.test(pg.text('gen-status')) && pg.els['cold-backup'].hidden && storage.m.size === 0);
  pg.set({ 'new-pass': PASS, 'new-pass2': PASS });
  await pg.click('create-wallet');
  check('create: the cold key is shown and nothing is stored before it is confirmed', !pg.els['cold-backup'].hidden && storage.m.size === 0);
  const coldHex = pg.text('cold-key').replace(/\s+/g, '');
  pg.set({ 'cold-check': coldHex.slice(-8) });
  await pg.click('cold-done');
  const rec = JSON.parse(storage.getItem(W.KEY_SEALED) || 'null');
  check('create: exactly one record is stored, the sealed one', !!rec && !!rec.seed_ct && storage.m.size === 1 && storage.getItem(W.KEY_PLAIN) === null);
  check('create: PBKDF2-SHA256 at 600,000 rounds with a random salt', rec.pass.iter === 600000 && /^[0-9a-f]{32}$/.test(rec.pass.salt));
  await pg.click('backup-toggle');
  const seedHex = (pg.text('seedbox').match(/[0-9a-f]{64}/) || [])[0];
  check('backup: Show hot key works while unlocked', !!seedHex && !pg.els.seedbox.hidden);
  check('at rest: neither the hot seed nor the cold key is anywhere in storage', !!seedHex && !storage.dump().includes(seedHex) && !storage.dump().includes(coldHex));
  await pg.click('backup-toggle');
  await pg.send('a first idea about tide pools');
  const p0 = node.posts[0];
  check('submit: a valid signature at index 0 reaches the node', !!p0 && p0.valid && p0.body.index === 0 && p0.body.identity === rec.address);
  check('counter: stored as 1 before the signature existed, and still 1 when it was sent', pg.signs[0].storedNext === 1 && p0.storedNextAtSend === 1);
  const live = pg.signs[0].seed;
  check('lock test setup: the page signed with the real seed', C.hexEncode(live) === seedHex);
  await pg.click('lock');
  check('lock: the in-memory seed is wiped to zero bytes', zeroed(live));
  await pg.click('backup-toggle');
  check('lock: Show hot key needs unlock', /Unlock first/.test(pg.text('w-status')) && pg.els.seedbox.hidden && !pg.text('seedbox'));
  await pg.send('anything');
  check('lock: submit refuses while locked', /Unlock the wallet first/.test(pg.text('toast')) && node.posts.length === 1);

  pg = openPage({ storage, node });
  await settle();
  check('reload: starts locked, no auto-unlock, and shows the address and keys left',
    !pg.els['unlock-box'].hidden && /locked/.test(pg.text('w-info')) && pg.text('w-info').includes(rec.address) && /keys left 65535 of/.test(pg.text('w-info')));
  await pg.unlock('wrong horse battery staple');
  check('unlock: a wrong passphrase is refused', /wrong passphrase/.test(pg.text('u-status')) && !pg.els['unlock-box'].hidden);
  await pg.send('another idea');
  check('unlock: still locked after a wrong passphrase, so submit refuses', /Unlock the wallet first/.test(pg.text('toast')) && node.posts.length === 1);
  await pg.unlock(PASS);
  check('unlock: the right passphrase opens it', pg.els['unlock-box'].hidden && /UNLOCKED/.test(pg.text('w-info')));
  node.reject = 'near-duplicate of an existing contribution';
  await pg.send('a second idea about kelp');
  check('rejected: the key at index 1 stays spent (stored counter 2)',
    node.posts[1].body.index === 1 && node.posts[1].storedNextAtSend === 2 && JSON.parse(storage.getItem(W.KEY_SEALED)).nextIndex === 2 && /spent/.test(pg.text('toast')));
  node.reject = null;
  node.highest = 9;
  await pg.send('a third idea about salt marsh');
  check('chain floor: resumes at 10 when the chain already saw 9', node.posts[2].body.index === 10 && node.posts[2].valid && node.posts[2].storedNextAtSend === 11);
  const live2 = pg.signs[pg.signs.length - 1].seed;
  pg.fire('pagehide');
  check('page unload: the seed is wiped and the page is locked', zeroed(live2) && !pg.els['unlock-box'].hidden);

  // ---- passkey (stub-tested): add, unlock with it alone, a different secret refused ----
  const auth = fakeAuthenticator();
  pg = openPage({ storage, node, creds: auth });
  await settle();
  check('passkey: no Add button while locked', pg.els['add-passkey'].hidden);
  await pg.unlock(PASS);
  await pg.click('add-passkey');
  const recK = JSON.parse(storage.getItem(W.KEY_SEALED));
  check('passkey (stub): added after two prompts, create and a confirming get', !!recK.passkey && auth.prompts === 2 && /Passkey added/.test(pg.text('w-status')));
  check('passkey: the counter and the passphrase wrap are kept', recK.nextIndex === 11 && recK.pass.salt === rec.pass.salt);
  pg = openPage({ storage, node, creds: auth });
  await settle();
  await pg.click('unlock-key');
  check('passkey (stub): unlocks on its own, no passphrase typed', !pg.els['unlock-key'].hidden && /UNLOCKED/.test(pg.text('w-info')));
  await pg.send('a fourth idea about eelgrass');
  check('passkey (stub): the unlocked wallet signs at 11', node.posts[3].valid && node.posts[3].body.index === 11);
  const other = fakeAuthenticator();
  other.secrets.set(recK.passkey.cred, randomBytes(32));
  pg = openPage({ storage, node, creds: other });
  await settle();
  await pg.click('unlock-key');
  check('passkey (stub): the same credential id with a different secret is refused', /does not open this wallet/.test(pg.text('u-status')) && !/UNLOCKED/.test(pg.text('w-info')));
  await pg.unlock(PASS);
  check('passkey: the passphrase still unlocks a wallet that has a passkey', /UNLOCKED/.test(pg.text('w-info')));
}

// ---- PRF not supported, and no WebAuthn at all: said plainly, passphrase only ----
{
  const storage = new FakeStorage({ [W.KEY_SEALED]: JSON.stringify(recA) }), node = fakeNode(storage);
  const noPrf = fakeAuthenticator({ prf: false });
  let pg = openPage({ storage, node, creds: noPrf });
  await settle();
  await pg.unlock(UPASS);
  await pg.click('add-passkey');
  const r = JSON.parse(storage.getItem(W.KEY_SEALED));
  check('no PRF (stub): the page says so plainly and adds no passkey', /does not support the WebAuthn PRF/.test(pg.text('w-status')) && r.passkey === null);
  pg = openPage({ storage, node, creds: null });
  await settle();
  await pg.unlock(UPASS);
  await pg.click('add-passkey');
  check('no WebAuthn: the page says so and the passphrase still unlocks', /no passkey support/.test(pg.text('w-status')) && /UNLOCKED/.test(pg.text('w-info')));
}

// ---- migration of an existing plain-text v2 wallet (the load-bearing path) ----
const MPASS = 'genesis wallet passphrase';
const plainRec = { v: 2, seed: SEED7, address: PUB.address, hot_root: PUB.hot_root, cold_commit: PUB.cold_commit, nextIndex: 41 };
const plainText = JSON.stringify(plainRec);
const v1Text = JSON.stringify({ v: 1, seed: 'aa'.repeat(32), address: 'bb'.repeat(32), nextIndex: 3 });
{
  const storage = new FakeStorage({ [W.KEY_PLAIN]: plainText, [W.KEY_V1]: v1Text }), node = fakeNode(storage);
  let pg = openPage({ storage, node });
  await settle();
  check('migration: the page asks for a passphrase to encrypt the plain-text wallet',
    !pg.els.migrate.hidden && pg.els['wallet-new'].hidden && pg.els['wallet-view'].hidden &&
    pg.text('m-info').includes(PUB.address) && /keys used 41 of/.test(pg.text('m-info')));
  await pg.send('an idea before migrating');
  check('migration: nothing is signed from the plain-text record', /Encrypt the wallet first/.test(pg.text('toast')) && node.posts.length === 0);
  pg.set({ 'm-pass': MPASS, 'm-pass2': MPASS + '!' });
  await pg.click('migrate-go');
  check('migration: mismatched passphrases are refused, storage unchanged', /do not match/.test(pg.text('m-status')) && storage.getItem(W.KEY_PLAIN) === plainText && storage.m.size === 2);
  pg.set({ 'm-pass': MPASS, 'm-pass2': MPASS });
  await pg.click('migrate-go');
  const m = JSON.parse(storage.getItem(W.KEY_SEALED) || 'null');
  check('migration: the plain-text record is gone and a sealed record is stored', storage.getItem(W.KEY_PLAIN) === null && !!m && !!m.seed_ct);
  check('migration: same address, hot root, cold commitment and counter (41)',
    !!m && m.address === plainRec.address && m.hot_root === plainRec.hot_root && m.cold_commit === plainRec.cold_commit && m.nextIndex === 41);
  check('migration: the seed is nowhere in storage any more', !storage.dump().includes(SEED7));
  check('migration: the v1 record is untouched, byte for byte', storage.getItem(W.KEY_V1) === v1Text);
  check('migration: the sealed copy decrypts to the identical seed (checked outside the page)', C.hexEncode((await W.unlockPass(m, MPASS)).seed) === SEED7);
  check('migration: the page reports it and is unlocked', /Wallet encrypted/.test(pg.text('toast')) && /UNLOCKED/.test(pg.text('w-info')) && pg.els.migrate.hidden);
  pg = openPage({ storage, node });
  await settle();
  await pg.unlock(MPASS);
  await pg.click('backup-toggle');
  check('migration: after a reload and unlock, the backup shows the original seed', pg.text('seedbox').includes(SEED7));
  await pg.send('the first idea from the migrated wallet');
  const p = node.posts[0];
  check('migration: the migrated wallet signs at index 41 as the same identity, counter 42 stored first',
    !!p && p.valid && p.body.index === 41 && p.body.identity === PUB.address && pg.signs[0].storedNext === 42 && p.storedNextAtSend === 42);
}
{ // storage full: the encrypted copy cannot be written, so the plain record must stay exactly as it was
  const storage = new FakeStorage({ [W.KEY_PLAIN]: plainText }), node = fakeNode(storage);
  storage.failSet = (k) => k === W.KEY_SEALED;
  const pg = openPage({ storage, node });
  await settle();
  pg.set({ 'm-pass': MPASS, 'm-pass2': MPASS });
  await pg.click('migrate-go');
  check('migration failure (storage full): error shown, plain record unchanged, nothing sealed',
    /Not encrypted/.test(pg.text('m-status')) && storage.getItem(W.KEY_PLAIN) === plainText && storage.getItem(W.KEY_SEALED) === null && !pg.els.migrate.hidden);
}
{ // a plain record whose seed does not rebuild its address is refused and left alone
  const bad = JSON.stringify({ ...plainRec, seed: '09'.repeat(32) });
  const storage = new FakeStorage({ [W.KEY_PLAIN]: bad }), node = fakeNode(storage);
  const pg = openPage({ storage, node });
  await settle();
  pg.set({ 'm-pass': MPASS, 'm-pass2': MPASS });
  await pg.click('migrate-go');
  check('migration failure (record does not match its key): refused, plain record unchanged',
    /does not rebuild its own address/.test(pg.text('m-status')) && storage.getItem(W.KEY_PLAIN) === bad && storage.getItem(W.KEY_SEALED) === null);
}
{ // an interrupted migration (or an old tab) left both copies: the higher counter wins, plain is removed
  const sealed50 = JSON.stringify({ ...recA, nextIndex: 50 });
  const storage = new FakeStorage({ [W.KEY_SEALED]: sealed50, [W.KEY_PLAIN]: JSON.stringify({ ...plainRec, nextIndex: 45 }) }), node = fakeNode(storage);
  const pg = openPage({ storage, node });
  await settle();
  check('both copies present: the page asks to finish encrypting, counter shown as 50', !pg.els.migrate.hidden && /keys used 50 of/.test(pg.text('m-info')));
  pg.set({ 'm-pass': MPASS, 'm-pass2': MPASS });
  await pg.click('migrate-go');
  const m = JSON.parse(storage.getItem(W.KEY_SEALED));
  check('both copies present: plain removed, counter 50 kept (never rolled back to 45)', storage.getItem(W.KEY_PLAIN) === null && m.nextIndex === 50 && m.address === PUB.address);
}

// ---- A5: a wrong origin reads the chain but cannot create or sign ----
{
  const storage = new FakeStorage(), node = fakeNode(storage);
  let pg = openPage({ origin: 'http://127.0.0.1:9955', storage, node });
  await settle();
  check('wrong origin: a warning names the wallet origin and says why',
    !pg.els['origin-note'].hidden && pg.text('origin-note').includes('http://localhost:9955') && /storage separately/.test(pg.text('origin-note')) && /passkeys/.test(pg.text('origin-note')));
  check('wrong origin: the chain is still read and shown', /height 7/.test(pg.text('chain')) && /live/.test(pg.text('live')));
  pg.set({ 'new-pass': PASS, 'new-pass2': PASS });
  await pg.click('create-wallet');
  check('wrong origin: creating a wallet is refused, nothing generated or stored',
    pg.els['create-wallet'].disabled && /off at this address/.test(pg.text('gen-status')) && pg.els['cold-backup'].hidden && storage.m.size === 0);
  const s2 = new FakeStorage({ [W.KEY_SEALED]: JSON.stringify(recA) }), n2 = fakeNode(s2);
  pg = openPage({ origin: 'https://abc.trycloudflare.com', storage: s2, node: n2, creds: fakeAuthenticator() });
  await settle();
  await pg.unlock(UPASS);
  await pg.send('an idea from a tunnel URL');
  check('wrong origin: a stored wallet can be unlocked here but signing is refused', /UNLOCKED/.test(pg.text('w-info')) && /Signing is off here/.test(pg.text('toast')) && n2.posts.length === 0);
  await pg.click('add-passkey');
  check('wrong origin: adding a passkey is refused', pg.els['add-passkey'].hidden && /off at this address/.test(pg.text('w-status')) && JSON.parse(s2.getItem(W.KEY_SEALED)).passkey === null);
}

{ // a replica answers a submit with 409 and the producer's URL: the page sends the SAME signed body there, once
  const storage = new FakeStorage({ [W.KEY_SEALED]: JSON.stringify(recA) }), node = fakeNode(storage);
  node.replicaOf = 'https://producer.example';
  node.state.replica = { producer: node.replicaOf, following: true, stopped: null };
  const pg = openPage({ storage, node });
  await settle();
  check('replica: the status line says whose replica this node is', /replica of https:\/\/producer\.example/.test(pg.text('live')));
  await pg.unlock(UPASS);
  const before = JSON.parse(storage.getItem(W.KEY_SEALED)).nextIndex;
  await pg.send('a note on eelgrass beds, sent through a replica');
  const [a, b] = node.posts;
  check('replica: the 409 is followed once, to the producer, with the identical signed body',
    node.posts.length === 2 && a.at !== node.replicaOf && b.at === node.replicaOf && JSON.stringify(a.body) === JSON.stringify(b.body) && b.valid);
  check('replica: one message, one one-time key spent', JSON.parse(storage.getItem(W.KEY_SEALED)).nextIndex === before + 1 && pg.signs.length === 1);
  check('replica: the toast names the producer', /finalized by the producer at https:\/\/producer\.example\./.test(pg.text('toast')));
}

{ // a browser that blocks storage: the page says so, still reads the chain, and offers no wallet
  const node = fakeNode(new FakeStorage());
  const pg = openPage({ storage: null, node, blockStorage: true });
  await settle();
  check('blocked storage: the page explains it, hides wallet creation, and still reads the chain',
    !pg.els['store-note'].hidden && /blocks storage/.test(pg.text('store-note')) && pg.els['wallet-new'].hidden && /height 7/.test(pg.text('chain')));
}

// ============ the key series and recovery (wallet decisions A1 and A7) ============
// wallet.js: the (key, counter) pair only moves forward; the recovery record; the cold key never stored.
{
  const st = new FakeStorage({ [W.KEY_SEALED]: JSON.stringify({ ...recA, nextIndex: 7 }) });
  const root1 = C.hexEncode(C.mtKeygenRoot(C.deriveKey(hot7, 1)));
  check('key series: a record from before A1 reads as key 0 with its own hot root',
    W.keyOf(recA).keyIndex === 0 && W.keyOf(recA).keyRoot === recA.hot_root);
  W.saveRotation(st, PUB.address, 1, root1);
  const k = W.storedKey(st, PUB.address);
  check('key series: a rotation moves to key 1 with its counter at 0', k.keyIndex === 1 && k.keyRoot === root1 && k.nextIndex === 0);
  check('key series: a rotation never goes back or repeats a key',
    await rejects(async () => W.saveRotation(st, PUB.address, 1, root1), /only moves forward/) &&
    await rejects(async () => W.saveRotation(st, PUB.address, 0, PUB.hot_root), /only moves forward/));
  check('key series: a counter save for the retired key is refused (another tab rotated)',
    await rejects(async () => W.saveCounter(st, PUB.address, 9, 0), /moved to another key/) && W.saveCounter(st, PUB.address, 3, 1) === 3);
  W.store(st, { ...recA, nextIndex: 40 }); // an older copy of the record, written back by an old tab
  const after = W.storedKey(st, PUB.address);
  check('key series: writing back an older record keeps the later key and its counter (key 1 at 3 beats key 0 at 40)',
    after.keyIndex === 1 && after.nextIndex === 3 && after.keyRoot === root1);
}

// A fake node with the key lane: every signature checked with the real verifier, the rule's states (no
// record, record, frozen, a pending rotation or recovery, a rotation held while frozen, maturity after the
// delay) kept the way node/src/identity_key.rs keeps them. The real rule is pinned in Rust; this checks
// what the page signs and sends, and what it shows.
const sigOf = (b) => ({ g: b.index, bottom: unpart(b.bottom, b.index & 255), top: unpart(b.top, b.index >>> 8) });
const ZERO32 = new Uint8Array(32);
function keyNode() {
  const node = { height: 7, chain: 0x7e57, delay: 3, recs: new Map(), queued: new Map(), used: new Map(), posts: [], keyPosts: [], lost: false,
                 state: { height: 7, jul_issued: 0, contributors: {}, chain_id: 0x7e57 } };
  const at = (r, h) => r && r.pending && h >= r.pending.effective_at && !(r.frozen && r.pending.change === 'rotation')
    ? { generation: r.generation + 1, hot_root: r.pending.new_root, cold_commit: r.cold_commit, hot_floor: 0, cold_floor: r.cold_floor, frozen: false, pending: null } : r;
  const current = (id, h) => (node.queued.get(id) || {}).after || at(node.recs.get(id), h) || null;
  const usedKey = (id, g) => node.used.get(id + '|' + g);
  const highest = (id, g, h) => {
    const r = current(id, h), ops = r && r.generation === g && r.hot_floor > 0 ? r.hot_floor - 1 : null;
    const c = usedKey(id, g);
    return c == null ? ops : ops == null ? c : Math.max(c, ops);
  };
  const json = (r, next) => r && { ...r, pending: r.pending && { ...r.pending, cancel_blocks_left: r.pending.effective_at - next,
    suspended: !!r.frozen && r.pending.change === 'rotation' } };
  const OPS = { rotate: C.OP_ROTATE, 'cancel-recovery': C.OP_CANCEL, recover: C.OP_RECOVER, 'cancel-rotation': C.OP_CANCEL_ROTATION,
                freeze: C.OP_FREEZE, unfreeze: C.OP_UNFREEZE };
  const BY_COLD = ['recover', 'cancel-rotation', 'freeze', 'unfreeze'];
  // The stored record as it stands at the next block (a matured change written in), so a test can inject
  // a state on top of it the way a later key operation would.
  node.mature = (id) => node.recs.set(id, at(node.recs.get(id), node.height + 1));
  node.block = () => { // one block: it carries every queued key operation
    node.height++; node.state.height = node.height;
    for (const [id, q] of node.queued) node.recs.set(id, q.after);
    node.queued.clear();
  };
  node.fetch = async (url, opts = {}) => {
    const path = String(url).replace(/^[a-z]+:\/\/[^/]+/, '');
    const reply = (status, obj) => ({ ok: status === 200, status, json: async () => obj });
    const next = node.height + 1;
    if (path === '/state') return reply(200, node.state);
    if (path.startsWith('/keys/')) {
      const id = path.slice(6), cur = current(id, next), onChain = at(node.recs.get(id), next) || null, q = node.queued.get(id);
      const generation = cur ? cur.generation : 0;
      return reply(200, { identity: id, chain_id: node.chain, height: node.height, next_block: next, key_rotation: true, key_change_delay_blocks: node.delay,
        on_chain: json(onChain, next), queued: q ? { op: q.op, index: q.index, new_root: q.new_root } : null, current: json(cur, next),
        generation, highest_used: highest(id, generation, next) ?? null });
    }
    if (path.startsWith('/index/')) return reply(404, { error: 'not found' });
    if (path === '/contributions') return reply(200, { contributions: [] });
    if (path === '/tokens') return reply(200, { cells: [] });
    if (path === '/blocks') return reply(200, { blocks: [] });
    if (path === '/key' && opts.method === 'POST') {
      const b = JSON.parse(opts.body);
      node.keyPosts.push({ body: b, raw: opts.body });
      const id = b.identity, prior = at(node.recs.get(id), next);
      if (node.queued.has(id)) return reply(400, { error: 'another key operation for this identity is waiting' });
      const op = OPS[b.op];
      if (op == null) return reply(400, { error: 'unknown op' });
      let eff = prior;
      if (!eff) {
        if (!b.birth || C.hexEncode(C.identity(C.hexDecode(b.birth.hot_root), C.hexDecode(b.birth.cold_commit))) !== id) return reply(400, { error: 'birth roots' });
        eff = { generation: 0, hot_root: b.birth.hot_root, cold_commit: b.birth.cold_commit, hot_floor: 0, cold_floor: 0, frozen: false, pending: null };
      } else if (b.birth) return reply(400, { error: 'this identity already has a key record' });
      const newRoot = b.op === 'rotate' || b.op === 'recover' ? C.hexDecode(b.new_root) : ZERO32;
      const msg = C.keyOpDigest(C.hexDecode(id), op, node.chain, eff.generation, newRoot, b.index);
      const pend = eff.pending && eff.pending.change, effective_at = next + node.delay + 1;
      let after;
      if (BY_COLD.includes(b.op)) {
        if (C.hexEncode(C.coldCommit(C.hexDecode(b.cold_root))) !== eff.cold_commit) return reply(400, { error: 'the cold key does not match this identity\'s cold commitment' });
        if (b.index < eff.cold_floor) return reply(400, { error: 'this one-time index of the cold key is already used' });
        if (b.op === 'recover') after = { ...eff, pending: { change: 'recovery', new_root: b.new_root, effective_at } };
        else if (b.op === 'cancel-rotation') { if (pend !== 'rotation') return reply(400, { error: 'no rotation is pending' }); after = { ...eff, pending: null }; }
        else if (b.op === 'freeze') { if (eff.frozen) return reply(400, { error: 'already frozen' }); after = { ...eff, frozen: true }; }
        else {
          if (!eff.frozen) return reply(400, { error: 'not frozen' });
          if (pend === 'rotation') return reply(400, { error: 'a rotation is pending' });
          after = { ...eff, frozen: false };
        }
        if (!C.mtVerify(C.hexDecode(b.cold_root), b.index, msg, sigOf(b))) return reply(400, { error: 'not signed by the cold key' });
        after = { ...after, cold_floor: b.index + 1 };
      } else {
        if (eff.frozen) return reply(400, { error: 'this identity is frozen by its cold key' });
        const hi = highest(id, eff.generation, next);
        if (b.index < eff.hot_floor || (hi != null && b.index <= hi)) return reply(400, { error: 'already used' });
        if (b.op === 'rotate') {
          if (pend) return reply(400, { error: 'a key change is pending' });
          after = { ...eff, pending: { change: 'rotation', new_root: b.new_root, effective_at } };
        } else {
          if (pend !== 'recovery' || b.index >= C.MT_MAX_SIGS - 1) return reply(400, { error: 'no recovery is pending' });
          after = { ...eff, pending: null };
        }
        if (!C.mtVerify(C.hexDecode(eff.hot_root), b.index, msg, sigOf(b))) return reply(400, { error: 'not signed by the current hot key' });
        after = { ...after, hot_floor: b.index + 1 };
      }
      if (!node.lost) node.queued.set(id, { op: b.op, index: b.index, new_root: b.new_root || null, after });
      return reply(200, { queued: true, after });
    }
    if (path === '/submit' && opts.method === 'POST') {
      const b = JSON.parse(opts.body), id = b.identity, cur = current(id, next);
      if (cur && cur.frozen) { node.posts.push({ body: b, valid: false, frozen: true }); return reply(400, { error: 'this identity is frozen by its cold key' }); }
      const msg = C.contributionDigest(C.hexDecode(id), b.index, b.parent ?? null, C.utf8(b.data));
      const keyOk = cur ? b.hot_root === cur.hot_root : C.hexEncode(C.identity(C.hexDecode(b.hot_root), C.hexDecode(b.cold_commit))) === id;
      const gen = cur ? cur.generation : 0, hi = highest(id, gen, next);
      const valid = keyOk && b.index < C.MT_MAX_SIGS - 1 && C.mtVerify(C.hexDecode(b.hot_root), b.index, msg, sigOf(b)) && (hi == null || b.index > hi);
      node.posts.push({ body: b, valid, gen });
      if (!valid) return reply(400, { error: keyOk ? 'already used or bad signature' : 'not this identity\'s current hot key' });
      node.used.set(id + '|' + gen, b.index);
      node.block();
      node.state.contributors[id] = (node.state.contributors[id] || 0) + 1;
      return reply(200, { block_height: node.height, contributors: node.state.contributors });
    }
    return reply(404, { error: 'not found' });
  };
  return node;
}

// ---- A1 on the page: rotate (needs unlock); the rotation waits out the delay, then the next key signs ----
const COLD8 = new Uint8Array(32).fill(8);
const COLD8_HEX = C.hexEncode(COLD8);
const coldRoot8 = C.mtKeygenRoot(COLD8);
const lastKeyPost = (node) => node.keyPosts[node.keyPosts.length - 1];
const stored = (storage) => JSON.parse(storage.getItem(W.KEY_SEALED));
{
  const storage = new FakeStorage({ [W.KEY_SEALED]: JSON.stringify(recA) }), node = keyNode();
  const addr = C.hexDecode(PUB.address);
  let pg = openPage({ storage, node });
  await settle();
  check('page: the wallet shows its key number and the signatures left for that key',
    /hot key   #0 of this wallet's series, key generation 0 on chain/.test(pg.text('w-info')) && /keys left 65495 of 65536 for this key/.test(pg.text('w-info')));
  check('page: with no key record on chain, the first hot key root is shown for the recovery sheet', pg.text('w-info').includes('first key ' + PUB.hot_root));
  check('rotate: the page explains the wait (3 blocks after the block that carries it) and that the cold key can cancel it',
    /takes effect 3 blocks after the block that carries the rotation/.test(pg.text('rotate-note')) && /the cold key can cancel the rotation/.test(pg.text('rotate-note')));
  await pg.click('rotate');
  check('rotate: refused while locked', /Unlock first to rotate/.test(pg.text('w-status')) && node.keyPosts.length === 0);
  await pg.unlock(UPASS);
  await pg.click('rotate');
  const kp = node.keyPosts[0];
  const root1 = C.mtKeygenRoot(C.deriveKey(hot7, 1));
  const rotMsg = kp && C.keyOpDigest(addr, C.OP_ROTATE, 0x7e57, 0, root1, 41);
  check('rotate: one signed rotation to key 1 of the series, by key 0 at the next one-time index (41), with the birth roots',
    !!kp && kp.body.op === 'rotate' && kp.body.new_root === C.hexEncode(root1) && kp.body.index === 41 &&
    kp.body.birth && kp.body.birth.hot_root === PUB.hot_root && C.mtVerify(root7, 41, rotMsg, sigOf(kp.body)));
  check('rotate: index 41 was spent (counter 42 stored) before the signature existed', pg.signs[0].g === 41 && pg.signs[0].storedNext === 42);
  const rec1 = stored(storage);
  check('rotate: the wallet stays on key 0 (counter 42) and keeps the signed rotation until the chain puts it in effect',
    !rec1.keyIndex && rec1.nextIndex === 42 && !!rec1.pendingKeyOp && rec1.pendingKeyOp.keyIndex === 1 && rec1.pendingKeyOp.index === 41 &&
    /takes effect 3 blocks after it; until then key #0 keeps signing/.test(pg.text('w-status')));
  check('rotate: the hot seed in storage is unchanged (one backup covers the series)', rec1.seed_ct.ct === recA.seed_ct.ct);
  await pg.send('a note signed by key zero while the rotation waits');
  const p0 = node.posts[0];
  check('during the delay: the next contribution is signed by key 0 at index 42, in the block that carries the rotation',
    !!p0 && p0.valid && p0.body.index === 42 && p0.body.hot_root === PUB.hot_root && p0.gen === 0);
  await settle();
  check('during the delay: the banner shows this wallet\'s rotation with the block it takes effect at and the countdown',
    /Your rotation to key #1 takes effect at block 12, 3 blocks from now\. Until then key #0 keeps signing; the cold key can cancel it/.test(pg.text('key-banner')));
  await pg.click('rotate');
  check('during the delay: a second rotation is refused, and no one-time key is spent', /a rotation is already pending/.test(pg.text('w-status')) &&
    node.keyPosts.length === 1 && stored(storage).nextIndex === 43);
  node.block(); node.block(); node.block(); // the delay runs out: block 12 is the first under key 1
  pg = openPage({ storage, node });
  await settle();
  const rec2 = stored(storage);
  check('in effect: the wallet moves to key 1 with its counter at 0, and the signed rotation is cleared',
    rec2.keyIndex === 1 && rec2.keyRoot === C.hexEncode(root1) && rec2.nextIndex === 0 && !rec2.pendingKeyOp &&
    /key generation 1 on chain/.test(pg.text('w-info')) && !pg.text('w-info').includes('first key') && pg.els['key-banner'].hidden);
  await pg.unlock(UPASS);
  await pg.send('the first note signed by key one');
  const p1 = node.posts[1];
  check('in effect: the next contribution is signed by key 1 at index 0', !!p1 && p1.valid && p1.body.index === 0 && p1.body.hot_root === C.hexEncode(root1) &&
    p1.gen === 1 && pg.signs[pg.signs.length - 1].seed.some((x, i) => x !== hot7[i]));
  // reload: unlock rebuilds key 1 from the seed and signs with it
  pg = openPage({ storage, node });
  await settle();
  await pg.unlock(UPASS);
  await pg.send('a second note from key one after a reload');
  check('reload: the unlocked wallet signs with key 1 at index 1', node.posts[2].valid && node.posts[2].body.index === 1 && node.posts[2].gen === 1);

  // ---- A7 on the page: a recovery request against this address raises a banner; the hot key cancels it ----
  const thiefRoot = C.mtKeygenRoot(new Uint8Array(32).fill(0x99));
  node.mature(PUB.address);
  node.recs.get(PUB.address).pending = { change: 'recovery', new_root: C.hexEncode(thiefRoot), effective_at: node.height + 1 + 3 };
  pg = openPage({ storage, node });
  await settle();
  check('banner: a pending recovery against this address is shown in red, with its block and countdown, and a cancel button',
    !pg.els['key-banner'].hidden && /WARNING: a recovery request against this address is pending/.test(pg.text('key-banner')) &&
    pg.text('key-banner').includes('at block ' + (node.height + 4)) && /3 blocks from now/.test(pg.text('key-banner')) && !pg.els['cancel-recovery'].hidden);
  await pg.click('cancel-recovery');
  check('cancel: refused while locked', /Unlock first to cancel/.test(pg.text('w-status')));
  await pg.unlock(UPASS);
  await pg.click('cancel-recovery');
  const cp = lastKeyPost(node);
  const cMsg = C.keyOpDigest(addr, C.OP_CANCEL, 0x7e57, 1, ZERO32, 2);
  check('cancel: signed by the current key (key 1) at its next index, for generation 1, and queued',
    cp.body.op === 'cancel-recovery' && cp.body.index === 2 && !cp.body.birth && C.mtVerify(root1, 2, cMsg, sigOf(cp.body)) && node.queued.has(PUB.address));
  check('cancel: the counter of key 1 moved past the cancel before it was signed', stored(storage).nextIndex === 3);
  node.block(); // a block lands it
  pg = openPage({ storage, node });
  await settle();
  check('cancel: once it lands the banner is gone', pg.els['key-banner'].hidden && pg.els['cancel-recovery'].hidden);

  // ---- a stolen hot key on the page: a rotation this wallet did not start; freeze, cancel it, unfreeze ----
  const thiefKey = C.hexEncode(C.mtKeygenRoot(new Uint8Array(32).fill(0x98)));
  node.mature(PUB.address);
  Object.assign(node.recs.get(PUB.address), { pending: { change: 'rotation', new_root: thiefKey, effective_at: node.height + 1 + 3 }, hot_floor: 4 });
  pg = openPage({ storage, node });
  await settle();
  const banner = pg.text('key-banner');
  check('stolen hot key: a rotation this wallet did not start raises a warning with its block and countdown that points to the cold key',
    /WARNING: a rotation of this address's hot key is pending/.test(banner) && banner.includes('at block ' + (node.height + 4)) &&
    /3 blocks from now/.test(banner) && /freeze the address and cancel the rotation/.test(banner));
  check('stolen hot key: the COLD KEY section shows the pending rotation and its countdown',
    !pg.els['cold-line'].hidden && /not frozen/.test(pg.text('cold-line')) &&
    /Rotation pending to .+: takes effect at block \d+, 3 blocks from now, unless the cold key cancels it/.test(pg.text('cold-line')));
  await pg.click('rec-open');
  pg.set({ 'rec-address': PUB.address, 'rec-cold': COLD8_HEX.match(/.{8}/g).join(' ') });
  await pg.click('freeze-go');
  const fz = lastKeyPost(node);
  check('freeze: one operation signed by the cold key (op 4) for generation 1, naming the cold root and no birth roots, at a spread index under 256',
    fz.body.op === 'freeze' && fz.body.cold_root === C.hexEncode(coldRoot8) && !fz.body.birth && fz.body.index < 256 &&
    C.mtVerify(coldRoot8, fz.body.index, C.keyOpDigest(addr, C.OP_FREEZE, 0x7e57, 1, ZERO32, fz.body.index), sigOf(fz.body)) && node.queued.has(PUB.address));
  check('freeze: the cold key field is cleared, and the cold key is nowhere in storage', pg.els['rec-cold'].value === '' && !storage.dump().includes(COLD8_HEX));
  check('freeze: the status says what follows', /Freeze sent/.test(pg.text('coldop-status')));
  node.block();
  pg = openPage({ storage, node });
  await settle();
  await pg.unlock(UPASS);
  check('frozen: the banner says so, and that the thief\'s rotation is held and must be cancelled before unfreezing',
    /FROZEN: the cold key has frozen this address/.test(pg.text('key-banner')) && /held while the address is frozen/.test(pg.text('key-banner')) &&
    /FROZEN by the cold key/.test(pg.text('cold-line')) && /it has FROZEN the address/.test(pg.text('w-info')));
  const before = { posts: node.posts.length, next: stored(storage).nextIndex, signs: pg.signs.length };
  await pg.send('a note the frozen address must not sign');
  check('frozen: the page refuses to sign a contribution; nothing is sent and no one-time key is spent',
    /frozen by its cold key/.test(pg.text('toast')) && node.posts.length === before.posts && stored(storage).nextIndex === before.next && pg.signs.length === before.signs);
  await pg.click('rotate');
  check('frozen: rotate is refused before anything is signed', /frozen by its cold key/.test(pg.text('w-status')) && pg.signs.length === before.signs);
  pg.set({ 'rec-address': PUB.address, 'rec-cold': COLD8_HEX });
  await pg.click('unfreeze-go');
  check('unfreeze with a rotation held: refused before the cold key signs', /a rotation is pending: cancel it first/.test(pg.text('coldop-status')) &&
    lastKeyPost(node) === fz && pg.els['rec-cold'].value === '');
  pg.set({ 'rec-cold': COLD8_HEX });
  await pg.click('cancel-rotation-go');
  const cr = lastKeyPost(node);
  check('cancel the rotation: signed by the cold key (op 6) for generation 1, above the freeze\'s cold index',
    cr.body.op === 'cancel-rotation' && cr.body.index > fz.body.index &&
    C.mtVerify(coldRoot8, cr.body.index, C.keyOpDigest(addr, C.OP_CANCEL_ROTATION, 0x7e57, 1, ZERO32, cr.body.index), sigOf(cr.body)));
  node.block();
  pg.set({ 'rec-cold': COLD8_HEX });
  await pg.click('unfreeze-go');
  const uf = lastKeyPost(node);
  check('unfreeze: signed by the cold key (op 5), above the cancel\'s cold index',
    uf.body.op === 'unfreeze' && uf.body.index > cr.body.index &&
    C.mtVerify(coldRoot8, uf.body.index, C.keyOpDigest(addr, C.OP_UNFREEZE, 0x7e57, 1, ZERO32, uf.body.index), sigOf(uf.body)));
  node.block();
  pg = openPage({ storage, node });
  await settle();
  await pg.unlock(UPASS);
  check('unfrozen: the banner is gone and the thief\'s key never took effect', pg.els['key-banner'].hidden && /key generation 1 on chain/.test(pg.text('w-info')));
  await pg.send('a note after the cold key unfroze the address');
  const pu = node.posts[node.posts.length - 1];
  check('unfrozen: key 1 signs again, above the index the thief\'s rotation spent', pu.valid && pu.gen === 1 && pu.body.index === 4);

  // ---- this wallet's own rotation, cancelled with the cold key ----
  await pg.click('rotate');
  const own = lastKeyPost(node);
  node.block(); // it lands, pending
  node.recs.get(PUB.address).pending = null; // the cold key cancels it (its hot floor stays past the rotation's index)
  pg = openPage({ storage, node });
  await settle();
  const rec3 = stored(storage);
  check('own rotation cancelled by the cold key: the wallet stays on key 1, drops the signed rotation, and says why',
    own.body.op === 'rotate' && rec3.keyIndex === 1 && !rec3.pendingKeyOp && /did not take effect: the cold key cancelled it/.test(pg.text('key-banner')));

  // ---- a key operation the node lost (a restart) is sent again, byte for byte ----
  await pg.unlock(UPASS);
  node.lost = true;
  await pg.click('rotate');
  const lost = lastKeyPost(node);
  node.lost = false;
  const count = node.keyPosts.length;
  pg = openPage({ storage, node });
  await settle();
  const again = node.keyPosts[count];
  check('re-send: a rotation the node lost is sent again with the identical body, and no second signature',
    !!again && again.raw === lost.raw && stored(storage).keyIndex === 1 && stored(storage).pendingKeyOp.keyIndex === 2 && node.queued.has(PUB.address));
}

// ---- A7 on the page: recover in a fresh browser with the cold key, freeze, wait, then use the recovered key ----
{
  const node = keyNode();
  const storage = new FakeStorage();
  let pg = openPage({ storage, node });
  await settle();
  check('recover: the COLD KEY section is offered in a browser with no wallet', !pg.els['rec-open'].hidden);
  await pg.click('rec-open');
  const coldHex = C.hexEncode(COLD8);
  pg.set({ 'rec-address': PUB.address, 'rec-cold': coldHex.match(/.{8}/g).join(' '), 'rec-hot': '', 'rec-pass': 'a new phrase for the new key', 'rec-pass2': 'a new phrase for the new key' });
  await pg.click('recover-go');
  check('recover: with no key record on chain and no first hot root, the request is refused before any signature',
    /must name the address's first hot key root/.test(pg.text('rec-status')) && node.keyPosts.length === 0 && pg.signs.length === 0);
  check('recover: the cold key field is cleared at once', pg.els['rec-cold'].value === '');
  pg.set({ 'rec-cold': '09'.repeat(32), 'rec-hot': PUB.hot_root, 'rec-pass': 'a new phrase for the new key', 'rec-pass2': 'a new phrase for the new key' });
  await pg.click('recover-go');
  check('recover: a cold key that does not make the address is refused', /do not make this address/.test(pg.text('rec-status')) && node.keyPosts.length === 0);
  pg.set({ 'rec-cold': coldHex, 'rec-hot': PUB.hot_root, 'rec-pass': 'a new phrase for the new key', 'rec-pass2': 'a new phrase for the new key' });
  await pg.click('recover-go');
  const rp = node.keyPosts[0];
  const coldRoot = C.mtKeygenRoot(COLD8);
  const rMsg = rp && C.keyOpDigest(C.hexDecode(PUB.address), C.OP_RECOVER, 0x7e57, 0, C.hexDecode(rp.body.new_root), rp.body.index);
  check('recover: one request signed by the cold key, naming its root and the birth roots, at a spread index under 256',
    !!rp && rp.body.op === 'recover' && rp.body.cold_root === C.hexEncode(coldRoot) && rp.body.birth.hot_root === PUB.hot_root &&
    rp.body.index < 256 && C.mtVerify(coldRoot, rp.body.index, rMsg, sigOf(rp.body)));
  check('recover: the cold key is nowhere in storage, and the new hot seed is stored encrypted only',
    !storage.dump().includes(coldHex) && !!storage.getItem(W.KEY_RECOVERY) && !storage.getItem(W.KEY_SEALED));
  node.block(); // the request lands: pending, in effect 3 blocks later
  pg = openPage({ storage, node });
  await settle();
  check('recover: the status shows PENDING with the block it takes effect at and the countdown, and warns the hot key can cancel it unless frozen',
    /PENDING\. It takes effect at block 12, 3 blocks from now/.test(pg.text('rec-line')) && /hot key can cancel it; freezing the address/.test(pg.text('rec-line')) &&
    pg.els['recover-finish'].hidden);
  // the owner freezes too, so a thief holding the hot key cannot cancel the request
  await pg.click('rec-open');
  pg.set({ 'rec-address': PUB.address, 'rec-cold': coldHex });
  await pg.click('freeze-go');
  const fz = node.keyPosts[node.keyPosts.length - 1];
  check('recover then freeze: the freeze needs no birth roots now (a key record exists) and sits above the request\'s cold index',
    fz.body.op === 'freeze' && !fz.body.birth && fz.body.index > rp.body.index && node.queued.has(PUB.address));
  node.block();
  pg = openPage({ storage, node });
  await settle();
  check('recover then freeze: the status says the frozen address\'s hot key cannot cancel the request',
    /PENDING\. It takes effect at block 12, 2 blocks from now/.test(pg.text('rec-line')) && /frozen, so its hot key\ncannot cancel it/.test(pg.text('rec-line')));
  node.block(); node.block(); // the delay runs out; the recovery also unfreezes the address
  pg = openPage({ storage, node });
  await settle();
  check('recover: once in effect the page says so and offers to use the recovered key', /IN EFFECT/.test(pg.text('rec-line')) && !pg.els['recover-finish'].hidden);
  await pg.click('recover-finish');
  const w = JSON.parse(storage.getItem(W.KEY_SEALED) || 'null');
  check('recover: the recovered key becomes this browser\'s wallet for the same address',
    !!w && w.address === PUB.address && w.recovered === 1 && w.keyRoot === rp.body.new_root && !storage.getItem(W.KEY_RECOVERY));
  await pg.unlock('a new phrase for the new key');
  await pg.send('the first note after recovering the address');
  const p = node.posts[0];
  check('recover: the recovered key signs for the address from index 0, generation 1, unfrozen',
    !!p && p.valid && p.body.index === 0 && p.body.hot_root === rp.body.new_root && p.gen === 1 && p.body.identity === PUB.address);
  // the old hot key (seed [7;32]) is retired: its signature is refused
  const old = node.fetch('http://x/submit', { method: 'POST', body: JSON.stringify((() => {
    const m = C.contributionDigest(C.hexDecode(PUB.address), 50, null, C.utf8('old key'));
    const sg = C.mtSign(hot7, 50, m);
    const pt = (t) => ({ ots_root: C.hexEncode(t.ots_root), auth: t.auth.map((a) => C.hexEncode(a)), ots_sig: C.hexEncode(t.ots_sig) });
    return { identity: PUB.address, hot_root: PUB.hot_root, cold_commit: PUB.cold_commit, index: 50, bottom: pt(sg.bottom), top: pt(sg.top), data: 'old key' };
  })()) });
  check('recover: after it takes effect the old hot key signs nothing for the address', (await old).status === 400);
}

console.log(`\n${passed}/${total} checks pass`);
console.log(ok ? 'ALL WALLET CHECKS PASS' : 'WALLET CHECK FAILURE');
process.exit(ok ? 0 : 1);
