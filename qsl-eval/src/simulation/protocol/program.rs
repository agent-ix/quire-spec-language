// SPDX-License-Identifier: AGPL-3.0-or-later
//! The static structure a protocol system walks, read once from the checked
//! protocol clause: each control's kind, name, path, parent and children;
//! each attempt's operation and selected contracts; each join's policy;
//! each `choice`'s cases, `repeat`'s parts, `await`'s parts and `check`'s
//! condition; each event, `send` and `receive` node's record and channel;
//! each channel's premises; each binder's slot and readers; and each
//! control's place in the tree and its innermost enclosing `repeat` body,
//! from which a rest point's reachable readers follow (ADR-027 PS-3, PS-5,
//! PS-6, PS-7, FO-1).
//!
//! Every walk keeps its own explicit order (tree order, or its reverse), so a
//! control tree of any depth is read without recursion.

use std::collections::{BTreeMap, BTreeSet};

use qsl_semantics::check::{
    AccessOrdering, ChannelCapacity, ChannelDelivery, ChannelOverflow, CheckedActivation,
    CheckedBody, CheckedJoinPolicy, CheckedOrdering, CheckedOutstanding, CheckedProtocol,
    CheckedTerminalMember, ControlStep, EdgeKind, LossSelector, Port, ProtocolNodeId,
};
use quire_exact::{EffectiveId, NodeKey, ValueType};

use super::{ProtocolRefusal, Unsupported};
use crate::simulation::model::enumerable;

/// One control of the `run` tree.
pub(super) struct Control {
    pub(super) step: ControlStep,
    pub(super) name: String,
    /// The static declaration path (QSpec FR-425 `<node>`).
    pub(super) text: String,
    pub(super) parent: Option<ProtocolNodeId>,
    pub(super) children: Vec<ProtocolNodeId>,
}

/// One `attempt`: its operation and the clauses its `contracts` select.
pub(super) struct Attempt<'a> {
    pub(super) declaring: EffectiveId,
    pub(super) operation: String,
    pub(super) receiver: EffectiveId,
    pub(super) contracts: std::sync::Arc<BTreeSet<NodeKey>>,
    pub(super) constraint: Option<&'a CheckedBody>,
    /// The binder's slot when the operation's result is a value of the
    /// binder's type: the attempt binds its result (ADR-027 ST-1).
    pub(super) binds: Option<usize>,
}

/// One `parallel`'s join (ADR-027 ST-6, ST-7).
pub(super) struct Join {
    pub(super) policy: CheckedJoinPolicy,
    pub(super) joined: Vec<String>,
    pub(super) outstanding: Option<CheckedOutstanding>,
    /// Each branch's label and body, in authored order.
    pub(super) branches: Vec<(String, ProtocolNodeId)>,
}

/// One `repeat` (ADR-027 PS-6, FO-1): its body, its `exhausted` control,
/// its iteration maximum and its guard.
pub(super) struct Loop<'a> {
    pub(super) body: ProtocolNodeId,
    pub(super) exhausted: Option<ProtocolNodeId>,
    pub(super) max: Option<u64>,
    pub(super) guard: &'a CheckedBody,
}

/// One `await` (ADR-027 ST-8): its matched event node, its `then` control
/// and its `timeout` control.
pub(super) struct Await {
    pub(super) event: ProtocolNodeId,
    pub(super) then: ProtocolNodeId,
    pub(super) timeout: ProtocolNodeId,
}

/// The record a node's step binds (ADR-027 ST-2 to ST-4): its constraint,
/// the binder's slot and the record type it ranges over.
pub(super) struct Record<'a> {
    pub(super) constraint: Option<&'a CheckedBody>,
    pub(super) slot: usize,
    pub(super) record_type: ValueType,
}

/// One `send` or `receive` (ADR-027 ST-3, ST-4): its record, its channel
/// and, for a `receive`, the `send` node its `of` names.
pub(super) struct Message<'a> {
    pub(super) record: Record<'a>,
    pub(super) channel: usize,
    pub(super) from: Option<ProtocolNodeId>,
}

/// One channel (ADR-027 PS-7, QSpec FR-170): its qualified name and its
/// premises, the `fifo` key included.
pub(super) struct Channel<'a> {
    pub(super) name: String,
    pub(super) capacity: usize,
    pub(super) overflow: ChannelOverflow,
    pub(super) delivery: ChannelDelivery,
    /// The `fifo` key over the message binder; `None` for `unordered`.
    pub(super) key: Option<&'a CheckedBody>,
}

