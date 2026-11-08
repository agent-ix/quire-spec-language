// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-279, FR-294, FR-296 (ADR-029 EB-1 to EB-3, EB-8): the execution-backend
//! seam, the interpreter backend and `execute`.
//!
//! Every backend is an [`ExecutionBackend`]. It prepares the selected
//! entries from a [`CheckedPackage`], the only input a backend admits, and
//! the prepared value runs them. An entry a backend cannot run is recorded
//! beside the prepared value as unsupported, for that entry only. The
//! interpreter is the reference: it accepts every entry S6a accepts and runs
//! S6a, and replay runs only through it.
//!
//! # Checked input only (FR-294-AC-2)
//!
//! `prepare` takes a [`CheckedPackage`], which only S4 constructs. Package
//! bytes and unchecked input do not compile:
//!
//! ```compile_fail,E0308
//! use qsl_eval::execution::{ExecutionBackend, Interpreter, PrepareLimits};
//! use quire_exact::Cancel;
//! fn bytes(backend: Interpreter, cancel: &Cancel) {
//!     let _ = backend.prepare(&b"package bytes"[..], &[], &PrepareLimits::default(), cancel);
//! }
//! ```
//!
//! A parsed source does not compile either:
//!
//! ```compile_fail,E0308
//! use qsl_cst::ParsedSource;
//! use qsl_eval::execution::{ExecutionBackend, Interpreter, PrepareLimits};
//! use quire_exact::Cancel;
//! fn parsed(backend: Interpreter, source: &ParsedSource, cancel: &Cancel) {
//!     let _ = backend.prepare(source, &[], &PrepareLimits::default(), cancel);
//! }
//! ```
//!
//! Nor does an unchecked expression:
//!
//! ```compile_fail,E0308
//! use qsl_eval::execution::{ExecutionBackend, Interpreter, PrepareLimits};
//! use qsl_forms::Expression;
//! use quire_exact::Cancel;
//! fn unchecked(backend: Interpreter, expression: &Expression, cancel: &Cancel) {
//!     let _ = backend.prepare(expression, &[], &PrepareLimits::default(), cancel);
//! }
//! ```
//!
//! The control passes a checked package and compiles:
//!
//! ```
//! use qsl_eval::execution::{ExecutionBackend, Interpreter, PrepareLimits};
//! use qsl_package::CheckedPackage;
//! use quire_exact::Cancel;
//! fn checked(backend: Interpreter, package: &CheckedPackage, cancel: &Cancel) {
//!     let _ = backend.prepare(package, &[], &PrepareLimits::default(), cancel);
//! }
//! ```
//!
//! A clause entry takes the clause's name and the `AdmittedObservations`
//! FR-106 admitted for it, the inputs of S6a's `evaluate_clause`. `execute`
//! does not compile, select or admit; `qsl_replay::spine::run_clause` is the
//! reference pipeline that does and calls `evaluate_clause` itself, so clause
//! execution has one evaluator and two callers.
//!
//! `ExecuteLimits` has no `Default`, because `ScalarLimits` has none: the
//! caller states `accounting` (FR-277 Inputs).
//!
//! ```compile_fail,E0063
//! use qsl_eval::execution::ExecuteLimits;
//! fn limits() -> ExecuteLimits {
//!     ExecuteLimits {
//!         supplied: Default::default(),
//!         prepare: Default::default(),
//!     }
//! }
//! ```
//!
//! ```compile_fail,E0277
//! use qsl_eval::execution::ExecuteLimits;
//! fn limits() -> ExecuteLimits {
//!     ExecuteLimits { ..Default::default() }
//! }
//! ```
//!
//! The control states `accounting` explicitly:
//!
//! ```
//! use qsl_eval::execution::{ExecuteLimits, PrepareLimits};
//! use quire_exact::ScalarLimits;
//! use quire_semantic_value::supplied_stop::SuppliedLimits;
//! fn limits(accounting: ScalarLimits) -> ExecuteLimits {
//!     ExecuteLimits {
//!         accounting,
//!         supplied: SuppliedLimits::default(),
//!         prepare: PrepareLimits::default(),
//!     }
//! }
//! ```
//!
//! `ExecutionBackend::Prepared` is a generic associated type because a
//! prepared value borrows the package it was prepared from; the interpreter
//! is that case.

