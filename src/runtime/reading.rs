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

/// Original read failure, without recovering fields from a formatted message.
#[derive(Debug, thiserror::Error)]
pub enum InputReadCause {
    /// Failed byte, version, kind or identity selection.
    #[error("{0}")]
    Selection(&'static str),
    /// Original Serde failure; line/column refer to the stated decode stage.
    #[error("{0}")]
    Json(#[from] serde_json::Error),
    /// Original structural error, including draft path and construction usage.
    #[error("{0}")]
    Construction(#[from] Box<InputError>),
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
    let mut stage = InputReadStage::Selection;
    let mut code = Code::ResourceExhausted;
    let mut read = || -> Result<Artifact<T>, InputReadCause> {
        if bytes.len() > limits.bounded().artifact_bytes {
            return Err(InputReadCause::Selection(
                "native input byte limit exceeded",
            ));
        }
        code = Code::StaleDependency;
        let digest = ByteDigest::of(bytes);
        if digest != expected.digest() {
            return Err(InputReadCause::Selection(
                "selected digest differs from input bytes",
            ));
        }
        stage = InputReadStage::Envelope;
        code = Code::InvalidRuntimeInput;
        let Object(envelope): Object<Envelope<'_>> = serde_json::from_slice(bytes)?;
        if envelope.version != "native-state-input/1" {
            code = Code::UnknownWire;
            return Err(InputReadCause::Selection(
                "native input version is not admitted",
            ));
        }
        if envelope.kind != T::KIND {
            return Err(InputReadCause::Selection(
                "native input artifact kind differs",
            ));
        }
        if &envelope.identity != expected.identity() {
            code = Code::StaleDependency;
            return Err(InputReadCause::Selection(
                "selected identity differs from envelope",
            ));
        }
        stage = InputReadStage::Body;
        let Object(draft): Object<T> = serde_json::from_str(envelope.body.get())?;
        stage = InputReadStage::Construction;
        let mut artifact = Artifact::new(envelope.identity, draft, limits).map_err(|error| {
            code = error.code;
            InputReadCause::Construction(error)
        })?;
        // Construction counters describe that actual pass. Retained bytes and
        // digest describe this read's external artifact, including its layout.
        artifact.bytes = bytes.to_vec();
        artifact.digest = digest;
        Ok(artifact)
    };
    read().map_err(|cause| {
        Box::new(InputReadError {
            code,
            expected,
            stage,
            cause,
        })
    })
}
