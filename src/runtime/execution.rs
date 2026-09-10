// SPDX-License-Identifier: AGPL-3.0-only
//! FR-023: native validation and execution with retained request provenance.

use crate::package::NativePackage;

use super::validation::{validate_retaining, FailedValidation};
use super::{
    evaluate, EvaluationLimits, EvaluationOutcome, EvaluationUsage, ExecutionSelection,
    ImplicationEvent, RuntimeInput, ValidationLimits, ValidationReport, ValidationUsage,
};

/// Independent caller-lowered budgets for the existing runtime stages.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ExecutionLimits {
    /// Offered artifact and model-aware validation limits.
    pub validation: ValidationLimits,
    /// Reference predicate execution limits.
    pub evaluation: EvaluationLimits,
}

/// The actual stage result; failed validation has no evaluation observations.
#[derive(Debug)]
pub enum ExecutionOutcome {
    /// Original failed validation, including details, terminal reason and usage.
    ValidationFailed(Box<ValidationReport>),
    /// Actual reference execution after complete validation.
    Evaluated {
        /// Concrete truth, incompleteness or a defensive invariant refusal.
        result: EvaluationOutcome,
        /// Actual admitted evaluation work, excluding validation.
        usage: EvaluationUsage,
        /// Actual ordered implication event prefix, including on a stopped run.
        events: Vec<ImplicationEvent>,
        /// Reference accounting version supplied by the evaluator.
        cost_model: &'static str,
    },
}

/// Immutable native result retaining the complete request after execution returns.
///
/// This is an in-process result, not a portable evidence envelope or attestation.
#[derive(Debug)]
pub struct ExecutionReport<'package, 'model> {
    package: &'package NativePackage<'model>,
    input: RuntimeInput,
    selection: ExecutionSelection,
    validation_usage: ValidationUsage,
    outcome: ExecutionOutcome,
}

impl<'package, 'model> ExecutionReport<'package, 'model> {
    /// Exact supplied package, including source, static identity and artifact bytes.
    pub fn package(&self) -> &'package NativePackage<'model> {
        self.package
    }

    /// Every original offered artifact, including rejected or unselected entries.
    pub fn input(&self) -> &RuntimeInput {
        &self.input
    }

    /// Exact requested authored clause and observation, even when invalid.
    pub fn selection(&self) -> &ExecutionSelection {
        &self.selection
    }

    /// Work admitted during validation, independent of later evaluation.
    pub fn validation_usage(&self) -> ValidationUsage {
        self.validation_usage
    }

    /// Actual failed validation or evaluation observations.
    pub fn outcome(&self) -> &ExecutionOutcome {
        &self.outcome
    }

    /// A Boolean only when the selected predicate completed.
    pub fn truth(&self) -> Option<bool> {
        match &self.outcome {
            ExecutionOutcome::Evaluated {
                result: EvaluationOutcome::Completed(value),
                ..
            } => Some(*value),
            ExecutionOutcome::ValidationFailed(_)
            | ExecutionOutcome::Evaluated {
                result: EvaluationOutcome::Incomplete(_) | EvaluationOutcome::Refused(_),
                ..
            } => None,
        }
    }
}

/// Validate and execute one exact native request, preserving every returned stop.
///
/// One poll is forwarded through both stages. A caller poll panic unwinds normally;
/// cancellation returns a classified result. Each invocation starts fresh budgets.
pub fn execute<'package, 'model>(
    package: &'package NativePackage<'model>,
    input: RuntimeInput,
    selection: ExecutionSelection,
    limits: ExecutionLimits,
    mut poll: impl FnMut() -> bool,
) -> ExecutionReport<'package, 'model> {
    match validate_retaining(
        package.checked(),
        input,
        selection,
        limits.validation,
        &mut poll,
    ) {
        Ok(context) => {
            let evaluation = evaluate(&context, limits.evaluation, poll);
            let cost_model = evaluation.cost_model();
            let (result, usage, events) = evaluation.into_parts();
            let validation_usage = context.usage();
            let (input, selection) = context.into_request();
            ExecutionReport {
                package,
                input,
                selection,
                validation_usage,
                outcome: ExecutionOutcome::Evaluated {
                    result,
                    usage,
                    events,
                    cost_model,
                },
            }
        }
        Err(failure) => {
            let FailedValidation {
                input,
                selection,
                report,
            } = *failure;
            ExecutionReport {
                package,
                input,
                selection,
                validation_usage: report.usage,
                outcome: ExecutionOutcome::ValidationFailed(report),
            }
        }
    }
}
