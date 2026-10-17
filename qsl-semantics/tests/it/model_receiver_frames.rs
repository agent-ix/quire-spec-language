// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1043 step 3: FR-154's intake of the receiver-scoped frame members
//! `modifies_receiver`, `deletes_receiver` and `deletes_parameters` (rows I1
//! to I9 and I13 to I17 through the full admission pipeline; rows I10 to I12
//! are the intake reader's member-shape contract and are asserted in
//! `qsl-semantics/src/model/intake.rs`, because the semantic-IR schema
//! refuses those documents before the reader runs).
//!
//! The fixture is TC-1043's: `Counter` (fields `value` and `step`, the
//! relationship `Supersedes`, and the operation `t(n: Integer, a:
//! Reference<Counter>)`), `Sub` specializing `Counter`, `Gauge` (field `d`)
//! and the process `Flow` (field `x`, one step and the operation `go(a:
//! Reference<Counter>)`).

use ix_trace_rs::trace;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::meaning;
use qsl_semantics::model::intake::{admit_unit, UnitIntakeCause};
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::refusal::ModelRefusalCause;
use serde_json::{json, Value};

use super::model_operations::{
    admit, admit_and_assemble, config_unit, parse_and_build, wire_construct, wire_envelope,
    wire_field, wire_type, PACKAGE_IDENTITY,
};

fn id(suffix: &str) -> String {
    format!("ix://{PACKAGE_IDENTITY}/{suffix}")
}

fn origin(identity: &str) -> Value {
    json!({
        "generated": {
            "generatorIdentity": identity,
            "generatorVersion": "1.0.0",
            "inputIdentities": [identity],
        }
    })
}

fn one() -> Value {
    json!({"lower": 1, "upper": 1, "ordered": false, "unique": true})
}

fn parameter(operation: &str, name: &str, type_ref: &str) -> Value {
    let identity = format!("{operation}/{name}");
    json!({
        "identity": identity,
        "name": name,
        "typeRef": type_ref,
        "presence": "required",
        "nullable": false,
        "defaultKind": "none",
        "multiplicity": one(),
        "origin": origin(&identity),
    })
}

/// An operation `owner/name` over `params` with `frame` (none when absent).
fn operation(owner: &str, name: &str, params: Vec<Value>, frame: Option<Value>) -> Value {
    let identity = id(&format!("{owner}/{name}"));
    let mut node = json!({
        "identity": identity,
        "name": name,
        "params": params,
        "pre": [],
        "post": [],
        "origin": origin(&identity),
    });
    if let (Some(frame), Some(map)) = (frame, node.as_object_mut()) {
        map.insert("frame".to_owned(), frame);
    }
    node
}

fn t_params(owner: &str) -> Vec<Value> {
    let op = id(&format!("{owner}/t"));
    vec![
        parameter(&op, "n", "ix://quire/native/Integer"),
        parameter(&op, "a", &id("Counter")),
    ]
}

/// TC-1043's `Supersedes` relationship between `Counter` objects.
fn supersedes_id() -> String {
    id("relationship/Counter-supersedes-Counter")
}

fn supersedes() -> Value {
    json!({
        "identity": supersedes_id(),
        "sourceEnd": {"type": id("Counter"), "role": "newer", "multiplicity": one()},
        "targetEnd": {"type": id("Counter"), "multiplicity": one()},
        "category": "structural",
        "composite": false,
        "direction": "source-to-target",
        "origin": {
            "source": {
                "sourceIdentity": id("spec"),
                "path": "spec.qspec",
                "startLine": 1,
                "startColumn": 1,
            },
        },
    })
}

