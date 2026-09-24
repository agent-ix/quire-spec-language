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
//! grow with nesting. S2's nesting-depth bound counts `Expression` nodes
//! (and, separately, type-form nodes): a node past the bound refuses the
//! unit with a limit naming that node's span, never a truncated form.

use qsl_cst::{CstElement, CstNode, CstToken, LosslessCst, Production, TokenClass, TokenKind};
use qsl_foundation::Span;
use quire_exact::{CollectionKind, Integer};

use super::dispatch::{Construct, FormsCause, FormsFailure, FormsLimits};
use super::spans::{DeclarationSpans, ExpressionSpans};
use super::syntax::{
    Accumulation, AliasForm, BinaryOperator, BinderQuery, BuiltinType, DeclarationForm,
    DeclaredName, Expression, FieldInitializer, FunctionDeclaration, RecordFieldForm, RecordForm,
    TupleForm, TypeForm, TypeFormHead, UsingAlias,
};

/// One significant child of a CST node: a token or a node.
#[derive(Clone, Copy)]
enum Item<'c> {
    Token(&'c CstToken),
    Node(&'c CstNode),
}

/// `node`'s significant children in order: its nodes, and its own
/// non-trivia tokens.
fn items<'c>(cst: &'c LosslessCst, node: &'c CstNode) -> Vec<Item<'c>> {
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
fn nodes_of<'c>(items: &[Item<'c>], production: Production) -> Vec<&'c CstNode> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Node(node) if node.production() == production => Some(*node),
            Item::Node(_) | Item::Token(_) => None,
        })
        .collect()
}

/// `node`'s own tokens of `kind`, in order.
fn tokens_of<'c>(items: &[Item<'c>], kind: TokenKind) -> Vec<&'c CstToken> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Token(token) if token.kind() == kind => Some(*token),
            Item::Token(_) | Item::Node(_) => None,
        })
        .collect()
}

/// Whether `node` has an own token spelled `spelling`.
fn has_token(items: &[Item<'_>], spelling: &[u8]) -> bool {
    items
        .iter()
        .any(|item| matches!(item, Item::Token(token) if token.spelling() == spelling))
}

/// The refusal for a CST node whose shape its grammar rule does not give.
fn unexpected(node: &CstNode) -> FormsFailure {
    FormsFailure::refused(
        FormsCause::UnexpectedShape {
            production: node.production(),
        },
        node.span(),
    )
}

/// The refusal for a construct no `Expression` variant represents.
fn unrepresented(production: Production, span: Span) -> FormsFailure {
    FormsFailure::refused(FormsCause::UnrepresentedConstruct { production }, span)
}

/// A token's spelling as text.
fn text(token: &CstToken, node: &CstNode) -> Result<String, FormsFailure> {
    std::str::from_utf8(token.spelling())
        .map(str::to_owned)
        .map_err(|_| unexpected(node))
}

/// Every significant token inside `node`'s span, spelled and joined with
/// no separator: a qualified name `M::T`, a signed integer `-2`, a
/// rounding mode `nearest-even`.
fn spelled(cst: &LosslessCst, node: &CstNode) -> Result<String, FormsFailure> {
    let span = node.span();
    let first = cst
        .tokens()
        .partition_point(|token| token.span().start < span.start);
    let mut spelling = String::new();
    for token in cst
        .tokens()
        .get(first..)
        .unwrap_or_default()
        .iter()
        .take_while(|token| token.span().end <= span.end)
        .filter(|token| token.class() == TokenClass::Token)
    {
        spelling.push_str(&text(token, node)?);
    }
    Ok(spelling)
}

/// An integer literal's value.
fn integer(spelling: &str, node: &CstNode) -> Result<Integer, FormsFailure> {
    spelling.parse().map_err(|_| unexpected(node))
}

/// The one production node under a `Declaration` node.
fn production_node<'c>(construct: Construct<'c>) -> Result<&'c CstNode, FormsFailure> {
    items(construct.cst, construct.node)
        .into_iter()
        .find_map(|item| match item {
            Item::Node(node) => Some(node),
            Item::Token(_) => None,
        })
        .ok_or_else(|| unexpected(construct.node))
}