impl Channel<'_> {
    /// Whether the delivery premise admits `duplicate` (ST-5).
    pub(super) fn duplicates(&self) -> bool {
        matches!(
            self.delivery,
            ChannelDelivery::Unknown | ChannelDelivery::AtLeastOnce
        )
    }

    /// Whether the delivery premise admits `lose` (ST-5).
    pub(super) fn loses(&self) -> bool {
        matches!(
            self.delivery,
            ChannelDelivery::Unknown | ChannelDelivery::AtMostOnce
        )
    }
}

/// The `finish` node.
pub(super) struct Finish<'a> {
    pub(super) node: ProtocolNodeId,
    pub(super) text: String,
    pub(super) constraint: Option<&'a CheckedBody>,
    /// The binder's slot and type.
    pub(super) binder: (usize, ValueType),
}

/// Where a thread rests (ADR-027 PS-3), by node identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) enum Rest {
    /// An event node: an `attempt`, `send`, `receive`, domain `event` or
    /// `commit`.
    Node(ProtocolNodeId),
    /// An `await`, whose matched event or `timeout` steps.
    Await(ProtocolNodeId),
    /// A `check` whose value is false.
    Check(ProtocolNodeId),
    /// A `parallel` waiting to fork.
    Fork(ProtocolNodeId),
    /// The join of a `parallel` the thread forked.
    Join(ProtocolNodeId),
    /// The `finish` node (root threads only).
    Finish,
    /// The end of a branch body.
    End,
}

/// What a settling can do, whatever its guards decide.
#[derive(Default)]
pub(super) struct SettleWalk {
    /// Whether it can end a branch body.
    pub(super) ends: bool,
    /// The `choice`, `repeat` and `check` nodes whose guards it can read.
    pub(super) guards: BTreeSet<ProtocolNodeId>,
}

/// Where settling starts (ADR-027 FO-1).
#[derive(Clone, Copy, Debug)]
pub(super) enum Cursor {
    /// Entering a control.
    Enter(ProtocolNodeId),
    /// Leaving a control.
    Exit(ProtocolNodeId),
    /// At a `repeat`'s guard: on entry and after each pass of its body.
    Guard(ProtocolNodeId),
}

/// The structure of one checked protocol clause.
pub(super) struct Program<'a> {
    pub(super) root: ProtocolNodeId,
    pub(super) controls: BTreeMap<ProtocolNodeId, Control>,
    pub(super) attempts: BTreeMap<ProtocolNodeId, Attempt<'a>>,
    pub(super) joins: BTreeMap<ProtocolNodeId, Join>,
    pub(super) loops: BTreeMap<ProtocolNodeId, Loop<'a>>,
    /// Each `choice`'s cases, each with its guard, in authored order.
    pub(super) choices: BTreeMap<ProtocolNodeId, Vec<(ProtocolNodeId, &'a CheckedBody)>>,
    pub(super) awaits: BTreeMap<ProtocolNodeId, Await>,
    /// Each `check`'s condition.
    pub(super) checks: BTreeMap<ProtocolNodeId, &'a CheckedBody>,
    /// Each `fence`'s ordering, by its label (ADR-027 TS-2).
    pub(super) fences: BTreeMap<ProtocolNodeId, AccessOrdering>,
    /// Each domain `event` and `commit` node's record.
    pub(super) events: BTreeMap<ProtocolNodeId, Record<'a>>,
    pub(super) messages: BTreeMap<ProtocolNodeId, Message<'a>>,
    pub(super) channels: Vec<Channel<'a>>,
    pub(super) finish: Finish<'a>,
    /// Each binder slot's node, `None` for the `over` input.
    pub(super) slots: Vec<Option<ProtocolNodeId>>,
    /// Each binder slot's readers.
    pub(super) readers: Vec<BTreeSet<ProtocolNodeId>>,
    /// The binder slot of each node that has one.
    pub(super) binder_of: BTreeMap<ProtocolNodeId, usize>,
    /// The protocol's `terminal` member.
    pub(super) terminal: &'a CheckedTerminalMember,
    /// Each control's place in the `run` tree.
    spans: BTreeMap<ProtocolNodeId, Span>,
    /// Each binder slot's readers by tree position.
    reader_positions: Vec<ReaderPositions>,
    /// Each control's innermost enclosing `repeat` whose body holds it.
    loop_of: BTreeMap<ProtocolNodeId, ProtocolNodeId>,
}

/// A control's place in the `run` tree: its preorder position, the last
/// position of its subtree, and the last position of its scope (its nearest
/// enclosing `branch`, else the whole tree).
struct Span {
    first: usize,
    last: usize,
    scope_first: usize,
    scope_last: usize,
    /// Whether its scope is the root thread's, which ends at `finish`.
    root_scope: bool,
}

/// One binder slot's readers: the preorder positions of the controls among
/// them, and whether the `finish` node reads it.
#[derive(Default)]
struct ReaderPositions {
    controls: BTreeSet<usize>,
    finish: bool,
}

