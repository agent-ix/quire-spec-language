// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1028 E6 to E14 through the real spine: a unit-language source is
//! composed (parse, check, emit, IR's checked-package/v2 read), then `replay`
//! recompiles it and binds native `UUID` and `Timestamp` arguments.
//!
//! Every assertion compares typed fields (the evaluated value, the category,
//! the refusal's code, cause, parameter position and kernel class), never
//! message text.

use super::*;

const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    function equ using v(a: UUID, b: UUID): Boolean pure { a = b }\n\
    function neu using v(a: UUID, b: UUID): Boolean pure { a != b }\n\
    function eqt using v(a: Timestamp, b: Timestamp): Boolean pure { a = b }\n\
    function net using v(a: Timestamp, b: Timestamp): Boolean pure { a != b }\n";

const UUID_F: &str = "00112233-4455-6677-8899-aabbccddeeff";
const UUID_E: &str = "00112233-4455-6677-8899-aabbccddeefe";
const UUID_UPPER: &str = "00112233-4455-6677-8899-AABBCCDDEEFF";
const MIN: &str = "-170141183460469231731687303715884105728";
const MAX: &str = "170141183460469231731687303715884105727";

fn uuid(text: &str) -> WitnessValue {
    WitnessValue::Uuid(text.to_owned())
}

fn timestamp(text: &str) -> WitnessValue {
    WitnessValue::Timestamp(text.to_owned())
}

/// A request replaying `function(a, b)` over the unit through the real spine.
fn call(function: &str, a: WitnessValue, b: WitnessValue) -> ReplayRequestWire {
    let compiled = spine(UNIT, &BTreeMap::new());
    let assignments = [a, b]
        .into_iter()
        .enumerate()
        .map(|(index, value)| CanonicalAssignment {
            parameter: parameter(&compiled, function, index),
            value,
        })
        .collect();
    request(
        UNIT.as_bytes(),
        compiled.emitted.package_id(),
        name(&[function]),
        ReplaySource::Input(assignments),
    )
}

/// The Boolean `function(a, b)` evaluates to through `replay`.
fn evaluates(function: &str, a: WitnessValue, b: WitnessValue) -> Option<EvaluatedValue> {
    let ReplayResult::Input(result) =
        replay(call(function, a, b), ReplayLimits::default()).expect("the replay runs")
    else {
        panic!("an Input-sourced request settles on the Input arm");
    };
    let expected = match result.value() {
        Some(EvaluatedValue::Boolean(true)) => Category::Success,
        Some(EvaluatedValue::Boolean(false)) => Category::Violation,
        other => panic!("expected a Boolean, got {other:?}"),
    };
    assert_eq!(result.category(), expected);
    result.value()
}

/// TC-1028 E6, E7, E14 (UUID): a canonical UUID pair binds through `replay`
/// and `=` and `!=` compare the payloads.
#[trace("TC-1028")]
#[test]
fn tc_1028_uuid_equality_runs_through_replay() {
    for (a, b, equal) in [(UUID_F, UUID_F, true), (UUID_F, UUID_E, false)] {
        assert_eq!(
            evaluates("equ", uuid(a), uuid(b)),
            Some(EvaluatedValue::Boolean(equal)),
            "{a} = {b}"
        );
        assert_eq!(
            evaluates("neu", uuid(a), uuid(b)),
            Some(EvaluatedValue::Boolean(!equal)),
            "{a} != {b}"
        );
    }
}

/// TC-1028 E9 to E13, E14 (Timestamp): a canonical Timestamp pair, the
/// signed 128-bit extremes included, binds through `replay` and compares by
/// nanosecond count.
#[trace("TC-1028")]
#[test]
fn tc_1028_timestamp_equality_runs_through_replay() {
    for (a, b, equal) in [
        ("0", "0", true),
        ("0", "1", false),
        ("-1", "0", false),
        (MIN, MIN, true),
        (MAX, MIN, false),
    ] {
        assert_eq!(
            evaluates("eqt", timestamp(a), timestamp(b)),
            Some(EvaluatedValue::Boolean(equal)),
            "{a} = {b}"
        );
        assert_eq!(
            evaluates("net", timestamp(a), timestamp(b)),
            Some(EvaluatedValue::Boolean(!equal)),
            "{a} != {b}"
        );
    }
}

/// TC-1028 E8, FR-056-AC-35 at the wire: the uppercase second operand is
/// refused `invalid_runtime_input`/`invalid-value` at position 1, over the
/// kernel's noncanonical class, and no comparison runs.
#[trace("FR-056-AC-35", "TC-1028")]
#[test]
fn tc_1028_noncanonical_uuid_is_refused_through_replay() {
    let refusal = replay(
        call("equ", uuid(UUID_F), uuid(UUID_UPPER)),
        ReplayLimits::default(),
    )
    .expect_err("the second operand is refused");
    assert_eq!(refusal.code(), Code::InvalidRuntimeInput);
    assert_eq!(refusal.cause(), Some("invalid-value"));
    let ReplayRefusal::NativePayload { parameter, refusal } = refusal else {
        panic!("expected a native payload refusal, got {refusal:?}");
    };
    assert_eq!(parameter, 1);
    assert_eq!(
        refusal.cause,
        quire_exact::ConstructionCause::UuidNoncanonical
    );
}

/// TC-1028 Timestamp admission: a noncanonical spelling and a canonical
/// count outside the signed 128-bit domain are refused at the input, each
/// over its own kernel class.
#[trace("FR-056-AC-35", "TC-1028")]
#[test]
fn tc_1028_noncanonical_and_out_of_domain_timestamps_are_refused_through_replay() {
    use quire_exact::ConstructionCause::{TimestampNoncanonical, TimestampOutOfDomain};
    for (text, class) in [
        ("+1", TimestampNoncanonical),
        ("00", TimestampNoncanonical),
        (
            "170141183460469231731687303715884105728",
            TimestampOutOfDomain,
        ),
    ] {
        let refusal = replay(
            call("eqt", timestamp("0"), timestamp(text)),
            ReplayLimits::default(),
        )
        .expect_err("the second operand is refused");
        assert_eq!(refusal.code(), Code::InvalidRuntimeInput, "{text}");
        assert_eq!(refusal.cause(), Some("invalid-value"), "{text}");
        let ReplayRefusal::NativePayload { parameter, refusal } = refusal else {
            panic!("expected a native payload refusal, got {refusal:?}");
        };
        assert_eq!((parameter, refusal.cause), (1, class), "{text}");
    }
}
