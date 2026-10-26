// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL FR-246 (TC-701): S3 checks a unit's task sets and the claims over them,
//! converts every time value exactly into the task set's unit (QSpec FR-142),
//! refuses each defect with its code at its span, and reports every defect in
//! a fixed order. Run over TC-458's `ConfigVersion` fixture package
//! (`crate::model_operations`).

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{
    CheckCause, CheckedGraph, ScheduleClaim, SchedulingPolicy, TaskSetDefect, TaskTimeMember,
    TemporalCause, Wcet,
};
use quire_exact::{Integer, Rational};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::json;

use crate::model_operations::{
    admit_and_assemble_with_profiles, attempt_update_modifies_version_number,
    config_unit_with_profiles, config_version_document,
};

const PROFILES: &str = "profile t = \"quire.temporal.complete/v1\";\n";

const UNITS: &str = "dimension Time;\n\
     dimension Length;\n\
     unit s : Time = rational(1, 1);\n\
     unit ms : Time = rational(1, 1000) * s;\n\
     unit us : Time = rational(1, 1000000) * s;\n\
     unit m : Length = rational(1, 1);\n";

/// ADR-026 section 12's `Ctl`, `(C, T = D)` of `(1, 4)`, `(2, 6)` and
/// `(5, 12)` ms, priorities by period.
fn ctl(third_wcet: u32) -> String {
    format!(
        "taskset Ctl {{\n\
         task a periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo;\n\
         task b periodic 6 ms deadline 6 ms wcet 2 ms priority 2 lo;\n\
         task c periodic 12 ms deadline 12 ms wcet {third_wcet} ms priority 3 lo;\n\
         }}\n"
    )
}

fn document() -> Vec<u8> {
    config_version_document(
        attempt_update_modifies_version_number(),
        Vec::new(),
        Vec::new(),
        json!([]),
    )
}

fn unit_text(declarations: &str) -> String {
    format!("{UNITS}{declarations}")
}

fn check(declarations: &str) -> Result<CheckedGraph, Vec<qsl_semantics::check::CheckRefusal>> {
    admit_and_assemble_with_profiles(&document(), PROFILES, &unit_text(declarations))
        .expect("S2 keeps the task-set forms")
        .check(CheckingLimits::default())
}

fn checked(declarations: &str) -> CheckedGraph {
    check(declarations).unwrap_or_else(|refusals| panic!("{declarations}: {refusals:?}"))
}

/// One reported refusal: its code, cause, the names it carries and the text
/// of its span.
#[derive(Debug, Eq, PartialEq)]
struct Reported {
    code: Code,
    cause: &'static str,
    names: Vec<String>,
    at: String,
}

fn reported(declarations: &str) -> Vec<Reported> {
    let text = config_unit_with_profiles(&document(), PROFILES, &unit_text(declarations)).0;
    check(declarations)
        .expect_err("S3 refuses")
        .iter()
        .map(|refusal| {
            let CheckCause::Temporal(cause) = &refusal.cause else {
                panic!("a temporal refusal: {refusal:?}");
            };
            let span = cause.span();
            let names = match &**cause {
                TemporalCause::InvalidTaskSet {
                    task_set, tasks, ..
                } if tasks.is_empty() => vec![task_set.clone()],
                TemporalCause::InvalidTaskSet { tasks, .. } => tasks.clone(),
                TemporalCause::DuplicateTask { task, .. } => vec![task.clone()],
                TemporalCause::UnknownResponseTask { task, .. } => vec![task.clone()],
                _ => Vec::new(),
            };
            Reported {
                code: cause.code(),
                cause: cause.tag(),
                names,
                at: text[span.start..span.end].to_owned(),
            }
        })
        .collect()
}

fn invalid(names: &[&str], at: &str) -> Reported {
    Reported {
        code: Code::InvalidTimedModel,
        cause: "invalid-task-set",
        names: names.iter().map(|name| (*name).to_owned()).collect(),
        at: at.to_owned(),
    }
}

fn int(value: i64) -> Rational {
    Rational::from_integer(Integer::from(value))
}

