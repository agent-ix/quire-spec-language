// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-425, FR-426 and FR-429's typed canonical forms (ADR-027 PS-2,
//! TS-2, PD-4): thread paths, rest points, protocol transition identities
//! and blocked threads.
//!
//! A node is named by its static declaration path: the names of the `run`
//! tree's controls from the root down to it, joined by `/`, or the
//! `finish` node's own name.

use quire_canonical::{Encode, Error, Sink, Writer};
use quire_exact::ObjectReference;
use quire_semantic_value::model_transition::ModelTransition;

use super::memory::{MemoryChoice, MemoryStepId};
use crate::simulation::model::encode::{reference_enc, Enc};

/// One step of a thread's path (QSpec FR-425): for a branch thread, the
/// `parallel` that forked it, its branch label and the parallel instance's
/// ordinal; for a compensation thread, its template and its ordinal
/// (ADR-027 PS-2).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PathStep {
    /// A branch thread's step.
    Branch {
        /// The `parallel` node's static declaration path.
        node: String,
        /// The branch label.
        branch: String,
        /// The ordinal (ADR-027 PS-2).
        ordinal: u64,
    },
    /// A compensation thread's step (ADR-027 CP-3).
    Compensation {
        /// The template's qualified name.
        template: String,
        /// The ordinal (ADR-027 PS-2).
        ordinal: u64,
    },
}

impl PathStep {
    /// A branch step's `parallel` node; `None` for a compensation step.
    pub fn node(&self) -> Option<&str> {
        match self {
            Self::Branch { node, .. } => Some(node),
            Self::Compensation { .. } => None,
        }
    }

    /// A branch step's label; `None` for a compensation step.
    pub fn branch(&self) -> Option<&str> {
        match self {
            Self::Branch { branch, .. } => Some(branch),
            Self::Compensation { .. } => None,
        }
    }

    /// A compensation step's template; `None` for a branch step.
    pub fn template(&self) -> Option<&str> {
        match self {
            Self::Compensation { template, .. } => Some(template),
            Self::Branch { .. } => None,
        }
    }

    /// The step's ordinal.
    pub fn ordinal(&self) -> u64 {
        match self {
            Self::Branch { ordinal, .. } | Self::Compensation { ordinal, .. } => *ordinal,
        }
    }

    /// The bytes of the names the step holds.
    pub fn text_len(&self) -> usize {
        match self {
            Self::Branch { node, branch, .. } => node.len().saturating_add(branch.len()),
            Self::Compensation { template, .. } => template.len(),
        }
    }

    pub(crate) fn enc(&self) -> Enc {
        match self {
            Self::Branch {
                node,
                branch,
                ordinal,
            } => Enc::Object(vec![
                ("node", Enc::text(node.clone())),
                ("branch", Enc::text(branch.clone())),
                ("ordinal", Enc::text(ordinal.to_string())),
            ]),
            Self::Compensation { template, ordinal } => Enc::Object(vec![
                ("template", Enc::text(template.clone())),
                ("ordinal", Enc::text(ordinal.to_string())),
            ]),
        }
    }
}

/// A thread's path (ADR-027 PS-2): its protocol instance's ordinal, then
/// one [`PathStep`] per `parallel` above it, outermost first, or, for a
/// compensation thread, its one compensation step.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ThreadPath {
    /// The protocol instance's ordinal.
    pub instance: u64,
    /// The steps, outermost first.
    pub steps: Vec<PathStep>,
}

impl ThreadPath {
    /// The root thread of instance `instance`.
    pub fn root(instance: u64) -> Self {
        Self {
            instance,
            steps: Vec::new(),
        }
    }

    /// The branch thread `branch` of the parallel instance `ordinal` of
    /// `node` forked by this thread.
    pub fn child(&self, node: &str, branch: &str, ordinal: u64) -> Self {
        let mut steps = self.steps.clone();
        steps.push(PathStep::Branch {
            node: node.to_owned(),
            branch: branch.to_owned(),
            ordinal,
        });
        Self {
            instance: self.instance,
            steps,
        }
    }

    /// The compensation thread of template `template` with ordinal
    /// `ordinal` of protocol instance `instance` (ADR-027 PS-2, CP-3).
    pub fn compensation(instance: u64, template: &str, ordinal: u64) -> Self {
        Self {
            instance,
            steps: vec![PathStep::Compensation {
                template: template.to_owned(),
                ordinal,
            }],
        }
    }

