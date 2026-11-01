// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-013 O-06: checked member identity.
//!
//! [`Member`] mirrors the v2 `OperationMember` union exactly
//! (`node-identity-preimage.schema.json` `$defs.OperationMember`, resolved
//! from `agent-ix/quire-specification`'s `checked-package-v2` proposal,
//! which ADR-013 O-06 names as this type's serialized authority alongside
//! QSpec FR-322). `declaration` is a [`NodeKey`], unique across packages
//! (O-04); the kernel itself carries a member only as an opaque `MemberId`
//! digest (QC-15, `quire_exact::MemberId`) -- this structured type
//! is QSL's own, not the kernel's.
//!
//! No production caller constructs a [`Member`] yet: the layer-3 check stage
//! that resolves members (O-06's own "Owner" row) is `CheckedGraph`, ADR-013
//! T-1, which is S-3's row. This type and its total wire mapping (its
//! `Serialize`, C-18) are S-2's to land regardless -- ADR-013 §7 gates S-2 on S-1 and the
//! QSpec tickets only, not on S-3 -- exactly as the kernel identity newtypes in
//! `quire-exact::identity` landed in S-1 ahead of the checker that will use
//! them, with tests as their only caller until then.

use qsl_forms::StateClauseKind;
use qsl_foundation::bound::TemporalInterval;
use quire_contract_model::ExecutionPoint;
use quire_exact::Identifier;
use quire_exact::NodeKey;
use quire_exact::Rational;
use quire_semantic_value::fairness::{FairnessGranularity, FairnessKind};

/// The accepted `ClauseBinding.execution_point` (QSpec FR-322-AC-131) as the
/// IR's closed four-variant [`ExecutionPoint`], which the IR owns: this
/// wrapper adds only the `Hash` the IR type lacks, over the variant and its
/// anchor name, so a [`Member`] and a node body stay hashable. Equality and
/// the wire form are the IR type's own.
///
/// quire:canonical
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcceptedPoint(ExecutionPoint);

impl AcceptedPoint {
    /// The accepted point `point`.
    pub fn new(point: ExecutionPoint) -> Self {
        Self(point)
    }

    /// The IR point.
    pub fn point(&self) -> &ExecutionPoint {
        &self.0
    }
}

impl std::hash::Hash for AcceptedPoint {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match &self.0 {
            ExecutionPoint::Initialization { name } => (0_u8, name.as_str()).hash(state),
            ExecutionPoint::Handler { name } => (1_u8, name.as_str()).hash(state),
            ExecutionPoint::Pre { operation } => (2_u8, operation.as_str()).hash(state),
            ExecutionPoint::Post { operation } => (3_u8, operation.as_str()).hash(state),
        }
    }
}

/// One closed checked-member identity (ADR-013 O-06).
///
/// Equality is `derive`d, which already gives ADR-013's own "declared"
/// equality rule -- "(declaring node id, identifier or declared position)"
/// -- directly: two `Member`s are equal only when they are the same variant
/// (the wire union's own `kind` discriminant) with equal fields, so a
/// `Field` and an `Operation` sharing a declaration and name are never
/// confused, matching the schema's own discriminated-union shape.
///
/// quire:canonical
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Member {
    /// A composite type's named field.
    Field {
        /// The declaring composite type.
        declaration: NodeKey,
        /// The field's own identifier.
        name: Identifier,
        /// Source explicitness for a model-object projection. `None` for an
        /// ordinary record projection; `Some` is admitted by IR as its typed
        /// `CheckedFieldReadForm` member projection.
        explicit_deref: Option<bool>,
    },
    /// A tuple's positional element.
    Position {
        /// The declaring tuple type.
        declaration: NodeKey,
        /// The zero-based position.
        position: u64,
    },
    /// A collection's element (position-independent).
    Element {
        /// The declaring collection type.
        declaration: NodeKey,
    },
    /// A relationship's named end.
    RelationshipEnd {
        /// The declaring relationship.
        declaration: NodeKey,
        /// The end's own identifier.
        name: Identifier,
        /// The model declaration node of the destination end's object type
        /// (FR-322): the type a navigation or reachability over this end
        /// arrives at.
        destination_type: NodeKey,
        /// The `bounded_domain`/`model_population` node of the population a
        /// reachability over this end walks, when the unit bounds that
        /// population (FR-365 "The population link", FR-380). Absent from
        /// the wire form, not `null`, when it is `None`.
        population: Option<NodeKey>,
    },
    /// A type's named operation.
    Operation {
        /// The declaring type.
        declaration: NodeKey,
        /// The operation's own identifier.
        name: Identifier,
    },
    /// A generic declaration's type argument (position-independent: a
    /// declaration has at most one type argument in this catalog).
    TypeArgument {
        /// The declaring generic type.
        declaration: NodeKey,
    },
    /// A semantic profile's operator, named directly rather than through a
    /// declaring node -- the only variant with no `declaration` (the schema's
    /// own shape: `{"kind": "profile_operator", "operator": ...}`, no
    /// `declaration` member).
    ProfileOperator {
        /// The operator's own identifier, e.g. `"add"`.
        operator: Identifier,
    },
    /// A `quire.op.state.clause` application's own clause kind (QSpec
    /// STD-111, FR-341): the only difference between an invariant's,
    /// precondition's and postcondition's spellings. Like
    /// [`Self::ProfileOperator`], this variant carries no `declaration` --
    /// the schema's shape is `{"kind": "state_clause", "clause": ...}`.
    StateClause {
        /// Invariant, precondition or postcondition -- an invalid kind is
        /// unrepresentable, since [`StateClauseKind`] is itself a closed enum.
        clause: StateClauseKind,
        /// The accepted `ClauseBinding.execution_point` (QSpec FR-341,
        /// FR-322-AC-131), in the IR's closed four-variant shape; `None`
        /// only when the checked clause has no point. It is part of the
        /// member, so it enters the node's application preimage and key.
        execution_point: Option<AcceptedPoint>,
    },
    /// An interval-capable temporal operator's interval (QSpec FR-370):
    /// `{kind: "temporal_interval", interval}`, with `interval` `null` for
    /// an unbounded operator. Carries no `declaration`.
    TemporalInterval {
        /// The checked interval; `None` for an unbounded operator.
        interval: Option<TemporalInterval>,
    },
    /// A hybrid claim's timed-form interval (QSpec FR-370 timed form,
    /// FR-481 "Hybrid claims"): `{kind: "temporal_interval", interval:
    /// {lower, lower_end, upper, upper_end}}`. Boxed: two exact rationals
    /// would make every member as large.
    TimedInterval(Box<TimedMember>),
    /// One member of a clause's resolved fairness set (QSpec FR-370):
    /// `{kind: "fairness", fairness_kind, granularity, declaration, name}`,
    /// always in its resolved form, so an unmarked constraint and its
    /// written `weak whole` form are one member (ADR-018 FA-6).
    Fairness {
        /// `weak` or `strong`.
        kind: FairnessKind,
        /// `whole` or `each`.
        granularity: FairnessGranularity,
        /// The operation's declaring `model`/`object_type` node.
        declaration: NodeKey,
        /// The operation's own identifier.
        name: Identifier,
    },
}

