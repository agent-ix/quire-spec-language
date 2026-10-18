// SPDX-License-Identifier: AGPL-3.0-or-later
//! The abstraction relation's S2 form (QSL FR-304, QSpec FR-450): an
//! `abstraction Name using p { binding+ }` declaration, with `type`,
//! `population` and `operation` bindings.
//!
//! This module reads the CST of one `AbstractionDeclaration` and builds its
//! [`AbstractionDeclarationForm`]. It selects no meaning, the same contract
//! `value`'s productions keep (FR-091): the model names stay spelled and
//! spanned, and each Rust spelling is the source bytes between its quotes,
//! unchecked. QSpec FR-450 owns the Rust syntax and its refusals; S3 checks
//! them.

use qsl_cst::{CstNode, CstToken, LosslessCst, Production, TokenKind};
use qsl_foundation::Span;

use super::dispatch::{Construct, FormsRefusal};
use super::syntax::{DeclarationForm, DeclaredName, NameForm, UsingAlias};
use super::value::{
    items, name_form, nodes_of, only, production_node, text, tokens_of, unexpected, Item,
};

/// A quoted Rust spelling: the source bytes between the quotes, and the span
/// of the string token. FR-450 "Rust spellings": the content has no escape
/// sequence, so a backslash in it is a defect S3 reports, and it is kept as
/// written.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustSpelling {
    /// The content between the quotes, as written.
    pub text: String,
    /// The span of the quoted string, quotes included.
    pub span: Span,
}

/// An `abstraction Name using p { ... }` declaration (FR-304).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AbstractionDeclarationForm {
    /// The declared name.
    pub name: DeclaredName,
    /// The `using` alias.
    pub profile: UsingAlias,
    /// The bindings in source order; never empty.
    pub bindings: Vec<AbstractionBindingForm>,
    /// The span of the whole declaration, closing brace included.
    pub span: Span,
}

/// One binding of an abstraction declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AbstractionBindingForm {
    /// `type M::T to "path" { f to "x"; } ;`.
    Object(ObjectBindingForm),
    /// `population M::P to "path";`.
    Population(PopulationBindingForm),
    /// `operation M::T::o to "path" receiver "r" (a to "x") ;`.
    Frame(FrameBindingForm),
}

/// `type M::T to "path" [{ field-binding+ }] ;`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectBindingForm {
    /// The object type's name as written, `M::T`.
    pub model_type: NameForm,
    /// The Rust type's path.
    pub rust_type: RustSpelling,
    /// The field bindings in source order; empty when no braces are written.
    pub fields: Vec<FieldBindingForm>,
    /// The span of the whole binding, `;` included.
    pub span: Span,
}

/// `field to "x";` inside an object binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldBindingForm {
    /// The field's name as written.
    pub field: DeclaredName,
    /// The Rust field.
    pub rust_field: RustSpelling,
    /// The span of the whole field binding, `;` included.
    pub span: Span,
}

/// `population M::P to "path";`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PopulationBindingForm {
    /// The population's name as written, `M::P`.
    pub population: NameForm,
    /// The Rust path from the implementation state root to the collection.
    pub collection: RustSpelling,
    /// The span of the whole binding, `;` included.
    pub span: Span,
}

/// `operation M::T::o to "path" receiver "r" (a to "x", ...);`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameBindingForm {
    /// The operation's name as written, `M::T::o`.
    pub operation: NameForm,
    /// The Rust function's path.
    pub function: RustSpelling,
    /// The function input that carries the operation's context object.
    pub receiver: RustSpelling,
    /// The parameter bindings in source order.
    pub parameters: Vec<ParameterBindingForm>,
    /// The span of the whole binding, `;` included.
    pub span: Span,
}

/// `parameter to "x"` inside a frame binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParameterBindingForm {
    /// The operation parameter's name as written.
    pub parameter: DeclaredName,
    /// The Rust parameter.
    pub rust_parameter: RustSpelling,
    /// The span of the parameter binding.
    pub span: Span,
}

