// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-123: activation of temporal obligations and immutable captures.
//!
//! FR-044. Expected instance identities, capture values, dispositions and
//! refusal causes come from the authored source and from the independently
//! constructed trigger observations, never from the evaluator's own output.
//!
//! The primary declaration is the timed-refund shape: a failed order triggers an
//! obligation that captures the paid amount and the payment reference.

#[path = "support/temporal/mod.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_spec_language::checking::composed::{proofs, TypeDisposition, TypeLimits};
use quire_spec_language::temporal::{
    self, Activation, Assessment, CaptureInput, Closure, Completeness, Dimension, Error, Eviction,
    Evidence, Execution, Incomplete, Obligation, Refusal, Trace, Trigger, Truth,
};
use setup::{
    admitted, assessment, holds_leaf, origin, position, trace, Declaration, Inputs, Unit,
    EVENT_POSITION,
};

/// The authored activation anchor every control binds to; the shared trace
/// asserts the same anchor.
const ANCHOR: &str = "origin";

/// The authored trigger binder and its activation guard.
const TRIGGER: &str = "on each (failed: M::Node) when (failed.n >= 1)";

/// Two authored captures in source order, then the obligation's formula.
const REFUND: &str = "capture paid: M::Amount = failed.amount; \
                      capture reference: M::Label = failed.label; \
                      eventually[0,1] holds(view.ready)";

/// Three authored captures reading three distinct observation fields.
const THREE_CAPTURES: &str = "capture paid: M::Amount = failed.amount; \
                              capture reference: M::Label = failed.label; \
                              capture tally: M::Tally = failed.tally; \
                              eventually[0,1] holds(view.ready)";

/// The timed-refund declaration over an authored body.
fn refund(body: &str) -> Declaration<'_> {
    Declaration {
        name: "Refund",
        profile: EVENT_POSITION,
        clock: "orders",
        body,
        activation: TRIGGER,
    }
}

/// One established capture input read at the authored activation anchor.
fn established(value: &str) -> CaptureInput {
    CaptureInput::Value {
        anchor: ANCHOR.into(),
        value: value.into(),
    }
}

/// One admitted delivery of a semantic trigger whose guard holds and whose
/// captures are all established.
fn delivery(identity: &str, receipt: &str, payload: &str, captures: &[&str]) -> Trigger {
    Trigger {
        identity: identity.into(),
        receipt: receipt.into(),
        anchor: ANCHOR.into(),
        payload: payload.into(),
        guard: Some(true),
        captures: captures.iter().copied().map(established).collect(),
    }
}

/// The shared trace carrying exactly the supplied deliveries.
fn delivered(triggers: Vec<Trigger>) -> Trace {
    let mut supplied = trace(EVENT_POSITION, "orders");
    supplied.triggers = triggers;
    supplied
}

/// Every obligation of a report that did not stop for a whole-declaration
/// reason.
fn obligations(report: &temporal::Report) -> &[Obligation] {
    report
        .result()
        .unwrap_or_else(|error| panic!("evaluation stopped: {error:?}"))
}

/// One obligation's assessment.
fn assessed(obligation: &Obligation) -> &Assessment {
    match obligation {
        Obligation::Assessed(assessment) => assessment,
        Obligation::Unactivated { error, .. } => panic!("activation failed: {error:?}"),
    }
}

/// The activation failure of one obligation.
fn unactivated(obligation: &Obligation) -> &Error {
    match obligation {
        Obligation::Unactivated { error, .. } => error,
        Obligation::Assessed(assessment) => {
            panic!(
                "expected an activation failure, got {:?}",
                assessment.activation
            )
        }
    }
}

/// Whether an activation failure is incomplete or refused, and the dimension it
/// names, separately from the subject that locates it.
fn classify(error: &Error) -> (&'static str, Dimension) {
    match error {
        Error::Incomplete(Incomplete { dimension, .. }) => ("incomplete", *dimension),
        Error::Refused(Refusal::Capture { dimension, .. }) => ("refused", *dimension),
        other => panic!("unexpected activation failure {other:?}"),
    }
}

