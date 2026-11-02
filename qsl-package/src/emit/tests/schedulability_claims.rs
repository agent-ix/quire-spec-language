// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-322 / FR-482 schedulability claim body and carrier emission.

use super::*;

#[test]
fn emits_admitted_claim_body_and_occurrence_carrier() {
    let (_, package) = package_from_text(
        "profile t = \"quire.temporal.complete/v1\";\n\
         dimension Time;\n\
         unit ms : Time = rational(1, 1);\n\
         taskset S { task a periodic 4 ms deadline 4 ms bcet 1 ms wcet 1 ms priority 1 lo; }\n\
         schedulable C on S under fixed_priority;\n\
         schedulable D on S under fixed_priority;\n\
         response R on S.a <= 4 ms under fixed_priority;\n",
    );
    let emission = emit_checked(&package).expect("the task claim emits");
    let bytes = emission.package().bytes();
    let admitted = match CheckedPackageV2::read(
        bytes,
        quire_contract_model::CheckedPackageReadLimits::bounded(),
        emission.evidence(),
    ) {
        quire_contract_model::CheckedPackageV2ReadResult::Admitted(package) => package,
        result => panic!("the emitted task claim is admitted: {result:?}"),
    };
    let claims = admitted.schedulability_claims().collect::<Vec<_>>();
    assert_eq!(claims.len(), 3);
    let schedulable = claims
        .iter()
        .filter(|claim| {
            claim.kind() == quire_contract_model::CheckedSchedulabilityKind::Schedulable
        })
        .collect::<Vec<_>>();
    assert_eq!(schedulable.len(), 2);
    assert!(schedulable.iter().all(
        |claim| claim.policy() == quire_contract_model::CheckedSchedulingPolicy::FixedPriority
    ));
    assert_eq!(schedulable[0].declaration(), schedulable[1].declaration());
    assert_ne!(
        schedulable[0].occurrence().ordinal,
        schedulable[1].occurrence().ordinal
    );
    for claim in schedulable {
        assert_eq!(claim.tasks().len(), 1);
        let task = claim.tasks().next().expect("one task row");
        assert_eq!(task.name(), "a");
        assert_eq!(task.period().type_id, task.deadline().type_id);
        assert_eq!(task.wcet().value, task.bcet().value);
    }
    let response = claims
        .iter()
        .find(|claim| claim.kind() == quire_contract_model::CheckedSchedulabilityKind::Response)
        .expect("response claim is emitted too");
    assert_eq!(response.response_task(), Some("a"));
    assert_eq!(admitted.schedulability_claim_rows().unwrap().len(), 3);
}
