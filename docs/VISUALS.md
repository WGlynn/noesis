# Noesis — Visuals (PRIVATE, stealth)

> Diagrams for the Proof-of-Mind value chain. Mermaid (renders on GitHub).
> Grounded in `WHITEPAPER.md`, `CRYPTOECONOMICS.md`, `POM-CONSENSUS.md`,
> `BLOCK-ECONOMY-SPEC.md`, `COORDINATION-SCHELLING.md`. Keep PRIVATE during stealth.

---

## Fig 1 — The PoM value pipeline (one block, end to end)

How a unit of contribution becomes consensus weight and tradable state.

```mermaid
flowchart LR
  A["Contribution<br/>(block of thought)"] --> B["Commit-reveal<br/>hash(block ‖ secret)"]
  B --> C["Temporal-novelty value<br/>(strategyproof: sybil/padding/collusion → 0)"]
  C --> D["× learned quality<br/>value = novelty × (1 + q)"]
  D --> E["Myerson value<br/>(intra-block co-authors, synergy)"]
  E --> F["PoM-standing<br/>SOULBOUND — consensus weight + right to mint"]
  E --> G["State-bytes<br/>TRANSFERABLE — 1 PoM = 1 byte"]
  F --> H["Consensus<br/>(PoM-weighted)"]
  G --> I["Medium of exchange<br/>buy storage, not consensus"]
  F -.decay (state-rent sink).-> J["Reclaim<br/>stale standing decays"]
  classDef sb fill:#1f2937,stroke:#60a5fa,color:#e5e7eb;
  classDef tr fill:#1f2937,stroke:#34d399,color:#e5e7eb;
  class F sb
  class G tr
```

---

## Fig 2 — Two-cell mint: soulbound standing vs transferable capacity

The split that makes "can't be bought" real while still giving a medium of exchange.

```mermaid
flowchart TD
  M["Verified contribution<br/>(temporal-novelty × quality)"] --> SPLIT{{"Two-cell mint"}}
  SPLIT --> STAND["STANDING cell<br/>SOULBOUND<br/>franchise = consensus + right-to-mint<br/>type-script invariant: no owner reassignment"]
  SPLIT --> CAP["CAPACITY cell<br/>TRANSFERABLE<br/>1 byte of state, rides the ownership fold"]
  STAND --> CW["Consensus weight<br/>(keyed by contributor, NOT owner)"]
  CAP --> EX["Trades freely<br/>(commodity: state)"]
  STAND -. cannot move .-> X["✗ sell consensus<br/>(would collapse to PoS)"]
  classDef sb fill:#1f2937,stroke:#60a5fa,color:#e5e7eb;
  classDef tr fill:#1f2937,stroke:#34d399,color:#e5e7eb;
  classDef no fill:#3f1d1d,stroke:#f87171,color:#fecaca;
  class STAND,CW sb
  class CAP,EX tr
  class X no
```

---

## Fig 3 — Three powers = rock-paper-scissors equilibrium

Why exactly three: 2 → binary dominance (capture); 3 → non-transitive, capture-resistant; 4+ → coalitions. Separation of powers (Tinbergen: one instrument per function).

```mermaid
flowchart LR
  subgraph RPS["Capture-resistant cycle (no dominant strategy)"]
    COG["COGNITION<br/>PoM-standing (soulbound)<br/>Proof of Mind"] -->|checks| CAP["CAPITAL<br/>state-bytes<br/>Proof of state-stake"]
    CAP -->|checks| COMP["COMPUTE<br/>JUL (PoW, money layer)<br/>Proof of Work"]
    COMP -->|checks| COG
  end
  GOV["VIBE — governance<br/>(separate instrument)"] -.orthogonal.-> RPS
  note["JUL NOT yet integrated (honest open item).<br/>Core today = soulbound PoM + transferable bytes; no PoW needed for consensus/state."]
  RPS -.-> note
```

---

## Fig 4 — Consensus stack

