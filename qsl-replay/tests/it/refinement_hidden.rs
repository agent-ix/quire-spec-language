// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-141 (TC-546): `check_initial` and `check_step` over a refinement with a
//! hidden abstract field (ADR-020 AX-2), over `Coin`, whose abstract `toss`
//! fixes a `side` that the concrete `reveal` shows later.

use ix_trace_rs::trace;
use qsl_eval::simulation::model::{operation_identity_text, ModelSimState};
use qsl_replay::{
    check_initial, check_step, ConcreteStep, RefinementFailure, RefinementPosition, StepRejection,
    StepVerdict, Taken, Undetermined,
};
use quire_exact::Value;
use quire_semantic_value::model_transition::ModelTransition;

use crate::refinement_aux::{
    coin_refines, coin_refines_extended, flag, int, Unit, World, UNLIMITED,
};

/// The concrete state of the one coin `r`.
fn coin(
    world: &World<'_>,
    tossed: bool,
    shown: bool,
    face: i64,
) -> qsl_semantics::model::state::ModelState {
    world.concrete_state(&[
        ("tossed", flag(tossed)),
        ("shown", flag(shown)),
        ("face", int(face)),
    ])
}

/// The concrete transition `operation` takes `pre` to `post`, with the state
/// it reaches, found in the concrete system's own expansion.
fn step(
    world: &World<'_>,
    unit: &Unit,
    pre: &qsl_semantics::model::state::ModelState,
    operation: &str,
    post: &qsl_semantics::model::state::ModelState,
) -> (ModelTransition, ModelSimState) {
    let system = world.concrete_system(UNLIMITED);
    let text =
        operation_identity_text(&unit.concrete_key(&format!("Coin/{operation}"))).expect("a text");
    let wanted = system.state_key_of(post).expect("a key").0;
    system
        .expand_state(pre)
        .expect("the state expands")
        .successors
        .into_iter()
        .find(|(transition, reached)| {
            transition.operation() == text
                && system.state_key_of(reached.state()).expect("a key").0 == wanted
        })
        .unwrap_or_else(|| panic!("{operation} does not take the state to the other"))
}

fn passes(verdict: StepVerdict) -> (RefinementPosition, Taken) {
    let StepVerdict::Passes { post, taken } = verdict else {
        panic!("the step passes: {verdict:?}");
    };
    (post, taken)
}

/// `check_step` on `operation` from `pre` to `post` at `position`.
fn decide(
    world: &World<'_>,
    unit: &Unit,
    pre: &RefinementPosition,
    (operation, post): (&str, &qsl_semantics::model::state::ModelState),
    position: u64,
) -> qsl_replay::StepResult {
    let (transition, reached) = step(world, unit, &pre.concrete, operation, post);
    check_step(
        world.refinement(),
        &world.concrete_system(UNLIMITED),
        &world.abstract_system(UNLIMITED),
        pre,
        &ConcreteStep {
            position,
            transition: Some(&transition),
            post: &reached,
        },
    )
}

/// The `side` of each abstract state of `position`, in its order.
fn sides(world: &World<'_>, position: &RefinementPosition) -> Vec<String> {
    let field = world.abstract_key_of("Coin/side");
    position
        .abstract_states
        .iter()
        .map(|state| {
            let object = &state.populations.values().next().expect("a population")["r"];
            match &object.fields[&field] {
                Value::Integer(integer) => integer.to_string(),
                other => panic!("an integer side, got {other:?}"),
            }
        })
        .collect()
}

fn initial(world: &World<'_>) -> RefinementPosition {
    let system = world.concrete_system(UNLIMITED);
    let (position, taken) = passes(check_initial(
        world.refinement(),
        &system,
        &world.abstract_system(UNLIMITED),
        0,
        system.initial_states()[0].state(),
    ));
    assert_eq!(taken, Taken::Stutter);
    position
}

/// FR-141-AC-4 (TC-546 step 4): after the concrete `toss` the candidate set
/// holds two states, `side` 0 and `side` 1; after `reveal` to `face` 1 it
/// holds the one with `side` 1.
#[trace("TC-546", "FR-141-AC-4")]
#[test]
fn a_hidden_side_is_a_candidate_set_that_a_later_step_narrows() {
    let unit = Unit::coin(1);
    let world = unit.world(&coin_refines(""));
    let start = initial(&world);
    assert_eq!(sides(&world, &start), ["0"]);
    let (tossed, taken) = passes(
        decide(
            &world,
            &unit,
            &start,
            ("toss", &coin(&world, true, false, 0)),
            1,
        )
        .verdict,
    );
    assert!(matches!(taken, Taken::Abstract(identities) if identities.len() == 1));
    assert_eq!(sides(&world, &tossed), ["0", "1"]);
    let (revealed, _) = passes(
        decide(
            &world,
            &unit,
            &tossed,
            ("reveal", &coin(&world, true, true, 1)),
            2,
        )
        .verdict,
    );
    assert_eq!(sides(&world, &revealed), ["1"]);
}

