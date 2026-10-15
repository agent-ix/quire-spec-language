// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-308: the value of a protocol reference is the value of the protocol
//! clause it names, and no stage evaluates a protocol clause, so evaluating a
//! function whose body is a reference refuses, typed, with a catalog cause.

use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_foundation::diagnostic::CatalogCode;
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::family::{FamilyOutcome, FamilyResult};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_exact::{Meter, ScalarLimits};
use quire_semantic_value::checking::CheckingLimits;

const MODEL: &[u8] = include_bytes!("../../../tests/fixtures/spine-model.semantic-ir.json");

const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    profile p = \"quire.protocol.finite-global/v1\";\n\
    model M = \"acme/orders\" version \"1.0.0\" digest \
    \"sha256-jcs:876d1b2d9e3f825763f6e301cca5c6d6462f4cc0daacb501f7a68a8b3089e114\";\n\
    function refers using v(): Boolean pure { protocol Fill }\n\
    protocol Fill using v over (input: Boolean) on origin {\n\
    role R on M::Gadget;\n\
    run sequence Main { }\n\
    finish End as (outcome: Boolean) { true };\n\
    }\n";

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

#[test]
fn evaluating_a_protocol_reference_refuses_unsupported_feature() {
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        UNIT.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let packages = qsl_semantics::model::intake::package_input([MODEL]);
    let models = qsl_semantics::model::intake::admit_unit(
        &unit.selections().models,
        &packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("I1 admits the model");
    let package = CheckedPackage::link(
        PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            unit,
            models,
            Vec::new(),
        )
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect("the package checks"),
    );
    let mut meter = Meter::new(UNLIMITED);
    let evaluation = package
        .call(
            &QualifiedName::unqualified("refers").expect("refers is an identifier"),
            Vec::new(),
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .expect("the call runs");
    let FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause)) = evaluation.outcome else {
        panic!("a family refusal, not {:?}", evaluation.outcome);
    };
    assert_eq!(
        cause.catalog_code(),
        CatalogCode::new("unknown_required_feature", "unsupported-feature")
    );
}
