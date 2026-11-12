// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-519: the protocol read surface of an admitted `quire.checked-package/v2`.
//!
//! The protocol is carried by `protocol` nodes alone (QSpec FR-464 to
//! FR-466); the causal relation is carried by no node and is derived here from
//! the control tree and the cross-references by QSpec FR-464 "Derived causal
//! relation". This module reads the borrowed [`CheckedPackageV2`] Contract IR
//! has already admitted; it never reads package bytes and wraps no reader.
//!
//! The records QSL's record-to-node map marks Ended (FR-519-AC-9) name nothing here: an
//! `ExportRef`, an `ArtifactKind`, a `Revision`, a `Language` and a `Definition`
//! table each fail to resolve.
//!
//! The control: the names the module does export resolve.
//!
//! ```
//! use qsl_package::protocol_view::{ControlOperation, Delivery, ViewRefusal};
//! ```
//!
//! ```compile_fail,E0432
//! use qsl_package::protocol_view::ExportRef;
//! ```
//!
//! ```compile_fail,E0432
//! use qsl_package::protocol_view::ArtifactKind;
//! ```
//!
//! ```compile_fail,E0432
//! use qsl_package::protocol_view::Revision;
//! ```
//!
//! ```compile_fail,E0432
//! use qsl_package::protocol_view::Language;
//! ```
//!
//! ```compile_fail,E0432
//! use qsl_package::protocol_view::Definition;
//! ```
//!
//! A window binding has no view (FR-519-AC-13): it is an evaluation input.
//!
//! ```compile_fail,E0432
//! use qsl_package::protocol_view::Window;
//! ```
//!
//! A read answers [`ViewRefusal`] when the package lacks a member the node
//! layout states, naming the node and the member, and never returns a partial
//! view or relation.

use std::collections::{BTreeMap, BTreeSet};

use quire_contract_model::{
    CheckedArtifactRef, CheckedDependencySelection, CheckedNodeId, CheckedOperationMember,
    CheckedPackageLockV2, CheckedPackageV2, CheckedProtocolActivationRow, CheckedSelectionRole,
    CheckedSemanticNodeV2, CheckedSourceMapEntry, CheckedSourceRef, CheckedTerm, LiteralKind,
    TemporalBinding,
};

use crate::wire_form::{has_form, has_tag, WireForm, WireTag};

pub use qsl_semantics::check::{EdgeKind, Port};

type Node = CheckedSemanticNodeV2;

/// Why a protocol read produced no view.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ViewRefusal {
    /// The node is absent from the package, or is not a `protocol` node.
    #[error("the package holds no protocol node {node:?}")]
    UnknownProtocol {
        /// The node id asked for.
        node: CheckedNodeId,
    },
    /// The node is a `protocol` node of another form, such as a
    /// `protocol_reference` carrier, not a `protocol_clause`.
    #[error("the protocol node {node:?} is not a protocol clause")]
    NotAProtocolClause {
        /// The node id asked for.
        node: CheckedNodeId,
    },
    /// The node is absent from the package, or is not a compensation.
    #[error("the package holds no compensation node {node:?}")]
    NotACompensation {
        /// The node id asked for.
        node: CheckedNodeId,
    },
    /// The node lacks a member its layout states.
    #[error("the node {node:?} carries no {member}")]
    MissingMember {
        /// The node that should hold the member.
        node: CheckedNodeId,
        /// The layout's name for the member.
        member: &'static str,
    },
    /// The node holds the member in a shape its layout does not state.
    #[error("the node {node:?} carries {member} in a shape its layout does not state")]
    MalformedMember {
        /// The node holding the member.
        node: CheckedNodeId,
        /// The layout's name for the member.
        member: &'static str,
    },
    /// The package lock lacks a selection the view was asked for.
    #[error("the package lock holds no {member} named {name}")]
    MissingSelection {
        /// The lock member kind.
        member: &'static str,
        /// The identity asked for.
        name: String,
    },
}

/// One port of one control node of the causal relation.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PortRef {
    /// The control the port belongs to.
    pub node: CheckedNodeId,
    /// Which of its two ports.
    pub port: Port,
}

/// One edge of the derived causal relation.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CausalEdge {
    /// The source port.
    pub from: PortRef,
    /// The target port.
    pub to: PortRef,
    /// The rule of QSpec FR-464 the edge comes from.
    pub kind: EdgeKind,
}

/// The causal relation of one protocol clause: strictly ascending and free of
/// duplicates, the `repeat` back edges included.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CausalRelation {
    edges: Vec<CausalEdge>,
}

impl CausalRelation {
    /// The edges, strictly ascending by source port, target port and kind.
    pub fn edges(&self) -> &[CausalEdge] {
        &self.edges
    }
}

