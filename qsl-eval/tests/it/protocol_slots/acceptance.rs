// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1070's E and L protocols, evaluated through their emitted identities.

use super::*;
use qsl_package::EmitLimits;
use qsl_semantics::model::observation::PathStep;
use qsl_semantics::model::observation::{DocumentRef, ObservationLimits};
use quire_exact::{
    CardinalityBound, CollectionKind, CollectionType, FieldValue, LimitKind, NodeKey, ObjectId,
    ObjectReference, ValueType,
};
use quire_semantic_value::admission::ObservationAdmission;

macro_rules! assert_value_eq {
    ($actual:expr, $expected:expr $(, $message:literal)?) => {{
        let actual = &$actual;
        let expected = &$expected;
        assert_eq!(quire_exact::compare_keys(actual, expected), Some(std::cmp::Ordering::Equal), "{}; actual {actual:?}; expected {expected:?}", concat!($($message)?));
    }};
}

mod bare_slots;
mod state_compensation;

#[path = "../../../../qsl-semantics/tests/support/tc1070.rs"]
mod tc1070;
use tc1070::{document, document_ep, model, SOURCE};

#[test]
fn ep_extension_is_admitted_by_the_shared_tc1070_fixture() {
    let _ = fixture_ep();
}

fn fixture_with_view() -> (
    CheckedPackage,
    qsl_semantics::model::normalize::EffectiveView,
) {
    fixture_from_source(SOURCE)
}

fn fixture_ep() -> (
    CheckedPackage,
    qsl_semantics::model::normalize::EffectiveView,
) {
    let bytes = serde_json::to_vec(&document_ep()).unwrap();
    fixture_from_document(SOURCE, &bytes)
}

fn fixture_from_source(
    source: &str,
) -> (
    CheckedPackage,
    qsl_semantics::model::normalize::EffectiveView,
) {
    fixture_from_document(source, &serde_json::to_vec(&document()).unwrap())
}

fn fixture_from_document(
    source: &str,
    bytes: &[u8],
) -> (
    CheckedPackage,
    qsl_semantics::model::normalize::EffectiveView,
) {
    fixture_named(source, bytes, "acceptance")
}

fn fixture_named(
    source: &str,
    bytes: &[u8],
    name: &str,
) -> (
    CheckedPackage,
    qsl_semantics::model::normalize::EffectiveView,
) {
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("protocol", "acceptance"),
        "acceptance.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .unwrap();
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).unwrap();
    let models = qsl_semantics::model::intake::admit_unit(
        &unit.selections().models,
        &qsl_semantics::model::intake::package_input([bytes]).unwrap(),
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .unwrap();
    let view = models[0].view.clone();
    (
        CheckedPackage::link(
            PackageDeclarations::assemble(
                parsed.source().reference().clone(),
                name.to_owned(),
                unit,
                models,
                Vec::new(),
            )
            .unwrap()
            .check(CheckingLimits::default())
            .expect("TC-1070's legitimate protocol package checks"),
        ),
        view,
    )
}

fn fixture() -> CheckedPackage {
    fixture_with_view().0
}

fn int(value: i64) -> Value {
    Value::Integer(value.into())
}

fn record(package: &CheckedPackage, name: &str, fields: Vec<(&str, Value)>) -> Value {
    let types = package.graph().scope().types();
    let key = types
        .composites()
        .find(|ty| ty.name() == format!("M::{name}"))
        .expect("fixture record type")
        .key();
    types
        .record(
            key,
            fields
                .into_iter()
                .map(|(name, value)| (name, FieldValue::Present(value)))
                .collect(),
        )
        .unwrap()
}

fn receipt(package: &CheckedPackage, total: i64) -> Value {
    record(package, "Receipt", vec![("total", int(total))])
}
fn attempt(package: &CheckedPackage, tally: i64) -> Value {
    record(package, "Attempt", vec![("tally", int(tally))])
}
fn flag(package: &CheckedPackage, ready: bool) -> Value {
    record(package, "Flag", vec![("ready", Value::Boolean(ready))])
}

fn binder(package: &CheckedPackage, protocol: &str, name: &str) -> NodeKey {
    package
        .graph()
        .protocol_node_keys(protocol)
        .unwrap()
        .binders
        .iter()
        .find(|binder| binder.name == name)
        .unwrap_or_else(|| panic!("{protocol} binds {name}"))
        .node
        .clone()
}

fn slot(
    package: &CheckedPackage,
    protocol: &str,
    owner: &str,
    member: &str,
    index: Option<usize>,
) -> ProtocolSlot {
    let keys = package.graph().protocol_node_keys(protocol).unwrap();
    let checked = package.graph().protocol(protocol).unwrap();
    let node = if owner == protocol {
        keys.clause.clone().unwrap()
    } else if let Some(index) = checked
        .obligations
        .compensations
        .iter()
        .position(|template| template.name == owner)
    {
        keys.compensations[&index].clone()
    } else if let Some((id, name)) = &checked.finish {
        if name == owner {
            keys.nodes[id].clone()
        } else {
            let control = checked
                .controls
                .controls
                .iter()
                .find(|control| control.name == owner)
                .expect("named control");
            keys.nodes[&control.node].clone()
        }
    } else {
        let control = checked
            .controls
            .controls
            .iter()
            .find(|control| control.name == owner)
            .expect("named control");
        keys.nodes[&control.node].clone()
    };
    ProtocolSlot {
        node,
        member: member.to_owned(),
        index,
    }
}

fn supply<'a>(
    package: &CheckedPackage,
    protocol: &str,
    values: Vec<(&str, Value)>,
    observation: Option<&'a ObservationAdmission<qsl_package::AdmittedProtocolObservation<'a>>>,
) -> ProtocolSupply<'a> {
    ProtocolSupply {
        binders: values
            .into_iter()
            .map(|(name, value)| {
                let references = match &value {
                    Value::Reference(_) => vec![ProtocolReferenceOccurrence {
                        path: Vec::new(),
                        population: population_node("nodes"),
                    }],
                    _ => Vec::new(),
                };
                (
                    binder(package, protocol, name),
                    ProtocolSuppliedValue {
                        value: SuppliedValue::Value(value),
                        references,
                    },
                )
            })
            .collect(),
        observation,
    }
}

fn population_node(name: &str) -> NodeKey {
    NodeKey::new(
        model::PACKAGE,
        vec![
            quire_exact::PathSegment::Text("relation".to_owned()),
            quire_exact::PathSegment::Text("population".to_owned()),
            quire_exact::PathSegment::Text(model::id(name)),
        ],
        0,
    )
    .unwrap()
}

fn other_reference(
    view: &qsl_semantics::model::normalize::EffectiveView,
    key: &str,
) -> ObjectReference {
    let declaration = quire_checked::key::DeclarationKey {
        package: model::PACKAGE.to_owned(),
        node: model::id("Other"),
    };
    ObjectReference::new(
        view.object_universe_of(&declaration)
            .unwrap()
            .identity()
            .unwrap(),
        view.type_identities()[&declaration].clone(),
        ObjectId::new(key).unwrap(),
    )
}

fn ep_receipt_and_bundle(package: &CheckedPackage, refs: [&ObjectReference; 4]) -> (Value, Value) {
    let types = package.graph().scope().types();
    let other = || Value::Reference(refs[0].clone());
    let sequence_type = CollectionType::new(
        CollectionKind::Sequence,
        ValueType::Reference(refs[3].object_type().clone()),
        Some(CardinalityBound::new(1, 2).unwrap()),
    );
    let sequence =
        quire_exact::from_admitted(sequence_type, vec![Value::Reference(refs[3].clone())]);
    let bundle_key = types
        .composites()
        .find(|ty| ty.name() == "M::PeerBundle")
        .unwrap()
        .key();
    let bundle = types
        .record(
            bundle_key,
            vec![
                (
                    "nested",
                    FieldValue::Present(Value::Reference(refs[1].clone())),
                ),
                (
                    "optional",
                    FieldValue::Present(Value::Reference(refs[2].clone())),
                ),
                ("sequence", FieldValue::Present(sequence)),
            ],
        )
        .unwrap();
    let receipt_key = types
        .composites()
        .find(|ty| ty.name() == "M::Receipt")
        .unwrap()
        .key();
    let receipt = types
        .record(
            receipt_key,
            vec![
                ("total", FieldValue::Present(int(5))),
                ("peer", FieldValue::Present(other())),
                ("bundle", FieldValue::Present(bundle.clone())),
            ],
        )
        .unwrap();
    (receipt, bundle)
}

fn ep_receipt(package: &CheckedPackage, refs: [&ObjectReference; 4]) -> Value {
    ep_receipt_and_bundle(package, refs).0
}

fn ep_observation<'a>(
    package: &'a CheckedPackage,
    _view: &qsl_semantics::model::normalize::EffectiveView,
    other_spare: Option<(bool, &[&str])>,
    other_nodes_complete: bool,
) -> ObservationAdmission<qsl_package::AdmittedProtocolObservation<'a>> {
    ep_observation_state(package, other_spare, other_nodes_complete, None, None, &[])
}

fn ep_observation_state<'a>(
    package: &'a CheckedPackage,
    other_spare: Option<(bool, &[&str])>,
    other_nodes_complete: bool,
    ghost_next: Option<(&str, &str)>,
    relationship_target: Option<(&str, &str)>,
    extra_other_nodes: &[&str],
) -> ObservationAdmission<qsl_package::AdmittedProtocolObservation<'a>> {
    let evaluation_view = package.evaluation_view();
    ep_observation_state_through_view(
        package,
        &evaluation_view,
        other_spare,
        other_nodes_complete,
        ghost_next,
        relationship_target,
        extra_other_nodes,
    )
}

fn ep_observation_state_through_view<'a>(
    package: &'a CheckedPackage,
    evaluation_view: &qsl_package::AdmittedEvaluationView<'a>,
    other_spare: Option<(bool, &[&str])>,
    other_nodes_complete: bool,
    ghost_next: Option<(&str, &str)>,
    relationship_target: Option<(&str, &str)>,
    extra_other_nodes: &[&str],
) -> ObservationAdmission<qsl_package::AdmittedProtocolObservation<'a>> {
    let (packages, selected, snapshots) = ep_snapshot(
        other_spare,
        other_nodes_complete,
        ghost_next,
        relationship_target,
        extra_other_nodes,
    );
    evaluation_view.admit_protocol_observation(
        &package.graph().model_selections()[0],
        &packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        &snapshots,
        &selected,
        ObservationLimits::default(),
    )
}

/// The domain package input, selected document and snapshot of an E-P
/// observation state (TC-1070 rows 34 to 46).
fn ep_snapshot(
    other_spare: Option<(bool, &[&str])>,
    other_nodes_complete: bool,
    ghost_next: Option<(&str, &str)>,
    relationship_target: Option<(&str, &str)>,
    extra_other_nodes: &[&str],
) -> (
    qsl_semantics::model::intake::PackageInput,
    DocumentRef,
    std::collections::BTreeMap<DocumentRef, Vec<u8>>,
) {
    let domain = serde_json::to_vec(&document_ep()).unwrap();
    let packages = qsl_semantics::model::intake::package_input([domain.as_slice()]).unwrap();
    let selected = DocumentRef {
        authority: "test".to_owned(),
        identity: "ep-current".to_owned(),
    };
    let object = |key: &str, pop: &str| {
        let next = if key == "ghost" && pop == "otherSpare" {
            ghost_next.map(|(population, target)| serde_json::json!({"present":{"reference":{"population":model::id(population),"key":target}}})).unwrap_or_else(|| serde_json::json!({"absent":{}}))
        } else {
            serde_json::json!({"absent":{}})
        };
        serde_json::json!({"key": key, "type": model::id("Other"), "fields": {"marker": {"boolean": true}, "next": next}})
    };
    let mut populations = vec![
        serde_json::json!({"population": model::id("nodes"), "complete": true, "objects": [{"key":"n5", "type":model::id("Node"), "fields":{"total":{"integer":"5"}}}]}),
    ];
    let mut other_nodes = Vec::new();
    if !other_spare.is_some_and(|(_, keys)| keys.contains(&"nested-1")) {
        other_nodes.push(object("nested-1", "otherNodes"));
    }
    if !other_spare.is_some_and(|(_, keys)| keys.contains(&"sequence-1")) {
        other_nodes.push(object("sequence-1", "otherNodes"));
    }
    other_nodes.extend(
        extra_other_nodes
            .iter()
            .map(|key| object(key, "otherNodes")),
    );
    let mut spare = Vec::new();
    let mut spare_complete = true;
    if let Some((complete, keys)) = other_spare {
        spare_complete = complete;
        spare.extend(keys.iter().map(|key| object(key, "otherSpare")));
    }
    if !other_spare.is_some_and(|(_, keys)| keys.contains(&"ghost")) {
        other_nodes.push(object("ghost", "otherNodes"));
    }
    populations.push(serde_json::json!({"population":model::id("otherNodes"), "complete":other_nodes_complete, "objects":other_nodes}));
    if other_spare.is_some() {
        populations.push(serde_json::json!({"population":model::id("otherSpare"), "complete":spare_complete, "objects":spare}));
    }
    let snapshot = serde_json::json!({
        "format":"quire.state.snapshot/v1",
        "identity":{"authority":selected.authority, "identity":selected.identity},
        "observation":"current", "model":{"identity":model::PACKAGE,"version":"1.0.0","digest":"set at admission"},
        "populations":populations,
        "relationships":[{"relationship":model::id("relationship/otherLinks"),"complete":true,"instances":relationship_target.map(|(population,key)| vec![serde_json::json!({"source":{"population":model::id("otherSpare"),"key":"ghost"},"target":{"population":model::id(population),"key":key}})]).unwrap_or_default()}]
    });
    let snapshots = [(selected.clone(), serde_json::to_vec(&snapshot).unwrap())]
        .into_iter()
        .collect();
    (packages, selected, snapshots)
}

#[test]
#[trace("TC-1070", "FR-390-AC-23")]
fn nested_reference_provenance_drives_the_named_population_admission() {
    let (package, view) = fixture_ep();
    let peer = other_reference(&view, "ghost");
    let nested = other_reference(&view, "nested-1");
    let optional = other_reference(&view, "option-1");
    let sequence = other_reference(&view, "sequence-1");
    let receipt = ep_receipt(&package, [&peer, &nested, &optional, &sequence]);
    let references = vec![
        ProtocolReferenceOccurrence {
            path: vec![PathStep::Field("peer".into())],
            population: population_node("otherSpare"),
        },
        ProtocolReferenceOccurrence {
            path: vec![
                PathStep::Field("bundle".into()),
                PathStep::Field("nested".into()),
            ],
            population: population_node("otherNodes"),
        },
        ProtocolReferenceOccurrence {
            path: vec![
                PathStep::Field("bundle".into()),
                PathStep::Field("optional".into()),
                PathStep::OptionValue,
            ],
            population: population_node("otherSpare"),
        },
        ProtocolReferenceOccurrence {
            path: vec![
                PathStep::Field("bundle".into()),
                PathStep::Field("sequence".into()),
                PathStep::Index(0),
            ],
            population: population_node("otherNodes"),
        },
    ];
    let observation = ep_observation(&package, &view, Some((true, &["ghost", "option-1"])), true);
    let mut meter = Meter::new(UNLIMITED);
    let output = package
        .evaluate_protocol_slot(
            &slot(
                &package,
                "Refunds",
                "Refund",
                "registration_captures",
                Some(0),
            ),
            &ep_supply(
                &package,
                receipt.clone(),
                references.clone(),
                Some(&observation),
            ),
            &mut meter,
        )
        .unwrap();
    assert_value_eq!(
        completed(output),
        int(5),
        "all four explicitly attributed nested references are admitted"
    );

    let incomplete = ep_observation(&package, &view, Some((false, &[])), true);
    let mut meter = Meter::new(UNLIMITED);
    let failure = package
        .evaluate_protocol_slot(
            &slot(
                &package,
                "Refunds",
                "Refund",
                "registration_captures",
                Some(0),
            ),
            &ep_supply(&package, receipt, references.clone(), Some(&incomplete)),
            &mut meter,
        )
        .unwrap_err();
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 0);
    let CallFailure::Admission(failure) = failure else {
        panic!("expected admission refusal")
    };
    let CallAdmissionFailure::Incomplete(record) = failure.as_ref() else {
        panic!("expected incomplete population")
    };
    assert_eq!(record.record.catalog.code(), "incomplete_population");
    assert_eq!(
        record.locus,
        CallAdmissionLocus::Population(population_node("otherSpare"))
    );

    for (state, expected_code) in [
        (None, "incomplete_population"),
        (Some((true, &[][..])), "dangling_reference"),
    ] {
        let observation = ep_observation(&package, &view, state, true);
        let mut meter = Meter::new(UNLIMITED);
        let failure = package
            .evaluate_protocol_slot(
                &slot(
                    &package,
                    "Refunds",
                    "Refund",
                    "registration_captures",
                    Some(0),
                ),
                &ep_supply(
                    &package,
                    ep_receipt(&package, [&peer, &nested, &optional, &sequence]),
                    references.clone(),
                    Some(&observation),
                ),
                &mut meter,
            )
            .unwrap_err();
        assert_eq!(meter.consumed(LimitKind::WorkUnits), 0);
        let CallFailure::Admission(failure) = failure else {
            panic!("expected E-P seed refusal")
        };
        let record = match failure.as_ref() {
            CallAdmissionFailure::Incomplete(record) | CallAdmissionFailure::Refused(record) => {
                record
            }
        };
        assert_eq!(record.record.catalog.code(), expected_code);
        assert_eq!(
            record.locus,
            if expected_code == "incomplete_population" {
                CallAdmissionLocus::Population(population_node("otherSpare"))
            } else {
                CallAdmissionLocus::Binder(binder(&package, "Refunds", "forward"))
            }
        );
    }

    for index in 0..references.len() {
        let mut omitted = references.clone();
        let expected_path = omitted.remove(index).path;
        let mut meter = Meter::new(UNLIMITED);
        let error = package
            .evaluate_protocol_slot(
                &slot(
                    &package,
                    "Refunds",
                    "Refund",
                    "registration_captures",
                    Some(0),
                ),
                &ep_supply(
                    &package,
                    ep_receipt(&package, [&peer, &nested, &optional, &sequence]),
                    omitted,
                    Some(&observation),
                ),
                &mut meter,
            )
            .unwrap_err();
        let CallFailure::Admission(failure) = error else {
            panic!("omitted occurrence must refuse")
        };
        let CallAdmissionFailure::Refused(record) = failure.as_ref() else {
            panic!("omission is a refusal")
        };
        assert_eq!(record.record.catalog.cause(), "missing-member");
        assert_eq!(record.record.path, expected_path);
        assert_eq!(meter.consumed(LimitKind::WorkUnits), 0);

        let mut duplicated = references.clone();
        let repeated = duplicated[index].clone();
        duplicated.insert(index + 1, repeated.clone());
        let error = package
            .evaluate_protocol_slot(
                &slot(
                    &package,
                    "Refunds",
                    "Refund",
                    "registration_captures",
                    Some(0),
                ),
                &ep_supply(
                    &package,
                    ep_receipt(&package, [&peer, &nested, &optional, &sequence]),
                    duplicated,
                    Some(&observation),
                ),
                &mut meter,
            )
            .unwrap_err();
        let CallFailure::Admission(failure) = error else {
            panic!("duplicate occurrence must refuse")
        };
        let CallAdmissionFailure::Refused(record) = failure.as_ref() else {
            panic!("duplicate is a refusal")
        };
        assert_eq!(record.record.catalog.cause(), "duplicate-member");
        assert_eq!(record.record.path, repeated.path);
        assert_eq!(meter.consumed(LimitKind::WorkUnits), 0);
    }

    for extra_path in [
        vec![PathStep::Field("total".into())],
        vec![
            PathStep::Field("bundle".into()),
            PathStep::Field("absent".into()),
        ],
    ] {
        let mut extra = references.clone();
        extra.push(ProtocolReferenceOccurrence {
            path: extra_path.clone(),
            population: population_node("otherNodes"),
        });
        let error = package
            .evaluate_protocol_slot(
                &slot(
                    &package,
                    "Refunds",
                    "Refund",
                    "registration_captures",
                    Some(0),
                ),
                &ep_supply(
                    &package,
                    ep_receipt(&package, [&peer, &nested, &optional, &sequence]),
                    extra,
                    Some(&observation),
                ),
                &mut Meter::new(UNLIMITED),
            )
            .unwrap_err();
        let CallFailure::Admission(failure) = error else {
            panic!("extra occurrence must refuse")
        };
        let CallAdmissionFailure::Refused(record) = failure.as_ref() else {
            panic!("extra is a refusal")
        };
        assert_eq!(record.record.catalog.cause(), "unknown-member");
        assert_eq!(record.record.path, extra_path);
    }

    for (population, expected_code, expected_cause) in [
        (
            population_node("elsewhereNodes"),
            "invalid_runtime_input",
            "wrong-value-kind",
        ),
        (
            NodeKey::new(
                "other/model",
                vec![
                    quire_exact::PathSegment::Text("relation".into()),
                    quire_exact::PathSegment::Text("population".into()),
                    quire_exact::PathSegment::Text("ix://other/model/externalOthers".into()),
                ],
                0,
            )
            .unwrap(),
            "invalid_model_binding",
            "wrong-model-selection",
        ),
    ] {
        let mut supplied = references.clone();
        supplied[0].population = population;
        let error = package
            .evaluate_protocol_slot(
                &slot(
                    &package,
                    "Refunds",
                    "Refund",
                    "registration_captures",
                    Some(0),
                ),
                &ep_supply(
                    &package,
                    ep_receipt(&package, [&peer, &nested, &optional, &sequence]),
                    supplied,
                    Some(&observation),
                ),
                &mut Meter::new(UNLIMITED),
            )
            .unwrap_err();
        let CallFailure::Admission(failure) = error else {
            panic!("incompatible provenance must refuse")
        };
        let CallAdmissionFailure::Refused(record) = failure.as_ref() else {
            panic!("provenance mismatch is a refusal")
        };
        assert_eq!(record.record.catalog.code(), expected_code);
        assert_eq!(record.record.catalog.cause(), expected_cause);
    }
}

