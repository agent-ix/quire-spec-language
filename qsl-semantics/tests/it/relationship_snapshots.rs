// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-106 / FR-364: a snapshot's `relationships` member is read and admitted
//! as relationship extents (TC-1035 rows 1 to 22, 28, 30 to 33), and the
//! snapshot-built manifest and bundle (TC-1041 rows 18 to 20). Documents are
//! written as FR-106 shows them and mutated one row at a time; every
//! expected value comes from the test cases' tables.
//!
//! The `pipe` Connection of TC-1033's fixture is not built: a Connection is
//! a systems declaration, and admission treats it as any declaration that is
//! no navigation relationship (an object type's identity stands for it in
//! rows 9).

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{CheckedGraph, PackageDeclarations};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::domain_package::OperationEffect;
use qsl_semantics::model::intake::{admit_unit, meaning, package_input};
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::observation::{
    admit_observations, AdmissionFailure, AdmissionRecord, AdmittedObservations, AnchorKind,
    ClauseFacts, ClauseSelection, ClauseSelectionInput, DocumentRef, ObservationLimits,
    ObservationRole, Provisions, SelectedAnchor, SelectedObject,
};
use qsl_semantics::model::operation::OperationDeclaration;
use quire_exact::ObjectReference;
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::relationship_extent::Resolution;
use serde_json::{json, Value};

const PACKAGE: &str = "test/orders";
const DIGEST: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

fn identity(node: &str) -> String {
    format!("ix://{PACKAGE}/{node}")
}

fn key(node: &str) -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE.to_owned(),
        node: identity(node),
    }
}

// ---------------------------------------------------------------------------
// The orders package
// ---------------------------------------------------------------------------

/// The variants of fixture **orders** the rows use.
#[derive(Clone)]
struct Variant {
    /// `unique` of the composition's source and target ends.
    composition_unique: (bool, bool),
    /// The populations `(name, member types)`; `Q` is always first.
    populations: Vec<(&'static str, Vec<&'static str>)>,
    /// Whether the unit declares the `rehome` clauses; a second population
    /// over `Pump` makes their population ambiguous.
    clauses: bool,
    /// The identities `rehome`'s frame declares in `modifies`.
    rehome_modifies: Vec<String>,
    /// The identities `rehome`'s frame declares in `creates`.
    rehome_creates: Vec<String>,
}

impl Variant {
    fn orders() -> Self {
        Self {
            composition_unique: (true, true),
            populations: vec![("Q", vec!["Sys", "Pump", "Tank"])],
            clauses: true,
            rehome_modifies: Vec::new(),
            rehome_creates: Vec::new(),
        }
    }

    /// `orders` whose `rehome` frame declares `modifies` and `creates`.
    fn declaring(modifies: &[String], creates: &[&str]) -> Self {
        Self {
            rehome_modifies: modifies.to_vec(),
            rehome_creates: creates.iter().map(|name| identity(name)).collect(),
            ..Self::orders()
        }
    }

    fn bag() -> Self {
        Self {
            composition_unique: (false, false),
            ..Self::orders()
        }
    }

    fn bag_target() -> Self {
        Self {
            composition_unique: (true, false),
            ..Self::orders()
        }
    }

    /// `Q` over `[Sys, Pump]` and `Tanks` over `[Tank]`.
    fn tanks() -> Self {
        Self {
            populations: vec![("Q", vec!["Sys", "Pump"]), ("Tanks", vec!["Tank"])],
            ..Self::orders()
        }
    }

    /// `Spare` over `[Pump]`.
    fn spare() -> Self {
        let mut variant = Self::orders();
        variant.populations.push(("Spare", vec!["Pump"]));
        variant.clauses = false;
        variant
    }

    /// Row 30: `Spare` over `[Pump]` and `Tanks` over `[Tank]` beside `Q`.
    fn spare_and_tanks() -> Self {
        let mut variant = Self::spare();
        variant.populations.push(("Tanks", vec!["Tank"]));
        variant
    }
}

fn provenance(identity: &str) -> Value {
    json!({
        "generated": {
            "generatorIdentity": identity,
            "generatorVersion": "1.0.0",
            "inputIdentities": [identity],
        }
    })
}

fn envelope(constructs: Value, types: Value, populations: Value) -> Value {
    json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1", "digest": "sha256-jcs"},
        "source": {
            "identity": identity("spec"), "version": "1.0.0",
            "dialect": "spec-bundle", "digest": DIGEST,
        },
        "package": {
            "identity": PACKAGE, "version": "1.0.0", "manifestDigest": DIGEST,
            "mappingVersions": [], "profileVersions": [], "lockDigest": DIGEST,
        },
        "occurrences": [],
        "extensions": [],
        "constructs": constructs,
        "types": types,
        "populations": populations,
    })
}

fn construct(name: &str, meaning: &str) -> Value {
    json!({
        "kind": {"module": PACKAGE, "name": name},
        "moduleVersion": "1.0.0",
        "manifestDigest": DIGEST,
        "construct": {"identity": "none", "shape": "record", "members": {}, "meaning": meaning},
    })
}

fn object_type(name: &str, extra: Value) -> Value {
    let node = identity(name);
    let mut value = json!({
        "identity": node,
        "displayName": node,
        "kind": {"module": PACKAGE, "name": "object_type"},
        "roles": [],
        "origin": provenance(&node),
        "constraints": [],
        "extensions": [],
        "unknownPolicy": "reject",
        "supertypes": [],
        "fields": [],
        "operations": [],
        "relationships": [],
    });
    if let (Some(value), Some(extra)) = (value.as_object_mut(), extra.as_object()) {
        for (member, entry) in extra {
            value.insert(member.clone(), entry.clone());
        }
    }
    value
}

fn end(object: &str, role: &str, lower: u64, upper: u64, unique: bool) -> Value {
    json!({
        "type": identity(object),
        "role": role,
        "multiplicity": {"lower": lower, "upper": upper, "ordered": false, "unique": unique},
    })
}

fn relationship(name: &str, source: Value, target: Value) -> Value {
    json!({
        "identity": identity(&format!("relationship/{name}")),
        "sourceEnd": source,
        "targetEnd": target,
        "category": "structural",
        "composite": false,
        "direction": "source-to-target",
        "origin": {"source": {
            "sourceIdentity": identity("spec"), "path": "spec.qspec",
            "startLine": 1, "startColumn": 1,
        }},
    })
}

fn rehome(variant: &Variant) -> Value {
    json!({
        "identity": identity("Pump/rehome"),
        "name": "rehome",
        "params": [],
        "pre": [],
        "post": [],
        "origin": {"source": {
            "sourceIdentity": identity("spec"), "path": "spec.qspec",
            "startLine": 1, "startColumn": 1,
        }},
        "frame": {
            "modifies": variant.rehome_modifies,
            "creates": variant.rehome_creates,
            "deletes": [],
        },
    })
}

fn package(variant: &Variant) -> Vec<u8> {
    let backup = identity("Pump/backup");
    let sys = object_type(
        "Sys",
        json!({"relationships": [relationship(
            "Sys-composition-Pump",
            end("Sys", "parts", 1, 1, variant.composition_unique.0),
            end("Pump", "owner", 0, 3, variant.composition_unique.1),
        )]}),
    );
    let pump = object_type(
        "Pump",
        json!({
            "fields": [{
                "identity": backup, "name": "backup", "typeRef": identity("Pump"),
                "presence": "optional", "nullable": false, "defaultKind": "none",
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                "origin": provenance(&backup),
            }],
            "operations": [rehome(variant)],
            "relationships": [relationship(
                "Pump-housing-Tank",
                end("Pump", "home", 1, 1, true),
                end("Tank", "pump", 1, 1, true),
            )],
        }),
    );
    let tank = object_type("Tank", json!({}));
    let populations: Vec<Value> = variant
        .populations
        .iter()
        .map(|(name, members)| {
            let node = identity(name);
            json!({
                "identity": node,
                "displayName": node,
                "kind": {"module": PACKAGE, "name": "population"},
                "members": members.iter().map(|member| identity(member)).collect::<Vec<_>>(),
                "extent": "closed",
                "origin": provenance(&node),
            })
        })
        .collect();
    serde_json::to_vec(&envelope(
        json!([
            construct("object_type", meaning::OBJECT_TYPE),
            construct("population", meaning::POPULATION),
        ]),
        json!([sys, pump, tank]),
        json!(populations),
    ))
    .expect("valid JSON")
}

