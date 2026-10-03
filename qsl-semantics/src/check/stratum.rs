// SPDX-License-Identifier: AGPL-3.0-or-later
//! The oracle for QSpec's five-stratum body grammar (FR-322 "Body grammar"):
//! whether a JSON node body, or a node-identity preimage's body, is drawn
//! from it. QSpec owns the grammar; this module only checks a written body
//! against it, so a test can walk every body of a checked package.
//!
//! The strata, lowest first: a Leaf, a Group, a Tuple, a Member and a Body.
//! No production names itself or a higher stratum, so a body's JSON depth is
//! fixed whatever the expression's depth.

use serde_json::Value;

/// Whether `term` is a Leaf: a literal, a reference, a dependency reference,
/// or the in-group reference a recursion group member's preimage names a
/// fellow member by.
fn is_leaf(term: &Value) -> bool {
    matches!(
        term["term"].as_str(),
        Some("literal" | "reference" | "dependency_reference" | "group_reference")
    )
}

/// Whether `term` is a `binding` that has a name and whose value `value`
/// admits.
fn is_binding(term: &Value, value: fn(&Value) -> bool) -> bool {
    term["term"] == "binding" && term["name"].is_string() && value(&term["value"])
}

/// Whether `term` is an `aggregate` whose members each satisfy `member`.
fn is_aggregate(term: &Value, member: fn(&Value) -> bool) -> bool {
    term["term"] == "aggregate"
        && term["members"]
            .as_array()
            .is_some_and(|members| members.iter().all(member))
}

/// A Group: an `aggregate` of Leaves and bindings of a Leaf.
fn is_group(term: &Value) -> bool {
    is_aggregate(term, |member| is_leaf(member) || is_binding(member, is_leaf))
}

/// A Tuple: an `aggregate` of Leaves and Groups.
fn is_tuple(term: &Value) -> bool {
    is_aggregate(term, |member| is_leaf(member) || is_group(member))
}

/// The value of a Member's binding: a Leaf, a Group or a Tuple.
fn is_bound_value(term: &Value) -> bool {
    is_leaf(term) || is_group(term) || is_tuple(term)
}

/// A Member: a Leaf, a Group, or a `binding` of a Leaf, a Group or a Tuple.
fn is_member(term: &Value) -> bool {
    is_leaf(term) || is_group(term) || is_binding(term, is_bound_value)
}

/// Whether `body` is a Body: a Leaf; an `application` whose arguments are
/// Members; an `aggregate` whose members are Members; a `frame`; or an
/// `abstraction_relation`.
pub fn is_stratified_body(body: &Value) -> bool {
    match body["term"].as_str() {
        Some("application") => body["arguments"]
            .as_array()
            .is_some_and(|arguments| arguments.iter().all(is_member)),
        Some("aggregate") => is_aggregate(body, is_member),
        Some("frame" | "abstraction_relation") => true,
        _ => is_leaf(body),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use serde_json::json;

    /// The oracle refuses an application nested in an application, an
    /// aggregate member holding an aggregate of aggregates, and a binding of
    /// a binding, and admits the reference that names such a node.
    #[trace("TC-725", "FR-258-AC-5")]
    #[test]
    fn the_stratum_walk_refuses_a_nested_composite() {
        let reference = json!({"term": "reference", "target": {}});
        let application = json!({"term": "application", "arguments": [reference]});
        assert!(is_stratified_body(&application));
        assert!(!is_stratified_body(
            &json!({"term": "application", "arguments": [application]})
        ));
        let group = json!({"term": "aggregate", "members": [reference]});
        assert!(is_stratified_body(
            &json!({"term": "aggregate", "members": [group]})
        ));
        assert!(!is_stratified_body(&json!({
            "term": "aggregate",
            "members": [{"term": "aggregate", "members": [group]}],
        })));
        let bound_group = json!({"term": "binding", "name": "g", "value": group});
        assert!(!is_stratified_body(&json!({
            "term": "application",
            "arguments": [{"term": "binding", "name": "f", "value": bound_group}],
        })));
        // A binding of a binding of a Leaf is outside the grammar; a binding
        // of a Group, which holds a binding of a Leaf, is inside it.
        let bound_leaf = json!({"term": "binding", "name": "n", "value": reference});
        let binding_of_binding = json!({"term": "binding", "name": "o", "value": bound_leaf});
        assert!(!is_stratified_body(&json!({
            "term": "application",
            "arguments": [binding_of_binding],
        })));
        let inner_group = json!({"term": "aggregate", "members": [bound_leaf]});
        let bound_inner = json!({"term": "binding", "name": "o", "value": inner_group});
        assert!(is_stratified_body(&json!({
            "term": "application",
            "arguments": [bound_inner],
        })));
        // A binding is no body root.
        assert!(!is_stratified_body(&bound_leaf));
    }
}
