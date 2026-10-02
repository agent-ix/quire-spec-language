// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-740, TC-741, TC-743 and TC-744 (FR-265, FR-266, FR-268, FR-269): the
//! separating witness of a state clause, over a `witness` unit on FR-108's
//! `Config` domain package.
//!
//! FR-265's fixture: a state-clause body cannot name a population
//! (ADR-016 FE-4), so each invariant here ranges over a collection built
//! inside the claim from `self`'s and its parent's members. The member
//! domains are in `witness_member`.

use super::frame::object;
use super::frame_replay::{package_digest, packet, request, unit_for, witness_source, Unit};
use super::*;
use crate::execute::{separate, settle_separation, stopped_reason, SeparationOutcome};
use crate::proof_result::SettlementBasis;
use crate::result::SeparatingWitnessRecord;
use crate::witness::{derive_separating_witness, Derived};
use crate::{
    replay_state_clause, DisagreementCause, InputSettlement, ObservationIdentity, ProofCategory,
    ReplayRefusal, ReplayResult, ReplaySource, RuntimeValuePath, SeparationReason, SeparationStep,
    StateClauseCounterexample, StateClauseReplayResult, ValuePathStep, ValuePathSubject, Verdict,
    WitnessEnvelope, WitnessFailure, WitnessSettlement,
};
use qsl_eval::value::ClauseEvaluation;
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_semantics::check::{Operator, SemanticTerm};
use qsl_semantics::model::accounting::ModelNormalizationLimits;

/// The `witness` unit's invariants, on `Config::ConfigVersion` at
/// `current`. `L` is `ints(sequence[0, self.versionNumber, self.versionNumber])`:
/// over `self` `mid` (600) it is `[0, 600, 600]`, whose first element at or
/// above 500 is at index 1, with an equal element after it; over `self`
/// `child` (1) it is `[0, 1, 1]`.
const CLAUSES: &[(&str, &str)] = &[
    (
        "AllBelow",
        "forall(n in ints(sequence[0, self.versionNumber, self.versionNumber]): n < 500)",
    ),
    (
        "SomeAtLeast",
        "exists(n in ints(sequence[0, self.versionNumber, self.versionNumber]): n >= 500)",
    ),
    (
        "NotAllBelow",
        "not forall(n in ints(sequence[0, self.versionNumber, self.versionNumber]): n < 500)",
    ),
    (
        "GuardedAll",
        "present(self.parent) implies \
         forall(c in refs(sequence[value(self.parent), self]): c.versionNumber < 500)",
    ),
    (
        "EqualsAll",
        "forall(n in ints(sequence[0, self.versionNumber, self.versionNumber]): n < 500) \
         = present(self.parent)",
    ),
    (
        "LetAll",
        "let k = 500 in \
         forall(n in ints(sequence[0, self.versionNumber, self.versionNumber]): n < k)",
    ),
    (
        "OrAll",
        "forall(n in ints(sequence[0, self.versionNumber, self.versionNumber]): n < 500) \
         or self.versionNumber = 0",
    ),
    (
        "NestedAll",
        "forall(c in ints(sequence[0, self.versionNumber]): \
         forall(d in ints(sequence[0, self.versionNumber]): d <= c + 100))",
    ),
    (
        "FilteredAll",
        "forall(n in filter(m in ints(sequence[0, 1, self.versionNumber]): m > 0): n < 500)",
    ),
    (
        "MappedAll",
        "forall(n in map(m in ints(sequence[0, self.versionNumber]): m + 1): n < 500)",
    ),
    (
        "RefusedDomain",
        "forall(n in ints(sequence[0, count<Zero>(m in ints(sequence[self.versionNumber]): m > 0)]): \
         n < 500)",
    ),
    (
        "NoneSelected",
        "forall(n in filter(m in ints(sequence[0, self.versionNumber]): m > 900): n < 0)",
    ),
];

/// The `witness` unit's text, selecting the `Config` domain package.
fn witness_unit_text() -> String {
    let document = config_version_domain_document();
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let mut unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model Config = {CONFIG_VERSION_PACKAGE_IDENTITY:?} version \"1.0.0\" \
         digest \"sha256-jcs:{}\";\n",
        hex(digest)
    );
    // A collection literal takes its type from the parameter it is passed
    // to; each function returns its argument unchanged.
    unit.push_str(
        "function ints using v(xs: Sequence<Integer>[0, 3]): Sequence<Integer>[0, 3] pure { xs }\n\
         function refs using v(xs: Sequence<Config::ConfigVersion>[0, 2]): \
         Sequence<Config::ConfigVersion>[0, 2] pure { xs }\n\
         type Zero = Int[0, 0];\n",
    );
    for (name, body) in CLAUSES {
        unit.push_str(&format!(
            "invariant {name} using v on Config::ConfigVersion at current {{ {body} }}\n"
        ));
    }
    unit
}

fn witness_unit() -> Unit {
    unit_for(witness_unit_text(), config_version_domain_document())
}

/// A snapshot of the `witness` fixture: `low` holds `root` (0) and `child`
/// (1, parent `root`); `high` holds `root` (0), `mid` (600, parent `root`)
/// and `leaf` (700, parent `mid`).
#[derive(Clone, Copy, Debug)]
enum Snapshot {
    Low,
    High,
}

impl Snapshot {
    fn identity(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::High => "high",
        }
    }

    fn objects(self) -> Vec<serde_json::Value> {
        match self {
            Self::Low => vec![object("root", 0, None), object("child", 1, Some("root"))],
            Self::High => vec![
                object("root", 0, None),
                object("mid", 600, Some("root")),
                object("leaf", 700, Some("mid")),
            ],
        }
    }

    /// The snapshot's label and bytes.
    fn document(self) -> (DocumentRef, Vec<u8>) {
        document_ref_and_bytes(self.identity(), |label| {
            json!({
                "format": "quire.state.snapshot/v1",
                "identity": document_identity_json(label),
                "observation": "current",
                "anchor": {"kind": "handler", "name": "validate"},
                "model": {
                    "identity": CONFIG_VERSION_PACKAGE_IDENTITY, "version": "1.0.0",
                    "digest": format!("sha256-jcs:{}", config_version_model_digest_hex()),
                },
                "populations": [{
                    "population": config_version_population_identity(),
                    "complete": true,
                    "objects": self.objects(),
                }],
            })
            .to_string()
            .into_bytes()
        })
    }
}

/// `clause` over `snapshot`, self `self_key`: the selection and the
/// snapshot's bytes by digest.
fn selection(
    clause: &str,
    snapshot: Snapshot,
    self_key: &str,
) -> (ClauseSelection, BTreeMap<[u8; 32], Vec<u8>>) {
    let (label, bytes) = snapshot.document();
    let snapshots = BTreeMap::from([(label.digest, bytes)]);
    (
        config_version_current_selection(clause, label, self_key),
        snapshots,
    )
}

/// The `witness` unit's `ClauseRunRequest` for `clause` over `snapshot`,
/// self `self_key`, under `work_units`.
fn clause_request(
    clause: &str,
    snapshot: Snapshot,
    self_key: &str,
    work_units: u64,
) -> ClauseRunRequest {
    let (selection, snapshots) = selection(clause, snapshot, self_key);
    let document = config_version_domain_document();
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let mut request = config_version_request_for(
        witness_unit_text(),
        packages,
        "witness.native",
        ClauseRunSelection::Clause(selection),
    );
    request.snapshots = snapshots;
    request.accounting = default_accounting(work_units);
    request
}

