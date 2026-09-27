// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `ProtocolClause` family's one S2 production (FR-102, ADR-012 §15.2,
//! §15.3 stage table row S2): `invariant`/`pre`/`post` state clauses.
//!
//! This module reads the CST of one `StateClause` declaration and builds
//! its [`StateClauseForm`]. It resolves no name and mints no identity, the
//! same contract `value`'s productions keep (FR-091): the `using` alias,
//! the model context and the operation member stay spelled and spanned,
//! unresolved, for the assembler and S3 (FR-104) to read. It shares the
//! `Value` family's own CST-walking helpers (`items`, `only`, `text`, ...)
//! and its one `Expression` builder, since the `Expression` enum and its
//! builder are defined once, in the `forms` core (ADR-012 §4.3): only the
//! declaration-level production is `ProtocolClause`'s own.

use qsl_cst::{Production, TokenKind};

use super::dispatch::{Construct, FormsFailure};
use super::spans::DeclarationSpans;
use super::syntax::{DeclarationForm, DeclaredName, StateClauseForm, StateClauseKind, UsingAlias};
use super::value::{
    expression, items, name_form, only, production_node, text, tokens_of, unexpected, Item,
};

/// `invariant N using p on M::T at current { e }`, `pre N using p on
/// M::T::op { e }` or `post N using p on M::T::op { e }` (FR-102 "Behavior"):
/// the kind is read from the leading token alone, and the body's nesting
/// depth is bound exactly as a function body's is (`value::expression`
/// applies the same [`super::dispatch::FormsLimits`]).
pub(crate) fn state_clause(construct: Construct<'_>) -> Result<DeclarationForm, FormsFailure> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let clause_items = items(cst, node);
    let kind = match clause_items.first() {
        Some(Item::Token(token)) if token.spelling() == b"invariant" => StateClauseKind::Invariant,
        Some(Item::Token(token)) if token.spelling() == b"pre" => StateClauseKind::Precondition,
        Some(Item::Token(token)) if token.spelling() == b"post" => StateClauseKind::Postcondition,
        Some(Item::Token(_) | Item::Node(_)) | None => return Err(unexpected(node)),
    };
    let identifiers = tokens_of(&clause_items, TokenKind::Identifier);
    let [name, alias] = identifiers.as_slice() else {
        return Err(unexpected(node));
    };
    let (context, operation) = match kind {
        StateClauseKind::Invariant => {
            let type_name = only(&clause_items, Production::TypeName, node)?;
            (name_form(cst, type_name)?, None)
        }
        StateClauseKind::Precondition | StateClauseKind::Postcondition => {
            let operation_name = only(&clause_items, Production::OperationName, node)?;
            let operation_items = items(cst, operation_name);
            let type_name = only(&operation_items, Production::TypeName, operation_name)?;
            let context = name_form(cst, type_name)?;
            let member = tokens_of(&operation_items, TokenKind::Identifier)
                .first()
                .copied()
                .ok_or_else(|| unexpected(operation_name))?;
            let operation = DeclaredName {
                name: text(member, operation_name)?,
                span: member.span(),
            };
            (context, Some(operation))
        }
    };
    let block = only(&clause_items, Production::Block, node)?;
    let block_items = items(cst, block);
    let body_node = only(&block_items, Production::Expression, block)?;
    let (body, body_spans) = expression(cst, body_node, construct.limits)?;
    let form = StateClauseForm {
        kind,
        name: DeclaredName {
            name: text(name, node)?,
            span: name.span(),
        },
        profile: UsingAlias {
            alias: text(alias, node)?,
            span: alias.span(),
        },
        context,
        operation,
        body,
        spans: DeclarationSpans {
            declaration: construct.node.span(),
            body: body_spans,
            measure: None,
        },
    };
    Ok(DeclarationForm::StateClause(Box::new(form)))
}
