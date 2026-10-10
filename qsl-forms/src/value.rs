// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `Value` family form builder (FR-091, ADR-011 §7.3 M-3b for
//! `Value`): the `function`, `type`, `record` and `tuple` declaration
//! productions, the expression mapping and the type-form reading.
//!
//! This module reads the CST of one declaration and builds its parsed form,
//! with the span of each `Expression` node's CST node (FR-091-AC-10). It
//! resolves no name and mints no identity: every type is a [`TypeForm`],
//! every name a spelling. It depends on the `forms` core, `qsl_cst`,
//! `qsl_foundation` and `quire_exact` only (FR-091-AC-11).
//!
//! Every walk runs on an explicit heap stack, so native stack use does not
//! grow with nesting. S2 takes no limit of its own: it builds at most one
//! node per CST node, so S1's node limit bounds it (FR-257).

use qsl_cst::{CstElement, CstNode, CstToken, LosslessCst, Production, TokenClass, TokenKind};
use qsl_foundation::absence::AbsenceMode;
use qsl_foundation::Span;
use quire_exact::{CollectionKind, Integer};

use super::dispatch::{Construct, FormsCause, FormsRefusal};
use super::spans::{DeclarationSpans, ExpressionSpans};
use super::syntax::{
    Accumulation, AliasForm, ArmForm, BinaryOperator, BinderQuery, BuiltinType, CaseForm,
    DeclarationForm, DeclarationKind, DeclaredName, DimensionForm, DimensionTermForm, EnumForm,
    EnumMemberForm, ExactNumberForm, ExactNumberKind, ExprId, ExprNode, Expression,
    ExpressionBuilder, FieldInitializer, FunctionDeclaration, NameForm, RecordFieldForm,
    RecordForm, TermOperator, TupleForm, TypeForm, TypeFormHead, UnitForm, UsingAlias,
};

/// One significant child of a CST node: a token or a node.
#[derive(Clone, Copy)]
pub(crate) enum Item<'c> {
    Token(&'c CstToken),
    Node(&'c CstNode),
}

/// `node`'s significant children in order: its nodes, and its own
/// non-trivia tokens.
pub(crate) fn items<'c>(cst: &'c LosslessCst, node: &'c CstNode) -> Vec<Item<'c>> {
    node.children()
        .iter()
        .filter_map(|child| match child {
            CstElement::Token(index) => cst
                .tokens()
                .get(*index)
                .filter(|token| token.class() == TokenClass::Token)
                .map(Item::Token),
            CstElement::Node(index) => cst.nodes().get(*index).map(Item::Node),
        })
        .collect()
}

/// `node`'s child nodes of `production`, in order.
pub(crate) fn nodes_of<'c>(items: &[Item<'c>], production: Production) -> Vec<&'c CstNode> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Node(node) if node.production() == production => Some(*node),
            Item::Node(_) | Item::Token(_) => None,
        })
        .collect()
}

/// `node`'s own tokens of `kind`, in order.
pub(crate) fn tokens_of<'c>(items: &[Item<'c>], kind: TokenKind) -> Vec<&'c CstToken> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Token(token) if token.kind() == kind => Some(*token),
            Item::Token(_) | Item::Node(_) => None,
        })
        .collect()
}

/// Whether `node` has an own token spelled `spelling`.
pub(crate) fn has_token(items: &[Item<'_>], spelling: &[u8]) -> bool {
    items
        .iter()
        .any(|item| matches!(item, Item::Token(token) if token.spelling() == spelling))
}

/// The refusal for a CST node whose shape its grammar rule does not give.
pub(crate) fn unexpected(node: &CstNode) -> FormsRefusal {
    FormsRefusal::at(
        FormsCause::UnexpectedShape {
            production: node.production(),
        },
        node.span(),
    )
}

/// The refusal for a construct no `Expression` variant represents.
pub(crate) fn unrepresented(production: Production, span: Span) -> FormsRefusal {
    FormsRefusal::at(FormsCause::UnrepresentedConstruct { production }, span)
}

/// A token's spelling as text.
pub(crate) fn text(token: &CstToken, node: &CstNode) -> Result<String, FormsRefusal> {
    std::str::from_utf8(token.spelling())
        .map(str::to_owned)
        .map_err(|_| unexpected(node))
}

/// Every significant token inside `node`'s span, spelled and joined with
/// no separator: a qualified name `M::T`, a signed integer `-2`, a
/// rounding mode `nearest-even`.
pub(crate) fn spelled(cst: &LosslessCst, node: &CstNode) -> Result<String, FormsRefusal> {
    let mut spelling = String::new();
    for token in significant_tokens(cst, node) {
        spelling.push_str(&text(token, node)?);
    }
    Ok(spelling)
}

/// Every significant token inside `node`'s span, in source order (trivia
/// such as whitespace and comments excluded).
pub(crate) fn significant_tokens<'c>(cst: &'c LosslessCst, node: &CstNode) -> Vec<&'c CstToken> {
    let span = node.span();
    let first = cst
        .tokens()
        .partition_point(|token| token.span().start < span.start);
    cst.tokens()
        .get(first..)
        .unwrap_or_default()
        .iter()
        .take_while(|token| token.span().end <= span.end)
        .filter(|token| token.class() == TokenClass::Token)
        .collect()
}

/// An integer literal's value.
fn integer(spelling: &str, node: &CstNode) -> Result<Integer, FormsRefusal> {
    spelling.parse().map_err(|_| unexpected(node))
}

/// The one production node under a `Declaration` node.
pub(crate) fn production_node<'c>(construct: Construct<'c>) -> Result<&'c CstNode, FormsRefusal> {
    items(construct.cst, construct.node)
        .into_iter()
        .find_map(|item| match item {
            Item::Node(node) => Some(node),
            Item::Token(_) => None,
        })
        .ok_or_else(|| unexpected(construct.node))
}

/// The declared name: the production's first identifier.
pub(crate) fn declared_name(
    items: &[Item<'_>],
    node: &CstNode,
) -> Result<DeclaredName, FormsRefusal> {
    let token = tokens_of(items, TokenKind::Identifier)
        .first()
        .copied()
        .ok_or_else(|| unexpected(node))?;
    Ok(DeclaredName {
        name: text(token, node)?,
        span: token.span(),
    })
}

/// The one child node of `production`.
pub(crate) fn only<'c>(
    items: &[Item<'c>],
    production: Production,
    node: &CstNode,
) -> Result<&'c CstNode, FormsRefusal> {
    match nodes_of(items, production).as_slice() {
        [child] => Ok(child),
        _ => Err(unexpected(node)),
    }
}

/// `function name using alias(parameters): result pure [decreases(m)] {
/// body }` (FR-091 "Function form").
pub(crate) fn function(construct: Construct<'_>) -> Result<DeclarationForm, FormsRefusal> {
    function_like(construct, DeclarationKind::Function)
}

/// `predicate name using alias(parameters): Boolean { body }` (FR-091
/// "Predicate form"): the `forms` `FunctionDeclaration` of kind
/// `Predicate`, with a `Boolean` result type form and no measure.
pub(crate) fn predicate(construct: Construct<'_>) -> Result<DeclarationForm, FormsRefusal> {
    function_like(construct, DeclarationKind::Predicate)
}

