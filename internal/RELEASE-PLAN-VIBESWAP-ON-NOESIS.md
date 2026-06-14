# RELEASE PLAN — VibeSwap on Noesis (PRIVATE, stealth, front-run-sensitive)

> **SEALED 2026-06-13** (Will: *"then we can ultimately release vibeswap ON noesis"* +
> *"seal that plan of perfection"*). This is the master sequencing doc that ties the private
> Noesis chain to the public VibeSwap protocol.
>
> **LEAK BOUNDARY (hard):** The *convergence thesis* — that VibeSwap deploys ON Noesis — is the
> front-run-sensitive crown jewel. It stays in THIS repo only. The four VibeSwap critique surfaces
> below are about the PUBLIC repo and may be remediated publicly **as ordinary protocol hardening**,
> but no public commit, doc, or PR may name Noesis / Noeum / the PoM-value-chain framing or imply a
> sovereign-chain deploy target until the head-start is banked. The sweep agent already confirmed
> codename leakage in 30 public files (`JarvisxWill` / `nda-locked`) — that scrub is a Will-gated
> prerequisite, tracked below as GATE 0.

---

## 0. The thesis, in one paragraph

VibeSwap is the **mechanism layer** (commit-reveal batch auction, uniform clearing price, Shapley
distribution, MEV-dissolution). Noesis is the **substrate** — a sovereign, CKB-shaped, Proof-of-Mind
chain. Today VibeSwap is specified in Solidity and prototyped on EVM/Vercel; its CKB cells are
spec-only. The endgame is to release VibeSwap **as application-layer cells on Noesis**, so the DEX's
fairness guarantees stop resting on EVM assumptions and instead inherit Noesis's consensus-sourced
ordering (the same `commit_order` + `index_rule` work shipped this session). One chain, two faces:
Noesis secures *what is true*; VibeSwap is *what people do with it*. "Release VibeSwap on Noesis" =
the mechanism stops being a contract on someone else's chain and becomes native to ours.

The release is **gated**, not dated. Each gate has an exit criterion. Nothing downstream ships until
its upstream gate is green. Phase 1 of the Noesis roadmap (un-gameable `v(S)`) gates everything,
because a DEX whose fairness rests on a gameable measurement is theater.

---

## 1. The release pipeline (ordered, gated)

```
GATE 0  Leak hygiene            → public repo carries zero private codenames; history decision made
GATE 1  Noesis Phase 1          → un-gameable v(S) (the moat) — learned-model bound, garbage gap closed
GATE 2  Noesis Phase 3 on-VM    → ordering + finalization ported to the type-script (deploy-coupled)
GATE 3  VibeSwap contract harden → the cross-chain HIGH findings + upgrade-safety closed
GATE 4  Frontend loss-of-funds  → secret storage, approvals, bridge-credit, settlement-confirm closed
GATE 5  Accessibility + honesty → WCAG, disclosure, demo-vs-real labeling closed
GATE 6  VibeSwap cells on Noesis → mechanism cells deploy on the Noesis testnet, e2e under ckb-vm
GATE 7  Matured release         → external critique pass, head-start banked, public reveal
```

