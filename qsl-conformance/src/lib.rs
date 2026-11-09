// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-563 (ADR-029 EB-5, FR-295): the engine parity runner.
//!
//! [`run_parity`] prepares each corpus or generated unit on the interpreter
//! (the reference) and on every supplied backend, calls each entry with each
//! argument tuple on both (twice on the backend), and reports every
//! difference. The runner has no option that allows, skips, retries or
//! tolerates a finding, and none that selects a reference.
//!
//! `run_parity_with_reference` exists only under the `test-support` feature.
//! Without it the name does not resolve:
#![cfg_attr(
    not(feature = "test-support"),
    doc = "\n```compile_fail\nuse qsl_conformance::run_parity_with_reference;\n```\n"
)]
//! and with it the name resolves (the control):
#![cfg_attr(
    feature = "test-support",
    doc = "\n```\nuse qsl_conformance::run_parity_with_reference;\n```\n"
)]
//!
//!
//! The corpus is a directory of `.native` units read in place; this runner
//! compares the zero-parameter functions of a corpus unit, since a corpus
//! unit carries no argument tuples of its own.

mod compare;
pub mod generator;

use std::path::PathBuf;

use qsl_eval::execution::{
    BackendCoverage, CallLimits, EntryKind, EntrySelection, Interpreter, PrepareLimits,
    PreparedEntries, SuppliedBackend,
};
use qsl_eval::value::QualifiedName;
use qsl_package::CheckedPackage;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_eval::{ExecutionBackendChoice, UnsupportedCause, UnsupportedEntry};
use quire_exact::{Cancel, ScalarLimits, Value};
use quire_semantic_value::supplied_stop::SuppliedLimits;

pub use compare::Member;
pub use generator::DEFAULT_SEED;

/// What a backend is run against.
#[derive(Clone, Debug)]
pub struct ParityInputs {
    /// The conformance corpus location, read in place, when given.
    pub corpus: Option<PathBuf>,
    /// The generator seed (default [`DEFAULT_SEED`]).
    pub seed: u64,
    /// The generated units to compare (default 10,000).
    pub count: u64,
}

impl Default for ParityInputs {
    fn default() -> Self {
        Self {
            corpus: None,
            seed: DEFAULT_SEED,
            count: 10_000,
        }
    }
}

/// How much a run reports and what each call is metered by.
#[derive(Clone, Copy, Debug)]
pub struct ParityLimits {
    /// The findings a report holds (default 100); the total is still counted.
    pub max_reported: usize,
    /// The prepare bounds, as `prepare` takes them.
    pub prepare: PrepareLimits,
    /// The accounting limits of every call.
    pub accounting: ScalarLimits,
}

impl Default for ParityLimits {
    fn default() -> Self {
        Self {
            max_reported: 100,
            prepare: PrepareLimits::default(),
            accounting: ScalarLimits {
                integer_bits: u64::MAX,
                decimal_digits: u64::MAX,
                scale_expansion: u64::MAX,
                text_input_bytes: u64::MAX,
                text_scalars: u64::MAX,
                normalized_scalars: u64::MAX,
                unit_edges: u64::MAX,
                value_occurrences: u64::MAX,
                work_units: 1_000_000,
                result_units: u64::MAX,
            },
        }
    }
}

/// Why the gate stopped without a verdict.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParityRefusal {
    /// No non-interpreter backend was supplied.
    EmptyScope,
    /// The corpus location cannot be read.
    UnreadableCorpus(PathBuf),
    /// The caller's handle was cancelled.
    Cancelled,
}

impl ParityRefusal {
    /// The exit code of a refusal (the convention of FR-353).
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        2
    }
}

/// What kind of finding failed the gate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FindingKind {
    /// The backend's result differs from the reference's in `Member`.
    Difference(Member),
    /// The backend's two calls with the same inputs differ.
    Nondeterministic,
    /// The backend listed an entry whose constructs all lie in its coverage.
    CoverageGap,
    /// The backend's `prepare` failed for a unit the reference prepared.
    PrepareFailed,
    /// No entry was compared for the backend.
    EmptyComparison,
    /// The reference returned `Fault` for an entry.
    ReferenceFault,
}

/// One finding with the FR-295 report fields.
#[derive(Clone, Debug)]
pub struct Finding {
    /// What failed.
    pub kind: FindingKind,
    /// The backend it concerns.
    pub backend: ExecutionBackendChoice,
    /// The generator seed.
    pub seed: u64,
    /// The unit's index, or `corpus:<file>` for a corpus unit.
    pub unit: String,
    /// The unit's source bytes.
    pub source: String,
    /// The entry.
    pub entry: String,
    /// The arguments, rendered.
    pub arguments: String,
    /// The reference result, rendered.
    pub reference: String,
    /// The backend result, rendered.
    pub result: String,
}

