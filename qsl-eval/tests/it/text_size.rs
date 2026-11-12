// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1409 (FR-093-AC-63): `size` over a `Text[..]` operand through S1, S2,
//! the checker and the evaluator. The lowering half is in the root crate's
//! `text_size_v2` test.

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckCause, CheckRefusal, PackageDeclarations};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::value::{admit_ieee_profile, DefinitionRole};
use quire_eval::FamilyOutcome;
use quire_exact::{
    admit_text, IllTypedCause, Integer, Meter, Outcome, ScalarLimits, TextPayload, TextProfile,
    TextType, Value,
};
use quire_semantic_value::checking::CheckingLimits;

const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

/// `declarations` as one unit, checked.
fn compile(declarations: &str) -> Result<CheckedPackage, Vec<CheckRefusal>> {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         {declarations}\n"
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the assembler builds the package declarations");
    declarations.ieee_profile =
        Some(admit_ieee_profile(&[DefinitionRole::IeeeProfile.reference()], &[]).unwrap());
    declarations
        .check(CheckingLimits::default())
        .map(CheckedPackage::link)
}

/// The one refusal `declarations` draws, as its `ill_typed` cause and the
/// depth of its location (1 is an operand of the call).
fn refused(declarations: &str) -> (IllTypedCause, usize) {
    let Err(refusals) = compile(declarations) else {
        panic!("{declarations}: expected a refusal");
    };
    let [refusal] = refusals.as_slice() else {
        panic!("{declarations}: one refusal, not {refusals:?}");
    };
    let CheckCause::IllTyped(cause) = &refusal.cause else {
        panic!("{declarations}: ill_typed, not {refusal:?}");
    };
    (*cause, refusal.location.depth())
}

/// The integer `n` returns for the payload `text`, admitted as `Text[0, 8; nfc]`.
fn size_of(package: &CheckedPackage, text: &str) -> Integer {
    let text_type = TextType::new(0, 8, TextProfile::Nfc).unwrap();
    let argument = Value::Text(
        admit_text(
            &TextPayload::from_utf8(text.as_bytes()).unwrap(),
            &text_type,
            &mut Meter::new(UNLIMITED),
        )
        .completed()
        .unwrap(),
    );
    let evaluation = package
        .call(
            &QualifiedName::unqualified("n").expect("an identifier"),
            vec![argument],
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .expect("the call runs");
    let FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(size))) = evaluation.outcome
    else {
        panic!("a completed integer, not {:?}", evaluation.outcome);
    };
    size
}

const SIZE_OF_TEXT: &str = "function n using v(t: Text[0, 8; nfc]): Int[0, 8] pure { size(t) }";

/// FR-093-AC-63 (TC-1409 step 1): the body checks as `Int[0, 8]` with no
/// coercion; a narrower declared result does not fit the text bounds.
#[trace("FR-093-AC-63", "TC-1409")]
#[test]
fn size_over_text_is_typed_by_the_text_bounds() {
    compile(SIZE_OF_TEXT).expect("Int[0, 8] holds the size of Text[0, 8; nfc]");
    assert!(compile("function n using v(t: Text[0, 8; nfc]): Int[0, 7] pure { size(t) }").is_err());
    compile("function n using v(t: Text[2, 5; nfc]): Int[2, 5] pure { size(t) }")
        .expect("the bounds are the text's own");
}

/// FR-093-AC-63 (TC-1409 step 2): `0`, `3`, and `1` for U+0065 U+0301, which
/// `nfc` retains as one scalar; a byte count or raw scalar count gives `2`.
#[trace("FR-093-AC-63", "TC-1409")]
#[test]
fn size_over_text_counts_scalars_after_profile_normalization() {
    let package = compile(SIZE_OF_TEXT).expect("checks");
    for (text, expected) in [("", 0_i64), ("abc", 3), ("e\u{301}", 1)] {
        assert_eq!(size_of(&package, text), Integer::from(expected), "{text:?}");
    }
}

/// FR-093-AC-63 (TC-1409 step 3): a `String` or `Integer` operand refuses
/// `ill_typed`/`type-mismatch` at the operand.
#[trace("FR-093-AC-63", "TC-1409")]
#[test]
fn size_over_a_string_or_integer_is_a_type_mismatch_at_the_operand() {
    for (parameter, ty) in [("s", "String"), ("i", "Int[0, 9]")] {
        assert_eq!(
            refused(&format!(
                "function f using v({parameter}: {ty}): Int[0, 9] pure {{ size({parameter}) }}"
            )),
            (IllTypedCause::TypeMismatch, 1),
            "{ty}"
        );
    }
}

/// FR-093-AC-63: a collection operand still sizes as a collection.
#[trace("FR-093-AC-63", "TC-1409")]
#[test]
fn size_over_a_collection_is_unchanged() {
    compile("function f using v(s: Sequence<Integer>[0, 9]): Integer pure { size(s) }")
        .expect("a collection size is an Integer");
}
