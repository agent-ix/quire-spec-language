// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-519 (TC-1314): the protocol read surface of an admitted checked package,
//! over the TC-134 workflow package: its causal relation, its protocol view
//! and the typed views of the record-to-node map.

use std::collections::BTreeSet;

use qsl_semantics::check::{CheckedProtocol, EdgeKind as InEdge, Port as InPort};
use quire_contract_model::{CheckedNodeId, CheckedPackageV2, CheckedSemanticNodeV2, CheckedTerm};

use super::protocol_requirements::fixture_w::{
    document, tc134_clock, tc134_selections, tc134_source, TIMED_PROFILES_W,
};
use super::*;
use crate::protocol_view::{
    binder, causal_relation, control_operation, declaration, delivery, dependency, locus,
    operation_identity, owner_model, protocol_view, queue_capacity, relation_over, selection,
    source, temporal_binding, ControlOperation, Delivery, QueueCapacity, Scheduling, ViewRefusal,
};

/// The TC-134 workflow plus an `await` anchored at an event, so every edge
/// rule of QSpec FR-464 has an edge.
fn source_text() -> String {
    tc134_source()
        .replace(
            "on origin { always true }",
            "on origin { always[0,3] true }",
        )
        .replace(
            "    commit Finalized by Orders",
            "    await WaitEvent after Main::PaymentRetries::ChargeCycle::Applied using T \
             clock \"ticks\" within [0,1] \
             match event SeenEvent by Orders as (seenEvent: Boolean) { true }; \
             then sequence AfterEvent { } timeout sequence LateEvent { }\n\
             commit Finalized by Orders",
        )
}

struct Fixture {
    wire: Value,
    /// IR's own read of the emitted bytes.
    admitted: Box<CheckedPackageV2>,
    /// QSL's I2 read of the same bytes.
    qsl_read: Arc<CheckedPackageV2>,
    protocol: CheckedProtocol,
    activation_rows: Vec<quire_contract_model::CheckedProtocolActivationRow>,
}

fn fixture_over(text: &str) -> Fixture {
    let domain = document();
    let packages =
        qsl_semantics::model::intake::package_input([domain.as_slice()]).expect("model intake");
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n{TIMED_PROFILES_W}\
         model M = \"acme/orders\";\n{text}\n"
    );
    let graph = assemble_with_models(source.as_bytes(), &packages)
        .expect("unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the workflow checks: {refusals:?}"));
    let selections = tc134_selections(&graph);
    let [protocol] = graph.protocols() else {
        panic!("one protocol");
    };
    let protocol = protocol.clone();
    let package = CheckedPackage::link(graph);
    let activation_rows = package.protocol_activation_relation();
    let emission = crate::emit::emit_checked_with_temporal_selections(&package, &selections)
        .expect("the package emits");
    let mut evidence = read_evidence(&emission);
    evidence
        .insert_domain_package_document(package_identity(&document()[..]), &document()[..])
        .expect("a fresh evidence key");
    let admitted = match quire_contract_model::read_checked_package(
        emission.package.bytes(),
        quire_contract_model::CheckedPackageReadLimits::bounded(),
        &evidence,
    ) {
        quire_contract_model::CheckedPackageDispatchResult::AdmittedV2(admitted) => admitted,
        other => panic!("IR admits the package: {other:?}"),
    };
    let Read::Verified {
        admitted: qsl_read, ..
    } = read_with(&emission, &evidence)
    else {
        panic!("QSL's I2 reader admits the package");
    };
    Fixture {
        wire: wire(&emission),
        admitted,
        qsl_read,
        protocol,
        activation_rows,
    }
}

fn fixture() -> Fixture {
    fixture_over(&source_text())
}

fn node<'p>(package: &'p CheckedPackageV2, id: &CheckedNodeId) -> &'p CheckedSemanticNodeV2 {
    package
        .graph()
        .nodes
        .iter()
        .find(|node| &node.node_id == id)
        .expect("a node of the package")
}

