// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-570, FR-574, FR-575 (TC-1410, TC-1414, TC-1415): projecting a checked
//! global protocol onto one role (ADR-036 slice 3).

use ix_trace_rs::trace;
use qsl_analyze::protocol_analysis::{
    project, unsupported_construct, ProjectionOutcome, ProjectionRefusal, ProjectionRequest,
};
use qsl_replay::protocol_analysis::{
    replay_projection_counterexample, AnalysisOutcome, AssessedRootDraft, ChoiceBasis,
    GlobalStructure, InconclusiveCause, LocalEventKind, ProjectionBoundsDraft,
    ProjectionCounterexample, ProjectionWitness, ProtocolAnalysisEvidence, RunBudget,
    UnsupportedSlug, WalkDisagreement,
};
use qsl_replay::spine::replay_protocol_analysis;
use qsl_replay::{DecidingElement, ObservationIdentity, ReplaySource};
use quire_contract_model::{
    CheckedNodeId, CheckedPackageDispatchResult, CheckedPackageReadLimits, CheckedPackageV2,
};

use crate::fill::{document, package, PACKAGE};
use crate::fixture_ids::{effective_id, universe_id};

const MEMBERS: &str = "role Buyer on M::Cell;\nrole Seller on M::Cell;\nrole Warehouse on M::Cell;\n\
    channel Order from Buyer to Seller carries Ack ordering unordered delivery exactly-once-premise capacity 4 overflow reject;\n\
    channel Report from Warehouse to Seller carries Ack ordering unordered delivery exactly-once-premise capacity 4 overflow reject;\n\
    channel Decision from Seller to Warehouse carries Note ordering unordered delivery exactly-once-premise capacity 4 overflow reject;\n\
    channel Pick from Seller to Warehouse carries PickMsg ordering unordered delivery exactly-once-premise capacity 4 overflow reject;\n\
    channel Cancelled from Seller to Warehouse carries CancelMsg ordering unordered delivery exactly-once-premise capacity 4 overflow reject;\n\
    channel Back from Warehouse to Seller carries Ack ordering unordered delivery exactly-once-premise capacity 4 overflow reject;\n\
    channel Back2 from Warehouse to Seller carries PickMsg ordering unordered delivery exactly-once-premise capacity 4 overflow reject;\n\
    channel Fwd from Warehouse to Buyer carries Ack ordering unordered delivery exactly-once-premise capacity 4 overflow reject;";

const RECORDS: &str = "record Ack { ok: Boolean; }\nrecord Note { ok: Boolean; }\n\
    record PickMsg { ok: Boolean; }\nrecord CancelMsg { ok: Boolean; }\n";

fn source(members: &str, run: &str) -> String {
    format!(
        "{RECORDS}protocol Flow using pp over (k: M::Cell) on origin {{\n{members}\nrun {run}\n\
         finish Done as (zz: Boolean) {{ true }};\n}}\n"
    )
}

fn admitted_source(text: &str) -> Box<CheckedPackageV2> {
    let checked = package(text);
    let emission = qsl_package::emit_checked(&checked, qsl_package::EmitLimits::fixture())
        .expect("the protocol package emits");
    let mut evidence = emission.evidence().clone();
    evidence
        .insert_domain_package_document(PACKAGE, document().as_slice())
        .expect("fresh domain evidence");
    match quire_contract_model::read_checked_package(
        emission.package().bytes(),
        CheckedPackageReadLimits::bounded(),
        &evidence,
    ) {
        CheckedPackageDispatchResult::AdmittedV2(admitted) => admitted,
        other => panic!("IR admits the package: {other:?}"),
    }
}

fn flow(run: &str) -> Box<CheckedPackageV2> {
    admitted_source(&source(MEMBERS, run))
}

fn clause(package: &CheckedPackageV2) -> CheckedNodeId {
    package
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| &*node.semantic_form == "protocol_clause")
        .expect("one protocol clause")
        .node_id
        .clone()
}

fn named(package: &CheckedPackageV2, last: &str) -> CheckedNodeId {
    package
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| {
            node.declaration.as_ref().is_some_and(|declaration| {
                declaration.qualified_name.last().map(|segment| &**segment) == Some(last)
            })
        })
        .unwrap_or_else(|| panic!("a node named {last}"))
        .node_id
        .clone()
}

