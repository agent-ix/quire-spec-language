// SPDX-License-Identifier: AGPL-3.0-only
//! FR-024: closed native artifact decoding and actual runtime reuse.

// The fixture also supports compiler, graph and operation integration tests.
#[allow(dead_code)]
#[path = "support/runtime_setup.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_spec_language::package::{NativePackage, PackageLimits};
use quire_spec_language::runtime::{
    execute, ArtifactLimits, ExecutionLimits, ExecutionOutcome, InputReadCause, InputReadStage,
    Invocation, RuntimeInput, Snapshot, SnapshotRef, ValueId, ValueNode,
};
use quire_spec_language::syntax::ClauseKind;
use quire_spec_language::{ByteDigest, Code, SourceIdentity};
use serde_json::{json, Value};

fn selected(bytes: &[u8]) -> SnapshotRef {
    SnapshotRef::new(
        SourceIdentity {
            identity: "test:runtime-current".into(),
            revision: "1".into(),
        },
        ByteDigest::of(bytes),
    )
    .unwrap()
}

fn full_snapshot() -> Snapshot {
    let model = setup::native_rule_model::parts().model();
    let mut draft = setup::draft(&model);
    draft.arena.extend([
        ValueNode::Boolean { value: true },
        ValueNode::Text {
            value: "Ω\n\"exact\"".into(),
        },
        ValueNode::Enum {
            declaration: setup::qualified(&model, "Status"),
            variant: setup::symbol("Ready"),
        },
        ValueNode::Record {
            declaration: setup::qualified(&model, "Payload"),
            fields: vec![setup::field("label", 6)],
        },
        ValueNode::Present {
            value: ValueId::new(0),
        },
        ValueNode::Object {
            identity: setup::object(&model, "self"),
        },
        ValueNode::Integer { value: i64::MIN },
        ValueNode::Integer { value: i64::MAX },
        ValueNode::Sequence {
            values: vec![ValueId::new(0); 2],
        },
    ]);
    // Structural admission preserves duplicate vector entries for later validation.
    draft.populations[0].objects[0]
        .fields
        .push(setup::field("n", 0));
    let duplicate = draft.populations[0].objects[0].clone();
    draft.populations[0].objects.push(duplicate);
    setup::snapshot(draft)
}

#[test]
#[trace("TC-099", "FR-024-AC-1", "FR-024-AC-3")]
fn reads_every_value_variant_and_preserves_external_layout_and_duplicates() {
    let original = full_snapshot();
    let value: Value = serde_json::from_slice(original.bytes()).unwrap();
    // Value's sorted object keys also change the constructor's declared field order.
    for bytes in [
        original.bytes().to_vec(),
        serde_json::to_vec_pretty(&value).unwrap(),
    ] {
        let expected = selected(&bytes);
        let read = Snapshot::read_verified(&expected, &bytes, ArtifactLimits::default()).unwrap();
        assert_eq!(read.reference(), expected);
        assert_eq!(read.bytes(), bytes);
        assert_eq!(read.draft(), original.draft());
        assert_eq!(read.usage(), original.usage());
        assert_eq!(read.digest(), ByteDigest::of(&bytes));
    }
}

#[test]
#[trace("TC-100", "FR-024-AC-2")]
fn reads_refuse_stale_selections_and_incompatible_envelopes_before_body_decoding() {
    let original = full_snapshot();
    let stale = SnapshotRef::new(original.identity().clone(), ByteDigest::of(b"stale")).unwrap();
    let failure =
        Snapshot::read_verified(&stale, original.bytes(), ArtifactLimits::default()).unwrap_err();
    assert_eq!(failure.code, Code::StaleDependency);
    assert_eq!(failure.stage, InputReadStage::Selection);
    assert_eq!(failure.expected.identity(), stale.identity());
    assert_eq!(failure.expected.digest(), stale.digest());
    for (field, value, code) in [
        ("version", json!("native-state-input/2"), Code::UnknownWire),
        ("kind", json!("invocation"), Code::InvalidRuntimeInput),
        (
            "identity",
            json!({"identity":"foreign", "revision":"1"}),
            Code::StaleDependency,
        ),
    ] {
        let mut wire: Value = serde_json::from_slice(original.bytes()).unwrap();
        wire[field] = value;
        wire["body"] = json!({"invalid_for_any_native_body": true});
        let bytes = serde_json::to_vec(&wire).unwrap();
        let expected = selected(&bytes);
        let failure =
            Snapshot::read_verified(&expected, &bytes, ArtifactLimits::default()).unwrap_err();
        assert_eq!(failure.code, code, "{field}");
        assert_eq!(failure.stage, InputReadStage::Envelope);
        assert_eq!(failure.expected.digest(), expected.digest());
        assert!(matches!(failure.cause, InputReadCause::Selection(_)));
    }
    for bytes in [b"{broken".to_vec(), b"[]".to_vec(), b"null".to_vec()] {
        let failure = Snapshot::read_verified(&selected(&bytes), &bytes, ArtifactLimits::default())
            .unwrap_err();
        assert_eq!(failure.stage, InputReadStage::Envelope);
        assert!(matches!(failure.cause, InputReadCause::Json(_)));
    }
}

