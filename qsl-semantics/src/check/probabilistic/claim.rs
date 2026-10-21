// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-186: a `probabilistic` claim resolved, typed and recorded.
//!
//! A claim that passes the form checks of [`super`] (thresholds, confidence
//! parameters, a quantile over a fraction) is resolved against the unit: its
//! subject is a `model` alias, its profile a temporal profile, its scheduler
//! a workload of the subject's model or `every scheduler` with a fairness
//! set, its `over` parameter a resolved type. Its event or measure is then
//! typed: each state predicate is a Boolean clause over the `over` parameter,
//! checked as an invariant body is; a temporal event is built and bounded as
//! a bounded-profile `temporal` clause is (FR-326); a reward names a reward
//! of the subject's model; a threshold has the type of the measure it bounds,
//! and a threshold in another unit of the reward's dimension converts exactly
//! (QSpec FR-142).
//!
//! The result is a [`CheckedProbabilisticClaim`] with its normal form
//! (ADR-024 PM-8, PF-10): the normal-form bytes of everything the claim
//! asserts and nothing of how a run meets it, from which S4 and CG derive the
//! ADR-013 O-09 obligation identity. It is not that identity. FR-125's model subject is
//! a request input; this check names it by the `model` alias and the domain
//! package the alias selects.
//!
//! The four forms FR-186 lists are typed, and the two forms only exact
//! evidence decides (FR-195): an unbounded `eventually`, `until` or `always`
//! event under the infinite-trace profile, and an `expected accumulate ...
//! until` claim. An `expected elapsed` claim, an `elapsed` measure, a timed
//! window and an `on each` activation are forms the timed checks own; each
//! refuses `unsupported_construct`/`not-yet-implemented`, so no claim
//! compiles unchecked.

use std::collections::BTreeMap;

use qsl_forms::{
    ActivationForm, BoundOp, ClauseKind, EventForm, ExactValueForm, ExpectedQuantity,
    ExpressionForm, FractionForm, HorizonForm, MeasureForm, MeasureWindowForm, ProbabilisticBody,
    ProbabilisticForm, SchedulerForm, StateClauseKind, TemporalFormulaForm, TemporalNodeForm,
    TemporalNodeId, TemporalOperator, TemporalOperatorForm, WeightingForm,
};
use qsl_foundation::selection::DefinitionRef;
use qsl_foundation::Span;
use quire_exact::{
    Identifier, IllTypedCause, Integer, IntegerInterval, Meter, Quantity, Rational, RationalDomain,
    UnitId, Value, ValueType,
};
use quire_semantic_value::fairness::{
    FairnessConstraint, FairnessGranularity, FairnessKind, FairnessOperation,
};
use quire_semantic_value::location::Location;
use quire_semantic_value::quantity::{convert_quantity, ConvertedValue, QuantityTarget};

use super::super::check::{bind_parameters, Scope, StateContext, Typer};
use super::super::facts::Definedness;
use super::super::hyper_clause::{encode_event, encode_predicate, node_bytes};
use super::super::ir::{CheckedBody, CheckedNode};
use super::super::observation::Observations;
use super::super::profile::{TemporalProfile, CLAUSE_PROFILE_AUTHORITY};
use super::super::refusal::{CheckCause, CheckRefusal, KeyFault, TemporalCause};
use super::super::state_graph_claim::StatePredicate;
use super::super::temporal_clause::{
    build_formula, checked_constraint, checked_operators, horizon, CheckedFormula, HorizonFault,
    IntervalReading,
};
use super::model::{literal, site_location, Pass, Site};
use super::{exact, exact_refusal, refusal, ProbabilisticCause};
use crate::family::CheckContext;
use crate::model::clause_observation::Observation;
use crate::model::domain_package::DomainPackageRef;

/// A slot no local carries: a predicate has no `self`.
const NO_SLOT: usize = usize::MAX;

/// The model a claim is over: the `model` alias it names, and the domain
/// package the alias selects (FR-125). The subject is part of every claim's
/// normal form.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelSubjectRef {
    /// The `model` alias, as written.
    pub alias: String,
    /// The domain package the alias selects.
    pub package: DomainPackageRef,
}

/// The scheduler a claim is stated under (ADR-024 PM-8).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProbScheduler {
    /// `under W`: the workload, by its package-wide name. A workload has no
    /// checked node to identify it by (FR-185 "Package emission"), so the
    /// name stands for it.
    Workload(String),
    /// `under every scheduler`; the claim's fairness set is
    /// [`CheckedProbabilisticClaim::fairness`].
    Every,
}

/// A probability or measure bound, `>= c` or `<= c`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Bound {
    /// `>= c`.
    AtLeast(Rational),
    /// `<= c`.
    AtMost(Rational),
}

impl Bound {
    fn new(op: BoundOp, value: Rational) -> Self {
        match op {
            BoundOp::AtLeast => Self::AtLeast(value),
            BoundOp::AtMost => Self::AtMost(value),
        }
    }

    fn bytes(&self) -> Vec<u8> {
        let (tag, value): (&[u8], _) = match self {
            Self::AtLeast(value) => (b"at-least", value),
            Self::AtMost(value) => (b"at-most", value),
        };
        node_bytes(tag, &[rational_bytes(value)])
    }
}

/// A Boolean state predicate of a claim, typed over the claim's `over`
/// parameter as an invariant body is (FR-104).
#[derive(Debug)]
pub struct CheckedPredicate {
    body: CheckedBody,
    slots: usize,
    /// The predicate's normal form (FR-174), which the claim's normal form
    /// binds.
    normal_form: Vec<u8>,
}

impl CheckedPredicate {
    /// The checked Boolean body.
    pub fn body(&self) -> CheckedNode<'_> {
        self.body.root()
    }

    /// The evaluation slot count: the `over` parameter's slot and every
    /// local the predicate binds.
    pub fn slots(&self) -> usize {
        self.slots
    }
}

impl StatePredicate for CheckedPredicate {
    fn body(&self) -> CheckedNode<'_> {
        Self::body(self)
    }

    fn slots(&self) -> usize {
        Self::slots(self)
    }
}

/// A claim's temporal event: a bounded formula and the predicate of each of
/// its `holds` atoms.
#[derive(Debug)]
pub struct CheckedTemporalFormula {
    formula: CheckedFormula,
    conditions: BTreeMap<usize, CheckedPredicate>,
    normal_form: Vec<u8>,
}

impl CheckedTemporalFormula {
    /// The checked formula.
    pub fn formula(&self) -> &CheckedFormula {
        &self.formula
    }

    /// The predicate of the `holds` atom at arena index `node`.
    pub fn condition(&self, node: usize) -> Option<&CheckedPredicate> {
        self.conditions.get(&node)
    }
}

/// A claim's event (ADR-024 PF-1 to PF-4).
#[derive(Debug)]
pub enum Event {
    /// A formula under a bounded profile with only interval operators, and
    /// its horizon (ADR-014 TR-4).
    Formula {
        /// The formula.
        formula: CheckedTemporalFormula,
        /// The greatest sum of interval upper bounds along a path.
        horizon: u64,
    },
    /// `M <= c` or `M >= c`.
    Compare {
        /// The measure.
        measure: BoundedMeasure,
        /// The comparison, `c` in the measure's own unit.
        bound: Bound,
    },
}

/// A fraction of the first `window` positions satisfying a predicate
/// (ADR-024 PF-5).
#[derive(Debug)]
pub struct FractionMeasure {
    /// The predicate.
    pub predicate: CheckedPredicate,
    /// The window's upper end: the fraction is over `[0, window]`.
    pub window: u64,
    /// The reward each position is weighted by, when written.
    pub weight: Option<Identifier>,
}

