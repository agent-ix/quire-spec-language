// SPDX-License-Identifier: AGPL-3.0-only
//! TC-121: the split-shipment choreography of #39 through the actual parse,
//! link, type, proof, admission, emission and public reader stages.
//!
//! Order O1 sends one shipment notice over the fulfilment channel and provider P
//! observes it as the two shipments S1 and S2. Two shipments are two transport
//! deliveries of the one message, never two business effects: the authored
//! attempt A1 and its single effect E1 stay one apiece while the delivery
//! observations stay two. Nothing here observes a shipment or a payment; every
//! emitted record is a static requirement for a later binding.

#[path = "support/native_protocol/mod.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_spec_language::checking::composed::{proofs, TypeDisposition, TypeLimits};
use quire_spec_language::linking::composed::scopes::{
    DeclarationScope, ScopeIssue, StructuralKind,
};
use quire_spec_language::protocol_artifact::{
    self as artifact, native, wire as w, Error, Limits, Unsupported,
};
use setup::{Inputs, Unit};

/// The forward payment attempt and the one declared business effect of it.
const PAYMENT: &str = "sequence Payment {
    attempt A1 by Provider on M::Node::step contracts [] as (attemptA1: M::Plain) { true };
    effect E1 of Payment::A1 as (effectE1: M::Plain) { true };
}";

/// One authored notice observed as the two shipments S1 and S2.
const SPLIT: &str = "send Notice via Shipments as (notice: M::Plain) { true };
    receive S1 via Shipments of Notice as (shipmentS1: M::Plain) { shipmentS1.ready };
    receive S2 via Shipments of Notice as (shipmentS2: M::Plain) { not shipmentS2.ready };";

fn inputs(run: &str) -> Inputs {
    let body = format!(
        "protocol Fulfilment using P over (view: M::Node) on origin {{
          role Merchant on M::Node;
          role Provider on M::Node;
          channel Shipments from Merchant to Provider carries M::Plain
            ordering unordered delivery [1,2];
          run sequence O1 {{ {run} {PAYMENT} }}
          finish Closed as (closed: M::Node) {{ true }};
        }}"
    );
    Inputs::new(&[Unit {
        name: "split-shipments",
        body: &body,
        declarations: &["Fulfilment"],
    }])
}

fn discharged(report: &proofs::ProofReport<'_, '_, '_>) {
    assert!(report.exhaustion().is_none(), "{:?}", report.exhaustion());
    for declaration in report.declarations() {
        let id = declaration.declaration();
        assert_eq!(
            report.types().disposition(id),
            Some(TypeDisposition::Typed),
            "type causes {:?}; scope issues {:?}",
            report.types().declaration(id).map(|typed| typed.causes()),
            report
                .types()
                .binding()
                .scopes()
                .and_then(|scopes| scopes.declaration(id))
                .map(|scope| &scope.issues)
        );
        assert_eq!(
            declaration.disposition(),
            proofs::ProofDisposition::Discharged,
            "{:?}",
            declaration.causes()
        );
    }
}

/// Admit through the real family stage, emit, and require the independently
/// reconstructed reader package to equal the admitted one.
fn admitted(inputs: &Inputs, inspect: impl FnOnce(&w::Package)) {
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let report = native::admit(proofs, selected, Limits::default());
            assert!(
                report.result().is_ok(),
                "{:?}; locus {:?}",
                report.result().err(),
                report.locus()
            );
            let admitted = report.into_result().unwrap();
            let package = admitted.package();
            inspect(package);
            let emitted = native::emit(&admitted, Limits::default())
                .into_result()
                .unwrap();
            let read = inputs.read(proofs, &emitted);
            assert!(
                read.result().is_ok(),
                "{:?}; locus {:?}",
                read.result().err(),
                read.locus()
            );
            assert_eq!(read.into_result().unwrap().package(), package);
        },
    );
}

/// The exact scope refusal of a source whose delivery identity does not resolve,
/// and the family refusal that follows it.
#[track_caller]
fn scope_refused(inputs: &Inputs, case: &str, inspect: impl FnOnce(&DeclarationScope)) {
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            let binding = proofs.types().binding();
            let [id] = binding.namespace().lookup("Fulfilment") else {
                panic!("{case}: one original protocol declaration")
            };
            let scope = binding.scopes().unwrap().declaration(*id).unwrap();
            inspect(scope);
            assert_eq!(
                proofs.types().disposition(*id),
                Some(TypeDisposition::Refused),
                "{case}"
            );
            let report = native::admit(proofs, selected, Limits::default());
            assert_eq!(
                report.result().err(),
                Some(&Error::Unsupported(Unsupported::FamilyProof)),
                "{case}; locus {:?}",
                report.locus()
            );
        },
    );
}

struct Protocol<'a> {
    controls: &'a [w::Control],
    channels: &'a [w::Channel],
    roles: &'a [w::Role],
}