/// TC-1043's package with `t` on `Counter` (or on `Sub` when `on_sub`)
/// framed `frame`, and `go` of `Flow` framed `flow_frame` (no frame member
/// when `None`).
fn document(frame: &Value, on_sub: bool, flow_frame: Option<&Value>) -> Vec<u8> {
    let counter = id("Counter");
    let t_owner = if on_sub { "Sub" } else { "Counter" };
    let t = operation(t_owner, "t", t_params(t_owner), Some(frame.clone()));
    let counter_ops = if on_sub { vec![] } else { vec![t.clone()] };
    let sub_ops = if on_sub { vec![t] } else { vec![] };
    let go = operation(
        "Flow",
        "go",
        vec![parameter(&id("Flow/go"), "a", &counter)],
        flow_frame.cloned(),
    );
    let step = id("Flow/begin");
    wire_envelope(
        json!([
            wire_construct("object_type", meaning::OBJECT_TYPE, json!({})),
            wire_construct(
                "process",
                meaning::PROCESS,
                json!({"identityFields": "required", "steps": "required"}),
            ),
        ]),
        json!([
            wire_type(
                &counter,
                "object_type",
                json!({
                    "supertypes": [],
                    "fields": [
                        wire_field(&id("Counter/value"), "value", "ix://quire/native/Boolean", "required", 1),
                        wire_field(&id("Counter/step"), "step", "ix://quire/native/Boolean", "required", 1),
                    ],
                    "operations": counter_ops,
                    "relationships": [supersedes()],
                }),
            ),
            wire_type(
                &id("Sub"),
                "object_type",
                json!({"supertypes": [counter], "fields": [], "operations": sub_ops, "relationships": []}),
            ),
            wire_type(
                &id("Gauge"),
                "object_type",
                json!({
                    "supertypes": [],
                    "fields": [wire_field(&id("Gauge/d"), "d", "ix://quire/native/Boolean", "required", 1)],
                    "operations": [],
                    "relationships": [],
                }),
            ),
            wire_type(
                &id("Flow"),
                "process",
                json!({
                    "identityFields": [id("Flow/id")],
                    "fields": [
                        wire_field(&id("Flow/id"), "id", "ix://quire/native/Boolean", "required", 1),
                        wire_field(&id("Flow/x"), "x", "ix://quire/native/Boolean", "required", 1),
                    ],
                    "operations": [go],
                    "steps": [{
                        "identity": step,
                        "name": "begin",
                        "stepKind": "command",
                        "consumes": [],
                        "emits": [],
                        "origin": origin(&step),
                    }],
                }),
            ),
        ]),
        json!([]),
    )
    .to_string()
    .into_bytes()
}

fn frame(members: Value) -> Value {
    let mut frame = json!({"modifies": [], "creates": [], "deletes": []});
    if let (Some(base), Some(extra)) = (frame.as_object_mut(), members.as_object()) {
        base.extend(extra.clone());
    }
    frame
}

/// The one refusal `admit` reports for `document`.
fn sole_refusal(document: &[u8]) -> qsl_semantics::model::normalize::ModelRefusal {
    let refusal = admit(document).expect_err("the frame refuses at intake");
    let UnitIntakeCause::Refused(refusals) = refusal.cause else {
        panic!("{refusal:?}: expected a Refused cause");
    };
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    refusals.into_iter().next().expect("one refusal")
}

fn t_effect(
    on_sub: bool,
    frame_value: &Value,
) -> qsl_semantics::model::domain_package::OperationEffect {
    let document = document(frame_value, on_sub, None);
    let declarations = admit_and_assemble(&document).expect("the frame admits and assembles");
    let (unit, packages) = config_unit(&document);
    let built = parse_and_build(&unit);
    let views = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the package admits");
    let owner = DeclarationKey {
        package: PACKAGE_IDENTITY.to_owned(),
        node: id(if on_sub { "Sub" } else { "Counter" }),
    };
    let owner_id = views[0].view.type_identities()[&owner];
    declarations
        .operations
        .declared_by(owner_id)
        .iter()
        .find(|operation| operation.name() == "t")
        .expect("t is declared on its owner")
        .effect()
        .clone()
}

fn key(suffix: &str) -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE_IDENTITY.to_owned(),
        node: id(suffix),
    }
}

/// I1: `modifies_receiver` naming no declaration refuses `missing-name` at the entry.
#[trace("TC-1043", "FR-154-AC-8")]
#[test]
fn a_receiver_entry_naming_no_declaration_refuses_missing_name() {
    let refusal = sole_refusal(&document(
        &frame(json!({"modifies_receiver": [id("Counter/nope")]})),
        false,
        None,
    ));
    assert_eq!(refusal.code.as_str(), "missing_declaration");
    assert_eq!(refusal.cause.as_str(), "missing-name");
    let ModelRefusalCause::FrameEntryMissing { entry, .. } = &refusal.cause else {
        panic!("{:?}: expected FrameEntryMissing", refusal.cause);
    };
    assert_eq!(entry, &id("Counter/nope"));
}

