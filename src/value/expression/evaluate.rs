// SPDX-License-Identifier: AGPL-3.0-or-later
//! The metered evaluator of checked expressions and function calls.
//!
//! Evaluation runs on an explicit task stack, so neither expression nesting
//! nor recursive call depth consumes host stack; `function.call` and every
//! per-element charge bound the work an evaluation can do. Literals,
//! parameters, `let` names, field projections, `present`, `value` and `if`
//! make no charge.

use std::cmp::Ordering;
use std::sync::Arc;

use super::super::accounting::{Charge, ChargePoint, LimitKind, Meter};
use super::super::collection::{form, form_grouped, member_equal, CollectionKind, CollectionValue};
use super::super::comparison::ComparisonOperator;
use super::super::composite::{
    retain_composite, CompositeShape, FieldValue, OptionValue, Value, ValueType,
};
use super::super::decimal::{
    evaluate_decimal, Decimal, DecimalLoss, DecimalOperation, DecimalType, RoundingMode,
};
use super::super::enumeration::compare_enum;
use super::super::equality::operand_value;
use super::super::ieee::{
    evaluate_ieee, ieee_to_exact, IeeeExactLoss, IeeeExactTarget, IeeeFlags, IeeeOperation,
};
use super::super::integer::{Integer, IntegerInterval};
use super::super::key::compare_keys;
use super::super::model_query::{evaluate_all_instances, evaluate_lookup};
use super::super::numeric::{
    evaluate_boolean, evaluate_integer_arithmetic, evaluate_rational_arithmetic, order_numbers,
    retain_boolean, ArithmeticOperator, BooleanConnective, IntegerArithmetic, OrderedOperands,
    OrderingOperator, RationalArithmetic,
};
use super::super::outcome::{Outcome, Refusal, Stop, Undefined};
use super::super::quantity::{compare_quantity, evaluate_quantity, QuantityOperation};
use super::super::rational::Rational;
use super::super::reference::ObjectEnvironment;
use super::super::text::compare_text;
use super::check::Scope;
use super::ir::{Arithmetic, Connective, Node, NodeKind, OrderedKind, RecordSlot, Slot, Visit};
use super::refusal::Location;
use crate::model::population::PopulationBinding;

/// A completed, undefined, refused or incomplete evaluation, located at the
/// expression where a non-completed outcome originated.
#[derive(Clone, Debug)]
pub struct Evaluation {
    /// The outcome.
    pub outcome: Outcome<Value>,
    /// Where a non-completed outcome originated; `None` when completed.
    pub location: Option<Location>,
    /// The loss records of the operations a completed evaluation performed,
    /// in evaluation order; empty unless completed.
    pub losses: Vec<LocatedLoss>,
}

/// Information one completed operation discarded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValueLoss {
    /// A rounded decimal operation or conversion (FR-140).
    Decimal(DecimalLoss),
    /// An IEEE-to-exact conversion (FR-148).
    IeeeExact(IeeeExactLoss),
    /// The non-empty flag set an IEEE operation raised (FR-148).
    IeeeFlags(IeeeFlags),
}

/// A loss record and the expression that produced it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatedLoss {
    /// The producing expression.
    pub location: Location,
    /// What was discarded.
    pub loss: ValueLoss,
}

/// The body a call runs.
pub(crate) struct Callable<'a> {
    pub(crate) body: &'a Node,
    pub(crate) slots: usize,
}

fn comparison(operator: OrderingOperator) -> ComparisonOperator {
    match operator {
        OrderingOperator::Less => ComparisonOperator::Less,
        OrderingOperator::LessOrEqual => ComparisonOperator::LessOrEqual,
        OrderingOperator::Greater => ComparisonOperator::Greater,
        OrderingOperator::GreaterOrEqual => ComparisonOperator::GreaterOrEqual,
    }
}

fn invariant() -> Stop {
    Stop::Refused(Refusal::CheckedInvariant)
}

/// `function.call`: one work unit per checked call.
fn charge_call(meter: &mut Meter) -> Result<(), Stop> {
    Ok(meter.charge(Charge::new(ChargePoint::FunctionCall))?)
}

/// A visit: one `collection.visit` work unit.
fn charge_visit(meter: &mut Meter) -> Result<(), Stop> {
    Ok(meter.charge(Charge::new(ChargePoint::CollectionVisit))?)
}

/// One `collection.element` before a literal element.
fn charge_element(meter: &mut Meter) -> Result<(), Stop> {
    Ok(meter.charge(Charge::new(ChargePoint::CollectionElement))?)
}

/// The declared `Int` domain of a `sum`; `None` for `Integer`.
fn sum_domain(value_type: &ValueType) -> Option<&IntegerInterval> {
    match value_type {
        ValueType::Int(domain) => Some(domain),
        _ => None,
    }
}

