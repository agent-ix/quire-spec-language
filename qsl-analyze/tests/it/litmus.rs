// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-025 §10's store-buffering model as a fixture: the `test/litmus`
//! domain package, a population `cells` of `Cell` (fields `id`, `v` and `w`,
//! each `Int[0, 3]`) with operations `set` (`v`), `add` (`v`) and `both`
//! (`v` and `w`), and a population `cores` of `Core` (registers `r1` to
//! `r4`) with operations `read1` to `read4(c: Cell)`, each modifying its own
//! register, and `copy`, which reads `r1` into `r3`.
//!
//! An attempt names no receiver, so a cell's `id` selects it: `IsX` holds
//! for `x` (`id = 0`) and `IsY` for `y` (`id = 1`); `id` is never written,
//! so no two branches share it. One `Core` object, `core`, holds every
//! thread's register: `read1` writes `r1` only, so the left thread's
//! register is `(core, r1)` and the right one's `(core, r2)`.

use std::collections::BTreeMap;

use qsl_eval::model_subject::{ModelInstance, ModelSubject, ProtocolSubject, RoleBinding};
use qsl_eval::simulation::model::{
    ExpansionLimits, ModelSimState, ModelSimulationRequest, PopulationUniverse,
};
use qsl_eval::simulation::protocol::{MemorySelection, ResolvedMemory};
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

pub(crate) const PACKAGE: &str = "test/litmus";
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

fn multiplicity() -> Json {
    json!({"lower": 1, "upper": 1, "ordered": false, "unique": true})
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

/// The operation `name` of `owner` over the `Cell` parameters `params`,
/// modifying the receiver's fields `modifies`.
fn operation(owner: &str, name: &str, params: &[&str], modifies: &[&str]) -> Json {
    let identity = ident(&format!("{owner}/{name}"));
    let params: Vec<Json> = params
        .iter()
        .map(|parameter| {
            let parameter_identity = format!("{identity}/{parameter}");
            json!({
                "identity": parameter_identity, "name": parameter, "typeRef": ident("Cell"),
                "presence": "required", "nullable": false, "defaultKind": "none",
                "multiplicity": multiplicity(), "origin": origin(&parameter_identity),
            })
        })
        .collect();
    json!({
        "identity": identity, "name": name, "params": params, "pre": [], "post": [],
        "origin": {"source": {"sourceIdentity": ident("spec"), "path": "spec.qspec",
            "startLine": 1, "startColumn": 1}},
        "frame": {"modifies": [],
            "modifies_receiver": modifies
                .iter()
                .map(|field| ident(&format!("{owner}/{field}")))
                .collect::<Vec<_>>(),
            "creates": [], "deletes": []},
    })
}

fn fields(owner: &str, names: &[&str]) -> Vec<Json> {
    names
        .iter()
        .map(|name| {
            let field = ident(&format!("{owner}/{name}"));
            json!({
                "identity": field, "name": name, "typeRef": ident("Level"),
                "presence": "required", "nullable": false, "defaultKind": "none",
                "multiplicity": multiplicity(), "origin": origin(&field),
            })
        })
        .collect()
}

fn population(name: &str, member: &str) -> Json {
    let identity = ident(name);
    json!({
        "identity": identity, "displayName": identity,
        "kind": {"module": PACKAGE, "name": "population"},
        "members": [ident(member)], "extent": "closed", "origin": origin(&identity),
    })
}

/// The domain package.
pub(crate) fn document() -> Vec<u8> {
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
            wire_type(&ident("Cell"), "object_type", json!({
                "supertypes": [],
                "fields": fields("Cell", &["id", "v", "w"]),
                "operations": [
                    operation("Cell", "set", &[], &["v"]),
                    operation("Cell", "add", &[], &["v"]),
                    operation("Cell", "both", &[], &["v", "w"]),
                ],
                "relationships": [],
            })),
            wire_type(&ident("Core"), "object_type", json!({
                "supertypes": [],
                "fields": fields("Core", &["r1", "r2", "r3", "r4"]),
                "operations": [
                    operation("Core", "read1", &["c"], &["r1"]),
                    operation("Core", "read2", &["c"], &["r2"]),
                    operation("Core", "read3", &["c"], &["r3"]),
                    operation("Core", "read4", &["c"], &["r4"]),
                    operation("Core", "copy", &[], &["r3"]),
                ],
                "relationships": [],
            })),
        ],
        "populations": [population("cells", "Cell"), population("cores", "Core")],
    })
    .to_string()
    .into_bytes()
}

