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

use super::super::collection::{form, form_grouped, member_equal, CollectionValue};
use super::super::composite::{
    retain_composite, CompositeShape, FieldValue, OptionValue, Value, ValueType,
};
use super::super::decimal::{evaluate_decimal, DecimalLoss, DecimalType};
use super::super::enumeration::compare_enum;
use super::super::equality::operand_value;
use super::super::ieee::{evaluate_ieee, ieee_to_exact, IeeeExactTarget};
use super::super::key::compare_keys;
use super::super::model_query::{evaluate_all_instances, evaluate_lookup};
use super::super::numeric::{
    evaluate_boolean, evaluate_integer_arithmetic, evaluate_rational_arithmetic, order_numbers,
    retain_boolean,
};
use super::super::outcome::{Outcome, PreconditionFailure, Refusal, Stop, Undefined};
use super::super::quantity::{compare_quantity, evaluate_quantity, QuantityOperation};
use super::super::reference::ObjectEnvironment;
use super::super::text::compare_text;
use crate::check::{
    Arithmetic, Connective, DispatchTable, Location, Node, NodeKind, OrderedKind, RecordSlot,
    Scope, Slot, Visit, WrongSnapshotCause,
};
use crate::model::population::PopulationBinding;
use qsl_foundation::diagnostic::InternalFault;
use quire_exact::ComparisonOperator;
use quire_exact::Rational;
use quire_exact::{
    ArithmeticOperator, BooleanConnective, IntegerArithmetic, OrderedOperands, OrderingOperator,
    RationalArithmetic,
};
use quire_exact::{
    Charge, ChargePoint, CollectionKind, Incomplete, Integer, IntegerInterval, LimitKind, Meter,
    PopulationId,
};
use quire_exact::{Decimal, DecimalOperation, RoundingMode};
use quire_exact::{IeeeExactLoss, IeeeFlags, IeeeOperation};

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
    /// The declared name, needed for the FR-151 `precondition-false` payload
    /// (`native-diagnostics.md`: "the selected method's effective identity").
    pub(crate) name: &'a str,
}

fn comparison(operator: OrderingOperator) -> ComparisonOperator {
    match operator {
        OrderingOperator::Less => ComparisonOperator::Less,
        OrderingOperator::LessOrEqual => ComparisonOperator::LessOrEqual,
        OrderingOperator::Greater => ComparisonOperator::Greater,
        OrderingOperator::GreaterOrEqual => ComparisonOperator::GreaterOrEqual,
    }
}

/// `Machine`'s own early-exit carrier: either a real evaluator [`Stop`], or
/// an S6a invariant break `Stop` itself cannot represent (PR #334 review
/// round 2, finding N1). Crate-private to this module alone -- nothing
/// outside `Machine` ever builds, matches or forwards one. Only
/// [`Machine::run`] unwraps a `Halt`, and only its `Fault` arm returns
/// `Err(InternalFault)`; every `Stop` arm still goes through
/// [`Machine::stopped`]/[`Outcome::from_stop`] exactly as before this change.
/// A future `Stop`-returning helper, or a new site inside `Machine`, cannot
/// smuggle a fault into `Outcome::from_stop`'s three real arms: there is no
/// `Stop` variant left to build.
enum Halt {
    /// An ordinary evaluator stop, to be converted to an `Outcome` as usual.
    Stop(Stop),
    /// An S6a invariant break: never converted to an `Outcome`.
    Fault(InternalFault),
}

impl From<Stop> for Halt {
    fn from(stop: Stop) -> Self {
        Self::Stop(stop)
    }
}

impl From<Incomplete> for Halt {
    fn from(record: Incomplete) -> Self {
        Self::Stop(Stop::Incomplete(record))
    }
}

/// The real `Stop` a checked-invariant break constructs -- shared by
/// [`invariant`] (for a `Halt`-returning `Machine` method) and
/// [`Machine::run`]'s own no-value fallback (which calls [`Machine::stopped`]
/// directly, needing a bare `Stop`, never a `Halt`).
fn checked_invariant() -> Stop {
    Stop::Refused(Refusal::CheckedInvariant)
}

