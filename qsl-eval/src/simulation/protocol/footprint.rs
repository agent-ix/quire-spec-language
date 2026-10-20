// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-216 (ADR-027 §5 FT-1 to FT-5): the static read, write and enabling
//! footprints of protocol steps over model and protocol locations, and a
//! step's visibility to a claim.
//!
//! A step's footprint is a function of its transition identity and the
//! checked protocol: an attempt's model locations are its application's
//! (FR-155) over the clauses its `contracts` select, and its protocol
//! locations are the control of its own thread, the binders S3 resolves for
//! its node, the queue of its own node's channel and the parallel instance
//! its fork or join names (FT-2). The state the step is taken from only
//! resolves which of those the identity names: the ordinal a fork takes, the
//! instance a join waits for, and the thread whose binding a reader reads.
//! Independence is FR-155's [`independent`](crate::simulation::independent)
//! over these footprints (FT-4): two steps of one thread both write its
//! `ctl` and are always dependent.
//!
//! Three readings go past FT-1's text, each to keep a footprint sound:
//!
//! - The step that ends a branch body writes that branch's disposition in
//!   its parallel instance's record ([`ProtocolLocation::Branch`]), not the
//!   whole record: the ends of two sibling branches commute, as FT-4 says
//!   their attempts do, and each still meets the join that reads the whole
//!   record.
//! - `finish` reads the control of every thread of its protocol instance
//!   (ST-9 is enabled only once every other thread has left `running`), and
//!   writes the queues of every channel of the instance it removes (AE-3).
//! - An `outstanding cancel` join writes the control of every thread at or
//!   below a branch that can fork ([`ProtocolLocation::Subtree`]): it
//!   cancels those threads and settles their parallel instances.
//!
//! Compensation registrations, replicated roles and live protocol instances
//! (`reg(c, ·)`, `role(r)`, `pinst`) belong to constructs this system
//! refuses, and the one memory model it steps, `sc`, names no memory
//! location and has no memory step.

use std::collections::BTreeSet;

use qsl_semantics::check::{ChannelOverflow, CheckedOutstanding, ControlStep, ProtocolNodeId};

use super::identity::{ProtocolTransition, StepKind, ThreadPath};
use super::memory::MemoryModel;
use super::program::{Cursor, Rest};
use super::{Instance, ProtocolRefusal, ProtocolState, ProtocolSystem, Thread};
use crate::simulation::footprint::{Footprint, Location};
use crate::simulation::model::FootprintError;

/// One parallel instance (ADR-027 PS-4): the thread that forked it, its
/// `parallel` node's static declaration path, and its ordinal.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ParallelInstance {
    /// The forking thread.
    pub parent: ThreadPath,
    /// The `parallel` node's static declaration path.
    pub node: String,
    /// The ordinal (PS-2).
    pub ordinal: u64,
}

/// A location of a protocol state (ADR-027 §5).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProtocolLocation {
    /// `ctl(t)`: thread `t`'s rest point and disposition.
    Control(ThreadPath),
    /// The `ctl` of the thread and of every thread below it, with the record
    /// of every parallel instance those threads forked.
    Subtree(ThreadPath),
    /// `inst(i)`: parallel instance `i`'s record, its settled flag and every
    /// branch's disposition.
    Instance(ParallelInstance),
    /// One branch's disposition in parallel instance `i`'s record.
    Branch {
        /// The parallel instance.
        instance: ParallelInstance,
        /// The branch label.
        label: String,
    },
    /// `bind(n, t)`: the binder of node `n`, by its static declaration
    /// path, bound by thread `t`.
    Binder {
        /// The binder's node.
        node: String,
        /// The binding thread.
        thread: ThreadPath,
    },
    /// `queue(h)`: every queue of channel `h` of one protocol instance.
    Queue {
        /// The protocol instance's ordinal.
        instance: u64,
        /// The channel's qualified name.
        channel: String,
    },
}

