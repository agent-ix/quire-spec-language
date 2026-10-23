// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-447 "Invariants, guards, resets and expression forms": lowering an S2
//! expression to a [`HybridTerm`].
//!
//! The walk keeps its own explicit work stack, so an expression of any depth
//! lowers in constant native stack, and charges every term node it builds
//! against the package's node ceiling, so a `let` that substitutes a large
//! term many times refuses at the ceiling rather than exhausting memory.
//! S3 folds no constant: `2 - 1` stays `Arithmetic(Sub, 2, 1)`.

use std::collections::BTreeMap;

use qsl_forms::{BinaryOperator, ExprId, ExprNode, ExpressionForm, ExpressionSpans, SpanId};
use qsl_foundation::Span;
use quire_exact::Rational;

use super::cause::MalformedReason;
use super::{ArithmeticOp, CompareOp, HybridCause, HybridTerm, LogicOp, VariableIndex};

/// The kind of value a hybrid expression denotes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Kind {
    /// A real number.
    Real,
    /// A Boolean.
    Boolean,
}

/// A lowered expression: its term, its kind, and the number of term nodes it
/// holds (what a `let` substitution charges again).
#[derive(Clone, Debug)]
pub(super) struct Lowered {
    pub(super) term: HybridTerm,
    pub(super) kind: Kind,
    size: u64,
}

/// Why lowering stopped.
#[derive(Debug)]
pub(super) enum Stop {
    /// The expression breaks a rule of the section.
    Cause(HybridCause),
    /// The package's node ceiling was reached.
    Limit {
        /// The declared ceiling.
        limit: u64,
        /// The node count the refused step would have reached.
        actual: u128,
    },
}

impl From<HybridCause> for Stop {
    fn from(cause: HybridCause) -> Self {
        Self::Cause(cause)
    }
}

/// The package-wide node counter and its ceiling.
pub(super) struct Meter<'a> {
    pub(super) used: &'a mut u64,
    pub(super) limit: u64,
}

impl Meter<'_> {
    fn charge(&mut self, nodes: u64) -> Result<(), Stop> {
        let reached = u128::from(*self.used) + u128::from(nodes);
        if reached > u128::from(self.limit) {
            return Err(Stop::Limit {
                limit: self.limit,
                actual: reached,
            });
        }
        *self.used = u64::try_from(reached).unwrap_or(u64::MAX);
        Ok(())
    }
}

/// What an expression's names resolve against.
pub(super) struct Env<'a> {
    /// The declaration's continuous variables by name.
    pub(super) variables: &'a BTreeMap<String, VariableIndex>,
    /// Whether the expression is read in a reset: only there is `pre(x)`
    /// admitted.
    pub(super) reset: bool,
}

enum Task {
    Visit(ExprId, SpanId),
    Finish(ExprId, SpanId),
    Bind(String),
    Unbind,
}

/// The span of `id`, or `parent`'s when the form's spans do not reach it.
fn span_of(spans: &ExpressionSpans, id: SpanId, parent: Span) -> Span {
    spans.span(id).unwrap_or(parent)
}

/// The span node of the `index`th child of `node`, or `node` itself when the
/// spans do not reach it.
fn child(spans: &ExpressionSpans, node: SpanId, index: usize) -> SpanId {
    spans.child(node, index).unwrap_or(node)
}

/// A `let` binder in scope and the term it stands for.
pub(super) type Binder = (String, Lowered);

/// Lower `form` to a term.
pub(super) fn lower(
    form: &ExpressionForm,
    env: &Env<'_>,
    meter: &mut Meter<'_>,
) -> Result<Lowered, Stop> {
    let start = (form.expression.root_id(), form.spans.root());
    lower_at(form, start, &[], env, meter)
}

