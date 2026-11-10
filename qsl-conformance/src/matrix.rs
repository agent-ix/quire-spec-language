// SPDX-License-Identifier: AGPL-3.0-or-later
//! The matrix vocabulary (FR-564, FR-565): verbs, outcome kinds and cases.

use qsl_analyze::analyze::DeadlineSupply;
use qsl_replay::ToolFailure;
use quire_exact::{CancelCause, ScalarLimits};
use quire_semantic_value::catalog::Category;

/// A lifecycle verb of FR-564's matrix.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Verb {
    /// `check`: S3 and S4 over a parsed unit.
    Check,
    /// `package`: E4 over a checked unit.
    Package,
    /// `execute`: a checked function called on an execution backend.
    Execute,
    /// `analyze`: claims decided by in-process engines.
    Analyze,
    /// `monitor`: temporal clauses over a supplied trace.
    Monitor,
    /// The QSL half of `prove`: terminal records of QSL engines and plugin
    /// settlements.
    Prove,
}

impl Verb {
    /// The verb's name, as FR-567 names a failing case.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Check => "check",
            Self::Package => "package",
            Self::Execute => "execute",
            Self::Analyze => "analyze",
            Self::Monitor => "monitor",
            Self::Prove => "prove",
        }
    }

    /// The verbs in FR-564's table order.
    pub const ALL: [Self; 6] = [
        Self::Check,
        Self::Package,
        Self::Execute,
        Self::Analyze,
        Self::Monitor,
        Self::Prove,
    ];
}

/// One of FR-564's eight outcome kinds.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum OutcomeKind {
    /// Category success, exit 0.
    Success,
    /// Category violation, exit 10.
    Violation,
    /// Category refusal, exit 20.
    Refusal,
    /// Category unsupported, exit 21.
    Unsupported,
    /// Category incomplete by a `Deadline` cancel, exit 22.
    Timeout,
    /// Category incomplete by a `Requested` cancel, exit 22.
    Cancellation,
    /// Category incomplete by an exhausted limit, exit 22.
    Incomplete,
    /// Category internal failure, exit 30.
    ToolFailure,
}

impl OutcomeKind {
    /// The kind's name, as FR-567 names a failing case.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Violation => "violation",
            Self::Refusal => "refusal",
            Self::Unsupported => "unsupported",
            Self::Timeout => "timeout",
            Self::Cancellation => "cancellation",
            Self::Incomplete => "incomplete",
            Self::ToolFailure => "tool-failure",
        }
    }

    /// The kinds in FR-564's table order.
    pub const ALL: [Self; 8] = [
        Self::Success,
        Self::Violation,
        Self::Refusal,
        Self::Unsupported,
        Self::Timeout,
        Self::Cancellation,
        Self::Incomplete,
        Self::ToolFailure,
    ];
}

/// When a case cancels its handle (FR-276): never, or before the operation
/// under test is called, so the outcome does not depend on timing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cancellation {
    /// The handle is never cancelled.
    None,
    /// The handle is cancelled with the cause before the operation is called.
    BeforeCall(CancelCause),
}

/// One input of a case: its bytes and the file name the driver writes them
/// under in the case's working directory (FR-565). Its authored source label
/// is supplied separately in [`RequestParameters`].
#[derive(Clone, Copy, Debug)]
pub struct Input {
    /// The file name, relative to the case's working directory.
    pub file_name: &'static str,
    /// The input's bytes.
    pub bytes: &'static [u8],
}

/// The working directory a case runs the `quire` CLI in (FR-565).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkingDirectory {
    /// A new empty directory the driver creates for the case; each input is
    /// written at its file name and each file argument is spelled as that
    /// file name.
    NewEmpty,
}

/// A limit a front-end case sets one below the counter its input reaches.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrontEndLimit {
    /// Every limit at its default.
    Default,
    /// `s3.nodes` set to the value (the `check` limit).
    S3Nodes(u64),
    /// `emit.output_bytes` set to the value (the `package` limit).
    OutputBytes(u64),
}

/// The inputs of a `check` or `package` case.
#[derive(Clone, Copy, Debug)]
pub struct FrontEndInput {
    /// The unit.
    pub unit: Input,
    /// The domain package documents the unit's `model` declarations select.
    pub packages: &'static [Input],
    /// The limit the operation under test runs under.
    pub limit: FrontEndLimit,
}