/// FR-141-AC-4 (TC-546 step 4): with the row `side = self.face` added,
/// `reveal` to `face` 1 fails `AbstractStepRejected{transition: show(c),
/// cause: Frame{..}}`, since `show` does not modify `side`.
#[trace("TC-546", "FR-141-AC-4")]
#[test]
fn a_mapped_side_that_changes_at_show_fails_the_frame() {
    let unit = Unit::coin(1);
    let world = unit.world(&coin_refines("side = self.face;"));
    let start = initial(&world);
    let (tossed, _) = passes(
        decide(
            &world,
            &unit,
            &start,
            ("toss", &coin(&world, true, false, 0)),
            1,
        )
        .verdict,
    );
    let StepVerdict::Fails(RefinementFailure::AbstractStepRejected {
        position,
        transition,
        cause,
    }) = decide(
        &world,
        &unit,
        &tossed,
        ("reveal", &coin(&world, true, true, 1)),
        2,
    )
    .verdict
    else {
        panic!("the step is rejected");
    };
    assert_eq!(position, 2);
    assert_eq!(
        transition.operation(),
        operation_identity_text(&unit.abstract_key("Coin/show")).expect("a text")
    );
    assert!(matches!(cause, StepRejection::Frame { .. }), "{cause:?}");
}

/// FR-141-AC-11 (TC-546), the `Coin3` half: `check_initial` on index 0
/// passes and on index 1 and 2 fails `InitialNotAbstract{initial: 1}` and
/// `{initial: 2}`; `toss` with `side` hidden from the initial state at index
/// 0 passes, and the check judged two abstract states, one for each `side`.
#[trace("TC-546", "FR-141-AC-11")]
#[test]
fn three_initial_states_and_a_toss_judge_the_hidden_side_both_ways() {
    let unit = Unit::coin(3);
    let world = unit.world(&coin_refines(""));
    let system = world.concrete_system(UNLIMITED);
    let abstracted = world.abstract_system(UNLIMITED);
    assert_eq!(system.initial_states().len(), 3);
    let verdicts: Vec<StepVerdict> = system
        .initial_states()
        .iter()
        .enumerate()
        .map(|(index, state)| {
            check_initial(
                world.refinement(),
                &system,
                &abstracted,
                index,
                state.state(),
            )
        })
        .collect();
    assert!(matches!(verdicts[0], StepVerdict::Passes { .. }));
    for (index, verdict) in verdicts.iter().enumerate().skip(1) {
        assert!(
            matches!(verdict, StepVerdict::Fails(RefinementFailure::InitialNotAbstract { initial }) if *initial == index),
            "{index}: {verdict:?}"
        );
    }
    let start = initial(&world);
    let result = decide(
        &world,
        &unit,
        &start,
        ("toss", &coin(&world, true, false, 0)),
        1,
    );
    assert_eq!(result.abstract_successors, 2);
    assert!(matches!(result.verdict, StepVerdict::Passes { .. }));
}

/// FR-141 (TC-546): an `any` row over a hidden `side` keeps the candidates
/// that have a successor with the mapped visible part: `reveal -> any` to
/// `face` 1 narrows the two candidates after `toss` to the one with `side` 1.
#[trace("TC-546")]
#[test]
fn an_any_row_keeps_the_candidates_with_a_matching_successor() {
    let unit = Unit::coin(1);
    let rows = coin_refines("").replace(
        "step Impl::Coin::reveal -> Spec::Coin::show(self);",
        "step Impl::Coin::reveal -> any;",
    );
    let world = unit.world(&rows);
    let start = initial(&world);
    let (tossed, _) = passes(
        decide(
            &world,
            &unit,
            &start,
            ("toss", &coin(&world, true, false, 0)),
            1,
        )
        .verdict,
    );
    assert_eq!(sides(&world, &tossed), ["0", "1"]);
    let result = decide(
        &world,
        &unit,
        &tossed,
        ("reveal", &coin(&world, true, true, 1)),
        2,
    );
    let (revealed, taken) = passes(result.verdict);
    assert!(matches!(taken, Taken::Abstract(identities) if identities.len() == 1));
    assert_eq!(sides(&world, &revealed), ["1"]);
}

/// FR-141 (TC-546): a completion of the hidden `side` whose contracts do not
/// decide may be a member of the set after the step, so the step is
/// `Undetermined(UndecidedSuccessor)` and not a pass over the set without it.
/// The abstract `toss` postcondition `1 mod self.side = 0` is undefined at
/// `side` 0 and true at `side` 1.
#[trace("TC-546", "FR-141-AC-12")]
#[test]
fn an_undecided_completion_leaves_the_candidate_set_unknown() {
    let unit = Unit::coin_with_contracts(
        "post SpecTossed using v on Spec::Coin::toss { self.tossed }",
        "post SpecTossed using v on Spec::Coin::toss { self.tossed and 1 mod self.side = 0 }",
    );
    let world = unit.world(&coin_refines(""));
    let start = initial(&world);
    let verdict = decide(
        &world,
        &unit,
        &start,
        ("toss", &coin(&world, true, false, 0)),
        1,
    )
    .verdict;
    assert!(
        matches!(
            verdict,
            StepVerdict::Undetermined(Undetermined::UndecidedSuccessor)
        ),
        "{verdict:?}"
    );
}