fn ep_supply<'a>(
    package: &CheckedPackage,
    receipt: Value,
    references: Vec<ProtocolReferenceOccurrence>,
    observation: Option<&'a ObservationAdmission<qsl_package::AdmittedProtocolObservation<'a>>>,
) -> ProtocolSupply<'a> {
    ProtocolSupply {
        binders: vec![(
            binder(package, "Refunds", "forward"),
            ProtocolSuppliedValue {
                value: SuppliedValue::Value(receipt),
                references,
            },
        )],
        observation,
    }
}

#[test]
#[trace("TC-1070", "FR-390-AC-23")]
fn nested_population_closure_uses_object_fields_then_relationship_ends() {
    let (package, view) = fixture_ep();
    let peer = other_reference(&view, "ghost");
    let nested = other_reference(&view, "nested-1");
    let optional = other_reference(&view, "option-1");
    let sequence = other_reference(&view, "sequence-1");
    let receipt = ep_receipt(&package, [&peer, &nested, &optional, &sequence]);
    let refs = vec![
        ProtocolReferenceOccurrence {
            path: vec![PathStep::Field("peer".into())],
            population: population_node("otherSpare"),
        },
        ProtocolReferenceOccurrence {
            path: vec![
                PathStep::Field("bundle".into()),
                PathStep::Field("nested".into()),
            ],
            population: population_node("otherSpare"),
        },
        ProtocolReferenceOccurrence {
            path: vec![
                PathStep::Field("bundle".into()),
                PathStep::Field("optional".into()),
                PathStep::OptionValue,
            ],
            population: population_node("otherSpare"),
        },
        ProtocolReferenceOccurrence {
            path: vec![
                PathStep::Field("bundle".into()),
                PathStep::Field("sequence".into()),
                PathStep::Index(0),
            ],
            population: population_node("otherSpare"),
        },
    ];
    let call =
        |observation: &ObservationAdmission<qsl_package::AdmittedProtocolObservation<'_>>,
         meter: &mut Meter| {
            package.evaluate_protocol_slot(
                &slot(
                    &package,
                    "Refunds",
                    "Refund",
                    "registration_captures",
                    Some(0),
                ),
                &ep_supply(&package, receipt.clone(), refs.clone(), Some(observation)),
                meter,
            )
        };
    let all_spare = Some((true, &["ghost", "nested-1", "option-1", "sequence-1"][..]));

    for (field, relation, complete) in [
        (Some(("otherNodes", "field-child")), None, false),
        (None, Some(("otherNodes", "edge-child")), false),
    ] {
        let extra = if field.is_some() {
            &["field-child"][..]
        } else {
            &["edge-child"][..]
        };
        let observation =
            ep_observation_state(&package, all_spare, complete, field, relation, extra);
        let mut meter = Meter::new(UNLIMITED);
        let result = call(&observation, &mut meter);
        let Err(CallFailure::Admission(failure)) = result else {
            panic!("recursive population must be incomplete: {result:?}")
        };
        let CallAdmissionFailure::Incomplete(record) = failure.as_ref() else {
            panic!("expected incomplete nested population")
        };
        assert_eq!(record.record.catalog.code(), "incomplete_population");
        assert_eq!(
            record.locus,
            CallAdmissionLocus::Population(population_node("otherNodes"))
        );
        assert_eq!(meter.consumed(LimitKind::WorkUnits), 0);
    }

    let observation = ep_observation_state(
        &package,
        all_spare,
        true,
        Some(("otherNodes", "field-ghost")),
        Some(("otherNodes", "edge-ghost")),
        &[],
    );
    let mut meter = Meter::new(UNLIMITED);
    let Err(CallFailure::Admission(failure)) = call(&observation, &mut meter) else {
        panic!("field closure defect must refuse")
    };
    let CallAdmissionFailure::Refused(record) = failure.as_ref() else {
        panic!("expected check-8 refusal")
    };
    assert_eq!(record.record.catalog.code(), "dangling_reference");
    assert_eq!(
        record.record.fields.get("object").map(String::as_str),
        Some("field-ghost")
    );
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 0);

    let observation = ep_observation_state(
        &package,
        all_spare,
        true,
        Some(("otherNodes", "field-child")),
        Some(("otherNodes", "edge-ghost")),
        &["field-child"],
    );
    let mut meter = Meter::new(UNLIMITED);
    let Err(CallFailure::Admission(failure)) = call(&observation, &mut meter) else {
        panic!("relationship closure defect must refuse")
    };
    let CallAdmissionFailure::Refused(record) = failure.as_ref() else {
        panic!("expected relationship check-8 refusal")
    };
    assert_eq!(record.record.catalog.code(), "dangling_reference");
    assert_eq!(
        record.record.fields.get("relationship").map(String::as_str),
        Some(model::id("relationship/otherLinks").as_str())
    );
    assert_eq!(
        record.record.fields.get("object").map(String::as_str),
        Some("edge-ghost")
    );
    assert_eq!(
        record.record.fields.get("instance").map(String::as_str),
        Some("0")
    );
    assert_eq!(
        record.record.fields.get("end").map(String::as_str),
        Some("target")
    );
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 0);

    let observation = ep_observation_state(
        &package,
        all_spare,
        true,
        Some(("otherNodes", "field-child")),
        Some(("otherNodes", "edge-child")),
        &["field-child", "edge-child"],
    );
    let mut meter = Meter::new(UNLIMITED);
    assert_value_eq!(completed(call(&observation, &mut meter).unwrap()), int(5));
}

#[test]
#[trace("TC-1070", "FR-390-AC-23")]
fn root_reference_occurrence_requires_the_exact_nodes_declaration() {
    let (package, view) = fixture_ep();
    let declaration = quire_checked::key::DeclarationKey {
        package: model::PACKAGE.to_owned(),
        node: model::id("Node"),
    };
    let reference = ObjectReference::new(
        view.object_universe_of(&declaration)
            .unwrap()
            .identity()
            .unwrap(),
        view.type_identities()[&declaration].clone(),
        ObjectId::new("n5").unwrap(),
    );
    let observation = ep_observation(&package, &view, None, true);
    let slot = slot(&package, "Pipeline", "Got", "related.from", Some(0));
    let invoke = |metadata: Vec<ProtocolReferenceOccurrence>, meter: &mut Meter| {
        package.evaluate_protocol_slot(
            &slot,
            &ProtocolSupply {
                binders: vec![(
                    binder(&package, "Pipeline", "view"),
                    ProtocolSuppliedValue {
                        value: SuppliedValue::Value(Value::Reference(reference.clone())),
                        references: metadata,
                    },
                )],
                observation: Some(&observation),
            },
            meter,
        )
    };
    let valid = ProtocolReferenceOccurrence {
        path: Vec::new(),
        population: population_node("nodes"),
    };
    let mut meter = Meter::new(UNLIMITED);
    assert!(
        matches!(completed(invoke(vec![valid.clone()], &mut meter).unwrap()), Value::Reference(actual) if actual == reference)
    );
    for (metadata, cause) in [
        (vec![], "missing-member"),
        (vec![valid.clone(), valid], "duplicate-member"),
    ] {
        let mut meter = Meter::new(UNLIMITED);
        let error = invoke(metadata, &mut meter).unwrap_err();
        let CallFailure::Admission(failure) = error else {
            panic!("root provenance mutation must refuse")
        };
        let CallAdmissionFailure::Refused(record) = failure.as_ref() else {
            panic!("root provenance mutation is a refusal")
        };
        assert_eq!(record.record.catalog.cause(), cause);
        assert!(record.record.path.is_empty());
        assert_eq!(meter.consumed(LimitKind::WorkUnits), 0);
    }
}

#[test]
#[trace("TC-1071", "FR-390-AC-6")]
fn observation_authority_is_bound_to_one_checked_package_instance() {
    let (first, first_view) = fixture_ep();
    let (second, second_view) = fixture_ep();
    let authority = ep_observation(&first, &first_view, None, true);
    let quire_semantic_value::admission::ObservationAdmission::Admitted(authority_value) =
        &authority
    else {
        panic!("the first package admits its snapshot")
    };
    assert!(authority_value.is_for_package(&first));
    assert!(!authority_value.is_for_package(&second));
    let reference = {
        let declaration = quire_checked::key::DeclarationKey {
            package: model::PACKAGE.to_owned(),
            node: model::id("Node"),
        };
        ObjectReference::new(
            second_view
                .object_universe_of(&declaration)
                .unwrap()
                .identity()
                .unwrap(),
            second_view.type_identities()[&declaration].clone(),
            ObjectId::new("n5").unwrap(),
        )
    };
    let mut meter = Meter::new(UNLIMITED);
    let failure = second
        .evaluate_protocol_slot(
            &slot(&second, "Pipeline", "Got", "related.from", Some(0)),
            &supply(
                &second,
                "Pipeline",
                vec![("view", Value::Reference(reference))],
                Some(&authority),
            ),
            &mut meter,
        )
        .unwrap_err();
    let CallFailure::Admission(failure) = failure else {
        panic!("cross-instance authority must refuse")
    };
    let CallAdmissionFailure::Refused(record) = failure.as_ref() else {
        panic!("cross-instance authority is a refusal")
    };
    assert_eq!(record.record.catalog.code(), "invalid_model_binding");
    assert_eq!(record.record.catalog.cause(), "wrong-model-selection");
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 0);
}

fn completed(result: ProtocolSlotEvaluation) -> Value {
    match result {
        ProtocolSlotEvaluation::Evaluated(
            quire_eval::Evaluation {
                outcome: FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(value)),
                ..
            },
            _,
        ) => value,
        other => panic!("expected completed slot value, got {other:?}"),
    }
}

fn observation<'a>(
    package: &'a CheckedPackage,
    view: &qsl_semantics::model::normalize::EffectiveView,
) -> (
    ObservationAdmission<qsl_package::AdmittedProtocolObservation<'a>>,
    ObjectReference,
) {
    observation_with_objects(package, view, &["n5"], true)
}

fn observation_with_objects<'a>(
    package: &'a CheckedPackage,
    view: &qsl_semantics::model::normalize::EffectiveView,
    keys: &[&str],
    complete: bool,
) -> (
    ObservationAdmission<qsl_package::AdmittedProtocolObservation<'a>>,
    ObjectReference,
) {
    let evaluation_view = package.evaluation_view();
    observation_through_view(package, &evaluation_view, view, keys, complete)
}

fn observation_through_view<'a>(
    package: &'a CheckedPackage,
    evaluation_view: &qsl_package::AdmittedEvaluationView<'a>,
    view: &qsl_semantics::model::normalize::EffectiveView,
    keys: &[&str],
    complete: bool,
) -> (
    ObservationAdmission<qsl_package::AdmittedProtocolObservation<'a>>,
    ObjectReference,
) {
    observation_with_row(package, evaluation_view, view, keys, Some(complete))
}

/// The `nodes` snapshot row: `None` leaves it out of the snapshot entirely,
/// `Some(complete)` lists it with that completeness.
fn observation_with_row<'a>(
    package: &'a CheckedPackage,
    evaluation_view: &qsl_package::AdmittedEvaluationView<'a>,
    view: &qsl_semantics::model::normalize::EffectiveView,
    keys: &[&str],
    row: Option<bool>,
) -> (
    ObservationAdmission<qsl_package::AdmittedProtocolObservation<'a>>,
    ObjectReference,
) {
    let key = quire_checked::key::DeclarationKey {
        package: model::PACKAGE.to_owned(),
        node: model::id("Node"),
    };
    let reference = ObjectReference::new(
        view.object_universe_of(&key).unwrap().identity().unwrap(),
        view.type_identities()[&key].clone(),
        ObjectId::new(keys[0]).unwrap(),
    );
    let domain_bytes = serde_json::to_vec(&document()).unwrap();
    let packages = qsl_semantics::model::intake::package_input([domain_bytes.as_slice()]).unwrap();
    let selected = DocumentRef {
        authority: "test".to_owned(),
        identity: "current".to_owned(),
    };
    let objects: Vec<_> = keys
        .iter()
        .enumerate()
        .map(|(index, object_key)| {
            serde_json::json!({
                "key": object_key,
                "type": model::id("Node"),
                "fields": {"total": {"integer": (5 + index).to_string()}},
            })
        })
        .collect();
    let populations: Vec<_> = row
        .map(|complete| {
            serde_json::json!({
                "population": model::id("nodes"),
                "complete": complete,
                "objects": objects,
            })
        })
        .into_iter()
        .collect();
    let snapshot = serde_json::json!({
        "format": "quire.state.snapshot/v1",
        "identity": {
            "authority": selected.authority.clone(),
            "identity": selected.identity.clone(),
            },
        "observation": "current",
        "model": {"identity": model::PACKAGE, "version": "1.0.0", "digest": "set at admission"},
        "populations": populations,
    });
    let snapshots = [(selected.clone(), serde_json::to_vec(&snapshot).unwrap())]
        .into_iter()
        .collect();
    let admission = evaluation_view.admit_protocol_observation(
        &package.graph().model_selections()[0],
        &packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        &snapshots,
        &selected,
        ObservationLimits::default(),
    );
    (admission, reference)
}

fn assert_reader_admits(package: &CheckedPackage) {
    let emission = qsl_package::emit_checked(package, EmitLimits::fixture())
        .expect("the complete E/L package emits");
    let bytes = serde_json::to_vec(&document()).unwrap();
    let input = qsl_semantics::model::intake::package_input([bytes.as_slice()]).unwrap();
    let mut admitted = qsl_package::AdmittedPackages::default();
    qsl_package::read_import_view(
        package,
        &emission,
        qsl_semantics::library::LibraryName::new(package.emitted_package_name().unwrap()).unwrap(),
        &input,
        &mut admitted,
        qsl_package::V2ReadLimits::fixture(),
        EmitLimits::fixture(),
    )
    .expect(
        "the actual contract reader admits the emitted related records before any slot evaluates",
    );
}

#[test]
#[trace("TC-1070", "FR-390-AC-1")]
fn activation_capture_and_both_related_ends_use_the_supplied_observation() {
    let (package, view) = fixture_with_view();
    assert_reader_admits(&package);
    let (observation, reference) = observation(&package, &view);
    let mut meter = Meter::new(UNLIMITED);
    let tick = record(
        &package,
        "Tick",
        vec![("ready", Value::Boolean(true)), ("count", int(2))],
    );
    assert_value_eq!(
        completed(
            package
                .evaluate_protocol_slot(
                    &slot(&package, "Refunds", "Refunds", "activation.guard", None),
                    &supply(&package, "Refunds", vec![("opened", tick)], None),
                    &mut meter
                )
                .unwrap()
        ),
        Value::Boolean(true)
    );
    assert_retain_points(&meter, &[], "activation guard (forwarded scalar)");
    assert_value_eq!(
        completed(
            package
                .evaluate_protocol_slot(
                    &slot(&package, "Refunds", "Refunds", "captures", Some(0)),
                    &supply(
                        &package,
                        "Refunds",
                        vec![("view", Value::Reference(reference.clone()))],
                        Some(&observation)
                    ),
                    &mut meter
                )
                .unwrap()
        ),
        int(5)
    );
    assert_retain_points(&meter, &[], "protocol capture (forwarded scalar)");
    for member in ["related.from", "related.to"] {
        let value = completed(
            package
                .evaluate_protocol_slot(
                    &slot(&package, "Pipeline", "Got", member, Some(0)),
                    &supply(
                        &package,
                        "Pipeline",
                        vec![("view", Value::Reference(reference.clone()))],
                        Some(&observation),
                    ),
                    &mut meter,
                )
                .unwrap(),
        );
        assert!(
            matches!(value, Value::Reference(actual) if actual == reference),
            "{member} returns the supplied endpoint"
        );
        assert_retain_points(&meter, &[], "related end (forwarded reference)");
    }
}

#[test]
#[trace("TC-1070", "FR-390-AC-1", "FR-390-AC-4")]
fn related_indexes_follow_the_emitted_record_order() {
    let source = SOURCE.replace("protocol Pipeline using P over (view: M::Node) on origin {", "protocol Pipeline using P over (view: M::Node) on origin {\n capture peer: M::Node = view;")
        .replace("related by Owns (view, view)", "related by Owns (view, peer) related by Owns (peer, view) related by Owns (peer, view)");
    let (package, view) = fixture_from_source(&source);
    assert_reader_admits(&package);
    let (_observation, n5) = observation(&package, &view);
    let n9 = ObjectReference::new(
        n5.universe().clone(),
        n5.object_type().clone(),
        ObjectId::new("n9").unwrap(),
    );
    let (observation_with_n9, _) = observation_with_objects(&package, &view, &["n5", "n9"], true);
    for (index, member, name, reference) in [
        (0, "related.from", "peer", &n9),
        (0, "related.to", "view", &n5),
        (1, "related.from", "view", &n5),
        (1, "related.to", "peer", &n9),
    ] {
        let value = completed(
            package
                .evaluate_protocol_slot(
                    &slot(&package, "Pipeline", "Got", member, Some(index)),
                    &supply(
                        &package,
                        "Pipeline",
                        vec![(name, Value::Reference(reference.clone()))],
                        Some(&observation_with_n9),
                    ),
                    &mut Meter::new(UNLIMITED),
                )
                .unwrap(),
        );
        assert!(
            matches!(value, Value::Reference(actual) if actual == *reference),
            "{member}[{index}] follows the canonical related record"
        );
    }
    assert_refused(
        &package,
        &slot(&package, "Pipeline", "Got", "related.from", Some(2)),
        &supply(&package, "Pipeline", vec![], None),
        CatalogCode::new("missing_declaration", "missing-name"),
        CallAdmissionLocus::Member {
            name: "related.from".to_owned(),
            index: Some(2),
        },
    );
}

fn set_limit(limits: &mut ScalarLimits, kind: LimitKind, bound: u64) {
    match kind {
        LimitKind::IntegerBits => limits.integer_bits = bound,
        LimitKind::DecimalDigits => limits.decimal_digits = bound,
        LimitKind::ScaleExpansion => limits.scale_expansion = bound,
        LimitKind::TextInputBytes => limits.text_input_bytes = bound,
        LimitKind::TextScalars => limits.text_scalars = bound,
        LimitKind::NormalizedScalars => limits.normalized_scalars = bound,
        LimitKind::UnitEdges => limits.unit_edges = bound,
        LimitKind::ValueOccurrences => limits.value_occurrences = bound,
        LimitKind::WorkUnits => limits.work_units = bound,
        LimitKind::ResultUnits => limits.result_units = bound,
    }
}

fn retry_supply(package: &CheckedPackage, first: i64) -> ProtocolSupply<'static> {
    supply(
        package,
        "Refunds",
        vec![
            ("earlier", attempt(package, first)),
            ("later", attempt(package, 1)),
            ("target", int(5)),
            ("activated", Value::Boolean(false)),
        ],
        None,
    )
}

fn assert_incomplete(result: ProtocolSlotEvaluation, counter: LimitKind) {
    match result {
        ProtocolSlotEvaluation::Evaluated(
            quire_eval::Evaluation {
                outcome: FamilyOutcome::Evaluated(quire_exact::Outcome::Incomplete(record)),
                ..
            },
            _,
        ) => assert_eq!(
            record.limit_kind, counter,
            "the denied counter is retained and no completed value is returned"
        ),
        other => panic!("expected {counter:?} exhaustion with no value: {other:?}"),
    }
}

fn measured_boundaries(
    package: &CheckedPackage,
    slot: &ProtocolSlot,
    supply: &ProtocolSupply<'_>,
    expected: Value,
) -> Meter {
    let mut measured = Meter::new(UNLIMITED);
    assert_value_eq!(
        completed(
            package
                .evaluate_protocol_slot(slot, supply, &mut measured)
                .unwrap()
        ),
        expected
    );
    let mut exact_limits = UNLIMITED;
    for counter in LimitKind::ALL {
        set_limit(&mut exact_limits, counter, measured.consumed(counter));
    }
    let mut exact = Meter::new(exact_limits);
    assert_value_eq!(
        completed(
            package
                .evaluate_protocol_slot(slot, supply, &mut exact)
                .unwrap()
        ),
        expected,
        "every counter at its measured usage completes"
    );
    assert_eq!(exact.admitted_charges(), measured.admitted_charges());
    for counter in LimitKind::ALL {
        let used = measured.consumed(counter);
        if used == 0 {
            continue;
        }
        let mut limits = exact_limits;
        set_limit(&mut limits, counter, used - 1);
        let mut short = Meter::new(limits);
        assert_incomplete(
            package
                .evaluate_protocol_slot(slot, supply, &mut short)
                .unwrap(),
            counter,
        );
        assert!(
            short.consumed(counter) < used,
            "the denied charge is not consumed"
        );
    }
    measured
}

fn assert_retain_points(meter: &Meter, expected: &[quire_exact::ChargePoint], call: &str) {
    let actual: Vec<_> = quire_exact::ChargePoint::ALL
        .into_iter()
        .filter(|point| point.as_str().ends_with(".result-retain"))
        .filter(|point| {
            meter
                .admitted_charges()
                .iter()
                .any(|charge| charge.point == *point)
        })
        .collect();
    assert_eq!(
        actual, expected,
        "{call} reaches exactly its existing S6a retain-point kinds"
    );
    if expected.is_empty() {
        assert_eq!(
            meter.consumed(LimitKind::ResultUnits),
            0,
            "{call}: result_units is zero by S6a, independently of measured boundary coverage"
        );
    } else {
        assert!(
            meter.consumed(LimitKind::ResultUnits) > 0,
            "{call}: existing retain points charge result_units"
        );
    }
}

