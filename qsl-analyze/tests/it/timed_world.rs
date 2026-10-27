// SPDX-License-Identifier: AGPL-3.0-or-later
//! A timed fixture for EN-6: the domain package `test/timed`, whose object
//! `Call` holds `phase: Int[0, 3]`, the Booleans `inflight`, `late` and
//! `retried`, and the clock fields a test names, with the operations it
//! names; and a unit over it whose body states the guards, resets, time
//! invariants and claims (ADR-026 §11's `Rpc` and the other FR-239 models).

use std::collections::BTreeMap;

use qsl_eval::simulation::model::{
    ExpansionLimits, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, meaning, package_input};
use qsl_semantics::model::key::hex;
use qsl_semantics::model::observation::{DocumentRef, ObservationLimits};
use quire_checked::key::DeclarationKey;
use quire_exact::ScalarLimits;
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value as Json};

pub(crate) const PACKAGE: &str = "test/timed";
const PLACEHOLDER: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

pub(crate) fn ident(suffix: &str) -> String {
    format!("ix://{PACKAGE}/{suffix}")
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

fn wire_field(name: &str, type_ref: &str) -> Json {
    let identity = ident(&format!("Call/{name}"));
    let unique = type_ref != "ix://quire/native/Clock";
    json!({
        "identity": identity, "name": name, "typeRef": type_ref, "presence": "required",
        "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": unique},
        "origin": origin(&identity),
    })
}

fn phase_type() -> Json {
    let identity = ident("Phase");
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

fn operation(name: &str, modifies: &[&str], unbounded: bool) -> Json {
    let identity = ident(&format!("Call/{name}"));
    // With `unbounded`, a parameter `n: Integer`, an unbounded root.
    let params: Vec<Json> = unbounded
        .then(|| {
            let parameter = format!("{identity}/n");
            json!({
                "identity": parameter, "name": "n", "typeRef": "ix://quire/native/Integer",
                "presence": "required", "nullable": false, "defaultKind": "none",
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                "origin": origin(&parameter),
            })
        })
        .into_iter()
        .collect();
    json!({
        "identity": identity, "name": name, "params": params, "pre": [], "post": [],
        "origin": {"source": {"sourceIdentity": ident("spec"), "path": "spec.qspec",
            "startLine": 1, "startColumn": 1}},
        "frame": {
            "modifies": modifies.iter().map(|field| ident(&format!("Call/{field}"))).collect::<Vec<_>>(),
            "creates": [], "deletes": [],
        },
    })
}

/// The package with clock fields `clocks` and `operations` (name, the
/// fields it modifies).
pub(crate) fn document(clocks: &[&str], operations: &[(&str, &[&str])]) -> Vec<u8> {
    document_with(clocks, operations, false, false)
}

/// [`document`] whose population is declared `symmetric` (FR-151).
pub(crate) fn symmetric_document(clocks: &[&str], operations: &[(&str, &[&str])]) -> Vec<u8> {
    document_with(clocks, operations, false, true)
}

/// [`document`] whose first operation also takes `n: Integer`, an
/// unbounded population root (FR-126).
pub(crate) fn document_unbounded(clocks: &[&str], operations: &[(&str, &[&str])]) -> Vec<u8> {
    document_with(clocks, operations, true, false)
}

fn document_with(
    clocks: &[&str],
    operations: &[(&str, &[&str])],
    unbounded: bool,
    symmetric: bool,
) -> Vec<u8> {
    let call = ident("Call");
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
    let mut fields = vec![
        wire_field("phase", &ident("Phase")),
        wire_field("inflight", "ix://quire/native/Boolean"),
        wire_field("late", "ix://quire/native/Boolean"),
        wire_field("retried", "ix://quire/native/Boolean"),
    ];
    fields.extend(
        clocks
            .iter()
            .map(|name| wire_field(name, "ix://quire/native/Clock")),
    );
    let operations: Vec<Json> = operations
        .iter()
        .enumerate()
        .map(|(at, (name, modifies))| operation(name, modifies, unbounded && at == 0))
        .collect();
    let population = ident("calls");
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
            phase_type(),
            wire_type(&call, "object_type", json!({
                "supertypes": [], "fields": fields, "operations": operations,
                "relationships": [],
            })),
        ],
        "populations": [{
            "identity": population, "displayName": population,
            "kind": {"module": PACKAGE, "name": "population"},
            "members": [call], "extent": "closed", "origin": origin(&population),
            "symmetric": symmetric,
        }],
    })
    .to_string()
    .into_bytes()
}