// ---------------------------------------------------------------------------
// Documents
// ---------------------------------------------------------------------------

fn label(name: &str) -> DocumentRef {
    DocumentRef {
        authority: "test".to_owned(),
        identity: name.to_owned(),
        revision_namespace: "fixture".to_owned(),
        revision: "fixture:1".to_owned(),
        digest: [0; 32],
    }
}

fn labels(reference: &DocumentRef) -> Value {
    json!({
        "authority": reference.authority, "identity": reference.identity,
        "revision_namespace": reference.revision_namespace, "revision": reference.revision,
    })
}

/// `value` as a provision: its reference with the recomputed digest, and its
/// bytes.
fn provision(reference: &DocumentRef, value: &Value) -> (DocumentRef, Vec<u8>) {
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

fn object_json(key: &str, type_name: &str) -> Value {
    let fields = if type_name == "Pump" {
        json!({"backup": {"absent": {}}})
    } else {
        json!({})
    };
    json!({"key": key, "type": identity(type_name), "fields": fields})
}

fn population_json(name: &str, complete: bool, objects: &[(&str, &str)]) -> Value {
    json!({
        "population": identity(name),
        "complete": complete,
        "objects": objects.iter().map(|(key, type_name)| object_json(key, type_name)).collect::<Vec<_>>(),
    })
}

fn end_json(population: &str, key: &str) -> Value {
    json!({"population": identity(population), "key": key})
}

fn instance_json(source: (&str, &str), target: (&str, &str)) -> Value {
    json!({"source": end_json(source.0, source.1), "target": end_json(target.0, target.1)})
}

/// An instance within population `Q`.
fn link(source: &str, target: &str) -> Value {
    instance_json(("Q", source), ("Q", target))
}

fn entry(relationship: &str, complete: bool, instances: Vec<Value>) -> Value {
    json!({
        "relationship": identity(&format!("relationship/{relationship}")),
        "complete": complete,
        "instances": instances,
    })
}

const COMPOSITION: &str = "Sys-composition-Pump";
const HOUSING: &str = "Pump-housing-Tank";

/// Base snapshot **B**.
fn base() -> Value {
    snapshot(
        &[population_json(
            "Q",
            true,
            &[
                ("s1", "Sys"),
                ("s2", "Sys"),
                ("p1", "Pump"),
                ("p2", "Pump"),
                ("p3", "Pump"),
                ("t1", "Tank"),
            ],
        )],
        Some(vec![
            entry(COMPOSITION, true, vec![link("s1", "p2"), link("s1", "p1")]),
            entry(HOUSING, true, vec![link("p1", "t1"), link("p2", "t1")]),
        ]),
    )
}

fn snapshot(populations: &[Value], relationships: Option<Vec<Value>>) -> Value {
    let mut document = json!({
        "format": "quire.state.snapshot/v1",
        "identity": labels(&label("current")),
        "observation": "current",
        "anchor": {"kind": "handler", "name": "validate"},
        "model": {"identity": PACKAGE, "version": "1.0.0", "digest": "set at admission"},
        "populations": populations,
    });
    if let Some(relationships) = relationships {
        document["relationships"] = json!(relationships);
    }
    document
}

/// The entry for `relationship` of `document`, mutably.
fn entry_mut<'d>(document: &'d mut Value, relationship: &str) -> &'d mut Value {
    let wanted = identity(&format!("relationship/{relationship}"));
    document["relationships"]
        .as_array_mut()
        .expect("relationships")
        .iter_mut()
        .find(|entry| entry["relationship"] == wanted)
        .expect("the entry is listed")
}

fn set_instances(document: &mut Value, relationship: &str, instances: Vec<Value>) {
    entry_mut(document, relationship)["instances"] = json!(instances);
}

// ---------------------------------------------------------------------------
// Admission
// ---------------------------------------------------------------------------

struct Admission {
    graph: CheckedGraph,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
    model_digest: [u8; 32],
}

fn checked(variant: &Variant) -> Admission {
    let document = package(variant);
    let packages = package_input([document.as_slice()]);
    let model_digest = *packages.keys().next().expect("one model");
    let digest = qsl_semantics::model::key::hex(&model_digest);
    let clauses = if variant.clauses {
        "pre RehomePre using v on M::Pump::rehome { true }\n\
         post RehomePost using v on M::Pump::rehome { true }\n"
    } else {
        ""
    };
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model M = {PACKAGE:?} version \"1.0.0\" digest \"sha256-jcs:{digest}\";\n\
         invariant SysOk using v on M::Sys at current {{ true }}\n{clauses}"
    );
    let built = crate::model_operations::parse_and_build(&unit);
    let models = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .unwrap_or_else(|refusal| panic!("I1 refused: {refusal:?}"));
    let raw = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "tc-1035", "fixture", "fixture:1"),
        "unit.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit")
    .source()
    .reference()
    .clone();
    let graph = PackageDeclarations::assemble(raw, built, models, Vec::new())
        .expect("the model assembles")
        .check(CheckingLimits::default())
        .expect("the unit checks");
    Admission {
        graph,
        packages,
        model_digest,
    }
}

impl Admission {
    /// Admits `document` as the current snapshot for `SysOk` with `self`
    /// `{Q, s1}`.
    fn admit(
        &self,
        mut document: Value,
        limits: ObservationLimits,
    ) -> (Result<AdmittedObservations, AdmissionFailure>, DocumentRef) {
        document["model"]["digest"] = json!(format!(
            "sha256-jcs:{}",
            qsl_semantics::model::key::hex(&self.model_digest)
        ));
        let (snapshot, bytes) = provision(&label("current"), &document);
        let snapshots = BTreeMap::from([(snapshot.digest, bytes)]);
        let invocations = BTreeMap::new();
        let clause = self.graph.state_clause("SysOk").expect("SysOk is checked");
        let facts = ClauseFacts {
            identity: clause.identity(),
            kind: clause.kind(),
            context: clause.context(),
            operation: None,
        };
        let admitted = admit_observations(
            self.graph.model_selections(),
            self.graph.scope().types(),
            &facts,
            &self.packages,
            ModelNormalizationLimits::default(),
            &Provisions {
                snapshots: &snapshots,
                invocations: &invocations,
            },
            &ClauseSelection {
                name: "SysOk".to_owned(),
                input: ClauseSelectionInput::Current {
                    snapshot: snapshot.clone(),
                    anchor: SelectedAnchor {
                        kind: AnchorKind::Handler,
                        name: "validate".to_owned(),
                    },
                    self_object: SelectedObject {
                        population: identity("Q"),
                        key: "s1".to_owned(),
                    },
                },
            },
            limits,
        );
        (admitted, snapshot)
    }
}

fn admit(variant: &Variant, document: Value) -> Result<AdmittedObservations, AdmissionFailure> {
    checked(variant)
        .admit(document, ObservationLimits::default())
        .0
}

fn refusal(variant: &Variant, document: Value) -> AdmissionRecord {
    match admit(variant, document) {
        Err(AdmissionFailure::Refused(record)) => record,
        other => panic!("a refusal, not {other:?}"),
    }
}

fn incomplete(variant: &Variant, document: Value) -> AdmissionRecord {
    match admit(variant, document) {
        Err(AdmissionFailure::Incomplete(record)) => record,
        other => panic!("an incomplete outcome, not {other:?}"),
    }
}

