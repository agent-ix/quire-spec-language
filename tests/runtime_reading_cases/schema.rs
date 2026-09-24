// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-024: structural schema checks do not replace byte or runtime admission.

use super::*;
use jsonschema::{Draft, JSONSchema};
use quire_spec_language::runtime::{SnapshotDraft, ValueBinding};
use std::collections::BTreeSet;

fn schema() -> JSONSchema {
    let document: Value = serde_json::from_str(include_str!(
        "../../schemas/native-state-input-1.schema.json"
    ))
    .unwrap();
    JSONSchema::options()
        .with_draft(Draft::Draft202012)
        .compile(&document)
        .expect("the local runtime schema must compile without external resolution")
}

/// Whether the local schema accepts `value`.
pub(super) fn validates(value: &Value) -> bool {
    schema().is_valid(value)
}

fn populated_inputs() -> RuntimeInput {
    let model = setup::native_rule_model::parts().model();
    let mut draft = full_snapshot().draft().clone();
    draft.values.push(ValueBinding {
        declaration: setup::qualified(&model, "argument"),
        value: ValueId::new(0),
    });
    let mut input = setup::input(setup::snapshot(draft.clone()));
    for result in [Some(ValueId::new(0)), None] {
        let (mut recorded, _) = setup::recorded(&model, draft.clone(), draft.clone(), |draft| {
            draft.parameters.push(ValueBinding {
                declaration: setup::qualified(&model, "argument"),
                value: ValueId::new(0),
            });
            draft.created.push(setup::object(&model, "created"));
            draft.deleted.push(setup::object(&model, "deleted"));
            draft.result = result;
        });
        input.snapshots.append(&mut recorded.snapshots);
        input.invocations.append(&mut recorded.invocations);
    }
    input
}

fn envelopes(input: &RuntimeInput) -> impl Iterator<Item = Value> + '_ {
    input
        .snapshots
        .iter()
        .map(Snapshot::bytes)
        .chain(input.invocations.iter().map(Invocation::bytes))
        .map(|bytes| serde_json::from_slice(bytes).unwrap())
}

#[test]
#[trace("TC-099", "FR-024-AC-5")]
fn schema_accepts_real_snapshots_and_invocations_with_all_value_variants() {
    let schema = schema();
    let input = populated_inputs();
    let mut observations = BTreeSet::new();
    let mut kinds = BTreeSet::new();
    for original in &input.snapshots {
        let wire: Value = serde_json::from_slice(original.bytes()).unwrap();
        assert!(schema.is_valid(&wire));
        observations.insert(wire["body"]["observation"].as_str().unwrap().to_owned());
        for value in wire["body"]["arena"].as_array().unwrap() {
            kinds.insert(value["kind"].as_str().unwrap().to_owned());
        }
        assert!(!original.draft().values.is_empty());
        let objects = &original.draft().populations[0].objects;
        assert_eq!(objects.len(), 2);
        assert_eq!(objects[0], objects[1]);
        assert_eq!(objects[0].fields.first(), objects[0].fields.last());
        assert_eq!(
            original.draft().arena.last().unwrap(),
            &ValueNode::Sequence {
                values: vec![ValueId::new(0); 2]
            }
        );
        let read = Snapshot::read_verified(
            &original.reference(),
            original.bytes(),
            ArtifactLimits::default(),
        )
        .unwrap();
        assert_eq!(read.draft(), original.draft());
        assert_eq!(read.bytes(), original.bytes());
    }
    assert_eq!(
        observations,
        ["current", "pre", "post"].map(str::to_owned).into()
    );
    assert_eq!(
        kinds,
        [
            "absent",
            "boolean",
            "enum",
            "integer",
            "object",
            "present",
            "record",
            "reference",
            "sequence",
            "text"
        ]
        .map(str::to_owned)
        .into()
    );
    assert_eq!(input.invocations.len(), 2);
    for (original, expected) in input.invocations.iter().zip([Some(ValueId::new(0)), None]) {
        assert!(schema.is_valid(&serde_json::from_slice(original.bytes()).unwrap()));
        assert_eq!(original.draft().result, expected);
        assert_eq!(original.draft().parameters.len(), 1);
        assert_eq!(original.draft().created.len(), 1);
        assert_eq!(original.draft().deleted.len(), 1);
        let read = Invocation::read_verified(
            &original.reference(),
            original.bytes(),
            ArtifactLimits::default(),
        )
        .unwrap();
        assert_eq!(read.draft(), original.draft());
        assert_eq!(read.bytes(), original.bytes());
    }
}