/// A bounded measure (ADR-024 PF-3, PF-4).
#[derive(Debug)]
pub enum BoundedMeasure {
    /// `accumulate R from holds(A) until holds(B) within h`.
    Accumulate {
        /// The reward.
        reward: Identifier,
        /// `A`.
        from: CheckedPredicate,
        /// `B`.
        until: CheckedPredicate,
        /// `h`.
        within: u64,
    },
    /// `steps from holds(A) until holds(B) within h`.
    Steps {
        /// `A`.
        from: CheckedPredicate,
        /// `B`.
        until: CheckedPredicate,
        /// `h`.
        within: u64,
    },
    /// `fraction holds(P) over [0, h]`.
    Fraction(FractionMeasure),
}

/// What an unbounded reachability claim asks of its state predicates
/// (ADR-028 XF-4): each is a state predicate over the `over` parameter, with
/// no temporal operator inside.
#[derive(Debug)]
pub enum ReachTarget {
    /// `eventually holds(P)`.
    Eventually(CheckedPredicate),
    /// `holds(A) until holds(B)`.
    Until(CheckedPredicate, CheckedPredicate),
    /// `always holds(P)`.
    Always(CheckedPredicate),
}

/// A claim's form (ADR-024 PF-2 to PF-6, ADR-028 XF-4, XF-5).
#[derive(Debug)]
pub enum ProbForm {
    /// `probability op θ [event]`.
    Probability {
        /// The probability bound `θ`.
        bound: Bound,
        /// The event.
        event: Event,
    },
    /// `quantile q of (measure) op c`.
    Quantile {
        /// The level `q`.
        q: Rational,
        /// The measure: an `accumulate` or a `steps`.
        measure: BoundedMeasure,
        /// The threshold `c`, in the reward's own unit for an `accumulate`.
        threshold: Bound,
    },
    /// `mean of (fraction) op θ`.
    MeanOfFraction {
        /// The bound `θ`.
        bound: Bound,
        /// The fraction.
        fraction: FractionMeasure,
    },
    /// `long_run_fraction holds(P) [weighted by R] op θ`.
    LongRunFraction {
        /// The bound `θ`.
        bound: Bound,
        /// `P`.
        predicate: CheckedPredicate,
        /// The reward positions are weighted by, when written.
        weight: Option<Identifier>,
    },
    /// `probability op θ [eventually | until | always]` over the
    /// infinite-trace profile, which only exact evidence decides.
    Reach {
        /// The probability bound `θ`.
        bound: Bound,
        /// The reachability target.
        target: ReachTarget,
    },
    /// `expected accumulate R until holds(B) op c`, which only exact
    /// evidence decides. An `expected elapsed` claim is not typed: no
    /// subject is timed yet.
    ExpectedReward {
        /// The bound `c` on the expected reward, in the reward's own unit.
        bound: Bound,
        /// The reward `R`.
        reward: Identifier,
        /// `B`.
        until: CheckedPredicate,
    },
}

/// The confidence parameters of a claim (ADR-024 PF-10).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Confidence {
    /// The greatest probability of rejecting a claim whose bound holds.
    pub alpha: Rational,
    /// The greatest probability of accepting a claim whose bound fails.
    pub beta: Rational,
    /// The indifference half-width.
    pub indifference: Rational,
}

/// A claim's `over (p: T)` parameter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OverBinding {
    /// The parameter's name.
    pub name: String,
    /// Its resolved type.
    pub value_type: ValueType,
}

/// The normal form of a claim: the content its S4 claim node will be keyed
/// over, from which S4 and CG derive the ADR-013 O-09 obligation identity. It
/// is not an identity (R-04): it carries no occurrence key and no digest.
/// It is the length-prefixed normal-form bytes of the subject, profile,
/// form, scheduler, fairness set, `over` type and stated confidence
/// parameters, and binds no source span, no `over` binder name, no method and
/// no budget. Two claims have equal normal forms exactly when they assert the
/// same content.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ClaimNormalForm(Vec<u8>);

impl ClaimNormalForm {
    /// The normal form's bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// A `probabilistic` claim, checked (FR-186 "Outputs").
#[derive(Debug)]
pub struct CheckedProbabilisticClaim {
    /// The claim's name.
    pub name: String,
    /// The span of the whole claim.
    pub span: Span,
    /// The model the claim is over (FR-125).
    pub subject: ModelSubjectRef,
    /// The temporal profile the claim is checked under (FR-123).
    pub profile: DefinitionRef,
    /// The scheduler.
    pub scheduler: ProbScheduler,
    /// The fairness set of a claim `under every scheduler`; empty under a
    /// workload (ADR-024 PM-7).
    pub fairness: Vec<FairnessConstraint>,
    /// The form.
    pub form: ProbForm,
    /// The confidence parameters, when the claim states them.
    pub confidence: Option<Confidence>,
    /// The `over` binding, when the claim states one.
    pub over: Option<OverBinding>,
    /// The claim's normal form.
    pub normal_form: ClaimNormalForm,
    /// The declaration's source order, as [`Reward`](super::Reward)'s.
    pub(super) declared: usize,
}

impl CheckedProbabilisticClaim {
    /// Whether only exact evidence decides the claim (ADR-028 XF-7): an
    /// unbounded reachability or expected-reward form, a claim `under every
    /// scheduler`, or a claim that states no confidence parameters
    /// (ADR-024 PF-10). Statistical evidence for such a claim is refused at
    /// negotiation.
    pub fn exact_only(&self) -> bool {
        self.exact_only_form()
            || self.scheduler == ProbScheduler::Every
            || self.confidence.is_none()
    }

    /// Whether the claim's form is one only exact evidence decides: an
    /// unbounded reachability or expected-reward form.
    pub fn exact_only_form(&self) -> bool {
        matches!(
            self.form,
            ProbForm::Reach { .. } | ProbForm::ExpectedReward { .. }
        )
    }
}

/// Every expression a claim's checked parts are typed from, in the order
/// [`site_expressions`] numbers them: a site names an expression by its
/// position in this list.
pub(super) fn claim_expressions(form: &ProbabilisticForm) -> Vec<&ExpressionForm> {
    let mut expressions = Vec::new();
    match &form.body {
        ProbabilisticBody::Probability { event, .. } => match event {
            EventForm::Temporal(formula) => {
                for (_, node) in formula.iter() {
                    if let TemporalNodeForm::Holds { condition, .. } = node {
                        expressions.push(condition);
                    }
                }
            }
            EventForm::Compare {
                measure, threshold, ..
            } => {
                measure_expressions(measure, &mut expressions);
                expressions.push(threshold);
            }
        },
        ProbabilisticBody::Quantile {
            measure, threshold, ..
        } => {
            measure_expressions(measure, &mut expressions);
            expressions.push(threshold);
        }
        ProbabilisticBody::MeanOfFraction { fraction, .. } => {
            expressions.push(&fraction.predicate);
        }
        ProbabilisticBody::LongRunFraction { predicate, .. } => expressions.push(predicate),
        ProbabilisticBody::ExpectedUntil {
            target, threshold, ..
        } => {
            expressions.push(target);
            expressions.push(threshold);
        }
    }
    expressions
}

/// Every exact value a claim states -- its threshold or level, then each
/// confidence item -- in the order a site names them.
pub(super) fn claim_exact_values(form: &ProbabilisticForm) -> Vec<&ExactValueForm> {
    let mut values = Vec::new();
    match &form.body {
        ProbabilisticBody::Probability { threshold, .. }
        | ProbabilisticBody::MeanOfFraction { threshold, .. }
        | ProbabilisticBody::LongRunFraction { threshold, .. } => values.push(threshold),
        ProbabilisticBody::Quantile { level, .. } => values.push(level),
        ProbabilisticBody::ExpectedUntil { .. } => {}
    }
    values.extend(form.confidence.iter().map(|item| &item.value));
    values
}

fn measure_expressions<'f>(measure: &'f MeasureForm, expressions: &mut Vec<&'f ExpressionForm>) {
    match measure {
        MeasureForm::Accumulate { window, .. }
        | MeasureForm::Steps { window, .. }
        | MeasureForm::Elapsed { window, .. } => {
            expressions.push(&window.from);
            expressions.push(&window.until);
        }
        MeasureForm::Fraction(fraction) => expressions.push(&fraction.predicate),
    }
}