/// An admitted current observation, read through the universes its
/// admission assigned (the three object types are unrelated, so each has its
/// own).
struct Reader {
    admitted: AdmittedObservations,
    universes: BTreeMap<char, quire_exact::UniverseId>,
}

/// Admits `document` over `variant` and wraps the observation for reading.
fn read(variant: &Variant, document: Value) -> Reader {
    let admission = checked(variant);
    let admitted = admission
        .admit(document, ObservationLimits::default())
        .0
        .expect("the document admits");
    let universes = [('s', "Sys"), ('p', "Pump"), ('t', "Tank")]
        .into_iter()
        .map(|(initial, type_name)| {
            let universe = qsl_semantics::model::observation::population_universe_for(
                admission.graph.model_selections(),
                &admission.packages,
                ModelNormalizationLimits::default(),
                crate::relationship_navigation::object(&admission.graph, type_name),
            )
            .expect("the type has a universe");
            (initial, universe)
        })
        .collect();
    Reader {
        admitted,
        universes,
    }
}

impl Reader {
    fn extents(&self) -> &qsl_semantics::model::observation::RelationshipExtents {
        self.admitted
            .current
            .as_ref()
            .expect("a current observation")
            .environment
            .extents()
            .expect("extents")
    }

    /// The object `object`; its first letter names its type (`s`, `p`, `t`).
    fn object(&self, object: &str) -> ObjectReference {
        let initial = object.chars().next().expect("a key");
        self.admitted
            .current
            .as_ref()
            .expect("a current observation")
            .environment
            .objects()
            .find(self.universes[&initial], object)
            .unwrap_or_else(|| panic!("{object} is held"))
            .clone()
    }

    fn targets(&self, relationship: &str, source: &str) -> Vec<ObjectReference> {
        self.extents()
            .targets(
                &key(&format!("relationship/{relationship}")),
                &self.object(source),
            )
            .cloned()
            .collect()
    }

    fn sources(&self, relationship: &str, target: &str) -> Vec<ObjectReference> {
        self.extents()
            .sources(
                &key(&format!("relationship/{relationship}")),
                &self.object(target),
            )
            .cloned()
            .collect()
    }

    fn complete(&self, relationship: &str) -> bool {
        self.extents()
            .complete(&key(&format!("relationship/{relationship}")))
    }
}

fn field<'r>(record: &'r AdmissionRecord, name: &str) -> Option<&'r str> {
    record.fields.get(name).map(String::as_str)
}

fn assert_refusal(record: &AdmissionRecord, code: Code, cause: &str) {
    assert_eq!(record.code, code, "{record:?}");
    assert_eq!(record.cause, cause, "{record:?}");
}

/// TC-1035 rows 1 and 2: B admits, and reordering the entries, the instances
/// and the members gives the same reads.
#[trace("TC-1035", "FR-106-AC-23")]
#[test]
fn a_snapshot_admits_its_relationships_as_extents_in_key_order() {
    let admitted = read(&Variant::orders(), base());
    let (p1, p2, s1) = (
        admitted.object("p1"),
        admitted.object("p2"),
        admitted.object("s1"),
    );
    assert!(admitted.complete(COMPOSITION));
    assert!(admitted.complete(HOUSING));
    assert_eq!(
        admitted.targets(COMPOSITION, "s1"),
        vec![p1.clone(), p2.clone()]
    );
    assert_eq!(admitted.sources(COMPOSITION, "p1"), vec![s1]);
    assert_eq!(admitted.sources(HOUSING, "t1"), vec![p1, p2]);

    // Row 2: entries, instances and JSON members reordered.
    let mut reordered = base();
    reordered["relationships"]
        .as_array_mut()
        .expect("relationships")
        .reverse();
    for relationship in [COMPOSITION, HOUSING] {
        entry_mut(&mut reordered, relationship)["instances"]
            .as_array_mut()
            .expect("instances")
            .reverse();
    }
    let swapped = read(&Variant::orders(), reordered);
    assert_eq!(
        swapped.targets(COMPOSITION, "s1"),
        admitted.targets(COMPOSITION, "s1")
    );
    assert_eq!(
        swapped.sources(HOUSING, "t1"),
        admitted.sources(HOUSING, "t1")
    );
}

/// TC-1035 rows 3 and 4: no `relationships` member lists none; an empty
/// complete entry is a complete extent with no instance.
#[trace("TC-1035", "FR-106-AC-23")]
#[test]
fn an_unlisted_relationship_and_an_empty_complete_entry_differ() {
    let mut without = base();
    without
        .as_object_mut()
        .expect("object")
        .remove("relationships");
    let admitted = read(&Variant::orders(), without);
    assert!(!admitted.complete(COMPOSITION));
    assert!(!admitted.complete(HOUSING));

    let mut empty = base();
    *entry_mut(&mut empty, COMPOSITION) = entry(COMPOSITION, true, vec![]);
    let admitted = read(&Variant::orders(), empty);
    assert!(admitted.complete(COMPOSITION));
    assert_eq!(admitted.targets(COMPOSITION, "s1"), vec![]);
    assert!(admitted.complete(HOUSING));
}

/// TC-1035 row 5: no multiplicity comparison.
#[trace("TC-1035", "FR-106-AC-23")]
#[test]
fn a_snapshot_compares_no_count_with_a_multiplicity_bound() {
    let mut document = base();
    document["populations"][0]["objects"]
        .as_array_mut()
        .expect("objects")
        .push(object_json("p4", "Pump"));
    set_instances(
        &mut document,
        COMPOSITION,
        vec![
            link("s1", "p1"),
            link("s1", "p2"),
            link("s1", "p3"),
            link("s1", "p4"),
        ],
    );
    set_instances(
        &mut document,
        HOUSING,
        vec![link("p1", "t1"), link("p2", "t1")],
    );
    let admitted = read(&Variant::orders(), document);
    assert_eq!(admitted.targets(COMPOSITION, "s1").len(), 4);
}

/// TC-1035 rows 6 to 8: check 6.6.
#[trace("TC-1035", "FR-106-AC-24")]
#[test]
fn check_6_6_refuses_a_malformed_entry() {
    let mut missing = base();
    entry_mut(&mut missing, COMPOSITION)
        .as_object_mut()
        .expect("entry")
        .remove("instances");
    let record = refusal(&Variant::orders(), missing);
    assert_refusal(&record, Code::InvalidRuntimeInput, "missing-member");
    assert_eq!(field(&record, "member"), Some("instances"));

    let mut extra = base();
    entry_mut(&mut extra, COMPOSITION)["note"] = json!("x");
    let record = refusal(&Variant::orders(), extra);
    assert_refusal(&record, Code::InvalidRuntimeInput, "unknown-member");
    assert_eq!(field(&record, "member"), Some("note"));

    let mut wrong = base();
    entry_mut(&mut wrong, COMPOSITION)["complete"] = json!("yes");
    let record = refusal(&Variant::orders(), wrong);
    assert_refusal(&record, Code::InvalidRuntimeInput, "wrong-value-kind");
    assert_eq!(field(&record, "member"), Some("complete"));
}

/// TC-1035 rows 9 and 10: check 6.7.
#[trace("TC-1035", "FR-106-AC-24")]
#[test]
fn check_6_7_refuses_a_non_navigation_relationship_and_a_second_entry() {
    for named in [identity("Sys"), identity("relationship/Nope-x-Pump")] {
        let mut document = base();
        entry_mut(&mut document, COMPOSITION)["relationship"] = json!(named);
        let record = refusal(&Variant::orders(), document);
        assert_refusal(&record, Code::InvalidRuntimeInput, "wrong-role-mapping");
        assert_eq!(field(&record, "relationship"), Some(named.as_str()));
    }

    let mut twice = base();
    twice["relationships"]
        .as_array_mut()
        .expect("relationships")
        .push(entry(COMPOSITION, true, vec![]));
    let record = refusal(&Variant::orders(), twice);
    assert_refusal(&record, Code::InvalidRuntimeInput, "conflicting-identity");
    assert_eq!(
        field(&record, "relationship"),
        Some(identity(&format!("relationship/{COMPOSITION}")).as_str())
    );
}

