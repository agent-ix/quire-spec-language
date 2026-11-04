// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-516 (TC-1311 rows 36 to 41) and FR-515 (TC-1310 row 23): the refusal
//! causes of the evidence call, each with an empty charge log, and the
//! clause-unreadable causes read from the formula graph a checked package
//! lists. Each refusal is paired with the request that differs from it only in
//! not meeting the condition.

use ix_trace_rs::trace;
use qsl_eval::temporal::evidence::{
    evaluate_temporal_evidence, instance_bound_indices, ActivationState, ClauseFormula, ClauseWork,
    CompletenessRecord, EvidenceCounter, EvidenceIncomplete, EvidenceRef, EvidenceRequest,
    EvidenceResult, ReachFault, SubjectOccurrence,
};
use qsl_eval::temporal::{ProgressCompleteness, ProgressLedger, Valuation};
use qsl_package::checked_views::FormulaNodeUse;
use qsl_package::CheckedOperationInterval;
use quire_eval::CallFailure;
use serde_json::json;

use crate::temporal_activation::{
    call, claim, delivery, env, event_env, expect_refusal, fixed_env, id, node, origin, position,
    request, unlimited, Env, EACH, ONE_CAPTURE,
};

fn interval(value: serde_json::Value) -> Option<CheckedOperationInterval> {
    Some(serde_json::from_value(value).expect("an interval"))
}

fn use_of(
    operation: &'static str,
    interval: Option<CheckedOperationInterval>,
    operands: &[usize],
) -> FormulaNodeUse<'static> {
    FormulaNodeUse {
        operation,
        interval,
        operands: operands.to_vec(),
    }
}

const ALWAYS: &str = "quire.op.temporal.always";
const HOLDS: &str = "quire.op.temporal.holds";
const NOT: &str = "quire.op.temporal.not";

/// An `on origin` request over three positions that value `p` true.
fn origin_request(env: &Env, p: u64) -> EvidenceRequest {
    let mut asked = request(
        env,
        (0..3)
            .map(|index| position(index, &[(p, Valuation::True)]))
            .collect(),
    );
    asked.origins = vec![origin("o1", "a")];
    asked.trigger = Some(id("o1"));
    asked
}

/// Whether the refusal's diagnostic names the member `name`.
fn names_member(result: &Result<EvidenceResult, CallFailure>, name: &str) -> bool {
    matches!(result, Err(failure) if format!("{failure:?}").contains(&format!("name: \"{name}\"")))
}

/// FR-516-AC-36 (TC-1311 row 36): a clause `on each` with a non-empty
/// `origins` and a clause `on origin` with a non-empty `deliveries` each
/// refuse `activation-list-mismatch` naming `origins` and `deliveries`, with
/// an empty charge log; the requests with the matching list are evaluated.
#[trace("FR-516-AC-36", "TC-1311")]
#[test]
fn an_activation_list_of_the_other_kind_is_refused() {
    let each = env(EACH, ONE_CAPTURE);
    let p = each.atom("p");
    let mut asked = request(
        &each,
        (0..3)
            .map(|index| position(index, &[(p, Valuation::True)]))
            .collect(),
    );
    asked.deliveries = vec![delivery("t1", "r", "payload", &["5"])];
    asked.trigger = Some(id("t1"));
    assert_eq!(
        call(&each, &each.subject(&[]), &asked)
            .expect("the matching list")
            .activation,
        ActivationState::Active
    );
    asked.origins = vec![origin("o1", "a")];
    let mut meter = unlimited();
    let refused = evaluate_temporal_evidence(&each.subject(&[]), &asked, &mut meter);
    assert!(names_member(&refused, "origins"), "{refused:?}");
    expect_refusal(refused, "activation-list-mismatch");
    assert!(meter.admitted_charges().is_empty());

    let origin_env = env("on origin", "eventually[0, 2] holds(p)");
    let p = origin_env.atom("p");
    let mut asked = origin_request(&origin_env, p);
    assert!(call(&origin_env, &origin_env.subject(&[]), &asked).is_ok());
    asked.deliveries = vec![delivery("t1", "r", "payload", &[])];
    let mut meter = unlimited();
    let refused = evaluate_temporal_evidence(&origin_env.subject(&[]), &asked, &mut meter);
    assert!(names_member(&refused, "deliveries"), "{refused:?}");
    expect_refusal(refused, "activation-list-mismatch");
    assert!(meter.admitted_charges().is_empty());
}

