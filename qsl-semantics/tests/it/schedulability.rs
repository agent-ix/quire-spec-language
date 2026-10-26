// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-246 task-set S3 admission and canonical exact time-unit selection.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::SourceIdentity;
use qsl_semantics::check::{
    CheckCause, CheckedArrival, CheckedSchedulabilityClaimKind, PackageDeclarations,
};
use quire_exact::Rational;
use quire_semantic_value::checking::CheckingLimits;

const PREFIX: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    dimension Time;\n\
    unit ms : Time = rational(1, 1);\n\
    unit us : Time = rational(1, 1000) * ms;\n\
    dimension Length;\n\
    unit m : Length = rational(1, 1);\n";

fn declarations(
    source: &str,
) -> Result<qsl_semantics::check::CheckedGraph, Vec<qsl_semantics::check::CheckRefusal>> {
    let text = format!("{PREFIX}{source}");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("test", "qsl-861", "fixture", "fixture:1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the package");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let forms = qsl_forms::build_unit(&parsed).expect("S2 builds task-set forms");
    let assembled = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        forms,
        qsl_semantics::model::intake::SelectedModels::default(),
        Vec::new(),
    )
    .expect("the declaration unit assembles");
    assembled.check(CheckingLimits::default())
}

fn rational(numerator: i64) -> Rational {
    Rational::from_integer(quire_exact::Integer::from(numerator))
}

const C_SET: &str = "taskset Ctl {\n\
    task c periodic 12 ms deadline 12 ms wcet 5 ms priority 3 lo;\n\
    task b periodic 6 ms deadline 6 ms wcet 2 ms priority 2 lo;\n\
    task a periodic 4000 us deadline 4 ms wcet 1 ms priority 1 lo;\
}\n\
schedulable CtlFp on Ctl under fixed_priority;\n\
response R on Ctl.c <= 12 ms under fixed_priority;\n";

const REORDERED_SET: &str = "taskset Ctl {\n\
    task a periodic 4000 us deadline 4 ms wcet 1 ms priority 1 lo;\n\
    task b periodic 6 ms deadline 6 ms wcet 2 ms priority 2 lo;\n\
    task c periodic 12 ms deadline 12 ms wcet 5 ms priority 3 lo;\n\
}\n\
schedulable CtlFp on Ctl under fixed_priority;\n\
response R on Ctl.c <= 12 ms under fixed_priority;\n";

#[trace("FR-246-AC-3")]
#[test]
fn task_name_order_selects_unit_and_all_conversions_are_exact_and_reorder_invariant() {
    let first = declarations(C_SET).expect("the first source order checks");
    let second = declarations(REORDERED_SET).expect("the reordered source checks");
    let first_set = &first.schedulability().task_sets[0];
    let second_set = &second.schedulability().task_sets[0];
    assert_eq!(first_set.unit.as_deref(), Some("us"));
    assert_eq!(
        first_set
            .tasks
            .iter()
            .map(|task| (
                task.name.as_str(),
                &task.arrival,
                &task.deadline,
                &task.wcet,
                task.priority,
                task.criticality,
                &task.jitter,
                &task.blocking,
            ))
            .collect::<Vec<_>>(),
        second_set
            .tasks
            .iter()
            .map(|task| (
                task.name.as_str(),
                &task.arrival,
                &task.deadline,
                &task.wcet,
                task.priority,
                task.criticality,
                &task.jitter,
                &task.blocking,
            ))
            .collect::<Vec<_>>(),
    );
    assert_eq!(
        first_set
            .tasks
            .iter()
            .map(|task| task.name.as_str())
            .collect::<Vec<_>>(),
        ["a", "b", "c"]
    );
    assert_eq!(first_set.tasks[0].deadline, rational(4000));
    assert_eq!(first_set.tasks[1].deadline, rational(6000));
    assert_eq!(first_set.tasks[2].deadline, rational(12000));
    let [_, response] = first.schedulability().claims.as_slice() else {
        panic!("two claims")
    };
    assert!(
        matches!(&response.kind, CheckedSchedulabilityClaimKind::Response { task, bound, bound_unit }
        if task == "c" && *bound == rational(12000) && bound_unit == "us")
    );
    assert!(matches!(
        first_set.tasks[0].arrival,
        CheckedArrival::Periodic(_)
    ));
    assert_eq!(response.capability.to_wire(), "schedulability");
    let changed = declarations(&C_SET.replace("wcet 5 ms", "wcet 6 ms"))
        .expect("the changed premise remains valid");
    assert_ne!(response.subject, changed.schedulability().claims[0].subject);
}