/// The one `FunctionDeclaration` reading `function` and `predicate` share:
/// they differ in the result type's spelling and in the `decreases` clause
/// only the `Function` production has.
fn function_like(
    construct: Construct<'_>,
    kind: DeclarationKind,
) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let items = items(cst, node);
    let identifiers = tokens_of(&items, TokenKind::Identifier);
    let [name, alias] = identifiers.as_slice() else {
        return Err(unexpected(node));
    };
    let mut parameters = Vec::new();
    for parameter in nodes_of(&items, Production::Parameter) {
        let parameter_items = self::items(cst, parameter);
        let name = tokens_of(&parameter_items, TokenKind::Identifier)
            .first()
            .copied()
            .ok_or_else(|| unexpected(parameter))?;
        let declared = only(&parameter_items, Production::ParameterType, parameter)?;
        let declared_items = self::items(cst, declared);
        let reference = only(&declared_items, Production::TypeReference, declared)?;
        parameters.push((text(name, parameter)?, type_form(cst, reference)?));
    }
    let result = match kind {
        DeclarationKind::Function => {
            type_form(cst, only(&items, Production::TypeReference, node)?)?
        }
        DeclarationKind::Predicate => {
            let boolean = items
                .iter()
                .find_map(|item| match item {
                    Item::Token(token) if token.spelling() == b"Boolean" => Some(*token),
                    Item::Token(_) | Item::Node(_) => None,
                })
                .ok_or_else(|| unexpected(node))?;
            TypeForm::builtin(BuiltinType::Boolean, boolean.span())
        }
    };
    let measure = match nodes_of(&items, Production::Expression).as_slice() {
        [] => None,
        [measure] => Some(expression(cst, measure)?),
        _ => return Err(unexpected(node)),
    };
    let block = only(&items, Production::Block, node)?;
    let block_items = self::items(cst, block);
    let (body, body_spans) = expression(cst, only(&block_items, Production::Expression, block)?)?;
    let (measure, measure_spans) = measure.map_or((None, None), |(measure, spans)| {
        (Some(measure), Some(spans))
    });
    let declaration =
        FunctionDeclaration::new(text(name, node)?, parameters, result, measure, body)
            .with_kind(kind)
            .with_using(UsingAlias {
                alias: text(alias, node)?,
                span: alias.span(),
            })
            .with_spans(DeclarationSpans {
                declaration: construct.node.span(),
                body: body_spans,
                measure: measure_spans,
            })
            .map_err(|_| unexpected(node))?;
    Ok(DeclarationForm::Function(Box::new(declaration)))
}

/// `[ordered] enum Name { A, B = "text", }` (FR-091 "Enum form"). Members
/// stay in source order; the assembler sorts an unordered enum's cases for
/// its preimage.
pub(crate) fn enumeration(construct: Construct<'_>) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let items = items(cst, node);
    let mut members = Vec::new();
    for member in nodes_of(&items, Production::EnumMember) {
        let member_items = self::items(cst, member);
        let display = match tokens_of(&member_items, TokenKind::Text).as_slice() {
            [] => None,
            [literal] => Some((text(literal, member)?, literal.span())),
            _ => return Err(unexpected(member)),
        };
        members.push(EnumMemberForm {
            case: declared_name(&member_items, member)?,
            display,
        });
    }
    Ok(DeclarationForm::Enum(EnumForm {
        name: declared_name(&items, node)?,
        ordered: has_token(&items, b"ordered"),
        members,
    }))
}

/// A `QualifiedName` node as a name form.
pub(crate) fn name_form(cst: &LosslessCst, node: &CstNode) -> Result<NameForm, FormsRefusal> {
    Ok(NameForm {
        name: spelled(cst, node)?,
        span: node.span(),
    })
}

/// `dimension Name;` and `dimension Name = T * U^-2 / V;` (FR-091
/// "Dimension form"). Terms stay as written: the assembler normalizes.
pub(crate) fn dimension(construct: Construct<'_>) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let items = items(cst, node);
    let mut terms = Vec::new();
    let mut operator = None;
    for item in &items {
        match item {
            Item::Token(token) => match token.spelling() {
                b"*" => operator = Some(TermOperator::Multiply),
                b"/" => operator = Some(TermOperator::Divide),
                _ => {}
            },
            Item::Node(term) if term.production() == Production::DimensionTerm => {
                let term_items = self::items(cst, term);
                let exponent = match nodes_of(&term_items, Production::SignedInteger).as_slice() {
                    [] => None,
                    [exponent] => Some((
                        integer(&spelled(cst, exponent)?, exponent)?,
                        exponent.span(),
                    )),
                    _ => return Err(unexpected(term)),
                };
                terms.push(DimensionTermForm {
                    operator: operator.take(),
                    name: name_form(cst, only(&term_items, Production::QualifiedName, term)?)?,
                    exponent,
                });
            }
            Item::Node(_) => {}
        }
    }
    Ok(DeclarationForm::Dimension(DimensionForm {
        name: declared_name(&items, node)?,
        terms,
    }))
}

/// An `ExactNumber` node as written.
fn exact_number(cst: &LosslessCst, node: &CstNode) -> Result<ExactNumberForm, FormsRefusal> {
    let items = items(cst, node);
    let signed = nodes_of(&items, Production::SignedInteger);
    let (kind, first, second) = if has_token(&items, b"rational") {
        let [numerator, denominator] = signed.as_slice() else {
            return Err(unexpected(node));
        };
        (
            ExactNumberKind::Rational,
            integer(&spelled(cst, numerator)?, node)?,
            integer(&spelled(cst, denominator)?, node)?,
        )
    } else {
        let scales = tokens_of(&items, TokenKind::Integer);
        let ([coefficient], [scale]) = (signed.as_slice(), scales.as_slice()) else {
            return Err(unexpected(node));
        };
        (
            ExactNumberKind::Decimal,
            integer(&spelled(cst, coefficient)?, node)?,
            integer(&text(scale, node)?, node)?,
        )
    };
    Ok(ExactNumberForm {
        kind,
        first,
        second,
        span: node.span(),
    })
}

/// `unit name : Dimension = scale [* target] [+ offset];` (FR-091 "Unit
/// form").
pub(crate) fn unit(construct: Construct<'_>) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let items = items(cst, node);
    let names = nodes_of(&items, Production::QualifiedName);
    let numbers = nodes_of(&items, Production::ExactNumber);
    let (dimension, target) = match names.as_slice() {
        [dimension] => (dimension, None),
        [dimension, target] => (dimension, Some(name_form(cst, target)?)),
        _ => return Err(unexpected(node)),
    };
    let (scale, offset) = match numbers.as_slice() {
        [scale] => (scale, None),
        [scale, offset] => (scale, Some(exact_number(cst, offset)?)),
        _ => return Err(unexpected(node)),
    };
    Ok(DeclarationForm::Unit(UnitForm {
        name: declared_name(&items, node)?,
        dimension: name_form(cst, dimension)?,
        scale: exact_number(cst, scale)?,
        target,
        offset,
    }))
}

/// `type Name = T;` (FR-091 "Alias form").
pub(crate) fn alias(construct: Construct<'_>) -> Result<DeclarationForm, FormsRefusal> {
    let node = production_node(construct)?;
    let items = items(construct.cst, node);
    Ok(DeclarationForm::Alias(AliasForm {
        name: declared_name(&items, node)?,
        target: type_form(
            construct.cst,
            only(&items, Production::TypeReference, node)?,
        )?,
    }))
}

