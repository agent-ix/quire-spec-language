// SPDX-License-Identifier: AGPL-3.0-or-later
//! The identity precondition for arbitrary nesting depth (ADR-030 D-4.3,
//! D-4.4): every identity preimage a checked body produces has a depth
//! fixed by its schema, never one that follows the source's depth. Lowering
//! places every composite subterm in its own node and names it by
//! `reference` (FR-258 behaviour 5), so a node's preimage holds its
//! operands' keys rather than their bodies.
//!
//! Each case checks the same form at a shallow and a deep nesting and
//! compares the deepest JSON nesting of any node's preimage bytes (node
//! keys, type nodes and nominal preimages alike). Were any preimage to nest
//! its operands, the deep package's deepest preimage would be deeper by
//! about one level per source level.

use super::depth::{chain, check_on_small_stack as check_records_on_small_stack};
use super::expression_depth::{check_on_small_stack, Form, FORMS};
use super::*;
use crate::check::json_depth;

/// The shallow nesting each form is checked at.
const SHALLOW: usize = 4;

/// The deep nesting each form is checked at: one every form admits at the
/// default checking limits.
const DEEP: usize = 63;

/// The deepest JSON nesting of any node preimage in `graph`.
fn deepest_preimage(graph: &CheckedGraph) -> usize {
    graph
        .semantic_graph()
        .nodes()
        .map(|node| json_depth(node.preimage()))
        .max()
        .expect("a checked package holds nodes")
}

#[test]
fn json_depth_counts_nesting_outside_strings() {
    assert_eq!(json_depth(b"true"), 0);
    assert_eq!(json_depth(br#"{"a":[1,{"b":2}],"c":{}}"#), 3);
    assert_eq!(json_depth(br#"{"a":"[[{{\"]]"}"#), 1);
}

/// Every nested expression form keys every node of its package with a
/// preimage no deeper at 63 levels than at 4.
#[trace("TC-728", "FR-259-AC-4")]
#[test]
fn every_expression_forms_identity_preimages_have_a_fixed_depth() {
    for form in FORMS {
        // An unguarded `value(….next)` refuses on its unproved presence at
        // any depth, so it keys no package.
        if matches!(form, Form::Field) {
            continue;
        }
        let checked = |levels| {
            check_on_small_stack(form, levels, CheckingLimits::default())
                .unwrap_or_else(|refusals| panic!("{form:?} at {levels} checks: {refusals:?}"))
        };
        let shallow = deepest_preimage(&checked(SHALLOW));
        let deep = deepest_preimage(&checked(DEEP));
        assert_eq!(
            shallow, deep,
            "{form:?}: the deepest preimage follows the source depth"
        );
    }
}

/// A parameter typed with nested `Option`s keys each type node with a
/// preimage no deeper at 100 levels than at 4.
#[trace("TC-728", "FR-259-AC-4")]
#[test]
fn a_nested_option_types_identity_preimages_have_a_fixed_depth() {
    let checked = |levels: usize| {
        let nested = (0..levels).fold(boolean(), |inner, _| {
            TypeForm::builtin(BuiltinType::Option, SPAN).with_arguments(vec![inner])
        });
        PackageDeclarations {
            functions: vec![function(
                "p",
                &[("x", nested)],
                boolean(),
                None,
                Expression::Boolean(true),
            )],
            ..PackageDeclarations::new(fixture_source())
        }
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{levels} nested options check: {refusals:?}"))
    };
    assert_eq!(
        deepest_preimage(&checked(SHALLOW)),
        deepest_preimage(&checked(100))
    );
}

/// A chain of records, each holding the next in an optional field, keys
/// each record with a preimage no deeper at 30 records than at 4.
#[trace("TC-728", "FR-259-AC-4")]
#[test]
fn a_record_chains_identity_preimages_have_a_fixed_depth() {
    let checked = |length| {
        check_records_on_small_stack(chain(length), Vec::new(), CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("{length} records check: {refusals:?}"))
    };
    assert_eq!(
        deepest_preimage(&checked(SHALLOW)),
        deepest_preimage(&checked(30))
    );
}
