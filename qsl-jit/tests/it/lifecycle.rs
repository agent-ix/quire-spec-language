// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-294, FR-561, FR-568: prepare limits, cancellation, kernel statuses
//! passed up unchanged, and the confinement of `unsafe`.

use ix_trace_rs::trace;
use qsl_eval::execution::{ExecutionBackend, PrepareFailure, PrepareLimits};
use qsl_jit::Jit;
use quire_eval::CallFailure;
use quire_exact::{Cancel, CancelCause};

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

/// FR-568: no `unsafe` outside `exec.rs`. The workspace gate named by the
/// spec (`make arch-lint-unsafe`) does not exist yet; this is the in-crate
/// check of the same rule.
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
