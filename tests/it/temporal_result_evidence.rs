// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-526-AC-20 to AC-23 (TC-1321 rows 20 to 23): the checked subject every
//! evidence result carries, and the support it names, over packages the
//! emitter produced and the reader admitted. Expected values are read from
//! the package and the request, never from the evaluator's own output.

use ix_trace_rs::trace;
use qsl_eval::temporal::evidence::{
    EvidenceCounter, EvidenceIncomplete, EvidencePackage, EvidenceRef, EvidenceRequest,
    EvidenceResult, ResultSubject, TemporalEvidenceLimits, Truth,
};
use qsl_eval::temporal::Valuation;
use qsl_package::{
    AdmittedPackageRead, CheckedNodeId, CheckedOccurrence, CheckedOccurrenceRole, CheckedPackageV2,
};

use super::checked_handoff::KINDS;
use super::temporal_package_route::{
    clause_ids, event_clauses, recipe, request, run, twins, Built,
};

fn claim(ordinal: u64) -> CheckedOccurrence {
    CheckedOccurrence {
        role: CheckedOccurrenceRole::Claim,
        ordinal,
    }
}

fn read_back<'a>(read: &'a AdmittedPackageRead<'a>) -> EvidencePackage<'a> {
    EvidencePackage::from_read(
        read,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("the read-back package")
}

/// The subject the package and the selected occurrence give.
fn expected_subject(
    package: &CheckedPackageV2,
    clause: &CheckedNodeId,
    occurrence: u64,
) -> ResultSubject {
    let source_map = package
        .source_map()
        .iter()
        .find(|entry| {
            entry.node_id == *clause
                && entry.role == CheckedOccurrenceRole::Claim
                && entry.ordinal == occurrence
        })
        .expect("the occurrence's source-map entry")
        .clone();
    ResultSubject {
        package_identity: package.name().to_owned(),
        package_version: quire_contract_model::CHECKED_PACKAGE_V2.to_owned(),
        clause: clause.clone(),
        occurrence: claim(occurrence),
        source_map,
    }
}

/// Every support entry of `result` names a request position and an atom with
/// an admitted valuation there, and its position's observation byte for byte.
fn assert_support_is_admitted(asked: &EvidenceRequest, result: &EvidenceResult) {
    let mut seen = Vec::new();
    for entry in &result.support {
        let position = &asked.positions[entry.position];
        assert_eq!(entry.observation, position.observation);
        let valued = |found: Option<&Valuation>| {
            found.is_some_and(|valuation| *valuation != Valuation::Missing)
        };
        assert!(
            valued(position.valuations.get(&entry.atom))
                || asked.trigger.as_ref().is_some_and(|trigger| valued(
                    position
                        .instance_valuations
                        .get(trigger)
                        .and_then(|valuations| valuations.get(&entry.atom))
                )),
            "{entry:?} names no admitted valuation"
        );
        assert!(
            !seen.contains(&(entry.position, entry.atom)),
            "{entry:?} appears twice"
        );
        seen.push((entry.position, entry.atom));
    }
}

/// FR-526-AC-20 (TC-1321 row 20): over each of recipe R's three packages an
/// `Original` call returns a `subject` equal to the package identity,
/// version, clause, occurrence and occurrence source-map entry read from the
/// package, and the request's `correspondence` member for member; a call
/// stopped one below a counter's usage carries the same `subject`.
#[trace("FR-526-AC-20", "TC-1321")]
#[test]
fn every_result_names_the_checked_subject_it_was_evaluated_for() {
    let mut called = 0;
    for kind in KINDS {
        let built = recipe(kind);
        let read = built.read(&built.emission);
        let package = read_back(&read);
        for clause in clause_ids(&package) {
            let view = package.temporal_subject(&clause).expect("reads");
            let [only] = view.occurrences() else {
                continue;
            };
            let ordinal = only.ordinal;
            let expected = expected_subject(read.admitted(), &clause, ordinal);
            let mut asked = request(&package, &clause, None, 3);
            asked.correspondence.authority_revision = "revision-7".to_owned();
            let (result, _) = run(&package, &clause, &asked);
            let Ok(result) = result else {
                continue;
            };
            called += 1;
            assert_eq!(result.subject, expected, "{kind}");
            assert_eq!(result.correspondence, asked.correspondence, "{kind}");
            assert_eq!(
                result.correspondence.authority_revision,
                "revision-7".to_owned()
            );

            // One below a counter's usage stops the call; the stopped
            // result carries the same subject.
            let (counter, used) = [EvidenceCounter::Positions, EvidenceCounter::Valuations]
                .into_iter()
                .map(|counter| (counter, result.usage.get(counter)))
                .find(|(_, used)| *used > 0)
                .expect("a counter the call charged");
            let mut stopped = asked.clone();
            stopped.limits = TemporalEvidenceLimits::default().with_ceiling(counter, used - 1);
            let (stopped, _) = run(&package, &clause, &stopped);
            let stopped = stopped.expect("an Ok result");
            assert!(
                matches!(stopped.incomplete, Some(EvidenceIncomplete::Limit(_))),
                "{kind}: {:?}",
                stopped.incomplete
            );
            assert_eq!(stopped.subject, expected, "{kind}");
        }
    }
    assert!(called >= KINDS.len(), "every package answered a call");
}

/// FR-526-AC-21 (TC-1321 row 21): over the twin package, two calls with equal
/// positions, valuations and correspondence, each with its own occurrence's
/// clock row, selecting occurrences 3 and 7 return subjects that agree in
/// package identity, version and clause and differ in occurrence.
#[trace("FR-526-AC-21", "TC-1321")]
#[test]
fn twin_occurrences_return_subjects_that_differ_only_in_occurrence() {
    let built = twins();
    let read = built.read(&built.emission);
    let package = read_back(&read);
    let [node] = clause_ids(&package).try_into().expect("one shared node");
    let mut subjects = Vec::new();
    for ordinal in [3, 7] {
        let asked = request(&package, &node, Some(ordinal), 3);
        let (result, _) = run(&package, &node, &asked);
        let result = result.expect("a result");
        assert_eq!(
            result.subject,
            expected_subject(read.admitted(), &node, ordinal)
        );
        subjects.push(result.subject);
    }
    let [three, seven] = &subjects[..] else {
        panic!("two subjects");
    };
    assert_eq!(three.package_identity, seven.package_identity);
    assert_eq!(three.package_version, seven.package_version);
    assert_eq!(three.clause, seven.clause);
    assert_eq!(three.occurrence, claim(3));
    assert_eq!(seven.occurrence, claim(7));
    assert_ne!(three.occurrence, seven.occurrence);
}

/// The unit of FR-526-AC-22: `eventually[0,1] (holds(a) and holds(b))`.
const BOTH: &str = "temporal Both using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin { eventually[0,1] (holds(c.versionNumber <= 1) and holds(c.versionNumber >= 0)) }";

fn both() -> Built {
    event_clauses(BOTH, &[("Both", "both-authority")])
}

/// `a` and `b` true at position 0 and false at position 1.
fn both_request(package: &EvidencePackage<'_>, clause: &CheckedNodeId) -> EvidenceRequest {
    let mut asked = request(package, clause, None, 2);
    let atoms = package
        .subject(clause, None)
        .expect("reads")
        .facts
        .formula()
        .atom_count();
    assert_eq!(atoms, 2);
    for (index, position) in asked.positions.iter_mut().enumerate() {
        let value = if index == 0 {
            Valuation::True
        } else {
            Valuation::False
        };
        position.valuations = [(0, value.clone()), (1, value)].into();
    }
    asked.completeness.as_mut().expect("a record").state =
        qsl_eval::temporal::ProgressCompleteness::Complete { through: 1 };
    asked
}

/// FR-526-AC-22 (TC-1321 row 22): the result is `True` and `support` is
/// exactly the two entries (position 0, `a`) and (position 0, `b`), each
/// naming position 0's observation; a reading that merges entries by
/// position, or by observation, gives one entry and fails.
#[trace("FR-526-AC-22", "TC-1321")]
#[test]
fn two_atoms_of_one_position_are_two_support_entries() {
    let built = both();
    let read = built.read(&built.emission);
    let package = read_back(&read);
    let [clause] = clause_ids(&package).try_into().expect("one clause");
    let asked = both_request(&package, &clause);
    let (result, _) = run(&package, &clause, &asked);
    let result = result.expect("a result");
    assert_eq!(result.truth, Some(Truth::True));
    let mut entries: Vec<_> = result
        .support
        .iter()
        .map(|entry| (entry.position, entry.atom, entry.observation.clone()))
        .collect();
    entries.sort_by_key(|(position, atom, _)| (*position, *atom));
    assert_eq!(
        entries,
        [
            (0, 0, EvidenceRef::named("obs0")),
            (0, 1, EvidenceRef::named("obs0")),
        ]
    );
    assert_support_is_admitted(&asked, &result);
}

/// FR-526-AC-23 (TC-1321 row 23): with `b`'s valuation at position 0
/// withdrawn, no `support` entry names a pair the request admitted no
/// valuation for, and every entry of every result names a request position
/// and an atom with an admitted valuation there.
#[trace("FR-526-AC-23", "TC-1321")]
#[test]
fn no_support_entry_names_a_pair_without_an_admitted_valuation() {
    let built = both();
    let read = built.read(&built.emission);
    let package = read_back(&read);
    let [clause] = clause_ids(&package).try_into().expect("one clause");
    let mut asked = both_request(&package, &clause);
    // With `b` valued the verdict rests on the pair (position 0, `b`) ...
    let (valued, _) = run(&package, &clause, &asked);
    let valued = valued.expect("a result");
    assert_eq!(valued.truth, Some(Truth::True));
    assert!(valued
        .support
        .iter()
        .any(|entry| entry.position == 0 && entry.atom == 1));
    // ... and with it withdrawn the same decision still names the pair the
    // evaluator's support rule selects, so only the admission filter removes it.
    asked.positions[0].valuations.remove(&1);
    let (result, _) = run(&package, &clause, &asked);
    let result = result.expect("a result");
    assert_eq!(result.truth, None);
    assert!(result
        .support
        .iter()
        .all(|entry| !(entry.position == 0 && entry.atom == 1)));
    assert_support_is_admitted(&asked, &result);

    // A Missing valuation is no admitted valuation either.
    let mut missing = both_request(&package, &clause);
    missing.positions[0]
        .valuations
        .insert(1, Valuation::Missing);
    let (result, _) = run(&package, &clause, &missing);
    let result = result.expect("a result");
    assert_support_is_admitted(&missing, &result);
}

/// A package whose source map lacks an occurrence's entry is never admitted,
/// so no subject read finds an occurrence without one: the reader refuses it
/// as `InvalidSourceMap` at `/source_map`.
#[trace("FR-526-AC-20", "TC-1321")]
#[test]
fn a_package_without_an_occurrences_source_map_entry_is_not_admitted() {
    let built = both();
    let mut wire: serde_json::Value =
        serde_json::from_slice(built.emission.package().bytes()).expect("JSON");
    let before = wire["source_map"].as_array().expect("entries").len();
    wire["source_map"]
        .as_array_mut()
        .expect("entries")
        .retain(|entry| entry["role"] != "claim");
    assert!(wire["source_map"].as_array().expect("entries").len() < before);
    let mutated = built.with_wire(&wire);
    let refused = mutated
        .try_read(&mutated.emission)
        .err()
        .expect("the reader refuses");
    let shown = format!("{refused:?}");
    assert!(shown.contains("InvalidSourceMap"), "{shown}");
    assert!(shown.contains("/source_map"), "{shown}");
}