fn root() -> AssessedRootDraft {
    AssessedRootDraft {
        observation: Some(ObservationIdentity {
            authority: "test".to_owned(),
            identity: "protocol-claim".to_owned(),
        }),
        object: Some(quire_exact::ObjectReference::new(
            universe_id("u"),
            effective_id("e"),
            quire_exact::ObjectId::new("flow-1").unwrap(),
        )),
    }
}

fn request(package: &CheckedPackageV2, role: &str) -> ProjectionRequest {
    ProjectionRequest {
        protocol: clause(package),
        definition_key: Some("ix://test/Flow".to_owned()),
        role: named(package, role),
        profile: Some("bounded-static".to_owned()),
        bounds: Some(ProjectionBoundsDraft {
            max_depth: Some(64),
        }),
        assessed_root: Some(root()),
        budget: RunBudget::default(),
    }
}

fn run(package: &CheckedPackageV2, role: &str) -> ProjectionOutcome {
    project(package, request(package, role)).expect("the request is admitted")
}

fn witness(outcome: ProjectionOutcome) -> ProjectionWitness {
    match outcome.outcome {
        AnalysisOutcome::Proved(witness) => witness,
        other => panic!("expected a sound projection: {other:?}"),
    }
}

fn counterexample(outcome: ProjectionOutcome) -> ProjectionCounterexample {
    match outcome.outcome {
        AnalysisOutcome::Refuted(counterexample) => counterexample,
        other => panic!("expected a counterexample: {other:?}"),
    }
}

/// `Buyer` places an order; `Seller` reads it and owns `ship | cancel`.
fn worked(ship: &str, cancel: &str, before: &str) -> Box<CheckedPackageV2> {
    flow(&format!(
        "sequence Main {{\n{before}\
         send Placed via Order as (placed: Ack) {{ true }};\n\
         receive Got via Order of Placed as (got: Ack) {{ true }};\n\
         choice Decide by Seller visible (got.ok) {{\n\
           case no when {{ not got.ok }} sequence Cancel {{ {cancel} }}\n\
           case yes when {{ got.ok }} sequence Ship {{ {ship} }}\n\
         }} }}"
    ))
}

const PICK: &str = "send PickIt via Pick as (p: PickMsg) { true };\n\
    receive PickedUp via Pick of PickIt as (q: PickMsg) { true };";
const PICK2: &str = "send PickIt2 via Pick as (p2: PickMsg) { true };\n\
    receive PickedUp2 via Pick of PickIt2 as (q2: PickMsg) { true };";
const CANCELLED: &str = "send Cn via Cancelled as (c: CancelMsg) { true };\n\
    receive Cn2 via Cancelled of Cn as (d: CancelMsg) { true };";

/// FR-570-AC-6 (TC-1410 step 3, first variant): `receive pick` against end.
#[trace("TC-1410", "FR-570-AC-6", "FR-570-AC-5", "FR-174-AC-2")]
#[test]
fn the_worked_example_without_a_distinguishing_message_has_a_counterexample() {
    let package = worked(PICK, "", "");
    let found = counterexample(run(&package, "Warehouse"));
    assert_eq!(found.owner, named(&package, "Seller"));
    assert_eq!(found.choice, named(&package, "Decide"));
    assert_eq!(found.role, named(&package, "Warehouse"));
    let action = found.action_a.as_ref().expect("A is the receive");
    assert_eq!(action.at.node, named(&package, "PickedUp"));
    assert_eq!(found.action_b, None, "B is the end of the branch");
    assert_eq!(
        found
            .path_a
            .iter()
            .map(|at| at.node.clone())
            .collect::<Vec<_>>(),
        [named(&package, "PickIt"), named(&package, "PickedUp")]
    );
    assert!(found.path_b.is_empty());
    assert_eq!(found.length(), 2);
    assert_eq!(found.case_a, named(&package, "Ship"));
    assert_eq!(found.case_b, named(&package, "Cancel"));
}

