// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-106 check 10 through public admission with typed operation facts.
//! The model and object environment come from the real intake/check path.
//! Operation signatures are supplied at the model admission boundary;
//! these tests establish admission, not checker or S6 execution support.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::observation::*;
use qsl_semantics::model::operation::OperationDeclaration;
use quire_exact::{CollectionKind, CollectionType, Integer, IntegerInterval, Value, ValueType};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::json;

use crate::model_operations::{
    admit_and_assemble_with_body, config_version_document, operation, operation_parameter,
};

const POPULATION: &str = "ix://example/config-version/config_history";

fn sequence(element: ValueType) -> ValueType {
    ValueType::collection(CollectionType::new(CollectionKind::Sequence, element, None))
}

fn reference(key: &str) -> SnapshotValue {
    SnapshotValue::reference(SelectedObject {
        population: POPULATION.to_owned(),
        key: key.to_owned(),
    })
}

fn label(name: &str) -> DocumentRef {
    DocumentRef {
        authority: "test".to_owned(),
        identity: name.to_owned(),
        revision_namespace: "fixture".to_owned(),
        revision: "fixture:1".to_owned(),
        digest: [0; 32],
    }
}

fn identity(reference: &DocumentRef) -> serde_json::Value {
    json!({
        "authority": reference.authority, "identity": reference.identity,
        "revision_namespace": reference.revision_namespace, "revision": reference.revision,
    })
}

