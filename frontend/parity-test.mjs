// Browser-crypto parity test: proves frontend/crypto.js is byte-identical to the Rust
// noesis_core::{lamport, xmss}. The SAME vectors are pinned in the Rust test
// `rpc::tests::xmss_parity_vectors_are_pinned` — if either side changes its hashing, one of the two
// tests fails. Run: `node frontend/parity-test.mjs`  (exit 0 = parity holds).
import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const here = dirname(fileURLToPath(import.meta.url));
eval(readFileSync(join(here, 'crypto.js'), 'utf8'));
const C = globalThis.NoesisCrypto;

const seed = new Uint8Array(32).fill(7);
const addr = C.keygenAddress(seed);

const cases = [
  ['XMSS address(seed=[7;32])', C.hexEncode(addr),
    'b8034627416d512d88c00d9cda4dfe0d1edb513102674e018c4c02732a34612e'],
  ['contribution_digest(addr,5,"hello noesis")',
    C.hexEncode(C.contributionDigest(addr, 5, C.utf8('hello noesis'))),
    '01dd86bf1b750baaefabf0b09d274eac98d2026b734b0b28f367f4b68a410f6d'],
];

let ok = true;
for (const [name, got, want] of cases) {
  const pass = got === want;
  ok = ok && pass;
  console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}`);
  if (!pass) { console.log(`  got:  ${got}`); console.log(`  want: ${want}`); }
}

// A full sign→verify roundtrip on a fresh random wallet: sign at a leaf, then run the SAME check the
// Rust node runs (lampVerify + fold the auth path back to the address). Also confirm tamper-rejection.
const master = C.randomSeed();
const leaves = C.buildLeaves(master);
const A = C.keygenAddress(master);
const msg = C.contributionDigest(A, 3, C.utf8('a fresh idea'));
const sig = C.xmssSign(master, 3, msg, leaves);

const good = C.xmssVerify(A, msg, sig);
console.log(`${good ? 'PASS' : 'FAIL'}  fresh-wallet sign→verify roundtrip`);
ok = ok && good;

const badMsg = C.contributionDigest(A, 3, C.utf8('swapped data'));
const tamperRejected = !C.xmssVerify(A, badMsg, sig);
console.log(`${tamperRejected ? 'PASS' : 'FAIL'}  tampered message rejected`);
ok = ok && tamperRejected;

const foreign = C.keygenAddress(C.randomSeed());
const foreignRejected = !C.xmssVerify(foreign, msg, sig);
console.log(`${foreignRejected ? 'PASS' : 'FAIL'}  foreign address rejected`);
ok = ok && foreignRejected;

console.log(ok ? '\nALL PARITY CHECKS PASS' : '\nPARITY FAILURE');
process.exit(ok ? 0 : 1);