/// TC-1035 rows 11 to 13: check 6.8.
#[trace("TC-1035", "FR-106-AC-24")]
#[test]
fn check_6_8_refuses_a_malformed_instance_and_an_end_of_the_wrong_type() {
    let composition = identity(&format!("relationship/{COMPOSITION}"));
    fn instance_zero(document: &mut Value) -> &mut Value {
        &mut entry_mut(document, COMPOSITION)["instances"][0]
    }

    let mut no_target = base();
    instance_zero(&mut no_target)
        .as_object_mut()
        .expect("instance")
        .remove("target");
    let mut extra = base();
    instance_zero(&mut extra)["note"] = json!("x");
    let mut numeric = base();
    instance_zero(&mut numeric)["source"]["key"] = json!(1);
    for (document, cause) in [
        (no_target, "missing-member"),
        (extra, "unknown-member"),
        (numeric, "wrong-value-kind"),
    ] {
        let record = refusal(&Variant::orders(), document);
        assert_refusal(&record, Code::InvalidRuntimeInput, cause);
        assert_eq!(field(&record, "relationship"), Some(composition.as_str()));
        assert_eq!(field(&record, "instance"), Some("0"));
    }

    let mut other = base();
    instance_zero(&mut other)["source"]["population"] = json!(identity("other"));
    let record = refusal(&Variant::orders(), other);
    assert_refusal(&record, Code::InvalidRuntimeInput, "wrong-role-mapping");
    assert_eq!(field(&record, "instance"), Some("0"));
    assert_eq!(field(&record, "end"), Some("source"));

    for (end, expected) in [("source", "source"), ("target", "target")] {
        let mut document = base();
        instance_zero(&mut document)[end]["key"] = json!("t1");
        let record = refusal(&Variant::orders(), document);
        assert_refusal(&record, Code::InvalidRuntimeInput, "wrong-value-kind");
        assert_eq!(field(&record, "relationship"), Some(composition.as_str()));
        assert_eq!(field(&record, "instance"), Some("0"));
        assert_eq!(field(&record, "end"), Some(expected));
    }
}

/// TC-1035 rows 14 and 15: check 6.9.
#[trace("TC-1035", "FR-106-AC-24")]
#[test]
fn check_6_9_refuses_a_repeat_unless_both_ends_are_not_unique() {
    let repeated = |variant: &Variant| {
        let mut document = base();
        set_instances(
            &mut document,
            COMPOSITION,
            vec![link("s1", "p2"), link("s1", "p1"), link("s1", "p2")],
        );
        (variant.clone(), document)
    };
    for variant in [Variant::orders(), Variant::bag_target()] {
        let (variant, document) = repeated(&variant);
        let record = refusal(&variant, document);
        assert_refusal(&record, Code::InvalidRuntimeInput, "conflicting-identity");
        assert_eq!(field(&record, "instance"), Some("2"));
    }
    let (variant, document) = repeated(&Variant::bag());
    let admitted = read(&variant, document);
    assert_eq!(
        admitted.targets(COMPOSITION, "s1"),
        vec![
            admitted.object("p1"),
            admitted.object("p2"),
            admitted.object("p2")
        ]
    );
}

/// TC-1035 rows 16, 32 and 33: the walk is entry by entry and instance by
/// instance.
#[trace("TC-1035", "FR-106-AC-24")]
#[test]
fn checks_run_entry_by_entry_and_instance_by_instance() {
    // Row 16: a wrong type at instance 0 precedes a bad population at 1.
    let mut sixteen = base();
    let mut second = link("s1", "p1");
    second["source"]["population"] = json!(identity("other"));
    set_instances(
        &mut sixteen,
        COMPOSITION,
        vec![instance_json(("Q", "s1"), ("Q", "t1")), second],
    );
    let record = refusal(&Variant::orders(), sixteen);
    assert_refusal(&record, Code::InvalidRuntimeInput, "wrong-value-kind");
    assert_eq!(field(&record, "instance"), Some("0"));

    // Row 32: a repeat at instance 1 precedes a wrong type at instance 2.
    let mut thirty_two = base();
    set_instances(
        &mut thirty_two,
        COMPOSITION,
        vec![link("s1", "p2"), link("s1", "p2"), link("s1", "t1")],
    );
    let record = refusal(&Variant::orders(), thirty_two);
    assert_refusal(&record, Code::InvalidRuntimeInput, "conflicting-identity");
    assert_eq!(field(&record, "instance"), Some("1"));

    // Row 33: instance 0 of entry 0 precedes a second entry.
    let mut thirty_three = base();
    set_instances(
        &mut thirty_three,
        COMPOSITION,
        vec![instance_json(("Q", "s1"), ("Q", "t1"))],
    );
    thirty_three["relationships"]
        .as_array_mut()
        .expect("relationships")
        .push(entry(COMPOSITION, true, vec![]));
    let record = refusal(&Variant::orders(), thirty_three);
    assert_refusal(&record, Code::InvalidRuntimeInput, "wrong-value-kind");
    assert_eq!(field(&record, "instance"), Some("0"));
    assert_eq!(field(&record, "end"), Some("target"));
}

/// TC-1035 rows 17 to 19 (and TC-1041 row 19): closure and completeness.
#[trace("TC-1035", "FR-106-AC-25", "TC-1041", "FR-364-AC-13")]
#[test]
fn a_dangling_instance_end_is_refused_at_check_8_and_an_incomplete_one_at_check_7() {
    // Row 17: housing p2-t9, t9 listed nowhere.
    let mut seventeen = base();
    set_instances(
        &mut seventeen,
        HOUSING,
        vec![link("p1", "t1"), link("p2", "t9")],
    );
    let record = refusal(&Variant::orders(), seventeen.clone());
    assert_refusal(
        &record,
        Code::DanglingReference,
        "absent-target-in-complete-population",
    );
    assert_eq!(field(&record, "instance"), Some("1"));
    assert_eq!(field(&record, "end"), Some("target"));
    assert_eq!(field(&record, "object"), Some("t9"));
    assert_eq!(field(&record, "population"), Some(identity("Q").as_str()));

    // Row 18: the same with Q incomplete is Incomplete naming Q.
    let mut eighteen = seventeen.clone();
    eighteen["populations"][0]["complete"] = json!(false);
    let record = incomplete(&Variant::orders(), eighteen);
    assert_refusal(&record, Code::IncompletePopulation, "incomplete-scope");
    assert_eq!(field(&record, "population"), Some(identity("Q").as_str()));

    // Row 19: a dangling `reference` value precedes the dangling instance end.
    let mut nineteen = seventeen;
    nineteen["populations"][0]["objects"][4]["fields"]["backup"] =
        json!({"present": {"reference": end_json("Q", "p9")}});
    let record = refusal(&Variant::orders(), nineteen);
    assert_refusal(
        &record,
        Code::DanglingReference,
        "absent-target-in-complete-population",
    );
    assert_eq!(field(&record, "object"), Some("p9"));
    assert_eq!(field(&record, "instance"), None);

    // TC-1041 row 19: a document end whose object is not listed has no type
    // to check, so the closure rule decides it, not condition 3.
    let mut document_end = base();
    set_instances(
        &mut document_end,
        COMPOSITION,
        vec![instance_json(("Q", "s1"), ("Q", "t9"))],
    );
    let record = refusal(&Variant::orders(), document_end.clone());
    assert_refusal(
        &record,
        Code::DanglingReference,
        "absent-target-in-complete-population",
    );
    document_end["populations"][0]["complete"] = json!(false);
    let record = incomplete(&Variant::orders(), document_end);
    assert_refusal(&record, Code::IncompletePopulation, "incomplete-scope");
    assert_eq!(field(&record, "population"), Some(identity("Q").as_str()));
}

