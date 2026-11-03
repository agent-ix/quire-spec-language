// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1301: FR-506's bounds on model admission, through the spine's model
//! intake (`admit_unit`): the `model.declaration_records` charge rule, the
//! raisable `model.sequence_maximum`, the Text `maxLength` ceiling, the
//! document byte limit and the absence of a nesting-depth limit.
//!
//! The package is the `example/config-version` fixture of TC-458, with the
//! members each test varies written into its JSON.

use ix_trace_rs::trace;
use qsl_semantics::model::accounting::{LimitKind, ModelNormalizationLimits};
use qsl_semantics::model::intake::{admit_unit, UnitIntakeCause, UnitIntakeRefusal};
use qsl_semantics::model::normalize::ModelRefusal;
use qsl_semantics::model::refusal::ModelRefusalCause;
use serde_json::{json, Value};

use super::model_operations::{
    attempt_update, config_unit, config_version_document, config_version_identity, parse_and_build,
    wire_type, PACKAGE_IDENTITY,
};

fn run(document: &[u8], limits: ModelNormalizationLimits) -> Result<(), Box<UnitIntakeRefusal>> {
    let (unit, packages) = config_unit(document);
    let built = parse_and_build(&unit);
    admit_unit(&built.selections().models, &packages, limits)
        .map(|_| ())
        .map_err(Box::new)
}

fn with_sequence_maximum(bound: u32) -> ModelNormalizationLimits {
    ModelNormalizationLimits {
        intake: qsl_foundation::IntakeLimits::default().with_sequence_maximum(bound),
        ..ModelNormalizationLimits::default()
    }
}

/// `attemptUpdate(p)` with a `modifies`, a `creates` and a `deletes` entry.
fn framed_operation() -> Value {
    let identity = format!("ix://{PACKAGE_IDENTITY}/ConfigVersion/attemptUpdate/p");
    attempt_update(
        json!([{
            "identity": identity,
            "name": "p",
            "typeRef": "ix://quire/native/Boolean",
            "presence": "required",
            "nullable": false,
            "defaultKind": "none",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "origin": {"generated": {
                "generatorIdentity": identity,
                "generatorVersion": "1.0.0",
                "inputIdentities": [identity],
            }},
        }]),
        json!({
            "modifies": [config_version_identity("versionNumber")],
            "creates": [format!("ix://{PACKAGE_IDENTITY}/ConfigVersion")],
            "deletes": [format!("ix://{PACKAGE_IDENTITY}/ConfigVersion")],
        }),
    )
}

fn document_of(operation: Value) -> Value {
    serde_json::from_slice(&config_version_document(
        operation,
        vec![],
        vec![],
        json!([]),
    ))
    .expect("the fixture is JSON")
}

fn bytes(document: &Value) -> Vec<u8> {
    serde_json::to_vec(document).expect("valid JSON")
}