/// The qualified name of the declared node `id`, as texts.
fn name(package: &CheckedPackageV2, id: &CheckedNodeId) -> Vec<String> {
    node(package, id)
        .declaration
        .as_ref()
        .expect("a declared node")
        .qualified_name
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn clause(package: &CheckedPackageV2) -> &CheckedSemanticNodeV2 {
    package
        .graph()
        .nodes
        .iter()
        .find(|node| &*node.semantic_form == "protocol_clause")
        .expect("one protocol clause")
}

fn path(segments: &[&str]) -> Vec<String> {
    segments.iter().map(ToString::to_string).collect()
}

/// The qualified name the in-process control `control` is emitted under: the
/// clause name, `run`, and the control names, without the `branch` and `case`
/// positions, which add no segment.
fn emitted_name(
    protocol: &CheckedProtocol,
    control: &qsl_semantics::check::CheckedControl,
) -> Vec<String> {
    let mut out = path(&[&protocol.name, "run"]);
    for (depth, segment) in control.path.iter().enumerate() {
        let prefix = &control.path[..=depth];
        let held = protocol
            .controls
            .controls
            .iter()
            .find(|candidate| candidate.path == prefix)
            .expect("an ancestor control");
        if !matches!(
            held.kind,
            qsl_forms::ProtocolNodeKind::Branch | qsl_forms::ProtocolNodeKind::Case
        ) {
            out.push(segment.clone());
        }
    }
    out
}

type Pair = (Vec<String>, InPort, Vec<String>, InPort, InEdge);

/// The compiled edge relation of the checked clause, with each control named
/// by its emitted qualified name.
fn in_process_pairs(protocol: &CheckedProtocol) -> BTreeSet<Pair> {
    let named = |node| {
        emitted_name(
            protocol,
            protocol
                .controls
                .control(node)
                .expect("a control of the tree"),
        )
    };
    protocol
        .edges
        .iter()
        .map(|edge| {
            (
                named(edge.from.node),
                edge.from.port,
                named(edge.to.node),
                edge.to.port,
                edge.kind,
            )
        })
        .collect()
}

fn derived_pairs(package: &CheckedPackageV2) -> Vec<Pair> {
    let clause = clause(package);
    causal_relation(package, &clause.node_id)
        .expect("the relation derives")
        .edges()
        .iter()
        .map(|edge| {
            (
                name(package, &edge.from.node),
                edge.from.port,
                name(package, &edge.to.node),
                edge.to.port,
                edge.kind,
            )
        })
        .collect()
}

/// The controls of the checked tree in the order the nodes carry them: a
/// control before the controls it holds, the cases and branches in label
/// order, the `branch` and `case` positions themselves omitted.
fn in_process_layout(protocol: &CheckedProtocol) -> Vec<Vec<String>> {
    use qsl_forms::ProtocolNodeKind as Kind;
    let tree = &protocol.controls;
    let mut layout = Vec::new();
    let mut pending = vec![tree.root().expect("a run root").node];
    while let Some(id) = pending.pop() {
        let control = tree.control(id).expect("a control of the tree");
        if !matches!(control.kind, Kind::Branch | Kind::Case) {
            layout.push(emitted_name(protocol, control));
        }
        let mut held = control.children.clone();
        if matches!(control.kind, Kind::Choice | Kind::Parallel) {
            held.sort_by_key(|child| tree.control(*child).expect("a child").name.clone());
        }
        pending.extend(held.into_iter().rev());
    }
    layout
}

/// FR-519-AC-1, FR-383-AC-16 (TC-1314 row 1; it calls
/// `protocol_view::causal_relation`; FR-519-AC-2's "no second derivation" is
/// not checked here): the relation derived from the emitted nodes holds the (source port, target port) pairs of the checked
/// clause's compiled edge relation, every edge rule present, the `repeat` back
/// edge included, strictly ascending and duplicate-free; the package read by IR
/// and the package read by QSL's I2 reader derive the same relation.
#[trace("FR-519-AC-1", "FR-383-AC-16", "TC-1314")]
#[test]
fn the_causal_relation_equals_the_checked_clauses_edge_relation() {
    let fixture = fixture();
    let expected = in_process_pairs(&fixture.protocol);
    let derived = derived_pairs(&fixture.admitted);
    let kinds: BTreeSet<InEdge> = derived.iter().map(|pair| pair.4).collect();
    assert_eq!(
        kinds,
        BTreeSet::from([
            InEdge::Sequence,
            InEdge::Branch,
            InEdge::Join,
            InEdge::RepeatProgress,
            InEdge::RepeatBack,
            InEdge::AwaitSuccess,
            InEdge::AwaitTimeout,
            InEdge::AwaitAnchor,
            InEdge::Message,
            InEdge::Effect,
        ]),
        "every edge rule of QSpec FR-464 contributes an edge"
    );
    assert_eq!(derived.iter().cloned().collect::<BTreeSet<_>>(), expected);

    let relation = causal_relation(&fixture.admitted, &clause(&fixture.admitted).node_id).unwrap();
    assert!(
        relation.edges().windows(2).all(|pair| pair[0] < pair[1]),
        "strictly ascending, no duplicate"
    );
    let back: Vec<_> = relation
        .edges()
        .iter()
        .filter(|edge| edge.kind == InEdge::RepeatBack)
        .collect();
    assert_eq!(back.len(), 2, "the repeats `PaymentRetries` and `Poll`");
    for edge in back {
        assert_eq!(edge.from.node, edge.to.node, "the body's exit to its entry");
        assert_eq!(
            (edge.from.port, edge.to.port),
            (InPort::Exit, InPort::Enter)
        );
    }

    assert_eq!(
        causal_relation(&fixture.qsl_read, &clause(&fixture.qsl_read).node_id).unwrap(),
        relation,
        "the relation does not depend on which reader admitted the package"
    );
}

/// FR-519-AC-3 (TC-1314 row 3): the view lists the clause, roles, channels,
/// queues, controls in layout order, compensations, the required temporal
/// clause, `terminal`, `scheduling` and the activation rows, each equal to what
/// the in-process checked clause holds.
#[trace("FR-519-AC-3", "FR-383-AC-19", "TC-1314")]
#[test]
fn the_protocol_view_lists_the_nodes_the_checked_clause_rebuilds() {
    let fixture = fixture();
    let package = &*fixture.admitted;
    let clause_node = clause(package);
    let view = protocol_view(package, &clause_node.node_id).unwrap();
    let protocol = &fixture.protocol;
    let prefix = |kind: &str, last: &str| path(&[&protocol.name, kind, last]);

    assert_eq!(view.clause(), &clause_node.node_id);
    let names = |ids: &[&CheckedNodeId]| -> Vec<Vec<String>> {
        ids.iter().map(|id| name(package, id)).collect()
    };
    let sorted = |mut names: Vec<Vec<String>>| {
        names.sort();
        names
    };
    assert_eq!(
        sorted(names(view.roles())),
        sorted(
            protocol
                .roles
                .iter()
                .map(|role| prefix("role", &role.name))
                .collect()
        )
    );
    assert_eq!(
        sorted(names(view.channels())),
        sorted(
            protocol
                .channels
                .iter()
                .map(|channel| prefix("channel", &channel.name))
                .collect()
        )
    );
    assert_eq!(
        view.queues().len(),
        protocol.channels.len(),
        "one queue per channel"
    );
    assert!(view
        .queues()
        .iter()
        .all(|id| node(package, id).declaration.is_none()));
    assert_eq!(
        sorted(names(view.compensations())),
        sorted(
            protocol
                .obligations
                .compensations
                .iter()
                .map(|template| prefix("compensate", &template.name))
                .collect()
        )
    );
    let [required] = view.requirements() else {
        panic!("one required temporal clause");
    };
    assert_eq!(&*node(package, required).semantic_form, "temporal_clause");
    assert_eq!(
        view.terminal().map(|id| name(package, id)),
        Some(path(&[&protocol.name, "terminal"]))
    );
    assert_eq!(view.scheduling(), Some(Scheduling::Adversarial));
    assert_eq!(
        name(package, view.finish()),
        path(&[&protocol.name, "finish", "Closed"])
    );
    assert_eq!(
        name(package, view.run()),
        path(&[&protocol.name, "run", "Main"])
    );

    // Layout order: a sequence in execution order, the branches and cases in
    // label order, a repeat's body then its exhausted control, an await's
    // event, then and timeout.
    let layout: Vec<String> = view
        .controls()
        .iter()
        .map(|id| name(package, id).last().cloned().unwrap())
        .collect();
    assert_eq!(
        layout,
        [
            "Main",
            "Requested",
            "Received",
            "PaymentRetries",
            "ChargeCycle",
            "Charged",
            "Applied",
            "RetriesExhausted",
            "Shipments",
            "East",
            "West",
            "Outcome",
            "Failed",
            "Done",
            "Poll",
            "Polled",
            "Limit",
            "WaitEvent",
            "SeenEvent",
            "AfterEvent",
            "LateEvent",
            "Finalized",
        ]
    );
    assert_eq!(
        names(view.controls()),
        in_process_layout(protocol),
        "the controls are the checked control tree's, in layout order"
    );

    // The activation rows are the in-process rows, node for node.
    let graph = qsl_semantics::check::wire_id::WireGraph::of_package_value(&fixture.wire).unwrap();
    let structural = |id: &CheckedNodeId| {
        qsl_semantics::check::wire_id::structural_id(
            &graph
                .key_of_value(&serde_json::to_value(id).unwrap())
                .unwrap(),
        )
    };
    let [row] = view.activation() else {
        panic!("one activation row");
    };
    let [expected] = fixture.activation_rows.as_slice() else {
        panic!("one in-process row");
    };
    use quire_contract_model::{
        CheckedProtocolActivation as Activation, CheckedProtocolActivationRow,
    };
    assert_eq!(
        CheckedProtocolActivationRow {
            clause: structural(&row.clause),
            activation: match &row.activation {
                Activation::Origin => Activation::Origin,
                Activation::Each { trigger } => Activation::Each {
                    trigger: structural(trigger),
                },
            },
            required_temporal: structural(&row.required_temporal),
        },
        *expected,
        "the whole row: clause, activation trigger and required clause"
    );
}

/// FR-519-AC-1 (TC-1314 row 1): a `parallel` whose join names only some of its
/// branches gets a `join` edge from those branch bodies alone, and the relation
/// still equals the checked clause's.
#[trace("FR-519-AC-1", "FR-383-AC-16", "TC-1314")]
#[test]
fn a_parallel_joins_only_the_branches_its_join_names() {
    let fixture = fixture_over(
        "protocol Flow using P over (view: M::OrderView) on origin {\n\
         role Orders on M::OrderView;\n\
         run sequence Main {\n\
         parallel Par { branch a sequence Ay { } branch b sequence Bee { } } join any [a];\n\
         }\n\
         finish End as (outcome: Boolean) { true };\n}",
    );
    let derived = derived_pairs(&fixture.admitted);
    assert_eq!(
        derived.iter().cloned().collect::<BTreeSet<_>>(),
        in_process_pairs(&fixture.protocol)
    );
    let joins: Vec<_> = derived
        .iter()
        .filter(|pair| pair.4 == InEdge::Join)
        .collect();
    assert_eq!(
        joins,
        [&(
            path(&["Flow", "run", "Main", "Par", "Ay"]),
            InPort::Exit,
            path(&["Flow", "run", "Main", "Par"]),
            InPort::Exit,
            InEdge::Join
        )],
        "only branch a is joined"
    );
    let branches = derived
        .iter()
        .filter(|pair| pair.4 == InEdge::Branch)
        .count();
    assert_eq!(branches, 2, "the fork enters both branches");
}

/// FR-519-AC-1 (TC-1314 row 1): an `await` anchored at a compensation checks,
/// emits `after` as kind `compensation` naming the template, and takes its
/// anchor edge from the enter port of the template's forward effect.
#[trace("FR-519-AC-1", "FR-466-AC-6", "TC-1314", "TC-948")]
#[test]
fn an_await_after_a_compensation_is_anchored_at_the_forward_effect() {
    let text = source_text().replace(
        "after Main::PaymentRetries::ChargeCycle::Applied",
        "after Refund",
    );
    assert!(text.contains("await WaitEvent after Refund using T"));
    let fixture = fixture_over(&text);
    let package = &*fixture.admitted;
    let clause_id = clause(package).node_id.clone();
    let refund = by_name(package, "Refund").node_id.clone();
    let forward = by_name(package, "Applied").node_id.clone();
    let wait = by_name(package, "WaitEvent");

    let CheckedTerm::Application { arguments, .. } = &wait.body else {
        panic!("an application body");
    };
    let after = arguments
        .iter()
        .find_map(|argument| match argument {
            CheckedTerm::Binding { name, value } if &**name == "after" => Some(&**value),
            _ => None,
        })
        .expect("an after argument");
    let CheckedTerm::Aggregate { members } = after else {
        panic!("after is a group");
    };
    let (mut kind, mut target) = (None, None);
    for member in members {
        match member {
            CheckedTerm::Binding { name, value } if &**name == "kind" => {
                let CheckedTerm::Literal { value, .. } = &**value else {
                    panic!("a text kind");
                };
                kind = Some(value.clone());
            }
            CheckedTerm::Binding { name, value } if &**name == "target" => {
                target = Some((**value).clone());
            }
            _ => {}
        }
    }
    assert_eq!(kind, Some(json!("compensation")));
    assert_eq!(target, Some(CheckedTerm::reference(refund)));

    assert_eq!(
        derived_pairs(package).into_iter().collect::<BTreeSet<_>>(),
        in_process_pairs(&fixture.protocol),
        "the emitted relation equals the checked clause's own"
    );

    let wait = wait.node_id.clone();
    let anchors: Vec<_> = causal_relation(package, &clause_id)
        .unwrap()
        .edges()
        .iter()
        .filter(|edge| edge.kind == InEdge::AwaitAnchor)
        .map(|edge| (edge.from.clone(), edge.to.clone()))
        .collect();
    use crate::protocol_view::PortRef;
    assert_eq!(
        anchors,
        [(
            PortRef {
                node: forward,
                port: InPort::Enter
            },
            PortRef {
                node: wait,
                port: InPort::Enter
            }
        )],
        "the forward effect's enter port, not an event's exit port"
    );
}

/// Two awaits after one template read one template node, and a template with
/// `commit never` is anchored at its forward effect alone. Both emit and
/// derive the same relation as the checked clause.
#[trace("FR-519-AC-1", "FR-466-AC-6", "TC-1314", "TC-948")]
#[test]
fn awaits_after_one_template_and_a_template_without_a_commit_check() {
    let second = "await WaitAgain after Refund using T clock \"ticks\" within [0,1] \
         match event SeenAgain by Orders as (seenAgain: Boolean) { true }; \
         then sequence AfterAgain { } timeout sequence LateAgain { }\n\
         await WaitPartial after PartialRefund using T clock \"ticks\" within [0,1] \
         match event SeenPartial by Orders as (seenPartial: Boolean) { true }; \
         then sequence AfterPartial { } timeout sequence LatePartial { }\n\
         commit Finalized by Orders";
    let text = source_text()
        .replace(
            "after Main::PaymentRetries::ChargeCycle::Applied",
            "after Refund",
        )
        .replace("commit Finalized by Orders", second);
    let fixture = fixture_over(&text);
    let package = &*fixture.admitted;
    let forward = by_name(package, "Applied").node_id.clone();
    let targets = ["WaitEvent", "WaitAgain", "WaitPartial"].map(|wait| {
        let CheckedTerm::Application { arguments, .. } = &by_name(package, wait).body else {
            panic!("an application body");
        };
        let after = arguments.iter().find_map(|argument| match argument {
            CheckedTerm::Binding { name, value } if &**name == "after" => Some(&**value),
            _ => None,
        });
        let Some(CheckedTerm::Aggregate { members }) = after else {
            panic!("after is a group");
        };
        members
            .iter()
            .find_map(|member| match member {
                CheckedTerm::Binding { name, value } if &**name == "target" => {
                    Some((**value).clone())
                }
                _ => None,
            })
            .expect("a target")
    });
    let refund = CheckedTerm::reference(by_name(package, "Refund").node_id.clone());
    let partial = CheckedTerm::reference(by_name(package, "PartialRefund").node_id.clone());
    assert_eq!(targets, [refund.clone(), refund, partial]);

    assert_eq!(
        derived_pairs(package).into_iter().collect::<BTreeSet<_>>(),
        in_process_pairs(&fixture.protocol)
    );
    let clause_id = clause(package).node_id.clone();
    let anchored: BTreeSet<_> = causal_relation(package, &clause_id)
        .unwrap()
        .edges()
        .iter()
        .filter(|edge| edge.kind == InEdge::AwaitAnchor)
        .map(|edge| (edge.from.node.clone(), edge.from.port))
        .collect();
    assert_eq!(anchored, BTreeSet::from([(forward, InPort::Enter)]));
    assert_eq!(
        causal_relation(package, &clause_id)
            .unwrap()
            .edges()
            .iter()
            .filter(|edge| edge.kind == InEdge::AwaitAnchor)
            .count(),
        3
    );
}

/// FR-519-AC-3 (TC-1314 row 3): a control held by two positions is refused
/// rather than listed twice.
#[trace("FR-519-AC-3", "TC-1314")]
#[test]
fn a_control_held_twice_is_refused_not_listed_twice() {
    let fixture = fixture();
    let package = &*fixture.admitted;
    let clause_id = clause(package).node_id.clone();
    let main = by_name(package, "Main").node_id.clone();
    let requested = by_name(package, "Requested").node_id.clone();
    let mut nodes = package.graph().nodes.clone();
    let node = nodes.iter_mut().find(|node| node.node_id == main).unwrap();
    let CheckedTerm::Application { arguments, .. } = &mut node.body else {
        panic!("an application body");
    };
    for argument in arguments {
        if let CheckedTerm::Binding { name, value } = argument {
            if &**name == "children" {
                let CheckedTerm::Aggregate { members } = &mut **value else {
                    panic!("children is a list");
                };
                members.push(CheckedTerm::reference(requested.clone()));
            }
        }
    }
    assert_eq!(
        relation_over(&nodes, &clause_id).unwrap_err(),
        ViewRefusal::MalformedMember {
            node: clause_id,
            member: "run"
        }
    );
}

/// FR-519-AC-4 (TC-1314 row 4): an absent id and a non-protocol node refuse
/// `UnknownProtocol`; a `protocol_reference` carrier refuses
/// `NotAProtocolClause`; the relation and the view refuse alike.
#[trace("FR-519-AC-4", "TC-1314")]
#[test]
fn an_unknown_or_non_clause_node_refuses_without_a_partial_result() {
    let fixture = fixture();
    let package = &*fixture.admitted;
    let clause_id = clause(package).node_id.clone();
    let mut absent = clause_id.clone();
    absent.ordinal += 1000;
    let temporal = package
        .graph()
        .nodes
        .iter()
        .find(|node| &*node.semantic_form == "temporal_clause")
        .expect("a temporal clause")
        .node_id
        .clone();
    for id in [&absent, &temporal] {
        let expected = ViewRefusal::UnknownProtocol { node: id.clone() };
        assert_eq!(protocol_view(package, id).unwrap_err(), expected);
        assert_eq!(causal_relation(package, id).unwrap_err(), expected);
    }

    // A predicate join over a function that reads its own protocol closes the
    // cycle through a `protocol_reference` carrier.
    let carrier_source = "function both using v(a: Boolean, b: Boolean): Boolean pure \
         { a and b and protocol Flow }\n\
         protocol Flow using p over (input: M::Order) on origin {\n\
         role Buyer on M::Order;\n\
         run parallel Par { branch a sequence Ay { } branch b sequence Bee { } } \
         join predicate(both) [a, b];\n\
         finish End as (outcome: Boolean) { true };\n}\n";
    let lowered = super::reachability::lower_with(
        super::protocol_graph::model(),
        super::protocol_graph::PROFILES,
        carrier_source,
    );
    let package = lowered.ir.as_ref().expect("the cyclic package is admitted");
    let carrier = package
        .graph()
        .nodes
        .iter()
        .find(|node| &*node.semantic_form == "protocol_reference")
        .expect("a protocol_reference carrier")
        .node_id
        .clone();
    let expected = ViewRefusal::NotAProtocolClause {
        node: carrier.clone(),
    };
    assert_eq!(protocol_view(package, &carrier).unwrap_err(), expected);
    assert_eq!(causal_relation(package, &carrier).unwrap_err(), expected);
    assert!(
        protocol_view(package, &clause(package).node_id).is_ok(),
        "the clause of the same package reads"
    );
}

fn by_name<'p>(package: &'p CheckedPackageV2, last: &str) -> &'p CheckedSemanticNodeV2 {
    package
        .graph()
        .nodes
        .iter()
        .find(|node| {
            node.declaration
                .as_ref()
                .is_some_and(|declared| declared.qualified_name.last().map(|n| &**n) == Some(last))
        })
        .unwrap_or_else(|| panic!("a node declared {last}"))
}