/// I2 and I3: a field of a type `Counter` does not conform to, and a
/// relationship, refuse `malformed-declaration` at the entry.
#[trace("TC-1043", "FR-154-AC-8")]
#[test]
fn a_receiver_entry_of_another_type_or_meaning_refuses_malformed_declaration() {
    for named in [id("Gauge/d"), supersedes_id()] {
        let refusal = sole_refusal(&document(
            &frame(json!({"modifies_receiver": [named.clone()]})),
            false,
            None,
        ));
        assert_eq!(refusal.code.as_str(), "invalid_model_binding", "{named}");
        assert_eq!(refusal.cause.as_str(), "malformed-declaration", "{named}");
        let ModelRefusalCause::FrameEntryMalformed { entry, .. } = &refusal.cause else {
            panic!("{:?}: expected FrameEntryMalformed", refusal.cause);
        };
        assert_eq!(entry, &named);
    }
}

/// I4 and I5: a `deletes_parameters` name that is no parameter, and the
/// name of an `Integer` parameter, refuse `missing-name` at the entry.
#[trace("TC-1043", "FR-154-AC-8")]
#[test]
fn a_deleted_parameter_that_is_no_reference_parameter_refuses_missing_name() {
    for named in ["missing", "n"] {
        let refusal = sole_refusal(&document(
            &frame(json!({"deletes_parameters": [named]})),
            false,
            None,
        ));
        assert_eq!(refusal.code.as_str(), "missing_declaration", "{named}");
        assert_eq!(refusal.cause.as_str(), "missing-name", "{named}");
        let ModelRefusalCause::FrameEntryMissing { entry, .. } = &refusal.cause else {
            panic!("{:?}: expected FrameEntryMissing", refusal.cause);
        };
        assert_eq!(entry, named);
    }
}

/// I6: `deletes_parameters` naming a `Reference<Counter>` parameter with
/// `deletes_receiver` admits and reaches the effect.
#[trace("TC-1043", "FR-154-AC-8")]
#[test]
fn a_reference_parameter_and_the_receiver_admit_as_deletes() {
    let effect = t_effect(
        false,
        &frame(json!({"deletes_parameters": ["a"], "deletes_receiver": true})),
    );
    assert!(effect.deletes_receiver);
    assert_eq!(effect.deletes_arguments, vec!["a".to_owned()]);
    assert!(effect.receiver_modifies.is_empty() && effect.modifies.is_empty());
}

/// I7: an operation of `Sub` names `value`, which `Sub` inherits from `Counter`.
#[trace("TC-1043", "FR-154-AC-8")]
#[test]
fn a_receiver_entry_may_name_an_inherited_field() {
    let effect = t_effect(
        true,
        &frame(json!({"modifies_receiver": [id("Counter/value")]})),
    );
    assert_eq!(effect.receiver_modifies, vec![key("Counter/value")]);
    assert!(effect.modifies.is_empty());
}

/// I8: a missing receiver entry is the only refusal; the conflict on `value`
/// is checked once every entry resolves.
#[trace("TC-1043", "FR-154-AC-8")]
#[test]
fn an_unresolved_entry_is_reported_before_a_scope_conflict() {
    let refusal = sole_refusal(&document(
        &frame(json!({
            "modifies": [id("Counter/value")],
            "modifies_receiver": [id("Counter/nope"), id("Counter/value")],
        })),
        false,
        None,
    ));
    assert_eq!(refusal.cause.as_str(), "missing-name");
    let ModelRefusalCause::FrameEntryMissing { entry, .. } = &refusal.cause else {
        panic!("{:?}: expected FrameEntryMissing", refusal.cause);
    };
    assert_eq!(entry, &id("Counter/nope"));
}

/// I9: both scopes for one field refuse
/// `conflicting-binding` naming both entries and the operation, and no
/// package is built.
#[trace("TC-1043", "FR-154-AC-8")]
#[test]
fn both_scopes_of_one_field_refuse_conflicting_binding() {
    let refusal = sole_refusal(&document(
        &frame(json!({
            "modifies": [id("Counter/value")],
            "modifies_receiver": [id("Counter/value")],
        })),
        false,
        None,
    ));
    assert_eq!(refusal.code.as_str(), "invalid_model_binding");
    assert_eq!(refusal.cause.as_str(), "conflicting-binding");
    let ModelRefusalCause::ConflictingBinding {
        key: operation,
        member,
    } = &refusal.cause
    else {
        panic!("{:?}: expected ConflictingBinding", refusal.cause);
    };
    assert_eq!(operation, &key("Counter/t"));
    assert!(
        format!("{member:?}").contains("modifies_receiver"),
        "located at the later entry: {member:?}"
    );
    assert!(
        refusal.detail.contains("modifies[0]") && refusal.detail.contains("modifies_receiver[0]"),
        "names both entries: {}",
        refusal.detail
    );
}

