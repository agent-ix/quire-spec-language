// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-246's shared grammar is parsed into span-carrying task and claim forms.

use ix_trace_rs::trace;
use qsl_forms::{
    build_unit, DeclarationForm, SchedulabilityClaimKind, TaskArrivalKind, TaskCriticality,
};
use qsl_foundation::SourceIdentity;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"quire.value.complete/v1\";\n";

fn forms(source: &str) -> Vec<DeclarationForm> {
    let text = format!("{HEADER}{source}");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    build_unit(&parsed)
        .expect("S2 constructs the shared forms")
        .forms()
        .iter()
        .map(|form| form.form().clone())
        .collect()
}

#[trace("FR-246-AC-1", "FR-246-AC-2")]
#[test]
fn taskset_and_claim_forms_keep_members_and_signed_quantities() {
    let forms = forms(
        "taskset Ctl { task t1 periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo; task t2 sporadic -1 ms deadline 6 ms wcet 2 ms priority 2 hi wcet_hi 2 ms jitter -1 ms blocking 0 ms; }\n\
         schedulable CtlFp on Ctl under fixed_priority;\n\
         response T3 on Ctl.t2 <= 12 ms under fixed_priority;",
    );
    let [DeclarationForm::TaskSet(set), DeclarationForm::SchedulabilityClaim(fp), DeclarationForm::SchedulabilityClaim(response)] =
        forms.as_slice()
    else {
        panic!("task set and two claims: {forms:?}");
    };
    assert_eq!(set.name.name, "Ctl");
    assert_eq!(set.tasks.len(), 2);
    assert_eq!(set.tasks[0].arrival, TaskArrivalKind::Periodic);
    assert_eq!(set.tasks[1].arrival, TaskArrivalKind::Sporadic);
    assert!(set.tasks[1].interval.negative);
    assert_eq!(set.tasks[1].criticality, TaskCriticality::Hi);
    assert!(set.tasks[1].wcet_hi.is_some());
    assert!(set.tasks[1].jitter.as_ref().unwrap().negative);
    assert_eq!(fp.task_set.name, "Ctl");
    assert!(
        matches!(&fp.kind, SchedulabilityClaimKind::Schedulable { policy } if policy == "fixed_priority")
    );
    assert!(
        matches!(&response.kind, SchedulabilityClaimKind::Response { task, .. } if task.name == "t2")
    );
}
