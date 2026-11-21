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
mod capacity;
mod channel;
mod compensation;
mod cycle;
mod fairness;
mod footprint;
mod identity;
mod memory;
mod package_program;
mod program;
mod ra;
mod race;
mod reach;
mod roles;
mod selection;
mod settle;
mod steps;
mod tso;
mod wire;

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use filament_eval::{FamilyOutcome, FamilyResult};
use filament_exact::{Integer, Meter, NodeKey, ObjectReference, Outcome, Value, ValueType};
use filament_semantic_value::catalog::{CatalogCode, InternalFault};
use fsl_foundation::bound::{DomainKey, DomainKind};
use fsl_foundation::diagnostic::Code;
use fsl_semantics::check::{CheckedActivation, CheckedProtocol};
use fsl_semantics::family::ClaimExtent;
use quire_canonical::{Encode, Error, Sink, Writer};

use crate::fairness::{Constraint, Identity};
use crate::model_subject::{CapacityBinding, ModelInstance, ProtocolSubject};
use crate::simulation::expansion::{Expansion, ExpansionLimit, ExpansionStop};
use crate::simulation::explore::TransitionSystem;
use crate::simulation::footprint::Location;
use crate::simulation::key::{self, EncodingRefusal};
use crate::simulation::model::encode::{
    self, map_enc, reference_enc, value_enc, Canonicalizer, Enc,
};
use crate::simulation::model::{
    verdict, ClauseOutcome, ModelEvaluator, ModelFinding, ModelSimState, ModelSystem, Observed,
    TerminalClass, Verdict, BROKEN,
};
use crate::simulation::not_simulated::{check_requires_bound, NotSimulated, RequiresBound};
use crate::simulation::progress::ZeroProgressError;
use crate::simulation::subject::ConstraintError;
use crate::simulation::subject::{Bounded, Cut, SubjectSystem};
use fsl_semantics::model::observation::Observation;
use steps::ProtocolExpansion;

pub use access::{
    AccessShape, AttemptInstance, InstanceAccess, LoadBufferingShape, WeakAccessFailure,
    WeakAccessShape, WeakAccesses, WeakAttempt,
};
pub use capacity::{capacity_value, Capacities, CapacityRefusal, UnboundCapacity};
pub use fairness::{
    protocol_constraints, FairnessOrigin, ProtocolClass, ProtocolFairness, ProtocolFairnessError,
    ProtocolTarget,
};
pub use footprint::{
    visible, AnchorName, ClaimReads, ParallelInstance, ProtocolFootprintError, ProtocolLocation,
};
pub use identity::{
    Actor, BlockedThread, BusinessEffectOccurrence, CarriedComponent, DeliveryIdentity,
    MessageOccurrence, PathStep, PositionAnchor, ProtocolRecommendation, ProtocolTransition,
    QueueFlow, RegistrationKey, RestKind, RestPoint, SendOutcome, SendRecord, StepKind, ThreadPath,
    WaitCause,
};
pub use memory::{
    Access, CarriedKey, Chosen, FootprintStep, GatedStep, MemoryBranch, MemoryChoice, MemoryModel,
    MemoryStep, MemoryStepId, Read, ReadChoice, Sc, ScComponent, SharedCell, Written,
};
pub use package_program::{
    PackageAttemptContext, PackageControl, PackageProgramRefusal, PackageProtocolProgram,
    PackageRole,
};
use program::{Program, Rest, Terminal};
pub use ra::{CarriedView, Ra, RaComponent};
pub use race::{AccessRecord, Conflict, DataRace, EarlierKind};
pub use reach::{ReachRefusal, Reachable, ReachedState};
pub use selection::{
    MemorySelection, MemorySelectionRefusal, ResolvedMemory, ResolvedParallel, SubjectMemory,
};
pub use tso::{Buffered, Tso, TsoComponent, DEFAULT_MEMORY_BOUND};
pub use wire::{
    blocked_json, fairness_json, location_json, over_reference, send_record_json, transition_json,
};

/// A construct this protocol system does not step yet.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Unsupported {
    /// A `parallel` whose resolved memory model the system's memory model
    /// does not implement.
    WeakMemory,
    /// A step binder whose record type has no enumeration here: a type with
    /// a field of no finite domain, such as a text type or a map.
    UnenumerableRecord,
    /// A checked package protocol operation this transition program does
    /// not step.
    PackageControl(fsl_package::protocol_view::ControlOperation),
    /// A package control's correlation premises have no executable form yet.
    PackageCorrelation,
    /// A package protocol declares a replicated role this program does not
    /// step yet.
    PackageReplicatedRole,
}

impl Unsupported {
    /// The construct, as a diagnostic names it.
    pub fn construct(self) -> &'static str {
        match self {
            Self::WeakMemory => "memory",
            Self::UnenumerableRecord => "step binder of an unenumerable record type",
            Self::PackageControl(_) => "package protocol control",
            Self::PackageCorrelation => "package protocol correlation",
            Self::PackageReplicatedRole => "package replicated role",
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
    /// The subject's capacity binding is refused (FSpec FR-785
    /// "Admission"), or a capacity the clause uses has none.
    #[error(transparent)]
    Capacity(CapacityRefusal),
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
    /// A constraint of a node in a weak `parallel` reads through a form
    /// FSpec FR-383 gives no footprint rule, so the model locations it reads
    /// cannot be told (`unsupported_construct`/`not-yet-implemented`).
    #[error("unsupported_construct/not-yet-implemented: the constraint of node {node}: {refusal}")]
    NodeFootprint {
        /// The node, by its static declaration path.
        node: String,
        /// Why its reads have no footprint.
        refusal: fsl_semantics::check::footprint::FootprintRefusal,
    },
    /// Building over an initial state stopped: settling it, or EN-1's
    /// classification of its attempts, ran out of a budget (FR-120
    /// "Post-states"), or a guard's evaluation broke while settling.
    #[error("{}/{}: building over an initial state", .0.cause.code(), .0.cause.cause())]
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
            Self::Capacity(refusal) => refusal.code(),
            Self::Unsupported(_) | Self::NodeFootprint { .. } | Self::WeakAccessShape(_) => {
                Code::UnsupportedConstruct
            }
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
            Self::Capacity(refusal) => refusal.cause(),
            Self::Unsupported(_) | Self::NodeFootprint { .. } => "not-yet-implemented",
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
    pending_access: Option<PendingAccess>,
}

/// The values and memory choices accumulated by the load steps of a
/// multi-location weak-memory access (FR-902).
#[derive(Clone, Debug)]
struct PendingAccess {
    transition: filament_semantic_value::model_transition::ModelTransition,
    receiver: Enc,
    arguments: Vec<Enc>,
    locations: Vec<Location>,
    reads: BTreeMap<Location, PendingRead>,
}

#[derive(Clone, Debug)]
struct PendingRead {
    value: Option<Value>,
    key: Option<String>,
    choice: memory::ReadChoice,
}

impl PartialEq for PendingRead {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.choice == other.choice
    }
}

impl Eq for PendingRead {}

impl PartialEq for PendingAccess {
    fn eq(&self, other: &Self) -> bool {
        self.transition == other.transition
            && self.receiver == other.receiver
            && self.arguments == other.arguments
            && self.locations == other.locations
            && self.reads == other.reads
    }
}

impl Eq for PendingAccess {}

/// A parallel instance's key (ADR-027 PS-4).
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ParallelKey {
    parent: ThreadPath,
    node: fsl_semantics::check::ProtocolNodeId,
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
    send: fsl_semantics::check::ProtocolNodeId,
    message: MessageOccurrence,
    delivery: u64,
    payload: Value,
    enc: Enc,
    bytes: Vec<u8>,
    /// The memory view the send carried (SE-1 channel effect); none under a
    /// model whose sends carry none.
    carried: Option<CarriedView>,
}

/// A compensation registration's status (ADR-027 CP-1 to CP-5, FSpec
/// FR-427).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Status {
    Registered,
    Activated,
    Closed,
    Recovered,
    Unrecovered,
    TimedOut,
}

impl Status {
    fn text(self) -> &'static str {
        match self {
            Self::Registered => "registered",
            Self::Activated => "activated",
            Self::Closed => "closed",
            Self::Recovered => "recovered",
            Self::Unrecovered => "unrecovered",
            Self::TimedOut => "timed-out",
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
    attempts: Vec<AttemptFact>,
}

/// One retained compensation attempt (FR-893): its own record and outcome,
/// and the predecessor index when this is a retry.
#[derive(Clone, Debug)]
pub(super) struct AttemptFact {
    pub(super) record: Enc,
    pub(super) outcome: &'static str,
    pub(super) retry_of: Option<u64>,
}

/// One compensation thread (ADR-027 CP-3): the registration it compensates,
/// its attempt count and its attempt record (PS-8), which it holds only
/// while the key keeps it.
#[derive(Clone, Debug)]
struct Compensating {
    registration: Registered,
    count: u64,
    attempt: Option<Bound>,
}

/// A bounded delivery interval send that filled a blocking queue before all
/// requested entries were appended. The original payload is retained so the
/// sender can continue the same send after a receive frees capacity.
#[derive(Clone, Debug)]
struct PendingSend {
    payload: Value,
    enc: Enc,
    remaining: u64,
    message: MessageOccurrence,
    next_delivery: u64,
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
#[derive(Clone, Debug)]
struct Instance {
    /// The value bound to the protocol's `over` input for this instance.
    over: Value,
    /// The trigger record that started the instance under `activation on
    /// each`, which the activation binder reads (ADR-027 AE-1).
    trigger: Option<Bound>,
    /// The protocol's captures in source order, evaluated once when the
    /// instance starts and frozen (ADR-027 ST-13).
    captures: Vec<Bound>,
    /// The instances of each replicated role (an index into
    /// [`Program::replicated`]), by object, with their status (RR-3). A
    /// retired instance stays, so it is never spawned again.
    roles: BTreeMap<usize, BTreeMap<ObjectReference, RoleStatus>>,
    threads: BTreeMap<ThreadPath, Thread>,
    parallels: BTreeMap<ParallelKey, Parallel>,
    /// Every live thread's binder environment (ADR-027 PS-5), by the thread
    /// that holds it, then binder node: each thread reads its own entry, and
    /// holds its own copy of every value it reads.
    environments: BTreeMap<ThreadPath, BTreeMap<fsl_semantics::check::ProtocolNodeId, Bound>>,
    /// Each `repeat` counter, by (repeat node, thread) (ADR-027 PS-6).
    counters: BTreeMap<(fsl_semantics::check::ProtocolNodeId, ThreadPath), u64>,
    /// The first-decision variant for each active proof-loop invocation,
    /// separate from the bounded-repeat counters above.
    loop_variants: BTreeMap<(fsl_semantics::check::ProtocolNodeId, ThreadPath), Value>,
    /// First loop-proof failure in this instance, for the proof item to
    /// consume at the corresponding reachable state.
    loop_proof_violation: Option<ProtocolLoopProofViolation>,
    /// The next occurrence ordinal of each send node on each thread path.
    message_ordinals: BTreeMap<(fsl_semantics::check::ProtocolNodeId, ThreadPath), u64>,
    /// The next occurrence ordinal of each effect node on each thread path.
    effect_ordinals: BTreeMap<(fsl_semantics::check::ProtocolNodeId, ThreadPath), u64>,
    /// The next delivery index of each message, retained after a copy leaves
    /// the queue so a later duplicate never reuses an index.
    next_deliveries: BTreeMap<MessageOccurrence, u64>,
    /// Each non-empty queue, a `fifo` one in delivery order and an
    /// `unordered` one in ascending JCS bytes of its entries (PS-7).
    queues: BTreeMap<QueueKey, Vec<Entry>>,
    /// The unfinished count of each interval delivery send, keyed by its
    /// sending thread and static send node.
    pending_sends: BTreeMap<(ThreadPath, fsl_semantics::check::ProtocolNodeId), PendingSend>,
    /// Each compensation registration (CP-1).
    registrations: BTreeMap<Registered, Registration>,
    /// Each compensation thread, by its path (CP-3).
    compensating: BTreeMap<ThreadPath, Compensating>,
    /// Each template commit node a step has occurred at (CP-4).
    committed: std::collections::BTreeSet<fsl_semantics::check::ProtocolNodeId>,
}

impl Default for Instance {
    fn default() -> Self {
        Self {
            over: Value::Boolean(false),
            trigger: None,
            captures: Vec::new(),
            roles: BTreeMap::new(),
            threads: BTreeMap::new(),
            parallels: BTreeMap::new(),
            environments: BTreeMap::new(),
            counters: BTreeMap::new(),
            loop_variants: BTreeMap::new(),
            loop_proof_violation: None,
            message_ordinals: BTreeMap::new(),
            effect_ordinals: BTreeMap::new(),
            next_deliveries: BTreeMap::new(),
            queues: BTreeMap::new(),
            pending_sends: BTreeMap::new(),
            registrations: BTreeMap::new(),
            compensating: BTreeMap::new(),
            committed: std::collections::BTreeSet::new(),
        }
    }
}

impl Instance {
    /// FR-736's instance recovery summary over this instance's registrations.
    pub(super) fn recovery_summary(&self) -> &'static str {
        let mut successful = false;
        let mut unsuccessful = false;
        let mut pending = false;
        for registration in self.registrations.values() {
            match registration.status {
                Status::Activated => pending = true,
                Status::Recovered => successful = true,
                Status::Unrecovered | Status::TimedOut => unsuccessful = true,
                Status::Registered | Status::Closed => {}
            }
        }
        if pending {
            "pending"
        } else if successful && unsuccessful {
            "partially-recovered"
        } else if successful {
            "recovered"
        } else if unsuccessful {
            "unrecovered"
        } else {
            "untriggered"
        }
    }

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
        node: fsl_semantics::check::ProtocolNodeId,
        reader: &ThreadPath,
    ) -> Option<&Bound> {
        self.environments.get(reader)?.get(&node)
    }

