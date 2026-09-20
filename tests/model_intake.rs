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
//! `reading_fcd_199s_golden_shape_admits_the_schema_and_refuses_4_of_its_12_types`
//! carries FCD PR #200's own architecture golden (vendored under
//! `tests/fixtures/architecture/expected/semantic-ir.json`, now that the pin
//! covers it) into `intake::read_records` and asserts the measured, per-node
//! breakdown: `validate_with_semantic_ir` admits the whole document at this
//! pin, so `read_records` reaches per-node reading, and 4 of the golden's 12
//! types refuse while 8 read clean -- see that test's own doc comment for
//! the breakdown.
//!
//! `a_qspec_conformant_document_admits_reads_and_classifies` is the
//! complementary positive case: a hand-written Semantic IR 2.0.0 document
//! using QSpec's own identity form throughout (not FCD's) and no
//! non-empty `frame` (FCD #199 gap 2 and a separate, still-latent gap this
//! fixture does not exercise), run through the full pipeline -- `admit` ->
//! `read_records` -> `DomainPackage::new` -> `classify` ->
//! `check_connection`/`check_allocation` -- asserting a wholly successful
//! outcome.

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

/// (a): `agent-ix/filament-core-data` PR #200 (merged; this crate's pinned
/// `agent-ix-extraction-frontend`/`agent-ix-semantic-ir` rev now includes
/// it) fixes FCD #199's four gaps and produces the architecture golden
/// vendored at `tests/fixtures/architecture/expected/semantic-ir.json`
/// (`cargo xtask revendor --tree test-fixtures-architecture`): 12 types,
/// `kind: {module, name}`, no package-local scalar/alias nodes, no
/// `/type//field//operation/` identity segments, and `Flow2`'s inline
/// `relationships[]` in the real `sourceEnd`/`targetEnd`
/// (`role`+`multiplicity`+`type`) shape.
///
/// Measured (not predicted) against the now-pinned rev:
/// `validate_with_semantic_ir` admits the whole document -- the schema at
/// this pin recognizes both a field's `constraints[]` and a relationship's
/// `sourceEnd`/`targetEnd`/`role`/`direction` shape, so `decide` reports no
/// `Severity::Error` diagnostic and `read_records` reaches its own per-node
/// reading over the golden for the first time. Of its 12 types (no
/// `populations[]`), 4 refuse and 8 read clean:
///
/// - `Count`: [`quire_spec_language::model::normalize::ModelRefusalCause::UnsupportedDeclarationForm`]
///   -- `quire.meaning.model.record-value-type/v1` has no reader yet (a
///   separate, still-open gap, not this pin's).
/// - `Pump.id`, `Sys.id`, `Tank.id`: `IntakeMalformedDeclaration` -- each
///   field's `typeRef` is `ix://quire/native/UUID`, R5's ruling that QSL's
///   native-value-type vocabulary is the conformant side and FCD's wider
///   one is not QSL's to narrow (see [`super::read_value_type_ref`]'s docs
///   and its own
///   `refuses_uuid_as_malformed_declaration_r5_holds_until_plat_836` test).
///   PLAT-836 (a separate ticket) narrows what FCD *emits* for these
///   fields, not this reader; until it lands, this refusal is correct.
///
/// A prior static reading of this reader predicted 5 of 12 refusing,
/// naming `Flow2` alongside `Count`/`Pump`/`Sys`/`Tank` on the theory that
/// its relationships would still miss a role/multiplicity this reader
/// requires. That prediction was never observable before the pin bump (the
/// whole-document schema refusal short-circuited every per-node read) and
/// the measurement above supersedes it: `Flow2` reads clean along with
/// `Flow`, `pipe`, `pump_alloc`, `pump_out`, `sys_pump`, `sys_tank` and
/// `tank_in` -- 8, not 7, read clean, and the refusing set is exactly
/// `{Count, Pump, Sys, Tank}`, not the predicted `{Count, Flow2, Pump, Sys,
/// Tank}`.
#[test]
fn reading_fcd_199s_golden_shape_admits_the_schema_and_refuses_4_of_its_12_types() {
    let package_identity = "agent-ix/architecture";
    let document = include_str!("fixtures/architecture/expected/semantic-ir.json");

    let refusals = read_records(package_identity, document.as_bytes()).expect_err(
        "measured: 4 of the golden's 12 types refuse at read_records even though the whole \
         document now clears validate_with_semantic_ir's schema check",
    );

    assert_eq!(
        refusals.len(),
        4,
        "measured refusal count over the post-#200 golden: {refusals:#?}"
    );

    use quire_spec_language::diagnostic::Code;
    use quire_spec_language::model::normalize::ModelRefusalCause;

    let (count, pump, sys, tank) = (&refusals[0], &refusals[1], &refusals[2], &refusals[3]);

    assert_eq!(count.code, Code::UnsupportedConstruct);
    match &count.cause {
        ModelRefusalCause::UnsupportedDeclarationForm { node, what } => {
            assert_eq!(node, "ix://agent-ix/architecture/Count");
            assert_eq!(what, "quire.meaning.model.record-value-type/v1");
        }
        other => panic!("Count: expected UnsupportedDeclarationForm, got {other:?}"),
    }
    assert!(
        count.detail.contains("has no reader yet"),
        "Count detail: {}",
        count.detail
    );

    for (refusal, expected_node) in [
        (pump, "ix://agent-ix/architecture/Pump/id"),
        (sys, "ix://agent-ix/architecture/Sys/id"),
        (tank, "ix://agent-ix/architecture/Tank/id"),
    ] {
        assert_eq!(refusal.code, Code::InvalidModelBinding);
        match &refusal.cause {
            ModelRefusalCause::IntakeMalformedDeclaration { node, .. } => {
                assert_eq!(node, expected_node);
            }
            other => panic!("{expected_node}: expected IntakeMalformedDeclaration, got {other:?}"),
        }
        assert!(
            refusal.detail.contains("ix://quire/native/UUID")
                && refusal
                    .detail
                    .contains("names no native value type QSL declares"),
            "{expected_node} detail: {}",
            refusal.detail
        );
    }
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
/// type-definition node, never FCD's own `ix://<pkg>/type/<id>` form (FCD
/// #199 gap 2) -- and no non-empty operation `frame` (a real, separate,
/// still-latent gap this fixture does not exercise). It carries no
/// `relationships[]` of its own here not because the shape is unsupported
/// -- `read_object_type` reads a type's inline `relationships[]` now (FCD
/// #199 gap 3, fixed) -- but because this positive case does not need one
/// to prove the pipeline whole; `read_relationship`'s own coverage is
/// separate. Every field's `typeRef` names a package type node declared in
/// this same document rather than a native value type
/// (`ix://quire/native/<Name>`); at the pinned rev,
/// `validate_with_semantic_ir` itself now resolves a native `typeRef` (see
/// `crate::model::intake::validate_with_semantic_ir`'s own doc comment for
/// FCD's upstream fix), but this fixture simply was not authored to
/// exercise that path -- native-typeRef resolution is covered separately by
/// direct unit tests of `read_value_type_ref` and, end to end, by
/// `reading_fcd_199s_golden_shape_admits_the_schema_and_refuses_4_of_its_12_types`.
/// It admits through `agent-ix-semantic-ir`'s own validator (M5) --
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
/// loads and extracts the bundle, then blocks on a rules-layer diagnostic.
///
/// This reproduces FCD's own `DUPLICATE_IDENTITY` negative fixture
/// (`crates/extraction-frontend/fixtures/negatives/DUPLICATE_IDENTITY` at
/// this crate's now-pinned rev): a single file where `FR-001`'s own field
/// and its own operation are both named `revision`. FCD #199/#200 collapses
/// the field and operation identity namespaces into one `<Owner>/<name>`
/// segment with no kind segment of its own, so the two now mint the same
/// identity and `lift_document` blocks. At the previously pinned rev
/// (`7dcb2f2c`, predating FCD #199/#200) this same bundle did *not*
/// collide -- `NodeKind::Field` and `NodeKind::Operation` minted under
/// different segments there -- which is why this test was deleted rather
/// than kept `#[ignore]`d until the pin bumped; this is that test, rebuilt
/// fresh against the collision this pin actually produces.
///
/// The expected diagnostic is read verbatim from FCD's own committed
/// `crates/extraction-frontend/fixtures/negatives/DUPLICATE_IDENTITY/expected/diagnostics.json`
/// at this crate's pinned rev, not invented here.
#[test]
fn lift_document_blocks_on_a_duplicate_identity() {
    let bundle = write_bundle(&[
        (
            "spec/spec.md",
            "---\ntype: master-requirements\nname: identity-service\norg: agent-ix\ntitle: \"Identity Service\"\n---\n# Identity Service\n",
        ),
        (
            "spec/functional/FR-001-note.md",
            "---\nid: FR-001\ntitle: Note\nobject: entity\ntype: FR\n---\n# FR-001: Note\n\n## Description\n\nAn authored fixture record whose field and whose own operation mint the\nsame member identity.\n\n## Properties\n\n| Field | Type | Multiplicity | Constraints |\n|-------|------|--------------|-------------|\n| revision | Integer | 1 | |\n| id | UUID | 1 | identity |\n\n## Operations\n\n### revision\n\nReturns: Integer [1]\n",
        ),
    ]);
    let fixtures = fixtures_dir();
    let module_roots = vec![
        fixtures.join("modules/spec-objects-business"),
        fixtures.join("modules/edge-vocabulary"),
        fixtures.join("modules/spec-objects-architecture"),
    ];

    let error = lift_document(bundle.path(), &module_roots).expect_err(
        "FR-001's own field and its own operation, both named `revision`, mint the same identity",
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
                    message: "identity `ix://agent-ix/identity-service/FR-001/revision` \
                              is already minted by an earlier node"
                        .to_owned(),
                    owner: "ix://agent-ix/filament-core-data/extraction-frontend".to_owned(),
                    locus: Some(agent_ix_extraction_frontend::Locus {
                        source_identity: "ix://agent-ix/identity-service/spec".to_owned(),
                        path: "spec/functional/FR-001-note.md".to_owned(),
                        start_line: 23,
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