/// The subject an activation failure locates itself by.
fn located(error: &Error) -> temporal::Subject {
    match error {
        Error::Incomplete(Incomplete { subject, .. })
        | Error::Refused(Refusal::Capture { subject, .. }) => *subject,
        other => panic!("unexpected activation failure {other:?}"),
    }
}

/// The retained capture values of one assessment, in retained order.
fn values(assessment: &Assessment) -> Vec<&str> {
    assessment
        .captures
        .iter()
        .map(|capture| capture.value.as_str())
        .collect()
}

/// FR-044-AC-1: two distinct semantic triggers carrying equal payloads and an
/// equal captured amount create two distinct instances, and neither instance's
/// retained captures appear in the other's assessment.
#[test]
#[trace("TC-123", "FR-044-AC-1")]
fn two_distinct_triggers_with_equal_payloads_create_two_instances() {
    admitted(&refund(REFUND), |package, at| {
        let supplied = delivered(vec![
            delivery("order:1", "receipt:1", "failed", &["5", "payment:a"]),
            delivery("order:2", "receipt:2", "failed", &["5", "payment:b"]),
        ]);
        let report = temporal::evaluate(package, at, &supplied, temporal::Limits::default());
        let [first, second] = obligations(&report) else {
            panic!("two distinct semantic triggers create two instances")
        };
        let (first, second) = (assessed(first), assessed(second));

        assert_eq!(first.instance.as_deref(), Some("order:1"));
        assert_eq!(second.instance.as_deref(), Some("order:2"));
        assert_ne!(
            first.instance, second.instance,
            "an equal captured amount never merges two semantic triggers",
        );
        assert_eq!(values(first), ["5", "payment:a"]);
        assert_eq!(values(second), ["5", "payment:b"]);
        assert!(
            !values(first).contains(&"payment:b") && !values(second).contains(&"payment:a"),
            "one instance's captures are not reachable from the other's assessment",
        );
        assert_ne!(
            first.subject.instance, second.subject.instance,
            "each instance locates its own stops",
        );
        for assessed in [first, second] {
            assert_eq!(assessed.activation, Activation::Active);
            assert_eq!(assessed.capture_evaluations, 1);
        }
    });
}

/// FR-044-AC-2: a repeated delivery of one semantic trigger under a second
/// receipt identity creates no second instance, retains that receipt's
/// provenance beside the first, and leaves the capture values unchanged.
#[test]
#[trace("TC-123", "FR-044-AC-2")]
fn a_repeated_delivery_adds_provenance_and_no_second_instance() {
    admitted(&refund(REFUND), |package, at| {
        let supplied = delivered(vec![
            delivery("order:1", "receipt:1", "failed", &["5", "payment:a"]),
            delivery("order:1", "receipt:2", "failed", &["9", "payment:z"]),
        ]);
        let report = temporal::evaluate(package, at, &supplied, temporal::Limits::default());
        let assessed = assessment(&report);
        assert_eq!(assessed.instance.as_deref(), Some("order:1"));
        assert_eq!(
            assessed.receipts,
            ["receipt:1", "receipt:2"],
            "the repeat's provenance is retained and separately inspectable",
        );
        assert_eq!(
            values(assessed),
            ["5", "payment:a"],
            "a repeated delivery never replaces a retained capture",
        );
        assert_eq!(assessed.capture_evaluations, 1);
    });
}

/// FR-044-AC-2: two deliveries asserting one semantic trigger identity with
/// conflicting payloads refuse with a typed contradiction naming that identity,
/// and produce no aliased instance.
#[test]
#[trace("TC-123", "FR-044-AC-2")]
fn conflicting_payloads_under_one_trigger_identity_refuse() {
    admitted(&refund(REFUND), |package, at| {
        let supplied = delivered(vec![
            delivery("order:1", "receipt:1", "failed:5", &["5", "payment:a"]),
            delivery("order:1", "receipt:2", "failed:9", &["5", "payment:a"]),
        ]);
        let report = temporal::evaluate(package, at, &supplied, temporal::Limits::default());
        let error = report
            .result()
            .expect_err("conflicting payloads under one identity refuse");
        let temporal::Error::Refused(Refusal::Contradiction { identity, .. }) = error else {
            panic!("expected a typed contradiction, got {error:?}")
        };
        assert_eq!(
            identity, "order:1",
            "the refusal names the conflicting semantic trigger",
        );
    });
}

