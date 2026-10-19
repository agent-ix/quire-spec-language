// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `TemporalTrace` family's `hyper` and `relation` clauses (FR-171, ADR-023
//! §1): a `hyper` clause over `behaviours` or over a finite trace domain, and
//! a `relation` over named executions.
//!
//! This module reads the CST of one clause and builds its form. It selects no
//! meaning, the same contract `value`'s productions keep (FR-091): the
//! `using` alias, every model alias, every operation name and every trace
//! variable stay spelled and spanned, and the `match` block and the body are
//! built as written. Whether an alias names a model and whether an execution
//! binding names an operation are S3's.

use qsl_cst::{CstNode, LosslessCst, Production, TokenKind};
use qsl_foundation::Span;

use super::dispatch::{Construct, FormsRefusal};
use super::syntax::{
    DeclarationForm, DeclaredName, ExpressionForm, FairnessConstraintForm, NameForm, ParameterForm,
    TemporalFormulaForm, UsingAlias,
};
use super::temporal_clause::{expression_form, fairness_form, formula_form, parameter_form};
use super::value::{
    declared_name, has_token, items, name_form, nodes_of, only, production_node, text, tokens_of,
    unexpected,
};

/// Whether a quantifier ranges universally or existentially.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuantifierKind {
    /// `forall trace`.
    Forall,
    /// `exists trace`.
    Exists,
}

/// One `forall|exists trace v [of M fair ...]` quantifier, as written.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuantifierForm {
    /// Universal or existential.
    pub kind: QuantifierKind,
    /// The trace variable.
    pub variable: DeclaredName,
    /// The model alias after `of`; `None` when none is written.
    pub alias: Option<DeclaredName>,
    /// The fairness constraints written after the alias, in source order.
    pub fairness: Vec<FairnessConstraintForm>,
    /// The span of the whole quantifier.
    pub span: Span,
}

/// `align skip { O, ... }` as written.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AlignSkipForm {
    /// The operation names in source order; empty for `align skip { }`.
    pub operations: Vec<NameForm>,
    /// The span of the whole `align skip { ... }`.
    pub span: Span,
}

/// A `hyper` clause over `behaviours` (FR-171 "Outputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HyperBehavioursForm {
    /// The declared clause name.
    pub name: DeclaredName,
    /// The `using` alias.
    pub profile: UsingAlias,
    /// The object parameters of `behaviours (v: V::Vault, ...)`.
    pub parameters: Vec<ParameterForm>,
    /// The quantifiers in source order.
    pub quantifiers: Vec<QuantifierForm>,
    /// The `match` block's condition; `None` when there is no block.
    pub match_block: Option<ExpressionForm>,
    /// The `align skip` list; `None` when there is none.
    pub align_skip: Option<AlignSkipForm>,
    /// The body, with each indexed atom as written.
    pub body: TemporalFormulaForm,
    /// The span of the whole clause.
    pub span: Span,
}

/// The `t in Q bounded N` domain of a finite `hyper` clause, as written.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceDomainForm {
    /// The trace set's variable.
    pub variable: DeclaredName,
    /// The qualified name of the trace population.
    pub population: NameForm,
    /// The bound `N`, as written.
    pub bound: u64,
    /// The span of the whole domain.
    pub span: Span,
}

/// A `hyper` clause over a declared finite trace domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HyperFiniteForm {
    /// The declared clause name.
    pub name: DeclaredName,
    /// The `using` alias.
    pub profile: UsingAlias,
    /// The finite trace domain.
    pub domain: TraceDomainForm,
    /// The quantifiers in source order.
    pub quantifiers: Vec<QuantifierForm>,
    /// The body.
    pub body: ExpressionForm,
    /// The span of the whole clause.
    pub span: Span,
}

/// One `x: Q` execution binding of a `relation`, as written.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionBindingForm {
    /// The execution variable.
    pub variable: DeclaredName,
    /// The qualified name the variable ranges over.
    pub operation: NameForm,
}

/// A `relation` over named executions (FR-171 "Outputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelRelationForm {
    /// The declared clause name.
    pub name: DeclaredName,
    /// The `using` alias.
    pub profile: UsingAlias,
    /// The execution bindings in source order.
    pub executions: Vec<ExecutionBindingForm>,
    /// The body.
    pub body: ExpressionForm,
    /// The span of the whole clause.
    pub span: Span,
}

