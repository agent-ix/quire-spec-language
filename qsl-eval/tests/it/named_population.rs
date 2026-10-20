// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-391 (TC-1076): a state clause or a `holds` atom that names a
//! domain-package population, evaluated at S6a over the snapshot of its
//! observation; its read footprint; and the package emission that refuses
//! it until FR-322 encodes the population-reference term. The S3 half is
//! `qsl-semantics`'s `named_population`.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_package::{emit_checked, EmitRefusal};
use qsl_semantics::check::footprint::{derive_footprint, SymbolicLocation};
use qsl_semantics::check::{CheckedGraph, FormulaNode};
use qsl_semantics::family::{FamilyOutcome, FamilyResult};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::observation::{
    admit_observations, AdmissionFailure, AdmittedObservations, ClauseFacts, ClauseSelection,
    ClauseSelectionInput, DocumentRef, ObservationLimits, OperationFacts, Provisions,
    SelectedAnchor, SelectedObject,
};
use qsl_semantics::model::operation::OperationLookup;
use qsl_semantics::model::state::StateModel;
use quire_exact::{Meter, Outcome, ScalarLimits, Value};
use serde_json::{json, Value as Json};

#[path = "../../../qsl-semantics/tests/support/named_population.rs"]
mod fixture;

use fixture::{checked, ident, key, PACKAGE};

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

/// A checked unit, its packages and the snapshots its tests add.
struct Unit {
    package: CheckedPackage,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
}

impl Unit {
    fn new(archive: bool, declarations: &str) -> Self {
        let checked = checked(archive, declarations);
        Self {
            package: CheckedPackage::link(checked.graph),
            packages: checked.packages,
            snapshots: BTreeMap::new(),
        }
    }

    fn graph(&self) -> &CheckedGraph {
        self.package.graph()
    }

