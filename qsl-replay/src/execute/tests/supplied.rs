// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-098-AC-9, FR-098-AC-12, FR-277-AC-4, FR-277-AC-5 (TC-906, TC-830):
//! the supplied-value phase budgets of the public replay. The caller's
//! `limits.supplied` ceilings bound the conversion helper work H and the
//! final admission helper work A as two cumulative budgets; a stop is a
//! phase-marked pre-call record with zero replay charges and no
//! `FunctionCall` charge point.
//!
//! The expected event sequences are enumerated here from FR-098's
//! conversion event table and QSV FR-109's membership event order, never
//! read back from the code under test.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use qsl_semantics::value::supplied::{
    AttemptedCount, SuppliedCause, SuppliedEvent, SuppliedLimitKind, SuppliedLimits, SuppliedPhase,
    SuppliedStop,
};
use quire_exact::{Cancel, CancelCause, Integer};

use super::*;
use crate::supplied_limits::{read_supplied_limits, SuppliedLimitsRefusal};

const SOURCE: &str = "function seq using v(xs: Sequence<Boolean>[0, 64]): Boolean pure { false }\n\
     function pair using v(a: Sequence<Boolean>[0, 64], b: Sequence<Boolean>[0, 64]): Boolean pure { false }\n\
     function big using v(xs: Sequence<Boolean>[0, 100000]): Boolean pure { false }\n\
     function digit using v(x: Int[0, 9]): Boolean pure { false }\n\
     function money using v(d: Decimal[0, 100; 0, 2; exact]): Boolean pure { false }\n";

struct Unit {
    source: String,
    compiled: ComposedUnit,
}

fn unit() -> Unit {
    let source = format!("language \"ix:native\" edition \"1-draft\";\n{PROFILE}{SOURCE}");
    let compiled = spine(&source, &BTreeMap::new());
    Unit { source, compiled }
}

impl Unit {
    fn request(&self, function: &str, arguments: Vec<WitnessValue>) -> ReplayRequestWire {
        let assignments = arguments
            .into_iter()
            .enumerate()
            .map(|(index, value)| CanonicalAssignment {
                parameter: parameter(&self.compiled, function, index),
                value,
            })
            .collect();
        request(
            self.source.as_bytes(),
            self.compiled.emitted.package_id(),
            name(&[function]),
            ReplaySource::Input(assignments),
        )
    }

    fn with(
        &self,
        function: &str,
        arguments: Vec<WitnessValue>,
        limits: SuppliedLimits,
    ) -> ReplayRequestWire {
        let mut wire = self.request(function, arguments);
        wire.supplied_limits = Some(limits);
        wire
    }
}

fn booleans(count: usize) -> WitnessValue {
    WitnessValue::Sequence(vec![WitnessValue::Boolean(true); count])
}

fn run(wire: ReplayRequestWire) -> Result<ReplayResult, ReplayRefusal> {
    replay(wire, ReplayLimits::default())
}

/// The conversion events H spends converting a `Sequence<Boolean>` of
/// `count` elements (FR-098's table): a preflight for the sequence and one
/// preflight and one materialization per element, the sequence's own
/// materialization after its children, then QSV's one value visit per
/// element admitted into the sequence.
fn conversion_events(count: usize) -> Vec<SuppliedEvent> {
    let mut events = vec![SuppliedEvent::WitnessPreflight];
    for _ in 0..count {
        events.push(SuppliedEvent::WitnessPreflight);
        events.push(SuppliedEvent::WitnessMaterialization);
    }
    events.push(SuppliedEvent::WitnessMaterialization);
    events.extend(std::iter::repeat_n(SuppliedEvent::ValueVisit, count));
    events
}

/// The QSV membership events A spends admitting the converted sequence:
/// the root visit, the kind-and-bound pair, the element type chain's one
/// pair, one scheduling per child, then one visit per child.
fn admission_events(count: usize) -> Vec<SuppliedEvent> {
    let mut events = vec![
        SuppliedEvent::ValueVisit,
        SuppliedEvent::TypeComparison,
        SuppliedEvent::TypeComparison,
    ];
    events.extend(std::iter::repeat_n(SuppliedEvent::ChildScheduling, count));
    events.extend(std::iter::repeat_n(SuppliedEvent::ValueVisit, count));
    events
}

fn total(events: &[SuppliedEvent]) -> u64 {
    u64::try_from(events.len()).unwrap()
}

fn only_h(units: u64) -> SuppliedLimits {
    SuppliedLimits::default().with_conversion_work_units(units)
}

fn only_a(units: u64) -> SuppliedLimits {
    SuppliedLimits::default().with_admission_work_units(units)
}

/// A pre-call limit: no call, `inconclusive` with cause `NoValue`, zero
/// replay charges, and no kernel `Incomplete` (so no charge point).
fn pre_call(result: &Result<ReplayResult, ReplayRefusal>) -> &SuppliedStop {
    let result = result.as_ref().expect("the replay settles");
    assert_eq!(result.category(), Category::Incomplete);
    assert!(matches!(
        result.disagreement(),
        Some(crate::result::DisagreementCause::NoValue { .. })
    ));
    assert_eq!(result.charges(), charges(std::iter::empty()), "no charge");
    assert!(
        result.limit().is_none(),
        "no kernel Incomplete, no FunctionCall"
    );
    result.pre_call_limit().expect("the pre-call limit record")
}

