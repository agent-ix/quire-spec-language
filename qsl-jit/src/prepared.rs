// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-294, FR-560, FR-561: the [`Jit`] backend and what it prepared.

use std::collections::BTreeMap;

use qsl_eval::execution::{
    CallLimits, EntryKind, EntrySelection, ExecutionBackend, PrepareFailure, PrepareLimits,
    PrepareMeter, PreparedEntries,
};
use qsl_eval::value::{admit_arguments, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::model::observation::AdmittedObservations;
use quire_eval::{
    CallFailure, Evaluation, ExecutionBackendChoice, FamilyOutcome, UnsupportedCause,
    UnsupportedEntry,
};
use quire_exact::{
    Cancel, Charge, ChargePoint, Meter, Outcome, Refusal, ScalarLimits, Value, ValueType,
};
use quire_semantic_value::call::InputRefusal;
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::location::ExpressionLocation;
use quire_semantic_value::stop::{outcome_from_stop, Stop};
use quire_semantic_value::supplied_stop::SuppliedRun;

use crate::classify::first_outside;
use crate::coverage::coverage;
use crate::exec::{Ctx, Entry, Halt, Machine};
use crate::translate::{integer_to_i64, translate, Failure, Op};

/// The just-in-time backend (ADR-029 EB-1): it translates the entries inside
/// [`coverage`] and lists every other selected entry unsupported.
#[derive(Clone, Copy, Debug, Default)]
pub struct Jit;

/// One translated function.
struct Compiled {
    entry: Entry,
    ops: Vec<Op>,
    /// Per parameter: `Some(true)` for a `Boolean`, `Some(false)` for an
    /// `Integer` carried in a register, `None` for a parameter the function
    /// body does not read through a register.
    parameters: Vec<Option<bool>>,
    result_is_boolean: bool,
    /// Where an evaluation that stops before the body reports: the body root.
    site: ExpressionLocation,
}

/// What [`Jit`] prepared: the translated functions and the unsupported
/// entries, with the executable memory behind the former.
pub struct JitPrepared<'p> {
    package: &'p CheckedPackage,
    compiled: BTreeMap<QualifiedName, Compiled>,
    unsupported: Vec<(EntrySelection, UnsupportedEntry)>,
    // Declared last: it is dropped last, after nothing can call an entry.
    _machine: Machine,
}

fn fault(invariant: &'static str) -> PrepareFailure {
    PrepareFailure::Fault(InternalFault::new("FR-560", invariant))
}

impl ExecutionBackend for Jit {
    type Prepared<'p> = JitPrepared<'p>;

