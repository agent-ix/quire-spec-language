// SPDX-License-Identifier: AGPL-3.0-only
//! TC-057: independently bounded input dimensions and flat shared-value storage.

use super::*;

fn limits_from(snapshot: &Snapshot) -> ArtifactLimits {
    let used = snapshot.usage();
    ArtifactLimits {
        artifact_bytes: used.artifact_bytes,
        nodes: used.nodes,
        entries: used.entries,
        depth: used.depth,
    }
}

#[test]
#[trace("TC-057", "FR-018-AC-6")]
fn independent_text_totals_include_all_metadata_and_value_strings() {
    let mut snapshot = empty();
    snapshot.models.push(ModelBinding {
        model: owner(),
        digest: zero_digest(),
    });
    snapshot.populations.push(Population {
        model: owner(),
        record: symbol("Node"),
        universe: symbol("nodes"),
        complete: true,
        objects: vec![ObjectEntry {
            key: "a".into(),
            fields: vec![field("n", 0)],
        }],
    });
    snapshot.values.push(binding("state", 0));
    snapshot.arena = vec![ValueNode::Text { value: "x".into() }];
    let artifact = Snapshot::new(identity(), snapshot.clone(), ArtifactLimits::default()).unwrap();
    // Labels 17; three owners 27 each; record/universe 9; key/field 2;
    // State name 5; payload 1. Digests and fixed encoding tokens are output work.
    assert_eq!(artifact.usage().text_bytes, 115);
    let error = Snapshot::new(
        identity(),
        snapshot,
        ArtifactLimits {
            artifact_bytes: 114,
            ..ArtifactLimits::default()
        },
    )
    .unwrap_err();
    assert_eq!(error.usage.text_bytes, 114);
    assert_eq!(error.usage.artifact_bytes, 0);
    assert_eq!(
        error.path,
        vec![
            DraftPathSegment::Field("arena"),
            DraftPathSegment::Index(0),
            DraftPathSegment::Field("value")
        ]
    );

    let mut operation = invocation();
    operation.models.push(ModelBinding {
        model: owner(),
        digest: zero_digest(),
    });
    operation.parameters.push(binding("arg", 0));
    operation.created.push(object("new"));
    operation.deleted.push(object("old"));
    operation.arena = vec![
        ValueNode::Text {
            value: "😀".into()
        },
        ValueNode::Record {
            declaration: qualified("Record"),
            fields: vec![field("n", 0)],
        },
        ValueNode::Reference {
            identity: object("a"),
        },
        ValueNode::Enum {
            declaration: qualified("E"),
            variant: symbol("V"),
        },
    ];
    let artifact =
        Invocation::new(identity(), operation.clone(), ArtifactLimits::default()).unwrap();
    // Labels 17 + model 27 + context 31 + operation/anchor 8 + self 36
    // + pre/post labels 34 + parameter 30 + deltas 78 + payload/record 38
    // + reference 37 + enum/variant 29.
    assert_eq!(artifact.usage().text_bytes, 365);
    let error = Invocation::new(
        identity(),
        operation,
        ArtifactLimits {
            artifact_bytes: 364,
            ..ArtifactLimits::default()
        },
    )
    .unwrap_err();
    assert_eq!(error.usage.text_bytes, 364);
    assert_eq!(error.usage.artifact_bytes, 0);
    assert_eq!(
        error.path,
        vec![
            DraftPathSegment::Field("arena"),
            DraftPathSegment::Index(3),
            DraftPathSegment::Field("variant")
        ]
    );
}

fn shared(depth: u32) -> SnapshotDraft {
    let mut draft = empty();
    if depth > 0 {
        draft.arena.push(ValueNode::Absent);
    }
    for index in 1..depth {
        draft.arena.push(ValueNode::Sequence {
            values: vec![ValueId::new(index - 1), ValueId::new(index - 1)],
        });
    }
    draft
}

