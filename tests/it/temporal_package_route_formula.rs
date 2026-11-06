// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-515 "One evaluator formula" and "Clause and row by key" (TC-1310 rows
//! 21 and 23), FR-516-AC-39 (TC-1311 row 39): the package route assesses the
//! checked clause the package holds, and a clause that cannot be paired by
//! name is refused at construction.

use ix_trace_rs::trace;
use qsl_eval::temporal::evidence::{
    ClauseFormula, ClauseWork, EvidencePackage, NonValueKind, PackageMismatch, Truth,
};
use qsl_eval::temporal::AggregateSample;
use qsl_package::CheckedNodeId;
use serde_json::Value;

use super::temporal_package_route::{
    cause, clause_ids, edited, event_clauses, recipe, request, run, same_named_clauses, twins,
    unlimited,
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
/// charge log; the unaltered clause is
/// evaluated; and the `EvidencePackage` built by `from_read` over the same
/// bytes, paired with the same in-process source graph, is refused the same
/// way. The third run writes another node under a connective, which is an
/// operation difference at that position: an operand-edge change alone cannot
/// exist, because the pre-order of operations and the arities fix the edges.
#[trace("FR-515-AC-24", "FR-516-AC-39", "FR-516-AC-46", "TC-1310", "TC-1311")]
#[test]
fn a_package_formula_that_differs_from_the_checked_clause_is_unreadable() {
    // The checked clause reads `eventually[0,1]`; the package is the
    // emission of a unit whose clause differs in one operation or one interval.
    for (what, source, body) in [
        (
            "an operation",
            ONLY.to_owned(),
            only("always[0,1] holds(c.versionNumber <= 1)"),
        ),
        (
            "an interval",
            ONLY.to_owned(),
            only("eventually[0,2] holds(c.versionNumber <= 1)"),
        ),
        (
            "a connective's operand (an operation difference)",
            only("eventually[0,1] (holds(c.versionNumber <= 1) and holds(c.versionNumber >= 0))"),
            only("eventually[0,1] (holds(c.versionNumber <= 1) and true)"),
        ),
    ] {
        let built = event_clauses(&source, &[("Only", "only-authority")]);
        let read = built.read(&built.emission);
        let unaltered = EvidencePackage::from_emission(
            &built.emission,
            &read,
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the emission");
        let [clause] = clause_ids(&unaltered).try_into().expect("one clause");
        let asked = request(&unaltered, &clause, None, 3);
        assert!(
            run(&unaltered, &clause, &asked).0.is_ok(),
            "{what}: the unaltered clause"
        );

        let other = event_clauses(&body, &[("Only", "only-authority")]);
        let changed = built.with_emission_of(other);
        let read = changed.read(&changed.emission);
        let emitted = EvidencePackage::from_emission(
            &changed.emission,
            &read,
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("rows");
        let [clause] = clause_ids(&emitted).try_into().expect("one clause");
        let mut meter = unlimited();
        let refused = emitted.evaluate_temporal_evidence(&clause, &asked, &mut meter);
        assert_eq!(
            cause(&refused.expect_err("a differing formula")),
            "clause-unreadable",
            "{what}"
        );
        assert!(meter.admitted_charges().is_empty(), "{what}");

        // The read-back package is paired with the same in-process source
        // graph, so the same bytes are refused the same way.
        let back = EvidencePackage::from_read(
            &read,
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the read-back package");
        let mut meter = unlimited();
        let refused = back.evaluate_temporal_evidence(&clause, &asked, &mut meter);
        assert_eq!(
            cause(&refused.expect_err("a differing formula")),
            "clause-unreadable",
            "{what}: read back"
        );
        assert!(meter.admitted_charges().is_empty(), "{what}: read back");
    }
}

/// FR-516-AC-45 (TC-1311 row 45): a call over a `CheckedNodeId` absent from the
/// package, and over the id of a `protocol_clause` node, each returns
/// `Err(CallFailure::Admission)` with cause `clause-root-unresolved`, an empty
/// charge log.
#[trace("FR-516-AC-45", "TC-1311")]
#[test]
fn a_clause_the_package_does_not_hold_is_unresolved() {
    let built = recipe("event_position");
    let read = built.read(&built.emission);
    let package = EvidencePackage::from_emission(
        &built.emission,
        &read,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("the emission");
    let due = clause_ids(&package).remove(0);
    let asked = request(&package, &due, None, 3);
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
    }
}

/// The formula the call measures is the package's own listing and not the
/// checked formula's arena.
#[trace("FR-515-AC-23", "TC-1310")]
#[test]
fn the_call_measures_the_package_listing() {
    let built = event_clauses(
        "temporal Due using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin { \
         always[0,5] (holds(c.versionNumber >= 1) and holds(c.versionNumber >= 2) \
         and holds(c.versionNumber >= 3)) }",
        &[("Due", "authority")],
    );
    let read = built.read(&built.emission);
    let package = EvidencePackage::from_read(
        &read,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("the read-back package");
    let [clause] = clause_ids(&package).try_into().expect("one clause");
    let subject = package.subject(&clause, None).expect("reads");
    let view = package.temporal_subject(&clause).expect("reads");
    assert_eq!(
        subject.facts.formula(),
        &ClauseFormula::of_graph(view.formula_graph())
    );
    let source = read
        .source()
        .graph()
        .temporal_clauses()
        .iter()
        .find(|node| node.clause().name() == "Due")
        .expect("Due");
    assert_eq!(
        subject.facts.formula().work(),
        ClauseWork::of(source.clause().formula())
    );
}

const FIRST_SECOND: &str = "temporal First using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin { eventually[0,1] holds(c.versionNumber <= 1) }\n\
     temporal Second using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin { eventually[0,2] holds(c.versionNumber <= 2) }";

/// FR-515-AC-25 (TC-1310 row 26): over an `Emission` whose clause reference at
/// a position points at a clause carrying another name than the name row (or
/// at no clause), `from_emission` returns `PackageMismatch::ClauseRow` at
/// construction and no call is made.
#[trace("FR-515-AC-25", "TC-1310")]
#[test]
fn a_clause_reference_at_another_clause_is_refused_at_construction() {
    let built = event_clauses(
        FIRST_SECOND,
        &[("First", "first-authority"), ("Second", "second-authority")],
    );
    let read = built.read(&built.emission);
    assert!(EvidencePackage::from_emission(
        &built.emission,
        &read,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
    )
    .is_ok());
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
                EvidencePackage::from_emission(
                    &wrong,
                    &read,
                    &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
                ),
                Err(PackageMismatch::ClauseRow)
            ),
            "{what}"
        );
    }
}

/// FR-515-AC-26 (TC-1310 row 27): over a read package with two clauses of one
/// name, and over one with two name rows pairing to one clause, `from_read`
/// returns `PackageMismatch::AmbiguousName`; `from_emission` over the
/// two-clauses-of-one-name package pairs by `clause` position and constructs.
#[trace("FR-515-AC-26", "TC-1310")]
#[test]
fn an_ambiguous_name_is_refused_by_the_read_route_only() {
    let same = same_named_clauses();
    let read = same.read(&same.emission);
    assert!(matches!(
        EvidencePackage::from_read(
            &read,
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
        ),
        Err(PackageMismatch::AmbiguousName)
    ));
    assert!(EvidencePackage::from_emission(
        &same.emission,
        &read,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
    )
    .is_ok());

    let renamed = edited(twins(), |wire| {
        for row in wire["temporal_clause_names"].as_array_mut().unwrap() {
            row["name"] = Value::from("First");
        }
    });
    let read = renamed.read(&renamed.emission);
    assert!(matches!(
        EvidencePackage::from_read(
            &read,
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
        ),
        Err(PackageMismatch::AmbiguousName)
    ));
}

/// FR-515-AC-27 (TC-1310 row 28): over an `Emission` whose `profile_selections`
/// do not match the package, `from_emission` returns
/// `PackageMismatch::ProfileSelections`.
#[trace("FR-515-AC-27", "TC-1310")]
#[test]
fn profile_selections_that_are_not_the_packages_are_refused() {
    let built = event_clauses(ONLY, &[("Only", "only-authority")]);
    let read = built.read(&built.emission);
    let mut selections = built.emission.temporal_rows().profile_selections().to_vec();
    selections.pop();
    let wrong = built
        .emission
        .clone()
        .with_temporal_profile_selections(selections);
    assert!(matches!(
        EvidencePackage::from_emission(
            &wrong,
            &read,
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
        ),
        Err(PackageMismatch::ProfileSelections)
    ));
}

/// FR-515-AC-28 (TC-1310 row 29): over an `Emission` whose `temporal_bindings`
/// rows do not match the package, `from_emission` returns
/// `PackageMismatch::Bindings`.
#[trace("FR-515-AC-28", "TC-1310")]
#[test]
fn bindings_that_are_not_the_packages_are_refused() {
    let built = event_clauses(ONLY, &[("Only", "only-authority")]);
    let read = built.read(&built.emission);
    let mut bindings = built.emission.temporal_rows().bindings().to_vec();
    bindings[0].clock_name = "another-clock".into();
    let wrong = built.emission.clone().with_temporal_bindings(bindings);
    assert!(matches!(
        EvidencePackage::from_emission(
            &wrong,
            &read,
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
        ),
        Err(PackageMismatch::Bindings)
    ));
    let none = built.emission.clone().with_temporal_bindings(Vec::new());
    assert!(matches!(
        EvidencePackage::from_emission(
            &none,
            &read,
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
        ),
        Err(PackageMismatch::Bindings)
    ));
}

/// FR-516-AC-47 (its `count` part only; `sum`, `min` and `max` are covered in
/// `qsl-eval`) and FR-516-AC-49 (TC-1311): the package subject
/// view reads an aggregate atom, so both clauses of the package are readable
/// and the aggregate atom is valued from the request's `aggregate_samples`,
/// never from the `holds` valuation map. `count over past[0,1]` of `Counted`
/// samples is 1 at position 0 and 2 afterwards: `min_count 1` makes the atom
/// true at every position (`truth = True`); `min_count 5` is starved, so the
/// result has no truth and a `Missing` `insufficient-data` non-value (count 1,
/// min_count 5) at position 0, although the request also supplies
/// `Valuation::True` at atom 0, which must not answer for it.
#[trace("FR-516-AC-47", "FR-516-AC-49", "TC-1311")]
#[test]
fn a_clause_with_an_aggregate_atom_is_read_from_its_samples() {
    let clauses = "record Sample { level: Int[0, 9]; }\n\
        temporal Fed using t over (e: Sample) clock \"model-steps\" \
        on origin { always[0,2] (count over past[0,1] min_count 1 >= 1) }\n\
        temporal Starved using t over (e: Sample) clock \"model-steps\" \
        on origin { always[0,2] (count over past[0,1] min_count 5 >= 1) }";
    let built = event_clauses(clauses, &[("Fed", "fed"), ("Starved", "starved")]);
    let read = built.read(&built.emission);
    let package = EvidencePackage::from_emission(
        &built.emission,
        &read,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("the emission");
    let ids = clause_ids(&package);
    assert_eq!(ids.len(), 2);
    assert!(
        ids.iter().all(|id| package.subject(id, None).is_ok()),
        "the view reads both aggregate clauses"
    );
    let named = |wanted: &str| {
        let row = package
            .rows()
            .names()
            .iter()
            .find(|row| &*row.name == wanted)
            .expect("the named clause");
        row.declaration.clone()
    };
    let call = |id: &CheckedNodeId| {
        let mut asked = request(&package, id, None, 3);
        for position in &mut asked.positions {
            position
                .aggregate_samples
                .insert(0, AggregateSample::Counted);
        }
        let mut meter = unlimited();
        package
            .evaluate_temporal_evidence(id, &asked, &mut meter)
            .expect("an aggregate clause evaluates from its samples")
    };
    assert_eq!(call(&named("Fed")).truth, Some(Truth::True));
    let starved = call(&named("Starved"));
    assert_eq!(starved.truth, None);
    let non_value = starved
        .non_value
        .expect("the starved window is a non-value");
    assert_eq!(non_value.kind, NonValueKind::Missing);
    assert_eq!(non_value.code, "insufficient-data");
    assert_eq!(non_value.dimension.as_deref(), Some("valuation:0@0"));
    assert_eq!(
        non_value.detail,
        [
            ("count".to_owned(), "1".to_owned()),
            ("min_count".to_owned(), "5".to_owned())
        ]
    );
}
