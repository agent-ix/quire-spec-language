// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-464 to TC-468 (FR-106, FR-107, FR-109).
//!
//! The domain package below (`test/nodes`) is a single self-referencing
//! `Node` type: `next: Reference<Node>` (optional), one closed population
//! `all_nodes`. `NoCycle` (`on M::Node at current { not reaches(self, self,
//! next) }`) is the one state clause every test shares; `isPositive`,
//! `seven` and `hasNext` are FR-109's `Function`-selection fixtures.
//!
//! **Scope note** (disclosed simplification, not a TC-465/468 gap this file
//! silently narrows): every FR-106 document here is a `current` snapshot
//! for `NoCycle`, an invariant. FR-106 check 11 (frame and delta) applies
//! only to an invocation document: the ConfigVersion sections below build
//! them for TC-466, and the `frame` submodule (TC-514, FR-115) runs check 11
//! as the thing under test through the `Frame` selection.

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU64;

use ix_trace_rs::trace;
use qsl_eval::value::{CallFailure, CheckedPackageEvaluation, QualifiedName};
use qsl_semantics::family::FamilyOutcome;
use qsl_semantics::library::LibraryName;
use qsl_semantics::model::key::hex;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::model::observation::{
    AdmittedObservations, ClauseSelection, ClauseSelectionInput, DocumentRef, Observation,
    ObservationLimits, Provisions, SelectedAnchor, SelectedObject,
};
use quire_exact::{
    ChargePoint, EffectiveId, FieldValue, InjectedDenial, LimitKind, Meter, NodeKey, ObjectId,
    ObjectReference, Outcome, UniverseId, Value,
};
use quire_semantic_value::call::InputRefusal;
use quire_semantic_value::object_closure::ObjectClosure;
use serde_json::json;

use super::{
    admit_clause_observations, run_clause, ClauseArgument, ClauseArgumentValue, ClauseDisposition,
    ClauseRunRefusal, ClauseRunRequest, ClauseRunSelection, ClauseRunSource, ClauseRunStage,
};
use crate::spine::{compose, default_accounting, ComposedUnit, DependencyInput, SpineLimits};

const PACKAGE_IDENTITY: &str = "test/nodes";
const PLACEHOLDER_DIGEST: &str =
    "sha256:0000000000000000000000000000000000000000000000000000000000000000";

fn node_type() -> String {
    format!("ix://{PACKAGE_IDENTITY}/Node")
}

fn population_identity() -> String {
    format!("ix://{PACKAGE_IDENTITY}/all_nodes")
}

/// The `test/nodes` domain package (Semantic IR 2.0.0): one `Node` type
/// with an optional self-reference `next`, one closed population.
fn domain_document() -> Vec<u8> {
    let node = node_type();
    let field_identity = format!("{node}/next");
    let population = population_identity();
    let envelope = json!({
        "contractVersion": "2.0.0",
        "source": {
            "identity": format!("ix://{PACKAGE_IDENTITY}/spec"),
            "version": "1.0.0",
            "dialect": "spec-bundle",
            "digest": PLACEHOLDER_DIGEST,
        },
        "package": {
            "identity": PACKAGE_IDENTITY,
            "version": "1.0.0",
            "manifestDigest": PLACEHOLDER_DIGEST,
            "mappingVersions": [],
            "profileVersions": [],
            "lockDigest": PLACEHOLDER_DIGEST,
        },
        "occurrences": [],
        "extensions": [],
        "constructs": [
            {
                "kind": {"module": PACKAGE_IDENTITY, "name": "object_type"},
                "moduleVersion": "1.0.0",
                "manifestDigest": PLACEHOLDER_DIGEST,
                "construct": {
                    "identity": "none",
                    "shape": "record",
                    "members": {},
                    "meaning": "quire.meaning.model.object-type/v1",
                },
            },
            {
                "kind": {"module": PACKAGE_IDENTITY, "name": "population"},
                "moduleVersion": "1.0.0",
                "manifestDigest": PLACEHOLDER_DIGEST,
                "construct": {
                    "identity": "none",
                    "shape": "record",
                    "members": {},
                    "meaning": "quire.meaning.model.population/v1",
                },
            },
        ],
        "types": [
            {
                "identity": node,
                "displayName": node,
                "kind": {"module": PACKAGE_IDENTITY, "name": "object_type"},
                "roles": [],
                "origin": {
                    "generated": {
                        "generatorIdentity": node,
                        "generatorVersion": "1.0.0",
                        "inputIdentities": [node],
                    }
                },
                "constraints": [],
                "extensions": [],
                "unknownPolicy": "reject",
                "supertypes": [],
                "fields": [
                    {
                        "identity": field_identity,
                        "name": "next",
                        "typeRef": node,
                        "presence": "optional",
                        "nullable": false,
                        "defaultKind": "none",
                        // `lower: 1` (not 0), matching the already-passing
                        // `qsl-semantics` `ConfigVersion` fixture's own
                        // optional self-reference field exactly: FR-104's
                        // `reaches` admits an edge field only when its
                        // *declared* type is a bare `Reference<T>` (never
                        // `Option<Reference<T>>`) -- `next`'s wire
                        // `presence: "optional"` alone (not `multiplicity`)
                        // is what lets an object omit it, and FR-106's own
                        // admission (`document.rs`'s `admit_object_field`)
                        // reads that same `presence`, not `multiplicity`.
                        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                        "origin": {
                            "generated": {
                                "generatorIdentity": field_identity,
                                "generatorVersion": "1.0.0",
                                "inputIdentities": [field_identity],
                            }
                        },
                    },
                ],
                "operations": [],
            },
        ],
        "populations": [
            {
                "identity": population,
                "displayName": population,
                "kind": {"module": PACKAGE_IDENTITY, "name": "population"},
                "members": [node],
                "extent": "closed",
                "origin": {
                    "generated": {
                        "generatorIdentity": population,
                        "generatorVersion": "1.0.0",
                        "inputIdentities": [population],
                    }
                },
            },
        ],
    });
    envelope.to_string().into_bytes()
}

/// The unit text selecting [`domain_document`] as model alias `M`, plus
/// `NoCycle` and the `Function`-selection fixtures.
fn unit_and_packages() -> (String, BTreeMap<[u8; 32], Vec<u8>>) {
    let document = domain_document();
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let digest = hex(digest);
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model M = {PACKAGE_IDENTITY:?} version \"1.0.0\" digest \"sha256-jcs:{digest}\";\n\
         invariant NoCycle using v on M::Node at current {{ not reaches(self, self, next) }}\n\
         function isPositive using v(n: Integer): Boolean pure {{ n > 0 }}\n\
         function seven using v(): Integer pure {{ 7 }}\n\
         function hasNext using v(n: M::Node): Boolean pure {{ present(deref(n).next) }}\n"
    );
    (unit, packages)
}

fn source() -> qsl_foundation::SourceIdentity {
    qsl_foundation::SourceIdentity::new("agent-ix", "clause-run-fixture", "git", "1")
}

fn compiled() -> ComposedUnit {
    let (unit, packages) = unit_and_packages();
    compose(
        source(),
        "clause-run.native",
        unit.as_bytes(),
        &packages,
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .unwrap_or_else(|refusal| panic!("compile refused: {refusal:?}"))
}

/// `document`'s `sha256-jcs` digest, computed the same way `model::
/// observation`'s own `document_digest` does (FR-056's digest-first rule):
/// bytes that parse as JSON are digested over RFC 8785 canonical bytes.
fn document_digest(bytes: &[u8]) -> [u8; 32] {
    let document = quire_canonical::read(bytes, u64::MAX).expect("test fixture is JSON");
    *quire_canonical::sha256(&document, quire_canonical::Limits::new(u64::MAX))
        .expect("test fixture is RFC 8785 canonical")
        .as_bytes()
}

fn model_header_json(model_digest_hex: &str) -> serde_json::Value {
    json!({
        "identity": PACKAGE_IDENTITY,
        "version": "1.0.0",
        "digest": format!("sha256-jcs:{model_digest_hex}"),
    })
}

fn document_identity_json(label: &DocumentRef) -> serde_json::Value {
    json!({
        "authority": label.authority,
        "identity": label.identity,
        "revision_namespace": label.revision_namespace,
        "revision": label.revision,
    })
}

/// A snapshot document naming `label`, admitting the given `next` chain in
/// population `all_nodes` (each entry a `(key, next_key)` pair; `None`
/// means `next` is absent), observed `current`.
fn snapshot_bytes(
    label: &DocumentRef,
    model_digest_hex: &str,
    chain: &[(&str, Option<&str>)],
) -> Vec<u8> {
    let population = population_identity();
    let node = node_type();
    let objects: Vec<_> = chain
        .iter()
        .map(|(key, next)| {
            let next_field = match next {
                Some(next_key) => {
                    json!({"present": {"reference": {"population": population, "key": next_key}}})
                }
                None => json!({"absent": {}}),
            };
            json!({
                "key": key,
                "type": node,
                "fields": {"next": next_field},
            })
        })
        .collect();
    let value = json!({
        "format": "quire.state.snapshot/v1",
        "identity": document_identity_json(label),
        "observation": "current",
        "anchor": {"kind": "initialization", "name": "init"},
        "model": model_header_json(model_digest_hex),
        "populations": [
            {"population": population, "complete": true, "objects": objects},
        ],
    });
    value.to_string().into_bytes()
}

/// A [`ClauseSelection`] naming `NoCycle` at `label`'s snapshot, self object
/// `self_key` in `all_nodes`.
fn no_cycle_selection(label: DocumentRef, self_key: &str) -> ClauseSelection {
    ClauseSelection {
        name: "NoCycle".to_owned(),
        input: ClauseSelectionInput::Current {
            snapshot: label,
            anchor: SelectedAnchor {
                kind: qsl_semantics::model::observation::AnchorKind::Initialization,
                name: "init".to_owned(),
            },
            self_object: SelectedObject {
                population: population_identity(),
                key: self_key.to_owned(),
            },
        },
    }
}

/// A `Node` object reference in a single hand-picked universe -- the
/// evaluator never resolves a universe by name, only by reference equality
/// (FR-107), so one arbitrary universe for the whole environment suffices.
fn node_reference(node_effective: EffectiveId, key: &str) -> ObjectReference {
    ObjectReference::new(
        UniverseId::from_digest([7; 32]),
        node_effective,
        ObjectId::new(key.to_owned()).expect("non-empty key"),
    )
}

/// An [`ObjectEnvironment`] of `Node` objects along `chain`, each pointing
/// to the next; the last object's `next` is absent.
fn node_environment(
    types: &quire_semantic_value::declaration::TypeEnvironment,
    node_effective: EffectiveId,
    chain: &[&str],
) -> ObjectEnvironment {
    let mut objects = Vec::with_capacity(chain.len());
    for (index, key) in chain.iter().enumerate() {
        let next = chain.get(index + 1).map(|next_key| {
            FieldValue::Present(Value::Reference(node_reference(node_effective, next_key)))
        });
        objects.push((
            node_reference(node_effective, key),
            vec![("next", next.unwrap_or(FieldValue::Absent))],
        ));
    }
    ObjectEnvironment::new(
        ObjectClosure::new(types, objects, &[]).expect("the chain is internally closed"),
    )
}

fn no_cycle_observations(
    clause_identity: quire_exact::NodeKey,
    environment: ObjectEnvironment,
    self_object: ObjectReference,
) -> AdmittedObservations {
    AdmittedObservations {
        usage: qsl_semantics::model::observation::AdmissionUsage::default(),
        clause: clause_identity,
        current: Some(Observation {
            identity: DocumentRef {
                authority: "test".to_owned(),
                identity: "snap".to_owned(),
                revision_namespace: "ns".to_owned(),
                revision: "1".to_owned(),
                digest: [0; 32],
            },
            environment,
            populations: BTreeMap::from([(population_identity(), true)]),
            out_of_range: Vec::new(),
        }),
        pre: None,
        post: None,
        self_object,
        parameters: Vec::new(),
        result: None,
        created: Vec::new(),
        deleted: Vec::new(),
    }
}

fn boolean_outcome(evaluation: qsl_eval::value::Evaluation) -> bool {
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(value))) => value,
        other => panic!("expected a completed Boolean outcome, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// FR-107: S6a `ProtocolClause` evaluation and the `reaches` charge log.
// ---------------------------------------------------------------------------

/// FR-107-AC-3 (TC-466 step 3): `NoCycle` over an acyclic chain `a -> b ->
/// c` completes `true` (`self` -- `a` -- never reaches itself), and the
/// `reaches` walk's own charges (filtered out of the full log, which also
/// carries the enclosing `not`'s own accounting, not this AC's concern) are
/// exactly `[graph.expand, graph.edge, graph.expand, graph.edge,
/// graph.expand, graph.result-retain]`: one expansion to enqueue `a`
/// itself, then one expand-and-edge pair per BFS step that discovers a new
/// object (`a` to `b`, `b` to `c`), then the walk exhausts the chain with
/// no match before the closing retain.
#[trace("TC-466", "FR-107-AC-3")]
#[test]
fn no_cycle_completes_true_with_the_expected_reaches_charge_log() {
    let compiled = compiled();
    let package = &compiled.package;
    let graph = package.graph();
    let clause = graph.state_clause("NoCycle").expect("NoCycle is declared");
    let node_effective = clause.context();
    let types = graph.scope().types();
    let environment = node_environment(types, node_effective, &["a", "b", "c"]);
    let observations = no_cycle_observations(
        clause.identity(),
        environment,
        node_reference(node_effective, "a"),
    );

    let mut meter = Meter::new(default_accounting(1_000_000));
    let name = QualifiedName::unqualified("NoCycle").unwrap();
    let evaluation = package
        .evaluate_clause(&name, &observations, &mut meter)
        .expect("NoCycle evaluates")
        .evaluation;
    assert!(boolean_outcome(evaluation), "a -> b -> c never reaches a");
    let graph_charges: Vec<_> = meter
        .admitted_charges()
        .iter()
        .copied()
        .filter(|point| {
            matches!(
                point,
                ChargePoint::GraphExpand | ChargePoint::GraphEdge | ChargePoint::GraphResultRetain
            )
        })
        .collect();
    assert_eq!(
        graph_charges,
        [
            ChargePoint::GraphExpand,
            ChargePoint::GraphEdge,
            ChargePoint::GraphExpand,
            ChargePoint::GraphEdge,
            ChargePoint::GraphExpand,
            ChargePoint::GraphResultRetain,
        ]
    );
}

/// FR-107-AC-3: over a cyclic chain `a -> b -> a`, `NoCycle` completes
/// `false` (`self` does reach itself).
#[trace("TC-466", "FR-107-AC-3")]
#[test]
fn no_cycle_completes_false_over_a_cycle() {
    let compiled = compiled();
    let package = &compiled.package;
    let graph = package.graph();
    let clause = graph.state_clause("NoCycle").expect("NoCycle is declared");
    let node_effective = clause.context();
    let types = graph.scope().types();
    // A two-node cycle: `a`'s `next` is `b`, `b`'s `next` is `a`.
    let environment = ObjectEnvironment::new(
        ObjectClosure::new(
            types,
            vec![
                (
                    node_reference(node_effective, "a"),
                    vec![(
                        "next",
                        FieldValue::Present(Value::Reference(node_reference(node_effective, "b"))),
                    )],
                ),
                (
                    node_reference(node_effective, "b"),
                    vec![(
                        "next",
                        FieldValue::Present(Value::Reference(node_reference(node_effective, "a"))),
                    )],
                ),
            ],
            &[],
        )
        .expect("the two-node cycle is internally closed"),
    );
    let observations = no_cycle_observations(
        clause.identity(),
        environment,
        node_reference(node_effective, "a"),
    );
    let mut meter = Meter::new(default_accounting(1_000_000));
    let name = QualifiedName::unqualified("NoCycle").unwrap();
    let evaluation = package
        .evaluate_clause(&name, &observations, &mut meter)
        .expect("NoCycle evaluates")
        .evaluation;
    assert!(!boolean_outcome(evaluation), "a -> b -> a reaches a");
}

/// FR-107 (`InputRefusal::UnknownClause`): a name resolving to no state
/// clause of the package refuses before any charge.
#[trace("TC-467", "FR-107-AC-5")]
#[test]
fn evaluate_clause_refuses_an_unknown_clause_name() {
    let compiled = compiled();
    let package = &compiled.package;
    let graph = package.graph();
    let clause = graph.state_clause("NoCycle").expect("NoCycle is declared");
    let node_effective = clause.context();
    let types = graph.scope().types();
    let environment = node_environment(types, node_effective, &["a"]);
    let observations = no_cycle_observations(
        clause.identity(),
        environment,
        node_reference(node_effective, "a"),
    );
    let mut meter = Meter::new(default_accounting(1_000_000));
    let name = QualifiedName::unqualified("NoSuchClause").unwrap();
    let failure = package
        .evaluate_clause(&name, &observations, &mut meter)
        .expect_err("no clause named NoSuchClause");
    assert!(matches!(
        failure,
        CallFailure::Input(InputRefusal::UnknownClause(_))
    ));
    assert!(
        meter.admitted_charges().is_empty(),
        "no charge before admission"
    );
}

/// FR-107 (`InputRefusal::ObservationsMismatch`): observations minted for
/// one clause's identity refuse when named against a different clause.
#[trace("TC-467", "FR-107-AC-5")]
#[test]
fn evaluate_clause_refuses_observations_admitted_for_another_clause() {
    let compiled = compiled();
    let package = &compiled.package;
    let graph = package.graph();
    let clause = graph.state_clause("NoCycle").expect("NoCycle is declared");
    let node_effective = clause.context();
    let types = graph.scope().types();
    let environment = node_environment(types, node_effective, &["a"]);
    // A clause identity that is not `NoCycle`'s own -- any other `NodeKey`
    // disagrees.
    assert_ne!(
        clause.identity(),
        quire_exact::NodeKey::from_digest([9; 32])
    );
    let wrong_identity = quire_exact::NodeKey::from_digest([9; 32]);
    let observations = no_cycle_observations(
        wrong_identity,
        environment,
        node_reference(node_effective, "a"),
    );
    let mut meter = Meter::new(default_accounting(1_000_000));
    let name = QualifiedName::unqualified("NoCycle").unwrap();
    let failure = package
        .evaluate_clause(&name, &observations, &mut meter)
        .expect_err("the observations were admitted for a different identity");
    assert!(matches!(
        failure,
        CallFailure::Input(InputRefusal::ObservationsMismatch)
    ));
    assert!(
        meter.admitted_charges().is_empty(),
        "no charge before the identity check"
    );
}

/// FR-107 determinism: the same observations evaluate to the same outcome
/// and the same charge log on a second, independent run.
#[trace("TC-467", "FR-107-AC-6")]
#[test]
fn evaluate_clause_is_deterministic() {
    let compiled = compiled();
    let package = &compiled.package;
    let graph = package.graph();
    let clause = graph.state_clause("NoCycle").expect("NoCycle is declared");
    let node_effective = clause.context();
    let types = graph.scope().types();

    let run = |environment: ObjectEnvironment, self_key: &str| {
        let observations = no_cycle_observations(
            clause.identity(),
            environment,
            node_reference(node_effective, self_key),
        );
        let mut meter = Meter::new(default_accounting(1_000_000));
        let name = QualifiedName::unqualified("NoCycle").unwrap();
        let evaluation = package
            .evaluate_clause(&name, &observations, &mut meter)
            .unwrap()
            .evaluation;
        (
            boolean_outcome(evaluation),
            meter.admitted_charges().to_vec(),
        )
    };
    // Every `test/nodes` case that reaches S6a (TC-467 step 3): the
    // acyclic chain (`Completed(true)`) and a two-node cycle
    // (`Completed(false)`), each run twice, giving equal outcomes and equal
    // charge logs.
    assert_eq!(
        run(
            node_environment(types, node_effective, &["a", "b", "c"]),
            "a"
        ),
        run(
            node_environment(types, node_effective, &["a", "b", "c"]),
            "a"
        )
    );
    let cycle = || {
        ObjectEnvironment::new(
            ObjectClosure::new(
                types,
                vec![
                    (
                        node_reference(node_effective, "a"),
                        vec![(
                            "next",
                            FieldValue::Present(Value::Reference(node_reference(
                                node_effective,
                                "b",
                            ))),
                        )],
                    ),
                    (
                        node_reference(node_effective, "b"),
                        vec![(
                            "next",
                            FieldValue::Present(Value::Reference(node_reference(
                                node_effective,
                                "a",
                            ))),
                        )],
                    ),
                ],
                &[],
            )
            .expect("the two-node cycle is internally closed"),
        )
    };
    assert_eq!(run(cycle(), "a"), run(cycle(), "a"));
}

/// TC-467 step 1 (FR-107-AC-4): a zero-budget meter completes `Incomplete`
/// -- an S6a `Outcome::Incomplete` is, by FR-107's own Behavior section
/// (`FR-107-evaluate-state-clauses-at-s6a.md:95-100`), always catalog code
/// `resource_exhausted`/`insufficient-next-charge` with no truth value, the
/// same charge point the zero-budget meter first denies (`WorkUnits`, the
/// only limit `default_accounting` sets); the same call with the default
/// budget then completes `true`, never a fault or a silent truncation.
#[trace("TC-467", "FR-107-AC-4")]
#[test]
fn evaluate_clause_reports_incomplete_when_the_meter_is_exhausted() {
    let compiled = compiled();
    let package = &compiled.package;
    let graph = package.graph();
    let clause = graph.state_clause("NoCycle").expect("NoCycle is declared");
    let node_effective = clause.context();
    let types = graph.scope().types();
    let environment = node_environment(types, node_effective, &["a", "b", "c"]);
    let observations = no_cycle_observations(
        clause.identity(),
        environment,
        node_reference(node_effective, "a"),
    );
    let name = QualifiedName::unqualified("NoCycle").unwrap();

    let mut exhausted_meter = Meter::new(default_accounting(0));
    let exhausted = package
        .evaluate_clause(&name, &observations, &mut exhausted_meter)
        .expect("an exhausted meter is a kernel outcome, not a fault")
        .evaluation;
    match exhausted.outcome {
        FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) => {
            assert_eq!(
                incomplete.limit_kind,
                quire_exact::LimitKind::WorkUnits,
                "a zero work_units budget denies the first work_units charge"
            );
        }
        other => panic!("expected Incomplete, got {other:?}"),
    }

    let mut default_meter = Meter::new(default_accounting(1_000_000));
    let completed = package
        .evaluate_clause(&name, &observations, &mut default_meter)
        .expect("the default budget evaluates cleanly")
        .evaluation;
    assert!(
        boolean_outcome(completed),
        "a -> b -> c never reaches a, with the default budget"
    );
}

// ---------------------------------------------------------------------------
// FR-109: `run_clause`.
// ---------------------------------------------------------------------------

fn model_digest_hex() -> String {
    let (unit, packages) = unit_and_packages();
    let _ = unit;
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    hex(digest)
}

fn request(selection: ClauseRunSelection) -> ClauseRunRequest {
    let (unit, packages) = unit_and_packages();
    ClauseRunRequest {
        source: ClauseRunSource::Program {
            identity: source(),
            path: "clause-run.native".to_owned(),
            bytes: unit.into_bytes(),
        },
        packages,
        dependencies: DependencyInput::default(),
        snapshots: BTreeMap::new(),
        invocations: BTreeMap::new(),
        selection,
        expected_package_id: None,
        limits: SpineLimits::default(),
        observation_limits: ObservationLimits::default(),
        accounting: default_accounting(1_000_000),
    }
}

