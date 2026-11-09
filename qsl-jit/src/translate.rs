// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-560, FR-561, FR-562: the translator from the checked representation of
//! one function to a Cranelift function.
//!
//! Every `Integer` operation is a call to the kernel shim of [`crate::exec`],
//! which runs the operation the interpreter runs with the interpreter's
//! charges; the translator only decides, from checked types at `prepare`
//! (FR-562), whether the result fits a signed 64-bit machine integer, so the
//! value can be carried in a register. The decision never depends on a
//! run-time value.
//!
//! A value the checked bounds do not put in a register is carried as a handle
//! (QSL-1825): an index into the arena the call owns, which holds the
//! unbounded `Integer` the kernel returned. The shim reads its operands from
//! the arena and runs the same kernel entry on them, so the handle path
//! changes where a value lives and nothing the meter sees.

use std::collections::HashMap;

use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    types, AbiParam, Block, InstBuilder, MemFlagsData, StackSlotData, StackSlotKind, Value as Clif,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::JITModule;
use cranelift_module::{FuncId, Module};
use qsl_eval::execution::{PrepareFailure, PrepareMeter};
use qsl_eval::value::collections::{is_exact_sum, Shape, Spec};
use qsl_semantics::check::{CheckedGraph, CheckedNode};
use quire_checked::integer_division::AdmittedIntegerDivision;
use quire_checked::node::{Arithmetic, Connective, IntegerDivisionOperator, NodeKind, OrderedKind};
use quire_checked::node::{RecordField, RecordSlot, Visit};
use quire_exact::{
    ArithmeticOperator, CollectionType, DecimalType, DivisionMember, DivisionProfile, Integer,
    IntegerDomain, IntegerInterval, Meter, NativeIntegerInterval, NativeIntegerOperation,
    NativeIntegerPlan, NodeKey, OrderingOperator, Outcome, RationalDomain, ScalarLimits, Value,
    ValueType,
};
use quire_semantic_value::declaration::EqualityOperator;
use quire_semantic_value::declaration::{CheckedEquality, EqualityOperand};
use quire_semantic_value::location::ExpressionLocation;

use crate::exec::{Shim, Signature, ARGUMENTS_OFFSET, FAILED_OFFSET, RESULT_OFFSET};
use qsl_eval::execution::{untranslated, Outside};

/// Deepest expression this backend translates. Translation recurses on the
/// host stack; a deeper tree is listed unsupported and the interpreter, which
/// evaluates without host recursion, runs it.
const MAX_DEPTH: usize = 192;

/// What a query or fold iterates, in the form the shared iteration reads
/// ([`Spec`]), owned by the operation table.
#[derive(Clone)]
pub(crate) struct IterSpec {
    pub(crate) shape: Shape,
    pub(crate) value_type: ValueType,
    pub(crate) only_over_filter: bool,
}

impl IterSpec {
    /// The shared iteration's view of this spec.
    pub(crate) fn of(&self) -> Spec<'_> {
        Spec {
            shape: self.shape,
            value_type: &self.value_type,
            only_over_filter: self.only_over_filter,
        }
    }
}

/// The charge schedule of a fast-path `Integer` operation: the kernel's
/// charge-only entry (FR-562), run on the operands the machine operation
/// then computes.
#[derive(Clone, Copy)]
pub(crate) enum NativeCharge {
    /// `+`, binary `-` or unary `-`, over the plan its operand intervals give.
    Arithmetic(NativeIntegerPlan, bool),
    /// `*`.
    Multiply,
    /// `div` or `rem`.
    Division,
    /// `mod`.
    Modulus,
    /// `<`, `<=`, `>` or `>=`.
    Ordering,
    /// `=` or `!=`.
    Equality,
}

/// What one shim call computes.
pub(crate) enum OpKind {
    /// The charges of the fast-path operation whose arithmetic the generated
    /// code computes itself; no kernel value operation runs.
    ChargeNative(NativeCharge),
    Arithmetic(Arithmetic),
    Negate,
    Divide {
        operator: IntegerDivisionOperator,
        selection: Option<AdmittedIntegerDivision>,
    },
    Order((OrderingOperator, OrderedKind)),
    Coerce(IntegerInterval),
    /// An integer widening `ConvertScalar`: the interpreter's `operand_value`.
    ConvertScalar(Box<EqualityOperand>),
    /// `=` or `!=` (the checked equality), over operands carried as
    /// Booleans (`true`) or Integers (`false`).
    Equality {
        checked: Box<CheckedEquality>,
        operands_boolean: bool,
    },
    Not,
    /// The `boolean.result-retain` charge of a connective's result.
    Retain,
    /// The `function.call` charge of a call.
    ChargeCall,
    /// A literal that does not fit a register (an `Integer` wider than 64
    /// bits, a `Rational`, `Decimal` or `Text`): the handle of a new arena
    /// entry. Charges nothing, as the interpreter's literal does.
    Literal(Value),
    /// `Integer / Integer` into a `Rational`.
    RationalDivide {
        domain: RationalDomain,
    },
    /// `Rational` arithmetic.
    Rational {
        operator: ArithmeticOperator,
        domain: RationalDomain,
    },
    RationalNegate {
        domain: RationalDomain,
    },
    /// `Decimal` arithmetic into `target`, recording the operation's loss.
    Decimal {
        operator: ArithmeticOperator,
        target: DecimalType,
    },
    DecimalNegate {
        target: DecimalType,
    },
    /// A `Decimal` or `Rational` converted into `target`.
    ConvertDecimal {
        target: DecimalType,
    },
    /// An operand value staged for the construction that follows (`true`: a
    /// `Boolean` register). Charges nothing.
    Stage {
        boolean: bool,
    },
    /// A record over the staged values of its present slots.
    Record {
        declaration: NodeKey,
        slots: Vec<RecordSlot<()>>,
    },
    /// A tuple over its `arity` staged values.
    Tuple {
        declaration: NodeKey,
        arity: usize,
    },
    /// A record field read, carried as `into`.
    Field {
        field: RecordField,
        optional: bool,
        value_type: ValueType,
        into: Carried,
    },
    /// The `collection.element` charge before a literal element.
    ChargeElement,
    /// A collection over its `count` staged elements.
    Collection {
        collection_type: CollectionType,
        count: usize,
    },
    Size,
    /// `contains`: the collection is the left operand, the item the right
    /// (`true`: a `Boolean` register).
    Contains {
        item_boolean: bool,
    },
    Flatten {
        result_type: ValueType,
    },
    ConvertCollection {
        target: CollectionType,
    },
    DropFirst,
    RemoveOne {
        item_boolean: bool,
    },
    /// Start a query or fold over the collection on the left; the right is its
    /// identity (`Some`: a `Boolean` register when `true`).
    IterStart {
        spec: IterSpec,
        /// The population an `only` ranges over, for its family result.
        population: Option<String>,
        identity_boolean: Option<bool>,
    },
    /// Cut the arena back to the loop's start and visit the next element:
    /// `1` when there is one.
    IterNext {
        spec: IterSpec,
    },
    /// The element just visited.
    IterElement,
    /// A fold's accumulator, bound to its slot before the step.
    IterAccumulator,
    /// Absorb the body result on the left.
    IterAbsorb {
        spec: IterSpec,
        /// Where a `sum` seed outside its domain is located.
        body: ExpressionLocation,
        result_boolean: bool,
    },
    /// Finish the query or fold: its result.
    IterFinish {
        spec: IterSpec,
    },
    /// The arena's length, taken before a call whose callee adds handles.
    Mark,
    /// Cut the arena back to a mark, keeping the callee's result.
    Reclaim,
    /// A register `Integer` moved to the arena, to meet a handle.
    Box,
    /// A handle whose value fits a register, moved back to one.
    Unbox,
}

/// Where a shim operand or result lives.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Repr {
    /// The 64-bit register value itself.
    Register,
    /// An index into the call's arena.
    Handle,
}