/// `clause` run over `snapshot`, self `self_key`, through `run_clause`.
fn run(clause: &str, snapshot: Snapshot, self_key: &str) -> crate::spine::ClauseRunReport {
    run_clause(clause_request(clause, snapshot, self_key, 1_000_000))
        .expect("a well-formed request always reports")
}

/// `clause` of `unit` admitted over `snapshot`, self `self_key`, and
/// evaluated once by FR-107 under `work_units`: the observations, the
/// evaluation and the meter.
fn evaluate(
    unit: &Unit,
    clause: &str,
    snapshot: Snapshot,
    self_key: &str,
    work_units: u64,
) -> (AdmittedObservations, ClauseEvaluation, Meter) {
    let (selection, snapshots) = selection(clause, snapshot, self_key);
    let packages = qsl_semantics::model::intake::package_input([unit.domain_document.as_slice()]);
    let graph = unit.compiled.package.graph();
    let declaration = graph.state_clause(clause).expect("the clause is declared");
    let invocations = BTreeMap::new();
    let observations = admit_clause_observations(
        graph,
        declaration,
        &packages,
        ModelNormalizationLimits::default(),
        &Provisions {
            snapshots: &snapshots,
            invocations: &invocations,
        },
        &selection,
        ObservationLimits::default(),
    )
    .expect("the snapshot admits");
    let mut meter = Meter::new(default_accounting(work_units));
    let evaluation = unit
        .compiled
        .package
        .evaluate_clause(
            &QualifiedName::unqualified(clause).unwrap(),
            &observations,
            &mut meter,
        )
        .expect("the clause evaluates");
    (observations, evaluation, meter)
}

/// FR-265's derivation of `clause` over `snapshot`, self `self_key`.
fn derive(unit: &Unit, clause: &str, snapshot: Snapshot, self_key: &str) -> Option<Derived> {
    let (_, evaluation, _) = evaluate(unit, clause, snapshot, self_key, 1_000_000);
    let declaration = unit.compiled.package.graph().state_clause(clause).unwrap();
    derive_separating_witness(declaration, &evaluation).expect("the derivation completes")
}

/// The occurrence key of the node `operator` lowers to at `path` in
/// `clause`'s body (ADR-013 O-07).
fn occurrence(unit: &Unit, clause: &str, path: &[usize], operator: Operator) -> OccurrenceKey {
    let graph = unit.compiled.package.graph();
    let semantic = graph.semantic_graph();
    graph
        .occurrences()
        .find(|(key, _, location)| {
            matches!(&location.origin, quire_semantic_value::location::Origin::StateClause {
                clause: name, ..
            } if name == clause)
                && location.path == path
                && matches!(
                    semantic.node(*key).map(|node| node.body()),
                    Some(SemanticTerm::Application { operator: applied, .. }) if *applied == operator
                )
        })
        .map(|(key, origin, _)| {
            OccurrenceKey::new(WireNodeId::from_digest(*key.as_bytes()), origin)
        })
        .unwrap_or_else(|| panic!("{clause} has a {operator:?} node at {path:?}"))
}

fn observation(snapshot: Snapshot) -> ObservationIdentity {
    ObservationIdentity {
        authority: "test".to_owned(),
        identity: snapshot.identity().to_owned(),
        revision_namespace: "ns".to_owned(),
        revision: "1".to_owned(),
    }
}

fn integer(value: i64) -> Value {
    Value::Integer(quire_exact::Integer::from(value))
}

/// The record FR-265 states for `clause` over `snapshot`: the quantifier
/// at `quantifier`, the element at `index` of the collection literal at
/// `literal`, whose value path is rooted at that literal's occurrence and
/// ends in an index step for `index` (FR-265-AC-5), and no trace position.
fn built_record(
    unit: &Unit,
    clause: &str,
    quantifier: &[usize],
    literal: &[usize],
    element: Value,
    index: u64,
    snapshot: Snapshot,
) -> SeparatingWitnessRecord {
    SeparatingWitnessRecord {
        quantifier: occurrence(unit, clause, quantifier, Operator::Quantify),
        deciding_element: element,
        index: Some(index),
        value_path: RuntimeValuePath {
            observation: observation(snapshot),
            subject: ValuePathSubject::Built(occurrence(
                unit,
                clause,
                literal,
                Operator::Collection,
            )),
            steps: vec![ValuePathStep::Index(index)],
        },
        trace_position: None,
    }
}

/// `AllBelow`'s record over `high`: `forall` at the root, element 600 at
/// index 1 of the literal inside `ints(..)`.
fn all_below_record(unit: &Unit) -> SeparatingWitnessRecord {
    built_record(
        unit,
        "AllBelow",
        &[],
        &[0, 0],
        integer(600),
        1,
        Snapshot::High,
    )
}

fn decisive(basis: SettlementBasis, record: SeparatingWitnessRecord) -> Option<Derived> {
    Some(Derived {
        basis,
        record: Some(record),
    })
}

fn closed() -> Option<Derived> {
    Some(Derived {
        basis: SettlementBasis::ClosedScope,
        record: None,
    })
}

/// TC-740 step 1 (FR-265-AC-1): `AllBelow` over `high` derives
/// `decisive-counterexample` and the record naming the `forall`, 600 at
/// index 1 and the built literal; the equal element after the stop is never
/// visited (two `collection.visit` charges); over `low` it derives
/// `closed-scope` and no record.
#[trace("TC-740", "FR-265-AC-1")]
#[test]
fn tc_740_a_refuted_forall_derives_its_record_and_visits_nothing_after_the_stop() {
    let unit = witness_unit();
    assert_eq!(
        derive(&unit, "AllBelow", Snapshot::High, "mid"),
        decisive(
            SettlementBasis::DecisiveCounterexample,
            all_below_record(&unit)
        )
    );
    let (_, _, meter) = evaluate(&unit, "AllBelow", Snapshot::High, "mid", 1_000_000);
    let visits = meter
        .admitted_charges()
        .iter()
        .filter(|point| **point == ChargePoint::CollectionVisit)
        .count();
    assert_eq!(visits, 2, "elements 0 and 1 are visited, element 2 is not");
    assert_eq!(derive(&unit, "AllBelow", Snapshot::Low, "child"), closed());
}

