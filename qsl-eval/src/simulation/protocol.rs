// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-205, FR-206, FR-210, FR-211, FR-215 and FR-216 (ADR-027):
//! [`ProtocolSystem`], the FR-101 [`TransitionSystem`] of a protocol subject,
//! beside [`ModelSystem`], and its steps' footprints.
//!
//! A protocol state is the model state, the live protocol instances (each
//! with its threads, their binder environments and its parallel instances)
//! and the memory component (ADR-027 PS-8). Its successors are every enabled step of every
//! running thread and every internal step of the memory model; a step
//! settles each thread it moves or creates to its next rest point, so a
//! structural move is never a step (FO-1). An attempt is one FR-120
//! application through the one application function `ModelSystem` uses,
//! over the clauses its `contracts` select (TS-3), and the system reaches
//! memory only through [`MemoryModel`] (SE-1).
//!
//! This system steps `sequence`, `parallel` with its join policies `all`,
//! `any` and `quorum`, `choice`, `repeat`, `check`, `await` with its
//! `timeout`, `attempt`, `send`, `receive`, domain `event`, `commit`,
//! `fence` (SE-5), channel `duplicate` and `lose`, `effect`, `finish`,
//! compensation templates with their `cattempt` and `cend` steps,
//! replicated roles with their `spawn` and `retire` steps, and `activation
//! on each` with its `activate` step and instance bound (ADR-027 §9 steps 3
//! to 5). A protocol that holds any other construct is refused, before any state is built, with
//! `unsupported_construct`/`not-yet-implemented` naming it.

mod access;
mod channel;
mod compensation;
mod fairness;
mod footprint;
mod identity;
mod memory;
mod program;
mod ra;
mod roles;
mod selection;
mod settle;
mod steps;
mod tso;

use std::borrow::Cow;
use std::collections::BTreeMap;

use qsl_foundation::diagnostic::Code;
use qsl_foundation::digest::{DigestRecord, WireNodeId};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckedActivation, CheckedProtocol, CheckedTerminalMember};
use quire_canonical::{Encode, Error, Limits as CanonicalLimits, Sink, Writer};
use quire_exact::{Meter, ObjectReference, Value, ValueType};
use quire_semantic_value::catalog::InternalFault;

use crate::fairness::{Constraint, Identity};
use crate::model_subject::{ModelInstance, ProtocolSubject};
use crate::simulation::expansion::{Expansion, ExpansionLimit, ExpansionStop};
use crate::simulation::explore::TransitionSystem;
use crate::simulation::key::{self, EncodingRefusal};
use crate::simulation::model::encode::{map_enc, reference_enc, Enc};
use crate::simulation::model::{
    verdict, ModelFinding, ModelSimState, ModelSystem, TerminalClass, Verdict, BROKEN,
};
use crate::simulation::not_simulated::{check_requires_bound, NotSimulated};
use crate::simulation::progress::ZeroProgressError;
use crate::simulation::subject::{Bounded, Cut, SubjectSystem};
use crate::value::evaluate_protocol_slot;
use steps::ProtocolExpansion;

pub use access::{
    AccessShape, AttemptInstance, InstanceAccess, WeakAccessShape, WeakAccesses, WeakAttempt,
};
pub use fairness::{
    protocol_constraints, FairnessOrigin, ProtocolClass, ProtocolFairness, ProtocolFairnessError,
    ProtocolTarget,
};
pub use footprint::{
    visible, AnchorName, ClaimReads, ParallelInstance, ProtocolFootprintError, ProtocolLocation,
};
pub use identity::{
    Actor, BlockedThread, CarriedComponent, PathStep, PositionAnchor, ProtocolTransition,
    RegistrationKey, RestKind, RestPoint, SendOutcome, SendRecord, StepKind, ThreadPath, WaitCause,
};
pub use memory::{
    Access, CarriedKey, Chosen, FootprintStep, GatedStep, MemoryChoice, MemoryModel, MemoryStep,
    MemoryStepId, Read, ReadChoice, Sc, ScComponent, SharedCell, Written,
};
use program::{Program, Rest};
pub use ra::{CarriedView, Ra, RaComponent};
pub use selection::{
    MemorySelection, MemorySelectionRefusal, ResolvedMemory, ResolvedParallel, SubjectMemory,
};
pub use tso::{Buffered, Tso, TsoComponent, DEFAULT_MEMORY_BOUND};

/// A construct this protocol system does not step yet.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Unsupported {
    /// A `run` control of this kind, by its keyword.
    Control(&'static str),
    /// A `parallel` whose resolved memory model the system's memory model
    /// does not implement.
    WeakMemory,
    /// An attempt of a weak `parallel` whose application has no footprint
    /// to classify its access by (FR-220).
    AccessFootprint,
    /// A channel whose capacity is `symbolic(..)`, with no bound to read.
    SymbolicCapacity,
    /// An `unordered` channel whose overflow loses its `head` or `tail`,
    /// which a bag does not order.
    UnorderedLossSelector,
    /// A `choice` guard, `repeat` guard or `check` undecided at an initial
    /// state, before any position a finding can name.
    UndecidedInitialGuard,
    /// A retry relation of a template with more than one attempt that reads
    /// the attempt records, which no operation result binds here (ST-11).
    UnboundAttemptRecord,
    /// A recovery predicate that reads its recovery binder, whose recovery
    /// view no rule derives from a model state (ST-12).
    RecoveryView,
    /// A capture of the protocol clause.
    Capture,
    /// `activation on each` in a protocol with a compensation template,
    /// whose expressions read the activation binder at a slot this system
    /// does not fill.
    ActivationOnEachTemplate,
    /// `join predicate(...)`.
    JoinPredicate,
    /// A join whose policy may leave branches running, with no
    /// `outstanding` rule.
    UnruledOutstanding,
    /// An `over` input whose type is not an object type.
    OverNotAnObject,
    /// A slot that reads an attempt binder whose operation returns no value
    /// of the binder's type, so the binder is never bound.
    UnboundAttemptBinder,
    /// A slot that reads a binder with no node other than the `over` input.
    NodelessBinder,
    /// A step binder whose bounded record type has no enumeration here (a
    /// record, an enum or a bounded text type).
    UnenumerableRecord,
}

