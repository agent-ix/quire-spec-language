// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-025 §10's store-buffering model as a fixture: the `test/litmus`
//! domain package, a population `cells` of `Cell` (fields `id`, `v` and `w`,
//! each `Int[0, 3]`) with operations `set` (`v`), `add` (`v`) and `both`
//! (`v` and `w`), and a population `cores` of `Core` (registers `r1` to
//! `r4`) with operations `read1` to `read4(c: Cell)`, each modifying its own
//! register, `look(c: Cell)`, which reads `v` into `r1` and returns whether it
//! read 1, and `copy`, which reads `r1` into `r3`.
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
use qsl_semantics::model::observation::{DocumentRef, ObservationLimits};
use quire_checked::key::DeclarationKey;
use quire_exact::Value;
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value as Json};

use crate::counter::UNLIMITED;

pub(crate) const PACKAGE: &str = "test/litmus";

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

/// `operation` with a Boolean result.
fn returning(mut operation: Json) -> Json {
    operation["returns"] = json!({"typeRef": "ix://quire/native/Boolean", "nullable": false,
        "multiplicity": multiplicity()});
    operation
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
            "construct": {"identity": "none", "shape": "record", "members": {},
                "meaning": meaning}})
    })
    .collect();
    json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1"},
        "source": {"identity": ident("spec"), "version": "1.0.0", "dialect": "spec-bundle"},
        "package": {"identity": PACKAGE, "version": "1.0.0",
            "mappingVersions": [], "profileVersions": []},
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
                    operation("Cell", "setw", &[], &["w"]),
                ],
                "relationships": [],
            })),
            wire_type(&ident("Spare"), "object_type", json!({
                "supertypes": [],
                "fields": fields("Spare", &["v"]),
                "operations": [],
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
                    returning(operation("Core", "look", &["c"], &["r1"])),
                    operation("Core", "copy", &[], &["r3"]),
                ],
                "relationships": [],
            })),
        ],
        "populations": [
            population("cells", "Cell"),
            population("cores", "Core"),
            population("spares", "Spare"),
        ],
    })
    .to_string()
    .into_bytes()
}

/// The `Cell` value domain used when a refinement has no abstract `Core` or
/// `Spare` counterpart.
pub(crate) fn cell_document() -> Vec<u8> {
    let mut value: Json = serde_json::from_slice(&document()).expect("litmus document JSON");
    value["types"]
        .as_array_mut()
        .expect("types")
        .retain(|node| node["identity"] == ident("Level") || node["identity"] == ident("Cell"));
    value["populations"]
        .as_array_mut()
        .expect("populations")
        .retain(|population| population["identity"] == ident("cells"));
    value.to_string().into_bytes()
}

/// A single admitted `Cell` state and its package provision.
pub(crate) struct CellDocuments {
    pub(crate) packages: qsl_semantics::model::intake::PackageInput,
    pub(crate) snapshots: qsl_semantics::model::observation::DocumentProvision,
    pub(crate) initial: DocumentRef,
    pub(crate) keys: Vec<String>,
}

impl CellDocuments {
    /// Cell `x` (`id` 0) with `v = w = 0`.
    pub(crate) fn new() -> Self {
        let int = |value: i64| json!({"integer": value.to_string()});
        let snapshot = json!({
            "format": "quire.state.snapshot/v1",
            "identity": {"authority": "test", "identity": "s0-cell", "revision_namespace": "fixture", "revision": "fixture:1"},
            "observation": "current", "model": {"identity": PACKAGE},
            "anchor": {"kind": "initialization", "name": "start"},
            "populations": [{"population": ident("cells"), "complete": true,
                "objects": [{"key": "c", "type": ident("Cell"),
                    "fields": {"id": int(0), "v": int(0), "w": int(0)}}]}],
        });
        let bytes = snapshot.to_string().into_bytes();
        let initial = qsl_semantics::model::observation::document_reference(&bytes)
            .expect("snapshot refers to itself");
        Self {
            packages: package_input([cell_document().as_slice()]).expect("cell package input"),
            snapshots: BTreeMap::from([(initial.clone(), bytes)]),
            initial,
            keys: vec!["c".to_owned()],
        }
    }
}

