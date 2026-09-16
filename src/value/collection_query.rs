// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-145 collection queries and explicit kind conversions.
//!
//! Every operation first type-checks (an [`IllTyped`] refusal consumes
//! nothing), then derives the result kind, order, uniqueness, multiplicity and
//! bound from the operation matrix. Caller-supplied functions are the only
//! metered steps (SPEC-GAP(119-9)); a non-completed function outcome stops the
//! operation with that outcome and no partial collection.

use super::accounting::Meter;
use super::collection::{
    keyed_ascending, CardinalityBound, CardinalityViolation, CollectionBuilder, CollectionKind,
    CollectionValue,
};
use super::comparison::{IllTyped, IllTypedCause};
use super::composite::{Value, ValueType};
use super::equality::decide_equal;
use super::integer::Integer;
use super::outcome::{Outcome, Refusal, Undefined};

/// A typed pure unary function, such as a `map` body or `filter` predicate.
pub trait ValueFunction {
    /// The declared parameter type.
    fn parameter_type(&self) -> &ValueType;
    /// The declared result type.
    fn result_type(&self) -> &ValueType;
    /// Apply the function to one argument of the parameter type.
    fn apply(&self, argument: &Value, meter: &mut Meter) -> Outcome<Value>;
}

/// Declared algebraic facts about a binary fold function.
// SPEC-GAP(119-13): FR-145 says fold and reduce "select a catalog operation"
// with an associativity requirement, but no catalog of such operations or of
// their properties is pinned. The properties are declared by the function and
// trusted, never inferred.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct AlgebraicProperties {
    /// `f(a, b) = f(b, a)`.
    pub commutative: bool,
    /// `f(f(a, b), c) = f(a, f(b, c))`.
    pub associative: bool,
}

/// A typed pure binary fold function `accumulator × element → accumulator`.
pub trait FoldFunction {
    /// The declared accumulator (and result) type.
    fn accumulator_type(&self) -> &ValueType;
    /// The declared element type.
    fn element_type(&self) -> &ValueType;
    /// The declared algebraic properties.
    fn properties(&self) -> AlgebraicProperties;
    /// Combine the accumulator with one occurrence.
    fn apply(&self, accumulator: &Value, element: &Value, meter: &mut Meter) -> Outcome<Value>;
}

/// The properties a kind conversion discards.
// SPEC-GAP(119-14): FR-145 records "each discarded property" but not whether
// loss is a kind-level fact or depends on the value (a sequence without
// duplicates loses no occurrence when converted to a set). It is kind-level:
// every property the source kind guarantees and the target kind does not.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct CollectionLoss {
    /// Semantic occurrence order.
    pub order: bool,
    /// The uniqueness guarantee.
    pub uniqueness: bool,
    /// Multiplicity of equal occurrences.
    pub multiplicity: bool,
}

impl CollectionLoss {
    /// No property is discarded.
    pub const NONE: Self = Self {
        order: false,
        uniqueness: false,
        multiplicity: false,
    };

    /// Every property may be discarded.
    pub const ALL: Self = Self {
        order: true,
        uniqueness: true,
        multiplicity: true,
    };

    /// The exact loss of converting `source` to `target`.
    pub fn of_conversion(source: CollectionKind, target: CollectionKind) -> Self {
        let (source, target) = (Self::guaranteed(source), Self::guaranteed(target));
        Self {
            order: source.order && !target.order,
            uniqueness: source.uniqueness && !target.uniqueness,
            multiplicity: source.multiplicity && !target.multiplicity,
        }
    }

    fn guaranteed(kind: CollectionKind) -> Self {
        Self {
            order: kind.is_ordered(),
            uniqueness: kind.is_unique(),
            multiplicity: !kind.is_unique(),
        }
    }