/// What a threshold of a measure is measured in.
#[derive(Clone, Copy)]
enum Measured {
    /// A step count: a natural number.
    Steps,
    /// A fraction: an exact rational in `(0, 1)`.
    Fraction,
    /// A reward's value: an integer or a quantity in the reward's unit.
    Reward(RewardValue),
    /// The expectation of a reward: an exact rational, or a quantity in the
    /// reward's unit, since a mean of integers is not an integer.
    Expectation(RewardValue),
}

/// A reward's value type, as a threshold must match it.
#[derive(Clone, Copy, PartialEq)]
enum RewardValue {
    Integer,
    Quantity(UnitId),
}

/// What the check of one claim reads of it, resolved.
struct ClaimCx<'c> {
    index: usize,
    form: &'c ProbabilisticForm,
    subject: &'c ModelSubjectRef,
    over: Option<&'c OverBinding>,
    expressions: Vec<&'c ExpressionForm>,
    exact_values: Vec<&'c ExactValueForm>,
}

impl ClaimCx<'_> {
    fn claim(&self) -> &str {
        &self.form.name.name
    }

    /// The location of `expression`, one of this claim's own.
    fn location(&self, expression: &ExpressionForm) -> Result<Location, CheckRefusal> {
        let position = self
            .expressions
            .iter()
            .position(|candidate| std::ptr::eq(*candidate, expression))
            .ok_or_else(|| broken("claim-expression-has-a-site"))?;
        Ok(site_location(self.index, Site::Expression(position)))
    }

    /// The location of `value`, one of this claim's own exact values.
    fn exact_location(&self, value: &ExactValueForm) -> Result<Location, CheckRefusal> {
        let position = self
            .exact_values
            .iter()
            .position(|candidate| std::ptr::eq(*candidate, value))
            .ok_or_else(|| broken("claim-exact-value-has-a-site"))?;
        Ok(site_location(self.index, Site::Exact(position)))
    }

    /// The `over` binder's name, which is not content.
    fn binder(&self) -> Option<&str> {
        self.over.map(|over| over.name.as_str())
    }
}

/// `result`'s value, recording its refusal on `refusals`.
fn keep<T>(refusals: &mut Vec<CheckRefusal>, result: Result<T, CheckRefusal>) -> Option<T> {
    result.map_err(|refused| refusals.push(refused)).ok()
}

/// A refusal that is a broken invariant of this check, never of the input.
fn broken(invariant: &'static str) -> CheckRefusal {
    CheckRefusal {
        location: Location::root(quire_semantic_value::location::Origin::Expression),
        cause: CheckCause::InternalFault(Box::new(KeyFault::StageFault(
            qsl_foundation::InternalFault::new("S3", invariant),
        ))),
    }
}

fn temporal_refusal(cause: TemporalCause) -> CheckRefusal {
    CheckRefusal {
        location: Location::root(quire_semantic_value::location::Origin::Expression),
        cause: CheckCause::Temporal(Box::new(cause)),
    }
}

fn rational_bytes(value: &Rational) -> Vec<u8> {
    value.to_string().into_bytes()
}

/// The type a fraction threshold is written at: a rational whose written
/// parts are within the ceiling of an exact value (FR-185 "Exact values"), so
/// its range is checked as a threshold (`0 < x < 1`), not as a type.
fn fraction_type(ceiling: u64) -> Option<ValueType> {
    let bound = Integer::one().shifted_left(ceiling);
    let numerator = IntegerInterval::new(bound.neg(), bound.clone()).ok()?;
    let denominator = IntegerInterval::new(Integer::one(), bound).ok()?;
    Some(ValueType::Rational(
        RationalDomain::new(numerator, denominator).ok()?,
    ))
}

/// Whether the profile a claim's `using` alias selected is the infinite-trace
/// profile; a claim whose profile was refused is not.
fn is_infinite(profile: Option<&DefinitionRef>) -> bool {
    profile.and_then(|profile| TemporalProfile::from_identity(profile.identity()))
        == Some(TemporalProfile::InfiniteTrace)
}

/// Which unbounded reachability operator a formula is.
#[derive(Clone, Copy)]
enum ReachKind {
    Eventually,
    Until,
    Always,
}

/// An event that is exactly one `eventually`, `until` or `always` written
/// with no interval, and its operands in source order.
struct ReachShape<'f> {
    kind: ReachKind,
    operands: &'f [TemporalNodeId],
}

/// `formula`'s [`ReachShape`], when it has one: its root is one of the three
/// operators with no interval and the operator's arity of operands. What the
/// operands are is for [`Pass::reach`] to judge.
fn reach_shape(formula: &TemporalFormulaForm) -> Option<ReachShape<'_>> {
    let Some(TemporalNodeForm::Operator(operator)) = formula.node(formula.root()) else {
        return None;
    };
    let (kind, arity) = match operator.operator {
        TemporalOperator::Eventually => (ReachKind::Eventually, 1),
        TemporalOperator::Always => (ReachKind::Always, 1),
        TemporalOperator::Until => (ReachKind::Until, 2),
        _ => return None,
    };
    (operator.interval.is_none() && operator.operands.len() == arity).then_some(ReachShape {
        kind,
        operands: &operator.operands,
    })
}

/// The span of an expression's root.
fn root_span(expression: &ExpressionForm, fallback: Span) -> Span {
    expression.spans.at(&[]).unwrap_or(fallback)
}

impl Pass<'_, '_> {
    /// A claim: its form checks (thresholds, confidence parameters), then,
    /// when it is well formed, its resolution and typing. A claim that
    /// checks is recorded on the model; one that does not records its
    /// refusals.
    pub(super) fn claim(
        &mut self,
        index: usize,
        form: &ProbabilisticForm,
        cx: &mut CheckContext<'_, Scope>,
        nodes: &mut u64,
    ) {
        let causes = super::claim(form);
        if !causes.is_empty() {
            self.refusals.extend(causes.into_iter().map(refusal));
            return;
        }
        let mut refusals = Vec::new();
        match self.resolve_claim(index, form, cx, nodes, &mut refusals) {
            Some(checked) => self.model.claims.push(checked),
            None => self.refusals.extend(refusals),
        }
    }

