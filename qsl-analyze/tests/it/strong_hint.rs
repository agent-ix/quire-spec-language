// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-133 (ADR-019 SV-6): a reproduced liveness lasso names the strong
//! constraints that would exclude it, as warning hints on the item's terminal
//! record beside the counterexample, changing no verdict.
//!
//! Over ADR-018 §6's ConfigVersion unit (`crate::example_units`) and the
//! `Counter` subject (`crate::counter`). Not covered: FR-133-AC-1's mutex,
//! which is ADR-019's subject and needs a strong-fairness engine, and the
//! suppression of a hint for a constraint the clause already holds, which
//! needs a clause with a strong constraint (replay refuses one, QSL-895).

use ix_trace_rs::trace;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::SourceIdentity;
use qsl_replay::{
    compile_package, replay_model_trace, settle_model_check, CertificateRequest, DependencyInput,
    DigestRecord, ModelCheckItem, ModelCheckLimits, ModelCheckOutcome, ModelCheckRecord,
    ModelTraceReplayResult, ModelTraceSubject, ReplayLimits, ResultLabel, StrongFairnessHint,
    TemporalCounterexample, TerminalValue, WireNodeId,
};
use qsl_semantics::check::CheckedTemporalClause;
use quire_exact::Cancel;
use quire_semantic_value::fairness::{FairnessConstraint, FairnessGranularity, FairnessKind};

use crate::counter::{document, document_with_ranged_parameter};
use crate::example_units::{config_over, decide, receiver, violated, Versions, FORMULA, WHOLE};
use crate::lasso_replay::hints_of;
use crate::liveness::{counter, is_zero};
use crate::model_settle::{envelope_at, request};
use crate::units::World;

fn is_two(version: i64) -> Option<bool> {
    Some(version == 2)
}

fn is_three(value: i64) -> Option<bool> {
    Some(value == 3)
}

fn below_two(value: i64) -> Option<bool> {
    Some(value < 2)
}

/// The config world's replay of `payload`, and the record `outcome` settles
/// to through that replay.
fn settled(
    world: &World,
    payload: &TemporalCounterexample,
    outcome: &ModelCheckOutcome,
) -> (ModelTraceReplayResult, ModelCheckRecord) {
    let letters = Versions::new(world, vec![is_two]);
    let id: DigestRecord = compile_package(
        SourceIdentity::new("agent-ix", "counter", "git", "1"),
        "counter.native",
        &world.unit,
        [world.domain.document.as_slice()],
        &DependencyInput::default(),
        qsl_replay::StageLimits::default(),
        ReplayLimits::default(),
    )
    .expect("the unit compiles")
    .package_id();
    let node = &world.package.graph().temporal_clauses()[0];
    let clause_node = WireNodeId::from_digest(*node.identity().as_bytes());
    let occurrence = OccurrenceKey::new(clause_node, node.claim().clone());
    let replay = |counterexample: &TemporalCounterexample| {
        replay_model_trace(
            request(
                &world.domain.document,
                world.snapshots.iter(),
                &world.unit,
                id,
            ),
            &ModelTraceSubject {
                initial: world.initial.clone(),
                universes: world.unlimited().simulation.universes,
                expansion: world.unlimited().simulation.expansion,
            },
            &envelope_at(
                &world.unit,
                id,
                counterexample.clone(),
                clause_node,
                occurrence.clone(),
            ),
            &letters,
            ReplayLimits::default(),
        )
    };
    let record = settle_model_check(
        outcome,
        &ModelCheckLimits::default(),
        &id,
        &CertificateRequest {
            subject: world.unlimited(),
            item: ModelCheckItem::Clause {
                clause: &world.clauses[0],
                letters: &letters,
            },
        },
        &Cancel::new(),
        replay,
    );
    (replay(payload).expect("replays"), record)
}

fn strong(clause: &CheckedTemporalClause, granularity: FairnessGranularity) -> FairnessConstraint {
    let operation = clause.fairness()[0].operation().clone();
    FairnessConstraint::new(FairnessKind::Strong, operation, granularity)
}