    /// A snapshot `label` at `observation`: the counters `counters` (key and
    /// `v`), listed complete when `complete`, and the widget `w1`, the
    /// watcher `w` and no `Other`, each listed complete.
    fn snapshot(
        &mut self,
        label: &str,
        observation: &str,
        counters: &[(&str, i64)],
        complete: bool,
    ) -> DocumentRef {
        let digest = *self.packages.keys().next().expect("one package");
        let counters: Vec<Json> = counters
            .iter()
            .map(|(key, v)| {
                json!({"key": key, "type": ident("Counter"),
                    "fields": {"v": {"integer": v.to_string()}}})
            })
            .collect();
        let mut document = json!({
            "format": "quire.state.snapshot/v1",
            "identity": {"authority": "test", "identity": label,
                "revision_namespace": "fixture", "revision": "1"},
            "observation": observation,
            "model": {"identity": PACKAGE, "version": "1.0.0",
                "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(&digest))},
            "populations": [
                {"population": ident("counters"), "complete": complete, "objects": counters},
                {"population": ident("widgets"), "complete": true, "objects": [
                    {"key": "w1", "type": ident("Widget"), "fields": {}}]},
                {"population": ident("watchers"), "complete": true, "objects": [
                    {"key": "w", "type": ident("Watch"), "fields": {}}]},
                {"population": ident("others"), "complete": true, "objects": []},
            ],
        });
        if observation == "current" {
            document["anchor"] = json!({"kind": "handler", "name": "validate"});
        }
        let bytes = serde_json::to_vec(&document).expect("the snapshot serializes");
        let read = quire_canonical::read(&bytes, u64::MAX).expect("JSON");
        let digest = *quire_canonical::sha256(&read, quire_canonical::Limits::new(u64::MAX))
            .expect("canonical")
            .as_bytes();
        self.snapshots.insert(digest, bytes);
        DocumentRef {
            authority: "test".to_owned(),
            identity: label.to_owned(),
            revision_namespace: "fixture".to_owned(),
            revision: "1".to_owned(),
            digest,
        }
    }

    /// The observations of the clause `name` selected by `input`.
    fn admit(
        &self,
        name: &str,
        input: ClauseSelectionInput,
    ) -> Result<AdmittedObservations, AdmissionFailure> {
        let clause = self.graph().state_clause(name).expect("the clause");
        let facts = ClauseFacts {
            identity: clause.identity(),
            kind: clause.kind(),
            context: clause.context(),
            operation: clause.operation().map(|operation| OperationFacts {
                declaring: operation.declaring,
                declaration: operation.declaration.clone(),
            }),
            operations: self.graph().scope().operations().clone(),
            named_populations: clause.named_reads(),
        };
        admit_observations(
            self.graph().model_selections(),
            self.graph().scope().types(),
            &facts,
            &self.packages,
            ModelNormalizationLimits::default(),
            &Provisions {
                snapshots: &self.snapshots,
                invocations: &BTreeMap::new(),
            },
            &ClauseSelection {
                name: name.to_owned(),
                input,
            },
            ObservationLimits::default(),
        )
    }

    /// The pre-call observation of the precondition `name` with `self` the
    /// object `key` of `population`.
    fn pre_call(
        &self,
        name: &str,
        snapshot: &DocumentRef,
        (population, key): (&str, &str),
    ) -> Result<AdmittedObservations, AdmissionFailure> {
        self.admit(
            name,
            ClauseSelectionInput::PreCall {
                snapshot: snapshot.clone(),
                self_object: Some(SelectedObject {
                    population: ident(population),
                    key: key.to_owned(),
                }),
                parameters: BTreeMap::new(),
            },
        )
    }

    /// The current observation of the invariant `name` with `self` the
    /// counter `key`.
    fn current(
        &self,
        name: &str,
        snapshot: &DocumentRef,
        key: &str,
    ) -> Result<AdmittedObservations, AdmissionFailure> {
        self.admit(
            name,
            ClauseSelectionInput::Current {
                snapshot: snapshot.clone(),
                anchor: SelectedAnchor {
                    kind: qsl_semantics::model::observation::AnchorKind::Handler,
                    name: "validate".to_owned(),
                },
                self_object: Some(SelectedObject {
                    population: ident("counters"),
                    key: key.to_owned(),
                }),
            },
        )
    }

    /// The outcome of the clause `name` over `observations`.
    fn evaluate(&self, name: &str, observations: &AdmittedObservations) -> FamilyOutcome<Value> {
        self.package
            .evaluate_clause(
                &QualifiedName::unqualified(name).expect("a name"),
                observations,
                &mut Meter::new(UNLIMITED),
            )
            .expect("the observations evaluate")
            .evaluation
            .outcome
    }

    /// The truth of the clause `name` over `observations`.
    fn holds(&self, name: &str, observations: &AdmittedObservations) -> bool {
        match self.evaluate(name, observations) {
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(holds))) => holds,
            other => panic!("{name} decides: {other:?}"),
        }
    }
}

/// The admission failure is FR-106's `Incomplete`
/// `incomplete_population`/`incomplete-scope`.
fn assert_incomplete(admitted: Result<AdmittedObservations, AdmissionFailure>) {
    let Err(AdmissionFailure::Incomplete(record)) = admitted else {
        panic!("an incomplete admission: {admitted:?}");
    };
    assert_eq!(
        (record.code.as_str(), record.cause),
        ("incomplete_population", "incomplete-scope")
    );
}

const ROOM: &str = "pre Room using v on M::Counter::bump { \
                    forall(o in allInstances<M::Counter>(M::counters) : o.v < 2) }";
const SIZE: &str = "pre Size using v on M::Counter::bump { \
                    size(allInstances<M::Counter>(M::counters)) = 3 }";
const GROW: &str = "pre Grow using v on M::Widget::grow { \
                    forall(o in allInstances<M::Counter>(M::counters) : o.v < 2) }";
const OTHER: &str = "pre Others using v on M::Widget::grow { \
                     size(allInstances<M::Other>(M::counters)) = 0 }";

