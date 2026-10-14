// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-103: the `requires` and `ensures` clauses a domain package authors on
//! its own operation, parsed by the caller and checked here.
//!
//! Intake keeps each clause's text ([`OperationContractClause`]) and does not
//! parse it. The compile driver parses it with the unit clause production
//! (`qsl_forms::parse_clause_text`) and hands the [`Expression`] to the
//! assembler through [`SelectedModel::operation_clauses`], so this layer takes
//! no dependency on S1. The assembler resolves each clause's operation into an
//! [`OperationClauseDeclaration`]; S3 checks it as a Boolean under the kind
//! FR-104 gives the position. A kept clause names no `using` profile, so it is
//! checked under the typer's profile-independent defaults. Checking yields a
//! refusal or nothing: a kept clause adds no checked state clause, claim or
//! requirement record.
//!
//! [`OperationContractClause`]: crate::model::domain_package::OperationContractClause
//! [`SelectedModel::operation_clauses`]: crate::model::intake::SelectedModel::operation_clauses

use qsl_forms::StateClauseKind;
use quire_exact::EffectiveId;
use quire_semantic_value::location::{Location, OperationClauseKind, Origin};

use super::refusal::CheckRefusal;
use super::state_clause::{ClauseOperation, ClauseView};
use crate::model::domain_package::ParsedOperationClause;
use crate::model::refusal::{MemberPath, MemberRoot};

/// A [`ParsedOperationClause`] with its operation resolved by the assembler.
#[derive(Clone, Debug)]
pub struct OperationClauseDeclaration {
    /// The clause as the driver parsed it.
    pub clause: ParsedOperationClause,
    /// The object type that owns the operation, and so binds `self`.
    pub context: EffectiveId,
    /// The operation, resolved in its owner's own declarations.
    pub operation: ClauseOperation,
}

impl OperationClauseDeclaration {
    /// Where the clause's checking refusals are located.
    pub(crate) fn origin(&self) -> Origin {
        Origin::ModelOperationClause {
            operation: self.clause.operation.node.clone(),
            kind: self.clause.kind,
            position: self.clause.position,
        }
    }

    /// The root location of the clause's expression.
    pub(crate) fn location(&self) -> Location {
        Location::root(self.origin())
    }

    /// What the family check reads: a `requires` is a precondition and an
    /// `ensures` a postcondition, with no profile selected.
    pub(crate) fn view(&self) -> ClauseView<'_> {
        ClauseView {
            kind: match self.clause.kind {
                OperationClauseKind::Requires => StateClauseKind::Precondition,
                OperationClauseKind::Ensures => StateClauseKind::Postcondition,
            },
            complete_value: false,
            context: self.context,
            operation: Some(&self.operation),
            context_population: None,
            frame_population: None,
            body: &self.clause.expression,
        }
    }
}

impl CheckRefusal {
    /// The operation a refusal in a kept `requires`/`ensures` clause is
    /// about, by its declaration node identity; `None` for a refusal located
    /// anywhere else.
    #[must_use]
    pub fn operation_node(&self) -> Option<&str> {
        match &self.location.origin {
            Origin::ModelOperationClause { operation, .. } => Some(operation),
            _ => None,
        }
    }

    /// The member a refusal in a kept `requires`/`ensures` clause is
    /// located at, spelled as TC-235 does: `Operation.pre[k]` for the
    /// operation's k-th `requires` and `Operation.post[k]` for its k-th
    /// `ensures`; `None` for a refusal located anywhere else.
    #[must_use]
    pub fn operation_member(&self) -> Option<MemberPath> {
        match &self.location.origin {
            Origin::ModelOperationClause { kind, position, .. } => {
                let list = match kind {
                    OperationClauseKind::Requires => "pre",
                    OperationClauseKind::Ensures => "post",
                };
                Some(
                    MemberPath::new(MemberRoot::Operation)
                        .name(list)
                        .index(*position),
                )
            }
            _ => None,
        }
    }
}