/// FR-044-AC-3: mutating the observation the captures were read from, and the
/// valuation environment they were read beside, leaves the retained capture
/// record identical to the activation-time record.
#[test]
#[trace("TC-123", "FR-044-AC-3")]
fn a_later_mutation_of_the_observation_leaves_the_retained_captures_identical() {
    admitted(&refund(REFUND), |package, at| {
        let leaf = holds_leaf(package, at);
        let mut supplied = delivered(vec![delivery(
            "order:1",
            "receipt:1",
            "failed",
            &["5", "payment:a"],
        )]);
        supplied.positions = vec![position(0, &[(leaf, true)])];
        let report = temporal::evaluate(package, at, &supplied, temporal::Limits::default());
        let retained = assessment(&report).captures.clone();
        assert_eq!(values(assessment(&report)), ["5", "payment:a"]);

        // The source observation, its reference and the valuation environment
        // all change after activation.
        supplied.triggers[0].captures = vec![established("11"), established("payment:mutated")];
        supplied.positions = vec![position(0, &[(leaf, false)])];

        assert_eq!(
            assessment(&report).captures,
            retained,
            "the retained record is not reachable through the mutated observation",
        );
        assert_eq!(values(assessment(&report)), ["5", "payment:a"]);

        // The mutation is real: a fresh activation over the mutated observation
        // establishes the changed values, so the control above is not vacuous.
        let mutated = temporal::evaluate(package, at, &supplied, temporal::Limits::default());
        assert_eq!(values(assessment(&mutated)), ["11", "payment:mutated"]);
        assert_ne!(assessment(&mutated).captures, retained);
    });
}

/// FR-044-AC-4: a missing, null, wrongly typed, stale or anchor-mismatched
/// capture input reports incomplete or refused activation naming the capture
/// dimension, and evaluates no position for that instance.
#[test]
#[trace("TC-123", "FR-044-AC-4")]
fn an_unestablished_capture_input_reports_incomplete_or_refused_activation() {
    let expected: [(&str, CaptureInput, (&str, Dimension)); 5] = [
        (
            "missing",
            CaptureInput::Missing,
            ("incomplete", Dimension::Capture),
        ),
        (
            "null",
            CaptureInput::Null,
            ("incomplete", Dimension::Capture),
        ),
        (
            "wrong type",
            CaptureInput::WrongType,
            ("refused", Dimension::Capture),
        ),
        (
            "stale",
            CaptureInput::Stale,
            ("refused", Dimension::Capture),
        ),
        (
            "anchor mismatch",
            CaptureInput::Value {
                anchor: "later".into(),
                value: "5".into(),
            },
            ("refused", Dimension::Anchor),
        ),
    ];

    admitted(&refund(REFUND), |package, at| {
        let leaf = holds_leaf(package, at);
        for (name, input, reported) in expected.clone() {
            let mut supplied = delivered(vec![delivery(
                "order:1",
                "receipt:1",
                "failed",
                &["5", "payment:a"],
            )]);
            supplied.triggers[0].captures[1] = input;
            supplied.positions = vec![position(0, &[(leaf, true)])];
            let report = temporal::evaluate(package, at, &supplied, temporal::Limits::default());
            let [obligation] = obligations(&report) else {
                panic!("{name}: one obligation")
            };
            let error = unactivated(obligation);
            assert_eq!(classify(error), reported, "{name}: {error:?}");
            assert_eq!(
                located(error),
                temporal::Subject {
                    declaration: at,
                    instance: 0,
                    node: None,
                    position: None,
                    // The unestablished input is the second declared capture,
                    // and the failure names it rather than only its dimension.
                    capture: Some(1),
                },
                "{name}: the failure locates its own instance and capture",
            );
            assert_eq!(
                report.usage().positions,
                0,
                "{name}: no position is evaluated for an unactivated instance",
            );
        }

        // An unrelated healthy instance in the same evaluation keeps its own
        // activation and its own captures.
        let mut supplied = delivered(vec![
            delivery("order:1", "receipt:1", "failed", &["5", "payment:a"]),
            delivery("order:2", "receipt:2", "failed", &["7", "payment:b"]),
        ]);
        supplied.triggers[0].captures[1] = CaptureInput::Missing;
        supplied.positions = vec![position(0, &[(leaf, true)])];
        let report = temporal::evaluate(package, at, &supplied, temporal::Limits::default());
        let [failed, healthy] = obligations(&report) else {
            panic!("one failed and one healthy instance")
        };
        assert!(matches!(
            unactivated(failed),
            Error::Incomplete(Incomplete {
                dimension: Dimension::Capture,
                ..
            })
        ));
        let healthy = assessed(healthy);
        assert_eq!(healthy.instance.as_deref(), Some("order:2"));
        assert_eq!(healthy.activation, Activation::Active);
        assert_eq!(values(healthy), ["7", "payment:b"]);
        assert_eq!(healthy.truth, Some(Truth::True));
        assert!(
            report.usage().positions > 0,
            "the healthy sibling does evaluate positions, so the zero usage above \
             is the unactivated instance's own",
        );
    });
}