/// `hyper Name using p over (behaviours ... | t in Q bounded N) quantifier+
/// ...` (FR-171).
pub(crate) fn hyper_clause(construct: Construct<'_>) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let clause_items = items(cst, node);
    let [_, alias] = tokens_of(&clause_items, TokenKind::Identifier)[..] else {
        return Err(unexpected(node));
    };
    let name = declared_name(&clause_items, node)?;
    let profile = UsingAlias {
        alias: text(alias, node)?,
        span: alias.span(),
    };
    let quantifiers = nodes_of(&clause_items, Production::Quantifier)
        .into_iter()
        .map(|quantifier| quantifier_form(cst, quantifier))
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(domain) = nodes_of(&clause_items, Production::TraceDomain).first() {
        return Ok(DeclarationForm::HyperFinite(Box::new(HyperFiniteForm {
            name,
            profile,
            domain: trace_domain_form(cst, domain)?,
            quantifiers,
            body: expression_form(
                cst,
                only(&clause_items, Production::Block, node)
                    .and_then(|block| only(&items(cst, block), Production::Expression, block))?,
            )?,
            span: node.span(),
        })));
    }
    let domain = only(&clause_items, Production::BehavioursDomain, node)?;
    let parameters = nodes_of(&items(cst, domain), Production::Parameter)
        .into_iter()
        .map(|parameter| parameter_form(cst, parameter))
        .collect::<Result<_, _>>()?;
    let match_block = nodes_of(&clause_items, Production::MatchBlock)
        .first()
        .map(|block| {
            let block_items = items(cst, block);
            expression_form(cst, only(&block_items, Production::HyperExpression, block)?)
        })
        .transpose()?;
    let align_skip = nodes_of(&clause_items, Production::AlignSkip)
        .first()
        .map(|align| {
            let operations = nodes_of(&items(cst, align), Production::QualifiedName)
                .into_iter()
                .map(|operation| name_form(cst, operation))
                .collect::<Result<_, _>>()?;
            Ok::<_, FormsRefusal>(AlignSkipForm {
                operations,
                span: align.span(),
            })
        })
        .transpose()?;
    let body = only(&clause_items, Production::TemporalExpression, node)?;
    Ok(DeclarationForm::HyperBehaviours(Box::new(
        HyperBehavioursForm {
            name,
            profile,
            parameters,
            quantifiers,
            match_block,
            align_skip,
            body: formula_form(cst, body, true)?,
            span: node.span(),
        },
    )))
}

/// `forall|exists trace v [of M fair ...]`.
fn quantifier_form(cst: &LosslessCst, node: &CstNode) -> Result<QuantifierForm, FormsRefusal> {
    let quantifier_items = items(cst, node);
    let kind = if has_token(&quantifier_items, b"forall") {
        QuantifierKind::Forall
    } else {
        QuantifierKind::Exists
    };
    let names = tokens_of(&quantifier_items, TokenKind::Identifier);
    let (variable, alias) = match names[..] {
        [variable] => (variable, None),
        [variable, alias] => (variable, Some(alias)),
        _ => return Err(unexpected(node)),
    };
    let spelled = |token: &qsl_cst::CstToken| -> Result<DeclaredName, FormsRefusal> {
        Ok(DeclaredName {
            name: text(token, node)?,
            span: token.span(),
        })
    };
    Ok(QuantifierForm {
        kind,
        variable: spelled(variable)?,
        alias: alias.map(spelled).transpose()?,
        fairness: nodes_of(&quantifier_items, Production::Fairness)
            .into_iter()
            .map(|constraint| fairness_form(cst, constraint))
            .collect::<Result<_, _>>()?,
        span: node.span(),
    })
}

/// `t in Q bounded N`.
fn trace_domain_form(cst: &LosslessCst, node: &CstNode) -> Result<TraceDomainForm, FormsRefusal> {
    let domain_items = items(cst, node);
    let bound = tokens_of(&domain_items, TokenKind::Integer);
    let [bound] = bound[..] else {
        return Err(unexpected(node));
    };
    Ok(TraceDomainForm {
        variable: declared_name(&domain_items, node)?,
        population: name_form(cst, only(&domain_items, Production::QualifiedName, node)?)?,
        bound: text(bound, node)?.parse().map_err(|_| unexpected(node))?,
        span: node.span(),
    })
}

/// `relation Name using p over (x: Q, y: Q, ...) { body }` (FR-171).
pub(crate) fn relation_clause(construct: Construct<'_>) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let clause_items = items(cst, node);
    let [_, alias] = tokens_of(&clause_items, TokenKind::Identifier)[..] else {
        return Err(unexpected(node));
    };
    let executions = nodes_of(&clause_items, Production::ExecutionBinding)
        .into_iter()
        .map(|binding| {
            let binding_items = items(cst, binding);
            Ok(ExecutionBindingForm {
                variable: declared_name(&binding_items, binding)?,
                operation: name_form(
                    cst,
                    only(&binding_items, Production::QualifiedName, binding)?,
                )?,
            })
        })
        .collect::<Result<_, FormsRefusal>>()?;
    let block = only(&clause_items, Production::Block, node)?;
    let body = only(&items(cst, block), Production::Expression, block)?;
    Ok(DeclarationForm::ModelRelation(Box::new(
        ModelRelationForm {
            name: declared_name(&clause_items, node)?,
            profile: UsingAlias {
                alias: text(alias, node)?,
                span: alias.span(),
            },
            executions,
            body: expression_form(cst, body)?,
            span: node.span(),
        },
    )))
}
