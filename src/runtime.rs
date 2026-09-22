// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-018: native input artifacts. Construction does not validate a population.

mod construction;
mod evaluation;
mod execution;
mod input;
mod reading;
mod validation;

pub use execution::{execute, ExecutionLimits, ExecutionOutcome, ExecutionReport};
pub use reading::{InputReadCause, InputReadError, InputReadStage};

pub use evaluation::{
    evaluate, EvaluationLimits, EvaluationOutcome, EvaluationReport, EvaluationUsage,
    ImplicationEvent, ImplicationEventKind,
};

pub use validation::{
    validate, ExecutionSelection, ObservationSelection, RuntimeInput, RuntimeLocation,
    RuntimePathSegment, RuntimeReference, ValidatedContext, ValidationDiagnostic, ValidationLimits,
    ValidationReport, ValidationStatus, ValidationUsage,
};

pub use input::{
    ArtifactLimits, ArtifactUsage, DraftPathSegment, FieldBinding, InputError, InvocationDraft,
    InvocationRef, ModelBinding, ObjectEntry, ObjectIdentity, Population, QualifiedName,
    SnapshotDraft, SnapshotRef, ValueBinding, ValueId, ValueNode,
};

use qsl_foundation::{ByteDigest, SourceIdentity};

/// Immutable byte-bound snapshot, awaiting model-aware runtime validation.
#[derive(Clone, Debug)]
pub struct Snapshot {
    artifact: construction::Artifact<SnapshotDraft>,
}

impl Snapshot {
    /// Read exact selected native-state-input/1 bytes and repeat structural checks.
    pub fn read_verified(
        expected: &SnapshotRef,
        bytes: &[u8],
        limits: ArtifactLimits,
    ) -> Result<Self, Box<InputReadError>> {
        reading::read(RuntimeReference::Snapshot(expected.clone()), bytes, limits)
            .map(|artifact| Self { artifact })
    }

    /// Check structure and emit exact native-state-input/1 snapshot bytes.
    pub fn new(
        identity: SourceIdentity,
        draft: SnapshotDraft,
        limits: ArtifactLimits,
    ) -> Result<Self, Box<InputError>> {
        construction::Artifact::new(identity, draft, limits).map(|artifact| Self { artifact })
    }

    /// Exact source labels chosen for this input artifact.
    pub fn identity(&self) -> &SourceIdentity {
        &self.artifact.identity
    }

    /// Exact artifact bytes, as constructed or read from the selected input.
    pub fn bytes(&self) -> &[u8] {
        &self.artifact.bytes
    }

    /// SHA-256 of the complete retained artifact bytes.
    pub const fn digest(&self) -> ByteDigest {
        self.artifact.digest
    }

    /// Original structurally checked draft, exposed immutably.
    pub fn draft(&self) -> &SnapshotDraft {
        &self.artifact.draft
    }

    /// Actual construction usage, independent of later validation.
    pub const fn usage(&self) -> ArtifactUsage {
        self.artifact.usage
    }

    /// Owned exact selector; copying labels detaches the selector from this borrow.
    pub fn reference(&self) -> SnapshotRef {
        SnapshotRef::from_artifact(self.identity().clone(), self.digest())
    }
}

/// Immutable byte-bound recorded invocation, awaiting model-aware validation.
#[derive(Clone, Debug)]
pub struct Invocation {
    artifact: construction::Artifact<InvocationDraft>,
}

impl Invocation {
    /// Read exact selected native-state-input/1 bytes and repeat structural checks.
    pub fn read_verified(
        expected: &InvocationRef,
        bytes: &[u8],
        limits: ArtifactLimits,
    ) -> Result<Self, Box<InputReadError>> {
        reading::read(
            RuntimeReference::Invocation(expected.clone()),
            bytes,
            limits,
        )
        .map(|artifact| Self { artifact })
    }

    /// Check structure and emit exact native-state-input/1 invocation bytes.
    pub fn new(
        identity: SourceIdentity,
        draft: InvocationDraft,
        limits: ArtifactLimits,
    ) -> Result<Self, Box<InputError>> {
        construction::Artifact::new(identity, draft, limits).map(|artifact| Self { artifact })
    }

    /// Exact source labels chosen for this input artifact.
    pub fn identity(&self) -> &SourceIdentity {
        &self.artifact.identity
    }

    /// Exact artifact bytes, as constructed or read from the selected input.
    pub fn bytes(&self) -> &[u8] {
        &self.artifact.bytes
    }

    /// SHA-256 of the complete retained artifact bytes.
    pub const fn digest(&self) -> ByteDigest {
        self.artifact.digest
    }

    /// Original structurally checked draft, exposed immutably.
    pub fn draft(&self) -> &InvocationDraft {
        &self.artifact.draft
    }

    /// Actual construction usage, independent of later validation.
    pub const fn usage(&self) -> ArtifactUsage {
        self.artifact.usage
    }

    /// Owned exact selector, distinct from a snapshot selector.
    pub fn reference(&self) -> InvocationRef {
        InvocationRef::from_artifact(self.identity().clone(), self.digest())
    }
}