/// FR-044-AC-5: the four trigger-scope shapes report `inactive`, `unknown` for
/// an open scope, and `unknown` carrying the distinct incomplete and refused
/// execution dispositions. None carries temporal truth and none evaluates a
/// position.
#[test]
#[trace("TC-123", "FR-044-AC-5")]
fn four_trigger_scope_shapes_keep_four_distinct_dispositions() {
    admitted(&refund(REFUND), |package, at| {
        let leaf = holds_leaf(package, at);
        let shape = |name: &'static str, prepare: fn(&mut Trace)| {
            let mut supplied = delivered(Vec::new());
            supplied.positions = vec![position(0, &[(leaf, true)])];
            prepare(&mut supplied);
            let report = temporal::evaluate(package, at, &supplied, temporal::Limits::default());
            let assessed = assessment(&report).clone();
            assert_eq!(assessed.truth, None, "{name}: activation is not a truth");
            assert_eq!(assessed.basis, None, "{name}");
            assert!(assessed.instance.is_none(), "{name}");
            assert!(assessed.captures.is_empty(), "{name}");
            assert!(assessed.support.is_empty(), "{name}");
            assert_eq!(
                report.usage().positions,
                0,
                "{name}: no position is evaluated",
            );
            assessed.activation
        };

        let closed = shape("closed and complete", |supplied| {
            supplied.trigger_scope = Closure::Closed;
            supplied.completeness = Completeness::Complete;
        });
        assert_eq!(
            closed,
            Activation::Inactive,
            "only a closed, complete scope with no instance-creating trigger is inactive",
        );

        let open = shape("open", |supplied| supplied.trigger_scope = Closure::Open);
        assert_eq!(
            open,
            Activation::Unknown {
                completeness: Completeness::Complete,
                execution: Execution::Completed,
            },
            "an open trigger scope is unknown, never inactive",
        );

        let missing = shape("missing trigger evidence", |supplied| {
            supplied.trigger_evidence = Evidence::Missing;
        });
        assert_eq!(
            missing,
            Activation::Unknown {
                completeness: Completeness::Incomplete,
                execution: Execution::Completed,
            },
        );

        let refused = shape("refused trigger evidence", |supplied| {
            supplied.trigger_evidence = Evidence::Refused;
        });
        assert_eq!(
            refused,
            Activation::Unknown {
                completeness: Completeness::Complete,
                execution: Execution::Failed,
            },
        );

        // The same positions under one admitted delivery are evaluated, so the
        // four zero-usage claims above are the dispositions' own.
        let mut active = delivered(vec![delivery(
            "order:1",
            "receipt:1",
            "failed",
            &["5", "payment:a"],
        )]);
        active.positions = vec![position(0, &[(leaf, true)])];
        let report = temporal::evaluate(package, at, &active, temporal::Limits::default());
        assert_eq!(assessment(&report).activation, Activation::Active);
        assert!(report.usage().positions > 0);

        let all = [closed, open, missing, refused];
        for (index, left) in all.iter().enumerate() {
            for right in &all[index + 1..] {
                assert_ne!(left, right, "the four dispositions stay distinct");
            }
        }
    });
}

