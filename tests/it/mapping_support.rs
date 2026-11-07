// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-527-AC-15 (TC-1322 row 15): the classification of each temporal clause
//! of the three packages of the ported IT-001 recipe R from the in-process
//! package has the same semantic facts as the classification from the package
//! read back with the reader, while each retains its own admission binding.

use ix_trace_rs::trace;
use qsl_package::checked_views::{
    mapping_support, temporal_subject, Classification, Closure, Support, Target, Unmatched,
};
use quire_contract_model::{CheckedNodeId, CheckedNodeKind, CheckedPackageV2, TemporalForm};
use quire_spec_language::checked_handoff::{evidence, write, Corpus};
use quire_spec_language::emit_checked_v2;

use super::checked_handoff::{admitted, request, tree, KINDS};

/// The temporal clauses of `package`, in graph order.
fn clauses(package: &CheckedPackageV2) -> Vec<CheckedNodeId> {
    package
        .graph()
        .nodes
        .iter()
        .zip(package.node_kinds())
        .filter(|(_, kind)| **kind == CheckedNodeKind::Temporal(TemporalForm::TemporalClause))
        .map(|(node, _)| node.node_id.clone())
        .collect()
}

/// Each clause of `package` classified under each closure.
fn classified(package: &CheckedPackageV2) -> Vec<(Closure, Classification)> {
    clauses(package)
        .iter()
        .flat_map(|id| {
            let view = temporal_subject(package, id).expect("the clause reads");
            [Closure::Complete, Closure::Open].map(|closure| {
                (
                    closure,
                    mapping_support(&view, closure).expect("the clause classifies"),
                )
            })
        })
        .collect()
}

#[trace("TC-1322", "FR-527-AC-15")]
#[test]
fn the_in_process_and_the_read_back_package_classify_alike() {
    let mut seen = 0;
    for kind in KINDS {
        let holder = tempfile::tempdir().expect("temporary directory");
        let out = holder.path().join("handoff");
        let handoff = request(kind, Corpus::None);
        write(&handoff, &out).expect("the handoff is written");
        let files = tree(&out);

        let mut held = evidence(&handoff).expect("evidence");
        held.insert_domain_package_document(
            "agent-ix/checked-handoff",
            files["models/0.json"].as_slice(),
        )
        .expect("the evidence holds the same document");
        let bytes = emit_checked_v2::emit(&handoff.emit).expect("the unit emits in process");
        let in_process = admitted(&bytes, &held);
        let read_back = admitted(&files["checked-package.json"], &held);

        let first = classified(&in_process);
        let second = classified(&read_back);
        assert_eq!(first.len(), second.len(), "{kind}");
        assert!(!first.is_empty(), "{kind}");
        seen += first.len() / 2;
        for ((left_closure, left), (right_closure, right)) in first.iter().zip(&second) {
            assert_eq!(left_closure, right_closure, "{kind}");
            assert_eq!(left.support, right.support, "{kind}");
            assert_eq!(left.retained, right.retained, "{kind}");
            assert_eq!(left.selection().package(), right.selection().package());
            assert_eq!(left.selection().clause(), right.selection().clause());
            assert_eq!(
                left.selection().definition_artifact(),
                right.selection().definition_artifact()
            );
            assert_ne!(
                left.selection().admission_binding(),
                right.selection().admission_binding()
            );
            assert!(left.belongs_to(&in_process));
            assert!(!left.belongs_to(&read_back));
            assert!(right.belongs_to(&read_back));
            assert!(!right.belongs_to(&in_process));
        }

        for (closure, result) in &first {
            let expected = match kind {
                "event_position" => Support::Supported {
                    target: match closure {
                        Closure::Complete => Target::ClosedTrace,
                        Closure::Open => Target::OnlinePrefix,
                    },
                    total_sample_valuation: false,
                    premises: qsl_package::checked_views::OUTSTANDING_PREMISES,
                },
                "fixed_sample" => Support::Supported {
                    target: match closure {
                        Closure::Complete => Target::ClosedTrace,
                        Closure::Open => Target::OnlinePrefix,
                    },
                    total_sample_valuation: true,
                    premises: qsl_package::checked_views::OUTSTANDING_PREMISES,
                },
                _ => Support::Unsupported {
                    dimensions: vec![Unmatched::FiniteWindow],
                },
            };
            assert_eq!(result.support, expected, "{kind} {closure:?}");
        }
    }
    // R's three clause-bearing packages hold `Due` and `DueLate`,
    // `DueSample` and `DueTimestamp`.
    assert_eq!(seen, 4);
}
