// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-337-AC-11 to AC-14 (TC-1267 steps 5 to 7): the engine fixes a temporal
//! clause's captures at the activation, ends the run with
//! `UndefinedEvaluation` located at a capture it cannot evaluate, and refuses
//! a clause whose `holds` condition reads a capture. Every run reads the
//! clause through the production letters over FR-124-AC-1's `Counter`
//! subject.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{check_model, ModelCheckRefusal, ModelCheckRequest};
use qsl_eval::model_subject::{ModelInstance, ModelLetters};
use qsl_eval::simulation::model::{ExpansionLimits, ModelSystem};
use qsl_eval::temporal::{CaptureReading, UndefinedCause};
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::SourceIdentity;
use qsl_replay::{
    check_closure, compile_package, replay_model_trace, CertificateRequest, CertificateRule,
    CounterexampleKind, DependencyInput, DisagreementCause, ModelCheckItem, ModelCheckLimits,
    ModelCheckOutcome, ModelTraceSettlement, ModelTraceSubject, ModelTraceValue, ReplayLimits,
    SmtEncodingLimits, TemporalCertificateCheck, TemporalCounterexample, UndefinedEvaluation,
    UndefinedLocus, WireNodeId,
};
use quire_exact::{Cancel, Meter};
use serde_json::json;

use crate::counter::{CONTRACT, UNLIMITED};
use crate::model_settle::{envelope_at, request};
use crate::model_world::{self, Clause, World, BOUNDED, INFINITE};

fn world(profile: &str, activation: &str, formula: &str) -> World {
    world_from(profile, activation, formula, 0)
}

fn world_from(profile: &str, activation: &str, formula: &str, start: i64) -> World {
    World::new(
        model_world::counter(),
        CONTRACT,
        profile,
        &[Clause {
            activation,
            formula,
        }],
        &[("c", json!({"value": {"integer": start.to_string()}}))],
    )
}

fn each(guard: &str) -> String {
    format!("on each (q: M::Counter) when ({guard})")
}

/// The run of the world's one clause through its production letters.
fn run(world: &World, limits: ModelCheckLimits) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
    let system =
        ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation).expect("the subject admits");
    let node = &world.package.graph().temporal_clauses()[0];
    let letters = system.clause_letters(node).expect("the clause has letters");
    check_model(
        ModelCheckRequest {
            subject: world.subject(&["c"], UNLIMITED),
            item: ModelCheckItem::Clause {
                clause: node.clause(),
                letters: &letters,
            },
            max_depth: None,
            limits,
            reductions: Default::default(),
        },
        &Cancel::new(),
    )
}

fn prefix(world: &World, outcome: &ModelCheckOutcome) -> Vec<i64> {
    let ModelCheckOutcome::Violated { counterexample, .. } = outcome else {
        panic!("expected a violation, got {outcome:?}");
    };
    world
        .positions(&["c"], counterexample)
        .iter()
        .map(|state| world.domain.integer(state, "c", "value"))
        .collect()
}

fn holds(outcome: &ModelCheckOutcome) {
    assert!(
        matches!(outcome, ModelCheckOutcome::Holds { .. }),
        "expected Holds, got {outcome:?}"
    );
}

const UNREAD: &str = "capture n: Int[0, 3] = c.value; ";

/// FR-337-AC-11: a capture no `holds` condition reads changes no verdict.
#[trace("TC-1267", "FR-337-AC-11")]
#[test]
fn an_unread_capture_returns_the_verdict_of_the_clause_without_it() {
    let formula = "eventually[0,1] holds(c.value = 3)";
    for (guard, violated) in [("c.value = 1", true), ("c.value >= 2", false)] {
        for captures in [UNREAD, ""] {
            let world = world(BOUNDED, &each(guard), &format!("{captures}{formula}"));
            let outcome = run(&world, ModelCheckLimits::default()).expect("checked");
            if violated {
                assert_eq!(prefix(&world, &outcome), [0, 1, 2], "{guard} {captures}");
            } else {
                holds(&outcome);
            }
        }
    }
}

