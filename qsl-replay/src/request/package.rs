// SPDX-License-Identifier: AGPL-3.0-or-later
//! Complete raw checked-package input retained until native admission.

use super::{PackageMemberCause, ReplayRequestRefusal};
use crate::identity::SourceRefWire;
use quire_canonical::{Document, Node, NodeRef, ReadError};
use quire_contract_model::CheckedRefinementInternal;
use quire_contract_model::{
    read_checked_package, CheckedPackageDispatchResult, CheckedPackageEvidence,
    CheckedPackageIncomplete, CheckedPackageReadLimits, CheckedPackageRefusal, CheckedPackageV2,
};

/// A complete raw checked-package document, without an admission claim.
///
/// The shared JSON document has flat ownership. Original bytes are retained
/// unchanged so native admission still sees noncanonical numbers and spellings.
#[derive(Clone)]
pub struct ReplayPackage {
    document: Document,
    bytes: Vec<u8>,
}

impl std::fmt::Debug for ReplayPackage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ReplayPackage")
            .field("input_bytes", &self.bytes.len())
            .finish()
    }
}

impl PartialEq for ReplayPackage {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl Eq for ReplayPackage {}

impl ReplayPackage {
    /// Read one complete raw JSON document under the caller's byte ceiling.
    /// This checks JSON syntax only, not the checked-package contract.
    pub fn read(bytes: &[u8], max_input_bytes: u64) -> Result<Self, ReadError> {
        let document = quire_canonical::read(bytes, max_input_bytes)?;
        let mut retained = Vec::new();
        retained
            .try_reserve(bytes.len())
            .map_err(|_| ReadError::Allocation {
                requested: bytes.len(),
            })?;
        retained.extend_from_slice(bytes);
        Ok(Self {
            document,
            bytes: retained,
        })
    }

    /// The entire original document, including every graph and sidecar member.
    #[must_use]
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// The original bytes passed to the native checked-package reader.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Admit the complete original document using caller-supplied genuine
    /// evidence and native limits. No partial admitted package is returned.
    pub fn admit(
        &self,
        evidence: &CheckedPackageEvidence,
        limits: CheckedPackageReadLimits,
    ) -> Result<Box<CheckedPackageV2>, ReplayPackageAdmissionRefusal> {
        match read_checked_package(&self.bytes, limits, evidence) {
            CheckedPackageDispatchResult::AdmittedV2(package) => Ok(package),
            CheckedPackageDispatchResult::Refused(refusal) => {
                Err(ReplayPackageAdmissionRefusal::from_native(refusal))
            }
            CheckedPackageDispatchResult::Incomplete(limit) => {
                Err(ReplayPackageAdmissionRefusal::Incomplete(limit))
            }
            CheckedPackageDispatchResult::CatalogUnavailable(error) => {
                Err(ReplayPackageAdmissionRefusal::CatalogUnavailable(error))
            }
        }
    }

    pub(super) fn descriptor(&self) -> Result<Descriptor, ReplayRequestRefusal> {
        let root = self.document.root();
        let name = string(required(root, "name", "/package")?, "/package/name")?.to_owned();
        let lock = required(root, "lock", "/package")?;
        let sources_node = required(lock, "sources", "/package/lock")?;
        let sources = array(sources_node, "/package/lock/sources")?
            .enumerate()
            .map(|(index, source)| {
                let pointer = format!("/package/lock/sources/{index}");
                Ok((
                    string(
                        required(source, "authority", &pointer)?,
                        &format!("{pointer}/authority"),
                    )?
                    .to_owned(),
                    string(
                        required(source, "identity", &pointer)?,
                        &format!("{pointer}/identity"),
                    )?
                    .to_owned(),
                ))
            })
            .collect::<Result<Vec<_>, ReplayRequestRefusal>>()?;
        let refinement_internals = member(lock, "refinement_internals")
            .map(|rows| read_internals(rows, "/package/lock/refinement_internals"))
            .transpose()?;
        Ok(Descriptor {
            name,
            sources,
            refinement_internals,
        })
    }
}

pub(super) struct Descriptor {
    pub(super) name: String,
    pub(super) sources: Vec<SourceRefWire>,
    pub(super) refinement_internals: Option<Vec<CheckedRefinementInternal>>,
}

fn read_internals(
    value: NodeRef<'_>,
    pointer: &str,
) -> Result<Vec<CheckedRefinementInternal>, ReplayRequestRefusal> {
    array(value, pointer)?
        .enumerate()
        .map(|(index, row)| {
            let pointer = format!("{pointer}/{index}");
            let refinement = string(
                required(row, "refinement", &pointer)?,
                &format!("{pointer}/refinement"),
            )?
            .into();
            let internal = array(
                required(row, "internal", &pointer)?,
                &format!("{pointer}/internal"),
            )?
            .enumerate()
            .map(|(index, value)| {
                string(value, &format!("{pointer}/internal/{index}")).map(Into::into)
            })
            .collect::<Result<Vec<_>, _>>()?;
            Ok(CheckedRefinementInternal {
                refinement,
                internal,
            })
        })
        .collect()
}

fn required<'document>(
    value: NodeRef<'document>,
    key: &str,
    pointer: &str,
) -> Result<NodeRef<'document>, ReplayRequestRefusal> {
    if !matches!(value.node(), Node::Object(_)) {
        return Err(field_refusal(pointer, PackageMemberCause::WrongKind));
    }
    member(value, key)
        .ok_or_else(|| field_refusal(&format!("{pointer}/{key}"), PackageMemberCause::Missing))
}