/// FR-133-AC-2: ADR-018 §6's ConfigVersion claim refuted under weak
/// `attemptUpdate` with no granularity, its loop three steps of one
/// receiver, carries one hint: `{Strong, attemptUpdate, Each}`, the identity
/// of the other receiver's update (never taken, enabled from the loop's first
/// position), and the record still settles `refuted` with the engine's
/// counterexample. Both bindings give length 3, so the lasso is bound to `a`
/// and loops on `b`, and the hint names `a`'s update.
#[trace("TC-534", "FR-133-AC-2")]
#[test]
fn a_weak_refutation_names_the_strong_each_that_would_exclude_it() {
    let world = config_over(false, WHOLE, FORMULA, (0, 0));
    let outcome = decide(&world);
    let counterexample = violated(&outcome).clone();
    let (replayed, record) = settled(&world, &counterexample, outcome.as_ref().expect("checked"));
    assert_eq!(receiver(&counterexample.prefix[0]), 'b');
    assert_eq!(replayed.hints.len(), 1, "{:?}", replayed.hints);
    let hint = &replayed.hints[0];
    assert_eq!(
        hint.suggested,
        strong(&world.clauses[0], FairnessGranularity::Each)
    );
    assert_eq!(hint.first_enabled_position, 0);
    let identity = hint.identity.as_ref().expect("an identity");
    assert!(identity.operation().contains("attemptUpdate"));
    assert!(
        identity.receiver().expect("a receiver").contains("\"61\""),
        "the identity is the update of `a`, which the loop never takes"
    );
    assert_eq!(StrongFairnessHint::CODE, "fairness.strong-would-exclude");

    assert_eq!(record.value, TerminalValue::Refuted);
    assert_eq!(record.label, ResultLabel::Refuted);
    assert_eq!(record.counterexample.as_ref(), Some(&counterexample));
    assert_eq!(record.hints, replayed.hints);
}

/// FR-133 "Behavior": an operation enabled at a loop state and taken by no
/// loop step is named `Whole`; one whose loop takes some identity but never
/// another enabled one is named `Each` with the first such identity. Over
/// `Counter`'s `setTo` variant, the lasso a weak `inc` constraint forces
/// (`0 -inc-> 1 -setTo(0)-> 0`) takes `inc` and `setTo(0)`, so `inc` has no
/// hint and `setTo` is named `Each` with an identity the loop enables and
/// never takes.
#[trace("TC-534")]
#[test]
fn an_operation_the_loop_takes_in_part_is_named_each_with_the_identity_it_leaves() {
    let fixture = counter(
        document_with_ranged_parameter(),
        "fair weak M::Counter::inc;",
        "always eventually holds(c.value = 3)",
    );
    let outcome = fixture.decide(is_three).expect("checked");
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("expected a violation, got {outcome:?}");
    };
    let hints = hints_of(&fixture, counterexample, is_three);
    assert_eq!(hints.len(), 1, "{hints:?}");
    let hint = &hints[0];
    assert_eq!(hint.suggested.kind(), FairnessKind::Strong);
    assert_eq!(hint.suggested.granularity(), FairnessGranularity::Each);
    assert_eq!(hint.suggested.operation().name(), "setTo");
    let identity = hint.identity.as_ref().expect("an identity");
    assert!(identity.operation().contains("Counter/setTo"));
    assert!(counterexample
        .prefix
        .iter()
        .all(|step| step.transition != *identity));
    assert_eq!(hint.first_enabled_position, 0);

    // With no constraint the loop is `setTo(0)` alone: `inc` is enabled and
    // never taken, so it is named `Whole` with no identity.
    let free = counter(
        document_with_ranged_parameter(),
        "",
        "always eventually holds(c.value = 3)",
    );
    let outcome = free.decide(is_three).expect("checked");
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("expected a violation, got {outcome:?}");
    };
    let hints = hints_of(&free, counterexample, is_three);
    let whole: Vec<_> = hints
        .iter()
        .filter(|hint| hint.suggested.granularity() == FairnessGranularity::Whole)
        .collect();
    assert_eq!(whole.len(), 1, "{hints:?}");
    assert_eq!(whole[0].suggested.operation().name(), "inc");
    assert_eq!(whole[0].identity, None);
    assert_eq!(whole[0].first_enabled_position, 0);
    assert_eq!(hints.len(), 2, "inc whole and setTo each: {hints:?}");
}

/// FR-133-AC-3: the terminal stutter refutation over `Counter` carries no
/// hint (nothing is enabled at its loop), nor does a bounded refutation,
/// which is no lasso.
#[trace("TC-534", "FR-133-AC-3")]
#[test]
fn a_stutter_loop_and_a_prefix_carry_no_hint() {
    let stutter = counter(document(false), "", "always eventually holds(c.value = 0)");
    let outcome = stutter.decide(is_zero).expect("checked");
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("expected a violation, got {outcome:?}");
    };
    assert!(counterexample
        .lasso
        .is_some_and(|lasso| lasso.terminal_stutter));
    assert!(hints_of(&stutter, counterexample, is_zero).is_empty());

    let safety = counter(document(false), "", "always holds(c.value < 2)");
    let outcome = safety.decide(below_two).expect("checked");
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("expected a violation, got {outcome:?}");
    };
    assert!(counterexample.lasso.is_none());
    assert!(hints_of(&safety, counterexample, below_two).is_empty());
}
