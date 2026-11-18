// Browser-crypto parity test: proves frontend/crypto.js is byte-identical to the Rust
// noesis_core::{lamport, xmss, xmss_mt, wallet}. The SAME vectors are pinned in the Rust test
// `rpc::tests::wallet_v2_parity_vectors_are_pinned` — if either side changes its hashing, one of the
// two tests fails. Run: `node frontend/parity-test.mjs`  (exit 0 = parity holds).
import { readFileSync } from 'fs';
import { createHash } from 'crypto';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const here = dirname(fileURLToPath(import.meta.url));
eval(readFileSync(join(here, 'crypto.js'), 'utf8'));
const C = globalThis.NoesisCrypto;

// Fixed keys: hot seed [7;32], cold seed [8;32]. Index 300 = subtree 1, leaf 44 (past the first subtree).
const hot = new Uint8Array(32).fill(7);
const cold = new Uint8Array(32).fill(8);
const cache = C.mtCache(hot);
const hotRoot = cache.root;
const coldCommit = C.coldCommit(C.mtKeygenRoot(cold));
const id = C.identity(hotRoot, coldCommit);
const data = C.utf8('hello noesis');
const rootDigest = C.contributionDigest(id, 300, null, data);
const onOne = C.contributionDigest(id, 300, 1, data);
const sig = C.mtSign(hot, 300, onOne, cache);
// BLAKE2b-512 over the signature's bytes in wire order (bottom then top: ots_root, auth, ots_sig).
const sigHash = createHash('blake2b512');
for (const part of [sig.bottom, sig.top]) for (const b of [part.ots_root, ...part.auth, part.ots_sig]) sigHash.update(b);

const cases = [
  ['single-tree XMSS address(seed=[7;32])', C.hexEncode(C.keygenAddress(hot)),
    'b8034627416d512d88c00d9cda4dfe0d1edb513102674e018c4c02732a34612e'],
  ['XMSS^MT hot root(seed=[7;32])', C.hexEncode(hotRoot),
    '12d796f0ff1476ddf8f735cf76d07eba31c2b282d26f926baf4834f13bc3cc2f'],
  ['cold_commit(MT root of seed=[8;32])', C.hexEncode(coldCommit),
    '67c96506105d8c24e3c4656e70625808b8117eeafad410c8623087cb8e1e811c'],
  ['identity(hot root, cold_commit)', C.hexEncode(id),
    'af3b14f66bde666ba77b3f7486f678f88b39ece95dded520587d8c9ebcf4ebe7'],
  ['contribution_digest(id,300,no parent,"hello noesis")', C.hexEncode(rootDigest),
    '172e999b35d8e167e3c8a9d7cfaebf61b6cf124950320349fb013f60e6f07d93'],
  ['contribution_digest(id,300,parent 1,"hello noesis")', C.hexEncode(onOne),
    '92bdcd71855d9533256b66e95fd0cc4765371487ca204aae6b09a6ed1b8bffe9'],
  ['blake2b-512 of the MT signature at 300', sigHash.digest('hex'),
    '406733314cd931ba7fe698409ffd3174fcb51fc0a29cdfe83679614028a4c59eff95e26e634a6ab0676f46731970b240dc7d74e39b0ccc23ec0bc256bd31a543'],
];

// The key series and key operations (A1/A7): key 1 of the hot seed, its root, a rotate digest at index
// 300 and a cancel digest at generation 5 and the last index, and the rotation signature's hash. The same
// bytes are pinned in `rpc::tests::key_series_parity_vectors_are_pinned`.
const k1 = C.deriveKey(hot, 1);
const root1 = C.mtKeygenRoot(k1);
const rotDigest = C.keyOpDigest(id, C.OP_ROTATE, 0x7e57, 0, root1, 300);
const rotSig = C.mtSign(hot, 300, rotDigest, cache);
const rotHash = createHash('blake2b512');
for (const part of [rotSig.bottom, rotSig.top]) for (const b of [part.ots_root, ...part.auth, part.ots_sig]) rotHash.update(b);
cases.push(
  ['key 0 is the hot seed itself (an existing wallet keeps its address)', C.hexEncode(C.deriveKey(hot, 0)), C.hexEncode(hot)],
  ['deriveKey(seed=[7;32], 1)', C.hexEncode(k1), '50daba6867e74718e73d9f282f3980df8d7f2f04e5849a655aed67dcdf2dd63b'],
  ['root of key 1', C.hexEncode(root1), '5e3e12e768494c491e23c7838a17315520f1e53b327ef9b43cbe5b8e7fa42dca'],
  ['keyOpDigest(id, rotate, 0x7e57, gen 0, root1, 300)', C.hexEncode(rotDigest),
    '4fdf5ffb1dc0f2b9becf130282a16e9e0b43af03ba3f4c225121f8f6d46ae597'],
  ['keyOpDigest(id, cancel, 0x7e57, gen 5, zeros, 65535)', C.hexEncode(C.keyOpDigest(id, C.OP_CANCEL, 0x7e57, 5, new Uint8Array(32), 65535)),
    '71c7833770367d9344de35ee4211b8c355f25246147d7477887e909abdb232ad'],
  ['blake2b-512 of the rotation signature at 300', rotHash.digest('hex'),
    '6142885ad721bec2d17c0a43066851591a28d0c1fdb62b5162c1aadf11ec8144d08ec126e8372b32c0ff680df007e6bb5567c074f4f1071d15b6176db64960f9'],
);

let ok = true;
const check = (name, pass) => { ok = ok && pass; console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}`); };
for (const [name, got, want] of cases) {
  check(name, got === want);
  if (got !== want) { console.log(`  got:  ${got}`); console.log(`  want: ${want}`); }
}

// The pinned signature verifies here (the Rust test verifies the same bytes), and only for its parent.
check('MT signature verifies under the hot root', C.mtVerify(hotRoot, 300, onOne, sig));
check('a re-pointed parent is rejected', !C.mtVerify(hotRoot, 300, C.contributionDigest(id, 300, 2, data), sig));
check('a stripped parent is rejected', !C.mtVerify(hotRoot, 300, rootDigest, sig));
check('a different index is rejected', !C.mtVerify(hotRoot, 301, onOne, sig));
check('a foreign root is rejected', !C.mtVerify(C.mtKeygenRoot(cold), 300, onOne, sig));

// A fresh random wallet: sign at the last leaf of subtree 0 and the first of subtree 1.
const master = C.randomSeed();
const mc = C.mtCache(master);
const freshId = C.identity(mc.root, C.coldCommit(C.mtKeygenRoot(C.randomSeed())));
for (const g of [255, 256]) {
  const m = C.contributionDigest(freshId, g, null, C.utf8('a fresh idea ' + g));
  check(`fresh-wallet sign and verify at index ${g}`, C.mtVerify(mc.root, g, m, C.mtSign(master, g, m, mc)));
}

check('the rotation signature verifies under key 0, and not under key 1', C.mtVerify(hotRoot, 300, rotDigest, rotSig) && !C.mtVerify(root1, 300, rotDigest, rotSig));
check('a rotation signature does not verify as another generation', !C.mtVerify(hotRoot, 300, C.keyOpDigest(id, C.OP_ROTATE, 0x7e57, 1, root1, 300), rotSig));

console.log(ok ? '\nALL PARITY CHECKS PASS' : '\nPARITY FAILURE');
process.exit(ok ? 0 : 1);
