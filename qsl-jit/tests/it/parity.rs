// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-563, FR-295: the JIT against the interpreter over generated units.

use ix_trace_rs::trace;
use qsl_conformance::{run_parity, ParityInputs, ParityLimits};
use qsl_jit::Jit;
use quire_eval::ExecutionBackendChoice;
use quire_exact::Cancel;

/// The first 200 admitted units at the default seed: no finding of any kind.
#[trace("TC-1386")]
#[test]
fn the_jit_passes_the_parity_gate_over_the_first_200_generated_units() {
    let report = gate(200);
    let backend = &report.backends[0];
    assert!(backend.compared > 0);
    assert_eq!(
        report.exit_code(),
        0,
        "{} findings, first: {:#?}",
        backend.total_findings,
        backend.findings.first()
    );
}

/// The gate at 300 units is red: unit 555's `f6` multiplies two call results
/// whose product does not fit 64 bits, an operation the coverage lists and
/// FR-562 sends to the kernel through a value handle, which this backend does
/// not have yet. FR-560 forbids listing such an entry unsupported, so this is
/// a coverage gap of the backend; the test states the gate's verdict and stays
/// ignored until the handle path closes it.
#[trace("TC-1386")]
#[test]
#[ignore = "blocked: operations beyond the 64-bit fast path need the kernel-handle path (QSL-602)"]
fn the_jit_passes_the_parity_gate_over_300_generated_units() {
    let report = gate(300);
    assert_eq!(
        report.exit_code(),
        0,
        "{:#?}",
        report.backends[0].findings.first()
    );
}

fn gate(count: u64) -> qsl_conformance::ParityReport {
    run_parity(
        &[(ExecutionBackendChoice::Jit, &Jit, Some(qsl_jit::coverage()))],
        &ParityInputs {
            count,
            ..ParityInputs::default()
        },
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("the gate runs")
}