impl Unsupported {
    /// The construct, as a diagnostic names it.
    pub fn construct(self) -> &'static str {
        match self {
            Self::Control(keyword) => keyword,
            Self::WeakMemory => "memory",
            Self::AccessFootprint => "attempt footprint under a weak memory model",
            Self::SymbolicCapacity => "symbolic channel capacity",
            Self::UnorderedLossSelector => "ordered loss selector on an unordered channel",
            Self::UndecidedInitialGuard => "undecided guard at an initial state",
            Self::UnboundAttemptRecord => "retry relation over unbound attempt records",
            Self::RecoveryView => "recovery predicate over the recovery view",
            Self::Capture => "protocol capture",
            Self::ActivationOnEachTemplate => "activation on each with a compensation template",
            Self::JoinPredicate => "join predicate",
            Self::UnruledOutstanding => "join with no outstanding rule",
            Self::OverNotAnObject => "over input of no object type",
            Self::UnboundAttemptBinder => "read of an unbound attempt binder",
            Self::NodelessBinder => "read of a nodeless binder",
            Self::UnenumerableRecord => "step binder of an unenumerable record type",
        }
    }
}

/// Why [`ProtocolSystem::new`] built no system (FR-205 "Subject").
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProtocolRefusal {
    /// The request names a protocol that is not a protocol clause of the
    /// package (`invalid_runtime_input`/`invalid-value`).
    #[error("invalid_runtime_input/invalid-value: no protocol {protocol}")]
    UnknownProtocol {
        /// The requested protocol.
        protocol: String,
    },
    /// A static role has no binding, or its binding names a model the role
    /// does not admit (`invalid_runtime_input`/`wrong-role-mapping`).
    #[error("invalid_runtime_input/wrong-role-mapping: role {role}")]
    RoleMapping {
        /// The role.
        role: String,
    },
    /// The `over` object is not an object of its declared type's population
    /// universe (`invalid_runtime_input`/`invalid-value`).
    #[error("invalid_runtime_input/invalid-value: {parameter} bound to {object}")]
    OverBinding {
        /// The `over` parameter.
        parameter: String,
        /// The object, by its universe key.
        object: String,
    },
    /// A replicated role's population has no finite universe
    /// (`invalid_runtime_input`/`invalid-value`).
    #[error("invalid_runtime_input/invalid-value: role {role} has no population universe")]
    RolePopulation {
        /// The replicated role.
        role: String,
    },
    /// The subject's resolved memory models do not resolve the protocol's
    /// outermost `parallel` controls (`invalid_runtime_input`/
    /// `invalid-value`).
    #[error(transparent)]
    Memory(MemorySelectionRefusal),
    /// The EN-1 pre-check found an access shape that is not one access
    /// under its weak model, or a weak `parallel` whose join is not `all`
    /// (FR-219, FR-220): the item settles V-8 `WeakAccessShape`.
    #[error("unsupported: weak-access-shape at {}", .0.site)]
    WeakAccessShape(Box<WeakAccessShape>),
    /// The protocol holds a construct this system does not step
    /// (`unsupported_construct`/`not-yet-implemented`).
    #[error("unsupported_construct/not-yet-implemented: {}", .0.construct())]
    Unsupported(Unsupported),
    /// An initial state's settling stopped: a guard's evaluation ran out of
    /// a budget, or broke.
    #[error("{}/{}: settling an initial state", .0.cause.code(), .0.cause.cause())]
    InitialStop(ExpansionStop),
    /// An invariant of the checked protocol broke.
    #[error("the protocol system broke invariant {0:?}")]
    Fault(InternalFault),
}

impl ProtocolRefusal {
    pub(crate) fn fault(invariant: &'static str) -> Self {
        Self::Fault(InternalFault::new("protocol-simulation", invariant))
    }

    /// The refusal's catalog code.
    pub fn code(&self) -> Code {
        match self {
            Self::UnknownProtocol { .. }
            | Self::RoleMapping { .. }
            | Self::OverBinding { .. }
            | Self::RolePopulation { .. }
            | Self::Memory(_) => Code::InvalidRuntimeInput,
            Self::Unsupported(_) | Self::WeakAccessShape(_) => Code::UnsupportedConstruct,
            Self::InitialStop(stop) => {
                Code::from_code(stop.cause.code()).unwrap_or(Code::RuntimeInvariant)
            }
            Self::Fault(_) => Code::RuntimeInvariant,
        }
    }

    /// The refusal's catalog cause.
    pub fn cause(&self) -> &'static str {
        match self {
            Self::UnknownProtocol { .. }
            | Self::OverBinding { .. }
            | Self::RolePopulation { .. }
            | Self::Memory(_) => "invalid-value",
            Self::RoleMapping { .. } => "wrong-role-mapping",
            Self::Unsupported(_) => "not-yet-implemented",
            Self::WeakAccessShape(_) => "weak-access-shape",
            Self::InitialStop(stop) => stop.cause.cause(),
            Self::Fault(_) => "established-invariant-broken",
        }
    }
}

/// A thread's disposition (ADR-027 PS-3).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Disposition {
    /// It can still take steps.
    Running,
    /// Its branch body ended.
    Completed,
    /// An `outstanding cancel` join cancelled it.
    Cancelled,
}

impl Disposition {
    fn text(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }
}

/// One live thread (ADR-027 PS-3): its rest point, its disposition, and
/// for a branch thread the parallel instance and branch label it runs.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Thread {
    rest: Rest,
    disposition: Disposition,
    owner: Option<(ParallelKey, String)>,
}

/// A parallel instance's key (ADR-027 PS-4).
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ParallelKey {
    parent: ThreadPath,
    node: qsl_semantics::check::ProtocolNodeId,
    ordinal: u64,
}

/// A parallel instance's record (ADR-027 PS-4).
#[derive(Clone, Debug, Eq, PartialEq)]
struct Parallel {
    settled: bool,
    dispositions: BTreeMap<String, Disposition>,
}

/// One bound binder value (ADR-027 PS-5): its value, `None` for an
/// attempt's record (receiver, arguments) when its operation returns no
/// value of the binder's type, which only its effects' enabledness reads
/// (ST-2); and its encoding.
#[derive(Clone, Debug)]
struct Bound {
    value: Option<Value>,
    enc: Enc,
}

/// A FIFO key's value: its encoding, ordered by its JCS bytes.
#[derive(Clone, Debug)]
struct FifoKey {
    bytes: Vec<u8>,
    enc: Enc,
}

impl PartialEq for FifoKey {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl Eq for FifoKey {}

impl PartialOrd for FifoKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FifoKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.bytes.cmp(&other.bytes)
    }
}

/// One queue of a protocol instance (ADR-027 PS-7): its channel, by index,
/// and its FIFO key, `None` for an `unordered` channel.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct QueueKey {
    channel: usize,
    fifo: Option<FifoKey>,
}

