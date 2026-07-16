// Noesis wallet crypto — REAL post-quantum hash-based keys, in the browser. No theater: this is a
// faithful port of the Rust `noesis_core::{lamport, xmss}` (byte-for-byte, verified against Rust test
// vectors — see the parity check). A wallet is a hash-based keypair; the address is a Merkle root over
// Lamport one-time public keys; every contribution is signed and the node verifies it.
//
// Works in the browser (served by the node at /crypto.js) AND in Node (for the parity test). Pure
// Uint8Array in/out; the only host dependency is `crypto.getRandomValues` for seeding.
;(function (root) {
  'use strict';

  // ============ blake2b-256 with personalization (matches Rust blake2b_ref) ============
  // Standard BLAKE2b (RFC 7693), 32-byte output, no key, no salt, 16-byte personalization folded into
  // the parameter block (bytes 48..63) exactly as `Blake2bBuilder::new(32).personal(p)` does.
  var BLAKE2B_IV32 = new Uint32Array([
    0xf3bcc908, 0x6a09e667, 0x84caa73b, 0xbb67ae85, 0xfe94f82b, 0x3c6ef372, 0x5f1d36f1, 0xa54ff53a,
    0xade682d1, 0x510e527f, 0x2b3e6c1f, 0x9b05688c, 0xfb41bd6b, 0x1f83d9ab, 0x137e2179, 0x5be0cd19
  ]);
  var SIGMA8 = [
    0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15, 14,10,4,8,9,15,13,6,1,12,0,2,11,7,5,3,
    11,8,12,0,5,2,15,13,10,14,3,6,7,1,9,4, 7,9,3,1,13,12,11,14,2,6,5,10,4,0,15,8,
    9,0,5,7,2,4,10,15,14,1,11,12,6,8,3,13, 2,12,6,10,0,11,8,3,4,13,7,5,15,14,1,9,
    12,5,1,15,14,13,4,10,0,7,6,3,9,2,8,11, 13,11,7,14,12,1,3,9,5,0,15,4,8,6,2,10,
    6,15,14,9,11,3,0,8,12,2,13,7,1,4,10,5, 10,2,8,4,7,6,1,5,15,11,9,14,3,12,13,0,
    0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15, 14,10,4,8,9,15,13,6,1,12,0,2,11,7,5,3
  ];
  var SIGMA82 = new Uint8Array(SIGMA8.map(function (x) { return x * 2; }));
  var v = new Uint32Array(32), m = new Uint32Array(32);

  function ADD64AA(a, i, b) {
    var o0 = a[i] + a[b], o1 = a[i + 1] + a[b + 1];
    if (o0 >= 0x100000000) o1++;
    a[i] = o0; a[i + 1] = o1;
  }
  function ADD64AC(a, i, b0, b1) {
    var o0 = a[i] + b0; if (b0 < 0) o0 += 0x100000000;
    var o1 = a[i + 1] + b1; if (o0 >= 0x100000000) o1++;
    a[i] = o0; a[i + 1] = o1;
  }
  function B2B_GET32(arr, i) {
    return arr[i] ^ (arr[i + 1] << 8) ^ (arr[i + 2] << 16) ^ (arr[i + 3] << 24);
  }
  function B2B_G(a, b, c, d, ix, iy) {
    var x0 = m[ix], x1 = m[ix + 1], y0 = m[iy], y1 = m[iy + 1];
    ADD64AA(v, a, b); ADD64AC(v, a, x0, x1);
    var xor0 = v[d] ^ v[a], xor1 = v[d + 1] ^ v[a + 1];
    v[d] = xor1; v[d + 1] = xor0;
    ADD64AA(v, c, d);
    xor0 = v[b] ^ v[c]; xor1 = v[b + 1] ^ v[c + 1];
    v[b] = (xor0 >>> 24) ^ (xor1 << 8); v[b + 1] = (xor1 >>> 24) ^ (xor0 << 8);
    ADD64AA(v, a, b); ADD64AC(v, a, y0, y1);
    xor0 = v[d] ^ v[a]; xor1 = v[d + 1] ^ v[a + 1];
    v[d] = (xor0 >>> 16) ^ (xor1 << 16); v[d + 1] = (xor1 >>> 16) ^ (xor0 << 16);
    ADD64AA(v, c, d);
    xor0 = v[b] ^ v[c]; xor1 = v[b + 1] ^ v[c + 1];
    v[b] = (xor1 >>> 31) ^ (xor0 << 1); v[b + 1] = (xor0 >>> 31) ^ (xor1 << 1);
  }
  function compress(ctx, last) {
    var i = 0;
    for (i = 0; i < 16; i++) { v[i] = ctx.h[i]; v[i + 16] = BLAKE2B_IV32[i]; }
    v[24] = v[24] ^ ctx.t; v[25] = v[25] ^ (ctx.t / 0x100000000);
    if (last) { v[28] = ~v[28]; v[29] = ~v[29]; }
    for (i = 0; i < 32; i++) m[i] = B2B_GET32(ctx.b, 4 * i);
    for (i = 0; i < 12; i++) {
      B2B_G(0, 8, 16, 24, SIGMA82[i * 16 + 0], SIGMA82[i * 16 + 1]);
      B2B_G(2, 10, 18, 26, SIGMA82[i * 16 + 2], SIGMA82[i * 16 + 3]);
      B2B_G(4, 12, 20, 28, SIGMA82[i * 16 + 4], SIGMA82[i * 16 + 5]);
      B2B_G(6, 14, 22, 30, SIGMA82[i * 16 + 6], SIGMA82[i * 16 + 7]);
      B2B_G(0, 10, 20, 30, SIGMA82[i * 16 + 8], SIGMA82[i * 16 + 9]);
      B2B_G(2, 12, 22, 24, SIGMA82[i * 16 + 10], SIGMA82[i * 16 + 11]);
      B2B_G(4, 14, 16, 26, SIGMA82[i * 16 + 12], SIGMA82[i * 16 + 13]);
      B2B_G(6, 8, 18, 28, SIGMA82[i * 16 + 14], SIGMA82[i * 16 + 15]);
    }
    for (i = 0; i < 16; i++) ctx.h[i] = ctx.h[i] ^ v[i] ^ v[i + 16];
  }

  // Init a 32-byte-output context with a 16-byte personalization (no key, no salt).
  function initCtx(personal16) {
    var ctx = { b: new Uint8Array(128), h: new Uint32Array(16), t: 0, c: 0, outlen: 32 };
    for (var i = 0; i < 16; i++) ctx.h[i] = BLAKE2B_IV32[i];
    // param block bytes 0..3: digest_length | key_length<<8 | fanout<<16 | depth<<24
    ctx.h[0] ^= 0x01010000 ^ 32;
    // param block bytes 48..63 = personalization (words 12..15), little-endian
    for (var j = 0; j < 4; j++) {
      ctx.h[12 + j] ^= (personal16[j * 4] | (personal16[j * 4 + 1] << 8) |
                        (personal16[j * 4 + 2] << 16) | (personal16[j * 4 + 3] << 24)) >>> 0;
    }
    return ctx;
  }
  function update(ctx, input) {
    for (var i = 0; i < input.length; i++) {
      if (ctx.c === 128) { ctx.t += ctx.c; compress(ctx, false); ctx.c = 0; }
      ctx.b[ctx.c++] = input[i];
    }
  }
  function final(ctx) {
    ctx.t += ctx.c;
    while (ctx.c < 128) ctx.b[ctx.c++] = 0;
    compress(ctx, true);
    var out = new Uint8Array(32);
    for (var i = 0; i < 32; i++) out[i] = (ctx.h[i >> 2] >> (8 * (i & 3))) & 0xff;
    return out;
  }

  function personalBytes(str) {
    var p = new Uint8Array(16); // 14 ascii chars + 2 zero bytes = 16
    for (var i = 0; i < str.length; i++) p[i] = str.charCodeAt(i) & 0xff;
    return p;
  }
  var P_LAMP = personalBytes('noesis-lamp-v1');
  var P_XMSS = personalBytes('noesis-xmss-v1');

  // Domain-separated hash: personal + a leading tag byte + concatenated parts (matches Rust `h`).
  function hash(personal16, tag, parts) {
    var ctx = initCtx(personal16);
    update(ctx, Uint8Array.of(tag));
    for (var i = 0; i < parts.length; i++) update(ctx, parts[i]);
    return final(ctx);
  }

  function u32le(n) {
    return Uint8Array.of(n & 0xff, (n >>> 8) & 0xff, (n >>> 16) & 0xff, (n >>> 24) & 0xff);
  }

  // ============ Lamport one-time signature (matches Rust lamport) ============
  var N = 256;
  function secretLeaf(seed, i, b) { return hash(P_LAMP, 0x01, [seed, u32le(i), Uint8Array.of(b)]); }
  function pkLeaf(preimage) { return hash(P_LAMP, 0x02, [preimage]); }
  function lampRoot(table) { // table: array of 2N Uint8Array(32)
    var flat = new Uint8Array(table.length * 32);
    for (var i = 0; i < table.length; i++) flat.set(table[i], i * 32);
    return hash(P_LAMP, 0x03, [flat]);
  }
  function keygenRoot(seed) {
    var table = new Array(2 * N);
    for (var i = 0; i < N; i++) {
      table[2 * i] = pkLeaf(secretLeaf(seed, i, 0));
      table[2 * i + 1] = pkLeaf(secretLeaf(seed, i, 1));
    }
    return lampRoot(table);
  }
  function bit(msg, i) { return (msg[i >> 3] >> (i & 7)) & 1; }
  function lampSign(seed, msg) {
    var sig = new Uint8Array(N * 64);
    for (var i = 0; i < N; i++) {
      var b = bit(msg, i);
      sig.set(secretLeaf(seed, i, b), i * 64);
      sig.set(pkLeaf(secretLeaf(seed, i, 1 - b)), i * 64 + 32);
    }
    return sig;
  }
  function lampVerify(rootCommit, msg, sig) {
    if (sig.length !== N * 64) return false;
    var table = new Array(2 * N);
    for (var i = 0; i < N; i++) {
      var b = bit(msg, i);
      var revealed = pkLeaf(sig.subarray(i * 64, i * 64 + 32));
      var sibling = sig.subarray(i * 64 + 32, i * 64 + 64);
      table[2 * i] = b === 0 ? revealed : sibling;
      table[2 * i + 1] = b === 0 ? sibling : revealed;
    }
    return bytesEqual(lampRoot(table), rootCommit);
  }

  // ============ XMSS multi-use wallet (matches Rust xmss) ============
  var H = 8, LEAVES = 1 << H;
  function xNode(l, r) { return hash(P_XMSS, 0x02, [l, r]); }
  function leafSeed(master, index) { return hash(P_XMSS, 0x01, [master, u32le(index)]); }
  function leafPub(master, index) { return keygenRoot(leafSeed(master, index)); }

  // O(2^H) OTS keygens — compute once, cache the leaves for fast signing.
  function buildLeaves(master) {
    var leaves = new Array(LEAVES);
    for (var i = 0; i < LEAVES; i++) leaves[i] = leafPub(master, i);
    return leaves;
  }
  function rootOf(leaves) {
    var level = leaves;
    while (level.length > 1) {
      var next = new Array(level.length >> 1);
      for (var i = 0; i < next.length; i++) next[i] = xNode(level[2 * i], level[2 * i + 1]);
      level = next;
    }
    return level[0];
  }
  function keygenAddress(master) { return rootOf(buildLeaves(master)); }

  function authPath(leaves, index) {
    var path = new Array(H), level = leaves, idx = index;
    for (var lvl = 0; lvl < H; lvl++) {
      path[lvl] = level[idx ^ 1];
      var next = new Array(level.length >> 1);
      for (var i = 0; i < next.length; i++) next[i] = xNode(level[2 * i], level[2 * i + 1]);
      level = next; idx = idx >> 1;
    }
    return path;
  }
  // Sign with leaf `index`. Pass a cached `leaves` (from buildLeaves) to avoid recomputing the tree.
  function xmssSign(master, index, msg, leaves) {
    leaves = leaves || buildLeaves(master);
    return {
      index: index,
      ots_root: leaves[index],
      auth: authPath(leaves, index),
      ots_sig: lampSign(leafSeed(master, index), msg)
    };
  }
  function foldRoot(index, leafValue, auth) {
    var acc = leafValue;
    for (var lvl = 0; lvl < H; lvl++) {
      acc = ((index >> lvl) & 1) === 0 ? xNode(acc, auth[lvl]) : xNode(auth[lvl], acc);
    }
    return acc;
  }
  // Verify a signature under `address` (the exact check the Rust node runs). Lets the wallet
  // self-check before it spends a one-time leaf.
  function xmssVerify(address, msg, sig) {
    if (sig.index >= LEAVES) return false;
    return lampVerify(sig.ots_root, msg, sig.ots_sig) &&
           bytesEqual(foldRoot(sig.index, sig.ots_root, sig.auth), address);
  }

  // The canonical message a contribution signs — binds address, one-time index, and data.
  var TE = new TextEncoder();
  function contributionDigest(address, index, dataBytes) {
    var dh = hash(P_XMSS, 0x04, [dataBytes]);
    return hash(P_XMSS, 0x03, [address, u32le(index), dh]);
  }

  // ============ helpers ============
  function hexEncode(bytes) {
    var s = '';
    for (var i = 0; i < bytes.length; i++) s += (bytes[i] >>> 4).toString(16) + (bytes[i] & 15).toString(16);
    return s;
  }
  function hexDecode(str) {
    var out = new Uint8Array(str.length >> 1);
    for (var i = 0; i < out.length; i++) out[i] = parseInt(str.substr(i * 2, 2), 16);
    return out;
  }
  function randomSeed() {
    var s = new Uint8Array(32);
    (root.crypto || globalThis.crypto).getRandomValues(s);
    return s;
  }
  function utf8(str) { return TE.encode(str); }
  function bytesEqual(a, b) {
    if (a.length !== b.length) return false;
    var d = 0;
    for (var i = 0; i < a.length; i++) d |= a[i] ^ b[i];
    return d === 0;
  }

  root.NoesisCrypto = {
    H: H, LEAVES: LEAVES,
    keygenRoot: keygenRoot, lampSign: lampSign, lampVerify: lampVerify,
    buildLeaves: buildLeaves, keygenAddress: keygenAddress,
    xmssSign: xmssSign, xmssVerify: xmssVerify,
    contributionDigest: contributionDigest,
    hexEncode: hexEncode, hexDecode: hexDecode, randomSeed: randomSeed, utf8: utf8,
    u32le: u32le, bytesEqual: bytesEqual
  };
})(typeof window !== 'undefined' ? window : globalThis);