fn protocol(package: &w::Package) -> Protocol<'_> {
    let w::Body::Protocol {
        controls,
        channels,
        roles,
        ..
    } = &package.declarations[0].body
    else {
        panic!("authored protocol declaration")
    };
    Protocol {
        controls,
        channels,
        roles,
    }
}

/// The authored control of that name, with the handle other records use.
#[track_caller]
fn control(controls: &[w::Control], name: &str) -> w::Handle {
    let index = controls
        .iter()
        .position(|control| control.name == name)
        .unwrap_or_else(|| panic!("authored control {name}"));
    w::Handle {
        declaration: 0,
        index: index as u32,
    }
}

/// The declaration-local subject of a control-owned binding requirement.
fn subject(control: &w::Handle) -> w::Subject {
    w::Subject::Control {
        control: control.clone(),
    }
}

fn integer_value(value: &w::Integer) -> i64 {
    let artifact::ProtocolNumber::Integer(value) = value.checked().unwrap() else {
        panic!("authored integer bound")
    };
    value.value()
}

/// Binding requirements of one kind, in their emitted table order.
fn requirements(
    declaration: &w::Declaration,
    kind: w::BindingKind,
) -> Vec<(u32, &w::BindingRequirement)> {
    declaration
        .bindings
        .iter()
        .enumerate()
        .filter(|(_, binding)| binding.kind == kind)
        .map(|(index, binding)| (index as u32, binding))
        .collect()
}

fn control_requirements(
    declaration: &w::Declaration,
    kind: w::BindingKind,
) -> Vec<(u32, &w::BindingRequirement)> {
    requirements(declaration, kind)
        .into_iter()
        .filter(|(_, binding)| matches!(binding.subject, w::Subject::Control { .. }))
        .collect()
}

/// Item 1 of the #39 sentence. One notice delivered twice under one business
/// effect keeps the cardinalities 1/2/1: the two shipments are two separate
/// transport delivery requirements of the same send over the same channel, and
/// neither of them is the channel's per-send delivery bound or the effect.
#[test]
#[trace("TC-121", "FR-042-AC-1", "FR-042-AC-6", "FR-042-AC-7")]
fn two_shipments_of_one_notice_keep_distinct_transport_delivery_identities() {
    let inputs = inputs(SPLIT);
    admitted(&inputs, |package| {
        let declaration = &package.declarations[0];
        let protocol = protocol(package);

        // The authored per-send delivery bound survives as the channel's own
        // interval, and provider P is the channel recipient of both shipments.
        let [channel] = protocol.channels else {
            panic!("one authored fulfilment channel")
        };
        assert_eq!(
            (
                integer_value(&channel.delivery.lower),
                integer_value(&channel.delivery.upper)
            ),
            (1, 2),
            "the authored per-send delivery bound admits the split shipment"
        );
        assert_eq!(
            protocol.roles[channel.to.index as usize].name, "Provider",
            "provider P is the recipient of both shipments"
        );

        // One send, two deliveries, one effect: 1/2/1.
        let notice = control(protocol.controls, "Notice");
        let shipments = [
            control(protocol.controls, "S1"),
            control(protocol.controls, "S2"),
        ];
        let effect = control(protocol.controls, "E1");
        let sends = control_requirements(declaration, w::BindingKind::Send);
        let receives = control_requirements(declaration, w::BindingKind::Receive);
        let effects = control_requirements(declaration, w::BindingKind::Effect);
        assert_eq!((sends.len(), receives.len(), effects.len()), (1, 2, 1));
        assert_eq!(sends[0].1.subject, subject(&notice));
        assert_eq!(effects[0].1.subject, subject(&effect));

        // Each shipment is its own delivery identity: its own requirement, its
        // own subject, its own binder and its own originating anchor. Both
        // observe the one notice over the one channel.
        let channel_handle = w::Handle {
            declaration: 0,
            index: 0,
        };
        let mut anchors = Vec::new();
        let mut binders = Vec::new();
        for (at, shipment) in shipments.iter().enumerate() {
            let (_, requirement) = receives
                .iter()
                .find(|(_, binding)| binding.subject == subject(shipment))
                .unwrap_or_else(|| panic!("delivery requirement of shipment {at}"));
            assert!(
                requirement.requires.contains(&channel.receive),
                "shipment {at} carries the channel's own receive authority"
            );
            assert!(requirement.requires.contains(&sends[0].0));
            anchors.push(requirement.anchor.clone());

            let w::ControlOperation::Event {
                event: w::Event::Receive { send, channel: via },
                binder,
                ..
            } = &protocol.controls[shipment.index as usize].operation
            else {
                panic!("shipment {at} is an authored delivery observation")
            };
            assert_eq!(send, &notice, "both shipments deliver the one notice");
            assert_eq!(via, &channel_handle);
            binders.push(declaration.binders[binder.index as usize].name.clone());
        }
        assert_ne!(anchors[0], anchors[1], "distinct delivery anchors");
        assert_eq!(binders, ["shipmentS1", "shipmentS2"]);
        assert_eq!(
            receives[0].1.anchor, anchors[0],
            "requirement order follows the authored shipment order"
        );

        // The channel's per-send delivery requirement is one record, distinct
        // from both shipment observations and from the business effect.
        let delivery = requirements(declaration, w::BindingKind::Delivery);
        assert_eq!(delivery.len(), 1, "one per-send delivery requirement");
        assert_eq!(delivery[0].0, channel.delivery_instance);
        assert_eq!(
            delivery[0].1.subject,
            w::Subject::Channel {
                channel: channel_handle
            },
            "the delivery bound is owned by the channel, not by a shipment"
        );
        let separate = [delivery[0].0, receives[0].0, receives[1].0, effects[0].0];
        let mut unique = separate.to_vec();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(
            unique.len(),
            separate.len(),
            "delivery, each shipment and the effect are separate requirements"
        );
        assert!(
            !effects[0].1.requires.contains(&delivery[0].0)
                && !effects[0].1.requires.contains(&receives[0].0)
                && !effects[0].1.requires.contains(&receives[1].0),
            "a delivery never becomes the business effect"
        );
    });
}

