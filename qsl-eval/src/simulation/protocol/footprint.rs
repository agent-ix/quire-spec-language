// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-216 (ADR-027 §5 FT-1 to FT-5): the static read, write and enabling
//! footprints of protocol steps over model and protocol locations, and a
//! step's visibility to a claim.
//!
//! A step's footprint is a function of its transition identity and the
//! checked protocol: an attempt's model locations are its application's
//! (FR-155) over the clauses its `contracts` select, and its protocol
//! locations are the control of its own thread, the binders S3 resolves for
//! its node in its own binder environment, which is part of its control
//! (PS-5), the queue of its own node's channel and the parallel instance its
//! fork or join names, with, for a join under `all`, its joined branch
//! threads' control, whose environments it copies (FT-2). The state the
//! step is taken from only resolves which of those the identity names: the
//! ordinal a fork takes and the instance a join waits for.
//! Independence is FR-155's [`independent`](crate::simulation::independent)
//! over these footprints (FT-4): two steps of one thread both write its
//! `ctl` and are always dependent.
//!
//! How FT-1's locations are laid out here:
//!
//! - A `fork` reads every sibling parallel instance of its node, from
//!   ordinal 0 to the one it takes, and every thread at or below each of
//!   their branches ([`ProtocolLocation::Subtree`]): the ordinal is the
//!   smallest no live sibling holds (PS-2).
//! - An `event`, `send` or `finish` whose record type holds a reference
//!   lists its records over the state's objects, so it reads the any-object
//!   membership of every population, a superset of the populations the
//!   reference ranges over.
//! - The binders the guards and `check`s of a step's settling read are in
//!   its read footprint (FT-1's last sentence), not its enabling one.
//! - A thread's binder environment is part of its control: an entry
//!   ([`ProtocolLocation::Binder`]) meets the control of the thread that
//!   holds it.
//! - The step that ends a branch body writes that branch's disposition in
//!   its parallel instance's record ([`ProtocolLocation::Branch`]), not the
//!   whole record, unless it ends the last running thread of an instance
//!   whose join has settled, which it removes (PS-4).
//! - `finish` reads the control of every thread of its protocol instance
//!   (ST-9 is enabled only once every other thread has left `running`), and
//!   writes the queues of every channel of the instance it removes (AE-3).
//! - An `outstanding cancel` join writes the control of every thread at or
//!   below a branch that can fork ([`ProtocolLocation::Subtree`]): it
//!   cancels those threads and settles their parallel instances.
//!
//! Compensation registrations, replicated roles and live protocol instances
//! (`reg(c, ·)`, `role(r)`, `pinst`) belong to constructs this system
//! refuses.
//!
//! The memory model adds its own locations (ADR-025 MR-1) through
//! [`MemoryModel::footprint`]: `sc` names none and has no memory step;
//! `tso` names each thread's store buffer, `buf(b)`
//! ([`ProtocolLocation::Buffer`]), beside the model locations that stand for
//! `mem(ℓ)`. An attempt's class is EN-1's static one (FR-220): a
//! read-modify-write that applies as a load keeps the read-modify-write's
//! larger footprint.

use std::collections::BTreeSet;

use qsl_semantics::check::{
    AccessClass, ChannelOverflow, CheckedJoinPolicy, CheckedOutstanding, ControlStep,
    ProtocolNodeId,
};

use qsl_semantics::check::footprint::FootprintRefusal;
use qsl_semantics::check::{FormulaNodeId, TemporalClauseNode};

