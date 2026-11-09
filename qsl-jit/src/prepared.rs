// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-294, FR-560, FR-561, FR-568: the [`Jit`] backend and [`PreparedJit`],
//! the safe wrapper that upholds the contract of the `exec` module.

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
use quire_exact::{Cancel, Charge, ChargePoint, Meter, Outcome, Refusal, ScalarLimits, Value};
use quire_semantic_value::call::InputRefusal;
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::location::ExpressionLocation;
use quire_semantic_value::stop::{outcome_from_stop, Stop};
use quire_semantic_value::supplied_stop::SuppliedRun;

use crate::coverage::coverage;
use crate::exec::{Entry, EntryFrame, EntryRefusal, Halt, Machine, Status};
use crate::kernel::State;
use crate::translate::{carried, integer_to_i64, Carried, Compiler, Failure, Op};
use qsl_eval::execution::{first_outside, Outside};

/// The just-in-time backend (ADR-029 EB-1): it translates the entries inside
/// [`coverage`] and lists every other selected entry unsupported.
#[derive(Clone, Copy, Debug, Default)]
pub struct Jit;

/// One translated function.
struct Compiled {
    entry: Entry,
    /// Per parameter: how it is carried, or `None` for a parameter the
    /// function body does not read through a register or handle.
    parameters: Vec<Option<Carried>>,
    result: Carried,
    /// Where an evaluation that stops before the body reports: the body root.
    site: ExpressionLocation,
}

/// What [`Jit`] prepared: the translated functions and the unsupported
/// entries, with the executable memory behind the former.
///
/// A callable entry borrows the value that holds its code:
///
/// ```compile_fail,E0505
/// use qsl_eval::value::QualifiedName;
/// use qsl_jit::PreparedJit;
/// fn held(prepared: PreparedJit<'_>, name: &QualifiedName) {
///     let entry = prepared.entry(name).expect("an entry");
///     drop(prepared);
///     let _ = entry.name();
/// }
/// ```
///
/// The control uses the entry within the borrow:
///
/// ```
/// use qsl_eval::value::QualifiedName;
/// use qsl_jit::PreparedJit;
/// fn within(prepared: PreparedJit<'_>, name: &QualifiedName) {
///     let entry = prepared.entry(name).expect("an entry");
///     let _ = entry.name();
///     drop(prepared);
/// }
/// ```
///
/// The frame type is private to `exec`:
///
/// ```compile_fail
/// use qsl_jit::exec::EntryFrame;
/// ```
pub struct PreparedJit<'p> {
    package: &'p CheckedPackage,
    compiled: BTreeMap<QualifiedName, Compiled>,
    ops: Vec<Op>,
    unsupported: Vec<(EntrySelection, UnsupportedEntry)>,
    /// The most handles the arena held during the last call.
    #[cfg(feature = "test-support")]
    arena_peak: std::cell::Cell<usize>,
    // Declared last: it is dropped last, after nothing can call an entry.
    _machine: Machine,
}

/// A callable entry of a [`PreparedJit`], valid while the value is borrowed.
pub struct CallableEntry<'a> {
    prepared: &'a PreparedJit<'a>,
    name: &'a QualifiedName,
}

impl<'a> CallableEntry<'a> {
    /// The entry's function name.
    #[must_use]
    pub fn name(&self) -> &QualifiedName {
        self.name
    }

    /// Call the entry as [`PreparedJit::call_metered`] does.
    ///
    /// # Errors
    /// As [`PreparedEntries::call`].
    pub fn call(
        &self,
        arguments: &[Value],
        environment: &ObjectEnvironment,
        meter: &mut Meter,
        run: &mut SuppliedRun,
        cancel: &Cancel,
    ) -> Result<Evaluation, CallFailure> {
        self.prepared
            .call_metered(self.name, arguments, environment, meter, run, cancel)
    }
}

fn fault(invariant: &'static str) -> PrepareFailure {
    PrepareFailure::Fault(InternalFault::new("FR-560", invariant))
}

impl ExecutionBackend for Jit {
    type Prepared<'p> = PreparedJit<'p>;

