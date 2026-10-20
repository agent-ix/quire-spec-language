// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-143 (TC-548): the liveness half of a refinement, checked under the
//! abstract fairness `F_A` on the product the safety half retains.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};

use ix_trace_rs::trace;
use qsl_analyze::model_check::{check_refinement, RefinementCheckRequest};
use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::model::{operation_identity_text, ModelSimulationRequest};
use qsl_eval::simulation::Limits;
use qsl_package::CheckedPackage;
use qsl_replay::{
    CounterexampleKind, IncompleteCause, LivenessHalf, ModelCheckLimit, ModelCheckLimits,
    ModelCheckOutcome, ReachedLimit, RefinementFailure, RefinementOutcome, SafetyOutcome,
    TemporalCounterexample, UnavailabilityCause, UndecidedCause,
};
use qsl_semantics::check::CheckedRefinement;
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::{Cancel, CancelCause};

use crate::refinement_aux::{
    halting_refinement, hidden_register_history, pair_refinement, probe_contracts,
    probe_refinement, Unit, World as AuxWorld, NEVER, UNLIMITED as AUX_UNLIMITED,
};
use crate::refinement_fixture::{
    cas_refines_counter, divergence_refinement, key, mutated, World, CAS, UNLIMITED,
};

fn check(world: &World) -> RefinementOutcome {
    run(
        world.refinement(),
        world.concrete_request(UNLIMITED),
        world.package(),
        world.abstract_initial(),
    )
}

fn check_aux(world: &AuxWorld<'_>) -> RefinementOutcome {
    run(
        world.refinement(),
        world.concrete_request(AUX_UNLIMITED),
        world.package(),
        world.abstract_initial(),
    )
}

