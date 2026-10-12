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

use std::collections::BTreeMap;

use qsl_foundation::diagnostic::{
    CatalogCode, LimitExceeded, StageFailure, ALLOCATION_FAILED, RESERVATION_SIZE_OVERFLOW,
};
use qsl_foundation::Setting;

use quire_semantic_value::declaration::{
    EnvironmentFailure, EnvironmentLimit, EnvironmentLimitKind, InvalidDeclaration, StorageRequest,
    StorageUnit,
};

/// A refusal raised while admitting the shared type-environment registry.
/// Storage causes retain the kernel's exact request amount and native unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnvironmentStageCause {
    /// The kernel refused a declaration.
    Declaration(InvalidDeclaration),
    /// A real kernel allocation request was denied.
    Allocation { request: StorageRequest },
    /// The kernel could not compute or represent the reservation size.
    Capacity { request: StorageRequest },
}

impl EnvironmentStageCause {
    /// The catalog code for a storage refusal, when this is one. Declaration
    /// refusals retain their kernel cause and are not reinterpreted here.
    pub fn storage_catalog_code(&self) -> Option<CatalogCode> {
        match self {
            Self::Allocation { .. } => Some(ALLOCATION_FAILED),
            Self::Capacity { .. } => Some(RESERVATION_SIZE_OVERFLOW),
            Self::Declaration(_) => None,
        }
    }

    /// The structured storage payload, without converting element counts to
    /// byte counts.
    pub fn storage_catalog_fields(&self) -> Option<BTreeMap<&'static str, String>> {
        let request = match self {
            Self::Allocation { request } | Self::Capacity { request } => request,
            Self::Declaration(_) => return None,
        };
        let unit = match request.unit() {
            StorageUnit::Bytes => "bytes",
            StorageUnit::AdditionalElements => "additional-elements",
        };
        Some(BTreeMap::from([
            ("requested", request.amount().to_string()),
            ("unit", unit.to_owned()),
        ]))
    }
}

/// `limit` as the stage limit it is: `ancestor_steps` is an edge count, the
/// types one conformance walk expands (`environment.ancestor_steps`), and
/// `work_units` is the work budget (`environment.work_units`).
pub fn stage_limit(limit: EnvironmentLimit) -> LimitExceeded {
    let setting = match limit.kind() {
        EnvironmentLimitKind::AncestorSteps => Setting::EnvironmentAncestorSteps,
        EnvironmentLimitKind::WorkUnits => Setting::EnvironmentWorkUnits,
    };
    LimitExceeded::new(setting, limit.configured_bound(), limit.actual())
}

/// `failure` as the admitting stage's failure: a refusal stays the
/// declaration's refusal, a ceiling becomes its [`stage_limit`].
pub fn stage_failure(failure: EnvironmentFailure) -> StageFailure<EnvironmentStageCause> {
    match failure {
        EnvironmentFailure::Refused(invalid) => {
            StageFailure::Refused(EnvironmentStageCause::Declaration(invalid))
        }
        EnvironmentFailure::Limit(limit) => StageFailure::Limit(stage_limit(limit)),
        EnvironmentFailure::Allocation(request) => {
            StageFailure::Refused(EnvironmentStageCause::Allocation { request })
        }
        EnvironmentFailure::Capacity(request) => {
            StageFailure::Refused(EnvironmentStageCause::Capacity { request })
        }
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
            LimitExceeded::new(Setting::EnvironmentAncestorSteps, 5, 6)
        );
        assert_eq!(
            stage_limit(EnvironmentLimit::new(EnvironmentLimitKind::WorkUnits, 4, 5)),
            LimitExceeded::new(Setting::EnvironmentWorkUnits, 4, 5)
        );
    }

    /// FR-082-AC-10: a registry denial preserves both native storage units.
    #[test]
    fn allocation_refusal_keeps_exact_amount_and_native_unit() {
        for request in [
            StorageRequest::new(6, StorageUnit::Bytes),
            StorageRequest::new(3, StorageUnit::AdditionalElements),
        ] {
            let StageFailure::Refused(EnvironmentStageCause::Allocation { request: observed }) =
                stage_failure(EnvironmentFailure::Allocation(request))
            else {
                panic!("allocator denial must remain a refusal")
            };
            assert_eq!(observed, request);
            let cause = EnvironmentStageCause::Allocation { request: observed };
            assert_eq!(cause.storage_catalog_code(), Some(ALLOCATION_FAILED));
            assert_eq!(
                cause.storage_catalog_fields(),
                Some(BTreeMap::from([
                    ("requested", request.amount().to_string()),
                    (
                        "unit",
                        match request.unit() {
                            StorageUnit::Bytes => "bytes".to_owned(),
                            StorageUnit::AdditionalElements => "additional-elements".to_owned(),
                        },
                    ),
                ]))
            );
        }
    }

    /// FR-082-AC-11: reservation-size overflow retains the native request and
    /// remains distinct from allocator denial.
    #[test]
    fn capacity_refusal_keeps_exact_amount_and_native_unit() {
        for request in [
            StorageRequest::new(9, StorageUnit::Bytes),
            StorageRequest::new(4, StorageUnit::AdditionalElements),
        ] {
            let StageFailure::Refused(EnvironmentStageCause::Capacity { request: observed }) =
                stage_failure(EnvironmentFailure::Capacity(request))
            else {
                panic!("reservation-size overflow must remain a refusal")
            };
            assert_eq!(observed, request);
            let cause = EnvironmentStageCause::Capacity { request: observed };
            assert_eq!(
                cause.storage_catalog_code(),
                Some(RESERVATION_SIZE_OVERFLOW)
            );
            assert_eq!(
                cause.storage_catalog_fields(),
                Some(BTreeMap::from([
                    ("requested", request.amount().to_string()),
                    (
                        "unit",
                        match request.unit() {
                            StorageUnit::Bytes => "bytes".to_owned(),
                            StorageUnit::AdditionalElements => "additional-elements".to_owned(),
                        },
                    ),
                ]))
            );
        }
    }
}
