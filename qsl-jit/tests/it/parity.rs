// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-563, FR-295: the JIT against the interpreter over generated units.

use qsl_conformance::{run_parity, ParityInputs, ParityLimits};
use qsl_jit::Jit;
use quire_eval::ExecutionBackendChoice;
use quire_exact::Cancel;

/// Smoke test, deliberately untagged (TC-1386 verifies the runner itself, in
/// `qsl-conformance`): the first 200 admitted units at the default seed show no
/// finding. It stops before unit 555, so it is not the gate.
#[test]
fn parity_smoke_over_the_first_200_generated_units() {
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

/// The gate at 300 units, which includes unit 555: its `f6` multiplies two
/// call results whose product exceeds 64 bits, which the handle path
/// (QSL-1825) computes through the kernel as the interpreter does. Untagged:
/// TC-1386 verifies the runner itself, in `qsl-conformance`.
#[test]
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
