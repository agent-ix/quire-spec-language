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
use quire_exact::{IeeeValue, Meter, Outcome, ScalarLimits, Value};

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

/// binary64 `1.0` and `1.5 * 2^-53` (three quarters of an ulp of `1.0`).
const ONE: u64 = 0x3ff0_0000_0000_0000;
const THREE_QUARTER_ULP: u64 = 0x3ca8_0000_0000_0000;

/// Compiles `function add using v(f: T, g: T): T pure { f + g }` for the
/// float type spelling `float`, and returns the outcome of `1.0 + 0.75 ulp`.
fn add(float: &str) -> Outcome<Value> {
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
                Value::Float(IeeeValue::binary64(ONE)),
                Value::Float(IeeeValue::binary64(THREE_QUARTER_ULP)),
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

#[trace("FR-091-OQ-4", "FR-091-AC-19", "FR-148-AC-8", "TC-405")]
#[test]
fn the_evaluator_applies_the_rounding_mode_the_float_type_carries() {
    // The exact sum lies three quarters of the way to the next binary64.
    assert_eq!(bits(add("Float64[nearest-even]")), ONE + 1);
    assert_eq!(bits(add("Float64[toward-zero]")), ONE);
    assert_eq!(bits(add("Float64[toward-positive]")), ONE + 1);
    assert_eq!(bits(add("Float64[toward-negative]")), ONE);
    assert_eq!(bits(add("Float64[nearest-away]")), ONE + 1);
    // A bare `Float64` is strict `exact`: an inexact sum has no bits.
    assert!(
        !matches!(add("Float64"), Outcome::Completed(_)),
        "strict exact must not round"
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
