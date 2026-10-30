// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-234: a `temporal` clause under the timed profile
//! `quire.temporal.timed/v1` (ADR-026 DF-1 to DF-7).
//!
//! The timed profile is selected by a `model-time` clock binding (ADR-026
//! CB-1; QSpec FR-250-AC-7), so a clause under it binds `model-time` over a
//! timed model first: FR-233's binding, read as far as FR-234 needs it.
//! `clock "model-time"` over a model with no time source refuses
//! `invalid_timed_model`/`model-time-on-untimed-model`. Any other clock role,
//! and a subject that is no model object type (a timed-trace subject, whose
//! binding QSL states nowhere), refuses as not yet implemented.
//!
//! Each timed interval's bounds are time quantities converted exactly to the
//! binding's unit (FR-233 "Intervals", through FR-230's time units), then
//! validated into a [`TimedInterval`]: a negative lower bound, `a > b` and an
//! open end with `a = b` refuse `invalid_timed_model`/`invalid-interval` at
//! the interval. A step interval `[a, b]` has bounds with no unit and refuses
//! `ill_typed`/`type-mismatch`; `[a, *]` refuses as ADR-014 TR-3 states.
//!
//! The clause is classified into one [`TimedPropertyForm`], in FR-234's
//! order, and a `TimedInvariant`, `TimedSafety` or `TimedLiveness` clause
//! holding a punctual interval records that interval's span: the request
//! writer disposes its item `Unsupported`, `PunctualInterval` (DF-5).

use qsl_forms::{QuantityForm, TimedIntervalForm, TimedUpper};
use qsl_foundation::bound::TimedInterval;
use qsl_foundation::diagnostic::{LimitExceeded, StageFailure};
use qsl_foundation::{Setting, Span};
use quire_exact::{Integer, Rational, ValueType};

use super::{
    in_safety_fragment, is_temporal_node, located, Activation, CheckedFormula, CheckedInterval,
    FormulaNode, TemporalDeclarations, UnaryTemporal,
};
use crate::check::probabilistic::{exact_value, ExactFault};
use crate::check::refusal::{CheckRefusal, TemporalCause};
use crate::check::time_units::TimeScale;

/// The clock role that binds `model-time` (ADR-026 CB-1).
const MODEL_TIME: &str = "model-time";

/// The kind of a timed model's time source (FR-230 `TimeSource`, FR-233
/// `TimeSourceKind`).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum TimeSourceKind {
    /// `time dense unit u`.
    Dense,
    /// `time tick O period p unit u`.
    Tick {
        /// The tick operation `O`, as resolved.
        operation: String,
        /// The period `p`, an exact positive rational in the unit.
        period: Rational,
    },
}

/// The checked time source of one timed model (FR-230's `TimedModel`, as
/// far as a `model-time` binding reads it).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ModelTimeSource {
    /// The model: the alias of the `model` declaration that selects its
    /// domain package, or the package identity when no alias names it.
    pub model: String,
    /// The complete source `time` declaration, for FR-322 occurrence mapping.
    pub span: Span,
    /// The model node of the selected package's one object type, the state
    /// object: the timed root's and every clock place's `model` (QSpec FR-322
    /// "Timed-model nodes", FR-415-AC-17).
    pub model_node: quire_exact::NodeKey,
    /// The FCD declaration key of a tick operation, used by S4 to build its
    /// checked operation-anchor node.
    pub tick_operation_key: Option<quire_checked::key::DeclarationKey>,
    /// The source span of the qualified tick-operation reference.
    pub tick_operation_span: Option<qsl_foundation::Span>,
    /// The checked `operation_anchor` node for the resolved tick operation.
    pub tick_operation_anchor: Option<quire_exact::NodeKey>,
    /// The source kind.
    pub source: TimeSourceKind,
    /// The time unit `u`, by its declared name in the package's unit graph.
    pub unit: String,
    /// The resolved unit node, retained for checked-package emission.
    pub unit_node: quire_exact::NodeKey,
    /// The exact nominal `Time` dimension node selected by `unit` (FR-230-AC-33).
    pub time_dimension: quire_exact::NodeKey,
    /// Nominal dimension and unit nodes needed by the timed root, including
    /// the resolved unit's target chain.
    pub nominal_nodes: Vec<TimeNominalNode>,
}

