// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `TemporalTrace` family's terminal declaration (FR-124, FR-382): a
//! `terminal Name using p on Model (when { P } | any);` declaration.
//!
//! This module reads the CST of one `TerminalDeclaration` and builds its
//! [`TerminalDeclarationForm`]. It selects no meaning, the same contract
//! `value`'s productions keep (FR-091): the `using` alias and the model alias
//! stay spelled and spanned, and the predicate is built as written.

use qsl_cst::{Production, TokenKind};

use super::dispatch::{Construct, FormsRefusal};
use super::syntax::{
    DeclarationForm, DeclaredName, ExpressionForm, TerminalDeclarationForm,
    TerminalDeclarationKind, UsingAlias,
};
use super::value::{
    expression, has_token, items, nodes_of, only, production_node, text, tokens_of, unexpected,
};

/// `terminal Name using p on Model when { P };` or `terminal Name using p on
/// Model any;` (FR-382 "Outputs").
pub(crate) fn terminal_declaration(
    construct: Construct<'_>,
) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let declaration_items = items(cst, node);
    // `terminal Name using p on Model ...`: the declaration's own name, its
    // `using` alias and its model alias are its only three direct identifier
    // tokens, in that order.
    let [name, alias, model] = tokens_of(&declaration_items, TokenKind::Identifier)[..] else {
        return Err(unexpected(node));
    };
    let kind = match nodes_of(&declaration_items, Production::Block)[..] {
        [block] => {
            let block_items = items(cst, block);
            let predicate = only(&block_items, Production::Expression, block)?;
            let (expression, spans) = expression(cst, predicate)?;
            TerminalDeclarationKind::When(ExpressionForm { expression, spans })
        }
        [] if has_token(&declaration_items, b"any") => TerminalDeclarationKind::Any,
        _ => return Err(unexpected(node)),
    };
    Ok(DeclarationForm::Terminal(Box::new(
        TerminalDeclarationForm {
            name: DeclaredName {
                name: text(name, node)?,
                span: name.span(),
            },
            profile: UsingAlias {
                alias: text(alias, node)?,
                span: alias.span(),
            },
            model: DeclaredName {
                name: text(model, node)?,
                span: model.span(),
            },
            kind,
            span: node.span(),
        },
    )))
}