impl ProtocolLocation {
    /// Whether the two locations share a cell of the protocol state: equal
    /// locations; a subtree and a control, subtree or parallel instance at
    /// or below its thread; a parallel instance's record and one of its
    /// branches' dispositions.
    pub fn meets(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Subtree(root), Self::Control(thread))
            | (Self::Control(thread), Self::Subtree(root)) => root.is_prefix_of(thread),
            (Self::Subtree(left), Self::Subtree(right)) => {
                left.is_prefix_of(right) || right.is_prefix_of(left)
            }
            (Self::Subtree(root), Self::Instance(instance))
            | (Self::Instance(instance), Self::Subtree(root))
            | (Self::Subtree(root), Self::Branch { instance, .. })
            | (Self::Branch { instance, .. }, Self::Subtree(root)) => {
                root.is_prefix_of(&instance.parent)
            }
            (Self::Instance(whole), Self::Branch { instance, .. })
            | (Self::Branch { instance, .. }, Self::Instance(whole)) => whole == instance,
            (Self::Control(_), Self::Control(_))
            | (Self::Instance(_), Self::Instance(_))
            | (Self::Branch { .. }, Self::Branch { .. })
            | (Self::Binder { .. }, Self::Binder { .. })
            | (Self::Queue { .. }, Self::Queue { .. }) => self == other,
            (
                Self::Control(_)
                | Self::Subtree(_)
                | Self::Instance(_)
                | Self::Branch { .. }
                | Self::Binder { .. }
                | Self::Queue { .. },
                _,
            ) => false,
        }
    }
}

/// Why a step has no footprint.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProtocolFootprintError {
    /// The attempt's application has no model footprint.
    #[error(transparent)]
    Model(#[from] FootprintError),
    /// An invariant of the checked protocol broke.
    #[error(transparent)]
    Refused(#[from] ProtocolRefusal),
    /// The step names no thread resting at its node, no parallel instance
    /// or no channel of the state, or is a memory step, which `sc` has none
    /// of.
    #[error("the step is not a step of the state")]
    UnknownStep,
}

/// What an atom of a claim reads its anchor for (ADR-021 POR-5): a node of
/// the protocol, by its static declaration path, or an operation, by its
/// qualified name.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AnchorName {
    /// A protocol node: the `protocol` anchor of its steps.
    Node(String),
    /// An operation: the operation anchor of the attempts that apply it.
    Operation(String),
}

/// What the atoms of a claim read: model locations, and the anchors they
/// name.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ClaimReads {
    /// The locations the atoms read.
    pub locations: BTreeSet<Location>,
    /// The anchors the atoms read, by what they name.
    pub anchors: BTreeSet<AnchorName>,
}

/// ADR-027 FT-5 (ADR-021 POR-5): whether `step`, with footprint
/// `footprint`, is visible to a claim whose atoms read `claim`: its write
/// footprint affects a location an atom reads, or an atom reads its
/// position's anchor and names its node or operation.
pub fn visible(step: &ProtocolTransition, footprint: &Footprint, claim: &ClaimReads) -> bool {
    let writes = footprint
        .writes
        .iter()
        .any(|written| claim.locations.iter().any(|read| written.affects(read)));
    let named = match step.application() {
        Some(application) => claim
            .anchors
            .contains(&AnchorName::Operation(application.operation().to_owned())),
        None => step
            .node()
            .is_some_and(|node| claim.anchors.contains(&AnchorName::Node(node.to_owned()))),
    };
    writes || named
}

fn protocol(location: ProtocolLocation) -> Location {
    Location::Protocol(location)
}

fn control(thread: &ThreadPath) -> Location {
    protocol(ProtocolLocation::Control(thread.clone()))
}

/// A footprint under construction: what the step reads, writes and needs
/// to be enabled.
#[derive(Default)]
struct Builder {
    footprint: Footprint,
}

impl Builder {
    /// `location` read by the step's enabledness.
    fn enabling(&mut self, location: Location) {
        self.footprint.reads.insert(location.clone());
        self.footprint.enabling.insert(location);
    }

    /// `location` read by the step's effect only.
    fn read(&mut self, location: Location) {
        self.footprint.reads.insert(location);
    }

    fn write(&mut self, location: Location) {
        self.footprint.writes.insert(location);
    }
}