fn wire_node<'w>(wire: &'w Value, id: &CheckedNodeId) -> &'w Value {
    let id = serde_json::to_value(id).unwrap();
    nodes(wire)
        .iter()
        .find(|node| node["node_id"] == id)
        .expect("a wire node")
}

/// FR-519-AC-5 (TC-1314 row 5), declaration, operation, binder and model:
/// each view returns the member the map names, equal to the value the emitted
/// wire carries, and refuses naming the member once the node lacks it.
#[trace("FR-519-AC-5", "FR-519-AC-8", "TC-1314")]
#[test]
fn the_node_views_return_the_map_successors_and_refuse_a_removed_member() {
    let fixture = fixture();
    let package = &*fixture.admitted;
    let clause_node = clause(package);

    // Declaration with its identity triple.
    let charged = by_name(package, "Charged");
    let (identity, qualified) = declaration(charged).unwrap();
    assert_eq!(identity, &charged.node_id);
    assert_eq!(
        qualified
            .iter()
            .map(|segment| &**segment)
            .collect::<Vec<_>>(),
        [
            "OrderFlow",
            "run",
            "Main",
            "PaymentRetries",
            "ChargeCycle",
            "Charged"
        ]
    );
    let mut stripped = charged.clone();
    stripped.declaration = None;
    assert_eq!(
        declaration(&stripped).unwrap_err(),
        ViewRefusal::MissingMember {
            node: charged.node_id.clone(),
            member: "declaration"
        }
    );

    // The control operation, from the operator the body applies.
    for (declared, operation) in [
        ("Main", ControlOperation::Sequence),
        ("Requested", ControlOperation::Send),
        ("Received", ControlOperation::Receive),
        ("Charged", ControlOperation::Attempt),
        ("Applied", ControlOperation::Effect),
        ("Shipments", ControlOperation::Parallel),
        ("Outcome", ControlOperation::Choice),
        ("Done", ControlOperation::Check),
        ("Poll", ControlOperation::Repeat),
        ("WaitEvent", ControlOperation::Await),
        ("SeenEvent", ControlOperation::Event),
        ("Finalized", ControlOperation::Commit),
        ("Refund", ControlOperation::Compensate),
        ("Closed", ControlOperation::Finish),
        ("Orders", ControlOperation::Role),
        ("Charges", ControlOperation::Channel),
    ] {
        let found = by_name(package, declared);
        assert_eq!(control_operation(found).unwrap(), operation, "{declared}");
        assert_eq!(
            operation_identity(found).unwrap(),
            "quire.op.protocol.control"
        );
    }
    assert_eq!(
        control_operation(clause_node).unwrap(),
        ControlOperation::Protocol
    );
    let mut bodiless = charged.clone();
    bodiless.body = CheckedTerm::reference(charged.node_id.clone());
    assert_eq!(
        control_operation(&bodiless).unwrap_err(),
        ViewRefusal::MissingMember {
            node: charged.node_id.clone(),
            member: "operation"
        }
    );

    // The temporal and value operation identities of the applications.
    let temporal = package
        .graph()
        .nodes
        .iter()
        .find(|node| &*node.semantic_form == "temporal_clause")
        .unwrap();
    assert_eq!(
        operation_identity(temporal).unwrap(),
        "quire.op.temporal.clause"
    );
    assert!(
        package.graph().nodes.iter().any(|node| {
            operation_identity(node).is_ok_and(|identity| identity == "quire.op.boolean.not")
        }),
        "the choice guard's `not` is a value operation node"
    );

    // The binder and its level, against the wire's own binder node.
    let sent = by_name(package, "Requested");
    let found = binder(package, sent).unwrap();
    let wire_binder = wire_node(&fixture.wire, found.node);
    let level = |term: &Value| term["members"][1]["value"]["value"].clone();
    assert_eq!(
        wire_binder["body"]["members"][0]["value"]["value"],
        found.name
    );
    assert_eq!(level(&wire_binder["body"]), json!(found.level.to_string()));
    assert_eq!(found.name, "sent");
    assert_eq!(found.level, 3, "over 0, the trigger 1, the capture 2");
    let mut unbound = sent.clone();
    let CheckedTerm::Application { arguments, .. } = &mut unbound.body else {
        panic!("an application body");
    };
    arguments.retain(
        |argument| !matches!(argument, CheckedTerm::Binding { name, .. } if &**name == "binder"),
    );
    assert_eq!(
        binder(package, &unbound).unwrap_err(),
        ViewRefusal::MissingMember {
            node: sent.node_id.clone(),
            member: "binder"
        }
    );

    // The owner model.
    assert_eq!(
        owner_model(clause_node).unwrap(),
        clause_node.owner_model.as_ref().unwrap()
    );
    assert_eq!(
        serde_json::to_value(owner_model(clause_node).unwrap()).unwrap(),
        wire_node(&fixture.wire, &clause_node.node_id)["owner_model"]
    );
    let mut ownerless = clause_node.clone();
    ownerless.owner_model = None;
    assert_eq!(
        owner_model(&ownerless).unwrap_err(),
        ViewRefusal::MissingMember {
            node: clause_node.node_id.clone(),
            member: "owner_model"
        }
    );
}