/// The value of one argument of an `execute` case.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArgumentValue {
    /// An integer.
    Integer(i64),
    /// A Boolean.
    Boolean(bool),
}

/// One `{parameter, value}` argument of an `execute` case.
#[derive(Clone, Copy, Debug)]
pub struct ExecuteArgument {
    /// The parameter's declared name.
    pub parameter: &'static str,
    /// The argument's value.
    pub value: ArgumentValue,
}

/// The execution backend an `execute` case chooses and supplies (FR-296).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendPlan {
    /// The interpreter, which `qsl-eval` owns.
    Interpreter,
    /// `jit` is chosen and not supplied: the call refuses as unsupported
    /// with cause `NotSupplied` (FR-296-AC-2).
    JitNotSupplied,
    /// `aot` is chosen and [`crate::FaultInjectingBackend`] is supplied for
    /// it: `prepare` succeeds and `call` returns `CallFailure::Fault`.
    FaultInjecting,
}

/// The inputs of an `execute` case.
#[derive(Clone, Copy, Debug)]
pub struct ExecuteInput {
    /// The unit that declares the called function.
    pub unit: Input,
    /// The called function's name.
    pub function: &'static str,
    /// The call's arguments.
    pub arguments: &'static [ExecuteArgument],
    /// The backend the call is made on.
    pub backend: BackendPlan,
}

/// The population a subject simulates and its finite key set (FR-120).
#[derive(Clone, Copy, Debug)]
pub struct PopulationInput {
    /// The population's package identity.
    pub package: &'static str,
    /// The population's declaration node identity.
    pub node: &'static str,
    /// The object keys the population may hold.
    pub keys: &'static [&'static str],
}

/// A model subject: the unit, its domain packages, the snapshot documents
/// and the initial states and universe an `analyze` request binds.
#[derive(Clone, Copy, Debug)]
pub struct SubjectInput {
    /// The unit that selects the model.
    pub unit: Input,
    /// The domain package documents.
    pub packages: &'static [Input],
    /// The snapshot documents, each read under the identity it names.
    pub snapshots: &'static [Input],
    /// The initial states, as `(authority, identity)` snapshot references.
    pub initial: &'static [(&'static str, &'static str)],
    /// The simulated population.
    pub population: PopulationInput,
}

/// The inputs of a `monitor` case.
#[derive(Clone, Copy, Debug)]
pub struct MonitorInput {
    /// The unit that declares the temporal clauses.
    pub unit: Input,
    /// The domain package documents.
    pub packages: &'static [Input],
    /// The snapshot documents the trace positions name.
    pub snapshots: &'static [Input],
    /// The `quire.monitor-trace/1` trace document.
    pub trace: Input,
    /// The selected clauses, in request order.
    pub clauses: &'static [&'static str],
    /// The population the clauses' `over` object lives in.
    pub population: &'static str,
    /// The `over` object's key.
    pub key: &'static str,
}

/// The engine item a case runs over its model subject.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EngineItem {
    /// An EN-4 probabilistic claim, by its declared name.
    Statistical {
        /// The claim's declared name.
        claim: &'static str,
    },
    /// An item over the unit's one authored temporal clause, negotiated to
    /// EN-1, which runs it.
    Clause,
    /// The same clause item with the `unsupported` disposition and its
    /// `unsupported-requested-capability` diagnostic that negotiation gives
    /// an item no layer-A engine runs (FR-281-AC-3).
    UnsupportedClause,
    /// An EN-5 exact probabilistic claim, by its declared name.
    Exact {
        /// The claim's declared name.
        claim: &'static str,
    },
}

/// The inputs of an `analyze` case.
#[derive(Clone, Copy, Debug)]
pub enum AnalyzeInput {
    /// One engine item over `subject`.
    Engine {
        /// The model subject.
        subject: SubjectInput,
        /// The item.
        item: EngineItem,
        /// The request's deadline.
        deadline: DeadlineSupply,
    },
}