/// FR-337-AC-11: the capture `z` has no value at the activation at value 2,
/// so the run ends `UndefinedEvaluation` located at `z`, at position 2.
#[trace("TC-1267", "FR-337-AC-11")]
#[test]
fn an_undefined_capture_is_an_undefined_evaluation_located_at_it() {
    let formula = "capture z: Boolean = (6 mod (2 - c.value) = 0); always holds(true)";
    let world = world(INFINITE, &each("true"), formula);
    let outcome = run(&world, ModelCheckLimits::default()).expect("checked");
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("expected a violation, got {outcome:?}");
    };
    let CounterexampleKind::UndefinedEvaluation(evaluation) = &counterexample.kind else {
        panic!("expected an undefined evaluation: {counterexample:?}");
    };
    assert_eq!(evaluation.site.position, 2);
    assert_eq!(
        evaluation.site.locus,
        UndefinedLocus::Capture("z".to_owned())
    );
    assert_eq!(
        evaluation.cause,
        UndefinedCause::Kernel(quire_exact::Undefined::DivisionByZero)
    );
    assert_eq!(prefix(&world, &outcome), [0, 1, 2]);

    // A guard no position meets (S3 refuses the spec's `c.value = 7`: the field is `Int[0, 3]`) never activates the clause: no capture is read.
    let world = self::world(INFINITE, &each("false"), formula);
    holds(&run(&world, ModelCheckLimits::default()).expect("checked"));
}

/// FR-337-AC-11: an `on origin` clause fixes its captures at position 0
/// alone, so a capture undefined only at a later value is never read.
#[trace("TC-1267", "FR-337-AC-11")]
#[test]
fn an_origin_clause_fixes_its_captures_at_position_zero() {
    let world = world(
        INFINITE,
        "on origin",
        "capture z: Boolean = (6 mod (2 - c.value) = 0); always holds(true)",
    );
    holds(&run(&world, ModelCheckLimits::default()).expect("checked"));
}

/// FR-337-AC-12: a `holds` condition that reads a capture is refused before
/// any state is explored, naming the clause and the first capture read.
#[trace("TC-1267", "FR-337-AC-12")]
#[test]
fn a_holds_read_of_a_capture_is_refused_before_exploration() {
    // No state may be reached: a run that explored would stop at the limit.
    let mut limits = ModelCheckLimits::default();
    limits.limits.max_states = 0;
    for (profile, activation, formula) in [
        (
            BOUNDED,
            each("c.value = 1"),
            "capture n: Int[0, 3] = c.value; \
             eventually[0,1] holds(n < 3 and c.value = n + 1)",
        ),
        (
            INFINITE,
            each("true"),
            "capture n: Int[0, 3] = c.value; always holds(c.value >= n)",
        ),
        (
            BOUNDED,
            "on origin".to_owned(),
            "capture n: Int[0, 3] = c.value; eventually[0,1] holds(c.value >= n)",
        ),
        (
            INFINITE,
            each("true"),
            "capture n: Int[0, 3] = c.value; capture m: Int[0, 3] = q.value; \
             always holds(c.value >= m)",
        ),
    ] {
        let world = world(profile, &activation, formula);
        let refusal = run(&world, limits).expect_err("a capture is read");
        let ModelCheckRefusal::CaptureRead { clause, capture } = refusal else {
            panic!("expected CaptureRead, got {refusal:?} for {formula}");
        };
        assert_eq!(clause, "C0");
        let expected = if formula.contains("holds(c.value >= m)") {
            "m"
        } else {
            "n"
        };
        assert_eq!(capture, expected, "{formula}");
    }

    // The refusal does not mask the verdict of a clause that reads none.
    let unread = world(
        BOUNDED,
        &each("c.value = 1"),
        &format!("{UNREAD}eventually[0,1] holds(c.value = 3)"),
    );
    assert!(run(&unread, ModelCheckLimits::default()).is_ok());
}

/// FR-337-AC-14: the activation's footprint holds `Counter::value` with the
/// `over` receiver, for a capture reading through `c` and through `q`, and
/// holds nothing without the capture; a partial-order run joins it (the
/// verdict is unchanged).
#[trace("TC-1267", "FR-337-AC-14")]
#[test]
fn a_capture_initializer_joins_the_activation_footprint() {
    use qsl_semantics::check::footprint::{FootprintObject, SymbolicLocation};

    let reads = |captures: &str| {
        let world = world(
            INFINITE,
            &each("true"),
            &format!("{captures}always holds(true)"),
        );
        let system = ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation)
            .expect("the subject admits");
        let node = &world.package.graph().temporal_clauses()[0];
        let letters = system.clause_letters(node).expect("the clause has letters");
        let instance = ModelInstance {
            population: world.domain.key(world.domain.population),
            key: "c".to_owned(),
        };
        let start = system.initial_states()[0].clone();
        if !captures.is_empty() {
            let reading = letters
                .captures(&instance, &start, &mut Meter::new(UNLIMITED))
                .expect("the captures read");
            assert_eq!(reading, CaptureReading::Fixed);
        }
        let reads = letters.activation_reads().expect("a footprint");
        (reads, world.domain.key("Counter/value"))
    };
    let (none, _) = reads("");
    assert!(none.is_empty());
    for capture in [
        "capture n: Int[0, 3] = q.value; ",
        "capture n: Int[0, 3] = c.value; ",
    ] {
        let (found, value) = reads(capture);
        assert_eq!(
            found.into_iter().collect::<Vec<_>>(),
            [SymbolicLocation::Field {
                object: FootprintObject::Receiver,
                field: value,
            }],
            "{capture}"
        );
    }
    // A partial-order run reads the capture's footprint and settles as the
    // unreduced run does.
    let world = world(
        INFINITE,
        &each("true"),
        "capture n: Int[0, 3] = q.value; always holds(true)",
    );
    let system =
        ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation).expect("the subject admits");
    let node = &world.package.graph().temporal_clauses()[0];
    let letters = system.clause_letters(node).expect("the clause has letters");
    let outcome = check_model(
        ModelCheckRequest {
            subject: world.subject(&["c"], UNLIMITED),
            item: ModelCheckItem::Clause {
                clause: node.clause(),
                letters: &letters,
            },
            max_depth: None,
            limits: ModelCheckLimits::default(),
            reductions: qsl_analyze::model_check::Reductions {
                partial_order: true,
                ..Default::default()
            },
        },
        &Cancel::new(),
    )
    .expect("checked");
    assert!(
        matches!(outcome, ModelCheckOutcome::HoldsReduced { .. }),
        "{outcome:?}"
    );
}