#[test]
#[trace("TC-100", "FR-024-AC-5")]
fn schema_refuses_missing_unknown_mistyped_and_positional_records() {
    let schema = schema();
    for original in envelopes(&populated_inputs()) {
        assert!(schema.is_valid(&original), "positive precondition");
        let mut pending = vec![(String::new(), &original)];
        let mut records = 0;
        while let Some((path, value)) = pending.pop() {
            match value {
                Value::Object(members) => {
                    records += 1;
                    let mut extra = original.clone();
                    extra
                        .pointer_mut(&path)
                        .unwrap()
                        .as_object_mut()
                        .unwrap()
                        .insert("unexpected".into(), json!(null));
                    assert!(!schema.is_valid(&extra), "unknown member at {path}");
                    let mut array = original.clone();
                    *array.pointer_mut(&path).unwrap() =
                        json!(members.values().collect::<Vec<_>>());
                    assert!(!schema.is_valid(&array), "positional record at {path}");
                    for (key, child) in members {
                        let mut missing = original.clone();
                        assert!(missing
                            .pointer_mut(&path)
                            .unwrap()
                            .as_object_mut()
                            .unwrap()
                            .remove(key)
                            .is_some());
                        assert!(!schema.is_valid(&missing), "missing {path}/{key}");
                        let mut mistyped = original.clone();
                        mistyped
                            .pointer_mut(&path)
                            .unwrap()
                            .as_object_mut()
                            .unwrap()
                            .insert(
                                key.clone(),
                                if child.is_array() {
                                    json!(false)
                                } else {
                                    json!([])
                                },
                            );
                        assert!(!schema.is_valid(&mistyped), "wrong type at {path}/{key}");
                        let escaped = key.replace('~', "~0").replace('/', "~1");
                        pending.push((format!("{path}/{escaped}"), child));
                    }
                }
                Value::Array(values) => pending.extend(
                    values
                        .iter()
                        .enumerate()
                        .map(|(index, child)| (format!("{path}/{index}"), child)),
                ),
                Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
            }
        }
        assert!(
            records >= 20,
            "both populated body kinds must exercise nested records"
        );
    }
}

#[test]
#[trace("TC-099", "TC-100", "FR-024-AC-5")]
fn schema_checks_kind_pairing_identifiers_and_exact_representable_scalar_bounds() {
    let schema = schema();
    let input = populated_inputs();
    let original = envelopes(&input).next().unwrap();
    assert!(schema.is_valid(&original));
    let invocation: Value = serde_json::from_slice(input.invocations[0].bytes()).unwrap();
    for (base, replacement) in [(&original, &invocation), (&invocation, &original)] {
        let mut wrong_body = base.clone();
        wrong_body["body"] = replacement["body"].clone();
        assert!(!schema.is_valid(&wrong_body));
    }
    let mut adverse = vec![
        ("/version", json!("native-state-input/2")),
        ("/kind", json!("future")),
        ("/body/observation", json!("future")),
        ("/identity/identity", json!("")),
        ("/identity/revision", json!("")),
        ("/body/arena/0/kind", json!("future")),
        ("/body/arena/0/value", json!(1.5)),
        ("/body/arena/0/value", json!(9_223_372_036_854_775_808_u64)),
        ("/body/models/0/model/revision", json!(0)),
        ("/body/models/0/model/revision", json!(-1)),
        ("/body/models/0/model/revision", json!(1.5)),
        ("/body/values/0/value", json!(-1)),
        ("/body/values/0/value", json!(1.5)),
        ("/body/values/0/value", json!(4_294_967_296_u64)),
    ];
    for name in [
        "",
        "1Name",
        "Name/Child",
        "Name\n",
        "Name\r",
        "Name\u{2028}",
        "Ω",
    ] {
        adverse.push(("/body/populations/0/record", json!(name)));
        adverse.push(("/body/models/0/model/requirement", json!(name)));
    }
    for package in [
        "",
        "/a",
        "a/",
        "a//b",
        "a/..",
        "a b",
        "a\n",
        "a\r",
        "a\u{2028}",
    ] {
        adverse.push(("/body/models/0/model/package", json!(package)));
    }
    for digest in [
        "0".repeat(63),
        "0".repeat(65),
        "A".repeat(64),
        "g".repeat(64),
        format!("sha256:{}", "0".repeat(64)),
        format!("{}\n", "0".repeat(64)),
    ] {
        adverse.push(("/body/models/0/digest", json!(digest)));
    }
    for (pointer, replacement) in adverse {
        let mut changed = original.clone();
        *changed.pointer_mut(pointer).unwrap() = replacement.clone();
        assert!(!schema.is_valid(&changed), "{pointer}: {replacement}");
    }
    // These are wire scalar domains, independent of arena/model correspondence.
    for (pointer, replacement) in [
        ("/body/arena/0/value", json!(i64::MIN)),
        ("/body/arena/0/value", json!(i64::MAX)),
        ("/body/models/0/model/revision", json!(u64::MAX)),
        ("/body/values/0/value", json!(u32::MAX)),
        ("/body/models/0/model/package", json!("1example/a-B._9")),
        ("/body/populations/0/record", json!("A-b._9")),
        ("/body/populations/0/objects/0/key", json!("")),
    ] {
        let mut changed = original.clone();
        *changed.pointer_mut(pointer).unwrap() = replacement;
        assert!(schema.is_valid(&changed), "{pointer}");
        serde_json::from_value::<SnapshotDraft>(changed["body"].clone()).unwrap();
    }
}

