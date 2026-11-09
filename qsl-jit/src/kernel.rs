// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-561, FR-562: the kernel operation behind each shim.
//!
//! Each operation is the call the interpreter makes for the same checked
//! node (`qsl_eval` S6a), with the same arguments, so the meter sees the same
//! charges at the same points. A fast-path operation replaces no arithmetic
//! here: the kernel computes, and the register only carries the result.

use quire_checked::node::{Arithmetic, OrderedKind};
use quire_exact::{
    compare_text, evaluate_boolean, evaluate_decimal, evaluate_integer_arithmetic,
    evaluate_rational_arithmetic, order_numbers, retain_boolean, ArithmeticOperator,
    BooleanConnective, Charge, ChargePoint, ComparisonOperator, Decimal, DecimalOperation, Integer,
    IntegerArithmetic, Meter, OrderedOperands, OrderingOperator, Rational, RationalArithmetic,
    Refusal, Value,
};
use quire_semantic_value::declaration::TypeEnvironment;
use quire_semantic_value::loss::{LocatedLoss, ValueLoss};
use quire_semantic_value::stop::{outcome_into_stop, Stop};

use crate::translate::{divide, integer_to_i64, Op, OpKind, Repr};

/// Why a kernel operation did not return a value.
pub(crate) enum Halt {
    /// The kernel's own stop (`Undefined`, `Refused` or `Incomplete`), passed
    /// up unchanged with the node's location.
    Stop(Stop),
    /// A backend invariant broke.
    Fault(&'static str),
}

impl From<Stop> for Halt {
    fn from(stop: Stop) -> Self {
        Self::Stop(stop)
    }
}

/// What a call keeps beside its registers: the values behind its handles
/// and the loss records its operations produced, in evaluation order.
#[derive(Default)]
pub(crate) struct State {
    pub(crate) arena: Vec<Value>,
    pub(crate) losses: Vec<LocatedLoss>,
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

/// Run `op` on the operands `raw` under `meter`, reading and writing handles
/// and loss records in `state`.
pub(crate) fn run(
    op: &Op,
    raw: [i64; 2],
    meter: &mut Meter,
    types: &TypeEnvironment,
    state: &mut State,
) -> Result<i64, Halt> {
    let [a, b] = raw;
    let State { arena, losses } = state;
    if let Some(done) = run_value(op, raw, meter, arena, losses) {
        return done;
    }
    let [left_repr, right_repr] = op.operands;
    let (left, right) = match &op.kind {
        OpKind::Not | OpKind::Retain | OpKind::ChargeCall => {
            (Integer::from(0_i64), Integer::from(0_i64))
        }
        OpKind::Equality {
            operands_boolean: true,
            ..
        } => (Integer::from(a), Integer::from(b)),
        OpKind::Equality { .. } if left_repr == Repr::Handle && right_repr == Repr::Handle => {
            (Integer::from(0_i64), Integer::from(0_i64))
        }
        _ => (
            integer(a, left_repr, arena)?,
            if matches!(op.kind, OpKind::Negate | OpKind::Coerce(_)) {
                Integer::from(0_i64)
            } else {
                integer(b, right_repr, arena)?
            },
        ),
    };
    match &op.kind {
        OpKind::Arithmetic(operator) => {
            let operation = match operator {
                Arithmetic::Add => IntegerArithmetic::Add(&left, &right),
                Arithmetic::Subtract => IntegerArithmetic::Subtract(&left, &right),
                Arithmetic::Multiply => IntegerArithmetic::Multiply(&left, &right),
            };
            let result = outcome_into_stop(evaluate_integer_arithmetic(operation, None, meter))?;
            carry(result, op.result, arena)
        }
        OpKind::Negate => {
            let result = outcome_into_stop(evaluate_integer_arithmetic(
                IntegerArithmetic::Negate(&left),
                None,
                meter,
            ))?;
            carry(result, op.result, arena)
        }
        OpKind::Divide {
            operator,
            selection,
        } => {
            let outcome = divide(*operator, selection.as_ref(), &left, &right, meter)
                .ok_or(Halt::Fault("jit-division-selection-missing"))?;
            carry(outcome_into_stop(outcome)?, op.result, arena)
        }
        OpKind::Equality {
            checked,
            operands_boolean,
        } => {
            let value = |raw: i64, repr: Repr, integer: &Integer| -> Result<Value, Halt> {
                if *operands_boolean {
                    Ok(Value::Boolean(raw != 0))
                } else if repr == Repr::Handle {
                    handle(raw, arena).cloned()
                } else {
                    Ok(Value::Integer(integer.clone()))
                }
            };
            let holds = outcome_into_stop(checked.evaluate(
                types,
                &value(a, left_repr, &left)?,
                &value(b, right_repr, &right)?,
                meter,
            ))?;
            Ok(i64::from(holds))
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
        OpKind::Coerce(target) => {
            if target.contains(&left) {
                carry(left, op.result, arena)
            } else {
                Err(Halt::Stop(Stop::Refused(Refusal::IntegerOutOfDomain {
                    target: Box::new(target.clone()),
                })))
            }
        }
        OpKind::Order(..)
        | OpKind::Literal(_)
        | OpKind::Box
        | OpKind::Unbox
        | OpKind::RationalDivide { .. }
        | OpKind::Rational { .. }
        | OpKind::RationalNegate { .. }
        | OpKind::Decimal { .. }
        | OpKind::DecimalNegate { .. }
        | OpKind::ConvertDecimal { .. } => Err(Halt::Fault("jit-operation-dispatch")),
    }
}

/// The operations over handles and the orderings, which read their operands
/// as values; `None` for an operation over `Integer` registers.
fn run_value(
    op: &Op,
    [a, b]: [i64; 2],
    meter: &mut Meter,
    arena: &mut Vec<Value>,
    losses: &mut Vec<LocatedLoss>,
) -> Option<Result<i64, Halt>> {
    let [left_repr, right_repr] = op.operands;
    let produced: Result<Value, Halt> = match &op.kind {
        OpKind::Literal(value) => Ok(value.clone()),
        OpKind::Box | OpKind::Unbox => {
            return Some(integer(a, left_repr, arena).and_then(|v| carry(v, op.result, arena)))
        }
        OpKind::Order(kind) => {
            return Some(order(*kind, [a, b], [left_repr, right_repr], meter, arena).map(i64::from))
        }
        OpKind::RationalDivide { domain } => (|| {
            let left = Rational::from_integer(integer(a, left_repr, arena)?);
            let right = Rational::from_integer(integer(b, right_repr, arena)?);
            Ok(Value::Rational(outcome_into_stop(
                evaluate_rational_arithmetic(
                    RationalArithmetic::Divide(&left, &right),
                    Some(domain),
                    meter,
                ),
            )?))
        })(),
        OpKind::Rational { operator, domain } => (|| {
            let (left, right) = (rational(a, arena)?, rational(b, arena)?);
            let operation = match operator {
                ArithmeticOperator::Add => RationalArithmetic::Add(left, right),
                ArithmeticOperator::Subtract => RationalArithmetic::Subtract(left, right),
                ArithmeticOperator::Multiply => RationalArithmetic::Multiply(left, right),
                ArithmeticOperator::Divide => RationalArithmetic::Divide(left, right),
            };
            Ok(Value::Rational(outcome_into_stop(
                evaluate_rational_arithmetic(operation, Some(domain), meter),
            )?))
        })(),
        OpKind::RationalNegate { domain } => (|| {
            let operand = rational(a, arena)?;
            Ok(Value::Rational(outcome_into_stop(
                evaluate_rational_arithmetic(
                    RationalArithmetic::Negate(operand),
                    Some(domain),
                    meter,
                ),
            )?))
        })(),
        OpKind::Decimal { operator, target } => (|| {
            let (left, right) = (decimal(a, arena)?, decimal(b, arena)?);
            let operation = match operator {
                ArithmeticOperator::Add => DecimalOperation::Add(left, right),
                ArithmeticOperator::Subtract => DecimalOperation::Subtract(left, right),
                ArithmeticOperator::Multiply => DecimalOperation::Multiply(left, right),
                ArithmeticOperator::Divide => DecimalOperation::Divide(left, right),
            };
            decimal_operation(op, operation, target, meter, losses)
        })(),
        OpKind::DecimalNegate { target } => (|| {
            let operand = decimal(a, arena)?;
            decimal_operation(op, DecimalOperation::Negate(operand), target, meter, losses)
        })(),
        OpKind::ConvertDecimal { target } => (|| match handle(a, arena)? {
            Value::Decimal(source) => {
                decimal_operation(op, DecimalOperation::Round(source), target, meter, losses)
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
                )
            }
            _ => Err(Halt::Fault("jit-decimal-conversion-operand-invalid")),
        })(),
        _ => return None,
    };
    Some(produced.and_then(|value| push(value, arena)))
}