#[test]
#[trace("TC-1073", "FR-390-AC-9", "FR-390-AC-11")]
// AC-11's repeated-call and refusal properties only; contract-reader parity
// requires an admitted package with the evaluator's checked-term interface.
fn retry_and_observation_capture_obey_every_measured_counter_boundary() {
    let (package, view) = fixture_with_view();
    let retry = slot(&package, "Refunds", "Refund", "retry.relation", None);
    let values = retry_supply(&package, 0);
    let measured = measured_boundaries(&package, &retry, &values, Value::Boolean(true));
    use quire_exact::ChargePoint::{
        BooleanResultRetain, EqualityResultRetain, IntegerArithmeticResultRetain,
        OrderingResultRetain,
    };
    assert_retain_points(
        &measured,
        &[
            EqualityResultRetain,
            IntegerArithmeticResultRetain,
            OrderingResultRetain,
            BooleanResultRetain,
        ],
        "T",
    );
    let expected_counters = vec![
        LimitKind::IntegerBits,
        LimitKind::ValueOccurrences,
        LimitKind::WorkUnits,
        LimitKind::ResultUnits,
    ];
    assert_eq!(LimitKind::ALL.into_iter().filter(|counter| measured.consumed(*counter) > 0).collect::<Vec<_>>(), expected_counters, "T's charged counters are independently required by its arithmetic, comparisons and completed result");
    let mut repeated = Meter::new(UNLIMITED);
    assert_value_eq!(
        completed(
            package
                .evaluate_protocol_slot(&retry, &values, &mut repeated)
                .unwrap()
        ),
        Value::Boolean(true)
    );
    assert_eq!(
        repeated.admitted_charges(),
        measured.admitted_charges(),
        "the same call has the same ordered charges"
    );
    let mut early = Meter::new(UNLIMITED);
    assert_value_eq!(
        completed(
            package
                .evaluate_protocol_slot(&retry, &retry_supply(&package, 3), &mut early)
                .unwrap()
        ),
        Value::Boolean(false)
    );
    assert!(
        early.consumed(LimitKind::WorkUnits) < measured.consumed(LimitKind::WorkUnits),
        "a first-operand refusal skips the remaining retry expression"
    );
    assert_retain_points(
        &early,
        &[OrderingResultRetain, BooleanResultRetain],
        "short-circuited T",
    );
    let mut zero = UNLIMITED;
    zero.work_units = 0;
    assert_incomplete(
        package
            .evaluate_protocol_slot(&retry, &values, &mut Meter::new(zero))
            .unwrap(),
        LimitKind::WorkUnits,
    );
    let (observation, reference) = observation(&package, &view);
    let capture = slot(&package, "Refunds", "Refunds", "captures", Some(0));
    let values = supply(
        &package,
        "Refunds",
        vec![("view", Value::Reference(reference))],
        Some(&observation),
    );
    let captured = measured_boundaries(&package, &capture, &values, int(5));
    assert_retain_points(&captured, &[], "C (forwarded scalar)");
    assert_eq!(
        LimitKind::ALL
            .into_iter()
            .filter(|counter| captured.consumed(*counter) > 0)
            .collect::<Vec<_>>(),
        vec![LimitKind::ValueOccurrences, LimitKind::WorkUnits],
        "C's observation read charges its value occurrence and work"
    );
    assert!(
        captured
            .admitted_charges()
            .iter()
            .any(|charge| charge.point == quire_exact::ChargePoint::ModelNavigate),
        "C actually navigates the observation"
    );
    assert!(
        !captured
            .admitted_charges()
            .iter()
            .any(|charge| charge.point == quire_exact::ChargePoint::ModelDeref),
        "a bare view.total read contains no explicit dereference"
    );
    let mut after_refusal = Meter::new(UNLIMITED);
    let before = after_refusal.clone();
    let mut missing = retry_supply(&package, 0);
    missing
        .binders
        .retain(|(key, _)| *key != binder(&package, "Refunds", "activated"));
    assert!(matches!(
        package.evaluate_protocol_slot(&retry, &missing, &mut after_refusal),
        Err(CallFailure::Admission(_))
    ));
    assert_eq!(after_refusal, before);
    assert_value_eq!(
        completed(
            package
                .evaluate_protocol_slot(&retry, &retry_supply(&package, 0), &mut after_refusal)
                .unwrap()
        ),
        Value::Boolean(true)
    );
    assert_eq!(
        after_refusal.admitted_charges(),
        measured.admitted_charges()
    );
    let mut constant_meter = Meter::new(UNLIMITED);
    assert_value_eq!(
        completed(
            package
                .evaluate_protocol_slot(
                    &slot(&package, "Refunds", "Closed", "constraint", None),
                    &supply(&package, "Refunds", vec![], None),
                    &mut constant_meter
                )
                .unwrap()
        ),
        Value::Boolean(true)
    );
    assert_retain_points(&constant_meter, &[], "Closed.constraint (literal constant)");
}

#[test]
#[trace("TC-1073", "FR-390-AC-9")]
fn an_explicit_dereference_capture_obeys_the_observation_charge_boundaries() {
    let (package, view) =
        fixture_from_source(&SOURCE.replace("= view.total;", "= deref(view).total;"));
    let (observation, reference) = observation(&package, &view);
    let capture = slot(&package, "Refunds", "Refunds", "captures", Some(0));
    let values = supply(
        &package,
        "Refunds",
        vec![("view", Value::Reference(reference))],
        Some(&observation),
    );
    let measured = measured_boundaries(&package, &capture, &values, int(5));
    assert_retain_points(
        &measured,
        &[],
        "D (explicit dereference of a forwarded scalar)",
    );
    let model_points: Vec<_> = measured
        .admitted_charges()
        .iter()
        .filter_map(|charge| {
            matches!(
                charge.point,
                quire_exact::ChargePoint::ModelDeref | quire_exact::ChargePoint::ModelNavigate
            )
            .then_some(charge.point)
        })
        .collect();
    assert_eq!(
        model_points,
        [
            quire_exact::ChargePoint::ModelDeref,
            quire_exact::ChargePoint::ModelNavigate
        ]
    );
    for charge in measured.admitted_charges().iter().filter(|charge| {
        matches!(
            charge.point,
            quire_exact::ChargePoint::ModelDeref | quire_exact::ChargePoint::ModelNavigate
        )
    }) {
        assert_eq!(charge.value_occurrences, Some(1), "the normative model read vector belongs to each charge, including the explicit dereference");
    }
}

#[test]
#[trace("TC-1073", "FR-390-AC-10")]
fn registration_activation_trigger_retry_and_recovery_share_the_sum_in_both_orders() {
    let package = fixture();
    let calls = [
        (
            slot(
                &package,
                "Refunds",
                "Refund",
                "registration_captures",
                Some(0),
            ),
            supply(
                &package,
                "Refunds",
                vec![("forward", receipt(&package, 5))],
                None,
            ),
            int(5),
        ),
        (
            slot(
                &package,
                "Refunds",
                "Refund",
                "activation_captures",
                Some(0),
            ),
            supply(
                &package,
                "Refunds",
                vec![("trigger", flag(&package, true))],
                None,
            ),
            Value::Boolean(true),
        ),
        (
            slot(&package, "Refunds", "Refund", "trigger.guard", None),
            supply(
                &package,
                "Refunds",
                vec![("trigger", flag(&package, false)), ("target", int(5))],
                None,
            ),
            Value::Boolean(true),
        ),
        (
            slot(&package, "Refunds", "Refund", "retry.relation", None),
            retry_supply(&package, 0),
            Value::Boolean(true),
        ),
        (
            slot(&package, "Refunds", "Refund", "recovery.predicate", None),
            supply(
                &package,
                "Refunds",
                vec![
                    ("recovered", receipt(&package, 5)),
                    ("target", int(5)),
                    ("activated", Value::Boolean(false)),
                ],
                None,
            ),
            Value::Boolean(true),
        ),
    ];
    let measured: Vec<_> = calls
        .iter()
        .map(|(slot, supply, expected)| {
            let mut meter = Meter::new(UNLIMITED);
            assert_value_eq!(
                completed(
                    package
                        .evaluate_protocol_slot(slot, supply, &mut meter)
                        .unwrap()
                ),
                *expected
            );
            meter
        })
        .collect();
    use quire_exact::ChargePoint::{
        BooleanResultRetain, EqualityResultRetain, IntegerArithmeticResultRetain,
        OrderingResultRetain,
    };
    let retain_points: &[(&str, &[quire_exact::ChargePoint])] = &[
        ("R (forwarded scalar)", &[]),
        ("A (forwarded scalar)", &[]),
        ("G", &[OrderingResultRetain, BooleanResultRetain]),
        (
            "T",
            &[
                EqualityResultRetain,
                IntegerArithmeticResultRetain,
                OrderingResultRetain,
                BooleanResultRetain,
            ],
        ),
        ("V", &[EqualityResultRetain, BooleanResultRetain]),
    ];
    for ((call, expected), meter) in retain_points.iter().zip(&measured) {
        assert_retain_points(meter, expected, call);
    }
    let sum: u64 = measured
        .iter()
        .map(|meter| meter.consumed(LimitKind::WorkUnits))
        .sum();
    let mut limits = UNLIMITED;
    limits.work_units = sum;
    let mut forward = Meter::new(limits);
    for (slot, supply, expected) in &calls {
        assert_value_eq!(
            completed(
                package
                    .evaluate_protocol_slot(slot, supply, &mut forward)
                    .unwrap()
            ),
            *expected
        );
    }
    assert_eq!(forward.consumed(LimitKind::WorkUnits), sum);
    let expected_log: Vec<_> = measured
        .iter()
        .flat_map(|meter| meter.admitted_charges().iter().cloned())
        .collect();
    assert_eq!(
        forward.admitted_charges(),
        expected_log,
        "five independent measured logs concatenate without resetting or reevaluating captures"
    );
    let mut reverse = Meter::new(limits);
    for (slot, supply, expected) in calls.iter().rev() {
        assert_value_eq!(
            completed(
                package
                    .evaluate_protocol_slot(slot, supply, &mut reverse)
                    .unwrap()
            ),
            *expected
        );
    }
    for counter in LimitKind::ALL {
        assert_eq!(
            reverse.consumed(counter),
            forward.consumed(counter),
            "order-independent {counter:?} usage"
        );
    }
    let reverse_log: Vec<_> = measured
        .iter()
        .rev()
        .flat_map(|meter| meter.admitted_charges().iter().cloned())
        .collect();
    assert_eq!(reverse.admitted_charges(), reverse_log);
    limits.work_units = sum - 1;
    let mut short = Meter::new(limits);
    for (slot, supply, expected) in &calls[..4] {
        assert_value_eq!(
            completed(
                package
                    .evaluate_protocol_slot(slot, supply, &mut short)
                    .unwrap()
            ),
            *expected
        );
    }
    assert_incomplete(
        package
            .evaluate_protocol_slot(&calls[4].0, &calls[4].1, &mut short)
            .unwrap(),
        LimitKind::WorkUnits,
    );
    assert_eq!(short.consumed(LimitKind::WorkUnits), sum - 1);
    let prefix: Vec<_> = measured[..4]
        .iter()
        .flat_map(|meter| meter.admitted_charges().iter().cloned())
        .collect();
    assert!(short.admitted_charges().starts_with(&prefix));
    assert!(
        measured[4]
            .admitted_charges()
            .starts_with(&short.admitted_charges()[prefix.len()..]),
        "V retains only its charges before denial"
    );
}

#[test]
#[trace("TC-1070", "FR-390-AC-1", "FR-390-AC-2")]
fn both_templates_and_the_remaining_run_slots_evaluate_their_own_expressions() {
    let package = fixture();
    let mut meter = Meter::new(UNLIMITED);
    for (owner, prefix) in [("Refund", ""), ("Partial", "partial")] {
        let name = |stem: &str| {
            if prefix.is_empty() {
                stem.to_owned()
            } else {
                format!("partial{}{}", stem[..1].to_uppercase(), &stem[1..])
            }
        };
        let mut run = |member, index, values: Vec<(&str, Value)>| {
            completed(
                package
                    .evaluate_protocol_slot(
                        &slot(&package, "Refunds", owner, member, index),
                        &supply(&package, "Refunds", values, None),
                        &mut meter,
                    )
                    .unwrap(),
            )
        };
        let forward = name("forward");
        let target = name("target");
        let trigger = name("trigger");
        let activated = name("activated");
        let earlier = name("earlier");
        let later = name("later");
        let recovered = name("recovered");
        let saved_five = run(
            "registration_captures",
            Some(0),
            vec![(&forward, receipt(&package, 5))],
        );
        assert_value_eq!(saved_five, int(5));
        for ready in [false, true] {
            assert_value_eq!(
                run(
                    "trigger.guard",
                    None,
                    vec![(&trigger, flag(&package, ready)), (&target, int(5))]
                ),
                Value::Boolean(!ready)
            );
            assert_value_eq!(
                run(
                    "activation_captures",
                    Some(0),
                    vec![(&trigger, flag(&package, ready))]
                ),
                Value::Boolean(ready)
            );
        }
        for (first, second, active, expected) in [
            (0, 1, false, true),
            (0, 1, true, false),
            (3, 1, false, false),
            (0, 0, false, false),
        ] {
            assert_value_eq!(
                run(
                    "retry.relation",
                    None,
                    vec![
                        (&earlier, attempt(&package, first)),
                        (&later, attempt(&package, second)),
                        (&target, int(5)),
                        (&activated, Value::Boolean(active))
                    ]
                ),
                Value::Boolean(expected)
            );
        }
        for (total, saved, active, expected) in [
            (5, 5, false, true),
            (9, 5, false, false),
            (5, 5, true, false),
            (9, 9, false, true),
        ] {
            assert_value_eq!(
                run(
                    "recovery.predicate",
                    None,
                    vec![
                        (&recovered, receipt(&package, total)),
                        (&target, int(saved)),
                        (&activated, Value::Boolean(active))
                    ]
                ),
                Value::Boolean(expected)
            );
        }
        let current_nine = run(
            "registration_captures",
            Some(0),
            vec![(&forward, receipt(&package, 9))],
        );
        assert_value_eq!(current_nine, int(9));
        assert_value_eq!(
            run(
                "recovery.predicate",
                None,
                vec![
                    (&recovered, receipt(&package, 9)),
                    (&target, saved_five),
                    (&activated, Value::Boolean(false))
                ]
            ),
            Value::Boolean(false),
            "a later registration result does not replace the supplied frozen five"
        );
        assert_value_eq!(
            run(
                "recovery.predicate",
                None,
                vec![
                    (&recovered, receipt(&package, 9)),
                    (&target, current_nine),
                    (&activated, Value::Boolean(false))
                ]
            ),
            Value::Boolean(true),
            "supplying the new captured nine changes the result without recomputation"
        );
    }
    for (owner, values) in [
        ("Applied", vec![("order", int(5))]),
        ("Committed", vec![("committed", flag(&package, true))]),
        ("Closed", vec![]),
    ] {
        let mut row_meter = Meter::new(UNLIMITED);
        assert_value_eq!(
            completed(
                package
                    .evaluate_protocol_slot(
                        &slot(&package, "Refunds", owner, "constraint", None),
                        &supply(&package, "Refunds", values, None),
                        &mut row_meter
                    )
                    .unwrap()
            ),
            Value::Boolean(true)
        );
        let points = if owner == "Applied" {
            &[quire_exact::ChargePoint::OrderingResultRetain][..]
        } else {
            &[]
        };
        assert_retain_points(&row_meter, points, owner);
    }
    for ready in [true, false] {
        for (member, index, expected) in [
            ("visible", 0, ready),
            // Case indexes follow the emitted UTF-8 label order: no, yes.
            ("cases.guard", 0, !ready),
            ("cases.guard", 1, ready),
        ] {
            let mut row_meter = Meter::new(UNLIMITED);
            assert_value_eq!(
                completed(
                    package
                        .evaluate_protocol_slot(
                            &slot(&package, "Pipeline", "Decide", member, Some(index)),
                            &supply(
                                &package,
                                "Pipeline",
                                vec![("got", flag(&package, ready))],
                                None
                            ),
                            &mut row_meter
                        )
                        .unwrap()
                ),
                Value::Boolean(expected)
            );
            let points = if member == "cases.guard" && index == 0 {
                &[quire_exact::ChargePoint::BooleanResultRetain][..]
            } else {
                &[]
            };
            assert_retain_points(&row_meter, points, member);
        }
    }
}

fn assert_refused(
    package: &CheckedPackage,
    slot: &ProtocolSlot,
    supply: &ProtocolSupply<'_>,
    code: CatalogCode,
    locus: CallAdmissionLocus,
) {
    let mut meter = Meter::new(UNLIMITED);
    let before = meter.clone();
    let failure = package.evaluate_protocol_slot(slot, supply, &mut meter);
    assert_eq!(meter, before, "admission refusal must charge nothing");
    let Err(CallFailure::Admission(payload)) = failure else {
        panic!("expected typed admission refusal: {failure:?}");
    };
    assert!(
        matches!(payload.as_ref(), CallAdmissionFailure::Refused(record) if record.record.catalog == code && record.locus == locus),
        "expected {code:?} at {locus:?}, got {payload:?}"
    );
}

#[test]
#[trace("TC-1071", "FR-390-AC-4")]
fn absent_members_and_each_index_defect_refuse_without_evaluating_a_neighbor() {
    let package = fixture_from_source(&SOURCE.replace(
        "  run sequence Main {",
        "  terminal any;\n  run sequence Main {",
    ))
    .0;
    let empty = supply(&package, "Refunds", vec![], None);
    for (owner, member, index) in [
        ("Refunds", "retry.relation", None),
        ("Refund", "registration_captures", Some(1)),
        ("Refund", "registration_captures", None),
        ("Refund", "retry.relation", Some(0)),
        ("Closed", "condition", None),
    ] {
        assert_refused(
            &package,
            &slot(&package, "Refunds", owner, member, index),
            &empty,
            CatalogCode::new("missing_declaration", "missing-name"),
            CallAdmissionLocus::Member {
                name: member.to_owned(),
                index,
            },
        );
    }
    let terminal = ProtocolSlot {
        node: package
            .graph()
            .protocol_node_keys("Refunds")
            .unwrap()
            .terminal
            .clone()
            .expect("terminal any emits its obligation"),
        member: "condition".to_owned(),
        index: None,
    };
    assert_refused(
        &package,
        &terminal,
        &empty,
        CatalogCode::new("missing_declaration", "missing-name"),
        CallAdmissionLocus::Member {
            name: "condition".to_owned(),
            index: None,
        },
    );
}

#[test]
#[trace("TC-1070", "TC-1071", "FR-390-AC-3", "FR-390-AC-5", "FR-390-AC-6")]
// AC-6 covers record shape and scalar domain here; observation refusals have
// their own cases and are not implied by this test.
fn crossed_template_sources_and_admission_precedence_name_the_exact_binder() {
    let package = fixture();
    let retry = slot(&package, "Refunds", "Refund", "retry.relation", None);
    let invalid = |name: &str| CallAdmissionLocus::Binder(binder(&package, "Refunds", name));
    let mut duplicate = retry_supply(&package, 0);
    duplicate.binders.push((
        binder(&package, "Refunds", "earlier"),
        SuppliedValue::Value(attempt(&package, 0)).into(),
    ));
    duplicate.binders.push((
        binder(&package, "Refunds", "partialTarget"),
        SuppliedValue::Value(int(5)).into(),
    ));
    duplicate
        .binders
        .retain(|(key, _)| *key != binder(&package, "Refunds", "activated"));
    assert_refused(
        &package,
        &retry,
        &duplicate,
        CatalogCode::new("invalid_runtime_input", "duplicate-member"),
        invalid("earlier"),
    );
    for unknown in ["forward", "partialTarget"] {
        let mut values = retry_supply(&package, 0);
        values.binders.push((
            binder(&package, "Refunds", unknown),
            SuppliedValue::Value(if unknown == "forward" {
                receipt(&package, 5)
            } else {
                int(5)
            })
            .into(),
        ));
        assert_refused(
            &package,
            &retry,
            &values,
            CatalogCode::new("invalid_runtime_input", "unknown-member"),
            invalid(unknown),
        );
    }
    for names in [["activated", "order"], ["order", "activated"]] {
        let values = supply(
            &package,
            "Refunds",
            names
                .into_iter()
                .map(|name| {
                    (
                        name,
                        if name == "activated" {
                            Value::Boolean(false)
                        } else {
                            int(5)
                        },
                    )
                })
                .collect(),
            None,
        );
        assert_refused(
            &package,
            &slot(
                &package,
                "Refunds",
                "Refund",
                "registration_captures",
                Some(0),
            ),
            &values,
            CatalogCode::new("invalid_runtime_input", "unknown-member"),
            invalid("order"),
        );
    }
    let crossed_trigger = supply(
        &package,
        "Refunds",
        vec![
            ("target", int(5)),
            ("partialTrigger", flag(&package, false)),
        ],
        None,
    );
    assert_refused(
        &package,
        &slot(&package, "Refunds", "Refund", "trigger.guard", None),
        &crossed_trigger,
        CatalogCode::new("invalid_runtime_input", "unknown-member"),
        invalid("partialTrigger"),
    );
    for (omitted, first) in [
        (vec!["activated"], "activated"),
        (vec!["activated", "target"], "target"),
    ] {
        let mut values = retry_supply(&package, 0);
        values.binders.retain(|(key, _)| {
            !omitted
                .iter()
                .any(|name| *key == binder(&package, "Refunds", name))
        });
        assert_refused(
            &package,
            &retry,
            &values,
            CatalogCode::new("invalid_runtime_input", "missing-member"),
            invalid(first),
        );
    }
    for (owner, member, index, first) in [
        ("Refund", "retry.relation", None, "target"),
        ("Refund", "trigger.guard", None, "target"),
        ("Refund", "recovery.predicate", None, "target"),
        ("Partial", "retry.relation", None, "partialTarget"),
        ("Partial", "recovery.predicate", None, "partialTarget"),
        ("Refund", "registration_captures", Some(0), "forward"),
        ("Refund", "activation_captures", Some(0), "trigger"),
    ] {
        assert_refused(
            &package,
            &slot(&package, "Refunds", owner, member, index),
            &supply(&package, "Refunds", vec![], None),
            CatalogCode::new("invalid_runtime_input", "missing-member"),
            invalid(first),
        );
    }
    for (name, value, cause) in [
        ("target", Value::Boolean(true), "wrong-value-kind"),
        ("earlier", flag(&package, true), "wrong-value-kind"),
        ("target", int(1001), "invalid-value"),
    ] {
        let mut values = retry_supply(&package, 0);
        let key = binder(&package, "Refunds", name);
        values
            .binders
            .iter_mut()
            .find(|(candidate, _)| *candidate == key)
            .unwrap()
            .1 = SuppliedValue::Value(value).into();
        assert_refused(
            &package,
            &retry,
            &values,
            CatalogCode::new("invalid_runtime_input", cause),
            invalid(name),
        );
    }
}