/// FR-570-AC-6 (step 3, second variant): distinct first-receive message
/// types from one peer let `Warehouse` learn the branch.
#[trace("TC-1410", "FR-570-AC-6", "FR-570-AC-4", "FR-174-AC-5")]
#[test]
fn distinct_first_receive_types_from_one_peer_distinguish_the_branches() {
    let package = worked(PICK, CANCELLED, "");
    let found = witness(run(&package, "Warehouse"));
    let [evidence] = &found.merge_evidence[..] else {
        panic!("one choice");
    };
    assert_eq!(
        evidence.basis,
        ChoiceBasis::ReceiveDistinguished {
            peer: named(&package, "Seller")
        }
    );
    assert!(found
        .local_graph
        .events
        .iter()
        .any(|event| event.kind == LocalEventKind::ExternalChoice));
}

/// FR-570-AC-6 (step 3, third variant): an earlier message the role sent,
/// whose payload the choice's decision fact reads, carries the outcome.
#[trace("TC-1410", "FR-570-AC-6", "FR-570-AC-4", "FR-053-AC-1")]
#[test]
fn an_earlier_message_whose_payload_the_decision_reads_carries_the_outcome() {
    let package = flow(&format!(
        "sequence Main {{\n\
         send Rep via Report as (rep: Ack) {{ true }};\n\
         receive Got via Report of Rep as (got: Ack) {{ true }};\n\
         choice Decide by Seller visible (got.ok) {{\n\
           case no when {{ not got.ok }} sequence Cancel {{ }}\n\
           case yes when {{ got.ok }} sequence Ship {{ {PICK} }}\n\
         }} }}"
    ));
    let found = witness(run(&package, "Warehouse"));
    assert!(matches!(
        found.merge_evidence[0].basis,
        ChoiceBasis::Informed { .. }
    ));
    // The owner holds an internal choice.
    let owner = witness(run(&package, "Seller"));
    assert!(owner
        .local_graph
        .events
        .iter()
        .any(|event| event.kind == LocalEventKind::InternalChoice));
}

/// FR-570-AC-6 (step 3, fourth variant): a `decision` message of one type in
/// both branches that no decision fact reads does not distinguish them.
#[trace("TC-1410", "FR-570-AC-6", "FR-570-AC-5")]
#[test]
fn a_decision_message_no_fact_reads_does_not_distinguish_the_branches() {
    let before = "send Dec via Decision as (dec: Note) { true };\n\
        receive Dec2 via Decision of Dec as (dec2: Note) { true };\n";
    let package = worked(PICK, "", before);
    let found = counterexample(run(&package, "Warehouse"));
    assert_eq!(found.action_b, None);
}

/// FR-570-AC-4: pairwise equal (direction, peer, message type) first events
/// merge, recursing on the continuations.
#[trace("TC-1410", "FR-570-AC-4", "FR-174-AC-5")]
#[test]
fn equal_first_actions_merge_and_recurse_on_their_continuations() {
    let package = worked(PICK, PICK2, "");
    let found = witness(run(&package, "Warehouse"));
    let [evidence] = &found.merge_evidence[..] else {
        panic!("one choice");
    };
    assert_eq!(evidence.basis, ChoiceBasis::MustMerge);
    assert_eq!(evidence.merged.len(), 1, "the receives pair off");
    assert_eq!(
        found.local_graph.alternatives.len(),
        1,
        "the paired receives are alternatives, not concurrent events"
    );
    // A continuation that differs after an equal first action does not merge.
    let longer = format!("{PICK}\nsend Tail via Back as (t: Ack) {{ true }};");
    let other = PICK2;
    let package = worked(&longer, other, "");
    let found = counterexample(run(&package, "Warehouse"));
    assert_eq!(
        found.action_a.as_ref().map(|a| a.at.node.clone()),
        Some(named(&package, "Tail"))
    );
    assert_eq!(found.action_b, None);
}

/// FR-570-AC-4: a send against a receive, and sends with differing message
/// types or peers, do not merge.
#[trace("TC-1410", "FR-570-AC-4", "FR-570-AC-5", "FR-174-AC-5")]
#[test]
fn a_send_against_a_receive_and_differing_sends_do_not_merge() {
    let send = |name: &str, channel: &str, record: &str| {
        format!("send {name} via {channel} as (x{name}: {record}) {{ true }};")
    };
    let cases = [
        // A send against a receive.
        (send("S1", "Back", "Ack"), PICK.to_owned()),
        // Two sends, differing message types.
        (send("S1", "Back", "Ack"), send("S2", "Back2", "PickMsg")),
        // Two sends, differing peers.
        (send("S1", "Back", "Ack"), send("S2", "Fwd", "Ack")),
    ];
    for (ship, cancel) in cases {
        let package = worked(&ship, &cancel, "");
        let outcome = run(&package, "Warehouse");
        assert!(
            matches!(outcome.outcome, AnalysisOutcome::Refuted(_)),
            "{ship} against {cancel}: {:?}",
            outcome.outcome
        );
    }
}

