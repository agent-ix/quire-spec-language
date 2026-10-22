// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-391 (TC-1076): a state clause or a `holds` atom that names a
//! domain-package population, evaluated at S6a over the snapshot of its
//! observation; its read footprint; and the package emission that writes
//! its population reference (FR-322). The S3 half is `qsl-semantics`'s
//! `named_population`.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_package::emit_checked;
use qsl_semantics::check::footprint::{derive_footprint, SymbolicLocation};
use qsl_semantics::check::{CheckedGraph, FormulaNode};
use qsl_semantics::family::{FamilyOutcome, FamilyResult};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::observation::{
    admit_observations, admit_trace_snapshot, AdmissionFailure, AdmittedObservations, ClauseFacts,
    ClauseSelection, ClauseSelectionInput, DocumentRef, ObservationLimits, OperationFacts,
    Provisions, SelectedAnchor, SelectedObject,
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
    invocations: BTreeMap<[u8; 32], Vec<u8>>,
}

/// What a snapshot of the fixture lists: the counters (key and `v`), and
/// whether `counters` and `others` are listed `complete`; `link` makes the
/// first counter reference the `Other` `x`.
struct Listing<'c> {
    counters: &'c [(&'c str, i64)],
    complete: bool,
    others_complete: bool,
    link: bool,
}

/// A document's reference, by its canonical digest.
fn document_ref(label: &str, bytes: &[u8]) -> DocumentRef {
    let read = quire_canonical::read(bytes, u64::MAX).expect("JSON");
    let digest = *quire_canonical::sha256(&read, quire_canonical::Limits::new(u64::MAX))
        .expect("canonical")
        .as_bytes();
    DocumentRef {
        authority: "test".to_owned(),
        identity: label.to_owned(),
        revision_namespace: "fixture".to_owned(),
        revision: "1".to_owned(),
        digest,
    }
}

fn identity(reference: &DocumentRef) -> Json {
    json!({"authority": reference.authority, "identity": reference.identity,
        "revision_namespace": reference.revision_namespace, "revision": reference.revision})
}

impl Unit {
    fn new(archive: bool, declarations: &str) -> Self {
        let checked = checked(archive, declarations);
        Self {
            package: CheckedPackage::link(checked.graph),
            packages: checked.packages,
            snapshots: BTreeMap::new(),
            invocations: BTreeMap::new(),
        }
    }