fn assert_reproduced(result: &Result<ReplayResult, ReplayRefusal>) {
    let result = result.as_ref().expect("the replay runs");
    assert_eq!(result.category(), Category::Violation);
    assert!(result.pre_call_limit().is_none());
    assert!(result.charges().work_units > 0, "the call ran");
}

/// QSpec FR-323 AC-10 (TC-278 RE-12, RE-14): an omitted `limits.supplied`
/// resolves to the published defaults before the request is read, so it is
/// the same request as one stating them, and changing either ceiling
/// changes the request.
#[trace("TC-906", "FR-098-AC-12")]
#[test]
fn a_defaulted_request_is_the_request_that_states_the_defaults() {
    let unit = unit();
    let decode = |wire| ReplayRequest::decode(wire, ReplayLimits::default()).expect("decodes");
    let omitted = decode(unit.request("seq", vec![booleans(2)]));
    assert_eq!(omitted.supplied_limits(), SuppliedLimits::default());
    let stated = decode(
        unit.with(
            "seq",
            vec![booleans(2)],
            SuppliedLimits::default()
                .with_admission_work_units(16_777_216)
                .with_conversion_work_units(16_777_216),
        ),
    );
    assert_eq!(
        omitted, stated,
        "defaulting does not change request identity"
    );
    assert_eq!(
        omitted.to_wire().supplied_limits,
        Some(SuppliedLimits::default())
    );

    let stated_b = decode(unit.with(
        "seq",
        vec![booleans(2)],
        SuppliedLimits::default().with_numeric_magnitude_bits(4096),
    ));
    assert_eq!(omitted, stated_b, "omitted B is the stated default 4096");
    let other_b = |bits| {
        decode(unit.with(
            "seq",
            vec![booleans(2)],
            SuppliedLimits::default().with_numeric_magnitude_bits(bits),
        ))
    };
    assert_ne!(omitted, other_b(4095), "B is part of the request");
    assert_ne!(omitted, other_b(4097));
    assert_eq!(omitted.package_id(), other_b(4095).package_id());

    let raised_a = decode(unit.with("seq", vec![booleans(2)], only_a(16_777_217)));
    let raised_h = decode(unit.with("seq", vec![booleans(2)], only_h(16_777_217)));
    assert_ne!(omitted, raised_a, "A is part of the request");
    assert_ne!(omitted, raised_h, "H is part of the request");
    assert_ne!(raised_a, raised_h);
    assert_eq!(
        raised_a.supplied_limits().conversion_work_units(),
        16_777_216
    );
    assert_eq!(
        raised_h.supplied_limits().admission_work_units(),
        16_777_216
    );
    assert_eq!(
        omitted.package_id(),
        raised_a.package_id(),
        "not package identity"
    );
}

/// QSpec FR-323 AC-10 (TC-278 RE-13), FR-255-AC-4: a supplied ceiling is
/// not a stage setting; naming one in `stage_limits` refuses before any
/// stage runs.
#[trace("TC-721", "FR-255-AC-4")]
#[test]
fn a_supplied_ceiling_in_stage_limits_refuses_before_any_stage() {
    let unit = unit();
    for member in [
        "supplied.admission_work_units",
        "supplied.conversion_work_units",
        "supplied.numeric_magnitude_bits",
    ] {
        let mut wire = unit.request("seq", vec![booleans(1)]);
        wire.stage_limits.insert(member.to_owned(), 7);
        let refusal = run(wire).expect_err("a stage entry that is no setting");
        assert!(
            matches!(
                &refusal,
                ReplayRefusal::Request(ReplayRequestRefusal::StageLimitEntry { name }) if name == member
            ),
            "{member}: {refusal:?}"
        );
        assert_eq!(refusal.code(), Code::InvalidRuntimeInput);
    }
}