/// `record Name { f: T; g: U?; }` (FR-091 "Record form").
pub(crate) fn record(construct: Construct<'_>) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let items = items(cst, node);
    let mut fields = Vec::new();
    for field in nodes_of(&items, Production::Field) {
        let field_items = self::items(cst, field);
        let name = declared_name(&field_items, field)?;
        fields.push(RecordFieldForm {
            name: name.name,
            type_form: type_form(cst, only(&field_items, Production::TypeReference, field)?)?,
            optional: has_token(&field_items, b"?"),
        });
    }
    Ok(DeclarationForm::Record(RecordForm {
        name: declared_name(&items, node)?,
        fields,
    }))
}

/// `tuple Name(T, U);` (FR-091 "Tuple form").
pub(crate) fn tuple(construct: Construct<'_>) -> Result<DeclarationForm, FormsRefusal> {
    let node = production_node(construct)?;
    let items = items(construct.cst, node);
    let mut elements = Vec::new();
    for element in nodes_of(&items, Production::TypeReference) {
        elements.push(type_form(construct.cst, element)?);
    }
    Ok(DeclarationForm::Tuple(TupleForm {
        name: declared_name(&items, node)?,
        elements,
    }))
}

/// A `TypeReference` (or a `Reference<Q>` argument's `QualifiedName`) as a
/// type form: its head, its bounds as spelled, and its argument forms,
/// built bottom-up on an explicit stack.
pub(crate) fn type_form(cst: &LosslessCst, root: &CstNode) -> Result<TypeForm, FormsRefusal> {
    enum Frame<'c> {
        Enter(&'c CstNode),
        Exit(&'c CstNode, usize),
    }
    let mut frames = vec![Frame::Enter(root)];
    let mut built: Vec<TypeForm> = Vec::new();
    while let Some(frame) = frames.pop() {
        match frame {
            Frame::Enter(node) => {
                let arguments = type_arguments(cst, node);
                frames.push(Frame::Exit(node, arguments.len()));
                frames.extend(arguments.into_iter().rev().map(Frame::Enter));
                #[cfg(test)]
                crate::syntax::stack_peak::note("type_form", frames.len() + built.len());
            }
            Frame::Exit(node, count) => {
                let split = built
                    .len()
                    .checked_sub(count)
                    .ok_or_else(|| unexpected(node))?;
                let arguments = built.split_off(split);
                built.push(type_head(cst, node)?.with_arguments(arguments));
            }
        }
    }
    match built.pop() {
        Some(form) if built.is_empty() => Ok(form),
        Some(_) | None => Err(unexpected(root)),
    }
}

/// The argument nodes of a type-form node: a `TypeReference`'s element or
/// payload `TypeReference`s, or a `Reference<Q>`'s `QualifiedName`. A
/// `TypeReference` headed by a qualified name, and a bare `QualifiedName`,
/// have none.
fn type_arguments<'c>(cst: &'c LosslessCst, node: &'c CstNode) -> Vec<&'c CstNode> {
    if node.production() != Production::TypeReference {
        return Vec::new();
    }
    let items = items(cst, node);
    if matches!(items.first(), Some(Item::Node(_))) {
        return Vec::new();
    }
    items
        .into_iter()
        .filter_map(|item| match item {
            Item::Node(child)
                if matches!(
                    child.production(),
                    Production::TypeReference | Production::QualifiedName
                ) =>
            {
                Some(child)
            }
            Item::Node(_) | Item::Token(_) => None,
        })
        .collect()
}

/// A type-form node's head and bounds, with no arguments yet.
fn type_head(cst: &LosslessCst, node: &CstNode) -> Result<TypeForm, FormsRefusal> {
    let span = node.span();
    if node.production() == Production::QualifiedName {
        return Ok(TypeForm::name(spelled(cst, node)?, span));
    }
    let items = items(cst, node);
    let head = match items.first() {
        Some(Item::Node(name)) if name.production() == Production::QualifiedName => {
            return Ok(TypeForm::name(spelled(cst, name)?, span));
        }
        Some(Item::Token(token)) => token.spelling(),
        Some(Item::Node(_)) | None => return Err(unexpected(node)),
    };
    let head = match head {
        b"Boolean" => TypeFormHead::Builtin(BuiltinType::Boolean),
        b"Integer" => TypeFormHead::Builtin(BuiltinType::Integer),
        b"Int" => TypeFormHead::Builtin(BuiltinType::Int),
        b"Rational" => TypeFormHead::Builtin(BuiltinType::Rational),
        b"Decimal" => TypeFormHead::Builtin(BuiltinType::Decimal),
        b"Float32" => TypeFormHead::Builtin(BuiltinType::Float32),
        b"Float64" => TypeFormHead::Builtin(BuiltinType::Float64),
        b"Text" => TypeFormHead::Builtin(BuiltinType::Text),
        b"Option" => TypeFormHead::Builtin(BuiltinType::Option),
        b"Reference" => TypeFormHead::Builtin(BuiltinType::Reference),
        b"Sequence" => TypeFormHead::Collection(CollectionKind::Sequence),
        b"Set" => TypeFormHead::Collection(CollectionKind::Set),
        b"Bag" => TypeFormHead::Collection(CollectionKind::Bag),
        b"OrderedSet" => TypeFormHead::Collection(CollectionKind::OrderedSet),
        _ => return Err(unexpected(node)),
    };
    let mut bounds = Vec::new();
    for item in items.iter().skip(1) {
        match item {
            Item::Token(token) if token.kind() == TokenKind::Integer => {
                bounds.push(text(token, node)?);
            }
            Item::Node(bound)
                if matches!(
                    bound.production(),
                    Production::SignedInteger | Production::RoundingMode | Production::TextProfile
                ) =>
            {
                bounds.push(spelled(cst, bound)?);
            }
            Item::Token(_) | Item::Node(_) => {}
        }
    }
    Ok(TypeForm::new(head, span).with_bounds(bounds))
}

/// One expression node before its children are built: its payload, its
/// span, its parent and its children, as pending-list indices in
/// [`ExprNode::children`] order.
struct Pending {
    shape: Shape,
    /// The CST production this node maps from, for a refusal naming it.
    production: Production,
    span: Span,
    parent: Option<usize>,
    children: Vec<usize>,
}

/// An [`ExprNode`] variant's payload, without its subexpressions.
enum Shape {
    Boolean(bool),
    Integer(Integer),
    Rational(Integer, Integer),
    Name(String),
    Let(String),
    If,
    Case(Vec<ArmSyntax>, Span),
    Binary(BinaryOperator),
    Negate,
    Not,
    Field(String),
    Present,
    Value,
    Deref,
    Pre,
    Call(String),
    /// Field names in source order, each with whether its value is `null`.
    Record(String, Vec<(String, bool)>),
    Collection(CollectionKind),
    Convert(TypeForm),
    AllInstances(TypeForm),
    Lookup(TypeForm, AbsenceMode),
    Dispatch(String),
    Query(BinderQuery, String),
    Flatten,
    Accumulate {
        form: Accumulation,
        accumulator_type: DeclaredName,
        accumulator: String,
        binder: String,
        identity: bool,
    },
    Count(DeclaredName, String),
    Sum(DeclaredName, String),
    Size,
    Contains,
    /// `self` (FR-102, ADR-012 §15.2, `ProtocolClause`).
    SelfRef,
    /// `result` (FR-102, ADR-012 §15.2, `ProtocolClause`).
    Result,
    /// `reaches(a, b, edge)` (FR-102, ADR-012 §15.2, `StateModel`): the
    /// edge member name and its span.
    Reaches(String, Span),
}

/// The built subexpressions of one node, consumed in order.
struct Operands(std::vec::IntoIter<ExprId>);

/// Case arm metadata before the iterative builder assigns a body handle.
struct ArmSyntax {
    member: NameForm,
    binders: Vec<DeclaredName>,
    span: Span,
}

impl Operands {
    fn next(&mut self) -> Option<ExprId> {
        self.0.next()
    }

    fn rest(self) -> Vec<ExprId> {
        self.0.collect()
    }
}

impl Shape {
    /// The node with this payload over `children`, or `None` when their
    /// number is not the one this shape takes.
    fn build(self, children: Vec<ExprId>) -> Option<ExprNode> {
        let count = children.len();
        let mut operands = Operands(children.into_iter());
        let arity = |expected: usize| (count == expected).then_some(());
        let node = match self {
            Self::Boolean(value) => {
                arity(0)?;
                ExprNode::Boolean(value)
            }
            Self::Integer(value) => {
                arity(0)?;
                ExprNode::Integer(value)
            }
            Self::Rational(numerator, denominator) => {
                arity(0)?;
                ExprNode::Rational(numerator, denominator)
            }
            Self::Name(name) => {
                arity(0)?;
                ExprNode::Name(name)
            }
            Self::Let(name) => {
                arity(2)?;
                ExprNode::Let {
                    name,
                    value: operands.next()?,
                    body: operands.next()?,
                }
            }
            Self::If => {
                arity(3)?;
                ExprNode::If {
                    condition: operands.next()?,
                    then: operands.next()?,
                    otherwise: operands.next()?,
                }
            }
            Self::Case(arms, span) => {
                arity(arms.len() + 1)?;
                let scrutinee = operands.next()?;
                let arms = arms
                    .into_iter()
                    .map(|arm| {
                        Some(ArmForm {
                            member: arm.member,
                            binders: arm.binders,
                            body: operands.next()?,
                            span: arm.span,
                        })
                    })
                    .collect::<Option<_>>()?;
                ExprNode::Case(CaseForm {
                    scrutinee,
                    arms,
                    span,
                })
            }
            Self::Binary(operator) => {
                arity(2)?;
                ExprNode::Binary {
                    operator,
                    left: operands.next()?,
                    right: operands.next()?,
                }
            }
            Self::Negate => {
                arity(1)?;
                ExprNode::Negate(operands.next()?)
            }
            Self::Not => {
                arity(1)?;
                ExprNode::Not(operands.next()?)
            }
            Self::Field(field) => {
                arity(1)?;
                ExprNode::Field {
                    operand: operands.next()?,
                    field,
                }
            }
            Self::Present => {
                arity(1)?;
                ExprNode::Present(operands.next()?)
            }
            Self::Value => {
                arity(1)?;
                ExprNode::Value(operands.next()?)
            }
            Self::Deref => {
                arity(1)?;
                ExprNode::Deref(operands.next()?)
            }
            Self::Pre => {
                arity(1)?;
                ExprNode::Pre(operands.next()?)
            }
            Self::Flatten => {
                arity(1)?;
                ExprNode::Flatten(operands.next()?)
            }
            Self::Size => {
                arity(1)?;
                ExprNode::Size(operands.next()?)
            }
            Self::Call(name) => ExprNode::Call {
                name,
                arguments: operands.rest(),
            },
            Self::Collection(kind) => ExprNode::Collection {
                kind,
                elements: operands.rest(),
            },
            Self::Record(name, names) => {
                arity(names.iter().filter(|(_, null)| !null).count())?;
                let mut fields = Vec::with_capacity(names.len());
                for (field, null) in names {
                    let initializer = if null {
                        FieldInitializer::Null
                    } else {
                        FieldInitializer::Value(operands.next()?)
                    };
                    fields.push((field, initializer));
                }
                ExprNode::Record { name, fields }
            }
            Self::Convert(target) => {
                arity(1)?;
                ExprNode::Convert {
                    target,
                    operand: operands.next()?,
                }
            }
            Self::AllInstances(target) => {
                arity(1)?;
                ExprNode::AllInstances {
                    target,
                    population: operands.next()?,
                }
            }
            Self::Lookup(target, absence) => {
                arity(2)?;
                ExprNode::Lookup {
                    target,
                    population: operands.next()?,
                    reference: operands.next()?,
                    absence,
                }
            }
            Self::Dispatch(member) => ExprNode::Dispatch {
                receiver: operands.next()?,
                member,
                arguments: operands.rest(),
            },
            Self::Query(query, binder) => {
                arity(2)?;
                ExprNode::Query {
                    query,
                    binder,
                    source: operands.next()?,
                    body: operands.next()?,
                }
            }
            Self::Accumulate {
                form,
                accumulator_type,
                accumulator,
                binder,
                identity,
            } => {
                arity(if identity { 3 } else { 2 })?;
                ExprNode::Accumulate {
                    form,
                    accumulator_type: accumulator_type.name,
                    accumulator_type_span: accumulator_type.span,
                    accumulator,
                    binder,
                    source: operands.next()?,
                    step: operands.next()?,
                    identity: if identity {
                        Some(operands.next()?)
                    } else {
                        None
                    },
                }
            }
            Self::Count(result_type, binder) => {
                arity(2)?;
                ExprNode::Count {
                    result_type: result_type.name,
                    result_type_span: result_type.span,
                    binder,
                    source: operands.next()?,
                    predicate: operands.next()?,
                }
            }
            Self::Sum(result_type, binder) => {
                arity(2)?;
                ExprNode::Sum {
                    result_type: result_type.name,
                    result_type_span: result_type.span,
                    binder,
                    source: operands.next()?,
                    summand: operands.next()?,
                }
            }
            Self::Contains => {
                arity(2)?;
                ExprNode::Contains {
                    collection: operands.next()?,
                    item: operands.next()?,
                }
            }
            Self::SelfRef => {
                arity(0)?;
                ExprNode::SelfRef
            }
            Self::Result => {
                arity(0)?;
                ExprNode::Result
            }
            Self::Reaches(edge, edge_span) => {
                arity(2)?;
                ExprNode::Reaches {
                    source: operands.next()?,
                    target: operands.next()?,
                    edge,
                    edge_span,
                }
            }
        };
        Some(node)
    }
}

/// One CST expression node still to map, and where its mapping attaches:
/// the parent's pending-list index (`None` for the root).
struct Task<'c> {
    node: &'c CstNode,
    parent: Option<usize>,
}