use qsl_foundation::diagnostic::LimitExceeded;
use qsl_foundation::{Setting, SettingLimits};
use qsl_package::CheckedPackage;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::model::observation::AdmittedObservations;
use quire_eval::{
    CallFailure, Evaluation, ExecutionBackendChoice, PrepareLimitReached, PrepareSetting,
    UnsupportedCause, UnsupportedEntry,
};
use quire_exact::{Cancel, CancelCause, ScalarLimits, Value};
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::supplied_stop::{SuppliedLimits, SuppliedRun};

use crate::value::{CheckedPackageEvaluation, QualifiedName};

/// FR-294: the translation work and generated code size one `prepare` may
/// use, the FR-255 settings `prepare.compile_work_units` and
/// `prepare.code_bytes`. Each bound is used as given, including zero. The
/// interpreter translates nothing and charges neither.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrepareLimits {
    /// Compile work units one `prepare` may charge.
    pub compile_work_units: u64,
    /// Generated code bytes one `prepare` may keep.
    pub code_bytes: u64,
}

impl PrepareLimits {
    /// The published defaults (FR-294).
    pub const DEFAULT: Self = Self {
        compile_work_units: 16_777_216,
        code_bytes: 67_108_864,
    };
}

impl Default for PrepareLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl SettingLimits for PrepareLimits {
    fn bounds(&self) -> Vec<(Setting, u64)> {
        vec![
            (Setting::PrepareCompileWorkUnits, self.compile_work_units),
            (Setting::PrepareCodeBytes, self.code_bytes),
        ]
    }

    fn set_bound(&mut self, setting: Setting, bound: u64) -> bool {
        match setting {
            Setting::PrepareCompileWorkUnits => self.compile_work_units = bound,
            Setting::PrepareCodeBytes => self.code_bytes = bound,
            _ => return false,
        }
        true
    }
}

/// One entry to prepare: a checked function or a checked state clause, by
/// name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EntrySelection {
    /// A checked function, by name.
    Function(QualifiedName),
    /// A checked state clause, by name.
    Clause(QualifiedName),
}

impl EntrySelection {
    /// The selected entry's name.
    #[must_use]
    pub const fn name(&self) -> &QualifiedName {
        match self {
            Self::Function(name) | Self::Clause(name) => name,
        }
    }
}

/// Why `prepare` stopped. It fails the whole `prepare`; there is no
/// per-entry variant (an unsupported entry is recorded, not failed).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrepareFailure {
    /// A `PrepareLimits` bound was reached.
    Limit(LimitExceeded),
    /// The caller's handle was cancelled.
    Cancelled(CancelCause),
    /// A backend invariant broke.
    Fault(InternalFault),
}

/// The limits one `call` runs under.
#[derive(Clone, Copy, Debug)]
pub struct CallLimits {
    /// The `quire.value.accounting/v1` limits.
    pub accounting: ScalarLimits,
    /// FR-277: the supplied-value helper ceilings.
    pub supplied: SuppliedLimits,
}

/// The prepared entries of one `prepare`.
pub trait PreparedEntries {
    /// The selected entries the backend did not prepare, in selection order,
    /// each with its cause.
    fn unsupported(&self) -> &[(EntrySelection, UnsupportedEntry)];

    /// Call the function `entry` with the contract of
    /// [`CheckedPackageEvaluation::call_supplied`] under a supplied-value run
    /// built from `limits.supplied` and `cancel`, metered by
    /// `limits.accounting`. An entry listed by [`Self::unsupported`] returns
    /// [`CallFailure::Unsupported`] with its [`UnsupportedEntry`].
    fn call(
        &self,
        entry: &QualifiedName,
        arguments: &[Value],
        environment: &ObjectEnvironment,
        limits: &CallLimits,
        cancel: &Cancel,
    ) -> Result<Evaluation, CallFailure>;

    /// Evaluate the state clause `clause` over `input` with the contract of
    /// [`CheckedPackageEvaluation::evaluate_clause`], metered by
    /// `accounting`, and return that evaluation's `evaluation` member. The
    /// input is already admitted, so no supplied-value phase applies. A
    /// clause listed by [`Self::unsupported`] returns
    /// [`CallFailure::Unsupported`] with its [`UnsupportedEntry`].
    fn evaluate(
        &self,
        clause: &QualifiedName,
        input: &AdmittedObservations,
        accounting: &ScalarLimits,
        cancel: &Cancel,
    ) -> Result<Evaluation, CallFailure>;
}

