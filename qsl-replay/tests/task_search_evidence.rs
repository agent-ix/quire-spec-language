// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-372 (FR-331): task-search evidence is a reachable incomplete frontier.

use ix_trace_rs::trace;
use qsl_eval::simulation::StateKey;
use qsl_foundation::{BackendId, RequestIndex};
use qsl_replay::{
    IncompleteCause, ModelCheckLimit, ModelIdentity, ObligationIdentity, OutcomeItem,
    TaskProductStateKey, TaskSearchEvidence, TaskSearchFrontier, TaskSearchLimit,
    TaskSearchLimitKind, TaskSearchNode, TaskSearchOpen, TaskSearchStopReason, TerminalRecord,
    Zone,
};
use quire_exact::NodeKey;

fn root() -> TaskSearchNode {
    TaskSearchNode {
        state: TaskProductStateKey::new(
            StateKey::from_canonical_bytes(b"task-root".to_vec()),
            empty_task_control(),
        )
        .into_state_key()
        .expect("state key lengths fit"),
        depth: 0,
        zone: Zone::zero(1),
        expanded: false,
    }
}

fn empty_task_control() -> Vec<u8> {
    let mut key = b"qsl-task-control-v1\0".to_vec();
    key.extend_from_slice(&0_u64.to_be_bytes()); // jobs
    key.push(0); // no running job
    key.extend_from_slice(&0_u64.to_be_bytes()); // activated tasks
    key.extend_from_slice(&0_u64.to_be_bytes()); // pending releases
    key
}

fn evidence<O>(identity: O, nodes: Vec<TaskSearchNode>) -> TaskSearchEvidence<O> {
    TaskSearchEvidence {
        identity,
        nodes,
        edges: Vec::new(),
        frontier: vec![TaskSearchFrontier {
            node: 0,
            reason: TaskSearchStopReason::StateLimit,
            open: vec![TaskSearchOpen::Successors, TaskSearchOpen::Delay],
        }],
        limit: Some(TaskSearchLimit {
            kind: TaskSearchLimitKind::States,
            value: 1,
            setting: "zone_check.max_symbolic_states".to_owned(),
        }),
    }
}

#[trace("TC-372", "FR-419-AC-6")]
#[test]
fn task_search_frontier_accepts_a_well_shaped_stopped_root() {
    assert!(evidence("schedulability-claim", vec![root()]).has_valid_shape());
}

#[trace("TC-372", "FR-419-AC-6")]
#[test]
fn task_search_frontier_accepts_multiple_initial_product_roots() {
    let second_root = TaskSearchNode {
        state: TaskProductStateKey::new(
            StateKey::from_canonical_bytes(b"task-second-root".to_vec()),
            empty_task_control(),
        )
        .into_state_key()
        .expect("state key lengths fit"),
        depth: 0,
        zone: Zone::zero(1),
        expanded: false,
    };
    let mut evidence = evidence("schedulability-claim", vec![root(), second_root]);
    evidence.frontier.push(TaskSearchFrontier {
        node: 1,
        reason: TaskSearchStopReason::StateLimit,
        open: vec![TaskSearchOpen::Successors],
    });
    assert!(evidence.has_valid_shape());
}

#[trace("TC-372", "FR-419-AC-6")]
#[test]
fn task_search_frontier_rejects_unreachable_or_unaccounted_nodes() {
    let unreachable = TaskSearchNode {
        state: TaskProductStateKey::new(
            StateKey::from_canonical_bytes(b"task-unreachable".to_vec()),
            empty_task_control(),
        )
        .into_state_key()
        .expect("state key lengths fit"),
        depth: 1,
        zone: Zone::zero(1),
        expanded: true,
    };
    assert!(!evidence("schedulability-claim", vec![root(), unreachable]).has_valid_shape());

    let mut unaccounted = evidence("schedulability-claim", vec![root()]);
    unaccounted.frontier.clear();
    assert!(!unaccounted.has_valid_shape());
}

#[trace("TC-372", "FR-419-AC-6")]
#[test]
fn task_search_frontier_rejects_a_limit_reason_mismatch() {
    let mut malformed = evidence("schedulability-claim", vec![root()]);
    malformed.frontier[0].reason = TaskSearchStopReason::TransitionLimit;
    assert!(!malformed.has_valid_shape());
}