    /// The thread that forked this one; `None` for a root thread.
    pub fn parent(&self) -> Option<Self> {
        let (_, rest) = self.steps.split_last()?;
        Some(Self {
            instance: self.instance,
            steps: rest.to_vec(),
        })
    }

    /// Whether this path is `other` or one of its ancestors.
    pub fn is_prefix_of(&self, other: &Self) -> bool {
        self.instance == other.instance && other.steps.starts_with(&self.steps)
    }

    pub(crate) fn enc(&self) -> Enc {
        Enc::tagged(
            "thread-path",
            vec![
                ("instance", Enc::text(self.instance.to_string())),
                (
                    "steps",
                    Enc::Array(self.steps.iter().map(PathStep::enc).collect()),
                ),
            ],
        )
    }
}

impl Encode for ThreadPath {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        self.enc().encode_into(writer)
    }
}

/// What a thread rests at (QSpec FR-425 `<rest>`).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RestKind {
    /// An event node, the `finish` node, or a compensation thread's
    /// compensation attempt, named by its template.
    Node,
    /// A `parallel` waiting to fork.
    Fork,
    /// The join of a `parallel` the thread forked.
    Join,
    /// A compensation thread's retry point (ADR-027 CP-3).
    RetryPoint,
    /// The end of a branch body.
    End,
}

impl RestKind {
    fn text(self) -> &'static str {
        match self {
            Self::Node => "node",
            Self::Fork => "fork",
            Self::Join => "join",
            Self::RetryPoint => "retry-point",
            Self::End => "end",
        }
    }
}

/// A thread's rest point (ADR-027 PS-3): its kind and node, none for
/// `end`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RestPoint {
    /// The kind.
    pub kind: RestKind,
    /// The node's static declaration path; `None` only for `end`.
    pub node: Option<String>,
}

impl RestPoint {
    pub(crate) fn enc(&self) -> Enc {
        Enc::tagged(
            "rest",
            vec![
                ("kind", Enc::text(self.kind.text())),
                ("node", self.node.clone().map_or(Enc::Null, Enc::Str)),
            ],
        )
    }
}

/// The kind of a protocol step (QSpec FR-426 `step`).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StepKind {
    /// ST-1.
    Attempt,
    /// ST-2: a domain `event` or `commit`.
    Event,
    /// ST-3.
    Send,
    /// ST-4.
    Receive,
    /// ST-5: a copy of a queued entry.
    Duplicate,
    /// ST-5: the loss of a queued entry.
    Lose,
    /// ST-6.
    Fork,
    /// ST-7.
    Join,
    /// ST-8.
    Timeout,
    /// ST-9.
    Finish,
    /// ST-10: a step of the memory model.
    Memory,
    /// SE-5: a `fence`.
    Fence,
    /// ST-11: a compensation attempt.
    CAttempt,
    /// ST-12: the end of a compensation thread.
    CEnd,
    /// ST-13: the start of a protocol instance by a trigger record.
    Activate,
    /// ST-14: an object becomes an instance of a replicated role.
    Spawn,
    /// ST-15: an instance of a replicated role retires.
    Retire,
}

impl StepKind {
    fn text(self) -> &'static str {
        match self {
            Self::Attempt => "attempt",
            Self::Event => "event",
            Self::Send => "send",
            Self::Receive => "receive",
            Self::Duplicate => "duplicate",
            Self::Lose => "lose",
            Self::Fork => "fork",
            Self::Join => "join",
            Self::Timeout => "timeout",
            Self::Finish => "finish",
            Self::Memory => "memory",
            Self::Fence => "fence",
            Self::CAttempt => "cattempt",
            Self::CEnd => "cend",
            Self::Activate => "activate",
            Self::Spawn => "spawn",
            Self::Retire => "retire",
        }
    }
}

