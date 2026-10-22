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

/// The one item a packet, envelope or request selects to replay (QSpec
/// FR-323). It holds typed names only.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayTarget {
    /// The selected function (FR-098).
    Function(QualifiedName),
    /// The operation whose frame is replayed (FR-116).
    FrameOperation(FrameOperation),
    /// The state clause, by its declared name (FR-122).
    StateClause(QualifiedName),
}

impl ReplayTarget {
    /// The FR-323 member this target is carried in.
    pub fn member(&self) -> TargetMember {
        match self {
            Self::Function(_) => TargetMember::Function,
            Self::FrameOperation(_) => TargetMember::FrameOperation,
            Self::StateClause(_) => TargetMember::StateClause,
        }
    }
}

impl fmt::Display for ReplayTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Function(name) | Self::StateClause(name) => {
                write!(f, "{} `{name}`", self.member())
            }
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
            (Some(name), None, None) => Ok(ReplayTarget::Function(name)),
            (None, Some(operation), None) => Ok(ReplayTarget::FrameOperation(operation)),
            (None, None, Some(name)) => Ok(ReplayTarget::StateClause(name)),
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
            ReplayTarget::Function(name) => Self::function(name),
            ReplayTarget::FrameOperation(operation) => Self::frame_operation(operation),
            ReplayTarget::StateClause(name) => Self::state_clause(name),
        }
    }
}

/// The replay target in a request's wire form: the three target members
/// and whether the request also carries the function-only `arguments` and
/// occurrence-key members (QSpec FR-323 decode rule 1 refuses either on a
/// frame or state-clause request).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RequestTargetWire {
    /// The target members.
    pub members: ReplayTargetMembers,
    /// The request carries an `arguments` member.
    pub arguments: bool,
    /// The request carries an `occurrence_key` member.
    pub occurrence_key: bool,
}

impl RequestTargetWire {
    /// A function request carrying neither extra member.
    pub fn function(name: QualifiedName) -> Self {
        ReplayTargetMembers::function(name).into()
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
            occurrence_key: false,
        }
    }
}

impl From<ReplayTarget> for RequestTargetWire {
    fn from(target: ReplayTarget) -> Self {
        ReplayTargetMembers::from(target).into()
    }
}
