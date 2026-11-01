// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-249 / FR-419: exact fixed-work task-product gate and finite slot bounds.

use ix_trace_rs::trace;
use qsl_analyze::zone_check::{
    fixed_work_task_product, Bound, Dbm, FixedWorkArrival, FixedWorkGateMiss, TaskCompletionError,
    TaskJobPhase, TaskSchedulerState, TaskServiceClock,
};
use qsl_foundation::Span;
use qsl_replay::{
    TaskArrivalKind, TaskClockCeiling, TaskClosureMethod, TaskProductAbstraction,
    TaskProductCertificate, TaskQueueBound, TaskTimeProgress, TaskTimeProgressMethod,
};
use qsl_semantics::check::{Arrival, Criticality, SchedulingPolicy, Task, TaskSet, Wcet};
use quire_exact::{Integer, Rational};

fn int(value: i64) -> Rational {
    Rational::from_integer(Integer::from(value))
}

fn task(name: &str, period: i64, deadline: i64, bcet: i64, wcet: Wcet) -> Task {
    Task {
        name: name.to_owned(),
        arrival: Arrival::Periodic {
            period: int(period),
        },
        deadline: int(deadline),
        bcet: int(bcet),
        wcet,
        priority: Some(1),
        criticality: Criticality::Lo,
        jitter: int(0),
        blocking: int(0),
    }
}

fn set(tasks: Vec<Task>) -> TaskSet {
    TaskSet {
        name: "Ctl".to_owned(),
        unit: None,
        tasks,
        span: Span { start: 0, end: 0 },
    }
}

#[trace("TC-372", "FR-419-AC-1")]
#[test]
fn exact_gate_uses_checked_fixed_work_and_ceil_deadline_slots() {
    let product = fixed_work_task_product(
        &set(vec![task("a", 4, 5, 1, Wcet::Single(int(1)))]),
        SchedulingPolicy::FixedPriority,
    )
    .expect("singleton work enters exact fixed-work route");

    assert_eq!(product.task_set, "Ctl");
    assert_eq!(product.live_job_cap, 3);
    assert_eq!(product.tasks[0].arrival, FixedWorkArrival::Periodic);
    assert_eq!(product.tasks[0].interarrival, int(4));
    assert_eq!(product.tasks[0].relative_deadline, int(5));
    assert_eq!(product.tasks[0].work, int(1));
    assert_eq!(product.tasks[0].max_jobs, 3);
}

#[trace("TC-372", "FR-419-AC-1")]
#[test]
fn a_wcet_upper_bound_without_equal_bcet_stays_outside_the_exact_gate() {
    let result = fixed_work_task_product(
        &set(vec![task("a", 4, 4, 1, Wcet::Single(int(2)))]),
        SchedulingPolicy::FixedPriority,
    );

    assert_eq!(result, Err(FixedWorkGateMiss::VariableDemand));
}

#[trace("TC-372", "FR-419-AC-11")]
#[test]
fn edf_fixed_work_is_in_the_exact_gate_without_authored_priorities() {
    let mut edf_task = task("a", 4, 4, 1, Wcet::Single(int(1)));
    edf_task.priority = None;
    let product = fixed_work_task_product(&set(vec![edf_task]), SchedulingPolicy::Edf)
        .expect("EDF scheduling order is derived from absolute deadlines");

    assert_eq!(product.policy, SchedulingPolicy::Edf);
    assert_eq!(product.tasks[0].priority, None);
    assert_eq!(product.live_job_cap, 2);
}

