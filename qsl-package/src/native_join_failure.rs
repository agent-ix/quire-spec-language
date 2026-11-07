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
}

impl NativeJoinFailure {
    /// The stage setting for an actual resource denial, otherwise `None`.
    pub fn setting(&self) -> Option<qsl_foundation::Setting> {
        match self {
            Self::Projection(error) => qsl_semantics::model::intake::native_budget_setting(error),
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