/// QSpec FR-323 AC-10 (TC-278 RE-13): a wire `limits.supplied` object is
/// closed and complete with each ceiling an exact `u64`; anything else
/// refuses before consumption, never defaulted and never borrowed from
/// another limit.
#[trace("TC-721", "FR-255-AC-4")]
#[test]
fn a_wire_supplied_object_is_closed_complete_and_u64() {
    let read = |text: &str| read_supplied_limits(&serde_json::from_str(text).unwrap());
    assert_eq!(
        read(
            r#"{"admission_work_units":0,"conversion_work_units":18446744073709551615,
                "numeric_magnitude_bits":0}"#
        ),
        Ok(SuppliedLimits::default()
            .with_admission_work_units(0)
            .with_conversion_work_units(u64::MAX)
            .with_numeric_magnitude_bits(0))
    );
    for (text, member) in [
        (
            r#"{"admission_work_units":5,"numeric_magnitude_bits":5}"#,
            "conversion_work_units",
        ),
        (
            r#"{"conversion_work_units":5,"numeric_magnitude_bits":5}"#,
            "admission_work_units",
        ),
        (
            r#"{"admission_work_units":5,"conversion_work_units":5}"#,
            "numeric_magnitude_bits",
        ),
    ] {
        assert_eq!(
            read(text),
            Err(SuppliedLimitsRefusal::Incomplete { member }),
            "{text}"
        );
    }
    assert_eq!(
        read(
            r#"{"admission_work_units":1,"conversion_work_units":1,
                "numeric_magnitude_bits":1,"depth":2}"#
        ),
        Err(SuppliedLimitsRefusal::UnknownMember {
            name: "depth".to_owned()
        })
    );
    for malformed in [
        r#"{"admission_work_units":18446744073709551616,"conversion_work_units":1,"numeric_magnitude_bits":1}"#,
        r#"{"admission_work_units":-1,"conversion_work_units":1,"numeric_magnitude_bits":1}"#,
        r#"{"admission_work_units":1.5,"conversion_work_units":1,"numeric_magnitude_bits":1}"#,
        r#"{"admission_work_units":"1","conversion_work_units":1,"numeric_magnitude_bits":1}"#,
        r#"{"admission_work_units":null,"conversion_work_units":1,"numeric_magnitude_bits":1}"#,
    ] {
        assert_eq!(
            read(malformed),
            Err(SuppliedLimitsRefusal::NotUnsigned64 {
                member: "admission_work_units"
            }),
            "{malformed}"
        );
    }
    for malformed in [
        "-1",
        "1.5",
        "18446744073709551616",
        "null",
        "\"64\"",
        "true",
    ] {
        let text = format!(
            r#"{{"admission_work_units":1,"conversion_work_units":1,"numeric_magnitude_bits":{malformed}}}"#
        );
        assert_eq!(
            read(&text),
            Err(SuppliedLimitsRefusal::NotUnsigned64 {
                member: "numeric_magnitude_bits"
            }),
            "{text}"
        );
    }
    assert_eq!(read("[1,2]"), Err(SuppliedLimitsRefusal::NotAnObject));
    assert_eq!(read("5"), Err(SuppliedLimitsRefusal::NotAnObject));
    assert_eq!(
        SuppliedLimitsRefusal::NotAnObject.code(),
        Code::InvalidRuntimeInput
    );
}

/// FR-098-AC-12 (TC-906): H-1 denies its actual next conversion event
/// unspent, H permits conversion; the denial names the phase, parameter,
/// event, setting, ceiling, successful spend and denied amount, makes no
/// call and settles `inconclusive`/`NoValue`.
#[trace("TC-906", "FR-098-AC-12", "FR-277-AC-4")]
#[test]
fn one_less_than_the_conversion_events_denies_the_next_event_and_exact_permits() {
    let unit = unit();
    for count in [0_usize, 1, 4] {
        let events = conversion_events(count);
        let exact = total(&events);
        let at_exact = run(unit.with("seq", vec![booleans(count)], only_h(exact)));
        assert_reproduced(&at_exact);

        for denied in [0, exact - 1] {
            let result = run(unit.with("seq", vec![booleans(count)], only_h(denied)));
            let stop = pre_call(&result);
            let index = usize::try_from(denied).unwrap();
            assert_eq!(
                stop.context.phase,
                SuppliedPhase::Conversion,
                "{count}/{denied}"
            );
            assert_eq!(stop.context.parameter, Some(0));
            assert_eq!(stop.context.event, Some(events[index]), "{count}/{denied}");
            assert_eq!(stop.context.consumed, denied, "the denied step is unspent");
            let limit = stop.limit().expect("a limit");
            assert_eq!(limit.limit_kind, SuppliedLimitKind::ConversionWorkUnits);
            assert_eq!(limit.field(), "supplied.conversion_work_units");
            assert_eq!(limit.limit, denied);
            assert_eq!(limit.next_charge, 1);
            assert_eq!(limit.counter, AttemptedCount::Count(denied + 1));
        }
    }
}

/// FR-277-AC-4: A-1 denies the actual next admission event
/// unspent and A admits, with other bounds permitting progress; the pre-call
/// stop reports zero replay charges.
#[trace("TC-906", "FR-098-AC-12", "FR-277-AC-4")]
#[test]
fn one_less_than_the_admission_events_denies_the_next_event_and_exact_admits() {
    let unit = unit();
    for count in [0_usize, 1, 4] {
        let events = admission_events(count);
        let exact = total(&events);
        assert_reproduced(&run(unit.with("seq", vec![booleans(count)], only_a(exact))));
        for denied in [0, exact - 1] {
            let result = run(unit.with("seq", vec![booleans(count)], only_a(denied)));
            let stop = pre_call(&result);
            let index = usize::try_from(denied).unwrap();
            assert_eq!(
                stop.context.phase,
                SuppliedPhase::Admission,
                "{count}/{denied}"
            );
            assert_eq!(stop.context.parameter, Some(0));
            assert_eq!(stop.context.event, Some(events[index]), "{count}/{denied}");
            assert_eq!(stop.context.consumed, denied);
            let limit = stop.limit().expect("a limit");
            assert_eq!(limit.limit_kind, SuppliedLimitKind::AdmissionWorkUnits);
            assert_eq!(limit.field(), "supplied.admission_work_units");
            assert_eq!(limit.limit, denied);
            assert_eq!(limit.counter, AttemptedCount::Count(denied + 1));
        }
    }
}

