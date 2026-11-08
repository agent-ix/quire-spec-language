// SPDX-License-Identifier: AGPL-3.0-or-later
//! The implementation guarantee's S2 form (QSL FR-529, QSpec FR-490): a
//! `guarantee G of P using A { initial to "f"; members }` declaration.
//!
//! This module reads the CST of one `GuaranteeDeclaration` and builds its
//! [`GuaranteeDeclarationForm`]. It selects no meaning, the same contract the
//! abstraction relation's form keeps: every name stays spelled and spanned,
//! and each Rust spelling is the source bytes between its quotes, unchecked.
//! QSpec FR-490 owns the static checks; S3 applies them.

use qsl_cst::{CstNode, LosslessCst, Production, TokenKind};
use qsl_foundation::Span;

use super::abstraction_declaration::RustSpelling;
use super::dispatch::{Construct, FormsRefusal};
use super::syntax::{DeclarationForm, DeclaredName, FairnessConstraintForm, NameForm};
use super::temporal_clause::fairness_form;
use super::value::{
    items, name_form, nodes_of, only, production_node, text, tokens_of, unexpected, Item,
};

/// A `guarantee G of P using A { ... }` declaration (QSpec FR-490).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuaranteeDeclarationForm {
    /// The declared guarantee name `G`.
    pub name: DeclaredName,
    /// The temporal clause name `P`, as written.
    pub clause: DeclaredName,
    /// The abstraction relation name `A`, as written.
    pub relation: DeclaredName,
    /// The `initial to "f";` member.
    pub initial: GuaranteeInitialForm,
    /// The `progress` and `environment` members in source order.
    pub members: Vec<GuaranteeMemberForm>,
    /// The span of the whole declaration, closing brace included.
    pub span: Span,
}

/// `initial to "f";`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuaranteeInitialForm {
    /// The Rust function whose result is an implementation initial state.
    pub function: RustSpelling,
    /// The span of the whole member, `;` included.
    pub span: Span,
}

/// One `progress` or `environment` member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GuaranteeMemberForm {
    /// `progress M::T::o decreases "m";` or `progress M::T::o bound N;`.
    Progress(GuaranteeProgressForm),
    /// `environment E by "s" ...`.
    Environment(GuaranteeEnvironmentForm),
}

/// The measure a `progress` member names.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GuaranteeMeasureForm {
    /// `decreases "m"`: the Rust path of a well-founded measure.
    Decreases(RustSpelling),
    /// `bound N`: the integer as written, not yet range-checked.
    Bound {
        /// The decimal digits as written.
        text: String,
        /// The span of the integer.
        span: Span,
    },
}

/// `progress M::T::o (decreases "m" | bound N);`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuaranteeProgressForm {
    /// The operation's name as written, `M::T::o`.
    pub operation: NameForm,
    /// The measure.
    pub measure: GuaranteeMeasureForm,
    /// The span of the whole member, `;` included.
    pub span: Span,
}

/// What an `environment` member supplies.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GuaranteeSuppliesForm {
    /// `fair [kind] [granularity] M::T::o;`.
    Fairness(FairnessConstraintForm),
    /// `assumes N;`: the predicate name as written.
    Assumes(NameForm),
}

/// `environment E by "s" (fair ... ; | assumes N ;)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuaranteeEnvironmentForm {
    /// The premise name `E`.
    pub name: DeclaredName,
    /// The supplier text `s`, the bytes between the quotes as written.
    pub supplier: RustSpelling,
    /// What the premise supplies.
    pub supplies: GuaranteeSuppliesForm,
    /// The span of the whole member, `;` included.
    pub span: Span,
}