fn document_digest(document: &[u8]) -> [u8; 32] {
    *package_input([document]).keys().next().expect("one model")
}

/// The contracts: `IsX`/`IsY` select a cell, `StoreOne` stores 1,
/// `StoreTwo` stores 2, `AddOne` adds 1,
/// `BothOne` stores 1 to `v` and `w`; `ReadsX<n>`/`ReadsY<n>` select the cell
/// register `n` loads, `Read<n>` loads its `v` and `ReadW` loads `w`.
pub(crate) fn contracts() -> String {
    let mut text = String::from(
        "pre IsX using v on M::Cell::set { self.id = 0 }\n\
         pre IsY using v on M::Cell::set { self.id = 1 }\n\
         post StoreOne using v on M::Cell::set { self.v = 1 }\n\
         post StoreTwo using v on M::Cell::set { self.v = 2 }\n\
         pre AddX using v on M::Cell::add { self.id = 0 and self.v < 3 }\n\
         post AddOne using v on M::Cell::add { self.v - pre(self.v) = 1 }\n\
         pre BothX using v on M::Cell::both { self.id = 0 }\n\
         post BothOne using v on M::Cell::both { self.v = 1 and self.w = 1 }\n",
    );
    for register in 1..=4 {
        text.push_str(&format!(
            "pre ReadsX{register} using v on M::Core::read{register} {{ c.id = 0 }}\n\
             pre ReadsY{register} using v on M::Core::read{register} {{ c.id = 1 }}\n\
             post Read{register} using v on M::Core::read{register} {{ self.r{register} = pre(c.v) }}\n"
        ));
    }
    text.push_str(
        "post ReadW using v on M::Core::read3 { self.r3 = pre(c.w) }\n\
         post CopyR using v on M::Core::copy { self.r3 = pre(self.r1) }\n",
    );
    text
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
        SourceIdentity::new("agent-ix", "litmus", "git", "1"),
        "litmus.native",
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

/// An attempt `node` by role `role` of `owner::operation` with `ordering`
/// (empty for none) and `contracts`.
pub(crate) fn attempt(
    node: &str,
    role: &str,
    target: &str,
    ordering: &str,
    contracts: &str,
) -> String {
    let ordering = if ordering.is_empty() {
        String::new()
    } else {
        format!(" ordering {ordering}")
    };
    format!(
        "attempt {node} by {role} on M::{target}{ordering} contracts [{contracts}] \
         as (b{node}: Boolean) {{ true }};"
    )
}

/// A protocol `name` over the core with roles `w` (cells) and `c` (cores)
/// and the run control `run`.
pub(crate) fn protocol(name: &str, run: &str) -> String {
    protocol_with(name, "", run)
}

/// [`protocol`] with the further members `members`, such as a channel.
pub(crate) fn protocol_with(name: &str, members: &str, run: &str) -> String {
    format!(
        "protocol {name} using v over (k: M::Core) on origin {{\n\
         role w on M::Cell;\n\
         role c on M::Core;\n\
         {members}\n\
         run {run}\n\
         finish Done as (d: Boolean) {{ d }};\n\
         }}\n"
    )
}

/// ADR-025 §10's `SB::Both` with `memory` (empty for none), its stores
/// ordered `store` and its loads `load` (empty for none).
pub(crate) fn sb_run(memory: &str, store: &str, load: &str) -> String {
    format!(
        "parallel Both {memory} {{ \
         branch left sequence L {{ {} {} }} \
         branch right sequence R {{ {} {} }} }} join all [left, right];",
        attempt("Sx", "w", "Cell::set", store, "IsX, StoreOne"),
        attempt("Ly", "c", "Core::read1", load, "ReadsY1, Read1"),
        attempt("Sy", "w", "Cell::set", store, "IsY, StoreOne"),
        attempt("Lx", "c", "Core::read2", load, "ReadsX2, Read2"),
    )
}

/// ADR-025 §10's claim: the two loads never both read 0.
pub(crate) const NO_BOTH_ZERO: &str = "temporal NoBothZero using t0 over (k: M::Core) clock \
     \"model-steps\" on origin { always holds(not (k.r1 = 0 and k.r2 = 0)) }\n";

/// The `SB` unit over `run`, with the claim.
pub(crate) fn sb_unit(run: &str) -> String {
    contracts() + &protocol("SB", run) + NO_BOTH_ZERO
}

/// A subject's documents: the domain package and the snapshot.
pub(crate) struct Documents {
    pub(crate) packages: BTreeMap<[u8; 32], Vec<u8>>,
    pub(crate) snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    pub(crate) initial: DocumentRef,
}

impl Documents {
    /// Cells `x` (`id` 0) and `y` (`id` 1) at `v = w = 0`, and `core` with
    /// every register at 3, "not yet read".
    pub(crate) fn new() -> Self {
        let int = |value: i64| json!({"integer": value.to_string()});
        let cell = |key: &str, id: i64| {
            json!({"key": key, "type": ident("Cell"),
                "fields": {"id": int(id), "v": int(0), "w": int(0)}})
        };
        let header = json!({"identity": PACKAGE, "version": "1.0.0",
            "digest": format!("sha256-jcs:{}", hex(&document_digest(&document())))});
        let snapshot = json!({
            "format": "quire.state.snapshot/v1",
            "identity": {"authority": "test", "identity": "s0", "revision_namespace": "fixture",
                "revision": "fixture:1"},
            "observation": "current", "model": header,
            "anchor": {"kind": "initialization", "name": "start"},
            "populations": [
                {"population": ident("cells"), "complete": true,
                    "objects": [cell("x", 0), cell("y", 1)]},
                {"population": ident("cores"), "complete": true,
                    "objects": [{"key": "core", "type": ident("Core"),
                        "fields": {"r1": int(3), "r2": int(3), "r3": int(3),
                            "r4": int(3)}}]},
            ],
        });
        let bytes = snapshot.to_string().into_bytes();
        let read = quire_canonical::read(&bytes, u64::MAX).expect("JSON fixture");
        let digest = *quire_canonical::sha256(&read, quire_canonical::Limits::new(u64::MAX))
            .expect("canonical fixture")
            .as_bytes();
        Self {
            packages: package_input([document().as_slice()]),
            snapshots: BTreeMap::from([(digest, bytes)]),
            initial: DocumentRef {
                authority: "test".to_owned(),
                identity: "s0".to_owned(),
                revision_namespace: "fixture".to_owned(),
                revision: "fixture:1".to_owned(),
                digest,
            },
        }
    }

    /// The universes: `cells` `{x, y}`, `cores` `{core}`.
    pub(crate) fn universes() -> Vec<PopulationUniverse> {
        vec![
            PopulationUniverse {
                population: key("cells"),
                keys: vec!["x".to_owned(), "y".to_owned()],
            },
            PopulationUniverse {
                population: key("cores"),
                keys: vec!["core".to_owned()],
            },
        ]
    }

    /// The protocol subject of `protocol` over `package`, `k` bound to
    /// `core`, under the request's `selection`.
    pub(crate) fn subject<'a>(
        &'a self,
        package: &'a CheckedPackage,
        protocol: &str,
        selection: &[MemorySelection],
    ) -> ProtocolSubject<'a> {
        let checked = package.graph().protocol(protocol).expect("the protocol");
        ProtocolSubject {
            model: ModelSubject {
                simulation: ModelSimulationRequest {
                    package,
                    packages: &self.packages,
                    normalization: ModelNormalizationLimits::default(),
                    snapshots: &self.snapshots,
                    initial: vec![self.initial.clone()],
                    universes: Self::universes(),
                    observation: ObservationLimits::default(),
                    expansion: ExpansionLimits {
                        max_candidates: 1_000_000,
                    },
                    meter: UNLIMITED,
                },
                proof_bounds: Vec::new(),
            },
            protocol: protocol.to_owned(),
            over: core(),
            roles: ["w", "c"]
                .into_iter()
                .map(|role| RoleBinding {
                    role: role.to_owned(),
                    model: PACKAGE.to_owned(),
                })
                .collect(),
            memory: ResolvedMemory::resolve(checked, selection).expect("the selection resolves"),
        }
    }
}

/// The `core` instance.
pub(crate) fn core() -> ModelInstance {
    ModelInstance {
        population: key("cores"),
        key: "core".to_owned(),
    }
}

/// The integer `field` of the object `object` of `population` at `state`.
pub(crate) fn field(state: &ModelSimState, population: &str, object: &str, field: &str) -> i64 {
    let owner = if population == "cells" {
        "Cell"
    } else {
        "Core"
    };
    match state.state().populations[&key(population)][object]
        .fields
        .get(&key(&format!("{owner}/{field}")))
    {
        Some(Value::Integer(value)) => value.to_string().parse().expect("a small value"),
        other => panic!("{object}.{field} is not an integer: {other:?}"),
    }
}
