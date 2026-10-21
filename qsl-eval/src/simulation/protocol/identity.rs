// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-425, FR-426 and FR-429's typed canonical forms (ADR-027 PS-2,
//! TS-2, PD-4): thread paths, rest points, protocol transition identities
//! and blocked threads.
//!
//! A node is named by its static declaration path: the names of the `run`
//! tree's controls from the root down to it, joined by `/`, or the
//! `finish` node's own name.

use quire_canonical::{Encode, Error, Sink, Writer};
use quire_semantic_value::model_transition::ModelTransition;

use super::memory::MemoryStepId;
use crate::simulation::model::encode::Enc;

/// One step of a branch thread's path (QSpec FR-425): the `parallel` that
/// forked it, its branch label and the parallel instance's ordinal.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PathStep {
    /// The `parallel` node's static declaration path.
    pub node: String,
    /// The branch label.
    pub branch: String,
    /// The ordinal (ADR-027 PS-2).
    pub ordinal: u64,
}

/// A thread's path (ADR-027 PS-2): its protocol instance's ordinal, then
/// one [`PathStep`] per `parallel` above it, outermost first.
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
        steps.push(PathStep {
            node: node.to_owned(),
            branch: branch.to_owned(),
            ordinal,
        });
        Self {
            instance: self.instance,
            steps,
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
                    Enc::Array(
                        self.steps
                            .iter()
                            .map(|step| {
                                Enc::Object(vec![
                                    ("node", Enc::text(step.node.clone())),
                                    ("branch", Enc::text(step.branch.clone())),
                                    ("ordinal", Enc::text(step.ordinal.to_string())),
                                ])
                            })
                            .collect(),
                    ),
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
    /// An event node, or the `finish` node.
    Node,
    /// A `parallel` waiting to fork.
    Fork,
    /// The join of a `parallel` the thread forked.
    Join,
    /// The end of a branch body.
    End,
}

impl RestKind {
    fn text(self) -> &'static str {
        match self {
            Self::Node => "node",
            Self::Fork => "fork",
            Self::Join => "join",
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

/// A protocol step's typed canonical transition identity (ADR-027 TS-2,
/// QSpec FR-426): `{"type":"protocol-transition","step":<kind>,...}` with
/// exactly the members its kind lists. A `send` step also carries its send
/// record (PX-1), and a step that took an entry off a queue the removal
/// PD-4 reads; neither is part of the identity, so two transitions are
/// equal when their identities are.
#[derive(Clone, Debug)]
pub struct ProtocolTransition {
    kind: StepKind,
    thread: Option<ThreadPath>,
    node: Option<String>,
    members: Members,
    removed: Option<QueueEvent>,
    send_record: Option<SendRecord>,
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
            },
            removed: None,
            send_record: None,
        }
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

    /// The entry this step took off a channel's queue, if any.
    pub(crate) fn removed(&self) -> Option<&QueueEvent> {
        self.removed.as_ref()
    }

    /// The step's kind.
    pub fn kind(&self) -> StepKind {
        self.kind
    }

    /// The owning thread's path; `None` for a channel or memory step.
    pub fn thread(&self) -> Option<&ThreadPath> {
        self.thread.as_ref()
    }

    /// The node's static declaration path; `None` for a channel or memory
    /// step.
    pub fn node(&self) -> Option<&str> {
        self.node.as_deref()
    }

    /// For an `attempt`, the FR-120 transition identity of the application
    /// it applied (ADR-027 TS-3).
    pub fn application(&self) -> Option<&ModelTransition> {
        match &self.members {
            Members::Attempt { application, .. } => Some(application),
            Members::Control
            | Members::Record { .. }
            | Members::Channel { .. }
            | Members::Memory { .. }
            | Members::Fence { .. } => None,
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
            | Members::Fence { .. } => None,
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
            | Members::Fence { .. } => None,
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
        match &self.members {
            Members::Attempt {
                operation,
                receiver,
                arguments,
                ..
            } => {
                members.push(("operation", Enc::text(operation.clone())));
                members.push(("receiver", receiver.clone()));
                members.push(("arguments", Enc::Array(arguments.clone())));
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