/// The declared name: the production's first identifier.
fn declared_name(items: &[Item<'_>], node: &CstNode) -> Result<DeclaredName, FormsFailure> {
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
fn only<'c>(
    items: &[Item<'c>],
    production: Production,
    node: &CstNode,
) -> Result<&'c CstNode, FormsFailure> {
    match nodes_of(items, production).as_slice() {
        [child] => Ok(child),
        _ => Err(unexpected(node)),
    }
}

/// `function name using alias(parameters): result pure [decreases(m)] {
/// body }` (FR-091 "Function form").
pub(crate) fn function(construct: Construct<'_>) -> Result<DeclarationForm, FormsFailure> {
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
        parameters.push((
            text(name, parameter)?,
            type_form(cst, reference, construct.limits)?,
        ));
    }
    let result = type_form(
        cst,
        only(&items, Production::TypeReference, node)?,
        construct.limits,
    )?;
    let measure = match nodes_of(&items, Production::Expression).as_slice() {
        [] => None,
        [measure] => Some(expression(cst, measure, construct.limits)?),
        _ => return Err(unexpected(node)),
    };
    let block = only(&items, Production::Block, node)?;
    let block_items = self::items(cst, block);
    let (body, body_spans) = expression(
        cst,
        only(&block_items, Production::Expression, block)?,
        construct.limits,
    )?;
    let (measure, measure_spans) = measure.map_or((None, None), |(measure, spans)| {
        (Some(measure), Some(spans))
    });
    let declaration =
        FunctionDeclaration::new(text(name, node)?, parameters, result, measure, body)
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

/// `type Name = T;` (FR-091 "Alias form").
pub(crate) fn alias(construct: Construct<'_>) -> Result<DeclarationForm, FormsFailure> {
    let node = production_node(construct)?;
    let items = items(construct.cst, node);
    Ok(DeclarationForm::Alias(AliasForm {
        name: declared_name(&items, node)?,
        target: type_form(
            construct.cst,
            only(&items, Production::TypeReference, node)?,
            construct.limits,
        )?,
    }))
}

/// `record Name { f: T; g: U?; }` (FR-091 "Record form").
pub(crate) fn record(construct: Construct<'_>) -> Result<DeclarationForm, FormsFailure> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let items = items(cst, node);
    let mut fields = Vec::new();
    for field in nodes_of(&items, Production::Field) {
        let field_items = self::items(cst, field);
        let name = declared_name(&field_items, field)?;
        fields.push(RecordFieldForm {
            name: name.name,
            type_form: type_form(
                cst,
                only(&field_items, Production::TypeReference, field)?,
                construct.limits,
            )?,
            optional: has_token(&field_items, b"?"),
        });
    }
    Ok(DeclarationForm::Record(RecordForm {
        name: declared_name(&items, node)?,
        fields,
    }))
}

/// `tuple Name(T, U);` (FR-091 "Tuple form").
pub(crate) fn tuple(construct: Construct<'_>) -> Result<DeclarationForm, FormsFailure> {
    let node = production_node(construct)?;
    let items = items(construct.cst, node);
    let mut elements = Vec::new();
    for element in nodes_of(&items, Production::TypeReference) {
        elements.push(type_form(construct.cst, element, construct.limits)?);
    }
    Ok(DeclarationForm::Tuple(TupleForm {
        name: declared_name(&items, node)?,
        elements,
    }))
}

/// A `TypeReference` (or a `Reference<Q>` argument's `QualifiedName`) as a
/// type form: its head, its bounds as spelled, and its argument forms,
/// built bottom-up on an explicit stack.
fn type_form(
    cst: &LosslessCst,
    root: &CstNode,
    limits: FormsLimits,
) -> Result<TypeForm, FormsFailure> {
    enum Frame<'c> {
        Enter(&'c CstNode, u64),
        Exit(&'c CstNode, usize),
    }
    let mut frames = vec![Frame::Enter(root, 1)];
    let mut built: Vec<TypeForm> = Vec::new();
    while let Some(frame) = frames.pop() {
        match frame {
            Frame::Enter(node, depth) => {
                if depth > limits.nesting_depth {
                    return Err(FormsFailure::depth(limits, node.span()));
                }
                let arguments = type_arguments(cst, node);
                frames.push(Frame::Exit(node, arguments.len()));
                frames.extend(
                    arguments
                        .into_iter()
                        .rev()
                        .map(|argument| Frame::Enter(argument, depth + 1)),
                );
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
fn type_head(cst: &LosslessCst, node: &CstNode) -> Result<TypeForm, FormsFailure> {
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

/// One `Expression` node before its children are built: its payload, its
/// span, its parent and its children, as arena indices in
/// [`Expression::children`] order.
struct Pending {
    shape: Shape,
    /// The CST production this node maps from, for a refusal naming it.
    production: Production,
    span: Span,
    parent: Option<usize>,
    children: Vec<usize>,
}

/// An `Expression` variant's payload, without its subexpressions.
enum Shape {
    Boolean(bool),
    Integer(Integer),
    Rational(Integer, Integer),
    Name(String),
    Let(String),
    If,
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
}

/// The built subexpressions of one node, consumed in order.
struct Operands(std::vec::IntoIter<Expression>);

impl Operands {
    fn next(&mut self) -> Option<Expression> {
        self.0.next()
    }

    fn boxed(&mut self) -> Option<Box<Expression>> {
        self.0.next().map(Box::new)
    }

    fn rest(self) -> Vec<Expression> {
        self.0.collect()
    }
}

impl Shape {
    /// The expression with this payload over `children`, or `None` when
    /// their number is not the one this shape takes.
    fn build(self, children: Vec<Expression>) -> Option<Expression> {
        let count = children.len();
        let mut operands = Operands(children.into_iter());
        let arity = |expected: usize| (count == expected).then_some(());
        let expression = match self {
            Self::Boolean(value) => {
                arity(0)?;
                Expression::Boolean(value)
            }
            Self::Integer(value) => {
                arity(0)?;
                Expression::Integer(value)
            }
            Self::Rational(numerator, denominator) => {
                arity(0)?;
                Expression::Rational(numerator, denominator)
            }
            Self::Name(name) => {
                arity(0)?;
                Expression::Name(name)
            }
            Self::Let(name) => {
                arity(2)?;
                Expression::Let {
                    name,
                    value: operands.boxed()?,
                    body: operands.boxed()?,
                }
            }
            Self::If => {
                arity(3)?;
                Expression::If {
                    condition: operands.boxed()?,
                    then: operands.boxed()?,
                    otherwise: operands.boxed()?,
                }
            }
            Self::Binary(operator) => {
                arity(2)?;
                Expression::Binary {
                    operator,
                    left: operands.boxed()?,
                    right: operands.boxed()?,
                }
            }
            Self::Negate => {
                arity(1)?;
                Expression::Negate(operands.boxed()?)
            }
            Self::Not => {
                arity(1)?;
                Expression::Not(operands.boxed()?)
            }
            Self::Field(field) => {
                arity(1)?;
                Expression::Field {
                    operand: operands.boxed()?,
                    field,
                }
            }
            Self::Present => {
                arity(1)?;
                Expression::Present(operands.boxed()?)
            }
            Self::Value => {
                arity(1)?;
                Expression::Value(operands.boxed()?)
            }
            Self::Deref => {
                arity(1)?;
                Expression::Deref(operands.boxed()?)
            }
            Self::Pre => {
                arity(1)?;
                Expression::Pre(operands.boxed()?)
            }
            Self::Flatten => {
                arity(1)?;
                Expression::Flatten(operands.boxed()?)
            }
            Self::Size => {
                arity(1)?;
                Expression::Size(operands.boxed()?)
            }
            Self::Call(name) => Expression::Call {
                name,
                arguments: operands.rest(),
            },
            Self::Collection(kind) => Expression::Collection {
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
                Expression::Record { name, fields }
            }
            Self::Convert(target) => {
                arity(1)?;
                Expression::Convert {
                    target,
                    operand: operands.boxed()?,
                }
            }
            Self::AllInstances(target) => {
                arity(1)?;
                Expression::AllInstances {
                    target,
                    population: operands.boxed()?,
                }
            }
            Self::Query(query, binder) => {
                arity(2)?;
                Expression::Query {
                    query,
                    binder,
                    source: operands.boxed()?,
                    body: operands.boxed()?,
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
                Expression::Accumulate {
                    form,
                    accumulator_type: accumulator_type.name,
                    accumulator_type_span: accumulator_type.span,
                    accumulator,
                    binder,
                    source: operands.boxed()?,
                    step: operands.boxed()?,
                    identity: if identity {
                        Some(operands.boxed()?)
                    } else {
                        None
                    },
                }
            }
            Self::Count(result_type, binder) => {
                arity(2)?;
                Expression::Count {
                    result_type: result_type.name,
                    result_type_span: result_type.span,
                    binder,
                    source: operands.boxed()?,
                    predicate: operands.boxed()?,
                }
            }
            Self::Sum(result_type, binder) => {
                arity(2)?;
                Expression::Sum {
                    result_type: result_type.name,
                    result_type_span: result_type.span,
                    binder,
                    source: operands.boxed()?,
                    summand: operands.boxed()?,
                }
            }
            Self::Contains => {
                arity(2)?;
                Expression::Contains {
                    collection: operands.boxed()?,
                    item: operands.boxed()?,
                }
            }
        };
        Some(expression)
    }
}

/// One CST expression node still to map, and where its mapping attaches:
/// the parent's arena index (`None` for the root) and the depth its first
/// `Expression` node takes.
struct Task<'c> {
    node: &'c CstNode,
    parent: Option<usize>,
    depth: u64,
}

/// The pre-order arena of one expression's nodes, filled from its CST on an
/// explicit work stack.
struct Mapping<'c> {
    cst: &'c LosslessCst,
    limits: FormsLimits,
    arena: Vec<Pending>,
    work: Vec<Task<'c>>,
}

/// An expression CST node (`Expression` or any production under it) as its
/// `Expression` tree and the spans of every node (FR-091 "Expression
/// mapping", "Every expression node carries its span").
fn expression(
    cst: &LosslessCst,
    root: &CstNode,
    limits: FormsLimits,
) -> Result<(Expression, ExpressionSpans), FormsFailure> {
    let mut mapping = Mapping {
        cst,
        limits,
        arena: Vec::new(),
        work: vec![Task {
            node: root,
            parent: None,
            depth: 1,
        }],
    };
    while let Some(task) = mapping.work.pop() {
        mapping.map(task)?;
    }
    mapping.finish(root)
}

impl<'c> Mapping<'c> {
    /// Add one `Expression` node at `depth` under `parent`.
    fn node(
        &mut self,
        shape: Shape,
        production: Production,
        span: Span,
        parent: Option<usize>,
        depth: u64,
    ) -> Result<usize, FormsFailure> {
        if depth > self.limits.nesting_depth {
            return Err(FormsFailure::depth(self.limits, span));
        }
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
        Ok(index)
    }

    /// Add one node and queue `children` under it, in source order.
    fn with_children(
        &mut self,
        shape: Shape,
        task: &Task<'c>,
        children: Vec<&'c CstNode>,
    ) -> Result<(), FormsFailure> {
        let index = self.node(
            shape,
            task.node.production(),
            task.node.span(),
            task.parent,
            task.depth,
        )?;
        self.queue(index, task.depth + 1, children);
        Ok(())
    }

    /// Queue `children` under arena node `parent` at `depth`, so the first
    /// is mapped first.
    fn queue(&mut self, parent: usize, depth: u64, children: Vec<&'c CstNode>) {
        self.work
            .extend(children.into_iter().rev().map(|node| Task {
                node,
                parent: Some(parent),
                depth,
            }));
    }

    /// Map `task`'s CST node to nothing (a pass-through), to one node, or
    /// to a chain of nodes.
    fn map(&mut self, task: Task<'c>) -> Result<(), FormsFailure> {
        let node = task.node;
        let items = items(self.cst, node);
        match node.production() {
            Production::Expression => self.expression(task, &items),
            Production::Implication
            | Production::Disjunction
            | Production::Conjunction
            | Production::Comparison
            | Production::Sum
            | Production::Product => self.chain(task, &items),
            Production::Unary => self.unary(task, &items),
            Production::Postfix => self.postfix(task, &items),
            Production::Primary => self.primary(task, &items),
            _ => Err(unexpected(node)),
        }
    }

    /// Continue with `child` in `task`'s place: `(e)` and single-operand
    /// precedence levels map to their operand.
    fn pass(&mut self, task: &Task<'c>, child: &'c CstNode) {
        self.work.push(Task {
            node: child,
            parent: task.parent,
            depth: task.depth,
        });
    }

    fn expression(&mut self, task: Task<'c>, items: &[Item<'c>]) -> Result<(), FormsFailure> {
        let node = task.node;
        let operands = nodes_of(items, Production::Expression);
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

    /// A binary precedence level: `o0 op1 o1 op2 o2 ...`. `implies` nests
    /// to the right in the CST itself; every other level is a
    /// left-associative chain `op_n(... op1(o0, o1) ..., o_n)`.
    fn chain(&mut self, task: Task<'c>, items: &[Item<'c>]) -> Result<(), FormsFailure> {
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
        for (depth, (operator, right)) in (task.depth..).zip(chain.iter().zip(&operands[1..]).rev())
        {
            let span = Span {
                start,
                end: right.span().end,
            };
            let index = self.node(
                Shape::Binary(*operator),
                node.production(),
                span,
                parent,
                depth,
            )?;
            links.push((index, depth));
            parent = Some(index);
        }
        links.reverse();
        // Queue operands so `o0` is mapped first: `o0` and `o1` under the
        // innermost link, and each later `o_k` under link `k`, as its right
        // operand.
        for (right, (link, link_depth)) in operands[1..].iter().zip(&links).rev() {
            self.work.push(Task {
                node: right,
                parent: Some(*link),
                depth: link_depth + 1,
            });
        }
        let (innermost, innermost_depth) =
            links.first().copied().ok_or_else(|| unexpected(node))?;
        self.work.push(Task {
            node: operands[0],
            parent: Some(innermost),
            depth: innermost_depth + 1,
        });
        Ok(())
    }

    fn unary(&mut self, task: Task<'c>, items: &[Item<'c>]) -> Result<(), FormsFailure> {
        let node = task.node;
        match items {
            [Item::Token(token), Item::Node(operand)] => {
                let shape = match token.spelling() {
                    b"not" => Shape::Not,
                    b"-" => Shape::Negate,
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

    /// `primary(.member)*`: a left-nested chain of `Field` nodes. Indexing
    /// `e[i]` has no variant.
    fn postfix(&mut self, task: Task<'c>, items: &[Item<'c>]) -> Result<(), FormsFailure> {
        let node = task.node;
        let Some((Item::Node(primary), rest)) = items.split_first() else {
            return Err(unexpected(node));
        };
        let mut members = Vec::new();
        let mut rest = rest.iter();
        while let Some(item) = rest.next() {
            match item {
                Item::Token(token) if token.spelling() == b"." => {
                    let Some(Item::Token(member)) = rest.next() else {
                        return Err(unexpected(node));
                    };
                    members.push(*member);
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
        let mut depth = task.depth;
        for member in members.iter().rev() {
            let span = Span {
                start,
                end: member.span().end,
            };
            parent = Some(self.node(
                Shape::Field(text(member, node)?),
                node.production(),
                span,
                parent,
                depth,
            )?);
            depth += 1;
        }
        self.work.push(Task {
            node: primary,
            parent,
            depth,
        });
        Ok(())
    }

    fn leaf(&mut self, shape: Shape, task: &Task<'c>) -> Result<(), FormsFailure> {
        self.node(
            shape,
            task.node.production(),
            task.node.span(),
            task.parent,
            task.depth,
        )
        .map(|_| ())
    }

    fn primary(&mut self, task: Task<'c>, items: &[Item<'c>]) -> Result<(), FormsFailure> {
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
    ) -> Result<(), FormsFailure> {
        let node = task.node;
        let one = |operands: &[&'c CstNode]| match operands {
            [operand] => Ok(vec![*operand]),
            _ => Err(unexpected(node)),
        };
        match token.spelling() {
            b"true" => self.leaf(Shape::Boolean(true), &task),
            b"false" => self.leaf(Shape::Boolean(false), &task),
            b"null" | b"none" | b"self" | b"result" | b"reaches" => {
                Err(unrepresented(Production::Primary, node.span()))
            }
            b"present" => self.with_children(Shape::Present, &task, one(&operands)?),
            b"value" => self.with_children(Shape::Value, &task, one(&operands)?),
            b"deref" => self.with_children(Shape::Deref, &task, one(&operands)?),
            b"pre" => self.with_children(Shape::Pre, &task, one(&operands)?),
            b"convert" | b"allInstances" => {
                let target = type_form(
                    self.cst,
                    only(items, Production::TypeReference, node)?,
                    self.limits,
                )?;
                let shape = if token.spelling() == b"convert" {
                    Shape::Convert(target)
                } else {
                    Shape::AllInstances(target)
                };
                self.with_children(shape, &task, one(&operands)?)
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
    fn primary_node(&mut self, task: Task<'c>, child: &'c CstNode) -> Result<(), FormsFailure> {
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
            _ => Err(unexpected(child)),
        }
    }

    /// `map`/`collect`/`filter`/`flatMap`/`forall`/`exists`, `flatten`,
    /// `fold`/`reduce` and `count`/`sum`.
    fn collection_call(
        &mut self,
        task: Task<'c>,
        node: &'c CstNode,
        items: &[Item<'c>],
    ) -> Result<(), FormsFailure> {
        let Some(Item::Token(head)) = items.first() else {
            return Err(unexpected(node));
        };
        let operands = nodes_of(items, Production::Expression);
        let identifiers = tokens_of(items, TokenKind::Identifier);
        let named_type = || -> Result<DeclaredName, FormsFailure> {
            let name = only(items, Production::QualifiedName, node)?;
            Ok(DeclaredName {
                name: spelled(self.cst, name)?,
                span: name.span(),
            })
        };
        let binder = |position: usize| -> Result<String, FormsFailure> {
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

    /// The expression and its spans, from the finished arena: spans in
    /// arena (pre-order) order, so each parent is placed before its
    /// children; expressions in reverse, so each node's children are built
    /// before it.
    fn finish(self, root: &CstNode) -> Result<(Expression, ExpressionSpans), FormsFailure> {
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
        let mut built: Vec<Option<Expression>> = Vec::with_capacity(self.arena.len());
        built.resize_with(self.arena.len(), || None);
        for (index, pending) in self.arena.into_iter().enumerate().rev() {
            let failure = shape_at(&pending);
            let mut children = Vec::with_capacity(pending.children.len());
            for child in pending.children {
                children.push(
                    built
                        .get_mut(child)
                        .and_then(Option::take)
                        .ok_or_else(|| failure.clone())?,
                );
            }
            let expression = pending.shape.build(children).ok_or(failure)?;
            if let Some(slot) = built.get_mut(index) {
                *slot = Some(expression);
            }
        }
        let expression = built
            .first_mut()
            .and_then(Option::take)
            .ok_or_else(|| unexpected(root))?;
        Ok((expression, spans))
    }
}

/// The refusal for an arena node whose children or span do not fit its
/// shape, naming that node's production and span.
fn shape_at(pending: &Pending) -> FormsFailure {
    FormsFailure::refused(
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
