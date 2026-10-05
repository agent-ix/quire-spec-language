// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-286 (ADR-029 CB-4): the one `quire-outcome/1` JSON outcome document
//! every QSL lifecycle operation's outcome serializes to, and its one
//! serializer.
//!
//! A driver builds a document with [`OutcomeDocument::new`] and its `with_*`
//! methods (items, diagnostics, artifacts) and writes it with
//! [`OutcomeDocument::to_bytes`]; `check` and `execute` have constructors
//! over their own outcome types ([`OutcomeDocument::from_check`],
//! [`OutcomeDocument::from_call`]). The CLI in machine mode writes exactly
//! these bytes, so the library and the CLI give one document for one
//! request (QSpec FR-300-AC-3).
//!
//! The document is a typed value: its members and their spellings are the
//! types below, not keys inserted by hand. The same value serializes to the
//! same bytes every time. A proof item's label type has no `undefined`
//! member, so an undefined claim evaluation can only be written as a
//! `refuted` item with the [`ItemCause::UndefinedEvaluation`] cause and
//! category violation; the label `undefined` appears only as the category of
//! a non-proof evaluation outcome such as `execute`.

use qsl_foundation::diagnostic::{Category, Code, Locus, StageFailure, Staged};
use qsl_foundation::digest::DigestRecord;
use qsl_foundation::source::provenance::{OccurrenceKey, RawSourceRef};
use qsl_foundation::RequestIndex;
use qsl_package::emit_checked;
use qsl_semantics::library::PackageId;
use quire_exact::CancelCause;
use serde::{Serialize, Serializer};

use crate::proof_result::{
    DeclineCode, IncompleteCause, InconclusiveCause, ProofRefusalCause, TerminalRecord,
    TerminalValue, UnavailabilityCause,
};
use crate::spine::{
    CallOutcome, CallRefusal, CheckedUnit, CompileRefusal, FrontEndFailure, SpineStage,
};

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
/// undefined claim evaluation is `Refuted` with
/// [`ItemCause::UndefinedEvaluation`].
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
    /// Neither proved nor refuted.
    Inconclusive,
    /// The tool failed.
    Failed,
}

/// The typed cause an item's terminal record carries.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum ItemCause {
    /// The claim's evaluation was undefined at `at`, for `cause`: the item
    /// is refuted, category violation (FR-281, FR-283).
    #[serde(rename = "UndefinedEvaluation")]
    UndefinedEvaluation {
        /// Where the evaluation was undefined.
        #[serde(rename = "where")]
        at: String,
        /// Why it was undefined, such as `division-by-zero`.
        cause: String,
    },
    /// A counterexample's replay settled inconclusive.
    #[serde(rename = "replay_parity")]
    ReplayParity,
    /// A counterexample's replay refused.
    #[serde(rename = "replay_refused")]
    ReplayRefused {
        /// The refusal's catalog code.
        code: String,
    },
    /// A proof with no SUCCESS check.
    #[serde(rename = "kani_vacuous_proof")]
    KaniVacuousProof,
    /// A declined request.
    #[serde(rename = "declined")]
    Declined {
        /// The refusal cause.
        cause: &'static str,
        /// The refusal's code.
        code: String,
    },
    /// An unavailable solver or backend.
    #[serde(rename = "unsupported")]
    Unsupported {
        /// What is absent.
        cause: &'static str,
    },
    /// A run that did not complete.
    #[serde(rename = "incomplete")]
    Incomplete {
        /// Why it did not complete.
        cause: &'static str,
    },
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
    /// The item an FR-331 terminal record settles.
    pub fn from_terminal(record: &TerminalRecord) -> Self {
        let value = record.value();
        let (result, cause) = match value {
            TerminalValue::Proved { success_checks: 0 } => {
                (ItemLabel::Proved, Some(ItemCause::KaniVacuousProof))
            }
            TerminalValue::Proved { .. } => (ItemLabel::Proved, None),
            TerminalValue::Tested => (ItemLabel::Tested, None),
            TerminalValue::Refuted => (ItemLabel::Refuted, None),
            TerminalValue::Declined { cause, code } => (
                ItemLabel::Declined,
                Some(ItemCause::Declined {
                    cause: match cause {
                        ProofRefusalCause::Refused => "refused",
                        ProofRefusalCause::InvalidInput => "invalid_input",
                        ProofRefusalCause::IncompleteInput => "incomplete_input",
                    },
                    code: match code {
                        DeclineCode::Qsl(code) => code.as_str().to_owned(),
                        DeclineCode::Std001(code) => code.as_str().to_owned(),
                    },
                }),
            ),
            TerminalValue::Unsupported(cause) => (
                ItemLabel::Unsupported,
                Some(ItemCause::Unsupported {
                    cause: match cause {
                        UnavailabilityCause::SolverAbsent => "solver_absent",
                        UnavailabilityCause::BackendAbsent => "backend_absent",
                    },
                }),
            ),
            TerminalValue::Incomplete(cause) => (
                ItemLabel::Incomplete,
                Some(ItemCause::Incomplete {
                    cause: match cause {
                        IncompleteCause::TimedOut => "timed_out",
                        IncompleteCause::Cancelled => "cancelled",
                        IncompleteCause::ResourceExhausted => "resource_exhausted",
                    },
                }),
            ),
            TerminalValue::Inconclusive(cause) => (
                ItemLabel::Inconclusive,
                Some(match cause {
                    InconclusiveCause::ReplayParity(_) => ItemCause::ReplayParity,
                    InconclusiveCause::ReplayRefused(code) => ItemCause::ReplayRefused {
                        code: code.as_str().to_owned(),
                    },
                }),
            ),
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
            cause: Some(ItemCause::UndefinedEvaluation {
                at: at.into(),
                cause: cause.into(),
            }),
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
        locus: Option<&Locus>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            cause,
            code: code.into(),
            locus: locus.map(OutcomeLocus::from),
            message: message.into(),
        }
    }

    /// The diagnostic of a spine compile refusal: its catalog code, cause,
    /// region and message.
    pub fn from_compile_refusal(refusal: &CompileRefusal) -> Self {
        Self::new(
            compile_cause(refusal),
            refusal.code().as_str(),
            refusal
                .region()
                .map(|region| Locus::Region(region.clone()))
                .as_ref(),
            refusal.to_string(),
        )
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

/// The catalog cause tag of a compile refusal, where its stage cause names
/// one.
fn compile_cause(refusal: &CompileRefusal) -> Option<&'static str> {
    match refusal {
        CompileRefusal::Check { refusals, .. } => refusals.first()?.cause.cause(),
        CompileRefusal::Profile { refusals, .. } => refusals.first().map(|r| r.cause()),
        CompileRefusal::DependencyInput(refusal) => refusal.cause(),
        CompileRefusal::Import { refusal, .. } => refusal.cause(),
        CompileRefusal::Dependency { refusal, .. } => compile_cause(refusal),
        CompileRefusal::Link(refusal) => refusal.cause(),
        CompileRefusal::Limit(limit) => Some(limit.kind().catalog_cause()),
        CompileRefusal::Source(_)
        | CompileRefusal::Forms { .. }
        | CompileRefusal::Intake { .. }
        | CompileRefusal::Assembly { .. }
        | CompileRefusal::Emit(_)
        | CompileRefusal::Omitted(_) => None,
    }
}