/// One queue entry (ADR-027 PS-7): the send node and the payload, with the
/// entry's encoding, which names the sending thread too, and its JCS bytes.
#[derive(Clone, Debug)]
struct Entry {
    send: qsl_semantics::check::ProtocolNodeId,
    payload: Value,
    enc: Enc,
    bytes: Vec<u8>,
    /// The memory view the send carried (SE-1 channel effect); none under a
    /// model whose sends carry none.
    carried: Option<CarriedView>,
}

/// A compensation registration's status (ADR-027 CP-1 to CP-5, QSpec
/// FR-427).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Status {
    Registered,
    Activated,
    Closed,
    Recovered,
    Unrecovered,
}

impl Status {
    fn text(self) -> &'static str {
        match self {
            Self::Registered => "registered",
            Self::Activated => "activated",
            Self::Closed => "closed",
            Self::Recovered => "recovered",
            Self::Unrecovered => "unrecovered",
        }
    }

    /// Whether the registration holds its ordinal (CP-1): until it closes
    /// or its compensation thread ends.
    fn live(self) -> bool {
        matches!(self, Self::Registered | Self::Activated)
    }
}

/// A registration's key inside one instance: its template's index, the
/// thread whose effect registered it, and its ordinal.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Registered {
    template: usize,
    thread: ThreadPath,
    ordinal: u64,
}

/// One compensation registration (ADR-027 CP-1): its status and its
/// captures, the registration captures then, once activated, the
/// activation captures.
#[derive(Clone, Debug)]
struct Registration {
    status: Status,
    captures: Vec<Bound>,
}

/// One compensation thread (ADR-027 CP-3): the registration it compensates
/// and its attempt count.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Compensating {
    registration: Registered,
    count: u64,
}

/// A role instance's status (ADR-027 RR-3).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RoleStatus {
    Live,
    Retired,
}

impl RoleStatus {
    fn text(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Retired => "retired",
        }
    }
}

/// One live protocol instance (ADR-027 AE-2).
#[derive(Clone, Debug, Default)]
struct Instance {
    /// The trigger record that started the instance under `activation on
    /// each`, which the activation binder reads (ADR-027 AE-1).
    trigger: Option<Bound>,
    /// The instances of each replicated role (an index into
    /// [`Program::replicated`]), by object, with their status (RR-3). A
    /// retired instance stays, so it is never spawned again.
    roles: BTreeMap<usize, BTreeMap<ObjectReference, RoleStatus>>,
    threads: BTreeMap<ThreadPath, Thread>,
    parallels: BTreeMap<ParallelKey, Parallel>,
    /// Every live thread's binder environment (ADR-027 PS-5), by the thread
    /// that holds it, then binder node: each thread reads its own entry, and
    /// holds its own copy of every value it reads.
    environments: BTreeMap<ThreadPath, BTreeMap<qsl_semantics::check::ProtocolNodeId, Bound>>,
    /// Each `repeat` counter, by (repeat node, thread) (ADR-027 PS-6).
    counters: BTreeMap<(qsl_semantics::check::ProtocolNodeId, ThreadPath), u64>,
    /// Each non-empty queue, a `fifo` one in delivery order and an
    /// `unordered` one in ascending JCS bytes of its entries (PS-7).
    queues: BTreeMap<QueueKey, Vec<Entry>>,
    /// Each compensation registration (CP-1).
    registrations: BTreeMap<Registered, Registration>,
    /// Each compensation thread, by its path (CP-3).
    compensating: BTreeMap<ThreadPath, Compensating>,
    /// Each template commit node a step has occurred at (CP-4).
    committed: std::collections::BTreeSet<qsl_semantics::check::ProtocolNodeId>,
}

impl Instance {
    /// The live instances of the replicated role `index` (ADR-027 RR-3), in
    /// object order.
    fn live_of(&self, index: usize) -> impl Iterator<Item = &ObjectReference> {
        self.roles
            .get(&index)
            .into_iter()
            .flatten()
            .filter(|(_, status)| **status == RoleStatus::Live)
            .map(|(object, _)| object)
    }

    /// The value the thread `reader` reads for the binder of `node`: its own
    /// environment's entry (ADR-027 PS-5).
    fn entry(
        &self,
        node: qsl_semantics::check::ProtocolNodeId,
        reader: &ThreadPath,
    ) -> Option<&Bound> {
        self.environments.get(reader)?.get(&node)
    }

    /// The entries of the thread `holder`'s environment.
    fn environment<'i>(
        &'i self,
        holder: &ThreadPath,
    ) -> impl Iterator<Item = (qsl_semantics::check::ProtocolNodeId, &'i Bound)> + 'i {
        self.environments
            .get(holder)
            .into_iter()
            .flat_map(|environment| environment.iter().map(|(node, bound)| (*node, bound)))
    }
}

/// A protocol state (ADR-027 PS-8).
pub struct ProtocolState<M: MemoryModel> {
    model: ModelSimState,
    instances: BTreeMap<u64, Instance>,
    memory: M::Component,
}

impl<M: MemoryModel> Clone for ProtocolState<M> {
    fn clone(&self) -> Self {
        Self {
            model: self.model.clone(),
            instances: self.instances.clone(),
            memory: self.memory.clone(),
        }
    }
}

impl<M: MemoryModel> std::fmt::Debug for ProtocolState<M> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProtocolState")
            .field("model", &self.model)
            .field("instances", &self.instances)
            .field("memory", &self.memory)
            .finish()
    }
}

impl<M: MemoryModel> ProtocolState<M> {
    /// The model state.
    pub fn model(&self) -> &ModelSimState {
        &self.model
    }

    /// The memory component.
    pub fn memory(&self) -> &M::Component {
        &self.memory
    }

    /// Whether some protocol instance is live (ADR-027 PD-1).
    pub fn has_live_instance(&self) -> bool {
        !self.instances.is_empty()
    }
}

/// One live thread as a reader sees it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThreadView {
    /// Its path.
    pub path: ThreadPath,
    /// Its rest point.
    pub rest_point: RestPoint,
    /// Its disposition.
    pub disposition: Disposition,
}

/// QSpec FR-425's state key: FR-181's `simulation-state` envelope with the
/// protocol members and the memory model's component (ADR-027 PS-8).
pub struct ProtocolKey<C> {
    members: Vec<(&'static str, Enc)>,
    memory: C,
}

impl<C: Encode> Encode for ProtocolKey<C> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        writer.begin_object()?;
        for (name, value) in &self.members {
            writer.name(name)?;
            value.encode_into(writer)?;
        }
        writer.name("memory")?;
        self.memory.encode_into(writer)?;
        writer.end_object()
    }
}

