// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-337: the Boolean condition of each `holds` atom of a checked temporal
//! clause, typed so S4 can lower it as FR-093 lowers a Boolean expression.
//!
//! A condition reads the clause's `over` parameter and its captures: it is
//! typed as a state clause types its body (FR-104), with the `over`
//! parameter bound at slot 0, the trigger binder's slot (when the activation
//! is `on each`) bound under a name no source can write, the captures at the
//! following slots and no `self`, so a field read through the `over` object
//! is a model read of the current state. A capture's initializer is typed
//! the same way, with the trigger binder named, the earlier captures bound
//! and the declared type as its goal. Each condition is located at its own
//! declaration site (`lowering::temporal_site`), so a refusal inside it, and
//! every occurrence its lowered nodes record, resolves to the condition's
//! own source.

use std::collections::BTreeMap;

use qsl_forms::{ActivationForm, ClauseKind, StateClauseKind, TemporalNodeForm};
use quire_exact::ValueType;
use quire_semantic_value::location::ExpressionLocation as Location;

use super::CheckedTemporalClause;
use crate::check::check::{bind_parameters, Scope, StateContext, Typer};
use crate::check::facts::Definedness;
use crate::check::ir::CheckedBody;
use crate::check::lowering::{temporal_location, AggregatePart, TemporalSite};
use crate::check::observation::Observations;
use crate::check::refusal::{CheckCause, CheckRefusal};
use crate::check::state_clause::ClauseDeclarations;
use crate::family::CheckContext;
use crate::model::clause_observation::Observation;

/// A slot no local carries: a condition has no `self`.
const NO_SLOT: usize = usize::MAX;

/// One `holds` atom's condition, typed (FR-337 "Inputs").
#[derive(Debug)]
pub(crate) struct TypedCondition {
    /// The checked Boolean condition.
    pub(crate) body: CheckedBody,
    /// The evaluation slot count: the `over` parameter's slot and every
    /// local the condition binds.
    pub(crate) slots: usize,
    /// The name each slot of the condition was bound under.
    pub(crate) slot_names: Vec<String>,
    /// The compound units typing formed, which lowering keys.
    pub(crate) formed_units: quire_semantic_value::quantity::UnitTable,
}

/// The slot name of the trigger binder in a `holds` condition: the binder is
/// not in scope there (FR-337-AC-7), but its slot is counted (FR-337
/// "Captures at S3").
const HIDDEN_TRIGGER: &str = " trigger";

/// The typed conditions of one clause: its `holds` atoms', its captures'
/// initializers and, for an `on each ... when (g)` activation, the guard.
#[derive(Debug, Default)]
pub(crate) struct TypedConditions {
    /// Each `holds` atom's condition, by its formula node's index.
    pub(crate) holds: BTreeMap<usize, TypedCondition>,
    /// Each window aggregate atom's typed expressions, by its formula
    /// node's index.
    pub(crate) aggregates: BTreeMap<usize, TypedAggregate>,
    /// The `when` guard, typed over the `over` parameter at slot 0 and the
    /// trigger binder at slot 1.
    pub(crate) guard: Option<TypedCondition>,
    /// Each capture's initializer in source order, typed against the
    /// capture's declared type over the `over` parameter, the trigger binder
    /// and the earlier captures.
    pub(crate) captures: Vec<TypedCondition>,
}

impl TypedConditions {
    /// Every condition, the guard first, then the captures.
    pub(crate) fn all(&self) -> impl Iterator<Item = &TypedCondition> {
        self.guard
            .iter()
            .chain(&self.captures)
            .chain(self.holds.values())
            .chain(self.aggregates.values().flat_map(TypedAggregate::all))
    }

    /// Every condition, mutably.
    pub(crate) fn all_mut(&mut self) -> impl Iterator<Item = &mut TypedCondition> {
        self.guard
            .iter_mut()
            .chain(&mut self.captures)
            .chain(self.holds.values_mut())
            .chain(
                self.aggregates
                    .values_mut()
                    .flat_map(TypedAggregate::all_mut),
            )
    }
}

/// The typed expressions of one window aggregate atom (FR-194, FR-337-AC-15).
#[derive(Debug)]
pub(crate) struct TypedAggregate {
    /// The `of` value expression of a `sum`, `min`, `max` or `quantile`.
    pub(crate) value: Option<TypedCondition>,
    /// The `where holds(...)` filter.
    pub(crate) filter: Option<TypedCondition>,
    /// The `fraction holds(P)` predicate.
    pub(crate) predicate: Option<TypedCondition>,
    /// The threshold, converted to the value's unit when it is a quantity
    /// of another unit of the value's dimension.
    pub(crate) threshold: TypedCondition,
}

