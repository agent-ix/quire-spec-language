// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-725 step 2 (FR-258-AC-5): every body lowering writes for the TC-415
//! nested forms is in the stratified v2 body grammar. The walk over the
//! 100,000-deep packages of step 1 runs with step 1, where the package is
//! emitted (`qsl-package`'s `emit::tests::deep_bodies`).

use super::expression_depth::{check_on_small_stack as check_form_on_small_stack, Form, FORMS};

/// The TC-415 nestings each form is lowered at.
const LEVELS: [usize; 2] = [2, 1_000];
use super::*;

/// Whether `term` is a Leaf: a literal, a reference, a dependency
/// reference, or the in-group reference a recursion group member's preimage
/// names a fellow member by, holding no other term.
fn is_leaf(term: &Json) -> bool {
    matches!(
        term["term"].as_str(),
        Some("literal" | "reference" | "dependency_reference" | "group_reference")
    )
}

/// Whether `term` is a named binding whose value `value` admits.
fn is_binding(term: &Json, value: fn(&Json) -> bool) -> bool {
    term["term"] == "binding" && term["name"].is_string() && value(&term["value"])
}

/// Whether `term` is a Group: an aggregate of Leaves and bindings of Leaves.
fn is_group(term: &Json) -> bool {
    term["term"] == "aggregate"
        && term["members"].as_array().is_some_and(|members| {
            members
                .iter()
                .all(|member| is_leaf(member) || is_binding(member, is_leaf))
        })
}

/// Whether `term` is a Member binding's value: a Leaf, a Group, or the
/// binding of a Leaf FR-092 gives an optional record field and FR-093 a
/// fold's step.
fn is_binding_value(term: &Json) -> bool {
    is_leaf(term) || is_group(term) || is_binding(term, is_leaf)
}

/// Whether `term` is a Member: a Leaf, a Group or a binding.
fn is_member(term: &Json) -> bool {
    is_leaf(term) || is_group(term) || is_binding(term, is_binding_value)
}

/// Whether `body` is a Body: a Leaf, an application or aggregate whose
/// arguments or members are Members, or a frame. Each stratum admits only
/// lower ones, so a composite subterm can only be a `reference` to its own
/// node.
fn is_body(body: &Json) -> bool {
    let all_members = |key: &str| {
        body[key]
            .as_array()
            .is_some_and(|terms| terms.iter().all(is_member))
    };
    match body["term"].as_str() {
        Some("application") => all_members("arguments"),
        Some("aggregate") => all_members("members"),
        Some("frame") => true,
        _ => is_leaf(body),
    }
}

/// Every node body of `graph` that is not in the stratified v2 body
/// grammar.
fn unstratified_bodies(graph: &CheckedGraph) -> Vec<Json> {
    graph
        .semantic_graph()
        .nodes()
        .map(|node| preimage(node)["body"].clone())
        .filter(|body| !is_body(body))
        .collect()
}

/// Every node body lowering writes for every TC-415 nested form, at 2 and
/// at 1,000 levels, is in the stratified grammar.
#[trace("TC-725", "FR-258-AC-5")]
#[test]
fn every_lowered_body_is_in_the_stratified_grammar() {
    for form in FORMS {
        // An unguarded `value(….next)` refuses on its unproved presence, so
        // it lowers no body.
        if matches!(form, Form::Field) {
            continue;
        }
        for levels in LEVELS {
            let graph = check_form_on_small_stack(form, levels, CheckingLimits::default())
                .unwrap_or_else(|refusals| panic!("{form:?} at {levels} checks: {refusals:?}"));
            assert_eq!(
                unstratified_bodies(&graph),
                Vec::<Json>::new(),
                "{form:?} at {levels}"
            );
        }
    }
}

/// The stratum walk refuses an application nested in an application, an
/// aggregate member holding an aggregate of aggregates, and a binding of a
/// binding of a Group, and admits the reference that names such a node.
#[test]
fn the_stratum_walk_refuses_a_nested_composite() {
    let reference = json!({"term": "reference", "target": {}});
    let application = json!({"term": "application", "arguments": [reference]});
    assert!(is_body(&application));
    assert!(!is_body(
        &json!({"term": "application", "arguments": [application]})
    ));
    let group = json!({"term": "aggregate", "members": [reference]});
    assert!(is_body(&json!({"term": "aggregate", "members": [group]})));
    assert!(!is_body(&json!({
        "term": "aggregate",
        "members": [{"term": "aggregate", "members": [group]}],
    })));
    let bound_group = json!({"term": "binding", "name": "g", "value": group});
    assert!(!is_body(&json!({
        "term": "application",
        "arguments": [{"term": "binding", "name": "f", "value": bound_group}],
    })));
}
