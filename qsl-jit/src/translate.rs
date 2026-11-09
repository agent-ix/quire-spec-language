// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-560, FR-561, FR-562: the translator from the checked representation of
//! one function to a Cranelift function.
//!
//! Every `Integer` operation is a call to the kernel shim of [`crate::exec`],
//! which runs the operation the interpreter runs with the interpreter's
//! charges; the translator only decides, from checked types at `prepare`
//! (FR-562), that the result fits a signed 64-bit machine integer, so the
//! value can be carried in a register. The decision never depends on a
//! run-time value.

use std::collections::HashMap;

use cranelift_codegen::ir::{types, AbiParam, Block, InstBuilder, MemFlagsData, Value as Clif};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::JITModule;
use cranelift_module::{FuncId, Module};
use qsl_eval::execution::{PrepareFailure, PrepareMeter};
use qsl_semantics::check::CheckedNode;
use quire_checked::integer_division::AdmittedIntegerDivision;
use quire_checked::node::{Arithmetic, IntegerDivisionOperator, NodeKind, OrderedKind};
use quire_exact::{
    DivisionMember, Integer, IntegerDomain, IntegerInterval, Meter, OrderingOperator, Outcome,
    ScalarLimits, Value, ValueType,
};
use quire_semantic_value::location::ExpressionLocation;

use crate::classify::{untranslated, Outside};
use crate::exec::Shim;

/// Deepest expression this backend translates. Translation recurses on the
/// host stack; a deeper tree is listed unsupported and the interpreter, which
/// evaluates without host recursion, runs it.
const MAX_DEPTH: usize = 192;

/// What one shim call computes.
pub(crate) enum OpKind {
    Arithmetic(Arithmetic),
    Negate,
    Divide {
        operator: IntegerDivisionOperator,
        selection: Option<AdmittedIntegerDivision>,
    },
    Order(OrderingOperator),
    Coerce(IntegerInterval),
}

/// One kernel operation of a translated function and where it is.
pub(crate) struct Op {
    pub(crate) kind: OpKind,
    pub(crate) location: ExpressionLocation,
}

/// The register representation of a checked value: a Boolean (`0` or `1`) or
/// an `Integer` with the interval FR-562 derives for it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Ty {
    Boolean,
    Int(i128, i128),
}

/// What a function was translated to.
pub(crate) struct Translated {
    pub(crate) id: FuncId,
    pub(crate) ops: Vec<Op>,
    pub(crate) result_is_boolean: bool,
}

/// Why a function was not translated.
pub(crate) enum Failure {
    /// The function holds a construct this backend does not translate.
    Unsupported(Outside),
    /// `prepare` stops (a limit, the handle, or a backend invariant).
    Prepare(PrepareFailure),
}

impl From<PrepareFailure> for Failure {
    fn from(failure: PrepareFailure) -> Self {
        Self::Prepare(failure)
    }
}

fn fault(invariant: &'static str) -> Failure {
    Failure::Prepare(PrepareFailure::Fault(
        quire_semantic_value::catalog::InternalFault::new("FR-560", invariant),
    ))
}

/// The interval of a declared parameter or result type, when it is a type
/// this backend carries in a register.
fn type_of(value_type: &ValueType) -> Option<Ty> {
    match value_type {
        ValueType::Boolean => Some(Ty::Boolean),
        ValueType::Int(interval) => {
            let (lo, hi) = (
                integer_to_i64(interval.lower())?,
                integer_to_i64(interval.upper())?,
            );
            Some(Ty::Int(lo.into(), hi.into()))
        }
        _ => None,
    }
}

/// `value` as a signed 64-bit integer, when it fits.
pub(crate) fn integer_to_i64(value: &Integer) -> Option<i64> {
    let magnitude = if value.is_negative() {
        value.neg().to_u64()?
    } else {
        value.to_u64()?
    };
    let wide = i128::from(magnitude);
    i64::try_from(if value.is_negative() { -wide } else { wide }).ok()
}

fn fits(low: i128, high: i128) -> bool {
    i128::from(i64::MIN) <= low && high <= i128::from(i64::MAX)
}

struct Emitter<'a, 'b, 'm> {
    builder: FunctionBuilder<'b>,
    module: &'a mut JITModule,
    shim: FuncId,
    context: Clif,
    exit: Block,
    ops: Vec<Op>,
    slots: HashMap<usize, (Clif, Ty)>,
    parameters: Vec<Option<Ty>>,
    meter: &'a mut PrepareMeter<'m>,
}