/// FR-519-AC-5, channel delivery and queue capacity: the delivery policy word
/// and a declared interval (FR-509), the literal and the symbolic capacity.
#[trace("FR-519-AC-5", "FR-519-AC-8", "TC-1314")]
#[test]
fn the_channel_and_queue_views_return_delivery_and_capacity() {
    let fixture = fixture();
    let package = &*fixture.admitted;
    let charges = by_name(package, "Charges");
    assert_eq!(delivery(charges).unwrap(), Delivery::AtLeastOnce);
    let held = fixture
        .protocol
        .channels
        .iter()
        .find(|channel| channel.name == "Charges")
        .expect("the checked clause holds the channel");
    assert_eq!(
        (held.delivery, &held.capacity),
        (
            qsl_forms::ChannelDelivery::AtLeastOnce,
            &qsl_forms::ChannelCapacity::Literal(4)
        ),
        "the value the checked clause holds in memory"
    );
    let view = protocol_view(package, &clause(package).node_id).unwrap();
    let [queue] = view.queues() else {
        panic!("one queue");
    };
    assert_eq!(
        queue_capacity(node(package, queue)).unwrap(),
        QueueCapacity::Literal(4)
    );

    let mut undelivered = charges.clone();
    let CheckedTerm::Application { arguments, .. } = &mut undelivered.body else {
        panic!("an application body");
    };
    arguments.retain(
        |argument| !matches!(argument, CheckedTerm::Binding { name, .. } if &**name == "delivery"),
    );
    assert_eq!(
        delivery(&undelivered).unwrap_err(),
        ViewRefusal::MissingMember {
            node: charges.node_id.clone(),
            member: "delivery"
        }
    );

    // A declared interval (FR-509) uses canonical decimal bounds.
    let with_text = |text: &str| {
        let mut channel = charges.clone();
        let CheckedTerm::Application { arguments, .. } = &mut channel.body else {
            panic!("an application body");
        };
        for argument in arguments {
            if let CheckedTerm::Binding { name, value } = argument {
                if &**name == "delivery" {
                    if let CheckedTerm::Literal { value, .. } = &mut **value {
                        *value = json!(text);
                    }
                }
            }
        }
        channel
    };
    assert_eq!(
        delivery(&with_text("[0,3]")).unwrap(),
        Delivery::Between { lower: 0, upper: 3 }
    );
    assert_eq!(
        delivery(&with_text("[2,2]")).unwrap(),
        Delivery::Between { lower: 2, upper: 2 }
    );
    assert_eq!(
        delivery(&with_text("[0,18446744073709551615]")).unwrap(),
        Delivery::Between {
            lower: 0,
            upper: u64::MAX
        }
    );
    for bad in [
        "[4,3]",
        "[0, 3]",
        "[ 2 , 2 ]",
        "[00,3]",
        "[+0,3]",
        "[-1,3]",
        "[0.5,3]",
        "[0,18446744073709551616]",
        "[3]",
        "[0,3",
        "0,3",
        "[3",
        "[a,b]",
        "sometimes",
        "[1,2,3]",
    ] {
        assert_eq!(
            delivery(&with_text(bad)).unwrap_err(),
            ViewRefusal::MalformedMember {
                node: charges.node_id.clone(),
                member: "delivery"
            },
            "{bad}"
        );
    }

    for (words, expected, capacity) in [
        (
            "delivery at-least-once capacity symbolic(Bound) overflow block",
            Delivery::AtLeastOnce,
            QueueCapacity::Symbolic("Bound"),
        ),
        (
            "delivery exactly-once-premise capacity 0 overflow block",
            Delivery::ExactlyOncePremise,
            QueueCapacity::Literal(0),
        ),
        (
            "delivery unknown capacity 2 overflow block",
            Delivery::Unknown,
            QueueCapacity::Literal(2),
        ),
        (
            "delivery at-most-once capacity 2 overflow block",
            Delivery::AtMostOnce,
            QueueCapacity::Literal(2),
        ),
    ] {
        let members = format!(
            "role Buyer on M::Order;\nrole Seller on M::Person;\n\
             channel Pay from Buyer to Seller carries Boolean ordering unordered {words};"
        );
        let lowered = super::reachability::lower_with(
            super::protocol_graph::model(),
            super::protocol_graph::PROFILES,
            &super::protocol_graph::flow_over("M::Order", "on origin", &members),
        );
        let package = lowered.ir.as_ref().expect("the package is admitted");
        assert_eq!(
            delivery(by_name(package, "Pay")).unwrap(),
            expected,
            "{words}"
        );
        let view = protocol_view(package, &clause(package).node_id).unwrap();
        let [queue] = view.queues() else {
            panic!("one queue");
        };
        assert_eq!(
            queue_capacity(node(package, queue)).unwrap(),
            capacity,
            "{words}"
        );
    }
}