/// TC-740 step 2 (FR-265-AC-2): `exists` satisfied and `not forall` true
/// derive `decisive-witness` naming their quantifier; `exists` false
/// derives `closed-scope`; a stopped `forall` under `=` derives
/// `closed-scope`.
#[trace("TC-740", "FR-265-AC-2")]
#[test]
fn tc_740_a_satisfied_exists_or_negated_forall_derives_a_decisive_witness() {
    let unit = witness_unit();
    assert_eq!(
        derive(&unit, "SomeAtLeast", Snapshot::High, "mid"),
        decisive(
            SettlementBasis::DecisiveWitness,
            built_record(
                &unit,
                "SomeAtLeast",
                &[],
                &[0, 0],
                integer(600),
                1,
                Snapshot::High
            )
        )
    );
    assert_eq!(
        derive(&unit, "NotAllBelow", Snapshot::High, "mid"),
        decisive(
            SettlementBasis::DecisiveWitness,
            built_record(
                &unit,
                "NotAllBelow",
                &[0],
                &[0, 0, 0],
                integer(600),
                1,
                Snapshot::High
            )
        )
    );
    assert_eq!(
        derive(&unit, "SomeAtLeast", Snapshot::Low, "child"),
        closed()
    );
    let (_, evaluation, _) = evaluate(&unit, "EqualsAll", Snapshot::High, "mid", 1_000_000);
    let forall = unit
        .compiled
        .package
        .graph()
        .state_clause("EqualsAll")
        .unwrap();
    let qsl_semantics::check::NodeKind::Equality(_, _, left, _) = forall.body().kind() else {
        panic!("EqualsAll's body is an equality");
    };
    assert!(
        evaluation.stop(left.location()).is_some(),
        "its forall stopped"
    );
    assert_eq!(derive(&unit, "EqualsAll", Snapshot::High, "mid"), closed());
}

/// TC-740 step 3 (FR-265-AC-3): the decision path through `implies`, `let`
/// and `or`.
#[trace("TC-740", "FR-265-AC-3")]
#[test]
fn tc_740_the_decision_path_reaches_the_decisive_quantifier() {
    let unit = witness_unit();
    let Some(Derived {
        basis: SettlementBasis::DecisiveCounterexample,
        record: Some(guarded),
    }) = derive(&unit, "GuardedAll", Snapshot::High, "mid")
    else {
        panic!("GuardedAll over high with a parent is decisive");
    };
    let (observations, _, _) = evaluate(&unit, "GuardedAll", Snapshot::High, "mid", 1_000_000);
    assert_eq!(
        guarded,
        built_record(
            &unit,
            "GuardedAll",
            &[1],
            &[1, 0, 0],
            Value::Reference(observations.self_object.clone()),
            1,
            Snapshot::High
        )
    );
    assert_eq!(
        derive(&unit, "GuardedAll", Snapshot::High, "root"),
        closed()
    );
    assert_eq!(
        derive(&unit, "LetAll", Snapshot::High, "mid"),
        decisive(
            SettlementBasis::DecisiveCounterexample,
            built_record(
                &unit,
                "LetAll",
                &[1],
                &[1, 0, 0],
                integer(600),
                1,
                Snapshot::High
            )
        )
    );
    assert_eq!(derive(&unit, "OrAll", Snapshot::High, "mid"), closed());
}

/// TC-740 step 4 (FR-265-AC-4): the record names the outer `forall`, its
/// element 0 at index 0, and no inner occurrence.
#[trace("TC-740", "FR-265-AC-4")]
#[test]
fn tc_740_a_nested_forall_records_the_outer_occurrence() {
    let unit = witness_unit();
    assert_eq!(
        derive(&unit, "NestedAll", Snapshot::High, "mid"),
        decisive(
            SettlementBasis::DecisiveCounterexample,
            built_record(
                &unit,
                "NestedAll",
                &[],
                &[0, 0],
                integer(0),
                0,
                Snapshot::High
            )
        )
    );
}

/// TC-740 step 5 (FR-265-AC-5): a `filter` domain records the source
/// position (2, not the filtered position 1) and a `map` domain the mapped
/// value, each on the built literal's path.
#[trace("TC-740", "FR-265-AC-5")]
#[test]
fn tc_740_computed_domains_record_the_source_position() {
    let unit = witness_unit();
    assert_eq!(
        derive(&unit, "FilteredAll", Snapshot::High, "mid"),
        decisive(
            SettlementBasis::DecisiveCounterexample,
            built_record(
                &unit,
                "FilteredAll",
                &[],
                &[0, 0, 0],
                integer(600),
                2,
                Snapshot::High
            )
        )
    );
    assert_eq!(
        derive(&unit, "MappedAll", Snapshot::High, "mid"),
        decisive(
            SettlementBasis::DecisiveCounterexample,
            built_record(
                &unit,
                "MappedAll",
                &[],
                &[0, 0, 0],
                integer(601),
                1,
                Snapshot::High
            )
        )
    );
}

/// TC-740 step 6 (FR-265-AC-6): an incomplete evaluation derives no basis;
/// an empty filtered domain derives `closed-scope`.
#[trace("TC-740", "FR-265-AC-6")]
#[test]
fn tc_740_no_boolean_derives_no_basis() {
    let unit = witness_unit();
    let (_, evaluation, _) = evaluate(&unit, "AllBelow", Snapshot::High, "mid", 0);
    assert!(matches!(
        evaluation.evaluation.outcome,
        FamilyOutcome::Evaluated(Outcome::Incomplete(_))
    ));
    let declaration = unit
        .compiled
        .package
        .graph()
        .state_clause("AllBelow")
        .unwrap();
    assert_eq!(
        derive_separating_witness(declaration, &evaluation).unwrap(),
        None
    );
    assert_eq!(
        derive(&unit, "NoneSelected", Snapshot::Low, "child"),
        closed()
    );
}

/// TC-741 step 1 (FR-266-AC-1): `run_clause` reports carry FR-265's basis
/// and record, and two runs give equal ones.
#[trace("TC-741", "FR-266-AC-1")]
#[test]
fn tc_741_clause_reports_carry_the_derived_basis_and_witness() {
    let unit = witness_unit();
    let refuted = run("AllBelow", Snapshot::High, "mid");
    assert_eq!(
        refuted.disposition.category(),
        qsl_foundation::diagnostic::Category::Violation
    );
    assert_eq!(refuted.disposition.truth(), Some(false));
    assert_eq!(refuted.exit_code(), 10);
    assert_eq!(refuted.basis, SettlementBasis::DecisiveCounterexample);
    assert_eq!(refuted.witness, Some(all_below_record(&unit)));

    let satisfied = run("SomeAtLeast", Snapshot::High, "mid");
    assert_eq!(satisfied.exit_code(), 0);
    assert_eq!(satisfied.basis, SettlementBasis::DecisiveWitness);
    assert_eq!(
        satisfied.witness,
        Some(built_record(
            &unit,
            "SomeAtLeast",
            &[],
            &[0, 0],
            integer(600),
            1,
            Snapshot::High
        ))
    );

    let holds = run("AllBelow", Snapshot::Low, "child");
    assert_eq!(
        holds.disposition.category(),
        qsl_foundation::diagnostic::Category::Success
    );
    assert_eq!(holds.basis, SettlementBasis::ClosedScope);
    assert_eq!(holds.witness, None);

    let again = run("AllBelow", Snapshot::High, "mid");
    assert_eq!(
        (again.basis, &again.witness, again.exit_code()),
        (refuted.basis, &refuted.witness, refuted.exit_code())
    );
}