/// TC-1071 rows 17-19 and 22: reference admission precedes every charge;
/// the same meter remains usable after each refused or incomplete admission.
#[test]
#[trace("TC-1071", "FR-390-AC-6", "FR-390-AC-22")]
fn reference_admission_refusals_leave_the_shared_meter_available() {
    let (package, view) = fixture_with_view();
    let (observation, reference) = observation(&package, &view);
    let capture = slot(&package, "Refunds", "Refunds", "captures", Some(0));
    let ghost = ObjectReference::new(
        reference.universe().clone(),
        reference.object_type().clone(),
        ObjectId::new("ghost").unwrap(),
    );
    let foreign_document = serde_json::to_string(&document())
        .unwrap()
        .replace("test/orders", "test/foreign");
    let (_foreign_package, foreign_view) = fixture_from_document(
        &SOURCE.replace("test/orders", "test/foreign"),
        foreign_document.as_bytes(),
    );
    let foreign_key = quire_checked::key::DeclarationKey {
        package: "test/foreign".to_owned(),
        node: "ix://test/foreign/Node".to_owned(),
    };
    let foreign = ObjectReference::new(
        foreign_view
            .object_universe_of(&foreign_key)
            .unwrap()
            .identity()
            .unwrap(),
        foreign_view.type_identities()[&foreign_key].clone(),
        ObjectId::new("n5").unwrap(),
    );
    assert_ne!(foreign.universe(), reference.universe());
    let mut meter = Meter::new(UNLIMITED);
    for (value, observed, code, incomplete) in [
        (
            ghost,
            Some(&observation),
            CatalogCode::new("dangling_reference", "absent-target-in-complete-population"),
            false,
        ),
        (
            foreign,
            Some(&observation),
            CatalogCode::new("foreign_reference", "foreign-universe"),
            false,
        ),
        (
            reference,
            None,
            CatalogCode::new("unavailable_observation", "missing-required-artifact"),
            true,
        ),
    ] {
        let before = meter.clone();
        let result = package.evaluate_protocol_slot(
            &capture,
            &supply(
                &package,
                "Refunds",
                vec![("view", Value::Reference(value))],
                observed,
            ),
            &mut meter,
        );
        assert_eq!(
            meter, before,
            "reference admission leaves every charge unchanged"
        );
        let Err(CallFailure::Admission(failure)) = result else {
            panic!("expected typed reference admission failure: {result:?}");
        };
        let record = match failure.as_ref() {
            CallAdmissionFailure::Incomplete(record) if incomplete => record,
            CallAdmissionFailure::Refused(record) if !incomplete => record,
            other => panic!("wrong admission outcome: {other:?}"),
        };
        assert_eq!(
            record.record.catalog.code(),
            code.code(),
            "observed cause: {}",
            record.record.catalog.cause()
        );
        assert_eq!(
            record.locus,
            CallAdmissionLocus::Binder(binder(&package, "Refunds", "view"))
        );
    }
    let retry = slot(&package, "Refunds", "Refund", "retry.relation", None);
    let mut fresh = Meter::new(UNLIMITED);
    let values = retry_supply(&package, 0);
    assert_value_eq!(
        completed(
            package
                .evaluate_protocol_slot(&retry, &values, &mut fresh)
                .unwrap()
        ),
        Value::Boolean(true)
    );
    assert_value_eq!(
        completed(
            package
                .evaluate_protocol_slot(&retry, &values, &mut meter)
                .unwrap()
        ),
        Value::Boolean(true)
    );
    assert_eq!(
        meter, fresh,
        "only the final valid recovery call charges the shared meter"
    );
}

/// FR-390-AC-22: malformed slots and supplied-source defects take precedence
/// over a failed observation result, and each refusal leaves the meter intact.
#[test]
#[trace("TC-1071", "FR-390-AC-22")]
fn source_defects_precede_a_refused_observation_result() {
    use quire_semantic_value::admission::AdmissionRecord;

    let package = fixture();
    let refused_record = AdmissionRecord {
        catalog: CatalogCode::new("invalid_model_binding", "wrong-model-selection"),
        fields: Default::default(),
        path: Vec::new(),
        kernel: None,
    };
    let refused = ObservationAdmission::Refused(refused_record);
    let retry = slot(&package, "Refunds", "Refund", "retry.relation", None);
    let locus = |name: &str| CallAdmissionLocus::Binder(binder(&package, "Refunds", name));
    let mut duplicate = retry_supply(&package, 0);
    duplicate.binders.push((
        binder(&package, "Refunds", "earlier"),
        SuppliedValue::Value(attempt(&package, 0)).into(),
    ));
    duplicate.observation = Some(&refused);

    let mut unknown = retry_supply(&package, 0);
    unknown.binders.push((
        binder(&package, "Refunds", "forward"),
        SuppliedValue::Value(receipt(&package, 5)).into(),
    ));
    unknown.observation = Some(&refused);

    let mut missing = retry_supply(&package, 0);
    missing
        .binders
        .retain(|(key, _)| *key != binder(&package, "Refunds", "target"));
    missing.observation = Some(&refused);

    let mut wrong_shape = retry_supply(&package, 0);
    let target = binder(&package, "Refunds", "target");
    wrong_shape
        .binders
        .iter_mut()
        .find(|(key, _)| *key == target)
        .expect("the target source is supplied")
        .1 = SuppliedValue::Value(Value::Boolean(true)).into();
    wrong_shape.observation = Some(&refused);

    let malformed_slot = ProtocolSlot {
        node: binder(&package, "Refunds", "target"),
        member: "retry.relation".to_owned(),
        index: None,
    };
    let empty = supply(&package, "Refunds", Vec::new(), Some(&refused));
    let cases = [
        (
            retry.clone(),
            duplicate,
            CatalogCode::new("invalid_runtime_input", "duplicate-member"),
            locus("earlier"),
        ),
        (
            retry.clone(),
            unknown,
            CatalogCode::new("invalid_runtime_input", "unknown-member"),
            locus("forward"),
        ),
        (
            retry.clone(),
            missing,
            CatalogCode::new("invalid_runtime_input", "missing-member"),
            locus("target"),
        ),
        (
            retry,
            wrong_shape,
            CatalogCode::new("invalid_runtime_input", "wrong-value-kind"),
            locus("target"),
        ),
        (
            malformed_slot.clone(),
            empty,
            CatalogCode::new("invalid_model_binding", "malformed-declaration"),
            CallAdmissionLocus::Node(malformed_slot.node),
        ),
    ];
    for (selected, supplied, expected, expected_locus) in cases {
        let mut meter = Meter::new(UNLIMITED);
        let before = meter.clone();
        let result = package.evaluate_protocol_slot(&selected, &supplied, &mut meter);
        assert_eq!(meter, before, "a pre-observation refusal charges nothing");
        let Err(CallFailure::Admission(failure)) = result else {
            panic!("input defect precedes the refused observation: {result:?}");
        };
        let CallAdmissionFailure::Refused(record) = failure.as_ref() else {
            panic!("the input defect remains a refusal: {failure:?}");
        };
        assert_eq!(record.record.catalog, expected);
        assert_eq!(record.locus, expected_locus);
    }
}

/// TC-1071 rows 23–25: the retained strict-reader view admits and evaluates
/// the same checked protocol term, while foreign and dangling references
/// refuse before changing any meter counter.
#[test]
#[trace("TC-1071", "FR-390-AC-6", "FR-390-AC-14")]
fn reader_backed_protocol_slot_uses_retained_view_and_preserves_refusal_evidence() {
    let (package, model_view) = fixture_with_view();
    let domain_bytes = serde_json::to_vec(&document()).unwrap();
    let domain_packages =
        qsl_semantics::model::intake::package_input([domain_bytes.as_slice()]).unwrap();
    let read = qsl_package::read_evaluation_package(
        &package,
        qsl_semantics::library::LibraryName::new("acceptance").unwrap(),
        &domain_packages,
        qsl_package::V2ReadLimits::fixture(),
        EmitLimits::fixture(),
    )
    .expect("strict contract-reader admission succeeds for checked fixture E");
    let reader_view = read
        .evaluation_view()
        .expect("the admitted graph is bound to its emitting checked package");
    assert!(reader_view.is_reader_backed());
    let second_read = qsl_package::read_evaluation_package(
        &package,
        qsl_semantics::library::LibraryName::new("acceptance").unwrap(),
        &domain_packages,
        qsl_package::V2ReadLimits::fixture(),
        EmitLimits::fixture(),
    )
    .expect("a second strict reader independently admits the same source package");
    let second_reader_view = second_read
        .evaluation_view()
        .expect("the second admitted graph is bound to the same source package");
    let (observation, reference) =
        observation_through_view(&package, &reader_view, &model_view, &["n5"], true);
    let capture = slot(&package, "Refunds", "Refunds", "captures", Some(0));
    let mut meter = Meter::new(UNLIMITED);

    let selected = reader_view
        .select_protocol_slot(&qsl_package::ProtocolSlotDescriptor::new(
            capture.node.clone(),
            capture.member.clone(),
            capture.index,
        ))
        .expect("reader view selects the capture term");
    assert!(
        matches!(
            selected.reader_slot(),
            Some(qsl_package::ReaderSlot::Projected(_))
        ),
        "reader view owns the root projection"
    );
    let unavailable = reader_view
        .evaluate_protocol_slot(
            &capture,
            &ProtocolSupply {
                binders: vec![(
                    binder(&package, "Refunds", "view"),
                    SuppliedValue::Unavailable(CatalogCode::new(
                        "unavailable_observation",
                        "missing-required-artifact",
                    ))
                    .into(),
                )],
                observation: None,
            },
            &mut meter,
        )
        .expect("unavailable supplied source is returned as a protocol outcome");
    let ProtocolSlotEvaluation::Unavailable {
        binder: reader_binder,
        cause: reader_cause,
        location,
        ..
    } = unavailable
    else {
        panic!("reader-backed unavailable source must halt at its read: {unavailable:?}");
    };
    assert_eq!(
        location, None,
        "the reader halt has no location and never takes the source package's"
    );
    let checked_unavailable = package
        .evaluate_protocol_slot(
            &capture,
            &ProtocolSupply {
                binders: vec![(
                    binder(&package, "Refunds", "view"),
                    SuppliedValue::Unavailable(CatalogCode::new(
                        "unavailable_observation",
                        "missing-required-artifact",
                    ))
                    .into(),
                )],
                observation: None,
            },
            &mut Meter::new(UNLIMITED),
        )
        .expect("the checked view returns the same unavailable outcome");
    let ProtocolSlotEvaluation::Unavailable {
        binder: checked_binder,
        cause: checked_cause,
        location: checked_location,
        ..
    } = checked_unavailable
    else {
        panic!("the checked view halts at the unavailable read");
    };
    assert!(checked_location.is_some(), "the in-process halt is located");
    assert_eq!(
        (reader_binder, reader_cause),
        (checked_binder, checked_cause),
        "the same unavailable binder and cause"
    );

    let before = meter.clone();
    let mismatched = second_reader_view.evaluate_protocol_slot(
        &capture,
        &supply(
            &package,
            "Refunds",
            vec![("view", Value::Reference(reference.clone()))],
            Some(&observation),
        ),
        &mut meter,
    );
    assert_eq!(
        meter, before,
        "a foreign view cannot consume evaluation budget"
    );
    let Err(CallFailure::Admission(failure)) = mismatched else {
        panic!("a reader authority from another view must refuse: {mismatched:?}");
    };
    let CallAdmissionFailure::Refused(record) = failure.as_ref() else {
        panic!("a mismatched view is a typed refusal: {failure:?}");
    };
    assert_eq!(record.record.catalog.code(), "invalid_model_binding");
    assert_eq!(record.record.catalog.cause(), "wrong-model-selection");

    assert_value_eq!(
        completed(
            reader_view
                .evaluate_protocol_slot(
                    &capture,
                    &supply(
                        &package,
                        "Refunds",
                        vec![("view", Value::Reference(reference.clone()))],
                        Some(&observation),
                    ),
                    &mut meter,
                )
                .expect("the reader-retained view evaluates the selected protocol slot")
        ),
        int(5)
    );

    let ghost = ObjectReference::new(
        reference.universe().clone(),
        reference.object_type().clone(),
        ObjectId::new("ghost").unwrap(),
    );
    let foreign_document = serde_json::to_string(&document())
        .unwrap()
        .replace("test/orders", "test/foreign");
    let (_foreign_package, foreign_model_view) = fixture_from_document(
        &SOURCE.replace("test/orders", "test/foreign"),
        foreign_document.as_bytes(),
    );
    let foreign_key = quire_checked::key::DeclarationKey {
        package: "test/foreign".to_owned(),
        node: "ix://test/foreign/Node".to_owned(),
    };
    let foreign = ObjectReference::new(
        foreign_model_view
            .object_universe_of(&foreign_key)
            .unwrap()
            .identity()
            .unwrap(),
        foreign_model_view.type_identities()[&foreign_key].clone(),
        ObjectId::new("n5").unwrap(),
    );
    assert_ne!(foreign.universe(), reference.universe());

    for (value, expected_code, expected_cause) in [
        (
            ghost,
            "dangling_reference",
            "absent-target-in-complete-population",
        ),
        (foreign, "foreign_reference", "foreign-universe"),
    ] {
        let before = meter.clone();
        let result = reader_view.evaluate_protocol_slot(
            &capture,
            &supply(
                &package,
                "Refunds",
                vec![("view", Value::Reference(value.clone()))],
                Some(&observation),
            ),
            &mut meter,
        );
        assert_eq!(
            meter, before,
            "refusal leaves every meter counter unchanged"
        );
        let Err(CallFailure::Admission(failure)) = result else {
            panic!("expected reader-backed reference refusal: {result:?}");
        };
        let CallAdmissionFailure::Refused(record) = failure.as_ref() else {
            panic!("reference refusal is a typed refusal: {failure:?}");
        };
        assert_eq!(record.record.catalog.code(), expected_code);
        assert_eq!(record.record.catalog.cause(), expected_cause);
        assert_eq!(
            record.locus,
            CallAdmissionLocus::Binder(binder(&package, "Refunds", "view"))
        );
        if expected_code == "dangling_reference" {
            assert_eq!(
                record.record.fields,
                std::collections::BTreeMap::from([
                    ("object", "ghost".to_owned()),
                    ("population", model::id("nodes")),
                ])
            );
        } else {
            let required = independent_universe_hex(reference.universe());
            let supplied = independent_universe_hex(value.universe());
            assert_eq!(
                record.record.fields,
                std::collections::BTreeMap::from([("required", required), ("supplied", supplied),])
            );
        }
    }
}

/// Values supplied to one TC-1070 protocol slot.
type SlotValues<'p> = Box<dyn Fn(Option<&ObjectReference>) -> Vec<(&'static str, Value)> + 'p>;

/// One TC-1070 slot compared across the checked and the strict-reader view.
struct ParitySlot<'p> {
    name: &'static str,
    protocol: &'static str,
    owner: &'static str,
    member: &'static str,
    index: Option<usize>,
    values: SlotValues<'p>,
    observed: bool,
    expected: Option<Value>,
}

fn parity_slots(package: &CheckedPackage) -> Vec<ParitySlot<'_>> {
    let on = |name,
              protocol,
              owner,
              member,
              index,
              values: Vec<(&'static str, Value)>,
              expected: Value| {
        ParitySlot {
            name,
            protocol,
            owner,
            member,
            index,
            values: Box::new(move |_| values.clone()),
            observed: false,
            expected: Some(expected),
        }
    };
    let observed = |name, owner, member, protocol, expected: Option<Value>| ParitySlot {
        name,
        protocol,
        owner,
        member,
        index: Some(0),
        values: Box::new(|reference: Option<&ObjectReference>| {
            let reference = reference.expect("an observed slot is supplied a reference");
            vec![("view", Value::Reference(reference.clone()))]
        }),
        observed: true,
        expected,
    };
    let tick = record(
        package,
        "Tick",
        vec![("ready", Value::Boolean(true)), ("count", int(2))],
    );
    let t = Value::Boolean(true);
    let f = Value::Boolean(false);
    let mut slots = vec![
        on(
            "activation guard",
            "Refunds",
            "Refunds",
            "activation.guard",
            None,
            vec![("opened", tick)],
            t.clone(),
        ),
        observed(
            "protocol capture",
            "Refunds",
            "captures",
            "Refunds",
            Some(int(5)),
        ),
    ];
    for (owner, forward, trigger, target, activated, earlier, later, recovered) in [
        (
            "Refund",
            "forward",
            "trigger",
            "target",
            "activated",
            "earlier",
            "later",
            "recovered",
        ),
        (
            "Partial",
            "partialForward",
            "partialTrigger",
            "partialTarget",
            "partialActivated",
            "partialEarlier",
            "partialLater",
            "partialRecovered",
        ),
    ] {
        slots.push(on(
            "registration capture",
            "Refunds",
            owner,
            "registration_captures",
            Some(0),
            vec![(forward, receipt(package, 5))],
            int(5),
        ));
        slots.push(on(
            "activation capture",
            "Refunds",
            owner,
            "activation_captures",
            Some(0),
            vec![(trigger, flag(package, true))],
            t.clone(),
        ));
        slots.push(on(
            "trigger guard",
            "Refunds",
            owner,
            "trigger.guard",
            None,
            vec![(trigger, flag(package, false)), (target, int(5))],
            t.clone(),
        ));
        slots.push(on(
            "retry relation",
            "Refunds",
            owner,
            "retry.relation",
            None,
            vec![
                (earlier, attempt(package, 0)),
                (later, attempt(package, 1)),
                (target, int(5)),
                (activated, f.clone()),
            ],
            t.clone(),
        ));
        slots.push(on(
            "recovery predicate",
            "Refunds",
            owner,
            "recovery.predicate",
            None,
            vec![
                (recovered, receipt(package, 5)),
                (target, int(5)),
                (activated, f.clone()),
            ],
            t.clone(),
        ));
    }
    slots.push(on(
        "effect constraint",
        "Refunds",
        "Applied",
        "constraint",
        None,
        vec![("order", int(5))],
        t.clone(),
    ));
    slots.push(on(
        "commit constraint",
        "Refunds",
        "Committed",
        "constraint",
        None,
        vec![("committed", flag(package, true))],
        t.clone(),
    ));
    slots.push(on(
        "finish constraint",
        "Refunds",
        "Closed",
        "constraint",
        None,
        vec![],
        t.clone(),
    ));
    slots.push(observed(
        "related from",
        "Got",
        "related.from",
        "Pipeline",
        None,
    ));
    slots.push(observed(
        "related to",
        "Got",
        "related.to",
        "Pipeline",
        None,
    ));
    slots.push(on(
        "choice visible",
        "Pipeline",
        "Decide",
        "visible",
        Some(0),
        vec![("got", flag(package, true))],
        t.clone(),
    ));
    slots.push(on(
        "choice guard no",
        "Pipeline",
        "Decide",
        "cases.guard",
        Some(0),
        vec![("got", flag(package, true))],
        f,
    ));
    slots.push(on(
        "choice guard yes",
        "Pipeline",
        "Decide",
        "cases.guard",
        Some(1),
        vec![("got", flag(package, true))],
        t,
    ));
    slots
}

fn charges_and_usage_match(label: &str, checked: &Meter, reader: &Meter) {
    assert_eq!(
        checked.admitted_charges(),
        reader.admitted_charges(),
        "{label}: ordered charge logs"
    );
    for counter in LimitKind::ALL {
        assert_eq!(
            checked.consumed(counter),
            reader.consumed(counter),
            "{label}: {counter:?} usage"
        );
    }
}

fn limits_at(measured: &Meter) -> ScalarLimits {
    let mut limits = UNLIMITED;
    for counter in LimitKind::ALL {
        set_limit(&mut limits, counter, measured.consumed(counter));
    }
    limits
}

fn read_checked_package(package: &CheckedPackage) -> qsl_package::AdmittedPackageRead<'_> {
    let bytes = serde_json::to_vec(&document()).unwrap();
    read_checked_package_with_model_document(package, &bytes)
}

