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
//! `reading_the_real_architecture_bundle_refuses_every_node_on_fcd_199_gaps`
//! carries the same lifted, admitted bytes into `intake::read_records` and
//! asserts the exact, complete, sorted refusal list this real bundle
//! produces: FCD's real wire never matches QSpec's own identity form (FCD
//! #199 gap 2), so every one of the fixture's 15 declared types refuses.
//!
//! `a_qspec_conformant_document_admits_reads_and_classifies` is the
//! complementary positive case: a hand-written Semantic IR 2.0.0 document
//! using QSpec's own identity form throughout (not FCD's), no `relationships[]`
//! and no non-empty `frame` (the shapes FCD #199 gaps 2/3/4 bar), run through
//! the full pipeline -- `admit` -> `read_records` -> `DomainPackage::new` ->
//! `classify` -> `check_connection`/`check_allocation` -- asserting a wholly
//! successful outcome.

use std::collections::BTreeMap;
use std::path::PathBuf;

use quire_spec_language::model::accounting::{Meter, ModelNormalizationLimits};
use quire_spec_language::model::domain_package::{DomainPackage, ModelSelection};
use quire_spec_language::model::intake::{admit, lift_document, read_records};
use quire_spec_language::model::key::{DeclarationKey, SHA256_JCS_DIGEST_DOMAIN};
use quire_spec_language::model::systems::{
    check_allocation, check_connection, classify, AllocationCheckOutcome, ConnectionCheckOutcome,
    ConnectionOutcome,
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

    let selection = ModelSelection {
        identity: identity.clone(),
        version: version.clone(),
        digest_domain: SHA256_JCS_DIGEST_DOMAIN.to_owned(),
        digest: actual_digest,
    };
    let (admitted, admitted_bytes) = admit(&selection, &bytes_by_digest).expect(
        "a selection matching the lifted package's own identity/version/digest is admitted",
    );
    assert_eq!(admitted.identity, identity);
    assert_eq!(admitted.version, version);
    assert_eq!(admitted_bytes, document);
}

/// (a): the real vendored architecture bundle, read through the full FR-154
/// pipeline, asserts the exact, complete, sorted refusal list -- documenting
/// FCD #199's gaps concretely rather than a value this test invents. FCD's
/// own wire uses its own identity form (`ix://<pkg>/type/<id>`,
/// `ix://<pkg>/field/<Owner>-<name>`), not QSpec's (`ix://<pkg>/<id>`,
/// `<owner>/<name>`) -- gap 2 -- so every node in this real bundle refuses
/// on identity form (M4) before any other check runs.
#[test]
fn reading_the_real_architecture_bundle_refuses_every_node_on_fcd_199_gaps() {
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
    let selection = ModelSelection {
        identity: identity.clone(),
        version,
        digest_domain: SHA256_JCS_DIGEST_DOMAIN.to_owned(),
        digest,
    };
    let (_admitted, admitted_bytes) = admit(&selection, &bytes_by_digest).expect(
        "a selection matching the lifted package's own identity/version/digest is admitted",
    );

    let refusals = read_records(&identity, admitted_bytes)
        .expect_err("FCD's real wire never matches QSpec's own identity form (FCD #199 gap 2)");

    // The exact, complete, sorted refusal list: every one of the fixture's
    // 15 declared types, ascending by its own FCD identity (the ordering
    // this reader sorts by, absent any QSpec-conformant declaration key to
    // sort by instead), each refusing on the identity-form gap alone.
    let refused_nodes: Vec<&str> = refusals
        .iter()
        .map(|refusal| {
            match &refusal.cause {
            quire_spec_language::model::normalize::ModelRefusalCause::IntakeMalformedDeclaration {
                node,
                ..
            } => node.as_str(),
            other => panic!("expected IntakeMalformedDeclaration, got {other:?}"),
        }
        })
        .collect();
    assert_eq!(
        refused_nodes,
        vec![
            "ix://agent-ix/architecture/type/Count",
            "ix://agent-ix/architecture/type/CountValue",
            "ix://agent-ix/architecture/type/Flow",
            "ix://agent-ix/architecture/type/Flow2",
            "ix://agent-ix/architecture/type/Integer",
            "ix://agent-ix/architecture/type/Pump",
            "ix://agent-ix/architecture/type/Sys",
            "ix://agent-ix/architecture/type/Tank",
            "ix://agent-ix/architecture/type/UUID",
            "ix://agent-ix/architecture/type/pipe",
            "ix://agent-ix/architecture/type/pump_alloc",
            "ix://agent-ix/architecture/type/pump_out",
            "ix://agent-ix/architecture/type/sys_pump",
            "ix://agent-ix/architecture/type/sys_tank",
            "ix://agent-ix/architecture/type/tank_in",
        ],
        "the fixture's own 15 declared types, ascending by FCD's own identity string, \
         each refused solely because FCD's identity form (`ix://<pkg>/type/<id>`) is not \
         QSpec's own (`ix://<pkg>/<id>`) -- FCD #199 gap 2"
    );
    assert!(
        refusals
            .iter()
            .all(|refusal| refusal.cause.as_str() == "malformed-declaration"),
        "every refusal in this bundle is the identity-form gap, not a different defect: {refusals:#?}"
    );
}