    /// The entries of the thread `holder`'s environment.
    fn environment<'i>(
        &'i self,
        holder: &ThreadPath,
    ) -> impl Iterator<Item = (fsl_semantics::check::ProtocolNodeId, &'i Bound)> + 'i {
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
    /// The value used for this root state, including before an `on each`
    /// instance is activated.
    over: Value,
    /// Findings raised while creating the initial protocol instance. An
    /// undefined expression at that position is settled by the ordinary
    /// finding path, with the empty prefix.
    initial_findings: Vec<ModelFinding>,
}

impl<M: MemoryModel> Clone for ProtocolState<M> {
    fn clone(&self) -> Self {
        Self {
            model: self.model.clone(),
            instances: self.instances.clone(),
            memory: self.memory.clone(),
            over: self.over.clone(),
            initial_findings: self.initial_findings.clone(),
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
            .field("over", &self.over)
            .field("initial_findings", &self.initial_findings)
            .finish()
    }
}

impl<M: MemoryModel> ProtocolState<M> {
    /// The model state.
    pub fn model(&self) -> &ModelSimState {
        &self.model
    }

    /// The value this protocol instance was created for, including a finite
    /// scalar `over` input.
    pub fn over(&self) -> &Value {
        &self.over
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

/// FSpec FR-425's state key: FR-181's `simulation-state` envelope with the
/// protocol members and the memory model's component (ADR-027 PS-8).
pub struct ProtocolKey<C> {
    members: Vec<(&'static str, Enc)>,
    memory: C,
}

impl<C: Encode> Encode for ProtocolKey<C> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        Envelope {
            members: &self.members,
            memory: &self.memory,
        }
        .encode_into(writer)
    }
}

/// A [`ProtocolKey`]'s layout, over borrowed parts: the one place both its
/// [`Encode`] implementation and its directly written bytes
/// ([`Envelope::to_vec`]) take the members from.
struct Envelope<'k, C> {
    members: &'k [(&'static str, Enc)],
    memory: &'k C,
}

impl<C> Envelope<'_, C> {
    /// The name of the memory model's member.
    const MEMORY: &'static str = "memory";
}

impl<C: Encode> Encode for Envelope<'_, C> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        writer.begin_object()?;
        for (name, value) in self.members {
            writer.name(name)?;
            value.encode_into(writer)?;
        }
        writer.name(Self::MEMORY)?;
        self.memory.encode_into(writer)?;
        writer.end_object()
    }
}

impl<C: Encode> Envelope<'_, C> {
    /// The key's RFC 8785 bytes under `limits`, the memory member's bytes
    /// `memory` when the memory model wrote them: the members written by
    /// `canonical` beside those bytes, or else this envelope's [`Encode`]
    /// bytes and refusal.
    fn to_vec(
        &self,
        canonical: &Canonicalizer,
        memory: Option<&[u8]>,
        limits: quire_canonical::Limits,
    ) -> Result<Vec<u8>, Error> {
        match memory
            .and_then(|memory| canonical.object_with(self.members, Self::MEMORY, memory, limits))
        {
            Some(bytes) => Ok(bytes),
            None => quire_canonical::to_vec(self, limits),
        }
    }
}

/// FR-215 "Model checking a protocol subject" (ADR-027 TS-5): the roots of
/// `protocol` beside its model's: the record type of every binder of an
/// `event`, `send`, `receive` and `finish` step, each at the admitted
/// parameter node `graph` keys it by, and the attempt type of every
/// compensation template, at its admitted compensation node. A binder's
/// bounded record type this module cannot enumerate is refused when the
/// system is built ([`Unsupported::UnenumerableRecord`]); an attempt type it
/// cannot enumerate is an unbounded root.
pub fn protocol_domains(
    protocol: &CheckedProtocol,
    evaluator: &dyn ModelEvaluator,
) -> Vec<(NodeKey, ValueType)> {
    let Some(keys) = evaluator.protocol_node_keys(&protocol.name) else {
        return Vec::new();
    };
    let binders = protocol
        .slots
        .binders
        .iter()
        .filter(|binder| binder.ranged)
        .filter_map(|binder| {
            // The admitted parameter node of the binder the introducing
            // node declares under this name.
            let introducing = binder.node?;
            let key = keys
                .binders
                .iter()
                .find(|key| key.introducing == Some(introducing) && key.name == binder.name)?;
            Some((
                key.node.clone(),
                super::model::root_classification_type(&binder.value_type),
            ))
        });
    // A `cattempt` takes one record of its template's attempt type (ST-11).
    let attempts = protocol
        .obligations
        .compensations
        .iter()
        .enumerate()
        .filter_map(|(index, template)| {
            let node = keys.compensations.get(&index)?;
            Some((
                node.clone(),
                super::model::root_classification_type(&template.attempt_type),
            ))
        });
    binders.chain(attempts).collect()
}

/// ADR-027 TS-5, ST-11: a bounded attempt type that `values_of` cannot
/// enumerate (a `String` or `Text` field, say, which `classify_extent` takes
/// as bounded) is as unbounded to the run as an `Integer`: every leaf that
/// cannot be enumerated is a domain of the request, at its child-index path.
fn check_attempt_types(
    model: &ModelSystem<'_>,
    protocol: &CheckedProtocol,
    evaluator: &dyn ModelEvaluator,
) -> Result<(), NotSimulated> {
    let mut domains = BTreeMap::new();
    let keys = evaluator.protocol_node_keys(&protocol.name);
    for (index, template) in protocol.obligations.compensations.iter().enumerate() {
        let Some(node) = keys.and_then(|keys| keys.compensations.get(&index)) else {
            continue;
        };
        if model.enumerates(&template.attempt_type) {
            continue;
        }
        for path in model.unenumerable_paths(&template.attempt_type) {
            domains.insert(
                DomainKey::Node {
                    node: node.clone(),
                    path,
                },
                DomainKind::Integer,
            );
        }
    }
    match ClaimExtent::from_domains(domains) {
        ClaimExtent::Bounded => Ok(()),
        ClaimExtent::Unbounded(domains) => Err(NotSimulated::RequiresBound(RequiresBound {
            domains,
            capacities: Vec::new(),
        })),
    }
}

/// FR-215 (ADR-027 TS-5): FR-101's requires-bound pre-check over the roots
/// of `model` and of `protocol`, with no state explored. A capacity name
/// `capacities` leaves unbound is an unbounded root too (FSpec FR-785): it
/// is reported with the other unbounded roots in one requires-bound.
///
/// # Errors
///
/// [`NotSimulated::RequiresBound`] when a root is unbounded, naming every
/// unbound capacity name with it;
/// [`NotSimulated::UnboundCapacity`] when only capacity names are;
/// [`NotSimulated::Extent`] when classification stops.
pub fn check_protocol_bounds(
    model: &ModelSystem<'_>,
    protocol: &CheckedProtocol,
    capacities: &Capacities,
    evaluator: &dyn ModelEvaluator,
    position_limit: u64,
) -> Result<(), NotSimulated> {
    let domains = subject_domains(model, protocol, evaluator);
    let with_capacities = |refusal| match refusal {
        NotSimulated::RequiresBound(mut bound) => {
            bound.capacities = capacities.unbound().to_vec();
            NotSimulated::RequiresBound(bound)
        }
        other => other,
    };
    check_requires_bound(&roots(&domains), model.types(), position_limit)
        .map_err(with_capacities)?;
    check_attempt_types(model, protocol, evaluator).map_err(with_capacities)?;
    if capacities.unbound().is_empty() {
        Ok(())
    } else {
        Err(NotSimulated::UnboundCapacity(capacities.unbound().to_vec()))
    }
}

/// The roots of `model` and of `protocol`.
fn subject_domains(
    model: &ModelSystem<'_>,
    protocol: &CheckedProtocol,
    evaluator: &dyn ModelEvaluator,
) -> Vec<(NodeKey, Vec<u32>, ValueType)> {
    let mut domains = model.rooted_domains();
    domains.extend(
        protocol_domains(protocol, evaluator)
            .into_iter()
            .map(|(node, value_type)| (node, Vec::new(), value_type)),
    );
    domains
}

