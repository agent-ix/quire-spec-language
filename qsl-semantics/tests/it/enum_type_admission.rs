// SPDX-License-Identifier: AGPL-3.0-or-later
//! The checker owns admission of authored enum type names against its package scope.

use ix_trace_rs::trace;
use qsl_forms::{BuiltinType, Expression, FunctionDeclaration, TypeForm};
use qsl_foundation::{Code, IdentityLimits, Span};
use qsl_semantics::check::{fixture_source, CheckCause, PackageDeclarations};
use quire_semantic_value::checking::CheckingLimits;

#[test]
#[trace("TC-481", "FR-091-AC-28")]
fn an_absent_enum_type_name_is_a_typed_missing_name() {
    let span = Span { start: 0, end: 0 };
    let function = FunctionDeclaration::new(
        "reads_status",
        vec![("status".to_owned(), TypeForm::name("AbsentStatus", span))],
        TypeForm::builtin(BuiltinType::Boolean, span),
        None,
        Expression::boolean(true),
    );
    let declarations = PackageDeclarations {
        functions: vec![function],
        ..PackageDeclarations::new(
            fixture_source(),
            crate::TEST_PACKAGE_NAME,
            IdentityLimits::default(),
        )
    };

    let refusals = declarations
        .check(CheckingLimits::default())
        .expect_err("an undeclared enum type name must be refused");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert!(matches!(
        &refusals[0].cause,
        CheckCause::MissingName(name) if name == "AbsentStatus"
    ));
    assert_eq!(refusals[0].cause.code(), Code::MissingDeclaration);
}
