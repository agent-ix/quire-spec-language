//! Same-identity collisions of one owner (field, operation, parameter) refuse
//! `conflicting-binding`, like a clause, state, transition or step (QSpec
//! FR-154's "two nodes share one identity"; cited by id). Typed code, cause,
//! owner key and member path only.

use super::*;
use ix_trace_rs::trace;

fn parameter(operation: &str, node_name: &str, name: &str) -> Value {
    let node = format!("{operation}/{node_name}");
    json!({
        "identity": id(&node), "name": name,
        "typeRef": "ix://quire/native/Boolean",
        "presence": "required", "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": origin(&node),
    })
}

/// The one intake refusal of `document`, as `(owner key, member path)`.
fn collision(document: &Value) -> (DeclarationKey, Option<MemberPath>) {
    let Err(refusals) = read_k2(document) else {
        panic!("the shared identity must be refused at intake");
    };
    assert_eq!(refusals.len(), 1, "one refusal: {refusals:?}");
    assert_eq!(refusals[0].code, Code::InvalidModelBinding);
    assert_eq!(refusals[0].cause.as_str(), "conflicting-binding");
    let ModelRefusalCause::ConflictingBinding { key, member } = &refusals[0].cause else {
        panic!("expected conflicting-binding, got {refusals:?}");
    };
    (key.clone(), member.clone())
}

/// Two fields of one owner carrying one identity.
#[trace("QSpec-TC-235", "QSpec-FR-154-AC-12")]
#[test]
fn two_fields_sharing_an_identity_are_conflicting_binding() {
    let document = mutate_k2(|document| {
        let mut second = field("Money", "amount_minor", "ix://quire/native/Integer");
        second["name"] = json!("amount_again");
        type_named_mut(document, "Money")["fields"]
            .as_array_mut()
            .unwrap()
            .push(second);
    });
    let (key, at) = collision(&document);
    assert_eq!(key, package_key("Money"));
    assert_eq!(at, Some(member("TypeDefinition.fields[2]")));
}

/// An operation carrying the identity of a field of its owner.
#[trace("QSpec-TC-235", "QSpec-FR-154-AC-12")]
#[test]
fn an_operation_sharing_a_fields_identity_is_conflicting_binding() {
    let document = mutate_k2(|document| {
        let mut shared = operation("OrderLifecycle", "placed");
        shared["name"] = json!("placed_again");
        type_named_mut(document, "OrderLifecycle")["operations"]
            .as_array_mut()
            .unwrap()
            .push(shared);
    });
    let (key, at) = collision(&document);
    assert_eq!(key, package_key("OrderLifecycle"));
    assert_eq!(at, Some(member("TypeDefinition.operations[2]")));
}

/// Two operations of one owner carrying one identity.
#[trace("QSpec-TC-235", "QSpec-FR-154-AC-12")]
#[test]
fn two_operations_sharing_an_identity_are_conflicting_binding() {
    let document = mutate_k2(|document| {
        let mut second = operation("OrderStore", "get");
        second["name"] = json!("get_again");
        type_named_mut(document, "OrderStore")["operations"]
            .as_array_mut()
            .unwrap()
            .push(second);
    });
    let (key, at) = collision(&document);
    assert_eq!(key, package_key("OrderStore"));
    assert_eq!(at, Some(member("TypeDefinition.operations[2]")));
}

/// Two parameters of one operation carrying one identity.
#[trace("QSpec-TC-235", "QSpec-FR-154-AC-12")]
#[test]
fn two_parameters_sharing_an_identity_are_conflicting_binding() {
    let document = mutate_k2(|document| {
        type_named_mut(document, "OrderStore")["operations"][0]["params"] = json!([
            parameter("OrderStore/get", "left", "left"),
            parameter("OrderStore/get", "left", "right"),
        ]);
    });
    let (key, at) = collision(&document);
    assert_eq!(key, package_key("OrderStore"));
    assert_eq!(
        at,
        Some(member("TypeDefinition.operations[0].params[1]"))
    );
}

/// Distinct identities in the same positions stay admitted.
#[trace("QSpec-TC-235", "QSpec-FR-154-AC-12")]
#[test]
fn distinct_identities_in_the_same_positions_are_admitted() {
    let document = mutate_k2(|document| {
        type_named_mut(document, "Money")["fields"]
            .as_array_mut()
            .unwrap()
            .push(field("Money", "extra", "ix://quire/native/Integer"));
        type_named_mut(document, "OrderStore")["operations"][0]["params"] = json!([
            parameter("OrderStore/get", "left", "left"),
            parameter("OrderStore/get", "right", "right"),
        ]);
    });
    if let Err(refusals) = read_k2(&document) {
        panic!("distinct identities are admitted: {refusals:?}");
    }
}
