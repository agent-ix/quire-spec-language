// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-249 / FR-419: exact fixed-work task-product gate and finite slot bounds.

use ix_trace_rs::trace;
use qsl_analyze::zone_check::{fixed_work_task_product, FixedWorkArrival, FixedWorkGateMiss};
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

#[trace("TC-704", "FR-419-AC-6")]
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

#[trace("TC-704", "FR-419-AC-6")]
#[test]
fn a_wcet_upper_bound_without_equal_bcet_stays_outside_the_exact_gate() {
    let result = fixed_work_task_product(
        &set(vec![task("a", 4, 4, 1, Wcet::Single(int(2)))]),
        SchedulingPolicy::FixedPriority,
    );

    assert_eq!(result, Err(FixedWorkGateMiss::VariableDemand));
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
