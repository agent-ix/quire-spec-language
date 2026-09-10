// SPDX-License-Identifier: AGPL-3.0-only
//! FR-007: exact validation requests, immutable contexts and classified reports.

use std::cmp::Ordering;

use quire_contract_ir as ir;

use super::super::{
    Invocation, InvocationRef, ObjectIdentity, QualifiedName, Snapshot, SnapshotRef,
};
use crate::checking::{CheckedClause, CheckedPackage};
use crate::{ByteDigest, Diagnostic, SourceIdentity};

#[cfg(test)]
#[path = "../../../tests/support/runtime_evaluation_invariants.rs"]
mod invariant_tests;

/// Owned offered artifacts; validation never mutates their contents.
#[derive(Clone, Debug, Default)]
pub struct RuntimeInput {
    /// Offered snapshots, including unselected entries checked for identity conflicts.
    pub snapshots: Vec<Snapshot>,
    /// Offered recorded invocations, in the same identity namespace as snapshots.
    pub invocations: Vec<Invocation>,
}

/// Observation inputs selected for one exact authored clause.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObservationSelection {
    /// Current-state invariant with an exact self object.
    Current {
        /// Expected complete snapshot bytes.
        snapshot: SnapshotRef,
        /// Object supplying the invariant context.
        self_object: ObjectIdentity,
    },
    /// Recorded pre/post operation with immutable invocation captures.
    Invocation {
        /// Expected complete invocation bytes.
        invocation: InvocationRef,
    },
}

/// Exact authored clause and its selected runtime input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionSelection {
    /// Authored requirement owner; model owners are not substituted here.
    pub requirement: ir::RequirementRef,
    /// Local clause identity under the selected requirement.
    pub clause: ir::ClauseId,
    /// Current invariant or recorded operation inputs.
    pub observation: ObservationSelection,
}

/// Exact native artifact provenance for a runtime diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeReference {
    /// Snapshot role, including an expected unavailable snapshot.
    Snapshot(SnapshotRef),
    /// Recorded invocation role, including an expected unavailable invocation.
    Invocation(InvocationRef),
}

impl RuntimeReference {
    /// Supplied or expected native identity/revision labels.
    pub fn identity(&self) -> &SourceIdentity {
        match self {
            Self::Snapshot(value) => value.identity(),
            Self::Invocation(value) => value.identity(),
        }
    }

    /// Supplied or expected complete byte digest.
    pub fn digest(&self) -> ByteDigest {
        match self {
            Self::Snapshot(value) => value.digest(),
            Self::Invocation(value) => value.digest(),
        }
    }

    pub(super) fn compare(&self, other: &Self) -> Ordering {
        let kind = |value: &Self| match value {
            Self::Snapshot(_) => 0,
            Self::Invocation(_) => 1,
        };
        (
            kind(self),
            &self.identity().identity,
            &self.identity().revision,
        )
            .cmp(&(
                kind(other),
                &other.identity().identity,
                &other.identity().revision,
            ))
            .then_with(|| self.digest().to_string().cmp(&other.digest().to_string()))
    }
}

/// Typed location within a population or recorded invocation.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RuntimePathSegment {
    /// Exact model owner.
    Model(ir::RequirementRef),
    /// Object payload and universe within that model.
    Population {
        /// Object payload record.
        record: ir::SymbolName,
        /// Declared universe.
        universe: ir::SymbolName,
    },
    /// Exact object key, without Unicode normalization.
    Object(String),
    /// Qualified State declaration.
    State(QualifiedName),
    /// Qualified operation parameter.
    Parameter(QualifiedName),
    /// Declared operation result.
    Result,
    /// Original field name rather than an unstable vector ordinal.
    Field(ir::SymbolName),
    /// Original sequence position.
    Index(usize),
}

/// Runtime provenance, separate from a diagnostic's original native source span.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeLocation {
    /// Actual or expected complete native input artifact.
    pub artifact: RuntimeReference,
    /// Observation of the value at this location, when established.
    pub observation: Option<ir::StateObservation>,
    /// Requested authored owner, retained even when foreign.
    pub requirement: ir::RequirementRef,
    /// Requested authored clause, retained even when absent.
    pub clause: ir::ClauseId,
    /// Model/population/object/value/field/sequence components.
    pub path: Vec<RuntimePathSegment>,
}

impl RuntimeLocation {
    pub(super) fn compare(&self, other: &Self) -> Ordering {
        self.artifact
            .compare(&other.artifact)
            .then_with(|| self.observation.cmp(&other.observation))
            .then_with(|| self.requirement.cmp(&other.requirement))
            .then_with(|| self.clause.cmp(&other.clause))
            .then_with(|| self.path.cmp(&other.path))
    }
}

/// Caller-lowered inclusive validation ceilings; larger options clamp to defaults.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidationLimits {
    /// Total offered artifacts, at most 64.
    pub artifacts: usize,
    /// Aggregate offered encoded bytes, at most 8 MiB.
    pub artifact_bytes: usize,
    /// Object occurrences across selected snapshots, at most 10,000.
    pub objects: usize,
    /// Charged validation visits, at most 1,000,000.
    pub work: usize,
    /// Unicode-scalar iterator advances, including end checks, at most 8 MiB.
    pub text_steps: usize,
    /// Detailed diagnostics, at most 256, separate from the terminal stop reason.
    pub diagnostics: usize,
}