/// `domains` as the requires-bound pre-check reads them.
fn roots(domains: &[(NodeKey, Vec<u32>, ValueType)]) -> Vec<(NodeKey, Vec<u32>, &ValueType)> {
    domains
        .iter()
        .map(|(id, base, value_type)| (id.clone(), base.clone(), value_type))
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
    if let Some(protocol) = system.protocol {
        check_attempt_types(&system.model, protocol, system.evaluator)?;
    }
    let domains = system.protocol.map_or_else(
        || system.model.rooted_domains(),
        |protocol| subject_domains(&system.model, protocol, system.evaluator),
    );
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

/// The `over` parameter's value: the reference to `over`'s key in its
/// declared type's universe.
pub(crate) fn over_value(
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
    let ValueType::Reference(object_type) = &input.value_type else {
        return Err(ProtocolRefusal::fault("over-value-request-is-reference"));
    };
    let instances = model.instances(&input.value_type).map_err(|_| refused())?;
    if !instances.contains(over) {
        return Err(refused());
    }
    model
        .bound_reference(object_type.clone(), over)
        .map(Value::Reference)
        .map_err(|_| refused())
}

/// The finite values of a protocol's `over` binder. FR-882-AC-3 binds an
/// object input as the subject's selected object and enumerates every value
/// of any other finite type.
fn over_values(
    model: &ModelSystem<'_>,
    protocol: &CheckedProtocol,
    over: &ModelInstance,
) -> Result<Vec<Value>, ProtocolRefusal> {
    let input = protocol
        .slots
        .binders
        .first()
        .filter(|binder| binder.node.is_none())
        .ok_or_else(|| ProtocolRefusal::fault("a-protocol-has-an-over-input"))?;
    if matches!(input.value_type, ValueType::Reference(_)) {
        return over_value(model, protocol, over).map(|value| vec![value]);
    }
    if !model.enumerates(&input.value_type) {
        return Err(ProtocolRefusal::Unsupported(
            Unsupported::UnenumerableRecord,
        ));
    }
    model
        .domain_values(&input.value_type, &[])
        .map_err(ProtocolRefusal::InitialStop)
}

/// FR-220 "Sharing and classification" (ADR-025 MA-1, MA-2): EN-1's
/// sharing and classification of every attempt of each `parallel` `memory`
/// resolves weak, over the initial states of `model`. It reads no memory
/// model's rules, so it classifies under `tso` and `ra` alike.
///
/// # Errors
///
/// [`ProtocolRefusal`] when `memory` does not resolve the protocol's
/// outermost `parallel` controls; [`ProtocolRefusal::OverBinding`] when a
/// reference-typed `over` input is bound to an object outside its universe;
/// for a construct this system does not step; when an attempt's
/// application has no footprint; and [`ProtocolRefusal::NodeFootprint`] when
/// the constraint of an `event`, `effect`, `commit`, `send` or `receive`
/// node of a weak `parallel` reads through a form with no footprint rule.
pub fn classify_accesses(
    model: &ModelSystem<'_>,
    protocol: &CheckedProtocol,
    memory: &ResolvedMemory,
    over: &ModelInstance,
    capacities: &[CapacityBinding],
) -> Result<WeakAccesses, ProtocolRefusal> {
    let governing = memory
        .governing(protocol)
        .map_err(ProtocolRefusal::Memory)?;
    let capacities = Capacities::admit(protocol, capacities).map_err(ProtocolRefusal::Capacity)?;
    let program = Program::new(model, protocol, &capacities)?;
    let mut accesses = WeakAccesses::default();
    for over in over_values(model, protocol, over)? {
        accesses.extend(access::classify(
            model,
            protocol,
            &program,
            &governing,
            model.initial().first(),
            &over,
            access::ClassificationMode::Enabled,
        )?);
    }
    Ok(accesses)
}

/// FR-226 (ADR-025 MX-4): EN-1's pre-check of the load-buffering shapes of
/// `protocol` under `memory`, one per distinct (load, store) pair of a
/// `parallel` resolved to `ra`, in ascending order. A subject resolving no
/// `parallel` to `ra` has none.
///
/// # Errors
///
/// As [`classify_accesses`].
pub fn load_buffering_shapes(
    model: &ModelSystem<'_>,
    protocol: &CheckedProtocol,
    memory: &ResolvedMemory,
    over: &ModelInstance,
    capacities: &[CapacityBinding],
) -> Result<Vec<LoadBufferingShape>, ProtocolRefusal> {
    let governing = memory
        .governing(protocol)
        .map_err(ProtocolRefusal::Memory)?;
    if !governing
        .values()
        .any(|model| *model == fsl_foundation::MemoryModelName::Ra)
    {
        return Ok(Vec::new());
    }
    let admitted = Capacities::admit(protocol, capacities).map_err(ProtocolRefusal::Capacity)?;
    let program = Program::new(model, protocol, &admitted)?;
    let accesses = classify_accesses(model, protocol, memory, over, capacities)?;
    Ok(
        access::load_buffering_shapes(&program, &governing, &accesses)?
            .into_iter()
            .collect(),
    )
}

/// FR-220, FR-940: classify weak-memory accesses from an admitted package
/// protocol program, without a source `CheckedProtocol`.
pub fn classify_package_accesses(
    model: &ModelSystem<'_>,
    program: &PackageProtocolProgram<'_>,
    over: &Value,
    potential: bool,
) -> Result<WeakAccesses, ProtocolRefusal> {
    let governing = program
        .governing_models()
        .map_err(ProtocolRefusal::Memory)?;
    let runtime = Program::new_package(model, program)?;
    let orderings = program
        .controls
        .values()
        .filter(|control| {
            control.operation == fsl_package::protocol_view::ControlOperation::Attempt
        })
        .map(|control| (control.id, control.ordering))
        .collect();
    access::classify_package(
        model,
        &runtime,
        &governing,
        model.initial().first(),
        over,
        if potential {
            access::ClassificationMode::Potential
        } else {
            access::ClassificationMode::Enabled
        },
        &orderings,
    )
}

/// FR-226, FR-940: load-buffering shapes of an admitted package protocol
/// under `ra`, preserving the same access and program-order rules as source.
pub fn package_load_buffering_shapes(
    model: &ModelSystem<'_>,
    program: &PackageProtocolProgram<'_>,
    over: &Value,
) -> Result<Vec<LoadBufferingShape>, ProtocolRefusal> {
    let governing = program
        .governing_models()
        .map_err(ProtocolRefusal::Memory)?;
    if !governing
        .values()
        .any(|model| *model == fsl_foundation::MemoryModelName::Ra)
    {
        return Ok(Vec::new());
    }
    let runtime = Program::new_package(model, program)?;
    let orderings = program
        .controls
        .values()
        .filter(|control| {
            control.operation == fsl_package::protocol_view::ControlOperation::Attempt
        })
        .map(|control| (control.id, control.ordering))
        .collect();
    let accesses = access::classify_package(
        model,
        &runtime,
        &governing,
        model.initial().first(),
        over,
        access::ClassificationMode::Potential,
        &orderings,
    )?;
    Ok(
        access::load_buffering_shapes(&runtime, &governing, &accesses)?
            .into_iter()
            .collect(),
    )
}

/// One bounded protocol expansion together with its terminal classification.
pub type ProtocolSuccessorsAndTerminal<M> = (
    Expansion<ProtocolTransition, ProtocolState<M>, ModelFinding>,
    TerminalClass,
);

/// FR-215: the transition system of a protocol subject.
pub struct ProtocolSystem<'a, M: MemoryModel> {
    model: ModelSystem<'a>,
    evaluator: &'a dyn ModelEvaluator,
    protocol: Option<&'a CheckedProtocol>,
    protocol_name: String,
    program: Program<'a>,
    /// The admitted package execution form, when this system was built from
    /// package nodes rather than a source `CheckedProtocol`.
    package_program: Option<PackageProtocolProgram<'a>>,
    /// The package route's model admission inputs for current slot observations.
    package_subject: Option<&'a ProtocolSubject<'a>>,
    /// A loop proof item is a product of the ordinary protocol state with
    /// the prior decision value for exactly one repeat invocation.
    loop_proof_target: Option<ProtocolLoopProofTarget>,
    memory: M,
    /// The per-channel queue occupancy bound supplied by model checking.
    channel_message_bound: Option<u64>,
    /// An explicit delivery read set for cycle exploration; `None` keeps
    /// the concrete transition system used by execution and replay.
    delivery_reads: Option<BTreeSet<String>>,
    /// An explicit message identity read set for cycle exploration.
    message_reads: Option<BTreeSet<String>>,
    /// The model governing each `parallel`: its outermost `parallel`'s
    /// resolved model (FR-219, ADR-025 MM-3).
    governing: BTreeMap<fsl_semantics::check::ProtocolNodeId, fsl_foundation::MemoryModelName>,
    /// Static weak-parallel ancestors of each protocol control, inner first.
    weak_ancestors: BTreeMap<fsl_semantics::check::ProtocolNodeId, Vec<access::WeakAncestor>>,
    /// Static paths of weak parallel controls, for filtering branch steps.
    weak_parallel_paths: BTreeSet<String>,
    /// Attempts grouped by their exact outer-to-inner weak branch context.
    attempts_by_weak_context:
        BTreeMap<Vec<(String, String)>, Vec<fsl_semantics::check::ProtocolNodeId>>,
    /// EN-1's initial enabled-footprint classification of the attempts of
    /// every weak `parallel` (FR-220).
    initial_enabled: WeakAccesses,
    /// EN-1's potential footprints, used to reserve memory backing for
    /// operations that may become enabled (ADR-025 MA-1).
    potential: WeakAccesses,
    /// The shared locations some attempt accesses with no `ordering`: those
    /// with a race summary (ADR-025 DR-3).
    non_atomic: BTreeSet<Location>,
    /// The finite domain of the `over` input. Object inputs have one bound
    /// value; finite scalar inputs have one entry for each value.
    over_values: Vec<Value>,
    /// Each static role's binding: role name, then model identity.
    roles: Vec<(String, String)>,
    /// The objects of each replicated role's population in the subject's
    /// universe (ADR-027 RR-1), by the role's index in the program.
    universes: Vec<Vec<ObjectReference>>,
    /// The instance bound of `activation on each` (ADR-027 AE-4).
    max_live_instances: u64,
    initial: Vec<ProtocolState<M>>,
    /// The model locations each application of an attempt reads and
    /// writes, by attempt node and transition identity: a function of the
    /// two alone, derived once per run.
    model_footprints: std::sync::Mutex<ModelFootprints>,
}

/// The part of a checked repeat's loop proof selected by an EN-1 item.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtocolLoopProofKind {
    /// Check the invariant at every repeat decision.
    Invariant,
    /// Check the variant on every completed iteration.
    Termination,
}

/// One repeat-specific loop proof obligation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtocolLoopProofTarget {
    /// The checked repeat node.
    pub repeat: fsl_semantics::check::ProtocolNodeId,
    /// The obligation to discharge.
    pub kind: ProtocolLoopProofKind,
}

/// Why a protocol loop-proof item failed at a reachable decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtocolLoopProofFailure {
    /// The invariant evaluated to false.
    InvariantFalse,
    /// The invariant or variant was undefined.
    Undefined,
    /// The variant did not strictly decrease across a back edge.
    VariantDidNotDecrease,
    /// The first value of a back edge violated the nonnegative floor.
    VariantBelowFloor,
    /// Evaluation exhausted a named resource before deciding the body.
    EvaluationIncomplete,
    /// The checked body refused evaluation.
    EvaluationRefused,
}