fn unsupported(construct: Unsupported) -> ProtocolRefusal {
    ProtocolRefusal::Unsupported(construct)
}

/// The constructs this protocol system steps: `sequence`, `parallel` and its
/// branches, `choice`, `repeat`, `check`, `await`, `attempt`, `send`,
/// `receive`, domain `event`, `commit`, `fence` and `finish`, channels with a
/// literal capacity, under `activation on origin` (ADR-027 §9 sequencing
/// steps 3 and 4). Which memory model each `parallel` runs under is the
/// subject's resolved model, read when the system is built.
fn refuse_unsupported(protocol: &CheckedProtocol) -> Result<(), ProtocolRefusal> {
    for control in &protocol.controls.controls {
        if let ControlStep::Unstepped(construct) = control.step() {
            return Err(unsupported(Unsupported::Control(construct)));
        }
    }
    for channel in &protocol.channels {
        if matches!(channel.capacity, ChannelCapacity::Symbolic(_)) {
            return Err(unsupported(Unsupported::SymbolicCapacity));
        }
        let ordered_loss = matches!(
            channel.overflow,
            ChannelOverflow::Loss(LossSelector::Head | LossSelector::Tail)
        );
        if ordered_loss && channel.ordering == CheckedOrdering::Unordered {
            return Err(unsupported(Unsupported::UnorderedLossSelector));
        }
    }
    if !protocol.obligations.compensations.is_empty() {
        return Err(unsupported(Unsupported::Compensation));
    }
    if !protocol.obligations.captures.is_empty() {
        return Err(unsupported(Unsupported::Capture));
    }
    if matches!(
        protocol.obligations.activation,
        CheckedActivation::OnEach { .. }
    ) {
        return Err(unsupported(Unsupported::ActivationOnEach));
    }
    if protocol.roles.iter().any(|role| role.replicated) {
        return Err(unsupported(Unsupported::ReplicatedRole));
    }
    // The pre-check (TS-5) has already refused an unbounded record type.
    if protocol
        .slots
        .binders
        .iter()
        .any(|binder| binder.ranged && !enumerable(&binder.value_type))
    {
        return Err(unsupported(Unsupported::UnenumerableRecord));
    }
    Ok(())
}

