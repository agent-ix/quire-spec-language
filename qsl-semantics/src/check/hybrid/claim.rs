// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-447 "Hybrid claims": the S3 check of a `temporal` clause whose `over`
//! parameter is typed by a `hybrid` declaration, in place of the
//! model-subject rules (FR-123, FR-233), with the meaning QSpec FR-193
//! "Hybrid claims" gives it.

use std::collections::BTreeMap;

use qsl_forms::{
    ActivationForm, ExprNode, ExpressionForm, FairnessConstraintForm, OperatorInterval,
    QuantityForm, TemporalClauseForm, TemporalFormulaForm, TemporalNodeForm, TemporalNodeId,
    TemporalOperator, TimedUpper, TypeFormHead,
};
use qsl_foundation::selection::ProfileSelection;
use qsl_foundation::Span;
use quire_exact::{Identifier, Integer, Rational};
use quire_semantic_value::fairness::{FairnessGranularity, FairnessKind};

use super::callee::{kind_of, Callees};
use super::cause::MalformedReason;
use super::model::into_refusal;
use super::term::{lower, ClaimNames, Env, Kind, Meter, Stop};
use super::{
    CheckedHybridClaim, CheckedHybridModel, HybridActivation, HybridCapture, HybridCause,
    HybridClock, HybridFairness, HybridFormula, HybridHorizon, HybridInterval, HybridModelIndex,
    HybridTerm, LogicOp, ModeIndex, TransitionIndex, VariableIndex,
};
use crate::check::probabilistic::{exact_value, ExactFault};
use crate::check::profile::TemporalProfile;
use crate::check::refusal::{CheckRefusal, CheckingLimitKind};
use crate::check::time_units::TimeUnits;
use crate::check::Scope;

/// A hybrid claim as the assembler hands it to the check: the `temporal`
/// clause S2 built, its whole-declaration span, and the profile selection its
/// `using` alias resolved to.
#[derive(Clone, Debug)]
pub struct HybridClaimDeclaration {
    /// The clause as S2 built it.
    pub form: TemporalClauseForm,
    /// The span of the whole declaration.
    pub span: Span,
    /// The unit's profile selection the `using` alias names.
    pub selection: ProfileSelection,
}

/// What the claims are checked against.
pub(in crate::check) struct ClaimInputs<'a> {
    /// The clean hybrid models, in source order.
    pub(in crate::check) models: &'a [CheckedHybridModel],
    /// Each clean model's position by declared name.
    pub(in crate::check) index: &'a BTreeMap<String, HybridModelIndex>,
    /// The type scope a capture's type resolves in.
    pub(in crate::check) scope: &'a Scope,
    /// The package's time units, for a timed interval's quantities.
    pub(in crate::check) units: &'a TimeUnits,
    /// The `s3.rational_bits` ceiling a quantity's magnitude is read under.
    pub(in crate::check) rational_bits: u64,
    /// The functions a term may call.
    pub(in crate::check) callees: &'a Callees<'a>,
    /// The package's node ceiling.
    pub(in crate::check) node_limit: u64,
}

/// What the check of a unit's hybrid claims produced.
pub(in crate::check) struct ClaimsChecked {
    /// The claims that checked clean, in source order.
    pub(in crate::check) claims: Vec<CheckedHybridClaim>,
    /// Every refusal, in source order.
    pub(in crate::check) refusals: Vec<CheckRefusal>,
}

/// Check each claim. A claim over a model that did not check clean is
/// skipped: the model's own refusals stand for it.
pub(in crate::check) fn check_claims(
    declarations: &[HybridClaimDeclaration],
    inputs: &ClaimInputs<'_>,
    nodes_used: &mut u64,
) -> ClaimsChecked {
    let mut checked = ClaimsChecked {
        claims: Vec::new(),
        refusals: Vec::new(),
    };
    let mut meter = Meter {
        used: nodes_used,
        limit: inputs.node_limit,
    };
    for declaration in declarations {
        let mut stops = Vec::new();
        let claim = check_claim(declaration, inputs, &mut meter, &mut stops);
        let limit_reached = stops.iter().any(|stop| matches!(stop, Stop::Limit { .. }));
        let clean = stops.is_empty();
        checked.refusals.extend(stops.into_iter().map(into_refusal));
        if let (Some(claim), true) = (claim, clean) {
            checked.claims.push(claim);
        }
        if limit_reached {
            break;
        }
    }
    checked
}

