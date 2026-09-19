// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-154: lift a spec-bundle to Semantic IR 2.0.0 bytes and admit them as a
//! domain package selection.
//!
//! Two independent halves. [`lift_document`] wraps
//! `agent_ix_extraction_frontend::lift::lift`, the sole entry point that
//! turns a spec bundle plus its module roots into IR 2.0.0 document bytes
//! (FCD FR-091..097). [`admit`] runs FR-154's own four-check admission
//! table (`model-complete.md:58`) over a selection and a byte map, and
//! returns the package's raw bytes for the per-node reader once it exists.
//! Neither function reads an IR node into a [`super::domain_package::DomainPackageRecord`];
//! that reader is a separate, not-yet-landed piece of this module.
#![allow(
    clippy::result_large_err,
    reason = "cold refusal path; ModelRefusalCause carries DeclarationKeys inline, matching state::evaluation's typed-failure precedent"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use agent_ix_extraction_frontend::lift::{lift, LiftOutcome, LiftRequest};
use agent_ix_extraction_frontend::{Diagnostic, Refusal};
use sha2::{Digest, Sha256};

use crate::diagnostic::Code;
use crate::model::domain_package::DomainPackageRef;
use crate::model::key::{hex, SHA256_JCS_DIGEST_DOMAIN};
use crate::model::normalize::{ModelRefusal, ModelRefusalCause};

/// A Quire meaning id (FR-208): the sole legitimate way to determine what a
/// construct or type definition IS. Kind names and modules are never
/// matched directly; every reader dispatches on this table's values.
pub mod meaning {
    pub const OBJECT_TYPE: &str = "quire.meaning.model.object-type/v1";
    pub const VALUE_TYPE: &str = "quire.meaning.model.value-type/v1";
    pub const RECORD_VALUE_TYPE: &str = "quire.meaning.model.record-value-type/v1";
    pub const VARIANT_TYPE: &str = "quire.meaning.model.variant-type/v1";
    pub const EVENT_TYPE: &str = "quire.meaning.model.event-type/v1";
    pub const STATE_MACHINE: &str = "quire.meaning.model.state-machine/v1";
    pub const PROCESS: &str = "quire.meaning.model.process/v1";
    pub const PERSISTENCE_INTERFACE: &str = "quire.meaning.model.persistence-interface/v1";
    pub const NAMESPACE: &str = "quire.meaning.model.namespace/v1";
    pub const POPULATION: &str = "quire.meaning.model.population/v1";
    pub const SYSTEMS_PART: &str = "quire.meaning.systems.part/v1";
    pub const SYSTEMS_PORT: &str = "quire.meaning.systems.port/v1";
    pub const SYSTEMS_INTERFACE: &str = "quire.meaning.systems.interface/v1";
    pub const SYSTEMS_CONNECTION: &str = "quire.meaning.systems.connection/v1";
    pub const SYSTEMS_ALLOCATION: &str = "quire.meaning.systems.allocation/v1";
}

