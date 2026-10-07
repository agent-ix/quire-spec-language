// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-286 (ADR-029 CB-4): the one `quire-outcome/1` JSON outcome document
//! every QSL lifecycle operation's outcome serializes to, and its one
//! serializer.
//!
//! A driver writes a document with a constructor over the operation's own
//! outcome ([`OutcomeDocument::from_check`], [`OutcomeDocument::from_package`],
//! [`OutcomeDocument::from_replay`]), or, for the operations whose outcome
//! types live above this crate (`prove`, `analyze`, `monitor`), with
//! [`OutcomeDocument::settled`] over their items; [`OutcomeDocument::to_bytes`]
//! writes it. The CLI in machine mode writes exactly these bytes, so the
//! library and the CLI give one document for one request (QSpec
//! FR-300-AC-3).
//!
//! The serializer is pure: it runs no stage and reads no clock. The document
//! is a typed value, so its members are the types below, not keys inserted by
//! hand, and the same value serializes to the same bytes every time. Every
//! cause spelling is its typed cause's own. A proof item's label type has no
//! `undefined` member, so an undefined claim evaluation can only be written as
//! a `refuted` item with the `undefined-evaluation` cause and category
//! violation; the label `undefined` appears only as the category of a
//! non-proof evaluation outcome such as `execute`.

use qsl_foundation::diagnostic::{Category, Code, Locus, StageFailure, Staged};
use qsl_foundation::digest::DigestRecord;
use qsl_foundation::source::provenance::{OccurrenceKey, RawSourceRef};
use qsl_foundation::{LocatedSpan, RequestIndex};
use qsl_semantics::library::PackageId;
use qsl_semantics::model::observation::AdmissionFailure;
use quire_exact::CancelCause;
use serde::{Serialize, Serializer};

use crate::proof_result::{
    DeclineCode, InconclusiveCause, ReportedInconclusiveCause, TerminalRecord, TerminalValue,
};
use crate::result::ReplayResult;
use crate::spine::{
    CallIncomplete, CallOutcome, CallRefusal, CallValue, CheckedUnit, CompileRefusal, EmittedUnit,
    FrontEndFailure, SpineStage,
};
use crate::ReplayRefusal;

/// The document's `format` member.
pub const OUTCOME_FORMAT: &str = "quire-outcome/1";

/// The library operation that produced the document (ADR-029 LC-1).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    /// `parse`.
    Parse,
    /// `format`.
    Format,
    /// `select`.
    Select,
    /// `check`.
    Check,
    /// `check_fences`.
    CheckFences,
    /// `package`.
    Package,
    /// `execute`.
    Execute,
    /// `monitor`.
    Monitor,
    /// `analyze`.
    Analyze,
    /// `lower`.
    Lower,
    /// `generate`.
    Generate,
    /// `prove`.
    Prove,
    /// `replay`.
    Replay,
    /// `inspect`.
    Inspect,
    /// `render`.
    Render,
}

/// An ADR-011 stage, the last one an operation reached.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum OutcomeStage {
    /// S0, source bytes.
    S0,
    /// S1, the lossless CST.
    S1,
    /// S2, parsed semantic forms.
    S2,
    /// I1, domain-package intake.
    I1,
    /// S3, the checked semantic graph.
    S3,
    /// S4, the linked checked package.
    S4,
    /// S5, contract IR.
    S5,
    /// S6a, reference execution.
    S6a,
    /// S6b, bounded proof.
    S6b,
    /// S6c, native model checking.
    S6c,
    /// S7, the typed witness.
    S7,
    /// S8, native replay.
    S8,
}

impl From<SpineStage> for OutcomeStage {
    fn from(stage: SpineStage) -> Self {
        match stage {
            SpineStage::Source => Self::S1,
            SpineStage::Forms => Self::S2,
            SpineStage::Intake => Self::I1,
            SpineStage::Assembly | SpineStage::Check => Self::S3,
            SpineStage::Emit => Self::S4,
        }
    }
}

/// A proof item's result label (FR-331). It has no `undefined` member: an
/// undefined claim evaluation is `refuted` with the `undefined-evaluation`
/// cause ([`ItemCause::undefined_evaluation`]).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemLabel {
    /// The claim was proved.
    Proved,
    /// The claim was tested, never promoted to proved.
    Tested,
    /// The claim does not hold.
    Refuted,
    /// The request was declined.
    Declined,
    /// No engine or backend supplies the claim.
    Unsupported,
    /// The run did not complete.
    Incomplete,
    /// Neither proved nor refuted, a vacuous proof included.
    Inconclusive,
    /// The tool failed.
    Failed,
}

/// Why a replay settled inconclusive: its disagreement reason and the two
/// verdicts' categories.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ParityReason {
    reason: &'static str,
    proved: &'static str,
    replayed: &'static str,
}

/// The typed cause an item's terminal record carries: its FR-331 spelling in
/// `kind`, and the members that spelling names.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ItemCause {
    kind: &'static str,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cause: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parity: Option<ParityReason>,
}

impl ItemCause {
    fn kind(kind: &'static str) -> Self {
        Self {
            kind,
            at: None,
            cause: None,
            code: None,
            parity: None,
        }
    }

    /// The claim's evaluation was undefined at `at`, for `cause`: the item
    /// is refuted, category violation (FR-281, FR-283).
    pub fn undefined_evaluation(at: impl Into<String>, cause: impl Into<String>) -> Self {
        Self {
            at: Some(at.into()),
            cause: Some(cause.into()),
            ..Self::kind("undefined-evaluation")
        }
    }