/// FR-570-AC-5: for two failing choices the counterexample is the smallest
/// by length, then by choice node id.
#[trace("TC-1410", "FR-570-AC-5", "FR-174-AC-2")]
#[test]
fn the_counterexample_is_the_shortest_then_the_first_choice() {
    let choice = |name: &str, branch: &str| {
        format!(
            "choice {name} by Seller visible () {{\n\
             case a when {{ true }} sequence {name}A {{ {branch} }}\n\
             case b when {{ false }} sequence {name}B {{ }}\n}}"
        )
    };
    let receive = |tag: &str| {
        format!(
            "send P{tag} via Pick as (p{tag}: PickMsg) {{ true }};\n\
             receive R{tag} via Pick of P{tag} as (r{tag}: PickMsg) {{ true }};"
        )
    };
    let send = |tag: &str| format!("send W{tag} via Back as (w{tag}: Ack) {{ true }};");
    // Equal lengths: the choice that sorts first wins.
    let package = flow(&format!(
        "sequence Main {{\n{}\n{}\n}}",
        choice("First", &receive("1")),
        choice("Second", &receive("2"))
    ));
    assert_eq!(
        counterexample(run(&package, "Warehouse")).choice,
        named(&package, "First")
    );
    // A shorter path in the later choice wins over the earlier choice.
    let package = flow(&format!(
        "sequence Main {{\n{}\n{}\n}}",
        choice("First", &receive("1")),
        choice("Second", &send("2"))
    ));
    let found = counterexample(run(&package, "Warehouse"));
    assert_eq!(found.choice, named(&package, "Second"));
    assert_eq!(found.length(), 1);
}

fn closure(witness: &ProjectionWitness) -> Vec<(u32, u32)> {
    let mut reach: Vec<(u32, u32)> = witness
        .local_graph
        .edges
        .iter()
        .map(|(from, to)| (from.0, to.0))
        .collect();
    loop {
        let mut grown = reach.clone();
        for (a, b) in &reach {
            for (c, d) in &reach {
                if b == c {
                    grown.push((*a, *d));
                }
            }
        }
        grown.sort_unstable();
        grown.dedup();
        if grown.len() == reach.len() {
            return reach;
        }
        reach = grown;
    }
}

fn parallel_flow(key: &str) -> Box<CheckedPackageV2> {
    let members = format!(
        "role Buyer on M::Cell;\nrole Seller on M::Cell;\nrole Warehouse on M::Cell;\n\
         channel Ch from Buyer to Seller carries Boolean ordering {key} \
         delivery exactly-once-premise capacity 4 overflow reject;\n\
         channel Side from Seller to Warehouse carries Boolean ordering unordered \
         delivery exactly-once-premise capacity 4 overflow reject;"
    );
    admitted_source(&source(
        &members,
        "sequence Main {\n\
         send S1 via Ch as (a: Boolean) { true };\n\
         send S2 via Ch as (b: Boolean) { true };\n\
         parallel Both {\n\
           branch left receive R1 via Ch of S1 as (c: Boolean) { true };\n\
           branch right receive R2 via Ch of S2 as (d: Boolean) { true };\n\
         } join all [left, right];\n\
         send Out via Side as (e: Boolean) { true };\n\
         receive In via Side of Out as (f: Boolean) { true };\n}",
    ))
}