/// A plugin run's settlement (FR-291), which the driver observes and QSL
/// turns into a terminal record.
#[derive(Clone, Copy, Debug)]
pub enum PluginSettlement {
    /// The run exceeded its deadline.
    TimedOut,
    /// The run was cancelled with the cause.
    Cancelled(CancelCause),
    /// The run failed with the defect.
    Failed(ToolFailure),
}

/// The inputs of a `prove` (QSL half) case.
#[derive(Clone, Copy, Debug)]
pub enum ProveInput {
    /// One engine item served as a provider through
    /// `qsl_analyze::provider::run`.
    Provider {
        /// The model subject.
        subject: SubjectInput,
        /// The item.
        item: EngineItem,
    },
    /// The terminal record FR-291 builds from a plugin run's settlement.
    Plugin(PluginSettlement),
}

/// The operation inputs of a case.
#[derive(Clone, Copy, Debug)]
pub enum CaseInput {
    /// A `check` case.
    Check(FrontEndInput),
    /// A `package` case; `check` runs first, uncancelled and at defaults.
    Package(FrontEndInput),
    /// An `execute` case.
    Execute(ExecuteInput),
    /// An `analyze` case.
    Analyze(AnalyzeInput),
    /// A `monitor` case.
    Monitor(MonitorInput),
    /// A `prove` (QSL half) case.
    Prove(ProveInput),
}

/// The request parameters and limits of a case (FR-565): everything the
/// operation is called with besides the input bytes and the cancellation
/// plan, so the driver reproduces a case from the case alone.
#[derive(Clone, Copy, Debug)]
pub struct RequestParameters {
    /// The authored source authority, independent of the input file path.
    pub source_authority: &'static str,
    /// The authored source identity, independent of the input file path.
    pub source_identity: &'static str,
    /// The package name the unit is checked under.
    pub package_name: &'static str,
    /// The `quire.value.accounting/v1` limits the operation charges under;
    /// `work_units` is the call's limit for `execute`.
    pub accounting: ScalarLimits,
    /// `monitor`'s `max_window_values`.
    pub max_window_values: u64,
    /// An EN-4 item's deterministic sample seed.
    pub statistical_seed: u64,
    /// An EN-4 item's retained-witness bound.
    pub max_witnesses: u32,
    /// An EN-4 item's sampler definition, as `(authority, identity)`.
    pub sampler: (&'static str, &'static str),
    /// `limits.max_states` of an EN-1 or EN-5 item, or `None` for the
    /// engine's published default.
    pub max_states: Option<usize>,
}

/// One applicable cell: its inputs, cancellation plan and expectations.
#[derive(Clone, Copy, Debug)]
pub struct MatrixCase {
    /// The cell's verb.
    pub verb: Verb,
    /// The cell's outcome kind.
    pub kind: OutcomeKind,
    /// The operation inputs; every byte the case needs.
    pub input: CaseInput,
    /// The request parameters and limits.
    pub parameters: RequestParameters,
    /// The cancellation plan.
    pub cancellation: Cancellation,
    /// The working directory the driver runs `quire` in.
    pub working_directory: WorkingDirectory,
    /// The O-16 category the outcome has.
    pub expected_category: Category,
    /// The FR-285 exit code the outcome has.
    pub expected_exit: u8,
}

/// A cell of the matrix.
#[derive(Clone, Copy, Debug)]
#[allow(
    clippy::large_enum_variant,
    reason = "cells are `Copy` static data, and the matrix holds a case in most of them"
)]
pub enum MatrixCell {
    /// An applicable cell.
    Case(MatrixCase),
    /// A cell the verb cannot reach, with FR-564's reason.
    NotApplicable {
        /// The cell's verb.
        verb: Verb,
        /// The cell's outcome kind.
        kind: OutcomeKind,
        /// FR-564's reason for the cell.
        reason: &'static str,
    },
}

impl MatrixCell {
    /// The cell's verb.
    #[must_use]
    pub const fn verb(&self) -> Verb {
        match self {
            Self::Case(case) => case.verb,
            Self::NotApplicable { verb, .. } => *verb,
        }
    }

    /// The cell's outcome kind.
    #[must_use]
    pub const fn kind(&self) -> OutcomeKind {
        match self {
            Self::Case(case) => case.kind,
            Self::NotApplicable { kind, .. } => *kind,
        }
    }
}
