// SPDX-License-Identifier: AGPL-3.0-or-later
//! The static structure a protocol system walks, read once from the checked
//! protocol clause: each control's kind, name, path, parent and children;
//! each attempt's operation and selected contracts; each join's policy;
//! each binder's slot and readers; and the nodes each rest point can still
//! reach (ADR-027 PS-3, PS-5, FO-1).
//!
//! Every walk keeps its own explicit order (tree order, or its reverse), so a
//! control tree of any depth is read without recursion.

use std::collections::{BTreeMap, BTreeSet};

use qsl_semantics::check::{
    CheckedActivation, CheckedBody, CheckedJoinPolicy, CheckedOutstanding, CheckedProtocol,
    CheckedTerminalMember, ControlStep, ProtocolNodeId,
};
use quire_exact::{EffectiveId, NodeKey, ValueType};

use super::{ProtocolRefusal, Unsupported};

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
    pub(super) contracts: BTreeSet<NodeKey>,
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
    /// An event node.
    Node(ProtocolNodeId),
    /// A `parallel` waiting to fork.
    Fork(ProtocolNodeId),
    /// The join of a `parallel` the thread forked.
    Join(ProtocolNodeId),
    /// The `finish` node (root threads only).
    Finish,
    /// The end of a branch body.
    End,
}

/// Where settling starts (ADR-027 FO-1).
#[derive(Clone, Copy, Debug)]
pub(super) enum Cursor {
    /// Entering a control.
    Enter(ProtocolNodeId),
    /// Leaving a control.
    Exit(ProtocolNodeId),
}

/// The structure of one checked protocol clause.
pub(super) struct Program<'a> {
    pub(super) root: ProtocolNodeId,
    pub(super) controls: BTreeMap<ProtocolNodeId, Control>,
    pub(super) attempts: BTreeMap<ProtocolNodeId, Attempt<'a>>,
    pub(super) joins: BTreeMap<ProtocolNodeId, Join>,
    pub(super) finish: Finish<'a>,
    /// Each binder slot's node, `None` for the `over` input.
    pub(super) slots: Vec<Option<ProtocolNodeId>>,
    /// Each binder slot's readers.
    pub(super) readers: Vec<BTreeSet<ProtocolNodeId>>,
    /// The binder slot of each node that has one.
    pub(super) binder_of: BTreeMap<ProtocolNodeId, usize>,
    /// The protocol's `terminal` member.
    pub(super) terminal: &'a CheckedTerminalMember,
    /// The nodes still reachable from each rest point.
    reach: BTreeMap<Rest, BTreeSet<ProtocolNodeId>>,
}

fn unsupported(construct: &'static str) -> ProtocolRefusal {
    ProtocolRefusal::Unsupported(Unsupported { construct })
}