impl TypedAggregate {
    /// Every typed expression in source order: value, filter, predicate,
    /// threshold.
    fn all(&self) -> impl Iterator<Item = &TypedCondition> {
        self.value
            .iter()
            .chain(&self.filter)
            .chain(&self.predicate)
            .chain(std::iter::once(&self.threshold))
    }

    /// Every typed expression, mutably.
    fn all_mut(&mut self) -> impl Iterator<Item = &mut TypedCondition> {
        self.value
            .iter_mut()
            .chain(&mut self.filter)
            .chain(&mut self.predicate)
            .chain(std::iter::once(&mut self.threshold))
    }
}

/// The typed condition of every `holds` atom of the clause at `clause`
/// (its index among the unit's temporal clauses), by its formula node's
/// index, typed over the clause's `over` parameter. `input.nodes_used` is
/// the package's running node total before the clause, and the total after
/// it is returned.
///
/// The refusals are in report order: an aggregate value refused as a carried
/// quantity (FR-194-AC-12) is followed by the refusal of its threshold.
pub(crate) fn check_conditions(
    clause: usize,
    form: &qsl_forms::TemporalClauseForm,
    checked: &CheckedTemporalClause,
    input: &ClauseDeclarations<'_>,
    cx: &mut CheckContext<'_, Scope>,
) -> Result<(TypedConditions, u64), Vec<CheckRefusal>> {
    let mut refused = Vec::new();
    match check_conditions_into(clause, form, checked, input, cx, &mut refused) {
        Ok(typed) if refused.is_empty() => Ok(typed),
        Ok(_) => Err(refused),
        Err(first) => {
            refused.push(first);
            Err(refused)
        }
    }
}