fn run(
    refinement: &CheckedRefinement,
    concrete: ModelSimulationRequest<'_>,
    package: &CheckedPackage,
    abstract_initial: DocumentRef,
) -> RefinementOutcome {
    check_refinement(
        RefinementCheckRequest {
            refinement,
            concrete: ModelSubject {
                simulation: concrete,
                proof_bounds: Vec::new(),
            },
            abstract_package: package,
            abstract_initial: vec![abstract_initial],
            max_depth: None,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect("the refinement is checked")
}

fn without_ensure(refinement: &str) -> String {
    mutated(refinement, "ensure fair weak each Spec::Counter::inc;", "")
}

fn holds(outcome: &RefinementOutcome) {
    assert!(matches!(outcome.safety, SafetyOutcome::Holds { .. }));
    let Some(LivenessHalf::Checked(ModelCheckOutcome::HoldsReduced { reductions, .. })) =
        &outcome.liveness
    else {
        panic!("the liveness half holds: {:?}", outcome.liveness);
    };
    assert!(reductions.is_empty());
}

/// The lasso `outcome`'s liveness half refutes with.
fn lasso(outcome: RefinementOutcome) -> TemporalCounterexample {
    assert!(matches!(outcome.safety, SafetyOutcome::Holds { .. }));
    let Some(LivenessHalf::Checked(ModelCheckOutcome::Violated { counterexample, .. })) =
        outcome.liveness
    else {
        panic!("the liveness half is violated: {:?}", outcome.liveness);
    };
    *counterexample
}

/// The operations of `counterexample`'s prefix, by name.
fn operations(counterexample: &TemporalCounterexample) -> Vec<&'static str> {
    let names = [
        "beginA", "beginB", "commitA", "commitB", "retryA", "retryB", "peek",
    ];
    counterexample
        .prefix
        .iter()
        .map(|step| {
            *names
                .iter()
                .find(|name| {
                    operation_identity_text(&key(CAS, &format!("Counter/{name}")))
                        .is_ok_and(|text| text == step.transition.operation())
                })
                .expect("a step of the concrete model")
        })
        .collect()
}

/// FR-143-AC-1 (TC-548 step 1): `CasRefinesCounter` holds both halves; its
/// divergence variant is refuted by an empty stem and a loop of one `peek`
/// step, failure `Divergence{inc}`; without the `ensure` row the same model
/// has no liveness half and a holding safety half.
#[trace("TC-548", "FR-143-AC-1")]
#[test]
fn the_compare_and_set_counter_holds_and_its_divergence_is_a_peek_loop() {
    holds(&check(&World::new(&cas_refines_counter())));

    let counterexample = lasso(check(&World::new(&divergence_refinement())));
    assert_eq!(counterexample.initial_state, 0);
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(
        counterexample
            .lasso
            .map(|lasso| (lasso.entry, lasso.terminal_stutter)),
        Some((0, false))
    );
    assert_eq!(operations(&counterexample), ["peek"]);
    let Some(RefinementFailure::Divergence { constraint }) = &counterexample.refinement else {
        panic!(
            "the failure is a divergence: {:?}",
            counterexample.refinement
        );
    };
    assert_eq!(constraint.operation().name(), "inc");

    let safety_only = check(&World::new(&without_ensure(&divergence_refinement())));
    assert!(matches!(safety_only.safety, SafetyOutcome::Holds { .. }));
    assert_eq!(safety_only.liveness, None);
}

/// The `terminal` declarations of FR-143-AC-2's combinations: each model
/// takes none, `terminal when self.value = 2`, or `terminal any`.
fn terminal_combinations() -> Vec<String> {
    let forms = |model: &str| {
        [
            String::new(),
            format!("terminal {model}Done using v on {model} when {{ self.value = 2 }};"),
            format!("terminal {model}Done using v on {model} any;"),
        ]
    };
    forms("Spec")
        .iter()
        .flat_map(|spec| forms("Impl").map(|concrete| format!("{spec}\n{concrete}")))
        .collect()
}

/// FR-143-AC-2 (TC-548 step 2): `Halting` is refuted by the stem `inc` and
/// the terminal stutter at `value` 1 with `Divergence{inc}`; without its
/// `ensure` row it has no liveness half; with the precondition
/// `self.value < 2` it holds. Each verdict is the same under every
/// combination of `terminal` declarations.
#[trace("TC-548", "FR-143-AC-2")]
#[test]
fn a_concrete_halt_is_a_divergence_whatever_the_terminal_declarations() {
    for terminals in terminal_combinations() {
        let unit = Unit::halting(1, &terminals);
        let counterexample = lasso(check_aux(&unit.world(&halting_refinement(true))));
        let inc = operation_identity_text(&unit.concrete_key("Counter/inc")).expect("a text");
        let stem: Vec<&str> = counterexample
            .prefix
            .iter()
            .map(|step| step.transition.operation())
            .collect();
        assert_eq!(stem, [inc.as_str()], "{terminals}");
        assert_eq!(
            counterexample
                .lasso
                .map(|lasso| (lasso.entry, lasso.terminal_stutter)),
            Some((1, true)),
            "{terminals}"
        );
        let Some(RefinementFailure::Divergence { constraint }) = &counterexample.refinement else {
            panic!("{terminals}: a divergence: {:?}", counterexample.refinement);
        };
        assert_eq!(constraint.operation().name(), "inc", "{terminals}");

        let safety_only = check_aux(&unit.world(&halting_refinement(false)));
        assert!(matches!(safety_only.safety, SafetyOutcome::Holds { .. }));
        assert_eq!(safety_only.liveness, None, "{terminals}");

        holds(&check_aux(
            &Unit::halting(2, &terminals).world(&halting_refinement(true)),
        ));
    }
}

/// FR-143-AC-3 (TC-548 step 3): with `flipA` fair and `flipB` not, a loop of
/// two `flipA` steps never takes the fair abstract `flipB`, and the failure
/// is `AbstractUnfair{flipB}`; making `flipB` fair as well holds.
#[trace("TC-548", "FR-143-AC-3")]
#[test]
fn a_loop_that_takes_abstract_steps_but_never_flip_b_is_abstract_unfair() {
    let unit = Unit::pair();
    let counterexample = lasso(check_aux(&unit.world(&pair_refinement(&["flipA"]))));
    assert_eq!(counterexample.prefix.len(), 2);
    assert_eq!(
        counterexample
            .lasso
            .map(|lasso| (lasso.entry, lasso.terminal_stutter)),
        Some((0, false))
    );
    let Some(RefinementFailure::AbstractUnfair { constraint }) = &counterexample.refinement else {
        panic!("an abstract unfairness: {:?}", counterexample.refinement);
    };
    assert_eq!(constraint.operation().name(), "flipB");
    let flipped: Vec<&str> = counterexample
        .prefix
        .iter()
        .map(|step| step.transition.operation())
        .collect();
    let flip_a = operation_identity_text(&unit.concrete_key("Pair/flipA")).expect("a text");
    assert_eq!(flipped, [flip_a.as_str(), flip_a.as_str()]);

    holds(&check_aux(
        &unit.world(&pair_refinement(&["flipA", "flipB"])),
    ));
}

/// FR-143-AC-4 (TC-548 step 4): `RegisterHistory` without its `prev` row
/// holds its safety half and settles its liveness half `Unsupported` with
/// an `ensure` row, and has none without it.
#[trace("TC-548", "FR-143-AC-4")]
#[test]
fn liveness_through_a_hidden_abstract_field_is_unsupported() {
    let unit = Unit::register();
    let ensured = hidden_register_history();
    let outcome = check_aux(&unit.world(&ensured));
    assert!(matches!(outcome.safety, SafetyOutcome::Holds { .. }));
    assert_eq!(
        outcome.liveness,
        Some(LivenessHalf::Unsupported(
            UnavailabilityCause::HiddenFieldLiveness
        ))
    );

    let outcome =
        check_aux(&unit.world(&ensured.replace("ensure fair weak Spec::Register::write;\n", "")));
    assert!(matches!(outcome.safety, SafetyOutcome::Holds { .. }));
    assert_eq!(outcome.liveness, None);
}

/// `Probe` over the concrete guards `concrete` (`flipA`, `flipB`, `setA`,
/// `peek`) and the abstract ones `abstract_` (`flipA`, `flipB`, `setA`),
/// starting at `initials`.
fn probe(concrete: [&str; 4], abstract_: [&str; 3], initials: &[(i64, i64)]) -> Unit {
    Unit::probe_pair(probe_contracts(concrete, abstract_), initials)
}

/// The concrete model that can only `peek`, the abstract one unguarded.
fn peeker() -> Unit {
    probe([NEVER, NEVER, NEVER, ""], ["", "", ""], &[(0, 0)])
}

fn operation_of(outcome: RefinementOutcome) -> String {
    match lasso(outcome).refinement {
        Some(
            RefinementFailure::Divergence { constraint }
            | RefinementFailure::AbstractUnfair { constraint },
        ) => constraint.operation().name().to_owned(),
        other => panic!("a liveness failure: {other:?}"),
    }
}

/// FR-143 Behavior (TC-548): a strong constraint in `F_A` is not decided by
/// the weak reading the second phase has, so the liveness half is
/// `Unsupported` with the proved safety half kept beside it, whatever else
/// `F_A` holds; a weak `F_A` is decided as before. The loop `flipA, flipA`
/// of the refinement visits `a = 0` forever and never takes the strong
/// `flipB`.
#[trace("TC-548")]
#[test]
fn a_strong_abstract_fairness_constraint_is_unsupported() {
    let unit = probe(
        ["", "self.a = 0", "", ""],
        ["", "self.a = 0", ""],
        &[(0, 0)],
    );
    let flip_a = "assume fair weak each Impl::Pair::flipA;";
    let weak = "ensure fair weak whole Spec::Pair::flipB;";
    let strong = "ensure fair strong whole Spec::Pair::flipB;";
    for (rows, supported) in [
        (vec![flip_a, weak], true),
        (vec![flip_a, strong], false),
        (vec![flip_a, weak, strong], false),
        (vec![flip_a, strong, weak], false),
    ] {
        let outcome = check_aux(&unit.world(&probe_refinement(&rows)));
        assert!(
            matches!(outcome.safety, SafetyOutcome::Holds { .. }),
            "{rows:?}"
        );
        if supported {
            holds(&outcome);
        } else {
            assert_eq!(
                outcome.liveness,
                Some(LivenessHalf::Unsupported(
                    UnavailabilityCause::StrongAbstractFairness
                )),
                "{rows:?}"
            );
        }
    }
}

/// FR-143 Second phase (TC-548): the classes are taken in `F_A`'s source
/// order, so of two violated constraints the first written is the one named.
#[trace("TC-548")]
#[test]
fn the_first_violated_constraint_in_source_order_is_named() {
    let unit = peeker();
    let flip_a = "ensure fair weak whole Spec::Pair::flipA;";
    let flip_b = "ensure fair weak whole Spec::Pair::flipB;";
    let named = |rows: &[&str]| operation_of(check_aux(&unit.world(&probe_refinement(rows))));
    assert_eq!(named(&[flip_a, flip_b]), "flipA");
    assert_eq!(named(&[flip_b, flip_a]), "flipB");
}

/// FR-143 Second phase (TC-548): an `each` constraint is one class for each
/// abstract transition identity and a `whole` constraint one class for the
/// operation. Where only `setA` runs, a loop of `setA(1)` leaves `setA(0)`
/// enabled and never taken, which violates `each` and not `whole`.
#[trace("TC-548")]
#[test]
fn an_each_constraint_is_a_class_for_each_identity_and_a_whole_one_for_the_operation() {
    let unit = probe([NEVER, NEVER, "", NEVER], ["", "", ""], &[(0, 0)]);
    let whole = unit.world(&probe_refinement(&[
        "ensure fair weak whole Spec::Pair::setA;",
    ]));
    holds(&check_aux(&whole));
    let each = unit.world(&probe_refinement(&[
        "ensure fair weak each Spec::Pair::setA;",
    ]));
    let counterexample = lasso(check_aux(&each));
    assert!(matches!(
        counterexample.refinement,
        Some(RefinementFailure::AbstractUnfair { .. })
    ));
    assert_eq!(
        counterexample.prefix.len() - counterexample.lasso.expect("a lasso").entry,
        1
    );
}

/// FR-143 Outputs (TC-548): the lasso starts from the initial state the
/// behaviour begins at. Of two initial states only the second can peek
/// forever, and the lasso's `initial_state` is 1.
#[trace("TC-548")]
#[test]
fn the_lasso_names_the_initial_state_it_starts_from() {
    let unit = probe(
        [NEVER, NEVER, NEVER, "self.a = 1"],
        ["", "self.a = 1", ""],
        &[(0, 0), (1, 0)],
    );
    let world = unit.world(&probe_refinement(&[
        "ensure fair weak whole Spec::Pair::flipB;",
    ]));
    let outcome = check_refinement(
        RefinementCheckRequest {
            refinement: world.refinement(),
            concrete: ModelSubject {
                simulation: world.concrete_request(AUX_UNLIMITED),
                proof_bounds: Vec::new(),
            },
            abstract_package: world.package(),
            abstract_initial: world.abstract_initials(),
            max_depth: None,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect("the refinement is checked");
    let counterexample = lasso(outcome);
    assert_eq!(counterexample.initial_state, 1);
    assert!(counterexample.prefix.is_empty() || counterexample.lasso.is_some());
}

fn peeker_run(limits: ModelCheckLimits, cancel: &Cancel) -> RefinementOutcome {
    let unit = peeker();
    let world = unit.world(&probe_refinement(&[
        "ensure fair weak whole Spec::Pair::flipA;",
        "ensure fair weak whole Spec::Pair::flipB;",
    ]));
    check_refinement(
        RefinementCheckRequest {
            refinement: world.refinement(),
            concrete: ModelSubject {
                simulation: world.concrete_request(AUX_UNLIMITED),
                proof_bounds: Vec::new(),
            },
            abstract_package: world.package(),
            abstract_initial: vec![world.abstract_initial()],
            max_depth: None,
            limits,
        },
        cancel,
    )
    .expect("the refinement is checked")
}

fn stopped(outcome: &RefinementOutcome) -> Option<(IncompleteCause, Option<ReachedLimit>)> {
    match &outcome.liveness {
        Some(LivenessHalf::Checked(ModelCheckOutcome::Stopped { cause, limit, .. })) => {
            Some((*cause, *limit))
        }
        _ => None,
    }
}

/// FR-143 Behavior (TC-548): the second phase respects `max_transitions`,
/// which the safety half has not used up, and returns `Stopped` naming it.
#[trace("TC-548")]
#[test]
fn the_second_phase_stops_at_the_transition_budget() {
    let full = peeker_run(ModelCheckLimits::default(), &Cancel::new());
    let SafetyOutcome::Holds { stats, .. } = &full.safety else {
        panic!("the safety half holds");
    };
    let used = usize::try_from(stats.product_edges).expect("a count");
    let limits = ModelCheckLimits {
        limits: Limits {
            max_transitions: used,
            ..Limits::default()
        },
        ..ModelCheckLimits::default()
    };
    let outcome = peeker_run(limits, &Cancel::new());
    assert!(matches!(outcome.safety, SafetyOutcome::Holds { .. }));
    assert_eq!(
        stopped(&outcome),
        Some((
            IncompleteCause::ResourceExhausted,
            Some(ReachedLimit {
                limit: ModelCheckLimit::MaxTransitions,
                value: u64::try_from(used).expect("a count"),
            })
        ))
    );
}

/// FR-143 Behavior (TC-548): a handle cancelled during the second phase
/// stops it with `Stopped{Cancelled}`, the safety half already holding.
#[trace("TC-548")]
#[test]
fn a_cancellation_during_the_second_phase_stops_it() {
    let charges = Arc::new(AtomicU32::new(0));
    let counter = Arc::clone(&charges);
    let counting = Cancel::observing(move || {
        counter.fetch_add(1, Ordering::Relaxed);
    });
    peeker_run(ModelCheckLimits::default(), &counting);
    let total = charges.load(Ordering::Relaxed);
    let cancelled_at = |at: u32| {
        let seen = Arc::new(AtomicU32::new(0));
        let handle: Arc<OnceLock<Cancel>> = Arc::default();
        let (count, inner) = (Arc::clone(&seen), Arc::clone(&handle));
        let cancel = Cancel::observing(move || {
            if count.fetch_add(1, Ordering::Relaxed) + 1 == at {
                if let Some(cancel) = inner.get() {
                    cancel.cancel(CancelCause::Requested);
                }
            }
        });
        assert!(handle.set(cancel.clone()).is_ok(), "set once");
        peeker_run(ModelCheckLimits::default(), &cancel)
    };
    let mut live = 0;
    for at in 1..=total {
        let outcome = cancelled_at(at);
        if let Some((cause, limit)) = stopped(&outcome) {
            assert_eq!(
                cause,
                IncompleteCause::Cancelled {
                    source: CancelCause::Requested
                }
            );
            assert_eq!(limit, None);
            assert!(matches!(outcome.safety, SafetyOutcome::Holds { .. }));
            live += 1;
        }
    }
    assert!(live > 0, "a charge of the second phase is cancelled");
}

/// FR-143 Behavior (TC-548): an abstract contract conjunction that does not
/// decide, here a postcondition of `flipB` undefined at `b = 1`, leaves the
/// enabledness of the class unknown, so the half is `Undecided`.
#[trace("TC-548")]
#[test]
fn an_undecided_abstract_contract_leaves_the_liveness_half_undecided() {
    let contracts = probe_contracts([NEVER, NEVER, NEVER, ""], ["", "", ""]).replace(
        "post SpecFlipB using v on Spec::Pair::flipB { self.b = 1 - pre(self.b) }",
        "post SpecFlipB using v on Spec::Pair::flipB { 1 mod (1 - pre(self.b)) >= 0 and self.b = 1 - pre(self.b) }",
    );
    let unit = Unit::probe_pair(contracts, &[(0, 1)]);
    let outcome = check_aux(&unit.world(&probe_refinement(&[
        "ensure fair weak whole Spec::Pair::flipB;",
    ])));
    assert!(matches!(outcome.safety, SafetyOutcome::Holds { .. }));
    assert!(matches!(
        outcome.liveness,
        Some(LivenessHalf::Checked(ModelCheckOutcome::Undecided {
            cause: UndecidedCause::UndecidedSuccessor,
            ..
        }))
    ));
}

/// FR-143 Behavior (TC-548): a refinement with a hidden abstract field and a
/// strong `F_A` constraint is reported `HiddenFieldLiveness`, which is
/// checked first.
#[trace("TC-548")]
#[test]
fn a_hidden_field_is_reported_before_a_strong_constraint() {
    let unit = Unit::register();
    let strong = hidden_register_history().replace("ensure fair weak", "ensure fair strong");
    let outcome = check_aux(&unit.world(&strong));
    assert_eq!(
        outcome.liveness,
        Some(LivenessHalf::Unsupported(
            UnavailabilityCause::HiddenFieldLiveness
        ))
    );
}
