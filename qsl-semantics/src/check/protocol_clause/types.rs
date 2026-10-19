//! The checked protocol's identity types: [`ProtocolNodeId`], the S3-level
//! [`BoundAttempt`] and the lowered [`CheckedAttempt`] and
//! [`CheckedProtocol`] (FR-113 Outputs, FR-114 Outputs).

use crate::check::protocol_controls::CheckedChannel;
use crate::check::protocol_obligation::CheckedObligations;
use crate::check::state_clause::{ClauseOperation, PopulationDomain};

/// The identity of one static protocol node: its index into the
/// declaration's own [`ProtocolDeclarationForm::declarations`](qsl_forms::ProtocolDeclarationForm::declarations) (FR-113
/// Outputs: "recorded on the checked protocol by that node's identity"). No
/// later stage recovers a target from a name.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProtocolNodeId(pub usize);

/// One protocol `attempt` FR-114's own S3 binding checked: its resolved
/// operation (already resolved by the FR-091 assembler, `AttemptDeclaration`
/// -- this only checks its `contracts` list against it) and each `contracts`
/// entry's own index into the package's `state_clauses`, in list order.
/// Everything S4 ([`super::super::lowering::Lowering::protocol_attempt`]) needs to
/// mint the operation's anchor and frame node identity, and to name each
/// contract's own checked identity.
#[derive(Clone, Debug)]
pub struct BoundAttempt {
    /// The index into the owning `ProtocolDeclarationForm::declarations` of
    /// this attempt's own static declaration.
    pub declaration: usize,
    /// The resolved operation `on M::T::op` names.
    pub operation: ClauseOperation,
    /// The population domain of the type declaring the operation.
    pub frame_population: Option<PopulationDomain>,
    /// Each `contracts` entry, resolved to the index of the unit's own
    /// `state_clauses` it names, in source order.
    pub contracts: Vec<usize>,
}

/// One protocol `attempt`, fully checked (FR-114 "Outputs"): the
/// identity of its operation's `state`/`operation_anchor` node and
/// `state`/`frame` node (FR-105), and the identity of each state clause its
/// `contracts` list names, in list order. The frame's own entries are never
/// copied here (FR-114: "SHALL record the attempt's frame only by the
/// frame node's identity, and SHALL NOT copy the frame's entries into the
/// attempt"). `check::mod`'s own pipeline fills this in once S4 (lowering)
/// mints the identities; `check` itself returns an empty list here (see
/// its own `Vec<BoundAttempt>` return value for the S3-level binding).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedAttempt {
    /// The index into the owning `ProtocolDeclarationForm::declarations` of
    /// this attempt's own static declaration.
    pub declaration: usize,
    /// The operation's `operation_anchor` node identity.
    pub anchor: quire_exact::NodeKey,
    /// The operation's `frame` node identity.
    pub frame: quire_exact::NodeKey,
    /// Each `contracts` entry's own checked state-clause identity, in list
    /// order.
    pub contracts: Vec<quire_exact::NodeKey>,
}

/// One protocol declaration FR-113 has checked: every scoped anchor's
/// resolved target, in the same order as the form's own `scoped_anchors`.
/// `attempts` starts empty: `check` checks FR-114's `contracts` binding
/// (its own `Vec<BoundAttempt>` return value) but mints no node identity,
/// since that needs S4 (lowering); `check::mod`'s own pipeline fills
/// `attempts` in once it does.
#[derive(Clone, Debug)]
pub struct CheckedProtocol {
    /// The declared protocol name.
    pub name: String,
    /// `resolved_anchors[i]` is the identity `scoped_anchors[i]` resolved
    /// to.
    pub resolved_anchors: Vec<ProtocolNodeId>,
    /// Every `attempt`'s own fully checked binding, filled in after S4.
    pub attempts: Vec<CheckedAttempt>,
    /// The protocol's activation, captures and compensation templates
    /// (FR-381), filled in by `check::mod`'s own pipeline once their
    /// expressions are checked.
    pub obligations: CheckedObligations,
    /// Every channel with its premises, in source order (FR-218 "Channels").
    pub channels: Vec<CheckedChannel>,
    /// The `run` control tree (FR-218 "Outputs").
    pub controls: CheckedControlTree,
    /// The compiled causal edge relation, the `repeat` back edges included
    /// (FR-218 "Outputs").
    pub edges: Vec<CausalEdge>,
    /// Every checked slot of the `run` tree and `finish` -- node bodies,
    /// each `choice` and `repeat` with its owner, visible facts and guards
    /// -- and each binder's readers (FR-218 "Scopes", "Choice and repeat
    /// decisions"), filled in by `check::mod`'s own pipeline once the slots
    /// are checked.
    pub slots: crate::check::protocol_slots::CheckedSlots,
    /// The operation nodes each `compensate` template is bound to,
    /// index-aligned with `obligations.compensations`, filled in after S4
    /// (FR-381 "Compensation registration").
    pub compensation_operations: Vec<CheckedOperationNodes>,
}