/// FR-509-AC-4 (TC-1304): emission writes normalized interval spelling and
/// the strict typed reader returns the same checked bounds.
#[trace("FR-509-AC-4", "TC-1304")]
#[test]
fn an_interval_channel_emits_and_reads_back_its_delivery_premise() {
    let source = tc134_source().replace("delivery at-least-once", "delivery [ 0 , 3 ]");
    let fixture = fixture_over(&source);
    let charge = by_name(&fixture.admitted, "Charges");
    assert_eq!(
        delivery(charge).unwrap(),
        Delivery::Between { lower: 0, upper: 3 }
    );
    assert_eq!(
        fixture.protocol.channels[0].delivery,
        qsl_forms::ChannelDelivery::Between { lower: 0, upper: 3 }
    );
    let qsl_charge = by_name(&fixture.qsl_read, "Charges");
    assert_eq!(
        delivery(qsl_charge).unwrap(),
        Delivery::Between { lower: 0, upper: 3 }
    );
}

/// FR-519-AC-5, locus: the package source-map entry of a node; and the clock
/// binding row of a temporal clause.
#[trace("FR-519-AC-5", "FR-519-AC-8", "TC-1314")]
#[test]
fn the_locus_and_the_clock_binding_are_the_packages_entries() {
    let fixture = fixture();
    let package = &*fixture.admitted;
    let clause_id = &clause(package).node_id;
    let entries = locus(package.source_map(), clause_id).unwrap();
    assert_eq!(entries.len(), 1, "the clause's declaration occurrence");
    let wire_entry: Vec<&Value> = fixture.wire["source_map"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["node_id"] == serde_json::to_value(clause_id).unwrap())
        .collect();
    let [wire_entry] = wire_entry[..] else {
        panic!("one wire entry");
    };
    assert_eq!(
        serde_json::to_value(entries[0]).unwrap(),
        *wire_entry,
        "the view returns the wire's entry"
    );
    assert_eq!(
        locus(&[], clause_id).unwrap_err(),
        ViewRefusal::MissingMember {
            node: clause_id.clone(),
            member: "source_map"
        }
    );

    let temporal = package
        .graph()
        .nodes
        .iter()
        .find(|node| &*node.semantic_form == "temporal_clause")
        .expect("a temporal clause");
    let rows = package
        .temporal_bindings()
        .expect("the timed package carries bindings");
    let row = temporal_binding(rows, &temporal.node_id).unwrap();
    assert_eq!(row.clock, tc134_clock());
    assert_eq!(&*row.clock_name, "model-steps");
    assert_eq!(
        temporal_binding(&[], &temporal.node_id).unwrap_err(),
        ViewRefusal::MissingMember {
            node: temporal.node_id.clone(),
            member: "temporal_bindings"
        },
        "the row removed"
    );
    assert_eq!(
        temporal_binding(rows, clause_id).unwrap_err(),
        ViewRefusal::MissingMember {
            node: clause_id.clone(),
            member: "temporal_bindings"
        }
    );
}

