// SPDX-License-Identifier: AGPL-3.0-or-later
//! The metered evaluator of checked expressions and function calls.
//!
//! Evaluation runs on an explicit task stack, so neither expression nesting
//! nor recursive call depth consumes host stack; `function.call` and every
//! per-element charge bound the work an evaluation can do. Literals,
//! parameters, `let` names, field projections, `present`, `value` and `if`
//! make no charge.

use filament_checked::node::{
    Arithmetic, AuthoredBound, Connective, ConnectiveOrigin, EndSide, IndexedKind,
    IntegerDivisionOperator, NodeKind, OrderedKind, Visit,
};
use std::cmp::Ordering;
use std::collections::{BTreeSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::Arc;

use super::causes::{
    HistoryUnresolved, ModelQueryRefusal, NamedPopulationIncomplete, NavigationRefusal,
    ObjectFieldIllTyped, ProtocolClauseSnapshot, UnsupportedConstruct,
};
use super::protocol_slot::charges::{BinderCharges, ChargeRouting, Owner, Owners};
use super::s6a::separation::{
    locatable, occurrence_at, ComparisonReport, InvocationBinding, Provenance, RuntimeValuePath,
    Separation, SeparationStep, StopReport, Trail, ValuePathStep, ValuePathSubject, WitnessClaim,
};
use super::slot_admission;
use crate::simulation::model::{ReferenceBinding, ReferenceBindings};
use filament_checked::key::DeclarationKey;
use filament_eval::modulo as integer_modulo;
use filament_eval::Evaluation;
use filament_eval::FamilyOutcome;
use filament_eval::FamilyResult;
use filament_eval::{
    LiveContext, PreconditionFailure, ProtocolReferenceUnsupported, ReferenceUnresolved,
    SessionLimits, StateModelUndefined,
};
use filament_exact::{
    admit_quantity_result, compare_text, BoundedQuantityType, ComparisonOperator,
    QuantityResultInterval,
};
use filament_exact::{
    compare_ieee, convert_ieee_width, evaluate_ieee, exact_to_ieee, ieee_to_exact, IeeeComparison,
    IeeeFlags, IeeeOperation, IeeeValue, IeeeWidth,
};
use filament_exact::{
    compare_keys, form, form_grouped, retain_composite, CollectionValue, CompositeValue,
    FieldValue, ObjectReference, OptionValue, Quantity, UnionValue, Value, ValueType,
};
use filament_exact::{
    evaluate_boolean, evaluate_integer_arithmetic, evaluate_rational_arithmetic, order_numbers,
    retain_boolean, ArithmeticOperator, BooleanConnective, IntegerArithmetic, OrderedOperands,
    OrderingOperator, RationalArithmetic,
};
use filament_exact::{evaluate_decimal, Decimal, DecimalOperation, DecimalType};
use filament_exact::{
    BoundViolation, CardinalityBound, Charge, ChargePoint, CollectionKind, Incomplete, Integer,
    LimitKind, Meter, PopulationId,
};
use filament_exact::{DivisionMember, IntegerDomain};
use filament_exact::{EffectiveId, NodeKey};
use filament_exact::{FloatType, Rational};
use filament_exact::{Outcome, Refusal, Undefined};
use filament_semantic_value::catalog::{render_identity, InternalFault};
use filament_semantic_value::declaration::{
    operand_value, CompositeShape, EqualityOperator, FieldDeclaration, FieldRef, TypeEnvironment,
};
use filament_semantic_value::enumeration::{compare_enum, EnumMemberIndex};
use filament_semantic_value::location::ExpressionLocation as Location;
use filament_semantic_value::loss::{LocatedLoss, ValueLoss};
use filament_semantic_value::quantity::{
    compare_quantity, convert_quantity, evaluate_quantity_unit, ConvertedValue, QuantityOperation,
    QuantityTarget, UnitScope,
};
use filament_semantic_value::relationship_extent::Resolution;
use filament_semantic_value::stop::{outcome_from_stop, outcome_into_stop, Stop};
use fsl_foundation::diagnostic::{kernel_refusal_record, Locus, RefusalRecord};
use fsl_foundation::source::provenance::SourceRegion;
use fsl_semantics::check::{CheckedGraph, DispatchTable, FormedUnits, Operator, Scope, Slot};
#[cfg(test)]
use fsl_semantics::check::{CheckedNode, NodeId};
use fsl_semantics::model::object_environment::{ObjectEnvironment, ObservedRead};
use fsl_semantics::model::observation::OutOfRange;
use fsl_semantics::model::population::PopulationBinding;
use fsl_semantics::value::divide as integer_divide;
use fsl_semantics::value::model_query::{evaluate_all_instances, evaluate_lookup, ModelQueryHalt};
use node_source::NodeSource;
pub(crate) use world::World;

/// FR-096, ADR-013 O-17: `evaluation`'s refusal as an O-17 [`RefusalRecord`]:
/// the family cause's code and catalog fields, and, as its locus, the
/// evaluation's location resolved by `resolve`. The locus is absent when the
/// location is `None` or `resolve` names no region for it (a tree not read
/// from a source unit).
///
/// `resolve` must be the region resolver of the checked package the
/// evaluation ran (`CheckedGraph::region`): a location is a position in that
/// package's declarations, and another package resolves it to another
/// region.
///
/// `None` when no record exists: the outcome is no refusal, a family cause
/// has no FR-096 key-table row, or a kernel refusal has no catalog code
/// (FR-096-AC-8, [`kernel_refusal_record`]).
pub fn refusal_record(
    evaluation: &Evaluation,
    resolve: impl Fn(&Location) -> Option<SourceRegion>,
) -> Option<RefusalRecord> {
    let locus = evaluation
        .location
        .as_ref()
        .and_then(resolve)
        .map(Locus::Region);
    match &evaluation.outcome {
        FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause)) => {
            fsl_foundation::diagnostic::refusal_record(cause.as_ref(), locus)
        }
        FamilyOutcome::Evaluated(Outcome::Refused(refusal)) => {
            kernel_refusal_record(refusal, locus)
        }
        _ => None,
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

/// `Machine`'s own early-exit carrier: an evaluator [`Stop`], a
/// family-owned evaluation-time result (FR-090-AC-7, AC-8, AC-11, AC-12), or
/// an S6a invariant break neither can represent. Crate-private to this
/// module: nothing outside `Machine` builds, matches or forwards one. Only
/// [`Machine::run`] unwraps a `Halt`: `Fault` returns `Err(InternalFault)`,
/// `Family` becomes `Ok(Evaluation { outcome: FamilyOutcome::
/// FamilyEvaluated(_), .. })` located at the current task like a `Stop`
/// (FR-090-OQ-3), and `Stop` goes through [`Self::stopped`]/
/// [`outcome_from_stop`]. No `Stop` variant carries a fault, so no
/// `Stop`-returning helper can pass one to [`outcome_from_stop`].
enum Halt {
    /// An ordinary evaluator stop, to be converted to an `Outcome` as usual.
    Stop(Stop),
    /// An evaluator stop located at a node other than the current task's
    /// (a `sum` seed is located at its summand, not at the `sum` node).
    Located(Box<LocatedStop>),
    /// FR-390: the selected caller supplied a known source as unavailable,
    /// and evaluation reached that local read.
    Unavailable {
        binder: filament_exact::NodeKey,
        cause: filament_semantic_value::catalog::CatalogCode,
    },
    /// A family-owned evaluation-time refusal or undefined result.
    Family(FamilyResult),
    /// An S6a invariant break: never converted to an `Outcome`.
    Fault(InternalFault),
}

/// A [`Stop`] and the location it reports, boxed in [`Halt::Located`] to keep
/// `Halt` small.
struct LocatedStop {
    stop: Stop,
    location: Option<Location>,
}

use crate::value::collections::{
    self, charge_element, charge_visit, retain_scalar, Absorbed, Flow, Iteration as IterationState,
    Sequence, Shape, Spec,
};
use crate::value::composite::{self, construct_record, construct_tuple, present_slots, read_field};

/// A shared collection operation's halt as the evaluator's. The cases the
/// evaluator handles where they arise (a `sum` seed, an `only` population)
/// are invariant breaks if they reach here.
fn lift(halt: collections::Halt) -> Halt {
    match halt {
        collections::Halt::Stop(stop) => Halt::from(stop),
        collections::Halt::Fault(identifier) => invariant(identifier),
        collections::Halt::SumSeedOutOfDomain | collections::Halt::OnlyNotOne { .. } => {
            invariant("collection-halt-unhandled")
        }
    }
}

impl From<collections::Halt> for Halt {
    fn from(halt: collections::Halt) -> Self {
        lift(halt)
    }
}

/// A quantity whose `UnitId` neither the package's unit table nor this
/// evaluation's formed units resolve. Checking resolves every quantity type
/// and admission matches every argument's unit id to its parameter's, so the
/// checked-expression route no longer reaches this S6a invariant break
/// (foreign expressions are refused, #2098), and an object environment typed
/// under a different unit refuses at entry or at the attribute read
/// (QSL-1470, QSL-1565, FR-049-AC-2). Reaching this is a broken invariant
/// (FR-090-AC-15).
fn unresolved_unit() -> Halt {
    Halt::Fault(InternalFault::new(
        "S6a",
        "quantity-unit-unresolved-past-admission",
    ))
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

/// FR-090-AC-8, AC-12: a population query's halt as the `StateModel`
/// family's own evaluation-time result.
impl From<ModelQueryHalt> for Halt {
    fn from(halt: ModelQueryHalt) -> Self {
        match halt {
            ModelQueryHalt::Fault(fault) => Self::Fault(fault),
            ModelQueryHalt::Stop(stop) => Self::Stop(stop),
            ModelQueryHalt::Refused(refusal) => Self::Family(FamilyResult::Refused(Box::new(
                ModelQueryRefusal::from(*refusal),
            ))),
            ModelQueryHalt::AbsentKey { population, key } => Self::Family(FamilyResult::Undefined(
                Box::new(StateModelUndefined::AbsentKey {
                    binding: population.node().to_string(),
                    key: render_identity(&key),
                }),
            )),
            ModelQueryHalt::IncompleteClosure {
                queried_type,
                uncovered_subtypes,
            } => Self::Family(FamilyResult::Undefined(Box::new(
                StateModelUndefined::IncompleteClosure {
                    queried_type,
                    uncovered_subtypes,
                },
            ))),
        }
    }
}

/// FSpec FR-207 "Computed collections": the source position of each
/// element of `formed`, the collection a `map` or `filter` formed (with
/// `kind`) from `results`, where `results[i]` came from source position
/// `sources[i]`. A sequence or ordered set keeps the results' order (an
/// ordered set keeps each first occurrence); a set or bag is in canonical
/// key order, so the results are matched in that order. `None` when an
/// element matches no result, which forming never produces.
fn formed_sources(
    kind: CollectionKind,
    sources: &[usize],
    results: &[Value],
    formed: &[Value],
) -> Option<Vec<usize>> {
    let mut order: Vec<usize> = (0..results.len()).collect();
    if matches!(kind, CollectionKind::Set | CollectionKind::Bag) {
        order.sort_by(|left, right| {
            compare_keys(&results[*left], &results[*right]).unwrap_or(Ordering::Equal)
        });
    }
    let mut candidates = order.into_iter();
    formed
        .iter()
        .map(|element| {
            candidates
                .by_ref()
                .find(|i| compare_keys(&results[*i], element) == Some(Ordering::Equal))
                .and_then(|i| sources.get(i).copied())
        })
        .collect()
}

/// A completed evaluation's value, or the evaluation that completed none.
fn completed(evaluation: Evaluation) -> Result<Value, Box<Evaluation>> {
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(value)) => Ok(value),
        outcome => Err(Box::new(Evaluation {
            outcome,
            ..evaluation
        })),
    }
}

/// The inclusive exact result interval of a bounded quantity type; a type
/// with an open endpoint never reaches evaluation, as `check` refuses it.
fn bounded_result_interval(bounded: &BoundedQuantityType) -> Result<QuantityResultInterval, Halt> {
    let (Some(min), Some(max)) = (bounded.min(), bounded.max()) else {
        return Err(invariant("bounded-quantity-result-endpoints-expected"));
    };
    Ok(QuantityResultInterval::new(
        bounded.unit().clone(),
        min.magnitude().clone(),
        max.magnitude().clone(),
    ))
}

/// The `size` of a text (its profile length); a collection is sized by
/// [`collections::size`] and any other operand is an invariant fault.
fn size_of(operand: Value) -> Result<Integer, Halt> {
    match operand {
        Value::Text(text) => Ok(Integer::from(text.length())),
        Value::Map(map) => Ok(Integer::from(map.entries().len())),
        Value::Bytes(_) => Err(unsupported_construct("bytes")),
        Value::JsonObject(_) => Err(unsupported_construct("json_object")),
        Value::Boolean(_)
        | Value::Integer(_)
        | Value::Rational(_)
        | Value::Decimal(_)
        | Value::Float(_)
        | Value::Quantity(_)
        | Value::String(_)
        | Value::Uuid(_)
        | Value::Timestamp(_)
        | Value::Enum(_)
        | Value::Population(_)
        | Value::Option(_)
        | Value::Composite(_)
        | Value::Union(_)
        | Value::Collection(_)
        | Value::Reference(_) => Err(invariant("collection-value-expected")),
    }
}

/// A checked construct whose evaluation is not implemented in this evaluator
/// returns its typed unsupported outcome, naming the construct (FR-091).
fn unsupported_construct(what: &'static str) -> Halt {
    Halt::Family(FamilyResult::Refused(Box::new(UnsupportedConstruct {
        what,
    })))
}

fn quantity_unit(value_type: &ValueType) -> Option<&filament_exact::UnitId> {
    match value_type {
        ValueType::Quantity(unit) => Some(unit),
        ValueType::BoundedQuantity(quantity) => Some(quantity.unit()),
        ValueType::CarriedQuantity(quantity) => Some(quantity.unit()),
        _ => None,
    }
}

/// An FSL-owned invariant at its detecting producer (FR-090).
fn invariant(identifier: &'static str) -> Halt {
    Halt::Fault(InternalFault::new("S6a", identifier))
}

/// Build the FR-230 identity for a clock place selected from an owner.
fn clock_place_reference(
    owner: ObjectReference,
    clock_type: EffectiveId,
    place: &NodeKey,
    selectors: Vec<Value>,
) -> Result<ObjectReference, Halt> {
    if !fsl_semantics::model::clock_reference::is_clock_type(&clock_type) {
        return Err(invariant("place-reference-result-type-expected"));
    }
    fsl_semantics::model::clock_reference::encode(&owner, place, &selectors)
        .map_err(|_| invariant("clock-place-reference-identity-invalid"))
}

/// The one target of a single-valued navigation end `[lower, upper]` (FR-364
/// step 4): the count is decided with no charge, and a count outside the
/// bound is the kernel's `cardinality_out_of_bound` refusal, whose
/// collection kind is `Set` for a `unique` end and `Bag` otherwise, with a
/// repeated instance of a non-unique end counted once per instance.
fn single_target(
    mut elements: Vec<Value>,
    lower: u64,
    upper: u64,
    unique: bool,
) -> Result<Option<Value>, Halt> {
    let count = u64::try_from(elements.len()).unwrap_or(u64::MAX);
    let violation = if count < lower {
        Some(BoundViolation::BelowMinimum)
    } else if count > upper {
        Some(BoundViolation::AboveMaximum)
    } else {
        None
    };
    if let Some(violation) = violation {
        let bound = CardinalityBound::new(lower, upper)
            .map_err(|_| invariant("navigate-single-valued-bound-invalid"))?;
        return Err(Halt::Stop(Stop::Refused(Refusal::CardinalityOutOfBound {
            violation,
            kind: if unique {
                CollectionKind::Set
            } else {
                CollectionKind::Bag
            },
            bound,
            count,
        })));
    }
    Ok(elements.pop())
}

/// FR-380: `objects` lists no more objects of the population `bound` limits
/// than its authored maximum; otherwise `cardinality_out_of_bound`/
/// `above-maximum` naming the maximum and the count, as an unbounded
/// reading of a population with a declared maximum refuses (FR-084).
fn check_authored_bound(objects: &ObjectEnvironment, bound: &AuthoredBound) -> Result<(), Halt> {
    match authored_bound_refusal(objects, bound).map_err(Halt::Fault)? {
        Some(refusal) => Err(Halt::Stop(Stop::Refused(refusal))),
        None => Ok(()),
    }
}

/// The evaluation that refuses with `refusal` and originates nowhere.
pub(super) fn refused_evaluation(refusal: Refusal) -> Evaluation {
    Evaluation {
        outcome: FamilyOutcome::Evaluated(Outcome::Refused(refusal)),
        location: None,
        losses: Vec::new(),
        frame_depth: 0,
        peak_live_occurrences: 0,
    }
}

/// The refusal [`check_authored_bound`] gives, when `objects` lists more
/// objects of the population than `bound` allows.
pub(super) fn authored_bound_refusal(
    objects: &ObjectEnvironment,
    bound: &AuthoredBound,
) -> Result<Option<Refusal>, InternalFault> {
    let Some(count) = objects
        .population_size(&bound.population.node)
        .filter(|count| *count > bound.maximum)
    else {
        return Ok(None);
    };
    let maximum = CardinalityBound::new(0, bound.maximum)
        .map_err(|_| InternalFault::new("S6a", "authored-population-bound-invalid"))?;
    Ok(Some(Refusal::CardinalityOutOfBound {
        violation: BoundViolation::AboveMaximum,
        kind: CollectionKind::Set,
        bound: maximum,
        count,
    }))
}

/// The limits of the scratch meter the ordering-part numeric-equality
/// decision runs on: nothing is bounded, and nothing it charges reaches the
/// evaluation's own meter (FR-397 "Evaluate once").
const UNMETERED: filament_exact::ScalarLimits = filament_exact::ScalarLimits {
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
};

/// `function.call`: one work unit per checked call.
fn charge_call(meter: &mut Meter) -> Result<(), Stop> {
    Ok(meter.charge(Charge::new(ChargePoint::FunctionCall))?)
}

/// The declaration identity of the concrete population a refinement row's
/// `only` ranges over, for the undefined cause's payload: the population
/// under any `where` filter of `source`.
fn population_of<'a, N: NodeSource<'a>>(source: N) -> Result<String, Halt> {
    let mut node = source;
    loop {
        match node.kind() {
            NodeKind::PopulationObjects { population } => return Ok(population.node.clone()),
            NodeKind::Query {
                visit: Visit::Filter,
                source,
                ..
            } => node = node.at(*source),
            _ => return Err(invariant("only-source-is-a-population")),
        }
    }
}

/// Whether `node` is a named population operand (FR-391).
fn is_named<'a, N: NodeSource<'a>>(node: N) -> bool {
    matches!(node.kind(), NodeKind::NamedPopulation { .. })
}

/// One charge at `point`, one work unit, no size or result units
/// (FR-107: `model.deref`, `model.navigate`, `graph.expand`, `graph.edge`,
/// `graph.result-retain`).
fn charge_named(meter: &mut Meter, point: ChargePoint) -> Result<(), Stop> {
    Ok(meter.charge(Charge::new(point))?)
}

/// A model dereference or navigation reads one semantic value occurrence.
fn charge_model_read(meter: &mut Meter, point: ChargePoint) -> Result<(), Stop> {
    Ok(meter.charge(Charge::new(point).size(LimitKind::ValueOccurrences, 1))?)
}

/// The `discovered`th `graph.expand` of one `reaches` walk (`value-accounting.md`:
/// `value_occurrences=k`, the distinct nodes discovered including this one;
/// one work unit).
fn charge_graph_expand(meter: &mut Meter, discovered: u64) -> Result<(), Stop> {
    Ok(meter.charge(
        Charge::new(ChargePoint::GraphExpand).size(LimitKind::ValueOccurrences, discovered),
    )?)
}

/// The number of nodes a walk has discovered, as a charge size.
fn discovered_count(discovered: &BTreeSet<ObjectReference>) -> u64 {
    u64::try_from(discovered.len()).unwrap_or(u64::MAX)
}

/// `graph.result-retain`: the walk's Boolean (`value-accounting.md`: one work
/// unit and one result unit).
fn charge_graph_result_retain(meter: &mut Meter) -> Result<(), Stop> {
    Ok(meter.charge(Charge::new(ChargePoint::GraphResultRetain).results(1))?)
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

/// The empty sum of the numeric family of `N`: its additive zero (FR-145).
fn sum_zero(value_type: &ValueType) -> Result<Value, Halt> {
    if let Some(zero) = collections::exact_zero(value_type) {
        return Ok(zero);
    }
    Ok(match value_type {
        ValueType::Float(float) => Value::Float(match float.width() {
            IeeeWidth::Binary32 => IeeeValue::binary32(0),
            IeeeWidth::Binary64 => IeeeValue::binary64(0),
        }),
        ValueType::Quantity(unit) => Value::Quantity(filament_exact::Quantity::new(
            Rational::from_integer(Integer::zero()),
            unit.clone(),
        )),
        // Zero is not a member of every carrier; the caller decides that.
        ValueType::CarriedQuantity(carried) => Value::Quantity(filament_exact::Quantity::new(
            Rational::from_integer(Integer::zero()),
            carried.unit().clone(),
        )),
        _ => return Err(invariant("sum-family-invalid")),
    })
}

/// Whether `value` is a member of the total type `N` of a `sum<N>`: a carried
/// quantity is a quantity of its unit inside its carrier (FR-142 "Carried
/// quantity sum"), which the kernel's `admits` does not decide.
fn sum_admits(value_type: &ValueType, value: &Value) -> bool {
    match (value_type, value) {
        (ValueType::CarriedQuantity(carried), Value::Quantity(quantity)) => {
            quantity.unit() == carried.unit()
                && carried_sum::carrier_admits(carried, quantity.magnitude())
        }
        _ => value_type.admits(value),
    }
}

/// The arguments a call popped, with whether each forwards the absent-key
/// parameter and the binder each derives from.
struct PoppedArguments {
    values: Vec<Value>,
    provenance: Vec<bool>,
    owners: Vec<Owner>,
}

/// One live occurrence for the call frame plus all occurrences retained by
/// the values bound to its parameters, matching `filament_eval::Session`.
fn frame_occurrences(arguments: &[Value]) -> Integer {
    arguments
        .iter()
        .fold(Integer::one(), |sum, argument| sum.add(&argument.occ()))
}

/// The binder of each root-frame source slot and the routing of one call's
/// charge intervals onto them (FR-520-AC-11).
struct BinderRouting {
    slots: std::collections::BTreeMap<Slot, filament_exact::NodeKey>,
    routing: ChargeRouting,
}

/// An unavailable binder read: the binder, the supplier's cause and the
/// reading location (`None` for a reader-backed read of a node whose admitted
/// wire carries no location, FR-390-AC-64).
pub(crate) type UnavailableRead = (
    filament_exact::NodeKey,
    filament_semantic_value::catalog::CatalogCode,
    Option<Location>,
);

/// Where a node is evaluated in a float ordering composition (FR-397
/// "Ordering parts"): the composition's root, its pair of operand
/// self-equalities, its tail, the negated equality of its tail, or one of its
/// three `numeric_equal` parts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Position {
    Ordering,
    Parts,
    Tail,
    NotPart,
    Part,
}

impl Position {
    fn left(self) -> Option<Self> {
        match self {
            Self::Ordering => Some(Self::Parts),
            Self::Parts => Some(Self::Part),
            Self::Tail | Self::NotPart | Self::Part => None,
        }
    }

    fn right<'a, N: NodeSource<'a>>(self, operand: N) -> Position {
        match self {
            Self::Ordering => Self::Tail,
            Self::Tail if matches!(operand.kind(), NodeKind::Not(_)) => Self::NotPart,
            Self::Parts | Self::Tail | Self::NotPart | Self::Part => Self::Part,
        }
    }
}

/// Whether an applied node charges the meter: an ordering part decides
/// without charging (FR-397 "Ordering parts").
#[derive(Clone, Copy, PartialEq, Eq)]
enum Application {
    Standalone,
    OrderingPart,
}

/// The one local or IEEE literal an ordering operand reads.
#[derive(Clone, Copy, PartialEq, Eq)]
enum OperandRead {
    Local(Slot),
    Literal(IeeeValue),
}

fn operand_read<'a, N: NodeSource<'a>>(node: N) -> Option<OperandRead> {
    match node.kind() {
        NodeKind::Local(slot) => Some(OperandRead::Local(*slot)),
        NodeKind::Literal(literal) => match N::literal(literal) {
            Value::Float(value) => Some(OperandRead::Literal(*value)),
            _ => None,
        },
        _ => None,
    }
}

/// Whether `node` is `numeric_equal(a, b)` whose operands read `a` and `b`.
fn numeric_equal_of<'a, N: NodeSource<'a>>(
    node: N,
    a: &Option<OperandRead>,
    b: &Option<OperandRead>,
) -> bool {
    let NodeKind::IeeeNumericEquality(left, right) = node.kind() else {
        return false;
    };
    operand_read(node.at(*left)) == *a && operand_read(node.at(*right)) == *b
}

/// FR-397 "Ordering parts": whether `node` is an ordering composition,
/// `and(and(ne(x, x), ne(y, y)), and(total(x, y), not(ne(x, y))))` or its
/// `or(total(x, y), ne(x, y))` tail, with `x` and `y` each one local or
/// literal read. Read from node kinds, operator identities and operand reads
/// only, so the source and the package route decide alike.
fn ordering_composition<'a, N: NodeSource<'a>>(node: N) -> bool {
    let NodeKind::Connective(Connective::And, _, parts, tail) = node.kind() else {
        return false;
    };
    let (parts, tail) = (node.at(*parts), node.at(*tail));
    let NodeKind::Connective(Connective::And, _, left, right) = parts.kind() else {
        return false;
    };
    let (left, right) = (parts.at(*left), parts.at(*right));
    let (NodeKind::IeeeNumericEquality(x, _), NodeKind::IeeeNumericEquality(y, _)) =
        (left.kind(), right.kind())
    else {
        return false;
    };
    let x = operand_read(left.at(*x));
    let y = operand_read(right.at(*y));
    if x.is_none()
        || y.is_none()
        || !numeric_equal_of(left, &x, &x)
        || !numeric_equal_of(right, &y, &y)
    {
        return false;
    }
    let NodeKind::Connective(connective, _, total, equal) = tail.kind() else {
        return false;
    };
    let (total, equal) = (tail.at(*total), tail.at(*equal));
    let NodeKind::IeeeTotalOrder(_, a, b) = total.kind() else {
        return false;
    };
    if operand_read(total.at(*a)) != x || operand_read(total.at(*b)) != y {
        return false;
    }
    match connective {
        Connective::And => match equal.kind() {
            NodeKind::Not(inner) => numeric_equal_of(equal.at(*inner), &x, &y),
            _ => false,
        },
        Connective::Or => numeric_equal_of(equal, &x, &y),
        Connective::Implies => false,
    }
}

enum Task<'a, N> {
    Eval(N),
    Apply(N),
    /// FR-397 "Ordering parts": a `numeric_equal` evaluated as an ordering
    /// part, which decides without charging.
    ApplyPart(N),
    /// Evaluate `N` at a position of an ordering composition.
    EvalAt(N, Position),
    Branch(N),
    Short(N, Option<Position>),
    /// FR-322: the `case` at this node has evaluated its scrutinee; select
    /// the arm of its active member and evaluate only that arm's body.
    Select(N),
    /// The right operand of a connective is evaluated; carries the left
    /// operand's owner, which the connective's own operation absorbs.
    Retain(N, Owner),
    /// A function call's body has returned; its result now belongs to the
    /// call application at `node`.
    ObserveApplication(N),
    Bind(Slot),
    ChargeElement(N),
    Return,
    Iterate(Box<Iteration<N>>),
    /// FR-139: a `seq(i in low .. high: e)` in progress.
    IterateSeq(Box<SeqIteration<N>>),
    /// FR-151 (TC-196 D06): the pushed frame's own effective-precondition
    /// body has just evaluated; decide it, then continue to the selected
    /// candidate's own body.
    DispatchGuard(Box<DispatchGuard<N>>),
    /// Restore the anchor a `pre(..)` or a `Call` saved before evaluating its
    /// operand/callee body.
    RestoreAnchor(Anchor),
    /// Restore the package an imported call switched from, once the
    /// imported function's body has returned (ADR-015 D-5).
    RestorePackage(Box<PackageContext<'a>>),
}

/// The package whose declarations a body evaluates against: its scope,
/// functions, dispatch tables, units and enum members. An imported call
/// evaluates the callee's body against the library's own package.
struct PackageContext<'a> {
    world: World<'a>,
    dispatch_tables: &'a [DispatchTable],
    units: UnitScope<'a>,
    enum_members: &'a EnumMemberIndex,
}

