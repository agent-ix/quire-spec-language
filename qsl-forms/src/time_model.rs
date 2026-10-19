// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-393: the S2 build of a state model's timed members, `time`,
//! `time invariant` and `urgent` (QSpec shared grammar `time-decl`,
//! `time-invariant`, `clock-bound` and `urgent-decl`).
//!
//! These forms select no meaning, the same contract `value`'s productions
//! keep (FR-091): the model alias, the unit and every operation stay spelled
//! and spanned. Uniqueness of the time source, the unit's dimension, clock
//! typing and a constant's sign are S3's checks (FR-230). The forms are built
//! under every facet selection, none included.

use qsl_cst::{CstNode, LosslessCst, Production, TokenKind};

use super::dispatch::{Construct, FormsRefusal};
use super::syntax::{
    ClockBoundForm, DeclarationForm, DeclaredName, ExpressionForm, TimeInvariantForm,
    TimeSourceForm, TimeSourceSyntax, UrgentForm, UrgentTarget,
};
use super::value::{
    expression, items, name_form, nodes_of, only, production_node, quantity_form, text, tokens_of,
    unexpected, Item,
};

/// A declaration headed `time`: the `TimeInvariant` production when `time`
/// is followed by `invariant`, else the `TimeDeclaration` production. The
/// grammar gives the `Declaration` node exactly one of the two, so the
/// selection reads the node S1 built, not the tokens again.
pub(crate) fn time_member(construct: Construct<'_>) -> Result<DeclarationForm, FormsRefusal> {
    let node = production_node(construct)?;
    match node.production() {
        Production::TimeInvariant => time_invariant(construct.cst, node),
        Production::TimeDeclaration => time_declaration(construct.cst, node),
        _ => Err(unexpected(node)),
    }
}

/// `time Model (dense | tick O period p) unit u;`.
fn time_declaration(cst: &LosslessCst, node: &CstNode) -> Result<DeclarationForm, FormsRefusal> {
    let declaration_items = items(cst, node);
    // `time Model dense ...` or `time Model tick ... period ...`: the
    // contextual words are identifier tokens, so the model alias is the
    // second identifier and the source word the third, whatever the alias
    // is spelled.
    let identifiers = tokens_of(&declaration_items, TokenKind::Identifier);
    let (Some(model), Some(word)) = (identifiers.get(1), identifiers.get(2)) else {
        return Err(unexpected(node));
    };
    let names = nodes_of(&declaration_items, Production::QualifiedName);
    let (source, unit) = if word.spelling() == b"tick" {
        let [operation, unit] = names[..] else {
            return Err(unexpected(node));
        };
        let period = only(&declaration_items, Production::QuantityLiteral, node)?;
        (
            TimeSourceSyntax::Tick {
                operation: name_form(cst, operation)?,
                period: quantity_form(cst, period)?,
            },
            unit,
        )
    } else if word.spelling() == b"dense" {
        let [unit] = names[..] else {
            return Err(unexpected(node));
        };
        (TimeSourceSyntax::Dense, unit)
    } else {
        return Err(unexpected(node));
    };
    Ok(DeclarationForm::TimeSource(Box::new(TimeSourceForm {
        model: DeclaredName {
            name: text(model, node)?,
            span: model.span(),
        },
        source,
        unit: name_form(cst, unit)?,
        span: node.span(),
    })))
}

/// `time invariant Name when { P } { bound (and bound)* }`.
fn time_invariant(cst: &LosslessCst, node: &CstNode) -> Result<DeclarationForm, FormsRefusal> {
    let declaration_items = items(cst, node);
    // `time` is a contextual word, so the declared name is the second
    // identifier token.
    let Some(name) = tokens_of(&declaration_items, TokenKind::Identifier)
        .get(1)
        .copied()
    else {
        return Err(unexpected(node));
    };
    let bounds = nodes_of(&declaration_items, Production::ClockBound)
        .into_iter()
        .map(|bound| clock_bound(cst, bound))
        .collect::<Result<_, _>>()?;
    Ok(DeclarationForm::TimeInvariant(Box::new(
        TimeInvariantForm {
            name: DeclaredName {
                name: text(name, node)?,
                span: name.span(),
            },
            guard: block_expression(cst, only(&declaration_items, Production::Block, node)?)?,
            bounds,
            span: node.span(),
        },
    )))
}

/// `clock (< | <=) quantity`.
fn clock_bound(cst: &LosslessCst, node: &CstNode) -> Result<ClockBoundForm, FormsRefusal> {
    let bound_items = items(cst, node);
    let strict = match bound_items.as_slice() {
        [Item::Node(_), Item::Token(operator), Item::Node(_)] => match operator.spelling() {
            b"<" => true,
            b"<=" => false,
            _ => return Err(unexpected(node)),
        },
        _ => return Err(unexpected(node)),
    };
    let (clock, spans) = expression(cst, only(&bound_items, Production::Postfix, node)?)?;
    Ok(ClockBoundForm {
        clock: ExpressionForm {
            expression: clock,
            spans,
        },
        strict,
        bound: quantity_form(cst, only(&bound_items, Production::QuantityLiteral, node)?)?,
        span: node.span(),
    })
}

/// `urgent Name (when { P } | O);`.
pub(crate) fn urgent_declaration(
    construct: Construct<'_>,
) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let declaration_items = items(cst, node);
    // `urgent` is a contextual word, so the declared name is the second
    // identifier token.
    let Some(name) = tokens_of(&declaration_items, TokenKind::Identifier)
        .get(1)
        .copied()
    else {
        return Err(unexpected(node));
    };
    let target = match (
        &nodes_of(&declaration_items, Production::Block)[..],
        &nodes_of(&declaration_items, Production::QualifiedName)[..],
    ) {
        ([block], []) => UrgentTarget::When(block_expression(cst, block)?),
        ([], [operation]) => UrgentTarget::Operation(name_form(cst, operation)?),
        _ => return Err(unexpected(node)),
    };
    Ok(DeclarationForm::Urgent(Box::new(UrgentForm {
        name: DeclaredName {
            name: text(name, node)?,
            span: name.span(),
        },
        target,
        span: node.span(),
    })))
}

/// A `Block` node's one expression.
fn block_expression(cst: &LosslessCst, block: &CstNode) -> Result<ExpressionForm, FormsRefusal> {
    let block_items = items(cst, block);
    let (expression, spans) = expression(cst, only(&block_items, Production::Expression, block)?)?;
    Ok(ExpressionForm { expression, spans })
}