#[test]
#[trace("TC-057", "FR-018-AC-6")]
fn generated_shared_arenas_obey_each_exact_lowered_dimension() {
    for depth in 1..=8 {
        let draft = shared(depth);
        let baseline = Snapshot::new(identity(), draft.clone(), ArtifactLimits::default()).unwrap();
        let exact = limits_from(&baseline);
        let admitted = Snapshot::new(identity(), draft.clone(), exact).unwrap();
        assert_eq!(admitted.bytes(), baseline.bytes());
        assert_eq!(admitted.usage().nodes, usize::try_from(depth).unwrap());
        assert_eq!(
            admitted.usage().entries,
            usize::try_from(2 * (depth - 1)).unwrap()
        );
        assert_eq!(admitted.usage().depth, usize::try_from(depth).unwrap());
        for dimension in 0..4 {
            for zero in [false, true] {
                let mut lower = exact;
                let limit = match dimension {
                    0 => &mut lower.artifact_bytes,
                    1 => &mut lower.nodes,
                    2 => &mut lower.entries,
                    3 => &mut lower.depth,
                    _ => unreachable!(),
                };
                if *limit == 0 {
                    continue;
                }
                *limit = if zero { 0 } else { *limit - 1 };
                let error = Snapshot::new(identity(), draft.clone(), lower).unwrap_err();
                assert_eq!(
                    error.code,
                    Code::ResourceExhausted,
                    "depth {depth}, dimension {dimension}"
                );
                assert!(error.is_incomplete());
                assert!(error.usage.artifact_bytes <= lower.artifact_bytes);
                assert!(error.usage.nodes <= lower.nodes);
                assert!(error.usage.entries <= lower.entries);
                assert!(error.usage.depth <= lower.depth);
            }
        }
    }
}

#[test]
#[trace("TC-057", "FR-018-AC-6")]
fn every_metadata_entry_class_is_counted_before_its_work() {
    let mut snapshot = empty();
    snapshot.models.push(ModelBinding {
        model: owner(),
        digest: zero_digest(),
    });
    snapshot.populations.push(Population {
        model: owner(),
        record: symbol("Node"),
        universe: symbol("nodes"),
        complete: true,
        objects: vec![ObjectEntry {
            key: "a".into(),
            fields: vec![field("n", 0)],
        }],
    });
    snapshot.values.push(binding("state", 0));
    snapshot.arena = vec![
        ValueNode::Absent,
        ValueNode::Record {
            declaration: qualified("Record"),
            fields: vec![field("n", 0)],
        },
        ValueNode::Present {
            value: ValueId::new(1),
        },
        ValueNode::Sequence {
            values: vec![ValueId::new(2), ValueId::new(2)],
        },
    ];
    let artifact = Snapshot::new(identity(), snapshot.clone(), ArtifactLimits::default()).unwrap();
    assert_eq!(artifact.usage().entries, 9);
    for entries in 0..=9 {
        let result = Snapshot::new(
            identity(),
            snapshot.clone(),
            ArtifactLimits {
                entries,
                ..ArtifactLimits::default()
            },
        );
        if entries == 9 {
            assert_eq!(result.unwrap().bytes(), artifact.bytes());
        } else {
            let error = result.unwrap_err();
            assert_eq!(error.code, Code::ResourceExhausted);
            assert!(error.usage.entries <= entries);
        }
    }
    let mut operation = invocation();
    operation.models.push(ModelBinding {
        model: owner(),
        digest: zero_digest(),
    });
    operation.parameters.push(binding("arg", 0));
    operation.result = Some(ValueId::new(0));
    operation.created.push(object("new"));
    operation.deleted.push(object("old"));
    operation.arena = vec![ValueNode::Boolean { value: true }];
    let artifact =
        Invocation::new(identity(), operation.clone(), ArtifactLimits::default()).unwrap();
    assert_eq!(artifact.usage().entries, 5);
    for entries in 0..=5 {
        let result = Invocation::new(
            identity(),
            operation.clone(),
            ArtifactLimits {
                entries,
                ..ArtifactLimits::default()
            },
        );
        if entries == 5 {
            assert_eq!(result.unwrap().bytes(), artifact.bytes());
        } else {
            let error = result.unwrap_err();
            assert_eq!(error.code, Code::ResourceExhausted);
            assert!(error.usage.entries <= entries);
        }
    }
}

