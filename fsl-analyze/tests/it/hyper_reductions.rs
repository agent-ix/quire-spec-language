// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-174-AC-7 (TC-3011): a key symmetry and a state constraint selected on an
//! HP-2 item that FR-174's hyper reduction eligibility rule admits are
//! executed, keep the verdict and report `HoldsReduced`; a "yes" selection on
//! an ineligible form settles V-8 before any expansion.

use filament_exact::Cancel;
use fsl_analyze::model_check::ModelCheckRefusal;
use fsl_foundation::ReductionKind;
use fsl_replay::{
    AppliedReduction, HyperOutcome, HyperReductionCause, ModelCheckLimits, ModelTraceSettlement,
    RunStats, SymmetryBrokenCause, SymmetryRefusal,
};
use fsl_route::reductions::{Reductions, SymmetryDeclaration};
use fsl_semantics::check::{HyperForm, TemporalProfile};
use ix_trace_rs::trace;

use crate::hyper::refuted;
use crate::hyper_world::{key, non_interference, opaque, Vault, World, EQUAL_INPUTS, STARTS};

/// The two vaults `v` and `w` of one initial state.
const TWO_VAULTS: [(&str, (i64, i64)); 2] = [("v", (0, 0)), ("w", (1, 0))];

/// The initial states of `keys`, each holding every vault with one secret:
/// closed under every renaming of the keys.
fn uniform_snapshots<'k>(keys: &[&'k str]) -> Vec<Vec<(&'k str, (i64, i64))>> {
    [0, 1]
        .into_iter()
        .map(|secret| keys.iter().map(|key| (*key, (secret, 0))).collect())
        .collect()
}

/// `NonInterference`'s `match`, pairing steps of one vault with equal inputs.
const SAME_VAULT: &str = "match { a.step.op = b.step.op and a.step.receiver = b.step.receiver \
     and a.step.Vault::step.i = b.step.Vault::step.i }";

/// The key symmetry of `vaults` over one class of `keys`.
fn class_of(keys: &[&str]) -> Reductions {
    Reductions {
        symmetry: vec![SymmetryDeclaration {
            population: key("vaults"),
            classes: vec![keys.iter().map(|key| (*key).to_owned()).collect()],
        }],
        ..Reductions::default()
    }
}

/// The key symmetry of `vaults` over `v` and `w`.
fn symmetry() -> Reductions {
    class_of(&["v", "w"])
}

/// [`symmetry`] with the state constraint `constraint` also selected.
fn with_constraint(reductions: Reductions, constraint: &str) -> Reductions {
    Reductions {
        constraint: Some(constraint.to_owned()),
        ..reductions
    }
}

/// The secure vault with the state constraints `Low` (met by every reachable
/// state) and `Zero` (cut on the first step that sets `l`) declared as
/// `at current` clauses of its model.
const SECURE_CONSTRAINED: Vault = Vault::Custom(
    "post Step using v on V::Vault::step { self.l = i }\n\
     invariant Low using v on V::Vault at current { self.l <= 1 }\n\
     invariant Zero using v on V::Vault at current { self.l = 0 }\n",
);

/// `NonInterference` over the vaults `v` and `w` of `vault`.
fn interference(vault: Vault) -> World {
    interference_over(vault, &["v", "w"])
}

/// `NonInterference` over the vaults `keys` of `vault`, whose initial states
/// are closed under every renaming of them.
fn interference_over(vault: Vault, keys: &[&str]) -> World {
    World::with_snapshots(
        vault,
        uniform_snapshots(keys),
        keys,
        &non_interference(SAME_VAULT),
    )
}

fn product_states(outcome: &HyperOutcome) -> u64 {
    match outcome {
        HyperOutcome::Holds { stats, .. } | HyperOutcome::HoldsReduced { stats, .. } => {
            stats.product_states
        }
        other => panic!("expected a holding outcome, got {other:?}"),
    }
}