use super::access::InstanceAccess;
use super::identity::{PositionAnchor, ProtocolTransition, StepKind, ThreadPath};
use super::memory::{FootprintStep, GatedStep, MemoryModel};
use super::program::{Cursor, Rest};
use super::{Disposition, ProtocolRefusal, ProtocolState, ProtocolSystem, Thread};
use crate::model_subject::ModelInstance;
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
    /// `bind(n, t)`: the entry for the binder of node `n`, by its static
    /// declaration path, in thread `t`'s binder environment, part of
    /// `ctl(t)` (PS-5).
    Binder {
        /// The binder's node.
        node: String,
        /// The thread whose environment holds the entry.
        thread: ThreadPath,
    },
    /// `buf(b)`: thread `b`'s store buffer in the memory component
    /// (ADR-025 MR-1).
    Buffer(ThreadPath),
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
    /// branches' dispositions; a thread's control, or a subtree above it,
    /// and an entry of its binder environment.
    pub fn meets(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Control(holder), Self::Binder { thread, .. })
            | (Self::Binder { thread, .. }, Self::Control(holder)) => holder == thread,
            (Self::Subtree(root), Self::Binder { thread, .. })
            | (Self::Binder { thread, .. }, Self::Subtree(root)) => root.is_prefix_of(thread),
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
            | (Self::Buffer(_), Self::Buffer(_))
            | (Self::Queue { .. }, Self::Queue { .. }) => self == other,
            (
                Self::Control(_)
                | Self::Subtree(_)
                | Self::Instance(_)
                | Self::Branch { .. }
                | Self::Binder { .. }
                | Self::Buffer(_)
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
    /// or no channel of the state, or is a memory step the memory model
    /// does not take at the state.
    #[error("the step is not a step of the state")]
    UnknownStep,
}

/// What an atom of a claim reads its anchor for (ADR-021 POR-5, ADR-027
/// PB-2): a node of the protocol, by its static declaration path, an
/// operation, by its qualified name, or a channel, by its qualified name.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AnchorName {
    /// A protocol node: the `protocol` anchor of its steps.
    Node(String),
    /// An operation: the operation anchor of the attempts that apply it.
    Operation(String),
    /// A channel, by its qualified name: the `protocol` anchor of its
    /// `send`, `receive`, `duplicate` and `lose` steps (PB-2).
    Channel(String),
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
/// position's anchor and names its operation, its node, or, for a channel
/// step, a `send` or a `receive`, its channel (PB-2).
pub fn visible(step: &ProtocolTransition, footprint: &Footprint, claim: &ClaimReads) -> bool {
    let writes = footprint
        .writes
        .iter()
        .any(|written| claim.locations.iter().any(|read| written.affects(read)));
    let named = |name: AnchorName| claim.anchors.contains(&name);
    let anchored = match step.anchor() {
        PositionAnchor::Operation(operation) => named(AnchorName::Operation(operation.to_owned())),
        PositionAnchor::Protocol {
            channel: Some(channel),
            ..
        } => named(AnchorName::Channel(channel.to_owned())),
        PositionAnchor::Protocol {
            step: kind,
            node,
            channel: None,
        } => {
            let by_node = node.is_some_and(|node| named(AnchorName::Node(node.to_owned())));
            // A `send` or `receive` writes exactly its own channel's queue.
            let by_channel = matches!(kind, StepKind::Send | StepKind::Receive)
                && footprint.writes.iter().any(|written| {
                    matches!(
                        written,
                        Location::Protocol(ProtocolLocation::Queue { channel, .. })
                            if named(AnchorName::Channel(channel.clone()))
                    )
                });
            by_node || by_channel
        }
        // No atom names a memory anchor (FT-5).
        PositionAnchor::Memory(_) => false,
    };
    writes || anchored
}

fn protocol(location: ProtocolLocation) -> Location {
    Location::Protocol(location)
}

fn control(thread: &ThreadPath) -> Location {
    protocol(ProtocolLocation::Control(thread.clone()))
}