    fn model(&self) -> Json {
        let digest = *self.packages.keys().next().expect("one package");
        json!({"identity": PACKAGE, "version": "1.0.0",
            "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(&digest))})
    }

    /// The invocation `label` of `Counter::bump` on the counter `a`, between
    /// `pre` and `post`.
    fn invocation(&mut self, label: &str, pre: &DocumentRef, post: &DocumentRef) -> DocumentRef {
        let reference = |reference: &DocumentRef| {
            json!({"identity": identity(reference),
                "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(&reference.digest))})
        };
        let bytes = serde_json::to_vec(&json!({
            "format": "quire.state.invocation/v1",
            "identity": {"authority": "test", "identity": label,
                "revision_namespace": "fixture", "revision": "1"},
            "model": self.model(),
            "context": ident("Counter"),
            "operation": "bump",
            "self": {"population": ident("counters"), "key": "a"},
            "pre": reference(pre),
            "post": reference(post),
            "parameters": {},
            "result": null,
            "created": [],
            "deleted": [],
        }))
        .expect("the invocation serializes");
        let document = document_ref(label, &bytes);
        self.invocations.insert(document.digest, bytes);
        document
    }

    /// The invocation observation of the postcondition `name`.
    fn invoked(
        &self,
        name: &str,
        invocation: &DocumentRef,
    ) -> Result<AdmittedObservations, AdmissionFailure> {
        self.admit(
            name,
            ClauseSelectionInput::Invocation {
                invocation: invocation.clone(),
            },
        )
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
        self.listed(
            label,
            observation,
            &Listing {
                counters,
                complete,
                others_complete: true,
                link: false,
            },
        )
    }

    /// A snapshot `label` at `observation` listing `listing`, the widget
    /// `w1` and the watcher `w`.
    fn listed(&mut self, label: &str, observation: &str, listing: &Listing<'_>) -> DocumentRef {
        let counters: Vec<Json> = listing
            .counters
            .iter()
            .enumerate()
            .map(|(index, (key, v))| {
                let link = if listing.link && index == 0 {
                    json!({"present": {"reference": {"population": ident("others"), "key": "x"}}})
                } else {
                    json!({"absent": {}})
                };
                let fields = json!({"v": {"integer": v.to_string()}, "link": link});
                json!({"key": key, "type": ident("Counter"), "fields": fields})
            })
            .collect();
        let mut document = json!({
            "format": "quire.state.snapshot/v1",
            "identity": {"authority": "test", "identity": label,
                "revision_namespace": "fixture", "revision": "1"},
            "observation": observation,
            "model": self.model(),
            "populations": [
                {"population": ident("counters"), "complete": listing.complete,
                    "objects": counters},
                {"population": ident("widgets"), "complete": true, "objects": [
                    {"key": "w1", "type": ident("Widget"), "fields": {}}]},
                {"population": ident("watchers"), "complete": true, "objects": [
                    {"key": "w", "type": ident("Watch"), "fields": {}}]},
                {"population": ident("others"), "complete": listing.others_complete,
                    "objects": []},
            ],
        });
        if observation == "current" {
            document["anchor"] = json!({"kind": "handler", "name": "validate"});
        }
        let bytes = serde_json::to_vec(&document).expect("the snapshot serializes");
        let document = document_ref(label, &bytes);
        self.snapshots.insert(document.digest, bytes);
        document
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
                invocations: &self.invocations,
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

/// TC-1076 row 8, the footprint half (FR-391-AC-6): `A` reads
/// `AnyMembership{counters}` and not `archive`'s, `B` the reverse, each
/// beside the receiver's own membership; FR-155-AC-2's precondition over
/// `counters` reads `AnyField{v}` and `AnyMembership{counters}`.
#[trace("TC-1076", "FR-391-AC-6", "TC-574", "FR-155-AC-2")]
#[test]
fn a_named_population_is_the_one_membership_read() {
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
}

/// TC-1076 row 9, the evaluation half (FR-391-AC-7): the postcondition `P`,
/// the precondition `Q` and the invariant `Cap`, each over `M::counters`,
/// evaluate to the result of their `allInstances` spellings, and the `holds`
/// atom of `C` reads the observed snapshot. Over a `counters` not listed
/// `complete`, `P`, `Q` and `Cap` are `Incomplete` at admission, and so is
/// `C`'s trace, whose snapshots require the populations its `holds` atoms
/// name; over an environment admitted without that requirement the atom's
/// read itself stops `incomplete_population`/`incomplete-scope`.
#[trace("TC-1076", "FR-391-AC-7")]
#[test]
fn a_population_domain_evaluates_as_its_all_instances_spelling() {
    let mut unit = Unit::new(
        false,
        "bound B on M::counters [3]\n\
         post P using v on M::Counter::bump { forall(o in M::counters : o.v < 2) }\n\
         post PAll using v on M::Counter::bump { \
         forall(o in allInstances<M::Counter>(M::counters) : o.v < 2) }\n\
         invariant Seen using v on M::Widget at current { true }\n\
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
        let post = unit.snapshot(&format!("post{index}"), "post", counters, true);
        let current = unit.snapshot(&format!("now{index}"), "current", counters, true);
        let invocation = unit.invocation(&format!("bump{index}"), &pre, &post);
        let p = unit.holds("P", &unit.invoked("P", &invocation).expect("P"));
        let p_all = unit.holds("PAll", &unit.invoked("PAll", &invocation).expect("PAll"));
        assert_eq!(p, p_all, "{counters:?}");
        assert_eq!(p, index == 1, "{counters:?}");
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
        assert!(
            matches!(
                holds_atom(&unit, objects),
                FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(holds)))
                    if holds == (index == 1)
            ),
            "{counters:?}"
        );
    }

    // Over a `counters` that is not `complete`, each is `Incomplete`.
    let counters: &[(&str, i64)] = &[("a", 0)];
    let pre = unit.snapshot("open-pre", "pre", counters, false);
    let post = unit.snapshot("open-post", "post", counters, false);
    let current = unit.snapshot("open-now", "current", counters, false);
    let invocation = unit.invocation("open-bump", &pre, &post);
    assert_incomplete(unit.invoked("P", &invocation));
    assert_incomplete(unit.pre_call("Q", &pre, ("counters", "a")));
    assert_incomplete(unit.pre_call("GrowQ", &pre, ("widgets", "w1")));
    assert_incomplete(unit.current("Cap", &current, "a"));
    // `C`'s trace: each snapshot requires the populations its `holds` atoms
    // name, so a trace over this one is `Incomplete` too.
    let complete = unit.snapshot("trace-now", "current", counters, true);
    let [clause] = unit.graph().temporal_clauses() else {
        panic!("one temporal clause");
    };
    assert_eq!(clause.named_populations(), [ident("counters")]);
    let trace = |snapshot: &DocumentRef| {
        admit_trace_snapshot(
            unit.graph().model_selections(),
            unit.graph().scope().types(),
            &unit.packages,
            ModelNormalizationLimits::default(),
            &unit.snapshots,
            snapshot,
            ObservationLimits::default(),
            &clause.named_populations(),
        )
        .map(|_| ())
    };
    trace(&complete).expect("a complete counters admits");
    let Err(AdmissionFailure::Incomplete(record)) = trace(&current) else {
        panic!("the trace over an incomplete counters is Incomplete");
    };
    assert_eq!(
        (record.code.as_str(), record.cause),
        ("incomplete_population", "incomplete-scope")
    );
    // An environment admitted without that requirement (a clause that
    // names no population): the atom's read itself still stops.
    let seen = unit
        .admit(
            "Seen",
            ClauseSelectionInput::Current {
                snapshot: current,
                anchor: SelectedAnchor {
                    kind: qsl_semantics::model::observation::AnchorKind::Handler,
                    name: "validate".to_owned(),
                },
                self_object: Some(SelectedObject {
                    population: ident("widgets"),
                    key: "w1".to_owned(),
                }),
            },
        )
        .expect("Seen requires only widgets");
    let objects = &seen.current.as_ref().expect("current").environment;
    assert_refused_incomplete(holds_atom(&unit, objects));
}

/// The one temporal clause's `holds` atom over `objects`, with its `over`
/// bound to the watcher `w`.
fn holds_atom(
    unit: &Unit,
    objects: &qsl_semantics::model::object_environment::ObjectEnvironment,
) -> FamilyOutcome<Value> {
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
    unit.package
        .evaluate_holds(
            clause,
            atom,
            Value::Reference(watcher),
            objects,
            &mut Meter::new(UNLIMITED),
        )
        .expect("the atom evaluates")
        .outcome
}

/// The read of a named population the snapshot does not list `complete`
/// stops `incomplete_population`/`incomplete-scope` naming `counters`.
fn assert_refused_incomplete(outcome: FamilyOutcome<Value>) {
    let FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause)) = outcome else {
        panic!("an incomplete population stops the read: {outcome:?}");
    };
    let code = cause.catalog_code();
    assert_eq!(
        (code.code(), code.cause()),
        ("incomplete_population", "incomplete-scope")
    );
    assert_eq!(
        cause.catalog_fields().expect("fields")["population"],
        ident("counters")
    );
}

/// SR-12229 FND-001 (FR-391-AC-5): a `counters` listed `complete` with no
/// objects is an empty domain: `forall` over it holds, `exists` does not,
/// and its `allInstances` has size 0.
#[trace("TC-1076", "FR-391-AC-5")]
#[test]
fn a_complete_population_with_no_objects_is_an_empty_domain() {
    let mut unit = Unit::new(
        false,
        &format!(
            "{GROW}\n{OTHER}\n\
             pre Some using v on M::Widget::grow {{ exists(o in M::counters : true) }}"
        ),
    );
    let empty = unit.snapshot("empty", "pre", &[], true);
    let admitted = |name| {
        unit.pre_call(name, &empty, ("widgets", "w1"))
            .expect("admits")
    };
    assert!(unit.holds("Grow", &admitted("Grow")));
    assert!(!unit.holds("Some", &admitted("Some")));
}

/// SR-12229 FND-004 (FR-106 "required population", FR-391 "Evaluation"):
/// the population a reference field of an object of a named population
/// names is required too, so a `counters` whose `a` links into an `others`
/// not listed `complete` is `Incomplete`; with `others` complete it admits.
#[trace("TC-1076", "FR-391-AC-5")]
#[test]
fn a_named_population_requires_the_populations_its_objects_reference() {
    let mut unit = Unit::new(false, GROW);
    let mut listed = |label, others_complete| {
        unit.listed(
            label,
            "pre",
            &Listing {
                counters: &[("a", 0)],
                complete: true,
                others_complete,
                link: true,
            },
        )
    };
    let open = listed("linked-open", false);
    let closed = listed("linked-closed", true);
    assert_incomplete(unit.pre_call("Grow", &open, ("widgets", "w1")));
    let Err(AdmissionFailure::Refused(record)) = unit.pre_call("Grow", &closed, ("widgets", "w1"))
    else {
        panic!("a link to an absent object of a complete population dangles");
    };
    assert_eq!(record.code.as_str(), "dangling_reference");
}

/// FR-391 "Typing" and "Evaluation" over `lookup<T>(M::P, r)`: a present
/// reference is found; a `T` no member type of the population covers is
/// FR-084's typed incomplete result.
#[trace("TC-1076")]
#[test]
fn a_named_lookup_finds_a_member_and_an_uncovered_type_is_incomplete() {
    let mut unit = Unit::new(
        false,
        "pre Found using m on M::Counter::bump { \
         lookup<M::Counter>(M::counters, self) absent refused = self }\n\
         pre Undefined using m on M::Counter::bump { \
         lookup<M::Counter>(M::counters, self) absent undefined = self }\n\
         pre Uncovered using m on M::Widget::grow { \
         lookup<M::Widget>(M::counters, self) absent refused = self }",
    );
    let pre = unit.snapshot("pre", "pre", &[("a", 0), ("b", 1)], true);
    assert!(unit.holds(
        "Found",
        &unit
            .pre_call("Found", &pre, ("counters", "a"))
            .expect("Found")
    ));
    assert!(unit.holds(
        "Undefined",
        &unit
            .pre_call("Undefined", &pre, ("counters", "a"))
            .expect("Undefined")
    ));
    let uncovered = unit
        .pre_call("Uncovered", &pre, ("widgets", "w1"))
        .expect("Uncovered");
    let FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause)) =
        unit.evaluate("Uncovered", &uncovered)
    else {
        panic!("an undefined result");
    };
    assert_eq!(
        cause.undefined_record().reason.as_str(),
        "incomplete-closure"
    );
}

