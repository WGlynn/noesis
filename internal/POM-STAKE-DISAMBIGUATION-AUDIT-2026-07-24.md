# PoM-standing vs state-rent-stake disambiguation audit — 2026-07-24

Correctness-critical readability sweep. Goal: every passage must let a reader tell the
SOULBOUND contribution franchise (PoM-standing) apart from the TRANSFERABLE capital axis
(state-rent stake / state-bytes). Prose/comments only; no code logic changed. NOT committed.

## 1. Confirmed ground-truth token model (read from code)

The prompt's model is CONFIRMED, no corrections. Code anchors:

- **PoM standing = SOULBOUND, non-transferable contribution franchise.**
  `node/src/lib.rs:596-608` — `mod soulbound`: the standing cell "carries the franchise and
  cannot move. Consensus reads `Standing.contributor`, never byte-ownership."
  `node/src/lib.rs:639-669` — `soulbound::apply` / `valid_transition`: reassigning owner or
  contributor is REJECTED ("that rejection IS the soulbound guarantee").
  `node/src/lib.rs:254-259` — attribution keyed by `type_script.args` (soulbound identity),
  NOT `lock.args`; explicitly "resolves the POM-CONSENSUS 'transferable credit' vs
  CRYPTOECONOMICS 'soulbound standing' contradiction in favor of the latter."
  Tests `lib.rs:751-770` prove reassigning owner/contributor is rejected (non-transferable).

- **State-rent stake = CKB-native PoS capital, TRANSFERABLE.**
  `node/src/lib.rs:139-141` — the cell's `lock.args` = "TRANSFERABLE component: byte-capacity
  ownership ... Changes on every transfer." `lib.rs:606-608` — the "freely-tradable capacity
  cell (state-bytes = money)" is the transferable half of the two-cell mint.
  `node/src/chainspec.rs:38-39` — bonded PoS validators carry finality weight via
  `staked_balance`; `pom` starts 0 and is sourced live.