/// The operator a `protocol` node's body applies (QSpec FR-464 "Operator
/// catalog").
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ControlOperation {
    /// The clause.
    Protocol,
    /// A role.
    Role,
    /// A channel.
    Channel,
    /// A channel's queue.
    Queue,
    /// The `finish` obligation.
    Finish,
    /// The `terminal` obligation.
    Terminal,
    /// A `sequence` control.
    Sequence,
    /// A `choice` control.
    Choice,
    /// A `parallel` control.
    Parallel,
    /// A `repeat` control.
    Repeat,
    /// An `await` control.
    Await,
    /// A `check` control.
    Check,
    /// A `send` control.
    Send,
    /// A `receive` control.
    Receive,
    /// An `attempt` control.
    Attempt,
    /// An `effect` control.
    Effect,
    /// An `event` control.
    Event,
    /// A `commit` control.
    Commit,
    /// A `fence` control.
    Fence,
    /// A compensation template.
    Compensate,
}

impl ControlOperation {
    /// The control form a wire word names, `None` for any other word.
    #[ix_trace_rs::string_edge]
    fn from_wire(text: &str) -> Option<Self> {
        Some(match text {
            "protocol" => Self::Protocol,
            "role" => Self::Role,
            "channel" => Self::Channel,
            "queue" => Self::Queue,
            "finish" => Self::Finish,
            "terminal" => Self::Terminal,
            "sequence" => Self::Sequence,
            "choice" => Self::Choice,
            "parallel" => Self::Parallel,
            "repeat" => Self::Repeat,
            "await" => Self::Await,
            "check" => Self::Check,
            "send" => Self::Send,
            "receive" => Self::Receive,
            "attempt" => Self::Attempt,
            "effect" => Self::Effect,
            "event" => Self::Event,
            "commit" => Self::Commit,
            "fence" => Self::Fence,
            "compensate" => Self::Compensate,
            _ => return None,
        })
    }
}

/// A clause's `scheduling` member when it is written.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Scheduling {
    /// `scheduling adversarial`.
    Adversarial,
}

/// The nodes of one protocol clause, in the order the nodes carry them.
#[derive(Clone, Debug)]
pub struct ProtocolView<'p> {
    clause: &'p CheckedNodeId,
    roles: Vec<&'p CheckedNodeId>,
    channels: Vec<&'p CheckedNodeId>,
    queues: Vec<&'p CheckedNodeId>,
    controls: Vec<&'p CheckedNodeId>,
    compensations: Vec<&'p CheckedNodeId>,
    requirements: Vec<&'p CheckedNodeId>,
    run: &'p CheckedNodeId,
    finish: &'p CheckedNodeId,
    terminal: Option<&'p CheckedNodeId>,
    scheduling: Option<Scheduling>,
    activation: Vec<CheckedProtocolActivationRow>,
}

impl<'p> ProtocolView<'p> {
    /// The `protocol_clause` node.
    pub fn clause(&self) -> &'p CheckedNodeId {
        self.clause
    }

    /// The `roles` set, in ascending node-id order.
    pub fn roles(&self) -> &[&'p CheckedNodeId] {
        &self.roles
    }

    /// The `channels` set, in ascending node-id order.
    pub fn channels(&self) -> &[&'p CheckedNodeId] {
        &self.channels
    }

    /// The `queues` set, in ascending node-id order.
    pub fn queues(&self) -> &[&'p CheckedNodeId] {
        &self.queues
    }

    /// Every control of the `run` tree in layout order: each control before
    /// the controls it holds; a `sequence`'s children in execution order, the
    /// cases and branches in label order, a `repeat`'s body then its
    /// `exhausted` control, an `await`'s `event`, `then` and `timeout`.
    pub fn controls(&self) -> &[&'p CheckedNodeId] {
        &self.controls
    }

    /// The `compensations` set, in ascending node-id order.
    pub fn compensations(&self) -> &[&'p CheckedNodeId] {
        &self.compensations
    }

    /// The `requirements` set: the temporal clauses the protocol requires.
    pub fn requirements(&self) -> &[&'p CheckedNodeId] {
        &self.requirements
    }

    /// The root `run` control.
    pub fn run(&self) -> &'p CheckedNodeId {
        self.run
    }

    /// The `finish` obligation.
    pub fn finish(&self) -> &'p CheckedNodeId {
        self.finish
    }

    /// The `terminal` obligation, when the clause has one.
    pub fn terminal(&self) -> Option<&'p CheckedNodeId> {
        self.terminal
    }

    /// The `scheduling` member, when written.
    pub fn scheduling(&self) -> Option<Scheduling> {
        self.scheduling
    }

    /// The FR-449 activation rows of this clause (`protocol_activation_relation`).
    pub fn activation(&self) -> &[CheckedProtocolActivationRow] {
        &self.activation
    }
}

struct Index<'p> {
    nodes: BTreeMap<&'p CheckedNodeId, &'p Node>,
}

impl<'p> Index<'p> {
    fn new(package: &'p CheckedPackageV2) -> Self {
        Self::of(&package.resolved_graph().nodes)
    }

    fn of(nodes: &'p [Node]) -> Self {
        Self {
            nodes: nodes.iter().map(|node| (&node.node_id, node)).collect(),
        }
    }

