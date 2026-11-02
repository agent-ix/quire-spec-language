// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-249 / FR-419: exact fixed-work task-product gate and finite slot bounds.

use ix_trace_rs::trace;
use qsl_analyze::zone_check::{
    certificate_item, check_timed_model, fixed_work_task_product, task_product_route, Bound, Dbm,
    FixedWorkArrival, FixedWorkGateMiss, TaskCompletionError, TaskJobPhase,
    TaskProductAdmissionError, TaskProductRoute, TaskSchedulerState, TaskSearchReason,
    TaskServiceClock, ZoneCheckItem, ZoneCheckLimits, ZoneCheckOutcome, ZoneCheckRequest,
    ZoneUndecided,
};
use qsl_eval::simulation::model::{TimedSystem, DEFAULT_MAX_CLOCKS};
use qsl_foundation::SourceIdentity;
use qsl_foundation::Span;
use qsl_replay::{
    check_zone_certificate, CertificateCheckLimits, CertificateLocus, CertificateMember,
    CertificateRule, CertifiedItem, TaskArrivalKind, TaskClockCeiling, TaskClosureMethod,
    TaskDiagonalConstraint, TaskDiagonalOperator, TaskProductAbstraction, TaskProductCertificate,
    TaskProductStateKey, TaskQueueBound, TaskTimeProgress, TaskTimeProgressMethod,
    ZoneCertificateRequest, ZoneCertificateVerdict,
};
use qsl_route::deadlock::{ItemObligation, ObligationKind};
use qsl_route::reductions::ObligationReductions;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::check::{Arrival, Criticality, SchedulingPolicy, Task, TaskSet, Wcet};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, package_input};
use quire_exact::Cancel;
use quire_exact::{Integer, Rational};
use quire_semantic_value::checking::CheckingLimits as SemanticCheckingLimits;

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

#[trace("TC-431", "FR-415-AC-26")]
#[test]
fn task_product_work_limit_has_the_published_default() {
    assert_eq!(ZoneCheckLimits::default().max_task_product_work, 1 << 20);
}

fn checked_task_package(unit: &str, body: &str) -> qsl_package::CheckedPackage {
    let document = crate::litmus::document();
    let packages = package_input([document.as_slice()]).expect("domain package input");
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile t = \"quire.temporal.complete/v1\";\n\
         profile inf = \"quire.temporal.infinite-trace/v1\";\n\
         model M = \"{}\";\n\
         dimension Time;\n\
         unit {unit} : Time = rational(1, 1);\n\
         {body}",
        crate::litmus::PACKAGE,
    );
    let parsed = qsl_cst::parse(
        SourceIdentity::new("agent-ix", "qsl-analyze-tests", "git", "1"),
        "task-product.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 parses task package");
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds task package");
    let models = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("model selection admits")
    .with_parsed_operation_clauses(|text| {
        qsl_forms::parse_clause_text(text, qsl_cst::Limits::default())
    })
    .expect("operation clauses parse");
    let graph = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "package".to_owned(),
        built,
        models,
        Vec::new(),
    )
    .expect("unit assembles")
    .check(SemanticCheckingLimits::default())
    .unwrap_or_else(|refusals| panic!("task package checks: {refusals:?}"));
    qsl_package::CheckedPackage::link(graph)
}

fn checked_timed_task_package(body: &str) -> qsl_package::CheckedPackage {
    let mut document: serde_json::Value =
        serde_json::from_slice(&crate::litmus::document()).expect("litmus document JSON");
    document
        .as_object_mut()
        .expect("document object")
        .remove("populations");
    document["types"]
        .as_array_mut()
        .expect("types array")
        .retain(|node| {
            node["identity"] == "ix://test/litmus/Cell"
                || node["identity"] == "ix://test/litmus/Level"
        });
    let document = serde_json::to_vec(&document).expect("single-type package JSON");
    let packages = package_input([document.as_slice()]).expect("timed domain package input");
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile t = \"quire.temporal.complete/v1\";\n\
         model M = \"{}\";\n\
         dimension Time;\n\
         unit ms : Time = rational(1, 1);\n\
         time M tick M::Cell::set period 4 ms unit ms;\n\
         {body}",
        crate::litmus::PACKAGE,
    );
    let parsed = qsl_cst::parse(
        SourceIdentity::new("agent-ix", "qsl-analyze-tests", "git", "1"),
        "task-product-timed.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("timed task package parses");
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds timed task package");
    let models = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("timed model selection admits")
    .with_parsed_operation_clauses(|text| {
        qsl_forms::parse_clause_text(text, qsl_cst::Limits::default())
    })
    .expect("operation clauses parse");
    let graph = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "package".to_owned(),
        built,
        models,
        Vec::new(),
    )
    .expect("timed task unit assembles")
    .check(SemanticCheckingLimits::default())
    .unwrap_or_else(|refusals| panic!("timed task package checks: {refusals:?}"));
    qsl_package::CheckedPackage::link(graph)
}