fn cause(cause: HybridCause) -> Stop {
    Stop::Cause(cause)
}

/// The identifier `name` spells, else the `InvalidName` refusal.
fn identifier(name: &str, span: Span, stops: &mut Vec<Stop>) -> Option<Identifier> {
    match Identifier::new(name) {
        Ok(identifier) => Some(identifier),
        Err(_) => {
            stops.push(cause(HybridCause::Malformed {
                reason: MalformedReason::InvalidName,
                span,
            }));
            None
        }
    }
}

fn root_span(form: &ExpressionForm) -> Span {
    form.spans
        .span(form.spans.root())
        .unwrap_or(Span { start: 0, end: 0 })
}

/// `form` lowered to a term of `kind`, else the refusal its root gives.
fn typed(
    form: &ExpressionForm,
    kind: Kind,
    env: &Env<'_>,
    meter: &mut Meter<'_>,
    stops: &mut Vec<Stop>,
) -> Option<HybridTerm> {
    match lower(form, env, meter) {
        Ok(lowered) if lowered.kind == kind => Some(lowered.term),
        Ok(_) => {
            stops.push(cause(match kind {
                Kind::Boolean => HybridCause::NonBoolean {
                    span: root_span(form),
                },
                Kind::Real => HybridCause::TypeMismatch {
                    span: root_span(form),
                },
            }));
            None
        }
        Err(stop) => {
            stops.push(stop);
            None
        }
    }
}