/// The `relation`/`population` nodes of an emitted package, as the node's
/// owning declaration identity.
fn population_references(unit: &Unit) -> Vec<String> {
    let emission = emit_checked(&unit.package).expect("the package emits");
    assert!(emission.omitted().is_empty(), "{:?}", emission.omitted());
    let wire: Json = serde_json::from_slice(emission.package().bytes()).expect("the wire is JSON");
    let mut owners: Vec<String> = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .filter(|node| node["node_tag"] == "relation" && node["semantic_form"] == "population")
        .map(|node| node["owner"]["node"].as_str().expect("an owner").to_owned())
        .collect();
    owners.sort();
    owners
}

/// TC-1076 row 8, the emission half (FR-391-AC-6): `A` and `B` reference two
/// population nodes, `C` the same node as `A`, so the package holds one
/// `relation`/`population` node per declared population read.
#[trace("TC-1076", "FR-391-AC-6")]
#[test]
fn a_named_population_is_emitted_once_per_declared_population_read() {
    let unit = Unit::new(
        true,
        "pre A using v on M::Widget::grow { \
         forall(o in allInstances<M::Counter>(M::counters) : true) }\n\
         pre B using v on M::Widget::grow { \
         forall(o in allInstances<M::Counter>(M::archive) : true) }\n\
         pre C using v on M::Widget::grow { \
         forall(o in allInstances<M::Counter>(M::counters) : true) }",
    );
    assert_eq!(
        population_references(&unit),
        [ident("archive"), ident("counters")]
    );
}

/// TC-1076 row 8 (FR-391-AC-6): a unit whose only population reference is
/// in a `holds` atom emits it too.
#[trace("TC-1076", "FR-391-AC-6")]
#[test]
fn a_holds_atom_naming_a_population_is_emitted() {
    let unit = Unit::new(
        false,
        "temporal C using t over (w: M::Watch) clock \"model-steps\" on origin { \
         eventually holds(forall(x in M::counters : x.v < 2)) }",
    );
    assert_eq!(population_references(&unit), [ident("counters")]);
}
