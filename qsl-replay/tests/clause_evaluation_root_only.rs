// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1274 (FR-491-AC-6): a consumer reaches `evaluate_clause` and every type
//! of its request and result through the `qsl_replay` root alone. This file
//! names nothing but `qsl_replay` root items, so it stops compiling if the
//! facade needs `qsl_replay::spine`, `qsl_semantics` or `quire_exact`.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_replay::{
    evaluate_clause, AnchorKind, ClauseEvaluation, ClauseEvaluationRequest, ClauseIncomplete,
    ClauseObservation, ClauseRefusal, ClauseRunStage, DependencyInput, ObservationLimits,
    ScalarLimits, SelectedAnchor, SourceIdentity, SpineLimits,
};

fn request(bytes: &[u8]) -> ClauseEvaluationRequest {
    ClauseEvaluationRequest {
        source: SourceIdentity::new("a", "u", "git", "1"),
        path: "unit.native".to_owned(),
        bytes: bytes.to_vec(),
        packages: Vec::new(),
        dependencies: DependencyInput::default(),
        clause: "C".to_owned(),
        observation: ClauseObservation::PreCall {
            snapshot: b"{}".to_vec(),
            self_object: None,
            parameters: BTreeMap::new(),
        },
        limits: SpineLimits::default(),
        observation_limits: ObservationLimits::default(),
        accounting: ScalarLimits {
            integer_bits: u64::MAX,
            decimal_digits: u64::MAX,
            scale_expansion: u64::MAX,
            text_input_bytes: u64::MAX,
            text_scalars: u64::MAX,
            normalized_scalars: u64::MAX,
            unit_edges: u64::MAX,
            value_occurrences: u64::MAX,
            work_units: 1,
            result_units: u64::MAX,
        },
    }
}

#[trace("TC-1274", "FR-491-AC-6")]
#[test]
fn the_entry_and_its_types_are_reachable_from_the_root_alone() {
    // An empty unit and an unreadable snapshot are refused with root types.
    assert!(matches!(
        evaluate_clause(request(b"")),
        ClauseEvaluation::Refused(ClauseRefusal::EmptySource)
    ));
    assert!(matches!(
        evaluate_clause(request(b"not a unit")),
        ClauseEvaluation::Refused(ClauseRefusal::UnreadableObservation)
    ));
    // The remaining result and observation types are nameable.
    let _: Option<ClauseIncomplete> = None;
    let _ = ClauseRunStage::Compile;
    let _ = ClauseObservation::Current {
        snapshot: Vec::new(),
        anchor: SelectedAnchor {
            kind: AnchorKind::Handler,
            name: String::new(),
        },
        self_object: None,
    };
}
