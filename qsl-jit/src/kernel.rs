// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-561, FR-562: the kernel operation behind each shim.
//!
//! Each operation is the call the interpreter makes for the same checked
//! node (`qsl_eval` S6a), with the same arguments, so the meter sees the same
//! charges at the same points. A fast-path operation replaces no arithmetic
//! here: the kernel computes, and the register only carries the result.

use quire_exact::{
    evaluate_boolean, evaluate_integer_arithmetic, order_numbers, retain_boolean,
    BooleanConnective, Charge, ChargePoint, Integer, IntegerArithmetic, Meter, OrderedOperands,
    Refusal, Value,
};
use quire_semantic_value::declaration::TypeEnvironment;
use quire_semantic_value::stop::{outcome_into_stop, Stop};

use crate::translate::{divide, integer_to_i64, Op, OpKind, Repr};
use quire_checked::node::Arithmetic;

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

fn narrow(value: &Integer) -> Result<i64, Halt> {
    integer_to_i64(value).ok_or(Halt::Fault("jit-fast-path-result-exceeds-64-bits"))
}

/// The `Integer` an operand names: the register itself, or the arena value
/// its handle indexes.
fn integer(raw: i64, repr: Repr, arena: &[Value]) -> Result<Integer, Halt> {
    match repr {
        Repr::Register => Ok(Integer::from(raw)),
        Repr::Handle => match usize::try_from(raw).ok().and_then(|index| arena.get(index)) {
            Some(Value::Integer(value)) => Ok(value.clone()),
            _ => Err(Halt::Fault("jit-handle-not-an-integer")),
        },
    }
}

/// The register or handle that carries `value`: a register must hold it
/// (the translator proved so from the checked types), a handle is a new arena
/// entry.
fn carry(value: Integer, repr: Repr, arena: &mut Vec<Value>) -> Result<i64, Halt> {
    match repr {
        Repr::Register => narrow(&value),
        Repr::Handle => {
            let handle = i64::try_from(arena.len()).map_err(|_| Halt::Fault("jit-arena-index"))?;
            arena.push(Value::Integer(value));
            Ok(handle)
        }
    }
}

/// Run `op` on the operands `raw` under `meter`, reading and writing handles
/// in `arena`.
pub(crate) fn run(
    op: &Op,
    raw: [i64; 2],
    meter: &mut Meter,
    types: &TypeEnvironment,
    arena: &mut Vec<Value>,
) -> Result<i64, Halt> {
    let [a, b] = raw;
    let [left_repr, right_repr] = op.operands;
    match &op.kind {
        OpKind::Literal(value) => return carry(value.clone(), op.result, arena),
        OpKind::Box | OpKind::Unbox => {
            return carry(integer(a, left_repr, arena)?, op.result, arena)
        }
        _ => {}
    }
    let (left, right) = match &op.kind {
        OpKind::Not | OpKind::Retain | OpKind::ChargeCall => {
            (Integer::from(0_i64), Integer::from(0_i64))
        }
        OpKind::Equality {
            operands_boolean: true,
            ..
        } => (Integer::from(a), Integer::from(b)),
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
        OpKind::Literal(_) | OpKind::Box | OpKind::Unbox => Err(Halt::Fault("jit-unreachable")),
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
        OpKind::Order(operator) => {
            let holds = outcome_into_stop(order_numbers(
                *operator,
                OrderedOperands::Integers(&left, &right),
                meter,
            ))?;
            Ok(i64::from(holds))
        }
        OpKind::Equality {
            checked,
            operands_boolean,
        } => {
            let value = |register: i64, integer: &Integer| {
                if *operands_boolean {
                    Value::Boolean(register != 0)
                } else {
                    Value::Integer(integer.clone())
                }
            };
            let holds = outcome_into_stop(checked.evaluate(
                types,
                &value(a, &left),
                &value(b, &right),
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
    }
}