    fn inconclusive(cause: &ReportedInconclusiveCause) -> Self {
        let mut item = Self::kind(cause.as_str());
        match cause {
            ReportedInconclusiveCause::KaniVacuousProof => {}
            ReportedInconclusiveCause::Cause(InconclusiveCause::ReplayParity(parity)) => {
                item.parity = Some(ParityReason {
                    reason: parity.as_str(),
                    proved: parity.proved().category().as_str(),
                    replayed: parity.replayed().category().as_str(),
                });
            }
            ReportedInconclusiveCause::Cause(InconclusiveCause::ReplayRefused(code)) => {
                item.code = Some(code.as_str().to_owned());
            }
            // The kind alone: the claim identity and outcome stay on the
            // typed cause, which holds them for the caller.
            ReportedInconclusiveCause::Cause(InconclusiveCause::ScalarAgrees(_)) => {}
        }
        item
    }
}

/// One requested item of a `prove`, `analyze` or `monitor` outcome: its
/// terminal record and its category.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct OutcomeItem {
    request_index: usize,
    result: ItemLabel,
    cause: Option<ItemCause>,
    #[serde(serialize_with = "category_str")]
    category: Category,
}

impl OutcomeItem {
    /// The item an FR-331 terminal record settles. A vacuous proof is
    /// `inconclusive` with its vacuity cause, never `proved`.
    pub fn from_terminal(record: &TerminalRecord) -> Self {
        let value = record.value();
        let (result, cause) = match value {
            TerminalValue::Proved { success_checks: 0 } | TerminalValue::Inconclusive(_) => (
                ItemLabel::Inconclusive,
                value
                    .inconclusive_cause()
                    .map(|cause| ItemCause::inconclusive(&cause)),
            ),
            TerminalValue::Proved { .. } => (ItemLabel::Proved, None),
            TerminalValue::Tested => (ItemLabel::Tested, None),
            TerminalValue::Refuted => (ItemLabel::Refuted, None),
            TerminalValue::Declined { cause, code } => (
                ItemLabel::Declined,
                Some(ItemCause {
                    code: Some(match code {
                        DeclineCode::Qsl(code) => code.as_str().to_owned(),
                        DeclineCode::Std001(code) => code.as_str().to_owned(),
                    }),
                    ..ItemCause::kind(cause.as_str())
                }),
            ),
            TerminalValue::Unsupported(cause) => (
                ItemLabel::Unsupported,
                Some(ItemCause::kind(cause.as_str())),
            ),
            TerminalValue::Incomplete(cause) => {
                (ItemLabel::Incomplete, Some(ItemCause::kind(cause.as_str())))
            }
            TerminalValue::Failed => (ItemLabel::Failed, None),
        };
        Self {
            request_index: record.request_index().get(),
            result,
            cause,
            category: value.category(),
        }
    }

    /// The item whose claim evaluated to an undefined value at `at` for
    /// `cause`: refuted, category violation, never labelled `undefined`.
    pub fn undefined_evaluation(
        request_index: RequestIndex,
        at: impl Into<String>,
        cause: impl Into<String>,
    ) -> Self {
        Self {
            request_index: request_index.get(),
            result: ItemLabel::Refuted,
            cause: Some(ItemCause::undefined_evaluation(at, cause)),
            category: Category::Violation,
        }
    }

    /// The item's O-16 category.
    pub fn category(&self) -> Category {
        self.category
    }
}

/// A diagnostic's position (ADR-013 T-5), as the document writes it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OutcomeLocus {
    /// An O-07 source region.
    Region {
        /// The source the offsets index.
        source: RawSourceRef,
        /// The region's start offset.
        start: u64,
        /// The region's end offset.
        end: u64,
    },
    /// A region rendered over its source, as an evaluation records it.
    Located {
        /// The `sha256:` digest of the source.
        source_digest: String,
        /// The region's span.
        span: LocatedSpan,
    },
    /// A kernel occurrence key, as its display form.
    Occurrence {
        /// The occurrence key's display form.
        occurrence: String,
    },
    /// A digest-addressed artifact and a JSON pointer into it.
    Artifact {
        /// The artifact's digest domain.
        domain: &'static str,
        /// The artifact's digest, lower-case hex.
        digest: String,
        /// The RFC 6901 pointer into the artifact.
        pointer: String,
    },
}

impl From<&Locus> for OutcomeLocus {
    fn from(locus: &Locus) -> Self {
        match locus {
            Locus::Region(region) => Self::Region {
                source: region.source().clone(),
                start: region.start(),
                end: region.end(),
            },
            Locus::Occurrence(location) => Self::Occurrence {
                occurrence: OccurrenceKey::from(location).to_string(),
            },
            Locus::Artifact { digest, pointer } => Self::Artifact {
                domain: digest.domain().as_str(),
                digest: digest.hex(),
                pointer: pointer.to_string(),
            },
        }
    }
}

/// One diagnostic: its typed cause, catalog code, locus and message.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct OutcomeDiagnostic {
    cause: Option<&'static str>,
    code: String,
    locus: Option<OutcomeLocus>,
    message: String,
}

