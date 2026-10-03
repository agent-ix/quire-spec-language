// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-269: the [`DisagreementCause`] wire codec and its strict reader.
//!
//! A `Witness` cause's `given` and `derived` records are each present in
//! full or absent (QSpec FR-351 has no partial record). An absent member is
//! an absent JSON member, never `null` or a default (QSpec FR-352-AC-7).
//! The record's members follow QSpec FR-352's `witness` table and its value
//! path QSpec FR-207's `runtime_value` form. The reader refuses an unknown
//! member, an unknown tag, a `null` member and a record missing a component
//! its family assigns.

use std::collections::BTreeMap;

use qsl_foundation::diagnostic::catalog_category;
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::witness::{
    ObservationIdentity, RuntimeValuePath, SeparationStep, ValuePathStep, ValuePathSubject,
};
use qsl_package::CheckedPackage;
use quire_exact::{Origin, Role, Value};
use quire_semantic_value::location::{Location, Origin as Declaration};
use serde::{Deserialize, Deserializer, Serialize};

use super::{
    DisagreementCause, SeparatingWitnessRecord, SeparationReason, SeparationRefusal, Verdict,
    WitnessFailure,
};
use crate::identity::TracePosition;
use qsl_foundation::diagnostic::Category;

mod value;
use value::{encode_element, reference_of, ElementWire, Keys};

/// Why a [`DisagreementCause`] did not encode or read.
#[derive(Debug, thiserror::Error)]
pub enum CauseCodecError {
    /// The document is not a cause this reader admits: malformed JSON, an
    /// unknown or missing member, an unknown tag, or a `null` member.
    #[error("the cause document does not read: {0}")]
    Malformed(#[from] serde_json::Error),
    /// A digest member is not 64 lowercase hexadecimal digits.
    #[error("the member {0} is not a 32-byte lowercase hex digest")]
    Digest(&'static str),
    /// An integer member is not a decimal integer.
    #[error("the member {0} is not a decimal integer")]
    Integer(&'static str),
    /// An identity member is empty.
    #[error("the member {0} is empty")]
    Empty(&'static str),
    /// A record naming a deciding quantifier carries no index: a collection
    /// quantifier's family assigns one (QSpec FR-351).
    #[error("a record naming a deciding quantifier carries no index")]
    MissingIndex,
    /// A deciding element of a kind this codec has no encoding for: a
    /// population, which no claim's domain holds.
    #[error("the deciding element's kind has no encoding")]
    UnsupportedElement,
    /// A declaration identity the checked package does not hold.
    #[error("the member {0} names no declaration of the checked package")]
    Unresolved(&'static str),
    /// A value member outside its kind's domain, or not in canonical form.
    #[error("the {0} is not a canonical value of its kind")]
    Value(&'static str),
    /// A refusal's code is not a catalog code, or its cause or a field name
    /// is empty or not a catalog spelling.
    #[error("the refusal's {0} is not a catalog spelling")]
    Spelling(&'static str),
}

impl DisagreementCause {
    /// FR-269: this cause's JSON document.
    pub fn to_json(&self) -> Result<String, CauseCodecError> {
        Ok(serde_json::to_string(&CauseWire::of(self)?)?)
    }

    /// FR-269: read a cause from `text`, refusing any document that is not
    /// exactly a cause this codec writes. Declaration identities resolve
    /// among those `package` admitted.
    pub fn from_json(text: &str, package: &CheckedPackage) -> Result<Self, CauseCodecError> {
        serde_json::from_str::<CauseWire>(text)?.read(Keys::new(package))
    }
}

/// An optional member that must be absent rather than `null`: present, it
/// reads as `T` (so `null` refuses); absent, `#[serde(default)]` gives
/// `None`.
fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "cause", rename_all = "kebab-case", deny_unknown_fields)]
enum CauseWire {
    Verdicts {
        proved: VerdictWire,
        replayed: VerdictWire,
    },
    NoValue {
        proved: VerdictWire,
        replayed: VerdictWire,
    },
    Witness {
        proved: VerdictWire,
        replayed: VerdictWire,
        #[serde(
            default,
            deserialize_with = "present",
            skip_serializing_if = "Option::is_none"
        )]
        given: Option<Box<RecordWire>>,
        #[serde(
            default,
            deserialize_with = "present",
            skip_serializing_if = "Option::is_none"
        )]
        derived: Option<Box<RecordWire>>,
        failure: FailureWire,
    },
}

impl CauseWire {
    fn of(cause: &DisagreementCause) -> Result<Self, CauseCodecError> {
        Ok(match cause {
            DisagreementCause::Verdicts { proved, replayed } => Self::Verdicts {
                proved: VerdictWire::of(*proved),
                replayed: VerdictWire::of(*replayed),
            },
            DisagreementCause::NoValue { proved, replayed } => Self::NoValue {
                proved: VerdictWire::of(*proved),
                replayed: VerdictWire::of(*replayed),
            },
            DisagreementCause::Witness {
                proved,
                replayed,
                given,
                derived,
                failure,
            } => Self::Witness {
                proved: VerdictWire::of(*proved),
                replayed: VerdictWire::of(*replayed),
                given: given
                    .as_deref()
                    .map(RecordWire::of)
                    .transpose()?
                    .map(Box::new),
                derived: derived
                    .as_deref()
                    .map(RecordWire::of)
                    .transpose()?
                    .map(Box::new),
                failure: FailureWire::of(failure),
            },
        })
    }

