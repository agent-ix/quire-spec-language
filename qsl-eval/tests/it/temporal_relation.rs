// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-526 (TC-1321): a temporal evidence result chains to its direct
//! predecessor, and a relation that changes nothing, names a result that is
//! not the latest or changes the obligation is refused.

use ix_trace_rs::trace;
use qsl_eval::temporal::evidence::{
    evaluate_temporal_evidence, EmptyIdentity, EvidenceCounter, EvidenceIncomplete,
    EvidenceRequest, EvidenceResult, Relation, SemanticTriggerIdentity, TemporalEvidenceLimits,
    Truth,
};
use qsl_eval::temporal::Valuation;
use quire_eval::CallFailure;

use crate::temporal_activation::{
    delivery, env, expect_refusal, id, position, request, unlimited, Env,
};

fn setup() -> (Env, u64) {
    let env = env(
        "on each (q: Counter)",
        "capture amount: Int[0, 9] = q.level; eventually[0, 1] holds(p)",
    );
    let p = env.atom("p");
    (env, p)
}

/// A request for `t1` whose position 1 reads `holds(p)` as `later`: changing
/// `later` changes the evidence.
fn asking(env: &Env, p: u64, later: Valuation) -> EvidenceRequest {
    let mut asked = request(
        env,
        vec![
            position(0, &[(p, Valuation::False)]),
            position(1, &[(p, later)]),
        ],
    );
    asked.deliveries = vec![delivery("t1", "r", "payload", &["5"])];
    asked.trigger = Some(id("t1"));
    asked
}

fn run(env: &Env, asked: &EvidenceRequest) -> Result<EvidenceResult, CallFailure> {
    evaluate_temporal_evidence(&env.subject(&[]), asked, &mut unlimited())
}

fn correct(
    env: &Env,
    p: u64,
    later: Valuation,
    previous: &EvidenceResult,
    make: fn(qsl_eval::temporal::evidence::ResultIdentity) -> Relation,
) -> EvidenceRequest {
    let mut asked = asking(env, p, later);
    asked.relation = make(previous.result_identity.clone());
    asked.ledger = Some(previous.ledger.clone());
    asked
}

#[trace("FR-526-AC-1", "FR-516-AC-6", "TC-1321", "TC-1311")]
#[test]
fn an_original_result_starts_a_chain_at_lineage_zero_and_is_recorded() {
    let (env, p) = setup();
    let r1 = run(&env, &asking(&env, p, Valuation::True)).expect("a result");
    assert_eq!(r1.lineage, 0);
    assert_eq!(r1.relation, Relation::Original);
    let entry = r1
        .ledger
        .result(&env.obligation(), Some(&id("t1")))
        .expect("the entry of the key");
    assert_eq!(entry.result_identity(), &r1.result_identity);
    assert_eq!(entry.lineage(), 0);
}

#[trace(
    "FR-526-AC-2",
    "FR-526-AC-3",
    "FR-526-AC-14",
    "FR-526-AC-19",
    "FR-516-AC-6",
    "TC-1321",
    "TC-1311"
)]
#[test]
fn a_chain_of_corrections_names_each_direct_predecessor_and_leaves_earlier_results() {
    let (env, p) = setup();
    let r1 = run(&env, &asking(&env, p, Valuation::True)).expect("a result");
    let r1_identity = r1.result_identity.clone();
    let r2 = run(
        &env,
        &correct(&env, p, Valuation::False, &r1, Relation::Superseding),
    )
    .expect("a result");
    assert_eq!(
        (r2.lineage, &r2.relation),
        (1, &Relation::Superseding(r1_identity.clone()))
    );
    assert_eq!(r1.result_identity, r1_identity);
    assert_eq!(r1.lineage, 0);
    let latest = r2
        .ledger
        .result(&env.obligation(), Some(&id("t1")))
        .expect("the latest");
    assert_eq!(latest.result_identity(), &r2.result_identity);
    let r3 = run(
        &env,
        &correct(&env, p, Valuation::True, &r2, Relation::Superseding),
    )
    .expect("a result");
    assert_eq!(r3.lineage, 2);
    assert_eq!(
        r3.relation,
        Relation::Superseding(r2.result_identity.clone())
    );
    for result in [&r1, &r2, &r3] {
        assert_eq!(result.trigger, Some(id("t1")));
        assert_eq!(
            result
                .trigger
                .as_ref()
                .map(SemanticTriggerIdentity::as_bytes),
            Some(b"t1".as_slice())
        );
    }
    assert_eq!((r1.lineage, r2.lineage, r3.lineage), (0, 1, 2));
}