/// Translate `body`, the checked body of the function `name` with the declared
/// `parameters`, into a function of `module`.
pub(crate) fn translate(
    module: &mut JITModule,
    shim: &Shim,
    name: &str,
    parameters: &[(String, ValueType)],
    body: CheckedNode<'_>,
    meter: &mut PrepareMeter<'_>,
) -> Result<Translated, Failure> {
    let pointer = module.target_config().pointer_type();
    let mut signature = module.make_signature();
    signature.params.push(AbiParam::new(pointer));
    signature.params.push(AbiParam::new(pointer));
    signature.returns.push(AbiParam::new(types::I64));
    let id = module
        .declare_function(
            &format!("qsl_{name}"),
            cranelift_module::Linkage::Export,
            &signature,
        )
        .map_err(|_| fault("jit-declare-function"))?;
    let frontend = module.target_config();
    let mut context = module.make_context();
    context.func.signature = signature;
    let mut builder_context = FunctionBuilderContext::new();
    let result_ty = {
        let mut builder = FunctionBuilder::new(&mut context.func, &mut builder_context);
        let entry = builder.create_block();
        builder.append_block_params_for_function_params(entry);
        builder.switch_to_block(entry);
        let (ctx_value, args) = (
            builder.block_params(entry)[0],
            builder.block_params(entry)[1],
        );
        let exit = builder.create_block();
        let mut emitter = Emitter {
            builder,
            module,
            shim: shim.id(),
            context: ctx_value,
            exit,
            ops: Vec::new(),
            slots: HashMap::new(),
            parameters: parameters.iter().map(|(_, t)| type_of(t)).collect(),
            meter,
        };
        for (index, ty) in emitter.parameters.clone().into_iter().enumerate() {
            if let Some(ty) = ty {
                let offset = i32::try_from(index * 8).map_err(|_| fault("jit-parameter-offset"))?;
                let value =
                    emitter
                        .builder
                        .ins()
                        .load(types::I64, MemFlagsData::trusted(), args, offset);
                emitter.slots.insert(index, (value, ty));
            }
        }
        let (result, ty) = emitter.emit(body, 0)?;
        emitter.builder.ins().return_(&[result]);
        emitter.builder.switch_to_block(exit);
        let zero = emitter.builder.ins().iconst(types::I64, 0);
        emitter.builder.ins().return_(&[zero]);
        emitter.builder.seal_all_blocks();
        let ops = std::mem::take(&mut emitter.ops);
        emitter.builder.finalize(frontend);
        (ty, ops)
    };
    let (ty, ops) = result_ty;
    module
        .define_function(id, &mut context)
        .map_err(|_| fault("jit-define-function"))?;
    let code_bytes = context.compiled_code().map_or(0, |code| {
        u64::try_from(code.code_buffer().len()).unwrap_or(u64::MAX)
    });
    module.clear_context(&mut context);
    meter.charge_block(code_bytes)?;
    Ok(Translated {
        id,
        ops,
        result_is_boolean: ty == Ty::Boolean,
    })
}

