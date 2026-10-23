// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-447: the S2 build of a `hybrid` declaration (QSpec FR-193 "Source form
//! and the clock line", shared grammar `hybrid-decl`).
//!
//! The form selects no meaning, the same contract `value`'s productions keep
//! (FR-091): the `using` alias, every mode and transition name, every
//! equation's left side and every expression stay spelled and spanned, in
//! declared order. Names, types, the equation set and the classification are
//! S3's checks.

use qsl_cst::{CstNode, LosslessCst, Production, TokenKind};

use super::dispatch::{Construct, FormsRefusal};
use super::syntax::{
    DeclarationForm, DeclaredName, EquationForm, ExpressionForm, HybridForm, HybridInitialForm,
    HybridModeForm, InitialEntryForm, TransitionForm, UsingAlias,
};
use super::value::{
    declared_name, expression, items, name_form, nodes_of, only, production_node, text, tokens_of,
    unexpected,
};

/// `hybrid Name using p { [initial { entry+ }] mode+ }`.
pub(crate) fn hybrid_declaration(
    construct: Construct<'_>,
) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let declaration_items = items(cst, node);
    // `hybrid Name using p {`: the declared name and the `using` alias are
    // the declaration's only two direct identifier tokens.
    let [_, alias] = tokens_of(&declaration_items, TokenKind::Identifier)[..] else {
        return Err(unexpected(node));
    };
    let initial = match nodes_of(&declaration_items, Production::HybridInitial).as_slice() {
        [] => None,
        [clause] => Some(initial_clause(cst, clause)?),
        _ => return Err(unexpected(node)),
    };
    let modes = nodes_of(&declaration_items, Production::HybridMode)
        .into_iter()
        .map(|mode| mode_form(cst, mode))
        .collect::<Result<_, _>>()?;
    Ok(DeclarationForm::Hybrid(Box::new(HybridForm {
        name: declared_name(&declaration_items, node)?,
        profile: UsingAlias {
            alias: text(alias, node)?,
            span: alias.span(),
        },
        initial,
        modes,
        span: node.span(),
    })))
}

/// `initial { mode M when { c }; ... }`.
fn initial_clause(cst: &LosslessCst, node: &CstNode) -> Result<HybridInitialForm, FormsRefusal> {
    let clause_items = items(cst, node);
    let entries = nodes_of(&clause_items, Production::InitialEntry)
        .into_iter()
        .map(|entry| {
            let entry_items = items(cst, entry);
            Ok(InitialEntryForm {
                mode: declared_name(&entry_items, entry)?,
                condition: block_expression(cst, only(&entry_items, Production::Block, entry)?)?,
                span: entry.span(),
            })
        })
        .collect::<Result<_, FormsRefusal>>()?;
    Ok(HybridInitialForm {
        entries,
        span: node.span(),
    })
}

/// `mode M invariant { i } flow { x' = e; ... } (transition to T when { g }
/// reset { r };)*`.
fn mode_form(cst: &LosslessCst, node: &CstNode) -> Result<HybridModeForm, FormsRefusal> {
    let mode_items = items(cst, node);
    let identifiers = tokens_of(&mode_items, TokenKind::Identifier);
    let (name, targets) = identifiers.split_first().ok_or_else(|| unexpected(node))?;
    let blocks = nodes_of(&mode_items, Production::Block);
    let (invariant, rest) = blocks.split_first().ok_or_else(|| unexpected(node))?;
    // Each transition writes one guard block then one reset block, in
    // declared order.
    if rest.len() != targets.len() * 2 {
        return Err(unexpected(node));
    }
    let (pairs, _) = rest.as_chunks::<2>();
    let transitions = targets
        .iter()
        .zip(pairs)
        .map(|(target, [guard, reset])| {
            Ok(TransitionForm {
                target: DeclaredName {
                    name: text(target, node)?,
                    span: target.span(),
                },
                guard: block_expression(cst, guard)?,
                reset: block_expression(cst, reset)?,
            })
        })
        .collect::<Result<_, FormsRefusal>>()?;
    let equations = nodes_of(&mode_items, Production::Equation)
        .into_iter()
        .map(|equation| equation_form(cst, equation))
        .collect::<Result<_, _>>()?;
    Ok(HybridModeForm {
        name: DeclaredName {
            name: text(name, node)?,
            span: name.span(),
        },
        invariant: block_expression(cst, invariant)?,
        equations,
        transitions,
        span: node.span(),
    })
}

/// `x' = e ;`.
fn equation_form(cst: &LosslessCst, node: &CstNode) -> Result<EquationForm, FormsRefusal> {
    let equation_items = items(cst, node);
    Ok(EquationForm {
        variable: name_form(cst, only(&equation_items, Production::QualifiedName, node)?)?,
        rate: expression_node(cst, only(&equation_items, Production::Expression, node)?)?,
        span: node.span(),
    })
}

/// `{ expression }` as an expression form.
fn block_expression(cst: &LosslessCst, block: &CstNode) -> Result<ExpressionForm, FormsRefusal> {
    let block_items = items(cst, block);
    expression_node(cst, only(&block_items, Production::Expression, block)?)
}

fn expression_node(cst: &LosslessCst, node: &CstNode) -> Result<ExpressionForm, FormsRefusal> {
    let (expression, spans) = expression(cst, node)?;
    Ok(ExpressionForm { expression, spans })
}