#[trace("TC-372", "QSpec-TC-967", "QSpec-FR-482-AC-2")]
#[test]
fn task_claim_occurrence_identity_tracks_normalized_fr482_body() {
    let base = "taskset Ctl {\n\
         task t1 periodic 4 ms deadline 4 ms bcet 0 ms wcet 1 ms priority 1 lo;\n\
         task t2 periodic 6 ms deadline 6 ms bcet 0 ms wcet 2 ms priority 2 lo;\n\
         task t3 periodic 12 ms deadline 12 ms bcet 0 ms wcet 5 ms priority 3 lo;\n\
         }\n\
         schedulable CtlFp on Ctl under fixed_priority;\n\
         release Ctl.t1 on Cell::set;\n";
    let package = checked_task_package("ms", base);
    let [claim] = package.graph().task_sets().claims() else {
        panic!("one schedulability claim");
    };
    let occurrence = claim.occurrence().expect("FR-482 claim occurrence");
    assert_eq!(occurrence.origin().role().as_str(), "claim");
    assert_eq!(occurrence.origin().ordinal(), 0);
    assert_eq!(package.graph().task_sets().releases().len(), 1);
    let base_key = claim.node_id().expect("derived checked claim node").clone();

    let renamed = checked_task_package("ms", &base.replace("CtlFp", "Other"));
    assert_eq!(
        renamed.graph().task_sets().claims()[0].node_id(),
        Some(&base_key)
    );

    let explicit_zero = checked_task_package(
        "ms",
        &base.replace("priority 1 lo;", "priority 1 jitter 0 ms lo;"),
    );
    assert_eq!(
        explicit_zero.graph().task_sets().claims()[0].node_id(),
        Some(&base_key)
    );

    let changed_policy = checked_task_package("ms", &base.replace("fixed_priority", "edf"));
    assert_ne!(
        changed_policy.graph().task_sets().claims()[0].node_id(),
        Some(&base_key)
    );

    let changed_task = checked_task_package("ms", &base.replace("priority 1", "priority 4"));
    assert_ne!(
        changed_task.graph().task_sets().claims()[0].node_id(),
        Some(&base_key)
    );

    let changed_unit = checked_task_package("tick", &base.replace(" ms", " tick"));
    assert_ne!(
        changed_unit.graph().task_sets().claims()[0].node_id(),
        Some(&base_key)
    );

    let ordered = checked_task_package("ms", base);
    let reversed_tasks = base.replace(
        "task t1 periodic 4 ms deadline 4 ms bcet 0 ms wcet 1 ms priority 1 lo;\n         task t2 periodic 6 ms deadline 6 ms bcet 0 ms wcet 2 ms priority 2 lo;\n         task t3 periodic 12 ms deadline 12 ms bcet 0 ms wcet 5 ms priority 3 lo;",
        "task t3 periodic 12 ms deadline 12 ms bcet 0 ms wcet 5 ms priority 3 lo;\n         task t2 periodic 6 ms deadline 6 ms bcet 0 ms wcet 2 ms priority 2 lo;\n         task t1 periodic 4 ms deadline 4 ms bcet 0 ms wcet 1 ms priority 1 lo;",
    );
    let reversed = checked_task_package("ms", &reversed_tasks);
    assert_eq!(
        ordered.graph().task_sets().claims()[0].node_id(),
        reversed.graph().task_sets().claims()[0].node_id()
    );

    let duplicate = checked_task_package(
        "ms",
        &base.replace(
            "schedulable CtlFp on Ctl under fixed_priority;",
            "schedulable CtlFp on Ctl under fixed_priority;\n\
             schedulable Other on Ctl under fixed_priority;",
        ),
    );
    let [first, second] = duplicate.graph().task_sets().claims() else {
        panic!("two equal-body claims");
    };
    assert_eq!(first.node_id(), second.node_id());
    assert_eq!(
        first.occurrence().expect("first claim").origin().ordinal(),
        0
    );
    assert_eq!(
        second
            .occurrence()
            .expect("second claim")
            .origin()
            .ordinal(),
        1
    );

    let with_timed_source = checked_timed_task_package(base);
    let emission = qsl_package::emit_checked(&with_timed_source)
        .expect("timed source provides an emitted graph for the unit NodeRef");
    let wire: serde_json::Value = serde_json::from_slice(emission.package().bytes())
        .expect("the emitted checked package is JSON");
    let nodes = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic graph nodes");
    let unit_nodes: Vec<_> = nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node["node_tag"] == "scalar_type" && node["semantic_form"] == "unit")
        .collect();
    let [(unit_index, unit_node)] = unit_nodes.as_slice() else {
        panic!("one emitted ms unit node");
    };
    assert_eq!(*unit_index, 6);
    assert_eq!(
        unit_node["node_id"],
        serde_json::json!({"package": "package", "path": ["ms"], "ordinal": 2})
    );
    let [timed_claim] = with_timed_source.graph().task_sets().claims() else {
        panic!("one checked timed schedulability claim");
    };
    assert_eq!(timed_claim.node_id(), Some(&base_key));
}

#[trace("TC-372", "FR-418-AC-14", "QSpec-TC-372-ZC-28")]
#[test]
fn no_release_certificate_reader_rejects_a_reachable_checked_release_at_root() {
    let document = crate::timed_world::document(&[], &[("tick", &[])]);
    let world = crate::timed_world::world(
        &document,
        "",
        "dimension Time;\n\
         unit ms : Time = rational(1, 1);\n\
         time M tick M::Call::tick period 4 ms unit ms;\n\
         taskset Ctl {\n\
         task t1 periodic 4 ms deadline 4 ms bcet 0 ms wcet 1 ms priority 1 lo;\n\
         task t2 periodic 6 ms deadline 6 ms bcet 0 ms wcet 2 ms priority 2 lo;\n\
         task t3 periodic 12 ms deadline 12 ms bcet 0 ms wcet 5 ms priority 3 lo;\n\
         }\n\
         schedulable CtlFp on Ctl under fixed_priority;\n\
         release Ctl.t1 on Call::tick;\n",
    );
    let [claim] = world.package.graph().task_sets().claims() else {
        panic!("one checked schedulability claim");
    };
    let occurrence = claim.occurrence().expect("claim occurrence");
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("timed model");
    let mut certificate = match check_timed_model(
        ZoneCheckRequest {
            subject: &timed,
            item: ZoneCheckItem::TimeLockFreedom,
            obligation: ItemObligation {
                subject: "task-product-reader",
                kind: ObligationKind::TimeLockFreedom,
                reductions: ObligationReductions::default(),
            },
            limits: ZoneCheckLimits::default(),
            reductions: &qsl_route::reductions::Reductions::default(),
        },
        &Cancel::new(),
    )
    .expect("the ordinary timed source is admitted")
    {
        ZoneCheckOutcome::Holds { certificate, .. } => *certificate,
        outcome => panic!("expected an ordinary source certificate, got {outcome:?}"),
    };
    certificate.obligation.kind = ObligationKind::TaskSchedulability(occurrence.clone());
    certificate.divergence = None;
    for node in &mut certificate.nodes {
        node.state_key = TaskProductStateKey::new(
            node.state_key.clone(),
            TaskProductStateKey::empty_scheduler_control(),
        )
        .into_state_key()
        .expect("the joint source/empty-scheduler key encodes");
    }
    certificate.task_product = Some(TaskProductCertificate {
        abstraction: TaskProductAbstraction::BoundedSubtractionDiagonalV1,
        queue_bounds: vec![
            TaskQueueBound {
                taskset: "Ctl".to_owned(),
                task: "t1".to_owned(),
                arrival: TaskArrivalKind::Periodic,
                interarrival: int(4),
                relative_deadline: int(4),
                max_jitter: int(0),
                work_lower: int(0),
                work_upper: int(1),
                max_jobs: 2,
            },
            TaskQueueBound {
                taskset: "Ctl".to_owned(),
                task: "t2".to_owned(),
                arrival: TaskArrivalKind::Periodic,
                interarrival: int(6),
                relative_deadline: int(6),
                max_jitter: int(0),
                work_lower: int(0),
                work_upper: int(2),
                max_jobs: 2,
            },
            TaskQueueBound {
                taskset: "Ctl".to_owned(),
                task: "t3".to_owned(),
                arrival: TaskArrivalKind::Periodic,
                interarrival: int(12),
                relative_deadline: int(12),
                max_jitter: int(0),
                work_lower: int(0),
                work_upper: int(5),
                max_jobs: 2,
            },
        ],
        clock_ceilings: Vec::new(),
        diagonal_constraints: Vec::new(),
        subtractions: Vec::new(),
        time_progress: TaskTimeProgress {
            method: TaskTimeProgressMethod::ClosedNoReleaseProductV1,
            live_job_cap: 0,
            closure: TaskClosureMethod::RecomputeAllSourceSuccessorsNoTaskReleaseV1,
        },
    });

    let result = check_zone_certificate(
        ZoneCertificateRequest {
            simulation: world.request(),
            model: "M",
            max_clocks: DEFAULT_MAX_CLOCKS,
            item: CertifiedItem::TaskSchedulability {
                claim,
                occurrence: &occurrence,
            },
        },
        &certificate.obligation,
        certificate_item,
        &certificate,
        CertificateCheckLimits::default(),
        &Cancel::new(),
    )
    .expect("the certificate is a typed rejection, not a reader refusal");
    assert_eq!(
        result,
        ZoneCertificateVerdict::Rejected(qsl_replay::CertificateRejection {
            rule: CertificateRule::SuccessorMissing,
            at: CertificateLocus::ZoneNode { node: 0 },
        })
    );
}