impl Emitter<'_, '_, '_> {
    fn unsupported(&self, node: CheckedNode<'_>) -> Failure {
        Failure::Unsupported(untranslated(node))
    }

    fn emit(&mut self, node: CheckedNode<'_>, depth: usize) -> Result<(Clif, Ty), Failure> {
        if depth > MAX_DEPTH {
            return Err(self.unsupported(node));
        }
        self.meter.charge_node()?;
        match node.kind() {
            NodeKind::Literal(literal) => match &literal.0 {
                Value::Boolean(value) => {
                    let v = self.builder.ins().iconst(types::I64, i64::from(*value));
                    Ok((v, Ty::Boolean))
                }
                Value::Integer(value) => {
                    let value = integer_to_i64(value).ok_or_else(|| self.unsupported(node))?;
                    let v = self.builder.ins().iconst(types::I64, value);
                    Ok((v, Ty::Int(value.into(), value.into())))
                }
                _ => Err(self.unsupported(node)),
            },
            NodeKind::Local(slot) => self
                .slots
                .get(slot)
                .copied()
                .ok_or_else(|| self.unsupported(node)),
            NodeKind::Let { slot, value, body } => {
                let bound = self.emit(node.at(*value), depth + 1)?;
                let previous = self.slots.insert(*slot, bound);
                let result = self.emit(node.at(*body), depth + 1);
                match previous {
                    Some(previous) => self.slots.insert(*slot, previous),
                    None => self.slots.remove(slot),
                };
                result
            }
            NodeKind::If {
                condition,
                then,
                otherwise,
            } => self.emit_if(node, (*condition, *then, *otherwise), depth),
            NodeKind::Arithmetic(operator, left, right) => {
                let (a, left_ty) = self.emit(node.at(*left), depth + 1)?;
                let (b, right_ty) = self.emit(node.at(*right), depth + 1)?;
                let (Ty::Int(al, ah), Ty::Int(bl, bh)) = (left_ty, right_ty) else {
                    return Err(self.unsupported(node));
                };
                let (low, high) = match operator {
                    Arithmetic::Add => (al + bl, ah + bh),
                    Arithmetic::Subtract => (al - bh, ah - bl),
                    Arithmetic::Multiply => {
                        let products = [al * bl, al * bh, ah * bl, ah * bh];
                        (
                            products.into_iter().min().unwrap_or(0),
                            products.into_iter().max().unwrap_or(0),
                        )
                    }
                };
                if !fits(low, high) {
                    return Err(self.unsupported(node));
                }
                let result = self.shim_call(node, OpKind::Arithmetic(*operator), a, b);
                Ok((result, Ty::Int(low, high)))
            }
            NodeKind::Negate(operand) => {
                let (a, ty) = self.emit(node.at(*operand), depth + 1)?;
                let Ty::Int(low, high) = ty else {
                    return Err(self.unsupported(node));
                };
                if !fits(-high, -low) {
                    return Err(self.unsupported(node));
                }
                let zero = self.builder.ins().iconst(types::I64, 0);
                let result = self.shim_call(node, OpKind::Negate, a, zero);
                Ok((result, Ty::Int(-high, -low)))
            }
            NodeKind::IntegerDivision {
                operator,
                selection,
                left,
                right,
            } => {
                let (a, left_ty) = self.emit(node.at(*left), depth + 1)?;
                let (b, right_ty) = self.emit(node.at(*right), depth + 1)?;
                let (Ty::Int(al, ah), Ty::Int(bl, bh)) = (left_ty, right_ty) else {
                    return Err(self.unsupported(node));
                };
                // FR-562: the divisor's interval must exclude 0, and the
                // result interval is the smallest holding the kernel's result
                // at every pair of operand interval ends.
                if bl <= 0 && 0 <= bh {
                    return Err(self.unsupported(node));
                }
                let (low, high) = division_interval(*operator, *selection, (al, ah), (bl, bh))
                    .ok_or_else(|| self.unsupported(node))?;
                if !fits(low, high) {
                    return Err(self.unsupported(node));
                }
                let kind = OpKind::Divide {
                    operator: *operator,
                    selection: *selection,
                };
                let result = self.shim_call(node, kind, a, b);
                Ok((result, Ty::Int(low, high)))
            }
            NodeKind::Order(operator, OrderedKind::Integers, left, right) => {
                let (a, left_ty) = self.emit(node.at(*left), depth + 1)?;
                let (b, right_ty) = self.emit(node.at(*right), depth + 1)?;
                if !matches!((left_ty, right_ty), (Ty::Int(..), Ty::Int(..))) {
                    return Err(self.unsupported(node));
                }
                let result = self.shim_call(node, OpKind::Order(*operator), a, b);
                Ok((result, Ty::Boolean))
            }
            NodeKind::Coerce(operand, target) => {
                let (a, ty) = self.emit(node.at(*operand), depth + 1)?;
                let Ty::Int(low, high) = ty else {
                    return Err(self.unsupported(node));
                };
                // The result lies in the operand's interval and the target's;
                // an empty intersection always refuses, so keep the operand's.
                let (target_low, target_high) = (
                    clamp(target.lower(), low, high),
                    clamp(target.upper(), low, high),
                );
                let zero = self.builder.ins().iconst(types::I64, 0);
                let result = self.shim_call(node, OpKind::Coerce(target.clone()), a, zero);
                let narrowed = if target_low <= target_high {
                    Ty::Int(target_low, target_high)
                } else {
                    ty
                };
                Ok((result, narrowed))
            }
            _ => Err(self.unsupported(node)),
        }
    }

    fn emit_if(
        &mut self,
        node: CheckedNode<'_>,
        (condition, then, otherwise): (
            qsl_semantics::check::NodeId,
            qsl_semantics::check::NodeId,
            qsl_semantics::check::NodeId,
        ),
        depth: usize,
    ) -> Result<(Clif, Ty), Failure> {
        let (selector, condition_ty) = self.emit(node.at(condition), depth + 1)?;
        if condition_ty != Ty::Boolean {
            return Err(self.unsupported(node));
        }
        let (then_block, else_block, merge) = (
            self.builder.create_block(),
            self.builder.create_block(),
            self.builder.create_block(),
        );
        self.builder.append_block_param(merge, types::I64);
        self.builder
            .ins()
            .brif(selector, then_block, &[], else_block, &[]);
        self.builder.switch_to_block(then_block);
        let (taken, then_ty) = self.emit(node.at(then), depth + 1)?;
        self.builder.ins().jump(merge, &[taken.into()]);
        self.builder.switch_to_block(else_block);
        let (other, else_ty) = self.emit(node.at(otherwise), depth + 1)?;
        self.builder.ins().jump(merge, &[other.into()]);
        self.builder.switch_to_block(merge);
        let joined = self.builder.block_params(merge)[0];
        let ty = match (then_ty, else_ty) {
            (Ty::Boolean, Ty::Boolean) => Ty::Boolean,
            (Ty::Int(a, b), Ty::Int(c, d)) => Ty::Int(a.min(c), b.max(d)),
            _ => return Err(self.unsupported(node)),
        };
        Ok((joined, ty))
    }

    /// Call the kernel shim for `kind` at `node`, then leave through `exit`
    /// when the shim recorded a stop (FR-561: the status is passed up
    /// unchanged).
    fn shim_call(&mut self, node: CheckedNode<'_>, kind: OpKind, a: Clif, b: Clif) -> Clif {
        let index = i64::try_from(self.ops.len()).unwrap_or(i64::MAX);
        self.ops.push(Op {
            kind,
            location: node.location().clone(),
        });
        let function = self
            .module
            .declare_func_in_func(self.shim, self.builder.func);
        let index = self.builder.ins().iconst(types::I64, index);
        let call = self
            .builder
            .ins()
            .call(function, &[self.context, index, a, b]);
        let result = self.builder.inst_results(call)[0];
        let failed = self
            .builder
            .ins()
            .load(types::I64, MemFlagsData::trusted(), self.context, 0);
        let next = self.builder.create_block();
        self.builder.ins().brif(failed, self.exit, &[], next, &[]);
        self.builder.switch_to_block(next);
        result
    }
}

