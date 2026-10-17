// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120 "Frame and observation seam": `qsl_semantics::model::state`, the
//! layer-3 seam a model simulator calls. Initial-state admission
//! (`StateModel::admit_initial`), a state's observation
//! (`StateModel::observation`) and the public frame decision
//! (`population::decide_frame`) over a state's own objects.

use std::collections::{BTreeMap, BTreeSet};

use qsl_foundation::diagnostic::Code;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::domain_package::OperationEffect;
use qsl_semantics::model::intake::package_input;
use qsl_semantics::model::key::{hex, DeclarationKey};
use qsl_semantics::model::normalize::ModelRefusalCause;
use qsl_semantics::model::observation::{
    admit_current_snapshot, declared_type_universe_for, AdmissionFailure, AdmissionRecord,
    AnchorKind, DocumentRef, ObservationLimits, SelectedAnchor,
};
use qsl_semantics::model::population::{
    decide_frame, FrameCharge, FrameDecision, FrameObject, FrameStop,
};
use qsl_semantics::model::state::{ModelState, StateModel, StateObject};
use quire_exact::{FieldValue, Integer, Value};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::json;

use crate::model_operations::{
    admit_and_assemble_with_body, config_version_document, config_version_identity, operation,
    PACKAGE_IDENTITY,
};

const POPULATION: &str = "ix://example/config-version/config_history";

fn population_key() -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE_IDENTITY.to_owned(),
        node: POPULATION.to_owned(),
    }
}

fn config_version_key() -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE_IDENTITY.to_owned(),
        node: format!("ix://{PACKAGE_IDENTITY}/ConfigVersion"),
    }
}

fn field(name: &str) -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE_IDENTITY.to_owned(),
        node: config_version_identity(name),
    }
}

struct Fixture {
    model: StateModel,
    graph: qsl_semantics::check::CheckedGraph,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
    header: serde_json::Value,
}

