// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original owner refusals from the shared native admission pass.

/// The owner operation that refused native admission. The original typed cause
/// is retained; equality exhaustion is never relabelled as a projection error.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum NativeJoinFailure {
    /// Identity or Exact type projection refused.
    #[error(transparent)]
    Projection(#[from] Box<quire_contract_model::ExactTypeProjectionError>),
    /// The admitted equality reader refused its work or storage charge.
    #[error(transparent)]
    Equality(#[from] Box<quire_contract_model::AdmittedEqualityReadError>),
    /// A resource denial with its owner error and original configured ceiling.
    #[error(transparent)]
    Limit(Box<crate::NativeJoinLimitError>),
    /// QSL producer row materialization refused before an allocation.
    #[error(transparent)]
    Rows(#[from] crate::NativeRowError),
    /// The read of the package's unit definitions (FR-533) ran out of the
    /// native allowance; `setting` names the exhausted allowance.
    #[error("the unit definition read exhausted the native join budget")]
    UnitDefinitions {
        /// The stage setting of the exhausted work or storage allowance.
        setting: qsl_foundation::Setting,
    },
    /// The selected-model population catalog ran out of its allowance, the
    /// budget's unconsumed work or storage at that point (FR-533
    /// "Observation admission").
    #[error("the population catalog exhausted the native join budget")]
    PopulationCatalog {
        /// `i2.work_units` for the work allowance, `i2.native_join_storage`
        /// for the entries allowance.
        setting: qsl_foundation::Setting,
        /// The stage limit diagnostic: bound the catalog error's `limit`,
        /// attempted total its `consumed`.
        limit: qsl_foundation::diagnostic::LimitExceeded,
    },
}

impl NativeJoinFailure {
    /// Attach the stage diagnostic using the same caller budget that refused.
    /// Semantic refusals remain unchanged. Already attached diagnostics are
    /// retained, so a later budget use cannot rewrite their attempted count.
    pub fn with_budget(self, budget: &quire_contract_model::NativeJoinBudget) -> Self {
        if matches!(self, Self::Limit(_) | Self::PopulationCatalog { .. }) {
            return self;
        }
        match crate::NativeJoinLimits::from_budget(budget).refusal(self.clone(), budget) {
            Some(error) => Self::Limit(Box::new(error)),
            None => self,
        }
    }

    /// The original owner refusal, without the stage diagnostic envelope.
    #[must_use]
    pub fn native(&self) -> &Self {
        match self {
            Self::Limit(error) => error.native.native(),
            native => native,
        }
    }

    /// The stage diagnostic attached at admission, if this was a resource denial.
    #[must_use]
    pub fn limit(&self) -> Option<&qsl_foundation::diagnostic::LimitExceeded> {
        match self {
            Self::Limit(error) => Some(&error.limit),
            Self::PopulationCatalog { limit, .. } => Some(limit),
            Self::Projection(_)
            | Self::Equality(_)
            | Self::Rows(_)
            | Self::UnitDefinitions { .. } => None,
        }
    }

    /// The stage setting for an actual resource denial, otherwise `None`.
    pub fn setting(&self) -> Option<qsl_foundation::Setting> {
        match self {
            Self::UnitDefinitions { setting } | Self::PopulationCatalog { setting, .. } => {
                Some(*setting)
            }
            Self::Limit(error) => error.native.setting(),
            Self::Projection(error) => qsl_semantics::model::intake::native_budget_setting(error),
            Self::Rows(error) => error.denied().map(|charge| match charge.kind {
                quire_contract_model::NativeJoinChargeKind::Work => {
                    qsl_foundation::Setting::I2WorkUnits
                }
                quire_contract_model::NativeJoinChargeKind::Storage => {
                    qsl_foundation::Setting::I2NativeJoinStorage
                }
            }),
            Self::Equality(error) => match error.code {
                quire_contract_model::AdmittedEqualityReadErrorCode::WorkLimit => {
                    Some(qsl_foundation::Setting::I2WorkUnits)
                }
                quire_contract_model::AdmittedEqualityReadErrorCode::StorageLimit => {
                    Some(qsl_foundation::Setting::I2NativeJoinStorage)
                }
                _ => None,
            },
        }
    }
}

impl From<quire_contract_model::ExactTypeProjectionError> for NativeJoinFailure {
    fn from(error: quire_contract_model::ExactTypeProjectionError) -> Self {
        Self::Projection(Box::new(error))
    }
}