/// The pre-order list of one expression's nodes, filled from its CST on an
/// explicit work stack.
struct Mapping<'c> {
    cst: &'c LosslessCst,
    arena: Vec<Pending>,
    work: Vec<Task<'c>>,
}

/// An expression CST node (`Expression` or any production under it) as its
/// `Expression` arena and the spans of every node (FR-091 "Expression
/// mapping", "Every expression node carries its span").
pub(crate) fn expression(
    cst: &LosslessCst,
    root: &CstNode,
) -> Result<(Expression, ExpressionSpans), FormsRefusal> {
    let mut mapping = Mapping {
        cst,
        arena: Vec::new(),
        work: vec![Task {
            node: root,
            parent: None,
        }],
    };
    while let Some(task) = mapping.work.pop() {
        mapping.map(task)?;
        #[cfg(test)]
        crate::syntax::stack_peak::note("expression", mapping.work.len());
    }
    mapping.finish(root)
}

impl<'c> Mapping<'c> {
    /// Add one expression node under `parent`.
    fn node(
        &mut self,
        shape: Shape,
        production: Production,
        span: Span,
        parent: Option<usize>,
    ) -> usize {
        let index = self.arena.len();
        if let Some(parent) = parent.and_then(|parent| self.arena.get_mut(parent)) {
            parent.children.push(index);
        }
        self.arena.push(Pending {
            shape,
            production,
            span,
            parent,
            children: Vec::new(),
        });
        index
    }

