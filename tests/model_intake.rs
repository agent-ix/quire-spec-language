// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-154 intake against FCD's real architecture fixture bundle
//! (`agent-ix/filament-core-data`, vendored under `tests/fixtures/architecture`
//! and `tests/fixtures/modules` at the same rev `Cargo.toml` pins its
//! `agent-ix-extraction-frontend`/`agent-ix-semantic-ir` git deps to -- the
//! rev is pinned in that one place, not repeated here).
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
use quire_spec_language::model::domain_package::{DomainPackage, DomainPackageRef};
use quire_spec_language::model::intake::{admit, lift_document, meaning, read_records};
use quire_spec_language::model::key::{DeclarationKey, SHA256_JCS_DIGEST_DOMAIN};
use quire_spec_language::model::systems::{
    check_allocation, check_connection, classify, AllocationCheckOutcome, ConnectionCheckOutcome,
    ConnectionOutcome,
};
use serde_json::Value;
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

    let offered = DomainPackageRef {
        identity: identity.clone(),
        version: version.clone(),
        digest: actual_digest,
    };
    let (admitted, admitted_bytes) = admit(&offered, SHA256_JCS_DIGEST_DOMAIN, &bytes_by_digest)
        .expect(
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
    let offered = DomainPackageRef {
        identity: identity.clone(),
        version,
        digest,
    };
    let (_admitted, admitted_bytes) = admit(&offered, SHA256_JCS_DIGEST_DOMAIN, &bytes_by_digest)
        .expect(
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

const PLACEHOLDER_DIGEST: &str =
    "sha256:0000000000000000000000000000000000000000000000000000000000000000";

/// The FR-154 intake document envelope `agent-ix-semantic-ir`'s own schema
/// layer requires beyond `constructs[]`/`types[]` (M5, review of PR #200):
/// `contractVersion`, `source`, `package`'s full required member set,
/// `occurrences` and `extensions`, filled with schema-valid stand-ins.
fn wire_envelope(package_identity: &str, constructs: Value, types: Value) -> Value {
    serde_json::json!({
        "contractVersion": "2.0.0",
        "source": {
            "identity": format!("ix://{package_identity}/spec"),
            "version": "1.0.0",
            "dialect": "spec-bundle",
            "digest": PLACEHOLDER_DIGEST,
        },
        "package": {
            "identity": package_identity,
            "version": "1.0.0",
            "manifestDigest": PLACEHOLDER_DIGEST,
            "mappingVersions": [],
            "profileVersions": [],
            "lockDigest": PLACEHOLDER_DIGEST,
        },
        "occurrences": [],
        "extensions": [],
        "constructs": constructs,
        "types": types,
    })
}

fn wire_construct(module: &str, name: &str, meaning: &str, members: Value) -> Value {
    serde_json::json!({
        "kind": {"module": module, "name": name},
        "moduleVersion": "1.0.0",
        "manifestDigest": PLACEHOLDER_DIGEST,
        "construct": {
            "identity": "none",
            "shape": "record",
            "members": members,
            "meaning": meaning,
        },
    })
}

/// Every member `semantic-ir.schema.json`'s `typeDefinition` requires beyond
/// `identity`/`kind`, filled with schema-valid stand-ins; `extra`'s own
/// members are then merged on top.
fn wire_type(identity: &str, kind: Value, extra: Value) -> Value {
    let mut node = serde_json::json!({
        "identity": identity,
        "displayName": identity,
        "kind": kind,
        "roles": [],
        "origin": {
            "generated": {
                "generatorIdentity": identity,
                "generatorVersion": "1.0.0",
                "inputIdentities": [identity],
            }
        },
        "constraints": [],
        "extensions": [],
        "unknownPolicy": "reject",
    });
    if let (Some(node), Some(extra)) = (node.as_object_mut(), extra.as_object()) {
        for (key, value) in extra {
            node.insert(key.clone(), value.clone());
        }
    }
    node
}

/// Every member `semantic-ir.schema.json`'s `field` requires beyond
/// `identity`/`typeRef`, filled with schema-valid stand-ins. The
/// multiplicity omits `upper`/`ordered`/`unique`: `crate::model::intake`'s
/// own reader requires `ordered`/`unique` present, but
/// `agent-ix-semantic-ir`'s `field_rules` refuses either present at all on
/// a single-valued (`upper <= 1`) field -- an unbounded multiplicity (no
/// `upper`) satisfies both.
fn wire_field(identity: &str, name: &str, type_ref: &str) -> Value {
    serde_json::json!({
        "identity": identity,
        "name": name,
        "typeRef": type_ref,
        "presence": "optional",
        "nullable": false,
        "defaultKind": "none",
        "multiplicity": {"lower": 0, "ordered": false, "unique": true},
        "origin": {
            "generated": {
                "generatorIdentity": identity,
                "generatorVersion": "1.0.0",
                "inputIdentities": [identity],
            }
        },
    })
}

/// (b): a hand-written Semantic IR 2.0.0 document using QSpec's own identity
/// form throughout -- `ix://<package identity>/<artifact id>` for every
/// type-definition node, never FCD's own `ix://<pkg>/type/<id>` form -- with
/// no `relationships[]` and no non-empty operation `frame` (FCD #199 gaps 2,
/// 3 and 4). Every field's `typeRef` names a package type node declared in
/// this same document: until the FCD rev bump, `validate_with_semantic_ir`
/// refuses any document naming a native value type
/// (`ix://quire/native/<Name>`), so a whole-pipeline success case cannot
/// exercise native-typeRef resolution -- that is covered separately by
/// direct unit tests of `read_value_type_ref`. It admits through
/// `agent-ix-semantic-ir`'s own validator (M5) --
/// `wire_envelope`/`wire_construct`/`wire_type`/`wire_field` fill every
/// member that validator's schema layer requires beyond the shape
/// `crate::model::intake` itself reads -- not just this reader's own
/// hand-rolled checks.
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
    let sys = format!("ix://{package_identity}/Sys");
    let pump = format!("ix://{package_identity}/Pump");
    let tank = format!("ix://{package_identity}/Tank");

    let document = wire_envelope(
        package_identity,
        serde_json::json!([
            wire_construct(
                package_identity,
                "object_type",
                meaning::OBJECT_TYPE,
                serde_json::json!({}),
            ),
            wire_construct(
                package_identity,
                "part",
                meaning::SYSTEMS_PART,
                serde_json::json!({
                    "declaredType": "required",
                    "fields": "forbidden",
                    "multiplicity": "required",
                    "operations": "forbidden",
                    "owner": "required",
                }),
            ),
            wire_construct(
                package_identity,
                "port",
                meaning::SYSTEMS_PORT,
                serde_json::json!({
                    "direction": "required",
                    "fields": "forbidden",
                    "interfaceType": "required",
                    "multiplicity": "required",
                    "operations": "forbidden",
                    "owner": "required",
                }),
            ),
            wire_construct(
                package_identity,
                "connection",
                meaning::SYSTEMS_CONNECTION,
                serde_json::json!({
                    "fields": "forbidden",
                    "flowDirection": "required",
                    "operations": "forbidden",
                    "sourceEnd": "required",
                    "targetEnd": "required",
                }),
            ),
            wire_construct(
                package_identity,
                "allocation",
                meaning::SYSTEMS_ALLOCATION,
                serde_json::json!({
                    "fields": "forbidden",
                    "operations": "forbidden",
                    "sourceElement": "required",
                    "targetElement": "required",
                }),
            ),
        ]),
        serde_json::json!([
            wire_type(
                &flow_type,
                serde_json::json!({"module": package_identity, "name": "object_type"}),
                serde_json::json!({
                    "supertypes": [],
                    "fields": [wire_field(
                        &format!("{flow_type}/rate"),
                        "rate",
                        &sys,
                    )],
                    "operations": [],
                }),
            ),
            wire_type(
                &sys,
                serde_json::json!({"module": package_identity, "name": "object_type"}),
                serde_json::json!({"supertypes": [], "fields": [], "operations": []}),
            ),
            wire_type(
                &pump,
                serde_json::json!({"module": package_identity, "name": "object_type"}),
                serde_json::json!({"supertypes": [], "fields": [], "operations": []}),
            ),
            wire_type(
                &tank,
                serde_json::json!({"module": package_identity, "name": "object_type"}),
                serde_json::json!({"supertypes": [], "fields": [], "operations": []}),
            ),
            wire_type(
                &sys_pump,
                serde_json::json!({"module": package_identity, "name": "part"}),
                serde_json::json!({
                    "owner": sys.clone(),
                    "declaredType": pump,
                    "multiplicity": multiplicity_one(1, Some(1)),
                }),
            ),
            wire_type(
                &sys_tank,
                serde_json::json!({"module": package_identity, "name": "part"}),
                serde_json::json!({
                    "owner": sys,
                    "declaredType": tank,
                    "multiplicity": multiplicity_one(1, Some(1)),
                }),
            ),
            wire_type(
                &pump_out,
                serde_json::json!({"module": package_identity, "name": "port"}),
                serde_json::json!({
                    "owner": sys_pump.clone(),
                    "interfaceType": flow_type.clone(),
                    "direction": "out",
                    "multiplicity": multiplicity_one(1, Some(1)),
                }),
            ),
            wire_type(
                &tank_in,
                serde_json::json!({"module": package_identity, "name": "port"}),
                serde_json::json!({
                    "owner": sys_tank,
                    "interfaceType": flow_type,
                    "direction": "in",
                    "multiplicity": multiplicity_one(1, Some(1)),
                }),
            ),
            wire_type(
                &pipe,
                serde_json::json!({"module": package_identity, "name": "connection"}),
                serde_json::json!({
                    "sourceEnd": {"type": pump_out.clone(), "multiplicity": multiplicity_one(1, Some(1))},
                    "targetEnd": {"type": tank_in, "multiplicity": multiplicity_one(1, Some(1))},
                    "flowDirection": "source-to-target",
                }),
            ),
            wire_type(
                &pump_alloc,
                serde_json::json!({"module": package_identity, "name": "allocation"}),
                serde_json::json!({
                    "sourceElement": pump_out,
                    "targetElement": sys_pump,
                }),
            ),
        ]),
    )
    .to_string()
    .into_bytes();

    let digest: [u8; 32] = Sha256::digest(&document).into();
    let mut bytes_by_digest = BTreeMap::new();
    bytes_by_digest.insert(digest, document.clone());
    let offered = DomainPackageRef {
        identity: package_identity.to_owned(),
        version: "1.0.0".to_owned(),
        digest,
    };
    let (package_ref, admitted_bytes) = admit(&offered, SHA256_JCS_DIGEST_DOMAIN, &bytes_by_digest)
        .expect("a matching selection admits");

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

/// Write `contents` (a `(relative path, bytes)` list) under a fresh tempdir
/// and return it.
fn write_bundle(contents: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    for (path, text) in contents {
        let full = dir.path().join(path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).expect("create fixture parent dir");
        }
        std::fs::write(&full, text).expect("write fixture file");
    }
    dir
}

/// (L2) `lift_document`'s `LiftFailure::Refused` branch: FCD's real `lift`
/// refuses a bundle outright, before extraction, when `spec.md` carries no
/// `org` (FCD's own `BUNDLE_UNIDENTIFIED` negative, `crates/extraction-frontend/
/// fixtures/negatives/BUNDLE_UNIDENTIFIED` at the pinned rev): FCD cannot
/// mint even the bundle's own identity, so no document is written at all.
#[test]
fn lift_document_refuses_a_bundle_with_no_identity() {
    let bundle = write_bundle(&[(
        "spec/spec.md",
        "---\ntype: master-requirements\nname: config-service\ntitle: \"Config Service\"\n---\n# Config Service\n",
    )]);
    let fixtures = fixtures_dir();
    let module_roots = vec![
        fixtures.join("modules/spec-objects-business"),
        fixtures.join("modules/edge-vocabulary"),
        fixtures.join("modules/spec-objects-architecture"),
    ];

    let error = lift_document(bundle.path(), &module_roots)
        .expect_err("a bundle with no org: carries no identity for FCD to mint");
    match error {
        quire_spec_language::model::intake::LiftFailure::Refused(refusal) => {
            assert_eq!(
                refusal,
                agent_ix_extraction_frontend::Refusal {
                    diagnostic: Box::new(agent_ix_extraction_frontend::Diagnostic {
                        code: agent_ix_extraction_frontend::WireCode::Registry(
                            agent_ix_extraction_frontend::Code::BundleUnidentified
                        ),
                        severity: agent_ix_extraction_frontend::Severity::Error,
                        message: "spec/spec.md carries no `org`".to_owned(),
                        owner: "ix://agent-ix/filament-core-data/extraction-frontend".to_owned(),
                        locus: Some(agent_ix_extraction_frontend::Locus {
                            source_identity: "ix://local/bundle/spec".to_owned(),
                            path: "spec/spec.md".to_owned(),
                            start_line: 1,
                            start_column: 1,
                        }),
                        blocking: true,
                        causes: Vec::new(),
                        related: Vec::new(),
                    }),
                }
            );
        }
        other => panic!("expected LiftFailure::Refused, got {other:?}"),
    }
}

/// (L2) `lift_document`'s `LiftFailure::Blocked` branch: FCD's real `lift`
/// loads and extracts the bundle, then blocks on a rules-layer diagnostic --
/// here two entities whose constrained field alias and authored id collide
/// on the same minted identity (FCD's own `DUPLICATE_IDENTITY` negative,
/// `crates/extraction-frontend/fixtures/negatives/DUPLICATE_IDENTITY` at the
/// pinned rev). Unlike `Refused`, the bundle itself loaded fine; the block
/// happens after extraction, so `diagnostics` carries the blocking finding.
#[test]
fn lift_document_blocks_on_a_duplicate_identity() {
    let bundle = write_bundle(&[
        (
            "spec/spec.md",
            "---\ntype: master-requirements\nname: identity-service\norg: agent-ix\ntitle: \"Identity Service\"\n---\n# Identity Service\n",
        ),
        (
            "spec/functional/FR-001-note.md",
            "---\nid: FR-001\ntitle: Note\nobject: entity\ntype: FR\n---\n# FR-001: Note\n\n## Description\n\nAn authored fixture record whose constrained field mints the `NoteRevision`\nalias.\n\n## Properties\n\n| Field | Type | Multiplicity | Constraints |\n|-------|------|--------------|-------------|\n| revision | Integer | 1 | min: 1 |\n| id | UUID | 1 | identity |\n",
        ),
        (
            "spec/functional/FR-002-note-revision.md",
            "---\nid: FR-001Revision\ntitle: NoteRevision\nobject: entity\ntype: FR\n---\n# FR-001Revision: NoteRevision\n\n## Description\n\nAn authored fixture record that collides with the alias minted for\n`Note.revision`.\n\n## Properties\n\n| Field | Type | Multiplicity | Constraints |\n|-------|------|--------------|-------------|\n| id | UUID | 1 | identity |\n",
        ),
    ]);
    let fixtures = fixtures_dir();
    let module_roots = vec![
        fixtures.join("modules/spec-objects-business"),
        fixtures.join("modules/edge-vocabulary"),
        fixtures.join("modules/spec-objects-architecture"),
    ];

    let error = lift_document(bundle.path(), &module_roots).expect_err(
        "FR-001Revision's authored id collides with the alias FR-001.revision already minted",
    );
    match error {
        quire_spec_language::model::intake::LiftFailure::Blocked(diagnostics) => {
            assert_eq!(
                diagnostics,
                vec![agent_ix_extraction_frontend::Diagnostic {
                    code: agent_ix_extraction_frontend::WireCode::Registry(
                        agent_ix_extraction_frontend::Code::DuplicateIdentity
                    ),
                    severity: agent_ix_extraction_frontend::Severity::Error,
                    message: "identity `ix://agent-ix/identity-service/type/FR-001Revision` \
                              is already minted by an earlier node"
                        .to_owned(),
                    owner: "ix://agent-ix/filament-core-data/extraction-frontend".to_owned(),
                    locus: Some(agent_ix_extraction_frontend::Locus {
                        source_identity: "ix://agent-ix/identity-service/spec".to_owned(),
                        path: "spec/functional/FR-002-note-revision.md".to_owned(),
                        start_line: 1,
                        start_column: 1,
                    }),
                    blocking: true,
                    causes: Vec::new(),
                    related: vec![agent_ix_extraction_frontend::Locus {
                        source_identity: "ix://agent-ix/identity-service/spec".to_owned(),
                        path: "spec/functional/FR-001-note.md".to_owned(),
                        start_line: 18,
                        start_column: 3,
                    }],
                }]
            );
        }
        other => panic!("expected LiftFailure::Blocked, got {other:?}"),
    }
}
