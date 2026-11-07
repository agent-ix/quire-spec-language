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

/// A genuine decisive-witness `/2` record, produced by the FR-267 writer.
const DECISIVE: &str = include_str!("../fixtures/native-run-result-decisive.json");

fn refusal(value: &Value) -> NativeRunReadError {
    match read(&bytes(value)) {
        Ok(_) => panic!("accepted an invalid document: {value}"),
        Err(error) => error,
    }
}

/// FR-535-AC-2: the QSpec TC-255 NR-05/06/07 mutations over a decisive-witness
/// record keep their FR-267 refusals; no fallback or normalized success.
#[test]
#[trace("TC-1340", "FR-535-AC-2")]
fn tc_1340_root_api_refuses_invalid_documents() {
    let valid: Value = serde_json::from_str(DECISIVE).unwrap();
    assert!(matches!(
        read(&bytes(&valid)),
        Ok(NativeRunResult::Clause(_))
    ));

    // NR-05: identifier changed or removed.
    for format in ["native-run-result/1", "native-run-result/3"] {
        let mut value = valid.clone();
        value["format"] = json!(format);
        assert!(
            matches!(refusal(&value), NativeRunReadError::UnsupportedFormat(found) if found == format)
        );
    }
    let mut value = valid.clone();
    value.as_object_mut().unwrap().remove("format");
    assert!(
        matches!(refusal(&value), NativeRunReadError::UnsupportedFormat(found) if found == "<missing>")
    );

    // NR-06: unknown basis, and basis/witness contradictions.
    let mut value = valid.clone();
    value["basis"] = json!("maybe");
    assert!(
        matches!(refusal(&value), NativeRunReadError::MalformedMember { member, .. } if member == "basis")
    );
    let mut value = valid.clone();
    value.as_object_mut().unwrap().remove("basis");
    assert!(
        matches!(refusal(&value), NativeRunReadError::MalformedMember { member, .. } if member == ".")
    );
    let mut value = valid.clone();
    value["basis"] = json!("closed-scope");
    assert!(matches!(
        refusal(&value),
        NativeRunReadError::Invalid {
            member: "witness",
            ..
        }
    ));
    let mut value = valid.clone();
    value.as_object_mut().unwrap().remove("witness");
    assert!(matches!(
        refusal(&value),
        NativeRunReadError::Invalid {
            member: "witness",
            ..
        }
    ));

    // NR-07: unknown top-level and disposition members, removed required member.
    let mut value = valid.clone();
    value["surprise"] = json!(1);
    assert!(
        matches!(refusal(&value), NativeRunReadError::MalformedMember { member, .. } if member == "surprise")
    );
    let mut value = valid.clone();
    value["disposition"]["surprise"] = json!(1);
    assert!(
        matches!(refusal(&value), NativeRunReadError::MalformedMember { member, .. } if member == "disposition.surprise")
    );
    let mut value = valid.clone();
    value["disposition"]
        .as_object_mut()
        .unwrap()
        .remove("stage");
    assert!(
        matches!(refusal(&value), NativeRunReadError::MalformedMember { member, .. } if member == "disposition")
    );

    // The malformed command-error shape.
    let mut malformed = command_error();
    malformed["details"] = json!({"phase": "profile"});
    assert!(matches!(
        refusal(&malformed),
        NativeRunReadError::Invalid {
            member: "details",
            ..
        }
    ));
}

/// FR-535-AC-4: the facade adds no tool-version, digest, receipt or token. The
/// limits are exactly the two bounds, and a written document keeps only the
/// FR-267 member set.
#[test]
#[trace("TC-1340", "FR-535-AC-4")]
fn tc_1340_root_api_adds_no_authentication_claim() {
    let NativeRunLimits {
        max_input_bytes: _,
        max_working_bytes: _,
    } = NativeRunLimits::UNBOUNDED;
    let written: Value =
        serde_json::from_slice(&write_clause_run_result(&report()).unwrap()).unwrap();
    let mut members: Vec<&str> = written
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    members.sort_unstable();
    assert_eq!(members, ["basis", "disposition", "format", "usage"]);
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