impl<'a> Program<'a> {
    /// The structure of `protocol`, or the refusal of a construct this
    /// system does not step.
    pub(super) fn new(protocol: &'a CheckedProtocol) -> Result<Self, ProtocolRefusal> {
        refuse_unsupported(protocol)?;
        let tree = &protocol.controls.controls;
        let root = tree
            .first()
            .map(|control| control.node)
            .ok_or_else(|| ProtocolRefusal::fault("a-protocol-has-a-run-control"))?;
        let mut controls: BTreeMap<ProtocolNodeId, Control> = BTreeMap::new();
        for control in tree {
            controls.insert(
                control.node,
                Control {
                    step: control.step(),
                    name: control.name.clone(),
                    text: control.path.join("/"),
                    parent: None,
                    children: control.children.clone(),
                },
            );
        }
        for control in tree {
            for child in &control.children {
                if let Some(entry) = controls.get_mut(child) {
                    entry.parent = Some(control.node);
                }
            }
        }

        let slots: Vec<Option<ProtocolNodeId>> = protocol
            .slots
            .binders
            .iter()
            .map(|binder| binder.node)
            .collect();
        let binder_of: BTreeMap<ProtocolNodeId, usize> = slots
            .iter()
            .enumerate()
            .filter_map(|(slot, node)| node.map(|node| (node, slot)))
            .collect();
        let readers: Vec<BTreeSet<ProtocolNodeId>> = protocol
            .slots
            .readers
            .iter()
            .map(|readers| readers.readers.iter().copied().collect())
            .collect();
        if readers.len() != slots.len() {
            return Err(ProtocolRefusal::fault("readers-align-with-binders"));
        }
        // Slot 0 is the `over` input. Any other binder with no node (the
        // activation binder, a capture) has no value this system binds, so a
        // slot that reads one is refused before any state is built.
        let nodeless_read = slots
            .iter()
            .zip(&readers)
            .skip(1)
            .any(|(node, readers)| node.is_none() && !readers.is_empty());
        if nodeless_read {
            return Err(unsupported(Unsupported::NodelessBinder));
        }
        let body_of = |node: ProtocolNodeId| {
            protocol
                .slots
                .bodies
                .iter()
                .find(|body| body.node == node)
                .map(|body| &body.body)
        };

        let mut attempts = BTreeMap::new();
        for attempt in &protocol.attempts {
            let node = ProtocolNodeId(attempt.declaration);
            let binder = binder_of.get(&node).copied();
            let binds = binder.filter(|slot| {
                protocol.slots.binders.get(*slot).is_some_and(|binder| {
                    attempt.operation.declaration.result() == Some(&binder.value_type)
                })
            });
            // An attempt binder with no value of its type is never bound, so
            // no slot may read it (ADR-027 ST-1).
            if let (Some(slot), None) = (binder, binds) {
                if readers.get(slot).is_some_and(|readers| !readers.is_empty()) {
                    return Err(unsupported(Unsupported::UnboundAttemptBinder));
                }
            }
            attempts.insert(
                node,
                Attempt {
                    declaring: attempt.operation.declaring,
                    operation: attempt.operation.declaration.name().to_owned(),
                    receiver: attempt.receiver,
                    contracts: std::sync::Arc::new(attempt.contracts.iter().copied().collect()),
                    constraint: body_of(node),
                    binds,
                },
            );
        }

        let mut joins = BTreeMap::new();
        for parallel in &protocol.parallels {
            if parallel.policy == CheckedJoinPolicy::Predicate {
                return Err(unsupported(Unsupported::JoinPredicate));
            }
            if parallel.outstanding.is_none() && parallel.policy != CheckedJoinPolicy::All {
                return Err(unsupported(Unsupported::UnruledOutstanding));
            }
            let control = controls
                .get(&parallel.node)
                .ok_or_else(|| ProtocolRefusal::fault("a-parallel-is-a-control"))?;
            let mut branches = Vec::with_capacity(control.children.len());
            for branch in &control.children {
                let label = controls
                    .get(branch)
                    .ok_or_else(|| ProtocolRefusal::fault("a-branch-is-a-control"))?;
                let [body] = label.children[..] else {
                    return Err(ProtocolRefusal::fault("a-branch-has-one-body"));
                };
                branches.push((label.name.clone(), body));
            }
            joins.insert(
                parallel.node,
                Join {
                    policy: parallel.policy.clone(),
                    joined: parallel.joined.clone(),
                    outstanding: parallel.outstanding,
                    branches,
                },
            );
        }

        let edge_to = |kind: EdgeKind, from: ProtocolNodeId, port: Port| {
            protocol
                .edges
                .iter()
                .find(|edge| edge.kind == kind && edge.from.node == from && edge.from.port == port)
                .map(|edge| edge.to.node)
        };
        let child_of = |node: &ProtocolNodeId| controls.get(node).map(|c| c.children.clone());

        let mut loops = BTreeMap::new();
        for repeat in &protocol.slots.repeats {
            let children = child_of(&repeat.node)
                .ok_or_else(|| ProtocolRefusal::fault("a-repeat-is-a-control"))?;
            // The body is the child the back edge returns to.
            let body = children
                .iter()
                .copied()
                .find(|child| {
                    protocol.edges.iter().any(|edge| {
                        edge.kind == EdgeKind::RepeatBack
                            && edge.from.node == *child
                            && edge.to.node == *child
                    })
                })
                .ok_or_else(|| ProtocolRefusal::fault("a-repeat-has-a-body"))?;
            let exhausted = children.iter().copied().find(|child| *child != body);
            loops.insert(
                repeat.node,
                Loop {
                    body,
                    exhausted,
                    max: repeat.max,
                    guard: &repeat.guard,
                },
            );
        }

        let mut choices = BTreeMap::new();
        for choice in &protocol.slots.choices {
            let children = child_of(&choice.node)
                .ok_or_else(|| ProtocolRefusal::fault("a-choice-is-a-control"))?;
            let mut cases = Vec::with_capacity(choice.cases.len());
            for case in &choice.cases {
                let node = children
                    .iter()
                    .copied()
                    .find(|child| controls.get(child).is_some_and(|c| c.name == case.name))
                    .ok_or_else(|| ProtocolRefusal::fault("a-case-is-a-child-of-its-choice"))?;
                cases.push((node, &case.guard));
            }
            choices.insert(choice.node, cases);
        }

        let mut awaits = BTreeMap::new();
        for awaited in &protocol.awaits {
            let event = edge_to(EdgeKind::AwaitSuccess, awaited.node, Port::Enter)
                .ok_or_else(|| ProtocolRefusal::fault("an-await-has-an-event"))?;
            let then = edge_to(EdgeKind::AwaitSuccess, event, Port::Exit)
                .ok_or_else(|| ProtocolRefusal::fault("an-await-has-a-then"))?;
            let timeout = edge_to(EdgeKind::AwaitTimeout, awaited.node, Port::Enter)
                .ok_or_else(|| ProtocolRefusal::fault("an-await-has-a-timeout"))?;
            awaits.insert(
                awaited.node,
                Await {
                    event,
                    then,
                    timeout,
                },
            );
        }

        let record = |node: ProtocolNodeId| -> Result<Record<'a>, ProtocolRefusal> {
            let slot = binder_of
                .get(&node)
                .copied()
                .ok_or_else(|| ProtocolRefusal::fault("an-event-node-has-a-binder"))?;
            let record_type = protocol
                .slots
                .binders
                .get(slot)
                .map(|binder| binder.value_type.clone())
                .ok_or_else(|| ProtocolRefusal::fault("an-event-binder-has-a-type"))?;
            Ok(Record {
                constraint: body_of(node),
                slot,
                record_type,
            })
        };
        let mut checks = BTreeMap::new();
        let mut events = BTreeMap::new();
        for (node, control) in &controls {
            match control.step {
                ControlStep::Check => {
                    let condition = body_of(*node)
                        .ok_or_else(|| ProtocolRefusal::fault("a-check-has-a-condition"))?;
                    checks.insert(*node, condition);
                }
                ControlStep::Event => {
                    events.insert(*node, record(*node)?);
                }
                _ => {}
            }
        }

