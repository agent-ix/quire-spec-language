// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-042/TC-121 and FR-050/TC-138: the published compiled-protocol handoff.
//!
//! An independent consumer reads a handoff directory: the offered artifact
//! bytes, an `expected-v2.json` reader selection, and a `mutations/manifest.json`
//! adverse corpus. These are the record shapes of those two files. They are the
//! interchange contract, so every consumer decodes the published types here
//! instead of re-declaring its own copy and drifting from the producer.
//!
//! These records serialize a caller's independent selections. They are inert
//! data: decoding one establishes no admission, and a consumer still supplies
//! the borrowed `Expected`/`ExpectedV2` inputs to the reader itself.

use serde::{Deserialize, Serialize};

use super::{v2, wire as w, Dimension, Limits, ACCOUNTING_VERSION};

/// Repository path of the committed compiled-protocol v2 consumer handoff.
pub const PUBLISHED_HANDOFF: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/artifacts/compiled-protocol-v2"
);

/// Handoff-relative file containing the canonical version-2 package bytes.
pub const PUBLISHED_OFFER_FILE: &str = "compiled-protocol-v2.json";

/// Handoff-relative file containing the independently authorized artifact reference.
pub const PUBLISHED_ARTIFACT_REFERENCE_FILE: &str = "compiled-protocol-v2.ref.json";

/// Handoff-relative file containing the independent version-2 reader selection.
pub const PUBLISHED_SELECTION_FILE: &str = "expected-v2.json";

/// Handoff-relative file containing the adverse mutation inventory.
pub const PUBLISHED_MUTATION_MANIFEST_FILE: &str = "mutations/manifest.json";

/// Handoff-relative file containing raw SHA-256 digests for the published inventory.
pub const PUBLISHED_CHECKSUMS_FILE: &str = "SHA256SUMS";

/// Format identity of the published version-2 mutation corpus.
pub const MUTATION_MANIFEST_FORMAT: &str = "quire.protocol.v2-mutations/1";

/// One selected dependency, naming the file holding its exact original bytes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedDependency {
    /// Exact dependency artifact reference.
    pub artifact: w::ArtifactRef,
    /// Handoff-relative file holding the dependency's original bytes.
    pub file: String,
    /// Complete selected closure this dependency itself requires.
    pub requires: Vec<w::ArtifactRef>,
}

/// One independently selected authored declaration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedDeclaration {
    /// Authored declaration name.
    pub name: String,
    /// Original source region owning the declaration.
    pub span: w::Span,
    /// Formal requirement owner.
    pub requirement: w::Requirement,
    /// Formal clause identity.
    pub clause: String,
    /// Authored execution point.
    pub execution: w::Execution,
}

/// One selected original source and the declarations the caller expects in it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedSource {
    /// Handoff-relative file holding the source's exact original bytes.
    pub file: String,
    /// Exact source identity, revision and raw-byte digest.
    pub source: w::Source,
    /// Complete expected declaration inventory for this source.
    pub declarations: Vec<SelectedDeclaration>,
}

/// The selected admitted model and the original bytes it was admitted from.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedModel {
    /// Exact model artifact reference.
    pub artifact: w::ArtifactRef,
    /// Exact model source identity, revision and raw-byte digest.
    pub source: w::Source,
    /// Handoff-relative file holding the model's original bytes.
    pub source_file: String,
    /// Original model source format label.
    pub source_format: String,
}

/// The serialized fields of one independent version-1 reader `Expected` input.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    /// Exact offered compiled artifact reference, including its external seal.
    pub artifact: w::ArtifactRef,
    /// Exact selected compiler/consumer contract dependency.
    pub contract: w::ArtifactRef,
    /// Exact accepted semantic baseline dependency.
    pub baseline: w::ArtifactRef,
    /// Explicit implementation, revision and immutable binary selection.
    pub producer: w::Producer,
    /// Accepted native language edition.
    pub language: w::Language,
    /// Complete original source inventory.
    pub sources: Vec<SelectedSource>,
    /// Complete exact-byte dependency inventory.
    pub dependencies: Vec<SelectedDependency>,
    /// The selected admitted model.
    pub model: SelectedModel,
}

/// The independently selected reader ceilings, with their accounting contract.
///
/// The version is part of the record: two runs are only comparable when they
/// charged the same counters.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedArtifactLimits {
    /// Counter contract identity these ceilings belong to.
    pub accounting_version: String,
    /// Complete offered bytes.
    pub payload_bytes: u64,
    /// Complete encoded bytes.
    pub output_bytes: u64,
    /// Each native or foreign original source.
    pub source_bytes: u64,
    /// Total decoded string UTF-8 bytes.
    pub content_bytes: u64,
    /// Source entries in either complete inventory.
    pub sources: u64,
    /// Dependency entries in either complete inventory.
    pub dependencies: u64,
    /// Selected definition entries.
    pub definitions: u64,
    /// Selected admitted-model entries.
    pub models: u64,
    /// Complete declaration entries.
    pub declarations: u64,
    /// Allocated table/member/index records.
    pub entries: u64,
    /// Subsequent reference, graph-edge and type visits.
    pub references: u64,
    /// Cumulative traversed, copied, hashed or compared bytes.
    pub byte_work: u64,
    /// Maximum JSON or graph traversal depth.
    pub depth: u64,
}