/// A dispatched call's linked candidate body, awaiting its effective
/// precondition's decision (TC-196 D06).
struct DispatchGuard<N> {
    body_function: usize,
    arguments: Vec<Value>,
    /// The `precondition-false` payload to report if the guard fails.
    failure: PreconditionFailure,
    /// The `NodeKind::Dispatch` node that pushed this guard: the dispatched
    /// `receiver.member(args)` call, whose location `Machine::run` reports
    /// if the guard fails. Without it, the location lookup falls back to
    /// the whole expression's root and `precondition-false`'s
    /// `UndefinedRecord` reports the wrong locus.
    node: N,
    /// The owner of each dispatched argument, receiver first.
    owners: Vec<Owner>,
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

/// A one-binder query or fold in progress: the shared iteration state and
/// the binder bookkeeping only this evaluator keeps.
struct Iteration<N> {
    node: N,
    state: IterationState,
    /// The binder the source derives from; element slots take it.
    source_owner: Owner,
    /// The binder the source and every result so far derive from.
    owner: Owner,
}

/// A `seq(i in low .. high: e)` in progress (FR-139).
struct SeqIteration<N> {
    node: N,
    state: Sequence,
    /// The binder the bounds derive from; the element slot takes it.
    source_owner: Owner,
    /// The binder the bounds and every result so far derive from.
    owner: Owner,
}

/// The selection a recorded read evaluated (FR-107 "Read paths").
#[derive(Clone, Copy)]
enum ReadStep<'s> {
    /// A record value's field.
    Field(&'s str),
    /// A model object's attribute.
    Attribute(&'s str),
}

/// Which value family a [`Machine`] evaluates (SR-750 FND-011 round 2):
/// set once by the constructor, never inferred from `reads.is_some()` at
/// each call to [`Machine::is_protocol_clause`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EvaluationFamily {
    /// [`Machine::new`]: the `Value` family, no state-clause observations.
    Value,
    /// [`Machine::with_pre`]: FR-107's `ProtocolClause` family (S6a).
    Protocol,
}

pub(crate) struct Machine<'a, 'm, N> {
    /// The package the body evaluates against: a checked package's scope
    /// and function table, or a bytes-only package's own world (FR-533).
    world: World<'a>,
    objects: &'a ObjectEnvironment,
    /// FR-107: a precondition's or postcondition's pre observation, distinct
    /// from `objects` (the clause's own observation: `current` for an
    /// invariant, `post` for a postcondition). `None` for the `Value`
    /// family, which has no state-clause observations at all.
    pre_objects: Option<&'a ObjectEnvironment>,
    /// FR-107: every model read's own observation, by its checked
    /// location (`CheckedStateClause::reads`), consulted only at an
    /// `Attribute` node to choose `objects` or `pre_objects`. Empty for the
    /// `Value` family.
    reads: Option<
        &'a std::collections::BTreeMap<
            Location,
            fsl_semantics::model::clause_observation::Observation,
        >,
    >,
    /// Which family this `Machine` evaluates (SR-750 FND-011 round 2): set
    /// once, explicitly, by whichever constructor built it, rather than
    /// inferred from `reads.is_some()`. [`Self::new`] always builds
    /// [`EvaluationFamily::Value`]; [`Self::with_pre`] always builds
    /// [`EvaluationFamily::Protocol`], since every call site of
    /// `with_pre` (`s6a::protocol_clause` and this module's own tests)
    /// supplies a `reads` map.
    family: EvaluationFamily,
    meter: &'m mut Meter,
    /// Completed operator occurrences and their values, when a caller needs
    /// expression-level evidence (FR-484 scalar-operation obligations).
    observed_applications: Option<&'m mut Vec<(Location, Value)>>,
    /// Capture drained at every actual machine step, including failed steps.
    operation_capture: Option<&'m mut super::OperationCapture>,
    /// Whether this checked package calls a dependency whose body is absent.
    /// Only the ValueValidity observer interprets that typed refusal.
    unavailable_imported_call: Option<&'m mut bool>,
    /// Case budget for each value-function body evaluation in this call.
    function_case_limit: Option<u64>,
    /// Function-body evaluations charged by this call and its nested calls.
    function_evaluations: Option<&'m mut u64>,
    /// Whether a nested body evaluation was refused by `max_cases`.
    function_case_limit_reached: Option<&'m mut bool>,
    dispatch_tables: &'a [DispatchTable],
    values: Vec<Value>,
    /// FR-125: whether each stack value is still the absent-key parameter
    /// being forwarded as a reference, rather than read by an operation.
    value_guard_provenance: Vec<bool>,
    frames: Vec<Vec<Option<Value>>>,
    /// Live occurrence charge retained for each call frame while it is on
    /// the stack; ordinary clause and expression roots retain zero or their
    /// separately admitted expression-root charge.
    frame_live_occurrences: Vec<u64>,
    /// The session's call-frame live-occurrence limit.
    live_occurrences_limit: u64,
    /// The current and peak occurrence counts of this machine's call frames.
    live_occurrences: u64,
    peak_live_occurrences: u64,
    /// A scheduler transition context cloned for this synchronous machine.
    live_context: Option<LiveContext>,
    peak_live_occurrences_sink: Option<Arc<AtomicU64>>,
    /// Per-slot provenance parallel to `frames`.
    frame_guard_provenance: Vec<Vec<bool>>,
    /// FR-390 source slots whose caller supplied an unavailable value.
    unavailable_slots: std::collections::BTreeMap<
        Slot,
        (
            filament_exact::NodeKey,
            filament_semantic_value::catalog::CatalogCode,
        ),
    >,
    /// The first unavailable read, recorded by the task loop so the
    /// protocol-slot API can retain the exact read location. A reader-backed
    /// read has no location.
    unavailable_read: Option<UnavailableRead>,
    /// FR-520-AC-11: the binder each root-frame source slot holds, and the
    /// routing of this call's charge intervals onto those binders.
    binder_routing: Option<BinderRouting>,
    /// FR-520-AC-11: the binder each stack value and frame slot derives from.
    owners: Option<Owners>,
    tasks: Vec<Task<'a, N>>,
    losses: Vec<LocatedLoss>,
    /// The anchor `allInstances`/`lookup` currently read; `pre(..)` toggles
    /// it for its operand and `Task::RestoreAnchor` restores it after.
    anchor: Anchor,
    /// The package's quantity units, then every compound unit a product or
    /// quotient formed during this evaluation.
    units: UnitScope<'a>,
    /// ADR-013 T-6 (last sentence): the checked `VariantId -> EnumValue`
    /// index, built once with the `Scope` and borrowed here,
    /// consulted by the `Enum` equality schedule and `OrderedKind::Enums`
    /// -- a bare kernel
    /// `Value::Enum` (O-14/OQ-D) carries no declaration, ordered flag or
    /// case name of its own.
    enum_members: &'a EnumMemberIndex,
    /// The `ImportedCall` nodes whose library bodies are running, outermost
    /// first. A halt inside a library body is located at the outermost one,
    /// the call in the caller's own graph: the library's node locations
    /// name another package's source (ADR-015 D-5).
    imported: Vec<N>,
    /// FR-265: what a state-clause evaluation records beside its outcome;
    /// `None` for every other evaluation.
    trail: Option<Trail>,
    /// How many queries and folds are in progress: a node is on the
    /// claim's own level only when none is.
    iterations: usize,
    /// The `Attribute` nodes whose latest read came from a slot FR-106
    /// check 6.5 witnessed out of range: the provenance an equality needs
    /// before it compares such an operand instead of faulting.
    witnessed_reads: std::collections::BTreeSet<Location>,
    /// FR-125 Instancing: the claim's `over` parameter when the state holds
    /// under its key no object conforming to the parameter's type. Every
    /// read through it is `absent-key` undefined, except the reference
    /// operand of a `lookup` (the guard).
    absent_parameters: Vec<AbsentParameter>,
    /// FR-487: the component environment each model read of a hyper atom's
    /// condition observes, when the machine runs one.
    components: Option<ComponentReads<'a>>,
    /// FR-1000: this subject's fixed/created decision for pending reference
    /// literals; absent outside model-state evaluation.
    reference_bindings: Option<&'a ReferenceBindings>,
    /// FR-1000/W38: the subject table for each hyper component. A created
    /// literal selects the table indexed by its checked `@ trace` location.
    component_reference_bindings: Option<&'a [&'a ReferenceBindings]>,
}

/// The component environments a hyper atom's condition reads (FR-487
/// "Evaluating an atom").
#[derive(Clone, Copy)]
pub(crate) struct ComponentReads<'a> {
    /// One observation per trace variable, in quantifier order.
    pub(crate) environments: &'a [&'a ObjectEnvironment],
    /// One pre-state observation per trace variable, when evaluating a model
    /// relation over transitions.
    pub(crate) pre_environments: Option<&'a [&'a ObjectEnvironment]>,
    /// The component each model read observes, keyed by the read's location.
    pub(crate) reads: &'a std::collections::BTreeMap<Location, usize>,
    /// The observation selected for each component read, when the caller has
    /// both pre- and post-state observations.
    pub(crate) observations: Option<
        &'a std::collections::BTreeMap<
            Location,
            fsl_semantics::model::clause_observation::Observation,
        >,
    >,
}

/// A parameter of a claim at a key with no conforming object.
struct AbsentParameter {
    /// The local slot holding the parameter.
    slot: Slot,
    /// The population the instance was listed under, named in the
    /// `absent-key` value.
    binding: String,
    /// The key the slot holds, which tells several absent slots apart; `None`
    /// for a claim's one `over` parameter.
    key: Option<ObjectReference>,
}