/// The kind of identity an artifact member holds.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    /// A checked package's `package_id`.
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
    /// A checked package's identity.
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

/// The `quire-outcome/1` document (FR-286).
///
/// `items` is always written, an empty array for an operation with no
/// requested items.
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
}

/// Why a document could not be written.
#[derive(Debug, thiserror::Error)]
#[error("the outcome document could not be encoded: {0}")]
pub struct OutcomeWriteError(#[from] serde_json::Error);

impl OutcomeDocument {
    /// A document of `operation` that last reached `last_stage` (`None`
    /// when no stage is known to have run, as when a handle was cancelled
    /// before the first) with the outcome's `category`, and no items,
    /// diagnostics or artifacts.
    pub fn new(operation: Operation, last_stage: Option<OutcomeStage>, category: Category) -> Self {
        Self {
            format: OUTCOME_FORMAT,
            operation,
            last_stage,
            category,
            items: Vec::new(),
            diagnostics: Vec::new(),
            artifacts: Vec::new(),
        }
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

    /// The document of a `check` outcome: success names the package's
    /// identity at S4; a failure names its category and one diagnostic.
    pub fn from_check(result: &Result<Staged<CheckedUnit>, FrontEndFailure>) -> Self {
        let failure = match result {
            Ok(staged) => return Self::from_checked(staged.value()),
            Err(failure) => failure,
        };
        match failure {
            StageFailure::Refused(refusal) => {
                let category = refusal.code().category();
                Self::new(Operation::Check, Some(refusal.stage().into()), category)
                    .with_diagnostics(vec![OutcomeDiagnostic::from_compile_refusal(refusal)])
            }
            StageFailure::Limit(limit) => {
                let refusal = CompileRefusal::Limit(limit.clone());
                Self::new(
                    Operation::Check,
                    Some(refusal.stage().into()),
                    Category::Incomplete,
                )
                .with_diagnostics(vec![OutcomeDiagnostic::from_compile_refusal(&refusal)])
            }
            StageFailure::Cancelled(cause) => {
                Self::new(Operation::Check, None, Category::Incomplete).with_diagnostics(vec![
                    OutcomeDiagnostic::new(
                        Some(match cause {
                            CancelCause::Requested => "requested",
                            CancelCause::Deadline => "deadline",
                        }),
                        Code::Cancelled.as_str(),
                        None,
                        "the operation was cancelled",
                    ),
                ])
            }
            StageFailure::Fault(fault) => Self::new(Operation::Check, None, fault.category())
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

    /// A checked unit's document. The package identity is the one E4 gives
    /// the package; a package that cannot be emitted has none, and the
    /// document reports the emitter's refusal at S4 instead.
    fn from_checked(unit: &CheckedUnit) -> Self {
        let refusal = match emit_checked(unit.package()) {
            Ok(emission) if emission.omitted().is_empty() => {
                return Self::new(Operation::Check, Some(OutcomeStage::S4), Category::Success)
                    .with_artifacts(vec![OutcomeArtifact::package_id(
                        &emission.package().package_id(),
                    )]);
            }
            Ok(emission) => CompileRefusal::Omitted(emission.omitted().to_vec()),
            Err(refusal) => CompileRefusal::Emit(refusal),
        };
        Self::new(
            Operation::Check,
            Some(refusal.stage().into()),
            refusal.code().category(),
        )
        .with_diagnostics(vec![OutcomeDiagnostic::from_compile_refusal(&refusal)])
    }

    /// The document of an `execute` outcome, last reaching S6a. A refused
    /// call carries its catalog code and cause as one diagnostic. An
    /// undefined call has category `undefined`, the one place that label
    /// appears.
    pub fn from_call(outcome: &CallOutcome) -> Self {
        let document = Self::new(
            Operation::Execute,
            Some(OutcomeStage::S6a),
            outcome.category(),
        );
        let CallOutcome::Refused(refusal) = outcome else {
            return document;
        };
        let (CallRefusal::Record { code, .. } | CallRefusal::Family { code, .. }) = refusal;
        document.with_diagnostics(vec![OutcomeDiagnostic::new(
            Some(code.cause()),
            code.code(),
            None,
            code.to_string(),
        )])
    }

    /// The document's bytes: the same document gives the same bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, OutcomeWriteError> {
        Ok(serde_json::to_vec(self)?)
    }
}

fn category_str<S: Serializer>(category: &Category, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(category.as_str())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ix_trace_rs::trace;
    use qsl_foundation::SourceIdentity;
    use quire_exact::Cancel;
    use serde_json::Value;

    use super::*;
    use crate::spine::{
        check, parse, select, DependencyInput, LockEvidence, ParseRequest, SpineLimits,
    };

    const FIXTURE: &str = include_str!("../../tests/fixtures/spine-compile.native");
    const ILL_TYPED: &str = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"quire.value.complete/v1\";\n\
        type Digit = Int[0, 9];\n\
        function inv using v(x: Digit): Boolean pure { 1 / x > 0 }\n";

    fn checked(text: &str) -> Result<Staged<CheckedUnit>, FrontEndFailure> {
        let source = SourceIdentity::new("agent-ix", "test:outcome", "fixture", "fixture:1");
        let limits = SpineLimits::default();
        let cancel = Cancel::new();
        let request = ParseRequest {
            source: &source,
            path: "unit.native",
            bytes: text.as_bytes(),
        };
        let parsed = parse(&request, limits.source, &cancel)?.into_value();
        let models = select(&parsed, &BTreeMap::new(), limits.model, &cancel)?.into_value();
        check(
            &parsed,
            &models,
            &DependencyInput::default(),
            &LockEvidence::default(),
            limits,
            &cancel,
        )
    }

    fn json(document: &OutcomeDocument) -> Value {
        serde_json::from_slice(&document.to_bytes().expect("encodes")).expect("is JSON")
    }

    /// FR-286-AC-1: the `check` outcome over the spine fixture.
    #[trace("TC-770", "FR-286-AC-1")]
    #[test]
    fn check_success_document_names_its_package_identity() {
        let result = checked(FIXTURE);
        let package_id = emit_checked(
            result
                .as_ref()
                .expect("the fixture checks")
                .value()
                .package(),
        )
        .expect("the package emits")
        .package()
        .package_id()
        .hex();
        let document = json(&OutcomeDocument::from_check(&result));
        assert_eq!(document["format"], "quire-outcome/1");
        assert_eq!(document["operation"], "check");
        assert_eq!(document["last_stage"], "S4");
        assert_eq!(document["category"], "success");
        assert_eq!(document["diagnostics"], serde_json::json!([]));
        assert_eq!(
            document["artifacts"],
            serde_json::json!([{"kind": "package_id", "id": package_id}])
        );
    }

    /// FR-286-AC-2: the `check` outcome of the `inv` source refuses with one
    /// diagnostic equal to the refusal's code, cause, locus and message.
    #[trace("TC-770", "FR-286-AC-2")]
    #[test]
    fn check_refusal_document_carries_the_refusals_diagnostic() {
        let result = checked(ILL_TYPED);
        let Err(StageFailure::Refused(refusal)) = &result else {
            panic!("expected a refusal");
        };
        let region = refusal.region().expect("the refusal is located");
        let document = json(&OutcomeDocument::from_check(&result));
        assert_eq!(document["category"], "refusal");
        assert_eq!(document["last_stage"], "S3");
        let diagnostics = document["diagnostics"].as_array().expect("an array");
        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic["code"], "ill_typed");
        assert_eq!(diagnostic["code"], refusal.code().as_str());
        assert_eq!(
            diagnostic["cause"],
            compile_cause(refusal).expect("a cause")
        );
        assert_eq!(diagnostic["message"], refusal.to_string());
        assert_eq!(diagnostic["locus"]["kind"], "region");
        assert_eq!(diagnostic["locus"]["start"], region.start());
        assert_eq!(diagnostic["locus"]["end"], region.end());
        assert_eq!(
            diagnostic["locus"]["source"],
            serde_json::to_value(region.source()).expect("a source ref")
        );
    }

    /// FR-286-AC-3: three items serialize in request order, each with its
    /// record and category, and a document serializes to equal bytes twice.
    #[trace("TC-770", "FR-286-AC-3")]
    #[test]
    fn analyze_items_keep_request_order_and_bytes_are_stable() {
        let records = [
            TerminalRecord::new(
                RequestIndex::new(0),
                TerminalValue::Unsupported(UnavailabilityCause::SolverAbsent),
            ),
            TerminalRecord::new(
                RequestIndex::new(1),
                TerminalValue::Proved { success_checks: 3 },
            ),
            TerminalRecord::new(RequestIndex::new(2), TerminalValue::Refuted),
        ];
        let document = OutcomeDocument::new(
            Operation::Analyze,
            Some(OutcomeStage::S6c),
            Category::Unsupported,
        )
        .with_items(records.iter().map(OutcomeItem::from_terminal).collect());
        assert_eq!(document.to_bytes().unwrap(), document.to_bytes().unwrap());
        let items = &json(&document)["items"];
        let seen: Vec<_> = items
            .as_array()
            .expect("an array")
            .iter()
            .map(|item| {
                (
                    item["request_index"].as_u64().unwrap(),
                    item["result"].as_str().unwrap().to_owned(),
                    item["category"].as_str().unwrap().to_owned(),
                )
            })
            .collect();
        assert_eq!(
            seen,
            [
                (0, "unsupported".to_owned(), "unsupported".to_owned()),
                (1, "proved".to_owned(), "success".to_owned()),
                (2, "refuted".to_owned(), "violation".to_owned()),
            ]
        );
        assert_eq!(items[0]["cause"]["cause"], "solver_absent");
        let refused = checked(ILL_TYPED);
        assert_eq!(
            OutcomeDocument::from_check(&refused).to_bytes().unwrap(),
            OutcomeDocument::from_check(&refused).to_bytes().unwrap()
        );
    }

    /// FR-286-AC-4: an undefined claim evaluation in an `analyze` or
    /// `monitor` item is a violation with cause `UndefinedEvaluation` and no
    /// `undefined` label; an `execute` outcome that is undefined keeps it.
    #[trace("TC-770", "FR-286-AC-4")]
    #[test]
    fn undefined_label_appears_only_on_a_non_proof_outcome() {
        for operation in [Operation::Analyze, Operation::Monitor] {
            let document =
                OutcomeDocument::new(operation, Some(OutcomeStage::S6c), Category::Violation)
                    .with_items(vec![OutcomeItem::undefined_evaluation(
                        RequestIndex::new(0),
                        "position 1",
                        "division-by-zero",
                    )]);
            let bytes = String::from_utf8(document.to_bytes().unwrap()).unwrap();
            assert!(!bytes.contains("\"undefined\""), "{bytes}");
            let item = &json(&document)["items"][0];
            assert_eq!(item["category"], "violation");
            assert_eq!(item["result"], "refuted");
            assert_eq!(item["cause"]["kind"], "UndefinedEvaluation");
            assert_eq!(item["cause"]["where"], "position 1");
            assert_eq!(item["cause"]["cause"], "division-by-zero");
        }
        let execute = OutcomeDocument::from_call(&CallOutcome::Undefined {
            reason: "sum-out-of-domain",
        });
        let bytes = String::from_utf8(execute.to_bytes().unwrap()).unwrap();
        assert_eq!(json(&execute)["category"], "undefined");
        assert_eq!(json(&execute)["operation"], "execute");
        assert!(bytes.contains("\"undefined\""), "{bytes}");
    }
}
