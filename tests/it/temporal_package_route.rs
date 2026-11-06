// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-515 (TC-1310 rows 18 to 23) and FR-516 (TC-1311 rows 18, 29, 30, 32
//! and 33): the temporal evidence call over the evaluation package built from
//! an `Emission` and over the one built from the emitted bytes read back.
//!
//! The three packages of the ported IT-001 recipe R are compiled in process
//! with their clock selections and emitted; the twin package no emitter
//! produces is the emitted bytes of two equal clauses with their occurrence
//! ordinals renumbered.

use std::collections::BTreeMap;
use std::path::Path;

use ix_trace_rs::trace;
use qsl_eval::temporal::evidence::{
    ActivationState, ClauseFormula, ClauseWork, CompletenessRecord, EvidencePackage,
    EvidencePosition, EvidenceRef, EvidenceRequest, EvidenceResult, ExecutionDisposition,
    ExecutionOrigin, PackageRows, Payload, Relation, SemanticTriggerIdentity,
    TemporalEvidenceLimits, TraceBinding, TriggerEvidence,
};
use qsl_eval::temporal::{ClockCover, ProgressClosure, ProgressCompleteness, Valuation};
use qsl_foundation::SourceIdentity;
use qsl_package::{
    read_evaluation_emission, AdmittedPackageRead, CheckedNodeId, CheckedOccurrence,
    CheckedOccurrenceRole, ClockConfiguration, Emission, TemporalClockSelection, V2ReadLimits,
};
use qsl_replay::spine::{DependencyInput, SuppliedLibrary};
use qsl_semantics::check::SCALAR_LIMITS_UNLIMITED;
use qsl_semantics::library::LibraryName;
use quire_eval::{CallAdmissionFailure, CallFailure};
use quire_exact::Meter;
use serde_json::Value;

use super::checked_handoff::{request as handoff_request, KINDS};
use quire_spec_language::checked_handoff::Corpus;

/// A compiled unit with the emission its clock selections give.
pub(super) struct Built {
    unit: qsl_replay::spine::CheckedUnit,
    pub(super) emission: Emission,
    packages: qsl_semantics::model::intake::PackageInput,
}

impl Built {
    /// This unit's in-process package with the emission of `other`.
    pub(super) fn with_emission_of(self, other: Built) -> Built {
        Built {
            emission: other.emission,
            ..self
        }
    }

    pub(super) fn read(&self, emission: &Emission) -> AdmittedPackageRead<'_> {
        self.try_read(emission)
            .expect("the reader admits the emission")
    }

    /// The identity string the replay route binds progress under: the unit's
    /// source identity.
    pub(super) fn replay_package_identity(&self) -> String {
        self.unit.source().identity().identity.clone()
    }

    /// The read of `emission`, which the reader may refuse.
    pub(super) fn try_read(
        &self,
        emission: &Emission,
    ) -> Result<AdmittedPackageRead<'_>, qsl_package::ImportViewRefusal> {
        self.try_read_under(emission, V2ReadLimits::fixture())
    }

    /// The read of `emission` under `limits`, which the reader may refuse.
    pub(super) fn try_read_under(
        &self,
        emission: &Emission,
        limits: V2ReadLimits,
    ) -> Result<AdmittedPackageRead<'_>, qsl_package::ImportViewRefusal> {
        read_evaluation_emission(
            self.unit.package(),
            emission,
            LibraryName::new(emission.package().name()).expect("a library name"),
            &self.packages,
            limits,
        )
    }
}

/// The clock `selections` joined to the clause of each name.
fn selections(
    unit: &qsl_replay::spine::CheckedUnit,
    clocks: &[(String, ClockConfiguration)],
) -> Vec<TemporalClockSelection> {
    clocks
        .iter()
        .map(|(name, clock)| {
            let span = unit
                .package()
                .graph()
                .temporal_clauses()
                .iter()
                .find(|node| node.clause().name() == name)
                .map(|node| node.clause().span())
                .or_else(|| {
                    unit.package()
                        .graph()
                        .protocols()
                        .iter()
                        .flat_map(|protocol| &protocol.obligations.compensations)
                        .find(|template| template.name == *name)
                        .map(|template| template.span)
                })
                .expect("a clause or template of that name");
            TemporalClockSelection {
                source: unit.source().reference().clone(),
                declaration_span: span,
                clock: clock.clone(),
            }
        })
        .collect()
}

fn emit(
    unit: qsl_replay::spine::CheckedUnit,
    clocks: &[(String, ClockConfiguration)],
    packages: qsl_semantics::model::intake::PackageInput,
) -> Built {
    let emission = qsl_package::emit_checked_with_temporal_selections(
        unit.package(),
        &selections(&unit, clocks),
    )
    .expect("the package emits");
    Built {
        unit,
        emission,
        packages,
    }
}