fn multiplicity_one(lower: u64, upper: Option<u64>) -> serde_json::Value {
    serde_json::json!({
        "lower": lower,
        "upper": upper,
        "ordered": false,
        "unique": true,
    })
}

fn unlimited_meter() -> Meter {
    Meter::new(ModelNormalizationLimits::UNLIMITED)
}

fn key(package: &str, identity: &str) -> DeclarationKey {
    DeclarationKey {
        package: package.to_owned(),
        node: identity.to_owned(),
    }
}

/// (b): a hand-written Semantic IR 2.0.0 document using QSpec's own identity
/// form throughout -- `ix://<package identity>/<artifact id>` for every
/// type-definition node, never FCD's own `ix://<pkg>/type/<id>` form -- with
/// no `relationships[]` and no non-empty operation `frame` (FCD #199 gaps 2,
/// 3 and 4). This crate has no FCD schema validator wired in yet (M5,
/// deferred); this document is instead hand-checked against
/// `crates/semantic-ir/src/schema.rs`'s own `TYPE_MEMBERS`/`CONNECTION_END_MEMBERS`/
/// `MULTIPLICITY_MEMBERS` wire shapes (the same ground truth `crate::model::intake`
/// itself reads against), field by field, in this function's construction below.
#[test]
fn a_qspec_conformant_document_admits_reads_and_classifies() {
    let package_identity = "test/plant";

    let flow_type = format!("ix://{package_identity}/Flow");
    let sys_pump = format!("ix://{package_identity}/SysPump");
    let sys_tank = format!("ix://{package_identity}/SysTank");
    let pump_out = format!("ix://{package_identity}/PumpOut");
    let tank_in = format!("ix://{package_identity}/TankIn");
    let pipe = format!("ix://{package_identity}/Pipe");
    let pump_alloc = format!("ix://{package_identity}/PumpAlloc");

    let document = serde_json::json!({
        "package": {"identity": package_identity, "version": "1"},
        "constructs": [
            {
                "kind": {"module": package_identity, "name": "object-type"},
                "moduleVersion": "1.0.0",
                "manifestDigest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
                "construct": {"meaning": "quire.meaning.model.object-type/v1"},
            },
            {
                "kind": {"module": package_identity, "name": "part"},
                "moduleVersion": "1.0.0",
                "manifestDigest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
                "construct": {"meaning": "quire.meaning.systems.part/v1"},
            },
            {
                "kind": {"module": package_identity, "name": "port"},
                "moduleVersion": "1.0.0",
                "manifestDigest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
                "construct": {"meaning": "quire.meaning.systems.port/v1"},
            },
            {
                "kind": {"module": package_identity, "name": "connection"},
                "moduleVersion": "1.0.0",
                "manifestDigest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
                "construct": {"meaning": "quire.meaning.systems.connection/v1"},
            },
            {
                "kind": {"module": package_identity, "name": "allocation"},
                "moduleVersion": "1.0.0",
                "manifestDigest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
                "construct": {"meaning": "quire.meaning.systems.allocation/v1"},
            },
        ],
        "types": [
            {
                "identity": flow_type.clone(),
                "kind": {"module": package_identity, "name": "object-type"},
                "supertypes": [],
                "fields": [],
                "operations": [],
            },
            {
                "identity": sys_pump.clone(),
                "kind": {"module": package_identity, "name": "part"},
                "owner": format!("ix://{package_identity}/Sys"),
                "declaredType": format!("ix://{package_identity}/Pump"),
                "multiplicity": multiplicity_one(1, Some(1)),
            },
            {
                "identity": sys_tank.clone(),
                "kind": {"module": package_identity, "name": "part"},
                "owner": format!("ix://{package_identity}/Sys"),
                "declaredType": format!("ix://{package_identity}/Tank"),
                "multiplicity": multiplicity_one(1, Some(1)),
            },
            {
                "identity": pump_out.clone(),
                "kind": {"module": package_identity, "name": "port"},
                "owner": sys_pump.clone(),
                "interfaceType": flow_type.clone(),
                "direction": "out",
                "multiplicity": multiplicity_one(1, Some(1)),
            },
            {
                "identity": tank_in.clone(),
                "kind": {"module": package_identity, "name": "port"},
                "owner": sys_tank,
                "interfaceType": flow_type,
                "direction": "in",
                "multiplicity": multiplicity_one(1, Some(1)),
            },
            {
                "identity": pipe.clone(),
                "kind": {"module": package_identity, "name": "connection"},
                "sourceEnd": {"type": pump_out.clone(), "multiplicity": multiplicity_one(1, Some(1))},
                "targetEnd": {"type": tank_in, "multiplicity": multiplicity_one(1, Some(1))},
                "flowDirection": "source-to-target",
            },
            {
                "identity": pump_alloc.clone(),
                "kind": {"module": package_identity, "name": "allocation"},
                "sourceElement": pump_out,
                "targetElement": sys_pump,
            },
        ],
    })
    .to_string()
    .into_bytes();

    // JCS bytes: `serde_json::Map` is a `BTreeMap` here (no `preserve_order`
    // feature), so `to_string`'s member order is already sorted ascending —
    // the same RFC 8785 JCS shape `crate::model::key::jcs_bytes` produces.
    let digest: [u8; 32] = Sha256::digest(&document).into();
    let mut bytes_by_digest = BTreeMap::new();
    bytes_by_digest.insert(digest, document.clone());
    let selection = ModelSelection {
        identity: package_identity.to_owned(),
        version: "1".to_owned(),
        digest_domain: SHA256_JCS_DIGEST_DOMAIN.to_owned(),
        digest,
    };
    let (package_ref, admitted_bytes) =
        admit(&selection, &bytes_by_digest).expect("a matching selection admits");

    let records = read_records(package_identity, admitted_bytes)
        .expect("a QSpec-conformant document reads with no refusals");
    let domain_package = DomainPackage::new(package_ref, records);

    let mut meter = unlimited_meter();
    let classification =
        classify(&domain_package, &mut meter).expect("an unlimited meter never runs out");
    assert_eq!(
        classification.refusals,
        Vec::new(),
        "every systems-model node in this document supplies its required capability"
    );

    let pipe_key = key(package_identity, &pipe);
    let pump_alloc_key = key(package_identity, &pump_alloc);

    let mut meter = unlimited_meter();
    let connection_outcome =
        check_connection(&domain_package, &classification, &pipe_key, &mut meter);
    assert_eq!(
        connection_outcome,
        ConnectionCheckOutcome::Completed(ConnectionOutcome::Admitted),
        "PumpOut (out, Flow) to TankIn (in, Flow), 1..1 to 1..1, is a wholly admitted connection"
    );

    let mut meter = unlimited_meter();
    let allocation_outcome = check_allocation(&classification, &pump_alloc_key, &mut meter);
    assert_eq!(
        allocation_outcome,
        AllocationCheckOutcome::Admitted,
        "PumpAlloc's target SysPump classifies as a Part"
    );
}