fn fixture() -> Fixture {
    let document = config_version_document(
        operation(
            "bump",
            json!([]),
            None,
            json!({"modifies": [config_version_identity("versionNumber")], "creates": [], "deletes": []}),
        ),
        vec![],
        vec![],
        json!([]),
    );
    let packages = package_input([document.as_slice()]);
    let digest = *packages.keys().next().expect("one model");
    let graph =
        admit_and_assemble_with_body(&document, "function noop using v(): Boolean pure { true }")
            .expect("assemble model")
            .check(CheckingLimits::default())
            .expect("check model");
    let model = StateModel::new(
        graph.model_selections(),
        graph.scope().types(),
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("model views");
    let header = json!({"identity": PACKAGE_IDENTITY, "version": "1.0.0",
        "digest": format!("sha256-jcs:{}", hex(&digest))});
    Fixture {
        model,
        graph,
        packages,
        header,
    }
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

fn object(key: &str, version: &str, parent: serde_json::Value) -> serde_json::Value {
    json!({"key": key, "type": format!("ix://{PACKAGE_IDENTITY}/ConfigVersion"),
        "fields": {"versionNumber": {"integer": version}, "parent": parent}})
}

fn parent_of(key: &str) -> serde_json::Value {
    json!({"present": {"reference": {"population": POPULATION, "key": key}}})
}

/// One `config_history` snapshot with the given observation, anchor, completeness
/// and objects, provisioned by digest.
fn snapshot(
    fixture: &Fixture,
    observation: &str,
    anchor: serde_json::Value,
    complete: bool,
    objects: Vec<serde_json::Value>,
) -> (DocumentRef, BTreeMap<[u8; 32], Vec<u8>>) {
    let reference = label("start");
    let mut document = json!({
        "format": "quire.state.snapshot/v1",
        "identity": {"authority": reference.authority, "identity": reference.identity,
            "revision_namespace": reference.revision_namespace, "revision": reference.revision},
        "observation": observation, "model": fixture.header,
        "populations": [{"population": POPULATION, "complete": complete, "objects": objects}],
    });
    if !anchor.is_null() {
        document["anchor"] = anchor;
    }
    let bytes = document.to_string().into_bytes();
    let read = quire_canonical::read(&bytes, u64::MAX).expect("JSON fixture");
    let digest = *quire_canonical::sha256(&read, quire_canonical::Limits::new(u64::MAX))
        .expect("canonical fixture")
        .as_bytes();
    (
        DocumentRef {
            digest,
            ..reference
        },
        BTreeMap::from([(digest, bytes)]),
    )
}

fn initial_anchor() -> serde_json::Value {
    json!({"kind": "initialization", "name": "start"})
}

fn two_objects() -> Vec<serde_json::Value> {
    vec![
        object("a", "1", json!({"absent": {}})),
        object("b", "2", parent_of("a")),
    ]
}

fn admit(
    fixture: &Fixture,
    observation: &str,
    anchor: serde_json::Value,
    complete: bool,
    objects: Vec<serde_json::Value>,
) -> Result<(ModelState, SelectedAnchor), AdmissionFailure> {
    let (selected, provision) = snapshot(fixture, observation, anchor, complete, objects);
    fixture
        .model
        .admit_initial(&provision, &selected, ObservationLimits::default())
}

fn refused(failure: Option<AdmissionFailure>) -> AdmissionRecord {
    match failure {
        Some(AdmissionFailure::Refused(record)) => record,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn integer(value: i64) -> Value {
    Value::Integer(Integer::from(value))
}

fn assert_integer(value: &Value, expected: i64) {
    let Value::Integer(value) = value else {
        panic!("expected an integer: {value:?}")
    };
    assert_eq!(value, &Integer::from(expected));
}

/// Integer, reference or absent equality: the only field forms these fixtures hold.
fn same(a: Option<&Value>, b: Option<&Value>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(Value::Integer(a)), Some(Value::Integer(b))) => a == b,
        (Some(Value::Reference(a)), Some(Value::Reference(b))) => a == b,
        _ => false,
    }
}

/// Trace: FR-120-AC-4
#[test]
fn an_initial_snapshot_admits_into_typed_objects_and_its_initialization_anchor() {
    let fixture = fixture();
    let (state, anchor) = admit(&fixture, "current", initial_anchor(), true, two_objects())
        .expect("initial state admits");
    assert_eq!(
        anchor,
        SelectedAnchor {
            kind: AnchorKind::Initialization,
            name: "start".to_owned()
        }
    );
    assert_eq!(
        state.populations.keys().collect::<Vec<_>>(),
        vec![&population_key()]
    );
    let objects = &state.populations[&population_key()];
    assert_eq!(
        objects.keys().map(String::as_str).collect::<Vec<_>>(),
        ["a", "b"]
    );
    let a = &objects["a"];
    assert_eq!(a.type_identity, config_version_key());
    assert_eq!(
        a.fields.keys().collect::<Vec<_>>(),
        vec![&field("versionNumber")],
        "an absent optional field has no entry"
    );
    assert_integer(&a.fields[&field("versionNumber")], 1);
    let b = &objects["b"];
    assert_integer(&b.fields[&field("versionNumber")], 2);
    let Value::Reference(parent) = &b.fields[&field("parent")] else {
        panic!("parent holds a reference")
    };
    assert_eq!(parent.object().as_str(), "a");
}

/// Trace: FR-120-AC-7
#[test]
fn an_initial_snapshot_that_is_not_a_current_initialization_refuses() {
    let fixture = fixture();
    let record = refused(admit(&fixture, "pre", initial_anchor(), true, two_objects()).err());
    assert_eq!(
        (record.code, record.cause),
        (Code::WrongSnapshot, "wrong-observation")
    );

    let record = refused(
        admit(
            &fixture,
            "current",
            json!({"kind": "handler", "name": "validate"}),
            true,
            two_objects(),
        )
        .err(),
    );
    assert_eq!(
        (record.code, record.cause),
        (Code::WrongSnapshot, "wrong-anchor")
    );

    let record = refused(
        admit(
            &fixture,
            "current",
            json!({"kind": "other", "name": "x"}),
            true,
            two_objects(),
        )
        .err(),
    );
    assert_eq!(
        (record.code, record.cause),
        (Code::InvalidRuntimeInput, "wrong-value-kind")
    );
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("anchor")
    );
}

/// Trace: FR-120-AC-7
#[test]
fn an_initial_snapshot_with_a_wrong_model_a_dangling_reference_or_an_incomplete_population_refuses()
{
    let mut fixture = fixture();
    let dangling = vec![object("b", "2", parent_of("zzz"))];
    let record = refused(admit(&fixture, "current", initial_anchor(), true, dangling).err());
    assert_eq!(
        (record.code, record.cause),
        (
            Code::DanglingReference,
            "absent-target-in-complete-population"
        )
    );

    let Some(AdmissionFailure::Incomplete(record)) =
        admit(&fixture, "current", initial_anchor(), false, two_objects()).err()
    else {
        panic!("an incomplete listed population is Incomplete")
    };
    assert_eq!(
        (record.code, record.cause),
        (Code::IncompletePopulation, "incomplete-scope")
    );

    fixture.header["version"] = json!("9.9.9");
    let record = refused(admit(&fixture, "current", initial_anchor(), true, two_objects()).err());
    assert_eq!(
        (record.code, record.cause),
        (Code::InvalidModelBinding, "wrong-model-selection")
    );
}

/// The `(field name, value)` slots of `reference` in `closure`'s environment.
fn slots(
    fixture: &Fixture,
    environment: &qsl_semantics::model::object_environment::ObjectEnvironment,
    reference: &quire_exact::ObjectReference,
) -> Vec<(String, Option<Value>)> {
    let types = fixture.graph.scope().types();
    types
        .attributes(reference.object_type())
        .expect("object type attributes")
        .iter()
        .map(|attribute| {
            let slot = environment
                .objects()
                .attribute(types, reference, &attribute.identity());
            let value = match slot {
                Some(FieldValue::Present(value)) => Some(value.clone()),
                Some(FieldValue::Absent) => None,
                other => panic!("unexpected slot {other:?}"),
            };
            (attribute.field().name().to_owned(), value)
        })
        .collect()
}

/// Trace: FR-120-AC-4
#[test]
fn a_states_observation_matches_the_environment_fr106_admits_from_the_same_document() {
    let fixture = fixture();
    let (selected, provision) =
        snapshot(&fixture, "current", initial_anchor(), true, two_objects());
    let (state, _) = fixture
        .model
        .admit_initial(&provision, &selected, ObservationLimits::default())
        .expect("admits");
    let identity = label("state");
    let observation = fixture
        .model
        .observation(&state, identity.clone())
        .expect("observation");
    assert_eq!(observation.identity, identity);
    assert_eq!(
        observation.populations,
        BTreeMap::from([(POPULATION.to_owned(), true)])
    );
    assert!(observation.out_of_range.is_empty());

    let (admitted, _usage) = admit_current_snapshot(
        fixture.graph.model_selections(),
        fixture.graph.scope().types(),
        &fixture.packages,
        ModelNormalizationLimits::default(),
        &provision,
        &selected,
        ObservationLimits::default(),
    )
    .expect("FR-106 admits the same document");

    let (_, universe) = declared_type_universe_for(
        fixture.graph.model_selections(),
        &fixture.packages,
        ModelNormalizationLimits::default(),
        &config_version_key(),
    )
    .expect("universe derivation")
    .expect("ConfigVersion is declared");
    for key in ["a", "b"] {
        let ours = observation
            .environment
            .objects()
            .find(universe, key)
            .expect("observation holds the object");
        let theirs = admitted
            .objects()
            .find(universe, key)
            .expect("FR-106 holds the object");
        assert_eq!(ours, theirs, "same universe, effective type and key");
        let ours = slots(&fixture, &observation.environment, ours);
        let theirs = slots(&fixture, &admitted, theirs);
        assert_eq!(ours.len(), theirs.len());
        for ((name, ours), (expected, theirs)) in ours.iter().zip(&theirs) {
            assert_eq!(name, expected);
            assert!(same(ours.as_ref(), theirs.as_ref()), "field {name}");
        }
    }
    let a = observation
        .environment
        .objects()
        .find(universe, "a")
        .expect("a");
    let a = slots(&fixture, &observation.environment, a);
    assert_eq!(a[0].0, "versionNumber");
    assert_integer(a[0].1.as_ref().expect("present"), 1);
}

/// Trace: FR-120-AC-5
#[test]
fn a_changed_state_observes_its_changed_field() {
    let fixture = fixture();
    let (mut state, _) =
        admit(&fixture, "current", initial_anchor(), true, two_objects()).expect("admits");
    state
        .populations
        .get_mut(&population_key())
        .and_then(|objects| objects.get_mut("a"))
        .expect("a")
        .fields
        .insert(field("versionNumber"), integer(5));
    let observation = fixture
        .model
        .observation(&state, label("changed"))
        .expect("observation");
    let (_, universe) = declared_type_universe_for(
        fixture.graph.model_selections(),
        &fixture.packages,
        ModelNormalizationLimits::default(),
        &config_version_key(),
    )
    .expect("universe derivation")
    .expect("declared");
    let a = observation
        .environment
        .objects()
        .find(universe, "a")
        .expect("a");
    let a = slots(&fixture, &observation.environment, a);
    assert_eq!(a[0].0, "versionNumber");
    assert_integer(a[0].1.as_ref().expect("present"), 5);
}

#[test]
fn a_state_that_lacks_a_required_field_has_no_observation() {
    let fixture = fixture();
    let (mut state, _) =
        admit(&fixture, "current", initial_anchor(), true, two_objects()).expect("admits");
    state
        .populations
        .get_mut(&population_key())
        .and_then(|objects| objects.get_mut("a"))
        .expect("a")
        .fields
        .clear();
    assert!(matches!(
        fixture.model.observation(&state, label("broken")),
        Err(AdmissionFailure::Fault(_))
    ));
}

fn states_object(version: i64) -> StateObject {
    StateObject {
        type_identity: config_version_key(),
        fields: BTreeMap::from([(field("versionNumber"), integer(version))]),
    }
}

fn frame_objects<'a>(objects: &'a BTreeMap<String, StateObject>) -> Vec<FrameObject<'a, Value>> {
    objects
        .iter()
        .map(|(key, object)| FrameObject {
            object: key.as_str(),
            type_identity: object.type_identity.clone(),
            fields: object.fields.clone(),
        })
        .collect()
}

