// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-487 (TC-1264): the `holds` atoms and `match` conjuncts of a hyper
//! clause are checked nodes, evaluated through the one clause evaluator over
//! ADR-023 section 8's vault (`crate::hyper_world`).

use ix_trace_rs::trace;
use qsl_eval::hyper::{
    CheckedHyperLetters, CheckedStepMatch, HyperLetters, HyperSlot, MatchReading, StepLabel,
    StepMatch,
};
use qsl_eval::model_subject::AtomStop;
use qsl_eval::simulation::model::{ModelSimState, ModelSystem};
use qsl_eval::simulation::TransitionSystem;
use qsl_eval::temporal::{AtomReading, UndefinedCause};
use qsl_eval::value::CheckedPackageEvaluation;
use qsl_semantics::check::{FormulaNode, FormulaNodeId};
use quire_exact::{LimitKind, Meter, Undefined};
use quire_semantic_value::model_transition::ModelTransition;

use crate::counter::{UNLIMITED, ZERO};
use crate::hyper_world::{input_of, instance, non_interference, Vault, World, EQUAL_INPUTS};

/// The one `holds` atom of the world's clause that is the `index`th.
fn atom(world: &World, index: usize) -> FormulaNodeId {
    let position = world
        .clause
        .body()
        .nodes()
        .iter()
        .enumerate()
        .filter(|(_, node)| matches!(node, FormulaNode::Holds { .. }))
        .nth(index)
        .expect("the atom exists")
        .0;
    world
        .clause
        .body()
        .node_id(position)
        .expect("the atom is in the arena")
}

fn system(world: &World) -> ModelSystem<'_> {
    ModelSystem::new(world.subject().simulation).expect("the subject admits")
}

/// The atom `atom` over the component states `states`, one per trace.
fn read(
    world: &World,
    atom: FormulaNodeId,
    states: &[&ModelSimState],
    meter: &mut Meter,
) -> Result<AtomReading, AtomStop> {
    let system = system(world);
    let systems = vec![&system; world.clause.quantifiers().len()];
    CheckedHyperLetters::new(&world.package, &world.clause, systems)
        .expect("the letters admit")
        .atom(atom, &instance(), states, meter)
}

/// FR-487-AC-3: an atom is evaluated over the slots of its traces' components.
#[trace("TC-1264", "FR-487-AC-3")]
#[test]
fn an_atom_is_evaluated_over_its_components_and_is_deterministic() {
    let world = World::new(
        Vault::Secure,
        false,
        &[(0, 0), (0, 1)],
        &non_interference(EQUAL_INPUTS),
    );
    let system = system(&world);
    let initial = system.initial();
    let [low_zero, low_one] = &initial[..] else {
        panic!("two initial states");
    };
    let at = atom(&world, 0);
    let mut meter = Meter::new(UNLIMITED);
    assert_eq!(
        read(&world, at, &[low_zero, low_zero], &mut meter).expect("reads"),
        AtomReading::True
    );
    assert_eq!(
        read(&world, at, &[low_zero, low_one], &mut meter).expect("reads"),
        AtomReading::False
    );
    // Two equal calls give equal readings and equal charges.
    let (mut first, mut second) = (Meter::new(UNLIMITED), Meter::new(UNLIMITED));
    assert_eq!(
        read(&world, at, &[low_zero, low_one], &mut first).expect("reads"),
        read(&world, at, &[low_zero, low_one], &mut second).expect("reads")
    );
    for kind in LimitKind::ALL {
        assert_eq!(first.consumed(kind), second.consumed(kind), "{kind:?}");
    }
    // A meter with a zero budget cannot read the atom.
    let mut empty = Meter::new(ZERO);
    assert!(matches!(
        read(&world, at, &[low_zero, low_one], &mut empty),
        Err(AtomStop::Incomplete(_))
    ));
    // A slot vector of the wrong length is a fault and charges nothing.
    let mut untouched = Meter::new(UNLIMITED);
    let fault = world.package.evaluate_hyper_holds(
        &world.clause,
        at,
        vec![HyperSlot::Bound(quire_exact::Value::Boolean(true))],
        &[],
        &mut untouched,
    );
    assert!(
        matches!(&fault, Err(failure) if format!("{failure:?}").starts_with("Fault")),
        "{fault:?}"
    );
    for kind in LimitKind::ALL {
        assert_eq!(untouched.consumed(kind), 0, "{kind:?}");
    }
}