/// FR-570-AC-1, AC-2, AC-3: the local graph's events and order equal the
/// global structure restricted through the correspondence table; one send and
/// one receive per communication; a `parallel` stays a partial order; two
/// receives of one FIFO key keep their order.
#[trace(
    "TC-1410",
    "FR-570-AC-1",
    "FR-570-AC-2",
    "FR-570-AC-3",
    "FR-174-AC-1",
    "FR-174-AC-4"
)]
#[test]
fn local_graphs_equal_the_global_structure_restricted_to_the_role() {
    for (key, ordered) in [
        ("fifo by (m: Boolean) { true }", true),
        ("fifo by (m: Boolean) { m }", false),
        ("unordered", false),
    ] {
        let package = parallel_flow(key);
        let structure =
            GlobalStructure::from_package(&package, &clause(&package), RunBudget::default())
                .unwrap();
        for role in ["Buyer", "Seller", "Warehouse"] {
            let found = witness(run(&package, role));
            let table = &found.correspondence.0;
            let local = closure(&found);
            for (first, a) in table {
                for (second, b) in table {
                    assert_eq!(
                        structure.precedes(first, second),
                        local.contains(&(a.0, b.0)),
                        "{role}: {first:?} -> {second:?} under {key}"
                    );
                }
            }
            assert_eq!(table.len(), found.local_graph.events.len(), "total and 1:1");
            let count = |wanted: fn(&LocalEventKind) -> bool| {
                found
                    .local_graph
                    .events
                    .iter()
                    .filter(|e| wanted(&e.kind))
                    .count()
            };
            let is_send = |kind: &LocalEventKind| matches!(kind, LocalEventKind::Send { .. });
            let is_receive = |kind: &LocalEventKind| matches!(kind, LocalEventKind::Receive { .. });
            match role {
                "Buyer" => assert_eq!((count(is_send), count(is_receive)), (2, 0)),
                "Seller" => assert_eq!((count(is_send), count(is_receive)), (1, 2)),
                _ => assert_eq!((count(is_send), count(is_receive)), (0, 1)),
            }
        }
        let seller = witness(run(&package, "Seller"));
        let region_events = seller
            .local_graph
            .events
            .iter()
            .filter(|e| matches!(e.kind, LocalEventKind::ParallelRegion))
            .count();
        assert_eq!(
            region_events, 1,
            "the parallel is a region, not interleavings"
        );
        let [r1, r2] = [named(&package, "R1"), named(&package, "R2")].map(|node| {
            seller.correspondence.0[&qsl_package::protocol_view::PortRef {
                node,
                port: qsl_package::protocol_view::Port::Enter,
            }]
        });
        let local = closure(&seller);
        assert_eq!(
            local.contains(&(r1.0, r2.0)),
            ordered,
            "FIFO keeps receive order exactly when the key is equal"
        );
        assert!(!local.contains(&(r2.0, r1.0)));
    }
}

/// FR-570-AC-7: a horizon smaller than the protocol needs is `inconclusive`
/// with no graph; a larger one decides.
#[trace("TC-1410", "FR-570-AC-7", "FR-174-AC-3")]
#[test]
fn a_horizon_short_of_the_protocol_is_inconclusive() {
    let package = parallel_flow("unordered");
    let mut short = request(&package, "Seller");
    short.bounds = Some(ProjectionBoundsDraft { max_depth: Some(2) });
    let outcome = project(&package, short).unwrap();
    assert!(matches!(
        outcome.outcome,
        AnalysisOutcome::Inconclusive(InconclusiveCause::Horizon)
    ));
}

/// PA-14: a run limit stops the run `failed`, stating the limit and value.
#[trace("TC-1410", "FR-570-AC-7")]
#[test]
fn a_run_limit_fails_the_run_naming_the_limit() {
    let package = parallel_flow("unordered");
    let mut limited = request(&package, "Seller");
    limited.budget = RunBudget {
        max_states: 3,
        max_transitions: 10_000,
    };
    match project(&package, limited).unwrap().outcome {
        AnalysisOutcome::Failed(limit) => {
            assert_eq!((limit.limit, limit.value), ("max_states", 3));
        }
        other => panic!("expected a run-limit failure: {other:?}"),
    }
    let mut limited = request(&package, "Seller");
    limited.budget = RunBudget {
        max_states: 100,
        max_transitions: 1,
    };
    assert!(matches!(
        project(&package, limited).unwrap().outcome,
        AnalysisOutcome::Failed(limit) if limit.limit == "max_transitions"
    ));
}

