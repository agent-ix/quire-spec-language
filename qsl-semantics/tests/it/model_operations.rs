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
//! `versionNumber` is a package-declared bound integer scalar
//! (`VersionNumber`, `Int[0, 1000]`), per TC-458's own fixture text, read
//! through `quire.meaning.model.value-type/v1` (`meaning::VALUE_TYPE`,
//! `read_value_type`, `qsl-semantics/src/model/intake.rs`) --
//! [`version_number_value_type`] builds the shared `VersionNumber`
//! construct/type pair every `ConfigVersion` fixture in this file uses.
//! [`bound_integer_value_type_admits_and_assembles`] below exercises the
//! same reader end to end over a second, dedicated fixture (a `Widget`
//! object type with a `VersionNumber`-typed field), independent of
//! `ConfigVersion`. AC-3's second parameter, `delta`, is typed by the same
//! `VersionNumber` declaration and assembles to the same `Int[0, 1000]`.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::meaning;
use qsl_semantics::model::intake::{admit_unit, package_input, UnitIntakeCause};
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::operation::OperationDeclaration;
use quire_exact::{Integer, IntegerInterval, Presence, ValueType};
use quire_semantic_value::declaration::ObjectTypeDeclaration;
use serde_json::{json, Value};

pub(super) const PACKAGE_IDENTITY: &str = "example/config-version";
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
pub(super) fn attempt_update(params: Value, frame: Value) -> Value {
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
            "source": {
                "sourceIdentity": format!("ix://{PACKAGE_IDENTITY}/spec"),
                "path": "spec.qspec",
                "startLine": 1,
                "startColumn": 1,
            },
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

pub(super) fn config_version_identity(suffix: &str) -> String {
    format!("ix://{PACKAGE_IDENTITY}/ConfigVersion/{suffix}")
}

/// `VersionNumber`'s node identity, `ix://example/config-version/VersionNumber`.
pub(super) fn version_number_identity() -> String {
    format!("ix://{PACKAGE_IDENTITY}/VersionNumber")
}

/// The `ValueType` a `VersionNumber`-typed field or parameter assembles to:
/// `Int[0, 1000]`.
pub(super) fn version_number_bound() -> ValueType {
    ValueType::Int(
        IntegerInterval::new(Integer::from(0_i64), Integer::from(1000_i64))
            .expect("0 <= 1000 is a nonempty interval"),
    )
}

/// `ix://example/config-version/VersionNumber`: the package-declared bound
/// integer scalar (`Int[0, 1000]`) `versionNumber` is really typed as, per
/// TC-458's own fixture text and the shape the
/// `examples/config-version/model.semantic-ir.json` corpus already declares.
/// Returns the `value_type` construct/type pair (for `constructs`/`types`)
/// and the type's own identity (`versionNumber`'s `typeRef`);
/// `read_value_type` (`qsl-semantics/src/model/intake.rs`) is what admits
/// this shape.
pub(super) fn version_number_value_type() -> (Value, Value, String) {
    let identity = version_number_identity();
    // `agent-ix-semantic-ir`'s own `CONSTRAINT_MEMBERS` requires every
    // constraint's `identity`/`appliesTo`/`diagnosticCode`/`origin` present
    // (this module's own tests all route through the full
    // `validate_with_semantic_ir` schema check, not just this reader's own
    // hand-rolled one), unlike `intake.rs`'s own direct-reader unit tests.
    // `appliesTo` names the native scalar the value type binds
    // (`ix://quire/native/Integer`), not `VersionNumber`'s own identity:
    // `agent-ix-semantic-ir`'s own applicability table (`rules.rs::applies_to`)
    // only ever resolves a construct-kind node's own `kind`/`shape` to
    // `"construct"`, never `"scalar"` (no `Shape` variant means "scalar" at
    // this pinned rev) -- a constraint whose `appliesTo` named `VersionNumber`
    // itself would refuse `CONSTRAINT_NOT_APPLICABLE` even though this is
    // exactly the bound `VersionNumber` names. Naming the native scalar
    // directly is what actually resolves to `Resolved::Native("integer")`,
    // and is what `VALUE_TYPE`'s own meaning describes: "naming the value
    // type and its bound native value type".
    let constraint = |keyword: &str, value: i64| {
        json!({
            "identity": format!("{identity}/constraints/{keyword}"),
            "keyword": keyword,
            "operands": {"value": value},
            "appliesTo": "ix://quire/native/Integer",
            "diagnosticCode": format!("bound.{keyword}"),
            "origin": {
                "generated": {
                    "generatorIdentity": identity.clone(),
                    "generatorVersion": "1.0.0",
                    "inputIdentities": [identity.clone()],
                }
            },
        })
    };
    let construct = wire_construct("value_type", meaning::VALUE_TYPE, json!({}));
    let type_node = wire_type(
        &identity,
        "value_type",
        json!({
            "scalar": "integer",
            "constraints": [constraint("min", 0), constraint("max", 1000)],
        }),
    );
    (construct, type_node, identity)
}

/// The full `ConfigVersion` document: an object type with `versionNumber`
/// (required, the bound `VersionNumber` scalar -- [`version_number_value_type`])
/// and `parent` (optional, self-referencing), `attemptUpdate` as its one
/// operation (`operation` supplies the whole operation node), a
/// `config_history` population, and `extra_types`/`extra_constructs` for a
/// sibling declaration (a relationship or a record value type) some tests
/// need beside it.
pub(super) fn config_version_document(
    operation: Value,
    extra_constructs: Vec<Value>,
    extra_types: Vec<Value>,
    relationships: Value,
) -> Vec<u8> {
    let config_version = format!("ix://{PACKAGE_IDENTITY}/ConfigVersion");
    let (version_number_construct, version_number_type, version_number) =
        version_number_value_type();
    let mut constructs = vec![
        wire_construct("object_type", meaning::OBJECT_TYPE, json!({})),
        wire_construct("population", meaning::POPULATION, json!({})),
        version_number_construct,
    ];
    constructs.extend(extra_constructs);
    let mut types = vec![
        version_number_type,
        wire_type(
            &config_version,
            "object_type",
            json!({
                "supertypes": [],
                "fields": [
                    wire_field(
                        &config_version_identity("versionNumber"),
                        "versionNumber",
                        &version_number,
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
        ),
    ];
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

/// A field's declaration key under an arbitrary owner identity (not just
/// `ConfigVersion`), for a frame resolved against a sibling object type.
fn frame_key_of(owner: &str, name: &str) -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE_IDENTITY.to_owned(),
        node: format!("{owner}/{name}"),
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

/// [`config_unit`], with `body` (further declarations, e.g. FR-104's state
/// clauses) appended in place of the plain `noop` function it writes.
pub(super) fn config_unit_with_body(
    document: &[u8],
    body: &str,
) -> (String, BTreeMap<[u8; 32], Vec<u8>>) {
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
         {body}\n"
    );
    (unit, packages)
}

/// [`admit_and_assemble`], over [`config_unit_with_body`]'s unit, keeping
/// the structured [`qsl_semantics::check::AssemblyRefusal`] rather than
/// formatting it, so a caller can inspect each error's own catalog code.
pub(super) fn admit_and_assemble_with_body(
    document: &[u8],
    body: &str,
) -> Result<PackageDeclarations, qsl_semantics::check::AssemblyRefusal> {
    let (unit, packages) = config_unit_with_body(document, body);
    let built = parse_and_build(&unit);
    let models = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .unwrap_or_else(|refusal| panic!("I1 refused: {refusal:?}"));
    let raw = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "tc-459", "fixture", "fixture:1"),
        "unit.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit")
    .source()
    .reference()
    .clone();
    PackageDeclarations::assemble(raw, built, models, Vec::new())
}

/// A domain-package operation `name(params): <returns>` with `frame`, on
/// `ConfigVersion`, generalizing [`attempt_update`] to an arbitrary name and
/// an optional result (`returns: None` omits the `returns` member, FR-103's
/// "no result" operation).
pub(super) fn operation(name: &str, params: Value, returns: Option<&str>, frame: Value) -> Value {
    let identity = format!("ix://{PACKAGE_IDENTITY}/ConfigVersion/{name}");
    let mut value = json!({
        "identity": identity,
        "name": name,
        "params": params,
        "pre": [],
        "post": [],
        "origin": {
            "source": {
                "sourceIdentity": format!("ix://{PACKAGE_IDENTITY}/spec"),
                "path": "spec.qspec",
                "startLine": 1,
                "startColumn": 1,
            },
        },
        "frame": frame,
    });
    if let Some(type_ref) = returns {
        value["returns"] = json!({
            "typeRef": type_ref,
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "nullable": false,
        });
    }
    value
}

/// An operation parameter, generalizing [`parameter`] to an arbitrary
/// operation name.
pub(super) fn operation_parameter(operation: &str, name: &str, type_ref: &str) -> Value {
    let identity = format!("ix://{PACKAGE_IDENTITY}/ConfigVersion/{operation}/{name}");
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

/// An empty frame: `modifies`/`creates`/`deletes` all empty.
pub(super) fn empty_frame() -> Value {
    json!({"modifies": [], "creates": [], "deletes": []})
}

/// A second population over `ConfigVersion` named `archive` (TC-461 step
/// 5), beside `config_history`.
pub(super) fn archive_population() -> Value {
    let config_version = format!("ix://{PACKAGE_IDENTITY}/ConfigVersion");
    let population_identity = format!("ix://{PACKAGE_IDENTITY}/archive");
    json!({
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
    })
}

/// [`config_version_document`], with `operations` on `ConfigVersion`
/// instead of exactly one (SR-736 FND-001: two operations of one frame
/// content still key two distinct `operation-contract` records).
pub(super) fn config_version_document_with_operations(operations: Vec<Value>) -> Vec<u8> {
    let config_version = format!("ix://{PACKAGE_IDENTITY}/ConfigVersion");
    let (version_number_construct, version_number_type, version_number) =
        version_number_value_type();
    let constructs = vec![
        wire_construct("object_type", meaning::OBJECT_TYPE, json!({})),
        wire_construct("population", meaning::POPULATION, json!({})),
        version_number_construct,
    ];
    let types = vec![
        version_number_type,
        wire_type(
            &config_version,
            "object_type",
            json!({
                "supertypes": [],
                "fields": [
                    wire_field(
                        &config_version_identity("versionNumber"),
                        "versionNumber",
                        &version_number,
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
                "operations": operations,
                "relationships": [],
            }),
        ),
    ];
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

/// `ConfigVersion` (with `operation`, TC-458's own shape) and `Sub`, an
/// object type with `supertypes: [ConfigVersion]` and no operations or
/// population membership of its own (SR-723 FND-007, SR-736 FND-002): a
/// state clause `on Config::Sub` covers `config_history` only by
/// conformance, and `on Config::Sub::<name>` resolves the operation
/// `ConfigVersion` declares, keeping `ConfigVersion` as its declaring type.
pub(super) fn subtype_document(operation: Value) -> Vec<u8> {
    let config_version = format!("ix://{PACKAGE_IDENTITY}/ConfigVersion");
    let sub = format!("ix://{PACKAGE_IDENTITY}/Sub");
    let (version_number_construct, version_number_type, version_number) =
        version_number_value_type();
    let constructs = vec![
        wire_construct("object_type", meaning::OBJECT_TYPE, json!({})),
        wire_construct("population", meaning::POPULATION, json!({})),
        version_number_construct,
    ];
    let config_version_type = wire_type(
        &config_version,
        "object_type",
        json!({
            "supertypes": [],
            "fields": [
                wire_field(
                    &config_version_identity("versionNumber"),
                    "versionNumber",
                    &version_number,
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
            "relationships": [],
        }),
    );
    let sub_type = wire_type(
        &sub,
        "object_type",
        json!({
            "supertypes": [config_version],
            "fields": [],
            "operations": [],
            "relationships": [],
        }),
    );
    let population_identity = format!("ix://{PACKAGE_IDENTITY}/config_history");
    let population = json!({
        "identity": population_identity.clone(),
        "displayName": population_identity.clone(),
        "kind": {"module": PACKAGE_IDENTITY, "name": "population"},
        "members": [format!("ix://{PACKAGE_IDENTITY}/ConfigVersion")],
        "extent": "closed",
        "origin": {
            "generated": {
                "generatorIdentity": population_identity.clone(),
                "generatorVersion": "1.0.0",
                "inputIdentities": [population_identity],
            }
        },
    });
    wire_envelope(
        json!(constructs),
        json!([version_number_type, config_version_type, sub_type]),
        json!([population]),
    )
    .to_string()
    .into_bytes()
}

/// [`subtype_document`]'s own `ConfigVersion` and `Sub`, but `config_history`
/// declares *both* as its own member types (SR-736 FND-011), not `Sub` by
/// conformance alone: a population with two or more declared member types
/// still has one canonical `DomainKey`.
pub(super) fn subtype_document_with_two_member_population(operation: Value) -> Vec<u8> {
    let config_version = format!("ix://{PACKAGE_IDENTITY}/ConfigVersion");
    let sub = format!("ix://{PACKAGE_IDENTITY}/Sub");
    let (version_number_construct, version_number_type, version_number) =
        version_number_value_type();
    let constructs = vec![
        wire_construct("object_type", meaning::OBJECT_TYPE, json!({})),
        wire_construct("population", meaning::POPULATION, json!({})),
        version_number_construct,
    ];
    let config_version_type = wire_type(
        &config_version,
        "object_type",
        json!({
            "supertypes": [],
            "fields": [
                wire_field(
                    &config_version_identity("versionNumber"),
                    "versionNumber",
                    &version_number,
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
            "relationships": [],
        }),
    );
    let sub_type = wire_type(
        &sub,
        "object_type",
        json!({
            "supertypes": [config_version],
            "fields": [],
            "operations": [],
            "relationships": [],
        }),
    );
    let population_identity = format!("ix://{PACKAGE_IDENTITY}/config_history");
    let population = json!({
        "identity": population_identity.clone(),
        "displayName": population_identity.clone(),
        "kind": {"module": PACKAGE_IDENTITY, "name": "population"},
        "members": [
            format!("ix://{PACKAGE_IDENTITY}/Sub"),
            format!("ix://{PACKAGE_IDENTITY}/ConfigVersion"),
        ],
        "extent": "closed",
        "origin": {
            "generated": {
                "generatorIdentity": population_identity.clone(),
                "generatorVersion": "1.0.0",
                "inputIdentities": [population_identity],
            }
        },
    });
    wire_envelope(
        json!(constructs),
        json!([version_number_type, config_version_type, sub_type]),
        json!([population]),
    )
    .to_string()
    .into_bytes()
}

/// Two unrelated object types `Left` and `Right`, each declaring its own
/// `dup(): Boolean` operation with an empty frame, and `Both`, whose
/// `supertypes: [Left, Right]` inherits both (SR-736 FND-006): neither
/// declaring type is more derived than the other, so `Config::Both::dup`
/// is ambiguous.
pub(super) fn ambiguous_operation_document() -> Vec<u8> {
    let left = format!("ix://{PACKAGE_IDENTITY}/Left");
    let right = format!("ix://{PACKAGE_IDENTITY}/Right");
    let both = format!("ix://{PACKAGE_IDENTITY}/Both");
    let dup = |owner: &str| {
        json!({
            "identity": format!("{owner}/dup"),
            "name": "dup",
            "params": [],
            "returns": {
                "typeRef": "ix://quire/native/Boolean",
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                "nullable": false,
            },
            "pre": [],
            "post": [],
            "origin": {
                "source": {
                    "sourceIdentity": format!("ix://{PACKAGE_IDENTITY}/spec"),
                    "path": "spec.qspec",
                    "startLine": 1,
                    "startColumn": 1,
                },
            },
            "frame": {"modifies": [], "creates": [], "deletes": []},
        })
    };
    let constructs = vec![wire_construct(
        "object_type",
        meaning::OBJECT_TYPE,
        json!({}),
    )];
    let left_type = wire_type(
        &left,
        "object_type",
        json!({
            "supertypes": [],
            "fields": [],
            "operations": [dup(&left)],
            "relationships": [],
        }),
    );
    let right_type = wire_type(
        &right,
        "object_type",
        json!({
            "supertypes": [],
            "fields": [],
            "operations": [dup(&right)],
            "relationships": [],
        }),
    );
    let both_type = wire_type(
        &both,
        "object_type",
        json!({
            "supertypes": [left, right],
            "fields": [],
            "operations": [],
            "relationships": [],
        }),
    );
    wire_envelope(
        json!(constructs),
        json!([left_type, right_type, both_type]),
        json!([]),
    )
    .to_string()
    .into_bytes()
}

/// [`config_version_document`], with an extra `populations[]` entry
/// appended (TC-461 step 5).
pub(super) fn config_version_document_with_population(
    operation: Value,
    extra_population: Value,
) -> Vec<u8> {
    let document = config_version_document(operation, Vec::new(), Vec::new(), json!([]));
    let mut envelope: Value = serde_json::from_slice(&document).expect("valid JSON");
    envelope["populations"]
        .as_array_mut()
        .expect("populations is an array")
        .push(extra_population);
    serde_json::to_vec(&envelope).expect("valid JSON")
}

pub(super) fn parse_and_build(unit: &str) -> qsl_forms::ParsedUnit {
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
/// `Config::ConfigVersion` has `versionNumber: Int[0, 1000]` (required) and
/// `parent: Reference<Config::ConfigVersion>` (optional), and `attemptUpdate`
/// has no parameters, result `Boolean`, and an effect whose `modifies` is
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
    assert_eq!(version_number.value_type(), &version_number_bound());
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

    let operation = declarations
        .operations
        .declared_by(config_version_id)
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

/// FR-103-AC-1's own bound-scalar half, over a small dedicated fixture
/// independent of `ConfigVersion`: the same package-declared `VersionNumber`
/// value type ([`version_number_value_type`]) and an object type `Widget`
/// with one required field of that type -- proving FR-056's `value-type/v1`
/// scalar reader all the way through this file's own
/// `admit_unit`/`PackageDeclarations::assemble` pipeline with no other
/// declaration beside it: `Widget.num` assembles to `Int[0, 1000]`.
#[trace("TC-458", "FR-103-AC-1")]
#[test]
fn bound_integer_value_type_admits_and_assembles() {
    let (version_number_construct, version_number_type, version_number) =
        version_number_value_type();
    let widget = format!("ix://{PACKAGE_IDENTITY}/Widget");
    let document = wire_envelope(
        json!([
            wire_construct("object_type", meaning::OBJECT_TYPE, json!({})),
            version_number_construct,
        ]),
        json!([
            version_number_type,
            wire_type(
                &widget,
                "object_type",
                json!({
                    "supertypes": [],
                    "operations": [],
                    "relationships": [],
                    "fields": [wire_field(
                        &format!("{widget}/num"),
                        "num",
                        &version_number,
                        "required",
                        1,
                    )],
                }),
            ),
        ]),
        json!([]),
    )
    .to_string()
    .into_bytes();
    let declarations =
        admit_and_assemble(&document).expect("the bound-scalar fixture admits and assembles");

    let (unit, packages) = config_unit(&document);
    let built = parse_and_build(&unit);
    let views = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the package admits");
    let widget_key = DeclarationKey {
        package: PACKAGE_IDENTITY.to_owned(),
        node: widget.clone(),
    };
    let widget_id = views[0].view.type_identities()[&widget_key];
    let declared = declarations
        .types
        .object_type(widget_id)
        .expect("Widget is declared");
    let num = declared
        .attributes()
        .iter()
        .find(|field| field.name() == "num")
        .expect("num is an attribute");
    assert_eq!(num.value_type(), &version_number_bound());
    assert_eq!(num.presence(), Presence::Required);
}

/// FR-103-AC-2 (TC-458 step 2, part 1): `modifies` naming no declaration of
/// the package refuses `missing_declaration`/`missing-name` at that entry,
/// and no declaration is admitted.
///
/// The pinned FCD schema validator (`agent-ix-semantic-ir`, rev `033e228`)
/// has its own, coarser frame-path resolution rule
/// (`UNRESOLVED_FRAME_PATH`), which would otherwise preempt this reader's
/// own, strictly finer classification (`resolve_pending_frames`) for every
/// case this AC and FND-003/SR-722 draw a real distinction over --
/// `validate_with_semantic_ir` filters that one diagnostic out so this
/// reader's own classification always decides (see its own doc comment).
#[trace("TC-458", "FR-103-AC-2")]
#[test]
fn a_modifies_entry_naming_no_declaration_refuses_missing_declaration() {
    let missing = config_version_identity("missing");
    let document = config_version_document(
        attempt_update(
            json!([]),
            json!({
                "modifies": [missing.clone()],
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
    assert_eq!(refusals[0].code.as_str(), "missing_declaration");
    assert_eq!(refusals[0].cause.as_str(), "missing-name");
    let qsl_semantics::model::refusal::ModelRefusalCause::FrameEntryMissing {
        node,
        entry,
        span,
        ..
    } = &refusals[0].cause
    else {
        panic!("{:?}: expected FrameEntryMissing", refusals[0].cause);
    };
    assert_eq!(node, &config_version_identity("attemptUpdate"));
    assert_eq!(entry, &missing);
    assert!(span.is_some(), "the operation's own source span is carried");
}

/// FR-103-AC-2 (TC-458 step 2, part 2): `modifies` naming an object type
/// (not a field) refuses `invalid_model_binding`/`malformed-declaration` at
/// that entry.
#[trace("TC-458", "FR-103-AC-2")]
#[test]
fn a_modifies_entry_naming_an_object_type_refuses_malformed_declaration() {
    let config_version = format!("ix://{PACKAGE_IDENTITY}/ConfigVersion");
    let document = config_version_document(
        attempt_update(
            json!([]),
            json!({
                "modifies": [config_version.clone()],
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
    let qsl_semantics::model::refusal::ModelRefusalCause::FrameEntryMalformed {
        node, entry, ..
    } = &refusals[0].cause
    else {
        panic!("{:?}: expected FrameEntryMalformed", refusals[0].cause);
    };
    assert_eq!(node, &config_version_identity("attemptUpdate"));
    assert_eq!(entry, &config_version);
}

/// FR-103-AC-2 (TC-458 step 2, part 3): `creates` naming a field refuses
/// `invalid_model_binding`/`malformed-declaration` at that entry.
#[trace("TC-458", "FR-103-AC-2")]
#[test]
fn a_creates_entry_naming_a_field_refuses_malformed_declaration() {
    let version_number = config_version_identity("versionNumber");
    let document = config_version_document(
        attempt_update(
            json!([]),
            json!({
                "modifies": [],
                "creates": [version_number.clone()],
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
    let qsl_semantics::model::refusal::ModelRefusalCause::FrameEntryMalformed {
        node, entry, ..
    } = &refusals[0].cause
    else {
        panic!("{:?}: expected FrameEntryMalformed", refusals[0].cause);
    };
    assert_eq!(node, &config_version_identity("attemptUpdate"));
    assert_eq!(entry, &version_number);
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
                "modifies": [supersedes.clone()],
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
    let qsl_semantics::model::refusal::ModelRefusalCause::FrameEntryUnsupported {
        node, entry, ..
    } = &refusals[0].cause
    else {
        panic!("{:?}: expected FrameEntryUnsupported", refusals[0].cause);
    };
    assert_eq!(node, &config_version_identity("attemptUpdate"));
    assert_eq!(entry, &supersedes);
}

/// FR-103-AC-2 (TC-458 step 2, part 5): with both `modifies` naming a
/// missing declaration and `creates` naming a field, only the `modifies`
/// refusal is reported (document order: `modifies`, then `creates`, then
/// `deletes`; `resolve_pending_frames` checks each member in that order and
/// stops at the first that fails).
#[trace("TC-458", "FR-103-AC-2")]
#[test]
fn only_the_modifies_refusal_is_reported_when_both_members_are_invalid() {
    let missing = config_version_identity("missing");
    let version_number = config_version_identity("versionNumber");
    let document = config_version_document(
        attempt_update(
            json!([]),
            json!({
                "modifies": [missing.clone()],
                "creates": [version_number],
                "deletes": [],
            }),
        ),
        Vec::new(),
        Vec::new(),
        json!([]),
    );
    let refusal = admit(&document).expect_err("modifies still refuses first");
    let UnitIntakeCause::Refused(refusals) = refusal.cause else {
        panic!("{refusal:?}: expected a Refused cause");
    };
    assert_eq!(refusals.len(), 1, "{refusals:?}: only one refusal reported");
    assert_eq!(refusals[0].code.as_str(), "missing_declaration");
    assert_eq!(refusals[0].cause.as_str(), "missing-name");
    let qsl_semantics::model::refusal::ModelRefusalCause::FrameEntryMissing { entry, .. } =
        &refusals[0].cause
    else {
        panic!("{:?}: expected FrameEntryMissing", refusals[0].cause);
    };
    assert_eq!(
        entry, &missing,
        "the reported entry is modifies', not creates'"
    );
}

/// FR-103-AC-3 (TC-458 step 3, part 1, as amended -- SR-723 gap-analysis
/// FND-003): a parameter typed `Decimal`, not `Text`, refuses
/// `unsupported_construct`/`declaration-form` at I1 (intake), the same
/// stage and cause a field of that type refuses at (FR-103's own Behavior:
/// "as a field of that type does"). `NativeValueType::unsupported_parameters`
/// is non-empty for `Decimal` (`qsl-semantics/src/model/domain_package.rs`:
/// needs `dmin`/`dmax`), so `read_value_type_ref`
/// (`qsl-semantics/src/model/intake.rs`) already refuses it at I1 for
/// *every* caller (a field, an operation parameter or an operation result
/// alike), before the assembled `OperationMemberRecord`/`FieldMemberRecord`
/// this parameter would produce ever exists for
/// `PackageDeclarations::assemble` to see -- `model_value_type`'s own
/// `NativeValueType::Rational | Decimal | Text => Unmapped::Unsupported`
/// arm (`qsl-semantics/src/check/assemble.rs`) is unreachable dead code for
/// a package-declared operation parameter/result.
///
/// `Text` itself cannot be used through the real pipeline at all: the
/// pinned FCD schema validator's own native-scalar vocabulary
/// (`agent-ix-semantic-ir`, rev `033e228`,
/// `crates/semantic-ir/src/rules.rs`'s `NATIVE_SCALARS`) has no `Text`
/// entry -- it spells the same concept `String` -- so
/// `validate_with_semantic_ir` refuses any `ix://quire/native/Text`
/// `typeRef` as `UNRESOLVED_TYPE_REF` before this reader's own
/// `read_value_type_ref` ever runs; `ix://quire/native/String` does resolve
/// at FCD's layer, but QSL's own `NativeValueType::from_str` has no
/// `String` variant (only `Text`), so it would refuse there instead, with a
/// different cause (`malformed-declaration`). That cross-repo vocabulary
/// gap is **QSL-290**, not fixed by this ticket; `Decimal` is the closest
/// substitute both vocabularies recognize by the same name.
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

/// FR-103-AC-3 (TC-458 step 3, part 2): a parameter typed `VersionNumber`
/// admits it typed the bound `Int[0, 1000]`, alongside `attemptUpdate`'s
/// existing no-op frame.
#[trace("TC-458", "FR-103-AC-3")]
#[test]
fn a_version_number_typed_parameter_admits_and_is_typed_bound_integer() {
    // `config_version_document` already declares `VersionNumber`
    // (`version_number_value_type`) for `versionNumber` itself; `delta`
    // reuses that same declaration by identity rather than redeclaring it.
    let version_number = version_number_identity();
    let document = config_version_document(
        attempt_update(
            json!([parameter("delta", &version_number)]),
            json!({"modifies": [], "creates": [], "deletes": []}),
        ),
        Vec::new(),
        Vec::new(),
        json!([]),
    );
    let declarations = admit_and_assemble(&document).expect("a VersionNumber parameter admits");
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
    declarations
        .types
        .object_type(config_version_id)
        .expect("ConfigVersion is declared");
    let operation = declarations
        .operations
        .declared_by(config_version_id)
        .iter()
        .find(|operation| operation.name() == "attemptUpdate")
        .expect("attemptUpdate is declared");
    assert_eq!(
        operation.parameters(),
        &[("delta".to_owned(), version_number_bound())]
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
    // A genuinely reordered document (SR-722 FND-002): a second object type
    // (`AuditLog`) the operation's own frame names, placed after
    // `ConfigVersion` in one document and before it in the other, so the
    // frame resolves a cross-type forward reference in one ordering and a
    // backward one in the other -- proving `resolve_pending_frames`'
    // two-phase design (queue while reading, resolve once every node of the
    // document is read) rather than a no-op comparison of a document with
    // itself.
    let audit_log = format!("ix://{PACKAGE_IDENTITY}/AuditLog");
    let audit_note = format!("{audit_log}/note");
    let audit_type = wire_type(
        &audit_log,
        "object_type",
        json!({
            "supertypes": [],
            "fields": [wire_field(
                &audit_note,
                "note",
                "ix://quire/native/Integer",
                "required",
                1,
            )],
            "operations": [],
            "relationships": [],
        }),
    );
    let frame = json!({
        "modifies": [audit_note],
        "creates": [],
        "deletes": [],
    });
    let config_version = format!("ix://{PACKAGE_IDENTITY}/ConfigVersion");
    let (version_number_construct, version_number_type, version_number) =
        version_number_value_type();
    let config_version_type = wire_type(
        &config_version,
        "object_type",
        json!({
            "supertypes": [],
            "fields": [
                wire_field(
                    &config_version_identity("versionNumber"),
                    "versionNumber",
                    &version_number,
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
            "operations": [attempt_update(json!([]), frame)],
            "relationships": [],
        }),
    );
    let constructs = json!([
        wire_construct("object_type", meaning::OBJECT_TYPE, json!({})),
        wire_construct("population", meaning::POPULATION, json!({})),
        version_number_construct,
    ]);
    let population_identity = format!("ix://{PACKAGE_IDENTITY}/config_history");
    let population = json!({
        "identity": population_identity.clone(),
        "displayName": population_identity.clone(),
        "kind": {"module": PACKAGE_IDENTITY, "name": "population"},
        "members": [config_version.clone()],
        "extent": "closed",
        "origin": {
            "generated": {
                "generatorIdentity": population_identity.clone(),
                "generatorVersion": "1.0.0",
                "inputIdentities": [population_identity],
            }
        },
    });
    let forward = wire_envelope(
        constructs.clone(),
        json!([
            version_number_type.clone(),
            config_version_type.clone(),
            audit_type.clone()
        ]),
        json!([population.clone()]),
    )
    .to_string()
    .into_bytes();
    let backward = wire_envelope(
        constructs,
        json!([audit_type, config_version_type, version_number_type]),
        json!([population]),
    )
    .to_string()
    .into_bytes();
    assert_ne!(
        forward, backward,
        "the two documents are genuinely reordered"
    );

    type Assembled = (ObjectTypeDeclaration, Vec<OperationDeclaration>);
    let assembled_of = |document: &[u8]| -> (Assembled, Assembled) {
        let declarations = admit_and_assemble(document).expect("the fixture admits and assembles");
        let (unit, packages) = config_unit(document);
        let built = parse_and_build(&unit);
        let views = admit_unit(
            &built.selections().models,
            &packages,
            ModelNormalizationLimits::default(),
        )
        .expect("the package admits");
        let identities = views[0].view.type_identities();
        let config_version_id = identities[&DeclarationKey {
            package: PACKAGE_IDENTITY.to_owned(),
            node: config_version.clone(),
        }];
        let audit_log_id = identities[&DeclarationKey {
            package: PACKAGE_IDENTITY.to_owned(),
            node: audit_log.clone(),
        }];
        let declared = |id| {
            (
                declarations
                    .types
                    .object_type(id)
                    .expect("the object type is declared")
                    .clone(),
                declarations.operations.declared_by(id).to_vec(),
            )
        };
        (declared(config_version_id), declared(audit_log_id))
    };

    let (config_version_a, audit_log_a) = assembled_of(&forward);
    let (config_version_b, audit_log_b) = assembled_of(&backward);
    assert_eq!(
        config_version_a, config_version_b,
        "ConfigVersion's assembled declaration (fields, operations, effect) agrees regardless \
         of document order"
    );
    assert_eq!(
        audit_log_a, audit_log_b,
        "AuditLog's assembled declaration agrees regardless of document order"
    );

    let effect = |(_, operations): &Assembled| {
        operations
            .iter()
            .find(|operation| operation.name() == "attemptUpdate")
            .expect("attemptUpdate is declared")
            .effect()
            .clone()
    };
    let resolved = effect(&config_version_a);
    assert_eq!(resolved.modifies, vec![frame_key_of(&audit_log, "note")]);
    assert_eq!(resolved.creates, Vec::new());
    assert_eq!(resolved.deletes, Vec::new());
}

/// SR-766 FND-001: `PackageDeclarations::alias_names`'s model-alias half
/// (assembled from `selections.models` -- the `.chain(selections.models
/// ...)` line -- not just `selections.profiles`), which FR-113's binder
/// no-shadowing rule reads to refuse a binder naming a model
/// alias the same way it refuses one naming a profile alias. Every other
/// shadowing test in `qsl-semantics::check::protocol_clause` only ever
/// exercises the profile-alias half (a fixture's own `v`), since that
/// module's own `assemble` test helper admits no real domain package;
/// deleting the model-alias half of `alias_names`'s assembly would leave
/// every one of those green while this one goes red.
#[trace("FR-113")]
#[test]
fn a_models_own_alias_is_recorded_in_the_packages_alias_names() {
    let document = config_version_document(
        attempt_update(
            json!([]),
            json!({"modifies": [], "creates": [], "deletes": []}),
        ),
        Vec::new(),
        Vec::new(),
        json!([]),
    );
    let declarations = admit_and_assemble(&document).expect("the fixture admits and assembles");
    assert_eq!(
        declarations.alias_names.get("Config"),
        Some(&qsl_semantics::check::AliasKind::Model),
        "{:?}",
        declarations.alias_names
    );
}