/// A healthy-parent (acyclic) or violating-parent (cyclic) `NoCycle`
/// request, for FR-109-AC-1 and FR-109-AC-5.
fn no_cycle_request(chain: &[(&str, Option<&str>)]) -> (ClauseRunRequest, DocumentRef) {
    let model_digest = model_digest_hex();
    let label = DocumentRef {
        authority: "test".to_owned(),
        identity: "snap".to_owned(),
        revision_namespace: "ns".to_owned(),
        revision: "1".to_owned(),
        digest: [0; 32],
    };
    let bytes = snapshot_bytes(&label, &model_digest, chain);
    let digest = document_digest(&bytes);
    let label = DocumentRef { digest, ..label };
    let selection = no_cycle_selection(label.clone(), "a");

    let mut request = request(ClauseRunSelection::Clause(selection));
    request.snapshots.insert(digest, bytes);
    (request, label)
}

/// TC-468 (FR-109-AC-1): a `Clause` selection over a `current` snapshot of
/// an acyclic chain evaluates `NoCycle` to `Completed(Boolean(true))`, and
/// a cyclic chain to `Completed(Boolean(false))`; both carry the
/// compiled `package_id` and the usage
/// (`FR-109-run-a-state-clause-through-the-spine.md:149`).
#[trace("TC-468", "FR-109-AC-1", "FR-109-AC-6")]
#[test]
fn run_clause_evaluates_a_clause_selection() {
    let (request, _label) = no_cycle_request(&[("a", Some("b")), ("b", None)]);
    // The package `compose` emits for this unit on its own, independent of
    // the run.
    let (unit, packages) = unit_and_packages();
    let expected_package_id = compose(
        source(),
        "clause-run.native",
        unit.as_bytes(),
        &packages,
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .expect("the unit compiles")
    .emitted
    .package_id();
    let report = run_clause(request).expect("a well-formed request always reports");
    match report.disposition {
        ClauseDisposition::Evaluate(super::CallOutcome::Completed(super::CallValue::Boolean(
            value,
        ))) => {
            assert!(value, "a -> b -> (nothing) never reaches a");
        }
        other => panic!("expected Evaluate(Completed(Boolean(true))), got {other:?}"),
    }
    assert_eq!(report.disposition.stage(), ClauseRunStage::Evaluate);
    assert_eq!(
        report.disposition.category(),
        qsl_foundation::diagnostic::Category::Success
    );
    assert_eq!(report.disposition.truth(), Some(true));
    assert_eq!(report.disposition.category().exit_code(), 0);

    assert_eq!(report.package_id, Some(expected_package_id));
    assert!(
        report.usage.evaluation_admissions > 0,
        "the evaluation meter's charges are reported"
    );
}

/// TC-468 (FR-109-AC-1): the violating-parent case (a cycle) reports
/// `violation`, `truth: false`, exit 10.
#[trace("TC-468", "FR-109-AC-1")]
#[test]
fn run_clause_reports_a_violation_over_a_cyclic_chain() {
    let (request, _label) = no_cycle_request(&[("a", Some("b")), ("b", Some("a"))]);
    let report = run_clause(request).expect("a well-formed request always reports");
    match report.disposition {
        ClauseDisposition::Evaluate(super::CallOutcome::Completed(super::CallValue::Boolean(
            value,
        ))) => {
            assert!(!value, "a -> b -> a reaches a");
        }
        other => panic!("expected Evaluate(Completed(Boolean(false))), got {other:?}"),
    }
    assert_eq!(
        report.disposition.category(),
        qsl_foundation::diagnostic::Category::Violation
    );
    assert_eq!(report.disposition.truth(), Some(false));
    assert_eq!(report.disposition.category().exit_code(), 10);
}

/// TC-468 (FR-109-AC-5): running one request twice gives equal reports,
/// including usage.
#[trace("TC-468", "FR-109-AC-5")]
#[test]
fn running_the_same_request_twice_gives_equal_reports_including_usage() {
    let (request_one, _) = no_cycle_request(&[("a", Some("b")), ("b", None)]);
    let (request_two, _) = no_cycle_request(&[("a", Some("b")), ("b", None)]);
    let report_one = run_clause(request_one).expect("a well-formed request always reports");
    let report_two = run_clause(request_two).expect("a well-formed request always reports");

    assert_eq!(report_one.package_id, report_two.package_id);
    assert_eq!(
        report_one.disposition.truth(),
        report_two.disposition.truth()
    );
    assert_eq!(
        report_one.disposition.category(),
        report_two.disposition.category()
    );
    assert_eq!(
        report_one.disposition.category().exit_code(),
        report_two.disposition.category().exit_code()
    );
    assert_eq!(
        report_one.usage.evaluation_admissions,
        report_two.usage.evaluation_admissions
    );
    assert_eq!(
        report_one.usage.evaluation_consumed,
        report_two.usage.evaluation_consumed
    );
    assert!(
        report_one.usage.evaluation_admissions > 0,
        "evaluating NoCycle admits at least one charge"
    );
}

/// TC-468 (FR-109-AC-2): a `Clause` selection naming an undeclared clause
/// refuses at stage `select`, `missing_declaration`.
#[trace("TC-468", "FR-109-AC-2")]
#[test]
fn run_clause_reports_missing_name_for_an_unknown_clause() {
    let model_digest = model_digest_hex();
    let label = DocumentRef {
        authority: "test".to_owned(),
        identity: "snap".to_owned(),
        revision_namespace: "ns".to_owned(),
        revision: "1".to_owned(),
        digest: [0; 32],
    };
    let bytes = snapshot_bytes(&label, &model_digest, &[("a", None)]);
    let digest = document_digest(&bytes);
    let label = DocumentRef { digest, ..label };
    let mut selection = no_cycle_selection(label, "a");
    selection.name = "NoSuchClause".to_owned();

    let mut request = request(ClauseRunSelection::Clause(selection));
    request.snapshots.insert(digest, bytes);
    let report = run_clause(request).unwrap();
    assert!(matches!(
        report.disposition,
        ClauseDisposition::MissingName { .. }
    ));
    assert_eq!(report.disposition.stage(), ClauseRunStage::Select);
    assert_eq!(report.disposition.category().exit_code(), 20);
}

/// TC-468 (FR-109-AC-3): a `Function` selection over an `Integer` argument
/// evaluates `isPositive` to `Completed(Boolean(true))`.
#[trace("TC-468", "FR-109-AC-3")]
#[test]
fn run_clause_evaluates_a_function_selection_with_an_integer_argument() {
    let model_digest = model_digest_hex();
    let label = DocumentRef {
        authority: "test".to_owned(),
        identity: "snap".to_owned(),
        revision_namespace: "ns".to_owned(),
        revision: "1".to_owned(),
        digest: [0; 32],
    };
    let bytes = snapshot_bytes(&label, &model_digest, &[]);
    let digest = document_digest(&bytes);
    let label = DocumentRef { digest, ..label };

    let mut request = request(ClauseRunSelection::Function {
        name: "isPositive".to_owned(),
        arguments: vec![ClauseArgument {
            parameter: "n".to_owned(),
            value: ClauseArgumentValue::Integer(5),
        }],
        snapshot: label,
    });
    request.snapshots.insert(digest, bytes);
    let report = run_clause(request).unwrap();
    match report.disposition {
        ClauseDisposition::Evaluate(super::CallOutcome::Completed(super::CallValue::Boolean(
            value,
        ))) => {
            assert!(value);
        }
        other => panic!("expected Evaluate(Completed(Boolean(true))), got {other:?}"),
    }
}

/// TC-468 (FR-109-AC-4): a `Function` selection over a `Reference` argument
/// resolves it in the current snapshot (`ObjectClosure::find`,
/// `population_universe`) and evaluates `hasNext`.
#[trace("TC-468", "FR-109-AC-4")]
#[test]
fn run_clause_evaluates_a_function_selection_with_a_reference_argument() {
    let model_digest = model_digest_hex();
    let label = DocumentRef {
        authority: "test".to_owned(),
        identity: "snap".to_owned(),
        revision_namespace: "ns".to_owned(),
        revision: "1".to_owned(),
        digest: [0; 32],
    };
    let bytes = snapshot_bytes(&label, &model_digest, &[("a", Some("b")), ("b", None)]);
    let digest = document_digest(&bytes);
    let label = DocumentRef { digest, ..label };

    let mut request = request(ClauseRunSelection::Function {
        name: "hasNext".to_owned(),
        arguments: vec![ClauseArgument {
            parameter: "n".to_owned(),
            value: ClauseArgumentValue::Reference {
                population: population_identity(),
                key: "a".to_owned(),
            },
        }],
        snapshot: label,
    });
    request.snapshots.insert(digest, bytes);
    let report = run_clause(request).unwrap();
    match report.disposition {
        ClauseDisposition::Evaluate(super::CallOutcome::Completed(super::CallValue::Boolean(
            value,
        ))) => {
            assert!(value, "a has a next (b)");
        }
        other => panic!("expected Evaluate(Completed(Boolean(true))), got {other:?}"),
    }
}

/// FR-109-AC-4's own worked example: naming an object no admitted snapshot
/// holds refuses at stage `admit`, `invalid_runtime_input`/
/// `wrong-role-mapping`.
#[trace("TC-468", "FR-109-AC-4")]
#[test]
fn run_clause_refuses_an_unresolved_reference_argument() {
    let model_digest = model_digest_hex();
    let label = DocumentRef {
        authority: "test".to_owned(),
        identity: "snap".to_owned(),
        revision_namespace: "ns".to_owned(),
        revision: "1".to_owned(),
        digest: [0; 32],
    };
    let bytes = snapshot_bytes(&label, &model_digest, &[("a", None)]);
    let digest = document_digest(&bytes);
    let label = DocumentRef { digest, ..label };

    let mut request = request(ClauseRunSelection::Function {
        name: "hasNext".to_owned(),
        arguments: vec![ClauseArgument {
            parameter: "n".to_owned(),
            value: ClauseArgumentValue::Reference {
                population: population_identity(),
                key: "ghost".to_owned(),
            },
        }],
        snapshot: label,
    });
    request.snapshots.insert(digest, bytes);
    let report = run_clause(request).unwrap();
    assert_eq!(report.disposition.stage(), ClauseRunStage::Admit);
    match report.disposition {
        ClauseDisposition::Admit(qsl_semantics::model::observation::AdmissionFailure::Refused(
            record,
        )) => {
            assert_eq!(
                record.code,
                qsl_foundation::diagnostic::Code::InvalidRuntimeInput
            );
            assert_eq!(record.cause, "wrong-role-mapping");
        }
        other => panic!("expected Admit(Refused(..)), got {other:?}"),
    }
}

/// TC-468 (FR-109-AC-5): a `Function` selection naming a non-`Boolean`
/// function refuses `NotAPredicate`, before any admission or evaluation.
#[trace("TC-468", "FR-109-AC-5")]
#[test]
fn run_clause_reports_not_a_predicate_for_an_integer_function() {
    let model_digest = model_digest_hex();
    let label = DocumentRef {
        authority: "test".to_owned(),
        identity: "snap".to_owned(),
        revision_namespace: "ns".to_owned(),
        revision: "1".to_owned(),
        digest: [0; 32],
    };
    let bytes = snapshot_bytes(&label, &model_digest, &[]);
    let digest = document_digest(&bytes);
    let label = DocumentRef { digest, ..label };

    let mut request = request(ClauseRunSelection::Function {
        name: "seven".to_owned(),
        arguments: Vec::new(),
        snapshot: label,
    });
    request.snapshots.insert(digest, bytes);
    let report = run_clause(request).unwrap();
    assert!(matches!(
        report.disposition,
        ClauseDisposition::NotAPredicate { .. }
    ));
    assert_eq!(report.disposition.stage(), ClauseRunStage::Select);
}

/// FR-098's stale-package rule (reused verbatim by FR-109): an
/// `expected_package_id` the recompile disagrees with refuses at stage
/// `compile`, before any selection.
#[trace("TC-468", "FR-109-AC-2")]
#[test]
fn run_clause_reports_a_stale_package() {
    let compiled = compiled();
    let real_id = compiled.emitted.package_id();
    let bogus_id = qsl_semantics::library::PackageId::of_preimage(b"not-the-real-package");
    assert_ne!(real_id, bogus_id, "the bogus id must actually differ");

    let mut request = request(ClauseRunSelection::Function {
        name: "seven".to_owned(),
        arguments: Vec::new(),
        snapshot: DocumentRef {
            authority: "test".to_owned(),
            identity: "snap".to_owned(),
            revision_namespace: "ns".to_owned(),
            revision: "1".to_owned(),
            digest: [0; 32],
        },
    });
    request.expected_package_id = Some(bogus_id);
    let report = run_clause(request).unwrap();
    assert!(matches!(
        report.disposition,
        ClauseDisposition::StalePackage { .. }
    ));
    assert_eq!(report.disposition.stage(), ClauseRunStage::Compile);
    assert_eq!(report.disposition.category().exit_code(), 20);
}

/// [`ClauseRunRefusal::EmptySource`]: an empty unit refuses before any
/// stage runs. Not one of FR-109's own AC rows (`ClauseRunRefusal` is only
/// for a request that cannot be formed, distinct from every `ClauseRunReport`
/// the AC table describes), so tagged by TC-468 alone.
#[trace("TC-468")]
#[test]
fn run_clause_refuses_an_empty_source() {
    let mut request = request(ClauseRunSelection::Function {
        name: "seven".to_owned(),
        arguments: Vec::new(),
        snapshot: DocumentRef {
            authority: "test".to_owned(),
            identity: "snap".to_owned(),
            revision_namespace: "ns".to_owned(),
            revision: "1".to_owned(),
            digest: [0; 32],
        },
    });
    request.source = ClauseRunSource::Program {
        identity: source(),
        path: "clause-run.native".to_owned(),
        bytes: Vec::new(),
    };
    assert!(matches!(
        run_clause(request),
        Err(ClauseRunRefusal::EmptySource)
    ));
}

/// TC-468 step 2 (FR-109-AC-2): no domain package bytes supplied for the
/// unit's `model` declaration refuses at stage `compile`,
/// `missing_import`/`missing-selection`, exit 20, with no `package_id`.
#[trace("TC-468", "FR-109-AC-2")]
#[test]
fn run_clause_reports_missing_import_when_no_package_bytes_are_supplied() {
    let label = DocumentRef {
        authority: "test".to_owned(),
        identity: "snap".to_owned(),
        revision_namespace: "ns".to_owned(),
        revision: "1".to_owned(),
        digest: [0; 32],
    };
    let mut request = request(ClauseRunSelection::Function {
        name: "seven".to_owned(),
        arguments: Vec::new(),
        snapshot: label,
    });
    // No snapshot bytes are inserted either: the request never gets past
    // compile, so nothing would be read even if a snapshot were named.
    request.packages = BTreeMap::new();
    let report = run_clause(request).unwrap();
    match &report.disposition {
        ClauseDisposition::Compile(refusal) => {
            assert_eq!(
                refusal.code(),
                qsl_foundation::diagnostic::Code::MissingImport
            );
            assert!(
                matches!(refusal.as_ref(), super::CompileRefusal::Intake { .. }),
                "expected an Intake refusal, got {refusal:?}"
            );
        }
        other => panic!("expected Compile(Intake(..)), got {other:?}"),
    }
    assert_eq!(report.disposition.stage(), ClauseRunStage::Compile);
    assert_eq!(report.disposition.category().exit_code(), 20);
    assert_eq!(report.package_id, None);
}

// ---------------------------------------------------------------------------
// FR-106 check 1.6: unknown top-level members report document order, not
// alphabetical order.
// ---------------------------------------------------------------------------

/// TC-465 (FR-106-AC-2): a snapshot document with two unknown top-level
/// members, "zzz_unknown" then "aaa_unknown" in the document's own text
/// order -- the reverse of their alphabetical order -- refuses naming
/// "zzz_unknown", the document-order-first one, never "aaa_unknown", the
/// alphabetically-first one. Before `OrderedJson` (this ticket's own fix,
/// `qsl-semantics/src/model/observation/ordered_json.rs`), `read_document`
/// walked a `serde_json::Value`'s `BTreeMap`-backed object and would have
/// named "aaa_unknown" instead.
#[trace("TC-465", "FR-106-AC-2")]
#[test]
fn run_clause_reports_the_document_order_first_unknown_member_not_the_alphabetical_one() {
    let model_digest = model_digest_hex();
    let label = DocumentRef {
        authority: "test".to_owned(),
        identity: "snap".to_owned(),
        revision_namespace: "ns".to_owned(),
        revision: "1".to_owned(),
        digest: [0; 32],
    };
    // Hand-written text, not `serde_json::json!` + `.to_string()`: without
    // the crate-wide `preserve_order` feature (deliberately off, see
    // `ordered_json.rs`'s own module doc), `json!`'s `Map` is `BTreeMap`-
    // backed and would serialize its members alphabetically regardless of
    // insertion order, destroying the very ordering this test exercises.
    let bytes = format!(
        r#"{{
            "format": "quire.state.snapshot/v1",
            "identity": {{"authority": "test", "identity": "snap", "revision_namespace": "ns", "revision": "1"}},
            "observation": "current",
            "model": {{"identity": "test/nodes", "version": "1.0.0", "digest": "sha256-jcs:{model_digest}"}},
            "populations": [],
            "zzz_unknown": true,
            "aaa_unknown": true
        }}"#
    )
    .into_bytes();
    let digest = document_digest(&bytes);
    let label = DocumentRef { digest, ..label };
    let selection = no_cycle_selection(label, "a");

    let mut request = request(ClauseRunSelection::Clause(selection));
    request.snapshots.insert(digest, bytes);
    let report = run_clause(request).expect("a well-formed request always reports");
    match report.disposition {
        ClauseDisposition::Admit(super::AdmissionFailure::Refused(record)) => {
            assert_eq!(record.cause, "unknown-member");
            assert_eq!(
                record.fields.get("field").map(String::as_str),
                Some("zzz_unknown")
            );
        }
        other => panic!("expected Admit(Refused(unknown-member)), got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// ADR-013 ruling (QSL-278 r6): `population_universe_for` must resolve the
// real `UniverseId` `crate::model::normalize` assigns an object's type, not
// an approximation over the population name -- a wrong universe gives a
// wrong `foreign_reference` result.
// ---------------------------------------------------------------------------

/// `population_universe_for` (admission's own sanctioned entry point) and a
/// direct, independent call to `crate::model::normalize::normalize` on the
/// same package must assign the `Node` type the same `UniverseId`. This
/// calls `normalize` itself, not `population_universe_for`'s internals, so
/// it is not circular: it is the coordinator-required proof that admission
/// and normalization agree over the same object type.
#[trace("TC-465", "FR-106-AC-1")]
#[test]
fn population_universe_for_agrees_with_normalize_s_own_universe_assignment() {
    let document = domain_document();
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let model_selection = qsl_semantics::model::domain_package::DomainPackageRef {
        identity: PACKAGE_IDENTITY.to_owned(),
        version: "1.0.0".to_owned(),
        digest: document_digest(&document),
    };

    // Admit and normalize the same package directly -- independent of
    // `population_universe_for`'s own internal re-derivation -- to learn
    // the `Node` type's real `EffectiveId` and the real `UniverseId`
    // normalization assigns its universe.
    let admitted = qsl_semantics::model::intake::admit_selections(
        std::slice::from_ref(&model_selection),
        qsl_semantics::model::key::SHA256_JCS_DIGEST_DOMAIN,
        &packages,
        qsl_foundation::IntakeLimits::default(),
    )
    .expect("the fixture package admits cleanly");
    let (package_ref, package_document) =
        admitted.into_iter().next().expect("one admitted package");
    let records =
        qsl_semantics::model::intake::read_records(&package_ref.identity, &package_document)
            .expect("the fixture package reads cleanly");
    let domain_package =
        qsl_semantics::model::domain_package::DomainPackage::new(package_ref, records);
    let limits = qsl_semantics::model::accounting::ModelNormalizationLimits::default();
    let view = match qsl_semantics::model::normalize::normalize(&domain_package, limits) {
        qsl_semantics::model::normalize::NormalizeOutcome::Completed(view) => view,
        other => panic!("expected the fixture package to normalize cleanly, got {other:?}"),
    };
    let node_key = qsl_semantics::model::key::DeclarationKey {
        package: PACKAGE_IDENTITY.to_owned(),
        node: node_type(),
    };
    let node_effective = *view
        .type_identities()
        .get(&node_key)
        .expect("Node is a declared object type");
    let expected_universe = view
        .object_universe_of(&node_key)
        .expect("Node has a universe")
        .identity();

    let admitted_universe = qsl_semantics::model::observation::population_universe_for(
        std::slice::from_ref(&model_selection),
        &packages,
        limits,
        node_effective,
    )
    .expect("population_universe_for resolves the same object type's universe");

    assert_eq!(
        admitted_universe, expected_universe,
        "admission's own universe resolution must agree with normalize's"
    );
}

// ---------------------------------------------------------------------------
// FR-106 check 4: the `model` member's own reader (`document::read_model`)
// refuses a missing or malformed member rather than silently defaulting to
// an empty string.
// ---------------------------------------------------------------------------

/// A snapshot document naming `label`, with `model` set to the raw JSON
/// `model_json` verbatim -- for exercising `read_model`'s own defect
/// reporting, never a well-formed `model` header.
fn snapshot_bytes_with_model(label: &DocumentRef, model_json: &str) -> Vec<u8> {
    format!(
        r#"{{
            "format": "quire.state.snapshot/v1",
            "identity": {{"authority": "{}", "identity": "{}", "revision_namespace": "{}", "revision": "{}"}},
            "observation": "current",
            "model": {model_json},
            "populations": []
        }}"#,
        label.authority, label.identity, label.revision_namespace, label.revision
    )
    .into_bytes()
}

/// FR-106 check 4 (`read_model`): a `model` member missing `digest`
/// refuses `invalid_runtime_input`/`missing-member` naming `digest`, not a
/// silent `""` that would later misreport `wrong-model-selection`.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn run_clause_refuses_a_model_with_a_missing_digest() {
    let label = DocumentRef {
        authority: "test".to_owned(),
        identity: "snap".to_owned(),
        revision_namespace: "ns".to_owned(),
        revision: "1".to_owned(),
        digest: [0; 32],
    };
    let bytes =
        snapshot_bytes_with_model(&label, r#"{"identity": "test/nodes", "version": "1.0.0"}"#);
    let digest = document_digest(&bytes);
    let label = DocumentRef { digest, ..label };
    let selection = no_cycle_selection(label, "a");

    let mut request = request(ClauseRunSelection::Clause(selection));
    request.snapshots.insert(digest, bytes);
    let report = run_clause(request).expect("a well-formed request always reports");
    match report.disposition {
        ClauseDisposition::Admit(super::AdmissionFailure::Refused(record)) => {
            assert_eq!(
                record.code,
                qsl_foundation::diagnostic::Code::InvalidRuntimeInput
            );
            assert_eq!(record.cause, "missing-member");
            assert_eq!(
                record.fields.get("field").map(String::as_str),
                Some("digest")
            );
        }
        other => panic!("expected Admit(Refused(missing-member/digest)), got {other:?}"),
    }
}

