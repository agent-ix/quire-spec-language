// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-563: `cargo xtask parity`, the engine parity gate. It links every
//! non-interpreter backend the QSL workspace has (the JIT) and calls
//! `qsl_conformance::run_parity` with the defaults. Exit 0 with no finding,
//! 1 with any finding, 2 on a refusal or an unknown option.

use std::path::PathBuf;

use qsl_conformance::{run_parity, ParityInputs, ParityLimits};
use quire_eval::ExecutionBackendChoice;
use quire_exact::Cancel;

use crate::Outcome;

/// The options `cargo xtask parity` takes. None allows, skips, retries or
/// tolerates a finding, and none selects a reference (FR-563-AC-8).
fn parse(operands: &[std::ffi::OsString]) -> Result<ParityInputs, String> {
    let mut inputs = ParityInputs::default();
    let mut iterator = operands.iter();
    while let Some(option) = iterator.next() {
        let option = option.to_string_lossy().into_owned();
        let mut value = |name: &str| {
            iterator
                .next()
                .map(|v| v.to_string_lossy().into_owned())
                .ok_or_else(|| format!("option {name} needs a value"))
        };
        match option.as_str() {
            "--seed" => {
                let text = value("--seed")?;
                let parsed = text
                    .strip_prefix("0x")
                    .map_or_else(|| text.parse(), |hex| u64::from_str_radix(hex, 16));
                inputs.seed =
                    parsed.map_err(|_| format!("--seed {text} is not a 64-bit integer"))?;
            }
            "--count" => {
                let text = value("--count")?;
                inputs.count = text
                    .parse()
                    .map_err(|_| format!("--count {text} is not an integer"))?;
            }
            "--corpus" => inputs.corpus = Some(PathBuf::from(value("--corpus")?)),
            other => return Err(format!("unknown option {other}")),
        }
    }
    Ok(inputs)
}

/// Run the gate over the JIT.
pub fn run(operands: &[std::ffi::OsString]) -> Outcome {
    let refusal = |message: String| Outcome {
        stdout: format!("parity: refused: {message}\n"),
        code: 2,
    };
    let inputs = match parse(operands) {
        Ok(inputs) => inputs,
        Err(message) => return refusal(message),
    };
    let jit = qsl_jit::Jit;
    let report = match run_parity(
        &[(ExecutionBackendChoice::Jit, &jit, Some(qsl_jit::coverage()))],
        &inputs,
        &ParityLimits::default(),
        &Cancel::new(),
    ) {
        Ok(report) => report,
        Err(error) => return refusal(format!("{error:?}")),
    };
    let mut text = format!(
        "parity: seed {:#x}, {} units, {} discarded\n",
        report.seed, report.units, report.discarded
    );
    for backend in &report.backends {
        text.push_str(&format!(
            "  {:?}: compared {}, unsupported {}, findings {}\n",
            backend.backend,
            backend.compared,
            backend.unsupported.len(),
            backend.total_findings
        ));
        for finding in &backend.findings {
            text.push_str(&format!(
                "    {:?} unit {} entry {} arguments {}\n      reference {}\n      result    {}\n",
                finding.kind,
                finding.unit,
                finding.entry,
                finding.arguments,
                finding.reference,
                finding.result
            ));
        }
    }
    Outcome {
        stdout: text,
        code: report.exit_code(),
    }
}
