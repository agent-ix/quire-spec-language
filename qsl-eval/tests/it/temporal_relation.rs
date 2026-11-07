// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-526 (TC-1321): a temporal evidence result chains to its direct
//! predecessor, and a relation that changes nothing, names a result that is
//! not the latest or changes the obligation is refused.

use ix_trace_rs::trace;
use qsl_eval::temporal::evidence::{
    evaluate_temporal_evidence, EmptyIdentity, EvidenceRequest, EvidenceResult, Relation,
    SemanticTriggerIdentity, Truth,
};
use qsl_eval::temporal::Valuation;
use quire_eval::CallFailure;
use quire_exact::LimitKind;

use crate::temporal_activation::{
    delivery, env, expect_refusal, id, origin, position, request, unlimited, Env,
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
    "FR-052-AC-12",
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
    // The ledger of the first call still holds the first result as the
    // latest of its own chain: the later call wrote another ledger.
    let own = r1
        .ledger
        .result(&env.obligation(), Some(&id("t1")))
        .expect("the first ledger's entry");
    assert_eq!(own.result_identity(), &r1_identity);
    assert_eq!(own.lineage(), 0);
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

#[trace(
    "FR-052-AC-12",
    "FR-526-AC-6",
    "FR-526-AC-7",
    "FR-516-AC-13",
    "TC-1321",
    "TC-1311"
)]
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

#[trace("FR-052-AC-12", "FR-526-AC-8", "FR-526-AC-15", "TC-1321")]
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

#[trace(
    "FR-052-AC-12",
    "FR-053-AC-8",
    "FR-053-AC-9",
    "FR-526-AC-9",
    "FR-526-AC-10",
    "FR-526-AC-11",
    "TC-1321"
)]
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

/// FR-526-AC-24 (TC-1321 row 24): a result `r1` and a result `r1'` of a
/// request that differs in one valuation, so that their truths differ. A
/// `Superseding(r1)` request carrying the ledger `r1'`'s call returned is
/// refused `predecessor-not-latest`; a `Superseding(r1')` request with
/// changed inputs carrying that ledger is accepted.
#[trace("FR-526-AC-24", "TC-1321")]
#[test]
fn only_the_ledger_entry_the_evaluator_wrote_for_the_predecessor_grants_authority() {
    let (env, p) = setup();
    let r1 = run(&env, &asking(&env, p, Valuation::True)).expect("a result");
    let other = run(&env, &asking(&env, p, Valuation::False)).expect("a result");
    assert_ne!(r1.truth, other.truth);
    let mut stale = correct(&env, p, Valuation::Missing, &r1, Relation::Superseding);
    stale.ledger = Some(other.ledger.clone());
    expect_refusal(run(&env, &stale), "predecessor-not-latest");
    let mut latest = correct(&env, p, Valuation::Missing, &other, Relation::Superseding);
    latest.ledger = Some(other.ledger.clone());
    let accepted = run(&env, &latest).expect("a result");
    assert_eq!(accepted.lineage, 1);
    assert_eq!(
        accepted.relation,
        Relation::Superseding(other.result_identity.clone())
    );
}

/// FR-526-AC-25 (TC-1321 row 25): over two occurrences of one declaration, a
/// `Superseding(r3)` request selecting occurrence 7 with occurrence 3's
/// ledger and changed inputs is refused `predecessor-unknown`, and `r3` and
/// its ledger are unchanged.
#[trace("FR-526-AC-25", "TC-1321")]
#[test]
fn a_correction_cannot_move_to_another_occurrence_of_the_same_declaration() {
    let (three, p) = setup();
    let three = three.occurrence(3);
    let seven = setup().0.occurrence(7);
    let r3 = run(&three, &asking(&three, p, Valuation::True)).expect("a result");
    let (before, ledger_before) = (format!("{r3:?}"), r3.ledger.clone());
    let moved = correct(&seven, p, Valuation::False, &r3, Relation::Superseding);
    expect_refusal(run(&seven, &moved), "predecessor-unknown");
    assert_eq!(format!("{r3:?}"), before);
    assert_eq!(r3.ledger, ledger_before);
}

/// FR-526-AC-26 (TC-1321 row 26): a `Superseding(r1)` request carrying a
/// clone of `r1`'s returned ledger, and one carrying that ledger moved out of
/// `r1`, each return a result equal to the request carrying the ledger
/// itself. The compile-fail half is the doc test on `ResultEntry`.
#[trace("FR-526-AC-26", "TC-1321")]
#[test]
fn a_cloned_or_moved_ledger_gives_the_same_admission_as_the_ledger_itself() {
    let (env, p) = setup();
    let r1 = run(&env, &asking(&env, p, Valuation::True)).expect("a result");
    let identity = r1.result_identity.clone();
    let carrying = |ledger: qsl_eval::temporal::evidence::EvidenceLedger| {
        let mut asked = asking(&env, p, Valuation::False);
        asked.relation = Relation::Superseding(identity.clone());
        asked.ledger = Some(ledger);
        run(&env, &asked)
    };
    // The ledger as the call returned it, a clone of a copy of the whole
    // result, and the ledger moved out of the result.
    let copy = r1.clone();
    assert_eq!(copy.ledger, r1.ledger);
    let itself = format!("{:?}", carrying(r1.ledger.clone()).expect("a result"));
    let cloned = format!("{:?}", carrying(copy.ledger.clone()).expect("a result"));
    let moved = format!("{:?}", carrying(r1.ledger).expect("a result"));
    assert_eq!(cloned, itself);
    assert_eq!(moved, itself);
    // The comparison can fail: another ledger gives another admission.
    expect_refusal(
        carrying(qsl_eval::temporal::evidence::EvidenceLedger::default()),
        "predecessor-unknown",
    );
}