/// A current snapshot of `config_history` holding `objects`, `complete`
/// as given, and its `ParentOrder` request with self `child`.
fn parent_order_request(
    identity: &str,
    objects: Vec<serde_json::Value>,
    complete: bool,
) -> ClauseRunRequest {
    let (label, bytes) = document_ref_and_bytes(identity, |label| {
        json!({
            "format": "quire.state.snapshot/v1",
            "identity": document_identity_json(label),
            "observation": "current",
            "anchor": {"kind": "handler", "name": "validate"},
            "model": {
                "identity": CONFIG_VERSION_PACKAGE_IDENTITY, "version": "1.0.0",
                "digest": format!("sha256-jcs:{}", config_version_model_digest_hex()),
            },
            "populations": [{
                "population": config_version_population_identity(),
                "complete": complete,
                "objects": objects,
            }],
        })
        .to_string()
        .into_bytes()
    });
    let selection = config_version_current_selection("ParentOrder", label.clone(), "child");
    let mut request = config_version_request(ClauseRunSelection::Clause(selection));
    request.snapshots.insert(label.digest, bytes);
    request
}

fn basis_of(request: ClauseRunRequest) -> (SettlementBasis, bool) {
    let report = run_clause(request).expect("a well-formed request always reports");
    (report.basis, report.witness.is_some())
}

/// TC-741 step 2 (FR-266-AC-2): FR-108's corpus requests carry
/// `closed-scope` when evaluated and `unavailable` otherwise, never a
/// witness; a `Function` selection carries `closed-scope` for its
/// violation and `unavailable` for an admission refusal.
#[trace("TC-741", "FR-266-AC-2")]
#[test]
fn tc_741_corpus_and_function_reports_carry_their_basis() {
    let healthy = || vec![object("root", 1, None), object("child", 2, Some("root"))];
    let closed = (SettlementBasis::ClosedScope, false);
    let unavailable = (SettlementBasis::Unavailable, false);
    assert_eq!(
        basis_of(parent_order_request("healthy-parent", healthy(), true)),
        closed
    );
    assert_eq!(
        basis_of(parent_order_request(
            "violating-parent",
            vec![object("root", 3, None), object("child", 2, Some("root"))],
            true
        )),
        closed
    );
    let changed = run_config_version_invocation_report(
        config_version_request,
        "VersionUnchanged",
        InvocationVersions::root_unchanged(1, 2, 3),
        "child",
    );
    assert_eq!(changed.disposition.truth(), Some(false));
    assert_eq!((changed.basis, changed.witness.is_some()), closed);

    let mut missing_model = parent_order_request("healthy-parent", healthy(), true);
    missing_model.packages.clear();
    assert_eq!(basis_of(missing_model), unavailable);
    assert_eq!(
        basis_of(parent_order_request(
            "dangling-parent",
            vec![object("child", 2, Some("ghost"))],
            true
        )),
        unavailable
    );
    assert_eq!(
        basis_of(parent_order_request(
            "incomplete-population",
            healthy(),
            false
        )),
        unavailable
    );
    let mut exhausted = parent_order_request("healthy-parent", healthy(), true);
    exhausted.accounting = default_accounting(0);
    assert_eq!(basis_of(exhausted), unavailable);

    let same_identity = |b: &str| {
        config_version_function_request(
            "sameIdentity",
            vec![
                ClauseArgument {
                    parameter: "a".to_owned(),
                    value: config_version_reference("root"),
                },
                ClauseArgument {
                    parameter: "b".to_owned(),
                    value: config_version_reference(b),
                },
            ],
        )
    };
    let violation = run_clause(same_identity("child")).unwrap();
    assert_eq!(
        violation.disposition.category(),
        qsl_foundation::diagnostic::Category::Violation
    );
    assert_eq!((violation.basis, violation.witness.is_some()), closed);
    let ghost = run_clause(same_identity("ghost")).unwrap();
    assert_eq!(ghost.disposition.stage(), ClauseRunStage::Admit);
    assert_eq!((ghost.basis, ghost.witness.is_some()), unavailable);
}

/// TC-741 step 3 (FR-266-AC-3): a `Frame` violation carries `closed-scope`
/// and its FR-115 witness unchanged; an `evaluate` internal failure carries
/// `unavailable`.
#[trace("TC-741", "FR-266-AC-3")]
#[test]
fn tc_741_frame_and_internal_failure_reports_carry_their_basis() {
    let input = super::frame::frame_input(
        config_version_request,
        "attemptUpdate",
        &[object("root", 1, None), object("child", 2, Some("root"))],
        &[object("root", 1, None), object("child", 2, None)],
        |_| {},
    );
    let report = super::frame::run(input);
    let ClauseDisposition::FrameViolation(witness) = &report.disposition else {
        panic!("expected FrameViolation, got {:?}", report.disposition);
    };
    assert_eq!(witness.cause, "unauthorized-change");
    assert_eq!(report.basis, SettlementBasis::ClosedScope);
    assert_eq!(report.witness, None);
    assert_eq!(
        super::super::undecided_basis(&ClauseDisposition::EvaluateFault(
            qsl_foundation::diagnostic::InternalFault::new("S6a", "checked-program-invariant")
        )),
        SettlementBasis::Unavailable
    );
}

/// The `witness` unit's FR-122 envelope for `clause` over `snapshot`, self
/// `self_key`, carrying `witness`, on `source`'s arm, and its request.
fn replay_case(
    unit: &Unit,
    clause: &str,
    snapshot: Snapshot,
    self_key: &str,
    witness: Option<SeparatingWitnessRecord>,
    source: ReplaySource,
) -> (
    crate::ReplayRequestWire,
    Result<WitnessEnvelope<StateClauseCounterexample>, crate::WitnessRefusal>,
) {
    let (label, bytes) = snapshot.document();
    let package_id = package_digest(unit.compiled.emitted.package_id());
    let wire = request(
        &unit.bytes,
        &unit.domain_document,
        package_id,
        &[(label.clone(), bytes)],
    );
    let declaration = unit.compiled.package.graph().state_clause(clause).unwrap();
    let node = WireNodeId::from_digest(*declaration.identity().as_bytes());
    let payload = StateClauseCounterexample {
        clause: quire_exact::Identifier::new(clause).unwrap(),
        observation: config_version_current_selection(clause, label, self_key).input,
        witness,
    };
    let envelope = WitnessEnvelope::reconstruct(packet(
        &unit.bytes,
        package_id,
        source,
        node,
        OccurrenceKey::new(node, declaration.claim().clone()),
        payload,
    ));
    (wire, envelope)
}

fn replay_with(
    unit: &Unit,
    clause: &str,
    witness: Option<SeparatingWitnessRecord>,
    source: ReplaySource,
) -> Result<StateClauseReplayResult, ReplayRefusal> {
    let (wire, envelope) = replay_case(unit, clause, Snapshot::High, "mid", witness, source);
    replay_state_clause(wire, &envelope.expect("the envelope reconstructs"))
}

fn input_source() -> ReplaySource {
    ReplaySource::Input(Vec::new())
}

fn witness_arm(result: &StateClauseReplayResult) -> &crate::WitnessArmResult {
    match result.result() {
        ReplayResult::Witness(arm) => arm,
        ReplayResult::Input(arm) => panic!("expected the witness arm, got {arm:?}"),
    }
}