    /// Whether every property in `self` is also in `accepted`.
    pub fn is_within(self, accepted: Self) -> bool {
        (!self.order || accepted.order)
            && (!self.uniqueness || accepted.uniqueness)
            && (!self.multiplicity || accepted.multiplicity)
    }
}

/// A completed explicit kind conversion and its exact recorded loss.
#[derive(Clone, Debug)]
pub struct CollectionConversion {
    value: Value,
    loss: CollectionLoss,
}

impl CollectionConversion {
    /// The converted collection.
    pub fn value(&self) -> &Value {
        &self.value
    }

    /// The discarded properties.
    pub fn loss(&self) -> CollectionLoss {
        self.loss
    }
}

fn ill_typed<T>(cause: IllTypedCause) -> Result<T, IllTyped> {
    Err(IllTyped { cause })
}

fn cardinality<T>(violation: CardinalityViolation) -> Outcome<T> {
    Outcome::Refused(Refusal::Cardinality(violation))
}

/// Materialize `occurrences` (`count` of them, all of `element_type`) as
/// `kind` within `bound`. The bound is checked before any occurrence is
/// cloned.
fn build<'a>(
    kind: CollectionKind,
    element_type: ValueType,
    bound: Option<CardinalityBound>,
    count: usize,
    occurrences: impl IntoIterator<Item = &'a Value>,
) -> Outcome<Value> {
    let result =
        CollectionBuilder::start(kind, element_type, bound, count).and_then(|mut builder| {
            occurrences
                .into_iter()
                .try_for_each(|occurrence| builder.push(occurrence.clone()))?;
            builder.finish()
        });
    match result {
        Ok(value) => Outcome::Completed(value),
        Err(violation) => cardinality(violation),
    }
}

/// A function outcome whose completed value must be of `result_type`; any
/// other outcome stops the operation unchanged.
fn call(result: Outcome<Value>, result_type: &ValueType) -> Result<Value, Outcome<Value>> {
    match result {
        Outcome::Completed(value) if result_type.admits(&value) => Ok(value),
        Outcome::Completed(_) => Err(Outcome::Refused(Refusal::FunctionResultOutsideType)),
        Outcome::Undefined(reason) => Err(Outcome::Undefined(reason)),
        Outcome::Refused(reason) => Err(Outcome::Refused(reason)),
        Outcome::Incomplete(record) => Err(Outcome::Incomplete(record)),
    }
}

/// `map` / `collect`: apply `function` to every retained occurrence in order
/// and materialize the results as the source kind within `bound`. A set
/// coalesces equal results, a bag keeps result multiplicity, and an ordered
/// set keeps first-result order.
pub fn map(
    source: &CollectionValue,
    function: &dyn ValueFunction,
    bound: Option<CardinalityBound>,
    meter: &mut Meter,
) -> Result<Outcome<Value>, IllTyped> {
    if function.parameter_type() != source.element_type() {
        return ill_typed(IllTypedCause::FunctionParameterType);
    }
    let result_type = function.result_type();
    let mut builder = match CollectionBuilder::start(
        source.kind(),
        result_type.clone(),
        bound,
        source.elements().len(),
    ) {
        Ok(builder) => builder,
        Err(violation) => return Ok(cardinality(violation)),
    };
    for element in source.elements() {
        let value = match call(function.apply(element, meter), result_type) {
            Ok(value) => value,
            Err(stop) => return Ok(stop),
        };
        if let Err(violation) = builder.push(value) {
            return Ok(cardinality(violation));
        }
    }
    Ok(match builder.finish() {
        Ok(value) => Outcome::Completed(value),
        Err(violation) => cardinality(violation),
    })
}

