// SPDX-License-Identifier: AGPL-3.0-or-later
//! `CheckMode` on `PackageDeclarations::check` (FR-495): `Linked` proves each
//! definedness obligation at check, `Kernel` records none and leaves it to
//! the evaluator; and the FR-093 `NodeKey` accessor. Every assertion compares typed causes and locations.

use ix_trace_rs::trace;
use qsl_forms::build_unit;
use qsl_foundation::SourceIdentity;
use qsl_semantics::check::{
    BodyTerm, CheckCause, CheckRefusal, CheckedGraph, CheckedNode, MeasureObligation, Obligation,
    PackageDeclarations,
};
use qsl_semantics::model::intake::SelectedModels;
use qsl_semantics::value::{admit_integer_division, DefinitionRole};
use quire_semantic_value::checking::{CheckMode, CheckingLimits};
use quire_semantic_value::location::ExpressionOwner as Origin;

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

fn refusals(declarations: &str, mode: CheckMode) -> Vec<CheckRefusal> {
    check(declarations, mode).expect_err("the unit is refused")
}

/// The one refusal of `declarations` under `mode`.
fn refusal(declarations: &str, mode: CheckMode) -> CheckRefusal {
    let mut all = refusals(declarations, mode);
    assert_eq!(all.len(), 1, "{declarations}: {all:?}");
    all.remove(0)
}

/// `declarations` is refused with `obligation` under `Linked`, and checks
/// under `Kernel`, which leaves the obligation to the evaluator.
fn kernel_drops(declarations: &str, obligation: impl Fn(&Obligation) -> bool) {
    match refusal(declarations, CheckMode::Linked).cause {
        CheckCause::Unproved(found) => assert!(obligation(&found), "{declarations}: {found:?}"),
        other => panic!("{declarations}: not an obligation: {other:?}"),
    }
    check(declarations, CheckMode::Kernel).expect("Kernel records no obligation");
}

/// FR-562-AC-6 (checker half): a result bound the addition's interval does
/// not prove is `unproved-range` under `Linked` and a run-time obligation
/// under `Kernel`; a proved bound checks under both.
#[test]
fn an_unproved_result_bound_checks_only_under_kernel() {
    kernel_drops(
        "function s using v(a: Int[0, 9], b: Int[0, 9]): Int[0, 9] pure { a + b }",
        |found| matches!(found, Obligation::Range { .. }),
    );
    let source = "function s7 using v(a: Int[0, 4], b: Int[0, 5]): Int[0, 9] pure { a + b }";
    check(source, CheckMode::Linked).expect("the interval [0, 9] proves the bound");
    check(source, CheckMode::Kernel).expect("Kernel checks it too");
}

/// FR-495-AC-3 (checker half): a decimal division by a literal zero is
/// `unproved-nonzero` under `Linked` and checks under `Kernel`.
#[test]
fn an_unproved_nonzero_divisor_checks_only_under_kernel() {
    kernel_drops(
        "type P = Decimal[-1000, 1000; 0, 2; nearest-even];\n\
         function d4 using v(): P pure { decimal(1, 0) / decimal(0, 3) }",
        |found| matches!(found, Obligation::Nonzero),
    );
}

/// `value(x)` with no proved `present(x)` is `unproved-presence` under
/// `Linked`; `Kernel` leaves it to the evaluator.
#[test]
fn an_unproved_presence_checks_only_under_kernel() {
    kernel_drops(
        "function p using v(x: Option<Integer>): Integer pure { value(x) }",
        |found| matches!(found, Obligation::Presence),
    );
}

/// FR-104 "Definedness facts": `Kernel` reaches function bodies and
/// measures only, so a state clause still refuses its unproved obligation.
#[test]
fn a_state_clause_still_refuses_its_obligation_under_kernel() {
    let clause = "invariant NoGuard using v on Config::ConfigVersion at current { \
                  deref(value(self.parent)).versionNumber < self.versionNumber }\n";
    for mode in [CheckMode::Linked, CheckMode::Kernel] {
        let declarations = crate::model_operations::admit_and_assemble_with_body(
            &crate::model_operations::tc465_document(),
            clause,
        )
        .unwrap_or_else(|refusal| panic!("assembly succeeds: {refusal:?}"));
        let refusals = declarations
            .check(mode, CheckingLimits::default())
            .expect_err("the clause is refused");
        assert!(
            matches!(
                refusals.as_slice(),
                [CheckRefusal {
                    cause: CheckCause::Unproved(Obligation::Presence),
                    ..
                }]
            ),
            "{mode:?}: {refusals:?}"
        );
    }
}

