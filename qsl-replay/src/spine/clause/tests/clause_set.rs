// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1039 (FR-107-AC-13 to AC-16, AC-18): a clause set reports its results
//! together over FR-108's `Config` unit, with the four added clauses on
//! `ConfigVersion::attemptUpdate`.
//!
//! `UpdateRaises` is written `self.versionNumber - pre(self.versionNumber) =
//! 1`: TC-1039's `= pre(..) + 1` is a `TypeMismatch` between `Int[0, 1000]`
//! and `Int[1, 1001]` on this unit. Step 6 (`UpdateDivides`, FR-107-AC-18) is
//! not built: the unit refuses it `AmbiguousLiteral` in every form tried, and
//! a `value(..)` of an optional not proved present is refused
//! `Unproved(Presence)`, so no clause reached an `Undefined` evaluation.

use super::frame_replay::{unit_for, Unit};
use super::*;
use qsl_eval::value::{ClauseSetEvaluation, ClauseSetOutcome};

const CLAUSES: &str = "\
pre UpdateAllowed using v on Config::ConfigVersion::attemptUpdate { self.versionNumber < 3 }\n\
pre UpdateNotMax using v on Config::ConfigVersion::attemptUpdate { self.versionNumber < 1000 }\n\
\
post UpdateRaises using v on Config::ConfigVersion::attemptUpdate { self.versionNumber - pre(self.versionNumber) = 1 }\n";

fn unit() -> Unit {
    let (base, _) = config_version_unit_and_packages();
    unit_for(format!("{base}{CLAUSES}"), config_version_domain_document())
}

/// The documents of the TC-1039 invocations and the current snapshot.
pub(super) struct Documents {
    pub(super) snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    pub(super) invocations: BTreeMap<[u8; 32], Vec<u8>>,
}

impl Documents {
    /// An `attemptUpdate` invocation on `child` whose version goes `from` to
    /// `to`, with `root` at 0 in both snapshots.
    pub(super) fn invocation(&mut self, name: &str, from: i64, to: i64) -> DocumentRef {
        let digest = config_version_model_digest_hex();
        let (pre, pre_bytes) = document_ref_and_bytes(&format!("{name}-pre"), |label| {
            config_version_invocation_snapshot(label, &digest, "pre", 0, from)
        });
        let (post, post_bytes) = document_ref_and_bytes(&format!("{name}-post"), |label| {
            config_version_invocation_snapshot(label, &digest, "post", 0, to)
        });
        let (invocation, bytes) = document_ref_and_bytes(name, |label| {
            config_version_invocation_bytes(label, &digest, &pre, &post, "child")
        });
        self.snapshots.insert(pre.digest, pre_bytes);
        self.snapshots.insert(post.digest, post_bytes);
        self.invocations.insert(invocation.digest, bytes);
        invocation
    }

    /// The current snapshot `cur`: `root` 0 and `child` 1.
    fn current(&mut self) -> DocumentRef {
        self.current_snapshot("cur", 0, Some(1))
    }

    /// A current snapshot named `name`: `root` at `root_version`, and `child`
    /// at `child_version` naming `root` when there is one.
    pub(super) fn current_snapshot(
        &mut self,
        name: &str,
        root_version: i64,
        child_version: Option<i64>,
    ) -> DocumentRef {
        let digest = config_version_model_digest_hex();
        let (current, bytes) = document_ref_and_bytes(name, |label| {
            config_version_snapshot(label, &digest, root_version, child_version)
        });
        self.snapshots.insert(current.digest, bytes);
        current
    }
}

pub(super) fn documents() -> Documents {
    Documents {
        snapshots: BTreeMap::new(),
        invocations: BTreeMap::new(),
    }
}

/// `clause` admitted by FR-106 over `selection`.
pub(super) fn admit(
    unit: &Unit,
    documents: &Documents,
    clause: &str,
    selection: ClauseSelection,
) -> AdmittedObservations {
    let packages = qsl_semantics::model::intake::package_input([unit.domain_document.as_slice()]);
    let graph = unit.compiled.package.graph();
    let declaration = graph.state_clause(clause).expect("the clause is declared");
    admit_clause_observations(
        graph,
        declaration,
        &packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        &Provisions {
            snapshots: &documents.snapshots,
            invocations: &documents.invocations,
        },
        &selection,
        ObservationLimits::default(),
    )
    .expect("the observations admit")
}