/// `bound` clamped into `[low, high]`, as an `i128`.
fn clamp(bound: &Integer, low: i128, high: i128) -> i128 {
    let negative = bound.is_negative();
    let wide = match bound.abs().to_u64() {
        Some(magnitude) if negative => -i128::from(magnitude),
        Some(magnitude) => i128::from(magnitude),
        None if negative => i128::MIN / 4,
        None => i128::MAX / 4,
    };
    wide.clamp(low, high)
}

/// The smallest interval holding the kernel's `div`, `rem` or `mod` at every
/// pair of operand interval ends (FR-562), computed by running the kernel on
/// each pair with an unlimited meter.
fn division_interval(
    operator: IntegerDivisionOperator,
    selection: Option<AdmittedIntegerDivision>,
    (al, ah): (i128, i128),
    (bl, bh): (i128, i128),
) -> Option<(i128, i128)> {
    let mut meter = Meter::new(ScalarLimits {
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
    });
    let mut low = i128::MAX;
    let mut high = i128::MIN;
    for dividend in [al, ah] {
        for divisor in [bl, bh] {
            let result = divide(
                operator,
                selection.as_ref(),
                &Integer::from(dividend),
                &Integer::from(divisor),
                &mut meter,
            )?;
            let Outcome::Completed(result) = result else {
                return None;
            };
            let value = integer_to_i64(&result)?;
            low = low.min(value.into());
            high = high.max(value.into());
        }
    }
    Some((low, high))
}

/// The kernel call the interpreter makes for `operator` (S6a,
/// `IntegerDivision`), or `None` when `div` or `rem` has no admitted
/// selection.
pub(crate) fn divide(
    operator: IntegerDivisionOperator,
    selection: Option<&AdmittedIntegerDivision>,
    dividend: &Integer,
    divisor: &Integer,
    meter: &mut Meter,
) -> Option<Outcome<Integer>> {
    let member = match operator {
        IntegerDivisionOperator::Mod => {
            return Some(quire_eval::modulo(
                dividend,
                divisor,
                &IntegerDomain::Mathematical,
                meter,
            ))
        }
        IntegerDivisionOperator::Div => DivisionMember::Quotient,
        IntegerDivisionOperator::Rem => DivisionMember::Remainder,
    };
    Some(qsl_semantics::value::divide(
        selection?,
        member,
        dividend,
        divisor,
        &IntegerDomain::Mathematical,
        meter,
    ))
}
