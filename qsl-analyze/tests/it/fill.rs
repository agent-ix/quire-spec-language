// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-027 §7's worked example as a fixture: the `test/fill` domain package
//! (population `cells` of `Cell`, field `v: Int[0, 3]`, operations `inc`,
//! `setTwo`, `bump` and `flag` modifying `v`, `flag` with a Boolean result,
//! and `touch`, which modifies nothing and is always enabled), a native unit
//! that selects it as `M` with their contracts, protocols over it, and the
//! snapshots and protocol subjects the protocol tests build from it.

use std::collections::BTreeMap;

use qsl_eval::model_subject::{ModelInstance, ModelSubject, ProtocolSubject, RoleBinding};
use qsl_eval::simulation::model::{
    ExpansionLimits, ModelSimState, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use qsl_eval::simulation::protocol::{
    ProtocolState, ProtocolSystem, ProtocolTransition, Sc, StepKind,
};
use qsl_eval::simulation::{SubjectSystem, TransitionSystem};
use qsl_foundation::SourceIdentity;
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, meaning, package_input};
use qsl_semantics::model::key::{hex, DeclarationKey};
use qsl_semantics::model::observation::{DocumentRef, ObservationLimits};
use quire_exact::Value;
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value as Json};

use crate::counter::UNLIMITED;

pub(crate) const PACKAGE: &str = "test/fill";
const PLACEHOLDER: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

fn ident(suffix: &str) -> String {
    format!("ix://{PACKAGE}/{suffix}")
}

pub(crate) fn key(suffix: &str) -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE.to_owned(),
        node: ident(suffix),
    }
}

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

fn level() -> Json {
    let identity = ident("Level");
    let constraint = |keyword: &str, value: &str| {
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
            "constraints": [constraint("min", "0"), constraint("max", "3")]}),
    )
}

/// The operation `name` of `Cell`, modifying `v` when `modifies`, with a
/// Boolean result when `returns`.
fn operation(name: &str, modifies: bool, returns: bool) -> Json {
    let identity = ident(&format!("Cell/{name}"));
    let mut operation = json!({
        "identity": identity, "name": name, "params": [], "pre": [], "post": [],
        "origin": {"source": {"sourceIdentity": ident("spec"), "path": "spec.qspec",
            "startLine": 1, "startColumn": 1}},
        "frame": {"modifies": if modifies { vec![ident("Cell/v")] } else { Vec::new() },
            "creates": [], "deletes": []},
    });
    if returns {
        operation["returns"] = json!({"typeRef": "ix://quire/native/Boolean", "nullable": false,
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true}});
    }
    operation
}

/// The domain package.
pub(crate) fn document() -> Vec<u8> {
    let cell = ident("Cell");
    let constructs: Vec<Json> = [
        ("object_type", meaning::OBJECT_TYPE),
        ("population", meaning::POPULATION),
        ("value_type", meaning::VALUE_TYPE),
    ]
    .into_iter()
    .map(|(name, meaning)| {
        json!({"kind": {"module": PACKAGE, "name": name}, "moduleVersion": "1.0.0",
            "manifestDigest": PLACEHOLDER,
            "construct": {"identity": "none", "shape": "record", "members": {},
                "meaning": meaning}})
    })
    .collect();
    let value = ident("Cell/v");
    let population = ident("cells");
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
            level(),
            wire_type(&cell, "object_type", json!({
                "supertypes": [],
                "fields": [{
                    "identity": value, "name": "v", "typeRef": ident("Level"),
                    "presence": "required", "nullable": false, "defaultKind": "none",
                    "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                    "origin": origin(&value),
                }],
                "operations": [
                    operation("inc", true, false),
                    operation("setTwo", true, false),
                    operation("bump", true, false),
                    operation("flag", true, true),
                    operation("touch", false, false),
                ],
                "relationships": [],
            })),
        ],
        "populations": [{
            "identity": population, "displayName": population,
            "kind": {"module": PACKAGE, "name": "population"},
            "members": [cell], "extent": "closed", "origin": origin(&population),
        }],
    })
    .to_string()
    .into_bytes()
}