#[trace("TC-372", "FR-419-AC-5")]
#[test]
fn exact_gate_refuses_bad_policy_premises_without_inventing_scheduler_order() {
    let mut first = task("a", 4, 4, 1, Wcet::Single(int(1)));
    let second = task("b", 4, 4, 1, Wcet::Single(int(1)));
    assert_eq!(
        fixed_work_task_product(
            &set(vec![first.clone(), second.clone()]),
            SchedulingPolicy::FixedPriority,
        ),
        Err(FixedWorkGateMiss::PriorityClash)
    );

    first.priority = None;
    first.jitter = int(1);
    assert_eq!(
        fixed_work_task_product(&set(vec![first]), SchedulingPolicy::Edf),
        Err(FixedWorkGateMiss::InvalidTaskSet)
    );
    assert_eq!(
        fixed_work_task_product(&set(vec![second]), SchedulingPolicy::Amc),
        Err(FixedWorkGateMiss::UnsupportedPolicy)
    );

    let mut blocked = task("blocked", 4, 4, 1, Wcet::Single(int(1)));
    blocked.blocking = int(1);
    assert_eq!(
        fixed_work_task_product(&set(vec![blocked]), SchedulingPolicy::FixedPriority),
        Err(FixedWorkGateMiss::BoundedSearchRequired)
    );
}

#[trace("TC-372", "FR-419-AC-6")]
#[test]
fn fixed_priority_jitter_stays_admitted_when_the_exact_edge_is_not_symbolic() {
    let mut jittered = task("jittered", 4, 4, 1, Wcet::Single(int(1)));
    jittered.jitter = int(1);
    assert_eq!(
        fixed_work_task_product(&set(vec![jittered]), SchedulingPolicy::FixedPriority,),
        Err(FixedWorkGateMiss::BoundedSearchRequired)
    );
}

#[trace("TC-372", "FR-419-AC-11")]
#[test]
fn fixed_priority_jobs_release_dispatch_complete_and_reuse_slots_by_generation() {
    let product = fixed_work_task_product(
        &set(vec![task("a", 4, 4, 1, Wcet::Single(int(1)))]),
        SchedulingPolicy::FixedPriority,
    )
    .expect("the checked fixed-work task enters the exact route");
    let source = quire_semantic_value::model_transition::ModelTransition::new(
        "\"activate\"".to_owned(),
        None,
        Vec::new(),
        quire_canonical::Limits::new(1024),
    )
    .expect("source transition identity");
    let mut scheduler = TaskSchedulerState::default();
    let first = scheduler
        .activate(&product, "a", source.clone(), int(0), int(0))
        .expect("first activation");

    assert_eq!(scheduler.jobs()[0].phase, TaskJobPhase::Ready);
    let dispatched = scheduler.dispatch(&product).expect("ready job dispatches");
    assert_eq!(dispatched.action, qsl_replay::TaskAction::Dispatch);
    let completed = scheduler.complete(None).expect("running job completes");
    assert_eq!(completed.action, qsl_replay::TaskAction::Complete);

    let second = scheduler
        .activate(&product, "a", source, int(0), int(4))
        .expect("slot is reused after completion");
    assert_eq!(second.slot, first.slot);
    assert_eq!(second.generation, first.generation + 1);
}

#[trace("TC-372", "FR-419-AC-6")]
#[test]
fn activations_share_arrival_phase_and_occurrence_across_release_bindings() {
    let mut spec = task("a", 4, 4, 1, Wcet::Single(int(1)));
    spec.arrival = Arrival::Sporadic {
        min_interarrival: int(4),
    };
    let product = fixed_work_task_product(&set(vec![spec]), SchedulingPolicy::FixedPriority)
        .expect("sporadic fixed-work task");
    let operation = |name: &str| {
        quire_semantic_value::model_transition::ModelTransition::new(
            format!("\"{name}\""),
            None,
            Vec::new(),
            quire_canonical::Limits::new(1024),
        )
        .expect("source transition identity")
    };
    let mut scheduler = TaskSchedulerState::default();
    let first = scheduler
        .activate(&product, "a", operation("start"), int(0), int(0))
        .expect("first activation establishes phase");
    assert_eq!(first.job, 0);
    assert_eq!(
        scheduler.activate(&product, "a", operation("alternate"), int(0), int(3)),
        Err(qsl_analyze::zone_check::TaskSchedulerError::ArrivalSpacing)
    );
    let second = scheduler
        .activate(&product, "a", operation("alternate"), int(0), int(4))
        .expect("a distinct operation shares the task's arrival phase");
    assert_eq!(second.job, 1);
}