/// The contracts: `IsX`/`IsY`/`IsZ` select a cell, `StoreZero` stores 0,
/// `StoreOne` stores 1, `StoreTwo` stores 2, `StoreThree` stores 3, `AddOne` adds 1,
/// `IsXw` and `FlagOne` store 1 to the `w` of `x`, `ReadFlag<n>` loads it,
/// `BothOne` stores 1 to `v` and `w`; `LooksY` selects the flag cell `look`
/// loads and `Looked` loads its `v` into `r1` and returns whether it is 1;
/// `ReadsX<n>`/`ReadsY<n>`/`ReadsZ<n>` select the cell
/// register `n` loads, `Read<n>` loads its `v` and `ReadW` loads `w`.
pub(crate) fn contracts() -> String {
    let mut text = String::from(
        "pre IsX using v on M::Cell::set { self.id = 0 }\n\
         pre XZero using v on M::Cell::set { self.id = 0 and self.v = 0 }\n\
         pre XWZero using v on M::Cell::set { self.id = 0 and self.w = 0 }\n\
         pre XWZeroBoth using v on M::Cell::both { self.id = 0 and self.w = 0 }\n\
         pre IsY using v on M::Cell::set { self.id = 1 }\n\
         post StoreZero using v on M::Cell::set { self.v = 0 }\n\
         post StoreOne using v on M::Cell::set { self.v = 1 }\n\
         post StoreTwo using v on M::Cell::set { self.v = 2 }\n\
         post StoreThree using v on M::Cell::set { self.v = 3 }\n\
         pre IsZ using v on M::Cell::set { self.id = 2 }\n\
         pre IsXw using v on M::Cell::setw { self.id = 0 }\n\
         post FlagOne using v on M::Cell::setw { self.w = 1 }\n\
         pre AddX using v on M::Cell::add { self.id = 0 and self.v < 3 }\n\
         post AddOne using v on M::Cell::add { self.v - pre(self.v) = 1 }\n\
         pre BothX using v on M::Cell::both { self.id = 0 }\n\
         post BothOne using v on M::Cell::both { self.v = 1 and self.w = 1 }\n",
    );
    for register in 1..=4 {
        text.push_str(&format!(
            "pre ReadsX{register} using v on M::Core::read{register} {{ c.id = 0 }}\n\
             pre ReadsY{register} using v on M::Core::read{register} {{ c.id = 1 }}\n\
             pre ReadsZ{register} using v on M::Core::read{register} {{ c.id = 2 }}\n\
             post Read{register} using v on M::Core::read{register} {{ self.r{register} = pre(c.v) }}\n\
             post ReadFlag{register} using v on M::Core::read{register} {{ self.r{register} = pre(c.w) }}\n"
        ));
    }
    text.push_str(
        "pre LooksY using v on M::Core::look { c.id = 1 }\n\
         post Looked using v on M::Core::look { self.r1 = pre(c.v) and result = (pre(c.v) = 1) }\n\
         post ReadW using v on M::Core::read3 { self.r3 = pre(c.w) }\n\
         post CopyR using v on M::Core::copy { self.r3 = pre(self.r1) }\n",
    );
    text
}

/// The unit selecting the domain package as `M`, with `body` after the
/// header.
pub(crate) fn unit_text(body: &str) -> String {
    unit_text_with("", body)
}

/// [`unit_text`] with the further `profiles` lines in its header.
pub(crate) fn unit_text_with(profiles: &str, body: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile t0 = \"quire.temporal.infinite-trace/v1\";\n{profiles}\
         model M = \"{PACKAGE}\";\n{body}"
    )
}

/// The checked package of `body`.
pub(crate) fn package(body: &str) -> CheckedPackage {
    package_with("", body)
}

/// The checked package of `body` that also selects the protocol profile,
/// so each protocol clause the package writes has its node.
pub(crate) fn protocol_profiled_package(body: &str) -> CheckedPackage {
    package_with(PROTOCOL_PROFILE, body)
}

/// The protocol-profiled package with checked abstract refinement internals.
pub(crate) fn protocol_profiled_package_with_internals(
    body: &str,
    internals: serde_json::Value,
) -> CheckedPackage {
    package_with_internals(PROTOCOL_PROFILE, body, Some(internals))
}

/// Build a protocol-profiled package from the litmus package plus additional
/// admitted domain packages.
pub(crate) fn protocol_package_with_provisions(
    body: &str,
    packages: qsl_semantics::model::intake::PackageInput,
    internals: serde_json::Value,
) -> CheckedPackage {
    let text = unit_text_with(PROTOCOL_PROFILE, body);
    package_with_provisions(&text, body, packages, Some(internals))
}

/// The header line selecting the protocol profile.
pub(crate) const PROTOCOL_PROFILE: &str = "profile pp = \"quire.protocol.finite-global/v1\";\n";

/// The checked package of `body` under the header's further `profiles`.
fn package_with(profiles: &str, body: &str) -> CheckedPackage {
    package_with_internals(profiles, body, None)
}

fn package_with_internals(
    profiles: &str,
    body: &str,
    internals: Option<serde_json::Value>,
) -> CheckedPackage {
    let document = document();
    let text = unit_text_with(profiles, body);
    let packages = package_input([document.as_slice()]).unwrap();
    package_with_provisions(&text, body, packages, internals)
}