/// FR-215 "Model checking a protocol subject" (ADR-027 TS-5): the roots of
/// `protocol` beside its model's: the record type of every binder of an
/// `event`, `send`, `receive` and `finish` step, each named by the
/// protocol and the binder. A bounded record type this module cannot
/// enumerate is refused when the system is built
/// ([`Unsupported::UnenumerableRecord`]).
pub fn protocol_domains(protocol: &CheckedProtocol) -> Vec<(WireNodeId, ValueType)> {
    protocol
        .slots
        .binders
        .iter()
        .filter(|binder| binder.ranged)
        .map(|binder| {
            (
                binder_root(&protocol.name, &binder.name),
                binder.value_type.clone(),
            )
        })
        .collect()
}

/// The root a protocol binder's record type is named by: the digest of
/// `{"protocol": <name>, "binder": <name>}`.
pub fn binder_root(protocol: &str, binder: &str) -> WireNodeId {
    let naming = Enc::Object(vec![
        ("protocol", Enc::text(protocol)),
        ("binder", Enc::text(binder)),
    ]);
    let digest = quire_canonical::sha256(&naming, CanonicalLimits::new(u64::MAX))
        .map_or([0; 32], |digest| *digest.as_bytes());
    WireNodeId::from_digest(digest)
}

/// FR-215 (ADR-027 TS-5): FR-101's requires-bound pre-check over the roots
/// of `model` and of `protocol`, with no state explored.
///
/// # Errors
///
/// [`NotSimulated::RequiresBound`] when a root is unbounded;
/// [`NotSimulated::Extent`] when classification stops.
pub fn check_protocol_bounds(
    model: &ModelSystem<'_>,
    protocol: &CheckedProtocol,
    position_limit: u64,
) -> Result<(), NotSimulated> {
    let domains = subject_domains(model, protocol);
    check_requires_bound(&roots(&domains), model.types(), position_limit)
}

/// The roots of `model` and of `protocol`.
fn subject_domains(
    model: &ModelSystem<'_>,
    protocol: &CheckedProtocol,
) -> Vec<(WireNodeId, ValueType)> {
    let mut domains = model.domains();
    domains.extend(protocol_domains(protocol));
    domains
}

/// `domains` as the requires-bound pre-check reads them.
fn roots(domains: &[(WireNodeId, ValueType)]) -> Vec<(WireNodeId, &ValueType)> {
    domains
        .iter()
        .map(|(id, value_type)| (*id, value_type))
        .collect()
}

/// FR-101's `explore_request` over `system`, the roots of its model and of
/// its protocol (ADR-027 TS-5) and the package's `TypeEnvironment`.
///
/// # Errors
///
/// [`NotSimulated`] as `explore_request` returns it, before any state is
/// expanded when a root is unbounded.
pub fn explore_protocol<M: MemoryModel>(
    system: &ProtocolSystem<'_, M>,
    position_limit: u64,
    limits: crate::simulation::explore::Limits,
    poll: impl FnMut() -> bool,
) -> Result<crate::simulation::explore::Exploration<ModelFinding>, NotSimulated> {
    let domains = subject_domains(&system.model, system.protocol);
    crate::simulation::explore::explore_request(
        system,
        &roots(&domains),
        system.model.types(),
        position_limit,
        limits,
        identity_limits(),
        None,
        poll,
    )
}

/// FR-220 "Sharing and classification" (ADR-025 MA-1, MA-2): EN-1's
/// sharing and classification of every attempt of each `parallel` `memory`
/// resolves weak, over the initial states of `model`. It reads no memory
/// model's rules, so it classifies under `tso` and `ra` alike.
///
/// # Errors
///
/// [`ProtocolRefusal`] when `memory` does not resolve the protocol's
/// outermost `parallel` controls, for a construct this system does not
/// step, or when an application has no footprint.
pub fn classify_accesses(
    model: &ModelSystem<'_>,
    protocol: &CheckedProtocol,
    memory: &ResolvedMemory,
) -> Result<WeakAccesses, ProtocolRefusal> {
    let governing = memory
        .governing(protocol)
        .map_err(ProtocolRefusal::Memory)?;
    let program = Program::new(protocol)?;
    access::classify(
        model,
        protocol,
        &program,
        &governing,
        model.initial().first(),
    )
}

/// FR-215: the transition system of a protocol subject.
pub struct ProtocolSystem<'a, M: MemoryModel> {
    model: ModelSystem<'a>,
    package: &'a CheckedPackage,
    protocol: &'a CheckedProtocol,
    program: Program<'a>,
    memory: M,
    /// The model governing each `parallel`: its outermost `parallel`'s
    /// resolved model (FR-219, ADR-025 MM-3).
    governing: BTreeMap<qsl_semantics::check::ProtocolNodeId, qsl_foundation::MemoryModelName>,
    /// EN-1's classification of the attempts of every weak `parallel`
    /// (FR-220).
    weak: WeakAccesses,
    /// The `over` parameter's bound value.
    over: Value,
    /// Each static role's binding: role name, then model identity.
    roles: Vec<(String, String)>,
    /// The objects of each replicated role's population in the subject's
    /// universe (ADR-027 RR-1), by the role's index in the program.
    universes: Vec<Vec<ObjectReference>>,
    /// The instance bound of `activation on each` (ADR-027 AE-4).
    max_live_instances: u64,
    initial: Vec<ProtocolState<M>>,
}

/// ADR-027 AE-4: the published default of `max_live_instances`.
pub const DEFAULT_MAX_LIVE_INSTANCES: u64 = 3;

/// Whether `protocol` activates `on each`, so that a run over it is bound by
/// `max_live_instances` and every verdict states that bound (ADR-027 AE-4).
pub fn activates_on_each(protocol: &CheckedProtocol) -> bool {
    matches!(
        protocol.obligations.activation,
        CheckedActivation::OnEach { .. }
    )
}

/// The `max_live_instances` a request that sets `requested` runs with
/// (ADR-027 AE-4): the request's value, else the published default.
pub const fn live_instance_bound(requested: Option<u64>) -> u64 {
    match requested {
        Some(bound) => bound,
        None => DEFAULT_MAX_LIVE_INSTANCES,
    }
}