/// FR-575-AC-1, AC-3: each profile-external construct answers `unsupported`
/// with its slug and node, and no graph; an unknown profile word is refused.
#[trace("TC-1415", "FR-575-AC-1", "FR-575-AC-3", "FR-174-AC-3")]
#[test]
fn constructs_outside_the_profile_answer_unsupported_with_no_graph() {
    let cases: [(&str, &str, &str, UnsupportedSlug, &str); 7] = [
        (
            MEMBERS,
            "sequence Main { parallel Par { branch a sequence Ay { } branch b sequence Bee { } } \
             join any [a] outstanding cancel; }",
            "Par",
            UnsupportedSlug::UnsupportedJoin,
            "any",
        ),
        (
            MEMBERS,
            "sequence Main { repeat Loop by Buyer visible () max 1 while { true } \
             sequence Body { send RS via Order as (rs: Ack) { true }; } exhausted sequence Spent { } }",
            "Loop",
            UnsupportedSlug::Repeat,
            "",
        ),
        (
            MEMBERS,
            "sequence Main { check Gate using v { true }; }",
            "Gate",
            UnsupportedSlug::UnsupportedControl,
            "",
        ),
        (
            "role Buyer on M::Cell;\nrole Seller on M::Cell;\nrole Warehouse on M::Cell;\n\
             channel Order from Buyer to Seller carries Ack ordering unordered \
             delivery at-least-once capacity symbolic(Bound) overflow block;",
            "sequence Main { }",
            "Order",
            UnsupportedSlug::SymbolicQueueCapacity,
            "Bound",
        ),
        (
            MEMBERS,
            "sequence Main { parallel Par { branch a sequence Ay { } branch b sequence Bee { } } \
             join quorum(1) [a] outstanding continue; }",
            "Par",
            UnsupportedSlug::UnsupportedJoin,
            "quorum",
        ),
        (
            "role Buyer on M::Cell;\nrole Pool each M::Cell from cells max 2 lifetime workflow;",
            "sequence Main { }",
            "Pool",
            UnsupportedSlug::DynamicRolePopulation,
            "",
        ),
        (
            MEMBERS,
            "sequence Main { parallel Par { branch a sequence Ay { } branch b sequence Bee { } } \
             join predicate(both) [a, b] outstanding cancel; }",
            "Par",
            UnsupportedSlug::UnsupportedJoin,
            "predicate",
        ),
    ];
    for (members, run_text, node, slug, detail) in cases {
        let mut text = source(members, run_text);
        if detail == "predicate" {
            text = format!("function both using v(a: Boolean, b: Boolean): Boolean pure {{ a and b }}\n{text}");
        }
        let package = admitted_source(&text);
        let cause = unsupported_construct(&package, &clause(&package))
            .unwrap()
            .unwrap_or_else(|| panic!("{node}: a construct outside the profile"));
        assert_eq!(cause.slug, slug, "{node}");
        assert_eq!(cause.slug.code(), "unsupported_projection");
        if slug != UnsupportedSlug::SymbolicQueueCapacity {
            assert_eq!(cause.node, named(&package, node));
        }
        if !detail.is_empty() {
            assert_eq!(cause.detail.as_deref(), Some(detail));
        }
        let outcome = run(&package, "Warehouse");
        assert!(
            matches!(&outcome.outcome, AnalysisOutcome::Unsupported(found) if found.slug == slug),
            "{node}: {:?}",
            outcome.outcome
        );
    }
    let package = worked(PICK, "", "");
    let mut open = request(&package, "Warehouse");
    open.profile = Some("open".to_owned());
    assert!(matches!(
        project(&package, open),
        Err(ProjectionRefusal::InvalidProfile(word)) if word == "open"
    ));
}

