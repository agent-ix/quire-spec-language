// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-036 PA-9: the global event structure of a checked protocol, built from
//! the FR-519 causal relation, and the questions a projection and its replay
//! arm ask of it (FR-570).
//!
//! The structure is the causal relation's reachability among the protocol's
//! events: every `send`, `receive` and `choice` control, and the entry and
//! exit of every `parallel`. It is built once and never enumerates an
//! interleaving; the order of two events is a path in the relation or it is
//! absent. The reader that fills it from a package lives in `spine`
//! (ADR-032 CK-3); this module holds no checked-package type.

use std::collections::{BTreeMap, BTreeSet};

use qsl_package::protocol_view::{Port, PortRef};
use quire_contract_model::CheckedNodeId;

use super::RunLimit;

/// The run limits a structure build counts against (PA-14). They are outside
/// every key: a run one of them stops settles `failed`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RunBudget {
    /// The most events (states) the structure may hold.
    pub max_states: u64,
    /// The most covering edges (transitions) the structure may hold.
    pub max_transitions: u64,
}

impl RunBudget {
    /// The published default of `max_states`.
    pub const DEFAULT_MAX_STATES: u64 = 4_096;
    /// The published default of `max_transitions`.
    pub const DEFAULT_MAX_TRANSITIONS: u64 = 262_144;
}

impl Default for RunBudget {
    fn default() -> Self {
        Self {
            max_states: Self::DEFAULT_MAX_STATES,
            max_transitions: Self::DEFAULT_MAX_TRANSITIONS,
        }
    }
}

/// One communication's endpoints: the channel and its sending and receiving
/// roles and message type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Comm {
    /// The `channel` node.
    pub channel: CheckedNodeId,
    /// The sending role.
    pub sender: CheckedNodeId,
    /// The receiving role.
    pub receiver: CheckedNodeId,
    /// The message type node.
    pub message_type: CheckedNodeId,
}

/// What a global event is.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GlobalEventKind {
    /// A `send` control.
    Send(Comm),
    /// A `receive` control.
    Receive(Comm),
    /// A `choice` control.
    Choice {
        /// The owning role (or the protocol node, which no role is).
        owner: CheckedNodeId,
        /// The cases `(label, body)`, ascending by label bytes.
        cases: Vec<(String, CheckedNodeId)>,
        /// The `receive` controls whose payload binder the choice's decision
        /// facts read: an outcome-carrying message (FR-053-AC-1).
        carriers: Vec<CheckedNodeId>,
    },
    /// The entry of a `parallel`.
    Region {
        /// The branch bodies, ascending by label bytes.
        branches: Vec<CheckedNodeId>,
    },
    /// The exit of a `parallel`.
    Join {
        /// The branch bodies, ascending by label bytes.
        branches: Vec<CheckedNodeId>,
    },
}

/// One event of the global structure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlobalEvent {
    /// The event's entry port: the event's name.
    pub at: PortRef,
    /// The port at which the event completes; its successors follow from it.
    pub out: PortRef,
    /// What the event is.
    pub kind: GlobalEventKind,
    /// The control nodes enclosing the event, outermost first, the event's
    /// own control last.
    pub within: Vec<CheckedNodeId>,
}

impl GlobalEvent {
    /// The role that performs the event, when one does: a send's sender, a
    /// receive's receiver, a choice's owner.
    #[must_use]
    pub fn performer(&self) -> Option<&CheckedNodeId> {
        match &self.kind {
            GlobalEventKind::Send(comm) => Some(&comm.sender),
            GlobalEventKind::Receive(comm) => Some(&comm.receiver),
            GlobalEventKind::Choice { owner, .. } => Some(owner),
            GlobalEventKind::Region { .. } | GlobalEventKind::Join { .. } => None,
        }
    }

    /// Whether the event is a communication.
    #[must_use]
    pub fn is_communication(&self) -> bool {
        matches!(
            self.kind,
            GlobalEventKind::Send(_) | GlobalEventKind::Receive(_)
        )
    }
}

/// A channel whose FIFO key reads nothing of the message: every message of
/// it carries an equal key value, so the channel orders all its deliveries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UniformFifo {
    /// The `channel` node.
    pub channel: CheckedNodeId,
}