/// FR-044-AC-6: three captures reading three distinct observation fields are
/// retained in authored source order at the activation anchor, and the
/// per-instance evaluation counter stays one across an incremental
/// re-evaluation and an admitted restoration.
#[test]
#[trace("TC-123", "FR-044-AC-6")]
fn captures_are_retained_in_authored_order_and_evaluated_once() {
    admitted(&refund(THREE_CAPTURES), |package, at| {
        let leaf = holds_leaf(package, at);
        let mut supplied = delivered(vec![delivery(
            "order:1",
            "receipt:1",
            "failed",
            &["5", "payment:a", "2"],
        )]);
        supplied.positions = vec![position(0, &[(leaf, true)])];
        let report = temporal::evaluate(package, at, &supplied, temporal::Limits::default());
        let assessed = assessment(&report);
        assert_eq!(values(assessed), ["5", "payment:a", "2"]);
        assert_eq!(
            assessed
                .captures
                .iter()
                .map(|capture| capture.declared)
                .collect::<Vec<_>>(),
            [0, 1, 2],
            "captures are retained in authored source order",
        );
        for capture in &assessed.captures {
            assert_eq!(
                capture.anchor, ANCHOR,
                "every capture is established at the authored activation anchor",
            );
        }
        assert_eq!(assessed.capture_evaluations, 1);

        // Incremental re-evaluation at a later watermark over a further
        // position, then an admitted restoration from the same activation.
        let mut incremental = supplied.clone();
        incremental.watermark = 3;
        incremental.positions.push(position(1, &[(leaf, true)]));
        let later = temporal::evaluate(package, at, &incremental, temporal::Limits::default());
        assert_eq!(assessment(&later).capture_evaluations, 1);
        assert_eq!(values(assessment(&later)), ["5", "payment:a", "2"]);

        let restored = temporal::evaluate(package, at, &supplied, temporal::Limits::default());
        assert_eq!(assessment(&restored).capture_evaluations, 1);
        assert_eq!(assessment(&restored).captures, assessed.captures);
    });
}

/// FR-044-AC-6: a capture initializer reading a later capture's name refuses,
/// while the same source reading an earlier capture's name types.
#[test]
#[trace("TC-123", "FR-044-AC-6")]
fn a_capture_initializer_reading_a_later_capture_refuses() {
    fn declaration(captures: &str) -> String {
        format!(
            "temporal Refund using T over (view: M::Plain) clock \"orders\" {TRIGGER} \
             {{ {captures} eventually[0,1] holds(view.ready) }}"
        )
    }

    let backward = declaration(
        "capture reference: M::Label = failed.label; capture echoed: M::Label = reference;",
    );
    let forward = declaration(
        "capture echoed: M::Label = reference; capture reference: M::Label = failed.label;",
    );
    for (name, body, typed) in [("backward", backward, true), ("forward", forward, false)] {
        let inputs = Inputs::new(&[Unit {
            name: "temporal-activation",
            body: &body,
            declarations: &["Refund"],
        }]);
        inputs.with_proofs(
            TypeLimits::default(),
            proofs::ProofLimits::default(),
            |proved, _| {
                let [id] = proved.types().binding().namespace().lookup("Refund") else {
                    panic!("one authored temporal declaration")
                };
                let disposition = proved.types().disposition(*id);
                if typed {
                    assert_eq!(
                        disposition,
                        Some(TypeDisposition::Typed),
                        "{name}: {:?}",
                        proved.types().declaration(*id).map(|entry| entry.causes()),
                    );
                } else {
                    assert_ne!(
                        disposition,
                        Some(TypeDisposition::Typed),
                        "{name}: a forward capture read is refused",
                    );
                }
            },
        );
    }
}

