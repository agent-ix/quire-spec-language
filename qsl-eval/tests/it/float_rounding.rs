// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-280 (FR-091-OQ-4, FR-148): a `Float64[mode]` type carries its rounding
//! mode and the evaluator rounds `+` by it.

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckingLimits, PackageDeclarations};
use qsl_semantics::family::FamilyOutcome;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::value::{CatalogRole, DefinitionLock, DefinitionReference, DefinitionRevision};
use quire_exact::{IeeeFlag, IeeeValue, Meter, Outcome, Refusal, ScalarLimits, Value};

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

/// binary64 `1.0`, `1.5 * 2^-53` (three quarters of an ulp of `1.0`) and
/// `2^-53` (exactly half an ulp: a tie).
const ONE: u64 = 0x3ff0_0000_0000_0000;
const THREE_QUARTER_ULP: u64 = 0x3ca8_0000_0000_0000;
const HALF_ULP: u64 = 0x3ca0_0000_0000_0000;
const NEG_ONE: u64 = 0xbff0_0000_0000_0000;
const NEG_THREE_QUARTER_ULP: u64 = 0xbca8_0000_0000_0000;

/// Compiles `function add using v(f: T, g: T): T pure { f + g }` for the
/// float type spelling `float`, and returns the outcome of `left + right`.
fn add(float: &str, left: u64, right: u64) -> Outcome<Value> {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\" version \"1\" digest \
         \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n\
         function add using v(f: {float}, g: {float}): {float} pure {{ f + g }}\n"
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed, qsl_forms::FormsLimits::default())
        .expect("S2 builds the unit");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the assembler admits the floating type");
    declarations.ieee_profile = Some(ieee_profile());
    let package = CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .expect("the package checks"),
    );
    let mut meter = Meter::new(UNLIMITED);
    let evaluation = package
        .call(
            &QualifiedName::unqualified("add").expect("add is an identifier"),
            vec![
                Value::Float(IeeeValue::binary64(left)),
                Value::Float(IeeeValue::binary64(right)),
            ],
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .expect("the call runs");
    let FamilyOutcome::Evaluated(outcome) = evaluation.outcome else {
        panic!("a kernel outcome, not {:?}", evaluation.outcome);
    };
    outcome
}

fn bits(outcome: Outcome<Value>) -> u64 {
    match outcome {
        Outcome::Completed(Value::Float(float)) => float.bits(),
        other => panic!("a completed float, not {other:?}"),
    }
}

#[trace("FR-091-AC-19", "FR-148-AC-8", "TC-405")]
#[test]
fn the_evaluator_applies_the_rounding_mode_the_float_type_carries() {
    let sum = |mode: &str, left, right| bits(add(&format!("Float64[{mode}]"), left, right));
    // 0.75 ulp above one: nearest and toward-positive round up.
    for (mode, expected) in [
        ("nearest-even", ONE + 1),
        ("nearest-away", ONE + 1),
        ("toward-positive", ONE + 1),
        ("toward-zero", ONE),
        ("toward-negative", ONE),
    ] {
        assert_eq!(sum(mode, ONE, THREE_QUARTER_ULP), expected, "{mode} 0.75");
    }
    // A tie: even keeps one, away goes up.
    assert_eq!(sum("nearest-even", ONE, HALF_ULP), ONE);
    assert_eq!(sum("nearest-away", ONE, HALF_ULP), ONE + 1);
    // Negative: toward-negative grows the magnitude, toward-positive and
    // toward-zero shrink it.
    for (mode, expected) in [
        ("toward-zero", NEG_ONE),
        ("toward-positive", NEG_ONE),
        ("toward-negative", NEG_ONE + 1),
    ] {
        assert_eq!(
            sum(mode, NEG_ONE, NEG_THREE_QUARTER_ULP),
            expected,
            "{mode} negative"
        );
    }
    // A bare `Float64` is strict `exact`: an inexact sum is refused.
    match add("Float64", ONE, THREE_QUARTER_ULP) {
        Outcome::Refused(Refusal::IeeeNotExact { would_be }) => {
            assert!(would_be.contains(IeeeFlag::Inexact));
        }
        other => panic!("strict exact refuses an inexact sum, not {other:?}"),
    }
}

/// QSpec FR-322:564 pins the rounding mode by operand type: operands of the
/// same width under different modes are a type mismatch.
#[trace("FR-091-AC-19", "TC-405")]
#[test]
fn operands_of_different_rounding_modes_are_a_type_mismatch() {
    let text = "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\" version \"1\" digest \
         \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n\
         function add using v(f: Float64[toward-zero], g: Float64[nearest-even]): \
         Float64[toward-zero] pure { f + g }\n";
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    let unit = qsl_forms::build_unit(&parsed, qsl_forms::FormsLimits::default())
        .expect("S2 builds the unit");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the assembler admits both types");
    declarations.ieee_profile = Some(ieee_profile());
    let refusal = declarations
        .check(CheckingLimits::default())
        .expect_err("mixed-mode operands refuse");
    assert!(
        format!("{refusal:?}").to_lowercase().contains("mismatch"),
        "a type mismatch, not {refusal:?}"
    );
}

fn ieee_profile() -> qsl_semantics::value::AdmittedIeeeProfile {
    let lock = DefinitionLock::pinned();
    let entry = lock.entry(CatalogRole::IeeeProfile).unwrap();
    lock.admit_ieee_profile(
        &[DefinitionReference {
            authority: entry.authority.to_owned(),
            identity: entry.identity.to_owned(),
            revision: DefinitionRevision {
                namespace: entry.revision_namespace.to_owned(),
                value: entry.revision_value.to_owned(),
            },
            digest_domain: "quire.definition.bytes/v1".to_owned(),
            digest: "0".repeat(64),
        }],
        &[],
    )
    .unwrap()
}
