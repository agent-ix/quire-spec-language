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
    ObjectEnvironment::new(types, objects, &[]).expect("the chain is internally closed")
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
        &[],
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
            .unwrap();
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
        .expect("the two-node cycle is internally closed")
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
        .expect("an exhausted meter is a kernel outcome, not a fault");
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
        .expect("the default budget evaluates cleanly");
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
/// a cyclic chain to `Completed(Boolean(false))`; both carry the source
/// digest, `package_id`, model selection, selection and the one snapshot's
/// identity and digest in their provenance
/// (`FR-109-run-a-state-clause-through-the-spine.md:149,81-84`).
#[trace("TC-468", "FR-109-AC-1")]
#[test]
fn run_clause_evaluates_a_clause_selection() {
    let (request, label) = no_cycle_request(&[("a", Some("b")), ("b", None)]);
    let expected_digest = qsl_foundation::ByteDigest::of(&request.bytes).to_string();
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
    assert_eq!(report.exit_code(), 0);

    // Provenance: source digest, package_id, model selection, selection and
    // the one snapshot's identity and digest.
    assert_eq!(report.source_digest, expected_digest);
    assert!(report.package_id.is_some());
    assert!(
        !report.provenance.model_selections.is_empty(),
        "the model selection the compiled package resolved is reported"
    );
    match &report.provenance.selection {
        super::ClauseRunSelection::Clause(selection) => {
            assert_eq!(selection.name, "NoCycle");
        }
        other => panic!("expected the Clause selection back, got {other:?}"),
    }
    assert_eq!(report.provenance.documents, [label]);
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
    assert_eq!(report.exit_code(), 10);
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

    assert_eq!(report_one.source_digest, report_two.source_digest);
    assert_eq!(report_one.package_id, report_two.package_id);
    assert_eq!(
        report_one.disposition.truth(),
        report_two.disposition.truth()
    );
    assert_eq!(
        report_one.disposition.category(),
        report_two.disposition.category()
    );
    assert_eq!(report_one.exit_code(), report_two.exit_code());
    assert_eq!(
        report_one.provenance.documents,
        report_two.provenance.documents
    );
    assert_eq!(
        report_one.provenance.model_selections.len(),
        report_two.provenance.model_selections.len()
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
    assert_eq!(report.exit_code(), 20);
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
    request.bytes = Vec::new();
    assert!(matches!(
        run_clause(request),
        Err(ClauseRunRefusal::EmptySource)
    ));
}

/// TC-468 step 2 (FR-109-AC-2): no domain package bytes supplied for the
/// unit's `model` declaration refuses at stage `compile`,
/// `missing_import`/`missing-selection`, exit 20, with no snapshot in
/// provenance (FR-109-AC-2's own worked example: a compile-stage failure
/// never read a document, so `provenance.documents` stays empty --
/// `FR-109-run-a-state-clause-through-the-spine.md:81-84`).
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
    assert_eq!(report.exit_code(), 20);
    assert!(
        report.provenance.documents.is_empty(),
        "a compile-stage failure never read a document"
    );
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

/// The `test/config-version` domain package: `ConfigVersion` (`versionNumber:
/// Integer`, `parent: Reference<ConfigVersion>?`), one operation
/// `attemptUpdate` (`modifies [versionNumber, parent]`, no parameters,
/// returns `Boolean`), one closed population `config_history`.
fn config_version_domain_document() -> Vec<u8> {
    let config_version = config_version_type();
    let version_number_identity = format!("{config_version}/versionNumber");
    let parent_identity = format!("{config_version}/parent");
    let operation_identity = format!("{config_version}/attemptUpdate");
    let population = config_version_population_identity();
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
        ],
        "types": [
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
                        "typeRef": "ix://quire/native/Integer",
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
                "operations": [
                    {
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
                        "frame": {
                            "modifies": [version_number_identity, parent_identity],
                            "creates": [],
                            "deletes": [],
                        },
                    },
                ],
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
/// `Config`, plus `ParentOrder` (invariant).
///
/// **Pending QSL-279** (TC-466 Status; not this fix round's own finding): a
/// postcondition of `attemptUpdate` (`VersionUnchanged`, or a
/// `pre(..)`-reading clause for TC-466 step 2) cannot be added to this
/// file's fixture. `evaluate_clause` is sealed to
/// `qsl_package::checked::CheckedPackage` (`qsl-eval/src/value/expression/
/// mod.rs:314`), reachable only through full compile-to-emit; reaching S5
/// (`qsl_package::emit::emit_checked`) with any clause that uses the
/// operation refuses `EmitRefusal::UnlocatedOccurrence` (`qsl-package/src/
/// emit.rs:142`, "Source regions", emit.rs:35-42), because FR-105's own
/// state-node emission is not implemented yet ("Today QSL emits no `state`
/// node", `spec/functional/FR-105-emit-state-nodes.md`): a hand-authored
/// domain-package operation's `origin.source` here is a synthetic
/// `{sourceIdentity, path, startLine, startColumn}` (the same shape
/// `qsl-semantics`' own TC-458 fixture uses, `model_operations.rs`'s
/// `operation()`), never a byte source S1/S2 ever actually parsed, and
/// `qsl-semantics`' own `tests/it` suite only checks through S3/S4
/// (`PackageDeclarations::check`), never S5, so this gap is invisible
/// there. Confirmed by removing each new clause one at a time:
/// `ParentOrder` alone (no clause referencing `attemptUpdate`) compiles
/// through `run_clause` cleanly; any postcondition of `attemptUpdate`
/// reaches the same `UnlocatedOccurrence` refusal regardless of its body.
/// QSL-279 ("A05-8: S4 state node emission and the ConfigVersion spine
/// corpus, FR-105, FR-108") is the ticket that implements the missing
/// emission this depends on.
fn config_version_unit_and_packages() -> (String, BTreeMap<[u8; 32], Vec<u8>>) {
    let document = config_version_domain_document();
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let digest = hex(digest);
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\" version \"1-draft.2\" digest \"{PROFILE_DIGEST}\";\n\
         model Config = {CONFIG_VERSION_PACKAGE_IDENTITY:?} version \"1.0.0\" \
         digest \"sha256-jcs:{digest}\";\n\
         invariant ParentOrder using v on Config::ConfigVersion at current {{ \
         present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber }}\n\
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

fn config_version_request(selection: ClauseRunSelection) -> ClauseRunRequest {
    let (unit, packages) = config_version_unit_and_packages();
    ClauseRunRequest {
        source: source(),
        path: "clause-run-config-version.native".to_owned(),
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

fn config_version_document_ref(identity: &str) -> DocumentRef {
    DocumentRef {
        authority: "test".to_owned(),
        identity: identity.to_owned(),
        revision_namespace: "ns".to_owned(),
        revision: "1".to_owned(),
        digest: [0; 32],
    }
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
    assert_eq!(report.exit_code(), 20);
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
    assert_eq!(report.exit_code(), 20);
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
    assert_eq!(report.exit_code(), 22);
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
        ClauseDisposition::Evaluate(super::CallOutcome::Incomplete { limit }) => {
            assert_eq!(*limit, quire_exact::LimitKind::WorkUnits.as_str());
        }
        other => panic!("expected Evaluate(Incomplete {{ .. }}), got {other:?}"),
    }
    assert_eq!(
        report.disposition.category(),
        qsl_foundation::diagnostic::Category::Incomplete
    );
    assert_eq!(report.disposition.truth(), None);
    assert_eq!(report.exit_code(), 22);
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

/// TC-468 step 5 (FR-109-AC-5): editing the snapshot bytes after the
/// selection's digest was taken (so the stored digest no longer matches the
/// bytes it labels) refuses `admit`, `stale_dependency`/
/// `byte-digest-mismatch`.
#[trace("TC-468", "FR-109-AC-5")]
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
            assert_eq!(record.cause, "byte-digest-mismatch");
        }
        other => panic!("expected Admit(Refused(byte-digest-mismatch)), got {other:?}"),
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
            quire_exact::Refusal::CheckedInvariant,
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
    assert_eq!(report_for(disposition).exit_code(), 30);

    // A `CallFailure::Fault` reaches `run_clause`'s own `EvaluateFault` arm
    // the same way (`clause.rs:523-524`), with the same reported shape --
    // constructed directly here, not run through `package.evaluate_clause`
    // (SR-751 FND-008 round 2, disclosed, not attempted): every
    // `CallFailure::Fault` `evaluate_clause` can itself construct
    // (`qsl-eval/src/value/expression/mod.rs:456,475,486`,
    // "clause-observations-missing-current-or-post"/
    // "postcondition-result-missing"/"clause-parameter-not-admitted") is
    // unreachable once admission has actually succeeded: FR-106's
    // `admit_operation` unconditionally sets both `pre` and `post` on
    // success (never leaving both `current` and `post` `None`), always
    // admits a `result` for an operation `binds_result()` declares
    // (refusing at admission otherwise, never reaching evaluate), and
    // always admits every one of the clause's own declared parameters
    // (check 10) -- the same "broken invariant, not a document defect"
    // shape `CheckedInvariant` above already is. This test file also has
    // no compiled package with a precondition/postcondition clause to
    // attempt it against in the first place: any clause on `attemptUpdate`
    // fails `spine::compile` (SR-751 FND-003, deferred to QSL-279, a
    // dependency this round does not touch), and `binds_result()`/`pre`/
    // `post` are meaningful only for a precondition or postcondition.
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
    assert_eq!(report_for(disposition_from_call_failure).exit_code(), 30);
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
    assert_eq!(report_for(disposition).exit_code(), 0);

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
    assert_eq!(report_for(disposition).exit_code(), 10);

    // A kernel `Undefined`: undefined, exit 20.
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
    assert_eq!(report_for(disposition).exit_code(), 20);

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
    assert_eq!(report_for(disposition).exit_code(), 22);
}

/// SR-750 FND-011 round 3: `AdmissionRecord.code` is a typed `Code`, so
/// `exit_code`'s `Admit(Refused | Incomplete)` arm maps every catalog code
/// to that code's own exit status, with no string lookup and no panic path.
#[test]
fn exit_code_maps_every_admission_record_code_to_its_own_exit_status() {
    for code in qsl_foundation::diagnostic::Code::all() {
        for failure in [
            qsl_semantics::model::observation::AdmissionFailure::Refused as fn(_) -> _,
            qsl_semantics::model::observation::AdmissionFailure::Incomplete,
        ] {
            let record = qsl_semantics::model::observation::AdmissionRecord {
                code: *code,
                cause: "any-cause",
                fields: BTreeMap::new(),
            };
            let disposition = ClauseDisposition::Admit(failure(record));
            assert_eq!(
                report_for(disposition).exit_code(),
                code.exit_code(),
                "{code}"
            );
        }
    }
}

/// Wraps a bare [`ClauseDisposition`] in a minimal [`ClauseRunReport`], for
/// `exit_code` (an inherent method of the report, not the disposition).
fn report_for(disposition: ClauseDisposition) -> super::ClauseRunReport {
    super::ClauseRunReport {
        source_digest: String::new(),
        package_id: None,
        disposition,
        provenance: super::ClauseRunProvenance {
            model_selections: Vec::new(),
            selection: ClauseRunSelection::Function {
                name: String::new(),
                arguments: Vec::new(),
                snapshot: config_version_document_ref("unused"),
            },
            documents: Vec::new(),
        },
        usage: super::ClauseRunUsage::default(),
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