fn string<'document>(
    value: NodeRef<'document>,
    pointer: &str,
) -> Result<&'document str, ReplayRequestRefusal> {
    match value.node() {
        Node::String(text) => Ok(text),
        _ => Err(field_refusal(pointer, PackageMemberCause::WrongKind)),
    }
}

fn array<'document>(
    value: NodeRef<'document>,
    pointer: &str,
) -> Result<quire_canonical::Items<'document>, ReplayRequestRefusal> {
    match value.node() {
        Node::Array(values) => Ok(values),
        _ => Err(field_refusal(pointer, PackageMemberCause::WrongKind)),
    }
}

fn field_refusal(pointer: &str, cause: PackageMemberCause) -> ReplayRequestRefusal {
    ReplayRequestRefusal::PackageMember {
        pointer: pointer.to_owned(),
        cause,
    }
}

/// The native refusal or resource stop while admitting a replay's package.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ReplayPackageAdmissionRefusal {
    /// The native reader rejected the whole package, retaining all native data.
    #[error("replay package refused at {pointer}: {refusal}", pointer = package_pointer(.refusal.loci.first().and_then(|locus| locus.path.as_ref()).map(|path| path.as_str())), refusal = .refusal)]
    Refused {
        /// The complete native refusal.
        refusal: Box<CheckedPackageRefusal>,
        /// The native serde-owned cause spelling, retained without a copied catalog.
        cause: Option<Box<str>>,
    },
    /// The native cause no longer has its published unit-variant wire shape.
    #[error("native checked-package cause is not its catalogued string: {refusal:?}")]
    Catalog {
        /// The original refusal, never replaced by a fabricated native failure.
        refusal: Box<CheckedPackageRefusal>,
    },
    /// The native reader exhausted a caller-selected package-read ceiling.
    #[error("replay package read exhausted {:?} at {} (bound {}, consumed {})", .0.limit_kind, package_pointer(.0.path.as_ref().map(|path| path.as_str())), .0.limit, .0.consumed)]
    Incomplete(CheckedPackageIncomplete),
    /// The native reader's operation catalog is unreadable; nothing was judged.
    #[error("replay package reader catalog unavailable: {0}")]
    CatalogUnavailable(quire_contract_model::CatalogReadError),
}

impl ReplayPackageAdmissionRefusal {
    fn from_native(refusal: CheckedPackageRefusal) -> Self {
        let cause = match refusal.cause {
            None => None,
            Some(cause) => match serde_json::to_value(cause) {
                Ok(serde_json::Value::String(spelling)) => Some(spelling.into_boxed_str()),
                _ => {
                    return Self::Catalog {
                        refusal: Box::new(refusal),
                    }
                }
            },
        };
        Self::Refused {
            refusal: Box::new(refusal),
            cause,
        }
    }

    /// The QSL catalog mapping of the native reader's typed result.
    #[must_use]
    pub fn code(&self) -> qsl_foundation::Code {
        match self {
            Self::Refused { refusal, .. } => {
                qsl_package::checked_package_refusal_code(refusal.code)
            }
            Self::Catalog { .. } | Self::CatalogUnavailable(_) => {
                qsl_foundation::Code::RuntimeInvariant
            }
            Self::Incomplete(_) => qsl_foundation::Code::StageLimitExceeded,
        }
    }

    /// The native reader's cause, without rewriting its vocabulary.
    #[must_use]
    pub fn cause(&self) -> Option<&str> {
        match self {
            Self::Refused { cause, .. } => cause.as_deref(),
            Self::Catalog { .. } | Self::CatalogUnavailable(_) | Self::Incomplete(_) => None,
        }
    }

    /// The primary native pointer prefixed with the request's package member.
    #[must_use]
    pub fn pointer(&self) -> String {
        let path = match self {
            Self::Refused { refusal, .. } | Self::Catalog { refusal } => {
                refusal.loci.first().and_then(|locus| locus.path.as_ref())
            }
            Self::Incomplete(limit) => limit.path.as_ref(),
            Self::CatalogUnavailable(_) => None,
        };
        package_pointer(path.map(|path| path.as_str()))
    }
}

pub(super) fn member<'document>(
    value: NodeRef<'document>,
    name: &str,
) -> Option<NodeRef<'document>> {
    let Node::Object(members) = value.node() else {
        return None;
    };
    members
        .into_iter()
        .find(|(key, _)| *key == name)
        .map(|(_, value)| value)
}

fn package_pointer(native: Option<&str>) -> String {
    format!("/package{}", native.unwrap_or_default())
}
