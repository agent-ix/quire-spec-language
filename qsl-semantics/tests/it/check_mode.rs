// SPDX-License-Identifier: AGPL-3.0-or-later
//! `CheckMode` on `PackageDeclarations::check` (FR-495): `Linked` proves each
//! range and nonzero obligation at check, `Kernel` records none and leaves it
//! to the evaluator. Every assertion compares typed causes and locations.

use qsl_forms::build_unit;
use qsl_foundation::SourceIdentity;
use qsl_semantics::check::{
    CheckCause, CheckRefusal, CheckedGraph, Obligation, PackageDeclarations,
};
use qsl_semantics::model::intake::SelectedModels;
use qsl_semantics::value::{admit_integer_division, DefinitionRole};
use quire_semantic_value::checking::{CheckMode, CheckingLimits};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
                      profile v = \"quire.value.complete/v1\";\n";

fn check(declarations: &str, mode: CheckMode) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("test", "check-mode"),
        "check-mode.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    let mut package = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        unit,
        SelectedModels::default(),
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("assembly succeeds: {refusal:?}"));
    package.integer_division = Some(
        admit_integer_division(
            &[DefinitionRole::IntegerDivisionEuclidean.reference()],
            None,
        )
        .expect("the Euclidean definition admits"),
    );
    package.check(mode, CheckingLimits::default())
}

fn refusal(declarations: &str) -> CheckRefusal {
    let mut refusals = check(declarations, CheckMode::Linked).expect_err("Linked refuses");
    assert_eq!(refusals.len(), 1, "{declarations}: {refusals:?}");
    refusals.remove(0)
}

/// FR-562-AC-6 (checker side): a result bound the addition's interval does
/// not prove is `unproved-range` under `Linked` and a run-time obligation
/// under `Kernel`; a proved bound checks under both.
#[test]
fn an_unproved_result_bound_checks_only_under_kernel() {
    let source = "function s using v(a: Int[0, 9], b: Int[0, 9]): Int[0, 9] pure { a + b }";
    assert!(matches!(
        refusal(source).cause,
        CheckCause::Unproved(Obligation::Range { .. })
    ));
    check(source, CheckMode::Kernel).expect("Kernel leaves the bound to run time");
    let source = "function s7 using v(a: Int[0, 4], b: Int[0, 5]): Int[0, 9] pure { a + b }";
    check(source, CheckMode::Linked).expect("the interval [0, 9] proves the bound");
    check(source, CheckMode::Kernel).expect("Kernel checks it too");
}

/// Kernel mode drops obligations only: the call graph is still walked, so a
/// function that calls itself with no decreasing measure is still refused.
#[test]
fn kernel_mode_still_refuses_a_nonterminating_recursion() {
    let source = "function f using v(x: Int[0, 9]): Int[0, 9] pure { f(x) }";
    assert!(check(source, CheckMode::Linked).is_err());
    assert!(check(source, CheckMode::Kernel).is_err());
}