    fn resolve_claim(
        &mut self,
        index: usize,
        form: &ProbabilisticForm,
        cx: &mut CheckContext<'_, Scope>,
        nodes: &mut u64,
        refusals: &mut Vec<CheckRefusal>,
    ) -> Option<CheckedProbabilisticClaim> {
        let subject = keep(refusals, self.subject(form));
        let profile = keep(refusals, self.profile(form));
        if let ActivationForm::Each { parameter, .. } = &form.activation {
            refusals.push(refusal(ProbabilisticCause::NotYetImplemented {
                claim: form.name.name.clone(),
                construct: "an `on each` activation",
                span: parameter.name.span,
            }));
        }
        let subject = subject?;
        let scheduler = keep(refusals, self.scheduler(form, &subject));
        let over = keep(refusals, self.over(form))?;
        let claim = ClaimCx {
            index,
            form,
            subject: &subject,
            over: over.as_ref(),
            expressions: claim_expressions(form),
            exact_values: claim_exact_values(form),
        };
        let confidence = keep(refusals, self.confidence(&claim));
        let body = self.form(&claim, profile.as_ref(), cx, nodes, refusals);
        let (scheduler, fairness) = scheduler?;
        let (profile, form_checked) = (profile?, body?);
        let confidence = confidence?;
        if !refusals.is_empty() {
            return None;
        }
        let normal_form = keep(
            refusals,
            self.normal_form(
                &claim,
                &profile,
                &scheduler,
                &fairness,
                &form_checked,
                confidence.as_ref(),
            ),
        )?;
        Some(CheckedProbabilisticClaim {
            name: form.name.name.clone(),
            span: form.span,
            subject,
            profile,
            scheduler,
            fairness,
            form: form_checked,
            confidence,
            over,
            normal_form,
            declared: index,
        })
    }

    /// The model the claim is over (FR-186 "Kind and subject"). A name that
    /// is no `model` alias but declares a type is a trace subject, whose
    /// observed statistics are window aggregates (FR-194).
    fn subject(&self, form: &ProbabilisticForm) -> Result<ModelSubjectRef, CheckRefusal> {
        let alias = form.subject.name.as_str();
        let claim = form.name.name.clone();
        let span = form.subject.span;
        match self
            .input
            .models
            .iter()
            .find(|model| model.alias() == Some(alias))
        {
            Some(model) => Ok(ModelSubjectRef {
                alias: alias.to_owned(),
                package: model.selection().clone(),
            }),
            None if self.input.scope.declares_type(alias) => {
                Err(refusal(ProbabilisticCause::TraceSubject {
                    claim,
                    subject: alias.to_owned(),
                    span,
                }))
            }
            None => Err(refusal(ProbabilisticCause::UnresolvedSubject {
                claim,
                subject: alias.to_owned(),
                span,
            })),
        }
    }

    /// The temporal profile the claim's `using` alias selects.
    fn profile(&self, form: &ProbabilisticForm) -> Result<DefinitionRef, CheckRefusal> {
        let claim = form.name.name.clone();
        let span = form.profile.span;
        let selection = self
            .input
            .profiles
            .iter()
            .find(|selection| selection.alias == form.profile.alias)
            .ok_or_else(|| {
                refusal(ProbabilisticCause::UnresolvedProfile {
                    claim: claim.clone(),
                    alias: form.profile.alias.clone(),
                    span,
                })
            })?;
        let profile = TemporalProfile::from_identity(&selection.identity).ok_or_else(|| {
            refusal(ProbabilisticCause::NotTemporalProfile {
                claim: claim.clone(),
                profile: selection.identity.clone(),
                span,
            })
        })?;
        if profile == TemporalProfile::Timed {
            return Err(refusal(ProbabilisticCause::TimedEventProfile { claim, span }));
        }
        DefinitionRef::new(CLAUSE_PROFILE_AUTHORITY, profile.identity())
            .map_err(|_| broken("profile-identity-valid"))
    }

    /// The claim's scheduler (FR-186 "Kind and subject"): a workload of the
    /// subject's model, or `every scheduler` with its fairness set.
    fn scheduler(
        &self,
        form: &ProbabilisticForm,
        subject: &ModelSubjectRef,
    ) -> Result<(ProbScheduler, Vec<FairnessConstraint>), CheckRefusal> {
        let claim = form.name.name.clone();
        match &form.scheduler {
            SchedulerForm::Workload(name) => {
                let workload = self.model.workload(&name.name).ok_or_else(|| {
                    refusal(ProbabilisticCause::UnresolvedWorkload {
                        claim: claim.clone(),
                        workload: name.name.clone(),
                        span: name.span,
                    })
                })?;
                if workload.model() != subject.alias {
                    return Err(refusal(ProbabilisticCause::ForeignWorkload {
                        claim,
                        workload: name.name.clone(),
                        model: workload.model().to_owned(),
                        span: name.span,
                    }));
                }
                Ok((ProbScheduler::Workload(name.name.clone()), Vec::new()))
            }
            SchedulerForm::Every { fairness } => {
                let mut set: Vec<FairnessConstraint> = Vec::new();
                let mut refused = None;
                for written in fairness {
                    match self
                        .input
                        .operation(&written.operation, Some(&subject.alias))
                    {
                        Ok((operation, _)) => {
                            let constraint = checked_constraint(
                                written,
                                FairnessOperation::new(
                                    operation.declaring(),
                                    operation.name().to_owned(),
                                ),
                            );
                            if !set.contains(&constraint) {
                                set.push(constraint);
                            }
                        }
                        Err(fault) => {
                            refused.get_or_insert(refusal(super::model::unresolved(
                                fault,
                                &written.operation,
                            )));
                        }
                    }
                }
                match refused {
                    Some(refused) => Err(refused),
                    None => Ok((ProbScheduler::Every, set)),
                }
            }
        }
    }

    /// The claim's `over` binding, its type resolved.
    fn over(&self, form: &ProbabilisticForm) -> Result<Option<OverBinding>, CheckRefusal> {
        let Some(parameter) = &form.over else {
            return Ok(None);
        };
        let value_type =
            super::super::type_form::resolve_form(self.input.scope, &parameter.value_type)
                .map_err(|error| {
                    temporal_refusal(TemporalCause::ParameterType {
                        parameter: parameter.name.name.clone(),
                        fault: error.fault,
                        span: error.span,
                    })
                })?;
        if !matches!(value_type, ValueType::Reference(_)) {
            return Err(refusal(ProbabilisticCause::OverNotObjectType {
                claim: form.name.name.clone(),
                parameter: parameter.name.name.clone(),
                span: parameter.name.span,
            }));
        }
        Ok(Some(OverBinding {
            name: parameter.name.name.clone(),
            value_type,
        }))
    }

    /// The confidence parameters the claim states, each an exact rational
    /// under the written-value bound; their ranges were checked with the
    /// form.
    fn confidence(&self, claim: &ClaimCx<'_>) -> Result<Option<Confidence>, CheckRefusal> {
        use qsl_forms::ConfidenceKey::{Alpha, Beta, Indifference};
        let form = claim.form;
        if form.confidence.is_empty() {
            return Ok(None);
        }
        let value = |key| {
            let item = form
                .confidence
                .iter()
                .find(|item| item.key == key)
                .ok_or_else(|| broken("confidence-states-every-parameter"))?;
            self.exact(claim, &item.value)
        };
        Ok(Some(Confidence {
            alpha: value(Alpha)?,
            beta: value(Beta)?,
            indifference: value(Indifference)?,
        }))
    }

    /// `value` reduced under the written-value bound (FR-185 "Exact values").
    fn exact(&self, claim: &ClaimCx<'_>, value: &ExactValueForm) -> Result<Rational, CheckRefusal> {
        let location = claim.exact_location(value)?;
        let ceiling = self.input.limits.rational_bits();
        exact::value(value, ceiling)
            .map_err(|fault| exact_refusal(fault, ceiling, value.span, location))
    }