fn completeness(through: u64) -> Option<CompletenessRecord> {
    Some(CompletenessRecord {
        reference: EvidenceRef::named("complete"),
        state: ProgressCompleteness::Complete { through },
        facts: Vec::new(),
    })
}

/// FR-516-AC-37 (TC-1311 row 37): over an event-position and a fixed-sample
/// clause a `Complete { through }` one below the last position's coordinate
/// is `progress-inconsistent`, and over the fixed-sample clause so is a
/// watermark one below it; the requests with the watermark and `through`
/// equal to that coordinate are evaluated, each refusal with an empty charge
/// log.
#[trace("FR-516-AC-37", "TC-1311")]
#[test]
fn a_progress_assertion_below_the_last_position_is_inconsistent() {
    for fixed in [false, true] {
        let clause = if fixed { fixed_env } else { event_env };
        let env = clause("on origin", "eventually[0, 1] holds(p)");
        let p = env.atom("p");
        let subject = env.subject(&[]);
        let mut asked = origin_request(&env, p);
        // The last position's coordinate is 2.
        asked.decision_progress = Some(2);
        asked.completeness = completeness(2);
        assert!(call(&env, &subject, &asked).is_ok(), "fixed: {fixed}");

        asked.completeness = completeness(1);
        let mut meter = unlimited();
        expect_refusal(
            evaluate_temporal_evidence(&subject, &asked, &mut meter),
            "progress-inconsistent",
        );
        assert!(meter.admitted_charges().is_empty(), "fixed: {fixed}");

        asked.completeness = completeness(2);
        asked.decision_progress = Some(1);
        let mut meter = unlimited();
        let result = evaluate_temporal_evidence(&subject, &asked, &mut meter);
        if fixed {
            expect_refusal(result, "progress-inconsistent");
            assert!(meter.admitted_charges().is_empty());
        } else {
            assert!(
                result.is_ok(),
                "the watermark is no assertion under event-position"
            );
        }
    }
}