/// FR-277: H and A are independent budgets and an event has one owner: the
/// ceiling of the other phase, however generous, neither pays for nor is
/// spent by an event.
#[trace("TC-906", "FR-098-AC-12")]
#[test]
fn the_two_ceilings_are_independent_budgets() {
    let unit = unit();
    let count = 3;
    let (h, a) = (
        total(&conversion_events(count)),
        total(&admission_events(count)),
    );
    let h_short = SuppliedLimits::default()
        .with_conversion_work_units(h - 1)
        .with_admission_work_units(u64::MAX);
    let result = run(unit.with("seq", vec![booleans(count)], h_short));
    assert_eq!(pre_call(&result).context.phase, SuppliedPhase::Conversion);

    let a_short = SuppliedLimits::default()
        .with_conversion_work_units(u64::MAX)
        .with_admission_work_units(a - 1);
    let result = run(unit.with("seq", vec![booleans(count)], a_short));
    assert_eq!(pre_call(&result).context.phase, SuppliedPhase::Admission);

    let both = SuppliedLimits::default()
        .with_conversion_work_units(h)
        .with_admission_work_units(a);
    assert_reproduced(&run(unit.with("seq", vec![booleans(count)], both)));
}

/// FR-098-AC-12, FR-277-AC-5: H and A are cumulative across arguments: the
/// second argument spends on top of the first and is the one denied.
#[trace("TC-906", "FR-098-AC-12", "FR-277-AC-5")]
#[test]
fn both_budgets_are_cumulative_across_arguments() {
    let unit = unit();
    let (first, second) = (2_usize, 3_usize);
    let arguments = || vec![booleans(first), booleans(second)];

    // H: the first argument's conversion events, then the second's.
    let h_second = conversion_events(second);
    let h_total = total(&conversion_events(first)) + total(&h_second);
    assert_reproduced(&run(unit.with("pair", arguments(), only_h(h_total))));
    let result = run(unit.with("pair", arguments(), only_h(h_total - 1)));
    let stop = pre_call(&result);
    assert_eq!(stop.context.phase, SuppliedPhase::Conversion);
    assert_eq!(stop.context.parameter, Some(1));
    assert_eq!(stop.context.consumed, h_total - 1);
    assert_eq!(stop.context.event, h_second.last().copied());

    // A: the same for the final admissions.
    let a_second = admission_events(second);
    let a_total = total(&admission_events(first)) + total(&a_second);
    assert_reproduced(&run(unit.with("pair", arguments(), only_a(a_total))));
    let result = run(unit.with("pair", arguments(), only_a(a_total - 1)));
    let stop = pre_call(&result);
    assert_eq!(stop.context.phase, SuppliedPhase::Admission);
    assert_eq!(stop.context.parameter, Some(1));
    assert_eq!(stop.context.consumed, a_total - 1);
    assert_eq!(stop.context.event, a_second.last().copied());
}

/// FR-098-AC-9 (TC-906), FR-277: the per-argument converted-node counter C
/// and occurrence count keep their meaning and are denied as phase-marked
/// pre-call records, not as a kernel `Incomplete` with a `FunctionCall`
/// charge point; neither is spent by helper events.
#[trace("TC-906", "FR-098-AC-9")]
#[test]
fn the_node_and_occurrence_limits_are_conversion_phase_pre_call_records() {
    let unit = unit();
    let count = 3;
    let nodes = u64::try_from(count + 1).unwrap();
    let mut wire = unit.request("seq", vec![booleans(count)]);
    wire.accounting_limits.work_units = nodes - 1;
    let result = run(wire);
    let stop = pre_call(&result);
    assert_eq!(stop.context.phase, SuppliedPhase::Conversion);
    assert_eq!(stop.context.event, Some(SuppliedEvent::ConvertedNode));
    assert_eq!(
        stop.context.consumed,
        nodes - 1,
        "the successful per-argument prefix"
    );
    let limit = stop.limit().expect("a limit");
    assert_eq!(limit.limit_kind, SuppliedLimitKind::WorkUnits);
    assert_eq!(limit.limit, nodes - 1);
    assert_eq!(limit.next_charge, 1);
    assert_eq!(limit.counter, AttemptedCount::Count(nodes));

    let mut at_bound = unit.request("seq", vec![booleans(count)]);
    at_bound.accounting_limits.work_units = nodes;
    // Evaluation needs its own allowance: the converted nodes do not seed it.
    let at_bound = run(at_bound);
    let at_bound = at_bound.as_ref().expect("the replay runs");
    assert!(at_bound.pre_call_limit().is_none(), "C at its bound admits");

    // A sequence of `count` booleans holds one occurrence per element and
    // one for itself.
    let occurrences = nodes;
    let mut wire = unit.request("seq", vec![booleans(count)]);
    wire.accounting_limits.value_occurrences = occurrences - 1;
    let result = run(wire);
    let stop = pre_call(&result);
    assert_eq!(stop.context.event, Some(SuppliedEvent::ValueOccurrence));
    assert_eq!(stop.context.consumed, 0);
    let limit = stop.limit().expect("a limit");
    assert_eq!(limit.limit_kind, SuppliedLimitKind::ValueOccurrences);
    assert_eq!(limit.limit, occurrences - 1);
    assert_eq!(limit.next_charge, occurrences, "the count reached");
}

