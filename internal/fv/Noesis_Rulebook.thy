(*
  Noesis_Rulebook.thy -- Phase-4 Step-3 machine-checked spec (VALUE level).

  MACHINE-CHECKED GREEN under Isabelle2025 on 2026-07-12 via `isabelle build -D internal/fv` (or
  internal/fv/verify.sh): conservation + no_double_spend + determinism all discharged, 0 sorry.
  Re-verify on any machine with the pinned Isabelle2025 (URL + sha256 in README.md, sec. "reproducible
  verification"). Claim "conservation is proved" only from a green build; this file last built green.

  Models runtime::apply_block at the VALUE level: state = a finite partial map (cell id -> amount).
  Abstracts away locks / scripts / PoM / finality / u64-saturation -- each abstraction is named in
  README.md, sec. "model-to-code gap". Mirrors validate_block + apply_transition for the fungible
  core: input existence, strict conservation, fresh outputs (the unique-id discipline), single-use.

  Style: ASCII Isabelle symbol escapes (\<rightharpoonup>, \<Sum>, ...) are used throughout the code
  for encoding-independence in headless builds. The value type is named `amount`, not `value`
  (the latter is a reserved Isabelle command).
*)

theory Noesis_Rulebook
  imports Main
begin

type_synonym cid    = nat
type_synonym amount = nat
type_synonym state  = "cid \<rightharpoonup> amount"   \<comment> \<open>the live UTXO set: a partial map id -> amount\<close>

record tx =
  ins  :: "cid set"            \<comment> \<open>consumed input ids\<close>
  outs :: "cid \<rightharpoonup> amount"  \<comment> \<open>produced (id -> amount) outputs; a map, so output ids are distinct\<close>

definition total :: "state \<Rightarrow> amount" where
  "total s = (\<Sum>i\<in>dom s. the (s i))"

definition consumed_value :: "state \<Rightarrow> cid set \<Rightarrow> amount" where
  "consumed_value s I = (\<Sum>i\<in>I. the (s i))"

definition produced_value :: "(cid \<rightharpoonup> amount) \<Rightarrow> amount" where
  "produced_value Ou = (\<Sum>i\<in>dom Ou. the (Ou i))"

text \<open>Validity of one tx against state s. Mirrors the code's per-tx gate at the value level:
  (0) FINITELY many outputs (a real tx; needed for the value bookkeeping);
  (1) every input EXISTS (ins t subseteq dom s -- the is_valid_in_ledger existence check);
  (2) STRICT conservation (consumed = produced -- no mint, no burn: see gap G-c);
  (3) outputs are FRESH (disjoint from the whole live set -- the code's unique-id discipline).\<close>
definition tx_valid :: "state \<Rightarrow> tx \<Rightarrow> bool" where
  "tx_valid s t \<longleftrightarrow>
       finite (dom (outs t))
     \<and> ins t \<subseteq> dom s
     \<and> consumed_value s (ins t) = produced_value (outs t)
     \<and> dom (outs t) \<inter> dom s = {}"

text \<open>The transition: retire the consumed inputs, then add the produced outputs.\<close>
definition apply_tx :: "state \<Rightarrow> tx \<Rightarrow> state" where
  "apply_tx s t = (s |` (dom s - ins t)) ++ outs t"

subsection \<open>Helper lemmas (finite-sum bookkeeping)\<close>

lemma total_disjoint_union:
  assumes fa: "finite (dom a)" and fb: "finite (dom b)" and dj: "dom a \<inter> dom b = {}"
  shows "total (a ++ b) = total a + total b"
proof -
  have dom: "dom (a ++ b) = dom a \<union> dom b" by auto
  have "total (a ++ b) = (\<Sum>i\<in>dom a \<union> dom b. the ((a ++ b) i))"
    unfolding total_def dom ..
  also have "\<dots> = (\<Sum>i\<in>dom a. the ((a ++ b) i)) + (\<Sum>i\<in>dom b. the ((a ++ b) i))"
    using fa fb dj by (rule sum.union_disjoint)
  also have "(\<Sum>i\<in>dom a. the ((a ++ b) i)) = (\<Sum>i\<in>dom a. the (a i))"
  proof (rule sum.cong)
    fix i assume "i \<in> dom a"
    with dj have "i \<notin> dom b" by auto
    thus "the ((a ++ b) i) = the (a i)" by (simp add: map_add_dom_app_simps)
  qed simp
  also have "(\<Sum>i\<in>dom b. the ((a ++ b) i)) = (\<Sum>i\<in>dom b. the (b i))"
  proof (rule sum.cong)
    fix i assume "i \<in> dom b"
    thus "the ((a ++ b) i) = the (b i)" by (simp add: map_add_dom_app_simps)
  qed simp
  finally show ?thesis by (simp add: total_def)
qed

lemma total_restrict_diff:
  assumes fin: "finite (dom s)" and sub: "I \<subseteq> dom s"
  shows "total (s |` (dom s - I)) = total s - consumed_value s I"
proof -
  have partition: "total s = (\<Sum>i\<in>dom s - I. the (s i)) + consumed_value s I"
    unfolding total_def consumed_value_def by (rule sum.subset_diff[OF sub fin])
  have dr: "dom (s |` (dom s - I)) = dom s - I" by (auto simp: dom_restrict)
  have restr: "total (s |` (dom s - I)) = (\<Sum>i\<in>dom s - I. the (s i))"
    unfolding total_def dr by (rule sum.cong[OF refl]) (simp add: restrict_map_def)
  show ?thesis using partition restr by simp
qed

subsection \<open>Theorem 1 -- conservation (no inflation)\<close>

theorem conservation:
  assumes fin: "finite (dom s)" and valid: "tx_valid s t"
  shows "total (apply_tx s t) = total s"
proof -
  from valid have finO: "finite (dom (outs t))"
    and I: "ins t \<subseteq> dom s"
    and cons: "consumed_value s (ins t) = produced_value (outs t)"
    and fresh: "dom (outs t) \<inter> dom s = {}"
    by (auto simp: tx_valid_def)
  have finR: "finite (dom (s |` (dom s - ins t)))"
    using fin by (simp add: dom_restrict)
  have disjR: "dom (s |` (dom s - ins t)) \<inter> dom (outs t) = {}"
    using fresh by (auto simp: dom_restrict)
  have "total (apply_tx s t) = total (s |` (dom s - ins t)) + total (outs t)"
    unfolding apply_tx_def using finR finO disjR by (rule total_disjoint_union)
  also have "total (s |` (dom s - ins t)) = total s - consumed_value s (ins t)"
    using fin I by (rule total_restrict_diff)
  also have "total (outs t) = produced_value (outs t)"
    by (simp add: total_def produced_value_def)
  finally have step:
    "total (apply_tx s t) = (total s - consumed_value s (ins t)) + produced_value (outs t)" .
  have le: "consumed_value s (ins t) \<le> total s"
    unfolding consumed_value_def total_def by (rule sum_mono2[OF fin I]) simp
  show ?thesis using step cons le by simp
qed

subsection \<open>Theorem 2 -- no double-spend (a consumed input is absent from the next state)\<close>

theorem no_double_spend:
  assumes valid: "tx_valid s t"
  shows "ins t \<inter> dom (apply_tx s t) = {}"
proof -
  from valid have I: "ins t \<subseteq> dom s" and fresh: "dom (outs t) \<inter> dom s = {}"
    by (auto simp: tx_valid_def)
  have "dom (apply_tx s t) = (dom s - ins t) \<union> dom (outs t)"
    by (auto simp: apply_tx_def dom_restrict)
  moreover have "ins t \<inter> (dom s - ins t) = {}" by auto
  moreover have "ins t \<inter> dom (outs t) = {}" using I fresh by auto
  ultimately show ?thesis by auto
qed

subsection \<open>Theorem 3 -- determinism (anchors I5; trivial in HOL, stated to be explicit)\<close>

theorem determinism: "apply_tx s t = apply_tx s t"
  by (rule refl)   \<comment> \<open>apply_tx is a function: same input, same output. The Rust I5 property is the
                       operational witness of this HOL triviality.\<close>

end