/// FR-106 check 4 (`read_model`): a `model.digest` not spelled with the
/// exact `sha256-jcs:` prefix refuses `invalid_runtime_input`/
/// `wrong-value-kind` naming `digest` -- `strip_prefix`, not
/// `trim_start_matches`, so a wrong prefix is a refusal, never a silent
/// pass-through of the whole unstripped string.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn run_clause_refuses_a_model_digest_with_the_wrong_prefix() {
    let label = DocumentRef {
        authority: "test".to_owned(),
        identity: "snap".to_owned(),
        revision_namespace: "ns".to_owned(),
        revision: "1".to_owned(),
        digest: [0; 32],
    };
    let model_digest = model_digest_hex();
    let bytes = snapshot_bytes_with_model(
        &label,
        &format!(
            r#"{{"identity": "test/nodes", "version": "1.0.0", "digest": "sha256:{model_digest}"}}"#
        ),
    );
    let digest = document_digest(&bytes);
    let label = DocumentRef { digest, ..label };
    let selection = no_cycle_selection(label, "a");

    let mut request = request(ClauseRunSelection::Clause(selection));
    request.snapshots.insert(digest, bytes);
    let report = run_clause(request).expect("a well-formed request always reports");
    match report.disposition {
        ClauseDisposition::Admit(super::AdmissionFailure::Refused(record)) => {
            assert_eq!(
                record.code,
                qsl_foundation::diagnostic::Code::InvalidRuntimeInput
            );
            assert_eq!(record.cause, "wrong-value-kind");
            assert_eq!(
                record.fields.get("field").map(String::as_str),
                Some("digest")
            );
        }
        other => panic!("expected Admit(Refused(wrong-value-kind/digest)), got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// TC-466 (FR-107-AC-1, AC-2, AC-3; SR-751 FND-003): S6a evaluates state
// clauses over their observations, `pre` reads and `reaches`, run through
// `run_clause` end to end -- genuine FR-106 admission (not a hand-built
// `AdmittedObservations`), over a `ConfigVersion` package distinct from this
// file's own `test/nodes`/`NoCycle` fixture (this crate has no path back
// into `qsl-semantics`' own `tests/it` integration binary, so TC-465's
// ConfigVersion fixture there cannot be reused here; this is a smaller,
// independently authored equivalent, in this file's own JSON idiom).
// ---------------------------------------------------------------------------

const CONFIG_VERSION_PACKAGE_IDENTITY: &str = "test/config-version";

fn config_version_type() -> String {
    format!("ix://{CONFIG_VERSION_PACKAGE_IDENTITY}/ConfigVersion")
}

fn config_version_population_identity() -> String {
    format!("ix://{CONFIG_VERSION_PACKAGE_IDENTITY}/config_history")
}

/// FR-108's own bound: `versionNumber` is `Int[0, 1000]`, a package-declared
/// `VersionNumber` value type (FR-056's `value-type/v1` scalar reader),
/// not the native `Integer` this fixture formerly used --
/// the below-range and above-range corpus cases need a real domain bound to
/// refuse against.
fn version_number_type() -> String {
    format!("ix://{CONFIG_VERSION_PACKAGE_IDENTITY}/VersionNumber")
}

/// The `test/config-version` domain package: `ConfigVersion` (`versionNumber:
/// VersionNumber` (`Int[0, 1000]`), `parent: Reference<ConfigVersion>?`),
/// one operation `attemptUpdate` (FR-105-AC-1's own frame, `modifies
/// [versionNumber]` only, no parameters, returns `Boolean`), one closed
/// population `config_history`.
fn config_version_domain_document() -> Vec<u8> {
    config_version_domain_document_with_operations(Vec::new())
}

/// [`config_version_domain_document`], with `extra_operations` appended
/// after `attemptUpdate` (TC-466 step 3): the one fact "what
/// `ConfigVersion` operations exist" stays in this one builder rather than
/// a second, hand-duplicated envelope for step 3's `probe`.
fn config_version_domain_document_with_operations(
    extra_operations: Vec<serde_json::Value>,
) -> Vec<u8> {
    let config_version = config_version_type();
    let version_number = version_number_type();
    let version_number_identity = format!("{config_version}/versionNumber");
    let parent_identity = format!("{config_version}/parent");
    let operation_identity = format!("{config_version}/attemptUpdate");
    let population = config_version_population_identity();
    let bound = |keyword: &str, value: &str| {
        json!({
            "identity": format!("{version_number}/constraints/{keyword}"),
            "keyword": keyword,
            "operands": {"value": value},
            "appliesTo": "ix://quire/native/Integer",
            "diagnosticCode": format!("bound.{keyword}"),
            "origin": {
                "generated": {
                    "generatorIdentity": version_number.clone(),
                    "generatorVersion": "1.0.0",
                    "inputIdentities": [version_number.clone()],
                }
            },
        })
    };
    let mut operations = vec![json!({
        "identity": operation_identity,
        "name": "attemptUpdate",
        "params": [],
        "returns": {
            "typeRef": "ix://quire/native/Boolean",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "nullable": false,
        },
        "pre": [],
        "post": [],
        "origin": {
            "source": {
                "sourceIdentity": format!("ix://{CONFIG_VERSION_PACKAGE_IDENTITY}/spec"),
                "path": "spec.qspec",
                "startLine": 1,
                "startColumn": 1,
            },
        },
        // FR-105-AC-1: `attemptUpdate`'s frame modifies exactly
        // `versionNumber`, never `parent`.
        "frame": {
            "modifies": [version_number_identity],
            "creates": [],
            "deletes": [],
        },
    })];
    operations.extend(extra_operations);
    let envelope = json!({
        "contractVersion": "2.0.0",
        "source": {
            "identity": format!("ix://{CONFIG_VERSION_PACKAGE_IDENTITY}/spec"),
            "version": "1.0.0",
            "dialect": "spec-bundle",
            "digest": PLACEHOLDER_DIGEST,
        },
        "package": {
            "identity": CONFIG_VERSION_PACKAGE_IDENTITY,
            "version": "1.0.0",
            "manifestDigest": PLACEHOLDER_DIGEST,
            "mappingVersions": [],
            "profileVersions": [],
            "lockDigest": PLACEHOLDER_DIGEST,
        },
        "occurrences": [],
        "extensions": [],
        "constructs": [
            {
                "kind": {"module": CONFIG_VERSION_PACKAGE_IDENTITY, "name": "object_type"},
                "moduleVersion": "1.0.0",
                "manifestDigest": PLACEHOLDER_DIGEST,
                "construct": {
                    "identity": "none",
                    "shape": "record",
                    "members": {},
                    "meaning": "quire.meaning.model.object-type/v1",
                },
            },
            {
                "kind": {"module": CONFIG_VERSION_PACKAGE_IDENTITY, "name": "population"},
                "moduleVersion": "1.0.0",
                "manifestDigest": PLACEHOLDER_DIGEST,
                "construct": {
                    "identity": "none",
                    "shape": "record",
                    "members": {},
                    "meaning": "quire.meaning.model.population/v1",
                },
            },
            {
                "kind": {"module": CONFIG_VERSION_PACKAGE_IDENTITY, "name": "value_type"},
                "moduleVersion": "1.0.0",
                "manifestDigest": PLACEHOLDER_DIGEST,
                "construct": {
                    "identity": "none",
                    "shape": "record",
                    "members": {},
                    "meaning": "quire.meaning.model.value-type/v1",
                },
            },
        ],
        "types": [
            {
                "identity": version_number.clone(),
                "displayName": version_number.clone(),
                "kind": {"module": CONFIG_VERSION_PACKAGE_IDENTITY, "name": "value_type"},
                "roles": [],
                "origin": {
                    "generated": {
                        "generatorIdentity": version_number.clone(),
                        "generatorVersion": "1.0.0",
                        "inputIdentities": [version_number.clone()],
                    }
                },
                "extensions": [],
                "unknownPolicy": "reject",
                "scalar": "integer",
                "constraints": [bound("min", "0"), bound("max", "1000")],
            },
            {
                "identity": config_version,
                "displayName": config_version,
                "kind": {"module": CONFIG_VERSION_PACKAGE_IDENTITY, "name": "object_type"},
                "roles": [],
                "origin": {
                    "generated": {
                        "generatorIdentity": config_version,
                        "generatorVersion": "1.0.0",
                        "inputIdentities": [config_version],
                    }
                },
                "constraints": [],
                "extensions": [],
                "unknownPolicy": "reject",
                "supertypes": [],
                "fields": [
                    {
                        "identity": version_number_identity,
                        "name": "versionNumber",
                        "typeRef": version_number,
                        "presence": "required",
                        "nullable": false,
                        "defaultKind": "none",
                        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                        "origin": {
                            "generated": {
                                "generatorIdentity": version_number_identity,
                                "generatorVersion": "1.0.0",
                                "inputIdentities": [version_number_identity],
                            }
                        },
                    },
                    {
                        "identity": parent_identity,
                        "name": "parent",
                        "typeRef": config_version,
                        "presence": "optional",
                        "nullable": false,
                        "defaultKind": "none",
                        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                        "origin": {
                            "generated": {
                                "generatorIdentity": parent_identity,
                                "generatorVersion": "1.0.0",
                                "inputIdentities": [parent_identity],
                            }
                        },
                    },
                ],
                "operations": operations,
            },
        ],
        "populations": [
            {
                "identity": population,
                "displayName": population,
                "kind": {"module": CONFIG_VERSION_PACKAGE_IDENTITY, "name": "population"},
                "members": [config_version],
                "extent": "closed",
                "origin": {
                    "generated": {
                        "generatorIdentity": population,
                        "generatorVersion": "1.0.0",
                        "inputIdentities": [population],
                    }
                },
            },
        ],
    });
    envelope.to_string().into_bytes()
}

/// The unit text selecting [`config_version_domain_document`] as model alias
/// `Config`: FR-108's own `ParentOrder`, `NoCycle` and `VersionUnchanged`
/// clauses, plus `sameIdentity` and `n`.
///
/// FR-105 implements the `state`/`state_clause`,
/// `state`/`operation_anchor` and `state`/`frame` emission a postcondition
/// of `attemptUpdate` needs to reach S5: before it, any clause naming the
/// operation refused `EmitRefusal::UnlocatedOccurrence` at S5
/// (`qsl_package::emit::emit_checked`), because S4 emitted no `state` node
/// at all ("Today QSL emits no `state` node",
/// `spec/functional/FR-105-emit-state-nodes.md`). `VersionUnchanged` below
/// is that postcondition, now reachable through `run_clause` end to end.
fn config_version_unit_and_packages() -> (String, BTreeMap<[u8; 32], Vec<u8>>) {
    config_version_unit_and_packages_for(config_version_domain_document())
}

/// [`config_version_unit_and_packages`]'s unit, selecting `document` (a
/// variant of [`config_version_domain_document`]) as model `Config`.
fn config_version_unit_and_packages_for(
    document: Vec<u8>,
) -> (String, BTreeMap<[u8; 32], Vec<u8>>) {
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let digest = hex(digest);
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model Config = {CONFIG_VERSION_PACKAGE_IDENTITY:?} version \"1.0.0\" \
         digest \"sha256-jcs:{digest}\";\n\
         invariant ParentOrder using v on Config::ConfigVersion at current {{ \
         present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber }}\n\
         invariant NoCycle using v on Config::ConfigVersion at current {{ \
         not reaches(self, self, parent) }}\n\
         post VersionUnchanged using v on Config::ConfigVersion::attemptUpdate {{ \
         self.versionNumber = pre(self.versionNumber) }}\n\
         function sameIdentity using v(a: Config::ConfigVersion, b: Config::ConfigVersion): \
         Boolean pure {{ a = b }}\n\
         function n using v(a: Config::ConfigVersion): Integer pure {{ 1 }}\n"
    );
    (unit, packages)
}

fn config_version_model_digest_hex() -> String {
    let (unit, packages) = config_version_unit_and_packages();
    let _ = unit;
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    hex(digest)
}

/// The shared `ClauseRunRequest` shape for a config-version unit: `unit` and
/// `packages` (either the base unit or a unit extending it) supply the
/// program, `path` names it for diagnostics, and every other member is the
/// same across every config-version test.
fn config_version_request_for(
    unit: String,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
    path: &str,
    selection: ClauseRunSelection,
) -> ClauseRunRequest {
    ClauseRunRequest {
        source: ClauseRunSource::Program {
            identity: source(),
            path: path.to_owned(),
            bytes: unit.into_bytes(),
        },
        packages,
        dependencies: DependencyInput::default(),
        snapshots: BTreeMap::new(),
        invocations: BTreeMap::new(),
        selection,
        expected_package_id: None,
        limits: SpineLimits::default(),
        observation_limits: ObservationLimits::default(),
        accounting: default_accounting(1_000_000),
    }
}

fn config_version_request(selection: ClauseRunSelection) -> ClauseRunRequest {
    let (unit, packages) = config_version_unit_and_packages();
    config_version_request_for(
        unit,
        packages,
        "clause-run-config-version.native",
        selection,
    )
}

fn config_version_document_ref(identity: &str) -> DocumentRef {
    DocumentRef {
        authority: "test".to_owned(),
        identity: identity.to_owned(),
        revision_namespace: "ns".to_owned(),
        revision: "1".to_owned(),
        digest: [0; 32],
    }
}

/// The `root`/`child` population objects shared by every config-version
/// snapshot: `root` at `root_version` (no parent), and, when `child_version`
/// is `Some`, `child` at that version naming `root`. `child_version` is
/// `None` for the absent-parent case (a single `root` object).
fn config_version_objects(root_version: i64, child_version: Option<i64>) -> Vec<serde_json::Value> {
    let population = config_version_population_identity();
    let config_version = config_version_type();
    let mut objects = vec![json!({
        "key": "root", "type": config_version,
        "fields": {"versionNumber": {"integer": root_version.to_string()}, "parent": {"absent": {}}},
    })];
    if let Some(child_version) = child_version {
        objects.push(json!({
            "key": "child", "type": config_version,
            "fields": {
                "versionNumber": {"integer": child_version.to_string()},
                "parent": {"present": {"reference": {"population": population, "key": "root"}}},
            },
        }));
    }
    objects
}

/// A current snapshot: `root` at `root_version` (no parent), `child` at
/// `child_version` naming `root` (or, when `root_version` is `None`, a
/// single `root` object with no parent -- the absent-parent case).
fn config_version_snapshot(
    label: &DocumentRef,
    model_digest_hex: &str,
    root_version: i64,
    child_version: Option<i64>,
) -> Vec<u8> {
    let population = config_version_population_identity();
    let objects = config_version_objects(root_version, child_version);
    let value = json!({
        "format": "quire.state.snapshot/v1",
        "identity": document_identity_json(label),
        "observation": "current",
        "anchor": {"kind": "handler", "name": "validate"},
        "model": {
            "identity": CONFIG_VERSION_PACKAGE_IDENTITY, "version": "1.0.0",
            "digest": format!("sha256-jcs:{model_digest_hex}"),
        },
        "populations": [{"population": population, "complete": true, "objects": objects}],
    });
    value.to_string().into_bytes()
}

/// A `ClauseSelection` naming `clause_name` at `label`'s current snapshot,
/// self object `self_key`.
fn config_version_current_selection(
    clause_name: &str,
    label: DocumentRef,
    self_key: &str,
) -> ClauseSelection {
    ClauseSelection {
        name: clause_name.to_owned(),
        input: ClauseSelectionInput::Current {
            snapshot: label,
            anchor: SelectedAnchor {
                kind: qsl_semantics::model::observation::AnchorKind::Handler,
                name: "validate".to_owned(),
            },
            self_object: SelectedObject {
                population: config_version_population_identity(),
                key: self_key.to_owned(),
            },
        },
    }
}

/// Runs `clause_name` (an invariant) over one current snapshot, through
/// `run_clause` end to end (genuine FR-106 admission, then S6a).
fn run_config_version_current(
    clause_name: &str,
    root_version: i64,
    child_version: Option<i64>,
    self_key: &str,
) -> ClauseDisposition {
    let model_digest_hex = config_version_model_digest_hex();
    let label = config_version_document_ref("current-snap");
    let bytes = config_version_snapshot(&label, &model_digest_hex, root_version, child_version);
    let digest = document_digest(&bytes);
    let label = DocumentRef { digest, ..label };
    let selection = config_version_current_selection(clause_name, label, self_key);

    let mut request = config_version_request(ClauseRunSelection::Clause(selection));
    request.snapshots.insert(digest, bytes);
    run_clause(request)
        .expect("a well-formed request always reports")
        .disposition
}

fn boolean_disposition(disposition: &ClauseDisposition) -> bool {
    match disposition {
        ClauseDisposition::Evaluate(super::CallOutcome::Completed(super::CallValue::Boolean(
            value,
        ))) => *value,
        other => panic!("expected Evaluate(Completed(Boolean(_))), got {other:?}"),
    }
}

/// A `ClauseRunRequest` selecting `Function { name, arguments, snapshot }`
/// over `config_version_snapshot`'s distinct-identities case (`root` at
/// version 1, `child` at version 2).
fn config_version_function_request(name: &str, arguments: Vec<ClauseArgument>) -> ClauseRunRequest {
    let model_digest_hex = config_version_model_digest_hex();
    let label = config_version_document_ref("distinct-identities");
    let bytes = config_version_snapshot(&label, &model_digest_hex, 1, Some(2));
    let digest = document_digest(&bytes);
    let label = DocumentRef { digest, ..label };

    let mut request = config_version_request(ClauseRunSelection::Function {
        name: name.to_owned(),
        arguments,
        snapshot: label,
    });
    request.snapshots.insert(digest, bytes);
    request
}

fn config_version_reference(key: &str) -> ClauseArgumentValue {
    ClauseArgumentValue::Reference {
        population: config_version_population_identity(),
        key: key.to_owned(),
    }
}

/// TC-468 step 2 (FR-109-AC-2): a `Clause` selection naming `Absent` (no
/// such clause is declared) refuses `select`, `missing_declaration`/
/// `missing-name`.
#[trace("TC-468", "FR-109-AC-2")]
#[test]
fn run_clause_reports_missing_name_selecting_absent_clause() {
    let model_digest_hex = config_version_model_digest_hex();
    let label = config_version_document_ref("current-snap");
    let bytes = config_version_snapshot(&label, &model_digest_hex, 1, None);
    let digest = document_digest(&bytes);
    let label = DocumentRef { digest, ..label };
    let selection = config_version_current_selection("Absent", label, "root");

    let mut request = config_version_request(ClauseRunSelection::Clause(selection));
    request.snapshots.insert(digest, bytes);
    let report = run_clause(request).unwrap();
    assert!(matches!(
        report.disposition,
        ClauseDisposition::MissingName { .. }
    ));
    assert_eq!(report.disposition.stage(), ClauseRunStage::Select);
}

/// TC-468 step 2 (FR-109-AC-2): a `Clause` selection naming the function
/// `sameIdentity` (declared `function`, not `invariant`/`protocol`) refuses
/// `select`, `missing_declaration`/`missing-name`: no state clause named
/// `sameIdentity` is declared, only a function.
#[trace("TC-468", "FR-109-AC-2")]
#[test]
fn run_clause_reports_missing_name_for_a_clause_selection_naming_a_function() {
    let model_digest_hex = config_version_model_digest_hex();
    let label = config_version_document_ref("current-snap");
    let bytes = config_version_snapshot(&label, &model_digest_hex, 1, None);
    let digest = document_digest(&bytes);
    let label = DocumentRef { digest, ..label };
    let selection = config_version_current_selection("sameIdentity", label, "root");

    let mut request = config_version_request(ClauseRunSelection::Clause(selection));
    request.snapshots.insert(digest, bytes);
    let report = run_clause(request).unwrap();
    assert!(matches!(
        report.disposition,
        ClauseDisposition::MissingName { .. }
    ));
    assert_eq!(report.disposition.stage(), ClauseRunStage::Select);
}

/// TC-468 step 2 (FR-109-AC-2): an `expected_package_id` taken from
/// compiling a different unit (the `test/model` fixture, not
/// `test/config-version`) refuses `compile`, `stale_dependency`, naming
/// both identities.
#[trace("TC-468", "FR-109-AC-2")]
#[test]
fn run_clause_reports_a_stale_package_naming_another_units_package_id() {
    let other = compiled();
    let other_id = other.emitted.package_id();

    let mut request = config_version_request(ClauseRunSelection::Function {
        name: "n".to_owned(),
        arguments: vec![ClauseArgument {
            parameter: "a".to_owned(),
            value: config_version_reference("root"),
        }],
        snapshot: config_version_document_ref("current-snap"),
    });
    request.expected_package_id = Some(other_id);
    let report = run_clause(request).unwrap();
    match &report.disposition {
        ClauseDisposition::StalePackage {
            expected, actual, ..
        } => {
            assert_eq!(*expected, other_id);
            assert_ne!(*actual, other_id, "the config-version package differs");
        }
        other => panic!("expected StalePackage {{ .. }}, got {other:?}"),
    }
    assert_eq!(report.disposition.stage(), ClauseRunStage::Compile);
    assert_eq!(report.disposition.category().exit_code(), 20);
}

/// TC-468 step 3 (FR-109-AC-2): a dangling `parent` reference (naming a key
/// no admitted population holds) refuses `admit`, `refusal`,
/// `dangling_reference`, exit 20.
#[trace("TC-468", "FR-109-AC-2")]
#[test]
fn run_clause_refuses_a_dangling_parent_reference() {
    let model_digest_hex = config_version_model_digest_hex();
    let label = config_version_document_ref("dangling-parent");
    let population = config_version_population_identity();
    let config_version = config_version_type();
    let value = json!({
        "format": "quire.state.snapshot/v1",
        "identity": {
            "authority": label.authority, "identity": label.identity,
            "revision_namespace": label.revision_namespace, "revision": label.revision,
        },
        "observation": "current",
        "anchor": {"kind": "handler", "name": "validate"},
        "model": {
            "identity": CONFIG_VERSION_PACKAGE_IDENTITY, "version": "1.0.0",
            "digest": format!("sha256-jcs:{model_digest_hex}"),
        },
        "populations": [{
            "population": population,
            "complete": true,
            "objects": [{
                "key": "child", "type": config_version,
                "fields": {
                    "versionNumber": {"integer": "2"},
                    "parent": {"present": {"reference": {"population": population, "key": "ghost"}}},
                },
            }],
        }],
    });
    let bytes = value.to_string().into_bytes();
    let digest = document_digest(&bytes);
    let label = DocumentRef { digest, ..label };
    let selection = config_version_current_selection("ParentOrder", label, "child");

    let mut request = config_version_request(ClauseRunSelection::Clause(selection));
    request.snapshots.insert(digest, bytes);
    let report = run_clause(request).unwrap();
    assert_eq!(report.disposition.stage(), ClauseRunStage::Admit);
    match &report.disposition {
        ClauseDisposition::Admit(qsl_semantics::model::observation::AdmissionFailure::Refused(
            record,
        )) => {
            assert_eq!(
                record.code,
                qsl_foundation::diagnostic::Code::DanglingReference
            );
        }
        other => panic!("expected Admit(Refused(dangling_reference)), got {other:?}"),
    }
    assert_eq!(report.disposition.category().exit_code(), 20);
}

/// TC-468 step 3 (FR-109-AC-2): an incomplete population (`complete:
/// false`, missing the referenced `root` member) refuses `admit`,
/// `incomplete`, `incomplete_population`, exit 22.
#[trace("TC-468", "FR-109-AC-2")]
#[test]
fn run_clause_reports_incomplete_for_an_incomplete_population() {
    let model_digest_hex = config_version_model_digest_hex();
    let label = config_version_document_ref("incomplete-population");
    let population = config_version_population_identity();
    let config_version = config_version_type();
    let value = json!({
        "format": "quire.state.snapshot/v1",
        "identity": {
            "authority": label.authority, "identity": label.identity,
            "revision_namespace": label.revision_namespace, "revision": label.revision,
        },
        "observation": "current",
        "anchor": {"kind": "handler", "name": "validate"},
        "model": {
            "identity": CONFIG_VERSION_PACKAGE_IDENTITY, "version": "1.0.0",
            "digest": format!("sha256-jcs:{model_digest_hex}"),
        },
        "populations": [{
            "population": population,
            "complete": false,
            "objects": [{
                "key": "child", "type": config_version,
                "fields": {
                    "versionNumber": {"integer": "2"},
                    "parent": {"present": {"reference": {"population": population, "key": "root"}}},
                },
            }],
        }],
    });
    let bytes = value.to_string().into_bytes();
    let digest = document_digest(&bytes);
    let label = DocumentRef { digest, ..label };
    let selection = config_version_current_selection("ParentOrder", label, "child");

    let mut request = config_version_request(ClauseRunSelection::Clause(selection));
    request.snapshots.insert(digest, bytes);
    let report = run_clause(request).unwrap();
    assert_eq!(report.disposition.stage(), ClauseRunStage::Admit);
    match &report.disposition {
        ClauseDisposition::Admit(
            qsl_semantics::model::observation::AdmissionFailure::Incomplete(record),
        ) => {
            assert_eq!(
                record.code,
                qsl_foundation::diagnostic::Code::IncompletePopulation
            );
        }
        other => panic!("expected Admit(Incomplete(incomplete_population)), got {other:?}"),
    }
    assert_eq!(
        report.disposition.category(),
        qsl_foundation::diagnostic::Category::Incomplete
    );
    assert_eq!(report.disposition.category().exit_code(), 22);
}

/// TC-468 step 3 (FR-109-AC-2): a `work_units` budget of 0 exhausts before
/// `ParentOrder` completes: `evaluate`, `incomplete`, limit `work_units`,
/// exit 22, no `truth`.
#[trace("TC-468", "FR-109-AC-2")]
#[test]
fn run_clause_reports_incomplete_when_work_units_are_exhausted() {
    let model_digest_hex = config_version_model_digest_hex();
    let label = config_version_document_ref("current-snap");
    let bytes = config_version_snapshot(&label, &model_digest_hex, 1, Some(2));
    let digest = document_digest(&bytes);
    let label = DocumentRef { digest, ..label };
    let selection = config_version_current_selection("ParentOrder", label, "child");

    let mut request = config_version_request(ClauseRunSelection::Clause(selection));
    request.snapshots.insert(digest, bytes);
    request.accounting = default_accounting(0);
    let report = run_clause(request).unwrap();
    assert_eq!(report.disposition.stage(), ClauseRunStage::Evaluate);
    match &report.disposition {
        ClauseDisposition::Evaluate(super::CallOutcome::Incomplete(incomplete)) => {
            assert_eq!(
                incomplete.limits_field(),
                quire_exact::LimitKind::WorkUnits.as_str()
            );
        }
        other => panic!("expected Evaluate(Incomplete {{ .. }}), got {other:?}"),
    }
    assert_eq!(
        report.disposition.category(),
        qsl_foundation::diagnostic::Category::Incomplete
    );
    assert_eq!(report.disposition.truth(), None);
    assert_eq!(report.disposition.category().exit_code(), 22);
}

/// TC-468 step 4 (FR-109-AC-4): the `sameIdentity` family over
/// distinct-identities (`root` v1, `child` v2, arguments given
/// out-of-declaration-order, `b` before `a`, matched by name).
#[trace("TC-468", "FR-109-AC-4")]
#[test]
fn run_clause_evaluates_the_same_identity_family() {
    // b: child, a: root -- distinct objects, violation.
    let request = config_version_function_request(
        "sameIdentity",
        vec![
            ClauseArgument {
                parameter: "b".to_owned(),
                value: config_version_reference("child"),
            },
            ClauseArgument {
                parameter: "a".to_owned(),
                value: config_version_reference("root"),
            },
        ],
    );
    let report = run_clause(request).expect("well-formed request reports");
    assert!(!boolean_disposition(&report.disposition), "root != child");
    assert_eq!(
        report.disposition.category(),
        qsl_foundation::diagnostic::Category::Violation
    );

    // a = b = child -- same object, success.
    let request = config_version_function_request(
        "sameIdentity",
        vec![
            ClauseArgument {
                parameter: "b".to_owned(),
                value: config_version_reference("child"),
            },
            ClauseArgument {
                parameter: "a".to_owned(),
                value: config_version_reference("child"),
            },
        ],
    );
    let report = run_clause(request).expect("well-formed request reports");
    assert!(boolean_disposition(&report.disposition), "child = child");
    assert_eq!(
        report.disposition.category(),
        qsl_foundation::diagnostic::Category::Success
    );

    // b names `ghost`, no admitted object -- admit refusal, wrong-role-mapping.
    let request = config_version_function_request(
        "sameIdentity",
        vec![
            ClauseArgument {
                parameter: "b".to_owned(),
                value: config_version_reference("ghost"),
            },
            ClauseArgument {
                parameter: "a".to_owned(),
                value: config_version_reference("root"),
            },
        ],
    );
    let report = run_clause(request).expect("well-formed request reports");
    assert_eq!(report.disposition.stage(), ClauseRunStage::Admit);
    match &report.disposition {
        ClauseDisposition::Admit(qsl_semantics::model::observation::AdmissionFailure::Refused(
            record,
        )) => {
            assert_eq!(
                record.code,
                qsl_foundation::diagnostic::Code::InvalidRuntimeInput
            );
            assert_eq!(record.cause, "wrong-role-mapping");
        }
        other => panic!("expected Admit(Refused(wrong-role-mapping)), got {other:?}"),
    }

    // An extra argument `c` -- FR-100's refusal for an unknown parameter,
    // stage admit.
    let request = config_version_function_request(
        "sameIdentity",
        vec![
            ClauseArgument {
                parameter: "b".to_owned(),
                value: config_version_reference("child"),
            },
            ClauseArgument {
                parameter: "a".to_owned(),
                value: config_version_reference("root"),
            },
            ClauseArgument {
                parameter: "c".to_owned(),
                value: config_version_reference("root"),
            },
        ],
    );
    let report = run_clause(request).expect("well-formed request reports");
    assert_eq!(report.disposition.stage(), ClauseRunStage::Admit);
    assert_eq!(
        report.disposition.category(),
        qsl_foundation::diagnostic::Category::Refusal
    );

    // `n(a: Config::ConfigVersion): Integer`, selected the same way, is not
    // a predicate -- stage select, ill_typed/type-mismatch, uncharged meter.
    let request = config_version_function_request(
        "n",
        vec![ClauseArgument {
            parameter: "a".to_owned(),
            value: config_version_reference("root"),
        }],
    );
    let report = run_clause(request).expect("well-formed request reports");
    assert_eq!(report.disposition.stage(), ClauseRunStage::Select);
    assert!(matches!(
        report.disposition,
        ClauseDisposition::NotAPredicate { .. }
    ));
    assert_eq!(report.usage.evaluation_admissions, 0, "meter never charged");
}

/// FR-106 check 1.3 (TC-465): editing the snapshot bytes after the
/// selection's digest was taken (so the stored digest no longer matches the
/// bytes it labels) refuses `admit`, `stale_dependency`/
/// `content-mismatch`.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
fn run_clause_refuses_a_snapshot_edited_after_its_digest_was_taken() {
    let model_digest_hex = config_version_model_digest_hex();
    let label = config_version_document_ref("current-snap");
    let bytes = config_version_snapshot(&label, &model_digest_hex, 1, None);
    let digest = document_digest(&bytes);
    let label = DocumentRef { digest, ..label };
    let selection = config_version_current_selection("ParentOrder", label, "root");

    // The selection's digest was taken over `bytes`; store different bytes
    // (root at a different version) under that same digest key.
    let edited = config_version_snapshot(
        &DocumentRef {
            digest: [0; 32],
            ..config_version_document_ref("current-snap")
        },
        &model_digest_hex,
        99,
        None,
    );

    let mut request = config_version_request(ClauseRunSelection::Clause(selection));
    request.snapshots.insert(digest, edited);
    let report = run_clause(request).unwrap();
    assert_eq!(report.disposition.stage(), ClauseRunStage::Admit);
    match &report.disposition {
        ClauseDisposition::Admit(qsl_semantics::model::observation::AdmissionFailure::Refused(
            record,
        )) => {
            assert_eq!(
                record.code,
                qsl_foundation::diagnostic::Code::StaleDependency
            );
            assert_eq!(record.cause, "content-mismatch");
        }
        other => panic!("expected Admit(Refused(content-mismatch)), got {other:?}"),
    }
}

/// TC-468 step 5 (FR-109-AC-5): `FR-100`'s outcome mapping
/// ([`super::call::convert_outcome`]) maps the kernel `CheckedInvariant`
/// refusal to `Err(RunRefusal::Fault(InternalFault { stage: "S6a",
/// invariant: "checked-program-invariant", .. }))`
/// (`qsl-replay/src/spine/call.rs:526-528`); `run_clause`'s own further
/// wrapping (`clause.rs`'s `EvaluateFault` arm) reports stage `evaluate`,
/// category `internal-failure`, no `outcome` member, and FR-100's
/// internal-failure exit status (30) for both this kernel invariant and a
/// `CallFailure::Fault` reaching the same arm
/// (`clause.rs:523-524`) -- confirmed directly against the mapping
/// functions themselves, since `CheckedInvariant` is deliberately
/// unreachable through any legitimate evaluation
/// (`qsl-eval/src/value/expression/evaluate.rs:623`, "reserved for a broken
/// evaluator invariant, never actually reachable in production").
#[trace("TC-468", "FR-109-AC-5")]
#[test]
fn checked_invariant_and_call_fault_both_report_the_same_internal_failure_shape() {
    let compiled = compiled();
    let sources = std::slice::from_ref(&compiled.source);

    let checked_invariant_evaluation = qsl_eval::value::Evaluation {
        outcome: FamilyOutcome::Evaluated(quire_exact::Outcome::Refused(
            quire_exact::Refusal::CheckedInvariant { cause: quire_exact::CheckedInvariantCause::EqualityOperandSourceNotAdmitted },
        )),
        location: None,
        losses: Vec::new(),
    };
    let mapped = super::super::call::convert_outcome(
        checked_invariant_evaluation,
        compiled.package.graph(),
        sources,
    );
    let fault = match mapped {
        Err(fault) => match *fault {
            super::super::RunRefusal::Fault(fault) => fault,
            other => panic!("expected RunRefusal::Fault, got {other:?}"),
        },
        Ok(outcome) => panic!("expected Err(Fault(..)), got {outcome:?}"),
    };
    assert_eq!(fault.stage(), "S6a");
    assert_eq!(fault.invariant(), "checked-program-invariant");

    let disposition = ClauseDisposition::EvaluateFault(fault);
    assert_eq!(disposition.stage(), ClauseRunStage::Evaluate);
    assert_eq!(
        disposition.category(),
        qsl_foundation::diagnostic::Category::InternalFailure
    );
    assert_eq!(disposition.truth(), None);
    assert_eq!(disposition.category().exit_code(), 30);

    // A `CallFailure::Fault` reaches `run_clause`'s own `EvaluateFault` arm
    // the same way (`clause.rs:523-524`), with the same reported shape --
    // constructed directly here, not run through `package.evaluate_clause`
    // (SR-751 FND-008 round 2, disclosed, not attempted): every
    // `CallFailure::Fault` `evaluate_clause` can itself construct
    // (`qsl-eval/src/value/expression/mod.rs:456,475,486`,
    // "clause-observations-missing-the-clause-observation"/
    // "postcondition-result-missing"/"clause-parameter-not-admitted") is
    // unreachable once admission has actually succeeded: FR-106 always
    // admits the observation the clause kind reads -- `current` for an
    // invariant, `pre` and `post` for an invocation, and `pre` for a
    // pre-call precondition, which leaves `current` and `post` both `None`
    // and is read through `pre` -- always
    // admits a `result` for an operation `binds_result()` declares
    // (refusing at admission otherwise, never reaching evaluate), and
    // always admits every one of the clause's own declared parameters
    // (check 10) -- the same "broken invariant, not a document defect"
    // shape `CheckedInvariant` above already is. This test's own fixture
    // (`compiled()`, `unit_and_packages()`) still has no precondition or
    // postcondition to attempt it against -- `test/nodes` declares only
    // `NoCycle`, an invariant. FR-105 lifted the compile-time blocker this
    // comment used to describe (SR-751 FND-003): `post VersionUnchanged` on
    // `attemptUpdate` now compiles through the spine, in
    // `config_version_compiled()` below, in this same file. Constructing
    // `CallFailure::Fault` directly here rather than through
    // `package.evaluate_clause` against that fixture is still the
    // deliberate choice this test makes (disclosed above), not one forced
    // by a compile-time gap any more.
    let call_failure_fault = qsl_foundation::diagnostic::InternalFault::new("call", "fault-kind");
    let disposition_from_call_failure = ClauseDisposition::EvaluateFault(call_failure_fault);
    assert_eq!(
        disposition_from_call_failure.stage(),
        ClauseRunStage::Evaluate
    );
    assert_eq!(
        disposition_from_call_failure.category(),
        qsl_foundation::diagnostic::Category::InternalFailure
    );
    assert_eq!(disposition_from_call_failure.truth(), None);
    assert_eq!(disposition_from_call_failure.category().exit_code(), 30);
}

/// SR-751 FND-005 round 2: the internal-failure comparison above covers
/// only two of FR-100-AC-9's enumerated outcome kinds. This covers four
/// more of the general (non-internal-failure) kinds `convert_outcome`
/// maps -- `Completed(true)`, `Completed(false)`, a kernel `Undefined`
/// and a kernel `Incomplete` -- each built as a real
/// `qsl_eval::value::Evaluation`, run through `super::super::
/// call::convert_outcome` itself (never a hand-copied expected
/// `CallOutcome`), and only then checked against
/// `ClauseDisposition::category`/`exit_code`'s own mapping (FR-301's exit
/// 0/10, and FR-100-AC-9's 20/22 for `Undefined`/`Incomplete`).
#[trace("TC-468", "FR-109-AC-5")]
#[test]
fn convert_outcome_drives_the_disposition_for_every_general_outcome_kind() {
    let compiled = compiled();
    let sources = std::slice::from_ref(&compiled.source);
    let graph = compiled.package.graph();

    let evaluation = |outcome| qsl_eval::value::Evaluation {
        outcome,
        location: None,
        losses: Vec::new(),
    };

    // Completed(true): success, exit 0.
    let mapped = super::super::call::convert_outcome(
        evaluation(FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(
            quire_exact::Value::Boolean(true),
        ))),
        graph,
        sources,
    )
    .expect("Completed(true) maps to Ok");
    let disposition = ClauseDisposition::Evaluate(mapped);
    assert_eq!(
        disposition.category(),
        qsl_foundation::diagnostic::Category::Success
    );
    assert_eq!(disposition.truth(), Some(true));
    assert_eq!(disposition.category().exit_code(), 0);

    // Completed(false): violation, exit 10.
    let mapped = super::super::call::convert_outcome(
        evaluation(FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(
            quire_exact::Value::Boolean(false),
        ))),
        graph,
        sources,
    )
    .expect("Completed(false) maps to Ok");
    let disposition = ClauseDisposition::Evaluate(mapped);
    assert_eq!(
        disposition.category(),
        qsl_foundation::diagnostic::Category::Violation
    );
    assert_eq!(disposition.truth(), Some(false));
    assert_eq!(disposition.category().exit_code(), 10);

    // A kernel `Undefined`: undefined, exit 10 (FR-285).
    let mapped = super::super::call::convert_outcome(
        evaluation(FamilyOutcome::Evaluated(quire_exact::Outcome::Undefined(
            quire_exact::Undefined::DivisionByZero,
        ))),
        graph,
        sources,
    )
    .expect("Undefined maps to Ok");
    let disposition = ClauseDisposition::Evaluate(mapped);
    assert_eq!(
        disposition.category(),
        qsl_foundation::diagnostic::Category::Undefined
    );
    assert_eq!(disposition.truth(), None);
    assert_eq!(disposition.category().exit_code(), 10);

    // A kernel `Incomplete`: incomplete, exit 22.
    let mapped = super::super::call::convert_outcome(
        evaluation(FamilyOutcome::Evaluated(quire_exact::Outcome::Incomplete(
            quire_exact::Incomplete {
                limit_kind: quire_exact::LimitKind::WorkUnits,
                limit: 1,
                consumed: 1,
                next_charge: 1_i64.into(),
                charge_point: quire_exact::ChargePoint::DecimalOperands,
            },
        ))),
        graph,
        sources,
    )
    .expect("Incomplete maps to Ok");
    let disposition = ClauseDisposition::Evaluate(mapped);
    assert_eq!(
        disposition.category(),
        qsl_foundation::diagnostic::Category::Incomplete
    );
    assert_eq!(disposition.truth(), None);
    assert_eq!(disposition.category().exit_code(), 22);
}

