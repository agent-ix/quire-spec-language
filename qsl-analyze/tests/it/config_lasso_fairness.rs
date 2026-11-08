// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-329-AC-4 (TC-839): ADR-018 section 6's three-step update lasso over the
//! ConfigVersion unit (`crate::example_units`), read as a `Lasso::Model`
//! through `replay_model_trace`, which re-executes the steps, closes the
//! loop, decides the lasso's fairness for each constraint of the clause's
//! fairness set and evaluates the formula over it.
//!
//! The lasso EN-1 finds binds `c` to the first object and loops on the other
//! object's update (`upd(b)` three times); the criterion names the mirror
//! image (`upd(a)` looped, `upd(b)` never taken). The two are symmetric: the
//! looped update is taken, the other is enabled throughout and never taken.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_replay::{
    replay_model_trace, ModelTraceReplayResult, ModelTraceSettlement, ModelTraceSubject,
    ModelTraceValue, ReplayLimits, ReplayRefusal, TemporalCounterexample,
};
use quire_semantic_value::fairness::{FairnessConstraint, FairnessGranularity, FairnessKind};

use crate::example_units::{config, decide, receiver, violated, Versions, EACH, WHOLE};
use crate::model_settle::{envelope_at, request};
use crate::units::World;

fn is_two(version: i64) -> Option<bool> {
    Some(version == 2)
}

/// `payload` replayed as the first clause of `world`.
fn replay(
    world: &World,
    payload: TemporalCounterexample,
) -> Result<ModelTraceReplayResult, ReplayRefusal> {
    let letters = Versions::new(world, vec![is_two]);
    let node = &world.package.graph().temporal_clauses()[0];
    let clause_node = node.identity().clone();
    let occurrence = OccurrenceKey::new(clause_node.clone(), node.claim().clone());
    replay_model_trace(
        request(
            &world.package,
            &[],
            &world.domain.document,
            world.snapshots.iter(),
            &world.unit,
        ),
        &ModelTraceSubject {
            timed: None,
            initial: world.initial.clone(),
            universes: world.unlimited().simulation.universes,
            expansion: world.unlimited().simulation.expansion,
        },
        &envelope_at(payload, clause_node, occurrence),
        &letters,
        ReplayLimits::default(),
    )
}

/// FR-329-AC-4: under `fair weak attemptUpdate` the three-step lasso is
/// fair (the loop takes some `attemptUpdate`), so `always eventually
/// holds(versionNumber = 2)` is `Completed(false)` at position 0; the same
/// lasso under `fair weak each attemptUpdate` is unfair, because the other
/// object's update is enabled throughout the loop and never taken, and
/// refuses `invalid_runtime_input`/`invalid-value` naming the constraint.
#[trace("TC-839", "FR-329-AC-4")]
#[test]
fn the_update_lasso_is_false_under_weak_and_refused_unfair_under_weak_each() {
    let weak = config(false, WHOLE);
    let outcome = decide(&weak);
    let lasso_payload = violated(&outcome).clone();
    let lasso = lasso_payload.lasso.expect("a lasso");
    assert_eq!((lasso.entry, lasso.terminal_stutter), (0, false));
    let receivers: Vec<char> = lasso_payload.prefix.iter().map(receiver).collect();
    assert_eq!(receivers.len(), 3, "a three-step loop");
    assert!(
        receivers.iter().all(|taken| *taken == receivers[0]),
        "one object's update, three times: {receivers:?}"
    );

    // Fair weak whole: the lasso is fair, the formula is false at position 0.
    let weak_constraint = weak.clauses[0].fairness().to_vec();
    assert_eq!(weak_constraint.len(), 1);
    assert_eq!(lasso_payload.fairness, weak_constraint);
    match replay(&weak, lasso_payload.clone()).expect("a fair lasso replays") {
        ModelTraceReplayResult {
            settlement:
                ModelTraceSettlement::Reproduced {
                    value: ModelTraceValue::Truth(false),
                    trace_position,
                },
            ..
        } => assert_eq!(trace_position, Some(0)),
        other => panic!("expected a reproduction at position 0, got {other:?}"),
    }

    // Fair weak each: the same steps carry the `each` clause's fairness set.
    let each = config(false, EACH);
    let each_constraints = each.clauses[0].fairness().to_vec();
    let [each_constraint] = each_constraints.as_slice() else {
        panic!("one constraint: {each_constraints:?}");
    };
    assert_eq!(
        each_constraint,
        &FairnessConstraint::new(
            FairnessKind::Weak,
            each_constraint.operation().clone(),
            FairnessGranularity::Each,
        )
    );
    let mut payload = lasso_payload;
    payload.fairness.clone_from(&each_constraints);
    let refused = replay(&each, payload);
    let Err(error) = refused else {
        panic!("expected an unfair lasso, got {refused:?}");
    };
    let ReplayRefusal::UnfairLasso { constraint } = &error else {
        panic!("expected an unfair lasso, got {error:?}");
    };
    assert_eq!(constraint, each_constraint);
    assert_eq!(error.code(), Code::InvalidRuntimeInput);
    assert_eq!(error.cause(), Some("invalid-value"));
}