/// TC-1035 rows 20 to 22, 30 and 31: an instance end makes a population
/// required; an unlisted key of a population that is not complete is
/// incomplete.
#[trace("TC-1035", "FR-106-AC-25")]
#[test]
fn an_instance_end_makes_its_other_endpoint_population_required() {
    // Row 20: Tanks (incomplete) holds t1; housing p1-t1 makes it required.
    let tanks = |with_housing: bool| {
        let mut document = snapshot(
            &[
                population_json(
                    "Q",
                    true,
                    &[
                        ("s1", "Sys"),
                        ("s2", "Sys"),
                        ("p1", "Pump"),
                        ("p2", "Pump"),
                        ("p3", "Pump"),
                    ],
                ),
                population_json("Tanks", false, &[("t1", "Tank")]),
            ],
            Some(vec![entry(COMPOSITION, true, vec![link("s1", "p1")])]),
        );
        if with_housing {
            document["relationships"]
                .as_array_mut()
                .expect("relationships")
                .push(entry(
                    HOUSING,
                    true,
                    vec![instance_json(("Q", "p1"), ("Tanks", "t1"))],
                ));
        }
        document
    };
    let record = incomplete(&Variant::tanks(), tanks(true));
    assert_refusal(&record, Code::IncompletePopulation, "incomplete-scope");
    assert_eq!(
        field(&record, "population"),
        Some(identity("Tanks").as_str())
    );
    // Row 21: without the housing entry no instance names Tanks.
    admit(&Variant::tanks(), tanks(false)).expect("row 21 admits");

    // Row 22: an extent listed incomplete admits.
    let mut incomplete_extent = base();
    *entry_mut(&mut incomplete_extent, COMPOSITION) =
        entry(COMPOSITION, false, vec![link("s1", "p1")]);
    let admitted = read(&Variant::orders(), incomplete_extent);
    assert!(!admitted.complete(COMPOSITION));

    // Rows 30 and 31: neither end is in a required population; t8 is
    // unlisted in the incomplete Tanks.
    let spare = |t8_listed: bool| {
        let mut document = base();
        document["populations"]
            .as_array_mut()
            .expect("populations")
            .extend([
                population_json("Spare", false, &[("p8", "Pump")]),
                population_json(
                    "Tanks",
                    false,
                    if t8_listed { &[("t8", "Tank")] } else { &[] },
                ),
            ]);
        entry_mut(&mut document, HOUSING)["instances"]
            .as_array_mut()
            .expect("instances")
            .push(instance_json(("Spare", "p8"), ("Tanks", "t8")));
        document
    };
    let record = incomplete(&Variant::spare_and_tanks(), spare(false));
    assert_refusal(&record, Code::IncompletePopulation, "incomplete-scope");
    assert_eq!(
        field(&record, "population"),
        Some(identity("Tanks").as_str())
    );
    admit(&Variant::spare_and_tanks(), spare(true)).expect("row 31 admits");
}

/// TC-1035 row 28: each instance is one value against `observation.values`.
#[trace("TC-1035", "FR-106-AC-27")]
#[test]
fn each_instance_counts_as_one_value_against_observation_values() {
    // B holds three `backup` values (p1, p2, p3) and four instances.
    let exactly = ObservationLimits {
        values_per_document: 7,
        ..ObservationLimits::default()
    };
    let admission = checked(&Variant::orders());
    admission
        .admit(base(), exactly)
        .0
        .expect("B at the bound admits");

    let mut one_more = base();
    entry_mut(&mut one_more, COMPOSITION)["instances"]
        .as_array_mut()
        .expect("instances")
        .push(link("s1", "p3"));
    let AdmissionFailure::Refused(record) = admission
        .admit(one_more, exactly)
        .0
        .expect_err("one more instance exceeds the bound")
    else {
        panic!("a refusal");
    };
    assert_refusal(&record, Code::StageLimitExceeded, "node-count-exceeded");
    assert_eq!(field(&record, "setting"), Some("observation.values"));
    assert_eq!(field(&record, "bound"), Some("7"));
    assert_eq!(field(&record, "actual"), Some("8"));
}

/// TC-1041 row 18: the snapshot-built manifest is closed with one row per
/// declared population, complete as the snapshot lists it, and every stored
/// end resolves as present.
#[trace("TC-1041", "FR-106-AC-39")]
#[test]
fn the_snapshot_built_manifest_is_closed_and_every_stored_end_is_present() {
    let variant = Variant::spare();
    let admission = checked(&variant);
    let source = qsl_foundation::source::provenance::RawSourceRef::new(
        "authority",
        "unit",
        qsl_foundation::digest::DigestRecord::mint(
            qsl_foundation::digest::DigestDomain::SourceBytesV1,
            [1; 32],
        ),
    )
    .expect("a source reference");
    let rows = |spare: Option<bool>| {
        let mut document = base();
        if let Some(complete) = spare {
            document["populations"]
                .as_array_mut()
                .expect("populations")
                .push(population_json("Spare", complete, &[]));
        }
        let reader = read(&variant, document);
        let types = admission.graph.scope().types();
        let bundle = reader
            .admitted
            .bundle(ObservationRole::Current, &source, types)
            .expect("a bundle");
        for (relationship, source_name) in [(COMPOSITION, "s1"), (HOUSING, "p1")] {
            for target in reader.targets(relationship, source_name) {
                assert_eq!(bundle.resolve(&target), Resolution::Present);
            }
        }
        let manifest = reader.extents().manifest(types).expect("a manifest");
        (
            manifest.state(),
            manifest
                .rows()
                .map(|row| (row.population().node.clone(), row.complete()))
                .collect::<Vec<_>>(),
        )
    };
    let (state, listed_rows) = rows(None);
    assert_eq!(
        state,
        quire_semantic_value::relationship_extent::ManifestState::Closed
    );
    assert_eq!(
        listed_rows,
        vec![(identity("Q"), true), (identity("Spare"), false)]
    );
    assert_eq!(
        rows(Some(false)).1,
        vec![(identity("Q"), true), (identity("Spare"), false)]
    );
    assert_eq!(
        rows(Some(true)).1,
        vec![(identity("Q"), true), (identity("Spare"), true)]
    );
}

// ---------------------------------------------------------------------------
// Invocation **I** of operation `rehome` (TC-1035 rows 23 to 27 and 29, and
// TC-1037's frame content)
// ---------------------------------------------------------------------------

/// A pre or post snapshot of invocation **I**: `Q` complete holding `s1`,
/// `p1`, `p2`, `t1` and `t2` (and `extra`), listing the given relationships.
fn side(observation: &str, extra: &[(&str, &str)], relationships: Vec<Value>) -> Value {
    let mut objects = vec![
        ("s1", "Sys"),
        ("p1", "Pump"),
        ("p2", "Pump"),
        ("t1", "Tank"),
        ("t2", "Tank"),
    ];
    objects.extend_from_slice(extra);
    let mut document = snapshot(&[population_json("Q", true, &objects)], Some(relationships));
    document["observation"] = json!(observation);
    document["anchor"] = Value::Null;
    document.as_object_mut().expect("object").remove("anchor");
    document
}

/// Housing as `instances` lists it, complete.
fn housing(instances: &[(&str, &str)]) -> Value {
    entry(
        HOUSING,
        true,
        instances
            .iter()
            .map(|(source, target)| link(source, target))
            .collect(),
    )
}

fn pre_of_i() -> Value {
    side("pre", &[], vec![housing(&[("p1", "t1"), ("p2", "t1")])])
}

fn post_of_i() -> Value {
    side("post", &[], vec![housing(&[("p1", "t2"), ("p2", "t1")])])
}

/// The documents of one invocation.
struct Documents {
    invocation: DocumentRef,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    invocations: BTreeMap<[u8; 32], Vec<u8>>,
}