/// What a builder reads of a package.
#[derive(Clone, Debug, Default)]
pub struct StructureInput {
    /// The events, in any order.
    pub events: Vec<GlobalEvent>,
    /// The causal relation's edges, `repeat` back edges excluded.
    pub edges: Vec<(PortRef, PortRef)>,
    /// Each choice case body, with the choice it belongs to.
    pub case_bodies: BTreeMap<CheckedNodeId, CheckedNodeId>,
    /// The channels that order every delivery.
    pub uniform_fifo: Vec<UniformFifo>,
    /// Each `receive` control with the `send` control it receives.
    pub receive_of: BTreeMap<CheckedNodeId, CheckedNodeId>,
}

/// A set of event indices.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct Bits(Vec<u64>);

impl Bits {
    fn new(size: usize) -> Self {
        Self(vec![0; size.div_ceil(64)])
    }

    fn set(&mut self, at: usize) {
        self.0[at / 64] |= 1 << (at % 64);
    }

    fn get(&self, at: usize) -> bool {
        self.0[at / 64] >> (at % 64) & 1 == 1
    }

    fn or(&mut self, other: &Self) {
        for (word, theirs) in self.0.iter_mut().zip(&other.0) {
            *word |= theirs;
        }
    }

    fn members(&self) -> impl Iterator<Item = usize> + '_ {
        self.0.iter().enumerate().flat_map(|(word, bits)| {
            (0..64).filter_map(move |bit| (bits >> bit & 1 == 1).then_some(word * 64 + bit))
        })
    }
}

/// The global event structure of one protocol clause.
#[derive(Clone, Debug)]
pub struct GlobalStructure {
    events: Vec<GlobalEvent>,
    index: BTreeMap<PortRef, usize>,
    below: Vec<Bits>,
    covers: Vec<Vec<usize>>,
    depth: Vec<u64>,
    pub(super) case_bodies: BTreeMap<CheckedNodeId, CheckedNodeId>,
}

fn reachable(adjacency: &[Vec<usize>], from: usize) -> Vec<bool> {
    let mut seen = vec![false; adjacency.len()];
    let mut pending = vec![from];
    while let Some(port) = pending.pop() {
        if std::mem::replace(&mut seen[port], true) {
            continue;
        }
        pending.extend(adjacency[port].iter().copied());
    }
    seen
}

