// SPDX-License-Identifier: AGPL-3.0-only
//! TC-056: exact input labels, bytes, independent expectations and role selectors.

use super::*;
use serde_json::json;

#[test]
#[trace("TC-056", "FR-018-AC-3", "FR-018-AC-4")]
fn empty_invocation_has_exact_independent_field_order_and_digest_spelling() {
    let artifact = Invocation::new(identity(), invocation(), ArtifactLimits::default()).unwrap();
    let expected = r#"{"version":"native-state-input/1","kind":"invocation","identity":{"identity":"snapshot:current","revision":"1"},"body":{"models":[],"context":{"model":{"package":"example/runtime","requirement":"RuntimeModel","revision":1},"name":"Node"},"operation":"step","anchor":"step","self_object":{"model":{"package":"example/runtime","requirement":"RuntimeModel","revision":1},"record":"Node","universe":"nodes","key":""},"pre":{"identity":{"identity":"snapshot:current","revision":"1"},"digest":"0000000000000000000000000000000000000000000000000000000000000000"},"post":{"identity":{"identity":"snapshot:current","revision":"1"},"digest":"0000000000000000000000000000000000000000000000000000000000000000"},"parameters":[],"result":null,"created":[],"deleted":[],"arena":[]}}"#;
    assert_eq!(artifact.bytes(), expected.as_bytes());
    assert_eq!(artifact.digest(), ByteDigest::of(expected.as_bytes()));
    assert_eq!(artifact.reference().digest(), artifact.digest());
}

#[test]
#[trace("TC-056", "FR-018-AC-3", "FR-018-AC-4", "FR-018-AC-7")]
fn label_and_observation_family_preserves_exact_bytes_on_repeat() {
    for label in ["a", "é", "e\u{301}", "😀", "\n\"\\", "\0"] {
        for revision in ["1", "01", "revision:β"] {
            for (observation, name) in [
                (ir::StateObservation::Current, "current"),
                (ir::StateObservation::Pre, "pre"),
                (ir::StateObservation::Post, "post"),
            ] {
                let selected = SourceIdentity {
                    identity: label.into(),
                    revision: revision.into(),
                };
                let mut draft = empty();
                draft.observation = observation;
                let first =
                    Snapshot::new(selected.clone(), draft.clone(), ArtifactLimits::default())
                        .unwrap();
                let second =
                    Snapshot::new(selected.clone(), draft, ArtifactLimits::default()).unwrap();
                // Authored envelope/field order; the specified serde_json string
                // primitive supplies escaping, not the constructor's object layout.
                let expected = format!(
                    "{{\"version\":\"native-state-input/1\",\"kind\":\"snapshot\",\"identity\":{{\"identity\":{},\"revision\":{}}},\"body\":{{\"observation\":\"{}\",\"models\":[],\"populations\":[],\"values\":[],\"arena\":[]}}}}",
                    serde_json::to_string(label).unwrap(),
                    serde_json::to_string(revision).unwrap(),
                    name,
                );
                assert_eq!(first.bytes(), expected.as_bytes());
                assert_eq!(first.bytes(), second.bytes());
                assert_eq!(first.digest(), second.digest());
                assert_eq!(first.usage(), second.usage());
                assert_eq!(first.identity(), &selected);
            }
        }
    }
}

#[test]
#[trace("TC-056", "FR-018-AC-3")]
fn each_snapshot_component_is_byte_bound_and_vectors_keep_order() {
    let mut baseline = empty();
    baseline.models.push(ModelBinding {
        model: owner(),
        digest: zero_digest(),
    });
    baseline.populations.push(Population {
        model: owner(),
        record: symbol("Node"),
        universe: symbol("nodes"),
        complete: true,
        objects: vec![ObjectEntry {
            key: "a".into(),
            fields: vec![field("first", 0), field("second", 1)],
        }],
    });
    baseline.values.push(binding("state", 0));
    baseline.arena = vec![
        ValueNode::Integer { value: 1 },
        ValueNode::Integer { value: 2 },
    ];
    let original = Snapshot::new(identity(), baseline.clone(), ArtifactLimits::default()).unwrap();
    for choice in 0..10 {
        let mut selected = identity();
        let mut draft = baseline.clone();
        match choice {
            0 => selected.identity.push('x'),
            1 => selected.revision.push('x'),
            2 => draft.observation = ir::StateObservation::Pre,
            3 => draft.models[0].digest = ByteDigest::of(b"changed"),
            4 => {
                draft.models[0].model =
                    ir::RequirementRef::parse("example/runtime", "RuntimeModel", 2).unwrap()
            }
            5 => draft.populations[0].complete = false,
            6 => draft.populations[0].objects[0].key.push('x'),
            7 => draft.populations[0].objects[0].fields.reverse(),
            8 => draft.values[0].value = ValueId::new(1),
            9 => draft.arena.reverse(),
            _ => unreachable!(),
        }
        let artifact = Snapshot::new(selected, draft, ArtifactLimits::default()).unwrap();
        assert_ne!(artifact.bytes(), original.bytes(), "mutation {choice}");
        assert_ne!(artifact.digest(), original.digest(), "mutation {choice}");
        assert_eq!(artifact.digest(), ByteDigest::of(artifact.bytes()));
    }
}

