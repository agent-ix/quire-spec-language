// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-058: the current-head lane's representative cross-repository contract.
//!
//! Each of the four repositories' real, public API is called at least once,
//! tied together where practical by data one call produces feeding the next
//! (QSL's real parse digest feeds quire-contract-runtime's identity type).
//! This is deliberately proportionate, not a full checked-package v2 round
//! trip: quire-contract-codegen's generation/coverage API and
//! quire-contract-runtime's accounting/verdict API are both constructed only
//! from their own crates' internal producers (a real `GeneratedBoundOracles`
//! or a real LLVM export, and a real evaluated `Verdict`, respectively), not
//! from an external caller supplying synthetic data -- by design, per their
//! own doc comments ("construction is restricted to successful ... probe
//! observation"). Exercising that deeper surface needs QSL's own job/model
//! fixture machinery (`tests/support/`), which is internal to QSL's own test
//! tree; wiring it through this separate, external lane crate is out of this
//! change's proportional scope. This test instead exercises each crate's
//! simplest genuinely public, unrestricted surface, which still fails
//! immediately if that surface's shape changes incompatibly at head.

use ix_trace_rs::trace;
use quire_contract_codegen::BOUND_COVERAGE_SCHEMA;
use quire_contract_ir::AnchorName;
use quire_contract_runtime::{ContractIdentity, RequirementId, RevisionId};
use qsl_foundation::SourceIdentity;
use quire_spec_language::{parse, Limits};

const NATIVE_SOURCE: &str = r#"language "ix:native" edition "0-draft";
profile "state-finite/0-draft";
model Config = "test/config" version "1" digest "unresolved";
invariant AlwaysTrue on Config::Version at current {
  self.number >= 0
}
"#;

/// QSL's own real, public entry point: parse a small native unit and read
/// back its source digest. No test-support fixture machinery is needed.
fn qsl_parse_digest() -> String {
    let unit = parse(
        SourceIdentity {
            identity: "integration:current-head".into(),
            revision: "1".into(),
        },
        "current-head-contract.native",
        NATIVE_SOURCE.as_bytes(),
        Limits::default(),
    )
    .expect("the fixture native source parses under a stable, unchanged grammar");
    unit.source().digest().to_string()
}

#[trace("IT-013-SC-01")]
#[test]
fn qsl_parses_at_head() {
    let digest = qsl_parse_digest();
    assert!(
        !digest.is_empty(),
        "a parsed unit always has a source digest"
    );
}

#[trace("IT-013-SC-01")]
#[test]
fn quire_contract_ir_anchor_name_round_trips_at_head() {
    let anchor = AnchorName::new("current_head_contract")
        .expect("AnchorName::new must still admit a plain lower_snake_case identifier at head");
    assert_eq!(anchor.as_str(), "current_head_contract");
}

/// Ties QSL's real parse output into quire-contract-runtime's identity type:
/// a change to either `RevisionId`'s constructor or its borrowed-string
/// contract would fail this call, not only an isolated RT-only unit test.
#[trace("IT-013-SC-01", "IT-013-SC-02")]
#[test]
fn quire_spec_language_digest_feeds_quire_contract_runtime_identity_at_head() {
    let digest = qsl_parse_digest();
    let identity = ContractIdentity::new(RequirementId::new("FR-058"), RevisionId::new(&digest));
    assert_eq!(identity.requirement.as_str(), "FR-058");
    assert_eq!(identity.revision.as_str(), digest);
}

/// quire-contract-codegen's published bound-coverage schema constant must
/// still parse as valid JSON and as a valid Draft 2020-12 schema at head --
/// the same structural check QSL's own
/// `tests/package_construction_cases/schema.rs` runs over its own schemas.
#[trace("IT-013-SC-03")]
#[test]
fn quire_contract_codegen_bound_coverage_schema_is_valid_draft202012_at_head() {
    let schema: serde_json::Value =
        serde_json::from_str(BOUND_COVERAGE_SCHEMA).expect("the published schema is valid JSON");
    jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .compile(&schema)
        .expect("the published bound-coverage schema compiles as Draft 2020-12 at head");
}

/// Returns the text of one `[[package]]` stanza (from its `name = "..."`
/// line up to, but not including, the next stanza or end of file), for
/// asserting on that one package's fields without a general TOML parser --
/// the same proportionate parsing `tools/arch-lint/duplicate_revisions.rs`
/// uses over this same file shape.
fn lockfile_stanza<'a>(lockfile: &'a str, package_name: &str) -> &'a str {
    let marker = format!("name = \"{package_name}\"");
    let start = lockfile
        .find(&marker)
        .unwrap_or_else(|| panic!("no [[package]] stanza named {package_name:?} in Cargo.lock"));
    let rest = &lockfile[start..];
    match rest.find("\n[[package]]") {
        Some(end) => &rest[..end],
        None => rest,
    }
}

/// IT-013-SC-04 (#249 review, LOW: this criterion previously had no tracing
/// tag and no automated check, only the doc's own "inspect the resolved
/// graph" instruction): read this lane's own, just-resolved `Cargo.lock`
/// text directly and confirm none of the three backend crates' packages
/// resolved from a published registry release. quire-contract-codegen
/// resolves from its git default branch (`source = "git+..."`);
/// quire-contract-ir, its workspace member quire-contract-model, and
/// quire-contract-runtime resolve through this lane's `[patch]` table to a
/// local clone, so their stanzas carry no `source` field at all (a
/// path dependency). Either shape is a real head resolution; only a
/// `"registry+https://github.com/rust-lang/crates.io-index"` source would be
/// a silent fallback to a released version, and this test's real gate is
/// that none of the four stanzas ever carries one.
#[trace("IT-013-SC-04")]
#[test]
fn backend_crates_resolve_from_head_not_a_registry_release() {
    let lockfile_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock");
    let lockfile = std::fs::read_to_string(&lockfile_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", lockfile_path.display()));
    for package in [
        "quire-contract-ir",
        "quire-contract-model",
        "quire-contract-runtime",
        "quire-contract-codegen",
    ] {
        let stanza = lockfile_stanza(&lockfile, package);
        assert!(
            !stanza.contains("source = \"registry+"),
            "{package} must not resolve from a registry release at head; stanza was:\n{stanza}"
        );
    }
}
