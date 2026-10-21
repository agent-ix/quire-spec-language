// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-234 (TC-689): the `TemporalTrace` family's check of a clause under the
//! timed profile `quire.temporal.timed/v1`, over TC-458's `ConfigVersion`
//! fixture package (`crate::model_operations`) with the unit's own `Time`
//! dimension and `ms` and `s` units.
//!
//! No `time` member reaches S3 (FR-230's facet gate), so the model `Config`
//! is made timed by supplying its checked time source, `dense` in `ms`, as
//! [`PackageDeclarations::time_sources`]. The family check does not type
//! `holds` conditions (the package check does, FR-337), so atoms are text.

use ix_trace_rs::trace;
use qsl_foundation::bound::{InvalidTimedInterval, TimedInterval};
use qsl_foundation::diagnostic::{Code, StageFailure};
use qsl_foundation::Span;
use qsl_semantics::check::{
    check_clauses, CheckCause, CheckRefusal, CheckedInterval, CheckedTemporalClause,
    ModelTimeBinding, ModelTimeSource, PackageDeclarations, TemporalCause, TemporalProfile,
    TemporalTraceFamily, TimeSourceKind, TimeUnitCause, TimedPropertyForm,
};
use qsl_semantics::family::{FamilyContract, UnsupportedItem};
use quire_exact::{Integer, Rational};
use serde_json::json;

use crate::model_operations::{
    admit_and_assemble_with_profiles, attempt_update_modifies_version_number,
    config_unit_with_profiles, config_version_document,
};

const TIMED: &str = "quire.temporal.timed/v1";
const OVER: &str = "Config::ConfigVersion";
const UNITS: &str = "dimension Time;\n\
     dimension Length;\n\
     unit s : Time = rational(1, 1);\n\
     unit ms : Time = rational(1, 1000) * s;\n\
     unit m : Length = rational(1, 1);\n";

type Outcome = Result<CheckedTemporalClause, StageFailure<CheckRefusal>>;

fn document() -> Vec<u8> {
    config_version_document(
        attempt_update_modifies_version_number(),
        Vec::new(),
        Vec::new(),
        json!([]),
    )
}

/// The checked time source that makes `Config` a timed model.
fn dense_ms() -> ModelTimeSource {
    ModelTimeSource {
        model: "Config".to_owned(),
        source: TimeSourceKind::Dense,
        unit: "ms".to_owned(),
    }
}

/// A unit under the timed profile with `units` declared and one clause `C`
/// over `c: over` with the clock role `clock` and `body`, `Config` timed by
/// `sources`: the unit text, the assembled package and the checked clause.
fn check_with(
    units: &str,
    over: &str,
    clock: &str,
    body: &str,
    sources: Vec<ModelTimeSource>,
) -> (String, PackageDeclarations, Outcome) {
    let profiles = format!("profile t = \"{TIMED}\";\n");
    let declarations = format!(
        "{units}temporal C using t over (c: {over}) clock \"{clock}\" on origin {{ {body} }}"
    );
    let mut package = admit_and_assemble_with_profiles(&document(), &profiles, &declarations)
        .unwrap_or_else(|refusal| panic!("{body}: the unit assembles: {refusal:?}"));
    package.time_sources = sources;
    let (text, _) = config_unit_with_profiles(&document(), &profiles, &declarations);
    let mut checked = check_clauses(&package);
    let Some(outcome) = checked.pop() else {
        panic!("{body}: one clause");
    };
    assert!(checked.is_empty(), "{body}: one clause");
    (text, package, outcome)
}

fn checked(body: &str) -> (String, CheckedTemporalClause) {
    match check_with(UNITS, OVER, "model-time", body, vec![dense_ms()]) {
        (text, _, Ok(clause)) => (text, clause),
        (_, _, Err(failure)) => panic!("{body}: the clause checks, not {failure:?}"),
    }
}

fn cause_of(outcome: Outcome) -> TemporalCause {
    match outcome {
        Err(StageFailure::Refused(CheckRefusal {
            cause: CheckCause::Temporal(cause),
            ..
        })) => *cause,
        other => panic!("a temporal refusal, not {other:?}"),
    }
}

fn refused(body: &str) -> (String, TemporalCause) {
    let (text, _, outcome) = check_with(UNITS, OVER, "model-time", body, vec![dense_ms()]);
    (text, cause_of(outcome))
}

/// The span of the first occurrence of `needle` in `text`.
fn first(text: &str, needle: &str) -> Span {
    let start = text.find(needle).expect("the needle is in the unit");
    Span {
        start,
        end: start + needle.len(),
    }
}