/// A nominal unit-graph node that S4 must include when a timed model names it.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TimeNominalNode {
    /// The exact nominal node identity.
    pub key: quire_exact::NodeKey,
    /// `dimension` or `unit`.
    pub form: String,
    /// The declaration's qualified name segments: `Some` exactly when
    /// [`Self::owner`] is [`NodeOwner::Source`], `None` for a model-owned
    /// node, which carries no declaration (FR-322, AC-105).
    pub qualified_declaration: Option<Vec<String>>,
    /// The admitted owner of the declaration. A node of the checked unit's
    /// own unit graph is owned by the unit's source
    /// ([`NodeOwner::Source`], whose `package_name` is the manifest
    /// package name) and has a declaration. A node a selected domain
    /// package declares (its `Time` dimension or a unit of the `Time`
    /// dimension) is owned by that model ([`NodeOwner::Model`], whose
    /// `identity` is the package identity and `node` the declaration's IR
    /// node identity) and has none.
    pub owner: crate::value::NodeOwner,
    /// Nominal nodes this node names.
    pub dependencies: Vec<quire_exact::NodeKey>,
}

impl TimeNominalNode {
    /// The declaration's qualified name segments (FR-322): `Some` for a
    /// [`NodeOwner::Source`] node, `None` for a [`NodeOwner::Model`] node.
    pub fn declaration(&self) -> Option<&[String]> {
        self.qualified_declaration.as_deref()
    }

    /// The admitted owner of the declaration; see the field's owner rule.
    pub fn owner(&self) -> &crate::value::NodeOwner {
        &self.owner
    }
}

/// A clause's `model-time` binding (FR-233's `ModelClockBinding::ModelTime`):
/// the model, its time source kind and its unit.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ModelTimeBinding {
    /// The model the clause's subject belongs to.
    pub model: String,
    /// The model's time source kind.
    pub source: TimeSourceKind,
    /// The unit every interval bound converts to.
    pub unit: String,
}

/// A timed clause's property form (FR-234 "Forms"; ADR-026 DF-4, TT-1 to
/// TT-4).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum TimedPropertyForm {
    /// TT-1: `always holds(I)`.
    TimedInvariant,
    /// TT-2: activation `on origin` with only interval operators; `horizon`
    /// is the sum of the formula's upper bounds, in the binding's unit.
    BoundedWindow {
        /// The sum of the formula's upper bounds.
        horizon: Rational,
    },
    /// TT-3: a formula in ADR-014 A-4's safety fragment.
    TimedSafety,
    /// TT-4: any other admitted formula.
    TimedLiveness,
}

/// What the timed check reads of the package beyond the clause.
pub(crate) struct TimedReader<'a> {
    pub(crate) declarations: &'a TemporalDeclarations<'a>,
    pub(crate) target: TimeScale,
}

/// The model a clause over the object type `object` binds, and its time
/// source when the model is timed.
fn model_of<'a>(
    declarations: &'a TemporalDeclarations<'_>,
    object: quire_exact::EffectiveId,
) -> Option<(String, Option<&'a ModelTimeSource>)> {
    let model = declarations
        .models
        .iter()
        .find(|model| model.declaration_key(object.clone()).is_some())?;
    let name = model
        .alias()
        .map_or_else(|| model.selection().identity.clone(), str::to_owned);
    let source = declarations
        .timed
        .time_sources
        .iter()
        .find(|source| source.model == name);
    Some((name, source))
}

/// Whether a clause whose `over` parameter has type `over` is over a timed
/// model, one with a `time` member of either source, `dense` or `tick`
/// (FR-243): its item is EN-6's, under any profile.
pub(crate) fn over_timed(over: &ValueType, declarations: &TemporalDeclarations<'_>) -> bool {
    let ValueType::Reference(object) = over else {
        return false;
    };
    model_of(declarations, object.clone())
        .and_then(|(_, source)| source)
        .is_some()
}

/// The clause's `model-time` binding under the timed profile (FR-233,
/// ADR-026 CB-1), or the refusal its clock role and subject give.
pub(crate) fn binding(
    clock: &qsl_forms::ClockForm,
    over: &ValueType,
    declarations: &TemporalDeclarations<'_>,
    clause: Span,
) -> Result<ModelTimeBinding, TemporalCause> {
    let ValueType::Reference(object) = over else {
        return Err(TemporalCause::TimedTraceSubject { span: clause });
    };
    let Some((model, source)) = model_of(declarations, object.clone()) else {
        return Err(TemporalCause::TimedTraceSubject { span: clause });
    };
    match (clock.text.as_str(), source) {
        (MODEL_TIME, Some(source)) => Ok(ModelTimeBinding {
            model,
            source: source.source.clone(),
            unit: source.unit.clone(),
        }),
        (MODEL_TIME, None) => Err(TemporalCause::ModelTimeOnUntimedModel {
            model,
            span: clock.span,
        }),
        (other, _) => Err(TemporalCause::TimedProfileWithoutModelTime {
            clock: other.to_owned(),
            span: clock.span,
        }),
    }
}

