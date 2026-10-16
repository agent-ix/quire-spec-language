// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-071: the request's optional `object_environment` member.
//!
//! The member names one `quire.state.snapshot/v1` document of the byte
//! provision: its four identity labels and its `sha256-jcs` digest, in the
//! form FR-106's invocation uses for its `pre` and `post` members. The
//! reader reads the member alone. It reads no document and requires no
//! byte-provision entry: the executor admits the document by FR-106
//! (FR-098).

use qsl_foundation::digest::{DigestDomain, DigestRecord};
use qsl_foundation::Code;
use qsl_semantics::model::observation::{snapshot_document_digest, write_environment_document};
pub use qsl_semantics::model::observation::{
    EnvironmentLabels, EnvironmentModel, EnvironmentObject, EnvironmentPopulation,
};
use serde_json::{Map, Value};

const MEMBER: &str = "object_environment";
const LABELS: [&str; 4] = ["authority", "identity", "revision_namespace", "revision"];

/// The `object_environment` member of a replay request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectEnvironmentRef {
    identity: EnvironmentLabels,
    digest: DigestRecord,
}

/// Why the member is refused at decode: always
/// `invalid_runtime_input`, naming the member or the member inside it.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ObjectEnvironmentRefusal {
    /// The member, `identity` or `digest`, or an identity label, is of the
    /// wrong JSON kind.
    #[error("invalid_runtime_input/wrong-value-kind: `{member}`")]
    WrongValueKind {
        /// The member at fault, as a dotted path.
        member: String,
    },
    /// `identity`, `digest` or an identity label is missing.
    #[error("invalid_runtime_input/missing-member: `{member}`")]
    MissingMember {
        /// The missing member, as a dotted path.
        member: String,
    },
    /// The first member in document order the closed object does not name.
    #[error("invalid_runtime_input/unknown-member: `{member}`")]
    UnknownMember {
        /// The unknown member, as a dotted path.
        member: String,
    },
    /// `digest` is not 64 lowercase hex digits in the `sha256-jcs` domain.
    #[error("invalid_runtime_input/invalid-value: `{member}`")]
    InvalidValue {
        /// The member at fault, as a dotted path.
        member: String,
    },
}

impl ObjectEnvironmentRefusal {
    /// The catalog code of this refusal.
    pub const fn code(&self) -> Code {
        Code::InvalidRuntimeInput
    }

    /// The catalog cause tag.
    pub const fn cause(&self) -> &'static str {
        match self {
            Self::WrongValueKind { .. } => "wrong-value-kind",
            Self::MissingMember { .. } => "missing-member",
            Self::UnknownMember { .. } => "unknown-member",
            Self::InvalidValue { .. } => "invalid-value",
        }
    }

    /// The member the refusal names, as a dotted path.
    pub fn member(&self) -> &str {
        match self {
            Self::WrongValueKind { member }
            | Self::MissingMember { member }
            | Self::UnknownMember { member }
            | Self::InvalidValue { member } => member,
        }
    }
}

impl ObjectEnvironmentRef {
    /// The member naming the document with `digest`.
    ///
    /// # Errors
    ///
    /// `invalid-value` naming `object_environment.digest` when `digest` is
    /// not in the `sha256-jcs` domain.
    pub fn new(
        identity: EnvironmentLabels,
        digest: DigestRecord,
    ) -> Result<Self, ObjectEnvironmentRefusal> {
        if digest.domain() != DigestDomain::Sha256Jcs {
            return Err(invalid_digest());
        }
        Ok(Self { identity, digest })
    }

    /// The labels retained with the digest.
    pub fn identity(&self) -> &EnvironmentLabels {
        &self.identity
    }

    /// The document's `sha256-jcs` digest.
    pub fn digest(&self) -> DigestRecord {
        self.digest
    }

    /// The member in its wire form.
    pub fn to_value(&self) -> Value {
        let labels = &self.identity;
        let mut identity = Map::new();
        for (name, label) in LABELS.into_iter().zip([
            &labels.authority,
            &labels.identity,
            &labels.revision_namespace,
            &labels.revision,
        ]) {
            identity.insert(name.to_owned(), Value::String(label.clone()));
        }
        let mut member = Map::new();
        member.insert("identity".to_owned(), Value::Object(identity));
        member.insert(
            "digest".to_owned(),
            Value::String(format!(
                "{}:{}",
                self.digest.domain().as_str(),
                self.digest.hex()
            )),
        );
        Value::Object(member)
    }
}

fn path(inner: &str) -> String {
    format!("{MEMBER}.{inner}")
}

fn invalid_digest() -> ObjectEnvironmentRefusal {
    ObjectEnvironmentRefusal::InvalidValue {
        member: path("digest"),
    }
}

fn labels(identity: &Value) -> Result<EnvironmentLabels, ObjectEnvironmentRefusal> {
    let Value::Object(members) = identity else {
        return Err(ObjectEnvironmentRefusal::WrongValueKind {
            member: path("identity"),
        });
    };
    if let Some(unknown) = members.keys().find(|name| !LABELS.contains(&name.as_str())) {
        return Err(ObjectEnvironmentRefusal::UnknownMember {
            member: path(&format!("identity.{unknown}")),
        });
    }
    let label = |name: &str| match members.get(name) {
        None => Err(ObjectEnvironmentRefusal::MissingMember {
            member: path(&format!("identity.{name}")),
        }),
        Some(Value::String(text)) => Ok(text.clone()),
        Some(_) => Err(ObjectEnvironmentRefusal::WrongValueKind {
            member: path(&format!("identity.{name}")),
        }),
    };
    Ok(EnvironmentLabels {
        authority: label("authority")?,
        identity: label("identity")?,
        revision_namespace: label("revision_namespace")?,
        revision: label("revision")?,
    })
}

