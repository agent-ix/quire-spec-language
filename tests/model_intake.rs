// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-154 intake against FCD's real architecture fixture bundle
//! (`agent-ix/filament-core-data` at `cbbe4908e314b9cc5c7e1aaee87517218af3e632`,
//! vendored under `tests/fixtures/architecture` and `tests/fixtures/modules`).
//!
//! `lifts_the_architecture_bundle_and_admits_it` proves
//! `crate::model::intake::lift_document` drives FCD's real `lift` pipeline
//! end to end (not a QSL-authored stand-in for its output) by checking the
//! produced bytes against the fixture's own committed fingerprint, then
//! proves `crate::model::intake::admit` accepts that output under a
//! matching selection.
//!
//! `lifts_admits_reads_and_classifies_the_architecture_bundle` carries the
//! same lifted, admitted bytes the rest of the way: `intake::read_records`
//! into a real `DomainPackage`, then `crate::model::systems::classify`,
//! `check_connection` and `check_allocation` over it, checked against the
//! fixture's own `Sys`/`Pump`/`Tank`/`Flow`/`Flow2` architecture (vendored
//! under `tests/fixtures/architecture/spec`), not a value this test invents.

use std::collections::BTreeMap;
use std::path::PathBuf;

use quire_spec_language::model::accounting::{Meter, ModelNormalizationLimits};
use quire_spec_language::model::domain_package::DomainPackage;
use quire_spec_language::model::intake::{admit, lift_document, read_records};
use quire_spec_language::model::key::{DeclarationKey, SHA256_JCS_DIGEST_DOMAIN};
use quire_spec_language::model::systems::{
    check_allocation, check_connection, classify, AllocationCheckOutcome, ConnectionCheckOutcome,
    ConnectionOutcome, Kind,
};
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

fn key(package: &str, node: &str) -> DeclarationKey {
    DeclarationKey {
        package: package.to_owned(),
        node: node.to_owned(),
    }
}

fn unlimited_meter() -> Meter {
    Meter::new(ModelNormalizationLimits::UNLIMITED)
}

#[test]
fn lifts_admits_reads_and_classifies_the_architecture_bundle() {
    let fixtures = fixtures_dir();
    let bundle_root = fixtures.join("architecture");
    let module_roots = vec![
        fixtures.join("modules/spec-objects-business"),
        fixtures.join("modules/edge-vocabulary"),
        fixtures.join("modules/spec-objects-architecture"),
    ];

    let document = lift_document(&bundle_root, &module_roots)
        .expect("FCD's real architecture fixture lifts cleanly");
    let digest: [u8; 32] = Sha256::digest(&document).into();

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
    bytes_by_digest.insert(digest, document.clone());
    let (selection, admitted_bytes) = admit(
        &identity,
        &version,
        SHA256_JCS_DIGEST_DOMAIN,
        digest,
        &bytes_by_digest,
    )
    .expect("a selection matching the lifted package's own identity/version/digest is admitted");

    let records = read_records(&identity, &admitted_bytes)
        .expect("the fixture's IR nodes read into domain package records");
    let domain_package = DomainPackage::new(selection, records);

    let mut meter = unlimited_meter();
    let classification =
        classify(&domain_package, &mut meter).expect("an unlimited meter never runs out");
    assert_eq!(
        classification.refusals,
        Vec::new(),
        "every systems-model node in the fixture supplies its required capability"
    );

    let sys_pump = key(&identity, "ix://agent-ix/architecture/type/sys_pump");
    let sys_tank = key(&identity, "ix://agent-ix/architecture/type/sys_tank");
    let pump_out = key(&identity, "ix://agent-ix/architecture/type/pump_out");
    let tank_in = key(&identity, "ix://agent-ix/architecture/type/tank_in");
    let flow = key(&identity, "ix://agent-ix/architecture/type/Flow");
    let flow2 = key(&identity, "ix://agent-ix/architecture/type/Flow2");
    let pipe = key(&identity, "ix://agent-ix/architecture/type/pipe");
    let pump_alloc = key(&identity, "ix://agent-ix/architecture/type/pump_alloc");

    assert_eq!(classification.actual_kind(&sys_pump), Kind::Part);
    assert_eq!(classification.actual_kind(&sys_tank), Kind::Part);
    assert_eq!(classification.actual_kind(&pump_out), Kind::Port);
    assert_eq!(classification.actual_kind(&tank_in), Kind::Port);
    assert_eq!(classification.actual_kind(&flow), Kind::Interface);
    assert_eq!(classification.actual_kind(&flow2), Kind::Interface);
    assert_eq!(classification.actual_kind(&pipe), Kind::Connection);
    assert_eq!(classification.actual_kind(&pump_alloc), Kind::Allocation);

    let mut meter = unlimited_meter();
    let connection_outcome = check_connection(&domain_package, &classification, &pipe, &mut meter);
    assert_eq!(
        connection_outcome,
        ConnectionCheckOutcome::Completed(ConnectionOutcome::Admitted),
        "pump_out (out, Flow) to tank_in (in, Flow), 1..1 to 1..1, is a wholly admitted connection"
    );

    let mut meter = unlimited_meter();
    let allocation_outcome = check_allocation(&classification, &pump_alloc, &mut meter);
    assert_eq!(
        allocation_outcome,
        AllocationCheckOutcome::Admitted,
        "pump_alloc's target sys_pump classifies as a Part"
    );
}