/// FR-526-AC-27 (TC-1321 row 27): a `Superseding(r1)` request with changed
/// inputs completes with the `lineage` ceiling 1; with the ceiling 0 it
/// returns an `Ok` `Incomplete` result naming `lineage`, the ceiling as given
/// and its setting, with the attempted lineage 1, the relation, no truth, no
/// instance or position charged and a ledger equal to the request's.
#[trace("FR-526-AC-27", "TC-1321")]
#[test]
fn the_lineage_ceiling_stops_a_correction_before_any_instance_or_position() {
    let (env, p) = setup();
    let r1 = run(&env, &asking(&env, p, Valuation::True)).expect("a result");
    let at = |ceiling: u64| {
        let mut asked = correct(&env, p, Valuation::False, &r1, Relation::Superseding);
        asked.limits = qsl_eval::temporal::evidence::TemporalEvidenceLimits::default()
            .with_ceiling(
                qsl_eval::temporal::evidence::EvidenceCounter::Lineage,
                ceiling,
            );
        asked
    };
    let done = run(&env, &at(1)).expect("a result");
    assert!(done.incomplete.is_none());
    assert_eq!(done.lineage, 1);

    let asked = at(0);
    let stopped = run(&env, &asked).expect("an Ok result");
    let Some(qsl_eval::temporal::evidence::EvidenceIncomplete::Limit(reached)) =
        stopped.incomplete.clone()
    else {
        panic!("a lineage stop, not {:?}", stopped.incomplete);
    };
    assert_eq!(
        reached.counter,
        qsl_eval::temporal::evidence::EvidenceCounter::Lineage
    );
    assert_eq!(reached.value, 0);
    assert_eq!(reached.setting.name(), "temporal_evidence.lineage");
    assert_eq!(stopped.lineage, 1);
    assert_eq!(
        stopped.relation,
        Relation::Superseding(r1.result_identity.clone())
    );
    assert_eq!(
        stopped.activation,
        qsl_eval::temporal::evidence::ActivationState::Unavailable
    );
    assert_eq!(stopped.truth, None);
    assert_eq!(
        stopped
            .usage
            .get(qsl_eval::temporal::evidence::EvidenceCounter::Instances),
        0
    );
    assert_eq!(
        stopped
            .usage
            .get(qsl_eval::temporal::evidence::EvidenceCounter::Positions),
        0
    );
    assert_eq!(Some(&stopped.ledger), asked.ledger.as_ref());
}

/// FR-526-AC-28 (TC-1321 row 28): a `Meter` charged `U` work units by one
/// call and passed to the stopped call reports `U` plus the units of the
/// stopped call's own charge log, never less than `U`; a third call on the
/// same meter continues from that total.
#[trace("FR-526-AC-28", "TC-1321")]
#[test]
fn a_lineage_stop_keeps_every_charge_an_earlier_call_made_on_the_meter() {
    let (env, p) = setup();
    let work = |meter: &quire_exact::Meter| meter.consumed(LimitKind::WorkUnits);
    let stopped_request = |previous: &EvidenceResult| {
        let mut asked = correct(&env, p, Valuation::False, previous, Relation::Superseding);
        asked.limits = qsl_eval::temporal::evidence::TemporalEvidenceLimits::default()
            .with_ceiling(qsl_eval::temporal::evidence::EvidenceCounter::Lineage, 0);
        asked
    };
    let mut meter = unlimited();
    let r1 = evaluate_temporal_evidence(
        &env.subject(&[]),
        &asking(&env, p, Valuation::True),
        &mut meter,
    )
    .expect("a result");
    let charged = work(&meter);
    assert!(charged > 0);

    // The stopped call's own units, on a fresh meter.
    let mut alone = unlimited();
    let stopped = evaluate_temporal_evidence(&env.subject(&[]), &stopped_request(&r1), &mut alone)
        .expect("an Ok result");
    assert!(stopped.incomplete.is_some());
    let own = work(&alone);

    let before = meter.admitted_charges().len();
    evaluate_temporal_evidence(&env.subject(&[]), &stopped_request(&r1), &mut meter)
        .expect("an Ok result");
    assert!(work(&meter) >= charged);
    assert_eq!(work(&meter), charged + own);
    assert_eq!(
        meter.admitted_charges().len() - before,
        alone.admitted_charges().len()
    );

    // A third call continues from that total.
    let total = work(&meter);
    let mut third_alone = unlimited();
    let ask = correct(&env, p, Valuation::False, &r1, Relation::Superseding);
    evaluate_temporal_evidence(&env.subject(&[]), &ask, &mut third_alone).expect("a result");
    evaluate_temporal_evidence(&env.subject(&[]), &ask, &mut meter).expect("a result");
    assert_eq!(work(&meter), total + work(&third_alone));
}