    fn clause(&self, id: &CheckedNodeId) -> Result<&'p Node, ViewRefusal> {
        let node = self
            .nodes
            .get(id)
            .copied()
            .filter(|node| has_tag(node, WireTag::Protocol))
            .ok_or_else(|| ViewRefusal::UnknownProtocol { node: id.clone() })?;
        if has_form(node, WireTag::Protocol, WireForm::ProtocolClause) {
            Ok(node)
        } else {
            Err(ViewRefusal::NotAProtocolClause { node: id.clone() })
        }
    }

    /// The node `target`, named by `owner`'s `member`.
    fn target(
        &self,
        owner: &CheckedNodeId,
        member: &'static str,
        target: &CheckedNodeId,
    ) -> Result<&'p Node, ViewRefusal> {
        self.nodes
            .get(target)
            .copied()
            .ok_or_else(|| missing(owner, member))
    }
}

fn missing(node: &CheckedNodeId, member: &'static str) -> ViewRefusal {
    ViewRefusal::MissingMember {
        node: node.clone(),
        member,
    }
}

fn malformed(node: &CheckedNodeId, member: &'static str) -> ViewRefusal {
    ViewRefusal::MalformedMember {
        node: node.clone(),
        member,
    }
}

/// The argument `name` of `node`'s application body.
pub(crate) fn argument<'n>(node: &'n Node, name: &str) -> Option<&'n CheckedTerm> {
    let CheckedTerm::Application { arguments, .. } = &node.body else {
        return None;
    };
    arguments.iter().find_map(|argument| match argument {
        CheckedTerm::Binding { name: bound, value } if &**bound == name => Some(&**value),
        _ => None,
    })
}

fn required<'n>(node: &'n Node, name: &'static str) -> Result<&'n CheckedTerm, ViewRefusal> {
    argument(node, name).ok_or_else(|| missing(&node.node_id, name))
}

/// The member `name` of the Group `term`.
pub(crate) fn group_member<'t>(term: &'t CheckedTerm, name: &str) -> Option<&'t CheckedTerm> {
    let CheckedTerm::Aggregate { members } = term else {
        return None;
    };
    members.iter().find_map(|member| match member {
        CheckedTerm::Binding { name: bound, value } if &**bound == name => Some(&**value),
        _ => None,
    })
}

pub(crate) fn reference(term: &CheckedTerm) -> Option<&CheckedNodeId> {
    match term {
        CheckedTerm::Reference { target } => Some(target),
        _ => None,
    }
}

pub(crate) fn text(term: &CheckedTerm) -> Option<&str> {
    match term {
        CheckedTerm::Literal {
            value_kind: LiteralKind::Text,
            value,
            ..
        } => value.as_str(),
        _ => None,
    }
}

pub(crate) fn integer(term: &CheckedTerm) -> Option<u64> {
    match term {
        CheckedTerm::Literal {
            value_kind: LiteralKind::Integer,
            value,
            ..
        } => value
            .as_str()
            .filter(|digits| !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()))
            .and_then(|digits| digits.parse().ok()),
        _ => None,
    }
}

fn members<'t>(
    owner: &Node,
    member: &'static str,
    term: &'t CheckedTerm,
) -> Result<&'t [CheckedTerm], ViewRefusal> {
    match term {
        CheckedTerm::Aggregate { members } => Ok(members),
        _ => Err(malformed(&owner.node_id, member)),
    }
}

/// The references an ordered list or set argument `name` of `node` holds.
fn references<'n>(
    node: &'n Node,
    name: &'static str,
) -> Result<Vec<&'n CheckedNodeId>, ViewRefusal> {
    members(node, name, required(node, name)?)?
        .iter()
        .map(|member| reference(member).ok_or_else(|| malformed(&node.node_id, name)))
        .collect()
}

/// The reference argument `name` of `node`.
fn single<'n>(node: &'n Node, name: &'static str) -> Result<&'n CheckedNodeId, ViewRefusal> {
    reference(required(node, name)?).ok_or_else(|| malformed(&node.node_id, name))
}

/// The optional argument `name`: a Group of no member or one.
fn optional<'n>(
    node: &'n Node,
    name: &'static str,
) -> Result<Option<&'n CheckedTerm>, ViewRefusal> {
    match members(node, name, required(node, name)?)? {
        [] => Ok(None),
        [one] => Ok(Some(one)),
        _ => Err(malformed(&node.node_id, name)),
    }
}

/// The `(label, body)` records of the sorted-records argument `name`.
fn records<'n>(
    node: &'n Node,
    name: &'static str,
) -> Result<Vec<(&'n str, &'n CheckedNodeId)>, ViewRefusal> {
    members(node, name, required(node, name)?)?
        .iter()
        .map(|record| {
            let label = group_member(record, "label").and_then(text);
            let body = group_member(record, "body").and_then(reference);
            label
                .zip(body)
                .ok_or_else(|| malformed(&node.node_id, name))
        })
        .collect()
}