/// TC-1076 row 7 (FR-391-AC-5): the precondition of AC-1 decides over
/// exactly the snapshot's `a`, `b` and `c`; a `counters` not listed
/// `complete` is `Incomplete` `incomplete_population`/`incomplete-scope`
/// before evaluation, even when `self` is a widget; `allInstances<M::Other>`
/// over `counters`, which no member type covers, is FR-084's typed
/// incomplete result, `Undefined` with reason `incomplete-closure` naming
/// `Other`.
#[trace("TC-1076", "FR-391-AC-5")]
#[test]
fn a_named_population_is_read_from_the_snapshot_of_the_clause_s_observation() {
    let mut unit = Unit::new(false, &format!("{ROOM}\n{SIZE}\n{GROW}\n{OTHER}"));
    let low = unit.snapshot("low", "pre", &[("a", 0), ("b", 1), ("c", 1)], true);
    let high = unit.snapshot("high", "pre", &[("a", 0), ("b", 1), ("c", 2)], true);
    let open = unit.snapshot("open", "pre", &[("a", 0), ("b", 1), ("c", 1)], false);
    let room = |snapshot| unit.pre_call("Room", snapshot, ("counters", "a"));
    assert!(unit.holds("Room", &room(&low).expect("admits")));
    assert!(
        !unit.holds("Room", &room(&high).expect("admits")),
        "c is read"
    );
    let size = unit
        .pre_call("Size", &low, ("counters", "a"))
        .expect("admits");
    assert!(unit.holds("Size", &size), "exactly a, b and c");

    let grow = |snapshot| unit.pre_call("Grow", snapshot, ("widgets", "w1"));
    assert!(unit.holds("Grow", &grow(&low).expect("admits")));
    assert!(!unit.holds("Grow", &grow(&high).expect("admits")));
    assert_incomplete(grow(&open));

    let others = unit
        .pre_call("Others", &low, ("widgets", "w1"))
        .expect("admits");
    let FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause)) =
        unit.evaluate("Others", &others)
    else {
        panic!("an undefined result");
    };
    let record = cause.undefined_record();
    assert_eq!(record.reason.as_str(), "incomplete-closure");
    assert_eq!(record.fields["queried_type"], ident("Other"));
}

/// The symbolic footprint of `operation` on `object_type` in `unit`.
fn footprint(
    unit: &Unit,
    object_type: &str,
    operation: &str,
) -> qsl_semantics::check::footprint::SymbolicFootprint {
    let graph = unit.graph();
    let model = StateModel::new(
        graph.model_selections(),
        graph.scope().types(),
        &unit.packages,
        ModelNormalizationLimits::default(),
    )
    .expect("model views");
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
        panic!("{object_type} declares {operation}");
    };
    derive_footprint(graph, &model, identity, declaring, operation, &[])
        .expect("a footprint derives")
}

