// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-522 and FR-523 (TC-1317, TC-1318) over the evidence call itself: the
//! authenticated binding a result carries, the refusal of a differing
//! binding with an empty charge log, the `result_identity` of each clause
//! kind and of each member of a request, and the retry of a stopped call.
//!
//! Every request goes through [`evaluate_temporal_evidence`]; the identities
//! compared are the ones the call returns, not ones a test builds.

use ix_trace_rs::trace;
use qsl_eval::temporal::evidence::{
    evaluate_temporal_evidence, EvidenceCounter, EvidenceRequest, EvidenceResult,
    TemporalEvidenceLimits,
};
use qsl_eval::temporal::{
    ClockCover, OrderKey, ProgressClosure, ProgressCompleteness, ProgressLedger, Valuation,
};
use qsl_semantics::check::SCALAR_LIMITS_UNLIMITED;
use quire_eval::{CallAdmissionFailure, CallFailure};
use quire_exact::{Meter, ScalarLimits};
use serde_json::json;

use crate::temporal_activation::{
    call, delivery, each_env, env, event_env, fixed_env, id, origin, position, request, rows_for,
    unlimited, Env,
};
use crate::temporal_trace::fixture;

/// An edit of one request member.
type RequestEdit = Box<dyn Fn(&mut EvidenceRequest)>;

const BODY: &str = "eventually[0, 2] holds(p)";
const FIXED_PROFILE: &str = "quire.temporal.fixed-sample.false-extension/v1";
const EVENT_PROFILE: &str = "quire.temporal.event-position.false-extension/v1";

fn fixed_clock(epoch: &str, period: &str, unit: &str) -> serde_json::Value {
    json!({
        "kind": "fixed_sample",
        "epoch": {"kind": "integer", "decimal": epoch},
        "period": {"kind": "integer", "decimal": period},
        "unit": unit,
    })
}

/// The fixed-sample clause of `BODY` under the clock row `clock`.
fn fixed_with(clock: serde_json::Value) -> Env {
    rows_for(
        fixture(FIXED_PROFILE, "c: Counter", "on origin", BODY),
        FIXED_PROFILE,
        clock,
    )
}

