// SPDX-License-Identifier: AGPL-3.0-or-later
//! S2's reading of a source `object` declaration (FR-569): the type's
//! `extends` and `identity` lists, its fields, populations and operations in
//! source order, each part spelled and spanned. It resolves no name; the
//! assembler and S3 check the parts.

use qsl_cst::{CstNode, CstToken, LosslessCst, Production, TokenKind};

use super::dispatch::{Construct, FormsRefusal};
use super::syntax::{
    DeclarationForm, DeclaredName, FrameEntryForm, ObjectFieldForm, ObjectForm, ObjectMemberForm,
    ObjectOperationForm, ObjectPopulationForm, ParameterForm, RedefinesForm,
};
use super::value::{
    declared_name, items, name_form, nodes_of, only, production_node, text, tokens_of, type_form, unexpected,
    Item,
};

/// The words `modifies` and `deletes` take as an entry's receiver.
const RECEIVER: &[u8] = b"self";

/// `[abstract] object T [extends A, B] [identity (f, g)] { member* }`.
pub(crate) fn object_declaration(
    construct: Construct<'_>,
) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let object_items = items(cst, node);
    let mut tokens = object_items.iter().filter_map(|item| match item {
        Item::Token(token) => Some(*token),
        Item::Node(_) => None,
    });
    let mut next = tokens.next();
    let abstract_span = match next {
        Some(token) if token.spelling() == b"abstract" => {
            next = tokens.next();
            Some(token.span())
        }
        Some(_) | None => None,
    };
    // `object` then the type's name.
    if !next.is_some_and(|token| token.spelling() == b"object") {
        return Err(unexpected(node));
    }
    let name_token = tokens.next().ok_or_else(|| unexpected(node))?;
    let name = token_name(name_token, node)?;
    let extends = nodes_of(&object_items, Production::TypeName)
        .into_iter()
        .map(|supertype| name_form(cst, supertype))
        .collect::<Result<Vec<_>, _>>()?;
    // The `identity` list's names are the identifier tokens between its
    // parentheses.
    let mut identity = Vec::new();
    let mut listing = false;
    for token in tokens {
        match token.spelling() {
            b"identity" => listing = true,
            b"," | b"(" => {}
            b")" | b"{" => break,
            _ if listing => identity.push(token_name(token, node)?),
            _ => {}
        }
    }
    let mut members = Vec::new();
    for item in &object_items {
        let Item::Node(member) = item else { continue };
        members.push(match member.production() {
            Production::Field => ObjectMemberForm::Field(field(cst, member)?),
            Production::ObjectPopulation => ObjectMemberForm::Population(population(cst, member)?),
            Production::ObjectOperation => ObjectMemberForm::Operation(operation(cst, member)?),
            Production::TypeName => continue,
            _ => return Err(unexpected(member)),
        });
    }
    Ok(DeclarationForm::Object(Box::new(ObjectForm {
        name,
        abstract_span,
        extends,
        identity,
        members,
        span: construct.node.span(),
    })))
}

fn token_name(token: &CstToken, node: &CstNode) -> Result<DeclaredName, FormsRefusal> {
    Ok(DeclaredName {
        name: text(token, node)?,
        span: token.span(),
    })
}

/// `f: T;` or `f: T?;`.
fn field(cst: &LosslessCst, node: &CstNode) -> Result<ObjectFieldForm, FormsRefusal> {
    let field_items = items(cst, node);
    Ok(ObjectFieldForm {
        name: declared_name(&field_items, node)?,
        type_form: type_form(cst, only(&field_items, Production::TypeReference, node)?)?,
        optional: field_items
            .iter()
            .any(|item| matches!(item, Item::Token(token) if token.spelling() == b"?")),
        span: node.span(),
    })
}

/// `[symmetric] population p;`.
fn population(cst: &LosslessCst, node: &CstNode) -> Result<ObjectPopulationForm, FormsRefusal> {
    let population_items = items(cst, node);
    // `population` is an identifier-shaped word, so the name is the last
    // identifier token.
    let name = tokens_of(&population_items, TokenKind::Identifier)
        .last()
        .copied()
        .ok_or_else(|| unexpected(node))?;
    Ok(ObjectPopulationForm {
        name: token_name(name, node)?,
        symmetric: population_items
            .iter()
            .any(|item| matches!(item, Item::Token(token) if token.spelling() == b"symmetric")),
        span: node.span(),
    })
}