/// TC-743 step 1 (FR-268-AC-1): an envelope carrying the producer's record
/// reproduces with that record on the `Witness` arm and without witness on
/// the `Input` arm; nested and filtered domains reproduce too.
#[trace("TC-743", "FR-268-AC-1")]
#[test]
fn tc_743_a_matching_record_that_separates_reproduces() {
    let unit = witness_unit();
    let record = all_below_record(&unit);
    let result = replay_with(&unit, "AllBelow", Some(record.clone()), witness_source()).unwrap();
    let arm = witness_arm(&result);
    assert_eq!(
        arm.settlement(),
        WitnessSettlement::ReproducedWithEvaluatedWitness
    );
    assert_eq!(arm.record(), Some(&record));

    let ReplayResult::Input(input) = replay_with(&unit, "AllBelow", Some(record), input_source())
        .unwrap()
        .result()
        .clone()
    else {
        panic!("an Input-sourced envelope settles on the Input arm");
    };
    assert_eq!(
        input.settlement(),
        InputSettlement::ReproducedWithoutWitness
    );

    for (clause, record) in [
        (
            "NestedAll",
            built_record(
                &unit,
                "NestedAll",
                &[],
                &[0, 0],
                integer(0),
                0,
                Snapshot::High,
            ),
        ),
        (
            "FilteredAll",
            built_record(
                &unit,
                "FilteredAll",
                &[],
                &[0, 0, 0],
                integer(600),
                2,
                Snapshot::High,
            ),
        ),
    ] {
        let result = replay_with(&unit, clause, Some(record.clone()), witness_source()).unwrap();
        let arm = witness_arm(&result);
        assert_eq!(
            arm.settlement(),
            WitnessSettlement::ReproducedWithEvaluatedWitness,
            "{clause}"
        );
        assert_eq!(arm.record(), Some(&record), "{clause}");
    }
}

/// `AllBelow`'s record with the element at index 2 (the equal element after
/// the stop), standing for FR-268-AC-2's `leaf` record.
fn after_stop_record(unit: &Unit) -> SeparatingWitnessRecord {
    built_record(
        unit,
        "AllBelow",
        &[],
        &[0, 0],
        integer(600),
        2,
        Snapshot::High,
    )
}

/// TC-743 step 2 (FR-268-AC-2): a record naming the element after the stop,
/// one naming another clause's `forall`, and no record each settle
/// `inconclusive` with a `Witness` cause holding both records.
#[trace("TC-743", "FR-268-AC-2")]
#[test]
fn tc_743_a_differing_or_missing_record_settles_inconclusive() {
    let unit = witness_unit();
    let derived = all_below_record(&unit);
    let other_quantifier = SeparatingWitnessRecord {
        quantifier: occurrence(&unit, "GuardedAll", &[1], Operator::Quantify),
        ..derived.clone()
    };
    for given in [Some(after_stop_record(&unit)), Some(other_quantifier), None] {
        let result = replay_with(&unit, "AllBelow", given.clone(), witness_source()).unwrap();
        let arm = witness_arm(&result);
        assert_eq!(arm.settlement(), WitnessSettlement::Inconclusive);
        let Some(DisagreementCause::Witness {
            given: held,
            derived: re_derived,
            ..
        }) = arm.disagreement()
        else {
            panic!("expected a Witness cause, got {:?}", arm.disagreement());
        };
        assert_eq!(held.as_deref(), given.as_ref());
        assert_eq!(re_derived.as_deref(), Some(&derived));
    }
}

/// FR-268's separation check of `record` over `clause` and the admitted
/// `high` observation (self `mid`), under `work_units`.
fn check(
    unit: &Unit,
    clause: &str,
    record: &SeparatingWitnessRecord,
    work_units: u64,
) -> SeparationOutcome {
    let (observations, _, _) = evaluate(unit, clause, Snapshot::High, "mid", 1_000_000);
    let mut meter = Meter::new(default_accounting(work_units));
    separate(
        &unit.compiled.package,
        clause,
        &observations,
        record,
        std::slice::from_ref(&unit.compiled.source),
        &mut meter,
    )
    .expect("the check settles")
}

fn unmet(step: SeparationStep) -> SeparationOutcome {
    SeparationOutcome::Failed(step, SeparationReason::Unmet)
}

/// TC-743 step 3 and TC-744 step 2 (FR-268-AC-3, FR-269-AC-2): the
/// separation check, called on its own.
#[trace("TC-743", "FR-268-AC-3")]
#[test]
fn tc_743_the_separation_check_names_its_failing_step() {
    let unit = witness_unit();
    let record = all_below_record(&unit);
    assert_eq!(
        check(&unit, "AllBelow", &record, 1_000_000),
        SeparationOutcome::Holds
    );
    let outside = SeparatingWitnessRecord {
        quantifier: occurrence(&unit, "SomeAtLeast", &[], Operator::Quantify),
        ..record.clone()
    };
    let past_end = SeparatingWitnessRecord {
        index: Some(3),
        value_path: RuntimeValuePath {
            steps: vec![ValuePathStep::Index(3)],
            ..record.value_path.clone()
        },
        ..record.clone()
    };
    let other_member = SeparatingWitnessRecord {
        value_path: RuntimeValuePath {
            steps: vec![
                ValuePathStep::Member("versionNumber".to_owned()),
                ValuePathStep::Index(1),
            ],
            ..record.value_path.clone()
        },
        ..record.clone()
    };
    let other_element = SeparatingWitnessRecord {
        deciding_element: integer(700),
        ..record.clone()
    };
    let holding_element = built_record(
        &unit,
        "AllBelow",
        &[],
        &[0, 0],
        integer(0),
        0,
        Snapshot::High,
    );
    for (failing, step) in [
        (&outside, SeparationStep::Quantifier),
        (&past_end, SeparationStep::Element),
        (&other_member, SeparationStep::Element),
        (&other_element, SeparationStep::Element),
        (&holding_element, SeparationStep::Body),
    ] {
        assert_eq!(check(&unit, "AllBelow", failing, 1_000_000), unmet(step));
    }

    // A domain refused on `high`: `count<Zero>` of one element.
    let refused = SeparatingWitnessRecord {
        quantifier: occurrence(&unit, "RefusedDomain", &[], Operator::Quantify),
        ..record.clone()
    };
    let SeparationOutcome::Failed(SeparationStep::Domain, SeparationReason::Refused(refusal)) =
        check(&unit, "RefusedDomain", &refused, 1_000_000)
    else {
        panic!("a refused domain fails step 2 with its refusal record");
    };
    assert_eq!(refusal.code, "integer_out_of_domain");

    assert_eq!(
        check(&unit, "AllBelow", &record, 0),
        SeparationOutcome::Exhausted
    );
}

/// TC-744 step 2 (FR-269-AC-2): an `undefined` step evaluation maps to
/// `UndefinedEvaluation`, naming the expression and its undefined reason.
/// A linked state clause proves every divisor nonzero, so no clause
/// reaches an `undefined` step; the mapping is checked over the S6a
/// evaluation such a step would return.
#[trace("TC-744", "FR-269-AC-2")]
#[test]
fn tc_744_an_undefined_step_maps_to_undefined_evaluation() {
    let unit = witness_unit();
    let at = quire_semantic_value::location::Location {
        origin: quire_semantic_value::location::Origin::StateClause {
            clause: "AllBelow".to_owned(),
            index: 0,
        },
        path: vec![0],
    };
    let evaluation = qsl_eval::value::Evaluation {
        outcome: FamilyOutcome::Evaluated(Outcome::Undefined(
            quire_exact::Undefined::DivisionByZero,
        )),
        location: Some(at.clone()),
        losses: Vec::new(),
    };
    let reason = stopped_reason(
        evaluation,
        unit.compiled.package.graph(),
        std::slice::from_ref(&unit.compiled.source),
    )
    .expect("the mapping settles");
    let Some(SeparationReason::UndefinedEvaluation { expression, cause }) = reason else {
        panic!("expected UndefinedEvaluation, got {reason:?}");
    };
    assert_eq!(expression, at);
    assert_eq!(cause, "division-by-zero");
}