fn read_checked_package_with_model_document<'p>(
    package: &'p CheckedPackage,
    model_document: &[u8],
) -> qsl_package::AdmittedPackageRead<'p> {
    let domain_packages = qsl_semantics::model::intake::package_input([model_document]).unwrap();
    qsl_package::read_evaluation_package(
        package,
        qsl_semantics::library::LibraryName::new("acceptance").unwrap(),
        &domain_packages,
        qsl_package::V2ReadLimits::fixture(),
        EmitLimits::fixture(),
    )
    .expect("strict I2 admission accepts the emitted TC-1070 package")
}

/// A call result with its location removed. A reader-backed read of a leaf
/// carries no location (the admitted wire has none for a leaf, and a reader
/// view never takes one from the source package), so parity is compared on
/// everything else: outcome, value, binder, cause, losses and the per-binder
/// charge log (FR-520-AC-9).
fn without_location(result: &Result<ProtocolSlotEvaluation, CallFailure>) -> String {
    match result {
        Ok(ProtocolSlotEvaluation::Evaluated(evaluation, charges)) => {
            format!(
                "{:?} {:?} {charges:?}",
                evaluation.outcome, evaluation.losses
            )
        }
        Ok(ProtocolSlotEvaluation::Unavailable {
            binder,
            cause,
            charges,
            ..
        }) => {
            format!("unavailable {binder:?} {cause:?} {charges:?}")
        }
        other => format!("{other:?}"),
    }
}

/// FR-390-AC-15 location parity: when the reader's halt carries a location
/// (an application-located read), it is the checked view's. A leaf halt
/// carries none on the reader view (AC-64), so only a `Some` is compared.
fn assert_location_parity(
    label: &str,
    checked: &Result<ProtocolSlotEvaluation, CallFailure>,
    reader: &Result<ProtocolSlotEvaluation, CallFailure>,
) {
    if let Ok(ProtocolSlotEvaluation::Unavailable {
        location: Some(reader),
        ..
    }) = reader
    {
        let Ok(ProtocolSlotEvaluation::Unavailable { location, .. }) = checked else {
            panic!("{label}: the checked view did not halt unavailable");
        };
        assert_eq!(location.as_ref(), Some(reader), "{label}: halt location");
    }
}

/// TC-1073: compares the explicit supported TC-1070 root list between the
/// in-process view and the strict-reader view.
#[test]
#[trace("TC-142", "TC-1073", "FR-049-AC-9", "FR-520-AC-9")]
fn reader_backed_tc1070_slots_match_the_checked_view_for_supported_roots() {
    let (package, model_view) = fixture_with_view();
    let read = read_checked_package(&package);
    let reader_view = read.evaluation_view().expect("the read retains its view");
    assert!(reader_view.is_reader_backed());
    let (checked_observation, checked_reference) = observation(&package, &model_view);
    let (reader_observation, reader_reference) =
        observation_through_view(&package, &reader_view, &model_view, &["n5"], true);
    assert_eq!(checked_reference, reader_reference);

    let specs = parity_slots(&package);
    let mut compared = Vec::new();
    for spec in specs {
        let selected = slot(&package, spec.protocol, spec.owner, spec.member, spec.index);
        let values = (spec.values)(Some(&checked_reference));
        let checked_supply = supply(
            &package,
            spec.protocol,
            values.clone(),
            spec.observed.then_some(&checked_observation),
        );
        let reader_supply = supply(
            &package,
            spec.protocol,
            values.clone(),
            spec.observed.then_some(&reader_observation),
        );
        let label = format!("{} ({}.{})", spec.name, spec.owner, spec.member);

        let mut checked_meter = Meter::new(UNLIMITED);
        let checked =
            package.evaluate_protocol_slot(&selected, &checked_supply, &mut checked_meter);
        let ProtocolSlotEvaluation::Evaluated(ref checked_evaluation, _) = *checked
            .as_ref()
            .unwrap_or_else(|failure| panic!("{label}: checked view: {failure:?}"))
        else {
            panic!("{label}: checked view did not evaluate");
        };
        let FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(value)) =
            &checked_evaluation.outcome
        else {
            panic!("{label}: checked view did not complete");
        };
        match &spec.expected {
            Some(expected) => assert_value_eq!(*value, *expected, "checked {label}"),
            None => assert!(
                matches!(value, Value::Reference(actual) if *actual == checked_reference),
                "{label}: returns the supplied endpoint"
            ),
        }

        let mut reader_meter = Meter::new(UNLIMITED);
        let reader =
            reader_view.evaluate_protocol_slot(&selected, &reader_supply, &mut reader_meter);
        compared.push(label.clone());
        assert_eq!(
            without_location(&checked).replace(&format!("{checked_reference:?}"), "REF"),
            without_location(&reader).replace(&format!("{reader_reference:?}"), "REF"),
            "{label}: complete results (value, losses)"
        );
        charges_and_usage_match(&label, &checked_meter, &reader_meter);

        // Running the call twice gives equal results and charge logs.
        let mut again = Meter::new(UNLIMITED);
        let second = reader_view.evaluate_protocol_slot(&selected, &reader_supply, &mut again);
        assert_eq!(
            without_location(&second),
            without_location(&reader),
            "{label}: run twice"
        );
        charges_and_usage_match(&format!("{label} twice"), &reader_meter, &again);

        // AC-9: each counter at its measured usage completes; one below returns
        // Incomplete naming that counter, on both views alike.
        let exact = limits_at(&checked_meter);
        let mut exact_checked = Meter::new(exact);
        let mut exact_reader = Meter::new(exact);
        let exact_checked_result =
            package.evaluate_protocol_slot(&selected, &checked_supply, &mut exact_checked);
        let exact_reader_result =
            reader_view.evaluate_protocol_slot(&selected, &reader_supply, &mut exact_reader);
        assert_eq!(
            without_location(&exact_checked_result),
            without_location(&exact_reader_result),
            "{label}: exact budget"
        );
        completed(exact_reader_result.unwrap());
        charges_and_usage_match(&format!("{label} exact"), &exact_checked, &exact_reader);
        for counter in LimitKind::ALL {
            let used = checked_meter.consumed(counter);
            if used == 0 {
                continue;
            }
            let mut limits = exact;
            set_limit(&mut limits, counter, used - 1);
            let mut short_checked = Meter::new(limits);
            let mut short_reader = Meter::new(limits);
            let short_checked_result =
                package.evaluate_protocol_slot(&selected, &checked_supply, &mut short_checked);
            let short_reader_result =
                reader_view.evaluate_protocol_slot(&selected, &reader_supply, &mut short_reader);
            assert_incomplete(short_reader_result.unwrap(), counter);
            assert_incomplete(short_checked_result.unwrap(), counter);
            charges_and_usage_match(
                &format!("{label} one short {counter:?}"),
                &short_checked,
                &short_reader,
            );
        }

        // Unavailable supply: the halt reports the same binder, cause and
        // charges made before the read (the location is not compared).
        let unavailable = |keyed: &[(&'static str, Value)]| ProtocolSupply {
            binders: keyed
                .iter()
                .map(|(name, _)| {
                    (
                        binder(&package, spec.protocol, name),
                        SuppliedValue::Unavailable(CatalogCode::new(
                            "unavailable_observation",
                            "missing-required-artifact",
                        ))
                        .into(),
                    )
                })
                .collect(),
            observation: None,
        };
        let keyed = (spec.values)(Some(&checked_reference));
        if !keyed.is_empty() {
            let supply_unavailable = unavailable(&keyed);
            let mut unavailable_checked = Meter::new(UNLIMITED);
            let mut unavailable_reader = Meter::new(UNLIMITED);
            let on_checked = package.evaluate_protocol_slot(
                &selected,
                &supply_unavailable,
                &mut unavailable_checked,
            );
            let on_reader = reader_view.evaluate_protocol_slot(
                &selected,
                &supply_unavailable,
                &mut unavailable_reader,
            );
            assert!(
                matches!(on_checked, Ok(ProtocolSlotEvaluation::Unavailable { .. })),
                "{label}: unavailable halt: {on_checked:?}"
            );
            assert_eq!(
                without_location(&on_checked),
                without_location(&on_reader),
                "{label}: unavailable halt binder and cause"
            );
            assert_location_parity(&label, &on_checked, &on_reader);
            assert!(
                matches!(
                    on_checked,
                    Ok(ProtocolSlotEvaluation::Unavailable {
                        location: Some(_),
                        ..
                    })
                ),
                "{label}: the in-process halt is located"
            );
            // FR-390-AC-64: the halt is at a `Local` read, a leaf whose
            // admitted wire carries no location of its own, so the reader
            // reports none and never the source package's.
            assert!(
                matches!(
                    on_reader,
                    Ok(ProtocolSlotEvaluation::Unavailable { location: None, .. })
                ),
                "{label}: the reader halt reports no location, never the source package's"
            );
            charges_and_usage_match(
                &format!("{label} unavailable"),
                &unavailable_checked,
                &unavailable_reader,
            );
        }
    }
    assert_eq!(
        compared.len(),
        20,
        "all listed TC-1070 protocol roots were evaluated through the reader"
    );
}

/// Selection refusals have the same locus on both views and leave their
/// meters unchanged, including refusals only the view can make (an absent
/// node, a node of another tag). The AC-15 matrix reuses this assertion so its
/// criterion trace includes the refusal path too.
#[test]
#[trace("TC-1071", "FR-390-AC-4")]
fn selection_refusals_match_across_the_checked_and_reader_views() {
    assert_selection_refusals_match_across_the_checked_and_reader_views();
}

fn assert_selection_refusals_match_across_the_checked_and_reader_views() {
    let package = fixture_from_source(&SOURCE.replace(
        "  run sequence Main {",
        "  terminal any;\n  run sequence Main {",
    ))
    .0;
    let read = read_checked_package(&package);
    let reader_view = read.evaluation_view().expect("the read retains its view");
    let empty = supply(&package, "Refunds", vec![], None);
    let mut cases = vec![
        slot(&package, "Refunds", "Refunds", "retry.relation", None),
        slot(
            &package,
            "Refunds",
            "Refund",
            "registration_captures",
            Some(1),
        ),
        slot(&package, "Refunds", "Refund", "registration_captures", None),
        slot(&package, "Refunds", "Refund", "retry.relation", Some(0)),
        slot(&package, "Refunds", "Closed", "condition", None),
    ];
    let real = slot(
        &package,
        "Refunds",
        "Refund",
        "registration_captures",
        Some(0),
    );
    cases.push(ProtocolSlot {
        node: population_node("absent-node"),
        ..real.clone()
    });
    cases.push(ProtocolSlot {
        node: binder(&package, "Refunds", "target"),
        ..real
    });
    for case in cases {
        let mut checked_meter = Meter::new(UNLIMITED);
        let mut reader_meter = Meter::new(UNLIMITED);
        let checked = package.evaluate_protocol_slot(&case, &empty, &mut checked_meter);
        let reader = reader_view.evaluate_protocol_slot(&case, &empty, &mut reader_meter);
        let (
            Err(CallFailure::Admission(checked_failure)),
            Err(CallFailure::Admission(reader_failure)),
        ) = (&checked, &reader)
        else {
            panic!("{case:?} must be a typed admission refusal on both views: {checked:?} / {reader:?}");
        };
        assert_eq!(
            checked_failure, reader_failure,
            "{case:?}: same record and locus"
        );
        // Pin what the reader refuses with, so a reader that ignored the
        // descriptor (or refused for another reason) cannot pass by agreeing
        // with the checked view: the record names this case's own locus.
        let CallAdmissionFailure::Refused(record) = reader_failure.as_ref() else {
            panic!("{case:?}: the reader refuses, it does not report incomplete");
        };
        let malformed = case.node == binder(&package, "Refunds", "target");
        assert_eq!(
            (record.record.catalog.code(), record.record.catalog.cause()),
            if malformed {
                ("invalid_model_binding", "malformed-declaration")
            } else {
                ("missing_declaration", "missing-name")
            },
            "{case:?}: reader refusal code and cause"
        );
        let absent = case.node == population_node("absent-node");
        assert_eq!(
            record.locus,
            if absent || malformed {
                CallAdmissionLocus::Node(case.node.clone())
            } else {
                CallAdmissionLocus::Member {
                    name: case.member.clone(),
                    index: case.index,
                }
            },
            "{case:?}: reader refusal locus is the case's own"
        );
        assert_eq!(
            checked_meter,
            Meter::new(UNLIMITED),
            "{case:?}: checked meter unchanged"
        );
        assert_eq!(
            reader_meter,
            Meter::new(UNLIMITED),
            "{case:?}: reader meter unchanged"
        );
    }
}

/// FR-390-AC-15: every TC-1070 protocol slot keeps the complete result and
/// ordered charges when evaluated through the bytes the I2 reader admits.
#[test]
#[trace("TC-1070", "TC-1073", "FR-390-AC-15")]
fn reader_parity_matches_every_tc1070_slot_result_and_charge_log() {
    let (package, model_view) = fixture_with_view();
    let read = read_checked_package(&package);
    let reader_view = read.evaluation_view().expect("the read retains its view");
    // AC-15 includes refusal parity and unchanged meters for absent and
    // malformed selected descriptors. The same assertion also has a focused
    // AC-4 test, while this full matrix remains gated on QSL-1625.
    assert_selection_refusals_match_across_the_checked_and_reader_views();
    let (checked_observation, checked_reference) = observation(&package, &model_view);
    let (reader_observation, reader_reference) =
        observation_through_view(&package, &reader_view, &model_view, &["n5"], true);
    assert_eq!(checked_reference, reader_reference);

    let specs = parity_slots(&package);
    for spec in &specs {
        let selected = slot(&package, spec.protocol, spec.owner, spec.member, spec.index);
        let values = (spec.values)(Some(&checked_reference));
        let checked_supply = supply(
            &package,
            spec.protocol,
            values.clone(),
            spec.observed.then_some(&checked_observation),
        );
        let reader_supply = supply(
            &package,
            spec.protocol,
            values.clone(),
            spec.observed.then_some(&reader_observation),
        );
        let mut checked_meter = Meter::new(UNLIMITED);
        let checked = package
            .evaluate_protocol_slot(&selected, &checked_supply, &mut checked_meter)
            .unwrap_or_else(|failure| panic!("{}: checked view: {failure:?}", spec.name));
        let mut reader_meter = Meter::new(UNLIMITED);
        let reader = reader_view
            .evaluate_protocol_slot(&selected, &reader_supply, &mut reader_meter)
            .unwrap_or_else(|failure| panic!("{}: reader view: {failure:?}", spec.name));
        assert_eq!(
            format!("{checked:?}"),
            format!("{reader:?}"),
            "{}: full outcome and ordered charge-bearing result",
            spec.name
        );
        charges_and_usage_match(spec.name, &checked_meter, &reader_meter);

        // AC-9: every independently charged counter has an exact boundary and
        // a one-short refusal with the same completed/incomplete outcome.
        let exact = limits_at(&checked_meter);
        let mut exact_checked = Meter::new(exact);
        let mut exact_reader = Meter::new(exact);
        let exact_checked_result =
            package.evaluate_protocol_slot(&selected, &checked_supply, &mut exact_checked);
        let exact_reader_result =
            reader_view.evaluate_protocol_slot(&selected, &reader_supply, &mut exact_reader);
        assert_eq!(
            without_location(&exact_checked_result),
            without_location(&exact_reader_result),
            "{}: exact AC-9 budget",
            spec.name
        );
        for counter in LimitKind::ALL {
            let used = checked_meter.consumed(counter);
            if used == 0 {
                continue;
            }
            let mut limits = exact;
            set_limit(&mut limits, counter, used - 1);
            let mut short_checked = Meter::new(limits);
            let mut short_reader = Meter::new(limits);
            let checked_short =
                package.evaluate_protocol_slot(&selected, &checked_supply, &mut short_checked);
            let reader_short =
                reader_view.evaluate_protocol_slot(&selected, &reader_supply, &mut short_reader);
            assert_eq!(
                without_location(&checked_short),
                without_location(&reader_short),
                "{}: one short at {counter:?}",
                spec.name
            );
            assert_incomplete(reader_short.unwrap(), counter);
            assert_incomplete(checked_short.unwrap(), counter);
            charges_and_usage_match(spec.name, &short_checked, &short_reader);
        }

        // Unavailable sources stop at the same binder with the same cause and
        // the same ordered charges already made before the halt.
        if !values.is_empty() {
            let unavailable = ProtocolSupply {
                binders: values
                    .iter()
                    .map(|(name, _)| {
                        (
                            binder(&package, spec.protocol, name),
                            SuppliedValue::Unavailable(CatalogCode::new(
                                "unavailable_observation",
                                "missing-required-artifact",
                            ))
                            .into(),
                        )
                    })
                    .collect(),
                observation: None,
            };
            let mut unavailable_checked = Meter::new(UNLIMITED);
            let mut unavailable_reader = Meter::new(UNLIMITED);
            let checked_unavailable =
                package.evaluate_protocol_slot(&selected, &unavailable, &mut unavailable_checked);
            let reader_unavailable = reader_view.evaluate_protocol_slot(
                &selected,
                &unavailable,
                &mut unavailable_reader,
            );
            assert_eq!(
                without_location(&checked_unavailable),
                without_location(&reader_unavailable),
                "{}: unavailable supply parity",
                spec.name
            );
            assert_location_parity(spec.name, &checked_unavailable, &reader_unavailable);
            assert!(matches!(
                checked_unavailable,
                Ok(ProtocolSlotEvaluation::Unavailable { .. })
            ));
            charges_and_usage_match(spec.name, &unavailable_checked, &unavailable_reader);
        }

        // TC-1070's retry relation short-circuits after the first false
        // comparison. Both views retain that result and its shorter log.
        if spec.name == "retry relation" {
            let mut short_values = values.clone();
            let earlier = if spec.owner == "Refund" {
                "earlier"
            } else {
                "partialEarlier"
            };
            let (_, value) = short_values
                .iter_mut()
                .find(|(name, _)| *name == earlier)
                .expect("retry supplies the earlier Attempt");
            *value = attempt(&package, 3);
            let short_checked_supply = supply(&package, spec.protocol, short_values.clone(), None);
            let short_reader_supply = supply(&package, spec.protocol, short_values, None);
            let mut short_checked_meter = Meter::new(UNLIMITED);
            let mut short_reader_meter = Meter::new(UNLIMITED);
            let checked_short = package.evaluate_protocol_slot(
                &selected,
                &short_checked_supply,
                &mut short_checked_meter,
            );
            let reader_short = reader_view.evaluate_protocol_slot(
                &selected,
                &short_reader_supply,
                &mut short_reader_meter,
            );
            assert_eq!(
                without_location(&checked_short),
                without_location(&reader_short),
                "{}: short-circuit result parity",
                spec.name
            );
            assert!(
                short_checked_meter.consumed(LimitKind::WorkUnits)
                    < checked_meter.consumed(LimitKind::WorkUnits),
                "the false first comparison skips the remaining relation"
            );
            charges_and_usage_match(spec.name, &short_checked_meter, &short_reader_meter);
        }
    }

    // A refused observation cannot mask supplied-record defects; each
    // checked/reader pair retains the exact refusal locus and leaves its meter
    // unchanged. The reader assertions remain part of the QSL-1625 gate.
    let refused_record = quire_semantic_value::admission::AdmissionRecord {
        catalog: CatalogCode::new("invalid_model_binding", "wrong-model-selection"),
        fields: Default::default(),
        path: Vec::new(),
        kernel: None,
    };
    let refused = ObservationAdmission::Refused(refused_record);
    let retry = slot(&package, "Refunds", "Refund", "retry.relation", None);
    let locus = |name: &str| CallAdmissionLocus::Binder(binder(&package, "Refunds", name));
    let mut duplicate = retry_supply(&package, 0);
    duplicate.binders.push((
        binder(&package, "Refunds", "earlier"),
        SuppliedValue::Value(attempt(&package, 0)).into(),
    ));
    duplicate.observation = Some(&refused);
    let mut unknown = retry_supply(&package, 0);
    unknown.binders.push((
        binder(&package, "Refunds", "forward"),
        SuppliedValue::Value(receipt(&package, 5)).into(),
    ));
    unknown.observation = Some(&refused);
    let mut missing = retry_supply(&package, 0);
    missing
        .binders
        .retain(|(key, _)| *key != binder(&package, "Refunds", "target"));
    missing.observation = Some(&refused);
    let mut wrong_shape = retry_supply(&package, 0);
    let target = binder(&package, "Refunds", "target");
    wrong_shape
        .binders
        .iter_mut()
        .find(|(key, _)| *key == target)
        .expect("the target source is supplied")
        .1 = SuppliedValue::Value(Value::Boolean(true)).into();
    wrong_shape.observation = Some(&refused);
    for (supplied, expected, expected_locus) in [
        (
            duplicate,
            CatalogCode::new("invalid_runtime_input", "duplicate-member"),
            locus("earlier"),
        ),
        (
            unknown,
            CatalogCode::new("invalid_runtime_input", "unknown-member"),
            locus("forward"),
        ),
        (
            missing,
            CatalogCode::new("invalid_runtime_input", "missing-member"),
            locus("target"),
        ),
        (
            wrong_shape,
            CatalogCode::new("invalid_runtime_input", "wrong-value-kind"),
            locus("target"),
        ),
    ] {
        let mut checked_meter = Meter::new(UNLIMITED);
        let mut reader_meter = Meter::new(UNLIMITED);
        let checked = package.evaluate_protocol_slot(&retry, &supplied, &mut checked_meter);
        let reader = reader_view.evaluate_protocol_slot(&retry, &supplied, &mut reader_meter);
        let (
            Err(CallFailure::Admission(checked_failure)),
            Err(CallFailure::Admission(reader_failure)),
        ) = (&checked, &reader)
        else {
            panic!("supply defects refuse before observation inspection: {checked:?} / {reader:?}");
        };
        assert_eq!(checked_failure, reader_failure);
        let CallAdmissionFailure::Refused(record) = reader_failure.as_ref() else {
            panic!("supply defect is a refusal: {reader_failure:?}");
        };
        assert_eq!(record.record.catalog, expected);
        assert_eq!(record.locus, expected_locus);
        assert_eq!(checked_meter, Meter::new(UNLIMITED));
        assert_eq!(reader_meter, Meter::new(UNLIMITED));
    }

    // AC-10: each compensation's five calls share one meter in both orders;
    // the exact total completes, and one below it is incomplete at the last
    // call that charges work.
    for owner in ["Refund", "Partial"] {
        let owner_specs: Vec<_> = specs
            .iter()
            .filter(|spec| {
                spec.owner == owner
                    && matches!(
                        spec.name,
                        "registration capture"
                            | "activation capture"
                            | "trigger guard"
                            | "retry relation"
                            | "recovery predicate"
                    )
            })
            .collect();
        assert_eq!(owner_specs.len(), 5, "{owner}: all AC-10 slots are covered");
        let selections: Vec<_> = owner_specs
            .iter()
            .map(|spec| slot(&package, spec.protocol, spec.owner, spec.member, spec.index))
            .collect();
        let supplies = |observation| {
            owner_specs
                .iter()
                .map(|spec| {
                    supply(
                        &package,
                        spec.protocol,
                        (spec.values)(Some(&checked_reference)),
                        spec.observed.then_some(observation),
                    )
                })
                .collect::<Vec<_>>()
        };
        let checked_supplies = supplies(&checked_observation);
        let reader_supplies = supplies(&reader_observation);
        let run = |reader: bool, order: &[usize], limits: ScalarLimits| {
            let mut meter = Meter::new(limits);
            let results = order
                .iter()
                .map(|index| {
                    if reader {
                        reader_view.evaluate_protocol_slot(
                            &selections[*index],
                            &reader_supplies[*index],
                            &mut meter,
                        )
                    } else {
                        package.evaluate_protocol_slot(
                            &selections[*index],
                            &checked_supplies[*index],
                            &mut meter,
                        )
                    }
                })
                .collect::<Vec<_>>();
            (meter, results)
        };
        let forward: Vec<_> = (0..selections.len()).collect();
        let reverse: Vec<_> = forward.iter().rev().copied().collect();
        let (checked_meter, checked_results) = run(false, &forward, UNLIMITED);
        let (reader_meter, reader_results) = run(true, &forward, UNLIMITED);
        for (checked, reader) in checked_results.iter().zip(&reader_results) {
            assert_eq!(without_location(checked), without_location(reader));
        }
        charges_and_usage_match(
            &format!("{owner} shared AC-10 meter"),
            &checked_meter,
            &reader_meter,
        );
        let (reverse_meter, reverse_results) = run(true, &reverse, UNLIMITED);
        for result in reverse_results {
            completed(result.unwrap());
        }
        for counter in LimitKind::ALL {
            assert_eq!(
                reverse_meter.consumed(counter),
                reader_meter.consumed(counter),
                "{owner}: reverse AC-10 order preserves {counter:?} total"
            );
        }
        let sum = reader_meter.consumed(LimitKind::WorkUnits);
        let mut exact_limits = UNLIMITED;
        set_limit(&mut exact_limits, LimitKind::WorkUnits, sum);
        let (_, exact_results) = run(true, &forward, exact_limits);
        for result in exact_results {
            completed(result.unwrap());
        }
        let last_charging = forward
            .iter()
            .rposition(|index| {
                let mut meter = Meter::new(UNLIMITED);
                let _ = reader_view.evaluate_protocol_slot(
                    &selections[*index],
                    &reader_supplies[*index],
                    &mut meter,
                );
                meter.consumed(LimitKind::WorkUnits) > 0
            })
            .expect("an AC-10 call charges work_units");
        let mut one_short = UNLIMITED;
        set_limit(&mut one_short, LimitKind::WorkUnits, sum - 1);
        for reader in [false, true] {
            let (_, results) = run(reader, &forward, one_short);
            for (position, result) in results.into_iter().enumerate() {
                if position < last_charging {
                    completed(result.unwrap());
                } else if position == last_charging {
                    assert_incomplete(result.unwrap(), LimitKind::WorkUnits);
                }
            }
        }
    }
}