fn check_claim(
    declaration: &HybridClaimDeclaration,
    inputs: &ClaimInputs<'_>,
    meter: &mut Meter<'_>,
    stops: &mut Vec<Stop>,
) -> Option<CheckedHybridClaim> {
    let form = &declaration.form;
    let TypeFormHead::Name(model_name) = &form.over.value_type.head else {
        return None;
    };
    let model_index = *inputs.index.get(model_name)?;
    let model = inputs.models.get(model_index.get())?;

    let profile = match TemporalProfile::from_identity(&declaration.selection.identity) {
        Some(profile) => profile,
        None => {
            stops.push(cause(HybridCause::NotTemporalProfile {
                alias: form.profile.alias.clone(),
                span: form.profile.span,
            }));
            return None;
        }
    };

    let clock = match &form.clock {
        None => Some(HybridClock::ModelTime),
        Some(written) => clock_of(
            &written.text,
            form.clock_clause_span.unwrap_or(written.span),
            stops,
        ),
    };

    let over = identifier(&form.over.name.name, form.over.name.span, stops);

    let variables: BTreeMap<String, VariableIndex> = model
        .variables
        .iter()
        .enumerate()
        .map(|(position, variable)| {
            (
                variable.name.as_str().to_owned(),
                VariableIndex::new(position),
            )
        })
        .collect();
    let trigger = match &form.activation {
        ActivationForm::Origin => None,
        ActivationForm::Each { parameter, .. } => Some(parameter.name.name.as_str()),
    };

    // Activation: the `on each` parameter is typed by the `over`
    // parameter's declaration; its guard reads the model, the activation
    // position, and no capture.
    let activation = match &form.activation {
        ActivationForm::Origin => Some(HybridActivation::Origin),
        ActivationForm::Each { parameter, when } => {
            let same_model = matches!(
                &parameter.value_type.head,
                TypeFormHead::Name(named) if named == model_name
            );
            if !same_model {
                stops.push(cause(HybridCause::TypeMismatch {
                    span: parameter.value_type.span,
                }));
            }
            let guard = when.as_ref().map(|guard| {
                let names = ClaimNames {
                    over: &form.over.name.name,
                    trigger,
                    captures: &[],
                };
                let env = Env {
                    variables: &variables,
                    reset: false,
                    claim: Some(&names),
                    callees: inputs.callees,
                };
                typed(guard, Kind::Boolean, &env, meter, stops)
            });
            let parameter = identifier(&parameter.name.name, parameter.name.span, stops);
            match (same_model, parameter, guard) {
                (true, Some(parameter), None) => Some(HybridActivation::Each {
                    parameter,
                    guard: None,
                }),
                (true, Some(parameter), Some(Some(guard))) => Some(HybridActivation::Each {
                    parameter,
                    guard: Some(guard),
                }),
                _ => None,
            }
        }
    };

    // Captures, in source order, each over the earlier ones.
    let mut captures: Vec<HybridCapture> = Vec::new();
    let mut capture_kinds: Vec<(String, Kind)> = Vec::new();
    let mut first_capture: BTreeMap<&str, Span> = BTreeMap::new();
    let mut captures_ok = true;
    for capture in &form.captures {
        let written = &capture.parameter;
        if let Some(first) = first_capture.get(written.name.name.as_str()) {
            stops.push(cause(HybridCause::DuplicateCapture {
                name: written.name.name.clone(),
                first: *first,
                span: written.name.span,
            }));
            captures_ok = false;
            continue;
        }
        first_capture.insert(written.name.name.as_str(), written.name.span);
        // The name joins the captures later initializers and the body read
        // even when its own type or value refuses, so one defect refuses
        // once.
        let position = capture_kinds.len();
        capture_kinds.push((written.name.name.clone(), Kind::Real));
        let resolved = crate::check::type_form::resolve_form(inputs.scope, &written.value_type);
        let ty = match resolved {
            Ok(ty) => ty,
            Err(error) => {
                stops.push(cause(HybridCause::CaptureType {
                    fault: error.fault,
                    span: error.span,
                }));
                captures_ok = false;
                continue;
            }
        };
        let Some(kind) = kind_of(&ty) else {
            stops.push(cause(HybridCause::TypeMismatch {
                span: written.value_type.span,
            }));
            captures_ok = false;
            continue;
        };
        if let Some(slot) = capture_kinds.get_mut(position) {
            slot.1 = kind;
        }
        let names = ClaimNames {
            over: &form.over.name.name,
            trigger,
            captures: &capture_kinds[..position],
        };
        let env = Env {
            variables: &variables,
            reset: false,
            claim: Some(&names),
            callees: inputs.callees,
        };
        let value = typed(&capture.value, kind, &env, meter, stops);
        let capture_name = identifier(&written.name.name, written.name.span, stops);
        match (value, capture_name) {
            (Some(value), Some(capture_name)) => captures.push(HybridCapture {
                name: capture_name,
                ty,
                value,
            }),
            _ => captures_ok = false,
        }
    }

    // Fairness.
    let bounded = profile.is_bounded();
    let fairness = fairness(&form.fairness, bounded, model, stops);

    // The formula, under the claim's clock and profile.
    let names = ClaimNames {
        over: &form.over.name.name,
        trigger,
        captures: &capture_kinds,
    };
    let env = Env {
        variables: &variables,
        reset: false,
        claim: Some(&names),
        callees: inputs.callees,
    };
    let lowered = match clock {
        Some(clock) => formula(
            &form.formula,
            &Reading {
                clock,
                profile,
                inputs,
            },
            &env,
            meter,
            stops,
        ),
        None => None,
    };

    let (clock, activation, over, lowered) = (clock?, activation?, over?, lowered?);
    if !captures_ok {
        return None;
    }
    let formula = lowered;
    let horizon = match hybrid_horizon(&formula, clock, &activation) {
        Ok(horizon) => horizon,
        Err(HorizonOutOfRange) => {
            stops.push(cause(HybridCause::HorizonOutOfRange {
                span: declaration.span,
            }));
            return None;
        }
    };
    Some(CheckedHybridClaim {
        model: model_index,
        profile,
        over,
        clock,
        activation,
        captures,
        fairness: fairness?,
        formula,
        horizon,
    })
}

/// The clock a `clock "..."` string binds: `model-time` or `model-steps`
/// (QSpec FR-252). A model subject has only these two bindings.
#[ix_trace_rs::string_edge]
fn clock_of(text: &str, span: Span, stops: &mut Vec<Stop>) -> Option<HybridClock> {
    if text == "model-time" {
        Some(HybridClock::ModelTime)
    } else if text == "model-steps" {
        Some(HybridClock::ModelSteps)
    } else {
        stops.push(cause(HybridCause::UnsupportedClock { span }));
        None
    }
}

