// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-143 (TC-548): the liveness half of a refinement, checked under the
//! abstract fairness `F_A` on the product the safety half retains.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{check_refinement, RefinementCheckRequest};
use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::model::{operation_identity_text, ModelSimulationRequest};
use qsl_package::CheckedPackage;
use qsl_replay::{
    CounterexampleKind, LivenessHalf, ModelCheckLimits, ModelCheckOutcome, RefinementFailure,
    RefinementOutcome, SafetyOutcome, TemporalCounterexample, UnavailabilityCause,
};
use qsl_semantics::check::CheckedRefinement;
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::Cancel;

use crate::refinement_aux::{
    halting_refinement, hidden_register_history, pair_refinement, Unit, World as AuxWorld,
    UNLIMITED as AUX_UNLIMITED,
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