/// FR-141, QSpec FR-378 (TC-546): a step after which the candidate set is
/// empty fails `NoAbstractMatch`, whichever candidate and completion the
/// abstract operation rejected. Here `show` can never hold.
#[trace("TC-546")]
#[test]
fn an_empty_candidate_set_is_no_abstract_match() {
    let unit = Unit::coin_with_contracts(
        "post SpecShown using v on Spec::Coin::show { self.shown and self.face = self.side }",
        "post SpecShown using v on Spec::Coin::show { self.shown and self.face = self.side and not self.shown }",
    );
    let world = unit.world(&coin_refines(""));
    let start = initial(&world);
    let (tossed, _) = passes(
        decide(
            &world,
            &unit,
            &start,
            ("toss", &coin(&world, true, false, 0)),
            1,
        )
        .verdict,
    );
    let verdict = decide(
        &world,
        &unit,
        &tossed,
        ("reveal", &coin(&world, true, true, 1)),
        2,
    )
    .verdict;
    assert!(
        matches!(
            verdict,
            StepVerdict::Fails(RefinementFailure::NoAbstractMatch { position: 2 })
        ),
        "{verdict:?}"
    );
}

/// FR-141 (TC-546): a `stutter` step over a hidden `side` keeps the whole
/// candidate set, as an `any` row does when the visible part is unchanged.
#[trace("TC-546")]
#[test]
fn a_stutter_step_keeps_the_candidate_set_for_a_stutter_row_and_an_any_row() {
    let unit = Unit::coin_extended();
    for rows in [
        coin_refines_extended(""),
        coin_refines_extended("").replace("peek -> stutter", "peek -> any"),
    ] {
        let world = unit.world(&rows);
        let start = initial(&world);
        let (tossed, _) = passes(
            decide(
                &world,
                &unit,
                &start,
                ("toss", &coin(&world, true, false, 0)),
                1,
            )
            .verdict,
        );
        assert_eq!(sides(&world, &tossed), ["0", "1"]);
        let (peeked, taken) = passes(
            decide(
                &world,
                &unit,
                &tossed,
                ("peek", &coin(&world, true, false, 0)),
                2,
            )
            .verdict,
        );
        assert_eq!(taken, Taken::Stutter);
        assert_eq!(sides(&world, &peeked), ["0", "1"]);
    }
}

/// FR-141-AC-12 (TC-546): with the pre set `[side 1]` and the concrete
/// `reveal` setting `face` 0, which no completion allows, the step fails
/// `NoAbstractMatch`, not `AbstractStepRejected`.
#[trace("TC-546", "FR-141-AC-12")]
#[test]
fn a_set_of_one_side_that_a_reveal_contradicts_is_no_abstract_match() {
    let unit = Unit::coin(1);
    let world = unit.world(&coin_refines(""));
    let pre = RefinementPosition {
        concrete: coin(&world, true, false, 0),
        history: qsl_replay::HistoryValues::default(),
        abstract_states: vec![world.abstract_state(&[
            ("tossed", flag(true)),
            ("side", int(1)),
            ("shown", flag(false)),
            ("face", int(0)),
        ])],
    };
    let verdict = decide(
        &world,
        &unit,
        &pre,
        ("reveal", &coin(&world, true, true, 0)),
        2,
    )
    .verdict;
    assert!(
        matches!(
            verdict,
            StepVerdict::Fails(RefinementFailure::NoAbstractMatch { position: 2 })
        ),
        "{verdict:?}"
    );
}

/// FR-141 (TC-546): in an `any` row an undecided successor counts only when
/// its visible part is the mapped one. The abstract `show` postcondition is
/// undefined at `face` 1 and true at `face` 0, so `reveal` to `face` 0 passes
/// over the one candidate that matches and is not undetermined by the
/// `face` 1 successor.
#[trace("TC-546")]
#[test]
fn an_undecided_successor_elsewhere_does_not_make_an_any_step_undetermined() {
    let unit = Unit::coin_with_contracts(
        "post SpecShown using v on Spec::Coin::show { self.shown and self.face = self.side }",
        "post SpecShown using v on Spec::Coin::show { self.shown and self.face = self.side and 1 mod (1 - self.face) = 0 }",
    );
    let rows = coin_refines("").replace(
        "step Impl::Coin::reveal -> Spec::Coin::show(self);",
        "step Impl::Coin::reveal -> any;",
    );
    let world = unit.world(&rows);
    let start = initial(&world);
    let (tossed, _) = passes(
        decide(
            &world,
            &unit,
            &start,
            ("toss", &coin(&world, true, false, 0)),
            1,
        )
        .verdict,
    );
    let verdict = decide(
        &world,
        &unit,
        &tossed,
        ("reveal", &coin(&world, true, true, 0)),
        2,
    )
    .verdict;
    let (revealed, _) = passes(verdict);
    assert_eq!(sides(&world, &revealed), ["0"]);
}