/// The package of recipe R of `kind`, compiled as the handoff writer does.
pub(super) fn recipe(kind: &str) -> Built {
    let emit_request = handoff_request(kind, Corpus::None).emit;
    let read = |path: &Path| std::fs::read(path).expect("a recipe file");
    let source = read(&emit_request.source);
    let models: Vec<Vec<u8>> = emit_request.models.iter().map(|path| read(path)).collect();
    let libraries: Vec<SuppliedLibrary> = emit_request
        .libraries
        .iter()
        .map(|(identity, path)| SuppliedLibrary {
            identity: identity.clone(),
            source: SourceIdentity::new("local", format!("library:{}", path.display())),
            path: path.display().to_string(),
            bytes: read(path),
        })
        .collect();
    let packages = qsl_semantics::model::intake::package_input(models.iter().map(Vec::as_slice))
        .expect("the recipe's model");
    let unit = crate::support::front_end::checked(
        SourceIdentity::new("local", format!("source:{}", emit_request.source.display())),
        &emit_request.source.display().to_string(),
        &source,
        &packages,
        &DependencyInput::new(libraries).expect("the recipe's library"),
    )
    .expect("S1 to S4 check the recipe");
    let clocks: Vec<_> = emit_request
        .temporal_selections
        .into_iter()
        .map(|selection| (selection.clause, selection.clock))
        .collect();
    emit(unit, &clocks, packages)
}

fn id(text: &str) -> SemanticTriggerIdentity {
    SemanticTriggerIdentity::new(text.as_bytes()).expect("a non-empty identity")
}

fn claim(ordinal: u64) -> CheckedOccurrence {
    CheckedOccurrence {
        role: CheckedOccurrenceRole::Claim,
        ordinal,
    }
}

pub(super) fn unlimited() -> Meter {
    Meter::new(SCALAR_LIMITS_UNLIMITED)
}

/// An origin-activated request over a clause of `package`, valuing its one
/// `holds` atom true at each of `ticks`, and asserting the clause's own row.
pub(super) fn request(
    package: &EvidencePackage<'_>,
    clause: &CheckedNodeId,
    occurrence: Option<u64>,
    ticks: u64,
) -> EvidenceRequest {
    let subject = package
        .subject(clause, occurrence.map(claim).as_ref())
        .expect("the clause reads");
    // Atom 0 is the first `holds` atom of the clause in formula order.
    assert!(subject.facts.formula().atom_count() >= 1);
    let atom = 0_u64;
    let chosen = occurrence.unwrap_or_else(|| subject.occurrences[0].occurrence.ordinal);
    let row = subject
        .occurrences
        .iter()
        .find(|found| found.occurrence.ordinal == chosen)
        .and_then(|found| found.clock)
        .expect("a clocked clause");
    EvidenceRequest {
        native_valuations: Vec::new(),
        trigger: Some(id("o1")),
        correspondence: EvidenceRef::named("corr"),
        positions: (0..ticks)
            .map(|index| EvidencePosition {
                observation: EvidenceRef::named(format!("obs{index}")),
                position: index,
                valuations: BTreeMap::from([(atom, Valuation::True)]),
                ..EvidencePosition::default()
            })
            .collect(),
        anchor: Some("obs0".to_owned()),
        deliveries: Vec::new(),
        origins: vec![ExecutionOrigin {
            identity: id("o1"),
            payload: Payload::new(b"a".as_slice()),
            captures: Vec::new(),
        }],
        trigger_evidence: TriggerEvidence::Admitted,
        trigger_scope: ProgressClosure::Closed,
        decision_progress: Some(100),
        decision_closure: Some(ProgressClosure::Closed),
        surrounding_progress: Some(100),
        surrounding_closure: Some(ProgressClosure::Closed),
        execution: ExecutionDisposition::Completed,
        completeness: Some(CompletenessRecord {
            reference: EvidenceRef::named("complete"),
            state: ProgressCompleteness::Complete { through: 100 },
            facts: Vec::new(),
        }),
        authoritative_origin: Some(true),
        evicted: Vec::new(),
        cover: ClockCover::Exact,
        progress_ledger: None,
        ledger: None,
        relation: Relation::Original,
        control: None,
        binding: TraceBinding {
            profile: subject.facts.profile().identity().into(),
            clock_name: row.clock_name.clone(),
            parameters: row.clock.clone(),
        },
        occurrence: occurrence.map(claim),
        limits: TemporalEvidenceLimits::default(),
    }
}

/// The result and the charge log of one call.
pub(super) fn run(
    package: &EvidencePackage<'_>,
    clause: &CheckedNodeId,
    asked: &EvidenceRequest,
) -> (Result<EvidenceResult, CallFailure>, Vec<String>) {
    let mut meter = unlimited();
    let result = package.evaluate_temporal_evidence(clause, asked, &mut meter);
    (result, charges(&meter))
}

fn charges(meter: &Meter) -> Vec<String> {
    meter
        .admitted_charges()
        .iter()
        .map(|charge| format!("{charge:?}"))
        .collect()
}