/// Splits a fairness name into its mode segment and the optional target
/// segment: `S` or `S::T`. `None` for a name of more than two segments.
#[ix_trace_rs::string_edge]
fn fairness_segments(name: &str) -> Option<(&str, Option<&str>)> {
    let mut segments = name.split("::");
    let mode = segments.next()?;
    let target = segments.next();
    if segments.next().is_some() {
        return None;
    }
    Some((mode, target))
}

/// The checked fairness set: each constraint once, in source order of first
/// occurrence (FR-447 "Hybrid claims"). `None` when any refused.
fn fairness(
    written: &[FairnessConstraintForm],
    bounded: bool,
    model: &CheckedHybridModel,
    stops: &mut Vec<Stop>,
) -> Option<Vec<HybridFairness>> {
    let mut checked: Vec<HybridFairness> = Vec::new();
    let mut ok = true;
    for constraint in written {
        if bounded {
            stops.push(cause(HybridCause::FairnessUnderBoundedProfile {
                span: constraint.span,
            }));
            ok = false;
            continue;
        }
        let missing = || {
            cause(HybridCause::MissingName {
                name: constraint.operation.name.clone(),
                span: constraint.operation.span,
            })
        };
        let resolved = fairness_segments(&constraint.operation.name).and_then(|(mode, target)| {
            let mode_index = model
                .modes
                .iter()
                .position(|candidate| candidate.name.as_str() == mode)?;
            let target = match target {
                Some(target) => Some(
                    model
                        .modes
                        .iter()
                        .position(|candidate| candidate.name.as_str() == target)?,
                ),
                None => None,
            };
            let transitions: Vec<(ModeIndex, TransitionIndex)> = model
                .modes
                .get(mode_index)?
                .transitions
                .iter()
                .enumerate()
                .filter(|(_, transition)| {
                    target.is_none_or(|target| transition.target.get() == target)
                })
                .map(|(position, _)| (ModeIndex::new(mode_index), TransitionIndex::new(position)))
                .collect();
            (!transitions.is_empty()).then_some((transitions, target.map(ModeIndex::new)))
        });
        let Some((transitions, named_target)) = resolved else {
            stops.push(missing());
            ok = false;
            continue;
        };
        let checked_constraint = HybridFairness {
            kind: match constraint.kind {
                Some(qsl_forms::FairnessKind::Strong) => FairnessKind::Strong,
                Some(qsl_forms::FairnessKind::Weak) | None => FairnessKind::Weak,
            },
            granularity: match constraint.granularity {
                Some(qsl_forms::FairnessGranularity::Each) => FairnessGranularity::Each,
                Some(qsl_forms::FairnessGranularity::Whole) | None => FairnessGranularity::Whole,
            },
            named_target,
            transitions,
        };
        if !checked.contains(&checked_constraint) {
            checked.push(checked_constraint);
        }
    }
    ok.then_some(checked)
}

/// How a claim's intervals and `holds` atoms are read.
struct Reading<'a> {
    clock: HybridClock,
    profile: TemporalProfile,
    inputs: &'a ClaimInputs<'a>,
}

/// One bound of a timed interval converted exactly to seconds.
fn quantity_seconds(quantity: &QuantityForm, reading: &Reading<'_>) -> Result<Rational, Stop> {
    let inputs = reading.inputs;
    let magnitude =
        exact_value(&quantity.magnitude, inputs.rational_bits).map_err(|fault| match fault {
            ExactFault::ZeroDenominator => cause(HybridCause::ZeroDenominator {
                span: quantity.magnitude.span,
            }),
            ExactFault::AboveCeiling { bits } => Stop::Limit {
                kind: CheckingLimitKind::RationalBits,
                limit: inputs.rational_bits,
                actual: u128::from(bits),
            },
        })?;
    let time_error = |reason| {
        cause(HybridCause::TimeQuantity {
            cause: Box::new(reason),
            span: quantity.unit.span,
        })
    };
    // A hybrid model is a timed subject in unit second.
    let target = inputs
        .units
        .target(SECOND)
        .map_err(time_error)?
        .ok_or_else(|| {
            cause(HybridCause::TypeMismatch {
                span: quantity.span,
            })
        })?;
    inputs
        .units
        .convert(&magnitude, &quantity.unit.name, &target)
        .map_err(time_error)
}