/// A handle that cancels itself with `cause` at its `at`-th poll, and the
/// count of polls the handle has seen.
fn cancelling_at(at: u64, cause: CancelCause) -> (Cancel, Arc<AtomicU64>) {
    let slot: Arc<OnceLock<Cancel>> = Arc::new(OnceLock::new());
    let polls = Arc::new(AtomicU64::new(0));
    let (watch_slot, watch_polls) = (Arc::clone(&slot), Arc::clone(&polls));
    let cancel = Cancel::observing(move || {
        let seen = watch_polls.fetch_add(1, Ordering::Relaxed) + 1;
        if seen == at {
            if let Some(handle) = watch_slot.get() {
                handle.cancel(cause);
            }
        }
    });
    slot.set(cancel.clone()).expect("set once");
    (cancel, polls)
}

fn supplied_stop(result: Result<ReplayResult, ReplayRefusal>) -> SuppliedStop {
    match result {
        Err(ReplayRefusal::Supplied(stop)) => *stop,
        other => panic!("a supplied-phase refusal, got {other:?}"),
    }
}

/// FR-277-AC-5 (TC-906, TC-830): the original handle reaches the first
/// conversion helper: a handle cancelled before the replay stops it with its
/// original cause, as a cancellation and not as a limit, in each cause.
#[trace("TC-906", "FR-098-AC-12", "FR-277-AC-5")]
#[test]
fn a_handle_cancelled_before_conversion_stops_it_with_its_original_cause() {
    let unit = unit();
    for cause in [CancelCause::Requested, CancelCause::Deadline] {
        let cancel = Cancel::new();
        cancel.cancel(cause);
        let stop = supplied_stop(replay_with_cancel(
            unit.request("seq", vec![booleans(2)]),
            ReplayLimits::default(),
            &cancel,
        ));
        assert_eq!(stop.cause, SuppliedCause::Cancelled(cause));
        assert!(stop.limit().is_none());
        assert_eq!(stop.context.phase, SuppliedPhase::Conversion);
        assert_eq!(stop.context.parameter, Some(0));
        assert_eq!(stop.context.event, Some(SuppliedEvent::WitnessPreflight));
        assert_eq!(stop.context.consumed, 0);
    }
}

/// FR-277-AC-5: cancellation during a later argument stays a cancellation
/// with its original cause and the cumulative spend: the handle that stops
/// argument 1 is the one that served argument 0, in each cause.
#[trace("TC-906", "FR-098-AC-12", "FR-277-AC-5")]
#[test]
fn a_cancellation_during_a_later_argument_keeps_its_cause_and_the_spend() {
    let unit = unit();
    let arguments = || vec![booleans(2), booleans(2)];
    // The polls the handle sees converting the first argument alone, measured
    // through the handle itself: a run with no admission allowance stops at
    // its first admission poll, which is the one poll past the conversion.
    let (counting, polls) = cancelling_at(u64::MAX, CancelCause::Requested);
    replay_with_cancel(
        unit.with("seq", vec![booleans(2)], only_a(0)),
        ReplayLimits::default(),
        &counting,
    )
    .expect("the single argument settles at its first admission event");
    let first_argument_polls = polls.load(Ordering::Relaxed) - 1;
    assert!(first_argument_polls > 0);

    for cause in [CancelCause::Requested, CancelCause::Deadline] {
        let (cancel, _) = cancelling_at(first_argument_polls + 1, cause);
        let stop = supplied_stop(replay_with_cancel(
            unit.request("pair", arguments()),
            ReplayLimits::default(),
            &cancel,
        ));
        assert_eq!(stop.cause, SuppliedCause::Cancelled(cause));
        assert_eq!(stop.context.parameter, Some(1), "the later argument");
        assert_eq!(
            stop.context.consumed,
            total(&conversion_events(2)),
            "the first argument's spend is kept, with no reset"
        );
    }
}