/// The members a step's identity carries beside its kind.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Members {
    /// `operation`, `receiver` and `arguments`, with the FR-120 identity of
    /// the application they name.
    Attempt {
        application: ModelTransition,
        operation: String,
        receiver: Enc,
        arguments: Vec<Enc>,
        /// `memory_choice`: the message or slot the memory model chose.
        choice: Option<MemoryChoice>,
    },
    /// No further member.
    Control,
    /// `record`: the bound record or payload.
    Record { record: Enc },
    /// `channel` and `entry`, with the channel's qualified name and its
    /// protocol instance's ordinal that `channel` encodes.
    Channel {
        name: String,
        instance: u64,
        entry: Enc,
    },
    /// `memory_step`, the typed identity its memory model gives.
    Memory { identity: MemoryStepId },
    /// `ordering`: a `fence`'s ordering (QSpec FR-426).
    Fence { ordering: &'static str },
    /// A `cattempt`'s `operation`, `receiver` and `arguments`, with the
    /// FR-120 identity of the application, and its `template` and
    /// `registration`.
    CAttempt {
        application: ModelTransition,
        operation: String,
        receiver: Enc,
        arguments: Vec<Enc>,
        template: String,
        registration: RegistrationKey,
    },
    /// A `cend`'s `template` and `registration`.
    CEnd {
        template: String,
        registration: RegistrationKey,
    },
    /// A `spawn` or `retire`: the `role`, the `object`'s reference and the
    /// `instance` ordinal of the protocol instance that owns the role
    /// instance.
    Role {
        role: String,
        object: ObjectReference,
        instance: u64,
    },
}

/// The instance of a replicated role that takes a step `by` the role
/// (ADR-027 RR-2, TS-2): the role, which a claim names (FT-5), and the
/// instance's object reference, which the identity's `actor` member is.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Actor {
    /// The replicated role.
    pub role: String,
    /// The acting instance's object.
    pub object: ObjectReference,
}

/// A compensation registration's key (ADR-027 CP-1, QSpec FR-425): its
/// template's qualified name, the path of the thread whose effect step
/// registered it, and its ordinal. Its canonical form is
/// `{"type":"registration","template","thread","ordinal"}`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RegistrationKey {
    /// The template's qualified name.
    pub template: String,
    /// The thread whose forward effect registered it.
    pub thread: ThreadPath,
    /// The ordinal (ADR-027 CP-1).
    pub ordinal: u64,
}

impl RegistrationKey {
    pub(crate) fn enc(&self) -> Enc {
        Enc::tagged(
            "registration",
            vec![
                ("template", Enc::text(self.template.clone())),
                ("thread", self.thread.enc()),
                ("ordinal", Enc::text(self.ordinal.to_string())),
            ],
        )
    }
}

/// What a `send` did with its entry (ADR-027 ST-3, QSpec FR-170).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SendOutcome {
    /// Appended, displacing a queued entry or not.
    Accepted,
    /// Refused at capacity under `overflow reject`.
    Rejected,
    /// Dropped at capacity under `overflow loss new`.
    Lost,
}

impl SendOutcome {
    fn text(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Lost => "lost",
        }
    }
}

/// A `send` step's send record (ADR-027 ST-3), the step's result: carried
/// by the step (PX-1), and neither a binder value, nor part of the state
/// key, nor part of the transition identity. Its canonical form is
/// `{"type":"send-record","channel","payload","outcome","displaced"}`, with
/// `displaced` the queued entry (PS-7) or `null`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SendRecord {
    channel: String,
    payload: Enc,
    outcome: SendOutcome,
    displaced: Option<Enc>,
}

impl SendRecord {
    /// The record of a send on `channel` of `payload`, with its `outcome`
    /// and the queued entry it `displaced`.
    pub(crate) fn new(
        channel: String,
        payload: Enc,
        outcome: SendOutcome,
        displaced: Option<Enc>,
    ) -> Self {
        Self {
            channel,
            payload,
            outcome,
            displaced,
        }
    }

    /// The channel's qualified name (PS-7).
    pub fn channel(&self) -> &str {
        &self.channel
    }

    /// What the send did with its entry.
    pub fn outcome(&self) -> SendOutcome {
        self.outcome
    }

    /// This record with `outcome` in place of its own, as a changed
    /// counterexample may record it (FR-217-AC-5).
    #[cfg(any(test, feature = "test-support"))]
    pub fn with_outcome(mut self, outcome: SendOutcome) -> Self {
        self.outcome = outcome;
        self
    }