/// `operation op(params): R [redefines T::op] frame* ;`. The operation's
/// name is a member name, so it may reuse a word the expression grammar
/// reserves (`size`, `count`).
fn operation(cst: &LosslessCst, node: &CstNode) -> Result<ObjectOperationForm, FormsRefusal> {
    let operation_items = items(cst, node);
    let tokens: Vec<&CstToken> = operation_items
        .iter()
        .filter_map(|item| match item {
            Item::Token(token) => Some(*token),
            Item::Node(_) => None,
        })
        .collect();
    // `operation` then the name.
    let name = token_name(tokens.get(1).ok_or_else(|| unexpected(node))?, node)?;
    // `redefines T :: op`: the three tokens after the keyword.
    let redefines = match tokens
        .iter()
        .position(|token| token.spelling() == b"redefines")
    {
        Some(at) => match tokens.get(at + 1..at + 4) {
            Some([declaring, _, member]) => Some(RedefinesForm {
                declaring: token_name(declaring, node)?,
                member: token_name(member, node)?,
            }),
            _ => return Err(unexpected(node)),
        },
        None => None,
    };
    let mut parameters = Vec::new();
    for parameter in nodes_of(&operation_items, Production::Parameter) {
        let parameter_items = self::items(cst, parameter);
        let declared = only(&parameter_items, Production::ParameterType, parameter)?;
        let declared_items = self::items(cst, declared);
        let reference = only(&declared_items, Production::TypeReference, declared)?;
        parameters.push(ParameterForm {
            name: declared_name(&parameter_items, parameter)?,
            value_type: type_form(cst, reference)?,
        });
    }
    let result = nodes_of(&operation_items, Production::TypeReference)
        .first()
        .map(|reference| type_form(cst, reference))
        .transpose()?;
    let mut frame = Vec::new();
    for clause in nodes_of(&operation_items, Production::FrameClause) {
        frame.extend(frame_entries(cst, clause)?);
    }
    Ok(ObjectOperationForm {
        name,
        parameters,
        result,
        redefines,
        frame,
        span: node.span(),
    })
}

/// The entries of one `modifies`, `creates` or `deletes` clause.
fn frame_entries(cst: &LosslessCst, clause: &CstNode) -> Result<Vec<FrameEntryForm>, FormsRefusal> {
    let clause_items = items(cst, clause);
    let keyword = clause_items
        .iter()
        .find_map(|item| match item {
            Item::Token(token) => Some(*token),
            Item::Node(_) => None,
        })
        .ok_or_else(|| unexpected(clause))?;
    match keyword.spelling() {
        b"modifies" => nodes_of(&clause_items, Production::FieldTarget)
            .into_iter()
            .map(|target| modifies_entry(cst, target))
            .collect(),
        b"creates" => clause_items
            .iter()
            .skip(1)
            .filter_map(|item| match item {
                Item::Token(token) if token.spelling() != b"," => Some(*token),
                Item::Token(_) | Item::Node(_) => None,
            })
            .map(|token| Ok(FrameEntryForm::Creates(token_name(token, clause)?)))
            .collect(),
        b"deletes" => nodes_of(&clause_items, Production::DeleteTarget)
            .into_iter()
            .map(|target| deletes_entry(cst, target))
            .collect(),
        _ => Err(unexpected(clause)),
    }
}

/// `self.f`, `f` or `T.f`.
fn modifies_entry(cst: &LosslessCst, target: &CstNode) -> Result<FrameEntryForm, FormsRefusal> {
    let tokens = target_tokens(cst, target);
    match tokens.as_slice() {
        [receiver, field] if receiver.spelling() == RECEIVER => {
            Ok(FrameEntryForm::ModifiesReceiver {
                field: token_name(field, target)?,
            })
        }
        [field] => Ok(FrameEntryForm::ModifiesField {
            field: token_name(field, target)?,
        }),
        [object, field] => Ok(FrameEntryForm::ModifiesTypeField {
            object: token_name(object, target)?,
            field: token_name(field, target)?,
        }),
        _ => Err(unexpected(target)),
    }
}

/// `self` or a parameter or type name.
fn deletes_entry(cst: &LosslessCst, target: &CstNode) -> Result<FrameEntryForm, FormsRefusal> {
    match target_tokens(cst, target).as_slice() {
        [receiver] if receiver.spelling() == RECEIVER => {
            Ok(FrameEntryForm::DeletesReceiver(receiver.span()))
        }
        [name] => Ok(FrameEntryForm::Deletes(token_name(name, target)?)),
        _ => Err(unexpected(target)),
    }
}

/// A frame target's name tokens, without its `.` separators.
fn target_tokens<'c>(cst: &'c LosslessCst, target: &'c CstNode) -> Vec<&'c CstToken> {
    items(cst, target)
        .into_iter()
        .filter_map(|item| match item {
            Item::Token(token) if token.spelling() != b"." => Some(token),
            Item::Token(_) | Item::Node(_) => None,
        })
        .collect()
}
