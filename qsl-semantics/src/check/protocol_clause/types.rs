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
}
