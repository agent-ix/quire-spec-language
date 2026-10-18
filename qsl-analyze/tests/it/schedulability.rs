// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-246 to FR-248 and QSpec FR-419: task sets, EN-7's closed-form
//! analyses, and the checker that recomputes their evidence before a
//! verdict settles. The fixtures are ADR-026 §12's `Ctl` and the sets the
//! acceptance criteria name.

use ix_trace_rs::trace;
use qsl_analyze::schedulability::{analyze, ScheduleLimits};
use qsl_replay::{
    recompute, settle, AmcEvidence, Arrival, Category, Certification, ClosedFormAnalysis,
    ClosedFormCheck, ClosedFormDisagreement, ClosedFormEvidence, ClosedFormOutcome, Code,
    Criticality, DisagreementCause, Identifier, InconclusiveCause, JobIterates,
    MissEvidence, ProofBasis, QualifiedName, ScheduleClaim, ScheduleLimit,
    ScheduleSubject, SchedulingPolicy, Task, TaskFixpoint, TaskSet, TaskSetRefusal, TerminalValue,
    Verdict, Wcet,
};
use qsl_replay::IncompleteCause;
use quire_exact::{Cancel, CancelCause, Integer, Rational};

fn r(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("a nonzero divisor")
}

fn n(value: i64) -> Rational {
    r(value, 1)
}

fn id(name: &str) -> Identifier {
    Identifier::new(name).expect("an identifier")
}

fn set_name(name: &str) -> QualifiedName {
    QualifiedName::new(vec![id(name)]).expect("one segment")
}

/// A `LO` task with `D = T`.
fn task(name: &str, wcet: i64, period: i64, priority: u64) -> Task {
    Task {
        name: id(name),
        arrival: Arrival::Periodic { period: n(period) },
        deadline: n(period),
        wcet: Wcet::Single(n(wcet)),
        priority: Some(priority),
        criticality: Criticality::Lo,
        jitter: n(0),
        blocking: n(0),
    }
}

fn hi_task(name: &str, lo: i64, hi: i64, period: i64, priority: u64) -> Task {
    Task {
        criticality: Criticality::Hi,
        wcet: Wcet::Dual { lo: n(lo), hi: n(hi) },
        ..task(name, lo, period, priority)
    }
}

/// ADR-026 §12's `Ctl`: `(C, T = D)` of `(1, 4)`, `(2, 6)` and `(c, 12)`,
/// priorities by period.
fn ctl(third: i64) -> TaskSet {
    TaskSet::new(
        set_name("Ctl"),
        vec![task("a", 1, 4, 1), task("b", 2, 6, 2), task("c", third, 12, 3)],
    )
    .expect("an admitted task set")
}

fn subject(set: TaskSet, claim: ScheduleClaim) -> ScheduleSubject {
    ScheduleSubject::new(set, claim).expect("an admitted claim")
}

fn schedulable(set: TaskSet, policy: SchedulingPolicy) -> ScheduleSubject {
    subject(set, ScheduleClaim::Schedulable(policy))
}

fn run(subject: &ScheduleSubject) -> ClosedFormOutcome {
    analyze(subject, &ScheduleLimits::default(), &Cancel::new())
}

fn check(subject: &ScheduleSubject, outcome: &ClosedFormOutcome) -> ClosedFormCheck {
    recompute(subject, outcome, &Cancel::new())
}

fn settled(subject: &ScheduleSubject, outcome: &ClosedFormOutcome) -> TerminalValue {
    settle(subject, outcome, &check(subject, outcome))
}

/// Each task's job-0 fixpoint, in the evidence's order.
fn fixpoints(evidence: &[TaskFixpoint]) -> Vec<Rational> {
    evidence
        .iter()
        .map(|entry| {
            entry
                .jobs
                .first()
                .and_then(JobIterates::last)
                .expect("a stated fixpoint")
                .clone()
        })
        .collect()
}

fn iterates(values: &[i64]) -> Vec<Rational> {
    values.iter().map(|value| n(*value)).collect()
}

fn fixed_points(outcome: &ClosedFormOutcome) -> &[TaskFixpoint] {
    let ClosedFormOutcome::Schedulable(ClosedFormEvidence::FixedPoints(listed)) = outcome else {
        panic!("expected fixed-priority fixpoints, got {outcome:?}");
    };
    listed
}