/// The operator `node`'s body applies.
///
/// # Errors
///
/// [`ViewRefusal::MissingMember`] when the body is no admitted application
/// naming a profile operator, [`ViewRefusal::MalformedMember`] when it names
/// an operator the protocol profile does not define.
pub fn control_operation(node: &CheckedSemanticNodeV2) -> Result<ControlOperation, ViewRefusal> {
    let CheckedTerm::Application { operation, .. } = &node.body else {
        return Err(missing(&node.node_id, "operation"));
    };
    let read = operation
        .read()
        .map_err(|_| missing(&node.node_id, "operation"))?;
    let Some(CheckedOperationMember::ProfileOperator {
        operator: Some(operator),
    }) = read.member
    else {
        return Err(missing(&node.node_id, "operator"));
    };
    ControlOperation::from_wire(operator).ok_or_else(|| malformed(&node.node_id, "operator"))
}

/// The catalogued operation identity of `node`'s application body: the
/// `quire.op.protocol.control`, `quire.op.temporal.*` or value operation the
/// node applies.
///
/// # Errors
///
/// [`ViewRefusal::MissingMember`] when the body is no admitted application.
pub fn operation_identity(node: &CheckedSemanticNodeV2) -> Result<&str, ViewRefusal> {
    let CheckedTerm::Application { operation, .. } = &node.body else {
        return Err(missing(&node.node_id, "operation"));
    };
    operation
        .identity()
        .ok_or_else(|| missing(&node.node_id, "operation"))
}

/// The children of the control `node`, in layout order.
fn children(node: &Node) -> Result<Vec<&CheckedNodeId>, ViewRefusal> {
    Ok(match control_operation(node)? {
        ControlOperation::Sequence => references(node, "children")?,
        ControlOperation::Choice => records(node, "cases")?
            .into_iter()
            .map(|(_, body)| body)
            .collect(),
        ControlOperation::Parallel => records(node, "branches")?
            .into_iter()
            .map(|(_, body)| body)
            .collect(),
        ControlOperation::Repeat => {
            let mut held = vec![single(node, "body")?];
            if let Some(exhausted) = optional(node, "exhausted")? {
                held.push(
                    reference(exhausted).ok_or_else(|| malformed(&node.node_id, "exhausted"))?,
                );
            }
            held
        }
        ControlOperation::Await => vec![
            single(node, "event")?,
            single(node, "then")?,
            single(node, "timeout")?,
        ],
        _ => Vec::new(),
    })
}

/// The view of the protocol clause `protocol`.
///
/// # Errors
///
/// [`ViewRefusal::UnknownProtocol`] for an absent node or a node that is not
/// a `protocol` node, [`ViewRefusal::NotAProtocolClause`] for a
/// `protocol_reference` carrier or another member form, and a
/// [`ViewRefusal`] naming the member when the clause or a node it lists lacks
/// one.
pub fn protocol_view<'p>(
    package: &'p CheckedPackageV2,
    protocol: &CheckedNodeId,
) -> Result<ProtocolView<'p>, ViewRefusal> {
    let index = Index::new(package);
    let clause = index.clause(protocol)?;
    let run = single(clause, "run")?;
    let finish = single(clause, "finish")?;
    let terminal = optional(clause, "terminal")?
        .map(|term| reference(term).ok_or_else(|| malformed(&clause.node_id, "terminal")))
        .transpose()?;
    let scheduling = match optional_text(clause, "scheduling")? {
        None => None,
        Some("adversarial") => Some(Scheduling::Adversarial),
        Some(_) => return Err(malformed(&clause.node_id, "scheduling")),
    };

    let controls = control_tree(&index, clause)?;
    let activation = package
        .protocol_activation_relation()
        .into_iter()
        .filter(|row| &row.clause == protocol)
        .collect();
    Ok(ProtocolView {
        clause: &clause.node_id,
        roles: references(clause, "roles")?,
        channels: references(clause, "channels")?,
        queues: references(clause, "queues")?,
        controls,
        compensations: references(clause, "compensations")?,
        requirements: references(clause, "requirements")?,
        run,
        finish,
        terminal,
        scheduling,
        activation,
    })
}

/// The controls of `clause`'s `run` tree in layout order. A control held by two
/// positions is refused: the tree holds each control once (QSpec FR-464
/// "Membership and containment"), and the walk never revisits one.
fn control_tree<'p>(
    index: &Index<'p>,
    clause: &'p Node,
) -> Result<Vec<&'p CheckedNodeId>, ViewRefusal> {
    let mut controls = Vec::new();
    let mut seen = BTreeSet::new();
    let mut pending = vec![single(clause, "run")?];
    while let Some(id) = pending.pop() {
        if !seen.insert(id) {
            return Err(malformed(&clause.node_id, "run"));
        }
        controls.push(id);
        let node = index.target(&clause.node_id, "run", id)?;
        pending.extend(children(node)?.into_iter().rev());
    }
    Ok(controls)
}

/// The optional text argument `name`.
fn optional_text<'n>(node: &'n Node, name: &'static str) -> Result<Option<&'n str>, ViewRefusal> {
    optional(node, name)?
        .map(|term| text(term).ok_or_else(|| malformed(&node.node_id, name)))
        .transpose()
}

