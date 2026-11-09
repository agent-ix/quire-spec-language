// SPDX-License-Identifier: AGPL-3.0-or-later
//! `CheckMode` on `PackageDeclarations::check` (FR-495): `Linked` proves each
//! range and nonzero obligation at check, `Kernel` records none and leaves it
//! to the evaluator. Every assertion compares typed causes and locations.

use qsl_forms::build_unit;
use qsl_foundation::SourceIdentity;
use qsl_semantics::check::{
    BodyTerm, CheckCause, CheckRefusal, CheckedGraph, CheckedNode, Obligation, PackageDeclarations,
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

/// FR-093: a checked operation exposes the `NodeKey` of the FR-322 node it
/// lowered to, by its location; structurally equal operations share one key
/// and their locations tell the sites apart; a local read is no node.
#[test]
fn a_checked_operation_exposes_its_node_key() {
    let graph = check(
        "function f using v(a: Int[0, 9], b: Int[0, 9]): Boolean pure { a + b = a + b }",
        CheckMode::Linked,
    )
    .expect("the function checks");
    let root = graph.function_state(0).expect("f is declared").body;
    let application = |node: CheckedNode<'_>| {
        let key = graph
            .operation_key(node.location())
            .unwrap_or_else(|| panic!("{:?} lowers to a node", node.kind()));
        match graph
            .semantic_graph()
            .node(key.clone())
            .expect("keyed")
            .body()
        {
            BodyTerm::Application(term) => term.operation.identity().to_owned(),
            other => panic!("not an application: {other:?}"),
        }
    };
    let [left, right] = root.children()[..] else {
        panic!("an equality has two operands");
    };
    assert_ne!(left.location(), right.location());
    assert_eq!(
        graph.operation_key(left.location()),
        graph.operation_key(right.location())
    );
    assert_ne!(
        graph.operation_key(left.location()),
        graph.operation_key(root.location())
    );
    assert_eq!(application(left), "quire.op.integer.add");
    assert_eq!(application(root), "quire.op.integer.eq");
    for read in left.children() {
        assert_eq!(graph.operation_key(read.location()), None);
    }
}