    fn prepare<'p>(
        &self,
        package: &'p CheckedPackage,
        entries: &[EntrySelection],
        limits: &PrepareLimits,
        cancel: &Cancel,
    ) -> Result<PreparedJit<'p>, PrepareFailure> {
        if let Some(cause) = cancel.cause() {
            return Err(PrepareFailure::Cancelled(cause));
        }
        let mut meter = PrepareMeter::new(limits, cancel);
        // The host refusing a code generator has no FR-294 cause yet
        // (`EnvironmentRefused` is not in `quire-eval`), so it is a fault.
        let (mut machine, shim) =
            Machine::new().map_err(|_| fault("jit-host-refused-code-generator"))?;
        let mut ops = Vec::new();
        let mut unsupported = Vec::new();
        let mut translated = Vec::new();
        let graph = package.graph();
        {
            let mut compiler = Compiler::new(
                machine.module(),
                &shim,
                graph,
                coverage(),
                &mut ops,
                &mut meter,
            );
            let mut seen = std::collections::BTreeSet::new();
            for selection in entries {
                if !seen.insert((selection.kind == EntryKind::Clause, selection.name.clone())) {
                    continue;
                }
                let name = selection.name.to_string();
                if selection.kind == EntryKind::Clause {
                    // The interpreter refuses an unknown clause at the call;
                    // so does this backend.
                    let Some(clause) = graph.state_clause(&name) else {
                        continue;
                    };
                    let outside = first_outside(clause.parameters(), clause.body(), coverage())
                        .unwrap_or_else(|| clause_outside(clause.body()));
                    unsupported.push((selection.clone(), untranslatable(outside)));
                    continue;
                }
                let Some(callable) = graph.callable(&name) else {
                    continue;
                };
                let Some(index) = compiler.index_of(&callable.identity) else {
                    return Err(fault("jit-callable-without-index"));
                };
                match compiler.entry(index) {
                    Ok(entry) => translated.push((selection.name.clone(), index, entry)),
                    Err(Failure::Unsupported(outside)) => {
                        unsupported.push((selection.clone(), untranslatable(outside)));
                    }
                    Err(Failure::Prepare(failure)) => return Err(failure),
                }
            }
        }
        machine
            .finalize()
            .map_err(|_| fault("jit-finalize-definitions"))?;
        let mut compiled = BTreeMap::new();
        for (name, index, entry) in translated {
            let (Some(function), Some(callable)) = (
                graph.function_state(index),
                graph.callable(&name.to_string()),
            ) else {
                return Err(fault("jit-callable-vanished"));
            };
            compiled.insert(
                name,
                Compiled {
                    entry: machine.entry(entry.id, callable.parameters.len(), entry.signature),
                    parameters: callable
                        .parameters
                        .iter()
                        .map(|(_, ty)| carried(ty))
                        .collect(),
                    result: entry.result,
                    site: function.body.location().clone(),
                },
            );
        }
        Ok(PreparedJit {
            package,
            compiled,
            ops,
            unsupported,
            #[cfg(feature = "test-support")]
            arena_peak: std::cell::Cell::new(0),
            _machine: machine,
        })
    }
}

/// A state clause none of whose constructs lies outside the coverage still
/// needs the FR-265 trail and receiver path, which this backend does not
/// reproduce, so it is listed at the clause's body as its `self` (FR-560:
/// the v1 coverage lists no `ProtocolClause`-family construct). Spec gap: the
/// location of that cause is not stated.
fn clause_outside(body: qsl_semantics::check::CheckedNode<'_>) -> Outside {
    Outside {
        construct: quire_eval::UntranslatableConstruct::ExpressionForm("self".to_owned()),
        location: body.location().clone(),
    }
}

fn untranslatable(outside: Outside) -> UnsupportedEntry {
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

impl PreparedEntries for PreparedJit<'_> {
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
        clause: &QualifiedName,
        _input: &AdmittedObservations,
        _accounting: &ScalarLimits,
        cancel: &Cancel,
    ) -> Result<Evaluation, CallFailure> {
        if let Some(cause) = cancel.cause() {
            return Err(CallFailure::Cancelled(cause));
        }
        if let Some((_, reason)) = self
            .unsupported
            .iter()
            .find(|(selection, _)| selection.kind == EntryKind::Clause && selection.name == *clause)
        {
            return Err(CallFailure::Unsupported(reason.clone()));
        }
        // Every selected clause is listed unsupported; an unselected or
        // unknown clause is the interpreter's refusal.
        Err(InputRefusal::UnknownClause(clause.to_string()).into())
    }
}

impl<'p> PreparedJit<'p> {
    /// The callable entry of the translated function `name`, borrowed from
    /// this value: the code it enters is freed when the value drops.
    #[must_use]
    pub fn entry<'a>(&'a self, name: &'a QualifiedName) -> Option<CallableEntry<'a>>
    where
        'p: 'a,
    {
        // SAFETY-free: the borrow of `self` ties the entry to the memory.
        self.compiled.contains_key(name).then(|| CallableEntry {
            prepared: self.shrink(),
            name,
        })
    }