impl<'a, M: MemoryModel> ProtocolSystem<'a, M> {
    /// FR-205 "Subject": build the protocol subject's system over the
    /// admitted `model` of the subject's model subject, with the memory
    /// model `memory` for its resolved models (FR-219), and its initial
    /// protocol states (FO-3). EN-1's pre-check runs first: a weak
    /// `parallel` whose join is not `all` with no `outstanding` clause, and
    /// an attempt instance that is not one access under its weak model,
    /// refuse with [`ProtocolRefusal::WeakAccessShape`] (FR-220 "Access
    /// shape").
    ///
    /// # Errors
    ///
    /// [`ProtocolRefusal`] as FR-205 "Subject" states, for a resolved model
    /// `memory` does not implement, for a weak access shape, or for a
    /// construct this system does not step.
    pub fn new(
        model: ModelSystem<'a>,
        subject: &ProtocolSubject<'a>,
        memory: M,
    ) -> Result<Self, ProtocolRefusal> {
        let package = subject.model.simulation.package;
        let protocol = package.graph().protocol(&subject.protocol).ok_or_else(|| {
            ProtocolRefusal::UnknownProtocol {
                protocol: subject.protocol.clone(),
            }
        })?;
        let mut roles = Vec::new();
        for role in protocol.roles.iter().filter(|role| !role.replicated) {
            let binding = subject
                .roles
                .iter()
                .find(|binding| binding.role == role.name)
                .ok_or_else(|| ProtocolRefusal::RoleMapping {
                    role: role.name.clone(),
                })?;
            let admits = role
                .context
                .and_then(|context| model.declaring_package(context))
                .is_some_and(|declaring| declaring == binding.model);
            if !admits {
                return Err(ProtocolRefusal::RoleMapping {
                    role: role.name.clone(),
                });
            }
            roles.push((role.name.clone(), binding.model.clone()));
        }
        let over = Self::over_value(&model, protocol, &subject.over)?;
        let governing = subject
            .memory
            .governing(protocol)
            .map_err(ProtocolRefusal::Memory)?;
        let implemented = memory.models();
        if governing
            .values()
            .any(|resolved| !implemented.contains(resolved))
        {
            return Err(ProtocolRefusal::Unsupported(Unsupported::WeakMemory));
        }
        if let Some(node) = selection::weak_join(protocol, &governing) {
            let site = protocol
                .controls
                .controls
                .iter()
                .find(|control| control.node == node)
                .map(|control| control.path.join("/"))
                .ok_or_else(|| ProtocolRefusal::fault("a-parallel-is-a-control"))?;
            return Err(ProtocolRefusal::WeakAccessShape(Box::new(
                WeakAccessShape::join_policy(site),
            )));
        }
        let program = Program::new(protocol)?;
        let weak = access::classify(
            &model,
            protocol,
            &program,
            &governing,
            model.initial().first(),
        )?;
        if let Some(shape) = weak.first_shape() {
            return Err(ProtocolRefusal::WeakAccessShape(Box::new(shape)));
        }
        let mut system = Self {
            model,
            package,
            protocol,
            program,
            memory,
            governing,
            weak,
            over,
            roles,
            universes: Vec::new(),
            max_live_instances: DEFAULT_MAX_LIVE_INSTANCES,
            initial: Vec::new(),
        };
        system.universes = system.role_universes()?;
        system.initial = system.initial_states()?;
        Ok(system)
    }

    /// This system with `bound` as `max_live_instances` (ADR-027 AE-4), a
    /// bound on the explored state space and no part of the subject. An
    /// `activate` step is not expanded at a state with that many live
    /// instances.
    #[must_use]
    pub fn with_max_live_instances(mut self, bound: u64) -> Self {
        self.max_live_instances = bound;
        self
    }

