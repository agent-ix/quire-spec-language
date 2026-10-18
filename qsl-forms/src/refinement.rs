// SPDX-License-Identifier: AGPL-3.0-or-later
//! The refinement declaration's S2 form (FR-310): a `refinement Name using p
//! abstract A concrete C { rows }` declaration.
//!
//! This module reads the CST of one `RefinementDeclaration` and builds its
//! [`RefinementForm`]. It selects no meaning, the same contract `value`'s
//! productions keep (FR-091): the `using` alias, both model aliases and every
//! name a row spells stay spelled and spanned, rows keep their source order,
//! and whether the rows are complete is S3's (FR-135 to FR-137).

use qsl_cst::{CstNode, LosslessCst, Production, TokenKind};
use qsl_foundation::Span;

use super::dispatch::{Construct, FormsRefusal};
use super::syntax::{
    DeclarationForm, DeclaredName, ExpressionForm, FairnessConstraintForm, NameForm, UsingAlias,
};
use super::temporal_clause::{expression_form, fairness_form};
use super::value::{
    items, name_form, nodes_of, only, production_node, text, tokens_of, unexpected, Item,
};

/// `refinement Name using p abstract A concrete C { row* }` (FR-310
/// "Outputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefinementForm {
    /// The declared name.
    pub name: DeclaredName,
    /// The `using` alias.
    pub profile: UsingAlias,
    /// The abstract model alias.
    pub abstract_model: DeclaredName,
    /// The concrete model alias.
    pub concrete_model: DeclaredName,
    /// Every row, in source order.
    pub rows: Vec<RefinementRowForm>,
    /// The span of the whole declaration, closing brace included.
    pub span: Span,
}

/// One row of a refinement declaration, as written.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RefinementRowForm {
    /// `population A::p from C::q;`.
    Population(PopulationRowForm),
    /// `object A::T from C::U { field = e; ... }`.
    Object(ObjectRowForm),
    /// `step C::T::op -> target;`.
    Step(StepRowForm),
    /// `assume fair ... C::T::op;` or `ensure fair ... A::T::op;`.
    Fairness(FairnessRowForm),
}

/// `population A::p from C::q;`: the abstract population and its concrete
/// source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PopulationRowForm {
    /// The abstract population.
    pub target: NameForm,
    /// The concrete population it is mapped from.
    pub source: NameForm,
    /// The span of the row, `;` included.
    pub span: Span,
}

/// `object A::T from C::U { field = e; ... }`: an abstract object type, the
/// concrete type it is mapped from and its field definitions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectRowForm {
    /// The abstract object type.
    pub target: NameForm,
    /// The concrete object type it is mapped from.
    pub source: NameForm,
    /// The field rows, in source order.
    pub fields: Vec<FieldRowForm>,
    /// The span of the row, closing brace included.
    pub span: Span,
}

/// `field = e;`: one abstract field defined over the concrete object.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldRowForm {
    /// The abstract field.
    pub field: DeclaredName,
    /// The defining expression.
    pub value: ExpressionForm,
    /// The span of the row, `;` included.
    pub span: Span,
}

/// `step C::T::op -> target;`: the abstract step a concrete operation maps
/// to.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepRowForm {
    /// The concrete operation.
    pub concrete: NameForm,
    /// The right side.
    pub target: StepTargetForm,
    /// The span of the row, `;` included.
    pub span: Span,
}

/// The right side of a step row, as written.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StepTargetForm {
    /// `A::T::op(receiver, argument, ...)`: an abstract operation applied to
    /// its receiver and arguments, the receiver first.
    Apply {
        /// The abstract operation.
        operation: NameForm,
        /// The receiver, then one entry per abstract parameter.
        arguments: Vec<ArgumentForm>,
        /// The span of the application.
        span: Span,
    },
    /// `stutter`.
    Stutter {
        /// The span of the word.
        span: Span,
    },
    /// `any`.
    Any {
        /// The span of the word.
        span: Span,
    },
}

/// One argument of an abstract step.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArgumentForm {
    /// An expression over the concrete step.
    Expression(ExpressionForm),
    /// `_`: the concrete step leaves the argument open.
    Open {
        /// The span of the `_`.
        span: Span,
    },
}

/// Which fairness set a row belongs to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FairnessSide {
    /// `assume`: a constraint the concrete model may rely on.
    Assume,
    /// `ensure`: a constraint the abstract model must be delivered.
    Ensure,
}

/// `assume fair ...;` or `ensure fair ...;`: a kind or granularity the row
/// does not write stays `None`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FairnessRowForm {
    /// The set the row belongs to.
    pub side: FairnessSide,
    /// The constraint, as written.
    pub constraint: FairnessConstraintForm,
}

