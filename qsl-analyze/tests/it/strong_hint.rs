// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-133 (ADR-019 SV-6): a reproduced liveness lasso names the strong
//! constraints that would exclude it, as warning hints on the item's terminal
//! record beside the counterexample, changing no verdict.
//!
//! Over ADR-018 section 6's ConfigVersion unit (`crate::example_units`), the
//! `Counter` subject (`crate::counter`) and ADR-019 section 6's mutex written
//! over `Counter`.

use ix_trace_rs::trace;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::SourceIdentity;
use qsl_replay::{
    compile_package, replay_model_trace, settle_model_check, CertificateRequest,
    CounterexampleKind, DependencyInput, DigestRecord, ModelCheckItem, ModelCheckLimits,
    ModelCheckOutcome, ModelCheckRecord, ModelTraceReplayResult, ModelTraceSettlement,
    ModelTraceSubject, ReplayLimits, ResultLabel, StrongFairnessHint, TemporalCounterexample,
    TerminalValue, WireNodeId,
};
use qsl_semantics::check::CheckedTemporalClause;
use quire_exact::Cancel;
use quire_semantic_value::fairness::{FairnessConstraint, FairnessGranularity, FairnessKind};

use crate::counter::{document, document_with_ranged_parameter, CONTRACT};
use crate::example_units::{config_over, decide, receiver, violated, Versions, FORMULA, WHOLE};
use crate::lasso_replay::{bounded_fixture, hints_of};
use crate::liveness::{counter, is_zero};
use crate::model_settle;
use crate::model_settle::{envelope_at, request};
use crate::units::World;

fn is_two(version: i64) -> Option<bool> {
    Some(version == 2)
}

fn is_three(value: i64) -> Option<bool> {
    Some(value == 3)
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
/// hint (nothing is enabled at its loop), nor does FR-126-AC-2's bounded
/// refutation (`eventually[0,1] holds(c.value = 3)`, the prefix `0 -inc-> 1`)
/// or the deadlock-freedom refutation (`0 -inc-> 1 -inc-> 2 -inc-> 3`), which
/// are no lasso.
#[trace("TC-534", "FR-133-AC-3")]
#[test]
fn a_stutter_loop_a_bounded_prefix_and_a_deadlock_carry_no_hint() {
    let stutter = counter(document(false), "", "always eventually holds(c.value = 0)");
    let outcome = stutter.decide(is_zero).expect("checked");
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("expected a violation, got {outcome:?}");
    };
    assert!(counterexample
        .lasso
        .is_some_and(|lasso| lasso.terminal_stutter));
    assert!(hints_of(&stutter, counterexample, is_zero).is_empty());

    let bounded = bounded_fixture("eventually[0,1] holds(c.value = 3)");
    let outcome = bounded.decide(is_three).expect("checked");
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("expected a violation, got {outcome:?}");
    };
    assert!(counterexample.lasso.is_none());
    assert_eq!(counterexample.prefix.len(), 1);
    assert!(hints_of(&bounded, counterexample, is_three).is_empty());

    let case = model_settle::case(CONTRACT, &[0]);
    let deadlock = model_settle::counterexample(&case);
    assert_eq!(deadlock.kind, CounterexampleKind::Deadlock);
    let replayed = model_settle::replay(&case, deadlock).expect("replays");
    assert!(matches!(
        replayed.settlement,
        ModelTraceSettlement::Reproduced { .. }
    ));
    assert!(replayed.hints.is_empty());
}

