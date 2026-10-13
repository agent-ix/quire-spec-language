// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-267: canonical clause-run result production from the FR-109 report.

use std::cell::RefCell;
use std::collections::BTreeMap;

use qsl_foundation::diagnostic::Locus;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::digest::{parse_lower_hex32, DigestDomain, DigestRecord};
use qsl_foundation::wire_format::WireFormat;
use qsl_foundation::LocatedSpan;
use quire_canonical::{Document, Encode, Limits, Node, NodeRef, Sink, Writer};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use super::call::{CallOutcome, CallRefusal, CallValue};
use super::{ClauseDisposition, ClauseRunReport};
use crate::result::RecordWire;
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::observation::{DocumentRef, FrameChange, FrameWitness};

/// Why a clause-run report could not be written as a canonical document.
#[derive(Debug, thiserror::Error)]
pub enum NativeRunResultError {
    /// The report's settlement basis and witness disagree.
    #[error("basis and witness presence disagree")]
    BasisWitness,
    /// A verdict has no decisive or closed settlement, or a non-verdict has one.
    #[error("basis and disposition disagree")]
    BasisDisposition,
    /// A separating witness did not have the record its family assigns.
    #[error("invalid separating witness: {0}")]
    Witness(#[from] crate::result::CauseCodecError),
    /// The typed report could not be represented as JSON.
    #[error("cannot serialize clause-run result: {0}")]
    Json(#[from] serde_json::Error),
    /// The canonical reader refused a producer document.
    #[error("cannot read clause-run result for canonical encoding: {0}")]
    Read(#[from] quire_canonical::ReadError),
    /// The canonical encoder could not write the report.
    #[error("cannot encode clause-run result canonically: {0}")]
    Canonical(#[from] quire_canonical::Error),
}

/// A refused `native-run-result/2` read, with the failing wire member named.
#[derive(Debug, thiserror::Error)]
pub enum NativeRunReadError {
    /// The identifier is absent or is not the one supported version.
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),
    /// The document or a closed member is malformed.
    #[error("invalid native run result: {0}")]
    Malformed(#[from] serde_json::Error),
    /// A closed member has a wrong type or shape at this exact path.
    #[error("invalid member {member}: {source}")]
    MalformedMember {
        /// The structured member path.
        member: String,
        /// The original JSON decoder error with its line and column.
        #[source]
        source: serde_json::Error,
    },
    /// The canonical JSON reader refused the document.
    #[error("invalid native run result: {0}")]
    Read(#[from] quire_canonical::ReadError),
    /// A report member contradicts another report member.
    #[error("invalid member {member}: {reason}")]
    Invalid {
        /// The member at fault.
        member: &'static str,
        /// The failed constraint.
        reason: &'static str,
    },
    /// The typed separating record is malformed.
    #[error("invalid member witness: {0}")]
    Witness(#[from] crate::result::CauseCodecError),
}

/// The two closed result variants of the same `/2` reader.
pub enum NativeRunResult {
    /// A clause run reached `run_clause` and returned its report.
    Clause(Box<ClauseRunRecord>),
    /// Request intake or command handling failed before a report existed.
    CommandError(Box<CommandErrorRecord>),
}

/// The typed clause-run document returned by the strict reader.
pub struct ClauseRunRecord(ClauseRead, Option<Vec<u8>>, Option<FrameWitness>);

impl ClauseRunRecord {
    /// The compiled package identity, absent before compile completed.
    pub fn package_id(&self) -> Option<&str> {
        self.0.package_id.as_deref()
    }

    /// The settlement basis label.
    pub fn basis(&self) -> &'static str {
        self.0.basis.as_str()
    }

    /// Whether the document carries a separating witness.
    pub fn has_witness(&self) -> bool {
        self.0.witness.is_some()
    }

    /// The typed separating witness, resolved against the checked package
    /// named by this result. A consumer must supply that trusted package.
    pub fn witness(
        &self,
        package: &qsl_package::CheckedPackage,
    ) -> Result<Option<crate::result::SeparatingWitnessRecord>, crate::result::CauseCodecError>
    {
        match (&self.0.witness, &self.1) {
            (Some(witness), Some(bytes)) => {
                let element = quire_canonical::read(bytes, u64::MAX)?;
                witness.read_state(package, element.root()).map(Some)
            }
            _ => Ok(None),
        }
    }

    /// The complete evaluated frame witness, separate from the top-level
    /// FR-351 separating witness.
    pub fn frame_witness(&self) -> Option<&FrameWitness> {
        self.2.as_ref()
    }

    /// The reported disposition stage.
    pub fn stage(&self) -> &'static str {
        self.0.disposition.stage.as_str()
    }

    /// The reported outcome category.
    pub fn category(&self) -> &'static str {
        self.0.disposition.category.as_str()
    }

    /// The truth of a completed Boolean claim, when one exists.
    pub fn truth(&self) -> Option<bool> {
        self.0.disposition.truth
    }

    /// The catalog code of a non-verdict disposition, when it has one.
    pub fn code(&self) -> Option<&str> {
        self.0
            .disposition
            .refusal
            .as_ref()
            .map(|r| r.code.as_str())
            .or(match self.0.disposition.outcome.as_ref() {
                Some(OutcomeRead::Refused { code, .. }) => Some(code.as_str()),
                _ => None,
            })
    }

    /// The catalog cause of a non-verdict disposition, when it has one.
    pub fn cause(&self) -> Option<&str> {
        self.0
            .disposition
            .refusal
            .as_ref()
            .and_then(|r| r.cause.as_deref())
            .or(match self.0.disposition.outcome.as_ref() {
                Some(OutcomeRead::Refused { cause, .. }) => Some(cause.as_str()),
                _ => None,
            })
    }

    /// Structured refusal fields supplied by the underlying stage.
    pub fn fields(&self) -> Option<&BTreeMap<String, String>> {
        self.0
            .disposition
            .refusal
            .as_ref()
            .and_then(|r| r.fields.as_ref())
            .or(match self.0.disposition.outcome.as_ref() {
                Some(OutcomeRead::Refused { fields, .. }) => fields.as_ref(),
                _ => None,
            })
    }

    /// Whether a refused S6a outcome retains its located source region.
    pub fn has_outcome_locus(&self) -> bool {
        matches!(
            self.0.disposition.outcome.as_ref(),
            Some(OutcomeRead::Refused { locus: Some(_), .. })
        )
    }

    /// The typed source locus of a refused S6a outcome, when assigned.
    pub fn outcome_locus(&self) -> Option<&CallLocusRead> {
        match self.0.disposition.outcome.as_ref() {
            Some(OutcomeRead::Refused { locus, .. }) => locus.as_ref(),
            _ => None,
        }
    }

    /// Whether a refused S6a outcome retains its typed expression location.
    pub fn has_outcome_location(&self) -> bool {
        matches!(
            self.0.disposition.outcome.as_ref(),
            Some(OutcomeRead::Refused {
                location: Some(_),
                ..
            })
        )
    }

    /// The typed expression location of a refused S6a outcome, when assigned.
    pub fn outcome_location(&self) -> Option<&CallLocationRead> {
        match self.0.disposition.outcome.as_ref() {
            Some(OutcomeRead::Refused { location, .. }) => location.as_ref(),
            _ => None,
        }
    }

    /// The source locus of a pre-evaluation or frame refusal, when assigned.
    pub fn refusal_locus(&self) -> Option<&RefusalLocusRead> {
        self.0
            .disposition
            .refusal
            .as_ref()
            .and_then(|r| r.locus.as_ref())
    }

    /// The stable identifiers of an internal failure, when present.
    pub fn internal_fault(&self) -> Option<(&str, &str)> {
        self.0
            .disposition
            .internal_fault
            .as_ref()
            .map(|fault| (fault.stage.as_str(), fault.invariant.as_str()))
    }

    /// The S6a outcome, when evaluation returned one.
    pub fn outcome(&self) -> Option<NativeRunOutcome<'_>> {
        self.0
            .disposition
            .outcome
            .as_ref()
            .map(|outcome| match outcome {
                OutcomeRead::Completed {
                    value: ValueRead::Boolean { value },
                } => NativeRunOutcome::Boolean(*value),
                OutcomeRead::Completed {
                    value: ValueRead::Integer { decimal },
                } => NativeRunOutcome::Integer(decimal),
                OutcomeRead::Refused {
                    code,
                    cause,
                    fields,
                    locus,
                    location,
                } => NativeRunOutcome::Refused {
                    code,
                    cause,
                    fields: fields.as_ref(),
                    locus: locus.as_ref(),
                    location: location.as_ref(),
                },
                OutcomeRead::Undefined { reason } => NativeRunOutcome::Undefined(reason),
                OutcomeRead::Incomplete { limit } => NativeRunOutcome::Incomplete(limit),
            })
    }

    /// Admission bytes, objects and values consumed, in that order.
    pub fn admission_usage(&self) -> (u64, u64, u64) {
        let usage = &self.0.usage.admission;
        (usage.document_bytes, usage.objects, usage.values)
    }

    /// The number of evaluation meter charges admitted.
    pub fn evaluation_admissions(&self) -> u64 {
        self.0.usage.evaluation_admissions
    }

    /// Evaluation counter names and consumed amounts, in catalog order.
    pub fn evaluation_consumed(&self) -> impl Iterator<Item = (&str, u64)> {
        self.0
            .usage
            .evaluation_consumed
            .iter()
            .map(|counter| (counter.kind.as_str(), counter.consumed))
    }
}

/// The typed S6a outcome retained by a clause-run document.
pub enum NativeRunOutcome<'a> {
    /// A Boolean claim completed with this truth.
    Boolean(bool),
    /// An integer value completed with this canonical decimal spelling.
    Integer(&'a str),
    /// Evaluation refused at its catalogued cause, retaining each assigned
    /// FR-100 member as a typed borrowed value.
    Refused {
        /// The catalog code.
        code: &'a str,
        /// The catalog cause.
        cause: &'a str,
        /// Cause-specific fields, when assigned.
        fields: Option<&'a BTreeMap<String, String>>,
        /// The source locus, when assigned.
        locus: Option<&'a CallLocusRead>,
        /// The expression location, when assigned.
        location: Option<&'a CallLocationRead>,
    },
    /// The claim was undefined for this reason.
    Undefined(&'a str),
    /// Evaluation exhausted this counter.
    Incomplete(&'a str),
}

/// The typed command-error document returned by the strict reader.
pub struct CommandErrorRecord(CommandErrorRead, Document);

impl CommandErrorRecord {
    /// The stage that failed.
    pub fn stage(&self) -> &str {
        &self.0.stage
    }

    /// The catalog code of the command failure.
    pub fn code(&self) -> &str {
        &self.0.code
    }

    /// The catalog cause, where the code has one.
    pub fn cause(&self) -> Option<&str> {
        self.0.cause.as_deref()
    }

    /// The human-readable diagnostic.
    pub fn message(&self) -> &str {
        &self.0.message
    }

    /// The code-specific detail object in the original canonical arena.
    /// A caller can inspect arbitrarily deep details without a JSON parser's
    /// recursive descent limit.
    pub fn details(&self) -> NodeRef<'_> {
        self.1
            .root()
            .get("details")
            .expect("validated command-error document has details")
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Basis {
    ClosedScope,
    DecisiveWitness,
    DecisiveCounterexample,
    Unsettled,
    Unavailable,
}

impl Basis {
    fn as_str(&self) -> &'static str {
        match self {
            Self::ClosedScope => "closed-scope",
            Self::DecisiveWitness => "decisive-witness",
            Self::DecisiveCounterexample => "decisive-counterexample",
            Self::Unsettled => "unsettled",
            Self::Unavailable => "unavailable",
        }
    }

    fn is_decisive(&self) -> bool {
        matches!(self, Self::DecisiveWitness | Self::DecisiveCounterexample)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Stage {
    Compile,
    Select,
    Admit,
    Evaluate,
}

impl Stage {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Compile => "compile",
            Self::Select => "select",
            Self::Admit => "admit",
            Self::Evaluate => "evaluate",
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Category {
    Success,
    Violation,
    Undefined,
    Refusal,
    Unsupported,
    Incomplete,
    Inconclusive,
    InternalFailure,
}

impl Category {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Violation => "violation",
            Self::Undefined => "undefined",
            Self::Refusal => "refusal",
            Self::Unsupported => "unsupported",
            Self::Incomplete => "incomplete",
            Self::Inconclusive => "inconclusive",
            Self::InternalFailure => "internal-failure",
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CounterRead {
    kind: String,
    consumed: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AdmissionRead {
    document_bytes: u64,
    objects: u64,
    values: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UsageRead {
    admission: AdmissionRead,
    evaluation_admissions: u64,
    evaluation_consumed: Vec<CounterRead>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum OutcomeRead {
    Completed {
        value: ValueRead,
    },
    Refused {
        code: String,
        cause: String,
        #[serde(default, deserialize_with = "present")]
        fields: Option<BTreeMap<String, String>>,
        #[serde(default, deserialize_with = "present")]
        locus: Option<CallLocusRead>,
        #[serde(default, deserialize_with = "present")]
        location: Option<CallLocationRead>,
    },
    Undefined {
        reason: String,
    },
    Incomplete {
        limit: String,
    },
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ValueRead {
    Boolean { value: bool },
    Integer { decimal: String },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(
    dead_code,
    reason = "the closed read shape validates every field during deserialization"
)]
/// Decoded source locus attached to an evaluation refusal.
pub struct CallLocusRead {
    /// Authored path of the source that owns this span.
    pub file: String,
    /// The located source span.
    pub span: SpanRead,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(
    dead_code,
    reason = "the closed read shape validates every field during deserialization"
)]
/// Decoded source span.
pub struct SpanRead {
    /// First position in the source region.
    pub start: PositionRead,
    /// Position after the source region.
    pub end: PositionRead,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(
    dead_code,
    reason = "the closed read shape validates every field during deserialization"
)]
/// Decoded position in an authored source.
pub struct PositionRead {
    /// Zero-based byte offset.
    pub byte: usize,
    /// One-based line number.
    pub line: usize,
    /// One-based column number.
    pub column: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(
    dead_code,
    reason = "the closed read shape validates every field during deserialization"
)]
/// Decoded expression location within a declaration.
pub struct CallLocationRead {
    /// Declaration origin of the expression.
    pub origin: CallOriginRead,
    /// Child-index path within the origin.
    pub path: Vec<usize>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
#[allow(
    dead_code,
    reason = "the closed read shape validates every field during deserialization"
)]
/// Declaration origin of a decoded expression location.
pub enum CallOriginRead {
    /// Function body clause.
    Body {
        /// Function name.
        function: String,
        /// Body clause index.
        index: usize,
    },
    /// Function measure clause.
    Measure {
        /// Function name.
        function: String,
        /// Measure clause index.
        index: usize,
    },
    /// Standalone expression.
    Expression,
    /// Named type declaration.
    TypeDeclaration {
        /// Type name.
        name: String,
    },
    /// State clause declaration.
    StateClause {
        /// State clause name.
        clause: String,
        /// Clause index.
        index: usize,
    },
    /// Protocol attempt declaration.
    ProtocolAttempt {
        /// Protocol index.
        protocol: usize,
        /// Attempt index.
        attempt: usize,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InternalFaultRead {
    stage: String,
    invariant: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(
    dead_code,
    reason = "the closed read shape validates every field during deserialization"
)]
struct RefusalRead {
    code: String,
    #[serde(default, deserialize_with = "present")]
    cause: Option<String>,
    #[serde(default, deserialize_with = "present")]
    fields: Option<BTreeMap<String, String>>,
    #[serde(default, deserialize_with = "present")]
    locus: Option<RefusalLocusRead>,
    #[serde(default, deserialize_with = "present")]
    frame_witness: Option<FrameWitnessRead>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FrameWitnessRead {
    code: String,
    cause: String,
    invocation: DocumentRefRead,
    pre: DocumentRefRead,
    post: DocumentRefRead,
    population: String,
    change: FrameChangeRead,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentRefRead {
    authority: String,
    identity: String,
    revision_namespace: String,
    revision: String,
    digest: String,
}

impl DocumentRefRead {
    #[qsl_attrs::string_edge]
    fn typed(&self, member: &'static str) -> Result<DocumentRef, NativeRunReadError> {
        let digest = self
            .digest
            .strip_prefix("sha256-jcs:")
            .and_then(parse_lower_hex32)
            .ok_or(NativeRunReadError::Invalid {
                member,
                reason: "expected sha256-jcs digest",
            })?;
        Ok(DocumentRef {
            authority: self.authority.clone(),
            identity: self.identity.clone(),
            revision_namespace: self.revision_namespace.clone(),
            revision: self.revision.clone(),
            digest,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeclarationKeyRead {
    package: String,
    node: String,
}

impl DeclarationKeyRead {
    fn typed(&self) -> DeclarationKey {
        DeclarationKey {
            package: self.package.clone(),
            node: self.node.clone(),
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum FrameChangeRead {
    FieldWrite {
        object: String,
        field: String,
        modifies: Vec<DeclarationKeyRead>,
    },
    Created {
        object: String,
        type_name: DeclarationKeyRead,
        creates: Vec<DeclarationKeyRead>,
    },
    Deleted {
        object: String,
        type_name: DeclarationKeyRead,
        deletes: Vec<DeclarationKeyRead>,
    },
    Retyped {
        object: String,
        pre_type: DeclarationKeyRead,
        post_type: DeclarationKeyRead,
    },
}

impl FrameChangeRead {
    fn typed(&self) -> FrameChange {
        match self {
            Self::FieldWrite {
                object,
                field,
                modifies,
            } => FrameChange::FieldWrite {
                object: object.clone(),
                field: field.clone(),
                modifies: modifies.iter().map(DeclarationKeyRead::typed).collect(),
            },
            Self::Created {
                object,
                type_name,
                creates,
            } => FrameChange::Created {
                object: object.clone(),
                type_name: type_name.typed(),
                creates: creates.iter().map(DeclarationKeyRead::typed).collect(),
            },
            Self::Deleted {
                object,
                type_name,
                deletes,
            } => FrameChange::Deleted {
                object: object.clone(),
                type_name: type_name.typed(),
                deletes: deletes.iter().map(DeclarationKeyRead::typed).collect(),
            },
            Self::Retyped {
                object,
                pre_type,
                post_type,
            } => FrameChange::Retyped {
                object: object.clone(),
                pre_type: pre_type.typed(),
                post_type: post_type.typed(),
            },
        }
    }
}

impl FrameWitnessRead {
    #[qsl_attrs::string_edge]
    fn typed(&self) -> Result<FrameWitness, NativeRunReadError> {
        if self.code != "frame_violation" || self.cause != "unauthorized-change" {
            return Err(NativeRunReadError::Invalid {
                member: "disposition.refusal.frame_witness",
                reason: "frame witness code or cause disagrees with catalog",
            });
        }
        Ok(FrameWitness {
            code: Code::FrameViolation,
            cause: "unauthorized-change",
            invocation: self
                .invocation
                .typed("disposition.refusal.frame_witness.invocation.digest")?,
            pre: self
                .pre
                .typed("disposition.refusal.frame_witness.pre.digest")?,
            post: self
                .post
                .typed("disposition.refusal.frame_witness.post.digest")?,
            population: self.population.clone(),
            change: self.change.typed(),
        })
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[allow(
    dead_code,
    reason = "the closed read shape validates every field during deserialization"
)]
/// Decoded locus attached to a compile or selection refusal.
pub enum RefusalLocusRead {
    /// Region within an authored source.
    Region {
        /// Source identity.
        source: SourceRefRead,
        /// First byte of the region.
        start: u64,
        /// Byte after the region.
        end: u64,
    },
    /// Source digest and line-column span.
    Located {
        /// Source digest.
        source_digest: String,
        /// Source span.
        span: SpanRead,
    },
    /// Named occurrence.
    Occurrence {
        /// Occurrence identifier.
        occurrence: String,
    },
    /// Pointer into an identified artifact.
    Artifact {
        /// Artifact domain.
        domain: String,
        /// Artifact digest.
        digest: String,
        /// Pointer within the artifact.
        pointer: String,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(
    dead_code,
    reason = "the closed read shape validates every field during deserialization"
)]
/// Decoded identity of an authored source.
pub struct SourceRefRead {
    /// Source authority.
    pub authority: String,
    /// Source identity.
    pub identity: String,
    /// Domain of the source digest.
    pub digest_domain: String,
    /// Lowercase source digest.
    pub digest: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DispositionRead {
    stage: Stage,
    category: Category,
    #[serde(default, deserialize_with = "present")]
    truth: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    outcome: Option<OutcomeRead>,
    #[serde(default, deserialize_with = "present")]
    internal_fault: Option<InternalFaultRead>,
    #[serde(default, deserialize_with = "present")]
    refusal: Option<RefusalRead>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ClauseRead {
    format: String,
    #[serde(default, deserialize_with = "present")]
    package_id: Option<String>,
    disposition: DispositionRead,
    usage: UsageRead,
    basis: Basis,
    #[serde(default, deserialize_with = "present")]
    witness: Option<RecordWire<Box<RawValue>>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CommandErrorRead {
    format: String,
    stage: String,
    code: String,
    #[serde(default, deserialize_with = "present")]
    cause: Option<String>,
    message: String,
    #[serde(rename = "details")]
    _details: Box<RawValue>,
    basis: Basis,
}

fn present<'de, D, T>(decoder: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(decoder).map(Some)
}

fn read_typed<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, NativeRunReadError> {
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    serde_path_to_error::deserialize(&mut decoder).map_err(|error| {
        NativeRunReadError::MalformedMember {
            member: error.path().to_string(),
            source: error.into_inner(),
        }
    })
}

/// FR-096's kernel rows always carry a located record and these catalog
/// fields. Family causes without a row have no fields; their optional
/// location cannot be inferred from their code alone.
#[qsl_attrs::string_edge]
fn kernel_fields(code: &str) -> Option<&'static [&'static str]> {
    match code {
        "cardinality_out_of_bound" => Some(&["collection", "bound", "count"]),
        "foreign_reference" => Some(&["required", "supplied"]),
        "inexact_decimal"
        | "decimal_out_of_domain"
        | "division_out_of_domain"
        | "modulo_out_of_domain"
        | "text_length_out_of_domain"
        | "integer_out_of_domain"
        | "rational_out_of_domain"
        | "ieee_rational_out_of_domain" => Some(&["expected"]),
        "ieee_not_exact" => Some(&["expected", "flags"]),
        "ieee_nan_payload_not_representable" => Some(&["expected", "actual"]),
        _ => None,
    }
}

#[qsl_attrs::string_edge]
fn kernel_causes(code: &str) -> Option<&'static [&'static str]> {
    match code {
        "cardinality_out_of_bound" => Some(&["above-maximum", "below-minimum"]),
        "foreign_reference" => Some(&["foreign-universe"]),
        "inexact_decimal" => Some(&["nonzero-discarded-digit"]),
        "division_out_of_domain" => Some(&["quotient-outside-domain", "remainder-outside-domain"]),
        "ieee_not_exact" => Some(&["rounding-required"]),
        "ieee_nan_payload_not_representable" => Some(&["payload-exceeds-target"]),
        "decimal_out_of_domain"
        | "modulo_out_of_domain"
        | "text_length_out_of_domain"
        | "integer_out_of_domain"
        | "rational_out_of_domain"
        | "ieee_rational_out_of_domain" => Some(&["outside-domain"]),
        _ => None,
    }
}

fn validate_refused_outcome(outcome: &OutcomeRead) -> Result<(), NativeRunReadError> {
    let OutcomeRead::Refused {
        code,
        cause,
        fields,
        locus,
        location,
        ..
    } = outcome
    else {
        return Ok(());
    };
    let Some(required) = kernel_fields(code) else {
        return Ok(());
    };
    if !kernel_causes(code).is_some_and(|causes| causes.contains(&cause.as_str())) {
        return Err(NativeRunReadError::Invalid {
            member: "disposition.outcome.cause",
            reason: "kernel refusal cause disagrees with catalog code",
        });
    }
    let Some(fields) = fields else {
        return Err(NativeRunReadError::Invalid {
            member: "disposition.outcome.fields",
            reason: "kernel refusal needs its catalog fields",
        });
    };
    if fields.len() != required.len() || required.iter().any(|key| !fields.contains_key(*key)) {
        return Err(NativeRunReadError::Invalid {
            member: "disposition.outcome.fields",
            reason: "kernel refusal catalog fields disagree",
        });
    }
    if locus.is_none() {
        return Err(NativeRunReadError::Invalid {
            member: "disposition.outcome.locus",
            reason: "kernel refusal needs its source locus",
        });
    }
    if location.is_none() {
        return Err(NativeRunReadError::Invalid {
            member: "disposition.outcome.location",
            reason: "kernel refusal needs its expression location",
        });
    }
    Ok(())
}

#[qsl_attrs::string_edge]
fn validate_compile_limit(refusal: &RefusalRead) -> Result<(), NativeRunReadError> {
    if refusal.code != "stage_limit_exceeded" {
        return Ok(());
    }
    let invalid = || NativeRunReadError::Invalid {
        member: "disposition.refusal.fields",
        reason: "compile limit needs its exact kind, bound, actual and setting",
    };
    let fields = refusal.fields.as_ref().ok_or_else(invalid)?;
    if fields.len() != 4 {
        return Err(invalid());
    }
    let setting = fields
        .get("setting")
        .and_then(|name| {
            qsl_foundation::Setting::ALL
                .iter()
                .copied()
                .find(|setting| setting.name() == name)
        })
        .ok_or_else(invalid)?;
    let bound = fields
        .get("bound")
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(invalid)?;
    let actual = fields
        .get("actual")
        .and_then(|value| value.parse::<u128>().ok())
        .ok_or_else(invalid)?;
    if fields.get("kind").map(String::as_str) != Some(setting.kind().catalog_cause())
        || refusal.cause.as_deref() != Some(setting.kind().catalog_cause())
        || fields.get("bound") != Some(&bound.to_string())
        || fields.get("actual") != Some(&actual.to_string())
    {
        return Err(invalid());
    }
    Ok(())
}

fn catalog_category(code: &str) -> Option<&'static str> {
    qsl_foundation::diagnostic::catalog_category(code)?;
    Some(
        Code::from_code(code)
            .map_or(
                qsl_foundation::diagnostic::Category::Refusal,
                Code::category,
            )
            .as_str(),
    )
}

#[qsl_attrs::string_edge]
fn validate_disposition(value: &DispositionRead) -> Result<(), NativeRunReadError> {
    let invalid = |member, reason| NativeRunReadError::Invalid { member, reason };
    match (&value.stage, &value.category) {
        (Stage::Compile | Stage::Select | Stage::Admit, Category::InternalFailure) => {
            if value.internal_fault.is_none()
                || value.refusal.is_some()
                || value.outcome.is_some()
                || value.truth.is_some()
            {
                return Err(invalid(
                    "disposition.internal_fault",
                    "internal failure needs only its typed fault",
                ));
            }
        }
        (Stage::Compile | Stage::Select | Stage::Admit, _) => {
            if value.refusal.is_none()
                || value.internal_fault.is_some()
                || value.outcome.is_some()
                || value.truth.is_some()
            {
                return Err(invalid(
                    "disposition.refusal",
                    "pre-evaluation result needs one refusal record",
                ));
            }
            if matches!(value.stage, Stage::Select) {
                let refusal = value.refusal.as_ref().expect("checked above");
                if refusal.cause.is_none() {
                    return Err(invalid(
                        "disposition.refusal.cause",
                        "selection refusal needs its cause",
                    ));
                }
                if refusal.fields.is_none() {
                    return Err(invalid(
                        "disposition.refusal.fields",
                        "selection refusal needs its fields",
                    ));
                }
                let valid = matches!(
                    (refusal.code.as_str(), refusal.cause.as_deref()),
                    ("missing_declaration", Some("missing-name"))
                        | ("ill_typed", Some("type-mismatch"))
                ) && refusal
                    .fields
                    .as_ref()
                    .is_some_and(|fields| fields.len() == 1 && fields.contains_key("name"));
                if !valid {
                    return Err(invalid(
                        "disposition.refusal.fields",
                        "selection refusal needs its catalog name field",
                    ));
                }
            }
            if matches!(value.stage, Stage::Compile) {
                validate_compile_limit(value.refusal.as_ref().expect("checked above"))?;
            }
            let refusal = value.refusal.as_ref().expect("checked above");
            if catalog_category(&refusal.code) != Some(value.category.as_str()) {
                return Err(invalid(
                    "disposition.category",
                    "category disagrees with refusal catalog code",
                ));
            }
            if value
                .refusal
                .as_ref()
                .is_some_and(|refusal| refusal.frame_witness.is_some())
            {
                return Err(invalid(
                    "disposition.refusal.frame_witness",
                    "only an evaluated Frame violation has a frame witness",
                ));
            }
        }
        (Stage::Evaluate, Category::InternalFailure) => {
            if value.internal_fault.is_none()
                || value.refusal.is_some()
                || value.outcome.is_some()
                || value.truth.is_some()
            {
                return Err(invalid(
                    "disposition.internal_fault",
                    "evaluate internal failure needs only its typed fault",
                ));
            }
        }
        (Stage::Evaluate, Category::Success | Category::Violation) => {
            if value.truth.is_none() {
                return Err(invalid(
                    "disposition.truth",
                    "verdict needs its Boolean truth",
                ));
            }
            let completed = match &value.outcome {
                Some(OutcomeRead::Completed {
                    value: ValueRead::Boolean { value },
                }) => Some(*value),
                _ => None,
            };
            let is_frame = matches!(value.category, Category::Violation)
                && value.refusal.is_some()
                && value.outcome.is_none();
            if is_frame {
                let refusal = value.refusal.as_ref().expect("checked above");
                if refusal.code != "frame_violation"
                    || refusal.cause.as_deref() != Some("unauthorized-change")
                    || refusal.frame_witness.is_none()
                    || refusal.fields.is_some()
                    || refusal.locus.is_some()
                {
                    return Err(invalid(
                        "disposition.refusal.frame_witness",
                        "Frame violation needs its complete catalog witness",
                    ));
                }
                let witness = refusal.frame_witness.as_ref().expect("checked above");
                if witness.code != refusal.code
                    || witness.cause != refusal.cause.as_deref().unwrap()
                {
                    return Err(invalid(
                        "disposition.refusal.frame_witness",
                        "frame witness code and cause disagree with refusal",
                    ));
                }
                witness.typed()?;
            }
            if value.internal_fault.is_some()
                || (!is_frame && value.refusal.is_some())
                || (!is_frame && (completed.is_none() || completed != value.truth))
                || (matches!(value.category, Category::Success) && completed != Some(true))
                || (matches!(value.category, Category::Violation)
                    && !is_frame
                    && completed != Some(false))
                || (is_frame && value.truth != Some(false))
            {
                return Err(invalid(
                    "disposition.outcome",
                    "verdict needs its Boolean completion or frame record",
                ));
            }
        }
        (Stage::Evaluate, Category::Undefined) => {
            if !matches!(value.outcome.as_ref(), Some(OutcomeRead::Undefined { .. }))
                || value.internal_fault.is_some()
                || value.refusal.is_some()
                || value.truth.is_some()
            {
                return Err(invalid(
                    "disposition.outcome",
                    "undefined needs its FR-100 reason",
                ));
            }
        }
        (Stage::Evaluate, Category::Refusal | Category::Unsupported) => {
            if !matches!(value.outcome.as_ref(), Some(OutcomeRead::Refused { .. }))
                || value.internal_fault.is_some()
                || value.refusal.is_some()
                || value.truth.is_some()
            {
                return Err(invalid(
                    "disposition.outcome",
                    "refusal needs its FR-100 record",
                ));
            }
            validate_refused_outcome(value.outcome.as_ref().expect("checked above"))?;
            if let Some(OutcomeRead::Refused { code, .. }) = value.outcome.as_ref() {
                if catalog_category(code) != Some(value.category.as_str()) {
                    return Err(invalid(
                        "disposition.category",
                        "category disagrees with outcome catalog code",
                    ));
                }
            }
        }
        (Stage::Evaluate, Category::Incomplete) => {
            if !matches!(
                value.outcome.as_ref(),
                Some(OutcomeRead::Incomplete { .. } | OutcomeRead::Refused { .. })
            ) || value.internal_fault.is_some()
                || value.refusal.is_some()
                || value.truth.is_some()
            {
                return Err(invalid(
                    "disposition.outcome",
                    "incomplete needs its FR-100 outcome",
                ));
            }
            if let Some(outcome) = value.outcome.as_ref() {
                validate_refused_outcome(outcome)?;
                if let OutcomeRead::Refused { code, .. } = outcome {
                    if catalog_category(code) != Some(value.category.as_str()) {
                        return Err(invalid(
                            "disposition.category",
                            "category disagrees with outcome catalog code",
                        ));
                    }
                }
            }
        }
        (Stage::Evaluate, Category::Inconclusive) => {
            return Err(invalid(
                "disposition.category",
                "clause run has no inconclusive category",
            ))
        }
    }
    Ok(())
}

/// FR-100's command refusal table fixes these rows' cause and detail shapes.
#[qsl_attrs::string_edge]
fn validate_command_details(
    document: &CommandErrorRead,
    details: NodeRef<'_>,
    code: Code,
) -> Result<(), NativeRunReadError> {
    let invalid = |member, reason| NativeRunReadError::Invalid { member, reason };
    let Node::Object(members) = details.node() else {
        return Err(invalid("details", "expected an object"));
    };
    let len = members.len();
    let string = |name| match details.get(name).map(|node| node.node()) {
        Some(Node::String(value)) => Some(value),
        _ => None,
    };
    let unsigned = |name| match details.get(name).map(|node| node.node()) {
        Some(Node::Number(value)) => value.text().parse::<u64>().ok(),
        _ => None,
    };
    match code {
        Code::UnknownEdition => {
            if document.stage != "profile" || document.cause.is_some() {
                return Err(invalid("cause", "unknown_edition has no cause at profile"));
            }
            if string("phase") != Some("profile")
                || string("code") != Some("unknown_edition")
                || string("path").is_none()
            {
                return Err(invalid(
                    "details",
                    "unknown_edition needs its profile diagnostic",
                ));
            }
        }
        Code::MissingDeclaration | Code::UnsupportedConstruct if document.stage == "call" => {
            if document.cause.is_some() {
                return Err(invalid("cause", "call selection refusal has no cause"));
            }
            if len != 1 || string("function").is_none() {
                return Err(invalid("details", "call selection needs function"));
            }
        }
        Code::InvalidRuntimeInput if document.stage == "call" => {
            if document.cause.is_some() {
                return Err(invalid("cause", "call argument refusal has no cause"));
            }
            let parameter = string("parameter");
            let position = unsigned("position");
            if len != 1 || (parameter.is_some() == position.is_some()) {
                return Err(invalid(
                    "details",
                    "call argument needs parameter or position",
                ));
            }
        }
        Code::ForeignReference if document.stage == "call" => {
            let Some(cause) = document.cause.as_deref() else {
                return Err(invalid("cause", "foreign_reference needs its cause"));
            };
            let position = unsigned("position");
            let valid_string = |name| string(name).is_some();
            let valid_key = |name| {
                let Some(node) = details.get(name) else {
                    return false;
                };
                let Node::Object(members) = node.node() else {
                    return false;
                };
                members.len() == 2
                    && matches!(
                        node.get("package").map(|value| value.node()),
                        Some(Node::String(_))
                    )
                    && matches!(
                        node.get("node").map(|value| value.node()),
                        Some(Node::String(_))
                    )
            };
            let valid_selection = || {
                let Some(node) = details.get("expected") else {
                    return false;
                };
                let Node::Object(members) = node.node() else {
                    return false;
                };
                members.len() == 4
                    && matches!(node.get("identity").map(|value| value.node()), Some(Node::String(_)))
                    && matches!(node.get("version").map(|value| value.node()), Some(Node::String(_)))
                    && node.get("digest_domain").is_some_and(|value| {
                        matches!(value.node(), Node::String(domain) if domain == "sha256-jcs")
                    })
                    && node
                        .get("digest")
                        .is_some_and(|value| {
                            matches!(value.node(), Node::String(digest) if digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()))
                        })
            };
            match cause {
                "foreign-model-selection" => {
                    let document_offer = valid_string("model_identity");
                    let population_offer = valid_key("population");
                    if len != 3
                        || position.is_none()
                        || document_offer == population_offer
                        || !valid_selection()
                    {
                        return Err(invalid(
                            "details",
                            "foreign-model-selection needs one offered selection and expected model",
                        ));
                    }
                }
                "foreign-type" => {
                    if len != 4
                        || position.is_none()
                        || !valid_string("member")
                        || !valid_key("type")
                    {
                        return Err(invalid(
                            "details",
                            "foreign-type needs position, member and type",
                        ));
                    }
                }
                "foreign-universe" => {
                    if len != 3
                        || position.is_none()
                        || !valid_string("required")
                        || !valid_string("supplied")
                    {
                        return Err(invalid(
                            "details",
                            "foreign-universe needs position, required and supplied",
                        ));
                    }
                }
                _ => return Err(invalid("cause", "unknown foreign_reference cause")),
            }
        }
        Code::RuntimeInvariant if document.stage == "call" => {
            if document.cause.as_deref() != Some("established-invariant-broken") {
                return Err(invalid(
                    "cause",
                    "runtime_invariant needs its catalog cause",
                ));
            }
            if len != 2 || string("stage").is_none() || string("invariant").is_none() {
                return Err(invalid(
                    "details",
                    "runtime_invariant needs stage and invariant",
                ));
            }
        }
        Code::InvalidRequest if document.stage == "request" && document.cause.is_some() => {
            return Err(invalid("cause", "invalid-request has no cause"));
        }
        _ => {}
    }
    Ok(())
}

/// Keep the potentially deep deciding element in the canonical arena while
/// serde reads the fixed-depth `/2` envelope. The original node is validated
/// and retained separately; the placeholder is never exposed to consumers.
struct WitnessSplice<'d> {
    document: &'d Document,
    element: RefCell<Option<NodeRef<'d>>>,
}

/// Keep command details in the canonical arena while serde checks the fixed
/// envelope. The retained arena, rather than the placeholder, is public.
struct DetailSplice<'d>(&'d Document);

impl Encode for DetailSplice<'_> {
    #[qsl_attrs::string_edge]
    fn encode_into<S: Sink + ?Sized>(
        &self,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        let root = self.0.root();
        let Node::Object(members) = root.node() else {
            return root.encode_into(writer);
        };
        writer.begin_object()?;
        for (name, member) in members {
            writer.name(name)?;
            if name == "details" {
                writer.begin_object()?;
                writer.end_object()?;
            } else {
                member.encode_into(writer)?;
            }
        }
        writer.end_object()
    }
}

impl Encode for WitnessSplice<'_> {
    #[qsl_attrs::string_edge]
    fn encode_into<S: Sink + ?Sized>(
        &self,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        let root = self.document.root();
        let Node::Object(members) = root.node() else {
            return root.encode_into(writer);
        };
        writer.begin_object()?;
        for (name, member) in members {
            writer.name(name)?;
            if name == "witness" {
                if let Node::Object(witness) = member.node() {
                    writer.begin_object()?;
                    for (name, value) in witness {
                        writer.name(name)?;
                        if name == "deciding_element" {
                            self.element.replace(Some(value));
                            writer.begin_object()?;
                            writer.name("kind")?;
                            writer.string("boolean")?;
                            writer.name("value")?;
                            writer.bool(true)?;
                            writer.end_object()?;
                        } else {
                            value.encode_into(writer)?;
                        }
                    }
                    writer.end_object()?;
                } else {
                    member.encode_into(writer)?;
                }
            } else {
                member.encode_into(writer)?;
            }
        }
        writer.end_object()
    }
}

/// Read the exact `/2` identifier before choosing the closed clause-run or
/// command-error shape. No other format is inferred from its payload.
#[qsl_attrs::string_edge]
pub fn read_native_run_result(bytes: &[u8]) -> Result<NativeRunResult, NativeRunReadError> {
    let tree = quire_canonical::read(bytes, u64::MAX)?;
    let root = tree.root();
    let format = root
        .get("format")
        .and_then(|node| match node.node() {
            Node::String(value) => Some(value),
            _ => None,
        })
        .unwrap_or("<missing>");
    if format != WireFormat::RunResult.as_str() {
        return Err(NativeRunReadError::UnsupportedFormat(format.to_owned()));
    }
    let command_shape = root.get("stage").is_some() || root.get("details").is_some();
    if !command_shape
        && (root.get("disposition").is_some()
            || root.get("usage").is_some()
            || root.get("package_id").is_some()
            || root.get("witness").is_some())
    {
        if root.get("disposition").is_none() {
            return Err(NativeRunReadError::Invalid {
                member: "disposition",
                reason: "clause-run document needs its disposition",
            });
        }
        let splice = WitnessSplice {
            document: &tree,
            element: RefCell::new(None),
        };
        let shallow = quire_canonical::to_vec(&splice, Limits::new(u64::MAX)).map_err(|_| {
            NativeRunReadError::Invalid {
                member: "witness",
                reason: "cannot traverse witness document",
            }
        })?;
        let document: ClauseRead = read_typed(&shallow)?;
        let deciding_element = splice
            .element
            .into_inner()
            .map(|node| {
                crate::result::validate_state_element(node).map_err(NativeRunReadError::Witness)?;
                quire_canonical::to_vec(&node, Limits::new(u64::MAX)).map_err(|_| {
                    NativeRunReadError::Invalid {
                        member: "witness.deciding_element",
                        reason: "cannot retain canonical deciding element",
                    }
                })
            })
            .transpose()?;
        if document.format != format {
            return Err(NativeRunReadError::UnsupportedFormat(document.format));
        }
        if document.package_id.as_ref().is_some_and(|id| {
            id.len() != 64
                || !id
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        }) {
            return Err(NativeRunReadError::Invalid {
                member: "package_id",
                reason: "expected 64 lowercase hex digits",
            });
        }
        if !matches!(document.disposition.stage, Stage::Compile) && document.package_id.is_none() {
            return Err(NativeRunReadError::Invalid {
                member: "package_id",
                reason: "completed compilation needs its package identity",
            });
        }
        if document.basis.is_decisive() != document.witness.is_some() {
            return Err(NativeRunReadError::Invalid {
                member: "witness",
                reason: "presence disagrees with basis",
            });
        }
        if let Some(witness) = &document.witness {
            witness.validate_state()?;
        }
        validate_disposition(&document.disposition)?;
        let frame_witness = document
            .disposition
            .refusal
            .as_ref()
            .and_then(|refusal| refusal.frame_witness.as_ref())
            .map(FrameWitnessRead::typed)
            .transpose()?;
        if frame_witness.is_some()
            && (!matches!(document.basis, Basis::ClosedScope) || document.witness.is_some())
        {
            return Err(NativeRunReadError::Invalid {
                member: "basis",
                reason: "frame violation requires closed scope without a separating witness",
            });
        }
        if matches!(
            document.disposition.category,
            Category::Success | Category::Violation
        ) != document.disposition.truth.is_some()
        {
            return Err(NativeRunReadError::Invalid {
                member: "truth",
                reason: "presence disagrees with category",
            });
        }
        let verdict = matches!(
            document.disposition.category,
            Category::Success | Category::Violation
        );
        if (verdict
            && !matches!(
                document.basis,
                Basis::ClosedScope | Basis::DecisiveWitness | Basis::DecisiveCounterexample
            ))
            || (!verdict && !matches!(document.basis, Basis::Unavailable))
        {
            return Err(NativeRunReadError::Invalid {
                member: "basis",
                reason: "basis disagrees with disposition",
            });
        }
        Ok(NativeRunResult::Clause(Box::new(ClauseRunRecord(
            document,
            deciding_element,
            frame_witness,
        ))))
    } else {
        let shallow = quire_canonical::to_vec(&DetailSplice(&tree), Limits::new(u64::MAX))
            .map_err(|_| NativeRunReadError::Invalid {
                member: "details",
                reason: "cannot traverse command-error document",
            })?;
        let document: CommandErrorRead = read_typed(&shallow)?;
        if document.format != format {
            return Err(NativeRunReadError::UnsupportedFormat(document.format));
        }
        if !matches!(document.basis, Basis::Unavailable) {
            return Err(NativeRunReadError::Invalid {
                member: "basis",
                reason: "command errors use unavailable",
            });
        }
        let details = root.get("details").ok_or(NativeRunReadError::Invalid {
            member: "details",
            reason: "expected an object",
        })?;
        let Some(code) = Code::from_code(&document.code) else {
            return Err(NativeRunReadError::Invalid {
                member: "code",
                reason: "unknown catalog code",
            });
        };
        validate_command_details(&document, details, code)?;
        Ok(NativeRunResult::CommandError(Box::new(CommandErrorRecord(
            document, tree,
        ))))
    }
}

#[derive(Serialize)]
struct CounterUsage {
    kind: &'static str,
    consumed: u64,
}

#[derive(Serialize)]
struct Usage {
    admission: AdmissionUsage,
    evaluation_admissions: u64,
    evaluation_consumed: Vec<CounterUsage>,
}

#[derive(Serialize)]
struct AdmissionUsage {
    document_bytes: u64,
    objects: u64,
    values: u64,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Outcome {
    Completed {
        value: ValueWire,
    },
    Refused {
        code: &'static str,
        cause: &'static str,
        #[serde(skip_serializing_if = "Option::is_none")]
        fields: Option<BTreeMap<&'static str, String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        locus: Option<CallLocusWire>,
        #[serde(skip_serializing_if = "Option::is_none")]
        location: Option<CallLocationWire>,
    },
    Undefined {
        reason: &'static str,
    },
    Incomplete {
        limit: &'static str,
    },
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ValueWire {
    Boolean { value: bool },
    Integer { decimal: String },
}

#[derive(Serialize)]
struct CallLocusWire {
    file: String,
    span: LocatedSpan,
}

#[derive(Serialize)]
struct CallLocationWire {
    origin: CallOriginWire,
    path: Vec<usize>,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum CallOriginWire {
    Body {
        function: String,
        index: usize,
    },
    Measure {
        function: String,
        index: usize,
    },
    Expression,
    TypeDeclaration {
        name: String,
    },
    StateClause {
        clause: String,
        index: usize,
    },
    ProtocolAttempt {
        protocol: usize,
        attempt: usize,
    },
    ModelOperationClause {
        operation: String,
        clause: &'static str,
        position: usize,
    },
}

impl From<&quire_semantic_value::location::Location> for CallLocationWire {
    fn from(location: &quire_semantic_value::location::Location) -> Self {
        use quire_semantic_value::location::{OperationClauseKind, Origin};
        let origin = match &location.origin {
            Origin::Body { function, index } => CallOriginWire::Body {
                function: function.clone(),
                index: *index,
            },
            Origin::Measure { function, index } => CallOriginWire::Measure {
                function: function.clone(),
                index: *index,
            },
            Origin::Expression => CallOriginWire::Expression,
            Origin::TypeDeclaration { name } => {
                CallOriginWire::TypeDeclaration { name: name.clone() }
            }
            Origin::StateClause { clause, index } => CallOriginWire::StateClause {
                clause: clause.clone(),
                index: *index,
            },
            Origin::ProtocolAttempt { protocol, attempt } => CallOriginWire::ProtocolAttempt {
                protocol: *protocol,
                attempt: *attempt,
            },
            Origin::ModelOperationClause {
                operation,
                kind,
                position,
            } => CallOriginWire::ModelOperationClause {
                operation: operation.clone(),
                clause: match kind {
                    OperationClauseKind::Requires => "requires",
                    OperationClauseKind::Ensures => "ensures",
                },
                position: *position,
            },
        };
        Self {
            origin,
            path: location.path(),
        }
    }
}

#[derive(Serialize)]
struct InternalFaultWire {
    stage: &'static str,
    invariant: &'static str,
}

#[derive(Serialize)]
struct RefusalWire {
    code: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    cause: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fields: Option<BTreeMap<&'static str, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    locus: Option<crate::outcome::OutcomeLocus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frame_witness: Option<FrameWitnessWire>,
}

impl RefusalWire {
    fn new(code: &'static str, cause: Option<&'static str>) -> Self {
        Self {
            code,
            cause,
            fields: None,
            locus: None,
            frame_witness: None,
        }
    }
    fn fields(mut self, fields: BTreeMap<&'static str, String>) -> Self {
        self.fields = Some(fields);
        self
    }
}

#[derive(Serialize)]
struct FrameWitnessWire {
    code: &'static str,
    cause: &'static str,
    invocation: DocumentRefWire,
    pre: DocumentRefWire,
    post: DocumentRefWire,
    population: String,
    change: FrameChangeWire,
}

impl From<&FrameWitness> for FrameWitnessWire {
    fn from(value: &FrameWitness) -> Self {
        Self {
            code: value.code.as_str(),
            cause: value.cause,
            invocation: DocumentRefWire::from(&value.invocation),
            pre: DocumentRefWire::from(&value.pre),
            post: DocumentRefWire::from(&value.post),
            population: value.population.clone(),
            change: FrameChangeWire::from(&value.change),
        }
    }
}

#[derive(Serialize)]
struct DocumentRefWire {
    authority: String,
    identity: String,
    revision_namespace: String,
    revision: String,
    digest: String,
}

impl From<&DocumentRef> for DocumentRefWire {
    fn from(value: &DocumentRef) -> Self {
        let digest = DigestRecord::mint(DigestDomain::Sha256Jcs, value.digest);
        Self {
            authority: value.authority.clone(),
            identity: value.identity.clone(),
            revision_namespace: value.revision_namespace.clone(),
            revision: value.revision.clone(),
            digest: format!("sha256-jcs:{}", digest.hex()),
        }
    }
}

#[derive(Serialize)]
struct DeclarationKeyWire {
    package: String,
    node: String,
}

impl From<&DeclarationKey> for DeclarationKeyWire {
    fn from(value: &DeclarationKey) -> Self {
        Self {
            package: value.package.clone(),
            node: value.node.clone(),
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum FrameChangeWire {
    FieldWrite {
        object: String,
        field: String,
        modifies: Vec<DeclarationKeyWire>,
    },
    Created {
        object: String,
        type_name: DeclarationKeyWire,
        creates: Vec<DeclarationKeyWire>,
    },
    Deleted {
        object: String,
        type_name: DeclarationKeyWire,
        deletes: Vec<DeclarationKeyWire>,
    },
    Retyped {
        object: String,
        pre_type: DeclarationKeyWire,
        post_type: DeclarationKeyWire,
    },
}

impl From<&FrameChange> for FrameChangeWire {
    fn from(value: &FrameChange) -> Self {
        match value {
            FrameChange::FieldWrite {
                object,
                field,
                modifies,
            } => Self::FieldWrite {
                object: object.clone(),
                field: field.clone(),
                modifies: modifies.iter().map(DeclarationKeyWire::from).collect(),
            },
            FrameChange::Created {
                object,
                type_name,
                creates,
            } => Self::Created {
                object: object.clone(),
                type_name: DeclarationKeyWire::from(type_name),
                creates: creates.iter().map(DeclarationKeyWire::from).collect(),
            },
            FrameChange::Deleted {
                object,
                type_name,
                deletes,
            } => Self::Deleted {
                object: object.clone(),
                type_name: DeclarationKeyWire::from(type_name),
                deletes: deletes.iter().map(DeclarationKeyWire::from).collect(),
            },
            FrameChange::Retyped {
                object,
                pre_type,
                post_type,
            } => Self::Retyped {
                object: object.clone(),
                pre_type: DeclarationKeyWire::from(pre_type),
                post_type: DeclarationKeyWire::from(post_type),
            },
        }
    }
}

#[derive(Serialize)]
struct Disposition {
    stage: &'static str,
    category: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    truth: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    outcome: Option<Outcome>,
    #[serde(skip_serializing_if = "Option::is_none")]
    internal_fault: Option<InternalFaultWire>,
    #[serde(skip_serializing_if = "Option::is_none")]
    refusal: Option<RefusalWire>,
}

#[derive(Serialize)]
struct ClauseRunDocument {
    format: WireFormat,
    #[serde(skip_serializing_if = "Option::is_none")]
    package_id: Option<String>,
    disposition: Disposition,
    usage: Usage,
    basis: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    witness: Option<RecordWire<Box<RawValue>>>,
}

impl Disposition {
    fn of(value: &ClauseDisposition) -> Self {
        let mut result = Self {
            stage: match value.stage() {
                super::ClauseRunStage::Compile => "compile",
                super::ClauseRunStage::Select => "select",
                super::ClauseRunStage::Admit => "admit",
                super::ClauseRunStage::Evaluate => "evaluate",
            },
            category: value.category().as_str(),
            truth: value.truth(),
            outcome: None,
            internal_fault: None,
            refusal: None,
        };
        match value {
            ClauseDisposition::Compile(refusal) => {
                let mut record = RefusalWire::new(refusal.code().as_str(), refusal.cause());
                record.locus = refusal.region().map(|region| {
                    crate::outcome::OutcomeLocus::from(&Locus::Region(region.clone()))
                });
                let mut source: &super::CompileRefusal = refusal.as_ref();
                while let super::CompileRefusal::Dependency { refusal, .. } = source {
                    source = refusal.as_ref();
                }
                record.fields = source.fields();
                if let super::CompileRefusal::Limit(limit) = source {
                    record.locus = limit.locus().map(crate::outcome::OutcomeLocus::from);
                }
                result.refusal = Some(record);
            }
            ClauseDisposition::UnknownLanguage { language } => {
                result.refusal = Some(
                    RefusalWire::new(Code::UnknownLanguage.as_str(), None)
                        .fields(BTreeMap::from([("language", language.clone())])),
                );
            }
            ClauseDisposition::StalePackage { expected, actual } => {
                result.refusal = Some(
                    RefusalWire::new(Code::StaleDependency.as_str(), Some("stale-package")).fields(
                        BTreeMap::from([("expected", expected.clone()), ("actual", actual.hex())]),
                    ),
                );
            }
            ClauseDisposition::MissingName { name } => {
                result.refusal = Some(
                    RefusalWire::new(Code::MissingDeclaration.as_str(), Some("missing-name"))
                        .fields(BTreeMap::from([("name", name.clone())])),
                );
            }
            ClauseDisposition::NotAPredicate { name } => {
                result.refusal = Some(
                    RefusalWire::new(Code::IllTyped.as_str(), Some("type-mismatch"))
                        .fields(BTreeMap::from([("name", name.clone())])),
                );
            }
            ClauseDisposition::Admit(failure) => match failure {
                qsl_semantics::model::observation::AdmissionFailure::Refused(record)
                | qsl_semantics::model::observation::AdmissionFailure::Incomplete(record) => {
                    result.refusal = Some(
                        RefusalWire::new(record.code.as_str(), Some(record.cause))
                            .fields(record.fields.clone()),
                    );
                }
                qsl_semantics::model::observation::AdmissionFailure::Fault(fault) => {
                    result.internal_fault = Some(InternalFaultWire {
                        stage: fault.stage(),
                        invariant: fault.invariant(),
                    });
                }
            },
            ClauseDisposition::ArgumentRefusal(refusal) => {
                if let super::RunRefusal::Fault(fault) = refusal.as_ref() {
                    result.internal_fault = Some(InternalFaultWire {
                        stage: fault.stage(),
                        invariant: fault.invariant(),
                    });
                    return result;
                }
                let cause = match refusal.as_ref() {
                    super::RunRefusal::Compile(compile) => compile.cause(),
                    super::RunRefusal::Fault(_) => None,
                    super::RunRefusal::MissingDeclaration { .. }
                    | super::RunRefusal::UnsupportedResult { .. }
                    | super::RunRefusal::UnknownParameter { .. }
                    | super::RunRefusal::DuplicateArgument { .. }
                    | super::RunRefusal::UnboundParameter { .. }
                    | super::RunRefusal::WrongValueKind { .. }
                    | super::RunRefusal::Admission { .. }
                    | super::RunRefusal::DanglingReference { .. }
                    | super::RunRefusal::Cancelled(_) => None,
                };
                let mut record = RefusalWire::new(refusal.code().as_str(), cause);
                match refusal.as_ref() {
                    super::RunRefusal::Compile(compile) => {
                        record.locus = compile.region().map(|region| {
                            crate::outcome::OutcomeLocus::from(&Locus::Region(region.clone()))
                        });
                    }
                    super::RunRefusal::MissingDeclaration { function }
                    | super::RunRefusal::UnsupportedResult { function } => {
                        record = record.fields(BTreeMap::from([("function", function.clone())]));
                    }
                    super::RunRefusal::UnknownParameter { parameter }
                    | super::RunRefusal::DuplicateArgument { parameter }
                    | super::RunRefusal::UnboundParameter { parameter } => {
                        record = record.fields(BTreeMap::from([("parameter", parameter.clone())]));
                    }
                    super::RunRefusal::WrongValueKind { position } => {
                        record =
                            record.fields(BTreeMap::from([("position", position.to_string())]));
                    }
                    super::RunRefusal::Admission { position, refusal } => {
                        let mut fields = BTreeMap::from([("position", position.to_string())]);
                        match &refusal.cause {
                            qsl_semantics::model::normalize::ModelRefusalCause::ForeignModelSelection {
                                actual,
                                expected,
                            } => match actual {
                                qsl_semantics::model::refusal::OfferedSelection::Document(
                                    model_identity,
                                ) => {
                                    fields.insert("model_identity", model_identity.clone());
                                    fields.insert("expected", format_model_selection(expected));
                                }
                                qsl_semantics::model::refusal::OfferedSelection::Population(
                                    population,
                                ) => {
                                    fields.insert("population", format_declaration_key(population));
                                    fields.insert("expected", format_model_selection(expected));
                                }
                            },
                            qsl_semantics::model::normalize::ModelRefusalCause::ForeignType {
                                member,
                                type_name,
                            } => {
                                fields.insert("member", member.clone());
                                fields.insert("type", format_declaration_key(type_name));
                            }
                            qsl_semantics::model::normalize::ModelRefusalCause::ForeignUniverse {
                                actual,
                                expected,
                            } => {
                                fields.insert(
                                    "required",
                                    qsl_semantics::model::key::hex(expected.as_bytes()),
                                );
                                fields.insert("supplied", qsl_semantics::model::key::hex(actual));
                            }
                            _ => {}
                        }
                        record = record.fields(fields);
                    }
                    super::RunRefusal::DanglingReference { position } => {
                        record =
                            record.fields(BTreeMap::from([("position", position.to_string())]));
                    }
                    super::RunRefusal::Fault(_) | super::RunRefusal::Cancelled(_) => {}
                }
                result.refusal = Some(record);
            }
            ClauseDisposition::Evaluate(outcome) => match outcome {
                CallOutcome::Completed(CallValue::Boolean(value)) => {
                    result.outcome = Some(Outcome::Completed {
                        value: ValueWire::Boolean { value: *value },
                    });
                }
                CallOutcome::Completed(CallValue::Integer(value)) => {
                    result.outcome = Some(Outcome::Completed {
                        value: ValueWire::Integer {
                            decimal: value.to_string(),
                        },
                    });
                }
                CallOutcome::Completed(CallValue::Reference(_)) => {
                    result.category = Category::InternalFailure.as_str();
                    result.internal_fault = Some(InternalFaultWire {
                        stage: "native-result",
                        invariant: "clause-completes-reference",
                    });
                }
                CallOutcome::Refused(refusal) => {
                    result.outcome = Some(match refusal {
                        CallRefusal::Record {
                            code,
                            fields,
                            locus,
                            location,
                        } => Outcome::Refused {
                            code: code.code(),
                            cause: code.cause(),
                            fields: Some(fields.clone()),
                            locus: locus.as_ref().map(|locus| CallLocusWire {
                                file: locus.file.clone(),
                                span: locus.span,
                            }),
                            location: location.as_ref().map(CallLocationWire::from),
                        },
                        CallRefusal::Family { code, location } => Outcome::Refused {
                            code: code.code(),
                            cause: code.cause(),
                            fields: None,
                            locus: None,
                            location: location.as_ref().map(CallLocationWire::from),
                        },
                    });
                }
                CallOutcome::Undefined { reason } => {
                    result.outcome = Some(Outcome::Undefined { reason });
                }
                CallOutcome::Incomplete(incomplete) => {
                    result.outcome = Some(Outcome::Incomplete {
                        limit: incomplete.limits_field(),
                    });
                }
            },
            ClauseDisposition::FrameViolation(witness) => {
                let mut record = RefusalWire::new(witness.code.as_str(), Some(witness.cause));
                record.frame_witness = Some(FrameWitnessWire::from(witness.as_ref()));
                result.refusal = Some(record);
            }
            ClauseDisposition::EvaluateFault(fault) => {
                result.internal_fault = Some(InternalFaultWire {
                    stage: fault.stage(),
                    invariant: fault.invariant(),
                });
            }
        }
        result
    }
}

fn format_declaration_key(key: &qsl_semantics::model::key::DeclarationKey) -> String {
    format!("{}/{}", key.package, key.node)
}

fn format_model_selection(
    selection: &qsl_semantics::model::domain_package::DomainPackageRef,
) -> String {
    format!(
        "{}@{}#{}",
        selection.identity,
        selection.version,
        qsl_semantics::model::key::hex(&selection.digest)
    )
}

/// Serialize one FR-109 report as RFC 8785 `native-run-result/2` bytes.
pub fn write_clause_run_result(report: &ClauseRunReport) -> Result<Vec<u8>, NativeRunResultError> {
    if report.basis.is_decisive() != report.witness.is_some() {
        return Err(NativeRunResultError::BasisWitness);
    }
    let verdict = matches!(
        report.disposition.category(),
        qsl_foundation::diagnostic::Category::Success
            | qsl_foundation::diagnostic::Category::Violation
    );
    if (verdict
        && !matches!(
            report.basis,
            crate::proof_result::SettlementBasis::ClosedScope
                | crate::proof_result::SettlementBasis::DecisiveWitness
                | crate::proof_result::SettlementBasis::DecisiveCounterexample
        ))
        || (!verdict
            && !matches!(
                report.basis,
                crate::proof_result::SettlementBasis::Unavailable
            ))
    {
        return Err(NativeRunResultError::BasisDisposition);
    }
    let witness = report.witness.as_ref().map(RecordWire::of).transpose()?;
    let document = ClauseRunDocument {
        format: WireFormat::RunResult,
        package_id: report.package_id.as_ref().map(|id| id.hex()),
        disposition: Disposition::of(&report.disposition),
        usage: Usage {
            admission: AdmissionUsage {
                document_bytes: report.usage.admission_consumed.document_bytes,
                objects: report.usage.admission_consumed.objects,
                values: report.usage.admission_consumed.values,
            },
            evaluation_admissions: report.usage.evaluation_admissions,
            evaluation_consumed: report
                .usage
                .evaluation_consumed
                .iter()
                .map(|(kind, consumed)| CounterUsage {
                    kind: kind.as_str(),
                    consumed: *consumed,
                })
                .collect(),
        },
        basis: report.basis.as_str(),
        witness,
    };
    let json = serde_json::to_vec(&document)?;
    let parsed = quire_canonical::read(&json, u64::MAX)?;
    Ok(quire_canonical::to_vec(
        &parsed.root(),
        quire_canonical::Limits::new(u64::MAX),
    )?)
}
