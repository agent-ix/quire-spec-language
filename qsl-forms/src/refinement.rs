// SPDX-License-Identifier: AGPL-3.0-or-later
//! The refinement declaration's S2 form (FR-310): a `refinement Name using p
//! abstract A concrete C { rows }` declaration.
//!
//! This module reads the CST of one `Refinement` and builds its
//! [`RefinementForm`]. It selects no meaning, the same contract `value`'s
//! productions keep (FR-091): the `using` alias, both model aliases and every
//! name a row spells stay spelled and spanned, rows keep their source order,
//! and whether an unkeyworded step source names an operation, a node or a
//! fence, whether the rows are complete and whether a literal is a member of
//! its declared type are S3's (FR-135 to FR-138, FR-147).

use qsl_cst::{CstNode, CstToken, LosslessCst, Production, TokenKind};
use qsl_foundation::Span;
use quire_exact::Integer;

use super::dispatch::{Construct, FormsRefusal};
use super::syntax::{
    DeclarationForm, DeclaredName, ExactNumberForm, ExpressionForm, FairnessConstraintForm,
    FairnessGranularity, FairnessKind, NameForm, TypeForm, UsingAlias,
};
use super::temporal_clause::expression_form;
use super::value::{
    exact_number, has_token, integer, items, name_form, nodes_of, only, production_node, spelled,
    text, tokens_of, type_form, unexpected, Item,
};

/// `refinement Name using p abstract A concrete C { row* }` (FR-310
/// "Outputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefinementForm {
    /// The declared name.
    pub name: DeclaredName,
    /// The `using` alias.
    pub profile: UsingAlias,
    /// The abstract side: a model alias, or a model alias and a protocol
    /// clause name.
    pub abstract_side: AbstractSideForm,
    /// The concrete model alias.
    pub concrete: DeclaredName,
    /// Every row, in source order.
    pub rows: Vec<RefinementRowForm>,
    /// The span of the whole declaration, closing brace included.
    pub span: Span,
}

/// The abstract side of a refinement header (FR-310 "Outputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AbstractSideForm {
    /// `abstract A`: a model alias.
    Model(DeclaredName),
    /// `abstract A::protocol`: a model alias and a protocol clause of it.
    /// QSpec FR-375's header admits a model alias only, so S1 never produces
    /// this side from source; it waits on that production (Linear STD-182,
    /// ADR-020 QS-11).
    Protocol {
        /// The model alias.
        alias: DeclaredName,
        /// The protocol clause name.
        clause: DeclaredName,
    },
}

/// One row of a refinement declaration, as written.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RefinementRowForm {
    /// `population A::p from C::q;`.
    Population(PopulationRowForm),
    /// `object A::T from C::U { field = e; ... }`.
    Object(ObjectRowForm),
    /// `step source -> target;`.
    Step(StepRowForm),
    /// `history C::T.field : Type = initial { on C::T::op : e; ... }`.
    History(Box<HistoryRowForm>),
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

/// `step source -> target;`: the abstract step a concrete step class maps
/// to.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepRowForm {
    /// The concrete step class.
    pub source: StepSourceForm,
    /// The right side.
    pub target: StepTargetForm,
    /// The span of the row, `;` included.
    pub span: Span,
}

/// The step-class keyword that may lead a step source (QSpec FR-432
/// `protocol-step-source`). Every other source is a bare qualified name.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StepClassKeyword {
    /// `fork`.
    Fork,
    /// `join`.
    Join,
    /// `cend`.
    Cend,
    /// `duplicate`.
    Duplicate,
    /// `lose`.
    Lose,
    /// `spawn`.
    Spawn,
    /// `retire`.
    Retire,
    /// `activate`.
    Activate,
}

/// The concrete side of a step row, as written (QSpec FR-432).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StepSourceForm {
    /// A qualified name with the keyword exactly as written, absent when
    /// none is. Whether an unkeyworded name is an operation, an attempt
    /// node, an event, a send, a receive, a timeout, a finish node or a
    /// fence is S3's (FR-136).
    Named {
        /// The leading keyword, when written.
        kind: Option<StepClassKeyword>,
        /// The name.
        name: NameForm,
    },
    /// `memory <kind>`: a memory step kind.
    Memory(DeclaredName),
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
    /// `node A::P::n` or `node A::P::n(argument, ...)`: an abstract protocol
    /// node and the expressions for its binder record (QSpec FR-432).
    Node {
        /// The abstract protocol node.
        node: NameForm,
        /// The binder expressions, in source order.
        arguments: Vec<ArgumentForm>,
        /// The span of the target.
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

/// `history C::T.field : Type = initial { on C::T::op : e; ... }`: an
/// auxiliary field added to a concrete object type (QSpec FR-375).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryRowForm {
    /// The span of the `history` keyword.
    pub keyword: Span,
    /// The concrete object type the field is added to.
    pub object: NameForm,
    /// The field.
    pub field: DeclaredName,
    /// The declared type.
    pub ty: TypeForm,
    /// The initial value, a literal.
    pub initial: HistoryLiteralForm,
    /// The span of the initial literal.
    pub initial_span: Span,
    /// The updates, in source order.
    pub updates: Vec<HistoryUpdateForm>,
    /// The span of the row, closing brace included.
    pub span: Span,
}