/// FR-285: an admission refusal exits by its record code's category, so
/// `unknown_required_feature` (FR-106 check 6.2) exits 21; an admission
/// `Incomplete` exits 22 whatever its code.
#[test]
fn admission_failure_exits_by_its_record_code_category() {
    use qsl_foundation::diagnostic::Code;
    use qsl_semantics::model::observation::{AdmissionFailure, AdmissionRecord};
    let record = |code: Code| AdmissionRecord {
        code,
        cause: "any-cause",
        fields: BTreeMap::new(),
    };
    let refused = |code| ClauseDisposition::Admit(AdmissionFailure::Refused(record(code)));
    assert_eq!(
        refused(Code::UnknownRequiredFeature).category().exit_code(),
        21
    );
    assert_eq!(
        refused(Code::InvalidRuntimeInput).category().exit_code(),
        20
    );
    for code in Code::all() {
        assert_eq!(
            refused(*code).category().exit_code(),
            code.category().exit_code(),
            "{code}"
        );
        let incomplete = ClauseDisposition::Admit(AdmissionFailure::Incomplete(record(*code)));
        assert_eq!(incomplete.category().exit_code(), 22, "{code}");
    }
}

/// TC-466 step 1 (FR-107-AC-1): `ParentOrder` over admitted current
/// snapshots -- `Completed(true)` for healthy-parent and absent-parent,
/// `Completed(false)` for violating-parent.
#[trace("TC-466", "FR-107-AC-1")]
#[test]
fn tc466_step1_parent_order_over_admitted_snapshots() {
    assert!(
        boolean_disposition(&run_config_version_current(
            "ParentOrder",
            1,
            Some(2),
            "child"
        )),
        "healthy-parent: root 1, child 2 -- child.versionNumber > root's"
    );
    assert!(
        boolean_disposition(&run_config_version_current("ParentOrder", 1, None, "root")),
        "absent-parent: root alone, present(self.parent) is false"
    );
    assert!(
        !boolean_disposition(&run_config_version_current(
            "ParentOrder",
            5,
            Some(2),
            "child"
        )),
        "violating-parent: root 5, child 2 -- child.versionNumber is not > root's"
    );
}

// ---------------------------------------------------------------------------
// TC-466 step 1's `VersionUnchanged` sub-case and step 2's three
// postcondition terms (FR-107-AC-1, AC-2): both need a real
// `attemptUpdate` invocation (pre and post snapshots plus an invocation
// document), unlike step 1's `ParentOrder`/`NoCycle` invariants above,
// which run over one `current` snapshot.
// ---------------------------------------------------------------------------

/// A pre/post snapshot for an `attemptUpdate` invocation: the same
/// `root`/`child` shape [`config_version_snapshot`] builds for `current`,
/// tagged `observation` (`"pre"`/`"post"`) instead of always `"current"`,
/// and with no `anchor` member -- FR-106's own Snapshot member set makes
/// `anchor` optional, and only a `current` snapshot carries one.
fn config_version_invocation_snapshot(
    label: &DocumentRef,
    model_digest_hex: &str,
    observation: &str,
    root_version: i64,
    child_version: i64,
) -> Vec<u8> {
    let population = config_version_population_identity();
    let objects = config_version_objects(root_version, Some(child_version));
    let value = json!({
        "format": "quire.state.snapshot/v1",
        "identity": document_identity_json(label),
        "observation": observation,
        "model": {
            "identity": CONFIG_VERSION_PACKAGE_IDENTITY, "version": "1.0.0",
            "digest": format!("sha256-jcs:{model_digest_hex}"),
        },
        "populations": [{"population": population, "complete": true, "objects": objects}],
    });
    value.to_string().into_bytes()
}

/// An invocation document naming `attemptUpdate` on `ConfigVersion`, self
/// `self_key`, `pre_ref`/`post_ref` as its pre/post snapshots, no
/// parameters, boolean `result`, and an empty `created`/`deleted` -- FR-105's
/// own frame modifies only `versionNumber`, never creates or deletes.
fn config_version_invocation_bytes(
    label: &DocumentRef,
    model_digest_hex: &str,
    pre_ref: &DocumentRef,
    post_ref: &DocumentRef,
    self_key: &str,
) -> Vec<u8> {
    let population = config_version_population_identity();
    let value = json!({
        "format": "quire.state.invocation/v1",
        "identity": document_identity_json(label),
        "model": {
            "identity": CONFIG_VERSION_PACKAGE_IDENTITY, "version": "1.0.0",
            "digest": format!("sha256-jcs:{model_digest_hex}"),
        },
        "context": config_version_type(),
        "operation": "attemptUpdate",
        "self": {"population": population, "key": self_key},
        "pre": {
            "identity": document_identity_json(pre_ref),
            "digest": format!("sha256-jcs:{}", hex(&pre_ref.digest)),
        },
        "post": {
            "identity": document_identity_json(post_ref),
            "digest": format!("sha256-jcs:{}", hex(&post_ref.digest)),
        },
        "parameters": {},
        "result": {"boolean": true},
        "created": [],
        "deleted": [],
    });
    value.to_string().into_bytes()
}

/// A `ClauseSelection` naming `clause_name` (a precondition/postcondition)
/// over the invocation at `label`.
fn config_version_invocation_selection(clause_name: &str, label: DocumentRef) -> ClauseSelection {
    ClauseSelection {
        name: clause_name.to_owned(),
        input: ClauseSelectionInput::Invocation { invocation: label },
    }
}

/// `root` and `child`'s versions across an `attemptUpdate` invocation's pre
/// and post snapshots. `root` need not be the same in both: a clause that
/// derefs `self.parent` (naming `root`) must read the observation its `pre`/
/// (no-`pre`) wrapper selects, so a fixture that gives `root` the same
/// version in pre and post cannot tell a correct pre-scoped read from a
/// wrong post-scoped one.
#[derive(Clone, Copy)]
struct InvocationVersions {
    root_pre: i64,
    root_post: i64,
    child_pre: i64,
    child_post: i64,
}

impl InvocationVersions {
    /// `root`'s version does not change across the invocation -- the
    /// ordinary case for a clause that only reads `child`.
    fn root_unchanged(root_version: i64, child_pre: i64, child_post: i64) -> Self {
        Self {
            root_pre: root_version,
            root_post: root_version,
            child_pre,
            child_post,
        }
    }
}

