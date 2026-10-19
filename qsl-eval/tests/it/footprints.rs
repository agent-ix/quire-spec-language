// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-155 (TC-574, TC-575, TC-589): the read, write and enabling footprints
//! of model transitions, derived from checked clauses and frames, decided
//! independent or dependent, and enforced against what a step reads.

use std::collections::{BTreeMap, BTreeSet};

use ix_trace_rs::trace;
use qsl_eval::simulation::model::{
    ExpansionLimits, ModelSimState, ModelSimulationRequest, ModelSystem, PopulationUniverse,
    WithinStop,
};
use qsl_eval::simulation::{independent, Footprint, Location, TransitionSystem};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{Observation, PackageDeclarations};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::footprint::{
    derive_footprint, FootprintObject, SymbolicFootprint, SymbolicLocation,
};
use qsl_semantics::model::intake::{admit_unit, meaning, package_input};
use qsl_semantics::model::key::{hex, DeclarationKey};
use qsl_semantics::model::observation::{
    declared_type_universe_for, DocumentRef, ObservationLimits,
};
use qsl_semantics::model::operation::OperationLookup;
use qsl_semantics::model::population::{
    decide_frame, FrameCharge, FrameDecision, FrameObject, FrameStop,
};
use qsl_semantics::model::refusal::ModelRefusalCause;
use qsl_semantics::model::state::StateObject;
use quire_exact::{ObjectId, ObjectReference, ScalarLimits, Value};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::model_transition::ModelTransition;
use serde_json::{json, Value as Json};

const PACKAGE: &str = "test/footprints";
const PLACEHOLDER: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

fn ident(suffix: &str) -> String {
    format!("ix://{PACKAGE}/{suffix}")
}

fn key(suffix: &str) -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE.to_owned(),
        node: ident(suffix),
    }
}

// ---------------------------------------------------------------------------
// The fixture domain packages
// ---------------------------------------------------------------------------

fn origin(identity: &str) -> Json {
    json!({"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0",
        "inputIdentities": [identity]}})
}

fn one() -> Json {
    json!({"lower": 1, "upper": 1, "ordered": false, "unique": true})
}

/// An object type with its fields and operations.
fn object_type(name: &str, fields: Vec<Json>, operations: Vec<Json>) -> Json {
    object_type_extending(name, &[], fields, operations)
}

/// An object type extending `supertypes`.
fn object_type_extending(
    name: &str,
    supertypes: &[&str],
    fields: Vec<Json>,
    operations: Vec<Json>,
) -> Json {
    let identity = ident(name);
    let supertypes: Vec<String> = supertypes.iter().map(|name| ident(name)).collect();
    json!({
        "identity": identity, "displayName": identity,
        "kind": {"module": PACKAGE, "name": "object_type"}, "roles": [],
        "origin": origin(&identity), "constraints": [], "extensions": [],
        "unknownPolicy": "reject",
        "supertypes": supertypes, "fields": fields, "operations": operations, "relationships": [],
    })
}

/// A field of `owner`: `presence` is `required` or `optional`.
fn field(owner: &str, name: &str, type_ref: &str, presence: &str) -> Json {
    let identity = ident(&format!("{owner}/{name}"));
    json!({
        "identity": identity, "name": name, "typeRef": type_ref, "presence": presence,
        "nullable": false, "defaultKind": "none", "multiplicity": one(),
        "origin": origin(&identity),
    })
}

/// A bounded integer type `0..=upper`.
fn level(name: &str, upper: u64) -> Json {
    let identity = ident(name);
    let constraint = |keyword: &str, value: String| {
        json!({
            "identity": format!("{identity}/constraints/{keyword}"), "keyword": keyword,
            "operands": {"value": value}, "appliesTo": "ix://quire/native/Integer",
            "diagnosticCode": format!("bound.{keyword}"), "origin": origin(&identity),
        })
    };
    json!({
        "identity": identity, "displayName": identity,
        "kind": {"module": PACKAGE, "name": "value_type"}, "roles": [],
        "origin": origin(&identity), "extensions": [], "unknownPolicy": "reject",
        "scalar": "integer",
        "constraints": [constraint("min", "0".into()), constraint("max", upper.to_string())],
    })
}

/// An operation of `owner` over `(name, type)` parameters, with `frame`.
fn operation(
    owner: &str,
    name: &str,
    params: &[(&str, &str)],
    returns: Option<&str>,
    frame: &Json,
) -> Json {
    let identity = ident(&format!("{owner}/{name}"));
    let params: Vec<Json> = params
        .iter()
        .map(|(parameter, type_ref)| {
            let parameter_identity = format!("{identity}/{parameter}");
            json!({
                "identity": parameter_identity, "name": parameter, "typeRef": type_ref,
                "presence": "required", "nullable": false, "defaultKind": "none",
                "multiplicity": one(), "origin": origin(&parameter_identity),
            })
        })
        .collect();
    let mut node = json!({
        "identity": identity, "name": name, "params": params, "pre": [], "post": [],
        "origin": {"source": {"sourceIdentity": ident("spec"), "path": "spec.qspec",
            "startLine": 1, "startColumn": 1}},
        "frame": frame,
    });
    if let Some(type_ref) = returns {
        node["returns"] = json!({"typeRef": type_ref, "nullable": false, "multiplicity": one()});
    }
    node
}