/// An effective reader ceiling could not be represented in the fixed-width
/// handoff record. Current hard ceilings are all below `u64::MAX`; retaining the
/// failure keeps that invariant checked at the interchange boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("compiled protocol {dimension:?} limit {value} exceeds the handoff u64 domain")]
pub struct LimitWidthError {
    /// Counter whose effective ceiling could not be represented.
    pub dimension: Dimension,
    /// Effective platform-width ceiling that was refused.
    pub value: usize,
}

impl SelectedArtifactLimits {
    /// Record the effective ceilings of one reader invocation.
    pub fn try_new(limits: Limits) -> Result<Self, LimitWidthError> {
        let limits = limits.bounded();
        macro_rules! fixed {
            ($field:ident, $dimension:expr) => {
                u64::try_from(limits.$field).map_err(|_| LimitWidthError {
                    dimension: $dimension,
                    value: limits.$field,
                })?
            };
        }
        Ok(Self {
            accounting_version: ACCOUNTING_VERSION.to_owned(),
            payload_bytes: fixed!(payload_bytes, Dimension::PayloadBytes),
            output_bytes: fixed!(output_bytes, Dimension::OutputBytes),
            source_bytes: fixed!(source_bytes, Dimension::SourceBytes),
            content_bytes: fixed!(content_bytes, Dimension::ContentBytes),
            sources: fixed!(sources, Dimension::Sources),
            dependencies: fixed!(dependencies, Dimension::Dependencies),
            definitions: fixed!(definitions, Dimension::Definitions),
            models: fixed!(models, Dimension::Models),
            declarations: fixed!(declarations, Dimension::Declarations),
            entries: fixed!(entries, Dimension::Entries),
            references: fixed!(references, Dimension::References),
            byte_work: fixed!(byte_work, Dimension::ByteWork),
            depth: fixed!(depth, Dimension::Depth),
        })
    }
}

/// One selected clock input and the file holding its exact original bytes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedClockInput {
    /// Clock-input identity.
    pub identity: String,
    /// Raw-byte digest of the clock input's original bytes.
    pub digest: String,
    /// Handoff-relative file holding those bytes.
    pub file: String,
    /// Typed clock configuration the consumer expects.
    pub configuration: v2::wire::ClockConfiguration,
}

/// One independently selected version-2 temporal expectation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedTemporal {
    /// Exact source artifact owning the declaration.
    pub source: w::ArtifactRef,
    /// The expected authored declaration.
    pub declaration: SelectedDeclaration,
    /// Registered definition identity.
    pub definition_identity: String,
    /// Registered semantic revision.
    pub definition_revision: w::Revision,
    /// Original definition artifact reference.
    pub definition_artifact: w::ArtifactRef,
    /// Expected clock input.
    pub clock_input: SelectedClockInput,
}

/// The serialized fields of one independent version-2 reader `ExpectedV2` input.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectionV2 {
    /// The inherited version-1 selection.
    pub inherited: Selection,
    /// Complete expected temporal table; admission never builds it from the offer.
    pub temporal: Vec<SelectedTemporal>,
    /// Ceilings the producer used, with their accounting contract.
    pub limits: SelectedArtifactLimits,
}

/// The machine-consumable adverse corpus published beside one handoff.
///
/// Every case names the exact refusal identity the consumer must observe. A
/// case that admits, or that refuses under a different code, is a failure.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MutationManifest {
    /// Corpus format identity.
    pub format: String,
    /// Handoff-relative unmutated control offer.
    pub base_offer: String,
    /// Handoff-relative base artifact reference file.
    pub base_artifact: String,
    /// Handoff-relative independent reader selection.
    pub independent_selection: String,
    /// Complete mutation inventory.
    pub cases: Vec<MutationCase>,
}

/// One adverse case and the refusal identity it must produce.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MutationCase {
    /// Stable case identity.
    pub identity: String,
    /// Selection axis this case mutates.
    pub axis: String,
    /// The mutation to apply.
    pub input: MutationInput,
    /// Exact expected refusal code, as returned by `Error::code`.
    pub expected_refusal_code: String,
}

/// The operation one mutation case applies to the control inputs.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "operation", rename_all = "snake_case")]
pub enum MutationInput {
    /// Offer mutated artifact bytes against the stated artifact reference.
    Offer {
        /// Handoff-relative file holding the mutated offer.
        file: String,
        /// Artifact reference to offer those bytes under.
        artifact: w::ArtifactRef,
    },
    /// Replace one named original input with mutated bytes, then offer the control.
    ReplaceOriginal {
        /// Handoff-relative original input the case replaces.
        target: String,
        /// Handoff-relative file holding the replacement bytes.
        file: String,
    },
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use ix_trace_rs::trace;
    use serde_json::json;