fn amc_evidence(outcome: &ClosedFormOutcome) -> &AmcEvidence {
    let (ClosedFormOutcome::Schedulable(ClosedFormEvidence::Amc(evidence))
    | ClosedFormOutcome::SufficientTestFailed(ClosedFormEvidence::Amc(evidence))) = outcome
    else {
        panic!("expected AMC evidence, got {outcome:?}");
    };
    evidence
}

/// The jitter-and-blocking set of FR-247-AC-2.
fn jitter_and_blocking() -> TaskSet {
    let b = Task {
        jitter: n(1),
        blocking: n(3),
        ..task("b", 2, 6, 2)
    };
    TaskSet::new(set_name("Jb"), vec![task("a", 1, 4, 1), b]).expect("an admitted task set")
}

fn mc(high_budget: i64, low_budget: i64) -> TaskSet {
    TaskSet::new(
        set_name("Mc"),
        vec![task("a", 1, 4, 1), hi_task("b", low_budget, high_budget, 6, 2)],
    )
    .expect("an admitted task set")
}

#[trace("TC-701", "FR-246-AC-1")]
#[test]
fn ctl_is_admitted_under_both_policies_and_its_premises_are_its_identity() {
    let fixed = schedulable(ctl(5), SchedulingPolicy::FixedPriority);
    let edf = schedulable(ctl(5), SchedulingPolicy::Edf);
    assert_eq!(fixed.task_set(), edf.task_set());
    assert_ne!(
        fixed,
        schedulable(ctl(6), SchedulingPolicy::FixedPriority),
        "the third WCET is a premise of the subject"
    );
    assert_eq!(fixed.analysis(), ClosedFormAnalysis::FixedPriorityRta);
    assert_eq!(edf.analysis(), ClosedFormAnalysis::EdfQpa);
}

#[trace("TC-701", "FR-246-AC-2")]
#[test]
fn a_task_set_or_claim_that_breaks_a_rule_is_refused_with_its_code() {
    let name = || set_name("S");
    let refusal = |tasks: Vec<Task>| TaskSet::new(name(), tasks).expect_err("a refusal");

    let zero_period = refusal(vec![task("a", 1, 0, 1)]);
    assert!(matches!(zero_period, TaskSetRefusal::NotPositive { .. }));
    assert_eq!(zero_period.code(), Code::InvalidModelBinding);
    assert_eq!(zero_period.cause(), "malformed-declaration");

    let inverted = refusal(vec![hi_task("h", 3, 2, 10, 1)]);
    assert!(matches!(inverted, TaskSetRefusal::BudgetOrder { .. }));
    assert_eq!(inverted.code(), Code::InvalidModelBinding);

    let duplicate = refusal(vec![task("a", 1, 4, 1), task("a", 1, 5, 2)]);
    assert_eq!(duplicate, TaskSetRefusal::DuplicateTask { task: id("a") });
    assert_eq!(duplicate.code(), Code::ConflictingDeclaration);
    assert_eq!(duplicate.cause(), "duplicate-declaration");

    let lo_dual = Task {
        wcet: Wcet::Dual { lo: n(1), hi: n(2) },
        ..task("l", 1, 4, 1)
    };
    assert!(matches!(
        refusal(vec![lo_dual]),
        TaskSetRefusal::LoDualBudgets { .. }
    ));

    let clash = TaskSet::new(name(), vec![task("a", 1, 4, 1), task("b", 1, 5, 1)])
        .expect("priorities are the claim's");
    let clashed = ScheduleSubject::new(
        clash.clone(),
        ScheduleClaim::Schedulable(SchedulingPolicy::FixedPriority),
    )
    .expect_err("a priority clash");
    assert_eq!(
        clashed,
        TaskSetRefusal::PriorityClash {
            first: id("a"),
            second: id("b"),
            priority: 1,
        }
    );
    assert_eq!(clashed.code(), Code::InvalidModelBinding);
    ScheduleSubject::new(clash, ScheduleClaim::Schedulable(SchedulingPolicy::Edf))
        .expect("an edf claim ignores priorities");

    let unknown = ScheduleSubject::new(
        ctl(5),
        ScheduleClaim::Response {
            task: id("zz"),
            bound: n(5),
        },
    )
    .expect_err("a task the set lacks");
    assert_eq!(unknown.code(), Code::MissingDeclaration);
    assert_eq!(unknown.cause(), "missing-name");
}