    fn enc(&self) -> Enc {
        Enc::tagged(
            "send-record",
            vec![
                ("channel", Enc::text(self.channel.clone())),
                ("payload", self.payload.clone()),
                ("outcome", Enc::text(self.outcome.text())),
                ("displaced", self.displaced.clone().unwrap_or(Enc::Null)),
            ],
        )
    }
}

impl Encode for SendRecord {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        self.enc().encode_into(writer)
    }
}

/// How a step took an entry of one send node off a channel's queue, as
/// ADR-027 PD-4's `Displaced` cause reads the reported prefix.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Removal {
    /// A `receive` took it.
    Received,
    /// A loss dropped it: a `lose` step, an overflow loss selector that
    /// displaced it, or a `lost` send of it.
    Lost,
}

/// One entry a step removed from, or dropped on, a channel of a protocol
/// instance (PD-4).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct QueueEvent {
    /// The channel's qualified name.
    pub(crate) channel: String,
    /// The protocol instance's ordinal.
    pub(crate) instance: u64,
    /// The static declaration path of the entry's send node.
    pub(crate) send: String,
    /// How it was removed.
    pub(crate) removal: Removal,
}

/// A step's post-state memory component in QSpec FR-181's typed canonical
/// form, as its JCS text (ADR-025 MK-1, FR-227): under `tso`, every store
/// buffer. It is data for the reader and for replay, which recomputes it;
/// the post-state digest already binds it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CarriedComponent(String);

impl CarriedComponent {
    /// The canonical form of `component`.
    pub(crate) fn of(component: &impl Encode) -> Result<Self, super::ProtocolRefusal> {
        String::from_utf8(super::steps::jcs(component)?)
            .map(Self)
            .map_err(|_| super::ProtocolRefusal::fault("jcs-is-utf-8"))
    }