/// One kernel operation of a translated function and where it is.
pub(crate) struct Op {
    pub(crate) kind: OpKind,
    pub(crate) location: ExpressionLocation,
    /// Where the left and right operands live.
    pub(crate) operands: [Repr; 2],
    /// Where the result lives.
    pub(crate) result: Repr,
}

/// How a parameter or result is carried across the entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Carried {
    /// A `Boolean` register (`0` or `1`).
    Boolean,
    /// An `Integer` in a register.
    Register,
    /// An `Integer` behind an arena handle.
    Handle,
}

/// The representation of a checked value: a Boolean (`0` or `1`), an
/// `Integer` in a register with the interval FR-562 derives for it, or an
/// `Integer` behind a handle (`Wide`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Ty {
    Boolean,
    Int(i128, i128),
    Wide,
    /// A `Rational`, `Decimal` or `Text` behind a handle.
    Rational,
    Decimal,
    Text,
    /// A record or tuple behind a handle.
    Record,
    /// A collection behind a handle.
    Collection,
}

impl Ty {
    fn carried(self) -> Carried {
        match self {
            Self::Boolean => Carried::Boolean,
            Self::Int(..) => Carried::Register,
            Self::Wide
            | Self::Rational
            | Self::Decimal
            | Self::Text
            | Self::Record
            | Self::Collection => Carried::Handle,
        }
    }

    fn is_integer(self) -> bool {
        matches!(self, Self::Int(..) | Self::Wide)
    }

    fn repr(self) -> Repr {
        match self.carried() {
            Carried::Handle => Repr::Handle,
            Carried::Boolean | Carried::Register => Repr::Register,
        }
    }
}

/// What an entry was translated to.
pub(crate) struct Translated {
    /// The `extern "C" fn(*const EntryFrame) -> u32` trampoline of the entry.
    pub(crate) id: FuncId,
    pub(crate) result: Carried,
    /// The signature the translator declared for the trampoline.
    pub(crate) signature: Signature,
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
            match (
                integer_to_i64(interval.lower()),
                integer_to_i64(interval.upper()),
            ) {
                (Some(lo), Some(hi)) => Some(Ty::Int(lo.into(), hi.into())),
                _ => Some(Ty::Wide),
            }
        }
        ValueType::Integer => Some(Ty::Wide),
        ValueType::Rational(_) => Some(Ty::Rational),
        ValueType::Decimal(_) => Some(Ty::Decimal),
        ValueType::Text(_) => Some(Ty::Text),
        ValueType::Composite(_) => Some(Ty::Record),
        ValueType::Collection(_) => Some(Ty::Collection),
        _ => None,
    }
}

/// How a parameter of `value_type` is carried; `None` for a type this backend
/// does not carry (an unread parameter of one is not an error).
pub(crate) fn carried(value_type: &ValueType) -> Option<Carried> {
    type_of(value_type).map(Ty::carried)
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

enum State {
    InProgress,
    /// The function, its result, its parameters and whether it (or a callee)
    /// adds handles to the arena.
    Done(FuncId, Ty, Vec<Option<Ty>>, bool),
    Unsupported,
}

/// The translator of one prepared value: the functions it has translated, the
/// operation table their shim calls index, and the prepare meter.
pub(crate) struct Compiler<'a, 'm> {
    module: &'a mut JITModule,
    shim: FuncId,
    graph: &'a CheckedGraph,
    coverage: &'a qsl_eval::execution::BackendCoverage,
    ops: &'a mut Vec<Op>,
    meter: &'a mut PrepareMeter<'m>,
    state: HashMap<usize, State>,
}

struct Emitter<'a, 'b, 'm> {
    builder: FunctionBuilder<'b>,
    module: &'a mut JITModule,
    shim: FuncId,
    context: Clif,
    exit: Block,
    ops: &'a mut Vec<Op>,
    state: &'a HashMap<usize, State>,
    slots: HashMap<usize, (Clif, Ty)>,
    meter: &'a mut PrepareMeter<'m>,
    /// A backend invariant a code-generation step could not uphold.
    refused: Option<&'static str>,
    /// Whether this function adds handles to the call's arena (itself or
    /// through a callee).
    allocates: bool,
}

impl<'a, 'm> Compiler<'a, 'm> {
    pub(crate) fn new(
        module: &'a mut JITModule,
        shim: &Shim,
        graph: &'a CheckedGraph,
        coverage: &'a qsl_eval::execution::BackendCoverage,
        ops: &'a mut Vec<Op>,
        meter: &'a mut PrepareMeter<'m>,
    ) -> Self {
        Self {
            module,
            shim: shim.id(),
            graph,
            coverage,
            ops,
            meter,
            state: HashMap::new(),
        }
    }

    /// The index of the function whose identity is `identity`.
    pub(crate) fn index_of(&self, identity: &quire_exact::NodeKey) -> Option<usize> {
        (0..)
            .map_while(|index| self.graph.function_identity_at(index))
            .position(|candidate| candidate == *identity)
    }

    /// Translate the function at `index` and its callees, then the entry
    /// trampoline that adapts it to the one entry signature (FR-568).
    pub(crate) fn entry(&mut self, index: usize) -> Result<Translated, Failure> {
        let (block, ty) = self.function(index, 0)?;
        let pointer = self.module.target_config().pointer_type();
        let mut signature = self.module.make_signature();
        signature.params.push(AbiParam::new(pointer));
        signature.returns.push(AbiParam::new(types::I32));
        let id = self
            .module
            .declare_function(
                &format!("qsl_entry_{index}"),
                cranelift_module::Linkage::Export,
                &signature,
            )
            .map_err(|_| fault("jit-declare-entry"))?;
        let frontend = self.module.target_config();
        let mut context = self.module.make_context();
        let recorded = Signature::of(&signature, pointer);
        context.func.signature = signature;
        let mut builder_context = FunctionBuilderContext::new();
        {
            let mut builder = FunctionBuilder::new(&mut context.func, &mut builder_context);
            let body = builder.create_block();
            builder.append_block_params_for_function_params(body);
            builder.switch_to_block(body);
            let frame = builder.block_params(body)[0];
            let flags = MemFlagsData::trusted();
            let arguments = builder.ins().load(pointer, flags, frame, ARGUMENTS_OFFSET);
            let callee = self.module.declare_func_in_func(block, builder.func);
            let call = builder.ins().call(callee, &[frame, arguments]);
            let result = builder.inst_results(call)[0];
            builder.ins().store(flags, result, frame, RESULT_OFFSET);
            let failed = builder.ins().load(types::I64, flags, frame, FAILED_OFFSET);
            let status = builder.ins().ireduce(types::I32, failed);
            builder.ins().return_(&[status]);
            builder.seal_all_blocks();
            builder.finalize(frontend);
        }
        self.module
            .define_function(id, &mut context)
            .map_err(|_| fault("jit-define-entry"))?;
        self.module.clear_context(&mut context);
        Ok(Translated {
            id,
            result: ty.carried(),
            signature: recorded,
        })
    }