/// An execution backend (ADR-029 EB-1).
pub trait ExecutionBackend {
    /// What this backend prepared; it may borrow the package.
    type Prepared<'p>: PreparedEntries + 'p;

    /// Prepare `entries` of `package`. An entry this backend cannot run is
    /// recorded in the prepared value as unsupported and does not fail the
    /// `prepare`.
    ///
    /// # Errors
    /// [`PrepareFailure`] when a prepare limit is reached, the handle is
    /// cancelled, or a backend invariant breaks.
    fn prepare<'p>(
        &self,
        package: &'p CheckedPackage,
        entries: &[EntrySelection],
        limits: &PrepareLimits,
        cancel: &Cancel,
    ) -> Result<Self::Prepared<'p>, PrepareFailure>;
}

/// The object-safe form of [`ExecutionBackend`] (FR-296), implemented for
/// every backend by boxing its prepared value.
pub trait SuppliedBackend {
    /// [`ExecutionBackend::prepare`], boxed.
    ///
    /// # Errors
    /// As [`ExecutionBackend::prepare`].
    fn prepare_entries<'p>(
        &self,
        package: &'p CheckedPackage,
        entries: &[EntrySelection],
        limits: &PrepareLimits,
        cancel: &Cancel,
    ) -> Result<Box<dyn PreparedEntries + 'p>, PrepareFailure>;
}

impl<B: ExecutionBackend> SuppliedBackend for B {
    fn prepare_entries<'p>(
        &self,
        package: &'p CheckedPackage,
        entries: &[EntrySelection],
        limits: &PrepareLimits,
        cancel: &Cancel,
    ) -> Result<Box<dyn PreparedEntries + 'p>, PrepareFailure> {
        let prepared = self.prepare(package, entries, limits, cancel)?;
        Ok(Box::new(prepared))
    }
}

/// The backends the caller supplies. The interpreter is always available, so
/// it has no member; `default()` supplies the interpreter alone.
#[derive(Clone, Copy, Default)]
pub struct SuppliedBackends<'a> {
    /// The ahead-of-time backend, if linked.
    pub aot: Option<&'a dyn SuppliedBackend>,
    /// The just-in-time backend, if linked.
    pub jit: Option<&'a dyn SuppliedBackend>,
}

impl<'a> SuppliedBackends<'a> {
    /// The supplied backend `choice` names, `None` for the interpreter,
    /// which `qsl-eval` owns (FR-296).
    ///
    /// # Errors
    /// [`UnsupportedEntry`] with cause `NotSupplied` when `choice` is a
    /// backend the caller did not supply.
    pub fn member(
        &self,
        choice: ExecutionBackendChoice,
    ) -> Result<Option<&'a dyn SuppliedBackend>, UnsupportedEntry> {
        let member = match choice {
            ExecutionBackendChoice::Interpreter => return Ok(None),
            ExecutionBackendChoice::Aot => self.aot,
            ExecutionBackendChoice::Jit => self.jit,
        };
        member.map(Some).ok_or(UnsupportedEntry {
            backend: choice,
            cause: UnsupportedCause::NotSupplied,
        })
    }
}

/// The charges of one `prepare` (FR-294 "Prepare charges"). A backend
/// charges one work unit before it translates each checked node and the byte
/// length of each block before it keeps it; each charge polls the handle.
#[derive(Debug)]
pub struct PrepareMeter<'a> {
    limits: &'a PrepareLimits,
    cancel: &'a Cancel,
    work: u64,
    bytes: u64,
}

impl<'a> PrepareMeter<'a> {
    /// A meter with nothing charged.
    #[must_use]
    pub const fn new(limits: &'a PrepareLimits, cancel: &'a Cancel) -> Self {
        Self {
            limits,
            cancel,
            work: 0,
            bytes: 0,
        }
    }