impl OutcomeDiagnostic {
    /// A diagnostic with the catalog `code` and its typed `cause` (the
    /// catalog's cause tag, where the code has one), raised at `locus`.
    pub fn new(
        cause: Option<&'static str>,
        code: impl Into<String>,
        locus: Option<OutcomeLocus>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            cause,
            code: code.into(),
            locus,
            message: message.into(),
        }
    }

    /// The diagnostic of a spine compile refusal: its catalog code, cause,
    /// region and message.
    pub fn from_compile_refusal(refusal: &CompileRefusal) -> Self {
        Self::new(
            refusal.cause(),
            refusal.code().as_str(),
            refusal
                .region()
                .map(|region| OutcomeLocus::from(&Locus::Region(region.clone()))),
            refusal.to_string(),
        )
    }

    /// The diagnostic of a refused call: its catalog code and cause, and
    /// the resolved region its record carries as locus.
    pub fn from_call_refusal(refusal: &CallRefusal) -> Self {
        let (CallRefusal::Record { code, .. } | CallRefusal::Family { code, .. }) = refusal;
        let locus = match refusal {
            CallRefusal::Record {
                locus: Some(locus), ..
            } => Some(OutcomeLocus::Located {
                source_digest: locus.source_digest.clone(),
                span: locus.span,
            }),
            CallRefusal::Record { locus: None, .. } | CallRefusal::Family { .. } => None,
        };
        Self::new(Some(code.cause()), code.code(), locus, code.to_string())
    }

    /// The catalog code.
    pub fn code(&self) -> &str {
        &self.code
    }

    /// The typed cause tag.
    pub fn cause(&self) -> Option<&'static str> {
        self.cause
    }

    /// The message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// The kind of identity an artifact member holds.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    /// An emitted package's `package_id`.
    PackageId,
    /// A generated artifact's content identity.
    Generated,
}

/// The identity of one artifact an operation produced.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct OutcomeArtifact {
    kind: ArtifactKind,
    id: String,
}

impl OutcomeArtifact {
    /// An emitted package's identity.
    pub fn package_id(id: &PackageId) -> Self {
        Self {
            kind: ArtifactKind::PackageId,
            id: id.hex(),
        }
    }

    /// A generated artifact's content identity.
    pub fn generated(digest: &DigestRecord) -> Self {
        Self {
            kind: ArtifactKind::Generated,
            id: digest.hex(),
        }
    }

    /// The identity, lower-case hex.
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// A completed call's value, in FR-100's rendering.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResultValue {
    /// A `Boolean` value.
    Boolean {
        /// The value.
        value: bool,
    },
    /// An integer value, in FR-038's ASCII decimal spelling.
    Integer {
        /// The decimal digits.
        decimal: String,
    },
}

/// The evaluation result of an `execute` outcome (FR-286's `result`).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExecuteResult {
    /// The call completed with a value.
    Completed {
        /// The value.
        value: ResultValue,
    },
    /// The call's result is undefined.
    Undefined {
        /// The reason, as FR-100's tables spell it.
        reason: &'static str,
    },
    /// An accounting limit stopped the call.
    Incomplete {
        /// The exhausted limit.
        limit: ResultLimit,
    },
}

/// The exhausted limit of an incomplete `execute` result (FR-277): bound and
/// counter (FR-277's counter at the failed charge) are
/// ASCII decimal strings.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ResultLimit {
    kind: &'static str,
    bound: String,
    counter: String,
    field: &'static str,
}

impl From<&CallIncomplete> for ResultLimit {
    fn from(incomplete: &CallIncomplete) -> Self {
        Self {
            kind: incomplete.record.limit_kind.as_str(),
            bound: incomplete.record.limit.to_string(),
            counter: incomplete.counter().to_string(),
            field: incomplete.limits_field(),
        }
    }
}

/// The `quire-outcome/1` document (FR-286).
///
/// `items` is always written, an empty array for an operation with no
/// requested items. `last_stage` is `null` when no stage is known to have
/// run. `result` is always written: `null` except for a completed,
/// undefined or incomplete `execute` outcome.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct OutcomeDocument {
    format: &'static str,
    operation: Operation,
    last_stage: Option<OutcomeStage>,
    #[serde(serialize_with = "category_str")]
    category: Category,
    items: Vec<OutcomeItem>,
    diagnostics: Vec<OutcomeDiagnostic>,
    artifacts: Vec<OutcomeArtifact>,
    result: Option<ExecuteResult>,
}

