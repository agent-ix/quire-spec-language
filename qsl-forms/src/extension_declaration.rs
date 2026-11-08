// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `Extension` variant of the parsed form (FR-354; ADR-012 §5.1 row S2):
//! a selected extension's declaration form
//! `keyword Name { clause: expr, ... }` (QSpec FR-467 "Production").
//!
//! This module reads the CST of one `ExtensionDeclaration` and builds its
//! [`ExtensionForm`]. S1 already matched the form against its production, so
//! the shape here is fixed for every extension: the leading keyword, the
//! declared name, then `name : expression` clauses. It selects no meaning;
//! the keyword stays spelled and S3 reads its definition.

use qsl_cst::Production;

use super::dispatch::{Construct, FormsRefusal};
use super::syntax::{
    DeclarationForm, DeclaredName, ExpressionForm, ExtensionClauseForm, ExtensionForm,
};
use super::value::{expression, items, production_node, text, unexpected, Item};

/// `keyword Name { clause: expr, ... }`, the one production shape every
/// extension declaration form has.
pub(crate) fn extension_declaration(
    construct: Construct<'_>,
) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let declaration = production_node(construct)?;
    if declaration.production() != Production::ExtensionDeclaration {
        return Err(unexpected(declaration));
    }
    let mut parts = items(cst, declaration).into_iter();
    let keyword = identifier(parts.next(), declaration)?;
    let name = identifier(parts.next(), declaration)?;
    let mut clauses = Vec::new();
    let mut pending: Option<DeclaredName> = None;
    for part in parts {
        match part {
            Item::Token(token) if token.spelling() == b":" => {
                if pending.is_none() {
                    return Err(unexpected(declaration));
                }
            }
            // The braces and the clause separators.
            Item::Token(token) if matches!(token.spelling(), b"{" | b"}" | b",") => {}
            Item::Token(token) => {
                if pending.is_some() {
                    return Err(unexpected(declaration));
                }
                pending = Some(DeclaredName {
                    name: text(token, declaration)?,
                    span: token.span(),
                });
            }
            Item::Node(value) if value.production() == Production::Expression => {
                let name = pending.take().ok_or_else(|| unexpected(declaration))?;
                let (tree, spans) = expression(cst, value)?;
                clauses.push(ExtensionClauseForm {
                    name,
                    value: ExpressionForm {
                        expression: tree,
                        spans,
                    },
                });
            }
            Item::Node(_) => return Err(unexpected(declaration)),
        }
    }
    if pending.is_some() {
        return Err(unexpected(declaration));
    }
    Ok(DeclarationForm::Extension(Box::new(ExtensionForm {
        keyword,
        name,
        clauses,
        span: declaration.span(),
    })))
}

/// The identifier `item` is, as a declared name.
fn identifier(
    item: Option<Item<'_>>,
    node: &qsl_cst::CstNode,
) -> Result<DeclaredName, FormsRefusal> {
    match item {
        Some(Item::Token(token)) => Ok(DeclaredName {
            name: text(token, node)?,
            span: token.span(),
        }),
        Some(Item::Node(_)) | None => Err(unexpected(node)),
    }
}