#[trace("FR-246-AC-2")]
#[test]
fn a_length_quantity_refuses_as_type_mismatch_at_its_literal() {
    let refusals =
        declarations("taskset Ctl { task a periodic 4 ms deadline 3 m wcet 1 ms priority 1 lo; }")
            .expect_err("3 m is not a time quantity");
    let [refusal] = refusals.as_slice() else {
        panic!("one typed refusal: {refusals:?}")
    };
    assert_eq!(refusal.cause.code(), Code::IllTyped);
    assert_eq!(refusal.cause.cause(), Some("type-mismatch"));
    let CheckCause::Schedulability(cause) = &refusal.cause else {
        panic!("FR-246 refusal")
    };
    assert!(matches!(
        &**cause,
        qsl_semantics::check::SchedulabilityRefusal::TimeUnit { .. }
    ));
    assert_eq!(cause.span().end - cause.span().start, "3 m".len());
}

#[trace("FR-246-AC-2")]
#[test]
fn an_empty_set_is_a_named_invalid_task_set() {
    let refusals = declarations("taskset Empty { }").expect_err("an empty task set refuses");
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal: {refusals:?}")
    };
    assert_eq!(refusal.cause.code(), Code::InvalidTimedModel);
    assert_eq!(refusal.cause.cause(), Some("invalid-task-set"));
}

#[trace("FR-246-AC-2")]
#[test]
fn task_set_and_policy_constraints_refuse_with_typed_causes() {
    let cases = [
        (
            "taskset Ctl { task a periodic 0 ms deadline 3 ms wcet 1 ms priority 1 lo; }",
            Code::InvalidTimedModel,
            "invalid-task-set",
        ),
        (
            "taskset Ctl { task a periodic 4 ms deadline 0 ms wcet 1 ms priority 1 lo; }",
            Code::InvalidTimedModel,
            "invalid-task-set",
        ),
        (
            "taskset Ctl { task a periodic 4 ms deadline 3 ms wcet 0 ms priority 1 lo; }",
            Code::InvalidTimedModel,
            "invalid-task-set",
        ),
        (
            "taskset Ctl { task a periodic 4 ms deadline 3 ms wcet 1 ms priority 1 lo jitter -1 ms; }",
            Code::InvalidTimedModel,
            "invalid-task-set",
        ),
        (
            "taskset Ctl { task a periodic 4 ms deadline 3 ms wcet 1 ms priority 1 lo blocking -1 ms; }",
            Code::InvalidTimedModel,
            "invalid-task-set",
        ),
        (
            "taskset Ctl { task a sporadic 0 ms deadline 3 ms wcet 1 ms priority 1 lo; }",
            Code::InvalidTimedModel,
            "invalid-task-set",
        ),
        (
            "taskset Ctl { task a periodic 4 ms deadline 3 ms wcet 3 ms priority 1 hi wcet_hi 2 ms; }",
            Code::InvalidTimedModel,
            "invalid-task-set",
        ),
        (
            "taskset Ctl { task a periodic 4 ms deadline 3 ms wcet 1 ms priority 1 lo wcet_hi 2 ms; }",
            Code::InvalidTimedModel,
            "invalid-task-set",
        ),
        (
            "taskset Ctl { task a periodic 4 ms deadline 3 ms wcet 1 ms lo; } schedulable C on Ctl under fixed_priority;",
            Code::InvalidTimedModel,
            "invalid-task-set",
        ),
        (
            "taskset Ctl { task a periodic 4 ms deadline 3 ms wcet 1 ms priority 1 lo; task b periodic 5 ms deadline 5 ms wcet 1 ms priority 1 lo; } schedulable C on Ctl under fixed_priority;",
            Code::InvalidTimedModel,
            "invalid-task-set",
        ),
        (
            "taskset Ctl { task a periodic 4 ms deadline 3 ms wcet 1 ms priority 1 lo blocking 1 ms; } schedulable C on Ctl under edf;",
            Code::InvalidTimedModel,
            "invalid-task-set",
        ),
        (
            "taskset Ctl { task a periodic 4 ms deadline 3 ms wcet 1 ms priority 1 lo jitter 1 ms; } schedulable C on Ctl under edf;",
            Code::InvalidTimedModel,
            "invalid-task-set",
        ),
        (
            "taskset Ctl { task a periodic 4 ms deadline 5 ms wcet 1 ms priority 1 lo; } schedulable C on Ctl under amc;",
            Code::InvalidTimedModel,
            "invalid-task-set",
        ),
        (
            "taskset Ctl { task a periodic 4 ms deadline 3 ms wcet 1 ms lo; } response R on Ctl.a <= 4 ms under fixed_priority;",
            Code::InvalidTimedModel,
            "invalid-task-set",
        ),
        (
            "taskset Ctl { task a periodic 4 ms deadline 3 ms wcet 1 ms priority 1 lo; } response R on Ctl.missing <= 4 ms under fixed_priority;",
            Code::MissingDeclaration,
            "missing-name",
        ),
    ];

    for (source, code, cause) in cases {
        let refusals = declarations(source).expect_err("the declared task-set defect refuses");
        assert!(
            refusals.iter().any(|refusal| refusal.cause.code() == code && refusal.cause.cause() == Some(cause)),
            "expected {code:?}/{cause} in {refusals:?} for {source}"
        );
    }
}

