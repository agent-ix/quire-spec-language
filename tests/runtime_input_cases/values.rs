// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-055: complete flat value/metadata preservation and structural refusals.

use super::*;
use serde_json::json;

#[test]
#[trace("TC-055", "FR-018-AC-1", "FR-018-AC-5")]
fn every_value_kind_has_its_exact_tag_and_payload() {
    let mut draft = empty();
    draft.arena = vec![
        ValueNode::Boolean { value: true },
        ValueNode::Integer { value: i64::MIN },
        ValueNode::Integer { value: i64::MAX },
        ValueNode::Text {
            value: "é\n\\\"😀".into(),
        },
        ValueNode::Enum {
            declaration: qualified("Color"),
            variant: symbol("Blue"),
        },
        ValueNode::Record {
            declaration: qualified("Pair"),
            fields: vec![field("right", 2), field("left", 1), field("left", 1)],
        },
        ValueNode::Absent,
        ValueNode::Present {
            value: ValueId::new(5),
        },
        ValueNode::Sequence {
            values: vec![ValueId::new(7), ValueId::new(7), ValueId::new(6)],
        },
        ValueNode::Reference {
            identity: object("a"),
        },
        ValueNode::Object {
            identity: object("a"),
        },
        ValueNode::Record {
            declaration: qualified("Empty"),
            fields: vec![],
        },
        ValueNode::Sequence { values: vec![] },
    ];
    let original = draft.clone();
    let artifact = Snapshot::new(identity(), draft, ArtifactLimits::default()).unwrap();
    assert_eq!(artifact.draft(), &original);
    let model = json!({"package":"example/runtime","requirement":"RuntimeModel","revision":1});
    let object = json!({"model":model,"record":"Node","universe":"nodes","key":"a"});
    let expected = json!([
        {"kind":"boolean","value":true},
        {"kind":"integer","value":-9223372036854775808_i64},
        {"kind":"integer","value":9223372036854775807_i64},
        {"kind":"text","value":"é\n\\\"😀"},
        {"kind":"enum","declaration":{"model":model,"name":"Color"},"variant":"Blue"},
        {"kind":"record","declaration":{"model":model,"name":"Pair"},"fields":[
            {"name":"right","value":2},{"name":"left","value":1},{"name":"left","value":1}
        ]},
        {"kind":"absent"},
        {"kind":"present","value":5},
        {"kind":"sequence","values":[7,7,6]},
        {"kind":"reference","identity":object},
        {"kind":"object","identity":object},
        {"kind":"record","declaration":{"model":model,"name":"Empty"},"fields":[]},
        {"kind":"sequence","values":[]}
    ]);
    let encoded: serde_json::Value = serde_json::from_slice(artifact.bytes()).unwrap();
    assert_eq!(encoded["body"]["arena"], expected);
    assert_eq!(artifact.usage().nodes, 13);
    assert_eq!(artifact.usage().entries, 7); // 3 fields + present + 3 sequence edges.
    assert_eq!(artifact.usage().depth, 4);
}