impl Admission {
    /// The provisions of invocation **I** with the given `pre` and `post`
    /// snapshots, `edit` applied to the invocation before its digest is
    /// taken.
    fn documents(&self, pre: Value, post: Value, edit: impl FnOnce(&mut Value)) -> Documents {
        let model = |document: &mut Value| {
            document["model"] = json!({
                "identity": PACKAGE, "version": "1.0.0",
                "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(&self.model_digest)),
            });
        };
        let call = label("call");
        let mut snapshots = BTreeMap::new();
        let mut referenced = Vec::new();
        for (name, mut document) in [("pre", pre), ("post", post)] {
            model(&mut document);
            document["identity"] = labels(&label(name));
            document["invocation"] = labels(&call);
            let (reference, bytes) = provision(&label(name), &document);
            snapshots.insert(reference.digest, bytes);
            referenced.push(reference);
        }
        let reference = |held: &DocumentRef| {
            json!({"identity": labels(held),
                "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(&held.digest))})
        };
        let mut invocation = json!({
            "format": "quire.state.invocation/v1",
            "identity": labels(&call),
            "context": identity("Pump"),
            "operation": "rehome",
            "self": end_json("Q", "p1"),
            "pre": reference(&referenced[0]),
            "post": reference(&referenced[1]),
            "parameters": {},
            "result": Value::Null,
            "created": [],
            "deleted": [],
        });
        model(&mut invocation);
        edit(&mut invocation);
        let (invocation, bytes) = provision(&call, &invocation);
        Documents {
            invocations: BTreeMap::from([(invocation.digest, bytes)]),
            invocation,
            snapshots,
        }
    }

    /// Admits invocation **I** for the clause `clause` (`RehomePre` or
    /// `RehomePost`).
    fn admit_invocation(
        &self,
        clause: &str,
        documents: &Documents,
        effect: &OperationEffect,
    ) -> Result<AdmittedObservations, AdmissionFailure> {
        let checked_clause = self
            .graph
            .state_clause(clause)
            .expect("the clause is checked");
        let operation = checked_clause.operation().expect("an operation clause");
        let facts = ClauseFacts {
            identity: checked_clause.identity(),
            kind: checked_clause.kind(),
            context: checked_clause.context(),
            operation: Some(qsl_semantics::model::observation::OperationFacts {
                declaring: operation.declaring,
                // The test supplies the frame the operation is checked
                // with, to vary it without a package per row.
                declaration: OperationDeclaration::new("rehome", Vec::new(), None, effect.clone()),
            }),
        };
        admit_observations(
            self.graph.model_selections(),
            self.graph.scope().types(),
            &facts,
            &self.packages,
            ModelNormalizationLimits::default(),
            &Provisions {
                snapshots: &documents.snapshots,
                invocations: &documents.invocations,
            },
            &ClauseSelection {
                name: clause.to_owned(),
                input: ClauseSelectionInput::Invocation {
                    invocation: documents.invocation.clone(),
                },
            },
            ObservationLimits::default(),
        )
    }

    /// FR-115's check 11 over invocation **I** with `effect`.
    fn check_frame(
        &self,
        documents: &Documents,
        effect: &qsl_semantics::model::domain_package::OperationEffect,
    ) -> Result<qsl_semantics::model::observation::FrameVerdict, AdmissionFailure> {
        self.run_frame(documents, Some(effect))
    }

    /// FR-115 over the frame the package itself declares for `rehome`.
    fn check_declared_frame(
        &self,
        documents: &Documents,
    ) -> Result<qsl_semantics::model::observation::FrameVerdict, AdmissionFailure> {
        self.run_frame(documents, None)
    }

    /// Checks the frame against `supplied`, or against the effect the
    /// package declares for the operation when `None`.
    fn run_frame(
        &self,
        documents: &Documents,
        supplied: Option<&qsl_semantics::model::domain_package::OperationEffect>,
    ) -> Result<qsl_semantics::model::observation::FrameVerdict, AdmissionFailure> {
        use qsl_semantics::model::observation::{
            admit_frame_invocation, FrameFacts, OperationFacts,
        };
        let identifier = |text: &str| quire_exact::Identifier::new(text).expect("an identifier");
        let selection = self
            .graph
            .resolve_operation(&identifier("M"), &identifier("Pump"), &identifier("rehome"))
            .expect("the operation resolves");
        let (context, frame) = self
            .graph
            .operation_frame(&selection)
            .expect("a clause names the operation, so the package holds its frame");
        let facts = FrameFacts {
            context,
            frame: frame.frame(),
            operation: OperationFacts {
                declaring: frame.operation().declaring,
                declaration: frame.operation().declaration.clone(),
            },
        };
        let admitted = admit_frame_invocation(
            self.graph.model_selections(),
            self.graph.scope().types(),
            &facts,
            &self.packages,
            ModelNormalizationLimits::default(),
            &Provisions {
                snapshots: &documents.snapshots,
                invocations: &documents.invocations,
            },
            &documents.invocation,
            ObservationLimits::default(),
        )?;
        let effect = supplied.unwrap_or_else(|| frame.operation().declaration.effect());
        Ok(admitted
            .check_frame(effect)
            .expect("check 11 does not fault"))
    }
}

fn created(objects: &[&str]) -> impl FnOnce(&mut Value) {
    let objects: Vec<Value> = objects.iter().map(|key| end_json("Q", key)).collect();
    move |invocation| invocation["created"] = json!(objects)
}

fn deleted(objects: &[&str]) -> impl FnOnce(&mut Value) {
    let objects: Vec<Value> = objects.iter().map(|key| end_json("Q", key)).collect();
    move |invocation| invocation["deleted"] = json!(objects)
}

/// Row 4's post: `p3` created, with housing `p3`-`t1` too.
fn post_with_p3() -> Value {
    side(
        "post",
        &[("p3", "Pump")],
        vec![housing(&[("p1", "t1"), ("p2", "t1"), ("p3", "t1")])],
    )
}

fn pre_with_housing(instances: &[(&str, &str)]) -> Value {
    side("pre", &[], vec![housing(instances)])
}

fn effect(
    modifies: &[DeclarationKey],
    creates: &[&str],
    deletes: &[&str],
) -> qsl_semantics::model::domain_package::OperationEffect {
    qsl_semantics::model::domain_package::OperationEffect {
        modifies: modifies.to_vec(),
        creates: creates.iter().map(|name| key(name)).collect(),
        deletes: deletes.iter().map(|name| key(name)).collect(),
        ..Default::default()
    }
}

fn housing_key() -> DeclarationKey {
    key(&format!("relationship/{HOUSING}"))
}

fn composition_key() -> DeclarationKey {
    key(&format!("relationship/{COMPOSITION}"))
}

/// TC-1035 rows 23 and 24: a postcondition over invocation **I** admits its
/// frame when `rehome` modifies the relationship, and refuses naming the
/// first differing instance, `removed`, when it does not.
#[trace("TC-1035", "FR-106-AC-26")]
#[test]
fn check_11_5_grants_a_changed_link_only_to_a_frame_that_modifies_the_relationship() {
    let admission = checked(&Variant::orders());
    let documents = admission.documents(pre_of_i(), post_of_i(), |_| {});
    admission
        .admit_invocation(
            "RehomePost",
            &documents,
            &effect(&[housing_key()], &[], &[]),
        )
        .expect("row 23 admits");

    let Err(AdmissionFailure::Refused(record)) =
        admission.admit_invocation("RehomePost", &documents, &effect(&[], &[], &[]))
    else {
        panic!("row 24 refuses");
    };
    assert_refusal(&record, Code::FrameViolation, "unauthorized-change");
    assert_eq!(
        field(&record, "relationship"),
        Some(housing_key().node.as_str())
    );
    assert_eq!(field(&record, "source"), Some("p1"));
    assert_eq!(field(&record, "target"), Some("t1"));
    assert_eq!(field(&record, "difference"), Some("removed"));
}

/// TC-1035 row 25: a link with an end at a deleted object goes with it.
#[trace("TC-1035", "FR-106-AC-26")]
#[test]
fn check_11_5_leaves_out_a_link_at_a_deleted_object() {
    let admission = checked(&Variant::orders());
    let pre = pre_with_housing(&[("p1", "t1"), ("p2", "t1")]);
    let mut post = side("post", &[], vec![housing(&[("p1", "t1")])]);
    post["populations"][0]["objects"]
        .as_array_mut()
        .expect("objects")
        .retain(|object| object["key"] != "p2");
    let documents = admission.documents(pre, post, deleted(&["p2"]));
    admission
        .admit_invocation("RehomePost", &documents, &effect(&[], &[], &["Pump"]))
        .expect("row 25 admits");
}

/// TC-1035 row 26: a link at a created object is an added instance and needs
/// the grant.
#[trace("TC-1035", "FR-106-AC-26")]
#[test]
fn check_11_5_counts_a_link_at_a_created_object_as_added() {
    let admission = checked(&Variant::orders());
    let documents = admission.documents(
        pre_with_housing(&[("p1", "t1"), ("p2", "t1")]),
        post_with_p3(),
        created(&["p3"]),
    );
    let Err(AdmissionFailure::Refused(record)) =
        admission.admit_invocation("RehomePost", &documents, &effect(&[], &["Pump"], &[]))
    else {
        panic!("row 26 refuses");
    };
    assert_refusal(&record, Code::FrameViolation, "unauthorized-change");
    assert_eq!(field(&record, "source"), Some("p3"));
    assert_eq!(field(&record, "target"), Some("t1"));
    assert_eq!(field(&record, "difference"), Some("added"));
}

/// TC-1035 row 27 and TC-1037 row 11: a relationship listed in only one
/// snapshot, or not complete in both, is not compared.
#[trace("TC-1035", "FR-106-AC-26")]
#[test]
fn check_11_5_compares_only_relationships_both_snapshots_list_complete() {
    let admission = checked(&Variant::orders());
    let only_pre = admission.documents(pre_of_i(), side("post", &[], vec![]), |_| {});
    admission
        .admit_invocation("RehomePost", &only_pre, &effect(&[], &[], &[]))
        .expect("row 27 admits");

    let mut incomplete_post = post_of_i();
    entry_mut(&mut incomplete_post, HOUSING)["complete"] = json!(false);
    let documents = admission.documents(pre_of_i(), incomplete_post, |_| {});
    admission
        .admit_invocation("RehomePost", &documents, &effect(&[], &[], &[]))
        .expect("an incomplete post extent admits");
}

/// TC-1035 row 29: a precondition ignores a malformed `relationships` entry
/// of the `post` snapshot.
#[trace("TC-1035", "FR-106-AC-27")]
#[test]
fn a_precondition_neither_reads_nor_checks_the_post_snapshots_relationships() {
    let admission = checked(&Variant::orders());
    let mut post = post_of_i();
    entry_mut(&mut post, HOUSING)["instances"] = json!("none");
    let documents = admission.documents(pre_of_i(), post, |_| {});
    let admitted = admission
        .admit_invocation("RehomePre", &documents, &effect(&[], &[], &[]))
        .expect("the precondition admits");
    assert!(admitted.post.is_none());
    assert!(admitted.pre.is_some());
}

fn violation_change(
    verdict: qsl_semantics::model::observation::FrameVerdict,
) -> (String, qsl_semantics::model::observation::FrameChange) {
    use qsl_semantics::model::observation::FrameVerdict;
    let FrameVerdict::Violation(witness) = verdict else {
        panic!("a violation, not {verdict:?}");
    };
    assert_eq!(witness.code, Code::FrameViolation);
    assert_eq!(witness.cause, "unauthorized-change");
    (witness.population.clone(), witness.change)
}

fn link_change(
    relationship: DeclarationKey,
    difference: qsl_semantics::model::observation::LinkDifference,
    source: &str,
    target: &str,
    modifies: Vec<DeclarationKey>,
) -> qsl_semantics::model::observation::FrameChange {
    qsl_semantics::model::observation::FrameChange::Link {
        relationship,
        difference,
        source: SelectedObject {
            population: identity("Q"),
            key: source.to_owned(),
        },
        target: SelectedObject {
            population: identity("Q"),
            key: target.to_owned(),
        },
        modifies,
    }
}

/// TC-1037 rows 1 to 3: the first differing instance in source-then-target
/// order is the `Link`, carrying the frame's checked `modifies`.
#[trace("TC-1037")]
#[test]
fn a_frame_run_reports_the_first_changed_link_in_source_then_target_order() {
    use qsl_semantics::model::observation::{FrameVerdict, LinkDifference};
    let admission = checked(&Variant::orders());
    let documents = admission.documents(pre_of_i(), post_of_i(), |_| {});

    // Row 1: housing modified.
    assert_eq!(
        admission
            .check_frame(&documents, &effect(&[housing_key()], &[], &[]))
            .expect("admits"),
        FrameVerdict::Holds
    );
    // Row 2: modifies [].
    let (population, change) = violation_change(
        admission
            .check_frame(&documents, &effect(&[], &[], &[]))
            .expect("admits"),
    );
    assert_eq!(population, identity("Q"));
    assert_eq!(
        change,
        link_change(housing_key(), LinkDifference::Removed, "p1", "t1", vec![])
    );
    // Row 3: a field entry grants no link.
    let field = key("Pump/backup");
    let (_, change) = violation_change(
        admission
            .check_frame(&documents, &effect(std::slice::from_ref(&field), &[], &[]))
            .expect("admits"),
    );
    assert_eq!(
        change,
        link_change(
            housing_key(),
            LinkDifference::Removed,
            "p1",
            "t1",
            vec![field]
        )
    );
    // Row 12: the same input twice gives equal reports.
    let first = admission
        .check_frame(&documents, &effect(&[], &[], &[]))
        .expect("admits");
    let second = admission
        .check_frame(&documents, &effect(&[], &[], &[]))
        .expect("admits");
    assert_eq!(first, second);
}

/// TC-1037 rows 4 and 5: a link at a created object is the `Link` after the
/// creation is granted, and the relationship grant covers it.
#[trace("TC-1037")]
#[test]
fn a_link_at_a_created_object_is_added_unless_the_relationship_is_modified() {
    use qsl_semantics::model::observation::{FrameVerdict, LinkDifference};
    let admission = checked(&Variant::orders());
    let documents = admission.documents(
        pre_with_housing(&[("p1", "t1"), ("p2", "t1")]),
        post_with_p3(),
        created(&["p3"]),
    );
    let (_, change) = violation_change(
        admission
            .check_frame(&documents, &effect(&[], &["Pump"], &[]))
            .expect("admits"),
    );
    assert_eq!(
        change,
        link_change(housing_key(), LinkDifference::Added, "p3", "t1", vec![])
    );
    assert_eq!(
        admission
            .check_frame(&documents, &effect(&[housing_key()], &["Pump"], &[]))
            .expect("admits"),
        FrameVerdict::Holds
    );
    // Row 8: without the `creates` grant the creation is found first.
    let (_, change) = violation_change(
        admission
            .check_frame(&documents, &effect(&[], &[], &[]))
            .expect("admits"),
    );
    assert!(matches!(
        change,
        qsl_semantics::model::observation::FrameChange::Created { .. }
    ));
    // Row 9: without the `created` list the delta disagrees, before 11.5.
    let undeclared = admission.documents(
        pre_with_housing(&[("p1", "t1"), ("p2", "t1")]),
        post_with_p3(),
        |_| {},
    );
    let FrameVerdict::Refused(record) = admission
        .check_frame(&undeclared, &effect(&[], &["Pump"], &[]))
        .expect("admits")
    else {
        panic!("row 9 is a refusal");
    };
    assert_refusal(&record, Code::PopulationDeltaMismatch, "delta-disagreement");
}

/// TC-1037 rows 6 and 7: relationships in ascending `DeclarationKey` order,
/// then the lower source first.
#[trace("TC-1037")]
#[test]
fn links_are_walked_by_relationship_key_then_by_source_then_target() {
    use qsl_semantics::model::observation::LinkDifference;
    let admission = checked(&Variant::orders());
    // Row 6: composition is listed first and also differs, but housing
    // precedes it in DeclarationKey order, not document order.
    let pre = side(
        "pre",
        &[],
        vec![
            entry(COMPOSITION, true, vec![link("s1", "p1")]),
            housing(&[("p1", "t1"), ("p2", "t1")]),
        ],
    );
    let post = side(
        "post",
        &[],
        vec![
            entry(COMPOSITION, true, vec![]),
            housing(&[("p1", "t2"), ("p2", "t1")]),
        ],
    );
    let documents = admission.documents(pre, post, |_| {});
    let (_, change) = violation_change(
        admission
            .check_frame(&documents, &effect(&[], &[], &[]))
            .expect("admits"),
    );
    assert_eq!(
        change,
        link_change(housing_key(), LinkDifference::Removed, "p1", "t1", vec![])
    );
    assert!(housing_key() < composition_key());

    // Row 7: p1-t2 added precedes p2-t1 removed by source, though t1 < t2.
    let documents = admission.documents(
        pre_with_housing(&[("p2", "t1")]),
        side("post", &[], vec![housing(&[("p1", "t2")])]),
        |_| {},
    );
    let (_, change) = violation_change(
        admission
            .check_frame(&documents, &effect(&[], &[], &[]))
            .expect("admits"),
    );
    assert_eq!(
        change,
        link_change(housing_key(), LinkDifference::Added, "p1", "t2", vec![])
    );
}

/// TC-1037 rows 10 and 11: a deleted object's links go with it; a
/// relationship listed in one snapshot, or incomplete, is not compared.
#[trace("TC-1037")]
#[test]
fn a_deleted_objects_links_and_an_uncompared_relationship_find_nothing() {
    use qsl_semantics::model::observation::FrameVerdict;
    let admission = checked(&Variant::orders());
    let mut post = side("post", &[], vec![housing(&[("p1", "t1")])]);
    post["populations"][0]["objects"]
        .as_array_mut()
        .expect("objects")
        .retain(|object| object["key"] != "p2");
    let documents = admission.documents(
        pre_with_housing(&[("p1", "t1"), ("p2", "t1")]),
        post,
        deleted(&["p2"]),
    );
    assert_eq!(
        admission
            .check_frame(&documents, &effect(&[], &[], &["Pump"]))
            .expect("admits"),
        FrameVerdict::Holds
    );

    let documents = admission.documents(pre_of_i(), side("post", &[], vec![]), |_| {});
    assert_eq!(
        admission
            .check_frame(&documents, &effect(&[], &[], &[]))
            .expect("admits"),
        FrameVerdict::Holds
    );
    let mut incomplete_post = post_of_i();
    entry_mut(&mut incomplete_post, HOUSING)["complete"] = json!(false);
    let documents = admission.documents(pre_of_i(), incomplete_post, |_| {});
    assert_eq!(
        admission
            .check_frame(&documents, &effect(&[], &[], &[]))
            .expect("admits"),
        FrameVerdict::Holds
    );
}

/// Snapshots that list no relationship have no link to compare: check 11.5
/// reads no object, so a Frame run holds (a state-machine context object has
/// no object universe to build a reference from).
#[trace("TC-1037")]
#[test]
fn snapshots_listing_no_relationship_leave_check_11_5_with_nothing_to_compare() {
    use qsl_semantics::model::observation::FrameVerdict;
    let admission = checked(&Variant::orders());
    let documents =
        admission.documents(side("pre", &[], vec![]), side("post", &[], vec![]), |_| {});
    assert_eq!(
        admission
            .check_frame(&documents, &effect(&[], &[], &[]))
            .expect("admits"),
        FrameVerdict::Holds
    );
}

/// FR-103-AC-2, FR-115-AC-8 and TC-1037 rows 1 and 5: a package whose
/// `rehome` frame declares `modifies [Pump-housing-Tank]` grants the changed
/// link, with no effect supplied by the test; the checked frame node carries
/// the relationship beside its fields.
#[trace("TC-1037", "FR-115-AC-8")]
#[test]
fn a_frame_a_package_declares_with_modifies_of_a_relationship_grants_its_links() {
    use qsl_semantics::check::{BodyTerm, FrameModifies, NodeTag};
    use qsl_semantics::model::observation::FrameVerdict;
    let admission = checked(&Variant::declaring(
        &[identity(&format!("relationship/{HOUSING}"))],
        &[],
    ));
    // Row 1.
    let documents = admission.documents(pre_of_i(), post_of_i(), |_| {});
    assert_eq!(
        admission.check_declared_frame(&documents).expect("admits"),
        FrameVerdict::Holds
    );
    // The declared effect names the relationship.
    let identifier = |text: &str| quire_exact::Identifier::new(text).expect("an identifier");
    let selection = admission
        .graph
        .resolve_operation(&identifier("M"), &identifier("Pump"), &identifier("rehome"))
        .expect("the operation resolves");
    let (_, frame) = admission
        .graph
        .operation_frame(&selection)
        .expect("a frame");
    assert_eq!(
        frame.operation().declaration.effect().modifies,
        vec![housing_key()]
    );
    // The frame node carries one relationship entry and no field entry.
    let node = admission
        .graph
        .semantic_graph()
        .node(frame.frame())
        .expect("the frame node is in the graph");
    assert_eq!(node.node_tag(), NodeTag::State);
    let BodyTerm::Frame(term) = node.body() else {
        panic!("a frame body, not {:?}", node.body());
    };
    let [FrameModifies::Relationship(declaration)] = term.modifies.as_slice() else {
        panic!("exactly one relationship entry, got {:?}", term.modifies);
    };
    let relation = admission
        .graph
        .semantic_graph()
        .node(declaration.0)
        .expect("the entry names a node of the package");
    assert_eq!(relation.node_tag(), NodeTag::Relation);
    assert_eq!(relation.semantic_form(), "relationship");

    // Row 5: the grant covers the link at a created object.
    let granting = checked(&Variant::declaring(
        &[identity(&format!("relationship/{HOUSING}"))],
        &["Pump"],
    ));
    let documents = granting.documents(
        pre_with_housing(&[("p1", "t1"), ("p2", "t1")]),
        post_with_p3(),
        created(&["p3"]),
    );
    assert_eq!(
        granting.check_declared_frame(&documents).expect("admits"),
        FrameVerdict::Holds
    );
}

/// TC-1037 rows 2 and 3 over declared frames: a package whose `rehome`
/// declares no relationship grant, and one that declares a field only, both
/// report the first changed link, carrying the declared `modifies`.
#[trace("TC-1037", "FR-115-AC-8")]
#[test]
fn a_frame_without_the_relationship_declared_reports_the_link_with_its_declared_modifies() {
    use qsl_semantics::model::observation::LinkDifference;
    let nothing = checked(&Variant::orders());
    let documents = nothing.documents(pre_of_i(), post_of_i(), |_| {});
    let (population, change) =
        violation_change(nothing.check_declared_frame(&documents).expect("admits"));
    assert_eq!(population, identity("Q"));
    assert_eq!(
        change,
        link_change(housing_key(), LinkDifference::Removed, "p1", "t1", vec![])
    );

    let field = key("Pump/backup");
    let field_only = checked(&Variant::declaring(std::slice::from_ref(&field.node), &[]));
    let documents = field_only.documents(pre_of_i(), post_of_i(), |_| {});
    let (_, change) =
        violation_change(field_only.check_declared_frame(&documents).expect("admits"));
    assert_eq!(
        change,
        link_change(
            housing_key(),
            LinkDifference::Removed,
            "p1",
            "t1",
            vec![field]
        )
    );
}