fn document_digest(document: &[u8]) -> [u8; 32] {
    *package_input([document]).keys().next().expect("one model")
}

/// ADR-027 §7's contracts, with `TwoPre` as `two_pre`; `bump`'s of FR-206-AC-2;
/// `flag`'s, which `inc` with a Boolean result.
pub(crate) fn contracts(two_pre: &str) -> String {
    format!(
        "pre IncPre using v on M::Cell::inc {{ self.v < 3 }}\n\
         post IncPost using v on M::Cell::inc {{ self.v - pre(self.v) = 1 }}\n\
         pre TwoPre using v on M::Cell::setTwo {{ {two_pre} }}\n\
         post TwoPost using v on M::Cell::setTwo {{ self.v = 2 }}\n\
         post BumpPost using v on M::Cell::bump {{ self.v - pre(self.v) >= 1 }}\n\
         pre FlagPre using v on M::Cell::flag {{ self.v < 3 }}\n\
         post FlagPost using v on M::Cell::flag {{ self.v - pre(self.v) = 1 }}\n\
         pre TouchPre using v on M::Cell::touch {{ true }}\n"
    )
}

/// The unit selecting the domain package as `M`, with `body` after the
/// header.
pub(crate) fn unit_text(body: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile t0 = \"quire.temporal.infinite-trace/v1\";\n\
         model M = \"{PACKAGE}\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n{body}",
        hex(&document_digest(&document()))
    )
}

/// The checked package of `body`.
pub(crate) fn package(body: &str) -> CheckedPackage {
    let document = document();
    let text = unit_text(body);
    let packages = package_input([document.as_slice()]);
    let parsed = qsl_cst::parse(
        SourceIdentity::new("agent-ix", "counter", "git", "1"),
        "counter.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 parses the unit");
    assert!(
        parsed.is_admissible(),
        "S1: {text}: {:?}",
        parsed.diagnostics()
    );
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
    .expect("the operation clauses parse");
    let graph = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        built,
        models,
        Vec::new(),
    )
    .expect("the unit assembles")
    .check(CheckingLimits::default())
    .unwrap_or_else(|refusals| panic!("{body}: the unit checks: {refusals:?}"));
    CheckedPackage::link(graph)
}

/// A protocol `name` over `k: M::Cell` with role `w`, `members` before
/// `run`, the run control `run` and the finish constraint `finish`.
pub(crate) fn protocol(name: &str, members: &str, run: &str, finish: &str) -> String {
    format!(
        "protocol {name} using v over (k: M::Cell) on origin {{\n\
         role w on M::Cell;\n\
         {members}\n\
         run {run}\n\
         finish Done as (d: Boolean) {{ {finish} }};\n\
         }}\n"
    )
}

/// An attempt `node` of `operation` with `contracts`, binder `binder` and
/// constraint `constraint`.
pub(crate) fn attempt(
    node: &str,
    operation: &str,
    contracts: &str,
    binder: &str,
    constraint: &str,
) -> String {
    format!(
        "attempt {node} by w on M::Cell::{operation} contracts [{contracts}] \
         as ({binder}: Boolean) {{ {constraint} }};"
    )
}

/// ADR-027 §7's `run` control: `fork Both` of `A` (`inc`) and `B`
/// (`setTwo`), joined `all`.
pub(crate) fn fill_run() -> String {
    format!(
        "sequence Main {{ parallel Both {{ branch left {} branch right {} }} join all [left, right]; }}",
        attempt("A", "inc", "IncPre, IncPost", "a", "true"),
        attempt("B", "setTwo", "TwoPre, TwoPost", "b", "true"),
    )
}

/// ADR-027 §7's `Fill` unit with `TwoPre` as `two_pre` and `members`.
pub(crate) fn fill_unit(two_pre: &str, members: &str) -> String {
    contracts(two_pre) + &protocol("Fill", members, &fill_run(), "d")
}

