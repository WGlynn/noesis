#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Per-block ownership + signing for the JARVIS governance chain — Bitcoin-shaped.

A "block" is a session-chain block (the unit a session actually PRODUCES). Each
block is locked to an OWNER public key: only the owner's key can produce a valid
block-attestation, and ownership is TRANSFERABLE — the current owner signs a
reassignment to a new pubkey, exactly like spending a UTXO to a new locking key.
Current ownership is NOT stored as a giant table; it is DERIVED from a genesis
owner plus a signed transfer log (the UTXO set is a fold over transaction
history). Start: a single owner (jarvis@local = Will). The model already supports
many owners + transfer; today the registry just points everything at one key.

Standard primitives only — Ed25519 via ssh-keygen, sha256, stdlib json. Additive:
does NOT touch the live chain-write path (chain.py / auto-checkpoint.py).

  genesis              seed the registry (genesis owner = the attestation key)
  owner <id>           print the current owner of block <id>
  attest <id> [key]    current owner signs block <id>   (key defaults to jarvis-attest)
  transfer <id> <pub> [key]
                       current owner signs reassignment of <id> to pubkey file <pub>
  verify <id>          check block <id>'s attestation against its CURRENT owner
"""
import hashlib
import json
import os
import subprocess
import sys
import tempfile
import time

try:
    sys.stdout.reconfigure(encoding="utf-8")
except Exception:
    pass

HOME = os.path.expanduser("~")
SC = os.path.join(HOME, ".claude", "session-chain")
BLOCKS = os.path.join(SC, "blocks")
KEYDIR = os.path.join(HOME, ".claude", "keys")
DEFAULT_KEY = os.path.join(KEYDIR, "jarvis-attest")
SYS = os.path.join(HOME, ".claude", "projects", "C--Users-Will", "memory", "_system")
REGISTRY = os.path.join(SYS, "block_ownership.json")     # genesis + transfer log
ATTEST = os.path.join(SYS, "block_attestations.json")    # block_id -> {owner_fpr, sig}
NS = "jarvis-block"


def _fpr(pub_path):
    r = subprocess.run(["ssh-keygen", "-lf", pub_path], capture_output=True, text=True)
    # "256 SHA256:xxxx comment (ED25519)"
    return r.stdout.split()[1] if r.returncode == 0 else None


def _publine(pub_path):
    return open(pub_path, encoding="utf-8").read().strip()


def _block_path(bid):
    return os.path.join(BLOCKS, bid if bid.endswith(".json") else bid + ".json")


def _block_hash(bid):
    p = _block_path(bid)
    return hashlib.sha256(open(p, "rb").read()).hexdigest()


def _sign(msg, key):
    """ssh-keygen detached signature over msg (exact bytes) with private key `key`."""
    with tempfile.TemporaryDirectory() as td:
        p = os.path.join(td, "m")
        open(p, "w", encoding="utf-8", newline="").write(msg)
        r = subprocess.run(["ssh-keygen", "-Y", "sign", "-f", key, "-n", NS, p],
                           capture_output=True, text=True)
        if r.returncode != 0:
            raise RuntimeError(r.stderr.strip())
        return open(p + ".sig", encoding="utf-8").read()


def _verify(msg, sig, owner_id, pub_line):
    """Verify sig over msg by the key whose pubkey is pub_line, identity owner_id."""
    with tempfile.TemporaryDirectory() as td:
        allowed = os.path.join(td, "allowed")
        open(allowed, "w", encoding="utf-8", newline="\n").write(
            f'{owner_id} namespaces="{NS}" {pub_line}\n')
        sp = os.path.join(td, "m.sig")
        open(sp, "w", encoding="utf-8", newline="").write(sig)
        r = subprocess.run(
            ["ssh-keygen", "-Y", "verify", "-f", allowed, "-I", owner_id, "-n", NS, "-s", sp],
            input=msg, capture_output=True, text=True)
        return r.returncode == 0


def _load(path, default):
    try:
        return json.load(open(path, encoding="utf-8"))
    except Exception:
        return default


def _save(path, obj):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    json.dump(obj, open(path, "w", encoding="utf-8", newline="\n"), indent=2)


def current_owner(bid, reg):
    """Fold the transfer log: last transfer of this block wins, else genesis."""
    owner = reg["genesis_owner"]
    for t in reg.get("transfers", []):
        if t["block_id"] == bid:
            owner = {"id": t["new_id"], "fpr": t["new_fpr"], "pub": t["new_pub"]}
    return owner


def cmd_genesis():
    pub = DEFAULT_KEY + ".pub"
    reg = {"genesis_owner": {"id": "jarvis@local", "fpr": _fpr(pub), "pub": _publine(pub)},
           "transfers": [], "note": "Bitcoin-shaped: current owner = genesis folded over the "
           "signed transfer log. Only the current owner's key can attest or transfer a block."}
    _save(REGISTRY, reg)
    print(f"GENESIS owner=jarvis@local fpr={reg['genesis_owner']['fpr']} — owns all blocks")


def cmd_owner(bid):
    o = current_owner(bid, _load(REGISTRY, {"genesis_owner": {}, "transfers": []}))
    print(f"{bid} owner: {o.get('id')} {o.get('fpr')}")


def cmd_attest(bid, key=DEFAULT_KEY):
    reg = _load(REGISTRY, None)
    if not reg:
        print("no registry — run genesis"); return 1
    owner = current_owner(bid, reg)
    signer_fpr = _fpr(key + ".pub" if not key.endswith(".pub") else key)
    if signer_fpr != owner["fpr"]:
        print(f"✗ REFUSED — you ({signer_fpr}) are not the current owner ({owner['fpr']}) of {bid}. "
              "Only the rights-holder can sign.")
        return 1
    msg = f"{bid}|{_block_hash(bid)}|{owner['fpr']}"
    sig = _sign(msg, key)
    att = _load(ATTEST, {})
    att[bid] = {"owner_fpr": owner["fpr"], "owner_id": owner["id"], "sig": sig,
                "block_hash": _block_hash(bid), "at": int(time.time())}
    _save(ATTEST, att)
    print(f"ATTESTED {bid} by owner {owner['id']} ({owner['fpr']})")


def cmd_transfer(bid, newpub, key=DEFAULT_KEY):
    reg = _load(REGISTRY, None)
    if not reg:
        print("no registry — run genesis"); return 1
    owner = current_owner(bid, reg)
    signer_fpr = _fpr(key + ".pub" if not key.endswith(".pub") else key)
    if signer_fpr != owner["fpr"]:
        print(f"✗ REFUSED — only the current owner ({owner['fpr']}) can transfer {bid}.")
        return 1
    new_fpr, new_pub = _fpr(newpub), _publine(newpub)
    ts = int(time.time())
    rec_msg = f"transfer|{bid}|{owner['fpr']}|{new_fpr}|{ts}"
    sig = _sign(rec_msg, key)            # current owner authorizes the transfer
    reg["transfers"].append({"block_id": bid, "prev_fpr": owner["fpr"],
                             "new_id": "owner:" + new_fpr[:16], "new_fpr": new_fpr,
                             "new_pub": new_pub, "ts": ts, "sig": sig})
    _save(REGISTRY, reg)
    print(f"TRANSFERRED {bid}: {owner['fpr']} → {new_fpr} (authorized by prior owner)")


def cmd_verify(bid):
    reg = _load(REGISTRY, None)
    att = _load(ATTEST, {})
    if not reg or bid not in att:
        print(f"{bid}: no attestation"); return 1
    owner = current_owner(bid, reg)
    a = att[bid]
    if a["owner_fpr"] != owner["fpr"]:
        print(f"⚠ STALE — {bid} was attested by {a['owner_fpr']} but current owner is "
              f"{owner['fpr']} (rights transferred). New owner must re-attest.")
        return 1
    if a.get("block_hash") != _block_hash(bid):
        print(f"⚠ DRIFT — {bid} content changed since attestation."); return 1
    msg = f"{bid}|{_block_hash(bid)}|{owner['fpr']}"
    ok = _verify(msg, a["sig"], owner["id"], owner["pub"])
    print(f"{'OK — ' if ok else '⚠ SIGNATURE INVALID — '}{bid} "
          f"{'attested by current owner ' + owner['id'] if ok else 'forged/keyless'}")
    return 0 if ok else 1


def main():
    a = sys.argv[1:]
    if not a:
        print(__doc__); return 0
    c = a[0]
    if c == "genesis": return cmd_genesis()
    if c == "owner": return cmd_owner(a[1])
    if c == "attest": return cmd_attest(a[1], a[2] if len(a) > 2 else DEFAULT_KEY)
    if c == "transfer": return cmd_transfer(a[1], a[2], a[3] if len(a) > 3 else DEFAULT_KEY)
    if c == "verify": return cmd_verify(a[1])
    print(f"unknown command: {c}"); return 1


if __name__ == "__main__":
    sys.exit(main() or 0)