/// TC-743 step 4 (FR-268-AC-4): an encoded record past the reader bound
/// refuses at decode; replaying one envelope twice gives equal results.
#[trace("TC-743", "FR-268-AC-4")]
#[test]
fn tc_743_an_oversized_record_refuses_and_replay_is_deterministic() {
    let unit = witness_unit();
    let mut oversized = all_below_record(&unit);
    oversized.value_path.steps.insert(
        0,
        ValuePathStep::Member("x".repeat(crate::MAX_ENCODED_BYTES + 1)),
    );
    let (_, envelope) = replay_case(
        &unit,
        "AllBelow",
        Snapshot::High,
        "mid",
        Some(oversized),
        witness_source(),
    );
    assert!(matches!(
        envelope,
        Err(crate::WitnessRefusal::BoundExceeded(_))
    ));
    let record = all_below_record(&unit);
    let first = replay_with(&unit, "AllBelow", Some(record.clone()), witness_source()).unwrap();
    let second = replay_with(&unit, "AllBelow", Some(record), witness_source()).unwrap();
    assert_eq!(first, second);
}

/// TC-744 step 1 (FR-269-AC-1): the after-stop and no-record replays hold
/// `Witness { violation, violation, given, derived, Mismatch }`, and the
/// result carries no record.
#[trace("TC-744", "FR-269-AC-1")]
#[test]
fn tc_744_a_mismatch_holds_both_records_and_no_result_record() {
    let unit = witness_unit();
    let violation = Verdict::from_category(ProofCategory::Violation);
    let derived = all_below_record(&unit);
    for given in [Some(after_stop_record(&unit)), None] {
        let result = replay_with(&unit, "AllBelow", given.clone(), witness_source()).unwrap();
        let arm = witness_arm(&result);
        assert_eq!(
            arm.disagreement(),
            Some(&DisagreementCause::Witness {
                proved: violation,
                replayed: violation,
                given: given.map(Box::new),
                derived: Some(Box::new(derived.clone())),
                failure: WitnessFailure::Mismatch,
            })
        );
        assert_eq!(arm.record(), None);
    }
}

/// TC-744 step 3 (FR-269-AC-3): a `Witness` cause round-trips; an
/// undefined `failure` and a `given` missing its `index` refuse; causes
/// differing only in `derived` compare unequal.
#[trace("TC-744", "FR-269-AC-3")]
#[test]
fn tc_744_the_witness_cause_round_trips_and_reads_strictly() {
    let unit = witness_unit();
    let result = replay_with(
        &unit,
        "AllBelow",
        Some(after_stop_record(&unit)),
        witness_source(),
    )
    .unwrap();
    let cause = witness_arm(&result).disagreement().unwrap().clone();
    let text = cause.to_json().expect("the cause encodes");
    assert_eq!(
        DisagreementCause::from_json(&text, &unit.compiled.package).unwrap(),
        cause
    );

    let mut document: serde_json::Value = serde_json::from_str(&text).unwrap();
    document["failure"] = json!({"failure": "approximate"});
    assert!(DisagreementCause::from_json(&document.to_string(), &unit.compiled.package).is_err());

    let mut document: serde_json::Value = serde_json::from_str(&text).unwrap();
    document["given"]
        .as_object_mut()
        .unwrap()
        .remove("index")
        .expect("the record carries its index");
    assert!(DisagreementCause::from_json(&document.to_string(), &unit.compiled.package).is_err());

    let mut document: serde_json::Value = serde_json::from_str(&text).unwrap();
    document["given"]["index"] = serde_json::Value::Null;
    assert!(DisagreementCause::from_json(&document.to_string(), &unit.compiled.package).is_err());

    let DisagreementCause::Witness {
        proved,
        replayed,
        given,
        failure,
        ..
    } = cause.clone()
    else {
        panic!("a Witness cause");
    };
    let other = DisagreementCause::Witness {
        proved,
        replayed,
        given,
        derived: Some(Box::new(built_record(
            &unit,
            "AllBelow",
            &[],
            &[0, 0],
            integer(0),
            0,
            Snapshot::High,
        ))),
        failure,
    };
    assert_ne!(cause, other);
}

/// TC-744 step 2 (FR-269-AC-2): a matching record whose separation check
/// fails at a step disagrees with `Witness`'s `Separation` failure naming
/// that step and reason, holding the record as both `given` and `derived`;
/// one that holds agrees; an exhausted check settles no witness check.
#[trace("TC-744", "FR-269-AC-2")]
#[test]
fn tc_744_a_failed_separation_check_disagrees_naming_its_step() {
    let unit = witness_unit();
    let record = all_below_record(&unit);
    assert_eq!(
        settle_separation(unmet(SeparationStep::Body), &record),
        Some(crate::WitnessCheck::Disagrees {
            given: Some(Box::new(record.clone())),
            derived: Some(Box::new(record.clone())),
            failure: WitnessFailure::Separation {
                step: SeparationStep::Body,
                reason: SeparationReason::Unmet,
            },
        })
    );
    assert_eq!(
        settle_separation(SeparationOutcome::Holds, &record),
        Some(crate::WitnessCheck::Agrees(Some(Box::new(record.clone()))))
    );
    assert_eq!(
        settle_separation(SeparationOutcome::Exhausted, &record),
        None
    );
}

/// The `witness` unit with a record declaration, so a composite deciding
/// element names a declaration the checked package holds.
fn codec_unit() -> Unit {
    let text = format!(
        "{}record Point {{ x: Int[0, 9]; y: Int[0, 9]; }}\n",
        witness_unit_text()
    );
    unit_for(text, config_version_domain_document())
}

/// A text value of `Text[0, 64; nfc]` holding `payload`.
fn text(payload: &str) -> Value {
    let text_type = quire_exact::TextType::new(0, 64, quire_exact::TextProfile::Nfc).unwrap();
    let payload = quire_exact::TextPayload::from_utf8(payload.as_bytes()).unwrap();
    let mut meter = Meter::new(default_accounting(1_000_000));
    match quire_exact::admit_text(&payload, &text_type, &mut meter) {
        Outcome::Completed(text) => Value::Text(text),
        other => panic!("the text admits: {other:?}"),
    }
}

/// A `Witness` cause whose `given` record decides on `element`.
fn cause_deciding(unit: &Unit, element: Value) -> DisagreementCause {
    let violation = Verdict::from_category(ProofCategory::Violation);
    DisagreementCause::Witness {
        proved: violation,
        replayed: violation,
        given: Some(Box::new(SeparatingWitnessRecord {
            deciding_element: element,
            ..all_below_record(unit)
        })),
        derived: None,
        failure: WitnessFailure::Mismatch,
    }
}