/// A concrete loop-proof refutation found while expanding a state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtocolLoopProofViolation {
    /// The checked repeat node.
    pub repeat: fsl_semantics::check::ProtocolNodeId,
    /// The obligation that failed.
    pub kind: ProtocolLoopProofKind,
    /// The failure.
    pub failure: ProtocolLoopProofFailure,
}

/// Flatten a checked integer or integer tuple variant into its ordered
/// components and declared component types.
fn loop_variant_components(
    value: &Value,
    value_type: &ValueType,
    typing: encode::Typing<'_>,
) -> Option<Vec<(Integer, ValueType)>> {
    match value_type {
        ValueType::Integer | ValueType::Int(_) => match value {
            Value::Integer(value) => Some(vec![(value.clone(), value_type.clone())]),
            _ => None,
        },
        ValueType::Composite(key) => {
            let Value::Composite(value) = value else {
                return None;
            };
            let types = typing.tuple_types(key)?;
            if value.slots().len() != types.len() {
                return None;
            }
            let mut components = Vec::with_capacity(types.len());
            for (slot, value_type) in value.slots().iter().zip(types) {
                let filament_exact::FieldValue::Present(value) = slot else {
                    return None;
                };
                let (Value::Integer(value), ValueType::Integer | ValueType::Int(_)) =
                    (value, value_type)
                else {
                    return None;
                };
                components.push((value.clone(), value_type.clone()));
            }
            Some(components)
        }
        _ => None,
    }
}

/// The lexicographic comparison of `left` and `right`, with `None` for a
/// value that does not match the checked integer/tuple shape.
fn compare_loop_variants(
    left: &Value,
    right: &Value,
    value_type: &ValueType,
    typing: encode::Typing<'_>,
) -> Option<std::cmp::Ordering> {
    let left = loop_variant_components(left, value_type, typing)?;
    let right = loop_variant_components(right, value_type, typing)?;
    if left.len() != right.len() {
        return None;
    }
    for ((left, _), (right, _)) in left.iter().zip(right.iter()) {
        let ordering = left.cmp(right);
        if ordering != std::cmp::Ordering::Equal {
            return Some(ordering);
        }
    }
    Some(std::cmp::Ordering::Equal)
}

/// Whether each component of the first-decision variant meets its
/// nonnegative floor, including the type-level nonnegative lower bound.
fn loop_variant_floor_holds(
    value: &Value,
    value_type: &ValueType,
    typing: encode::Typing<'_>,
) -> bool {
    let Some(components) = loop_variant_components(value, value_type, typing) else {
        return false;
    };
    components.iter().all(|(value, value_type)| {
        let type_proves_floor =
            matches!(value_type, ValueType::Int(interval) if !interval.lower().is_negative());
        type_proves_floor || !value.is_negative()
    })
}

/// The model locations of each application of an attempt, by attempt node
/// and transition identity.
type ModelFootprints = BTreeMap<
    (
        fsl_semantics::check::ProtocolNodeId,
        filament_semantic_value::model_transition::ModelTransition,
    ),
    std::sync::Arc<ModelFootprint>,
>;