#[test]
#[trace("TC-100", "FR-024-AC-2", "FR-024-AC-3")]
fn native_records_require_object_shapes_and_closed_fields() {
    let original = full_snapshot();
    let wire: Value = serde_json::from_slice(original.bytes()).unwrap();
    for pointer in [
        "",
        "/identity",
        "/body",
        "/body/models/0",
        "/body/models/0/model",
        "/body/populations/0",
        "/body/populations/0/objects/0",
        "/body/populations/0/objects/0/fields/0",
        "/body/arena/2",
        "/body/arena/3",
        "/body/arena/3/identity",
        "/body/arena/7",
        "/body/arena/7/declaration",
        "/body/arena/8",
        "/body/arena/8/fields/0",
    ] {
        let object = wire.pointer(pointer).unwrap().as_object().unwrap();
        for removed in object.keys() {
            let mut changed = wire.clone();
            changed
                .pointer_mut(pointer)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(removed);
            let bytes = serde_json::to_vec(&changed).unwrap();
            let failure =
                Snapshot::read_verified(&selected(&bytes), &bytes, ArtifactLimits::default())
                    .unwrap_err();
            assert_eq!(
                failure.code,
                Code::InvalidRuntimeInput,
                "missing {pointer}/{removed}"
            );
            assert!(matches!(failure.cause, InputReadCause::Json(_)));
        }
        let mut changed = wire.clone();
        changed
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), json!(true));
        let bytes = serde_json::to_vec(&changed).unwrap();
        assert!(
            Snapshot::read_verified(&selected(&bytes), &bytes, ArtifactLimits::default()).is_err(),
            "extra {pointer}"
        );
        // Positional struct sequences are Serde-compatible but not native JSON records.
        let mut changed = wire.clone();
        *changed.pointer_mut(pointer).unwrap() = Value::Array(object.values().cloned().collect());
        let bytes = serde_json::to_vec(&changed).unwrap();
        assert!(
            Snapshot::read_verified(&selected(&bytes), &bytes, ArtifactLimits::default()).is_err(),
            "array {pointer}"
        );
    }
    for fragment in [
        "\"version\":\"native-state-input/1\"",
        "\"observation\":\"current\"",
        "\"kind\":\"absent\"",
        "\"package\":\"example/rule-tests\"",
    ] {
        let text = std::str::from_utf8(original.bytes()).unwrap();
        assert!(
            text.contains(fragment),
            "duplicate control must alter an existing field"
        );
        let bytes = text
            .replacen(fragment, &format!("{fragment},{fragment}"), 1)
            .into_bytes();
        let failure = Snapshot::read_verified(&selected(&bytes), &bytes, ArtifactLimits::default())
            .unwrap_err();
        assert_eq!(failure.code, Code::InvalidRuntimeInput);
        assert!(matches!(failure.cause, InputReadCause::Json(_)));
    }
}

#[test]
#[trace("TC-100", "FR-024-AC-3", "FR-024-AC-4")]
fn malformed_values_and_limits_refuse_without_losing_fresh_read_behavior() {
    let original = full_snapshot();
    for (pointer, value) in [
        ("/body/models/0/model/revision", json!(0)),
        ("/body/models/0/model/package", json!("")),
        ("/body/models/0/digest", json!("A".repeat(64))),
        ("/body/populations/0/record", json!("")),
        ("/body/arena/0/value", json!(1.5)),
        ("/body/arena/0/value", json!(9_223_372_036_854_775_808_u64)),
        ("/body/arena/0/kind", json!("unknown")),
        ("/body/arena/9/value", json!(-1)),
    ] {
        let mut wire: Value = serde_json::from_slice(original.bytes()).unwrap();
        *wire.pointer_mut(pointer).unwrap() = value;
        let bytes = serde_json::to_vec(&wire).unwrap();
        let failure = Snapshot::read_verified(&selected(&bytes), &bytes, ArtifactLimits::default())
            .unwrap_err();
        assert_eq!(failure.stage, InputReadStage::Body, "{pointer}");
        assert!(matches!(failure.cause, InputReadCause::Json(_)));
    }
    let mut wire: Value = serde_json::from_slice(original.bytes()).unwrap();
    wire["body"]["arena"][0] = json!({"kind":"present","value":0});
    let bytes = serde_json::to_vec(&wire).unwrap();
    let failure =
        Snapshot::read_verified(&selected(&bytes), &bytes, ArtifactLimits::default()).unwrap_err();
    assert_eq!(failure.stage, InputReadStage::Construction);
    assert!(
        matches!(failure.cause, InputReadCause::Construction(ref e) if e.code == Code::InvalidRuntimeInput && !e.path.is_empty())
    );
    for limits in [
        ArtifactLimits {
            artifact_bytes: original.bytes().len() - 1,
            ..ArtifactLimits::default()
        },
        ArtifactLimits {
            nodes: 0,
            ..ArtifactLimits::default()
        },
        ArtifactLimits {
            entries: 0,
            ..ArtifactLimits::default()
        },
        ArtifactLimits {
            depth: 0,
            ..ArtifactLimits::default()
        },
    ] {
        let failure =
            Snapshot::read_verified(&original.reference(), original.bytes(), limits).unwrap_err();
        assert!(failure.is_incomplete());
        assert_eq!(failure.expected.digest(), original.digest());
        let retry = Snapshot::read_verified(
            &original.reference(),
            original.bytes(),
            ArtifactLimits::default(),
        )
        .unwrap();
        assert_eq!(retry.draft(), original.draft());
    }
    let nested = format!("{{\"version\":\"native-state-input/1\",\"kind\":\"snapshot\",\"identity\":{{\"identity\":\"test:runtime-current\",\"revision\":\"1\"}},\"body\":{}0{}}}", "[".repeat(200), "]".repeat(200)).into_bytes();
    assert!(
        Snapshot::read_verified(&selected(&nested), &nested, ArtifactLimits::default()).is_err()
    );
}