#[trace("TC-372", "FR-418-AC-11", "FR-418-AC-14")]
#[test]
fn no_release_certificate_reader_accepts_closed_source_with_no_release_binding() {
    let document = crate::timed_world::document(&[], &[("tick", &[])]);
    let world = crate::timed_world::world(
        &document,
        "",
        "dimension Time;\n\
         unit ms : Time = rational(1, 1);\n\
         time M tick M::Call::tick period 4 ms unit ms;\n\
         taskset Ctl {\n\
         task t1 periodic 4 ms deadline 4 ms bcet 0 ms wcet 1 ms priority 1 lo;\n\
         }\n\
         schedulable CtlFp on Ctl under fixed_priority;\n",
    );
    let [claim] = world.package.graph().task_sets().claims() else {
        panic!("one checked schedulability claim");
    };
    let occurrence = claim.occurrence().expect("claim occurrence");
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("timed model");
    let mut certificate = match check_timed_model(
        ZoneCheckRequest {
            subject: &timed,
            item: ZoneCheckItem::TimeLockFreedom,
            obligation: ItemObligation {
                subject: "task-product-no-release-reader",
                kind: ObligationKind::TimeLockFreedom,
                reductions: ObligationReductions::default(),
            },
            limits: ZoneCheckLimits::default(),
            reductions: &qsl_route::reductions::Reductions::default(),
        },
        &Cancel::new(),
    )
    .expect("the ordinary timed source is admitted")
    {
        ZoneCheckOutcome::Holds { certificate, .. } => *certificate,
        outcome => panic!("expected an ordinary source certificate, got {outcome:?}"),
    };
    certificate.obligation.kind = ObligationKind::TaskSchedulability(occurrence.clone());
    certificate.divergence = None;
    for node in &mut certificate.nodes {
        node.state_key = TaskProductStateKey::new(
            node.state_key.clone(),
            TaskProductStateKey::empty_scheduler_control(),
        )
        .into_state_key()
        .expect("the joint source/empty-scheduler key encodes");
    }
    certificate.task_product = Some(TaskProductCertificate {
        abstraction: TaskProductAbstraction::BoundedSubtractionDiagonalV1,
        queue_bounds: vec![TaskQueueBound {
            taskset: "Ctl".to_owned(),
            task: "t1".to_owned(),
            arrival: TaskArrivalKind::Periodic,
            interarrival: int(4),
            relative_deadline: int(4),
            max_jitter: int(0),
            work_lower: int(0),
            work_upper: int(1),
            max_jobs: 2,
        }],
        clock_ceilings: Vec::new(),
        diagonal_constraints: Vec::new(),
        subtractions: Vec::new(),
        time_progress: TaskTimeProgress {
            method: TaskTimeProgressMethod::ClosedNoReleaseProductV1,
            live_job_cap: 0,
            closure: TaskClosureMethod::RecomputeAllSourceSuccessorsNoTaskReleaseV1,
        },
    });

    let result = check_zone_certificate(
        ZoneCertificateRequest {
            simulation: world.request(),
            model: "M",
            max_clocks: DEFAULT_MAX_CLOCKS,
            item: CertifiedItem::TaskSchedulability {
                claim,
                occurrence: &occurrence,
            },
        },
        &certificate.obligation,
        certificate_item,
        &certificate,
        CertificateCheckLimits::default(),
        &Cancel::new(),
    )
    .expect("the no-release certificate is checked independently");
    assert_eq!(result, ZoneCertificateVerdict::Accepted);
}