/// FR-574-AC-1..5: the record of a decisive counterexample, and the root it
/// copies; a missing root or component is refused naming it.
#[trace(
    "TC-1414",
    "FR-574-AC-1",
    "FR-574-AC-2",
    "FR-574-AC-3",
    "FR-574-AC-4",
    "FR-574-AC-5",
    "FR-574-AC-7"
)]
#[test]
fn a_decisive_counterexample_carries_one_record_and_a_missing_root_is_refused() {
    let package = worked(PICK, "", "");
    let found = counterexample(run(&package, "Warehouse"));
    let record = &found.separating_witness;
    assert!(record.quantifier.is_none() && record.index.is_none());
    assert_eq!(
        record.deciding_element,
        DecidingElement::Protocol(
            qsl_replay::protocol_analysis::node_text(&named(&package, "PickedUp")).unwrap()
        )
    );
    assert_eq!(
        record.trace_position.as_ref().map(|p| p.as_str()),
        Some("1")
    );
    assert!(record.value_path.steps.is_empty());
    assert_eq!(record.value_path.observation.identity, "protocol-claim");
    // A sound projection carries no record.
    let sound = run(&worked(PICK, CANCELLED, ""), "Warehouse");
    assert!(matches!(sound.outcome, AnalysisOutcome::Proved(_)));
    // The root is a request input, not a key member.
    let mut other = request(&package, "Warehouse");
    other.assessed_root.as_mut().unwrap().observation = Some(ObservationIdentity {
        authority: "test".to_owned(),
        identity: "another".to_owned(),
    });
    let second = project(&package, other).unwrap();
    assert_eq!(second.key, run(&package, "Warehouse").key);
    // Absent root and absent components.
    for (edit, member) in [
        (0, "assessed_root"),
        (1, "assessed_root.observation"),
        (2, "assessed_root.object"),
    ] {
        let mut broken = request(&package, "Warehouse");
        match edit {
            0 => broken.assessed_root = None,
            1 => broken.assessed_root.as_mut().unwrap().observation = None,
            _ => broken.assessed_root.as_mut().unwrap().object = None,
        }
        match project(&package, broken) {
            Err(ProjectionRefusal::Missing(missing)) => assert_eq!(missing.member, member),
            other => panic!("expected a refusal naming {member}: {other:?}"),
        }
    }
}

/// FR-574-AC-4: the position is the index of the divergent action in path A.
#[trace("TC-1414", "FR-574-AC-4")]
#[test]
fn the_trace_position_is_the_divergent_actions_index_in_path_a() {
    let long = format!(
        "send Other via Order as (o1: Ack) {{ true }};\n\
         receive Other2 via Order of Other as (o2: Ack) {{ true }};\n\
         send Early via Decision as (e1: Note) {{ true }};\n\
         receive Early2 via Decision of Early as (e2: Note) {{ true }};\n{PICK}"
    );
    let package = worked(&long, "", "");
    let found = counterexample(run(&package, "Warehouse"));
    assert_eq!(
        found.path_a.len(),
        4,
        "two events of other roles precede the divergence"
    );
    assert_eq!(
        found
            .separating_witness
            .trace_position
            .as_ref()
            .map(|p| p.as_str()),
        Some("3")
    );
    // The first divergent action is the Decision receive, third event on.
    assert_eq!(
        found.action_a.as_ref().unwrap().at.node,
        named(&package, "Early2")
    );
    assert_eq!(found.path_a.last().unwrap().node, named(&package, "Early2"));
}

/// PA-20, FR-573-AC-3: the replay arm re-walks the paths and ends in the
/// stated divergence; a walk that does not settles as a disagreement.
#[trace("TC-1413", "FR-573-AC-3")]
#[test]
fn the_replay_arm_reproduces_a_counterexample_and_refuses_an_edited_one() {
    let package = worked(PICK, "", "");
    let found = counterexample(run(&package, "Warehouse"));
    let structure =
        GlobalStructure::from_package(&package, &clause(&package), RunBudget::default()).unwrap();
    assert_eq!(
        replay_projection_counterexample(&structure, &found, 64),
        Ok(())
    );
    let source = ReplaySource::ProtocolAnalysis(Box::new(ProtocolAnalysisEvidence {
        counterexample: found.clone(),
    }));
    let replayed =
        replay_protocol_analysis(&package, &clause(&package), &source, RunBudget::default())
            .unwrap();
    assert_eq!(
        replayed.category(),
        quire_semantic_value::catalog::Category::Success
    );
    // A step that is not an admitted successor.
    let mut skipped = found.clone();
    skipped.path_a.remove(0);
    assert!(matches!(
        replay_projection_counterexample(&structure, &skipped, 64),
        Err(WalkDisagreement::StepNotSuccessor(_))
    ));
    // A walk that does not end in the stated divergence: the end is swapped
    // for a path that stops early.
    let mut early = found.clone();
    early.path_a.pop();
    assert_eq!(
        replay_projection_counterexample(&structure, &early, 64),
        Err(WalkDisagreement::ActionMismatch)
    );
    let source = ReplaySource::ProtocolAnalysis(Box::new(ProtocolAnalysisEvidence {
        counterexample: early,
    }));
    let replayed =
        replay_protocol_analysis(&package, &clause(&package), &source, RunBudget::default())
            .unwrap();
    assert_eq!(
        replayed.category(),
        quire_semantic_value::catalog::Category::Inconclusive
    );
    // A role that is told the outcome has nothing to merge.
    let mut informed = found;
    informed.role = named(&package, "Seller");
    assert_eq!(
        replay_projection_counterexample(&structure, &informed, 64),
        Err(WalkDisagreement::NotMustMerge)
    );
}