/// FR-133-AC-1's `weak each` half over ADR-019 section 6's mutex, written
/// over `Counter`: `acquire(p)` is `setTo(n)` (precondition `value = 0`,
/// `n` in 1 to 2) and `release` is `inc` (precondition `value != 0`,
/// post-state 0). Under `fair weak each acquire`, `always eventually
/// holds(value = 1)` is refuted by `0 -acq(2)-> 2 -rel-> 0` with an empty
/// stem; the one hint is `{Strong, acquire, Each}`, identity `acq(1)`, loop
/// position 0, and `release`, taken in the loop, gets none.
#[trace("TC-534", "FR-133-AC-1")]
#[test]
fn the_weak_each_mutex_refutation_names_acq_one_at_loop_position_zero() {
    let fixture = mutex("fair weak each M::Counter::setTo;");
    let outcome = fixture.decide(is_one).expect("checked");
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("expected a violation, got {outcome:?}");
    };
    assert_eq!(fixture.values(counterexample, "c"), [0, 2, 0]);
    assert_eq!(counterexample.lasso.map(|lasso| lasso.entry), Some(0));
    let hints = hints_of(&fixture, counterexample, is_one);
    let record = crate::strong_fairness::settle_en1(&fixture, &outcome, is_one);
    assert_eq!(record.value, TerminalValue::Refuted);
    assert_eq!(
        record.counterexample.as_ref(),
        Some(counterexample.as_ref())
    );
    assert_eq!(record.hints, hints);
    assert_eq!(hints.len(), 1, "{hints:?}");
    let hint = &hints[0];
    assert_eq!(hint.suggested.kind(), FairnessKind::Strong);
    assert_eq!(hint.suggested.granularity(), FairnessGranularity::Each);
    assert_eq!(hint.suggested.operation().name(), "setTo");
    assert_eq!(hint.first_enabled_position, 0);
    let identity = hint.identity.as_ref().expect("an identity");
    assert!(identity.operation().contains("Counter/setTo"));
    assert!(counterexample
        .prefix
        .iter()
        .all(|step| step.transition != *identity));

    // The same loop behind a stem: the position is the loop's, not the
    // trace's, so it is still 0 where `acq(1)` is first enabled in the loop.
    let mut stemmed = counterexample.clone();
    let loop_steps = stemmed.prefix.clone();
    stemmed.prefix.extend(loop_steps);
    stemmed.lasso = Some(qsl_replay::Lasso {
        entry: 2,
        terminal_stutter: false,
    });
    let hints = hints_of(&fixture, &stemmed, is_one);
    assert_eq!(hints.len(), 1, "{hints:?}");
    assert_eq!(hints[0].first_enabled_position, 0);
}

/// FR-133-AC-1's `strong` (whole) half: under `fair strong acquire` the
/// mutex is refuted by the same lasso, and the clause's whole constraint does
/// not suppress the `each` hint, which names `acq(1)` at loop position 0.
#[trace("TC-534", "FR-133-AC-1")]
#[test]
fn the_strong_whole_mutex_refutation_carries_the_same_hint() {
    let fixture = mutex("fair strong M::Counter::setTo;");
    let outcome = fixture.decide(is_one).expect("checked");
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("expected a violation, got {outcome:?}");
    };
    assert_eq!(fixture.values(counterexample, "c"), [0, 2, 0]);
    let hints = hints_of(&fixture, counterexample, is_one);
    let record = crate::strong_fairness::settle_en1(&fixture, &outcome, is_one);
    assert_eq!(record.value, TerminalValue::Refuted);
    assert_eq!(
        record.counterexample.as_ref(),
        Some(counterexample.as_ref())
    );
    assert_eq!(record.hints, hints);
    assert_eq!(hints.len(), 1, "{hints:?}");
    let hint = &hints[0];
    assert_eq!(hint.suggested.kind(), FairnessKind::Strong);
    assert_eq!(hint.suggested.granularity(), FairnessGranularity::Each);
    assert_eq!(hint.suggested.operation().name(), "setTo");
    assert_eq!(hint.first_enabled_position, 0);
    let identity = hint.identity.as_ref().expect("an identity");
    assert!(counterexample
        .prefix
        .iter()
        .all(|step| step.transition != *identity));
}

/// The hints come in the model's operation order: by the declaring type's
/// effective identity, then operation name bytes. Over `Counter`'s `setTo`
/// variant with no constraint, `inc` precedes `setTo`.
#[trace("TC-534")]
#[test]
fn hints_come_in_the_models_operation_order() {
    let free = counter(
        document_with_ranged_parameter(),
        "",
        "always eventually holds(c.value = 3)",
    );
    let outcome = free.decide(is_three).expect("checked");
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("expected a violation, got {outcome:?}");
    };
    let names: Vec<_> = hints_of(&free, counterexample, is_three)
        .iter()
        .map(|hint| hint.suggested.operation().name().to_owned())
        .collect();
    assert_eq!(names, ["inc", "setTo"]);
}

pub(crate) fn is_one(value: i64) -> Option<bool> {
    Some(value == 1)
}

/// ADR-019 section 6's mutex over `Counter`'s ranged `setTo`, with
/// `fairness`.
pub(crate) fn mutex(fairness: &str) -> crate::liveness::Fixture {
    let body = "pre Free using v on M::Counter::setTo { self.value = 0 and n >= 1 and n <= 2 }\n\
        post Takes using v on M::Counter::setTo { self.value = n }\n\
        pre Held using v on M::Counter::inc { self.value != 0 }\n\
        post Frees using v on M::Counter::inc { self.value = 0 }\n";
    crate::liveness::fixture(
        document_with_ranged_parameter(),
        body,
        fairness,
        "always eventually holds(c.value = 1)",
        &[("c", 0)],
    )
}

