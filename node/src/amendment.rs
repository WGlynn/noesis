//! Pragma layer — Constitution-amendment coherence (the socket). **🟡 first slice: governance-layer.**
//!
//! Second line of defence. Line 1 (`internal/fv/Noesis_Rulebook.thy`, `node/tests/fv_invariants.rs`)
//! proves the value axioms I1–I5 for a **fixed** rulebook. But the [`Constitution`] is *governable*:
//! governance can amend how value is measured, and line 1 says nothing about the **space** of
//! rulebooks governance can reach. Tom's danger quadrant is **Confluent + Axiom-breaking** — an
//! amended rulebook every replica agrees on, yet which silently violates an axiom. Neither line-1 FV
//! (it is not re-run against the amended rulebook) nor confluence-alone (the rulebook *is* confluent —
//! that is exactly why it is dangerous) sees it. Closing that cell needs an axiom-preservation check
//! AND a confluence check on the amendment itself.
//!
//! This module is the **socket** (`docs/DESIGN-pragma-layer-amendment-coherence.md`): a typed
//! inspectable amendment op (§5a) + a stated obligation checklist (§5b) + one layered gate (§5c). It is
//! NOT a confluence engine — Knuth–Bendix / confluent-rewriting is Pragma's product; we provide the
//! surface and the obligations, they discharge the expensive proof (§6, terms-first).
//!
//! **Status discipline.** `verify_amendment` returning `Ok(())` means *no socket-detectable breach* —
//! it does NOT assert the amended rulebook is coherent. The confluence proof and the full
//! attribution-preservation proof are the Pragma attach point; `Amendment::obligations` tags exactly
//! which obligations this socket discharges ([`Discharger::Socket`]) versus which remain open
//! ([`Discharger::Pragma`]). Never round `Ok` up to "proven safe".
//!
//! Scope of THIS slice: the **governance layer** (the amendments that actually happen today). The
//! constitutional-layer dimension-set moves are still `pending` upstream (`runtime.rs` `Constitution`
//! doc), so the gate rejects them as [`ObligationBreach::ConstitutionalPending`]; the physics layer is
//! near-immutable and always rejected with a stated reason.

use crate::consensus::{Mix, BPS, TWO_THIRDS_BPS};
use crate::runtime::Constitution;

/// Q16.16 unit (1.0). `theta_sim_q16` is a fraction in `[0, ONE_Q16]`.
const ONE_Q16: u64 = 1 << 16;

/// The three governance layers of the measurement/finalization frame (DESIGN note §3). Physics is
/// near-immutable; constitutional (the dimension set) is verifier-gated and `pending` upstream;
/// governance (bounded weights/params) is fluid — the surface this slice checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    Physics,
    Constitutional,
    Governance,
}

/// WHO may authorize an amendment (`docs/DESIGN-governance-authority-tiering.md`). ORTHOGONAL to
/// [`Layer`] (which says WHAT is changed): a Governance-layer move still requires *Contribution*
/// authority when it touches the MEASURE. This is inert classification METADATA — the vote-counting
/// layer (PoM-standing-weighted for `Contribution`, buyable VIBE stewardship for `Stewardship`) is
/// DEFERRED: it needs the VIBE token + a sybil-resistant vote curve, which is a mechanism-design-paper
/// decision, not invented here. This tags the attach-point that layer will read. The anti-plutocracy
/// property does NOT depend on it — it is carried structurally (soulbound weight + the `pos ≤ pom` mix
/// bound + the axiom gate); the authority tag is legitimacy on top.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Authority {
    /// Earned, soulbound PoM-standing must authorize. A say over WHAT COUNTS AS CONTRIBUTION (the
    /// measure) or the MIX DIRECTION cannot be bought — sybil-resistance is inherited from the PoM
    /// measure itself (you cannot split soulbound standing across wallets), so it opens no new sybil
    /// surface; it reduces to the un-gameability moat the chain already stakes on.
    Contribution,
    /// Buyable, exitable VIBE stewardship. Operational dials that tune the machine without redefining
    /// contribution or shifting power between capital and contribution; bounded + axiom-gated, so even
    /// vote-domination cannot reach a plutocratic state (the outcome bound does the work).
    Stewardship,
}

/// A governance-layer scalar field of [`Constitution`]. (`decay_pos` is carried as 0/1; `mix` is not
/// here — it has its own [`Amendment::AmendMix`] variant because it is a 3-tuple with a sum invariant.)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GovField {
    ThresholdBps,
    QuorumFloorBps,
    Horizon,
    DecayPos,
    ThetaSimQ16,
    VestingW,
    MaxMempool,
}

