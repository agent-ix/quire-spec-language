// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-775: FR-291, the typed terminal records of plugin-run failures.

use ix_trace_rs::trace;
use qsl_foundation::BackendId;
use qsl_replay::{
    IncompleteCause, Operation, OutcomeDocument, OutcomeItem, OutcomeStage, RequestIndex,
    TerminalRecord, ToolFailure,
};
use quire_exact::CancelCause;
use quire_semantic_value::catalog::Category;
use serde_json::{json, Value};

fn write(items: Vec<OutcomeItem>) -> (OutcomeDocument, Value) {
    let document = OutcomeDocument::settled(Operation::Prove, Some(OutcomeStage::S8), items);
    let written = serde_json::from_slice(&document.to_bytes().expect("encodes")).expect("JSON");
    (document, written)
}

fn backend() -> BackendId {
    BackendId::new("acme")
}

fn item(record: &TerminalRecord) -> OutcomeItem {
    OutcomeItem::from_terminal(record)
}

/// FR-291-AC-1: each of the fourteen variants serializes as internal failure
/// with `tool-failure{defect}`, exits 30, and reads back to itself.
#[trace("TC-775", "FR-291-AC-1")]
#[test]
fn each_tool_failure_serializes_and_reads_back() {
    let cases = [
        (ToolFailure::Crash, "crash"),
        (ToolFailure::NonzeroExit, "nonzero-exit"),
        (ToolFailure::MalformedFrame, "malformed-frame"),
        (ToolFailure::NoncanonicalFrame, "noncanonical-frame"),
        (
            ToolFailure::FrameLength {
                limit: 1_048_576,
                value: 1_048_577,
            },
            "frame-length",
        ),
        (
            ToolFailure::TotalBytes {
                limit: 4_194_304,
                value: 4_194_305,
            },
            "total-bytes",
        ),
        (ToolFailure::UnknownProtocol, "unknown-protocol"),
        (ToolFailure::UnknownFrameKind, "unknown-frame-kind"),
        (ToolFailure::EnvelopeMember, "envelope-member"),
        (ToolFailure::UnknownRequestId, "unknown-request-id"),
        (ToolFailure::MalformedBody, "malformed-body"),
        (ToolFailure::UnroutedItem, "unrouted-item"),
        (ToolFailure::UnadvertisedKind, "unadvertised-kind"),
        (
            ToolFailure::MissingTerminalRecord,
            "missing-terminal-record",
        ),
    ];
    assert_eq!(cases.len(), 14);
    for (failure, defect) in cases {
        let record = TerminalRecord::tool_failed(RequestIndex::new(0), backend(), failure);
        let (document, written) = write(vec![item(&record)]);
        let entry = &written["items"][0];
        assert_eq!(entry["result"], "failed");
        assert_eq!(entry["category"], "internal-failure");
        let mut expected = json!({"kind": "tool-failure", "defect": defect});
        if let Some((limit, value)) = failure.byte_counts() {
            expected["limit"] = json!(limit);
            expected["value"] = json!(value);
        }
        assert_eq!(entry["cause"], expected);
        assert_eq!(document.category(), Category::InternalFailure);
        assert_eq!(document.category().exit_code(), 30);
        assert_eq!(ToolFailure::read_cause(&entry["cause"]), Some(failure));
    }
    assert_eq!(
        ToolFailure::read_cause(&json!({"kind": "tool-failure", "defect": "crash", "limit": 1})),
        None
    );
    assert_eq!(
        ToolFailure::read_cause(&json!({"kind": "tool-failure", "defect": "frame-length"})),
        None
    );
}

/// FR-291-AC-2: `TimedOut` and cancelled are category incomplete, exit 22.
#[trace("TC-775", "FR-291-AC-2")]
#[test]
fn timed_out_and_cancelled_are_incomplete() {
    for cause in [
        IncompleteCause::TimedOut,
        IncompleteCause::Cancelled {
            source: CancelCause::Requested,
        },
    ] {
        let record = TerminalRecord::incomplete(RequestIndex::new(0), backend(), cause);
        let (document, written) = write(vec![item(&record)]);
        assert_eq!(written["items"][0]["result"], "incomplete");
        assert_eq!(written["items"][0]["category"], "incomplete");
        assert_eq!(document.category().exit_code(), 22);
    }
}

/// FR-291-AC-3: a plugin item's failure leaves a sibling record unchanged.
#[trace("TC-775", "FR-291-AC-3")]
#[test]
fn a_plugin_failure_leaves_sibling_records_unchanged() {
    let proved =
        |index| TerminalRecord::proved(RequestIndex::new(index), BackendId::new("compile-time"), 2);
    let failed = TerminalRecord::tool_failed(RequestIndex::new(0), backend(), ToolFailure::Crash);
    let (document, mixed) = write(vec![item(&failed), item(&proved(1))]);
    let (_, alone) = write(vec![item(&proved(1))]);
    assert_eq!(
        serde_json::to_vec(&mixed["items"][1]).expect("bytes"),
        serde_json::to_vec(&alone["items"][0]).expect("bytes"),
    );
    assert_eq!(document.category().exit_code(), 30);
}