/// FR-337-AC-12: an unbounded root is `RequiresBound`, not `CaptureRead`.
#[trace("TC-1267", "FR-337-AC-12")]
#[test]
fn an_unbounded_root_is_refused_before_a_capture_read() {
    let text = format!(
        "{}always holds(c.value >= n)",
        "capture n: Int[0, 3] = c.value; "
    );
    let world = World::new(
        model_world::Domain {
            document: crate::counter::document(true),
            ..model_world::counter()
        },
        CONTRACT,
        INFINITE,
        &[Clause {
            activation: &each("true"),
            formula: &text,
        }],
        &[("c", json!({"value": {"integer": "0"}}))],
    );
    let refusal = run(&world, ModelCheckLimits::default()).expect_err("refused");
    assert!(
        matches!(refusal, ModelCheckRefusal::RequiresBound(_)),
        "{refusal:?}"
    );
}

/// FR-337-AC-11 through the lasso reader: an infinite-trace `eventually`
/// fixes `z` at each position its guard admits and finds it undefined at
/// position 2; an `on origin` clause, whose capture is defined at position 0,
/// and a guard that never holds read none there.
#[trace("TC-1267", "FR-337-AC-11")]
#[test]
fn the_liveness_reader_fixes_captures_at_the_activation() {
    let formula = "capture z: Boolean = (6 mod (2 - c.value) = 0); eventually holds(c.value = 3)";
    for guard in ["true", "c.value = 2"] {
        let world = world(INFINITE, &each(guard), formula);
        let outcome = run(&world, ModelCheckLimits::default()).expect("checked");
        let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
            panic!("{guard}: expected a violation, got {outcome:?}");
        };
        let CounterexampleKind::UndefinedEvaluation(evaluation) = &counterexample.kind else {
            panic!("{guard}: expected an undefined evaluation: {counterexample:?}");
        };
        assert_eq!(evaluation.site.position, 2, "{guard}");
        assert_eq!(
            evaluation.site.locus,
            UndefinedLocus::Capture("z".to_owned())
        );
        assert_eq!(prefix(&world, &outcome), [0, 1, 2], "{guard}");
    }
    for activation in ["on origin".to_owned(), each("false")] {
        let world = world(INFINITE, &activation, formula);
        let outcome = run(&world, ModelCheckLimits::default()).expect("checked");
        assert!(
            matches!(
                outcome,
                ModelCheckOutcome::Holds {
                    certificate: qsl_replay::ProofCertificate::Component(_),
                    ..
                }
            ),
            "{activation}: {outcome:?}"
        );
    }
}