    /// Add one node and queue `children` under it, in source order.
    fn with_children(
        &mut self,
        shape: Shape,
        task: &Task<'c>,
        children: Vec<&'c CstNode>,
    ) -> Result<(), FormsRefusal> {
        let index = self.node(shape, task.node.production(), task.node.span(), task.parent);
        self.queue(index, children);
        Ok(())
    }

    /// Queue `children` under pending node `parent`, so the first is mapped
    /// first.
    fn queue(&mut self, parent: usize, children: Vec<&'c CstNode>) {
        self.work
            .extend(children.into_iter().rev().map(|node| Task {
                node,
                parent: Some(parent),
            }));
    }

    /// Map `task`'s CST node to nothing (a pass-through), to one node, or
    /// to a chain of nodes.
    fn map(&mut self, task: Task<'c>) -> Result<(), FormsRefusal> {
        let node = task.node;
        let items = items(self.cst, node);
        match node.production() {
            Production::Expression | Production::ScrutineeExpression => {
                self.expression(task, &items)
            }
            Production::CaseExpression => self.case(task, &items),
            Production::Implication
            | Production::Disjunction
            | Production::Conjunction
            | Production::Comparison
            | Production::Sum
            | Production::Product
            | Production::ScrutineeImplication
            | Production::ScrutineeDisjunction
            | Production::ScrutineeConjunction
            | Production::ScrutineeComparison
            | Production::ScrutineeSum
            | Production::ScrutineeProduct => self.chain(task, &items),
            Production::Unary | Production::ScrutineeUnary => self.unary(task, &items),
            Production::Postfix | Production::ScrutineePostfix => self.postfix(task, &items),
            Production::Primary | Production::ScrutineePrimary => self.primary(task, &items),
            Production::CompleteUnit
            | Production::Header
            | Production::Profile
            | Production::ImportDeclaration
            | Production::Model
            | Production::Declaration
            | Production::TypeReference
            | Production::QualifiedName
            | Production::ModelName
            | Production::TypeName
            | Production::OperationName
            | Production::ParameterType
            | Production::RoundingMode
            | Production::TextProfile
            | Production::DimensionDeclaration
            | Production::DimensionTerm
            | Production::UnitDeclaration
            | Production::EnumDeclaration
            | Production::EnumMember
            | Production::RecordDeclaration
            | Production::Field
            | Production::TupleDeclaration
            | Production::UnionDeclaration
            | Production::UnionMember
            | Production::CaseArm
            | Production::CaseBinder
            | Production::AliasDeclaration
            | Production::FunctionDeclaration
            | Production::Predicate
            | Production::Parameter
            | Production::StateClause
            | Production::Block
            | Production::ExactNumber
            | Production::FloatValue
            | Production::Hex32
            | Production::Hex64
            | Production::HexDigit
            | Production::EnumValue
            | Production::CollectionValue
            | Production::RecordValue
            | Production::FieldValue
            | Production::TupleValue
            | Production::CollectionCall
            | Production::SignedInteger
            | Production::TemporalClause
            | Production::Activation
            | Production::Capture
            | Production::Interval
            | Production::TemporalExpression
            | Production::TemporalImplication
            | Production::TemporalDisjunction
            | Production::TemporalConjunction
            | Production::TemporalRelation
            | Production::TemporalUnary
            | Production::TemporalPrimary
            | Production::ProtocolClause
            | Production::Role
            | Production::RoleLifetime
            | Production::Relationship
            | Production::Channel
            | Production::Ordering
            | Production::DeliveryPolicy
            | Production::Capacity
            | Production::OverflowPolicy
            | Production::ProtocolRequirement
            | Production::NodeReference
            | Production::Compensation
            | Production::Control
            | Production::Sequence
            | Production::Visibility
            | Production::Choice
            | Production::Case
            | Production::Parallel
            | Production::JoinPolicy
            | Production::Branch
            | Production::Repetition
            | Production::AwaitControl
            | Production::Related
            | Production::EventNode
            | Production::Check
            | Production::Commit
            | Production::Finish
            | Production::RelationClause
            | Production::ExecutionBinding
            | Production::HyperClause
            | Production::TraceDomain
            | Production::Quantifier
            | Production::HybridDeclaration
            | Production::HybridMode
            | Production::Equation
            | Production::SynthesisDeclaration
            | Production::VerificationPlan
            | Production::VerificationStep => Err(unexpected(node)),
        }
    }

    /// Continue with `child` in `task`'s place: `(e)` and single-operand
    /// precedence levels map to their operand.
    fn pass(&mut self, task: &Task<'c>, child: &'c CstNode) {
        self.work.push(Task {
            node: child,
            parent: task.parent,
        });
    }