impl GlobalStructure {
    /// Build the structure of `input`.
    ///
    /// # Errors
    ///
    /// [`RunLimit`] when the events exceed `budget.max_states` or the
    /// covering edges exceed `budget.max_transitions`.
    pub fn build(input: StructureInput, budget: RunBudget) -> Result<Self, RunLimit> {
        let StructureInput {
            mut events,
            edges,
            case_bodies,
            uniform_fifo,
            receive_of,
        } = input;
        events.sort_by(|left, right| left.at.cmp(&right.at));
        if events.len() as u64 > budget.max_states {
            return Err(RunLimit {
                limit: "max_states",
                value: budget.max_states,
            });
        }
        let index: BTreeMap<PortRef, usize> = events
            .iter()
            .enumerate()
            .map(|(at, event)| (event.at.clone(), at))
            .collect();

        // The port graph: every port an event or an edge names.
        let mut ports: BTreeMap<PortRef, usize> = BTreeMap::new();
        let mut port = |reference: &PortRef| {
            let next = ports.len();
            *ports.entry(reference.clone()).or_insert(next)
        };
        let mut arcs: BTreeSet<(usize, usize)> = BTreeSet::new();
        for (from, to) in &edges {
            arcs.insert((port(from), port(to)));
        }
        for event in &events {
            let (entry, exit) = (port(&event.at), port(&event.out));
            if entry != exit && event.is_communication() {
                arcs.insert((entry, exit));
            }
        }
        let size = ports.len();
        let entry_port: Vec<usize> = events.iter().map(|event| ports[&event.at]).collect();
        let exit_port: Vec<usize> = events.iter().map(|event| ports[&event.out]).collect();

        let closure = |arcs: &BTreeSet<(usize, usize)>| -> Vec<Bits> {
            let mut adjacency = vec![Vec::new(); size];
            for (from, to) in arcs {
                adjacency[*from].push(*to);
            }
            (0..events.len())
                .map(|source| {
                    let seen = reachable(&adjacency, exit_port[source]);
                    let mut bits = Bits::new(events.len());
                    for target in 0..events.len() {
                        if target != source && seen[entry_port[target]] {
                            bits.set(target);
                        }
                    }
                    bits
                })
                .collect()
        };
        let mut below = closure(&arcs);

        // FIFO: a channel whose key reads nothing of the message delivers in
        // send order, so two receives of it follow their sends' order.
        let uniform: BTreeSet<&CheckedNodeId> =
            uniform_fifo.iter().map(|fifo| &fifo.channel).collect();
        let receives: Vec<(usize, &Comm, &CheckedNodeId)> = events
            .iter()
            .enumerate()
            .filter_map(|(at, event)| match &event.kind {
                GlobalEventKind::Receive(comm) if uniform.contains(&comm.channel) => {
                    Some((at, comm, &event.at.node))
                }
                _ => None,
            })
            .collect();
        let sent_by: BTreeMap<usize, usize> = receives
            .iter()
            .filter_map(|(at, _, node)| {
                let send = PortRef {
                    node: (*receive_of.get(*node)?).clone(),
                    port: Port::Enter,
                };
                Some((*at, *index.get(&send)?))
            })
            .collect();
        let mut added = false;
        for (first, first_comm, _) in &receives {
            for (second, second_comm, _) in &receives {
                if first == second || first_comm.channel != second_comm.channel {
                    continue;
                }
                let (Some(send_first), Some(send_second)) = (sent_by.get(first), sent_by.get(second))
                else {
                    continue;
                };
                if below[*send_first].get(*send_second) && !below[*first].get(*second) {
                    arcs.insert((exit_port[*first], entry_port[*second]));
                    added = true;
                }
            }
        }
        if added {
            below = closure(&arcs);
        }

        // The covering relation: `a` covers `b` when `b` follows `a` and no
        // event lies between.
        let mut covers = vec![Vec::new(); events.len()];
        let mut transitions = 0_u64;
        for (a, row) in below.iter().enumerate() {
            let mut between = Bits::new(events.len());
            for b in row.members() {
                between.or(&below[b]);
            }
            for b in row.members() {
                if !between.get(b) {
                    covers[a].push(b);
                    transitions += 1;
                    if transitions > budget.max_transitions {
                        return Err(RunLimit {
                            limit: "max_transitions",
                            value: budget.max_transitions,
                        });
                    }
                }
            }
        }

        // The depth of an event: the longest chain of events ending at it.
        let mut depth = vec![1_u64; events.len()];
        let mut order: Vec<usize> = (0..events.len()).collect();
        order.sort_by_key(|at| below.iter().filter(|row| row.get(*at)).count());
        for at in order {
            for next in &covers[at] {
                depth[*next] = depth[*next].max(depth[at] + 1);
            }
        }
        Ok(Self {
            events,
            index,
            below,
            covers,
            depth,
            case_bodies,
        })
    }

    /// The events, ascending by entry port.
    #[must_use]
    pub fn events(&self) -> &[GlobalEvent] {
        &self.events
    }

    /// The event entered at `at`.
    #[must_use]
    pub fn event(&self, at: &PortRef) -> Option<&GlobalEvent> {
        self.index.get(at).map(|position| &self.events[*position])
    }

    /// Whether the event `first` precedes the event `second`.
    #[must_use]
    pub fn precedes(&self, first: &PortRef, second: &PortRef) -> bool {
        match (self.index.get(first), self.index.get(second)) {
            (Some(first), Some(second)) => self.below[*first].get(*second),
            _ => false,
        }
    }

    /// The events that directly follow `at`: its covering successors.
    #[must_use]
    pub fn successors(&self, at: &PortRef) -> Vec<&PortRef> {
        self.index.get(at).map_or_else(Vec::new, |position| {
            self.covers[*position]
                .iter()
                .map(|next| &self.events[*next].at)
                .collect()
        })
    }

    /// The length of the longest chain of events ending at `at`.
    #[must_use]
    pub fn depth(&self, at: &PortRef) -> Option<u64> {
        self.index.get(at).map(|position| self.depth[*position])
    }

    /// Whether some event lies beyond a horizon of `max_depth` events.
    #[must_use]
    pub fn is_cut(&self, max_depth: u64) -> bool {
        self.depth.iter().any(|depth| *depth > max_depth)
    }