#[test]
#[trace("TC-057", "FR-018-AC-6")]
fn hard_entry_and_depth_limits_ignore_elevated_options() {
    let elevated = ArtifactLimits {
        artifact_bytes: usize::MAX,
        nodes: usize::MAX,
        entries: usize::MAX,
        depth: usize::MAX,
    };
    let mut draft = empty();
    draft.arena = vec![
        ValueNode::Absent,
        ValueNode::Sequence {
            values: vec![ValueId::new(0); 100_000],
        },
    ];
    let artifact = Snapshot::new(identity(), draft.clone(), elevated).unwrap();
    assert_eq!(artifact.usage().entries, 100_000);
    assert_eq!(artifact.usage().nodes, 2);
    let ValueNode::Sequence { values } = &mut draft.arena[1] else {
        unreachable!()
    };
    values.push(ValueId::new(0));
    let error = Snapshot::new(identity(), draft, elevated).unwrap_err();
    assert_eq!(error.code, Code::ResourceExhausted);
    assert_eq!(error.usage.artifact_bytes, 0);
    assert_eq!(
        error.path,
        vec![
            DraftPathSegment::Field("arena"),
            DraftPathSegment::Index(1),
            DraftPathSegment::Field("values")
        ]
    );

    assert_eq!(
        Snapshot::new(identity(), shared(64), elevated)
            .unwrap()
            .usage()
            .depth,
        64
    );
    let error = Snapshot::new(identity(), shared(65), elevated).unwrap_err();
    assert_eq!(error.code, Code::ResourceExhausted);
    assert_eq!(error.usage.depth, 64);
    assert_eq!(
        error.path,
        vec![
            DraftPathSegment::Field("arena"),
            DraftPathSegment::Index(64)
        ]
    );
}

#[test]
#[trace("TC-057", "FR-018-AC-6")]
fn hard_node_limit_and_coupled_byte_limit_remain_distinct() {
    let elevated = ArtifactLimits {
        artifact_bytes: usize::MAX,
        nodes: usize::MAX,
        entries: usize::MAX,
        depth: usize::MAX,
    };
    let mut draft = empty();
    draft.arena = vec![ValueNode::Absent; 100_000];
    let error = Snapshot::new(identity(), draft.clone(), elevated).unwrap_err();
    assert_eq!(error.code, Code::ResourceExhausted);
    assert_eq!(error.usage.nodes, 100_000);
    assert_eq!(error.usage.depth, 1);
    assert!(error.usage.artifact_bytes > 0);
    assert!(error.usage.artifact_bytes <= 1_048_576);
    assert!(error.path.is_empty()); // Encoding stopped; node/depth admission passed.
    draft.arena.push(ValueNode::Absent);
    let error = Snapshot::new(identity(), draft, elevated).unwrap_err();
    assert_eq!(error.code, Code::ResourceExhausted);
    assert_eq!(error.usage.nodes, 0);
    assert_eq!(error.usage.artifact_bytes, 0);
    assert_eq!(error.path, vec![DraftPathSegment::Field("arena")]);
}