fn ratio(numerator: u64, denominator: u64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("nonzero")
}

fn whole(value: u64) -> Rational {
    ratio(value, 1)
}

/// The timed interval of the clause's only interval operator.
fn only_interval(clause: &CheckedTemporalClause) -> &TimedInterval {
    let timed: Vec<&TimedInterval> = clause
        .operators()
        .iter()
        .filter_map(|operator| match &operator.interval {
            Some(CheckedInterval::Timed(interval)) => Some(interval),
            _ => None,
        })
        .collect();
    let [interval] = timed.as_slice() else {
        panic!("one timed interval: {timed:?}");
    };
    interval
}

/// FR-234 "Forms", over the timed `Config` model: `Settles` is timed safety,
/// `always holds(I)` a timed invariant and an `on origin` interval-only
/// formula a bounded window whose horizon is the sum of its upper bounds.
/// `always eventually[0 ms, 5 ms] holds(P)` is in ADR-014 A-4's safety
/// fragment as ADR-018 IV-4 states it, so FR-234's rule order classifies
/// it `TimedSafety`; FR-234-AC-1 lists it as `TimedLiveness`, which the
/// rule does not give (remainder). An unbounded `eventually` over an
/// interval operator is outside the fragment: `TimedLiveness`.
#[trace("TC-689", "FR-234")]
#[test]
fn timed_clauses_classify_in_fr234_order() {
    for (body, form) in [
        (
            "always (holds(c.healthy) implies eventually[0 ms, 3 ms] holds(not c.healthy))",
            TimedPropertyForm::TimedSafety,
        ),
        (
            "always holds(not c.healthy)",
            TimedPropertyForm::TimedInvariant,
        ),
        (
            "eventually[0 ms, 3 ms] holds(c.versionNumber = 2)",
            TimedPropertyForm::BoundedWindow { horizon: whole(3) },
        ),
        (
            "always[0 ms, 1 ms] eventually(1 ms, 3 ms] holds(c.healthy)",
            TimedPropertyForm::BoundedWindow { horizon: whole(4) },
        ),
        (
            "always eventually[0 ms, 5 ms] holds(c.versionNumber = 0)",
            TimedPropertyForm::TimedSafety,
        ),
        (
            "eventually always[0 ms, 5 ms] holds(c.healthy)",
            TimedPropertyForm::TimedLiveness,
        ),
    ] {
        let (_, clause) = checked(body);
        assert_eq!(clause.profile(), TemporalProfile::Timed, "{body}");
        assert_eq!(clause.timed_form(), Some(&form), "{body}");
        assert_eq!(clause.property_form(), None, "{body}");
        assert_eq!(clause.horizon(), None, "{body}");
        assert_eq!(
            clause.time_binding(),
            Some(&ModelTimeBinding {
                model: "Config".to_owned(),
                source: TimeSourceKind::Dense,
                unit: "ms".to_owned(),
            }),
            "{body}"
        );
        assert_eq!(clause.unsupported_punctual(), None, "{body}");
    }
}