#[trace("TC-3011", "FR-174-AC-7")]
#[test]
fn a_key_symmetry_keeps_the_verdict_with_fewer_states() {
    let secure = interference(Vault::Secure);
    let unreduced = secure
        .run(ModelCheckLimits::default(), &Cancel::new())
        .expect("not refused");
    let reduced = secure.run_selecting(symmetry()).expect("not refused");
    let HyperOutcome::HoldsReduced { reductions, .. } = &reduced else {
        panic!("expected HoldsReduced, got {reduced:?}");
    };
    assert!(
        matches!(&reductions[..], [AppliedReduction::Symmetry { orbit, .. }, AppliedReduction::CopySwap { .. }] if orbit == &["v".to_owned(), "w".to_owned()]),
        "{reductions:?}"
    );
    assert!(product_states(&reduced) < product_states(&unreduced));

    let leaky = interference(Vault::Leaky);
    let counterexample = refuted(leaky.run_selecting(symmetry()).expect("not refused"));
    assert_eq!(counterexample.traces.len(), 2);
    assert!(matches!(
        leaky.replay(&counterexample),
        Ok(ModelTraceSettlement::Reproduced { .. })
    ));
}

#[trace("TC-3011", "FR-174-AC-7")]
#[test]
fn a_state_constraint_reduces_and_holds_reduced() {
    let secure = interference(SECURE_CONSTRAINED);
    let low = secure
        .run_selecting(with_constraint(Reductions::default(), "Low"))
        .expect("not refused");
    let HyperOutcome::HoldsReduced { reductions, .. } = &low else {
        panic!("expected HoldsReduced under Low, got {low:?}");
    };
    assert!(
        matches!(&reductions[..], [AppliedReduction::CopySwap { .. }]),
        "{reductions:?}"
    );
    let zero = secure
        .run_selecting(with_constraint(Reductions::default(), "Zero"))
        .expect("not refused");
    assert!(
        format!("{zero:?}").contains("ConstraintReached"),
        "{zero:?}"
    );
    let both = secure
        .run_selecting(with_constraint(symmetry(), "Low"))
        .expect("not refused");
    let HyperOutcome::HoldsReduced { reductions, .. } = &both else {
        panic!("expected HoldsReduced, got {both:?}");
    };
    assert!(
        matches!(
            &reductions[..],
            [
                AppliedReduction::Symmetry { .. },
                AppliedReduction::CopySwap { .. }
            ]
        ),
        "{reductions:?}"
    );
}

/// Finite HP-2 still enumerates closed executions when no reduction is
/// selected. Selected product reductions are explicitly unsupported until
/// the finite route executes them; preservation refusals retain precedence.
#[trace("TC-3011", "FR-174-AC-7", "FR-173-AC-6")]
#[test]
fn a_finite_hp2_selection_is_explicitly_unsupported_before_expansion() {
    let world = World::with_snapshots(
        SECURE_CONSTRAINED,
        uniform_snapshots(&["v", "w"]),
        &["v", "w"],
        "hyper Bounded using ep over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V \
         { always [0, 2] holds(v.l @ a <= 1 and v.l @ b <= 1) }",
    );
    assert_eq!(world.clause.form(), Some(&HyperForm::Universal));
    assert_eq!(
        world.clause.profile(),
        TemporalProfile::EventPositionFalseExtension
    );
    let ordinary = world
        .run_selecting(Reductions::default())
        .expect("the finite clause runs");
    let HyperOutcome::HoldsClosed { stats } = ordinary else {
        panic!("expected ordinary closed enumeration, got {ordinary:?}");
    };
    assert!(stats.product_states > 0, "{stats:?}");

    // Zero is true initially but cuts a reachable step setting l to 1.
    // Returning HoldsClosed here would silently bypass that selection.
    for (selection, reduction) in [
        (
            with_constraint(Reductions::default(), "Zero"),
            ReductionKind::StateConstraint,
        ),
        (symmetry(), ReductionKind::Symmetry),
        (with_constraint(symmetry(), "Zero"), ReductionKind::Symmetry),
    ] {
        assert_eq!(
            world
                .run_selecting(selection)
                .expect("the selection settles"),
            HyperOutcome::Reduction {
                cause: HyperReductionCause::Unsupported { reduction },
                stats: RunStats::default(),
            }
        );
    }
    let mut nonpreserving = with_constraint(symmetry(), "Zero");
    nonpreserving.partial_order = true;
    assert_eq!(
        world
            .run_selecting(nonpreserving)
            .expect("the preservation row settles"),
        HyperOutcome::Reduction {
            cause: HyperReductionCause::NotPreserving {
                reduction: ReductionKind::PartialOrder,
                form: HyperForm::Universal,
            },
            stats: RunStats::default(),
        }
    );
}

