// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec's native diagnostics catalog, read by reference.
//!
//! [`native_diagnostics_catalog`] builds the `DefinitionRef` a checked
//! package's diagnostics are qualified by from the `quire_specification`
//! crate's compiled-in `native-diagnostics.md` bytes. It reads no JSON, so
//! it hashes raw bytes only (ADR-013 §2).

use std::sync::OnceLock;

use qsl_foundation::ByteDigest;

use super::definition::{DefinitionReference, DefinitionRevision};

/// QSpec's `quire.native.diagnostics/v1` catalog as the `DefinitionRef` a
/// checked package's diagnostics are qualified by. Identity and revision come
/// from the document's own header line, and the digest is the SHA-256 of the
/// `quire_specification::NATIVE_DIAGNOSTICS` bytes.
///
/// # Panics
///
/// If the compiled-in document has no `Interpretation identity: `…`;
/// revision: `…`` header; `the_native_diagnostics_catalog_reads_its_header`
/// holds it to one.
pub fn native_diagnostics_catalog() -> &'static DefinitionReference {
    static CATALOG: OnceLock<DefinitionReference> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let document = quire_specification::NATIVE_DIAGNOSTICS;
        let (identity, revision) = diagnostics_header(document)
            .expect("QSpec's native-diagnostics.md opens with its identity and revision");
        DefinitionReference {
            authority: "agent-ix".to_owned(),
            identity: identity.to_owned(),
            revision: DefinitionRevision {
                namespace: "quire-draft".to_owned(),
                value: revision.to_owned(),
            },
            digest_domain: "quire.definition.bytes/v1".to_owned(),
            digest: format!("{:x}", ByteDigest::of(document.as_bytes())),
        }
    })
}

/// The identity and revision of a diagnostics catalog document's
/// ``Interpretation identity: `X`; revision: `Y`.`` header line.
#[qsl_attrs::string_edge]
fn diagnostics_header(document: &str) -> Option<(&str, &str)> {
    let line = document
        .lines()
        .find_map(|line| line.strip_prefix("Interpretation identity: `"))?;
    let (identity, rest) = line.split_once("`; revision: `")?;
    let (revision, _) = rest.split_once('`')?;
    Some((identity, revision))
}