/// `on C::T::op : e;`: one concrete operation's update of a history field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryUpdateForm {
    /// The concrete operation.
    pub operation: NameForm,
    /// The new value.
    pub value: ExpressionForm,
    /// The span of the update, `;` included.
    pub span: Span,
}

/// QSpec FR-375's closed `history-literal` production. A collection, record,
/// float or other non-literal spelling is not a form, because S1 refuses it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryLiteralForm {
    /// An integer, signed as written.
    Integer(Integer),
    /// `true` or `false`.
    Boolean(bool),
    /// `none`.
    None,
    /// A quoted text, decoded.
    Text(String),
    /// `rational(n, d)` or `decimal(c, s)`.
    Exact(ExactNumberForm),
    /// A full-path enum member.
    EnumMember(NameForm),
}

/// Which fairness set a row belongs to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FairnessSide {
    /// `assume`: a constraint the concrete model may rely on.
    Assume,
    /// `ensure`: a constraint the abstract model must be delivered.
    Ensure,
}

/// `assume fair <kind> ...;` or `ensure fair <kind> ...;`. The kind is
/// always written (a refinement row has no weak default, so S1 refuses a
/// kindless row); an unwritten granularity stays `None`, for S3 to read as
/// `whole` (FR-137).
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
    let rows = declaration_items
        .iter()
        .filter_map(|item| match item {
            Item::Node(row) => Some(*row),
            Item::Token(_) => None,
        })
        .map(|row| row_form(cst, row))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(DeclarationForm::Refinement(Box::new(RefinementForm {
        name: declared(name, node)?,
        profile: UsingAlias {
            alias: text(alias, node)?,
            span: alias.span(),
        },
        abstract_side: AbstractSideForm::Model(declared(abstract_model, node)?),
        concrete: declared(concrete_model, node)?,
        rows,
        span: node.span(),
    })))
}

fn declared(token: &CstToken, node: &CstNode) -> Result<DeclaredName, FormsRefusal> {
    Ok(DeclaredName {
        name: text(token, node)?,
        span: token.span(),
    })
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
            source: step_source_form(cst, only(&row_items, Production::StepSource, row)?)?,
            target: step_target_form(cst, only(&row_items, Production::StepTarget, row)?)?,
            span: row.span(),
        })),
        Production::HistoryRow => history_form(cst, row, &row_items),
        Production::FairnessRow => {
            let side = match row_items.first() {
                Some(Item::Token(word)) if word.spelling() == b"assume" => FairnessSide::Assume,
                Some(Item::Token(word)) if word.spelling() == b"ensure" => FairnessSide::Ensure,
                _ => return Err(unexpected(row)),
            };
            Ok(RefinementRowForm::Fairness(FairnessRowForm {
                side,
                constraint: fairness_row_constraint(cst, row, &row_items)?,
            }))
        }
        _ => Err(unexpected(row)),
    }
}

