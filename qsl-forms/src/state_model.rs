// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `StateModel` family's S2 population-bound reading (FR-380, ADR-012
//! §3, §4.3): `bound L on M::p [N]`.
//!
//! This module reads the CST of one `PopulationBound` declaration and builds
//! its [`PopulationBoundForm`]. It resolves nothing: the alias, the
//! population name and the digits of `N` stay spelled and spanned, for the
//! assembler to resolve.

use qsl_cst::{Production, TokenKind};
use qsl_foundation::Span;

use super::dispatch::{Construct, FormsRefusal};
use super::syntax::{DeclarationForm, DeclaredName, PopulationBoundForm};
use super::value::{items, only, production_node, text, tokens_of, unexpected, Item};

/// `bound L on M::p [N]` (FR-380 "The declaration"): the label is the
/// declaration's own identifier token, the alias and the population are the
/// two segments of its `ModelName`, and `N` is its integer token.
pub(crate) fn population_bound(construct: Construct<'_>) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let bound_items = items(cst, node);
    let spelled = |token: &qsl_cst::CstToken| -> Result<DeclaredName, FormsRefusal> {
        Ok(DeclaredName {
            name: text(token, node)?,
            span: token.span(),
        })
    };
    let labels = tokens_of(&bound_items, TokenKind::Identifier);
    let [label] = labels.as_slice() else {
        return Err(unexpected(node));
    };
    let maxima = tokens_of(&bound_items, TokenKind::Integer);
    let [maximum] = maxima.as_slice() else {
        return Err(unexpected(node));
    };
    let target = only(&bound_items, Production::ModelName, node)?;
    let target_items = items(cst, target);
    // The population segment is a member name, which may reuse a word.
    let segments: Vec<&qsl_cst::CstToken> = target_items
        .iter()
        .filter_map(|item| match item {
            Item::Token(token) if token.spelling() != b"::" => Some(*token),
            Item::Token(_) | Item::Node(_) => None,
        })
        .collect();
    let [model, population] = segments.as_slice() else {
        return Err(unexpected(target));
    };
    let delimiter = |spelling: &[u8]| {
        bound_items.iter().find_map(|item| match item {
            Item::Token(token) if token.spelling() == spelling => Some(token.span()),
            Item::Token(_) | Item::Node(_) => None,
        })
    };
    let (Some(open), Some(close)) = (delimiter(b"["), delimiter(b"]")) else {
        return Err(unexpected(node));
    };
    Ok(DeclarationForm::PopulationBound(PopulationBoundForm {
        label: spelled(label)?,
        model: spelled(model)?,
        population: spelled(population)?,
        maximum: spelled(maximum)?,
        bracketed: Span {
            start: open.start,
            end: close.end,
        },
        span: construct.node.span(),
    }))
}
