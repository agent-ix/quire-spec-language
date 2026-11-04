// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-515 "One evaluator formula" and "Clause and row by key" (TC-1310 rows
//! 21 and 23), FR-516-AC-39 (TC-1311 row 39): the package route assesses the
//! checked clause the package holds, and a clause that cannot be paired by
//! name is refused at construction.

use ix_trace_rs::trace;
use qsl_eval::temporal::evidence::{
    ClauseFormula, ClauseWork, EvidenceLedger, EvidencePackage, PackageMismatch,
};
use qsl_eval::temporal::ProgressLedger;
use serde_json::Value;

use super::temporal_package_route::{
    cause, clause_ids, edited, event_clauses, recipe, request, run, twins, unlimited,
};

const ONLY: &str = "temporal Only using t over (c: Config::ConfigVersion) clock \"model-steps\" \
                    on origin { eventually[0,1] holds(c.versionNumber <= 1) }";

/// The `Only` clause with `body` as its formula.
fn only(body: &str) -> String {
    format!(
        "temporal Only using t over (c: Config::ConfigVersion) clock \"model-steps\" \
         on origin {{ {body} }}"
    )
}

/// FR-515-AC-24, FR-516-AC-46 (TC-1310 row 24, TC-1311 row 46): an
/// `EvidencePackage` built by `from_emission` from an `Emission` whose
/// in-process clause formula differs from the package's `formula_graph()` in
/// one operation or one interval (each in its own run) refuses
/// `clause-unreadable` at the first call that reaches the clause, with an empty
/// charge log and the caller's ledgers unchanged; the unaltered clause is
/// evaluated; and the `EvidencePackage` built by `from_read` over the same
/// bytes is evaluated, because it compares no in-process clause. A change of
/// operand edge alone cannot differ: the pre-order of operations and the
/// arities fix the edges.
#[trace("FR-515-AC-24", "FR-516-AC-39", "FR-516-AC-46", "TC-1310", "TC-1311")]
#[test]
fn a_package_formula_that_differs_from_the_checked_clause_is_unreadable() {
    // The checked clause reads `eventually[0,1]`; the package is the
    // emission of a unit whose clause differs in one operation or one interval.
    for (what, body) in [
        ("an operation", "always[0,1] holds(c.versionNumber <= 1)"),
        ("an interval", "eventually[0,2] holds(c.versionNumber <= 1)"),
    ] {
        let built = event_clauses(ONLY, &[("Only", "only-authority")]);
        let read = built.read(&built.emission);
        let unaltered =
            EvidencePackage::from_emission(&built.emission, &read).expect("the emission");
        let [clause] = clause_ids(&unaltered).try_into().expect("one clause");
        let mut asked = request(&unaltered, &clause, None, 3);
        assert!(
            run(&unaltered, &clause, &asked).0.is_ok(),
            "{what}: the unaltered clause"
        );
        asked.ledger = Some(EvidenceLedger::default());
        asked.progress_ledger = Some(ProgressLedger::new());
        let ledgers = format!("{:?}", (&asked.ledger, &asked.progress_ledger));

        let other = event_clauses(&only(body), &[("Only", "only-authority")]);
        let changed = built.with_emission_of(other);
        let read = changed.read(&changed.emission);
        let emitted = EvidencePackage::from_emission(&changed.emission, &read).expect("rows");
        let [clause] = clause_ids(&emitted).try_into().expect("one clause");
        let mut meter = unlimited();
        let refused = emitted.evaluate_temporal_evidence(&clause, &asked, &mut meter);
        assert_eq!(
            cause(&refused.expect_err("a differing formula")),
            "clause-unreadable",
            "{what}"
        );
        assert!(meter.admitted_charges().is_empty(), "{what}");
        assert_eq!(
            format!("{:?}", (&asked.ledger, &asked.progress_ledger)),
            ledgers,
            "{what}"
        );

        let back = EvidencePackage::from_read(&read).expect("the read-back package");
        assert!(
            run(&back, &clause, &asked).0.is_ok(),
            "{what}: the read-back package holds no in-process clause to compare"
        );
    }
}