    /// Translate the function at `index` (once) after the functions it calls.
    fn function(&mut self, index: usize, depth: usize) -> Result<(FuncId, Ty), Failure> {
        match self.state.get(&index) {
            Some(State::Done(id, ty, ..)) => return Ok((*id, *ty)),
            Some(State::Unsupported) | Some(State::InProgress) => {
                return Err(self.placeholder(index));
            }
            None => {}
        }
        let graph = self.graph;
        let (Some(function), Some(identity)) = (
            graph.function_state(index),
            graph.function_identity_at(index),
        ) else {
            return Err(fault("jit-function-index"));
        };
        let Some(callable) = graph.callable_by_identity(identity) else {
            self.state.insert(index, State::Unsupported);
            return Err(self.placeholder(index));
        };
        if depth > MAX_DEPTH {
            self.state.insert(index, State::Unsupported);
            return Err(Failure::Unsupported(untranslated(function.body)));
        }
        if let Some(outside) =
            qsl_eval::execution::first_outside(callable.parameters, function.body, self.coverage)
        {
            self.state.insert(index, State::Unsupported);
            return Err(Failure::Unsupported(outside));
        }
        self.state.insert(index, State::InProgress);
        // Callees first, so a call site finds them translated.
        let mut pending = vec![function.body];
        while let Some(node) = pending.pop() {
            if let NodeKind::Call {
                function: callee, ..
            } = node.kind()
            {
                match self.function(*callee, depth + 1) {
                    Ok(_) => {}
                    Err(Failure::Unsupported(_)) => {
                        self.state.insert(index, State::Unsupported);
                        return Err(Failure::Unsupported(untranslated(node)));
                    }
                    Err(other) => return Err(other),
                }
            }
            pending.extend(node.children().into_iter().rev());
        }
        let parameters: Vec<Option<Ty>> = callable
            .parameters
            .iter()
            .map(|(_, t)| type_of(t))
            .collect();
        match self.build(index, &parameters, function.body) {
            Ok((id, ty, allocates)) => {
                self.state
                    .insert(index, State::Done(id, ty, parameters, allocates));
                Ok((id, ty))
            }
            Err(failure) => {
                self.state.insert(index, State::Unsupported);
                Err(failure)
            }
        }
    }

    fn placeholder(&self, index: usize) -> Failure {
        match self.graph.function_state(index) {
            Some(function) => Failure::Unsupported(untranslated(function.body)),
            None => fault("jit-function-index"),
        }
    }

