// SPDX-License-Identifier: AGPL-3.0-or-later
//! The collection operations the S6a evaluator and `qsl-jit` share: the
//! per-element charges, `size`, `contains`, `flatten`, the conversion between
//! collection kinds, the one-binder queries and folds, `seq`, and the exact
//! running sum.
//!
//! The evaluator drives the iteration from its explicit task stack and the
//! JIT from generated code, but both advance the same state with the same
//! functions, so the charges, the early exits and the stops come from one
//! body of code. A host decides only where each element is evaluated: it
//! calls [`Iteration::absorb`] with the previous body result and
//! [`Iteration::next`] for the next element, and binds the element itself.

use std::cmp::Ordering;
use std::sync::Arc;

use quire_checked::node::Visit;
use quire_exact::{
    compare_keys, evaluate_decimal, evaluate_integer_arithmetic, evaluate_rational_arithmetic,
    form, form_grouped, member_equal, Charge, ChargePoint, CollectionKind, CollectionType,
    CollectionValue, Decimal, DecimalOperation, DecimalType, Integer, IntegerArithmetic,
    IntegerInterval, LimitKind, Meter, Rational, RationalArithmetic, Refusal, Undefined, Value,
    ValueType,
};
use quire_semantic_value::stop::{outcome_into_stop, Stop};