#[trace("TC-3011", "FR-174-AC-7")]
#[test]
fn a_key_symmetry_on_an_ineligible_form_settles_unsupported() {
    let world = World::with_vaults(Vault::Secure, &TWO_VAULTS, &opaque(""));
    let outcome = world.run_selecting(symmetry()).expect("not refused");
    let HyperOutcome::Reduction { cause, stats } = outcome else {
        panic!("expected a reduction cause, got {outcome:?}");
    };
    assert_eq!(
        cause,
        HyperReductionCause::Unsupported {
            reduction: ReductionKind::Symmetry
        }
    );
    assert_eq!(stats, fsl_replay::RunStats::default());
    // The eligible HP-2 item is not refused by the same selection.
    let eligible = interference(Vault::Secure)
        .run_selecting(symmetry())
        .expect("not refused");
    assert!(!matches!(eligible, HyperOutcome::Reduction { .. }));
}

/// Whether two outcomes give one verdict: both hold, or both are violated.
fn same_verdict(left: &HyperOutcome, right: &HyperOutcome) -> bool {
    let holds = |outcome: &HyperOutcome| {
        matches!(
            outcome,
            HyperOutcome::Holds { .. } | HyperOutcome::HoldsReduced { .. }
        )
    };
    let violated = |outcome: &HyperOutcome| matches!(outcome, HyperOutcome::Violated { .. });
    (holds(left) && holds(right)) || (violated(left) && violated(right))
}

#[trace("TC-3011", "FR-174-AC-7")]
#[test]
fn a_state_constraint_is_eligible_on_every_hp2_item() {
    let power = World::with_second_vaults(
        SECURE_CONSTRAINED,
        &[("v", (0, 0))],
        &[("v", (1, 0))],
        "hyper SavesPower using inf over behaviours (d: V::Vault, e: N::Vault) \
         forall trace s of V forall trace n of N \
         match { s.step.Vault::step.i = n.step.Vault::step.i } \
         { always holds(d.l @ s = e.l @ n) }",
    );
    let each = World::new(
        SECURE_CONSTRAINED,
        false,
        &crate::hyper_world::STARTS,
        "hyper P using inf over behaviours (v: V::Vault) \
         forall trace a of V fair weak each V::Vault::step; \
         forall trace b of V fair weak each V::Vault::step; \
         match { a.step.op = b.step.op } { always holds(v.l @ a = v.l @ b) }",
    );
    for world in [&power, &each] {
        let constrained = world
            .run_selecting(with_constraint(Reductions::default(), "Low"))
            .expect("not refused");
        assert!(
            !matches!(constrained, HyperOutcome::Reduction { .. }),
            "{constrained:?}"
        );
        let unconstrained = world
            .run_selecting(Reductions::default())
            .expect("not refused");
        assert!(
            same_verdict(&constrained, &unconstrained),
            "{constrained:?} against {unconstrained:?}"
        );
    }
}

/// FR-151 "Form": a symmetry declaration naming a key outside the subject's
/// universe is refused before any expansion.
#[trace("TC-3011", "FR-174-AC-7")]
#[test]
fn a_symmetry_over_a_key_outside_the_universe_is_refused() {
    let world = World::new(
        Vault::Secure,
        false,
        &STARTS,
        &non_interference(EQUAL_INPUTS),
    );
    let refusal = world
        .run_selecting(symmetry())
        .expect_err("the key `w` is not in the universe");
    assert!(
        matches!(
            refusal,
            ModelCheckRefusal::Symmetry(SymmetryRefusal::UnknownKey { .. })
        ),
        "{refusal:?}"
    );
}