#[trace("FR-526-AC-4", "FR-526-AC-5", "TC-1321")]
#[test]
fn invalidating_derives_its_truth_from_its_own_inputs_like_superseding() {
    let (env, p) = setup();
    let r1 = run(&env, &asking(&env, p, Valuation::True)).expect("a result");
    let superseding = run(
        &env,
        &correct(&env, p, Valuation::False, &r1, Relation::Superseding),
    )
    .expect("a result");
    let invalidating = run(
        &env,
        &correct(&env, p, Valuation::False, &r1, Relation::Invalidating),
    )
    .expect("a result");
    assert_eq!(
        invalidating.relation,
        Relation::Invalidating(r1.result_identity.clone())
    );
    assert_eq!(invalidating.lineage, superseding.lineage);
    assert_eq!(invalidating.truth, superseding.truth);
    assert_eq!(invalidating.settlement, superseding.settlement);
    assert_eq!(invalidating.support.len(), superseding.support.len());
    assert_eq!(superseding.truth, Some(Truth::False));
    assert_ne!(invalidating.result_identity, superseding.result_identity);
}

#[trace("FR-526-AC-6", "FR-526-AC-7", "FR-516-AC-13", "TC-1321", "TC-1311")]
#[test]
fn a_relation_that_changes_nothing_or_has_no_ledger_is_refused_uncharged() {
    let (env, p) = setup();
    let r1 = run(&env, &asking(&env, p, Valuation::True)).expect("a result");
    for make in [Relation::Superseding, Relation::Invalidating] {
        let same = correct(&env, p, Valuation::True, &r1, make);
        let mut meter = unlimited();
        let refused = evaluate_temporal_evidence(&env.subject(&[]), &same, &mut meter);
        expect_refusal(refused, "unchanged-input");
        assert!(meter.admitted_charges().is_empty());
    }
    let mut bare = correct(&env, p, Valuation::False, &r1, Relation::Superseding);
    bare.ledger = None;
    expect_refusal(run(&env, &bare), "no-ledger");
}

#[trace("FR-526-AC-8", "FR-526-AC-15", "TC-1321")]
#[test]
fn only_the_latest_result_can_be_corrected_and_an_original_restarts_the_chain() {
    let (env, p) = setup();
    let r1 = run(&env, &asking(&env, p, Valuation::True)).expect("a result");
    let r2 = run(
        &env,
        &correct(&env, p, Valuation::False, &r1, Relation::Superseding),
    )
    .expect("a result");
    let mut stale = correct(&env, p, Valuation::Missing, &r1, Relation::Superseding);
    stale.ledger = Some(r2.ledger.clone());
    expect_refusal(run(&env, &stale), "predecessor-not-latest");
    let mut latest = correct(&env, p, Valuation::Missing, &r2, Relation::Superseding);
    latest.ledger = Some(r2.ledger.clone());
    assert!(run(&env, &latest).is_ok());
    let mut restart = asking(&env, p, Valuation::Missing);
    restart.ledger = Some(r2.ledger.clone());
    let restarted = run(&env, &restart).expect("a result");
    assert_eq!(restarted.lineage, 0);
    let entry = restarted
        .ledger
        .result(&env.obligation(), Some(&id("t1")))
        .expect("replaced");
    assert_eq!(entry.result_identity(), &restarted.result_identity);
}

#[trace("FR-526-AC-9", "FR-526-AC-10", "FR-526-AC-11", "TC-1321")]
#[test]
fn another_obligation_trigger_or_correspondence_is_not_the_predecessors() {
    let (env, p) = setup();
    let r1 = run(&env, &asking(&env, p, Valuation::True)).expect("a result");
    let other_clause = setup().0.moved("recipe", "Other");
    let asked = correct(
        &other_clause,
        p,
        Valuation::False,
        &r1,
        Relation::Superseding,
    );
    expect_refusal(run(&other_clause, &asked), "predecessor-unknown");
    let other_package = setup().0.moved("elsewhere", "Due");
    let asked = correct(
        &other_package,
        p,
        Valuation::False,
        &r1,
        Relation::Superseding,
    );
    expect_refusal(run(&other_package, &asked), "predecessor-unknown");
    let mut byte = correct(&env, p, Valuation::False, &r1, Relation::Superseding);
    byte.deliveries = vec![delivery("t2", "r", "payload", &["5"])];
    byte.trigger = Some(id("t2"));
    expect_refusal(run(&env, &byte), "predecessor-unknown");
    let mut corr = correct(&env, p, Valuation::False, &r1, Relation::Superseding);
    corr.correspondence.identity = "another".to_owned();
    expect_refusal(run(&env, &corr), "correspondence-changed");
}