fn package_with_provisions(
    text: &str,
    body: &str,
    packages: qsl_semantics::model::intake::PackageInput,
    internals: Option<serde_json::Value>,
) -> CheckedPackage {
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
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "package".to_owned(),
        built,
        models,
        Vec::new(),
    )
    .expect("the unit assembles");
    let declarations = match internals {
        Some(internals) => declarations.with_refinement_internals(internals),
        None => declarations.with_unwritten_refinements(),
    };
    let graph = declarations
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
    pub(crate) cells: Vec<&'static str>,
    pub(crate) packages: qsl_semantics::model::intake::PackageInput,
    pub(crate) snapshots: qsl_semantics::model::observation::DocumentProvision,
    pub(crate) initial: DocumentRef,
    /// Whether the universe also holds the `Spare` object `z`.
    spare: bool,
    /// Whether the initial model holds the `Core` instance.
    has_core: bool,
}

impl Documents {
    /// Cells `x` (`id` 0) and `y` (`id` 1) at `v = w = 0`, and `core` with
    /// every register at 3, "not yet read".
    pub(crate) fn new() -> Self {
        Self::build(&["x", "y"], false, true)
    }

    /// [`Self::new`] with the cells `cells` (`x`, `y`, `z` in `id` order 0, 1, 2).
    pub(crate) fn with_cells(cells: &[&'static str]) -> Self {
        Self::build(cells, false, true)
    }

    /// [`Self::with_cells`] without the `Core` object.
    pub(crate) fn cells_without_core(cells: &[&'static str]) -> Self {
        Self::build(cells, false, false)
    }

    /// [`Self::new`] with the `Spare` object `z`, which no operation touches,
    /// in the universe.
    pub(crate) fn with_spare_cell() -> Self {
        Self::build(&["x", "y"], true, true)
    }

    fn build(cells: &[&'static str], spare: bool, has_core: bool) -> Self {
        let int = |value: i64| json!({"integer": value.to_string()});
        let cell = |key: &str, id: i64| {
            json!({"key": key, "type": ident("Cell"),
                "fields": {"id": int(id), "v": int(0), "w": int(0)}})
        };
        let objects: Vec<Json> = cells
            .iter()
            .zip(0..)
            .map(|(name, id)| cell(name, id))
            .collect();
        let header = json!({"identity": PACKAGE});
        let spares: Vec<Json> = if spare {
            vec![json!({"key": "z", "type": ident("Spare"),
                "fields": {"v": int(0)}})]
        } else {
            Vec::new()
        };
        let cores: Vec<Json> = if has_core {
            vec![json!({"key": "core", "type": ident("Core"),
                "fields": {"r1": int(3), "r2": int(3), "r3": int(3), "r4": int(3)}})]
        } else {
            Vec::new()
        };
        let snapshot = json!({
            "format": "quire.state.snapshot/v1",
            "identity": {"authority": "test", "identity": "s0", "revision_namespace": "fixture",
                "revision": "fixture:1"},
            "observation": "current", "model": header,
            "anchor": {"kind": "initialization", "name": "start"},
            "populations": [
                {"population": ident("spares"), "complete": true, "objects": spares},
                {"population": ident("cells"), "complete": true,
                    "objects": objects},
                {"population": ident("cores"), "complete": true, "objects": cores},
            ],
        });
        let bytes = snapshot.to_string().into_bytes();
        Self {
            cells: cells.to_vec(),
            packages: package_input([document().as_slice()]).unwrap(),
            snapshots: BTreeMap::from([(
                qsl_semantics::model::observation::document_reference(&bytes)
                    .expect("a document names itself"),
                bytes,
            )]),
            initial: DocumentRef {
                authority: "test".to_owned(),
                identity: "s0".to_owned(),
                revision_namespace: "fixture".to_owned(),
                revision: "fixture:1".to_owned(),
            },
            spare,
            has_core,
        }
    }

    /// The universes: `cells` the cells of the snapshot, `spares` the spare
    /// object if present, `cores` `{core}`.
    pub(crate) fn universes(&self) -> Vec<PopulationUniverse> {
        vec![
            PopulationUniverse {
                population: key("cells"),
                keys: self.cells.iter().map(|name| (*name).to_owned()).collect(),
            },
            PopulationUniverse {
                population: key("spares"),
                keys: if self.spare {
                    vec!["z".to_owned()]
                } else {
                    Vec::new()
                },
            },
            PopulationUniverse {
                population: key("cores"),
                keys: if self.has_core {
                    vec!["core".to_owned()]
                } else {
                    Vec::new()
                },
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
                    universes: self.universes(),
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