/// TC-1076 row 8, the footprint and emission half (FR-391-AC-6): `A` reads
/// `AnyMembership{counters}` and not `archive`'s, `B` the reverse, each
/// beside the receiver's own membership; FR-155-AC-2's precondition over
/// `counters` reads `AnyField{v}` and `AnyMembership{counters}`; the package
/// refuses at emission `unsupported_construct`/`not-yet-implemented`.
#[trace("TC-1076", "FR-391-AC-6", "TC-574", "FR-155-AC-2")]
#[test]
fn a_named_population_is_the_one_membership_read_and_is_not_emitted() {
    const A: &str = "pre A using v on M::Widget::grow { \
                     forall(o in allInstances<M::Counter>(M::counters) : true) }";
    const B: &str = "pre B using v on M::Widget::grow { \
                     forall(o in allInstances<M::Counter>(M::archive) : true) }";
    for (clause, read) in [(A, "counters"), (B, "archive")] {
        let grow = footprint(&Unit::new(true, clause), "Widget", "grow");
        let memberships: Vec<_> = grow
            .reads
            .iter()
            .filter_map(|location| match location {
                SymbolicLocation::AnyMembership { population } => Some(population.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(memberships, [key(read)], "{clause}");
        assert!(
            grow.reads
                .iter()
                .any(|location| matches!(location, SymbolicLocation::Membership { .. })),
            "the receiver's own membership: {clause}"
        );
    }

    let unit = Unit::new(false, ROOM);
    let bump = footprint(&unit, "Counter", "bump");
    assert!(bump.reads.contains(&SymbolicLocation::AnyField {
        field: key("Counter/v")
    }));
    assert!(bump.reads.contains(&SymbolicLocation::AnyMembership {
        population: key("counters")
    }));

    let Err(refusal) = emit_checked(&unit.package) else {
        panic!("emission refuses the population reference");
    };
    assert!(matches!(
        &refusal,
        EmitRefusal::PopulationReferenceNotEmitted { population } if *population == ident("counters")
    ));
    assert_eq!(
        (refusal.code().as_str(), refusal.cause()),
        ("unsupported_construct", Some("not-yet-implemented"))
    );
}

/// TC-1076 row 9, the evaluation half (FR-391-AC-7): the precondition
/// ranging `exists` over `M::counters` and the invariant counting over it
/// evaluate to the result of their `allInstances` spellings; a `holds` atom
/// ranging over `M::counters` reads the observed snapshot; a `counters` not
/// listed `complete` is `Incomplete`.
#[trace("TC-1076", "FR-391-AC-7")]
#[test]
fn a_population_domain_evaluates_as_its_all_instances_spelling() {
    let mut unit = Unit::new(
        false,
        "bound B on M::counters [3]\n\
         pre Q using v on M::Counter::bump { exists(o in M::counters : o.v = 2) }\n\
         pre QAll using v on M::Counter::bump { \
         exists(o in allInstances<M::Counter>(M::counters) : o.v = 2) }\n\
         pre GrowQ using v on M::Widget::grow { exists(o in M::counters : o.v = 2) }\n\
         invariant Cap using v on M::Counter at current { \
         count(o in M::counters where o.v = 0) < 2 }\n\
         invariant CapAll using v on M::Counter at current { \
         size(filter(o in allInstances<M::Counter>(M::counters) : o.v = 0)) < 2 }\n\
         temporal C using t over (w: M::Watch) clock \"model-steps\" on origin { \
         eventually holds(forall(x in M::counters : x.v < 2)) }",
    );
    let shapes: [&[(&str, i64)]; 2] = [&[("a", 0), ("b", 2)], &[("a", 0), ("b", 0)]];
    for (index, counters) in shapes.into_iter().enumerate() {
        let pre = unit.snapshot(&format!("pre{index}"), "pre", counters, true);
        let current = unit.snapshot(&format!("now{index}"), "current", counters, true);
        let q = unit.holds(
            "Q",
            &unit.pre_call("Q", &pre, ("counters", "a")).expect("Q"),
        );
        let q_all = unit.holds(
            "QAll",
            &unit
                .pre_call("QAll", &pre, ("counters", "a"))
                .expect("QAll"),
        );
        assert_eq!(q, q_all, "{counters:?}");
        assert_eq!(q, index == 0, "{counters:?}");
        let cap = unit.holds("Cap", &unit.current("Cap", &current, "a").expect("Cap"));
        let cap_all = unit.holds(
            "CapAll",
            &unit.current("CapAll", &current, "a").expect("CapAll"),
        );
        assert_eq!(cap, cap_all, "{counters:?}");
        assert_eq!(cap, index == 0, "{counters:?}");

        // The `holds` atom over the current snapshot's objects.
        let observations = unit.current("Cap", &current, "a").expect("Cap");
        let objects = &observations.current.as_ref().expect("current").environment;
        let [clause] = unit.graph().temporal_clauses() else {
            panic!("one temporal clause");
        };
        let atom = clause
            .clause()
            .formula()
            .nodes()
            .iter()
            .position(|node| matches!(node, FormulaNode::Holds { .. }))
            .and_then(|index| clause.clause().formula().node_id(index))
            .expect("a holds atom");
        let watcher = objects
            .references()
            .iter()
            .find(|reference| reference.object().as_str() == "w")
            .expect("the watcher")
            .clone();
        let evaluated = unit
            .package
            .evaluate_holds(
                clause,
                atom,
                Value::Reference(watcher),
                objects,
                &mut Meter::new(UNLIMITED),
            )
            .expect("the atom evaluates");
        assert!(
            matches!(
                evaluated.outcome,
                FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(holds)))
                    if holds == (index == 1)
            ),
            "{counters:?}: {:?}",
            evaluated.outcome
        );
    }
    let open = unit.snapshot("open", "pre", &[("a", 0)], false);
    assert_incomplete(unit.pre_call("GrowQ", &open, ("widgets", "w1")));
}
