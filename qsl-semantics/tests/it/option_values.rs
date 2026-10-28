// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-093 and FR-140: `Some(value)` is a checked present option value.

use ix_trace_rs::trace;
use qsl_forms::build_unit;
use qsl_foundation::{Code, SourceIdentity};
use qsl_semantics::check::{CheckCause, PackageDeclarations};
use qsl_semantics::model::intake::SelectedModels;
use quire_semantic_value::checking::CheckingLimits;

fn check(
    declarations: &str,
) -> Result<qsl_semantics::check::CheckedGraph, Vec<qsl_semantics::check::CheckRefusal>> {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         {declarations}\n"
    );
    let parsed = qsl_cst::parse(
        SourceIdentity::new("test", "option-values", "fixture", "fixture:1"),
        "option-values.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        SelectedModels::default(),
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("assembly succeeds: {refusal:?}"))
    .check(CheckingLimits::default())
}

/// TC-1273's S3 negative vector: Some(Integer) does not match Option<Boolean>.
#[trace("TC-1273")]
#[test]
fn some_with_a_payload_outside_the_element_type_refuses_at_the_call() {
    let error = check("function bad using v(x: Integer): Option<Boolean> pure { Some(x) }")
        .expect_err("the payload is not a Boolean");
    let [refusal] = error.as_slice() else {
        panic!("one typed refusal: {error:?}");
    };
    assert_eq!(refusal.cause.code(), Code::IllTyped);
    assert_eq!(refusal.cause.cause(), Some("type-mismatch"));
    assert!(matches!(
        &refusal.cause,
        CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch)
    ));
    assert!(
        !refusal.location.path().is_empty(),
        "the call is the failing expression: {:?}",
        refusal.location
    );
}