- **PoM must NOT mint PoS / capital must not buy franchise (anti-plutocracy invariant).**
  `node/src/amendment.rs:317-324` — `check_mix`: `pos ≤ pom` enforced (capital's share may
  NEVER exceed contribution's). `node/src/reserve.rs:55` — "funding buys NOTHING — no PoM
  standing, no PoS weight, no governance voice." `docs/DESIGN-jul-money-layer.md:150-152` +
  the `money_never_buys_standing` test make it executable.

- **JUL = PoW money layer, transferable.** `node/src/jul.rs:9,161` — JUL settles as a
  transferable `Fungible` cell. **VIBE = governance** (separate; `amendment.rs:197`).

- **Consensus vs finality mixes (not asserted from memory — read):**
  NCI mix `Mix{pow:0.10, pos:0.30, pom:0.60}` at `node/src/lib.rs:3705` (`MIN_STAKE` at 3813).
  Note: the global CLAUDE.md pointer `lib.rs:3289` is stale per
  `internal/HANDOFF-2026-07-10...md:22`; the live constant is at 3705. I did not touch that
  (it is a memory-file pointer, out of scope), but flag it.

**Overall finding:** the repo is already ~95% correctly disambiguated. The two-cell
soulbound/transferable split is described correctly and color-coded in nearly every doc and
diagram. The residual errors were concentrated in (a) the "PoM is the stake" consensus idiom
(where "stake" is the exact conflation word), and (b) a few bare "1 PoM = 1 byte" labels
sitting on a node marked TRANSFERABLE, which in isolation could read as "PoM is transferable."

## 2. Edits MADE

| File | Before → After |
|---|---|
| `docs/WHITEPAPER.md:228` | "PoM is the stake." → "soulbound PoM-standing (never the transferable state-rent stake) is what validators are weighted by." |
| `docs/WHITEPAPER.md:237` (mermaid) | "(stake = accumulated Myerson value)" → "(weight = accumulated Myerson value in soulbound standing)" |
| `docs/WHITEPAPER.md:77` (mermaid) | "State-bytes ... 1 PoM = 1 byte" → "... 1 standing unit mints 1 byte" (label sat on the TRANSFERABLE node) |
| `docs/POM-CONSENSUS.md:39` | "PoM is the stake" → "soulbound PoM-standing (not the transferable state-rent stake) is what carries consensus weight" |
| `docs/POM-CONSENSUS.md:30` | "**Stake = demonstrated mind.** The thing at risk is your accumulated proof..." → "**What is at risk = demonstrated mind.** The thing slashed is your accumulated soulbound standing..." |
| `docs/whitepaper/noesis-whitepaper.tex:567` | "PoM is the stake" → "soulbound PoM-standing (never the transferable state-rent stake) is what carries consensus weight" |
| `docs/VISUALS.md:20` (mermaid) | "State-bytes ... 1 PoM = 1 byte" → "... 1 standing unit mints 1 byte" (TRANSFERABLE node) |
| `docs/CRYPTOECONOMICS.md:122` (table) | "state-bytes (PoM-minted...) \| (minted by PoM)" → "(minted by PoM-standing...) \| (minted by PoM-standing)" |
| `docs/CRYPTOECONOMICS.md:147` (header) | Added "— NOT ADOPTED; see 'Bridge to a tradable unit (RESOLVED)' below" + a 3-line note that this superseded proposal used bare "PoM" for the tradable byte and the canonical model does not. (Did NOT rewrite the historical proposal body.) |
| `docs/TOKENOMICS.md:61` (table) | "state-bytes (1 PoM = 1 byte)" → "state-bytes (1 standing unit mints 1 byte)" |
| `internal/STUDY-GUIDE-TOKEN-FLOW.md:20` (table) | "1 PoM = 1 byte" → "1 unit of standing mints 1 byte" |
| `STUDY-GUIDE.md:18` | "PoM is the right to occupy it. Your accumulated PoM is your state budget." → "soulbound PoM-standing is the right to occupy it (the transferable state-bytes it mints are your state budget, never the standing itself)." |

12 edits across 9 files.

## 3. FLAGGED for Will (NOT touched — judgment call, not a clear referent)

1. **`docs/CRYPTOECONOMICS.md:147-169` (proposal block).** This is a clearly-labeled,
   explicitly-superseded PROPOSAL ("DIVERGES from implemented NCI", line 161) where Will
   proposed making PoM itself the "contribution-token-AND-state-byte" (i.e. bare "PoM" as
   the tradable unit at lines 167-169: "PoM=byte is TRADABLE"). Line 176 RESOLVES it against
   that in favor of the soulbound model. I added a NOT-ADOPTED banner + note at the header but
   did NOT rewrite the proposal body, because editing it would misrepresent what was proposed.
   Decision for Will: keep as history (current state), or delete the dead proposal entirely.

2. **`docs/CRYPTOECONOMICS.md:1` title + `docs/TOKENOMICS.md:72-74` + `docs/whitepaper/noesis-cryptoeconomics.md:101` + `internal/SYSTEM-MAP.md:20` + several `internal/CONTINUE.md` lines** all use the house shorthand **"1 PoM = 1 byte."** In each, the surrounding text DEFINES the shorthand (CRYPTOECONOMICS.md:12 explicitly: "Shorthand '1 PoM = 1 byte' means this 1:1 mint right, not that standing itself is the tradable byte"). Left as-is because they are defined-in-place and it is the established repo shorthand. If Will wants zero-ambiguity everywhere, a global "1 PoM = 1 byte" → "1 standing unit mints 1 byte" pass would do it — but that is a style call, not a correctness error, so I did not force it.

3. **`docs/whitepaper/noesis-whitepaper.tex:754`** "one byte of transferable on-chain state ... Standing is the mint" — reads correctly in context (standing mints the transferable byte) but the sentence is dense; possible future polish, not an error.

## 4. Coverage

- Swept: all `*.md` under root, `docs/`, `docs/whitepaper/`, `docs/research/`, `internal/`,
  `patent/`; `README.md`; both `docs/*.tex` (`noesis-whitepaper.tex`, `cybernetics-economic-layer.tex`);
  and `//`/`//!` doc-comments in `node/src/*.rs`.
- **Code doc-comments: clean.** Every `node/src` comment already draws the soulbound (type_script /
  contributor) vs transferable (lock / owner byte) line correctly (esp. `lib.rs:139-146,254-259,596-608`).
  No comment edits were needed.
- **`docs/dist/` (HTML/PDF/TXT) NOT edited** — these are GENERATED artifacts from the `docs/*.md`
  sources. They now lag the edited sources (`NOESIS-LITEPAPER`, `NOESIS-ONEPAGER`, `NOESIS-FAQ`,
  `NOESIS-FOR-DUMMIES` were NOT among the edited files, so dist for those is still coherent; but
  any regeneration should re-run the dist build to stay in sync).
- **`docs/whitepaper/noesis-whitepaper.tex` was edited** — per repo convention a substantive WP
  change bumps `\date{Version ...}` and rebuilds a dated Desktop PDF via
  `docs/whitepaper/build-wp.sh`. I judged this a readability/disambiguation edit (not a claim
  change) and, per the task's "do not commit/build — Will reviews the diff," did NOT bump the
  version or rebuild. **If Will considers it substantive: bump the version + run build-wp.sh.**

No files were left unread within scope.