    fn read(self, keys: Keys<'_>) -> Result<DisagreementCause, CauseCodecError> {
        Ok(match self {
            Self::Verdicts { proved, replayed } => DisagreementCause::Verdicts {
                proved: proved.read(),
                replayed: replayed.read(),
            },
            Self::NoValue { proved, replayed } => DisagreementCause::NoValue {
                proved: proved.read(),
                replayed: replayed.read(),
            },
            Self::Witness {
                proved,
                replayed,
                given,
                derived,
                failure,
            } => DisagreementCause::Witness {
                proved: proved.read(),
                replayed: replayed.read(),
                given: given
                    .map(|record| record.read(keys))
                    .transpose()?
                    .map(Box::new),
                derived: derived
                    .map(|record| record.read(keys))
                    .transpose()?
                    .map(Box::new),
                failure: failure.read()?,
            },
        })
    }
}

/// A verdict on the wire: its category's [`Category::as_str`] label. A
/// proof or replay verdict is never `undefined` (FR-285), so the reader
/// refuses that label and every label `Category` does not spell.
#[derive(Clone, Copy)]
struct VerdictWire(Category);

impl VerdictWire {
    fn of(verdict: Verdict) -> Self {
        Self(verdict.category())
    }

    fn read(self) -> Verdict {
        Verdict::from_category(self.0)
    }
}

impl Serialize for VerdictWire {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0.as_str())
    }
}

impl<'de> Deserialize<'de> for VerdictWire {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let label = String::deserialize(deserializer)?;
        Category::ALL
            .into_iter()
            .filter(|category| *category != Category::Undefined)
            .find(|category| category.as_str() == label)
            .map(Self)
            .ok_or_else(|| serde::de::Error::custom(format!("not a verdict: {label}")))
    }
}

#[cfg(test)]
mod verdict_tests {
    use super::{Category, VerdictWire};

    /// Every verdict category round-trips through its `as_str` label, and
    /// `undefined` and an unknown label are refused.
    #[test]
    fn verdict_labels_round_trip_and_undefined_is_refused() {
        for category in Category::ALL {
            let label = serde_json::to_string(&VerdictWire(category)).unwrap();
            assert_eq!(label, format!("\"{}\"", category.as_str()));
            let read = serde_json::from_str::<VerdictWire>(&label);
            if category == Category::Undefined {
                assert!(read.is_err(), "undefined must not read");
            } else {
                assert_eq!(read.unwrap().0, category);
            }
        }
        assert!(serde_json::from_str::<VerdictWire>("\"maybe\"").is_err());
    }
}