/// Builds `name`'s `DocumentRef`, calls `build` with it to produce the
/// document's bytes (a document's own identity fields, but never its digest,
/// are named inside its bytes), then relabels with the real digest computed
/// over those bytes.
fn document_ref_and_bytes(
    name: &str,
    build: impl FnOnce(&DocumentRef) -> Vec<u8>,
) -> (DocumentRef, Vec<u8>) {
    let label = config_version_document_ref(name);
    let bytes = build(&label);
    let digest = document_digest(&bytes);
    (DocumentRef { digest, ..label }, bytes)
}

/// Runs `clause_name` (a precondition/postcondition of `attemptUpdate`),
/// built by `request_builder`, over one invocation: `root` and `child` at
/// `versions`' pre/post versions, self `self_key`, through `run_clause` end
/// to end (genuine FR-106 admission, then S6a).
fn run_config_version_invocation(
    request_builder: fn(ClauseRunSelection) -> ClauseRunRequest,
    clause_name: &str,
    versions: InvocationVersions,
    self_key: &str,
) -> ClauseDisposition {
    run_config_version_invocation_report(request_builder, clause_name, versions, self_key)
        .disposition
}

/// [`run_config_version_invocation`]'s whole report.
fn run_config_version_invocation_report(
    request_builder: fn(ClauseRunSelection) -> ClauseRunRequest,
    clause_name: &str,
    versions: InvocationVersions,
    self_key: &str,
) -> super::ClauseRunReport {
    let model_digest_hex = config_version_model_digest_hex();

    let (pre_label, pre_bytes) = document_ref_and_bytes("invocation-pre", |label| {
        config_version_invocation_snapshot(
            label,
            &model_digest_hex,
            "pre",
            versions.root_pre,
            versions.child_pre,
        )
    });
    let (post_label, post_bytes) = document_ref_and_bytes("invocation-post", |label| {
        config_version_invocation_snapshot(
            label,
            &model_digest_hex,
            "post",
            versions.root_post,
            versions.child_post,
        )
    });
    let (invocation_label, invocation_bytes) = document_ref_and_bytes("invocation", |label| {
        config_version_invocation_bytes(label, &model_digest_hex, &pre_label, &post_label, self_key)
    });

    let selection = config_version_invocation_selection(clause_name, invocation_label.clone());
    let mut request = request_builder(ClauseRunSelection::Clause(selection));
    request.snapshots.insert(pre_label.digest, pre_bytes);
    request.snapshots.insert(post_label.digest, post_bytes);
    request
        .invocations
        .insert(invocation_label.digest, invocation_bytes);
    run_clause(request).expect("a well-formed request always reports")
}

/// TC-466 step 1 (FR-107-AC-1): `VersionUnchanged` over an `attemptUpdate`
/// invocation -- `Completed(true)` when `self.versionNumber` does not
/// change across pre and post (unchanged-version), `Completed(false)` when
/// it does (changed-version).
#[trace("TC-466", "FR-107-AC-1")]
#[test]
fn tc466_step1_version_unchanged_over_an_invocation() {
    assert!(
        boolean_disposition(&run_config_version_invocation(
            config_version_request,
            "VersionUnchanged",
            InvocationVersions::root_unchanged(1, 2, 2),
            "child",
        )),
        "unchanged-version: child.versionNumber is 2 in both pre and post"
    );
    assert!(
        !boolean_disposition(&run_config_version_invocation(
            config_version_request,
            "VersionUnchanged",
            InvocationVersions::root_unchanged(1, 2, 3),
            "child",
        )),
        "changed-version: child.versionNumber moves from 2 (pre) to 3 (post)"
    );
}

/// `config_version_unit_and_packages`'s domain package, with a distinct
/// unit adding the three FR-107-AC-2 postcondition clauses over
/// `attemptUpdate` -- a separate unit rather than extra clauses on that
/// other one, because `s4_emits_exactly_the_fr_105_state_nodes` (TC-462,
/// below) counts that other unit's own emitted `state_clause` nodes
/// exactly.
fn config_version_step2_unit_and_packages() -> (String, BTreeMap<[u8; 32], Vec<u8>>) {
    let (base_unit, packages) = config_version_unit_and_packages();
    let unit = format!(
        "{base_unit}\
         post PreVersionIsTwo using v on Config::ConfigVersion::attemptUpdate {{ \
         pre(self.versionNumber) = 2 }}\n\
         post PostVersionIsThree using v on Config::ConfigVersion::attemptUpdate {{ \
         self.versionNumber = 3 }}\n\
         post PreParentVersionIsOne using v on Config::ConfigVersion::attemptUpdate {{ \
         pre(present(self.parent) implies deref(value(self.parent)).versionNumber = 1) }}\n"
    );
    (unit, packages)
}

fn config_version_step2_request(selection: ClauseRunSelection) -> ClauseRunRequest {
    let (unit, packages) = config_version_step2_unit_and_packages();
    config_version_request_for(
        unit,
        packages,
        "clause-run-config-version-step2.native",
        selection,
    )
}

/// TC-466 step 2 (FR-107-AC-2): over changed-version, `pre(self.versionNumber)`
/// evaluates to 2.
#[trace("TC-466", "FR-107-AC-2")]
#[test]
fn tc466_step2_pre_version_number_is_two() {
    assert!(
        boolean_disposition(&run_config_version_invocation(
            config_version_step2_request,
            "PreVersionIsTwo",
            InvocationVersions::root_unchanged(1, 2, 3),
            "child",
        )),
        "pre(self.versionNumber) = 2 over the changed-version invocation"
    );
}

/// TC-466 step 2 (FR-107-AC-2): over changed-version, `self.versionNumber`
/// evaluates to 3 in the post state.
#[trace("TC-466", "FR-107-AC-2")]
#[test]
fn tc466_step2_post_version_number_is_three() {
    assert!(
        boolean_disposition(&run_config_version_invocation(
            config_version_step2_request,
            "PostVersionIsThree",
            InvocationVersions::root_unchanged(1, 2, 3),
            "child",
        )),
        "self.versionNumber = 3 over the changed-version invocation"
    );
}

/// TC-466 step 2 (FR-107-AC-2): `pre(present(self.parent) implies
/// deref(value(self.parent)).versionNumber = 1)` is `Completed(true)` when
/// `root` (named by `child.parent`) is version 1 in *pre* -- here 1 in pre,
/// 5 in post, so an evaluator that wrongly derefed through the post
/// observation would compute `5 = 1` and give `false`; FR-107's own
/// contract ("A reference keeps the observation it was read in, so a deref
/// of it reads that observation") is what the correct pre-scoped read
/// depends on. [`tc466_step2_pre_parent_implies_version_one_false_when_pre_root_is_not_one`]
/// is the paired control: swapping which observation carries version 1
/// flips the clause to `Completed(false)`, so neither case can pass
/// vacuously.
#[trace("TC-466", "FR-107-AC-2")]
#[test]
fn tc466_step2_pre_parent_implies_version_one() {
    assert!(
        boolean_disposition(&run_config_version_invocation(
            config_version_step2_request,
            "PreParentVersionIsOne",
            InvocationVersions {
                root_pre: 1,
                root_post: 5,
                child_pre: 2,
                child_post: 3,
            },
            "child",
        )),
        "pre(present(self.parent) implies deref(value(self.parent)).versionNumber = 1); root is 1 in pre, 5 in post"
    );
}

/// TC-466 step 2 (FR-107-AC-2), the control for the case above: `root` is
/// version 5 in pre and 1 in post, so the pre-scoped deref reads 5 and the
/// clause is `Completed(false)`. Together with the case above, this pins
/// that the evaluator reads the deref through the `pre` observation the
/// clause names, not through post.
#[trace("TC-466", "FR-107-AC-2")]
#[test]
fn tc466_step2_pre_parent_implies_version_one_false_when_pre_root_is_not_one() {
    assert!(
        !boolean_disposition(&run_config_version_invocation(
            config_version_step2_request,
            "PreParentVersionIsOne",
            InvocationVersions {
                root_pre: 5,
                root_post: 1,
                child_pre: 2,
                child_post: 3,
            },
            "child",
        )),
        "pre(present(self.parent) implies deref(value(self.parent)).versionNumber = 1); root is 5 in pre, 1 in post"
    );
}

// ---------------------------------------------------------------------------
// TC-466 step 3 (FR-107-AC-3): `ReachesTarget` over a `probe`
// invocation -- a variant of the `config-version` fixture package above
// that adds one operation, `probe(target: ConfigVersion)`, with no result
// and an empty frame (TC-466's own step 3 wording), and one precondition,
// `pre ReachesTarget using v on Config::ConfigVersion::probe { reaches(self,
// target, parent) }`. A distinct unit/package pair from step 1/2's (a
// different domain document, since `probe` is a new operation, not a new
// clause over `attemptUpdate`), so this section builds its own digest,
// request builder and snapshot/invocation shapes rather than reusing
// `config_version_unit_and_packages`/`config_version_request`.
// ---------------------------------------------------------------------------

/// The `probe` operation entry TC-466 step 3 adds to the `config-version`
/// package: one parameter `target: ConfigVersion`, no `returns` (the JSON
/// envelope's `operations[].returns` member is entirely absent for a
/// no-result operation -- `qsl-semantics`' own `model::intake` reads
/// `returns` as `Option`), and an empty frame (`modifies`/`creates`/
/// `deletes` all empty: `probe` reads, it never writes).
fn config_version_probe_operation() -> serde_json::Value {
    let config_version = config_version_type();
    let operation_identity = format!("{config_version}/probe");
    let target_identity = format!("{operation_identity}/target");
    json!({
        "identity": operation_identity,
        "name": "probe",
        "params": [
            {
                "identity": target_identity,
                "name": "target",
                "typeRef": config_version,
                "presence": "required",
                "nullable": false,
                "defaultKind": "none",
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                "origin": {
                    "generated": {
                        "generatorIdentity": target_identity,
                        "generatorVersion": "1.0.0",
                        "inputIdentities": [target_identity],
                    }
                },
            },
        ],
        "pre": [],
        "post": [],
        "origin": {
            "source": {
                "sourceIdentity": format!("ix://{CONFIG_VERSION_PACKAGE_IDENTITY}/spec"),
                "path": "spec.qspec",
                "startLine": 1,
                "startColumn": 1,
            },
        },
        "frame": {"modifies": [], "creates": [], "deletes": []},
    })
}

/// [`config_version_domain_document`] plus [`config_version_probe_operation`].
fn config_version_step3_domain_document() -> Vec<u8> {
    config_version_domain_document_with_operations(vec![config_version_probe_operation()])
}

/// The package provision holding [`config_version_step3_domain_document`]
/// and nothing else.
fn config_version_step3_packages() -> BTreeMap<[u8; 32], Vec<u8>> {
    qsl_semantics::model::intake::package_input([config_version_step3_domain_document().as_slice()])
}

/// The hex model digest of [`config_version_step3_packages`]'s one document.
fn config_version_step3_model_digest_hex() -> String {
    let packages = config_version_step3_packages();
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    hex(digest)
}

/// The unit text selecting [`config_version_step3_domain_document`] as
/// model alias `Config`, with one precondition: `ReachesTarget`.
fn config_version_step3_unit_and_packages() -> (String, BTreeMap<[u8; 32], Vec<u8>>) {
    let packages = config_version_step3_packages();
    let digest = config_version_step3_model_digest_hex();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model Config = {CONFIG_VERSION_PACKAGE_IDENTITY:?} version \"1.0.0\" \
         digest \"sha256-jcs:{digest}\";\n\
         pre ReachesTarget using v on Config::ConfigVersion::probe {{ \
         reaches(self, target, parent) }}\n"
    );
    (unit, packages)
}

fn config_version_step3_request(selection: ClauseRunSelection) -> ClauseRunRequest {
    let (unit, packages) = config_version_step3_unit_and_packages();
    config_version_request_for(
        unit,
        packages,
        "clause-run-config-version-step3.native",
        selection,
    )
}

/// A `ConfigVersion` snapshot for a `probe` invocation's `pre`/`post`
/// observation (no `anchor`, matching [`config_version_invocation_snapshot`]'s
/// own shape) over an arbitrary object graph, rather than the fixed
/// `root`/`child` pair invocation snapshots elsewhere in this file use --
/// step 3's `chain`/`loop` graphs need more than two objects and a
/// self-loop, which `config_version_objects` cannot express.
fn config_version_step3_snapshot(
    label: &DocumentRef,
    model_digest_hex: &str,
    observation: &str,
    objects: &[serde_json::Value],
) -> Vec<u8> {
    let population = config_version_population_identity();
    let value = json!({
        "format": "quire.state.snapshot/v1",
        "identity": document_identity_json(label),
        "observation": observation,
        "model": {
            "identity": CONFIG_VERSION_PACKAGE_IDENTITY, "version": "1.0.0",
            "digest": format!("sha256-jcs:{model_digest_hex}"),
        },
        "populations": [
            {"population": population, "complete": true, "objects": objects},
        ],
    });
    value.to_string().into_bytes()
}

/// One `ConfigVersion` object at `key`, `versionNumber` `1` (the shape
/// TC-466 step 3 needs cares only about `parent`, never `versionNumber`),
/// `parent` naming `parent_key` when `Some`, absent when `None`.
fn config_version_step3_object(key: &str, parent_key: Option<&str>) -> serde_json::Value {
    let population = config_version_population_identity();
    let config_version = config_version_type();
    let parent = match parent_key {
        Some(parent_key) => {
            json!({"present": {"reference": {"population": population, "key": parent_key}}})
        }
        None => json!({"absent": {}}),
    };
    json!({
        "key": key,
        "type": config_version,
        "fields": {"versionNumber": {"integer": "1"}, "parent": parent},
    })
}

/// The acyclic chain `a -> b -> c` (`a.parent = b`, `b.parent = c`,
/// `c.parent` absent) TC-466 step 3 (a)-(c) evaluates `ReachesTarget` over.
fn config_version_step3_chain_objects() -> Vec<serde_json::Value> {
    vec![
        config_version_step3_object("a", Some("b")),
        config_version_step3_object("b", Some("c")),
        config_version_step3_object("c", None),
    ]
}

/// The self-loop `a -> a` (`a.parent = a`) TC-466 step 3 (d) evaluates
/// `ReachesTarget` over.
fn config_version_step3_loop_objects() -> Vec<serde_json::Value> {
    vec![config_version_step3_object("a", Some("a"))]
}

/// A `probe` invocation naming `self_key`/`target_key`, `pre`/`post` both
/// `pre_ref`/`post_ref` (TC-466 step 3: "each invocation uses the same
/// snapshot as pre and post"), no result (`probe` has none): `parameters`
/// carries `target` as a bare `{"reference": {...}}`, not the `present`/
/// `absent`-wrapped shape an optional *field* value uses, because an
/// operation parameter's `(name, ValueType)` signature carries no separate
/// presence of its own (`qsl-semantics`' own `document::field_kind_matches`
/// doc comment).
fn config_version_probe_invocation_bytes(
    label: &DocumentRef,
    model_digest_hex: &str,
    pre_ref: &DocumentRef,
    post_ref: &DocumentRef,
    self_key: &str,
    target_key: &str,
) -> Vec<u8> {
    let population = config_version_population_identity();
    let value = json!({
        "format": "quire.state.invocation/v1",
        "identity": document_identity_json(label),
        "model": {
            "identity": CONFIG_VERSION_PACKAGE_IDENTITY, "version": "1.0.0",
            "digest": format!("sha256-jcs:{model_digest_hex}"),
        },
        "context": config_version_type(),
        "operation": "probe",
        "self": {"population": population, "key": self_key},
        "pre": {
            "identity": document_identity_json(pre_ref),
            "digest": format!("sha256-jcs:{}", hex(&pre_ref.digest)),
        },
        "post": {
            "identity": document_identity_json(post_ref),
            "digest": format!("sha256-jcs:{}", hex(&post_ref.digest)),
        },
        "parameters": {"target": {"reference": {"population": population, "key": target_key}}},
        "result": serde_json::Value::Null,
        "created": [],
        "deleted": [],
    });
    value.to_string().into_bytes()
}

/// One `probe` invocation's provisioned documents: the invocation's own
/// label (what a `ReachesTarget` selection names), and the snapshot and
/// invocation provisions keyed by digest, ready for a `ClauseRunRequest`
/// or a hand-built `Provisions`.
struct ProbeDocuments {
    invocation: DocumentRef,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    invocations: BTreeMap<[u8; 32], Vec<u8>>,
}

/// Builds a `probe` invocation over `objects` (chain or loop), naming
/// `self_key`/`target_key` as the probe's `self` and `target`, with the same
/// graph snapshot as both `pre` and `post` (TC-466 step 3).
fn probe_documents(
    objects: &[serde_json::Value],
    self_key: &str,
    target_key: &str,
) -> ProbeDocuments {
    let model_digest_hex = config_version_step3_model_digest_hex();

    let (pre_label, pre_bytes) = document_ref_and_bytes("probe-pre", |label| {
        config_version_step3_snapshot(label, &model_digest_hex, "pre", objects)
    });
    let (post_label, post_bytes) = document_ref_and_bytes("probe-post", |label| {
        config_version_step3_snapshot(label, &model_digest_hex, "post", objects)
    });
    let (invocation_label, invocation_bytes) =
        document_ref_and_bytes("probe-invocation", |label| {
            config_version_probe_invocation_bytes(
                label,
                &model_digest_hex,
                &pre_label,
                &post_label,
                self_key,
                target_key,
            )
        });

    ProbeDocuments {
        snapshots: BTreeMap::from([
            (pre_label.digest, pre_bytes),
            (post_label.digest, post_bytes),
        ]),
        invocations: BTreeMap::from([(invocation_label.digest, invocation_bytes)]),
        invocation: invocation_label,
    }
}

/// Runs `ReachesTarget` over a `probe` invocation through `run_clause` end
/// to end (genuine FR-106 admission, then S6a), over
/// [`probe_documents`]`(objects, self_key, target_key)`.
fn run_config_version_probe(
    objects: &[serde_json::Value],
    self_key: &str,
    target_key: &str,
) -> ClauseDisposition {
    let documents = probe_documents(objects, self_key, target_key);
    let selection = config_version_invocation_selection("ReachesTarget", documents.invocation);
    let mut request = config_version_step3_request(ClauseRunSelection::Clause(selection));
    request.snapshots.extend(documents.snapshots);
    request.invocations.extend(documents.invocations);
    run_clause(request)
        .expect("a well-formed request always reports")
        .disposition
}

/// TC-466 step 3(a) (FR-107-AC-3): `self` `a`, `target` `c`, over the chain
/// `a -> b -> c` -- `a` reaches `c` through `parent`.
#[trace("TC-466", "FR-107-AC-3")]
#[test]
fn tc466_step3_reaches_target_true_self_a_target_c_over_the_chain() {
    assert!(
        boolean_disposition(&run_config_version_probe(
            &config_version_step3_chain_objects(),
            "a",
            "c",
        )),
        "a -> b -> c: a reaches c through parent"
    );
}

/// TC-466 step 3(a)'s own charge-log assertion: `Completed(true)`, and the
/// `reaches` walk's own charges (filtered out of the full log, which also
/// carries the enclosing precondition's own accounting) are exactly
/// `[graph.expand, graph.edge, graph.expand, graph.edge,
/// graph.result-retain]` -- one expansion to enqueue `a` itself, then one
/// expand-and-edge pair per BFS step (`a` to `b`, `b` to `c`), and `c` (the
/// target) is found on that second edge, so the walk never expands `c`
/// itself before the closing retain. Uses [`evaluate_step3_case_a_with_meter`]
/// (below, TC-466 step 3(e)'s own seam) with a fresh, undenied meter, so
/// this is the one test in the file that names step 3(a)'s exact charge
/// log over a real `probe` invocation, distinct from the `NoCycle`
/// charge-log test above (a different clause, over a "never reaches"
/// exhaustive walk that ends in a third expansion, not a result found
/// early).
#[trace("TC-466", "FR-107-AC-3")]
#[test]
fn tc466_step3_reaches_target_charge_log_over_the_chain() {
    let mut meter = Meter::new(default_accounting(1_000_000));
    let evaluation = evaluate_step3_case_a_with_meter(&mut meter);
    assert!(
        boolean_outcome(evaluation),
        "a -> b -> c: a reaches c through parent"
    );
    assert_eq!(step3_graph_charges(&meter), STEP3_CASE_A_GRAPH_CHARGES);
}

/// TC-466 step 3(b): `self` `a`, `target` `a`, over the chain -- `a` never
/// reaches itself (the chain is acyclic).
#[trace("TC-466", "FR-107-AC-3")]
#[test]
fn tc466_step3_reaches_target_false_self_a_target_a_over_the_chain() {
    assert!(
        !boolean_disposition(&run_config_version_probe(
            &config_version_step3_chain_objects(),
            "a",
            "a",
        )),
        "a -> b -> c: a does not reach itself"
    );
}

/// TC-466 step 3(c): `self` `c`, `target` `a`, over the chain -- `c` has no
/// `parent`, so it reaches nothing.
#[trace("TC-466", "FR-107-AC-3")]
#[test]
fn tc466_step3_reaches_target_false_self_c_target_a_over_the_chain() {
    assert!(
        !boolean_disposition(&run_config_version_probe(
            &config_version_step3_chain_objects(),
            "c",
            "a",
        )),
        "a -> b -> c: c has no parent, so c never reaches a"
    );
}

/// TC-466 step 3(d): `self` `a`, `target` `a`, over the self-loop `a -> a`
/// -- `a` reaches itself through its own `parent`.
#[trace("TC-466", "FR-107-AC-3")]
#[test]
fn tc466_step3_reaches_target_true_self_a_target_a_over_the_loop() {
    assert!(
        boolean_disposition(&run_config_version_probe(
            &config_version_step3_loop_objects(),
            "a",
            "a",
        )),
        "a -> a: a reaches itself through its own parent"
    );
}

// ---------------------------------------------------------------------------
// TC-466 step 3(e) (FR-107-AC-3): a charge-denial `Meter` over
// step 3(a)'s own chain/self/target shape, denying the `reaches` walk's
// 1st through 5th charge in turn. `ClauseRunRequest` builds its own
// `Meter::new(request.accounting)` internally (`qsl-replay/src/spine/
// clause.rs`'s own `run_clause`) with no seam for a caller-supplied one, so
// this section calls the same two functions `run_clause` calls --
// `super::admit_clause_observations` (FR-106, `run_clause`'s own admission
// step) and `CheckedPackage::evaluate_clause` (S6a) -- directly, as the
// existing `no_cycle_completes_true_with_the_expected_reaches_charge_log`
// charge-log test above (this file's established seam for a hand-supplied
// `Meter`) does for `NoCycle`, but over a real FR-106-admitted `probe`
// invocation instead of a hand-built `AdmittedObservations`.
//
// Step 3(a)'s own chain (`self` `a`, `target` `c`) charges the `reaches`
// walk exactly `[graph.expand, graph.edge, graph.expand, graph.edge,
// graph.result-retain]` (TC-466's own Expected Results for step 3(a)):
// expand `a`, edge `a` to `b`, expand `b`, edge `b` to `c` -- which finds
// `target`, so the walk never expands `c` itself -- then the closing
// result retain. Its five charges are, in order: `graph.expand` occurrence
// 1, `graph.edge` occurrence 1, `graph.expand` occurrence 2, `graph.edge`
// occurrence 2, `graph.result-retain` occurrence 1 -- exactly what
// `quire_exact::InjectedDenial { point, occurrence: NonZeroU64::new(occurrence).expect("nonzero occurrence") }` denies.
// ---------------------------------------------------------------------------

