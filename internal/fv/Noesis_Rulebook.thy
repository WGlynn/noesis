(*
  Noesis_Rulebook.thy — Phase-4 Step-3 machine-checked spec (VALUE level).

  ┌───────────────────────────────────────────────────────────────────────────────────────┐
  │ ⚠ STATUS: NOT machine-checked in the authoring environment (no Isabelle/HOL installed). │
  │   This is the review artifact the Phase-4 plan calls for ("a spec HE reviews"). The     │
  │   DEFINITIONS and THEOREM STATEMENTS are the deliverable; the proof bodies are          │
  │   best-effort Isar written to be discharged/adjusted under Isabelle by the owner. Do NOT │
  │   cite "conservation is proved" until this theory loads clean. See README.md for the     │
  │   model-to-code gap (required reading before trusting any theorem here).                 │
  └───────────────────────────────────────────────────────────────────────────────────────┘

  Models runtime::apply_block at the VALUE level: state = a finite partial map (cell id ⇀ value).
  Abstracts away locks / scripts / PoM / finality / u64-saturation — each abstraction is named in
  README.md §"model-to-code gap". Mirrors validate_block + apply_transition for the fungible core:
  input existence, strict conservation, fresh outputs (the unique-id discipline), single-use.
*)

theory Noesis_Rulebook
  imports Main
begin

type_synonym cid   = nat            (* cell identity (the code's unique Cell.id)              *)
type_synonym value = nat            (* fungible amount (ℕ; the code's u128, see gap G-a)      *)
type_synonym state = "cid ⇀ value"  (* the live UTXO set: a partial map id ⇀ value            *)

record tx =
  ins  :: "cid set"          (* consumed input ids                                            *)
  outs :: "cid ⇀ value"      (* produced (id ⇀ value) outputs; a map ⇒ output ids are distinct *)

text ‹Total value held by a (finite) state.›
definition total :: "state ⇒ value" where
  "total s = (∑ i ∈ dom s. the (s i))"

definition consumed_value :: "state ⇒ cid set ⇒ value" where
  "consumed_value s I = (∑ i ∈ I. the (s i))"

definition produced_value :: "(cid ⇀ value) ⇒ value" where
  "produced_value O = (∑ i ∈ dom O. the (O i))"

text ‹Validity of one tx against state @{term s}. Mirrors the code's per-tx gate at the value level:
  (1) every input EXISTS (@{term \<open>ins t ⊆ dom s\<close>} — is_valid_in_ledger existence);
  (2) STRICT conservation (consumed = produced — no mint, no burn: see gap G-c);
  (3) outputs are FRESH (disjoint from the whole live set — the code's unique-id discipline; this is
      what makes single-use clean).›
definition tx_valid :: "state ⇒ tx ⇒ bool" where
  "tx_valid s t ⟷
       ins t ⊆ dom s
     ∧ consumed_value s (ins t) = produced_value (outs t)
     ∧ dom (outs t) ∩ dom s = {}"

text ‹The transition: retire the consumed inputs, then add the produced outputs.›
definition apply_tx :: "state ⇒ tx ⇒ state" where
  "apply_tx s t = (s |` (dom s - ins t)) ++ outs t"

subsection ‹Helper lemmas (finite-sum bookkeeping)›

lemma total_disjoint_union:
  assumes "finite (dom a)" "finite (dom b)" "dom a ∩ dom b = {}"
  shows "total (a ++ b) = total a + total b"
  (* Proof plan: dom (a++b) = dom a ∪ dom b (disjoint); on dom b, (a++b) i = b i; on dom a it = a i
     (by disjointness). Then sum.union_disjoint. *)
  sorry

lemma total_restrict_diff:
  assumes "finite (dom s)" "I ⊆ dom s"
  shows "total (s |` (dom s - I)) = total s - consumed_value s I"
  (* Proof plan: dom (s|`(dom s - I)) = dom s - I; total s = (∑ over I) + (∑ over dom s - I) by
     sum.subset_diff; the first sum is consumed_value s I. ℕ subtraction is exact here because the
     consumed part is ≤ total s. *)
  sorry

subsection ‹Theorem 1 — conservation (no inflation)›

theorem conservation:
  assumes fin: "finite (dom s)"
      and valid: "tx_valid s t"
    shows "total (apply_tx s t) = total s"
proof -
  have I: "ins t ⊆ dom s" and cons: "consumed_value s (ins t) = produced_value (outs t)"
       and fresh: "dom (outs t) ∩ dom s = {}"
    using valid by (auto simp: tx_valid_def)
  have finO: "finite (dom (outs t))"       (* outputs are a finite map in any real tx *) sorry
  have finR: "finite (dom (s |` (dom s - ins t)))" using fin by simp
  have disj: "dom (s |` (dom s - ins t)) ∩ dom (outs t) = {}"
    using fresh by (auto simp: restrict_map_def dom_def split: if_splits)
  have "total (apply_tx s t)
          = total (s |` (dom s - ins t)) + total (outs t)"
    unfolding apply_tx_def using finR finO disj by (rule total_disjoint_union)
  also have "total (s |` (dom s - ins t)) = total s - consumed_value s (ins t)"
    using fin I by (rule total_restrict_diff)
  also have "total (outs t) = produced_value (outs t)"
    by (simp add: total_def produced_value_def)
  finally show ?thesis using cons
    (* consumed = produced ⇒ (total s - consumed) + produced = total s, since consumed ≤ total s. *)
    sorry
qed

subsection ‹Theorem 2 — no double-spend (a consumed input is absent from the next state)›

theorem no_double_spend:
  assumes valid: "tx_valid s t"
  shows "ins t ∩ dom (apply_tx s t) = {}"
proof -
  have I: "ins t ⊆ dom s" and fresh: "dom (outs t) ∩ dom s = {}"
    using valid by (auto simp: tx_valid_def)
  have "dom (apply_tx s t) = (dom s - ins t) ∪ dom (outs t)"
    by (auto simp: apply_tx_def)
  moreover have "ins t ∩ (dom s - ins t) = {}" by auto
  moreover have "ins t ∩ dom (outs t) = {}" using I fresh by auto
  ultimately show ?thesis by auto
qed

subsection ‹Theorem 3 — determinism (anchors I5; trivial in HOL, stated to be explicit)›

theorem determinism:
  shows "apply_tx s t = apply_tx s t"
  by (rule refl)   (* apply_tx is a function ⇒ same input, same output. The Rust I5 property is the
                       operational witness of this HOL triviality. *)

end
