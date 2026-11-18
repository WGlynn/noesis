// Wallet test: frontend/wallet.js on its own (A4 sealing, A5 origin), then the page's OWN inline script
// run in a fake browser: stub elements parsed from index.html, a Map-backed localStorage, a fake node
// that checks every signature with the real verifier, and a simulated passkey.
// Run: `node frontend/wallet-test.mjs`  (exit 0 = all pass). It takes a minute or two: every wallet here
// builds real XMSS^MT trees, and every seal or unlock runs real PBKDF2 at 600,000 rounds.
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
    signs.push({ seed: master, g, storedNext: JSON.parse(storage.getItem(W.KEY_SEALED)).nextIndex });
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

console.log(`\n${passed}/${total} checks pass`);
console.log(ok ? 'ALL WALLET CHECKS PASS' : 'WALLET CHECK FAILURE');
process.exit(ok ? 0 : 1);