/// FR-337-AC-13: the FR-338 checker reads the captures at each state it
/// examines. A clause whose captures are defined at every reachable state
/// proves `Holds`, and its certificate is accepted; the certificate of the
/// same clause over a subject that stays at value 3, checked over a subject
/// that starts at value 2, where `z` is undefined, is rejected `BadState`.
#[trace("TC-1267", "FR-337-AC-13")]
#[test]
fn the_closure_checker_reads_the_captures_and_rejects_an_undefined_one() {
    let formula = "capture z: Boolean = (6 mod (2 - c.value) = 0); always holds(true)";
    let check = |world: &World, certificate: &qsl_replay::ClosureCertificate| {
        let system = ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation)
            .expect("the subject admits");
        let node = &world.package.graph().temporal_clauses()[0];
        let letters = system.clause_letters(node).expect("the clause has letters");
        check_closure(
            &CertificateRequest {
                subject: world.subject(&["c"], UNLIMITED),
                item: ModelCheckItem::Clause {
                    clause: node.clause(),
                    letters: &letters,
                },
                smt_encoding: SmtEncodingLimits::default(),
            },
            certificate,
            &ModelCheckLimits::default(),
            &Cancel::new(),
        )
        .expect("the request is checked")
    };
    let at_three = world_from(INFINITE, "on each (q: M::Counter)", formula, 3);
    let outcome = run(&at_three, ModelCheckLimits::default()).expect("checked");
    let ModelCheckOutcome::Holds { certificate, .. } = &outcome else {
        panic!("expected Holds, got {outcome:?}");
    };
    let qsl_replay::ProofCertificate::Closure(certificate) = certificate else {
        panic!("a safety clause holds with a closure: {certificate:?}");
    };
    assert!(!certificate.states.is_empty());
    assert_eq!(
        check(&at_three, certificate),
        TemporalCertificateCheck::Accepted
    );

    let at_two = world_from(INFINITE, "on each (q: M::Counter)", formula, 2);
    let TemporalCertificateCheck::Rejected(rejection) = check(&at_two, certificate) else {
        panic!("a state with an undefined capture is a bad state");
    };
    assert_eq!(rejection.rule, CertificateRule::BadState);
}

/// FR-337-AC-13: FR-128 replays AC-11's undefined-capture counterexample
/// through the production letters. The recompiled clause reads the capture
/// `z` at position 2, where `6 mod (2 - c.value)` divides by zero, and the
/// replay reproduces `UndefinedEvaluation` at position 2 located at `z`; the
/// same payload located at the activation or at position 1 settles
/// `inconclusive`.
#[trace("TC-1267", "FR-337-AC-13")]
#[test]
fn replay_reproduces_an_undefined_capture_located_at_it() {
    let formula = "capture z: Boolean = (6 mod (2 - c.value) = 0); always holds(true)";
    let world = world(INFINITE, &each("true"), formula);
    let outcome = run(&world, ModelCheckLimits::default()).expect("checked");
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("expected a violation, got {outcome:?}");
    };
    let CounterexampleKind::UndefinedEvaluation(claimed) = &counterexample.kind else {
        panic!("expected an undefined evaluation: {counterexample:?}");
    };
    let claimed = claimed.clone();

    let document = world.domain.document.as_slice();
    let package_id = compile_package(
        SourceIdentity::new("agent-ix", "counter", "git", "1"),
        "counter.native",
        &world.unit,
        [document],
        &DependencyInput::default(),
        qsl_replay::StageLimits::default(),
        ReplayLimits::default(),
    )
    .expect("the unit compiles")
    .package_id();
    let node = &world.package.graph().temporal_clauses()[0];
    let identity = WireNodeId::from_digest(*node.identity().as_bytes());
    let occurrence = OccurrenceKey::new(identity, node.claim().clone());
    let system =
        ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation).expect("the subject admits");
    let letters = system.clause_letters(node).expect("the clause has letters");
    let subject = ModelTraceSubject {
        exploration: qsl_eval::simulation::Limits::default(),
        initial: world.initial.clone(),
        universes: crate::counter::universe(&["c"]),
        expansion: ExpansionLimits {
            max_candidates: 1_000_000,
        },
    };
    let replay = |payload: TemporalCounterexample| {
        replay_model_trace(
            request(document, world.snapshot_bytes(), &world.unit, package_id),
            &subject,
            &envelope_at(
                &world.unit,
                package_id,
                payload,
                identity,
                occurrence.clone(),
            ),
            &letters,
            ReplayLimits::default(),
        )
        .expect("replays")
        .settlement
    };

    let reproduced = replay(counterexample.as_ref().clone());
    assert_eq!(
        reproduced,
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Undefined(claimed.clone()),
            trace_position: counterexample.trace_position(),
        }
    );
    for edit in [
        (|evaluation: &mut UndefinedEvaluation| evaluation.site.locus = UndefinedLocus::Activation)
            as fn(&mut UndefinedEvaluation),
        |evaluation| evaluation.site.locus = UndefinedLocus::Capture("other".to_owned()),
        |evaluation| evaluation.site.position = 1,
    ] {
        let mut payload = counterexample.as_ref().clone();
        let CounterexampleKind::UndefinedEvaluation(evaluation) = &mut payload.kind else {
            panic!("an undefined evaluation");
        };
        edit(evaluation);
        assert!(
            matches!(
                replay(payload),
                ModelTraceSettlement::Inconclusive(DisagreementCause::Verdicts { .. })
            ),
            "a payload that misplaces the undefined capture is a `ReplayParity` disagreement"
        );
    }
}