    /// The claim's form, typed. Every refusal goes onto `refusals`.
    fn form(
        &mut self,
        claim: &ClaimCx<'_>,
        profile: Option<&DefinitionRef>,
        cx: &mut CheckContext<'_, Scope>,
        nodes: &mut u64,
        refusals: &mut Vec<CheckRefusal>,
    ) -> Option<ProbForm> {
        match &claim.form.body {
            ProbabilisticBody::Probability {
                op,
                threshold,
                event,
            } => {
                let bound = keep(refusals, self.exact(claim, threshold))
                    .map(|bound| Bound::new(*op, bound));
                if let (EventForm::Temporal(formula), true) = (event, is_infinite(profile)) {
                    if let Some(shape) = reach_shape(formula) {
                        let target = self.reach(claim, formula, shape, cx, nodes, refusals);
                        return Some(ProbForm::Reach {
                            bound: bound?,
                            target: target?,
                        });
                    }
                }
                let event = self.event(claim, profile, event, cx, nodes, refusals);
                Some(ProbForm::Probability {
                    bound: bound?,
                    event: event?,
                })
            }
            ProbabilisticBody::Quantile {
                level,
                measure,
                op,
                threshold,
            } => {
                let q = keep(refusals, self.exact(claim, level));
                let measured = keep(refusals, self.measure(claim, measure, cx, nodes));
                let (measure, measured) = measured?;
                let threshold = keep(
                    refusals,
                    self.threshold(claim, threshold, measured, cx, nodes),
                )
                .map(|value| Bound::new(*op, value));
                Some(ProbForm::Quantile {
                    q: q?,
                    measure,
                    threshold: threshold?,
                })
            }
            ProbabilisticBody::MeanOfFraction {
                fraction,
                op,
                threshold,
            } => {
                let bound = keep(refusals, self.exact(claim, threshold))
                    .map(|bound| Bound::new(*op, bound));
                let fraction = keep(refusals, self.fraction(claim, fraction, cx, nodes));
                Some(ProbForm::MeanOfFraction {
                    bound: bound?,
                    fraction: fraction?,
                })
            }
            ProbabilisticBody::LongRunFraction {
                predicate,
                weighting,
                op,
                threshold,
            } => {
                let bound = keep(refusals, self.exact(claim, threshold))
                    .map(|bound| Bound::new(*op, bound));
                let predicate = keep(refusals, self.predicate(claim, predicate, cx, nodes));
                let weight = keep(refusals, self.weight(claim, weighting.as_ref()));
                Some(ProbForm::LongRunFraction {
                    bound: bound?,
                    predicate: predicate?,
                    weight: weight?,
                })
            }
            ProbabilisticBody::ExpectedUntil {
                quantity,
                target,
                op,
                threshold,
            } => match quantity {
                ExpectedQuantity::Elapsed => {
                    // No subject is timed yet (the timed facet is not
                    // selectable), so no `elapsed` quantity exists to sum.
                    refusals.push(refusal(ProbabilisticCause::NotYetImplemented {
                        claim: claim.claim().to_owned(),
                        construct: "an `expected elapsed` claim",
                        span: root_span(target, claim.form.span),
                    }));
                    None
                }
                ExpectedQuantity::Accumulate(name) => {
                    let measured = keep(refusals, self.reward_named(claim, &name.name, name.span));
                    let until = keep(refusals, self.predicate(claim, target, cx, nodes));
                    let (reward, value) = measured?;
                    let bound = keep(
                        refusals,
                        self.threshold(claim, threshold, Measured::Expectation(value), cx, nodes),
                    )
                    .map(|value| Bound::new(*op, value));
                    Some(ProbForm::ExpectedReward {
                        bound: bound?,
                        reward,
                        until: until?,
                    })
                }
            },
        }
    }

    /// An unbounded reachability claim's state predicates (ADR-028 XF-4):
    /// each operand of the one operator is a `holds(P)`. A nested temporal
    /// operator is not a state predicate (FR-195). A constant (`eventually
    /// true`) is refused too, which FR-195 does not say: it has no checked
    /// predicate to label the states with. A judgment call, open for the spec.
    fn reach(
        &mut self,
        claim: &ClaimCx<'_>,
        formula: &TemporalFormulaForm,
        shape: ReachShape,
        cx: &mut CheckContext<'_, Scope>,
        nodes: &mut u64,
        refusals: &mut Vec<CheckRefusal>,
    ) -> Option<ReachTarget> {
        let mut predicates = Vec::with_capacity(shape.operands.len());
        for operand in shape.operands {
            match formula.node(*operand) {
                Some(TemporalNodeForm::Holds { condition, .. }) => {
                    predicates.push(keep(refusals, self.predicate(claim, condition, cx, nodes)));
                }
                Some(
                    TemporalNodeForm::Operator(TemporalOperatorForm { span, .. })
                    | TemporalNodeForm::Constant { span, .. },
                ) => {
                    predicates.push(None);
                    refusals.push(refusal(ProbabilisticCause::NotStatePredicate {
                        claim: claim.claim().to_owned(),
                        span: *span,
                    }));
                }
                None => {
                    refusals.push(broken("reach-operand-is-in-the-formula"));
                    return None;
                }
            }
        }
        let mut predicates = predicates.into_iter();
        match (shape.kind, predicates.next()?, predicates.next()) {
            (ReachKind::Eventually, Some(target), None) => Some(ReachTarget::Eventually(target)),
            (ReachKind::Always, Some(target), None) => Some(ReachTarget::Always(target)),
            (ReachKind::Until, Some(left), Some(Some(right))) => {
                Some(ReachTarget::Until(left, right))
            }
            _ => None,
        }
    }

    /// A claim's event (FR-186 "Events and measures").
    fn event(
        &mut self,
        claim: &ClaimCx<'_>,
        profile: Option<&DefinitionRef>,
        event: &EventForm,
        cx: &mut CheckContext<'_, Scope>,
        nodes: &mut u64,
        refusals: &mut Vec<CheckRefusal>,
    ) -> Option<Event> {
        match event {
            EventForm::Temporal(formula) => {
                // A claim whose profile was refused is refused for that alone.
                let bounded = profile.is_none_or(|profile| {
                    TemporalProfile::from_identity(profile.identity())
                        .is_some_and(TemporalProfile::is_bounded)
                });
                let (formula, horizon) =
                    self.formula(claim, formula, bounded, cx, nodes, refusals)?;
                Some(Event::Formula { formula, horizon })
            }
            EventForm::Compare {
                measure,
                op,
                threshold,
            } => {
                let measured = keep(refusals, self.measure(claim, measure, cx, nodes));
                let (measure, measured) = measured?;
                let value = keep(
                    refusals,
                    self.threshold(claim, threshold, measured, cx, nodes),
                )?;
                Some(Event::Compare {
                    measure,
                    bound: Bound::new(*op, value),
                })
            }
        }
    }

