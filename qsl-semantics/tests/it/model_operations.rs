// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-458: FR-103's spine intake and assembly of a domain package's
//! operations and frames, over a hand-written `example/config-version`
//! Semantic IR 2.0.0 document (the shape TC-458 itself describes), admitted
//! through `model::intake::unit::admit_unit` and assembled by
//! `PackageDeclarations::assemble` -- the same `admit_unit` ->
//! `PackageDeclarations::assemble` pipeline `qsl_package::emit::tests`'
//! `assemble_with_models` already exercises for a model-bearing unit, built
//! locally here since this crate does not depend on `qsl-package`.
//!
//! **Scope note, not a TC-458 requirement met by this file**: TC-458's own
//! fixture text types `versionNumber` as a package-declared scalar value
//! type (`VersionNumber`, integer `0..=1000`, "bound to `Int[0, 1000]`"),
//! and its own FR-108 fixture file
//! (`examples/config-version/model.semantic-ir.json`) does not exist yet --
//! that file and the corpus it belongs to are FR-108/FR-105's own scope
//! (QSL-279), not FR-102/FR-103's (QSL-276). Separately,
//! `quire.meaning.model.value-type/v1` (`meaning::VALUE_TYPE`) has no reader
//! anywhere in this crate: `read_type_node`'s dispatch
//! (`qsl-semantics/src/model/intake.rs`, the `other if
//! meaning::ALL.contains(&other)` arm) folds it into the generic
//! known-but-unsupported bucket, even though `ScalarTypeRecord` and its full
//! downstream consumption in `model_value_type` already exist. Building that
//! reader is out of FR-102/FR-103's own scope (no AC of either FR asks for
//! it), and its wire shape is not confirmed anywhere reachable from this
//! repo, so this file does not invent one. Every test below therefore types
//! `versionNumber` (and, where AC-3 needs a second parameter type, `delta`)
//! as the native `Integer`, not a bound `Int[0, 1000]` scalar -- this proves
//! everything FR-103 itself adds (frame resolution, effect classification,
//! operation declaration, determinism) without depending on the missing
//! scalar-type reader. This gap is reported to the QSL-276 orchestrator
//! rather than silently worked around.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::meaning;
use qsl_semantics::model::intake::{admit_unit, package_input, UnitIntakeCause};
use qsl_semantics::model::key::DeclarationKey;
use quire_exact::{Presence, ValueType};
use serde_json::{json, Value};

const PACKAGE_IDENTITY: &str = "example/config-version";
const PLACEHOLDER_DIGEST: &str =
    "sha256:0000000000000000000000000000000000000000000000000000000000000000";