/// The unit a hybrid model's time is counted in (QSpec FR-252: "a hybrid
/// model is a timed subject in unit second").
const SECOND: &str = "s";

fn whole(value: u64) -> Rational {
    Rational::from_integer(Integer::from(value))
}

/// An operator's interval, converted to exact bounds and checked against the
/// claim's clock and profile (FR-447 "Intervals").
fn interval(
    written: Option<&OperatorInterval>,
    operator_span: Span,
    reading: &Reading<'_>,
) -> Result<Option<HybridInterval>, Stop> {
    let fit = |span| {
        cause(HybridCause::ProfileFit {
            profile: reading.profile,
            span,
        })
    };
    let Some(written) = written else {
        // An operator with no interval is admitted only where the profile
        // admits an unbounded operator.
        return if reading.profile.is_bounded() {
            Err(fit(operator_span))
        } else {
            Ok(None)
        };
    };
    let (interval, span) = match written {
        OperatorInterval::Steps(steps) => {
            let upper = match steps.upper {
                qsl_forms::IntervalUpper::Finite(upper) => Some(whole(upper)),
                qsl_forms::IntervalUpper::Open => None,
            };
            (
                HybridInterval {
                    lower: whole(steps.lower),
                    lower_open: false,
                    upper_open: upper.is_none(),
                    upper,
                },
                steps.span,
            )
        }
        OperatorInterval::Timed(timed) => {
            if reading.clock == HybridClock::ModelSteps {
                return Err(cause(HybridCause::TypeMismatch { span: timed.span }));
            }
            let upper = match &timed.upper {
                TimedUpper::Finite(quantity) => Some(quantity_seconds(quantity, reading)?),
                TimedUpper::Open => None,
            };
            (
                HybridInterval {
                    lower: quantity_seconds(&timed.lower, reading)?,
                    lower_open: !timed.lower_closed,
                    upper_open: upper.is_none() || !timed.upper_closed,
                    upper,
                },
                timed.span,
            )
        }
    };
    if let Some(upper) = &interval.upper {
        let inverted = interval.lower > *upper
            || (interval.lower == *upper && (interval.lower_open || interval.upper_open));
        if inverted {
            return Err(cause(HybridCause::InvalidInterval { span }));
        }
    }
    let timed_profile = reading.profile == TemporalProfile::Timed;
    if !timed_profile {
        // An integer profile: whole bounds, both ends closed, and `*` only
        // where the profile admits an unbounded operator.
        let whole_bounds =
            interval.lower.is_integer() && interval.upper.as_ref().is_none_or(Rational::is_integer);
        let closed = !interval.lower_open && (interval.upper.is_none() || !interval.upper_open);
        let finite = interval.upper.is_some() || !reading.profile.is_bounded();
        if !(whole_bounds && closed && finite) {
            return Err(fit(span));
        }
    }
    Ok(Some(interval))
}

/// The claim's formula.
type Lowered = HybridFormula;