/// FR-234-AC-2: `[0 ms, 3 ms)` and `(1 ms, 3 ms]` check with their ends'
/// openness recorded and FR-255 keys unequal to `[0 ms, 3 ms]`'s;
/// `(2 ms, 2 ms]` and `[3 ms, 2 ms]` refuse
/// `invalid_timed_model`/`invalid-interval` at the interval's span. A bound
/// in `s` converts exactly to the binding's `ms` (FR-233 "Intervals").
#[trace("TC-689", "FR-234-AC-2")]
#[test]
fn timed_intervals_keep_their_openness_and_refuse_empty_or_inverted_bounds() {
    let key = |body: &str| {
        let (_, clause) = checked(body);
        let operator = clause.operators()[0].node;
        assert_eq!(clause.interval_key(operator), None, "{body}");
        let key = clause
            .timed_interval_key(operator)
            .expect("a timed interval key");
        (only_interval(&clause).clone(), key)
    };
    let (closed, closed_key) = key("eventually[0 ms, 3 ms] holds(c.healthy)");
    let (right_open, right_open_key) = key("eventually[0 ms, 3 ms) holds(c.healthy)");
    let (left_open, left_open_key) = key("eventually(1 ms, 3 ms] holds(c.healthy)");
    assert_eq!((closed.lower_closed(), closed.upper_closed()), (true, true));
    assert_eq!(
        (right_open.lower_closed(), right_open.upper_closed()),
        (true, false)
    );
    assert_eq!(
        (left_open.lower_closed(), left_open.upper_closed()),
        (false, true)
    );
    assert_eq!(
        (left_open.lower(), left_open.upper()),
        (&whole(1), &whole(3))
    );
    assert_ne!(right_open_key, closed_key);
    assert_ne!(left_open_key, closed_key);
    let (seconds, seconds_key) = key("eventually[0 s, decimal(3, 3) s] holds(c.healthy)");
    assert_eq!(seconds, closed);
    assert_eq!(seconds_key, closed_key);
    let (_, half) = checked("eventually[0 ms, rational(1, 2) ms] holds(c.healthy)");
    assert_eq!(only_interval(&half).upper(), &ratio(1, 2));

    for (body, interval, reason) in [
        (
            "eventually(2 ms, 2 ms] holds(c.healthy)",
            "(2 ms, 2 ms]",
            InvalidTimedInterval::EmptyOpenEnd,
        ),
        (
            "eventually[3 ms, 2 ms] holds(c.healthy)",
            "[3 ms, 2 ms]",
            InvalidTimedInterval::Inverted,
        ),
    ] {
        let (text, cause) = refused(body);
        assert_eq!(
            cause,
            TemporalCause::InvalidTimedInterval {
                reason,
                span: first(&text, interval),
            },
            "{body}"
        );
        assert_eq!(cause.code(), Code::InvalidTimedModel);
        assert_eq!(cause.tag(), "invalid-interval");
    }
    let (text, cause) = refused("eventually[1 s, 2 ms] holds(c.healthy)");
    assert_eq!(
        cause,
        TemporalCause::InvalidTimedInterval {
            reason: InvalidTimedInterval::Inverted,
            span: first(&text, "[1 s, 2 ms]"),
        }
    );
}

/// FR-234-AC-3: the punctual `always (holds(p) implies eventually[2 ms,
/// 2 ms] holds(q))` over the timed subject is disposed `Unsupported`,
/// `PunctualInterval`, naming the interval, in its requirement record; the
/// bounded `eventually[2 ms, 2 ms] holds(q)` on origin is admitted as
/// `BoundedWindow{horizon: 2}` with no disposition.
#[trace("TC-689", "FR-234-AC-3")]
#[test]
fn a_punctual_interval_is_unsupported_outside_a_bounded_window() {
    let (text, unbounded) =
        checked("always (holds(c.healthy) implies eventually[2 ms, 2 ms] holds(not c.healthy))");
    let span = first(&text, "[2 ms, 2 ms]");
    assert!(only_interval(&unbounded).is_punctual());
    assert_eq!(unbounded.unsupported_punctual(), Some(span));
    let [claim] = TemporalTraceFamily::requirements(&unbounded)
        .try_into()
        .expect("one claim");
    let requirements = claim
        .requirements(
            qsl_foundation::digest::WireNodeId::from_digest([1; 32]),
            &[qsl_foundation::digest::WireNodeId::from_digest([2; 32])],
            Some(qsl_foundation::bound::DomainKey::Population {
                member_type: qsl_foundation::digest::WireNodeId::from_digest([3; 32]),
                ordinal: 0,
            }),
            &[],
        )
        .expect("the record is keyed");
    assert_eq!(
        requirements.unsupported(),
        Some(&UnsupportedItem::PunctualInterval { span })
    );

    let (_, bounded) = checked("eventually[2 ms, 2 ms] holds(not c.healthy)");
    assert_eq!(
        bounded.timed_form(),
        Some(&TimedPropertyForm::BoundedWindow { horizon: whole(2) })
    );
    assert!(only_interval(&bounded).is_punctual());
    assert_eq!(bounded.unsupported_punctual(), None);
}

/// FR-233 as FR-234 reads it: `model-time` over a model with no time source
/// refuses `invalid_timed_model`/`model-time-on-untimed-model` at the clock
/// clause naming the model; another clock role, or a subject that is no
/// model object type, has no stated timed binding and refuses as not yet
/// implemented.
#[trace("TC-689", "FR-234")]
#[test]
fn the_timed_profile_needs_a_model_time_binding_over_a_timed_model() {
    let body = "eventually[0 ms, 3 ms] holds(c.healthy)";
    let (text, _, outcome) = check_with(UNITS, OVER, "model-time", body, Vec::new());
    let cause = cause_of(outcome);
    assert_eq!(
        cause,
        TemporalCause::ModelTimeOnUntimedModel {
            model: "Config".to_owned(),
            span: first(&text, "\"model-time\""),
        }
    );
    assert_eq!(
        (cause.code(), cause.tag()),
        (Code::InvalidTimedModel, "model-time-on-untimed-model")
    );

    let (text, _, outcome) = check_with(UNITS, OVER, "model-steps", body, vec![dense_ms()]);
    assert_eq!(
        cause_of(outcome),
        TemporalCause::TimedProfileWithoutModelTime {
            clock: "model-steps".to_owned(),
            span: first(&text, "\"model-steps\""),
        }
    );

    let units = format!("{UNITS}record Loose {{ n: Integer; }}\n");
    let (text, _, outcome) = check_with(&units, "Loose", "model-time", body, vec![dense_ms()]);
    let cause = cause_of(outcome);
    assert_eq!(
        cause,
        TemporalCause::TimedTraceSubject {
            span: Span {
                start: first(&text, "temporal C").start,
                end: text.trim_end().len(),
            },
        }
    );
    assert_eq!(
        (cause.code(), cause.tag()),
        (Code::UnsupportedConstruct, "not-yet-implemented")
    );
}

