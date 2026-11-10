// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-564's `execute` tool-failure cell: a backend outside QSL's evaluator
//! that faults.

use qsl_eval::execution::{
    CallLimits, EntrySelection, ExecutionBackend, PrepareFailure, PrepareLimits, PreparedEntries,
};
use qsl_eval::value::QualifiedName;
use qsl_package::CheckedPackage;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::model::observation::AdmittedObservations;
use quire_eval::{CallFailure, Evaluation, UnsupportedEntry};
use quire_exact::{Cancel, ScalarLimits, Value};
use quire_semantic_value::catalog::InternalFault;

/// A test backend whose `prepare` succeeds and whose `call` returns
/// [`CallFailure::Fault`] (FR-564, FR-565). The driver links it into `quire`
/// as a supplied backend, behind its test-only feature, so the cell reaches
/// the CLI.
#[derive(Clone, Copy, Debug, Default)]
pub struct FaultInjectingBackend;

/// What [`FaultInjectingBackend`] prepared: nothing, and no unsupported
/// entry.
#[derive(Clone, Copy, Debug)]
pub struct FaultInjectingPrepared;

fn fault() -> CallFailure {
    CallFailure::Fault(InternalFault::new(
        "qsl-conformance",
        "the fault-injecting backend faults on every call",
    ))
}

impl ExecutionBackend for FaultInjectingBackend {
    type Prepared<'p> = FaultInjectingPrepared;

    fn prepare<'p>(
        &self,
        _package: &'p CheckedPackage,
        _entries: &[EntrySelection],
        _limits: &PrepareLimits,
        _cancel: &Cancel,
    ) -> Result<Self::Prepared<'p>, PrepareFailure> {
        Ok(FaultInjectingPrepared)
    }
}

impl PreparedEntries for FaultInjectingPrepared {
    fn unsupported(&self) -> &[(EntrySelection, UnsupportedEntry)] {
        &[]
    }

    fn call(
        &self,
        _entry: &QualifiedName,
        _arguments: &[Value],
        _environment: &ObjectEnvironment,
        _limits: &CallLimits,
        _cancel: &Cancel,
    ) -> Result<Evaluation, CallFailure> {
        Err(fault())
    }

    fn evaluate(
        &self,
        _clause: &QualifiedName,
        _input: &AdmittedObservations,
        _accounting: &ScalarLimits,
        _cancel: &Cancel,
    ) -> Result<Evaluation, CallFailure> {
        Err(fault())
    }
}