fn document_ref(reference: &DocumentRef) -> serde_json::Value {
    json!({"identity": identity(reference),
        "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(&reference.digest))})
}

fn provision(reference: &DocumentRef, value: serde_json::Value) -> (DocumentRef, Vec<u8>) {
    let bytes = value.to_string().into_bytes();
    let document = quire_canonical::read(&bytes, u64::MAX).expect("JSON fixture");
    let digest = *quire_canonical::sha256(&document, quire_canonical::Limits::new(u64::MAX))
        .expect("canonical fixture")
        .as_bytes();
    (
        DocumentRef {
            digest,
            ..reference.clone()
        },
        bytes,
    )
}

struct Case {
    parameters: Vec<(String, ValueType)>,
    supplied: BTreeMap<String, SnapshotValue>,
    wire_parameters: serde_json::Value,
    result_type: Option<ValueType>,
    result: serde_json::Value,
    pre_call: bool,
    pre_complete: bool,
    limits: ObservationLimits,
    parent: serde_json::Value,
    source_signature: bool,
}

impl Case {
    fn new(value_type: ValueType, raw: SnapshotValue, result: serde_json::Value) -> Self {
        Self {
            parameters: vec![("arg".to_owned(), value_type.clone())],
            supplied: BTreeMap::from([("arg".to_owned(), raw)]),
            wire_parameters: json!({"arg": result.clone()}),
            result_type: Some(value_type),
            result,
            pre_call: false,
            pre_complete: true,
            limits: ObservationLimits::default(),
            parent: json!({"absent": {}}),
            source_signature: false,
        }
    }

    fn admit(self) -> Result<AdmittedObservations, AdmissionFailure> {
        let mut model_operation = operation(
            "probe",
            json!([]),
            None,
            json!({
                "modifies": [], "creates": ["ix://example/config-version/ConfigVersion"],
                "deletes": ["ix://example/config-version/ConfigVersion"],
            }),
        );
        if self.source_signature {
            let multiplicity = json!({"lower": 0, "upper": 4, "ordered": true, "unique": false});
            let mut parameter = operation_parameter("probe", "arg", "ix://quire/native/Integer");
            parameter["multiplicity"] = multiplicity.clone();
            model_operation["params"] = json!([parameter]);
            model_operation["returns"] = json!({"typeRef": "ix://quire/native/Integer", "multiplicity": multiplicity, "nullable": false});
        }
        let document = config_version_document(model_operation, vec![], vec![], json!([]));
        let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
        let model_digest = *packages.keys().next().expect("one model");
        let body = if self.pre_call {
            "pre Holds using v on Config::ConfigVersion::probe { true }"
        } else {
            "post Holds using v on Config::ConfigVersion::probe { true }"
        };
        let graph = admit_and_assemble_with_body(&document, body)
            .expect("assemble model")
            .check(CheckingLimits::default())
            .expect("check model");
        let checked = graph.state_clause("Holds").expect("checked clause");
        let base = checked.operation().expect("operation");
        let declaration = if self.source_signature {
            assert_eq!(base.declaration.parameters(), self.parameters);
            assert_eq!(base.declaration.result(), self.result_type.as_ref());
            base.declaration.clone()
        } else {
            OperationDeclaration::new(
                "probe",
                self.parameters,
                self.result_type,
                base.declaration.effect().clone(),
            )
        };
        let facts = ClauseFacts {
            identity: checked.identity(),
            kind: checked.kind(),
            context: checked.context(),
            operation: Some(OperationFacts {
                declaring: base.declaring,
                declaration,
            }),
            operations: Default::default(),
        };
        let model = json!({"identity": "example/config-version", "version": "1.0.0",
            "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(&model_digest))});
        let object = |key: &str| {
            json!({"key": key, "type": "ix://example/config-version/ConfigVersion",
            "fields": {"versionNumber": {"integer": "1"}, "parent": self.parent}})
        };
        let pre = label("pre");
        let (pre, pre_bytes) = provision(
            &pre,
            json!({
                "format": "quire.state.snapshot/v1", "identity": identity(&pre),
                "observation": "pre", "model": model,
                "populations": [{"population": POPULATION, "complete": self.pre_complete,
                    "objects": [object("self"), object("before")]}],
            }),
        );
        let post = label("post");
        let (post, post_bytes) = provision(
            &post,
            json!({
                "format": "quire.state.snapshot/v1", "identity": identity(&post),
                "observation": "post", "model": model,
                "populations": [{"population": POPULATION, "complete": true,
                    "objects": [object("self"), object("after")]}],
            }),
        );
        let self_object = SelectedObject {
            population: POPULATION.to_owned(),
            key: "self".to_owned(),
        };
        let snapshots = BTreeMap::from([(pre.digest, pre_bytes), (post.digest, post_bytes)]);
        let invocation = label("call");
        let (invocation, invocation_bytes) = provision(
            &invocation,
            json!({
                "format": "quire.state.invocation/v1", "identity": identity(&invocation),
                "model": model, "context": "ix://example/config-version/ConfigVersion", "operation": "probe",
                "self": {"population": POPULATION, "key": "self"}, "pre": document_ref(&pre),
                "post": document_ref(&post), "parameters": self.wire_parameters, "result": self.result,
                "created": [{"population": POPULATION, "key": "after"}],
                "deleted": [{"population": POPULATION, "key": "before"}],
            }),
        );
        let invocations = BTreeMap::from([(invocation.digest, invocation_bytes)]);
        let input = if self.pre_call {
            ClauseSelectionInput::PreCall {
                snapshot: pre,
                self_object: Some(self_object),
                parameters: self
                    .supplied
                    .into_iter()
                    .map(|(name, value)| {
                        (
                            quire_exact::Identifier::new(name).expect("parameter identifier"),
                            value,
                        )
                    })
                    .collect(),
            }
        } else {
            ClauseSelectionInput::Invocation { invocation }
        };
        admit_observations(
            graph.model_selections(),
            graph.scope().types(),
            &facts,
            &packages,
            ModelNormalizationLimits::default(),
            &Provisions {
                snapshots: &snapshots,
                invocations: &invocations,
            },
            &ClauseSelection {
                name: "Holds".to_owned(),
                input,
            },
            self.limits,
        )
    }
}

fn assert_integer(value: &Value, expected: i64) {
    let Value::Integer(value) = value else {
        panic!("expected integer: {value:?}")
    };
    assert_eq!(value, &Integer::from(expected));
}

fn assert_refused(case: Case, code: Code, cause: &str, field: Option<&str>) -> AdmissionRecord {
    let AdmissionFailure::Refused(record) = case.admit().expect_err("refused") else {
        panic!("expected refusal")
    };
    assert_eq!(record.code, code);
    assert_eq!(record.cause, cause);
    assert_eq!(record.fields.get("field").map(String::as_str), field);
    record
}

/// Trace: FR-106-AC-2, FR-106-AC-8
#[test]
fn options_admit_present_and_absent_parameters_and_results() {
    for pre_call in [false, true] {
        for present in [false, true] {
            let raw = if present {
                SnapshotValue::present(SnapshotValue::integer("7"))
            } else {
                SnapshotValue::absent()
            };
            let wire = if present {
                json!({"present": {"integer": "7"}})
            } else {
                json!({"absent": {}})
            };
            let mut case = Case::new(ValueType::option(ValueType::Integer), raw, wire);
            case.pre_call = pre_call;
            let admitted = case.admit().expect("Option typed values admit");
            assert_eq!(admitted.parameters[0].0, "arg");
            let Value::Option(value) = &admitted.parameters[0].1 else {
                panic!("expected Option")
            };
            assert_eq!(value.payload_type(), &ValueType::Integer);
            assert_eq!(value.payload().is_some(), present);
            if present {
                assert_integer(value.payload().expect("present"), 7);
            }
            if pre_call {
                assert!(admitted.result.is_none());
            } else {
                let Value::Option(result) = admitted.result.expect("declared result") else {
                    panic!("expected Option")
                };
                assert_eq!(result.payload().is_some(), present);
                if present {
                    assert_integer(result.payload().expect("present"), 7);
                }
            }
        }
    }
}

/// Trace: FR-106-AC-2, FR-106-AC-8
#[test]
fn sequences_retain_empty_values_duplicates_and_occurrence_order() {
    for pre_call in [false, true] {
        for values in [vec![], vec![7, 2, 7]] {
            let raw = SnapshotValue::sequence(
                values
                    .iter()
                    .map(|v| SnapshotValue::integer(v.to_string()))
                    .collect(),
            );
            let wire = json!({"sequence": values.iter().map(|v| json!({"integer": v.to_string()})).collect::<Vec<_>>()});
            let mut case = Case::new(sequence(ValueType::Integer), raw, wire);
            case.pre_call = pre_call;
            let admitted = case.admit().expect("Sequence typed values admit");
            let Value::Collection(value) = &admitted.parameters[0].1 else {
                panic!("expected sequence")
            };
            assert_eq!(value.elements().len(), values.len());
            for (actual, expected) in value.elements().iter().zip(&values) {
                assert_integer(actual, *expected);
            }
            if !pre_call {
                let Value::Collection(result) = admitted.result.expect("result") else {
                    panic!("expected sequence")
                };
                assert_eq!(result.elements().len(), values.len());
                for (actual, expected) in result.elements().iter().zip(&values) {
                    assert_integer(actual, *expected);
                }
            }
        }
    }
}

/// Trace: FR-106-AC-2, FR-106-AC-8
#[test]
fn nested_option_sequence_values_keep_their_declared_types_and_payloads() {
    let declared = ValueType::option(sequence(ValueType::option(ValueType::Boolean)));
    let raw = SnapshotValue::present(SnapshotValue::sequence(vec![
        SnapshotValue::absent(),
        SnapshotValue::present(SnapshotValue::boolean(false)),
    ]));
    let case = Case::new(
        declared,
        raw,
        json!({"present": {"sequence": [
            {"absent": {}}, {"present": {"boolean": false}},
        ]}}),
    );
    let admitted = case.admit().expect("nested owned wire forms admit");
    for value in [
        &admitted.parameters[0].1,
        admitted.result.as_ref().expect("result"),
    ] {
        let Value::Option(outer) = value else {
            panic!("outer Option")
        };
        assert_eq!(
            outer.payload_type(),
            &sequence(ValueType::option(ValueType::Boolean))
        );
        let Value::Collection(items) = outer.payload().expect("present") else {
            panic!("Sequence")
        };
        assert_eq!(
            items.collection_type(),
            &CollectionType::new(
                CollectionKind::Sequence,
                ValueType::option(ValueType::Boolean),
                None
            )
        );
        assert_eq!(items.elements().len(), 2);
        let Value::Option(first) = &items.elements()[0] else {
            panic!("first Option")
        };
        assert!(first.payload().is_none());
        let Value::Option(second) = &items.elements()[1] else {
            panic!("second Option")
        };
        assert!(matches!(second.payload(), Some(Value::Boolean(false))));
    }
}

/// Trace: FR-106-AC-3, FR-106-AC-7, FR-106-AC-12
#[test]
fn malformed_nested_values_refuse_at_the_first_declared_parameter() {
    let bounded = ValueType::Int(
        IntegerInterval::new(Integer::from(0_i64), Integer::from(9_i64)).expect("range"),
    );
    for (raw, wire, cause) in [
        (
            SnapshotValue::boolean(true),
            json!({"boolean": true}),
            "wrong-value-kind",
        ),
        (
            SnapshotValue::integer("01"),
            json!({"integer": "01"}),
            "invalid-value",
        ),
        (
            SnapshotValue::integer("10"),
            json!({"integer": "10"}),
            "invalid-value",
        ),
    ] {
        for pre_call in [false, true] {
            for witness in [false, true] {
                let mut case = Case::new(
                    sequence(ValueType::option(bounded.clone())),
                    SnapshotValue::sequence(vec![SnapshotValue::present(raw.clone())]),
                    json!({"sequence": [{"present": wire}]}),
                );
                case.pre_call = pre_call;
                if witness {
                    case.limits.post_state = PostStateRange::Witness;
                }
                case.parameters
                    .push(("later".to_owned(), ValueType::Boolean));
                // Missing later parameter and unknown member must not overtake arg.
                case.supplied
                    .insert("extra".to_owned(), SnapshotValue::boolean(true));
                assert_refused(case, Code::InvalidRuntimeInput, cause, Some("arg"));
            }
        }
    }
}

/// Trace: FR-106-AC-3, FR-106-AC-12
#[test]
fn malformed_results_refuse_after_valid_composite_parameters() {
    for (wire, cause) in [
        (json!({"present": {"boolean": true}}), "wrong-value-kind"),
        (json!({"present": {"integer": "-0"}}), "invalid-value"),
        (json!({"present": {"integer": "10"}}), "invalid-value"),
    ] {
        let mut case = Case::new(
            ValueType::option(ValueType::Integer),
            SnapshotValue::absent(),
            json!({"absent": {}}),
        );
        case.result_type = Some(ValueType::option(ValueType::Int(
            IntegerInterval::new(Integer::from(0_i64), Integer::from(9_i64)).expect("range"),
        )));
        case.result = wire;
        case.limits.post_state = PostStateRange::Witness;
        assert_refused(case, Code::InvalidRuntimeInput, cause, Some("result"));
    }
}

/// Trace: FR-106-AC-3, FR-106-AC-8
#[test]
fn missing_unknown_and_null_remain_distinct_from_absent_values() {
    let make = || {
        Case::new(
            ValueType::option(ValueType::Boolean),
            SnapshotValue::absent(),
            json!({"absent": {}}),
        )
    };
    let mut missing = make();
    missing.pre_call = true;
    missing.supplied.clear();
    assert_refused(
        missing,
        Code::InvalidRuntimeInput,
        "missing-member",
        Some("arg"),
    );
    let mut unknown = make();
    unknown.pre_call = true;
    unknown
        .supplied
        .insert("extra".to_owned(), SnapshotValue::boolean(true));
    assert_refused(
        unknown,
        Code::InvalidRuntimeInput,
        "unknown-member",
        Some("extra"),
    );
    let mut null = make();
    null.result = serde_json::Value::Null;
    assert_refused(null, Code::InvalidRuntimeInput, "missing-member", None);
    let mut unexpected = make();
    unexpected.result_type = None;
    assert_refused(
        unexpected,
        Code::InvalidRuntimeInput,
        "unknown-member",
        None,
    );
    let mut no_result = make();
    no_result.result_type = None;
    no_result.result = serde_json::Value::Null;
    assert!(no_result
        .admit()
        .expect("undeclared null result admits")
        .result
        .is_none());
}

/// Trace: FR-106-AC-2, FR-106-AC-4, FR-106-AC-8
#[test]
fn nested_references_resolve_parameters_in_pre_and_results_in_post() {
    // Use the same admitted model as Case to obtain its actual context type.
    let document = config_version_document(
        operation(
            "probe",
            json!([]),
            None,
            json!({"modifies": [], "creates": ["ix://example/config-version/ConfigVersion"], "deletes": ["ix://example/config-version/ConfigVersion"]}),
        ),
        vec![],
        vec![],
        json!([]),
    );
    let graph = admit_and_assemble_with_body(
        &document,
        "post Holds using v on Config::ConfigVersion::probe { true }",
    )
    .expect("assemble")
    .check(CheckingLimits::default())
    .expect("check");
    let context = graph.state_clause("Holds").expect("clause").context();
    let ty = ValueType::option(sequence(ValueType::Reference(context)));
    let make = |key: &str| {
        let mut case = Case::new(
            ty.clone(),
            SnapshotValue::present(SnapshotValue::sequence(vec![reference(key)])),
            json!({"present": {"sequence": [{"reference": {"population": POPULATION, "key": key}}]}}),
        );
        case.result = json!({"present": {"sequence": [{"reference": {"population": POPULATION, "key": "after"}}]}});
        case
    };
    let admitted = make("before")
        .admit()
        .expect("pre parameter and created post result admit");
    for (value, key) in [
        (&admitted.parameters[0].1, "before"),
        (admitted.result.as_ref().expect("result"), "after"),
    ] {
        let Value::Option(outer) = value else {
            panic!("Option")
        };
        let Value::Collection(sequence) = outer.payload().expect("present") else {
            panic!("Sequence")
        };
        let Value::Reference(reference) = &sequence.elements()[0] else {
            panic!("Reference")
        };
        assert_eq!(reference.object().as_str(), key);
        assert_eq!(reference.object_type(), context);
    }
    let record = assert_refused(
        make("after"),
        Code::DanglingReference,
        "absent-target-in-complete-population",
        None,
    );
    assert_eq!(
        record.fields.get("population").map(String::as_str),
        Some(POPULATION)
    );
    assert_eq!(
        record.fields.get("object").map(String::as_str),
        Some("after")
    );
    let mut wrong_post = make("before");
    wrong_post.result = json!({"present": {"sequence": [{"reference": {"population": POPULATION, "key": "before"}}]}});
    let record = assert_refused(
        wrong_post,
        Code::DanglingReference,
        "absent-target-in-complete-population",
        None,
    );
    assert_eq!(
        record.fields.get("object").map(String::as_str),
        Some("before")
    );
    let mut incomplete = make("ghost");
    incomplete.pre_call = true;
    incomplete.pre_complete = false;
    let AdmissionFailure::Incomplete(record) = incomplete
        .admit()
        .expect_err("required population incomplete")
    else {
        panic!("Incomplete")
    };
    assert_eq!(record.code, Code::IncompletePopulation);
    assert_eq!(record.cause, "incomplete-scope");
    assert_eq!(
        record.fields.get("population").map(String::as_str),
        Some(POPULATION)
    );
}

/// Trace: FR-106-AC-1, FR-106-AC-3
#[test]
fn optional_object_field_presence_keeps_its_own_rule() {
    let make = || {
        Case::new(
            ValueType::option(ValueType::Boolean),
            SnapshotValue::absent(),
            json!({"absent": {}}),
        )
    };
    let mut present = make();
    present.parent = json!({"present": {"reference": {"population": POPULATION, "key": "self"}}});
    present
        .admit()
        .expect("optional reference field admits present");
    let mut bare = make();
    bare.parent = json!({"reference": {"population": POPULATION, "key": "self"}});
    assert_refused(
        bare,
        Code::InvalidRuntimeInput,
        "wrong-value-kind",
        Some("parent"),
    );
}

/// Trace: FR-106-AC-8
#[test]
fn deeply_nested_owned_forms_admit_without_recursive_value_walks() {
    let mut declared = ValueType::Boolean;
    let mut raw = SnapshotValue::boolean(true);
    for _ in 0..10_000 {
        declared = ValueType::option(declared);
        raw = SnapshotValue::present(raw);
    }
    let mut case = Case::new(declared, raw, serde_json::Value::Null);
    case.pre_call = true;
    let admitted = case.admit().expect("deep typed operation argument admits");
    let mut value = &admitted.parameters[0].1;
    let mut depth = 0;
    while let Value::Option(option) = value {
        value = option.payload().expect("every wrapper is present");
        depth += 1;
    }
    assert_eq!(depth, 10_000);
    assert!(matches!(value, Value::Boolean(true)));
}

/// Trace: FR-106-AC-3
#[test]
fn composite_wire_values_keep_the_existing_value_limit() {
    let mut case = Case::new(
        sequence(ValueType::Boolean),
        SnapshotValue::sequence(vec![SnapshotValue::boolean(true); 4]),
        json!({"sequence": [{"boolean": true}, {"boolean": true}, {"boolean": true}, {"boolean": true}]}),
    );
    case.limits = case.limits.with_values_per_document(4);
    let AdmissionFailure::Refused(record) = case.admit().expect_err("wire value limit") else {
        panic!("Refused")
    };
    assert_eq!(record.code, Code::StageLimitExceeded);
    assert_eq!(record.cause, "node-count-exceeded");
    assert_eq!(
        record.fields.get("setting").map(String::as_str),
        Some("observation.values")
    );
}

/// Trace: FR-106-AC-7, FR-106-AC-8
#[test]
fn declared_parameter_order_controls_admission_and_first_refusal() {
    let mut case = Case::new(
        ValueType::option(ValueType::Boolean),
        SnapshotValue::absent(),
        json!({"absent": {}}),
    );
    case.pre_call = true;
    case.parameters = vec![
        ("z".to_owned(), ValueType::option(ValueType::Boolean)),
        ("a".to_owned(), sequence(ValueType::Integer)),
    ];
    case.supplied = BTreeMap::from([
        (
            "a".to_owned(),
            SnapshotValue::sequence(vec![SnapshotValue::integer("3")]),
        ),
        (
            "z".to_owned(),
            SnapshotValue::present(SnapshotValue::boolean(false)),
        ),
    ]);
    let admitted = case.admit().expect("typed parameters");
    assert_eq!(
        admitted
            .parameters
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["z", "a"]
    );
    let Value::Option(z) = &admitted.parameters[0].1 else {
        panic!("Option")
    };
    assert!(matches!(z.payload(), Some(Value::Boolean(false))));
    let Value::Collection(a) = &admitted.parameters[1].1 else {
        panic!("Sequence")
    };
    assert_integer(&a.elements()[0], 3);

    let mut case = Case::new(
        ValueType::option(sequence(ValueType::Integer)),
        SnapshotValue::present(SnapshotValue::sequence(vec![
            SnapshotValue::integer("01"),
            SnapshotValue::boolean(true),
        ])),
        json!({"present": {"sequence": [{"integer": "01"}, {"boolean": true}]}}),
    );
    case.pre_call = true;
    assert_refused(
        case,
        Code::InvalidRuntimeInput,
        "invalid-value",
        Some("arg"),
    );
}

/// Trace: FR-106-AC-2, FR-106-AC-8
#[test]
fn nested_sequences_preserve_sibling_boundaries() {
    let raw = SnapshotValue::sequence(vec![
        SnapshotValue::sequence(vec![
            SnapshotValue::integer("4"),
            SnapshotValue::integer("1"),
        ]),
        SnapshotValue::sequence(vec![]),
        SnapshotValue::sequence(vec![SnapshotValue::integer("8")]),
    ]);
    let admitted = Case::new(sequence(sequence(ValueType::Integer)), raw,
        json!({"sequence": [{"sequence": [{"integer": "4"}, {"integer": "1"}]}, {"sequence": []}, {"sequence": [{"integer": "8"}]}]}))
        .admit().expect("nested sequences");
    for value in [
        &admitted.parameters[0].1,
        admitted.result.as_ref().expect("result"),
    ] {
        let Value::Collection(outer) = value else {
            panic!("outer sequence")
        };
        assert_eq!(outer.elements().len(), 3);
        for (value, expected) in outer.elements().iter().zip([vec![4, 1], vec![], vec![8]]) {
            let Value::Collection(inner) = value else {
                panic!("inner sequence")
            };
            assert_eq!(inner.elements().len(), expected.len());
            for (actual, expected) in inner.elements().iter().zip(expected) {
                assert_integer(actual, expected);
            }
        }
    }
}

/// Trace: FR-106-AC-3, FR-106-AC-8
#[test]
fn declared_sequence_bounds_are_checked_before_retaining_values() {
    let declared = ValueType::collection(CollectionType::new(
        CollectionKind::Sequence,
        ValueType::Boolean,
        Some(quire_exact::CardinalityBound::new(1, 2).expect("bound")),
    ));
    for count in [0, 1, 2, 3] {
        let raw = SnapshotValue::sequence(vec![SnapshotValue::boolean(false); count]);
        let wire = json!({"sequence": vec![json!({"boolean": false}); count]});
        let mut parameter = Case::new(declared.clone(), raw, wire.clone());
        parameter.pre_call = true;
        let mut result = Case::new(
            ValueType::option(ValueType::Boolean),
            SnapshotValue::absent(),
            json!({"absent": {}}),
        );
        result.result_type = Some(declared.clone());
        result.result = wire;
        if [1, 2].contains(&count) {
            let admitted = parameter.admit().expect("within bound");
            let Value::Collection(value) = &admitted.parameters[0].1 else {
                panic!("sequence")
            };
            assert_eq!(value.elements().len(), count);
            assert_eq!(
                value.collection_type().bound(),
                Some(quire_exact::CardinalityBound::new(1, 2).expect("bound"))
            );
            let admitted = result.admit().expect("result within bound");
            let Value::Collection(value) = admitted.result.expect("result") else {
                panic!("sequence")
            };
            assert_eq!(value.elements().len(), count);
        } else {
            let cause = if count < 1 {
                "below-minimum"
            } else {
                "above-maximum"
            };
            assert_refused(parameter, Code::CardinalityOutOfBound, cause, Some("arg"));
            assert_refused(result, Code::CardinalityOutOfBound, cause, Some("result"));
        }
    }
}

/// Trace: FR-106-AC-2
#[test]
fn model_owned_sequence_signature_admits_through_real_intake_and_checking() {
    let mut case = Case::new(
        ValueType::collection(CollectionType::new(
            CollectionKind::Sequence,
            ValueType::Integer,
            Some(quire_exact::CardinalityBound::at_most(4)),
        )),
        SnapshotValue::sequence(vec![
            SnapshotValue::integer("7"),
            SnapshotValue::integer("2"),
        ]),
        json!({"sequence": [{"integer": "7"}, {"integer": "2"}]}),
    );
    case.source_signature = true;
    let admitted = case
        .admit()
        .expect("the actual model-owned sequence signature admits");
    for value in [
        &admitted.parameters[0].1,
        admitted.result.as_ref().expect("result"),
    ] {
        let Value::Collection(value) = value else {
            panic!("sequence")
        };
        assert_eq!(value.elements().len(), 2);
        assert_integer(&value.elements()[0], 7);
        assert_integer(&value.elements()[1], 2);
    }
}

// ---------------------------------------------------------------------------
// FR-106 "Set, bag and ordered-set values" (TC-1055)
// ---------------------------------------------------------------------------

fn int9() -> ValueType {
    ValueType::Int(IntegerInterval::new(Integer::from(0_i64), Integer::from(9_i64)).expect("range"))
}

fn collection_of(kind: CollectionKind, element: ValueType, bound: Option<(u64, u64)>) -> ValueType {
    ValueType::collection(CollectionType::new(
        kind,
        element,
        bound.map(|(minimum, maximum)| {
            quire_exact::CardinalityBound::new(minimum, maximum).expect("non-empty bound")
        }),
    ))
}

/// An integer element, as a raw value and as its wire object.
fn int(spelling: &str) -> (SnapshotValue, serde_json::Value) {
    (
        SnapshotValue::integer(spelling),
        json!({"integer": spelling}),
    )
}

fn boolean_true() -> (SnapshotValue, serde_json::Value) {
    (SnapshotValue::boolean(true), json!({"boolean": true}))
}

fn integers(spellings: &[&str]) -> Vec<(SnapshotValue, serde_json::Value)> {
    spellings.iter().map(|spelling| int(spelling)).collect()
}

/// A probe case whose parameter `arg` and result are `items` under the tag of
/// `kind`, declared as `declared`.
fn collection_case(
    declared: ValueType,
    kind: CollectionKind,
    items: Vec<(SnapshotValue, serde_json::Value)>,
) -> Case {
    let (raws, wires): (Vec<_>, Vec<_>) = items.into_iter().unzip();
    let mut wire = serde_json::Map::new();
    wire.insert(
        collection_tag(kind).to_owned(),
        serde_json::Value::Array(wires),
    );
    Case::new(
        declared,
        SnapshotValue::collection(kind, raws),
        serde_json::Value::Object(wire),
    )
}

fn element_spellings(value: &Value) -> Vec<String> {
    let Value::Collection(collection) = value else {
        panic!("expected a collection: {value:?}")
    };
    collection
        .elements()
        .iter()
        .map(|element| {
            let Value::Integer(integer) = element else {
                panic!("expected an integer element: {element:?}")
            };
            integer.to_string()
        })
        .collect()
}

/// Admission of `case` refused, as a parameter through `PreCall` and as the
/// parameter of an invocation; both refusals are returned.
fn refused_both_ways(make: impl Fn() -> Case, code: Code, cause: &str) -> Vec<AdmissionRecord> {
    [true, false]
        .into_iter()
        .map(|pre_call| {
            let mut case = make();
            case.pre_call = pre_call;
            assert_refused(case, code, cause, Some("arg"))
        })
        .collect()
}

/// FR-106-AC-50: each kind admits as a parameter through `PreCall` and as a
/// parameter and a result of an invocation, a set and a bag in key order
/// whatever the array order, an ordered set and a sequence in array order.
#[trace("TC-1055", "FR-106-AC-50")]
#[test]
fn collection_kinds_admit_with_their_element_order() {
    for (kind, supplied, expected) in [
        (CollectionKind::Set, vec!["2", "1"], vec!["1", "2"]),
        (CollectionKind::Set, vec!["1", "2"], vec!["1", "2"]),
        (
            CollectionKind::Bag,
            vec!["1", "2", "1"],
            vec!["1", "1", "2"],
        ),
        (
            CollectionKind::Bag,
            vec!["1", "1", "2"],
            vec!["1", "1", "2"],
        ),
        (CollectionKind::OrderedSet, vec!["2", "1"], vec!["2", "1"]),
        (CollectionKind::OrderedSet, vec!["1", "2"], vec!["1", "2"]),
        (
            CollectionKind::Sequence,
            vec!["1", "2", "1"],
            vec!["1", "2", "1"],
        ),
    ] {
        let declared = collection_of(kind, int9(), None);
        for pre_call in [true, false] {
            let mut case = collection_case(declared.clone(), kind, integers(&supplied));
            case.pre_call = pre_call;
            let admitted = case.admit().expect("the collection admits");
            assert_eq!(
                element_spellings(&admitted.parameters[0].1),
                expected,
                "{kind:?} {supplied:?} parameter"
            );
            if !pre_call {
                let result = admitted.result.expect("the invocation's result");
                assert_eq!(
                    element_spellings(&result),
                    expected,
                    "{kind:?} {supplied:?} result"
                );
            }
        }
    }
}

/// FR-106-AC-51: the raw comparison the frame check applies to a field.
#[trace("TC-1055", "FR-106-AC-51")]
#[test]
fn raw_set_and_bag_compare_as_multisets_and_the_others_by_order() {
    let of = |kind: CollectionKind, spellings: &[&str]| {
        SnapshotValue::collection(
            kind,
            integers(spellings)
                .into_iter()
                .map(|(raw, _)| raw)
                .collect(),
        )
    };
    assert_eq!(
        of(CollectionKind::Set, &["1", "2"]),
        of(CollectionKind::Set, &["2", "1"])
    );
    assert_eq!(
        of(CollectionKind::Bag, &["1", "1", "2"]),
        of(CollectionKind::Bag, &["1", "2", "1"])
    );
    assert_ne!(
        of(CollectionKind::Bag, &["1", "1", "2"]),
        of(CollectionKind::Bag, &["1", "2", "2"])
    );
    assert_ne!(
        of(CollectionKind::OrderedSet, &["1", "2"]),
        of(CollectionKind::OrderedSet, &["2", "1"])
    );
    assert_ne!(
        of(CollectionKind::Sequence, &["1", "2"]),
        of(CollectionKind::Sequence, &["2", "1"])
    );
    assert_ne!(
        of(CollectionKind::Set, &["1", "2"]),
        of(CollectionKind::OrderedSet, &["1", "2"])
    );
}

/// FR-106-AC-52: a duplicate in a set or ordered set refuses at the later
/// element naming the earlier one, a bag and a sequence admit it, and every
/// element is read before a duplicate is decided.
#[trace("TC-1055", "FR-106-AC-52")]
#[test]
fn duplicates_refuse_in_sets_and_ordered_sets_only() {
    for (kind, supplied) in [
        (CollectionKind::Set, vec!["1", "2", "1"]),
        (CollectionKind::OrderedSet, vec!["2", "1", "2"]),
    ] {
        let declared = collection_of(kind, int9(), None);
        let records = refused_both_ways(
            || collection_case(declared.clone(), kind, integers(&supplied)),
            Code::InvalidRuntimeInput,
            "duplicate-member",
        );
        for record in records {
            assert_eq!(record.path, vec![PathStep::Index(2)], "{kind:?}");
            assert_eq!(
                record.fields.get("earlier-index").map(String::as_str),
                Some("0"),
                "{kind:?}"
            );
        }
    }
    for kind in [CollectionKind::Bag, CollectionKind::Sequence] {
        let declared = collection_of(kind, int9(), None);
        for pre_call in [true, false] {
            let mut case = collection_case(declared.clone(), kind, integers(&["1", "2", "1"]));
            case.pre_call = pre_call;
            let admitted = case.admit().expect("equal elements admit");
            assert_eq!(
                element_spellings(&admitted.parameters[0].1).len(),
                3,
                "{kind:?}"
            );
        }
    }
    let declared = collection_of(CollectionKind::Set, int9(), Some((0, 3)));
    let mut items = integers(&["1", "1"]);
    items.push((SnapshotValue::integer("x"), json!({"integer": "x"})));
    let records = refused_both_ways(
        || collection_case(declared.clone(), CollectionKind::Set, items.clone()),
        Code::InvalidRuntimeInput,
        "invalid-value",
    );
    for record in records {
        assert_eq!(record.path, vec![PathStep::Index(2)]);
    }
}

/// FR-106-AC-53: the bound of every kind is checked before any element.
#[trace("TC-1055", "FR-106-AC-53")]
#[test]
fn bounds_refuse_before_any_element_is_read() {
    for (kind, supplied) in [
        (CollectionKind::Set, vec!["1", "2", "3"]),
        (CollectionKind::Bag, vec!["1", "1", "1"]),
        (CollectionKind::OrderedSet, vec!["1", "2", "3"]),
        (CollectionKind::Sequence, vec!["1", "2", "3"]),
    ] {
        let declared = collection_of(kind, int9(), Some((0, 2)));
        for record in refused_both_ways(
            || collection_case(declared.clone(), kind, integers(&supplied)),
            Code::CardinalityOutOfBound,
            "above-maximum",
        ) {
            assert_eq!(record.fields.get("count").map(String::as_str), Some("3"));
            assert!(record.path.is_empty(), "{kind:?}");
        }
    }
    let below = collection_of(CollectionKind::Sequence, int9(), Some((1, 3)));
    for record in refused_both_ways(
        || collection_case(below.clone(), CollectionKind::Sequence, vec![]),
        Code::CardinalityOutOfBound,
        "below-minimum",
    ) {
        assert_eq!(record.fields.get("count").map(String::as_str), Some("0"));
    }
    // The bound comes before the duplicate decision and before any element.
    let set = collection_of(CollectionKind::Set, int9(), Some((0, 2)));
    refused_both_ways(
        || collection_case(set.clone(), CollectionKind::Set, integers(&["1", "1", "1"])),
        Code::CardinalityOutOfBound,
        "above-maximum",
    );
    let mut with_boolean = integers(&["1", "2"]);
    with_boolean.push(boolean_true());
    refused_both_ways(
        || collection_case(set.clone(), CollectionKind::Set, with_boolean.clone()),
        Code::CardinalityOutOfBound,
        "above-maximum",
    );
    // At the bound the collection admits.
    for pre_call in [true, false] {
        let mut case = collection_case(set.clone(), CollectionKind::Set, integers(&["2", "1"]));
        case.pre_call = pre_call;
        let admitted = case.admit().expect("a set at its bound admits");
        assert_eq!(element_spellings(&admitted.parameters[0].1), ["1", "2"]);
    }
}

/// FR-106-AC-54: a form of another kind, a payload that is no array, a member
/// beside the tag and a counted pair are all `wrong-value-kind`.
#[trace("TC-1055", "FR-106-AC-54")]
#[test]
fn forms_of_another_kind_and_unreadable_forms_refuse_as_wrong_value_kind() {
    for (declared_kind, supplied_kind) in [
        (CollectionKind::Set, CollectionKind::Bag),
        (CollectionKind::Set, CollectionKind::Sequence),
        (CollectionKind::Sequence, CollectionKind::Set),
    ] {
        let declared = collection_of(declared_kind, int9(), None);
        refused_both_ways(
            || collection_case(declared.clone(), supplied_kind, integers(&["1"])),
            Code::InvalidRuntimeInput,
            "wrong-value-kind",
        );
    }
    let set = collection_of(CollectionKind::Set, int9(), None);
    let bag = collection_of(CollectionKind::Bag, int9(), None);
    for (declared, wire) in [
        (set.clone(), json!({"set": {}})),
        (set, json!({"set": [{"integer": "1"}], "count": "1"})),
        (
            bag,
            json!({"bag": [{"element": {"integer": "1"}, "count": "2"}]}),
        ),
    ] {
        let mut case = Case::new(declared, SnapshotValue::absent(), wire);
        case.pre_call = false;
        assert_refused(
            case,
            Code::InvalidRuntimeInput,
            "wrong-value-kind",
            Some("arg"),
        );
    }
}

/// FR-106-AC-55: a set of an element type with no value form is refused
/// before any element, and a set of UUIDs decides duplicates by UUID.
#[trace("TC-1055", "FR-106-AC-55")]
#[test]
fn element_types_without_a_value_form_refuse_and_native_elements_admit() {
    let decimal = quire_exact::DecimalType::new(
        Integer::from(0_i64),
        Integer::from(100_i64),
        0,
        2,
        quire_exact::RoundingMode::Exact,
    )
    .expect("decimal type");
    let declared = collection_of(CollectionKind::Set, ValueType::Decimal(decimal), None);
    for record in refused_both_ways(
        || collection_case(declared.clone(), CollectionKind::Set, vec![]),
        Code::UnknownRequiredFeature,
        "unsupported-feature",
    ) {
        assert_eq!(
            record.fields.get("feature").map(String::as_str),
            Some("Decimal")
        );
    }
    let uuid = |text: &str| {
        (
            SnapshotValue::native(
                RawNativeKind::Known(NativeKind::Uuid),
                vec![("text".to_owned(), Some(text.to_owned()))],
            ),
            json!({"kind": "uuid", "text": text}),
        )
    };
    let first = "00000000-0000-0000-0000-000000000001";
    let second = "00000000-0000-0000-0000-000000000002";
    let declared = collection_of(CollectionKind::Set, ValueType::Uuid, None);
    for record in refused_both_ways(
        || {
            collection_case(
                declared.clone(),
                CollectionKind::Set,
                vec![uuid(first), uuid(first)],
            )
        },
        Code::InvalidRuntimeInput,
        "duplicate-member",
    ) {
        assert_eq!(record.path, vec![PathStep::Index(1)]);
    }
    for pre_call in [true, false] {
        let mut case = collection_case(
            declared.clone(),
            CollectionKind::Set,
            vec![uuid(first), uuid(second)],
        );
        case.pre_call = pre_call;
        let admitted = case.admit().expect("two different UUIDs admit");
        let Value::Collection(set) = &admitted.parameters[0].1 else {
            panic!("expected a set")
        };
        assert_eq!(set.elements().len(), 2);
    }
}

/// FR-106-AC-56: the writer orders a set's and a bag's elements by their
/// encoded bytes, and an ordered set's and a sequence's in their order.
#[trace("TC-1055", "FR-106-AC-56")]
#[test]
fn the_writer_orders_sets_and_bags_by_encoded_bytes() {
    let of = |kind: CollectionKind, spellings: &[&str]| {
        SnapshotValue::collection(
            kind,
            integers(spellings)
                .into_iter()
                .map(|(raw, _)| raw)
                .collect(),
        )
        .to_json()
    };
    let written = |spellings: &[&str]| {
        json!(spellings
            .iter()
            .map(|spelling| json!({"integer": spelling}))
            .collect::<Vec<_>>())
    };
    assert_eq!(
        of(CollectionKind::Set, &["9", "10"]),
        json!({"set": written(&["10", "9"])})
    );
    assert_eq!(
        of(CollectionKind::Bag, &["9", "10", "9"]),
        json!({"bag": written(&["10", "9", "9"])})
    );
    assert_eq!(
        of(CollectionKind::OrderedSet, &["2", "1"]),
        json!({"ordered-set": written(&["2", "1"])})
    );
    assert_eq!(
        of(CollectionKind::Sequence, &["2", "1"]),
        json!({"sequence": written(&["2", "1"])})
    );
}

/// FR-106-AC-58: a reader decides the elements of a set and a bag by the
/// equality of their type, so two spellings of one `nfc` Text are one member.
#[trace("TC-1055", "FR-106-AC-58")]
#[test]
fn text_elements_are_one_member_when_their_retained_sequences_are_equal() {
    let text_type =
        quire_exact::TextType::new(0, 8, quire_exact::TextProfile::Nfc).expect("text type");
    let element = |payload: &str| {
        (
            SnapshotValue::native(
                RawNativeKind::Known(NativeKind::Text),
                vec![("text".to_owned(), Some(payload.to_owned()))],
            ),
            json!({"kind": "text", "text": payload}),
        )
    };
    let composed = "\u{e9}";
    let decomposed = "e\u{301}";
    let set = collection_of(
        CollectionKind::Set,
        ValueType::Text(text_type.clone()),
        None,
    );
    for record in refused_both_ways(
        || {
            collection_case(
                set.clone(),
                CollectionKind::Set,
                vec![element(composed), element(decomposed)],
            )
        },
        Code::InvalidRuntimeInput,
        "duplicate-member",
    ) {
        assert_eq!(record.path, vec![PathStep::Index(1)]);
        assert_eq!(
            record.fields.get("earlier-index").map(String::as_str),
            Some("0")
        );
    }
    let bag = collection_of(CollectionKind::Bag, ValueType::Text(text_type), None);
    for pre_call in [true, false] {
        let mut case = collection_case(
            bag.clone(),
            CollectionKind::Bag,
            vec![element(decomposed), element(composed)],
        );
        case.pre_call = pre_call;
        let admitted = case.admit().expect("a bag admits equal elements");
        let Value::Collection(held) = &admitted.parameters[0].1 else {
            panic!("expected a bag")
        };
        let retained: Vec<&str> = held
            .elements()
            .iter()
            .map(|element| {
                let Value::Text(text) = element else {
                    panic!("expected a Text element: {element:?}")
                };
                text.retained()
            })
            .collect();
        assert_eq!(retained, [composed, composed]);
    }
}
