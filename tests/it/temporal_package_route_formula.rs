// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-515 "One evaluator formula" and "Clause and row by key" (TC-1310 rows
//! 21 and 23), FR-516-AC-39 (TC-1311 row 39): the package route assesses the
//! checked clause the package holds, and a clause that cannot be paired by
//! name is refused at construction.

use ix_trace_rs::trace;
use qsl_eval::temporal::evidence::{ClauseFormula, ClauseWork, EvidencePackage, PackageMismatch};
use serde_json::Value;

use super::temporal_package_route::{
    cause, clause_ids, edited, event_clauses, request, run, twins, unlimited,
};

/// Replace the first `upper` bound `from` of an interval below `value`.
fn bump_interval(value: &mut Value, from: &str, to: &str) -> bool {
    match value {
        Value::Object(members) => {
            if members.get("upper") == Some(&Value::from(from)) && members.contains_key("lower") {
                members.insert("upper".to_owned(), Value::from(to));
                return true;
            }
            members
                .values_mut()
                .any(|member| bump_interval(member, from, to))
        }
        Value::Array(items) => items.iter_mut().any(|item| bump_interval(item, from, to)),
        _ => false,
    }
}

const ONLY: &str = "temporal Only using t over (c: Config::ConfigVersion) clock \"model-steps\" \
                    on origin { eventually[0,1] holds(c.versionNumber <= 1) }";

/// The evaluator measures the package's formula and assesses the checked
/// clause's, so a package whose formula differs from the checked clause's in
/// one interval is `clause-unreadable` with an empty charge log.
#[trace("FR-515-AC-23", "FR-516-AC-39", "TC-1310", "TC-1311")]
#[test]
fn a_package_formula_that_differs_from_the_checked_clause_is_unreadable() {
    let built = event_clauses(ONLY, &[("Only", "only-authority")]);
    let read = built.read(&built.emission);
    let package = EvidencePackage::from_read(&read).expect("the read-back package");
    let [clause] = clause_ids(&package).try_into().expect("one clause");
    let asked = request(&package, &clause, None, 3);
    assert!(
        run(&package, &clause, &asked).0.is_ok(),
        "the unedited twin"
    );

    let changed = edited(built, |wire| {
        let mut done = false;
        for node in wire["semantic_graph"]["nodes"].as_array_mut().unwrap() {
            if node["semantic_form"] == "formula"
                && node.to_string().contains("quire.op.temporal.eventually")
            {
                done = bump_interval(node, "1", "2");
            }
        }
        assert!(done, "an interval to edit");
    });
    let read = changed.read(&changed.emission);
    let package = EvidencePackage::from_read(&read).expect("the read-back package");
    let [clause] = clause_ids(&package).try_into().expect("one clause");
    let mut meter = unlimited();
    let refused = package.evaluate_temporal_evidence(&clause, &asked, &mut meter);
    assert_eq!(
        cause(&refused.expect_err("a differing formula")),
        "clause-unreadable"
    );
    assert!(meter.admitted_charges().is_empty());
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