/// FR-487-AC-3: a divisor that is zero in one component is undefined there.
#[trace("TC-1264", "FR-487-AC-3")]
#[test]
fn an_atom_dividing_by_zero_in_one_component_is_undefined() {
    let world = World::new(
        Vault::Secure,
        false,
        &[(1, 0), (0, 0)],
        "hyper Divided using inf over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V \
         { always holds(1 mod (1 - v.h @ a) = 1 mod (1 - v.h @ b)) }",
    );
    let system = system(&world);
    let initial = system.initial();
    let [high, low] = &initial[..] else {
        panic!("two initial states");
    };
    let mut meter = Meter::new(UNLIMITED);
    assert_eq!(
        read(&world, atom(&world, 0), &[high, low], &mut meter).expect("reads"),
        AtomReading::Undefined(UndefinedCause::Kernel(Undefined::DivisionByZero))
    );
}

/// The `step(0)` and `step(1)` transitions out of the first initial state of
/// `world`.
fn steps(world: &World) -> (ModelTransition, ModelTransition) {
    let system = system(world);
    let state = system.initial()[0].clone();
    let mut found = [None, None];
    for (transition, _) in system.successors(&state).expect("expands").successors {
        match input_of(&StepLabel::Operation(&transition)) {
            Some(0) => found[0] = Some(transition),
            Some(1) => found[1] = Some(transition),
            _ => {}
        }
    }
    let [Some(zero), Some(one)] = found else {
        panic!("step(0) and step(1) are enabled");
    };
    (zero, one)
}