/// [`check_conditions`], pushing a refusal that precedes the one it returns
/// onto `refused`.
fn check_conditions_into(
    clause: usize,
    form: &qsl_forms::TemporalClauseForm,
    checked: &CheckedTemporalClause,
    input: &ClauseDeclarations<'_>,
    cx: &mut CheckContext<'_, Scope>,
    refused: &mut Vec<CheckRefusal>,
) -> Result<(TypedConditions, u64), CheckRefusal> {
    let mut nodes = input.nodes_used;
    let mut conditions = TypedConditions::default();
    let named = crate::check::check::NamedScope::of(input.models, input.scope);
    // FR-337: the activation precedes the body, so its guard is typed first
    // and a defect in both refuses at the guard. The guard reads the `over`
    // parameter at slot 0 and the trigger binder at slot 1.
    let trigger = checked
        .trigger()
        .map(|(name, value_type)| (name.to_owned(), value_type.clone()));
    let guard = match (&form.activation, &trigger) {
        (
            ActivationForm::Each {
                when: Some(when), ..
            },
            Some(trigger),
        ) => {
            let bound = [checked.over().clone(), trigger.clone()];
            Some((when, bound, temporal_location(clause, TemporalSite::Guard)))
        }
        _ => None,
    };
    if let Some((when, bound, location)) = guard {
        conditions.guard = Some(type_condition(
            Goal::Boolean,
            when,
            &bound,
            &location,
            input,
            &named,
            &mut nodes,
            cx,
        )?);
    }
    // Then the captures in source order, each over the binders before it.
    let mut visible = vec![checked.over().clone()];
    visible.extend(trigger.clone());
    for (index, ((name, declared), written)) in checked.captures().zip(&form.captures).enumerate() {
        let location = temporal_location(clause, TemporalSite::CaptureValue(index));
        let typed = type_condition(
            Goal::Declared(declared),
            &written.value,
            &visible,
            &location,
            input,
            &named,
            &mut nodes,
            cx,
        )?;
        conditions.captures.push(typed);
        visible.push((name.to_owned(), declared.clone()));
    }
    // A `holds` condition reads `over` and the captures: the trigger binder
    // keeps its slot under a name no source writes.
    let mut bound = vec![checked.over().clone()];
    if let Some((_, value_type)) = &trigger {
        bound.push((HIDDEN_TRIGGER.to_owned(), value_type.clone()));
    }
    bound.extend(visible.into_iter().skip(1 + usize::from(trigger.is_some())));
    for (id, node) in form.formula.iter() {
        match node {
            TemporalNodeForm::Holds { condition, .. } => {
                let location = temporal_location(clause, TemporalSite::Condition(id.index()));
                let typed = type_condition(
                    Goal::Boolean,
                    condition,
                    &bound,
                    &location,
                    input,
                    &named,
                    &mut nodes,
                    cx,
                )?;
                conditions.holds.insert(id.index(), typed);
            }
            TemporalNodeForm::AggregateAtom(atom) => {
                let at =
                    |part| temporal_location(clause, TemporalSite::Aggregate(id.index(), part));
                let mut typed = |goal, expression, part| {
                    type_condition(
                        goal,
                        expression,
                        &bound,
                        &at(part),
                        input,
                        &named,
                        &mut nodes,
                        cx,
                    )
                };
                let value = atom
                    .term
                    .value
                    .as_ref()
                    .map(|value| typed(Goal::Any, value, AggregatePart::Value))
                    .transpose()?;
                // QSpec FR-370 "Window aggregate atom": a value is of
                // family `integer`, `rational` or `quantity`. A carried
                // quantity is none of them (QSL FR-194-AC-12): the value is
                // refused, and its threshold is still checked against the
                // carried quantity's `Quantity<u>`, so a refusal at the
                // threshold follows the one at the value.
                let value_refusal = value
                    .as_ref()
                    .is_some_and(|value| family(value.body.root().value_type()).is_none())
                    .then(|| CheckRefusal {
                        location: at(AggregatePart::Value),
                        cause: CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch),
                    });
                let aggregate_value_type =
                    value
                        .as_ref()
                        .map(|value| match value.body.root().value_type() {
                            ValueType::CarriedQuantity(carried) => {
                                ValueType::Quantity(carried.unit().clone())
                            }
                            other => other.clone(),
                        });
                if let Some(refusal) = value_refusal {
                    if aggregate_value_type
                        .as_ref()
                        .is_none_or(|value_type| family(value_type).is_none())
                    {
                        return Err(refusal);
                    }
                    refused.push(refusal);
                }
                let filter = atom
                    .term
                    .filter
                    .as_ref()
                    .map(|filter| typed(Goal::Boolean, filter, AggregatePart::Filter))
                    .transpose()?;
                let predicate = match &atom.term.kind {
                    qsl_forms::AggregateKindForm::Fraction(predicate) => {
                        Some(typed(Goal::Boolean, predicate, AggregatePart::Predicate)?)
                    }
                    qsl_forms::AggregateKindForm::Count
                    | qsl_forms::AggregateKindForm::Sum
                    | qsl_forms::AggregateKindForm::Min
                    | qsl_forms::AggregateKindForm::Max
                    | qsl_forms::AggregateKindForm::Quantile(_) => None,
                };
                // FR-194: an exact fraction `n/d` of integer literals is the
                // rational literal `rational(n, d)`, folded as the quantile
                // level is.
                let threshold = match exact_fraction(
                    &atom.threshold,
                    input.checking_limits.rational_bits(),
                    &at(AggregatePart::Threshold),
                )? {
                    Some((form, point)) => {
                        typed(Goal::Declared(&point), &form, AggregatePart::Threshold)?
                    }
                    None => typed(
                        Goal::Threshold(aggregate_value_type.as_ref()),
                        &atom.threshold,
                        AggregatePart::Threshold,
                    )?,
                };
                // QSpec FR-370: the threshold is of the aggregate's result
                // family, `integer` for `count`, `rational` for `fraction`,
                // otherwise the family of the value, and for a quantity
                // the value's unit (converted above). An integer literal
                // is written as the rational of its value where a rational
                // is required.
                let threshold_root = threshold.body.root();
                let required = match &atom.term.kind {
                    qsl_forms::AggregateKindForm::Count => Some(Family::Integer),
                    qsl_forms::AggregateKindForm::Fraction(_) => Some(Family::Rational),
                    qsl_forms::AggregateKindForm::Sum
                    | qsl_forms::AggregateKindForm::Min
                    | qsl_forms::AggregateKindForm::Max
                    | qsl_forms::AggregateKindForm::Quantile(_) => {
                        aggregate_value_type.as_ref().and_then(family)
                    }
                };
                // FR-194-AC-7, QSpec FR-370 step 5: a quantity threshold the
                // conversion above left in another unit, of another dimension
                // or not convertible exactly, is a unit mismatch; any other
                // family is a type mismatch.
                let mismatch = match (required, family(threshold_root.value_type())) {
                    (Some(required), Some(found)) if required == found => None,
                    (Some(Family::Rational), Some(Family::Integer))
                        if is_integer_literal(threshold_root) =>
                    {
                        None
                    }
                    (Some(Family::Quantity(_)), Some(Family::Quantity(_))) => {
                        Some(CheckCause::UnitMismatch)
                    }
                    _ => Some(CheckCause::IllTyped(
                        quire_exact::IllTypedCause::TypeMismatch,
                    )),
                };
                if let Some(cause) = mismatch {
                    return Err(CheckRefusal {
                        location: at(AggregatePart::Threshold),
                        cause,
                    });
                }
                conditions.aggregates.insert(
                    id.index(),
                    TypedAggregate {
                        value,
                        filter,
                        predicate,
                        threshold,
                    },
                );
            }
            TemporalNodeForm::Constant { .. } | TemporalNodeForm::Operator(_) => {}
        }
    }
    Ok((conditions, nodes))
}