    /// A temporal event: bounded, with only interval operators (ADR-018
    /// TP-2), its horizon and the predicate of each `holds` atom.
    fn formula(
        &mut self,
        claim: &ClaimCx<'_>,
        formula: &TemporalFormulaForm,
        bounded: bool,
        cx: &mut CheckContext<'_, Scope>,
        nodes: &mut u64,
        refusals: &mut Vec<CheckRefusal>,
    ) -> Option<(CheckedTemporalFormula, u64)> {
        let mut causes: Vec<TemporalCause> = Vec::new();
        let operators = match checked_operators(
            formula,
            &IntervalReading::Steps { bounded: true },
            &mut causes,
        ) {
            Ok((operators, _)) => operators,
            Err(_) => {
                refusals.push(broken("event-operators-checked"));
                return None;
            }
        };
        let root = match formula.node(formula.root()) {
            Some(TemporalNodeForm::Operator(operator)) => operator.span,
            Some(
                TemporalNodeForm::Holds { span, .. } | TemporalNodeForm::Constant { span, .. },
            ) => *span,
            None => claim.form.span,
        };
        if !bounded {
            refusals.push(refusal(ProbabilisticCause::UnboundedEvent {
                claim: claim.claim().to_owned(),
                span: root,
            }));
        }
        let before = refusals.len();
        for cause in causes {
            refusals.push(match cause {
                TemporalCause::MissingInterval { span } | TemporalCause::OpenInterval { span } => {
                    refusal(ProbabilisticCause::UnboundedEvent {
                        claim: claim.claim().to_owned(),
                        span,
                    })
                }
                other => temporal_refusal(other),
            });
        }
        let mut conditions = BTreeMap::new();
        for (id, node) in formula.iter() {
            let TemporalNodeForm::Holds { condition, .. } = node else {
                continue;
            };
            if let Some(predicate) = keep(refusals, self.predicate(claim, condition, cx, nodes)) {
                conditions.insert(id.index(), predicate);
            }
        }
        if refusals.len() > before || !bounded {
            return None;
        }
        let intervals = operators
            .iter()
            .map(|operator| (operator.node, operator.interval.clone()))
            .collect();
        let checked = match build_formula(formula, &intervals) {
            Ok(checked) => checked,
            Err(_) => {
                refusals.push(broken("event-formula-built"));
                return None;
            }
        };
        let horizon = match horizon(formula) {
            Ok(horizon) => horizon,
            Err(HorizonFault::Overflow { span, deepest, own }) => {
                let operator = formula.iter().find_map(|(id, node)| match node {
                    TemporalNodeForm::Operator(operator) if operator.operator_span == span => {
                        Some(id.index())
                    }
                    _ => None,
                });
                refusals.push(match operator {
                    Some(operator) => super::horizon_refusal(
                        u128::from(deepest) + u128::from(own),
                        site_location(claim.index, Site::Operator(operator)),
                    ),
                    None => broken("overflowing-operator-is-in-the-event"),
                });
                return None;
            }
            Err(HorizonFault::Broken(invariant)) => {
                refusals.push(broken(invariant));
                return None;
            }
        };
        Some((
            CheckedTemporalFormula {
                formula: checked,
                conditions,
                normal_form: encode_event(formula, claim.binder()),
            },
            horizon,
        ))
    }

    /// A bounded measure, and what its threshold is measured in.
    fn measure(
        &mut self,
        claim: &ClaimCx<'_>,
        measure: &MeasureForm,
        cx: &mut CheckContext<'_, Scope>,
        nodes: &mut u64,
    ) -> Result<(BoundedMeasure, Measured), CheckRefusal> {
        match measure {
            MeasureForm::Accumulate {
                reward,
                window,
                span: _,
            } => {
                let (name, value) = self.reward_named(claim, &reward.name, reward.span)?;
                let (from, until, within) = self.window(claim, window, cx, nodes)?;
                Ok((
                    BoundedMeasure::Accumulate {
                        reward: name,
                        from,
                        until,
                        within,
                    },
                    Measured::Reward(value),
                ))
            }
            MeasureForm::Steps { window, span: _ } => {
                let (from, until, within) = self.window(claim, window, cx, nodes)?;
                Ok((
                    BoundedMeasure::Steps {
                        from,
                        until,
                        within,
                    },
                    Measured::Steps,
                ))
            }
            MeasureForm::Elapsed { span, .. } => {
                Err(refusal(ProbabilisticCause::NotYetImplemented {
                    claim: claim.claim().to_owned(),
                    construct: "an `elapsed` measure",
                    span: *span,
                }))
            }
            MeasureForm::Fraction(fraction) => Ok((
                BoundedMeasure::Fraction(self.fraction(claim, fraction, cx, nodes)?),
                Measured::Fraction,
            )),
        }
    }

    /// `from holds(A) until holds(B) within h`.
    fn window(
        &mut self,
        claim: &ClaimCx<'_>,
        window: &MeasureWindowForm,
        cx: &mut CheckContext<'_, Scope>,
        nodes: &mut u64,
    ) -> Result<(CheckedPredicate, CheckedPredicate, u64), CheckRefusal> {
        let from = self.predicate(claim, &window.from, cx, nodes);
        let until = self.predicate(claim, &window.until, cx, nodes);
        let within = self.steps(claim, &window.within);
        Ok((from?, until?, within?))
    }

    /// `fraction holds(P) over [0, h] [weighted by R]`.
    fn fraction(
        &mut self,
        claim: &ClaimCx<'_>,
        fraction: &FractionForm,
        cx: &mut CheckContext<'_, Scope>,
        nodes: &mut u64,
    ) -> Result<FractionMeasure, CheckRefusal> {
        let predicate = self.predicate(claim, &fraction.predicate, cx, nodes);
        let window = self.steps(claim, &fraction.horizon);
        let weight = self.weight(claim, fraction.weighting.as_ref());
        Ok(FractionMeasure {
            predicate: predicate?,
            window: window?,
            weight: weight?,
        })
    }

    /// A window's upper end, a natural number of steps. A time quantity is
    /// a timed window (FR-393), which no S3 check reads yet.
    fn steps(&self, claim: &ClaimCx<'_>, horizon: &HorizonForm) -> Result<u64, CheckRefusal> {
        match horizon {
            HorizonForm::Steps(steps) => Ok(steps.value),
            HorizonForm::Quantity(quantity) => {
                Err(refusal(ProbabilisticCause::NotYetImplemented {
                    claim: claim.claim().to_owned(),
                    construct: "a timed window",
                    span: quantity.span,
                }))
            }
        }
    }

    /// The reward a `weighted by R` names, when one is written.
    fn weight(
        &self,
        claim: &ClaimCx<'_>,
        weighting: Option<&WeightingForm>,
    ) -> Result<Option<Identifier>, CheckRefusal> {
        weighting
            .map(|weighting| {
                self.reward_named(claim, &weighting.name.name, weighting.name.span)
                    .map(|(name, _)| name)
            })
            .transpose()
    }

    /// The reward `name`: one the subject's model declares on at least one
    /// operation. Every declaration of it has one value type.
    fn reward_named(
        &self,
        claim: &ClaimCx<'_>,
        name: &str,
        span: Span,
    ) -> Result<(Identifier, RewardValue), CheckRefusal> {
        let mut declared = self
            .model
            .rewards
            .iter()
            .filter(|reward| {
                reward.name() == name && reward.operation().model() == claim.subject.alias
            })
            .map(|reward| match reward.value_type() {
                ValueType::Quantity(unit) => RewardValue::Quantity(*unit),
                _ => RewardValue::Integer,
            });
        let Some(value) = declared.next() else {
            return Err(refusal(ProbabilisticCause::UnresolvedReward {
                claim: claim.claim().to_owned(),
                reward: name.to_owned(),
                span,
            }));
        };
        if declared.any(|other| other != value) {
            return Err(refusal(ProbabilisticCause::AmbiguousReward {
                claim: claim.claim().to_owned(),
                reward: name.to_owned(),
                span,
            }));
        }
        let identifier = Identifier::new(name).map_err(|_| broken("reward-name-is-identifier"))?;
        Ok((identifier, value))
    }