impl<M: MemoryModel> ProtocolSystem<'_, M> {
    /// FR-216: the footprint of `step` taken from `state`, enabled there or
    /// not: its read, write and enabling locations (ADR-027 FT-1, FT-3),
    /// each protocol location as [`Location::Protocol`].
    ///
    /// # Errors
    ///
    /// [`ProtocolFootprintError::Model`] when an attempt's application has
    /// no model footprint; [`ProtocolFootprintError::UnknownStep`] when the
    /// step is not one `state` can take; [`ProtocolFootprintError::Refused`]
    /// when an invariant of the checked protocol broke.
    pub fn footprint(
        &self,
        state: &ProtocolState<M>,
        step: &ProtocolTransition,
    ) -> Result<Footprint, ProtocolFootprintError> {
        let unknown = || ProtocolFootprintError::UnknownStep;
        let mut built = Builder::default();
        if let Some((channel, instance)) = step.channel_queue() {
            state.instances.get(&instance).ok_or_else(unknown)?;
            let queue = protocol(ProtocolLocation::Queue {
                instance,
                channel: channel.to_owned(),
            });
            built.enabling(queue.clone());
            built.write(queue);
            return Ok(built.footprint);
        }
        let path = step.thread().ok_or_else(unknown)?;
        let instance = state.instances.get(&path.instance).ok_or_else(unknown)?;
        let thread = instance.threads.get(path).ok_or_else(unknown)?;
        let (node, after) = self.stepped(thread, step)?;
        let own = control(path);
        built.enabling(own.clone());
        built.write(own);
        match step.kind() {
            StepKind::Attempt => {
                let application = step.application().ok_or_else(unknown)?;
                let attempt = self.program.attempts.get(&node).ok_or_else(unknown)?;
                let model = self
                    .model
                    .footprint_selecting(application, Some(&attempt.contracts))?;
                built.footprint.reads.extend(model.reads);
                built.footprint.writes.extend(model.writes);
                built.footprint.enabling.extend(model.enabling);
                self.binders_read(&mut built, instance, path, node)?;
                if attempt.binds.is_some() {
                    built.write(self.binder(node, path)?);
                }
            }
            StepKind::Event | StepKind::Send | StepKind::Receive => {
                self.binders_read(&mut built, instance, path, node)?;
                built.write(self.binder(node, path)?);
                if step.kind() != StepKind::Event {
                    let channel = self.channel_of(node)?;
                    let queue = protocol(ProtocolLocation::Queue {
                        instance: path.instance,
                        channel: channel.name.clone(),
                    });
                    built.write(queue.clone());
                    // A `send` that does not block is enabled whatever its
                    // queue holds (FT-3).
                    if step.kind() == StepKind::Send && channel.overflow != ChannelOverflow::Block {
                        built.read(queue);
                    } else {
                        built.enabling(queue);
                    }
                }
            }
            StepKind::Fork => {
                let join = self.program.joins.get(&node).ok_or_else(unknown)?;
                let text = self.program.text(node)?;
                let ordinal = Self::free_ordinal(instance, path, node, text, join)?;
                built.write(protocol(ProtocolLocation::Instance(ParallelInstance {
                    parent: path.clone(),
                    node: text.to_owned(),
                    ordinal,
                })));
                for (label, _) in &join.branches {
                    built.write(control(&path.child(text, label, ordinal)));
                }
            }
            StepKind::Join => {
                let join = self.program.joins.get(&node).ok_or_else(unknown)?;
                let text = self.program.text(node)?;
                let (key, _) = Self::joined_instance(instance, path, node).ok_or_else(unknown)?;
                let record = protocol(ProtocolLocation::Instance(ParallelInstance {
                    parent: path.clone(),
                    node: text.to_owned(),
                    ordinal: key.ordinal,
                }));
                built.enabling(record.clone());
                built.write(record);
                if join.outstanding == Some(CheckedOutstanding::Cancel) {
                    for (label, body) in &join.branches {
                        let branch = path.child(text, label, key.ordinal);
                        built.write(protocol(if self.program.forks_within(*body)? {
                            ProtocolLocation::Subtree(branch)
                        } else {
                            ProtocolLocation::Control(branch)
                        }));
                    }
                }
            }
            StepKind::Timeout => {}
            StepKind::Finish => {
                built.enabling(protocol(ProtocolLocation::Subtree(ThreadPath::root(
                    path.instance,
                ))));
                self.binders_read(&mut built, instance, path, node)?;
                built.write(self.binder(node, path)?);
                for channel in &self.program.channels {
                    built.write(protocol(ProtocolLocation::Queue {
                        instance: path.instance,
                        channel: channel.name.clone(),
                    }));
                }
            }
            StepKind::Duplicate | StepKind::Lose | StepKind::Memory => return Err(unknown()),
        }
        if let (Some((key, label)), Some(after)) = (&thread.owner, after) {
            if self.program.may_end(after)? {
                let parallel = ParallelInstance {
                    parent: key.parent.clone(),
                    node: self.program.text(key.node)?.to_owned(),
                    ordinal: key.ordinal,
                };
                built.write(protocol(ProtocolLocation::Branch {
                    instance: parallel,
                    label: label.clone(),
                }));
            }
        }
        Ok(built.footprint)
    }

    /// The node `step` of the resting `thread` steps, and where its thread
    /// settles from after it; `None` for a `fork`, whose owner rests at its
    /// join, and for `finish`, which removes the instance.
    fn stepped(
        &self,
        thread: &Thread,
        step: &ProtocolTransition,
    ) -> Result<(ProtocolNodeId, Option<Cursor>), ProtocolFootprintError> {
        let unknown = || ProtocolFootprintError::UnknownStep;
        let (node, after) = match (thread.rest, step.kind()) {
            (
                Rest::Node(node),
                StepKind::Attempt | StepKind::Event | StepKind::Send | StepKind::Receive,
            ) => (node, Some(Cursor::Exit(node))),
            (Rest::Await(node), StepKind::Timeout) => {
                let awaited = self.program.awaited(node)?;
                (node, Some(Cursor::Enter(awaited.timeout)))
            }
            (
                Rest::Await(node),
                StepKind::Attempt | StepKind::Event | StepKind::Send | StepKind::Receive,
            ) => {
                let awaited = self.program.awaited(node)?;
                (awaited.event, Some(Cursor::Enter(awaited.then)))
            }
            (Rest::Fork(node), StepKind::Fork) => (node, None),
            (Rest::Join(node), StepKind::Join) => (node, Some(Cursor::Exit(node))),
            (Rest::Finish, StepKind::Finish) => (self.program.finish.node, None),
            _ => return Err(unknown()),
        };
        if self.program.text(node).ok() != step.node() {
            return Err(unknown());
        }
        let kind = (node != self.program.finish.node)
            .then(|| self.program.control(node).map(|control| control.step))
            .transpose()?;
        let matches = match step.kind() {
            StepKind::Attempt => kind == Some(ControlStep::Attempt),
            StepKind::Event => kind == Some(ControlStep::Event),
            StepKind::Send => kind == Some(ControlStep::Send),
            StepKind::Receive => kind == Some(ControlStep::Receive),
            StepKind::Fork | StepKind::Join => kind == Some(ControlStep::Parallel),
            StepKind::Timeout => kind == Some(ControlStep::Await),
            StepKind::Finish => kind.is_none(),
            StepKind::Duplicate | StepKind::Lose | StepKind::Memory => false,
        };
        if !matches {
            return Err(unknown());
        }
        Ok((node, after))
    }

    /// `bind(node, path)`.
    fn binder(
        &self,
        node: ProtocolNodeId,
        path: &ThreadPath,
    ) -> Result<Location, ProtocolFootprintError> {
        let text = self.program.text(node)?;
        Ok(protocol(ProtocolLocation::Binder {
            node: text.to_owned(),
            thread: path.clone(),
        }))
    }

    /// The binders `node`'s slots read, as the thread `path` reads them
    /// (PS-5): each by the thread whose binding it sees.
    fn binders_read(
        &self,
        built: &mut Builder,
        instance: &Instance,
        path: &ThreadPath,
        node: ProtocolNodeId,
    ) -> Result<(), ProtocolFootprintError> {
        for (slot, bound) in self.program.slots.iter().enumerate() {
            let Some(bound) = bound else {
                continue;
            };
            if !self.program.reads(slot, node) {
                continue;
            }
            for by in instance.read_by(*bound, path) {
                built.enabling(self.binder(*bound, &by)?);
            }
        }
        Ok(())
    }
}