/// Lower the subtree of `form` at `start` (a node and its span node) under
/// the `let` binders `outer` already in scope.
pub(super) fn lower_at(
    form: &ExpressionForm,
    start: (ExprId, SpanId),
    outer: &[Binder],
    env: &Env<'_>,
    meter: &mut Meter<'_>,
) -> Result<Lowered, Stop> {
    let spans = &form.spans;
    let whole = span_of(spans, start.1, Span { start: 0, end: 0 });
    let mut tasks = vec![Task::Visit(start.0, start.1)];
    let mut values: Vec<Lowered> = Vec::new();
    let mut bound: Vec<Binder> = outer.to_vec();
    while let Some(task) = tasks.pop() {
        match task {
            Task::Visit(id, at) => {
                let span = span_of(spans, at, whole);
                let node = form.expression.at(id).node();
                match node {
                    ExprNode::Boolean(value) => {
                        values.push(leaf(meter, HybridTerm::Boolean(*value), Kind::Boolean)?);
                    }
                    ExprNode::Integer(value) => {
                        let constant = Rational::from_integer(value.clone());
                        values.push(leaf(meter, HybridTerm::Constant(constant), Kind::Real)?);
                    }
                    ExprNode::Rational(numerator, denominator) => {
                        let constant = Rational::new(numerator.clone(), denominator.clone())
                            .map_err(|_| HybridCause::ZeroDenominator { span })?;
                        values.push(leaf(meter, HybridTerm::Constant(constant), Kind::Real)?);
                    }
                    ExprNode::Name(name) => {
                        values.push(resolve(name, span, env, &bound, meter)?);
                    }
                    ExprNode::Pre(operand) => {
                        let _ = operand;
                        values.push(pre(form, id, span, env, &bound, meter)?);
                    }
                    ExprNode::Let {
                        name, value, body, ..
                    } => {
                        tasks.push(Task::Unbind);
                        tasks.push(Task::Visit(*body, child(spans, at, 1)));
                        tasks.push(Task::Bind(name.clone()));
                        tasks.push(Task::Visit(*value, child(spans, at, 0)));
                    }
                    ExprNode::If {
                        condition,
                        then,
                        otherwise,
                    } => {
                        tasks.push(Task::Finish(id, at));
                        tasks.push(Task::Visit(*otherwise, child(spans, at, 2)));
                        tasks.push(Task::Visit(*then, child(spans, at, 1)));
                        tasks.push(Task::Visit(*condition, child(spans, at, 0)));
                    }
                    ExprNode::Binary { left, right, .. } => {
                        tasks.push(Task::Finish(id, at));
                        tasks.push(Task::Visit(*right, child(spans, at, 1)));
                        tasks.push(Task::Visit(*left, child(spans, at, 0)));
                    }
                    ExprNode::Negate(operand) | ExprNode::Not(operand) => {
                        tasks.push(Task::Finish(id, at));
                        tasks.push(Task::Visit(*operand, child(spans, at, 0)));
                    }
                    other => return Err(unsupported(other, span)),
                }
            }
            Task::Bind(name) => {
                let value = values.pop().ok_or_else(|| broken(whole))?;
                bound.push((name, value));
            }
            Task::Unbind => {
                bound.pop();
            }
            Task::Finish(id, at) => {
                let span = span_of(spans, at, whole);
                let node = form.expression.at(id).node();
                let lowered = finish(node, span, &mut values, meter)?;
                values.push(lowered);
            }
        }
    }
    values.pop().ok_or_else(|| broken(whole))
}

/// An invariant of the walk broke: the work stack and the value stack
/// disagree. The walk pushes one value per visited node, so no input
/// reaches this.
fn broken(span: Span) -> Stop {
    HybridCause::TypeMismatch { span }.into()
}

fn leaf(meter: &mut Meter<'_>, term: HybridTerm, kind: Kind) -> Result<Lowered, Stop> {
    meter.charge(1)?;
    Ok(Lowered {
        term,
        kind,
        size: 1,
    })
}

/// A node no hybrid expression form admits: every other construct has an
/// operand or result that is neither real nor Boolean, or is an integer
/// operation (QSpec FR-193 "Continuous values and expressions").
fn unsupported(node: &ExprNode, span: Span) -> Stop {
    // A call is resolved by the section that checks pure functions; a name
    // that is no function in scope is `missing_declaration`.
    match node {
        ExprNode::Call { name, .. } => HybridCause::MissingName {
            name: name.clone(),
            span,
        }
        .into(),
        _ => HybridCause::TypeMismatch { span }.into(),
    }
}

fn resolve(
    name: &str,
    span: Span,
    env: &Env<'_>,
    bound: &[Binder],
    meter: &mut Meter<'_>,
) -> Result<Lowered, Stop> {
    if let Some((_, value)) = bound.iter().rev().find(|(binder, _)| binder == name) {
        meter.charge(value.size)?;
        return Ok(value.clone());
    }
    match env.variables.get(name) {
        Some(index) => leaf(meter, HybridTerm::Variable(*index), Kind::Real),
        None => Err(HybridCause::MissingName {
            name: name.to_owned(),
            span,
        }
        .into()),
    }
}

/// `pre(x)`: admitted only in a reset, over a continuous variable, and read
/// before the jump there, which is the variable itself.
fn pre(
    form: &ExpressionForm,
    id: ExprId,
    span: Span,
    env: &Env<'_>,
    bound: &[Binder],
    meter: &mut Meter<'_>,
) -> Result<Lowered, Stop> {
    let malformed = HybridCause::Malformed {
        reason: MalformedReason::PreMisplaced,
        span,
    };
    let ExprNode::Pre(operand) = form.expression.at(id).node() else {
        return Err(malformed.into());
    };
    let ExprNode::Name(name) = form.expression.at(*operand).node() else {
        return Err(malformed.into());
    };
    let shadowed = bound.iter().any(|(binder, _)| binder == name);
    match env.variables.get(name) {
        Some(index) if env.reset && !shadowed => {
            leaf(meter, HybridTerm::Variable(*index), Kind::Real)
        }
        _ => Err(malformed.into()),
    }
}

fn take(values: &mut Vec<Lowered>, span: Span) -> Result<Lowered, Stop> {
    values.pop().ok_or_else(|| broken(span))
}