/// The retry bound of the compensation node `compensation`: the `maximum`
/// member of its `attempts` group, a `u64` of at least one that counts the
/// initial attempt (FR-381 `maximum_attempts`).
///
/// # Errors
///
/// [`ViewRefusal::NotACompensation`] for an absent node or a node that is not
/// a compensation, [`ViewRefusal::MissingMember`] naming `attempts.maximum`
/// when the group lacks it, [`ViewRefusal::MalformedMember`] naming
/// `attempts.maximum` when it is not an integer or is zero.
pub fn attempts_maximum(
    package: &CheckedPackageV2,
    compensation: &CheckedNodeId,
) -> Result<u64, ViewRefusal> {
    attempts_over(&package.resolved_graph().nodes, compensation)
}

/// [`attempts_maximum`] over the graph nodes themselves.
pub(crate) fn attempts_over(
    nodes: &[CheckedSemanticNodeV2],
    compensation: &CheckedNodeId,
) -> Result<u64, ViewRefusal> {
    let node = Index::of(nodes)
        .nodes
        .get(compensation)
        .copied()
        .filter(|node| has_form(node, WireTag::Protocol, WireForm::Compensation))
        .ok_or_else(|| ViewRefusal::NotACompensation {
            node: compensation.clone(),
        })?;
    let group = required(node, "attempts")?;
    let maximum =
        group_member(group, "maximum").ok_or_else(|| missing(compensation, "attempts.maximum"))?;
    integer(maximum)
        .filter(|count| *count >= 1)
        .ok_or_else(|| malformed(compensation, "attempts.maximum"))
}

/// The causal relation of the protocol clause `protocol`, derived from its
/// control tree and cross-references by QSpec FR-464 "Derived causal
/// relation".
///
/// # Errors
///
/// As [`protocol_view`].
pub fn causal_relation(
    package: &CheckedPackageV2,
    protocol: &CheckedNodeId,
) -> Result<CausalRelation, ViewRefusal> {
    relation_over(&package.resolved_graph().nodes, protocol)
}

/// [`causal_relation`] over the graph nodes themselves.
pub(crate) fn relation_over(
    nodes: &[CheckedSemanticNodeV2],
    protocol: &CheckedNodeId,
) -> Result<CausalRelation, ViewRefusal> {
    let index = Index::of(nodes);
    let clause = index.clause(protocol)?;
    let mut derive = Derive {
        index: &index,
        edges: Vec::new(),
    };
    for control in control_tree(&index, clause)? {
        derive.control(index.target(&clause.node_id, "run", control)?)?;
    }
    let mut edges = derive.edges;
    edges.sort_unstable();
    edges.dedup();
    Ok(CausalRelation { edges })
}

struct Derive<'a, 'p> {
    index: &'a Index<'p>,
    edges: Vec<CausalEdge>,
}

fn port(node: &CheckedNodeId, port: Port) -> PortRef {
    PortRef {
        node: node.clone(),
        port,
    }
}

impl Derive<'_, '_> {
    fn edge(&mut self, kind: EdgeKind, from: PortRef, to: PortRef) {
        self.edges.push(CausalEdge { from, to, kind });
    }

    fn control(&mut self, node: &Node) -> Result<(), ViewRefusal> {
        let id = &node.node_id;
        let (enter, exit) = (Port::Enter, Port::Exit);
        match control_operation(node)? {
            ControlOperation::Sequence => {
                let held = references(node, "children")?;
                match (held.first(), held.last()) {
                    (Some(first), Some(last)) => {
                        self.edge(EdgeKind::Sequence, port(id, enter), port(first, enter));
                        for pair in held.windows(2) {
                            self.edge(
                                EdgeKind::Sequence,
                                port(pair[0], exit),
                                port(pair[1], enter),
                            );
                        }
                        self.edge(EdgeKind::Sequence, port(last, exit), port(id, exit));
                    }
                    _ => self.edge(EdgeKind::Sequence, port(id, enter), port(id, exit)),
                }
            }
            ControlOperation::Choice => {
                for (_, body) in records(node, "cases")? {
                    self.edge(EdgeKind::Branch, port(id, enter), port(body, enter));
                    self.edge(EdgeKind::Branch, port(body, exit), port(id, exit));
                }
            }
            ControlOperation::Parallel => {
                let joined = group_texts(node, "joined")?;
                for (label, body) in records(node, "branches")? {
                    self.edge(EdgeKind::Branch, port(id, enter), port(body, enter));
                    if joined.contains(&label) {
                        self.edge(EdgeKind::Join, port(body, exit), port(id, exit));
                    }
                }
            }
            ControlOperation::Repeat => {
                let body = single(node, "body")?;
                self.edge(EdgeKind::RepeatProgress, port(id, enter), port(body, enter));
                self.edge(EdgeKind::RepeatProgress, port(body, exit), port(id, exit));
                if let Some(exhausted) = optional(node, "exhausted")? {
                    let exhausted =
                        reference(exhausted).ok_or_else(|| malformed(id, "exhausted"))?;
                    self.edge(
                        EdgeKind::RepeatProgress,
                        port(id, enter),
                        port(exhausted, enter),
                    );
                    self.edge(
                        EdgeKind::RepeatProgress,
                        port(exhausted, exit),
                        port(id, exit),
                    );
                }
                self.edge(EdgeKind::RepeatBack, port(body, exit), port(body, enter));
            }
            ControlOperation::Await => self.awaiting(node)?,
            ControlOperation::Receive => {
                let sent = single(node, "of")?;
                self.edge(EdgeKind::Message, port(sent, exit), port(id, enter));
            }
            ControlOperation::Effect => {
                let attempt = single(node, "of")?;
                self.edge(EdgeKind::Effect, port(attempt, exit), port(id, enter));
            }
            ControlOperation::Protocol
            | ControlOperation::Role
            | ControlOperation::Channel
            | ControlOperation::Queue
            | ControlOperation::Finish
            | ControlOperation::Terminal
            | ControlOperation::Check
            | ControlOperation::Send
            | ControlOperation::Attempt
            | ControlOperation::Event
            | ControlOperation::Commit
            | ControlOperation::Fence
            | ControlOperation::Compensate => {}
        }
        Ok(())
    }