/// FR-516-AC-39, FR-515-AC-23 (TC-1311 row 39, TC-1310 row 23): a clause whose
/// formula graph holds an operator without a finite upper bound, a timed
/// interval, a cycle or an operand position that names no entry is
/// `clause-unreadable` with an empty charge log; the readable graph of the
/// same shape returns a result; a tick sum that overflows `u64` is the
/// `history_span` `Incomplete` and not this cause.
#[trace("FR-516-AC-39", "FR-515-AC-23", "TC-1311", "TC-1310")]
#[test]
fn an_unreadable_formula_graph_is_refused_before_anything_is_charged() {
    let env = env("on origin", "eventually[0, 2] holds(p)");
    let p = env.atom("p");
    let asked = origin_request(&env, p);
    let bounded = || interval(json!({"lower": "0", "upper": "5"}));
    let readable = vec![use_of(ALWAYS, bounded(), &[1]), use_of(HOLDS, None, &[])];
    let unreadable: [(&str, Vec<FormulaNodeUse<'static>>, ReachFault); 6] = [
        (
            "unbounded: no interval",
            vec![use_of(ALWAYS, None, &[1]), use_of(HOLDS, None, &[])],
            ReachFault::Unbounded,
        ),
        (
            "unbounded: no upper end",
            vec![
                use_of(ALWAYS, interval(json!({"lower": "0", "upper": null})), &[1]),
                use_of(HOLDS, None, &[]),
            ],
            ReachFault::Unbounded,
        ),
        (
            "timed interval",
            vec![
                use_of(
                    ALWAYS,
                    interval(json!({
                        "lower": {"numerator": "1", "denominator": "2"},
                        "upper": {"numerator": "3", "denominator": "1"},
                        "lower_end": "closed",
                        "upper_end": "closed",
                    })),
                    &[1],
                ),
                use_of(HOLDS, None, &[]),
            ],
            ReachFault::Timed,
        ),
        (
            "cycle",
            vec![use_of(NOT, None, &[1]), use_of(NOT, None, &[0])],
            ReachFault::Malformed,
        ),
        (
            "out of range",
            vec![use_of(ALWAYS, bounded(), &[5]), use_of(HOLDS, None, &[])],
            ReachFault::Malformed,
        ),
        (
            "three operands",
            vec![use_of(NOT, None, &[0, 0, 0])],
            ReachFault::Malformed,
        ),
    ];

    let mut subject = env.subject(&[]);
    subject.formula = ClauseFormula::of_graph(&readable);
    assert!(evaluate_temporal_evidence(&subject, &asked, &mut unlimited()).is_ok());
    for (what, graph, fault) in unreadable {
        let formula = ClauseFormula::of_graph(&graph);
        assert_eq!(formula.work(), Err(fault), "{what}");
        let mut subject = env.subject(&[]);
        subject.formula = formula;
        let mut meter = unlimited();
        expect_refusal(
            evaluate_temporal_evidence(&subject, &asked, &mut meter),
            "clause-unreadable",
        );
        assert!(meter.admitted_charges().is_empty(), "{what}");
    }

    // A tick sum over `u64` is a stop of the call and no refusal.
    let top = json!({"lower": "0", "upper": u64::MAX.to_string()});
    let overflow = vec![
        use_of(ALWAYS, interval(top.clone()), &[1]),
        use_of(ALWAYS, interval(top), &[2]),
        use_of(HOLDS, None, &[]),
    ];
    let mut subject = env.subject(&[]);
    subject.formula = ClauseFormula::of_graph(&overflow);
    let result = evaluate_temporal_evidence(&subject, &asked, &mut unlimited())
        .expect("an overflowing sum is a result");
    assert_eq!(result.activation, ActivationState::Unavailable);
    assert_eq!(result.truth, None);
    match result.incomplete {
        Some(EvidenceIncomplete::Limit(reached)) => {
            assert_eq!(reached.counter, EvidenceCounter::HistorySpan);
        }
        other => panic!("the history_span stop, not {other:?}"),
    }

    // The package's work counters are the checked formula's own.
    assert_eq!(
        ClauseFormula::of_checked(env.fixture.clause.formula()).work(),
        ClauseWork::of(env.fixture.clause.formula())
    );
}

/// FR-516-AC-39 (TC-1311 row 39), FR-515 "Instance-bound atoms are derived":
/// a clause whose `predicate_leaves()` count differs from the formula's
/// `holds` count has no atom mapping, and the count that agrees maps the
/// `k`-th entry to the `k`-th `holds` node.
#[trace("FR-516-AC-39", "FR-515-AC-22", "TC-1311", "TC-1310")]
#[test]
fn a_leaf_count_that_differs_from_the_holds_count_maps_no_atoms() {
    let env = env("on origin", "eventually[0, 2] holds(p)");
    let p = env.atom("p");
    let formula = env.fixture.clause.formula();
    let nodes = formula.nodes().len();
    assert_eq!(
        instance_bound_indices(formula, &[true], nodes),
        Some([p].into())
    );
    assert_eq!(
        instance_bound_indices(formula, &[false], nodes),
        Some([].into())
    );
    assert_eq!(instance_bound_indices(formula, &[true, false], nodes), None);
    assert_eq!(instance_bound_indices(formula, &[], nodes), None);
    assert_eq!(instance_bound_indices(formula, &[true], 1), None);
}

/// FR-516-AC-40, FR-516-AC-41 (TC-1311 rows 40 and 41): each cause of the
/// table that this call can meet is `Err(CallFailure::Admission)` with that
/// cause, an empty charge log and the request's ledgers unchanged, and the
/// request that differs from it only in not meeting the condition is `Ok`.
#[trace("FR-516-AC-40", "FR-516-AC-41", "TC-1311")]
#[test]
fn each_cause_refuses_with_nothing_charged_and_its_unmet_twin_is_evaluated() {
    let (each, p) = {
        let env = env(EACH, ONE_CAPTURE);
        let p = env.atom("p");
        (env, p)
    };
    let delivered = |trigger: &str| {
        let mut asked = request(
            &each,
            (0..3)
                .map(|index| position(index, &[(p, Valuation::True)]))
                .collect(),
        );
        asked.deliveries = vec![delivery("t1", "r", "payload", &["5"])];
        asked.trigger = Some(id(trigger));
        asked
    };
    let check = |env: &Env,
                 subject: &qsl_eval::temporal::evidence::EvidenceSubject<'_>,
                 refused: EvidenceRequest,
                 met: EvidenceRequest,
                 cause: &str| {
        let mut meter = unlimited();
        let result = evaluate_temporal_evidence(subject, &refused, &mut meter);
        expect_refusal(result, cause);
        assert!(meter.admitted_charges().is_empty(), "{cause}");
        assert!(call(env, subject, &met).is_ok(), "{cause}: the unmet twin");
    };

    let subject = each.subject(&[]);
    let mut none = delivered("t1");
    none.trigger = None;
    check(&each, &subject, none, delivered("t1"), "trigger-required");
    check(
        &each,
        &subject,
        delivered("zz"),
        delivered("t1"),
        "trigger-unknown",
    );

    // The occurrence selector of twins.
    let row: qsl_package::TemporalBinding = serde_json::from_value(json!({
        "declaration": {"package": "recipe", "path": ["Due"], "ordinal": 0},
        "occurrence": 0,
        "clock_name": "stamp-clock",
        "clock": {"kind": "timestamped_event", "timestamp_unit": "ms"},
    }))
    .expect("a row");
    let mut twins = each.subject(&[]);
    twins.occurrences = vec![
        SubjectOccurrence {
            occurrence: claim(0),
            clock: Some(&row),
        },
        SubjectOccurrence {
            occurrence: claim(1),
            clock: Some(&row),
        },
    ];
    let mut selected = delivered("t1");
    selected.occurrence = Some(claim(0));
    check(
        &each,
        &twins,
        delivered("t1"),
        selected.clone(),
        "occurrence-required",
    );
    let mut unknown = delivered("t1");
    unknown.occurrence = Some(claim(5));
    check(&each, &twins, unknown, selected, "occurrence-unknown");

    let mut crossed = delivered("t1");
    crossed.origins = vec![origin("o1", "a")];
    check(
        &each,
        &subject,
        crossed,
        delivered("t1"),
        "activation-list-mismatch",
    );

    let mut surplus = delivered("t1");
    surplus.deliveries[0] = delivery("t1", "r", "payload", &["5", "6"]);
    check(&each, &subject, surplus, delivered("t1"), "surplus-capture");

    let mut conflict = delivered("t1");
    conflict
        .deliveries
        .push(delivery("t1", "r2", "other", &["5"]));
    check(
        &each,
        &subject,
        conflict,
        delivered("t1"),
        "delivery-contradiction",
    );

    let mut mapped = delivered("t1");
    mapped.control = Some(node("Ctl"));
    let with_row = env(EACH, ONE_CAPTURE).with_row("Ctl");
    let mapped_subject = with_row.subject(&[]);
    let mut missing = mapped.clone();
    missing.control = Some(node("Nope"));
    check(
        &with_row,
        &mapped_subject,
        missing,
        mapped,
        "activation-mapping-missing",
    );

    // A progress assertion that contradicts the retained ledger.
    let mut first = delivered("t1");
    first.progress_ledger = Some(ProgressLedger::new());
    let kept = call(&each, &subject, &first)
        .expect("a result")
        .progress_ledger
        .expect("a ledger");
    let mut regress = delivered("t1");
    regress.progress_ledger = Some(kept.clone());
    regress.decision_progress = Some(50);
    let mut same = delivered("t1");
    same.progress_ledger = Some(kept);
    check(&each, &subject, regress, same, "progress-contradiction");

    // A progress assertion below the last position under a sample profile.
    let fixed = fixed_env("on origin", "eventually[0, 1] holds(p)");
    let fixed_p = fixed.atom("p");
    let fixed_subject = fixed.subject(&[]);
    let mut met = origin_request(&fixed, fixed_p);
    met.decision_progress = Some(2);
    met.completeness = completeness(2);
    let mut below = met.clone();
    below.completeness = completeness(1);
    check(&fixed, &fixed_subject, below, met, "progress-inconsistent");

    // A formula no tick reach can measure.
    let reach = env("on origin", "eventually[0, 2] holds(p)");
    let reach_p = reach.atom("p");
    let mut unreadable = reach.subject(&[]);
    unreadable.formula =
        ClauseFormula::of_graph(&[use_of(ALWAYS, None, &[1]), use_of(HOLDS, None, &[])]);
    let asked = origin_request(&reach, reach_p);
    let mut meter = unlimited();
    expect_refusal(
        evaluate_temporal_evidence(&unreadable, &asked, &mut meter),
        "clause-unreadable",
    );
    assert!(meter.admitted_charges().is_empty());
    assert!(call(&reach, &reach.subject(&[]), &asked).is_ok());
}
