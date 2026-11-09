// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-561, FR-562: the kernel operation behind each shim.
//!
//! Each operation is the call the interpreter makes for the same checked
//! node (`qsl_eval` S6a), with the same arguments, so the meter sees the same
//! charges at the same points. A fast-path operation replaces no arithmetic
//! here: the kernel computes, and the register only carries the result.

use quire_exact::{
    evaluate_integer_arithmetic, order_numbers, Integer, IntegerArithmetic, Meter, OrderedOperands,
    Refusal,
};
use quire_semantic_value::stop::{outcome_into_stop, Stop};

use crate::translate::{divide, integer_to_i64, Op, OpKind};
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

/// Run `op` on the operands `a` and `b` under `meter`.
pub(crate) fn run(op: &Op, a: i64, b: i64, meter: &mut Meter) -> Result<i64, Halt> {
    let (left, right) = (Integer::from(a), Integer::from(b));
    match &op.kind {
        OpKind::Arithmetic(operator) => {
            let operation = match operator {
                Arithmetic::Add => IntegerArithmetic::Add(&left, &right),
                Arithmetic::Subtract => IntegerArithmetic::Subtract(&left, &right),
                Arithmetic::Multiply => IntegerArithmetic::Multiply(&left, &right),
            };
            let result = outcome_into_stop(evaluate_integer_arithmetic(operation, None, meter))?;
            narrow(&result)
        }
        OpKind::Negate => {
            let result = outcome_into_stop(evaluate_integer_arithmetic(
                IntegerArithmetic::Negate(&left),
                None,
                meter,
            ))?;
            narrow(&result)
        }
        OpKind::Divide {
            operator,
            selection,
        } => {
            let outcome = divide(*operator, selection.as_ref(), &left, &right, meter)
                .ok_or(Halt::Fault("jit-division-selection-missing"))?;
            narrow(&outcome_into_stop(outcome)?)
        }
        OpKind::Order(operator) => {
            let holds = outcome_into_stop(order_numbers(
                *operator,
                OrderedOperands::Integers(&left, &right),
                meter,
            ))?;
            Ok(i64::from(holds))
        }
        OpKind::Coerce(target) => {
            if target.contains(&left) {
                Ok(a)
            } else {
                Err(Halt::Stop(Stop::Refused(Refusal::IntegerOutOfDomain {
                    target: Box::new(target.clone()),
                })))
            }
        }
    }
}