/// FR-098-AC-8 (TC-906), FR-277-AC-4: an invalid argument keeps its
/// original `WrongValueKind` refusal and parameter, with the conversion
/// phase context; it is neither a limit nor a failed predicate.
#[trace("TC-906", "FR-098-AC-8", "FR-277-AC-4")]
#[test]
fn invalid_input_keeps_its_refusal_and_gains_the_phase_context() {
    let unit = unit();
    let stop = supplied_stop(run(unit.request(
        "seq",
        vec![WitnessValue::Sequence(vec![WitnessValue::Integer(1)])],
    )));
    assert_eq!(
        stop.cause,
        SuppliedCause::Input {
            refusal: quire_semantic_value::call::InputRefusal::WrongValueKind { parameter: 0 },
            membership: None,
        }
    );
    assert_eq!(stop.context.phase, SuppliedPhase::Conversion);
    assert_eq!(stop.context.parameter, Some(0));
    let refusal = ReplayRefusal::Supplied(Box::new(stop));
    assert_eq!(
        refusal.input_refusal(),
        Some(&quire_semantic_value::call::InputRefusal::WrongValueKind { parameter: 0 })
    );
    assert_eq!(refusal.code(), Code::InvalidRuntimeInput);
}

/// A Decimal parameter replays through the bounded Decimal caller, and a
/// starved B stops it on the named Decimal operand as a pre-call result.
#[trace("TC-906", "FR-277-AC-7")]
#[test]
fn a_decimal_parameter_replays_and_is_bounded_by_b() {
    let unit = unit();
    let money = |limits: SuppliedLimits| {
        unit.with(
            "money",
            vec![WitnessValue::Decimal {
                coefficient: Integer::from(150_i64),
                scale: Integer::from(2_i64),
            }],
            limits,
        )
    };
    assert_reproduced(&run(money(SuppliedLimits::default())));
    let starved = run(money(
        SuppliedLimits::default().with_numeric_magnitude_bits(1),
    ));
    let limit = pre_call(&starved).limit().expect("a limit");
    assert_eq!(limit.limit_kind, SuppliedLimitKind::NumericMagnitudeBits);
    assert_eq!(limit.limit, 1);
    assert!(limit.operand.is_some());
}

/// FR-277: at their published defaults both ceilings admit a large supplied
/// argument, and at that size A and H are still exactly the executed event
/// sequences: the exact ceiling admits and one less denies the last event.
/// This is a flat sequence; TC-830's Tree is
/// `the_full_size_supplied_tree_is_bounded_by_occurrences_and_admission_events`.
#[trace("TC-906", "FR-098-AC-12", "FR-277-AC-4")]
#[test]
fn a_large_flat_argument_is_admitted_by_the_defaults_and_counted_exactly() {
    let unit = unit();
    let count = 20_000;
    let (h, a) = (
        total(&conversion_events(count)),
        total(&admission_events(count)),
    );
    assert_reproduced(&run(unit.request("big", vec![booleans(count)])));

    let exact = SuppliedLimits::default()
        .with_conversion_work_units(h)
        .with_admission_work_units(a);
    assert_reproduced(&run(unit.with("big", vec![booleans(count)], exact)));

    let h_short = run(unit.with(
        "big",
        vec![booleans(count)],
        exact.with_conversion_work_units(h - 1),
    ));
    let stop = pre_call(&h_short);
    assert_eq!(stop.context.phase, SuppliedPhase::Conversion);
    assert_eq!(stop.context.consumed, h - 1);
    assert_eq!(stop.context.event, conversion_events(count).last().copied());

    let a_short = run(unit.with(
        "big",
        vec![booleans(count)],
        exact.with_admission_work_units(a - 1),
    ));
    let stop = pre_call(&a_short);
    assert_eq!(stop.context.phase, SuppliedPhase::Admission);
    assert_eq!(stop.context.consumed, a - 1);
    assert_eq!(stop.context.event, admission_events(count).last().copied());
}

/// TC-830 step 4's unit: QSL-503's `Tree { Leaf(Integer), Branch(Integer,
/// Option<Tree>) }`, declared in source and supplied to a function.
fn tree_unit() -> Unit {
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n{PROFILE}\
         union Tree {{ Leaf(Integer), Branch(Integer, Option<Tree>) }}\n\
         function size using v(t: Tree): Boolean pure {{ false }}\n"
    );
    let compiled = spine(&source, &BTreeMap::new());
    Unit { source, compiled }
}

/// The `Tree` of `depth` levels QSL-503's fixture builds (`depth - 1`
/// branches, each holding 1 and the present next level, over a leaf holding
/// 1), as the witness spelling of the unit's own union.
fn tree_witness(unit: &Unit, depth: usize) -> WitnessValue {
    let types = unit.compiled.package.graph().scope().types();
    let handle = types
        .composites()
        .find(|declaration| declaration.name() == "Tree")
        .expect("Tree is declared")
        .key();
    let key = types.union_key(handle).expect("check sealed the union");
    let declaration = WireNodeId::from_digest(*key.as_bytes());
    let member = |name: &str, components| WitnessValue::Union {
        declaration,
        member: name.to_owned(),
        components,
    };
    let one = || WitnessValue::Integer(1);
    let mut tree = member("Leaf", vec![one()]);
    for _ in 1..depth {
        tree = member(
            "Branch",
            vec![one(), WitnessValue::Option(Some(Box::new(tree)))],
        );
    }
    tree
}

