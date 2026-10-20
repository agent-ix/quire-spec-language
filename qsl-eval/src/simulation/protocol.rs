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
//! `fence` (SE-5), channel `duplicate` and `lose`, and `finish`, under
//! `activation on origin` (ADR-027 §9 steps 3 and 4). A protocol that holds any other construct is
//! refused, before any state is built, with
//! `unsupported_construct`/`not-yet-implemented` naming it.

mod channel;
mod footprint;
mod identity;
mod memory;
mod program;
mod settle;
mod steps;

use std::borrow::Cow;
use std::collections::BTreeMap;

use qsl_foundation::diagnostic::{Code, InternalFault};
use qsl_foundation::digest::{DigestRecord, WireNodeId};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckedProtocol, CheckedTerminalMember};
use quire_canonical::{Encode, Error, Limits as CanonicalLimits, Sink, Writer};
use quire_exact::{Meter, Value, ValueType};

use crate::model_subject::{ModelInstance, ProtocolSubject};
use crate::simulation::expansion::{Expansion, ExpansionLimit, ExpansionStop};
use crate::simulation::explore::TransitionSystem;
use crate::simulation::key::{self, EncodingRefusal};
use crate::simulation::model::encode::{map_enc, Enc};
use crate::simulation::model::{
    verdict, ModelFinding, ModelSimState, ModelSystem, TerminalClass, Verdict, BROKEN,
};
use crate::simulation::not_simulated::{check_requires_bound, NotSimulated};
use crate::simulation::subject::SubjectSystem;
use crate::value::evaluate_protocol_slot;

pub use footprint::{
    visible, AnchorName, ClaimReads, ParallelInstance, ProtocolFootprintError, ProtocolLocation,
};
pub use identity::{
    BlockedThread, PathStep, ProtocolTransition, RestKind, RestPoint, SendOutcome, SendRecord,
    StepKind, ThreadPath, WaitCause,
};
pub use memory::{GatedStep, MemoryModel, MemoryStep, Sc, ScComponent};
use program::{Program, Rest};