    fn awaiting(&mut self, node: &Node) -> Result<(), ViewRefusal> {
        let id = &node.node_id;
        let (enter, exit) = (Port::Enter, Port::Exit);
        let (event, then, timeout) = (
            single(node, "event")?,
            single(node, "then")?,
            single(node, "timeout")?,
        );
        self.edge(EdgeKind::AwaitSuccess, port(id, enter), port(event, enter));
        self.edge(EdgeKind::AwaitSuccess, port(event, exit), port(then, enter));
        self.edge(EdgeKind::AwaitSuccess, port(then, exit), port(id, exit));
        self.edge(
            EdgeKind::AwaitTimeout,
            port(id, enter),
            port(timeout, enter),
        );
        self.edge(EdgeKind::AwaitTimeout, port(timeout, exit), port(id, exit));

        let AwaitAnchor { kind, target } = await_anchor(node)?;
        match kind {
            AwaitAnchorKind::Event => {
                self.edge(EdgeKind::AwaitAnchor, port(target, exit), port(id, enter));
            }
            AwaitAnchorKind::Compensation => {
                let template = self.index.target(id, "after", target)?;
                let forward = single(template, "forward")?;
                self.edge(EdgeKind::AwaitAnchor, port(forward, enter), port(id, enter));
            }
        }
        Ok(())
    }
}

/// What an `await` clause's `after` anchor names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AwaitAnchorKind {
    /// An `event` control: the await follows it.
    Event,
    /// A `compensation` template: the await follows its forward body.
    Compensation,
}

impl AwaitAnchorKind {
    /// The anchor a wire word names, `None` for any other word.
    #[ix_trace_rs::string_edge]
    fn from_wire(word: &str) -> Option<Self> {
        match word {
            "event" => Some(Self::Event),
            "compensation" => Some(Self::Compensation),
            _ => None,
        }
    }
}

/// The `after` anchor of an `await` control: what kind of node it names and
/// that node's id.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AwaitAnchor<'n> {
    /// What the anchor names.
    pub kind: AwaitAnchorKind,
    /// The `event` control or `compensation` template the await follows.
    pub target: &'n CheckedNodeId,
}

/// The `after` anchor of the `await` control `node`.
///
/// # Errors
///
/// [`ViewRefusal::MissingMember`] naming `after` when the node has none,
/// [`ViewRefusal::MalformedMember`] when `after` is no `kind`/`target` group
/// of a known kind and a reference.
pub fn await_anchor(node: &CheckedSemanticNodeV2) -> Result<AwaitAnchor<'_>, ViewRefusal> {
    let after = required(node, "after")?;
    let wrong = || malformed(&node.node_id, "after");
    let kind = group_member(after, "kind")
        .and_then(text)
        .and_then(AwaitAnchorKind::from_wire)
        .ok_or_else(wrong)?;
    let target = group_member(after, "target")
        .and_then(reference)
        .ok_or_else(wrong)?;
    Ok(AwaitAnchor { kind, target })
}

/// The texts of the Group of texts `name` of `node`.
fn group_texts<'n>(node: &'n Node, name: &'static str) -> Result<Vec<&'n str>, ViewRefusal> {
    members(node, name, required(node, name)?)?
        .iter()
        .map(|member| text(member).ok_or_else(|| malformed(&node.node_id, name)))
        .collect()
}

/// A channel's delivery premise (QSpec FR-465 "Channel", QSL FR-509).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Delivery {
    /// `unknown`.
    Unknown,
    /// `at-most-once`.
    AtMostOnce,
    /// `at-least-once`.
    AtLeastOnce,
    /// `exactly-once-premise`.
    ExactlyOncePremise,
    /// `[lower,upper]`: a declared delivery count interval (QSL FR-509), both
    /// bounds inclusive.
    Between {
        /// The least count.
        lower: u64,
        /// The greatest count.
        upper: u64,
    },
}