/// One port of a control (FR-218 "Outputs", QSpec FR-464 "Derived causal
/// relation"): every control of the `run` tree has an enter and an exit
/// port, and a leaf control is entered before it exits.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Port {
    /// The point at which the control is entered.
    Enter,
    /// The point at which the control is left.
    Exit,
}

/// One port of one control node.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PortRef {
    /// The control the port belongs to.
    pub node: ProtocolNodeId,
    /// Which of its two ports.
    pub port: Port,
}

/// Which rule of the compiled edge relation an edge comes from (QSpec
/// FR-464 "Derived causal relation").
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EdgeKind {
    /// Into, through and out of a `sequence`'s children in authored order.
    Sequence,
    /// Into each case body of a `choice` or branch body of a `parallel`,
    /// and out of each case body.
    Branch,
    /// Out of each branch body a `parallel`'s join list names.
    Join,
    /// Into and out of a `repeat`'s body and its `exhausted` control.
    RepeatProgress,
    /// A `repeat`'s back edge, from the exit of its body to its entry: the
    /// one edge kind the acyclicity rule leaves out.
    RepeatBack,
    /// Into an `await`'s matched event, from it into `then`, and out of
    /// `then`.
    AwaitSuccess,
    /// Into and out of an `await`'s `timeout` control.
    AwaitTimeout,
    /// From the control an `await`'s `after` names to the `await`.
    AwaitAnchor,
    /// From a `send` to each `receive` whose `of` names it.
    Message,
    /// From an `attempt` to each `effect` whose `of` names it.
    Effect,
}

/// One edge of the compiled causal edge relation.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CausalEdge {
    /// The rule the edge comes from.
    pub kind: EdgeKind,
    /// The source port.
    pub from: PortRef,
    /// The target port.
    pub to: PortRef,
}

/// One node of the checked control tree: a control, a `branch` or a `case`
/// of the `run` tree, with the labels of the controls that hold it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedControl {
    /// The node.
    pub node: ProtocolNodeId,
    /// Its kind.
    pub kind: qsl_forms::ProtocolNodeKind,
    /// Its own name.
    pub name: String,
    /// The names of the nodes from the `run` root down to and including
    /// this one.
    pub path: Vec<String>,
    /// Its direct children, in authored order.
    pub children: Vec<ProtocolNodeId>,
}

/// The checked control tree of a protocol's `run` control (FR-218
/// "Outputs"), in tree order: each node before its children, children in
/// authored order.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CheckedControlTree {
    /// Every node of the tree, in tree order; the first is the root.
    pub controls: Vec<CheckedControl>,
}

impl CheckedControlTree {
    /// The tree's root, the `run` control.
    pub fn root(&self) -> Option<&CheckedControl> {
        self.controls.first()
    }

    /// The tree node of `node`, when it is in the tree.
    pub fn control(&self, node: ProtocolNodeId) -> Option<&CheckedControl> {
        self.controls.iter().find(|control| control.node == node)
    }

    /// The tree node whose path is `path`.
    pub fn at_path(&self, path: &[&str]) -> Option<&CheckedControl> {
        self.controls
            .iter()
            .find(|control| control.path.iter().map(String::as_str).eq(path.iter().copied()))
    }
}

/// The operation's shared anchor and frame node a `compensate` template is
/// bound to (FR-381 "Compensation registration"), the nodes every attempt
/// and template naming the operation share.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CheckedOperationNodes {
    /// The operation's `operation_anchor` node identity.
    pub anchor: quire_exact::NodeKey,
    /// The operation's `frame` node identity.
    pub frame: quire_exact::NodeKey,
}
