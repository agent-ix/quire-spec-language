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
        }
    }
}

/// The members a step's identity carries beside its kind.
#[derive(Clone, Debug, Eq, PartialEq)]
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
    /// `channel` and `entry`.
    Channel { channel: Enc, entry: Enc },
    /// `memory_step`, the canonical text its memory model gives.
    Memory { identity: String },
}

/// A protocol step's typed canonical transition identity (ADR-027 TS-2,
/// QSpec FR-426): `{"type":"protocol-transition","step":<kind>,...}` with
/// exactly the members its kind lists.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolTransition {
    kind: StepKind,
    thread: Option<ThreadPath>,
    node: Option<String>,
    members: Members,
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
        }
    }

    /// A channel step of `kind`, `duplicate` or `lose`, on `channel` and its
    /// queued `entry`.
    pub(crate) fn channel(kind: StepKind, channel: Enc, entry: Enc) -> Self {
        Self {
            kind,
            thread: None,
            node: None,
            members: Members::Channel { channel, entry },
        }
    }

    pub(crate) fn memory(identity: String) -> Self {
        Self {
            kind: StepKind::Memory,
            thread: None,
            node: None,
            members: Members::Memory { identity },
        }
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
            | Members::Memory { .. } => None,
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
            Members::Channel { channel, entry } => {
                members.push(("channel", channel.clone()));
                members.push(("entry", entry.clone()));
            }
            Members::Memory { identity } => {
                members.push(("memory_step", Enc::text(identity.clone())));
            }
        }
        Enc::tagged("protocol-transition", members)
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
    /// A `receive` with no deliverable entry.
    EmptyQueue {
        /// The channel's qualified name.
        channel: String,
    },
    /// A `send` under `overflow block` whose queue is at capacity for every
    /// payload its constraint admits.
    FullQueue {
        /// The channel's qualified name.
        channel: String,
    },
    /// A false `check`.
    CheckFalse {
        /// The `check` node's static declaration path.
        node: String,
    },
    /// A join, or a `finish`, naming the threads it still waits for.
    JoinWaiting {
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