/// FR-151 "Closed initial states": initial states a declared swap of `v` and
/// `w` maps to no initial state settle `SymmetryBroken` before any expansion.
#[trace("TC-3011", "FR-174-AC-7")]
#[test]
fn initial_states_not_closed_under_the_symmetry_settle_symmetry_broken() {
    let world = World::with_vaults(Vault::Secure, &TWO_VAULTS, &non_interference(EQUAL_INPUTS));
    let outcome = world.run_selecting(symmetry()).expect("not refused");
    let HyperOutcome::Reduction { cause, stats } = outcome else {
        panic!("expected a reduction cause, got {outcome:?}");
    };
    assert!(
        matches!(
            cause,
            HyperReductionCause::SymmetryBroken {
                cause: SymmetryBrokenCause::InitialStatesNotClosed { .. },
                ..
            }
        ),
        "{cause:?}"
    );
    assert_eq!(stats, RunStats::default());
}

/// The diagonal action over a stabiliser that still has keys to swap: the
/// clause binds `v`, so `w` and `x` stay interchangeable in the one instance
/// the orbit `[v, w, x]` leaves. The secure vault holds with fewer product
/// states than the unreduced check, and the leaky vault's counterexample is
/// concretised through the key renaming and replays.
#[trace("TC-3011", "FR-174-AC-7")]
#[test]
fn a_key_symmetry_with_a_nontrivial_stabiliser_keeps_the_verdict() {
    let keys = ["v", "w", "x"];
    let secure = interference_over(Vault::Secure, &keys);
    let unreduced = secure
        .run(ModelCheckLimits::default(), &Cancel::new())
        .expect("not refused");
    let reduced = secure.run_selecting(class_of(&keys)).expect("not refused");
    let HyperOutcome::HoldsReduced { reductions, .. } = &reduced else {
        panic!("expected HoldsReduced, got {reduced:?}");
    };
    let [AppliedReduction::Symmetry {
        orbit, stabiliser, ..
    }, AppliedReduction::CopySwap { .. }] = &reductions[..]
    else {
        panic!("expected a symmetry then a copy swap, got {reductions:?}");
    };
    assert_eq!(orbit, &["v", "w", "x"]);
    let [class] = &stabiliser[..] else {
        panic!("expected one stabiliser class, got {stabiliser:?}");
    };
    assert_eq!(class.keys, ["w", "x"]);
    assert!(product_states(&reduced) < product_states(&unreduced));

    let leaky = interference_over(Vault::Leaky, &keys);
    let counterexample = refuted(leaky.run_selecting(class_of(&keys)).expect("not refused"));
    assert!(matches!(
        leaky.replay(&counterexample),
        Ok(ModelTraceSettlement::Reproduced { .. })
    ));
}

/// FR-174-AC-4: a state constraint that does not preserve HP-3 is rejected
/// before any state is expanded.
#[trace("TC-599", "FR-174-AC-4")]
#[test]
fn a_selected_nonpreserving_reduction_is_refused_before_expansion() {
    let world = World::new(
        Vault::Secure,
        false,
        &STARTS,
        "hyper Opaque using inf over behaviours (v: V::Vault) \
         forall trace a of V exists trace b of V \
         match { a.step.Vault::step.i = b.step.Vault::step.i } \
         { holds(v.h @ a != v.h @ b) and always holds(v.l @ a = v.l @ b) }",
    );
    let selected = Reductions {
        constraint: Some("Bounded".to_owned()),
        ..Reductions::default()
    };
    let outcome = world
        .run_selecting(selected)
        .expect("the reduction selection parses");
    assert_eq!(
        outcome,
        HyperOutcome::Reduction {
            cause: HyperReductionCause::NotPreserving {
                reduction: ReductionKind::StateConstraint,
                form: HyperForm::ForallExistsSafety,
            },
            stats: RunStats::default(),
        }
    );
}
