// SPDX-License-Identifier: AGPL-3.0-or-later
//! A client outside the crate names the O-16 `Category` through
//! `qsl_replay`'s root alone. As an integration test it reaches only the
//! public API, so a re-export that is not `pub` fails to compile here.

use ix_trace_rs::trace;
use qsl_replay::{
    Category, EvaluatedValue, InputArmResult, InputSettlement, ScalarLimits, Verdict, WitnessCheck,
};

fn charges() -> ScalarLimits {
    ScalarLimits {
        integer_bits: 64,
        decimal_digits: 34,
        scale_expansion: 8,
        text_input_bytes: 1024,
        text_scalars: 1024,
        normalized_scalars: 1024,
        unit_edges: 4,
        value_occurrences: 16,
        work_units: 100,
        result_units: 10,
    }
}

/// FR-072-AC-2: `Success` and `Violation` named through the facade build
/// verdicts that round-trip their category, differ from each other, and an
/// arm settled with the facade's `Category::Success` reports it back.
#[trace("TC-190", "FR-072-AC-2")]
#[test]
fn category_success_and_violation_are_usable_through_the_facade() {
    let success = Verdict::from_category(Category::Success);
    let violation = Verdict::from_category(Category::Violation);
    assert_eq!(success.category(), Category::Success);
    assert_eq!(violation.category(), Category::Violation);
    assert_ne!(success, violation);

    let agreed = InputArmResult::settle(
        success,
        success,
        Category::Success,
        Some(EvaluatedValue::Boolean(true)),
        &WitnessCheck::Agrees(None),
        Vec::new(),
        charges(),
    );
    assert_eq!(agreed.category(), Category::Success);
    assert_eq!(
        agreed.settlement(),
        InputSettlement::ReproducedWithoutWitness
    );

    let disagreed = InputArmResult::settle(
        success,
        violation,
        Category::Violation,
        Some(EvaluatedValue::Boolean(false)),
        &WitnessCheck::Agrees(None),
        Vec::new(),
        charges(),
    );
    assert_eq!(disagreed.category(), Category::Violation);
    assert_eq!(disagreed.settlement(), InputSettlement::Inconclusive);
}