#[test]
#[trace("TC-057", "FR-018-AC-6")]
fn exact_hard_output_bytes_and_escaped_text_expansion_are_bounded() {
    let mut draft = empty();
    draft.arena = vec![ValueNode::Text {
        value: String::new(),
    }];
    let overhead = Snapshot::new(identity(), draft.clone(), ArtifactLimits::default())
        .unwrap()
        .bytes()
        .len();
    let ValueNode::Text { value } = &mut draft.arena[0] else {
        unreachable!()
    };
    *value = "a".repeat(1_048_576 - overhead);
    let elevated = ArtifactLimits {
        artifact_bytes: usize::MAX,
        ..ArtifactLimits::default()
    };
    let artifact = Snapshot::new(identity(), draft.clone(), elevated).unwrap();
    assert_eq!(artifact.bytes().len(), 1_048_576);
    let ValueNode::Text { value } = &mut draft.arena[0] else {
        unreachable!()
    };
    value.push('a');
    let error = Snapshot::new(identity(), draft, elevated).unwrap_err();
    assert_eq!(error.code, Code::ResourceExhausted);
    assert!(error.path.is_empty());
    assert!(error.usage.artifact_bytes <= 1_048_576);

    for text in ["é😀", "\n\n\n", "\0\0\0", "\\\"\\\""] {
        let mut draft = empty();
        draft.arena = vec![ValueNode::Text { value: text.into() }];
        let artifact = Snapshot::new(identity(), draft.clone(), ArtifactLimits::default()).unwrap();
        assert_eq!(
            artifact.usage().text_bytes,
            identity().identity.len() + 1 + text.len()
        );
        let exact = limits_from(&artifact);
        assert_eq!(
            Snapshot::new(identity(), draft.clone(), exact)
                .unwrap()
                .bytes(),
            artifact.bytes()
        );
        let error = Snapshot::new(
            identity(),
            draft,
            ArtifactLimits {
                artifact_bytes: exact.artifact_bytes - 1,
                ..exact
            },
        )
        .unwrap_err();
        assert_eq!(error.code, Code::ResourceExhausted);
    }
}

#[test]
#[trace("TC-057", "TC-056", "FR-018-AC-6", "FR-018-AC-7")]
fn oversized_text_stops_before_encoding_and_retains_original_labels() {
    let selected = SourceIdentity {
        identity: "a".into(),
        revision: "b".into(),
    };
    let mut draft = empty();
    draft.arena = vec![ValueNode::Text {
        value: "a".repeat(1_048_575),
    }];
    let error = Snapshot::new(selected.clone(), draft, ArtifactLimits::default()).unwrap_err();
    assert_eq!(error.identity, selected);
    assert_eq!(error.usage.text_bytes, 2);
    assert_eq!(error.usage.artifact_bytes, 0);
    assert_eq!(
        error.path,
        vec![
            DraftPathSegment::Field("arena"),
            DraftPathSegment::Index(0),
            DraftPathSegment::Field("value")
        ]
    );
    let oversized = SourceIdentity {
        identity: "x".repeat(1_048_577),
        revision: "1".into(),
    };
    let error = Snapshot::new(oversized.clone(), empty(), ArtifactLimits::default()).unwrap_err();
    assert_eq!(error.identity, oversized);
    assert_eq!(error.usage.text_bytes, 0);
    assert_eq!(error.usage.artifact_bytes, 0);
    assert_eq!(error.code, Code::ResourceExhausted);
    assert_eq!(
        SnapshotRef::new(oversized.clone(), zero_digest())
            .unwrap_err()
            .code,
        Code::ResourceExhausted
    );
    assert_eq!(
        InvocationRef::new(oversized, zero_digest())
            .unwrap_err()
            .code,
        Code::ResourceExhausted
    );
}

#[test]
#[trace("TC-057", "FR-018-AC-6")]
fn empty_dimensions_and_flat_destruction_do_not_require_recursive_work() {
    let artifact = Snapshot::new(
        identity(),
        empty(),
        ArtifactLimits {
            nodes: 0,
            entries: 0,
            depth: 0,
            ..ArtifactLimits::default()
        },
    )
    .unwrap();
    assert_eq!(artifact.usage().nodes, 0);
    assert_eq!(artifact.usage().entries, 0);
    assert_eq!(artifact.usage().depth, 0);
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(|| {
            let artifact =
                Snapshot::new(identity(), shared(64), ArtifactLimits::default()).unwrap();
            assert_eq!(artifact.usage().depth, 64);
            drop(artifact);
            let mut large = empty();
            large.arena = vec![ValueNode::Absent; 100_001];
            let error = Snapshot::new(identity(), large, ArtifactLimits::default()).unwrap_err();
            assert_eq!(error.code, Code::ResourceExhausted);
            drop(error);
        })
        .unwrap()
        .join()
        .unwrap();
}