/// Evaluates TC-466 step 3(a)'s `ReachesTarget` (`self` `a`, `target` `c`,
/// over the chain) through real FR-106 admission, charging `meter` instead
/// of a fresh one -- the seam sub-case (e) needs.
fn evaluate_step3_case_a_with_meter(meter: &mut Meter) -> qsl_eval::value::Evaluation {
    let documents = probe_documents(&config_version_step3_chain_objects(), "a", "c");
    let (unit, packages) = config_version_step3_unit_and_packages();
    let compiled = compose(
        source(),
        "clause-run-config-version-step3-denial.native",
        unit.as_bytes(),
        &packages,
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .unwrap_or_else(|refusal| panic!("compile refused: {refusal:?}"));
    let package = &compiled.package;
    let clause = package
        .graph()
        .state_clause("ReachesTarget")
        .expect("ReachesTarget is declared");

    let provisions = Provisions {
        snapshots: &documents.snapshots,
        invocations: &documents.invocations,
    };
    let selection = config_version_invocation_selection("ReachesTarget", documents.invocation);
    let observations = admit_clause_observations(
        package.graph(),
        clause,
        &packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        &provisions,
        &selection,
        ObservationLimits::default(),
    )
    .expect("step 3(a)'s own invocation is well-formed");

    let name = QualifiedName::unqualified("ReachesTarget").unwrap();
    package
        .evaluate_clause(&name, &observations, meter)
        .expect("ReachesTarget evaluates")
        .evaluation
}

/// FR-107: a clause reads its own observation by its kind -- `current`
/// for an invariant, `post` for a postcondition -- and never another one in
/// its place. `VersionUnchanged` over observations that lost their post
/// snapshot faults naming the missing clause observation rather than
/// evaluating over `pre`.
#[trace("TC-466")]
#[test]
fn a_postcondition_with_only_a_pre_observation_faults() {
    let compiled = config_version_compiled();
    let (_, packages) = config_version_unit_and_packages();
    let package = &compiled.package;
    let model_digest_hex = config_version_model_digest_hex();
    let (pre, pre_bytes) = document_ref_and_bytes("only-pre", |label| {
        config_version_invocation_snapshot(label, &model_digest_hex, "pre", 1, 2)
    });
    let (post, post_bytes) = document_ref_and_bytes("only-post", |label| {
        config_version_invocation_snapshot(label, &model_digest_hex, "post", 1, 3)
    });
    let (invocation, invocation_bytes) = document_ref_and_bytes("only-invocation", |label| {
        config_version_invocation_bytes(label, &model_digest_hex, &pre, &post, "child")
    });
    let snapshots = BTreeMap::from([(pre.digest, pre_bytes), (post.digest, post_bytes)]);
    let invocations = BTreeMap::from([(invocation.digest, invocation_bytes)]);
    let clause = package
        .graph()
        .state_clause("VersionUnchanged")
        .expect("VersionUnchanged is declared");
    let mut observations = admit_clause_observations(
        package.graph(),
        clause,
        &packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        &Provisions {
            snapshots: &snapshots,
            invocations: &invocations,
        },
        &config_version_invocation_selection("VersionUnchanged", invocation),
        ObservationLimits::default(),
    )
    .expect("changed-version admits");
    observations.post = None;
    let name = QualifiedName::unqualified("VersionUnchanged").unwrap();
    let mut meter = Meter::new(default_accounting(1_000_000));
    match package.evaluate_clause(&name, &observations, &mut meter) {
        Err(CallFailure::Fault(fault)) => {
            assert_eq!(fault.stage(), "S6a");
            assert_eq!(
                fault.invariant(),
                "clause-observations-missing-the-clause-observation"
            );
        }
        other => panic!("expected an S6a fault, got {other:?}"),
    }
}

/// Step 3(a)'s exact `reaches` charge log (TC-466's own Expected Results).
const STEP3_CASE_A_GRAPH_CHARGES: [ChargePoint; 5] = [
    ChargePoint::GraphExpand,
    ChargePoint::GraphEdge,
    ChargePoint::GraphExpand,
    ChargePoint::GraphEdge,
    ChargePoint::GraphResultRetain,
];

/// The `reaches` walk's own charges out of `meter`'s full admitted log,
/// which also carries the enclosing precondition's own accounting.
fn step3_graph_charges(meter: &Meter) -> Vec<ChargePoint> {
    meter
        .admitted_charges()
        .iter()
        .copied()
        .filter(|point| {
            matches!(
                point,
                ChargePoint::GraphExpand | ChargePoint::GraphEdge | ChargePoint::GraphResultRetain
            )
        })
        .collect()
}

/// Asserts that denying `point`'s `occurrence`th admission -- the walk's
/// `nth` charge (1-based) -- makes step 3(a) `Incomplete` at exactly that
/// charge: `charge_point` is `point`, the exhausted counter is
/// `work_units`, `nth - 1` work units were consumed before it, and the
/// admitted walk charges are exactly the first `nth - 1` entries of
/// [`STEP3_CASE_A_GRAPH_CHARGES`]. One of TC-466 step 3(e)'s five sub-cases;
/// each asserts a different prefix and point, so no two pass for the same
/// reason.
fn assert_step3_denial_is_incomplete(point: ChargePoint, occurrence: u64, nth: usize) {
    assert_eq!(
        STEP3_CASE_A_GRAPH_CHARGES[nth - 1],
        point,
        "the walk's charge {nth} is {:?}, not {point:?}",
        STEP3_CASE_A_GRAPH_CHARGES[nth - 1]
    );
    let mut meter =
        Meter::new(default_accounting(1_000_000)).with_injected_denial(InjectedDenial {
            point,
            occurrence: NonZeroU64::new(occurrence).expect("nonzero occurrence"),
        });
    let evaluation = evaluate_step3_case_a_with_meter(&mut meter);
    let FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) = evaluation.outcome else {
        panic!(
            "denying {point:?} occurrence {occurrence} (the reaches walk's charge {nth}) \
             should make ReachesTarget Incomplete, got {:?}",
            evaluation.outcome
        );
    };
    assert_eq!(incomplete.charge_point, point);
    assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
    assert_eq!(incomplete.consumed, u64::try_from(nth - 1).unwrap());
    assert_eq!(
        step3_graph_charges(&meter),
        STEP3_CASE_A_GRAPH_CHARGES[..nth - 1],
        "only the walk's first {} charges are admitted before the denial",
        nth - 1
    );
}

/// TC-466 step 3(e), sub-case 1: deny the `reaches` walk's 1st charge
/// (`graph.expand` occurrence 1, the enqueue of `a` itself).
#[trace("TC-466", "FR-107-AC-3")]
#[test]
fn tc466_step3_incomplete_when_the_first_reaches_charge_is_denied() {
    assert_step3_denial_is_incomplete(ChargePoint::GraphExpand, 1, 1);
}

/// TC-466 step 3(e), sub-case 2: deny the `reaches` walk's 2nd charge
/// (`graph.edge` occurrence 1, the edge from `a` to `b`).
#[trace("TC-466", "FR-107-AC-3")]
#[test]
fn tc466_step3_incomplete_when_the_second_reaches_charge_is_denied() {
    assert_step3_denial_is_incomplete(ChargePoint::GraphEdge, 1, 2);
}

/// TC-466 step 3(e), sub-case 3: deny the `reaches` walk's 3rd charge
/// (`graph.expand` occurrence 2, the enqueue of `b`).
#[trace("TC-466", "FR-107-AC-3")]
#[test]
fn tc466_step3_incomplete_when_the_third_reaches_charge_is_denied() {
    assert_step3_denial_is_incomplete(ChargePoint::GraphExpand, 2, 3);
}

/// TC-466 step 3(e), sub-case 4: deny the `reaches` walk's 4th charge
/// (`graph.edge` occurrence 2, the edge from `b` to `c`).
#[trace("TC-466", "FR-107-AC-3")]
#[test]
fn tc466_step3_incomplete_when_the_fourth_reaches_charge_is_denied() {
    assert_step3_denial_is_incomplete(ChargePoint::GraphEdge, 2, 4);
}

/// TC-466 step 3(e), sub-case 5: deny the `reaches` walk's 5th and final
/// charge (`graph.result-retain` occurrence 1). Step 3(a)'s own charge log
/// is exactly `[graph.expand, graph.edge, graph.expand, graph.edge,
/// graph.result-retain]` -- unlike the `NoCycle` "never reaches" charge-log
/// test above, `target` (`c`) is found on the second edge (`b` to `c`), so
/// the walk never expands `c` itself; it goes straight to the closing
/// retain.
#[trace("TC-466", "FR-107-AC-3")]
#[test]
fn tc466_step3_incomplete_when_the_fifth_reaches_charge_is_denied() {
    assert_step3_denial_is_incomplete(ChargePoint::GraphResultRetain, 1, 5);
}

/// TC-466 step 3(e) control: denying `graph.expand` occurrence 3 -- a
/// charge step 3(a)'s walk never makes, because it finds `c` on the second
/// edge -- never fires, so `ReachesTarget` still completes `true` with the
/// full five-charge log. This is what makes the five sub-cases above
/// discriminating: a denial that missed its charge would complete `true`
/// here rather than go `Incomplete`.
#[trace("TC-466", "FR-107-AC-3")]
#[test]
fn tc466_step3_completes_true_when_the_denied_charge_is_never_made() {
    let mut meter =
        Meter::new(default_accounting(1_000_000)).with_injected_denial(InjectedDenial {
            point: ChargePoint::GraphExpand,
            occurrence: NonZeroU64::new(3).expect("nonzero occurrence"),
        });
    let evaluation = evaluate_step3_case_a_with_meter(&mut meter);
    assert!(
        boolean_outcome(evaluation),
        "a -> b -> c: a reaches c, the third expansion is never charged"
    );
    assert_eq!(step3_graph_charges(&meter), STEP3_CASE_A_GRAPH_CHARGES);
}

// ---------------------------------------------------------------------------
// FR-109-AC-6: an I3 extracted source.
// ---------------------------------------------------------------------------

#[cfg(feature = "quire-extraction")]
mod extracted {
    use super::*;
    use qsl_foundation::{Source, SourceIdentity};

    const QUIRE_PACKAGE: &str = "example/clause-run";

    pub(super) fn original_identity() -> SourceIdentity {
        SourceIdentity::new("agent-ix", "ix://example/clause-run/spec", "git", "7")
    }

    pub(super) fn body_identity() -> SourceIdentity {
        SourceIdentity::new("agent-ix", "clause-run-body", "git", "7")
    }

    /// The authored Markdown document whose one fence under the `no_cycle`
    /// heading, tagged `language`, is the `NoCycle` unit, plus an
    /// unselected fence.
    pub(super) fn original_text_in(language: &str) -> String {
        let (unit, _) = unit_and_packages();
        format!(
            "# Clause run document\n\n## Invariants\n\n### no_cycle\n```{language}\n{unit}```\n\n\
             ### unselected\n```ix:native\nopaque body remains unparsed\n```\n"
        )
    }

    /// `qsl_source::extract`'s verified body of the `ix:native` `no_cycle`
    /// fence.
    fn extracted() -> qsl_source::ExtractedSource {
        extracted_in("ix:native")
    }

    /// `qsl_source::extract`'s verified body of the `no_cycle` fence tagged
    /// `language`.
    pub(super) fn extracted_in(language: &str) -> qsl_source::ExtractedSource {
        let text = original_text_in(language);
        let original = Source::read(original_identity(), "rules.md", text.as_bytes(), text.len())
            .expect("the fixture document reads");
        let context = qsl_source::clause_context(QUIRE_PACKAGE, &original)
            .expect("the clause-only Quire context validates");
        qsl_source::extract(
            original,
            &context,
            qsl_source::Selection {
                clause_id: "no_cycle".to_owned(),
                package: QUIRE_PACKAGE.to_owned(),
                body: body_identity(),
            },
            qsl_source::Limits::default(),
        )
        .expect("the no_cycle fence extracts")
    }

    /// TC-468 step 6 (FR-109-AC-6): the healthy-parent request whose unit
    /// is an I3 extracted source compiles the extracted body and reports
    /// `success` as the program source does, with the body's `package_id`.
    #[trace("TC-468", "FR-109-AC-6")]
    #[test]
    fn run_clause_compiles_an_extracted_body() {
        let (mut request, _label) = no_cycle_request(&[("a", Some("b")), ("b", None)]);
        let (unit, packages) = unit_and_packages();
        let extracted = extracted();
        let body = extracted.map().body();
        assert_eq!(
            body.text(),
            unit.trim_end_matches('\n'),
            "the extracted body is the fenced unit"
        );
        // The package the extracted body compiles to on its own: the unit
        // `run_clause` compiled is that body, not the original document.
        let body_package_id = compose(
            body.identity().clone(),
            body.path(),
            body.text().as_bytes(),
            &packages,
            &DependencyInput::default(),
            SpineLimits::default(),
        )
        .expect("the extracted body compiles")
        .emitted
        .package_id();
        request.source = ClauseRunSource::Extracted(extracted);
        let report = run_clause(request).expect("a well-formed request always reports");

        assert_eq!(report.disposition.stage(), ClauseRunStage::Evaluate);
        assert_eq!(report.disposition.truth(), Some(true));
        assert_eq!(report.disposition.category().exit_code(), 0);
        assert_eq!(report.package_id, Some(body_package_id));
    }

    /// TC-468 step 6 (FR-109-AC-6): a violating-parent I3 request reports
    /// `violation`, and a compile refusal over an I3 source (no domain
    /// package supplied) reports stage `compile` with no `package_id`.
    #[trace("TC-468", "FR-109-AC-6")]
    #[test]
    fn run_clause_reports_a_violation_and_a_compile_refusal_over_an_extracted_body() {
        let (mut violating, _) = no_cycle_request(&[("a", Some("b")), ("b", Some("a"))]);
        violating.source = ClauseRunSource::Extracted(extracted());
        let report = run_clause(violating).expect("a well-formed request always reports");
        assert_eq!(report.disposition.truth(), Some(false));
        assert_eq!(report.disposition.category().exit_code(), 10);

        let (mut missing, _) = no_cycle_request(&[("a", None)]);
        missing.source = ClauseRunSource::Extracted(extracted());
        missing.packages = BTreeMap::new();
        let report = run_clause(missing).expect("a well-formed request always reports");
        assert_eq!(report.disposition.stage(), ClauseRunStage::Compile);
        assert_eq!(report.package_id, None);
    }
}

/// TC-468 step 6 (FR-109-AC-6): an I3 extracted source whose fence declares
/// a language other than `ix:native` reports stage `compile`, `refusal`,
/// `unknown_language`, exit 20, with no `package_id` -- the same refusal the root crate's
/// `mapped::compile` gives that fence.
#[cfg(feature = "quire-extraction")]
#[trace("TC-468", "FR-109-AC-6")]
#[test]
fn run_clause_refuses_an_extracted_fence_that_is_not_ix_native() {
    use extracted::extracted_in;
    let (mut request, _) = no_cycle_request(&[("a", None)]);
    let source = extracted_in("ix:formal");
    assert_eq!(source.language(), "ix:formal");
    request.source = ClauseRunSource::Extracted(source);
    let report = run_clause(request).expect("a well-formed request always reports");
    match &report.disposition {
        ClauseDisposition::UnknownLanguage { language } => assert_eq!(language, "ix:formal"),
        other => panic!("expected UnknownLanguage, got {other:?}"),
    }
    assert_eq!(report.disposition.stage(), ClauseRunStage::Compile);
    assert_eq!(
        report.disposition.category(),
        qsl_foundation::diagnostic::Category::Refusal
    );
    assert_eq!(report.disposition.truth(), None);
    assert_eq!(
        report.disposition.category().exit_code(),
        qsl_foundation::diagnostic::Code::UnknownLanguage
            .category()
            .exit_code()
    );
    assert_eq!(report.disposition.category().exit_code(), 20);
    assert_eq!(report.package_id, None);
}

// ---------------------------------------------------------------------------
// FR-105: S4 state-node emission (TC-462, TC-463).
// ---------------------------------------------------------------------------

fn config_version_compiled() -> ComposedUnit {
    let (unit, packages) = config_version_unit_and_packages();
    compose(
        source(),
        "clause-run-config-version.native",
        unit.as_bytes(),
        &packages,
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .unwrap_or_else(|refusal| panic!("compile refused: {refusal:?}"))
}

/// `node`'s body, expecting a `quire.op.state.clause` application, and the
/// `clause` its `state_clause` member names.
fn state_clause_kind(node: &qsl_semantics::check::SemanticNode) -> &'static str {
    let qsl_semantics::check::BodyTerm::Application(qsl_semantics::check::ApplicationTerm {
        operator,
        operation,
        ..
    }) = node.body()
    else {
        panic!(
            "state_clause body must be an application, got {:?}",
            node.body()
        );
    };
    assert_eq!(*operator, qsl_semantics::check::Operator::StateClause);
    assert_eq!(operation.identity(), "quire.op.state.clause");
    match operation.member() {
        Some(qsl_semantics::value::Member::StateClause { clause }) => match clause {
            qsl_forms::StateClauseKind::Invariant => "invariant",
            qsl_forms::StateClauseKind::Precondition => "precondition",
            qsl_forms::StateClauseKind::Postcondition => "postcondition",
        },
        other => panic!("state_clause member must be StateClause, got {other:?}"),
    }
}

/// TC-462 (FR-105-AC-1, FR-105-AC-2): S4 emits exactly the `state` nodes
/// FR-105's Outputs table describes for FR-108's unit: three `state_clause`
/// nodes (`invariant`, `invariant`, `postcondition`), one `operation_anchor`
/// (context `ConfigVersion`, operation `"attemptUpdate"`) and one `frame`
/// (`modifies` exactly `versionNumber`, `creates`/`deletes` empty) -- and no
/// `model`/`field_declaration`, `model`/`operation_declaration`,
/// `state`/`snapshot` or `state`/`transition` node. Also QSpec FR-341-AC-10:
/// every `value`/`parameter` node this compile builds (the clauses' `self`
/// and `result`; this fixture's `attemptUpdate` binds no operation
/// parameters, but they go through the same `parameter()` call) has an
/// occurrence with role `expression`.
#[trace("TC-462", "FR-105-AC-1")]
#[trace("TC-462", "FR-105-AC-2")]
#[trace("TC-462", "QSpec-FR-341-AC-10")]
#[test]
fn s4_emits_exactly_the_fr_105_state_nodes() {
    let compiled = config_version_compiled();
    let graph = compiled.package.graph();
    let state_nodes: Vec<&qsl_semantics::check::SemanticNode> = graph
        .semantic_graph()
        .nodes()
        .filter(|node| node.node_tag() == qsl_semantics::check::NodeTag::State)
        .collect();

    let mut clause_kinds: Vec<&str> = state_nodes
        .iter()
        .filter(|node| node.semantic_form() == "state_clause")
        .map(|node| state_clause_kind(node))
        .collect();
    clause_kinds.sort_unstable();
    assert_eq!(clause_kinds, ["invariant", "invariant", "postcondition"]);

    let anchors: Vec<_> = state_nodes
        .iter()
        .filter(|node| node.semantic_form() == "operation_anchor")
        .collect();
    assert_eq!(anchors.len(), 1, "{anchors:?}");
    let qsl_semantics::check::BodyTerm::Aggregate(qsl_semantics::check::AggregateTerm { members }) =
        anchors[0].body()
    else {
        panic!("operation_anchor body must be an aggregate");
    };
    let [context, operation, frame_ref] = members.as_slice() else {
        panic!("operation_anchor has exactly 3 bindings, got {members:?}");
    };
    let qsl_semantics::check::MemberTerm::Binding(qsl_semantics::check::Binding { name, value }) =
        operation
    else {
        panic!("operation_anchor's second member is a binding");
    };
    assert_eq!(name, "operation");
    let qsl_semantics::check::BindingValue::Leaf(qsl_semantics::check::LeafTerm::Literal {
        value: qsl_semantics::check::LiteralValue::Text(text),
        ..
    }) = value
    else {
        panic!("operation's own value is a text literal");
    };
    assert_eq!(text, "attemptUpdate");
    let qsl_semantics::check::MemberTerm::Binding(qsl_semantics::check::Binding {
        name: context_name,
        value: context_value,
    }) = context
    else {
        panic!("operation_anchor's first member is a binding");
    };
    assert_eq!(context_name, "context");
    let qsl_semantics::check::BindingValue::Leaf(qsl_semantics::check::LeafTerm::Reference {
        target: context_ref,
    }) = context_value
    else {
        panic!("context's value is a reference");
    };

    // FND-005: find ConfigVersion's own `model`/`object_type` node
    // independently of the anchor/frame chain above, through
    // `resolve_declaration` -- the model correspondence FR-088-AC-2 built
    // during checking, not a value this test derived from the emission
    // path it is checking. If `context_ref`/`only.declaration()` above
    // consistently named the wrong node (say, the population node), this
    // lookup would still find the real one and the comparison below would
    // catch the mismatch.
    let expected_declaration = qsl_semantics::model::key::DeclarationKey {
        package: CONFIG_VERSION_PACKAGE_IDENTITY.to_owned(),
        node: config_version_type(),
    };
    let config_version_node = graph
        .semantic_graph()
        .nodes()
        .find(|node| {
            node.node_tag() == qsl_semantics::check::NodeTag::Model
                && node.semantic_form() == "object_type"
                && graph.resolve_declaration(node.key()) == Some(&expected_declaration)
        })
        .expect("ConfigVersion's own object_type node is in the graph")
        .key();
    assert_eq!(
        context_ref.0, config_version_node,
        "operation_anchor's context is ConfigVersion's own node, verified via resolve_declaration"
    );

    // TC-462 step 2: each invariant's own anchor argument (state_clause's
    // second application argument) also names ConfigVersion directly --
    // an invariant has no operation to anchor at, so lowering anchors it
    // at its own context object type (`Lowering::state_clause`, the `None`
    // arm).
    for node in state_nodes
        .iter()
        .filter(|node| node.semantic_form() == "state_clause")
        .filter(|node| state_clause_kind(node) == "invariant")
    {
        let qsl_semantics::check::BodyTerm::Application(qsl_semantics::check::ApplicationTerm {
            arguments,
            ..
        }) = node.body()
        else {
            panic!("state_clause body must be an application");
        };
        let [_, anchor_argument, _] = arguments.as_slice() else {
            panic!("state_clause has exactly 3 arguments, got {arguments:?}");
        };
        let qsl_semantics::check::MemberTerm::Leaf(qsl_semantics::check::LeafTerm::Reference {
            target,
        }) = anchor_argument
        else {
            panic!("state_clause's anchor argument is a reference");
        };
        assert_eq!(
            target.0, config_version_node,
            "an invariant's anchor argument names ConfigVersion's own node"
        );
    }

    let frames: Vec<_> = state_nodes
        .iter()
        .filter(|node| node.semantic_form() == "frame")
        .collect();
    assert_eq!(frames.len(), 1, "{frames:?}");
    let qsl_semantics::check::BodyTerm::Frame(qsl_semantics::check::FrameTerm {
        modifies,
        creates,
        deletes,
    }) = frames[0].body()
    else {
        panic!(
            "frame body must be BodyTerm::Frame, got {:?}",
            frames[0].body()
        );
    };
    assert!(creates.is_empty(), "{creates:?}");
    assert!(deletes.is_empty(), "{deletes:?}");
    let [only] = modifies.as_slice() else {
        panic!("exactly one modifies entry, got {modifies:?}");
    };
    assert_eq!(only.name(), "versionNumber");
    assert_eq!(
        only.declaration().0,
        context_ref.0,
        "modifies names ConfigVersion's own node"
    );
    assert_eq!(
        frames[0].semantic_type(),
        Some(context_ref.0),
        "the frame's own semantic_type is the declaring object type"
    );
    let qsl_semantics::check::MemberTerm::Binding(qsl_semantics::check::Binding {
        name: frame_name,
        value: frame_value,
    }) = frame_ref
    else {
        panic!("operation_anchor's third member is a binding");
    };
    assert_eq!(frame_name, "frame");
    let qsl_semantics::check::BindingValue::Leaf(qsl_semantics::check::LeafTerm::Reference {
        target: frame_target,
    }) = frame_value
    else {
        panic!("frame's value is a reference");
    };
    assert_eq!(
        frame_target.0,
        frames[0].key(),
        "the anchor's frame binding names the frame node"
    );

    // No model/field_declaration, model/operation_declaration,
    // state/snapshot or state/transition node (ADR-012 §15.3/§15.4).
    for node in graph.semantic_graph().nodes() {
        assert_ne!(
            (node.node_tag(), node.semantic_form()),
            (qsl_semantics::check::NodeTag::Model, "field_declaration")
        );
        assert_ne!(
            (node.node_tag(), node.semantic_form()),
            (
                qsl_semantics::check::NodeTag::Model,
                "operation_declaration"
            )
        );
        assert_ne!(
            (node.node_tag(), node.semantic_form()),
            (qsl_semantics::check::NodeTag::State, "snapshot")
        );
        assert_ne!(
            (node.node_tag(), node.semantic_form()),
            (qsl_semantics::check::NodeTag::State, "transition")
        );
    }

    // QSpec FR-341-AC-10: a state clause's own `self`, `result`
    // and operation-parameter nodes are `value`/`parameter` nodes like any
    // other binder, so their own occurrence also carries role `expression`,
    // not `anchor` -- the function-parameter case is covered by
    // `qsl-package`'s `source_text_compiles_through_the_spine_and_reads_back_verified`;
    // this is the state-clause side of the same requirement.
    let parameter_nodes: Vec<_> = graph
        .semantic_graph()
        .nodes()
        .filter(|node| {
            node.node_tag() == qsl_semantics::check::NodeTag::Value
                && node.semantic_form() == "parameter"
        })
        .collect();
    assert!(
        !parameter_nodes.is_empty(),
        "ConfigVersion's clauses bind self, result and operation parameters"
    );
    for node in &parameter_nodes {
        let key = node.key();
        let mut placed = 0;
        for (_, origin, _) in graph.occurrences().filter(|(id, _, _)| *id == key) {
            assert_eq!(
                origin.role().as_str(),
                "expression",
                "FR-341-AC-10: {key:?}'s occurrence has role expression"
            );
            placed += 1;
        }
        assert!(placed > 0, "{key:?} has at least one recorded occurrence");
    }
}