        let mut channels = Vec::with_capacity(protocol.channels.len());
        for channel in &protocol.channels {
            let ChannelCapacity::Literal(capacity) = channel.capacity else {
                return Err(unsupported(Unsupported::SymbolicCapacity));
            };
            channels.push(Channel {
                name: format!("{}::{}", protocol.name, channel.name),
                // A capacity past the address space admits every queue a run
                // can hold.
                capacity: usize::try_from(capacity).unwrap_or(usize::MAX),
                overflow: channel.overflow,
                delivery: channel.delivery,
                key: match &channel.ordering {
                    CheckedOrdering::Unordered => None,
                    CheckedOrdering::Fifo { key, .. } => Some(&**key),
                },
            });
        }
        let mut messages = BTreeMap::new();
        for via in &protocol.vias {
            let channel = protocol
                .channels
                .iter()
                .position(|channel| channel.name == via.channel)
                .ok_or_else(|| ProtocolRefusal::fault("a-message-names-a-channel"))?;
            let from = protocol
                .edges
                .iter()
                .find(|edge| edge.kind == EdgeKind::Message && edge.to.node == via.node)
                .map(|edge| edge.from.node);
            messages.insert(
                via.node,
                Message {
                    record: record(via.node)?,
                    channel,
                    from,
                },
            );
        }

        let (finish_node, finish_name) = protocol
            .finish
            .clone()
            .ok_or_else(|| ProtocolRefusal::fault("a-protocol-has-a-finish"))?;
        let finish_slot = binder_of
            .get(&finish_node)
            .copied()
            .ok_or_else(|| ProtocolRefusal::fault("a-finish-has-a-binder"))?;
        let finish_type = protocol
            .slots
            .binders
            .get(finish_slot)
            .map(|binder| binder.value_type.clone())
            .ok_or_else(|| ProtocolRefusal::fault("a-finish-binder-has-a-type"))?;
        let finish = Finish {
            node: finish_node,
            text: finish_name,
            constraint: body_of(finish_node),
            binder: (finish_slot, finish_type),
        };