/// FR-337 "`on each` activation at S3": the activation is checked before the
/// clause's body, so a guard defect refuses ahead of a body refusal. For a
/// clause with a `when` guard whose binder types resolve alike, the refusal of
/// typing the guard; `None` when the clause has no guard, its binder types
/// are refused by the clause check itself, or the guard types.
pub(crate) fn guard_refusal(
    clause: usize,
    form: &qsl_forms::TemporalClauseForm,
    input: &ClauseDeclarations<'_>,
    cx: &mut CheckContext<'_, Scope>,
) -> Option<CheckRefusal> {
    let ActivationForm::Each {
        parameter,
        when: Some(when),
    } = &form.activation
    else {
        return None;
    };
    let resolve = |type_form| {
        crate::check::type_form::resolve_form(
            &input.scope.for_declaration(&form.name.name),
            type_form,
        )
        .ok()
        .filter(|resolved| !matches!(resolved, ValueType::Population(_)))
    };
    let over = resolve(&form.over.value_type)?;
    let trigger = resolve(&parameter.value_type)?;
    if trigger != over || parameter.name.name == form.over.name.name {
        return None;
    }
    let bound = [
        (form.over.name.name.clone(), over),
        (parameter.name.name.clone(), trigger),
    ];
    let named = crate::check::check::NamedScope::of(input.models, input.scope);
    let mut nodes = input.nodes_used;
    let location = temporal_location(clause, TemporalSite::Guard);
    type_condition(
        Goal::Boolean,
        when,
        &bound,
        &location,
        input,
        &named,
        &mut nodes,
        cx,
    )
    .err()
}

/// What a condition is typed against.
#[derive(Clone, Copy)]
enum Goal<'t> {
    /// A Boolean: a `holds` condition or the guard.
    Boolean,
    /// A capture's declared type.
    Declared(&'t ValueType),
    /// Infer an aggregate value without a dimension constraint.
    Any,
    /// Infer an aggregate threshold, converted to the aggregate value's
    /// type when both are quantities of one dimension in different units
    /// (FR-337-AC-15's converted threshold).
    Threshold(Option<&'t ValueType>),
}

/// The QSpec FR-370 term family of an aggregate value or threshold.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Family {
    /// `integer`: an `Integer` or a bounded `Int`.
    Integer,
    /// `rational`.
    Rational,
    /// `quantity` in this unit.
    Quantity(quire_exact::UnitId),
}

/// The family of `value_type`, or `None` for a type no aggregate value or
/// threshold takes. A `BoundedQuantity` is deliberately none: no source form
/// builds the type yet (QSL-1155), so it is refused conservatively rather
/// than compared as a `Quantity` unchecked; a `Decimal` has no FR-370 family.
pub(crate) fn family(value_type: &ValueType) -> Option<Family> {
    match value_type {
        ValueType::Integer | ValueType::Int(_) => Some(Family::Integer),
        ValueType::Rational(_) => Some(Family::Rational),
        ValueType::Quantity(unit) => Some(Family::Quantity(unit.clone())),
        _ => None,
    }
}