/// FR-044-AC-7: a guard evaluating false creates no instance and is not a
/// refusal, so the scope's disposition follows from whatever remains; a guard
/// that cannot be established is incomplete and is never read as false.
#[test]
#[trace("TC-123", "FR-044-AC-7")]
fn a_false_guard_creates_no_instance_and_is_not_a_refusal() {
    admitted(&refund(REFUND), |package, at| {
        let leaf = holds_leaf(package, at);
        let unguarded = |guard: Option<bool>, scope: Closure| {
            let mut supplied = delivered(vec![
                delivery("order:1", "receipt:1", "failed", &["5", "payment:a"]),
                delivery("order:2", "receipt:2", "failed", &["7", "payment:b"]),
            ]);
            for trigger in &mut supplied.triggers {
                trigger.guard = guard;
            }
            supplied.trigger_scope = scope;
            supplied.positions = vec![position(0, &[(leaf, true)])];
            temporal::evaluate(package, at, &supplied, temporal::Limits::default())
        };

        let closed = unguarded(Some(false), Closure::Closed);
        let assessed = assessment(&closed);
        assert_eq!(
            assessed.activation,
            Activation::Inactive,
            "a closed, complete scope whose every trigger is guard-false is inactive",
        );
        assert!(
            assessed.instance.is_none() && assessed.captures.is_empty(),
            "a false guard creates no instance",
        );
        assert_eq!(assessed.truth, None, "a false guard is not a truth");

        let open = assessment(&unguarded(Some(false), Closure::Open)).activation;
        assert_eq!(
            open,
            Activation::Unknown {
                completeness: Completeness::Complete,
                execution: Execution::Completed,
            },
            "guard-false is not a disposition of its own",
        );

        // An unestablished guard is reported against its own trigger, so it is
        // neither read as a false guard nor erased by a healthy sibling.
        let unestablished = unguarded(None, Closure::Closed);
        let reported = obligations(&unestablished);
        assert!(
            !reported.is_empty(),
            "an unestablished guard is not silently discarded",
        );
        for obligation in reported {
            let temporal::Obligation::Unactivated {
                instance, error, ..
            } = obligation
            else {
                panic!("an unestablished guard does not activate: {obligation:?}")
            };
            assert!(
                matches!(
                    error,
                    Error::Incomplete(Incomplete {
                        dimension: Dimension::Guard,
                        ..
                    })
                ),
                "{error:?}",
            );
            assert!(
                !instance.is_empty(),
                "the failure names the trigger it belongs to",
            );
        }
        // The guard-false scope reports one assessed, inactive obligation; the
        // unestablished guard reports unactivated ones. The two shapes differ.
        assert!(
            obligations(&closed)
                .iter()
                .all(|obligation| matches!(obligation, temporal::Obligation::Assessed(_))),
            "a guard-false scope is assessed, not unactivated",
        );
    });
}