/// FR-043-AC-17 on the evidence call: over one occurrence of
/// `always[0,1] holds(p)` activated at position 0 with a closed, complete scope
/// of two positions, the result is read from the request's valuations alone.
/// `p` true at both positions gives `True`, settled by the closed scope, with
/// decision support at positions 0 and 1. `p` absent at position 0 gives no
/// truth, never `false`, and the typed `Indeterminate::MissingValuation`
/// locating position 0 and the atom of `p` (an atom that reads no instance binder
/// names no instance). A request
/// admits at most one valuation per position and atom, so a true row beside a
/// false row for one operand has no request form. The request carries no
/// leaf-handle map to set.
#[trace("TC-144", "FR-043-AC-17")]
#[test]
fn the_requests_valuations_alone_decide_an_always_clause() {
    use qsl_eval::temporal::evidence::{Indeterminate, Settlement};
    let env = env("on origin", "always[0, 1] holds(p)");
    let p = env.atom("p");
    let subject = env.subject(&[]);
    let at = |first: Option<Valuation>| {
        let zero: Vec<(u64, Valuation)> = first.map(|value| (p, value)).into_iter().collect();
        let mut asked = request(
            &env,
            vec![position(0, &zero), position(1, &[(p, Valuation::True)])],
        );
        asked.origins = vec![origin("o1", "a")];
        asked.trigger = Some(id("o1"));
        asked.completeness.as_mut().expect("a record").state =
            qsl_eval::temporal::ProgressCompleteness::Complete { through: 1 };
        evaluate_temporal_evidence(&subject, &asked, &mut unlimited()).expect("a result")
    };
    let valued = at(Some(Valuation::True));
    assert_eq!(valued.truth, Some(Truth::True));
    assert_eq!(valued.settlement, Some(Settlement::ClosedScope));
    let mut positions: Vec<usize> = valued.support.iter().map(|entry| entry.position).collect();
    positions.sort_unstable();
    assert_eq!(positions, [0, 1]);

    let absent = at(None);
    assert_eq!(absent.truth, None);
    assert_eq!(
        absent.indeterminate,
        Some(Indeterminate::MissingValuation {
            position: 0,
            atom: p,
            instance: None,
            also_held_by: Vec::new(),
        })
    );
}

/// FR-043-AC-16 `OperandNotInFormula` on the request's atom keys: a valuation
/// keyed by an atom the clause's formula does not have is refused
/// `invalid_runtime_input` / `invalid-value` with cause `valuation-atom-unknown`
/// at member `valuations` (`instance_valuations` for an instance's own map),
/// naming the atom index and the position's stamp, before any charge; a stray
/// key beside valid keys refuses too, and the same request without it is
/// accepted.
#[trace("TC-144")]
#[test]
fn a_valuation_keyed_by_an_atom_the_formula_lacks_is_refused_before_any_charge() {
    let (env, p) = setup();
    let subject = env.subject(&[]);
    let stray = 99_u64;
    let refused = |asked: &EvidenceRequest| {
        let mut meter = unlimited();
        let failure =
            evaluate_temporal_evidence(&subject, asked, &mut meter).expect_err("a refusal");
        assert!(meter.admitted_charges().is_empty());
        let CallFailure::Admission(failure) = failure else {
            panic!("an admission refusal, not {failure:?}");
        };
        let quire_eval::CallAdmissionFailure::Refused(record) = *failure else {
            panic!("a refusal");
        };
        (
            record.record.catalog,
            record.locus.clone(),
            record.record.fields.clone(),
        )
    };
    let named = |member: &str, at: &str| {
        (
            quire_semantic_value::catalog::CatalogCode::new(
                "invalid_runtime_input",
                "invalid-value",
            ),
            quire_eval::CallAdmissionLocus::Member {
                name: member.to_owned(),
                index: None,
            },
            std::collections::BTreeMap::from([
                ("cause", "valuation-atom-unknown".to_owned()),
                ("atom", stray.to_string()),
                ("position", at.to_owned()),
            ]),
        )
    };

    let valid = asking(&env, p, Valuation::True);
    assert!(run(&env, &valid).is_ok());

    // Beside valid keys, at the second position.
    let mut shared = valid.clone();
    shared.positions[1]
        .valuations
        .insert(stray, Valuation::True);
    assert_eq!(refused(&shared), named("valuations", "1"));

    // Alone at the first position of a request whose other keys are valid.
    let mut first = valid.clone();
    first.positions[0]
        .valuations
        .insert(stray, Valuation::False);
    assert_eq!(refused(&first), named("valuations", "0"));

    // In an instance's own map.
    let mut bound = valid.clone();
    bound.positions[1]
        .instance_valuations
        .entry(id("t1"))
        .or_default()
        .insert(stray, Valuation::True);
    assert_eq!(refused(&bound), named("instance_valuations", "1"));
}
