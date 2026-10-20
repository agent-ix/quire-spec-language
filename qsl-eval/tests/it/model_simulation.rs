// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120 (TC-471, TC-472, TC-473): `ModelSystem`, the transition system of a
//! checked package's state family, over the `test/counters` fixture.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_eval::simulation::model::{
    explore_model, sample_model, ClauseOutcome, ExpansionLimits, ModelFinding, ModelSampleError,
    ModelSimulationRequest, ModelSystem, PopulationUniverse, StepEffect, UndecidedSuccessor,
    Unsteppable,
};
use qsl_eval::simulation::{
    sample_probabilistic, Exploration, Limits, NotSimulated, Outcome, ReplayError, SampleStop,
    StopReason, TransitionSystem,
};
use qsl_foundation::diagnostic::Code;
use qsl_foundation::selection::DefinitionRef;
use qsl_foundation::CatalogCode;
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, meaning, package_input};
use qsl_semantics::model::key::{hex, DeclarationKey};
use qsl_semantics::model::observation::{
    declared_type_universe_for, AdmissionFailure, AnchorKind, DocumentRef, ObservationLimits,
    SelectedAnchor,
};
use qsl_semantics::model::state::{ModelState, StateModel};
use quire_exact::Cancel;
use quire_exact::{ObjectId, ObjectReference, ScalarLimits, Value};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value as Json};

pub(crate) const PACKAGE: &str = "test/counters";

fn ident(suffix: &str) -> String {
    format!("ix://{PACKAGE}/{suffix}")
}

fn key(suffix: &str) -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE.to_owned(),
        node: ident(suffix),
    }
}

fn counters() -> DeclarationKey {
    key("counters")
}

fn counter() -> DeclarationKey {
    key("Counter")
}

fn field(name: &str) -> DeclarationKey {
    key(&format!("Counter/{name}"))
}

/// An operation's declaration identity text (FR-120 "Identities").
fn operation_text(name: &str) -> String {
    format!(
        r#"{{"digest_domain":"sha256-jcs","node":"{}","package":"{PACKAGE}"}}"#,
        ident(&format!("Counter/{name}"))
    )
}

// ---------------------------------------------------------------------------
// The fixture domain package
// ---------------------------------------------------------------------------

const PLACEHOLDER: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

fn origin(identity: &str) -> Json {
    json!({"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0",
        "inputIdentities": [identity]}})
}

fn wire_type(identity: &str, kind: &str, extra: Json) -> Json {
    let mut node = json!({
        "identity": identity, "displayName": identity,
        "kind": {"module": PACKAGE, "name": kind}, "roles": [],
        "origin": origin(identity), "constraints": [], "extensions": [],
        "unknownPolicy": "reject",
    });
    for (name, value) in extra.as_object().expect("extra members") {
        node[name] = value.clone();
    }
    node
}

fn wire_field(name: &str, type_ref: &str, presence: &str) -> Json {
    let identity = ident(&format!("Counter/{name}"));
    json!({
        "identity": identity, "name": name, "typeRef": type_ref, "presence": presence,
        "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": origin(&identity),
    })
}

fn bound_type(name: &str, upper: u64) -> Json {
    let identity = ident(name);
    let constraint = |keyword: &str, value: String| {
        json!({
            "identity": format!("{identity}/constraints/{keyword}"), "keyword": keyword,
            "operands": {"value": value}, "appliesTo": "ix://quire/native/Integer",
            "diagnosticCode": format!("bound.{keyword}"), "origin": origin(&identity),
        })
    };
    wire_type(
        &identity,
        "value_type",
        json!({"scalar": "integer",
            "constraints": [constraint("min", "0".into()), constraint("max", upper.to_string())]}),
    )
}

/// One operation of `Counter`: parameters as `(name, type)` with a native
/// or package type reference, an optional result and its frame.
fn operation(name: &str, params: &[(&str, &str)], result: Option<&str>, frame: Json) -> Json {
    let identity = ident(&format!("Counter/{name}"));
    let params: Vec<Json> = params
        .iter()
        .map(|(parameter, type_ref)| {
            let parameter_identity = format!("{identity}/{parameter}");
            json!({
                "identity": parameter_identity, "name": parameter, "typeRef": type_ref,
                "presence": "required", "nullable": false, "defaultKind": "none",
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                "origin": origin(&parameter_identity),
            })
        })
        .collect();
    let mut node = json!({
        "identity": identity, "name": name, "params": params, "pre": [], "post": [],
        "origin": {"source": {"sourceIdentity": ident("spec"), "path": "spec.qspec",
            "startLine": 1, "startColumn": 1}},
        "frame": frame,
    });
    if let Some(type_ref) = result {
        node["returns"] = json!({"typeRef": type_ref, "nullable": false,
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true}});
    }
    node
}

fn frame(modifies: &[&str], creates: bool, deletes: bool) -> Json {
    json!({
        "modifies": modifies.iter().map(|name| ident(&format!("Counter/{name}"))).collect::<Vec<_>>(),
        "creates": if creates { vec![ident("Counter")] } else { vec![] },
        "deletes": if deletes { vec![ident("Counter")] } else { vec![] },
    })
}

pub(crate) fn increment() -> Json {
    operation("increment", &[], None, frame(&["value"], false, false))
}

pub(crate) fn acquire() -> Json {
    operation("acquire", &[], None, frame(&["value"], false, false))
}

pub(crate) fn release() -> Json {
    operation("release", &[], None, frame(&["value"], false, false))
}

pub(crate) fn counters_document(operations: Vec<Json>, extras: bool) -> Vec<u8> {
    let counter_identity = ident("Counter");
    let constructs = vec![
        ("object_type", meaning::OBJECT_TYPE),
        ("population", meaning::POPULATION),
        ("value_type", meaning::VALUE_TYPE),
    ];
    let constructs: Vec<Json> = constructs
        .into_iter()
        .map(|(name, meaning)| {
            json!({"kind": {"module": PACKAGE, "name": name}, "moduleVersion": "1.0.0",
                "manifestDigest": PLACEHOLDER,
                "construct": {"identity": "none", "shape": "record", "members": {},
                    "meaning": meaning}})
        })
        .collect();
    let population = ident("counters");
    let mut fields = vec![
        wire_field("value", &ident("Level"), "required"),
        wire_field("label", &ident("Flag"), "required"),
        wire_field("next", &counter_identity, "optional"),
    ];
    if extras {
        // Granted by no frame and with no finite domain.
        fields.push(wire_field("tag", "ix://quire/native/Integer", "optional"));
        fields.push(wire_field("id", "ix://quire/native/Integer", "required"));
    }
    json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1", "digest": "sha256-jcs"},
        "source": {"identity": ident("spec"), "version": "1.0.0", "dialect": "spec-bundle",
            "digest": PLACEHOLDER},
        "package": {"identity": PACKAGE, "version": "1.0.0", "manifestDigest": PLACEHOLDER,
            "mappingVersions": [], "profileVersions": [], "lockDigest": PLACEHOLDER},
        "occurrences": [], "extensions": [],
        "constructs": constructs,
        "types": [
            bound_type("Level", 2),
            bound_type("Flag", 1),
            bound_type("Count", 100),
            bound_type("Huge", 1_000_000_000_000_000),
            wire_type(&counter_identity, "object_type", json!({
                "supertypes": [],
                "fields": fields,
                "operations": operations,
                "relationships": [],
            })),
        ],
        "populations": [{
            "identity": population, "displayName": population,
            "kind": {"module": PACKAGE, "name": "population"},
            "members": [counter_identity], "extent": "closed", "origin": origin(&population),
        }],
    })
    .to_string()
    .into_bytes()
}

// ---------------------------------------------------------------------------
// A checked package over the fixture
// ---------------------------------------------------------------------------

struct World {
    extras: bool,
    package: CheckedPackage,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
    header: Json,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
}

