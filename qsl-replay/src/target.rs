// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-070, FR-071 (QSpec FR-323): the replay target.
//!
//! A counterexample packet, its envelope and the replay request each select
//! the item to replay by exactly one of three FR-323 target members: a
//! function's [`QualifiedName`], a frame's [`FrameOperation`] or a state
//! clause's own declared [`QualifiedName`]. [`ReplayTargetMembers`] is the
//! wire shape (each member optional, so a packet can omit or double one),
//! and [`ReplayTarget`] is the one typed target a reader builds from it.

use std::fmt;

use crate::identity::QualifiedName;
use crate::witness::FrameOperation;
use qsl_foundation::source::provenance::OccurrenceKey;

/// One of QSpec FR-323's replay target members, in FR-323's order.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TargetMember {
    /// `function`: a function's `QualifiedName`.
    Function,
    /// `frame_operation`: `{object, operation}`.
    FrameOperation,
    /// `state_clause`: a state clause's declared `QualifiedName`.
    StateClause,
}

impl TargetMember {
    /// The member's wire name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::FrameOperation => "frame_operation",
            Self::StateClause => "state_clause",
        }
    }

    /// The RFC 6901 pointer of this member in a replay request.
    pub fn pointer(self) -> &'static str {
        match self {
            Self::Function => "/replay/function",
            Self::FrameOperation => "/replay/frame_operation",
            Self::StateClause => "/replay/state_clause",
        }
    }
}

impl fmt::Display for TargetMember {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The replay entries that read a replay target, as a refusal names them.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ReplayEntry {
    /// `replay` (FR-098).
    Replay,
    /// `replay_value_parity` (FR-357).
    ReplayValueParity,
    /// `replay_operator_parity` (FR-357).
    ReplayOperatorParity,
    /// `replay_composite_parity` (FR-358).
    ReplayCompositeParity,
    /// `replay_frame` (FR-116).
    ReplayFrame,
    /// `replay_state_clause` (FR-122).
    ReplayStateClause,
}

impl ReplayEntry {
    /// The entry's function name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Replay => "replay",
            Self::ReplayValueParity => "replay_value_parity",
            Self::ReplayOperatorParity => "replay_operator_parity",
            Self::ReplayCompositeParity => "replay_composite_parity",
            Self::ReplayFrame => "replay_frame",
            Self::ReplayStateClause => "replay_state_clause",
        }
    }
}

impl fmt::Display for ReplayEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Which declaration a named replay target selects.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NamedKind {
    /// A function, by its declared name (`function`).
    Function,
    /// A state clause, by its declared name (`state_clause`).
    StateClause,
}

/// The one item a packet, envelope or request selects (QSpec FR-323). It
/// holds typed names only. A declared name is a component of the target,
/// never an identity on its own (FR-088-AC-6): it is resolved against the
/// recompiled package, so it sits beside the kind of declaration it names.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayTarget {
    /// A function or a state clause, by its declared name (FR-098, FR-122).
    Declaration {
        /// Which kind of declaration the name selects.
        kind: NamedKind,
        /// The declared name.
        name: QualifiedName,
        /// A `function` request's QSpec FR-323 `occurrence_key`: the failing
        /// node's occurrence key, which selects the claim the replay decides
        /// (FR-071, FR-098). Present on every decoded `function` request;
        /// absent on a `state_clause` target and on an envelope's target,
        /// whose packet carries no such member.
        occurrence_key: Option<OccurrenceKey>,
    },
    /// The operation whose frame is replayed (FR-116).
    FrameOperation(FrameOperation),
}

impl ReplayTarget {
    /// A `function` target, with its `occurrence_key` when it comes from a
    /// request.
    pub fn function(name: QualifiedName, occurrence_key: Option<OccurrenceKey>) -> Self {
        Self::Declaration {
            kind: NamedKind::Function,
            name,
            occurrence_key,
        }
    }

    /// A `state_clause` target.
    pub fn state_clause(name: QualifiedName) -> Self {
        Self::Declaration {
            kind: NamedKind::StateClause,
            name,
            occurrence_key: None,
        }
    }

    /// The FR-323 member this target is carried in.
    pub fn member(&self) -> TargetMember {
        match self {
            Self::Declaration {
                kind: NamedKind::Function,
                ..
            } => TargetMember::Function,
            Self::Declaration {
                kind: NamedKind::StateClause,
                ..
            } => TargetMember::StateClause,
            Self::FrameOperation(_) => TargetMember::FrameOperation,
        }
    }

    /// The function name, when this is a `function` target.
    pub fn as_function(&self) -> Option<&QualifiedName> {
        match self {
            Self::Declaration {
                kind: NamedKind::Function,
                name,
                ..
            } => Some(name),
            Self::Declaration {
                kind: NamedKind::StateClause,
                ..
            }
            | Self::FrameOperation(_) => None,
        }
    }

    /// The clause name, when this is a `state_clause` target.
    pub fn as_state_clause(&self) -> Option<&QualifiedName> {
        match self {
            Self::Declaration {
                kind: NamedKind::StateClause,
                name,
                ..
            } => Some(name),
            Self::Declaration {
                kind: NamedKind::Function,
                ..
            }
            | Self::FrameOperation(_) => None,
        }
    }

    /// The `function` request's `occurrence_key`, when it carries one.
    pub fn occurrence_key(&self) -> Option<&OccurrenceKey> {
        match self {
            Self::Declaration { occurrence_key, .. } => occurrence_key.as_ref(),
            Self::FrameOperation(_) => None,
        }
    }

    /// The frame operation, when this is a `frame_operation` target.
    pub fn as_frame_operation(&self) -> Option<&FrameOperation> {
        match self {
            Self::FrameOperation(operation) => Some(operation),
            Self::Declaration { .. } => None,
        }
    }
}