/// A frame: the type-wide `modifies`, `creates` and `deletes` entries, and
/// the receiver-scoped members (`modifies_receiver`, `deletes_receiver`).
fn frame(members: &Json) -> Json {
    let mut frame = json!({"modifies": [], "creates": [], "deletes": []});
    for (name, value) in members.as_object().expect("frame members") {
        frame[name] = value.clone();
    }
    frame
}

fn document(types: Vec<Json>, populations: &[(&str, &str)]) -> Vec<u8> {
    let constructs: Vec<Json> = [
        ("object_type", meaning::OBJECT_TYPE),
        ("population", meaning::POPULATION),
        ("value_type", meaning::VALUE_TYPE),
    ]
    .into_iter()
    .filter(|(name, _)| {
        *name == "population" || types.iter().any(|node| node["kind"]["name"] == *name)
    })
    .map(|(name, meaning)| {
        json!({"kind": {"module": PACKAGE, "name": name}, "moduleVersion": "1.0.0",
            "manifestDigest": PLACEHOLDER,
            "construct": {"identity": "none", "shape": "record", "members": {},
                "meaning": meaning}})
    })
    .collect();
    let populations: Vec<Json> = populations
        .iter()
        .map(|(name, member)| {
            let identity = ident(name);
            json!({
                "identity": identity, "displayName": identity,
                "kind": {"module": PACKAGE, "name": "population"},
                "members": [ident(member)], "extent": "closed", "origin": origin(&identity),
            })
        })
        .collect();
    json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1", "digest": "sha256-jcs"},
        "source": {"identity": ident("spec"), "version": "1.0.0", "dialect": "spec-bundle",
            "digest": PLACEHOLDER},
        "package": {"identity": PACKAGE, "version": "1.0.0", "manifestDigest": PLACEHOLDER,
            "mappingVersions": [], "profileVersions": [], "lockDigest": PLACEHOLDER},
        "occurrences": [], "extensions": [],
        "constructs": constructs, "types": types, "populations": populations,
    })
    .to_string()
    .into_bytes()
}

// ---------------------------------------------------------------------------
// A checked package over a fixture
// ---------------------------------------------------------------------------

struct Subject {
    package: CheckedPackage,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
    header: Json,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
}

/// The fixture `document` checked with `clauses` as its unit.
fn subject(document: Vec<u8>, clauses: &[&str]) -> Subject {
    let packages = package_input([document.as_slice()]);
    let digest = *packages.keys().next().expect("one model");
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n\
         model M = \"{PACKAGE}\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         function Noop using v(): Boolean pure {{ true }}\n{}\n",
        hex(&digest),
        clauses.join("\n"),
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "fr-155", "fixture", "1"),
        "fr-155.native",
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
    let graph = declarations
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}"));
    Subject {
        package: CheckedPackage::link(graph),
        header: json!({"identity": PACKAGE, "version": "1.0.0",
            "digest": format!("sha256-jcs:{}", hex(&digest))}),
        packages,
        snapshots: BTreeMap::new(),
    }
}

/// One object of a snapshot: its population, key, type and field values.
struct Obj {
    population: &'static str,
    key: &'static str,
    object_type: &'static str,
    fields: Json,
}

fn integer(value: i64) -> Json {
    json!({"integer": value.to_string()})
}

fn boolean(value: bool) -> Json {
    json!({"boolean": value})
}

fn reference(population: &str, key: &str) -> Json {
    json!({"reference": {"population": ident(population), "key": key}})
}

fn present(value: &Json) -> Json {
    json!({"present": value})
}

fn absent() -> Json {
    json!({"absent": {}})
}

