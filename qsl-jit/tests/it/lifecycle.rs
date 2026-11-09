// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-294, FR-561, FR-568: prepare limits, cancellation, kernel statuses
//! passed up unchanged, and the confinement of `unsafe`.

use ix_trace_rs::trace;
use qsl_eval::execution::{ExecutionBackend, PrepareFailure, PrepareLimits};
use qsl_jit::Jit;
use quire_eval::CallFailure;
use quire_exact::{Cancel, CancelCause, Integer, Value};

use crate::common::{
    int, interpreted, jitted, jitted_call, package_of, prepare, select, UNLIMITED,
};

const SOURCE: &str = "\
 function narrow using v(a: Int[0, 9], b: Int[0, 9]): Int[0, 18] pure { a + b }\n\
 function id using v(a: Int[0, 9]): Integer pure { a + 0 }\n";

#[trace("TC-1385")]
#[test]
fn a_declared_result_bound_gives_the_interpreters_outcome_and_charges() {
    let package = package_of(SOURCE);
    let prepared = prepare(&package, &["narrow"]);
    for (a, b) in [(0, 0), (4, 5), (9, 9)] {
        let arguments = [int(a), int(b)];
        let want = interpreted(&package, "narrow", &arguments, UNLIMITED);
        let got = jitted(&prepared, "narrow", &arguments, UNLIMITED);
        assert_eq!(got.result, want.result, "{a} {b}");
        assert_eq!(got.charges, want.charges, "{a} {b} charges");
    }
}

#[trace("TC-1382")]
#[test]
fn a_call_with_a_cancelled_handle_returns_cancelled_requested() {
    let package = package_of(SOURCE);
    let prepared = prepare(&package, &["id"]);
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let result = qsl_eval::execution::PreparedEntries::call(
        &prepared,
        &crate::common::name("id"),
        &[int(1)],
        &Default::default(),
        &qsl_eval::execution::CallLimits {
            accounting: UNLIMITED,
            supplied: Default::default(),
        },
        &cancel,
    );
    assert!(matches!(
        result,
        Err(CallFailure::Cancelled(CancelCause::Requested))
    ));
    assert!(jitted_call(&prepared, "id", &[int(1)]).is_ok());
}

#[test]
fn prepare_stops_at_a_compile_work_unit_or_code_byte_limit() {
    let package = package_of(SOURCE);
    for limits in [
        PrepareLimits {
            compile_work_units: 0,
            ..PrepareLimits::default()
        },
        PrepareLimits {
            code_bytes: 0,
            ..PrepareLimits::default()
        },
    ] {
        let result = Jit.prepare(&package, &select(&["id"]), &limits, &Cancel::new());
        assert!(matches!(result, Err(PrepareFailure::Limit(_))));
    }
}

#[test]
fn prepare_with_a_cancelled_handle_is_cancelled() {
    let package = package_of(SOURCE);
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let result = Jit.prepare(
        &package,
        &select(&["id"]),
        &PrepareLimits::default(),
        &cancel,
    );
    assert!(matches!(
        result,
        Err(PrepareFailure::Cancelled(CancelCause::Requested))
    ));
}

/// FR-568: no `unsafe` outside `exec.rs`. `make arch-lint-unsafe` checks the
/// whole workspace; this is the in-crate check of the same rule.
#[trace("TC-1400")]
#[test]
fn unsafe_code_appears_only_in_the_exec_module() {
    let sources = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for entry in std::fs::read_dir(sources).expect("src exists") {
        let path = entry.expect("an entry").path();
        let text = std::fs::read_to_string(&path).expect("readable");
        let code: String = text
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        let uses_unsafe = code.contains("unsafe ") || code.contains("unsafe{");
        let allows = code.contains("allow(unsafe_code)");
        let is_exec = path.file_name().is_some_and(|name| name == "exec.rs");
        assert_eq!(uses_unsafe, is_exec, "{path:?}");
        assert!(!allows || is_exec || path.ends_with("lib.rs"), "{path:?}");
    }
}

const SIGNATURE_SOURCE: &str = " function id using v(a: Int[0, 9]): Integer pure { a + 0 }\n";