/// The delivery premise the channel node `channel` carries.
///
/// # Errors
///
/// [`ViewRefusal::MissingMember`] naming `delivery` when the node has none,
/// [`ViewRefusal::MalformedMember`] when its text is no policy word or
/// interval.
#[ix_trace_rs::string_edge]
pub fn delivery(channel: &CheckedSemanticNodeV2) -> Result<Delivery, ViewRefusal> {
    let word = text(required(channel, "delivery")?)
        .ok_or_else(|| malformed(&channel.node_id, "delivery"))?;
    match word {
        "unknown" => return Ok(Delivery::Unknown),
        "at-most-once" => return Ok(Delivery::AtMostOnce),
        "at-least-once" => return Ok(Delivery::AtLeastOnce),
        "exactly-once-premise" => return Ok(Delivery::ExactlyOncePremise),
        _ => {}
    }
    let bounds = word
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .and_then(|inner| inner.split_once(','))
        .and_then(|(lower, upper)| Some((canonical_u64(lower)?, canonical_u64(upper)?)))
        .filter(|(lower, upper)| lower <= upper);
    bounds
        .map(|(lower, upper)| Delivery::Between { lower, upper })
        .ok_or_else(|| malformed(&channel.node_id, "delivery"))
}

fn canonical_u64(value: &str) -> Option<u64> {
    if value.is_empty()
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return None;
    }
    value.parse().ok()
}

/// The endpoints and message type of a channel (QSpec FR-465 "Channel").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChannelEndpoints<'n> {
    /// The `role` node that sends.
    pub sender: &'n CheckedNodeId,
    /// The `role` node that receives.
    pub receiver: &'n CheckedNodeId,
    /// The type node of the message the channel carries.
    pub message_type: &'n CheckedNodeId,
}

/// The sender, receiver and message type the channel node `channel` carries.
///
/// # Errors
///
/// [`ViewRefusal::MissingMember`] naming `from`, `to` or `carries` when the
/// node lacks it, [`ViewRefusal::MalformedMember`] when it is no reference.
pub fn channel_endpoints(
    channel: &CheckedSemanticNodeV2,
) -> Result<ChannelEndpoints<'_>, ViewRefusal> {
    Ok(ChannelEndpoints {
        sender: single(channel, "from")?,
        receiver: single(channel, "to")?,
        message_type: single(channel, "carries")?,
    })
}

/// A queue's capacity (QSpec FR-465 "Queue").
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QueueCapacity<'n> {
    /// A written integer capacity.
    Literal(u64),
    /// A symbolic capacity, by name.
    Symbolic(&'n str),
}

/// The capacity the queue node `queue` carries.
///
/// # Errors
///
/// [`ViewRefusal::MissingMember`] naming `capacity` when the node has none,
/// [`ViewRefusal::MalformedMember`] when it is neither a literal nor a
/// symbolic capacity.
pub fn queue_capacity(queue: &CheckedSemanticNodeV2) -> Result<QueueCapacity<'_>, ViewRefusal> {
    let group = required(queue, "capacity")?;
    let wrong = || malformed(&queue.node_id, "capacity");
    match group_member(group, "kind").and_then(text) {
        Some("literal") => group_member(group, "value")
            .and_then(|value| integer(value).or_else(|| text(value)?.parse().ok()))
            .map(QueueCapacity::Literal)
            .ok_or_else(wrong),
        Some("symbolic") => group_member(group, "name")
            .and_then(text)
            .map(QueueCapacity::Symbolic)
            .ok_or_else(wrong),
        _ => Err(wrong()),
    }
}

/// The choice-visibility evidence of a `choice` control: who decides and which
/// `expression` nodes that role sees at the decision (QSpec FR-466 "choice",
/// FR-172-AC-6). It is not the FR-227 visibility record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChoiceVisibility<'n> {
    /// The `choice` control the evidence belongs to.
    pub choice: &'n CheckedNodeId,
    /// The `protocol` or `role` node that owns the decision.
    pub owner: &'n CheckedNodeId,
    /// The `expression` nodes whose values the owner sees, in the order the
    /// node carries them.
    pub visible: Vec<&'n CheckedNodeId>,
}

/// The owner and visible set of the `choice` control `node`.
///
/// # Errors
///
/// [`ViewRefusal::MissingMember`] naming `operation` or `operator` when the
/// node is no application of a profile operator (a role or a value, for
/// example), [`ViewRefusal::MalformedMember`] naming `operator` when it names
/// an operator the protocol profile does not define or a control of another
/// operation, [`ViewRefusal::MissingMember`] naming `owner` or `visible` when
/// the node lacks it, [`ViewRefusal::MalformedMember`] naming that member when
/// `owner` is no reference or `visible` is no set of references.
pub fn choice_visibility(
    node: &CheckedSemanticNodeV2,
) -> Result<ChoiceVisibility<'_>, ViewRefusal> {
    if control_operation(node)? != ControlOperation::Choice {
        return Err(malformed(&node.node_id, "operator"));
    }
    Ok(ChoiceVisibility {
        choice: &node.node_id,
        owner: single(node, "owner")?,
        visible: references(node, "visible")?,
    })
}