/// FCD `agent_ix_extraction_frontend::lift` did not produce a document.
///
/// Wraps FCD's own outcome types rather than stringifying them (matching
/// `crate::command::extraction::ExtractionError`'s `Context(Vec<SemanticFailure>)`):
/// the caller's diagnostics stay FCD-owned and typed.
#[derive(Debug, thiserror::Error)]
pub enum LiftFailure {
    /// A scratch directory for `lift`'s required output path could not be
    /// created.
    #[error("could not create a scratch directory for lift output: {0}")]
    Scratch(#[source] std::io::Error),
    /// FCD refused the bundle outright (parse, manifest or module failure).
    #[error("bundle refused: {0}")]
    Refused(Refusal),
    /// FCD produced at least one blocking diagnostic (schema or rules
    /// failure); no document was written.
    #[error("{} blocking diagnostic(s)", .0.len())]
    Blocked(Vec<Diagnostic>),
}

/// Lift one spec bundle to Semantic IR 2.0.0 document bytes.
///
/// `lift` (FCD FR-099) always writes its document to a required output
/// path; the diagnostics and provenance sidecars are skipped (`None`).
/// The scratch directory is removed when this function returns.
pub fn lift_document(bundle_root: &Path, module_roots: &[PathBuf]) -> Result<Vec<u8>, LiftFailure> {
    let scratch = tempfile::tempdir().map_err(LiftFailure::Scratch)?;
    let request = LiftRequest {
        bundle_root: bundle_root.to_path_buf(),
        module_roots: module_roots.to_vec(),
        out: scratch.path().join("document.json"),
        diagnostics: None,
        provenance: None,
    };
    match lift(&request) {
        LiftOutcome::Refused(refusal) => Err(LiftFailure::Refused(refusal)),
        LiftOutcome::Blocked { diagnostics } => Err(LiftFailure::Blocked(diagnostics)),
        LiftOutcome::Written { document, .. } => Ok(document),
    }
}

/// FR-154 Intake's four-check admission table (`model-complete.md:58-70`).
///
/// Runs the checks in table order and stops at the first failure: digest
/// domain, then byte presence, then digest equality, then the package's own
/// declared identity/version. Returns the admitted selection and the
/// package's raw bytes; reading the package's IR nodes into
/// [`super::domain_package::DomainPackageRecord`]s is a separate, later step.
pub fn admit(
    identity: &str,
    version: &str,
    digest_domain: &str,
    digest: [u8; 32],
    bytes_by_digest: &BTreeMap<[u8; 32], Vec<u8>>,
) -> Result<(DomainPackageRef, Vec<u8>), ModelRefusal> {
    if digest_domain != SHA256_JCS_DIGEST_DOMAIN {
        return Err(ModelRefusal {
            code: Code::StaleDependency,
            cause: ModelRefusalCause::DigestDomainMismatch {
                expected: SHA256_JCS_DIGEST_DOMAIN,
                actual: digest_domain.to_owned(),
            },
            detail: format!(
                "domain package selection {identity}@{version} names digest domain \
                 {digest_domain:?}, not {SHA256_JCS_DIGEST_DOMAIN:?}"
            ),
        });
    }
    let selection = DomainPackageRef {
        identity: identity.to_owned(),
        version: version.to_owned(),
        digest,
    };
    let Some(bytes) = bytes_by_digest.get(&digest) else {
        return Err(ModelRefusal {
            code: Code::MissingImport,
            cause: ModelRefusalCause::MissingSelection {
                selection: selection.clone(),
            },
            detail: format!("no package bytes supplied under digest {}", hex(&digest)),
        });
    };
    let actual_digest: [u8; 32] = Sha256::digest(bytes.as_slice()).into();
    if actual_digest != digest {
        return Err(ModelRefusal {
            code: Code::StaleDependency,
            cause: ModelRefusalCause::ByteDigestMismatch {
                expected: digest,
                actual: actual_digest,
            },
            detail: format!(
                "domain package {identity}@{version} bytes hash to {}, not the selected {}",
                hex(&actual_digest),
                hex(&digest)
            ),
        });
    }
    // The package's own declared identity/version, read defensively: bytes
    // that fail to parse or omit `package` simply supply no identity/version,
    // which check 4 below reports as a `wrong-model-selection` mismatch
    // rather than a separate malformed-package case FR-154's table does not
    // name.
    let parsed: Option<serde_json::Value> = serde_json::from_slice(bytes).ok();
    let package = parsed.as_ref().and_then(|value| value.get("package"));
    let actual_identity = package
        .and_then(|value| value.get("identity"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let actual_version = package
        .and_then(|value| value.get("version"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    if actual_identity != identity || actual_version != version {
        return Err(ModelRefusal {
            code: Code::InvalidModelBinding,
            cause: ModelRefusalCause::WrongModelSelection {
                selection: selection.clone(),
                actual_identity: actual_identity.clone(),
                actual_version: actual_version.clone(),
            },
            detail: format!(
                "domain package selection names {identity}@{version} but the package \
                 declares {actual_identity}@{actual_version}"
            ),
        });
    }
    Ok((selection, bytes.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest_of(bytes: &[u8]) -> [u8; 32] {
        Sha256::digest(bytes).into()
    }

    fn package_bytes(identity: &str, version: &str) -> Vec<u8> {
        serde_json::json!({
            "contractVersion": "2.0.0",
            "package": {"identity": identity, "version": version},
        })
        .to_string()
        .into_bytes()
    }

    #[test]
    fn admits_matching_selection() {
        let bytes = package_bytes("acme/orders", "1");
        let digest = digest_of(&bytes);
        let mut map = BTreeMap::new();
        map.insert(digest, bytes.clone());
        let (selection, admitted_bytes) =
            admit("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, digest, &map).unwrap();
        assert_eq!(selection.identity, "acme/orders");
        assert_eq!(selection.version, "1");
        assert_eq!(selection.digest, digest);
        assert_eq!(admitted_bytes, bytes);
    }

    #[test]
    fn refuses_foreign_digest_domain() {
        let map = BTreeMap::new();
        let refusal = admit("acme/orders", "1", "sha1", [0; 32], &map).unwrap_err();
        assert_eq!(refusal.code, Code::StaleDependency);
        assert_eq!(refusal.cause.as_str(), "digest-domain-mismatch");
    }

    #[test]
    fn refuses_missing_bytes() {
        let map = BTreeMap::new();
        let refusal =
            admit("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, [0; 32], &map).unwrap_err();
        assert_eq!(refusal.code, Code::MissingImport);
        assert_eq!(refusal.cause.as_str(), "missing-selection");
    }

    #[test]
    fn refuses_byte_digest_mismatch() {
        let bytes = package_bytes("acme/orders", "1");
        let wrong_digest = digest_of(b"not the package");
        let mut map = BTreeMap::new();
        map.insert(wrong_digest, bytes);
        let refusal = admit(
            "acme/orders",
            "1",
            SHA256_JCS_DIGEST_DOMAIN,
            wrong_digest,
            &map,
        )
        .unwrap_err();
        assert_eq!(refusal.code, Code::StaleDependency);
        assert_eq!(refusal.cause.as_str(), "byte-digest-mismatch");
    }

    #[test]
    fn refuses_wrong_package_identity() {
        let bytes = package_bytes("acme/other", "1");
        let digest = digest_of(&bytes);
        let mut map = BTreeMap::new();
        map.insert(digest, bytes);
        let refusal =
            admit("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, digest, &map).unwrap_err();
        assert_eq!(refusal.code, Code::InvalidModelBinding);
        assert_eq!(refusal.cause.as_str(), "wrong-model-selection");
    }

    #[test]
    fn refuses_non_json_bytes_as_wrong_selection() {
        let bytes = b"not json".to_vec();
        let digest = digest_of(&bytes);
        let mut map = BTreeMap::new();
        map.insert(digest, bytes);
        let refusal =
            admit("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, digest, &map).unwrap_err();
        assert_eq!(refusal.cause.as_str(), "wrong-model-selection");
    }
}