/// FR-519-AC-5, lock members: the locked source, the profile selection and
/// the dependency selection each equal the wire's lock row, and each is refused
/// by name once absent.
#[trace("FR-519-AC-5", "FR-519-AC-8", "TC-1314")]
#[test]
fn the_lock_views_return_the_selections_and_refuse_an_absent_one() {
    use quire_contract_model::CheckedSelectionRole as Role;
    let fixture = fixture();
    let lock = fixture.admitted.lock();
    let wire_lock = &fixture.wire["lock"];

    let wire_source = &wire_lock["sources"][0];
    let identity = wire_source["identity"].as_str().unwrap();
    assert_eq!(
        serde_json::to_value(source(lock, identity).unwrap()).unwrap(),
        *wire_source
    );
    assert_eq!(
        source(lock, "no/such/source").unwrap_err(),
        ViewRefusal::MissingSelection {
            member: "sources",
            name: "no/such/source".into()
        }
    );

    for (role, wire_role) in [
        (Role::ProtocolProfile, "protocol_profile"),
        (Role::TemporalProfile, "temporal_profile"),
    ] {
        let expected = wire_lock["profile_selections"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["role"] == wire_role)
            .expect("the wire selects the profile");
        assert_eq!(
            serde_json::to_value(selection(lock, role).unwrap()).unwrap(),
            expected["definition"]
        );
    }
    let mut without = lock.clone();
    without.profile_selections.clear();
    assert_eq!(
        selection(&without, Role::TemporalProfile).unwrap_err(),
        ViewRefusal::MissingSelection {
            member: "profile_selections",
            name: "TemporalProfile".into()
        }
    );

    // A package that imports a library carries its dependency selection.
    let middle = linked(
        "test/b",
        vec![import("test/units", super::dependency("test/units"))],
    );
    let root = CheckedPackage::link_with(root_graph(), vec![import("test/b", middle)])
        .expect("the root links");
    let emission = emit(&root);
    let mut evidence = read_evidence(&emission);
    crate::checked_v2::supply_closure(
        &root,
        &std::collections::BTreeMap::new(),
        crate::V2ReadLimits::default(),
        &mut evidence,
    )
    .expect("each dependency reads back");
    let Read::Verified { admitted, .. } = read_with(&emission, &evidence) else {
        panic!("the root reads back");
    };
    let wire_dependency = &wire(&emission)["lock"]["dependency_selections"][0];
    assert_eq!(wire_dependency["identity"], "test/b");
    assert_eq!(
        serde_json::to_value(dependency(admitted.lock(), "test/b").unwrap()).unwrap(),
        *wire_dependency
    );
    assert_eq!(
        dependency(admitted.lock(), "test/units").unwrap_err(),
        ViewRefusal::MissingSelection {
            member: "dependency_selections",
            name: "test/units".into()
        },
        "only the direct dependency is selected"
    );
}