    /// Charge one compile work unit.
    ///
    /// # Errors
    /// [`PrepareFailure::Cancelled`] for a cancelled handle, else
    /// [`PrepareFailure::Limit`] naming the counter the charge would reach.
    pub fn charge_node(&mut self) -> Result<(), PrepareFailure> {
        self.poll()?;
        let reached = u128::from(self.work) + 1;
        if reached > u128::from(self.limits.compile_work_units) {
            return Err(PrepareFailure::Limit(LimitExceeded::new(
                Setting::PrepareCompileWorkUnits,
                self.limits.compile_work_units,
                reached,
            )));
        }
        self.work += 1;
        Ok(())
    }

    /// Charge `length` generated-code bytes.
    ///
    /// # Errors
    /// As [`Self::charge_node`], for `prepare.code_bytes`.
    pub fn charge_block(&mut self, length: u64) -> Result<(), PrepareFailure> {
        self.poll()?;
        let reached = u128::from(self.bytes) + u128::from(length);
        if reached > u128::from(self.limits.code_bytes) {
            return Err(PrepareFailure::Limit(LimitExceeded::new(
                Setting::PrepareCodeBytes,
                self.limits.code_bytes,
                reached,
            )));
        }
        self.bytes += length;
        Ok(())
    }

    fn poll(&self) -> Result<(), PrepareFailure> {
        self.cancel
            .cause()
            .map_or(Ok(()), |cause| Err(PrepareFailure::Cancelled(cause)))
    }
}

/// The interpreter backend: it accepts every entry S6a accepts, translates
/// nothing, and runs S6a.
#[derive(Clone, Copy, Debug, Default)]
pub struct Interpreter;

/// What [`Interpreter`] prepared: the package, nothing translated.
#[derive(Debug)]
pub struct InterpreterPrepared<'p> {
    package: &'p CheckedPackage,
}

impl ExecutionBackend for Interpreter {
    type Prepared<'p> = InterpreterPrepared<'p>;

    fn prepare<'p>(
        &self,
        package: &'p CheckedPackage,
        _entries: &[EntrySelection],
        _limits: &PrepareLimits,
        cancel: &Cancel,
    ) -> Result<InterpreterPrepared<'p>, PrepareFailure> {
        match cancel.cause() {
            Some(cause) => Err(PrepareFailure::Cancelled(cause)),
            None => Ok(InterpreterPrepared { package }),
        }
    }
}

impl PreparedEntries for InterpreterPrepared<'_> {
    fn unsupported(&self) -> &[(EntrySelection, UnsupportedEntry)] {
        &[]
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
        let mut meter = quire_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
        // One run for the whole invocation: both helper budgets and the
        // caller's original handle, never a fresh one (FR-277).
        let mut run =
            SuppliedRun::new(limits.supplied, cancel.clone()).map_err(CallFailure::Supplied)?;
        let evaluation = self.package.call_supplied(
            function,
            arguments.to_vec(),
            environment,
            &mut meter,
            &mut run,
        );
        if let Some(cause) = cancel.tripped() {
            return Err(CallFailure::Cancelled(cause));
        }
        evaluation
    }

    fn evaluate(
        &self,
        clause: &QualifiedName,
        input: &AdmittedObservations,
        accounting: &ScalarLimits,
        cancel: &Cancel,
    ) -> Result<Evaluation, CallFailure> {
        if let Some(cause) = cancel.cause() {
            return Err(CallFailure::Cancelled(cause));
        }
        let mut meter = quire_exact::Meter::new(*accounting).with_cancel(cancel.clone());
        let evaluated = self.package.evaluate_clause(clause, input, &mut meter);
        if let Some(cause) = cancel.tripped() {
            return Err(CallFailure::Cancelled(cause));
        }
        evaluated.map(|clause| clause.evaluation)
    }
}

/// What `execute` calls.
#[derive(Clone, Debug)]
#[allow(
    clippy::large_enum_variant,
    reason = "FR-279 states the clause input by value"
)]
pub enum ExecuteSelection {
    /// A function, its argument values in parameter order and the objects it
    /// reads.
    Function {
        /// The function.
        name: QualifiedName,
        /// Its arguments.
        arguments: Vec<Value>,
        /// The object environment the call reads objects from.
        environment: ObjectEnvironment,
    },
    /// A state clause and the observations FR-106 admitted for it, which
    /// supply every object and value it reads.
    Clause {
        /// The clause.
        clause: QualifiedName,
        /// The admitted observations.
        input: AdmittedObservations,
    },
}

