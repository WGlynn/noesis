# JARVIS on Noesis — formal design-doc PLAN (PRIVATE, stealth)

> **Status: OUTLINE / plan, not a spec.** This sketches the architecture and the phased
> document set for a full Rust implementation of JARVIS (the agent) running against — and
> eventually *inside* — the Noesis chain. It exists to scope the work and lock the load-bearing
> decisions before any of the constituent specs are written.
>
> **Front-run-sensitive.** The coherent tying-together of (a) the private PoM value chain and
> (b) an agent whose *cognition itself* becomes a deterministic, verifiable on-chain object is
> the krabby patty. Keep in `internal/` (stripped at public-release time, per the CONTINUE flag).
>
> **Design constraint inherited:** Bitcoin-lean. Every component earns its place; prefer
> delete/simplify; one implementation, single-sourced from `noesis-core`. Rigor ≠ bloat.

---

## 0. Thesis (why this is one bet, not two)

Noesis already makes *value* a verifiable on-chain object: Proof-of-Mind scores a contribution's
realized downstream value, commit-reveal binds provenance, and the dispute stack makes hypocrisy
unprofitable. JARVIS is the agent that *produces* contributions. The closing of the loop:

> **If JARVIS runs as a deterministic Rust program, its inference becomes replayable; if its
> inference is replayable, its outputs can be PoM-scored and dispute-bound exactly like any other
> contributor's. The agent's cognition becomes a first-class on-chain object — provenance-of-mind
> in the literal sense.**

This is the same recursion already in the memory stack (VibeSwap-on-EVM = JARVIS-on-Claude = same
pattern): here it is JARVIS-on-Noesis, where the substrate is one we control end-to-end and can
make deterministic. The determinism we need for consensus is *also* what makes the agent
PoM-scoreable. One property, two payoffs.

---

## 1. The load-bearing decision (decide first, everything hangs off it)

**Execute inference as integer-only arithmetic on a deterministic single-threaded VM.**

The entire GPU-determinism research industry exists to fight floating-point non-associativity
`(a+b)+c ≠ a+(b+c)` under batch-variable kernel reduction orders. A RISC-V-VM (CKB-VM style)
execution path *does not have this problem*: one canonical instruction stream, one reduction
order, integer ops that are associative by construction. The cost the GPU world pays (batch-
invariant kernels, ~20–25% throughput) we simply never incur in the consensus path.

Consequences that ripple through every later spec:
- Inference must be **integer / fixed-point end-to-end** (BitNet-ternary native, or I-LLM-style
  integer PTQ of a standard model). No float libm transcendentals — use fixed-point polynomial
  approximations or in-consensus lookup tables for the non-linearities (exp/gelu/softmax). *Bonus:
  lookup-based non-linearity is also what lookup-argument zkVMs (Jolt Atlas) exploit for cheap
  proofs — determinism and provability are the same engineering effort here.*
- Decoding is **greedy (argmax) with explicit tie-break** (lowest token id), or a fully-seeded
  PRNG that is part of consensus state. No temperature/top-p nondeterminism.
- The committed object is the tuple **(weights-hash, tokenizer-hash, runtime-hash, seed)**.
  Determinism is only meaningful relative to a pinned tuple.

If we get this one decision right, both verification paths (§4) fall out of a single binary. If we
get it wrong (float inference off-chain, "trust me" results), the whole PoM-scoreable-cognition
thesis collapses to an oracle.

---

## 2. Architecture (three rings)

```
  ┌─ Ring 0: NOESIS CHAIN (consensus) ───────────────────────────────┐
  │  cells · PoM value gate · commit-reveal · dispute/settlement     │
  │  + verification surface for agent outputs (opML re-exec / zk)    │
  └─────────────────────────────────────────────────────────────────┘
            ▲ commits outputs + provenance        │ scores via PoM
            │                                      ▼
  ┌─ Ring 1: AGENT RUNTIME (deterministic Rust) ─────────────────────┐
  │  integer inference engine (no_std-capable) · tool dispatch ·     │
  │  context/memory as content-addressed cells · ReAct loop          │
  └─────────────────────────────────────────────────────────────────┘
            ▲ pinned (weights,tokenizer,runtime) hashes
            │
  ┌─ Ring 2: OFF-CHAIN EXECUTION (host, fast) ───────────────────────┐
  │  candle / mistral.rs GPU or CPU · same model, same quantization  │
  │  produces results + the replayable trace for Ring 0 to check     │
  └─────────────────────────────────────────────────────────────────┘
```