#[trace("TC-372", "FR-418-AC-11")]
#[test]
fn fixed_work_reader_rejects_changed_clock_metadata_and_missing_release_state() {
    let document = crate::timed_world::document(&[], &[("tick", &[])]);
    let world = crate::timed_world::world(
        &document,
        "",
        "dimension Time;\n\
         unit ms : Time = rational(1, 1);\n\
         time M tick M::Call::tick period 4 ms unit ms;\n\
         taskset Ctl {\n\
         task t1 periodic 4 ms deadline 4 ms bcet 1 ms wcet 1 ms priority 1 lo;\n\
         }\n\
         schedulable CtlFp on Ctl under fixed_priority;\n\
         release Ctl.t1 on Call::tick;\n",
    );
    let [claim] = world.package.graph().task_sets().claims() else {
        panic!("one checked schedulability claim");
    };
    let occurrence = claim.occurrence().expect("claim occurrence");
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("timed model");
    let mut certificate = match check_timed_model(
        ZoneCheckRequest {
            subject: &timed,
            item: ZoneCheckItem::TimeLockFreedom,
            obligation: ItemObligation {
                subject: "fixed-task-metadata",
                kind: ObligationKind::TimeLockFreedom,
                reductions: ObligationReductions::default(),
            },
            limits: ZoneCheckLimits::default(),
            reductions: &qsl_route::reductions::Reductions::default(),
        },
        &Cancel::new(),
    )
    .expect("the ordinary timed source is admitted")
    {
        ZoneCheckOutcome::Holds { certificate, .. } => *certificate,
        outcome => panic!("expected an ordinary source certificate, got {outcome:?}"),
    };
    certificate.obligation.kind = ObligationKind::TaskSchedulability(occurrence.clone());
    certificate.divergence = None;
    for node in &mut certificate.nodes {
        node.state_key = TaskProductStateKey::new(
            node.state_key.clone(),
            TaskProductStateKey::empty_scheduler_control(),
        )
        .into_state_key()
        .expect("joint product state key encodes");
    }
    certificate.task_product = Some(TaskProductCertificate {
        abstraction: TaskProductAbstraction::BoundedSubtractionDiagonalV1,
        queue_bounds: vec![TaskQueueBound {
            taskset: "Ctl".to_owned(),
            task: "t1".to_owned(),
            arrival: TaskArrivalKind::Periodic,
            interarrival: int(4),
            relative_deadline: int(4),
            max_jitter: int(0),
            work_lower: int(1),
            work_upper: int(1),
            max_jobs: 2,
        }],
        clock_ceilings: vec![
            TaskClockCeiling {
                clock: "task:Ctl:t1:arrival".to_owned(),
                maximum: Integer::from(4_i64),
            },
            TaskClockCeiling {
                clock: "task:Ctl:t1:slot0:service".to_owned(),
                maximum: Integer::from(4_i64),
            },
            TaskClockCeiling {
                clock: "task:Ctl:t1:slot0:deadline".to_owned(),
                maximum: Integer::from(4_i64),
            },
            TaskClockCeiling {
                clock: "task:Ctl:t1:slot1:service".to_owned(),
                maximum: Integer::from(4_i64),
            },
            TaskClockCeiling {
                clock: "task:Ctl:t1:slot1:deadline".to_owned(),
                maximum: Integer::from(4_i64),
            },
        ],
        diagonal_constraints: vec![
            TaskDiagonalConstraint {
                left: "task:Ctl:t1:slot0:service".to_owned(),
                operator: TaskDiagonalOperator::Less,
                right: "task:Ctl:t1:slot1:service".to_owned(),
                constant: Integer::from(1_i64),
            },
            TaskDiagonalConstraint {
                left: "task:Ctl:t1:slot1:service".to_owned(),
                operator: TaskDiagonalOperator::Less,
                right: "task:Ctl:t1:slot0:service".to_owned(),
                constant: Integer::from(1_i64),
            },
        ],
        subtractions: Vec::new(),
        time_progress: TaskTimeProgress {
            method: TaskTimeProgressMethod::ClosedFiniteProductV1,
            live_job_cap: 2,
            closure: TaskClosureMethod::RecomputeAllReachableSuccessorsV1,
        },
    });
    certificate
        .task_product
        .as_mut()
        .expect("task certificate")
        .clock_ceilings[0]
        .maximum = Integer::from(5_i64);

    let result = check_zone_certificate(
        ZoneCertificateRequest {
            simulation: world.request(),
            model: "M",
            max_clocks: DEFAULT_MAX_CLOCKS,
            item: CertifiedItem::TaskSchedulability {
                claim,
                occurrence: &occurrence,
            },
        },
        &certificate.obligation,
        certificate_item,
        &certificate,
        CertificateCheckLimits::default(),
        &Cancel::new(),
    )
    .expect("the forged clock metadata is a typed rejection");
    assert_eq!(
        result,
        ZoneCertificateVerdict::Rejected(qsl_replay::CertificateRejection {
            rule: CertificateRule::Malformed,
            at: CertificateLocus::CertificateMember(CertificateMember::TaskProduct),
        })
    );

    certificate
        .task_product
        .as_mut()
        .expect("task certificate")
        .clock_ceilings[0]
        .maximum = Integer::from(4_i64);
    let result = check_zone_certificate(
        ZoneCertificateRequest {
            simulation: world.request(),
            model: "M",
            max_clocks: DEFAULT_MAX_CLOCKS,
            item: CertifiedItem::TaskSchedulability {
                claim,
                occurrence: &occurrence,
            },
        },
        &certificate.obligation,
        certificate_item,
        &certificate,
        CertificateCheckLimits::default(),
        &Cancel::new(),
    )
    .expect("a missing derived activation is a typed certificate rejection");
    assert_eq!(
        result,
        ZoneCertificateVerdict::Rejected(qsl_replay::CertificateRejection {
            rule: CertificateRule::Malformed,
            at: CertificateLocus::CertificateMember(CertificateMember::TaskProduct),
        })
    );
}