/// An entry a backend listed unsupported.
#[derive(Clone, Debug)]
pub struct UnsupportedRecord {
    /// The unit it belongs to.
    pub unit: String,
    /// The entry.
    pub entry: String,
    /// The backend's reason.
    pub reason: UnsupportedEntry,
}

/// One backend's result.
#[derive(Clone, Debug)]
pub struct BackendReport {
    /// The backend.
    pub backend: ExecutionBackendChoice,
    /// The calls compared with the reference.
    pub compared: u64,
    /// The entries it listed unsupported.
    pub unsupported: Vec<UnsupportedRecord>,
    /// The findings, up to `max_reported`.
    pub findings: Vec<Finding>,
    /// The total number of findings.
    pub total_findings: u64,
}

/// The gate's report.
#[derive(Clone, Debug)]
pub struct ParityReport {
    /// The generator seed.
    pub seed: u64,
    /// The units compared.
    pub units: u64,
    /// The generated units `check` refused and the generator replaced.
    pub discarded: u64,
    /// Per backend.
    pub backends: Vec<BackendReport>,
}

impl ParityReport {
    /// 0 when no backend has a finding, 1 otherwise.
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        u8::from(self.backends.iter().any(|b| b.total_findings > 0))
    }

    /// The total findings over all backends.
    #[must_use]
    pub fn total_findings(&self) -> u64 {
        self.backends.iter().map(|b| b.total_findings).sum()
    }
}

/// A backend entry for [`run_parity`].
pub type BackendEntry<'a> = (
    ExecutionBackendChoice,
    &'a dyn SuppliedBackend,
    Option<&'a BackendCoverage>,
);