/// Why a collection operation did not produce a value.
#[derive(Debug)]
pub enum Halt {
    /// The kernel's own stop.
    Stop(Stop),
    /// The first summand of a `sum` is outside the total type: an
    /// `Undefined(SumOutOfDomain)` the host locates at the summand's body.
    SumSeedOutOfDomain,
    /// An `only` over a population that does not hold exactly one object
    /// (FR-139): the host reports it as a family result with the population.
    OnlyNotOne {
        /// How many objects the population holds.
        found: u64,
    },
    /// A broken invariant, by identifier.
    Fault(&'static str),
}

impl From<Stop> for Halt {
    fn from(stop: Stop) -> Self {
        Self::Stop(stop)
    }
}

/// A visit: one `collection.visit` work unit.
///
/// # Errors
/// The meter's denial.
pub fn charge_visit(meter: &mut Meter) -> Result<(), Stop> {
    Ok(meter.charge(Charge::new(ChargePoint::CollectionVisit))?)
}

/// One `collection.element` before a literal element.
///
/// # Errors
/// The meter's denial.
pub fn charge_element(meter: &mut Meter) -> Result<(), Stop> {
    Ok(meter.charge(Charge::new(ChargePoint::CollectionElement))?)
}

/// A scalar retain: `collection.result-retain` with one result unit.
///
/// # Errors
/// The meter's denial.
pub fn retain_scalar(value: Value, meter: &mut Meter) -> Result<Value, Stop> {
    meter.charge(Charge::new(ChargePoint::CollectionResultRetain).results(1))?;
    Ok(value)
}

/// An accumulator retain: `collection.result-retain` with `occ(result)` value
/// occurrences and result units.
///
/// # Errors
/// The meter's denial.
pub fn retain_accumulator(value: Value, meter: &mut Meter) -> Result<Value, Stop> {
    let occ = value.occ();
    meter.charge(
        Charge::new(ChargePoint::CollectionResultRetain)
            .exact_size(LimitKind::ValueOccurrences, occ.clone())
            .exact_results(occ),
    )?;
    Ok(value)
}

/// `size(collection)`.
///
/// # Errors
/// The meter's denial.
pub fn size(collection: &CollectionValue, meter: &mut Meter) -> Result<Value, Stop> {
    retain_scalar(
        Value::Integer(Integer::from(collection.elements().len())),
        meter,
    )
}

/// `contains(collection, item)`: a bag compares its distinct members only.
///
/// # Errors
/// The kernel's stop.
pub fn contains(
    item: &Value,
    collection: &CollectionValue,
    meter: &mut Meter,
) -> Result<Value, Stop> {
    let bag = collection.collection_type().kind() == CollectionKind::Bag;
    let mut found = false;
    let mut previous: Option<&Value> = None;
    for member in collection.elements() {
        let repeated = bag
            && previous
                .is_some_and(|previous| compare_keys(previous, member) == Some(Ordering::Equal));
        previous = Some(member);
        if repeated {
            continue;
        }
        if outcome_into_stop(member_equal(item, member, meter))? {
            found = true;
            break;
        }
    }
    retain_scalar(Value::Boolean(found), meter)
}

/// `flatten(outer)` into the collection type `result_type`.
///
/// # Errors
/// The kernel's stop, or a fault for a result that is no collection type or
/// an element that is no collection.
pub fn flatten(
    result_type: &ValueType,
    outer: &CollectionValue,
    meter: &mut Meter,
) -> Result<Value, Halt> {
    let ValueType::Collection(result_type) = result_type else {
        return Err(Halt::Fault("flatten-collection-result-expected"));
    };
    let mut produced = Vec::new();
    for inner in outer.elements() {
        charge_visit(meter)?;
        let Value::Collection(inner) = inner else {
            return Err(Halt::Fault("flatten-inner-collection-expected"));
        };
        for element in inner.elements() {
            charge_visit(meter)?;
            produced.push(element.clone());
        }
    }
    Ok(outcome_into_stop(form(result_type, produced, meter))?)
}

/// The conversion of `source` to the collection type `target`: the value and,
/// when `keep` is set, the elements the conversion kept.
///
/// # Errors
/// The kernel's stop.
pub fn convert(
    target: &CollectionType,
    source: &CollectionValue,
    keep: bool,
    meter: &mut Meter,
) -> Result<(Value, Option<Vec<Value>>), Stop> {
    let source_kind = source.collection_type().kind();
    let distinct = source_kind == CollectionKind::Bag && target.kind().is_unique();
    let mut produced: Vec<Value> = Vec::with_capacity(source.elements().len());
    for element in source.elements() {
        let repeated = distinct
            && produced
                .last()
                .is_some_and(|previous| compare_keys(previous, element) == Some(Ordering::Equal));
        if repeated {
            continue;
        }
        charge_visit(meter)?;
        produced.push(element.clone());
    }
    // Only a traced run reads the kept elements again.
    let kept = keep.then(|| produced.clone());
    let value =
        if source_kind == CollectionKind::Sequence && target.kind() != CollectionKind::Sequence {
            outcome_into_stop(form(target, produced, meter))?
        } else {
            outcome_into_stop(form_grouped(target, produced, meter))?
        };
    Ok((value, kept))
}

/// The declared `Int` domain of a `sum`; `None` for `Integer`.
fn sum_domain(value_type: &ValueType) -> Option<&IntegerInterval> {
    match value_type {
        ValueType::Int(domain) => Some(domain),
        _ => None,
    }
}

/// Whether a `sum` over `value_type` is one of the exact numeric families
/// ([`sum_exact`] runs it).
#[must_use]
pub fn is_exact_sum(value_type: &ValueType) -> bool {
    matches!(
        value_type,
        ValueType::Integer | ValueType::Int(_) | ValueType::Rational(_) | ValueType::Decimal(_)
    )
}

/// The empty sum of an exact numeric family: its additive zero (FR-145).
#[must_use]
pub fn exact_zero(value_type: &ValueType) -> Option<Value> {
    Some(match value_type {
        ValueType::Integer | ValueType::Int(_) => Value::Integer(Integer::zero()),
        ValueType::Rational(_) => Value::Rational(Rational::from_integer(Integer::zero())),
        ValueType::Decimal(_) => Value::Decimal(Decimal::new(Integer::zero(), 0)),
        _ => return None,
    })
}

/// One step of an exact `sum<N>` (FR-145): the first summand seeds the
/// running total with no addition, and each later one is added by the family
/// of `N`. `None` when `value_type` is not an exact family. A seed or total
/// outside `N`'s domain is the `Undefined` outcome, never the family's
/// membership refusal; a rounded decimal sum also returns its loss.
#[must_use]
pub fn sum_exact(
    value_type: &ValueType,
    total: Option<Value>,
    summand: Value,
    meter: &mut Meter,
) -> Option<Result<(Value, Option<quire_exact::DecimalLoss>), Halt>> {
    if !is_exact_sum(value_type) {
        return None;
    }
    let out_of_domain = || Halt::Stop(Stop::Undefined(Undefined::SumOutOfDomain));
    let Some(total) = total else {
        return Some(if value_type.admits(&summand) {
            Ok((summand, None))
        } else {
            Err(Halt::SumSeedOutOfDomain)
        });
    };
    Some(match (value_type, total, summand) {
        (
            ValueType::Integer | ValueType::Int(_),
            Value::Integer(total),
            Value::Integer(summand),
        ) => match outcome_into_stop(evaluate_integer_arithmetic(
            IntegerArithmetic::Add(&total, &summand),
            sum_domain(value_type),
            meter,
        )) {
            Err(Stop::Refused(Refusal::IntegerOutOfDomain { .. })) => Err(out_of_domain()),
            Err(stop) => Err(stop.into()),
            Ok(sum) => Ok((Value::Integer(sum), None)),
        },
        (ValueType::Rational(domain), Value::Rational(total), Value::Rational(summand)) => {
            match outcome_into_stop(evaluate_rational_arithmetic(
                RationalArithmetic::Add(&total, &summand),
                Some(domain),
                meter,
            )) {
                Err(Stop::Refused(Refusal::RationalOutOfDomain { .. })) => Err(out_of_domain()),
                Err(stop) => Err(stop.into()),
                Ok(sum) => Ok((Value::Rational(sum), None)),
            }
        }
        (ValueType::Decimal(target), Value::Decimal(total), Value::Decimal(summand)) => {
            match outcome_into_stop(evaluate_decimal(
                DecimalOperation::Add(&total, &summand),
                target,
                meter,
            )) {
                Err(Stop::Refused(Refusal::DecimalOutOfDomain { .. })) => Err(out_of_domain()),
                Err(stop) => Err(stop.into()),
                Ok(result) => Ok((
                    Value::Decimal(result.value().clone()),
                    result.loss().cloned(),
                )),
            }
        }
        _ => Err(Halt::Fault("sum-accumulator-value-invalid")),
    })
}

/// The kind of one-binder iteration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Shape {
    /// A query: `map`, `filter`, `forall`, `exists`, `count`, `sum` or `only`.
    Query(Visit),
    /// A fold, or a reduce when it has no identity.
    Fold,
}

