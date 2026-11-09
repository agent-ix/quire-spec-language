// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-563: which member of two results differs.

use qsl_eval::execution::BackendCoverage;
use qsl_package::CheckedPackage;
use quire_eval::{CallFailure, Evaluation, FamilyOutcome};
use quire_exact::Outcome;

/// The member of a result in which a backend differs from the reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Member {
    /// `Ok` against `Err`.
    OkAgainstErr,
    /// The `Evaluation` outcome kind.
    OutcomeKind,
    /// The completed value.
    CompletedValue,
    /// The typed cause (of an `Undefined`, `Refused` or family result).
    TypedCause,
    /// The `Location`.
    Location,
    /// The loss records.
    Losses,
    /// An `Incomplete`'s limit or charge point.
    IncompleteChargePoint,
    /// A `CallFailure`'s variant or payload.
    CallFailurePayload,
}

fn kind(outcome: &FamilyOutcome<quire_exact::Value>) -> u8 {
    match outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(_)) => 0,
        FamilyOutcome::Evaluated(Outcome::Undefined(_)) => 1,
        FamilyOutcome::Evaluated(Outcome::Refused(_)) => 2,
        FamilyOutcome::Evaluated(Outcome::Incomplete(_)) => 3,
        _ => 4,
    }
}

fn evaluations(want: &Evaluation, got: &Evaluation) -> Option<Member> {
    if kind(&want.outcome) != kind(&got.outcome) {
        return Some(Member::OutcomeKind);
    }
    match (&want.outcome, &got.outcome) {
        (
            FamilyOutcome::Evaluated(Outcome::Completed(a)),
            FamilyOutcome::Evaluated(Outcome::Completed(b)),
        ) if format!("{a:?}") != format!("{b:?}") => return Some(Member::CompletedValue),
        (
            FamilyOutcome::Evaluated(Outcome::Incomplete(a)),
            FamilyOutcome::Evaluated(Outcome::Incomplete(b)),
        ) if format!("{a:?}") != format!("{b:?}") => return Some(Member::IncompleteChargePoint),
        (
            FamilyOutcome::Evaluated(Outcome::Undefined(a)),
            FamilyOutcome::Evaluated(Outcome::Undefined(b)),
        ) if format!("{a:?}") != format!("{b:?}") => return Some(Member::TypedCause),
        (
            FamilyOutcome::Evaluated(Outcome::Refused(a)),
            FamilyOutcome::Evaluated(Outcome::Refused(b)),
        ) if format!("{a:?}") != format!("{b:?}") => return Some(Member::TypedCause),
        (FamilyOutcome::FamilyEvaluated(a), FamilyOutcome::FamilyEvaluated(b))
            if format!("{a:?}") != format!("{b:?}") =>
        {
            return Some(Member::TypedCause)
        }
        _ => {}
    }
    if format!("{:?}", want.location) != format!("{:?}", got.location) {
        return Some(Member::Location);
    }
    if format!("{:?}", want.losses) != format!("{:?}", got.losses) {
        return Some(Member::Losses);
    }
    None
}

/// The member in which `got` differs from `want`, if any. A backend `Fault`
/// where the reference returned anything else is a difference.
pub(crate) fn difference(
    want: &Result<Evaluation, CallFailure>,
    got: &Result<Evaluation, CallFailure>,
) -> Option<Member> {
    match (want, got) {
        (Ok(a), Ok(b)) => evaluations(a, b),
        (Err(a), Err(b)) => {
            (format!("{a:?}") != format!("{b:?}")).then_some(Member::CallFailurePayload)
        }
        _ => Some(Member::OkAgainstErr),
    }
}

/// Whether every construct of the function `entry` lies in `coverage`.
pub(crate) fn within_coverage(
    package: &CheckedPackage,
    entry: &str,
    coverage: &BackendCoverage,
) -> bool {
    let graph = package.graph();
    let Some(callable) = graph.callable(entry) else {
        return false;
    };
    let Some(function) = graph.function_by_identity(callable.identity.clone()) else {
        return false;
    };
    qsl_eval::execution::first_outside(callable.parameters, function.body, coverage).is_none()
}