    /// The covering edges of the structure, ascending.
    #[must_use]
    pub fn covering_edges(&self) -> Vec<(&PortRef, &PortRef)> {
        let mut edges = Vec::new();
        for (from, row) in self.covers.iter().enumerate() {
            for to in row {
                edges.push((&self.events[from].at, &self.events[*to].at));
            }
        }
        edges.sort();
        edges
    }

    /// The shortest chain of events from the choice event `from` (exclusive)
    /// to the event `to` (inclusive) along covering edges, the
    /// lexicographically least among the shortest; `None` when `to` does not
    /// follow `from`.
    #[must_use]
    pub fn shortest_path(&self, from: &PortRef, to: &PortRef) -> Option<Vec<PortRef>> {
        let (&start, &goal) = (self.index.get(from)?, self.index.get(to)?);
        if !self.below[start].get(goal) {
            return None;
        }
        // Distance to `goal` along covering edges, computed backwards.
        let mut distance: BTreeMap<usize, usize> = BTreeMap::from([(goal, 0)]);
        let mut frontier = vec![goal];
        while !frontier.is_empty() {
            let mut next = Vec::new();
            for node in frontier {
                for (before, row) in self.covers.iter().enumerate() {
                    if row.contains(&node) && !distance.contains_key(&before) {
                        distance.insert(before, distance[&node] + 1);
                        next.push(before);
                    }
                }
            }
            frontier = next;
        }
        let mut path = Vec::new();
        let mut at = start;
        while at != goal {
            let remaining = *distance.get(&at)?;
            let step = self.covers[at]
                .iter()
                .filter(|next| distance.get(*next) == Some(&(remaining - 1)))
                .min_by(|left, right| self.events[**left].at.cmp(&self.events[**right].at))?;
            path.push(self.events[*step].at.clone());
            at = *step;
        }
        Some(path)
    }

    /// The enclosing choice case of `event`, nearest first: `(choice, body)`.
    fn nearest_case<'a>(
        &'a self,
        event: &'a GlobalEvent,
    ) -> Option<(&'a CheckedNodeId, &'a CheckedNodeId)> {
        event
            .within
            .iter()
            .rev()
            .find_map(|node| self.case_bodies.get(node).map(|choice| (choice, node)))
    }

    /// The `(direction, peer, message type)` of a communication seen from
    /// `role`: the triple two actions must share to merge.
    #[must_use]
    pub fn action_of(&self, event: &PortRef, role: &CheckedNodeId) -> Option<Action> {
        let event = self.event(event)?;
        match &event.kind {
            GlobalEventKind::Send(comm) if &comm.sender == role => Some(Action {
                direction: Direction::Send,
                peer: comm.receiver.clone(),
                message_type: comm.message_type.clone(),
                at: event.at.clone(),
            }),
            GlobalEventKind::Receive(comm) if &comm.receiver == role => Some(Action {
                direction: Direction::Receive,
                peer: comm.sender.clone(),
                message_type: comm.message_type.clone(),
                at: event.at.clone(),
            }),
            _ => None,
        }
    }

    /// The communications `role` performs directly in the case `body` of
    /// `choice` (not inside a nested case), within a horizon of `max_depth`,
    /// and whether any such communication lies beyond that horizon.
    fn case_actions(
        &self,
        role: &CheckedNodeId,
        choice: &CheckedNodeId,
        body: &CheckedNodeId,
        max_depth: u64,
    ) -> (Vec<Action>, bool) {
        let mut visible = Vec::new();
        let mut hidden = false;
        for (position, event) in self.events.iter().enumerate() {
            if self.nearest_case(event) != Some((choice, body)) {
                continue;
            }
            let Some(action) = self.action_of(&event.at, role) else {
                continue;
            };
            if self.depth[position] > max_depth {
                hidden = true;
            } else {
                visible.push(action);
            }
        }
        (visible, hidden)
    }

    /// The minimal actions of `actions` under the structure's order.
    fn minimal(&self, actions: &[Action]) -> Vec<Action> {
        actions
            .iter()
            .filter(|action| {
                !actions
                    .iter()
                    .any(|other| self.precedes(&other.at, &action.at))
            })
            .cloned()
            .collect()
    }
}

/// Whether a communication sends or receives.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Direction {
    /// A send.
    Send,
    /// A receive.
    Receive,
}