/// Why a step reads a binder: its enabledness reads it, or only the
/// settling after it does (FO-1, FO-2).
#[derive(Clone, Copy)]
enum Read {
    Enabling,
    Settling,
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
    /// ADR-027 FT-5's claim reads at `state`: the locations the `holds`
    /// atoms `atoms` of `clause` read for the instance `over` of its `over`
    /// parameter (FR-156 "Atom footprints"), each location read through the
    /// memory model's atom values (SE-1 g). A node of `atoms` that is no
    /// `holds` atom reads no model location. The anchors an atom reads are
    /// not derived here, so the result names none.
    ///
    /// # Errors
    ///
    /// [`FootprintRefusal`] when an atom's condition reads through a form
    /// QSpec FR-383 gives no footprint rule.
    pub fn claim_reads(
        &self,
        clause: &TemporalClauseNode,
        atoms: &[FormulaNodeId],
        over: &ModelInstance,
        state: &ProtocolState<M>,
    ) -> Result<ClaimReads, FootprintRefusal> {
        let observed = self.memory.atom_values(&state.memory, &state.model);
        let mut locations = BTreeSet::new();
        for atom in atoms {
            let Some((condition, _)) = clause.condition(*atom) else {
                continue;
            };
            let reads = self.model.atom_reads(clause.clause().over(), condition)?;
            locations.extend(self.model.atom_locations(&reads, over, &observed));
        }
        Ok(ClaimReads {
            locations,
            anchors: BTreeSet::new(),
        })
    }

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
        if let Some(identity) = step.memory_step() {
            let memory = FootprintStep::Memory {
                identity,
                stored: self.weak.shared(),
            };
            if !self.memory.footprint(
                &state.memory,
                &identity.thread,
                memory,
                &mut built.footprint,
            ) {
                return Err(unknown());
            }
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
                let access = match self
                    .weak
                    .of(node)
                    .map(|weak| (weak, weak.instance(application)))
                {
                    None => FootprintStep::Attempt {
                        class: AccessClass::Local,
                        ordering: None,
                        shared: None,
                    },
                    Some((weak, Some(InstanceAccess::Access { class, location }))) => {
                        FootprintStep::Attempt {
                            class: *class,
                            ordering: weak.ordering,
                            shared: location.as_ref(),
                        }
                    }
                    Some((_, Some(InstanceAccess::Shape { .. }) | None)) => {
                        return Err(ProtocolRefusal::fault(
                            "a-weak-attempt-application-is-one-classified-access",
                        )
                        .into())
                    }
                };
                self.memory
                    .footprint(&state.memory, path, access, &mut built.footprint);
                self.binders_read(&mut built, path, node, Read::Enabling)?;
                if attempt.binds.is_some() {
                    built.write(self.binder(node, path)?);
                }
            }
            StepKind::Event | StepKind::Send | StepKind::Receive => {
                self.binders_read(&mut built, path, node, Read::Enabling)?;
                // An `event` and a `send` enumerate their records over the
                // state's objects; a `receive` takes its record from the queue.
                let listed = match step.kind() {
                    StepKind::Event => self
                        .program
                        .events
                        .get(&node)
                        .map(|record| &record.record_type),
                    StepKind::Send => self
                        .program
                        .messages
                        .get(&node)
                        .map(|message| &message.record.record_type),
                    StepKind::Receive
                    | StepKind::Attempt
                    | StepKind::Duplicate
                    | StepKind::Lose
                    | StepKind::Fork
                    | StepKind::Join
                    | StepKind::Timeout
                    | StepKind::Finish
                    | StepKind::Fence
                    | StepKind::Memory => None,
                };
                if let Some(record_type) = listed {
                    for location in self.model.record_reads(record_type) {
                        built.enabling(location);
                    }
                }
                built.write(self.binder(node, path)?);
                if step.kind() == StepKind::Send {
                    self.memory.footprint(
                        &state.memory,
                        path,
                        FootprintStep::Gated(GatedStep::Send),
                        &mut built.footprint,
                    );
                }
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
                self.memory.footprint(
                    &state.memory,
                    path,
                    FootprintStep::Gated(GatedStep::Fork),
                    &mut built.footprint,
                );
                let join = self.program.joins.get(&node).ok_or_else(unknown)?;
                let text = self.program.text(node)?;
                let ordinal = Self::free_ordinal(instance, path, node, text, join)?;
                // The ordinal is the smallest no live sibling holds (PS-2): the
                // fork reads every sibling instance up to it, and every thread
                // at or below each of their branches.
                for sibling in 0..=ordinal {
                    built.read(protocol(ProtocolLocation::Instance(ParallelInstance {
                        parent: path.clone(),
                        node: text.to_owned(),
                        ordinal: sibling,
                    })));
                    for (label, _) in &join.branches {
                        built.read(protocol(ProtocolLocation::Subtree(
                            path.child(text, label, sibling),
                        )));
                    }
                }
                built.write(protocol(ProtocolLocation::Instance(ParallelInstance {
                    parent: path.clone(),
                    node: text.to_owned(),
                    ordinal,
                })));
                for (label, body) in &join.branches {
                    let child = path.child(text, label, ordinal);
                    for guard in &self.program.settle_walk(Cursor::Enter(*body))?.guards {
                        self.binders_read(&mut built, &child, *guard, Read::Settling)?;
                    }
                    built.write(control(&child));
                }
            }
            StepKind::Join => {
                self.memory.footprint(
                    &state.memory,
                    path,
                    FootprintStep::Gated(GatedStep::Join),
                    &mut built.footprint,
                );
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
                if join.policy == CheckedJoinPolicy::All {
                    for label in &join.joined {
                        built.read(control(&path.child(text, label, key.ordinal)));
                    }
                }
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
            // The memory model's fence rule names its memory locations
            // (SE-5, ADR-025 MR-1).
            StepKind::Fence => {
                let ordering = self.program.fence_ordering(node)?;
                self.memory.footprint(
                    &state.memory,
                    path,
                    FootprintStep::Gated(GatedStep::Fence(ordering)),
                    &mut built.footprint,
                );
            }
            StepKind::Finish => {
                built.enabling(protocol(ProtocolLocation::Subtree(ThreadPath::root(
                    path.instance,
                ))));
                self.binders_read(&mut built, path, node, Read::Enabling)?;
                for location in self.model.record_reads(&self.program.finish.binder.1) {
                    built.enabling(location);
                }
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
        if let Some(after) = after {
            let walk = self.program.settle_walk(after)?;
            for guard in &walk.guards {
                self.binders_read(&mut built, path, *guard, Read::Settling)?;
            }
            if let (Some((key, label)), true) = (&thread.owner, walk.ends) {
                let parallel = ParallelInstance {
                    parent: key.parent.clone(),
                    node: self.program.text(key.node)?.to_owned(),
                    ordinal: key.ordinal,
                };
                // Ending the last running branch of a settled instance
                // removes the whole record (PS-4).
                let removes = instance.parallels.get(key).is_some_and(|record| {
                    record.settled
                        && record.dispositions.iter().all(|(other, disposition)| {
                            other == label || *disposition != Disposition::Running
                        })
                });
                built.write(protocol(if removes {
                    ProtocolLocation::Instance(parallel)
                } else {
                    ProtocolLocation::Branch {
                        instance: parallel,
                        label: label.clone(),
                    }
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
                StepKind::Attempt
                | StepKind::Event
                | StepKind::Send
                | StepKind::Receive
                | StepKind::Fence,
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
            StepKind::Fence => kind == Some(ControlStep::Fence),
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

    /// The binders `node`'s slots read, as the thread `path` reads them: each
    /// in `path`'s own environment (PS-5).
    fn binders_read(
        &self,
        built: &mut Builder,
        path: &ThreadPath,
        node: ProtocolNodeId,
        reads: Read,
    ) -> Result<(), ProtocolFootprintError> {
        for (slot, bound) in self.program.slots.iter().enumerate() {
            let Some(bound) = bound else {
                continue;
            };
            if !self.program.reads(slot, node) {
                continue;
            }
            let location = self.binder(*bound, path)?;
            match reads {
                Read::Enabling => built.enabling(location),
                Read::Settling => built.read(location),
            }
        }
        Ok(())
    }
}