#[trace("TC-372", "FR-419-AC-11")]
#[test]
fn edf_uses_absolute_deadline_then_task_name_and_keeps_optional_priority() {
    let mut a = task("a", 4, 4, 1, Wcet::Single(int(1)));
    a.priority = Some(9);
    let mut b = task("b", 4, 4, 1, Wcet::Single(int(1)));
    b.priority = Some(1);
    let product = fixed_work_task_product(&set(vec![a, b]), SchedulingPolicy::Edf)
        .expect("EDF does not require a priority ordering");
    let operation = |name: &str| {
        quire_semantic_value::model_transition::ModelTransition::new(
            format!("\"{name}\""),
            None,
            Vec::new(),
            quire_canonical::Limits::new(1024),
        )
        .expect("source transition identity")
    };
    let mut scheduler = TaskSchedulerState::default();
    scheduler
        .activate(&product, "b", operation("b_release"), int(0), int(0))
        .expect("activate b first");
    scheduler
        .activate(&product, "a", operation("a_release"), int(0), int(0))
        .expect("activate a second");

    let dispatched = scheduler.dispatch(&product).expect("EDF dispatches");
    assert_eq!(dispatched.task, "a");
    assert_eq!(dispatched.priority, Some(9));
}

#[trace("TC-372", "FR-419-AC-11")]
#[test]
fn edf_preempts_for_earlier_deadline_and_uses_activation_identity_for_ties() {
    let mut a = task("a", 4, 4, 1, Wcet::Single(int(1)));
    a.priority = None;
    let mut b = task("b", 4, 2, 1, Wcet::Single(int(1)));
    b.priority = None;
    let product = fixed_work_task_product(&set(vec![a, b]), SchedulingPolicy::Edf)
        .expect("EDF fixed-work gate");
    let operation = |name: &str| {
        quire_semantic_value::model_transition::ModelTransition::new(
            format!("\"{name}\""),
            None,
            Vec::new(),
            quire_canonical::Limits::new(1024),
        )
        .expect("source transition identity")
    };
    let mut scheduler = TaskSchedulerState::default();
    scheduler
        .activate(&product, "a", operation("z_release"), int(0), int(0))
        .expect("activate a");
    scheduler.dispatch(&product).expect("a starts first");
    scheduler
        .activate(&product, "b", operation("b_release"), int(0), int(0))
        .expect("activate earlier-deadline b");
    assert_eq!(
        scheduler.preempt(&product).expect("EDF preempts a").task,
        "a"
    );
    assert_eq!(
        scheduler
            .dispatch(&product)
            .expect("b has earlier deadline")
            .task,
        "b"
    );

    scheduler.complete(None).expect("b completes");
    assert_eq!(
        scheduler.dispatch(&product).expect("a resumes").action,
        qsl_replay::TaskAction::Resume
    );
    scheduler.complete(None).expect("a completes");

    let next = scheduler
        .activate(&product, "a", operation("a_release"), int(0), int(4))
        .expect("the next periodic activation uses a different release binding");
    assert_eq!(next.job, 1);
    assert_eq!(next.release, operation("a_release"));
}

#[trace("TC-371", "FR-418-AC-11")]
#[test]
fn task_certificate_gate_is_typed_and_checks_cap_and_singleton_work() {
    let certificate = TaskProductCertificate {
        abstraction: TaskProductAbstraction::BoundedSubtractionDiagonalV1,
        queue_bounds: vec![TaskQueueBound {
            taskset: "Ctl".to_owned(),
            task: "a".to_owned(),
            arrival: TaskArrivalKind::Periodic,
            interarrival: int(4),
            relative_deadline: int(4),
            max_jitter: int(0),
            work_lower: int(1),
            work_upper: int(1),
            max_jobs: 2,
        }],
        clock_ceilings: vec![TaskClockCeiling {
            clock: "task:Ctl.a:arrival".to_owned(),
            maximum: Integer::from(4_i64),
        }],
        diagonal_constraints: Vec::new(),
        subtractions: Vec::new(),
        time_progress: TaskTimeProgress {
            method: TaskTimeProgressMethod::ClosedFiniteProductV1,
            live_job_cap: 2,
            closure: TaskClosureMethod::RecomputeAllReachableSuccessorsV1,
        },
    };
    assert!(certificate.has_valid_shape());

    let mut bad_cap = certificate.clone();
    bad_cap.time_progress.live_job_cap = 1;
    assert!(!bad_cap.has_valid_shape());

    let mut variable_work = certificate;
    variable_work.queue_bounds[0].work_upper = int(2);
    assert!(!variable_work.has_valid_shape());
}