impl Default for ValidationLimits {
    fn default() -> Self {
        Self {
            artifacts: 64,
            artifact_bytes: 8_388_608,
            objects: 10_000,
            work: 1_000_000,
            text_steps: 8_388_608,
            diagnostics: 256,
        }
    }
}

impl ValidationLimits {
    pub(super) fn bounded(self) -> Self {
        let hard = Self::default();
        Self {
            artifacts: self.artifacts.min(hard.artifacts),
            artifact_bytes: self.artifact_bytes.min(hard.artifact_bytes),
            objects: self.objects.min(hard.objects),
            work: self.work.min(hard.work),
            text_steps: self.text_steps.min(hard.text_steps),
            diagnostics: self.diagnostics.min(hard.diagnostics),
        }
    }
}

/// Work actually admitted by this validation request, without construction fuel.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Serialize)]
pub struct ValidationUsage {
    /// Offered artifacts admitted by inventory preflight.
    pub artifacts: usize,
    /// Their aggregate encoded byte lengths.
    pub artifact_bytes: usize,
    /// Selected object occurrences admitted for inspection.
    pub objects: usize,
    /// Completed admission of validation work visits.
    pub work: usize,
    /// Admitted Unicode-scalar iterator advances.
    pub text_steps: usize,
    /// Retained detailed diagnostics, excluding a terminal reason.
    pub diagnostics: usize,
}

/// Failed validation cannot be represented as a predicate Boolean.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationStatus {
    /// At least one observed input defect is known invalid.
    Refused,
    /// Only unavailable data, cancellation or exhausted work prevented validation.
    Incomplete,
}

/// Retained actual defects and the optional reason traversal stopped.
#[derive(Clone, Debug, thiserror::Error)]
#[error("native runtime validation: {status:?}")]
pub struct ValidationReport {
    /// Overall classification; known invalid input takes precedence.
    pub status: ValidationStatus,
    /// Deterministically ordered observed details, possibly only a stopped prefix.
    pub diagnostics: Vec<Diagnostic>,
    /// Separate resource/cancellation stop reason, without a duplicate detail entry.
    pub terminal: Option<Box<Diagnostic>>,
    /// Actual work admitted before the report.
    pub usage: ValidationUsage,
}

/// An immutable, completely validated input context tied to its checked package.
pub struct ValidatedContext<'checked, 'model> {
    pub(super) checked: &'checked CheckedPackage<'model>,
    pub(super) input: RuntimeInput,
    pub(super) selection: ExecutionSelection,
    pub(super) clause_index: usize,
    pub(super) selected: super::Selected,
    pub(super) indexes: super::PopulationIndexes,
    pub(super) states: super::StateIndexes,
    pub(super) usage: ValidationUsage,
}

impl std::fmt::Debug for ValidatedContext<'_, '_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ValidatedContext")
            .field("clause", &self.selection.clause)
            .field("selected", &self.selected)
            .field("usage", &self.usage)
            .finish_non_exhaustive()
    }
}

impl<'checked, 'model> ValidatedContext<'checked, 'model> {
    /// Transfer exact offered data after the evaluator's borrow has ended.
    pub(in crate::runtime) fn into_request(self) -> (RuntimeInput, ExecutionSelection) {
        (self.input, self.selection)
    }

    /// Original source-order position established during complete validation.
    pub(crate) fn clause_index(&self) -> usize {
        self.clause_index
    }
    /// Exact static package this context was validated against.
    pub fn checked(&self) -> &'checked CheckedPackage<'model> {
        self.checked
    }
    /// Exact selected authored clause.
    pub fn clause(&self) -> &'checked CheckedClause<'model> {
        &self.checked.clauses()[self.clause_index]
    }
    /// Immutable original offered input bytes and drafts.
    pub fn input(&self) -> &RuntimeInput {
        &self.input
    }
    /// Exact requested clause and observation bindings.
    pub fn selection(&self) -> &ExecutionSelection {
        &self.selection
    }
    /// Actual fresh validation work.
    pub fn usage(&self) -> ValidationUsage {
        self.usage
    }
    /// Selected snapshot for an observation, without inferring unavailable counterparts.
    pub fn snapshot(&self, observation: ir::StateObservation) -> Option<&Snapshot> {
        self.selected
            .snapshot(observation)
            .and_then(|index| self.input.snapshots.get(index))
    }
    /// Exact selected recorded invocation, absent for an invariant.
    pub fn invocation(&self) -> Option<&Invocation> {
        self.selected
            .invocation
            .and_then(|index| self.input.invocations.get(index))
    }

    /// Exact State root index within the selected observation's snapshot arena.
    pub fn state(
        &self,
        observation: ir::StateObservation,
        name: &QualifiedName,
    ) -> Option<super::super::ValueId> {
        let snapshot = self.selected.snapshot(observation)?;
        self.states.get(&snapshot)?.get(name).copied().flatten()
    }
    /// Exact original object entry in a validated population.
    pub fn object(
        &self,
        observation: ir::StateObservation,
        identity: &ObjectIdentity,
    ) -> Option<&super::super::ObjectEntry> {
        let snapshot = self.selected.snapshot(observation)?;
        let index = self
            .indexes
            .get(&snapshot)?
            .get(&super::PopulationKey::of(identity))?;
        let object = index.objects.get(&identity.key).copied().flatten()?;
        self.input
            .snapshots
            .get(snapshot)?
            .draft()
            .populations
            .get(index.position)?
            .objects
            .get(object)
    }
}