/// FR-044-AC-8: a whole-execution origin creates exactly one instance per
/// admitted execution origin, keyed by the clause subject and that identity;
/// two admitted executions never alias, a re-evaluated execution does not
/// split, and conflicting payloads under one origin identity refuse.
#[test]
#[trace("TC-123", "FR-044-AC-8")]
fn a_whole_execution_origin_keys_one_instance_per_admitted_execution() {
    let body = "eventually[0,1] holds(view.ready)";
    admitted(&origin("Due", EVENT_POSITION, body), |package, at| {
        let leaf = holds_leaf(package, at);
        let mut supplied = delivered(vec![
            delivery("execution:1", "receipt:1", "started", &[]),
            delivery("execution:2", "receipt:2", "started", &[]),
        ]);
        supplied.positions = vec![position(0, &[(leaf, true)])];
        let report = temporal::evaluate(package, at, &supplied, temporal::Limits::default());
        let [first, second] = obligations(&report) else {
            panic!("one instance per admitted execution origin")
        };
        let (first, second) = (assessed(first), assessed(second));
        assert_eq!(first.instance.as_deref(), Some("execution:1"));
        assert_eq!(second.instance.as_deref(), Some("execution:2"));
        assert_eq!(first.subject.declaration, at, "keyed by the clause subject");
        assert_eq!(second.subject.declaration, at);
        assert_ne!(first.subject.instance, second.subject.instance);
        assert_eq!(first.receipts, ["receipt:1"]);
        assert_eq!(second.receipts, ["receipt:2"]);

        // Re-evaluating one admitted execution does not split it.
        let mut single_execution = supplied.clone();
        single_execution.triggers.truncate(1);
        for watermark in [0, 4] {
            let mut incremental = single_execution.clone();
            incremental.watermark = watermark;
            let report = temporal::evaluate(package, at, &incremental, temporal::Limits::default());
            assert_eq!(obligations(&report).len(), 1, "watermark {watermark}");
            assert_eq!(
                assessment(&report).instance.as_deref(),
                Some("execution:1"),
                "an incremental re-evaluation keeps one instance identity",
            );
        }

        let mut conflicting = single_execution.clone();
        conflicting
            .triggers
            .push(delivery("execution:1", "receipt:3", "restarted", &[]));
        let report = temporal::evaluate(package, at, &conflicting, temporal::Limits::default());
        let error = report
            .result()
            .expect_err("conflicting payloads under one origin identity refuse");
        let temporal::Error::Refused(Refusal::Contradiction { identity, .. }) = error else {
            panic!("expected a typed contradiction, got {error:?}")
        };
        assert_eq!(identity, "execution:1");
    });
}

/// FR-044-AC-9: an evicted retained capture makes its instance incomplete, and
/// no capture initializer is re-evaluated at a later anchor during incremental
/// re-evaluation, restoration or replay.
#[test]
#[trace("TC-123", "FR-044-AC-9")]
fn an_evicted_capture_makes_its_instance_incomplete() {
    admitted(&refund(REFUND), |package, at| {
        let leaf = holds_leaf(package, at);
        let mut supplied = delivered(vec![delivery(
            "order:1",
            "receipt:1",
            "failed",
            &["5", "payment:a"],
        )]);
        supplied.decision_scope = Closure::Open;
        supplied.positions = vec![position(0, &[(leaf, false)])];
        let report = temporal::evaluate(package, at, &supplied, temporal::Limits::default());
        let assessed = assessment(&report);
        assert_eq!(assessed.activation, Activation::Active);
        assert_eq!(
            assessed.truth,
            Some(Truth::Pending),
            "the instance remains unsettled",
        );
        assert_eq!(assessed.capture_evaluations, 1);

        // Incremental re-evaluation at a later watermark, an admitted
        // restoration and a replay all reuse the established captures.
        let mut incremental = supplied.clone();
        incremental.watermark = 5;
        incremental.positions.push(position(1, &[(leaf, false)]));
        for (name, trace) in [
            ("incremental", &incremental),
            ("restoration", &supplied.clone()),
            ("replay", &supplied),
        ] {
            let report = temporal::evaluate(package, at, trace, temporal::Limits::default());
            assert_eq!(
                assessment(&report).capture_evaluations,
                1,
                "{name}: no initializer is re-evaluated",
            );
            assert_eq!(values(assessment(&report)), ["5", "payment:a"], "{name}");
        }

        let mut evicted = incremental.clone();
        evicted.evicted = vec![Eviction::Capture {
            instance: "order:1".into(),
            capture: 1,
        }];
        let report = temporal::evaluate(package, at, &evicted, temporal::Limits::default());
        let [obligation] = obligations(&report) else {
            panic!("one obligation")
        };
        assert_eq!(
            unactivated(obligation),
            &Error::Incomplete(Incomplete {
                dimension: Dimension::Capture,
                subject: temporal::Subject {
                    declaration: at,
                    instance: 0,
                    node: None,
                    position: None,
                    capture: Some(1),
                },
            }),
            "an evicted retained capture is named, incomplete and never recomputed",
        );
    });
}