/// A subject's documents: the domain package and the snapshots.
pub(crate) struct Documents {
    pub(crate) packages: BTreeMap<[u8; 32], Vec<u8>>,
    pub(crate) snapshots: BTreeMap<[u8; 32], Vec<u8>>,
}

impl Documents {
    pub(crate) fn new() -> Self {
        Self {
            packages: package_input([document().as_slice()]),
            snapshots: BTreeMap::new(),
        }
    }

    /// An initialization snapshot named `name` of the cells `objects`, each
    /// `(key, v)`, provisioned here.
    pub(crate) fn start(&mut self, name: &str, objects: &[(&str, i64)]) -> DocumentRef {
        let objects: Vec<Json> = objects
            .iter()
            .map(|(key, value)| {
                json!({"key": key, "type": ident("Cell"),
                    "fields": {"v": {"integer": value.to_string()}}})
            })
            .collect();
        let header = json!({"identity": PACKAGE, "version": "1.0.0",
            "digest": format!("sha256-jcs:{}", hex(&document_digest(&document())))});
        let document = json!({
            "format": "quire.state.snapshot/v1",
            "identity": {"authority": "test", "identity": name, "revision_namespace": "fixture",
                "revision": "fixture:1"},
            "observation": "current", "model": header,
            "anchor": {"kind": "initialization", "name": "start"},
            "populations": [{"population": ident("cells"), "complete": true,
                "objects": objects}],
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
}

/// The instance of the cell `key` of `cells`.
pub(crate) fn instance(key: &str) -> ModelInstance {
    ModelInstance {
        population: self::key("cells"),
        key: key.to_owned(),
    }
}

/// The universe `{keys}` of `cells`.
pub(crate) fn universe(keys: &[&str]) -> Vec<PopulationUniverse> {
    vec![PopulationUniverse {
        population: key("cells"),
        keys: keys.iter().map(|key| (*key).to_owned()).collect(),
    }]
}

/// The model subject over `package` from `initial` with universe `keys`.
pub(crate) fn model_subject<'a>(
    package: &'a CheckedPackage,
    documents: &'a Documents,
    initial: Vec<DocumentRef>,
    keys: &[&str],
) -> ModelSubject<'a> {
    ModelSubject {
        simulation: ModelSimulationRequest {
            package,
            packages: &documents.packages,
            normalization: ModelNormalizationLimits::default(),
            snapshots: &documents.snapshots,
            initial,
            universes: universe(keys),
            observation: ObservationLimits::default(),
            expansion: ExpansionLimits {
                max_candidates: 1_000_000,
            },
            meter: UNLIMITED,
        },
        proof_bounds: Vec::new(),
    }
}

/// The protocol subject of `protocol` over `package`, `over` bound to the
/// cell `over` and `w` bound to the model.
pub(crate) fn subject<'a>(
    package: &'a CheckedPackage,
    documents: &'a Documents,
    initial: Vec<DocumentRef>,
    protocol: &str,
    over: &str,
) -> ProtocolSubject<'a> {
    ProtocolSubject {
        model: model_subject(package, documents, initial, &["c"]),
        protocol: protocol.to_owned(),
        over: instance(over),
        roles: vec![RoleBinding {
            role: "w".to_owned(),
            model: PACKAGE.to_owned(),
        }],
    }
}

/// The system of `subject` under `Sc`.
pub(crate) fn sc_system<'a>(subject: &ProtocolSubject<'a>) -> ProtocolSystem<'a, Sc> {
    let model = ModelSystem::new(subject.model.simulation.clone()).expect("the subject admits");
    ProtocolSystem::new(model, subject, Sc).expect("the protocol system builds")
}

/// The `v` of the cell `object` at `state`.
pub(crate) fn v(state: &ModelSimState, object: &str) -> i64 {
    match state.state().populations[&key("cells")][object]
        .fields
        .get(&key("Cell/v"))
    {
        Some(Value::Integer(value)) => value.to_string().parse().expect("a small value"),
        other => panic!("{object}.v is not an integer: {other:?}"),
    }
}