fn universal(
    world: &World,
    labels: &[StepLabel<'_>],
    meter: &mut Meter,
) -> Result<MatchReading, qsl_eval::hyper::MatchIncomplete> {
    CheckedStepMatch::new(&world.package, &world.clause).universal(&instance(), labels, meter)
}

/// FR-487-AC-4: the `match` conjuncts read the labels of a joint step in
/// order and stop at the first that does not hold.
#[trace("TC-1264", "FR-487-AC-4")]
#[test]
fn the_match_conjuncts_are_evaluated_in_order_and_short_circuit() {
    let world = World::new(
        Vault::Secure,
        false,
        &[(0, 0)],
        &non_interference(EQUAL_INPUTS),
    );
    let (zero, one) = steps(&world);
    let mut meter = Meter::new(UNLIMITED);
    let both = |a: StepLabel<'_>, b: StepLabel<'_>, meter: &mut Meter| {
        universal(&world, &[a, b], meter).expect("complete")
    };
    assert_eq!(
        both(
            StepLabel::Operation(&zero),
            StepLabel::Operation(&zero),
            &mut meter
        ),
        MatchReading::Holds
    );
    assert_eq!(
        both(StepLabel::Stutter, StepLabel::Stutter, &mut meter),
        MatchReading::Holds
    );
    let work = |a: StepLabel<'_>, b: StepLabel<'_>| {
        let mut meter = Meter::new(UNLIMITED);
        let reading = both(a, b, &mut meter);
        (reading, meter.consumed(LimitKind::WorkUnits))
    };
    let (differing_inputs, both_conjuncts) =
        work(StepLabel::Operation(&zero), StepLabel::Operation(&one));
    assert_eq!(differing_inputs, MatchReading::Fails);
    let (differing_labels, first_conjunct) = work(StepLabel::Operation(&zero), StepLabel::Stutter);
    assert_eq!(differing_labels, MatchReading::Fails);
    // The second conjunct is neither evaluated nor charged once the first
    // is false.
    assert!(
        first_conjunct < both_conjuncts,
        "{first_conjunct} {both_conjuncts}"
    );
}

/// FR-487-AC-5: a conjunct without a value, or whose meter runs out, ends the
/// reading at that conjunct; one that follows a false conjunct is not charged.
#[trace("TC-1264", "FR-487-AC-5")]
#[test]
fn a_conjunct_without_a_value_or_a_budget_ends_the_reading_there() {
    let divided = "match { a.step.op = b.step.op and \
         a.step.Vault::step.i = b.step.Vault::step.i and \
         (if present(a.step.Vault::step.i) then 1 mod value(a.step.Vault::step.i) else 0) = 0 }";
    let world = World::new(Vault::Secure, false, &[(0, 0)], &non_interference(divided));
    let (zero, one) = steps(&world);
    let mut meter = Meter::new(UNLIMITED);
    assert_eq!(
        universal(
            &world,
            &[StepLabel::Operation(&zero), StepLabel::Operation(&zero)],
            &mut meter
        )
        .expect("complete"),
        MatchReading::Undefined {
            conjunct: 2,
            cause: UndefinedCause::Kernel(Undefined::DivisionByZero)
        }
    );
    // Over `(step(0), step(1))` the second conjunct fails first, and the
    // third charges nothing: the work equals that of the clause without it.
    let plain = World::new(
        Vault::Secure,
        false,
        &[(0, 0)],
        &non_interference(EQUAL_INPUTS),
    );
    let labels = [StepLabel::Operation(&zero), StepLabel::Operation(&one)];
    let (mut with, mut without) = (Meter::new(UNLIMITED), Meter::new(UNLIMITED));
    assert_eq!(
        universal(&world, &labels, &mut with).expect("complete"),
        MatchReading::Fails
    );
    assert_eq!(
        universal(&plain, &labels, &mut without).expect("complete"),
        MatchReading::Fails
    );
    assert_eq!(
        with.consumed(LimitKind::WorkUnits),
        without.consumed(LimitKind::WorkUnits)
    );
    // A budget that covers the first two conjuncts and no more exhausts at
    // the third.
    let mut measured = Meter::new(UNLIMITED);
    universal(
        &plain,
        &[StepLabel::Operation(&zero), StepLabel::Operation(&zero)],
        &mut measured,
    )
    .expect("complete");
    let mut limits = UNLIMITED;
    limits.work_units = measured.consumed(LimitKind::WorkUnits);
    let mut tight = Meter::new(limits);
    let stop = universal(
        &world,
        &[StepLabel::Operation(&zero), StepLabel::Operation(&zero)],
        &mut tight,
    )
    .expect_err("the third conjunct exhausts the meter");
    assert_eq!(stop.conjunct, 2);
}

/// FR-487-AC-11: a call in a conjunct receives the step's argument as an
/// option, present for a `step` and absent for a stutter.
#[trace("TC-1264", "FR-487-AC-11")]
#[test]
fn a_call_in_a_conjunct_receives_the_argument_as_an_option() {
    let contracts = "post Step using v on V::Vault::step { self.l = i }\n\
         function hasInput using v(x: Option<Int[0, 1]>): Boolean pure { present(x) }\n";
    let world = World::new(
        Vault::Custom(contracts),
        false,
        &[(0, 0)],
        &non_interference("match { hasInput(a.step.Vault::step.i) }"),
    );
    let (_, one) = steps(&world);
    let [conjunct] = world.clause.mu_universal() else {
        panic!("one conjunct");
    };
    assert_eq!(conjunct.slots().len(), 1);
    let mut meter = Meter::new(UNLIMITED);
    assert_eq!(
        universal(
            &world,
            &[StepLabel::Operation(&one), StepLabel::Operation(&one)],
            &mut meter
        )
        .expect("complete"),
        MatchReading::Holds
    );
    assert_eq!(
        universal(
            &world,
            &[StepLabel::Stutter, StepLabel::Stutter],
            &mut meter
        )
        .expect("complete"),
        MatchReading::Fails
    );
}