/// The model locations one application of an attempt reads and writes, its
/// protocol locations left out.
struct ModelFootprint {
    reads: BTreeSet<Location>,
    writes: BTreeSet<Location>,
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
        Self::new_with_loop_proof(model, subject, memory, None)
    }

    /// FR-730: build a proof-product protocol system for one repeat item.
    /// The target is installed before initial states settle, so decisions at
    /// the first reachable state participate in the obligation.
    pub fn new_with_loop_proof(
        model: ModelSystem<'a>,
        subject: &ProtocolSubject<'a>,
        memory: M,
        loop_proof_target: Option<ProtocolLoopProofTarget>,
    ) -> Result<Self, ProtocolRefusal> {
        let evaluator = subject.model.simulation.input.evaluator;
        let protocol = evaluator.protocol(&subject.protocol).ok_or_else(|| {
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
                .clone()
                .and_then(|context| model.declaring_package(context))
                .is_some_and(|declaring| declaring == binding.model);
            if !admits {
                return Err(ProtocolRefusal::RoleMapping {
                    role: role.name.clone(),
                });
            }
            roles.push((role.name.clone(), binding.model.clone()));
        }
        // FSL FR-1033-AC-5: a binding naming no role of the protocol is not one
        // the protocol admits.
        if let Some(unknown) = subject
            .roles
            .iter()
            .find(|binding| !protocol.roles.iter().any(|role| role.name == binding.role))
        {
            return Err(ProtocolRefusal::RoleMapping {
                role: unknown.role.clone(),
            });
        }
        let over_values = over_values(&model, protocol, &subject.over)?;
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
        let capacities =
            Capacities::admit(protocol, &subject.capacities).map_err(ProtocolRefusal::Capacity)?;
        let program = Program::new(&model, protocol, &capacities)?;
        let mut enabled = WeakAccesses::default();
        let mut weak = WeakAccesses::default();
        for over in &over_values {
            enabled.extend(access::classify(
                &model,
                protocol,
                &program,
                &governing,
                model.initial().first(),
                over,
                access::ClassificationMode::Enabled,
            )?);
            weak.extend(access::classify(
                &model,
                protocol,
                &program,
                &governing,
                model.initial().first(),
                over,
                access::ClassificationMode::Potential,
            )?);
        }
        let weak_ancestors = access::weak_ancestors(&program, &governing)?;
        let mut weak_parallel_paths = BTreeSet::new();
        for (node, model) in &governing {
            if model.is_weak()
                && program.control(*node)?.step == fsl_semantics::check::ControlStep::Parallel
            {
                weak_parallel_paths.insert(program.text(*node)?.to_owned());
            }
        }
        let mut attempts_by_weak_context = BTreeMap::new();
        for node in program.attempts.keys() {
            let Some(ancestors) = weak_ancestors.get(node) else {
                return Err(ProtocolRefusal::fault("an-attempt-has-weak-ancestors"));
            };
            let context = ancestors
                .iter()
                .rev()
                .filter(|ancestor| ancestor.is_branch_context)
                .map(|ancestor| (ancestor.path.clone(), ancestor.branch.clone()))
                .collect::<Vec<_>>();
            if !context.is_empty() {
                attempts_by_weak_context
                    .entry(context)
                    .or_insert_with(Vec::new)
                    .push(*node);
            }
        }
        let mut system = Self {
            model,
            evaluator,
            protocol: Some(protocol),
            protocol_name: protocol.name.clone(),
            program,
            package_program: None,
            package_subject: None,
            loop_proof_target,
            memory,
            channel_message_bound: None,
            delivery_reads: None,
            message_reads: None,
            governing,
            weak_ancestors,
            weak_parallel_paths,
            attempts_by_weak_context,
            non_atomic: weak.non_atomic(),
            potential: weak,
            initial_enabled: enabled,
            over_values,
            roles,
            universes: Vec::new(),
            max_live_instances: DEFAULT_MAX_LIVE_INSTANCES,
            initial: Vec::new(),
            model_footprints: std::sync::Mutex::default(),
        };
        if let Some(target) = system.loop_proof_target {
            if !system
                .program
                .repeat(target.repeat)
                .is_ok_and(|repeat| repeat.proof.is_some())
            {
                return Err(ProtocolRefusal::fault(
                    "a-loop-proof-target-names-a-checked-loop-proof",
                ));
            }
        }
        system.universes = system.role_universes()?;
        system.initial = system.initial_states()?;
        Ok(system)
    }

    /// FR-940: build the sequence, weak-parallel, attempt and finish
    /// transition system directly from the admitted package protocol form.
    /// The checked package node identities remain the expression and
    /// operation authority throughout execution.
    pub fn new_package(
        model: ModelSystem<'a>,
        subject: &'a ProtocolSubject<'a>,
        memory: M,
    ) -> Result<Self, ProtocolRefusal> {
        let input = &subject.model.simulation.input;
        let form = input
            .protocol_definition(&subject.protocol)
            .and_then(|definition| match definition {
                super::model::ProtocolDefinition::Package(form) => Some(form),
                super::model::ProtocolDefinition::Source(_) => None,
            })
            .ok_or_else(|| ProtocolRefusal::UnknownProtocol {
                protocol: subject.protocol.clone(),
            })?;
        let package = PackageProtocolProgram::with_memory(input, *form, subject.memory.clone())
            .map_err(|refusal| {
                eprintln!("RECEIVE-FIFO-DIAG package-program {refusal:?}");
                match refusal {
                    PackageProgramRefusal::Memory(refusal) => ProtocolRefusal::Memory(refusal),
                    PackageProgramRefusal::Unsupported(operation) => {
                        ProtocolRefusal::Unsupported(Unsupported::PackageControl(operation))
                    }
                    PackageProgramRefusal::Correlation { .. } => {
                        ProtocolRefusal::Unsupported(Unsupported::PackageCorrelation)
                    }
                    _ => ProtocolRefusal::fault("a-selected-package-protocol-builds"),
                }
            })?;
        if let Some(binding) = subject.capacities.first() {
            return Err(ProtocolRefusal::Capacity(CapacityRefusal::UnknownName {
                name: binding.name.clone(),
            }));
        }
        if package.roles.iter().any(|role| role.replicated) {
            return Err(ProtocolRefusal::Unsupported(
                Unsupported::PackageReplicatedRole,
            ));
        }
        let mut roles = Vec::new();
        for role in package.roles.iter().filter(|role| !role.replicated) {
            let binding = subject
                .roles
                .iter()
                .find(|binding| binding.role == role.name)
                .ok_or_else(|| ProtocolRefusal::RoleMapping {
                    role: role.name.clone(),
                })?;
            let admits = model
                .declaring_package(role.model_identity.clone())
                .is_some_and(|declaring| declaring == binding.model);
            if !admits {
                return Err(ProtocolRefusal::RoleMapping {
                    role: role.name.clone(),
                });
            }
            roles.push((role.name.clone(), binding.model.clone()));
        }
        if let Some(unknown) = subject.roles.iter().find(|binding| {
            !package
                .roles
                .iter()
                .any(|role| !role.replicated && role.name == binding.role)
        }) {
            return Err(ProtocolRefusal::RoleMapping {
                role: unknown.role.clone(),
            });
        }
        let over_values = match &package.over_type {
            ValueType::Reference(object_type) => {
                let instances = model.instances(&package.over_type).map_err(|_| {
                    ProtocolRefusal::OverBinding {
                        parameter: "over".to_owned(),
                        object: subject.over.key.clone(),
                    }
                })?;
                if !instances.contains(&subject.over) {
                    return Err(ProtocolRefusal::OverBinding {
                        parameter: "over".to_owned(),
                        object: subject.over.key.clone(),
                    });
                }
                vec![Value::Reference(
                    model
                        .bound_reference(object_type.clone(), &subject.over)
                        .map_err(|_| ProtocolRefusal::fault("a-package-over-reference-resolves"))?,
                )]
            }
            _ => {
                return Err(ProtocolRefusal::fault(
                    "package-protocol-over-is-a-reference-value",
                ));
            }
        };
        let governing = package
            .governing_models()
            .map_err(ProtocolRefusal::Memory)?;
        let implemented = memory.models();
        if governing
            .values()
            .any(|resolved| !implemented.contains(resolved))
        {
            return Err(ProtocolRefusal::Unsupported(Unsupported::WeakMemory));
        }
        let program = Program::new_package(&model, &package).inspect_err(|failure| {
            eprintln!("RECEIVE-FIFO-DIAG normalized-program {failure:?}");
        })?;
        let mut enabled = WeakAccesses::default();
        let mut weak = WeakAccesses::default();
        for over in &over_values {
            enabled.extend(classify_package_accesses(&model, &package, over, false)?);
            weak.extend(classify_package_accesses(&model, &package, over, true)?);
        }
        let weak_ancestors = access::weak_ancestors(&program, &governing)?;
        let mut weak_parallel_paths = BTreeSet::new();
        for (node, resolved) in &governing {
            if resolved.is_weak()
                && program.control(*node)?.step == fsl_semantics::check::ControlStep::Parallel
            {
                weak_parallel_paths.insert(program.text(*node)?.to_owned());
            }
        }
        let mut attempts_by_weak_context = BTreeMap::new();
        for node in program.attempts.keys() {
            let Some(ancestors) = weak_ancestors.get(node) else {
                return Err(ProtocolRefusal::fault("an-attempt-has-weak-ancestors"));
            };
            let context = ancestors
                .iter()
                .rev()
                .filter(|ancestor| ancestor.is_branch_context)
                .map(|ancestor| (ancestor.path.clone(), ancestor.branch.clone()))
                .collect::<Vec<_>>();
            if !context.is_empty() {
                attempts_by_weak_context
                    .entry(context)
                    .or_insert_with(Vec::new)
                    .push(*node);
            }
        }
        let mut system = Self {
            model,
            evaluator: input.evaluator,
            protocol: None,
            protocol_name: subject.protocol.clone(),
            program,
            package_program: Some(package),
            package_subject: Some(subject),
            loop_proof_target: None,
            memory,
            channel_message_bound: None,
            delivery_reads: None,
            message_reads: None,
            governing,
            weak_ancestors,
            weak_parallel_paths,
            attempts_by_weak_context,
            non_atomic: weak.non_atomic(),
            potential: weak,
            initial_enabled: enabled,
            over_values,
            roles,
            universes: Vec::new(),
            max_live_instances: DEFAULT_MAX_LIVE_INSTANCES,
            initial: Vec::new(),
            model_footprints: std::sync::Mutex::default(),
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

    /// This system with a per-channel `max_messages` bound (FR-225-AC-6).
    /// A send offered to a full channel is left unexpanded before its
    /// overflow policy is applied.
    #[must_use]
    pub fn with_channel_message_bound(mut self, bound: u64) -> Self {
        self.channel_message_bound = Some(bound);
        self
    }

    /// Whether the checked protocol declares at least one channel.
    pub fn has_channels(&self) -> bool {
        !self.program.channels.is_empty()
    }

    /// The selected per-channel message bound, when model checking supplied one.
    pub fn channel_message_bound(&self) -> Option<u64> {
        self.channel_message_bound
    }

    /// Whether any live instance holds an undelivered message on an
    /// exactly-once-premise channel.
    ///
    /// Global conformance uses this at `finish`: the protocol simulator
    /// retires an instance there, while FR-896 still owes the channel's
    /// exactly-once premise over the completed execution.
    pub fn has_pending_exactly_once_delivery(&self, state: &ProtocolState<M>) -> bool {
        state.instances.values().any(|instance| {
            instance.queues.iter().any(|(queue, entries)| {
                !entries.is_empty()
                    && self
                        .program
                        .channels
                        .get(queue.channel)
                        .is_some_and(|channel| {
                            channel.delivery
                                == fsl_semantics::check::ChannelDelivery::ExactlyOncePremise
                        })
            })
        })
    }

    /// The first target-specific loop-proof failure carried by `state`, if
    /// any. Ordinary protocol systems never populate this field.
    pub fn loop_proof_violation<'s>(
        &self,
        state: &'s ProtocolState<M>,
    ) -> Option<&'s ProtocolLoopProofViolation> {
        state
            .instances
            .values()
            .find_map(|instance| instance.loop_proof_violation.as_ref())
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
                    .instances(&ValueType::Reference(role.object.clone()))
                    .map_err(|_| refused())?;
                let mut objects = instances
                    .iter()
                    .map(|instance| self.model.bound_reference(role.object.clone(), instance))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| refused())?;
                objects.sort();
                objects.dedup();
                Ok(objects)
            })
            .collect()
    }

    /// FO-3: for each initial model state, instance 0 with its root thread
    /// settled from the start of `run`; under `activation on each`, no
    /// instance (ADR-027 AE-1).
    fn initial_states(&self) -> Result<Vec<ProtocolState<M>>, ProtocolRefusal> {
        let mut states = Vec::new();
        for model in self.model.initial() {
            for over in &self.over_values {
                let (instances, initial_findings) = if self.program.activation.is_some() {
                    (BTreeMap::new(), Vec::new())
                } else {
                    let started =
                        self.start(0, None, &model, over)
                            .map_err(|error| match error {
                                settle::SettleError::Refused(refusal) => refusal,
                                settle::SettleError::Stopped(stop) => {
                                    ProtocolRefusal::InitialStop(stop)
                                }
                            })?;
                    match started {
                        roles::Started::Instance(instance) => {
                            (BTreeMap::from([(0, *instance)]), Vec::new())
                        }
                        roles::Started::Undecided { instance, finding } => {
                            // A proof failure is a reachable decision even when
                            // the guard cannot choose the next control.
                            let instances = if instance.loop_proof_violation.is_some() {
                                BTreeMap::from([(0, *instance)])
                            } else {
                                BTreeMap::new()
                            };
                            (instances, vec![*finding])
                        }
                    }
                };
                states.push(ProtocolState {
                    model: model.clone(),
                    instances,
                    memory: self.memory.initial(),
                    over: over.clone(),
                    initial_findings,
                });
            }
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
            .expect("the source protocol accessor is used on a source route")
    }

    /// FR-212 (ADR-027 PA-3, FSpec FR-430): the fairness set of an
    /// infinite-trace clause whose authored constraints are `authored`
    /// over this subject: the authored members, unless the protocol
    /// declares `scheduling adversarial` its scheduler constraints, and the
    /// `memory` constraints of its memory model (FR-228), in ascending JCS
    /// bytes.
    ///
    /// # Errors
    ///
    /// [`ProtocolFairnessError`] when an authored constraint names an
    /// operation the subject's model does not apply.
    pub fn fairness_set(
        &self,
        authored: &[filament_semantic_value::fairness::FairnessConstraint],
    ) -> Result<Vec<ProtocolFairness>, ProtocolFairnessError> {
        self.fairness_set_with_targets(authored, &[])
    }

    /// The FR-212 fairness set including checked protocol-node targets.
    pub fn fairness_set_with_targets(
        &self,
        authored: &[filament_semantic_value::fairness::FairnessConstraint],
        authored_targets: &[fsl_semantics::check::CheckedProtocolFairness],
    ) -> Result<Vec<ProtocolFairness>, ProtocolFairnessError> {
        let Some(protocol) = self.protocol else {
            return self.package_fairness_set(authored, authored_targets);
        };
        let memory = self
            .memory
            .fairness(&fairness::memory_branches(protocol, &self.governing));
        fairness::fairness_set(
            protocol,
            authored,
            authored_targets,
            |constraint| {
                let operation = constraint.operation();
                self.model
                    .operation_text(operation.declaring_type(), operation.name())
            },
            memory,
        )
    }

    /// Build the same fairness set from the admitted package protocol form.
    /// Package controls use checked-package node identities, so source
    /// `ProtocolNodeId` targets cannot be resolved against them directly.
    fn package_fairness_set(
        &self,
        authored: &[filament_semantic_value::fairness::FairnessConstraint],
        authored_targets: &[fsl_semantics::check::CheckedProtocolFairness],
    ) -> Result<Vec<ProtocolFairness>, ProtocolFairnessError> {
        let Some(package) = self.package_program.as_ref() else {
            return Err(ProtocolFairnessError::UnknownProtocolTarget {
                name: self.protocol_name.clone(),
            });
        };
        if let Some(target) = authored_targets.first() {
            return Err(ProtocolFairnessError::UnknownProtocolTarget {
                name: target.protocol.clone(),
            });
        }

        let mut members = Vec::new();
        for constraint in authored {
            let operation = constraint.operation();
            let name = self
                .model
                .operation_text(operation.declaring_type(), operation.name())
                .ok_or_else(|| ProtocolFairnessError::UnknownOperation {
                    name: operation.name().to_owned(),
                })?;
            members.push(ProtocolFairness::new(
                constraint.kind(),
                ProtocolTarget::Operation {
                    name: name.to_owned(),
                },
                constraint.granularity(),
                FairnessOrigin::Authored,
            ));
        }

        if package.form.protocol.scheduling()
            != Some(fsl_package::protocol_view::Scheduling::Adversarial)
        {
            members.push(ProtocolFairness::scheduler(ProtocolTarget::Root));
            for control in package.controls.values().filter(|control| {
                control.operation == fsl_package::protocol_view::ControlOperation::Parallel
            }) {
                let parallel = control.path.join("/");
                for (branch, _) in &control.branches {
                    members.push(ProtocolFairness::scheduler(ProtocolTarget::branch(
                        parallel.clone(),
                        branch.clone(),
                    )));
                }
            }
        }

        let branches = self
            .governing
            .iter()
            .filter_map(|(parallel, model)| {
                let control = package.controls.get(parallel)?;
                Some(
                    control
                        .branches
                        .iter()
                        .map(move |(branch, _)| MemoryBranch {
                            model: *model,
                            parallel: control.path.join("/"),
                            branch: branch.clone(),
                        }),
                )
            })
            .flatten()
            .collect::<Vec<_>>();
        members.extend(self.memory.fairness(&branches));

        let mut keyed = BTreeMap::new();
        for member in members {
            let bytes = quire_canonical::to_vec(&member, quire_canonical::Limits::new(u64::MAX))
                .map_err(|error| ProtocolFairnessError::Encoding(error.to_string()))?;
            keyed.entry(bytes).or_insert(member);
        }
        Ok(keyed.into_values().collect())
    }

    /// The memory model it runs under.
    pub fn memory(&self) -> &M {
        &self.memory
    }

    /// Initial-observation EN-1 access snapshot, retained for inspection.
    /// Runtime expansion derives a fresh classification for each reachable
    /// protocol state and thread observation (FR-220, ADR-025 MA-1).
    pub fn weak_accesses(&self) -> &WeakAccesses {
        &self.initial_enabled
    }

    /// Whether `state` has an enabled non-atomic access to a shared location
    /// under EN-1's state-specific classification (FR-220, FR-224).
    ///
    /// Potential footprints do not count. The query applies each attempt at
    /// the model observations of this protocol state and considers only an
    /// enabled load or store without an ordering whose location is shared.
    ///
    /// # Errors
    ///
    /// [`ExpansionStop`] when the state-specific application classification
    /// cannot complete within its evaluation budget.
    pub fn has_non_atomic_access_at(
        &self,
        state: &ProtocolState<M>,
    ) -> Result<bool, ExpansionStop> {
        let mut budget = self.model.budget();
        self.state_accesses(state, &mut budget)
            .map(|accesses| accesses.has_non_atomic_access())
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

    /// The protocol captures of each instance live at `state` (ADR-027
    /// ST-13), ascending by ordinal: the instance's ordinal and the RFC 8785
    /// bytes of each capture's name and FR-181 typed value, the values every
    /// role of the instance holds (FR-218). A capture is evaluated once when
    /// its instance starts and is gone once the instance finishes, so a
    /// reader binds each instance's bytes when it first sees the instance.
    /// Empty when the protocol declares no capture.
    ///
    /// # Errors
    ///
    /// An [`ExpansionStop`] when an instance holds a different number of
    /// captures than the protocol declares, or a capture value has no typed
    /// canonical representation.
    pub fn capture_bytes(
        &self,
        state: &ProtocolState<M>,
    ) -> Result<Vec<(u64, Vec<u8>)>, ExpansionStop> {
        if self.program.captures.is_empty() {
            return Ok(Vec::new());
        }
        state
            .instances
            .iter()
            .map(|(ordinal, instance)| {
                if instance.captures.len() != self.program.captures.len() {
                    return Err(broken());
                }
                let captures = self
                    .program
                    .captures
                    .iter()
                    .zip(&instance.captures)
                    .map(|(capture, bound)| (Enc::text(capture.name.clone()), bound.enc.clone()))
                    .collect();
                quire_canonical::to_vec(&map_enc(captures), quire_canonical::Limits::new(u64::MAX))
                    .map(|bytes| (*ordinal, bytes))
                    .map_err(|_| broken())
            })
            .collect()
    }

    /// FSpec FR-425's `<rest>` of `rest`.
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

    /// The canonical key of the registration `key` (FSpec FR-425).
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
    /// successors, and PS-8 lists no member for that mapping either. A
    /// compensation thread's attempt record is an `observations` entry
    /// `{"type":"attempt-record","thread"}`.
    fn key_members(&self, state: &ProtocolState<M>) -> Vec<(&'static str, Enc)> {
        let text = |value: &str| Enc::tagged("text", vec![("value", Enc::text(value))]);
        let mut control = Vec::new();
        let mut observations = Vec::new();
        let mut roles = Vec::new();
        let mut queues = Vec::new();
        let mut bounds = Vec::new();
        if self.over_values.len() > 1 {
            observations.push((
                Enc::tagged("protocol-over", Vec::new()),
                value_enc(&state.over, self.model.typing()),
            ));
        }
        for (ordinal, instance) in &state.instances {
            control.push((
                Enc::tagged(
                    "protocol-instance",
                    vec![("ordinal", Enc::text(ordinal.to_string()))],
                ),
                Enc::tagged(
                    "instance-state",
                    vec![
                        ("finished", Enc::Bool(false)),
                        ("recovery_summary", Enc::text(instance.recovery_summary())),
                    ],
                ),
            ));
            for (path, thread) in &instance.threads {
                let rest = self
                    .rest_point(thread.rest)
                    .map_or(Enc::Unencodable("a rest point"), |rest| rest.enc());
                let mut members = vec![
                    ("rest", rest),
                    ("disposition", Enc::text(thread.disposition.text())),
                ];
                if let Some(pending) = &thread.pending_access {
                    let transition = &pending.transition;
                    let reads = pending
                        .locations
                        .iter()
                        .filter_map(|location| {
                            let read = pending.reads.get(location)?;
                            Some(Enc::tagged(
                                "pending-access-read",
                                vec![
                                    (
                                        "location",
                                        access::location_enc(
                                            location,
                                            self.potential.cell(location),
                                        ),
                                    ),
                                    (
                                        "value",
                                        read.value.as_ref().map_or(Enc::Null, |value| {
                                            value_enc(value, self.model.typing())
                                        }),
                                    ),
                                    ("read_depth", Enc::text(read.choice.depth.to_string())),
                                ],
                            ))
                        })
                        .collect();
                    members.push((
                        "pending_access",
                        Enc::tagged(
                            "pending-access",
                            vec![
                                (
                                    "transition",
                                    Enc::tagged(
                                        "model-transition",
                                        vec![
                                            (
                                                "operation",
                                                Enc::text(transition.operation().to_owned()),
                                            ),
                                            (
                                                "receiver",
                                                transition
                                                    .receiver()
                                                    .map_or(Enc::Null, |receiver| {
                                                        Enc::text(receiver.to_owned())
                                                    }),
                                            ),
                                            (
                                                "arguments",
                                                Enc::Array(
                                                    transition
                                                        .arguments()
                                                        .iter()
                                                        .cloned()
                                                        .map(Enc::text)
                                                        .collect(),
                                                ),
                                            ),
                                        ],
                                    ),
                                ),
                                ("receiver", pending.receiver.clone()),
                                ("arguments", Enc::Array(pending.arguments.clone())),
                                (
                                    "locations",
                                    Enc::Array(
                                        pending
                                            .locations
                                            .iter()
                                            .map(|location| {
                                                access::location_enc(
                                                    location,
                                                    self.potential.cell(location),
                                                )
                                            })
                                            .collect(),
                                    ),
                                ),
                                ("reads", Enc::Array(reads)),
                            ],
                        ),
                    ));
                }
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
            for ((path, node), pending) in &instance.pending_sends {
                let node = self
                    .program
                    .text(*node)
                    .map_or(Enc::Unencodable("a pending send node"), Enc::text);
                control.push((
                    Enc::tagged(
                        "send-pending",
                        vec![
                            ("instance", Enc::text(ordinal.to_string())),
                            ("thread", path.enc()),
                            ("node", node),
                        ],
                    ),
                    Enc::tagged(
                        "pending-send-state",
                        vec![
                            ("remaining", Enc::text(pending.remaining.to_string())),
                            ("payload", pending.enc.clone()),
                            ("message", pending.message.enc()),
                            (
                                "next_delivery",
                                Enc::text(pending.next_delivery.to_string()),
                            ),
                        ],
                    ),
                ));
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
            for ((node, thread), value) in &instance.loop_variants {
                let node = self
                    .program
                    .text(*node)
                    .map_or(Enc::Unencodable("a repeat node"), Enc::text);
                bounds.push((
                    Enc::tagged(
                        "loop-proof-variant",
                        vec![("node", node), ("thread", thread.enc())],
                    ),
                    value_enc(value, self.model.typing()),
                ));
            }
            if let Some(violation) = instance.loop_proof_violation {
                let node = self
                    .program
                    .text(violation.repeat)
                    .map_or(Enc::Unencodable("a repeat node"), Enc::text);
                bounds.push((
                    Enc::tagged("loop-proof-violation", vec![("node", node)]),
                    Enc::tagged(
                        "failure",
                        vec![
                            (
                                "kind",
                                Enc::text(match violation.kind {
                                    ProtocolLoopProofKind::Invariant => "invariant",
                                    ProtocolLoopProofKind::Termination => "termination",
                                }),
                            ),
                            (
                                "failure",
                                Enc::text(match violation.failure {
                                    ProtocolLoopProofFailure::InvariantFalse => "invariant-false",
                                    ProtocolLoopProofFailure::Undefined => "undefined",
                                    ProtocolLoopProofFailure::VariantDidNotDecrease => {
                                        "variant-did-not-decrease"
                                    }
                                    ProtocolLoopProofFailure::VariantBelowFloor => {
                                        "variant-below-floor"
                                    }
                                    ProtocolLoopProofFailure::EvaluationIncomplete => {
                                        "evaluation-incomplete"
                                    }
                                    ProtocolLoopProofFailure::EvaluationRefused => {
                                        "evaluation-refused"
                                    }
                                }),
                            ),
                        ],
                    ),
                ));
            }
            for ((node, thread), counter) in &instance.message_ordinals {
                let node = self
                    .program
                    .text(*node)
                    .map_or(Enc::Unencodable("a send node"), Enc::text);
                bounds.push((
                    Enc::tagged(
                        "message-ordinal",
                        vec![("node", node), ("thread", thread.enc())],
                    ),
                    Enc::tagged("integer", vec![("value", Enc::text(counter.to_string()))]),
                ));
            }
            for ((node, thread), counter) in &instance.effect_ordinals {
                let node = self
                    .program
                    .text(*node)
                    .map_or(Enc::Unencodable("an effect node"), Enc::text);
                bounds.push((
                    Enc::tagged(
                        "business-effect-ordinal",
                        vec![("node", node), ("thread", thread.enc())],
                    ),
                    Enc::tagged("integer", vec![("value", Enc::text(counter.to_string()))]),
                ));
            }
            for (message, next) in &instance.next_deliveries {
                bounds.push((
                    Enc::tagged("next-delivery", vec![("message", message.enc())]),
                    Enc::tagged("integer", vec![("value", Enc::text(next.to_string()))]),
                ));
            }
            for (capture, bound) in self.program.captures.iter().zip(&instance.captures) {
                observations.push((
                    Enc::tagged(
                        "capture",
                        vec![
                            ("instance", Enc::text(ordinal.to_string())),
                            ("name", Enc::text(capture.name.clone())),
                        ],
                    ),
                    bound.enc.clone(),
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
                let attempts = registration
                    .attempts
                    .iter()
                    .map(|attempt| {
                        let mut members = vec![
                            ("record", attempt.record.clone()),
                            ("outcome", Enc::text(attempt.outcome)),
                        ];
                        if let Some(retry_of) = attempt.retry_of {
                            members.push(("retry_of", Enc::text(retry_of.to_string())));
                        }
                        Enc::tagged("compensation-attempt", members)
                    })
                    .collect();
                let mut registration_members = vec![
                    ("status", Enc::text(registration.status.text())),
                    ("captures", map_enc(captures)),
                    ("attempts", Enc::Array(attempts)),
                ];
                let recovery = match registration.status {
                    Status::Activated => Some("pending"),
                    Status::Recovered => Some("successful"),
                    Status::Unrecovered => Some("failed"),
                    Status::TimedOut => Some("timed-out"),
                    Status::Registered | Status::Closed => None,
                };
                if let Some(recovery) = recovery {
                    registration_members.push(("recovery", Enc::text(recovery)));
                }
                control.push((
                    named.enc(),
                    Enc::tagged("registration-state", registration_members),
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
                if let Some(attempt) = &compensating.attempt {
                    observations.push((
                        Enc::tagged("attempt-record", vec![("thread", thread.enc())]),
                        attempt.enc.clone(),
                    ));
                }
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
                    None if channel.send_ordered => ("sequence", Enc::Null),
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
                                                self.memory.carried_member(),
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
        reader: fsl_semantics::check::ProtocolNodeId,
    ) -> Vec<Option<Value>> {
        let mut frame = Vec::with_capacity(self.program.slots.len());
        for (slot, node) in self.program.slots.iter().enumerate() {
            let value = match node {
                None if slot == 0 => Some(instance.over.clone()),
                None if Some(slot) == self.program.activation_slot => instance
                    .trigger
                    .as_ref()
                    .filter(|_| self.program.reads(slot, reader))
                    .and_then(|bound| bound.value.clone()),
                None => self
                    .program
                    .capture_of(slot)
                    .and_then(|capture| instance.captures.get(capture))
                    .filter(|_| self.program.reads(slot, reader))
                    .and_then(|bound| bound.value.clone()),
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
        body: &fsl_semantics::check::CheckedBody,
        frame: Vec<Option<Value>>,
        observed: &ModelSimState,
    ) -> Result<Verdict, ExpansionStop> {
        self.evaluate_in(body, frame, &self.model.observation_of(observed)?)
    }

    /// Evaluate a checked loop-proof expression without coercing a scalar
    /// variant to a Boolean verdict.
    fn evaluate_loop_value(
        &self,
        body: &fsl_semantics::check::CheckedBody,
        frame: Vec<Option<Value>>,
        observed: &ModelSimState,
    ) -> Result<Result<Value, ProtocolLoopProofFailure>, ExpansionStop> {
        let observation = self.model.observation_of(observed)?;
        let evaluation = self
            .evaluator
            .evaluate_protocol_slot(
                body,
                frame,
                &observation.environment,
                &mut Meter::new(self.model.meter()),
            )
            .map_err(|_| broken())?;
        let result = match evaluation.outcome {
            FamilyOutcome::Evaluated(Outcome::Completed(value)) => Ok(value),
            FamilyOutcome::Evaluated(Outcome::Undefined(_))
            | FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(_)) => {
                Err(ProtocolLoopProofFailure::Undefined)
            }
            FamilyOutcome::Evaluated(Outcome::Incomplete(_)) => {
                Err(ProtocolLoopProofFailure::EvaluationIncomplete)
            }
            FamilyOutcome::Evaluated(Outcome::Refused(_))
            | FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(_)) => {
                Err(ProtocolLoopProofFailure::EvaluationRefused)
            }
        };
        Ok(result)
    }

    /// [`Self::evaluate`] over a thread's observation, whose synthesized
    /// observation is built once however many bodies read it.
    fn evaluate_observed(
        &self,
        body: &fsl_semantics::check::CheckedBody,
        frame: Vec<Option<Value>>,
        observed: &Observed<'_>,
    ) -> Result<Verdict, ExpansionStop> {
        self.evaluate_in(body, frame, self.model.observation_at(observed)?)
    }

    /// `body` with `frame`, read over `observation`.
    fn evaluate_in(
        &self,
        body: &fsl_semantics::check::CheckedBody,
        frame: Vec<Option<Value>>,
        observation: &Observation,
    ) -> Result<Verdict, ExpansionStop> {
        let evaluation = self
            .evaluator
            .evaluate_protocol_slot(
                body,
                frame,
                &observation.environment,
                &mut Meter::new(self.model.meter()),
            )
            .map_err(|_| broken())?;
        Ok(verdict(&evaluation.outcome))
    }

    /// Evaluate a protocol expression that yields a value rather than a
    /// Boolean predicate (FR-390, FR-731).
    fn evaluate_value_in(
        &self,
        body: &fsl_semantics::check::CheckedBody,
        frame: Vec<Option<Value>>,
        observation: &Observation,
    ) -> Result<Result<Value, ClauseOutcome>, ExpansionStop> {
        let evaluation = self
            .evaluator
            .evaluate_protocol_slot(
                body,
                frame,
                &observation.environment,
                &mut Meter::new(self.model.meter()),
            )
            .map_err(|_| broken())?;
        Ok(match evaluation.outcome {
            filament_eval::FamilyOutcome::Evaluated(outcome) => match outcome {
                filament_exact::Outcome::Completed(value) => Ok(value),
                filament_exact::Outcome::Undefined(undefined) => {
                    Err(ClauseOutcome::KernelUndefined(undefined))
                }
                filament_exact::Outcome::Refused(refusal) => {
                    Err(ClauseOutcome::KernelRefused(refusal))
                }
                filament_exact::Outcome::Incomplete(incomplete) => {
                    Err(ClauseOutcome::Incomplete(incomplete))
                }
            },
            filament_eval::FamilyOutcome::FamilyEvaluated(family) => Err(match family {
                filament_eval::FamilyResult::Refused(cause) => {
                    ClauseOutcome::FamilyRefused(cause.catalog_code())
                }
                filament_eval::FamilyResult::Undefined(cause) => {
                    ClauseOutcome::FamilyUndefined(cause.undefined_record().reason)
                }
            }),
        })
    }

    /// Evaluate a package control constraint from admitted binder identities.
    /// A package expression receives the state values by node identity and
    /// never crosses through a synthetic `CheckedBody`.
    pub(super) fn evaluate_package_control(
        &self,
        node: fsl_semantics::check::ProtocolNodeId,
        frame: Vec<Option<Value>>,
        state: &ModelSimState,
    ) -> Result<Option<Verdict>, ExpansionStop> {
        let Some(package) = self.package_program.as_ref() else {
            return Ok(None);
        };
        let Some(control) = package.controls.get(&node) else {
            return Ok(None);
        };
        let Some(slot) = control.constraint.as_ref() else {
            return Ok(None);
        };
        let observation = self.package_observation(state)?;
        let binders = self.package_binder_values(frame, &observation, slot)?;
        let supply = package
            .supply_for_slot(slot, binders, Some(&observation))
            .map_err(|failure| {
                eprintln!("RECEIVE-FIFO-DIAG control-supply node={node:?} {failure:?}");
                broken()
            })?;
        let result = package
            .evaluate_control_constraint(node, &supply, &mut Meter::new(self.model.meter()))
            .map_err(|failure| {
                eprintln!("RECEIVE-FIFO-DIAG control-evaluate node={node:?} {failure:?}");
                broken()
            })?;
        package_slot_verdict(result).map(Some)
    }

    /// Evaluate the package finish constraint over its finish record.
    pub(super) fn evaluate_package_finish(
        &self,
        frame: Vec<Option<Value>>,
        state: &ModelSimState,
    ) -> Result<Option<Verdict>, ExpansionStop> {
        let Some(package) = self.package_program.as_ref() else {
            return Ok(None);
        };
        let Some(slot) = package.finish.as_ref() else {
            return Ok(None);
        };
        let observation = self.package_observation(state)?;
        let binders = self.package_binder_values(frame, &observation, slot)?;
        let supply = package
            .supply_for_slot(slot, binders, Some(&observation))
            .map_err(|_| broken())?;
        let result = package
            .evaluate_finish(&supply, &mut Meter::new(self.model.meter()))
            .map_err(|_| broken())?;
        result
            .map(|evaluation| package_slot_verdict(Some(evaluation)))
            .transpose()
    }

    fn package_observation(
        &self,
        state: &ModelSimState,
    ) -> Result<
        filament_semantic_value::admission::ObservationAdmission<
            fsl_package::AdmittedProtocolObservation<'static>,
        >,
        ExpansionStop,
    > {
        let subject = self.package_subject.ok_or_else(broken)?;
        let request = &subject.model.simulation;
        let package = self.package_program.as_ref().ok_or_else(broken)?;
        let ValueType::Reference(over) = &package.over_type else {
            return Err(broken());
        };
        let declaring = self
            .model
            .declaring_package(over.clone())
            .ok_or_else(broken)?;
        let selection = request
            .input
            .model_selections
            .iter()
            .find(|selection| selection.identity == declaring)
            .ok_or_else(broken)?;
        self.evaluator
            .package_protocol_observation(
                selection,
                request.packages,
                request.normalization,
                state.state(),
                self.model.observation_of(state)?.identity,
            )
            .map_err(|failure| {
                eprintln!("RECEIVE-FIFO-DIAG observation {failure:?}");
                broken()
            })
    }

    fn package_binder_values(
        &self,
        frame: Vec<Option<Value>>,
        observation: &filament_semantic_value::admission::ObservationAdmission<
            fsl_package::AdmittedProtocolObservation<'_>,
        >,
        slot: &crate::value::ProtocolSlot,
    ) -> Result<
        Vec<(
            fsl_package::CheckedNodeId,
            crate::value::ProtocolSuppliedValue,
        )>,
        ExpansionStop,
    > {
        let binders = self
            .program
            .package_binders
            .iter()
            .zip(frame)
            .filter_map(|(binder, value)| binder.as_ref().map(|binder| (binder, value)));
        self.package_supplied_binders(binders, observation, slot)
    }

    /// Admit values from either a live protocol frame or a FIFO key's own
    /// payload binder through the same reference-population provenance.
    fn package_supplied_binders<'b>(
        &self,
        binders: impl IntoIterator<Item = (&'b fsl_package::CheckedNodeId, Option<Value>)>,
        observation: &filament_semantic_value::admission::ObservationAdmission<
            fsl_package::AdmittedProtocolObservation<'_>,
        >,
        slot: &crate::value::ProtocolSlot,
    ) -> Result<
        Vec<(
            fsl_package::CheckedNodeId,
            crate::value::ProtocolSuppliedValue,
        )>,
        ExpansionStop,
    > {
        let filament_semantic_value::admission::ObservationAdmission::Admitted(observation) =
            observation
        else {
            use filament_semantic_value::admission::ObservationAdmission;
            match observation {
                ObservationAdmission::Refused(record) => {
                    eprintln!("RECEIVE-FIFO-DIAG supply-observation-refused {record:?}")
                }
                ObservationAdmission::Incomplete(record) => {
                    eprintln!("RECEIVE-FIFO-DIAG supply-observation-incomplete {record:?}")
                }
                ObservationAdmission::Fault(failure) => {
                    eprintln!("RECEIVE-FIFO-DIAG supply-observation-fault {failure:?}")
                }
                ObservationAdmission::Admitted(_) => {}
            }
            return Err(broken());
        };
        let subject = self.package_subject.ok_or_else(broken)?;
        let package = self.package_program.as_ref().ok_or_else(broken)?;
        let required: BTreeSet<_> = package
            .slot_binders(slot)
            .map_err(|failure| {
                eprintln!("RECEIVE-FIFO-DIAG supply-binders {failure:?}");
                broken()
            })?
            .into_iter()
            .collect();
        binders
            .into_iter()
            .filter(|(binder, _)| {
                self.evaluator
                    .package_node_key(binder)
                    .is_some_and(|key| required.contains(&key))
            })
            .map(|(binder, value)| {
                let mut references = Vec::new();
                let value = match value {
                    Some(value) => {
                        for (path, reference) in crate::value::reference_occurrences(
                            &value,
                            subject.model.simulation.input.types,
                        )
                        .map_err(|_| {
                            eprintln!("RECEIVE-FIFO-DIAG reference-walk binder={binder:?}");
                            broken()
                        })?
                        {
                            let environment = observation.environment();
                            let population = if binder == &package.over {
                                fsl_semantics::model::key::population_node_key(
                                    &subject.over.population,
                                )
                                .map_err(|_| broken())?
                            } else {
                                environment
                                    .population_completeness()
                                    .keys()
                                    .find(|population| {
                                        environment
                                            .population_members(population)
                                            .is_some_and(|members| members.contains(&reference))
                                    })
                                    .and_then(|population| environment.population_node(population))
                                    .cloned()
                                    .ok_or_else(|| {
                                        eprintln!("RECEIVE-FIFO-DIAG reference-population binder={binder:?}");
                                        broken()
                                    })?
                            };
                            references.push(crate::value::ProtocolReferenceOccurrence {
                                path,
                                population,
                            });
                        }
                        crate::value::SuppliedValue::Value(value)
                    }
                    None => crate::value::SuppliedValue::Unavailable(BROKEN),
                };
                Ok((
                    binder.clone(),
                    crate::value::ProtocolSuppliedValue { value, references },
                ))
            })
            .collect()
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
        match &self.program.terminal {
            Terminal::None => Ok(TerminalClass::Deadlocked),
            Terminal::Any => Ok(TerminalClass::Intended),
            terminal @ (Terminal::SourceWhen(_) | Terminal::PackageWhen(_)) => {
                let mut frame = vec![None; self.program.slots.len()];
                if let Some(first) = frame.first_mut() {
                    *first = Some(state.over.clone());
                }
                // Only `on origin` lets the predicate read the captures
                // (FR-381), and then the one instance is ordinal 0.
                if let Some(instance) = state.instances.get(&0) {
                    for (index, bound) in instance.captures.iter().enumerate() {
                        let slot = self.program.capture_slots.get(index).copied();
                        if let Some(entry) = slot.and_then(|slot| frame.get_mut(slot)) {
                            *entry = bound.value.clone();
                        }
                    }
                }
                let observed = self.memory.atom_values(&state.memory, &state.model);
                let verdict = match terminal {
                    Terminal::SourceWhen(predicate) => {
                        self.evaluate(predicate, frame, &observed)?
                    }
                    Terminal::PackageWhen(slot) => {
                        let package = self.package_program.as_ref().ok_or_else(broken)?;
                        let observation = self.package_observation(&observed)?;
                        let binders = self.package_binder_values(frame, &observation, slot)?;
                        let supply = package
                            .supply_for_slot(slot, binders, Some(&observation))
                            .map_err(|_| broken())?;
                        let result = self
                            .evaluator
                            .evaluate_package_protocol_slot(
                                slot,
                                &supply,
                                &mut Meter::new(self.model.meter()),
                            )
                            .map_err(|_| broken())?;
                        package_slot_verdict(Some(result))?
                    }
                    Terminal::None | Terminal::Any => return Err(broken()),
                };
                Ok(match verdict {
                    Verdict::True => TerminalClass::Intended,
                    Verdict::False => TerminalClass::Deadlocked,
                    Verdict::Undecided(outcome) => {
                        if let Some(cause) = outcome.undefined_cause() {
                            TerminalClass::Undefined {
                                model: super::model::TerminalModel::named(
                                    self.protocol_name.clone(),
                                ),
                                cause,
                            }
                        } else if let ClauseOutcome::Incomplete(incomplete) = outcome {
                            return Err(ExpansionStop {
                                cause: CatalogCode::new(
                                    "resource_exhausted",
                                    "insufficient-next-charge",
                                ),
                                limit: Some(ExpansionLimit::EvaluationMeter(incomplete.limit_kind)),
                            });
                        } else {
                            TerminalClass::Undecided {
                                model: super::model::TerminalModel::named(
                                    self.protocol_name.clone(),
                                ),
                            }
                        }
                    }
                })
            }
        }
    }
}

fn package_slot_verdict(
    result: Option<crate::value::ProtocolSlotEvaluation>,
) -> Result<Verdict, ExpansionStop> {
    match result {
        None => Ok(Verdict::True),
        Some(crate::value::ProtocolSlotEvaluation::Evaluated(evaluation, _)) => {
            Ok(verdict(&evaluation.outcome))
        }
        Some(crate::value::ProtocolSlotEvaluation::Unavailable { .. }) => Err(broken()),
    }
}

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
        let mut expansion = self.expand_bounded(state)?.0;
        self.canonical_successors(&mut expansion)?;
        Ok(expansion)
    }
}

impl<M: MemoryModel> SubjectSystem for ProtocolSystem<'_, M> {
    fn loop_proof_violation<'s>(
        &self,
        state: &'s ProtocolState<M>,
    ) -> Option<&'s ProtocolLoopProofViolation> {
        ProtocolSystem::loop_proof_violation(self, state)
    }

    fn state_key(
        &self,
        state: &Self::State,
    ) -> Result<(Vec<u8>, crate::simulation::StateKey), EncodingRefusal> {
        let bytes = self.key_bytes(state)?;
        Ok((
            bytes.clone(),
            crate::simulation::StateKey::from_canonical_bytes(bytes),
        ))
    }

    fn canonical_state_key(
        &self,
        state: &Self::State,
    ) -> Result<crate::simulation::StateKey, EncodingRefusal> {
        self.key_bytes(state)
            .map(crate::simulation::StateKey::from_canonical_bytes)
    }

    fn loop_state_key(
        &self,
        state: &Self::State,
    ) -> Result<crate::simulation::StateKey, EncodingRefusal> {
        // Checked temporal atoms currently have no message- or delivery-index
        // read form; keep the explicit seams for future checked-item reads.
        self.cycle_key_with_reads(
            state,
            self.message_reads.as_ref().unwrap_or(&BTreeSet::new()),
            self.delivery_reads.as_ref().unwrap_or(&BTreeSet::new()),
        )
    }

    fn concrete_cycle_step(
        &self,
        source: &Self::State,
        transition: &Self::TransitionId,
        target: &Self::State,
    ) -> Result<Option<(Self::TransitionId, Self::State)>, ExpansionStop> {
        ProtocolSystem::concrete_cycle_step(self, source, transition, target)
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
        authored: &[filament_semantic_value::fairness::FairnessConstraint],
        transitions: &[ProtocolTransition],
        identity: &dyn Fn(&ProtocolTransition) -> Result<Identity, ZeroProgressError>,
    ) -> Result<Vec<Constraint>, ZeroProgressError> {
        let members = self.fairness_set(authored)?;
        Ok(protocol_constraints(&members, transitions, identity)?
            .into_iter()
            .map(|(_, constraint)| constraint)
            .collect())
    }

    fn clause_constraints(
        &self,
        authored: &[filament_semantic_value::fairness::FairnessConstraint],
        transitions: &[ProtocolTransition],
        identity: &dyn Fn(&ProtocolTransition) -> Option<Identity>,
    ) -> Result<Vec<Constraint>, ConstraintError> {
        let members = self.fairness_set(authored)?;
        Ok(protocol_constraints(&members, transitions, |transition| {
            identity(transition).ok_or(ConstraintError::Unnumbered)
        })?
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
        let (mut expansion, cut) = self.expand_bounded(state)?;
        self.canonical_successors(&mut expansion)?;
        Ok((expansion, cut))
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

impl<M: MemoryModel> ProtocolSystem<'_, M> {
    /// The RFC 8785 bytes of `state`'s key ([`TransitionSystem::key`]),
    /// written without the event writer: the members through the model's
    /// canonical writer, the memory member through [`MemoryModel::key_bytes`].
    fn key_bytes(&self, state: &ProtocolState<M>) -> Result<Vec<u8>, EncodingRefusal> {
        let identity = identity_limits();
        let limits = quire_canonical::Limits::new(identity.input_bytes);
        let canonical = self.model.canonical();
        let memory = self.memory.key_bytes(&state.memory, canonical, limits);
        Envelope {
            members: &self.key_members(state),
            memory: &state.memory,
        }
        .to_vec(canonical, memory.as_deref(), limits)
        .map_err(|error| EncodingRefusal::of(&error, identity))
    }
}

/// The identity limits exploration encodes under.
fn identity_limits() -> fsl_foundation::IdentityLimits {
    fsl_foundation::IdentityLimits::default()
}

/// FR-120 "Contract clauses": an undecided protocol slot as data.
fn undetermined(node: &str, outcome: crate::simulation::model::ClauseOutcome) -> ModelFinding {
    ModelFinding::ProtocolExpressionUndetermined {
        node: node.to_owned(),
        evaluation: outcome,
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use quire_canonical::Limits;

    use super::*;

    /// The envelope's directly written result, bytes or refusal, against
    /// `quire_canonical::to_vec` of the same envelope.
    fn written_as_encoded(
        envelope: &Envelope<'_, Enc>,
        memory: Option<&[u8]>,
        bound: u64,
    ) -> Result<Vec<u8>, Error> {
        let limits = Limits::new(bound);
        let written = envelope.to_vec(&Canonicalizer::default(), memory, limits);
        let encoded = quire_canonical::to_vec(envelope, limits);
        assert_eq!(
            format!("{written:?}"),
            format!("{encoded:?}"),
            "under {bound} bytes"
        );
        written
    }

    /// FR-101-AC-2 and FR-101-AC-11: a protocol state key written from its
    /// members and the memory model's own bytes is the key's RFC 8785
    /// encoding, and refuses as that encoding does: a member or a memory
    /// component with no encoding, a member named `memory`, or a key past
    /// the byte bound.
    #[trace("TC-453", "FR-101-AC-2", "FR-101-AC-11")]
    #[test]
    fn a_protocol_key_is_written_and_refused_as_it_encodes() {
        let members = vec![
            ("type", Enc::text("simulation-state")),
            (
                "control",
                map_enc(vec![
                    (Enc::text("b"), Enc::Null),
                    (Enc::text("a"), Enc::Null),
                ]),
            ),
        ];
        let memory = Enc::Object(vec![("z", Enc::Null), ("y", Enc::text("\n"))]);
        let memory_bytes =
            quire_canonical::to_vec(&memory, Limits::new(u64::MAX)).expect("the memory encodes");
        let envelope = Envelope {
            members: &members,
            memory: &memory,
        };
        let length = quire_canonical::to_vec(&envelope, Limits::new(u64::MAX))
            .expect("the key encodes")
            .len();
        let length = u64::try_from(length).expect("a length fits u64");
        for bound in 0..length {
            written_as_encoded(&envelope, Some(&memory_bytes), bound)
                .expect_err("a key past the bound refuses");
        }
        written_as_encoded(&envelope, Some(&memory_bytes), length).expect("the key encodes");
        // The memory model wrote no bytes: the envelope's encoding decides.
        written_as_encoded(&envelope, None, u64::MAX).expect("the key encodes");

        let unencodable_memory = Enc::Unencodable("a memory component");
        written_as_encoded(
            &Envelope {
                members: &members,
                memory: &unencodable_memory,
            },
            None,
            u64::MAX,
        )
        .expect_err("the key has no encoding");
        let mut refused = members.clone();
        refused.push(("rest", Enc::Unencodable("a rest point")));
        written_as_encoded(
            &Envelope {
                members: &refused,
                memory: &memory,
            },
            Some(&memory_bytes),
            u64::MAX,
        )
        .expect_err("the key has no encoding");
        let mut duplicated = members;
        duplicated.push(("memory", Enc::Null));
        written_as_encoded(
            &Envelope {
                members: &duplicated,
                memory: &memory,
            },
            Some(&memory_bytes),
            u64::MAX,
        )
        .expect_err("the key has no encoding");
    }
}