fn decide(
    fixture: &Fixture,
    effect: &OperationEffect,
    pre: &BTreeMap<String, StateObject>,
    post: &BTreeMap<String, StateObject>,
) -> Result<(BTreeSet<String>, BTreeSet<String>), Box<ModelRefusalCause>> {
    let index = fixture
        .model
        .population_index(&population_key())
        .expect("the population's index");
    let created: Vec<String> = post
        .keys()
        .filter(|key| !pre.contains_key(*key))
        .cloned()
        .collect();
    let deleted: Vec<String> = pre
        .keys()
        .filter(|key| !post.contains_key(*key))
        .cloned()
        .collect();
    let frame = FrameDecision {
        index,
        ancestor_steps: fixture.model.ancestor_steps(),
        effect,
        declared_created: &created,
        declared_deleted: &deleted,
        receiver: Some("a"),
        arguments: &[],
    };
    decide_frame(
        &frame,
        &frame_objects(pre),
        &frame_objects(post),
        |_, pre, post| same(pre, post),
        &mut |_: FrameCharge| Ok::<(), std::convert::Infallible>(()),
    )
    .map(|delta| (delta.created, delta.deleted))
    .map_err(|stop| match stop {
        FrameStop::Refused(refusal) => Box::new(refusal.cause),
        FrameStop::Denied(never) => match never {},
    })
}

