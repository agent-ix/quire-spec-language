// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-572 (TC-1412): `ProtocolPremises::from_view` over a real checked
//! package, read only through `qsl_package::protocol_view` (ADR-036 PA-3,
//! PA-6).

use super::frame_replay::unit_for;
use super::*;
use crate::protocol_analysis::{
    choice_premises, pair_channels, CapacityPremise, DeliveryPremise, OrderingPremise,
    OverflowPremise, PremiseRefusal, ProtocolPremises,
};
use qsl_package::protocol_view::protocol_view;
use quire_contract_model::{
    CheckedNodeId, CheckedPackageDispatchResult, CheckedPackageReadLimits, CheckedPackageV2,
};

fn text() -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         profile p = \"quire.protocol.finite-global/v1\";\n\
         model Config = {CONFIG_VERSION_PACKAGE_IDENTITY:?};\n\
         protocol Flow using p over (input: Config::ConfigVersion) on origin {{\n\
           role Buyer on Config::ConfigVersion;\n\
           role Seller on Config::ConfigVersion;\n\
           channel Charges from Buyer to Seller carries Boolean\n\
             ordering fifo by (m: Boolean) {{ m }}\n\
             delivery at-least-once capacity 4 overflow reject;\n\
           channel Notes from Seller to Buyer carries Boolean ordering unordered\n\
             delivery unknown capacity 2 overflow block;\n\
           run sequence Main {{\n\
             choice Outcome by Buyer visible () {{\n\
               case a when {{ true }} sequence A {{ }}\n\
               case z when {{ false }} sequence Z {{ }}\n\
             }}\n\
           }}\n\
           finish End as (outcome: Boolean) {{ true }};\n\
         }}\n"
    )
}

fn admitted() -> Box<CheckedPackageV2> {
    let unit = unit_for(text(), config_version_domain_document());
    let emission =
        qsl_package::emit_checked(&unit.compiled.package, qsl_package::EmitLimits::fixture())
            .expect("the protocol package emits");
    let mut evidence = emission.evidence().clone();
    evidence
        .insert_domain_package_document(
            CONFIG_VERSION_PACKAGE_IDENTITY,
            config_version_domain_document().as_slice(),
        )
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

fn named<'p>(
    package: &'p CheckedPackageV2,
    last: &str,
) -> &'p quire_contract_model::CheckedSemanticNodeV2 {
    package
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| {
            node.declaration.as_ref().is_some_and(|declaration| {
                declaration.qualified_name.last().map(|s| &**s) == Some(last)
            })
        })
        .unwrap_or_else(|| panic!("a node named {last}"))
}

/// Groups (a) and (b) are the package's channels, their queues and its
/// choice, as typed node ids ordered by node id; groups (c) and (d) are not
/// stated.
#[trace("FR-572-AC-3", "TC-1412")]
#[test]
fn premises_are_built_from_the_protocol_view() {
    let package = admitted();
    let premises = ProtocolPremises::from_view(&package, &clause(&package)).unwrap();
    assert_eq!(premises.role_knowledge, None);
    assert_eq!(premises.environment, None);
    let [first, second] = &premises.channels[..] else {
        panic!("two channels: {:?}", premises.channels);
    };
    assert!(first.channel < second.channel);
    let charges = premises
        .channels
        .iter()
        .find(|premise| premise.channel == named(&package, "Charges").node_id)
        .expect("Charges");
    assert!(matches!(charges.ordering, OrderingPremise::Fifo { .. }));
    assert_eq!(charges.delivery, DeliveryPremise::AtLeastOnce);
    assert_eq!(charges.capacity, CapacityPremise::Literal { value: 4 });
    assert_eq!(charges.overflow, OverflowPremise::Reject);
    assert_eq!(charges.sender, named(&package, "Buyer").node_id);
    assert_eq!(charges.receiver, named(&package, "Seller").node_id);
    let notes = premises
        .channels
        .iter()
        .find(|premise| premise.channel == named(&package, "Notes").node_id)
        .expect("Notes");
    assert_eq!(notes.ordering, OrderingPremise::Unordered);
    assert_eq!(notes.overflow, OverflowPremise::Block);
    let [outcome] = &premises.choices[..] else {
        panic!("one choice: {:?}", premises.choices);
    };
    assert_eq!(outcome.choice, named(&package, "Outcome").node_id);
    assert_eq!(outcome.owner, named(&package, "Buyer").node_id);
    assert!(outcome.visible.is_empty());
}

/// A queue naming no channel of the clause, a channel with no queue, and a
/// control whose operation refuses are each refused, never skipped.
#[trace("TC-1412")]
#[test]
fn premises_refuse_an_orphan_queue_a_queueless_channel_and_an_unreadable_control() {
    let package = admitted();
    let view = protocol_view(&package, &clause(&package)).unwrap();
    let node = |id: &CheckedNodeId| {
        package
            .resolved_graph()
            .nodes
            .iter()
            .find(|node| &node.node_id == id)
            .expect("a node of the package")
    };
    let channels: Vec<_> = view.channels().iter().map(|id| node(id)).collect();
    let queues: Vec<_> = view.queues().iter().map(|id| node(id)).collect();
    assert_eq!(pair_channels(&channels, &queues).unwrap().len(), 2);

    let orphan = pair_channels(&channels[..1], &queues).unwrap_err();
    assert!(
        matches!(orphan, PremiseRefusal::OrphanQueue { .. }),
        "{orphan:?}"
    );
    let queueless = pair_channels(&channels, &queues[..1]).unwrap_err();
    assert!(
        matches!(queueless, PremiseRefusal::ChannelWithoutQueue { .. }),
        "{queueless:?}"
    );

    // A node whose operation does not read is refused by the premises
    // instead of being dropped from group (b).
    let unreadable = package
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| qsl_package::protocol_view::control_operation(node).is_err())
        .expect("the package holds a node that is no profile operator application");
    let refused = choice_premises(&[unreadable]).unwrap_err();
    assert!(matches!(refused, PremiseRefusal::View(_)), "{refused:?}");
    let controls: Vec<_> = view.controls().iter().map(|id| node(id)).collect();
    assert_eq!(choice_premises(&controls).unwrap().len(), 1);
}
