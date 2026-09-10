// SPDX-License-Identifier: AGPL-3.0-only
//! FR-030: distinguish input ceilings, profile skew and foreign Quire context.

use super::{
    Limits, Selection, SemanticContext, Source, CONTRACT_VERSION, MAX_LINES, MAX_SOURCE_BYTES,
    SEMANTIC_CORE_VERSION,
};
use crate::Code;

/// Failure before invoking Quire, with a typed discriminator and actual context.
#[derive(Debug, Eq, PartialEq, thiserror::Error, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum PreflightFailure {
    /// Original document exceeds the selected byte ceiling.
    #[error("Quire source has {actual} bytes; maximum {maximum}")]
    SourceBytes {
        /// Original source bytes.
        actual: usize,
        /// Effective caller-lowered ceiling.
        maximum: usize,
    },
    /// Original line count cannot be admitted under the selected ceiling.
    #[error("Quire source line count {actual:?} exceeds maximum {maximum}")]
    SourceLines {
        /// Actual final line, or absence if the source index cannot locate EOF.
        actual: Option<usize>,
        /// Effective caller-lowered ceiling.
        maximum: usize,
    },
    /// The context selects a different Quire contract version.
    #[error("Quire contract version {actual} differs from {expected}")]
    ContractVersion {
        /// Context-supplied version.
        actual: String,
        /// Consumer-selected version.
        expected: &'static str,
    },
    /// The context selects a different semantic-core version.
    #[error("Quire semantic core {actual} differs from {expected}")]
    SemanticCore {
        /// Context-supplied version.
        actual: String,
        /// Consumer-selected version.
        expected: &'static str,
    },
    /// Context identity is absent or belongs to another document.
    #[error("Quire source identity {actual:?} differs from {expected}")]
    SourceIdentity {
        /// Context-supplied label, including explicit absence.
        actual: Option<String>,
        /// Selected original document label.
        expected: String,
    },
    /// Context path belongs to another selected document.
    #[error("Quire path {actual} differs from {expected}")]
    SourcePath {
        /// Context-supplied path.
        actual: String,
        /// Selected source path.
        expected: String,
    },
    /// Context package differs from the authored clause owner.
    #[error("Quire package {actual} differs from {expected}")]
    Package {
        /// Context-supplied package.
        actual: String,
        /// Authored clause package.
        expected: String,
    },
}

impl PreflightFailure {
    /// Existing stable native code; the variant identifies the precise refusal.
    pub fn code(&self) -> Code {
        match self {
            Self::SourceBytes { .. } | Self::SourceLines { .. } => Code::ResourceExhausted,
            Self::ContractVersion { .. } | Self::SemanticCore { .. } => Code::UnknownProfile,
            Self::SourceIdentity { .. } | Self::SourcePath { .. } | Self::Package { .. } => {
                Code::InvalidModelBinding
            }
        }
    }
}

pub(super) fn check(
    original: &Source,
    context: &SemanticContext,
    selection: &Selection,
    limits: Limits,
) -> Result<(), Box<PreflightFailure>> {
    let actual = original.text().len();
    let maximum = limits.source_bytes.min(MAX_SOURCE_BYTES);
    if actual > maximum {
        return Err(Box::new(PreflightFailure::SourceBytes { actual, maximum }));
    }
    let actual = original
        .position(original.text().len())
        .map(|position| position.line);
    let maximum = limits.lines.min(MAX_LINES);
    if actual.is_none_or(|actual| actual > maximum) {
        return Err(Box::new(PreflightFailure::SourceLines { actual, maximum }));
    }
    if context.module.contract_version != CONTRACT_VERSION {
        return Err(Box::new(PreflightFailure::ContractVersion {
            actual: context.module.contract_version.clone(),
            expected: CONTRACT_VERSION,
        }));
    }
    if context.module.semantic_core != SEMANTIC_CORE_VERSION {
        return Err(Box::new(PreflightFailure::SemanticCore {
            actual: context.module.semantic_core.clone(),
            expected: SEMANTIC_CORE_VERSION,
        }));
    }
    if context.source_identity.as_deref() != Some(original.identity().identity.as_str()) {
        return Err(Box::new(PreflightFailure::SourceIdentity {
            actual: context.source_identity.clone(),
            expected: original.identity().identity.clone(),
        }));
    }
    if context.path != original.path() {
        return Err(Box::new(PreflightFailure::SourcePath {
            actual: context.path.clone(),
            expected: original.path().into(),
        }));
    }
    if context.identity_package() != selection.binding.requirement.package().as_str() {
        return Err(Box::new(PreflightFailure::Package {
            actual: context.identity_package().into(),
            expected: selection.binding.requirement.package().as_str().into(),
        }));
    }
    Ok(())
}