impl GovField {
    /// The live value of this field in `c`, as u64 (`decay_pos` → 0/1). Lets the gate check that an
    /// amendment was computed against the CURRENT base (no stale-base application).
    fn live(self, c: &Constitution) -> u64 {
        match self {
            GovField::ThresholdBps => c.threshold_bps,
            GovField::QuorumFloorBps => c.quorum_floor_bps,
            GovField::Horizon => c.horizon,
            GovField::DecayPos => c.decay_pos as u64,
            GovField::ThetaSimQ16 => c.theta_sim_q16,
            GovField::VestingW => c.vesting_w,
            GovField::MaxMempool => c.max_mempool as u64,
        }
    }

    fn name(self) -> &'static str {
        match self {
            GovField::ThresholdBps => "threshold_bps",
            GovField::QuorumFloorBps => "quorum_floor_bps",
            GovField::Horizon => "horizon",
            GovField::DecayPos => "decay_pos",
            GovField::ThetaSimQ16 => "theta_sim_q16",
            GovField::VestingW => "vesting_w",
            GovField::MaxMempool => "max_mempool",
        }
    }

    /// Which authority class may amend this field (`docs/DESIGN-governance-authority-tiering.md`).
    /// MEASURE-defining fields (what counts as contribution + how fast it clears) require earned,
    /// soulbound `Contribution` authority; the rest are operational dials under buyable `Stewardship`.
    pub fn authority(self) -> Authority {
        match self {
            GovField::ThetaSimQ16 | GovField::VestingW => Authority::Contribution,
            GovField::ThresholdBps
            | GovField::QuorumFloorBps
            | GovField::Horizon
            | GovField::DecayPos
            | GovField::MaxMempool => Authority::Stewardship,
        }
    }
}

/// A typed, inspectable mutation of the measurement/finalization frame (DESIGN note §5a). NOT an opaque
/// replacement `Constitution`: each variant names the layer it touches, so a reviewer (human, this gate,
/// or Pragma's engine) reads the amendment as a *mutation*, not a struct diff.
#[derive(Clone, Debug)]
pub enum Amendment {
    /// Governance layer — a bounded move of one scalar field. `old` is the value the proposer saw.
    AmendParam { field: GovField, old: u64, new: u64 },
    /// Governance layer — reweight the NCI overall consensus mix. (Finality safety uses the LOCKED
    /// `FINALITY_MIX` + `MIN_DIM_BPS`, not this field, so this is genuinely governance-tunable.)
    AmendMix { old: Mix, new: Mix },

    /// Constitutional layer — add a measurement dimension. `pending` upstream ⇒ gate rejects.
    AddDimension { id: u64 },
    /// Constitutional layer — retire a measurement dimension. `pending` upstream ⇒ gate rejects.
    RetireDimension { id: u64 },
    /// Constitutional layer — reweight a dimension. `pending` upstream ⇒ gate rejects.
    ReweightDimension { id: u64, old_bps: u64, new_bps: u64 },

    /// Physics layer — present ONLY so the gate can name WHY it is refused (near-immutable by design).
    AmendPhysics { what: &'static str },
}

/// Who discharges an obligation: this socket (cheap/structural), or the Pragma confluence engine
/// (the expensive confluence + attribution-preservation proof, §6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Discharger {
    Socket,
    Pragma,
}

/// One stated obligation an amendment must preserve (DESIGN note §5b). The checklist is honest and
/// inspectable: an `Ok` from [`verify_amendment`] clears only the [`Discharger::Socket`] rows.
#[derive(Clone, Copy, Debug)]
pub struct Obligation {
    pub name: &'static str,
    pub by: Discharger,
    pub note: &'static str,
}