/// Why a document could not be written.
#[derive(Debug, thiserror::Error)]
#[error("the outcome document could not be encoded: {0}")]
pub struct OutcomeWriteError(#[from] serde_json::Error);

impl OutcomeDocument {
    /// A document of `operation` that last reached `last_stage` with the
    /// outcome's `category`, and no items, diagnostics or artifacts.
    pub fn new(operation: Operation, last_stage: Option<OutcomeStage>, category: Category) -> Self {
        Self {
            format: OUTCOME_FORMAT,
            operation,
            last_stage,
            category,
            items: Vec::new(),
            diagnostics: Vec::new(),
            artifacts: Vec::new(),
            result: None,
        }
    }

    /// A document of `operation` over its requested `items`, in request
    /// order. Its category is the most severe item's under ADR-029 CB-4
    /// (FR-285's order 30, 20, 21, 22, 10, 0), the first such item's on a
    /// tie, and `success` with no items. A `monitor` clause still pending
    /// when the trace ends takes the trace row.
    pub fn settled(
        operation: Operation,
        last_stage: Option<OutcomeStage>,
        items: Vec<OutcomeItem>,
    ) -> Self {
        let code = |item: &OutcomeItem| match operation {
            Operation::Monitor => item.category.trace_exit_code(),
            _ => item.category.exit_code(),
        };
        let category = Category::most_severe(items.iter().map(code))
            .and_then(|worst| items.iter().find(|item| code(item) == worst))
            .map_or(Category::Success, |item| item.category);
        Self::new(operation, last_stage, category).with_items(items)
    }

    /// This document with one entry per requested item, in request order.
    #[must_use]
    pub fn with_items(mut self, items: Vec<OutcomeItem>) -> Self {
        self.items = items;
        self
    }

    /// This document with its diagnostics.
    #[must_use]
    pub fn with_diagnostics(mut self, diagnostics: Vec<OutcomeDiagnostic>) -> Self {
        self.diagnostics = diagnostics;
        self
    }

    /// This document with the identities of the artifacts produced.
    #[must_use]
    pub fn with_artifacts(mut self, artifacts: Vec<OutcomeArtifact>) -> Self {
        self.artifacts = artifacts;
        self
    }

    /// The outcome's O-16 category, from which FR-285 derives the exit code.
    pub fn category(&self) -> Category {
        self.category
    }

    /// The document's items, in request order.
    pub fn items(&self) -> &[OutcomeItem] {
        &self.items
    }

    /// The document's diagnostics.
    pub fn diagnostics(&self) -> &[OutcomeDiagnostic] {
        &self.diagnostics
    }

    /// The document of a `check` outcome: success at S4, which mints no
    /// identity, so no artifact; a failure names its category and one
    /// diagnostic.
    pub fn from_check(result: &Result<Staged<CheckedUnit>, FrontEndFailure>) -> Self {
        match result {
            Ok(_) => Self::new(Operation::Check, Some(OutcomeStage::S4), Category::Success),
            Err(failure) => Self::from_failure(Operation::Check, failure),
        }
    }

    /// The document of a `package` outcome: success at S4 holds the
    /// emitted package's `package_id`.
    pub fn from_package(result: &Result<Staged<EmittedUnit>, FrontEndFailure>) -> Self {
        match result {
            Ok(staged) => Self::new(
                Operation::Package,
                Some(OutcomeStage::S4),
                Category::Success,
            )
            .with_artifacts(vec![OutcomeArtifact::package_id(
                &staged.value().package().package_id(),
            )]),
            Err(failure) => Self::from_failure(Operation::Package, failure),
        }
    }

    /// The document of a front-end operation's failure.
    fn from_failure(operation: Operation, failure: &FrontEndFailure) -> Self {
        match failure {
            StageFailure::Refused(refusal) => Self::new(
                operation,
                Some(refusal.stage().into()),
                refusal.code().category(),
            )
            .with_diagnostics(vec![OutcomeDiagnostic::from_compile_refusal(refusal)]),
            StageFailure::Limit(limit) => {
                let refusal = CompileRefusal::Limit(limit.clone());
                Self::new(
                    operation,
                    Some(refusal.stage().into()),
                    Category::Incomplete,
                )
                .with_diagnostics(vec![OutcomeDiagnostic::from_compile_refusal(&refusal)])
            }
            StageFailure::Cancelled(cause) => Self::new(operation, None, Category::Incomplete)
                .with_diagnostics(vec![OutcomeDiagnostic::new(
                    Some(match cause {
                        CancelCause::Requested => "requested",
                        CancelCause::Deadline => "deadline",
                    }),
                    Code::Cancelled.as_str(),
                    None,
                    "the operation was cancelled",
                )]),
            StageFailure::Fault(fault) => Self::new(operation, None, fault.category())
                .with_diagnostics(vec![OutcomeDiagnostic::new(
                    Some(fault.catalog_code().cause()),
                    fault.catalog_code().code(),
                    None,
                    format!(
                        "internal invariant {} broken in {}",
                        fault.invariant(),
                        fault.stage()
                    ),
                )]),
        }
    }

    /// The document of a `replay` outcome: a settled arm's category at S8,
    /// or the refusal's category and diagnostic.
    pub fn from_replay(result: &Result<ReplayResult, ReplayRefusal>) -> Self {
        match result {
            Ok(ReplayResult::Witness(arm)) => {
                Self::new(Operation::Replay, Some(OutcomeStage::S8), arm.category())
            }
            Ok(ReplayResult::Input(arm)) => {
                Self::new(Operation::Replay, Some(OutcomeStage::S8), arm.category())
            }
            Err(refusal) => Self::new(
                Operation::Replay,
                replay_stage(refusal),
                refusal.code().category(),
            )
            .with_diagnostics(vec![OutcomeDiagnostic::new(
                refusal.cause(),
                refusal.code().as_str(),
                None,
                refusal.to_string(),
            )]),
        }
    }

    /// The document of an `execute` outcome, last reaching S6a. A refused
    /// call carries its catalog code, cause and locus as one diagnostic; a
    /// completed or undefined call carries its `result`. An undefined call has category `undefined`, the one place that label
    /// appears.
    pub fn from_call(outcome: &CallOutcome) -> Self {
        let mut document = Self::new(
            Operation::Execute,
            Some(OutcomeStage::S6a),
            outcome.category(),
        );
        match outcome {
            CallOutcome::Refused(refusal) => {
                document.diagnostics = vec![OutcomeDiagnostic::from_call_refusal(refusal)];
            }
            CallOutcome::Completed(value) => {
                document.result = Some(ExecuteResult::Completed {
                    value: match value {
                        CallValue::Boolean(value) => ResultValue::Boolean { value: *value },
                        CallValue::Integer(value) => ResultValue::Integer {
                            decimal: value.to_string(),
                        },
                    },
                });
            }
            CallOutcome::Undefined { reason } => {
                document.result = Some(ExecuteResult::Undefined { reason });
            }
            CallOutcome::Incomplete(incomplete) => {
                document.result = Some(ExecuteResult::Incomplete {
                    limit: ResultLimit::from(incomplete),
                });
            }
        }
        document
    }

    /// The document's bytes: the same document gives the same bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, OutcomeWriteError> {
        Ok(serde_json::to_vec(self)?)
    }
}