/// Each named clause admitted over `invocation`, in the order named.
fn over(
    unit: &Unit,
    documents: &Documents,
    invocation: &DocumentRef,
    names: &[&str],
) -> Vec<(QualifiedName, AdmittedObservations)> {
    names
        .iter()
        .map(|name| {
            let selection = config_version_invocation_selection(name, invocation.clone());
            (
                QualifiedName::unqualified(*name).unwrap(),
                admit(unit, documents, name, selection),
            )
        })
        .collect()
}

fn evaluate(
    unit: &Unit,
    set: &[(QualifiedName, AdmittedObservations)],
    work_units: u64,
) -> Result<ClauseSetEvaluation, CallFailure> {
    let borrowed: Vec<_> = set.iter().map(|(name, o)| (name.clone(), o)).collect();
    unit.compiled
        .package
        .evaluate_clause_set(&borrowed, default_accounting(work_units))
}

/// The entries as (name, completed Boolean or none).
fn entries(evaluation: &ClauseSetEvaluation) -> Vec<(String, Option<bool>)> {
    evaluation
        .results
        .iter()
        .map(|entry| {
            let completed = match &entry.evaluation.evaluation.outcome {
                FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(value))) => Some(*value),
                _ => None,
            };
            (entry.name.to_string(), completed)
        })
        .collect()
}

fn expected(entries: &[(&str, Option<bool>)]) -> Vec<(String, Option<bool>)> {
    entries
        .iter()
        .map(|(name, value)| ((*name).to_owned(), *value))
        .collect()
}

const UNLIMITED: u64 = 1_000_000;

/// TC-1039 step 1.
#[trace("TC-1039", "FR-107-AC-13")]
#[test]
fn a_set_reports_both_results_when_the_precondition_holds_and_one_when_it_does_not() {
    let unit = unit();
    let mut docs = documents();
    let (up12, up34) = (docs.invocation("up12", 1, 2), docs.invocation("up34", 3, 4));
    let names = ["UpdateAllowed", "UpdateRaises"];

    let held = evaluate(&unit, &over(&unit, &docs, &up12, &names), UNLIMITED).unwrap();
    assert_eq!(
        entries(&held),
        expected(&[("UpdateAllowed", Some(true)), ("UpdateRaises", Some(true))])
    );
    assert_eq!(held.outcome, ClauseSetOutcome::True);

    let refused = evaluate(&unit, &over(&unit, &docs, &up34, &names), UNLIMITED).unwrap();
    assert_eq!(
        entries(&refused),
        expected(&[("UpdateAllowed", Some(false))])
    );
    assert_eq!(refused.outcome, ClauseSetOutcome::False);

    // The slice order does not change a result.
    let reversed = over(&unit, &docs, &up12, &["UpdateRaises", "UpdateAllowed"]);
    assert_eq!(
        entries(&evaluate(&unit, &reversed, UNLIMITED).unwrap()),
        entries(&held)
    );
}

/// TC-1039 steps 2 and 3.
#[trace("TC-1039", "FR-107-AC-14")]
#[test]
fn a_set_orders_its_entries_by_kind_then_name_and_skips_postconditions_after_a_false_precondition()
{
    let unit = unit();
    let mut docs = documents();
    let (up12, up34) = (docs.invocation("up12", 1, 2), docs.invocation("up34", 3, 4));
    let names = ["UpdateRaises", "UpdateNotMax", "UpdateAllowed"];

    let held = evaluate(&unit, &over(&unit, &docs, &up12, &names), UNLIMITED).unwrap();
    assert_eq!(
        entries(&held),
        expected(&[
            ("UpdateAllowed", Some(true)),
            ("UpdateNotMax", Some(true)),
            ("UpdateRaises", Some(true)),
        ])
    );
    assert_eq!(held.outcome, ClauseSetOutcome::True);

    let refused = evaluate(&unit, &over(&unit, &docs, &up34, &names), UNLIMITED).unwrap();
    assert_eq!(
        entries(&refused),
        expected(&[("UpdateAllowed", Some(false)), ("UpdateNotMax", Some(true))])
    );
    assert_eq!(refused.outcome, ClauseSetOutcome::False);

    // Step 3: the invariant `NoCycle` over its own observation comes first.
    let current = docs.current();
    let mut set = over(&unit, &docs, &up34, &["UpdateRaises", "UpdateAllowed"]);
    set.push((
        QualifiedName::unqualified("NoCycle").unwrap(),
        admit(
            &unit,
            &docs,
            "NoCycle",
            config_version_current_selection("NoCycle", current, "child"),
        ),
    ));
    let with_invariant = evaluate(&unit, &set, UNLIMITED).unwrap();
    assert_eq!(
        entries(&with_invariant),
        expected(&[("NoCycle", Some(true)), ("UpdateAllowed", Some(false))])
    );
    assert_eq!(with_invariant.outcome, ClauseSetOutcome::False);
}