    fn shrink<'a>(&'a self) -> &'a PreparedJit<'a>
    where
        'p: 'a,
    {
        self
    }

    /// Record `parameters` and `returns_u32` as the signature of the entry
    /// `name` in place of the real one (a test hook for FR-568-AC-3): a call
    /// on it then returns `Fault` without entering the code.
    #[cfg(feature = "test-support")]
    pub fn record_signature_for_test(
        &mut self,
        name: &QualifiedName,
        parameters: u8,
        returns_u32: bool,
    ) {
        if let Some(compiled) = self.compiled.get_mut(name) {
            compiled.entry = compiled
                .entry
                .with_recorded_signature(crate::exec::Signature::for_test(parameters, returns_u32));
        }
    }

    /// Replace the entry `name` with one returning a status outside the
    /// closed status type (a test hook for FR-568-AC-3).
    #[cfg(feature = "test-support")]
    pub fn return_out_of_range_status_for_test(&mut self, name: &QualifiedName) {
        if let Some(compiled) = self.compiled.get_mut(name) {
            compiled.entry = compiled.entry.returning_out_of_range_status();
        }
    }

    /// The signature recorded at generation for the entry `name`, as its
    /// parameter count and whether it returns a `u32`.
    #[cfg(feature = "test-support")]
    #[must_use]
    pub fn recorded_signature_for_test(&self, name: &QualifiedName) -> Option<(u8, bool)> {
        self.compiled
            .get(name)
            .map(|compiled| compiled.entry.recorded().shape())
    }

    /// The most handles the arena held during the last call (a test hook for
    /// the arena bound).
    #[cfg(feature = "test-support")]
    #[must_use]
    pub fn arena_peak_for_test(&self) -> usize {
        self.arena_peak.get()
    }

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
        // A function the caller did not select at `prepare` has no generated
        // code, and this backend never runs what it did not translate
        // (FR-296): it is a caller condition, refused as an unknown entry of
        // this prepared value, not a backend fault.
        let Some(compiled) = self.compiled.get(function) else {
            return Err(InputRefusal::UnknownFunction(function.to_string()).into());
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
        let mut state = State::bounded(Vec::new(), self.ops.len().saturating_add(arguments.len()));
        let registers = registers(compiled, arguments, &mut state.arena)?;
        let mut frame = EntryFrame::new(meter, &self.ops, graph.scope(), state);
        let status = compiled.entry.call(&mut frame, &registers);
        let value = frame.result();
        let (halt, state) = frame.into_parts();
        #[cfg(feature = "test-support")]
        self.arena_peak.set(state.peak);
        let status = status.map_err(|refusal| {
            CallFailure::Fault(InternalFault::new(
                "FR-568",
                match refusal {
                    EntryRefusal::Signature => "jit-entry-signature-mismatch",
                    EntryRefusal::Status => "jit-entry-status-out-of-range",
                    EntryRefusal::Arity => "jit-argument-count",
                },
            ))
        })?;
        if let Some(cause) = cancel.tripped() {
            return Err(CallFailure::Cancelled(cause));
        }
        match (status, halt) {
            (Status::Completed, None) => {
                let result = match compiled.result {
                    Carried::Boolean => Value::Boolean(value != 0),
                    Carried::Register => Value::Integer(value.into()),
                    Carried::Handle => usize::try_from(value)
                        .ok()
                        .and_then(|index| state.arena.get(index))
                        .cloned()
                        .ok_or_else(|| {
                            CallFailure::Fault(InternalFault::new("FR-560", "jit-result-handle"))
                        })?,
                };
                Ok(Evaluation {
                    outcome: FamilyOutcome::Evaluated(Outcome::Completed(result)),
                    location: None,
                    losses: state.losses,
                })
            }
            (Status::Stopped, Some(Halt::Stop(stop, location))) => stopped(stop, Some(location)),
            (_, Some(Halt::Fault(invariant))) => {
                Err(CallFailure::Fault(InternalFault::new("FR-560", invariant)))
            }
            _ => Err(CallFailure::Fault(InternalFault::new(
                "FR-568",
                "jit-status-halt-disagree",
            ))),
        }
    }
}

/// The 64-bit register image of `arguments` for `compiled`: a handle
/// parameter's value is placed in `arena` and passed as its index.
fn registers(
    compiled: &Compiled,
    arguments: &[Value],
    arena: &mut Vec<Value>,
) -> Result<Vec<i64>, CallFailure> {
    let broken = || CallFailure::Fault(InternalFault::new("FR-560", "jit-argument-kind"));
    compiled
        .parameters
        .iter()
        .zip(arguments)
        .map(|(kind, argument)| match (kind, argument) {
            (Some(Carried::Boolean), Value::Boolean(value)) => Ok(i64::from(*value)),
            (Some(Carried::Register), Value::Integer(value)) => {
                integer_to_i64(value).ok_or_else(broken)
            }
            (Some(Carried::Handle), _) => {
                let handle = i64::try_from(arena.len()).map_err(|_| broken())?;
                arena.push(argument.clone());
                Ok(handle)
            }
            (None, _) => Ok(0),
            _ => Err(broken()),
        })
        .collect()
}