impl<'a, 'm, N: NodeSource<'a>> Machine<'a, 'm, N> {
    /// A value-family evaluator over the declarations held by a bytes-only
    /// package and the caller's admitted object environment.
    pub(crate) fn from_bare_package(
        world: &'a fsl_package::BareWorld,
        objects: &'a ObjectEnvironment,
        meter: &'m mut Meter,
    ) -> Self {
        Self::build(
            World::Bytes(world),
            objects,
            None,
            None,
            meter,
            &[],
            EvaluationFamily::Value,
        )
    }

    /// The bytes-only counterpart of [`Self::observing_applications`].
    pub(crate) fn observing_bare_package(
        world: &'a fsl_package::BareWorld,
        objects: &'a ObjectEnvironment,
        meter: &'m mut Meter,
        observation: super::ApplicationObserver<'m>,
    ) -> Self {
        let mut machine = Self::from_bare_package(world, objects, meter);
        machine.observed_applications = Some(observation.observed);
        machine.operation_capture = Some(observation.capture);
        machine.unavailable_imported_call = Some(observation.unavailable_imported_call);
        machine.function_case_limit = Some(observation.function_case_limit);
        machine.function_evaluations = Some(observation.function_evaluations);
        machine.function_case_limit_reached = Some(observation.function_case_limit_reached);
        machine
    }

    pub(crate) fn new(
        scope: &'a Scope,
        graph: &'a CheckedGraph,
        objects: &'a ObjectEnvironment,
        meter: &'m mut Meter,
        dispatch_tables: &'a [DispatchTable],
    ) -> Self {
        Self::build(
            World::Checked { scope, graph },
            objects,
            None,
            None,
            meter,
            dispatch_tables,
            EvaluationFamily::Value,
        )
    }

    /// Retain compound units formed by the checker for this exact checked
    /// body. Evaluation may read those identities at conversion boundaries.
    pub(crate) fn with_formed_units(mut self, formed: &FormedUnits) -> Self {
        formed.seed(&mut self.units);
        self
    }

    /// Apply the configured live-occurrence bound to calls held in this
    /// machine's frame stack.
    pub(crate) fn with_session_limits(mut self, limits: SessionLimits) -> Self {
        self.live_occurrences_limit = limits.live_occurrences;
        self
    }

    /// Enforce the enclosing scheduler's aggregate count for this machine.
    pub(crate) fn with_live_context(mut self, context: LiveContext) -> Self {
        self.live_context = Some(context);
        self
    }

    pub(crate) fn with_peak_live_occurrences_sink(mut self, sink: Arc<AtomicU64>) -> Self {
        self.peak_live_occurrences_sink = Some(sink);
        self
    }

    /// A value-family evaluator that records each completed application and
    /// its result. An occurrence whose operand or operator stopped is absent
    /// from the trace.
    pub(crate) fn observing_applications(
        scope: &'a Scope,
        graph: &'a CheckedGraph,
        objects: &'a ObjectEnvironment,
        meter: &'m mut Meter,
        dispatch_tables: &'a [DispatchTable],
        observation: super::ApplicationObserver<'m>,
    ) -> Self {
        let mut machine = Self::new(scope, graph, objects, meter, dispatch_tables);
        machine.observed_applications = Some(observation.observed);
        machine.operation_capture = Some(observation.capture);
        machine.unavailable_imported_call = Some(observation.unavailable_imported_call);
        machine.function_case_limit = Some(observation.function_case_limit);
        machine.function_evaluations = Some(observation.function_evaluations);
        machine.function_case_limit_reached = Some(observation.function_case_limit_reached);
        machine
    }

    /// [`Self::new`], additionally carrying a precondition's or
    /// postcondition's pre observation and the clause's own model-read
    /// observations (FR-107): `pre_objects` is consulted at an `Attribute`
    /// node whose location `reads` maps to `Observation::Pre`. Both are
    /// `None` for every caller but [`super::protocol_clause`]'s S6a hook,
    /// which always supplies `reads` (`EvaluationFamily::Protocol`).
    pub(crate) fn with_pre(
        scope: &'a Scope,
        graph: &'a CheckedGraph,
        objects: &'a ObjectEnvironment,
        pre_objects: Option<&'a ObjectEnvironment>,
        reads: Option<
            &'a std::collections::BTreeMap<
                Location,
                fsl_semantics::model::clause_observation::Observation,
            >,
        >,
        meter: &'m mut Meter,
        dispatch_tables: &'a [DispatchTable],
    ) -> Self {
        Self::with_world(
            World::Checked { scope, graph },
            objects,
            pre_objects,
            reads,
            meter,
            dispatch_tables,
        )
    }

    /// [`Self::with_pre`] over any package world: a checked package's, or the
    /// world a bytes-only package gives (FR-533).
    pub(crate) fn with_world(
        world: World<'a>,
        objects: &'a ObjectEnvironment,
        pre_objects: Option<&'a ObjectEnvironment>,
        reads: Option<
            &'a std::collections::BTreeMap<
                Location,
                fsl_semantics::model::clause_observation::Observation,
            >,
        >,
        meter: &'m mut Meter,
        dispatch_tables: &'a [DispatchTable],
    ) -> Self {
        Self::build(
            world,
            objects,
            pre_objects,
            reads,
            meter,
            dispatch_tables,
            EvaluationFamily::Protocol,
        )
    }

    fn build(
        world: World<'a>,
        objects: &'a ObjectEnvironment,
        pre_objects: Option<&'a ObjectEnvironment>,
        reads: Option<
            &'a std::collections::BTreeMap<
                Location,
                fsl_semantics::model::clause_observation::Observation,
            >,
        >,
        meter: &'m mut Meter,
        dispatch_tables: &'a [DispatchTable],
        family: EvaluationFamily,
    ) -> Self {
        let enum_members = world.enum_member_index();
        Self {
            world,
            objects,
            pre_objects,
            reads,
            family,
            meter,
            observed_applications: None,
            operation_capture: None,
            unavailable_imported_call: None,
            function_case_limit: None,
            function_evaluations: None,
            function_case_limit_reached: None,
            dispatch_tables,
            values: Vec::new(),
            value_guard_provenance: Vec::new(),
            frames: Vec::new(),
            frame_live_occurrences: Vec::new(),
            live_occurrences_limit: SessionLimits::default().live_occurrences,
            live_occurrences: 0,
            peak_live_occurrences: 0,
            live_context: None,
            peak_live_occurrences_sink: None,
            frame_guard_provenance: Vec::new(),
            unavailable_slots: std::collections::BTreeMap::new(),
            unavailable_read: None,
            binder_routing: None,
            owners: None,
            tasks: Vec::new(),
            losses: Vec::new(),
            anchor: Anchor::Post,
            units: UnitScope::new(world.types().units()),
            enum_members,
            imported: Vec::new(),
            trail: None,
            iterations: 0,
            witnessed_reads: std::collections::BTreeSet::new(),
            absent_parameters: Vec::new(),
            components: None,
            reference_bindings: None,
            component_reference_bindings: None,
        }
    }

    /// FR-1000: evaluate against the reference literals admitted with the
    /// model subject that owns the current state.
    pub(crate) fn with_reference_bindings(mut self, bindings: &'a ReferenceBindings) -> Self {
        self.reference_bindings = Some(bindings);
        self
    }

    /// W38: bind hyper literal occurrences to the component table selected
    /// by the checked innermost `@ trace` component.
    pub(crate) fn with_component_reference_bindings(
        mut self,
        bindings: &'a [&'a ReferenceBindings],
    ) -> Self {
        self.component_reference_bindings = Some(bindings);
        self
    }

    /// FR-125 Instancing: `slot` carries absent-key guard provenance when the
    /// instance has no conforming object; `binding` names its population.
    pub(crate) fn with_absent_parameter(mut self, slot: Slot, binding: String) -> Self {
        self.absent_parameters.push(AbsentParameter {
            slot,
            binding,
            key: None,
        });
        self
    }

    /// [`Self::with_absent_parameter`] for several slots that alias the one
    /// absent object (an `on each` guard's `over` and trigger binders).
    pub(crate) fn with_absent_parameters(mut self, slots: Vec<Slot>, binding: String) -> Self {
        for slot in slots {
            self = self.with_absent_parameter(slot, binding.clone());
        }
        self
    }

    /// FR-487: `slot` holds `key`, which names no object of `binding`'s
    /// population in its component; any number of slots may be absent.
    pub(crate) fn with_absent_key(
        mut self,
        slot: Slot,
        binding: String,
        key: ObjectReference,
    ) -> Self {
        self.absent_parameters.push(AbsentParameter {
            slot,
            binding,
            key: Some(key),
        });
        self
    }

    /// FR-487: read each model read of the condition in the component
    /// `components` records for it.
    pub(crate) fn with_components(mut self, components: ComponentReads<'a>) -> Self {
        self.components = Some(components);
        self
    }

    /// FR-265: record `trail` while evaluating (a state clause's own
    /// evaluation, or FR-268's separation check).
    pub(crate) fn with_trail(mut self, trail: Trail) -> Self {
        self.trail = Some(trail);
        self
    }

    /// Whether a node evaluated now is on the claim's own level of a
    /// traced evaluation: outside every query or fold body and every
    /// called function's body.
    fn on_claim_level(&self) -> bool {
        self.trail.is_some()
            && self.frames.len() == 1
            && self.iterations == 0
            && self.imported.is_empty()
    }

    /// FR-265: retain a completed authored comparison on the claim's own
    /// level, alongside its result and operands.
    fn note_comparison(&mut self, node: N, left: &Value, right: &Value, result: bool) {
        if !self.on_claim_level() {
            return;
        }
        let Some(location) = node.location() else {
            return;
        };
        // A source node's occurrence is the one its checked graph records at
        // the location; a package node carries its `protocol_locations`
        // row's occurrence.
        let Some(comparison) = self
            .world
            .graph()
            .and_then(|graph| occurrence_at(graph, location, Some(Operator::Binary)))
            .or_else(|| node.occurrence())
        else {
            return;
        };
        if let Some(trail) = self.trail.as_mut() {
            trail.compared(
                location,
                ComparisonReport {
                    comparison,
                    operands: [left.clone(), right.clone()],
                    result,
                },
            );
        }
    }

    /// Evaluate against `package` from now on, returning the package this
    /// machine evaluated against until now.
    fn enter(&mut self, package: PackageContext<'a>) -> PackageContext<'a> {
        PackageContext {
            world: std::mem::replace(&mut self.world, package.world),
            dispatch_tables: std::mem::replace(&mut self.dispatch_tables, package.dispatch_tables),
            units: std::mem::replace(&mut self.units, package.units),
            enum_members: std::mem::replace(&mut self.enum_members, package.enum_members),
        }
    }

    /// Evaluate `root` with `arguments` in its first slots.
    ///
    /// **No entry-level `function.call` charge here (PR #302 review finding
    /// 2).** `ValueFunctionFamily::evaluate` (`family.rs`) charges the
    /// top-level call's `function.call` to the caller's meter, then runs this
    /// machine against that same meter, so a charge here would
    /// count the one call twice. Every *nested* `NodeKind::Call`/dispatch
    /// site this method's own task loop reaches charges `function.call`
    /// against `self.meter` (`charge_call`, called directly at those sites)
    /// -- those are genuinely separate calls.
    ///
    /// **`Err(InternalFault)` (FR-090-AC-10, FR-096-AC-15).** Two distinct
    /// invariant breaks return `Err` from here, never `Ok(Evaluation {
    /// outcome: Outcome::Refused(_), .. })`: [`Self::resolve_population`]
    /// returns `Err(Stop::Fault(_))` rather than a `Refusal` when a
    /// `Value::Population` argument reaches it unresolved or with a
    /// mismatched declared maximum -- a condition admission already rules
    /// out for any checked program reached through `CheckedPackage::call` or
    /// `CheckedPackage::evaluate` -- and this loop matches `Stop::Fault` out
    /// the moment a task fails, before `Self::stopped` ever converts the
    /// remaining `Stop` shapes into an `Outcome`; and a kernel
    /// `Stop::Refused(Refusal::CheckedInvariant { .. })`, which `Self::stopped`
    /// itself turns into `Err(InternalFault)` rather than an `Outcome`
    /// (FR-096-AC-15).
    pub(crate) fn run(
        self,
        root: N,
        slots: usize,
        arguments: Vec<Value>,
    ) -> Result<Evaluation, InternalFault> {
        self.run_with_root_frame(root, slots, arguments, false, false)
    }

    /// Evaluate a call frame whose `function.call` charge was already granted.
    pub(crate) fn run_call(
        self,
        root: N,
        slots: usize,
        arguments: Vec<Value>,
    ) -> Result<Evaluation, InternalFault> {
        self.run_with_root_frame(root, slots, arguments, true, true)
    }

    /// Evaluate an expression root, which contributes a live frame to nested
    /// calls but has no call charge and is not itself tested against the bound.
    pub(crate) fn run_expression(
        self,
        root: N,
        slots: usize,
        arguments: Vec<Value>,
    ) -> Result<Evaluation, InternalFault> {
        self.run_with_root_frame(root, slots, arguments, true, false)
    }

    fn run_with_root_frame(
        mut self,
        root: N,
        slots: usize,
        arguments: Vec<Value>,
        count_root: bool,
        enforce_root: bool,
    ) -> Result<Evaluation, InternalFault> {
        let root_location = root.location().cloned();
        // The scheduler already admitted this root frame. Its context starts
        // with that charge, so the mirrored Machine frame must not charge it
        // again.
        let (count_root, enforce_root) = if self.live_context.is_some() {
            (false, false)
        } else {
            (count_root, enforce_root)
        };
        if let Err(halt) = self.enter_frame(slots, arguments, count_root, enforce_root) {
            return match halt {
                Halt::Stop(stop) => self.stopped(stop, root_location.as_ref()),
                Halt::Fault(fault) => Err(fault),
                _ => Err(InternalFault::new(
                    "S6a",
                    "call-frame-limit-returned-non-stop",
                )),
            };
        }
        let mut evaluation = self.drive(root)?;
        evaluation.peak_live_occurrences = self.live_peak();
        if let Some(sink) = self.peak_live_occurrences_sink.as_ref() {
            sink.fetch_max(evaluation.peak_live_occurrences, AtomicOrdering::Relaxed);
        }
        Ok(evaluation)
    }

    /// [`Self::run`] over a frame given slot by slot: `frame[i]` is local
    /// slot `i`, `None` for a slot the body may bind but is not given
    /// (ADR-027 ST-1: a protocol slot reads only the binders bound at it).
    pub(crate) fn run_frame(
        mut self,
        root: N,
        frame: Vec<Option<Value>>,
    ) -> Result<Evaluation, InternalFault> {
        let provenance = vec![false; frame.len()];
        self.push_frame(frame, provenance);
        self.drive(root)
    }

    /// FR-390's protocol-slot frame with unavailable caller sources. An
    /// unavailable source stops only when the checked expression reads its
    /// local slot, preserving all charges made before that read.
    pub(crate) fn run_protocol_frame(
        mut self,
        root: N,
        frame: Vec<Option<Value>>,
        unavailable: std::collections::BTreeMap<
            Slot,
            (
                filament_exact::NodeKey,
                filament_semantic_value::catalog::CatalogCode,
            ),
        >,
        binder_slots: std::collections::BTreeMap<Slot, filament_exact::NodeKey>,
    ) -> Result<(Result<Evaluation, UnavailableRead>, BinderCharges), InternalFault> {
        self.unavailable_slots = unavailable;
        self.binder_routing = Some(BinderRouting {
            slots: binder_slots,
            routing: ChargeRouting::new(),
        });
        let provenance = vec![false; frame.len()];
        let mut source_owners = vec![Owner::None; frame.len()];
        if let Some(binders) = self.binder_routing.as_ref() {
            for slot in binders.slots.keys() {
                if let Some(owner) = source_owners.get_mut(*slot) {
                    *owner = Owner::Binder(*slot);
                }
            }
        }
        self.owners = Some(Owners::default());
        self.push_frame(frame, provenance);
        if let Some(owners) = self.owners.as_mut() {
            owners
                .set_frame(source_owners)
                .ok_or_else(|| InternalFault::new("FR-520", "protocol-root-frame-owners"))?;
        }
        let result = match self.drive(root) {
            Ok(evaluation) => Ok(evaluation),
            Err(fault) => match self.unavailable_read.take() {
                Some(read) => Err(read),
                None => return Err(fault),
            },
        };
        let mut routing = self
            .binder_routing
            .take()
            .map(|binders| binders.routing)
            .ok_or_else(|| InternalFault::new("FR-520", "protocol-slot-routing-present"))?;
        routing.attribute(self.meter, None).ok_or_else(|| {
            InternalFault::new("FR-520", "protocol-slot-meter-captures-charge-intervals")
        })?;
        Ok((result, routing.finish()))
    }

    /// FR-265: [`Self::run`] over a state clause's body, returning the
    /// evaluation and the trail recorded beside it. The machine must carry
    /// a trail ([`Self::with_trail`]).
    pub(crate) fn run_traced(
        mut self,
        root: N,
        slots: usize,
        arguments: Vec<Value>,
    ) -> Result<(Evaluation, Trail), InternalFault> {
        self.enter_frame(slots, arguments, false, false)
            .map_err(|halt| match halt {
                Halt::Fault(fault) => fault,
                _ => InternalFault::new("S6a", "traced-root-frame-unexpected-limit"),
            })?;
        let evaluation = self.drive(root)?;
        let trail = self
            .trail
            .take()
            .ok_or_else(|| InternalFault::new("S6a", "traced-evaluation-carries-a-trail"))?;
        Ok((evaluation, trail))
    }

    /// Push the outermost frame: `arguments` in its first slots.
    fn enter_frame(
        &mut self,
        slots: usize,
        arguments: Vec<Value>,
        count_occurrences: bool,
        enforce_limit: bool,
    ) -> Result<(), Halt> {
        let occurrences = if count_occurrences {
            frame_occurrences(&arguments)
        } else {
            Integer::zero()
        };
        let mut frame: Vec<Option<Value>> = arguments.into_iter().map(Some).collect();
        frame.resize(slots.max(frame.len()), None);
        let mut provenance = vec![false; frame.len()];
        for absent in &self.absent_parameters {
            if let Some(slot) = provenance.get_mut(absent.slot) {
                *slot = true;
            }
        }
        self.push_frame_owned(frame, provenance, Vec::new(), occurrences, enforce_limit)
    }

    fn push_frame(&mut self, frame: Vec<Option<Value>>, provenance: Vec<bool>) {
        // Protocol and clause frames are not pure-function call frames.
        if let Some(tracked) = self.owners.as_mut() {
            tracked.push_frame(Vec::new(), frame.len());
        }
        self.frames.push(frame);
        self.frame_guard_provenance.push(provenance);
        self.frame_live_occurrences.push(0);
    }

    /// Push a frame whose first slots derive from `owners`, one per argument.
    fn push_frame_owned(
        &mut self,
        frame: Vec<Option<Value>>,
        mut provenance: Vec<bool>,
        owners: Vec<Owner>,
        next_charge: Integer,
        enforce_limit: bool,
    ) -> Result<(), Halt> {
        let (occurrences, total) = if let Some(context) = self.live_context.as_mut() {
            if enforce_limit {
                (context.open(&next_charge)?, self.live_occurrences)
            } else {
                (0, self.live_occurrences)
            }
        } else {
            let admitted_occurrences = next_charge.to_u64();
            let occurrences = admitted_occurrences.unwrap_or(u64::MAX);
            let total = self.live_occurrences.checked_add(occurrences);
            if enforce_limit
                && (admitted_occurrences.is_none()
                    || total.is_none_or(|total| total > self.live_occurrences_limit))
            {
                return Err(Halt::Stop(Stop::Incomplete(Incomplete {
                    limit_kind: LimitKind::LiveOccurrences,
                    limit: self.live_occurrences_limit,
                    consumed: self.live_occurrences,
                    next_charge,
                    charge_point: ChargePoint::FunctionCall,
                })));
            }
            (occurrences, total.unwrap_or(u64::MAX))
        };
        provenance.resize(frame.len(), false);
        if let Some(tracked) = self.owners.as_mut() {
            tracked.push_frame(owners, frame.len());
        }
        self.frames.push(frame);
        self.frame_guard_provenance.push(provenance);
        self.frame_live_occurrences.push(occurrences);
        self.live_occurrences = total;
        self.peak_live_occurrences = self.peak_live_occurrences.max(total);
        Ok(())
    }

    fn pop_live_frame(&mut self) -> Result<(), Halt> {
        let Some(occurrences) = self.frame_live_occurrences.pop() else {
            return Err(invariant("evaluation-live-occurrence-frame-missing"));
        };
        if let Some(context) = self.live_context.as_mut() {
            if occurrences > 0 {
                context.close(occurrences);
            }
        } else {
            self.live_occurrences = self
                .live_occurrences
                .checked_sub(occurrences)
                .ok_or_else(|| invariant("evaluation-live-occurrence-count-underflow"))?;
        }
        Ok(())
    }

    fn live_peak(&self) -> u64 {
        self.live_context
            .as_ref()
            .map_or(self.peak_live_occurrences, LiveContext::peak)
    }

    /// Evaluate `root` on the current frame to its outcome: the task loop
    /// [`Self::run`] and FR-268's separation check share.
    fn drive(&mut self, root: N) -> Result<Evaluation, InternalFault> {
        self.tasks.push(Task::Eval(root));
        while let Some(task) = self.tasks.pop() {
            // The task's node, borrowed from the checked tree, not
            // its `Location`: a location is cloned only when a halt reports
            // one, never once per task.
            let located: N = match &task {
                Task::Eval(node)
                | Task::Apply(node)
                | Task::Branch(node)
                | Task::Short(node, _)
                | Task::ApplyPart(node)
                | Task::EvalAt(node, _)
                | Task::Select(node)
                | Task::Retain(node, _)
                | Task::ObserveApplication(node)
                | Task::ChargeElement(node) => *node,
                Task::Iterate(iteration) => iteration.node,
                Task::IterateSeq(iteration) => iteration.node,
                Task::DispatchGuard(guard) => guard.node,
                Task::Bind(_) | Task::Return | Task::RestoreAnchor(_) | Task::RestorePackage(_) => {
                    root
                }
            };
            let applied = matches!(
                &task,
                Task::Apply(node)
                    if !matches!(
                        node.kind(),
                        NodeKind::Call { .. }
                            | NodeKind::ImportedCall { .. }
                            | NodeKind::Dispatch { .. }
                    )
            );
            if let Some(owners) = self.owners.as_mut() {
                owners.begin_step();
            }
            let on_clause_frame = self.trail.is_some() && self.frames.len() == 1;
            let stepped = self.step(task);
            // FR-269 (TC-3121): a call the clause's own frame makes into a
            // function body, kept so a stop inside it names the clause's call.
            if on_clause_frame && self.frames.len() > 1 {
                if let Some(trail) = self.trail.as_mut() {
                    trail.enter_call(located.location());
                }
            }
            if let Some(capture) = self.operation_capture.as_deref_mut() {
                capture.drain(self.meter, located.location());
            }
            self.route_step_charges()?;
            if let Err(halt) = stepped {
                if self.frames.len() > 1 {
                    if let Some(trail) = self.trail.as_mut() {
                        trail.stop_in_call();
                    }
                }
                let located = self.imported.first().copied().unwrap_or(located);
                // A reader node reports only the location its admitted wire
                // carries, never one of the source package (FR-390-AC-64).
                let reported_location = located.location();
                return match halt {
                    Halt::Fault(fault) => Err(fault),
                    Halt::Unavailable { binder, cause } => {
                        self.unavailable_read = Some((binder, cause, reported_location.cloned()));
                        Err(InternalFault::new(
                            "S6a",
                            "protocol-slot-unavailable-source-read",
                        ))
                    }
                    // FR-090-OQ-3 ruling: a family-owned result is located
                    // like a `Stop`, at the task that produced it.
                    Halt::Family(result) => Ok(Evaluation {
                        outcome: FamilyOutcome::FamilyEvaluated(result),
                        location: reported_location.cloned(),
                        losses: Vec::new(),
                        frame_depth: self.frame_depth(),
                        peak_live_occurrences: 0,
                    }),
                    Halt::Stop(stop) => self.stopped(stop, reported_location),
                    // An imported call site still locates at that call.
                    Halt::Located(at) => self.stopped(
                        at.stop,
                        match self.imported.first() {
                            Some(call) => call.location(),
                            None => at.location.as_ref(),
                        },
                    ),
                };
            } else if applied {
                if let Some(observed) = self.observed_applications.as_deref_mut() {
                    if let Some(value) = self.values.last() {
                        if let Some(location) = located.location() {
                            observed.push((location.clone(), value.clone()));
                        }
                    }
                }
            }
        }
        let value = match self.pop() {
            Ok(value) => value,
            Err(Halt::Fault(fault)) => return Err(fault),
            Err(Halt::Unavailable { .. }) => {
                return Err(InternalFault::new(
                    "S6a",
                    "protocol-slot-unavailable-source-read",
                ));
            }
            Err(Halt::Family(result)) => {
                return Ok(Evaluation {
                    outcome: FamilyOutcome::FamilyEvaluated(result),
                    location: root.location().cloned(),
                    losses: Vec::new(),
                    frame_depth: self.frame_depth(),
                    peak_live_occurrences: 0,
                });
            }
            Err(Halt::Stop(stop)) => return self.stopped(stop, root.location()),
            Err(Halt::Located(at)) => {
                return self.stopped(at.stop, at.location.as_ref());
            }
        };
        if !self.values.is_empty() || !self.value_guard_provenance.is_empty() {
            return Err(InternalFault::new("S6a", "evaluation-result-stack-invalid"));
        }
        Ok(Evaluation {
            outcome: FamilyOutcome::Evaluated(Outcome::Completed(value)),
            location: None,
            losses: std::mem::take(&mut self.losses),
            frame_depth: 0,
            peak_live_occurrences: 0,
        })
    }

    /// FR-268 (ADR-031 SW-13): the separation check's steps 2 to 4 over
    /// `quantifier`, a `forall` or `exists` node of a state clause's claim
    /// that step 1 resolved, with the clause's bindings in its first slots.
    /// `lets` are the `let` nodes on the path from the claim's root to
    /// `quantifier`, outermost first: their values are bound before the
    /// domain evaluates (ADR-031 SW-4). The machine must carry a trail, so
    /// the domain's provenance is recorded. Every evaluation is charged to
    /// this machine's meter.
    pub(crate) fn separate(
        mut self,
        slots: usize,
        arguments: Vec<Value>,
        lets: &[N],
        quantifier: N,
        claim: WitnessClaim<'_>,
    ) -> Result<Separation, InternalFault> {
        let fault = || InternalFault::new("S6a", "separation-check-over-a-checked-claim");
        let NodeKind::Query {
            visit,
            slot,
            source,
            body,
            ..
        } = quantifier.kind()
        else {
            return Err(fault());
        };
        let (source, body) = (quantifier.at(*source), quantifier.at(*body));
        self.enter_frame(slots, arguments, false, false)
            .map_err(|halt| match halt {
                Halt::Fault(fault) => fault,
                _ => InternalFault::new("S6a", "separation-root-frame-unexpected-limit"),
            })?;
        for binding in lets {
            let NodeKind::Let { slot, value, .. } = binding.kind() else {
                return Err(fault());
            };
            match completed(self.drive(binding.at(*value))?) {
                Ok(bound) => *self.slot(*slot).map_err(|_| fault())? = Some(bound),
                Err(stopped) => return Ok(Separation::Stopped(SeparationStep::Domain, stopped)),
            }
        }
        let domain = match completed(self.drive(source)?) {
            Ok(Value::Collection(domain)) => domain,
            Ok(_) => return Err(fault()),
            Err(stopped) => return Ok(Separation::Stopped(SeparationStep::Domain, stopped)),
        };
        let Some(provenance) = self.provenance_of(&domain, source) else {
            return Err(fault());
        };
        let found = (0..domain.elements().len())
            .find(|j| provenance.position(*j) == Some(claim.index))
            .and_then(|j| domain.elements().get(j));
        let Some(element) = found else {
            return Ok(Separation::Unmet(SeparationStep::Element));
        };
        if provenance.path(claim.index) != *claim.value_path
            || compare_keys(element, claim.element) != Some(Ordering::Equal)
        {
            return Ok(Separation::Unmet(SeparationStep::Element));
        }
        *self.slot(*slot).map_err(|_| fault())? = Some(element.clone());
        let separates = match completed(self.drive(body)?) {
            Ok(Value::Boolean(holds)) => match visit {
                Visit::Forall => !holds,
                Visit::Exists => holds,
                Visit::Map | Visit::Filter | Visit::Count | Visit::Sum | Visit::Only => {
                    return Err(fault());
                }
            },
            Ok(_) => return Err(fault()),
            Err(stopped) => return Ok(Separation::Stopped(SeparationStep::Body, stopped)),
        };
        Ok(if separates {
            Separation::Holds
        } else {
            Separation::Unmet(SeparationStep::Body)
        })
    }

    /// FR-265: where `collection`'s elements are stored: its recorded
    /// provenance (a member or a record field read from a stored object, a
    /// conversion or selection of a stored collection, a literal), or, for a
    /// collection with no stored location (computed by a call), the
    /// collection the expression at `node` builds inside the claim
    /// (FSpec FR-207).
    fn provenance_of(&self, collection: &Arc<CollectionValue>, node: N) -> Option<Provenance> {
        let trail = self.trail.as_ref()?;
        if let Some(provenance) = trail.provenance(&Value::Collection(Arc::clone(collection))) {
            return Some(provenance.clone());
        }
        let occurrence = occurrence_at(self.world.graph()?, node.location()?, None)?;
        Some(Provenance::built(trail.observation.clone(), occurrence))
    }

    /// Converts a stop to an `Evaluation`. A kernel `CheckedInvariant` is an
    /// S6a invariant break, an `InternalFault` and never a refusal record
    /// (FR-096-AC-19, observed through
    /// [`fsl_semantics::check::ValueFunctionFamily::evaluate`]).
    fn stopped(
        &self,
        stop: Stop,
        location: Option<&Location>,
    ) -> Result<Evaluation, InternalFault> {
        let mut evaluation = Self::stopped_at_depth(stop, location, self.frame_depth())?;
        evaluation.peak_live_occurrences = self.live_peak();
        Ok(evaluation)
    }

    fn stopped_at_depth(
        stop: Stop,
        location: Option<&Location>,
        frame_depth: u32,
    ) -> Result<Evaluation, InternalFault> {
        if let Stop::Refused(Refusal::CheckedInvariant { cause }) = stop {
            return Err(InternalFault::from_kernel(
                "S6a",
                "checked-program-invariant",
                cause,
            ));
        }
        Ok(Evaluation {
            outcome: FamilyOutcome::Evaluated(outcome_from_stop(Err(stop))),
            location: location.cloned(),
            losses: Vec::new(),
            frame_depth,
            peak_live_occurrences: 0,
        })
    }

    /// The call frames above the selected call's own frame (FR-100).
    fn frame_depth(&self) -> u32 {
        u32::try_from(self.frames.len().saturating_sub(1)).unwrap_or(u32::MAX)
    }

    fn pop(&mut self) -> Result<Value, Halt> {
        Ok(self.pop_owned()?.0)
    }

    /// [`Self::pop`] with the owner of the popped value.
    fn pop_owned(&mut self) -> Result<(Value, Owner), Halt> {
        let value = self
            .values
            .pop()
            .ok_or_else(|| invariant("evaluation-value-stack-underflow"))?;
        let provenance = self
            .value_guard_provenance
            .pop()
            .ok_or_else(|| invariant("evaluation-provenance-stack-underflow"))?;
        let owner = self.pop_owners(1)?.first().copied().unwrap_or(Owner::None);
        if provenance {
            return Err(self.absent_key_halt(&value)?);
        }
        Ok((value, owner))
    }

    /// Pop a value while preserving whether it forwards the absent-key
    /// parameter through a let binding, lookup reference, or helper argument.
    fn pop_forwarded(&mut self) -> Result<(Value, bool), Halt> {
        let value = self
            .values
            .pop()
            .ok_or_else(|| invariant("evaluation-value-stack-underflow"))?;
        let provenance = self
            .value_guard_provenance
            .pop()
            .ok_or_else(|| invariant("evaluation-provenance-stack-underflow"))?;
        self.pop_owners(1)?;
        Ok((value, provenance))
    }

    fn absent_key_halt(&self, value: &Value) -> Result<Halt, Halt> {
        let Value::Reference(reference) = value else {
            return Err(invariant("absent-parameter-reference-expected"));
        };
        let absent = self
            .absent_parameters
            .iter()
            .find(|absent| absent.key.as_ref() == Some(reference))
            .or_else(|| self.absent_parameters.first())
            .ok_or_else(|| invariant("guard-provenance-without-absent-parameter"))?;
        let key = render_identity(reference.object().as_str().as_bytes());
        Ok(Halt::Family(FamilyResult::Undefined(Box::new(
            StateModelUndefined::AbsentKey {
                binding: absent.binding.clone(),
                key,
            },
        ))))
    }

    fn push_value(&mut self, value: Value, provenance: bool) {
        self.values.push(value);
        self.value_guard_provenance.push(provenance);
        if let Some(owners) = self.owners.as_mut() {
            owners.push_value();
        }
    }

    /// Pop the owners of the top `count` values; none when owners are untracked.
    fn pop_owners(&mut self, count: usize) -> Result<Vec<Owner>, Halt> {
        let Some(owners) = self.owners.as_mut() else {
            return Ok(Vec::new());
        };
        owners
            .take_values(count)
            .ok_or_else(|| invariant("evaluation-owner-stack-underflow"))
    }

    fn pop_frame_owners(&mut self) -> Result<(), Halt> {
        match self.owners.as_mut() {
            Some(owners) => owners
                .pop_frame()
                .ok_or_else(|| invariant("evaluation-owner-frame-underflow")),
            None => Ok(()),
        }
    }

    /// The owner of the operands the running step has popped so far.
    fn step_owner(&self) -> Owner {
        self.owners.as_ref().map_or(Owner::None, Owners::step)
    }

    fn absorb_owner(&mut self, owner: Owner) {
        if let Some(owners) = self.owners.as_mut() {
            owners.absorb(owner);
        }
    }

    fn set_slot_owner(&mut self, slot: Slot, owner: Owner) -> Result<(), Halt> {
        match self.owners.as_mut() {
            Some(owners) => owners
                .set_slot(slot, owner)
                .ok_or_else(|| invariant("evaluation-owner-frame-underflow")),
            None => Ok(()),
        }
    }

    /// [`Self::pop_many`] with the owner of each popped value.
    fn pop_many_owned(&mut self, count: usize) -> Result<(Vec<Value>, Vec<Owner>), Halt> {
        let start = self
            .values
            .len()
            .checked_sub(count)
            .ok_or_else(|| invariant("evaluation-value-stack-underflow"))?;
        let values = self.values.split_off(start);
        let provenance = self.value_guard_provenance.split_off(start);
        let owners = self.pop_owners(count)?;
        if let Some(index) = provenance.iter().position(|provenance| *provenance) {
            let reference = values
                .get(index)
                .ok_or_else(|| invariant("forwarded-value-missing"))?;
            return Err(self.absent_key_halt(reference)?);
        }
        Ok((values, owners))
    }

    fn pop_many(&mut self, count: usize) -> Result<Vec<Value>, Halt> {
        Ok(self.pop_many_owned(count)?.0)
    }

    fn pop_many_forwarded(&mut self, count: usize) -> Result<PoppedArguments, Halt> {
        let start = self
            .values
            .len()
            .checked_sub(count)
            .ok_or_else(|| invariant("evaluation-value-stack-underflow"))?;
        let owners = self.pop_owners(count)?;
        Ok(PoppedArguments {
            values: self.values.split_off(start),
            provenance: self.value_guard_provenance.split_off(start),
            owners,
        })
    }

    fn pop_integer(&mut self) -> Result<Integer, Halt> {
        match self.pop()? {
            Value::Integer(value) => Ok(value),
            _ => Err(invariant("integer-value-expected")),
        }
    }

    fn pop_boolean(&mut self) -> Result<bool, Halt> {
        match self.pop()? {
            Value::Boolean(value) => Ok(value),
            _ => Err(invariant("boolean-value-expected")),
        }
    }

    fn pop_collection(&mut self) -> Result<Arc<CollectionValue>, Halt> {
        match self.pop()? {
            Value::Collection(collection) => Ok(collection),
            _ => Err(invariant("collection-value-expected")),
        }
    }

    fn pop_rational(&mut self) -> Result<Rational, Halt> {
        match self.pop()? {
            Value::Rational(value) => Ok(value),
            _ => Err(invariant("rational-value-expected")),
        }
    }

    fn pop_decimal(&mut self) -> Result<Decimal, Halt> {
        match self.pop()? {
            Value::Decimal(value) => Ok(value),
            _ => Err(invariant("decimal-value-expected")),
        }
    }

    /// FR-397 "Conversion into a float": a float operand converts across
    /// widths and an exact operand rounds once, each through the owned
    /// `filament_exact` operation under the target's mode.
    fn convert_to_float(
        &mut self,
        node: N,
        value: &Value,
        target: FloatType,
    ) -> Result<Value, Halt> {
        if !self.world.has_ieee_profile() {
            return Err(invariant("ieee-profile-unresolved"));
        }
        let (width, rounding) = (target.width(), target.rounding());
        let outcome = match value {
            Value::Float(float) => convert_ieee_width(*float, width, rounding, self.meter),
            Value::Integer(integer) => exact_to_ieee(integer, width, rounding, self.meter),
            Value::Rational(rational) => exact_to_ieee(rational, width, rounding, self.meter),
            Value::Decimal(decimal) => exact_to_ieee(decimal, width, rounding, self.meter),
            _ => return Err(invariant("ieee-conversion-operand-expected")),
        };
        let result = outcome_into_stop(outcome)?;
        if result.flags() != IeeeFlags::EMPTY {
            self.record(node, ValueLoss::IeeeFlags(result.flags()))?;
        }
        Ok(Value::Float(result.value()))
    }

    /// Record `loss` at `node`. A loss is always located: a reader node
    /// whose admitted wire carries no location cannot report one, and its
    /// evaluation faults rather than drop or invent the location (FR-390).
    fn record(&mut self, node: N, loss: ValueLoss) -> Result<(), Halt> {
        let location = node
            .location()
            .ok_or_else(|| invariant("value-loss-site-is-located"))?;
        self.losses.push(LocatedLoss {
            location: location.clone(),
            loss,
        });
        Ok(())
    }

    /// One step of `sum<N>` (FR-145): the first summand seeds the running
    /// total with no addition, and each later one is added by the family of
    /// `N`. A seed or total outside `N`'s domain is the located undefined
    /// outcome, never the family's membership refusal.
    fn sum_step(
        &mut self,
        node: N,
        body: N,
        total: Option<Value>,
        summand: Value,
    ) -> Result<Value, Halt> {
        let value_type = node.value_type();
        // The exact families run the shared step, which the JIT runs too.
        if collections::is_exact_sum(value_type) {
            let stepped = collections::sum_exact(value_type, total, summand, self.meter)
                .ok_or_else(|| invariant("sum-family-invalid"))?;
            let (value, loss) = stepped.map_err(|halt| match halt {
                collections::Halt::SumSeedOutOfDomain => Halt::Located(Box::new(LocatedStop {
                    stop: Stop::Undefined(Undefined::SumOutOfDomain),
                    location: body.location().cloned(),
                })),
                other => lift(other),
            })?;
            if let Some(loss) = loss {
                self.record(node, ValueLoss::Decimal(loss))?;
            }
            return Ok(value);
        }
        let Some(total) = total else {
            if !sum_admits(value_type, &summand) {
                return Err(Halt::Located(Box::new(LocatedStop {
                    stop: Stop::Undefined(Undefined::SumOutOfDomain),
                    location: body.location().cloned(),
                })));
            }
            return Ok(summand);
        };
        match (value_type, total, summand) {
            (ValueType::Float(float), Value::Float(total), Value::Float(summand)) => {
                if !self.world.has_ieee_profile() {
                    return Err(invariant("ieee-profile-unresolved"));
                }
                let result = outcome_into_stop(
                    evaluate_ieee(
                        IeeeOperation::Add(total, summand),
                        float.rounding(),
                        self.meter,
                    )
                    .map_err(|_| invariant("ieee-operation-rejected"))?,
                )?;
                if result.flags() != IeeeFlags::EMPTY {
                    self.record(node, ValueLoss::IeeeFlags(result.flags()))?;
                }
                Ok(Value::Float(result.value()))
            }
            (ValueType::Quantity(_), Value::Quantity(total), Value::Quantity(summand)) => {
                let (Some(left), Some(right)) =
                    (self.units.resolve(&total), self.units.resolve(&summand))
                else {
                    return Err(unresolved_unit());
                };
                let (outcome, unit) =
                    evaluate_quantity_unit(QuantityOperation::Add(left, right), self.meter)
                        .map_err(|_| invariant("quantity-operation-rejected"))?;
                let quantity = outcome_into_stop(outcome)?;
                self.units.form(unit);
                Ok(Value::Quantity(quantity))
            }
            (
                ValueType::CarriedQuantity(carried),
                Value::Quantity(total),
                Value::Quantity(summand),
            ) => {
                // FR-142: no unit edge, no rounding and no loss record; the
                // bound is decided after every addition.
                let quantity =
                    carried_sum::add(carried, &total, &summand, self.meter).map_err(Halt::from)?;
                Ok(Value::Quantity(quantity))
            }
            _ => Err(invariant("sum-accumulator-value-invalid")),
        }
    }

    /// Evaluate a decimal operation into `target`, recording its loss.
    fn decimal(
        &mut self,
        node: N,
        operation: DecimalOperation<'_>,
        target: &DecimalType,
    ) -> Result<Value, Halt> {
        let result = outcome_into_stop(evaluate_decimal(operation, target, self.meter))?;
        if let Some(loss) = result.loss() {
            self.record(node, ValueLoss::Decimal(loss.clone()))?;
        }
        Ok(Value::Decimal(result.value().clone()))
    }

    /// Bind `value` to `slot` in the current frame.
    fn bind(&mut self, slot: Slot, value: Value) -> Result<(), Halt> {
        self.bind_with_provenance(slot, value, false)
    }

    fn bind_with_provenance(
        &mut self,
        slot: Slot,
        value: Value,
        provenance: bool,
    ) -> Result<(), Halt> {
        if let Some(trail) = self.trail.as_mut() {
            trail.element_paths.remove(&slot);
        }
        *self.slot(slot)? = Some(value);
        *self.frame_provenance_slot(slot)? = provenance;
        let owner = self.step_owner();
        self.set_slot_owner(slot, owner)
    }

    fn slot_has_guard_provenance(&self, slot: Slot) -> Result<bool, Halt> {
        self.frame_guard_provenance
            .last()
            .and_then(|frame| frame.get(slot))
            .cloned()
            .ok_or_else(|| invariant("evaluation-local-provenance-unbound"))
    }

    fn frame_provenance_slot(&mut self, slot: Slot) -> Result<&mut bool, Halt> {
        self.frame_guard_provenance
            .last_mut()
            .and_then(|frame| frame.get_mut(slot))
            .ok_or_else(|| invariant("evaluation-local-provenance-unbound"))
    }

    /// FR-322: select the one arm of the `case` at `node` whose member is the
    /// scrutinee's active member, bind its payload positionally and queue only
    /// that arm's body. Selecting and binding charge nothing. A scrutinee that
    /// matches no arm broke an invariant admission (FR-321) and
    /// exhaustiveness (FR-318) establish.
    fn select_arm(&mut self, node: N) -> Result<(), Halt> {
        let NodeKind::Case {
            declaration, arms, ..
        } = node.kind()
        else {
            return Err(invariant("evaluation-case-node-expected"));
        };
        let Value::Union(scrutinee) = self.pop()? else {
            return Err(invariant("case-scrutinee-union-expected"));
        };
        let (position, _) = self
            .world
            .types()
            .union_member(declaration.clone(), scrutinee.variant().clone())
            .ok_or_else(|| invariant("case-scrutinee-member-unresolved"))?;
        let arm = arms
            .iter()
            .find(|arm| arm.member == position)
            .ok_or_else(|| invariant("case-arm-unmatched"))?;
        if arm.binders.len() != scrutinee.payload().len() {
            return Err(invariant("case-arm-binder-arity"));
        }
        for (slot, value) in arm.binders.iter().zip(scrutinee.payload()) {
            self.bind(*slot, value.clone())?;
        }
        self.tasks.push(Task::Eval(node.at(arm.body)));
        Ok(())
    }

    fn slot(&mut self, slot: Slot) -> Result<&mut Option<Value>, Halt> {
        self.frames
            .last_mut()
            .and_then(|frame| frame.get_mut(slot))
            .ok_or_else(|| invariant("evaluation-local-slot-unresolved"))
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
    /// [`admit_invocation`]: fsl_semantics::model::population::admit_invocation
    /// [`admit_binding`]: fsl_semantics::model::population::admit_binding
    fn select_anchor<'x>(
        &self,
        binding: &'x PopulationBinding,
    ) -> Result<&'x PopulationBinding, Halt> {
        match self.anchor {
            Anchor::Post => Ok(binding),
            // ADR-013 T-6, FR-090-AC-7: a `ProtocolClause` family-owned
            // refusal.
            Anchor::Pre => binding.pre_anchor().ok_or_else(|| {
                Halt::Family(FamilyResult::Refused(Box::new(
                    // The pre anchor is required; the binding was admitted
                    // with its post selection only (no `pre_anchor`).
                    ProtocolClauseSnapshot::WrongAnchor {
                        required: "pre",
                        supplied: "post",
                    },
                )))
            }),
        }
    }

    /// FR-089-AC-3/AC-4/AC-5, FR-090-AC-10: resolves `population_id` (an
    /// `allInstances`/`lookup` population operand's own identity) to the
    /// admitted `PopulationBinding` this evaluation's own recorded
    /// correspondence (`self.objects`, `model`'s
    /// `admit_binding`/`admit_invocation` mint into) recorded it against, by
    /// lookup alone -- never by decoding `population_id`'s own bytes.
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
        maximum: Option<u64>,
    ) -> Result<&'a PopulationBinding, Halt> {
        match self.objects.resolve_population(population_id) {
            // The declared maximum matches exactly, absence included: an
            // unbounded population binds only an unbounded parameter.
            Some(binding) if binding.declared_maximum() == maximum => Ok(binding),
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

    /// FR-106 check 7 / FR-046-AC-6: no population query may return a
    /// successful value from a snapshot that does not establish complete
    /// membership. Both explicit function arguments and named model
    /// populations apply this rule to their own selected observation.
    fn require_complete_population(
        objects: &ObjectEnvironment,
        population: &DeclarationKey,
    ) -> Result<(), Halt> {
        let listed = objects
            .population_completeness()
            .contains_key(&population.node);
        if !objects.population_complete(&population.node) {
            return Err(Halt::Family(FamilyResult::Refused(Box::new(
                NamedPopulationIncomplete {
                    population: population.node.clone(),
                    listed,
                },
            ))));
        }
        Ok(())
    }

    /// Charge one nested value-function body against FR-484's `max_cases`.
    fn enter_function_case(&mut self) -> Result<(), Halt> {
        let Some(limit) = self.function_case_limit else {
            return Ok(());
        };
        let Some(evaluations) = self.function_evaluations.as_deref_mut() else {
            return Err(invariant("value-check-function-case-counter-present"));
        };
        if *evaluations >= limit {
            if let Some(reached) = self.function_case_limit_reached.as_deref_mut() {
                *reached = true;
            }
            return Err(Halt::Stop(Stop::Incomplete(Incomplete {
                limit_kind: LimitKind::WorkUnits,
                limit,
                consumed: limit,
                next_charge: Integer::from(1_i64),
                charge_point: ChargePoint::FunctionCall,
            })));
        }
        *evaluations += 1;
        Ok(())
    }

    fn step(&mut self, task: Task<'a, N>) -> Result<(), Halt> {
        match task {
            Task::Eval(node) => self.eval(node),
            Task::Apply(node) => self.apply(node),
            Task::ApplyPart(node) => self.apply_ordering_part(node),
            Task::EvalAt(node, position) => self.eval_at(node, position),
            Task::Branch(node) => {
                let NodeKind::If {
                    then, otherwise, ..
                } = node.kind()
                else {
                    return Err(invariant("evaluation-branch-node-expected"));
                };
                let condition = self.pop_boolean()?;
                self.decide(node, condition);
                let taken = if condition { then } else { otherwise };
                self.tasks.push(Task::Eval(node.at(*taken)));
                Ok(())
            }
            Task::Short(node, position) => {
                let NodeKind::Connective(connective, origin, _, right) = node.kind() else {
                    return Err(invariant("evaluation-connective-node-expected"));
                };
                let (left, left_owner) = match self.pop_owned()? {
                    (Value::Boolean(value), owner) => (value, owner),
                    _ => return Err(invariant("boolean-value-expected")),
                };
                // Only a connective the source wrote has a decision (FR-265);
                // the checker's own composition of a float ordering has none.
                if matches!(origin, ConnectiveOrigin::Authored) {
                    self.decide(node, left);
                }
                let skipped = match connective {
                    Connective::And => (!left).then_some(false),
                    Connective::Or => left.then_some(true),
                    Connective::Implies => (!left).then_some(true),
                };
                match skipped {
                    Some(result) => {
                        let result = retain_boolean(result, self.meter)?;
                        self.push_value(Value::Boolean(result), false);
                    }
                    None => {
                        self.tasks.push(Task::Retain(node, left_owner));
                        let right_position =
                            position.map(|position| position.right(node.at(*right)));
                        self.push_operand(node.at(*right), right_position);
                    }
                }
                Ok(())
            }
            Task::Retain(_, left_owner) => {
                self.absorb_owner(left_owner);
                let result = retain_boolean(self.pop_boolean()?, self.meter)?;
                self.push_value(Value::Boolean(result), false);
                Ok(())
            }
            Task::ObserveApplication(node) => {
                if let Some(observed) = self.observed_applications.as_deref_mut() {
                    let value = self
                        .values
                        .last()
                        .ok_or_else(|| invariant("evaluation-call-result-missing"))?;
                    if let Some(location) = node.location() {
                        observed.push((location.clone(), value.clone()));
                    }
                }
                Ok(())
            }
            Task::Bind(slot) => {
                let (value, provenance) = self.pop_forwarded()?;
                self.bind_with_provenance(slot, value, provenance)
            }
            Task::Select(node) => self.select_arm(node),
            Task::ChargeElement(_) => charge_element(self.meter).map_err(Into::into),
            Task::Return => {
                self.frames
                    .pop()
                    .ok_or_else(|| invariant("evaluation-return-frame-missing"))?;
                self.frame_guard_provenance
                    .pop()
                    .ok_or_else(|| invariant("evaluation-return-provenance-frame-missing"))?;
                self.pop_live_frame()?;
                self.pop_frame_owners()?;
                Ok(())
            }
            Task::DispatchGuard(guard) => {
                let DispatchGuard {
                    body_function,
                    arguments,
                    failure,
                    node,
                    owners,
                } = *guard;
                let holds = self.pop_boolean()?;
                self.frames
                    .pop()
                    .ok_or_else(|| invariant("evaluation-return-frame-missing"))?;
                self.frame_guard_provenance
                    .pop()
                    .ok_or_else(|| invariant("evaluation-return-provenance-frame-missing"))?;
                self.pop_live_frame()?;
                self.pop_frame_owners()?;
                if !holds {
                    // ADR-013 O-16, FR-090-AC-11: a `StateModel` family-owned
                    // undefined result.
                    return Err(Halt::Family(FamilyResult::Undefined(Box::new(
                        StateModelUndefined::PreconditionFalse(failure),
                    ))));
                }
                let callable = node
                    .callee(self.world.graph(), body_function)
                    .ok_or_else(|| invariant("evaluation-callable-unresolved"))?;
                // The call is made over its arguments, so its charge belongs to
                // the binder they derive from.
                for owner in &owners {
                    self.absorb_owner(*owner);
                }
                charge_call(self.meter)?;
                let next_charge = frame_occurrences(&arguments);
                let mut frame: Vec<Option<Value>> = arguments.into_iter().map(Some).collect();
                frame.resize(callable.slots.max(frame.len()), None);
                let provenance = vec![false; frame.len()];
                self.push_frame_owned(frame, provenance, owners, next_charge, true)?;
                self.tasks.push(Task::ObserveApplication(node));
                self.tasks.push(Task::Return);
                self.tasks.push(Task::Eval(callable.body));
                Ok(())
            }
            Task::Iterate(iteration) => self.iterate(iteration),
            Task::IterateSeq(iteration) => self.iterate_seq(iteration),
            Task::RestoreAnchor(previous) => {
                self.anchor = previous;
                Ok(())
            }
            Task::RestorePackage(previous) => {
                self.enter(*previous);
                self.imported.pop();
                Ok(())
            }
        }
    }

    /// FR-520-AC-11: a read of the root frame's source `slot` registers its
    /// binder as read.
    fn route_binder_read(&mut self, slot: Slot) {
        if self.frames.len() != 1 {
            return;
        }
        if let Some(binders) = self.binder_routing.as_mut() {
            if let Some(binder) = binders.slots.get(&slot) {
                binders.routing.register(binder);
            }
        }
    }

    /// Drain the meter's interval onto the binder that owns the step just run.
    fn route_step_charges(&mut self) -> Result<(), InternalFault> {
        let owner = self.step_owner();
        let Some(binders) = self.binder_routing.as_mut() else {
            return Ok(());
        };
        let binder = match owner {
            Owner::Binder(slot) => binders.slots.get(&slot),
            Owner::None | Owner::Mixed => None,
        };
        binders
            .routing
            .attribute(self.meter, binder)
            .ok_or_else(|| {
                InternalFault::new("FR-520", "protocol-slot-meter-captures-charge-intervals")
            })
    }

    fn eval_local(&mut self, slot: Slot) -> Result<(), Halt> {
        self.route_binder_read(slot);
        let owner = self
            .owners
            .as_ref()
            .map_or(Owner::None, |owners| owners.slot(slot));
        self.absorb_owner(owner);
        if let Some((binder, cause)) = self.unavailable_slots.get(&slot) {
            return Err(Halt::Unavailable {
                binder: binder.clone(),
                cause: *cause,
            });
        }
        let value = self
            .slot(slot)?
            .clone()
            .ok_or_else(|| invariant("evaluation-local-value-unbound"))?;
        let provenance = self.slot_has_guard_provenance(slot)?;
        self.push_value(value, provenance);
        Ok(())
    }

    /// Queue `node`, evaluated at `position` of an ordering composition when
    /// it has one.
    fn push_operand(&mut self, node: N, position: Option<Position>) {
        self.tasks.push(match position {
            Some(position) => Task::EvalAt(node, position),
            None => Task::Eval(node),
        });
    }

    /// FR-397 "Ordering parts": evaluate `node`, reached along an ordering
    /// composition's operand edges at `position`.
    fn eval_at(&mut self, node: N, position: Position) -> Result<(), Halt> {
        match (position, node.kind()) {
            (
                Position::Ordering | Position::Parts | Position::Tail,
                NodeKind::Connective(_, _, left, _),
            ) => {
                self.tasks.push(Task::Short(node, Some(position)));
                self.push_operand(node.at(*left), position.left());
                Ok(())
            }
            (Position::NotPart, NodeKind::Not(operand)) => {
                self.tasks.push(Task::Apply(node));
                self.push_operand(node.at(*operand), Some(Position::Part));
                Ok(())
            }
            (Position::Part, NodeKind::IeeeNumericEquality(left, right)) => {
                self.tasks.push(Task::ApplyPart(node));
                self.tasks.push(Task::Eval(node.at(*right)));
                self.tasks.push(Task::Eval(node.at(*left)));
                Ok(())
            }
            _ => Err(invariant("ordering-composition-position-mismatch")),
        }
    }

    fn apply(&mut self, node: N) -> Result<(), Halt> {
        self.apply_as(node, Application::Standalone)
    }

    fn apply_ordering_part(&mut self, node: N) -> Result<(), Halt> {
        self.apply_as(node, Application::OrderingPart)
    }

    fn eval(&mut self, node: N) -> Result<(), Halt> {
        match node.kind() {
            NodeKind::ReferenceLiteral {
                object_type,
                identity,
            } => {
                let key = (object_type, identity);
                let component_binding = self
                    .components
                    .and_then(|components| {
                        node.location()
                            .and_then(|location| components.reads.get(location))
                    })
                    .and_then(|component| {
                        self.component_reference_bindings
                            .and_then(|bindings| bindings.get(*component).copied())
                    })
                    .and_then(|bindings| bindings.get(key.0, key.1));
                let binding = component_binding
                    .or_else(|| {
                        self.reference_bindings
                            .and_then(|bindings| bindings.get(key.0, key.1))
                    })
                    .or_else(|| {
                        self.component_reference_bindings.and_then(|bindings| {
                            bindings
                                .iter()
                                .find_map(|bindings| bindings.get(key.0, key.1))
                        })
                    })
                    .ok_or_else(|| invariant("reference-literal-subject-binding-exists"))?;
                match binding {
                    ReferenceBinding::Fixed(reference) => {
                        self.push_value(Value::Reference(reference.clone()), false);
                    }
                    ReferenceBinding::CreatedObject {
                        object_type,
                        identity,
                        population,
                        populations,
                    } => {
                        let objects = self.objects_for(node)?;
                        self.log_reference_membership_read(node, objects, populations);
                        let reference = populations.iter().find_map(|population| {
                            objects
                                .population_members(&population.node)
                                .into_iter()
                                .flatten()
                                .find(|reference| {
                                    reference.object().as_str() == identity
                                        && self.world.types().conforms(
                                            reference.object_type().clone(),
                                            object_type.clone(),
                                        )
                                })
                        });
                        let Some(reference) = reference else {
                            return Err(Halt::Family(FamilyResult::Undefined(Box::new(
                                StateModelUndefined::AbsentKey {
                                    binding: population.node.clone(),
                                    key: render_identity(identity.as_bytes()),
                                },
                            ))));
                        };
                        self.push_value(Value::Reference(reference.clone()), false);
                    }
                }
                return Ok(());
            }
            NodeKind::Literal(literal) => {
                self.push_value(N::literal(literal).clone(), false);
                return Ok(());
            }
            NodeKind::Local(slot) => return self.eval_local(*slot),
            NodeKind::Let {
                slot, value, body, ..
            } => {
                self.tasks.push(Task::Eval(node.at(*body)));
                self.tasks.push(Task::Bind(*slot));
                self.tasks.push(Task::Eval(node.at(*value)));
                return Ok(());
            }
            NodeKind::If { condition, .. } => {
                self.tasks.push(Task::Branch(node));
                self.tasks.push(Task::Eval(node.at(*condition)));
                return Ok(());
            }
            NodeKind::Connective(_, _, left, _) => {
                let position = ordering_composition(node).then_some(Position::Ordering);
                self.tasks.push(Task::Short(node, position));
                self.push_operand(node.at(*left), position.and_then(Position::left));
                return Ok(());
            }
            NodeKind::Collection { elements, .. } => {
                self.tasks.push(Task::Apply(node));
                for element in elements.iter().rev() {
                    self.tasks.push(Task::Eval(node.at(*element)));
                    self.tasks.push(Task::ChargeElement(node));
                }
                return Ok(());
            }
            NodeKind::Query { source, .. } => {
                self.tasks.push(Task::Apply(node));
                self.tasks.push(Task::Eval(node.at(*source)));
                return Ok(());
            }
            // FR-139: only the bounds are children to evaluate now; the
            // body is reached once per integer through `Task::IterateSeq`.
            NodeKind::Seq { low, high, .. } => {
                self.tasks.push(Task::Apply(node));
                self.tasks.push(Task::Eval(node.at(*high)));
                self.tasks.push(Task::Eval(node.at(*low)));
                return Ok(());
            }
            // FR-322: only the scrutinee is a child to evaluate now; the arm
            // bodies are reached through `Task::Select`.
            NodeKind::Case { scrutinee, .. } => {
                self.tasks.push(Task::Select(node));
                self.tasks.push(Task::Eval(node.at(*scrutinee)));
                return Ok(());
            }
            NodeKind::Fold {
                source, identity, ..
            } => {
                self.tasks.push(Task::Apply(node));
                if let Some(identity) = identity {
                    self.tasks.push(Task::Eval(node.at(*identity)));
                }
                self.tasks.push(Task::Eval(node.at(*source)));
                return Ok(());
            }
            // FR-391: a named population is read from the snapshot by its
            // parent, never evaluated as an operand.
            NodeKind::AllInstances { population } if is_named(node.at(*population)) => {
                self.tasks.push(Task::Apply(node));
                return Ok(());
            }
            NodeKind::Lookup {
                population,
                reference,
                ..
            } if is_named(node.at(*population)) => {
                self.tasks.push(Task::Apply(node));
                self.tasks.push(Task::Eval(node.at(*reference)));
                return Ok(());
            }
            NodeKind::Pre(operand) => {
                // FR-153: read the invocation pre population for every
                // `allInstances`/`lookup` under `operand`, then restore.
                // Identity-typed, so no `Task::Apply` follows: `operand`'s
                // own value is `pre(operand)`'s value.
                self.tasks.push(Task::RestoreAnchor(self.anchor));
                self.anchor = Anchor::Pre;
                self.tasks.push(Task::Eval(node.at(*operand)));
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

    /// FR-063-AC-7: `#[deny(...)]` closes the `_ => unsupported(...)` escape
    /// hatch this function's own `NodeKind` match -- the S3 evaluator seam
    /// -- would otherwise let slip past unnoticed by the seam probe alone.
    /// The few genuinely defensive wildcard arms further down this
    /// function's body match over a *different* type (`Value`, from
    /// `self.pop()?`, never `NodeKind`), each locally `#[allow]`-annotated
    /// (or expressed as a refutable `let ... else`, which this lint does not
    /// reach) rather than left to this `deny` -- see each site's own note.
    #[deny(clippy::wildcard_enum_match_arm)]
    fn apply_as(&mut self, node: N, application: Application) -> Result<(), Halt> {
        let value = match node.kind() {
            NodeKind::Literal(_)
            | NodeKind::ReferenceLiteral { .. }
            | NodeKind::Local(_)
            | NodeKind::Let { .. }
            | NodeKind::If { .. }
            | NodeKind::Connective(..)
            | NodeKind::Pre(_) => return Err(invariant("evaluation-apply-node-invalid")),
            // FR-322: `eval` queues a `case` through `Task::Select`, never
            // `Task::Apply`.
            NodeKind::Case { .. } => return Err(invariant("evaluation-apply-node-invalid")),
            // Quantity scaling is applied below with the source unit kept on
            // the result (FR-142 quantity scaling).
            NodeKind::QuantityScaleDiv { .. } => {
                return Err(unsupported_construct("quantity_scale"));
            }
            NodeKind::TupleProject {
                operand,
                declaration,
                position,
            } => {
                if node.at(*operand).value_type() != &ValueType::Composite(declaration.clone()) {
                    return Err(invariant("tuple-project-operand-type-mismatch"));
                }
                let Value::Composite(tuple) = self.pop()? else {
                    return Err(invariant("tuple-project-composite-value-expected"));
                };
                if tuple.declaration() != declaration {
                    return Err(invariant("tuple-project-declaration-mismatch"));
                }
                let positions = match self
                    .world
                    .types()
                    .composite(declaration.clone())
                    .map(|declaration| declaration.shape())
                {
                    Some(CompositeShape::Tuple(positions)) => positions,
                    _ => return Err(invariant("tuple-project-declaration-unresolved")),
                };
                if positions.get(*position) != Some(node.value_type()) {
                    return Err(invariant("tuple-project-position-type-mismatch"));
                }
                let Some(FieldValue::Present(value)) = tuple.slots().get(*position) else {
                    return Err(invariant("tuple-project-position-unavailable"));
                };
                // FR-143: a tuple position read retains exactly like a record
                // field projection. It adds no charge of its own.
                value.clone()
            }
            // FSpec FR-144 "Indexing": a map entry is located by its key's
            // canonical key; selection charges one `collection.element` and
            // retains the value; an absent key has no element.
            NodeKind::Index {
                kind: IndexedKind::Map,
                ..
            } => {
                let key = self.pop()?;
                let Value::Map(map) = self.pop()? else {
                    return Err(invariant("map-value-expected"));
                };
                let value = crate::value::map_entry(&map, &key)
                    .cloned()
                    .ok_or(Stop::Undefined(Undefined::NoElement))?;
                collections::charge_element(self.meter)?;
                collections::retain_accumulator(value, self.meter)?
            }
            NodeKind::Index {
                index,
                kind: kind @ (IndexedKind::Sequence | IndexedKind::OrderedSet),
                ..
            } => {
                let position = self.pop_integer()?;
                let collection = self.pop_collection()?;
                let expected_kind = match collection.collection_type().kind() {
                    CollectionKind::Sequence => IndexedKind::Sequence,
                    CollectionKind::OrderedSet => IndexedKind::OrderedSet,
                    CollectionKind::Set | CollectionKind::Bag => {
                        return Err(invariant("index-collection-kind-mismatch"));
                    }
                };
                if *kind != expected_kind
                    || node.at(*index).value_type() != &ValueType::Integer
                    || node.value_type() != collection.collection_type().element()
                {
                    return Err(invariant("index-collection-type-mismatch"));
                }
                let Some(position) = position
                    .to_u64()
                    .and_then(|position| usize::try_from(position).ok())
                else {
                    return Err(Stop::Undefined(Undefined::NoElement).into());
                };
                let Some(value) = collection.elements().get(position) else {
                    return Err(Stop::Undefined(Undefined::NoElement).into());
                };
                charge_element(self.meter)?;
                retain_scalar(value.clone(), self.meter)?
            }
            NodeKind::ConvertText(target, _) => {
                if matches!(target, ValueType::Float(_)) && !self.world.has_ieee_profile() {
                    return Err(invariant("ieee-profile-unresolved"));
                }
                let source = self.pop()?;
                let (value, loss) = text_conversion::convert(
                    self.world.types(),
                    &self.units,
                    target,
                    &source,
                    self.meter,
                )
                .map_err(Halt::from)?;
                if let Some(loss) = loss {
                    self.record(node, loss)?;
                }
                value
            }
            // FR-322: the payload arguments are evaluated and on the stack in
            // position order; the first stopped one never reaches here. Type
            // checking proved each payload against its position (FR-317), so
            // the member is only resolved and its arity confirmed -- a
            // re-admission of each payload would walk every nested union again
            // and make a depth-n construction chain quadratic. One
            // `composite.result-retain` follows the arguments.
            NodeKind::UnionValue {
                declaration,
                member,
                arguments,
            } => {
                let payload = self.pop_many(arguments.len())?;
                let binding = match self
                    .world
                    .types()
                    .composite(declaration.clone())
                    .map(|union| union.shape())
                {
                    Some(CompositeShape::Union(members)) => members
                        .get(*member)
                        .filter(|declared| declared.positions().len() == payload.len())
                        .and_then(|declared| declared.member()),
                    _ => None,
                }
                .ok_or_else(|| invariant("union-member-unresolved"))?;
                outcome_into_stop(retain_composite(
                    UnionValue::from_admitted(binding.clone(), payload),
                    self.meter,
                ))?
            }
            NodeKind::OptionSome(_) => {
                let ValueType::Option(payload_type) = node.value_type() else {
                    return Err(invariant("option-some-result-type-invalid"));
                };
                let payload = self.pop()?;
                let value = OptionValue::from_admitted((**payload_type).clone(), Some(payload));
                outcome_into_stop(retain_composite(value, self.meter))?
            }
            // FR-107: `reaches(source, target, edge)`, checked
            // only inside a state clause.
            NodeKind::Reaches { edge, .. } => {
                let Value::Reference(target) = self.pop()? else {
                    return Err(invariant("reaches-reference-value-expected"));
                };
                let Value::Reference(source) = self.pop()? else {
                    return Err(invariant("reaches-reference-value-expected"));
                };
                Value::Boolean(self.evaluate_reaches(node, &source, &target, edge)?)
            }
            // FR-365: `reaches(source, target, name)` over a relationship end,
            // read from the clause observation's extent of the relationship.
            NodeKind::ReachesEnd {
                relationship,
                near,
                population,
                ..
            } => {
                let Value::Reference(target) = self.pop()? else {
                    return Err(invariant("reaches-reference-value-expected"));
                };
                let Value::Reference(source) = self.pop()? else {
                    return Err(invariant("reaches-reference-value-expected"));
                };
                Value::Boolean(self.evaluate_reaches_end(
                    node,
                    &source,
                    &target,
                    relationship,
                    *near,
                    population.as_ref(),
                )?)
            }
            // FR-308: the value of a protocol reference is the value of its
            // protocol clause, which nothing evaluates.
            NodeKind::ProtocolReference { .. } => {
                return Err(Halt::Family(FamilyResult::Refused(Box::new(
                    ProtocolReferenceUnsupported,
                ))));
            }
            // FR-063: no arm for the probe variant under `--cfg seam_probe`
            // alone (`E0004`, this seam's evidence). The arm below exists
            // only in the probe's build of the crates above `fsl-eval`
            // (`--cfg seam_probe_eval_downstream`): `fsl-replay`
            // depends on this crate, so it must compile there for the root
            // crate's own seams to be reached at all.
            #[cfg(seam_probe_eval_downstream)]
            NodeKind::__SeamProbe => return Err(invariant("compile-only-seam-probe")),
            NodeKind::Coerce(_, target) => {
                let value = self.pop_integer()?;
                if !target.contains(&value) {
                    return Err(Stop::Refused(Refusal::IntegerOutOfDomain {
                        target: Box::new(target.clone()),
                    })
                    .into());
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
                Value::Integer(outcome_into_stop(evaluate_integer_arithmetic(
                    operation, None, self.meter,
                ))?)
            }
            NodeKind::IntegerDivision {
                operator,
                selection,
                ..
            } => {
                let right = self.pop_integer()?;
                let left = self.pop_integer()?;
                let outcome = match operator {
                    IntegerDivisionOperator::Div | IntegerDivisionOperator::Rem => {
                        let selection = selection
                            .as_ref()
                            .ok_or_else(|| invariant("integer-division-selection-missing"))?;
                        let member = match operator {
                            IntegerDivisionOperator::Div => DivisionMember::Quotient,
                            IntegerDivisionOperator::Rem => DivisionMember::Remainder,
                            IntegerDivisionOperator::Mod => {
                                return Err(invariant("integer-division-member-mismatch"));
                            }
                        };
                        integer_divide(
                            selection,
                            member,
                            &left,
                            &right,
                            &IntegerDomain::Mathematical,
                            self.meter,
                        )
                    }
                    IntegerDivisionOperator::Mod => {
                        integer_modulo(&left, &right, &IntegerDomain::Mathematical, self.meter)
                    }
                };
                Value::Integer(outcome_into_stop(outcome)?)
            }
            NodeKind::Negate(_) => match node.value_type() {
                ValueType::Integer | ValueType::Int(_) => {
                    let operand = self.pop_integer()?;
                    Value::Integer(outcome_into_stop(evaluate_integer_arithmetic(
                        IntegerArithmetic::Negate(&operand),
                        None,
                        self.meter,
                    ))?)
                }
                ValueType::Float(float) => {
                    let Value::Float(operand) = self.pop()? else {
                        return Err(invariant("ieee-float-value-expected"));
                    };
                    if !self.world.has_ieee_profile() {
                        return Err(invariant("ieee-profile-unresolved"));
                    }
                    let result = outcome_into_stop(
                        evaluate_ieee(IeeeOperation::Negate(operand), float.rounding(), self.meter)
                            .map_err(|_| invariant("ieee-operation-rejected"))?,
                    )?;
                    if result.flags() != IeeeFlags::EMPTY {
                        self.record(node, ValueLoss::IeeeFlags(result.flags()))?;
                    }
                    Value::Float(result.value())
                }
                ValueType::Boolean
                | ValueType::Rational(_)
                | ValueType::Decimal(_)
                | ValueType::Quantity(_)
                | ValueType::CarriedQuantity(_)
                | ValueType::Text(_)
                | ValueType::String
                | ValueType::Bytes
                | ValueType::JsonObject
                | ValueType::Uuid
                | ValueType::Timestamp
                | ValueType::Enum(_)
                | ValueType::Option(_)
                | ValueType::Composite(_)
                | ValueType::Collection(_)
                | ValueType::Map(_)
                | ValueType::BoundedQuantity(_)
                | ValueType::Reference(_)
                | ValueType::Population(_) => return Err(invariant("numeric-value-expected")),
            },
            NodeKind::Divide { domain, .. } => {
                let right = Rational::from_integer(self.pop_integer()?);
                let left = Rational::from_integer(self.pop_integer()?);
                Value::Rational(outcome_into_stop(evaluate_rational_arithmetic(
                    RationalArithmetic::Divide(&left, &right),
                    Some(domain),
                    self.meter,
                ))?)
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
                Value::Rational(outcome_into_stop(evaluate_rational_arithmetic(
                    operation,
                    Some(domain),
                    self.meter,
                ))?)
            }
            NodeKind::RationalNegate(_, domain) => {
                let operand = self.pop_rational()?;
                Value::Rational(outcome_into_stop(evaluate_rational_arithmetic(
                    RationalArithmetic::Negate(&operand),
                    Some(domain),
                    self.meter,
                ))?)
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
            // Not the FR-063 seam: the inner match below is over `Value`
            // (checking already fixed which two variants a `ConvertDecimal`
            // operand can be), not `NodeKind`, so its fallback is a genuine
            // invariant check, not an escape hatch over the seam's own enum.
            #[allow(clippy::wildcard_enum_match_arm)]
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
                _ => return Err(invariant("decimal-conversion-operand-invalid")),
            },
            NodeKind::Ieee(operator, rounding, _, _) => {
                let (Value::Float(right), Value::Float(left)) = (self.pop()?, self.pop()?) else {
                    return Err(invariant("ieee-float-value-expected"));
                };
                let operation = match operator {
                    ArithmeticOperator::Add => IeeeOperation::Add(left, right),
                    ArithmeticOperator::Subtract => IeeeOperation::Subtract(left, right),
                    ArithmeticOperator::Multiply => IeeeOperation::Multiply(left, right),
                    ArithmeticOperator::Divide => IeeeOperation::Divide(left, right),
                };
                if !self.world.has_ieee_profile() {
                    return Err(invariant("ieee-profile-unresolved"));
                }
                let result = outcome_into_stop(
                    evaluate_ieee(operation, *rounding, self.meter)
                        .map_err(|_| invariant("ieee-operation-rejected"))?,
                )?;
                if result.flags() != IeeeFlags::EMPTY {
                    self.record(node, ValueLoss::IeeeFlags(result.flags()))?;
                }
                Value::Float(result.value())
            }
            NodeKind::IeeeSquareRoot(rounding, _) => {
                let Value::Float(operand) = self.pop()? else {
                    return Err(invariant("ieee-float-value-expected"));
                };
                if !self.world.has_ieee_profile() {
                    return Err(invariant("ieee-profile-unresolved"));
                }
                let result = outcome_into_stop(
                    evaluate_ieee(IeeeOperation::SquareRoot(operand), *rounding, self.meter)
                        .map_err(|_| invariant("ieee-operation-rejected"))?,
                )?;
                if result.flags() != IeeeFlags::EMPTY {
                    self.record(node, ValueLoss::IeeeFlags(result.flags()))?;
                }
                Value::Float(result.value())
            }
            NodeKind::IeeeFloat32Fma(rounding, ..) | NodeKind::IeeeFloat64Fma(rounding, ..) => {
                let (Value::Float(addend), Value::Float(right), Value::Float(left)) =
                    (self.pop()?, self.pop()?, self.pop()?)
                else {
                    return Err(invariant("ieee-float-value-expected"));
                };
                if !self.world.has_ieee_profile() {
                    return Err(invariant("ieee-profile-unresolved"));
                }
                let result = outcome_into_stop(
                    evaluate_ieee(
                        IeeeOperation::FusedMultiplyAdd(left, right, addend),
                        *rounding,
                        self.meter,
                    )
                    .map_err(|_| invariant("ieee-operation-rejected"))?,
                )?;
                if result.flags() != IeeeFlags::EMPTY {
                    self.record(node, ValueLoss::IeeeFlags(result.flags()))?;
                }
                Value::Float(result.value())
            }
            NodeKind::IeeeTotalOrder(..) | NodeKind::IeeeBitIdentical(..) => {
                let comparison = if matches!(node.kind(), NodeKind::IeeeTotalOrder(..)) {
                    IeeeComparison::TotalOrder
                } else {
                    IeeeComparison::BitIdentical
                };
                let (Value::Float(right), Value::Float(left)) = (self.pop()?, self.pop()?) else {
                    return Err(invariant("ieee-float-value-expected"));
                };
                if !self.world.has_ieee_profile() {
                    return Err(invariant("ieee-profile-unresolved"));
                }
                Value::Boolean(outcome_into_stop(
                    compare_ieee(comparison, left, right, self.meter)
                        .map_err(|_| invariant("ieee-operation-rejected"))?,
                )?)
            }
            NodeKind::Quantity(operator, _, _) => {
                let (Value::Quantity(right), Value::Quantity(left)) = (self.pop()?, self.pop()?)
                else {
                    return Err(invariant("quantity-value-expected"));
                };
                let (Some(l), Some(r)) = (self.units.resolve(&left), self.units.resolve(&right))
                else {
                    return Err(unresolved_unit());
                };
                if matches!(self.world, World::Bytes(_))
                    && matches!(
                        operator,
                        ArithmeticOperator::Multiply | ArithmeticOperator::Divide
                    )
                {
                    let operation = match operator {
                        ArithmeticOperator::Multiply => {
                            filament_semantic_value::quantity::UnitOperation::Multiply
                        }
                        ArithmeticOperator::Divide => {
                            filament_semantic_value::quantity::UnitOperation::Divide
                        }
                        ArithmeticOperator::Add | ArithmeticOperator::Subtract => {
                            return Err(invariant("compound-quantity-operation-expected"));
                        }
                    };
                    let nominal = filament_semantic_value::quantity::result_unit(
                        operation,
                        l.unit(),
                        r.unit(),
                    )
                    .map_err(|_| invariant("quantity-operation-rejected"))?;
                    let expected = match node.value_type() {
                        ValueType::Quantity(unit) => unit,
                        _ => return Err(invariant("quantity-result-type-expected")),
                    };
                    let nominal = nominal
                        .id()
                        .map_err(|_| invariant("quantity-result-unit-keyed"))?;
                    if &nominal != expected {
                        return Err(unsupported_construct(
                            "dimension-equivalence-package-reconstruction",
                        ));
                    }
                }
                let operation = match operator {
                    ArithmeticOperator::Add => QuantityOperation::Add(l, r),
                    ArithmeticOperator::Subtract => QuantityOperation::Subtract(l, r),
                    ArithmeticOperator::Multiply => QuantityOperation::Multiply(l, r),
                    ArithmeticOperator::Divide => QuantityOperation::Divide(l, r),
                };
                let (outcome, unit) = evaluate_quantity_unit(operation, self.meter)
                    .map_err(|_| invariant("quantity-operation-rejected"))?;
                let quantity = outcome_into_stop(outcome)?;
                // A later operation reads this result's unit by its id; the
                // scope keeps one entry per distinct unit.
                self.units.form(unit);
                Value::Quantity(quantity)
            }
            NodeKind::QuantityScale { .. } => {
                let factor = match self.pop()? {
                    Value::Integer(value) => Rational::from_integer(value),
                    Value::Rational(value) => value,
                    Value::Decimal(value) => value.normalized().to_rational(),
                    Value::Boolean(_)
                    | Value::Float(_)
                    | Value::Quantity(_)
                    | Value::Text(_)
                    | Value::String(_)
                    | Value::Bytes(_)
                    | Value::JsonObject(_)
                    | Value::Uuid(_)
                    | Value::Timestamp(_)
                    | Value::Enum(_)
                    | Value::Population(_)
                    | Value::Option(_)
                    | Value::Composite(_)
                    | Value::Union(_)
                    | Value::Collection(_)
                    | Value::Map(_)
                    | Value::Reference(_) => {
                        return Err(invariant("quantity-scale-factor-expected"));
                    }
                };
                let Value::Quantity(quantity) = self.pop()? else {
                    return Err(invariant("quantity-scale-operand-expected"));
                };
                if self.units.resolve(&quantity).is_none() {
                    return Err(unresolved_unit());
                }
                let operation = RationalArithmetic::Multiply(quantity.magnitude(), &factor);
                self.meter.charge(
                    Charge::new(ChargePoint::UnitIdentityRead)
                        .size(LimitKind::ValueOccurrences, 1)
                        .size(LimitKind::IntegerBits, quantity.magnitude().max_part_bits()),
                )?;
                self.meter
                    .charge(Charge::new(ChargePoint::UnitRationalArithmetic).exact_size(
                        LimitKind::IntegerBits,
                        filament_exact::rational_arithmetic_bits(operation),
                    ))?;
                let magnitude = quantity.magnitude().mul(&factor);
                self.meter
                    .charge(Charge::new(ChargePoint::UnitResultRetain).results(1))?;
                Value::Quantity(Quantity::new(magnitude, quantity.unit().clone()))
            }
            NodeKind::QuantityNegate(_) => {
                let Value::Quantity(operand) = self.pop()? else {
                    return Err(invariant("quantity-negate-operand-expected"));
                };
                let Some(unit_operand) = self.units.resolve(&operand) else {
                    return Err(unresolved_unit());
                };
                let (outcome, unit) =
                    evaluate_quantity_unit(QuantityOperation::Negate(unit_operand), self.meter)
                        .map_err(|_| invariant("quantity-operation-rejected"))?;
                let quantity = outcome_into_stop(outcome)?;
                self.units.form(unit);
                Value::Quantity(quantity)
            }
            NodeKind::QuantityPower(_, _) => {
                let (Value::Integer(exponent), Value::Quantity(base)) = (self.pop()?, self.pop()?)
                else {
                    return Err(invariant("quantity-power-operands-expected"));
                };
                let Some(unit_base) = self.units.resolve(&base) else {
                    return Err(unresolved_unit());
                };
                if matches!(self.world, World::Bytes(_)) {
                    // The shared operation determines its result unit before
                    // its first charge. As in the checker's power typing,
                    // deny that charge on a separate meter: no power value
                    // is computed and the caller's meter is untouched.
                    let mut type_meter = Meter::new(filament_exact::ScalarLimits {
                        integer_bits: 0,
                        decimal_digits: 0,
                        scale_expansion: 0,
                        text_input_bytes: 0,
                        text_scalars: 0,
                        normalized_scalars: 0,
                        unit_edges: 0,
                        value_occurrences: 0,
                        work_units: 0,
                        result_units: 0,
                    });
                    let (_, nominal) = evaluate_quantity_unit(
                        QuantityOperation::Power(unit_base, &exponent),
                        &mut type_meter,
                    )
                    .map_err(|_| invariant("quantity-operation-rejected"))?;
                    if quantity_unit(node.value_type()) != Some(&nominal.id()) {
                        return Err(unsupported_construct(
                            "dimension-equivalence-package-reconstruction",
                        ));
                    }
                }
                let (outcome, unit) = evaluate_quantity_unit(
                    QuantityOperation::Power(unit_base, &exponent),
                    self.meter,
                )
                .map_err(|_| invariant("quantity-operation-rejected"))?;
                let quantity = outcome_into_stop(outcome)?;
                self.units.form(unit);
                Value::Quantity(quantity)
            }
            NodeKind::DropFirst(_) => {
                let source = self.pop_collection()?;
                let outcome = filament_eval::collection::drop_first(&source, self.meter)
                    .map_err(|_| invariant("collection-removal-rejected"))?;
                let value = outcome_into_stop(outcome)?;
                self.note_built(node, &value);
                value
            }
            NodeKind::RemoveOne(_, _) => {
                let probe = self.pop()?;
                let source = self.pop_collection()?;
                let outcome = filament_eval::collection::remove_one(
                    &source,
                    &probe,
                    self.world.types(),
                    self.meter,
                )
                .map_err(|_| invariant("collection-removal-rejected"))?;
                let value = outcome_into_stop(outcome)?;
                self.note_built(node, &value);
                value
            }
            NodeKind::Order(operator, kind, _, _) => {
                let right = self.pop()?;
                let left = self.pop()?;
                let result = self.order(*operator, *kind, &left, &right)?;
                self.note_comparison(node, &left, &right, result);
                Value::Boolean(result)
            }
            NodeKind::Equality(operator, checked, left_node, right_node) => {
                let right = self.pop()?;
                let left = self.pop()?;
                // An integer FR-106 check 6.5 witnessed outside its field's
                // declared range does not fit the static type the kernel's
                // checked equality requires of its operand. Integers
                // compare by value whatever their declared range, so such
                // a pair compares directly. Only a witnessed value gets
                // this: any other operand outside its type still faults.
                let outside_declared_range = |id, value: &Value| {
                    matches!(value, Value::Integer(_))
                        && !node.at(id).value_type().admits(value)
                        && self.reads_witnessed(node.at(id))
                };
                let result = match (&left, &right) {
                    (Value::Integer(l), Value::Integer(r))
                        if outside_declared_range(*left_node, &left)
                            || outside_declared_range(*right_node, &right) =>
                    {
                        (l == r) == (*operator == EqualityOperator::Equal)
                    }
                    _ => outcome_into_stop(checked.evaluate(
                        self.world.types(),
                        &left,
                        &right,
                        self.meter,
                    ))?,
                };
                self.note_comparison(node, &left, &right, result);
                Value::Boolean(result)
            }
            NodeKind::IeeeNumericEquality(_, _) => {
                let right = self.pop()?;
                let left = self.pop()?;
                let (Value::Float(left), Value::Float(right)) = (left, right) else {
                    return Err(Halt::Fault(InternalFault::new(
                        "S6a",
                        "typed-ieee-equality-operands-are-not-floats",
                    )));
                };
                if application == Application::Standalone {
                    // A standalone `=` or `numericEqual` is the one owned
                    // core of FR-148: it charges its operands and result.
                    if !self.world.has_ieee_profile() {
                        return Err(invariant("ieee-profile-unresolved"));
                    }
                    let result = outcome_into_stop(
                        compare_ieee(IeeeComparison::NumericEqual, left, right, self.meter)
                            .map_err(|_| invariant("ieee-operation-rejected"))?,
                    )?;
                    self.note_comparison(node, &Value::Float(left), &Value::Float(right), result);
                    Value::Boolean(result)
                } else {
                    // FR-397 "Evaluate once": the numeric-equality parts of
                    // an ordering's composition add no charge. The decision
                    // is the owned core's, run on a scratch meter with no
                    // limits so the evaluation's own meter is untouched. The
                    // checker types both operands at one width and the call
                    // boundary rejects a mismatched argument, so the core's
                    // ill-typed refusal is an invariant fault, as it is for a
                    // standalone node.
                    let mut scratch = Meter::new(UNMETERED);
                    let numeric_equal =
                        compare_ieee(IeeeComparison::NumericEqual, left, right, &mut scratch)
                            .map_err(|_| invariant("ieee-operation-rejected"))?;
                    Value::Boolean(outcome_into_stop(numeric_equal)?)
                }
            }
            NodeKind::Not(_) => {
                let operand = self.pop_boolean()?;
                Value::Boolean(outcome_into_stop(evaluate_boolean(
                    BooleanConnective::Not(operand),
                    self.meter,
                ))?)
            }
            NodeKind::Field {
                operand,
                field,
                optional,
            } => {
                let record = self.pop()?;
                let world = self.world;
                let read = read_field(
                    world.types(),
                    record,
                    field,
                    |declaration, field| world.record_slot(declaration, field),
                    *optional,
                    node.value_type(),
                )
                .map_err(invariant)?;
                self.note_field(&read.composite, read.index, &read.value);
                self.note_read(node, *operand, ReadStep::Field(field.name()), None);
                read.value
            }
            NodeKind::Attribute {
                reference: operand,
                field,
                optional,
                derefed,
            } => {
                let Value::Reference(reference) = self.pop()? else {
                    return Err(invariant("attribute-reference-value-expected"));
                };
                // FR-107's own model.deref/model.navigate charges are an
                // S6a accounting rule (`value-accounting.md`, cited by
                // FR-107's Behavior section), never a `Value`-family one:
                // this `Attribute` arm is shared by both families, and
                // charging it unconditionally would add charge units to
                // every `Value`-family attribute read that had none before
                // FR-107 (SR-750 FND-008). `model.deref` charges once per
                // explicit `deref(...)` in the source, never for a bare
                // `self.f`/`r.f` read that types identically (SR-750
                // FND-008 round 2, `derefed`, `check/ir.rs`'s own doc).
                if self.is_protocol_clause() {
                    if *derefed {
                        charge_model_read(self.meter, ChargePoint::ModelDeref)?;
                    }
                    charge_model_read(self.meter, ChargePoint::ModelNavigate)?;
                }
                let read_reference = reference.clone();
                let reference = if self.reads.is_some_and(|reads| {
                    node.location().and_then(|location| reads.get(location))
                        == Some(&fsl_semantics::model::clause_observation::Observation::Pre)
                }) {
                    self.trail
                        .as_ref()
                        .and_then(|trail| {
                            (trail.receiver.as_ref() == Some(&reference))
                                .then(|| trail.receiver_pre.clone())
                                .flatten()
                        })
                        .unwrap_or(reference)
                } else {
                    reference
                };
                let objects = self.objects_for(node)?;
                // A replay's references are unresolved: there is no object to
                // read, so the read completes no value.
                if objects.references_are_unresolved() && !objects.objects().contains(&reference) {
                    return Err(Halt::Family(FamilyResult::Refused(Box::new(
                        ReferenceUnresolved,
                    ))));
                }
                let (slot, witnessed) = self.admitted_slot(objects, &reference, field)?;
                self.log_field_read(node, objects, &reference, field)?;
                // A model read is an application, located by its own row
                // on the reader path (FR-390).
                let location = node
                    .location()
                    .ok_or_else(|| invariant("attribute-read-is-located"))?;
                if witnessed {
                    self.witnessed_reads.insert(location.clone());
                } else {
                    self.witnessed_reads.remove(location);
                }
                let value = Self::project(
                    self.world.types(),
                    slot,
                    *optional,
                    node.value_type(),
                    witnessed,
                )?;
                self.note_member(node, &reference, &field.name, &value);
                self.note_read(
                    node,
                    *operand,
                    ReadStep::Attribute(&field.name),
                    Some(&read_reference),
                );
                value
            }
            NodeKind::History { field, .. } => {
                let Value::Reference(reference) = self.pop()? else {
                    return Err(invariant("history-reference-value-expected"));
                };
                // FR-138: a history field is the refinement's, kept beside
                // the observation; `pre(..)` reads the pre-state's.
                // A failed update leaves no value to read: undetermined.
                self.objects_for(node)?
                    .history(&reference, field)
                    .cloned()
                    .ok_or_else(|| {
                        Halt::Family(FamilyResult::Refused(Box::new(HistoryUnresolved {
                            field: field.clone(),
                        })))
                    })?
            }
            NodeKind::Present(_) => {
                let Value::Option(option) = self.pop()? else {
                    return Err(invariant("option-value-expected"));
                };
                Value::Boolean(option.payload().is_some())
            }
            // FR-415-AC-20: every (some) element's presence bit the timed
            // system recorded for the owner's `Sequence<Option<Clock>>`.
            NodeKind::SlotPresence { field, all, .. } => {
                let Value::Reference(reference) = self.pop()? else {
                    return Err(invariant("slot-presence-reference-expected"));
                };
                let bits = self
                    .objects_for(node)?
                    .slot_presences(&reference, &field.name)
                    .ok_or_else(|| invariant("slot-presence-unrecorded"))?;
                Value::Boolean(if *all {
                    bits.iter().all(|bit| *bit)
                } else {
                    bits.iter().any(|bit| *bit)
                })
            }
            // FR-230 "Presence is derived": the presence the timed system
            // recorded for the owner's place.
            NodeKind::PlacePresence { field, case, .. } => {
                let Value::Reference(reference) = self.pop()? else {
                    return Err(invariant("place-presence-reference-expected"));
                };
                let present = self
                    .objects_for(node)?
                    .place_presence(&reference, &field.name, case.as_deref())
                    .ok_or_else(|| invariant("place-presence-unrecorded"))?;
                Value::Boolean(present)
            }
            // FR-230: a place reference identifies the selected clock
            // location, without reading the clock value.
            NodeKind::PlaceReference { place, slots, .. } => {
                let count = slots
                    .len()
                    .checked_add(1)
                    .ok_or_else(|| invariant("place-reference-child-count-overflow"))?;
                let mut children = self.pop_many(count)?.into_iter();
                let Some(Value::Reference(owner)) = children.next() else {
                    return Err(invariant("place-reference-owner-expected"));
                };
                let ValueType::Reference(clock_type) = node.value_type() else {
                    return Err(invariant("place-reference-result-type-expected"));
                };
                Value::Reference(clock_place_reference(
                    owner,
                    clock_type.clone(),
                    place,
                    children.collect(),
                )?)
            }
            NodeKind::Value(_) => {
                let Value::Option(option) = self.pop()? else {
                    return Err(invariant("option-value-expected"));
                };
                let payload = option
                    .payload()
                    .cloned()
                    .ok_or(Stop::Undefined(Undefined::NoneValue))?;
                self.note_unwrapped(&option, &payload);
                payload
            }
            NodeKind::Call {
                function,
                arguments,
            } => {
                let callable = node
                    .callee(self.world.graph(), *function)
                    .ok_or_else(|| invariant("evaluation-callable-unresolved"))?;
                let arguments_len = arguments.len();
                let PoppedArguments {
                    values: arguments,
                    provenance,
                    owners: argument_owners,
                } = self.pop_many_forwarded(arguments_len)?;
                let next_charge = frame_occurrences(&arguments);
                charge_call(self.meter)?;
                self.enter_function_case()?;
                let mut frame: Vec<Option<Value>> = arguments.into_iter().map(Some).collect();
                frame.resize(callable.slots.max(frame.len()), None);
                let mut frame_provenance = provenance;
                frame_provenance.resize(frame.len(), false);
                self.push_frame_owned(frame, frame_provenance, argument_owners, next_charge, true)?;
                // shared-grammar.md: "self, result and pre(...) are ...
                // unavailable ... inside a reusable predicate" -- a callee's
                // own body is never anchored by its caller's `pre(..)`, so
                // `pre(F(p))` never lets the anchor leak into `F`'s body.
                // Reset to `Post` for the callee, then restore the caller's
                // anchor once its own `Task::Eval`/`Task::Return` complete
                // (mirrors `NodeKind::Pre`'s own save/restore pair).
                self.tasks.push(Task::ObserveApplication(node));
                self.tasks.push(Task::RestoreAnchor(self.anchor));
                self.anchor = Anchor::Post;
                self.tasks.push(Task::Return);
                self.tasks.push(Task::Eval(callable.body));
                return Ok(());
            }
            // ADR-015 D-5: the callee is a function of an imported library,
            // at the index check time resolved from its node id; its body
            // evaluates against the library's own package.
            NodeKind::ImportedCall {
                callee,
                function,
                arguments,
            } => {
                let Some(library) = self.world.imported(callee) else {
                    if let Some(unavailable) = self.unavailable_imported_call.as_deref_mut() {
                        *unavailable = true;
                    }
                    return Err(invariant("evaluation-imported-graph-unresolved"));
                };
                let Some(callable) = node.callee(Some(library), *function) else {
                    if let Some(unavailable) = self.unavailable_imported_call.as_deref_mut() {
                        *unavailable = true;
                    }
                    return Err(invariant("evaluation-callable-unresolved"));
                };
                let arguments_len = arguments.len();
                let PoppedArguments {
                    values: arguments,
                    provenance,
                    owners: argument_owners,
                } = self.pop_many_forwarded(arguments_len)?;
                let next_charge = frame_occurrences(&arguments);
                charge_call(self.meter)?;
                self.enter_function_case()?;
                let mut frame: Vec<Option<Value>> = arguments.into_iter().map(Some).collect();
                frame.resize(callable.slots.max(frame.len()), None);
                let mut frame_provenance = provenance;
                frame_provenance.resize(frame.len(), false);
                self.push_frame_owned(frame, frame_provenance, argument_owners, next_charge, true)?;
                let previous = self.enter(PackageContext {
                    world: World::Checked {
                        scope: library.scope(),
                        graph: library,
                    },
                    dispatch_tables: library.dispatch_tables(),
                    units: UnitScope::new(library.scope().types().units()),
                    enum_members: library.scope().enum_member_index(),
                });
                self.imported.push(node);
                self.tasks.push(Task::ObserveApplication(node));
                self.tasks.push(Task::RestorePackage(Box::new(previous)));
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
                let value = construct_tuple(self.world.types(), declaration, arguments)
                    .map_err(invariant)?;
                outcome_into_stop(retain_composite(value, self.meter))?
            }
            NodeKind::Record { declaration, slots } => {
                let values = self.pop_many(present_slots(slots))?;
                let value = construct_record(self.world.types(), declaration, slots, values)
                    .map_err(invariant)?;
                outcome_into_stop(retain_composite(value, self.meter))?
            }
            NodeKind::Collection {
                collection_type,
                elements,
            } => {
                let occurrences = self.pop_many(elements.len())?;
                let value = outcome_into_stop(form(collection_type, occurrences, self.meter))?;
                self.note_built(node, &value);
                value
            }
            NodeKind::ConvertCollection { target, .. } => {
                let source = self.pop_collection()?;
                // Only a traced run reads the kept elements again.
                let (value, kept) =
                    collections::convert(target, &source, self.trail.is_some(), self.meter)?;
                if let Some(kept) = kept {
                    self.note_converted(&source, &kept, &value);
                }
                value
            }
            NodeKind::ConvertScalar(operand, source_node) => {
                let value = self.pop()?;
                let source_type = node.at(*source_node).value_type();
                let target_type = node.value_type();
                let carried_conversion = matches!(source_type, ValueType::CarriedQuantity(_))
                    || matches!(target_type, ValueType::CarriedQuantity(_));
                if matches!(self.world, World::Bytes(_)) {
                    let source = quantity_unit(source_type);
                    let target = quantity_unit(target_type);
                    if let (Some(source), Some(target)) = (source, target) {
                        let source = self.units.get(source.clone());
                        let target = self.units.get(target.clone());
                        if source
                            .zip(target)
                            .is_some_and(|(source, target)| !source.converts_to(target))
                        {
                            return Err(unsupported_construct(
                                "dimension-equivalence-package-reconstruction",
                            ));
                        }
                    }
                }
                if let ValueType::Float(float) = target_type {
                    self.convert_to_float(node, &value, *float)?
                } else if carried_conversion {
                    let Value::Quantity(quantity) = &value else {
                        return Err(invariant("carried-quantity-conversion-source-expected"));
                    };
                    let Some(source) = self.units.resolve(quantity) else {
                        return Err(unresolved_unit());
                    };
                    let (unit_id, target) = if let ValueType::Quantity(unit) = target_type {
                        (unit.clone(), QuantityTarget::Exact)
                    } else if let ValueType::CarriedQuantity(target) = target_type {
                        (
                            target.unit().clone(),
                            QuantityTarget::Carried(target.carrier().clone()),
                        )
                    } else {
                        return Err(invariant("carried-quantity-conversion-target-expected"));
                    };
                    if !matches!(
                        source_type,
                        ValueType::Quantity(_) | ValueType::CarriedQuantity(_)
                    ) {
                        return Err(invariant(
                            "carried-quantity-conversion-source-type-expected",
                        ));
                    }
                    let Some(unit) = self.units.get(unit_id.clone()) else {
                        return Err(unresolved_unit());
                    };
                    let conversion = convert_quantity(source, unit, &target, self.meter)
                        .map_err(|_| invariant("carried-quantity-conversion-rejected"))?;
                    let conversion = outcome_into_stop(conversion)?;
                    let magnitude = match conversion.value() {
                        ConvertedValue::Exact(exact) => exact.clone(),
                        ConvertedValue::Decimal(result) => {
                            if let Some(loss) = result.loss() {
                                self.record(node, ValueLoss::Decimal(loss.clone()))?;
                            }
                            result.value().normalized().to_rational()
                        }
                        ConvertedValue::Integer { value, loss } => {
                            if let Some(loss) = loss {
                                self.record(node, ValueLoss::Decimal(loss.clone()))?;
                            }
                            Rational::from_integer(value.value().clone())
                        }
                    };
                    Value::Quantity(Quantity::new(magnitude, unit_id))
                } else {
                    if matches!(target_type, ValueType::Integer)
                        && source_type.admits(&value)
                        && matches!(&value, Value::Rational(value) if !value.is_integer())
                    {
                        // Integer has no finite domain to put in an
                        // IntegerOutOfDomain payload. Preserve the unmet
                        // conversion obligation in the native refusal.
                        return Err(Halt::Family(FamilyResult::Refused(Box::new(
                            super::causes::NonintegralNarrowing,
                        ))));
                    }
                    let converted = operand_value(
                        self.world.types(),
                        operand,
                        &value,
                        &self.units,
                        self.meter,
                    )
                    .map_err(|stop| {
                        // Standalone narrowing can meet an unproved obligation
                        // in Kernel mode. Equality's target-admission invariant
                        // is instead the ordinary domain refusal here.
                        use filament_exact::CheckedInvariantCause;
                        let numeric_source = matches!(
                            source_type,
                            ValueType::Integer
                                | ValueType::Int(_)
                                | ValueType::Rational(_)
                                | ValueType::Decimal(_)
                        );
                        let narrowing = numeric_source
                            && !filament_semantic_value::declaration::admits_equality_conversion(
                                source_type,
                                target_type,
                                &self.units,
                            );
                        match (&stop, target_type) {
                            (
                                Stop::Refused(Refusal::CheckedInvariant {
                                    cause:
                                        CheckedInvariantCause::EqualityOperandTargetNotAdmitted
                                        | CheckedInvariantCause::EqualityConversionShapeMismatch,
                                }),
                                ValueType::Int(target),
                            ) if narrowing => Stop::Refused(Refusal::IntegerOutOfDomain {
                                target: Box::new(target.clone()),
                            }),
                            (
                                Stop::Refused(Refusal::CheckedInvariant {
                                    cause: CheckedInvariantCause::EqualityOperandTargetNotAdmitted,
                                }),
                                ValueType::Rational(target),
                            ) if narrowing => Stop::Refused(Refusal::RationalOutOfDomain {
                                target: Box::new(target.clone()),
                            }),
                            (
                                Stop::Refused(Refusal::CheckedInvariant {
                                    cause: CheckedInvariantCause::EqualityOperandTargetNotAdmitted,
                                }),
                                ValueType::Decimal(target),
                            ) if narrowing => Stop::Refused(Refusal::DecimalOutOfDomain {
                                target: Box::new(target.clone()),
                            }),
                            _ => stop,
                        }
                    })?;
                    // FSpec FR-322-AC-384: a bounded result checks the exact
                    // converted magnitude against both inclusive endpoints.
                    match (target_type, converted) {
                        (ValueType::BoundedQuantity(bounded), Value::Quantity(quantity)) => {
                            let interval = bounded_result_interval(bounded)?;
                            let admitted = admit_quantity_result(quantity, &interval, self.meter)
                                .map_err(|_| {
                                invariant("bounded-quantity-result-unit-mismatch")
                            })?;
                            Value::Quantity(outcome_into_stop(admitted)?)
                        }
                        (ValueType::BoundedQuantity(_), _) => {
                            return Err(invariant("bounded-quantity-conversion-quantity-expected"));
                        }
                        (_, converted) => converted,
                    }
                }
            }
            NodeKind::IeeeToRational(_, domain) => {
                let Value::Float(value) = self.pop()? else {
                    return Err(invariant("ieee-conversion-float-value-expected"));
                };
                if !self.world.has_ieee_profile() {
                    return Err(invariant("ieee-profile-unresolved"));
                }
                let exact = outcome_into_stop(ieee_to_exact(value, domain, self.meter))?;
                if let Some(loss) = exact.loss() {
                    self.record(node, ValueLoss::IeeeExact(loss))?;
                }
                Value::Rational(exact.value().clone())
            }
            NodeKind::Query { .. } | NodeKind::Fold { .. } => {
                return self.start_iteration(node);
            }
            NodeKind::Seq { .. } => return self.start_seq(node),
            NodeKind::PopulationObjects { population } => {
                self.population_objects(node, population)?
            }
            NodeKind::Flatten(_) => {
                if !matches!(node.value_type(), ValueType::Collection(_)) {
                    return Err(invariant("flatten-collection-result-expected"));
                }
                let outer = self.pop_collection()?;
                collections::flatten(node.value_type(), &outer, self.meter).map_err(lift)?
            }
            NodeKind::Size(_) => {
                let operand = self.pop()?;
                if let Value::Collection(collection) = &operand {
                    collections::size(collection, self.meter)?
                } else {
                    retain_scalar(Value::Integer(size_of(operand)?), self.meter)?
                }
            }
            NodeKind::Contains(_, _) => {
                let item = self.pop()?;
                match self.pop()? {
                    Value::Collection(collection) => {
                        collections::contains(&item, &collection, self.meter)?
                    }
                    // FSpec FR-144 "Map values": `contains(m, k)` tests keys
                    // only.
                    Value::Map(map) => {
                        let found = crate::value::map_entry(&map, &item).is_some();
                        collections::retain_scalar(Value::Boolean(found), self.meter)?
                    }
                    Value::Boolean(_)
                    | Value::Integer(_)
                    | Value::Rational(_)
                    | Value::Decimal(_)
                    | Value::Float(_)
                    | Value::Quantity(_)
                    | Value::Text(_)
                    | Value::String(_)
                    | Value::Bytes(_)
                    | Value::JsonObject(_)
                    | Value::Uuid(_)
                    | Value::Timestamp(_)
                    | Value::Enum(_)
                    | Value::Population(_)
                    | Value::Option(_)
                    | Value::Composite(_)
                    | Value::Union(_)
                    | Value::Reference(_) => return Err(invariant("collection-value-expected")),
                }
            }
            NodeKind::Navigate {
                relationship,
                near,
                unique,
                ..
            } => self.navigate(node, relationship, *near, *unique)?,
            NodeKind::NamedPopulation { .. } => {
                return Err(invariant("named-population-is-read-by-its-parent"));
            }
            NodeKind::AllInstances { population }
                if let NodeKind::NamedPopulation {
                    population,
                    members,
                } = node.at(*population).kind() =>
            {
                self.named_all_instances(node, population, members)?
            }
            NodeKind::Lookup {
                population,
                absence,
                ..
            } if let NodeKind::NamedPopulation {
                population,
                members,
            } = node.at(*population).kind() =>
            {
                let (reference, _) = self.pop_forwarded()?;
                self.named_lookup(node, population, members, *absence, reference)?
            }
            NodeKind::AllInstances { population } => {
                let Value::Population(population_id) = self.pop()? else {
                    return Err(invariant("query-population-value-expected"));
                };
                let ValueType::Population(maximum) = node.at(*population).value_type() else {
                    return Err(invariant("query-population-type-expected"));
                };
                let ValueType::Collection(collection_type) = node.value_type() else {
                    return Err(invariant("all-instances-collection-result-expected"));
                };
                let binding = self.resolve_population(population_id, *maximum)?;
                let binding = self.select_anchor(binding)?;
                let objects = self.objects_for(node)?;
                Self::require_complete_population(objects, binding.population_key())?;
                self.log_membership_read(binding, None);
                evaluate_all_instances(binding, collection_type, self.meter)?
            }
            NodeKind::Lookup {
                reference,
                absence,
                population,
            } => {
                let (reference_value, _) = self.pop_forwarded()?;
                let Value::Population(population_id) = self.pop()? else {
                    return Err(invariant("query-population-value-expected"));
                };
                let ValueType::Population(maximum) = node.at(*population).value_type() else {
                    return Err(invariant("query-population-type-expected"));
                };
                let binding = self.resolve_population(population_id, *maximum)?;
                let ValueType::Reference(static_key) = node.at(*reference).value_type() else {
                    return Err(invariant("lookup-reference-type-expected"));
                };
                // Not the FR-063 seam: both matches below are over
                // `ValueType` (checking already fixed this node's own type
                // to one of two shapes), not `NodeKind`, so each fallback is
                // a genuine invariant check, not an escape hatch over the
                // seam's own enum.
                #[allow(clippy::wildcard_enum_match_arm)]
                let target_key = match node.value_type() {
                    ValueType::Reference(key) => key.clone(),
                    ValueType::Option(payload) => match &**payload {
                        ValueType::Reference(key) => key.clone(),
                        _ => return Err(invariant("lookup-reference-result-expected")),
                    },
                    _ => return Err(invariant("lookup-reference-result-expected")),
                };
                let binding = self.select_anchor(binding)?;
                let looked_up = if let Value::Reference(looked_up) = &reference_value {
                    Some(looked_up.clone())
                } else {
                    None
                };
                self.log_membership_read(binding, looked_up);
                evaluate_lookup(
                    binding,
                    target_key,
                    static_key.clone(),
                    reference_value,
                    *absence,
                    node.value_type(),
                    self.meter,
                )?
            }
            NodeKind::Dispatch {
                table,
                operation,
                arguments,
                ..
            } => {
                let (arguments, argument_owners) = self.pop_many_owned(arguments.len())?;
                let (Value::Reference(reference), receiver_owner) = self.pop_owned()? else {
                    return Err(invariant("dispatch-reference-value-expected"));
                };
                let mut full_owners = Vec::with_capacity(argument_owners.len() + 1);
                full_owners.push(receiver_owner);
                full_owners.extend(argument_owners);
                let subtype = reference.object_type();
                // The exact call site that produced this node (`check.rs`'s
                // `dispatch_call`), never re-derived by searching
                // `dispatch_operations` for a `table` match: two call sites
                // can share a table, and a `table`-only search would report
                // whichever operation happens to come first, not the one
                // this node's own `receiver.member(args)` actually named.
                let operation = self
                    .world
                    .dispatch_operations()
                    .get(*operation)
                    .ok_or_else(|| invariant("dispatch-operation-unresolved"))?;
                let table_ref = self
                    .dispatch_tables
                    .get(*table)
                    .ok_or_else(|| invariant("dispatch-table-unresolved"))?;
                charge_dispatch_select(self.meter, table_ref.candidate_count())?;
                let candidate = table_ref
                    .linked_for(subtype)
                    .ok_or_else(|| invariant("dispatch-candidate-unresolved"))?
                    .clone();
                let mut full_arguments = Vec::with_capacity(arguments.len() + 1);
                full_arguments.push(Value::Reference(reference.clone()));
                full_arguments.extend(arguments);
                match candidate.precondition {
                    Some(precondition_function) => {
                        let callable = node
                            .callee(self.world.graph(), precondition_function)
                            .ok_or_else(|| invariant("evaluation-callable-unresolved"))?;
                        let selected = node
                            .callee(self.world.graph(), candidate.body)
                            .ok_or_else(|| invariant("evaluation-callable-unresolved"))?
                            .name
                            .to_owned();
                        let next_charge = frame_occurrences(&full_arguments);
                        let mut frame: Vec<Option<Value>> =
                            full_arguments.clone().into_iter().map(Some).collect();
                        frame.resize(callable.slots.max(frame.len()), None);
                        let provenance = vec![false; frame.len()];
                        self.push_frame_owned(
                            frame,
                            provenance,
                            full_owners.clone(),
                            next_charge,
                            true,
                        )?;
                        self.tasks.push(Task::DispatchGuard(Box::new(DispatchGuard {
                            body_function: candidate.body,
                            arguments: full_arguments,
                            failure: PreconditionFailure::new(
                                operation.member.clone(),
                                selected,
                                reference,
                            ),
                            node,
                            owners: full_owners,
                        })));
                        self.tasks.push(Task::Eval(callable.body));
                        return Ok(());
                    }
                    None => {
                        let callable = node
                            .callee(self.world.graph(), candidate.body)
                            .ok_or_else(|| invariant("evaluation-callable-unresolved"))?;
                        charge_call(self.meter)?;
                        let next_charge = frame_occurrences(&full_arguments);
                        let mut frame: Vec<Option<Value>> =
                            full_arguments.into_iter().map(Some).collect();
                        frame.resize(callable.slots.max(frame.len()), None);
                        let provenance = vec![false; frame.len()];
                        self.push_frame_owned(frame, provenance, full_owners, next_charge, true)?;
                        self.tasks.push(Task::ObserveApplication(node));
                        self.tasks.push(Task::Return);
                        self.tasks.push(Task::Eval(callable.body));
                        return Ok(());
                    }
                }
            }
        };
        self.push_value(value, false);
        Ok(())
    }

    /// FR-107: the `ObjectEnvironment` a model read at `node` observes --
    /// `pre_objects` when `reads` maps `node`'s location to
    /// `Observation::Pre` and a pre environment is carried, `objects`
    /// (the clause's own `current`/`post` observation) otherwise. Always
    /// `objects` for the `Value` family, which carries neither `reads` nor
    /// `pre_objects`.
    ///
    /// **A `pre` read reached with no pre observation refuses, never falls
    /// back silently** (`FR-107-evaluate-state-clauses-at-s6a.md:102-105`:
    /// "If a `pre(e)` is reached in a postcondition whose observations
    /// carry no pre observation, then the evaluator SHALL return
    /// `FamilyResult::Refused` with `wrong_snapshot`/`wrong-anchor`").
    /// Admission makes this unreachable for a well-formed selection; the
    /// arm exists so the case has a typed result, the same rationale
    /// `select_anchor` above already documents for the population-read
    /// case (SR-750 FND-013).
    fn objects_for(&self, node: N) -> Result<&'a ObjectEnvironment, Halt> {
        if let Some(components) = &self.components {
            if let Some((location, component)) = node.location().and_then(|location| {
                components
                    .reads
                    .get(location)
                    .map(|component| (location, component))
            }) {
                if components
                    .observations
                    .and_then(|observations| observations.get(location))
                    == Some(&fsl_semantics::model::clause_observation::Observation::Pre)
                {
                    if let Some(pre_environments) = components.pre_environments {
                        return pre_environments
                            .get(*component)
                            .cloned()
                            .ok_or_else(|| invariant("component-pre-environment-resolved"));
                    }
                }
                return components
                    .environments
                    .get(*component)
                    .cloned()
                    .ok_or_else(|| invariant("component-environment-resolved"));
            }
        }
        // FR-120 "Eligible" (W50): a read inside `pre(...)` observes the
        // pre-state, also a read in the body or precondition of a function or
        // dispatched query that `pre(...)` calls, whose locations the clause's
        // `reads` does not list.
        if let (Some(_), Anchor::Pre, Some(pre)) = (self.reads, self.anchor, self.pre_objects) {
            return Ok(pre);
        }
        match self.reads {
            Some(reads)
                if node.location().and_then(|location| reads.get(location))
                    == Some(&fsl_semantics::model::clause_observation::Observation::Pre) =>
            {
                self.pre_objects.ok_or_else(|| {
                    Halt::Family(FamilyResult::Refused(Box::new(
                        ProtocolClauseSnapshot::WrongAnchor {
                            required: "pre",
                            supplied: "post",
                        },
                    )))
                })
            }
            _ => Ok(self.objects),
        }
    }

    /// Records the read of `reference`'s `field` in `objects`' read log,
    /// when it keeps one, under the instant the attribute node observes.
    fn log_field_read(
        &self,
        node: N,
        objects: &ObjectEnvironment,
        reference: &ObjectReference,
        field: &FieldRef,
    ) -> Result<(), Halt> {
        let Some(log) = objects.read_log() else {
            return Ok(());
        };
        let attribute = self
            .world
            .types()
            .attributes(reference.object_type().clone())
            .and_then(|attributes| {
                attributes
                    .iter()
                    .find(|attribute| attribute.stands_for(field))
            })
            .ok_or_else(|| invariant("logged-attribute-unresolved"))?;
        let observation = self
            .reads
            .and_then(|reads| node.location().and_then(|location| reads.get(location)))
            .copied()
            .unwrap_or(fsl_semantics::model::clause_observation::Observation::Current);
        log.record(ObservedRead::Field {
            observation,
            object: reference.clone(),
            owner: attribute.owner(),
            name: attribute.field().name().to_owned(),
        });
        Ok(())
    }

    /// Records the read of `binding`'s population, at the anchor the
    /// evaluation selected, in the read log of the evaluation's objects.
    fn log_membership_read(&self, binding: &PopulationBinding, object: Option<ObjectReference>) {
        self.log_membership_of(binding.population_key(), object);
    }

    /// Records the read of the population `key`'s membership, at the anchor
    /// the evaluation selected.
    fn log_membership_of(&self, key: &DeclarationKey, object: Option<ObjectReference>) {
        let Some(log) = self.objects.read_log() else {
            return;
        };
        log.record(ObservedRead::Membership {
            observation: match self.anchor {
                Anchor::Pre => fsl_semantics::model::clause_observation::Observation::Pre,
                Anchor::Post => fsl_semantics::model::clause_observation::Observation::Post,
            },
            population: key.clone(),
            object,
        });
    }

    /// Records a created-object reference literal's lookup in the population
    /// selected during subject binding. The graph stays subject-independent;
    /// footprint derivation admits all populations related to its type.
    fn log_reference_membership_read(
        &self,
        node: N,
        objects: &ObjectEnvironment,
        populations: &[DeclarationKey],
    ) {
        let Some(log) = objects.read_log() else {
            return;
        };
        let observation = self
            .reads
            .and_then(|reads| node.location().and_then(|location| reads.get(location)))
            .copied()
            .unwrap_or(match self.anchor {
                Anchor::Pre => fsl_semantics::model::clause_observation::Observation::Pre,
                Anchor::Post => fsl_semantics::model::clause_observation::Observation::Post,
            });
        for population in populations {
            log.record(ObservedRead::Membership {
                observation,
                population: population.clone(),
                object: None,
            });
        }
    }

    /// FR-139: the objects of the concrete population `population` in the
    /// observed state, in ascending key order, as the set a refinement row
    /// reads.
    fn population_objects(&mut self, node: N, population: &DeclarationKey) -> Result<Value, Halt> {
        let objects = self.objects_for(node)?;
        let members = objects
            .population_members(&population.node)
            .ok_or_else(|| invariant("population-members-unobserved"))?;
        let ValueType::Collection(result_type) = node.value_type() else {
            return Err(invariant("population-objects-collection-result-expected"));
        };
        self.log_membership_of(population, None);
        let elements = members.iter().cloned().map(Value::Reference).collect();
        Ok(outcome_into_stop(form(result_type, elements, self.meter))?)
    }

    /// FR-391: `allInstances<T>(M::P)`, and a named population read as a
    /// domain, over the population the snapshot of the node's observation
    /// lists: every member whose type conforms to `T`, one `population.visit`
    /// charge per member, FR-084's typed incomplete result when no declared
    /// member type covers `T`.
    fn named_all_instances(
        &mut self,
        node: N,
        population: &DeclarationKey,
        members: &[filament_exact::EffectiveId],
    ) -> Result<Value, Halt> {
        let ValueType::Collection(result_type) = node.value_type() else {
            return Err(invariant("all-instances-collection-result-expected"));
        };
        let ValueType::Reference(target) = result_type.element() else {
            return Err(invariant("all-instances-reference-element-expected"));
        };
        self.named_covers(members, target.clone())?;
        let types = self.world.types();
        let listed = self.named_members(node, population)?;
        let mut selected = Vec::new();
        for reference in listed {
            charge_named(self.meter, ChargePoint::PopulationVisit)?;
            if types.conforms(reference.object_type().clone(), target.clone()) {
                selected.push(Value::Reference(reference.clone()));
            }
        }
        let count = u64::try_from(selected.len()).unwrap_or(u64::MAX);
        self.meter.charge(
            Charge::new(ChargePoint::CollectionBound).size(LimitKind::ValueOccurrences, count),
        )?;
        self.log_membership_of(population, None);
        Ok(outcome_into_stop(form(result_type, selected, self.meter))?)
    }

    /// FR-084's coverage check for a named population (FR-391 "Typing"):
    /// FR-084's typed incomplete result when no declared member type of the
    /// population covers `target`. It charges nothing.
    fn named_covers(
        &self,
        members: &[filament_exact::EffectiveId],
        target: filament_exact::EffectiveId,
    ) -> Result<(), Halt> {
        let types = self.world.types();
        if members
            .iter()
            .any(|member| types.conforms(target.clone(), member.clone()))
        {
            return Ok(());
        }
        let queried = self
            .world
            .object_type_node(target)
            .ok_or_else(|| invariant("named-queried-type-declared"))?;
        Err(ModelQueryHalt::IncompleteClosure {
            queried_type: queried.clone(),
            uncovered_subtypes: vec![queried],
        }
        .into())
    }

    /// The objects the snapshot of `node`'s observation lists for
    /// `population` (FR-391 "Evaluation"): a population that snapshot does
    /// not list `complete` stops `incomplete_population`, with
    /// `missing-required-artifact` for no row and `incomplete-scope` for a
    /// row not `complete`, whichever path admitted it.
    fn named_members(
        &self,
        node: N,
        population: &DeclarationKey,
    ) -> Result<&'a [ObjectReference], Halt> {
        let objects = self.objects_for(node)?;
        Self::require_complete_population(objects, population)?;
        objects
            .population_members(&population.node)
            .ok_or_else(|| invariant("named-population-unobserved"))
    }

    /// FR-391: `lookup<T>(M::P, r) absent m` over the population the snapshot
    /// of the node's observation lists, with FR-084's coverage check of `T`
    /// and `lookup`'s conformance of the reference to `T`.
    fn named_lookup(
        &mut self,
        node: N,
        population: &DeclarationKey,
        members: &[filament_exact::EffectiveId],
        absence: filament_semantic_value::absence::AbsenceMode,
        reference: Value,
    ) -> Result<Value, Halt> {
        use filament_semantic_value::absence::AbsenceMode;
        let Value::Reference(reference) = reference else {
            return Err(invariant("lookup-reference-value-expected"));
        };
        let target = match node.value_type() {
            ValueType::Option(payload) => &**payload,
            other => other,
        };
        let ValueType::Reference(target) = target else {
            return Err(invariant("lookup-reference-result-expected"));
        };
        self.named_covers(members, target.clone())?;
        if !self
            .world
            .types()
            .conforms(reference.object_type().clone(), target.clone())
        {
            return Err(Halt::Family(FamilyResult::Refused(Box::new(
                ModelQueryRefusal::from(
                    fsl_semantics::model::normalize::ModelRefusal::type_mismatch(
                        self.world
                            .object_type_node(reference.object_type().clone())
                            .ok_or_else(|| invariant("named-reference-type-declared"))?,
                        self.world
                            .object_type_node(target.clone())
                            .ok_or_else(|| invariant("named-queried-type-declared"))?,
                    ),
                ),
            ))));
        }
        self.meter
            .charge(Charge::new(ChargePoint::LookupKey).size(LimitKind::ValueOccurrences, 1))?;
        let present = self.named_members(node, population)?.contains(&reference);
        self.log_membership_of(population, Some(reference.clone()));
        let retain = |meter: &mut Meter, occurrences: u64| -> Result<(), Halt> {
            Ok(meter.charge(
                Charge::new(ChargePoint::LookupResultRetain)
                    .size(LimitKind::ValueOccurrences, occurrences)
                    .results(occurrences),
            )?)
        };
        let payload = || match node.value_type() {
            ValueType::Option(payload) => Ok((**payload).clone()),
            _ => Err(invariant("lookup-option-result-expected")),
        };
        let key = render_identity(reference.object().as_str().as_bytes());
        match (present, absence) {
            (true, AbsenceMode::Empty) => {
                retain(self.meter, 2)?;
                Ok(OptionValue::from_admitted(
                    payload()?,
                    Some(Value::Reference(reference)),
                ))
            }
            (true, AbsenceMode::Undefined | AbsenceMode::Refused) => {
                retain(self.meter, 1)?;
                Ok(Value::Reference(reference))
            }
            (false, AbsenceMode::Empty) => {
                retain(self.meter, 1)?;
                Ok(OptionValue::from_admitted(payload()?, None))
            }
            (false, AbsenceMode::Undefined) => Err(Halt::Family(FamilyResult::Undefined(
                Box::new(StateModelUndefined::AbsentKey {
                    binding: population.node.clone(),
                    key,
                }),
            ))),
            (false, AbsenceMode::Refused) => Err(Halt::Family(FamilyResult::Refused(Box::new(
                ModelQueryRefusal::from(fsl_semantics::model::normalize::ModelRefusal::absent_key(
                    population.node.clone(),
                    reference.object().as_str().as_bytes().to_vec(),
                )),
            )))),
        }
    }

    /// FR-139: start `seq(i in low .. high: e)` over its evaluated bounds.
    fn start_seq(&mut self, node: N) -> Result<(), Halt> {
        let (Value::Integer(high), Value::Integer(low)) = (self.pop()?, self.pop()?) else {
            return Err(invariant("seq-integer-bounds-expected"));
        };
        self.iterations = self.iterations.saturating_add(1);
        self.iterate_seq(Box::new(SeqIteration {
            node,
            state: Sequence::new(low, high),
            source_owner: self.step_owner(),
            owner: self.step_owner(),
        }))
    }

    /// FR-139: bind the next integer and evaluate the element, charging one
    /// work unit per element; the sequence is empty when `high < low`.
    fn iterate_seq(&mut self, mut iteration: Box<SeqIteration<N>>) -> Result<(), Halt> {
        let node = iteration.node;
        let NodeKind::Seq { slot, body, .. } = node.kind() else {
            return Err(invariant("seq-node-expected"));
        };
        self.absorb_owner(iteration.owner);
        let result = if iteration.state.awaiting {
            Some(self.pop()?)
        } else {
            None
        };
        let Some(bound) = iteration.state.step(result, self.meter)? else {
            self.iterations = self.iterations.saturating_sub(1);
            let value = iteration
                .state
                .finish(node.value_type(), self.meter)
                .map_err(lift)?;
            self.push_value(value, false);
            return Ok(());
        };
        *self.slot(*slot)? = Some(Value::Integer(bound));
        iteration.owner = self.step_owner();
        self.set_slot_owner(*slot, iteration.source_owner)?;
        self.tasks.push(Task::IterateSeq(iteration));
        self.tasks.push(Task::Eval(node.at(*body)));
        Ok(())
    }

    /// FR-364 "Runtime navigation reads only the extent of its relationship":
    /// `r.role` over the relationship end nearest `r` (`near`), stopping at
    /// the first outcome. Charges `model.navigate`; an extent that is not
    /// complete is `incomplete_population`/`incomplete-scope` with no further
    /// charge; each selected reference, in reference-key order, is decided
    /// by `resolve` (present, dangling or undecidable, over the observation's
    /// own manifest and closure) before its `collection.visit`, so a missing
    /// target decides before the count bound; then the multiplicity-
    /// determined value is bounded and retained.
    fn navigate(
        &mut self,
        node: N,
        relationship: &DeclarationKey,
        near: EndSide,
        unique: bool,
    ) -> Result<Value, Halt> {
        let Value::Reference(receiver) = self.pop()? else {
            return Err(invariant("navigate-reference-value-expected"));
        };
        charge_model_read(self.meter, ChargePoint::ModelNavigate)?;
        let objects = self.objects_for(node)?;
        let refuse =
            |cause: NavigationRefusal| Halt::Family(FamilyResult::Refused(Box::new(cause)));
        let Some(extents) = objects
            .extents()
            .filter(|extents| extents.complete(relationship))
        else {
            return Err(refuse(NavigationRefusal::ExtentIncomplete {
                relationship: relationship.clone(),
                receiver,
            }));
        };
        let selected: Vec<&ObjectReference> = match near {
            EndSide::Source => extents.targets(relationship, &receiver).collect(),
            EndSide::Target => extents.sources(relationship, &receiver).collect(),
        };
        let types = self.world.types();
        let manifest = extents
            .manifest(types)
            .map_err(|_| invariant("navigate-manifest-unavailable"))?;
        let multi_valued = matches!(node.value_type(), ValueType::Collection(_));
        let mut elements = Vec::with_capacity(selected.len());
        for target in selected {
            match objects.objects().resolve(target, types, &manifest) {
                Resolution::Present => {}
                Resolution::Dangling => {
                    return Err(refuse(NavigationRefusal::TargetDangling {
                        relationship: relationship.clone(),
                        reference: target.clone(),
                    }));
                }
                Resolution::Undecidable => {
                    return Err(refuse(NavigationRefusal::TargetUndecidable {
                        relationship: relationship.clone(),
                        reference: target.clone(),
                    }));
                }
            }
            if multi_valued {
                charge_visit(self.meter)?;
            }
            elements.push(Value::Reference(target.clone()));
        }
        match node.value_type() {
            ValueType::Collection(collection_type) => Ok(outcome_into_stop(form_grouped(
                collection_type,
                elements,
                self.meter,
            ))?),
            ValueType::Reference(_) => {
                let reference = single_target(elements, 1, 1, unique)?;
                reference.ok_or_else(|| invariant("navigate-required-target-absent"))
            }
            ValueType::Option(payload) => {
                let target = single_target(elements, 0, 1, unique)?;
                Ok(OptionValue::from_admitted((**payload).clone(), target))
            }
            // The checker types a navigation to exactly the three shapes
            // above (`state_model::navigation`); any other type is an
            // invariant break, not a closed enum's missing arm.
            #[allow(clippy::wildcard_enum_match_arm)]
            _ => Err(invariant("navigate-result-type-unexpected")),
        }
    }

    /// Whether this `Machine` is running a `ProtocolClause` (FR-107):
    /// `self.family` is set once, explicitly, by whichever constructor
    /// built it (SR-750 FND-011 round 2), never inferred per-call from
    /// `self.reads.is_some()`. Consulted to apply protocol-specific model
    /// read accounting.
    fn is_protocol_clause(&self) -> bool {
        self.family == EvaluationFamily::Protocol
    }

    /// FR-107: `reaches(source, target, edge)`, evaluated as
    /// `value-accounting.md`'s "Model and graph evaluation" paragraph
    /// states: charge `graph.expand` for `source` and enqueue it; for each
    /// dequeued node and each target `t` of `edge`, in the edge's own
    /// order, charge one `graph.edge`, return `true` when `t` is `target`,
    /// and otherwise charge `graph.expand` for an undiscovered `t` and
    /// enqueue it; an empty queue gives `false`; then charge
    /// `graph.result-retain`.
    fn evaluate_reaches(
        &mut self,
        node: N,
        source: &ObjectReference,
        target: &ObjectReference,
        edge: &FieldRef,
    ) -> Result<bool, Halt> {
        charge_named(self.meter, ChargePoint::GraphExpand)?;
        let mut discovered: BTreeSet<ObjectReference> = BTreeSet::new();
        discovered.insert(source.clone());
        let mut queue: VecDeque<ObjectReference> = VecDeque::new();
        queue.push_back(source.clone());
        let mut found = false;
        'walk: while let Some(current) = queue.pop_front() {
            for next in self.edge_targets(node, &current, edge)? {
                charge_named(self.meter, ChargePoint::GraphEdge)?;
                if next == *target {
                    found = true;
                    break 'walk;
                }
                if discovered.insert(next.clone()) {
                    charge_named(self.meter, ChargePoint::GraphExpand)?;
                    queue.push_back(next);
                }
            }
        }
        charge_named(self.meter, ChargePoint::GraphResultRetain)?;
        Ok(found)
    }

    /// FR-365 "Evaluation": `reaches(source, target, name)` over the extent of
    /// `relationship` in the clause's observation, reading only `complete`,
    /// `targets` (a forward role) or `sources` (an inverse role) and
    /// `resolve`, stopping at the first outcome. `source` is decided by
    /// `resolve` with no charge; `graph.expand` is charged for it; an extent
    /// that is not complete is `incomplete_population`/`incomplete-scope`; each
    /// dequeued node's neighbours, in the order the read returns them, are
    /// decided by `resolve` before their `graph.edge`; the target is compared
    /// by whole-key identity and need not be held; `graph.result-retain` is
    /// charged last. No neighbour count is compared with a multiplicity.
    fn evaluate_reaches_end(
        &mut self,
        node: N,
        source: &ObjectReference,
        target: &ObjectReference,
        relationship: &DeclarationKey,
        near: EndSide,
        population: Option<&AuthoredBound>,
    ) -> Result<bool, Halt> {
        let objects = self.objects_for(node)?;
        // FR-380: a population with an authored bound holds at most that
        // many objects; more refuses `cardinality_out_of_bound` before any
        // charge, and is no resource outcome (ADR-014 B-1).
        if let Some(bound) = population {
            check_authored_bound(objects, bound)?;
        }
        let refuse =
            |cause: NavigationRefusal| Halt::Family(FamilyResult::Refused(Box::new(cause)));
        let extents = objects.extents();
        let types = self.world.types();
        let manifest = extents
            .map(|extents| extents.manifest(types))
            .transpose()
            .map_err(|_| invariant("reaches-manifest-unavailable"))?;
        // A neighbour or the start decided as `navigate` decides a target.
        let decide = |reference: &ObjectReference| match &manifest {
            Some(manifest) => match objects.objects().resolve(reference, types, manifest) {
                Resolution::Present => Ok(()),
                Resolution::Dangling => Err(refuse(NavigationRefusal::TargetDangling {
                    relationship: relationship.clone(),
                    reference: reference.clone(),
                })),
                Resolution::Undecidable => Err(refuse(NavigationRefusal::TargetUndecidable {
                    relationship: relationship.clone(),
                    reference: reference.clone(),
                })),
            },
            None => Ok(()),
        };
        decide(source)?;
        charge_graph_expand(self.meter, 1)?;
        let Some(extents) = extents.filter(|extents| extents.complete(relationship)) else {
            return Err(refuse(NavigationRefusal::ExtentIncomplete {
                relationship: relationship.clone(),
                receiver: source.clone(),
            }));
        };
        let mut discovered: BTreeSet<ObjectReference> = BTreeSet::new();
        discovered.insert(source.clone());
        let mut queue: VecDeque<ObjectReference> = VecDeque::new();
        queue.push_back(source.clone());
        let mut found = false;
        'walk: while let Some(current) = queue.pop_front() {
            let neighbours: Vec<&ObjectReference> = match near {
                EndSide::Source => extents.targets(relationship, &current).collect(),
                EndSide::Target => extents.sources(relationship, &current).collect(),
            };
            for next in neighbours {
                decide(next)?;
                charge_named(self.meter, ChargePoint::GraphEdge)?;
                if next == target {
                    found = true;
                    break 'walk;
                }
                if discovered.insert(next.clone()) {
                    charge_graph_expand(self.meter, discovered_count(&discovered))?;
                    queue.push_back(next.clone());
                }
            }
        }
        charge_graph_result_retain(self.meter)?;
        Ok(found)
    }

    /// The targets `edge` names on `current`, in the edge's own order: one
    /// for a required `Reference<T>` field, zero or one for an optional
    /// one, and each element in sequence order for a required sequence of
    /// `Reference<T>` (FR-104's own restriction on `edge`'s declared kind,
    /// `check.rs`'s `reaches` eligibility check).
    ///
    /// A field's presence (`Presence::Required`/`Presence::Optional`) is a
    /// declaration-level property distinct from its `value_type()`
    /// (`FieldDeclaration` keeps them as two fields, not one wrapping the
    /// other, `model::intake::read_field_member`): an optional
    /// `Reference<T>` field's `value_type()` is the bare
    /// `ValueType::Reference(_)`, never `ValueType::Option(_)` -- that
    /// variant names a genuinely `Option`-typed *value* (e.g. a checked
    /// `NodeKind::Attribute` read's own synthesized static type), not model
    /// field presence, and `reaches`'s own S3 eligibility check never
    /// admits one. This reads `presence()` to decide whether an absent slot
    /// is a valid zero-target read or a broken invariant.
    fn edge_targets(
        &self,
        node: N,
        current: &ObjectReference,
        edge: &FieldRef,
    ) -> Result<Vec<ObjectReference>, Halt> {
        let declared = self
            .world
            .types()
            .attributes(current.object_type().clone())
            .ok_or_else(|| invariant("edge-attribute-declarations-unresolved"))?;
        let attribute = declared
            .iter()
            .find(|attribute| attribute.stands_for(edge))
            .ok_or_else(|| invariant("edge-attribute-unresolved"))?;
        let field = attribute.field();
        let objects = self.objects_for(node)?;
        if !objects.objects().contains(current) {
            return Err(invariant("edge-slot-unresolved"));
        }
        let (slot, _) = self.admitted_slot(objects, current, edge)?;
        Self::slot_edge_targets(field, slot)
    }

    /// The references an admitted `slot` of the edge field `field` holds.
    fn slot_edge_targets(
        field: &FieldDeclaration,
        slot: &FieldValue,
    ) -> Result<Vec<ObjectReference>, Halt> {
        match field.value_type() {
            ValueType::Reference(_) => match (slot, field.presence()) {
                (FieldValue::Present(Value::Reference(reference)), _) => {
                    Ok(vec![reference.clone()])
                }
                (FieldValue::Absent | FieldValue::Null, filament_exact::Presence::Optional) => {
                    Ok(Vec::new())
                }
                _ => Err(invariant("edge-reference-slot-invalid")),
            },
            ValueType::Collection(collection) if collection.kind() == CollectionKind::Sequence => {
                match slot {
                    FieldValue::Present(Value::Collection(collection)) => collection
                        .elements()
                        .iter()
                        .map(|element| match element {
                            Value::Reference(reference) => Ok(reference.clone()),
                            _ => Err(invariant("edge-reference-element-expected")),
                        })
                        .collect(),
                    _ => Err(invariant("edge-sequence-slot-invalid")),
                }
            }
            _ => Err(invariant("edge-field-type-invalid")),
        }
    }

    /// The caller-supplied slot of `reference` for `field`, admitted against
    /// the evaluating package's own types (FR-049-AC-2/AC-3), and whether it
    /// holds a witnessed out-of-range integer.
    ///
    /// The closure names each slot by its attribute name, so the slot is the
    /// one the package's attribute for `field` is named, whatever order the
    /// caller's environment gave the attributes. A call or expression entry
    /// has already admitted the whole environment
    /// (`slot_admission::admit_objects`); this read-time check is the
    /// admission for the protocol, temporal and clause entries, which reach
    /// the evaluator without it. A missing or ill-typed slot is caller input
    /// that refuses `invalid_runtime_input`/`wrong-value-kind`, never a fault
    /// or a misread.
    fn admitted_slot<'o>(
        &self,
        objects: &'o ObjectEnvironment,
        reference: &ObjectReference,
        field: &FieldRef,
    ) -> Result<(&'o FieldValue, bool), Halt> {
        let types = self.world.types();
        let refuse = || {
            Halt::Family(FamilyResult::Refused(Box::new(ObjectFieldIllTyped {
                field: field.name.clone(),
            })))
        };
        // An admitted reference whose object is absent has no value to read;
        // the population correspondence distinguishes absence from a bad layout.
        if !objects.objects().contains(reference) {
            let binding = objects
                .population_completeness()
                .keys()
                .find(|identity| {
                    objects
                        .population_node(identity)
                        .and_then(|node| objects.population_provenance(node))
                        .is_some_and(|population| {
                            population.member_types().iter().any(|(member, universe)| {
                                member == reference.object_type()
                                    && universe == reference.universe()
                            })
                        })
                        || objects
                            .population_binding(identity)
                            .is_some_and(|binding| binding.universe() == reference.universe())
                })
                .ok_or_else(|| invariant("missing-object-has-admitted-population"))?;
            return Err(Halt::Family(FamilyResult::Undefined(Box::new(
                StateModelUndefined::AbsentKey {
                    binding: binding.clone(),
                    key: render_identity(reference.object().as_str().as_bytes()),
                },
            ))));
        }
        let declared = types
            .attributes(reference.object_type().clone())
            .ok_or_else(refuse)?;
        let attribute = declared
            .iter()
            .find(|attribute| attribute.stands_for(field))
            .ok_or_else(|| invariant("attribute-slot-unresolved"))?;
        let slot = objects
            .objects()
            .attribute(types, reference, field)
            .ok_or_else(refuse)?;
        let witnesses: Vec<&OutOfRange> = objects.witnesses(types, reference, field).collect();
        match slot_admission::admits_slot(
            types,
            attribute.field(),
            slot,
            &witnesses,
            objects.holds_out_of_range(),
        ) {
            Ok(true) => Ok((slot, !witnesses.is_empty())),
            Ok(false) => Err(refuse()),
            Err(failure) => Err(invariant(slot_admission::fault_identifier(&failure))),
        }
    }

    /// [`composite::project`] as the evaluator's halt.
    fn project(
        types: &TypeEnvironment,
        slot: &FieldValue,
        optional: bool,
        value_type: &ValueType,
        witnessed: bool,
    ) -> Result<Value, Halt> {
        composite::project(types, slot, optional, value_type, witnessed).map_err(invariant)
    }

    /// Whether `operand` reads a slot FR-106 check 6.5 witnessed out of
    /// range: an `Attribute` node whose latest read was witnessed, seen
    /// through `value(..)`.
    fn reads_witnessed(&self, operand: N) -> bool {
        match operand.kind() {
            NodeKind::Attribute { .. } => operand
                .location()
                .is_some_and(|location| self.witnessed_reads.contains(location)),
            NodeKind::Value(inner) => self.reads_witnessed(operand.at(*inner)),
            _ => false,
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
                let (Some(l), Some(r)) = (
                    self.enum_members.resolve(l.variant().clone()),
                    self.enum_members.resolve(r.variant().clone()),
                ) else {
                    return Err(invariant("ordered-enumeration-variant-unresolved"));
                };
                return outcome_into_stop(
                    compare_enum(comparison(operator), l, r, self.meter)
                        .map_err(|_| invariant("ordered-enumeration-comparison-rejected"))?,
                )
                .map_err(Into::into);
            }
            (OrderedKind::Texts, Value::Text(l), Value::Text(r)) => {
                return outcome_into_stop(
                    compare_text(comparison(operator), l, r, self.meter)
                        .map_err(|_| invariant("ordered-text-comparison-rejected"))?,
                )
                .map_err(Into::into);
            }
            (OrderedKind::Quantities, Value::Quantity(l), Value::Quantity(r)) => {
                let (Some(l), Some(r)) = (self.units.resolve(l), self.units.resolve(r)) else {
                    return Err(unresolved_unit());
                };
                return outcome_into_stop(
                    compare_quantity(comparison(operator), l, r, self.meter)
                        .map_err(|_| invariant("ordered-quantity-comparison-rejected"))?,
                )
                .map_err(Into::into);
            }
            _ => return Err(invariant("ordered-operand-kind-invalid")),
        };
        outcome_into_stop(order_numbers(operator, operands, self.meter)).map_err(Into::into)
    }

    /// What the shared iteration needs of the query or fold at `node`.
    fn iteration_spec(node: N) -> Result<Spec<'a>, Halt> {
        let shape = match node.kind() {
            NodeKind::Query { visit, .. } => Shape::Query(*visit),
            NodeKind::Fold { .. } => Shape::Fold,
            _ => return Err(invariant("iteration-node-invalid")),
        };
        let source_is_filter = matches!(
            node.kind(),
            NodeKind::Query { source, .. }
                if matches!(node.at(*source).kind(), NodeKind::Query { visit: Visit::Filter, .. })
        );
        Ok(Spec::new(shape, node.value_type(), source_is_filter))
    }

    fn start_iteration(&mut self, node: N) -> Result<(), Halt> {
        self.iterations = self.iterations.saturating_add(1);
        let (identity, reduce) = match node.kind() {
            NodeKind::Fold { identity, .. } => (
                match identity {
                    Some(_) => Some(self.pop()?),
                    None => None,
                },
                identity.is_none(),
            ),
            NodeKind::Query { .. } => (None, false),
            _ => return Err(invariant("iteration-node-invalid")),
        };
        let source = self.pop_collection()?;
        let spec = Self::iteration_spec(node)?;
        let state = match IterationState::start(&spec, source, identity, reduce, self.meter) {
            Ok(state) => state,
            Err(collections::Halt::OnlyNotOne { found }) => {
                self.iterations = self.iterations.saturating_sub(1);
                let NodeKind::Query {
                    source: operand, ..
                } = node.kind()
                else {
                    return Err(invariant("iteration-node-invalid"));
                };
                return Err(Halt::Family(FamilyResult::Undefined(Box::new(
                    StateModelUndefined::OnlyNotOne {
                        population: population_of(node.at(*operand))?,
                        found,
                    },
                ))));
            }
            Err(other) => return Err(lift(other)),
        };
        self.iterate(Box::new(Iteration {
            node,
            state,
            source_owner: self.step_owner(),
            owner: self.step_owner(),
        }))
    }

    fn iterate(&mut self, mut iteration: Box<Iteration<N>>) -> Result<(), Halt> {
        let node = iteration.node;
        let spec = match Self::iteration_spec(node) {
            Ok(spec) => spec,
            Err(fault) => {
                // A node that is neither query nor fold is detected where the
                // step would first touch it: after the pending result is taken
                // or after the element visit is charged.
                self.absorb_owner(iteration.owner);
                if iteration.state.awaiting {
                    self.pop()?;
                } else {
                    collections::charge_visit(self.meter)?;
                }
                return Err(fault);
            }
        };
        self.absorb_owner(iteration.owner);
        if iteration.state.awaiting {
            let result = self.pop()?;
            // Only a `sum` locates a stop at the body.
            let body = match node.kind() {
                NodeKind::Query { body, .. } => node.at(*body),
                _ => node,
            };
            let absorbed = iteration.state.absorb(&spec, result, |total, summand| {
                self.sum_step(node, body, total, summand)
            })?;
            match absorbed {
                Absorbed::Continue => {}
                Absorbed::Exit { found } => {
                    self.iterations = self.iterations.saturating_sub(1);
                    self.note_stop(&iteration)?;
                    let value = retain_scalar(Value::Boolean(found), self.meter)?;
                    self.push_value(value, false);
                    return Ok(());
                }
            }
        }
        let (element, position) = match iteration.state.next(&spec, self.meter).map_err(lift)? {
            Flow::Exhausted => return self.finish(*iteration),
            Flow::Visit { element, position } => (element, position),
        };
        let body = match node.kind() {
            NodeKind::Query {
                slot, body, source, ..
            } => {
                self.note_element(node.at(*source), &iteration.state.source, *slot, position);
                *self.slot(*slot)? = Some(element);
                iteration.owner = self.step_owner();
                self.set_slot_owner(*slot, iteration.source_owner)?;
                body
            }
            NodeKind::Fold {
                accumulator,
                binder,
                step,
                ..
            } => {
                let current = iteration
                    .state
                    .accumulator()
                    .cloned()
                    .ok_or_else(|| invariant("fold-accumulator-missing"))?;
                if let Some(trail) = self.trail.as_mut() {
                    trail.element_paths.remove(accumulator);
                    trail.element_paths.remove(binder);
                }
                *self.slot(*accumulator)? = Some(current);
                *self.slot(*binder)? = Some(element);
                iteration.owner = self.step_owner();
                self.set_slot_owner(*accumulator, iteration.owner)?;
                self.set_slot_owner(*binder, iteration.source_owner)?;
                step
            }
            _ => return Err(invariant("iteration-node-invalid")),
        };
        self.tasks.push(Task::Iterate(iteration));
        self.tasks.push(Task::Eval(node.at(*body)));
        Ok(())
    }

    /// FR-265: record `value` as the decision of the `and`, `or`,
    /// `implies` or `if` at `node`, on the claim's own level.
    fn decide(&mut self, node: N, value: bool) {
        if self.on_claim_level() {
            if let Some(trail) = self.trail.as_mut() {
                if let Some(location) = node.location() {
                    trail.decide(location, value);
                }
            }
        }
    }

    /// FR-265: a collection literal at `node` on the claim's own level is
    /// built inside the claim (FSpec FR-207-AC-9).
    fn note_built(&mut self, node: N, value: &Value) {
        if !matches!(value, Value::Collection(_)) {
            return;
        }
        if !self.on_claim_level() {
            return;
        }
        let Some(occurrence) = node.location().and_then(|location| {
            occurrence_at(self.world.graph()?, location, Some(Operator::Collection))
        }) else {
            return;
        };
        if let Some(trail) = self.trail.as_mut() {
            let provenance = Provenance::built(trail.observation.clone(), occurrence);
            trail.record(value, provenance);
        }
    }

    /// FR-265: a collection or record read from `member` of `object` at
    /// `node` on the claim's own level is stored there, in the observation
    /// the read observes.
    fn note_member(&mut self, node: N, object: &ObjectReference, member: &str, value: &Value) {
        if !locatable(value) || !self.on_claim_level() {
            return;
        }
        let pre = self.reads.is_some_and(|reads| {
            node.location().and_then(|location| reads.get(location))
                == Some(&fsl_semantics::model::clause_observation::Observation::Pre)
        });
        if let Some(trail) = self.trail.as_mut() {
            if trail.receiver_unlocated
                && (trail.receiver.as_ref() == Some(object)
                    || trail.receiver_pre.as_ref() == Some(object))
            {
                return;
            }
            let observation = match (&trail.pre, pre) {
                (Some(observation), true) => observation.clone(),
                _ => trail.observation.clone(),
            };
            let provenance = match trail.receiver_invocation.as_ref() {
                Some(invocation)
                    if trail.receiver.as_ref() == Some(object)
                        || trail.receiver_pre.as_ref() == Some(object) =>
                {
                    Provenance::receiver_field(observation, invocation.clone(), member.to_owned())
                }
                _ => Provenance::member(observation, object.clone(), member.to_owned()),
            };
            trail.record(value, provenance);
        }
    }

    /// FR-107 "Read paths": record the field or member selection at `node`,
    /// whose operand is `operand`, after its charge was met and its value
    /// read. `reference` is the object an attribute selection read from.
    fn note_read(
        &mut self,
        node: N,
        operand: N::Child,
        step: ReadStep<'_>,
        reference: Option<&ObjectReference>,
    ) {
        if self.trail.is_none() {
            return;
        }
        let path = self.read_path(node, operand, step, reference);
        if let Some(trail) = self.trail.as_mut() {
            if let Some(location) = node.location() {
                trail.read(location, path);
            }
        }
    }

    /// The runtime value path (FSpec FR-207) of the selection at `node`: its
    /// operand's path extended by the selection's step, or none when the
    /// operand has no location or the selection is not in the clause's own
    /// frame.
    fn read_path(
        &self,
        node: N,
        operand: N::Child,
        step: ReadStep<'_>,
        reference: Option<&ObjectReference>,
    ) -> Option<RuntimeValuePath> {
        let trail = self.trail.as_ref()?;
        if self.frames.len() != 1 || !self.imported.is_empty() {
            return None;
        }
        let operand = node.at(operand);
        let mut path = if matches!(step, ReadStep::Attribute(_)) && Self::navigated(operand) {
            // FR-107 "Read paths": a selection on a navigation's result is
            // located at the referenced object's declared identity.
            RuntimeValuePath {
                observation: trail.observation.clone(),
                subject: ValuePathSubject::Object(reference?.clone()),
                steps: Vec::new(),
            }
        } else {
            self.located(operand)?
        };
        let under_pre = self.reads.is_some_and(|reads| {
            node.location().and_then(|location| reads.get(location))
                == Some(&fsl_semantics::model::clause_observation::Observation::Pre)
        });
        if let (true, Some(pre)) = (under_pre, trail.pre.as_ref()) {
            path.observation = pre.clone();
        }
        match step {
            ReadStep::Field(name) => path.steps.push(ValuePathStep::Field(name.to_owned())),
            ReadStep::Attribute(name) => {
                let at_root = path.steps.is_empty();
                let receiver_binding = at_root
                    && matches!(
                        &path.subject,
                        ValuePathSubject::Invocation {
                            binding: InvocationBinding::Receiver,
                            ..
                        }
                    );
                if receiver_binding {
                    path.steps.push(ValuePathStep::Field(name.to_owned()));
                } else {
                    let declared = at_root
                        && matches!(&path.subject, ValuePathSubject::Object(object)
                            if Some(object) == reference);
                    if !declared {
                        path.steps
                            .push(ValuePathStep::Reference(reference?.clone()));
                    }
                    path.steps.push(ValuePathStep::Member(name.to_owned()));
                }
            }
        }
        Some(path)
    }

    /// Whether `node` is a relationship navigation, possibly under `value`,
    /// `pre` or a coercion.
    fn navigated(node: N) -> bool {
        match node.kind() {
            NodeKind::Navigate { .. } => true,
            NodeKind::Value(operand) | NodeKind::Pre(operand) | NodeKind::Coerce(operand, _) => {
                Self::navigated(node.at(*operand))
            }
            _ => false,
        }
    }

    /// Whether `node` is a field or member selection.
    fn selected(node: N) -> bool {
        matches!(
            node.kind(),
            NodeKind::Attribute { .. } | NodeKind::Field { .. }
        )
    }

    /// The path of the value `node` evaluated to, when it has one: a clause
    /// binding, a quantifier element, a selection or the payload of an
    /// optional value read from one.
    fn located(&self, node: N) -> Option<RuntimeValuePath> {
        let trail = self.trail.as_ref()?;
        match node.kind() {
            NodeKind::Local(slot) => trail.element_paths.get(slot).cloned().or_else(|| {
                let subject = trail.bindings.get(*slot)?.clone()?;
                Some(RuntimeValuePath {
                    observation: trail.observation.clone(),
                    subject,
                    steps: Vec::new(),
                })
            }),
            NodeKind::Attribute { .. } | NodeKind::Field { .. } => node
                .location()
                .and_then(|location| trail.selection_path(location))
                .cloned(),
            NodeKind::Value(operand) => {
                let mut path = self.located(node.at(*operand))?;
                path.steps.push(ValuePathStep::OptionValue);
                Some(path)
            }
            NodeKind::Pre(operand) | NodeKind::Coerce(operand, _) => {
                self.located(node.at(*operand))
            }
            _ => None,
        }
    }

    /// FR-107 "Read paths": the quantifier element at `position` of
    /// `domain`, bound to `slot`, is located where FR-265 locates it.
    fn note_element(
        &mut self,
        source: N,
        domain: &Arc<CollectionValue>,
        slot: Slot,
        position: usize,
    ) {
        if self.trail.is_none() || self.frames.len() != 1 || !self.imported.is_empty() {
            return;
        }
        let Some(trail) = self.trail.as_ref() else {
            return;
        };
        let path = if Self::navigated(source) {
            // An element of a navigation's result is located at its own
            // declared identity.
            match domain.elements().get(position) {
                Some(Value::Reference(object)) => Some(RuntimeValuePath {
                    observation: trail.observation.clone(),
                    subject: ValuePathSubject::Object(object.clone()),
                    steps: Vec::new(),
                }),
                _ => None,
            }
        } else if Self::selected(source)
            && trail
                .provenance(&Value::Collection(Arc::clone(domain)))
                .is_none()
        {
            // A collection read from an owner with no location has none.
            None
        } else {
            self.provenance_of(domain, source)
                .and_then(|provenance| provenance.position(position).map(|at| provenance.path(at)))
        };
        if let Some(trail) = self.trail.as_mut() {
            match path {
                Some(path) => {
                    trail.element_paths.insert(slot, path);
                }
                None => {
                    trail.element_paths.remove(&slot);
                }
            }
        }
    }

    /// FR-265: field `index` of a record with a stored location is stored
    /// at that location's field of the same name (FSpec FR-207's field
    /// selection). A tuple position has no field spelling and is not
    /// located.
    fn note_field(&mut self, composite: &Arc<CompositeValue>, index: usize, value: &Value) {
        if !locatable(value) || !self.on_claim_level() {
            return;
        }
        let Some(trail) = self.trail.as_mut() else {
            return;
        };
        let Some(record) = trail.provenance(&Value::Composite(Arc::clone(composite))) else {
            return;
        };
        let Some(CompositeShape::Record(fields)) = self
            .world
            .types()
            .composite(composite.declaration().clone())
            .map(|declaration| declaration.shape())
        else {
            return;
        };
        let Some(provenance) = fields
            .get(index)
            .and_then(|field| record.field(field.name()))
        else {
            return;
        };
        trail.record(value, provenance);
    }

    /// FR-265: the payload of a present optional value with a stored
    /// location is stored at that location's option-value selection
    /// (FSpec FR-207).
    fn note_unwrapped(&mut self, option: &Arc<OptionValue>, payload: &Value) {
        if !locatable(payload) || !self.on_claim_level() {
            return;
        }
        let Some(trail) = self.trail.as_mut() else {
            return;
        };
        let Some(provenance) = trail
            .provenance(&Value::Option(Arc::clone(option)))
            .and_then(Provenance::option_value)
        else {
            return;
        };
        trail.record(payload, provenance);
    }

    /// FR-265: a conversion of a collection with a recorded location keeps
    /// each element's stored position: `produced` lists the elements the
    /// conversion kept, in source order, and `value` is what it formed.
    fn note_converted(&mut self, source: &Arc<CollectionValue>, produced: &[Value], value: &Value) {
        let Value::Collection(collection) = value else {
            return;
        };
        if !self.on_claim_level() {
            return;
        }
        let Some(trail) = self.trail.as_mut() else {
            return;
        };
        let Some(provenance) = trail.provenance(&Value::Collection(Arc::clone(source))) else {
            return;
        };
        // Each kept element's position in the source, in source order.
        let mut cursor = 0usize;
        let mut sources = Vec::with_capacity(produced.len());
        for element in produced {
            let Some(offset) = source.elements()[cursor..]
                .iter()
                .position(|candidate| compare_keys(candidate, element) == Some(Ordering::Equal))
            else {
                return;
            };
            sources.push(cursor + offset);
            cursor += offset + 1;
        }
        let kind = collection.collection_type().kind();
        let Some(positions) = formed_sources(kind, &sources, produced, collection.elements())
        else {
            return;
        };
        if let Some(selected) = provenance.select(&positions) {
            trail.record(value, selected);
        }
    }

    /// FR-265 (FSpec FR-207 "Computed collections"): a `map` or `filter`
    /// on the claim's own level whose `results` came from `source_value`'s
    /// elements at `sources` formed `value`; each element of `value` is
    /// bound from the stored or built collection `source_value` is.
    fn note_computed(
        &mut self,
        source_value: &Arc<CollectionValue>,
        source: N,
        sources: &[usize],
        results: &[Value],
        value: &Value,
    ) {
        let Value::Collection(collection) = value else {
            return;
        };
        let Some(provenance) = self.provenance_of(source_value, source) else {
            return;
        };
        let kind = collection.collection_type().kind();
        let Some(positions) = formed_sources(kind, sources, results, collection.elements()) else {
            return;
        };
        if let (Some(selected), Some(trail)) = (provenance.select(&positions), self.trail.as_mut())
        {
            trail.record(value, selected);
        }
    }

    /// FR-265 (ADR-031 SW-1, SW-9): `iteration`, a `forall` or `exists`,
    /// stopped at its last visited element; on the claim's own level,
    /// report it with its quantifier, the element, its source position and
    /// value path.
    fn note_stop(&mut self, iteration: &Iteration<N>) -> Result<(), Halt> {
        if !self.on_claim_level() {
            return Ok(());
        }
        let NodeKind::Query { source, .. } = iteration.node.kind() else {
            return Err(invariant("query-stop-node-expected"));
        };
        let position = iteration
            .state
            .next
            .checked_sub(1)
            .ok_or_else(|| invariant("query-stop-position-invalid"))?;
        let element = iteration
            .state
            .source
            .elements()
            .get(position)
            .cloned()
            .ok_or_else(|| invariant("query-stop-position-invalid"))?;
        let quantifier = iteration
            .node
            .location()
            .and_then(|location| {
                occurrence_at(self.world.graph()?, location, Some(Operator::Quantify))
            })
            .ok_or_else(|| {
                Halt::Fault(InternalFault::new("S6a", "quantifier-has-an-occurrence"))
            })?;
        let provenance = self
            .provenance_of(&iteration.state.source, iteration.node.at(*source))
            .ok_or_else(|| Halt::Fault(InternalFault::new("S6a", "domain-has-an-occurrence")))?;
        let index = provenance
            .position(position)
            .ok_or_else(|| invariant("query-stop-provenance-position-invalid"))?;
        let report = StopReport {
            quantifier,
            index,
            element,
            value_path: provenance.path(index),
        };
        if let Some(trail) = self.trail.as_mut() {
            if let Some(location) = iteration.node.location() {
                trail.stopped(location, report);
            }
        }
        Ok(())
    }

    fn finish(&mut self, iteration: Iteration<N>) -> Result<(), Halt> {
        self.iterations = self.iterations.saturating_sub(1);
        let node = iteration.node;
        let spec = Self::iteration_spec(node)?;
        let mut state = iteration.state;
        // The empty sum of a family the shared iteration does not run (a
        // float or a quantity) takes this evaluator's zero (FR-145).
        if spec.shape() == Shape::Query(Visit::Sum)
            && state.accumulator.is_none()
            && !collections::is_exact_sum(spec.value_type())
        {
            let zero = sum_zero(spec.value_type())?;
            if !sum_admits(spec.value_type(), &zero) {
                return Err(Stop::Undefined(Undefined::SumOutOfDomain).into());
            }
            state.accumulator = Some(zero);
        }
        let source = Arc::clone(&state.source);
        let keep = self.on_claim_level();
        let finished = state.finish(&spec, keep, self.meter).map_err(lift)?;
        if let (
            Some((sources, results)),
            NodeKind::Query {
                source: operand, ..
            },
        ) = (finished.produced, node.kind())
        {
            self.note_computed(
                &source,
                node.at(*operand),
                &sources,
                &results,
                &finished.value,
            );
        }
        self.push_value(finished.value, false);
        Ok(())
    }
}