/// FR-233 "Intervals" through FR-230's time units: a step interval has
/// bounds with no unit (`ill_typed`/`type-mismatch`), `[a, *]` refuses as
/// ADR-014 TR-3 states, and a bound whose unit is unknown or of another
/// dimension refuses at the unit. A supplied time source whose unit is no
/// time unit is a broken input, not a refusal: FR-230 checks the unit.
#[trace("TC-689", "FR-234")]
#[test]
fn timed_bounds_are_time_quantities_of_the_unit_graph() {
    let (text, cause) = refused("eventually[0, 3] holds(c.healthy)");
    assert_eq!(
        cause,
        TemporalCause::UnitlessTimedBound {
            span: first(&text, "[0, 3]")
        }
    );
    assert_eq!(
        (cause.code(), cause.tag()),
        (Code::IllTyped, "type-mismatch")
    );
    let (text, cause) = refused("eventually[3, *] holds(c.healthy)");
    assert_eq!(
        cause,
        TemporalCause::OpenInterval {
            span: first(&text, "[3, *]")
        }
    );

    for (body, unit, expected) in [
        (
            "eventually[0 ms, 3 hour] holds(c.healthy)",
            "hour",
            TimeUnitCause::UnknownUnit {
                name: "hour".to_owned(),
            },
        ),
        (
            "eventually[0 ms, 3 m] holds(c.healthy)",
            "m]",
            TimeUnitCause::NotTimeDimension {
                name: "m".to_owned(),
            },
        ),
    ] {
        let (text, cause) = refused(body);
        let mut at = first(&text, unit);
        at.end = at.start + unit.trim_end_matches(']').len();
        assert_eq!(
            cause,
            TemporalCause::TimeQuantity {
                cause: Box::new(expected),
                span: at,
            },
            "{body}"
        );
    }
    let (_, cause) = refused("eventually[0 ms, 3 hour] holds(c.healthy)");
    assert_eq!(
        (cause.code(), cause.tag()),
        (Code::MissingDeclaration, "missing-name")
    );

    let no_time = "dimension Length;\nunit m : Length = rational(1, 1);\n";
    let (_, _, outcome) = check_with(
        no_time,
        OVER,
        "model-time",
        "eventually[0 m, 3 m] holds(c.healthy)",
        vec![ModelTimeSource {
            unit: "m".to_owned(),
            ..dense_ms()
        }],
    );
    assert!(
        matches!(outcome, Err(StageFailure::Fault(_))),
        "a time source whose unit is not a time unit is a broken input: {outcome:?}"
    );
}

/// A clause that checks under the timed profile is refused by the package
/// check as not emitted: S4 writes no timed interval (QSpec FR-370's timed
/// form), so the clause never compiles into a package that lacks it.
#[trace("TC-689", "FR-234")]
#[test]
fn a_checked_timed_clause_is_refused_as_not_emitted() {
    let body = "eventually[0 ms, 3 ms] holds(c.healthy)";
    let (text, package, outcome) = check_with(UNITS, OVER, "model-time", body, vec![dense_ms()]);
    assert!(outcome.is_ok(), "{outcome:?}");
    let refusals = package
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect_err("a timed clause is not emitted");
    let causes: Vec<&TemporalCause> = refusals
        .iter()
        .filter_map(|refusal| match &refusal.cause {
            CheckCause::Temporal(cause) => Some(&**cause),
            _ => None,
        })
        .collect();
    assert_eq!(
        causes,
        [&TemporalCause::NotEmitted {
            name: "C".to_owned(),
            span: Span {
                start: first(&text, "temporal C").start,
                end: text.trim_end().len(),
            },
        }],
        "{refusals:?}"
    );
}