#[test]
#[trace("TC-099", "FR-024-AC-1", "FR-024-AC-4")]
fn reread_snapshots_and_invocations_reach_actual_native_execution() {
    let models = [setup::native_rule_model::parts().model()];
    let package = NativePackage::new(
        setup::checked(&models, "self.n = 1"),
        PackageLimits::default(),
    )
    .unwrap();
    for value in [1, 2] {
        let mut draft = setup::draft(&models[0]);
        setup::change_field(&mut draft, "n", ValueNode::Integer { value });
        let original = setup::snapshot(draft);
        let read = Snapshot::read_verified(
            &original.reference(),
            original.bytes(),
            ArtifactLimits::default(),
        )
        .unwrap();
        let selection = setup::selection(&models[0], read.reference());
        assert_eq!(
            execute(
                &package,
                setup::input(read),
                selection,
                ExecutionLimits::default(),
                || false
            )
            .truth(),
            Some(value == 1)
        );
    }
    let package = NativePackage::new(
        setup::checked_kind(
            &models,
            "pre(self.n) = 1 and self.n = 2 and result",
            ClauseKind::Postcondition,
        ),
        PackageLimits::default(),
    )
    .unwrap();
    for bad_frame in [false, true] {
        let before = setup::draft(&models[0]);
        let mut after = before.clone();
        setup::change_field(&mut after, "n", ValueNode::Integer { value: 2 });
        if bad_frame {
            setup::change_field(&mut after, "signed", ValueNode::Integer { value: 1 });
        }
        let (offered, selection) = setup::recorded(&models[0], before, after, |_| {});
        let snapshots = offered
            .snapshots
            .iter()
            .map(|s| {
                Snapshot::read_verified(&s.reference(), s.bytes(), ArtifactLimits::default())
                    .unwrap()
            })
            .collect();
        let invocations = offered
            .invocations
            .iter()
            .map(|i| {
                let read =
                    Invocation::read_verified(&i.reference(), i.bytes(), ArtifactLimits::default())
                        .unwrap();
                assert_eq!(read.draft(), i.draft());
                assert_eq!(read.bytes(), i.bytes());
                assert_eq!(read.reference(), i.reference());
                read
            })
            .collect();
        let report = execute(
            &package,
            RuntimeInput {
                snapshots,
                invocations,
            },
            selection,
            ExecutionLimits::default(),
            || false,
        );
        if bad_frame {
            assert_eq!(report.truth(), None);
            assert!(
                matches!(report.outcome(), ExecutionOutcome::ValidationFailed(f) if f.diagnostics.iter().any(|d| d.code == Code::FrameViolation))
            );
        } else {
            assert_eq!(report.truth(), Some(true));
        }
        let original = &offered.invocations[0];
        let mut wire: Value = serde_json::from_slice(original.bytes()).unwrap();
        wire["body"]["result"] = Value::Null;
        let bytes = serde_json::to_vec_pretty(&wire).unwrap();
        let expected = quire_spec_language::runtime::InvocationRef::new(
            original.identity().clone(),
            ByteDigest::of(&bytes),
        )
        .unwrap();
        assert_eq!(
            Invocation::read_verified(&expected, &bytes, ArtifactLimits::default())
                .unwrap()
                .draft()
                .result,
            None
        );
        wire["body"].as_object_mut().unwrap().remove("result");
        let bytes = serde_json::to_vec(&wire).unwrap();
        let expected = quire_spec_language::runtime::InvocationRef::new(
            original.identity().clone(),
            ByteDigest::of(&bytes),
        )
        .unwrap();
        assert!(Invocation::read_verified(&expected, &bytes, ArtifactLimits::default()).is_err());
    }
}