/// `Kernel` drops obligations only. A recursion with no `decreases` measure
/// is refused `missing-measure` at its call in both modes, and a body whose
/// result bound `Linked` refuses first still reaches that refusal under
/// `Kernel`.
#[test]
fn kernel_mode_still_refuses_a_nonterminating_recursion() {
    let missing = |refusal: &CheckRefusal, path: &[usize]| {
        assert_eq!(
            refusal.cause,
            CheckCause::UnprovedDecrease {
                cycle: vec!["f".to_owned(), "f".to_owned()],
                obligation: MeasureObligation::MissingMeasure,
            }
        );
        assert!(
            matches!(&refusal.location.origin, Origin::Body { function, .. } if function == "f"),
            "{:?}",
            refusal.location
        );
        assert_eq!(refusal.location.path(), path);
    };
    let source = "function f using v(x: Int[0, 9]): Int[0, 9] pure { f(x) }";
    for mode in [CheckMode::Linked, CheckMode::Kernel] {
        missing(&refusal(source, mode), &[]);
    }
    let source = "function f using v(x: Int[0, 9]): Int[0, 9] pure { f(x) + 1 }";
    assert!(matches!(
        refusal(source, CheckMode::Linked).cause,
        CheckCause::Unproved(Obligation::Range { .. })
    ));
    missing(&refusal(source, CheckMode::Kernel), &[0]);
}

/// The identity of the application `graph` keys at `node`'s location.
fn application(graph: &CheckedGraph, node: CheckedNode<'_>) -> (quire_exact::NodeKey, String) {
    let key = graph
        .operation_key(node.location())
        .unwrap_or_else(|| panic!("{:?} lowers to a node", node.kind()))
        .clone();
    match graph
        .semantic_graph()
        .node(key.clone())
        .expect("keyed")
        .body()
    {
        BodyTerm::Application(term) => (key, term.operation.identity().to_owned()),
        other => panic!("not an application: {other:?}"),
    }
}

/// FR-093-AC-62: the accessor returns the multiplication's key and, for both
/// `a + b`, one key at two locations; `a * b` has a fourth key; a local read
/// is no node.
#[trace("FR-093-AC-62", "TC-415")]
#[test]
fn a_checked_operation_exposes_its_node_key() {
    let graph = check(
        "function w using v(a: Integer, b: Integer): Integer pure { (a + b) * (a + b) }\n\
         function m using v(a: Integer, b: Integer): Integer pure { a * b }",
        CheckMode::Linked,
    )
    .expect("the functions check");
    let root = graph.function_state(0).expect("w is declared").body;
    let [left, right] = root.children()[..] else {
        panic!("a product has two operands");
    };
    assert_ne!(left.location(), right.location());
    let (product, identity) = application(&graph, root);
    assert_eq!(identity, "quire.op.integer.mul");
    let (sum, identity) = application(&graph, left);
    assert_eq!(identity, "quire.op.integer.add");
    assert_eq!(application(&graph, right).0, sum);
    let other = graph.function_state(1).expect("m is declared").body;
    let (other, identity) = application(&graph, other);
    assert_eq!(identity, "quire.op.integer.mul");
    assert!(other != product && other != sum);
    for read in left.children() {
        assert_eq!(graph.operation_key(read.location()), None);
    }
}

/// FR-560 `Operation(k)`: the key of a `call` is its application node's,
/// including a call that lowers through a draft because its callee is in the
/// same recursion group.
#[test]
fn a_call_exposes_the_key_of_its_application_node() {
    let call_key = |source: &str, caller: usize| {
        let graph = check(source, CheckMode::Linked).expect("the functions check");
        let mut pending = vec![graph.function_state(caller).expect("declared").body];
        let call = loop {
            let node = pending.pop().expect("the body holds a call");
            if graph.operation_key(node.location()).is_some()
                && application(&graph, node).1 == "quire.op.function.call"
            {
                break node;
            }
            pending.extend(node.children());
        };
        let (key, identity) = application(&graph, call);
        assert_eq!(identity, "quire.op.function.call");
        key
    };
    call_key(
        "function late using v(a: Int[0, 9]): Int[0, 9] pure { early(a) }\n\
         function early using v(a: Int[0, 9]): Int[0, 9] pure { a }",
        0,
    );
    call_key(
        "function walk using v(n: Int[0, 9]): Int[0, 9] pure decreases(n) \
         { if n > 0 then walk(n - 1) else 0 }",
        0,
    );
}