pub(super) fn cause(failure: &CallFailure) -> String {
    match failure {
        CallFailure::Admission(failure) => match &**failure {
            CallAdmissionFailure::Refused(record) => record
                .record
                .fields
                .get("cause")
                .cloned()
                .expect("a refusal names its cause"),
            other => panic!("a refusal, not {other:?}"),
        },
        other => panic!("an admission refusal, not {other:?}"),
    }
}

/// The result of an `Ok` call; the sibling check of every refusal test.
fn evaluated(result: Result<EvidenceResult, CallFailure>) -> EvidenceResult {
    result.unwrap_or_else(|failure| panic!("a result, not {failure:?}"))
}

/// The declaration ids of the clause occurrences of `package`'s rows, one
/// per named clause.
pub(super) fn clause_ids(package: &EvidencePackage<'_>) -> Vec<CheckedNodeId> {
    let mut ids: Vec<CheckedNodeId> = package
        .rows()
        .names()
        .iter()
        .map(|found| found.declaration.clone())
        .collect();
    ids.dedup();
    ids
}

/// FR-515-AC-18, FR-515-AC-21, FR-516-AC-29 (TC-1310 rows 18 and 21, TC-1311
/// row 29): over each of R's packages the evaluation package built from the
/// `Emission` and the one built from the bytes agree on `profile()`,
/// `clock()` and `occurrences()` of each clause, hold the `Emission`'s rows
/// byte for byte, and name each clause by the id of the `temporal_clause_names`
/// row read back at its occurrence.
#[trace("FR-515-AC-18", "FR-515-AC-21", "FR-516-AC-29", "TC-1310", "TC-1311")]
#[test]
fn both_routes_hold_the_emissions_rows_and_ids() {
    let mut seen = Vec::new();
    for kind in KINDS {
        let built = recipe(kind);
        let read = built.read(&built.emission);
        let emitted = EvidencePackage::from_emission(
            &built.emission,
            &read,
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the emission");
        let back = EvidencePackage::from_read(
            &read,
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the read-back package");

        let rows = built.emission.temporal_rows();
        assert_eq!(
            emitted.rows(),
            &PackageRows::of_package(read.admitted()).expect("the package's rows")
        );
        assert_eq!(
            serde_json::to_vec(emitted.rows().bindings()).unwrap(),
            serde_json::to_vec(rows.bindings()).unwrap()
        );
        assert_eq!(
            serde_json::to_vec(emitted.rows().profile_selections()).unwrap(),
            serde_json::to_vec(rows.profile_selections()).unwrap()
        );
        assert_eq!(emitted.rows().bindings(), back.rows().bindings());
        assert_eq!(
            emitted.rows().profile_selections(),
            back.rows().profile_selections()
        );
        // AC-21: the emitter's ids are the read-back name rows' ids.
        let names = read.admitted().temporal_clause_names().expect("name rows");
        for found in emitted.rows().names() {
            assert!(
                names.iter().any(|row| row.declaration == found.declaration
                    && row.occurrence == found.occurrence
                    && row.name == found.name),
                "{found:?}"
            );
            assert!(
                read.source()
                    .graph()
                    .temporal_clauses()
                    .iter()
                    .any(|node| node.clause().name() == found.name.as_ref()),
                "{found:?}"
            );
        }
        assert_eq!(emitted.rows().names(), back.rows().names());

        for clause in clause_ids(&emitted) {
            let left = emitted.temporal_subject(&clause).expect("the clause reads");
            let right = back.temporal_subject(&clause).expect("the clause reads");
            assert_eq!(emitted.profile(&left), back.profile(&right));
            assert_eq!(emitted.profile(&left), Some(left.profile()));
            assert_eq!(emitted.clock(&left), back.clock(&right));
            assert!(emitted.clock(&left).is_some(), "R's clauses are clocked");
            assert_eq!(left.occurrences(), right.occurrences());
            seen.push(
                emitted
                    .rows()
                    .names()
                    .iter()
                    .find(|found| found.declaration == clause)
                    .expect("a name row")
                    .name
                    .to_string(),
            );
        }
    }
    for name in ["Due", "DueLate", "DueSample", "DueTimestamp"] {
        assert!(seen.iter().any(|seen| seen == name), "{name}");
    }
}

/// FR-515-AC-23 (TC-1310 row 23): the evaluator formula built from the
/// package from the `Emission` equals, node for node, the one from the read-back
/// package, and measures what the checked formula measures.
#[trace("FR-515-AC-23", "TC-1310")]
#[test]
fn both_routes_give_the_evaluator_the_same_formula() {
    for kind in KINDS {
        let built = recipe(kind);
        let read = built.read(&built.emission);
        let emitted = EvidencePackage::from_emission(
            &built.emission,
            &read,
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the emission");
        let back = EvidencePackage::from_read(
            &read,
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the read-back package");
        for clause in clause_ids(&emitted) {
            let left = emitted.subject(&clause, None).expect("the clause reads");
            let right = back.subject(&clause, None).expect("the clause reads");
            assert_eq!(left.facts, right.facts);
            assert_eq!(left.instance_bound, right.instance_bound);
            let name = left.facts.name();
            let source = read
                .source()
                .graph()
                .temporal_clauses()
                .iter()
                .find(|node| node.clause().name() == name)
                .expect("the source clause of that name");
            assert_eq!(
                left.facts.formula().work(),
                ClauseWork::of(source.clause().formula()),
                "the package's formula measures what the checked formula does"
            );
            assert!(left.facts.formula().shapes().is_ok());
        }
    }
    // The evaluator formula of a package whose view lists the always entry
    // with one operand naming the holds entry has two nodes.
    let built = recipe("event_position");
    let read = built.read(&built.emission);
    let back = EvidencePackage::from_read(
        &read,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("the read-back package");
    let due = clause_ids(&back)
        .into_iter()
        .find(|clause| {
            back.subject(clause, None)
                .is_ok_and(|subject| subject.facts.name() == "Due")
        })
        .expect("Due");
    let view = back.temporal_subject(&due).expect("Due reads");
    assert_eq!(
        view.formula_graph()
            .iter()
            .map(|node| (node.operation, node.operands.clone()))
            .collect::<Vec<_>>(),
        [
            ("quire.op.temporal.always", vec![1]),
            ("quire.op.temporal.holds", vec![])
        ]
    );
    assert_eq!(
        view.formula()
            .iter()
            .map(|used| used.operation)
            .collect::<Vec<_>>(),
        ["quire.op.temporal.always", "quire.op.temporal.holds"]
    );
    assert_eq!(
        ClauseFormula::of_graph(view.formula_graph())
            .work()
            .unwrap()
            .formula_nodes,
        2
    );
}

/// FR-522-AC-20 (TC-1317) runs here too, and only under `--features
/// checked-handoff`.
/// FR-516-AC-18, FR-516-AC-29 (TC-1311 rows 18 and 29): the result of the call
/// over the package from the `Emission` equals, field for field, the one over
/// the read-back package, the charge log and the `result_identity` included,
/// and the clause's own `binding` authenticates; replacing one clock row, one
/// `CheckedNodeId` or one instance-bound atom of either side makes the two
/// results differ.
#[trace("FR-516-AC-18", "FR-516-AC-29", "FR-522-AC-20", "TC-1311", "TC-1317")]
#[test]
fn the_call_gives_equal_results_over_both_routes_and_any_replaced_input_differs() {
    for kind in KINDS {
        let built = recipe(kind);
        let read = built.read(&built.emission);
        let emitted = EvidencePackage::from_emission(
            &built.emission,
            &read,
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the emission");
        let back = EvidencePackage::from_read(
            &read,
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the read-back package");
        for clause in clause_ids(&emitted) {
            let subject = emitted.subject(&clause, None).expect("the clause reads");
            if !matches!(subject.facts.name(), "Due" | "DueSample" | "DueTimestamp") {
                continue;
            }
            let asked = request(&emitted, &clause, None, 3);
            let (left, left_charges) = run(&emitted, &clause, &asked);
            let (right, right_charges) = run(&back, &clause, &asked);
            let (left, right) = (evaluated(left), evaluated(right));
            assert_eq!(
                left.activation,
                ActivationState::Active,
                "{}",
                subject.facts.name()
            );
            assert_eq!(format!("{left:?}"), format!("{right:?}"));
            assert_eq!(left.result_identity, right.result_identity);
            assert_eq!(left_charges, right_charges);
            assert!(!left_charges.is_empty());

            // Replace one input of one side.
            let call = |changed: &qsl_eval::temporal::evidence::EvidenceSubject<'_>| {
                let mut meter = unlimited();
                let result = qsl_eval::temporal::evidence::evaluate_temporal_evidence(
                    changed, &asked, &mut meter,
                );
                (result, charges(&meter))
            };
            // One `CheckedNodeId`.
            let mut other = clause.clone();
            other.ordinal += 1;
            let mut changed = emitted.subject(&clause, None).expect("reads");
            changed.declaration = &other;
            let moved = evaluated(call(&changed).0);
            assert_ne!(moved.result_identity, right.result_identity);
            // One instance-bound atom.
            let atom = 0_u64;
            let mut changed = back.subject(&clause, None).expect("reads");
            changed.instance_bound.insert(atom);
            let bound = evaluated(call(&changed).0);
            assert_ne!(format!("{bound:?}"), format!("{left:?}"));
            // One clock row: the same row under another clock name.
            let mut replaced = subject.occurrences[0].clock.expect("a row").clone();
            replaced.clock_name = "another-clock".into();
            let mut changed = emitted.subject(&clause, None).expect("reads");
            changed.occurrences[0].clock = Some(&replaced);
            match call(&changed).0 {
                Err(failure) => assert_eq!(cause(&failure), "binding-mismatch"),
                Ok(result) => assert_ne!(result.result_identity, right.result_identity),
            }
        }
    }
}

/// FR-516-AC-30, FR-515-AC-19 (TC-1311 row 30, TC-1310 row 19): in a package
/// with two clauses of different formulas and different clock rows, each
/// clause's id pairs with its own row: a `binding` asserting the other
/// clause's row is refused `binding-mismatch`, in both routes.
#[trace("FR-516-AC-30", "FR-515-AC-19", "TC-1311", "TC-1310")]
#[test]
fn each_clause_pairs_with_its_own_clock_row() {
    let built = event_clauses(
        "temporal First using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin { eventually[0,1] holds(c.versionNumber <= 1) }\n\
         temporal Second using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin { eventually[0,2] holds(c.versionNumber <= 2) }",
        &[("First", "first-authority"), ("Second", "second-authority")],
    );
    let read = built.read(&built.emission);
    let emitted = EvidencePackage::from_emission(
        &built.emission,
        &read,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("the emission");
    let back = EvidencePackage::from_read(
        &read,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("the read-back package");
    let ids = clause_ids(&emitted);
    assert_eq!(ids.len(), 2, "two nodes");
    for package in [&emitted, &back] {
        for clause in &ids {
            let own = request(package, clause, None, 3);
            let (result, _) = run(package, clause, &own);
            assert_eq!(evaluated(result).activation, ActivationState::Active);
            let other = ids.iter().find(|other| *other != clause).expect("another");
            let mut foreign = request(package, other, None, 3);
            foreign.occurrence = None;
            let mut meter = unlimited();
            let refused = package.evaluate_temporal_evidence(clause, &foreign, &mut meter);
            assert_eq!(
                cause(&refused.expect_err("another clause's row")),
                "binding-mismatch"
            );
            assert!(meter.admitted_charges().is_empty());
        }
    }
}

/// A unit of `clauses` over the ConfigVersion model under the event-position
/// profile, each clause named in `authorities` given the sequence authority
/// beside it.
pub(super) fn event_clauses(clauses: &str, authorities: &[(&str, &str)]) -> Built {
    event_clauses_over("", &[], clauses, authorities)
}

/// [`event_clauses`] importing `libraries` (identity, source), with `imports`
/// the unit's own import lines.
pub(super) fn event_clauses_over(
    imports: &str,
    libraries: &[(&str, &str)],
    clauses: &str,
    authorities: &[(&str, &str)],
) -> Built {
    let packages = qsl_semantics::model::intake::package_input([
        crate::support::config_version_spine::spine::DOMAIN_PACKAGE.as_bytes(),
    ])
    .expect("the ConfigVersion model");
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         profile t = \"quire.temporal.event-position.false-extension/v1\";\n\
         {imports}model Config = \"example/config-version\";\n{clauses}\n"
    );
    let supplied: Vec<SuppliedLibrary> = libraries
        .iter()
        .map(|(identity, text)| SuppliedLibrary {
            identity: (*identity).to_owned(),
            source: SourceIdentity::new("local", format!("library:{identity}")),
            path: (*identity).to_owned(),
            bytes: text.as_bytes().to_vec(),
        })
        .collect();
    let unit = crate::support::front_end::checked(
        SourceIdentity::new("local", "source:route"),
        "route.native",
        source.as_bytes(),
        &packages,
        &DependencyInput::new(supplied).expect("the libraries"),
    )
    .expect("S1 to S4 check the unit");
    let clocks: Vec<_> = authorities
        .iter()
        .map(|(name, authority)| {
            (
                (*name).to_owned(),
                ClockConfiguration::EventPosition {
                    sequence_authority: (*authority).into(),
                },
            )
        })
        .collect();
    emit(unit, &clocks, packages)
}

/// The twin package: two structurally equal clauses, `First` and `Second`,
/// whose emitted occurrence ordinals 0 and 1 are renumbered 7 and 3, so that
/// the node's `occurrences` array runs 7 then 3 while the lock's rows ascend 3
/// then 7, each with its own clock configuration. No emitter produces it.
pub(super) fn twins() -> Built {
    let body = "eventually[0,1] holds(c.versionNumber <= 1)";
    let built = event_clauses(
        &format!(
            "temporal First using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin {{ {body} }}\n\
             temporal Second using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin {{ {body} }}"
        ),
        &[("First", "first-authority"), ("Second", "second-authority")],
    );
    let mut wire: Value = serde_json::from_slice(built.emission.package().bytes()).expect("JSON");
    let renumber = |ordinal: &Value| -> Value {
        match ordinal.as_u64() {
            Some(0) => Value::from(7),
            Some(1) => Value::from(3),
            other => panic!("an emitted ordinal: {other:?}"),
        }
    };
    let node = wire["temporal_bindings"][0]["declaration"].clone();
    for graph_node in wire["semantic_graph"]["nodes"].as_array_mut().unwrap() {
        if graph_node["node_id"] == node {
            for occurrence in graph_node["occurrences"].as_array_mut().unwrap() {
                occurrence["ordinal"] = renumber(&occurrence["ordinal"]);
            }
        }
    }
    for entry in wire["source_map"].as_array_mut().unwrap() {
        if entry["node_id"] == node && entry["role"] == "claim" {
            entry["ordinal"] = renumber(&entry["ordinal"]);
        }
    }
    for member in ["temporal_bindings", "temporal_clause_names"] {
        let rows = wire[member].as_array_mut().unwrap();
        for row in rows.iter_mut() {
            row["occurrence"] = renumber(&row["occurrence"]);
        }
        rows.sort_by_key(|row| row["occurrence"].as_u64());
    }
    let bytes = serde_json::to_vec(&wire).expect("the wire encodes");
    Built {
        emission: built.emission.clone().with_package_bytes(bytes),
        ..built
    }
}

fn authority_of(row: &qsl_package::TemporalBinding) -> String {
    match &row.clock {
        ClockConfiguration::EventPosition { sequence_authority } => sequence_authority.to_string(),
        other => panic!("an event-position clock: {other:?}"),
    }
}

/// FR-515-AC-20, FR-516-AC-32, FR-516-AC-33 (TC-1310 row 20, TC-1311 rows 32
/// and 33): over the twin package read back, a request with `occurrence`
/// ordinal 3 authenticates the row keyed 3 and refuses the row keyed 7 with
/// `binding-mismatch`, and the reverse for 7; no `occurrence` is
/// `occurrence-required` and ordinal 5 `occurrence-unknown`, each charging
/// nothing; over a node with one occurrence the selector may be omitted.
#[trace("FR-515-AC-20", "FR-516-AC-32", "FR-516-AC-33", "TC-1310", "TC-1311")]
#[test]
fn twins_pair_their_rows_by_key_over_the_read_back_route() {
    let built = twins();
    let read = built.read(&built.emission);
    let back = EvidencePackage::from_read(
        &read,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("the read-back package");
    let [node] = clause_ids(&back).try_into().expect("one shared node");

    let view = back.temporal_subject(&node).expect("the twins read");
    let ordinals: Vec<_> = view
        .occurrences()
        .iter()
        .map(|found| found.ordinal)
        .collect();
    assert_eq!(ordinals, [7, 3]);
    assert_eq!(
        back.rows()
            .bindings()
            .iter()
            .map(|row| row.occurrence)
            .collect::<Vec<_>>(),
        [3, 7]
    );
    assert_eq!(back.clock(&view), None);
    let [seven, three] = view.occurrences() else {
        panic!("two occurrences");
    };
    assert_eq!(
        authority_of(back.clock_of(three).expect("ordinal 3")),
        "second-authority"
    );
    assert_eq!(
        authority_of(back.clock_of(seven).expect("ordinal 7")),
        "first-authority"
    );

    for (ordinal, name, own, other) in [
        (3, "Second", "second-authority", "first-authority"),
        (7, "First", "first-authority", "second-authority"),
    ] {
        // The selector picks the clause occurrence once, and the subject the
        // call reads carries it: the clause named by that occurrence's name
        // row and progress retained under its ordinal.
        let chosen = back
            .subject(&node, Some(&claim(ordinal)))
            .expect("the occurrence reads");
        assert_eq!(chosen.facts.name(), name);
        assert_eq!(chosen.progress.occurrence, ordinal);
        let mut asked = request(&back, &node, Some(ordinal), 3);
        let result = evaluated(run(&back, &node, &asked).0);
        assert_eq!(result.activation, ActivationState::Active);
        asked.binding.parameters = ClockConfiguration::EventPosition {
            sequence_authority: other.into(),
        };
        let mut meter = unlimited();
        let refused = back.evaluate_temporal_evidence(&node, &asked, &mut meter);
        assert_eq!(
            cause(&refused.expect_err("the other row")),
            "binding-mismatch"
        );
        assert!(meter.admitted_charges().is_empty());
        asked.binding.parameters = ClockConfiguration::EventPosition {
            sequence_authority: own.into(),
        };
        assert!(run(&back, &node, &asked).0.is_ok());
    }

    let mut asked = request(&back, &node, Some(3), 3);
    asked.occurrence = None;
    let mut meter = unlimited();
    let refused = back.evaluate_temporal_evidence(&node, &asked, &mut meter);
    assert_eq!(
        cause(&refused.expect_err("no selector")),
        "occurrence-required"
    );
    assert!(meter.admitted_charges().is_empty());
    asked.occurrence = Some(claim(5));
    let mut meter = unlimited();
    let refused = back.evaluate_temporal_evidence(&node, &asked, &mut meter);
    assert_eq!(cause(&refused.expect_err("unknown")), "occurrence-unknown");
    assert!(meter.admitted_charges().is_empty());

    // A node with one occurrence needs no selector.
    let single = event_clauses(
        "temporal Only using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin { eventually[0,1] holds(c.versionNumber <= 1) }",
        &[("Only", "only-authority")],
    );
    let read = single.read(&single.emission);
    let package = EvidencePackage::from_read(
        &read,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("the read-back package");
    let [clause] = clause_ids(&package).try_into().expect("one clause");
    let mut asked = request(&package, &clause, None, 3);
    asked.occurrence = None;
    assert!(run(&package, &clause, &asked).0.is_ok());
}

/// FR-515-AC-22 (TC-1310 row 22): the instance-bound atoms of a clause are
/// derived from the package, equal over both routes, and no caller supplies
/// them. The atoms are the one reading a capture (index of the first
/// `holds`) and the one reading `over` and the capture; the guard has no index.
#[trace("FR-515-AC-22", "TC-1310")]
#[test]
fn instance_bound_atoms_are_derived_and_equal_over_both_routes() {
    let capture = "capture n: Int[0, 1000] = c.versionNumber; ";
    let build = |atoms: &str| {
        event_clauses(
            &format!(
                "temporal Due using t over (c: Config::ConfigVersion) clock \"model-steps\" \
                 on each (q: Config::ConfigVersion) when (q.versionNumber >= 2) {{ {capture}always[0,5] ({atoms}) }}"
            ),
            &[("Due", "authority")],
        )
    };
    let atom_indices = |built: &Built| {
        let read = built.read(&built.emission);
        let emitted = EvidencePackage::from_emission(
            &built.emission,
            &read,
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the emission");
        let back = EvidencePackage::from_read(
            &read,
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the read-back package");
        let [clause] = clause_ids(&emitted).try_into().expect("one clause");
        let left = emitted.subject(&clause, None).expect("reads");
        let right = back.subject(&clause, None).expect("reads");
        assert_eq!(left.instance_bound, right.instance_bound);
        left.instance_bound.clone()
    };
    let bound = atom_indices(&build(
        "holds(n >= 1) and holds(c.versionNumber <= 5) and holds(true) and holds(c.versionNumber >= n)",
    ));
    // Atoms are numbered by first use over the `holds` nodes only, so the
    // set is the first and the fourth atom of the formula (FR-532-AC-4).
    assert_eq!(bound.iter().copied().collect::<Vec<_>>(), [0, 3]);

    let swapped = atom_indices(&build(
        "holds(c.versionNumber <= 5) and holds(n >= 1) and holds(true) and holds(c.versionNumber >= n)",
    ));
    let ranks: Vec<_> = swapped.iter().copied().collect();
    assert_eq!(ranks, [1, 3], "the indices move with the atoms");
}

/// Edit the wire of `built` with `edit` and present the bytes as its emission.
pub(super) fn edited(built: Built, edit: impl FnOnce(&mut Value)) -> Built {
    let mut wire: Value = serde_json::from_slice(built.emission.package().bytes()).expect("JSON");
    edit(&mut wire);
    let bytes = serde_json::to_vec(&wire).expect("the wire encodes");
    Built {
        emission: built.emission.clone().with_package_bytes(bytes),
        ..built
    }
}

/// Two clauses of one name, `Same`, with their own formulas and clocks. The
/// checker admits them, so the clock selections are joined by position.
pub(super) fn same_named_clauses() -> Built {
    let packages = qsl_semantics::model::intake::package_input([
        crate::support::config_version_spine::spine::DOMAIN_PACKAGE.as_bytes(),
    ])
    .expect("the ConfigVersion model");
    let clause = |bound: u8| {
        format!(
            "temporal Same using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin {{ eventually[0,{bound}] holds(c.versionNumber <= {bound}) }}\n"
        )
    };
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.model.complete/v1\";\n\
         profile t = \"quire.temporal.event-position.false-extension/v1\";\n\
         model Config = \"example/config-version\";\n{}{}",
        clause(1),
        clause(2)
    );
    let unit = crate::support::front_end::checked(
        SourceIdentity::new("local", "source:same"),
        "same.native",
        source.as_bytes(),
        &packages,
        &DependencyInput::default(),
    )
    .expect("the checker admits two clauses of one name");
    let selections: Vec<TemporalClockSelection> = unit
        .package()
        .graph()
        .temporal_clauses()
        .iter()
        .enumerate()
        .map(|(index, node)| TemporalClockSelection {
            source: unit.source().reference().clone(),
            declaration_span: node.clause().span(),
            clock: ClockConfiguration::EventPosition {
                sequence_authority: format!("authority-{index}").into(),
            },
        })
        .collect();
    let emission = qsl_package::emit_checked_with_temporal_selections(unit.package(), &selections)
        .expect("the package emits");
    Built {
        unit,
        emission,
        packages,
    }
}

/// FR-532-AC-8 (TC-1332): the twins' rows pair by key over `from_package`, as
/// over the read-back route.
#[trace("FR-532-AC-8", "TC-1332")]
#[test]
fn twins_pair_their_rows_by_key_over_the_bytes_only_route() {
    let built = twins();
    let read = built.read(&built.emission);
    let bare = qsl_package::BarePackage::new(read.admitted());
    let back = EvidencePackage::from_package(&bare).expect("the bare package");
    let [node] = clause_ids(&back).try_into().expect("one shared node");

    let view = back.temporal_subject(&node).expect("the twins read");
    let ordinals: Vec<_> = view
        .occurrences()
        .iter()
        .map(|found| found.ordinal)
        .collect();
    assert_eq!(ordinals, [7, 3]);
    assert_eq!(
        back.rows()
            .bindings()
            .iter()
            .map(|row| row.occurrence)
            .collect::<Vec<_>>(),
        [3, 7]
    );
    assert_eq!(back.clock(&view), None);
    let [seven, three] = view.occurrences() else {
        panic!("two occurrences");
    };
    assert_eq!(
        authority_of(back.clock_of(three).expect("ordinal 3")),
        "second-authority"
    );
    assert_eq!(
        authority_of(back.clock_of(seven).expect("ordinal 7")),
        "first-authority"
    );

    for (ordinal, name, own, other) in [
        (3, "Second", "second-authority", "first-authority"),
        (7, "First", "first-authority", "second-authority"),
    ] {
        // The selector picks the clause occurrence once, and the subject the
        // call reads carries it: the clause named by that occurrence's name
        // row and progress retained under its ordinal.
        let chosen = back
            .subject(&node, Some(&claim(ordinal)))
            .expect("the occurrence reads");
        assert_eq!(chosen.facts.name(), name);
        assert_eq!(chosen.progress.occurrence, ordinal);
        let mut asked = request(&back, &node, Some(ordinal), 3);
        let result = evaluated(run(&back, &node, &asked).0);
        assert_eq!(result.activation, ActivationState::Active);
        asked.binding.parameters = ClockConfiguration::EventPosition {
            sequence_authority: other.into(),
        };
        let mut meter = unlimited();
        let refused = back.evaluate_temporal_evidence(&node, &asked, &mut meter);
        assert_eq!(
            cause(&refused.expect_err("the other row")),
            "binding-mismatch"
        );
        assert!(meter.admitted_charges().is_empty());
        asked.binding.parameters = ClockConfiguration::EventPosition {
            sequence_authority: own.into(),
        };
        assert!(run(&back, &node, &asked).0.is_ok());
    }

    let mut asked = request(&back, &node, Some(3), 3);
    asked.occurrence = None;
    let mut meter = unlimited();
    let refused = back.evaluate_temporal_evidence(&node, &asked, &mut meter);
    assert_eq!(
        cause(&refused.expect_err("no selector")),
        "occurrence-required"
    );
    assert!(meter.admitted_charges().is_empty());
    asked.occurrence = Some(claim(5));
    let mut meter = unlimited();
    let refused = back.evaluate_temporal_evidence(&node, &asked, &mut meter);
    assert_eq!(cause(&refused.expect_err("unknown")), "occurrence-unknown");
    assert!(meter.admitted_charges().is_empty());

    // A node with one occurrence needs no selector.
    let single = event_clauses(
        "temporal Only using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin { eventually[0,1] holds(c.versionNumber <= 1) }",
        &[("Only", "only-authority")],
    );
    let read = single.read(&single.emission);
    let package = EvidencePackage::from_read(
        &read,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("the read-back package");
    let [clause] = clause_ids(&package).try_into().expect("one clause");
    let mut asked = request(&package, &clause, None, 3);
    asked.occurrence = None;
    assert!(run(&package, &clause, &asked).0.is_ok());
}

/// FR-532-AC-8 (TC-1332): each clause pairs with its own clock row over
/// `from_package`, and another clause's row is `binding-mismatch`.
#[trace("FR-532-AC-8", "TC-1332")]
#[test]
fn each_clause_pairs_with_its_own_clock_row_over_the_bytes_only_route() {
    let built = event_clauses(
        "temporal First using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin { eventually[0,1] holds(c.versionNumber <= 1) }\n\
         temporal Second using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin { eventually[0,2] holds(c.versionNumber <= 2) }",
        &[("First", "first-authority"), ("Second", "second-authority")],
    );
    let read = built.read(&built.emission);
    let emitted = EvidencePackage::from_emission(
        &built.emission,
        &read,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("the emission");
    let bare = qsl_package::BarePackage::new(read.admitted());
    let back = EvidencePackage::from_package(&bare).expect("the bare package");
    let ids = clause_ids(&emitted);
    assert_eq!(ids.len(), 2, "two nodes");
    for package in [&emitted, &back] {
        for clause in &ids {
            let own = request(package, clause, None, 3);
            let (result, _) = run(package, clause, &own);
            assert_eq!(evaluated(result).activation, ActivationState::Active);
            let other = ids.iter().find(|other| *other != clause).expect("another");
            let mut foreign = request(package, other, None, 3);
            foreign.occurrence = None;
            let mut meter = unlimited();
            let refused = package.evaluate_temporal_evidence(clause, &foreign, &mut meter);
            assert_eq!(
                cause(&refused.expect_err("another clause's row")),
                "binding-mismatch"
            );
            assert!(meter.admitted_charges().is_empty());
        }
    }
}