fn config_version_mut(document: &mut Value) -> &mut Value {
    let identity = format!("ix://{PACKAGE_IDENTITY}/ConfigVersion");
    document["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|entry| entry["identity"] == identity.as_str())
        .expect("ConfigVersion is declared")
}

fn refused(refusal: &UnitIntakeRefusal) -> &ModelRefusal {
    match &refusal.cause {
        UnitIntakeCause::Refused(refusals) => &refusals[0],
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn refused_cause(refusal: &UnitIntakeRefusal) -> &ModelRefusalCause {
    &refused(refusal).cause
}

/// FR-506-AC-1 (TC-1301 rows 1 to 3): the fixture declares 3 types (the
/// scalar `VersionNumber`, `ConfigVersion`, the population `config_history`),
/// 2 fields, 1 operation, 1 parameter, 1 result and 3 frame entries: 11
/// records. The limit 11 admits, 10 refuses at `model.declaration_records`
/// and 0 refuses. Before the charge rule counted parameters, results and
/// frame entries the count was 6, so 10 admitted.
#[trace(
    "FR-506-AC-1",
    "FR-506-AC-2",
    "FR-506-AC-3",
    "FR-506-AC-4",
    "FR-506-AC-5",
    "FR-506-AC-6",
    "FR-506-AC-7",
    "FR-506-AC-8",
    "TC-1301"
)]
#[test]
fn declaration_records_charge_each_type_field_operation_parameter_result_and_frame_entry() {
    let document = bytes(&document_of(framed_operation()));
    let limited = |bound: u64| ModelNormalizationLimits {
        declaration_records: bound,
        ..ModelNormalizationLimits::default()
    };
    run(&document, limited(11)).expect("11 records fit a limit of 11");
    for bound in [10, 0] {
        let refusal = run(&document, limited(bound)).expect_err("the records exceed the limit");
        match refusal.cause {
            UnitIntakeCause::Limit(incomplete) => {
                assert_eq!(incomplete.limit_kind, LimitKind::DeclarationRecords);
                assert_eq!(incomplete.limit, bound);
                assert_eq!(incomplete.next_charge, bound + 1);
            }
            other => panic!("expected a denied declaration_records charge, got {other:?}"),
        }
    }
}

/// The multiplicity `upper` of the first field, a parameter, the result and
/// a field of an unused type, each set to `upper`.
fn documents_with_maximum(upper: u64) -> Vec<(&'static str, Vec<u8>)> {
    let multiplicity = json!({"lower": 0, "upper": upper, "ordered": false, "unique": true});
    let mut field = document_of(framed_operation());
    config_version_mut(&mut field)["fields"][0]["multiplicity"] = multiplicity.clone();
    let mut parameter = document_of(framed_operation());
    config_version_mut(&mut parameter)["operations"][0]["params"][0]["multiplicity"] =
        multiplicity.clone();
    let mut result = document_of(framed_operation());
    config_version_mut(&mut result)["operations"][0]["returns"]["multiplicity"] =
        multiplicity.clone();
    let mut unused = document_of(framed_operation());
    let mut unused_type = config_version_mut(&mut unused).clone();
    let unused_identity = format!("ix://{PACKAGE_IDENTITY}/Unused");
    unused_type["identity"] = json!(unused_identity);
    unused_type["displayName"] = json!(unused_identity);
    unused_type["operations"] = json!([]);
    unused_type["fields"] = json!([{
        "identity": format!("{unused_identity}/items"),
        "name": "items",
        "typeRef": "ix://quire/native/Boolean",
        "presence": "optional",
        "nullable": false,
        "defaultKind": "none",
        "multiplicity": multiplicity,
        "origin": {"generated": {
            "generatorIdentity": unused_identity,
            "generatorVersion": "1.0.0",
            "inputIdentities": [unused_identity],
        }},
    }]);
    unused["types"]
        .as_array_mut()
        .expect("types")
        .push(unused_type);
    vec![
        ("a field", bytes(&field)),
        ("a parameter", bytes(&parameter)),
        ("a result", bytes(&result)),
        ("an unused type's field", bytes(&unused)),
    ]
}

/// FR-506-AC-2, AC-3 and AC-7 (TC-1301 rows 4 to 8 and 14, the library
/// builder): a maximum of 10,000 admits at the default and 10,001 refuses
/// `resource_exhausted`/`intake-limit-exceeded` carrying 10,000, 10,001 and
/// `model.sequence_maximum` in each position, the unused type included; the
/// limit 10,001 admits all; 0 refuses; 3 admits 3 and refuses 4.
#[trace("FR-506-AC-2", "FR-506-AC-3", "FR-506-AC-7", "FR-506-AC-8", "TC-1301")]
#[test]
fn a_declared_collection_maximum_is_bounded_by_the_raisable_sequence_maximum() {
    for (position, at_limit) in documents_with_maximum(10_000) {
        run(&at_limit, ModelNormalizationLimits::default())
            .unwrap_or_else(|refusal| panic!("{position} at 10,000 admits: {refusal:?}"));
    }
    for (position, over) in documents_with_maximum(10_001) {
        let refusal = run(&over, ModelNormalizationLimits::default())
            .expect_err("10,001 exceeds the default");
        let cause = refused_cause(&refusal);
        let catalog = cause.catalog_code();
        assert_eq!(
            (catalog.code(), catalog.cause()),
            ("resource_exhausted", "intake-limit-exceeded"),
            "{position}"
        );
        assert_eq!(
            refused(&refusal).code.category().exit_code(),
            22,
            "{position}"
        );
        let fields = cause.catalog_fields().expect("a key-table row");
        assert_eq!(fields["bound"], "10000", "{position}");
        assert_eq!(fields["actual"], "10001", "{position}");
        assert_eq!(fields["setting"], "model.sequence_maximum", "{position}");
        assert_eq!(fields["kind"], "node-count-exceeded", "{position}");
        run(&over, with_sequence_maximum(10_001))
            .unwrap_or_else(|refusal| panic!("{position} admits once raised: {refusal:?}"));
        run(&over, with_sequence_maximum(0)).expect_err("0 never disables the limit");
    }
    for (position, at_three) in documents_with_maximum(3) {
        run(&at_three, with_sequence_maximum(3))
            .unwrap_or_else(|refusal| panic!("{position}: 3 admits at 3: {refusal:?}"));
    }
    for (position, four) in documents_with_maximum(4) {
        let refusal = run(&four, with_sequence_maximum(3)).expect_err("4 exceeds 3");
        let fields = refused_cause(&refusal).catalog_fields().expect("fields");
        assert_eq!(
            (fields["bound"].as_str(), fields["actual"].as_str()),
            ("3", "4"),
            "{position}"
        );
    }
    let widest = u64::from(u32::MAX);
    for (position, widest_document) in documents_with_maximum(widest) {
        run(&widest_document, with_sequence_maximum(u32::MAX))
            .unwrap_or_else(|refusal| panic!("{position}: u32::MAX admits: {refusal:?}"));
    }
}

fn text_document(max_length: u64) -> Vec<u8> {
    let mut document = document_of(framed_operation());
    let identity = format!("ix://{PACKAGE_IDENTITY}/Label");
    let constraint = |keyword: &str, operands: Value| {
        json!({
            "identity": format!("{identity}/constraints/{keyword}"),
            "keyword": keyword,
            "operands": operands,
            "appliesTo": identity,
            "diagnosticCode": format!("text.{keyword}"),
            "origin": {"generated": {
                "generatorIdentity": identity,
                "generatorVersion": "1.0.0",
                "inputIdentities": [identity],
            }},
        })
    };
    let label = wire_type(
        &identity,
        "value_type",
        json!({
            "scalar": "string",
            "constraints": [
                constraint("minLength", json!({"value": 0})),
                constraint("maxLength", json!({"value": max_length})),
                constraint("format", json!({"name": "text-profile:unicode-scalars"})),
            ],
        }),
    );
    document["types"].as_array_mut().expect("types").push(label);
    bytes(&document)
}

/// FR-506-AC-4 (TC-1301 row 9): a Text `maxLength` of 1,048,576 scalars is
/// at the Contract IR ceiling and 1,048,577 is over it, whatever the limits.
#[trace("FR-506-AC-4", "TC-1301")]
#[test]
fn a_text_max_length_above_the_contract_ir_ceiling_is_refused() {
    let at_ceiling = run(
        &text_document(1_048_576),
        ModelNormalizationLimits::default(),
    );
    if let Err(refusal) = &at_ceiling {
        let detail = format!("{refusal:?}");
        assert!(
            !detail.contains("ceiling"),
            "the ceiling itself is not refused for exceeding it: {detail}"
        );
    }
    let refusal = run(
        &text_document(1_048_577),
        ModelNormalizationLimits::default(),
    )
    .expect_err("1,048,577 exceeds the ceiling");
    let cause = refused_cause(&refusal);
    let catalog = cause.catalog_code();
    assert_eq!(
        (catalog.code(), catalog.cause()),
        ("unsupported_construct", "declaration-form")
    );
    assert!(format!("{refusal:?}").contains("1048576"), "{refusal:?}");
}

/// FR-506-AC-5, intake part (TC-1301 row 10): a document of exactly the
/// `intake.input_bytes` limit admits, one byte short refuses
/// `resource_exhausted` naming `intake.input_bytes`, and 0 refuses.
#[trace("FR-506-AC-5", "TC-1301")]
#[test]
fn the_document_byte_limit_is_exact_and_names_its_setting() {
    let document = bytes(&document_of(framed_operation()));
    let exact = u64::try_from(document.len()).expect("fits");
    let limited = |bound: u64| ModelNormalizationLimits {
        intake: qsl_foundation::IntakeLimits::default().with_input_bytes(bound),
        ..ModelNormalizationLimits::default()
    };
    run(&document, limited(exact)).expect("the exact size admits");
    for bound in [exact - 1, 0] {
        let refusal = run(&document, limited(bound)).expect_err("over the byte limit");
        let fields = refused_cause(&refusal).catalog_fields().expect("fields");
        assert_eq!(fields["setting"], "intake.input_bytes");
        assert_eq!(fields["bound"], bound.to_string());
        assert_eq!(fields["actual"], exact.to_string());
        assert!(!format!("{refusal:?}").contains("limit exceeded"));
    }
}

/// FR-506-AC-6 (TC-1301 row 13): option and sequence types nested 200 levels deep admit
/// at the default limits; depth is not a limit (ADR-030 D-1).
#[trace("FR-506-AC-6", "TC-1301")]
#[test]
fn nesting_depth_is_no_limit() {
    let mut document = document_of(framed_operation());
    let mut inner = "ix://quire/native/Boolean".to_owned();
    for level in 0..200 {
        let identity = format!("ix://{PACKAGE_IDENTITY}/Opt{level}");
        // Options and sequences alternate: an option never wraps an option.
        let (kind, member) = if level % 2 == 0 {
            ("option", "inner")
        } else {
            ("sequence", "items")
        };
        let mut option = json!({
            "identity": identity,
            "displayName": identity,
            "kind": kind,
            "roles": [],
            "origin": {"generated": {
                "generatorIdentity": identity,
                "generatorVersion": "1.0.0",
                "inputIdentities": [identity],
            }},
            "constraints": [],
            "extensions": [],
            "unknownPolicy": "reject",
        });
        option[member] = json!(inner);
        document["types"]
            .as_array_mut()
            .expect("types")
            .push(option);
        inner = identity;
    }
    run(&bytes(&document), ModelNormalizationLimits::default())
        .unwrap_or_else(|refusal| panic!("200 nested options and sequences admit: {refusal:?}"));
}