fn formula(
    form: &TemporalFormulaForm,
    reading: &Reading<'_>,
    env: &Env<'_>,
    meter: &mut Meter<'_>,
    stops: &mut Vec<Stop>,
) -> Option<Lowered> {
    let total = form.nodes().len();
    // An operator's operands follow it in the arena, so a descending pass
    // sees every operand before its operator.
    let mut built: Vec<Option<Lowered>> = (0..total).map(|_| None).collect();
    let mut ok = true;
    for (id, node) in form.iter().collect::<Vec<_>>().into_iter().rev() {
        let lowered = match node {
            TemporalNodeForm::Constant { value, .. } => {
                Some(HybridFormula::Holds(HybridTerm::Boolean(*value)))
            }
            TemporalNodeForm::Holds { condition, .. } => {
                if condition.expression.iter().any(|node| {
                    matches!(
                        node.node(),
                        ExprNode::Indexed { .. } | ExprNode::StepLabel { .. } | ExprNode::Stutter
                    )
                }) {
                    stops.push(cause(HybridCause::HyperExpression {
                        span: root_span(condition),
                    }));
                    None
                } else {
                    typed(condition, Kind::Boolean, env, meter, stops).map(HybridFormula::Holds)
                }
            }
            TemporalNodeForm::Operator(operator) => {
                let mut operands = Vec::with_capacity(operator.operands.len());
                for operand in &operator.operands {
                    operands.push(take(&mut built, *operand));
                }
                let interval =
                    interval(operator.interval.as_ref(), operator.operator_span, reading);
                let interval = match interval {
                    Ok(interval) => Some(interval),
                    Err(stop) => {
                        stops.push(stop);
                        None
                    }
                };
                match (interval, operands.into_iter().collect::<Option<Vec<_>>>()) {
                    (Some(interval), Some(operands)) => {
                        combine(operator.operator, interval, operands)
                    }
                    _ => None,
                }
            }
        };
        ok &= lowered.is_some();
        if let Some(slot) = built.get_mut(id.index()) {
            *slot = lowered;
        }
    }
    let root = take(&mut built, form.root());
    if ok {
        root
    } else {
        None
    }
}

fn zero() -> Rational {
    Rational::from_integer(Integer::zero())
}

fn take(built: &mut [Option<Lowered>], id: TemporalNodeId) -> Option<Lowered> {
    built.get_mut(id.index())?.take()
}

/// `operator` applied to its lowered `operands` (source order).
fn combine(
    operator: TemporalOperator,
    interval: Option<HybridInterval>,
    mut operands: Vec<Lowered>,
) -> Option<Lowered> {
    let right = if operands.len() == 2 {
        operands.pop()
    } else {
        None
    };
    let left = operands.pop()?;
    match (operator, right) {
        (TemporalOperator::Not, None) => Some(HybridFormula::Not(Box::new(left))),
        (TemporalOperator::And | TemporalOperator::Or | TemporalOperator::Implies, Some(right)) => {
            let op = match operator {
                TemporalOperator::And => LogicOp::And,
                TemporalOperator::Or => LogicOp::Or,
                _ => LogicOp::Implies,
            };
            Some(HybridFormula::Logic(op, Box::new(left), Box::new(right)))
        }
        (TemporalOperator::Always, None) => Some(HybridFormula::Always(interval, Box::new(left))),
        (TemporalOperator::Eventually, None) => {
            Some(HybridFormula::Eventually(interval, Box::new(left)))
        }
        (TemporalOperator::Historically, None) => {
            Some(HybridFormula::Historically(interval, Box::new(left)))
        }
        (TemporalOperator::Once, None) => Some(HybridFormula::Once(interval, Box::new(left))),
        (TemporalOperator::Until, Some(right)) => Some(HybridFormula::Until(
            interval,
            Box::new(left),
            Box::new(right),
        )),
        (TemporalOperator::Release, Some(right)) => Some(HybridFormula::Release(
            interval,
            Box::new(left),
            Box::new(right),
        )),
        (TemporalOperator::Since, Some(right)) => Some(HybridFormula::Since(
            interval,
            Box::new(left),
            Box::new(right),
        )),
        (TemporalOperator::Triggered, Some(right)) => Some(HybridFormula::Triggered(
            interval,
            Box::new(left),
            Box::new(right),
        )),
        _ => None,
    }
}

/// The horizon of a claim with `formula` under `clock` and `activation`
/// (FR-447 "Horizon"): the reach of the formula, in seconds under
/// `model-time` and in positions under `model-steps`, and `None` for an
/// `each` activation or an unbounded formula. QSL's I2 read derives the
/// horizon of a claim from its formula nodes by this one rule.
///
/// # Errors
///
/// [`HorizonOutOfRange`] when a `model-steps` reach is not a whole number
/// of positions that fits a `u64`.
pub fn hybrid_horizon(
    formula: &HybridFormula,
    clock: HybridClock,
    activation: &HybridActivation,
) -> Result<Option<HybridHorizon>, HorizonOutOfRange> {
    let reach = match (activation, reach(formula)) {
        (HybridActivation::Each { .. }, _) | (_, None) => return Ok(None),
        (HybridActivation::Origin, Some(reach)) => reach,
    };
    match clock {
        HybridClock::ModelTime => Ok(Some(HybridHorizon::Seconds(reach))),
        HybridClock::ModelSteps => match reach.numerator().to_u64() {
            Some(positions) if reach.is_integer() => Ok(Some(HybridHorizon::Positions(positions))),
            _ => Err(HorizonOutOfRange),
        },
    }
}