/// Why a timed interval did not check: a cause that joins the clause's
/// refusals, or a failure that stops the check (a limit or a malformed
/// exact value).
pub(crate) enum IntervalFailure {
    Cause(TemporalCause),
    Stop(StageFailure<CheckRefusal>),
}

impl TimedReader<'_> {
    /// One bound converted exactly to the binding's unit.
    fn bound(&self, quantity: &QuantityForm) -> Result<Rational, IntervalFailure> {
        let ceiling = self.declarations.timed.rational_bits;
        let magnitude = exact_value(&quantity.magnitude, ceiling).map_err(|fault| match fault {
            ExactFault::ZeroDenominator => IntervalFailure::Cause(TemporalCause::ZeroDenominator {
                span: quantity.magnitude.span,
            }),
            ExactFault::AboveCeiling { bits } => IntervalFailure::Stop(located(
                LimitExceeded::new(Setting::S3RationalBits, ceiling, u128::from(bits)),
                self.declarations,
                quantity.magnitude.span,
            )),
        })?;
        self.declarations
            .timed
            .units
            .convert(&magnitude, &quantity.unit.name, &self.target)
            .map_err(|cause| {
                IntervalFailure::Cause(TemporalCause::TimeQuantity {
                    cause: Box::new(cause),
                    span: quantity.unit.span,
                })
            })
    }

    /// The validated interval `written` denotes in the binding's unit.
    pub(crate) fn interval(
        &self,
        written: &TimedIntervalForm,
    ) -> Result<TimedInterval, IntervalFailure> {
        let lower = self.bound(&written.lower)?;
        let TimedUpper::Finite(upper) = &written.upper else {
            // `[q, *]` is a hybrid claim's form (FR-447); a model-subject
            // clause has no unbounded timed interval (ADR-014 TR-3).
            return Err(IntervalFailure::Cause(TemporalCause::OpenInterval {
                span: written.span,
            }));
        };
        let upper = self.bound(upper)?;
        TimedInterval::new(lower, written.lower_closed, upper, written.upper_closed).map_err(
            |reason| {
                IntervalFailure::Cause(TemporalCause::InvalidTimedInterval {
                    reason,
                    span: written.span,
                })
            },
        )
    }
}

/// The timed interval a node of `formula` carries, if any.
fn timed(node: &FormulaNode) -> Option<&TimedInterval> {
    match node {
        FormulaNode::Unary {
            interval: Some(CheckedInterval::Timed(interval)),
            ..
        }
        | FormulaNode::Binary {
            interval: Some(CheckedInterval::Timed(interval)),
            ..
        } => Some(interval),
        FormulaNode::Unary { .. }
        | FormulaNode::Binary { .. }
        | FormulaNode::Constant(_)
        | FormulaNode::Holds { .. }
        | FormulaNode::Aggregate { .. }
        | FormulaNode::Not(_)
        | FormulaNode::And(..)
        | FormulaNode::Or(..)
        | FormulaNode::Implies(..) => None,
    }
}

/// The clause's timed property form (FR-234 "Forms"), in the order FR-234
/// states.
pub(crate) fn property_form(formula: &CheckedFormula, activation: Activation) -> TimedPropertyForm {
    let nodes = formula.nodes();
    let invariant = match nodes.get(formula.root().index()) {
        Some(FormulaNode::Unary {
            operator: UnaryTemporal::Always,
            interval: None,
            operand,
        }) => matches!(nodes.get(operand.index()), Some(FormulaNode::Holds { .. })),
        _ => false,
    };
    if invariant {
        return TimedPropertyForm::TimedInvariant;
    }
    let only_interval_operators = nodes
        .iter()
        .filter(|node| is_temporal_node(node))
        .all(|node| timed(node).is_some());
    if activation == Activation::Origin && only_interval_operators {
        let horizon = nodes
            .iter()
            .filter_map(timed)
            .fold(Rational::from_integer(Integer::zero()), |sum, interval| {
                sum.add(interval.upper())
            });
        return TimedPropertyForm::BoundedWindow { horizon };
    }
    if in_safety_fragment(formula) {
        TimedPropertyForm::TimedSafety
    } else {
        TimedPropertyForm::TimedLiveness
    }
}

/// The span of the first punctual interval, in arena order, of a clause
/// DF-5 does not admit over a timed subject: one whose form is not
/// `BoundedWindow` (FR-234 "Punctuality").
pub(crate) fn unsupported_punctual(form: &TimedPropertyForm, punctual: &[Span]) -> Option<Span> {
    match form {
        TimedPropertyForm::BoundedWindow { .. } => None,
        TimedPropertyForm::TimedInvariant
        | TimedPropertyForm::TimedSafety
        | TimedPropertyForm::TimedLiveness => punctual.first().copied(),
    }
}