    fn prepare<'p>(
        &self,
        package: &'p CheckedPackage,
        entries: &[EntrySelection],
        limits: &PrepareLimits,
        cancel: &Cancel,
    ) -> Result<JitPrepared<'p>, PrepareFailure> {
        if let Some(cause) = cancel.cause() {
            return Err(PrepareFailure::Cancelled(cause));
        }
        let mut meter = PrepareMeter::new(limits, cancel);
        // The host refusing a code generator has no FR-294 cause yet
        // (`EnvironmentRefused` is not in `quire-eval`), so it is a fault.
        let (mut machine, shim) =
            Machine::new().map_err(|_| fault("jit-host-refused-code-generator"))?;
        let mut compiled = BTreeMap::new();
        let mut unsupported = Vec::new();
        let mut finalized = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for selection in entries {
            if !seen.insert((selection.kind == EntryKind::Clause, selection.name.clone())) {
                continue;
            }
            if selection.kind == EntryKind::Clause {
                // Clause entries need forall/exists, deref, `self` and the
                // FR-265 witness reproduction; none is translated yet, and
                // their body is not reachable through the function accessors.
                return Err(fault("jit-clause-entries-not-implemented"));
            }
            let graph = package.graph();
            let Some(callable) = graph.callable(&selection.name.to_string()) else {
                // The interpreter accepts any selection and refuses an
                // unknown name at the call; so does this backend.
                continue;
            };
            let Some(function) = graph.function_by_identity(callable.identity.clone()) else {
                return Err(fault("jit-callable-without-body"));
            };
            if let Some(outside) = first_outside(callable.parameters, function.body, coverage()) {
                unsupported.push((selection.clone(), untranslatable(outside)));
                continue;
            }
            let result = translate(
                machine.module(),
                &shim,
                &selection.name.to_string().replace("::", "_"),
                callable.parameters,
                function.body,
                &mut meter,
            );
            match result {
                Ok(translated) => {
                    finalized.push((selection.name.clone(), translated, callable.parameters));
                }
                Err(Failure::Unsupported(outside)) => {
                    unsupported.push((selection.clone(), untranslatable(outside)));
                }
                Err(Failure::Prepare(failure)) => return Err(failure),
            }
        }
        machine
            .finalize()
            .map_err(|_| fault("jit-finalize-definitions"))?;
        for (name, translated, parameters) in finalized {
            let graph = package.graph();
            let Some(callable) = graph.callable(&name.to_string()) else {
                return Err(fault("jit-callable-vanished"));
            };
            let Some(function) = graph.function_by_identity(callable.identity.clone()) else {
                return Err(fault("jit-callable-without-body"));
            };
            let entry = machine.entry(translated.id, parameters.len());
            compiled.insert(
                name,
                Compiled {
                    entry,
                    ops: translated.ops,
                    parameters: parameters
                        .iter()
                        .map(|(_, ty)| match ty {
                            ValueType::Boolean => Some(true),
                            ValueType::Int(_) => Some(false),
                            _ => None,
                        })
                        .collect(),
                    result_is_boolean: translated.result_is_boolean,
                    site: function.body.location().clone(),
                },
            );
        }
        Ok(JitPrepared {
            package,
            compiled,
            unsupported,
            _machine: machine,
        })
    }
}

fn untranslatable(outside: crate::classify::Outside) -> UnsupportedEntry {
    UnsupportedEntry {
        backend: ExecutionBackendChoice::Jit,
        cause: UnsupportedCause::Untranslatable {
            construct: outside.construct,
            location: outside.location,
        },
    }
}

/// The evaluation a stop gives, as the interpreter gives it.
fn stopped(stop: Stop, location: Option<ExpressionLocation>) -> Result<Evaluation, CallFailure> {
    if let Stop::Refused(Refusal::CheckedInvariant { cause }) = stop {
        return Err(CallFailure::Fault(InternalFault::from_kernel(
            "S6a",
            "checked-program-invariant",
            cause,
        )));
    }
    Ok(Evaluation {
        outcome: FamilyOutcome::Evaluated(outcome_from_stop(Err(stop))),
        location,
        losses: Vec::new(),
    })
}

impl PreparedEntries for JitPrepared<'_> {
    fn unsupported(&self) -> &[(EntrySelection, UnsupportedEntry)] {
        &self.unsupported
    }

    fn call(
        &self,
        function: &QualifiedName,
        arguments: &[Value],
        environment: &ObjectEnvironment,
        limits: &CallLimits,
        cancel: &Cancel,
    ) -> Result<Evaluation, CallFailure> {
        if let Some(cause) = cancel.cause() {
            return Err(CallFailure::Cancelled(cause));
        }
        let mut meter = Meter::new(limits.accounting).with_cancel(cancel.clone());
        let mut run =
            SuppliedRun::new(limits.supplied, cancel.clone()).map_err(CallFailure::Supplied)?;
        self.call_metered(
            function,
            arguments,
            environment,
            &mut meter,
            &mut run,
            cancel,
        )
    }

    fn evaluate(
        &self,
        _clause: &QualifiedName,
        _input: &AdmittedObservations,
        _accounting: &ScalarLimits,
        _cancel: &Cancel,
    ) -> Result<Evaluation, CallFailure> {
        // `prepare` refuses clause selections, so no clause is prepared.
        Err(CallFailure::Fault(InternalFault::new(
            "FR-294",
            "jit-clause-entry-not-prepared",
        )))
    }
}