/// FR-519-AC-6 (TC-1314 row 6): the resolved dependency graph of `qsl-package`
/// (`Cargo.lock`) holds no `quire-protocol`, and the view module names no
/// byte reader.
#[trace("FR-519-AC-6", "TC-1314")]
#[test]
fn the_dependency_graph_holds_no_quire_protocol_and_the_views_read_no_bytes() {
    use std::collections::BTreeMap;
    let lock = include_str!("../../../../Cargo.lock");
    let mut edges: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut current = "";
    let mut listing = false;
    for line in lock.lines() {
        if let Some(name) = line.strip_prefix("name = \"") {
            current = name.trim_end_matches('"');
            edges.entry(current).or_default();
        } else if line.starts_with("dependencies = [") {
            listing = true;
        } else if listing && line.trim() == "]" {
            listing = false;
        } else if listing {
            let entry = line.trim().trim_matches(|c| c == '"' || c == ',');
            edges
                .entry(current)
                .or_default()
                .push(entry.split(' ').next().unwrap_or(entry));
        }
    }
    assert!(
        edges.contains_key("qsl-package"),
        "the lock lists qsl-package"
    );
    let mut seen = BTreeSet::new();
    let mut pending = vec!["qsl-package"];
    while let Some(package) = pending.pop() {
        if seen.insert(package) {
            pending.extend(edges.get(package).into_iter().flatten().copied());
        }
    }
    assert!(seen.len() > 10, "the closure is read: {}", seen.len());
    assert!(!seen.contains("quire-protocol"), "{seen:?}");

    let text = include_str!("../../protocol_view.rs");
    for reader in [
        "read_checked_package",
        "from_slice",
        "from_str",
        "quire_canonical",
        "CheckedPackageV2::read",
    ] {
        assert!(!text.contains(reader), "protocol_view.rs names {reader}");
    }
}