/// Read a wire `object_environment` member (FR-071): a closed object with
/// exactly `identity`, the four identity labels, and `digest`, a
/// `sha256-jcs:` digest of 64 lowercase hex digits.
///
/// # Errors
///
/// [`ObjectEnvironmentRefusal`] for a member that is not an object, an
/// `identity` or `digest` of the wrong JSON kind, a missing member, an
/// unknown member, or a `digest` outside the `sha256-jcs` domain or whose
/// hex is not 64 lowercase digits.
pub fn read_object_environment(
    value: &Value,
) -> Result<ObjectEnvironmentRef, ObjectEnvironmentRefusal> {
    let Value::Object(members) = value else {
        return Err(ObjectEnvironmentRefusal::WrongValueKind {
            member: MEMBER.to_owned(),
        });
    };
    if let Some(unknown) = members
        .keys()
        .find(|name| !matches!(name.as_str(), "identity" | "digest"))
    {
        return Err(ObjectEnvironmentRefusal::UnknownMember {
            member: path(unknown),
        });
    }
    let member = |name: &str| {
        members
            .get(name)
            .ok_or_else(|| ObjectEnvironmentRefusal::MissingMember { member: path(name) })
    };
    let identity = member("identity")?;
    let digest = member("digest")?;
    let identity = labels(identity)?;
    let Value::String(digest) = digest else {
        return Err(ObjectEnvironmentRefusal::WrongValueKind {
            member: path("digest"),
        });
    };
    let (domain, hex) = digest.split_once(':').ok_or_else(invalid_digest)?;
    let digest = DigestRecord::from_wire_expecting(DigestDomain::Sha256Jcs, Some(domain), hex)
        .map_err(|_| invalid_digest())?;
    ObjectEnvironmentRef::new(identity, digest)
}

/// An object environment document written for a request: its canonical
/// bytes, its digest and the `object_environment` member that names it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectEnvironmentDocument {
    bytes: Vec<u8>,
    member: ObjectEnvironmentRef,
}

/// Why a document could not be written: a broken invariant of the writer.
#[derive(Debug, thiserror::Error)]
#[error("the object environment document could not be written: {0:?}")]
pub struct ObjectEnvironmentWriteError(qsl_semantics::model::observation::AdmissionFailure);

impl ObjectEnvironmentDocument {
    /// The document's canonical bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The `object_environment` member: the labels and the `sha256-jcs`
    /// digest.
    pub fn member(&self) -> &ObjectEnvironmentRef {
        &self.member
    }

    /// The document as a `ReplayRequestWire::byte_provision` entry, under
    /// its `sha256-jcs` digest.
    pub fn provision_entry(&self) -> (Option<String>, String, Vec<u8>) {
        (
            Some(DigestDomain::Sha256Jcs.as_str().to_owned()),
            self.member.digest().hex(),
            self.bytes.clone(),
        )
    }
}

/// Write the `quire.state.snapshot/v1` document of `populations`, the
/// objects listed by full reference (population and key) with every
/// declared field, as the FR-106 admission the replay runs reads it
/// (FR-098). It returns the bytes, the digest and the labels as the FR-071
/// `object_environment` member.
///
/// ```
/// use qsl_replay::{
///     write_object_environment, EnvironmentLabels, EnvironmentModel, EnvironmentObject,
///     EnvironmentPopulation, SnapshotValue,
/// };
///
/// let node = EnvironmentObject {
///     key: "a".to_owned(),
///     type_identity: "ix://test/graph/Node".to_owned(),
///     fields: vec![
///         (
///             "items".to_owned(),
///             SnapshotValue::sequence(vec![SnapshotValue::integer("3")]),
///         ),
///         ("next".to_owned(), SnapshotValue::absent()),
///     ],
/// };
/// let document = write_object_environment(
///     EnvironmentLabels {
///         authority: "agent-ix".to_owned(),
///         identity: "ix://test/graph/environment".to_owned(),
///         revision_namespace: "git".to_owned(),
///         revision: "1".to_owned(),
///     },
///     &EnvironmentModel {
///         identity: "test/graph".to_owned(),
///         version: "1.0.0".to_owned(),
///         digest: [7; 32],
///     },
///     &[EnvironmentPopulation {
///         population: "ix://test/graph/G".to_owned(),
///         complete: true,
///         objects: vec![node],
///     }],
/// )
/// .unwrap();
/// assert_eq!(document.member().digest().hex().len(), 64);
/// ```
///
/// # Errors
///
/// [`ObjectEnvironmentWriteError`] when the document has no canonical form.
pub fn write_object_environment(
    identity: EnvironmentLabels,
    model: &EnvironmentModel,
    populations: &[EnvironmentPopulation],
) -> Result<ObjectEnvironmentDocument, ObjectEnvironmentWriteError> {
    let bytes = write_environment_document(&identity, model, populations)
        .map_err(ObjectEnvironmentWriteError)?;
    let digest = snapshot_document_digest(&bytes).map_err(ObjectEnvironmentWriteError)?;
    let member = ObjectEnvironmentRef {
        identity,
        digest: DigestRecord::mint(DigestDomain::Sha256Jcs, digest),
    };
    Ok(ObjectEnvironmentDocument { bytes, member })
}
