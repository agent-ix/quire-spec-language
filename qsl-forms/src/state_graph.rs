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
/// operators written around them, in source order. A predicate S1 read as a
/// `StateGraphFormula` (a temporal operator inside a connective, a negation
/// or parentheses) keeps its comparison operands and its temporal operators;
/// the connectives are dropped, since S3 refuses the predicate for the
/// operator and never reads its operands.
fn predicate_form(
    cst: &LosslessCst,
    predicate: &CstNode,
) -> Result<StateGraphPredicateForm, FormsRefusal> {
    let mut operands = Vec::new();
    let mut temporal_operators = Vec::new();
    for item in items(cst, predicate) {
        match item {
            Item::Node(operand) if operand.production() == Production::Expression => {
                operands.push(expression_form(cst, operand)?);
            }
            Item::Node(interval) if interval.production() == Production::Interval => {}
            Item::Node(formula) if formula.production() == Production::StateGraphFormula => {
                formula_parts(cst, formula, &mut operands, &mut temporal_operators)?;
            }
            Item::Node(_) => return Err(unexpected(predicate)),
            Item::Token(token) => temporal_operators.push(TemporalOperatorUse {
                operator: operator_of(token.spelling()).ok_or_else(|| unexpected(predicate))?,
                span: token.span(),
            }),
        }
    }
    Ok(StateGraphPredicateForm {
        operands,
        temporal_operators,
    })
}

/// The comparison operands and temporal operators of a `StateGraphFormula`,
/// in source order, walked with an explicit stack.
fn formula_parts(
    cst: &LosslessCst,
    formula: &CstNode,
    operands: &mut Vec<ExpressionForm>,
    temporal_operators: &mut Vec<TemporalOperatorUse>,
) -> Result<(), FormsRefusal> {
    let mut pending = vec![formula];
    while let Some(node) = pending.pop() {
        let mut children = Vec::new();
        for item in items(cst, node) {
            match item {
                Item::Token(token) => {
                    if let Some(operator) = operator_of(token.spelling()) {
                        children.push(Part::Operator(TemporalOperatorUse {
                            operator,
                            span: token.span(),
                        }));
                    }
                }
                Item::Node(child) => match child.production() {
                    Production::Interval => {}
                    Production::Comparison => {
                        children.push(Part::Operand(expression_form(cst, child)?));
                    }
                    Production::StateGraphFormula | Production::StateGraphTerm => {
                        children.push(Part::Nested(child));
                    }
                    _ => return Err(unexpected(child)),
                },
            }
        }
        // Source order: this node's parts are emitted before any later
        // sibling's nested parts, so nested nodes go back on the stack in
        // reverse beneath the parts already emitted.
        let mut nested = Vec::new();
        for part in children {
            match part {
                Part::Operand(operand) => operands.push(operand),
                Part::Operator(operator) => temporal_operators.push(operator),
                Part::Nested(child) => nested.push(child),
            }
        }
        pending.extend(nested.into_iter().rev());
    }
    if temporal_operators.is_empty() {
        return Err(unexpected(formula));
    }
    Ok(())
}

/// One part of a formula node.
enum Part<'c> {
    Operand(ExpressionForm),
    Operator(TemporalOperatorUse),
    Nested(&'c CstNode),
}