/// The bounds of a [`Member::TimedInterval`], exact rationals in seconds.
/// `upper` is `None`, with an open upper end, for `[q, *]`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TimedMember {
    /// The lower bound.
    pub lower: Rational,
    /// Whether the lower end is closed.
    pub lower_closed: bool,
    /// The upper bound; `None` for `*`.
    pub upper: Option<Rational>,
    /// Whether the upper end is closed.
    pub upper_closed: bool,
}

impl Member {
    /// The closed wire spelling of a state clause member's kind (FR-341):
    /// `invariant`, `constraint`, `precondition` or `postcondition`; `None`
    /// for any other member kind.
    pub fn state_clause_wire_kind(&self) -> Option<&'static str> {
        match self {
            Self::StateClause { clause, .. } => Some(match clause {
                StateClauseKind::Invariant => "invariant",
                StateClauseKind::Constraint => "constraint",
                StateClauseKind::Precondition => "precondition",
                StateClauseKind::Postcondition => "postcondition",
            }),
            _ => None,
        }
    }

    /// The declaring node this member names, or `None` for a member kind
    /// that names none.
    pub fn declaration(&self) -> Option<NodeKey> {
        match self {
            Self::Field { declaration, .. }
            | Self::Position { declaration, .. }
            | Self::Element { declaration }
            | Self::RelationshipEnd { declaration, .. }
            | Self::Operation { declaration, .. }
            | Self::TypeArgument { declaration }
            | Self::Fairness { declaration, .. } => Some(declaration.clone()),
            Self::ProfileOperator { .. }
            | Self::StateClause { .. }
            | Self::TemporalInterval { .. }
            | Self::TimedInterval(_) => None,
        }
    }

    /// The destination type node a relationship end names, when this member
    /// is one.
    pub fn destination_type(&self) -> Option<NodeKey> {
        match self {
            Self::RelationshipEnd {
                destination_type, ..
            } => Some(destination_type.clone()),
            _ => None,
        }
    }

    /// The population node this member links, when it links one.
    pub fn population(&self) -> Option<NodeKey> {
        match self {
            Self::RelationshipEnd { population, .. } => population.clone(),
            Self::Field { .. }
            | Self::Position { .. }
            | Self::Element { .. }
            | Self::Operation { .. }
            | Self::TypeArgument { .. }
            | Self::ProfileOperator { .. }
            | Self::StateClause { .. }
            | Self::TemporalInterval { .. }
            | Self::TimedInterval(_)
            | Self::Fairness { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(fill: u8) -> NodeKey {
        NodeKey::new(
            "test",
            vec![quire_exact::PathSegment::Text(fill.to_string())],
            0,
        )
        .expect("a test node key")
    }

    /// A field and an operation of one name on one declaration are unequal:
    /// they are different variants of the discriminated union -- not merely
    /// different `(declaration, name)` tuples.
    #[test]
    fn field_and_operation_of_the_same_name_are_unequal() {
        let field = Member::Field {
            declaration: node(7),
            name: Identifier::new("same").unwrap(),
            explicit_deref: None,
        };
        let operation = Member::Operation {
            declaration: node(7),
            name: Identifier::new("same").unwrap(),
        };
        assert_ne!(field, operation);
    }
}