/// TC-463 step 4 (FR-105-AC-6): a fault injected at the ConfigVersion state
/// clause's own `frame` node refuses the whole compile and emits no `state`
/// node and no package bytes -- AC-6's own frame-node claim. `qsl-package`'s
/// generic mid-list fault test
/// (`a_fault_injected_partway_through_node_emission_writes_nothing`) proves
/// the same all-or-nothing mechanism, but only over three plain function
/// nodes with no `state`/`frame` node; this test drives it over the real
/// `frame` node of a compiled state-clause package. It reaches
/// `qsl-package`'s `test-support`-gated seam
/// (`emit_checked_with_fault`/`checked_node_id_of`) across the crate
/// boundary, enabled only by this crate's own `[dev-dependencies]` edge
/// (`qsl-replay/Cargo.toml`; `no_shipped_dependency_enables_test_support`
/// checks no shipped dependency ever enables it).
#[trace("TC-463", "FR-105-AC-6")]
#[test]
fn a_fault_on_the_frame_node_refuses_the_whole_config_version_package() {
    let compiled = config_version_compiled();
    let graph = compiled.package.graph();
    let frame_key = graph
        .semantic_graph()
        .nodes()
        .find(|node| {
            node.node_tag() == qsl_semantics::check::NodeTag::State
                && node.semantic_form() == "frame"
        })
        .expect("exactly one frame node (asserted by s4_emits_exactly_the_fr_105_state_nodes)")
        .key();
    let frame_id = qsl_package::checked_node_id_of(frame_key);
    let fault_reason = "fault injection (test): forced encoding failure at the frame node";

    let refusal = qsl_package::emit_checked_with_fault(&compiled.package, |id| {
        if *id == frame_id {
            Some(qsl_package::EmitRefusal::Encoding {
                reason: fault_reason.to_owned(),
            })
        } else {
            None
        }
    })
    .expect_err("a fault at the frame node refuses the whole package");

    // The fault actually fired at the frame node, not some earlier,
    // unrelated refusal the fixture happened to trip.
    assert!(
        matches!(&refusal, qsl_package::EmitRefusal::Encoding { reason } if reason == fault_reason),
        "{refusal:?}"
    );
    // `EmitRefusal::Encoding` carries no package: every path from here to a
    // built `Emission` (and therefore to an emitted `state` node or wire
    // bytes) runs only after `emit_package_inner`'s `nodes` step has fully
    // collected, which this fault stops before completing
    // (`collect::<Result<Vec<_>, _>>()?` drops the partial `Vec` on the
    // first `Err`). There is no partial `Emission`, no emitted `state` node
    // and no package bytes for a caller to observe -- the all-or-nothing
    // violation AC-6 guards against is unrepresentable by this function's
    // own return type, not merely untested.
}

/// TC-462 (FR-105-AC-2): the `BodyNames` Frame dependency-walk arm
/// (`qsl-package`'s generic emitter, `emit.rs`) actually lists the frame's
/// `modifies` declaration among the emitted node's own `dependencies` --
/// the graph-reading test above only sees the in-process `SemanticGraph`,
/// so replacing that arm's body with `{}` would still pass every other
/// test here while the emitted frame stopped naming ConfigVersion as a
/// dependency, violating FR-340. This decodes `compiled.emitted.bytes()`
/// directly, the one non-ignored assertion on the emitted wire.
#[trace("TC-462", "FR-105-AC-2")]
#[test]
fn s4_emitted_frame_node_lists_configversion_in_dependencies() {
    let compiled = config_version_compiled();
    let graph = compiled.package.graph();
    let expected_declaration = qsl_semantics::model::key::DeclarationKey {
        package: CONFIG_VERSION_PACKAGE_IDENTITY.to_owned(),
        node: config_version_type(),
    };
    let config_version_node = graph
        .semantic_graph()
        .nodes()
        .find(|node| {
            node.node_tag() == qsl_semantics::check::NodeTag::Model
                && node.semantic_form() == "object_type"
                && graph.resolve_declaration(node.key()) == Some(&expected_declaration)
        })
        .expect("ConfigVersion's own object_type node is in the graph")
        .key();
    let expected_id = json!({
        "domain": quire_exact::NODE_KEY_DOMAIN,
        "digest": config_version_node.to_string(),
    });

    let wire: serde_json::Value =
        serde_json::from_slice(compiled.emitted.bytes()).expect("emitted bytes are JSON");
    let nodes = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic_graph.nodes is an array");
    let frame = nodes
        .iter()
        .find(|node| node["body"]["term"] == "frame")
        .expect("exactly one emitted frame node");
    assert_eq!(
        frame["body"]["modifies"],
        json!([{"kind": "field", "declaration": expected_id, "name": "versionNumber"}]),
        "the emitted frame body's modifies entry"
    );
    assert_eq!(frame["body"]["creates"], json!([]));
    assert_eq!(frame["body"]["deletes"], json!([]));
    assert!(
        frame["dependencies"]
            .as_array()
            .expect("dependencies is an array")
            .contains(&expected_id),
        "the frame's own emitted dependencies must list ConfigVersion: {:?}",
        frame["dependencies"]
    );
}

/// TC-463 step 2 (FR-105-AC-4): compiling FR-108's unit twice gives
/// identical bytes and the same `package_id`.
#[trace("TC-463", "FR-105-AC-4")]
#[test]
fn s4_state_package_emission_is_stable_across_compiles() {
    let first = config_version_compiled();
    let second = config_version_compiled();
    assert_eq!(
        first.emitted.bytes(),
        second.emitted.bytes(),
        "compiling the same unit twice gives identical bytes"
    );
    assert_eq!(first.emitted.package_id(), second.emitted.package_id());
}

// ---------------------------------------------------------------------------
// FR-105-AC-2/AC-4 remainder: dependencies and
// occurrences beyond the frame, the state_clause `semantic_type`, the
// condition-term shapes, and the rename/`<=`/duplicate/second-post/
// Sub-anchor-sharing FR-105-AC-4 cases `s4_state_package_emission_is_
// stable_across_compiles` above does not cover.
// ---------------------------------------------------------------------------

/// ConfigVersion's own `model`/`object_type` node key, found the same way
/// `s4_emits_exactly_the_fr_105_state_nodes` does (FND-005): through
/// `resolve_declaration`, the model correspondence FR-088-AC-2 built during
/// checking, independently of whatever anchor/frame/field-read this test
/// is itself checking.
fn config_version_node_key(graph: &qsl_semantics::check::CheckedGraph) -> NodeKey {
    let expected_declaration = qsl_semantics::model::key::DeclarationKey {
        package: CONFIG_VERSION_PACKAGE_IDENTITY.to_owned(),
        node: config_version_type(),
    };
    graph
        .semantic_graph()
        .nodes()
        .find(|node| {
            node.node_tag() == qsl_semantics::check::NodeTag::Model
                && node.semantic_form() == "object_type"
                && graph.resolve_declaration(node.key()) == Some(&expected_declaration)
        })
        .expect("ConfigVersion's own object_type node is in the graph")
        .key()
}

/// The single `Boolean` scalar type node: `Lowering::scalar` dedups
/// identical content to one key, so every `Boolean`-typed expression in the
/// unit -- including a `state_clause`'s own `semantic_type` (FR-105's
/// Outputs table) -- shares this one node.
fn boolean_scalar_type_node_key(graph: &qsl_semantics::check::CheckedGraph) -> NodeKey {
    graph
        .semantic_graph()
        .nodes()
        .find(|node| {
            node.node_tag() == qsl_semantics::check::NodeTag::ScalarType
                && node.semantic_form() == "boolean"
        })
        .expect("the Boolean scalar type node is in the graph")
        .key()
}

/// Every `state`/`state_clause` node of `graph`, each paired with its
/// FR-105 clause kind ([`state_clause_kind`]).
fn state_clause_nodes(
    graph: &qsl_semantics::check::CheckedGraph,
) -> Vec<(&qsl_semantics::check::SemanticNode, &'static str)> {
    graph
        .semantic_graph()
        .nodes()
        .filter(|node| {
            node.node_tag() == qsl_semantics::check::NodeTag::State
                && node.semantic_form() == "state_clause"
        })
        .map(|node| (node, state_clause_kind(node)))
        .collect()
}

/// `node`'s condition term: a `state_clause` body's 3rd application
/// argument (FR-105's Outputs table: `aggregate` of parameters, a
/// `reference` to the anchor, then the condition).
fn state_clause_condition(
    node: &qsl_semantics::check::SemanticNode,
) -> &qsl_semantics::check::MemberTerm {
    let qsl_semantics::check::BodyTerm::Application(qsl_semantics::check::ApplicationTerm {
        arguments,
        ..
    }) = node.body()
    else {
        panic!("state_clause body must be an application");
    };
    let [_, _, condition] = arguments.as_slice() else {
        panic!("state_clause has exactly 3 arguments, got {arguments:?}");
    };
    condition
}

/// The `state_clause` node of the clause declared as `name`, found through
/// `CheckedGraph::state_clause`'s own identity (FR-104's in-process record),
/// never inferred from the body shape this file is checking.
fn named_clause_node<'g>(
    graph: &'g qsl_semantics::check::CheckedGraph,
    name: &str,
) -> &'g qsl_semantics::check::SemanticNode {
    let identity = graph
        .state_clause(name)
        .unwrap_or_else(|| panic!("clause {name} is checked"))
        .identity();
    graph
        .semantic_graph()
        .node(identity)
        .unwrap_or_else(|| panic!("clause {name}'s node {identity} is in the graph"))
}

/// The one `state`/`<form>` node of `graph` (`operation_anchor` or `frame`).
fn single_state_node_key(graph: &qsl_semantics::check::CheckedGraph, form: &str) -> NodeKey {
    let keys: Vec<NodeKey> = graph
        .semantic_graph()
        .nodes()
        .filter(|node| {
            node.node_tag() == qsl_semantics::check::NodeTag::State && node.semantic_form() == form
        })
        .map(qsl_semantics::check::SemanticNode::key)
        .collect();
    let [key] = keys.as_slice() else {
        panic!("exactly one state/{form} node, got {keys:?}");
    };
    *key
}

/// The body of the `expression` node `leaf` references (FR-093's
/// application-node preimage: every nested application is its own node,
/// named from its parent by reference), or `None` for any other leaf.
fn resolve_expression<'g>(
    graph: &'g qsl_semantics::check::CheckedGraph,
    leaf: &qsl_semantics::check::LeafTerm,
) -> Option<&'g qsl_semantics::check::BodyTerm> {
    let qsl_semantics::check::LeafTerm::Reference { target } = leaf else {
        return None;
    };
    let node = graph.semantic_graph().node(target.0)?;
    (node.node_tag() == qsl_semantics::check::NodeTag::Expression).then(|| node.body())
}

/// Depth-first search of the expression nodes `leaves` reference
/// ([`resolve_expression`]), never into model, type or parameter nodes, for
/// the first application `matches` accepts. The expression graph is
/// content-addressed, so it is acyclic and the walk terminates.
fn find_application<'g>(
    graph: &'g qsl_semantics::check::CheckedGraph,
    leaves: Vec<&qsl_semantics::check::LeafTerm>,
    matches: &dyn Fn(&qsl_semantics::check::ApplicationTerm) -> bool,
) -> Option<&'g qsl_semantics::check::ApplicationTerm> {
    leaves.into_iter().find_map(|leaf| {
        let body = resolve_expression(graph, leaf)?;
        if let qsl_semantics::check::BodyTerm::Application(application) = body {
            if matches(application) {
                return Some(application);
            }
        }
        find_application(graph, body.leaves(), matches)
    })
}

/// [`find_application`] for the first application under `term` whose
/// operator is `operator` -- e.g. the `quire.op.state.pre`/`quire.op.model.
/// reaches_field` subterm FR-105-AC-2 requires, nested inside the
/// equality/negation the clause's own condition wraps it in.
fn find_application_by_operator<'g>(
    graph: &'g qsl_semantics::check::CheckedGraph,
    term: &qsl_semantics::check::MemberTerm,
    operator: qsl_semantics::check::Operator,
) -> Option<&'g qsl_semantics::check::ApplicationTerm> {
    find_application(graph, term.leaves(), &|found| found.operator == operator)
}

/// The `Member::Field` of the first field-read application under `term`
/// naming `field_name` -- the member FR-093 gives `deref(self).f`
/// (FR-105's condition-lowering rule).
fn find_field_read_member<'g>(
    graph: &'g qsl_semantics::check::CheckedGraph,
    term: &qsl_semantics::check::MemberTerm,
    field_name: &str,
) -> Option<&'g qsl_semantics::value::Member> {
    let is_field_read = |found: &qsl_semantics::check::ApplicationTerm| {
        found.operator != qsl_semantics::check::Operator::Reaches
            && matches!(
                found.operation.member(),
                Some(qsl_semantics::value::Member::Field { name, .. }) if name.as_str() == field_name
            )
    };
    find_application(graph, term.leaves(), &is_field_read)?
        .operation
        .member()
}

/// `key`'s wire `node_id` object (`quire.checked-semantic-node/v1`).
fn wire_node_id(key: NodeKey) -> serde_json::Value {
    json!({
        "domain": quire_exact::NODE_KEY_DOMAIN,
        "digest": key.to_string(),
    })
}

/// Every occurrence of `key` as a `(role, ordinal)` pair (`claim`, `anchor`,
/// `generated`), sorted, read from `CheckedGraph::occurrences` (ADR-013
/// O-07): the in-process record `check` built. Not filtered by role, so a
/// stray occurrence in an unexpected role fails the comparison.
fn occurrences_of(graph: &qsl_semantics::check::CheckedGraph, key: NodeKey) -> Vec<(String, u64)> {
    let mut occurrences: Vec<(String, u64)> = graph
        .occurrences()
        .filter(|(node, _, _)| *node == key)
        .map(|(_, origin, _)| (origin.role().as_str().to_owned(), origin.ordinal()))
        .collect();
    occurrences.sort_unstable();
    occurrences
}

/// The expected [`occurrences_of`] list for a node whose occurrences are
/// all in `role`, at `ordinals`.
fn occurrences_in(role: &str, ordinals: &[u64]) -> Vec<(String, u64)> {
    ordinals
        .iter()
        .map(|ordinal| (role.to_owned(), *ordinal))
        .collect()
}

/// `values` sorted by their JSON text, for order-insensitive comparison.
fn sorted_json(mut values: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
    values.sort_by_key(ToString::to_string);
    values
}

/// `key`'s own `dependencies` array, read from `compiled`'s emitted wire
/// bytes (the only place FR-105-AC-2's `dependencies` rule is observable --
/// `BodyNames`/`Candidate::of` in `qsl-package`'s generic emitter compute
/// them at emission time, never as an in-process `CheckedGraph` API).
fn emitted_dependencies_of(compiled: &ComposedUnit, key: NodeKey) -> Vec<serde_json::Value> {
    let wire: serde_json::Value =
        serde_json::from_slice(compiled.emitted.bytes()).expect("emitted bytes are JSON");
    let nodes = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic_graph.nodes is an array");
    let id = wire_node_id(key);
    nodes
        .iter()
        .find(|node| node["node_id"] == id)
        .unwrap_or_else(|| panic!("node {key} is in the emitted wire"))["dependencies"]
        .as_array()
        .expect("dependencies is an array")
        .clone()
}

/// `clause`'s parameter aggregate (its first `quire.op.state.clause`
/// argument), each member resolved to its `value`/`parameter` node's key
/// and bound name, in aggregate order.
fn clause_parameters(
    graph: &qsl_semantics::check::CheckedGraph,
    clause: &qsl_semantics::check::SemanticNode,
) -> Vec<(NodeKey, String)> {
    let qsl_semantics::check::BodyTerm::Application(qsl_semantics::check::ApplicationTerm {
        arguments,
        ..
    }) = clause.body()
    else {
        panic!("state_clause body must be an application");
    };
    let Some(qsl_semantics::check::MemberTerm::Group(parameters)) = arguments.first() else {
        panic!("state_clause's first argument is the parameter aggregate: {arguments:?}");
    };
    parameters
        .members
        .iter()
        .map(|member| {
            let qsl_semantics::check::GroupMember::Leaf(
                qsl_semantics::check::LeafTerm::Reference { target },
            ) = member
            else {
                panic!("each parameter aggregate member is a reference, got {member:?}");
            };
            let node = graph
                .semantic_graph()
                .node(target.0)
                .expect("the parameter node is in the graph");
            assert_eq!(
                (node.node_tag(), node.semantic_form()),
                (qsl_semantics::check::NodeTag::Value, "parameter"),
                "each parameter aggregate member names a value/parameter node"
            );
            let qsl_semantics::check::BodyTerm::Aggregate(qsl_semantics::check::AggregateTerm {
                members: bindings,
            }) = node.body()
            else {
                panic!("a parameter node's body is an aggregate");
            };
            let name = bindings
                .iter()
                .find_map(|binding| match binding {
                    qsl_semantics::check::MemberTerm::Binding(qsl_semantics::check::Binding {
                        name,
                        value:
                            qsl_semantics::check::BindingValue::Leaf(
                                qsl_semantics::check::LeafTerm::Literal {
                                    value: qsl_semantics::check::LiteralValue::Text(text),
                                    ..
                                },
                            ),
                    }) if name == "name" => Some(text.clone()),
                    _ => None,
                })
                .expect("a parameter node binds its name as a text literal");
            (target.0, name)
        })
        .collect()
}

/// `clause`'s condition argument's own reference target: the root
/// `expression` node of its lowered condition.
fn clause_condition_node(clause: &qsl_semantics::check::SemanticNode) -> NodeKey {
    let qsl_semantics::check::MemberTerm::Leaf(qsl_semantics::check::LeafTerm::Reference {
        target,
    }) = state_clause_condition(clause)
    else {
        panic!("a state_clause's condition argument references its expression node");
    };
    target.0
}

/// TC-462 (FR-105-AC-2): each `state`/`state_clause`, `state`/
/// `operation_anchor` and condition field-read node's own emitted
/// `dependencies`, beyond the frame's own (which
/// `s4_emitted_frame_node_lists_configversion_in_dependencies` covers).
/// FR-093's rule is direct: a node depends on what its own body references
/// (plus each application member's declaration), not transitively.
///
/// - `VersionUnchanged` (a postcondition of `attemptUpdate`, which returns
///   `Boolean`): its parameter aggregate is `self` then `result`, and its
///   dependencies are exactly those two parameter nodes, the
///   `operation_anchor` and its condition's expression node.
/// - `ParentOrder`/`NoCycle` (invariants): parameter aggregate `self`
///   alone; dependencies exactly `self`, ConfigVersion (the invariant's
///   anchor) and the condition's expression node.
/// - The `operation_anchor`: exactly ConfigVersion (`context`) and the frame
///   -- its `operation` binding is a text literal, which contributes none.
/// - The `versionNumber` field read under `VersionUnchanged`'s condition
///   depends on ConfigVersion through its `field` member's declaration.
#[trace("TC-462", "FR-105-AC-2")]
#[test]
fn s4_state_clause_and_anchor_dependencies_beyond_the_frame() {
    let compiled = config_version_compiled();
    let graph = compiled.package.graph();
    let config_version_node = config_version_node_key(graph);
    let anchor_key = single_state_node_key(graph, "operation_anchor");
    let frame_key = single_state_node_key(graph, "frame");

    let version_unchanged = named_clause_node(graph, "VersionUnchanged");
    let parameters = clause_parameters(graph, version_unchanged);
    let names: Vec<&str> = parameters.iter().map(|(_, name)| name.as_str()).collect();
    assert_eq!(names, ["self", "result"], "self, then result");
    let mut expected: Vec<serde_json::Value> = parameters
        .iter()
        .map(|(key, _)| wire_node_id(*key))
        .collect();
    expected.push(wire_node_id(anchor_key));
    expected.push(wire_node_id(clause_condition_node(version_unchanged)));
    assert_eq!(
        sorted_json(emitted_dependencies_of(&compiled, version_unchanged.key())),
        sorted_json(expected),
        "VersionUnchanged depends on exactly self, result, the anchor and its condition"
    );

    for name in ["ParentOrder", "NoCycle"] {
        let invariant = named_clause_node(graph, name);
        let parameters = clause_parameters(graph, invariant);
        let [(self_key, self_name)] = parameters.as_slice() else {
            panic!("{name}'s parameter aggregate is self alone, got {parameters:?}");
        };
        assert_eq!(self_name, "self");
        assert_eq!(
            sorted_json(emitted_dependencies_of(&compiled, invariant.key())),
            sorted_json(vec![
                wire_node_id(*self_key),
                wire_node_id(config_version_node),
                wire_node_id(clause_condition_node(invariant)),
            ]),
            "{name} depends on exactly self, ConfigVersion (its anchor) and its condition"
        );
    }

    assert_eq!(
        sorted_json(emitted_dependencies_of(&compiled, anchor_key)),
        sorted_json(vec![
            wire_node_id(config_version_node),
            wire_node_id(frame_key)
        ]),
        "the operation_anchor's own dependencies are exactly its two reference bindings: \
         ConfigVersion (context) and the frame"
    );

    let version_number_member = qsl_semantics::value::Member::Field {
        declaration: config_version_node,
        name: quire_exact::Identifier::new("versionNumber").expect("valid identifier"),
    };
    let field_read = graph
        .semantic_graph()
        .nodes()
        .find(|node| {
            node.node_tag() == qsl_semantics::check::NodeTag::Expression
                && matches!(
                    node.body(),
                    qsl_semantics::check::BodyTerm::Application(qsl_semantics::check::ApplicationTerm { operation, .. })
                        if operation.member() == Some(&version_number_member)
                )
        })
        .expect("a versionNumber field-read expression node");
    let qsl_semantics::check::BodyTerm::Application(qsl_semantics::check::ApplicationTerm {
        arguments,
        ..
    }) = field_read.body()
    else {
        unreachable!("the find above matched an application body");
    };
    let [qsl_semantics::check::MemberTerm::Leaf(qsl_semantics::check::LeafTerm::Reference {
        target: receiver,
    })] = arguments.as_slice()
    else {
        panic!("the field read has one argument, a reference to its receiver, got {arguments:?}");
    };
    assert_eq!(
        graph
            .semantic_graph()
            .node(receiver.0)
            .map(qsl_semantics::check::SemanticNode::node_tag),
        Some(qsl_semantics::check::NodeTag::Expression),
        "the field read's receiver is the `deref(self)` expression node"
    );
    assert_eq!(
        sorted_json(emitted_dependencies_of(&compiled, field_read.key())),
        sorted_json(vec![
            wire_node_id(config_version_node),
            wire_node_id(receiver.0),
        ]),
        "the versionNumber field read depends on exactly ConfigVersion (its member's \
         declaration) and its receiver expression node"
    );
}