/// The fixture package with `operations`, checked with `clauses` as its unit.
fn world(operations: Vec<Json>, clauses: &str) -> World {
    try_world(operations, clauses).expect("the unit checks")
}

/// [`world`] with `tag: Integer?` and `id: Integer` added to `Counter`: fields
/// no frame grants and no finite domain covers.
fn world_with_unbounded_fields(operations: Vec<Json>) -> World {
    build_world(operations, "", true).expect("the unit checks")
}

/// [`world`], or the refusals S3 gives the unit.
fn try_world(
    operations: Vec<Json>,
    clauses: &str,
) -> Result<World, Vec<qsl_semantics::check::CheckRefusal>> {
    build_world(operations, clauses, false)
}

fn build_world(
    operations: Vec<Json>,
    clauses: &str,
    extras: bool,
) -> Result<World, Vec<qsl_semantics::check::CheckRefusal>> {
    let document = counters_document(operations, extras);
    let packages = package_input([document.as_slice()]);
    let digest = *packages.keys().next().expect("one model");
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.model.complete/v1\";\n\
         model M = \"{PACKAGE}\" version \"1.0.0\" digest \"sha256-jcs:{}\";\nfunction Noop using v(): Boolean pure {{ true }}\n{clauses}\n",
        hex(&digest)
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "fr-120", "fixture", "1"),
        "fr-120.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 parses the unit");
    assert!(parsed.is_admissible(), "S1: {:?}", parsed.diagnostics());
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the model selection admits")
    .with_parsed_operation_clauses(|text| {
        qsl_forms::parse_clause_text(text, qsl_cst::Limits::default())
    })
    .expect("the kept operation clauses parse");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        built,
        models,
        Vec::new(),
    )
    .expect("the unit assembles");
    let graph = declarations.check(CheckingLimits::default())?;
    Ok(World {
        extras,
        package: CheckedPackage::link(graph),
        header: json!({"identity": PACKAGE, "version": "1.0.0",
            "digest": format!("sha256-jcs:{}", hex(&digest))}),
        packages,
        snapshots: BTreeMap::new(),
    })
}

/// One object of a snapshot: key, `value`, `label` and `next`.
struct Obj(&'static str, i64, i64, Option<&'static str>);

fn object_json(object: &Obj, extras: bool) -> Json {
    let next = match object.3 {
        None => json!({"absent": {}}),
        Some(target) => {
            json!({"present": {"reference": {"population": ident("counters"), "key": target}}})
        }
    };
    let mut fields = json!({"value": {"integer": object.1.to_string()},
        "label": {"integer": object.2.to_string()}, "next": next});
    if extras {
        fields["tag"] = json!({"absent": {}});
        fields["id"] = json!({"integer": "0"});
    }
    json!({"key": object.0, "type": ident("Counter"), "fields": fields})
}

impl World {
    /// A snapshot document `name`, provisioned into this world.
    fn snapshot(
        &mut self,
        name: &str,
        observation: &str,
        anchor: Json,
        objects: &[Obj],
    ) -> DocumentRef {
        let mut document = json!({
            "format": "quire.state.snapshot/v1",
            "identity": {"authority": "test", "identity": name, "revision_namespace": "fixture",
                "revision": "fixture:1"},
            "observation": observation, "model": self.header,
            "populations": [{"population": ident("counters"), "complete": true,
                "objects": objects.iter().map(|object| object_json(object, self.extras)).collect::<Vec<_>>()}],
        });
        if !anchor.is_null() {
            document["anchor"] = anchor;
        }
        let bytes = document.to_string().into_bytes();
        let read = quire_canonical::read(&bytes, u64::MAX).expect("JSON fixture");
        let digest = *quire_canonical::sha256(&read, quire_canonical::Limits::new(u64::MAX))
            .expect("canonical fixture")
            .as_bytes();
        self.snapshots.insert(digest, bytes);
        DocumentRef {
            authority: "test".to_owned(),
            identity: name.to_owned(),
            revision_namespace: "fixture".to_owned(),
            revision: "fixture:1".to_owned(),
            digest,
        }
    }

    /// The initial snapshot `s0`: `c1` with `value` `value`, anchored
    /// `{initialization, start}`.
    fn start(&mut self, name: &str, objects: &[Obj]) -> DocumentRef {
        self.snapshot(
            name,
            "current",
            json!({"kind": "initialization", "name": "start"}),
            objects,
        )
    }

    fn model(&self) -> StateModel {
        StateModel::new(
            self.package.graph().model_selections(),
            self.package.graph().scope().types(),
            &self.packages,
            ModelNormalizationLimits::default(),
        )
        .expect("model views")
    }

    /// The reference to `object` of `Counter`, derived from the package's
    /// own universe rule rather than from the simulator.
    fn reference(&self, object: &str) -> ObjectReference {
        let (effective, universe) = declared_type_universe_for(
            self.package.graph().model_selections(),
            &self.packages,
            ModelNormalizationLimits::default(),
            &counter(),
        )
        .expect("universe derives")
        .expect("Counter is declared");
        ObjectReference::new(universe, effective, ObjectId::new(object).expect("an id"))
    }
}

const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: 1 << 20,
    decimal_digits: 1 << 20,
    scale_expansion: 1 << 20,
    text_input_bytes: 1 << 20,
    text_scalars: 1 << 20,
    normalized_scalars: 1 << 20,
    unit_edges: 1 << 20,
    value_occurrences: 1 << 20,
    work_units: 1 << 30,
    result_units: 1 << 30,
};

const ZERO: ScalarLimits = ScalarLimits {
    integer_bits: 0,
    decimal_digits: 0,
    scale_expansion: 0,
    text_input_bytes: 0,
    text_scalars: 0,
    normalized_scalars: 0,
    unit_edges: 0,
    value_occurrences: 0,
    work_units: 0,
    result_units: 0,
};

fn sampler() -> DefinitionRef {
    DefinitionRef::new("agent-ix", "quire.simulation.sampler/v1").expect("a valid DefinitionRef")
}

fn universe(keys: &[&str]) -> Vec<PopulationUniverse> {
    vec![PopulationUniverse {
        population: counters(),
        keys: keys.iter().map(|key| (*key).to_owned()).collect(),
    }]
}

fn request<'a>(
    world: &'a World,
    initial: Vec<DocumentRef>,
    universes: Vec<PopulationUniverse>,
    meter: ScalarLimits,
    max_candidates: u64,
) -> ModelSimulationRequest<'a> {
    ModelSimulationRequest {
        package: &world.package,
        packages: &world.packages,
        normalization: ModelNormalizationLimits::default(),
        snapshots: &world.snapshots,
        initial,
        universes,
        observation: ObservationLimits::default(),
        expansion: ExpansionLimits { max_candidates },
        meter,
    }
}

fn system<'w>(
    world: &'w World,
    initial: Vec<DocumentRef>,
    keys: &[&str],
    meter: ScalarLimits,
) -> ModelSystem<'w> {
    ModelSystem::new(request(world, initial, universe(keys), meter, 1_000_000))
        .expect("the system admits")
}

// ---------------------------------------------------------------------------
// Reading results
// ---------------------------------------------------------------------------

fn int_field(state: &ModelState, object: &str, name: &str) -> String {
    match state.populations[&counters()][object]
        .fields
        .get(&field(name))
    {
        Some(Value::Integer(value)) => value.to_string(),
        other => panic!("{object}.{name} is not an integer: {other:?}"),
    }
}

/// The `(value, successor)` pairs of expanding the only initial state.
fn successors_of(system: &ModelSystem<'_>, index: usize) -> Vec<(String, ModelStateView)> {
    let initial = system.initial();
    let expansion = system
        .successors(&initial[index])
        .expect("the state expands");
    expansion
        .successors
        .into_iter()
        .map(|(transition, state)| {
            let bytes = quire_canonical::to_vec(&transition, quire_canonical::Limits::new(1 << 20))
                .expect("a transition encodes");
            (
                String::from_utf8(bytes).expect("UTF-8"),
                ModelStateView(state.state().clone()),
            )
        })
        .collect()
}