/// Item 1, the per-shipment half. Each shipment keeps its own authored
/// constraint over its own binder: the second shipment's `not` is not the
/// first's, and neither constraint reads the other shipment's binder.
#[test]
#[trace("TC-121", "FR-042-AC-4", "FR-042-AC-7")]
fn each_shipment_keeps_its_own_authored_constraint_over_its_own_binder() {
    let inputs = inputs(SPLIT);
    admitted(&inputs, |package| {
        let declaration = &package.declarations[0];
        let protocol = protocol(package);
        let mut constraints = Vec::new();
        for name in ["S1", "S2"] {
            let handle = control(protocol.controls, name);
            let w::ControlOperation::Event {
                binder, constraint, ..
            } = &protocol.controls[handle.index as usize].operation
            else {
                panic!("{name} is an authored delivery observation")
            };
            constraints.push((binder.clone(), constraint.clone()));
        }
        assert_ne!(constraints[0].1, constraints[1].1);

        // S1 reads its own binder directly; S2 negates its own.
        let field = |value: &w::Handle| -> (&str, &w::ExportRef) {
            let w::ValueOperation::Field { base, field } =
                &declaration.values[value.index as usize].operation
            else {
                panic!("authored Boolean field read")
            };
            let w::ValueOperation::Read { binder } =
                &declaration.values[base.index as usize].operation
            else {
                panic!("authored field read of an original binder")
            };
            (
                declaration.binders[binder.index as usize].name.as_str(),
                field,
            )
        };
        let (first, first_field) = field(&constraints[0].1);
        let w::ValueOperation::Unary {
            operator: w::Unary::Not,
            value,
        } = &declaration.values[constraints[1].1.index as usize].operation
        else {
            panic!("S2 negates its own authored observation")
        };
        let (second, second_field) = field(value);
        assert_eq!(first, "shipmentS1");
        assert_eq!(second, "shipmentS2");
        assert_eq!(
            first_field, second_field,
            "both read the same admitted Boolean export of the payload"
        );
        assert_eq!(
            package.models[first_field.model as usize].exports[first_field.export as usize].path,
            ["Plain", "ready"]
        );
    });
}

/// Item 1, the adverse half. A second shipment may neither collapse onto the
/// first delivery identity by reusing its name, nor substitute another delivery
/// for the send it observes.
#[test]
#[trace("TC-121", "FR-036-AC-4", "FR-042-AC-6", "FR-042-AC-8")]
fn a_collapsed_or_substituted_shipment_delivery_identity_is_refused() {
    let collapsed = SPLIT.replace("receive S2 via Shipments", "receive S1 via Shipments");
    scope_refused(
        &inputs(&collapsed),
        "two shipments cannot share one identity",
        |scope| {
            assert!(
                scope
                    .issues
                    .iter()
                    .any(|issue| matches!(issue, ScopeIssue::DuplicateSymbol { .. })),
                "{:?}",
                scope.issues
            );
        },
    );

    let substituted = SPLIT.replace(
        "receive S2 via Shipments of Notice",
        "receive S2 via Shipments of O1::S1",
    );
    scope_refused(
        &inputs(&substituted),
        "a delivery cannot stand in for the send it observes",
        |scope| {
            let target = scope
                .issues
                .iter()
                .find_map(|issue| match issue {
                    ScopeIssue::WrongTargetKind { reference, target }
                        if scope.references[*reference].required == StructuralKind::Send =>
                    {
                        Some(*target)
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{:?}", scope.issues));
            assert_eq!(
                scope.symbols[target.index()].name.value,
                "S1",
                "the substituted target is the sibling shipment, not the notice"
            );
        },
    );
}