/// TC-462 (FR-105-AC-2): `state_clause` occurrences are `claim` kind
/// (ordinal 0, since `ParentOrder`, `NoCycle` and `VersionUnchanged` are
/// each declared once); the `operation_anchor` has one `anchor` occurrence;
/// the `frame` has one `generated` occurrence -- the Outputs table's
/// occurrence column for all three node kinds, read from `CheckedGraph::
/// occurrences` (ADR-013 O-07), the in-process record `check` itself built,
/// not re-derived from the emission path under test.
#[trace("TC-462", "FR-105-AC-2")]
#[test]
fn s4_state_clause_anchor_and_frame_occurrences_match_the_outputs_table() {
    let compiled = config_version_compiled();
    let graph = compiled.package.graph();
    let anchor_key = single_state_node_key(graph, "operation_anchor");
    let frame_key = single_state_node_key(graph, "frame");

    for (node, kind) in state_clause_nodes(graph) {
        assert_eq!(
            occurrences_of(graph, node.key()),
            occurrences_in("claim", &[0]),
            "{kind}'s state_clause node has exactly one occurrence, a claim at ordinal 0, \
             since it is declared exactly once"
        );
    }
    assert_eq!(
        occurrences_of(graph, anchor_key),
        occurrences_in("anchor", &[0]),
        "the operation_anchor has exactly one occurrence, an anchor: one clause \
         (VersionUnchanged) names attemptUpdate"
    );
    assert_eq!(
        occurrences_of(graph, frame_key),
        occurrences_in("generated", &[0]),
        "the frame has exactly one occurrence, generated: one named operation (attemptUpdate)"
    );
}

/// TC-462 (FR-105-AC-2): a `state_clause` node's own `semantic_type` is the
/// `Boolean` scalar type node -- the Outputs table's `state_clause` row,
/// untested before this (only the frame's `semantic_type` was covered).
#[trace("TC-462", "FR-105-AC-2")]
#[test]
fn s4_state_clause_semantic_type_is_the_boolean_scalar_type_node() {
    let compiled = config_version_compiled();
    let graph = compiled.package.graph();
    let boolean = boolean_scalar_type_node_key(graph);

    for (node, kind) in state_clause_nodes(graph) {
        assert_eq!(
            node.semantic_type(),
            Some(boolean),
            "{kind}'s state_clause node's own semantic_type is the Boolean scalar type node"
        );
    }
}

/// TC-462 (FR-105-AC-2): `VersionUnchanged`'s condition
/// (`self.versionNumber = pre(self.versionNumber)`) holds a
/// `quire.op.state.pre` application over the field read of
/// `versionNumber`: `pre`'s single argument is `deref(self).versionNumber`,
/// whose member is `{kind: "field", declaration: <ConfigVersion node>,
/// name: "versionNumber"}` (FR-105's condition-lowering rule, FR-093).
#[trace("TC-462", "FR-105-AC-2")]
#[test]
fn s4_version_unchanged_condition_holds_a_pre_application_over_the_versionnumber_field_read() {
    let compiled = config_version_compiled();
    let graph = compiled.package.graph();
    let config_version_node = config_version_node_key(graph);

    let version_unchanged = named_clause_node(graph, "VersionUnchanged");
    let condition = state_clause_condition(version_unchanged);

    let pre_application =
        find_application_by_operator(graph, condition, qsl_semantics::check::Operator::Pre)
            .unwrap_or_else(|| {
                panic!("condition holds a quire.op.state.pre application: {condition:?}")
            });
    let qsl_semantics::check::ApplicationTerm {
        operation,
        arguments,
        ..
    } = pre_application;
    assert_eq!(operation.identity(), "quire.op.state.pre");
    let [pre_operand] = arguments.as_slice() else {
        panic!("quire.op.state.pre takes exactly one argument, got {arguments:?}");
    };

    let Some(qsl_semantics::check::BodyTerm::Application(qsl_semantics::check::ApplicationTerm {
        operation: field_read_operation,
        ..
    })) = pre_operand
        .leaves()
        .first()
        .and_then(|leaf| resolve_expression(graph, leaf))
    else {
        panic!("pre's own operand is the versionNumber field read, got {pre_operand:?}");
    };
    assert_eq!(
        field_read_operation.member(),
        Some(&qsl_semantics::value::Member::Field {
            declaration: config_version_node,
            name: quire_exact::Identifier::new("versionNumber").expect("valid identifier"),
        }),
        "pre's own operand is deref(self).versionNumber"
    );
}

/// TC-462 (FR-105-AC-2): `NoCycle`'s condition (`not reaches(self, self,
/// parent)`) holds a `quire.op.model.reaches_field` application whose
/// member is `{kind: "field", declaration: <ConfigVersion node>, name:
/// "parent"}` -- FR-105's `reaches(a, b, edge)` lowering rule.
#[trace("TC-462", "FR-105-AC-2")]
#[test]
fn s4_no_cycle_condition_holds_a_reaches_field_application_naming_parent() {
    let compiled = config_version_compiled();
    let graph = compiled.package.graph();
    let config_version_node = config_version_node_key(graph);

    let condition = state_clause_condition(named_clause_node(graph, "NoCycle"));
    let qsl_semantics::check::ApplicationTerm { operation, .. } =
        find_application_by_operator(graph, condition, qsl_semantics::check::Operator::Reaches)
            .unwrap_or_else(|| panic!("NoCycle's condition holds a reaches application"));
    assert_eq!(operation.identity(), "quire.op.model.reaches_field");
    assert_eq!(
        operation.member(),
        Some(&qsl_semantics::value::Member::Field {
            declaration: config_version_node,
            name: quire_exact::Identifier::new("parent").expect("valid identifier"),
        }),
        "reaches(self, self, parent)'s member names ConfigVersion's own parent field"
    );
}

/// TC-462 (FR-105-AC-2): `ParentOrder`'s `self.parent` field read carries
/// the same `Member` shape as `NoCycle`'s `reaches_field` member -- both
/// name ConfigVersion's `parent` field by the same (declaration, name)
/// pair, compared directly with `assert_eq` on the `Member` values
/// themselves (not field-by-field), proving they are identical rather than
/// merely equal-looking.
#[trace("TC-462", "FR-105-AC-2")]
#[test]
fn s4_parent_order_self_parent_read_shares_no_cycles_member_shape() {
    let compiled = config_version_compiled();
    let graph = compiled.package.graph();
    let parent_order = named_clause_node(graph, "ParentOrder");
    let no_cycle = named_clause_node(graph, "NoCycle");

    let parent_order_member =
        find_field_read_member(graph, state_clause_condition(parent_order), "parent")
            .expect("ParentOrder's condition reads self.parent");

    let qsl_semantics::check::ApplicationTerm { operation, .. } = find_application_by_operator(
        graph,
        state_clause_condition(no_cycle),
        qsl_semantics::check::Operator::Reaches,
    )
    .expect("NoCycle holds a reaches application");
    let no_cycle_member = operation.member().expect("reaches_field carries a member");

    assert_eq!(
        parent_order_member, no_cycle_member,
        "ParentOrder's self.parent field read and NoCycle's reaches_field member name the \
         same (declaration, name) pair"
    );
    assert_eq!(
        parent_order_member,
        &qsl_semantics::value::Member::Field {
            declaration: config_version_node_key(graph),
            name: quire_exact::Identifier::new("parent").expect("valid identifier"),
        },
        "and that pair is ConfigVersion's own parent field"
    );
}

// ---------------------------------------------------------------------------
// FR-105-AC-4 remainder: identity under source edits. Each test
// compiles an edited copy of FR-108's unit (a one-place textual edit of
// `config_version_unit_and_packages`' own text, asserted to apply exactly
// once) and compares it against the unedited compile.
// ---------------------------------------------------------------------------

/// Compile `unit` against `packages` through the whole spine (S1 to S5).
fn compile_config_version_unit(unit: &str, packages: &BTreeMap<[u8; 32], Vec<u8>>) -> ComposedUnit {
    compose(
        source(),
        "clause-run-config-version.native",
        unit.as_bytes(),
        packages,
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .unwrap_or_else(|refusal| panic!("compile refused: {refusal:?}\n{unit}"))
}

/// `unit` with its one occurrence of `from` replaced by `to`.
fn edit_once(unit: &str, from: &str, to: &str) -> String {
    assert_eq!(
        unit.matches(from).count(),
        1,
        "the edit target {from:?} occurs exactly once in the unit"
    );
    unit.replacen(from, to, 1)
}

/// `unit` with `line` inserted as its own line directly after the clause
/// line declaring `after` (`<keyword> <after> using ...`).
fn insert_clause_after(unit: &str, after: &str, line: &str) -> String {
    let marker = format!(" {after} using ");
    let mut lines: Vec<&str> = unit.lines().collect();
    let index = lines
        .iter()
        .position(|candidate| candidate.contains(&marker))
        .unwrap_or_else(|| panic!("the unit declares {after}"));
    lines.insert(index + 1, line);
    let mut edited = lines.join("\n");
    edited.push('\n');
    edited
}

/// Every node key of `compiled`'s checked graph.
fn node_keys(compiled: &ComposedUnit) -> BTreeSet<NodeKey> {
    compiled
        .package
        .graph()
        .semantic_graph()
        .nodes()
        .map(qsl_semantics::check::SemanticNode::key)
        .collect()
}

/// `clause`'s anchor argument's reference target.
fn clause_anchor_target(clause: &qsl_semantics::check::SemanticNode) -> NodeKey {
    let qsl_semantics::check::BodyTerm::Application(qsl_semantics::check::ApplicationTerm {
        arguments,
        ..
    }) = clause.body()
    else {
        panic!("state_clause body must be an application");
    };
    let Some(qsl_semantics::check::MemberTerm::Leaf(qsl_semantics::check::LeafTerm::Reference {
        target,
    })) = arguments.get(1)
    else {
        panic!("state_clause's second argument is a reference to its anchor: {arguments:?}");
    };
    target.0
}

/// TC-463 (FR-105-AC-4): renaming `ParentOrder` changes no node id -- the
/// declared name enters no key (FR-088) -- and the renamed clause's
/// identity is `ParentOrder`'s.
#[trace("TC-463", "FR-105-AC-4")]
#[test]
fn s4_renaming_parent_order_changes_no_node_id() {
    let (unit, packages) = config_version_unit_and_packages();
    let base = compile_config_version_unit(&unit, &packages);
    let renamed_unit = edit_once(
        &unit,
        "invariant ParentOrder using",
        "invariant Ordered using",
    );
    let renamed = compile_config_version_unit(&renamed_unit, &packages);

    assert_eq!(
        node_keys(&base),
        node_keys(&renamed),
        "renaming a clause changes no node id"
    );
    assert!(renamed
        .package
        .graph()
        .state_clause("ParentOrder")
        .is_none());
    assert_eq!(
        renamed
            .package
            .graph()
            .state_clause("Ordered")
            .expect("the renamed clause is checked")
            .identity(),
        base.package
            .graph()
            .state_clause("ParentOrder")
            .expect("ParentOrder is checked")
            .identity(),
    );
}

/// TC-463 (FR-105-AC-4): changing `ParentOrder`'s `<` to `<=` changes its
/// `state_clause` node id and the `package_id`, and leaves the other two
/// clauses' node ids unchanged.
#[trace("TC-463", "FR-105-AC-4")]
#[test]
fn s4_changing_parent_orders_comparison_changes_its_node_id_and_the_package_id() {
    let (unit, packages) = config_version_unit_and_packages();
    let base = compile_config_version_unit(&unit, &packages);
    let edited_unit = edit_once(
        &unit,
        ".versionNumber < self.versionNumber",
        ".versionNumber <= self.versionNumber",
    );
    let edited = compile_config_version_unit(&edited_unit, &packages);
    let identity = |compiled: &ComposedUnit, name: &str| {
        compiled
            .package
            .graph()
            .state_clause(name)
            .unwrap_or_else(|| panic!("{name} is checked"))
            .identity()
    };

    assert_ne!(
        identity(&base, "ParentOrder"),
        identity(&edited, "ParentOrder"),
        "ParentOrder's state_clause node id covers its body"
    );
    assert_ne!(
        base.emitted.package_id(),
        edited.emitted.package_id(),
        "the package_id covers the state_clause node"
    );
    for name in ["NoCycle", "VersionUnchanged"] {
        assert_eq!(
            identity(&base, name),
            identity(&edited, name),
            "{name}'s node id is untouched by ParentOrder's edit"
        );
    }
}

/// TC-463 (FR-105-AC-4, FR-104-AC-6): adding `ParentOrder2` with
/// `ParentOrder`'s body adds no node and a second `claim` occurrence
/// (ordinal 1) to `ParentOrder`'s node.
#[trace("TC-463", "FR-105-AC-4")]
#[test]
fn s4_parent_order2_with_parent_orders_body_adds_no_node_and_a_second_claim() {
    let (unit, packages) = config_version_unit_and_packages();
    let base = compile_config_version_unit(&unit, &packages);
    let parent_order_line = unit
        .lines()
        .find(|line| line.contains(" ParentOrder using "))
        .expect("the unit declares ParentOrder");
    let duplicate = parent_order_line.replacen(" ParentOrder using ", " ParentOrder2 using ", 1);
    let edited = compile_config_version_unit(
        &insert_clause_after(&unit, "ParentOrder", &duplicate),
        &packages,
    );
    let graph = edited.package.graph();
    let parent_order = graph
        .state_clause("ParentOrder")
        .expect("ParentOrder is checked")
        .identity();

    assert_eq!(
        node_keys(&base),
        node_keys(&edited),
        "a second declaration of the same clause adds no node"
    );
    assert_eq!(
        graph
            .state_clause("ParentOrder2")
            .expect("ParentOrder2 is checked")
            .identity(),
        parent_order,
        "ParentOrder2 checks to ParentOrder's node"
    );
    assert_eq!(
        occurrences_of(base.package.graph(), parent_order),
        occurrences_in("claim", &[0])
    );
    assert_eq!(
        occurrences_of(graph, parent_order),
        occurrences_in("claim", &[0, 1]),
        "ParentOrder's node gains a second claim occurrence, ordinal 1"
    );
}

/// TC-463 (FR-105-AC-4): adding a second `post` clause on `attemptUpdate`
/// adds one `state_clause` node and no second anchor or frame: the new
/// clause anchors at the existing `operation_anchor`, which gains a second
/// `anchor` occurrence.
#[trace("TC-463", "FR-105-AC-4")]
#[test]
fn s4_a_second_post_on_attempt_update_adds_one_clause_node_and_no_anchor_or_frame() {
    let (unit, packages) = config_version_unit_and_packages();
    let base = compile_config_version_unit(&unit, &packages);
    let edited = compile_config_version_unit(
        &insert_clause_after(
            &unit,
            "VersionUnchanged",
            "post NeverDecreases using v on Config::ConfigVersion::attemptUpdate { \
             pre(self.versionNumber) <= self.versionNumber }",
        ),
        &packages,
    );
    let base_graph = base.package.graph();
    let graph = edited.package.graph();

    assert_eq!(state_clause_nodes(base_graph).len(), 3);
    assert_eq!(
        state_clause_nodes(graph).len(),
        4,
        "one more state_clause node"
    );
    let anchor = single_state_node_key(graph, "operation_anchor");
    assert_eq!(
        anchor,
        single_state_node_key(base_graph, "operation_anchor"),
        "the same single operation_anchor"
    );
    assert_eq!(
        single_state_node_key(graph, "frame"),
        single_state_node_key(base_graph, "frame"),
        "the same single frame"
    );
    for name in ["VersionUnchanged", "NeverDecreases"] {
        assert_eq!(
            clause_anchor_target(named_clause_node(graph, name)),
            anchor,
            "{name} anchors at the shared operation_anchor"
        );
    }
    assert_eq!(
        occurrences_of(graph, anchor),
        occurrences_in("anchor", &[0, 1]),
        "one anchor occurrence per clause naming attemptUpdate"
    );
}

/// [`config_version_domain_document`] plus `Sub`, an object type whose
/// `supertypes` is `[ConfigVersion]` and which declares no field or
/// operation of its own, so `Config::Sub::attemptUpdate` is ConfigVersion's
/// inherited operation.
fn config_version_domain_document_with_sub() -> Vec<u8> {
    let config_version = config_version_type();
    let sub = format!("ix://{CONFIG_VERSION_PACKAGE_IDENTITY}/Sub");
    let mut envelope: serde_json::Value =
        serde_json::from_slice(&config_version_domain_document()).expect("the document is JSON");
    envelope["types"]
        .as_array_mut()
        .expect("types is an array")
        .push(json!({
            "identity": sub,
            "displayName": sub,
            "kind": {"module": CONFIG_VERSION_PACKAGE_IDENTITY, "name": "object_type"},
            "roles": [],
            "origin": {
                "generated": {
                    "generatorIdentity": sub,
                    "generatorVersion": "1.0.0",
                    "inputIdentities": [sub],
                }
            },
            "constraints": [],
            "extensions": [],
            "unknownPolicy": "reject",
            "supertypes": [config_version],
            "fields": [],
            "operations": [],
        }));
    envelope.to_string().into_bytes()
}

/// TC-463 (FR-105-AC-4): over a package where `Sub` specializes
/// `ConfigVersion`, `pre A ... on Config::ConfigVersion::attemptUpdate` and
/// `pre B ... on Config::Sub::attemptUpdate` share one `operation_anchor`,
/// whose context is ConfigVersion (the declaring type), never `Sub`.
#[trace("TC-463", "FR-105-AC-4")]
#[test]
fn s4_pre_clauses_via_config_version_and_sub_share_one_anchor_at_config_version() {
    let (unit, packages) =
        config_version_unit_and_packages_for(config_version_domain_document_with_sub());
    let unit = insert_clause_after(
        &unit,
        "VersionUnchanged",
        "pre A using v on Config::ConfigVersion::attemptUpdate { not reaches(self, self, parent) }\n\
         pre B using v on Config::Sub::attemptUpdate { self.versionNumber = self.versionNumber }",
    );
    let compiled = compile_config_version_unit(&unit, &packages);
    let graph = compiled.package.graph();
    let anchor = single_state_node_key(graph, "operation_anchor");
    let a = named_clause_node(graph, "A");
    let b = named_clause_node(graph, "B");

    assert_ne!(a.key(), b.key(), "A and B have distinct bodies");
    assert_eq!(clause_anchor_target(a), anchor);
    assert_eq!(
        clause_anchor_target(b),
        anchor,
        "B, named through Sub, anchors at the same operation_anchor as A"
    );
    let anchor_node = graph
        .semantic_graph()
        .node(anchor)
        .expect("the anchor node is in the graph");
    let qsl_semantics::check::BodyTerm::Aggregate(qsl_semantics::check::AggregateTerm { members }) =
        anchor_node.body()
    else {
        panic!("operation_anchor body must be an aggregate");
    };
    let Some(qsl_semantics::check::MemberTerm::Binding(qsl_semantics::check::Binding {
        name,
        value,
    })) = members.first()
    else {
        panic!("operation_anchor's first member is the context binding: {members:?}");
    };
    assert_eq!(name, "context");
    let config_version_node = config_version_node_key(graph);
    let qsl_semantics::check::BindingValue::Leaf(qsl_semantics::check::LeafTerm::Reference {
        target: context,
    }) = value
    else {
        panic!("the context binding's value is a reference, got {value:?}");
    };
    assert_eq!(
        context.0, config_version_node,
        "the shared anchor's context is ConfigVersion, the operation's declaring type"
    );
    assert_eq!(anchor_node.semantic_type(), Some(config_version_node));
    assert_eq!(
        occurrences_of(graph, anchor),
        occurrences_in("anchor", &[0, 1, 2]),
        "one anchor occurrence per clause naming attemptUpdate: VersionUnchanged, A and B"
    );
}

/// TC-463 step 1 (FR-105-AC-3): the emitted package reads back through
/// QSL's I2 reader (`read_import_view`), including its frame step, with its
/// recomputed `package_id` equal to the emitted one. `ParentOrder`'s
/// cycle-safety predicate lowers to a `quire.op.model.reaches_field`
/// application, so the read also checks that application's reference edge.
#[trace("TC-463", "FR-105-AC-3")]
#[test]
fn s4_state_package_reads_back_through_i2() {
    let (_, packages) = config_version_unit_and_packages();
    let first = config_version_compiled();

    let emission = qsl_package::emit_checked(&first.package)
        .unwrap_or_else(|refusal| panic!("emission refused: {refusal:?}"));
    assert_eq!(emission.package().package_id(), first.emitted.package_id());
    let library = LibraryName::new(CONFIG_VERSION_PACKAGE_IDENTITY.to_owned()).unwrap();
    let mut admitted = qsl_package::AdmittedPackages::default();
    let view = qsl_package::read_import_view(
        &first.package,
        &emission,
        library,
        &packages,
        &mut admitted,
        qsl_package::V2ReadLimits::default(),
    )
    .unwrap_or_else(|refusal| panic!("I2 read refused: {refusal:?}"));
    let _ = view;
}

mod call_site;
mod frame;
mod frame_replay;
mod pre_call_invocation;
mod state_clause_replay;
mod witness;
mod witness_member;