impl fmt::Display for ReplayTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Declaration { name, .. } => write!(f, "{} `{name}`", self.member()),
            Self::FrameOperation(operation) => write!(f, "{} `{operation}`", self.member()),
        }
    }
}

/// The wire shape of the replay target: the three FR-323 members, each
/// present or absent. Exactly one must be present for
/// [`ReplayTargetMembers::select`] to build a [`ReplayTarget`].
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReplayTargetMembers {
    /// The `function` member.
    pub function: Option<QualifiedName>,
    /// The `frame_operation` member.
    pub frame_operation: Option<FrameOperation>,
    /// The `state_clause` member.
    pub state_clause: Option<QualifiedName>,
}

/// [`ReplayTargetMembers::select`]'s refusal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TargetSelectionRefusal {
    /// No target member is present.
    Missing,
    /// More than one target member is present: the members it carries, in
    /// FR-323's order.
    Conflicting {
        /// The members present.
        members: Vec<TargetMember>,
    },
}

impl ReplayTargetMembers {
    /// A `function` target.
    pub fn function(name: QualifiedName) -> Self {
        Self {
            function: Some(name),
            ..Self::default()
        }
    }

    /// A `frame_operation` target.
    pub fn frame_operation(operation: FrameOperation) -> Self {
        Self {
            frame_operation: Some(operation),
            ..Self::default()
        }
    }

    /// A `state_clause` target.
    pub fn state_clause(name: QualifiedName) -> Self {
        Self {
            state_clause: Some(name),
            ..Self::default()
        }
    }

    /// The members present, in FR-323's order.
    fn present(&self) -> Vec<TargetMember> {
        [
            (self.function.is_some(), TargetMember::Function),
            (self.frame_operation.is_some(), TargetMember::FrameOperation),
            (self.state_clause.is_some(), TargetMember::StateClause),
        ]
        .into_iter()
        .filter_map(|(present, member)| present.then_some(member))
        .collect()
    }

    /// The one target these members carry. Refuses none and several; no
    /// target is defaulted or inferred.
    pub fn select(self) -> Result<ReplayTarget, TargetSelectionRefusal> {
        let members = self.present();
        match (self.function, self.frame_operation, self.state_clause) {
            (Some(name), None, None) => Ok(ReplayTarget::function(name, None)),
            (None, Some(operation), None) => Ok(ReplayTarget::FrameOperation(operation)),
            (None, None, Some(name)) => Ok(ReplayTarget::state_clause(name)),
            (None, None, None) => Err(TargetSelectionRefusal::Missing),
            _ => Err(TargetSelectionRefusal::Conflicting { members }),
        }
    }

    /// The members' measured encoded size.
    pub(crate) fn measured_bytes(&self) -> usize {
        self.function
            .iter()
            .chain(self.state_clause.iter())
            .map(|name| name.to_string().len())
            .sum::<usize>()
            + self
                .frame_operation
                .as_ref()
                .map_or(0, |operation| operation.to_string().len())
    }
}

impl From<ReplayTarget> for ReplayTargetMembers {
    fn from(target: ReplayTarget) -> Self {
        match target {
            ReplayTarget::Declaration {
                kind: NamedKind::Function,
                name,
                ..
            } => Self::function(name),
            ReplayTarget::Declaration {
                kind: NamedKind::StateClause,
                name,
                ..
            } => Self::state_clause(name),
            ReplayTarget::FrameOperation(operation) => Self::frame_operation(operation),
        }
    }
}

/// The replay target in a request's wire form: the three target members,
/// the function-only `occurrence_key` and whether the request carries an
/// `arguments` member (QSpec FR-323 decode rule 1: a `function` request
/// requires the occurrence key; a frame or state-clause request refuses it
/// and `arguments`). A function request ignores `arguments`: decode keeps
/// no such member, so [`crate::ReplayRequest::to_wire`] writes `false`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RequestTargetWire {
    /// The target members.
    pub members: ReplayTargetMembers,
    /// The request carries an `arguments` member.
    pub arguments: bool,
    /// The request's `occurrence_key` member, if it carries one.
    pub occurrence_key: Option<OccurrenceKey>,
}

impl RequestTargetWire {
    /// A function request for the claim at `occurrence_key`.
    pub fn function(name: QualifiedName, occurrence_key: OccurrenceKey) -> Self {
        Self {
            occurrence_key: Some(occurrence_key),
            ..ReplayTargetMembers::function(name).into()
        }
    }

    /// A frame request.
    pub fn frame_operation(operation: FrameOperation) -> Self {
        ReplayTargetMembers::frame_operation(operation).into()
    }

    /// A state-clause request.
    pub fn state_clause(name: QualifiedName) -> Self {
        ReplayTargetMembers::state_clause(name).into()
    }
}

impl From<ReplayTargetMembers> for RequestTargetWire {
    fn from(members: ReplayTargetMembers) -> Self {
        Self {
            members,
            arguments: false,
            occurrence_key: None,
        }
    }
}

impl From<ReplayTarget> for RequestTargetWire {
    fn from(target: ReplayTarget) -> Self {
        Self {
            occurrence_key: target.occurrence_key().cloned(),
            ..ReplayTargetMembers::from(target).into()
        }
    }
}

/// A function request's `occurrence_key` for tests: a `declaration`
/// occurrence at ordinal 0.
#[cfg(test)]
pub(crate) fn test_key() -> OccurrenceKey {
    OccurrenceKey::new(
        qsl_foundation::digest::WireNodeId::from_digest([7; 32]),
        quire_exact::Origin::new(quire_exact::Role::new("declaration"), 0),
    )
}