    /// A component given as its canonical text, as a counterexample from
    /// outside may carry one; replay compares it with the recomputed one.
    #[cfg(any(test, feature = "test-support"))]
    pub fn from_text(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// Its JCS text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The anchor a step's position is observed with (ADR-027 PB-2, ADR-025
/// MS-2).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PositionAnchor<'a> {
    /// An `attempt` or `cattempt`: its operation's anchor, by the operation's qualified
    /// name (ADR-018 SM-3).
    Operation(&'a str),
    /// Every other protocol step: the `protocol` anchor naming its kind and
    /// its node, or, for a `duplicate` or `lose`, its channel.
    Protocol {
        /// The step kind.
        step: StepKind,
        /// The node's static declaration path; `None` for a channel step.
        node: Option<&'a str>,
        /// A channel step's channel, by its qualified name.
        channel: Option<&'a str>,
    },
    /// A memory step, such as `flush(b)`: the `memory` anchor naming the
    /// step.
    Memory(&'a MemoryStepId),
}

/// A protocol step's typed canonical transition identity (ADR-027 TS-2,
/// QSpec FR-426): `{"type":"protocol-transition","step":<kind>,...}` with
/// exactly the members its kind lists. A `send` step also carries its send
/// record (PX-1), a step that took an entry off a queue the removal PD-4
/// reads, and a step over a subject with a weak `parallel` its post-state's
/// memory component (ADR-025 MK-1); none is part of the identity, so two
/// transitions are equal when their identities are.
#[derive(Clone, Debug)]
pub struct ProtocolTransition {
    kind: StepKind,
    thread: Option<ThreadPath>,
    node: Option<String>,
    members: Members,
    removed: Option<QueueEvent>,
    send_record: Option<SendRecord>,
    memory: Option<CarriedComponent>,
    actor: Option<Actor>,
}

impl ProtocolTransition {
    /// `fork(node)` by `thread`.
    pub fn fork(thread: ThreadPath, node: impl Into<String>) -> Self {
        Self::control(StepKind::Fork, thread, node.into())
    }

    /// `join(node)` by `thread`.
    pub fn join(thread: ThreadPath, node: impl Into<String>) -> Self {
        Self::control(StepKind::Join, thread, node.into())
    }

    /// A step of `kind` by `thread` at `node` with no further member: a
    /// `fork`, `join` or `timeout`.
    pub(crate) fn control(kind: StepKind, thread: ThreadPath, node: String) -> Self {
        Self {
            kind,
            thread: Some(thread),
            node: Some(node),
            members: Members::Control,
            removed: None,
            send_record: None,
            memory: None,
            actor: None,
        }
    }

    /// The `fence` at `node` by `thread`, with its `ordering` (ADR-027
    /// TS-2, SE-5).
    pub(crate) fn fence(thread: ThreadPath, node: String, ordering: &'static str) -> Self {
        Self {
            kind: StepKind::Fence,
            thread: Some(thread),
            node: Some(node),
            members: Members::Fence { ordering },
            removed: None,
            send_record: None,
            memory: None,
            actor: None,
        }
    }

    pub(crate) fn attempt(
        thread: ThreadPath,
        node: String,
        application: ModelTransition,
        receiver: Enc,
        arguments: Vec<Enc>,
    ) -> Self {
        Self {
            kind: StepKind::Attempt,
            thread: Some(thread),
            node: Some(node),
            members: Members::Attempt {
                operation: application.operation().to_owned(),
                application,
                receiver,
                arguments,
                choice: None,
            },
            removed: None,
            send_record: None,
            memory: None,
            actor: None,
        }
    }

    /// This attempt, having chosen `choice` (QSpec FR-426 `memory_choice`).
    pub(crate) fn choosing(mut self, choice: Option<MemoryChoice>) -> Self {
        if let Members::Attempt { choice: held, .. } = &mut self.members {
            *held = choice;
        }
        self
    }

    /// A step of `kind` by `thread` at `node` that binds `record`: an
    /// `event`, `send`, `receive` or `finish`.
    pub(crate) fn record(kind: StepKind, thread: ThreadPath, node: String, record: Enc) -> Self {
        Self {
            kind,
            thread: Some(thread),
            node: Some(node),
            members: Members::Record { record },
            removed: None,
            send_record: None,
            memory: None,
            actor: None,
        }
    }

    /// A channel step of `kind`, `duplicate` or `lose`, on the queue of the
    /// channel `name` of protocol instance `instance`, and its queued
    /// `entry`.
    pub(crate) fn channel(kind: StepKind, name: String, instance: u64, entry: Enc) -> Self {
        Self {
            kind,
            thread: None,
            node: None,
            members: Members::Channel {
                name,
                instance,
                entry,
            },
            removed: None,
            send_record: None,
            memory: None,
            actor: None,
        }
    }

    /// `cattempt` by the compensation thread `thread` of `registration`,
    /// applying `application` (ADR-027 ST-11, TS-2).
    pub(crate) fn cattempt(
        thread: ThreadPath,
        registration: RegistrationKey,
        application: ModelTransition,
        receiver: Enc,
        arguments: Vec<Enc>,
    ) -> Self {
        Self {
            kind: StepKind::CAttempt,
            thread: Some(thread),
            node: None,
            members: Members::CAttempt {
                operation: application.operation().to_owned(),
                application,
                receiver,
                arguments,
                template: registration.template.clone(),
                registration,
            },
            removed: None,
            send_record: None,
            memory: None,
            actor: None,
        }
    }

    /// `cend` by the compensation thread `thread` of `registration`
    /// (ADR-027 ST-12, TS-2).
    pub(crate) fn cend(thread: ThreadPath, registration: RegistrationKey) -> Self {
        Self {
            kind: StepKind::CEnd,
            thread: Some(thread),
            node: None,
            members: Members::CEnd {
                template: registration.template.clone(),
                registration,
            },
            removed: None,
            send_record: None,
            memory: None,
            actor: None,
        }
    }

    pub(crate) fn memory(identity: MemoryStepId) -> Self {
        Self {
            kind: StepKind::Memory,
            thread: None,
            node: None,
            members: Members::Memory { identity },
            removed: None,
            send_record: None,
            memory: None,
            actor: None,
        }
    }

    /// `activate` by the environment of an `on each` clause with the trigger
    /// `record` (ADR-027 ST-13).
    pub(crate) fn activate(record: Enc) -> Self {
        Self {
            kind: StepKind::Activate,
            thread: None,
            node: None,
            members: Members::Record { record },
            removed: None,
            send_record: None,
            memory: None,
            actor: None,
        }
    }

    /// `spawn` or `retire` (`kind`) of `object` as an instance of `role` in
    /// protocol instance `instance` (ADR-027 ST-14, ST-15).
    pub(crate) fn role(
        kind: StepKind,
        instance: u64,
        role: String,
        object: ObjectReference,
    ) -> Self {
        Self {
            kind,
            thread: None,
            node: None,
            members: Members::Role {
                role,
                object,
                instance,
            },
            removed: None,
            send_record: None,
            memory: None,
            actor: None,
        }
    }

    /// This step, taken `by` the instance `actor` of a replicated role
    /// (ADR-027 RR-2).
    pub(crate) fn acting(mut self, actor: Actor) -> Self {
        self.actor = Some(actor);
        self
    }

    /// The instance of a replicated role this step is taken by.
    pub fn actor(&self) -> Option<&Actor> {
        self.actor.as_ref()
    }

    /// For a `spawn` or `retire`, the replicated role, the object and the
    /// protocol instance's ordinal.
    pub fn role_instance(&self) -> Option<(&str, &ObjectReference, u64)> {
        match &self.members {
            Members::Role {
                role,
                object,
                instance,
            } => Some((role, object, *instance)),
            _ => None,
        }
    }

    /// This step, recording that it took `removed` off its queue.
    pub(crate) fn removing(mut self, removed: QueueEvent) -> Self {
        self.removed = Some(removed);
        self
    }

    /// This `send` step, carrying its send record (PX-1).
    pub(crate) fn recording(mut self, record: SendRecord) -> Self {
        self.send_record = Some(record);
        self
    }

    /// This step carrying `record` as its send record, or none, as a
    /// counterexample from outside may: replay refuses a `send` without one
    /// and any other step with one (PX-2).
    #[cfg(any(test, feature = "test-support"))]
    pub fn carrying(mut self, record: Option<SendRecord>) -> Self {
        self.send_record = record;
        self
    }

    /// A `send` step's send record (ADR-027 ST-3, PX-1); `None` for every
    /// other step.
    pub fn send_record(&self) -> Option<&SendRecord> {
        self.send_record.as_ref()
    }

    /// This step, carrying `component`, its post-state's memory component
    /// (ADR-025 MK-1).
    pub(crate) fn with_memory(mut self, component: CarriedComponent) -> Self {
        self.memory = Some(component);
        self
    }

    /// This step carrying `component` as its post-state's memory
    /// component, or none, as a counterexample from outside may: replay
    /// refuses one that differs from the recomputed component (FR-227).
    #[cfg(any(test, feature = "test-support"))]
    pub fn carrying_memory(mut self, component: Option<CarriedComponent>) -> Self {
        self.memory = component;
        self
    }

    /// The post-state's memory component a counterexample or replay step
    /// over a subject with a weak `parallel` carries (ADR-025 MK-1, set by
    /// `SubjectSystem::reported`); `None` for an explored successor and over
    /// a subject whose every `parallel` resolves to `sc`.
    pub fn memory_component(&self) -> Option<&CarriedComponent> {
        self.memory.as_ref()
    }

    /// This attempt, naming `choice` as its message or slot, as a
    /// counterexample from outside may: replay refuses a depth the state
    /// has no message or slot at (FR-227).
    #[cfg(any(test, feature = "test-support"))]
    pub fn naming(self, choice: Option<MemoryChoice>) -> Self {
        self.choosing(choice)
    }

    /// The message or slot an access chose (QSpec FR-426 `memory_choice`);
    /// `None` for a step whose memory model chooses none.
    pub fn memory_choice(&self) -> Option<&MemoryChoice> {
        match &self.members {
            Members::Attempt { choice, .. } => choice.as_ref(),
            _ => None,
        }
    }

    /// The anchor the step's position is observed with (ADR-027 PB-2,
    /// ADR-025 MS-2): an attempt's operation anchor, a memory step's
    /// `memory` anchor naming it, and every other step's `protocol` anchor.
    pub fn anchor(&self) -> PositionAnchor<'_> {
        match &self.members {
            Members::Attempt { operation, .. } | Members::CAttempt { operation, .. } => {
                PositionAnchor::Operation(operation)
            }
            Members::Memory { identity } => PositionAnchor::Memory(identity),
            Members::Channel { name, .. } => PositionAnchor::Protocol {
                step: self.kind,
                node: None,
                channel: Some(name),
            },
            Members::Control
            | Members::Record { .. }
            | Members::Fence { .. }
            | Members::Role { .. }
            | Members::CEnd { .. } => PositionAnchor::Protocol {
                step: self.kind,
                node: self.node.as_deref(),
                channel: None,
            },
        }
    }

    /// The entry this step took off a channel's queue, if any.
    pub(crate) fn removed(&self) -> Option<&QueueEvent> {
        self.removed.as_ref()
    }

    /// The step's kind.
    pub fn kind(&self) -> StepKind {
        self.kind
    }

    /// For a `cattempt` or `cend`, its template's qualified name.
    pub fn template(&self) -> Option<&str> {
        self.registration().map(|key| key.template.as_str())
    }

    /// For a `cattempt` or `cend`, the registration its thread compensates.
    pub fn registration(&self) -> Option<&RegistrationKey> {
        match &self.members {
            Members::CAttempt { registration, .. } | Members::CEnd { registration, .. } => {
                Some(registration)
            }
            Members::Attempt { .. }
            | Members::Control
            | Members::Record { .. }
            | Members::Channel { .. }
            | Members::Memory { .. }
            | Members::Role { .. }
            | Members::Fence { .. } => None,
        }
    }

    /// The owning thread's path; `None` for a channel or memory step.
    pub fn thread(&self) -> Option<&ThreadPath> {
        self.thread.as_ref()
    }

    /// The node's static declaration path; `None` for a channel, memory,
    /// `cattempt` or `cend` step.
    pub fn node(&self) -> Option<&str> {
        self.node.as_deref()
    }

    /// For an `attempt` or a `cattempt`, the FR-120 transition identity of
    /// the application it applied (ADR-027 TS-3).
    pub fn application(&self) -> Option<&ModelTransition> {
        match &self.members {
            Members::Attempt { application, .. } | Members::CAttempt { application, .. } => {
                Some(application)
            }
            Members::Control
            | Members::Record { .. }
            | Members::Channel { .. }
            | Members::Memory { .. }
            | Members::Fence { .. }
            | Members::Role { .. }
            | Members::CEnd { .. } => None,
        }
    }

    /// For a memory step, its identity (ADR-027 SE-1 d).
    pub fn memory_step(&self) -> Option<&MemoryStepId> {
        match &self.members {
            Members::Memory { identity } => Some(identity),
            Members::Attempt { .. }
            | Members::Control
            | Members::Record { .. }
            | Members::Channel { .. }
            | Members::Fence { .. }
            | Members::Role { .. }
            | Members::CAttempt { .. }
            | Members::CEnd { .. } => None,
        }
    }

    /// For a `duplicate` or `lose`, the channel's qualified name and its
    /// protocol instance's ordinal.
    pub fn channel_queue(&self) -> Option<(&str, u64)> {
        match &self.members {
            Members::Channel { name, instance, .. } => Some((name, *instance)),
            Members::Attempt { .. }
            | Members::Control
            | Members::Record { .. }
            | Members::Memory { .. }
            | Members::Fence { .. }
            | Members::Role { .. }
            | Members::CAttempt { .. }
            | Members::CEnd { .. } => None,
        }
    }

    fn enc(&self) -> Enc {
        let mut members = vec![("step", Enc::text(self.kind.text()))];
        if let Some(thread) = &self.thread {
            members.push(("thread", thread.enc()));
        }
        if let Some(node) = &self.node {
            members.push(("node", Enc::text(node.clone())));
        }
        if let Some(actor) = &self.actor {
            members.push(("actor", reference_enc(&actor.object)));
        }
        match &self.members {
            Members::Attempt {
                operation,
                receiver,
                arguments,
                choice,
                ..
            } => {
                members.push(("operation", Enc::text(operation.clone())));
                members.push(("receiver", receiver.clone()));
                members.push(("arguments", Enc::Array(arguments.clone())));
                if let Some(choice) = choice {
                    members.push(("memory_choice", choice.enc()));
                }
            }
            Members::Control => {}
            Members::Record { record } => members.push(("record", record.clone())),
            Members::Channel {
                name,
                instance,
                entry,
            } => {
                members.push((
                    "channel",
                    Enc::tagged(
                        "channel",
                        vec![
                            ("name", Enc::text(name.clone())),
                            ("instance", Enc::text(instance.to_string())),
                        ],
                    ),
                ));
                members.push(("entry", entry.clone()));
            }
            Members::Memory { identity } => {
                members.push(("memory_step", identity.enc()));
            }
            Members::Fence { ordering } => members.push(("ordering", Enc::text(*ordering))),
            Members::CAttempt {
                operation,
                receiver,
                arguments,
                template,
                registration,
                ..
            } => {
                members.push(("operation", Enc::text(operation.clone())));
                members.push(("receiver", receiver.clone()));
                members.push(("arguments", Enc::Array(arguments.clone())));
                members.push(("template", Enc::text(template.clone())));
                members.push(("registration", registration.enc()));
            }
            Members::CEnd {
                template,
                registration,
            } => {
                members.push(("template", Enc::text(template.clone())));
                members.push(("registration", registration.enc()));
            }
            Members::Role {
                role,
                object,
                instance,
            } => {
                members.push(("role", Enc::text(role.clone())));
                members.push(("object", reference_enc(object)));
                members.push(("instance", Enc::text(instance.to_string())));
            }
        }
        Enc::tagged("protocol-transition", members)
    }
}

impl PartialEq for ProtocolTransition {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.thread == other.thread
            && self.node == other.node
            && self.members == other.members
            && self.actor == other.actor
    }
}

impl Eq for ProtocolTransition {}

/// A total order consistent with equality: by kind, thread, node and the
/// identity's members, so a step's send record and queue removal, which are
/// not part of its identity, do not order it. It is not the canonical order
/// of FR-101, which is the order of the identities' JCS bytes.
impl Ord for ProtocolTransition {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.kind, &self.thread, &self.node, &self.members).cmp(&(
            other.kind,
            &other.thread,
            &other.node,
            &other.members,
        ))
    }
}

