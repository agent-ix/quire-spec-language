// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-366: the invariant clauses a domain package attaches to its types,
//! parsed by the caller and checked here.
//!
//! Intake keeps each type-level clause's text. The compile driver parses the
//! text of each clause classified `Invariant` with the unit clause production
//! and hands the [`Expression`] to the assembler through
//! [`SelectedModel::type_invariants`], so this layer takes no dependency on
//! S1. The assembler resolves each clause's owner into a
//! [`TypeInvariantDeclaration`]; S3 checks it as a Boolean invariant whose
//! `self` is a value of the owner. A checked invariant adds no declaration,
//! claim or requirement record: the graph keeps its checked body for
//! evaluation per value ([`CheckedTypeInvariant`]).
//!
//! [`Expression`]: qsl_forms::Expression
//! [`SelectedModel::type_invariants`]: crate::model::intake::SelectedModel::type_invariants

use qsl_forms::StateClauseKind;
use quire_exact::{EffectiveId, NodeKey, ValueType};
use quire_semantic_value::location::{Location, Origin};

use super::ir::{CheckedBody, CheckedNode};
use super::refusal::{CheckCause, CheckRefusal};
use super::state_clause::ClauseView;
use crate::model::domain_package::ParsedTypeInvariant;
use crate::model::key::DeclarationKey;
use crate::model::refusal::MemberPath;

/// The type whose values an invariant ranges over, as a value shows it: a
/// record or event value shows the declaration node of its most-specific
/// type, an object its effective identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TypeSlot {
    /// A record value type or an event type, by its composite declaration.
    Composite(NodeKey),
    /// An object type, a state machine or a process, by its effective
    /// identity.
    Object(EffectiveId),
}

/// A [`ParsedTypeInvariant`] with its owner resolved by the assembler.
#[derive(Clone, Debug)]
pub struct TypeInvariantDeclaration {
    /// The clause as the driver parsed it.
    pub invariant: ParsedTypeInvariant,
    /// The type of `self`: a composite of the owner for a record value type
    /// or an event type, a reference to the owner otherwise.
    pub self_type: ValueType,
    /// The owner's effective identity.
    pub context: EffectiveId,
    /// Every type the clause is an effective invariant of: its owner and
    /// each descendant of the owner that does not make it a guard.
    pub applies_to: Vec<TypeSlot>,
}

impl TypeInvariantDeclaration {
    /// What the family check reads: an invariant with no profile selected.
    pub(crate) fn view(&self) -> ClauseView<'_> {
        ClauseView {
            kind: StateClauseKind::Invariant,
            complete_value: false,
            context: Some(self.context),
            receiver: Some(("self", self.self_type.clone())),
            operation: None,
            context_population: None,
            frame_population: None,
            body: &self.invariant.expression,
        }
    }

    /// A refusal of this clause's checking, located at
    /// `TypeDefinition.clauses[k]` of its owner.
    pub(crate) fn refusal(&self, refusal: CheckRefusal) -> CheckRefusal {
        CheckRefusal {
            location: refusal.location,
            cause: CheckCause::TypeInvariant(Box::new(TypeInvariantRefusal {
                clause: self.invariant.clause.clone(),
                position: self.invariant.position,
                cause: refusal.cause,
            })),
        }
    }

    /// The root location of the clause's expression.
    pub(crate) fn location(&self) -> Location {
        Location::root(Origin::Expression)
    }
}

/// A refusal of a type invariant's checking: the clause it is about and the
/// refusal itself (FR-366 "Parse and check").
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TypeInvariantRefusal {
    /// The clause's declaration key.
    pub clause: DeclarationKey,
    /// The clause's position among its owner's clauses, the `k` of
    /// `TypeDefinition.clauses[k]`.
    pub position: usize,
    /// The checking refusal.
    pub cause: CheckCause,
}

impl TypeInvariantRefusal {
    /// `TypeDefinition.clauses[k]`.
    #[must_use]
    pub fn member(&self) -> MemberPath {
        MemberPath::type_definition()
            .name("clauses")
            .index(self.position)
    }
}

impl CheckRefusal {
    /// The type invariant a refusal is about, by its declaration key; `None`
    /// for a refusal located anywhere else.
    #[must_use]
    pub fn type_invariant_clause(&self) -> Option<&DeclarationKey> {
        match &self.cause {
            CheckCause::TypeInvariant(refusal) => Some(&refusal.clause),
            _ => None,
        }
    }

    /// The member a refusal in a type invariant is located at,
    /// `TypeDefinition.clauses[k]`; `None` for a refusal located anywhere
    /// else.
    #[must_use]
    pub fn type_invariant_member(&self) -> Option<MemberPath> {
        match &self.cause {
            CheckCause::TypeInvariant(refusal) => Some(refusal.member()),
            _ => None,
        }
    }

    /// The cause of a refusal in a type invariant, without the clause it is
    /// about.
    #[must_use]
    pub fn type_invariant_cause(&self) -> Option<&CheckCause> {
        match &self.cause {
            CheckCause::TypeInvariant(refusal) => Some(&refusal.cause),
            _ => None,
        }
    }
}

/// One checked type invariant: its owner's clause, the types it is an
/// effective invariant of, and the checked Boolean body evaluation runs with
/// `self` bound to a value of the type.
#[derive(Debug)]
pub struct CheckedTypeInvariant {
    pub(crate) clause: DeclarationKey,
    pub(crate) owner: DeclarationKey,
    pub(crate) applies_to: Vec<TypeSlot>,
    pub(crate) parameters: Vec<(String, ValueType)>,
    pub(crate) body: CheckedBody,
    pub(crate) slots: usize,
}

impl CheckedTypeInvariant {
    /// The clause's declaration key.
    pub fn clause(&self) -> &DeclarationKey {
        &self.clause
    }

    /// The type that owns the clause.
    pub fn owner(&self) -> &DeclarationKey {
        &self.owner
    }

    /// Whether the clause is an effective invariant of the type `slot`.
    pub fn applies_to(&self, slot: TypeSlot) -> bool {
        self.applies_to.contains(&slot)
    }

    /// `self` with its type: the one slot the body is checked over.
    pub fn parameters(&self) -> &[(String, ValueType)] {
        &self.parameters
    }

    /// The checked Boolean body.
    pub fn body(&self) -> CheckedNode<'_> {
        self.body.root()
    }

    /// The number of evaluation slots of the body.
    pub fn slots(&self) -> usize {
        self.slots
    }
}
