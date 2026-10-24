// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `StateGraph` family's S2 production (FR-165, ADR-022 SG-1 to SG-3):
//! a `possible`, `always possible` or `unique path` claim.
//!
//! This module reads the CST of one `StateGraphClaim` declaration and builds
//! its [`StateGraphClaimForm`]. It selects no meaning, the same contract
//! `value`'s productions keep (FR-091): the `using` alias and the `over` type
//! stay spelled and spanned, a fairness constraint is kept as written, and a
//! predicate keeps every temporal operator written around its operands.

use qsl_cst::{CstNode, LosslessCst, Production, TokenKind};

use super::dispatch::{Construct, FormsRefusal};
use super::syntax::{
    DeclarationForm, ExpressionForm, StateGraphClaimForm, StateGraphClaimKind,
    StateGraphPredicateForm, TemporalOperatorUse, UsingAlias,
};
use super::temporal_clause::{expression_form, fairness_form, operator_of, parameter_form};
use super::value::{
    declared_name, items, nodes_of, only, production_node, text, tokens_of, unexpected, Item,
};

/// `possible Name using p over (x: T) { fairness* P }`, `always possible
/// Name using p over (x: T) [from (Q)] { fairness* P }` or `unique path Name
/// using p over (x: T) from (X) to (Y)` (FR-165 "Forms").
pub(crate) fn state_graph_claim(construct: Construct<'_>) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let claim_items = items(cst, node);
    let head = match claim_items.first() {
        Some(Item::Token(token)) => token.spelling(),
        Some(Item::Node(_)) | None => return Err(unexpected(node)),
    };
    // The claim's own name and its `using` alias are its last two direct
    // identifier tokens; the head words (`possible`, `unique path`) come
    // before them.
    let identifiers = tokens_of(&claim_items, TokenKind::Identifier);
    let [.., name, alias] = identifiers[..] else {
        return Err(unexpected(node));
    };
    let (fairness, kind) = match head {
        b"possible" => {
            let body = body_of(cst, &claim_items, node)?;
            (body.0, StateGraphClaimKind::Possible { target: body.1 })
        }
        b"always" => {
            let from = match nodes_of(&claim_items, Production::StateGraphPredicate)[..] {
                [] => None,
                [from] => Some(predicate_form(cst, from)?),
                _ => return Err(unexpected(node)),
            };
            let body = body_of(cst, &claim_items, node)?;
            (
                body.0,
                StateGraphClaimKind::AlwaysPossible {
                    from,
                    target: body.1,
                },
            )
        }
        b"unique" => {
            let [from, to] = nodes_of(&claim_items, Production::StateGraphPredicate)[..] else {
                return Err(unexpected(node));
            };
            (
                Vec::new(),
                StateGraphClaimKind::UniquePath {
                    from: predicate_form(cst, from)?,
                    to: predicate_form(cst, to)?,
                },
            )
        }
        _ => return Err(unexpected(node)),
    };
    Ok(DeclarationForm::StateGraphClaim(Box::new(
        StateGraphClaimForm {
            name: declared_name_of(name, node)?,
            profile: UsingAlias {
                alias: text(alias, node)?,
                span: alias.span(),
            },
            over: match nodes_of(&claim_items, Production::Parameter)[..] {
                [] => None,
                [parameter] => Some(parameter_form(cst, parameter)?),
                _ => return Err(unexpected(node)),
            },
            fairness,
            kind,
            span: node.span(),
        },
    )))
}

fn declared_name_of(
    token: &qsl_cst::CstToken,
    node: &CstNode,
) -> Result<super::syntax::DeclaredName, FormsRefusal> {
    declared_name(&[Item::Token(token)], node)
}

/// The braced body's fairness constraints and predicate.
fn body_of(
    cst: &LosslessCst,
    claim_items: &[Item<'_>],
    node: &CstNode,
) -> Result<
    (
        Vec<super::syntax::FairnessConstraintForm>,
        StateGraphPredicateForm,
    ),
    FormsRefusal,
> {
    let body = only(claim_items, Production::StateGraphBody, node)?;
    let body_items = items(cst, body);
    let fairness = nodes_of(&body_items, Production::Fairness)
        .into_iter()
        .map(|constraint| fairness_form(cst, constraint))
        .collect::<Result<_, _>>()?;
    let predicate = only(&body_items, Production::StateGraphPredicate, body)?;
    Ok((fairness, predicate_form(cst, predicate)?))
}

/// One `StateGraphPredicate` node: its expression operands and the temporal
/// operators written in it, both in source order. A predicate that derives
/// as an expression is that one operand with no operator. Otherwise the
/// connectives, `let` and `if` around the operands are dropped: S3 refuses
/// the predicate for its first temporal operator and never reads its
/// operands as one expression.
fn predicate_form(
    cst: &LosslessCst,
    predicate: &CstNode,
) -> Result<StateGraphPredicateForm, FormsRefusal> {
    let mut operands = Vec::new();
    let mut scopes = Vec::new();
    let mut temporal_operators = Vec::new();
    // Source order: the next node to read is the last one pushed, so each
    // node's children go back in reverse.
    let mut pending: Vec<(&CstNode, Vec<(String, ExpressionForm)>)> = vec![(predicate, Vec::new())];
    while let Some((node, scope)) = pending.pop() {
        let mut nested = Vec::new();
        let node_items = items(cst, node);
        // `let name = value in body`: the body sees the name.
        let binder = match node_items[..] {
            [Item::Token(first), Item::Token(name), ..] if first.spelling() == b"let" => {
                Some(text(name, node)?)
            }
            _ => None,
        };
        let mut value = None;
        for item in node_items {
            match item {
                Item::Token(token) => {
                    if let Some(operator) = operator_of(token.spelling()) {
                        temporal_operators.push(TemporalOperatorUse {
                            operator,
                            span: token.span(),
                        });
                    }
                }
                Item::Node(child) => match child.production() {
                    Production::Interval => {}
                    Production::Expression | Production::Comparison => {
                        let form = expression_form(cst, child)?;
                        if binder.is_some() {
                            value = Some(form.clone());
                        }
                        operands.push(form);
                        scopes.push(scope.clone());
                    }
                    Production::StateGraphPredicate
                    | Production::StateGraphImplication
                    | Production::StateGraphDisjunction
                    | Production::StateGraphConjunction
                    | Production::StateGraphRelation
                    | Production::StateGraphUnary => nested.push(child),
                    _ => return Err(unexpected(child)),
                },
            }
        }
        let inner = match (binder, value) {
            (Some(name), Some(value)) => {
                let mut inner = scope;
                inner.push((name, value));
                inner
            }
            _ => scope,
        };
        pending.extend(nested.into_iter().rev().map(|child| (child, inner.clone())));
    }
    // The walk reads a node's own tokens before its children, so a binary
    // operator precedes the operator inside its left operand.
    temporal_operators.sort_by_key(|operator| operator.span.start);
    Ok(StateGraphPredicateForm {
        operands,
        temporal_operators,
        scopes,
    })
}