/// FR-279: the `execute` request.
#[derive(Clone, Debug)]
pub struct ExecuteRequest {
    /// The entry to run.
    pub selection: ExecuteSelection,
    /// The backend that runs it (FR-296); `Interpreter` by default.
    pub backend: ExecutionBackendChoice,
}

/// FR-279: the limits `execute` runs under. It has no `Default`, because
/// `ScalarLimits` has none: the caller states `accounting`.
#[derive(Clone, Copy, Debug)]
pub struct ExecuteLimits {
    /// The accounting limits the run charges under.
    pub accounting: ScalarLimits,
    /// FR-277: the supplied-value helper ceilings; read for a function's
    /// arguments only.
    pub supplied: SuppliedLimits,
    /// The one `PrepareLimits` that applies to whichever backend is chosen.
    pub prepare: PrepareLimits,
}

/// FR-279: run a checked function or clause through the chosen backend:
/// prepare exactly the selected entry under `limits.prepare`, then call or
/// evaluate it.
///
/// # Errors
/// [`CallFailure::Unsupported`] when the chosen backend is not supplied or
/// listed the entry unsupported; [`CallFailure::Limit`] when `prepare`
/// reached a prepare limit; [`CallFailure::Cancelled`] or
/// [`CallFailure::Fault`] from `prepare` or the run; otherwise the run's.
pub fn execute(
    package: &CheckedPackage,
    request: &ExecuteRequest,
    backends: &SuppliedBackends<'_>,
    limits: &ExecuteLimits,
    cancel: &Cancel,
) -> Result<Evaluation, CallFailure> {
    let entry = match &request.selection {
        ExecuteSelection::Function { name, .. } => EntrySelection::Function(name.clone()),
        ExecuteSelection::Clause { clause, .. } => EntrySelection::Clause(clause.clone()),
    };
    let interpreter = Interpreter;
    let chosen: &dyn SuppliedBackend = backends
        .member(request.backend)
        .map_err(CallFailure::Unsupported)?
        .unwrap_or(&interpreter);
    let prepared = chosen
        .prepare_entries(
            package,
            std::slice::from_ref(&entry),
            &limits.prepare,
            cancel,
        )
        .map_err(call_failure)?;
    if let Some((_, unsupported)) = prepared
        .unsupported()
        .iter()
        .find(|(listed, _)| *listed == entry)
    {
        return Err(CallFailure::Unsupported(unsupported.clone()));
    }
    match &request.selection {
        ExecuteSelection::Function {
            name,
            arguments,
            environment,
        } => prepared.call(
            name,
            arguments,
            environment,
            &CallLimits {
                accounting: limits.accounting,
                supplied: limits.supplied,
            },
            cancel,
        ),
        ExecuteSelection::Clause { clause, input } => {
            prepared.evaluate(clause, input, &limits.accounting, cancel)
        }
    }
}

/// FR-279: a `prepare` failure as the call failure `execute` returns. A
/// reached limit becomes the leaf's locus-free [`PrepareLimitReached`]; a
/// limit naming a setting that is not a prepare setting, or a counter beyond
/// `u64`, is a backend invariant break.
fn call_failure(failure: PrepareFailure) -> CallFailure {
    match failure {
        PrepareFailure::Limit(limit) => {
            let setting = match limit.setting() {
                Setting::PrepareCompileWorkUnits => PrepareSetting::CompileWorkUnits,
                Setting::PrepareCodeBytes => PrepareSetting::CodeBytes,
                _ => {
                    return CallFailure::Fault(InternalFault::new(
                        "execution",
                        "a-prepare-limit-names-a-prepare-setting",
                    ))
                }
            };
            match u64::try_from(limit.actual()) {
                Ok(actual) => CallFailure::Limit(PrepareLimitReached {
                    setting,
                    bound: limit.configured_bound(),
                    actual,
                }),
                Err(_) => CallFailure::Fault(InternalFault::new(
                    "execution",
                    "a-prepare-counter-fits-u64",
                )),
            }
        }
        PrepareFailure::Cancelled(cause) => CallFailure::Cancelled(cause),
        PrepareFailure::Fault(fault) => CallFailure::Fault(fault),
    }
}