/// `guarantee G of P using A { initial to "f"; members }` (QSpec FR-490).
pub(crate) fn declaration(construct: Construct<'_>) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let declaration_items = items(cst, node);
    // `guarantee G of P using A`: the contextual leading word, then the
    // declaration's three direct identifier tokens, in that order.
    let [_, name, clause, relation] = tokens_of(&declaration_items, TokenKind::Identifier)[..]
    else {
        return Err(unexpected(node));
    };
    let initial = only(&declaration_items, Production::GuaranteeInitial, node)?;
    let mut members = Vec::new();
    for item in declaration_items.iter().copied() {
        let Item::Node(child) = item else {
            continue;
        };
        match child.production() {
            Production::GuaranteeInitial => {}
            Production::GuaranteeProgress => {
                members.push(GuaranteeMemberForm::Progress(progress(cst, child)?));
            }
            Production::GuaranteeEnvironment => {
                members.push(GuaranteeMemberForm::Environment(environment(cst, child)?));
            }
            _ => return Err(unexpected(child)),
        }
    }
    let named = |token: &qsl_cst::CstToken| -> Result<DeclaredName, FormsRefusal> {
        Ok(DeclaredName {
            name: text(token, node)?,
            span: token.span(),
        })
    };
    Ok(DeclarationForm::Guarantee(Box::new(
        GuaranteeDeclarationForm {
            name: named(name)?,
            clause: named(clause)?,
            relation: named(relation)?,
            initial: initial_member(initial, cst)?,
            members,
            span: node.span(),
        },
    )))
}

fn initial_member(node: &CstNode, cst: &LosslessCst) -> Result<GuaranteeInitialForm, FormsRefusal> {
    let member_items = items(cst, node);
    let [path] = tokens_of(&member_items, TokenKind::Text)[..] else {
        return Err(unexpected(node));
    };
    Ok(GuaranteeInitialForm {
        function: rust_spelling(path, node)?,
        span: node.span(),
    })
}

fn progress(cst: &LosslessCst, node: &CstNode) -> Result<GuaranteeProgressForm, FormsRefusal> {
    let member_items = items(cst, node);
    let measure = match (
        tokens_of(&member_items, TokenKind::Text)[..].first(),
        tokens_of(&member_items, TokenKind::Integer)[..].first(),
    ) {
        (Some(path), None) => GuaranteeMeasureForm::Decreases(rust_spelling(path, node)?),
        (None, Some(count)) => GuaranteeMeasureForm::Bound {
            text: text(count, node)?,
            span: count.span(),
        },
        _ => return Err(unexpected(node)),
    };
    Ok(GuaranteeProgressForm {
        operation: name_form(cst, only(&member_items, Production::OperationName, node)?)?,
        measure,
        span: node.span(),
    })
}

pub(crate) fn environment(
    cst: &LosslessCst,
    node: &CstNode,
) -> Result<GuaranteeEnvironmentForm, FormsRefusal> {
    let member_items = items(cst, node);
    // The contextual leading word, the premise name, and for `assumes` that
    // contextual word too.
    let Some(name) = tokens_of(&member_items, TokenKind::Identifier)
        .get(1)
        .copied()
    else {
        return Err(unexpected(node));
    };
    let [supplier] = tokens_of(&member_items, TokenKind::Text)[..] else {
        return Err(unexpected(node));
    };
    let supplies = match nodes_of(&member_items, Production::Fairness)[..] {
        [constraint] => GuaranteeSuppliesForm::Fairness(fairness_form(cst, constraint)?),
        [] => GuaranteeSuppliesForm::Assumes(name_form(
            cst,
            only(&member_items, Production::QualifiedName, node)?,
        )?),
        _ => return Err(unexpected(node)),
    };
    Ok(GuaranteeEnvironmentForm {
        name: DeclaredName {
            name: text(name, node)?,
            span: name.span(),
        },
        supplier: rust_spelling(supplier, node)?,
        supplies,
        span: node.span(),
    })
}

/// A quoted string token as its content: the bytes between its quotes.
fn rust_spelling(token: &qsl_cst::CstToken, node: &CstNode) -> Result<RustSpelling, FormsRefusal> {
    let spelled = text(token, node)?;
    let content = spelled
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .ok_or_else(|| unexpected(node))?;
    Ok(RustSpelling {
        text: content.to_owned(),
        span: token.span(),
    })
}