/// The `refinement` declaration form (FR-310 "Outputs").
pub(crate) fn refinement_declaration(
    construct: Construct<'_>,
) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let declaration_items = items(cst, node);
    // `refinement Name using p abstract A concrete C { ... }`: the two
    // leading words are contextual, so the declaration's direct identifier
    // tokens are the word, the name, the `using` alias, the word, the
    // abstract alias, the word and the concrete alias.
    let [_, name, alias, _, abstract_model, _, concrete_model] =
        tokens_of(&declaration_items, TokenKind::Identifier)[..]
    else {
        return Err(unexpected(node));
    };
    let named = |token: &qsl_cst::CstToken| -> Result<DeclaredName, FormsRefusal> {
        Ok(DeclaredName {
            name: text(token, node)?,
            span: token.span(),
        })
    };
    let rows = declaration_items
        .iter()
        .filter_map(|item| match item {
            Item::Node(row) => Some(*row),
            Item::Token(_) => None,
        })
        .map(|row| row_form(cst, row))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(DeclarationForm::Refinement(Box::new(RefinementForm {
        name: named(name)?,
        profile: UsingAlias {
            alias: text(alias, node)?,
            span: alias.span(),
        },
        abstract_model: named(abstract_model)?,
        concrete_model: named(concrete_model)?,
        rows,
        span: node.span(),
    })))
}

fn row_form(cst: &LosslessCst, row: &CstNode) -> Result<RefinementRowForm, FormsRefusal> {
    let row_items = items(cst, row);
    match row.production() {
        Production::PopulationRow => {
            let [target, source] = nodes_of(&row_items, Production::ModelName)[..] else {
                return Err(unexpected(row));
            };
            Ok(RefinementRowForm::Population(PopulationRowForm {
                target: name_form(cst, target)?,
                source: name_form(cst, source)?,
                span: row.span(),
            }))
        }
        Production::ObjectRow => {
            let [target, source] = nodes_of(&row_items, Production::TypeName)[..] else {
                return Err(unexpected(row));
            };
            Ok(RefinementRowForm::Object(ObjectRowForm {
                target: name_form(cst, target)?,
                source: name_form(cst, source)?,
                fields: nodes_of(&row_items, Production::FieldRow)
                    .into_iter()
                    .map(|field| field_form(cst, field))
                    .collect::<Result<_, _>>()?,
                span: row.span(),
            }))
        }
        Production::StepRow => Ok(RefinementRowForm::Step(StepRowForm {
            concrete: name_form(cst, only(&row_items, Production::OperationName, row)?)?,
            target: step_target_form(cst, only(&row_items, Production::StepTarget, row)?)?,
            span: row.span(),
        })),
        Production::FairnessRow => {
            let side = match row_items.first() {
                Some(Item::Token(word)) if word.spelling() == b"assume" => FairnessSide::Assume,
                Some(Item::Token(word)) if word.spelling() == b"ensure" => FairnessSide::Ensure,
                _ => return Err(unexpected(row)),
            };
            Ok(RefinementRowForm::Fairness(FairnessRowForm {
                side,
                constraint: fairness_form(cst, row)?,
            }))
        }
        _ => Err(unexpected(row)),
    }
}

fn field_form(cst: &LosslessCst, field: &CstNode) -> Result<FieldRowForm, FormsRefusal> {
    let field_items = items(cst, field);
    let Some(Item::Token(name)) = field_items.first() else {
        return Err(unexpected(field));
    };
    Ok(FieldRowForm {
        field: DeclaredName {
            name: text(name, field)?,
            span: name.span(),
        },
        value: expression_form(cst, only(&field_items, Production::Expression, field)?)?,
        span: field.span(),
    })
}

fn step_target_form(cst: &LosslessCst, target: &CstNode) -> Result<StepTargetForm, FormsRefusal> {
    let target_items = items(cst, target);
    let span = target.span();
    if let [Item::Token(word)] = target_items[..] {
        return match word.spelling() {
            b"stutter" => Ok(StepTargetForm::Stutter { span }),
            b"any" => Ok(StepTargetForm::Any { span }),
            _ => Err(unexpected(target)),
        };
    }
    let receiver = expression_form(cst, only(&target_items, Production::Expression, target)?)?;
    let mut arguments = vec![ArgumentForm::Expression(receiver)];
    for argument in nodes_of(&target_items, Production::StepArgument) {
        arguments.push(argument_form(cst, argument)?);
    }
    Ok(StepTargetForm::Apply {
        operation: name_form(cst, only(&target_items, Production::OperationName, target)?)?,
        arguments,
        span,
    })
}

fn argument_form(cst: &LosslessCst, argument: &CstNode) -> Result<ArgumentForm, FormsRefusal> {
    let argument_items = items(cst, argument);
    match argument_items[..] {
        [Item::Token(open)] if open.spelling() == b"_" => Ok(ArgumentForm::Open {
            span: argument.span(),
        }),
        [Item::Node(expression)] if expression.production() == Production::Expression => {
            Ok(ArgumentForm::Expression(expression_form(cst, expression)?))
        }
        _ => Err(unexpected(argument)),
    }
}
