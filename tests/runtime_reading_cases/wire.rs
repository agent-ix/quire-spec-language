// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-024: the public record itself owns closed decoding at every nesting level.

use super::*;
use quire_spec_language::runtime::{InvocationDraft, InvocationRef, ModelBinding};
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

fn closed_record<T: Serialize + DeserializeOwned + PartialEq + Debug>(
    original: &T,
    fields: &[&str],
) {
    let text = serde_json::to_string(original).unwrap();
    let value: Value = serde_json::from_str(&text).unwrap();
    let object = value.as_object().unwrap();
    assert_eq!(object.len(), fields.len());
    assert_eq!(serde_json::from_str::<T>(&text).unwrap(), *original);
    assert_eq!(
        serde_json::from_value::<T>(value.clone()).unwrap(),
        *original
    );

    // Preserve the Rust field order: default Serde accepts these arrays for structs.
    let array = json!(fields
        .iter()
        .map(|field| &object[*field])
        .collect::<Vec<_>>());
    assert!(
        serde_json::from_value::<T>(array.clone()).is_err(),
        "array: {text}"
    );
    assert!(
        serde_json::from_str::<T>(&array.to_string()).is_err(),
        "array: {text}"
    );
    for field in fields {
        let mut missing = object.clone();
        assert!(missing.remove(*field).is_some());
        assert!(
            serde_json::from_value::<T>(Value::Object(missing)).is_err(),
            "missing {field}: {text}"
        );
        let mut malformed = object.clone();
        malformed.insert((*field).into(), json!({"invalid_field_shape": true}));
        assert!(
            serde_json::from_value::<T>(Value::Object(malformed)).is_err(),
            "malformed {field}: {text}"
        );
    }
    let mut unknown = object.clone();
    assert!(unknown.insert("unexpected".into(), json!(true)).is_none());
    assert!(
        serde_json::from_value::<T>(Value::Object(unknown)).is_err(),
        "unknown: {text}"
    );
    // Keep raw duplicate keys; Value would coalesce them before the decoder sees them.
    let field = fields[0];
    let duplicate = format!("{{{}:{},{}", json!(field), object[field], &text[1..]);
    assert!(
        serde_json::from_str::<T>(&duplicate).is_err(),
        "duplicate: {duplicate}"
    );
}

#[test]
#[trace("TC-099", "TC-100", "FR-024-AC-3")]
fn public_runtime_records_own_closed_object_decoding() {
    let model = setup::native_rule_model::parts().model();
    let snapshot = full_snapshot();
    let draft = snapshot.draft();
    closed_record(
        draft,
        &["observation", "models", "populations", "values", "arena"],
    );
    closed_record(&draft.models[0], &["model", "digest"]);
    closed_record(
        &draft.populations[0],
        &["model", "record", "universe", "complete", "objects"],
    );
    closed_record(&draft.populations[0].objects[0], &["key", "fields"]);
    closed_record(
        &draft.populations[0].objects[0].fields[0],
        &["name", "value"],
    );
    closed_record(&setup::qualified(&model, "Node"), &["model", "name"]);
    closed_record(
        &setup::object(&model, "self"),
        &["model", "record", "universe", "key"],
    );
    closed_record(&snapshot.reference(), &["identity", "digest"]);
    let (offered, _) = setup::recorded(
        &model,
        setup::draft(&model),
        setup::draft(&model),
        |draft| {
            draft
                .parameters
                .push(quire_spec_language::runtime::ValueBinding {
                    declaration: setup::qualified(&model, "argument"),
                    value: ValueId::new(0),
                });
        },
    );
    assert_eq!(offered.invocations.len(), 1);
    let invocation = &offered.invocations[0];
    closed_record(&invocation.reference(), &["identity", "digest"]);
    closed_record(
        invocation.draft(),
        &[
            "models",
            "context",
            "operation",
            "anchor",
            "self_object",
            "pre",
            "post",
            "parameters",
            "result",
            "created",
            "deleted",
            "arena",
        ],
    );
    assert!(!invocation.draft().parameters.is_empty());
    closed_record(&invocation.draft().parameters[0], &["declaration", "value"]);
}

#[test]
#[trace("TC-099", "TC-100", "FR-024-AC-3")]
fn all_value_variants_own_closed_object_decoding() {
    let snapshot = full_snapshot();
    let mut kinds = std::collections::BTreeSet::new();
    for node in &snapshot.draft().arena {
        let fields: &[&str] = match node {
            ValueNode::Boolean { .. }
            | ValueNode::Integer { .. }
            | ValueNode::Text { .. }
            | ValueNode::Present { .. } => &["kind", "value"],
            ValueNode::Enum { .. } => &["kind", "declaration", "variant"],
            ValueNode::Record { .. } => &["kind", "declaration", "fields"],
            ValueNode::Absent => &["kind"],
            ValueNode::Sequence { .. } => &["kind", "values"],
            ValueNode::Reference { .. } | ValueNode::Object { .. } => &["kind", "identity"],
        };
        closed_record(node, fields);
        kinds.insert(
            serde_json::to_value(node).unwrap()["kind"]
                .as_str()
                .unwrap()
                .to_owned(),
        );
    }
    assert_eq!(kinds.len(), 10, "every declared variant must be exercised");
}

#[test]
#[trace("TC-099", "TC-100", "FR-024-AC-3")]
fn nullable_result_and_native_digest_keep_their_exact_wire_domains() {
    let model = setup::native_rule_model::parts().model();
    let (offered, _) = setup::recorded(&model, setup::draft(&model), setup::draft(&model), |_| {});
    let original = &offered.invocations[0];
    let mut value = serde_json::to_value(original.draft()).unwrap();
    value["result"] = Value::Null;
    assert_eq!(
        serde_json::from_value::<InvocationDraft>(value.clone())
            .unwrap()
            .result,
        None
    );
    for invalid in [
        json!(true),
        json!(-1),
        json!(1.5),
        json!(4_294_967_296_u64),
        json!("0"),
        json!([]),
    ] {
        value["result"] = invalid;
        assert!(serde_json::from_value::<InvocationDraft>(value.clone()).is_err());
    }
    assert!(value.as_object_mut().unwrap().remove("result").is_some());
    assert!(serde_json::from_value::<InvocationDraft>(value).is_err());

    let empty_digest = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    let mut binding = original.draft().models[0].clone();
    binding.digest = ByteDigest::of(b"");
    assert_eq!(format!("{:x}", binding.digest), empty_digest);
    assert_eq!(binding.digest.to_string(), format!("sha256:{empty_digest}"));
    assert_eq!(
        binding.digest.to_string().parse::<ByteDigest>().unwrap(),
        binding.digest
    );
    assert!(empty_digest.parse::<ByteDigest>().is_err());
    let mut wire = serde_json::to_value(&binding).unwrap();
    assert_eq!(wire["digest"], empty_digest);
    assert_eq!(
        serde_json::from_value::<ModelBinding>(wire.clone()).unwrap(),
        binding
    );
    for invalid in [
        "".to_owned(),
        "0".repeat(63),
        "0".repeat(65),
        "é".repeat(32),
        "g".repeat(64),
        empty_digest.to_uppercase(),
        format!("sha256:{empty_digest}"),
    ] {
        wire["digest"] = json!(invalid);
        assert!(
            serde_json::from_value::<ModelBinding>(wire.clone()).is_err(),
            "{invalid}"
        );
        let mut reference = serde_json::to_value(original.reference()).unwrap();
        reference["digest"] = json!(invalid);
        assert!(
            serde_json::from_value::<InvocationRef>(reference).is_err(),
            "{invalid}"
        );
    }
}
