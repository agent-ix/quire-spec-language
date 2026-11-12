// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-572 (TC-1412): the structural key, its builder and its reader.

use ix_trace_rs::trace;

use super::*;

type Remove<T> = (&'static str, fn(&mut T));

fn channel(name: &str) -> ChannelPremise {
    ChannelPremise {
        channel: name.to_owned(),
        sender: "a".to_owned(),
        receiver: "b".to_owned(),
        message_type: "t".to_owned(),
        ordering: OrderingPremise::Unordered,
        delivery: DeliveryPremise::Unknown,
        capacity: CapacityPremise::Literal { value: 2 },
        overflow: OverflowPremise::Block,
    }
}

fn choice(name: &str) -> ChoicePremise {
    ChoicePremise {
        choice: name.to_owned(),
        owner: "a".to_owned(),
        visible: vec!["v".to_owned()],
    }
}

fn projection() -> ProjectionKeyDraft {
    ProjectionKeyDraft {
        definition_key: Some("pkg/Def".to_owned()),
        role: Some("buyer".to_owned()),
        profile: Some("v1".to_owned()),
        channels: Some(vec![channel("c")]),
        choices: Some(vec![choice("k")]),
        bounds: Some(BoundsDraft {
            max_depth: Some(8),
            max_history: None,
        }),
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
    let edits: [fn(&mut ProjectionKeyDraft); 6] = [
        |d| d.definition_key = Some("pkg/Other".to_owned()),
        |d| d.role = Some("seller".to_owned()),
        |d| d.profile = Some("v2".to_owned()),
        |d| d.channels = Some(vec![channel("other")]),
        |d| d.choices = Some(vec![choice("other")]),
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
    let edits: [fn(&mut RealizabilityKeyDraft); 10] = [
        |d| d.definition_key = Some("pkg/Other".to_owned()),
        |d| d.strategy_profile = Some("v2".to_owned()),
        |d| d.role_knowledge = Some(Some(BTreeSet::from(["fact".to_owned()]))),
        |d| d.channels = Some(vec![channel("other")]),
        |d| d.choices = Some(vec![choice("other")]),
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
}

#[trace("FR-572-AC-4", "TC-1412")]
#[test]
fn an_absent_member_is_refused_naming_it_by_builder_and_reader() {
    let missing = |member| MissingMember { member };
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
        let mut stored: serde_json::Value =
            serde_json::from_str(&built(projection()).to_json().unwrap()).unwrap();
        stored.as_object_mut().unwrap().remove(member);
        assert!(matches!(
            ProtocolAnalysisKey::from_json(&stored.to_string()),
            Err(KeyReadError::Missing(found)) if found == missing(member)
        ));
    }
    let mut depth = projection();
    depth.bounds = Some(BoundsDraft::default());
    assert_eq!(
        ProtocolAnalysisKey::projection(depth).unwrap_err(),
        missing("max_depth")
    );

    let realizabilities: [Remove<RealizabilityKeyDraft>; 9] = [
        ("definition_key", |d| d.definition_key = None),
        ("strategy_profile", |d| d.strategy_profile = None),
        ("role_knowledge", |d| d.role_knowledge = None),
        ("channels", |d| d.channels = None),
        ("choices", |d| d.choices = None),
        ("environment", |d| d.environment = None),
        ("bounds", |d| d.bounds = None),
        ("max_history", |d| {
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
    }
    let mut stored: serde_json::Value =
        serde_json::from_str(&real(realizability()).to_json().unwrap()).unwrap();
    for member in ["role_knowledge", "environment", "analyzer"] {
        let removed = stored.as_object_mut().unwrap().remove(member).unwrap();
        assert!(matches!(
            ProtocolAnalysisKey::from_json(&stored.to_string()),
            Err(KeyReadError::Missing(found)) if found == missing(member)
        ));
        stored
            .as_object_mut()
            .unwrap()
            .insert(member.to_owned(), removed);
    }
    let read = ProtocolAnalysisKey::from_json(&stored.to_string()).unwrap();
    assert_eq!(read, real(realizability()), "null is present, not absent");
    assert_eq!(missing("x").code(), "invalid_runtime_input");
    assert_eq!(missing("x").cause(), "missing-member");
}

#[trace("FR-572-AC-6", "TC-1412")]
#[test]
fn run_limits_and_the_order_of_premises_are_outside_the_key() {
    let mut reordered = projection();
    reordered.channels = Some(vec![channel("z"), channel("a")]);
    let mut sorted = projection();
    sorted.channels = Some(vec![channel("a"), channel("z")]);
    assert_eq!(built(reordered), built(sorted));
    let key = built(projection());
    let result: ProtocolAnalysisResult<(), ()> = ProtocolAnalysisResult {
        key: key.clone(),
        outcome: AnalysisOutcome::Failed(RunLimit {
            limit: "max_states",
            value: 1,
        }),
    };
    let other = ProtocolAnalysisResult {
        outcome: AnalysisOutcome::Failed(RunLimit {
            limit: "max_states",
            value: 2,
        }),
        ..result.clone()
    };
    assert_eq!(result.key, other.key);
    assert!(!matches!(result.outcome, AnalysisOutcome::Proved(())));
}