/// What an iteration is over, from the checked node.
#[derive(Clone, Copy, Debug)]
pub struct Spec<'t> {
    /// The query or fold.
    pub shape: Shape,
    /// The node's value type.
    pub value_type: &'t ValueType,
    /// An `only` over a `where` filter: the one object that passed the filter
    /// was already visited (FR-139), so no visit is charged.
    pub only_over_filter: bool,
}

/// The state of a one-binder iteration in progress.
#[derive(Debug)]
pub struct Iteration {
    /// The collection iterated.
    pub source: Arc<CollectionValue>,
    /// The position of the next element.
    pub next: usize,
    /// Whether an element's body result is awaited.
    pub awaiting: bool,
    /// The kept results (`map`, `filter`, `only`).
    pub results: Vec<Value>,
    /// For a `map` or `filter`, the source position of each of `results`.
    pub sources: Vec<usize>,
    /// The running total or accumulator.
    pub accumulator: Option<Value>,
    /// The count of admitted occurrences.
    pub count: Integer,
}

/// What an absorbed body result leaves to do.
#[derive(Debug)]
pub enum Absorbed {
    /// Go on to the next element, having absorbed the result.
    Continue,
    /// A `sum` summand awaits the host's addition: it passes the running total
    /// and the summand to its own sum, stores the new total in
    /// [`Iteration::accumulator`], and goes on.
    ///
    /// The payload is the running total (absent before the first summand) and
    /// the body's result; it is boxed to keep the common variants small.
    Summand(Box<(Option<Value>, Value)>),
    /// A `forall` or `exists` is decided: it stops with `found`.
    Exit {
        /// The decided value.
        found: bool,
    },
}

/// What follows the elements visited so far.
#[derive(Debug)]
pub enum Flow {
    /// Evaluate the body on this element.
    Visit {
        /// The element.
        element: Value,
        /// Its source position.
        position: usize,
    },
    /// Every element is visited: [`Iteration::finish`] gives the result.
    Exhausted,
}

/// A finished query or fold.
#[derive(Debug)]
pub struct Finished {
    /// The result.
    pub value: Value,
    /// For a `map` or `filter` finished with `keep` set, the source position
    /// and value of each result.
    pub produced: Option<(Vec<usize>, Vec<Value>)>,
}