#[trace("TC-702", "FR-247-AC-1", "QSpec-FR-419-AC-1")]
#[test]
fn ctl_is_schedulable_under_fixed_priority_and_edf() {
    let fixed = run(&schedulable(ctl(5), SchedulingPolicy::FixedPriority));
    let listed = fixed_points(&fixed);
    assert_eq!(fixpoints(listed), vec![n(1), n(3), n(12)]);
    assert_eq!(
        listed[2].jobs,
        vec![JobIterates(iterates(&[5, 9, 12, 12]))]
    );

    let edf = run(&schedulable(ctl(5), SchedulingPolicy::Edf));
    assert_eq!(
        edf,
        ClosedFormOutcome::Schedulable(ClosedFormEvidence::Demand {
            busy_period: n(12),
            qpa_sequence: vec![n(8)],
        })
    );
}

#[trace("TC-702", "FR-247-AC-1")]
#[test]
fn a_response_claim_holds_at_twelve_and_misses_at_eleven() {
    let claim = |bound| ScheduleClaim::Response {
        task: id("c"),
        bound: n(bound),
    };
    let holds = run(&subject(ctl(5), claim(12)));
    assert_eq!(fixpoints(fixed_points(&holds)), vec![n(12)]);
    assert_eq!(
        run(&subject(ctl(5), claim(11))),
        ClosedFormOutcome::Miss(MissEvidence::FixedPriority {
            task: id("c"),
            job: 0,
            iterates: iterates(&[5, 9, 12]),
        })
    );
}

#[trace("TC-702", "FR-247-AC-2", "QSpec-FR-419-AC-2")]
#[test]
fn a_third_wcet_of_six_misses_under_fixed_priority_and_edf() {
    assert_eq!(
        run(&schedulable(ctl(6), SchedulingPolicy::FixedPriority)),
        ClosedFormOutcome::Miss(MissEvidence::FixedPriority {
            task: id("c"),
            job: 0,
            iterates: iterates(&[6, 10, 13]),
        })
    );
    assert_eq!(
        run(&schedulable(ctl(6), SchedulingPolicy::Edf)),
        ClosedFormOutcome::Miss(MissEvidence::Utilization(r(13, 12)))
    );
}

#[trace("TC-702", "FR-247-AC-2")]
#[test]
fn jitter_and_blocking_move_the_miss_to_b_and_its_evidence_rechecks() {
    let subject = schedulable(jitter_and_blocking(), SchedulingPolicy::FixedPriority);
    let outcome = run(&subject);
    assert_eq!(
        outcome,
        ClosedFormOutcome::Miss(MissEvidence::FixedPriority {
            task: id("b"),
            job: 0,
            iterates: iterates(&[5, 7]),
        })
    );
    assert_eq!(check(&subject, &outcome), ClosedFormCheck::Agrees);
}

#[trace("TC-702", "FR-247-AC-3", "QSpec-FR-419-AC-3")]
#[test]
fn amc_decides_schedulable_sufficient_test_failed_and_a_low_mode_miss() {
    let base = run(&schedulable(mc(3, 1), SchedulingPolicy::Amc));
    let evidence = amc_evidence(&base);
    assert_eq!(fixpoints(&evidence.low), vec![n(1), n(2)]);
    assert_eq!(fixpoints(&evidence.high), vec![n(4)]);
    assert!(matches!(base, ClosedFormOutcome::Schedulable(_)));

    let high_fails = run(&schedulable(mc(6, 1), SchedulingPolicy::Amc));
    assert!(matches!(high_fails, ClosedFormOutcome::SufficientTestFailed(_)));

    assert_eq!(
        run(&schedulable(mc(5, 5), SchedulingPolicy::Amc)),
        ClosedFormOutcome::Miss(MissEvidence::FixedPriority {
            task: id("b"),
            job: 0,
            iterates: iterates(&[5, 7]),
        })
    );
}

#[trace("TC-702", "FR-247-AC-4")]
#[test]
fn qpa_stops_at_the_demand_point_limit_naming_it() {
    // Utilization 19/20 < 1; QPA checks 16, 15, 14, 9, 7.
    let set = TaskSet::new(
        set_name("Many"),
        vec![
            Task {
                deadline: n(5),
                ..task("a", 3, 10, 1)
            },
            task("b", 2, 8, 2),
            task("c", 2, 5, 3),
        ],
    )
    .expect("an admitted task set");
    let subject = schedulable(set, SchedulingPolicy::Edf);
    let ClosedFormOutcome::Schedulable(ClosedFormEvidence::Demand { qpa_sequence, .. }) =
        run(&subject)
    else {
        panic!("the set is schedulable under the default limit");
    };
    assert_eq!(qpa_sequence, iterates(&[16, 15, 14, 9, 7]));

    let limits = ScheduleLimits {
        max_demand_points: 3,
    };
    assert_eq!(
        analyze(&subject, &limits, &Cancel::new()),
        ClosedFormOutcome::Stopped(
            IncompleteCause::ResourceExhausted,
            ScheduleLimit::MaxDemandPoints(3)
        )
    );
}