/// FR-246-AC-1 (the check and routing halves; identity needs FR-454's node,
/// which the checked-package model has no form for yet): `Ctl` checks, and its
/// two claims check with their policies and the `schedulability` kind.
#[trace("TC-701")]
#[test]
fn ctl_checks_and_its_claims_carry_the_schedulability_kind() {
    let graph = checked(&format!(
        "{}schedulable CtlFp on Ctl under fixed_priority;\nschedulable CtlEdf on Ctl under edf;\n",
        ctl(5)
    ));
    let sets = graph.task_sets();
    let [set] = sets.task_sets() else {
        panic!("one task set: {:?}", sets.task_sets());
    };
    assert_eq!(set.name, "Ctl");
    assert_eq!(set.unit.as_ref().map(|unit| unit.name.as_str()), Some("ms"));
    let [a, b, c] = &set.tasks[..] else {
        panic!("three tasks");
    };
    assert_eq!(
        (a.priority, b.priority, c.priority),
        (Some(1), Some(2), Some(3))
    );
    assert_eq!(c.wcet, Wcet::Single(int(5)));
    assert_eq!(c.deadline, int(12));
    assert_eq!(c.jitter, int(0));
    let [fp, edf] = sets.claims() else {
        panic!("two claims: {:?}", sets.claims());
    };
    assert_eq!(fp.name, "CtlFp");
    assert_eq!(
        fp.claim,
        ScheduleClaim::Schedulable(SchedulingPolicy::FixedPriority)
    );
    assert_eq!(edf.claim, ScheduleClaim::Schedulable(SchedulingPolicy::Edf));
    for claim in [fp, edf] {
        assert_eq!(
            claim.requirements().kind(),
            qsl_semantics::check::Capability::Schedulability
        );
        assert!(matches!(
            claim.requirements().extent(),
            qsl_semantics::family::ClaimExtent::Bounded
        ));
    }
}

/// FR-246-AC-2: each defect refuses with its catalog code, naming the task
/// (the task set for the empty one, both tasks for a clash), at its span.
#[trace("TC-701", "FR-246-AC-2")]
#[test]
fn each_defect_refuses_with_its_code_naming_its_task() {
    let set = |body: &str| format!("taskset Ctl {{ {body} }}\n");
    let task = "task c periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo;";
    let with = |claim: &str, body: &str| format!("{}{claim}\n", set(body));
    let fp = "schedulable S on Ctl under fixed_priority;";
    let cases: Vec<(String, Vec<Reported>)> = vec![
        (set(""), vec![invalid(&["Ctl"], "taskset Ctl {  }")]),
        (
            set("task c periodic 0 ms deadline 4 ms wcet 1 ms priority 1 lo;"),
            vec![invalid(&["c"], "0 ms")],
        ),
        (
            set("task c periodic 4 ms deadline 0 ms wcet 1 ms priority 1 lo;"),
            vec![invalid(&["c"], "0 ms")],
        ),
        (
            set("task c periodic 4 ms deadline 4 ms wcet 0 ms priority 1 lo;"),
            vec![invalid(&["c"], "0 ms")],
        ),
        (
            set("task c periodic 4 ms deadline 4 ms wcet 1 ms priority 1 jitter -1 ms lo;"),
            vec![invalid(&["c"], "-1 ms")],
        ),
        (
            set("task c periodic 4 ms deadline 4 ms wcet 1 ms priority 1 blocking -1 ms lo;"),
            vec![invalid(&["c"], "-1 ms")],
        ),
        (
            set("task c sporadic 0 ms deadline 4 ms wcet 1 ms priority 1 lo;"),
            vec![invalid(&["c"], "0 ms")],
        ),
        (
            set("task c periodic 4 ms deadline 4 ms wcet 3 ms priority 1 hi wcet_hi 2 ms;"),
            vec![invalid(&["c"], "3 ms")],
        ),
        (
            set("task c periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo wcet_hi 2 ms;"),
            vec![invalid(&["c"], "2 ms")],
        ),
        (
            with(fp, "task c periodic 4 ms deadline 4 ms wcet 1 ms lo;"),
            vec![invalid(&["c"], "c")],
        ),
        (
            with(
                fp,
                "task a periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo; \
                 task b periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo;",
            ),
            vec![invalid(&["a", "b"], "b")],
        ),
        (
            with(
                "schedulable S on Ctl under edf;",
                "task c periodic 4 ms deadline 4 ms wcet 1 ms priority 1 blocking 1 ms lo;",
            ),
            vec![invalid(&["c"], "1 ms")],
        ),
        (
            with(
                "schedulable S on Ctl under edf;",
                "task c periodic 4 ms deadline 4 ms wcet 1 ms priority 1 jitter 1 ms lo;",
            ),
            vec![invalid(&["c"], "1 ms")],
        ),
        (
            with(
                "schedulable S on Ctl under amc;",
                "task a periodic 4 ms deadline 4 ms wcet 1 ms lo; \
                 task b periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo; \
                 task d periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo;",
            ),
            vec![invalid(&["a"], "a"), invalid(&["b", "d"], "d")],
        ),
        (
            with(
                "response R on Ctl.c <= 12 ms under fixed_priority;",
                "task c periodic 4 ms deadline 4 ms wcet 1 ms lo;",
            ),
            vec![invalid(&["c"], "c")],
        ),
        (
            with(
                "response R on Ctl.c <= 12 ms under fixed_priority;",
                "task c periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo; \
                 task d periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo;",
            ),
            vec![invalid(&["c", "d"], "d")],
        ),
        (
            with(
                "schedulable S on Ctl under amc;",
                "task c periodic 4 ms deadline 5 ms wcet 1 ms priority 1 lo;",
            ),
            vec![invalid(&["c"], "5 ms")],
        ),
        (
            with(
                "schedulable S on Ctl under fixed_priority;",
                "task a periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo; \
                 task a periodic 4 ms deadline 4 ms wcet 1 ms priority 2 lo;",
            ),
            vec![Reported {
                code: Code::ConflictingDeclaration,
                cause: "duplicate-declaration",
                names: vec!["a".to_owned()],
                at: "a".to_owned(),
            }],
        ),
        (
            with("response R on Ctl.zz <= 5 ms under fixed_priority;", task),
            vec![Reported {
                code: Code::MissingDeclaration,
                cause: "missing-name",
                names: vec!["zz".to_owned()],
                at: "zz".to_owned(),
            }],
        ),
        (
            set("task c periodic 4 ms deadline 3 m wcet 1 ms priority 1 lo;"),
            vec![Reported {
                code: Code::IllTyped,
                cause: "type-mismatch",
                names: Vec::new(),
                at: "3 m".to_owned(),
            }],
        ),
    ];
    for (declarations, expected) in cases {
        assert_eq!(reported(&declarations), expected, "{declarations}");
    }
    // The same priority clash and a task with no priority under `edf` check.
    checked(
        "taskset Ctl { task a periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo; \
         task b periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo; \
         task c periodic 4 ms deadline 4 ms wcet 1 ms lo; }\n\
         schedulable S on Ctl under edf;\n",
    );
}

