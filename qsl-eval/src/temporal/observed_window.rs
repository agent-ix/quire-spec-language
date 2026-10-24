// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-194 (FR-107): the [`WindowSource`] that reads a window aggregate's
//! filter, value and predicate from observed snapshots, the one clause
//! evaluator filling the [`WindowSource`] seam as [`super::observed`] fills
//! [`super::TemporalLetters`].
//!
//! Each position of the trace is the object environment its snapshot
//! admitted. Each expression has one parameter, the object the clause's
//! `over` parameter is bound to, and is read through
//! [`CheckedPackageEvaluation::evaluate`] with that position's environment.

use qsl_package::CheckedPackage;
use qsl_semantics::check::CheckedExpression;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_eval::{Evaluation, FamilyOutcome, FamilyResult};
use quire_exact::{Meter, ObjectReference, Outcome, Rational, Value};
use quire_semantic_value::catalog::InternalFault;

use super::observed::HoldsStop;
use super::window::{WindowRead, WindowSource};
use super::{LetterStop, TemporalPosition, UndefinedCause};
use crate::value::CheckedPackageEvaluation;

/// The expressions of an aggregate term, each with the `over` object as its
/// one parameter.
#[derive(Clone, Copy, Debug, Default)]
pub struct ObservedWindowExpressions<'a> {
    /// The term's filter, a Boolean; absent admits every position.
    pub filter: Option<&'a CheckedExpression>,
    /// The term's value expression, a number; read for `sum`, `min`, `max`
    /// and `quantile`.
    pub value: Option<&'a CheckedExpression>,
    /// The predicate of `fraction holds(P)`, a Boolean.
    pub predicate: Option<&'a CheckedExpression>,
}

/// A [`WindowSource`] over the object environment of each position of an
/// observed trace.
#[derive(Debug)]
pub struct ObservedWindowSource<'a> {
    package: &'a CheckedPackage,
    environments: &'a [ObjectEnvironment],
    over: &'a ObjectReference,
    expressions: ObservedWindowExpressions<'a>,
    stopped: Option<HoldsStop>,
}

fn fault(code: &'static str) -> LetterStop {
    LetterStop::Fault(InternalFault::new("temporal.observed-window", code))
}

impl<'a> ObservedWindowSource<'a> {
    /// A source reading `expressions` of `package` at position `i` over
    /// `environments[i]`, with `over` the object the clause's `over`
    /// parameter is bound to.
    pub fn new(
        package: &'a CheckedPackage,
        environments: &'a [ObjectEnvironment],
        over: &'a ObjectReference,
        expressions: ObservedWindowExpressions<'a>,
    ) -> Self {
        Self {
            package,
            environments,
            over,
            expressions,
            stopped: None,
        }
    }

    /// Why an expression stopped the run, if one did; the stop's result is
    /// then a fault.
    pub fn stopped(&self) -> Option<&HoldsStop> {
        self.stopped.as_ref()
    }

    /// The outcome of `expression` at `position`.
    #[deny(clippy::wildcard_enum_match_arm)]
    fn read(
        &mut self,
        expression: &CheckedExpression,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<WindowRead<Value>, LetterStop> {
        let objects = usize::try_from(position.index())
            .ok()
            .and_then(|slot| self.environments.get(slot))
            .ok_or_else(|| fault("position-has-an-environment"))?;
        let evaluation: Evaluation = match self.package.evaluate(
            expression,
            vec![Value::Reference(self.over.clone())],
            objects,
            meter,
        ) {
            Ok(evaluation) => evaluation,
            Err(failure) => {
                self.stopped = Some(HoldsStop::Failed(failure));
                return Err(fault("window-expression-call-failed"));
            }
        };
        match evaluation.outcome {
            FamilyOutcome::Evaluated(Outcome::Completed(value)) => Ok(WindowRead::Defined(value)),
            FamilyOutcome::Evaluated(Outcome::Undefined(undefined)) => {
                Ok(WindowRead::Undefined(UndefinedCause::Kernel(undefined)))
            }
            FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause)) => Ok(
                WindowRead::Undefined(UndefinedCause::Family(cause.undefined_record())),
            ),
            FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) => {
                Err(LetterStop::Incomplete(incomplete))
            }
            FamilyOutcome::Evaluated(Outcome::Refused(_))
            | FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(_)) => {
                self.stopped = Some(HoldsStop::Refused(evaluation));
                Err(fault("window-expression-refused"))
            }
            #[cfg(seam_probe_eval_downstream)]
            FamilyOutcome::__SeamProbe => unreachable!("never constructed outside the probe build"),
        }
    }

    fn boolean(
        &mut self,
        expression: &CheckedExpression,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<WindowRead<bool>, LetterStop> {
        match self.read(expression, position, meter)? {
            WindowRead::Defined(Value::Boolean(held)) => Ok(WindowRead::Defined(held)),
            WindowRead::Defined(_) => Err(fault("window-condition-boolean")),
            WindowRead::Undefined(cause) => Ok(WindowRead::Undefined(cause)),
        }
    }
}

impl WindowSource for ObservedWindowSource<'_> {
    fn admits(
        &mut self,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<WindowRead<bool>, LetterStop> {
        match self.expressions.filter {
            Some(filter) => self.boolean(filter, position, meter),
            None => Ok(WindowRead::Defined(true)),
        }
    }

    fn value(
        &mut self,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<WindowRead<Rational>, LetterStop> {
        let expression = self
            .expressions
            .value
            .ok_or_else(|| fault("aggregate-has-a-value-expression"))?;
        match self.read(expression, position, meter)? {
            WindowRead::Defined(Value::Rational(value)) => Ok(WindowRead::Defined(value)),
            WindowRead::Defined(Value::Integer(value)) => {
                Ok(WindowRead::Defined(Rational::from_integer(value)))
            }
            WindowRead::Defined(_) => Err(fault("window-value-is-a-number")),
            WindowRead::Undefined(cause) => Ok(WindowRead::Undefined(cause)),
        }
    }

    fn predicate(
        &mut self,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<WindowRead<bool>, LetterStop> {
        let expression = self
            .expressions
            .predicate
            .ok_or_else(|| fault("aggregate-has-a-predicate-expression"))?;
        self.boolean(expression, position, meter)
    }
}