/// Why an amendment is refused. A breach is a HARD stop. Note: a [`Discharger::Pragma`] obligation is
/// NOT a breach — it is simply out of this gate's scope, listed by [`Amendment::obligations`].
#[derive(Clone, Debug, PartialEq)]
pub enum ObligationBreach {
    /// Physics layer is near-immutable.
    PhysicsImmutable { what: &'static str },
    /// Constitutional (dimension-set) amendment rules are not built in code yet (`pending` upstream).
    ConstitutionalPending,
    /// The amendment's stated `old` does not match the live [`Constitution`] — computed off stale base.
    StaleBase { field: &'static str, stated: u64, live: u64 },
    /// A scalar governance field would leave its valid range.
    OutOfBounds { field: &'static str, reason: &'static str },
    /// Mix components do not sum to 1 (reported in units of 1e-4).
    MixNotNormalized { sum_e4: i64 },
    /// A mix component is negative.
    MixNegative,
    /// An `AmendMix` would make capital (PoS) outweigh contribution (PoM) — the number-free
    /// anti-plutocracy mix bound (`pos ≤ pom`). Governance may retune the NCI mix but can never tilt the
    /// chain toward capital-rule (`docs/DESIGN-governance-authority-tiering.md`).
    CapitalOutweighsContribution { pos_e4: i64, pom_e4: i64 },
}

impl Amendment {
    /// The layer this amendment touches.
    pub fn layer(&self) -> Layer {
        match self {
            Amendment::AmendParam { .. } | Amendment::AmendMix { .. } => Layer::Governance,
            Amendment::AddDimension { .. }
            | Amendment::RetireDimension { .. }
            | Amendment::ReweightDimension { .. } => Layer::Constitutional,
            Amendment::AmendPhysics { .. } => Layer::Physics,
        }
    }

    /// WHO may authorize this amendment (`docs/DESIGN-governance-authority-tiering.md`). `None` = never
    /// authorizable (physics near-immutable). ORTHOGONAL to [`Self::layer`]: a Governance-layer param can
    /// be either class depending on whether it touches the MEASURE. Reweighting the mix and defining the
    /// dimension set both shift/define what contribution is worth ⇒ `Contribution`. Inert metadata: the
    /// vote-counting layer that will read this is DEFERRED (needs VIBE — *earned by validation*, proposed
    /// — plus a sybil-resistant vote curve, an MD-paper decision).
    pub fn authority(&self) -> Option<Authority> {
        match self {
            Amendment::AmendParam { field, .. } => Some(field.authority()),
            Amendment::AmendMix { .. } => Some(Authority::Contribution),
            Amendment::AddDimension { .. }
            | Amendment::RetireDimension { .. }
            | Amendment::ReweightDimension { .. } => Some(Authority::Contribution),
            Amendment::AmendPhysics { .. } => None,
        }
    }

    /// The full obligation checklist for this amendment — every property it must preserve, tagged with
    /// who discharges it (DESIGN note §5b/§6). Governance moves are token-blind (Family A trivial by
    /// construction — the value path folds over `token_cells`, never the measurement frame) and cannot
    /// reach identity (Family-B anonymity-relaxation preserved by construction). The EXPENSIVE
    /// obligations — attribution-map preservation over the Myerson-restricted, anonymity-relaxed value,
    /// and confluence of the amended rulebook — are the Pragma attach point.
    pub fn obligations(&self) -> Vec<Obligation> {
        match self.layer() {
            Layer::Governance => vec![
                Obligation {
                    name: "FamilyA: value-invariants I1-I5",
                    by: Discharger::Socket,
                    note: "trivial by construction: governance params never enter the token value gate",
                },
                Obligation {
                    name: "FamilyB: anonymity-relaxation preserved (Cheng-Friedman)",
                    by: Discharger::Socket,
                    note: "param/mix moves do not touch identity ⇒ a fresh name stays worth zero",
                },
                Obligation {
                    name: "FamilyB: attribution-map preservation (Myerson-restricted Shapley)",
                    by: Discharger::Pragma,
                    note: "evaluated per-property by attribution_verdicts(): most preserved-by-construction here (only theta_sim reaches pom_scores); a theta_sim RAISE is AtRisk on null-player ⇒ needs this Pragma discharge",
                },
                Obligation {
                    name: "Confluence of the amended rulebook (Newman: local confluence + termination)",
                    by: Discharger::Pragma,
                    note: "sub-second pre-merge CI hook; not this socket's to discharge",
                },
            ],
            // Constitutional/physics amendments are refused by the gate; their obligations are stated
            // for inspectability but not reachable in this slice.
            Layer::Constitutional => vec![Obligation {
                name: "constitutional dimension-set amendment rules",
                by: Discharger::Pragma,
                note: "pending upstream (Constitution dimension matrix); gate rejects until built",
            }],
            Layer::Physics => vec![Obligation {
                name: "physics layer immutability",
                by: Discharger::Socket,
                note: "near-immutable by design; any physics amendment is refused",
            }],
        }
    }
}

/// The gate (DESIGN note §5c). Discharges the cheap/structural obligations for a governance amendment
/// and refuses physics (immutable), constitutional (pending), stale-base, and out-of-bounds moves.
///
/// `Ok(())` = **no socket-detectable breach** — NOT a proof of coherence. The confluence proof and the
/// full attribution-preservation proof are the Pragma attach point ([`Amendment::obligations`] rows
/// tagged [`Discharger::Pragma`]). Do not round `Ok` up to "proven safe".
pub fn verify_amendment(old: &Constitution, a: &Amendment) -> Result<(), ObligationBreach> {
    match a {
        Amendment::AmendPhysics { what } => Err(ObligationBreach::PhysicsImmutable { what }),

        Amendment::AddDimension { .. }
        | Amendment::RetireDimension { .. }
        | Amendment::ReweightDimension { .. } => Err(ObligationBreach::ConstitutionalPending),

        Amendment::AmendParam { field, old: stated, new } => {
            let live = field.live(old);
            if *stated != live {
                return Err(ObligationBreach::StaleBase { field: field.name(), stated: *stated, live });
            }
            check_gov_param(*field, *new)
        }

        Amendment::AmendMix { old: stated, new } => {
            if !mix_eq(stated, &old.mix) {
                // stale base: the proposer reweighted from a mix that is not the live one.
                return Err(ObligationBreach::StaleBase { field: "mix", stated: 0, live: 0 });
            }
            check_mix(new)
        }
    }
}

/// Bounded-weight check for a single governance scalar (DESIGN note §5c "bounded-weight check"). Bounds
/// are the REAL safety invariants, not cosmetic: `threshold_bps` may never drop below the 2/3
/// supermajority bar (BFT safety), `theta_sim_q16` is a fraction ≤ 1.0, `max_mempool` must admit ≥ 1.
fn check_gov_param(field: GovField, new: u64) -> Result<(), ObligationBreach> {
    let oob = |reason| Err(ObligationBreach::OutOfBounds { field: field.name(), reason });
    match field {
        // Lowering finalization below 2/3 breaks Byzantine safety; above 100% is unreachable.
        GovField::ThresholdBps if new < TWO_THIRDS_BPS => oob("below the 2/3 supermajority safety bar"),
        GovField::ThresholdBps if new > BPS => oob("above 100% (BPS)"),
        GovField::QuorumFloorBps if new > BPS => oob("above 100% (BPS)"),
        GovField::ThetaSimQ16 if new > ONE_Q16 => oob("above 1.0 (Q16.16) — overlap fraction > 100%"),
        GovField::DecayPos if new > 1 => oob("decay_pos is boolean (0 or 1)"),
        GovField::MaxMempool if new == 0 => oob("zero mempool cap bricks proposal admission"),
        // horizon, vesting_w: any u64 is structurally valid (0 = inert; no upper safety bound here).
        _ => Ok(()),
    }
}

/// Mix must have non-negative components summing to 1 (DESIGN note §5c). Finality safety is enforced
/// separately by the LOCKED `FINALITY_MIX` + `MIN_DIM_BPS`, so the only invariant on the NCI overall
/// mix is a valid probability split.
fn check_mix(m: &Mix) -> Result<(), ObligationBreach> {
    if m.pow < 0.0 || m.pos < 0.0 || m.pom < 0.0 {
        return Err(ObligationBreach::MixNegative);
    }
    let sum = m.pow + m.pos + m.pom;
    if (sum - 1.0).abs() > 1e-9 {
        return Err(ObligationBreach::MixNotNormalized { sum_e4: ((sum - 1.0) * 10_000.0).round() as i64 });
    }
    // Anti-plutocracy mix bound (DESIGN-governance-authority-tiering.md): capital's share may NEVER
    // exceed contribution's — `pos ≤ pom`. The anti-plutocracy thesis in token form, NUMBER-FREE (no
    // invented floor): contribution stays the dominant axis, so a buyable-governance capture cannot tilt
    // the NCI mix toward capital-rule. (Finality safety is separately fenced by the LOCKED FINALITY_MIX +
    // MIN_DIM_BPS; this bounds the OVERALL NCI mix the socket governs, which those do not.)
    if m.pos > m.pom {
        return Err(ObligationBreach::CapitalOutweighsContribution {
            pos_e4: (m.pos * 10_000.0).round() as i64,
            pom_e4: (m.pom * 10_000.0).round() as i64,
        });
    }
    Ok(())
}

/// Component-wise mix equality within Q-float tolerance (stale-base comparison).
fn mix_eq(a: &Mix, b: &Mix) -> bool {
    (a.pow - b.pow).abs() < 1e-9 && (a.pos - b.pos).abs() < 1e-9 && (a.pom - b.pom).abs() < 1e-9
}

// ============ Family-B: attribution-preservation obligations (DESIGN note §5b) ============
//
// GROUND TRUTH (`node/src/runtime.rs:748`): the PoM attribution map is
// `pom_scores_with_similarity_floor_q16(&state.cells, params.theta_sim_q16)`. Among the governance
// fields of [`Constitution`], ONLY `theta_sim_q16` reaches the attribution map — every other param
// (`mix`, `threshold_bps`, `quorum_floor_bps`, `horizon`, `decay_pos`, `vesting_w`, `max_mempool`) is
// finalization/liveness and never enters `pom_scores`. That single fact is what lets the socket
// discharge most Family-B obligations *by construction* instead of deferring the whole family to Pragma.

/// A Family-B attribution property the amendment must preserve. Each is pinned to a live regression in
/// `node/src/lib.rs` (re-verify line numbers at source): the coalition value is a Myerson graph-restricted,
/// seeded Data-Shapley value over the provenance graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttributionProperty {
    /// Credited shares sum to the coalition value (the split creates/destroys no value).
    Efficiency,
    /// A redundant / no-value contribution earns ~0 marginal (`redundant_cell_gets_low_shapley_marginal`).
    NullPlayer,
    /// Value stays a cooperative Shapley split, never a naive additive win-share
    /// (`synergy_shapley_differs_from_additive_copeland`).
    Synergy,
    /// Provenance edges gate credit; disconnected coalitions cannot pool value
    /// (`myerson_restricts_value_to_provenance`).
    MyersonRestriction,
    /// The seeded Data-Shapley estimate is bit-identical across replicas (`sampled_value`, seeded PRNG).
    Determinism,
    /// The **deliberately-relaxed** anonymity axiom (Cheng-Friedman): a fresh identity is worth zero by
    /// construction. The obligation is to PRESERVE the relaxation — an amendment that quietly re-introduces
    /// symmetry/anonymity re-opens the Sybil hole while staying confluent (DESIGN note §5b caution box).
    AnonymityRelaxation,
}

impl AttributionProperty {
    /// All six, in declaration order.
    pub const ALL: [AttributionProperty; 6] = [
        AttributionProperty::Efficiency,
        AttributionProperty::NullPlayer,
        AttributionProperty::Synergy,
        AttributionProperty::MyersonRestriction,
        AttributionProperty::Determinism,
        AttributionProperty::AnonymityRelaxation,
    ];
}

/// The socket's per-property verdict on an amendment. Honest three-way split: what we can clear here,
/// what trades off (needs the Pragma discharge), and what genuinely reshapes credit (Pragma's proof).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropertyVerdict {
    /// The amendment cannot reach the mechanism that implements this property ⇒ discharged HERE.
    PreservedByConstruction { why: &'static str },
    /// The amendment touches a param that trades off against this property. NOT a hard breach (it may be
    /// a legitimate governance choice), but NOT discharged here either — it needs the Pragma confluence +
    /// attribution-preservation discharge, or an explicit governance acknowledgment, before merge.
    AtRisk { why: &'static str },
    /// A genuine reshaping of the measurement matrix whose full preservation proof is the confluence
    /// engine's (the constitutional dimension-set surface).
    DeferredToPragma { why: &'static str },
}

/// Evaluate every Family-B attribution property for `a` (DESIGN note §5b/§9). This is the socket
/// *discharging* the obligations it can and *stating* the ones it cannot — never rounding a trade-off
/// up to "preserved". See [`family_b_at_risk`] for the one-bit summary.
pub fn attribution_verdicts(a: &Amendment) -> Vec<(AttributionProperty, PropertyVerdict)> {
    use AttributionProperty::*;
    use PropertyVerdict::*;

    let uniform = |v: PropertyVerdict| AttributionProperty::ALL.iter().map(|p| (*p, v.clone())).collect();

    match a {
        // theta_sim_q16 is the ONLY governance param that reshapes credit. Raising it (more permissive
        // near-duplicate floor) lets near-duplicate cells earn novelty ⇒ weakens the null/dummy
        // paraphrase-padding defense. Lowering/holding it is strictly stronger dedup. Everything else the
        // attribution map computes (the Shapley split, Myerson restriction, determinism, and the identity-
        // anchored anonymity relaxation) is structural and untouched by a scalar novelty-floor move.
        Amendment::AmendParam { field: GovField::ThetaSimQ16, old, new } => AttributionProperty::ALL
            .iter()
            .map(|p| {
                let v = match p {
                    NullPlayer if new > old => AtRisk {
                        why: "raising the similarity floor lets near-duplicate cells earn novelty, weakening the null/dummy dedup — needs the Pragma preservation discharge",
                    },
                    NullPlayer => PreservedByConstruction {
                        why: "lowering/holding the similarity floor is strictly stronger dedup",
                    },
                    _ => PreservedByConstruction {
                        why: "theta_sim reshapes only the novelty/dedup floor; the Shapley split, Myerson restriction, determinism and anonymity-relaxation are structural",
                    },
                };
                (*p, v)
            })
            .collect(),

        // Every other governance param never enters pom_scores (runtime.rs:748) ⇒ it cannot reach the
        // attribution map at all. Full Family-B discharge, by construction.
        Amendment::AmendParam { .. } | Amendment::AmendMix { .. } => uniform(PreservedByConstruction {
            why: "finalization/liveness/mix param; never enters pom_scores ⇒ cannot reach the attribution map",
        }),

        // Constitutional dimension-set moves genuinely reshape the measurement matrix; the full
        // preservation proof is the confluence engine's (and the surface is pending upstream anyway).
        Amendment::AddDimension { .. }
        | Amendment::RetireDimension { .. }
        | Amendment::ReweightDimension { .. } => uniform(DeferredToPragma {
            why: "dimension-set change reshapes the attribution matrix; preservation is the confluence engine's discharge",
        }),

        // Physics amendments are refused outright by the gate; no attribution evaluation applies.
        Amendment::AmendPhysics { .. } => vec![],
    }
}

/// One-bit summary: does any Family-B property trade off (needs the Pragma discharge before merge)?
/// A governance amendment can be `verify_amendment`-Ok (in bounds) yet `family_b_at_risk` — the two
/// answer different questions (safety-bounds vs attribution-preservation).
pub fn family_b_at_risk(a: &Amendment) -> bool {
    attribution_verdicts(a).iter().any(|(_, v)| matches!(v, PropertyVerdict::AtRisk { .. }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::NCI;

    fn base() -> Constitution {
        Constitution::default()
    }

    // --- governance param: in-bounds passes, out-of-bounds fails (each with its RED twin) ---

    #[test]
    fn threshold_at_bar_ok_below_bar_rejected() {
        let c = base();
        // exactly the 2/3 bar clears (RED anchor)
        let ok = Amendment::AmendParam { field: GovField::ThresholdBps, old: c.threshold_bps, new: TWO_THIRDS_BPS };
        assert!(verify_amendment(&c, &ok).is_ok());
        // one bps below the bar is refused
        let bad = Amendment::AmendParam { field: GovField::ThresholdBps, old: c.threshold_bps, new: TWO_THIRDS_BPS - 1 };
        assert!(matches!(verify_amendment(&c, &bad), Err(ObligationBreach::OutOfBounds { .. })));
    }

    #[test]
    fn theta_sim_at_one_ok_above_one_rejected() {
        let c = base();
        let ok = Amendment::AmendParam { field: GovField::ThetaSimQ16, old: c.theta_sim_q16, new: ONE_Q16 };
        assert!(verify_amendment(&c, &ok).is_ok());
        let bad = Amendment::AmendParam { field: GovField::ThetaSimQ16, old: c.theta_sim_q16, new: ONE_Q16 + 1 };
        assert!(matches!(verify_amendment(&c, &bad), Err(ObligationBreach::OutOfBounds { .. })));
    }

    #[test]
    fn max_mempool_zero_rejected_nonzero_ok() {
        let c = base();
        let bad = Amendment::AmendParam { field: GovField::MaxMempool, old: c.max_mempool as u64, new: 0 };
        assert!(matches!(verify_amendment(&c, &bad), Err(ObligationBreach::OutOfBounds { .. })));
        let ok = Amendment::AmendParam { field: GovField::MaxMempool, old: c.max_mempool as u64, new: 1 };
        assert!(verify_amendment(&c, &ok).is_ok());
    }

    #[test]
    fn decay_pos_is_boolean() {
        let c = base();
        let ok = Amendment::AmendParam { field: GovField::DecayPos, old: 0, new: 1 };
        assert!(verify_amendment(&c, &ok).is_ok());
        let bad = Amendment::AmendParam { field: GovField::DecayPos, old: 0, new: 2 };
        assert!(matches!(verify_amendment(&c, &bad), Err(ObligationBreach::OutOfBounds { .. })));
    }

    #[test]
    fn horizon_and_vesting_are_unbounded_u64() {
        let c = base();
        let h = Amendment::AmendParam { field: GovField::Horizon, old: c.horizon, new: u64::MAX };
        assert!(verify_amendment(&c, &h).is_ok());
        let v = Amendment::AmendParam { field: GovField::VestingW, old: c.vesting_w, new: 999_999 };
        assert!(verify_amendment(&c, &v).is_ok());
    }

    // --- stale base: an amendment computed against the wrong current value is refused ---

    #[test]
    fn stale_base_rejected() {
        let c = base(); // threshold_bps == TWO_THIRDS_BPS
        let stale = Amendment::AmendParam { field: GovField::ThresholdBps, old: 9999, new: 8000 };
        assert!(matches!(verify_amendment(&c, &stale), Err(ObligationBreach::StaleBase { .. })));
    }

    // --- mix: normalized + non-negative passes; violations fail (RED twins) ---

    #[test]
    fn mix_reweight_normalized_ok() {
        let c = base();
        let new = Mix { pow: 0.05, pos: 0.35, pom: 0.60 };
        let a = Amendment::AmendMix { old: NCI, new };
        assert!(verify_amendment(&c, &a).is_ok());
    }

    #[test]
    fn mix_not_summing_to_one_rejected() {
        let c = base();
        let new = Mix { pow: 0.5, pos: 0.5, pom: 0.5 }; // sums to 1.5
        let a = Amendment::AmendMix { old: NCI, new };
        assert!(matches!(verify_amendment(&c, &a), Err(ObligationBreach::MixNotNormalized { .. })));
    }

    #[test]
    fn mix_negative_component_rejected() {
        let c = base();
        let new = Mix { pow: -0.1, pos: 0.5, pom: 0.6 }; // sums to 1.0 but negative pow
        let a = Amendment::AmendMix { old: NCI, new };
        assert!(matches!(verify_amendment(&c, &a), Err(ObligationBreach::MixNegative)));
    }

    // --- anti-plutocracy mix bound: capital may never outweigh contribution (pos <= pom) ---

    #[test]
    fn mix_capital_outweighing_contribution_rejected() {
        let c = base();
        // valid probability split (sum 1, non-negative) but pos 0.50 > pom 0.40 — a tilt toward capital.
        let new = Mix { pow: 0.10, pos: 0.50, pom: 0.40 };
        let a = Amendment::AmendMix { old: NCI, new };
        // ANTI-THEATER: drop the `pos <= pom` check in `check_mix` ⇒ this capital-dominant mix validates ⇒ RED.
        assert!(matches!(
            verify_amendment(&c, &a),
            Err(ObligationBreach::CapitalOutweighsContribution { .. })
        ));
    }

    #[test]
    fn mix_capital_equal_to_contribution_ok() {
        let c = base();
        let new = Mix { pow: 0.20, pos: 0.40, pom: 0.40 }; // pos == pom is the allowed boundary
        let a = Amendment::AmendMix { old: NCI, new };
        assert!(verify_amendment(&c, &a).is_ok(), "pos == pom is the knife-edge, allowed");
    }

    #[test]
    fn mix_capital_outweighing_within_epsilon_rejected() {
        let c = base();
        // pos exceeds pom by 1e-10 — capital DOES outweigh contribution, violating `pos <= pom`.
        // ANTI-THEATER: the old `+ 1e-9` axiom tolerance let this tilt validate ⇒ GREEN when it must be RED.
        let new = Mix { pow: 0.3999999999, pos: 0.3000000001, pom: 0.3000000000 };
        let a = Amendment::AmendMix { old: NCI, new };
        assert!(
            matches!(
                verify_amendment(&c, &a),
                Err(ObligationBreach::CapitalOutweighsContribution { .. })
            ),
            "any pos > pom, however small, breaks the anti-plutocracy axiom"
        );
    }

    // --- authority classification: measure -> Contribution, dials -> Stewardship ---

    #[test]
    fn amendment_authority_tiers_measure_vs_dials() {
        let param = |f| Amendment::AmendParam { field: f, old: 0, new: 0 };
        // measure-defining -> Contribution (a say over what counts as contribution is not buyable).
        assert_eq!(param(GovField::ThetaSimQ16).authority(), Some(Authority::Contribution));
        assert_eq!(param(GovField::VestingW).authority(), Some(Authority::Contribution));
        // reweighting the mix / defining the dimension set also shape what contribution is worth.
        assert_eq!(
            Amendment::AmendMix { old: NCI, new: NCI }.authority(),
            Some(Authority::Contribution)
        );
        assert_eq!(Amendment::AddDimension { id: 1 }.authority(), Some(Authority::Contribution));
        // operational dials -> Stewardship (buyable, exitable, bounded).
        assert_eq!(param(GovField::ThresholdBps).authority(), Some(Authority::Stewardship));
        assert_eq!(param(GovField::QuorumFloorBps).authority(), Some(Authority::Stewardship));
        assert_eq!(param(GovField::MaxMempool).authority(), Some(Authority::Stewardship));
        // physics is near-immutable -> no one authorizes.
        assert_eq!(Amendment::AmendPhysics { what: "x" }.authority(), None);
    }

    // --- layer routing: physics rejected, constitutional pending ---

    #[test]
    fn physics_amendment_always_rejected() {
        let c = base();
        let a = Amendment::AmendPhysics { what: "noise floor" };
        assert!(matches!(verify_amendment(&c, &a), Err(ObligationBreach::PhysicsImmutable { .. })));
    }

    #[test]
    fn constitutional_dimension_moves_are_pending() {
        let c = base();
        for a in [
            Amendment::AddDimension { id: 1 },
            Amendment::RetireDimension { id: 1 },
            Amendment::ReweightDimension { id: 1, old_bps: 5000, new_bps: 6000 },
        ] {
            assert_eq!(verify_amendment(&c, &a), Err(ObligationBreach::ConstitutionalPending));
        }
    }

    // --- the obligation checklist is honest: governance clears Socket rows, defers Pragma rows ---

    #[test]
    fn governance_obligations_tag_socket_and_pragma() {
        let a = Amendment::AmendParam { field: GovField::ThetaSimQ16, old: 62259, new: 60000 };
        let obs = a.obligations();
        // exactly two Socket-discharged (Family A trivial + anonymity-relaxation) ...
        assert_eq!(obs.iter().filter(|o| o.by == Discharger::Socket).count(), 2);
        // ... and two deferred to Pragma (attribution preservation + confluence) — Ok never claims these.
        assert_eq!(obs.iter().filter(|o| o.by == Discharger::Pragma).count(), 2);
    }

    #[test]
    fn layer_classification() {
        assert_eq!(Amendment::AmendMix { old: NCI, new: NCI }.layer(), Layer::Governance);
        assert_eq!(Amendment::AddDimension { id: 0 }.layer(), Layer::Constitutional);
        assert_eq!(Amendment::AmendPhysics { what: "x" }.layer(), Layer::Physics);
    }

    // --- Family-B attribution verdicts ---

    fn verdict_for(a: &Amendment, p: AttributionProperty) -> PropertyVerdict {
        attribution_verdicts(a).into_iter().find(|(q, _)| *q == p).unwrap().1
    }

    #[test]
    fn theta_sim_raise_flags_null_player_atrisk_but_passes_bounds() {
        let c = base(); // theta_sim_q16 == 62259
        // a RAISE (more permissive dedup) is within bounds (<= 1<<16) so the gate is Ok ...
        let raise = Amendment::AmendParam { field: GovField::ThetaSimQ16, old: 62259, new: 63000 };
        assert!(verify_amendment(&c, &raise).is_ok());
        // ... yet Family-B flags the null/dummy trade-off (the two gates answer different questions).
        assert!(family_b_at_risk(&raise));
        assert!(matches!(verdict_for(&raise, AttributionProperty::NullPlayer), PropertyVerdict::AtRisk { .. }));
        // the other five properties are untouched by a novelty-floor move.
        for p in [
            AttributionProperty::Efficiency,
            AttributionProperty::Synergy,
            AttributionProperty::MyersonRestriction,
            AttributionProperty::Determinism,
            AttributionProperty::AnonymityRelaxation,
        ] {
            assert!(matches!(verdict_for(&raise, p), PropertyVerdict::PreservedByConstruction { .. }));
        }
    }

    #[test]
    fn theta_sim_lower_preserves_all() {
        let lower = Amendment::AmendParam { field: GovField::ThetaSimQ16, old: 62259, new: 60000 };
        assert!(!family_b_at_risk(&lower));
        assert!(matches!(verdict_for(&lower, AttributionProperty::NullPlayer), PropertyVerdict::PreservedByConstruction { .. }));
    }

    #[test]
    fn mix_and_finalization_params_preserve_all_attribution() {
        // mix never enters pom_scores ...
        let mix = Amendment::AmendMix { old: NCI, new: Mix { pow: 0.05, pos: 0.35, pom: 0.60 } };
        // ... nor does a finalization threshold move.
        let thr = Amendment::AmendParam { field: GovField::ThresholdBps, old: 6667, new: 8000 };
        for a in [&mix, &thr] {
            assert!(!family_b_at_risk(a));
            for (_, v) in attribution_verdicts(a) {
                assert!(matches!(v, PropertyVerdict::PreservedByConstruction { .. }));
            }
        }
    }

    #[test]
    fn constitutional_amendment_defers_attribution_to_pragma() {
        let a = Amendment::AddDimension { id: 1 };
        assert!(!family_b_at_risk(&a)); // deferred is not a trade-off flag
        for (_, v) in attribution_verdicts(&a) {
            assert!(matches!(v, PropertyVerdict::DeferredToPragma { .. }));
        }
    }

    #[test]
    fn every_governable_amendment_covers_all_six_properties() {
        let gov = Amendment::AmendParam { field: GovField::VestingW, old: 0, new: 5 };
        let con = Amendment::RetireDimension { id: 2 };
        assert_eq!(attribution_verdicts(&gov).len(), 6);
        assert_eq!(attribution_verdicts(&con).len(), 6);
        // physics is refused outright ⇒ no attribution evaluation.
        assert!(attribution_verdicts(&Amendment::AmendPhysics { what: "x" }).is_empty());
    }
}