/// FR-246's unit rule (FR-246-AC-3's unit halves): the task set's unit is the
/// unit of the first time quantity of the first task in canonical task-name
/// order, every value and the `response` bound converted exactly into it.
#[trace("TC-701")]
#[test]
fn the_unit_is_the_first_task_s_and_every_value_converts_into_it() {
    let graph = checked(&format!(
        "{}response R on Ctl.c <= 12 ms under fixed_priority;\n",
        ctl(5).replace("task a periodic 4 ms", "task a periodic 4000 us")
    ));
    let sets = graph.task_sets();
    let set = &sets.task_sets()[0];
    assert_eq!(set.unit.as_ref().map(|unit| unit.name.as_str()), Some("us"));
    assert_eq!(set.tasks[0].deadline, int(4000));
    assert_eq!(set.tasks[2].deadline, int(12_000));
    assert_eq!(set.tasks[2].wcet, Wcet::Single(int(5000)));
    assert_eq!(
        sets.claims()[0].claim,
        ScheduleClaim::Response {
            task: "c".to_owned(),
            bound: int(12_000)
        }
    );
    // Listing the tasks in another order leaves the unit and every value
    // unchanged (QSpec FR-482).
    let reordered = checked(
        "taskset Ctl {\n\
         task c periodic 12 ms deadline 12 ms wcet 5 ms priority 3 lo;\n\
         task a periodic 4000 us deadline 4 ms wcet 1 ms priority 1 lo;\n\
         }\n",
    );
    let set = &reordered.task_sets().task_sets()[0];
    assert_eq!(set.unit.as_ref().map(|unit| unit.name.as_str()), Some("us"));
    assert_eq!(set.tasks[0].deadline, int(12_000));
}

/// FR-246's several-defect order (FR-246-AC-3's refusal halves).
#[trace("TC-701")]
#[test]
fn every_defect_is_reported_in_the_stated_order() {
    let tasks = "taskset Ctl {\n\
         task t1 periodic 4 ms deadline 4 ms wcet 1 ms jitter -1 ms lo;\n\
         task t2 periodic 6 ms deadline 6 ms wcet 2 ms blocking -1 ms lo;\n\
         }\n\
         schedulable CtlEdf on Ctl under edf;\n";
    assert_eq!(
        reported(tasks),
        vec![
            invalid(&["t1"], "-1 ms"),
            invalid(&["t2"], "-1 ms"),
            invalid(&["t1"], "-1 ms"),
            invalid(&["t2"], "-1 ms"),
        ]
    );
    let duplicates = reported(
        "taskset Ctl {\n\
         task a periodic 4 ms deadline 4 ms wcet 1 ms lo;\n\
         task a periodic 4 ms deadline 4 ms wcet 1 ms lo;\n\
         task z periodic 0 ms deadline 4 ms wcet 1 ms lo;\n\
         }\n",
    );
    assert_eq!(duplicates.len(), 2);
    assert_eq!(
        (duplicates[0].code, duplicates[0].cause),
        (Code::ConflictingDeclaration, "duplicate-declaration")
    );
    assert_eq!(duplicates[1], invalid(&["z"], "0 ms"));
    assert_eq!(
        reported("taskset Empty { }\nresponse R on Empty.c <= 12 ms under fixed_priority;\n"),
        vec![
            invalid(&["Empty"], "taskset Empty { }"),
            Reported {
                code: Code::MissingDeclaration,
                cause: "missing-name",
                names: vec!["c".to_owned()],
                at: "c".to_owned(),
            }
        ]
    );
    let _ = (TaskSetDefect::NoTask, TaskTimeMember::Period);
}