#[trace("FR-246-AC-2")]
#[test]
fn edf_ignores_missing_and_duplicate_priorities_and_duplicate_task_names_refuse() {
    for tasks in [
        "task a periodic 4 ms deadline 3 ms wcet 1 ms lo;",
        "task a periodic 4 ms deadline 3 ms wcet 1 ms priority 1 lo; task b periodic 5 ms deadline 4 ms wcet 1 ms priority 1 lo;",
    ] {
        let checked = declarations(&format!(
            "taskset Ctl {{ {tasks} }} schedulable E on Ctl under edf;"
        ))
        .expect("EDF does not read priorities");
        assert_eq!(checked.schedulability().claims[0].capability.to_wire(), "schedulability");
    }

    let refusals = declarations(
        "taskset Ctl { task a periodic 4 ms deadline 3 ms wcet 1 ms priority 1 lo; task a periodic 5 ms deadline 4 ms wcet 1 ms priority 2 lo; }",
    )
    .expect_err("duplicate task names refuse");
    assert!(refusals.iter().any(|refusal| {
        refusal.cause.code() == Code::ConflictingDeclaration
            && refusal.cause.cause() == Some("duplicate-declaration")
    }));
}

#[trace("FR-246-AC-3")]
#[test]
fn all_task_set_defects_precede_claim_defects_and_keep_source_order() {
    let source = "taskset Ctl { task a periodic 0 ms deadline 0 ms wcet 0 ms priority 1 lo jitter -1 ms blocking -1 ms; } schedulable C on Ctl under edf;";
    let refusals = declarations(source).expect_err("the set and EDF claim both have defects");
    let found = refusals
        .iter()
        .map(|refusal| {
            let CheckCause::Schedulability(cause) = &refusal.cause else {
                panic!("task-set cause: {refusal:?}");
            };
            (cause.span().start, cause.tag())
        })
        .collect::<Vec<_>>();
    assert_eq!(
        found.len(),
        6,
        "five own defects and one EDF defect: {found:?}"
    );
    assert!(
        found[..5].windows(2).all(|pair| pair[0].0 <= pair[1].0),
        "{found:?}"
    );

    let with_claim = format!("{source} schedulable E on Ctl under edf;");
    let with_claim =
        declarations(&with_claim).expect_err("both claim checks run despite set defects");
    assert!(
        with_claim.len() >= 7,
        "claim-specific nonzero-term refusals were lost: {with_claim:?}"
    );
}

#[trace("FR-246-AC-2")]
#[test]
fn an_ill_typed_time_value_does_not_hide_priority_or_reference_defects() {
    let refusals = declarations(
        "taskset Ctl { task a periodic 4 ms deadline 3 m wcet 1 ms lo; } response R on Ctl.missing <= 4 ms under fixed_priority;",
    )
    .expect_err("the time type, missing priority, and task reference each refuse");
    let observed = refusals
        .iter()
        .map(|refusal| {
            let CheckCause::Schedulability(cause) = &refusal.cause else {
                panic!("schedulability cause: {refusal:?}");
            };
            cause.tag()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        observed,
        ["type-mismatch", "invalid-task-set", "missing-name"]
    );
}