    /// A state predicate, `holds(P)`'s `P`: a Boolean clause over the claim's
    /// `over` parameter, typed as an invariant body is, reading no `self`.
    fn predicate(
        &mut self,
        claim: &ClaimCx<'_>,
        expression: &ExpressionForm,
        cx: &mut CheckContext<'_, Scope>,
        nodes: &mut u64,
    ) -> Result<CheckedPredicate, CheckRefusal> {
        let location = claim.location(expression)?;
        let bound: Vec<(String, ValueType)> = claim
            .over
            .map(|over| (over.name.clone(), over.value_type.clone()))
            .into_iter()
            .collect();
        let input = self.input;
        let mut typer = Typer::new(
            input.scope,
            input.signatures,
            input.limits,
            nodes,
            ClauseKind::Invariant,
        )
        .with_cancel(Some(input.cancel))
        .with_context(cx.reborrow(input.scope));
        bind_parameters(&mut typer, &bound, &location)?;
        typer.enter_state_clause(StateContext {
            parent_projections: BTreeMap::new(),
            kind: StateClauseKind::Invariant,
            operation: None,
            self_slot: NO_SLOT,
            result_slot: None,
        });
        let body = typer.infer(&expression.expression, Some(&ValueType::Boolean), &location)?;
        if body.root().value_type() != &ValueType::Boolean {
            return Err(CheckRefusal {
                location,
                cause: CheckCause::NonBooleanRoot,
            });
        }
        let slots = typer.slots();
        let observations = Observations::of(body.root(), Observation::Current, bound.len());
        Definedness::new(
            bound.len(),
            input.dispatch_tables,
            &input.scope.dispatch_operations,
        )
        .with_observations(&observations)
        .check(body.root())?;
        Ok(CheckedPredicate {
            body,
            slots,
            normal_form: encode_predicate(&expression.expression, claim.binder()),
        })
    }

    /// A comparison's threshold, a literal of the type of what it bounds
    /// (FR-186 "Events and measures", "Thresholds and units"). A quantity in
    /// another unit of the reward's dimension converts exactly (QSpec
    /// FR-142); one of another dimension refuses.
    fn threshold(
        &mut self,
        claim: &ClaimCx<'_>,
        expression: &ExpressionForm,
        measured: Measured,
        cx: &mut CheckContext<'_, Scope>,
        nodes: &mut u64,
    ) -> Result<Rational, CheckRefusal> {
        let location = claim.location(expression)?;
        let span = root_span(expression, claim.form.span);
        let input = self.input;
        let hint = match measured {
            Measured::Fraction | Measured::Expectation(_) => Some(
                fraction_type(self.input.limits.rational_bits())
                    .ok_or_else(|| broken("fraction-type-valid"))?,
            ),
            Measured::Steps | Measured::Reward(_) => None,
        };
        let body = input
            .typer(nodes, cx)
            .infer(&expression.expression, hint.as_ref(), &location)
            // A `rational(n, d)` has no type to take where the measure is
            // not a fraction: that is a threshold of the wrong type.
            .map_err(|refused| match refused.cause {
                CheckCause::IllTyped(IllTypedCause::AmbiguousLiteral) => {
                    CheckRefusal::ill_typed(&location, IllTypedCause::TypeMismatch)
                }
                _ => refused,
            })?;
        let mismatch = || CheckRefusal::ill_typed(&location, IllTypedCause::TypeMismatch);
        let Some(literal) = literal(body.root()) else {
            return Err(refusal(ProbabilisticCause::ThresholdNotLiteral {
                claim: claim.claim().to_owned(),
                span,
            }));
        };
        match (measured, literal.0) {
            (Measured::Steps | Measured::Reward(RewardValue::Integer), Value::Integer(steps))
                if !steps.is_negative() =>
            {
                Ok(Rational::from_integer(steps))
            }
            (Measured::Fraction, value) => {
                let rational = match value {
                    Value::Rational(rational) => rational,
                    Value::Decimal(decimal) => decimal.representation().to_rational(),
                    Value::Integer(integer) => Rational::from_integer(integer),
                    _ => return Err(mismatch()),
                };
                let above_zero = !rational.is_zero() && !rational.numerator().is_negative();
                if above_zero && rational.numerator() < rational.denominator() {
                    Ok(rational)
                } else {
                    Err(refusal(ProbabilisticCause::FractionBound {
                        claim: claim.claim().to_owned(),
                        span,
                    }))
                }
            }
            (Measured::Expectation(RewardValue::Integer), value) => match value {
                Value::Rational(rational) => Ok(rational),
                Value::Decimal(decimal) => Ok(decimal.representation().to_rational()),
                Value::Integer(integer) => Ok(Rational::from_integer(integer)),
                Value::Quantity(quantity) => {
                    Err(self.unit_mismatch(claim, quantity.unit(), None, span))
                }
                _ => Err(mismatch()),
            },
            (
                Measured::Reward(RewardValue::Quantity(unit))
                | Measured::Expectation(RewardValue::Quantity(unit)),
                Value::Quantity(quantity),
            ) => self.convert(claim, &quantity, unit, span),
            (Measured::Reward(RewardValue::Integer), Value::Quantity(quantity)) => {
                Err(self.unit_mismatch(claim, quantity.unit(), None, span))
            }
            _ => Err(mismatch()),
        }
    }

    /// `quantity` in the reward's `unit`, exactly.
    fn convert(
        &self,
        claim: &ClaimCx<'_>,
        quantity: &Quantity,
        unit: UnitId,
        span: Span,
    ) -> Result<Rational, CheckRefusal> {
        let units = self.input.scope.types().units();
        let (Some(from), Some(target)) = (units.resolve(quantity), units.get(unit)) else {
            return Err(broken("threshold-units-declared"));
        };
        if !from.unit().converts_to(target) {
            return Err(self.unit_mismatch(claim, quantity.unit(), Some(unit), span));
        }
        let mut meter = Meter::new(super::super::family::SCALAR_LIMITS_UNLIMITED);
        let outcome = convert_quantity(from, target, &QuantityTarget::Exact, &mut meter)
            .map_err(|_| broken("threshold-converts-within-its-dimension"))?;
        match outcome {
            quire_exact::Outcome::Completed(conversion) => match conversion.value() {
                ConvertedValue::Exact(value) => Ok(value.clone()),
                _ => Err(broken("exact-conversion-is-exact")),
            },
            _ => Err(broken("exact-conversion-completes")),
        }
    }

    /// A threshold whose dimension is not the reward's, naming both.
    fn unit_mismatch(
        &self,
        claim: &ClaimCx<'_>,
        threshold: UnitId,
        reward: Option<UnitId>,
        span: Span,
    ) -> CheckRefusal {
        refusal(ProbabilisticCause::UnitMismatch {
            claim: claim.claim().to_owned(),
            threshold: self.dimension_name(Some(threshold)),
            reward: self.dimension_name(reward),
            span,
        })
    }

    /// The name of `unit`'s dimension, or `dimensionless`.
    fn dimension_name(&self, unit: Option<UnitId>) -> String {
        let units = self.input.scope.types().units();
        unit.and_then(|unit| match units.get(unit)? {
            quire_semantic_value::quantity::QuantityUnit::Declared(declared) => {
                let node = units.nominal_node(declared.dimension_node().as_bytes())?;
                node.declaration.qualified_declaration.last().cloned()
            }
            quire_semantic_value::quantity::QuantityUnit::Compound(_) => None,
        })
        .unwrap_or_else(|| "dimensionless".to_owned())
    }

    /// The claim's normal form: what its S4 claim node will be keyed over.
    fn normal_form(
        &self,
        claim: &ClaimCx<'_>,
        profile: &DefinitionRef,
        scheduler: &ProbScheduler,
        fairness: &[FairnessConstraint],
        form: &ProbForm,
        confidence: Option<&Confidence>,
    ) -> Result<ClaimNormalForm, CheckRefusal> {
        let subject = &claim.subject.package;
        let text = |text: &str| text.as_bytes().to_vec();
        let subject = node_bytes(
            b"subject",
            &[
                text(&claim.subject.alias),
                text(&subject.identity),
                text(&subject.version),
                subject.digest.to_vec(),
            ],
        );
        let profile = node_bytes(
            b"profile",
            &[text(profile.authority()), text(profile.identity())],
        );
        let scheduler = match scheduler {
            ProbScheduler::Workload(name) => self.workload_bytes(name)?,
            ProbScheduler::Every => node_bytes(b"every", &[]),
        };
        let mut fairness: Vec<Vec<u8>> = fairness
            .iter()
            .map(|constraint| self.fairness_bytes(&claim.subject.alias, constraint))
            .collect::<Result<_, _>>()?;
        fairness.sort();
        // The binder's name is not content: the predicates encode it as a
        // binder, and only its type is bound here.
        let over = match claim.over {
            None => node_bytes(b"no-over", &[]),
            Some(over) => node_bytes(b"over", &[self.type_bytes(&over.value_type)?]),
        };
        let confidence = confidence.map_or_else(
            || node_bytes(b"no-confidence", &[]),
            |confidence| {
                node_bytes(
                    b"confidence",
                    &[
                        rational_bytes(&confidence.alpha),
                        rational_bytes(&confidence.beta),
                        rational_bytes(&confidence.indifference),
                    ],
                )
            },
        );
        Ok(ClaimNormalForm(node_bytes(
            b"probabilistic-satisfaction",
            &[
                subject,
                profile,
                scheduler,
                node_bytes(b"fairness", &fairness),
                over,
                self.form_bytes(&claim.subject.alias, form),
                confidence,
            ],
        )))
    }