#[trace("TC-702", "FR-247-AC-4")]
#[test]
fn a_cancelled_handle_stops_the_analysis() {
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let outcome = analyze(
        &schedulable(ctl(5), SchedulingPolicy::FixedPriority),
        &ScheduleLimits::default(),
        &cancel,
    );
    assert!(matches!(
        outcome,
        ClosedFormOutcome::Stopped(IncompleteCause::Cancelled { .. }, _)
    ));
}

fn proved(analysis: ClosedFormAnalysis) -> TerminalValue {
    TerminalValue::Proved {
        basis: ProofBasis::ClosedForm { analysis },
        certification: Certification::Certified,
    }
}

#[trace("TC-703", "FR-248-AC-1", "QSpec-FR-419-AC-1", "QSpec-FR-419-AC-2")]
#[test]
fn ctl_settles_proved_with_its_fixpoints_and_a_third_wcet_of_six_settles_refuted() {
    let fixed = schedulable(ctl(5), SchedulingPolicy::FixedPriority);
    let outcome = run(&fixed);
    assert_eq!(fixpoints(fixed_points(&outcome)), vec![n(1), n(3), n(12)]);
    assert_eq!(
        settled(&fixed, &outcome),
        proved(ClosedFormAnalysis::FixedPriorityRta)
    );
    let edf = schedulable(ctl(5), SchedulingPolicy::Edf);
    assert_eq!(settled(&edf, &run(&edf)), proved(ClosedFormAnalysis::EdfQpa));

    for policy in [SchedulingPolicy::FixedPriority, SchedulingPolicy::Edf] {
        let missing = schedulable(ctl(6), policy);
        let outcome = run(&missing);
        assert!(matches!(outcome, ClosedFormOutcome::Miss(_)));
        assert_eq!(settled(&missing, &outcome), TerminalValue::Refuted);
        assert_eq!(settled(&missing, &outcome).category(), Category::Violation);
    }
}

fn parity() -> TerminalValue {
    TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(DisagreementCause::NoValue {
        proved: Verdict::from_category(Category::Success),
        replayed: Verdict::from_category(Category::Inconclusive),
    }))
}

#[trace("TC-703", "FR-248-AC-2", "QSpec-FR-419-AC-4")]
#[test]
fn altered_evidence_settles_inconclusive_replay_parity() {
    let fixed = schedulable(ctl(5), SchedulingPolicy::FixedPriority);
    let mut tampered = run(&fixed);
    let ClosedFormOutcome::Schedulable(ClosedFormEvidence::FixedPoints(listed)) = &mut tampered
    else {
        panic!("Ctl is schedulable");
    };
    listed[2].jobs[0] = JobIterates(iterates(&[5, 9, 11, 11]));
    assert!(matches!(
        check(&fixed, &tampered),
        ClosedFormCheck::Disagrees(ClosedFormDisagreement::Iterate { index: 3, .. })
    ));
    assert_eq!(settled(&fixed, &tampered), parity());

    let missing = schedulable(ctl(6), SchedulingPolicy::FixedPriority);
    let mut altered = run(&missing);
    let ClosedFormOutcome::Miss(MissEvidence::FixedPriority { iterates, .. }) = &mut altered else {
        panic!("the third WCET of six misses");
    };
    iterates[2] = n(12);
    assert!(matches!(
        check(&missing, &altered),
        ClosedFormCheck::Disagrees(ClosedFormDisagreement::Iterate { .. })
    ));
    let TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(cause)) =
        settled(&missing, &altered)
    else {
        panic!("a disagreement settles replay-parity");
    };
    assert_eq!(cause.proved().category(), Category::Violation);
}

/// `a` (`C = 1`, `T = D = 2`, priority 1) and `b` (`C = 1`, `T = 3`,
/// `D = 1`, blocking 1, priority 2).
fn busy_period_set() -> ScheduleSubject {
    let b = Task {
        deadline: n(1),
        blocking: n(1),
        ..task("b", 1, 3, 2)
    };
    schedulable(
        TaskSet::new(set_name("Bp"), vec![task("a", 1, 2, 1), b]).expect("an admitted task set"),
        SchedulingPolicy::FixedPriority,
    )
}