/// The constructs this protocol system steps: `sequence`, `parallel` and its
/// branches, `attempt` and `finish` under `activation on origin`, every
/// `parallel` under `sc` (ADR-027 §9 sequencing step 3).
fn refuse_unsupported(protocol: &CheckedProtocol) -> Result<(), ProtocolRefusal> {
    for control in &protocol.controls.controls {
        if let ControlStep::Unstepped(construct) = control.step() {
            return Err(unsupported(construct));
        }
    }
    if protocol
        .memory
        .parallels
        .iter()
        .any(|parallel| !parallel.is_sequentially_consistent())
    {
        return Err(unsupported("memory"));
    }
    if !protocol.channels.is_empty() {
        return Err(unsupported("channel"));
    }
    if !protocol.obligations.compensations.is_empty() {
        return Err(unsupported("compensation"));
    }
    if !protocol.obligations.captures.is_empty() {
        return Err(unsupported("capture"));
    }
    if matches!(
        protocol.obligations.activation,
        CheckedActivation::OnEach { .. }
    ) {
        return Err(unsupported("activation on each"));
    }
    if protocol.roles.iter().any(|role| role.replicated) {
        return Err(unsupported("replicated role"));
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
                    return Err(unsupported("a read of an attempt binder with no result"));
                }
            }
            attempts.insert(
                node,
                Attempt {
                    declaring: attempt.operation.declaring,
                    operation: attempt.operation.declaration.name().to_owned(),
                    receiver: attempt.receiver,
                    contracts: attempt.contracts.iter().copied().collect(),
                    constraint: body_of(node),
                    binds,
                },
            );
        }

        let mut joins = BTreeMap::new();
        for parallel in &protocol.parallels {
            if parallel.policy == CheckedJoinPolicy::Predicate {
                return Err(unsupported("join predicate"));
            }
            if parallel.outstanding.is_none() && parallel.policy != CheckedJoinPolicy::All {
                return Err(unsupported(
                    "a join that may leave branches outstanding with no `outstanding` rule",
                ));
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
            finish,
            slots,
            readers,
            binder_of,
            terminal: &protocol.slots.terminal,
            reach: BTreeMap::new(),
        };
        program.reach = program.reachability(tree.iter().map(|control| control.node).collect());
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

    /// FO-1: apply structural moves from `cursor` until a rest point.
    pub(super) fn settle(&self, mut cursor: Cursor) -> Result<Rest, ProtocolRefusal> {
        // Each move enters a child or leaves a node for its parent's
        // continuation, so the walk ends within twice the tree's size.
        for _ in 0..=self.controls.len().saturating_mul(2) {
            cursor = match cursor {
                Cursor::Enter(node) => {
                    let control = self.control(node)?;
                    match control.step {
                        ControlStep::Sequence => match control.children.first() {
                            Some(first) => Cursor::Enter(*first),
                            None => Cursor::Exit(node),
                        },
                        ControlStep::Branch => match control.children[..] {
                            [body] => Cursor::Enter(body),
                            _ => return Err(ProtocolRefusal::fault("a-branch-has-one-body")),
                        },
                        ControlStep::Parallel => return Ok(Rest::Fork(node)),
                        ControlStep::Attempt => return Ok(Rest::Node(node)),
                        ControlStep::Unstepped(_) => {
                            return Err(ProtocolRefusal::fault("a-stepped-construct-is-admitted"))
                        }
                    }
                }
                Cursor::Exit(node) => {
                    let Some(parent) = self.control(node)?.parent else {
                        return Ok(Rest::Finish);
                    };
                    let holder = self.control(parent)?;
                    match holder.step {
                        ControlStep::Sequence => {
                            let position = holder
                                .children
                                .iter()
                                .position(|child| *child == node)
                                .ok_or_else(|| ProtocolRefusal::fault("a-child-of-its-parent"))?;
                            match holder.children.get(position + 1) {
                                Some(next) => Cursor::Enter(*next),
                                None => Cursor::Exit(parent),
                            }
                        }
                        ControlStep::Branch => return Ok(Rest::End),
                        ControlStep::Parallel
                        | ControlStep::Attempt
                        | ControlStep::Unstepped(_) => {
                            return Err(ProtocolRefusal::fault("a-parent-is-a-sequence-or-branch"))
                        }
                    }
                }
            };
        }
        Err(ProtocolRefusal::fault("settling-reaches-a-rest-point"))
    }

    /// The nodes a thread resting at `rest` can still reach (ADR-027 PS-5).
    pub(super) fn reach(&self, rest: Rest) -> &BTreeSet<ProtocolNodeId> {
        static EMPTY: BTreeSet<ProtocolNodeId> = BTreeSet::new();
        self.reach.get(&rest).unwrap_or(&EMPTY)
    }

    /// Every rest point's reachable nodes, from `order`, the `run` tree in
    /// tree order (each node before its children). A thread reaches the
    /// nodes of the control it rests in and of every later control of its
    /// own body; a branch thread's reach ends with its branch, and the
    /// root's ends at `finish`.
    fn reachability(&self, order: Vec<ProtocolNodeId>) -> BTreeMap<Rest, BTreeSet<ProtocolNodeId>> {
        let mut subtree: BTreeMap<ProtocolNodeId, BTreeSet<ProtocolNodeId>> = BTreeMap::new();
        for node in order.iter().rev() {
            let mut nodes = BTreeSet::from([*node]);
            if let Some(control) = self.controls.get(node) {
                for child in &control.children {
                    nodes.extend(subtree.get(child).into_iter().flatten().copied());
                }
            }
            subtree.insert(*node, nodes);
        }
        let mut after: BTreeMap<ProtocolNodeId, BTreeSet<ProtocolNodeId>> = BTreeMap::new();
        for node in &order {
            let reached = match self.controls.get(node).and_then(|control| control.parent) {
                None => BTreeSet::from([self.finish.node]),
                Some(parent) => {
                    let Some(holder) = self.controls.get(&parent) else {
                        continue;
                    };
                    match holder.step {
                        ControlStep::Sequence => {
                            let mut reached = after.get(&parent).cloned().unwrap_or_default();
                            let later = holder
                                .children
                                .iter()
                                .skip_while(|child| *child != node)
                                .skip(1);
                            for sibling in later {
                                reached.extend(subtree.get(sibling).into_iter().flatten().copied());
                            }
                            reached
                        }
                        // A branch body's end is its thread's end; the
                        // children of a `parallel` are its branches, which
                        // the forking thread does not walk.
                        ControlStep::Branch
                        | ControlStep::Parallel
                        | ControlStep::Attempt
                        | ControlStep::Unstepped(_) => BTreeSet::new(),
                    }
                }
            };
            after.insert(*node, reached);
        }
        let mut reach = BTreeMap::new();
        for node in &order {
            let Some(control) = self.controls.get(node) else {
                continue;
            };
            let mut entered = subtree.get(node).cloned().unwrap_or_default();
            entered.extend(after.get(node).into_iter().flatten().copied());
            match control.step {
                ControlStep::Parallel => {
                    reach.insert(Rest::Fork(*node), entered);
                    reach.insert(
                        Rest::Join(*node),
                        after.get(node).cloned().unwrap_or_default(),
                    );
                }
                ControlStep::Attempt => {
                    reach.insert(Rest::Node(*node), entered);
                }
                ControlStep::Sequence | ControlStep::Branch | ControlStep::Unstepped(_) => {}
            }
        }
        reach.insert(Rest::Finish, BTreeSet::from([self.finish.node]));
        reach
    }
}