/// The QSV membership events A spends admitting the 10,000-level `Tree`, in
/// order, enumerated by hand from QSV FR-109. The worklist is last-in
/// first-out and children are scheduled last to first, so each `Branch` level
/// is: the union visit, its descriptor query, the scheduling of the option
/// and then of the integer, the integer's visit, the option's visit, the
/// option's one type-link comparison (its retained payload type against the
/// declared one), and the scheduling of the present next level. The `Leaf`
/// is the union visit, its descriptor query, the scheduling of its one
/// integer and that integer's visit. An unbounded `Integer` runs no numeric
/// helper, so it adds nothing to a visit.
fn tree_admission_events(depth: usize) -> Vec<SuppliedEvent> {
    use SuppliedEvent::{ChildScheduling, DescriptorQuery, TypeComparison, ValueVisit};
    let mut events = Vec::new();
    for _ in 1..depth {
        events.extend([
            ValueVisit,
            DescriptorQuery,
            ChildScheduling,
            ChildScheduling,
            ValueVisit,
            ValueVisit,
            TypeComparison,
            ChildScheduling,
        ]);
    }
    events.extend([ValueVisit, DescriptorQuery, ChildScheduling, ValueVisit]);
    events
}

/// TC-830 step 4 through the public `replay`, with the unchanged shape of
/// QSL-503's full-size fixture (10,000 levels, 29,999 semantic occurrences):
/// the occurrence bound 29,998 denies before evaluation and 29,999 admits,
/// and with the evaluation limits identical and sufficient the admission
/// ceiling A-1 denies its actual next event unspent while A admits, where A
/// is the hand-enumerated event sequence above.
#[trace("TC-830")]
#[test]
fn the_full_size_supplied_tree_is_bounded_by_occurrences_and_admission_events() {
    const DEPTH: usize = 10_000;
    let unit = tree_unit();
    let occurrences = 29_999_u64;

    // Hand arithmetic, independent of the enumerator: 9,999 branch levels of
    // 8 events and the leaf's 4.
    let events = tree_admission_events(DEPTH);
    let a = total(&events);
    assert_eq!(a, 9_999 * 8 + 4);
    assert_eq!(a, 79_996);

    let request = |limits: SuppliedLimits, bound: u64| {
        let mut wire = unit.with("size", vec![tree_witness(&unit, DEPTH)], limits);
        wire.accounting_limits.value_occurrences = bound;
        wire
    };

    // The occurrence bound: one below denies before evaluation, naming the
    // limit and the count reached; the count itself admits.
    let over_bound = run(request(SuppliedLimits::default(), occurrences - 1));
    let stop = pre_call(&over_bound);
    assert_eq!(stop.context.phase, SuppliedPhase::Conversion);
    assert_eq!(stop.context.parameter, Some(0));
    assert_eq!(stop.context.event, Some(SuppliedEvent::ValueOccurrence));
    let limit = stop.limit().expect("a limit");
    assert_eq!(limit.limit_kind, SuppliedLimitKind::ValueOccurrences);
    assert_eq!(limit.limit, occurrences - 1);
    assert_eq!(limit.next_charge, occurrences);
    assert_reproduced(&run(request(SuppliedLimits::default(), occurrences)));

    // The admission ceiling, under the same sufficient evaluation limits.
    assert_reproduced(&run(request(only_a(a), occurrences)));
    let last_level = 8 * (DEPTH / 2);
    let sampled = (0..=8)
        .chain(last_level..last_level + 8)
        .chain(events.len() - 4..events.len());
    for denied in sampled.map(|index| u64::try_from(index).unwrap()) {
        let result = run(request(only_a(denied), occurrences));
        let stop = pre_call(&result);
        let index = usize::try_from(denied).unwrap();
        assert_eq!(stop.context.phase, SuppliedPhase::Admission, "{denied}");
        assert_eq!(stop.context.parameter, Some(0));
        assert_eq!(stop.context.event, Some(events[index]), "{denied}");
        assert_eq!(stop.context.consumed, denied, "the denied step is unspent");
        let limit = stop.limit().expect("a limit");
        assert_eq!(limit.limit_kind, SuppliedLimitKind::AdmissionWorkUnits);
        assert_eq!(limit.field(), "supplied.admission_work_units");
        assert_eq!(limit.limit, denied);
        assert_eq!(limit.next_charge, 1);
        assert_eq!(limit.counter, AttemptedCount::Count(denied + 1));
    }
}

fn exact(bits: u64) -> WitnessValue {
    // 2^(bits-1) has exactly `bits` bits.
    WitnessValue::ExactInteger(Integer::one().shifted_left(bits - 1))
}