impl Iteration {
    /// Start `spec` over `source`: an `only` checks its population holds one
    /// object, and a reduce takes the first element as its accumulator,
    /// charging its visit.
    ///
    /// # Errors
    /// [`Halt::OnlyNotOne`], the undefined empty reduction, or the meter's
    /// denial.
    pub fn start(
        spec: &Spec<'_>,
        source: Arc<CollectionValue>,
        identity: Option<Value>,
        reduce: bool,
        meter: &mut Meter,
    ) -> Result<Self, Halt> {
        if spec.shape == Shape::Query(Visit::Only) {
            // FR-139: `only` is `e` for the one object satisfying the
            // condition, and undefined for none or several, before `e`
            // reads any object.
            let found = u64::try_from(source.elements().len()).unwrap_or(u64::MAX);
            if found != 1 {
                return Err(Halt::OnlyNotOne { found });
            }
        }
        let mut iteration = Self {
            source,
            next: 0,
            awaiting: false,
            results: Vec::new(),
            sources: Vec::new(),
            accumulator: identity,
            count: Integer::zero(),
        };
        if reduce {
            let first = iteration
                .source
                .elements()
                .first()
                .cloned()
                .ok_or(Stop::Undefined(Undefined::EmptyReduction))?;
            charge_visit(meter)?;
            iteration.accumulator = Some(first);
            iteration.next = 1;
        }
        Ok(iteration)
    }

    /// Absorb the result of the body on the element visited last.
    ///
    /// # Errors
    /// A fault for a result of the wrong kind.
    pub fn absorb(&mut self, spec: &Spec<'_>, result: Value) -> Result<Absorbed, Halt> {
        self.awaiting = false;
        let element = self
            .next
            .checked_sub(1)
            .and_then(|index| self.source.elements().get(index))
            .cloned()
            .ok_or(Halt::Fault("iteration-source-position-invalid"))?;
        match spec.shape {
            Shape::Query(visit) => match (visit, result) {
                (Visit::Map, value) => {
                    self.results.push(value);
                    self.sources.push(self.next - 1);
                    Ok(Absorbed::Continue)
                }
                (Visit::Filter, Value::Boolean(kept)) => {
                    if kept {
                        self.results.push(element);
                        self.sources.push(self.next - 1);
                    }
                    Ok(Absorbed::Continue)
                }
                (Visit::Forall, Value::Boolean(holds)) => Ok(if holds {
                    Absorbed::Continue
                } else {
                    Absorbed::Exit { found: false }
                }),
                (Visit::Exists, Value::Boolean(holds)) => Ok(if holds {
                    Absorbed::Exit { found: true }
                } else {
                    Absorbed::Continue
                }),
                (Visit::Sum, summand) => Ok(Absorbed::Summand(Box::new((
                    self.accumulator.take(),
                    summand,
                )))),
                (Visit::Count, Value::Boolean(holds)) => {
                    if holds {
                        self.count = self.count.add(&Integer::one());
                    }
                    Ok(Absorbed::Continue)
                }
                (Visit::Only, value) => {
                    self.results.push(value);
                    Ok(Absorbed::Continue)
                }
                _ => Err(Halt::Fault("query-body-value-invalid")),
            },
            Shape::Fold => {
                self.accumulator = Some(result);
                Ok(Absorbed::Continue)
            }
        }
    }

    /// The next element to visit, having charged its visit, or that none is
    /// left.
    ///
    /// # Errors
    /// The meter's denial.
    pub fn next(&mut self, spec: &Spec<'_>, meter: &mut Meter) -> Result<Flow, Halt> {
        let Some(element) = self.source.elements().get(self.next).cloned() else {
            return Ok(Flow::Exhausted);
        };
        if !spec.only_over_filter {
            charge_visit(meter)?;
        }
        self.next = self.next.saturating_add(1);
        self.awaiting = true;
        Ok(Flow::Visit {
            element,
            position: self.next.saturating_sub(1),
        })
    }