        let mut program = Self {
            root,
            controls,
            attempts,
            joins,
            loops,
            choices,
            awaits,
            checks,
            fences: protocol
                .memory
                .fences
                .iter()
                .map(|fence| (fence.node, fence.ordering))
                .collect(),
            events,
            messages,
            channels,
            finish,
            slots,
            readers,
            binder_of,
            terminal: &protocol.slots.terminal,
            spans: BTreeMap::new(),
            reader_positions: Vec::new(),
            loop_of: BTreeMap::new(),
        };
        program.spans = program.spans()?;
        program.loop_of = program.enclosing_loops()?;
        program.reader_positions = program
            .readers
            .iter()
            .map(|readers| ReaderPositions {
                controls: readers
                    .iter()
                    .filter_map(|node| program.spans.get(node).map(|span| span.first))
                    .collect(),
                finish: readers.contains(&program.finish.node),
            })
            .collect();
        Ok(program)
    }

    /// The control `node`.
    pub(super) fn control(&self, node: ProtocolNodeId) -> Result<&Control, ProtocolRefusal> {
        self.controls
            .get(&node)
            .ok_or_else(|| ProtocolRefusal::fault("a-node-is-a-control"))
    }

    /// The static declaration path of `node`, the `finish` node's included.
    pub(super) fn text(&self, node: ProtocolNodeId) -> Result<&str, ProtocolRefusal> {
        if node == self.finish.node {
            return Ok(&self.finish.text);
        }
        Ok(&self.control(node)?.text)
    }

    /// The control `node`'s parent, `None` for the `run` root.
    pub(super) fn parent(
        &self,
        node: ProtocolNodeId,
    ) -> Result<Option<ProtocolNodeId>, ProtocolRefusal> {
        Ok(self.control(node)?.parent)
    }

    /// The `repeat` `node`.
    pub(super) fn repeat(&self, node: ProtocolNodeId) -> Result<&Loop<'a>, ProtocolRefusal> {
        self.loops
            .get(&node)
            .ok_or_else(|| ProtocolRefusal::fault("a-repeat-node-is-a-repeat"))
    }

    /// The ordering of the `fence` `node`.
    pub(super) fn fence_ordering(
        &self,
        node: ProtocolNodeId,
    ) -> Result<AccessOrdering, ProtocolRefusal> {
        self.fences
            .get(&node)
            .copied()
            .ok_or_else(|| ProtocolRefusal::fault("a-fence-has-an-ordering"))
    }

    /// The `await` `node`.
    pub(super) fn awaited(&self, node: ProtocolNodeId) -> Result<&Await, ProtocolRefusal> {
        self.awaits
            .get(&node)
            .ok_or_else(|| ProtocolRefusal::fault("an-await-node-is-an-await"))
    }

    /// Whether the node `reader`'s slots read binder slot `slot`.
    pub(super) fn reads(&self, slot: usize, reader: ProtocolNodeId) -> bool {
        self.readers
            .get(slot)
            .is_some_and(|readers| readers.contains(&reader))
    }

    /// Whether a thread resting at `rest` can still reach a node that reads
    /// binder slot `slot` before its step binds the slot again (ADR-027
    /// PS-5).
    ///
    /// A thread reaches the controls of its own body from where it rests to
    /// the end of its scope: the nearest enclosing `branch`, or the whole
    /// `run` tree and then `finish` for a root thread. In tree order that is
    /// one interval. Every reader of a binder follows its node in tree order
    /// and inside the innermost `repeat` body that holds the node (a
    /// `repeat` adds nothing past itself, QSpec FR-466), and every path into
    /// a reader passes the node first. So a thread resting at or before the
    /// node reaches no reader of the current binding; one past the node
    /// reaches the readers from its rest point to the end of that `repeat`
    /// body, and, through the back edge of each `repeat` that encloses it
    /// inside that body and inside its scope, every reader of that
    /// `repeat`'s body too.
    pub(super) fn reads_from(&self, rest: Rest, slot: usize) -> bool {
        let Some(readers) = self.reader_positions.get(slot) else {
            return false;
        };
        let Some(bound) = self.slots.get(slot).copied().flatten() else {
            return false;
        };
        let (at, start) = match rest {
            Rest::Finish => return readers.finish,
            Rest::End | Rest::Check(_) => return false,
            Rest::Node(node) | Rest::Fork(node) | Rest::Await(node) => {
                match self.spans.get(&node) {
                    Some(span) => (node, span.first),
                    None => return false,
                }
            }
            Rest::Join(node) => match self.spans.get(&node) {
                Some(span) => (node, span.last.saturating_add(1)),
                None => return false,
            },
        };
        let (Some(here), Some(binding)) = (self.spans.get(&at), self.spans.get(&bound)) else {
            return false;
        };
        if start <= binding.first {
            return false;
        }
        let innermost = self.loop_of.get(&bound).copied();
        let mut end = here.scope_last;
        let mut finish = here.root_scope;
        if let Some(repeat) = innermost {
            let Some(body) = self
                .loops
                .get(&repeat)
                .and_then(|lp| self.spans.get(&lp.body))
            else {
                return false;
            };
            if here.first < body.first || here.first > body.last {
                return false;
            }
            end = end.min(body.last);
            finish = false;
        }
        let mut start = start;
        let mut enclosing = self.loop_of.get(&at).copied();
        while let Some(repeat) = enclosing {
            if Some(repeat) == innermost {
                break;
            }
            let Some(span) = self.spans.get(&repeat) else {
                break;
            };
            if span.first < here.scope_first {
                break;
            }
            start = start.min(span.first);
            enclosing = self.loop_of.get(&repeat).copied();
        }
        (finish && readers.finish)
            || (start <= end && readers.controls.range(start..=end).next().is_some())
    }

    /// Where settling from `from` can go, whatever the guards and `check`s
    /// it passes decide: a walk of FO-1's structural moves that takes every
    /// move a guard could choose and stops at each rest point. It gives
    /// whether the settling can end a branch body (ADR-027 FT-1 "when the
    /// step ends a branch") and every guard and `check` it can read.
    pub(super) fn settle_walk(&self, from: Cursor) -> Result<SettleWalk, ProtocolRefusal> {
        let mut walk = SettleWalk::default();
        let mut seen: BTreeSet<(u8, ProtocolNodeId)> = BTreeSet::new();
        let mut pending = vec![from];
        while let Some(cursor) = pending.pop() {
            let key = match cursor {
                Cursor::Enter(node) => (0, node),
                Cursor::Exit(node) => (1, node),
                Cursor::Guard(node) => (2, node),
            };
            if !seen.insert(key) {
                continue;
            }
            match cursor {
                Cursor::Enter(node) => {
                    let control = self.control(node)?;
                    match control.step {
                        ControlStep::Sequence => pending.push(
                            control
                                .children
                                .first()
                                .map_or(Cursor::Exit(node), |first| Cursor::Enter(*first)),
                        ),
                        ControlStep::Branch | ControlStep::Case => {
                            pending
                                .extend(control.children.iter().map(|child| Cursor::Enter(*child)));
                        }
                        ControlStep::Choice => {
                            walk.guards.insert(node);
                            pending
                                .extend(control.children.iter().map(|child| Cursor::Enter(*child)));
                        }
                        ControlStep::Check => {
                            walk.guards.insert(node);
                            pending.push(Cursor::Exit(node));
                        }
                        ControlStep::Repeat => pending.push(Cursor::Guard(node)),
                        ControlStep::Parallel
                        | ControlStep::Attempt
                        | ControlStep::Send
                        | ControlStep::Receive
                        | ControlStep::Event
                        | ControlStep::Fence
                        | ControlStep::Await
                        | ControlStep::Unstepped(_) => {}
                    }
                }
                Cursor::Guard(node) => {
                    let repeat = self.repeat(node)?;
                    walk.guards.insert(node);
                    pending.push(Cursor::Exit(node));
                    pending.push(Cursor::Enter(repeat.body));
                    if let Some(exhausted) = repeat.exhausted {
                        pending.push(Cursor::Enter(exhausted));
                    }
                }
                Cursor::Exit(node) => {
                    let Some(parent) = self.parent(node)? else {
                        continue;
                    };
                    let holder = self.control(parent)?;
                    match holder.step {
                        ControlStep::Sequence => {
                            let position = holder
                                .children
                                .iter()
                                .position(|child| *child == node)
                                .ok_or_else(|| ProtocolRefusal::fault("a-child-of-its-parent"))?;
                            pending.push(
                                holder
                                    .children
                                    .get(position.saturating_add(1))
                                    .map_or(Cursor::Exit(parent), |next| Cursor::Enter(*next)),
                            );
                        }
                        ControlStep::Branch => walk.ends = true,
                        ControlStep::Case | ControlStep::Choice => {
                            pending.push(Cursor::Exit(parent));
                        }
                        ControlStep::Repeat => pending.push(if self.repeat(parent)?.body == node {
                            Cursor::Guard(parent)
                        } else {
                            Cursor::Exit(parent)
                        }),
                        ControlStep::Await => {
                            let awaited = self.awaited(parent)?;
                            pending.push(if awaited.event == node {
                                Cursor::Enter(awaited.then)
                            } else {
                                Cursor::Exit(parent)
                            });
                        }
                        ControlStep::Parallel
                        | ControlStep::Attempt
                        | ControlStep::Check
                        | ControlStep::Send
                        | ControlStep::Receive
                        | ControlStep::Event
                        | ControlStep::Fence
                        | ControlStep::Unstepped(_) => {
                            return Err(ProtocolRefusal::fault("a-parent-holds-controls"))
                        }
                    }
                }
            }
        }
        Ok(walk)
    }

    /// Whether the control `inner` lies in the subtree of the control
    /// `outer`, `outer` itself included.
    pub(super) fn encloses(
        &self,
        outer: ProtocolNodeId,
        inner: ProtocolNodeId,
    ) -> Result<bool, ProtocolRefusal> {
        match (self.spans.get(&outer), self.spans.get(&inner)) {
            (Some(outer), Some(inner)) => Ok((outer.first..=outer.last).contains(&inner.first)),
            _ => Err(ProtocolRefusal::fault("a-control-is-in-the-run-tree")),
        }
    }

    /// Whether the binder of `node` is in S3's `Out` of the branch whose body
    /// is `body` (ADR-027 PS-5): `node` lies in `body`, and not below an
    /// inner `parallel` joined by a policy other than `all`, in a `choice`
    /// case, in a `repeat` body or in an `await`, whose binders do not
    /// escape them.
    pub(super) fn escapes(
        &self,
        body: ProtocolNodeId,
        node: ProtocolNodeId,
    ) -> Result<bool, ProtocolRefusal> {
        if !self.encloses(body, node)? {
            return Ok(false);
        }
        let mut at = node;
        while at != body {
            let Some(parent) = self.parent(at)? else {
                return Err(ProtocolRefusal::fault("a-branch-body-encloses-its-nodes"));
            };
            let escapes = match self.control(parent)?.step {
                ControlStep::Case | ControlStep::Await => false,
                ControlStep::Repeat => self.repeat(parent)?.body != at,
                ControlStep::Branch => {
                    let parallel = self
                        .parent(parent)?
                        .ok_or_else(|| ProtocolRefusal::fault("a-branch-has-its-parallel"))?;
                    self.joins
                        .get(&parallel)
                        .is_some_and(|join| join.policy == CheckedJoinPolicy::All)
                }
                ControlStep::Sequence
                | ControlStep::Parallel
                | ControlStep::Choice
                | ControlStep::Attempt
                | ControlStep::Check
                | ControlStep::Send
                | ControlStep::Receive
                | ControlStep::Event
                | ControlStep::Fence
                | ControlStep::Unstepped(_) => true,
            };
            if !escapes {
                return Ok(false);
            }
            at = parent;
        }
        Ok(true)
    }

    /// Whether the subtree of the control `node` holds a `parallel`, so a
    /// thread running it can fork threads of its own.
    pub(super) fn forks_within(&self, node: ProtocolNodeId) -> Result<bool, ProtocolRefusal> {
        let span = self
            .spans
            .get(&node)
            .ok_or_else(|| ProtocolRefusal::fault("a-control-is-in-the-run-tree"))?;
        Ok(self.controls.iter().any(|(inner, control)| {
            control.step == ControlStep::Parallel
                && self
                    .spans
                    .get(inner)
                    .is_some_and(|at| (span.first..=span.last).contains(&at.first))
        }))
    }

    /// Each control's innermost enclosing `repeat` whose body holds it, from
    /// a preorder walk of the `run` tree.
    fn enclosing_loops(&self) -> Result<BTreeMap<ProtocolNodeId, ProtocolNodeId>, ProtocolRefusal> {
        let mut loop_of = BTreeMap::new();
        let mut stack: Vec<(ProtocolNodeId, Option<ProtocolNodeId>)> = vec![(self.root, None)];
        while let Some((node, enclosing)) = stack.pop() {
            if let Some(repeat) = enclosing {
                loop_of.insert(node, repeat);
            }
            let control = self.control(node)?;
            let body = self.loops.get(&node).map(|lp| lp.body);
            for child in &control.children {
                let inner = if body == Some(*child) {
                    Some(node)
                } else {
                    enclosing
                };
                stack.push((*child, inner));
            }
        }
        Ok(loop_of)
    }

    /// Each control's [`Span`], from a preorder walk of the `run` tree.
    fn spans(&self) -> Result<BTreeMap<ProtocolNodeId, Span>, ProtocolRefusal> {
        let mut order: Vec<(ProtocolNodeId, Option<ProtocolNodeId>)> = Vec::new();
        let mut stack = vec![(self.root, None)];
        while let Some((node, scope)) = stack.pop() {
            if order.len() >= self.controls.len() {
                return Err(ProtocolRefusal::fault("the-run-controls-form-a-tree"));
            }
            order.push((node, scope));
            let control = self.control(node)?;
            let inner = if matches!(control.step, ControlStep::Branch) {
                Some(node)
            } else {
                scope
            };
            stack.extend(control.children.iter().rev().map(|child| (*child, inner)));
        }
        let first: BTreeMap<ProtocolNodeId, usize> = order
            .iter()
            .enumerate()
            .map(|(position, (node, _))| (*node, position))
            .collect();
        let mut size: BTreeMap<ProtocolNodeId, usize> = BTreeMap::new();
        for (node, _) in order.iter().rev() {
            let children = self.control(*node)?.children.iter();
            let below: usize = children.filter_map(|child| size.get(child)).sum();
            size.insert(*node, below + 1);
        }
        let at = |node: ProtocolNodeId| -> Result<usize, ProtocolRefusal> {
            first
                .get(&node)
                .copied()
                .ok_or_else(|| ProtocolRefusal::fault("a-control-is-in-the-run-tree"))
        };
        let last = |node: ProtocolNodeId| -> Result<usize, ProtocolRefusal> {
            match (first.get(&node), size.get(&node)) {
                (Some(first), Some(size)) => Ok(first + size - 1),
                _ => Err(ProtocolRefusal::fault("a-control-is-in-the-run-tree")),
            }
        };
        let mut spans = BTreeMap::new();
        for (position, (node, scope)) in order.iter().enumerate() {
            spans.insert(
                *node,
                Span {
                    first: position,
                    last: last(*node)?,
                    scope_first: at(scope.unwrap_or(self.root))?,
                    scope_last: last(scope.unwrap_or(self.root))?,
                    root_scope: scope.is_none(),
                },
            );
        }
        Ok(spans)
    }
}