/// One communication seen from a role (PA-10).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Action {
    /// Send or receive.
    pub direction: Direction,
    /// The other role.
    pub peer: CheckedNodeId,
    /// The message type node.
    pub message_type: CheckedNodeId,
    /// The global event.
    pub at: PortRef,
}

impl Action {
    fn shape(&self) -> (Direction, &CheckedNodeId, &CheckedNodeId) {
        (self.direction, &self.peer, &self.message_type)
    }
}

/// Why a role is not made to merge the branches of a choice (PA-10).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChoiceBasis {
    /// The role owns the choice: a local internal choice.
    Owner,
    /// An earlier message the role sent or received carries the outcome: a
    /// local external choice, keyed by that message.
    Informed {
        /// The `receive` control whose payload the decision reads.
        carrier: CheckedNodeId,
    },
    /// Every branch starts with a receive from one peer and the message types
    /// differ across branches: a local external choice keyed by the type.
    ReceiveDistinguished {
        /// The one peer.
        peer: CheckedNodeId,
    },
    /// The role is told nothing: its branches must merge.
    MustMerge,
}

/// One divergent pair of actions at the first layer two branches differ.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Divergence {
    /// The action of the first branch; `None` when that branch has none left.
    pub first: Option<Action>,
    /// The action of the second branch; `None` when that branch has none left.
    pub second: Option<Action>,
}

/// The result of merging two branches for a role.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BranchMerge {
    /// Every layer paired off: the pairs of global events that merge.
    Merged(Vec<(PortRef, PortRef)>),
    /// Some layer did not pair off: every divergent pair at that layer.
    Diverged(Vec<Divergence>),
    /// A branch holds events beyond the horizon, so the pairing is not known.
    Cut,
}

impl GlobalStructure {
    /// How `role` stands at the choice event `choice`, within a horizon of
    /// `max_depth` (PA-10).
    ///
    /// # Panics
    ///
    /// Never for a `choice` that names a choice event of this structure.
    #[must_use]
    pub fn classify(
        &self,
        role: &CheckedNodeId,
        choice: &PortRef,
        max_depth: u64,
    ) -> Option<ChoiceBasis> {
        let event = self.event(choice)?;
        let GlobalEventKind::Choice {
            owner,
            cases,
            carriers,
        } = &event.kind
        else {
            return None;
        };
        if owner == role {
            return Some(ChoiceBasis::Owner);
        }
        for carrier in carriers {
            let receive = PortRef {
                node: carrier.clone(),
                port: Port::Enter,
            };
            let Some(GlobalEventKind::Receive(comm)) = self.event(&receive).map(|e| &e.kind)
            else {
                continue;
            };
            if (&comm.sender == role || &comm.receiver == role)
                && self.precedes(&receive, choice)
            {
                return Some(ChoiceBasis::Informed {
                    carrier: carrier.clone(),
                });
            }
        }
        // Receive-distinguished: one peer sends each branch's first receive
        // and the message types differ across branches.
        let firsts: Vec<Vec<Action>> = cases
            .iter()
            .map(|(_, body)| {
                let (actions, _) = self.case_actions(role, &choice.node, body, max_depth);
                self.minimal(&actions)
            })
            .collect();
        let peers: BTreeSet<&CheckedNodeId> = firsts
            .iter()
            .flatten()
            .filter(|action| action.direction == Direction::Receive)
            .map(|action| &action.peer)
            .collect();
        for peer in peers {
            let types: Vec<BTreeSet<&CheckedNodeId>> = firsts
                .iter()
                .map(|first| {
                    first
                        .iter()
                        .filter(|action| {
                            action.direction == Direction::Receive && &action.peer == peer
                        })
                        .map(|action| &action.message_type)
                        .collect()
                })
                .collect();
            let every_branch = types.iter().all(|held| !held.is_empty());
            let disjoint = types.iter().enumerate().all(|(at, held)| {
                types[at + 1..]
                    .iter()
                    .all(|other| held.is_disjoint(other))
            });
            if every_branch && disjoint {
                return Some(ChoiceBasis::ReceiveDistinguished { peer: peer.clone() });
            }
        }
        Some(ChoiceBasis::MustMerge)
    }