/// The last stage a replay refusal reached: a recompile refusal's own spine
/// stage, S8 for every other refusal (the replay itself refused), and none
/// for a fault, whose stage is untyped.
fn replay_stage(refusal: &ReplayRefusal) -> Option<OutcomeStage> {
    match refusal {
        ReplayRefusal::Recompile(refusal) => Some(refusal.stage().into()),
        ReplayRefusal::Fault(_) | ReplayRefusal::Admission(AdmissionFailure::Fault(_)) => None,
        ReplayRefusal::Request(_)
        | ReplayRefusal::LimitAboveReader(_)
        | ReplayRefusal::NotASource(_)
        | ReplayRefusal::SourceCount(_)
        | ReplayRefusal::DependencySelections(_)
        | ReplayRefusal::DependencyInput(_)
        | ReplayRefusal::DependencyIdentityMismatch { .. }
        | ReplayRefusal::PackageIdMismatch { .. }
        | ReplayRefusal::UnknownFunction { .. }
        | ReplayRefusal::UnknownOperation { .. }
        | ReplayRefusal::FrameIdentity(_)
        | ReplayRefusal::ClauseIdentity(_)
        | ReplayRefusal::UnknownClause { .. }
        | ReplayRefusal::WrongObservation { .. }
        | ReplayRefusal::Admission(_)
        | ReplayRefusal::UnknownParameter(_)
        | ReplayRefusal::DuplicateArgument(_)
        | ReplayRefusal::UnboundParameter(_)
        | ReplayRefusal::Witness { .. }
        | ReplayRefusal::NotAPredicate { .. }
        | ReplayRefusal::NotAValueFunction { .. }
        | ReplayRefusal::ScalarIdentity(_)
        | ReplayRefusal::ParityBound(_)
        | ReplayRefusal::Input(_) => Some(OutcomeStage::S8),
    }
}