#[trace("TC-371", "FR-418-AC-11")]
#[test]
fn completion_subtracts_exact_work_from_suspended_service_clocks_atomically() {
    let mut a = task("a", 4, 4, 1, Wcet::Single(int(1)));
    let mut b = task("b", 4, 4, 1, Wcet::Single(int(1)));
    a.priority = Some(1);
    b.priority = Some(2);
    let product = fixed_work_task_product(&set(vec![a, b]), SchedulingPolicy::FixedPriority)
        .expect("fixed-priority exact product");
    let transition = |name: &str| {
        quire_semantic_value::model_transition::ModelTransition::new(
            format!("\"{name}\""),
            None,
            Vec::new(),
            quire_canonical::Limits::new(1024),
        )
        .expect("source transition identity")
    };
    let mut scheduler = TaskSchedulerState::default();
    let suspended = scheduler
        .activate(&product, "b", transition("b_release"), int(0), int(0))
        .expect("activate b");
    scheduler.dispatch(&product).expect("b starts");
    let running = scheduler
        .activate(&product, "a", transition("a_release"), int(0), int(0))
        .expect("activate a");
    scheduler.preempt(&product).expect("a preempts b");
    scheduler.dispatch(&product).expect("a starts");

    let running_clock = TaskServiceClock {
        job: running,
        index: 1,
        name: "task:Ctl.a:slot0:service".to_owned(),
        maximum: num_bigint::BigInt::from(4),
    };
    let suspended_clock = TaskServiceClock {
        job: suspended,
        index: 2,
        name: "task:Ctl.b:slot0:service".to_owned(),
        maximum: num_bigint::BigInt::from(4),
    };
    let scale = num_bigint::BigInt::from(1);
    let mut empty = Dbm::zero(3);
    empty
        .constrain(2, 0, Bound::le(num_bigint::BigInt::from(0)))
        .expect("service clock is in range");
    let before_scheduler = scheduler.clone();
    let before_zone = empty.clone();
    assert_eq!(
        scheduler.complete_with_subtractions(
            &product,
            &mut empty,
            &scale,
            &[running_clock.clone(), suspended_clock.clone()],
            None,
        ),
        Err(TaskCompletionError::EmptySuccessor)
    );
    assert_eq!(scheduler, before_scheduler);
    assert_eq!(empty, before_zone);

    let mut zone = Dbm::zero(3);
    zone.up();
    zone.constrain(2, 0, Bound::le(num_bigint::BigInt::from(2)))
        .expect("service clock upper bound");
    zone.constrain(0, 2, Bound::le(num_bigint::BigInt::from(-2)))
        .expect("service clock lower bound");
    let (completion, subtractions) = scheduler
        .complete_with_subtractions(
            &product,
            &mut zone,
            &scale,
            &[running_clock, suspended_clock],
            None,
        )
        .expect("the bounded subtraction has an enabled zone");
    assert_eq!(completion.action, qsl_replay::TaskAction::Complete);
    assert_eq!(subtractions.len(), 1);
    assert_eq!(subtractions[0].transition, completion);
    assert_eq!(subtractions[0].constant, Integer::one());
    assert_eq!(
        zone.bound(2, 0),
        Ok(&Bound::le(num_bigint::BigInt::from(1)))
    );
    assert_eq!(
        zone.bound(1, 0),
        Ok(&Bound::le(num_bigint::BigInt::from(0)))
    );
}