/// A binder a control introduces: its node, name and level (QSpec FR-464
/// "Binders").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Binder<'p> {
    /// The `value`/`parameter` node.
    pub node: &'p CheckedNodeId,
    /// The binder's name.
    pub name: &'p str,
    /// The binder's level.
    pub level: u64,
}

/// The binder the control `control` introduces through its `binder` argument.
///
/// # Errors
///
/// [`ViewRefusal::MissingMember`] naming `binder` when the control has none
/// or its target is absent, [`ViewRefusal::MalformedMember`] when the target
/// body is no `name`/`level` aggregate.
pub fn binder<'p>(
    package: &'p CheckedPackageV2,
    control: &'p CheckedSemanticNodeV2,
) -> Result<Binder<'p>, ViewRefusal> {
    let target = single(control, "binder")?;
    let node = Index::new(package).target(&control.node_id, "binder", target)?;
    let name = group_member(&node.body, "name").and_then(text);
    let level = group_member(&node.body, "level").and_then(integer);
    name.zip(level)
        .map(|(name, level)| Binder {
            node: &node.node_id,
            name,
            level,
        })
        .ok_or_else(|| malformed(&control.node_id, "binder"))
}

/// The model declaration node owning the clause `clause`.
///
/// # Errors
///
/// [`ViewRefusal::MissingMember`] naming `owner_model` when the clause has no
/// edge.
pub fn owner_model(clause: &CheckedSemanticNodeV2) -> Result<&CheckedNodeId, ViewRefusal> {
    clause
        .owner_model
        .as_ref()
        .ok_or_else(|| missing(&clause.node_id, "owner_model"))
}

/// The declaration `node` makes: its identity triple (`node_id`) and its
/// qualified name.
///
/// # Errors
///
/// [`ViewRefusal::MissingMember`] naming `declaration` when the node has none.
pub fn declaration(
    node: &CheckedSemanticNodeV2,
) -> Result<(&CheckedNodeId, &[Box<str>]), ViewRefusal> {
    node.declaration
        .as_ref()
        .map(|declared| (&node.node_id, declared.qualified_name.as_slice()))
        .ok_or_else(|| missing(&node.node_id, "declaration"))
}

/// The source-map entries of the node `node`: its locus.
///
/// # Errors
///
/// [`ViewRefusal::MissingMember`] naming `source_map` when the map holds no
/// entry of the node.
pub fn locus<'p>(
    source_map: &'p [CheckedSourceMapEntry],
    node: &CheckedNodeId,
) -> Result<Vec<&'p CheckedSourceMapEntry>, ViewRefusal> {
    let found: Vec<_> = source_map
        .iter()
        .filter(|entry| &entry.node_id == node)
        .collect();
    if found.is_empty() {
        Err(missing(node, "source_map"))
    } else {
        Ok(found)
    }
}

/// The clock binding row of the temporal clause `clause` among the package's
/// `temporal_bindings` rows (`CheckedPackageV2::temporal_bindings`).
///
/// # Errors
///
/// [`ViewRefusal::MissingMember`] naming `temporal_bindings` when `rows` has no
/// row of the clause.
pub fn temporal_binding<'p>(
    rows: &'p [TemporalBinding],
    clause: &CheckedNodeId,
) -> Result<&'p TemporalBinding, ViewRefusal> {
    rows.iter()
        .find(|row| &row.declaration == clause)
        .ok_or_else(|| missing(clause, "temporal_bindings"))
}

/// The dependency selection of the library `identity`.
///
/// # Errors
///
/// [`ViewRefusal::MissingSelection`] naming `dependency_selections`.
pub fn dependency<'l>(
    lock: &'l CheckedPackageLockV2,
    identity: &str,
) -> Result<&'l CheckedDependencySelection, ViewRefusal> {
    lock.dependency_selections
        .iter()
        .find(|selection| &*selection.identity == identity)
        .ok_or_else(|| ViewRefusal::MissingSelection {
            member: "dependency_selections",
            name: identity.to_owned(),
        })
}

/// The locked source document of `identity`.
///
/// # Errors
///
/// [`ViewRefusal::MissingSelection`] naming `sources`.
pub fn source<'l>(
    lock: &'l CheckedPackageLockV2,
    identity: &str,
) -> Result<&'l CheckedSourceRef, ViewRefusal> {
    lock.sources
        .iter()
        .find(|locked| &*locked.identity == identity)
        .ok_or_else(|| ViewRefusal::MissingSelection {
            member: "sources",
            name: identity.to_owned(),
        })
}

/// The selection the lock makes for `role`: the profile selection a node's law
/// names (`{ authority, identity }` of the definition only).
///
/// # Errors
///
/// [`ViewRefusal::MissingSelection`] naming `profile_selections`.
pub fn selection(
    lock: &CheckedPackageLockV2,
    role: CheckedSelectionRole,
) -> Result<&CheckedArtifactRef, ViewRefusal> {
    lock.profile_selections
        .iter()
        .find(|selected| selected.role == role)
        .map(|selected| &selected.definition)
        .ok_or_else(|| ViewRefusal::MissingSelection {
            member: "profile_selections",
            name: format!("{role:?}"),
        })
}