impl Subject {
    /// The initial snapshot `name`, anchored `{initialization, start}`.
    fn start(&mut self, name: &str, populations: &[&str], objects: &[Obj]) -> DocumentRef {
        let listed: Vec<Json> = populations
            .iter()
            .map(|population| {
                let members: Vec<Json> = objects
                    .iter()
                    .filter(|object| object.population == *population)
                    .map(|object| {
                        json!({"key": object.key, "type": ident(object.object_type),
                            "fields": object.fields})
                    })
                    .collect();
                json!({"population": ident(population), "complete": true, "objects": members})
            })
            .collect();
        let document = json!({
            "format": "quire.state.snapshot/v1",
            "identity": {"authority": "test", "identity": name, "revision_namespace": "fixture",
                "revision": "fixture:1"},
            "observation": "current", "model": self.header,
            "anchor": {"kind": "initialization", "name": "start"},
            "populations": listed,
        });
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

    fn model(&self) -> qsl_semantics::model::state::StateModel {
        qsl_semantics::model::state::StateModel::new(
            self.package.graph().model_selections(),
            self.package.graph().scope().types(),
            &self.packages,
            ModelNormalizationLimits::default(),
        )
        .expect("model views")
    }

    /// The reference to `object` of `object_type`, derived from the
    /// package's own universe rule rather than from the simulator.
    fn reference(&self, object_type: &str, object: &str) -> ObjectReference {
        let (effective, universe) = declared_type_universe_for(
            self.package.graph().model_selections(),
            &self.packages,
            ModelNormalizationLimits::default(),
            &key(object_type),
        )
        .expect("universe derives")
        .expect("the type is declared");
        ObjectReference::new(universe, effective, ObjectId::new(object).expect("an id"))
    }

    fn system<'s>(
        &'s self,
        initial: DocumentRef,
        universes: &[(&str, &[&str])],
    ) -> ModelSystem<'s> {
        ModelSystem::new(ModelSimulationRequest {
            package: &self.package,
            packages: &self.packages,
            normalization: ModelNormalizationLimits::default(),
            snapshots: &self.snapshots,
            initial: vec![initial],
            universes: universes
                .iter()
                .map(|(population, keys)| PopulationUniverse {
                    population: key(population),
                    keys: keys.iter().map(|key| (*key).to_owned()).collect(),
                })
                .collect(),
            observation: ObservationLimits::default(),
            expansion: ExpansionLimits {
                max_candidates: 1_000_000,
            },
            meter: UNLIMITED,
        })
        .expect("the system admits")
    }

    /// The symbolic footprint of `operation` on `object_type`.
    fn symbolic(&self, object_type: &str, operation: &str) -> SymbolicFootprint {
        let graph = self.package.graph();
        let model = self.model();
        let identity = model
            .type_identity(&key(object_type))
            .expect("the type is declared");
        let OperationLookup::Declared {
            declaring,
            operation,
        } = graph
            .scope()
            .operations()
            .resolve(graph.scope().types(), identity, operation)
        else {
            panic!("{object_type} declares no operation {operation}");
        };
        derive_footprint(graph, &model, identity, declaring, operation)
            .expect("a footprint derives")
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

// ---------------------------------------------------------------------------
// Transitions and their canonical texts
// ---------------------------------------------------------------------------

/// A reference's canonical text, as a transition names a receiver or a
/// reference argument.
fn reference_text(reference: &ObjectReference) -> String {
    format!(
        r#"{{"identity":"{}","object_type":"{}","type":"reference","universe":"{}"}}"#,
        reference
            .object()
            .as_str()
            .bytes()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        hex(reference.object_type().as_bytes()),
        hex(reference.universe().as_bytes()),
    )
}

/// The transition of `object_type`'s `operation` on `receiver` with the
/// canonical texts `arguments`.
fn transition(
    object_type: &str,
    operation: &str,
    receiver: &ObjectReference,
    arguments: Vec<String>,
) -> ModelTransition {
    let operation = format!(
        r#"{{"digest_domain":"sha256-jcs","node":"{}","package":"{PACKAGE}"}}"#,
        ident(&format!("{object_type}/{operation}"))
    );
    ModelTransition::new(
        operation,
        Some(reference_text(receiver)),
        arguments,
        quire_canonical::Limits::new(1 << 20),
    )
    .expect("a transition")
}

// ---------------------------------------------------------------------------
// Comparing footprints
// ---------------------------------------------------------------------------

fn field_of(object: &FootprintObject, name: &str) -> SymbolicLocation {
    SymbolicLocation::Field {
        object: object.clone(),
        field: key(name),
    }
}

fn symbolic(locations: impl IntoIterator<Item = SymbolicLocation>) -> BTreeSet<SymbolicLocation> {
    locations.into_iter().collect()
}

fn any_field(name: &str) -> SymbolicLocation {
    SymbolicLocation::AnyField { field: key(name) }
}

fn membership(population: &str, object: &FootprintObject) -> SymbolicLocation {
    SymbolicLocation::Membership {
        population: key(population),
        object: object.clone(),
    }
}

fn any_membership(population: &str) -> SymbolicLocation {
    SymbolicLocation::AnyMembership {
        population: key(population),
    }
}

fn field_at(object: &ObjectReference, name: &str) -> Location {
    Location::Field {
        object: object.clone(),
        field: key(name),
    }
}

fn member_of(population: &str, object: &ObjectReference) -> Location {
    Location::Membership {
        population: key(population),
        object: object.clone(),
    }
}

fn locations(locations: impl IntoIterator<Item = Location>) -> BTreeSet<Location> {
    locations.into_iter().collect()
}

// ---------------------------------------------------------------------------
// TC-574: the Counter subject
// ---------------------------------------------------------------------------

const COUNTER_CLAUSES: [&str; 6] = [
    "pre BumpLow using v on M::Counter::bump { self.v < 2 }",
    "post BumpUp using v on M::Counter::bump { self.v - pre(self.v) = 1 }",
    "pre BumpAllLow using v on M::Counter::bumpAll { self.v < 2 }",
    "post BumpAllUp using v on M::Counter::bumpAll { self.v - pre(self.v) = 1 }",
    "post CopyPeer using v on M::Counter::copy { self.v = self.peer.v }",
    "pre GiveLow using v on M::Counter::give { dest.v < 2 }",
];

fn counters_subject() -> Subject {
    let counter = ident("Counter");
    let operations = vec![
        operation(
            "Counter",
            "bump",
            &[],
            None,
            &frame(&json!({"modifies_receiver": [ident("Counter/v")]})),
        ),
        operation(
            "Counter",
            "bumpAll",
            &[],
            None,
            &frame(&json!({"modifies": [ident("Counter/v")]})),
        ),
        operation(
            "Counter",
            "copy",
            &[],
            None,
            &frame(&json!({"modifies_receiver": [ident("Counter/v")]})),
        ),
        operation(
            "Counter",
            "give",
            &[("dest", &counter)],
            None,
            &frame(&json!({"modifies_receiver": [ident("Counter/v")]})),
        ),
        operation(
            "Counter",
            "spawn",
            &[],
            None,
            &frame(&json!({"creates": [counter]})),
        ),
    ];
    subject(
        document(
            vec![
                level("Level", 2),
                object_type(
                    "Counter",
                    vec![
                        field("Counter", "v", &ident("Level"), "required"),
                        field("Counter", "peer", &counter, "required"),
                    ],
                    operations,
                ),
            ],
            &[("counters", "Counter")],
        ),
        &COUNTER_CLAUSES,
    )
}

/// Counters `a`, `b` and `c` at zero, each pointing at the next.
fn counters_start(subject: &mut Subject) -> DocumentRef {
    let counter = |key, peer| Obj {
        population: "counters",
        key,
        object_type: "Counter",
        fields: json!({"v": integer(0), "peer": reference("counters", peer)}),
    };
    subject.start(
        "s0",
        &["counters"],
        &[counter("a", "b"), counter("b", "c"), counter("c", "a")],
    )
}

const COUNTER_KEYS: [(&str, &[&str]); 1] = [("counters", &["a", "b", "c", "d"])];

/// Footprints of `operation` on `receiver` with `arguments`, at the initial
/// state of `system`.
fn footprint_of(
    system: &ModelSystem<'_>,
    operation: &str,
    receiver: &ObjectReference,
    arguments: &[&ObjectReference],
) -> Footprint {
    let state = &system.initial()[0];
    let arguments = arguments
        .iter()
        .map(|argument| reference_text(argument))
        .collect();
    system
        .footprint(
            state,
            &transition("Counter", operation, receiver, arguments),
        )
        .expect("a footprint instantiates")
}

/// FR-155-AC-1: `bump` reads and writes only its receiver's `v` and reads the
/// receiver's membership, instantiated per receiver; two receivers are
/// independent and one receiver is not independent of itself.
#[trace("TC-574", "FR-155-AC-1")]
#[test]
fn a_receiver_scoped_bump_touches_only_its_own_counter() {
    let mut subject = counters_subject();
    let start = counters_start(&mut subject);
    let system = subject.system(start, &COUNTER_KEYS);

    let symbolic_bump = subject.symbolic("Counter", "bump");
    let receiver = FootprintObject::Receiver;
    assert_eq!(
        symbolic_bump.reads,
        symbolic([
            field_of(&receiver, "Counter/v"),
            membership("counters", &receiver)
        ])
    );
    assert_eq!(
        symbolic_bump.writes,
        symbolic([field_of(&receiver, "Counter/v")])
    );
    assert_eq!(symbolic_bump.enabling, symbolic_bump.reads);

    let (a, b) = (
        subject.reference("Counter", "a"),
        subject.reference("Counter", "b"),
    );
    let bump_a = footprint_of(&system, "bump", &a, &[]);
    let bump_b = footprint_of(&system, "bump", &b, &[]);
    assert_eq!(
        bump_a.reads,
        locations([field_at(&a, "Counter/v"), member_of("counters", &a)])
    );
    assert_eq!(bump_a.writes, locations([field_at(&a, "Counter/v")]));
    assert_eq!(bump_a.enabling, bump_a.reads);
    assert!(independent(&bump_a, &bump_b));
    assert!(!independent(&bump_a, &bump_a));
}

/// An every-object frame writes the field on any object; a
/// post clause reading a navigated field reads the reference and the field
/// on any object; a reference parameter's field and membership are in the
/// enabling footprint; a creation writes the population's membership. The
/// two quantification clauses of FR-155-AC-2 are not covered, because no state
/// clause can name a population, so the test claims no criterion.
#[trace("TC-574")]
#[test]
fn every_object_frames_navigation_parameters_and_creation_widen_the_footprint() {
    let mut subject = counters_subject();
    let start = counters_start(&mut subject);
    let system = subject.system(start, &COUNTER_KEYS);
    let receiver = FootprintObject::Receiver;

    let bump_all = subject.symbolic("Counter", "bumpAll");
    assert_eq!(bump_all.writes, symbolic([any_field("Counter/v")]));

    let copy = subject.symbolic("Counter", "copy");
    assert_eq!(
        copy.reads,
        symbolic([
            field_of(&receiver, "Counter/peer"),
            any_field("Counter/v"),
            membership("counters", &receiver),
        ])
    );

    let target = FootprintObject::Param("dest".to_owned());
    let give = subject.symbolic("Counter", "give");
    assert_eq!(
        give.enabling,
        symbolic([
            field_of(&target, "Counter/v"),
            membership("counters", &receiver),
            membership("counters", &target),
        ])
    );

    let spawn = subject.symbolic("Counter", "spawn");
    assert_eq!(spawn.writes, symbolic([any_membership("counters")]));

    let (a, b, c) = (
        subject.reference("Counter", "a"),
        subject.reference("Counter", "b"),
        subject.reference("Counter", "c"),
    );
    let bump = |receiver| footprint_of(&system, "bump", receiver, &[]);
    let bump_all_a = footprint_of(&system, "bumpAll", &a, &[]);
    let bump_all_b = footprint_of(&system, "bumpAll", &b, &[]);
    assert!(!independent(&bump_all_a, &bump_all_b));

    let copy_a = footprint_of(&system, "copy", &a, &[]);
    for other in [&a, &b, &c] {
        assert!(!independent(&copy_a, &bump(other)));
    }

    let give_ab = footprint_of(&system, "give", &a, &[&b]);
    assert_eq!(
        give_ab.enabling,
        locations([
            field_at(&b, "Counter/v"),
            member_of("counters", &a),
            member_of("counters", &b),
        ])
    );
    assert!(!independent(&give_ab, &bump(&b)));
    assert!(independent(&give_ab, &bump(&c)));

    let spawn_a = footprint_of(&system, "spawn", &a, &[]);
    for other in [&a, &b, &c] {
        assert!(!independent(&spawn_a, &bump(other)));
    }
}

// ---------------------------------------------------------------------------
// TC-575: enforcement
// ---------------------------------------------------------------------------

/// FR-155-AC-3: a step whose clause reads `Field{a, v}`, held to a footprint
/// without it, stops the expansion naming the transition and the location;
/// held to the derived footprint it expands as an unenforced expansion does.
#[trace("TC-575", "FR-155-AC-3")]
#[test]
fn a_read_outside_the_footprint_stops_the_expansion_naming_it() {
    let mut subject = counters_subject();
    let start = counters_start(&mut subject);
    let system = subject.system(start, &COUNTER_KEYS);
    let state = &system.initial()[0];
    let a = subject.reference("Counter", "a");
    let bump_a = transition("Counter", "bump", &a, Vec::new());

    let unenforced = system.successors(state).expect("the state expands");
    let derived = system
        .successors_within(state, &|_| BTreeSet::new())
        .expect("the derived footprint admits every read");
    let transitions = |successors: &[(ModelTransition, ModelSimState)]| {
        successors
            .iter()
            .map(|(transition, _)| transition.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        transitions(&derived.successors),
        transitions(&unenforced.successors)
    );

    let narrowed = |transition: &ModelTransition| {
        if *transition == bump_a {
            locations([field_at(&a, "Counter/v")])
        } else {
            BTreeSet::new()
        }
    };
    let Err(WithinStop::Violation(violation)) = system.successors_within(state, &narrowed) else {
        panic!("the narrowed footprint must stop the expansion");
    };
    assert_eq!(violation.transition, bump_a);
    assert_eq!(violation.location, field_at(&a, "Counter/v"));
    assert_eq!(violation.observation, Observation::Pre);
    assert_eq!(violation.cause().code(), "runtime_invariant");
    assert_eq!(violation.cause().cause(), "established-invariant-broken");
}

/// FR-155-AC-3, post clauses: `copy(a)`'s postcondition reads the peer's `v`
/// in the post-state; held to a footprint without the field on any object,
/// the step stops naming that read.
#[trace("TC-575", "FR-155-AC-3")]
#[test]
fn a_post_state_read_outside_the_footprint_stops_the_expansion() {
    let mut subject = counters_subject();
    let start = counters_start(&mut subject);
    let system = subject.system(start, &COUNTER_KEYS);
    let state = &system.initial()[0];
    let (a, b) = (
        subject.reference("Counter", "a"),
        subject.reference("Counter", "b"),
    );
    let copy_a = transition("Counter", "copy", &a, Vec::new());
    let narrowed = |transition: &ModelTransition| {
        if *transition == copy_a {
            locations([Location::AnyField {
                field: key("Counter/v"),
            }])
        } else {
            BTreeSet::new()
        }
    };
    let Err(WithinStop::Violation(violation)) = system.successors_within(state, &narrowed) else {
        panic!("the narrowed footprint must stop the expansion");
    };
    assert_eq!(violation.transition, copy_a);
    assert_eq!(violation.location, field_at(&b, "Counter/v"));
    assert_eq!(violation.observation, Observation::Post);
    assert_eq!(violation.cause().code(), "runtime_invariant");
}

fn frame_objects(objects: &BTreeMap<String, StateObject>) -> Vec<FrameObject<'_, Value>> {
    objects
        .iter()
        .map(|(key, object)| FrameObject {
            object: key.as_str(),
            type_identity: object.type_identity.clone(),
            fields: object.fields.clone(),
        })
        .collect()
}

/// FR-155-AC-4: every step the enforced expansion takes from the initial
/// state is admitted by the frame decision with its receiver, and a
/// successor of `bump(a)` that also changes `b.v` is a frame violation.
/// Footprints of one package and transition are equal on every derivation.
#[trace("TC-575", "FR-155-AC-4")]
#[test]
fn enforced_steps_pass_the_frame_decision_and_footprints_are_functions_of_the_package() {
    let mut subject = counters_subject();
    let start = counters_start(&mut subject);
    let system = subject.system(start.clone(), &COUNTER_KEYS);
    let a = subject.reference("Counter", "a");

    let initial = system.initial();

    // Every step of every reachable state, with no free key to create under,
    // is admitted by the frame decision and by its footprint.
    let closed = subject.system(start, &[("counters", &["a", "b", "c"])]);
    let state_bytes = |state: &ModelSimState| {
        quire_canonical::to_vec(&closed.key(state), quire_canonical::Limits::new(1 << 20))
            .expect("a state key encodes")
    };
    let mut seen = BTreeSet::new();
    let mut pending = closed.initial();
    let mut steps = 0_usize;
    while let Some(state) = pending.pop() {
        if !seen.insert(state_bytes(&state)) {
            continue;
        }
        let expansion = closed
            .successors_within(&state, &|_| BTreeSet::new())
            .expect("every step is admitted");
        steps += expansion.successors.len();
        pending.extend(expansion.successors.into_iter().map(|(_, state)| state));
    }
    assert_eq!(seen.len(), 27);
    assert!(steps > 0);

    let graph = subject.package.graph();
    let model = subject.model();
    let identity = model
        .type_identity(&key("Counter"))
        .expect("Counter is declared");
    let OperationLookup::Declared { operation, .. } =
        graph
            .scope()
            .operations()
            .resolve(graph.scope().types(), identity, "bump")
    else {
        panic!("Counter declares bump");
    };
    let before = &initial[0].state().populations[&key("counters")];
    let after = |changed: &[&str]| {
        let mut after = before.clone();
        for object in changed {
            after.get_mut(*object).expect("an object").fields.insert(
                key("Counter/v"),
                Value::Integer(quire_exact::Integer::from(1_i64)),
            );
        }
        after
    };
    let decide = |post: &BTreeMap<String, StateObject>| {
        decide_frame(
            &FrameDecision {
                index: model
                    .population_index(&key("counters"))
                    .expect("the population's index"),
                ancestor_steps: model.ancestor_steps(),
                effect: operation.effect(),
                declared_created: &[],
                declared_deleted: &[],
                receiver: Some("a"),
                arguments: &[],
            },
            &frame_objects(before),
            &frame_objects(post),
            |_, left, right| match (left, right) {
                (Some(Value::Integer(left)), Some(Value::Integer(right))) => left == right,
                (Some(Value::Reference(left)), Some(Value::Reference(right))) => left == right,
                (None, None) => true,
                _ => false,
            },
            &mut |_: FrameCharge| Ok::<(), std::convert::Infallible>(()),
        )
        .map(|_| ())
        .map_err(|stop| match stop {
            FrameStop::Refused(refusal) => Box::new(refusal.cause),
            FrameStop::Denied(never) => match never {},
        })
    };
    assert!(decide(&after(&["a"])).is_ok());
    let Err(cause) = decide(&after(&["a", "b"])) else {
        panic!("a change to b.v is outside bump's frame");
    };
    assert_eq!(
        *cause,
        ModelRefusalCause::FrameFieldWriteOutsideGrant {
            object: "b".to_owned(),
            field: key("Counter/v"),
        }
    );

    // Two derivations from independently checked packages agree.
    let mut other = counters_subject();
    let other_start = counters_start(&mut other);
    let other_system = other.system(other_start, &COUNTER_KEYS);
    for operation in ["bump", "bumpAll", "copy", "give", "spawn"] {
        assert_eq!(
            subject.symbolic("Counter", operation),
            other.symbolic("Counter", operation)
        );
    }
    assert_eq!(
        footprint_of(&system, "bump", &a, &[]),
        footprint_of(&other_system, "bump", &other.reference("Counter", "a"), &[])
    );
}

// ---------------------------------------------------------------------------
// TC-589: complete enabling footprints
// ---------------------------------------------------------------------------

const BOOLEAN: &str = "ix://quire/native/Boolean";

/// A gate over cells: a guard written in a postcondition, a navigation
/// through a rewritable reference and a `result` that ranges over the
/// post-state.
fn gates_subject() -> Subject {
    let cell = ident("Cell");
    let gate_frame = |entry: &str| frame(&json!({"modifies_receiver": [ident(entry)]}));
    subject(
        document(
            vec![
                level("Bit", 1),
                object_type(
                    "Cell",
                    vec![field("Cell", "v", &ident("Bit"), "required")],
                    vec![operation(
                        "Cell",
                        "set",
                        &[],
                        None,
                        &frame(&json!({"modifies_receiver": [ident("Cell/v")]})),
                    )],
                ),
                object_type(
                    "Gate",
                    vec![
                        field("Gate", "armed", BOOLEAN, "required"),
                        field("Gate", "alarm", BOOLEAN, "required"),
                        field("Gate", "cell", &cell, "required"),
                        field("Gate", "alt", &cell, "required"),
                    ],
                    vec![
                        operation("Gate", "disarm", &[], None, &gate_frame("Gate/armed")),
                        operation("Gate", "boom", &[], None, &gate_frame("Gate/alarm")),
                        operation("Gate", "boomNav", &[], None, &gate_frame("Gate/alarm")),
                        operation("Gate", "point", &[], None, &gate_frame("Gate/cell")),
                        operation("Gate", "pick", &[], Some(&cell), &gate_frame("Gate/alarm")),
                        operation(
                            "Gate",
                            "build",
                            &[],
                            None,
                            &frame(&json!({"creates": [ident("Gate")]})),
                        ),
                    ],
                ),
            ],
            &[("cells", "Cell"), ("gates", "Gate")],
        ),
        &[
            "post Disarmed using v on M::Gate::disarm { not self.armed }",
            "pre BoomArmed using v on M::Gate::boom { self.armed }",
            "post BoomRaised using v on M::Gate::boom { self.alarm and pre(self.cell.v) = 1 }",
            "pre NavArmed using v on M::Gate::boomNav { self.armed and self.cell.v = 1 }",
            "post NavRaised using v on M::Gate::boomNav { self.alarm }",
            "pre PointMoves using v on M::Gate::point { self.cell != self.alt }",
            "post PointTo using v on M::Gate::point { self.cell = pre(self.alt) }",
            "pre PickArmed using v on M::Gate::pick { self.armed }",
            "post PickRaised using v on M::Gate::pick { self.alarm and result.v = 1 }",
            "pre SetZero using v on M::Cell::set { self.v = 0 }",
            "post SetOne using v on M::Cell::set { self.v = 1 }",
        ],
    )
}

fn gates_start(subject: &mut Subject) -> DocumentRef {
    let cell = |key, v| Obj {
        population: "cells",
        key,
        object_type: "Cell",
        fields: json!({"v": integer(v)}),
    };
    subject.start(
        "s0",
        &["cells", "gates"],
        &[
            cell("a", 0),
            cell("b", 1),
            Obj {
                population: "gates",
                key: "g",
                object_type: "Gate",
                fields: json!({"armed": boolean(true), "alarm": boolean(false),
                    "cell": reference("cells", "a"), "alt": reference("cells", "b")}),
            },
        ],
    )
}

/// The footprint of `object_type`'s `operation` on `receiver` at the initial
/// state of `system`.
fn footprint_on(
    system: &ModelSystem<'_>,
    object_type: &str,
    operation: &str,
    receiver: &ObjectReference,
) -> Footprint {
    system
        .footprint(
            &system.initial()[0],
            &transition(object_type, operation, receiver, Vec::new()),
        )
        .expect("a footprint instantiates")
}

/// FR-155-AC-5, guard-in-postcondition and navigation subjects: `boom`
/// enables on the cell field its postcondition reads in the pre-state, and a
/// navigated read includes the intermediate reference.
#[trace("TC-589", "FR-155-AC-5")]
#[test]
fn a_guard_in_a_postcondition_and_a_navigated_reference_are_in_the_enabling_footprint() {
    let mut subject = gates_subject();
    let start = gates_start(&mut subject);
    let system = subject.system(start, &[("cells", &["a", "b"]), ("gates", &["g"])]);
    let receiver = FootprintObject::Receiver;

    let boom = subject.symbolic("Gate", "boom");
    assert_eq!(
        boom.enabling,
        symbolic([
            field_of(&receiver, "Gate/armed"),
            field_of(&receiver, "Gate/cell"),
            any_field("Cell/v"),
            membership("gates", &receiver),
        ])
    );
    let navigated = subject.symbolic("Gate", "boomNav");
    assert!(navigated.reads.contains(&field_of(&receiver, "Gate/cell")));

    let (a, g) = (
        subject.reference("Cell", "a"),
        subject.reference("Gate", "g"),
    );
    let set_a = footprint_on(&system, "Cell", "set", &a);
    let boom_g = footprint_on(&system, "Gate", "boom", &g);
    assert!(!independent(&set_a, &boom_g));
    assert!(set_a
        .writes
        .iter()
        .any(|written| boom_g.enabling.iter().any(|read| written.meets(read))));

    let point_g = footprint_on(&system, "Gate", "point", &g);
    let boom_nav_g = footprint_on(&system, "Gate", "boomNav", &g);
    assert!(boom_nav_g.reads.contains(&field_at(&g, "Gate/cell")));
    assert!(point_g.writes.contains(&field_at(&g, "Gate/cell")));
    assert!(!independent(&point_g, &boom_nav_g));
}

/// A postcondition reading through `result` reads the field on any object and
/// the membership of the result's population. This is the footprint half of
/// FR-155-AC-6; its reduced run needs a partial-order engine the repository
/// does not have, so the test claims no criterion.
#[trace("TC-589")]
#[test]
fn a_postcondition_reading_through_result_reads_the_result_populations() {
    let mut subject = gates_subject();
    let start = gates_start(&mut subject);
    let system = subject.system(start, &[("cells", &["a", "b"]), ("gates", &["g"])]);
    let receiver = FootprintObject::Receiver;

    let pick = subject.symbolic("Gate", "pick");
    assert_eq!(
        pick.enabling,
        symbolic([
            field_of(&receiver, "Gate/armed"),
            any_field("Cell/v"),
            any_membership("cells"),
            membership("gates", &receiver),
        ])
    );
    let (a, g) = (
        subject.reference("Cell", "a"),
        subject.reference("Gate", "g"),
    );
    let set_a = footprint_on(&system, "Cell", "set", &a);
    let pick_g = footprint_on(&system, "Gate", "pick", &g);
    assert!(!independent(&set_a, &pick_g));
}

/// Jobs that may be cancelled, in a population `jobs`: deleting the receiver,
/// deleting any job, and unlinking the reference that blocks a deletion.
fn jobs_subject() -> Subject {
    let job = ident("Job");
    let own = |entries: &[&str]| -> Json {
        json!(entries.iter().map(|entry| ident(entry)).collect::<Vec<_>>())
    };
    subject(
        document(
            vec![object_type(
                "Job",
                vec![
                    field("Job", "armed", BOOLEAN, "required"),
                    field("Job", "cancellable", BOOLEAN, "required"),
                    field("Job", "bad", BOOLEAN, "required"),
                    field("Job", "peer", &job, "optional"),
                    field("Job", "target", &job, "optional"),
                ],
                vec![
                    operation(
                        "Job",
                        "boom",
                        &[],
                        None,
                        &frame(&json!({"modifies": own(&["Job/bad"])})),
                    ),
                    operation(
                        "Job",
                        "cancelSelf",
                        &[],
                        None,
                        &frame(&json!({"deletes_receiver": true})),
                    ),
                    operation(
                        "Job",
                        "cancelAny",
                        &[],
                        None,
                        &frame(&json!({"deletes": own(&["Job"]),
                            "modifies_receiver": own(&["Job/target"])})),
                    ),
                    operation(
                        "Job",
                        "unlink",
                        &[],
                        None,
                        &frame(&json!({"modifies_receiver": own(&["Job/peer"])})),
                    ),
                ],
            )],
            &[("jobs", "Job")],
        ),
        &[
            "pre BoomArmed using v on M::Job::boom { self.armed }",
            "post BoomBad using v on M::Job::boom { present(self.peer) and value(self.peer).bad }",
            "pre SelfCancellable using v on M::Job::cancelSelf { self.cancellable }",
            "pre AnyCancellable using v on M::Job::cancelAny { self.cancellable and present(self.target) }",
            "pre HasPeer using v on M::Job::unlink { present(self.peer) }",
            "post NoPeer using v on M::Job::unlink { not present(self.peer) }",
        ],
    )
}

/// FR-155-AC-5, the deletion subjects: deleting the receiver writes its
/// membership, a type-wide delete writes any membership, and a delete reads
/// every reference field that could name the deleted object.
#[trace("TC-589", "FR-155-AC-5")]
#[test]
fn deletions_write_membership_and_read_the_references_that_block_them() {
    let mut subject = jobs_subject();
    let objects = |key, peer: Json| Obj {
        population: "jobs",
        key,
        object_type: "Job",
        fields: json!({"armed": boolean(true), "cancellable": boolean(true),
            "bad": boolean(false), "peer": peer, "target": absent()}),
    };
    let start = subject.start(
        "s0",
        &["jobs"],
        &[
            objects("j", present(&reference("jobs", "k"))),
            objects("k", absent()),
            objects("m", absent()),
        ],
    );
    let system = subject.system(start, &[("jobs", &["j", "k", "m"])]);
    let receiver = FootprintObject::Receiver;

    let cancel_self = subject.symbolic("Job", "cancelSelf");
    assert_eq!(
        cancel_self.writes,
        symbolic([membership("jobs", &receiver)])
    );
    assert!(cancel_self.reads.contains(&any_field("Job/peer")));
    assert!(cancel_self.reads.contains(&any_field("Job/target")));
    let cancel_any = subject.symbolic("Job", "cancelAny");
    assert!(cancel_any.writes.contains(&any_membership("jobs")));

    let (j, k, m) = (
        subject.reference("Job", "j"),
        subject.reference("Job", "k"),
        subject.reference("Job", "m"),
    );
    let boom_j = footprint_on(&system, "Job", "boom", &j);
    assert!(boom_j.reads.contains(&member_of("jobs", &j)));
    assert!(!independent(
        &footprint_on(&system, "Job", "cancelSelf", &j),
        &boom_j
    ));
    assert!(!independent(
        &footprint_on(&system, "Job", "cancelAny", &m),
        &boom_j
    ));

    let cancel_k = footprint_on(&system, "Job", "cancelSelf", &k);
    let unlink_j = footprint_on(&system, "Job", "unlink", &j);
    assert!(cancel_k.reads.contains(&Location::AnyField {
        field: key("Job/peer")
    }));
    assert!(unlink_j.writes.contains(&field_at(&j, "Job/peer")));
    assert!(!independent(&cancel_k, &unlink_j));
}

/// FR-383 item 6: a creation reads the membership of its own population and of
/// the population each of its reference fields names, and a write to a
/// reference-valued field reads the membership of the population it names.
#[trace("TC-589")]
#[test]
fn creations_and_reference_writes_read_the_membership_of_their_targets() {
    let subject = gates_subject();
    let build = subject.symbolic("Gate", "build");
    assert!(build.reads.contains(&any_membership("gates")));
    assert!(build.reads.contains(&any_membership("cells")));
    let point = subject.symbolic("Gate", "point");
    assert!(point.reads.contains(&any_membership("cells")));
    let disarm = subject.symbolic("Gate", "disarm");
    assert!(!disarm.reads.contains(&any_membership("cells")));
}

/// FR-383-AC-12 through FR-155-AC-5's deletion rule: a deletion framed on a
/// supertype, applied to a receiver of a subtype, reads the references that
/// could name the subtype and writes the subtype population's membership.
#[trace("TC-589")]
#[test]
fn a_supertype_deletion_applied_to_a_subtype_uses_the_receivers_type() {
    let sub = ident("Sub");
    let mut subject = subject(
        document(
            vec![
                object_type(
                    "Base",
                    vec![],
                    vec![operation(
                        "Base",
                        "cancel",
                        &[],
                        None,
                        &frame(&json!({"deletes_receiver": true})),
                    )],
                ),
                object_type_extending("Sub", &["Base"], vec![], vec![]),
                object_type(
                    "Holder",
                    vec![field("Holder", "r", &sub, "optional")],
                    vec![operation(
                        "Holder",
                        "unlink",
                        &[],
                        None,
                        &frame(&json!({"modifies_receiver": [ident("Holder/r")]})),
                    )],
                ),
            ],
            &[("bases", "Base"), ("subs", "Sub"), ("holders", "Holder")],
        ),
        &[],
    );
    let receiver = FootprintObject::Receiver;
    let cancel = subject.symbolic("Sub", "cancel");
    assert!(cancel.writes.contains(&membership("subs", &receiver)));
    assert!(cancel.reads.contains(&any_field("Holder/r")));

    let objects = [
        Obj {
            population: "bases",
            key: "s",
            object_type: "Sub",
            fields: json!({}),
        },
        Obj {
            population: "holders",
            key: "h",
            object_type: "Holder",
            fields: json!({"r": present(&reference("bases", "s"))}),
        },
    ];
    let start = subject.start("s0", &["bases", "subs", "holders"], &objects);
    let system = subject.system(
        start,
        &[("bases", &["s"]), ("subs", &[]), ("holders", &["h"])],
    );
    let (s, h) = (
        subject.reference("Sub", "s"),
        subject.reference("Holder", "h"),
    );
    let cancel_s = footprint_on(&system, "Base", "cancel", &s);
    let unlink_h = footprint_on(&system, "Holder", "unlink", &h);
    assert!(cancel_s.reads.contains(&Location::AnyField {
        field: key("Holder/r")
    }));
    assert!(!independent(&cancel_s, &unlink_h));
}
