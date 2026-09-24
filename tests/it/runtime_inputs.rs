// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-018: exact public runtime input construction, before population validation.

#[path = "../runtime_input_cases/identity.rs"]
mod identity_cases;
#[path = "../runtime_input_cases/limits.rs"]
mod limits;
#[path = "../runtime_input_cases/values.rs"]
mod values;

use ix_trace_rs::trace;
use qsl_foundation::{ByteDigest, Code, SourceIdentity};
use quire_contract_ir as ir;
use quire_spec_language::runtime::{
    ArtifactLimits, DraftPathSegment, FieldBinding, Invocation, InvocationDraft, InvocationRef,
    ModelBinding, ObjectEntry, ObjectIdentity, Population, QualifiedName, Snapshot, SnapshotDraft,
    SnapshotRef, ValueBinding, ValueId, ValueNode,
};

fn owner() -> ir::RequirementRef {
    ir::RequirementRef::parse("example/runtime", "RuntimeModel", 1).unwrap()
}

fn symbol(name: &str) -> ir::SymbolName {
    ir::SymbolName::new(name).unwrap()
}

fn qualified(name: &str) -> QualifiedName {
    QualifiedName {
        model: owner(),
        name: symbol(name),
    }
}

fn object(key: &str) -> ObjectIdentity {
    ObjectIdentity {
        model: owner(),
        record: symbol("Node"),
        universe: symbol("nodes"),
        key: key.into(),
    }
}

fn field(name: &str, value: u32) -> FieldBinding {
    FieldBinding {
        name: symbol(name),
        value: ValueId::new(value),
    }
}

fn binding(name: &str, value: u32) -> ValueBinding {
    ValueBinding {
        declaration: qualified(name),
        value: ValueId::new(value),
    }
}

fn zero_digest() -> ByteDigest {
    "sha256:0000000000000000000000000000000000000000000000000000000000000000"
        .parse()
        .unwrap()
}

fn invocation() -> InvocationDraft {
    let selected = SnapshotRef::new(identity(), zero_digest()).unwrap();
    InvocationDraft {
        models: Vec::new(),
        context: qualified("Node"),
        operation: symbol("step"),
        anchor: ir::AnchorName::new("step").unwrap(),
        self_object: object(""),
        pre: selected.clone(),
        post: selected,
        parameters: Vec::new(),
        result: None,
        created: Vec::new(),
        deleted: Vec::new(),
        arena: Vec::new(),
    }
}

fn identity() -> SourceIdentity {
    SourceIdentity::new("agent-ix", "snapshot:current", "git", "1")
}

fn empty() -> SnapshotDraft {
    SnapshotDraft {
        observation: ir::StateObservation::Current,
        models: Vec::new(),
        populations: Vec::new(),
        values: Vec::new(),
        arena: Vec::new(),
    }
}

#[test]
#[trace("TC-056", "FR-018-AC-3", "FR-018-AC-4")]
fn empty_snapshot_has_exact_independently_authored_envelope() {
    let snapshot = Snapshot::new(identity(), empty(), ArtifactLimits::default()).unwrap();
    let expected = br#"{"version":"native-state-input/1","kind":"snapshot","identity":{"authority":"agent-ix","identity":"snapshot:current","revision_namespace":"git","revision":"1"},"body":{"observation":"current","models":[],"populations":[],"values":[],"arena":[]}}"#;
    assert_eq!(snapshot.bytes(), expected);
    assert_eq!(snapshot.digest(), ByteDigest::of(expected));
    // Independently precomputed over the exact authored envelope, not through
    // the constructor or its serializer (sha256sum, retained literal oracle).
    assert_eq!(
        snapshot.digest().to_string(),
        "sha256:85d806fb536170880ba729e21c01bf27f845b30a6887d8424b52c97ba877b721"
    );
    assert_eq!(snapshot.reference().identity(), &identity());
    assert_eq!(snapshot.reference().digest(), snapshot.digest());
    assert_eq!(snapshot.usage().artifact_bytes, expected.len());
    assert_eq!(snapshot.draft(), &empty());
}

#[test]
#[trace("TC-055", "FR-018-AC-2", "FR-018-AC-5", "FR-018-AC-7")]
fn unused_self_edge_refuses_at_its_actual_draft_path() {
    let mut draft = empty();
    draft.arena.push(ValueNode::Present {
        value: ValueId::new(0),
    });
    let retained = draft.clone();
    let error = Snapshot::new(identity(), draft.clone(), ArtifactLimits::default()).unwrap_err();
    assert_eq!(error.code, Code::InvalidRuntimeInput);
    assert_eq!(error.identity, identity());
    assert_eq!(
        error.path,
        vec![
            DraftPathSegment::Field("arena"),
            DraftPathSegment::Index(0),
            DraftPathSegment::Field("value"),
        ]
    );
    assert!(!error.is_incomplete());
    assert_eq!(draft, retained);
    let _: &dyn std::error::Error = error.as_ref();
}

#[test]
#[trace("TC-057", "FR-018-AC-6")]
fn output_limit_admits_exact_bytes_and_refuses_one_below() {
    let baseline = Snapshot::new(identity(), empty(), ArtifactLimits::default()).unwrap();
    let exact = ArtifactLimits {
        artifact_bytes: baseline.bytes().len(),
        ..ArtifactLimits::default()
    };
    let admitted = Snapshot::new(identity(), empty(), exact).unwrap();
    assert_eq!(admitted.bytes(), baseline.bytes());
    let error = Snapshot::new(
        identity(),
        empty(),
        ArtifactLimits {
            artifact_bytes: exact.artifact_bytes - 1,
            ..exact
        },
    )
    .unwrap_err();
    assert_eq!(error.code, Code::ResourceExhausted);
    assert!(error.is_incomplete());
    assert!(error.usage.artifact_bytes < baseline.bytes().len());
}

/// TC-431 steps 1-2 (FR-018-AC-8): a snapshot names all four labels in its
/// bytes and reference; an empty or blank authority or revision namespace
/// refuses as `invalid_source_identity`; the revision namespace alone
/// changes the bytes and digest.
#[test]
#[trace("TC-431", "FR-018-AC-8")]
fn snapshots_carry_the_four_labels() {
    let snapshot =
        |labels: SourceIdentity| Snapshot::new(labels, empty(), ArtifactLimits::default());
    let git = snapshot(SourceIdentity::new("agent-ix", "s", "git", "1")).unwrap();
    let wire: serde_json::Value = serde_json::from_slice(git.bytes()).unwrap();
    assert_eq!(
        wire["identity"],
        serde_json::json!({
            "authority": "agent-ix",
            "identity": "s",
            "revision_namespace": "git",
            "revision": "1",
        })
    );
    assert_eq!(
        git.reference().identity(),
        &SourceIdentity::new("agent-ix", "s", "git", "1")
    );
    let semver = snapshot(SourceIdentity::new("agent-ix", "s", "semver", "1")).unwrap();
    assert_ne!(git.bytes(), semver.bytes());
    assert_ne!(git.digest(), semver.digest());

    for labels in [
        SourceIdentity::new("", "s", "git", "1"),
        SourceIdentity::new("agent-ix", "s", " ", "1"),
    ] {
        let error = snapshot(labels.clone()).unwrap_err();
        assert_eq!(error.code, Code::InvalidSourceIdentity, "{labels:?}");
        assert_eq!(error.identity, labels);
    }
}