#[trace("TC-372", "FR-418-AC-11", "FR-419-AC-1")]
#[test]
fn fixed_work_reader_accepts_independently_closed_product() {
    let document = crate::timed_world::document(&[], &[("go", &[])]);
    let world = crate::timed_world::world(
        &document,
        "",
        "dimension Time;\n\
         unit ms : Time = rational(1, 1);\n\
         time M dense unit ms;\n\
         taskset Ctl {\n\
         task t1 periodic 4 ms deadline 4 ms bcet 1 ms wcet 1 ms priority 1 lo;\n\
         }\n\
         schedulable CtlFp on Ctl under fixed_priority;\n\
         release Ctl.t1 on Call::go;\n",
    );
    let [claim] = world.package.graph().task_sets().claims() else {
        panic!("one checked schedulability claim");
    };
    let occurrence = claim.occurrence().expect("claim occurrence");
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("timed source");
    let obligation = ItemObligation {
        subject: "fixed-task-product-reader",
        kind: ObligationKind::TaskSchedulability(occurrence.clone()),
        reductions: ObligationReductions::default(),
    };
    let certificate = match check_timed_model(
        ZoneCheckRequest {
            subject: &timed,
            item: ZoneCheckItem::TaskSchedulability {
                claim,
                occurrence: &occurrence,
            },
            obligation,
            limits: ZoneCheckLimits::default(),
            reductions: &qsl_route::reductions::Reductions::default(),
        },
        &Cancel::new(),
    )
    .expect("fixed-work timed task subject is admitted")
    {
        ZoneCheckOutcome::Holds { certificate, .. } => *certificate,
        outcome => panic!("expected a closed fixed-work product, got {outcome:?}"),
    };
    let task_product = certificate.task_product.as_ref().expect("task product");
    assert_eq!(task_product.time_progress.live_job_cap, 2);
    let result = check_zone_certificate(
        ZoneCertificateRequest {
            simulation: world.request(),
            model: "M",
            max_clocks: DEFAULT_MAX_CLOCKS,
            item: CertifiedItem::TaskSchedulability {
                claim,
                occurrence: &occurrence,
            },
        },
        &certificate.obligation,
        certificate_item,
        &certificate,
        CertificateCheckLimits::default(),
        &Cancel::new(),
    )
    .expect("the independent fixed-work reader runs");
    assert_eq!(result, ZoneCertificateVerdict::Accepted);
}