/// FR-390-AC-19: a bare `view.total` read and an explicit `deref(view).total`
/// read keep their own in-process charge logs through emitted-package
/// admission, including FR-107's explicit-dereference charge.
#[test]
#[trace("TC-1073", "FR-390-AC-19")]
fn bare_and_explicit_dereference_reads_keep_their_charge_logs_through_the_reader() {
    let mut model_logs = Vec::new();
    for (replacement, expected) in [
        (
            "= view.total;",
            vec![quire_exact::ChargePoint::ModelNavigate],
        ),
        (
            "= deref(view).total;",
            vec![
                quire_exact::ChargePoint::ModelDeref,
                quire_exact::ChargePoint::ModelNavigate,
            ],
        ),
    ] {
        let (package, model_view) =
            fixture_from_source(&SOURCE.replace("= view.total;", replacement));
        let read = read_checked_package(&package);
        let reader_view = read.evaluation_view().expect("the read retains its view");
        let (checked_observation, reference) = observation(&package, &model_view);
        let (reader_observation, _) =
            observation_through_view(&package, &reader_view, &model_view, &["n5"], true);
        let capture = slot(&package, "Refunds", "Refunds", "captures", Some(0));
        let values = |observation| {
            supply(
                &package,
                "Refunds",
                vec![("view", Value::Reference(reference.clone()))],
                Some(observation),
            )
        };
        let mut checked_meter = Meter::new(UNLIMITED);
        let mut reader_meter = Meter::new(UNLIMITED);
        let checked = package
            .evaluate_protocol_slot(&capture, &values(&checked_observation), &mut checked_meter)
            .unwrap();
        let reader = reader_view
            .evaluate_protocol_slot(&capture, &values(&reader_observation), &mut reader_meter)
            .expect("an attribute-rooted capture is reader-backed");
        assert_value_eq!(completed(checked), int(5));
        assert_value_eq!(completed(reader), int(5));
        charges_and_usage_match(replacement, &checked_meter, &reader_meter);
        let model_points: Vec<_> = reader_meter
            .admitted_charges()
            .iter()
            .filter_map(|charge| {
                matches!(
                    charge.point,
                    quire_exact::ChargePoint::ModelDeref | quire_exact::ChargePoint::ModelNavigate
                )
                .then_some(charge.point)
            })
            .collect();
        assert_eq!(model_points, expected, "{replacement}");
        model_logs.push(reader_meter.admitted_charges().to_vec());
    }
    assert_ne!(
        model_logs[0], model_logs[1],
        "the explicit dereference keeps its extra charge"
    );
}

/// FR-390-AC-18: a reader-backed slot preserves the declared record identity
/// and recursively nested optional and sequence references in TC-1070's E-P
/// `Receipt` value.
#[test]
#[trace("TC-1070", "FR-390-AC-18")]
fn reader_backed_projection_preserves_nested_option_and_collection_value_types() {
    let source = SOURCE.replace(
        "capture target: M::Total = forward.total;",
        "capture target: M::Total = forward.total;\n    \
         capture nested: M::PeerBundle = forward.bundle;",
    );
    let model_bytes = serde_json::to_vec(&document_ep()).unwrap();
    let (package, model_view) = fixture_from_document(&source, &model_bytes);
    let read = read_checked_package_with_model_document(&package, &model_bytes);
    let reader_view = read.evaluation_view().expect("the read retains its view");
    let peer = other_reference(&model_view, "ghost");
    let nested = other_reference(&model_view, "nested-1");
    let optional = other_reference(&model_view, "option-1");
    let sequence = other_reference(&model_view, "sequence-1");
    let (receipt, expected_bundle) =
        ep_receipt_and_bundle(&package, [&peer, &nested, &optional, &sequence]);
    let references = vec![
        ProtocolReferenceOccurrence {
            path: vec![PathStep::Field("peer".into())],
            population: population_node("otherSpare"),
        },
        ProtocolReferenceOccurrence {
            path: vec![
                PathStep::Field("bundle".into()),
                PathStep::Field("nested".into()),
            ],
            population: population_node("otherNodes"),
        },
        ProtocolReferenceOccurrence {
            path: vec![
                PathStep::Field("bundle".into()),
                PathStep::Field("optional".into()),
                PathStep::OptionValue,
            ],
            population: population_node("otherSpare"),
        },
        ProtocolReferenceOccurrence {
            path: vec![
                PathStep::Field("bundle".into()),
                PathStep::Field("sequence".into()),
                PathStep::Index(0),
            ],
            population: population_node("otherNodes"),
        },
    ];
    let observation = ep_observation(
        &package,
        &model_view,
        Some((true, &["ghost", "option-1"])),
        true,
    );
    let reader_observation = ep_observation_state_through_view(
        &package,
        &reader_view,
        Some((true, &["ghost", "option-1"])),
        true,
        None,
        None,
        &[],
    );
    let values = || {
        ep_supply(
            &package,
            receipt.clone(),
            references.clone(),
            Some(&observation),
        )
    };
    let selected = slot(
        &package,
        "Refunds",
        "Refund",
        "registration_captures",
        Some(1),
    );
    let mut checked_meter = Meter::new(UNLIMITED);
    let checked_evaluation =
        package.evaluate_protocol_slot(&selected, &values(), &mut checked_meter);
    let mut reader_meter = Meter::new(UNLIMITED);
    let reader_values = || {
        ep_supply(
            &package,
            receipt.clone(),
            references.clone(),
            Some(&reader_observation),
        )
    };
    let reader_evaluation =
        reader_view.evaluate_protocol_slot(&selected, &reader_values(), &mut reader_meter);
    assert_eq!(
        without_location(&checked_evaluation),
        without_location(&reader_evaluation),
        "nested record capture result parity"
    );
    let expected =
        completed(checked_evaluation.expect("checked view evaluates the nested record capture"));
    let actual =
        completed(reader_evaluation.expect("reader view projects the nested record capture"));
    assert_value_eq!(expected, expected_bundle, "checked nested bundle");
    assert_value_eq!(actual, expected_bundle, "reader nested bundle");
    charges_and_usage_match(
        "nested Option/Sequence Receipt",
        &checked_meter,
        &reader_meter,
    );
}

/// FR-390-AC-24: native String, UUID and Timestamp equality keeps its exact
/// result and charges when protocol slot evaluation crosses the I2 reader.
#[test]
#[trace("TC-1289", "FR-390-AC-24")]
fn reader_backed_projection_native_equality_preserves_scalar_identity_and_charges() {
    let source = SOURCE
        .replace(
            "profile P = \"quire.protocol.finite-global/v1\";",
            "profile P = \"quire.protocol.finite-global/v1\";\n\
             profile v = \"quire.value.complete/v1\";",
        )
        .replace(
            "model M = \"test/orders\";",
            "model M = \"test/orders\";\n\
             record Native { label: String; id: UUID; stamp: Timestamp; }\n\
             record NativeTwin { label: String; id: UUID; stamp: Timestamp; }\n\
             function sameString using v(a: String, b: String): Boolean pure { a = b }\n\
             function sameUuid using v(a: UUID, b: UUID): Boolean pure { a = b }\n\
             function sameTimestamp using v(a: Timestamp, b: Timestamp): Boolean pure { a = b }\n\
             function sameNative using v(a: Native, b: Native): Boolean pure { a.label = b.label and a.id = b.id and a.stamp = b.stamp }\n\
             function sameNativeTwin using v(a: NativeTwin, b: NativeTwin): Boolean pure { a.label = b.label and a.id = b.id and a.stamp = b.stamp }",
        )
        .replace(
            "capture order: M::Total = view.total;",
            "capture order: M::Total = view.total;\n\
             capture stringEquality: Boolean = sameString(\"é\", \"é\");\n\
             capture uuidEquality: Boolean = sameUuid(UUID(\"00112233-4455-6677-8899-aabbccddeeff\"), UUID(\"00112233-4455-6677-8899-aabbccddeeff\"));\n\
             capture timestampEquality: Boolean = sameTimestamp(Timestamp(5), Timestamp(5));\n\
             capture nestedNativeEquality: Boolean = sameNative(Native{ label: \"first\", id: UUID(\"00112233-4455-6677-8899-aabbccddeeff\"), stamp: Timestamp(5) }, Native{ label: \"first\", id: UUID(\"00112233-4455-6677-8899-aabbccddeeff\"), stamp: Timestamp(5) });\n\
             capture nestedNativeDifference: Boolean = sameNative(Native{ label: \"first\", id: UUID(\"00112233-4455-6677-8899-aabbccddeeff\"), stamp: Timestamp(5) }, Native{ label: \"second\", id: UUID(\"00112233-4455-6677-8899-aabbccddeeff\"), stamp: Timestamp(5) });\n\
             capture sameShapeNativeTwin: Boolean = sameNativeTwin(NativeTwin{ label: \"first\", id: UUID(\"00112233-4455-6677-8899-aabbccddeeff\"), stamp: Timestamp(5) }, NativeTwin{ label: \"first\", id: UUID(\"00112233-4455-6677-8899-aabbccddeeff\"), stamp: Timestamp(5) });",
        );
    let (package, model_view) = fixture_from_source(&source);
    let read = read_checked_package(&package);
    let reader_view = read.evaluation_view().expect("the read retains its view");
    let (observation, _) = observation(&package, &model_view);
    let (reader_observation, _) =
        observation_through_view(&package, &reader_view, &model_view, &["n5"], true);
    let expected = [false, true, true, true, false, true];
    for (index, expected) in expected.into_iter().enumerate() {
        let selected = slot(&package, "Refunds", "Refunds", "captures", Some(index + 1));
        let checked_supplies = supply(&package, "Refunds", Vec::new(), Some(&observation));
        let reader_supplies = supply(&package, "Refunds", Vec::new(), Some(&reader_observation));
        let mut checked_meter = Meter::new(UNLIMITED);
        let checked = package
            .evaluate_protocol_slot(&selected, &checked_supplies, &mut checked_meter)
            .expect("checked view evaluates the native comparison");
        assert_value_eq!(completed(checked), Value::Boolean(expected));
        let mut reader_meter = Meter::new(UNLIMITED);
        let reader = reader_view
            .evaluate_protocol_slot(&selected, &reader_supplies, &mut reader_meter)
            .expect("reader view evaluates the native comparison");
        assert_value_eq!(completed(reader), Value::Boolean(expected));
        charges_and_usage_match("native scalar equality", &checked_meter, &reader_meter);
    }
}

/// FR-390-AC-25: a reader-backed projection preserves two declared units of
/// one dimension as distinct identities, including conversion to the selected
/// target unit.
#[test]
#[trace("TC-1290", "FR-390-AC-25")]
fn reader_backed_projection_preserves_declared_and_compound_quantity_units() {
    let source = SOURCE
        .replace(
            "model M = \"test/orders\";",
            "model M = \"test/orders\";\n\
             dimension Length;\n\
             unit m : Length = rational(1, 1);\n\
             unit cm : Length = rational(1, 100) * m;",
        )
        .replace(
            "capture order: M::Total = view.total;",
            "capture order: M::Total = view.total;\n\
             capture metres: Quantity<m> = 1 m;\n\
             capture centimetres: Quantity<cm> = 100 cm;\n\
             capture convertedUnit: Boolean = convert<Quantity<m>>(100 cm) = 1 m;",
        );
    let (package, model_view) = fixture_from_source(&source);
    let read = read_checked_package(&package);
    let reader_view = read.evaluation_view().expect("the read retains its view");
    let (observation, _) = observation(&package, &model_view);
    let (reader_observation, _) =
        observation_through_view(&package, &reader_view, &model_view, &["n5"], true);
    let mut declared_units = Vec::new();
    for index in 1..=3 {
        let selected = slot(&package, "Refunds", "Refunds", "captures", Some(index));
        let checked_supplies = supply(&package, "Refunds", Vec::new(), Some(&observation));
        let reader_supplies = supply(&package, "Refunds", Vec::new(), Some(&reader_observation));
        let mut checked_meter = Meter::new(UNLIMITED);
        let checked = package
            .evaluate_protocol_slot(&selected, &checked_supplies, &mut checked_meter)
            .expect("checked view evaluates the quantity identity case");
        let checked_value = completed(checked);
        if index <= 2 {
            let Value::Quantity(quantity) = &checked_value else {
                panic!("declared quantity value: {checked_value:?}");
            };
            declared_units.push(quantity.unit().clone());
        } else {
            assert_value_eq!(checked_value, Value::Boolean(true));
        }

        let mut reader_meter = Meter::new(UNLIMITED);
        let reader = reader_view
            .evaluate_protocol_slot(&selected, &reader_supplies, &mut reader_meter)
            .expect("reader view evaluates the quantity identity case");
        assert_value_eq!(completed(reader), checked_value);
        charges_and_usage_match("quantity unit identity", &checked_meter, &reader_meter);
    }
    assert_ne!(declared_units[0], declared_units[1]);
}

/// FR-390-AC-25: a compound quantity identity is retained by equality and by
/// the reader-backed protocol slot view.
#[test]
#[trace("TC-1290", "FR-390-AC-25")]
fn reader_backed_projection_preserves_compound_quantity_identity() {
    let source = SOURCE
        .replace(
            "model M = \"test/orders\";",
            "model M = \"test/orders\";\n\
             dimension Length;\n\
             unit m : Length = rational(1, 1);",
        )
        .replace(
            "capture order: M::Total = view.total;",
            "capture order: M::Total = view.total;\n\
             capture compoundUnit: Boolean = (1 m * 1 m) = (1 m * (1 m));",
        );
    let (package, model_view) = fixture_from_source(&source);
    let selected = slot(&package, "Refunds", "Refunds", "captures", Some(1));
    let view = package.evaluation_view();
    let selected_term = view
        .select_protocol_slot(&qsl_package::ProtocolSlotDescriptor::new(
            selected.node.clone(),
            selected.member.clone(),
            selected.index,
        ))
        .expect("the checked view selects the compound equality");
    let mut pending = vec![selected_term.checked_body().root()];
    let mut compound_units = Vec::new();
    while let Some(node) = pending.pop() {
        if let ValueType::Quantity(unit) = node.value_type() {
            if unit.domain() == quire_exact::UnitDomain::Compound {
                compound_units.push(unit.clone());
            }
        }
        pending.extend(node.children());
    }
    assert!(
        compound_units.len() >= 2,
        "both equality operands have compound quantity types"
    );
    assert!(compound_units.iter().all(|unit| unit == &compound_units[0]));
    assert!(
        compound_units[0]
            .compound_terms()
            .is_some_and(|terms| !terms.is_empty()),
        "the held type identity has canonical compound terms"
    );
    let mut checked_meter = Meter::new(UNLIMITED);
    let checked = package
        .evaluate_protocol_slot(
            &selected,
            &supply(&package, "Refunds", Vec::new(), None),
            &mut checked_meter,
        )
        .expect("checked view evaluates equality of separately constructed compound units");
    assert_value_eq!(completed(checked), Value::Boolean(true));

    let read = read_checked_package(&package);
    let reader_view = read.evaluation_view().expect("the read retains its view");
    let (reader_observation, _) =
        observation_through_view(&package, &reader_view, &model_view, &["n5"], true);
    let reader = reader_view
        .evaluate_protocol_slot(
            &selected,
            &supply(&package, "Refunds", Vec::new(), Some(&reader_observation)),
            &mut Meter::new(UNLIMITED),
        )
        .expect("reader view evaluates equality of separately constructed compound units");
    assert_value_eq!(completed(reader), Value::Boolean(true));
}

/// QSL-1626: compound units formed in a protocol capture initializer are
/// retained by the package unit table and equality uses their exact identity.
#[test]
fn protocol_capture_compound_quantity_identity_is_held() {
    let source = SOURCE
        .replace(
            "model M = \"test/orders\";",
            "model M = \"test/orders\";\n\
             dimension Length;\n\
             unit m : Length = rational(1, 1);",
        )
        .replace(
            "capture order: M::Total = view.total;",
            "capture order: M::Total = view.total;\n\
             capture compoundUnit: Boolean = (1 m * 1 m) = (1 m * (1 m));",
        );
    let package = fixture_from_source(&source).0;
    let selected = slot(&package, "Refunds", "Refunds", "captures", Some(1));
    let mut meter = Meter::new(UNLIMITED);
    let result = package
        .evaluate_protocol_slot(
            &selected,
            &supply(&package, "Refunds", Vec::new(), None),
            &mut meter,
        )
        .expect("the checked protocol retains the compound unit identity");
    assert_value_eq!(completed(result), Value::Boolean(true));
}