    fn expression(&mut self, task: Task<'c>, items: &[Item<'c>]) -> Result<(), FormsRefusal> {
        let node = task.node;
        let operands = items
            .iter()
            .filter_map(|item| match item {
                Item::Node(child)
                    if matches!(
                        child.production(),
                        Production::Expression | Production::ScrutineeExpression
                    ) =>
                {
                    Some(*child)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        match items.first() {
            Some(Item::Token(token)) if token.spelling() == b"let" => {
                let name = tokens_of(items, TokenKind::Identifier)
                    .first()
                    .copied()
                    .ok_or_else(|| unexpected(node))?;
                if operands.len() != 2 {
                    return Err(unexpected(node));
                }
                self.with_children(Shape::Let(text(name, node)?), &task, operands)
            }
            Some(Item::Token(token)) if token.spelling() == b"if" => {
                if operands.len() != 3 {
                    return Err(unexpected(node));
                }
                self.with_children(Shape::If, &task, operands)
            }
            Some(Item::Node(child)) if items.len() == 1 => {
                self.pass(&task, child);
                Ok(())
            }
            Some(Item::Token(_) | Item::Node(_)) | None => Err(unexpected(node)),
        }
    }

    fn case(&mut self, task: Task<'c>, items: &[Item<'c>]) -> Result<(), FormsRefusal> {
        let mut children = vec![only(items, Production::ScrutineeExpression, task.node)?];
        let mut arms = Vec::new();
        for arm in nodes_of(items, Production::CaseArm) {
            let arm_items = self::items(self.cst, arm);
            let member = name_form(self.cst, only(&arm_items, Production::QualifiedName, arm)?)?;
            let binders = nodes_of(&arm_items, Production::CaseBinder)
                .into_iter()
                .map(|binder| declared_name(&self::items(self.cst, binder), binder))
                .collect::<Result<_, FormsRefusal>>()?;
            let body = only(&arm_items, Production::Expression, arm)?;
            arms.push(ArmSyntax {
                member,
                binders,
                span: arm.span(),
            });
            children.push(body);
        }
        self.with_children(Shape::Case(arms, task.node.span()), &task, children)
    }

    /// A binary precedence level: `o0 op1 o1 op2 o2 ...`. `implies` nests
    /// to the right in the CST itself; every other level is a
    /// left-associative chain `op_n(... op1(o0, o1) ..., o_n)`.
    fn chain(&mut self, task: Task<'c>, items: &[Item<'c>]) -> Result<(), FormsRefusal> {
        let node = task.node;
        let mut operands = Vec::new();
        let mut operators = Vec::new();
        for item in items {
            match item {
                Item::Node(operand) => operands.push(*operand),
                Item::Token(token) => operators.push(*token),
            }
        }
        if operands.len() == 1 && operators.is_empty() {
            self.pass(&task, operands[0]);
            return Ok(());
        }
        if operands.len() != operators.len() + 1 {
            return Err(unexpected(node));
        }
        let mut chain = Vec::with_capacity(operators.len());
        for (position, token) in operators.iter().enumerate() {
            let operator = match token.spelling() {
                b"implies" => BinaryOperator::Implies,
                b"or" => BinaryOperator::Or,
                b"and" => BinaryOperator::And,
                b"=" => BinaryOperator::Equal,
                b"!=" => BinaryOperator::NotEqual,
                b"<" => BinaryOperator::Less,
                b"<=" => BinaryOperator::LessOrEqual,
                b">" => BinaryOperator::Greater,
                b">=" => BinaryOperator::GreaterOrEqual,
                b"+" => BinaryOperator::Add,
                b"-" => BinaryOperator::Subtract,
                b"*" => BinaryOperator::Multiply,
                b"/" => BinaryOperator::Divide,
                b"div" | b"rem" | b"mod" => {
                    let end = operands
                        .get(position + 1)
                        .map_or(node.span().end, |right| right.span().end);
                    return Err(unrepresented(
                        node.production(),
                        Span {
                            start: node.span().start,
                            end,
                        },
                    ));
                }
                _ => return Err(unexpected(node)),
            };
            chain.push(operator);
        }
        // Link `k` (1-based) applies `chain[k - 1]` to operands `0..=k`; the
        // last link is the root, and each link is the left child of the
        // next. Links are added root first, so a parent precedes its child.
        let start = node.span().start;
        let mut parent = task.parent;
        let mut links = Vec::with_capacity(chain.len());
        for (operator, right) in chain.iter().zip(&operands[1..]).rev() {
            let span = Span {
                start,
                end: right.span().end,
            };
            let index = self.node(Shape::Binary(*operator), node.production(), span, parent);
            links.push(index);
            parent = Some(index);
        }
        links.reverse();
        // Queue operands so `o0` is mapped first: `o0` and `o1` under the
        // innermost link, and each later `o_k` under link `k`, as its right
        // operand.
        for (right, link) in operands[1..].iter().zip(&links).rev() {
            self.work.push(Task {
                node: right,
                parent: Some(*link),
            });
        }
        let innermost = links.first().copied().ok_or_else(|| unexpected(node))?;
        self.work.push(Task {
            node: operands[0],
            parent: Some(innermost),
        });
        Ok(())
    }

    fn unary(&mut self, task: Task<'c>, items: &[Item<'c>]) -> Result<(), FormsRefusal> {
        let node = task.node;
        match items {
            [Item::Token(token), Item::Node(operand)] => {
                let shape = match token.spelling() {
                    b"not" => Shape::Not,
                    b"-" => {
                        if self.is_directly_negated_two_pow_127(operand)? {
                            // FR-091: `-170141183460469231731687303715884105728`
                            // is the one literal `i128::MIN`, as Rust folds
                            // `-128i8`.
                            return self.leaf(Shape::Integer(Integer::from(i128::MIN)), &task);
                        }
                        Shape::Negate
                    }
                    _ => return Err(unexpected(node)),
                };
                self.with_children(shape, &task, vec![*operand])
            }
            [Item::Node(operand)] => {
                self.pass(&task, operand);
                Ok(())
            }
            _ => Err(unexpected(node)),
        }
    }

    /// Whether `operand`, the operand of a unary `-`, is exactly one integer
    /// literal token of value 2^127: nothing but layout stands between the
    /// `-` and the literal, and no parenthesis, operator or other token
    /// stands in it (FR-091 "directly negated").
    fn is_directly_negated_two_pow_127(&self, operand: &CstNode) -> Result<bool, FormsRefusal> {
        let tokens = significant_tokens(self.cst, operand);
        let [token] = tokens.as_slice() else {
            return Ok(false);
        };
        if token.kind() != TokenKind::Integer {
            return Ok(false);
        }
        Ok(integer(&text(token, operand)?, operand)? == Integer::from(i128::MIN).neg())
    }

    /// A left-nested chain of field navigation and dispatched calls.
    /// Indexing `e[i]` has no variant.
    fn postfix(&mut self, task: Task<'c>, items: &[Item<'c>]) -> Result<(), FormsRefusal> {
        let node = task.node;
        let Some((Item::Node(primary), rest)) = items.split_first() else {
            return Err(unexpected(node));
        };
        let mut suffixes = Vec::new();
        let mut rest = rest.iter().peekable();
        while let Some(item) = rest.next() {
            match item {
                Item::Token(token) if token.spelling() == b"." => {
                    let Some(Item::Token(member)) = rest.next() else {
                        return Err(unexpected(node));
                    };
                    let mut end = member.span().end;
                    let mut arguments = Vec::new();
                    let call =
                        matches!(rest.peek(), Some(Item::Token(token)) if token.spelling() == b"(");
                    if call {
                        rest.next();
                        loop {
                            match rest.next() {
                                Some(Item::Node(argument))
                                    if argument.production() == Production::Expression =>
                                {
                                    arguments.push(*argument);
                                }
                                Some(Item::Token(token)) if token.spelling() == b"," => {}
                                Some(Item::Token(token)) if token.spelling() == b")" => {
                                    end = token.span().end;
                                    break;
                                }
                                Some(Item::Token(_) | Item::Node(_)) | None => {
                                    return Err(unexpected(node))
                                }
                            }
                        }
                    }
                    let name = text(member, node)?;
                    let shape = if call {
                        Shape::Dispatch(name)
                    } else {
                        Shape::Field(name)
                    };
                    suffixes.push((shape, end, arguments));
                }
                Item::Token(token) if token.spelling() == b"[" => {
                    let end = rest
                        .find_map(|item| match item {
                            Item::Token(close) if close.spelling() == b"]" => {
                                Some(close.span().end)
                            }
                            Item::Token(_) | Item::Node(_) => None,
                        })
                        .unwrap_or(node.span().end);
                    return Err(unrepresented(
                        Production::Postfix,
                        Span {
                            start: node.span().start,
                            end,
                        },
                    ));
                }
                Item::Token(_) | Item::Node(_) => return Err(unexpected(node)),
            }
        }
        let start = node.span().start;
        let mut parent = task.parent;
        for (shape, end, arguments) in suffixes.into_iter().rev() {
            let span = Span { start, end };
            let index = self.node(shape, node.production(), span, parent);
            self.queue(index, arguments);
            parent = Some(index);
        }
        self.work.push(Task {
            node: primary,
            parent,
        });
        Ok(())
    }

    fn leaf(&mut self, shape: Shape, task: &Task<'c>) -> Result<(), FormsRefusal> {
        self.node(shape, task.node.production(), task.node.span(), task.parent);
        Ok(())
    }

    fn primary(&mut self, task: Task<'c>, items: &[Item<'c>]) -> Result<(), FormsRefusal> {
        let node = task.node;
        let operands = nodes_of(items, Production::Expression);
        match items.first() {
            Some(Item::Node(child)) => self.primary_node(task, child),
            Some(Item::Token(token)) => match token.kind() {
                TokenKind::Integer => {
                    let value = integer(&text(token, node)?, node)?;
                    self.leaf(Shape::Integer(value), &task)
                }
                TokenKind::Text => Err(unrepresented(Production::Primary, node.span())),
                TokenKind::Identifier => {
                    // `name(arguments)`: a call of an unqualified name.
                    self.with_children(Shape::Call(text(token, node)?), &task, operands)
                }
                TokenKind::Grammar
                | TokenKind::HexPrefix
                | TokenKind::HexDigit
                | TokenKind::Whitespace
                | TokenKind::Comment
                | TokenKind::Invalid => self.keyword(task, token, items, operands),
            },
            None => Err(unexpected(node)),
        }
    }

    /// A `Primary` headed by a keyword or delimiter token.
    fn keyword(
        &mut self,
        task: Task<'c>,
        token: &CstToken,
        items: &[Item<'c>],
        operands: Vec<&'c CstNode>,
    ) -> Result<(), FormsRefusal> {
        let node = task.node;
        let one = |operands: &[&'c CstNode]| match operands {
            [operand] => Ok(vec![*operand]),
            _ => Err(unexpected(node)),
        };
        match token.spelling() {
            b"true" => self.leaf(Shape::Boolean(true), &task),
            b"false" => self.leaf(Shape::Boolean(false), &task),
            b"null" | b"none" => Err(unrepresented(Production::Primary, node.span())),
            b"self" => self.leaf(Shape::SelfRef, &task),
            b"result" => self.leaf(Shape::Result, &task),
            b"present" => self.with_children(Shape::Present, &task, one(&operands)?),
            b"value" => self.with_children(Shape::Value, &task, one(&operands)?),
            b"deref" => self.with_children(Shape::Deref, &task, one(&operands)?),
            b"pre" => self.with_children(Shape::Pre, &task, one(&operands)?),
            b"reaches" => {
                if operands.len() != 2 {
                    return Err(unexpected(node));
                }
                let edge_node = only(items, Production::QualifiedName, node)?;
                let edge_items = self::items(self.cst, edge_node);
                let segments = tokens_of(&edge_items, TokenKind::Identifier);
                let [edge_token] = segments.as_slice() else {
                    // S1 parses the edge as a `QualifiedName`; a
                    // multi-segment spelling names a member of the
                    // operands' own type, which S3 resolves (FR-102-AC-3).
                    return Err(unrepresented(Production::QualifiedName, edge_node.span()));
                };
                let edge = text(edge_token, node)?;
                let edge_span = edge_token.span();
                self.with_children(Shape::Reaches(edge, edge_span), &task, operands)
            }
            b"convert" | b"allInstances" => {
                let target = type_form(self.cst, only(items, Production::TypeReference, node)?)?;
                let shape = if token.spelling() == b"convert" {
                    Shape::Convert(target)
                } else {
                    Shape::AllInstances(target)
                };
                self.with_children(shape, &task, one(&operands)?)
            }
            b"lookup" => {
                if operands.len() != 2 {
                    return Err(unexpected(node));
                }
                let target = type_form(self.cst, only(items, Production::TypeReference, node)?)?;
                let Some(Item::Token(mode)) = items.last() else {
                    return Err(unexpected(node));
                };
                let absence = match mode.spelling() {
                    b"undefined" => AbsenceMode::Undefined,
                    b"empty" => AbsenceMode::Empty,
                    b"refused" => AbsenceMode::Refused,
                    _ => return Err(unexpected(node)),
                };
                self.with_children(Shape::Lookup(target, absence), &task, operands)
            }
            b"size" => {
                if !nodes_of(items, Production::TypeReference).is_empty() {
                    return Err(unrepresented(Production::Primary, node.span()));
                }
                self.with_children(Shape::Size, &task, one(&operands)?)
            }
            b"contains" => {
                if operands.len() != 2 {
                    return Err(unexpected(node));
                }
                self.with_children(Shape::Contains, &task, operands)
            }
            b"(" => {
                let [inner] = operands.as_slice() else {
                    return Err(unexpected(node));
                };
                self.pass(&task, inner);
                Ok(())
            }
            _ => Err(unexpected(node)),
        }
    }

    /// A `Primary` whose first child is a node.
    fn primary_node(&mut self, task: Task<'c>, child: &'c CstNode) -> Result<(), FormsRefusal> {
        let items = items(self.cst, child);
        match child.production() {
            Production::QualifiedName | Production::EnumValue => {
                let name = spelled(self.cst, child)?;
                self.leaf(Shape::Name(name), &task)
            }
            Production::ExactNumber => {
                if !has_token(&items, b"rational") {
                    return Err(unrepresented(Production::ExactNumber, child.span()));
                }
                let parts = nodes_of(&items, Production::SignedInteger);
                let [numerator, denominator] = parts.as_slice() else {
                    return Err(unexpected(child));
                };
                let numerator = integer(&spelled(self.cst, numerator)?, child)?;
                let denominator = integer(&spelled(self.cst, denominator)?, child)?;
                self.leaf(Shape::Rational(numerator, denominator), &task)
            }
            Production::FloatValue => Err(unrepresented(Production::FloatValue, child.span())),
            Production::CollectionValue => {
                let kind = match items.first() {
                    Some(Item::Token(token)) => match token.spelling() {
                        b"sequence" => CollectionKind::Sequence,
                        b"set" => CollectionKind::Set,
                        b"bag" => CollectionKind::Bag,
                        b"orderedSet" => CollectionKind::OrderedSet,
                        _ => return Err(unexpected(child)),
                    },
                    Some(Item::Node(_)) | None => return Err(unexpected(child)),
                };
                let elements = nodes_of(&items, Production::Expression);
                self.with_children(Shape::Collection(kind), &task, elements)
            }
            Production::RecordValue => {
                let name = spelled(self.cst, only(&items, Production::QualifiedName, child)?)?;
                let mut fields = Vec::new();
                let mut values = Vec::new();
                for field in nodes_of(&items, Production::FieldValue) {
                    let field_items = self::items(self.cst, field);
                    let field_name = tokens_of(&field_items, TokenKind::Identifier)
                        .first()
                        .copied()
                        .ok_or_else(|| unexpected(field))?;
                    let value = only(&field_items, Production::Expression, field)?;
                    let null = is_bare_null(self.cst, value);
                    if !null {
                        values.push(value);
                    }
                    fields.push((text(field_name, field)?, null));
                }
                self.with_children(Shape::Record(name, fields), &task, values)
            }
            Production::TupleValue => {
                let name = spelled(self.cst, only(&items, Production::QualifiedName, child)?)?;
                let arguments = nodes_of(&items, Production::Expression);
                self.with_children(Shape::Call(name), &task, arguments)
            }
            Production::CollectionCall => self.collection_call(task, child, &items),
            Production::UnionDeclaration
            | Production::UnionMember
            | Production::CaseExpression
            | Production::CaseArm
            | Production::CaseBinder
            | Production::ScrutineeExpression
            | Production::ScrutineeImplication
            | Production::ScrutineeDisjunction
            | Production::ScrutineeConjunction
            | Production::ScrutineeComparison
            | Production::ScrutineeSum
            | Production::ScrutineeProduct
            | Production::ScrutineeUnary
            | Production::ScrutineePostfix
            | Production::ScrutineePrimary => Err(unexpected(child)),
            Production::CompleteUnit
            | Production::Header
            | Production::Profile
            | Production::ImportDeclaration
            | Production::Model
            | Production::Declaration
            | Production::TypeReference
            | Production::ModelName
            | Production::TypeName
            | Production::OperationName
            | Production::ParameterType
            | Production::RoundingMode
            | Production::TextProfile
            | Production::DimensionDeclaration
            | Production::DimensionTerm
            | Production::UnitDeclaration
            | Production::EnumDeclaration
            | Production::EnumMember
            | Production::RecordDeclaration
            | Production::Field
            | Production::TupleDeclaration
            | Production::AliasDeclaration
            | Production::FunctionDeclaration
            | Production::Predicate
            | Production::Parameter
            | Production::StateClause
            | Production::Block
            | Production::Expression
            | Production::Implication
            | Production::Disjunction
            | Production::Conjunction
            | Production::Comparison
            | Production::Sum
            | Production::Product
            | Production::Unary
            | Production::Postfix
            | Production::Primary
            | Production::Hex32
            | Production::Hex64
            | Production::HexDigit
            | Production::FieldValue
            | Production::SignedInteger
            | Production::TemporalClause
            | Production::Activation
            | Production::Capture
            | Production::Interval
            | Production::TemporalExpression
            | Production::TemporalImplication
            | Production::TemporalDisjunction
            | Production::TemporalConjunction
            | Production::TemporalRelation
            | Production::TemporalUnary
            | Production::TemporalPrimary
            | Production::ProtocolClause
            | Production::Role
            | Production::RoleLifetime
            | Production::Relationship
            | Production::Channel
            | Production::Ordering
            | Production::DeliveryPolicy
            | Production::Capacity
            | Production::OverflowPolicy
            | Production::ProtocolRequirement
            | Production::NodeReference
            | Production::Compensation
            | Production::Control
            | Production::Sequence
            | Production::Visibility
            | Production::Choice
            | Production::Case
            | Production::Parallel
            | Production::JoinPolicy
            | Production::Branch
            | Production::Repetition
            | Production::AwaitControl
            | Production::Related
            | Production::EventNode
            | Production::Check
            | Production::Commit
            | Production::Finish
            | Production::RelationClause
            | Production::ExecutionBinding
            | Production::HyperClause
            | Production::TraceDomain
            | Production::Quantifier
            | Production::HybridDeclaration
            | Production::HybridMode
            | Production::Equation
            | Production::SynthesisDeclaration
            | Production::VerificationPlan
            | Production::VerificationStep => Err(unexpected(child)),
        }
    }

    /// `map`/`collect`/`filter`/`flatMap`/`forall`/`exists`, `flatten`,
    /// `fold`/`reduce` and `count`/`sum`.
    fn collection_call(
        &mut self,
        task: Task<'c>,
        node: &'c CstNode,
        items: &[Item<'c>],
    ) -> Result<(), FormsRefusal> {
        let Some(Item::Token(head)) = items.first() else {
            return Err(unexpected(node));
        };
        let operands = nodes_of(items, Production::Expression);
        let identifiers = tokens_of(items, TokenKind::Identifier);
        let named_type = || -> Result<DeclaredName, FormsRefusal> {
            let name = only(items, Production::QualifiedName, node)?;
            Ok(DeclaredName {
                name: spelled(self.cst, name)?,
                span: name.span(),
            })
        };
        let binder = |position: usize| -> Result<String, FormsRefusal> {
            identifiers
                .get(position)
                .map_or_else(|| Err(unexpected(node)), |token| text(token, node))
        };
        let query = match head.spelling() {
            b"map" | b"collect" => Some(BinderQuery::Map),
            b"filter" => Some(BinderQuery::Filter),
            b"flatMap" => Some(BinderQuery::FlatMap),
            b"forall" => Some(BinderQuery::Forall),
            b"exists" => Some(BinderQuery::Exists),
            _ => None,
        };
        let shape = if let Some(query) = query {
            Shape::Query(query, binder(0)?)
        } else {
            match head.spelling() {
                b"flatten" => Shape::Flatten,
                b"fold" | b"reduce" => Shape::Accumulate {
                    form: if head.spelling() == b"fold" {
                        Accumulation::Fold
                    } else {
                        Accumulation::Reduce
                    },
                    accumulator_type: named_type()?,
                    accumulator: binder(0)?,
                    binder: binder(1)?,
                    identity: operands.len() == 3,
                },
                b"count" => Shape::Count(named_type()?, binder(0)?),
                b"sum" => Shape::Sum(named_type()?, binder(0)?),
                _ => return Err(unexpected(node)),
            }
        };
        let task = Task { node, ..task };
        self.with_children(shape, &task, operands)
    }

    /// The expression and its spans, from the finished pending list: spans
    /// in pending (pre-order) order, so each parent is placed before its
    /// children; expression nodes in reverse, so each node's children are
    /// stored before it and the root, pending node 0, is stored last.
    fn finish(self, root: &CstNode) -> Result<(Expression, ExpressionSpans), FormsRefusal> {
        let Some(first) = self.arena.first() else {
            return Err(unexpected(root));
        };
        let mut spans = ExpressionSpans::new(first.span).map_err(|_| unexpected(root))?;
        let mut ids = vec![spans.root()];
        for pending in self.arena.iter().skip(1) {
            let parent = pending
                .parent
                .and_then(|parent| ids.get(parent).copied())
                .ok_or_else(|| shape_at(pending))?;
            ids.push(
                spans
                    .push_child(parent, pending.span)
                    .map_err(|_| shape_at(pending))?,
            );
        }
        let mut built: Vec<Option<ExprId>> = vec![None; self.arena.len()];
        let mut builder = ExpressionBuilder::new();
        for (index, pending) in self.arena.into_iter().enumerate().rev() {
            let failure = shape_at(&pending);
            let mut children = Vec::with_capacity(pending.children.len());
            for child in pending.children {
                children.push(
                    built
                        .get(child)
                        .copied()
                        .flatten()
                        .ok_or_else(|| failure.clone())?,
                );
            }
            let node = pending
                .shape
                .build(children)
                .ok_or_else(|| failure.clone())?;
            let id = builder.push(node).map_err(|_| failure)?;
            if let Some(slot) = built.get_mut(index) {
                *slot = Some(id);
            }
        }
        let expression = builder.build().map_err(|_| unexpected(root))?;
        Ok((expression, spans))
    }
}

/// The refusal for an arena node whose children or span do not fit its
/// shape, naming that node's production and span.
fn shape_at(pending: &Pending) -> FormsRefusal {
    FormsRefusal::at(
        FormsCause::UnexpectedShape {
            production: pending.production,
        },
        pending.span,
    )
}

/// Whether an expression CST node is exactly `null`: a chain of
/// single-child precedence levels down to a `Primary` whose one token is
/// `null` (FR-091: "a field whose whole value is `null` is
/// `FieldInitializer::Null`").
fn is_bare_null(cst: &LosslessCst, node: &CstNode) -> bool {
    let mut node = node;
    loop {
        let items = items(cst, node);
        match items.as_slice() {
            [Item::Token(token)] => {
                return node.production() == Production::Primary && token.spelling() == b"null";
            }
            [Item::Node(child)] => node = child,
            _ => return false,
        }
    }
}
