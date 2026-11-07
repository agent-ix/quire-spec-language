// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-535: the bounded `native-run-result/2` boundary, through crate-root names only.

use ix_trace_rs::trace;
use qsl_replay::{
    read_native_run_result_bounded, write_clause_run_result, ClauseDisposition, ClauseRunReport,
    NativeRunLimits, NativeRunReadError, NativeRunResult, SettlementBasis,
};
use serde_json::{json, Value};

fn report() -> ClauseRunReport {
    ClauseRunReport {
        disposition: ClauseDisposition::MissingName {
            name: "Absent".to_owned(),
        },
        usage: Default::default(),
        basis: SettlementBasis::Unavailable,
        witness: None,
        consequent_source_events: Default::default(),
        finite_window: None,
        counterexample: None,
        decisive: None,
        authenticated_temporal_selection: None,
        refused_at: None,
        ledger: None,
    }
}

fn command_error() -> Value {
    json!({
        "format": "native-run-result/2", "stage": "profile",
        "code": "unknown_edition", "message": "unknown edition in program.native",
        "details": {"phase": "profile", "code": "unknown_edition", "path": "program.native"},
        "basis": "unavailable"
    })
}

fn read(bytes: &[u8]) -> Result<NativeRunResult, NativeRunReadError> {
    read_native_run_result_bounded(bytes, NativeRunLimits::UNBOUNDED)
}

fn bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

/// FR-535-AC-1: write and read back through the root, both variants.
#[test]
#[trace("TC-1340", "FR-535-AC-1")]
fn tc_1340_root_api_round_trips_both_variants() {
    let written = write_clause_run_result(&report()).unwrap();
    let NativeRunResult::Clause(clause) = read(&written).unwrap() else {
        panic!("expected the clause variant");
    };
    assert_eq!(clause.stage(), "select");
    assert_eq!(clause.category(), "refusal");
    assert_eq!(clause.code(), Some("missing_declaration"));
    assert_eq!(clause.basis(), "unavailable");

    let NativeRunResult::CommandError(error) = read(&bytes(&command_error())).unwrap() else {
        panic!("expected the command-error variant");
    };
    assert_eq!(error.stage(), "profile");
    assert_eq!(error.code(), "unknown_edition");
    assert_eq!(error.message(), "unknown edition in program.native");
}

/// FR-535-AC-2: wire refusals keep their owning typed refusal.
#[test]
#[trace("TC-1340", "FR-535-AC-2")]
fn tc_1340_root_api_refuses_invalid_documents() {
    let clause: Value =
        serde_json::from_slice(&write_clause_run_result(&report()).unwrap()).unwrap();
    let mut old = clause.clone();
    old["format"] = json!("native-run-result/1");
    assert!(matches!(
        read(&bytes(&old)),
        Err(NativeRunReadError::UnsupportedFormat(format)) if format == "native-run-result/1"
    ));
    let mut unknown = clause.clone();
    unknown["surprise"] = json!(1);
    assert!(matches!(
        read(&bytes(&unknown)),
        Err(NativeRunReadError::MalformedMember { .. } | NativeRunReadError::Malformed(_))
    ));
    let mut contradictory = clause;
    contradictory["basis"] = json!("decisive-witness");
    assert!(matches!(
        read(&bytes(&contradictory)),
        Err(NativeRunReadError::Invalid {
            member: "witness",
            ..
        })
    ));
    let mut malformed = command_error();
    malformed["details"] = json!({"phase": "profile"});
    assert!(read(&bytes(&malformed)).is_err());
}

/// FR-535-AC-3: both bounds, refused before a result exists.
#[test]
#[trace("TC-1340", "FR-535-AC-3")]
fn tc_1340_root_api_enforces_both_limits() {
    let written = write_clause_run_result(&report()).unwrap();
    let length = u64::try_from(written.len()).unwrap();
    let limits = |max_input_bytes, max_working_bytes| NativeRunLimits {
        max_input_bytes,
        max_working_bytes,
    };

    let over = read_native_run_result_bounded(&written, limits(length - 1, u64::MAX));
    assert!(matches!(
        over,
        Err(NativeRunReadError::Read(quire_canonical::ReadError::Limit(limit)))
            if limit.bound == length - 1 && limit.required == length
    ));

    // The smallest sufficient working provision, by bisection: cumulative
    // charging makes sufficiency monotonic in the bound.
    let (mut low, mut high) = (0_u64, 1_u64 << 20);
    while low < high {
        let middle = low + (high - low) / 2;
        if read_native_run_result_bounded(&written, limits(length, middle)).is_ok() {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    assert!(low > 0);
    let refused = read_native_run_result_bounded(&written, limits(length, low - 1));
    assert!(matches!(
        refused,
        Err(NativeRunReadError::WorkingBytes { bound, required })
            if bound == low - 1 && required > bound
    ));
    let exact = read_native_run_result_bounded(&written, limits(length, low)).unwrap();
    let NativeRunResult::Clause(clause) = exact else {
        panic!("expected the clause variant");
    };
    assert_eq!(clause.code(), Some("missing_declaration"));
}
