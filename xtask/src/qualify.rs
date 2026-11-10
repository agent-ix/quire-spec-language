// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-567: the `qualify` gate's verdict covers the final path the driver
//! calls. The gate runs three parts in every run: the FR-350 `corpus`, the
//! FR-565 `lifecycle-matrix` and the FR-563 `parity` over the non-interpreter
//! backends the workspace links.

use std::ffi::OsString;
use std::path::PathBuf;

use qsl_conformance::{lifecycle_matrix, run_case, MatrixCell};
use serde::Serialize;

use crate::error::Result;
use crate::{qualification, Outcome};

/// How a part of the gate settled.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PartResult {
    /// The part passed.
    Passed,
    /// The part failed; its `failures` name what.
    Failed,
    /// `parity` only: the workspace links no non-interpreter backend, which
    /// passes.
    NoBackend,
}

impl PartResult {
    /// The result's report spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::NoBackend => "no-backend",
        }
    }
}

/// What one part of the gate settled. The report lists the parts as
/// `corpus`, `lifecycle-matrix`, `parity`.
#[derive(Debug, Serialize)]
pub struct PartReport {
    /// The part's name: `corpus`, `lifecycle-matrix` or `parity`.
    pub part: &'static str,
    /// How the part settled.
    pub result: PartResult,
    /// What failed, as `lifecycle-matrix/<verb>/<kind>` or `parity/<backend>`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub failures: Vec<String>,
}

impl PartReport {
    /// Whether the part lets the run be `qualified`: `no-backend` is a pass.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.result != PartResult::Failed
    }
}

/// The inputs of [`run`].
#[derive(Debug)]
pub struct QualifyInputs {
    /// The QSpec checkout the corpus schema is read from.
    pub qspec: PathBuf,
    /// The `qualify` command's operands.
    pub operands: Vec<OsString>,
    /// A replacement matrix cell set. A test may make the `lifecycle-matrix`
    /// part fail with it; the command line offers none.
    #[cfg(feature = "test-support")]
    pub matrix: Option<Vec<MatrixCell>>,
}

/// The `lifecycle-matrix` part: every `Case` gives a document or item whose
/// category and exit code are the case's expected ones.
fn lifecycle_matrix_part(cells: &[MatrixCell]) -> PartReport {
    let failures: Vec<String> = cells
        .iter()
        .filter_map(|cell| {
            let MatrixCell::Case(case) = cell else {
                return None;
            };
            let passes = run_case(case).is_ok_and(|outcome| {
                outcome.category() == case.expected_category
                    && outcome.exit_code() == case.expected_exit
            });
            (!passes).then(|| format!("lifecycle-matrix/{}/{}", case.verb.name(), case.kind.name()))
        })
        .collect();
    PartReport {
        part: "lifecycle-matrix",
        result: if failures.is_empty() {
            PartResult::Passed
        } else {
            PartResult::Failed
        },
        failures,
    }
}

/// The `parity` part (FR-563, FR-567).
///
/// Known gap: FR-563's runner (`run_parity`) is not in this repository yet.
/// It arrives from the jit lane's branch and moves into `qsl-conformance`
/// after this lands. Until then the part cannot run a backend, so it reports
/// `no-backend`, which is true only while the workspace links none (it does
/// not: `qsl-conformance` does not depend on `qsl-jit`). The code does not
/// compute that from the linked backends, so the part must be replaced, not
/// left in place, when a backend is linked. FR-567-AC-1's linked delegating
/// backend, AC-3's `parity/jit` and AC-4's `reference-fault` case have no
/// code path here and no test.
fn parity_part() -> PartReport {
    PartReport {
        part: "parity",
        result: PartResult::NoBackend,
        failures: Vec::new(),
    }
}

/// The parts after `corpus`, in report order.
pub(crate) fn lifecycle_and_parity_parts(replacement: Option<&[MatrixCell]>) -> Vec<PartReport> {
    vec![
        lifecycle_matrix_part(replacement.unwrap_or_else(|| lifecycle_matrix())),
        parity_part(),
    ]
}

/// Run the gate: the corpus against QSL, the lifecycle matrix and the parity
/// of the linked backends, with the verdict `qualified` only when all three
/// pass (FR-567). The exit contract is FR-353's.
///
/// # Errors
///
/// [`crate::error::Error::QualificationRefusal`] for a refusal before any
/// verdict.
pub fn run(inputs: &QualifyInputs) -> Result<Outcome> {
    #[cfg(feature = "test-support")]
    let replacement = inputs.matrix.as_deref();
    #[cfg(not(feature = "test-support"))]
    let replacement = None;
    qualification::execute_with_parts(&inputs.qspec, &inputs.operands, replacement)
        .map(|execution| execution.outcome)
}