- **Ring 2** runs the model fast (off-chain, GPU allowed) and emits results plus a replayable
  trace. This is where the agent actually *thinks* in the common case.
- **Ring 1** is the *deterministic* re-implementation: the same model, integer-quantized, runnable
  inside the VM. In the happy path it never runs; it exists so any output can be *re-derived*.
- **Ring 0** scores agent outputs with PoM and adjudicates disputes by re-execution (§4). The
  agent is just another contributor whose provenance happens to be a program.

The honest split: **candle/mistral.rs are Ring-2 tools (std, GPU-aware). `burn` (or hand-rolled
integer kernels) is the Ring-1 substrate** — it is the only major Rust ML framework that targets
`no_std`/embedded/deterministic execution. Do not conflate them; they are different code paths
sharing a pinned weights/tokenizer tuple.

---

## 3. Model selection (the brain)

Findings (full table + sources in the session's research report; reproduce into `docs/` when this
plan graduates to a spec):

**Runs locally today in Rust (Ring 2):**
| Pick | Params | License | Footprint Q4_K_M | Why |
|---|---|---|---|---|
| **Qwen3-4B** | 4B | Apache-2.0 | ~2.5 GB | Reliable tool-calling, 128K ctx, best Rust quant tooling (mistral.rs). Default brain. |
| **Phi-4-mini** | 3.8B | MIT | ~2.3 GB | Best-in-class function-calling at this size. Use when the loop is tool-heavy. |
| SmolLM2-360M | 360M | Apache-2.0 | <0.3 GB | Cheap candle-based router/classifier sidecar, not the agent brain. |

> Hard finding: below ~3B the failure mode is tool-call *initiation*, not accuracy — small models
> often fail to invoke tools at all. ~3–4B is the practical floor for an *autonomous* loop. Do not
> romanticize sub-2B for the agent brain. Avoid Llama-3.2 / Gemma as the canonical model: custom
> restricted licenses clash with an open/CC0 posture.

**Plausible verifiable-inference path, 12–24 mo (Ring 1):**
> An **integer-only ~2–4B model — BitNet-ternary (MIT, integer-native, ~0.4–1.1 GB) or
> I-LLM-quantized Qwen3-4B — executed deterministically inside the RISC-V VM.** BitNet's arithmetic
> is associative end-to-end (the determinism win is structural, not bolted on); its agentic
> capability is currently ~1B-class and unproven, so it is the *research bet*, not a drop-in.
> Validate tool-call initiation on a real ReAct/BFCL harness before committing.

---

## 4. Verification path (the on-chain / L2 question Will asked)

**Full on-chain LLM inference is infeasible and stays so for the horizon.** A 3–4B model is
billions of MACs per token; no L1 runs that in-block. Every viable design moves *compute* off-chain
and puts only *verification* on-chain. Two families, and our substrate gets both from one binary:

| | **opML (optimistic / fraud-proof)** | **zkML (validity proof)** |
|---|---|---|
| Per-inference cost | ~native | 10³–10⁴× native |
| Latency to result | immediate | minutes–hours |
| Finality | dispute window (~days) | proof time |
| Model ceiling today | 7B+ in production (ORA) | ~7–13B, slow (zkLLM ~388 s/7B) |
| Trust | 1 honest verifier + stake | cryptographic |
| **Fit for the agent loop** | **default path** | high-value attestations only |

**Recommended design: opML-style fraud proofs as the default, zk on dispute.**
- ORA already runs 7B models opML in production; their FPVM explicitly targets **RISC-V** — directly
  convergent with the Noesis VM. A 2–4B agent is comfortably inside today's feasible envelope.
- The fraud proof *requires deterministic replay* — which §1 gives us for free. This is why the
  integer-VM decision is load-bearing: without it there is no cheap verification path at all.