/// FR-390-AC-26: aliases to native scalars, Quantity and Union keep the target
/// type's behavior through protocol slot reader admission.
#[test]
#[trace("TC-1291", "FR-390-AC-26")]
fn reader_backed_projection_preserves_alias_target_behavior() {
    let source = SOURCE
        .replace(
            "profile P = \"quire.protocol.finite-global/v1\";",
            "profile P = \"quire.protocol.finite-global/v1\";\n\
             profile v = \"quire.value.complete/v1\";",
        )
        .replace(
            "model M = \"test/orders\";",
            "model M = \"test/orders\";\n\
             dimension Length; unit m : Length = rational(1, 1);\n\
             union Shape { Empty, Circle(Integer), Rect(Integer, Integer) }\n\
             type Label = String; type Token = UUID; type Moment = Timestamp;\n\
             type Distance = Quantity<m>; type Form = Shape;\n\
             function labelIs using v(x: Label): Boolean pure { x = \"ok\" }\n\
             function tokenIs using v(x: Token): Boolean pure { x = UUID(\"00112233-4455-6677-8899-aabbccddeeff\") }\n\
             function momentIs using v(x: Moment): Boolean pure { x = Timestamp(5) }\n\
             function distanceIs using v(x: Distance): Boolean pure { x = 1 m }\n\
             function formIs using v(x: Form): Boolean pure { case x { Empty: true; Circle(r): false; Rect(w, h): false; } }\n\
             ",
        )
        .replace(
            "capture order: M::Total = view.total;",
            "capture order: M::Total = view.total;\n\
             capture labelAlias: Boolean = labelIs(\"ok\");\n\
             capture tokenAlias: Boolean = tokenIs(UUID(\"00112233-4455-6677-8899-aabbccddeeff\"));\n\
             capture momentAlias: Boolean = momentIs(Timestamp(5));\n\
             capture distanceAlias: Boolean = distanceIs(1 m);\n\
             capture formAlias: Boolean = formIs(Shape::Empty);",
        );
    let (package, model_view) = fixture_from_source(&source);
    let read = read_checked_package(&package);
    let reader_view = read.evaluation_view().expect("the read retains its view");
    let (observation, _) = observation(&package, &model_view);
    let (reader_observation, _) =
        observation_through_view(&package, &reader_view, &model_view, &["n5"], true);
    for index in 1..=5 {
        let selected = slot(&package, "Refunds", "Refunds", "captures", Some(index));
        let checked_supplies = supply(&package, "Refunds", Vec::new(), Some(&observation));
        let reader_supplies = supply(&package, "Refunds", Vec::new(), Some(&reader_observation));
        let mut checked_meter = Meter::new(UNLIMITED);
        let checked = package
            .evaluate_protocol_slot(&selected, &checked_supplies, &mut checked_meter)
            .expect("checked view evaluates the aliased target type");
        assert_value_eq!(completed(checked), Value::Boolean(true));
        let mut reader_meter = Meter::new(UNLIMITED);
        let reader = reader_view
            .evaluate_protocol_slot(&selected, &reader_supplies, &mut reader_meter)
            .expect("reader view evaluates the aliased target type");
        assert_value_eq!(completed(reader), Value::Boolean(true));
        charges_and_usage_match("alias target behavior", &checked_meter, &reader_meter);
    }
}

/// FR-390-AC-27: the reader projection retains exact Union/member identity and
/// positional payload when a case selects each active constructor.
#[test]
#[trace("TC-1292", "FR-390-AC-27")]
fn reader_backed_projection_preserves_union_identity_and_case_payload_order() {
    let source = SOURCE
        .replace(
            "profile P = \"quire.protocol.finite-global/v1\";",
            "profile P = \"quire.protocol.finite-global/v1\";\n\
             profile v = \"quire.value.complete/v1\";",
        )
        .replace(
            "model M = \"test/orders\";",
            "model M = \"test/orders\";\n\
             union Shape { Empty, Circle(Integer), Square(Integer), Rect(Integer, Integer) }\n\
             function area using v(shape: Shape): Integer pure { case shape { Empty: 0; Circle(r): 100 + r; Square(s): 200 + s; Rect(w, h): w * h; } }",
        )
        .replace(
            "capture order: M::Total = view.total;",
            "capture order: M::Total = view.total;\n\
             capture empty: Integer = area(Shape::Empty);\n\
             capture circle: Integer = area(Shape::Circle(2));\n\
             capture square: Integer = area(Shape::Square(2));\n\
             capture rectangle: Integer = area(Shape::Rect(2, 3));",
        );
    let (package, model_view) = fixture_from_source(&source);
    let read = read_checked_package(&package);
    let reader_view = read.evaluation_view().expect("the read retains its view");
    let (observation, _) = observation(&package, &model_view);
    let (reader_observation, _) =
        observation_through_view(&package, &reader_view, &model_view, &["n5"], true);
    for (index, expected) in [0, 102, 202, 6].into_iter().enumerate() {
        let selected = slot(&package, "Refunds", "Refunds", "captures", Some(index + 1));
        let checked_supplies = supply(&package, "Refunds", Vec::new(), Some(&observation));
        let reader_supplies = supply(&package, "Refunds", Vec::new(), Some(&reader_observation));
        let mut checked_meter = Meter::new(UNLIMITED);
        let checked = package
            .evaluate_protocol_slot(&selected, &checked_supplies, &mut checked_meter)
            .expect("checked view evaluates the selected Union arm");
        assert_value_eq!(completed(checked), int(expected));
        let mut reader_meter = Meter::new(UNLIMITED);
        let reader = reader_view
            .evaluate_protocol_slot(&selected, &reader_supplies, &mut reader_meter)
            .expect("reader view evaluates the selected Union arm");
        assert_value_eq!(completed(reader), int(expected));
        charges_and_usage_match("union arm payload", &checked_meter, &reader_meter);
    }
}

/// TC-1073: one meter passed to the reader-backed (plain-data root) slots in
/// turn accumulates the sum of their charges on both views; a `work_units`
/// budget of the sum completes all of them, one below returns Incomplete at the
/// last call, and the reverse order gives the same total. AC-10 names the
/// registration capture, activation capture, trigger guard, retry relation and
/// recovery predicate; the last three are composed roots with no reader
/// projection yet, so this test does not carry the AC-10 tag.
#[test]
#[trace("TC-1073")]
fn reader_backed_shared_meter_sums_and_bounds_in_both_orders() {
    let (package, model_view) = fixture_with_view();
    let read = read_checked_package(&package);
    let reader_view = read.evaluation_view().expect("the read retains its view");
    let (checked_observation, reference) = observation(&package, &model_view);
    let (reader_observation, _) =
        observation_through_view(&package, &reader_view, &model_view, &["n5"], true);
    let specs = parity_slots(&package);
    assert!(specs.len() >= 8);
    let selections: Vec<_> = specs
        .iter()
        .map(|spec| slot(&package, spec.protocol, spec.owner, spec.member, spec.index))
        .collect();
    let supplies = |observation| -> Vec<_> {
        specs
            .iter()
            .map(|spec| {
                supply(
                    &package,
                    spec.protocol,
                    (spec.values)(Some(&reference)),
                    spec.observed.then_some(observation),
                )
            })
            .collect()
    };
    let checked_supplies = supplies(&checked_observation);
    let reader_supplies = supplies(&reader_observation);
    let run = |view_is_reader: bool, order: &[usize], limits: ScalarLimits| {
        let mut meter = Meter::new(limits);
        let mut results = Vec::new();
        for index in order {
            results.push(if view_is_reader {
                reader_view.evaluate_protocol_slot(
                    &selections[*index],
                    &reader_supplies[*index],
                    &mut meter,
                )
            } else {
                package.evaluate_protocol_slot(
                    &selections[*index],
                    &checked_supplies[*index],
                    &mut meter,
                )
            });
        }
        (meter, results)
    };
    // The work_units each call adds, measured alone, locates the last call
    // that charges.
    let work = |index: usize| {
        let (meter, _) = run(true, &[index], UNLIMITED);
        meter.consumed(LimitKind::WorkUnits)
    };
    let calls = selections.len();
    let forward: Vec<usize> = (0..calls).collect();
    let reversed: Vec<usize> = forward.iter().rev().copied().collect();
    let (checked_meter, _) = run(false, &forward, UNLIMITED);
    let (reader_meter, _) = run(true, &forward, UNLIMITED);
    charges_and_usage_match("forward sum", &checked_meter, &reader_meter);
    let (reversed_meter, _) = run(true, &reversed, UNLIMITED);
    for counter in LimitKind::ALL {
        assert_eq!(
            reversed_meter.consumed(counter),
            reader_meter.consumed(counter),
            "the reverse order gives the same {counter:?} total"
        );
    }
    let sum = reader_meter.consumed(LimitKind::WorkUnits);
    assert!(sum > 0);
    for order in [&forward, &reversed] {
        let mut at_sum = UNLIMITED;
        set_limit(&mut at_sum, LimitKind::WorkUnits, sum);
        let (meter, results) = run(true, order, at_sum);
        for result in results {
            completed(result.unwrap());
        }
        assert_eq!(meter.consumed(LimitKind::WorkUnits), sum);
        let last_charging = order
            .iter()
            .rposition(|index| work(*index) > 0)
            .expect("a call charges work_units");
        let mut one_short = UNLIMITED;
        set_limit(&mut one_short, LimitKind::WorkUnits, sum - 1);
        for reader in [true, false] {
            let (_, results) = run(reader, order, one_short);
            for (position, result) in results.into_iter().enumerate() {
                if position < last_charging {
                    completed(result.unwrap());
                } else if position == last_charging {
                    assert_incomplete(result.unwrap(), LimitKind::WorkUnits);
                }
            }
        }
    }
}

/// FR-390-AC-6: a foreign reference nested in a binder whose declared type is
/// not itself a Reference has no single required universe, so the
/// `foreign_reference` record carries `supplied` alone.
#[test]
#[trace("TC-1071", "FR-390-AC-6")]
fn foreign_reference_in_a_non_reference_binder_omits_the_required_universe() {
    let (package, view) = fixture_ep();
    let peer = other_reference(&view, "ghost");
    let nested = other_reference(&view, "nested-1");
    let optional = other_reference(&view, "option-1");
    let sequence = other_reference(&view, "sequence-1");
    let foreign_document = serde_json::to_string(&document_ep())
        .unwrap()
        .replace("test/orders", "test/foreign");
    let (_foreign_package, foreign_view) = fixture_from_document(
        &SOURCE.replace("test/orders", "test/foreign"),
        foreign_document.as_bytes(),
    );
    let foreign_key = quire_checked::key::DeclarationKey {
        package: "test/foreign".to_owned(),
        node: "ix://test/foreign/Node".to_owned(),
    };
    let foreign_universe = foreign_view
        .object_universe_of(&foreign_key)
        .unwrap()
        .identity()
        .unwrap();
    assert_ne!(&foreign_universe, peer.universe());
    let foreign_peer = ObjectReference::new(
        foreign_universe.clone(),
        peer.object_type().clone(),
        ObjectId::new("ghost").unwrap(),
    );
    let receipt = ep_receipt(&package, [&foreign_peer, &nested, &optional, &sequence]);
    let refs = vec![ProtocolReferenceOccurrence {
        path: vec![PathStep::Field("peer".into())],
        population: population_node("otherSpare"),
    }];
    let observation = ep_observation(
        &package,
        &view,
        Some((true, &["ghost", "nested-1", "option-1", "sequence-1"])),
        true,
    );
    let mut meter = Meter::new(UNLIMITED);
    let before = meter.clone();
    let result = package.evaluate_protocol_slot(
        &slot(
            &package,
            "Refunds",
            "Refund",
            "registration_captures",
            Some(0),
        ),
        &ep_supply(&package, receipt, refs, Some(&observation)),
        &mut meter,
    );
    assert_eq!(
        meter, before,
        "refusal leaves every meter counter unchanged"
    );
    let Err(CallFailure::Admission(failure)) = result else {
        panic!("expected a foreign-reference refusal: {result:?}");
    };
    let CallAdmissionFailure::Refused(record) = failure.as_ref() else {
        panic!("foreign reference is a typed refusal: {failure:?}");
    };
    assert_eq!(record.record.catalog.code(), "foreign_reference");
    assert_eq!(record.record.catalog.cause(), "foreign-universe");
    assert_eq!(
        record.locus,
        CallAdmissionLocus::Binder(binder(&package, "Refunds", "forward"))
    );
    assert!(
        !record.record.fields.contains_key("required"),
        "a non-Reference binder names no required universe: {:?}",
        record.record.fields
    );
    let supplied = independent_universe_hex(&foreign_universe);
    assert_eq!(
        record.record.fields.get("supplied").map(String::as_str),
        Some(supplied.as_str())
    );
}

/// FR-390-AC-14: each selected source reports its binder's own first
/// occurrence, not the selected expression's root location.
#[test]
#[trace("FR-390-AC-14")]
fn protocol_source_location_is_the_binders_first_occurrence() {
    let package = fixture();
    let view = package.evaluation_view();
    let capture = slot(&package, "Refunds", "Refunds", "captures", Some(0));
    let selected = view
        .select_protocol_slot(&qsl_package::ProtocolSlotDescriptor::new(
            capture.node.clone(),
            capture.member.clone(),
            capture.index,
        ))
        .expect("the view selects the capture expression");
    let root_location = selected.checked_body().root().location().clone();
    assert!(!selected.sources().is_empty());
    for source in selected.sources() {
        let expected = package
            .graph()
            .occurrences()
            .find(|(key, _, _)| key == source.binder())
            .map(|(_, _, location)| location.clone())
            .expect("every lowered binder has an occurrence");
        assert_eq!(source.location(), &expected);
        assert_ne!(
            source.location(),
            &root_location,
            "the source location is not the expression root's"
        );
    }
}

/// FR-390-AC-14: a selected opaque term cannot be evaluated against a
/// different checked-package receiver, even when the checked graph shapes
/// are equivalent.
#[test]
#[trace("FR-390-AC-14")]
fn selected_protocol_term_is_bound_to_its_receiver_package_and_view() {
    let first_package = fixture();
    let first_view = first_package.evaluation_view();
    let capture = slot(&first_package, "Refunds", "Refunds", "captures", Some(0));
    let descriptor = qsl_package::ProtocolSlotDescriptor::new(
        capture.node.clone(),
        capture.member.clone(),
        capture.index,
    );
    let selected = first_view
        .select_protocol_slot(&descriptor)
        .expect("the first view selects the checked capture expression");

    let second_package = fixture();
    let second_view = second_package.evaluation_view();
    let supply = ProtocolSupply {
        binders: Vec::new(),
        observation: None,
    };
    let mut meter = Meter::new(UNLIMITED);
    let before = meter.clone();
    let alternate_view = first_package.evaluation_view();
    let same_package_other_view = first_package.evaluate_selected_protocol_slot(
        &alternate_view,
        &selected,
        &supply,
        &mut meter,
    );
    assert_eq!(
        meter, before,
        "view ownership refusal is before all charges"
    );
    let Err(CallFailure::Admission(failure)) = same_package_other_view else {
        panic!("another view of the same package must refuse: {same_package_other_view:?}");
    };
    let CallAdmissionFailure::Refused(record) = failure.as_ref() else {
        panic!("view ownership is a typed refusal: {failure:?}");
    };
    assert_eq!(record.record.catalog.code(), "invalid_model_binding");
    assert_eq!(record.record.catalog.cause(), "wrong-model-selection");

    let result = second_package.evaluate_selected_protocol_slot(
        &second_view,
        &selected,
        &supply,
        &mut meter,
    );
    assert_eq!(meter, before, "ownership refusal is before all charges");
    let Err(CallFailure::Admission(failure)) = result else {
        panic!("a foreign selected term must refuse: {result:?}");
    };
    let CallAdmissionFailure::Refused(record) = failure.as_ref() else {
        panic!("selected-term ownership is a typed refusal: {failure:?}");
    };
    assert_eq!(record.record.catalog.code(), "invalid_model_binding");
    assert_eq!(record.record.catalog.cause(), "wrong-model-selection");
    assert_eq!(
        record.locus,
        CallAdmissionLocus::Node(selected.root().node().clone())
    );
}

fn independent_universe_hex(universe: &quire_exact::UniverseId) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let identity = qsl_foundation::node_wire::node_key_to_text(universe.root_type()).unwrap();
    let mut output = String::with_capacity(identity.len().saturating_mul(2));
    for byte in identity.as_bytes() {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

/// TC-1071 row 20: an incomplete sole population is named before checking the
/// supplied reference's membership, with no S6a charge.
#[test]
#[trace(
    "TC-1071",
    "FR-390-AC-6",
    "FR-390-AC-20",
    "FR-390-AC-21",
    "FR-106-AC-62"
)]
fn incomplete_sole_population_is_returned_with_its_population_locus() {
    let (package, view) = fixture_with_view();
    let (observation, reference) = observation_with_objects(&package, &view, &["n5"], false);
    let slot = slot(&package, "Refunds", "Refunds", "captures", Some(0));
    let mut meter = Meter::new(UNLIMITED);
    let before = meter.clone();
    let result = package.evaluate_protocol_slot(
        &slot,
        &supply(
            &package,
            "Refunds",
            vec![("view", Value::Reference(reference))],
            Some(&observation),
        ),
        &mut meter,
    );
    assert_eq!(meter, before);
    let Err(CallFailure::Admission(failure)) = result else {
        panic!("expected the incomplete population admission result: {result:?}");
    };
    let CallAdmissionFailure::Incomplete(record) = failure.as_ref() else {
        panic!("expected an incomplete admission: {failure:?}");
    };
    assert_eq!(
        record.record.catalog,
        CatalogCode::new("incomplete_population", "incomplete-scope")
    );
    assert_eq!(record.record.fields["population"], model::id("nodes"));
    assert_eq!(
        record.locus,
        CallAdmissionLocus::Population(
            NodeKey::new(
                model::PACKAGE,
                vec![
                    quire_exact::PathSegment::Text("relation".to_owned()),
                    quire_exact::PathSegment::Text("population".to_owned()),
                    quire_exact::PathSegment::Text(model::id("nodes")),
                ],
                0,
            )
            .unwrap()
        )
    );
}

/// FR-106-AC-65: a supplied ghost reference attributed to `nodes` makes that
/// declaration required. With no `nodes` row the admission is
/// `incomplete_population`/`missing-required-artifact`, with the row listed
/// but not complete it is `incomplete-scope`, both at that population before
/// check 8; with the row complete, check 8 reports the ghost against it.
#[test]
#[trace("TC-1071", "FR-106-AC-65")]
fn a_seed_population_without_a_row_is_a_missing_required_artifact() {
    let (package, view) = fixture_with_view();
    let evaluation_view = package.evaluation_view();
    let capture = slot(&package, "Refunds", "Refunds", "captures", Some(0));
    for (row, incomplete, code) in [
        (
            None,
            true,
            CatalogCode::new("incomplete_population", "missing-required-artifact"),
        ),
        (
            Some(false),
            true,
            CatalogCode::new("incomplete_population", "incomplete-scope"),
        ),
        (
            Some(true),
            false,
            CatalogCode::new("dangling_reference", "absent-target-in-complete-population"),
        ),
    ] {
        let (observation, reference) =
            observation_with_row(&package, &evaluation_view, &view, &["n5"], row);
        let ghost = ObjectReference::new(
            reference.universe().clone(),
            reference.object_type().clone(),
            ObjectId::new("ghost").unwrap(),
        );
        let mut meter = Meter::new(UNLIMITED);
        let before = meter.clone();
        let result = package.evaluate_protocol_slot(
            &capture,
            &supply(
                &package,
                "Refunds",
                vec![("view", Value::Reference(ghost))],
                Some(&observation),
            ),
            &mut meter,
        );
        assert_eq!(
            meter, before,
            "row {row:?}: admission precedes every charge"
        );
        let Err(CallFailure::Admission(failure)) = result else {
            panic!("row {row:?}: expected an admission failure: {result:?}");
        };
        let record = match failure.as_ref() {
            CallAdmissionFailure::Incomplete(record) if incomplete => record,
            CallAdmissionFailure::Refused(record) if !incomplete => record,
            other => panic!("row {row:?}: wrong admission outcome: {other:?}"),
        };
        assert_eq!(record.record.catalog, code, "row {row:?}");
        assert_eq!(
            record.record.fields["population"],
            model::id("nodes"),
            "row {row:?}"
        );
    }
}

/// FR-390-AC-20: a successful immutable observation authority remains usable
/// when an unrelated population is incomplete; completing that population
/// does not change the call result or ordered charges.
#[test]
#[trace("TC-1071", "FR-390-AC-20")]
fn unrelated_incomplete_population_does_not_change_a_successful_slot() {
    let (package, view) = fixture_ep();
    let peer = other_reference(&view, "ghost");
    let nested = other_reference(&view, "nested-1");
    let optional = other_reference(&view, "option-1");
    let sequence = other_reference(&view, "sequence-1");
    let receipt = ep_receipt(&package, [&peer, &nested, &optional, &sequence]);
    let paths = vec![
        (vec![PathStep::Field("peer".into())], "ghost"),
        (
            vec![
                PathStep::Field("bundle".into()),
                PathStep::Field("nested".into()),
            ],
            "nested-1",
        ),
        (
            vec![
                PathStep::Field("bundle".into()),
                PathStep::Field("optional".into()),
                PathStep::OptionValue,
            ],
            "option-1",
        ),
        (
            vec![
                PathStep::Field("bundle".into()),
                PathStep::Field("sequence".into()),
                PathStep::Index(0),
            ],
            "sequence-1",
        ),
    ];
    let references: Vec<ProtocolReferenceOccurrence> = paths
        .into_iter()
        .map(|(path, _)| ProtocolReferenceOccurrence {
            path,
            population: population_node("otherSpare"),
        })
        .collect();
    let keys = ["ghost", "nested-1", "option-1", "sequence-1"];
    let unrelated_incomplete =
        ep_observation_state(&package, Some((true, &keys)), false, None, None, &[]);
    let all_complete = ep_observation_state(&package, Some((true, &keys)), true, None, None, &[]);
    assert!(matches!(
        &unrelated_incomplete,
        ObservationAdmission::Admitted(_)
    ));
    assert!(matches!(&all_complete, ObservationAdmission::Admitted(_)));

    let selected = slot(
        &package,
        "Refunds",
        "Refund",
        "registration_captures",
        Some(0),
    );
    let evaluate =
        |observation: &ObservationAdmission<qsl_package::AdmittedProtocolObservation<'_>>| {
            let mut meter = Meter::new(UNLIMITED);
            let result = package.evaluate_protocol_slot(
                &selected,
                &ep_supply(
                    &package,
                    receipt.clone(),
                    references.clone(),
                    Some(observation),
                ),
                &mut meter,
            );
            assert!(
                matches!(result, Ok(ProtocolSlotEvaluation::Evaluated(_, _))),
                "the selected capture evaluates: {result:?}"
            );
            (without_location(&result), meter.admitted_charges().to_vec())
        };
    assert_eq!(
        evaluate(&unrelated_incomplete),
        evaluate(&all_complete),
        "only the unrelated population's completeness changes"
    );
}

