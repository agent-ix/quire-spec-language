// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-561, FR-562: the kernel operation behind each shim.
//!
//! Each operation is the call the interpreter makes for the same checked
//! node (`qsl_eval` S6a), with the same arguments, so the meter sees the same
//! charges at the same points. A fast-path operation replaces no arithmetic
//! here: the kernel computes, and the register only carries the result.

use std::sync::Arc;

use qsl_eval::value::collections::{self, Absorbed, Flow, Iteration, Shape};
use qsl_eval::value::composite::{construct_record, construct_tuple, present_slots, read_field};
use qsl_semantics::check::Scope;
use quire_checked::node::{Arithmetic, OrderedKind};
use quire_eval::{FamilyResult, StateModelUndefined};
use quire_exact::{
    compare_text, evaluate_boolean, evaluate_decimal, evaluate_integer_arithmetic,
    evaluate_rational_arithmetic, form, order_numbers, retain_boolean, retain_composite,
    ArithmeticOperator, BooleanConnective, Charge, ChargePoint, CollectionValue,
    ComparisonOperator, Decimal, DecimalOperation, Integer, IntegerArithmetic, Meter,
    OrderedOperands, OrderingOperator, Rational, RationalArithmetic, Refusal, Undefined, Value,
};
use quire_semantic_value::location::ExpressionLocation;
use quire_semantic_value::loss::{LocatedLoss, ValueLoss};
use quire_semantic_value::stop::{outcome_into_stop, Stop};

use crate::translate::{divide, integer_to_i64, Carried, Op, OpKind, Repr};

/// Why a kernel operation did not return a value.
pub(crate) enum Halt {
    /// The kernel's own stop (`Undefined`, `Refused` or `Incomplete`), passed
    /// up unchanged with the node's location.
    Stop(Stop),
    /// A kernel stop at a node other than the operation's own (a `sum` seed
    /// is located at its summand, not at the `sum`).
    StopAt(Box<(Stop, ExpressionLocation)>),
    /// A family-owned result (an `only` over a population that does not hold
    /// exactly one object), located at the operation's node.
    Family(FamilyResult),
    /// A backend invariant broke.
    Fault(&'static str),
}

impl From<collections::Halt> for Halt {
    fn from(halt: collections::Halt) -> Self {
        lift(halt, None)
    }
}

impl From<Stop> for Halt {
    fn from(stop: Stop) -> Self {
        Self::Stop(stop)
    }
}

/// A one-binder query or fold in progress: the shared iteration state, the
/// arena length when it started and the element just produced. The innermost
/// loop is the last frame; loops nest as the translated code does, so no
/// operation names its frame.
pub(crate) struct Frame {
    iteration: Iteration,
    mark: usize,
    element: Option<Value>,
}

/// What a call keeps beside its registers: the values behind its handles
/// and the loss records its operations produced, in evaluation order.
#[derive(Default)]
pub(crate) struct State {
    pub(crate) arena: Vec<Value>,
    /// The loops in progress, innermost last.
    pub(crate) frames: Vec<Frame>,
    /// Operand values staged for the record or tuple construction that
    /// consumes them, innermost last.
    pub(crate) staged: Vec<Value>,
    pub(crate) losses: Vec<LocatedLoss>,
    /// The most handles the arena held at once, for the tests' bound.
    pub(crate) peak: usize,
    /// The most handles the arena may hold (`None`: unbounded, as in a
    /// state built before `prepare` knows its bound).
    pub(crate) limit: Option<usize>,
}

impl State {
    /// A state whose arena holds the entry's argument handles and may grow
    /// to `limit` handles.
    pub(crate) fn bounded(arena: Vec<Value>, limit: usize) -> Self {
        Self {
            peak: arena.len(),
            arena,
            limit: Some(limit),
            ..Self::default()
        }
    }