/// TC-744 step 3 (FR-269-AC-3): a deciding element of each value kind
/// round-trips through the cause codec, reading back equal and encoding
/// back to the same document; an element naming a declaration the package
/// does not hold refuses.
#[trace("TC-744", "FR-269-AC-3")]
#[test]
fn tc_744_every_deciding_element_kind_round_trips() {
    use quire_exact::{
        CardinalityBound, CollectionKind, CollectionType, Decimal, FieldValue, IeeeValue,
        IntegerInterval, ObjectId, ObjectReference, OptionValue, Quantity, Rational, UnitId,
        UniverseId, ValueType, VariantId,
    };
    let unit = codec_unit();
    let package = &unit.compiled.package;
    let types = package.graph().scope().types();
    let point = types.composites().next().expect("Point is declared").key();
    let object_type = types
        .object_types()
        .next()
        .expect("ConfigVersion is declared")
        .key();
    let int = |value: i64| Value::Integer(quire_exact::Integer::from(value));
    let digit = ValueType::Int(
        IntegerInterval::new(
            quire_exact::Integer::from(0_i64),
            quire_exact::Integer::from(9_i64),
        )
        .unwrap(),
    );
    let rational = |n: i64, d: i64| {
        Rational::new(quire_exact::Integer::from(n), quire_exact::Integer::from(d)).unwrap()
    };
    let sequence = CollectionType::new(
        CollectionKind::Sequence,
        ValueType::Integer,
        Some(CardinalityBound::new(0, 3).unwrap()),
    );
    let set = CollectionType::new(CollectionKind::Set, ValueType::Integer, None);
    let references = CollectionType::new(
        CollectionKind::Sequence,
        ValueType::Reference(object_type),
        None,
    );
    let elements = [
        ("boolean", Value::Boolean(true)),
        ("integer", int(-42)),
        ("rational", Value::Rational(rational(-3, 4))),
        (
            "decimal",
            Value::Decimal(Decimal::new(quire_exact::Integer::from(12_345_i64), 2)),
        ),
        (
            "float64",
            Value::Float(IeeeValue::binary64(0x4009_21fb_5444_2d18)),
        ),
        ("float32", Value::Float(IeeeValue::binary32(0x4049_0fdb))),
        (
            "compound quantity",
            Value::Quantity(Quantity::new(rational(1, 1), UnitId::compound([5; 32]))),
        ),
        ("text", text("cafe\u{301}")),
        (
            "enum",
            Value::Enum(quire_exact::EnumMember::new(
                VariantId::from_digest([9; 32]),
                1,
            )),
        ),
        (
            "present option",
            OptionValue::present(digit.clone(), int(7)).unwrap(),
        ),
        (
            "absent option",
            OptionValue::none(ValueType::Composite(point)),
        ),
        (
            "composite",
            quire_exact::from_admitted_slots(
                point,
                vec![FieldValue::Present(int(1)), FieldValue::Present(int(2))].into(),
            ),
        ),
        (
            "sequence",
            quire_exact::from_admitted(sequence, vec![int(2), int(1), int(2)]),
        ),
        ("set", quire_exact::from_admitted(set, vec![int(1), int(2)])),
        (
            "empty reference sequence",
            quire_exact::from_admitted(references, Vec::new()),
        ),
        (
            "reference",
            Value::Reference(ObjectReference::new(
                UniverseId::from_digest([2; 32]),
                object_type,
                ObjectId::new("root".to_owned()).unwrap(),
            )),
        ),
    ];
    for (kind, element) in elements {
        let cause = cause_deciding(&unit, element);
        let text = cause
            .to_json()
            .unwrap_or_else(|error| panic!("{kind}: {error}"));
        let read = DisagreementCause::from_json(&text, package)
            .unwrap_or_else(|error| panic!("{kind}: {error}"));
        assert_eq!(read, cause, "{kind}");
        assert_eq!(read.to_json().unwrap(), text, "{kind}");
    }

    // A declared unit the package's unit table does not hold refuses.
    let declared = cause_deciding(
        &unit,
        Value::Quantity(Quantity::new(
            rational(5, 2),
            UnitId::declared(quire_exact::NodeKey::from_digest([6; 32])),
        )),
    )
    .to_json()
    .unwrap();
    assert!(matches!(
        DisagreementCause::from_json(&declared, package),
        Err(crate::CauseCodecError::Unresolved("unit"))
    ));

    // Slots of another shape than `Point`'s refuse (QSpec FR-351-AC-5):
    // absent and null for its two required fields, one slot too few, and a
    // slot outside its field's type.
    for slots in [
        vec![FieldValue::Absent, FieldValue::Null],
        vec![FieldValue::Present(int(1))],
        vec![FieldValue::Present(int(1)), FieldValue::Present(int(10))],
    ] {
        let foreign = cause_deciding(&unit, quire_exact::from_admitted_slots(point, slots.into()))
            .to_json()
            .unwrap();
        assert!(matches!(
            DisagreementCause::from_json(&foreign, package),
            Err(crate::CauseCodecError::Value("slots"))
        ));
    }

    // The witness unit declares no `Point`: a well-shaped one refuses there.
    let composite = cause_deciding(
        &unit,
        quire_exact::from_admitted_slots(
            point,
            vec![FieldValue::Present(int(1)), FieldValue::Present(int(2))].into(),
        ),
    )
    .to_json()
    .unwrap();
    assert!(DisagreementCause::from_json(&composite, package).is_ok());
    assert!(matches!(
        DisagreementCause::from_json(&composite, &witness_unit().compiled.package),
        Err(crate::CauseCodecError::Unresolved("declaration"))
    ));
}

/// A checked package whose unit table holds the declared unit `metre` of
/// dimension `Length`, both keyed from their QSpec FR-142 preimages, and
/// that unit's id.
fn metre_package() -> (qsl_package::CheckedPackage, quire_exact::UnitId) {
    use qsl_semantics::value::{
        admit_unit_graph, DimensionPreimage, NodeIdentityPreimage, NodeOwner, OwnerSelection,
        OwnerSubject, UnitPreimage,
    };
    let owner = json!({"kind": "definition", "authority": "agent-ix", "identity": "example"});
    let length = DimensionPreimage::from_json(json!({
        "version": "quire.dimension-node/v1",
        "owner": owner,
        "qualified_declaration": ["Example", "Length"],
        "terms": [],
    }))
    .expect("a base dimension");
    let length_key = quire_exact::NodeKey::from_digest(length.digest().unwrap());
    let metre = UnitPreimage::from_json(json!({
        "version": "quire.unit-node/v1",
        "owner": owner,
        "qualified_declaration": ["Example", "metre"],
        "dimension_node_id": {
            "domain": quire_exact::NODE_KEY_DOMAIN,
            "digest": hex(length_key.as_bytes()),
        },
        "target_unit_node_id": null,
        "scale": {"numerator": "1", "denominator": "1"},
        "offset": {"numerator": "0", "denominator": "1"},
    }))
    .expect("a root unit");
    let metre_key = quire_exact::NodeKey::from_digest(metre.digest().unwrap());
    let graph = admit_unit_graph(
        [(length, length_key)],
        [(metre, metre_key)],
        &OwnerSelection::new([NodeOwner::Definition(OwnerSubject {
            authority: "agent-ix".into(),
            identity: "example".into(),
        })]),
    )
    .expect("the unit graph admits");
    let units = quire_semantic_value::quantity::UnitTable::declared(&graph);
    let graph = qsl_semantics::check::PackageDeclarations {
        types: quire_semantic_value::declaration::TypeEnvironment::default().with_units(units),
        ..qsl_semantics::check::PackageDeclarations::new(qsl_semantics::check::fixture_source())
    }
    .check(quire_semantic_value::checking::CheckingLimits::default())
    .expect("the unit table checks");
    (
        qsl_package::CheckedPackage::link(graph),
        quire_exact::UnitId::declared(metre_key),
    )
}