impl PartialOrd for ProtocolTransition {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Encode for ProtocolTransition {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        self.enc().encode_into(writer)
    }
}

/// Why a waiting thread waits (ADR-027 PD-4, QSpec FR-429).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WaitCause {
    /// An attempt whose first false selected precondition, in clause order,
    /// is the one named.
    Precondition {
        /// The clause's name.
        clause: String,
    },
    /// An attempt whose preconditions hold and no post-state satisfies its
    /// postconditions within the frame.
    NoPostState,
    /// An event, attempt or `finish` whose constraint no record satisfies,
    /// or a `send` whose constraint no payload satisfies.
    Constraint,
    /// A `receive` with no entry of its send queued, and none lost since
    /// the last was received.
    EmptyQueue {
        /// The channel's qualified name.
        channel: String,
    },
    /// A `receive` whose send's entries all wait behind the head of a `fifo`
    /// queue that another send of the channel put there.
    HeadBlocked {
        /// The channel's qualified name.
        channel: String,
    },
    /// A `send` under `overflow block` whose queue is at capacity for every
    /// payload its constraint admits.
    FullQueue {
        /// The channel's qualified name.
        channel: String,
    },
    /// A `receive` with no entry of its send queued, whose last removal on
    /// the reported prefix was a loss, not a `receive`.
    Displaced {
        /// The channel's qualified name.
        channel: String,
    },
    /// A false `check`.
    CheckFalse {
        /// The `check` node's static declaration path.
        node: String,
    },
    /// A join, naming the branch threads it still waits for.
    JoinWaiting {
        /// The threads, in ascending JCS bytes of their paths.
        threads: Vec<ThreadPath>,
    },
    /// The root at `finish` while other threads of its instance are
    /// `running`, naming them.
    FinishWaiting {
        /// The threads, in ascending JCS bytes of their paths.
        threads: Vec<ThreadPath>,
    },
    /// A compensation thread at its first compensation attempt after a
    /// step of its template's commit node has occurred in its instance,
    /// naming the template.
    CommitClosed {
        /// The template's qualified name.
        template: String,
    },
    /// An event, attempt, send, receive or compensation attempt owned by a
    /// replicated role that has no live instance and no enabled `spawn`,
    /// naming the role.
    NoRoleInstance {
        /// The replicated role's name.
        role: String,
    },
    /// A step disabled only by its memory model's gate.
    Gate,
}

/// One waiting thread of a deadlocked state (ADR-027 PD-4).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockedThread {
    /// The thread's path.
    pub path: ThreadPath,
    /// Its rest point.
    pub rest_point: RestPoint,
    /// Why it waits.
    pub cause: WaitCause,
}