/// A construct this protocol system does not step yet.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Unsupported {
    /// A `run` control of this kind, by its keyword.
    Control(&'static str),
    /// A `parallel` whose resolved memory model is not `sc`.
    WeakMemory,
    /// A channel whose capacity is `symbolic(..)`, with no bound to read.
    SymbolicCapacity,
    /// An `unordered` channel whose overflow loses its `head` or `tail`,
    /// which a bag does not order.
    UnorderedLossSelector,
    /// A `choice` guard, `repeat` guard or `check` undecided at an initial
    /// state, before any position a finding can name.
    UndecidedInitialGuard,
    /// A `compensate` obligation.
    Compensation,
    /// A protocol capture.
    Capture,
    /// `activation on each`.
    ActivationOnEach,
    /// A replicated role.
    ReplicatedRole,
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
            Self::SymbolicCapacity => "symbolic channel capacity",
            Self::UnorderedLossSelector => "ordered loss selector on an unordered channel",
            Self::UndecidedInitialGuard => "undecided guard at an initial state",
            Self::Compensation => "compensation",
            Self::Capture => "capture",
            Self::ActivationOnEach => "activation on each",
            Self::ReplicatedRole => "replicated role",
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
            Self::UnknownProtocol { .. } | Self::RoleMapping { .. } | Self::OverBinding { .. } => {
                Code::InvalidRuntimeInput
            }
            Self::Unsupported(_) => Code::UnsupportedConstruct,
            Self::InitialStop(stop) => {
                Code::from_code(stop.cause.code()).unwrap_or(Code::RuntimeInvariant)
            }
            Self::Fault(_) => Code::RuntimeInvariant,
        }
    }

    /// The refusal's catalog cause.
    pub fn cause(&self) -> &'static str {
        match self {
            Self::UnknownProtocol { .. } | Self::OverBinding { .. } => "invalid-value",
            Self::RoleMapping { .. } => "wrong-role-mapping",
            Self::Unsupported(_) => "not-yet-implemented",
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

/// One bound binder value (ADR-027 PS-5).
#[derive(Clone, Debug)]
struct Bound {
    value: Value,
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
}

/// One live protocol instance (ADR-027 AE-2).
#[derive(Clone, Debug, Default)]
struct Instance {
    threads: BTreeMap<ThreadPath, Thread>,
    parallels: BTreeMap<ParallelKey, Parallel>,
    /// Every live thread's binder environment (ADR-027 PS-5), by (binder
    /// node, the thread whose environment holds the entry): each thread
    /// reads its own entry, and holds its own copy of every value it reads.
    environments: BTreeMap<(qsl_semantics::check::ProtocolNodeId, ThreadPath), Bound>,
    /// Each `repeat` counter, by (repeat node, thread) (ADR-027 PS-6).
    counters: BTreeMap<(qsl_semantics::check::ProtocolNodeId, ThreadPath), u64>,
    /// Each non-empty queue, a `fifo` one in delivery order and an
    /// `unordered` one in ascending JCS bytes of its entries (PS-7).
    queues: BTreeMap<QueueKey, Vec<Entry>>,
}

impl Instance {
    /// The value the thread `reader` reads for the binder of `node`: its own
    /// environment's entry (ADR-027 PS-5).
    fn entry(
        &self,
        node: qsl_semantics::check::ProtocolNodeId,
        reader: &ThreadPath,
    ) -> Option<&Bound> {
        self.environments.get(&(node, reader.clone()))
    }

    /// The entries of the thread `holder`'s environment.
    fn environment<'i>(
        &'i self,
        holder: &'i ThreadPath,
    ) -> impl Iterator<Item = (qsl_semantics::check::ProtocolNodeId, &'i Bound)> + 'i {
        self.environments
            .iter()
            .filter(move |((_, thread), _)| thread == holder)
            .map(|((node, _), bound)| (*node, bound))
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

/// FR-215: the transition system of a protocol subject.
pub struct ProtocolSystem<'a, M: MemoryModel> {
    model: ModelSystem<'a>,
    package: &'a CheckedPackage,
    protocol: &'a CheckedProtocol,
    program: Program<'a>,
    memory: M,
    /// The `over` parameter's bound value.
    over: Value,
    /// Each static role's binding: role name, then model identity.
    roles: Vec<(String, String)>,
    initial: Vec<ProtocolState<M>>,
}

impl<'a, M: MemoryModel> ProtocolSystem<'a, M> {
    /// FR-205 "Subject": build the protocol subject's system over the
    /// admitted `model` of the subject's model subject, with the memory
    /// model `memory`, and its initial protocol states (FO-3).
    ///
    /// # Errors
    ///
    /// [`ProtocolRefusal`] as FR-205 "Subject" states, or for a construct
    /// this system does not step.
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
        let program = Program::new(protocol)?;
        let mut system = Self {
            model,
            package,
            protocol,
            program,
            memory,
            over,
            roles,
            initial: Vec::new(),
        };
        system.initial = system.initial_states()?;
        Ok(system)
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
    /// settled from the start of `run`.
    fn initial_states(&self) -> Result<Vec<ProtocolState<M>>, ProtocolRefusal> {
        let mut states = Vec::new();
        let root = ThreadPath::root(0);
        for model in self.model.initial() {
            let mut instance = Instance::default();
            let rest = match self
                .settle(
                    &mut instance,
                    &root,
                    program::Cursor::Enter(self.program.root),
                    &model,
                )
                .map_err(|error| match error {
                    settle::SettleError::Refused(refusal) => refusal,
                    settle::SettleError::Stopped(stop) => ProtocolRefusal::InitialStop(stop),
                })? {
                settle::Settled::Rest(rest) => rest,
                settle::Settled::Undecided(_) => {
                    return Err(ProtocolRefusal::Unsupported(
                        Unsupported::UndecidedInitialGuard,
                    ))
                }
            };
            instance.threads.insert(
                root.clone(),
                Thread {
                    rest,
                    disposition: Disposition::Running,
                    owner: None,
                },
            );
            states.push(ProtocolState {
                model,
                instances: BTreeMap::from([(0, instance)]),
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
        for (node, thread) in state
            .instances
            .values()
            .flat_map(|instance| instance.environments.keys())
        {
            binders.push((self.program.text(*node)?.to_owned(), thread.clone()));
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
            Rest::End => Ok(RestPoint {
                kind: RestKind::End,
                node: None,
            }),
        }
    }

    /// ADR-027 PS-8: the key members of `state`, the memory component
    /// apart.
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
                control.push((
                    path.enc(),
                    Enc::tagged(
                        "thread-state",
                        vec![
                            ("rest", rest),
                            ("disposition", Enc::text(thread.disposition.text())),
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
            for ((node, thread), bound) in &instance.environments {
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
                            Enc::Array(entries.iter().map(|entry| entry.enc.clone()).collect()),
                        )],
                    ),
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
                // Refused in `Program::new` when any slot reads one.
                None => None,
                Some(_) if !self.program.reads(slot, reader) => None,
                Some(node) => instance.entry(*node, path).map(|bound| bound.value.clone()),
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
        has_successor: bool,
    ) -> Result<TerminalClass, ExpansionStop> {
        if has_successor {
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
        self.expand(state)
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
        has_successor: bool,
    ) -> Result<TerminalClass, ExpansionStop> {
        self.classify(state, has_successor)
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