/// QSpec FR-323 numeric magnitude bound (TC-278 RE-23, RE-24; TC-906 step
/// 10): at the default B of 4096 a 4096-bit operand passes the bound (the
/// declared domain then refuses it by kind), and a 4097-bit operand stops
/// on operand `value` with the `limit` cause for
/// `supplied.numeric_magnitude_bits`, at event value-visit, as a pre-call
/// result with zero charges and no kernel `Incomplete`.
#[trace("TC-906", "FR-098-AC-12", "FR-277-AC-7")]
#[test]
fn the_default_magnitude_bound_admits_4096_bits_and_stops_4097_on_the_value() {
    let unit = unit();
    let at_bound = run(unit.request("digit", vec![exact(4096)]));
    let refusal = at_bound.expect_err("outside the domain, not over B");
    assert_eq!(
        refusal.input_refusal(),
        Some(&quire_semantic_value::call::InputRefusal::WrongValueKind { parameter: 0 })
    );

    let over = run(unit.request("digit", vec![exact(4097)]));
    let stop = pre_call(&over);
    let limit = stop.limit().expect("a limit cause");
    assert_eq!(limit.limit_kind, SuppliedLimitKind::NumericMagnitudeBits);
    assert_eq!(limit.field(), "supplied.numeric_magnitude_bits");
    assert_eq!(limit.limit, 4096);
    assert_eq!(limit.next_charge, 4097);
    assert_eq!(limit.counter, AttemptedCount::Count(4097));
    assert_eq!(limit.operand.map(|operand| operand.as_str()), Some("value"));
    assert_eq!(stop.context.event, Some(SuppliedEvent::ValueVisit));
    assert_eq!(stop.context.consumed, 0);
    assert_eq!(stop.context.parameter, Some(0));
}

/// RE-23, RE-24: a stated B overrides the default exactly, 0 included: a
/// 2-bit value with endpoints 0 and 9 passes B = 4, stops on `upper` at
/// B = 3 and on `value` at B = 1; B = 0 admits no operand, and zero is one
/// bit.
#[trace("TC-906", "FR-098-AC-12", "FR-277-AC-7")]
#[test]
fn a_stated_magnitude_bound_overrides_the_default_exactly() {
    let unit = unit();
    let with = |bits| {
        unit.with(
            "digit",
            vec![WitnessValue::Integer(3)],
            SuppliedLimits::default().with_numeric_magnitude_bits(bits),
        )
    };
    // The interval's endpoint 9 has 4 bits, the value 3 has 2; every
    // operand is checked in order value, lower, upper.
    assert_reproduced(&run(with(4)));
    let three = run(with(3));
    let limit = pre_call(&three).limit().expect("a limit");
    assert_eq!(limit.limit, 3);
    assert_eq!(limit.next_charge, 4);
    assert_eq!(limit.operand.map(|operand| operand.as_str()), Some("upper"));
    let one = run(with(1));
    let limit = pre_call(&one).limit().expect("a limit");
    assert_eq!(limit.limit, 1);
    assert_eq!(limit.next_charge, 2);
    assert_eq!(limit.operand.map(|operand| operand.as_str()), Some("value"));

    // Zero counts as one bit: B = 1 admits it and B = 0 admits no operand.
    let zero = |bits| {
        unit.with(
            "digit",
            vec![WitnessValue::Integer(0)],
            SuppliedLimits::default().with_numeric_magnitude_bits(bits),
        )
    };
    let upper = run(zero(1));
    let limit = pre_call(&upper).limit().expect("a limit");
    assert_eq!(limit.next_charge, 4, "zero passes, the endpoint 9 does not");
    assert_eq!(limit.operand.map(|operand| operand.as_str()), Some("upper"));
    let none = run(zero(0));
    let limit = pre_call(&none).limit().expect("a limit");
    assert_eq!(limit.next_charge, 1);
    assert_eq!(limit.operand.map(|operand| operand.as_str()), Some("value"));
}

/// RE-26: the three bounds are independent. A starved B leaves A and H
/// unspent in the record; a starved A or H is never reported as B.
#[trace("TC-906", "FR-277-AC-7")]
#[test]
fn a_h_and_b_are_accounted_independently() {
    let unit = unit();
    let digit = |limits| unit.with("digit", vec![WitnessValue::Integer(3)], limits);
    let b = run(digit(
        SuppliedLimits::default().with_numeric_magnitude_bits(1),
    ));
    assert_eq!(
        pre_call(&b).limit().map(|l| l.limit_kind),
        Some(SuppliedLimitKind::NumericMagnitudeBits)
    );
    let a = run(digit(only_a(0)));
    assert_eq!(
        pre_call(&a).limit().map(|l| l.limit_kind),
        Some(SuppliedLimitKind::AdmissionWorkUnits)
    );
    let h = run(digit(only_h(0)));
    assert_eq!(
        pre_call(&h).limit().map(|l| l.limit_kind),
        Some(SuppliedLimitKind::ConversionWorkUnits)
    );
    // Raising B does not change what A or H admit.
    assert_reproduced(&run(digit(
        SuppliedLimits::default()
            .with_numeric_magnitude_bits(u64::MAX)
            .with_admission_work_units(2)
            .with_conversion_work_units(5),
    )));
}