#[trace("FR-526-AC-12", "TC-1321")]
#[test]
fn the_lineage_ceiling_stops_the_chain_at_the_named_counter() {
    let (env, p) = setup();
    let values = [
        Valuation::True,
        Valuation::False,
        Valuation::Missing,
        Valuation::True,
    ];
    let chain = |ceiling: u64| {
        let mut previous: Option<EvidenceResult> = None;
        for (index, value) in values.iter().enumerate() {
            let mut asked = match &previous {
                None => asking(&env, p, value.clone()),
                Some(earlier) => correct(&env, p, value.clone(), earlier, Relation::Superseding),
            };
            asked.limits =
                TemporalEvidenceLimits::default().with_ceiling(EvidenceCounter::Lineage, ceiling);
            let result = run(&env, &asked).expect("a result");
            if index == 3 {
                return result;
            }
            previous = Some(result);
        }
        unreachable!("four requests")
    };
    let stopped = chain(2);
    assert_eq!(stopped.lineage, 3);
    assert_eq!(stopped.truth, None);
    match stopped.incomplete {
        Some(EvidenceIncomplete::Limit(reached)) => {
            assert_eq!(reached.counter, EvidenceCounter::Lineage);
        }
        other => panic!("a lineage stop, not {other:?}"),
    }
    assert_eq!(chain(3).lineage, 3);
}

#[trace("FR-526-AC-16", "TC-1321")]
#[test]
fn equal_requests_give_equal_results_ledgers_and_charge_logs() {
    let (env, p) = setup();
    let r1 = run(&env, &asking(&env, p, Valuation::True)).expect("a result");
    let asked = correct(&env, p, Valuation::False, &r1, Relation::Superseding);
    let (mut first, mut second) = (unlimited(), unlimited());
    let one = evaluate_temporal_evidence(&env.subject(&[]), &asked, &mut first).expect("a result");
    let two = evaluate_temporal_evidence(&env.subject(&[]), &asked, &mut second).expect("a result");
    assert_eq!(one.result_identity, two.result_identity);
    assert_eq!(one.ledger, two.ledger);
    assert_eq!(first.admitted_charges(), second.admitted_charges());
}

#[trace("FR-526-AC-18", "FR-516-AC-4", "TC-1321", "TC-1311")]
#[test]
fn an_empty_identity_is_refused_and_one_byte_changes_the_result_identity() {
    assert_eq!(SemanticTriggerIdentity::new(Vec::new()), Err(EmptyIdentity));
    let (env, p) = setup();
    let one = run(&env, &asking(&env, p, Valuation::True)).expect("a result");
    let mut other = asking(&env, p, Valuation::True);
    other.deliveries = vec![delivery("t\u{2}", "r", "payload", &["5"])];
    other.trigger = Some(id("t\u{2}"));
    let two = run(&env, &other).expect("a result");
    assert_ne!(one.result_identity, two.result_identity);
}

#[trace(
    "FR-502-AC-12",
    "FR-526-AC-9",
    "FR-524-AC-19",
    "TC-1296",
    "TC-1321",
    "TC-1319"
)]
#[test]
fn two_occurrences_of_one_body_keep_separate_instances_results_and_progress() {
    let (first, p) = setup();
    let second = setup().0.occurrence(1);
    // Retain `t1` and a progress entry under occurrence 0.
    let mut one = asking(&first, p, Valuation::True);
    one.progress_ledger = Some(qsl_eval::temporal::ProgressLedger::new());
    let r1 = run(&first, &one).expect("a result");
    // Occurrence 1 under the same ledgers: `t1` with another payload is not a
    // contradiction, a lower watermark is not a regression, and the result
    // of occurrence 0 is not its predecessor.
    let mut other = asking(&second, p, Valuation::True);
    other.deliveries[0].payload = qsl_eval::temporal::evidence::Payload::new(b"another".as_slice());
    other.ledger = Some(r1.ledger.clone());
    other.progress_ledger = r1.progress_ledger.clone();
    other.decision_progress = Some(50);
    other.completeness = other.completeness.map(|mut record| {
        record.state = qsl_eval::temporal::ProgressCompleteness::Complete { through: 50 };
        record
    });
    let r2 = run(&second, &other).expect("a separate occurrence is accepted");
    assert_eq!(r2.lineage, 0);
    assert_eq!(r2.ledger.instance_count(), 2);
    assert_eq!(r2.ledger.result_count(), 2);
    assert_eq!(
        r2.progress_ledger.as_ref().map(|ledger| ledger.len()),
        Some(2)
    );
    let mut correcting = asking(&second, p, Valuation::False);
    correcting.relation = Relation::Superseding(r1.result_identity.clone());
    correcting.ledger = Some(r2.ledger.clone());
    expect_refusal(run(&second, &correcting), "predecessor-not-latest");
}