fn expect(operand: Lowered, kind: Kind, span: Span) -> Result<Lowered, Stop> {
    if operand.kind == kind {
        Ok(operand)
    } else {
        Err(HybridCause::TypeMismatch { span }.into())
    }
}

fn combine(
    meter: &mut Meter<'_>,
    kind: Kind,
    operand_size: u64,
    term: HybridTerm,
) -> Result<Lowered, Stop> {
    meter.charge(1)?;
    Ok(Lowered {
        term,
        kind,
        size: operand_size.saturating_add(1),
    })
}

fn finish(
    node: &ExprNode,
    span: Span,
    values: &mut Vec<Lowered>,
    meter: &mut Meter<'_>,
) -> Result<Lowered, Stop> {
    match node {
        ExprNode::Negate(_) => {
            let operand = expect(take(values, span)?, Kind::Real, span)?;
            let size = operand.size;
            combine(
                meter,
                Kind::Real,
                size,
                HybridTerm::Negate(Box::new(operand.term)),
            )
        }
        ExprNode::Not(_) => {
            let operand = expect(take(values, span)?, Kind::Boolean, span)?;
            let size = operand.size;
            combine(
                meter,
                Kind::Boolean,
                size,
                HybridTerm::Not(Box::new(operand.term)),
            )
        }
        ExprNode::If { .. } => {
            let otherwise = take(values, span)?;
            let then = take(values, span)?;
            let condition = expect(take(values, span)?, Kind::Boolean, span)?;
            if then.kind != otherwise.kind {
                return Err(HybridCause::TypeMismatch { span }.into());
            }
            let kind = then.kind;
            let size = condition
                .size
                .saturating_add(then.size)
                .saturating_add(otherwise.size);
            let term = HybridTerm::Conditional(
                Box::new(condition.term),
                Box::new(then.term),
                Box::new(otherwise.term),
            );
            combine(meter, kind, size, term)
        }
        ExprNode::Binary { operator, .. } => {
            let right = take(values, span)?;
            let left = take(values, span)?;
            binary(*operator, span, left, right, meter)
        }
        other => Err(unsupported(other, span)),
    }
}

fn binary(
    operator: BinaryOperator,
    span: Span,
    left: Lowered,
    right: Lowered,
    meter: &mut Meter<'_>,
) -> Result<Lowered, Stop> {
    let size = 1_u64.saturating_add(left.size).saturating_add(right.size);
    let (operands, result, build): (Kind, Kind, Build) = match operator {
        BinaryOperator::Add => (Kind::Real, Kind::Real, Build::Arithmetic(ArithmeticOp::Add)),
        BinaryOperator::Subtract => (Kind::Real, Kind::Real, Build::Arithmetic(ArithmeticOp::Sub)),
        BinaryOperator::Multiply => (Kind::Real, Kind::Real, Build::Arithmetic(ArithmeticOp::Mul)),
        BinaryOperator::Divide => (Kind::Real, Kind::Real, Build::Arithmetic(ArithmeticOp::Div)),
        // An integer operation over a real operand has no meaning over the
        // reals (QSpec FR-193).
        BinaryOperator::IntegerDiv | BinaryOperator::IntegerRem | BinaryOperator::IntegerMod => {
            return Err(HybridCause::TypeMismatch { span }.into());
        }
        BinaryOperator::Equal => (Kind::Real, Kind::Boolean, Build::Compare(CompareOp::Eq)),
        BinaryOperator::NotEqual => (Kind::Real, Kind::Boolean, Build::Compare(CompareOp::Ne)),
        BinaryOperator::Less => (Kind::Real, Kind::Boolean, Build::Compare(CompareOp::Lt)),
        BinaryOperator::LessOrEqual => (Kind::Real, Kind::Boolean, Build::Compare(CompareOp::Le)),
        BinaryOperator::Greater => (Kind::Real, Kind::Boolean, Build::Compare(CompareOp::Gt)),
        BinaryOperator::GreaterOrEqual => {
            (Kind::Real, Kind::Boolean, Build::Compare(CompareOp::Ge))
        }
        BinaryOperator::And => (Kind::Boolean, Kind::Boolean, Build::Logic(LogicOp::And)),
        BinaryOperator::Or => (Kind::Boolean, Kind::Boolean, Build::Logic(LogicOp::Or)),
        BinaryOperator::Implies => (Kind::Boolean, Kind::Boolean, Build::Logic(LogicOp::Implies)),
    };
    let left = Box::new(expect(left, operands, span)?.term);
    let right = Box::new(expect(right, operands, span)?.term);
    let term = match build {
        Build::Arithmetic(op) => HybridTerm::Arithmetic(op, left, right),
        Build::Compare(op) => HybridTerm::Compare(op, left, right),
        Build::Logic(op) => HybridTerm::Logic(op, left, right),
    };
    meter.charge(1)?;
    Ok(Lowered {
        term,
        kind: result,
        size,
    })
}

/// Which term constructor a binary operator builds.
#[derive(Clone, Copy)]
enum Build {
    Arithmetic(ArithmeticOp),
    Compare(CompareOp),
    Logic(LogicOp),
}