#[trace("TC-372", "FR-419-AC-6")]
#[test]
fn task_search_frontier_rejects_a_malformed_scheduler_control_key() {
    let malformed_state = TaskProductStateKey::new(
        StateKey::from_canonical_bytes(b"task-source".to_vec()),
        b"not-a-task-control-key".to_vec(),
    )
    .into_state_key()
    .expect("outer frame lengths fit");
    let mut malformed = evidence("schedulability-claim", vec![root()]);
    malformed.nodes[0].state = malformed_state;
    assert!(!malformed.has_valid_shape());
}

#[trace("TC-372", "FR-419-AC-6")]
#[test]
fn task_search_frontier_accepts_unresolved_dynamics_without_claiming_a_limit() {
    let mut evidence = evidence("schedulability-claim", vec![root()]);
    evidence.limit = None;
    evidence.frontier[0].reason = TaskSearchStopReason::UnresolvedChoice;
    evidence.frontier[0].open = vec![TaskSearchOpen::Demand, TaskSearchOpen::Queue];
    assert!(evidence.has_valid_shape());
}

#[trace("TC-372", "FR-419-AC-6")]
#[test]
fn task_search_frontier_rejects_unresolved_choice_without_a_dynamic_dimension() {
    let mut evidence = evidence("schedulability-claim", vec![root()]);
    evidence.limit = None;
    evidence.frontier[0].reason = TaskSearchStopReason::UnresolvedChoice;
    evidence.frontier[0].open = vec![TaskSearchOpen::Successors, TaskSearchOpen::Delay];
    assert!(!evidence.has_valid_shape());
}

#[trace("TC-372", "FR-419-AC-6")]
#[test]
fn incomplete_terminal_serializes_the_task_search_frontier() {
    let node = NodeKey::new(
        "task-search",
        vec![quire_exact::PathSegment::Text("claim".to_owned())],
        7,
    )
    .expect("node identity");
    let node_id = node.to_string();
    let identity = ObligationIdentity::new(node.clone(), Some(ModelIdentity::new("orders")), 2);
    let evidence_value = evidence(identity, vec![root()]);
    let terminal = TerminalRecord::incomplete(
        RequestIndex::new(0),
        BackendId::new("qsl"),
        IncompleteCause::LimitReached {
            limit: ModelCheckLimit::ZoneMaxSymbolicStates,
            value: 1,
        },
    )
    .with_task_search(evidence_value)
    .expect("the evidence matches the stopped limit");
    let wire = serde_json::to_value(OutcomeItem::from_terminal(&terminal)).expect("serializes");
    assert_eq!(wire["task_search"]["type"], "task-search-frontier");
    assert_eq!(wire["task_search"]["identity"]["node_id"], node_id);
    assert_eq!(wire["task_search"]["identity"]["model"], "orders");
    assert_eq!(
        wire["task_search"]["limit"]["setting"],
        "zone_check.max_symbolic_states"
    );
    use base64::Engine as _;
    let expected_state_key =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(root().state.as_bytes());
    assert_eq!(
        wire["task_search"]["nodes"][0]["state"]["key"],
        expected_state_key
    );
    assert_eq!(
        wire["task_search"]["nodes"][0]["zone"][0][0],
        serde_json::json!({"bound":"0","strict":false})
    );
    assert!(wire["task_search"]["edges"].as_array().unwrap().is_empty());

    let mut unresolved = evidence(
        ObligationIdentity::new(node, Some(ModelIdentity::new("orders")), 2),
        vec![root()],
    );
    unresolved.limit = None;
    unresolved.frontier[0].reason = TaskSearchStopReason::UnresolvedChoice;
    unresolved.frontier[0].open = vec![TaskSearchOpen::Demand, TaskSearchOpen::Queue];
    let unresolved_terminal = TerminalRecord::incomplete(
        RequestIndex::new(1),
        BackendId::new("qsl"),
        IncompleteCause::BoundedSearchIncomplete,
    )
    .with_task_search(unresolved)
    .expect("unresolved dynamics does not claim a reached limit");
    let unresolved_wire =
        serde_json::to_value(OutcomeItem::from_terminal(&unresolved_terminal)).expect("serializes");
    assert!(unresolved_wire["task_search"].get("limit").is_none());
    assert_eq!(
        unresolved_wire["task_search"]["frontier"][0]["reason"],
        "unresolved-choice"
    );
}
