// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-563, FR-295: the JIT against the interpreter over generated units.

use ix_trace_rs::trace;
use qsl_conformance::{run_parity, FindingKind, ParityInputs, ParityLimits};
use qsl_jit::Jit;
use quire_eval::ExecutionBackendChoice;
use quire_exact::Cancel;

#[trace("TC-1386")]
#[test]
fn the_jit_has_no_difference_over_generated_units_and_only_known_coverage_gaps() {
    let report = run_parity(
        &[(ExecutionBackendChoice::Jit, &Jit, Some(qsl_jit::coverage()))],
        &ParityInputs {
            count: 300,
            ..ParityInputs::default()
        },
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("the gate runs");
    let backend = &report.backends[0];
    assert!(backend.compared > 0);
    // Every finding is a coverage gap: an operation the coverage lists whose
    // result does not fit 64 bits (FR-562) needs the kernel call through a
    // value handle, which this backend does not have yet. No difference,
    // nondeterminism, prepare failure or fault is allowed.
    let others: Vec<_> = backend
        .findings
        .iter()
        .filter(|f| f.kind != FindingKind::CoverageGap)
        .collect();
    assert!(others.is_empty(), "{:#?}", others.first());
}
