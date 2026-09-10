// SPDX-License-Identifier: AGPL-3.0-only
//! FR-024: selected native bytes, closed Serde decoding and structural admission.

use crate::serde_object::Object;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::value::RawValue;

use super::construction::{Artifact, Body};
use super::{ArtifactLimits, InputError, RuntimeReference};
use crate::{ByteDigest, Code, SourceIdentity};

/// Actual stage of a native artifact read failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputReadStage {
    /// Input byte limit or selected digest preflight.
    Selection,
    /// Closed envelope, version, kind or identity selection.
    Envelope,
    /// Closed native body decoding, before structural inspection.
    Body,
    /// Existing artifact construction, including value indices and limits.
    Construction,
}

impl InputReadStage {
    /// Stable phase spelling for native read diagnostics.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Selection => "selection",
            Self::Envelope => "envelope",
            Self::Body => "body",
            Self::Construction => "construction",
        }
    }
}

impl std::fmt::Display for InputReadStage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Original read failure, without recovering fields from a formatted message.
#[derive(Debug, thiserror::Error)]
pub enum InputReadCause {
    /// Input bytes exceeded the effective clamped ceiling, before hashing.
    #[error("native input byte limit exceeded")]
    ByteLimit {
        /// Original input byte length.
        actual: usize,
        /// Effective caller-lowered or implementation ceiling.
        maximum: usize,
    },
    /// Computed input digest differs from the separately retained selected reference.
    #[error("selected digest differs from input bytes")]
    DigestMismatch {
        /// Digest of the actual input bytes.
        actual: ByteDigest,
    },
    /// Envelope version differs from the admitted native input format.
    #[error("native input version is not admitted")]
    Version {
        /// Original version spelling, without normalization.
        actual: String,
    },
    /// Envelope artifact kind differs from the selected reader's role.
    #[error("native input artifact kind differs")]
    ArtifactKind {
        /// Role selected by the snapshot or invocation reader.
        expected: &'static str,
        /// Original kind spelling.
        actual: String,
    },
    /// Envelope identity differs from the separately retained selected reference.
    #[error("selected identity differs from envelope")]
    Identity {
        /// Original identity decoded from the envelope.
        actual: SourceIdentity,
    },
    /// Original envelope JSON error; coordinates refer to the full input.
    #[error("{0}")]
    Envelope(#[source] serde_json::Error),
    /// Original body JSON error; coordinates refer to the raw body slice.
    #[error("{0}")]
    Body(#[source] serde_json::Error),
    /// Original structural error, including draft path and construction usage.
    #[error("{0}")]
    Construction(#[from] Box<InputError>),
}

impl InputReadCause {
    fn classification(&self) -> (InputReadStage, Code) {
        match self {
            Self::ByteLimit { .. } => (InputReadStage::Selection, Code::ResourceExhausted),
            Self::DigestMismatch { .. } => (InputReadStage::Selection, Code::StaleDependency),
            Self::Version { .. } => (InputReadStage::Envelope, Code::UnknownWire),
            Self::ArtifactKind { .. } | Self::Envelope(_) => {
                (InputReadStage::Envelope, Code::InvalidRuntimeInput)
            }
            Self::Identity { .. } => (InputReadStage::Envelope, Code::StaleDependency),
            Self::Body(_) => (InputReadStage::Body, Code::InvalidRuntimeInput),
            Self::Construction(error) => (InputReadStage::Construction, error.code),
        }
    }
}

/// Failed input read retaining the selected role, identity and digest.
#[derive(Debug, thiserror::Error)]
#[error("{code}: {cause}")]
pub struct InputReadError {
    /// Stable native refusal or resource-exhaustion code.
    pub code: Code,
    /// Original expected reference; no observed identity is substituted.
    pub expected: RuntimeReference,
    /// Stage that actually failed.
    pub stage: InputReadStage,
    /// Original typed cause.
    #[source]
    pub cause: InputReadCause,
}

impl InputReadError {
    /// A byte or structural budget prevented completion.
    pub fn is_incomplete(&self) -> bool {
        self.code == Code::ResourceExhausted
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope<'a> {
    version: String,
    kind: String,
    #[serde(deserialize_with = "super::input::deserialize_identity")]
    identity: SourceIdentity,
    #[serde(borrow)]
    body: &'a RawValue,
}

pub(super) fn read<T: Body + DeserializeOwned>(
    expected: RuntimeReference,
    bytes: &[u8],
    limits: ArtifactLimits,
) -> Result<Artifact<T>, Box<InputReadError>> {
    read_selected(&expected, bytes, limits).map_err(|cause| {
        let (stage, code) = cause.classification();
        Box::new(InputReadError {
            code,
            expected,
            stage,
            cause,
        })
    })
}

fn read_selected<T: Body + DeserializeOwned>(
    expected: &RuntimeReference,
    bytes: &[u8],
    limits: ArtifactLimits,
) -> Result<Artifact<T>, InputReadCause> {
    let maximum = limits.bounded().artifact_bytes;
    if bytes.len() > maximum {
        return Err(InputReadCause::ByteLimit {
            actual: bytes.len(),
            maximum,
        });
    }
    let digest = ByteDigest::of(bytes);
    if digest != expected.digest() {
        return Err(InputReadCause::DigestMismatch { actual: digest });
    }
    // Preserve Serde's default recursion limit on both untrusted decode phases.
    let Object(envelope): Object<Envelope<'_>> =
        serde_json::from_slice(bytes).map_err(InputReadCause::Envelope)?;
    if envelope.version != super::input::FORMAT {
        return Err(InputReadCause::Version {
            actual: envelope.version,
        });
    }
    if envelope.kind != T::KIND {
        return Err(InputReadCause::ArtifactKind {
            expected: T::KIND,
            actual: envelope.kind,
        });
    }
    if &envelope.identity != expected.identity() {
        return Err(InputReadCause::Identity {
            actual: envelope.identity,
        });
    }
    let Object(draft): Object<T> =
        serde_json::from_str(envelope.body.get()).map_err(InputReadCause::Body)?;
    Artifact::from_external(envelope.identity, draft, bytes, digest, limits)
        .map_err(InputReadCause::Construction)
}