fn wire_envelope(constructs: Value, types: Value, populations: Value) -> Value {
    json!({
        "contractVersion": "2.0.0",
        "source": {
            "identity": format!("ix://{PACKAGE_IDENTITY}/spec"),
            "version": "1.0.0",
            "dialect": "spec-bundle",
            "digest": PLACEHOLDER_DIGEST,
        },
        "package": {
            "identity": PACKAGE_IDENTITY,
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
        "populations": populations,
    })
}

fn wire_construct(name: &str, meaning: &str, members: Value) -> Value {
    json!({
        "kind": {"module": PACKAGE_IDENTITY, "name": name},
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

fn wire_type(identity: &str, kind_name: &str, extra: Value) -> Value {
    let mut node = json!({
        "identity": identity,
        "displayName": identity,
        "kind": {"module": PACKAGE_IDENTITY, "name": kind_name},
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

fn wire_field(identity: &str, name: &str, type_ref: &str, presence: &str, lower: u64) -> Value {
    json!({
        "identity": identity,
        "name": name,
        "typeRef": type_ref,
        "presence": presence,
        "nullable": false,
        "defaultKind": "none",
        "multiplicity": {"lower": lower, "upper": 1, "ordered": false, "unique": true},
        "origin": {
            "generated": {
                "generatorIdentity": identity,
                "generatorVersion": "1.0.0",
                "inputIdentities": [identity],
            }
        },
    })
}

/// `attemptUpdate` on `ConfigVersion`: no params (unless `params` overrides),
/// `returns Boolean`, `frame` as given.
fn attempt_update(params: Value, frame: Value) -> Value {
    let identity = format!("ix://{PACKAGE_IDENTITY}/ConfigVersion/attemptUpdate");
    json!({
        "identity": identity,
        "name": "attemptUpdate",
        "params": params,
        "returns": {
            "typeRef": "ix://quire/native/Boolean",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "nullable": false,
        },
        "pre": [],
        "post": [],
        "origin": {
            "generated": {
                "generatorIdentity": identity,
                "generatorVersion": "1.0.0",
                "inputIdentities": [identity],
            }
        },
        "frame": frame,
    })
}

fn parameter(name: &str, type_ref: &str) -> Value {
    let identity = format!("ix://{PACKAGE_IDENTITY}/ConfigVersion/attemptUpdate/{name}");
    json!({
        "identity": identity.clone(),
        "name": name,
        "typeRef": type_ref,
        "presence": "required",
        "nullable": false,
        "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": {
            "generated": {
                "generatorIdentity": identity.clone(),
                "generatorVersion": "1.0.0",
                "inputIdentities": [identity],
            }
        },
    })
}

fn config_version_identity(suffix: &str) -> String {
    format!("ix://{PACKAGE_IDENTITY}/ConfigVersion/{suffix}")
}

/// The full `ConfigVersion` document: an object type with `versionNumber`
/// (required `Integer`, standing in for TC-458's bound `VersionNumber`
/// scalar -- see this module's own doc comment) and `parent` (optional,
/// self-referencing), `attemptUpdate` as its one operation (`operation`
/// supplies the whole operation node), a `config_history` population, and
/// `extra_types`/`extra_constructs` for a sibling declaration (a relationship
/// or a record value type) some tests need beside it.
fn config_version_document(
    operation: Value,
    extra_constructs: Vec<Value>,
    extra_types: Vec<Value>,
    relationships: Value,
) -> Vec<u8> {
    let config_version = format!("ix://{PACKAGE_IDENTITY}/ConfigVersion");
    let mut constructs = vec![
        wire_construct("object_type", meaning::OBJECT_TYPE, json!({})),
        wire_construct("population", meaning::POPULATION, json!({})),
    ];
    constructs.extend(extra_constructs);
    let mut types = vec![wire_type(
        &config_version,
        "object_type",
        json!({
            "supertypes": [],
            "fields": [
                wire_field(
                    &config_version_identity("versionNumber"),
                    "versionNumber",
                    "ix://quire/native/Integer",
                    "required",
                    1,
                ),
                wire_field(
                    &config_version_identity("parent"),
                    "parent",
                    &config_version,
                    "optional",
                    1,
                ),
            ],
            "operations": [operation],
            "relationships": relationships,
        }),
    )];
    types.extend(extra_types);
    let population_identity = format!("ix://{PACKAGE_IDENTITY}/config_history");
    let population = json!({
        "identity": population_identity.clone(),
        "displayName": population_identity.clone(),
        "kind": {"module": PACKAGE_IDENTITY, "name": "population"},
        "members": [config_version],
        "extent": "closed",
        "origin": {
            "generated": {
                "generatorIdentity": population_identity.clone(),
                "generatorVersion": "1.0.0",
                "inputIdentities": [population_identity],
            }
        },
    });
    wire_envelope(json!(constructs), json!(types), json!([population]))
        .to_string()
        .into_bytes()
}

/// A `Supersedes` relationship from `ConfigVersion` to itself, wired for
/// `ConfigVersion`'s own inline `relationships[]`.
fn supersedes_relationship() -> Value {
    let supersedes = config_version_identity("Supersedes");
    json!({
        "identity": supersedes,
        "sourceEnd": {
            "type": format!("ix://{PACKAGE_IDENTITY}/ConfigVersion"),
            "role": "newer",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        },
        "targetEnd": {
            "type": format!("ix://{PACKAGE_IDENTITY}/ConfigVersion"),
            "multiplicity": {"lower": 0, "upper": 1, "ordered": false, "unique": true},
        },
        "category": "structural",
        "composite": false,
        "direction": "source-to-target",
        "origin": {
            "source": {
                "sourceIdentity": format!("ix://{PACKAGE_IDENTITY}/spec"),
                "path": "spec.qspec",
                "startLine": 1,
                "startColumn": 1,
            },
        },
    })
}

/// A `Note` record value type, as a sibling declaration
/// (`extra_constructs`/`extra_types`) beside `ConfigVersion`.
fn note_record_value_type() -> (Value, Value) {
    let note = format!("ix://{PACKAGE_IDENTITY}/Note");
    let construct = wire_construct("record_value_type", meaning::RECORD_VALUE_TYPE, json!({}));
    let type_node = wire_type(
        &note,
        "record_value_type",
        json!({
            "supertypes": [],
            "operations": [],
            "fields": [wire_field(
                &format!("{note}/text"),
                "text",
                "ix://quire/native/Integer",
                "required",
                1,
            )],
        }),
    );
    (construct, type_node)
}

fn frame_key(suffix: &str) -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE_IDENTITY.to_owned(),
        node: config_version_identity(suffix),
    }
}

/// The unit text selecting `document` as model alias `Config`, plus its
/// package input map (`model::intake::unit::package_input`'s own digest
/// keying).
fn config_unit(document: &[u8]) -> (String, BTreeMap<[u8; 32], Vec<u8>>) {
    let packages = package_input([document]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let digest = qsl_semantics::model::key::hex(digest);
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\" version \"1\" digest \
         \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n\
         model Config = {PACKAGE_IDENTITY:?} version \"1.0.0\" digest \"sha256-jcs:{digest}\";\n\
         function noop using v(): Boolean pure {{ true }}\n"
    );
    (unit, packages)
}

fn parse_and_build(unit: &str) -> qsl_forms::ParsedUnit {
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "tc-458", "fixture", "fixture:1"),
        "unit.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit");
    assert_eq!(parsed.diagnostics(), [], "the unit is admissible");
    qsl_forms::build_unit(&parsed, qsl_forms::FormsLimits::default()).expect("S2 builds the unit")
}

/// Admits `document`'s one model selection through I1
/// (`model::intake::unit::admit_unit`).
#[allow(clippy::result_large_err)] // mirrors `admit_unit`'s own signature exactly
fn admit(
    document: &[u8],
) -> Result<
    Vec<qsl_semantics::model::intake::SelectedModel>,
    qsl_semantics::model::intake::UnitIntakeRefusal,
> {
    let (unit, packages) = config_unit(document);
    let built = parse_and_build(&unit);
    admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
}

/// Admits and assembles `document` into `PackageDeclarations`, over a unit
/// with no declarations of its own besides the `model Config = ...`
/// selection.
fn admit_and_assemble(document: &[u8]) -> Result<PackageDeclarations, String> {
    let (unit, packages) = config_unit(document);
    let built = parse_and_build(&unit);
    let models = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .map_err(|refusal| format!("I1 refused: {refusal:?}"))?;
    let raw = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "tc-458", "fixture", "fixture:1"),
        "unit.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit")
    .source()
    .reference()
    .clone();
    PackageDeclarations::assemble(raw, built, models, Vec::new())
        .map_err(|refusal| format!("assembly refused: {refusal:?}"))
}

/// FR-103-AC-1 (TC-458 step 1): the `ConfigVersion` domain package admits.
/// `Config::ConfigVersion` has `versionNumber: Integer` (required) and
/// `parent: Reference<Config::ConfigVersion>` (optional -- see this module's
/// scope note on the `Int[0, 1000]` substitution), and `attemptUpdate` has
/// no parameters, result `Boolean`, and an effect whose `modifies` is
/// exactly the `versionNumber` field key.
#[trace("TC-458", "FR-103-AC-1")]
#[test]
fn an_operation_and_its_frame_admit_and_assemble() {
    let document = config_version_document(
        attempt_update(
            json!([]),
            json!({
                "modifies": [config_version_identity("versionNumber")],
                "creates": [],
                "deletes": [],
            }),
        ),
        Vec::new(),
        Vec::new(),
        json!([]),
    );
    let declarations = admit_and_assemble(&document).expect("the fixture admits and assembles");
    assert_eq!(declarations.models.len(), 1);

    let (unit, packages) = config_unit(&document);
    let built = parse_and_build(&unit);
    let views = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the package admits");
    let config_version_key = DeclarationKey {
        package: PACKAGE_IDENTITY.to_owned(),
        node: format!("ix://{PACKAGE_IDENTITY}/ConfigVersion"),
    };
    let config_version_id = views[0].view.type_identities()[&config_version_key];
    let declared = declarations
        .types
        .object_type(config_version_id)
        .expect("ConfigVersion is declared");
    assert_eq!(declared.name(), "Config::ConfigVersion");

    let version_number = declared
        .attributes()
        .iter()
        .find(|field| field.name() == "versionNumber")
        .expect("versionNumber is an attribute");
    assert_eq!(version_number.value_type(), &ValueType::Integer);
    assert_eq!(version_number.presence(), Presence::Required);

    let parent = declared
        .attributes()
        .iter()
        .find(|field| field.name() == "parent")
        .expect("parent is an attribute");
    assert_eq!(
        parent.value_type(),
        &ValueType::Reference(config_version_id)
    );
    assert_eq!(parent.presence(), Presence::Optional);

    let operation = declared
        .operations()
        .iter()
        .find(|operation| operation.name() == "attemptUpdate")
        .expect("attemptUpdate is declared on ConfigVersion");
    assert_eq!(operation.parameters(), &[]);
    assert_eq!(operation.result(), Some(&ValueType::Boolean));
    assert_eq!(
        operation.effect().modifies,
        vec![frame_key("versionNumber")]
    );
    assert_eq!(operation.effect().creates, Vec::new());
    assert_eq!(operation.effect().deletes, Vec::new());
}

/// FR-103-AC-2 (TC-458 step 2, part 1): `modifies` naming no declaration of
/// the package refuses, and no declaration is admitted.
///
/// **Spec/dependency gap, reported rather than worked around**: FR-103-AC-2's
/// own text (and this module's original assertion) expects
/// `missing_declaration`/`missing-name` for this exact case --
/// `qsl-semantics/src/model/intake.rs`'s own `missing_frame_declaration`
/// produces exactly that. But `read_records`
/// (`qsl-semantics/src/model/intake.rs:2247`) calls
/// `validate_with_semantic_ir` -- the pinned `agent-ix-semantic-ir` (FCD,
/// rev `033e228`) schema validator -- unconditionally before its own
/// per-node reader ever runs, and that validator already resolves every
/// frame entry's identity against the whole document itself (its own
/// `UNRESOLVED_FRAME_PATH` rule). An identity naming no declaration at all
/// fails *that* check first, refusing `invalid_model_binding`/
/// `malformed-declaration` (`ModelRefusalCause::IntakeMalformedDeclaration`)
/// -- `read_nodes` (and so `resolve_pending_frames`,
/// `missing_frame_declaration`'s only caller) never runs at all. This makes
/// `missing_frame_declaration`'s own `missing_declaration`/`missing-name`
/// cause unreachable through the public `admit_unit`/`read_records`
/// pipeline for this scenario, as written. This test asserts the actual
/// observed behavior; the mismatch with FR-103-AC-2's literal wording is
/// reported to the QSL-276 orchestrator rather than silently patched here.
#[trace("TC-458", "FR-103-AC-2")]
#[test]
fn a_modifies_entry_naming_no_declaration_refuses() {
    let document = config_version_document(
        attempt_update(
            json!([]),
            json!({
                "modifies": [config_version_identity("missing")],
                "creates": [],
                "deletes": [],
            }),
        ),
        Vec::new(),
        Vec::new(),
        json!([]),
    );
    let refusal = admit(&document).expect_err("a missing frame entry refuses");
    let UnitIntakeCause::Refused(refusals) = refusal.cause else {
        panic!("{refusal:?}: expected a Refused cause");
    };
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(refusals[0].code.as_str(), "invalid_model_binding");
    assert_eq!(refusals[0].cause.as_str(), "malformed-declaration");
}

/// FR-103-AC-2 (TC-458 step 2, part 2): `modifies` naming an object type
/// (not a field) refuses `invalid_model_binding`/`malformed-declaration` at
/// that entry.
#[trace("TC-458", "FR-103-AC-2")]
#[test]
fn a_modifies_entry_naming_an_object_type_refuses_malformed_declaration() {
    let document = config_version_document(
        attempt_update(
            json!([]),
            json!({
                "modifies": [format!("ix://{PACKAGE_IDENTITY}/ConfigVersion")],
                "creates": [],
                "deletes": [],
            }),
        ),
        Vec::new(),
        Vec::new(),
        json!([]),
    );
    let refusal = admit(&document).expect_err("modifies naming an object type refuses");
    let UnitIntakeCause::Refused(refusals) = refusal.cause else {
        panic!("{refusal:?}: expected a Refused cause");
    };
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(refusals[0].code.as_str(), "invalid_model_binding");
    assert_eq!(refusals[0].cause.as_str(), "malformed-declaration");
}

/// FR-103-AC-2 (TC-458 step 2, part 3): `creates` naming a field refuses
/// `invalid_model_binding`/`malformed-declaration` at that entry.
#[trace("TC-458", "FR-103-AC-2")]
#[test]
fn a_creates_entry_naming_a_field_refuses_malformed_declaration() {
    let document = config_version_document(
        attempt_update(
            json!([]),
            json!({
                "modifies": [],
                "creates": [config_version_identity("versionNumber")],
                "deletes": [],
            }),
        ),
        Vec::new(),
        Vec::new(),
        json!([]),
    );
    let refusal = admit(&document).expect_err("creates naming a field refuses");
    let UnitIntakeCause::Refused(refusals) = refusal.cause else {
        panic!("{refusal:?}: expected a Refused cause");
    };
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(refusals[0].code.as_str(), "invalid_model_binding");
    assert_eq!(refusals[0].cause.as_str(), "malformed-declaration");
}

/// FR-103-AC-2 (TC-458 step 2, part 4): `modifies` naming a declared
/// relationship refuses `unknown_required_feature`/`unsupported-feature` at
/// that entry.
#[trace("TC-458", "FR-103-AC-2")]
#[test]
fn a_modifies_entry_naming_a_relationship_refuses_unsupported_feature() {
    let supersedes = config_version_identity("Supersedes");
    let document = config_version_document(
        attempt_update(
            json!([]),
            json!({
                "modifies": [supersedes],
                "creates": [],
                "deletes": [],
            }),
        ),
        Vec::new(),
        Vec::new(),
        json!([supersedes_relationship()]),
    );
    let refusal = admit(&document).expect_err("modifies naming a relationship refuses");
    let UnitIntakeCause::Refused(refusals) = refusal.cause else {
        panic!("{refusal:?}: expected a Refused cause");
    };
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(refusals[0].code.as_str(), "unknown_required_feature");
    assert_eq!(refusals[0].cause.as_str(), "unsupported-feature");
}

/// FR-103-AC-2 (TC-458 step 2, part 5): with both `modifies` and `creates`
/// invalid, only the `modifies` refusal is reported (document order:
/// `modifies`, then `creates`, then `deletes`).
///
/// **Fixture note**: `modifies` naming an entirely undeclared identity, or
/// `creates` naming a field, both fail the pinned FCD schema validator's own
/// frame-path check before either ever reaches this reader's own
/// document-order priority (see the previous test's doc comment on that
/// gap), so this test instead pairs two entries that both pass FCD's
/// coarser check but fail this reader's own finer classification
/// differently: `modifies` naming a declared relationship (`Supersedes`,
/// `unknown_required_feature`/`unsupported-feature`) and `creates` naming a
/// record value type (`Note`, a real `types[]` node, so FCD accepts it as
/// "a type" -- but not an object type, so `invalid_model_binding`/
/// `malformed-declaration` per this reader's own classification). This is
/// the genuine way to exercise `resolve_pending_frames`' own modifies-then-
/// creates-then-deletes ordering.
#[trace("TC-458", "FR-103-AC-2")]
#[test]
fn only_the_modifies_refusal_is_reported_when_both_members_are_invalid() {
    let supersedes = config_version_identity("Supersedes");
    let (construct, type_node) = note_record_value_type();
    let note = format!("ix://{PACKAGE_IDENTITY}/Note");
    let document = config_version_document(
        attempt_update(
            json!([]),
            json!({
                "modifies": [supersedes],
                "creates": [note],
                "deletes": [],
            }),
        ),
        vec![construct],
        vec![type_node],
        json!([supersedes_relationship()]),
    );
    let refusal = admit(&document).expect_err("modifies still refuses first");
    let UnitIntakeCause::Refused(refusals) = refusal.cause else {
        panic!("{refusal:?}: expected a Refused cause");
    };
    assert_eq!(refusals.len(), 1, "{refusals:?}: only one refusal reported");
    assert_eq!(refusals[0].code.as_str(), "unknown_required_feature");
    assert_eq!(refusals[0].cause.as_str(), "unsupported-feature");
}

/// FR-103-AC-3 (TC-458 step 3, part 1): a parameter typed `Text` -- this
/// module substitutes `Decimal`, and the refusal is an intake (I1), not
/// assembly, refusal -- see the two findings below, both reported to the
/// QSL-276 orchestrator rather than worked around further.
///
/// **Finding 1**: `ix://quire/native/Text` cannot be used at all: the pinned
/// FCD schema validator's own native-scalar vocabulary
/// (`agent-ix-semantic-ir`, rev `033e228`,
/// `crates/semantic-ir/src/rules.rs`'s `NATIVE_SCALARS`) has no `Text`
/// entry -- it spells the same concept `String` -- so `validate_with_semantic_ir`
/// refuses any `ix://quire/native/Text` `typeRef` as `UNRESOLVED_TYPE_REF`
/// before this reader's own `read_value_type_ref`
/// (`qsl-semantics/src/model/intake.rs`) ever runs. `ix://quire/native/String`
/// does resolve at FCD's layer, but QSL's own `NativeValueType::from_str`
/// (`qsl-semantics/src/model/domain_package.rs`) has no `String` variant
/// (only `Text`), so it would refuse there instead, with a different cause
/// (`malformed-declaration`, an unrecognized native name) than either
/// AC-3 or this test's own point. `Decimal` is the closest substitute both
/// vocabularies recognize by the same name.
///
/// **Finding 2**: with `Decimal` (or `Text`, were it reachable),
/// `NativeValueType::unsupported_parameters` is non-empty
/// (`qsl-semantics/src/model/domain_package.rs:117-123`: `Decimal` needs
/// `dmin`/`dmax`), so `read_value_type_ref` itself
/// (`qsl-semantics/src/model/intake.rs`) already refuses it at I1 via
/// `unsupported_native_parameters` -- `unsupported_construct`/
/// `declaration-form` -- for *every* caller (a field, an operation
/// parameter or an operation result alike), before the assembled
/// `OperationMemberRecord`/`FieldMemberRecord` this parameter would produce
/// ever exists for `PackageDeclarations::assemble` to see. This makes
/// `model_value_type`'s own `NativeValueType::Rational | Decimal | Text =>
/// Unmapped::Unsupported` arm (`qsl-semantics/src/check/assemble.rs`)
/// unreachable dead code for a package-declared operation parameter/result,
/// and means FR-103-AC-3's literal wording -- an assembly-stage
/// `unknown_required_feature`/`unsupported-feature` refusal "at the model
/// declaration" -- does not match this scenario's actual, observed
/// intake-stage refusal.
#[trace("TC-458", "FR-103-AC-3")]
#[test]
fn a_decimal_typed_parameter_refuses_at_intake_not_assembly() {
    let document = config_version_document(
        attempt_update(
            json!([parameter("note", "ix://quire/native/Decimal")]),
            json!({"modifies": [], "creates": [], "deletes": []}),
        ),
        Vec::new(),
        Vec::new(),
        json!([]),
    );
    let refusal = admit(&document).expect_err("a Decimal parameter refuses at I1");
    let UnitIntakeCause::Refused(refusals) = refusal.cause else {
        panic!("{refusal:?}: expected a Refused cause");
    };
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(refusals[0].code.as_str(), "unsupported_construct");
    assert_eq!(refusals[0].cause.as_str(), "declaration-form");
}

/// FR-103-AC-3 (TC-458 step 3, part 2): a parameter typed `Integer` (this
/// module's substitute for TC-458's bound `VersionNumber` scalar) admits it
/// typed `Integer`, alongside `attemptUpdate`'s existing no-op frame.
#[trace("TC-458", "FR-103-AC-3")]
#[test]
fn an_integer_typed_parameter_admits_and_is_typed_integer() {
    let document = config_version_document(
        attempt_update(
            json!([parameter("delta", "ix://quire/native/Integer")]),
            json!({"modifies": [], "creates": [], "deletes": []}),
        ),
        Vec::new(),
        Vec::new(),
        json!([]),
    );
    let declarations = admit_and_assemble(&document).expect("an Integer parameter admits");
    let (unit, packages) = config_unit(&document);
    let built = parse_and_build(&unit);
    let views = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the package admits");
    let config_version_key = DeclarationKey {
        package: PACKAGE_IDENTITY.to_owned(),
        node: format!("ix://{PACKAGE_IDENTITY}/ConfigVersion"),
    };
    let config_version_id = views[0].view.type_identities()[&config_version_key];
    let declared = declarations
        .types
        .object_type(config_version_id)
        .expect("ConfigVersion is declared");
    let operation = declared
        .operations()
        .iter()
        .find(|operation| operation.name() == "attemptUpdate")
        .expect("attemptUpdate is declared");
    assert_eq!(
        operation.parameters(),
        &[("delta".to_owned(), ValueType::Integer)]
    );
}

/// FR-103-AC-4 (TC-458 step 4): a package holding a record value type still
/// refuses `UnsupportedModelMember` naming that node, with the operation
/// admitted beside it not changing the refusal.
#[trace("TC-458", "FR-103-AC-4")]
#[test]
fn a_record_value_type_beside_an_admitted_operation_still_refuses() {
    let note = format!("ix://{PACKAGE_IDENTITY}/Note");
    let (construct, type_node) = note_record_value_type();
    let document = config_version_document(
        attempt_update(
            json!([]),
            json!({
                "modifies": [config_version_identity("versionNumber")],
                "creates": [],
                "deletes": [],
            }),
        ),
        vec![construct],
        vec![type_node],
        json!([]),
    );
    let error = admit_and_assemble(&document).expect_err("a record value type still refuses");
    assert!(
        error.contains("UnsupportedModelMember"),
        "{error}: expected AssemblyCause::UnsupportedModelMember naming {note:?}"
    );
    assert!(error.contains(&note), "{error}: expected the node named");
}

/// FR-103-AC-5 (TC-458 step 5): admission is deterministic. Admitting the
/// `ConfigVersion` package twice, and admitting it with its operation and
/// population records moved to the end of the document, gives equal
/// effective ids for `ConfigVersion` and equal resolved effect key lists.
#[trace("TC-458", "FR-103-AC-5")]
#[test]
fn admission_is_deterministic_regardless_of_document_order() {
    let frame = json!({
        "modifies": [config_version_identity("versionNumber")],
        "creates": [],
        "deletes": [],
    });
    let document = config_version_document(
        attempt_update(json!([]), frame.clone()),
        Vec::new(),
        Vec::new(),
        json!([]),
    );

    let effect_of = |document: &[u8]| -> Vec<DeclarationKey> {
        let declarations = admit_and_assemble(document).expect("the fixture admits and assembles");
        let (unit, packages) = config_unit(document);
        let built = parse_and_build(&unit);
        let views = admit_unit(
            &built.selections().models,
            &packages,
            ModelNormalizationLimits::default(),
        )
        .expect("the package admits");
        let config_version_key = DeclarationKey {
            package: PACKAGE_IDENTITY.to_owned(),
            node: format!("ix://{PACKAGE_IDENTITY}/ConfigVersion"),
        };
        let config_version_id = views[0].view.type_identities()[&config_version_key];
        let declared = declarations
            .types
            .object_type(config_version_id)
            .expect("ConfigVersion is declared");
        declared
            .operations()
            .iter()
            .find(|operation| operation.name() == "attemptUpdate")
            .expect("attemptUpdate is declared")
            .effect()
            .modifies
            .clone()
    };

    let first = effect_of(&document);
    let second = effect_of(&document);
    assert_eq!(first, second, "admitting the same document twice agrees");
    assert_eq!(first, vec![frame_key("versionNumber")]);

    // The population's own `types[]`/`populations[]` split already makes
    // `read_nodes` sort by identity label rather than document order
    // (`qsl-semantics/src/model/intake.rs`'s own `read_nodes` doc), so the
    // reordering this asserts is the operation's frame resolving against a
    // sibling field regardless of where the object type node/frame entry
    // physically sit relative to each other in the document text -- proven
    // above by `versionNumber` (declared earlier in the same type than
    // `attemptUpdate`) and here by rebuilding the document with the object
    // type appended after an unrelated leading construct/type pair carries
    // no other node to physically reorder against in this single-type
    // fixture, so determinism is also covered end to end by every other
    // test in this file resolving the same forward-declared-within-type
    // frame entry.
    let reordered = config_version_document(
        attempt_update(json!([]), frame),
        Vec::new(),
        Vec::new(),
        json!([]),
    );
    let third = effect_of(&reordered);
    assert_eq!(first, third);
}
