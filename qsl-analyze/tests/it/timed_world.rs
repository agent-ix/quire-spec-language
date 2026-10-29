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
pub(crate) const PLACEHOLDER: &str =
    "sha256:0000000000000000000000000000000000000000000000000000000000000000";

pub(crate) fn ident(suffix: &str) -> String {
    format!("ix://{PACKAGE}/{suffix}")
}

fn origin(identity: &str) -> Json {
    json!({"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0",
        "inputIdentities": [identity]}})
}

pub(crate) fn wire_type(identity: &str, kind: &str, extra: Json) -> Json {
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

pub(crate) fn wire_field(name: &str, type_ref: &str) -> Json {
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

/// With `scoped`, the frame modifies `modifies` on the receiver only
/// (`modifies_receiver`), as each object of a symmetric population needs.
fn operation(name: &str, modifies: &[&str], unbounded: bool, scoped: bool) -> Json {
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
    let fields: Vec<String> = modifies
        .iter()
        .map(|field| ident(&format!("Call/{field}")))
        .collect();
    json!({
        "identity": identity, "name": name, "params": params, "pre": [], "post": [],
        "origin": {"source": {"sourceIdentity": ident("spec"), "path": "spec.qspec",
            "startLine": 1, "startColumn": 1}},
        "frame": {
            "modifies": if scoped { Vec::new() } else { fields.clone() },
            "modifies_receiver": if scoped { fields } else { Vec::new() },
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
        .map(|(at, (name, modifies))| operation(name, modifies, unbounded && at == 0, symmetric))
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

/// [`document`] with the record `Timer { x: Clock; n: Phase }` and the
/// field `ts: Timer [0, max]`, a sequence (FR-230 "Clock collections").
pub(crate) fn timers_document(max: u64, operations: &[(&str, &[&str])]) -> Vec<u8> {
    let mut document: Json = serde_json::from_slice(&document(&[], operations)).expect("JSON");
    let timer = ident("Timer");
    let member = |name: &str, type_ref: &str| {
        let identity = format!("{timer}/{name}");
        json!({
            "identity": identity, "name": name, "typeRef": type_ref, "presence": "required",
            "nullable": false, "defaultKind": "none",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "origin": origin(&identity),
        })
    };
    document["constructs"]
        .as_array_mut()
        .expect("constructs")
        .push(
            json!({"kind": {"module": PACKAGE, "name": "value_record"}, "moduleVersion": "1.0.0",
            "manifestDigest": PLACEHOLDER,
            "construct": {"identity": "none", "shape": "record", "members": {},
                "meaning": meaning::RECORD_VALUE_TYPE}}),
        );
    let types = document["types"].as_array_mut().expect("types");
    types.push(wire_type(
        &timer,
        "value_record",
        json!({"fields": [member("x", "ix://quire/native/Clock"), member("n", &ident("Phase"))]}),
    ));
    let mut ts = wire_field("ts", &timer);
    ts["multiplicity"] = json!({"lower": 0, "upper": max, "ordered": true, "unique": false});
    types[1]["fields"]
        .as_array_mut()
        .expect("Call fields")
        .push(ts);
    document.to_string().into_bytes()
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

/// [`world`] whose `c` also holds the fields `extra`.
pub(crate) fn world_holding(document: &[u8], profiles: &str, body: &str, extra: &Json) -> World {
    world_built(document, profiles, body, &["c"], &[], extra)
}

/// [`world`] over the universe `keys`, each object in the snapshot with
/// `phase` 0 and every Boolean false.
pub(crate) fn world_over(document: &[u8], profiles: &str, body: &str, keys: &[&str]) -> World {
    world_armed(document, profiles, body, keys, &[])
}

/// [`world_over`] whose objects `armed` start with `inflight` true.
pub(crate) fn world_armed(
    document: &[u8],
    profiles: &str,
    body: &str,
    keys: &[&str],
    armed: &[&str],
) -> World {
    world_built(document, profiles, body, keys, armed, &json!({}))
}

/// [`world_armed`] whose objects also hold the fields `extra`.
pub(crate) fn world_built(
    document: &[u8],
    profiles: &str,
    body: &str,
    keys: &[&str],
    armed: &[&str],
    extra: &Json,
) -> World {
    let packages = package_input([document]);
    let digest = *packages.keys().next().expect("one model");
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         profile t = \"quire.temporal.complete/v1\";\n{profiles}\
         model M = \"{PACKAGE}\";\n{body}",
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
    let mut fields = json!({"phase": {"integer": "0"}, "inflight": {"boolean": false},
        "late": {"boolean": false}, "retried": {"boolean": false}});
    for (name, value) in extra.as_object().expect("extra fields") {
        fields[name] = value.clone();
    }
    let snapshot = json!({
        "format": "quire.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "s0", "revision_namespace": "fixture",
            "revision": "fixture:1"},
        "observation": "current", "model": header,
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{"population": ident("calls"), "complete": true,
            "objects": keys.iter().map(|key| {
                let mut fields = fields.clone();
                fields["inflight"] = json!({"boolean": armed.contains(key)});
                json!({"key": key, "type": ident("Call"), "fields": fields})
            }).collect::<Vec<_>>()}],
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

/// `document` (a package of this fixture) as the package `package`: every
/// identity under `test/timed` moved under `package`, for a second model of
/// one unit.
pub(crate) fn renamed_package(document: &[u8], package: &str) -> Vec<u8> {
    String::from_utf8(document.to_vec())
        .expect("the fixture writes UTF-8")
        .replace(PACKAGE, package)
        .into_bytes()
}

/// A unit over two packages of this fixture and its snapshots: the model
/// `A` (`abstract_package`) and the model `M` (`test/timed`), each with the
/// one object `c` (`phase` 0, every Boolean false).
pub(crate) struct Refining {
    pub(crate) package: CheckedPackage,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    /// `M`'s initial states, in index order.
    concrete_initials: Vec<DocumentRef>,
    /// `A`'s initial state.
    pub(crate) abstract_initial: DocumentRef,
}

/// The unit over the abstract model `A`, the package `abstract_document`
/// names, and the concrete model `M`, `concrete_document`, with `body`
/// under the model, temporal and infinite-trace (`inf`) profiles; its
/// refusals when S3 refuses it, each with the text at its region.
pub(crate) fn refining(
    abstract_document: &[u8],
    concrete_document: &[u8],
    body: &str,
) -> Result<Refining, Vec<(String, String, String)>> {
    refining_from(abstract_document, concrete_document, body, false)
}

/// [`refining`] whose abstract object `c` starts with `inflight` equal to
/// `abstract_inflight`.
pub(crate) fn refining_from(
    abstract_document: &[u8],
    concrete_document: &[u8],
    body: &str,
    abstract_inflight: bool,
) -> Result<Refining, Vec<(String, String, String)>> {
    let packages = package_input([abstract_document, concrete_document]);
    let digest_of = |document: &[u8]| {
        packages
            .iter()
            .find(|(_, bytes)| bytes.as_slice() == document)
            .map(|(digest, _)| hex(digest))
            .expect("the package is provisioned")
    };
    let abstract_package = serde_json::from_slice::<Json>(abstract_document).expect("JSON")
        ["package"]["identity"]
        .as_str()
        .expect("an identity")
        .to_owned();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         profile t = \"quire.temporal.complete/v1\";\n\
         profile inf = \"quire.temporal.infinite-trace/v1\";\n\
         model A = \"{abstract_package}\";\n\
         model M = \"{PACKAGE}\";\n{body}",
        digest_of(abstract_document),
        digest_of(concrete_document),
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "fr-430", "fixture", "1"),
        "fr-430.native",
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
    .expect("the model selections admit")
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
    .expect("the unit assembles")
    .with_unwritten_timed_members()
    .with_unwritten_refinements();
    let regions = declarations.regions();
    let graph = declarations
        .check(CheckingLimits::default())
        .map_err(|refusals| {
            refusals
                .into_iter()
                .map(|refusal| {
                    let at = regions
                        .refusal_region(&refusal)
                        .map_or_else(String::new, |region| {
                            let start = usize::try_from(region.start()).expect("a start");
                            let end = usize::try_from(region.end()).expect("an end");
                            unit[start..end].to_owned()
                        });
                    (
                        refusal.cause.code().as_str().to_owned(),
                        refusal.cause.cause().unwrap_or_default().to_owned(),
                        at,
                    )
                })
                .collect::<Vec<_>>()
        })?;
    let mut snapshots = BTreeMap::new();
    let abstract_initial = snapshot(
        &packages,
        &mut snapshots,
        (&abstract_package, abstract_document),
        "a0",
        [abstract_inflight, false, false],
    );
    let concrete_initial = snapshot(
        &packages,
        &mut snapshots,
        (PACKAGE, concrete_document),
        "m0",
        [false; 3],
    );
    Ok(Refining {
        package: CheckedPackage::link(graph),
        packages,
        snapshots,
        concrete_initials: vec![concrete_initial],
        abstract_initial,
    })
}

/// The snapshot `name` of the model `package` (its `document`) whose one
/// object `c` has `phase` 0 and `inflight`, `late` and `retried` as
/// `flags` gives them, stored in `snapshots`.
fn snapshot(
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    snapshots: &mut BTreeMap<[u8; 32], Vec<u8>>,
    (package, document): (&str, &[u8]),
    name: &str,
    [inflight, late, retried]: [bool; 3],
) -> DocumentRef {
    let digest = packages
        .iter()
        .find(|(_, bytes)| bytes.as_slice() == document)
        .map(|(digest, _)| hex(digest))
        .expect("the package is provisioned");
    let identity = |suffix: &str| format!("ix://{package}/{suffix}");
    let snapshot = json!({
        "format": "quire.state.snapshot/v1",
        "identity": {"authority": "test", "identity": name, "revision_namespace": "fixture",
            "revision": "fixture:1"},
        "observation": "current",
        "model": {"identity": package, "version": "1.0.0",
            "digest": format!("sha256-jcs:{digest}")},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{"population": identity("calls"), "complete": true,
            "objects": [{"key": "c", "type": identity("Call"),
                "fields": {"phase": {"integer": "0"}, "inflight": {"boolean": inflight},
                    "late": {"boolean": late}, "retried": {"boolean": retried}}}]}],
    });
    let bytes = snapshot.to_string().into_bytes();
    let read = quire_canonical::read(&bytes, u64::MAX).expect("JSON fixture");
    let digest = *quire_canonical::sha256(&read, quire_canonical::Limits::new(u64::MAX))
        .expect("canonical fixture")
        .as_bytes();
    snapshots.insert(digest, bytes);
    DocumentRef {
        authority: "test".to_owned(),
        identity: name.to_owned(),
        revision_namespace: "fixture".to_owned(),
        revision: "fixture:1".to_owned(),
        digest,
    }
}

impl Refining {
    /// This world with `M`'s initial states replaced by one per entry of
    /// `states`, in order: `c`'s `inflight`, `late` and `retried`.
    /// `concrete_document` is `M`'s package.
    pub(crate) fn with_concrete_initials(
        mut self,
        concrete_document: &[u8],
        states: &[[bool; 3]],
    ) -> Self {
        self.concrete_initials = states
            .iter()
            .enumerate()
            .map(|(index, &state)| {
                snapshot(
                    &self.packages,
                    &mut self.snapshots,
                    (PACKAGE, concrete_document),
                    &format!("m{index}"),
                    state,
                )
            })
            .collect();
        self
    }

    /// FR-120's request of the concrete model `M` over the universe `{c}`.
    pub(crate) fn concrete_request(&self) -> ModelSimulationRequest<'_> {
        ModelSimulationRequest {
            package: &self.package,
            packages: &self.packages,
            normalization: ModelNormalizationLimits::default(),
            snapshots: &self.snapshots,
            initial: self.concrete_initials.clone(),
            universes: vec![PopulationUniverse {
                population: DeclarationKey {
                    package: PACKAGE.to_owned(),
                    node: ident("calls"),
                },
                keys: vec!["c".to_owned()],
            }],
            observation: ObservationLimits::default(),
            expansion: ExpansionLimits {
                max_candidates: 1_000_000,
            },
            meter: METER,
        }
    }
}