fn invariant() -> Halt {
    Halt::Stop(checked_invariant())
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

/// `dispatch.select`: one dispatched `receiver.member(args)` call, sized by
/// `candidates`, the table's total distinct-candidate count
/// (`value-accounting.md`: `value_occurrences=c`; `work_units += c`).
fn charge_dispatch_select(meter: &mut Meter, candidates: u64) -> Result<(), Stop> {
    Ok(meter.charge(
        Charge::new(ChargePoint::DispatchSelect)
            .size(LimitKind::ValueOccurrences, candidates)
            .work(Integer::from(candidates)),
    )?)
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
    /// FR-151 (TC-196 D06): the pushed frame's own effective-precondition
    /// body has just evaluated; decide it, then continue to the selected
    /// candidate's own body.
    DispatchGuard(Box<DispatchGuard>),
    /// Restore the anchor a `pre(..)` or a `Call` saved before evaluating its
    /// operand/callee body.
    RestoreAnchor(Anchor),
}

/// A dispatched call's linked candidate body, awaiting its effective
/// precondition's decision (TC-196 D06).
struct DispatchGuard {
    body_function: usize,
    arguments: Vec<Value>,
    /// The `precondition-false` payload to report if the guard fails.
    failure: PreconditionFailure,
}

/// Which population an `allInstances`/`lookup` reads: the ambient post
/// population, or (underneath a `pre(..)`) the invocation's pre population.
/// FR-153. Switching this counter is a structural evaluator decision, like
/// the closure/foreign checks `value-accounting.md`'s "Model and graph
/// evaluation" paragraph already names as making no evaluation charge; that
/// document defines no charge point for an anchor at all, so `Task::
/// RestoreAnchor` and every `self.anchor` assignment below are uncharged by
/// the conservative reading the same paragraph already establishes for this
/// module's other structural decisions.
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
    dispatch_tables: &'a [DispatchTable],
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
        dispatch_tables: &'a [DispatchTable],
    ) -> Self {
        Self {
            scope,
            functions,
            objects,
            meter,
            dispatch_tables,
            values: Vec::new(),
            frames: Vec::new(),
            tasks: Vec::new(),
            losses: Vec::new(),
            anchor: Anchor::Post,
        }
    }

    /// Evaluate `root` with `arguments` in its first slots.
    ///
    /// **No entry-level `function.call` charge here (PR #302 review finding
    /// 2).** An earlier version took a `call: bool` and charged
    /// `function.call` against `self.meter` (`env.local_meter`) once, up
    /// front, whenever the root was a function body -- the exact same named
    /// charge `ValueFunctionFamily::evaluate` (`family.rs`) now charges
    /// against its own contract-level `meter` parameter for that same
    /// top-level call, against a *different* meter instance. Charging the
    /// same point twice for one logical call, even against two different
    /// meters, is not two real facts -- it is one fact restated twice.
    /// `evaluate`'s own charge is the top-level call's sole admission
    /// charge now; every *nested* `NodeKind::Call`/dispatch site this
    /// method's own task loop reaches still charges `function.call` against
    /// `self.meter` (`charge_call`, called directly at those sites) --
    /// those are genuinely separate calls, not a restatement of this one.
    ///
    /// **`Err(InternalFault)` (FR-090-AC-10).** [`Self::resolve_population`]
    /// returns `Err(Stop::Fault(_))` rather than a `Refusal` when a
    /// `Value::Population` argument reaches it unresolved or with a
    /// mismatched declared maximum -- a condition admission already rules
    /// out for any checked program reached through `CheckedPackage::call` or
    /// `CheckedPackage::evaluate`. This loop matches `Stop::Fault` out the
    /// moment a task fails, before `Self::stopped` ever converts the
    /// remaining `Stop` shapes into an `Outcome`, so that broken invariant
    /// surfaces as `Err`, never as `Ok(Evaluation { outcome:
    /// Outcome::Refused(_), .. })`.
    pub(crate) fn run(
        mut self,
        root: &'a Node,
        slots: usize,
        arguments: Vec<Value>,
    ) -> Result<Evaluation, InternalFault> {
        let mut frame: Vec<Option<Value>> = arguments.into_iter().map(Some).collect();
        frame.resize(slots.max(frame.len()), None);
        self.frames.push(frame);
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
                Task::Bind(_) | Task::Return | Task::DispatchGuard(_) | Task::RestoreAnchor(_) => {
                    None
                }
            };
            let location = location.cloned().unwrap_or_else(|| root.location.clone());
            if let Err(halt) = self.step(task) {
                return match halt {
                    Halt::Fault(fault) => Err(fault),
                    Halt::Stop(stop) => Ok(Self::stopped(stop, &location)),
                };
            }
        }
        match (self.values.pop(), self.values.is_empty()) {
            (Some(value), true) => Ok(Evaluation {
                outcome: Outcome::Completed(value),
                location: None,
                losses: self.losses,
            }),
            _ => Ok(Self::stopped(checked_invariant(), &root.location)),
        }
    }

    fn stopped(stop: Stop, location: &Location) -> Evaluation {
        Evaluation {
            outcome: Outcome::from_stop(Err(stop)),
            location: Some(location.clone()),
            losses: Vec::new(),
        }
    }

    fn pop(&mut self) -> Result<Value, Halt> {
        self.values.pop().ok_or_else(invariant)
    }

    fn pop_many(&mut self, count: usize) -> Result<Vec<Value>, Halt> {
        let start = self.values.len().checked_sub(count).ok_or_else(invariant)?;
        Ok(self.values.split_off(start))
    }

    fn pop_integer(&mut self) -> Result<Integer, Halt> {
        match self.pop()? {
            Value::Integer(value) => Ok(value),
            _ => Err(invariant()),
        }
    }

    fn pop_boolean(&mut self) -> Result<bool, Halt> {
        match self.pop()? {
            Value::Boolean(value) => Ok(value),
            _ => Err(invariant()),
        }
    }

    fn pop_collection(&mut self) -> Result<Arc<CollectionValue>, Halt> {
        match self.pop()? {
            Value::Collection(collection) => Ok(collection),
            _ => Err(invariant()),
        }
    }

    fn pop_rational(&mut self) -> Result<Rational, Halt> {
        match self.pop()? {
            Value::Rational(value) => Ok(value),
            _ => Err(invariant()),
        }
    }

    fn pop_decimal(&mut self) -> Result<Decimal, Halt> {
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

    fn slot(&mut self, slot: Slot) -> Result<&mut Option<Value>, Halt> {
        self.frames
            .last_mut()
            .and_then(|frame| frame.get_mut(slot))
            .ok_or_else(invariant)
    }

    /// FR-153: the population `allInstances`/`lookup` reads for `binding` at
    /// the current anchor -- `binding` itself when reading the ambient post
    /// population, or `binding`'s attached pre population underneath
    /// `pre(..)`. A `Pre` anchor with no attached pre population is a real,
    /// caller-input-reachable refusal, not a checked invariant: the
    /// *checker* only admits `pre(..)` in a postcondition, over an operand
    /// with an eligible read underneath it that is not itself a captured
    /// `let` alias (`check.rs`'s `contains_pre_eligible_read`/
    /// `contains_captured_pre_alias`), but it has no way to see whether the
    /// `Value::Population` a caller supplies at evaluation time was actually
    /// admitted through [`admit_invocation`] (the only constructor that
    /// attaches a `pre_anchor`) rather than [`admit_binding`] directly. A
    /// binding with no attached pre population reaching here is exactly that
    /// caller-input defect, refused `wrong_snapshot`/`wrong-anchor` --
    /// `CheckedInvariant` is reserved for a broken evaluator invariant, never
    /// for input a caller controls.
    ///
    /// [`admit_invocation`]: crate::model::population::admit_invocation
    /// [`admit_binding`]: crate::model::population::admit_binding
    fn select_anchor<'x>(
        &self,
        binding: &'x PopulationBinding,
    ) -> Result<&'x PopulationBinding, Halt> {
        match self.anchor {
            Anchor::Post => Ok(binding),
            Anchor::Pre => binding.pre_anchor().ok_or_else(|| {
                Stop::Refused(Refusal::WrongSnapshot(WrongSnapshotCause::WrongAnchor)).into()
            }),
        }
    }

    /// FR-089-AC-3/AC-4/AC-5, FR-090-AC-10: resolves `population_id` (an
    /// `allInstances`/`lookup` population operand's own identity) to the
    /// admitted `PopulationBinding` this evaluation's own recorded
    /// correspondence (`self.objects`, `model`'s
    /// `admit_binding`/`admit_invocation` mint into) recorded it against, by
    /// lookup alone -- never by decoding `population_id`'s own bytes. The
    /// pairing this checks is the one `ValueType::admits` performed directly
    /// when `Value::Population` still carried the binding itself
    /// (`value::composite`'s own doc, before this identity replaced it).
    ///
    /// **FR-090-AC-10: an invariant break, not a `Refusal` (ADR-013 T-4).**
    /// `CheckedPackage::call`'s own `validate` (`expression/mod.rs`) admits
    /// this identical pairing at argument-admission time, over every
    /// top-level `Population<T>[N]` parameter -- the only context FR-153
    /// lets one appear in (`check::check::bind_parameters`'s own doc), and
    /// the same recorded correspondence this method reads. Nested-call
    /// argument types are structurally checked equal to their callee's own
    /// declared parameter types (never bypassed the way
    /// `Typer::check_declared_type` is for a *declaration*), so a
    /// `Value::Population` that admitted at the outer boundary keeps
    /// resolving, with the same declared maximum, at every nested
    /// consumption site -- meeting an unresolved identity or a mismatched
    /// maximum here means this checked program reached S6a without going
    /// through admission at all, an internal fault rather than caller
    /// input. Returns `Err(Halt::Fault(_))`, read back by [`Self::run`]
    /// before anything converts a `Stop` into an `Outcome` -- this was the
    /// one production call site of the kernel-shaped `Refusal::
    /// UnresolvedPopulation`/`Refusal::PopulationMaximumMismatch` variants
    /// this method used to construct; both are deleted along with it.
    fn resolve_population(
        &self,
        population_id: PopulationId,
        maximum: u64,
    ) -> Result<&'a PopulationBinding, Halt> {
        match self.objects.resolve_population(population_id) {
            Some(binding) if binding.declared_maximum() == Some(maximum) => Ok(binding),
            Some(_) => Err(Halt::Fault(InternalFault::new(
                "S6a",
                "population-argument-maximum-mismatch-past-admission",
            ))),
            None => Err(Halt::Fault(InternalFault::new(
                "S6a",
                "population-argument-unresolved-past-admission",
            ))),
        }
    }

    fn step(&mut self, task: Task<'a>) -> Result<(), Halt> {
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
            Task::ChargeElement(_) => charge_element(self.meter).map_err(Into::into),
            Task::Return => {
                self.frames.pop().ok_or_else(invariant)?;
                Ok(())
            }
            Task::DispatchGuard(guard) => {
                let DispatchGuard {
                    body_function,
                    arguments,
                    failure,
                } = *guard;
                let holds = self.pop_boolean()?;
                self.frames.pop().ok_or_else(invariant)?;
                if !holds {
                    return Err(
                        Stop::Undefined(Undefined::PreconditionFalse(Box::new(failure))).into(),
                    );
                }
                let callable = self.functions.get(body_function).ok_or_else(invariant)?;
                charge_call(self.meter)?;
                let mut frame: Vec<Option<Value>> = arguments.into_iter().map(Some).collect();
                frame.resize(callable.slots.max(frame.len()), None);
                self.frames.push(frame);
                self.tasks.push(Task::Return);
                self.tasks.push(Task::Eval(callable.body));
                Ok(())
            }
            Task::Iterate(iteration) => self.iterate(iteration),
            Task::RestoreAnchor(previous) => {
                self.anchor = previous;
                Ok(())
            }
        }
    }

    fn eval(&mut self, node: &'a Node) -> Result<(), Halt> {
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

    fn apply(&mut self, node: &'a Node) -> Result<(), Halt> {
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
                    return Err(Stop::Refused(Refusal::IntegerOutOfDomain).into());
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
                identity: _,
                function,
                arguments,
            } => {
                let callable = self.functions.get(*function).ok_or_else(invariant)?;
                let arguments = self.pop_many(arguments.len())?;
                charge_call(self.meter)?;
                let mut frame: Vec<Option<Value>> = arguments.into_iter().map(Some).collect();
                frame.resize(callable.slots.max(frame.len()), None);
                self.frames.push(frame);
                // shared-grammar.md: "self, result and pre(...) are ...
                // unavailable ... inside a reusable predicate" -- a callee's
                // own body is never anchored by its caller's `pre(..)`, so
                // `pre(F(p))` never lets the anchor leak into `F`'s body.
                // Reset to `Post` for the callee, then restore the caller's
                // anchor once its own `Task::Eval`/`Task::Return` complete
                // (mirrors `NodeKind::Pre`'s own save/restore pair).
                self.tasks.push(Task::RestoreAnchor(self.anchor));
                self.anchor = Anchor::Post;
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
            NodeKind::AllInstances { population } => {
                let Value::Population(population_id) = self.pop()? else {
                    return Err(invariant());
                };
                let ValueType::Population(maximum) = &population.value_type else {
                    return Err(invariant());
                };
                let ValueType::Collection(collection_type) = &node.value_type else {
                    return Err(invariant());
                };
                let binding = self.resolve_population(population_id, *maximum)?;
                let binding = self.select_anchor(binding)?;
                evaluate_all_instances(binding, collection_type, self.meter)?
            }
            NodeKind::Lookup {
                reference,
                absence,
                population,
            } => {
                let reference_value = self.pop()?;
                let Value::Population(population_id) = self.pop()? else {
                    return Err(invariant());
                };
                let ValueType::Population(maximum) = &population.value_type else {
                    return Err(invariant());
                };
                let binding = self.resolve_population(population_id, *maximum)?;
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
                let binding = self.select_anchor(binding)?;
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
            NodeKind::Dispatch {
                table,
                operation,
                arguments,
                ..
            } => {
                let arguments = self.pop_many(arguments.len())?;
                let Value::Reference(reference) = self.pop()? else {
                    return Err(invariant());
                };
                let subtype = reference.object_type();
                // The exact call site that produced this node (`check.rs`'s
                // `dispatch_call`), never re-derived by searching
                // `dispatch_operations` for a `table` match: two call sites
                // can share a table, and a `table`-only search would report
                // whichever operation happens to come first, not the one
                // this node's own `receiver.member(args)` actually named.
                let operation = self
                    .scope
                    .dispatch_operations
                    .get(*operation)
                    .ok_or_else(invariant)?;
                let table_ref = self.dispatch_tables.get(*table).ok_or_else(invariant)?;
                charge_dispatch_select(self.meter, table_ref.candidate_count())?;
                let candidate = table_ref
                    .linked_for(&subtype)
                    .ok_or_else(invariant)?
                    .clone();
                let mut full_arguments = Vec::with_capacity(arguments.len() + 1);
                full_arguments.push(Value::Reference(reference.clone()));
                full_arguments.extend(arguments);
                match candidate.precondition {
                    Some(precondition_function) => {
                        let callable = self
                            .functions
                            .get(precondition_function)
                            .ok_or_else(invariant)?;
                        let selected = self
                            .functions
                            .get(candidate.body)
                            .ok_or_else(invariant)?
                            .name
                            .to_owned();
                        let mut frame: Vec<Option<Value>> =
                            full_arguments.clone().into_iter().map(Some).collect();
                        frame.resize(callable.slots.max(frame.len()), None);
                        self.frames.push(frame);
                        self.tasks.push(Task::DispatchGuard(Box::new(DispatchGuard {
                            body_function: candidate.body,
                            arguments: full_arguments,
                            failure: PreconditionFailure {
                                operation: operation.member.clone(),
                                selected,
                                receiver: reference,
                            },
                        })));
                        self.tasks.push(Task::Eval(callable.body));
                        return Ok(());
                    }
                    None => {
                        let callable = self.functions.get(candidate.body).ok_or_else(invariant)?;
                        charge_call(self.meter)?;
                        let mut frame: Vec<Option<Value>> =
                            full_arguments.into_iter().map(Some).collect();
                        frame.resize(callable.slots.max(frame.len()), None);
                        self.frames.push(frame);
                        self.tasks.push(Task::Return);
                        self.tasks.push(Task::Eval(callable.body));
                        return Ok(());
                    }
                }
            }
        };
        self.values.push(value);
        Ok(())
    }

    fn project(slot: &FieldValue, optional: bool, value_type: &ValueType) -> Result<Value, Halt> {
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
    ) -> Result<bool, Halt> {
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
                    .into_stop()
                    .map_err(Into::into);
            }
            (OrderedKind::Texts, Value::Text(l), Value::Text(r)) => {
                return compare_text(comparison(operator), l, r, self.meter)
                    .map_err(|_| invariant())?
                    .into_stop()
                    .map_err(Into::into);
            }
            (OrderedKind::Quantities, Value::Quantity(l), Value::Quantity(r)) => {
                return compare_quantity(comparison(operator), l, r, self.meter)
                    .map_err(|_| invariant())?
                    .into_stop()
                    .map_err(Into::into);
            }
            _ => return Err(invariant()),
        };
        order_numbers(operator, operands, self.meter)
            .into_stop()
            .map_err(Into::into)
    }

    fn start_iteration(&mut self, node: &'a Node) -> Result<(), Halt> {
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

    fn iterate(&mut self, mut iteration: Box<Iteration<'a>>) -> Result<(), Halt> {
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

    fn finish(&mut self, iteration: Iteration<'a>) -> Result<(), Halt> {
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
                        return Err(Stop::Refused(Refusal::IntegerOutOfDomain).into());
                    }
                    retain_scalar(Value::Integer(total), self.meter)?
                }
                Visit::Count => {
                    if let ValueType::Int(domain) = &node.value_type {
                        if !domain.contains(&iteration.count) {
                            return Err(Stop::Refused(Refusal::IntegerOutOfDomain).into());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{CheckingLimits, PackageDeclarations};
    use crate::family::ReferenceEvaluation;
    use crate::forms::{Expression, FunctionDeclaration, TypeForm};
    use crate::model::accounting::ModelNormalizationLimits;
    // `TypeForm`'s span carries no identity (ADR-011 §2.2 row E2).
    const SPAN: qsl_foundation::Span = qsl_foundation::Span { start: 0, end: 0 };
    use crate::model::dispatch::GeneralizationClosure;
    use crate::model::domain_package::{
        DomainPackage, DomainPackageRecord, DomainPackageRef, Extent, ObjectTypeRecord,
        PopulationRecord,
    };
    use crate::model::key::DeclarationKey;
    use crate::model::normalize::{normalize, NormalizeOutcome};
    use crate::model::population::{
        admit_binding, AdmissionMeter, AdmissionOutcome, PopulationAdmissionLimits,
        PopulationDocument, PopulationMember,
    };
    use ix_trace_rs::trace;
    use qsl_foundation::diagnostic::Category;

    /// The shared minimal one-type (`model.A`), one-population
    /// (`model.pop.p1`) domain package [`population_binding`] and
    /// [`population_function_package`] both normalize below (PR #334 review
    /// round 2, finding N5) -- deliberately not `tests/it/
    /// model_reference_queries.rs`'s richer `fixture_f1` (generalization,
    /// two object types), which that file's own `allInstances`/`lookup`
    /// source-form coverage needs and these evaluator-internal tests do
    /// not. `reference` distinguishes the two callers' own
    /// `DomainPackageRef::fixture` identities.
    fn domain_package(reference: &str) -> DomainPackage {
        DomainPackage::new(
            DomainPackageRef::fixture(reference),
            vec![
                DomainPackageRecord::ObjectType(ObjectTypeRecord {
                    key: DeclarationKey::fixture("model.A"),
                    interface_features: None,
                    abstract_type: false,
                    supertypes: Vec::new(),
                }),
                DomainPackageRecord::Population(PopulationRecord {
                    key: DeclarationKey::fixture("model.pop.p1"),
                    member_types: vec![DeclarationKey::fixture("model.A")],
                    extent: Extent::Closed,
                }),
            ],
        )
    }

    /// One admitted binding over [`domain_package`], for
    /// [`Machine::resolve_population`]'s own fault tests below.
    fn population_binding(declared_maximum: Option<u64>) -> PopulationBinding {
        let domain_package = domain_package("bundle.qsl174-ac10");
        let view = match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
            NormalizeOutcome::Completed(view) => view,
            other => panic!("expected a completed effective view, got {other:?}"),
        };
        let document = PopulationDocument {
            model_identity: "test/orders".to_owned(),
            members: vec![PopulationMember {
                object: "a1".to_owned(),
                type_identity: DeclarationKey::fixture("model.A"),
                field_values: Vec::new(),
            }],
        };
        let mut meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
        match admit_binding(
            &domain_package,
            &view,
            &document,
            &DeclarationKey::fixture("model.pop.p1"),
            GeneralizationClosure::Closed,
            declared_maximum,
            &mut meter,
        ) {
            AdmissionOutcome::Admitted(binding) => binding,
            other => panic!("expected an admitted binding, got {other:?}"),
        }
    }

    /// A checked package declaring one `Value` function,
    /// `F(p: Population<M::A>[3]): Integer = size(allInstances<M::A>(p))`,
    /// over [`domain_package`] -- TC-391's own fixture. Returns the linked
    /// package and `F`'s checked identity, so a test can call
    /// [`crate::check::ValueFunctionFamily::evaluate`] directly -- the S6a
    /// seam itself, bypassing `CheckedPackage::call`'s admission.
    fn population_function_package(
    ) -> (crate::checked_package::CheckedPackage, quire_exact::NodeKey) {
        let domain_package = domain_package("bundle.qsl174-ac10-seam");
        let view = match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
            NormalizeOutcome::Completed(view) => view,
            other => panic!("expected a completed effective view, got {other:?}"),
        };
        let a = view
            .declarations()
            .iter()
            .find(|entry| {
                entry.preimage.owner_effective_type.is_none()
                    && entry.preimage.original == DeclarationKey::fixture("model.A")
            })
            .map(|entry| entry.effective_id)
            .expect("model.A has a type-level effective declaration");
        let node_a = crate::value::node::NodeKey::from_digest(*a.as_bytes());
        let types = crate::value::composite::TypeEnvironment::new(
            [],
            [crate::value::composite::ObjectTypeDeclaration::new(
                node_a,
                "M::A",
                Vec::new(),
            )],
        )
        .expect("one object type admits cleanly");
        let graph = PackageDeclarations {
            types,
            functions: vec![FunctionDeclaration::new(
                "F",
                vec![(
                    "p".to_owned(),
                    TypeForm::name("Population", SPAN)
                        .with_arguments(vec![TypeForm::name("M::A", SPAN)])
                        .with_bounds(vec!["3".to_owned()]),
                )],
                TypeForm::keyword(qsl_cst::token::Kind::IntegerType, SPAN),
                None,
                Expression::Size(Box::new(Expression::AllInstances {
                    target: TypeForm::name("M::A", SPAN),
                    population: Box::new(Expression::Name("p".to_owned())),
                })),
            )],
            ..PackageDeclarations::default()
        }
        .check(CheckingLimits::default())
        .expect("F(p: Population<M::A>[3]): Integer = size(allInstances<M::A>(p)) checks cleanly");
        let identity = graph
            .function_identity("F")
            .expect("F is declared in this package");
        (
            crate::checked_package::CheckedPackage::link(graph),
            identity,
        )
    }

    /// TC-391 (FR-090-AC-10): a `Value::Population` argument whose id names
    /// no recorded binding, passed directly to the S6a seam
    /// (`ValueFunctionFamily::evaluate`) rather than through
    /// `CheckedPackage::call` -- the condition `call`'s own `validate`
    /// already refuses at admission (`InputRefusal::WrongValueKind`,
    /// TC-294/FR-089-AC-4) -- raises `Err(EvaluateFailure::Fault(_))`
    /// naming stage `"S6a"` and this fault's own literal invariant
    /// identifier, never a panic and never `Ok(Evaluation { outcome:
    /// Outcome::Refused(_), .. })`.
    #[trace("FR-090-AC-10", "TC-391")]
    #[test]
    fn evaluate_bypassing_admission_with_an_unresolved_population_id_is_an_internal_fault() {
        let (package, identity) = population_function_package();
        let objects = ObjectEnvironment::default();
        let unresolved_id = PopulationId::from_digest([7; 32]);
        let mut local_meter = Meter::new(crate::check::SCALAR_LIMITS_UNLIMITED);
        let mut env = crate::value::expression::family::EvaluationEnv {
            package: &package,
            objects: &objects,
            arguments: Some(vec![Value::Population(unresolved_id)]),
            local_meter: &mut local_meter,
        };
        let mut contract_meter = Meter::new(crate::check::SCALAR_LIMITS_UNLIMITED);
        let failure =
            crate::check::ValueFunctionFamily::evaluate(&identity, &mut env, &mut contract_meter)
                .expect_err("an unresolved population id must fault, never evaluate");
        match failure {
            crate::family::EvaluateFailure::Fault(fault) => {
                assert_eq!(fault.stage(), "S6a");
                assert_eq!(fault.category(), Category::InternalFailure);
                assert_eq!(
                    fault.invariant(),
                    "population-argument-unresolved-past-admission"
                );
            }
            other => panic!("expected EvaluateFailure::Fault(_), got {other:?}"),
        }
    }

    /// TC-391 (FR-090-AC-10): the companion case, a `Value::Population`
    /// argument that resolves but whose binding's declared maximum (2)
    /// differs from `F`'s declared `Population<3>` -- also
    /// `Err(EvaluateFailure::Fault(_))` from the S6a seam directly, with
    /// its own distinct literal invariant identifier.
    #[trace("FR-090-AC-10", "TC-391")]
    #[test]
    fn evaluate_bypassing_admission_with_a_population_maximum_mismatch_is_an_internal_fault() {
        let (package, identity) = population_function_package();
        let binding = population_binding(Some(2));
        let id = binding.population_id();
        let objects = ObjectEnvironment::default()
            .with_population(binding)
            .unwrap();
        let mut local_meter = Meter::new(crate::check::SCALAR_LIMITS_UNLIMITED);
        let mut env = crate::value::expression::family::EvaluationEnv {
            package: &package,
            objects: &objects,
            arguments: Some(vec![Value::Population(id)]),
            local_meter: &mut local_meter,
        };
        let mut contract_meter = Meter::new(crate::check::SCALAR_LIMITS_UNLIMITED);
        let failure =
            crate::check::ValueFunctionFamily::evaluate(&identity, &mut env, &mut contract_meter)
                .expect_err("a mismatched declared maximum must fault, never evaluate");
        match failure {
            crate::family::EvaluateFailure::Fault(fault) => {
                assert_eq!(fault.stage(), "S6a");
                assert_eq!(fault.category(), Category::InternalFailure);
                assert_eq!(
                    fault.invariant(),
                    "population-argument-maximum-mismatch-past-admission"
                );
            }
            other => panic!("expected EvaluateFailure::Fault(_), got {other:?}"),
        }
    }
}