    /// The objects of each replicated role's population (ADR-027 RR-1): the
    /// objects of its object type in the subject's universe, in reference
    /// order.
    fn role_universes(&self) -> Result<Vec<Vec<ObjectReference>>, ProtocolRefusal> {
        self.program
            .replicated
            .iter()
            .map(|role| {
                let refused = || ProtocolRefusal::RolePopulation {
                    role: role.name.clone(),
                };
                let instances = self
                    .model
                    .instances(&ValueType::Reference(role.object))
                    .map_err(|_| refused())?;
                let mut objects = instances
                    .iter()
                    .map(|instance| self.model.bound_reference(role.object, instance))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| refused())?;
                objects.sort();
                objects.dedup();
                Ok(objects)
            })
            .collect()
    }

    /// The `over` parameter's value: the reference to `over`'s key in its
    /// declared type's universe.
    fn over_value(
        model: &ModelSystem<'_>,
        protocol: &CheckedProtocol,
        over: &ModelInstance,
    ) -> Result<Value, ProtocolRefusal> {
        let input = protocol
            .slots
            .binders
            .first()
            .filter(|binder| binder.node.is_none())
            .ok_or_else(|| ProtocolRefusal::fault("a-protocol-has-an-over-input"))?;
        let refused = || ProtocolRefusal::OverBinding {
            parameter: input.name.clone(),
            object: over.key.clone(),
        };
        let ValueType::Reference(object_type) = input.value_type else {
            return Err(ProtocolRefusal::Unsupported(Unsupported::OverNotAnObject));
        };
        let instances = model.instances(&input.value_type).map_err(|_| refused())?;
        if !instances.contains(over) {
            return Err(refused());
        }
        model
            .bound_reference(object_type, over)
            .map(Value::Reference)
            .map_err(|_| refused())
    }

    /// FO-3: for each initial model state, instance 0 with its root thread
    /// settled from the start of `run`; under `activation on each`, no
    /// instance (ADR-027 AE-1).
    fn initial_states(&self) -> Result<Vec<ProtocolState<M>>, ProtocolRefusal> {
        let mut states = Vec::new();
        for model in self.model.initial() {
            let instances = if self.program.activation.is_some() {
                BTreeMap::new()
            } else {
                let started = self.start(0, None, &model).map_err(|error| match error {
                    settle::SettleError::Refused(refusal) => refusal,
                    settle::SettleError::Stopped(stop) => ProtocolRefusal::InitialStop(stop),
                })?;
                match started {
                    roles::Started::Instance(instance) => BTreeMap::from([(0, *instance)]),
                    roles::Started::Undecided(_) => {
                        return Err(ProtocolRefusal::Unsupported(
                            Unsupported::UndecidedInitialGuard,
                        ))
                    }
                }
            };
            states.push(ProtocolState {
                model,
                instances,
                memory: self.memory.initial(),
            });
        }
        Ok(states)
    }

    /// The subject's admitted model system.
    pub fn model(&self) -> &ModelSystem<'a> {
        &self.model
    }

    /// The checked protocol clause.
    pub fn protocol(&self) -> &'a CheckedProtocol {
        self.protocol
    }

    /// FR-212 (ADR-027 PA-3, QSpec FR-430): the fairness set of an
    /// infinite-trace clause whose authored constraints are `authored`
    /// over this subject: the authored members and, unless the protocol
    /// declares `scheduling adversarial`, its scheduler constraints, in
    /// ascending JCS bytes.
    ///
    /// # Errors
    ///
    /// [`ProtocolFairnessError`] when an authored constraint names an
    /// operation the subject's model does not apply.
    pub fn fairness_set(
        &self,
        authored: &[quire_semantic_value::fairness::FairnessConstraint],
    ) -> Result<Vec<ProtocolFairness>, ProtocolFairnessError> {
        fairness::fairness_set(self.protocol, authored, |constraint| {
            let operation = constraint.operation();
            self.model
                .operation_text(operation.declaring_type(), operation.name())
        })
    }

    /// The memory model it runs under.
    pub fn memory(&self) -> &M {
        &self.memory
    }

    /// EN-1's sharing and classification of the attempts of every weak
    /// `parallel` (FR-220).
    pub fn weak_accesses(&self) -> &WeakAccesses {
        &self.weak
    }

    /// The live threads of `state`, by instance and path.
    ///
    /// # Errors
    ///
    /// [`ProtocolRefusal::Fault`] when a thread rests at a node the protocol
    /// does not name.
    pub fn threads(&self, state: &ProtocolState<M>) -> Result<Vec<ThreadView>, ProtocolRefusal> {
        let mut views = Vec::new();
        for instance in state.instances.values() {
            for (path, thread) in &instance.threads {
                views.push(ThreadView {
                    path: path.clone(),
                    rest_point: self.rest_point(thread.rest)?,
                    disposition: thread.disposition,
                });
            }
        }
        Ok(views)
    }

    /// The binder environments of `state` (ADR-027 PS-5): each entry's
    /// binder node and the thread whose environment holds it.
    ///
    /// # Errors
    ///
    /// [`ProtocolRefusal::Fault`] when a binder's node is not one the
    /// protocol names.
    pub fn binders(
        &self,
        state: &ProtocolState<M>,
    ) -> Result<Vec<(String, ThreadPath)>, ProtocolRefusal> {
        let mut binders = Vec::new();
        for (thread, environment) in state
            .instances
            .values()
            .flat_map(|instance| &instance.environments)
        {
            for node in environment.keys() {
                binders.push((self.program.text(*node)?.to_owned(), thread.clone()));
            }
        }
        Ok(binders)
    }

    /// QSpec FR-425's `<rest>` of `rest`.
    fn rest_point(&self, rest: Rest) -> Result<RestPoint, ProtocolRefusal> {
        let named = |kind, node| -> Result<RestPoint, ProtocolRefusal> {
            Ok(RestPoint {
                kind,
                node: Some(self.program.text(node)?.to_owned()),
            })
        };
        match rest {
            Rest::Node(node) | Rest::Await(node) | Rest::Check(node) => named(RestKind::Node, node),
            Rest::Fork(node) => named(RestKind::Fork, node),
            Rest::Join(node) => named(RestKind::Join, node),
            Rest::Finish => named(RestKind::Node, self.program.finish.node),
            Rest::Compensate(template) => Ok(RestPoint {
                kind: RestKind::Node,
                node: Some(self.program.template(template)?.name.clone()),
            }),
            Rest::Retry(template) => Ok(RestPoint {
                kind: RestKind::RetryPoint,
                node: Some(self.program.template(template)?.name.clone()),
            }),
            Rest::End => Ok(RestPoint {
                kind: RestKind::End,
                node: None,
            }),
        }
    }

    /// The canonical key of the registration `key` (QSpec FR-425).
    fn registration_key(&self, key: &Registered) -> Result<RegistrationKey, ProtocolRefusal> {
        Ok(RegistrationKey {
            template: self.program.template(key.template)?.name.clone(),
            thread: key.thread.clone(),
            ordinal: key.ordinal,
        })
    }

    /// ADR-027 PS-8: the key members of `state`, the memory component
    /// apart. A template's commit node that a step has occurred at is a
    /// `control` entry `{"type":"commit","instance","node"}`, with value
    /// `{"type":"commit-state","occurred":true}`: CP-1 and CP-4 read it, and
    /// PS-8 lists no member for it. A compensation thread's `thread-state`
    /// also names the registration it compensates, as `registration`: which
    /// registration each thread's `cattempt` and `cend` act on decides its
    /// successors, and PS-8 lists no member for that mapping either.
    fn key_members(&self, state: &ProtocolState<M>) -> Vec<(&'static str, Enc)> {
        let text = |value: &str| Enc::tagged("text", vec![("value", Enc::text(value))]);
        let mut control = Vec::new();
        let mut observations = Vec::new();
        let mut roles = Vec::new();
        let mut queues = Vec::new();
        let mut bounds = Vec::new();
        for (ordinal, instance) in &state.instances {
            control.push((
                Enc::tagged(
                    "protocol-instance",
                    vec![("ordinal", Enc::text(ordinal.to_string()))],
                ),
                Enc::tagged("instance-state", vec![("finished", Enc::Bool(false))]),
            ));
            for (path, thread) in &instance.threads {
                let rest = self
                    .rest_point(thread.rest)
                    .map_or(Enc::Unencodable("a rest point"), |rest| rest.enc());
                let mut members = vec![
                    ("rest", rest),
                    ("disposition", Enc::text(thread.disposition.text())),
                ];
                // A compensation thread's registration decides its steps,
                // so its thread state names it (see `key_members`).
                if let Some(compensating) = instance.compensating.get(path) {
                    members.push((
                        "registration",
                        self.registration_key(&compensating.registration)
                            .map_or(Enc::Unencodable("a registration"), |key| key.enc()),
                    ));
                }
                control.push((path.enc(), Enc::tagged("thread-state", members)));
            }
            for (key, parallel) in &instance.parallels {
                let node = self
                    .program
                    .text(key.node)
                    .map_or(Enc::Unencodable("a parallel node"), Enc::text);
                control.push((
                    Enc::tagged(
                        "parallel-instance",
                        vec![
                            ("parent", key.parent.enc()),
                            ("node", node),
                            ("ordinal", Enc::text(key.ordinal.to_string())),
                        ],
                    ),
                    Enc::tagged(
                        "parallel-state",
                        vec![
                            ("settled", Enc::Bool(parallel.settled)),
                            (
                                "dispositions",
                                map_enc(
                                    parallel
                                        .dispositions
                                        .iter()
                                        .map(|(label, disposition)| {
                                            (text(label), text(disposition.text()))
                                        })
                                        .collect(),
                                ),
                            ),
                        ],
                    ),
                ));
            }
            for (thread, node, bound) in
                instance
                    .environments
                    .iter()
                    .flat_map(|(thread, environment)| {
                        environment
                            .iter()
                            .map(move |(node, bound)| (thread, node, bound))
                    })
            {
                let node = self
                    .program
                    .text(*node)
                    .map_or(Enc::Unencodable("a binder node"), Enc::text);
                observations.push((
                    Enc::tagged("binder", vec![("node", node), ("thread", thread.enc())]),
                    bound.enc.clone(),
                ));
            }
            for ((node, thread), counter) in &instance.counters {
                let node = self
                    .program
                    .text(*node)
                    .map_or(Enc::Unencodable("a repeat node"), Enc::text);
                bounds.push((
                    Enc::tagged(
                        "repeat-counter",
                        vec![("node", node), ("thread", thread.enc())],
                    ),
                    Enc::tagged("integer", vec![("value", Enc::text(counter.to_string()))]),
                ));
            }
            if let (true, Some(trigger)) = (self.program.trigger_read, &instance.trigger) {
                observations.push((
                    Enc::tagged(
                        "activation",
                        vec![("instance", Enc::text(ordinal.to_string()))],
                    ),
                    trigger.enc.clone(),
                ));
            }
            for (key, registration) in &instance.registrations {
                let entry = self
                    .registration_key(key)
                    .and_then(|named| Ok((named, self.program.template(key.template)?)));
                let Ok((named, template)) = entry else {
                    control.push((Enc::Unencodable("a registration"), Enc::Null));
                    continue;
                };
                let names = template
                    .registration
                    .iter()
                    .chain(&template.activation)
                    .map(|(name, _)| name);
                let captures = names
                    .zip(&registration.captures)
                    .map(|(name, bound)| (text(name), bound.enc.clone()))
                    .collect();
                control.push((
                    named.enc(),
                    Enc::tagged(
                        "registration-state",
                        vec![
                            ("status", Enc::text(registration.status.text())),
                            ("captures", map_enc(captures)),
                        ],
                    ),
                ));
            }
            for node in &instance.committed {
                let node = self
                    .program
                    .text(*node)
                    .map_or(Enc::Unencodable("a commit node"), Enc::text);
                control.push((
                    Enc::tagged(
                        "commit",
                        vec![("instance", Enc::text(ordinal.to_string())), ("node", node)],
                    ),
                    Enc::tagged("commit-state", vec![("occurred", Enc::Bool(true))]),
                ));
            }
            for (thread, compensating) in &instance.compensating {
                bounds.push((
                    Enc::tagged("attempt-count", vec![("thread", thread.enc())]),
                    Enc::tagged(
                        "integer",
                        vec![("value", Enc::text(compensating.count.to_string()))],
                    ),
                ));
            }
            for (key, entries) in &instance.queues {
                let Some(channel) = self.program.channels.get(key.channel) else {
                    queues.push((Enc::Unencodable("a queue's channel"), Enc::Null));
                    continue;
                };
                let (kind, fifo) = match &key.fifo {
                    Some(fifo) => ("sequence", fifo.enc.clone()),
                    None => ("bag", Enc::Null),
                };
                queues.push((
                    Enc::tagged(
                        "queue",
                        vec![
                            ("instance", Enc::text(ordinal.to_string())),
                            ("channel", Enc::text(channel.name.clone())),
                            ("fifo_key", fifo),
                        ],
                    ),
                    Enc::tagged(
                        kind,
                        vec![(
                            "elements",
                            Enc::Array(
                                entries
                                    .iter()
                                    .map(|entry| match &entry.carried {
                                        Some(carried) => Enc::Object(vec![
                                            ("entry", entry.enc.clone()),
                                            (
                                                "view",
                                                self.memory.carried_key(&state.memory, carried).0,
                                            ),
                                        ]),
                                        None => entry.enc.clone(),
                                    })
                                    .collect(),
                            ),
                        )],
                    ),
                ));
            }
            for (index, replicated) in self.program.replicated.iter().enumerate() {
                let instances = instance
                    .roles
                    .get(&index)
                    .into_iter()
                    .flatten()
                    .map(|(object, status)| {
                        (
                            reference_enc(object),
                            Enc::tagged("text", vec![("value", Enc::text(status.text()))]),
                        )
                    })
                    .collect();
                roles.push((
                    Enc::tagged(
                        "role",
                        vec![
                            ("instance", Enc::text(ordinal.to_string())),
                            ("role", Enc::text(replicated.name.clone())),
                        ],
                    ),
                    map_enc(instances),
                ));
            }
            for (role, model) in &self.roles {
                roles.push((
                    Enc::tagged(
                        "role",
                        vec![
                            ("instance", Enc::text(ordinal.to_string())),
                            ("role", Enc::text(role.clone())),
                        ],
                    ),
                    text(model),
                ));
            }
        }
        vec![
            ("type", Enc::text("simulation-state")),
            ("semantic", self.model.semantic_enc(&state.model)),
            ("control", map_enc(control)),
            ("queues", map_enc(queues)),
            ("roles", map_enc(roles)),
            ("observations", map_enc(observations)),
            ("bounds", map_enc(bounds)),
        ]
    }

    /// The value of each binder slot the node `reader` of the thread `path`
    /// reads: slot 0, the `over` input; a binder of a node, the entry of
    /// `path`'s own environment (ADR-027 PS-5). A slot `reader` does not
    /// read is `None`.
    fn frame(
        &self,
        instance: &Instance,
        path: &ThreadPath,
        reader: qsl_semantics::check::ProtocolNodeId,
    ) -> Vec<Option<Value>> {
        let mut frame = Vec::with_capacity(self.program.slots.len());
        for (slot, node) in self.program.slots.iter().enumerate() {
            let value = match node {
                None if slot == 0 => Some(self.over.clone()),
                None if Some(slot) == self.program.activation_slot => instance
                    .trigger
                    .as_ref()
                    .filter(|_| self.program.reads(slot, reader))
                    .and_then(|bound| bound.value.clone()),
                // Refused in `Program::new` when any slot reads one.
                None => None,
                Some(_) if !self.program.reads(slot, reader) => None,
                Some(node) => instance
                    .entry(*node, path)
                    .and_then(|bound| bound.value.clone()),
            };
            frame.push(value);
        }
        frame
    }

    /// One slot body at the observation `observed`, over `frame`.
    fn evaluate(
        &self,
        body: &qsl_semantics::check::CheckedBody,
        frame: Vec<Option<Value>>,
        observed: &ModelSimState,
    ) -> Result<Verdict, ExpansionStop> {
        let observation = self.model.observation_of(observed)?;
        let evaluation = evaluate_protocol_slot(
            self.package,
            body,
            frame,
            &observation.environment,
            &mut Meter::new(self.model.meter()),
        )
        .map_err(|_| broken())?;
        Ok(verdict(&evaluation.outcome))
    }

    /// FR-211 and ADR-027 PD-1: the class of `state`. A state with no live
    /// instance is intended; otherwise the protocol's `terminal` member
    /// decides, read through the one clause evaluator.
    fn classify(
        &self,
        state: &ProtocolState<M>,
        expansion: &ProtocolExpansion<M>,
        cut: Cut,
    ) -> Result<TerminalClass, ExpansionStop> {
        let has_successor = !expansion.successors.is_empty();
        if self.program.activation.is_some() {
            // ADR-027 PB-4, PD-2, PD-5: under `on each`, a state at the
            // instance bound is never terminal; a state with a live instance
            // whose every enabled step is an `activate` is terminal, since no
            // instance can move until a trigger arrives.
            if cut.instances {
                return Ok(TerminalClass::NotTerminal);
            }
            let only_activations = expansion
                .successors
                .iter()
                .all(|(transition, _)| transition.kind() == StepKind::Activate);
            if has_successor && (state.instances.is_empty() || !only_activations) {
                return Ok(TerminalClass::NotTerminal);
            }
        } else if has_successor {
            return Ok(TerminalClass::NotTerminal);
        }
        if state.instances.is_empty() {
            return Ok(TerminalClass::Intended);
        }
        match self.program.terminal {
            CheckedTerminalMember::None => Ok(TerminalClass::Deadlocked),
            CheckedTerminalMember::Any => Ok(TerminalClass::Intended),
            CheckedTerminalMember::When(predicate) => {
                let mut frame = vec![None; self.program.slots.len()];
                if let Some(first) = frame.first_mut() {
                    *first = Some(self.over.clone());
                }
                let observed = self.memory.atom_values(&state.memory, &state.model);
                Ok(match self.evaluate(predicate, frame, &observed)? {
                    Verdict::True => TerminalClass::Intended,
                    Verdict::False => TerminalClass::Deadlocked,
                    Verdict::Undecided(outcome) => {
                        if let Some(cause) = outcome.undefined_cause() {
                            TerminalClass::Undefined {
                                model: self.protocol.name.clone(),
                                cause,
                            }
                        } else if let Some(cause) = outcome.incomplete_code() {
                            return Err(ExpansionStop {
                                cause,
                                limit: Some(ExpansionLimit::EvaluationMeter),
                            });
                        } else {
                            TerminalClass::Undecided {
                                model: self.protocol.name.clone(),
                            }
                        }
                    }
                })
            }
        }
    }
}