/// The compiled `package_id` of the unit with `body`, through FR-098's
/// compile.
pub(crate) fn package_id(body: &str) -> qsl_replay::DigestRecord {
    compiled(body).expect("the unit compiles").package_id()
}

/// FR-098's compile of the unit with `body`.
pub(crate) fn compiled(
    body: &str,
) -> Result<qsl_replay::CompiledPackage, qsl_replay::ReplayRefusal> {
    let document = document();
    qsl_replay::compile_package(
        SourceIdentity::new("agent-ix", "counter", "git", "1"),
        "counter.native",
        unit_text(body).as_bytes(),
        [document.as_slice()],
        &qsl_replay::DependencyInput::default(),
        qsl_replay::StageLimits::default(),
        qsl_replay::ReplayLimits::default(),
    )
}

/// One reachable state of a protocol system: its key digest, the state, and
/// each successor's identity and index.
pub(crate) struct Reached {
    pub(crate) state: ProtocolState<Sc>,
    pub(crate) steps: Vec<(ProtocolTransition, usize)>,
}

/// Every reachable state of `system`, breadth-first from its initial states,
/// each once by its state key, successors in the engine's order.
pub(crate) fn reachable(system: &ProtocolSystem<'_, Sc>) -> Vec<Reached> {
    let mut known: BTreeMap<Vec<u8>, usize> = BTreeMap::new();
    let mut reached: Vec<Reached> = Vec::new();
    let mut queue = std::collections::VecDeque::new();
    let mut admit = |state: ProtocolState<Sc>,
                     reached: &mut Vec<Reached>,
                     queue: &mut std::collections::VecDeque<usize>|
     -> usize {
        let (key, _) = SubjectSystem::state_key(system, &state).expect("a key");
        *known.entry(key).or_insert_with(|| {
            reached.push(Reached {
                state,
                steps: Vec::new(),
            });
            queue.push_back(reached.len() - 1);
            reached.len() - 1
        })
    };
    for state in system.initial() {
        admit(state, &mut reached, &mut queue);
    }
    while let Some(at) = queue.pop_front() {
        let expansion = system
            .successors(&reached[at].state)
            .expect("the state expands");
        let mut successors: Vec<(Vec<u8>, ProtocolTransition, ProtocolState<Sc>)> = expansion
            .successors
            .into_iter()
            .map(|(transition, state)| {
                (
                    system.transition_key(&transition).expect("an identity"),
                    transition,
                    state,
                )
            })
            .collect();
        successors.sort_by(|left, right| left.0.cmp(&right.0));
        for (_, transition, state) in successors {
            let next = admit(state, &mut reached, &mut queue);
            reached[at].steps.push((transition, next));
        }
    }
    reached
}

/// Every maximal behaviour's projection onto its attempt nodes, from each
/// initial state of `reached`, by the attempts' node names' last segment.
pub(crate) fn projections(reached: &[Reached]) -> std::collections::BTreeSet<Vec<String>> {
    fn walk(
        reached: &[Reached],
        at: usize,
        prefix: &mut Vec<String>,
        out: &mut std::collections::BTreeSet<Vec<String>>,
    ) {
        if reached[at].steps.is_empty() {
            out.insert(prefix.clone());
            return;
        }
        for (transition, next) in &reached[at].steps {
            let named = match transition.kind() {
                StepKind::Attempt | StepKind::Finish => transition
                    .node()
                    .and_then(|node| node.rsplit('/').next())
                    .map(str::to_owned),
                StepKind::Fork | StepKind::Join | StepKind::Memory => None,
            };
            if let Some(name) = &named {
                prefix.push(name.clone());
            }
            walk(reached, *next, prefix, out);
            if named.is_some() {
                prefix.pop();
            }
        }
    }
    let mut out = std::collections::BTreeSet::new();
    walk(reached, 0, &mut Vec::new(), &mut out);
    out
}

/// The JSON of a canonical key or identity's bytes.
pub(crate) fn json_of(bytes: &[u8]) -> Json {
    serde_json::from_slice(bytes).expect("canonical bytes are JSON")
}