- For the rare high-value attestation ("this governance act was produced by model X on input Y"),
  prove the *same RISC-V binary* with a RISC-V zkVM (RISC Zero R0VM 2.0 / SP1 / Jolt Atlas). **One
  Rust binary, both verification paths** — the strongest reason to keep Ring 1 integer-only and
  RISC-V-native.
- L2 framing: the agent runtime is effectively an **AI co-processor / L2 to Noesis** — off-chain
  execution, on-chain settlement of a commitment + dispute game. Cost justifies itself only for
  outputs whose PoM-scored value clears the verification cost; cheap/low-stakes agent turns settle
  optimistically and are only ever re-executed if challenged.

---

## 5. Phased document set (what to actually write, in dependency order)

Each becomes a `docs/` spec when it graduates from this plan. Ordered so the load-bearing risk
comes first (mirrors the chain roadmap's own discipline).

1. **`AGENT-DETERMINISM.md`** — the integer-inference contract: quantization scheme, fixed-point
   format (reuse the chain's Q32.32 where it fits), non-linearity lookup tables, greedy-decode
   tie-break, the pinned (weights,tokenizer,runtime,seed) tuple and how it is committed as a cell.
   *This is the `v(S)`-equivalent gate: get it right or nothing downstream is sound.*
2. **`AGENT-RUNTIME.md`** — Ring 1 engine in Rust: `burn`/integer-kernel choice, the ReAct loop,
   tool dispatch ABI, context/memory as content-addressed cells. Drift-guarded ≡ Ring 2 (the
   off-chain engine) exactly as the chain's on-VM ports are drift-guarded ≡ the reference model.
3. **`AGENT-VERIFICATION.md`** — the opML fraud-proof game on the Noesis VM + the zk fallback;
   challenge window, bisection-to-divergent-instruction, bonds (reuse the dispute stack's appeal
   bond doubling), and the privacy treatment of prompts in a replayable trace.
4. **`AGENT-AS-CONTRIBUTOR.md`** — how agent outputs enter the PoM value gate: lineage, the
   causal-share/`v(S)` treatment of machine-authored cells, and whether agent standing is
   PoM-only or carries its own dimension. *Touches the dispute stack we just hardened (the
   asymmetric-appeal guard) — an agent defendant is a down-weighted-dimension holder like any
   other.*
5. **`AGENT-ECONOMICS.md`** — when does on-chain/L2 verification pay? The break-even between
   PoM-scored output value and verification cost; the optimistic-by-default / prove-on-challenge
   policy; free-tier-inference posture (no paid plans, per standing constraint).

---

## 6. Open questions (resolve before the specs, not during)

- **Q1 — quantization fidelity vs determinism.** Does integer PTQ of Qwen3-4B retain tool-call
  initiation reliability, or does only a natively-integer model (BitNet) hold up? *Empirical —
  needs a BFCL run on the quantized weights before we commit Ring 1's model.*
- **Q2 — trace size / prompt privacy.** opML replay needs the full trace; that leaks the prompt.
  Hybridize with zk for the sensitive span (opp/ai pattern) or accept public prompts for the
  agent's on-chain acts?
- **Q3 — context as cells.** Is the agent's working memory committed per-turn (auditable, heavy)
  or only hashed (cheap, less inspectable)? Interacts with PoM scoring of intermediate reasoning.
- **Q4 — recused-dimension lockstep.** If the agent gets its own consensus dimension, the dispute
  guard's hardcoded-PoM recusal (see the NEXT RSAW target) must generalize. Coordinate this with
  the per-certifier-clamp tick already queued.
- **Q5 — does the agent run in Ring 1 ever, in the happy path?** Almost certainly no — Ring 1 is
  the *verification* substrate, not the *execution* one. Confirm before investing in Ring-1 speed.

---

## 7. First concrete step (when this plan is greenlit)

Write **`AGENT-DETERMINISM.md`** and, alongside it, a throwaway `research/` prototype: quantize
Qwen3-4B (or BitNet 2B) to integer, run greedy decode twice on two machines, and assert
bit-identical logits. That single experiment de-risks the entire load-bearing decision in §1
before any runtime code is written — the same "measure the moat, don't assert it" discipline the
chain's held-out `v(S)` harness already follows.