/// FR-516-AC-45 (TC-1311 row 45): a call over a `CheckedNodeId` absent from the
/// package, and over the id of a `protocol_clause` node, each returns
/// `Err(CallFailure::Admission)` with cause `clause-root-unresolved`, an empty
/// charge log and the caller's ledgers unchanged.
#[trace("FR-516-AC-45", "TC-1311")]
#[test]
fn a_clause_the_package_does_not_hold_is_unresolved() {
    let built = recipe("event_position");
    let read = built.read(&built.emission);
    let package = EvidencePackage::from_emission(&built.emission, &read).expect("the emission");
    let due = clause_ids(&package).remove(0);
    let mut asked = request(&package, &due, None, 3);
    asked.ledger = Some(EvidenceLedger::default());
    asked.progress_ledger = Some(ProgressLedger::new());
    let ledgers = format!("{:?}", (&asked.ledger, &asked.progress_ledger));
    let protocol_clause = read
        .admitted()
        .graph()
        .nodes
        .iter()
        .zip(read.admitted().node_kinds())
        .find(|(_, kind)| {
            **kind
                == quire_contract_model::CheckedNodeKind::Protocol(
                    quire_contract_model::ProtocolForm::ProtocolClause,
                )
        })
        .map(|(node, _)| node.node_id.clone())
        .expect("R holds a protocol clause");
    let mut absent = due.clone();
    absent.ordinal += 1;
    for (what, id) in [("absent", absent), ("protocol clause", protocol_clause)] {
        let mut meter = unlimited();
        let refused = package.evaluate_temporal_evidence(&id, &asked, &mut meter);
        assert_eq!(
            cause(&refused.expect_err(what)),
            "clause-root-unresolved",
            "{what}"
        );
        assert!(meter.admitted_charges().is_empty(), "{what}");
        assert_eq!(
            format!("{:?}", (&asked.ledger, &asked.progress_ledger)),
            ledgers,
            "{what}"
        );
    }
}

/// The formula the call measures is the package's own listing and not the
/// checked formula's arena.
#[trace("FR-515-AC-23", "TC-1310")]
#[test]
fn the_call_measures_the_package_listing_and_not_the_checked_arena() {
    let built = event_clauses(
        "temporal Due using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin { \
         always[0,5] (holds(c.versionNumber >= 1) and holds(c.versionNumber >= 2) \
         and holds(c.versionNumber >= 3)) }",
        &[("Due", "authority")],
    );
    let read = built.read(&built.emission);
    let package = EvidencePackage::from_read(&read).expect("the read-back package");
    let [clause] = clause_ids(&package).try_into().expect("one clause");
    let subject = package.subject(&clause, None).expect("reads");
    let view = package.temporal_subject(&clause).expect("reads");
    assert_eq!(
        subject.formula,
        ClauseFormula::of_graph(view.formula_graph())
    );
    assert_ne!(
        subject.formula,
        ClauseFormula::of_checked(subject.clause.formula()),
        "the arena lists the nodes in an order of its own"
    );
    assert_eq!(
        subject.formula.work(),
        ClauseWork::of(subject.clause.formula())
    );
}

/// A clause reference whose position carries another clause's name, or names
/// no clause, is no emission of the package; two name rows that pair to one
/// clause cannot be paired by name.
#[trace("FR-515-AC-21", "TC-1310")]
#[test]
fn a_clause_that_cannot_be_paired_by_name_is_refused_at_construction() {
    let built = event_clauses(
        "temporal First using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin { eventually[0,1] holds(c.versionNumber <= 1) }\n\
         temporal Second using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin { eventually[0,2] holds(c.versionNumber <= 2) }",
        &[("First", "first-authority"), ("Second", "second-authority")],
    );
    let read = built.read(&built.emission);
    assert!(EvidencePackage::from_emission(&built.emission, &read).is_ok());
    let refs = built.emission.temporal_rows().clauses().to_vec();
    let at = |position: fn(usize) -> usize| -> Vec<qsl_package::TemporalClauseRef> {
        refs.iter()
            .map(|found| qsl_package::TemporalClauseRef {
                clause: position(found.clause),
                ..found.clone()
            })
            .collect()
    };
    for (what, position) in [
        (
            "another clause's position",
            (|at| at ^ 1) as fn(usize) -> usize,
        ),
        ("a position beyond the graph", |_| 9),
    ] {
        let wrong = built
            .emission
            .clone()
            .with_temporal_clause_refs(at(position));
        assert!(
            matches!(
                EvidencePackage::from_emission(&wrong, &read),
                Err(PackageMismatch::ClauseRow)
            ),
            "{what}"
        );
    }

    let renamed = edited(twins(), |wire| {
        for row in wire["temporal_clause_names"].as_array_mut().unwrap() {
            row["name"] = Value::from("First");
        }
    });
    let read = renamed.read(&renamed.emission);
    assert!(matches!(
        EvidencePackage::from_read(&read),
        Err(PackageMismatch::AmbiguousName)
    ));
}
