// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL FR-246 (TC-701 step 1): each checked schedulability claim carries the
//! FR-290 kind `schedulability`, and FR-075's request writer routes it to EN-7,
//! the one backend whose manifest advertises that kind.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_analyze::schedulability::manifest;
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::Span;
use qsl_route::request::items_from_requirements;
use qsl_route::{CandidateOutcome, ProviderOrigin, Registry};
use qsl_semantics::check::{
    Capability, CheckedScheduleClaim, RequirementRecord, ScheduleClaim, SchedulingPolicy,
};
use quire_exact::{Origin, Role};

fn claim(name: &str, policy: SchedulingPolicy) -> CheckedScheduleClaim {
    CheckedScheduleClaim {
        name: name.to_owned(),
        task_set: "Ctl".to_owned(),
        claim: ScheduleClaim::Schedulable(policy),
        span: Span { start: 0, end: 1 },
    }
}

#[test]
#[trace("TC-701")]
fn en_7_registers_and_a_schedulability_claim_routes_to_it() {
    let mut registry = Registry::new();
    registry
        .register(
            manifest::descriptor(ProviderOrigin::Linked).expect("EN-7's manifest is admitted"),
        )
        .expect("EN-7 registers");
    let records: BTreeMap<OccurrenceKey, RequirementRecord> = [
        ("CtlFp", SchedulingPolicy::FixedPriority, 1_u8),
        ("CtlEdf", SchedulingPolicy::Edf, 2),
    ]
    .into_iter()
    .map(|(name, policy, fill)| {
        (
            OccurrenceKey::new(
                crate::fixture_ids::node_key([fill]),
                Origin::new(Role::new("claim"), 0),
            ),
            RequirementRecord::model_claim(
                claim(name, policy).requirements(),
                crate::fixture_ids::node_key([99]),
            ),
        )
    })
    .collect();
    let items = items_from_requirements(&records, &registry, None);
    assert_eq!(items.len(), 2);
    for item in &items {
        assert_eq!(item.kind(), Capability::Schedulability);
        let CandidateOutcome::Candidates(set) = item.candidates() else {
            panic!("a candidate set: {:?}", item.candidates());
        };
        let ids: Vec<_> = set.candidates().iter().map(|c| c.id().as_str()).collect();
        assert_eq!(ids, ["en-7"]);
    }
}