fn miss_of(job: u64, values: &[i64]) -> ClosedFormOutcome {
    ClosedFormOutcome::Miss(MissEvidence::FixedPriority {
        task: id("b"),
        job,
        iterates: iterates(values),
    })
}

#[trace("TC-703", "FR-248-AC-2")]
#[test]
fn a_miss_for_a_job_past_the_busy_period_names_the_job_that_closes_it() {
    let subject = busy_period_set();
    assert_eq!(
        check(&subject, &miss_of(2, &[4, 6, 7, 8])),
        ClosedFormCheck::Disagrees(ClosedFormDisagreement::BusyPeriodClosed {
            task: id("b"),
            job: 1,
        })
    );
    assert_eq!(settled(&subject, &miss_of(2, &[4, 6, 7, 8])), parity_violation());
    assert_eq!(settled(&subject, &miss_of(0, &[2])), TerminalValue::Refuted);
}

fn parity_violation() -> TerminalValue {
    TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(DisagreementCause::NoValue {
        proved: Verdict::from_category(Category::Violation),
        replayed: Verdict::from_category(Category::Inconclusive),
    }))
}

#[trace("TC-703", "FR-248-AC-3", "QSpec-FR-419-AC-3")]
#[test]
fn amc_settles_proved_and_a_failed_sufficient_test_settles_inconclusive() {
    let base = schedulable(mc(3, 1), SchedulingPolicy::Amc);
    assert_eq!(
        settled(&base, &run(&base)),
        proved(ClosedFormAnalysis::AmcRtb)
    );
    let high_fails = schedulable(mc(6, 1), SchedulingPolicy::Amc);
    assert_eq!(
        settled(&high_fails, &run(&high_fails)),
        TerminalValue::Inconclusive(InconclusiveCause::SufficientTestFailed)
    );
    let low_miss = schedulable(mc(5, 5), SchedulingPolicy::Amc);
    assert_eq!(settled(&low_miss, &run(&low_miss)), TerminalValue::Refuted);
}

#[trace("TC-703", "FR-248-AC-1")]
#[test]
fn a_stopped_analysis_settles_incomplete() {
    let subject = schedulable(ctl(5), SchedulingPolicy::Edf);
    let outcome = ClosedFormOutcome::Stopped(
        IncompleteCause::ResourceExhausted,
        ScheduleLimit::MaxDemandPoints(3),
    );
    assert_eq!(
        settled(&subject, &outcome),
        TerminalValue::Incomplete(IncompleteCause::ResourceExhausted)
    );
}

#[trace("TC-703", "FR-248-AC-2")]
#[test]
fn evidence_of_another_analysis_or_for_another_subject_disagrees() {
    let fixed = schedulable(ctl(5), SchedulingPolicy::FixedPriority);
    let edf = schedulable(ctl(5), SchedulingPolicy::Edf);
    assert_eq!(
        check(&fixed, &run(&edf)),
        ClosedFormCheck::Disagrees(ClosedFormDisagreement::Shape)
    );
    // The same evidence checked against a task set with a raised WCET.
    let raised = schedulable(ctl(6), SchedulingPolicy::FixedPriority);
    assert!(matches!(
        check(&raised, &run(&fixed)),
        ClosedFormCheck::Disagrees(_)
    ));
    // A shortened QPA sequence and a wrong busy-period length disagree.
    let ClosedFormOutcome::Schedulable(ClosedFormEvidence::Demand { busy_period, .. }) = run(&edf)
    else {
        panic!("Ctl is schedulable under edf");
    };
    let shortened = ClosedFormOutcome::Schedulable(ClosedFormEvidence::Demand {
        busy_period: busy_period.clone(),
        qpa_sequence: vec![],
    });
    assert!(matches!(
        check(&edf, &shortened),
        ClosedFormCheck::Disagrees(ClosedFormDisagreement::QpaSequence { .. })
    ));
    let wrong = ClosedFormOutcome::Schedulable(ClosedFormEvidence::Demand {
        busy_period: n(11),
        qpa_sequence: vec![n(8)],
    });
    assert_eq!(
        check(&edf, &wrong),
        ClosedFormCheck::Disagrees(ClosedFormDisagreement::BusyPeriodLength)
    );
}