/// I13, I14 and I17: over the process `Flow`, a non-empty `modifies_receiver`
/// and `deletes_receiver: true` refuse at the member, `modifies_receiver`
/// first when both are present.
#[trace("TC-1043", "FR-154-AC-8")]
#[test]
fn a_process_operation_refuses_receiver_scoped_members_at_the_member() {
    let ok = frame(json!({}));
    let cases = [
        (
            json!({"modifies_receiver": [id("Flow/x")]}),
            "modifies_receiver",
        ),
        (json!({"deletes_receiver": true}), "deletes_receiver"),
        (
            json!({"modifies_receiver": [id("Flow/x")], "deletes_receiver": true}),
            "modifies_receiver",
        ),
    ];
    for (members, member) in cases {
        let refusal = sole_refusal(&document(&ok, false, Some(&frame(members))));
        assert_eq!(refusal.code.as_str(), "invalid_model_binding", "{member}");
        assert_eq!(refusal.cause.as_str(), "malformed-declaration", "{member}");
        // At the member itself, before any entry is read: the typed cause is
        // the node's malformed-declaration and its member path ends at the
        // member with no entry index.
        let ModelRefusalCause::IntakeMalformedDeclaration { member: at, .. } = &refusal.cause
        else {
            panic!(
                "{member}: expected IntakeMalformedDeclaration, got {:?}",
                refusal.cause
            );
        };
        assert!(
            format!("{at:?}").ends_with(&format!("Name(\"frame\"), Name(\"{member}\")] }}")),
            "{member}: {at:?}"
        );
    }
}

/// `document` plus the record value type `Pair`, whose operation `swap()`
/// is framed `frame_value`.
fn document_with_pair(frame_value: &Value) -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(&document(&frame(json!({})), false, None))
        .expect("the fixture is JSON");
    let swap = operation("Pair", "swap", Vec::new(), Some(frame_value.clone()));
    let pair = wire_type(
        &id("Pair"),
        "record_value_type",
        json!({
            "fields": [wire_field(&id("Pair/n"), "n", "ix://quire/native/Boolean", "required", 1)],
            "operations": [swap],
        }),
    );
    document["types"].as_array_mut().expect("types").push(pair);
    document["constructs"]
        .as_array_mut()
        .expect("constructs")
        .push(wire_construct(
            "record_value_type",
            meaning::RECORD_VALUE_TYPE,
            json!({"identityFields": "optional"}),
        ));
    document.to_string().into_bytes()
}

/// I23: over the record value type `Pair`, whose operation has no object
/// receiver, a non-empty `modifies_receiver` with `deletes_receiver: true`
/// refuses `malformed-declaration` at `modifies_receiver` alone, and the
/// entry `Counter/value` is not read.
#[trace("TC-1043", "FR-154-AC-8")]
#[test]
fn a_record_value_type_operation_refuses_receiver_scoped_members_at_the_member() {
    let refusal = sole_refusal(&document_with_pair(&frame(json!({
        "modifies_receiver": [id("Counter/value")],
        "deletes_receiver": true,
    }))));
    assert_eq!(refusal.code.as_str(), "invalid_model_binding");
    let ModelRefusalCause::IntakeMalformedDeclaration { member, .. } = &refusal.cause else {
        panic!(
            "expected IntakeMalformedDeclaration, got {:?}",
            refusal.cause
        );
    };
    assert!(
        format!("{member:?}").ends_with("Name(\"frame\"), Name(\"modifies_receiver\")] }"),
        "{member:?}"
    );
}

/// I15 and I16: over `Flow`, an empty `modifies_receiver` with
/// `deletes_receiver: false`, and `deletes_parameters` naming a
/// `Reference<Counter>` parameter, admit.
#[trace("TC-1043", "FR-154-AC-8")]
#[test]
fn a_process_operation_admits_empty_receiver_members_and_deleted_parameters() {
    for members in [
        json!({"modifies_receiver": [], "deletes_receiver": false}),
        json!({"deletes_parameters": ["a"]}),
    ] {
        let document = document(&frame(json!({})), false, Some(&frame(members.clone())));
        admit(&document).unwrap_or_else(|refusal| panic!("{members}: {refusal:?}"));
    }
}