/// TC-1039 step 4.
#[trace("TC-1039", "FR-107-AC-15")]
#[test]
fn a_set_stops_at_an_incomplete_clause_and_an_empty_set_is_true() {
    let unit = unit();
    let mut docs = documents();
    let up12 = docs.invocation("up12", 1, 2);
    let names = ["UpdateAllowed", "UpdateRaises"];
    let set = over(&unit, &docs, &up12, &names);

    let exhausted = evaluate(&unit, &set, 0).unwrap();
    assert_eq!(exhausted.results.len(), 1);
    assert_eq!(exhausted.results[0].name.to_string(), "UpdateAllowed");
    // The denied charge is `resource_exhausted`/`insufficient-next-charge`:
    // the first work-units charge against a zero budget.
    let FamilyOutcome::Evaluated(Outcome::Incomplete(denied)) =
        &exhausted.results[0].evaluation.evaluation.outcome
    else {
        panic!("a zero work budget leaves the clause incomplete");
    };
    assert_eq!(denied.limit_kind, quire_exact::LimitKind::WorkUnits);
    assert_eq!(denied.limit, 0);
    assert_eq!(exhausted.outcome, ClauseSetOutcome::Incomplete);

    let empty = evaluate(&unit, &[], UNLIMITED).unwrap();
    assert!(empty.results.is_empty());
    assert_eq!(empty.outcome, ClauseSetOutcome::True);

    let first = evaluate(&unit, &set, UNLIMITED).unwrap();
    let second = evaluate(&unit, &set, UNLIMITED).unwrap();
    assert_eq!(entries(&first), entries(&second));
    assert_eq!(first.outcome, second.outcome);
    for (left, right) in first.results.iter().zip(&second.results) {
        assert_eq!(
            format!("{:?}", left.evaluation.evaluation),
            format!("{:?}", right.evaluation.evaluation)
        );
    }
}

/// TC-1039 step 5, the unknown-name and mismatch refusals (the duplicate
/// refusal needs `InputRefusal::DuplicateClause`, which is not in this test).
#[trace("TC-1039")]
#[test]
fn a_set_naming_an_unknown_clause_or_mismatched_observations_is_refused_whole() {
    let unit = unit();
    let mut docs = documents();
    let up12 = docs.invocation("up12", 1, 2);
    let allowed = over(&unit, &docs, &up12, &["UpdateAllowed"]);
    let raises = over(&unit, &docs, &up12, &["UpdateRaises"]);

    let unknown = vec![
        allowed[0].clone(),
        (
            QualifiedName::unqualified("sameIdentity").unwrap(),
            allowed[0].1.clone(),
        ),
    ];
    assert!(matches!(
        evaluate(&unit, &unknown, UNLIMITED),
        Err(CallFailure::Input(InputRefusal::UnknownClause(name))) if name == "sameIdentity"
    ));

    let mismatched = vec![(allowed[0].0.clone(), raises[0].1.clone())];
    assert!(matches!(
        evaluate(&unit, &mismatched, UNLIMITED),
        Err(CallFailure::Input(InputRefusal::ObservationsMismatch))
    ));
}

/// TC-1039 step 5, the duplicate refusal. Needs `InputRefusal::DuplicateClause`
/// from quire-semantic-value.
#[trace("TC-1039", "FR-107-AC-16")]
#[test]
fn a_set_naming_a_clause_twice_is_refused_naming_it() {
    let unit = unit();
    let mut docs = documents();
    let up12 = docs.invocation("up12", 1, 2);
    let allowed = over(&unit, &docs, &up12, &["UpdateAllowed"]);
    let twice = vec![allowed[0].clone(), allowed[0].clone()];
    assert!(matches!(
        evaluate(&unit, &twice, UNLIMITED),
        Err(CallFailure::Input(InputRefusal::DuplicateClause(name))) if name == "UpdateAllowed"
    ));
}