/// FR-194: a threshold written as an exact fraction `n/d` of two integer
/// literals, as the form `rational(n, d)` and the point type of its value,
/// reduced and measured against the rational-bits `ceiling` as the quantile
/// level is (`quantity_magnitude`); `None` for any other threshold.
fn exact_fraction(
    threshold: &qsl_forms::ExpressionForm,
    ceiling: u64,
    location: &Location,
) -> Result<Option<(qsl_forms::ExpressionForm, ValueType)>, CheckRefusal> {
    use qsl_forms::{BinaryOperator, ExprNode};
    let expression = &threshold.expression;
    let root = expression.root();
    let ExprNode::Binary {
        operator: BinaryOperator::Divide,
        left,
        right,
    } = root.node()
    else {
        return Ok(None);
    };
    let operand = |id| match expression.get(id).map(qsl_forms::ExprRef::node) {
        Some(ExprNode::Integer(value)) => Some(value.clone()),
        _ => None,
    };
    let (Some(numerator), Some(denominator)) = (operand(*left), operand(*right)) else {
        return Ok(None);
    };
    let written = qsl_forms::ExactValueForm {
        kind: qsl_forms::ExactValueKind::Rational,
        first: numerator,
        second: denominator,
        span: qsl_foundation::Span { start: 0, end: 0 },
    };
    let value = crate::check::probabilistic::quantity_magnitude(&written, ceiling, location)?;
    let point = |value: &quire_exact::Integer| {
        quire_exact::IntegerInterval::new(value.clone(), value.clone())
    };
    let (Some(numerators), Some(denominators)) = (
        point(value.numerator()).ok(),
        point(value.denominator()).ok(),
    ) else {
        return Ok(None);
    };
    let Ok(domain) = quire_exact::RationalDomain::new(numerators, denominators) else {
        return Ok(None);
    };
    let leaf = ExprNode::Rational(value.numerator().clone(), value.denominator().clone());
    let Some(form) = threshold.cut(root.id(), &BTreeMap::from([(root.id(), leaf)])) else {
        return Ok(None);
    };
    Ok(Some((form, ValueType::Rational(domain))))
}

/// Whether `node` is an integer literal.
fn is_integer_literal(node: crate::check::ir::CheckedNode<'_>) -> bool {
    matches!(
        node.kind(),
        quire_checked::node::NodeKind::Literal(quire_checked::node::CheckedLiteral(
            quire_exact::Value::Integer(_)
        ))
    )
}

/// Type `condition` against `goal` over `bound`, bound from slot 0, at
/// `location`, and check its definedness over the current-state observation.
#[allow(clippy::too_many_arguments)]
fn type_condition(
    goal: Goal<'_>,
    condition: &qsl_forms::ExpressionForm,
    bound: &[(String, ValueType)],
    location: &Location,
    input: &ClauseDeclarations<'_>,
    named: &crate::check::check::NamedScope,
    nodes: &mut u64,
    cx: &mut CheckContext<'_, Scope>,
) -> Result<TypedCondition, CheckRefusal> {
    let location = location.clone();
    {
        let mut typer = Typer::new(
            input.scope,
            input.signatures,
            input.checking_limits,
            nodes,
            ClauseKind::Invariant,
        )
        .with_cancel(input.cancel)
        .with_context(cx.reborrow(input.scope))
        // FR-391: a `holds` atom and a guard are population positions.
        .with_named_populations(named.clone());
        bind_parameters(&mut typer, bound, &location)?;
        typer.enter_state_clause(StateContext {
            parent_projections: BTreeMap::new(),
            kind: StateClauseKind::Invariant,
            operation: None,
            self_slot: NO_SLOT,
            result_slot: None,
        });
        let body = match goal {
            Goal::Boolean => {
                let body =
                    typer.infer(&condition.expression, Some(&ValueType::Boolean), &location)?;
                if body.root().value_type() != &ValueType::Boolean {
                    return Err(CheckRefusal {
                        location,
                        cause: CheckCause::NonBooleanRoot,
                    });
                }
                body
            }
            Goal::Declared(declared) => {
                typer.check_as(&condition.expression, declared, &location)?
            }
            Goal::Any | Goal::Threshold(None) => {
                typer.infer(&condition.expression, None, &location)?
            }
            Goal::Threshold(Some(value)) => {
                typer.infer_in_unit_of(&condition.expression, value, &location)?
            }
        };
        let slots = typer.slots();
        let slot_names = typer.slot_names().to_vec();
        let formed_units = typer.into_formed_units();
        let observations = Observations::of(body.root(), Observation::Current, bound.len());
        Definedness::new(
            bound.len(),
            input.dispatch_tables,
            &input.scope.dispatch_operations,
        )
        .with_observations(&observations)
        .check(body.root())?;
        Ok(TypedCondition {
            body,
            slots,
            slot_names,
            formed_units,
        })
    }
}