/// Run the parity gate with the interpreter as the reference.
///
/// # Errors
/// [`ParityRefusal`] when no backend is supplied, the corpus cannot be read,
/// or the handle is cancelled.
pub fn run_parity(
    backends: &[BackendEntry<'_>],
    inputs: &ParityInputs,
    limits: &ParityLimits,
    cancel: &Cancel,
) -> Result<ParityReport, ParityRefusal> {
    run(&Interpreter, backends, inputs, limits, cancel)
}

/// [`run_parity`] with `reference` in place of the interpreter.
///
/// # Errors
/// As [`run_parity`].
#[cfg(feature = "test-support")]
pub fn run_parity_with_reference(
    reference: &dyn SuppliedBackend,
    backends: &[BackendEntry<'_>],
    inputs: &ParityInputs,
    limits: &ParityLimits,
    cancel: &Cancel,
) -> Result<ParityReport, ParityRefusal> {
    run(reference, backends, inputs, limits, cancel)
}

struct Unit {
    label: String,
    source: String,
    package: CheckedPackage,
    functions: Vec<(String, Vec<Vec<Value>>)>,
}

fn corpus_units(path: &std::path::Path) -> Result<Vec<Unit>, ParityRefusal> {
    let unreadable = || ParityRefusal::UnreadableCorpus(path.to_path_buf());
    let mut files: Vec<_> = std::fs::read_dir(path)
        .map_err(|_| unreadable())?
        .flatten()
        .map(|entry| entry.path())
        .filter(|p| p.extension().is_some_and(|e| e == "native"))
        .collect();
    files.sort();
    let mut units = Vec::new();
    for file in files {
        let source = std::fs::read_to_string(&file).map_err(|_| unreadable())?;
        // A corpus unit is the unit as written: it already holds its
        // language and profile declarations.
        let Some(package) = generator::compile_whole(&source) else {
            continue;
        };
        let functions = generator::zero_parameter_functions(&source)
            .into_iter()
            .map(|name| (name, vec![Vec::new()]))
            .collect();
        units.push(Unit {
            label: format!(
                "corpus:{}",
                file.file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
            ),
            source,
            package,
            functions,
        });
    }
    Ok(units)
}

fn run(
    reference: &dyn SuppliedBackend,
    backends: &[BackendEntry<'_>],
    inputs: &ParityInputs,
    limits: &ParityLimits,
    cancel: &Cancel,
) -> Result<ParityReport, ParityRefusal> {
    if backends.is_empty() {
        return Err(ParityRefusal::EmptyScope);
    }
    let mut units = match &inputs.corpus {
        Some(path) => corpus_units(path)?,
        None => Vec::new(),
    };
    let (generated, discarded) = generator::generate(inputs.seed, inputs.count);
    units.extend(generated.into_iter().map(|(unit, package)| Unit {
        label: unit.index.to_string(),
        source: unit.source,
        package,
        functions: unit.functions,
    }));
    let mut reports: Vec<BackendReport> = backends
        .iter()
        .map(|(backend, _, _)| BackendReport {
            backend: *backend,
            compared: 0,
            unsupported: Vec::new(),
            findings: Vec::new(),
            total_findings: 0,
        })
        .collect();
    let call_limits = CallLimits {
        accounting: limits.accounting,
        supplied: SuppliedLimits::default(),
    };
    for unit in &units {
        if cancel.cause().is_some() {
            return Err(ParityRefusal::Cancelled);
        }
        let selections: Vec<EntrySelection> = unit
            .functions
            .iter()
            .filter_map(|(name, _)| QualifiedName::unqualified(name).ok())
            .map(EntrySelection::function)
            .collect();
        let Ok(expected) =
            reference.prepare_entries(&unit.package, &selections, &limits.prepare, cancel)
        else {
            continue;
        };
        for (slot, (choice, backend, coverage)) in backends.iter().enumerate() {
            let report = &mut reports[slot];
            let mut record =
                |kind: FindingKind, entry: &str, args: String, want: String, got: String| {
                    report.total_findings += 1;
                    if report.findings.len() < limits.max_reported {
                        report.findings.push(Finding {
                            kind,
                            backend: *choice,
                            seed: inputs.seed,
                            unit: unit.label.clone(),
                            source: unit.source.clone(),
                            entry: entry.to_owned(),
                            arguments: args,
                            reference: want,
                            result: got,
                        });
                    }
                };
            let prepared = match backend.prepare_entries(
                &unit.package,
                &selections,
                &limits.prepare,
                cancel,
            ) {
                Ok(prepared) => prepared,
                Err(failure) => {
                    record(
                        FindingKind::PrepareFailed,
                        "",
                        String::new(),
                        String::new(),
                        format!("{failure:?}"),
                    );
                    continue;
                }
            };
            for (selection, reason) in prepared.unsupported() {
                let entry = selection.name.to_string();
                report.unsupported.push(UnsupportedRecord {
                    unit: unit.label.clone(),
                    entry: entry.clone(),
                    reason: reason.clone(),
                });
                if let (Some(coverage), UnsupportedCause::Untranslatable { .. }) =
                    (coverage, &reason.cause)
                {
                    if compare::within_coverage(&unit.package, &entry, coverage) {
                        record(
                            FindingKind::CoverageGap,
                            &entry,
                            String::new(),
                            String::new(),
                            format!("{reason:?}"),
                        );
                    }
                }
            }
            for (name, tuples) in &unit.functions {
                let Ok(qualified) = QualifiedName::unqualified(name) else {
                    continue;
                };
                if prepared
                    .unsupported()
                    .iter()
                    .any(|(s, _)| s.kind == EntryKind::Function && s.name == qualified)
                {
                    continue;
                }
                for arguments in tuples {
                    let call = |side: &dyn PreparedEntries| {
                        side.call(
                            &qualified,
                            arguments,
                            &ObjectEnvironment::default(),
                            &call_limits,
                            cancel,
                        )
                    };
                    let want = call(expected.as_ref());
                    let rendered_args = format!("{arguments:?}");
                    let want_text = format!("{want:?}");
                    if matches!(want, Err(quire_eval::CallFailure::Fault(_))) {
                        record(
                            FindingKind::ReferenceFault,
                            name,
                            rendered_args,
                            want_text,
                            String::new(),
                        );
                        continue;
                    }
                    let first = call(prepared.as_ref());
                    let second = call(prepared.as_ref());
                    let (first_text, second_text) = (format!("{first:?}"), format!("{second:?}"));
                    report.compared += 1;
                    if first_text != second_text {
                        record(
                            FindingKind::Nondeterministic,
                            name,
                            rendered_args,
                            first_text,
                            second_text,
                        );
                        continue;
                    }
                    if let Some(member) = compare::difference(&want, &first) {
                        record(
                            FindingKind::Difference(member),
                            name,
                            rendered_args,
                            want_text,
                            first_text,
                        );
                    }
                }
            }
        }
    }
    for report in &mut reports {
        if report.compared == 0 {
            report.total_findings += 1;
            report.findings.push(Finding {
                kind: FindingKind::EmptyComparison,
                backend: report.backend,
                seed: inputs.seed,
                unit: String::new(),
                source: String::new(),
                entry: String::new(),
                arguments: String::new(),
                reference: String::new(),
                result: String::new(),
            });
        }
    }
    Ok(ParityReport {
        seed: inputs.seed,
        units: units.len() as u64,
        discarded,
        backends: reports,
    })
}
