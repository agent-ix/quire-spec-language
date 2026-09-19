// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-154 intake against FCD's real architecture fixture bundle
//! (`agent-ix/filament-core-data` at `cbbe4908e314b9cc5c7e1aaee87517218af3e632`,
//! vendored under `tests/fixtures/architecture` and `tests/fixtures/modules`).
//!
//! Proves `crate::model::intake::lift_document` drives FCD's real `lift`
//! pipeline end to end (not a QSL-authored stand-in for its output) by
//! checking the produced bytes against the fixture's own committed
//! fingerprint, then proves `crate::model::intake::admit` accepts that
//! output under a matching selection. The per-node reader into
//! `crate::model::domain_package::DomainPackageRecord` is not wired yet
//! (`#131` slice 3 keeps that small until `#131` slice 2's record reshape
//! lands), so this test stops at admitted bytes.

use std::collections::BTreeMap;
use std::path::PathBuf;

use quire_spec_language::model::intake::{admit, lift_document};
use quire_spec_language::model::key::SHA256_JCS_DIGEST_DOMAIN;
use sha2::{Digest, Sha256};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[test]
fn lifts_the_architecture_bundle_and_admits_it() {
    let fixtures = fixtures_dir();
    let bundle_root = fixtures.join("architecture");
    let module_roots = vec![
        fixtures.join("modules/spec-objects-business"),
        fixtures.join("modules/edge-vocabulary"),
        fixtures.join("modules/spec-objects-architecture"),
    ];

    let document = lift_document(&bundle_root, &module_roots)
        .expect("FCD's real architecture fixture lifts cleanly");

    // The fixture's own committed fingerprint: proof this ran FCD's actual
    // pipeline over the actual bundle, not a value this test invented.
    let fingerprint: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(bundle_root.join("expected/semantic-ir.json.fingerprint"))
            .expect("fixture fingerprint file is present"),
    )
    .expect("fixture fingerprint file is valid JSON");
    let expected_digest = fingerprint["digest"]
        .as_str()
        .expect("fingerprint has a digest field")
        .strip_prefix("sha256-jcs:")
        .expect("fingerprint digest carries the sha256-jcs prefix");

    let actual_digest: [u8; 32] = Sha256::digest(&document).into();
    let actual_digest_hex = actual_digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(
        actual_digest_hex, expected_digest,
        "lift_document's bytes must hash to the fixture's own committed fingerprint"
    );

    let package: serde_json::Value =
        serde_json::from_slice(&document).expect("lifted document is valid JSON");
    let identity = package["package"]["identity"]
        .as_str()
        .expect("document declares its package identity")
        .to_owned();
    let version = package["package"]["version"]
        .as_str()
        .expect("document declares its package version")
        .to_owned();

    let mut bytes_by_digest = BTreeMap::new();
    bytes_by_digest.insert(actual_digest, document.clone());

    let (selection, admitted_bytes) = admit(
        &identity,
        &version,
        SHA256_JCS_DIGEST_DOMAIN,
        actual_digest,
        &bytes_by_digest,
    )
    .expect("a selection matching the lifted package's own identity/version/digest is admitted");
    assert_eq!(selection.identity, identity);
    assert_eq!(selection.version, version);
    assert_eq!(admitted_bytes, document);
}