/// QSpec FR-352's `witness` record.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordWire {
    quantifier: OccurrenceWire,
    deciding_element: ElementWire,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    index: Option<u64>,
    value_path: PathWire,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    trace_position: Option<String>,
}

impl RecordWire {
    fn of(record: &SeparatingWitnessRecord) -> Result<Self, CauseCodecError> {
        if record.index.is_none() {
            return Err(CauseCodecError::MissingIndex);
        }
        Ok(Self {
            quantifier: OccurrenceWire::of(&record.quantifier),
            deciding_element: ElementWire::of(&record.deciding_element)?,
            index: record.index,
            value_path: PathWire::of(&record.value_path),
            trace_position: record
                .trace_position
                .as_ref()
                .map(|position| position.as_str().to_owned()),
        })
    }

    fn read(self, keys: Keys<'_>) -> Result<SeparatingWitnessRecord, CauseCodecError> {
        // The record names a deciding quantifier, so its family is a
        // collection quantifier's, which assigns an index (QSpec FR-351).
        let index = self.index.ok_or(CauseCodecError::MissingIndex)?;
        Ok(SeparatingWitnessRecord {
            quantifier: self.quantifier.read()?,
            deciding_element: self.deciding_element.read(keys)?,
            index: Some(index),
            value_path: self.value_path.read(keys)?,
            trace_position: self.trace_position.map(TracePosition::new),
        })
    }
}

/// An occurrence key: `node_id`, `role`, `ordinal` (QSpec FR-322).
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OccurrenceWire {
    node_id: String,
    role: String,
    ordinal: u64,
}

impl OccurrenceWire {
    fn of(occurrence: &OccurrenceKey) -> Self {
        Self {
            node_id: hex(occurrence.node().as_bytes()),
            role: occurrence.origin().role().as_str().to_owned(),
            ordinal: occurrence.origin().ordinal(),
        }
    }

    fn read(self) -> Result<OccurrenceKey, CauseCodecError> {
        Ok(OccurrenceKey::new(
            WireNodeId::from_digest(digest(&self.node_id, "node_id")?),
            Origin::new(Role::new(self.role), self.ordinal),
        ))
    }
}