#[test]
#[trace("TC-056", "FR-018-AC-3", "FR-018-AC-4")]
fn expected_refs_and_invocation_mutations_retain_complete_correspondence() {
    let first = Snapshot::new(identity(), empty(), ArtifactLimits::default()).unwrap();
    let expected = SnapshotRef::new(identity(), first.digest()).unwrap();
    assert_eq!(first.reference(), expected);
    let foreign_digest = SnapshotRef::new(identity(), zero_digest()).unwrap();
    assert_ne!(expected, foreign_digest);
    assert_eq!(foreign_digest.digest(), zero_digest());
    let invocation_ref = InvocationRef::new(identity(), zero_digest()).unwrap();
    assert_eq!(invocation_ref.identity(), &identity());
    assert_eq!(invocation_ref.digest(), zero_digest());
    let mut baseline = invocation();
    baseline.arena = vec![
        ValueNode::Boolean { value: true },
        ValueNode::Boolean { value: false },
    ];
    let original =
        Invocation::new(identity(), baseline.clone(), ArtifactLimits::default()).unwrap();
    let encoded: serde_json::Value = serde_json::from_slice(original.bytes()).unwrap();
    assert_eq!(encoded["body"]["result"], json!(null));
    for choice in 0..11 {
        let mut draft = baseline.clone();
        match choice {
            0 => draft.models.push(ModelBinding {
                model: owner(),
                digest: zero_digest(),
            }),
            1 => draft.context.name = symbol("Other"),
            2 => draft.operation = symbol("other_step"),
            3 => draft.anchor = ir::AnchorName::new("other_anchor").unwrap(),
            4 => draft.self_object.key = "different".into(),
            5 => draft.pre = expected.clone(),
            6 => draft.post = expected.clone(),
            7 => draft.parameters.push(binding("arg", 0)),
            8 => draft.result = Some(ValueId::new(1)),
            9 => draft.created.push(object("created")),
            10 => draft.deleted.push(object("deleted")),
            _ => unreachable!(),
        }
        let changed =
            Invocation::new(identity(), draft.clone(), ArtifactLimits::default()).unwrap();
        assert_ne!(
            changed.bytes(),
            original.bytes(),
            "invocation mutation {choice}"
        );
        assert_ne!(changed.digest(), original.digest());
        assert_eq!(changed.draft(), &draft);
        assert_eq!(changed.usage().artifact_bytes, changed.bytes().len());
    }
}

#[test]
#[trace("TC-056", "FR-018-AC-4", "FR-018-AC-7")]
fn invalid_labels_have_source_free_errors_for_both_artifacts_and_refs() {
    for field in ["identity", "revision"] {
        let mut selected = identity();
        match field {
            "identity" => selected.identity.clear(),
            "revision" => selected.revision.clear(),
            _ => unreachable!(),
        }
        let errors = [
            Snapshot::new(selected.clone(), empty(), ArtifactLimits::default()).unwrap_err(),
            Invocation::new(selected.clone(), invocation(), ArtifactLimits::default()).unwrap_err(),
            SnapshotRef::new(selected.clone(), zero_digest()).unwrap_err(),
            InvocationRef::new(selected.clone(), zero_digest()).unwrap_err(),
        ];
        for error in errors {
            assert_eq!(error.code, Code::InvalidSourceIdentity);
            assert_eq!(error.identity, selected);
            assert_eq!(
                error.path,
                vec![
                    DraftPathSegment::Field("identity"),
                    DraftPathSegment::Field(field)
                ]
            );
            assert_eq!(error.usage.artifact_bytes, 0);
            assert!(!error.is_incomplete());
            assert!(std::error::Error::source(error.as_ref()).is_none());
            assert!(error.to_string().starts_with("invalid_source_identity: "));
        }
    }
}
