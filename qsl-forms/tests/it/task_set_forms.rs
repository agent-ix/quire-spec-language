// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-419's `taskset-decl`, `task`, `signed-quantity` and
//! `schedulability-clause` productions: S2 builds each as written and defers
//! every meaning check (sign, emptiness, the `lo` task's `wcet_hi`, units,
//! names and policy) to S3, run on real source through `qsl_cst::parse` and
//! `build_unit`.

use ix_trace_rs::trace;
use qsl_forms::{
    build_unit, ArrivalForm, CriticalityForm, DeclarationForm, ExactValueKind, PolicyForm,
    SignedQuantityForm,
};
use qsl_foundation::SourceIdentity;
use quire_exact::Integer;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile t = \"quire.temporal.infinite-trace/v1\";\n";

fn build(declarations: &str) -> (String, qsl_forms::ParsedUnit) {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).unwrap_or_else(|failure| panic!("{text}: {failure:?}"));
    (text, unit)
}

/// The magnitude as written, its sign and its unit.
fn quantity(form: &SignedQuantityForm) -> (bool, u64, &str) {
    assert_eq!(form.quantity.magnitude.kind, ExactValueKind::Natural);
    (
        form.negative,
        form.quantity
            .magnitude
            .first
            .to_u64()
            .expect("a small magnitude"),
        form.quantity.unit.name.as_str(),
    )
}

#[trace("TC-701")]
#[test]
fn s2_builds_task_sets_and_claims_as_written() {
    let (text, unit) = build(
        "taskset Ctl {\n\
         task a periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo;\n\
         task b sporadic 6 ms deadline 5 ms wcet 2 ms priority 2 jitter 1 ms blocking 3 ms hi wcet_hi 3 ms;\n\
         }\n\
         schedulable CtlFp on Ctl under fixed_priority;\n\
         schedulable CtlEdf on Ctl under edf;\n\
         schedulable CtlAmc on Ctl under amc;\n\
         response R on Ctl.b <= 12 ms under fixed_priority;",
    );
    let forms: Vec<_> = unit.forms().iter().map(|form| form.form()).collect();
    let [DeclarationForm::Taskset(set), DeclarationForm::Schedulable(fp), DeclarationForm::Schedulable(edf), DeclarationForm::Schedulable(amc), DeclarationForm::Response(response)] =
        forms[..]
    else {
        panic!("a task set and four claims: {forms:?}");
    };
    assert_eq!(set.name.name, "Ctl");
    assert_eq!(&text[set.name.span.start..set.name.span.end], "Ctl");
    let [a, b] = &set.tasks[..] else {
        panic!("two tasks: {:?}", set.tasks);
    };
    assert_eq!(a.name.name, "a");
    assert_eq!(a.arrival, ArrivalForm::Periodic);
    assert_eq!(quantity(&a.interval), (false, 4, "ms"));
    assert_eq!(quantity(&a.wcet), (false, 1, "ms"));
    assert_eq!(
        a.priority.as_ref().map(|p| p.value.clone()),
        Some(Integer::from(1_i64))
    );
    assert_eq!(a.criticality, CriticalityForm::Lo);
    assert!(a.jitter.is_none() && a.blocking.is_none() && a.wcet_hi.is_none());
    assert_eq!(b.arrival, ArrivalForm::Sporadic);
    assert_eq!(quantity(&b.deadline), (false, 5, "ms"));
    assert_eq!(
        quantity(b.jitter.as_ref().expect("jitter")),
        (false, 1, "ms")
    );
    assert_eq!(
        quantity(b.blocking.as_ref().expect("blocking")),
        (false, 3, "ms")
    );
    assert_eq!(b.criticality, CriticalityForm::Hi);
    assert_eq!(
        quantity(b.wcet_hi.as_ref().expect("wcet_hi")),
        (false, 3, "ms")
    );
    assert_eq!(
        &text[b.span.start..b.span.end],
        "task b sporadic 6 ms deadline 5 ms wcet 2 ms priority 2 jitter 1 ms blocking 3 ms hi wcet_hi 3 ms;"
    );

    assert_eq!(
        (fp.name.name.as_str(), fp.task_set.name.as_str()),
        ("CtlFp", "Ctl")
    );
    assert_eq!(fp.policy, PolicyForm::FixedPriority);
    assert_eq!(
        &text[fp.policy_span.start..fp.policy_span.end],
        "fixed_priority"
    );
    assert_eq!(edf.policy, PolicyForm::Edf);
    assert_eq!(amc.policy, PolicyForm::Amc);
    assert_eq!(response.name.name, "R");
    assert_eq!(response.task_set.name, "Ctl");
    assert_eq!(response.task.name, "b");
    assert_eq!(response.bound.unit.name, "ms");
}