/// A scalar retain: `collection.result-retain` with one result unit.
fn retain_scalar(value: Value, meter: &mut Meter) -> Result<Value, Stop> {
    meter.charge(Charge::new(ChargePoint::CollectionResultRetain).results(1))?;
    Ok(value)
}

/// An accumulator retain: `collection.result-retain` with `occ(result)` value
/// occurrences and result units.
fn retain_accumulator(value: Value, meter: &mut Meter) -> Result<Value, Stop> {
    let occ = value.occ();
    meter.charge(
        Charge::new(ChargePoint::CollectionResultRetain)
            .exact_size(LimitKind::ValueOccurrences, occ.clone())
            .exact_results(occ),
    )?;
    Ok(value)
}

enum Task<'a> {
    Eval(&'a Node),
    Apply(&'a Node),
    Branch(&'a Node),
    Short(&'a Node),
    Retain(&'a Node),
    Bind(Slot),
    ChargeElement(&'a Node),
    Return,
    Iterate(Box<Iteration<'a>>),
    /// Restore the anchor `pre(..)` had saved before evaluating its operand.
    RestoreAnchor(Anchor),
}

/// Which population an `allInstances`/`lookup` reads: the ambient post
/// population, or (underneath a `pre(..)`) the invocation's pre population.
/// FR-153.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Anchor {
    Post,
    Pre,
}

/// A one-binder query or fold in progress.
struct Iteration<'a> {
    node: &'a Node,
    source: Arc<CollectionValue>,
    next: usize,
    awaiting: bool,
    results: Vec<Value>,
    accumulator: Option<Value>,
    count: Integer,
}

pub(crate) struct Machine<'a, 'm> {
    scope: &'a Scope,
    functions: &'a [Callable<'a>],
    objects: &'a ObjectEnvironment,
    meter: &'m mut Meter,
    values: Vec<Value>,
    frames: Vec<Vec<Option<Value>>>,
    tasks: Vec<Task<'a>>,
    losses: Vec<LocatedLoss>,
    /// The anchor `allInstances`/`lookup` currently read; `pre(..)` toggles
    /// it for its operand and `Task::RestoreAnchor` restores it after.
    anchor: Anchor,
}

impl<'a, 'm> Machine<'a, 'm> {
    pub(crate) fn new(
        scope: &'a Scope,
        functions: &'a [Callable<'a>],
        objects: &'a ObjectEnvironment,
        meter: &'m mut Meter,
    ) -> Self {
        Self {
            scope,
            functions,
            objects,
            meter,
            values: Vec::new(),
            frames: Vec::new(),
            tasks: Vec::new(),
            losses: Vec::new(),
            anchor: Anchor::Post,
        }
    }

    /// Evaluate `root` with `arguments` in its first slots. With `call`, the
    /// root is a function body and `function.call` is charged first.
    pub(crate) fn run(
        mut self,
        root: &'a Node,
        slots: usize,
        arguments: Vec<Value>,
        call: bool,
    ) -> Evaluation {
        let mut frame: Vec<Option<Value>> = arguments.into_iter().map(Some).collect();
        frame.resize(slots.max(frame.len()), None);
        self.frames.push(frame);
        if call {
            if let Err(stop) = charge_call(self.meter) {
                return Self::stopped(stop, &root.location);
            }
        }
        self.tasks.push(Task::Eval(root));
        while let Some(task) = self.tasks.pop() {
            let location = match &task {
                Task::Eval(node)
                | Task::Apply(node)
                | Task::Branch(node)
                | Task::Short(node)
                | Task::Retain(node)
                | Task::ChargeElement(node) => Some(&node.location),
                Task::Iterate(iteration) => Some(&iteration.node.location),
                Task::Bind(_) | Task::Return | Task::RestoreAnchor(_) => None,
            };
            let location = location.cloned().unwrap_or_else(|| root.location.clone());
            if let Err(stop) = self.step(task) {
                return Self::stopped(stop, &location);
            }
        }
        match (self.values.pop(), self.values.is_empty()) {
            (Some(value), true) => Evaluation {
                outcome: Outcome::Completed(value),
                location: None,
                losses: self.losses,
            },
            _ => Self::stopped(invariant(), &root.location),
        }
    }

    fn stopped(stop: Stop, location: &Location) -> Evaluation {
        Evaluation {
            outcome: Outcome::from_stop(Err(stop)),
            location: Some(location.clone()),
            losses: Vec::new(),
        }
    }

    fn pop(&mut self) -> Result<Value, Stop> {
        self.values.pop().ok_or_else(invariant)
    }

    fn pop_many(&mut self, count: usize) -> Result<Vec<Value>, Stop> {
        let start = self.values.len().checked_sub(count).ok_or_else(invariant)?;
        Ok(self.values.split_off(start))
    }

    fn pop_integer(&mut self) -> Result<Integer, Stop> {
        match self.pop()? {
            Value::Integer(value) => Ok(value),
            _ => Err(invariant()),
        }
    }

    fn pop_boolean(&mut self) -> Result<bool, Stop> {
        match self.pop()? {
            Value::Boolean(value) => Ok(value),
            _ => Err(invariant()),
        }
    }

    fn pop_collection(&mut self) -> Result<Arc<CollectionValue>, Stop> {
        match self.pop()? {
            Value::Collection(collection) => Ok(collection),
            _ => Err(invariant()),
        }
    }

    fn pop_rational(&mut self) -> Result<Rational, Stop> {
        match self.pop()? {
            Value::Rational(value) => Ok(value),
            _ => Err(invariant()),
        }
    }

    fn pop_decimal(&mut self) -> Result<Decimal, Stop> {
        match self.pop()? {
            Value::Decimal(value) => Ok(value),
            _ => Err(invariant()),
        }
    }

    fn record(&mut self, node: &Node, loss: ValueLoss) {
        self.losses.push(LocatedLoss {
            location: node.location.clone(),
            loss,
        });
    }

    /// Evaluate a decimal operation into `target`, recording its loss.
    fn decimal(
        &mut self,
        node: &Node,
        operation: DecimalOperation<'_>,
        target: &DecimalType,
    ) -> Result<Value, Stop> {
        let result = evaluate_decimal(operation, target, self.meter).into_stop()?;
        if let Some(loss) = result.loss() {
            self.record(node, ValueLoss::Decimal(loss.clone()));
        }
        Ok(Value::Decimal(result.value().clone()))
    }

    fn slot(&mut self, slot: Slot) -> Result<&mut Option<Value>, Stop> {
        self.frames
            .last_mut()
            .and_then(|frame| frame.get_mut(slot))
            .ok_or_else(invariant)
    }

    /// FR-153: the population `allInstances`/`lookup` reads for `binding` at
    /// the current anchor -- `binding` itself when reading the ambient post
    /// population, or `binding`'s attached pre population underneath
    /// `pre(..)`. A `Pre` anchor with no attached pre population is a
    /// checked invariant: the checker only admits `pre(..)` where the
    /// population operand carries one (a postcondition's parameter, bound
    /// to an admitted invocation).
    fn select_anchor<'x>(
        &self,
        binding: &'x PopulationBinding,
    ) -> Result<&'x PopulationBinding, Stop> {
        match self.anchor {
            Anchor::Post => Ok(binding),
            Anchor::Pre => binding.pre_anchor().ok_or_else(invariant),
        }
    }

    fn step(&mut self, task: Task<'a>) -> Result<(), Stop> {
        match task {
            Task::Eval(node) => self.eval(node),
            Task::Apply(node) => self.apply(node),
            Task::Branch(node) => {
                let NodeKind::If {
                    then, otherwise, ..
                } = &node.kind
                else {
                    return Err(invariant());
                };
                let taken = if self.pop_boolean()? { then } else { otherwise };
                self.tasks.push(Task::Eval(taken));
                Ok(())
            }
            Task::Short(node) => {
                let NodeKind::Connective(connective, _, right) = &node.kind else {
                    return Err(invariant());
                };
                let left = self.pop_boolean()?;
                let skipped = match connective {
                    Connective::And => (!left).then_some(false),
                    Connective::Or => left.then_some(true),
                    Connective::Implies => (!left).then_some(true),
                };
                match skipped {
                    Some(result) => {
                        let result = retain_boolean(result, self.meter)?;
                        self.values.push(Value::Boolean(result));
                    }
                    None => {
                        self.tasks.push(Task::Retain(node));
                        self.tasks.push(Task::Eval(right));
                    }
                }
                Ok(())
            }
            Task::Retain(_) => {
                let result = retain_boolean(self.pop_boolean()?, self.meter)?;
                self.values.push(Value::Boolean(result));
                Ok(())
            }
            Task::Bind(slot) => {
                let value = self.pop()?;
                *self.slot(slot)? = Some(value);
                Ok(())
            }
            Task::ChargeElement(_) => charge_element(self.meter),
            Task::Return => {
                self.frames.pop().ok_or_else(invariant)?;
                Ok(())
            }
            Task::Iterate(iteration) => self.iterate(iteration),
            Task::RestoreAnchor(previous) => {
                self.anchor = previous;
                Ok(())
            }
        }
    }

    fn eval(&mut self, node: &'a Node) -> Result<(), Stop> {
        match &node.kind {
            NodeKind::Literal(value) => {
                self.values.push(value.clone());
                return Ok(());
            }
            NodeKind::Local(slot) => {
                let value = self.slot(*slot)?.clone().ok_or_else(invariant)?;
                self.values.push(value);
                return Ok(());
            }
            NodeKind::Let { slot, value, body } => {
                self.tasks.push(Task::Eval(body));
                self.tasks.push(Task::Bind(*slot));
                self.tasks.push(Task::Eval(value));
                return Ok(());
            }
            NodeKind::If { condition, .. } => {
                self.tasks.push(Task::Branch(node));
                self.tasks.push(Task::Eval(condition));
                return Ok(());
            }
            NodeKind::Connective(_, left, _) => {
                self.tasks.push(Task::Short(node));
                self.tasks.push(Task::Eval(left));
                return Ok(());
            }
            NodeKind::Collection { elements, .. } => {
                self.tasks.push(Task::Apply(node));
                for element in elements.iter().rev() {
                    self.tasks.push(Task::Eval(element));
                    self.tasks.push(Task::ChargeElement(node));
                }
                return Ok(());
            }
            NodeKind::Query { source, .. } => {
                self.tasks.push(Task::Apply(node));
                self.tasks.push(Task::Eval(source));
                return Ok(());
            }
            NodeKind::Fold {
                source, identity, ..
            } => {
                self.tasks.push(Task::Apply(node));
                if let Some(identity) = identity {
                    self.tasks.push(Task::Eval(identity));
                }
                self.tasks.push(Task::Eval(source));
                return Ok(());
            }
            NodeKind::Pre(operand) => {
                // FR-153: read the invocation pre population for every
                // `allInstances`/`lookup` under `operand`, then restore.
                // Identity-typed, so no `Task::Apply` follows: `operand`'s
                // own value is `pre(operand)`'s value.
                self.tasks.push(Task::RestoreAnchor(self.anchor));
                self.anchor = Anchor::Pre;
                self.tasks.push(Task::Eval(operand));
                return Ok(());
            }
            _ => {}
        }
        self.tasks.push(Task::Apply(node));
        for child in node.children().into_iter().rev() {
            self.tasks.push(Task::Eval(child));
        }
        Ok(())
    }

    fn apply(&mut self, node: &'a Node) -> Result<(), Stop> {
        let value = match &node.kind {
            NodeKind::Literal(_)
            | NodeKind::Local(_)
            | NodeKind::Let { .. }
            | NodeKind::If { .. }
            | NodeKind::Connective(..)
            | NodeKind::Pre(_) => return Err(invariant()),
            NodeKind::Coerce(_, target) => {
                let value = self.pop_integer()?;
                if !target.contains(&value) {
                    return Err(Stop::Refused(Refusal::IntegerOutOfDomain));
                }
                Value::Integer(value)
            }
            NodeKind::Arithmetic(operator, _, _) => {
                let right = self.pop_integer()?;
                let left = self.pop_integer()?;
                let operation = match operator {
                    Arithmetic::Add => IntegerArithmetic::Add(&left, &right),
                    Arithmetic::Subtract => IntegerArithmetic::Subtract(&left, &right),
                    Arithmetic::Multiply => IntegerArithmetic::Multiply(&left, &right),
                };
                Value::Integer(
                    evaluate_integer_arithmetic(operation, None, self.meter).into_stop()?,
                )
            }
            NodeKind::Negate(_) => {
                let operand = self.pop_integer()?;
                Value::Integer(
                    evaluate_integer_arithmetic(
                        IntegerArithmetic::Negate(&operand),
                        None,
                        self.meter,
                    )
                    .into_stop()?,
                )
            }
            NodeKind::Divide { domain, .. } => {
                let right = Rational::from_integer(self.pop_integer()?);
                let left = Rational::from_integer(self.pop_integer()?);
                Value::Rational(
                    evaluate_rational_arithmetic(
                        RationalArithmetic::Divide(&left, &right),
                        Some(domain),
                        self.meter,
                    )
                    .into_stop()?,
                )
            }
            NodeKind::Rational {
                operator, domain, ..
            } => {
                let right = self.pop_rational()?;
                let left = self.pop_rational()?;
                let operation = match operator {
                    ArithmeticOperator::Add => RationalArithmetic::Add(&left, &right),
                    ArithmeticOperator::Subtract => RationalArithmetic::Subtract(&left, &right),
                    ArithmeticOperator::Multiply => RationalArithmetic::Multiply(&left, &right),
                    ArithmeticOperator::Divide => RationalArithmetic::Divide(&left, &right),
                };
                Value::Rational(
                    evaluate_rational_arithmetic(operation, Some(domain), self.meter)
                        .into_stop()?,
                )
            }
            NodeKind::RationalNegate(_, domain) => {
                let operand = self.pop_rational()?;
                Value::Rational(
                    evaluate_rational_arithmetic(
                        RationalArithmetic::Negate(&operand),
                        Some(domain),
                        self.meter,
                    )
                    .into_stop()?,
                )
            }
            NodeKind::Decimal {
                operator, target, ..
            } => {
                let right = self.pop_decimal()?;
                let left = self.pop_decimal()?;
                let operation = match operator {
                    ArithmeticOperator::Add => DecimalOperation::Add(&left, &right),
                    ArithmeticOperator::Subtract => DecimalOperation::Subtract(&left, &right),
                    ArithmeticOperator::Multiply => DecimalOperation::Multiply(&left, &right),
                    ArithmeticOperator::Divide => DecimalOperation::Divide(&left, &right),
                };
                self.decimal(node, operation, target)?
            }
            NodeKind::DecimalNegate(_, target) => {
                let operand = self.pop_decimal()?;
                self.decimal(node, DecimalOperation::Negate(&operand), target)?
            }
            NodeKind::ConvertDecimal(_, target) => match self.pop()? {
                Value::Decimal(source) => {
                    self.decimal(node, DecimalOperation::Round(&source), target)?
                }
                Value::Rational(source) => {
                    let numerator = Decimal::new(source.numerator().clone(), 0);
                    let denominator = Decimal::new(source.denominator().clone(), 0);
                    self.decimal(
                        node,
                        DecimalOperation::Divide(&numerator, &denominator),
                        target,
                    )?
                }
                _ => return Err(invariant()),
            },
            NodeKind::Ieee(operator, _, _) => {
                let (Value::Float(right), Value::Float(left)) = (self.pop()?, self.pop()?) else {
                    return Err(invariant());
                };
                let operation = match operator {
                    ArithmeticOperator::Add => IeeeOperation::Add(left, right),
                    ArithmeticOperator::Subtract => IeeeOperation::Subtract(left, right),
                    ArithmeticOperator::Multiply => IeeeOperation::Multiply(left, right),
                    ArithmeticOperator::Divide => IeeeOperation::Divide(left, right),
                };
                let profile = self.scope.ieee_profile.as_ref().ok_or_else(invariant)?;
                let result = evaluate_ieee(profile, operation, RoundingMode::Exact, self.meter)
                    .map_err(|_| invariant())?
                    .into_stop()?;
                if result.flags() != IeeeFlags::EMPTY {
                    self.record(node, ValueLoss::IeeeFlags(result.flags()));
                }
                Value::Float(result.value())
            }
            NodeKind::Quantity(operator, _, _) => {
                let (Value::Quantity(right), Value::Quantity(left)) = (self.pop()?, self.pop()?)
                else {
                    return Err(invariant());
                };
                let operation = match operator {
                    ArithmeticOperator::Add => QuantityOperation::Add(&left, &right),
                    ArithmeticOperator::Subtract => QuantityOperation::Subtract(&left, &right),
                    ArithmeticOperator::Multiply => QuantityOperation::Multiply(&left, &right),
                    ArithmeticOperator::Divide => QuantityOperation::Divide(&left, &right),
                };
                Value::Quantity(
                    evaluate_quantity(operation, self.meter)
                        .map_err(|_| invariant())?
                        .into_stop()?,
                )
            }
            NodeKind::Order(operator, kind, _, _) => {
                let right = self.pop()?;
                let left = self.pop()?;
                Value::Boolean(self.order(*operator, *kind, &left, &right)?)
            }
            NodeKind::Equality(_, checked, _, _) => {
                let right = self.pop()?;
                let left = self.pop()?;
                Value::Boolean(checked.evaluate(&left, &right, self.meter).into_stop()?)
            }
            NodeKind::Not(_) => {
                let operand = self.pop_boolean()?;
                Value::Boolean(
                    evaluate_boolean(BooleanConnective::Not(operand), self.meter).into_stop()?,
                )
            }
            NodeKind::Field {
                index, optional, ..
            } => {
                let Value::Composite(composite) = self.pop()? else {
                    return Err(invariant());
                };
                let slot = composite.slots().get(*index).ok_or_else(invariant)?;
                Self::project(slot, *optional, &node.value_type)?
            }
            NodeKind::Attribute { name, optional, .. } => {
                let Value::Reference(reference) = self.pop()? else {
                    return Err(invariant());
                };
                let slot = self
                    .objects
                    .attribute(&self.scope.types, &reference, name)
                    .ok_or_else(invariant)?;
                Self::project(slot, *optional, &node.value_type)?
            }
            NodeKind::Present(_) => {
                let Value::Option(option) = self.pop()? else {
                    return Err(invariant());
                };
                Value::Boolean(option.payload().is_some())
            }
            NodeKind::Value(_) => {
                let Value::Option(option) = self.pop()? else {
                    return Err(invariant());
                };
                option
                    .payload()
                    .cloned()
                    .ok_or(Stop::Undefined(Undefined::NoneValue))?
            }
            NodeKind::Call {
                function,
                arguments,
            } => {
                let callable = self.functions.get(*function).ok_or_else(invariant)?;
                let arguments = self.pop_many(arguments.len())?;
                charge_call(self.meter)?;
                let mut frame: Vec<Option<Value>> = arguments.into_iter().map(Some).collect();
                frame.resize(callable.slots.max(frame.len()), None);
                self.frames.push(frame);
                self.tasks.push(Task::Return);
                self.tasks.push(Task::Eval(callable.body));
                return Ok(());
            }
            NodeKind::Tuple {
                declaration,
                arguments,
            } => {
                let arguments = self.pop_many(arguments.len())?;
                let value = self
                    .scope
                    .types
                    .tuple(*declaration, arguments)
                    .map_err(|_| invariant())?;
                retain_composite(value, self.meter)?
            }
            NodeKind::Record { declaration, slots } => {
                let present = slots
                    .iter()
                    .filter(|slot| matches!(slot, RecordSlot::Present(_)))
                    .count();
                let mut values = self.pop_many(present)?.into_iter();
                let Some(CompositeShape::Record(declared)) = self
                    .scope
                    .types
                    .composite(*declaration)
                    .map(|declaration| declaration.shape())
                else {
                    return Err(invariant());
                };
                let mut fields = Vec::with_capacity(slots.len());
                for (slot, field) in slots.iter().zip(declared) {
                    let value = match slot {
                        RecordSlot::Absent => FieldValue::Absent,
                        RecordSlot::Null => FieldValue::Null,
                        RecordSlot::Present(_) => {
                            FieldValue::Present(values.next().ok_or_else(invariant)?)
                        }
                    };
                    fields.push((field.name(), value));
                }
                let value = self
                    .scope
                    .types
                    .record(*declaration, fields)
                    .map_err(|_| invariant())?;
                retain_composite(value, self.meter)?
            }
            NodeKind::Collection {
                collection_type,
                elements,
            } => {
                let occurrences = self.pop_many(elements.len())?;
                form(collection_type, occurrences, self.meter)?
            }
            NodeKind::ConvertCollection { target, .. } => {
                let source = self.pop_collection()?;
                let source_kind = source.collection_type().kind();
                let distinct = source_kind == CollectionKind::Bag && target.kind().is_unique();
                let mut produced: Vec<Value> = Vec::with_capacity(source.elements().len());
                for element in source.elements() {
                    let repeated = distinct
                        && produced.last().is_some_and(|previous| {
                            compare_keys(previous, element) == Some(Ordering::Equal)
                        });
                    if repeated {
                        continue;
                    }
                    charge_visit(self.meter)?;
                    produced.push(element.clone());
                }
                if source_kind == CollectionKind::Sequence
                    && target.kind() != CollectionKind::Sequence
                {
                    form(target, produced, self.meter)?
                } else {
                    form_grouped(target, produced, self.meter)?
                }
            }
            NodeKind::ConvertScalar(operand, _) => {
                let value = self.pop()?;
                operand_value(operand, &value, self.meter)?
            }
            NodeKind::IeeeToRational(_, domain) => {
                let Value::Float(value) = self.pop()? else {
                    return Err(invariant());
                };
                let profile = self.scope.ieee_profile.as_ref().ok_or_else(invariant)?;
                let exact = ieee_to_exact(
                    profile,
                    value,
                    IeeeExactTarget::Rational(domain),
                    self.meter,
                )
                .map_err(|_| invariant())?
                .into_stop()?;
                if let Some(loss) = exact.loss() {
                    self.record(node, ValueLoss::IeeeExact(loss));
                }
                Value::Rational(exact.value().clone())
            }
            NodeKind::Query { .. } | NodeKind::Fold { .. } => {
                return self.start_iteration(node);
            }
            NodeKind::Flatten(_) => {
                let ValueType::Collection(result_type) = &node.value_type else {
                    return Err(invariant());
                };
                let outer = self.pop_collection()?;
                let mut produced = Vec::new();
                for inner in outer.elements() {
                    charge_visit(self.meter)?;
                    let Value::Collection(inner) = inner else {
                        return Err(invariant());
                    };
                    for element in inner.elements() {
                        charge_visit(self.meter)?;
                        produced.push(element.clone());
                    }
                }
                form(result_type, produced, self.meter)?
            }
            NodeKind::Size(_) => {
                let collection = self.pop_collection()?;
                retain_scalar(
                    Value::Integer(Integer::from(collection.elements().len())),
                    self.meter,
                )?
            }
            NodeKind::Contains(_, _) => {
                let item = self.pop()?;
                let collection = self.pop_collection()?;
                let bag = collection.collection_type().kind() == CollectionKind::Bag;
                let mut found = false;
                let mut previous: Option<&Value> = None;
                for member in collection.elements() {
                    let repeated = bag
                        && previous.is_some_and(|previous| {
                            compare_keys(previous, member) == Some(Ordering::Equal)
                        });
                    previous = Some(member);
                    if repeated {
                        continue;
                    }
                    if member_equal(&item, member, self.meter)? {
                        found = true;
                        break;
                    }
                }
                retain_scalar(Value::Boolean(found), self.meter)?
            }
            NodeKind::AllInstances { .. } => {
                let Value::Population(binding) = self.pop()? else {
                    return Err(invariant());
                };
                let ValueType::Collection(collection_type) = &node.value_type else {
                    return Err(invariant());
                };
                let binding = self.select_anchor(&binding)?;
                evaluate_all_instances(binding, collection_type, self.meter)?
            }
            NodeKind::Lookup {
                reference, absence, ..
            } => {
                let reference_value = self.pop()?;
                let Value::Population(binding) = self.pop()? else {
                    return Err(invariant());
                };
                let ValueType::Reference(static_key) = &reference.value_type else {
                    return Err(invariant());
                };
                let target_key = match &node.value_type {
                    ValueType::Reference(key) => *key,
                    ValueType::Option(payload) => match &**payload {
                        ValueType::Reference(key) => *key,
                        _ => return Err(invariant()),
                    },
                    _ => return Err(invariant()),
                };
                let binding = self.select_anchor(&binding)?;
                evaluate_lookup(
                    binding,
                    target_key,
                    *static_key,
                    reference_value,
                    *absence,
                    &node.value_type,
                    self.meter,
                )?
            }
        };
        self.values.push(value);
        Ok(())
    }

    fn project(slot: &FieldValue, optional: bool, value_type: &ValueType) -> Result<Value, Stop> {
        match (slot, optional, value_type) {
            (FieldValue::Present(value), false, _) => Ok(value.clone()),
            (FieldValue::Present(value), true, ValueType::Option(payload)) => {
                OptionValue::present((**payload).clone(), value.clone()).map_err(|_| invariant())
            }
            (FieldValue::Absent | FieldValue::Null, true, ValueType::Option(payload)) => {
                Ok(OptionValue::none((**payload).clone()))
            }
            _ => Err(invariant()),
        }
    }

    fn order(
        &mut self,
        operator: OrderingOperator,
        kind: OrderedKind,
        left: &Value,
        right: &Value,
    ) -> Result<bool, Stop> {
        let operands = match (kind, left, right) {
            (OrderedKind::Integers, Value::Integer(l), Value::Integer(r)) => {
                OrderedOperands::Integers(l, r)
            }
            (OrderedKind::Rationals, Value::Rational(l), Value::Rational(r)) => {
                OrderedOperands::Rationals(l, r)
            }
            (OrderedKind::Decimals, Value::Decimal(l), Value::Decimal(r)) => {
                OrderedOperands::Decimals(l, r)
            }
            (OrderedKind::Enums, Value::Enum(l), Value::Enum(r)) => {
                return compare_enum(comparison(operator), l, r, self.meter)
                    .map_err(|_| invariant())?
                    .into_stop();
            }
            (OrderedKind::Texts, Value::Text(l), Value::Text(r)) => {
                return compare_text(comparison(operator), l, r, self.meter)
                    .map_err(|_| invariant())?
                    .into_stop();
            }
            (OrderedKind::Quantities, Value::Quantity(l), Value::Quantity(r)) => {
                return compare_quantity(comparison(operator), l, r, self.meter)
                    .map_err(|_| invariant())?
                    .into_stop();
            }
            _ => return Err(invariant()),
        };
        order_numbers(operator, operands, self.meter).into_stop()
    }

    fn start_iteration(&mut self, node: &'a Node) -> Result<(), Stop> {
        let (identity, reduce) = match &node.kind {
            NodeKind::Fold { identity, .. } => (
                match identity {
                    Some(_) => Some(self.pop()?),
                    None => None,
                },
                identity.is_none(),
            ),
            _ => (None, false),
        };
        let source = self.pop_collection()?;
        let mut iteration = Box::new(Iteration {
            node,
            source,
            next: 0,
            awaiting: false,
            results: Vec::new(),
            accumulator: identity,
            count: Integer::zero(),
        });
        if reduce {
            let first = iteration
                .source
                .elements()
                .first()
                .cloned()
                .ok_or(Stop::Undefined(Undefined::EmptyReduction))?;
            charge_visit(self.meter)?;
            iteration.accumulator = Some(first);
            iteration.next = 1;
        }
        self.iterate(iteration)
    }

    fn iterate(&mut self, mut iteration: Box<Iteration<'a>>) -> Result<(), Stop> {
        let node = iteration.node;
        if iteration.awaiting {
            iteration.awaiting = false;
            let result = self.pop()?;
            let element = iteration
                .next
                .checked_sub(1)
                .and_then(|index| iteration.source.elements().get(index))
                .cloned()
                .ok_or_else(invariant)?;
            let stop_early = match &node.kind {
                NodeKind::Query { visit, .. } => match (visit, result) {
                    (Visit::Map, value) => {
                        iteration.results.push(value);
                        false
                    }
                    (Visit::Filter, Value::Boolean(kept)) => {
                        if kept {
                            iteration.results.push(element);
                        }
                        false
                    }
                    (Visit::Forall, Value::Boolean(holds)) => !holds,
                    (Visit::Exists, Value::Boolean(holds)) => holds,
                    (Visit::Sum, Value::Integer(summand)) => {
                        iteration.accumulator =
                            Some(Value::Integer(match iteration.accumulator.take() {
                                None => summand,
                                Some(Value::Integer(total)) => evaluate_integer_arithmetic(
                                    IntegerArithmetic::Add(&total, &summand),
                                    sum_domain(&node.value_type),
                                    self.meter,
                                )
                                .into_stop()?,
                                Some(_) => return Err(invariant()),
                            }));
                        false
                    }
                    (Visit::Count, Value::Boolean(holds)) => {
                        if holds {
                            iteration.count = iteration.count.add(&Integer::one());
                        }
                        false
                    }
                    _ => return Err(invariant()),
                },
                NodeKind::Fold { .. } => {
                    iteration.accumulator = Some(result);
                    false
                }
                _ => return Err(invariant()),
            };
            if stop_early {
                let found = matches!(
                    node.kind,
                    NodeKind::Query {
                        visit: Visit::Exists,
                        ..
                    }
                );
                let value = retain_scalar(Value::Boolean(found), self.meter)?;
                self.values.push(value);
                return Ok(());
            }
        }
        let Some(element) = iteration.source.elements().get(iteration.next).cloned() else {
            return self.finish(*iteration);
        };
        charge_visit(self.meter)?;
        iteration.next = iteration.next.saturating_add(1);
        let body = match &node.kind {
            NodeKind::Query { slot, body, .. } => {
                *self.slot(*slot)? = Some(element);
                body
            }
            NodeKind::Fold {
                accumulator,
                binder,
                step,
                ..
            } => {
                let current = iteration.accumulator.clone().ok_or_else(invariant)?;
                *self.slot(*accumulator)? = Some(current);
                *self.slot(*binder)? = Some(element);
                step
            }
            _ => return Err(invariant()),
        };
        iteration.awaiting = true;
        self.tasks.push(Task::Iterate(iteration));
        self.tasks.push(Task::Eval(body));
        Ok(())
    }

    fn finish(&mut self, iteration: Iteration<'a>) -> Result<(), Stop> {
        let node = iteration.node;
        let value = match &node.kind {
            NodeKind::Query { visit, .. } => match visit {
                Visit::Map | Visit::Filter => {
                    let ValueType::Collection(result_type) = &node.value_type else {
                        return Err(invariant());
                    };
                    if *visit == Visit::Map {
                        form(result_type, iteration.results, self.meter)?
                    } else {
                        form_grouped(result_type, iteration.results, self.meter)?
                    }
                }
                Visit::Forall => retain_scalar(Value::Boolean(true), self.meter)?,
                Visit::Exists => retain_scalar(Value::Boolean(false), self.meter)?,
                Visit::Sum => {
                    let total = match iteration.accumulator {
                        None => Integer::zero(),
                        Some(Value::Integer(total)) => total,
                        Some(_) => return Err(invariant()),
                    };
                    if sum_domain(&node.value_type).is_some_and(|domain| !domain.contains(&total)) {
                        return Err(Stop::Refused(Refusal::IntegerOutOfDomain));
                    }
                    retain_scalar(Value::Integer(total), self.meter)?
                }
                Visit::Count => {
                    if let ValueType::Int(domain) = &node.value_type {
                        if !domain.contains(&iteration.count) {
                            return Err(Stop::Refused(Refusal::IntegerOutOfDomain));
                        }
                    }
                    retain_scalar(Value::Integer(iteration.count), self.meter)?
                }
            },
            NodeKind::Fold { .. } => {
                let accumulator = iteration.accumulator.ok_or_else(invariant)?;
                retain_accumulator(accumulator, self.meter)?
            }
            _ => return Err(invariant()),
        };
        self.values.push(value);
        Ok(())
    }
}