/// TC-1071 row 21 / FR-106 AC-63: the S4 admission arm preserves the original
/// wrong-model record when the selected snapshot belongs to another model.
#[test]
#[trace("TC-1071", "FR-390-AC-6", "FR-390-AC-21", "FR-106-AC-63")]
fn another_model_snapshot_returns_the_original_qsv_refusal() {
    let (package, model_view) = fixture_with_view();
    let selected = DocumentRef {
        authority: "test".to_owned(),
        identity: "foreign".to_owned(),
    };
    let local_bytes = serde_json::to_vec(&document()).unwrap();
    let foreign_bytes = String::from_utf8(local_bytes.clone())
        .unwrap()
        .replace("test/orders", "test/foreign")
        .into_bytes();
    let packages = qsl_semantics::model::intake::package_input([
        local_bytes.as_slice(),
        foreign_bytes.as_slice(),
    ])
    .unwrap();
    let snapshot = serde_json::json!({
        "format": "quire.state.snapshot/v1",
        "identity": {
            "authority": selected.authority.clone(),
            "identity": selected.identity.clone(),
            },
        "observation": "current",
        "model": {"identity": "test/foreign", "version": "1.0.0", "digest": "set at admission"},
        "populations": [{
            "population": "ix://test/foreign/nodes",
            "complete": true,
            "objects": [{
                "key": "n5",
                "type": "ix://test/foreign/Node",
                "fields": {"total": {"integer": "5"}},
            }],
        }],
    });
    let snapshots = [(selected.clone(), serde_json::to_vec(&snapshot).unwrap())]
        .into_iter()
        .collect();
    let admission = package.evaluation_view().admit_protocol_observation(
        &package.graph().model_selections()[0],
        &packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        &snapshots,
        &selected,
        ObservationLimits::default(),
    );
    let ObservationAdmission::Refused(record) = &admission else {
        panic!("expected the original wrong-model refusal: {admission:?}");
    };
    assert_eq!(
        record.catalog,
        quire_semantic_value::catalog::CatalogCode::new(
            "invalid_model_binding",
            "wrong-model-selection"
        )
    );
    let expected = record.clone();
    let (_, reference) = observation(&package, &model_view);
    let capture = slot(&package, "Refunds", "Refunds", "captures", Some(0));
    let mut meter = Meter::new(UNLIMITED);
    let before = meter.clone();
    let result = package.evaluate_protocol_slot(
        &capture,
        &supply(
            &package,
            "Refunds",
            vec![("view", Value::Reference(reference))],
            Some(&admission),
        ),
        &mut meter,
    );
    assert_eq!(meter, before, "a refused observation charges nothing");
    let Err(CallFailure::Admission(failure)) = result else {
        panic!("the original observation refusal is returned: {result:?}");
    };
    let CallAdmissionFailure::Refused(returned) = failure.as_ref() else {
        panic!("a refused observation remains refused: {failure:?}");
    };
    assert_eq!(
        returned.record, expected,
        "no foreign-reference replacement"
    );
}

/// FR-390-AC-21: an observation fault remains the original internal fault
/// when supplied to a protocol-slot call and charges no evaluation meter.
#[test]
#[trace("TC-1071", "FR-390-AC-21")]
fn observation_fault_is_preserved_without_charging_the_meter() {
    use quire_semantic_value::catalog::InternalFault;

    let (package, model_view) = fixture_with_view();
    let fault = InternalFault::new("test-observation", "broken-authority");
    let observation_result = ObservationAdmission::Fault(fault.clone());
    let (_, reference) = observation(&package, &model_view);
    let capture = slot(&package, "Refunds", "Refunds", "captures", Some(0));
    let mut meter = Meter::new(UNLIMITED);
    let before = meter.clone();
    let result = package.evaluate_protocol_slot(
        &capture,
        &supply(
            &package,
            "Refunds",
            vec![("view", Value::Reference(reference))],
            Some(&observation_result),
        ),
        &mut meter,
    );
    assert_eq!(meter, before, "an observation fault charges nothing");
    let Err(CallFailure::Fault(returned)) = result else {
        panic!("the original observation fault is returned: {result:?}");
    };
    assert_eq!(returned, fault);
}

fn bare_rows(
    package: &CheckedPackage,
    v2: &qsl_package::CheckedPackageV2,
) -> qsl_package::ReadIdentityRows {
    qsl_package::emit_checked(package, EmitLimits::fixture())
        .expect("emits")
        .identity_rows(
            v2,
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("final identity rows")
}

/// FR-533-AC-1 (selection half), AC-4 (selection path), with the bytes-route
/// site names equal to the source selector's: over every slot kind of TC-1070
/// the selection read from the package's nodes alone names the root and the
/// binders (name, level, type, key) the source selector names.
#[trace("FR-533-AC-4", "FR-533-AC-12", "TC-1333")]
#[test]
fn the_bare_selection_equals_the_source_selection_for_every_tc1070_slot() {
    let package = fixture();
    let read = read_checked_package(&package);
    let v2 = read.admitted();
    let rows = bare_rows(&package, v2);
    let equality = qsl_package::emit_checked(&package, EmitLimits::fixture())
        .expect("emits")
        .equality_witness(
            v2,
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("fixture allowance materializes equality witness");
    let bare = qsl_package::BarePackage::new(v2)
        .with_supply(
            qsl_package::BareSupply {
                identities: &rows,
                equality: Some(&equality),
            },
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply");
    let view = package.evaluation_view();
    let row_origins: std::collections::BTreeSet<String> = v2
        .checked_term_data()
        .unwrap()
        .protocol_locations
        .iter()
        .map(|row| format!("{:?}", row.location.origin))
        .collect();
    let mut kinds = std::collections::BTreeSet::new();
    for parity in parity_slots(&package) {
        let wanted = slot(
            &package,
            parity.protocol,
            parity.owner,
            parity.member,
            parity.index,
        );
        let descriptor = qsl_package::ProtocolSlotDescriptor::new(
            wanted.node.clone(),
            wanted.member.clone(),
            wanted.index,
        );
        let source = view
            .select_protocol_slot(&descriptor)
            .expect("source selects");
        // The source selector reads a compensation node's slots from the
        // template at the node's position in the id-sorted `keys`, a
        // position the source-ordered template list does not share (the
        // node `slot()` returns for "Refund" is declared "Partial"). The
        // bare selection reads the node's own body, so it is compared
        // through the node whose declaration names the owner.
        let node = if parity.protocol == parity.owner {
            wanted.node.clone()
        } else {
            let declared = v2
                .resolved_graph()
                .nodes
                .iter()
                .find(|node| {
                    node.declaration.as_ref().is_some_and(|d| {
                        d.qualified_name.last().map(|s| &**s) == Some(parity.owner)
                            && d.qualified_name.first().map(|s| &**s) == Some(parity.protocol)
                    }) && &*node.node_tag == "protocol"
                })
                .expect("the owner is declared");
            let ids = qsl_semantics::check::wire_id::WireGraph::new(
                v2.resolved_graph().nodes.iter().map(|node| &node.node_id),
            )
            .expect("wire ids")
            .key(&declared.node_id)
            .expect("key");
            if &*declared.semantic_form == "compensation" {
                ids
            } else {
                wanted.node.clone()
            }
        };
        let bare_descriptor =
            qsl_package::ProtocolSlotDescriptor::new(node, wanted.member.clone(), wanted.index);
        let chosen = bare
            .select_protocol_slot(&bare_descriptor)
            .unwrap_or_else(|failure| panic!("{}: {failure:?}", parity.name));
        assert_eq!(
            chosen.root(),
            source.root().node(),
            "{} owner {} node {:?}",
            parity.name,
            parity.owner,
            descriptor.node()
        );
        let mut expected: Vec<_> = source
            .sources()
            .iter()
            .map(|s| {
                (
                    s.level(),
                    s.binder().clone(),
                    s.name().to_owned(),
                    s.value_type().clone(),
                )
            })
            .collect();
        expected.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
        let got: Vec<_> = chosen
            .sources()
            .iter()
            .map(|s| {
                (
                    s.level(),
                    s.binder().clone(),
                    s.name().to_owned(),
                    s.value_type().clone(),
                )
            })
            .collect();
        assert_eq!(got, expected, "{}", parity.name);
        assert_eq!(chosen.frame_len(), got.len());
        // The site name the source selector locates the slot under.
        let source_origin = format!("{:?}", source.checked_body().root().location().origin);
        // A slot whose root holds rows has exactly one origin, never none.
        assert!(
            !row_origins.contains(&source_origin) || chosen.origin().is_some(),
            "{}: the slot's rows exist under {source_origin} but the bytes route found no origin",
            parity.name
        );
        // FR-533-AC-12: exactly one origin, the source route's.
        if let Some(origin) = chosen.origin() {
            assert_eq!(
                format!("TypeDeclaration {{ name: {origin:?} }}"),
                source_origin,
                "{}",
                parity.name
            );
            assert!(
                row_origins.contains(&source_origin) || origin.ends_with(" body"),
                "{}: {origin}",
                parity.name
            );
        }
        kinds.insert(parity.member);
    }
    assert!(kinds.len() >= 12, "{kinds:?}");
}

/// FR-531-AC-5: rows produced for another package are `Foreign`. `Unknown` is
/// not reachable through a package read: the intake join drops a row naming a
/// node the package does not hold, so IR's `from_stage_rows` only fails for a
/// row the join cannot reconcile, which this fixture cannot produce; it is
/// not claimed here.
#[trace("FR-531-AC-5", "TC-1331")]
#[test]
fn rows_of_another_package_are_a_foreign_identity_witness() {
    use qsl_package::{BareSelectionFailure, IdentityWitnessRefusal};
    let package = fixture();
    let read = read_checked_package(&package);
    let rows = bare_rows(&package, read.admitted());
    let (other, _) = fixture_named(SOURCE, &serde_json::to_vec(&document()).unwrap(), "other");
    let domain = qsl_semantics::model::intake::package_input([serde_json::to_vec(&document())
        .unwrap()
        .as_slice()])
    .unwrap();
    let other_read = qsl_package::read_evaluation_package(
        &other,
        qsl_semantics::library::LibraryName::new("other").unwrap(),
        &domain,
        qsl_package::V2ReadLimits::fixture(),
        EmitLimits::fixture(),
    )
    .expect("the other package is admitted");
    let bare = qsl_package::BarePackage::new(other_read.admitted())
        .with_supply(
            qsl_package::BareSupply {
                identities: &rows,
                equality: None,
            },
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply");
    let clause = slot(&other, "Refunds", "Refunds", "activation.guard", None).node;
    let result = bare.select_protocol_slot(&qsl_package::ProtocolSlotDescriptor::new(
        clause,
        "activation.guard",
        None,
    ));
    assert!(matches!(
        result,
        Err(BareSelectionFailure::Identity(
            IdentityWitnessRefusal::Foreign
        ))
    ));
}

/// FR-531-AC-5 (Missing) and FR-390 selection rules 1 and 2 over the
/// bare package: no supply is `Missing` before any node is read; an unknown
/// node, a non-protocol node, an absent member and an index defect refuse as
/// the source selector does.
#[trace("FR-531-AC-4", "TC-1331")]
#[test]
fn the_bare_selection_refuses_as_the_source_selection_does() {
    use qsl_package::{BareSelectionFailure, IdentityWitnessRefusal, SlotSelectionFailure};
    let package = fixture();
    let read = read_checked_package(&package);
    let v2 = read.admitted();
    let rows = bare_rows(&package, v2);
    let bare = qsl_package::BarePackage::new(v2)
        .with_supply(
            qsl_package::BareSupply {
                identities: &rows,
                equality: None,
            },
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply");
    let view = package.evaluation_view();
    let clause = slot(&package, "Refunds", "Refunds", "activation.guard", None).node;
    let descriptor = |member: &str, index| {
        qsl_package::ProtocolSlotDescriptor::new(clause.clone(), member, index)
    };
    let unsupplied = qsl_package::BarePackage::new(v2)
        .select_protocol_slot(&descriptor("activation.guard", None));
    assert!(matches!(
        unsupplied,
        Err(BareSelectionFailure::Identity(
            IdentityWitnessRefusal::Missing
        ))
    ));
    for (member, index) in [
        ("activation.guard", Some(0)),
        ("captures", None),
        ("captures", Some(9)),
        ("nothing", None),
    ] {
        let d = descriptor(member, index);
        let want = view.select_protocol_slot(&d).err().expect("source refuses");
        let got = bare.select_protocol_slot(&d).expect_err("bare refuses");
        assert_eq!(
            format!("{want:?}"),
            match got {
                BareSelectionFailure::Slot(failure) => format!("{failure:?}"),
                other => panic!("{other:?}"),
            }
        );
    }
    let expression = view
        .select_protocol_slot(&descriptor("activation.guard", None))
        .expect("source selects")
        .root()
        .node()
        .clone();
    let d = qsl_package::ProtocolSlotDescriptor::new(expression, "x", None);
    let want = view.select_protocol_slot(&d).err().expect("source refuses");
    assert!(matches!(want, SlotSelectionFailure::MalformedNode(_)));
    assert!(matches!(
        bare.select_protocol_slot(&d),
        Err(BareSelectionFailure::Slot(
            SlotSelectionFailure::MalformedNode(_)
        ))
    ));
}

/// FR-533-AC-13, FR-390-AC-15, FR-383-AC-63: a fault inside an
/// application-rooted `related` end is reported at the location of the end's
/// own node, and that location is one value on the checked view and on the
/// bytes route, under the position name.
#[trace("FR-533-AC-13", "FR-390-AC-15", "FR-383-AC-63", "TC-1333")]
#[test]
fn a_related_call_is_located_alike_on_both_routes() {
    let source = SOURCE
        .replace("profile S = \"quire.state.graph/v1\";", "profile S = \"quire.state.graph/v1\";\nprofile v = \"quire.value.complete/v1\";")
        .replace("model M = \"test/orders\";", "model M = \"test/orders\";\nfunction identityRef using v(x: M::Node): M::Node pure { x }")
        .replace("related by Owns (view, view)", "related by Owns (identityRef(view), identityRef(view))");
    assert_ne!(source, SOURCE);
    let (package, _view) = fixture_from_source(&source);
    let read = read_checked_package(&package);
    let v2 = read.admitted();
    let rows = bare_rows(&package, v2);
    let equality = qsl_package::emit_checked(&package, EmitLimits::fixture())
        .expect("emits")
        .equality_witness(
            v2,
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("budgeted equality witness");
    let bare = qsl_package::BarePackage::new(v2)
        .with_supply(
            qsl_package::BareSupply {
                identities: &rows,
                equality: Some(&equality),
            },
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply");
    let view = package.evaluation_view();
    for (member, side) in [("related.from", "from"), ("related.to", "to")] {
        let wanted = slot(&package, "Pipeline", "Got", member, Some(0));
        let descriptor =
            qsl_package::ProtocolSlotDescriptor::new(wanted.node, member, wanted.index);
        let checked = view.select_protocol_slot(&descriptor).expect("selects");
        let selected = bare.select_protocol_slot(&descriptor).expect("selects");
        let projection = bare.project_protocol_slot(&selected).expect("projects");
        let on_bare = projection
            .location(projection.root())
            .expect("the call is located");
        assert_eq!(
            checked.checked_body().root().location(),
            on_bare,
            "{member}: one location on both routes"
        );
        assert_eq!(
            on_bare.origin,
            quire_semantic_value::location::ExpressionOwner::TypeDeclaration {
                name: format!("protocol Pipeline run Main Got related 0 {side}"),
            }
        );
    }
}

/// FR-533-AC-21, FR-383-AC-64: over the compiled two-record variant of fixture
/// L, in each source order of its two clauses, the checked view and the bytes
/// route locate `Got.related.from[i]` and the `to` end that is a call alike,
/// under `related <i> from` and `related <k> to` (`k` the emitted position of
/// the call's record, equal in both orders); the two `from` ends differ and
/// no location is under a span-named `related` origin.
#[trace("FR-533-AC-21", "FR-383-AC-64", "TC-1333", "TC-1063")]
#[test]
fn the_two_record_related_variant_is_located_alike_in_each_clause_order() {
    let call_both = "related by Owns (identityRef(view), identityRef(view))";
    let call_from = "related by Owns (identityRef(view), view)";
    let mut call_to_positions = Vec::new();
    for clauses in [
        format!("{call_from} {call_both}"),
        format!("{call_both} {call_from}"),
    ] {
        let source = SOURCE
            .replace("profile S = \"quire.state.graph/v1\";", "profile S = \"quire.state.graph/v1\";\nprofile v = \"quire.value.complete/v1\";")
            .replace("model M = \"test/orders\";", "model M = \"test/orders\";\nfunction identityRef using v(x: M::Node): M::Node pure { x }")
            .replace("related by Owns (view, view)", &clauses);
        assert_ne!(source, SOURCE);
        let (package, _view) = fixture_from_source(&source);
        let read = read_checked_package(&package);
        let v2 = read.admitted();
        let rows = bare_rows(&package, v2);
        let equality = qsl_package::emit_checked(&package, EmitLimits::fixture())
            .expect("emits")
            .equality_witness(
                v2,
                &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
            )
            .expect("budgeted equality witness");
        let bare = qsl_package::BarePackage::new(v2)
            .with_supply(
                qsl_package::BareSupply {
                    identities: &rows,
                    equality: Some(&equality),
                },
                &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
            )
            .expect("the fixture budget admits the supply");
        let view = package.evaluation_view();
        let located = |member: &str, index: usize| {
            let wanted = slot(&package, "Pipeline", "Got", member, Some(index));
            let descriptor =
                qsl_package::ProtocolSlotDescriptor::new(wanted.node, member, wanted.index);
            let checked = view.select_protocol_slot(&descriptor).expect("selects");
            let selected = bare.select_protocol_slot(&descriptor).expect("selects");
            let on_bare = selected.origin().map(|_| {
                let projection = bare.project_protocol_slot(&selected).expect("projects");
                projection
                    .location(projection.root())
                    .expect("the call is located")
                    .clone()
            });
            (checked.checked_body().root().location().clone(), on_bare)
        };
        let name = |position: usize, side: &str| {
            quire_semantic_value::location::ExpressionOwner::TypeDeclaration {
                name: format!("protocol Pipeline run Main Got related {position} {side}"),
            }
        };
        let mut from = Vec::new();
        for index in 0..2 {
            let (checked, bare) = located("related.from", index);
            assert_eq!(Some(&checked), bare.as_ref(), "from[{index}]");
            assert_eq!(checked.origin, name(index, "from"));
            from.push(checked);
        }
        assert_ne!(from[0], from[1]);
        let mut calls = Vec::new();
        for index in 0..2 {
            let (checked, bare) = located("related.to", index);
            if let Some(bare) = bare {
                assert_eq!(checked, bare, "to[{index}]");
                assert_eq!(checked.origin, name(index, "to"));
                calls.push(index);
            }
        }
        assert_eq!(calls.len(), 1, "one `to` end is a call: {calls:?}");
        call_to_positions.push(calls[0]);
    }
    assert_eq!(call_to_positions.len(), 2);
}

/// FR-533-AC-11: over TC-1070 fixture L the bare route selects the visible
/// entry and both case guards of `Decide` by (control node, member, index)
/// alone, each with the one origin the table of "Slot naming on the bytes
/// route" gives (the case guards named by label), and the `related` ends of
/// `Got` (leaf references) have none.
#[trace("FR-533-AC-11", "FR-533-AC-12", "TC-1333")]
#[test]
fn the_bare_route_names_fixture_l_decision_slots_by_node_member_and_index() {
    let package = fixture();
    let read = read_checked_package(&package);
    let v2 = read.admitted();
    let rows = bare_rows(&package, v2);
    let equality = qsl_package::emit_checked(&package, EmitLimits::fixture())
        .expect("emits")
        .equality_witness(
            v2,
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("fixture allowance materializes equality witness");
    let bare = qsl_package::BarePackage::new(v2)
        .with_supply(
            qsl_package::BareSupply {
                identities: &rows,
                equality: Some(&equality),
            },
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply");
    let origin = |owner: &str, member: &str, index: usize| {
        let wanted = slot(&package, "Pipeline", owner, member, Some(index));
        bare.select_protocol_slot(&qsl_package::ProtocolSlotDescriptor::new(
            wanted.node,
            member,
            Some(index),
        ))
        .unwrap_or_else(|failure| panic!("{owner}.{member}[{index}]: {failure:?}"))
        .origin()
        .map(str::to_owned)
    };
    let name = |tail: &str| format!("protocol Pipeline run Main Decide {tail}");
    assert_eq!(
        origin("Decide", "visible", 0),
        Some(name("decision 2641..2650"))
    );
    assert_eq!(
        origin("Decide", "cases.guard", 0),
        Some(name("case no guard"))
    );
    assert_eq!(
        origin("Decide", "cases.guard", 1),
        Some(name("case yes guard"))
    );
    assert_eq!(origin("Got", "related.from", 0), None);
    assert_eq!(origin("Got", "related.to", 0), None);
}