/// QSpec FR-207's `runtime_value` wire form.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PathWire {
    kind: PathKind,
    root: RootWire,
    steps: Vec<StepWire>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PathKind {
    RuntimeValue,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RootWire {
    observation: ObservationWire,
    subject: SubjectWire,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservationWire {
    authority: String,
    identity: String,
    revision_namespace: String,
    revision: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "tag", rename_all = "snake_case", deny_unknown_fields)]
enum SubjectWire {
    Object {
        universe: String,
        #[serde(rename = "type")]
        object_type: String,
        object_identity: String,
    },
    Built {
        occurrence: OccurrenceWire,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "tag", rename_all = "snake_case", deny_unknown_fields)]
enum StepWire {
    Field { name: String },
    OptionValue,
    Member { name: String },
    Index { index: u64 },
}

impl PathWire {
    fn of(path: &RuntimeValuePath) -> Self {
        let observation = &path.observation;
        Self {
            kind: PathKind::RuntimeValue,
            root: RootWire {
                observation: ObservationWire {
                    authority: observation.authority.clone(),
                    identity: observation.identity.clone(),
                    revision_namespace: observation.revision_namespace.clone(),
                    revision: observation.revision.clone(),
                },
                subject: match &path.subject {
                    ValuePathSubject::Object(object) => SubjectWire::Object {
                        universe: hex(object.universe().as_bytes()),
                        object_type: hex(object.object_type().as_bytes()),
                        object_identity: object.object().as_str().to_owned(),
                    },
                    ValuePathSubject::Built(occurrence) => SubjectWire::Built {
                        occurrence: OccurrenceWire::of(occurrence),
                    },
                },
            },
            steps: path
                .steps
                .iter()
                .map(|step| match step {
                    ValuePathStep::Field(name) => StepWire::Field { name: name.clone() },
                    ValuePathStep::OptionValue => StepWire::OptionValue,
                    ValuePathStep::Member(name) => StepWire::Member { name: name.clone() },
                    ValuePathStep::Index(index) => StepWire::Index { index: *index },
                })
                .collect(),
        }
    }

    fn read(self, keys: Keys<'_>) -> Result<RuntimeValuePath, CauseCodecError> {
        let PathKind::RuntimeValue = self.kind;
        let observation = self.root.observation;
        Ok(RuntimeValuePath {
            observation: ObservationIdentity {
                authority: observation.authority,
                identity: observation.identity,
                revision_namespace: observation.revision_namespace,
                revision: observation.revision,
            },
            subject: match self.root.subject {
                SubjectWire::Object {
                    universe,
                    object_type,
                    object_identity,
                } => ValuePathSubject::Object(reference_of(
                    keys,
                    &universe,
                    &object_type,
                    object_identity,
                )?),
                SubjectWire::Built { occurrence } => ValuePathSubject::Built(occurrence.read()?),
            },
            steps: self
                .steps
                .into_iter()
                .map(|step| match step {
                    StepWire::Field { name } => ValuePathStep::Field(name),
                    StepWire::OptionValue => ValuePathStep::OptionValue,
                    StepWire::Member { name } => ValuePathStep::Member(name),
                    StepWire::Index { index } => ValuePathStep::Index(index),
                })
                .collect(),
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "failure", rename_all = "kebab-case", deny_unknown_fields)]
enum FailureWire {
    Mismatch,
    Separation { step: StepName, reason: ReasonWire },
}

/// FR-268's step names.
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum StepName {
    Quantifier,
    Domain,
    Element,
    Body,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "kebab-case", deny_unknown_fields)]
enum ReasonWire {
    Unmet,
    UndefinedEvaluation {
        expression: LocationWire,
        cause: String,
    },
    Refused {
        code: String,
        cause: String,
        fields: BTreeMap<String, String>,
    },
}

impl FailureWire {
    fn of(failure: &WitnessFailure) -> Self {
        match failure {
            WitnessFailure::Mismatch => Self::Mismatch,
            WitnessFailure::Separation { step, reason } => Self::Separation {
                step: match step {
                    SeparationStep::Quantifier => StepName::Quantifier,
                    SeparationStep::Domain => StepName::Domain,
                    SeparationStep::Element => StepName::Element,
                    SeparationStep::Body => StepName::Body,
                },
                reason: match reason {
                    SeparationReason::Unmet => ReasonWire::Unmet,
                    SeparationReason::UndefinedEvaluation { expression, cause } => {
                        ReasonWire::UndefinedEvaluation {
                            expression: LocationWire::of(expression),
                            cause: cause.clone(),
                        }
                    }
                    SeparationReason::Refused(refusal) => ReasonWire::Refused {
                        code: refusal.code.clone(),
                        cause: refusal.cause.clone(),
                        fields: refusal.fields.clone(),
                    },
                },
            },
        }
    }

    fn read(self) -> Result<WitnessFailure, CauseCodecError> {
        Ok(match self {
            Self::Mismatch => WitnessFailure::Mismatch,
            Self::Separation { step, reason } => WitnessFailure::Separation {
                step: match step {
                    StepName::Quantifier => SeparationStep::Quantifier,
                    StepName::Domain => SeparationStep::Domain,
                    StepName::Element => SeparationStep::Element,
                    StepName::Body => SeparationStep::Body,
                },
                reason: match reason {
                    ReasonWire::Unmet => SeparationReason::Unmet,
                    ReasonWire::UndefinedEvaluation { expression, cause } => {
                        SeparationReason::UndefinedEvaluation {
                            expression: expression.read(),
                            cause,
                        }
                    }
                    ReasonWire::Refused {
                        code,
                        cause,
                        fields,
                    } => SeparationReason::Refused(refusal(code, cause, fields)?),
                },
            },
        })
    }
}

/// A refusal read from the wire: its code is a catalog code, and its cause
/// and each field name are catalog spellings, lower-case words joined by
/// `-` or `_`.
fn refusal(
    code: String,
    cause: String,
    fields: BTreeMap<String, String>,
) -> Result<SeparationRefusal, CauseCodecError> {
    if catalog_category(&code).is_none() {
        return Err(CauseCodecError::Spelling("code"));
    }
    if !is_spelling(&cause) {
        return Err(CauseCodecError::Spelling("cause"));
    }
    if !fields.keys().all(|name| is_spelling(name)) {
        return Err(CauseCodecError::Spelling("field"));
    }
    Ok(SeparationRefusal {
        code,
        cause,
        fields,
    })
}

/// Whether `text` is a catalog spelling: lower-case ASCII words of letters
/// and digits, joined by single `-` or `_`.
fn is_spelling(text: &str) -> bool {
    !text.is_empty()
        && text.split(['-', '_']).all(|word| {
            !word.is_empty()
                && word
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// An expression location: its declaration and child-index path.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LocationWire {
    declaration: DeclarationWire,
    path: Vec<usize>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum DeclarationWire {
    Body { function: String, index: usize },
    Measure { function: String, index: usize },
    Expression,
    TypeDeclaration { name: String },
    StateClause { clause: String, index: usize },
    ProtocolAttempt { protocol: usize, attempt: usize },
}

impl LocationWire {
    fn of(location: &Location) -> Self {
        Self {
            declaration: match &location.origin {
                Declaration::Body { function, index } => DeclarationWire::Body {
                    function: function.clone(),
                    index: *index,
                },
                Declaration::Measure { function, index } => DeclarationWire::Measure {
                    function: function.clone(),
                    index: *index,
                },
                Declaration::Expression => DeclarationWire::Expression,
                Declaration::TypeDeclaration { name } => {
                    DeclarationWire::TypeDeclaration { name: name.clone() }
                }
                Declaration::StateClause { clause, index } => DeclarationWire::StateClause {
                    clause: clause.clone(),
                    index: *index,
                },
                Declaration::ProtocolAttempt { protocol, attempt } => {
                    DeclarationWire::ProtocolAttempt {
                        protocol: *protocol,
                        attempt: *attempt,
                    }
                }
            },
            path: location.path(),
        }
    }

    fn read(self) -> Location {
        Location::at(
            match self.declaration {
                DeclarationWire::Body { function, index } => Declaration::Body { function, index },
                DeclarationWire::Measure { function, index } => {
                    Declaration::Measure { function, index }
                }
                DeclarationWire::Expression => Declaration::Expression,
                DeclarationWire::TypeDeclaration { name } => Declaration::TypeDeclaration { name },
                DeclarationWire::StateClause { clause, index } => {
                    Declaration::StateClause { clause, index }
                }
                DeclarationWire::ProtocolAttempt { protocol, attempt } => {
                    Declaration::ProtocolAttempt { protocol, attempt }
                }
            },
            &self.path,
        )
    }
}

/// Whether `left` and `right` have the same typed value encoding; a value
/// with no encoding is the same as nothing.
pub(super) fn same_encoding(left: &Value, right: &Value) -> bool {
    matches!(
        (encode_element(left), encode_element(right)),
        (Ok(left), Ok(right)) if left.get() == right.get()
    )
}

/// `bytes` as 64 lowercase hexadecimal digits.
fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// A 32-byte digest from exactly 64 lowercase hexadecimal digits.
fn digest(text: &str, member: &'static str) -> Result<[u8; 32], CauseCodecError> {
    WireNodeId::from_hex(text)
        .map(|id| *id.as_bytes())
        .ok_or(CauseCodecError::Digest(member))
}