    /// A workload's content: its name, its model and each weight, the
    /// weights in operation order (the order they are written in is not
    /// content).
    fn workload_bytes(&self, name: &str) -> Result<Vec<u8>, CheckRefusal> {
        let workload = self
            .model
            .workload(name)
            .ok_or_else(|| broken("claim-workload-is-recorded"))?;
        let mut weights: Vec<Vec<u8>> = workload
            .weights()
            .iter()
            .map(|(operation, weight)| {
                node_bytes(
                    b"weight",
                    &[
                        operation.key().node.as_bytes().to_vec(),
                        rational_bytes(weight),
                    ],
                )
            })
            .collect();
        weights.sort();
        Ok(node_bytes(
            b"workload",
            &[
                name.as_bytes().to_vec(),
                workload.model().as_bytes().to_vec(),
                node_bytes(b"weights", &weights),
            ],
        ))
    }

    fn fairness_bytes(
        &self,
        alias: &str,
        constraint: &FairnessConstraint,
    ) -> Result<Vec<u8>, CheckRefusal> {
        let operation = constraint.operation();
        let declaring = self
            .input
            .models
            .iter()
            .filter(|model| model.alias() == Some(alias))
            .find_map(|model| model.declaration_key(operation.declaring_type()))
            .ok_or_else(|| broken("fairness-operation-is-declared"))?;
        let kind: &[u8] = match constraint.kind() {
            FairnessKind::Weak => b"weak",
            FairnessKind::Strong => b"strong",
        };
        let granularity: &[u8] = match constraint.granularity() {
            FairnessGranularity::Whole => b"whole",
            FairnessGranularity::Each => b"each",
        };
        Ok(node_bytes(
            b"constraint",
            &[
                kind.to_vec(),
                granularity.to_vec(),
                declaring.node.as_bytes().to_vec(),
                operation.name().as_bytes().to_vec(),
            ],
        ))
    }

    /// An `over` type's content: an object type, by its declaration in its
    /// model. No other type has an encoding ([`Self::over`] refuses it).
    fn type_bytes(&self, value_type: &ValueType) -> Result<Vec<u8>, CheckRefusal> {
        let ValueType::Reference(object) = value_type else {
            return Err(broken("over-type-is-an-object-type"));
        };
        let key = self
            .input
            .models
            .iter()
            .find_map(|model| model.declaration_key(*object))
            .ok_or_else(|| broken("over-type-is-declared"))?;
        Ok(node_bytes(b"reference", &[key.node.as_bytes().to_vec()]))
    }

    fn reward_bytes(&self, subject: &str, name: &Identifier) -> Vec<u8> {
        let mut operations: Vec<Vec<u8>> = self
            .model
            .rewards
            .iter()
            .filter(|reward| {
                reward.name() == name.as_str() && reward.operation().model() == subject
            })
            .map(|reward| reward.operation().key().node.as_bytes().to_vec())
            .collect();
        operations.sort();
        node_bytes(
            b"reward",
            &[
                name.as_str().as_bytes().to_vec(),
                node_bytes(b"on", &operations),
            ],
        )
    }

    fn weight_bytes(&self, subject: &str, weight: Option<&Identifier>) -> Vec<u8> {
        weight.map_or_else(
            || node_bytes(b"unweighted", &[]),
            |name| self.reward_bytes(subject, name),
        )
    }

    fn fraction_bytes(&self, subject: &str, fraction: &FractionMeasure) -> Vec<u8> {
        node_bytes(
            b"fraction",
            &[
                fraction.predicate.normal_form.clone(),
                fraction.window.to_be_bytes().to_vec(),
                self.weight_bytes(subject, fraction.weight.as_ref()),
            ],
        )
    }

    fn measure_bytes(&self, subject: &str, measure: &BoundedMeasure) -> Vec<u8> {
        match measure {
            BoundedMeasure::Accumulate {
                reward,
                from,
                until,
                within,
            } => node_bytes(
                b"accumulate",
                &[
                    self.reward_bytes(subject, reward),
                    from.normal_form.clone(),
                    until.normal_form.clone(),
                    within.to_be_bytes().to_vec(),
                ],
            ),
            BoundedMeasure::Steps {
                from,
                until,
                within,
            } => node_bytes(
                b"steps",
                &[
                    from.normal_form.clone(),
                    until.normal_form.clone(),
                    within.to_be_bytes().to_vec(),
                ],
            ),
            BoundedMeasure::Fraction(fraction) => self.fraction_bytes(subject, fraction),
        }
    }

    fn form_bytes(&self, subject: &str, form: &ProbForm) -> Vec<u8> {
        match form {
            ProbForm::Probability { bound, event } => {
                let event = match event {
                    Event::Formula { formula, horizon } => node_bytes(
                        b"formula",
                        &[formula.normal_form.clone(), horizon.to_be_bytes().to_vec()],
                    ),
                    Event::Compare { measure, bound } => node_bytes(
                        b"compare",
                        &[self.measure_bytes(subject, measure), bound.bytes()],
                    ),
                };
                node_bytes(b"probability", &[bound.bytes(), event])
            }
            ProbForm::Quantile {
                q,
                measure,
                threshold,
            } => node_bytes(
                b"quantile",
                &[
                    rational_bytes(q),
                    self.measure_bytes(subject, measure),
                    threshold.bytes(),
                ],
            ),
            ProbForm::MeanOfFraction { bound, fraction } => node_bytes(
                b"mean-of-fraction",
                &[bound.bytes(), self.fraction_bytes(subject, fraction)],
            ),
            ProbForm::LongRunFraction {
                bound,
                predicate,
                weight,
            } => node_bytes(
                b"long-run-fraction",
                &[
                    bound.bytes(),
                    predicate.normal_form.clone(),
                    self.weight_bytes(subject, weight.as_ref()),
                ],
            ),
            ProbForm::Reach { bound, target } => {
                let target = match target {
                    ReachTarget::Eventually(target) => {
                        node_bytes(b"eventually", std::slice::from_ref(&target.normal_form))
                    }
                    ReachTarget::Until(left, right) => node_bytes(
                        b"until",
                        &[left.normal_form.clone(), right.normal_form.clone()],
                    ),
                    ReachTarget::Always(target) => {
                        node_bytes(b"always", std::slice::from_ref(&target.normal_form))
                    }
                };
                node_bytes(b"reach", &[bound.bytes(), target])
            }
            ProbForm::ExpectedReward {
                bound,
                reward,
                until,
            } => node_bytes(
                b"expected-reward",
                &[
                    bound.bytes(),
                    self.reward_bytes(subject, reward),
                    until.normal_form.clone(),
                ],
            ),
        }
    }
}
