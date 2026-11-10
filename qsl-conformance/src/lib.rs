// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-564, FR-565 (ADR-029 CB-4): the lifecycle conformance matrix.
//!
//! [`lifecycle_matrix`] publishes a cell for each (verb, outcome kind) pair of
//! the verbs it holds: a [`MatrixCase`] with every input byte it needs where
//! QSL's library operation reaches that kind, a
//! [`MatrixCell::NotApplicable`] with FR-564's reason where it does not.
//! [`run_case`] runs a case through
//! the library and returns the typed outcome and its exit code. The driver
//! repository can run the same cases through its CLI and compare; this crate
//! holds the only copy of a case.

mod cases;
mod fault_backend;
mod matrix;
mod run;

pub use matrix::{
    AnalyzeInput, ArgumentValue, BackendPlan, Cancellation, CaseInput, EngineItem, ExecuteArgument,
    ExecuteInput, FrontEndInput, FrontEndLimit, Input, MatrixCase, MatrixCell, MonitorInput,
    OutcomeKind, PluginSettlement, PopulationInput, ProveInput, RequestParameters, SubjectInput,
    Verb, WorkingDirectory,
};
pub use run::{run_case, run_case_with, CaseError, CaseOutcome, CaseStage, Report};

pub use cases::lifecycle_matrix;
pub use fault_backend::FaultInjectingBackend;