/// `fair (weak | strong) [whole | each] Operation`: the kind is always
/// written, so a row without one is a broken invariant between S1 and S2.
fn fairness_row_constraint(
    cst: &LosslessCst,
    row: &CstNode,
    row_items: &[Item<'_>],
) -> Result<FairnessConstraintForm, FormsRefusal> {
    let kind = match (
        has_token(row_items, b"weak"),
        has_token(row_items, b"strong"),
    ) {
        (true, false) => FairnessKind::Weak,
        (false, true) => FairnessKind::Strong,
        _ => return Err(unexpected(row)),
    };
    let granularity = match (
        has_token(row_items, b"whole"),
        has_token(row_items, b"each"),
    ) {
        (true, false) => Some(FairnessGranularity::Whole),
        (false, true) => Some(FairnessGranularity::Each),
        (false, false) => None,
        (true, true) => return Err(unexpected(row)),
    };
    Ok(FairnessConstraintForm {
        kind: Some(kind),
        operation: name_form(cst, only(row_items, Production::OperationName, row)?)?,
        granularity,
        span: row.span(),
    })
}

fn field_form(cst: &LosslessCst, field: &CstNode) -> Result<FieldRowForm, FormsRefusal> {
    let field_items = items(cst, field);
    let Some(Item::Token(name)) = field_items.first() else {
        return Err(unexpected(field));
    };
    Ok(FieldRowForm {
        field: declared(name, field)?,
        value: expression_form(cst, only(&field_items, Production::Expression, field)?)?,
        span: field.span(),
    })
}

fn step_source_form(cst: &LosslessCst, source: &CstNode) -> Result<StepSourceForm, FormsRefusal> {
    let source_items = items(cst, source);
    match source_items[..] {
        [Item::Token(word), Item::Token(kind)] if word.spelling() == b"memory" => {
            Ok(StepSourceForm::Memory(declared(kind, source)?))
        }
        [Item::Node(name)] => Ok(StepSourceForm::Named {
            kind: None,
            name: name_form(cst, name)?,
        }),
        [Item::Token(keyword), Item::Node(name)] => {
            let kind = match keyword.spelling() {
                b"fork" => StepClassKeyword::Fork,
                b"join" => StepClassKeyword::Join,
                b"cend" => StepClassKeyword::Cend,
                b"duplicate" => StepClassKeyword::Duplicate,
                b"lose" => StepClassKeyword::Lose,
                b"spawn" => StepClassKeyword::Spawn,
                b"retire" => StepClassKeyword::Retire,
                b"activate" => StepClassKeyword::Activate,
                _ => return Err(unexpected(source)),
            };
            Ok(StepSourceForm::Named {
                kind: Some(kind),
                name: name_form(cst, name)?,
            })
        }
        _ => Err(unexpected(source)),
    }
}

fn step_target_form(cst: &LosslessCst, target: &CstNode) -> Result<StepTargetForm, FormsRefusal> {
    let target_items = items(cst, target);
    let span = target.span();
    match target_items.first() {
        Some(Item::Token(word)) if word.spelling() == b"stutter" => {
            Ok(StepTargetForm::Stutter { span })
        }
        Some(Item::Token(word)) if word.spelling() == b"any" => Ok(StepTargetForm::Any { span }),
        Some(Item::Token(word)) if word.spelling() == b"node" => {
            let arguments = nodes_of(&target_items, Production::StepArgument)
                .into_iter()
                .map(|argument| argument_form(cst, argument))
                .collect::<Result<_, _>>()?;
            Ok(StepTargetForm::Node {
                node: name_form(cst, only(&target_items, Production::QualifiedName, target)?)?,
                arguments,
                span,
            })
        }
        _ => {
            let receiver =
                expression_form(cst, only(&target_items, Production::Expression, target)?)?;
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
    }
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

fn history_form(
    cst: &LosslessCst,
    row: &CstNode,
    row_items: &[Item<'_>],
) -> Result<RefinementRowForm, FormsRefusal> {
    // The field is the identifier right after the `.` that follows the type
    // name.
    let field = row_items
        .windows(2)
        .find_map(|pair| match pair {
            [Item::Token(dot), Item::Token(field)] if dot.spelling() == b"." => Some(*field),
            _ => None,
        })
        .ok_or_else(|| unexpected(row))?;
    let updates = nodes_of(row_items, Production::HistoryUpdate)
        .into_iter()
        .map(|update| {
            let update_items = items(cst, update);
            Ok(HistoryUpdateForm {
                operation: name_form(cst, only(&update_items, Production::OperationName, update)?)?,
                value: expression_form(cst, only(&update_items, Production::Expression, update)?)?,
                span: update.span(),
            })
        })
        .collect::<Result<_, FormsRefusal>>()?;
    let Some(Item::Token(keyword)) = row_items.first() else {
        return Err(unexpected(row));
    };
    Ok(RefinementRowForm::History(Box::new(HistoryRowForm {
        keyword: keyword.span(),
        object: name_form(cst, only(row_items, Production::TypeName, row)?)?,
        field: declared(field, row)?,
        ty: type_form(cst, only(row_items, Production::TypeReference, row)?)?,
        initial: history_literal(cst, only(row_items, Production::HistoryLiteral, row)?)?,
        initial_span: only(row_items, Production::HistoryLiteral, row)?.span(),
        updates,
        span: row.span(),
    })))
}

fn history_literal(
    cst: &LosslessCst,
    literal: &CstNode,
) -> Result<HistoryLiteralForm, FormsRefusal> {
    let literal_items = items(cst, literal);
    match literal_items[..] {
        [Item::Node(node)] => match node.production() {
            Production::SignedInteger => Ok(HistoryLiteralForm::Integer(integer(
                &spelled(cst, node)?,
                node,
            )?)),
            Production::ExactNumber => Ok(HistoryLiteralForm::Exact(exact_number(cst, node)?)),
            Production::QualifiedName => Ok(HistoryLiteralForm::EnumMember(name_form(cst, node)?)),
            _ => Err(unexpected(literal)),
        },
        [Item::Token(token)] => match (token.kind(), token.spelling()) {
            (_, b"true") => Ok(HistoryLiteralForm::Boolean(true)),
            (_, b"false") => Ok(HistoryLiteralForm::Boolean(false)),
            (_, b"none") => Ok(HistoryLiteralForm::None),
            (TokenKind::Text, _) => Ok(HistoryLiteralForm::Text(
                qsl_cst::token::decode_quoted(&text(token, literal)?)
                    .ok_or_else(|| unexpected(literal))?,
            )),
            _ => Err(unexpected(literal)),
        },
        _ => Err(unexpected(literal)),
    }
}