/// `abstraction Name using p { binding+ }` (FR-304 "Placement").
pub(crate) fn abstraction_declaration(
    construct: Construct<'_>,
) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let declaration_items = items(cst, node);
    // `abstraction Name using p { ... }`: the declaration's own name and its
    // `using` alias are its only two direct identifier tokens.
    let [name, alias] = tokens_of(&declaration_items, TokenKind::Identifier)[..] else {
        return Err(unexpected(node));
    };
    let mut bindings = Vec::new();
    for item in declaration_items.iter().copied() {
        let Item::Node(child) = item else {
            continue;
        };
        bindings.push(match child.production() {
            Production::ObjectBinding => {
                AbstractionBindingForm::Object(object_binding(cst, child)?)
            }
            Production::PopulationBinding => {
                AbstractionBindingForm::Population(population_binding(cst, child)?)
            }
            Production::FrameBinding => AbstractionBindingForm::Frame(frame_binding(cst, child)?),
            _ => return Err(unexpected(child)),
        });
    }
    if bindings.is_empty() {
        return Err(unexpected(node));
    }
    Ok(DeclarationForm::Abstraction(Box::new(
        AbstractionDeclarationForm {
            name: named(name, node)?,
            profile: UsingAlias {
                alias: text(alias, node)?,
                span: alias.span(),
            },
            bindings,
            span: node.span(),
        },
    )))
}

/// `type M::T to "path" [{ f to "x"; ... }] ;`.
fn object_binding(cst: &LosslessCst, node: &CstNode) -> Result<ObjectBindingForm, FormsRefusal> {
    let binding_items = items(cst, node);
    let [path] = tokens_of(&binding_items, TokenKind::Text)[..] else {
        return Err(unexpected(node));
    };
    let fields = nodes_of(&binding_items, Production::FieldBinding)
        .into_iter()
        .map(|field| field_binding(cst, field))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ObjectBindingForm {
        model_type: name_form(cst, only(&binding_items, Production::TypeName, node)?)?,
        rust_type: rust_spelling(path, node)?,
        fields,
        span: node.span(),
    })
}

/// `f to "x";`.
fn field_binding(cst: &LosslessCst, node: &CstNode) -> Result<FieldBindingForm, FormsRefusal> {
    let binding_items = items(cst, node);
    let [rust] = tokens_of(&binding_items, TokenKind::Text)[..] else {
        return Err(unexpected(node));
    };
    Ok(FieldBindingForm {
        field: named(leading_token(&binding_items, node)?, node)?,
        rust_field: rust_spelling(rust, node)?,
        span: node.span(),
    })
}

/// `population M::P to "path";`.
fn population_binding(
    cst: &LosslessCst,
    node: &CstNode,
) -> Result<PopulationBindingForm, FormsRefusal> {
    let binding_items = items(cst, node);
    let [path] = tokens_of(&binding_items, TokenKind::Text)[..] else {
        return Err(unexpected(node));
    };
    Ok(PopulationBindingForm {
        population: name_form(cst, only(&binding_items, Production::ModelName, node)?)?,
        collection: rust_spelling(path, node)?,
        span: node.span(),
    })
}

/// `operation M::T::o to "path" receiver "r" (a to "x", ...);`.
fn frame_binding(cst: &LosslessCst, node: &CstNode) -> Result<FrameBindingForm, FormsRefusal> {
    let binding_items = items(cst, node);
    // The function path, then the receiver: the binding's only two direct
    // string tokens, in that order.
    let [function, receiver] = tokens_of(&binding_items, TokenKind::Text)[..] else {
        return Err(unexpected(node));
    };
    let parameters = nodes_of(&binding_items, Production::ParameterBinding)
        .into_iter()
        .map(|parameter| parameter_binding(cst, parameter))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(FrameBindingForm {
        operation: name_form(cst, only(&binding_items, Production::OperationName, node)?)?,
        function: rust_spelling(function, node)?,
        receiver: rust_spelling(receiver, node)?,
        parameters,
        span: node.span(),
    })
}

/// `a to "x"`.
fn parameter_binding(
    cst: &LosslessCst,
    node: &CstNode,
) -> Result<ParameterBindingForm, FormsRefusal> {
    let binding_items = items(cst, node);
    let [rust] = tokens_of(&binding_items, TokenKind::Text)[..] else {
        return Err(unexpected(node));
    };
    Ok(ParameterBindingForm {
        parameter: named(leading_token(&binding_items, node)?, node)?,
        rust_parameter: rust_spelling(rust, node)?,
        span: node.span(),
    })
}

/// The binding's leading token: its member name, an identifier or a word the
/// grammar reserves.
fn leading_token<'c>(items: &[Item<'c>], node: &CstNode) -> Result<&'c CstToken, FormsRefusal> {
    match items.first() {
        Some(Item::Token(token)) => Ok(*token),
        Some(Item::Node(_)) | None => Err(unexpected(node)),
    }
}

/// `token` as a declared name.
fn named(token: &CstToken, node: &CstNode) -> Result<DeclaredName, FormsRefusal> {
    Ok(DeclaredName {
        name: text(token, node)?,
        span: token.span(),
    })
}

/// A quoted string token as a Rust spelling: the bytes between its quotes.
fn rust_spelling(token: &CstToken, node: &CstNode) -> Result<RustSpelling, FormsRefusal> {
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