/// PA-15: one key yields one result; the key carries the structural members.
#[trace("TC-1412")]
#[test]
fn two_runs_on_one_key_give_one_result() {
    let package = worked(PICK, CANCELLED, "");
    assert_eq!(run(&package, "Warehouse"), run(&package, "Warehouse"));
}

/// FR-570 (nested choice): a choice nested in a branch contributes its
/// alternatives to the branch's actions, so a role that receives in one
/// branch and nothing in the other is not projectable.
#[trace("TC-1410", "FR-570-AC-4", "FR-570-AC-5")]
#[test]
fn a_choice_nested_in_a_branch_counts_toward_the_outer_merge() {
    let inner = format!(
        "choice Inner by Seller visible (got.ok) {{\n\
           case a when {{ got.ok }} sequence InnerA {{ {PICK} }}\n\
           case b when {{ not got.ok }} sequence InnerB {{ {PICK2} }}\n}}"
    );
    let package = worked(&inner, "", "");
    let found = counterexample(run(&package, "Warehouse"));
    assert_eq!(found.choice, named(&package, "Decide"));
    assert_eq!(found.action_b, None);
}

fn two_senders(second: &str, guard_reads: &str) -> Box<CheckedPackageV2> {
    flow(&format!(
        "sequence Main {{\n\
         send Rep via Report as (rep: Ack) {{ true }};\n\
         receive Got via Report of Rep as (got: Ack) {{ true }};\n\
         send Placed via {second} as (placed: Ack) {{ true }};\n\
         receive Got2 via {second} of Placed as (got2: Ack) {{ true }};\n\
         choice Decide by Seller visible (got.ok, got2.ok) {{\n\
           case no when {{ not {guard_reads} }} sequence Cancel {{ }}\n\
           case yes when {{ {guard_reads} }} sequence Ship {{ {PICK} }}\n\
         }} }}"
    ))
}

/// FR-570 (told the outcome): a role is told only when every message the
/// decision reads is one it took part in.
#[trace("TC-1410", "FR-570-AC-4", "FR-570-AC-6", "FR-053-AC-1")]
#[test]
fn a_decision_reading_a_message_the_role_took_no_part_in_does_not_inform_it() {
    // `got` came from Warehouse, `got2` from Buyer, and the guards test `got2`.
    let package = two_senders("Order", "got2.ok");
    let found = counterexample(run(&package, "Warehouse"));
    assert_eq!(found.choice, named(&package, "Decide"));
    // Both messages are Warehouse's: the whole decision is known to it.
    let package = two_senders("Back", "got2.ok");
    let told = witness(run(&package, "Warehouse"));
    assert!(matches!(
        told.merge_evidence[0].basis,
        ChoiceBasis::Informed { .. }
    ));
    // The owner is told in both protocols.
    assert!(matches!(
        run(&package, "Seller").outcome,
        AnalysisOutcome::Proved(_)
    ));
}

/// FR-570 (horizon): a divergence in the first layer is a counterexample
/// even when the branch holds events beyond the horizon.
#[trace("TC-1410", "FR-570-AC-5", "FR-570-AC-7")]
#[test]
fn a_divergence_before_the_horizon_is_found_though_later_events_are_cut() {
    let tail = format!("{PICK}\nsend T1 via Back as (t1: Ack) {{ true }};");
    let package = worked(&tail, "", "");
    let mut shallow = request(&package, "Warehouse");
    shallow.bounds = Some(ProjectionBoundsDraft { max_depth: Some(5) });
    let found = match project(&package, shallow).unwrap().outcome {
        AnalysisOutcome::Refuted(found) => found,
        other => panic!("expected the shallow counterexample: {other:?}"),
    };
    assert_eq!(found.action_a.as_ref().unwrap().at.node, named(&package, "PickedUp"));
}