/// `filter`: keep the occurrences for which `predicate` is `true`, preserving
/// the source kind, surviving order and multiplicity, within `bound`.
pub fn filter(
    source: &CollectionValue,
    predicate: &dyn ValueFunction,
    bound: Option<CardinalityBound>,
    meter: &mut Meter,
) -> Result<Outcome<Value>, IllTyped> {
    if predicate.parameter_type() != source.element_type() {
        return ill_typed(IllTypedCause::FunctionParameterType);
    }
    if predicate.result_type() != &ValueType::Boolean {
        return ill_typed(IllTypedCause::PredicateNotBoolean);
    }
    let mut surviving = Vec::new();
    for element in source.elements() {
        match call(predicate.apply(element, meter), &ValueType::Boolean) {
            Ok(Value::Boolean(true)) => surviving.push(element),
            Ok(_) => {}
            Err(stop) => return Ok(stop),
        }
    }
    Ok(build(
        source.kind(),
        source.element_type().clone(),
        bound,
        surviving.len(),
        surviving,
    ))
}

/// `flatten`: combine a collection of collections into the outer kind,
/// traversing outer then inner occurrence order: a sequence concatenates, a
/// set unions, a bag sums multiplicities and an ordered set keeps first
/// occurrences. No implicit deduplication occurs for a sequence or bag.
// SPEC-GAP(119-12): FR-145 "defines source/element kind combinations" without
// listing them. An ordered outer kind over an unordered inner kind has no
// semantic inner order to concatenate, so that combination is refused as
// `UnsupportedFlattenSource`; every other combination is admitted.
pub fn flatten(
    source: &CollectionValue,
    bound: Option<CardinalityBound>,
) -> Result<Outcome<Value>, IllTyped> {
    let ValueType::Collection(inner_kind, inner_type) = source.element_type() else {
        return ill_typed(IllTypedCause::UnsupportedFlattenSource);
    };
    if source.kind().is_ordered() && !inner_kind.is_ordered() {
        return ill_typed(IllTypedCause::UnsupportedFlattenSource);
    }
    // Every element is a collection of `inner_type` by the element-type
    // invariant of `CollectionValue`.
    let inner = source
        .elements()
        .iter()
        .filter_map(|element| match element {
            Value::Collection(inner) => Some(inner.elements()),
            Value::Boolean(_)
            | Value::Integer(_)
            | Value::Rational(_)
            | Value::Decimal(_)
            | Value::Quantity(_)
            | Value::Text(_)
            | Value::Enum(_)
            | Value::Option(_)
            | Value::Composite(_)
            | Value::Reference(_) => None,
        });
    let count = inner
        .clone()
        .map(<[Value]>::len)
        .fold(0_usize, usize::saturating_add);
    Ok(build(
        source.kind(),
        (**inner_type).clone(),
        bound,
        count,
        inner.flatten(),
    ))
}

/// `count`: the occurrences equal to `value` under FR-149 equality: matching
/// occurrences for a sequence, the multiplicity for a bag, and zero or one for
/// a set or ordered set.
// SPEC-GAP(119-17): FR-145 does not say whether `count` takes a value or a
// predicate; the matrix wording ("return zero or one for a value") selects a
// value.
pub fn count(source: &CollectionValue, value: &Value) -> Result<Integer, IllTyped> {
    if !source.element_type().admits(value) {
        return ill_typed(IllTypedCause::FunctionParameterType);
    }
    let matching = source
        .elements()
        .iter()
        .filter(|element| decide_equal(element, value))
        .count();
    Ok(Integer::from(matching))
}

fn check_unordered_properties(
    kind: CollectionKind,
    properties: AlgebraicProperties,
) -> Result<(), IllTyped> {
    if !kind.is_ordered() && !(properties.commutative && properties.associative) {
        return ill_typed(IllTypedCause::UnorderedFoldRequiresCommutativeAssociative);
    }
    Ok(())
}

fn fold_from(
    accumulator: Value,
    occurrences: &[Value],
    function: &dyn FoldFunction,
    meter: &mut Meter,
) -> Outcome<Value> {
    let mut accumulator = accumulator;
    for element in occurrences {
        accumulator = match call(
            function.apply(&accumulator, element, meter),
            function.accumulator_type(),
        ) {
            Ok(value) => value,
            Err(stop) => return stop,
        };
    }
    Outcome::Completed(accumulator)
}