/// A test hook records a wrong signature: the call returns `Fault` without
/// entering the code.
#[trace("TC-1400", "FR-568-AC-3")]
#[test]
fn an_entry_with_a_wrongly_recorded_signature_returns_fault_without_being_called() {
    let package = package_of(SIGNATURE_SOURCE);
    let mut prepared = prepare(&package, &["id"]);
    prepared.record_signature_for_test(&crate::common::name("id"), 2, true);
    let result = jitted_call(&prepared, "id", &[int(1)]);
    let Err(CallFailure::Fault(fault)) = result else {
        panic!("expected a Fault");
    };
    assert_eq!(fault.invariant(), "jit-entry-signature-mismatch");
}

/// A test entry returns a status outside the closed status type: `Fault`
/// and no value.
#[trace("TC-1400", "FR-568-AC-3")]
#[test]
fn a_status_outside_the_closed_status_type_returns_fault_and_no_value() {
    let package = package_of(SIGNATURE_SOURCE);
    let mut prepared = prepare(&package, &["id"]);
    prepared.return_out_of_range_status_for_test(&crate::common::name("id"));
    let result = jitted_call(&prepared, "id", &[int(1)]);
    let Err(CallFailure::Fault(fault)) = result else {
        panic!("expected a Fault");
    };
    assert_eq!(fault.invariant(), "jit-entry-status-out-of-range");
}

/// The signature recorded at generation is read from the declared Cranelift
/// signature: one pointer parameter and a `u32` result.
#[trace("TC-1400")]
#[test]
fn a_generated_entry_records_the_entry_signature() {
    let package = package_of(SIGNATURE_SOURCE);
    let prepared = prepare(&package, &["id"]);
    assert_eq!(
        prepared.recorded_signature_for_test(&crate::common::name("id")),
        Some((1, true))
    );
}

const EDGE_SOURCE: &str = "\
 function narrow_mod using v(a: Int[0, 10], b: Int[0, 4000000000000000000]): Integer pure { (a mod 5) * b }\n\
 function fitting_mod using v(a: Int[0, 10], b: Int[0, 1000000000000000000]): Integer pure { (a mod 5) * b }\n\
 function unread using v(x: Int[0, 100000000000000000000]): Boolean pure { true }\n\
 function other using v(a: Int[0, 9]): Integer pure { a + 1 }\n";

/// The remainder is not monotone: `a mod 5` is 0 at both ends of `[0, 10]`
/// and 4 between them, so a product with `b` must not be judged to fit 64
/// bits from the ends. It takes the handle path and returns the interpreter's
/// value and charges where the product exceeds 64 bits and where it does not.
#[trace("TC-1385")]
#[test]
fn a_remainder_times_a_large_operand_is_not_judged_to_fit_from_the_interval_ends() {
    let package = package_of(EDGE_SOURCE);
    let prepared = prepare(&package, &["narrow_mod", "fitting_mod"]);
    assert!(crate::common::unsupported_names(&prepared).is_empty());
    for (function, b) in [
        ("fitting_mod", 1_000_000_000_000_000_000),
        ("narrow_mod", 4_000_000_000_000_000_000),
    ] {
        let arguments = [int(9), int(b)];
        let want = interpreted(&package, function, &arguments, UNLIMITED);
        let got = jitted(&prepared, function, &arguments, UNLIMITED);
        assert_eq!(got.result, want.result);
        assert_eq!(got.charges, want.charges);
    }
}

/// A parameter the body does not read, of a type wider than 64 bits, is
/// admitted and ignored exactly as the interpreter does.
#[test]
fn an_unread_wide_parameter_is_not_a_fault() {
    let package = package_of(EDGE_SOURCE);
    let prepared = prepare(&package, &["unread"]);
    assert!(crate::common::unsupported_names(&prepared).is_empty());
    let big = Value::Integer(Integer::from(1_i128 << 65));
    let want = interpreted(&package, "unread", std::slice::from_ref(&big), UNLIMITED);
    let got = jitted(&prepared, "unread", std::slice::from_ref(&big), UNLIMITED);
    assert_eq!(got.result, want.result);
    assert_eq!(got.charges, want.charges);
}

/// A function the caller did not select at `prepare` is a caller condition,
/// not a backend fault.
#[test]
fn a_function_not_selected_at_prepare_is_refused_not_faulted() {
    let package = package_of(EDGE_SOURCE);
    let prepared = prepare(&package, &["other"]);
    let result = jitted_call(&prepared, "unread", &[int(1)]);
    assert!(matches!(result, Err(CallFailure::Input(_))), "{result:?}");
}
