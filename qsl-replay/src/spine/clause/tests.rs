// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-464 to TC-468 (FR-106, FR-107, FR-109, QSL-278).
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
//! only to a precondition's or postcondition's invocation document, which
//! no test in this file builds; `qsl-semantics/src/model/observation/
//! frame.rs` is exercised only by its own unit tests, not from here.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_eval::value::{CallFailure, CheckedPackageEvaluation, InputRefusal, QualifiedName};
use qsl_semantics::family::FamilyOutcome;
use qsl_semantics::model::key::hex;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::model::observation::{
    AdmittedObservations, ClauseSelection, ClauseSelectionInput, DocumentRef, Observation,
    ObservationLimits, SelectedAnchor, SelectedObject,
};
use quire_exact::{
    ChargePoint, EffectiveId, FieldValue, Meter, ObjectId, ObjectReference, Outcome, UniverseId,
    Value,
};
use serde_json::json;

use super::{
    run_clause, ClauseArgument, ClauseArgumentValue, ClauseDisposition, ClauseRunRefusal,
    ClauseRunRequest, ClauseRunSelection, ClauseRunStage,
};
use crate::spine::{compile, default_accounting, Compiled, DependencyInput, SpineLimits};

const PACKAGE_IDENTITY: &str = "test/nodes";
const PROFILE_DIGEST: &str =
    "sha256:c8c7ae9fbe783286369ecc83f006190f83be4c3c8fc585766617c90f27a25b16";
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
         profile v = \"quire.value.complete/v1\" version \"1-draft.2\" digest \"{PROFILE_DIGEST}\";\n\
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

fn compiled() -> Compiled {
    let (unit, packages) = unit_and_packages();
    compile(
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
    let value: serde_json::Value = serde_json::from_slice(bytes).expect("test fixture is JSON");
    let limits = quire_canonical::Limits::new(u64::MAX, quire_canonical::Limits::MAX_DEPTH)
        .expect("MAX_DEPTH is within MAX_DEPTH");
    *quire_canonical::sha256(&value, limits)
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
                None => json!({"absent": null}),
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
    types: &qsl_semantics::value::declaration::TypeEnvironment,
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
    ObjectEnvironment::new(types, objects).expect("the chain is internally closed")
}

fn no_cycle_observations(
    clause_identity: quire_exact::NodeKey,
    environment: ObjectEnvironment,
    self_object: ObjectReference,
) -> AdmittedObservations {
    AdmittedObservations {
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
        .expect("NoCycle evaluates");
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
    )
    .expect("the two-node cycle is internally closed");
    let observations = no_cycle_observations(
        clause.identity(),
        environment,
        node_reference(node_effective, "a"),
    );
    let mut meter = Meter::new(default_accounting(1_000_000));
    let name = QualifiedName::unqualified("NoCycle").unwrap();
    let evaluation = package
        .evaluate_clause(&name, &observations, &mut meter)
        .expect("NoCycle evaluates");
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

    let run = || {
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
            .unwrap();
        (
            boolean_outcome(evaluation),
            meter.admitted_charges().to_vec(),
        )
    };
    assert_eq!(run(), run());
}

/// FR-090/FR-107: an exhausted `work_units` budget completes `incomplete`,
/// never a fault or a silent truncation.
#[trace("TC-467", "FR-107-AC-7")]
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
    let mut meter = Meter::new(default_accounting(0));
    let name = QualifiedName::unqualified("NoCycle").unwrap();
    let evaluation = package
        .evaluate_clause(&name, &observations, &mut meter)
        .expect("an exhausted meter is a kernel outcome, not a fault");
    assert!(
        matches!(
            evaluation.outcome,
            FamilyOutcome::Evaluated(Outcome::Incomplete(_))
        ),
        "expected Incomplete, got {:?}",
        evaluation.outcome
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
        source: source(),
        path: "clause-run.native".to_owned(),
        bytes: unit.into_bytes(),
        packages,
        dependencies: DependencyInput::default(),
        snapshots: BTreeMap::new(),
        invocations: BTreeMap::new(),
        selection,
        expected_package_id: None,
        limits: SpineLimits::default(),
        observation_limits: ObservationLimits::default(),
        model_limits: qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        accounting: default_accounting(1_000_000),
    }
}

/// TC-468 (FR-109-AC-1): a `Clause` selection over a `current` snapshot of
/// an acyclic chain evaluates `NoCycle` to `Completed(Boolean(true))`.
#[trace("TC-468", "FR-109-AC-1")]
#[test]
fn run_clause_evaluates_a_clause_selection() {
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
    let selection = no_cycle_selection(label.clone(), "a");

    let mut request = request(ClauseRunSelection::Clause(selection));
    request.snapshots.insert(digest, bytes);
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
    assert_eq!(report.exit_code(), 0);
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
    assert_eq!(report.exit_code(), 20);
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
/// resolves it in the current snapshot (`ObjectEnvironment::find`,
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
            assert_eq!(record.code, "invalid_runtime_input");
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
#[trace("TC-468", "FR-109-AC-6")]
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
    assert_eq!(report.exit_code(), 20);
}

/// [`ClauseRunRefusal::EmptySource`]: an empty unit refuses before any
/// stage runs.
#[trace("TC-468", "FR-109-AC-7")]
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
    request.bytes = Vec::new();
    assert!(matches!(
        run_clause(request),
        Err(ClauseRunRefusal::EmptySource)
    ));
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
            assert_eq!(record.code, "invalid_runtime_input");
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
            assert_eq!(record.code, "invalid_runtime_input");
            assert_eq!(record.cause, "wrong-value-kind");
            assert_eq!(
                record.fields.get("field").map(String::as_str),
                Some("digest")
            );
        }
        other => panic!("expected Admit(Refused(wrong-value-kind/digest)), got {other:?}"),
    }
}