/// A `model-steps` horizon that is not a whole number of positions that
/// fits a `u64`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("the formula's reach is not a whole number of positions that fits a u64")]
pub struct HorizonOutOfRange;

/// The reach of `formula` (`None` when unbounded): `r(Holds)` is 0, a future
/// operator's reach is its interval's upper bound plus its operands', and a
/// past operator reads only earlier positions.
fn reach(formula: &HybridFormula) -> Option<Rational> {
    enum Step<'f> {
        Visit(&'f HybridFormula),
        Build(&'f HybridFormula),
    }
    let mut steps = vec![Step::Visit(formula)];
    let mut done: Vec<Option<Rational>> = Vec::new();
    while let Some(step) = steps.pop() {
        match step {
            Step::Visit(node) => match node {
                HybridFormula::Holds(_) => done.push(Some(zero())),
                HybridFormula::Not(operand)
                | HybridFormula::Always(_, operand)
                | HybridFormula::Eventually(_, operand)
                | HybridFormula::Historically(_, operand)
                | HybridFormula::Once(_, operand) => {
                    steps.push(Step::Build(node));
                    steps.push(Step::Visit(operand));
                }
                HybridFormula::Logic(_, left, right)
                | HybridFormula::Until(_, left, right)
                | HybridFormula::Release(_, left, right)
                | HybridFormula::Since(_, left, right)
                | HybridFormula::Triggered(_, left, right) => {
                    steps.push(Step::Build(node));
                    steps.push(Step::Visit(right));
                    steps.push(Step::Visit(left));
                }
            },
            Step::Build(node) => {
                let combined = match node {
                    HybridFormula::Holds(_) | HybridFormula::Not(_) => continue,
                    HybridFormula::Always(interval, _) | HybridFormula::Eventually(interval, _) => {
                        future(interval, done.pop()?)
                    }
                    HybridFormula::Historically(interval, _) | HybridFormula::Once(interval, _) => {
                        past(interval, done.pop()?)
                    }
                    HybridFormula::Logic(..) => {
                        let (right, left) = (done.pop()?, done.pop()?);
                        greater(left, right)
                    }
                    HybridFormula::Until(interval, ..) | HybridFormula::Release(interval, ..) => {
                        let (right, left) = (done.pop()?, done.pop()?);
                        future(interval, greater(left, right))
                    }
                    HybridFormula::Since(interval, ..) | HybridFormula::Triggered(interval, ..) => {
                        let (right, left) = (done.pop()?, done.pop()?);
                        greater(left, past(interval, right))
                    }
                };
                done.push(combined);
            }
        }
    }
    done.pop()?
}

/// The greater of two reaches; unbounded when either is.
fn greater(left: Option<Rational>, right: Option<Rational>) -> Option<Rational> {
    Some(std::cmp::max(left?, right?))
}

/// A future operator's reach: its interval's upper bound plus the operand's
/// reach; unbounded with no interval or the upper bound `*`.
fn future(interval: &Option<HybridInterval>, operand: Option<Rational>) -> Option<Rational> {
    let upper = interval.as_ref()?.upper.as_ref()?;
    Some(upper.add(&operand?))
}

/// A past operator's reach: `max(0, operand - lower)`, `lower` 0 with no
/// interval. A past operator reads only earlier positions, so it is bounded
/// with no interval or the upper bound `*`.
fn past(interval: &Option<HybridInterval>, operand: Option<Rational>) -> Option<Rational> {
    let lower = interval
        .as_ref()
        .map_or_else(zero, |interval| interval.lower.clone());
    Some(std::cmp::max(zero(), operand?.sub(&lower)))
}