/// The grammar admits an empty task set, a negative value and a `lo` task
/// with a `wcet_hi`; S2 keeps each as written for S3 to refuse.
#[trace("TC-701")]
#[test]
fn s2_keeps_what_s3_refuses() {
    let (_, unit) = build(
        "taskset Empty { }\n\
         taskset Odd { task t periodic 4 ms deadline 3 m wcet 1 ms jitter -1 ms lo wcet_hi 2 ms; }",
    );
    let forms: Vec<_> = unit.forms().iter().map(|form| form.form()).collect();
    let [DeclarationForm::Taskset(empty), DeclarationForm::Taskset(odd)] = forms[..] else {
        panic!("two task sets: {forms:?}");
    };
    assert!(empty.tasks.is_empty());
    let [task] = &odd.tasks[..] else {
        panic!("one task");
    };
    assert_eq!(quantity(&task.deadline), (false, 3, "m"));
    assert_eq!(
        quantity(task.jitter.as_ref().expect("jitter")),
        (true, 1, "ms")
    );
    assert_eq!(task.criticality, CriticalityForm::Lo);
    assert!(task.wcet_hi.is_some());
}

/// The member words are contextual (QSpec FR-419), so a task may be named by
/// one and S2 still reads each member by its position.
#[trace("TC-701")]
#[test]
fn a_task_named_by_a_member_word_keeps_its_members() {
    let (_, unit) = build(
        "taskset Ctl {\n\
         task lo periodic 4 ms deadline 5 ms wcet 1 ms lo;\n\
         task hi sporadic 6 ms deadline 6 ms wcet 2 ms hi wcet_hi 3 ms;\n\
         task wcet_hi periodic 8 ms deadline 8 ms wcet 1 ms priority 3 lo;\n\
         task periodic periodic 9 ms deadline 9 ms wcet 1 ms lo;\n\
         }",
    );
    let forms: Vec<_> = unit.forms().iter().map(|form| form.form()).collect();
    let [DeclarationForm::Taskset(set)] = forms[..] else {
        panic!("one task set: {forms:?}");
    };
    let names: Vec<_> = set
        .tasks
        .iter()
        .map(|task| task.name.name.as_str())
        .collect();
    assert_eq!(names, ["lo", "hi", "wcet_hi", "periodic"]);
    let [lo, hi, wcet_hi, periodic] = &set.tasks[..] else {
        panic!("four tasks");
    };
    assert_eq!(lo.criticality, CriticalityForm::Lo);
    assert_eq!(quantity(&lo.deadline), (false, 5, "ms"));
    assert_eq!(hi.criticality, CriticalityForm::Hi);
    assert_eq!(
        quantity(hi.wcet_hi.as_ref().expect("wcet_hi")),
        (false, 3, "ms")
    );
    assert_eq!(
        wcet_hi.priority.as_ref().map(|p| p.value.clone()),
        Some(Integer::from(3_i64))
    );
    assert_eq!(periodic.arrival, ArrivalForm::Periodic);
    assert_eq!(quantity(&periodic.interval), (false, 9, "ms"));
}