/// `fold`: combine every retained occurrence, once each, into `identity` in
/// occurrence order. An empty collection returns exactly `identity`. A set or
/// bag requires a commutative and associative function.
pub fn fold(
    source: &CollectionValue,
    function: &dyn FoldFunction,
    identity: Value,
    meter: &mut Meter,
) -> Result<Outcome<Value>, IllTyped> {
    if function.element_type() != source.element_type() {
        return ill_typed(IllTypedCause::FunctionParameterType);
    }
    if !function.accumulator_type().admits(&identity) {
        return ill_typed(IllTypedCause::FoldIdentityType);
    }
    check_unordered_properties(source.kind(), function.properties())?;
    Ok(fold_from(identity, source.elements(), function, meter))
}

/// `reduce`: fold from the first retained occurrence with no identity. A set or
/// bag requires a commutative and associative function; an empty collection
/// has the disposition of [`empty_reduction`].
pub fn reduce(
    source: &CollectionValue,
    function: &dyn FoldFunction,
    meter: &mut Meter,
) -> Result<Outcome<Value>, IllTyped> {
    if function.element_type() != source.element_type()
        || function.accumulator_type() != source.element_type()
    {
        return ill_typed(IllTypedCause::FunctionParameterType);
    }
    check_unordered_properties(source.kind(), function.properties())?;
    let Some((first, rest)) = source.elements().split_first() else {
        return Ok(empty_reduction(source.kind()));
    };
    Ok(fold_from(first.clone(), rest, function, meter))
}

/// The disposition of reducing an empty collection of `kind`.
// SPEC-GAP(119-10): FR-145's matrix makes an empty sequence or set reduction
// undefined and an empty bag or ordered-set reduction refused, its prose makes
// every empty reduction "a located undefined outcome", and FR-145-AC-2 says
// "a reduction without one refuses". The matrix is followed. This is the only
// place that decides it.
fn empty_reduction(kind: CollectionKind) -> Outcome<Value> {
    match kind {
        CollectionKind::Sequence | CollectionKind::Set => {
            Outcome::Undefined(Undefined::EmptyReduction)
        }
        CollectionKind::Bag | CollectionKind::OrderedSet => {
            Outcome::Refused(Refusal::EmptyReduction)
        }
    }
}

/// An explicit conversion of `source` to `target` within `bound`. Every
/// property the conversion discards must be in `accepted`, and the exact loss
/// is recorded. Converting an unordered source to an ordered kind orders it by
/// ascending total element key, and refuses when the element type has none.
pub fn convert(
    source: &CollectionValue,
    target: CollectionKind,
    accepted: CollectionLoss,
    bound: Option<CardinalityBound>,
) -> Result<Outcome<CollectionConversion>, IllTyped> {
    let loss = CollectionLoss::of_conversion(source.kind(), target);
    if !loss.is_within(accepted) {
        return ill_typed(IllTypedCause::UnacceptedCollectionLoss);
    }
    let element_type = source.element_type().clone();
    let count = source.elements().len();
    let built = if target.is_ordered() && !source.kind().is_ordered() {
        let keyed = keyed_ascending(source.element_type(), source.elements())
            .or_else(|_| ill_typed(IllTypedCause::NoTotalElementKey))?;
        build(
            target,
            element_type,
            bound,
            count,
            keyed.into_iter().map(|(_, value)| value),
        )
    } else {
        build(target, element_type, bound, count, source.elements())
    };
    Ok(match built {
        Outcome::Completed(value) => Outcome::Completed(CollectionConversion { value, loss }),
        Outcome::Undefined(reason) => Outcome::Undefined(reason),
        Outcome::Refused(reason) => Outcome::Refused(reason),
        Outcome::Incomplete(record) => Outcome::Incomplete(record),
    })
}