    use super::{
        MutationInput, MutationManifest, SelectedArtifactLimits, MUTATION_MANIFEST_FORMAT,
        PUBLISHED_ARTIFACT_REFERENCE_FILE, PUBLISHED_CHECKSUMS_FILE, PUBLISHED_HANDOFF,
        PUBLISHED_MUTATION_MANIFEST_FILE, PUBLISHED_OFFER_FILE, PUBLISHED_SELECTION_FILE,
    };
    use crate::protocol_artifact::{Limits, ACCOUNTING_VERSION};
    use crate::ByteDigest;

    #[test]
    #[trace("TC-121", "FR-042-AC-10")]
    fn published_handoff_path_exists_and_checksums_verify() {
        let root = Path::new(PUBLISHED_HANDOFF);
        assert!(root.is_dir(), "published handoff directory: {root:?}");
        let sums =
            fs::read_to_string(root.join(PUBLISHED_CHECKSUMS_FILE)).expect("published SHA256SUMS");
        assert!(!sums.is_empty(), "published SHA256SUMS is not empty");

        for (line_index, line) in sums.lines().enumerate() {
            let (expected, relative) = line
                .split_once("  ./")
                .unwrap_or_else(|| panic!("malformed SHA256SUMS line {}", line_index + 1));
            let bytes = fs::read(root.join(relative)).unwrap_or_else(|error| {
                panic!("read published handoff file {relative:?}: {error}")
            });
            assert_eq!(
                format!("{:x}", ByteDigest::of(&bytes)),
                expected,
                "published handoff digest for {relative}"
            );
        }
    }

    #[test]
    #[trace("TC-138", "FR-050-AC-7")]
    fn published_handoff_addresses_resolve_the_owned_inventory() {
        let root = Path::new(PUBLISHED_HANDOFF);
        for relative in [
            PUBLISHED_OFFER_FILE,
            PUBLISHED_ARTIFACT_REFERENCE_FILE,
            PUBLISHED_SELECTION_FILE,
            PUBLISHED_MUTATION_MANIFEST_FILE,
            PUBLISHED_CHECKSUMS_FILE,
        ] {
            assert!(root.join(relative).is_file(), "published member {relative}");
        }

        let manifest: MutationManifest = serde_json::from_slice(
            &fs::read(root.join(PUBLISHED_MUTATION_MANIFEST_FILE))
                .expect("published mutation manifest"),
        )
        .expect("decode published mutation manifest");
        assert_eq!(manifest.format, MUTATION_MANIFEST_FORMAT);
        assert_eq!(manifest.base_offer, PUBLISHED_OFFER_FILE);
        assert_eq!(manifest.base_artifact, PUBLISHED_ARTIFACT_REFERENCE_FILE);
        assert_eq!(manifest.independent_selection, PUBLISHED_SELECTION_FILE);
    }

    #[test]
    #[trace("TC-121", "FR-042-AC-9")]
    fn selected_limits_are_effective_fixed_width_and_closed() {
        let raised = Limits {
            payload_bytes: usize::MAX,
            ..Limits::default()
        };
        let limits = SelectedArtifactLimits::try_new(raised).expect("bounded limits fit u64");
        assert_eq!(limits.accounting_version, ACCOUNTING_VERSION);
        assert_eq!(limits.payload_bytes, 8 * 1_048_576);

        let mut encoded = serde_json::to_value(&limits).expect("serialize published limits");
        encoded
            .as_object_mut()
            .expect("record shape")
            .insert("unknown".into(), json!(0));
        assert!(serde_json::from_value::<SelectedArtifactLimits>(encoded).is_err());
    }

    #[test]
    #[trace("TC-138", "FR-050-AC-4")]
    fn mutation_envelope_and_inputs_are_closed_records() {
        let manifest = MutationManifest {
            format: MUTATION_MANIFEST_FORMAT.into(),
            base_offer: PUBLISHED_OFFER_FILE.into(),
            base_artifact: PUBLISHED_ARTIFACT_REFERENCE_FILE.into(),
            independent_selection: PUBLISHED_SELECTION_FILE.into(),
            cases: Vec::new(),
        };
        let mut encoded = serde_json::to_value(&manifest).expect("serialize published manifest");
        encoded
            .as_object_mut()
            .expect("record shape")
            .insert("unknown".into(), json!(0));
        assert!(serde_json::from_value::<MutationManifest>(encoded).is_err());

        assert!(serde_json::from_value::<MutationInput>(json!({
            "operation": "replace_original",
            "target": "dependencies/definition.bin",
            "file": "mutations/definition.bin",
            "unknown": false
        }))
        .is_err());
    }
}