/// Trace: FR-120-AC-3
#[test]
fn the_public_frame_decision_returns_the_created_and_deleted_sets() {
    let fixture = fixture();
    let pre = BTreeMap::from([("a".to_owned(), states_object(1))]);
    let post = BTreeMap::from([("a".to_owned(), states_object(2))]);
    let modifies = OperationEffect {
        modifies: vec![field("versionNumber")],
        ..OperationEffect::default()
    };
    assert_eq!(
        decide(&fixture, &modifies, &pre, &post).expect("in frame"),
        (BTreeSet::new(), BTreeSet::new())
    );

    let created = BTreeMap::from([
        ("a".to_owned(), states_object(1)),
        ("c".to_owned(), states_object(1)),
    ]);
    let creates = OperationEffect {
        creates: vec![config_version_key()],
        ..OperationEffect::default()
    };
    assert_eq!(
        decide(&fixture, &creates, &pre, &created).expect("in frame"),
        (BTreeSet::from(["c".to_owned()]), BTreeSet::new())
    );
    let deletes = OperationEffect {
        deletes: vec![config_version_key()],
        ..OperationEffect::default()
    };
    assert_eq!(
        decide(&fixture, &deletes, &created, &pre).expect("in frame"),
        (BTreeSet::new(), BTreeSet::from(["c".to_owned()]))
    );
}

/// Trace: FR-120-AC-1
#[test]
fn the_public_frame_decision_refuses_a_change_outside_the_frame() {
    let fixture = fixture();
    let pre = BTreeMap::from([("a".to_owned(), states_object(1))]);
    let post = BTreeMap::from([("a".to_owned(), states_object(2))]);
    let empty = OperationEffect::default();
    assert_eq!(
        *decide(&fixture, &empty, &pre, &post).expect_err("outside the frame"),
        ModelRefusalCause::FrameFieldWriteOutsideGrant {
            object: "a".to_owned(),
            field: field("versionNumber"),
        }
    );
    let created = BTreeMap::from([
        ("a".to_owned(), states_object(1)),
        ("c".to_owned(), states_object(1)),
    ]);
    assert_eq!(
        *decide(&fixture, &empty, &pre, &created).expect_err("create outside the frame"),
        ModelRefusalCause::FrameCreateOutsideGrant {
            object: "c".to_owned(),
            type_name: config_version_key(),
        }
    );
    assert_eq!(
        *decide(&fixture, &empty, &created, &pre).expect_err("delete outside the frame"),
        ModelRefusalCause::FrameDeleteOutsideGrant {
            object: "c".to_owned(),
            type_name: config_version_key(),
        }
    );
}

#[test]
fn only_a_declared_population_has_a_model_index() {
    let fixture = fixture();
    assert!(fixture.model.population_index(&population_key()).is_some());
    let unknown = DeclarationKey {
        package: PACKAGE_IDENTITY.to_owned(),
        node: "ix://example/config-version/nope".to_owned(),
    };
    assert!(fixture.model.population_index(&unknown).is_none());
}