impl JitPrepared<'_> {
    /// [`PreparedEntries::call`] under a caller-owned `meter` and supplied-value
    /// `run`, with the contract of `CheckedPackageEvaluation::call_supplied`:
    /// the meter then shows the charges of the call, which the parity gate
    /// compares with the interpreter's (FR-562).
    ///
    /// # Errors
    /// As [`PreparedEntries::call`].
    pub fn call_metered(
        &self,
        function: &QualifiedName,
        arguments: &[Value],
        environment: &ObjectEnvironment,
        meter: &mut Meter,
        run: &mut SuppliedRun,
        cancel: &Cancel,
    ) -> Result<Evaluation, CallFailure> {
        if let Some((_, reason)) = self.unsupported.iter().find(|(selection, _)| {
            selection.kind == EntryKind::Function && selection.name == *function
        }) {
            return Err(CallFailure::Unsupported(reason.clone()));
        }
        let graph = self.package.graph();
        let callable = graph
            .callable(&function.to_string())
            .ok_or_else(|| InputRefusal::UnknownFunction(function.to_string()))?;
        let Some(compiled) = self.compiled.get(function) else {
            return Err(CallFailure::Fault(InternalFault::new(
                "FR-294",
                "jit-entry-not-prepared",
            )));
        };
        admit_arguments(
            callable.parameters,
            arguments,
            environment,
            graph.scope().types(),
            run,
        )?;
        // FR-401: the scheduler charges `function.call` before the body.
        if let Err(denied) = meter.charge(Charge::new(ChargePoint::FunctionCall)) {
            if let Some(cause) = cancel.tripped() {
                return Err(CallFailure::Cancelled(cause));
            }
            return Ok(Evaluation {
                outcome: FamilyOutcome::Evaluated(Outcome::Incomplete(denied)),
                location: Some(compiled.site.clone()),
                losses: Vec::new(),
            });
        }
        let registers = registers(compiled, arguments)?;
        let mut ctx = Ctx::new(meter, &compiled.ops);
        let value = compiled.entry.call(&mut ctx, &registers).ok_or_else(|| {
            CallFailure::Fault(InternalFault::new("FR-560", "jit-argument-count"))
        })?;
        let halt = ctx.into_halt();
        if let Some(cause) = cancel.tripped() {
            return Err(CallFailure::Cancelled(cause));
        }
        match halt {
            None => Ok(Evaluation {
                outcome: FamilyOutcome::Evaluated(Outcome::Completed(
                    if compiled.result_is_boolean {
                        Value::Boolean(value != 0)
                    } else {
                        Value::Integer(value.into())
                    },
                )),
                location: None,
                losses: Vec::new(),
            }),
            Some(Halt::Stop(stop, location)) => stopped(stop, Some(location)),
            Some(Halt::Fault(invariant)) => {
                Err(CallFailure::Fault(InternalFault::new("FR-560", invariant)))
            }
        }
    }
}

/// The 64-bit register image of `arguments` for `compiled`.
fn registers(compiled: &Compiled, arguments: &[Value]) -> Result<Vec<i64>, CallFailure> {
    let broken = || CallFailure::Fault(InternalFault::new("FR-560", "jit-argument-kind"));
    compiled
        .parameters
        .iter()
        .zip(arguments)
        .map(|(kind, argument)| match (kind, argument) {
            (Some(true), Value::Boolean(value)) => Ok(i64::from(*value)),
            (Some(false), Value::Integer(value)) => integer_to_i64(value).ok_or_else(broken),
            (None, _) => Ok(0),
            _ => Err(broken()),
        })
        .collect()
}