    /// Merge the cases `first` and `second` of the choice `choice` for
    /// `role` (PA-10): layer by layer, the minimal actions of the two cases
    /// pair off by (direction, peer, message type), matched pairs leave, and
    /// the rest recurse; the first layer that does not pair off diverges.
    #[must_use]
    pub fn merge(
        &self,
        role: &CheckedNodeId,
        choice: &CheckedNodeId,
        first: &CheckedNodeId,
        second: &CheckedNodeId,
        max_depth: u64,
    ) -> BranchMerge {
        let (mut left, left_hidden) = self.case_actions(role, choice, first, max_depth);
        let (mut right, right_hidden) = self.case_actions(role, choice, second, max_depth);
        if left_hidden || right_hidden {
            return BranchMerge::Cut;
        }
        let mut merged = Vec::new();
        loop {
            let mut front_left = self.minimal(&left);
            let mut front_right = self.minimal(&right);
            front_left.sort_by(|a, b| (a.shape(), &a.at).cmp(&(b.shape(), &b.at)));
            front_right.sort_by(|a, b| (a.shape(), &a.at).cmp(&(b.shape(), &b.at)));
            if front_left.is_empty() && front_right.is_empty() {
                return BranchMerge::Merged(merged);
            }
            let mut unmatched_left = Vec::new();
            let mut remaining_right = front_right;
            for action in front_left {
                match remaining_right
                    .iter()
                    .position(|other| other.shape() == action.shape())
                {
                    Some(position) => {
                        let other = remaining_right.remove(position);
                        merged.push((action.at.clone(), other.at.clone()));
                        left.retain(|held| held.at != action.at);
                        right.retain(|held| held.at != other.at);
                    }
                    None => unmatched_left.push(action),
                }
            }
            if unmatched_left.is_empty() && remaining_right.is_empty() {
                continue;
            }
            let mut pairs = Vec::new();
            match (unmatched_left.is_empty(), remaining_right.is_empty()) {
                (true, _) => {
                    for b in remaining_right {
                        pairs.push(Divergence {
                            first: None,
                            second: Some(b),
                        });
                    }
                }
                (_, true) => {
                    for a in unmatched_left {
                        pairs.push(Divergence {
                            first: Some(a),
                            second: None,
                        });
                    }
                }
                _ => {
                    for a in &unmatched_left {
                        for b in &remaining_right {
                            pairs.push(Divergence {
                                first: Some(a.clone()),
                                second: Some(b.clone()),
                            });
                        }
                    }
                }
            }
            return BranchMerge::Diverged(pairs);
        }
    }

    /// The events `role` performs and the choices it takes part in, for a
    /// local graph: communications it performs, choices it owns or is
    /// told the outcome of, and the entry and exit of every `parallel` whose
    /// branches hold at least two of those.
    #[must_use]
    pub fn role_events(&self, role: &CheckedNodeId, max_depth: u64) -> Vec<&GlobalEvent> {
        let mut kept: Vec<&GlobalEvent> = self
            .events
            .iter()
            .filter(|event| match &event.kind {
                GlobalEventKind::Send(_) | GlobalEventKind::Receive(_) => {
                    self.action_of(&event.at, role).is_some()
                }
                GlobalEventKind::Choice { .. } => matches!(
                    self.classify(role, &event.at, max_depth),
                    Some(
                        ChoiceBasis::Owner
                            | ChoiceBasis::Informed { .. }
                            | ChoiceBasis::ReceiveDistinguished { .. }
                    )
                ),
                GlobalEventKind::Region { .. } | GlobalEventKind::Join { .. } => false,
            })
            .collect();
        let parallels: Vec<&GlobalEvent> = self
            .events
            .iter()
            .filter(|event| matches!(event.kind, GlobalEventKind::Region { .. }))
            .collect();
        let mut regions = Vec::new();
        for region in parallels {
            let GlobalEventKind::Region { branches } = &region.kind else {
                continue;
            };
            let holding = branches
                .iter()
                .filter(|branch| kept.iter().any(|event| event.within.contains(branch)))
                .count();
            if holding >= 2 {
                regions.push(region);
                let join = PortRef {
                    node: region.at.node.clone(),
                    port: Port::Exit,
                };
                if let Some(event) = self.event(&join) {
                    regions.push(event);
                }
            }
        }
        kept.extend(regions);
        kept.sort_by(|left, right| left.at.cmp(&right.at));
        kept
    }
}