#[test]
#[trace("TC-055", "FR-018-AC-1", "FR-018-AC-5", "FR-018-AC-7")]
fn snapshot_metadata_preserves_duplicates_for_the_model_validator() {
    let mut draft = empty();
    draft.models = vec![
        ModelBinding {
            model: owner(),
            digest: zero_digest(),
        },
        ModelBinding {
            model: owner(),
            digest: zero_digest(),
        },
    ];
    let entry = ObjectEntry {
        key: "".into(),
        fields: vec![field("n", 0), field("n", 0)],
    };
    draft.populations = vec![Population {
        model: owner(),
        record: symbol("Node"),
        universe: symbol("nodes"),
        complete: false,
        objects: vec![entry.clone(), entry],
    }];
    draft.values = vec![binding("state", 0), binding("state", 0)];
    draft.arena = vec![ValueNode::Text {
        value: "not an admitted integer".into(),
    }];
    let original = draft.clone();
    let artifact = Snapshot::new(identity(), draft.clone(), ArtifactLimits::default()).unwrap();
    assert_eq!(artifact.draft(), &original);
    assert_eq!(draft, original);
    let encoded: serde_json::Value = serde_json::from_slice(artifact.bytes()).unwrap();
    assert_eq!(encoded["body"]["populations"][0]["complete"], false);
    assert_eq!(
        encoded["body"]["populations"][0]["objects"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(encoded["body"]["populations"][0]["objects"][0]["key"], "");
    assert_eq!(encoded["body"]["models"][0]["digest"], "0".repeat(64));
    assert_eq!(
        encoded["body"]["values"],
        json!([
            {"declaration":{"model":{"package":"example/runtime","requirement":"RuntimeModel","revision":1},"name":"state"},"value":0},
            {"declaration":{"model":{"package":"example/runtime","requirement":"RuntimeModel","revision":1},"name":"state"},"value":0}
        ])
    );
    assert_eq!(artifact.usage().entries, 11); // Models 2 + population 1 + objects 2 + fields 4 + roots 2.
}

#[test]
#[trace("TC-055", "FR-018-AC-1", "FR-018-AC-3", "FR-018-AC-4", "FR-018-AC-5")]
fn invocation_encodes_every_original_binding_without_claiming_validity() {
    let mut draft = invocation();
    draft.models.push(ModelBinding {
        model: owner(),
        digest: zero_digest(),
    });
    draft.parameters = vec![
        binding("arg_b", 0),
        binding("arg_a", 0),
        binding("arg_a", 0),
    ];
    draft.result = Some(ValueId::new(0));
    draft.created = vec![object("new"), object("new")];
    draft.deleted = vec![object("new")];
    draft.arena = vec![ValueNode::Boolean { value: false }];
    let original = draft.clone();
    let artifact = Invocation::new(identity(), draft, ArtifactLimits::default()).unwrap();
    assert_eq!(artifact.draft(), &original);
    assert_eq!(artifact.identity(), &identity());
    assert_eq!(artifact.digest(), ByteDigest::of(artifact.bytes()));
    assert_eq!(artifact.reference().identity(), artifact.identity());
    assert_eq!(artifact.reference().digest(), artifact.digest());
    let model = json!({"package":"example/runtime","requirement":"RuntimeModel","revision":1});
    let object = json!({"model":model,"record":"Node","universe":"nodes","key":"new"});
    let selection = json!({"identity":{"authority":"agent-ix","identity":"snapshot:current","revision_namespace":"git","revision":"1"},"digest":"0".repeat(64)});
    let expected = json!({
        "models":[{"model":model,"digest":"0".repeat(64)}],
        "context":{"model":model,"name":"Node"},
        "operation":"step","anchor":"step",
        "self_object":{"model":model,"record":"Node","universe":"nodes","key":""},
        "pre":selection,"post":selection,
        "parameters":[
            {"declaration":{"model":model,"name":"arg_b"},"value":0},
            {"declaration":{"model":model,"name":"arg_a"},"value":0},
            {"declaration":{"model":model,"name":"arg_a"},"value":0}
        ],
        "result":0,"created":[object,object],"deleted":[object],
        "arena":[{"kind":"boolean","value":false}]
    });
    let encoded: serde_json::Value = serde_json::from_slice(artifact.bytes()).unwrap();
    assert_eq!(encoded["kind"], "invocation");
    assert_eq!(encoded["body"], expected);
    assert_eq!(artifact.usage().entries, 8); // Model + parameters 3 + result + deltas 3.
    assert_eq!(artifact.usage().nodes, 1);
}

#[test]
#[trace("TC-055", "FR-018-AC-2", "FR-018-AC-5", "FR-018-AC-7")]
fn every_container_rejects_forward_self_and_out_of_range_children() {
    for child in [1, 2, u32::MAX] {
        for container in [
            ValueNode::Present {
                value: ValueId::new(child),
            },
            ValueNode::Sequence {
                values: vec![ValueId::new(0), ValueId::new(child)],
            },
            ValueNode::Record {
                declaration: qualified("Record"),
                fields: vec![field("ok", 0), field("bad", child)],
            },
        ] {
            let mut draft = empty();
            draft.arena = vec![ValueNode::Absent, container];
            let error = Snapshot::new(identity(), draft, ArtifactLimits::default()).unwrap_err();
            assert_eq!(error.code, Code::InvalidRuntimeInput);
            assert_eq!(
                &error.path[..2],
                &[DraftPathSegment::Field("arena"), DraftPathSegment::Index(1)]
            );
            assert_eq!(error.usage.artifact_bytes, 0);
        }
    }
    let mut draft = empty();
    draft.arena = vec![
        ValueNode::Absent,
        ValueNode::Present {
            value: ValueId::new(0),
        },
    ];
    assert_eq!(
        Snapshot::new(identity(), draft, ArtifactLimits::default())
            .unwrap()
            .usage()
            .depth,
        2
    );
}

#[test]
#[trace("TC-055", "FR-018-AC-2", "FR-018-AC-7")]
fn all_snapshot_and_invocation_root_positions_are_checked() {
    for invalid in [0, u32::MAX] {
        let mut state = empty();
        state.values.push(binding("state", invalid));
        let error = Snapshot::new(identity(), state, ArtifactLimits::default()).unwrap_err();
        assert_eq!(error.code, Code::InvalidRuntimeInput);
        assert_eq!(
            error.path,
            vec![
                DraftPathSegment::Field("values"),
                DraftPathSegment::Index(0),
                DraftPathSegment::Field("value")
            ]
        );

        let mut population = empty();
        population.populations.push(Population {
            model: owner(),
            record: symbol("Node"),
            universe: symbol("nodes"),
            complete: true,
            objects: vec![ObjectEntry {
                key: "key".into(),
                fields: vec![field("field", invalid)],
            }],
        });
        let error = Snapshot::new(identity(), population, ArtifactLimits::default()).unwrap_err();
        assert_eq!(error.code, Code::InvalidRuntimeInput);
        assert_eq!(
            error.path,
            vec![
                DraftPathSegment::Field("populations"),
                DraftPathSegment::Index(0),
                DraftPathSegment::Field("objects"),
                DraftPathSegment::Index(0),
                DraftPathSegment::Field("fields"),
                DraftPathSegment::Index(0),
                DraftPathSegment::Field("value"),
            ]
        );
        let mut parameter = invocation();
        parameter.parameters.push(binding("arg", invalid));
        let error = Invocation::new(identity(), parameter, ArtifactLimits::default()).unwrap_err();
        assert_eq!(error.code, Code::InvalidRuntimeInput);
        assert_eq!(
            error.path,
            vec![
                DraftPathSegment::Field("parameters"),
                DraftPathSegment::Index(0),
                DraftPathSegment::Field("value")
            ]
        );
        let mut result = invocation();
        result.result = Some(ValueId::new(invalid));
        let error = Invocation::new(identity(), result, ArtifactLimits::default()).unwrap_err();
        assert_eq!(error.code, Code::InvalidRuntimeInput);
        assert_eq!(error.path, vec![DraftPathSegment::Field("result")]);
    }
}