    /// Record the arena's size after an operation, and refuse a size over the
    /// bound. With the callee reclamation of [`OpKind::Reclaim`] every
    /// operation of a frame adds at most one handle, and no function is
    /// active twice (recursion is unsupported). An operation in a loop body
    /// runs once per element, but every loop cuts the arena back to the
    /// length it had when the loop started before it visits the next element
    /// ([`OpKind::IterNext`]) and when it finishes, and
    /// the body result is copied out of the arena before that, so within one
    /// visit each operation still holds at most one live handle. The live
    /// handles therefore never exceed the operation table plus the
    /// arguments, whatever the element count; reaching the bound is a broken
    /// backend invariant.
    pub(crate) fn settle(&mut self) -> Result<(), Halt> {
        self.peak = self.peak.max(self.arena.len());
        match self.limit {
            Some(limit) if self.arena.len() > limit => Err(Halt::Fault("jit-arena-bound")),
            _ => Ok(()),
        }
    }
}

fn narrow(value: &Integer) -> Result<i64, Halt> {
    integer_to_i64(value).ok_or(Halt::Fault("jit-fast-path-result-exceeds-64-bits"))
}

/// The value a handle names.
fn handle(raw: i64, arena: &[Value]) -> Result<&Value, Halt> {
    usize::try_from(raw)
        .ok()
        .and_then(|index| arena.get(index))
        .ok_or(Halt::Fault("jit-handle-unbound"))
}

/// The `Integer` an operand names: the register itself, or the arena value
/// its handle indexes.
fn integer(raw: i64, repr: Repr, arena: &[Value]) -> Result<Integer, Halt> {
    match repr {
        Repr::Register => Ok(Integer::from(raw)),
        Repr::Handle => match handle(raw, arena)? {
            Value::Integer(value) => Ok(value.clone()),
            _ => Err(Halt::Fault("jit-handle-not-an-integer")),
        },
    }
}

fn rational(raw: i64, arena: &[Value]) -> Result<&Rational, Halt> {
    match handle(raw, arena)? {
        Value::Rational(value) => Ok(value),
        _ => Err(Halt::Fault("jit-handle-not-a-rational")),
    }
}

fn decimal(raw: i64, arena: &[Value]) -> Result<&Decimal, Halt> {
    match handle(raw, arena)? {
        Value::Decimal(value) => Ok(value),
        _ => Err(Halt::Fault("jit-handle-not-a-decimal")),
    }
}

/// The collection a handle names.
fn collection(raw: i64, arena: &[Value]) -> Result<Arc<CollectionValue>, Halt> {
    match handle(raw, arena)? {
        Value::Collection(value) => Ok(Arc::clone(value)),
        _ => Err(Halt::Fault("jit-handle-not-a-collection")),
    }
}

/// The value an operand names: a `Boolean` register, an `Integer` register or
/// the arena value of a handle.
fn operand_value(raw: i64, repr: Repr, boolean: bool, arena: &[Value]) -> Result<Value, Halt> {
    if repr == Repr::Handle {
        handle(raw, arena).cloned()
    } else if boolean {
        Ok(Value::Boolean(raw != 0))
    } else {
        Ok(Value::Integer(Integer::from(raw)))
    }
}

/// `value` as the register or handle a result of kind `repr` is carried in.
fn deliver(value: Value, repr: Repr, arena: &mut Vec<Value>) -> Result<i64, Halt> {
    match (repr, value) {
        (Repr::Register, Value::Boolean(held)) => Ok(i64::from(held)),
        (Repr::Register, Value::Integer(value)) => narrow(&value),
        (Repr::Register, _) => Err(Halt::Fault("jit-register-kind")),
        (Repr::Handle, value) => push(value, arena),
    }
}

/// The halt a shared collection operation gives. A `sum` seed outside its
/// domain is located at `body`.
fn lift(halt: collections::Halt, body: Option<&ExpressionLocation>) -> Halt {
    match halt {
        collections::Halt::Stop(stop) => Halt::Stop(stop),
        collections::Halt::SumSeedOutOfDomain => {
            let stop = Stop::Undefined(Undefined::SumOutOfDomain);
            match body {
                Some(location) => Halt::StopAt(Box::new((stop, location.clone()))),
                None => Halt::Stop(stop),
            }
        }
        collections::Halt::OnlyNotOne { .. } => Halt::Fault("jit-only-not-one-unexpected"),
        collections::Halt::Fault(fault) => Halt::Fault(fault),
    }
}

/// The innermost loop.
fn innermost(frames: &mut [Frame]) -> Result<&mut Frame, Halt> {
    frames.last_mut().ok_or(Halt::Fault("jit-iteration-frame"))
}

/// The last `count` staged operand values, in order.
fn take_staged(staged: &mut Vec<Value>, count: usize) -> Result<Vec<Value>, Halt> {
    let start = staged
        .len()
        .checked_sub(count)
        .ok_or(Halt::Fault("jit-staged-operands-missing"))?;
    Ok(staged.split_off(start))
}

/// A new arena entry holding `value`, and its handle.
fn push(value: Value, arena: &mut Vec<Value>) -> Result<i64, Halt> {
    let handle = i64::try_from(arena.len()).map_err(|_| Halt::Fault("jit-arena-index"))?;
    arena.push(value);
    Ok(handle)
}

/// The register or handle that carries the `Integer` `value`: a register
/// must hold it (the translator proved so from the checked types), a handle
/// is a new arena entry.
fn carry(value: Integer, repr: Repr, arena: &mut Vec<Value>) -> Result<i64, Halt> {
    match repr {
        Repr::Register => narrow(&value),
        Repr::Handle => push(Value::Integer(value), arena),
    }
}

fn comparison(operator: OrderingOperator) -> ComparisonOperator {
    match operator {
        OrderingOperator::Less => ComparisonOperator::Less,
        OrderingOperator::LessOrEqual => ComparisonOperator::LessOrEqual,
        OrderingOperator::Greater => ComparisonOperator::Greater,
        OrderingOperator::GreaterOrEqual => ComparisonOperator::GreaterOrEqual,
    }
}

/// An ordering over the operands the handles `a` and `b` name, or the
/// registers when both are `Integer`.
fn order(
    (operator, kind): (OrderingOperator, OrderedKind),
    [a, b]: [i64; 2],
    [left, right]: [Repr; 2],
    meter: &mut Meter,
    arena: &[Value],
) -> Result<bool, Halt> {
    let held = match kind {
        OrderedKind::Integers => {
            let (l, r) = (integer(a, left, arena)?, integer(b, right, arena)?);
            return Ok(outcome_into_stop(order_numbers(
                operator,
                OrderedOperands::Integers(&l, &r),
                meter,
            ))?);
        }
        OrderedKind::Rationals => order_numbers(
            operator,
            OrderedOperands::Rationals(rational(a, arena)?, rational(b, arena)?),
            meter,
        ),
        OrderedKind::Decimals => order_numbers(
            operator,
            OrderedOperands::Decimals(decimal(a, arena)?, decimal(b, arena)?),
            meter,
        ),
        OrderedKind::Texts => {
            let (Value::Text(l), Value::Text(r)) = (handle(a, arena)?, handle(b, arena)?) else {
                return Err(Halt::Fault("jit-handle-not-a-text"));
            };
            compare_text(comparison(operator), l, r, meter)
                .map_err(|_| Halt::Fault("jit-ordered-text-comparison-rejected"))?
        }
        OrderedKind::Float | OrderedKind::Enums | OrderedKind::Quantities => {
            return Err(Halt::Fault("jit-ordered-kind-untranslated"))
        }
    };
    Ok(outcome_into_stop(held)?)
}

/// A `Decimal` operation into `target`, recording its loss at `op`.
fn decimal_operation(
    op: &Op,
    operation: DecimalOperation<'_>,
    target: &quire_exact::DecimalType,
    meter: &mut Meter,
    state_losses: &mut Vec<LocatedLoss>,
) -> Result<Value, Halt> {
    let result = outcome_into_stop(evaluate_decimal(operation, target, meter))?;
    if let Some(loss) = result.loss() {
        state_losses.push(LocatedLoss {
            location: op.location.clone(),
            loss: ValueLoss::Decimal(loss.clone()),
        });
    }
    Ok(Value::Decimal(result.value().clone()))
}

/// Run `op` on the operands `raw` under `meter`, reading and writing handles,
/// staged operands and loss records in `state`. Every operation kind has its
/// own arm, so a new kind is a compile error here.
pub(crate) fn run(
    op: &Op,
    raw: [i64; 2],
    meter: &mut Meter,
    scope: &Scope,
    state: &mut State,
) -> Result<i64, Halt> {
    let [a, b] = raw;
    let [left_repr, right_repr] = op.operands;
    let State {
        arena,
        staged,
        losses,
        frames,
        ..
    } = state;
    match &op.kind {
        OpKind::Arithmetic(operator) => {
            let (left, right) = (
                integer(a, left_repr, arena)?,
                integer(b, right_repr, arena)?,
            );
            let operation = match operator {
                Arithmetic::Add => IntegerArithmetic::Add(&left, &right),
                Arithmetic::Subtract => IntegerArithmetic::Subtract(&left, &right),
                Arithmetic::Multiply => IntegerArithmetic::Multiply(&left, &right),
            };
            let result = outcome_into_stop(evaluate_integer_arithmetic(operation, None, meter))?;
            carry(result, op.result, arena)
        }
        OpKind::Negate => {
            let operand = integer(a, left_repr, arena)?;
            let result = outcome_into_stop(evaluate_integer_arithmetic(
                IntegerArithmetic::Negate(&operand),
                None,
                meter,
            ))?;
            carry(result, op.result, arena)
        }
        OpKind::Divide {
            operator,
            selection,
        } => {
            let (left, right) = (
                integer(a, left_repr, arena)?,
                integer(b, right_repr, arena)?,
            );
            let outcome = divide(*operator, selection.as_ref(), &left, &right, meter)
                .ok_or(Halt::Fault("jit-division-selection-missing"))?;
            carry(outcome_into_stop(outcome)?, op.result, arena)
        }
        OpKind::Coerce(target) => {
            let operand = integer(a, left_repr, arena)?;
            if target.contains(&operand) {
                carry(operand, op.result, arena)
            } else {
                Err(Halt::Stop(Stop::Refused(Refusal::IntegerOutOfDomain {
                    target: Box::new(target.clone()),
                })))
            }
        }
        OpKind::Equality {
            checked,
            operands_boolean,
        } => {
            let operand = |raw: i64, repr: Repr| -> Result<Value, Halt> {
                if *operands_boolean {
                    Ok(Value::Boolean(raw != 0))
                } else if repr == Repr::Handle {
                    handle(raw, arena).cloned()
                } else {
                    Ok(Value::Integer(Integer::from(raw)))
                }
            };
            let holds = outcome_into_stop(checked.evaluate(
                scope.types(),
                &operand(a, left_repr)?,
                &operand(b, right_repr)?,
                meter,
            ))?;
            Ok(i64::from(holds))
        }
        OpKind::Order(kind) => {
            order(*kind, [a, b], [left_repr, right_repr], meter, arena).map(i64::from)
        }
        OpKind::Not => {
            let result =
                outcome_into_stop(evaluate_boolean(BooleanConnective::Not(a != 0), meter))?;
            Ok(i64::from(result))
        }
        OpKind::Retain => {
            let result = retain_boolean(a != 0, meter).map_err(Stop::Incomplete)?;
            Ok(i64::from(result))
        }
        OpKind::ChargeCall => {
            meter
                .charge(Charge::new(ChargePoint::FunctionCall))
                .map_err(Stop::Incomplete)?;
            Ok(0)
        }
        OpKind::Literal(value) => push(value.clone(), arena),
        OpKind::Box | OpKind::Unbox => carry(integer(a, left_repr, arena)?, op.result, arena),
        OpKind::Stage { boolean } => {
            let value = if left_repr == Repr::Handle {
                handle(a, arena)?.clone()
            } else if *boolean {
                Value::Boolean(a != 0)
            } else {
                Value::Integer(Integer::from(a))
            };
            staged.push(value);
            Ok(0)
        }
        OpKind::Field {
            field,
            optional,
            value_type,
            into,
        } => {
            let record = handle(a, arena)?.clone();
            let read = read_field(
                scope.types(),
                record,
                field,
                |declaration, field| scope.record_slot(declaration, field),
                *optional,
                value_type,
            )
            .map_err(Halt::Fault)?;
            match (into, read.value) {
                (Carried::Boolean, Value::Boolean(held)) => Ok(i64::from(held)),
                (Carried::Register, Value::Integer(value)) => narrow(&value),
                (Carried::Handle, value) => push(value, arena),
                (Carried::Boolean | Carried::Register, _) => Err(Halt::Fault("jit-field-kind")),
            }
        }
        OpKind::Record { declaration, slots } => {
            let values = take_staged(staged, present_slots(slots))?;
            let value =
                construct_record(scope.types(), declaration, slots, values).map_err(Halt::Fault)?;
            push(outcome_into_stop(retain_composite(value, meter))?, arena)
        }
        OpKind::Tuple { declaration, arity } => {
            let values = take_staged(staged, *arity)?;
            let value = construct_tuple(scope.types(), declaration, values).map_err(Halt::Fault)?;
            push(outcome_into_stop(retain_composite(value, meter))?, arena)
        }
        OpKind::RationalDivide { domain } => {
            let left = Rational::from_integer(integer(a, left_repr, arena)?);
            let right = Rational::from_integer(integer(b, right_repr, arena)?);
            let divided = outcome_into_stop(evaluate_rational_arithmetic(
                RationalArithmetic::Divide(&left, &right),
                Some(domain),
                meter,
            ))?;
            push(Value::Rational(divided), arena)
        }
        OpKind::Rational { operator, domain } => {
            let (left, right) = (rational(a, arena)?, rational(b, arena)?);
            let operation = match operator {
                ArithmeticOperator::Add => RationalArithmetic::Add(left, right),
                ArithmeticOperator::Subtract => RationalArithmetic::Subtract(left, right),
                ArithmeticOperator::Multiply => RationalArithmetic::Multiply(left, right),
                ArithmeticOperator::Divide => RationalArithmetic::Divide(left, right),
            };
            let result =
                outcome_into_stop(evaluate_rational_arithmetic(operation, Some(domain), meter))?;
            push(Value::Rational(result), arena)
        }
        OpKind::RationalNegate { domain } => {
            let operand = rational(a, arena)?;
            let result = outcome_into_stop(evaluate_rational_arithmetic(
                RationalArithmetic::Negate(operand),
                Some(domain),
                meter,
            ))?;
            push(Value::Rational(result), arena)
        }
        OpKind::Decimal { operator, target } => {
            let (left, right) = (decimal(a, arena)?, decimal(b, arena)?);
            let operation = match operator {
                ArithmeticOperator::Add => DecimalOperation::Add(left, right),
                ArithmeticOperator::Subtract => DecimalOperation::Subtract(left, right),
                ArithmeticOperator::Multiply => DecimalOperation::Multiply(left, right),
                ArithmeticOperator::Divide => DecimalOperation::Divide(left, right),
            };
            let result = decimal_operation(op, operation, target, meter, losses)?;
            push(result, arena)
        }
        OpKind::DecimalNegate { target } => {
            let operand = decimal(a, arena)?;
            let result =
                decimal_operation(op, DecimalOperation::Negate(operand), target, meter, losses)?;
            push(result, arena)
        }
        OpKind::ConvertDecimal { target } => {
            let result = match handle(a, arena)? {
                Value::Decimal(source) => {
                    decimal_operation(op, DecimalOperation::Round(source), target, meter, losses)?
                }
                Value::Rational(source) => {
                    let numerator = Decimal::new(source.numerator().clone(), 0);
                    let denominator = Decimal::new(source.denominator().clone(), 0);
                    decimal_operation(
                        op,
                        DecimalOperation::Divide(&numerator, &denominator),
                        target,
                        meter,
                        losses,
                    )?
                }
                _ => return Err(Halt::Fault("jit-decimal-conversion-operand-invalid")),
            };
            push(result, arena)
        }
        OpKind::ChargeElement => {
            collections::charge_element(meter)?;
            Ok(0)
        }
        OpKind::Collection {
            collection_type,
            count,
        } => {
            let values = take_staged(staged, *count)?;
            let value = outcome_into_stop(form(collection_type, values, meter))?;
            push(value, arena)
        }
        OpKind::Size => {
            let source = collection(a, arena)?;
            deliver(collections::size(&source, meter)?, op.result, arena)
        }
        OpKind::Contains { item_boolean } => {
            let source = collection(a, arena)?;
            let item = operand_value(b, right_repr, *item_boolean, arena)?;
            deliver(
                collections::contains(&item, &source, meter)?,
                op.result,
                arena,
            )
        }
        OpKind::Flatten { result_type } => {
            let outer = collection(a, arena)?;
            let value =
                collections::flatten(result_type, &outer, meter).map_err(|h| lift(h, None))?;
            push(value, arena)
        }
        OpKind::ConvertCollection { target } => {
            let source = collection(a, arena)?;
            let (value, _) = collections::convert(target, &source, false, meter)?;
            push(value, arena)
        }
        OpKind::DropFirst => {
            let source = collection(a, arena)?;
            let outcome = quire_eval::collection::drop_first(&source, meter)
                .map_err(|_| Halt::Fault("collection-removal-rejected"))?;
            push(outcome_into_stop(outcome)?, arena)
        }
        OpKind::RemoveOne { item_boolean } => {
            let source = collection(a, arena)?;
            let probe = operand_value(b, right_repr, *item_boolean, arena)?;
            let outcome = quire_eval::collection::remove_one(&source, &probe, scope.types(), meter)
                .map_err(|_| Halt::Fault("collection-removal-rejected"))?;
            push(outcome_into_stop(outcome)?, arena)
        }
        OpKind::IterStart {
            spec,
            population,
            identity_boolean,
        } => {
            let source = collection(a, arena)?;
            // The identity is the second operand; a fold without one is a
            // reduce, a query has none.
            let identity = match identity_boolean {
                Some(boolean) => Some(operand_value(b, right_repr, *boolean, arena)?),
                None => None,
            };
            let reduce = spec.shape == Shape::Fold && identity.is_none();
            match Iteration::start(&spec.of(), source, identity, reduce, meter) {
                Ok(iteration) => {
                    frames.push(Frame {
                        iteration,
                        mark: arena.len(),
                        element: None,
                    });
                    Ok(0)
                }
                Err(collections::Halt::OnlyNotOne { found }) => Err(match population {
                    Some(population) => Halt::Family(FamilyResult::Undefined(Box::new(
                        StateModelUndefined::OnlyNotOne {
                            population: population.clone(),
                            found,
                        },
                    ))),
                    None => Halt::Fault("only-source-is-a-population"),
                }),
                Err(other) => Err(lift(other, None)),
            }
        }
        OpKind::IterNext { spec } => {
            let Frame {
                iteration,
                mark,
                element,
            } = innermost(frames)?;
            arena.truncate(*mark);
            match iteration
                .next(&spec.of(), meter)
                .map_err(|h| lift(h, None))?
            {
                Flow::Exhausted => Ok(0),
                Flow::Visit { element: next, .. } => {
                    *element = Some(next);
                    Ok(1)
                }
            }
        }
        OpKind::IterElement => {
            let element = innermost(frames)?
                .element
                .take()
                .ok_or(Halt::Fault("jit-iteration-element-missing"))?;
            deliver(element, op.result, arena)
        }
        OpKind::IterAccumulator => {
            let accumulator = innermost(frames)?
                .iteration
                .accumulator()
                .cloned()
                .ok_or(Halt::Fault("fold-accumulator-missing"))?;
            deliver(accumulator, op.result, arena)
        }
        OpKind::IterAbsorb {
            spec,
            body,
            result_boolean,
        } => {
            let result = operand_value(a, left_repr, *result_boolean, arena)?;
            let iteration = &mut innermost(frames)?.iteration;
            let absorbed = iteration.absorb(&spec.of(), result, |total, summand| {
                let (sum, loss) = collections::sum_exact(&spec.value_type, total, summand, meter)
                    .ok_or(Halt::Fault("sum-family-invalid"))?
                    .map_err(|h| lift(h, Some(body)))?;
                if let Some(loss) = loss {
                    losses.push(LocatedLoss {
                        location: op.location.clone(),
                        loss: ValueLoss::Decimal(loss),
                    });
                }
                Ok::<Value, Halt>(sum)
            })?;
            match absorbed {
                Absorbed::Continue => Ok(0),
                // `forall` and `exists` are outside the coverage.
                Absorbed::Exit { .. } => Err(Halt::Fault("jit-iteration-exit-untranslated")),
            }
        }
        OpKind::IterFinish { spec } => {
            let Frame {
                iteration, mark, ..
            } = frames.pop().ok_or(Halt::Fault("jit-iteration-frame"))?;
            arena.truncate(mark);
            let finished = iteration
                .finish(&spec.of(), false, meter)
                .map_err(|h| lift(h, None))?;
            deliver(finished.value, op.result, arena)
        }
        OpKind::Mark => i64::try_from(arena.len()).map_err(|_| Halt::Fault("jit-arena-index")),
        OpKind::Reclaim => {
            let mark = usize::try_from(b).map_err(|_| Halt::Fault("jit-arena-mark"))?;
            if mark > arena.len() {
                return Err(Halt::Fault("jit-arena-mark"));
            }
            let result = usize::try_from(a).map_err(|_| Halt::Fault("jit-handle-unbound"));
            match (left_repr, result) {
                // The callee returned a handle from above the mark: its value
                // moves down to the mark. A handle below the mark is the
                // caller's own (an argument passed through) and stays.
                (Repr::Handle, Ok(index)) if index >= mark => {
                    let kept = std::mem::replace(
                        arena
                            .get_mut(index)
                            .ok_or(Halt::Fault("jit-handle-unbound"))?,
                        Value::Boolean(false),
                    );
                    arena.truncate(mark);
                    push(kept, arena)
                }
                (Repr::Handle, Err(fault)) => Err(fault),
                (Repr::Handle | Repr::Register, _) => {
                    arena.truncate(mark);
                    Ok(a)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    /// An arena over its bound is a backend fault, not unbounded growth.
    #[test]
    fn an_arena_over_its_bound_is_a_fault() {
        let mut state = State::bounded(Vec::new(), 1);
        state.arena.push(Value::Boolean(true));
        assert!(state.settle().is_ok());
        state.arena.push(Value::Boolean(false));
        assert!(matches!(
            state.settle(),
            Err(Halt::Fault("jit-arena-bound"))
        ));
        assert_eq!(state.peak, 2);
    }

    fn booleans(count: usize) -> Value {
        use quire_exact::{CardinalityBound, CollectionKind, CollectionType, Outcome, ValueType};
        let kind = CollectionType::new(
            CollectionKind::Sequence,
            ValueType::Boolean,
            Some(CardinalityBound::at_most(4)),
        );
        let mut meter = Meter::new(unlimited());
        match form(&kind, vec![Value::Boolean(true); count], &mut meter) {
            Outcome::Completed(value) => value,
            other => panic!("the sequence forms: {other:?}"),
        }
    }

    fn unlimited() -> quire_exact::ScalarLimits {
        quire_exact::ScalarLimits {
            integer_bits: u64::MAX,
            decimal_digits: u64::MAX,
            scale_expansion: u64::MAX,
            text_input_bytes: u64::MAX,
            text_scalars: u64::MAX,
            normalized_scalars: u64::MAX,
            unit_edges: u64::MAX,
            value_occurrences: u64::MAX,
            work_units: u64::MAX,
            result_units: u64::MAX,
        }
    }

    /// The start of an `only` over a population holding other than one object
    /// is the population's family result, located at the operation's node
    /// (FR-139); over one object it starts, and a source that names no
    /// population is a backend fault, as in the interpreter.
    #[trace("TC-1385")]
    #[test]
    fn an_only_over_other_than_one_object_is_the_populations_family_result() {
        use crate::translate::IterSpec;
        use quire_checked::node::Visit;
        use quire_semantic_value::location::{ExpressionLocation, ExpressionOwner};

        let scope = qsl_semantics::check::scope_with(Default::default(), Vec::new());
        let start = |population: Option<&str>, count: usize| {
            let op = Op {
                kind: OpKind::IterStart {
                    spec: IterSpec {
                        shape: Shape::Query(Visit::Only),
                        value_type: quire_exact::ValueType::Boolean,
                        only_over_filter: false,
                    },
                    population: population.map(str::to_owned),
                    identity_boolean: None,
                },
                location: ExpressionLocation::root(ExpressionOwner::Body {
                    function: "f".to_owned(),
                    index: 0,
                }),
                operands: [Repr::Handle, Repr::Register],
                result: Repr::Register,
            };
            let mut state = State::bounded(vec![booleans(count)], 4);
            let mut meter = Meter::new(unlimited());
            let ran = run(&op, [0, 0], &mut meter, &scope, &mut state);
            (ran, state.frames.len())
        };
        for count in [0, 2] {
            let (ran, frames) = start(Some("Account"), count);
            let Err(Halt::Family(result)) = ran else {
                panic!("a family result for {count} objects");
            };
            let expected = FamilyResult::Undefined(Box::new(StateModelUndefined::OnlyNotOne {
                population: "Account".to_owned(),
                found: count as u64,
            }));
            assert_eq!(format!("{result:?}"), format!("{expected:?}"));
            assert_eq!(frames, 0);
        }
        let (ran, frames) = start(Some("Account"), 1);
        assert!(matches!(ran, Ok(0)));
        assert_eq!(frames, 1);
        let (ran, _) = start(None, 2);
        assert!(matches!(
            ran,
            Err(Halt::Fault("only-source-is-a-population"))
        ));
    }
}
