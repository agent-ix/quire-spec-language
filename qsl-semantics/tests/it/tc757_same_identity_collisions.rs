//! Same-identity collisions of one owner (field, operation, parameter) refuse
//! `conflicting-binding`, like a clause, state, transition or step (QSpec
//! FR-154's "two nodes share one identity", FR-154-AC-13; cited by id). Typed code, cause,
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

/// The one refusal of `document`, as `(shared identity key, member path)`.
fn collision(document: &Value) -> (DeclarationKey, Option<MemberPath>) {
    let package = read_k2(document).expect("intake retains the typed refusal for normalization");
    let (outcome, _meter) = normalize_with_meter(&package, ModelNormalizationLimits::default());
    let NormalizeOutcome::Refused(refusals) = outcome else {
        panic!("the shared identity must be refused, got {outcome:?}");
    };
    let refusals: Vec<ModelRefusal> = refusals.iter().cloned().collect();
    assert_eq!(refusals.len(), 1, "one refusal: {refusals:?}");
    assert_eq!(refusals[0].code, Code::InvalidModelBinding);
    assert_eq!(refusals[0].cause.as_str(), "conflicting-binding");
    let ModelRefusalCause::ConflictingBinding { key, member } = &refusals[0].cause else {
        panic!("expected conflicting-binding, got {refusals:?}");
    };
    (key.clone(), member.clone())
}

/// Two fields of one owner carrying one identity.
#[trace("FR-056-AC-30", "TC-1003", "QSpec-FR-154-AC-13")]
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
    assert_eq!(key, package_key("Money/amount_minor"));
    assert_eq!(at, Some(member("TypeDefinition.fields[2]")));
}

/// An operation carrying the identity of a field of its owner.
#[trace("FR-056-AC-30", "TC-1003", "QSpec-FR-154-AC-13")]
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
    assert_eq!(key, package_key("OrderLifecycle/placed"));
    assert_eq!(at, Some(member("TypeDefinition.operations[2]")));
}

/// Two operations of one owner carrying one identity.
#[trace("FR-056-AC-30", "TC-1003", "QSpec-FR-154-AC-13")]
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
    assert_eq!(key, package_key("OrderStore/get"));
    assert_eq!(at, Some(member("TypeDefinition.operations[2]")));
}

/// Two parameters of one operation carrying one identity.
#[trace("FR-056-AC-30", "TC-1003", "QSpec-FR-154-AC-13")]
#[test]
fn two_parameters_sharing_an_identity_are_conflicting_binding() {
    let document = mutate_k2(|document| {
        type_named_mut(document, "OrderStore")["operations"][0]["params"] = json!([
            parameter("OrderStore/get", "left", "left"),
            parameter("OrderStore/get", "left", "right"),
        ]);
    });
    let (key, at) = collision(&document);
    assert_eq!(key, package_key("OrderStore/get/left"));
    assert_eq!(at, Some(member("TypeDefinition.operations[0].params[1]")));
}

/// Normalizes `document`; it must be admitted, with no intake refusal kept on
/// the package and no normalization refusal.
fn assert_admitted(document: &Value) {
    let package = read_k2(document).expect("intake retains typed refusals for normalization");
    let (outcome, _meter) = normalize_with_meter(&package, ModelNormalizationLimits::default());
    assert!(
        matches!(outcome, NormalizeOutcome::Completed(_)),
        "distinct identities are admitted, got {outcome:?}"
    );
}

/// Distinct identities in the positions above stay admitted.
#[trace("FR-056-AC-30", "TC-1003", "QSpec-FR-154-AC-13")]
#[test]
fn distinct_identities_in_the_same_positions_are_admitted() {
    assert_admitted(&mutate_k2(|document| {
        type_named_mut(document, "Money")["fields"]
            .as_array_mut()
            .unwrap()
            .push(field("Money", "extra", "ix://quire/native/Integer"));
    }));
    assert_admitted(&mutate_k2(|document| {
        type_named_mut(document, "OrderStore")["operations"][0]["params"] = json!([
            parameter("OrderStore/get", "left", "left"),
            parameter("OrderStore/get", "right", "right"),
        ]);
    }));
}

/// One member name under two different owners names two nodes, not a
/// collision: `label` on `Order` and `Money`, `reset` on `OrderStore` and
/// `OrderLifecycle`.
#[trace("FR-056-AC-30", "TC-1003", "QSpec-FR-154-AC-13")]
#[test]
fn one_member_name_in_different_owners_is_admitted() {
    assert_admitted(&mutate_k2(|document| {
        for owner in ["Order", "Money"] {
            type_named_mut(document, owner)["fields"]
                .as_array_mut()
                .unwrap()
                .push(field(owner, "label", "ix://quire/native/String"));
        }
        for owner in ["OrderStore", "OrderLifecycle"] {
            type_named_mut(document, owner)["operations"]
                .as_array_mut()
                .unwrap()
                .push(operation(owner, "reset"));
        }
    }));
}

/// The admitted helper can fail: a colliding package is not admitted.
#[test]
#[should_panic(expected = "distinct identities are admitted")]
fn the_admitted_helper_rejects_a_collision() {
    assert_admitted(&mutate_k2(|document| {
        let mut second = operation("OrderStore", "get");
        second["name"] = json!("get_again");
        type_named_mut(document, "OrderStore")["operations"]
            .as_array_mut()
            .unwrap()
            .push(second);
    }));
}