/// `runtime_invariant`/`established-invariant-broken`.
fn broken() -> ExpansionStop {
    ExpansionStop {
        cause: BROKEN,
        limit: None,
    }
}

/// The stop a refusal of the checked protocol is during an expansion: an
/// established invariant broke.
impl From<ProtocolRefusal> for ExpansionStop {
    fn from(_: ProtocolRefusal) -> Self {
        broken()
    }
}

impl<'a, M: MemoryModel> TransitionSystem for ProtocolSystem<'a, M> {
    type State = ProtocolState<M>;
    type TransitionId = ProtocolTransition;
    type Key = ProtocolKey<M::Component>;
    type Finding = ModelFinding;

    fn initial(&self) -> Vec<Self::State> {
        self.initial.clone()
    }

    fn key(&self, state: &Self::State) -> Self::Key {
        ProtocolKey {
            members: self.key_members(state),
            memory: state.memory.clone(),
        }
    }

    fn successors(
        &self,
        state: &Self::State,
    ) -> Result<Expansion<Self::TransitionId, Self::State, Self::Finding>, ExpansionStop> {
        Ok(self.expand_bounded(state)?.0)
    }
}

impl<M: MemoryModel> SubjectSystem for ProtocolSystem<'_, M> {
    fn state_key(&self, state: &Self::State) -> Result<(Vec<u8>, DigestRecord), EncodingRefusal> {
        let (key, digest) = key::state_key(&self.key(state), identity_limits())?;
        Ok((key.as_bytes().to_vec(), digest))
    }

    fn transition_key(&self, transition: &ProtocolTransition) -> Result<Vec<u8>, EncodingRefusal> {
        key::canonical_bytes(transition, identity_limits())
    }

    fn terminal_class(
        &self,
        state: &Self::State,
        expansion: &ProtocolExpansion<M>,
        cut: Cut,
    ) -> Result<TerminalClass, ExpansionStop> {
        self.classify(state, expansion, cut)
    }

    fn counts_step(&self, transition: &ProtocolTransition) -> bool {
        transition.counts()
    }

    fn progress_constraints(
        &self,
        authored: &[quire_semantic_value::fairness::FairnessConstraint],
        transitions: &[ProtocolTransition],
        identity: &dyn Fn(&ProtocolTransition) -> Result<Identity, ZeroProgressError>,
    ) -> Result<Vec<Constraint>, ZeroProgressError> {
        let members = self.fairness_set(authored)?;
        Ok(protocol_constraints(&members, transitions, identity)?
            .into_iter()
            .map(|(_, constraint)| constraint)
            .collect())
    }

    fn atom_state<'s>(&self, state: &'s Self::State) -> Cow<'s, ModelSimState> {
        self.memory.atom_values(&state.memory, &state.model)
    }

    fn blocked(
        &self,
        state: &Self::State,
        prefix: &[&ProtocolTransition],
    ) -> Result<Vec<BlockedThread>, ExpansionStop> {
        self.blocked_threads(state, prefix)
    }

    fn successors_bounded(
        &self,
        state: &Self::State,
    ) -> Result<Bounded<Self::TransitionId, Self::State>, ExpansionStop> {
        self.expand_bounded(state)
    }

    /// Over a weak `parallel`, `transition` carrying `post`'s memory
    /// component (ADR-025 MK-1).
    fn reported(
        &self,
        transition: ProtocolTransition,
        post: &Self::State,
    ) -> Result<ProtocolTransition, ExpansionStop> {
        if self.governing.values().any(|model| model.is_weak()) {
            Ok(transition.with_memory(CarriedComponent::of(&post.memory)?))
        } else {
            Ok(transition)
        }
    }
}

/// The identity limits exploration encodes under.
fn identity_limits() -> qsl_foundation::IdentityLimits {
    qsl_foundation::IdentityLimits::default()
}

/// FR-120 "Contract clauses": an undecided protocol slot as data.
fn undetermined(node: &str, outcome: crate::simulation::model::ClauseOutcome) -> ModelFinding {
    ModelFinding::ProtocolExpressionUndetermined {
        node: node.to_owned(),
        evaluation: outcome,
    }
}