/// FR-519-AC-10 (TC-1314): `protocol_view`, `causal_relation` and `binder` each
/// take `&CheckedPackageV2`, and the node references they return are
/// `CheckedNodeId`s.
#[trace("FR-519-AC-10", "TC-1314")]
#[test]
fn the_package_views_take_the_admitted_package_and_return_node_ids() {
    let fixture = fixture();
    let package: &CheckedPackageV2 = &fixture.admitted;
    let clause_id: &CheckedNodeId = &clause(package).node_id;
    let view = protocol_view(package, clause_id).unwrap();
    let _: &CheckedNodeId = view.clause();
    let _: Vec<&CheckedNodeId> = view.controls().to_vec();
    let relation = causal_relation(package, clause_id).unwrap();
    let _: &CheckedNodeId = &relation.edges()[0].from.node;
    let requested = by_name(package, "Requested");
    let found = binder(package, requested).unwrap();
    let _: &CheckedNodeId = found.node;
    assert_eq!(found.name, "sent");
}

/// FR-519-AC-11 (TC-1314 row 11), over the protocol_view fixture (the TC-134
/// workflow plus an invariant over the signed-64 extrema): the `semantic_type` nodes and the literal
/// and reference terms that stand for the retired `Type`, `Value` and `Number`
/// records are read through `CheckedPackageV2::term` and equal the values the
/// emitter built in memory, with each number in the FR-322 encoding (a decimal
/// text, never a JSON number). The workflow gains an invariant over the
/// signed-64 extrema so that the numbers reach the extremes of the encoding.
#[trace("FR-519-AC-11", "TC-1314")]
#[test]
fn types_values_and_numbers_are_the_terms_the_emitter_built() {
    let text = format!(
        "{}\ninvariant Wide using v on M::OrderView at current {{ \
         self.order <= 9223372036854775807 and self.order >= -9223372036854775808 }}\n",
        source_text()
    );
    let fixture = fixture_over(&text);
    let package = &*fixture.admitted;

    let mut types = 0;
    let mut literals = Vec::new();
    let mut negated = Vec::new();
    for emitted in nodes(&fixture.wire) {
        let id: CheckedNodeId = serde_json::from_value(emitted["node_id"].clone()).unwrap();
        let read = node(package, &id);
        let form = emitted["semantic_form"].as_str().unwrap();
        if matches!(form, "boolean" | "integer" | "text" | "reference") {
            types += 1;
            assert_eq!(&*read.semantic_form, form);
            assert_eq!(serde_json::to_value(read).unwrap(), *emitted, "{form}");
        }
        let Some(term) = package.term(&id) else {
            continue;
        };
        assert_eq!(
            serde_json::to_value(term).unwrap(),
            emitted["body"],
            "the term of {form}"
        );
        match term {
            CheckedTerm::Literal {
                value_kind: LiteralKind::Integer,
                value,
                ..
            } => {
                literals.push(
                    value
                        .as_str()
                        .expect("an integer is a decimal text")
                        .to_owned(),
                );
            }
            CheckedTerm::Application { arguments, .. }
                if emitted["body"]["operation"]["identity"] == "quire.op.integer.negate" =>
            {
                negated.push(serde_json::to_value(&arguments[0]).unwrap()["value"].clone());
            }
            _ => {}
        }
    }
    assert!(
        types >= 3,
        "boolean, integer and text scalar types: {types}"
    );
    literals.sort();
    assert_eq!(
        literals,
        ["9223372036854775807"],
        "the maximum is a literal decimal text"
    );
    assert_eq!(
        negated,
        ["9223372036854775808"],
        "the signed minimum is the negation of its magnitude"
    );
}

/// FR-519-AC-12 (TC-1314 row 12): the population declaration node of the
/// model, selected from the admitted package by its node id, equals the node
/// the emitter built in memory, with its `max` bound in the FR-322 encoding.
/// R of FR-517 declares no population, so, as the merged FR-519-AC-12 states, this
/// runs over TC-1054's model (TC-1046's fixture with `orders` and `people`).
#[trace("FR-519-AC-12", "TC-1314")]
#[test]
fn the_population_declaration_node_is_selected_by_its_id() {
    let lowered = super::reachability::lower_over(
        super::population_bound::document(),
        &format!(
            "bound OrdersBound on M::orders [3]\n{}",
            super::reachability::NO_LOOP
        ),
    );
    let package = lowered.ir.as_ref().expect("IR admits the package");
    let populations: Vec<&Value> = nodes(&lowered.wire)
        .iter()
        .filter(|node| node["semantic_form"] == "model_population")
        .collect();
    let [emitted] = populations.as_slice() else {
        panic!("one population node");
    };
    let id: CheckedNodeId = serde_json::from_value(emitted["node_id"].clone()).unwrap();
    let read = node(package, &id);
    assert_eq!(&*read.semantic_form, "model_population");
    assert_eq!(serde_json::to_value(read).unwrap(), **emitted);
    let term = package.term(&id).expect("the node has a body term");
    assert_eq!(serde_json::to_value(term).unwrap(), emitted["body"]);
    let maximum = emitted["body"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["name"] == "max")
        .expect("the bound's max");
    assert_eq!(maximum["value"]["value"], "3");
}