/// The time units and a dense source for `M`, in ms.
pub(crate) const DENSE: &str = "dimension Time;\nunit s : Time = rational(1, 1);\n\
     unit ms : Time = rational(1, 1000) * s;\ntime M dense unit ms;\n";

/// A checked package with its snapshot of `c` (`phase` 0, every Boolean
/// false).
pub(crate) struct World {
    pub(crate) package: CheckedPackage,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    initial: DocumentRef,
    keys: Vec<String>,
}

/// `document` checked with `body` as its unit, under the model and the
/// complete temporal (`t`) profiles and the header lines `profiles`.
pub(crate) fn world(document: &[u8], profiles: &str, body: &str) -> World {
    world_over(document, profiles, body, &["c"])
}

/// [`world`] over the universe `keys`, each object in the snapshot with
/// `phase` 0 and every Boolean false.
pub(crate) fn world_over(document: &[u8], profiles: &str, body: &str, keys: &[&str]) -> World {
    let packages = package_input([document]);
    let digest = *packages.keys().next().expect("one model");
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         profile t = \"quire.temporal.complete/v1\";\n{profiles}\
         model M = \"{PACKAGE}\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n{body}",
        hex(&digest)
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "fr-239", "fixture", "1"),
        "fr-239.native",
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
    let graph = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        built,
        models,
        Vec::new(),
    )
    .expect("the unit assembles")
    .with_unwritten_timed_members()
    .check(CheckingLimits::default())
    .unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}"));
    let header = json!({"identity": PACKAGE, "version": "1.0.0",
        "digest": format!("sha256-jcs:{}", hex(&digest))});
    let snapshot = json!({
        "format": "quire.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "s0", "revision_namespace": "fixture",
            "revision": "fixture:1"},
        "observation": "current", "model": header,
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{"population": ident("calls"), "complete": true,
            "objects": keys.iter().map(|key| json!({
                "key": key, "type": ident("Call"),
                "fields": {"phase": {"integer": "0"}, "inflight": {"boolean": false},
                    "late": {"boolean": false}, "retried": {"boolean": false}},
            })).collect::<Vec<_>>()}],
    });
    let bytes = snapshot.to_string().into_bytes();
    let read = quire_canonical::read(&bytes, u64::MAX).expect("JSON fixture");
    let snapshot_digest = *quire_canonical::sha256(&read, quire_canonical::Limits::new(u64::MAX))
        .expect("canonical fixture")
        .as_bytes();
    World {
        package: CheckedPackage::link(graph),
        packages,
        snapshots: BTreeMap::from([(snapshot_digest, bytes)]),
        initial: DocumentRef {
            authority: "test".to_owned(),
            identity: "s0".to_owned(),
            revision_namespace: "fixture".to_owned(),
            revision: "fixture:1".to_owned(),
            digest: snapshot_digest,
        },
        keys: keys.iter().map(|key| (*key).to_owned()).collect(),
    }
}

const METER: ScalarLimits = ScalarLimits {
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

impl World {
    /// The model system over the world's universe.
    pub(crate) fn system(&self) -> ModelSystem<'_> {
        ModelSystem::new(self.request()).expect("the system admits")
    }

    /// FR-120's request of [`Self::system`].
    pub(crate) fn request(&self) -> ModelSimulationRequest<'_> {
        ModelSimulationRequest {
            package: &self.package,
            packages: &self.packages,
            normalization: ModelNormalizationLimits::default(),
            snapshots: &self.snapshots,
            initial: vec![self.initial.clone()],
            universes: vec![PopulationUniverse {
                population: DeclarationKey {
                    package: PACKAGE.to_owned(),
                    node: ident("calls"),
                },
                keys: self.keys.clone(),
            }],
            observation: ObservationLimits::default(),
            expansion: ExpansionLimits {
                max_candidates: 1_000_000,
            },
            meter: METER,
        }
    }
}