    /// The result of the iteration once every element is visited.
    ///
    /// # Errors
    /// The kernel's stop, or a fault for a node of the wrong type.
    pub fn finish(self, spec: &Spec<'_>, keep: bool, meter: &mut Meter) -> Result<Finished, Halt> {
        let plain = |value| Finished {
            value,
            produced: None,
        };
        match spec.shape {
            Shape::Query(visit) => Ok(match visit {
                Visit::Map | Visit::Filter => {
                    let ValueType::Collection(result_type) = spec.value_type else {
                        return Err(Halt::Fault("query-collection-result-expected"));
                    };
                    let produced = keep.then(|| (self.sources, self.results.clone()));
                    let value = if visit == Visit::Map {
                        outcome_into_stop(form(result_type, self.results, meter))?
                    } else {
                        outcome_into_stop(form_grouped(result_type, self.results, meter))?
                    };
                    Finished { value, produced }
                }
                Visit::Forall => plain(retain_scalar(Value::Boolean(true), meter)?),
                Visit::Exists => plain(retain_scalar(Value::Boolean(false), meter)?),
                Visit::Sum => {
                    // Every non-empty total was decided as a running total;
                    // only the empty sum's seed `0` is still undecided
                    // (FR-145).
                    let total = match self.accumulator {
                        None => {
                            let zero = exact_zero(spec.value_type)
                                .ok_or(Halt::Fault("sum-family-invalid"))?;
                            if !spec.value_type.admits(&zero) {
                                return Err(Stop::Undefined(Undefined::SumOutOfDomain).into());
                            }
                            zero
                        }
                        Some(total) => total,
                    };
                    plain(retain_scalar(total, meter)?)
                }
                Visit::Only => plain(
                    self.results
                        .into_iter()
                        .next()
                        .ok_or(Halt::Fault("only-element-missing"))?,
                ),
                Visit::Count => {
                    if let ValueType::Int(domain) = spec.value_type {
                        if !domain.contains(&self.count) {
                            return Err(Stop::Refused(Refusal::IntegerOutOfDomain {
                                target: Box::new(domain.clone()),
                            })
                            .into());
                        }
                    }
                    plain(retain_scalar(Value::Integer(self.count), meter)?)
                }
            }),
            Shape::Fold => {
                let accumulator = self
                    .accumulator
                    .ok_or(Halt::Fault("fold-accumulator-missing"))?;
                Ok(plain(retain_accumulator(accumulator, meter)?))
            }
        }
    }
}

/// A `seq(i in low .. high: e)` in progress (FR-139).
#[derive(Debug)]
pub struct Sequence {
    /// The integer the next element binds.
    pub next: Integer,
    /// The last integer.
    pub high: Integer,
    /// Whether an element's body result is awaited.
    pub awaiting: bool,
    /// The results so far.
    pub results: Vec<Value>,
}

impl Sequence {
    /// A sequence over `low .. high`.
    #[must_use]
    pub fn new(low: Integer, high: Integer) -> Self {
        Self {
            next: low,
            high,
            awaiting: false,
            results: Vec::new(),
        }
    }

    /// Absorb the awaited result, if any, then yield the next integer to
    /// bind, having charged its visit, or `None` when the sequence is done
    /// (empty when `high < low`).
    ///
    /// # Errors
    /// The meter's denial.
    pub fn step(
        &mut self,
        result: Option<Value>,
        meter: &mut Meter,
    ) -> Result<Option<Integer>, Stop> {
        if self.awaiting {
            self.awaiting = false;
            if let Some(element) = result {
                self.results.push(element);
            }
        }
        if self.next > self.high {
            return Ok(None);
        }
        charge_visit(meter)?;
        let bound = self.next.clone();
        self.next = self.next.add(&Integer::one());
        self.awaiting = true;
        Ok(Some(bound))
    }

    /// The collection of the results.
    ///
    /// # Errors
    /// The kernel's stop, or a fault for a node of the wrong type.
    pub fn finish(self, value_type: &ValueType, meter: &mut Meter) -> Result<Value, Halt> {
        let ValueType::Collection(result_type) = value_type else {
            return Err(Halt::Fault("seq-collection-result-expected"));
        };
        Ok(outcome_into_stop(form(result_type, self.results, meter))?)
    }
}

/// The decimal type of a `sum`, for the hosts' loss record.
#[must_use]
pub fn sum_target(value_type: &ValueType) -> Option<&DecimalType> {
    match value_type {
        ValueType::Decimal(target) => Some(target),
        _ => None,
    }
}