    fn build(
        &mut self,
        index: usize,
        parameters: &[Option<Ty>],
        body: CheckedNode<'_>,
    ) -> Result<(FuncId, Ty, bool), Failure> {
        let module = &mut *self.module;
        let pointer = module.target_config().pointer_type();
        let mut signature = module.make_signature();
        signature.params.push(AbiParam::new(pointer));
        signature.params.push(AbiParam::new(pointer));
        signature.returns.push(AbiParam::new(types::I64));
        let id = module
            .declare_function(
                &format!("qsl_f{index}"),
                cranelift_module::Linkage::Export,
                &signature,
            )
            .map_err(|_| fault("jit-declare-function"))?;
        let frontend = module.target_config();
        let mut context = module.make_context();
        context.func.signature = signature;
        let mut builder_context = FunctionBuilderContext::new();
        let (ty, allocates) = {
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
                module: &mut *module,
                shim: self.shim,
                context: ctx_value,
                exit,
                ops: &mut *self.ops,
                state: &self.state,
                slots: HashMap::new(),
                meter: &mut *self.meter,
                refused: None,
                allocates: false,
            };
            for (position, ty) in parameters.iter().enumerate() {
                if let Some(ty) = ty {
                    let offset =
                        i32::try_from(position * 8).map_err(|_| fault("jit-parameter-offset"))?;
                    let value = emitter.builder.ins().load(
                        types::I64,
                        MemFlagsData::trusted(),
                        args,
                        offset,
                    );
                    emitter.slots.insert(position, (value, *ty));
                }
            }
            let (result, ty) = emitter.emit(body, 0)?;
            if let Some(invariant) = emitter.refused {
                return Err(fault(invariant));
            }
            emitter.builder.ins().return_(&[result]);
            emitter.builder.switch_to_block(exit);
            let zero = emitter.builder.ins().iconst(types::I64, 0);
            emitter.builder.ins().return_(&[zero]);
            emitter.builder.seal_all_blocks();
            let allocates = emitter.allocates;
            emitter.builder.finalize(frontend);
            (ty, allocates)
        };
        module
            .define_function(id, &mut context)
            .map_err(|_| fault("jit-define-function"))?;
        let code_bytes = match context.compiled_code() {
            Some(code) => {
                u64::try_from(code.code_buffer().len()).map_err(|_| fault("jit-code-size"))?
            }
            None => return Err(fault("jit-code-missing")),
        };
        module.clear_context(&mut context);
        self.meter.charge_block(code_bytes)?;
        Ok((id, ty, allocates))
    }
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
                Value::Integer(value) => match integer_to_i64(value) {
                    Some(small) => {
                        let v = self.builder.ins().iconst(types::I64, small);
                        Ok((v, Ty::Int(small.into(), small.into())))
                    }
                    None => {
                        let zero = self.builder.ins().iconst(types::I64, 0);
                        let operands = [(zero, Ty::Boolean), (zero, Ty::Boolean)];
                        let result = self.shim_call(
                            node,
                            OpKind::Literal(Value::Integer(value.clone())),
                            operands,
                            Repr::Handle,
                        );
                        Ok((result, Ty::Wide))
                    }
                },
                Value::Rational(_) | Value::Decimal(_) | Value::Text(_) | Value::Composite(_) => {
                    let ty = match &literal.0 {
                        Value::Rational(_) => Ty::Rational,
                        Value::Decimal(_) => Ty::Decimal,
                        Value::Composite(_) => Ty::Record,
                        _ => Ty::Text,
                    };
                    let zero = self.builder.ins().iconst(types::I64, 0);
                    let operands = [(zero, Ty::Boolean), (zero, Ty::Boolean)];
                    let kind = OpKind::Literal(literal.0.clone());
                    Ok((self.shim_call(node, kind, operands, Repr::Handle), ty))
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
                if !(left_ty.is_integer() && right_ty.is_integer()) {
                    return Err(self.unsupported(node));
                }
                // FR-562: a result interval inside 64 bits is a register
                // result; any other result is an arena handle.
                let interval = match (left_ty, right_ty) {
                    (Ty::Int(al, ah), Ty::Int(bl, bh)) => {
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
                        fits(low, high).then_some((low, high))
                    }
                    _ => None,
                };
                let ty = interval.map_or(Ty::Wide, |(low, high)| Ty::Int(low, high));
                let operands = [(a, left_ty), (b, right_ty)];
                if let (Ty::Int(..), (Ty::Int(al, ah), Ty::Int(bl, bh))) = (ty, (left_ty, right_ty))
                {
                    let (left, right) = (native_interval(al, ah)?, native_interval(bl, bh)?);
                    let operation = match operator {
                        Arithmetic::Add => NativeIntegerOperation::Add { left, right },
                        Arithmetic::Subtract => NativeIntegerOperation::Subtract { left, right },
                        Arithmetic::Multiply => NativeIntegerOperation::Multiply { left, right },
                    };
                    let charge = match operator {
                        Arithmetic::Multiply => NativeCharge::Multiply,
                        _ => NativeCharge::Arithmetic(native_plan(operation)?, false),
                    };
                    self.shim_call(node, OpKind::ChargeNative(charge), operands, Repr::Register);
                    let ins = self.builder.ins();
                    let value = match operator {
                        Arithmetic::Add => ins.iadd(a, b),
                        Arithmetic::Subtract => ins.isub(a, b),
                        Arithmetic::Multiply => ins.imul(a, b),
                    };
                    return Ok((value, ty));
                }
                let result =
                    self.shim_call(node, OpKind::Arithmetic(*operator), operands, ty.repr());
                Ok((result, ty))
            }
            NodeKind::Negate(operand) => {
                let (a, operand_ty) = self.emit(node.at(*operand), depth + 1)?;
                let ty = match operand_ty {
                    Ty::Int(low, high) if fits(-high, -low) => Ty::Int(-high, -low),
                    Ty::Int(..) | Ty::Wide => Ty::Wide,
                    Ty::Boolean
                    | Ty::Rational
                    | Ty::Decimal
                    | Ty::Text
                    | Ty::Record
                    | Ty::Collection => return Err(self.unsupported(node)),
                };
                let zero = self.builder.ins().iconst(types::I64, 0);
                let operands = [(a, operand_ty), (zero, Ty::Boolean)];
                if let (Ty::Int(..), Ty::Int(low, high)) = (ty, operand_ty) {
                    let operation = NativeIntegerOperation::Negate {
                        operand: native_interval(low, high)?,
                    };
                    let charge = NativeCharge::Arithmetic(native_plan(operation)?, true);
                    self.shim_call(node, OpKind::ChargeNative(charge), operands, Repr::Register);
                    let value = self.builder.ins().ineg(a);
                    return Ok((value, ty));
                }
                let result = self.shim_call(node, OpKind::Negate, operands, ty.repr());
                Ok((result, ty))
            }
            NodeKind::IntegerDivision {
                operator,
                selection,
                left,
                right,
            } => {
                let (a, left_ty) = self.emit(node.at(*left), depth + 1)?;
                let (b, right_ty) = self.emit(node.at(*right), depth + 1)?;
                if !(left_ty.is_integer() && right_ty.is_integer()) {
                    return Err(self.unsupported(node));
                }
                // A `div` or `rem` with no admitted selection is the
                // interpreter's invariant stop; it is listed unsupported, not
                // translated.
                if lacks_selection(*operator, selection.as_ref()) {
                    return Err(self.unsupported(node));
                }
                // FR-562: a divisor interval that excludes 0 and a result
                // interval inside 64 bits give a register result; any other
                // division returns a handle (a zero divisor is the kernel's
                // `Undefined`).
                let interval = match (left_ty, right_ty) {
                    (Ty::Int(al, ah), Ty::Int(bl, bh)) if !(bl <= 0 && 0 <= bh) => {
                        division_interval(*operator, *selection, (al, ah), (bl, bh))
                            .filter(|(low, high)| fits(*low, *high))
                    }
                    _ => None,
                };
                let ty = interval.map_or(Ty::Wide, |(low, high)| Ty::Int(low, high));
                let operands = [(a, left_ty), (b, right_ty)];
                if let (Ty::Int(..), Ty::Int(..), Ty::Int(..)) = (ty, left_ty, right_ty) {
                    let charge = match operator {
                        IntegerDivisionOperator::Mod => NativeCharge::Modulus,
                        _ => NativeCharge::Division,
                    };
                    self.shim_call(node, OpKind::ChargeNative(charge), operands, Repr::Register);
                    let (profile, member) = match (operator, selection) {
                        (IntegerDivisionOperator::Mod, _) => {
                            (DivisionProfile::Euclidean, DivisionMember::Remainder)
                        }
                        (IntegerDivisionOperator::Div, Some(law)) => {
                            (law.profile(), DivisionMember::Quotient)
                        }
                        (_, Some(law)) => (law.profile(), DivisionMember::Remainder),
                        (_, None) => return Err(self.unsupported(node)),
                    };
                    let value = self.machine_division(profile, member, a, b);
                    return Ok((value, ty));
                }
                let kind = OpKind::Divide {
                    operator: *operator,
                    selection: *selection,
                };
                let result = self.shim_call(node, kind, operands, ty.repr());
                Ok((result, ty))
            }
            NodeKind::Order(operator, kind, left, right) => {
                let (a, left_ty) = self.emit(node.at(*left), depth + 1)?;
                let (b, right_ty) = self.emit(node.at(*right), depth + 1)?;
                let operands_fit = match kind {
                    OrderedKind::Integers => left_ty.is_integer() && right_ty.is_integer(),
                    OrderedKind::Rationals => (left_ty, right_ty) == (Ty::Rational, Ty::Rational),
                    OrderedKind::Decimals => (left_ty, right_ty) == (Ty::Decimal, Ty::Decimal),
                    OrderedKind::Texts => (left_ty, right_ty) == (Ty::Text, Ty::Text),
                    OrderedKind::Float | OrderedKind::Enums | OrderedKind::Quantities => false,
                };
                if !operands_fit {
                    return Err(self.unsupported(node));
                }
                let operands = [(a, left_ty), (b, right_ty)];
                if matches!((left_ty, right_ty), (Ty::Int(..), Ty::Int(..)))
                    && *kind == OrderedKind::Integers
                {
                    self.shim_call(
                        node,
                        OpKind::ChargeNative(NativeCharge::Ordering),
                        operands,
                        Repr::Register,
                    );
                    let condition = match operator {
                        OrderingOperator::Less => IntCC::SignedLessThan,
                        OrderingOperator::LessOrEqual => IntCC::SignedLessThanOrEqual,
                        OrderingOperator::Greater => IntCC::SignedGreaterThan,
                        OrderingOperator::GreaterOrEqual => IntCC::SignedGreaterThanOrEqual,
                    };
                    let held = self.builder.ins().icmp(condition, a, b);
                    let value = self.builder.ins().uextend(types::I64, held);
                    return Ok((value, Ty::Boolean));
                }
                let order = OpKind::Order((*operator, *kind));
                let result = self.shim_call(node, order, operands, Repr::Register);
                Ok((result, Ty::Boolean))
            }
            NodeKind::Record { declaration, slots } => {
                for slot in slots {
                    if let RecordSlot::Present(child) = slot {
                        let operand = self.emit(node.at(*child), depth + 1)?;
                        self.stage(node, operand);
                    }
                }
                let slots = slots
                    .iter()
                    .map(|slot| match slot {
                        RecordSlot::Absent => RecordSlot::Absent,
                        RecordSlot::Null => RecordSlot::Null,
                        RecordSlot::Present(_) => RecordSlot::Present(()),
                    })
                    .collect();
                let kind = OpKind::Record {
                    declaration: declaration.clone(),
                    slots,
                };
                let zero = self.builder.ins().iconst(types::I64, 0);
                let result = self.shim_call(
                    node,
                    kind,
                    [(zero, Ty::Boolean), (zero, Ty::Boolean)],
                    Repr::Handle,
                );
                Ok((result, Ty::Record))
            }
            NodeKind::Tuple {
                declaration,
                arguments,
            } => {
                for argument in arguments {
                    let operand = self.emit(node.at(*argument), depth + 1)?;
                    self.stage(node, operand);
                }
                let kind = OpKind::Tuple {
                    declaration: declaration.clone(),
                    arity: arguments.len(),
                };
                let zero = self.builder.ins().iconst(types::I64, 0);
                let result = self.shim_call(
                    node,
                    kind,
                    [(zero, Ty::Boolean), (zero, Ty::Boolean)],
                    Repr::Handle,
                );
                Ok((result, Ty::Record))
            }
            NodeKind::Field {
                operand,
                field,
                optional,
            } => {
                let (a, operand_ty) = self.emit(node.at(*operand), depth + 1)?;
                let Some(ty) = type_of(node.value_type()) else {
                    return Err(self.unsupported(node));
                };
                if operand_ty != Ty::Record {
                    return Err(self.unsupported(node));
                }
                let kind = OpKind::Field {
                    field: field.clone(),
                    optional: *optional,
                    value_type: node.value_type().clone(),
                    into: ty.carried(),
                };
                let zero = self.builder.ins().iconst(types::I64, 0);
                let result = self.shim_call(
                    node,
                    kind,
                    [(a, operand_ty), (zero, Ty::Boolean)],
                    ty.repr(),
                );
                Ok((result, ty))
            }
            NodeKind::Collection {
                collection_type,
                elements,
            } => {
                for element in elements {
                    self.nullary(node, OpKind::ChargeElement, Repr::Register);
                    let operand = self.emit(node.at(*element), depth + 1)?;
                    self.stage(node, operand);
                }
                let kind = OpKind::Collection {
                    collection_type: collection_type.clone(),
                    count: elements.len(),
                };
                let result = self.nullary(node, kind, Repr::Handle);
                Ok((result, Ty::Collection))
            }
            NodeKind::Size(operand) => {
                let source = self.emit_collection(node, *operand, depth)?;
                let Some(ty) = type_of(node.value_type()).filter(|ty| ty.is_integer()) else {
                    return Err(self.unsupported(node));
                };
                let result = self.unary(node, OpKind::Size, source, ty.repr());
                Ok((result, ty))
            }
            NodeKind::Contains(collection, item) => {
                let source = self.emit_collection(node, *collection, depth)?;
                let (b, item_ty) = self.emit(node.at(*item), depth + 1)?;
                let kind = OpKind::Contains {
                    item_boolean: item_ty == Ty::Boolean,
                };
                let result = self.shim_call(node, kind, [source, (b, item_ty)], Repr::Register);
                Ok((result, Ty::Boolean))
            }
            NodeKind::Flatten(operand) => {
                let source = self.emit_collection(node, *operand, depth)?;
                if !matches!(node.value_type(), ValueType::Collection(_)) {
                    return Err(self.unsupported(node));
                }
                let kind = OpKind::Flatten {
                    result_type: node.value_type().clone(),
                };
                let result = self.unary(node, kind, source, Repr::Handle);
                Ok((result, Ty::Collection))
            }
            NodeKind::ConvertCollection { target, operand } => {
                let source = self.emit_collection(node, *operand, depth)?;
                let kind = OpKind::ConvertCollection {
                    target: target.clone(),
                };
                let result = self.unary(node, kind, source, Repr::Handle);
                Ok((result, Ty::Collection))
            }
            NodeKind::DropFirst(operand) => {
                let source = self.emit_collection(node, *operand, depth)?;
                let result = self.unary(node, OpKind::DropFirst, source, Repr::Handle);
                Ok((result, Ty::Collection))
            }
            NodeKind::RemoveOne(collection, item) => {
                let source = self.emit_collection(node, *collection, depth)?;
                let (b, item_ty) = self.emit(node.at(*item), depth + 1)?;
                let kind = OpKind::RemoveOne {
                    item_boolean: item_ty == Ty::Boolean,
                };
                let result = self.shim_call(node, kind, [source, (b, item_ty)], Repr::Handle);
                Ok((result, Ty::Collection))
            }
            NodeKind::Query { .. } | NodeKind::Fold { .. } => self.emit_iteration(node, depth),
            NodeKind::Divide {
                left,
                right,
                domain,
            } => {
                let (a, left_ty) = self.emit(node.at(*left), depth + 1)?;
                let (b, right_ty) = self.emit(node.at(*right), depth + 1)?;
                if !(left_ty.is_integer() && right_ty.is_integer()) {
                    return Err(self.unsupported(node));
                }
                let kind = OpKind::RationalDivide {
                    domain: domain.clone(),
                };
                let result =
                    self.shim_call(node, kind, [(a, left_ty), (b, right_ty)], Repr::Handle);
                Ok((result, Ty::Rational))
            }
            NodeKind::Rational {
                operator,
                left,
                right,
                domain,
            } => {
                let (a, left_ty) = self.emit(node.at(*left), depth + 1)?;
                let (b, right_ty) = self.emit(node.at(*right), depth + 1)?;
                if (left_ty, right_ty) != (Ty::Rational, Ty::Rational) {
                    return Err(self.unsupported(node));
                }
                let kind = OpKind::Rational {
                    operator: *operator,
                    domain: domain.clone(),
                };
                let result =
                    self.shim_call(node, kind, [(a, left_ty), (b, right_ty)], Repr::Handle);
                Ok((result, Ty::Rational))
            }
            NodeKind::RationalNegate(operand, domain) => {
                let (a, ty) = self.emit(node.at(*operand), depth + 1)?;
                if ty != Ty::Rational {
                    return Err(self.unsupported(node));
                }
                let zero = self.builder.ins().iconst(types::I64, 0);
                let kind = OpKind::RationalNegate {
                    domain: domain.clone(),
                };
                let result =
                    self.shim_call(node, kind, [(a, ty), (zero, Ty::Boolean)], Repr::Handle);
                Ok((result, Ty::Rational))
            }
            NodeKind::Decimal {
                operator,
                left,
                right,
                target,
            } => {
                let (a, left_ty) = self.emit(node.at(*left), depth + 1)?;
                let (b, right_ty) = self.emit(node.at(*right), depth + 1)?;
                if (left_ty, right_ty) != (Ty::Decimal, Ty::Decimal) {
                    return Err(self.unsupported(node));
                }
                let kind = OpKind::Decimal {
                    operator: *operator,
                    target: target.clone(),
                };
                let result =
                    self.shim_call(node, kind, [(a, left_ty), (b, right_ty)], Repr::Handle);
                Ok((result, Ty::Decimal))
            }
            NodeKind::DecimalNegate(operand, target) => {
                let (a, ty) = self.emit(node.at(*operand), depth + 1)?;
                if ty != Ty::Decimal {
                    return Err(self.unsupported(node));
                }
                let zero = self.builder.ins().iconst(types::I64, 0);
                let kind = OpKind::DecimalNegate {
                    target: target.clone(),
                };
                let result =
                    self.shim_call(node, kind, [(a, ty), (zero, Ty::Boolean)], Repr::Handle);
                Ok((result, Ty::Decimal))
            }
            NodeKind::ConvertDecimal(operand, target) => {
                let (a, ty) = self.emit(node.at(*operand), depth + 1)?;
                if !matches!(ty, Ty::Decimal | Ty::Rational) {
                    return Err(self.unsupported(node));
                }
                let zero = self.builder.ins().iconst(types::I64, 0);
                let kind = OpKind::ConvertDecimal {
                    target: target.clone(),
                };
                let result =
                    self.shim_call(node, kind, [(a, ty), (zero, Ty::Boolean)], Repr::Handle);
                Ok((result, Ty::Decimal))
            }
            NodeKind::Coerce(operand, target) => {
                let (a, ty) = self.emit(node.at(*operand), depth + 1)?;
                let narrowed = match ty {
                    Ty::Int(low, high) => {
                        // The result lies in the operand's interval and the
                        // target's; an empty intersection always refuses, so
                        // keep the operand's.
                        let (target_low, target_high) = (
                            clamp(target.lower(), low, high),
                            clamp(target.upper(), low, high),
                        );
                        if target_low <= target_high {
                            Ty::Int(target_low, target_high)
                        } else {
                            ty
                        }
                    }
                    // A handle narrowed to a target inside 64 bits is a
                    // register; any other target keeps the handle.
                    Ty::Wide => match (
                        integer_to_i64(target.lower()),
                        integer_to_i64(target.upper()),
                    ) {
                        (Some(low), Some(high)) => Ty::Int(low.into(), high.into()),
                        _ => Ty::Wide,
                    },
                    Ty::Boolean
                    | Ty::Rational
                    | Ty::Decimal
                    | Ty::Text
                    | Ty::Record
                    | Ty::Collection => return Err(self.unsupported(node)),
                };
                let zero = self.builder.ins().iconst(types::I64, 0);
                let operands = [(a, ty), (zero, Ty::Boolean)];
                let result = self.shim_call(
                    node,
                    OpKind::Coerce(target.clone()),
                    operands,
                    narrowed.repr(),
                );
                Ok((result, narrowed))
            }
            NodeKind::ConvertScalar(operand, source_node) => {
                // Only an Integer-like to Integer-like conversion is
                // translated; float, quantity, decimal and rational
                // conversions stay with the interpreter.
                let integer_like =
                    |ty: &ValueType| matches!(ty, ValueType::Integer | ValueType::Int(_));
                let source = node.at(*source_node);
                if !integer_like(source.value_type()) || !integer_like(node.value_type()) {
                    return Err(self.unsupported(node));
                }
                let (a, ty) = self.emit(source, depth + 1)?;
                let Some(result_ty) = type_of(node.value_type()) else {
                    return Err(self.unsupported(node));
                };
                if !ty.is_integer() || !result_ty.is_integer() {
                    return Err(self.unsupported(node));
                }
                let zero = self.builder.ins().iconst(types::I64, 0);
                let result = self.shim_call(
                    node,
                    OpKind::ConvertScalar(Box::new(operand.clone())),
                    [(a, ty), (zero, Ty::Boolean)],
                    result_ty.repr(),
                );
                Ok((result, result_ty))
            }
            NodeKind::Equality(operator, checked, left, right) => {
                let (a, left_ty) = self.emit(node.at(*left), depth + 1)?;
                let (b, right_ty) = self.emit(node.at(*right), depth + 1)?;
                let operands_boolean = match (left_ty, right_ty) {
                    (Ty::Boolean, Ty::Boolean) => true,
                    (l, r) if l.is_integer() && r.is_integer() => false,
                    (l, r) if l == r && l.repr() == Repr::Handle => false,
                    _ => return Err(self.unsupported(node)),
                };
                let operands = [(a, left_ty), (b, right_ty)];
                if matches!((left_ty, right_ty), (Ty::Int(..), Ty::Int(..))) {
                    self.shim_call(
                        node,
                        OpKind::ChargeNative(NativeCharge::Equality),
                        operands,
                        Repr::Register,
                    );
                    let condition = match operator {
                        EqualityOperator::Equal => IntCC::Equal,
                        EqualityOperator::NotEqual => IntCC::NotEqual,
                    };
                    let held = self.builder.ins().icmp(condition, a, b);
                    let value = self.builder.ins().uextend(types::I64, held);
                    return Ok((value, Ty::Boolean));
                }
                let kind = OpKind::Equality {
                    checked: checked.clone(),
                    operands_boolean,
                };
                let result = self.shim_call(node, kind, operands, Repr::Register);
                Ok((result, Ty::Boolean))
            }
            NodeKind::Not(operand) => {
                let (a, ty) = self.emit(node.at(*operand), depth + 1)?;
                if ty != Ty::Boolean {
                    return Err(self.unsupported(node));
                }
                let zero = self.builder.ins().iconst(types::I64, 0);
                let result = self.shim_call(
                    node,
                    OpKind::Not,
                    [(a, Ty::Boolean), (zero, Ty::Boolean)],
                    Repr::Register,
                );
                Ok((result, Ty::Boolean))
            }
            NodeKind::Connective(connective, _, left, right) => {
                self.emit_connective(node, (*connective, *left, *right), depth)
            }
            NodeKind::Call {
                function,
                arguments,
            } => {
                let Some(State::Done(callee, result_ty, parameters, callee_allocates)) =
                    self.state.get(function)
                else {
                    return Err(self.unsupported(node));
                };
                let (callee, result_ty, parameters, callee_allocates) =
                    (*callee, *result_ty, parameters.clone(), *callee_allocates);
                // A callee that adds handles has them dropped when it returns
                // (the interpreter drops its intermediates as it goes): the
                // arena mark is taken before the arguments, and the arena is
                // cut back to it keeping only the result.
                let mark = if callee_allocates {
                    self.allocates = true;
                    let zero = self.builder.ins().iconst(types::I64, 0);
                    Some(self.shim_call(
                        node,
                        OpKind::Mark,
                        [(zero, Ty::Boolean), (zero, Ty::Boolean)],
                        Repr::Register,
                    ))
                } else {
                    None
                };
                let mut values = Vec::with_capacity(arguments.len());
                for (position, argument) in arguments.iter().enumerate() {
                    let emitted = self.emit(node.at(*argument), depth + 1)?;
                    // The callee carries each parameter as its own type
                    // says; an argument carried the other way is boxed or
                    // unboxed here (`None`: the callee does not read it).
                    let value = match parameters.get(position).copied().flatten() {
                        Some(parameter) => self.convert(node, emitted, parameter)?,
                        None => emitted.0,
                    };
                    values.push(value);
                }
                let zero = self.builder.ins().iconst(types::I64, 0);
                let charged = self.shim_call(
                    node,
                    OpKind::ChargeCall,
                    [(zero, Ty::Boolean), (zero, Ty::Boolean)],
                    Repr::Register,
                );
                let _ = charged;
                let Ok(size) = u32::try_from(values.len().max(1) * 8) else {
                    return Err(self.unsupported(node));
                };
                let slot = self.builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    size,
                    3,
                ));
                for (position, value) in values.iter().enumerate() {
                    let Ok(offset) = i32::try_from(position * 8) else {
                        return Err(self.unsupported(node));
                    };
                    self.builder
                        .ins()
                        .stack_store(types::I64, *value, slot, offset);
                }
                let pointer = self.module.target_config().pointer_type();
                let address = self.builder.ins().stack_addr(pointer, slot, 0);
                let function_ref = self.module.declare_func_in_func(callee, self.builder.func);
                let call = self
                    .builder
                    .ins()
                    .call(function_ref, &[self.context, address]);
                let mut result = self.builder.inst_results(call)[0];
                self.leave_on_failure();
                if let Some(mark) = mark {
                    result = self.shim_call(
                        node,
                        OpKind::Reclaim,
                        [(result, result_ty), (mark, Ty::Boolean)],
                        result_ty.repr(),
                    );
                }
                Ok((result, result_ty))
            }
            _ => Err(self.unsupported(node)),
        }
    }

    fn emit_connective(
        &mut self,
        node: CheckedNode<'_>,
        (connective, left, right): (
            Connective,
            qsl_semantics::check::NodeId,
            qsl_semantics::check::NodeId,
        ),
        depth: usize,
    ) -> Result<(Clif, Ty), Failure> {
        let (selector, left_ty) = self.emit(node.at(left), depth + 1)?;
        if left_ty != Ty::Boolean {
            return Err(self.unsupported(node));
        }
        let (skip, evaluate, merge) = (
            self.builder.create_block(),
            self.builder.create_block(),
            self.builder.create_block(),
        );
        self.builder.append_block_param(merge, types::I64);
        // The left operand alone decides `and` when false, `or` when true and
        // `implies` when false; the decided result is retained (charged) like
        // any other connective result.
        let (decided, when_decided) = match connective {
            Connective::And => (false, false),
            Connective::Or => (true, true),
            Connective::Implies => (true, false),
        };
        if when_decided {
            self.builder.ins().brif(selector, skip, &[], evaluate, &[]);
        } else {
            self.builder.ins().brif(selector, evaluate, &[], skip, &[]);
        }
        self.builder.switch_to_block(skip);
        let constant = self.builder.ins().iconst(types::I64, i64::from(decided));
        let zero = self.builder.ins().iconst(types::I64, 0);
        let retained = self.shim_call(
            node,
            OpKind::Retain,
            [(constant, Ty::Boolean), (zero, Ty::Boolean)],
            Repr::Register,
        );
        self.builder.ins().jump(merge, &[retained.into()]);
        self.builder.switch_to_block(evaluate);
        let (value, right_ty) = self.emit(node.at(right), depth + 1)?;
        if right_ty != Ty::Boolean {
            return Err(self.unsupported(node));
        }
        let zero = self.builder.ins().iconst(types::I64, 0);
        let retained = self.shim_call(
            node,
            OpKind::Retain,
            [(value, Ty::Boolean), (zero, Ty::Boolean)],
            Repr::Register,
        );
        self.builder.ins().jump(merge, &[retained.into()]);
        self.builder.switch_to_block(merge);
        let joined = self.builder.block_params(merge)[0];
        Ok((joined, Ty::Boolean))
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
        let (then_join, else_join) = (self.builder.create_block(), self.builder.create_block());
        for block in [merge, then_join, else_join] {
            self.builder.append_block_param(block, types::I64);
        }
        self.builder
            .ins()
            .brif(selector, then_block, &[], else_block, &[]);
        self.builder.switch_to_block(then_block);
        let (taken, then_ty) = self.emit(node.at(then), depth + 1)?;
        self.builder.ins().jump(then_join, &[taken.into()]);
        self.builder.switch_to_block(else_block);
        let (other, else_ty) = self.emit(node.at(otherwise), depth + 1)?;
        self.builder.ins().jump(else_join, &[other.into()]);
        let ty = match (then_ty, else_ty) {
            (Ty::Boolean, Ty::Boolean) => Ty::Boolean,
            (Ty::Int(a, b), Ty::Int(c, d)) => Ty::Int(a.min(c), b.max(d)),
            (a, b) if a.is_integer() && b.is_integer() => Ty::Wide,
            (a, b) if a == b => a,
            _ => return Err(self.unsupported(node)),
        };
        // Each branch meets the join in the join's representation.
        for (block, branch_ty) in [(then_join, then_ty), (else_join, else_ty)] {
            self.builder.switch_to_block(block);
            let value = self.builder.block_params(block)[0];
            let converted = self.convert(node, (value, branch_ty), ty)?;
            self.builder.ins().jump(merge, &[converted.into()]);
        }
        self.builder.switch_to_block(merge);
        let joined = self.builder.block_params(merge)[0];
        Ok((joined, ty))
    }

    /// The collection operand `operand` of `node`, evaluated.
    fn emit_collection(
        &mut self,
        node: CheckedNode<'_>,
        operand: qsl_semantics::check::NodeId,
        depth: usize,
    ) -> Result<(Clif, Ty), Failure> {
        let emitted = self.emit(node.at(operand), depth + 1)?;
        if emitted.1 != Ty::Collection {
            return Err(self.unsupported(node));
        }
        Ok(emitted)
    }

    /// A shim call with no operands.
    fn nullary(&mut self, node: CheckedNode<'_>, kind: OpKind, result: Repr) -> Clif {
        let zero = self.builder.ins().iconst(types::I64, 0);
        self.shim_call(
            node,
            kind,
            [(zero, Ty::Boolean), (zero, Ty::Boolean)],
            result,
        )
    }

    /// A shim call over one operand.
    fn unary(
        &mut self,
        node: CheckedNode<'_>,
        kind: OpKind,
        operand: (Clif, Ty),
        result: Repr,
    ) -> Clif {
        let zero = self.builder.ins().iconst(types::I64, 0);
        self.shim_call(node, kind, [operand, (zero, Ty::Boolean)], result)
    }

    /// Begin a loop: a header that asks `next` whether another element
    /// follows, and the body block it enters. Returns the header and the
    /// block after the loop.
    fn open_loop(&mut self, node: CheckedNode<'_>, next: OpKind) -> (Block, Block) {
        let (header, body, done) = (
            self.builder.create_block(),
            self.builder.create_block(),
            self.builder.create_block(),
        );
        self.builder.ins().jump(header, &[]);
        self.builder.switch_to_block(header);
        let more = self.nullary(node, next, Repr::Register);
        self.builder.ins().brif(more, body, &[], done, &[]);
        self.builder.switch_to_block(body);
        (header, done)
    }

    /// End a loop body: back to the header, then on from the block after.
    fn close_loop(&mut self, (header, done): (Block, Block)) {
        self.builder.ins().jump(header, &[]);
        self.builder.switch_to_block(done);
    }

    /// Bind `slot` for a loop body, returning what it replaced.
    fn bind(&mut self, slot: usize, bound: (Clif, Ty)) -> Option<(Clif, Ty)> {
        self.slots.insert(slot, bound)
    }

    fn unbind(&mut self, slot: usize, previous: Option<(Clif, Ty)>) {
        match previous {
            Some(previous) => self.slots.insert(slot, previous),
            None => self.slots.remove(&slot),
        };
    }

    /// A one-binder query or a fold: the loop runs the shared iteration of
    /// `qsl_eval::value::collections` through the shim, so the visits, the
    /// retained results and the stops are the interpreter's. `forall` and
    /// `exists` are outside the coverage (FR-560) and are not translated.
    fn emit_iteration(
        &mut self,
        node: CheckedNode<'_>,
        depth: usize,
    ) -> Result<(Clif, Ty), Failure> {
        let (shape, source, body, identity) = match node.kind() {
            NodeKind::Query {
                visit: Visit::Forall | Visit::Exists,
                ..
            } => return Err(self.unsupported(node)),
            NodeKind::Query {
                visit,
                source,
                body,
                ..
            } => (Shape::Query(*visit), *source, *body, None),
            NodeKind::Fold {
                source,
                step,
                identity,
                ..
            } => (Shape::Fold, *source, *step, *identity),
            _ => return Err(self.unsupported(node)),
        };
        let Some(result_ty) = type_of(node.value_type()) else {
            return Err(self.unsupported(node));
        };
        let family_ok = match shape {
            Shape::Query(Visit::Map | Visit::Filter) => result_ty == Ty::Collection,
            Shape::Query(Visit::Count) => result_ty.is_integer(),
            // The empty and float sums are the interpreter's own: only the
            // exact families run the shared step.
            Shape::Query(Visit::Sum) => is_exact_sum(node.value_type()),
            Shape::Query(Visit::Only) | Shape::Fold => true,
            Shape::Query(Visit::Forall | Visit::Exists) => false,
        };
        if !family_ok {
            return Err(self.unsupported(node));
        }
        let source_node = node.at(source);
        let element_ty = match source_node.value_type() {
            ValueType::Collection(collection) => type_of(collection.element()),
            _ => None,
        };
        let Some(element_ty) = element_ty else {
            return Err(self.unsupported(node));
        };
        let collection = self.emit_collection(node, source, depth)?;
        let identity = match identity {
            Some(identity) => Some(self.emit(node.at(identity), depth + 1)?),
            None => None,
        };
        let source_is_filter = matches!(
            source_node.kind(),
            NodeKind::Query {
                visit: Visit::Filter,
                ..
            }
        );
        let spec = IterSpec {
            shape,
            value_type: node.value_type().clone(),
            only_over_filter: Spec::new(shape, node.value_type(), source_is_filter)
                .only_over_filter,
        };
        let start = OpKind::IterStart {
            spec: spec.clone(),
            population: qsl_eval::value::only_population(source_node),
            identity_boolean: identity.map(|(_, ty)| ty == Ty::Boolean),
        };
        let identity = identity.unwrap_or_else(|| {
            let zero = self.builder.ins().iconst(types::I64, 0);
            (zero, Ty::Boolean)
        });
        self.shim_call(node, start, [collection, identity], Repr::Register);
        let looping = self.open_loop(node, OpKind::IterNext { spec: spec.clone() });
        let element = self.nullary(node, OpKind::IterElement, element_ty.repr());
        let bound = match node.kind() {
            NodeKind::Query { slot, .. } => {
                vec![(*slot, self.bind(*slot, (element, element_ty)))]
            }
            NodeKind::Fold {
                accumulator,
                binder,
                ..
            } => {
                let current = self.nullary(node, OpKind::IterAccumulator, result_ty.repr());
                vec![
                    (*accumulator, self.bind(*accumulator, (current, result_ty))),
                    (*binder, self.bind(*binder, (element, element_ty))),
                ]
            }
            _ => return Err(self.unsupported(node)),
        };
        let (result, result_type) = self.emit(node.at(body), depth + 1)?;
        let absorb = OpKind::IterAbsorb {
            spec: spec.clone(),
            body: node.at(body).location().clone(),
            result_boolean: result_type == Ty::Boolean,
        };
        self.unary(node, absorb, (result, result_type), Repr::Register);
        for (slot, previous) in bound.into_iter().rev() {
            self.unbind(slot, previous);
        }
        self.close_loop(looping);
        let finished = self.nullary(node, OpKind::IterFinish { spec }, result_ty.repr());
        Ok((finished, result_ty))
    }

    /// The `div`, `rem` or `mod` of two registers the checked bounds prove
    /// have a nonzero divisor and a result that fits, under `profile`.
    fn machine_division(
        &mut self,
        profile: DivisionProfile,
        member: DivisionMember,
        a: Clif,
        b: Clif,
    ) -> Clif {
        let quotient = self.builder.ins().sdiv(a, b);
        let remainder = self.builder.ins().srem(a, b);
        let zero = self.builder.ins().iconst(types::I64, 0);
        let one = self.builder.ins().iconst(types::I64, 1);
        let negative_remainder = self
            .builder
            .ins()
            .icmp(IntCC::SignedLessThan, remainder, zero);
        let negative_divisor = self.builder.ins().icmp(IntCC::SignedLessThan, b, zero);
        let (q, r) = match profile {
            DivisionProfile::Truncating => (quotient, remainder),
            DivisionProfile::Floor => {
                // A nonzero remainder of the opposite sign to the divisor
                // moves one step.
                let opposite = self
                    .builder
                    .ins()
                    .bxor(negative_remainder, negative_divisor);
                let nonzero = self.builder.ins().icmp(IntCC::NotEqual, remainder, zero);
                let step = self.builder.ins().band(opposite, nonzero);
                let lowered = self.builder.ins().isub(quotient, one);
                let shifted = self.builder.ins().iadd(remainder, b);
                (
                    self.builder.ins().select(step, lowered, quotient),
                    self.builder.ins().select(step, shifted, remainder),
                )
            }
            DivisionProfile::Euclidean => {
                // A negative remainder moves toward the divisor's sign.
                let positive_divisor = self.builder.ins().icmp(IntCC::SignedGreaterThan, b, zero);
                let down = self.builder.ins().isub(quotient, one);
                let up = self.builder.ins().iadd(quotient, one);
                let plus = self.builder.ins().iadd(remainder, b);
                let minus = self.builder.ins().isub(remainder, b);
                let moved_q = self.builder.ins().select(positive_divisor, down, up);
                let moved_r = self.builder.ins().select(positive_divisor, plus, minus);
                (
                    self.builder
                        .ins()
                        .select(negative_remainder, moved_q, quotient),
                    self.builder
                        .ins()
                        .select(negative_remainder, moved_r, remainder),
                )
            }
        };
        match member {
            DivisionMember::Quotient => q,
            DivisionMember::Remainder => r,
        }
    }

    /// Stage an operand value for the record or tuple construction that
    /// follows.
    fn stage(&mut self, node: CheckedNode<'_>, (value, ty): (Clif, Ty)) {
        let zero = self.builder.ins().iconst(types::I64, 0);
        let kind = OpKind::Stage {
            boolean: ty == Ty::Boolean,
        };
        self.shim_call(
            node,
            kind,
            [(value, ty), (zero, Ty::Boolean)],
            Repr::Register,
        );
    }

    /// `value` of type `from` in the representation of `to`: a register
    /// `Integer` is boxed into the arena, a handle is unboxed to a register
    /// (a backend invariant stop when it does not fit, which the checked
    /// types rule out), and the same representation is unchanged.
    fn convert(
        &mut self,
        node: CheckedNode<'_>,
        (value, from): (Clif, Ty),
        to: Ty,
    ) -> Result<Clif, Failure> {
        if from.carried() == to.carried() {
            return Ok(value);
        }
        let kind = match (from, to) {
            (Ty::Int(..), Ty::Wide) => OpKind::Box,
            (Ty::Wide, Ty::Int(..)) => OpKind::Unbox,
            _ => return Err(self.unsupported(node)),
        };
        let zero = self.builder.ins().iconst(types::I64, 0);
        Ok(self.shim_call(node, kind, [(value, from), (zero, Ty::Boolean)], to.repr()))
    }

    /// Call the kernel shim for `kind` at `node`, then leave through `exit`
    /// when the shim recorded a stop (FR-561: the status is passed up
    /// unchanged).
    fn shim_call(
        &mut self,
        node: CheckedNode<'_>,
        kind: OpKind,
        [(a, left), (b, right)]: [(Clif, Ty); 2],
        result: Repr,
    ) -> Clif {
        let Ok(index) = i64::try_from(self.ops.len()) else {
            self.refused = Some("jit-operation-index");
            return a;
        };
        self.allocates |= result == Repr::Handle;
        self.ops.push(Op {
            kind,
            location: node.location().clone(),
            operands: [left.repr(), right.repr()],
            result,
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
        self.leave_on_failure();
        result
    }

    /// Return from the function when the call just made recorded a stop.
    fn leave_on_failure(&mut self) {
        let failed = self.builder.ins().load(
            types::I64,
            MemFlagsData::trusted(),
            self.context,
            FAILED_OFFSET,
        );
        let next = self.builder.create_block();
        self.builder.ins().brif(failed, self.exit, &[], next, &[]);
        self.builder.switch_to_block(next);
    }
}

/// The native interval `[low, high]` of a register operand.
fn native_interval(low: i128, high: i128) -> Result<NativeIntegerInterval, Failure> {
    NativeIntegerInterval::new(low, high).map_err(|_| fault("jit-native-interval"))
}

/// The plan of a fast-path arithmetic operation over its operand intervals.
fn native_plan(operation: NativeIntegerOperation) -> Result<NativeIntegerPlan, Failure> {
    NativeIntegerPlan::new(operation, None).map_err(|_| fault("jit-native-plan"))
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

/// Whether `operator` needs an admitted division selection it lacks: `div`
/// and `rem` run the selected law, and `mod` needs none. Such a node is the
/// interpreter's invariant stop, so it is listed unsupported at prepare, not
/// translated.
fn lacks_selection(
    operator: IntegerDivisionOperator,
    selection: Option<&AdmittedIntegerDivision>,
) -> bool {
    operator != IntegerDivisionOperator::Mod && selection.is_none()
}

/// A sound interval for the kernel's `div`, `rem` or `mod` of operands in
/// the given intervals, whose divisor interval excludes 0.
///
/// The quotient is monotone in each operand while the divisor keeps one
/// sign, so its extremes are at the operand interval ends and are found by
/// running the kernel there with an unlimited meter. The remainder is not
/// monotone (`a mod 5` over `a` in `[0, 10]` takes 0 at both ends and 4 in
/// between), so `rem` and `mod` take the interval every admitted law keeps
/// them in: strictly smaller than the divisor in magnitude. FR-562's table
/// text, which takes the extremes at the ends for all three, is unsound for
/// the remainders; this follows the soundness the rule exists for.
fn division_interval(
    operator: IntegerDivisionOperator,
    selection: Option<AdmittedIntegerDivision>,
    (al, ah): (i128, i128),
    (bl, bh): (i128, i128),
) -> Option<(i128, i128)> {
    if lacks_selection(operator, selection.as_ref()) {
        return None;
    }
    if matches!(
        operator,
        IntegerDivisionOperator::Rem | IntegerDivisionOperator::Mod
    ) {
        let bound = bl.abs().max(bh.abs()) - 1;
        return Some((-bound, bound));
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A remainder with no admitted division selection has no interval, so the
    /// entry is listed unsupported at prepare instead of faulting at run time
    /// (`jit-division-selection-missing`); `mod` needs none.
    #[test]
    fn a_division_or_remainder_without_a_selection_is_not_translated() {
        for (operator, lacks) in [
            (IntegerDivisionOperator::Div, true),
            (IntegerDivisionOperator::Rem, true),
            (IntegerDivisionOperator::Mod, false),
        ] {
            assert_eq!(lacks_selection(operator, None), lacks, "{operator:?}");
        }
        assert_eq!(
            division_interval(IntegerDivisionOperator::Div, None, (0, 10), (1, 5)),
            None
        );
    }

    #[test]
    fn a_remainder_without_a_selection_has_no_interval() {
        assert_eq!(
            division_interval(IntegerDivisionOperator::Rem, None, (0, 10), (1, 5)),
            None
        );
        assert_eq!(
            division_interval(IntegerDivisionOperator::Mod, None, (0, 10), (1, 5)),
            Some((-4, 4))
        );
    }
}