mod carried_sum;
#[cfg(test)]
mod invariants;
mod text_conversion;

pub(super) mod node_source;
pub(super) mod reader_source;
mod world;

/// The machine over checked bodies, the one source every test drives.
#[cfg(test)]
type CheckedMachine<'a, 'm> = Machine<'a, 'm, CheckedNode<'a>>;

#[cfg(test)]
mod union_case;

#[cfg(test)]
mod tests {
    use crate::value::composite::{construction_fault_identifier, ConstructionSite};
    use filament_exact::IntegerInterval;
    use filament_semantic_value::declaration::{
        Component, ConstructionCause, ConstructionRefusal, StorageRequest, StorageUnit,
    };

    fn fault_identifier(site: ConstructionSite, cause: ConstructionCause) -> &'static str {
        let refusal = ConstructionRefusal {
            component: Component::Value,
            cause,
        };
        construction_fault_identifier(site, &refusal)
    }

    /// A storage failure of tuple or record construction keeps its own fault
    /// identifier; every other cause stays `*-construction-rejected`.
    #[test]
    fn construction_storage_causes_have_their_own_fault_identifiers() {
        let request = StorageRequest::new(4, StorageUnit::AdditionalElements);
        let cases = [
            (
                ConstructionSite::Tuple,
                ConstructionCause::StorageDenied(request),
                "tuple-construction-admission-allocation-denied",
            ),
            (
                ConstructionSite::Tuple,
                ConstructionCause::StorageCapacity(request),
                "tuple-construction-admission-capacity-overflow",
            ),
            (
                ConstructionSite::Record,
                ConstructionCause::StorageDenied(request),
                "record-construction-admission-allocation-denied",
            ),
            (
                ConstructionSite::Record,
                ConstructionCause::StorageCapacity(request),
                "record-construction-admission-capacity-overflow",
            ),
            (
                ConstructionSite::Tuple,
                ConstructionCause::TypeMismatch,
                "tuple-construction-rejected",
            ),
            (
                ConstructionSite::Record,
                ConstructionCause::MissingField,
                "record-construction-rejected",
            ),
        ];
        for (site, cause, expected) in cases {
            assert_eq!(fault_identifier(site, cause), expected);
        }
    }

    use super::super::s6a::ReferenceEvaluation;
    use super::*;
    use filament_checked::key::DeclarationKey;
    use filament_checked::node::CheckedLiteral;
    use filament_exact::ObjectId;
    use filament_semantic_value::catalog::Category;
    use filament_semantic_value::checking::{CheckMode, CheckingLimits};
    use filament_semantic_value::object_closure::ObjectClosure;
    use fsl_forms::{Expression, FunctionDeclaration, TypeForm};
    use fsl_semantics::check::PackageDeclarations;
    use fsl_semantics::model::accounting::ModelNormalizationLimits;
    use fsl_semantics::model::dispatch::GeneralizationClosure;
    use fsl_semantics::model::domain_package::{
        DomainPackage, DomainPackageRecord, DomainPackageRef, Extent, ObjectTypeRecord,
        PopulationRecord,
    };
    use fsl_semantics::model::normalize::{normalize, NormalizeOutcome};
    use fsl_semantics::model::population::{
        admit_binding, AdmissionMeter, AdmissionOutcome, PopulationAdmissionLimits,
        PopulationDocument, PopulationMember,
    };
    use ix_trace_rs::trace;

    // `TypeForm`'s span carries no identity (ADR-011 §2.2 row E2).
    const SPAN: fsl_foundation::Span = fsl_foundation::Span { start: 0, end: 0 };

    /// FR-230-AC-43: a reference identifies its owner, canonical place key,
    /// and concrete slot path without carrying a clock value.
    #[trace("FR-230-AC-43")]
    #[test]
    fn clock_place_references_encode_the_place_and_ordered_slots() {
        let owner = ObjectReference::new(
            crate::test_ids::universe_id([1]),
            crate::test_ids::effective_id([2]),
            ObjectId::new("call").expect("fixture object id is non-empty"),
        );
        let place = crate::test_ids::node_key([4]);
        let clock_type = fsl_semantics::model::clock_reference::encode(&owner, &place, &[])
            .expect("the public codec issues the canonical Clock target")
            .object_type()
            .clone();
        let expected_prefix = "place/v1/63616c6c/7b226f7264696e616c223a302c227061636b616765223a2266697874757265222c2270617468223a5b225b345d225d7d";

        assert!(
            clock_place_reference(
                owner.clone(),
                crate::test_ids::effective_id([3]),
                &place,
                Vec::new(),
            )
            .is_err(),
            "a non-clock target is refused"
        );

        let unindexed =
            clock_place_reference(owner.clone(), clock_type.clone(), &place, Vec::new())
                .unwrap_or_else(|_| panic!("an unindexed place reference is well formed"));
        assert_eq!(unindexed.object().as_str(), format!("{expected_prefix}/-"));
        assert_eq!(unindexed.universe(), owner.universe());
        assert_eq!(unindexed.object_type(), &clock_type);

        let indexed = clock_place_reference(
            owner,
            clock_type,
            &place,
            vec![
                Value::Integer(Integer::from(0_i64)),
                Value::Integer(Integer::from(1_i64)),
            ],
        )
        .unwrap_or_else(|_| panic!("an indexed place reference is well formed"));
        assert_eq!(indexed.object().as_str(), format!("{expected_prefix}/0.1"));
        assert_ne!(indexed, unindexed);
    }

    #[trace("FSpec-TC-235", "FSpec-FR-208-AC-11", "FSpec-FR-207-AC-8")]
    #[test]
    fn context_pre_reads_remap_the_receiver_and_emit_invocation_field_paths() {
        use super::super::s6a::separation::{
            InvocationBinding, ObservationIdentity, RuntimeValuePath, ValuePathStep,
            ValuePathSubject,
        };
        use filament_semantic_value::declaration::{
            FieldDeclaration, ObjectTypeDeclaration, TypeEnvironment,
        };
        use std::collections::BTreeMap;
        let context_type = crate::test_ids::effective_id([12]);
        let collection_type =
            filament_exact::CollectionType::new(CollectionKind::Sequence, ValueType::Integer, None);
        let graph = {
            let mut declarations = PackageDeclarations::new(
                fsl_semantics::check::fixture_source(),
                "fixture".to_owned(),
                fsl_foundation::IdentityLimits::default(),
            );
            declarations.types = TypeEnvironment::new(
                [],
                [ObjectTypeDeclaration::new(
                    context_type.clone(),
                    "Context",
                    vec![FieldDeclaration::new(
                        "items",
                        ValueType::collection(collection_type.clone()),
                        filament_exact::Presence::Required,
                    )],
                )],
            )
            .unwrap();
            declarations
        }
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::value::S6aRewardEvaluator,
        )
        .unwrap();
        let package = fsl_package::CheckedPackage::link(graph);
        let field = || Expression::field(Expression::deref(Expression::name("self")), "items");
        let checked = package
            .graph()
            .check_clause_expression(
                vec![(
                    "self".to_owned(),
                    ValueType::Reference(context_type.clone()),
                )],
                &Expression::binary(fsl_forms::BinaryOperator::Equal, field(), field()),
                None,
                fsl_forms::ClauseKind::Postcondition,
                CheckMode::Kernel,
                CheckingLimits::default(),
            )
            .unwrap();
        let NodeKind::Equality(_, _, left, _) = checked.root().kind() else {
            panic!("checked equality");
        };
        let reads = BTreeMap::from([(
            checked.root().at(*left).location().clone(),
            fsl_semantics::model::clause_observation::Observation::Pre,
        )]);
        let reference = |universe| {
            ObjectReference::new(
                crate::test_ids::universe_id([universe]),
                context_type.clone(),
                filament_exact::ObjectId::new("i1").unwrap(),
            )
        };
        let pre_receiver = reference(1).clone();
        let receiver = reference(2);
        let pre_items = filament_exact::from_admitted(
            collection_type.clone(),
            vec![Value::Integer(Integer::from(0_i64))],
        );
        let post_items = filament_exact::from_admitted(
            collection_type,
            vec![Value::Integer(Integer::from(1_i64))],
        );
        let objects = |reference, items: Value| {
            ObjectEnvironment::new(
                ObjectClosure::new(
                    package.graph().scope().types(),
                    [(reference, vec![("items", FieldValue::Present(items))])],
                    &[],
                )
                .unwrap(),
            )
        };
        let pre = objects(pre_receiver.clone(), pre_items.clone());
        let post = objects(receiver.clone(), post_items.clone());
        let identity = |name: &str| ObservationIdentity {
            authority: "test".to_owned(),
            identity: name.to_owned(),
        };
        for context in [true, false] {
            let mut trail = Trail::new(identity("q1"), Some(identity("p1")));
            trail.receiver = Some(receiver.clone());
            trail.receiver_pre = Some(pre_receiver.clone());
            trail.receiver_invocation = context.then(|| identity("i1"));
            let mut meter = Meter::new(fsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
            let (result, trail) = Machine::with_pre(
                package.graph().scope(),
                package.graph(),
                &post,
                Some(&pre),
                Some(&reads),
                &mut meter,
                &[],
            )
            .with_trail(trail)
            .run_traced(
                checked.root(),
                checked.slots(),
                vec![Value::Reference(receiver.clone())],
            )
            .unwrap();
            assert!(matches!(
                result.outcome,
                FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false)))
            ));
            for (observation, object, value) in [
                ("p1", &pre_receiver, &pre_items),
                ("q1", &receiver, &post_items),
            ] {
                let subject = if context {
                    ValuePathSubject::Invocation {
                        invocation: identity("i1"),
                        binding: InvocationBinding::Receiver,
                    }
                } else {
                    ValuePathSubject::Object(object.clone())
                };
                let step = if context {
                    ValuePathStep::Field("items".to_owned())
                } else {
                    ValuePathStep::Member("items".to_owned())
                };
                assert_eq!(
                    trail.provenance(value).unwrap().path(0),
                    RuntimeValuePath {
                        observation: identity(observation),
                        subject,
                        steps: vec![step],
                    }
                );
            }
        }
    }

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
                    symmetric: false,
                }),
            ],
        )
    }

    /// One admitted binding over [`domain_package`], for
    /// [`Machine::resolve_population`]'s own fault tests below.
    pub(super) fn population_binding(declared_maximum: Option<u64>) -> PopulationBinding {
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
    /// [`fsl_semantics::check::ValueFunctionFamily::evaluate`] directly -- the S6a
    /// seam itself, bypassing `CheckedPackage::call`'s admission.
    pub(super) fn population_function_package(
    ) -> (fsl_package::CheckedPackage, filament_exact::NodeKey) {
        let domain_package = domain_package("bundle.qsl174-ac10-seam");
        let view = match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
            NormalizeOutcome::Completed(view) => view,
            other => panic!("expected a completed effective view, got {other:?}"),
        };
        let a = view
            .type_identities()
            .get(&DeclarationKey::fixture("model.A"))
            .cloned()
            .expect("model.A has a type-level effective declaration");
        let types = filament_semantic_value::declaration::TypeEnvironment::new(
            [],
            [
                filament_semantic_value::declaration::ObjectTypeDeclaration::new(
                    a,
                    "M::A",
                    Vec::new(),
                ),
            ],
        )
        .expect("one object type admits cleanly");
        let model = fsl_semantics::check::AdmittedModel::new(&domain_package, &view)
            .expect("the view is the domain package's own");
        let graph = {
            let mut declarations = PackageDeclarations::new(
                fsl_semantics::check::fixture_source(),
                "fixture".to_owned(),
                fsl_foundation::IdentityLimits::default(),
            );
            declarations.types = types;
            declarations.models = vec![model];
            declarations.functions = vec![FunctionDeclaration::new(
                "F",
                vec![(
                    "p".to_owned(),
                    TypeForm::new(fsl_forms::TypeFormHead::Population, SPAN)
                        .with_arguments(vec![TypeForm::name("M::A", SPAN)])
                        .with_bounds(vec!["3".to_owned()]),
                )],
                TypeForm::builtin(fsl_forms::BuiltinType::Integer, SPAN),
                None,
                Expression::size(
                    Expression::all_instances(
                        TypeForm::name("M::A", SPAN),
                        Expression::name("p".to_owned()),
                    ),
                    None,
                ),
            )];
            declarations
        }
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::value::S6aRewardEvaluator,
        )
        .expect("F(p: Population<M::A>[3]): Integer = size(allInstances<M::A>(p)) checks cleanly");
        let identity = graph
            .function_identity("F")
            .expect("F is declared in this package");
        (fsl_package::CheckedPackage::link(graph), identity)
    }

    /// TC-391 (FR-090-AC-10): a `Value::Population` argument whose id names
    /// no recorded binding, passed directly to the S6a seam
    /// (`ValueFunctionFamily::evaluate`) rather than through
    /// `CheckedPackage::call` -- the condition `call`'s own `validate`
    /// already refuses at admission (`InputRefusal::WrongValueKind`,
    /// TC-294/FR-089-AC-4) -- raises `Err(InternalFault)`
    /// naming stage `"S6a"` and this fault's own literal invariant
    /// identifier, never a panic and never `Ok(Evaluation { outcome:
    /// Outcome::Refused(_), .. })`.
    #[trace("FR-090-AC-10", "TC-391")]
    #[test]
    fn evaluate_bypassing_admission_with_an_unresolved_population_id_is_an_internal_fault() {
        let (package, identity) = population_function_package();
        let objects = ObjectEnvironment::default();
        let unresolved_id = PopulationId::new(crate::test_ids::node_key([7]));
        let mut env = crate::value::expression::family::EvaluationEnv::new(
            &package,
            &objects,
            vec![Value::Population(unresolved_id)],
        );
        let mut meter = Meter::new(fsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
        let fault =
            fsl_semantics::check::ValueFunctionFamily::evaluate(&identity, &mut env, &mut meter)
                .expect_err("an unresolved population id must fault, never evaluate");
        assert_eq!(fault.stage(), "S6a");
        assert_eq!(fault.category(), Category::InternalFailure);
        assert_eq!(
            fault.invariant(),
            "population-argument-unresolved-past-admission"
        );
    }

    /// TC-391 (FR-090-AC-10): the companion case, a `Value::Population`
    /// argument that resolves but whose binding's declared maximum (2)
    /// differs from `F`'s declared `Population<3>` -- also
    /// `Err(InternalFault)` from the S6a seam directly, with its own
    /// distinct literal invariant identifier.
    #[trace("FR-090-AC-10", "TC-391")]
    #[test]
    fn evaluate_bypassing_admission_with_a_population_maximum_mismatch_is_an_internal_fault() {
        let (package, identity) = population_function_package();
        let binding = population_binding(Some(2));
        let id = binding.population_id();
        let objects = ObjectEnvironment::default()
            .with_population(binding)
            .unwrap();
        let mut env = crate::value::expression::family::EvaluationEnv::new(
            &package,
            &objects,
            vec![Value::Population(id)],
        );
        let mut meter = Meter::new(fsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
        let fault =
            fsl_semantics::check::ValueFunctionFamily::evaluate(&identity, &mut env, &mut meter)
                .expect_err("a mismatched declared maximum must fault, never evaluate");
        assert_eq!(fault.stage(), "S6a");
        assert_eq!(fault.category(), Category::InternalFailure);
        assert_eq!(
            fault.invariant(),
            "population-argument-maximum-mismatch-past-admission"
        );
    }

    /// FR-883: `allInstances<T>(p)` follows the normal population evaluation
    /// path while a protocol clause is being evaluated. `reads: Some(_)`
    /// supplies the protocol evaluation context used by `Machine`.
    #[trace("TC-467")]
    #[test]
    fn all_instances_under_a_protocol_clause_evaluates() {
        let (package, _identity) = population_function_package();
        let graph = package.graph();
        let expression = graph
            .check_expression(
                vec![("p".to_owned(), ValueType::Population(Some(3)))],
                &Expression::all_instances(
                    TypeForm::name("M::A", SPAN),
                    Expression::name("p".to_owned()),
                ),
                None,
                CheckMode::Linked,
                CheckingLimits::default(),
            )
            .expect("allInstances<M::A>(p) checks as a standalone expression");
        let binding = population_binding(Some(3));
        let population_id = binding.population_id();
        let objects = ObjectEnvironment::default()
            .with_population(binding)
            .expect("the admitted population binds cleanly");
        let reads: std::collections::BTreeMap<
            Location,
            fsl_semantics::model::clause_observation::Observation,
        > = std::collections::BTreeMap::new();
        let mut meter = Meter::new(fsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
        let arguments = vec![Value::Population(population_id)];
        let evaluation = Machine::with_pre(
            graph.scope(),
            graph,
            &objects,
            None,
            Some(&reads),
            &mut meter,
            graph.dispatch_tables(),
        )
        .run(expression.root(), expression.slots(), arguments)
        .expect("population evaluation completes without an internal fault");
        match evaluation.outcome {
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Collection(values))) => {
                assert_eq!(values.elements().len(), 1);
            }
            other => panic!("expected one population member, got {other:?}"),
        }
    }

    /// FR-883-AC-2 (TC-2507): an absent `lookup` key under a protocol clause
    /// returns the ordinary absent-key result.
    #[trace("TC-2507", "FR-883-AC-2")]
    #[test]
    fn lookup_under_a_protocol_clause_evaluates() {
        use filament_semantic_value::absence::AbsenceMode;

        let (package, _identity) = population_function_package();
        let graph = package.graph();
        let a = match normalize(
            &domain_package("bundle.qsl174-ac10-seam"),
            ModelNormalizationLimits::UNLIMITED,
        ) {
            NormalizeOutcome::Completed(view) => view
                .type_identities()
                .get(&DeclarationKey::fixture("model.A"))
                .cloned()
                .expect("model.A has a type-level effective declaration"),
            other => panic!("expected a completed effective view, got {other:?}"),
        };
        let expression = graph
            .check_expression(
                vec![
                    ("p".to_owned(), ValueType::Population(Some(3))),
                    ("r".to_owned(), ValueType::Reference(a.clone())),
                ],
                &Expression::lookup(
                    TypeForm::name("M::A", SPAN),
                    Expression::name("p".to_owned()),
                    Expression::name("r".to_owned()),
                    AbsenceMode::Refused,
                ),
                None,
                CheckMode::Linked,
                CheckingLimits::default(),
            )
            .expect("lookup<M::A>(p, r) absent Refused checks as a standalone expression");
        let binding = population_binding(Some(3));
        let population_id = binding.population_id();
        let universe = binding.universe().clone();
        let objects = ObjectEnvironment::default()
            .with_population(binding)
            .expect("the admitted population binds cleanly");
        let reads: std::collections::BTreeMap<
            Location,
            fsl_semantics::model::clause_observation::Observation,
        > = std::collections::BTreeMap::new();
        let present_arguments = vec![
            Value::Population(population_id.clone()),
            Value::Reference(ObjectReference::new(
                universe.clone(),
                a.clone(),
                filament_exact::ObjectId::new("a1".to_owned()).expect("non-empty key"),
            )),
        ];
        let mut meter = Meter::new(fsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
        let present = Machine::with_pre(
            graph.scope(),
            graph,
            &objects,
            None,
            Some(&reads),
            &mut meter,
            graph.dispatch_tables(),
        )
        .run(expression.root(), expression.slots(), present_arguments)
        .expect("a present lookup completes under the protocol context");
        match present.outcome {
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Reference(reference))) => {
                assert_eq!(reference.object().as_str(), "a1");
            }
            other => panic!("expected the present population member, got {other:?}"),
        }

        let missing_arguments = vec![
            Value::Population(population_id.clone()),
            Value::Reference(ObjectReference::new(
                universe.clone(),
                a.clone(),
                filament_exact::ObjectId::new("root".to_owned()).expect("non-empty key"),
            )),
        ];
        let mut meter = Meter::new(fsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
        let missing = Machine::with_pre(
            graph.scope(),
            graph,
            &objects,
            None,
            Some(&reads),
            &mut meter,
            graph.dispatch_tables(),
        )
        .run(expression.root(), expression.slots(), missing_arguments)
        .expect("the absent lookup returns its family result, never an `Err`");
        match missing.outcome {
            FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause)) => {
                assert_eq!(cause.catalog_code().cause(), "absent-key");
            }
            other => panic!("expected the ordinary absent-key result, got {other:?}"),
        }

        let undefined_expression = graph
            .check_expression(
                vec![
                    ("p".to_owned(), ValueType::Population(Some(3))),
                    ("r".to_owned(), ValueType::Reference(a.clone())),
                ],
                &Expression::lookup(
                    TypeForm::name("M::A", SPAN),
                    Expression::name("p".to_owned()),
                    Expression::name("r".to_owned()),
                    AbsenceMode::Undefined,
                ),
                None,
                CheckMode::Linked,
                CheckingLimits::default(),
            )
            .expect("lookup<M::A>(p, r) absent Undefined checks under the protocol context");
        let undefined_arguments = vec![
            Value::Population(population_id),
            Value::Reference(ObjectReference::new(
                universe,
                a,
                filament_exact::ObjectId::new("root".to_owned()).expect("non-empty key"),
            )),
        ];
        let mut meter = Meter::new(fsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
        let undefined = Machine::with_pre(
            graph.scope(),
            graph,
            &objects,
            None,
            Some(&reads),
            &mut meter,
            graph.dispatch_tables(),
        )
        .run(
            undefined_expression.root(),
            undefined_expression.slots(),
            undefined_arguments,
        )
        .expect("the absent lookup returns its family result, never an `Err`");
        match undefined.outcome {
            FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause)) => {
                assert_eq!(cause.undefined_record().reason.as_str(), "absent-key");
            }
            other => panic!("expected the ordinary absent-key undefined result, got {other:?}"),
        }
    }

    /// SR-750 FND-013 (FR-107-evaluate-state-clauses-at-s6a.md:102-105): a
    /// `pre` read reached with no pre observation refuses `wrong_snapshot`/
    /// `wrong-anchor`, never silently falling back to `objects`
    /// (`self.objects`). Admission makes this unreachable for a
    /// well-formed selection (a `reads` entry marking a location `Pre`
    /// always comes with an attached `pre_objects`); this test bypasses
    /// admission the same way the two tests above do, to exercise the
    /// guard itself.
    #[trace("TC-467")]
    #[test]
    fn a_pre_read_with_no_pre_observation_refuses_wrong_anchor() {
        let a_with_field = fsl_semantics::model::domain_package::DomainPackage::new(
            fsl_semantics::model::domain_package::DomainPackageRef::fixture("bundle.qsl278-fnd013"),
            vec![
                fsl_semantics::model::domain_package::DomainPackageRecord::ObjectType(
                    fsl_semantics::model::domain_package::ObjectTypeRecord {
                        key: DeclarationKey::fixture("model.A"),
                        interface_features: None,
                        abstract_type: false,
                        supertypes: Vec::new(),
                    },
                ),
            ],
        );
        let view = match normalize(&a_with_field, ModelNormalizationLimits::UNLIMITED) {
            NormalizeOutcome::Completed(view) => view,
            other => panic!("expected a completed effective view, got {other:?}"),
        };
        let a = view
            .type_identities()
            .get(&DeclarationKey::fixture("model.A"))
            .cloned()
            .expect("model.A has a type-level effective declaration");
        let types = filament_semantic_value::declaration::TypeEnvironment::new(
            [],
            [
                filament_semantic_value::declaration::ObjectTypeDeclaration::new(
                    a.clone(),
                    "M::A",
                    vec![filament_semantic_value::declaration::FieldDeclaration::new(
                        "x",
                        filament_exact::ValueType::Integer,
                        filament_exact::Presence::Required,
                    )],
                ),
            ],
        )
        .expect("one object type with one field admits cleanly");
        let model = fsl_semantics::check::AdmittedModel::new(&a_with_field, &view)
            .expect("the view is the domain package's own");
        let graph = {
            let mut declarations = PackageDeclarations::new(
                fsl_semantics::check::fixture_source(),
                "fixture".to_owned(),
                fsl_foundation::IdentityLimits::default(),
            );
            declarations.types = types;
            declarations.models = vec![model];
            declarations
        }
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::value::S6aRewardEvaluator,
        )
        .expect("one type with one field checks cleanly");

        let expression = graph
            .check_expression(
                vec![("r".to_owned(), ValueType::Reference(a.clone()))],
                &Expression::field(
                    Expression::deref(Expression::name("r".to_owned())),
                    "x".to_owned(),
                ),
                None,
                CheckMode::Linked,
                CheckingLimits::default(),
            )
            .expect("deref(r).x checks as a standalone expression");
        // `reads` marks the Attribute node's own location `Pre`, but
        // `pre_objects` (below) is `None` -- exactly the caller-input
        // defect `objects_for` must now refuse rather than silently read
        // `objects` (the post/current environment) for.
        let reads: std::collections::BTreeMap<
            Location,
            fsl_semantics::model::clause_observation::Observation,
        > = std::collections::BTreeMap::from([(
            expression.root().location().clone(),
            fsl_semantics::model::clause_observation::Observation::Pre,
        )]);
        let objects = ObjectEnvironment::default();
        let mut meter = Meter::new(fsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
        let arguments = vec![Value::Reference(ObjectReference::new(
            crate::test_ids::universe_id([9]),
            a,
            filament_exact::ObjectId::new("a1".to_owned()).expect("non-empty key"),
        ))];
        let evaluation = Machine::with_pre(
            graph.scope(),
            &graph,
            &objects,
            None,
            Some(&reads),
            &mut meter,
            graph.dispatch_tables(),
        )
        .run(expression.root(), expression.slots(), arguments)
        .expect("a family-owned refusal is `Ok(FamilyEvaluated)`, never an `Err`");
        match evaluation.outcome {
            FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause)) => {
                let code = cause.catalog_code();
                assert_eq!(code.code(), "wrong_snapshot");
                assert_eq!(code.cause(), "wrong-anchor");
            }
            other => panic!("expected FamilyEvaluated(Refused(_)), got {other:?}"),
        }
    }

    /// SR-750 FND-008 round 2 (`FR-107-evaluate-state-clauses-at-s6a.md:92-93`,
    /// "charge `model.deref` for each `deref(...)` and `model.navigate` for
    /// each field read"): a protocol-clause `deref(r).x` charges both
    /// `ModelDeref` and `ModelNavigate`, in that order -- confirmed by
    /// running it (this fails against the code before this fix, which
    /// charged `ModelNavigate` alone for every `Attribute` read regardless
    /// of an explicit `deref(...)`).
    #[trace("TC-467")]
    #[test]
    fn a_protocol_clause_deref_charges_model_deref_then_model_navigate() {
        let a_with_field = fsl_semantics::model::domain_package::DomainPackage::new(
            fsl_semantics::model::domain_package::DomainPackageRef::fixture("bundle.qsl278-fnd008"),
            vec![
                fsl_semantics::model::domain_package::DomainPackageRecord::ObjectType(
                    fsl_semantics::model::domain_package::ObjectTypeRecord {
                        key: DeclarationKey::fixture("model.A"),
                        interface_features: None,
                        abstract_type: false,
                        supertypes: Vec::new(),
                    },
                ),
            ],
        );
        let view = match normalize(&a_with_field, ModelNormalizationLimits::UNLIMITED) {
            NormalizeOutcome::Completed(view) => view,
            other => panic!("expected a completed effective view, got {other:?}"),
        };
        let a = view
            .type_identities()
            .get(&DeclarationKey::fixture("model.A"))
            .cloned()
            .expect("model.A has a type-level effective declaration");
        let types = filament_semantic_value::declaration::TypeEnvironment::new(
            [],
            [
                filament_semantic_value::declaration::ObjectTypeDeclaration::new(
                    a.clone(),
                    "M::A",
                    vec![filament_semantic_value::declaration::FieldDeclaration::new(
                        "x",
                        filament_exact::ValueType::Integer,
                        filament_exact::Presence::Required,
                    )],
                ),
            ],
        )
        .expect("one object type with one field admits cleanly");
        let model = fsl_semantics::check::AdmittedModel::new(&a_with_field, &view)
            .expect("the view is the domain package's own");
        let graph = {
            let mut declarations = PackageDeclarations::new(
                fsl_semantics::check::fixture_source(),
                "fixture".to_owned(),
                fsl_foundation::IdentityLimits::default(),
            );
            declarations.types = types;
            declarations.models = vec![model];
            declarations
        }
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::value::S6aRewardEvaluator,
        )
        .expect("one type with one field checks cleanly");

        let expression = graph
            .check_expression(
                vec![("r".to_owned(), ValueType::Reference(a.clone()))],
                &Expression::field(
                    Expression::deref(Expression::name("r".to_owned())),
                    "x".to_owned(),
                ),
                None,
                CheckMode::Linked,
                CheckingLimits::default(),
            )
            .expect("deref(r).x checks as a standalone expression");
        // Marking this location `Current` (never `Pre`) makes `reads.is_some()`
        // true (`is_protocol_clause`) while `objects_for` reads `objects`
        // itself, no `pre_objects` needed.
        let reads: std::collections::BTreeMap<
            Location,
            fsl_semantics::model::clause_observation::Observation,
        > = std::collections::BTreeMap::from([(
            expression.root().location().clone(),
            fsl_semantics::model::clause_observation::Observation::Current,
        )]);
        let reference = ObjectReference::new(
            crate::test_ids::universe_id([9]),
            a,
            filament_exact::ObjectId::new("a1".to_owned()).expect("non-empty key"),
        );
        let objects = ObjectEnvironment::new(
            ObjectClosure::new(
                graph.scope().types(),
                [(
                    reference.clone(),
                    vec![(
                        "x",
                        filament_exact::FieldValue::Present(Value::Integer(7_i64.into())),
                    )],
                )],
                &[],
            )
            .expect("one object with its one required field admits cleanly"),
        );
        let mut meter = Meter::new(fsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
        let arguments = vec![Value::Reference(reference)];
        let evaluation = Machine::with_pre(
            graph.scope(),
            &graph,
            &objects,
            None,
            Some(&reads),
            &mut meter,
            graph.dispatch_tables(),
        )
        .run(expression.root(), expression.slots(), arguments)
        .expect("deref(r).x over an admitted object completes");
        match evaluation.outcome {
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value))) => {
                assert_eq!(value, filament_exact::Integer::from(7_i64));
            }
            other => panic!("expected Evaluated(Completed(Integer(_))), got {other:?}"),
        }
        let charges: Vec<_> = meter
            .admitted_charges()
            .iter()
            .map(|charge| charge.point)
            .filter(|point| matches!(point, ChargePoint::ModelDeref | ChargePoint::ModelNavigate))
            .collect();
        assert_eq!(
            charges,
            [ChargePoint::ModelDeref, ChargePoint::ModelNavigate],
            "deref(r).x charges model.deref once, then model.navigate once"
        );
    }

    /// FR-265 (FSpec FR-207's field and option-value selections): on the
    /// claim's own level, a record read from a stored object locates its
    /// field `items` at the record's path plus a `Field` step, and the
    /// payload of a present optional value stored there at that path plus
    /// an `OptionValue` step. A domain package declares no record-typed
    /// member, so no admitted snapshot stores a record; this drives
    /// `note_field` and `note_unwrapped` directly.
    #[trace("TC-740", "FR-265-AC-7")]
    #[test]
    fn a_stored_record_locates_its_field_and_an_option_its_payload() {
        use super::super::s6a::separation::{
            ObservationIdentity, RuntimeValuePath, ValuePathStep, ValuePathSubject,
        };
        use filament_semantic_value::declaration::{
            CompositeDeclaration, FieldDeclaration as DeclaredField, TypeEnvironment,
        };
        let items_type =
            filament_exact::CollectionType::new(CollectionKind::Sequence, ValueType::Integer, None);
        let point = crate::test_ids::node_key([4]);
        let record = CompositeDeclaration::new(
            point.clone(),
            "Holder",
            CompositeShape::Record(vec![DeclaredField::new(
                "items",
                ValueType::collection(items_type.clone()),
                filament_exact::Presence::Required,
            )]),
        );
        let graph = {
            let mut declarations = PackageDeclarations::new(
                fsl_semantics::check::fixture_source(),
                "fixture".to_owned(),
                fsl_foundation::IdentityLimits::default(),
            );
            declarations.types = TypeEnvironment::new([record], []).expect("one record admits");
            declarations
        }
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::value::S6aRewardEvaluator,
        )
        .expect("the record checks");
        let package = fsl_package::CheckedPackage::link(graph);
        let objects = ObjectEnvironment::default();
        let mut meter = Meter::new(fsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
        let observation = ObservationIdentity {
            authority: "test".to_owned(),
            identity: "current".to_owned(),
        };
        let mut machine = CheckedMachine::new(
            package.graph().scope(),
            package.graph(),
            &objects,
            &mut meter,
            &[],
        )
        .with_trail(Trail::new(observation.clone(), None));
        assert!(machine.enter_frame(0, Vec::new(), false, false).is_ok());

        let items = filament_exact::from_admitted(
            items_type.clone(),
            vec![Value::Integer(Integer::from(1_i64))],
        );
        let stored = filament_exact::from_admitted_slots(
            point,
            vec![FieldValue::Present(items.clone())].into(),
        );
        let Value::Composite(holder) = &stored else {
            panic!("a record value");
        };
        let object = ObjectReference::new(
            crate::test_ids::universe_id([1]),
            crate::test_ids::effective_id([2]),
            filament_exact::ObjectId::new("o".to_owned()).unwrap(),
        );
        let trail = machine.trail.as_mut().unwrap();
        trail.record(
            &stored,
            Provenance::member(observation.clone(), object.clone(), "holder".to_owned()),
        );
        machine.note_field(holder, 0, &items);
        let path = |steps: Vec<ValuePathStep>| RuntimeValuePath {
            observation: observation.clone(),
            subject: ValuePathSubject::Object(object.clone()),
            steps,
        };
        let trail = machine.trail.as_ref().unwrap();
        assert_eq!(
            trail.provenance(&items).map(|located| located.path(0)),
            Some(path(vec![
                ValuePathStep::Member("holder".to_owned()),
                ValuePathStep::Field("items".to_owned()),
            ]))
        );

        let option = OptionValue::present(ValueType::collection(items_type), items.clone())
            .expect("the payload admits");
        let Value::Option(maybe) = &option else {
            panic!("an optional value");
        };
        let trail = machine.trail.as_mut().unwrap();
        trail.record(
            &option,
            Provenance::member(observation.clone(), object.clone(), "maybe".to_owned()),
        );
        let payload = maybe.payload().cloned().unwrap();
        machine.note_unwrapped(maybe, &payload);
        let trail = machine.trail.as_ref().unwrap();
        assert_eq!(
            trail.provenance(&payload).map(|located| located.path(0)),
            Some(path(vec![
                ValuePathStep::Member("maybe".to_owned()),
                ValuePathStep::OptionValue,
            ]))
        );
    }
    /// FR-106 check 6.5: only a witnessed optional slot skips the checked
    /// construction; the same out-of-range integer in an unwitnessed slot,
    /// and a value of the wrong kind in any slot, is a broken invariant.
    #[trace("TC-465", "FR-106-AC-12")]
    #[test]
    fn project_relaxes_only_a_witnessed_optional_slot() {
        let range = ValueType::Int(
            IntegerInterval::new(Integer::from(0_i64), Integer::from(1000_i64))
                .expect("a non-empty interval"),
        );
        let optional = ValueType::option(range);
        let outside = FieldValue::Present(Value::Integer(Integer::from(-1_i64)));
        assert!(CheckedMachine::project(
            &TypeEnvironment::default(),
            &outside,
            true,
            &optional,
            true
        )
        .is_ok());
        assert!(CheckedMachine::project(
            &TypeEnvironment::default(),
            &outside,
            true,
            &optional,
            false
        )
        .is_err());
        let wrong_kind = FieldValue::Present(Value::Boolean(true));
        assert!(CheckedMachine::project(
            &TypeEnvironment::default(),
            &wrong_kind,
            true,
            &optional,
            false
        )
        .is_err());
    }
    /// FR-106 check 6.5: an equality compares an out-of-type integer by
    /// value only when the operand reads a witnessed slot. Two integers
    /// outside `Int[0, 1000]` that no witnessed slot produced (here, passed
    /// in as arguments that bypass admission) still fault, whether or not
    /// an equal value was witnessed elsewhere.
    #[trace("TC-465", "FR-106-AC-12")]
    #[test]
    fn an_unwitnessed_out_of_type_operand_equal_to_a_witnessed_value_still_faults() {
        let (package, _identity) = population_function_package();
        let graph = package.graph();
        let range = ValueType::Int(
            IntegerInterval::new(Integer::from(0_i64), Integer::from(1000_i64))
                .expect("a non-empty interval"),
        );
        let expression = graph
            .check_expression(
                vec![("x".to_owned(), range.clone()), ("y".to_owned(), range)],
                &Expression::binary(
                    fsl_forms::BinaryOperator::Equal,
                    Expression::name("x".to_owned()),
                    Expression::name("y".to_owned()),
                ),
                None,
                CheckMode::Linked,
                CheckingLimits::default(),
            )
            .expect("x = y checks as a standalone expression");
        let objects = ObjectEnvironment::default();
        let reads: std::collections::BTreeMap<
            Location,
            fsl_semantics::model::clause_observation::Observation,
        > = std::collections::BTreeMap::new();
        let mut meter = Meter::new(fsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
        let outside = Value::Integer(Integer::from(-1_i64));
        let fault = Machine::with_pre(
            graph.scope(),
            graph,
            &objects,
            None,
            Some(&reads),
            &mut meter,
            graph.dispatch_tables(),
        )
        .run(
            expression.root(),
            expression.slots(),
            vec![outside.clone(), outside],
        )
        .expect_err("operands outside their type with no witnessed read fault");
        assert_eq!(fault.invariant(), "checked-program-invariant");
        assert_eq!(
            fault.kernel_cause(),
            Some(filament_exact::CheckedInvariantCause::EqualityOperandSourceNotAdmitted)
        );
    }
    /// FR-265 records a decision for each authored `and`; the connectives a
    /// float ordering lowers to carry none, so a claim's trail holds exactly
    /// the authored decision.
    #[trace("TC-1091", "FR-397-AC-12")]
    #[test]
    fn a_float_ordering_records_no_decision_but_an_authored_connective_does() {
        use filament_exact::{FloatType, IeeeValue, IeeeWidth};
        use fsl_forms::BinaryOperator;
        use fsl_semantics::value::{admit_ieee_profile, DefinitionRole};
        let package = fsl_package::CheckedPackage::link(
            {
                let mut declarations = PackageDeclarations::new(
                    fsl_semantics::check::fixture_source(),
                    "fixture".to_owned(),
                    fsl_foundation::IdentityLimits::default(),
                );
                declarations.ieee_profile = Some(
                    admit_ieee_profile(&[DefinitionRole::IeeeProfile.reference()], &[]).unwrap(),
                );
                declarations
            }
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &crate::value::S6aRewardEvaluator,
            )
            .unwrap(),
        );
        let double = ValueType::Float(FloatType::exact(IeeeWidth::Binary64));
        let ordering =
            |operator| Expression::binary(operator, Expression::name("f"), Expression::name("g"));
        let run = |expression: &Expression| {
            let checked = package
                .graph()
                .check_clause_expression(
                    vec![
                        ("f".to_owned(), double.clone()),
                        ("g".to_owned(), double.clone()),
                    ],
                    expression,
                    None,
                    fsl_forms::ClauseKind::Invariant,
                    CheckMode::Kernel,
                    CheckingLimits::default(),
                )
                .unwrap();
            let objects = ObjectEnvironment::default();
            let mut meter = Meter::new(fsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
            let trail = Trail::new(
                super::super::s6a::separation::ObservationIdentity {
                    authority: "test".to_owned(),
                    identity: "current".to_owned(),
                },
                None,
            );
            let (evaluation, trail) = Machine::new(
                package.graph().scope(),
                package.graph(),
                &objects,
                &mut meter,
                &[],
            )
            .with_trail(trail)
            .run_traced(
                checked.root(),
                checked.slots(),
                vec![
                    Value::Float(IeeeValue::binary64(0x3ff0_0000_0000_0000)),
                    Value::Float(IeeeValue::binary64(0x4000_0000_0000_0000)),
                ],
            )
            .unwrap();
            assert!(matches!(
                evaluation.outcome,
                FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
            ));
            let mut nodes = vec![checked.root()];
            let mut decisions = Vec::new();
            while let Some(node) = nodes.pop() {
                let children: Vec<_> = match node.kind() {
                    NodeKind::Connective(_, _, left, right) => vec![*left, *right],
                    NodeKind::Not(operand) => vec![*operand],
                    _ => Vec::new(),
                };
                if matches!(node.kind(), NodeKind::Connective(..)) {
                    decisions.push(trail.decision(node.location()));
                }
                nodes.extend(children.into_iter().map(|child| node.at(child)));
            }
            decisions
        };
        // 1.0 < 2.0 and 1.0 <= 2.0: no connective carries a decision.
        for operator in [BinaryOperator::Less, BinaryOperator::LessOrEqual] {
            let decisions = run(&ordering(operator));
            assert!(!decisions.is_empty(), "the ordering lowers to connectives");
            assert!(decisions.iter().all(Option::is_none), "{decisions:?}");
        }
        // An authored `and` over two orderings records exactly its own
        // decision, the value of its left operand.
        let authored = Expression::binary(
            BinaryOperator::And,
            ordering(BinaryOperator::Less),
            ordering(BinaryOperator::Less),
        );
        let decisions = run(&authored);
        assert_eq!(decisions.iter().flatten().count(), 1, "{decisions:?}");
        assert!(decisions.contains(&Some(true)));
    }
    /// FR-265: whether a connective records a decision is read from its
    /// origin, not from where it sits. Every node of this body has one
    /// location, so an authored connective shares its left operand's.
    #[trace("TC-1091", "FR-397-AC-12")]
    #[test]
    fn a_connective_records_a_decision_by_its_origin_not_its_location() {
        use fsl_semantics::check::CheckedBody;
        let package = fsl_package::CheckedPackage::link(
            PackageDeclarations::new(
                fsl_semantics::check::fixture_source(),
                "fixture".to_owned(),
                fsl_foundation::IdentityLimits::default(),
            )
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &crate::value::S6aRewardEvaluator,
            )
            .unwrap(),
        );
        let decision = |origin| {
            let literal = || {
                (
                    NodeKind::Literal(CheckedLiteral(Value::Boolean(true))),
                    ValueType::Boolean,
                )
            };
            let body = CheckedBody::from_nodes(vec![
                literal(),
                literal(),
                (
                    NodeKind::Connective(
                        Connective::And,
                        origin,
                        CheckedBody::id_at(0),
                        CheckedBody::id_at(1),
                    ),
                    ValueType::Boolean,
                ),
            ]);
            assert_eq!(
                body.root().at(CheckedBody::id_at(0)).location(),
                body.root().location()
            );
            let objects = ObjectEnvironment::default();
            let mut meter = Meter::new(fsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
            let trail = Trail::new(
                super::super::s6a::separation::ObservationIdentity {
                    authority: "test".to_owned(),
                    identity: "current".to_owned(),
                },
                None,
            );
            let (_, trail) = Machine::new(
                package.graph().scope(),
                package.graph(),
                &objects,
                &mut meter,
                &[],
            )
            .with_trail(trail)
            .run_traced(body.root(), 0, Vec::new())
            .unwrap();
            trail.decision(body.root().location())
        };
        assert_eq!(decision(ConnectiveOrigin::Authored), Some(true));
        assert_eq!(decision(ConnectiveOrigin::Synthesized), None);
    }

    /// FR-047-AC-6: an absent optional edge slot and an empty edge sequence
    /// have no outgoing edge, a sequence edge gives one edge per element in
    /// order, and no other edge shape or an absent required edge is admitted.
    #[trace("TC-130", "FR-047-AC-6")]
    #[test]
    fn absent_optional_and_empty_sequence_edges_have_no_outgoing_edge() {
        use crate::test_ids::{effective_id, universe_id};
        use filament_exact::{
            from_admitted, CollectionKind, CollectionType, FieldValue, ObjectId, Presence,
        };

        let target = effective_id("Node");
        let reference = |object: &str| {
            ObjectReference::new(
                universe_id("graph"),
                target.clone(),
                ObjectId::new(object).unwrap(),
            )
        };
        let edge_type = ValueType::Reference(target.clone());
        let sequence = |elements: &[&str]| {
            FieldValue::Present(from_admitted(
                CollectionType::new(CollectionKind::Sequence, edge_type.clone(), None),
                elements
                    .iter()
                    .map(|object| Value::Reference(reference(object)))
                    .collect(),
            ))
        };
        let targets = |field: &FieldDeclaration, slot: &FieldValue| {
            CheckedMachine::slot_edge_targets(field, slot).ok()
        };

        let optional = FieldDeclaration::new("next", edge_type.clone(), Presence::Optional);
        assert_eq!(targets(&optional, &FieldValue::Absent), Some(Vec::new()));
        assert_eq!(targets(&optional, &FieldValue::Null), Some(Vec::new()));
        assert_eq!(
            targets(
                &optional,
                &FieldValue::Present(Value::Reference(reference("b")))
            ),
            Some(vec![reference("b")])
        );

        let required = FieldDeclaration::new("parent", edge_type.clone(), Presence::Required);
        assert_eq!(targets(&required, &FieldValue::Absent), None);

        let kids = FieldDeclaration::new(
            "kids",
            ValueType::Collection(
                CollectionType::new(CollectionKind::Sequence, edge_type.clone(), None).into(),
            ),
            Presence::Required,
        );
        assert_eq!(targets(&kids, &sequence(&[])), Some(Vec::new()));
        assert_eq!(
            targets(&kids, &sequence(&["b", "a"])),
            Some(vec![reference("b"), reference("a")]),
            "one edge per element, in order"
        );

        let set = FieldDeclaration::new(
            "tags",
            ValueType::Collection(
                CollectionType::new(CollectionKind::Set, edge_type.clone(), None).into(),
            ),
            Presence::Required,
        );
        assert_eq!(targets(&set, &sequence(&["a"])), None);
    }
}