/// The operation order is by declaring type, then name, whatever order a type
/// declares its operations in and whichever admitted type inherits them:
/// `Counter` inherits `inc` and `setTo` from a supertype that declares them
/// (in either order) and declares `zap` itself. Across supertype names, the
/// declaring identities fall both ways round: where `Counter` sorts below its
/// supertype the order is `zap, inc, setTo` (the own operation first although
/// its name sorts last), where above `inc, setTo, zap`,
/// with the supertype alone, before or after the subtype among the admitted
/// types, and with only the subtype admitted (inherited operations).
#[trace("TC-534", "FR-133-AC-4")]
#[test]
fn operations_are_ordered_by_declaring_type_then_name() {
    let mut seen = [false, false];
    for base in [
        "Base", "Root", "Alpha", "Zed", "Core", "Mid", "Top", "Under",
    ] {
        let mut subtype_first = None;
        for (reversed, admitted) in [(false, "c"), (true, "c"), (false, "bc"), (true, "cb")] {
            let (names, first) = operation_names(base, reversed, admitted);
            match subtype_first {
                None => subtype_first = Some(first),
                Some(known) => assert_eq!(known, first, "{base}: identities are fixed"),
            }
            let expected: [&str; 3] = if first {
                ["zap", "inc", "setTo"]
            } else {
                ["inc", "setTo", "zap"]
            };
            assert_eq!(
                names, expected,
                "{base} reversed {reversed} admitting {admitted}"
            );
        }
        seen[usize::from(subtype_first == Some(true))] = true;
    }
    assert_eq!(
        seen,
        [true, true],
        "both orders of the identities are exercised"
    );
}

/// The operation names of a `Counter` subtype whose supertype `base` declares
/// `inc` and `setTo` (reversed or not), with the types in `admitted` (`c`
/// the subtype, `b` the supertype) admitted in that order, and whether the
/// declaring identity of `zap` sorts before that of `inc`.
fn operation_names(base_name: &str, reversed: bool, admitted: &str) -> (Vec<String>, bool) {
    use qsl_eval::simulation::model::ModelSystem;
    let counter_id = "ix://test/counter/Counter".to_owned();
    let base_id = format!("ix://test/counter/{base_name}");
    let mut domain: serde_json::Value =
        serde_json::from_slice(&document_with_ranged_parameter()).expect("JSON");
    let types = domain["types"].as_array_mut().expect("types");
    let counter = types
        .iter_mut()
        .find(|node| node["identity"] == counter_id.as_str())
        .expect("Counter");
    let mut operations = counter["operations"].as_array().expect("ops").clone();
    let mut zap = operations[0].clone();
    if reversed {
        operations.reverse();
    }
    let mut base = counter.clone();
    base["operations"] = operations.into();
    let mut base: serde_json::Value = serde_json::from_str(&base.to_string().replace(
        "test/counter/Counter/",
        &format!("test/counter/{base_name}/"),
    ))
    .expect("JSON");
    base["identity"] = base_id.clone().into();
    zap["identity"] = "ix://test/counter/Counter/zap".into();
    zap["name"] = "zap".into();
    zap["frame"]["modifies"] = serde_json::json!([format!("{base_id}/value")]);
    counter["supertypes"] = serde_json::json!([base_id]);
    counter["fields"] = serde_json::json!([]);
    counter["operations"] = serde_json::json!([zap]);
    types.push(base);
    let members: Vec<&str> = admitted
        .chars()
        .map(|kind| {
            if kind == 'c' {
                counter_id.as_str()
            } else {
                base_id.as_str()
            }
        })
        .collect();
    domain["populations"][0]["members"] = serde_json::json!(members);
    let fixture = crate::liveness::fixture(
        domain.to_string().into_bytes(),
        "",
        "",
        "always eventually holds(c.value = 3)",
        &[("c", 0)],
    );
    let system = ModelSystem::new(
        crate::counter::subject(
            &fixture.package,
            &fixture.documents,
            fixture.initial.clone(),
            &fixture.keys,
            crate::counter::UNLIMITED,
        )
        .simulation,
    )
    .expect("admits");
    // The ids of the declaring types of `zap` and of the inherited `inc`
    // follow the order the operations were declared in; read them by name.
    let operations = system.operations();
    let id_of = |wanted: &str| {
        operations
            .iter()
            .find(|(_, name, _)| *name == wanted)
            .map(|(id, ..)| *id)
            .expect("the operation is listed")
    };
    let first = id_of("zap") < id_of("inc");
    (
        operations
            .iter()
            .map(|(_, name, _)| (*name).to_owned())
            .collect(),
        first,
    )
}