struct ModelStateView(ModelState);

impl ModelStateView {
    fn value(&self, object: &str) -> String {
        int_field(&self.0, object, "value")
    }

    fn objects(&self) -> Vec<&str> {
        self.0.populations[&counters()]
            .keys()
            .map(String::as_str)
            .collect()
    }
}

fn explore(system: &ModelSystem<'_>) -> Exploration<ModelFinding> {
    explore_model(system, 10_000, Limits::default(), || false).expect("the request is bounded")
}

fn stats(exploration: &Exploration<ModelFinding>) -> (usize, usize, usize) {
    match &exploration.outcome {
        Outcome::Exhaustive(stats) => (stats.states, stats.transitions, stats.depth),
        other => panic!("expected an exhaustive run, got {other:?}"),
    }
}

fn expected_transition(operation: &str, receiver: &ObjectReference, arguments: &[&str]) -> String {
    let receiver = {
        let enc = format!(
            r#"{{"identity":"{}","object_type":"{}","type":"reference","universe":"{}"}}"#,
            hex_bytes(receiver.object().as_str().as_bytes()),
            hex(receiver.object_type().as_bytes()),
            hex(receiver.universe().as_bytes()),
        );
        enc
    };
    format!(
        r#"{{"arguments":[{}],"operation":{},"receiver":{receiver},"type":"transition"}}"#,
        arguments.join(","),
        serde_json::to_string(&operation_text(operation)).expect("a JSON string"),
    )
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn int_arg(value: i64) -> String {
    format!(r#"{{"type":"integer","value":"{value}"}}"#)
}

// ---------------------------------------------------------------------------
// TC-471: successors, state key
// ---------------------------------------------------------------------------

/// FR-120-AC-1 (TC-471): `increment()` over `s0` has three successors with
/// the receiver apart from the (empty) arguments, and `label` and `next`
/// keep their values.
#[trace("TC-471", "FR-120-AC-1")]
#[test]
fn an_unconstrained_increment_has_one_successor_per_value() {
    let mut world = world(vec![increment()], "");
    let s0 = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = system(&world, vec![s0], &["c1", "c2"], UNLIMITED);

    let successors = successors_of(&system, 0);
    let identity = expected_transition("increment", &world.reference("c1"), &[]);
    let mut values: Vec<String> = successors
        .iter()
        .map(|(_, state)| state.value("c1"))
        .collect();
    values.sort();
    assert_eq!(values, ["0", "1", "2"]);
    for (transition, state) in &successors {
        assert_eq!(transition, &identity);
        assert_eq!(int_field(&state.0, "c1", "label"), "0");
        assert!(!state.0.populations[&counters()]["c1"]
            .fields
            .contains_key(&field("next")));
    }
}

/// FR-120-AC-1 (TC-471): a postcondition leaves only `v1`; a precondition
/// removes the `increment` successors of `v2` and keeps three at `v0`, `v1`.
#[trace("TC-471", "FR-120-AC-1")]
#[test]
fn increment_contracts_select_successors() {
    let post = "post Inc using v on M::Counter::increment { self.value - pre(self.value) = 1 }";
    let mut world = world(vec![increment()], post);
    let s0 = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = self::system(&world, vec![s0], &["c1", "c2"], UNLIMITED);
    let successors = successors_of(&system, 0);
    assert_eq!(successors.len(), 1);
    assert_eq!(successors[0].1.value("c1"), "1");

    let pre = "pre CanInc using v on M::Counter::increment { self.value < 2 }";
    let mut world = self::world(vec![increment()], pre);
    let docs = [
        world.start("v0", &[Obj("c1", 0, 0, None)]),
        world.start("v1", &[Obj("c1", 1, 0, None)]),
        world.start("v2", &[Obj("c1", 2, 0, None)]),
    ];
    let system = self::system(&world, docs.to_vec(), &["c1", "c2"], UNLIMITED);
    let by_value: BTreeMap<String, usize> = system
        .initial()
        .iter()
        .enumerate()
        .map(|(index, state)| (int_field(state.state(), "c1", "value"), index))
        .collect();
    assert_eq!(successors_of(&system, by_value["0"]).len(), 3);
    assert_eq!(successors_of(&system, by_value["1"]).len(), 3);
    assert_eq!(successors_of(&system, by_value["2"]).len(), 0);
}

/// FR-120-AC-2 (TC-471): `setTo(n: Int[0, 2])` has one successor per `n`,
/// each receiver its own identity, and two preconditions both must hold.
#[trace("TC-471", "FR-120-AC-2")]
#[test]
fn arguments_come_from_the_parameter_domain_and_preconditions_conjoin() {
    let set_to = operation(
        "setTo",
        &[("n", &ident("Level"))],
        None,
        frame(&["value"], false, false),
    );
    let clauses = "post SetsIt using v on M::Counter::setTo { self.value - n = 0 }";
    let mut world = world(vec![set_to.clone()], clauses);
    let s0 = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = self::system(&world, vec![s0], &["c1", "c2"], UNLIMITED);
    let successors = successors_of(&system, 0);
    let c1 = world.reference("c1");
    let mut transitions: Vec<String> = successors.iter().map(|(t, _)| t.clone()).collect();
    transitions.sort();
    let expected: Vec<String> = (0..3)
        .map(|n| expected_transition("setTo", &c1, &[&int_arg(n)]))
        .collect();
    assert_eq!(transitions, expected);

    let mut world = self::world(vec![set_to.clone()], clauses);
    let both = world.start("both", &[Obj("c1", 0, 0, None), Obj("c2", 0, 0, None)]);
    let system = self::system(&world, vec![both], &["c1", "c2"], UNLIMITED);
    let successors = successors_of(&system, 0);
    let receivers: std::collections::BTreeSet<String> = successors
        .iter()
        .map(|(transition, _)| transition.clone())
        .collect();
    assert_eq!(receivers.len(), 6, "each receiver gives its own identity");

    let guarded = "pre A using v on M::Counter::setTo { self.value >= 1 }\n\
                   pre B using v on M::Counter::setTo { self.value <= 1 }";
    let mut world = self::world(vec![set_to], guarded);
    let docs = [
        world.start("v0", &[Obj("c1", 0, 0, None)]),
        world.start("v1", &[Obj("c1", 1, 0, None)]),
        world.start("v2", &[Obj("c1", 2, 0, None)]),
    ];
    let system = self::system(&world, docs.to_vec(), &["c1"], UNLIMITED);
    let counts: BTreeMap<String, usize> = (0..3)
        .map(|index| {
            (
                int_field(system.initial()[index].state(), "c1", "value"),
                successors_of(&system, index)
                    .into_iter()
                    .map(|(transition, _)| transition)
                    .collect::<std::collections::BTreeSet<_>>()
                    .len(),
            )
        })
        .collect();
    assert_eq!(counts["0"], 0);
    assert_eq!(counts["1"], 3);
    assert_eq!(counts["2"], 0);
}

/// FR-120-AC-3 (TC-471): `spawn()` creates `c2` under every value of its
/// fields, and `remove()` deletes any closed subset of the objects.
#[trace("TC-471", "FR-120-AC-3")]
#[test]
fn creation_and_deletion_candidates_are_closed_and_framed() {
    let spawn = operation("spawn", &[], None, frame(&[], true, false));
    let mut world = world(vec![spawn], "");
    let s0 = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = self::system(&world, vec![s0], &["c1", "c2"], UNLIMITED);
    let successors = successors_of(&system, 0);
    assert_eq!(successors.len(), 1 + 18);
    assert_eq!(
        successors
            .iter()
            .filter(|(_, state)| state.objects() == ["c1"])
            .count(),
        1
    );
    assert_eq!(
        successors
            .iter()
            .filter(|(_, state)| state.objects() == ["c1", "c2"])
            .count(),
        18
    );

    let remove = operation("remove", &[], None, frame(&[], false, true));
    let mut world = self::world(vec![remove], "");
    let linked = world.start(
        "linked",
        &[Obj("c1", 0, 0, Some("c2")), Obj("c2", 0, 0, None)],
    );
    let system = self::system(&world, vec![linked], &["c1", "c2"], UNLIMITED);
    let successors = successors_of(&system, 0);
    let c1 = expected_transition("remove", &world.reference("c1"), &[]);
    let c2 = expected_transition("remove", &world.reference("c2"), &[]);
    let kept = |receiver: &str| -> Vec<Vec<String>> {
        let mut sets: Vec<Vec<String>> = successors
            .iter()
            .filter(|(transition, _)| transition == receiver)
            .map(|(_, state)| state.objects().iter().map(|s| (*s).to_owned()).collect())
            .collect();
        sets.sort();
        sets
    };
    // Deleting {c2} alone leaves c1.next dangling and is no candidate.
    let expected: Vec<Vec<String>> =
        vec![vec![], vec!["c1".into(), "c2".into()], vec!["c2".into()]];
    assert_eq!(kept(&c1), expected);
    assert_eq!(kept(&c2), expected);
    assert_eq!(successors.len(), 6);
}

/// FR-120-AC-4 (TC-471): the state key of `s0` is FR-181's envelope with the
/// population's objects as records, the `U` and `E` of the reference admission
/// gives `c1`, and a changed field changes the key.
#[trace("TC-471", "FR-120-AC-4")]
#[test]
fn the_state_key_is_the_simulation_state_envelope() {
    let mut world = world(vec![increment()], "");
    let s0 = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let s0_label = world.start("s0-label", &[Obj("c1", 0, 1, None)]);
    let s0_next = world.start("s0-next", &[Obj("c1", 0, 0, Some("c1"))]);
    let system = self::system(
        &world,
        vec![s0.clone(), s0_label, s0_next],
        &["c1", "c2"],
        UNLIMITED,
    );
    let model = world.model();
    let (state, _) = model
        .admit_initial(&world.snapshots, &s0, ObservationLimits::default())
        .expect("s0 admits");
    let observation = model
        .observation(&state, s0.clone())
        .expect("the observation builds");
    let reference = world.reference("c1");
    assert!(
        observation.environment.objects().contains(&reference),
        "admission gives c1 the universe and type the key reads"
    );

    let initial = system.initial();
    let bytes = |index: usize| -> String {
        let key = system.key(&initial[index]);
        String::from_utf8(
            quire_canonical::to_vec(&key, quire_canonical::Limits::new(1 << 20)).expect("encodes"),
        )
        .expect("UTF-8")
    };
    // QSpec FR-181: an absent presence-optional field is a plain `null`
    // entry; a present one is its bare encoded value.
    let own = json!({"type": "reference",
        "universe": hex(reference.universe().as_bytes()),
        "object_type": hex(reference.object_type().as_bytes()),
        "identity": "6331"});
    let record = |label: i64, next: Json| {
        json!({"type": "record", "name": counter_text(), "fields": [
            {"name": "value", "value": {"type": "integer", "value": "0"}},
            {"name": "label", "value": {"type": "integer", "value": label.to_string()}},
            {"name": "next", "value": next}]})
    };
    let envelope = |label: i64, next: Json| {
        let empty = json!({"type": "map", "entries": []});
        json!({"type": "simulation-state",
            "semantic": {"type": "map", "entries": [{
                "key": {"type": "text", "value": population_text()},
                "value": {"type": "map", "entries": [{
                    "key": {"type": "reference",
                        "universe": hex(reference.universe().as_bytes()),
                        "object_type": hex(reference.object_type().as_bytes()),
                        "identity": "6331"},
                    "value": record(label, next)}]}}]},
            "control": empty, "queues": empty, "roles": empty, "observations": empty,
            "bounds": empty, "memory": empty})
    };
    let canonical = |value: Json| -> String {
        let read = quire_canonical::read(value.to_string().as_bytes(), u64::MAX).expect("reads");
        String::from_utf8(
            quire_canonical::to_vec(&read, quire_canonical::Limits::new(1 << 20)).expect("encodes"),
        )
        .expect("UTF-8")
    };
    assert_ne!(bytes(0), bytes(1), "label 1 is a different key");
    let keys = [bytes(0), bytes(1), bytes(2)];
    for (label, next) in [(0, Json::Null), (1, Json::Null), (0, own)] {
        let expected = canonical(envelope(label, next));
        assert!(keys.contains(&expected), "no key equals {expected}");
    }
}

fn counter_text() -> String {
    format!(
        r#"{{"digest_domain":"sha256-jcs","node":"{}","package":"{PACKAGE}"}}"#,
        ident("Counter")
    )
}

fn population_text() -> String {
    format!(
        r#"{{"digest_domain":"sha256-jcs","node":"{}","package":"{PACKAGE}"}}"#,
        ident("counters")
    )
}

// ---------------------------------------------------------------------------
// TC-472: exploration, findings, stops
// ---------------------------------------------------------------------------

const SMALL: &str = "invariant Small using v on M::Counter at current { self.value < 2 }";
const CAN_INC: &str = "pre CanInc using v on M::Counter::increment { self.value < 2 }";
const INC: &str = "post Inc using v on M::Counter::increment { self.value - pre(self.value) = 1 }";

fn clauses(parts: &[&str]) -> String {
    parts.join("\n")
}

fn handler(name: &str) -> SelectedAnchor {
    SelectedAnchor {
        kind: AnchorKind::Handler,
        name: operation_text(name),
    }
}

/// FR-120-AC-5 (TC-472): the violating state is recorded, kept and expanded.
#[trace("TC-472", "FR-120-AC-5")]
#[test]
fn an_invariant_violation_is_a_finding_and_exploration_continues() {
    let mut world = world(vec![increment()], &clauses(&[CAN_INC, INC, SMALL]));
    let s0 = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = self::system(&world, vec![s0], &["c1", "c2"], UNLIMITED);
    let exploration = explore(&system);
    assert_eq!(stats(&exploration), (3, 2, 2));
    assert_eq!(exploration.findings.len(), 1);
    let entry = &exploration.findings[0];
    assert_eq!(entry.depth, 2);
    assert_eq!(
        entry.findings,
        vec![ModelFinding::InvariantViolated {
            clause: "Small".to_owned(),
            self_object: world.reference("c1"),
            anchor: handler("increment"),
        }]
    );

    let reset = operation("reset", &[], None, frame(&["value"], false, false));
    let reset_post = "post Zero using v on M::Counter::reset { self.value <= 0 }";
    let mut world = self::world(
        vec![increment(), reset],
        &clauses(&[CAN_INC, INC, SMALL, reset_post]),
    );
    let s0 = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = self::system(&world, vec![s0], &["c1", "c2"], UNLIMITED);
    let exploration = explore(&system);
    assert_eq!(
        stats(&exploration),
        (3, 5, 2),
        "v2 is expanded, reaching v0"
    );
}

/// FR-120-AC-5 (TC-472): `s0`'s findings carry the initialization anchor.
#[trace("TC-472", "FR-120-AC-5")]
#[test]
fn the_initial_state_is_anchored_to_its_initialization() {
    let mut world = world(
        vec![increment()],
        "invariant Zero using v on M::Counter at current { self.value > 0 }",
    );
    let s0 = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = self::system(&world, vec![s0], &["c1"], UNLIMITED);
    let findings = system
        .successors(&system.initial()[0])
        .expect("expands")
        .findings;
    assert_eq!(
        findings,
        vec![ModelFinding::InvariantViolated {
            clause: "Zero".to_owned(),
            self_object: world.reference("c1"),
            anchor: SelectedAnchor {
                kind: AnchorKind::Initialization,
                name: "start".to_owned()
            },
        }]
    );
}

fn exhausted() -> CatalogCode {
    CatalogCode::new("resource_exhausted", "insufficient-next-charge")
}

/// FR-120-AC-6 (TC-472): a zero meter budget leaves the invariant
/// undetermined, or stops the run when a precondition cannot be decided; a
/// small candidate bound stops the run.
#[trace("TC-472", "FR-120-AC-6")]
#[test]
fn an_exhausted_budget_is_undetermined_or_stops_the_run() {
    let mut world = world(vec![increment()], SMALL);
    let s0 = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = self::system(&world, vec![s0], &["c1", "c2"], ZERO);
    let exploration = explore(&system);
    assert_eq!(stats(&exploration).0, 3);
    assert_eq!(exploration.findings.len(), 3);
    for entry in &exploration.findings {
        let [ModelFinding::InvariantUndetermined {
            clause,
            self_object,
            evaluation,
            ..
        }] = &entry.findings[..]
        else {
            panic!("one undetermined invariant per state: {:?}", entry.findings)
        };
        assert_eq!(clause, "Small");
        assert_eq!(self_object, &world.reference("c1"));
        assert!(matches!(evaluation, ClauseOutcome::Incomplete(_)));
        assert_eq!(evaluation.incomplete_code(), Some(exhausted()));
    }

    let mut world = self::world(vec![increment()], CAN_INC);
    let s0 = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = self::system(&world, vec![s0], &["c1", "c2"], ZERO);
    let exploration = explore(&system);
    let Outcome::Stopped {
        cause, frontier, ..
    } = &exploration.outcome
    else {
        panic!("expected a stopped run, got {:?}", exploration.outcome)
    };
    assert_eq!(*cause, exhausted());
    assert_eq!(frontier.len(), 1);
    assert_eq!(
        exploration.outcome.category(),
        qsl_foundation::diagnostic::Category::Incomplete
    );

    let mut world = self::world(vec![increment()], "");
    let s0 = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let bounded = ModelSystem::new(request(
        &world,
        vec![s0],
        universe(&["c1", "c2"]),
        UNLIMITED,
        2,
    ))
    .expect("admits");
    let exploration = explore(&bounded);
    assert!(
        matches!(&exploration.outcome, Outcome::Stopped { cause, .. } if *cause == exhausted()),
        "{:?}",
        exploration.outcome
    );
}

fn refusal_of(failure: Option<AdmissionFailure>) -> (Code, &'static str) {
    match failure {
        Some(AdmissionFailure::Refused(record)) => (record.code, record.cause),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// FR-120-AC-7 (TC-472): the checks `ModelSystem::new` makes, in order.
#[trace("TC-472", "FR-120-AC-7")]
#[test]
fn a_system_refuses_what_is_not_an_initialization_or_not_in_its_universe() {
    let mut world = world(vec![increment()], "");
    let pre = world.snapshot(
        "pre",
        "pre",
        json!({"kind": "initialization", "name": "start"}),
        &[Obj("c1", 0, 0, None)],
    );
    let validate = world.snapshot(
        "validate",
        "current",
        json!({"kind": "handler", "name": "validate"}),
        &[Obj("c1", 0, 0, None)],
    );
    let other = world.snapshot(
        "other",
        "current",
        json!({"kind": "other", "name": "x"}),
        &[Obj("c1", 0, 0, None)],
    );
    let c3 = world.start("c3", &[Obj("c3", 0, 0, None)]);
    let ok = world.start("ok", &[Obj("c1", 0, 0, None)]);
    let new = |world: &World, initial: DocumentRef, universes| {
        ModelSystem::new(request(world, vec![initial], universes, UNLIMITED, 1000)).err()
    };
    assert_eq!(
        refusal_of(new(&world, pre, universe(&["c1", "c2"]))),
        (Code::WrongSnapshot, "wrong-observation")
    );
    assert_eq!(
        refusal_of(new(&world, validate, universe(&["c1", "c2"]))),
        (Code::WrongSnapshot, "wrong-anchor")
    );
    assert_eq!(
        refusal_of(new(&world, other, universe(&["c1", "c2"]))),
        (Code::InvalidRuntimeInput, "wrong-value-kind")
    );
    assert_eq!(
        refusal_of(new(&world, c3, universe(&["c1", "c2"]))),
        (Code::InvalidRuntimeInput, "invalid-value")
    );
    assert_eq!(
        refusal_of(new(&world, ok.clone(), universe(&["c1", "c1"]))),
        (Code::InvalidRuntimeInput, "conflicting-identity")
    );
    let unknown = vec![PopulationUniverse {
        population: key("elsewhere"),
        keys: vec!["x".to_owned()],
    }];
    assert_eq!(
        refusal_of(new(&world, ok, unknown)),
        (Code::InvalidRuntimeInput, "wrong-role-mapping")
    );
}

/// FR-120-AC-7 (TC-472): a universe population the snapshot omits is
/// incomplete, and a population without a universe is a requires-bound root.
#[trace("TC-472", "FR-120-AC-7")]
#[test]
fn an_unbounded_population_or_parameter_requires_a_bound() {
    let mut world = world(vec![increment()], "");
    let s0 = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let unbounded = ModelSystem::new(request(
        &world,
        vec![s0.clone()],
        Vec::new(),
        UNLIMITED,
        1000,
    ))
    .expect("a system with no universe admits");
    let Err(NotSimulated::RequiresBound(bound)) =
        explore_model(&unbounded, 10_000, Limits::default(), || false)
    else {
        panic!("a population with no universe is unbounded")
    };
    assert!(
        bound
            .domains
            .iter()
            .any(|(key, _)| *key == root_key(&declaration_naming(&counters()))),
        "the population root is named by the hash of its declaration identity"
    );
    let sampler = sampler();
    assert!(matches!(
        sample_model(&unbounded, 10_000, &sampler, 424_242, 0, 4),
        Err(ModelSampleError::NotSimulated(NotSimulated::RequiresBound(
            _
        )))
    ));

    let integer_parameter = operation(
        "add",
        &[("n", "ix://quire/native/Integer")],
        None,
        frame(&["value"], false, false),
    );
    let mut world = self::world(vec![integer_parameter], "");
    let s0 = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = self::system(&world, vec![s0], &["c1"], UNLIMITED);
    let Err(NotSimulated::RequiresBound(bound)) =
        explore_model(&system, 10_000, Limits::default(), || false)
    else {
        panic!("an Integer parameter is unbounded")
    };
    assert_eq!(bound.domains.len(), 1);
    let naming = json!({
        "operation": declaration_naming(&key("Counter/add")),
        "parameter": "n",
    });
    assert_eq!(
        bound
            .domains
            .iter()
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>(),
        vec![root_key(&naming)],
        "the parameter root is named by the hash of its naming object"
    );
}

fn declaration_naming(key: &DeclarationKey) -> Json {
    json!({"digest_domain": "sha256-jcs", "node": key.node, "package": key.package})
}

/// The requires-bound domain key FR-120 "Bounds" gives the root named by
/// `naming`: the SHA-256 of its JCS bytes.
fn root_key(naming: &Json) -> qsl_foundation::bound::DomainKey {
    let read = quire_canonical::read(naming.to_string().as_bytes(), u64::MAX).expect("reads");
    let digest = *quire_canonical::sha256(&read, quire_canonical::Limits::new(1 << 20))
        .expect("hashes")
        .as_bytes();
    qsl_foundation::bound::DomainKey::Node {
        node: qsl_foundation::digest::WireNodeId::from_digest(digest),
        path: Vec::new(),
    }
}

/// FR-120-AC-7 (TC-472): a universe population the snapshot omits returns
/// `Incomplete` with `incomplete_population`/`incomplete-scope`.
#[trace("TC-472", "FR-120-AC-7")]
#[test]
fn a_universe_population_the_snapshot_omits_is_incomplete() {
    let mut world = world(vec![increment()], "");
    let document = json!({
        "format": "quire.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "empty", "revision_namespace": "fixture",
            "revision": "fixture:1"},
        "observation": "current", "model": world.header,
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [],
    });
    let bytes = document.to_string().into_bytes();
    let read = quire_canonical::read(&bytes, u64::MAX).expect("JSON fixture");
    let digest = *quire_canonical::sha256(&read, quire_canonical::Limits::new(u64::MAX))
        .expect("canonical fixture")
        .as_bytes();
    world.snapshots.insert(digest, bytes);
    let omitted = DocumentRef {
        authority: "test".to_owned(),
        identity: "empty".to_owned(),
        revision_namespace: "fixture".to_owned(),
        revision: "fixture:1".to_owned(),
        digest,
    };
    let failure = ModelSystem::new(request(
        &world,
        vec![omitted],
        universe(&["c1"]),
        UNLIMITED,
        10,
    ))
    .err();
    let Some(AdmissionFailure::Incomplete(record)) = failure else {
        panic!("expected Incomplete, got {failure:?}")
    };
    assert_eq!(
        (record.code, record.cause),
        (Code::IncompletePopulation, "incomplete-scope")
    );
}

/// FR-120 "Post-states": only the (candidate, result) pairs actually
/// evaluated are counted, so an operation whose first pair is accepted stays
/// exhaustive however large its result domain is.
#[trace("TC-472", "FR-120-AC-6")]
#[test]
fn only_evaluated_result_pairs_are_counted() {
    let count = operation(
        "count",
        &[],
        Some(&ident("Count")),
        frame(&[], false, false),
    );
    let mut world = world(vec![count], "");
    let s0 = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = ModelSystem::new(request(&world, vec![s0], universe(&["c1"]), UNLIMITED, 50))
        .expect("admits");
    let exploration = explore(&system);
    assert_eq!(stats(&exploration), (1, 1, 0));
}

/// FR-120 "Post-states": every argument vector counts against the bound,
/// enabled or not, so a huge parameter domain behind a false precondition
/// stops the expansion.
#[trace("TC-472", "FR-120-AC-13")]
#[test]
fn every_argument_vector_counts_against_the_bound() {
    let set_to = operation(
        "setTo",
        &[("n", &ident("Huge"))],
        None,
        frame(&["value"], false, false),
    );
    let mut world = world(
        vec![set_to],
        "pre Match using v on M::Counter::setTo { self.value - n = 0 }",
    );
    let s0 = world.start("s0", &[Obj("c1", 2, 0, None)]);
    let system = ModelSystem::new(request(&world, vec![s0], universe(&["c1"]), UNLIMITED, 10))
        .expect("admits");
    let exploration = explore(&system);
    assert!(
        matches!(&exploration.outcome, Outcome::Stopped { cause, .. } if *cause == exhausted()),
        "{:?}",
        exploration.outcome
    );
}

/// FR-120-AC-13 (TC-472): a candidate product over 10^9 stops after at most
/// eleven candidates without being allocated.
#[trace("TC-472", "FR-120-AC-13")]
#[test]
fn a_huge_candidate_product_stops_at_the_candidate_bound() {
    let spawn = operation("spawn", &[], None, frame(&[], true, false));
    let mut world = world(vec![spawn], "");
    let s0 = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let keys: Vec<String> = (2..32).map(|n| format!("c{n}")).collect();
    let mut all = vec![PopulationUniverse {
        population: counters(),
        keys: vec!["c1".to_owned()],
    }];
    all[0].keys.extend(keys);
    let system = ModelSystem::new(request(&world, vec![s0], all, UNLIMITED, 10)).expect("admits");
    let exploration = explore_model(&system, 10_000, Limits::default(), || false).expect("bounded");
    assert!(
        matches!(&exploration.outcome, Outcome::Stopped { cause, .. } if *cause == exhausted()),
        "{:?}",
        exploration.outcome
    );
}

/// FR-120-AC-8 (TC-472): a sampled trace ends where no successor remains,
/// records the finding, and replays; the same trace without its finding, or
/// replayed under another budget, refuses.
#[trace("TC-472", "FR-120-AC-8")]
#[test]
fn a_sampled_trace_replays_and_two_equal_runs_agree() {
    let mut world = world(vec![increment()], &clauses(&[CAN_INC, INC, SMALL]));
    let s0 = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = self::system(&world, vec![s0], &["c1", "c2"], UNLIMITED);
    let sampler = sampler();
    let sampled = sample_model(&system, 10_000, &sampler, 424_242, 0, 4).expect("samples");
    assert_eq!(sampled.trace.steps.len(), 2);
    assert_eq!(
        sampled.trace.provenance.as_ref().map(|p| p.stopped),
        Some(StopReason::NoSuccessors)
    );
    assert_eq!(sampled.trace.findings.len(), 1);
    assert_eq!(sampled.trace.findings[0].depth, 2);
    assert_eq!(sampled.effects.len(), 2);

    let mut missing = sampled.trace.clone();
    missing.findings.clear();
    assert!(matches!(
        qsl_eval::simulation::replay(&system, &missing, qsl_foundation::IdentityLimits::default()),
        Err(ReplayError::FindingMismatch { step: 2 })
    ));

    let again = sample_model(&system, 10_000, &sampler, 424_242, 0, 4).expect("samples again");
    assert_eq!(sampled, again);
    assert_eq!(explore(&system), explore(&system));

    let mut world = self::world(vec![increment()], CAN_INC);
    let s0 = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let starved = self::system(&world, vec![s0.clone()], &["c1", "c2"], ZERO);
    let stopped = sample_model(&starved, 10_000, &sampler, 424_242, 0, 4).expect("samples");
    assert_eq!(stopped.trace.steps.len(), 0);
    assert_eq!(
        stopped.trace.provenance.as_ref().map(|p| p.stopped),
        Some(StopReason::Stopped(exhausted()))
    );
    let funded = self::system(&world, vec![s0], &["c1", "c2"], UNLIMITED);
    let replayed = qsl_eval::simulation::replay(
        &funded,
        &stopped.trace,
        qsl_foundation::IdentityLimits::default(),
    );
    assert!(matches!(
        replayed,
        Err(ReplayError::Stopped {
            step: 0,
            recorded: Some(_),
            replayed: None
        })
    ));
}

// ---------------------------------------------------------------------------
// TC-473: effects as data
// ---------------------------------------------------------------------------

/// The effect of the step from initial state `index` with `receiver` whose
/// post-state `pick` selects.
fn effect_of(
    system: &ModelSystem<'_>,
    index: usize,
    receiver: &ObjectReference,
    pick: impl Fn(&ModelStateView) -> bool,
) -> StepEffect {
    let initial = system.initial();
    let expansion = system.successors(&initial[index]).expect("expands");
    let receiver_text = hex_bytes(receiver.object().as_str().as_bytes());
    expansion
        .successors
        .iter()
        .filter(|(transition, _)| {
            transition
                .receiver()
                .is_some_and(|text| text.contains(&receiver_text))
        })
        .find(|(_, state)| pick(&ModelStateView(state.state().clone())))
        .and_then(|(_, state)| state.reached_by().cloned())
        .expect("a matching successor")
}

/// FR-120-AC-10 (TC-473): a query operation's result is part of the step's
/// effect, and its root is in `domains()`.
#[trace("TC-473", "FR-120-AC-10")]
#[test]
fn a_query_result_is_step_effect_data() {
    let is_zero = operation(
        "isZero",
        &[],
        Some("ix://quire/native/Boolean"),
        frame(&[], false, false),
    );
    let z = "post Z using v on M::Counter::isZero { result = (self.value <= 0) }";
    let mut world = world(vec![is_zero], z);
    let docs = [
        world.start("v0", &[Obj("c1", 0, 0, None)]),
        world.start("v1", &[Obj("c1", 1, 0, None)]),
    ];
    let system = self::system(&world, docs.to_vec(), &["c1"], UNLIMITED);
    let c1 = world.reference("c1");
    for index in 0..2 {
        let value = int_field(system.initial()[index].state(), "c1", "value");
        let effect = effect_of(&system, index, &c1, |_| true);
        let expected = if value == "0" { "true" } else { "false" };
        assert_eq!(
            effect.result.as_deref(),
            Some(format!(r#"{{"type":"boolean","value":{expected}}}"#).as_str())
        );
        assert!(
            effect.created.is_empty() && effect.deleted.is_empty() && effect.changed.is_empty()
        );
    }
    assert!(system
        .domains()
        .iter()
        .any(|(_, value_type)| matches!(value_type, quire_exact::ValueType::Boolean)));
}

/// FR-120-AC-11 (TC-473): increment, spawn and remove steps carry their
/// creations, deletions and field changes as data.
#[trace("TC-473", "FR-120-AC-11")]
#[test]
fn increment_spawn_and_remove_effects_are_typed_data() {
    let spawn = operation("spawn", &[], None, frame(&[], true, false));
    let remove = operation("remove", &[], None, frame(&[], false, true));
    let mut world = world(vec![increment(), spawn, remove], INC);
    let v0 = world.start("v0", &[Obj("c1", 0, 0, None)]);
    let w = world.start("w", &[Obj("c1", 0, 0, None), Obj("c2", 0, 0, None)]);
    let system = self::system(&world, vec![v0.clone(), w], &["c1", "c2"], UNLIMITED);
    let c1 = world.reference("c1");
    let c2 = world.reference("c2");
    let index_of = |count: usize| {
        system
            .initial()
            .iter()
            .position(|state| state.state().populations[&counters()].len() == count)
            .expect("an initial state")
    };

    let effect = effect_of(&system, index_of(1), &c1, |state| state.value("c1") == "1");
    assert!(effect.created.is_empty() && effect.deleted.is_empty());
    assert_eq!(effect.result, None);
    let [change] = &effect.changed[..] else {
        panic!("one changed field: {:?}", effect.changed)
    };
    assert_eq!(change.object, c1);
    assert_eq!(change.field, "value");
    assert_eq!(
        change.pre.as_deref(),
        Some(r#"{"type":"integer","value":"0"}"#)
    );
    assert_eq!(
        change.post.as_deref(),
        Some(r#"{"type":"integer","value":"1"}"#)
    );

    let spawned = effect_of(&system, index_of(1), &c1, |state| {
        state.objects() == ["c1", "c2"]
    });
    assert_eq!(spawned.created, vec![c2.clone()]);
    assert!(spawned.deleted.is_empty());

    let removed = effect_of(&system, index_of(2), &c1, |state| state.objects() == ["c2"]);
    assert_eq!(removed.deleted, vec![c1]);
    assert!(removed.created.is_empty());
}

/// FR-120-AC-11 (TC-473): expanding a state changes no input.
#[trace("TC-473", "FR-120-AC-11")]
#[test]
fn expansion_leaves_its_inputs_unchanged() {
    let mut world = world(vec![increment()], &clauses(&[CAN_INC, INC, SMALL]));
    let s0 = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = self::system(&world, vec![s0], &["c1", "c2"], UNLIMITED);
    let key = |system: &ModelSystem<'_>| {
        quire_canonical::to_vec(
            &system.key(&system.initial()[0]),
            quire_canonical::Limits::new(1 << 20),
        )
        .expect("encodes")
    };
    let before = key(&system);
    let first = explore(&system);
    let second = explore(&system);
    assert_eq!(first, second);
    assert_eq!(before, key(&system));
    assert_eq!(int_field(system.initial()[0].state(), "c1", "value"), "0");
}

/// FR-120-AC-12 (TC-473): the forms that read ambient state outside their
/// anchor refuse at S3, so no package reaches the simulator.
#[trace("TC-473", "FR-120-AC-12")]
#[test]
fn ambient_reads_refuse_before_a_package_exists() {
    for (clause, code, cause) in [
        (
            "function F using v(): Boolean pure { self.value < 1 }",
            "missing_declaration",
            "missing-name",
        ),
        (
            "invariant I using v on M::Counter at current { result }",
            "wrong_snapshot",
            "wrong-anchor",
        ),
        (
            "invariant I using v on M::Counter at current { pre(self.value) < 1 }",
            "wrong_snapshot",
            "forbidden-pre-read",
        ),
        (
            "function F using v(c: M::Counter): Boolean pure { reaches(c, c, next) }",
            "ill_typed",
            "operator-ineligible",
        ),
    ] {
        let Err(refusals) = try_world(vec![increment()], clause) else {
            panic!("{clause} must refuse")
        };
        let [refusal] = &refusals[..] else {
            panic!("one refusal for {clause}: {refusals:?}")
        };
        let catalog = refusal.cause.catalog_code().expect("a catalog code");
        assert_eq!((catalog.code(), catalog.cause()), (code, cause), "{clause}");
    }
}

/// FR-120-AC-9's `test/orders` dispatch fixture with `body`, its initial
/// snapshot `t1` and its universe.
struct OrdersWorld {
    package: CheckedPackage,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    reference: DocumentRef,
}

impl OrdersWorld {
    fn new(body: &str) -> Self {
        Self::over(body, false)
    }

    /// With `wobble`, `A` also declares `wobble()`, which writes `flag` under
    /// no contract: two post-states.
    fn over(body: &str, wobble: bool) -> Self {
        let (package, packages) =
            crate::state_model_family::simulation_package_over(body, false, wobble);
        let digest = *packages.keys().next().expect("one model");
        let document = json!({
            "format": "quire.state.snapshot/v1",
            "identity": {"authority": "test", "identity": "t1", "revision_namespace": "fixture",
                "revision": "fixture:1"},
            "observation": "current",
            "model": {"identity": "test/orders", "version": "1.0.0",
                "digest": format!("sha256-jcs:{}", hex(&digest))},
            "anchor": {"kind": "initialization", "name": "start"},
            "populations": [{"population": "ix://test/orders/pop", "complete": true,
                "objects": [{"key": "t1", "type": "ix://test/orders/A",
                    "fields": {"flag": {"boolean": true},
                        "next": {"reference": {"population": "ix://test/orders/pop",
                            "key": "t1"}}}}]}],
        });
        let bytes = document.to_string().into_bytes();
        let read = quire_canonical::read(&bytes, u64::MAX).expect("JSON fixture");
        let snapshot_digest =
            *quire_canonical::sha256(&read, quire_canonical::Limits::new(u64::MAX))
                .expect("canonical fixture")
                .as_bytes();
        let snapshots = BTreeMap::from([(snapshot_digest, bytes)]);
        let reference = DocumentRef {
            authority: "test".to_owned(),
            identity: "t1".to_owned(),
            revision_namespace: "fixture".to_owned(),
            revision: "fixture:1".to_owned(),
            digest: snapshot_digest,
        };
        Self {
            package,
            packages,
            snapshots,
            reference,
        }
    }

    fn system(&self) -> ModelSystem<'_> {
        ModelSystem::new(ModelSimulationRequest {
            package: &self.package,
            packages: &self.packages,
            normalization: ModelNormalizationLimits::default(),
            snapshots: &self.snapshots,
            initial: vec![self.reference.clone()],
            universes: vec![PopulationUniverse {
                population: DeclarationKey {
                    package: "test/orders".to_owned(),
                    node: "ix://test/orders/pop".to_owned(),
                },
                keys: vec!["t1".to_owned()],
            }],
            observation: ObservationLimits::default(),
            expansion: ExpansionLimits {
                max_candidates: 1000,
            },
            meter: UNLIMITED,
        })
        .expect("the dispatch model admits")
    }
}

/// FR-188-AC-8 (TC-623 step 6): over `OrdersUndecided` (FR-120-AC-9's
/// fixture with `pre Low { self.ready() }` on `ready`, undefined at `t1`)
/// `actions` returns `UndecidedSuccessor{state}`, `weighted_steps`
/// `Unsteppable::UndecidedSuccessor` and `sample_probabilistic` stops
/// `SampleStop::UndecidedSuccessor`. With a further operation `wobble` whose
/// (empty) drawn vector has two post-states, the result is the same
/// undecided one, not `NotMarkov`. An undecided postcondition is the same.
#[trace("TC-623", "FR-188-AC-8")]
#[test]
fn an_undecided_contract_is_an_undecided_successor() {
    let pre = "pre Low using v on M::A::ready { self.ready() }\n";
    // The postcondition half: every candidate of `ready` is undecided.
    let post = "post LowPost using v on M::A::ready { self.ready() }\n";
    for (undecided, wobble, workload) in [
        (
            pre,
            false,
            "workload Steady on M { weight A::ready = 1; }\n",
        ),
        (
            post,
            false,
            "workload Steady on M { weight A::ready = 1; }\n",
        ),
        (
            pre,
            true,
            "workload Steady on M { weight A::ready = 1; weight A::wobble = 1; }\n",
        ),
    ] {
        let world = OrdersWorld::over(&format!("{undecided}{workload}"), wobble);
        let system = world.system();
        let state = system.initial().remove(0);
        let (_, digest) = system.state_key(&state).expect("a key");
        let expected = UndecidedSuccessor { state: digest };
        assert_eq!(
            system
                .actions(&state)
                .expect("expands")
                .expect_err("undecided"),
            expected,
            "wobble = {wobble}"
        );
        let workload = world
            .package
            .graph()
            .probabilistic_model()
            .workload("Steady")
            .expect("declared");
        let Ok(Err(Unsteppable::UndecidedSuccessor(undecided))) =
            system.weighted_steps(&state, workload)
        else {
            panic!("UndecidedSuccessor (wobble = {wobble})");
        };
        assert_eq!(undecided, expected);
        assert_eq!(
            sample_probabilistic(
                &system,
                workload,
                &state,
                &sampler(),
                7,
                0,
                3,
                &Cancel::new()
            )
            .expect_err("undecided"),
            SampleStop::UndecidedSuccessor,
            "wobble = {wobble}"
        );
    }
    // Without the undecided precondition, `wobble`'s two post-states are
    // `NotMarkov`: the undecided result above is not that.
    let world = OrdersWorld::over(
        "workload Steady on M { weight A::ready = 1; weight A::wobble = 1; }\n",
        true,
    );
    let system = world.system();
    let state = system.initial().remove(0);
    let workload = world
        .package
        .graph()
        .probabilistic_model()
        .workload("Steady")
        .expect("declared");
    let Ok(Err(Unsteppable::NotMarkov(not_markov))) = system.weighted_steps(&state, workload)
    else {
        panic!("NotMarkov without the undecided precondition");
    };
    assert_eq!(not_markov.post_states, 2);
}

/// FR-120-AC-9 (TC-472): a precondition whose dispatched query is undefined
/// at the state is a contract finding and the operation is not enabled; a
/// clause that is `false` hides it.
#[trace("TC-472", "FR-120-AC-9")]
#[test]
fn an_undecided_precondition_is_a_contract_finding() {
    let run = |body: &str| {
        let world = OrdersWorld::new(body);
        let system = world.system();
        let state = system.initial().remove(0);
        let expansion = system.successors(&state).expect("expands");
        let key_bytes =
            quire_canonical::to_vec(&system.key(&state), quire_canonical::Limits::new(1 << 20))
                .expect("a key encodes");
        let digest = qsl_foundation::digest::DigestRecord::mint(
            qsl_foundation::digest::DigestDomain::SimulationStateKeyV1,
            qsl_foundation::ByteDigest::of(&key_bytes)
                .as_bytes()
                .to_owned(),
        );
        (expansion.successors.len(), expansion.findings, digest)
    };

    let (successors, findings, _) = run("pre Low using v on M::A::ready { self.ready() }");
    assert_eq!(successors, 0, "an undecided precondition enables nothing");
    let [ModelFinding::ContractUndetermined {
        candidate,
        clause,
        evaluation,
        ..
    }] = &findings[..]
    else {
        panic!("one contract finding: {findings:?}")
    };
    assert_eq!(clause, "Low");
    assert_eq!(*candidate, None);
    assert_eq!(
        *evaluation,
        ClauseOutcome::FamilyUndefined(
            qsl_foundation::diagnostic::UndefinedReason::PreconditionFalse
        )
    );

    let (_, findings, _) = run(
        "pre Low using v on M::A::ready { self.ready() }\npre Never using v on M::A::ready { false }",
    );
    assert!(
        findings.is_empty(),
        "a false clause hides the undecided one: {findings:?}"
    );

    // The postcondition half: the finding names the candidate by its
    // state-key digest, and no candidate is a successor.
    let (successors, findings, digest) =
        run("post LowPost using v on M::A::ready { self.ready() }");
    assert_eq!(successors, 0);
    assert!(!findings.is_empty());
    for finding in &findings {
        let ModelFinding::ContractUndetermined {
            candidate,
            clause,
            evaluation,
            ..
        } = finding
        else {
            panic!("a contract finding: {finding:?}")
        };
        assert_eq!(clause, "LowPost");
        assert_eq!(*candidate, Some(digest));
        assert_eq!(
            *evaluation,
            ClauseOutcome::FamilyUndefined(
                qsl_foundation::diagnostic::UndefinedReason::PreconditionFalse
            )
        );
    }
}

const CHILD: &str = "FR120_CHILD_EXPLORATION";

/// The AC-5 package's exploration, rendered.
fn ac5_exploration() -> String {
    let mut world = world(vec![increment()], &clauses(&[CAN_INC, INC, SMALL]));
    let s0 = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = self::system(&world, vec![s0], &["c1", "c2"], UNLIMITED);
    format!("{:?}", explore(&system))
}

/// FR-120-AC-11 (TC-473): exploring twice, the second time in a child process
/// started in an empty temporary directory, gives equal explorations and
/// leaves that directory empty.
#[trace("TC-473", "FR-120-AC-11")]
#[test]
fn a_child_process_in_an_empty_directory_explores_the_same() {
    if std::env::var_os(CHILD).is_some() {
        println!("EXPLORATION={}", ac5_exploration());
        return;
    }
    let mine = ac5_exploration();
    let directory = std::env::temp_dir().join(format!("fr120-child-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("a temporary directory");
    let output = std::process::Command::new(std::env::current_exe().expect("the test binary"))
        .args([
            "--exact",
            "model_simulation::a_child_process_in_an_empty_directory_explores_the_same",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD, "1")
        .current_dir(&directory)
        .output()
        .expect("the child runs");
    let untouched = std::fs::read_dir(&directory).expect("readable").count() == 0;
    std::fs::remove_dir(&directory).expect("an empty directory is removed");
    assert!(output.status.success(), "{output:?}");
    assert!(untouched, "the child wrote into its directory");
    let stdout = String::from_utf8(output.stdout).expect("UTF-8");
    let theirs = stdout
        .lines()
        .find_map(|line| line.split_once("EXPLORATION=").map(|(_, rest)| rest))
        .expect("the child printed its exploration");
    assert_eq!(theirs, mine);
}

/// FR-120 "Post-states": a field no frame grants and whose type has no finite
/// domain (here `tag: Integer?` and `id: Integer`) neither stops an expansion nor becomes a
/// requires-bound root.
#[trace("TC-471", "FR-120-AC-1")]
#[test]
fn an_ungranted_unbounded_field_is_neither_probed_nor_a_root() {
    let mut world = world_with_unbounded_fields(vec![increment()]);
    let s0 = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = self::system(&world, vec![s0], &["c1"], UNLIMITED);
    assert_eq!(successors_of(&system, 0).len(), 3);
    assert!(system
        .domains()
        .iter()
        .all(|(_, value_type)| !matches!(value_type, quire_exact::ValueType::Integer)));
    assert_eq!(stats(&explore(&system)).0, 3);
}
