// SPDX-License-Identifier: AGPL-3.0-or-later
//! A driver outside the crate builds and writes the `quire-outcome/1`
//! document through `qsl_replay`'s root alone, with no serializer of its own.

use ix_trace_rs::trace;
use qsl_replay::{
    Category, Certification, ItemLabel, Operation, OutcomeDocument, OutcomeItem, OutcomeStage,
    ProofBasis, RequestIndex, TerminalRecord, TerminalValue,
};

/// FR-286-AC-3: a `prove` document built and written through the facade
/// carries its items in request order under the `quire-outcome/1` format.
#[trace("TC-770", "FR-286-AC-3")]
#[test]
fn a_driver_writes_a_prove_document_through_the_facade() {
    let items = [
        TerminalRecord::new(
            RequestIndex::new(0),
            TerminalValue::Proved {
                basis: ProofBasis::Checks { success_checks: 2 },
                certification: Certification::Certified,
            },
        ),
        TerminalRecord::new(RequestIndex::new(1), TerminalValue::Failed),
    ]
    .iter()
    .map(OutcomeItem::from_terminal)
    .collect();
    let document = OutcomeDocument::settled(Operation::Prove, Some(OutcomeStage::S8), items);
    let written: serde_json::Value =
        serde_json::from_slice(&document.to_bytes().expect("encodes")).expect("is JSON");
    assert_eq!(written["format"], "quire-outcome/1");
    assert_eq!(written["operation"], "prove");
    assert_eq!(written["last_stage"], "S8");
    assert_eq!(written["category"], "internal-failure");
    assert_eq!(written["items"][0]["result"], "proved");
    assert_eq!(written["items"][0]["certification"], "certified");
    assert_eq!(
        written["items"][0]["basis"],
        serde_json::json!({"type": "bounded-proof", "checks": 2})
    );
    assert_eq!(written["items"][1]["result"], "failed");
    assert_eq!(document.category(), Category::InternalFailure);
    assert_eq!(document.items()[1].category(), Category::InternalFailure);
    assert_ne!(ItemLabel::Proved, ItemLabel::Failed);
}