/// TC-744 step 3 (FR-269-AC-3): a quantity in a declared unit the package's
/// unit table holds round-trips; the declared unit resolves by lookup.
#[trace("TC-744", "FR-269-AC-3")]
#[test]
fn tc_744_a_declared_unit_quantity_round_trips() {
    let unit = witness_unit();
    let (package, metre) = metre_package();
    let magnitude = quire_exact::Rational::new(
        quire_exact::Integer::from(5_i64),
        quire_exact::Integer::from(2_i64),
    )
    .unwrap();
    let cause = cause_deciding(
        &unit,
        Value::Quantity(quire_exact::Quantity::new(magnitude, metre)),
    );
    let text = cause.to_json().unwrap();
    let read = DisagreementCause::from_json(&text, &package).unwrap();
    assert_eq!(read, cause);
    assert_eq!(read.to_json().unwrap(), text);
    assert!(matches!(
        DisagreementCause::from_json(&text, &unit.compiled.package),
        Err(crate::CauseCodecError::Unresolved("unit"))
    ));
}

/// TC-744 step 3 (FR-269-AC-3): a value path with QSpec FR-207's member,
/// option-value, field and index steps round-trips, each step in order;
/// a field step and a member step of the same name read back distinct.
#[trace("TC-744", "FR-269-AC-3")]
#[test]
fn tc_744_member_option_value_and_field_steps_round_trip() {
    let unit = witness_unit();
    let package = &unit.compiled.package;
    let violation = Verdict::from_category(ProofCategory::Violation);
    let mut record = all_below_record(&unit);
    record.value_path.steps = vec![
        ValuePathStep::Member("record".to_owned()),
        ValuePathStep::OptionValue,
        ValuePathStep::Field("items".to_owned()),
        ValuePathStep::Index(1),
    ];
    let cause = DisagreementCause::Witness {
        proved: violation,
        replayed: violation,
        given: Some(Box::new(record.clone())),
        derived: None,
        failure: WitnessFailure::Mismatch,
    };
    let text = cause.to_json().unwrap();
    let document: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        document["given"]["value_path"]["steps"],
        json!([
            {"tag": "member", "name": "record"},
            {"tag": "option_value"},
            {"tag": "field", "name": "items"},
            {"tag": "index", "index": 1},
        ])
    );
    assert_eq!(DisagreementCause::from_json(&text, package).unwrap(), cause);

    let mut renamed = record;
    renamed.value_path.steps[2] = ValuePathStep::Member("items".to_owned());
    assert_ne!(
        cause,
        DisagreementCause::Witness {
            proved: violation,
            replayed: violation,
            given: Some(Box::new(renamed)),
            derived: None,
            failure: WitnessFailure::Mismatch,
        }
    );
}

/// TC-744 step 3 (FR-269-AC-3, QSpec FR-352-AC-7): a refused separation
/// step round-trips; a refusal whose code is not a catalog code, or whose
/// cause or a field name is empty or not a catalog spelling, refuses.
#[trace("TC-744", "FR-269-AC-3")]
#[test]
fn tc_744_a_refusal_reads_only_catalog_spellings() {
    let unit = witness_unit();
    let package = &unit.compiled.package;
    let violation = Verdict::from_category(ProofCategory::Violation);
    let cause = DisagreementCause::Witness {
        proved: violation,
        replayed: violation,
        given: None,
        derived: Some(Box::new(all_below_record(&unit))),
        failure: WitnessFailure::Separation {
            step: SeparationStep::Domain,
            reason: SeparationReason::Refused(crate::SeparationRefusal {
                code: "integer_out_of_domain".to_owned(),
                cause: "value-out-of-domain".to_owned(),
                fields: BTreeMap::from([("value".to_owned(), "1".to_owned())]),
            }),
        },
    };
    let text = cause.to_json().unwrap();
    assert_eq!(DisagreementCause::from_json(&text, package).unwrap(), cause);
    for (member, value) in [
        ("code", json!("")),
        ("code", json!("made_up_code")),
        ("cause", json!("")),
        ("cause", json!("Value Out")),
        ("fields", json!({"": "1"})),
        ("fields", json!({"Value": "1"})),
    ] {
        let mut document: serde_json::Value = serde_json::from_str(&text).unwrap();
        document["failure"]["reason"][member] = value.clone();
        assert!(
            matches!(
                DisagreementCause::from_json(&document.to_string(), package),
                Err(crate::CauseCodecError::Spelling(_))
            ),
            "{member} = {value}"
        );
    }
}

/// TC-743 step 4 (FR-268-AC-4): a record whose deciding element is a long
/// text, or whose value path names an object with a long identity, is
/// measured by those lengths and refuses past the reader bound.
#[trace("TC-743", "FR-268-AC-4")]
#[test]
fn tc_743_long_text_or_object_identity_exceeds_the_reader_bound() {
    let unit = witness_unit();
    let long = "x".repeat(crate::MAX_ENCODED_BYTES + 1);
    let long_text = {
        let text_type = quire_exact::TextType::new(
            0,
            u64::try_from(long.len()).unwrap(),
            quire_exact::TextProfile::UnicodeScalars,
        )
        .unwrap();
        let payload = quire_exact::TextPayload::from_utf8(long.as_bytes()).unwrap();
        let mut meter = Meter::new(default_accounting(u64::MAX));
        match quire_exact::admit_text(&payload, &text_type, &mut meter) {
            Outcome::Completed(text) => Value::Text(text),
            other => panic!("the text admits: {other:?}"),
        }
    };
    let text_record = SeparatingWitnessRecord {
        deciding_element: long_text,
        ..all_below_record(&unit)
    };
    let object = quire_exact::ObjectReference::new(
        quire_exact::UniverseId::from_digest([2; 32]),
        quire_exact::EffectiveId::from_digest([3; 32]),
        quire_exact::ObjectId::new(long).unwrap(),
    );
    let mut object_record = all_below_record(&unit);
    object_record.value_path.subject = ValuePathSubject::Object(object);
    for record in [text_record, object_record] {
        assert!(record.measured_bytes() > crate::MAX_ENCODED_BYTES);
        let (_, envelope) = replay_case(
            &unit,
            "AllBelow",
            Snapshot::High,
            "mid",
            Some(record),
            witness_source(),
        );
        assert!(matches!(
            envelope,
            Err(crate::WitnessRefusal::BoundExceeded(_))
        ));
    }
}