```mermaid
flowchart TD
  CHAIN["Tamper-evident signed owned chain<br/>(the ledger)"] --> W["PoM-weighted agreement<br/>(stake = accumulated Myerson value)"]
  W --> STAB["Core / nucleolus stability<br/>no validator coalition profits by deviating"]
  STAB --> FIN["Finalization"]
  FIN -. fallback .-> NI["Nakamoto-Infinity<br/>liveness fallback"]
  SLASH["Slashing"] -.-> CHAIN
  SLASH --> S1["invalid reveal"]
  SLASH --> S2["refuted value (dispute window)"]
```

---

## Fig 5 — The coordination Schelling point: inward + outward (same fold, two radii)

The deployment thesis. The *same* reconciliation primitive yields a coherent self and a coherent network. See `COORDINATION-SCHELLING.md`.

```mermaid
flowchart TD
  subgraph INWARD["INWARD consensus — one mind"]
    direction LR
    s1["context / drafts"] --> J1["JARVIS fold<br/>(WWWD / ETM)"]
    s2["sub-agents"] --> J1
    s3["memory"] --> J1
    J1 --> SELF["one coherent will"]
  end
  subgraph OUTWARD["OUTWARD consensus — many minds"]
    direction LR
    SELF --> P["PoM commit-reveal"]
    n2["other node"] --> NET["PoM-weighted<br/>network consensus"]
    n3["other node"] --> NET
    P --> NET
  end
  MID["JARVIS = in the middle on BOTH sides<br/>(honest broker, augments the invariant)"]
  MID -.-> INWARD
  MID -.-> OUTWARD
  E1["Edge 1: shared PROTOCOL, not shared instance<br/>(else centralization, not consensus)"]
  E2["Edge 2: openness + neutrality = what makes it focal<br/>(dishonest-in-the-middle = not-focal)"]
  OUTWARD --- E1
  OUTWARD --- E2
```

---

## Fig 6 — Fair launch: genesis-burn vs chain-reset

Will's open question. Recommendation = genesis-burn (provable > asserted). See `COORDINATION-SCHELLING.md`.

```mermaid
flowchart TD
  Q{{"At launch: neutralize creator's<br/>pre-launch advantage"}}
  Q --> R["RESET the chain"]
  Q --> BURN["GENESIS-BURN pre-launch blocks"]
  R --> R1["history gone<br/>outsiders must TRUST nothing was kept"]
  R1 --> R2["fair launch = CLAIM<br/>(trust-me ≠ Schelling point)"]
  BURN --> B1["chain continuous from genesis<br/>pre-launch blocks EXIST (auditable)"]
  B1 --> B2["PoM-standing + value burned to 0<br/>at the launch height, on-chain"]
  B2 --> B3["fair launch = PROOF<br/>(dissolves hidden-premine class)"]
  classDef rec fill:#14302a,stroke:#34d399,color:#d1fae5;
  classDef weak fill:#3f1d1d,stroke:#f87171,color:#fecaca;
  class B1,B2,B3 rec
  class R1,R2 weak
```

---

## Fig 7 — ToM → ETM → PoM (closing the airgap)

```mermaid
flowchart LR
  TOM["Theory of Mind<br/>INFERENCE: guess if another mind is trustworthy<br/>(the airgap)"] --> ETM["Economic Theory of Mind<br/>mind = an economy whose outputs carry value"]
  ETM --> POM["Proof of Mind<br/>verifiable economic PROOF of that economy<br/>(airgap closed: recompute, don't infer)"]
  POM --> RES["Trust = structural property,<br/>not a per-mind guess"]
```

---

## Fig 8 — Mint ↔ sink conservation (why supply closes)

```mermaid
flowchart LR
  CONTRIB["Novel verified contribution"] -->|MINT| LIVE["Live PoM / state-bytes"]
  LIVE -->|DECAY = state-rent sink| RECLAIM["Reclaimed capacity"]
  RECLAIM -.-> COMMONS["State commons<br/>(re-allocatable)"]
  COMMONS -.-> CONTRIB
  note2["Endogenous mint needs a sink.<br/>Decay bounds total live PoM and forces ongoing contribution to retain state."]
  LIVE -.-> note2
```