/// The three clause kinds of the recipe, each `on origin` over `BODY`.
fn kinds() -> [(&'static str, Env); 3] {
    [
        ("finite-window", env("on origin", BODY)),
        ("fixed-sample", fixed_env("on origin", BODY)),
        ("event-position", event_env("on origin", BODY)),
    ]
}

/// One origin and three positions at which `p` is true.
fn origin_request(env: &Env) -> EvidenceRequest {
    let p = env.atom("p");
    let mut asked = request(
        env,
        (0..3)
            .map(|index| position(index, &[(p, Valuation::True)]))
            .collect(),
    );
    asked.origins = vec![origin("o1", "a")];
    asked.trigger = Some(id("o1"));
    asked
}

fn evaluated(env: &Env, asked: &EvidenceRequest) -> EvidenceResult {
    call(env, &env.subject(&[]), asked).expect("the call returns a result")
}

fn identity_of(env: &Env, asked: &EvidenceRequest) -> String {
    format!("{:?}", evaluated(env, asked).result_identity)
}

/// FR-522-AC-9: a request that agrees on profile, clock name and parameters
/// is evaluated, and its result's `authenticated` holds the package, the
/// declaration, the profile definition, the clock name and the clock
/// configuration of the lock and the `temporal_bindings` row.
#[trace("FR-522-AC-9", "TC-1317")]
#[test]
fn an_agreeing_request_is_evaluated_and_reports_the_rows_it_agreed_with() {
    for (kind, env) in kinds() {
        let subject = env.subject(&[]);
        let result = evaluated(&env, &origin_request(&env));
        let row = subject.occurrences[0].clock.expect("a clock row");
        let authenticated = &result.authenticated;
        assert_eq!(&*authenticated.package, "recipe", "{kind}");
        assert_eq!(&authenticated.declaration, subject.declaration, "{kind}");
        assert_eq!(authenticated.profile, subject.profile.definition, "{kind}");
        assert_eq!(authenticated.clock_name, row.clock_name, "{kind}");
        assert_eq!(authenticated.clock, row.clock, "{kind}");
    }
}

/// The refusal an `Err(CallFailure::Admission)` carries: its
/// `mismatched_dimension` field.
fn mismatched_dimension(failure: CallFailure) -> String {
    let CallFailure::Admission(failure) = failure else {
        panic!("an admission failure, not {failure:?}");
    };
    let CallAdmissionFailure::Refused(record) = *failure else {
        panic!("a refusal");
    };
    record
        .record
        .fields
        .get("mismatched_dimension")
        .cloned()
        .expect("the refusal names the mismatched dimension")
}

/// FR-522-AC-13: the refusal of AC-1 to AC-6 is `Err(CallFailure::Admission)`
/// carrying the same `mismatched_dimension`, with an empty charge log and no
/// result.
#[trace("FR-522-AC-13", "TC-1317")]
#[test]
fn a_differing_binding_is_an_admission_error_that_charges_nothing() {
    let fixed = fixed_env("on origin", BODY);
    let fixed_edits: Vec<(&str, RequestEdit)> = vec![
        (
            "Profile",
            Box::new(|r| r.binding.profile = EVENT_PROFILE.into()),
        ),
        (
            "Clock",
            Box::new(|r| r.binding.clock_name = "another-clock".into()),
        ),
        (
            "ClockParameters",
            Box::new(|r| {
                r.binding.parameters = serde_json::from_value(
                    json!({"kind": "event_position", "sequence_authority": "order-events"}),
                )
                .expect("a clock");
            }),
        ),
        (
            "Epoch",
            Box::new(|r| {
                r.binding.parameters =
                    serde_json::from_value(fixed_clock("7", "1", "sample")).expect("a clock");
            }),
        ),
        (
            "SamplePeriod",
            Box::new(|r| {
                r.binding.parameters =
                    serde_json::from_value(fixed_clock("0", "9", "sample")).expect("a clock");
            }),
        ),
        (
            "ClockUnit",
            Box::new(|r| {
                r.binding.parameters =
                    serde_json::from_value(fixed_clock("0", "1", "ms")).expect("a clock");
            }),
        ),
    ];
    for (dimension, edit) in fixed_edits {
        let mut asked = origin_request(&fixed);
        edit(&mut asked);
        let mut meter = unlimited();
        let failure = evaluate_temporal_evidence(&fixed.subject(&[]), &asked, &mut meter)
            .expect_err("a differing binding is an admission error, never a result");
        assert_eq!(mismatched_dimension(failure), dimension);
        assert!(meter.admitted_charges().is_empty(), "{dimension}");
    }

    let event = event_env("on origin", BODY);
    let mut asked = origin_request(&event);
    asked.binding.parameters = serde_json::from_value(
        json!({"kind": "event_position", "sequence_authority": "another-events"}),
    )
    .expect("a clock");
    let mut meter = unlimited();
    let failure = evaluate_temporal_evidence(&event.subject(&[]), &asked, &mut meter)
        .expect_err("another sequence authority");
    assert_eq!(mismatched_dimension(failure), "SequenceAuthority");
    assert!(meter.admitted_charges().is_empty());
}

/// FR-522-AC-15 and AC-16: evaluating one request of each of the three
/// clause kinds twice gives one `result_identity` per request, and the three
/// identities are pairwise distinct.
#[trace("FR-522-AC-15", "FR-522-AC-16", "TC-1317")]
#[test]
fn a_request_has_one_result_identity_and_the_three_clause_kinds_have_three() {
    let identities: Vec<(&str, String)> = kinds()
        .into_iter()
        .map(|(kind, env)| {
            let asked = origin_request(&env);
            let first = identity_of(&env, &asked);
            assert_eq!(first, identity_of(&env, &asked), "{kind}");
            (kind, first)
        })
        .collect();
    for (index, (left_kind, left)) in identities.iter().enumerate() {
        for (right_kind, right) in identities.iter().skip(index + 1) {
            assert_ne!(left, right, "{left_kind} / {right_kind}");
        }
    }
}

/// FR-522-AC-17 and AC-18: over the fixed-sample clause, changing exactly
/// one member of the identity list gives a `result_identity` distinct from
/// the unchanged request's and from every other change, and the first
/// request's result is neither reused nor rewritten by evaluating the changed
/// ones. Varied alone: the clock row's `epoch`, `period` and `unit`, the
/// cover, one stamp, one valuation, one order key, the watermark, `through`,
/// the scope closure, the surrounding-execution closure, the lower boundary
/// and each of the thirteen ceilings; the meter's work-unit ceiling is varied
/// through the meter.
#[trace("FR-522-AC-17", "FR-522-AC-18", "TC-1317")]
#[test]
fn each_member_of_a_request_changes_its_result_identity_and_leaves_the_first_result() {
    let base_env = fixed_env("on origin", BODY);
    let base = origin_request(&base_env);
    let first = evaluated(&base_env, &base);
    let first_bytes = format!("{first:?}");
    let reference = format!("{:?}", first.result_identity);
    let p = base_env.atom("p");

    let mut variants: Vec<(String, String)> = Vec::new();
    // The clock row's members.
    for (name, clock) in [
        ("epoch", fixed_clock("1", "1", "sample")),
        ("period", fixed_clock("0", "2", "sample")),
        ("unit", fixed_clock("0", "1", "tick")),
    ] {
        let env = fixed_with(clock);
        variants.push((name.to_owned(), identity_of(&env, &origin_request(&env))));
    }
    // The request's own members.
    let edits: Vec<(&str, RequestEdit)> = vec![
        (
            "cover",
            Box::new(|r| r.cover = ClockCover::Counter { uncertainty: 1 }),
        ),
        ("stamp", Box::new(|r| r.positions[2].position = 3)),
        (
            "valuation",
            Box::new(move |r| {
                r.positions[1].valuations.insert(p, Valuation::False);
            }),
        ),
        (
            "order key",
            Box::new(|r| {
                r.positions[0].order_key = Some(OrderKey {
                    authority: "order".to_owned(),
                    key: 1,
                });
            }),
        ),
        ("watermark", Box::new(|r| r.decision_progress = Some(99))),
        (
            "through",
            Box::new(|r| {
                if let Some(record) = r.completeness.as_mut() {
                    record.state = ProgressCompleteness::Complete { through: 99 };
                }
            }),
        ),
        (
            "scope closure",
            Box::new(|r| r.decision_closure = Some(ProgressClosure::Open)),
        ),
        (
            "surrounding closure",
            Box::new(|r| r.surrounding_closure = Some(ProgressClosure::Open)),
        ),
        (
            "lower boundary",
            Box::new(|r| r.authoritative_origin = Some(false)),
        ),
    ];
    for (name, edit) in edits {
        let mut one = base.clone();
        edit(&mut one);
        variants.push((name.to_owned(), identity_of(&base_env, &one)));
    }
    for counter in EvidenceCounter::ALL {
        let mut one = base.clone();
        one.limits = TemporalEvidenceLimits::default().with_ceiling(
            counter,
            TemporalEvidenceLimits::default().ceiling(counter) - 1,
        );
        variants.push((
            format!("ceiling {}", counter.name()),
            identity_of(&base_env, &one),
        ));
    }
    // The meter's work-unit ceiling.
    let metered = |work_units: u64| {
        let mut meter = Meter::new(ScalarLimits {
            work_units,
            ..SCALAR_LIMITS_UNLIMITED
        });
        let result = evaluate_temporal_evidence(&base_env.subject(&[]), &base, &mut meter)
            .expect("a result");
        format!("{:?}", result.result_identity)
    };
    variants.push(("meter ceiling".to_owned(), metered(1_000_000)));

    for (name, identity) in &variants {
        assert_ne!(identity, &reference, "{name}");
    }
    for (index, (name, left)) in variants.iter().enumerate() {
        for (other, right) in variants.iter().skip(index + 1) {
            assert_ne!(left, right, "{name} / {other}");
        }
    }
    // AC-18: the first request's result is evaluated again after the changed
    // ones, and its bytes equal those it returned before.
    assert_eq!(format!("{:?}", evaluated(&base_env, &base)), first_bytes);
}

/// FR-522-AC-19: for an event-position request, changing alone the
/// `sequence_authority` of the row, the watermark or the scope closure gives
/// a distinct `result_identity`.
#[trace("FR-522-AC-19", "TC-1317")]
#[test]
fn an_event_position_identity_depends_on_authority_watermark_and_scope() {
    let base_env = event_env("on origin", BODY);
    let base = origin_request(&base_env);
    let reference = identity_of(&base_env, &base);

    let other_authority = rows_for(
        fixture(EVENT_PROFILE, "c: Counter", "on origin", BODY),
        EVENT_PROFILE,
        json!({"kind": "event_position", "sequence_authority": "other-events"}),
    );
    let by_authority = identity_of(&other_authority, &origin_request(&other_authority));

    let mut moved = base.clone();
    moved.decision_progress = Some(99);
    let by_watermark = identity_of(&base_env, &moved);

    let mut open = base.clone();
    open.decision_closure = Some(ProgressClosure::Open);
    let by_scope = identity_of(&base_env, &open);

    let all = [&reference, &by_authority, &by_watermark, &by_scope];
    for (index, left) in all.iter().enumerate() {
        for right in all.iter().skip(index + 1) {
            assert_ne!(left, right);
        }
    }
}

/// A request over the `each` clause of one captured delivery.
fn each_request(env: &Env, p: u64, trigger: &str) -> EvidenceRequest {
    let mut asked = request(
        env,
        (0..3)
            .map(|index| position(index, &[(p, Valuation::True)]))
            .collect(),
    );
    asked.deliveries = vec![delivery(trigger, "r", "payload", &["5"])];
    asked.trigger = Some(id(trigger));
    asked
}

/// FR-522-AC-22: two requests that differ only in a retained-instance entry
/// of the ledger have distinct `result_identity` values.
#[trace("FR-522-AC-22", "TC-1317")]
#[test]
fn a_retained_instance_entry_changes_the_result_identity() {
    let (env, p) = each_env();
    let subject = env.subject(&[]);
    let seeded = |trigger: &str| {
        call(&env, &subject, &each_request(&env, p, trigger))
            .expect("a seeding result")
            .ledger
    };
    let (left, right) = (seeded("t0"), seeded("t9"));
    assert_ne!(left, right, "the two seeds retain different instances");
    let with = |ledger| {
        let mut asked = each_request(&env, p, "t1");
        asked.ledger = Some(ledger);
        format!(
            "{:?}",
            call(&env, &subject, &asked)
                .expect("a result")
                .result_identity
        )
    };
    assert_ne!(with(left), with(right));
}

/// FR-522-AC-23: two requests that differ only in the progress-ledger entry
/// have distinct `result_identity` values.
#[trace("FR-522-AC-23", "TC-1317")]
#[test]
fn a_progress_ledger_entry_changes_the_result_identity() {
    let (env, p) = each_env();
    let subject = env.subject(&[]);
    let seeded = |watermark: u64| {
        let mut asked = each_request(&env, p, "t0");
        asked.progress_ledger = Some(ProgressLedger::new());
        asked.decision_progress = Some(watermark);
        call(&env, &subject, &asked)
            .expect("a seeding result")
            .progress_ledger
            .expect("the call returns the ledger it was given")
    };
    let (left, right) = (seeded(100), seeded(90));
    assert_ne!(left, right, "the two seeds retain different watermarks");
    let with = |ledger| {
        let mut asked = each_request(&env, p, "t1");
        asked.progress_ledger = Some(ledger);
        format!(
            "{:?}",
            call(&env, &subject, &asked)
                .expect("a result")
                .result_identity
        )
    };
    assert_ne!(with(left), with(right));
}

/// FR-523-AC-11: a retry of the AC-3 call after an AC-5 stop, with sufficient
/// ceilings from the same inputs, returns the result of a call that never
/// stopped, byte for byte.
#[trace("FR-523-AC-11", "TC-1318")]
#[test]
fn a_retry_after_a_counter_stop_equals_a_call_that_never_stopped() {
    let env = env("on origin", "eventually[0, 5] holds(p)");
    let p = env.atom("p");
    let subject = env.subject(&[]);
    let mut asked = request(
        &env,
        (0..3)
            .map(|index| position(index, &[(p, Valuation::False)]))
            .collect(),
    );
    asked.origins = vec![origin("o1", "a")];
    asked.trigger = Some(id("o1"));

    let never_stopped = call(&env, &subject, &asked).expect("a result");
    assert_eq!(never_stopped.usage.get(EvidenceCounter::Positions), 3);

    let mut stopped = asked.clone();
    stopped.limits = TemporalEvidenceLimits::default().with_ceiling(EvidenceCounter::Positions, 2);
    let stop = call(&env, &subject, &stopped).expect("a stop is an Ok result");
    assert_eq!(stop.truth, None);
    assert!(
        stop.incomplete.is_some(),
        "the lowered ceiling stops the call"
    );

    let retry = call(&env, &subject, &asked).expect("a result");
    assert_eq!(format!("{retry:?}"), format!("{never_stopped:?}"));
}
