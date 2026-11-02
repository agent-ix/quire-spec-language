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
    binder, causal_relation, control_operation, declaration, definition, delivery, dependency,
    locus, operation_identity, owner_model, protocol_view, queue_capacity, source,
    temporal_binding, ControlOperation, Delivery, QueueCapacity, Scheduling, ViewRefusal,
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

/// FR-519-AC-1, AC-2, FR-383-AC-16 (TC-1314 rows 1 and 2; this is the
/// FR-383-AC-16 check and it calls `protocol_view::causal_relation`): the relation derived from the
/// emitted nodes holds the (source port, target port) pairs of the checked
/// clause's compiled edge relation, every edge rule present, the `repeat` back
/// edge included, strictly ascending and duplicate-free; the package read by IR
/// and the package read by QSL's I2 reader derive the same relation.
#[trace("FR-519-AC-1", "FR-519-AC-2", "FR-383-AC-16", "TC-1314")]
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
        [
            prefix("compensate", "PartialRefund"),
            prefix("compensate", "Refund")
        ]
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
    let in_process: BTreeSet<Vec<String>> = protocol
        .controls
        .controls
        .iter()
        .filter(|control| {
            !matches!(
                control.kind,
                qsl_forms::ProtocolNodeKind::Branch | qsl_forms::ProtocolNodeKind::Case
            )
        })
        .map(|control| emitted_name(protocol, control))
        .collect();
    assert_eq!(
        names(view.controls()).into_iter().collect::<BTreeSet<_>>(),
        in_process,
        "the controls are the checked control tree's"
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
    assert_eq!(structural(&row.clause), expected.clause);
    assert_eq!(
        structural(&row.required_temporal),
        expected.required_temporal
    );
    assert!(matches!(
        (&row.activation, &expected.activation),
        (
            quire_contract_model::CheckedProtocolActivation::Each { .. },
            quire_contract_model::CheckedProtocolActivation::Each { .. }
        )
    ));
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
         protocol Flow using v over (input: M::Order) on origin {\n\
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
#[trace("FR-519-AC-5", "TC-1314")]
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
#[trace("FR-519-AC-5", "TC-1314")]
#[test]
fn the_channel_and_queue_views_return_delivery_and_capacity() {
    let fixture = fixture();
    let package = &*fixture.admitted;
    let charges = by_name(package, "Charges");
    assert_eq!(delivery(charges).unwrap(), Delivery::AtLeastOnce);
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

    // A declared interval (FR-509) is the delivery text `[lower,upper]`; the
    // source syntax is not yet accepted, so the node carries the text here.
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
        delivery(&with_text("[ 2 , 2 ]")).unwrap(),
        Delivery::Between { lower: 2, upper: 2 }
    );
    for bad in ["[3", "[a,b]", "sometimes", "[1,2,3]"] {
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

/// FR-519-AC-5, locus: the package source-map entry of a node; and the clock
/// binding row of a temporal clause.
#[trace("FR-519-AC-5", "TC-1314")]
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
    let row = temporal_binding(package, &temporal.node_id).unwrap();
    assert_eq!(row.clock, tc134_clock());
    assert_eq!(&*row.clock_name, "model-steps");
    assert_eq!(
        temporal_binding(package, clause_id).unwrap_err(),
        ViewRefusal::MissingMember {
            node: clause_id.clone(),
            member: "temporal_bindings"
        }
    );
}

/// FR-519-AC-5, lock members: the locked source, the selected definition and
/// the dependency selection, each equal to the wire's lock row, each refused
/// by name once absent.
#[trace("FR-519-AC-5", "TC-1314")]
#[test]
fn the_lock_views_return_the_selections_and_refuse_an_absent_one() {
    let fixture = fixture();
    let lock = fixture.admitted.lock();
    let locked = &lock.sources[0];
    assert_eq!(source(lock, locked).unwrap(), locked);
    let selected = &lock.profile_selections[0].definition;
    assert_eq!(
        definition(lock, selected).unwrap_err(),
        ViewRefusal::MissingSelection {
            member: "definition_selections",
            name: selected.identity.to_string(),
        },
        "a profile selection is not a definition selection"
    );
    let mut with_definition = lock.clone();
    with_definition.definition_selections.push(selected.clone());
    assert_eq!(definition(&with_definition, selected).unwrap(), selected);
    assert_eq!(
        serde_json::to_value(&lock.sources).unwrap(),
        fixture.wire["lock"]["sources"],
        "the locked sources are the wire's"
    );

    let mut without = lock.clone();
    without.sources.clear();
    assert_eq!(
        source(&without, locked).unwrap_err(),
        ViewRefusal::MissingSelection {
            member: "sources",
            name: locked.identity.to_string()
        }
    );
    assert_eq!(
        dependency(lock, "acme/lib").unwrap_err(),
        ViewRefusal::MissingSelection {
            member: "dependency_selections",
            name: "acme/lib".into()
        }
    );
    let mut with_dependency = lock.clone();
    with_dependency
        .dependency_selections
        .push(quire_contract_model::CheckedDependencySelection {
            identity: "acme/lib".into(),
        });
    assert_eq!(
        &*dependency(&with_dependency, "acme/lib").unwrap().identity,
        "acme/lib"
    );
}

/// FR-519-AC-6 (TC-1314 row 6): no function of the view module reads package
/// bytes; the module names no byte reader, canonical form or JSON parser.
#[trace("FR-519-AC-6", "TC-1314")]
#[test]
fn the_view_module_reads_no_package_bytes() {
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
    let manifest = include_str!("../../../Cargo.toml");
    assert!(!manifest.contains("quire-protocol"));
}