fn category_str<S: Serializer>(category: &Category, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(category.as_str())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ix_trace_rs::trace;
    use qsl_foundation::diagnostic::{CatalogCode, InternalFault};
    use qsl_foundation::digest::DigestDomain;
    use qsl_foundation::{Position, SourceIdentity};
    use qsl_semantics::library::LibraryName;
    use quire_exact::{Cancel, CancelCause, Identifier, ScalarLimits};
    use serde_json::{json, Value};

    use super::*;
    use crate::identity::QualifiedName;
    use crate::proof_result::{IncompleteCause, ProofRefusalCause, UnavailabilityCause};
    use crate::result::{DisagreementCause, EvaluatedValue, InputArmResult, Verdict, WitnessCheck};
    use crate::spine::{
        check, default_accounting, package, parse, run, select, Call, CallLocus, DependencyInput,
        LockEvidence, PackageLimits, ParseRequest, SpineLimits,
    };

    const FIXTURE: &str = include_str!("../../tests/fixtures/spine-compile.native");
    const ILL_TYPED: &str = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"quire.value.complete/v1\";\n\
        type Digit = Int[0, 9];\n\
        function inv using v(x: Digit): Boolean pure { 1 / x > 0 }\n";

    type Checked = Result<Staged<CheckedUnit>, FrontEndFailure>;
    type Packaged = Result<Staged<EmittedUnit>, FrontEndFailure>;

    /// `check`, then `package`, over `text`.
    fn compiled(text: &str) -> (Checked, Option<Packaged>) {
        let source = SourceIdentity::new("agent-ix", "test:outcome", "fixture", "fixture:1");
        let limits = SpineLimits::default();
        let cancel = Cancel::new();
        let request = ParseRequest {
            source: &source,
            path: "unit.native",
            bytes: text.as_bytes(),
        };
        let parsed = parse(&request, limits.source, &cancel)
            .expect("parses")
            .into_value();
        let models = select(
            &parsed,
            &BTreeMap::new(),
            limits.model,
            &cancel,
        )
        .expect("selects")
        .into_value();
        let checked = check(
            &parsed,
            &models,
            &DependencyInput::default(),
            &LockEvidence::default(),
            limits,
            &cancel,
        );
        let emitted = checked
            .as_ref()
            .ok()
            .map(|staged| package(staged.value(), PackageLimits::default(), &cancel));
        (checked, emitted)
    }

    fn json_of(document: &OutcomeDocument) -> Value {
        serde_json::from_slice(&document.to_bytes().expect("encodes")).expect("is JSON")
    }

    fn text_of(document: &OutcomeDocument) -> String {
        String::from_utf8(document.to_bytes().expect("encodes")).expect("is UTF-8")
    }

    fn record(index: usize, value: TerminalValue) -> OutcomeItem {
        OutcomeItem::from_terminal(&TerminalRecord::new(RequestIndex::new(index), value))
    }

    /// FR-286-AC-1: `check` over the spine fixture is one whole document, at
    /// S4 with no diagnostics; the package identity is `package`'s.
    #[trace("TC-770", "FR-286-AC-1")]
    #[test]
    fn check_and_package_documents_over_the_fixture() {
        let (checked, emitted) = compiled(FIXTURE);
        assert_eq!(
            json_of(&OutcomeDocument::from_check(&checked)),
            json!({
                "format": "quire-outcome/1",
                "operation": "check",
                "last_stage": "S4",
                "category": "success",
                "items": [],
                "diagnostics": [],
                "artifacts": [],
                "result": null,
            })
        );
        let emitted = emitted.expect("the fixture checks");
        let id = emitted
            .as_ref()
            .expect("the fixture packages")
            .value()
            .package()
            .package_id()
            .hex();
        assert_eq!(id.len(), 64);
        assert_eq!(
            json_of(&OutcomeDocument::from_package(&emitted)),
            json!({
                "format": "quire-outcome/1",
                "operation": "package",
                "last_stage": "S4",
                "category": "success",
                "items": [],
                "diagnostics": [],
                "artifacts": [{"kind": "package_id", "id": id}],
                "result": null,
            })
        );
    }

    /// FR-286-AC-2: the `check` outcome of the `inv` source refuses with one
    /// diagnostic equal to the refusal's code, cause, locus and message.
    #[trace("TC-770", "FR-286-AC-2")]
    #[test]
    fn check_refusal_document_carries_the_refusals_diagnostic() {
        let (result, _) = compiled(ILL_TYPED);
        let Err(StageFailure::Refused(refusal)) = &result else {
            panic!("expected a refusal");
        };
        let region = refusal.region().expect("the refusal is located");
        let document = json_of(&OutcomeDocument::from_check(&result));
        assert_eq!(
            document,
            json!({
                "format": "quire-outcome/1",
                "operation": "check",
                "last_stage": "S3",
                "category": "refusal",
                "items": [],
                "diagnostics": [{
                    "cause": "ambiguous-literal",
                    "code": "ill_typed",
                    "locus": {
                        "kind": "region",
                        "source": serde_json::to_value(region.source()).expect("a source ref"),
                        "start": region.start(),
                        "end": region.end(),
                    },
                    "message": refusal.to_string(),
                }],
                "artifacts": [],
                "result": null,
            })
        );
    }

    /// FR-286-AC-3: three items serialize in request order with their
    /// records and categories, to exactly these bytes, twice over.
    #[trace("TC-770", "FR-286-AC-3")]
    #[test]
    fn analyze_items_keep_request_order_and_bytes_are_stable() {
        let document = OutcomeDocument::settled(
            Operation::Analyze,
            Some(OutcomeStage::S6c),
            vec![
                record(
                    0,
                    TerminalValue::Unsupported(UnavailabilityCause::SolverAbsent),
                ),
                record(1, TerminalValue::Proved { success_checks: 3 }),
                record(2, TerminalValue::Refuted),
            ],
        );
        let expected = concat!(
            r#"{"format":"quire-outcome/1","operation":"analyze","last_stage":"S6c","#,
            r#""category":"unsupported","items":["#,
            r#"{"request_index":0,"result":"unsupported","cause":{"kind":"solver-absent"},"category":"unsupported"},"#,
            r#"{"request_index":1,"result":"proved","cause":null,"category":"success"},"#,
            r#"{"request_index":2,"result":"refuted","cause":null,"category":"violation"}"#,
            r#"],"diagnostics":[],"artifacts":[],"result":null}"#
        );
        assert_eq!(text_of(&document), expected);
        assert_eq!(document.to_bytes().unwrap(), document.to_bytes().unwrap());
        let (refused, _) = compiled(ILL_TYPED);
        assert_eq!(
            OutcomeDocument::from_check(&refused).to_bytes().unwrap(),
            OutcomeDocument::from_check(&refused).to_bytes().unwrap()
        );
    }

    /// QSpec FR-331-AC-8: a proof with no SUCCESS check is `inconclusive`
    /// with its vacuity cause, never `proved`.
    #[trace("TC-770", "FR-286-AC-3")]
    #[test]
    fn a_vacuous_proof_is_inconclusive_never_proved() {
        let item = serde_json::to_value(record(4, TerminalValue::Proved { success_checks: 0 }))
            .expect("serializes");
        assert_eq!(
            item,
            json!({
                "request_index": 4,
                "result": "inconclusive",
                "cause": {"kind": "kani-vacuous-proof"},
                "category": "inconclusive",
            })
        );
    }

    /// Each terminal cause is written in its FR-331 spelling, with its
    /// payload.
    #[trace("TC-770", "FR-286-AC-3", "FR-357-AC-12")]
    #[test]
    fn terminal_causes_use_their_own_spellings() {
        let parity = DisagreementCause::Verdicts {
            proved: Verdict::from_category(Category::Success),
            replayed: Verdict::from_category(Category::Violation),
        };
        let cases = [
            (
                TerminalValue::Declined {
                    cause: ProofRefusalCause::InvalidInput,
                    code: DeclineCode::Qsl(Code::IllTyped),
                },
                json!({"result": "declined", "cause": {"kind": "invalid-input", "code": "ill_typed"}, "category": "refusal"}),
            ),
            (
                TerminalValue::Incomplete(IncompleteCause::Cancelled),
                json!({"result": "incomplete", "cause": {"kind": "cancelled"}, "category": "incomplete"}),
            ),
            (
                TerminalValue::Incomplete(IncompleteCause::ResourceExhausted),
                json!({"result": "incomplete", "cause": {"kind": "limit-reached"}, "category": "incomplete"}),
            ),
            (
                TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(
                    Code::StaleDependency,
                )),
                json!({"result": "inconclusive", "cause": {"kind": "replay-refused", "code": "stale_dependency"}, "category": "inconclusive"}),
            ),
            (
                TerminalValue::Inconclusive(InconclusiveCause::ScalarAgrees(
                    crate::scalar::ScalarAgreement::new(
                        crate::scalar::ScalarClaim::Function(Box::new(crate::ValueIdentity {
                            obligation: crate::identity::ObligationIdentity::from_digest([1; 32]),
                            package_id: (None, String::new()),
                            function: QualifiedName::new(vec![Identifier::new("f").unwrap()])
                                .unwrap(),
                            source: crate::ReplaySource::Input(Vec::new()),
                            limits: quire_exact::ScalarLimits {
                                integer_bits: 1,
                                decimal_digits: 1,
                                scale_expansion: 1,
                                text_input_bytes: 1,
                                text_scalars: 1,
                                normalized_scalars: 1,
                                unit_edges: 1,
                                value_occurrences: 1,
                                work_units: 1,
                                result_units: 1,
                            },
                            generated: crate::scalar::ScalarOutcome::OutOfRange,
                        })),
                        crate::scalar::ScalarOutcome::OutOfRange,
                    ),
                )),
                json!({"result": "inconclusive", "cause": {"kind": "scalar-agrees"}, "category": "inconclusive"}),
            ),
            (
                TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(parity)),
                json!({"result": "inconclusive", "cause": {"kind": "replay-parity", "parity": {"reason": "verdicts", "proved": "success", "replayed": "violation"}}, "category": "inconclusive"}),
            ),
            (
                TerminalValue::Failed,
                json!({"result": "failed", "cause": null, "category": "internal-failure"}),
            ),
        ];
        for (value, mut expected) in cases {
            expected["request_index"] = json!(1);
            assert_eq!(
                serde_json::to_value(record(1, value.clone())).expect("serializes"),
                expected,
                "{value:?}"
            );
        }
    }

    /// FR-286-AC-4: an undefined claim evaluation in an `analyze` or
    /// `monitor` item is a violation with cause `undefined-evaluation` and no
    /// `undefined` label; an `execute` outcome that is undefined keeps it.
    #[trace("TC-770", "FR-286-AC-4")]
    #[test]
    fn undefined_label_appears_only_on_a_non_proof_outcome() {
        for (operation, name) in [
            (Operation::Analyze, "analyze"),
            (Operation::Monitor, "monitor"),
        ] {
            let document = OutcomeDocument::settled(
                operation,
                Some(OutcomeStage::S6c),
                vec![OutcomeItem::undefined_evaluation(
                    RequestIndex::new(0),
                    "position 1",
                    "division-by-zero",
                )],
            );
            assert!(!text_of(&document).contains("\"undefined\""));
            assert_eq!(
                json_of(&document),
                json!({
                    "format": "quire-outcome/1",
                    "operation": name,
                    "last_stage": "S6c",
                    "category": "violation",
                    "items": [{
                        "request_index": 0,
                        "result": "refuted",
                        "cause": {
                            "kind": "undefined-evaluation",
                            "where": "position 1",
                            "cause": "division-by-zero",
                        },
                        "category": "violation",
                    }],
                    "diagnostics": [],
                    "artifacts": [],
                    "result": null,
                })
            );
        }
        let execute = OutcomeDocument::from_call(&CallOutcome::Undefined {
            reason: "sum-out-of-domain",
        });
        assert_eq!(
            json_of(&execute),
            json!({
                "format": "quire-outcome/1",
                "operation": "execute",
                "last_stage": "S6a",
                "category": "undefined",
                "items": [],
                "diagnostics": [],
                "artifacts": [],
                "result": {"kind": "undefined", "reason": "sum-out-of-domain"},
            })
        );
    }

    /// FR-286-AC-5: `seven` completes with integer 7 in the `result`
    /// member, and a `check` under an already cancelled handle is incomplete
    /// with no last stage and a null `result`.
    #[trace("TC-770", "FR-286-AC-5")]
    #[test]
    fn execute_result_and_cancelled_check_documents() {
        let call = Call {
            function: "seven".to_owned(),
            arguments: Vec::new(),
            accounting: default_accounting(1_000_000),
        };
        let (_, outcome) = run(
            SourceIdentity::new("agent-ix", "test:outcome", "fixture", "fixture:1"),
            "unit.native",
            FIXTURE.as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
            SpineLimits::default(),
            &call,
        )
        .expect("seven runs");
        assert_eq!(
            json_of(&OutcomeDocument::from_call(&outcome)),
            json!({
                "format": "quire-outcome/1",
                "operation": "execute",
                "last_stage": "S6a",
                "category": "success",
                "items": [],
                "diagnostics": [],
                "artifacts": [],
                "result": {"kind": "completed", "value": {"kind": "integer", "decimal": "7"}},
            })
        );

        let starved = Call {
            accounting: default_accounting(0),
            ..call
        };
        let (_, outcome) = run(
            SourceIdentity::new("agent-ix", "test:outcome", "fixture", "fixture:1"),
            "unit.native",
            FIXTURE.as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
            SpineLimits::default(),
            &starved,
        )
        .expect("seven runs");
        assert_eq!(
            json_of(&OutcomeDocument::from_call(&outcome)),
            json!({
                "format": "quire-outcome/1",
                "operation": "execute",
                "last_stage": "S6a",
                "category": "incomplete",
                "items": [],
                "diagnostics": [],
                "artifacts": [],
                "result": {"kind": "incomplete", "limit": {
                    "kind": "work_units", "bound": "0", "counter": "1", "field": "work_units"}},
            })
        );

        let source = SourceIdentity::new("agent-ix", "test:outcome", "fixture", "fixture:1");
        let limits = SpineLimits::default();
        let live = Cancel::new();
        let request = ParseRequest {
            source: &source,
            path: "unit.native",
            bytes: FIXTURE.as_bytes(),
        };
        let parsed = parse(&request, limits.source, &live)
            .expect("parses")
            .into_value();
        let models = select(
            &parsed,
            &BTreeMap::new(),
            limits.model,
            &live,
        )
        .expect("selects")
        .into_value();
        let cancelled = Cancel::new();
        cancelled.cancel(CancelCause::Requested);
        let result = check(
            &parsed,
            &models,
            &DependencyInput::default(),
            &LockEvidence::default(),
            limits,
            &cancelled,
        );
        let document = json_of(&OutcomeDocument::from_check(&result));
        assert_eq!(document["category"], "incomplete");
        assert_eq!(document["last_stage"], Value::Null);
        assert_eq!(document["items"], json!([]));
        assert_eq!(document["result"], Value::Null);
    }

    /// The whole-outcome category is the most severe item's, and a monitor
    /// clause pending at the end of the trace stays below a violation.
    #[trace("TC-770", "FR-286-AC-3")]
    #[test]
    fn the_document_category_is_the_most_severe_items() {
        let declined = TerminalValue::Declined {
            cause: ProofRefusalCause::Refused,
            code: DeclineCode::Qsl(Code::IllTyped),
        };
        let mixed = OutcomeDocument::settled(
            Operation::Prove,
            Some(OutcomeStage::S8),
            vec![
                record(0, TerminalValue::Proved { success_checks: 1 }),
                record(1, TerminalValue::Refuted),
                record(2, declined),
                record(3, TerminalValue::Incomplete(IncompleteCause::TimedOut)),
            ],
        );
        assert_eq!(mixed.category(), Category::Refusal);
        let pending = || {
            vec![record(
                0,
                TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(
                    Code::StaleDependency,
                )),
            )]
        };
        let monitored = OutcomeDocument::settled(Operation::Monitor, None, pending());
        assert_eq!(monitored.category(), Category::Inconclusive);
        assert_eq!(monitored.category().trace_exit_code(), 0);
        let violated = OutcomeDocument::settled(
            Operation::Monitor,
            None,
            pending()
                .into_iter()
                .chain([record(1, TerminalValue::Refuted)])
                .collect(),
        );
        assert_eq!(violated.category(), Category::Violation);
        assert_eq!(
            OutcomeDocument::settled(Operation::Prove, None, pending()).category(),
            Category::Inconclusive
        );
        assert_eq!(
            OutcomeDocument::settled(Operation::Prove, None, Vec::new()).category(),
            Category::Success
        );
    }

    /// A refused call's diagnostic keeps the refusal's locus.
    #[trace("TC-770", "FR-286-AC-2")]
    #[test]
    fn a_refused_call_keeps_its_locus() {
        let span = qsl_foundation::LocatedSpan {
            start: Position {
                byte: 4,
                line: 1,
                column: 5,
            },
            end: Position {
                byte: 9,
                line: 1,
                column: 10,
            },
        };
        let refusal = CallRefusal::Record {
            code: CatalogCode::new("ill_typed", "type-mismatch"),
            fields: BTreeMap::new(),
            locus: Some(CallLocus {
                source_digest: "sha256:ab".to_owned(),
                span,
            }),
            location: None,
        };
        let document = json_of(&OutcomeDocument::from_call(&CallOutcome::Refused(refusal)));
        assert_eq!(document["category"], "refusal");
        assert_eq!(document["diagnostics"][0]["code"], "ill_typed");
        assert_eq!(document["diagnostics"][0]["cause"], "type-mismatch");
        assert_eq!(
            document["diagnostics"][0]["locus"],
            json!({
                "kind": "located",
                "source_digest": "sha256:ab",
                "span": serde_json::to_value(span).expect("a span"),
            })
        );
    }

    /// A replay outcome is its arm's category at S8, or its refusal's.
    #[trace("TC-770", "FR-286-AC-3")]
    #[test]
    fn replay_documents_follow_the_arm_and_the_refusal() {
        let charges = ScalarLimits {
            integer_bits: 64,
            decimal_digits: 34,
            scale_expansion: 8,
            text_input_bytes: 1024,
            text_scalars: 1024,
            normalized_scalars: 1024,
            unit_edges: 4,
            value_occurrences: 16,
            work_units: 100,
            result_units: 10,
        };
        let success = Verdict::from_category(Category::Success);
        let arm = InputArmResult::settle(
            success,
            success,
            Category::Success,
            Some(EvaluatedValue::Boolean(true)),
            &WitnessCheck::Agrees(None),
            Vec::new(),
            charges,
        );
        assert_eq!(
            json_of(&OutcomeDocument::from_replay(&Ok(ReplayResult::Input(arm)))),
            json!({
                "format": "quire-outcome/1",
                "operation": "replay",
                "last_stage": "S8",
                "category": "success",
                "items": [],
                "diagnostics": [],
                "artifacts": [],
                "result": null,
            })
        );
        let mismatch = ReplayRefusal::DependencyIdentityMismatch {
            identity: LibraryName::new("test/geometry").expect("a library name"),
            requested: DigestRecord::mint(DigestDomain::PackageSemanticV2, [1; 32]),
            recompiled: PackageId::of_preimage(b"other"),
        };
        let stale = json_of(&OutcomeDocument::from_replay(&Err(mismatch)));
        assert_eq!(stale["last_stage"], "S8");
        assert_eq!(stale["category"], "refusal");
        assert_eq!(stale["diagnostics"][0]["code"], "stale_dependency");
        assert_eq!(stale["diagnostics"][0]["cause"], "content-mismatch");
        let changed = ReplayRefusal::PackageIdMismatch {
            requested: DigestRecord::mint(DigestDomain::PackageSemanticV2, [1; 32]),
            recompiled: PackageId::of_preimage(b"other"),
        };
        let changed = json_of(&OutcomeDocument::from_replay(&Err(changed)));
        assert_eq!(changed["last_stage"], "S8");
        assert_eq!(changed["diagnostics"][0]["code"], "stale_dependency");
        assert_eq!(changed["diagnostics"][0]["cause"], "content-mismatch");
        let unknown = ReplayRefusal::UnknownFunction {
            selection: QualifiedName::new(vec![Identifier::new("seven").expect("an identifier")])
                .expect("a qualified name"),
            package: PackageId::of_preimage(b"geometry"),
        };
        let unknown = json_of(&OutcomeDocument::from_replay(&Err(unknown)));
        assert_eq!(unknown["last_stage"], "S8");
        assert_eq!(unknown["diagnostics"][0]["code"], "missing_declaration");
        assert_eq!(unknown["diagnostics"][0]["cause"], "missing-name");
        let fault = ReplayRefusal::Fault(InternalFault::new("replay", "broken"));
        let document = json_of(&OutcomeDocument::from_replay(&Err(fault)));
        assert_eq!(document["category"], "internal-failure");
        assert_eq!(document["last_stage"], Value::Null);
        assert_eq!(document["diagnostics"][0]["code"], "runtime_invariant");
    }
}