GATE 1 and GATE 2 are the **Noesis roadmap** (ROADMAP.md). GATE 3–5 are the **VibeSwap critique
backlog** (this session's four surveys, §2 below) and can run **in parallel** with GATE 1–2 because
they harden the public reference even if the eventual home is Noesis. GATE 6 is where the two tracks
join: the hardened mechanism gets reinterpreted as cells and deployed on the chain. GATE 7 is the
reveal.

---

## 2. VibeSwap critique backlog (captured 2026-06-13 — four decisive surveys)

These are the public-repo findings. Severity is honest-calibrated (spec-only / demo-mode noted).
Each becomes a remediation ticket behind its gate. **The fix work is public; the fact it serves a
Noesis deploy is private.**

### GATE 3 — Contracts (`contracts/core/`)
- **HIGH** `CommitRevealAuction.sol` — cross-chain reveal copies `_xChainRecipients[commitId]` to the
  settlement recipient with no `address(0)` / validity check ⇒ output can settle to the zero address.
  Fix: reject `destinationRecipient == address(0)` at commit; treat unset as "use default."
- **HIGH** `CommitRevealAuction.sol` — `commitOrderCrossChain` skips the `ESTIMATE_TOLERANCE_X`
  validation regular commits enforce (`estimatedTradeValue = 0`), so reveal-time estimate checks
  silently no-op; only the TRP-R46-F02 collateral check backstops. Fix: require+validate a cross-chain
  estimate, or document the collateral floor as the explicit single invariant with a code comment.
- **MEDIUM** `VibeSwapCore.sol` — `initializeC39Migration` (reinitializer(2)) is not enforced by the
  UUPS path; an `upgradeTo` that forgets it leaves `c39SecurityDefaultsInitialized == false` with no
  retry. Fix: guard `_authorizeUpgrade` to revert when defaults are uninitialized.
- **MEDIUM** `CommitRevealAuction.sol` — priority-bid tiebreak sort swaps on a higher index (later
  reveal) when bids tie ⇒ later reveals win, inverting the documented earlier-reveal-wins fairness.
  Fix: flip the comparison to `sorted[j] < sorted[j+1]`.
- **MEDIUM** `CommitRevealAuction.sol` — `_revealWithPoW` refunds excess to `msg.sender`; for
  cross-chain reveals `msg.sender` is the settler, not the depositor ⇒ silent fund redirection.
  Fix: refund the original depositor, or reject PoW reveals on cross-chain commits.
- **MEDIUM** settlement block-tracking: `settleBatch` reverts if `advancePhase` was never called
  (`revealEndBlock == 0`), and the >256-block fallback uses `prevrandao` (validator-influenceable).
  Fix: default `revealEndBlock = block.number - 1` in `settleBatch`; harden the entropy fallback.
- LOW: `advancePhase` redundant-call gas griefing; immutability-claim unguarded on pool config;
  cross-chain identity coupling fragility (safe today, refactor-fragile).

### GATE 4 — Frontend loss-of-funds + broken flows (`frontend/`)
- **CRITICAL** `useCKBContracts.jsx` — commit-reveal **secrets stored as plaintext in localStorage**
  (`ckb_secret_{batchId}`). XSS ⇒ forge reveals ⇒ steal deposits. Fix: keep secrets in the in-memory
  `secretsRef` only; persist the commitment hash, never the secret; clear after reveal.
- **HIGH** `useBridge.jsx:180` — token approval uses `MaxUint256` (unbounded). Fix: approve exactly
  `amountWei`.
- **CRITICAL (flow)** `useBridge.jsx` — demo bridge deducts source balance but never credits
  destination ⇒ user sees funds vanish. Fix: credit destination (or drop mock balance writes and show
  a pending state until real confirmation).
- **HIGH** `useBridge.jsx` — gas estimate silently falls back to stale demo constants on RPC failure;
  bridge "completed" fires on a timeout, not destination confirmation. Fix: surface fallback as a
  banner; poll the destination event before marking complete.
- **MEDIUM** WebAuthn key derivation (`useDeviceWallet.jsx`) derives an Ethereum key from the public
  credential id via keccak — architecturally unsound (passkey is a signer, not a KDF seed). Fix:
  random seed at creation, sealed by the passkey, decrypt-on-sign.

### GATE 5 — Accessibility + honesty drift
- **HIGH** WCAG: `black-200` (#646464) body text on black fails contrast; matrix-green is borderline.
  Fix: white body text, reserve `#00ff41` for accent.
- **HIGH** missing `aria-label`/`<label>` on swap controls and slippage input. Fix: label every
  interactive element.
- **MEDIUM** stale-price feed proceeds silently with fallback prices; no live-data warning. Fix: stale
  banner.
- **MEDIUM** no pre-commit disclosure of the batch delay or the 0% protocol-fee claim on the main swap
  form. Fix: a disclosure row before the swap button.
- **MEDIUM/CRITICAL (honesty)** balance-fetch failure returns silent `0` (looks like empty wallet);
  CKB wallet shows full bridge UI while internally demo-only; demo banner sits below the fold and
  under-states that outcomes are fictional. Fix: explicit error/loading states; gate the CKB Send
  button; pin + sharpen the demo banner.
- README honesty drift: paper/audit counts stale vs. disk. Fix: verify counts in a pre-release pass.

> **Wallet-security verdict (honest):** private-key handling is actually SOUND — keys are computed
> on-demand, never stored or logged. The real loss-of-funds vector is the **commit-reveal secret in
> localStorage**, not key exposure. Calibrate the narrative to that.

---

## 3. The Noesis-side gates (from ROADMAP.md, restated as release gates)

- **GATE 1 (Phase 1 — the moat):** the learned `v(S)` must preserve strategyproofness. Status: intake
  novelty + similarity + semantic floors shipped; v5/v6/v7 flow+identity+seed gates shipped; dispute
  slashing + escalation court shipped; the learned outcome-evaluator is role-BOUNDED (advance timing
  + dispute evidence, never the verdict). Remaining: real outcome-label data; the structured-but-
  valueless out-of-band frontier (acknowledged research-open, contained economically). **Exit:** the
  adversarial-gaming loop (RSAW) produces no new in-gate class for K consecutive rounds.
- **GATE 2 (Phase 3 — on-VM):** ordering + finalization run inside the type-script. Status: index-dep
  binding ported (exit 23, sentinel-inert pre-deploy); T1–T8 execution tier done; **commit-order now
  wired into `index_rule` reference-side (this session)**; on-VM ordering port + finalization mirror
  (`ON-VM-FINALIZATION.md`) DESIGNED, deploy-coupled. **Exit:** header-sourced height + reveal-sourced
  XOR seed + canonical-order exit code execute on ckb-vm with the activated-path fixtures green.

---

## 4. GATE 6 — the join (VibeSwap cells on Noesis)

The CKB cell specs already exist in the public repo (`contracts-ckb/specs/`: CommitRevealAuction,
VibeAMM, ShapleyDistributor, etc. — REINTERPRET classifications). GATE 6 is where the hardened
mechanism (GATE 3 fixes folded in) is built as **Noesis** cells and deployed on the Noesis testnet,
with the auction's ordering inheriting `commit_order`/`index_rule` natively instead of re-deriving it
in Solidity. **Exit:** a commit→reveal→settle batch executes end-to-end on the Noesis dev chain under
ckb-vm, clearing-price + Shapley split verified against the EVM reference.

---

## 5. GATE 7 — matured release

External-economist + adversarial audit pass on the joined system; head-start banked; THEN the public
reveal that VibeSwap runs on Noesis. Only at GATE 7 does the leak boundary lift.

---

## 6. The seal (invariants this plan will not violate)

1. **Phase 1 gates everything.** No consensus, no DEX deploy, on a measurement that can be gamed.
2. **The convergence is private until GATE 7.** Public hardening is fine; naming the target is not.
3. **Loss-of-funds before polish.** GATE 4's CRITICALs (secret storage, bridge credit) outrank GATE 5.
4. **Honest counts, honest demo labeling.** No marketing number survives a disk check; no fictional
   outcome is presented as real.
5. **Each gate has an exit criterion, not a date.** Done = the criterion is green and adversarially
   checked, not "it looks finished."

> Next concrete move when this plan resumes: pick GATE 1 (RSAW adversarial tick on `v(S)`) or GATE 2
> (on-VM ordering port) on the Noesis side, and/or open the GATE 3 HIGH cross-chain-recipient ticket
> on the public side. The two tracks advance in parallel until GATE 6 joins them.
