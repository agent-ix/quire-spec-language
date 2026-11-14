// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-572 (TC-1412): the structural key, its builder and its reader.

use ix_trace_rs::trace;

use quire_contract_model::CheckedNodePathSegment;

use super::*;

type Remove<T> = (&'static str, fn(&mut T));

fn id(name: &str, ordinal: u64) -> CheckedNodeId {
    CheckedNodeId {
        package: "pkg".into(),
        path: vec![CheckedNodePathSegment::Name(name.into())],
        ordinal,
    }
}

fn channel(name: &str) -> ChannelPremise {
    ChannelPremise {
        channel: id(name, 0),
        sender: id("a", 0),
        receiver: id("b", 0),
        message_type: id("t", 0),
        ordering: OrderingPremise::Unordered,
        delivery: DeliveryPremise::Unknown,
        capacity: CapacityPremise::Literal { value: 2 },
        overflow: OverflowPremise::Block,
    }
}

fn choice(name: &str) -> ChoicePremise {
    ChoicePremise {
        choice: id(name, 0),
        owner: id("a", 0),
        visible: vec![id("v", 0)],
    }
}

fn projection() -> ProjectionKeyDraft {
    ProjectionKeyDraft {
        definition_key: Some("pkg/Def".to_owned()),
        role: Some("buyer".to_owned()),
        profile: Some("v1".to_owned()),
        channels: Some(vec![channel("c")]),
        choices: Some(vec![choice("k")]),
        bounds: Some(ProjectionBoundsDraft { max_depth: Some(8) }),
    }
}

fn realizability() -> RealizabilityKeyDraft {
    RealizabilityKeyDraft {
        definition_key: Some("pkg/Def".to_owned()),
        strategy_profile: Some("v1".to_owned()),
        role_knowledge: Some(None),
        channels: Some(vec![channel("c")]),
        choices: Some(vec![choice("k")]),
        environment: Some(Some(BTreeSet::new())),
        bounds: Some(BoundsDraft {
            max_depth: Some(8),
            max_history: Some(3),
        }),
        analyzer: Some(AnalyzerIdentity {
            identity: "qsl-analyze::protocol_analysis".to_owned(),
            version: "0.1.0".to_owned(),
        }),
    }
}

fn built(draft: ProjectionKeyDraft) -> ProtocolAnalysisKey {
    ProtocolAnalysisKey::projection(draft).unwrap()
}

fn real(draft: RealizabilityKeyDraft) -> ProtocolAnalysisKey {
    ProtocolAnalysisKey::realizability(draft).unwrap()
}

#[trace("FR-572-AC-1", "TC-1412")]
#[test]
fn each_projection_member_changes_the_key() {
    let base = built(projection());
    assert_eq!(base, built(projection()));
    let edits: [fn(&mut ProjectionKeyDraft); 9] = [
        |d| d.definition_key = Some("pkg/Other".to_owned()),
        |d| d.role = Some("seller".to_owned()),
        |d| d.profile = Some("v2".to_owned()),
        |d| d.channels = Some(vec![channel("other")]),
        |d| d.channels.as_mut().unwrap()[0].delivery = DeliveryPremise::AtLeastOnce,
        |d| d.channels.as_mut().unwrap()[0].overflow = OverflowPremise::Reject,
        |d| d.choices = Some(vec![choice("other")]),
        |d| d.choices.as_mut().unwrap()[0].visible.push(id("w", 0)),
        |d| d.bounds.as_mut().unwrap().max_depth = Some(9),
    ];
    for edit in edits {
        let mut draft = projection();
        edit(&mut draft);
        assert_ne!(built(draft), base);
    }
}

#[trace("FR-572-AC-2", "TC-1412")]
#[test]
fn each_realizability_member_changes_the_key() {
    let base = real(realizability());
    assert_eq!(base, real(realizability()));
    let edits: [fn(&mut RealizabilityKeyDraft); 12] = [
        |d| d.definition_key = Some("pkg/Other".to_owned()),
        |d| d.strategy_profile = Some("v2".to_owned()),
        |d| d.role_knowledge = Some(Some(BTreeSet::from(["fact".to_owned()]))),
        |d| d.channels = Some(vec![channel("other")]),
        |d| d.channels.as_mut().unwrap()[0].capacity = CapacityPremise::Literal { value: 9 },
        |d| d.choices = Some(vec![choice("other")]),
        |d| d.choices.as_mut().unwrap()[0].owner = id("z", 0),
        |d| d.environment = Some(Some(BTreeSet::from(["event".to_owned()]))),
        |d| d.bounds.as_mut().unwrap().max_depth = Some(9),
        |d| d.bounds.as_mut().unwrap().max_history = Some(4),
        |d| d.analyzer.as_mut().unwrap().version = "0.2.0".to_owned(),
        |d| d.analyzer.as_mut().unwrap().identity = "other".to_owned(),
    ];
    for edit in edits {
        let mut draft = realizability();
        edit(&mut draft);
        assert_ne!(real(draft), base);
    }
}

#[trace("FR-572-AC-3", "TC-1412")]
#[test]
fn a_not_stated_premise_is_present_and_unequal_to_a_declared_empty_one() {
    let mut stated = realizability();
    stated.role_knowledge = Some(Some(BTreeSet::new()));
    stated.environment = Some(None);
    let mut empty = realizability();
    empty.role_knowledge = Some(None);
    empty.environment = Some(Some(BTreeSet::new()));
    assert_ne!(real(stated), real(empty));
    let json = real(realizability()).to_json().unwrap();
    assert!(json.contains("\"role_knowledge\":null"), "{json}");
    assert!(!json.contains("digest"), "{json}");
    // Node members are typed ids, not text: the stored key reads back equal.
    let read = ProtocolAnalysisKey::from_json(&json).unwrap();
    assert_eq!(read, real(realizability()));
    let ProtocolAnalysisKey::Realizability(key) = read else {
        panic!("a realizability key");
    };
    assert_eq!(key.premises.channels[0].sender, id("a", 0));
}

/// FR-572 Behavior (PA-15): premises are ordered by checked node id, so
/// ordinal 9 sorts before ordinal 10 and input order does not change the key.
#[trace("TC-1412")]
#[test]
fn premises_are_ordered_by_checked_node_id() {
    let ordinals = |order: [u64; 2]| {
        let mut draft = projection();
        draft.channels = Some(order.map(channel_at).to_vec());
        built(draft)
    };
    assert_eq!(ordinals([10, 9]), ordinals([9, 10]));
    let ProtocolAnalysisKey::Projection(key) = ordinals([10, 9]) else {
        panic!("a projection key");
    };
    assert_eq!(key.channels[0].channel.ordinal, 9);
    assert_eq!(key.channels[1].channel.ordinal, 10);
}

fn channel_at(ordinal: u64) -> ChannelPremise {
    ChannelPremise {
        channel: id("c", ordinal),
        ..channel("c")
    }
}

fn stored(key: &ProtocolAnalysisKey) -> serde_json::Value {
    serde_json::from_str(&key.to_json().unwrap()).unwrap()
}

/// Remove the member at `path` (dotted, array indexes as `[n]`) and return
/// the document.
fn without(mut value: serde_json::Value, path: &[&str]) -> serde_json::Value {
    let (last, parents) = path.split_last().unwrap();
    let mut at = &mut value;
    for step in parents {
        at = match step.strip_prefix('[') {
            Some(index) => {
                let index: usize = index.trim_end_matches(']').parse().unwrap();
                &mut at[index]
            }
            None => &mut at[*step],
        };
    }
    at.as_object_mut().unwrap().remove(*last).unwrap();
    value
}

#[trace("FR-572-AC-4", "TC-1412")]
#[test]
fn an_absent_member_is_refused_naming_it_by_builder_and_reader() {
    let missing = |member: &str| MissingMember::new(member);
    let projections: [Remove<ProjectionKeyDraft>; 6] = [
        ("definition_key", |d| d.definition_key = None),
        ("role", |d| d.role = None),
        ("profile", |d| d.profile = None),
        ("channels", |d| d.channels = None),
        ("choices", |d| d.choices = None),
        ("bounds", |d| d.bounds = None),
    ];
    for (member, remove) in projections {
        let mut draft = projection();
        remove(&mut draft);
        assert_eq!(
            ProtocolAnalysisKey::projection(draft).unwrap_err(),
            missing(member)
        );
        let document = without(stored(&built(projection())), &[member]);
        assert!(matches!(
            ProtocolAnalysisKey::from_json(&document.to_string()),
            Err(KeyReadError::Missing(found)) if found == missing(member)
        ));
    }
    let mut depth = projection();
    depth.bounds = Some(ProjectionBoundsDraft::default());
    assert_eq!(
        ProtocolAnalysisKey::projection(depth).unwrap_err(),
        missing("bounds.max_depth")
    );
    let document = without(stored(&built(projection())), &["bounds", "max_depth"]);
    assert!(matches!(
        ProtocolAnalysisKey::from_json(&document.to_string()),
        Err(KeyReadError::Missing(found)) if found == missing("bounds.max_depth")
    ));

    let realizabilities: [Remove<RealizabilityKeyDraft>; 10] = [
        ("definition_key", |d| d.definition_key = None),
        ("strategy_profile", |d| d.strategy_profile = None),
        ("role_knowledge", |d| d.role_knowledge = None),
        ("channels", |d| d.channels = None),
        ("choices", |d| d.choices = None),
        ("environment", |d| d.environment = None),
        ("bounds", |d| d.bounds = None),
        ("bounds.max_depth", |d| {
            d.bounds.as_mut().unwrap().max_depth = None
        }),
        ("bounds.max_history", |d| {
            d.bounds.as_mut().unwrap().max_history = None
        }),
        ("analyzer", |d| d.analyzer = None),
    ];
    for (member, remove) in realizabilities {
        let mut draft = realizability();
        remove(&mut draft);
        assert_eq!(
            ProtocolAnalysisKey::realizability(draft).unwrap_err(),
            missing(member)
        );
        let path: Vec<&str> = member.split('.').collect();
        let document = without(stored(&real(realizability())), &path);
        assert!(matches!(
            ProtocolAnalysisKey::from_json(&document.to_string()),
            Err(KeyReadError::Missing(found)) if found == missing(member)
        ));
    }
    // Nested premise and analyzer members are named by path.
    let nested: [(&[&str], &str); 8] = [
        (&["channels", "[0]", "delivery"], "channels[0].delivery"),
        (&["channels", "[0]", "overflow"], "channels[0].overflow"),
        (&["channels", "[0]", "sender"], "channels[0].sender"),
        (
            &["channels", "[0]", "ordering", "binder"],
            "channels[0].ordering.binder",
        ),
        (&["choices", "[0]", "visible"], "choices[0].visible"),
        (&["choices", "[0]", "owner"], "choices[0].owner"),
        (&["analyzer", "version"], "analyzer.version"),
        (&["analyzer", "identity"], "analyzer.identity"),
    ];
    for (path, member) in nested {
        let mut draft = realizability();
        draft.channels.as_mut().unwrap()[0].ordering = OrderingPremise::Fifo {
            binder: id("b", 0),
            key: id("k", 0),
        };
        let document = without(stored(&real(draft)), path);
        let error = ProtocolAnalysisKey::from_json(&document.to_string()).unwrap_err();
        assert!(
            matches!(&error, KeyReadError::Missing(found) if *found == missing(member)),
            "{member}: {error:?}"
        );
        assert_eq!(error.cause(), "missing-member");
    }
    assert_eq!(missing("x").code(), "invalid_runtime_input");
    assert_eq!(missing("x").cause(), "missing-member");

    // `null` is a present premise, not an absent one.
    let read = ProtocolAnalysisKey::from_json(&real(realizability()).to_json().unwrap());
    assert_eq!(read.unwrap(), real(realizability()));
}

#[trace("FR-572-AC-4", "TC-1412")]
#[test]
fn an_unknown_kind_and_a_foreign_member_are_not_missing_members() {
    let mut document = stored(&built(projection()));
    document["kind"] = serde_json::json!("sideways");
    let error = ProtocolAnalysisKey::from_json(&document.to_string()).unwrap_err();
    assert!(matches!(error, KeyReadError::UnknownKind(_)), "{error:?}");
    assert_eq!(error.cause(), "invalid-value");
    assert_eq!(error.code(), "invalid_runtime_input");

    // A projection has no `max_history`: a stored one is refused, not dropped.
    let mut document = stored(&built(projection()));
    document["bounds"]["max_history"] = serde_json::json!(3);
    let error = ProtocolAnalysisKey::from_json(&document.to_string()).unwrap_err();
    assert!(matches!(error, KeyReadError::Malformed(_)), "{error:?}");
    assert_eq!(error.cause(), "invalid-value");
}

/// FR-574-AC-3 (PA-24): the canonical string of a node id is its RFC 8785
/// wire text; the reader refuses text that is not exactly that.
#[trace("FR-574-AC-3", "TC-1414")]
#[test]
fn a_node_text_is_the_canonical_wire_text_of_the_id() {
    let node = id("c", 3);
    let text = node_text(&node).unwrap();
    assert_eq!(text, r#"{"ordinal":3,"package":"pkg","path":["c"]}"#);
    assert_eq!(read_node_text(&text).unwrap(), node);
    assert_eq!(
        read_node_text(r#"{"package":"pkg","path":["c"],"ordinal":3}"#),
        Err(NodeTextError::NotCanonical)
    );
    assert_eq!(read_node_text("node"), Err(NodeTextError::NotANodeId));
}

/// FR-572 Behavior: a run limit is outside the key, so two failed results
/// that differ only in the limit value share their key.
#[trace("TC-1412")]
#[test]
fn a_run_limit_is_carried_by_the_outcome_not_the_key() {
    let key = built(projection());
    let failed = |value| ProtocolAnalysisResult::<(), ()> {
        key: key.clone(),
        outcome: AnalysisOutcome::Failed(RunLimit {
            limit: crate::ModelCheckLimit::MaxStates,
            value,
        }),
    };
    assert_eq!(failed(1).key, failed(2).key);
    assert_ne!(failed(1), failed(2));
}