#[test]
#[trace("TC-100", "FR-024-AC-5")]
fn schema_pass_does_not_replace_raw_byte_selection_or_arena_admission() {
    let schema = schema();
    let original = full_snapshot();
    Snapshot::read_verified(
        &original.reference(),
        original.bytes(),
        ArtifactLimits::default(),
    )
    .unwrap();
    let text = std::str::from_utf8(original.bytes()).unwrap();
    for (fragment, replacement, stage) in [
        (
            "\"observation\":\"current\"",
            "\"observation\":\"current\",\"observation\":\"current\"",
            InputReadStage::Body,
        ),
        (
            "\"kind\":\"snapshot\"",
            "\"kind\":\"snapshot\",\"kind\":\"snapshot\"",
            InputReadStage::Envelope,
        ),
        (
            "\"kind\":\"integer\",\"value\":1}",
            "\"kind\":\"integer\",\"value\":1.0}",
            InputReadStage::Body,
        ),
        (
            "\"kind\":\"integer\",\"value\":1}",
            "\"kind\":\"integer\",\"value\":1e0}",
            InputReadStage::Body,
        ),
    ] {
        assert!(text.contains(fragment));
        let bytes = text.replacen(fragment, replacement, 1).into_bytes();
        assert!(schema.is_valid(&serde_json::from_slice(&bytes).unwrap()));
        let failure = Snapshot::read_verified(&selected(&bytes), &bytes, ArtifactLimits::default())
            .unwrap_err();
        assert_eq!(failure.code, Code::InvalidRuntimeInput);
        assert_eq!(failure.stage, stage);
        match (stage, failure.cause) {
            (InputReadStage::Envelope, InputReadCause::Envelope(_))
            | (InputReadStage::Body, InputReadCause::Body(_)) => {}
            (stage, cause) => panic!("{stage:?} produced the wrong typed cause: {cause:?}"),
        }
    }
    let stale = SnapshotRef::new(original.identity().clone(), ByteDigest::of(b"stale")).unwrap();
    assert!(schema.is_valid(&serde_json::from_slice(original.bytes()).unwrap()));
    let failure =
        Snapshot::read_verified(&stale, original.bytes(), ArtifactLimits::default()).unwrap_err();
    assert_eq!(failure.stage, InputReadStage::Selection);
    assert!(matches!(
        failure.cause,
        InputReadCause::DigestMismatch { .. }
    ));
    let mut cyclic: Value = serde_json::from_slice(original.bytes()).unwrap();
    cyclic["body"]["arena"][0] = json!({"kind": "present", "value": 0});
    assert!(schema.is_valid(&cyclic));
    let bytes = serde_json::to_vec(&cyclic).unwrap();
    let failure =
        Snapshot::read_verified(&selected(&bytes), &bytes, ArtifactLimits::default()).unwrap_err();
    assert_eq!(failure.stage, InputReadStage::Construction);
    assert_eq!(failure.code, Code::InvalidRuntimeInput);
    assert!(matches!(failure.cause, InputReadCause::Construction(_)));
}