#[trace("TC-372", "FR-419-AC-6", "FR-418-AC-14", "FR-364-AC-3")]
#[test]
fn fixed_work_miss_lasso_keeps_the_source_release_and_task_actions() {
    let document = crate::timed_world::document(&[], &[("go", &[])]);
    let world = crate::timed_world::world(
        &document,
        "",
        "dimension Time;\n\
         unit ms : Time = rational(1, 1);\n\
         time M dense unit ms;\n\
         taskset Ctl {\n\
         task t1 periodic 5 ms deadline 1 ms bcet 2 ms wcet 2 ms priority 1 lo;\n\
         task t2 periodic 5 ms deadline 5 ms bcet 1 ms wcet 1 ms priority 2 lo;\n\
         }\n\
         schedulable CtlFp on Ctl under fixed_priority;\n\
         release Ctl.t1 on Call::go;\n\
         release Ctl.t2 on Call::go;\n",
    );
    let [claim] = world.package.graph().task_sets().claims() else {
        panic!("one checked schedulability claim");
    };
    let occurrence = claim.occurrence().expect("claim occurrence");
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("timed source");
    let limits = ZoneCheckLimits::default();
    let outcome = check_timed_model(
        ZoneCheckRequest {
            subject: &timed,
            item: ZoneCheckItem::TaskSchedulability {
                claim,
                occurrence: &occurrence,
            },
            obligation: ItemObligation {
                subject: "task-product-miss-lasso",
                kind: ObligationKind::TaskSchedulability(occurrence.clone()),
                reductions: ObligationReductions::default(),
            },
            limits,
            reductions: &qsl_route::reductions::Reductions::default(),
        },
        &Cancel::new(),
    )
    .expect("the task subject is admitted");
    let ZoneCheckOutcome::TaskViolated { counterexample, .. } = outcome else {
        panic!("a deadline miss with a source-release continuation must refute: {outcome:?}");
    };
    let lasso = counterexample.lasso.expect("positive-time lasso");
    let task_set = world
        .package
        .graph()
        .task_sets()
        .task_set_of(claim)
        .expect("checked task set");
    let releases = world
        .package
        .graph()
        .task_sets()
        .releases()
        .iter()
        .filter(|release| release.task_set() == task_set.name)
        .cloned()
        .collect::<Vec<_>>();
    qsl_replay::replay_task_counterexample(
        &timed,
        task_set,
        &releases,
        qsl_semantics::check::SchedulingPolicy::FixedPriority,
        &counterexample,
    )
    .expect("independent replay reproduces the miss and typed lasso");
    assert!(!lasso.terminal_stutter);
    assert!(matches!(
        counterexample.prefix.first().map(|step| &step.transition),
        Some(qsl_replay::TaskSearchTransition::Source(_))
    ));
    let activations_before_dispatch: Vec<_> = counterexample
        .prefix
        .iter()
        .take_while(|step| {
            !matches!(
                step.transition,
                qsl_replay::TaskSearchTransition::Task(ref task)
                    if task.action == qsl_replay::TaskAction::Dispatch
            )
        })
        .filter_map(|step| match &step.transition {
            qsl_replay::TaskSearchTransition::Task(task)
                if task.action == qsl_replay::TaskAction::Activate =>
            {
                Some(task.task.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(activations_before_dispatch, ["t1", "t2"]);
    let miss = counterexample
        .prefix
        .iter()
        .position(|step| {
            matches!(&step.transition, qsl_replay::TaskSearchTransition::Task(task)
                if task.action == qsl_replay::TaskAction::DeadlineMiss)
        })
        .expect("typed deadline-miss edge");
    assert!(lasso.entry > miss, "the lasso must continue after the miss");
    let loop_steps = &counterexample.prefix[lasso.entry..];
    assert!(loop_steps
        .iter()
        .any(|step| matches!(step.transition, qsl_replay::TaskSearchTransition::Source(_))));
    assert!(loop_steps.iter().any(|step| matches!(
        &step.transition,
        qsl_replay::TaskSearchTransition::Task(task)
            if task.action == qsl_replay::TaskAction::Activate
    )));
    assert!(loop_steps.iter().any(|step| matches!(
        &step.transition,
        qsl_replay::TaskSearchTransition::Task(task)
            if task.action == qsl_replay::TaskAction::DeadlineMiss
    )));
    assert!(loop_steps
        .iter()
        .filter_map(|step| step.delay.as_ref())
        .any(|delay| delay > &int(0)));
    let loop_dispatch = loop_steps
        .iter()
        .find_map(|step| match &step.transition {
            qsl_replay::TaskSearchTransition::Task(task)
                if task.action == qsl_replay::TaskAction::Dispatch =>
            {
                Some(task)
            }
            _ => None,
        })
        .expect("the lasso dispatches a concretely active job");
    let loop_activation = counterexample.prefix[..lasso.entry]
        .iter()
        .rev()
        .find_map(|step| match &step.transition {
            qsl_replay::TaskSearchTransition::Task(task)
                if task.action == qsl_replay::TaskAction::Activate
                    && task.taskset == loop_dispatch.taskset
                    && task.task == loop_dispatch.task
                    && task.slot == loop_dispatch.slot
                    && task.release == loop_dispatch.release =>
            {
                Some(task)
            }
            _ => None,
        })
        .expect("the loop's active job has a concrete activation before the loop");
    assert_eq!(loop_dispatch.job, loop_activation.job);
    assert_eq!(loop_dispatch.generation, loop_activation.generation);
    let dispatch_index = counterexample
        .prefix
        .iter()
        .position(|step| {
            matches!(&step.transition, qsl_replay::TaskSearchTransition::Task(task)
            if task.action == qsl_replay::TaskAction::Dispatch
                && task.task == loop_dispatch.task
                && task.slot == loop_dispatch.slot)
        })
        .expect("typed dispatch edge in the lasso");
    let mut forged = counterexample.as_ref().clone();
    let qsl_replay::TaskSearchTransition::Task(task) =
        &mut forged.prefix[dispatch_index].transition
    else {
        panic!("task transition retained");
    };
    task.generation = task.generation.saturating_add(1);
    assert!(qsl_replay::replay_task_counterexample(
        &timed,
        task_set,
        &releases,
        qsl_semantics::check::SchedulingPolicy::FixedPriority,
        &forged,
    )
    .is_err());
    let mut stamp = int(0);
    let mut deadlines = Vec::new();
    for step in &counterexample.prefix {
        stamp = stamp.add(step.delay.as_ref().expect("task witness delay"));
        let qsl_replay::TaskSearchTransition::Task(identity) = &step.transition else {
            continue;
        };
        if identity.action == qsl_replay::TaskAction::Activate {
            let relative = match identity.task.as_str() {
                "t1" => int(1),
                "t2" => int(5),
                task => panic!("unexpected task {task}"),
            };
            let expected = stamp.add(&relative);
            assert_eq!(identity.deadline, expected);
            deadlines.push((identity.as_ref().clone(), expected));
        } else {
            let (_, expected) = deadlines
                .iter()
                .find(|(activated, _)| {
                    activated.task == identity.task
                        && activated.job == identity.job
                        && activated.slot == identity.slot
                        && activated.generation == identity.generation
                        && activated.release == identity.release
                })
                .expect("the task was activated before this action");
            assert_eq!(&identity.deadline, expected);
        }
    }
}

#[trace("TC-372", "FR-419-AC-6", "FR-418-AC-14", "FR-364-AC-3")]
#[test]
fn fixed_work_miss_without_positive_time_continuation_is_not_refuted() {
    let document =
        crate::timed_world::document(&["x"], &[("go", &["phase"]), ("finish", &["phase", "x"])]);
    let world = crate::timed_world::world(
        &document,
        "",
        "dimension Time;\n\
         unit ms : Time = rational(1, 1);\n\
         time M dense unit ms;\n\
         pre Start using v on M::Call::go { self.phase = 0 }\n\
         post Started using v on M::Call::go { self.phase = 1 }\n\
         pre Finish using v on M::Call::finish { self.phase = 1 and self.x >= 1 ms }\n\
         post Finished using v on M::Call::finish { self.phase = 2 and self.x = 0 ms }\n\
         time invariant Stalled when { self.phase = 2 } { self.x <= 0 ms }\n\
         taskset Ctl {\n\
         task t1 periodic 5 ms deadline 1 ms bcet 2 ms wcet 2 ms priority 1 lo;\n\
         }\n\
         schedulable CtlFp on Ctl under fixed_priority;\n\
         release Ctl.t1 on Call::go;\n",
    );
    let [claim] = world.package.graph().task_sets().claims() else {
        panic!("one checked schedulability claim");
    };
    let occurrence = claim.occurrence().expect("claim occurrence");
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("timed source");
    let outcome = check_timed_model(
        ZoneCheckRequest {
            subject: &timed,
            item: ZoneCheckItem::TaskSchedulability {
                claim,
                occurrence: &occurrence,
            },
            obligation: ItemObligation {
                subject: "task-product-time-locked-miss",
                kind: ObligationKind::TaskSchedulability(occurrence.clone()),
                reductions: ObligationReductions::default(),
            },
            limits: ZoneCheckLimits::default(),
            reductions: &qsl_route::reductions::Reductions::default(),
        },
        &Cancel::new(),
    )
    .expect("the task subject is admitted");
    match outcome {
        ZoneCheckOutcome::Undecided {
            cause: ZoneUndecided::LassoNotConcretized,
            ..
        } => {}
        other => panic!("a miss with no positive-time continuation is not refuted: {other:?}"),
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

#[trace("TC-372", "FR-419-AC-6")]
#[test]
fn admitted_exact_gate_misses_route_to_bounded_search() {
    let mut jittered = task("jittered", 4, 4, 1, Wcet::Single(int(1)));
    jittered.jitter = int(1);
    assert_eq!(
        task_product_route(&set(vec![jittered]), SchedulingPolicy::FixedPriority),
        Ok(TaskProductRoute::Bounded(
            TaskSearchReason::BoundedProductData
        ))
    );

    let variable = task("variable", 4, 4, 1, Wcet::Single(int(2)));
    assert_eq!(
        task_product_route(&set(vec![variable]), SchedulingPolicy::FixedPriority),
        Ok(TaskProductRoute::Bounded(TaskSearchReason::VariableDemand))
    );

    let invalid_edf = {
        let mut task = task("invalid-edf", 4, 4, 1, Wcet::Single(int(1)));
        task.jitter = int(1);
        task
    };
    assert_eq!(
        task_product_route(&set(vec![invalid_edf]), SchedulingPolicy::Edf),
        Err(TaskProductAdmissionError::InvalidTaskSet)
    );
}

#[trace("TC-372", "FR-419-AC-6")]
#[test]
fn bounded_unknown_carries_independently_replayed_demand_frontier() {
    let document = crate::timed_world::document(&["x"], &[("go", &["x"])]);
    let world = crate::timed_world::world(
        &document,
        "",
        "dimension Time;\n\
         unit ms : Time = rational(1, 1);\n\
         time M dense unit ms;\n\
         pre Go using v on M::Call::go { self.x >= 1 ms }\n\
         post ResetGo using v on M::Call::go { self.x = 0 ms }\n\
         taskset Ctl {\n\
         task t1 periodic 4 ms deadline 4 ms bcet 1 ms wcet 2 ms priority 1 lo;\n\
         }\n\
         schedulable CtlFp on Ctl under fixed_priority;\n\
         release Ctl.t1 on Call::go;\n",
    );
    let [claim] = world.package.graph().task_sets().claims() else {
        panic!("one checked schedulability claim");
    };
    let occurrence = claim.occurrence().expect("claim occurrence");
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("timed source");
    let outcome = check_timed_model(
        ZoneCheckRequest {
            subject: &timed,
            item: ZoneCheckItem::TaskSchedulability {
                claim,
                occurrence: &occurrence,
            },
            obligation: ItemObligation {
                subject: "bounded-task-frontier",
                kind: ObligationKind::TaskSchedulability(occurrence.clone()),
                reductions: ObligationReductions::default(),
            },
            limits: ZoneCheckLimits::default(),
            reductions: &qsl_route::reductions::Reductions::default(),
        },
        &Cancel::new(),
    )
    .expect("bounded task analysis returns a sound incomplete result");
    let ZoneCheckOutcome::Stopped {
        cause: qsl_analyze::zone_check::ZoneStop::TaskProductUnresolved,
        task_search: Some(evidence),
        ..
    } = outcome
    else {
        panic!("an unresolved demand/queue choice carries evidence: {outcome:?}");
    };
    assert!(evidence.limit.is_none());
    let task_set = world
        .package
        .graph()
        .task_sets()
        .task_set_of(claim)
        .expect("checked task set");
    let releases = world
        .package
        .graph()
        .task_sets()
        .releases()
        .iter()
        .filter(|release| release.task_set() == task_set.name)
        .cloned()
        .collect::<Vec<_>>();
    qsl_replay::replay_bounded_task_search_evidence(&timed, claim, task_set, &releases, &evidence)
        .expect("the independent reader replays every source edge and demand frontier");
    let mut omitted = evidence.as_ref().clone();
    omitted.edges.clear();
    assert!(qsl_replay::replay_bounded_task_search_evidence(
        &timed, claim, task_set, &releases, &omitted,
    )
    .is_err());
}

#[trace("TC-431", "FR-415-AC-26", "FR-419-AC-6")]
#[test]
fn bounded_task_work_limit_has_replayable_source_clock_frontier() {
    let document =
        crate::timed_world::document(&["x"], &[("start", &["phase"]), ("go", &["phase"])]);
    let world = crate::timed_world::world(
        &document,
        "",
        "dimension Time;\n\
         unit ms : Time = rational(1, 1);\n\
         time M dense unit ms;\n\
         pre Start using v on M::Call::start { self.phase = 0 }\n\
         post Started using v on M::Call::start { self.phase = 1 }\n\
         pre Run using v on M::Call::go { self.phase = 1 }\n\
         post Ran using v on M::Call::go { self.phase = 2 }\n\
         taskset Ctl {\n\
         task t1 periodic 4 ms deadline 4 ms bcet 1 ms wcet 2 ms priority 1 lo;\n\
         }\n\
         schedulable CtlFp on Ctl under fixed_priority;\n\
         release Ctl.t1 on Call::go;\n",
    );
    let [claim] = world.package.graph().task_sets().claims() else {
        panic!("one checked schedulability claim");
    };
    let occurrence = claim.occurrence().expect("claim occurrence");
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("timed source");
    let outcome = check_timed_model(
        ZoneCheckRequest {
            subject: &timed,
            item: ZoneCheckItem::TaskSchedulability {
                claim,
                occurrence: &occurrence,
            },
            obligation: ItemObligation {
                subject: "bounded-task-work-limit",
                kind: ObligationKind::TaskSchedulability(occurrence.clone()),
                reductions: ObligationReductions::default(),
            },
            limits: ZoneCheckLimits {
                max_task_product_work: 1,
                ..ZoneCheckLimits::default()
            },
            reductions: &qsl_route::reductions::Reductions::default(),
        },
        &Cancel::new(),
    )
    .expect("the configured work limit produces an incomplete outcome");
    let ZoneCheckOutcome::Stopped {
        cause:
            qsl_analyze::zone_check::ZoneStop::LimitReached(
                qsl_analyze::zone_check::ZoneCheckLimit::TaskProductWork(1),
            ),
        task_search: Some(evidence),
        ..
    } = outcome
    else {
        panic!("the work limit carries its actual frontier: {outcome:?}");
    };
    assert_eq!(
        evidence
            .limit
            .as_ref()
            .map(|limit| (limit.kind, limit.value)),
        Some((qsl_replay::TaskSearchLimitKind::Work, 1))
    );
    let task_set = world
        .package
        .graph()
        .task_sets()
        .task_set_of(claim)
        .expect("checked task set");
    let releases = world
        .package
        .graph()
        .task_sets()
        .releases()
        .iter()
        .filter(|release| release.task_set() == task_set.name)
        .cloned()
        .collect::<Vec<_>>();
    qsl_replay::replay_bounded_task_search_evidence(&timed, claim, task_set, &releases, &evidence)
        .expect("the reader validates the real work count and retained clocked source graph");
    let mut overstated_work = evidence.as_ref().clone();
    overstated_work.limit.as_mut().expect("work limit").value = 2;
    assert!(qsl_replay::replay_bounded_task_search_evidence(
        &timed,
        claim,
        task_set,
        &releases,
        &overstated_work,
    )
    .is_err());
}

#[trace("TC-372", "FR-419-AC-6", "FR-419-AC-10")]
#[test]
fn bounded_amc_frontier_records_high_criticality_budget_change() {
    let document = crate::timed_world::document(&["x"], &[("go", &[])]);
    let world = crate::timed_world::world(
        &document,
        "",
        "dimension Time;\n\
         unit ms : Time = rational(1, 1);\n\
         time M dense unit ms;\n\
         taskset Ctl {\n\
         task t1 periodic 4 ms deadline 4 ms bcet 1 ms wcet 1 ms priority 1 hi wcet_hi 2 ms;\n\
         }\n\
         schedulable CtlAmc on Ctl under amc;\n\
         release Ctl.t1 on Call::go;\n",
    );
    let [claim] = world.package.graph().task_sets().claims() else {
        panic!("one checked schedulability claim");
    };
    let occurrence = claim.occurrence().expect("claim occurrence");
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("timed source");
    let outcome = check_timed_model(
        ZoneCheckRequest {
            subject: &timed,
            item: ZoneCheckItem::TaskSchedulability {
                claim,
                occurrence: &occurrence,
            },
            obligation: ItemObligation {
                subject: "bounded-amc-frontier",
                kind: ObligationKind::TaskSchedulability(occurrence.clone()),
                reductions: ObligationReductions::default(),
            },
            limits: ZoneCheckLimits::default(),
            reductions: &qsl_route::reductions::Reductions::default(),
        },
        &Cancel::new(),
    )
    .expect("AMC analysis retains a sound incomplete frontier");
    let ZoneCheckOutcome::Stopped {
        cause: qsl_analyze::zone_check::ZoneStop::TaskProductUnresolved,
        task_search: Some(evidence),
        ..
    } = outcome
    else {
        panic!("the pending high-criticality job carries its frontier: {outcome:?}");
    };
    assert!(evidence.frontier.iter().any(|frontier| {
        frontier.open.contains(&qsl_replay::TaskSearchOpen::Demand)
            && frontier.open.contains(&qsl_replay::TaskSearchOpen::Queue)
            && frontier
                .open
                .contains(&qsl_replay::TaskSearchOpen::BudgetChange)
    }));
    let task_set = world
        .package
        .graph()
        .task_sets()
        .task_set_of(claim)
        .expect("checked task set");
    let releases = world
        .package
        .graph()
        .task_sets()
        .releases()
        .iter()
        .filter(|release| release.task_set() == task_set.name)
        .cloned()
        .collect::<Vec<_>>();
    qsl_replay::replay_bounded_task_search_evidence(&timed, claim, task_set, &releases, &evidence)
        .expect("the strict reader validates the admitted AMC budget-change frontier");
    let mut omitted_budget_change = evidence.as_ref().clone();
    for frontier in &mut omitted_budget_change.frontier {
        frontier
            .open
            .retain(|open| *open != qsl_replay::TaskSearchOpen::BudgetChange);
    }
    assert!(qsl_replay::replay_bounded_task_search_evidence(
        &timed,
        claim,
        task_set,
        &releases,
        &omitted_budget_change,
    )
    .is_err());
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

#[trace("TC-371", "FR-419-AC-6")]
#[test]
fn task_control_key_keeps_timing_in_the_dbm_not_discrete_identity() {
    let mut product = fixed_work_task_product(
        &set(vec![task("a", 4, 4, 1, Wcet::Single(int(1)))]),
        SchedulingPolicy::FixedPriority,
    )
    .expect("fixed-work task");
    product.tasks[0].max_jitter = int(2);
    let source = quire_semantic_value::model_transition::ModelTransition::new(
        "\"activate\"".to_owned(),
        None,
        Vec::new(),
        quire_canonical::Limits::new(1024),
    )
    .expect("source transition identity");
    let mut first = TaskSchedulerState::default();
    first
        .activate(&product, "a", source.clone(), int(1), int(0))
        .expect("first selected jitter");
    let mut second = TaskSchedulerState::default();
    second
        .activate(&product, "a", source, int(2), int(9))
        .expect("second selected jitter");

    assert_eq!(
        first.canonical_control_key(),
        second.canonical_control_key(),
        "per-job jitter/deadline are DBM values, not finite control state"
    );
}

#[trace("TC-372", "FR-419-AC-6")]
#[test]
fn activations_share_arrival_phase_and_occurrence_across_release_bindings() {
    let spec = task("a", 4, 4, 1, Wcet::Single(int(1)));
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
    let mut arrival_zone = Dbm::zero(2);
    assert!(scheduler
        .apply_arrival_guard(
            &product,
            "a",
            1,
            &num_bigint::BigInt::from(1),
            &mut arrival_zone,
        )
        .expect("first activation establishes phase"));
    let first = scheduler
        .activate(&product, "a", operation("start"), int(0), int(0))
        .expect("first activation establishes phase");
    assert_eq!(first.job, 0);
    let mut early = Dbm::zero(2);
    early.up();
    early
        .constrain(1, 0, Bound::le(num_bigint::BigInt::from(3)))
        .expect("clock upper bound");
    let before_early = early.clone();
    assert!(!scheduler
        .apply_arrival_guard(&product, "a", 1, &num_bigint::BigInt::from(1), &mut early,)
        .expect("an early cross-binding activation is disabled"));
    assert_eq!(early, before_early);
    let mut on_period = Dbm::zero(2);
    on_period.up();
    on_period
        .constrain(1, 0, Bound::le(num_bigint::BigInt::from(4)))
        .expect("clock upper bound");
    on_period
        .constrain(0, 1, Bound::le(num_bigint::BigInt::from(-4)))
        .expect("clock lower bound");
    assert!(scheduler
        .apply_arrival_guard(
            &product,
            "a",
            1,
            &num_bigint::BigInt::from(1),
            &mut on_period,
        )
        .expect("a distinct binding shares the exact periodic phase"));
    let second = scheduler
        .activate(&product, "a", operation("alternate"), int(0), int(4))
        .expect("the shared arrival guard permits the exact next activation");
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
    assert_eq!(subtractions[0].transition.action, completion.action);
    assert_eq!(subtractions[0].transition.taskset, completion.taskset);
    assert_eq!(subtractions[0].transition.task, completion.task);
    assert_eq!(subtractions[0].transition.job_rank, 0);
    assert_eq!(subtractions[0].transition.slot, completion.slot);
    assert_eq!(subtractions[0].transition.release, completion.release);
    assert_eq!(
        subtractions[0].transition.source_transition,
        completion.source_transition
    );
    assert_eq!(subtractions[0].transition.priority, completion.priority);
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
