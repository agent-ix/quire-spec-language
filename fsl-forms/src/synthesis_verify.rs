// SPDX-License-Identifier: AGPL-3.0-or-later
//! S2 dispatch for declarations whose detailed check/run forms are owned by
//! FR-753 and FR-754. Keep their source identity and complete spelling intact.

use crate::dispatch::{Construct, FormsRefusal};
use crate::syntax::{
    DeclarationForm, DeclaredName, ExpressionForm, NameForm, SynthesisDirectionForm, SynthesisForm,
    SynthesisObjectiveForm, UsingAlias, VerificationPlanForm, VerificationStepForm,
};
use crate::value::{
    expression, items, name_form, nodes_of, only, production_node, significant_tokens, text,
    tokens_of, unexpected, Item,
};
use fsl_cst::{CstNode, LosslessCst, Production, TokenKind};
use fsl_foundation::Span;

/// Build the source-bearing S2 form for `synthesis` (FR-754).
pub(crate) fn synthesis(construct: Construct<'_>) -> Result<DeclarationForm, FormsRefusal> {
    let (name, profile, source, span) = identity(construct)?;
    let cst = construct.cst;
    let node = production_node(construct)?;
    let children = items(cst, node);
    let grammar = keyed_name(cst, &children, "grammar", node)?;
    let domain = keyed_name(cst, &children, "domain", node)?;
    let blocks = nodes_of(&children, Production::Block);
    let [block] = blocks.as_slice() else {
        return Err(unexpected(node));
    };
    let block_children = items(cst, block);
    let predicate = only(&block_children, Production::Expression, block)?;
    let (predicate, predicate_spans) = expression(cst, predicate)?;
    let objective = objective(cst, &children, node)?;
    Ok(DeclarationForm::Synthesis(Box::new(SynthesisForm {
        name,
        profile,
        grammar,
        domain,
        objective,
        satisfies: ExpressionForm {
            expression: predicate,
            spans: predicate_spans,
        },
        satisfies_span: block.span(),
        source,
        span,
    })))
}
/// The name after the `keyword` token of a `synthesis` declaration: a
/// qualified name, or the builtin `Integer`, which is a keyword token and
/// not an identifier.
fn keyed_name(
    cst: &LosslessCst,
    children: &[Item<'_>],
    keyword: &str,
    node: &CstNode,
) -> Result<NameForm, FormsRefusal> {
    let mut after_keyword = false;
    for child in children {
        match child {
            Item::Token(token) if after_keyword => {
                return Ok(NameForm::new(text(token, node)?, token.span()));
            }
            Item::Token(token) => after_keyword = token.spelling() == keyword.as_bytes(),
            Item::Node(name) if after_keyword && name.production() == Production::QualifiedName => {
                return name_form(cst, name);
            }
            Item::Node(_) => after_keyword = false,
        }
    }
    Err(unexpected(node))
}

/// Build the source-bearing S2 form for `verify` (FR-753).
pub(crate) fn verification_plan(construct: Construct<'_>) -> Result<DeclarationForm, FormsRefusal> {
    let (name, profile, source, span) = identity(construct)?;
    let node = production_node(construct)?;
    let declaration_items = items(construct.cst, node);
    let steps = nodes_of(&declaration_items, fsl_cst::Production::VerificationStep)
        .into_iter()
        .map(|step| verification_step(construct.cst, step))
        .collect::<Result<_, _>>()?;
    Ok(DeclarationForm::VerificationPlan(Box::new(
        VerificationPlanForm {
            name,
            profile,
            steps,
            source,
            span,
        },
    )))
}

fn verification_step(
    cst: &fsl_cst::LosslessCst,
    node: &fsl_cst::CstNode,
) -> Result<VerificationStepForm, FormsRefusal> {
    let step_items = items(cst, node);
    let identifiers = tokens_of(&step_items, fsl_cst::TokenKind::Identifier);
    let [name, depends @ ..] = identifiers.as_slice() else {
        return Err(unexpected(node));
    };
    let names = nodes_of(&step_items, fsl_cst::Production::QualifiedName);
    let [claim, method, domain] = names.as_slice() else {
        return Err(unexpected(node));
    };
    let tokens = significant_tokens(cst, node);
    let bound = tokens
        .windows(2)
        .find(|pair| pair[0].spelling() == b"bound")
        .map(|pair| {
            let value = std::str::from_utf8(pair[1].spelling())
                .ok()
                .map(str::to_owned)
                .ok_or_else(|| unexpected(node))?;
            Ok((value, pair[1].span()))
        })
        .transpose()?;
    let depends = depends
        .iter()
        .map(|token| {
            Ok(DeclaredName {
                name: text(token, node)?,
                span: token.span(),
            })
        })
        .collect::<Result<_, FormsRefusal>>()?;
    Ok(VerificationStepForm {
        name: DeclaredName {
            name: text(name, node)?,
            span: name.span(),
        },
        claim: name_form(cst, claim)?,
        method: name_form(cst, method)?,
        domain: name_form(cst, domain)?,
        depends,
        bound: bound.as_ref().map(|(value, _)| value.clone()),
        bound_span: bound.map(|(_, span)| span),
        span: node.span(),
    })
}

fn identity(
    construct: Construct<'_>,
) -> Result<(DeclaredName, UsingAlias, String, fsl_foundation::Span), FormsRefusal> {
    let tokens = significant_tokens(construct.cst, construct.node);
    let node = construct.node;
    let name_token = tokens
        .iter()
        .skip(1)
        .find(|token| token.kind() == TokenKind::Identifier)
        .copied()
        .ok_or_else(|| unexpected(node))?;
    let using_index = tokens
        .iter()
        .position(|token| token.spelling() == b"using")
        .ok_or_else(|| unexpected(node))?;
    let profile_token = tokens
        .get(using_index + 1)
        .filter(|token| token.kind() == TokenKind::Identifier)
        .copied()
        .ok_or_else(|| unexpected(node))?;
    let name = DeclaredName {
        name: text(name_token, node)?,
        span: name_token.span(),
    };
    let profile = UsingAlias {
        alias: text(profile_token, node)?,
        span: profile_token.span(),
    };
    let source = construct
        .cst
        .source()
        .text()
        .get(node.span().start..node.span().end)
        .ok_or_else(|| unexpected(node))?
        .to_owned();
    Ok((name, profile, source, node.span()))
}

fn objective(
    cst: &LosslessCst,
    children: &[Item<'_>],
    node: &CstNode,
) -> Result<Option<SynthesisObjectiveForm>, FormsRefusal> {
    let direction = children.iter().find_map(|item| match item {
        Item::Token(token) if token.spelling() == b"minimize" => {
            Some((SynthesisDirectionForm::Minimize, token.span()))
        }
        Item::Token(token) if token.spelling() == b"maximize" => {
            Some((SynthesisDirectionForm::Maximize, token.span()))
        }
        Item::Token(_) | Item::Node(_) => None,
    });
    let mut expressions = nodes_of(children, Production::Expression);
    expressions.extend(nodes_of(children, Production::StepSumObjective));
    match (direction, expressions.as_slice()) {
        (None, []) => Ok(None),
        (Some((direction, direction_span)), [expression_node]) => {
            let (expression, spans) = expression(cst, expression_node)?;
            let text = significant_tokens(cst, expression_node)
                .into_iter()
                .map(|token| self::text(token, expression_node))
                .collect::<Result<Vec<_>, _>>()?
                .join(" ");
            let span = Span {
                start: direction_span.start,
                end: expression_node.span().end,
            };
            Ok(Some(SynthesisObjectiveForm {
                direction,
                expression: ExpressionForm { expression, spans },
                text,
                span,
            }))
        }
        _ => Err(unexpected(node)),
    }
}
