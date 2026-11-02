// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-519: the protocol read surface of an admitted `quire.checked-package/v2`.
//!
//! The protocol is carried by `protocol` nodes alone (QSpec FR-464 to
//! FR-466); the causal relation is carried by no node and is derived here from
//! the control tree and the cross-references by QSpec FR-464 "Derived causal
//! relation". This module reads the borrowed [`CheckedPackageV2`] Contract IR
//! has already admitted; it never reads package bytes and wraps no reader.
//!
//! The records QSL's record-to-node map marks Ended name nothing here: an
//! `ExportRef`, an `ArtifactKind`, a `Revision`, a `Language` and a `Definition`
//! table each fail to resolve.
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
//! A read answers [`ViewRefusal`] when the package lacks a member the node
//! layout states, naming the node and the member, and never returns a partial
//! view or relation.

use std::collections::BTreeMap;

use quire_contract_model::{
    CheckedArtifactRef, CheckedDependencySelection, CheckedNodeId, CheckedOperationMember,
    CheckedPackageLockV2, CheckedPackageV2, CheckedProtocolActivationRow, CheckedSemanticNodeV2,
    CheckedSourceMapEntry, CheckedSourceRef, CheckedTerm, LiteralKind, TemporalBinding,
};
use serde_json::Value;

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
        Self {
            nodes: package
                .graph()
                .nodes
                .iter()
                .map(|node| (&node.node_id, node))
                .collect(),
        }
    }

    fn clause(&self, id: &CheckedNodeId) -> Result<&'p Node, ViewRefusal> {
        let node = self
            .nodes
            .get(id)
            .copied()
            .filter(|node| &*node.node_tag == "protocol")
            .ok_or_else(|| ViewRefusal::UnknownProtocol { node: id.clone() })?;
        if &*node.semantic_form == "protocol_clause" {
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
fn argument<'n>(node: &'n Node, name: &str) -> Option<&'n CheckedTerm> {
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
fn group_member<'t>(term: &'t CheckedTerm, name: &str) -> Option<&'t CheckedTerm> {
    let CheckedTerm::Aggregate { members } = term else {
        return None;
    };
    members.iter().find_map(|member| match member {
        CheckedTerm::Binding { name: bound, value } if &**bound == name => Some(&**value),
        _ => None,
    })
}

fn reference(term: &CheckedTerm) -> Option<&CheckedNodeId> {
    match term {
        CheckedTerm::Reference { target } => Some(target),
        _ => None,
    }
}

fn text(term: &CheckedTerm) -> Option<&str> {
    match term {
        CheckedTerm::Literal {
            value_kind: LiteralKind::Text,
            value: Value::String(text),
            ..
        } => Some(text),
        _ => None,
    }
}

fn integer(term: &CheckedTerm) -> Option<u64> {
    match term {
        CheckedTerm::Literal {
            value_kind: LiteralKind::Integer,
            value: Value::String(digits),
            ..
        } => digits.parse().ok(),
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
fn children<'n>(node: &'n Node) -> Result<Vec<&'n CheckedNodeId>, ViewRefusal> {
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

    let mut controls = Vec::new();
    let mut pending = vec![run];
    while let Some(id) = pending.pop() {
        controls.push(id);
        let node = index.target(&clause.node_id, "run", id)?;
        pending.extend(children(node)?.into_iter().rev());
    }
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

/// The optional text argument `name`.
fn optional_text<'n>(node: &'n Node, name: &'static str) -> Result<Option<&'n str>, ViewRefusal> {
    optional(node, name)?
        .map(|term| text(term).ok_or_else(|| malformed(&node.node_id, name)))
        .transpose()
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
    let view = protocol_view(package, protocol)?;
    let index = Index::new(package);
    let mut derive = Derive {
        index: &index,
        edges: Vec::new(),
    };
    for &control in view.controls() {
        derive.control(index.target(view.clause, "run", control)?)?;
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

        let after = required(node, "after")?;
        let kind = group_member(after, "kind")
            .and_then(text)
            .ok_or_else(|| malformed(id, "after"))?;
        let target = group_member(after, "target")
            .and_then(reference)
            .ok_or_else(|| malformed(id, "after"))?;
        match kind {
            "event" => self.edge(EdgeKind::AwaitAnchor, port(target, exit), port(id, enter)),
            "compensation" => {
                let template = self.index.target(id, "after", target)?;
                let forward = single(template, "forward")?;
                self.edge(EdgeKind::AwaitAnchor, port(forward, enter), port(id, enter));
            }
            _ => return Err(malformed(id, "after")),
        }
        Ok(())
    }
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
    /// `[lower,upper]`: a declared delivery interval, both bounds inclusive.
    Between {
        /// The least delay.
        lower: u64,
        /// The greatest delay.
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
        .and_then(|(lower, upper)| Some((lower.trim().parse().ok()?, upper.trim().parse().ok()?)));
    bounds
        .map(|(lower, upper)| Delivery::Between { lower, upper })
        .ok_or_else(|| malformed(&channel.node_id, "delivery"))
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

/// The clock binding row of the temporal clause `clause`.
///
/// # Errors
///
/// [`ViewRefusal::MissingMember`] naming `temporal_bindings` when the package
/// has no row of the clause.
pub fn temporal_binding<'p>(
    package: &'p CheckedPackageV2,
    clause: &CheckedNodeId,
) -> Result<&'p TemporalBinding, ViewRefusal> {
    package
        .temporal_bindings()
        .and_then(|rows| rows.iter().find(|row| &row.declaration == clause))
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

/// The locked source document `source`.
///
/// # Errors
///
/// [`ViewRefusal::MissingSelection`] naming `sources`.
pub fn source<'l>(
    lock: &'l CheckedPackageLockV2,
    source: &CheckedSourceRef,
) -> Result<&'l CheckedSourceRef, ViewRefusal> {
    lock.sources
        .iter()
        .find(|locked| *locked == source)
        .ok_or_else(|| ViewRefusal::MissingSelection {
            member: "sources",
            name: source.identity.to_string(),
        })
}

/// The selected definition `definition`.
///
/// # Errors
///
/// [`ViewRefusal::MissingSelection`] naming `definition_selections`.
pub fn definition<'l>(
    lock: &'l CheckedPackageLockV2,
    definition: &CheckedArtifactRef,
) -> Result<&'l CheckedArtifactRef, ViewRefusal> {
    lock.definition_selections
        .iter()
        .find(|selected| *selected == definition)
        .ok_or_else(|| ViewRefusal::MissingSelection {
            member: "definition_selections",
            name: definition.identity.to_string(),
        })
}
