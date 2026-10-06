// SPDX-License-Identifier: AGPL-3.0-or-later
//! Type-environment admission as a compiler stage outcome (ADR-013 T-4).
//!
//! The registry reports a reached [`TypeEnvironmentLimits`] ceiling as its
//! own [`EnvironmentLimit`], since it is a layer below the compiler's
//! diagnostics. A stage that admits a type environment reports that
//! ceiling as a stage limit (ADR-014 B-3) with no locus: the object types
//! come from an admitted domain package, not a source unit (FR-096).
//!
//! [`TypeEnvironmentLimits`]: quire_semantic_value::declaration::TypeEnvironmentLimits

use qsl_foundation::diagnostic::{LimitExceeded, LimitKind, StageFailure};

use quire_semantic_value::declaration::{
    EnvironmentFailure, EnvironmentLimit, EnvironmentLimitKind, InvalidDeclaration,
};

/// `limit` as the stage limit it is: `ancestor_steps` is an edge count, the
/// `supertypes` edges one conformance walk follows, and `work_units` is the
/// work budget.
pub fn stage_limit(limit: EnvironmentLimit) -> LimitExceeded {
    let kind = match limit.kind() {
        EnvironmentLimitKind::AncestorSteps => LimitKind::EdgeCount,
        EnvironmentLimitKind::WorkUnits => LimitKind::WorkBudget,
    };
    LimitExceeded::new(kind, limit.configured_bound(), limit.actual())
}

/// `failure` as the admitting stage's failure: a refusal stays the
/// declaration's refusal, a ceiling becomes its [`stage_limit`].
pub fn stage_failure(failure: EnvironmentFailure) -> StageFailure<InvalidDeclaration> {
    match failure {
        EnvironmentFailure::Refused(invalid) => StageFailure::Refused(invalid),
        EnvironmentFailure::Limit(limit) => StageFailure::Limit(stage_limit(limit)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each ceiling maps to its stage limit kind with its bound and counter
    /// unchanged.
    #[test]
    fn each_ceiling_is_its_stage_limit() {
        assert_eq!(
            stage_limit(EnvironmentLimit::new(
                EnvironmentLimitKind::AncestorSteps,
                5,
                6
            )),
            LimitExceeded::new(LimitKind::EdgeCount, 5, 6)
        );
        assert_eq!(
            stage_limit(EnvironmentLimit::new(EnvironmentLimitKind::WorkUnits, 4, 5)),
            LimitExceeded::new(LimitKind::WorkBudget, 4, 5)
        );
    }
}
