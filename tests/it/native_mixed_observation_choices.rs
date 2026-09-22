// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-042: one choice observing an actual received record and the chooser's own
//! attempt record together. These are static admission facts about retained
//! graphs and emitted bytes, not runtime decision outcomes.

use crate::support::native_protocol as setup;

use ix_trace_rs::trace;
use quire_spec_language::checking::composed::{proofs, TypeDisposition, TypeLimits};
use quire_spec_language::protocol_artifact::{native, wire as w, Error, Limits, Unsupported};
use setup::{Inputs, Unit};

/// Both observation kinds carry the same `M::Plain` record type and the same
/// `ready` field; only their original binders and anchors separate the facts.
const MIXED_RUN: &str = "sequence Main {
    send Sent via Messages as (sent: M::Plain) { true };
    receive Got via Messages of Sent as (got: M::Plain) { true };
    attempt Tried by Receiver on M::Node::step contracts [Ready,Done]
        as (attempted: M::Plain) { true };
    choice Decide by Receiver visible (got.ready, attempted.ready) {
        case yes when { got.ready and attempted.ready } check Accepted using S { true };
        case no when { not (got.ready and attempted.ready) } check Rejected using S { true };
    }
}";

#[test]
#[trace("TC-121", "FR-042-AC-1", "FR-042-AC-4", "FR-042-AC-5", "FR-042-AC-7")]
fn a_received_record_and_an_own_attempt_record_supply_one_combined_partition() {
    let mut inputs = Inputs::new(&[
        Unit {
            name: "mixed-contracts",
            body: "pre Ready using S on M::Node::step { delta >= 0 }\npost Done using S on M::Node::step { result }",
            declarations: &["Ready", "Done"],
        },
        Unit {
            name: "mixed-observations",
            body: &format!(
                "protocol Decisions using P over (view: M::Node) on origin {{
                    role Sender on M::Node;
                    role Receiver on M::Node;
                    channel Messages from Sender to Receiver carries M::Plain
                        ordering unordered delivery [1,1];
                    run {MIXED_RUN}
                    finish Closed as (closed: M::Node) {{ true }};
                }}"
            ),
            declarations: &["Decisions"],
        },
    ]);
    let expected_operation = inputs.step_contracts("Ready", "Done");
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
            let admission = report.into_result().unwrap();
            let package = admission.package();
            assert_eq!((package.sources.len(), package.declarations.len()), (2, 3));
            let find = |name| {
                package
                    .declarations
                    .iter()
                    .position(|d| d.name == name)
                    .unwrap() as u32
            };
            let (ready, done, decisions) = (find("Ready"), find("Done"), find("Decisions"));
            let namespace = proofs.types().binding().namespace();
            for (name, id, source_index) in [
                ("Ready", ready, 0),
                ("Done", done, 0),
                ("Decisions", decisions, 1),
            ] {
                let retained = &package.declarations[id as usize];
                let [original] = namespace.lookup(name) else {
                    panic!("one authored declaration")
                };
                let syntax = namespace.syntax(*original).unwrap();
                assert_eq!(
                    (
                        retained.locus.span.start as usize,
                        retained.locus.span.end as usize
                    ),
                    (syntax.span.start, syntax.span.end)
                );
                let source = &package.sources[retained.locus.source as usize];
                assert_eq!(
                    source.native.identity,
                    inputs.sources[source_index].identity().identity
                );
                assert_eq!(source.text, inputs.sources[source_index].text());
                assert_eq!(
                    source.native.revision,
                    inputs.sources[source_index].identity().revision
                );
            }
            assert_eq!(
                package.declarations[ready as usize].execution,
                w::Execution::Pre {
                    operation: expected_operation.clone()
                }
            );
            assert_eq!(
                package.declarations[done as usize].execution,
                w::Execution::Post {
                    operation: expected_operation.clone()
                }
            );
            let model = &package.models[expected_operation.model as usize];
            assert_eq!(
                model.exports[expected_operation.export as usize].path,
                ["Node", "step"]
            );
            assert_eq!(
                package.dependencies[model.artifact as usize].artifact,
                inputs.model_reference
            );

            let declaration = &package.declarations[decisions as usize];
            let mut contracts = vec![ready, done];
            contracts.sort_unstable();
            assert_eq!(declaration.requires, contracts);
            let w::Body::Protocol {
                roles,
                channels,
                controls,
                causal_edges,
                ..
            } = &declaration.body
            else {
                panic!("protocol")
            };
            let handle = |name| w::Handle {
                declaration: decisions,
                index: controls.iter().position(|c| c.name == name).unwrap() as u32,
            };
            let (got, tried, decide) = (handle("Got"), handle("Tried"), handle("Decide"));
            let w::ControlOperation::Event {
                event: w::Event::Receive { channel, send },
                binder: received,
                ..
            } = &controls[got.index as usize].operation
            else {
                panic!("actual received record")
            };
            assert_eq!(channel.index, 0);
            assert_eq!(channel.declaration, decisions);
            assert_eq!(send, &handle("Sent"));
            let w::ControlOperation::Event {
                event:
                    w::Event::Attempt {
                        owner: attempt_owner,
                        operation,
                        contracts: retained,
                        ..
                    },
                binder: attempted,
                ..
            } = &controls[tried.index as usize].operation
            else {
                panic!("actual own attempt record")
            };
            assert_eq!(operation, &expected_operation);
            assert_eq!(retained, &contracts);
            let w::ControlOperation::Choice {
                owner,
                visible,
                cases,
            } = &controls[decide.index as usize].operation
            else {
                panic!("choice")
            };
            assert_eq!(owner, attempt_owner);
            assert_eq!(owner.declaration, decisions);
            assert_eq!(owner, &channels[0].to);
            assert_eq!(roles[owner.index as usize].name, "Receiver");
            assert_eq!(visible.len(), 2);
            assert_eq!(
                cases.iter().map(|c| c.label.as_str()).collect::<Vec<_>>(),
                ["yes", "no"]
            );
            assert_eq!(cases[0].body, handle("Accepted"));
            assert_eq!(cases[1].body, handle("Rejected"));
            for case in cases {
                assert_eq!(case.guard.declaration, decisions);
                for (kind, from, to) in [
                    (w::EdgeKind::Branch, &decide, &case.body),
                    (w::EdgeKind::Join, &case.body, &decide),
                ] {
                    assert!(causal_edges.iter().any(|edge| edge.owner == decide
                        && edge.kind == kind
                        && edge.from.node == *from
                        && edge.to.node == *to));
                }
            }
            let original_choice = {
                let [id] = namespace.lookup("Decisions") else {
                    panic!("one original protocol")
                };
                let unit = namespace
                    .unit(proofs.types().declaration(*id).unwrap().unit())
                    .unwrap();
                &unit.controls()[controls[decide.index as usize].original_node as usize]
            };
            assert_eq!(original_choice.name.value, "Decide");
            assert_eq!(
                controls[decide.index as usize].locus.source,
                declaration.locus.source
            );
            assert_eq!(
                (
                    controls[decide.index as usize].locus.span.start as usize,
                    controls[decide.index as usize].locus.span.end as usize
                ),
                (original_choice.span.start, original_choice.span.end)
            );

            for (binder, node, name) in [(received, &got, "got"), (attempted, &tried, "attempted")]
            {
                assert_eq!(binder.declaration, decisions);
                let observation = &declaration.binders[binder.index as usize];
                assert_eq!(observation.name, name);
                assert_eq!(observation.kind, w::BinderKind::Event);
                assert_eq!(observation.anchor.declaration, decisions);
                let anchor = &declaration.anchors[observation.anchor.index as usize];
                assert_eq!(anchor.kind, w::AnchorKind::Control);
                assert_eq!(observation.locus.source, declaration.locus.source);
                let source = &package.sources[observation.locus.source as usize].text;
                assert_eq!(
                    &source[observation.locus.span.start as usize
                        ..observation.locus.span.end as usize],
                    name
                );
                assert_eq!(
                    declaration.anchors[observation.anchor.index as usize]
                        .owner
                        .0
                        .as_ref(),
                    Some(node)
                );
            }
            let (received_binder, attempted_binder) = (
                &declaration.binders[received.index as usize],
                &declaration.binders[attempted.index as usize],
            );
            assert_eq!(received_binder.value_type, attempted_binder.value_type);
            assert_ne!(
                received_binder.anchor, attempted_binder.anchor,
                "one record type and field name at two events keeps two facts"
            );

            // Each Boolean atom must be `<observation>.ready` on the fixture's
            // `M::Plain` record, read through its own original binder.
            let atom = |value: &w::Handle, expected_binder: &w::Handle| {
                assert_eq!(value.declaration, decisions);
                let retained = &declaration.values[value.index as usize];
                let w::ValueOperation::Field { base, field } = &retained.operation else {
                    panic!("observation field")
                };
                assert_eq!(base.declaration, decisions);
                assert_eq!(field.model, expected_operation.model);
                assert_eq!(
                    declaration.values[base.index as usize].operation,
                    w::ValueOperation::Read {
                        binder: expected_binder.clone()
                    }
                );
                assert_eq!(
                    package.models[field.model as usize].exports[field.export as usize].path,
                    ["Plain", "ready"]
                );
                assert_eq!(retained.locus.source, declaration.locus.source);
                let text = &package.sources[retained.locus.source as usize].text;
                text[retained.locus.span.start as usize..retained.locus.span.end as usize]
                    .to_string()
            };
            assert_eq!(atom(&visible[0], received), "got.ready");
            assert_eq!(atom(&visible[1], attempted), "attempted.ready");
            let conjunction = |value: &w::Handle| {
                let w::ValueOperation::Binary {
                    operator: w::Binary::And,
                    left,
                    right,
                } = &declaration.values[value.index as usize].operation
                else {
                    panic!("combined guard over both observations")
                };
                assert_eq!(atom(left, received), "got.ready");
                assert_eq!(atom(right, attempted), "attempted.ready");
            };
            conjunction(&cases[0].guard);
            let w::ValueOperation::Unary {
                operator: w::Unary::Not,
                value: negated,
            } = &declaration.values[cases[1].guard.index as usize].operation
            else {
                panic!("complementary guard")
            };
            let w::ValueOperation::Group { value: grouped } =
                &declaration.values[negated.index as usize].operation
            else {
                panic!("authored grouping")
            };
            conjunction(grouped);

            let emitted = native::emit(&admission, Limits::default())
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

#[test]
#[trace("TC-121", "FR-042-AC-5")]
fn mixed_observation_choices_refuse_foreign_owners_and_nonpartitions() {
    for (case, chooser, attempter, visible, yes, no) in [
        (
            "received record belongs to the other role",
            "Sender",
            "Sender",
            "got.ready, attempted.ready",
            "got.ready and attempted.ready",
            "not (got.ready and attempted.ready)",
        ),
        (
            "attempt record belongs to the other role",
            "Receiver",
            "Sender",
            "got.ready, attempted.ready",
            "got.ready and attempted.ready",
            "not (got.ready and attempted.ready)",
        ),
        (
            "attempt atom omitted from the advertised basis",
            "Receiver",
            "Receiver",
            "got.ready",
            "got.ready and attempted.ready",
            "not (got.ready and attempted.ready)",
        ),
        (
            "composite result does not disclose its individual operands",
            "Receiver",
            "Receiver",
            "got.ready and attempted.ready",
            "got.ready",
            "not got.ready",
        ),
        (
            "distinct received and attempt atoms neither cover nor separate",
            "Receiver",
            "Receiver",
            "got.ready, attempted.ready",
            "got.ready",
            "not attempted.ready",
        ),
    ] {
        let inputs = protocol(&format!(
            "sequence Main {{
                send Sent via Messages as (sent: M::Plain) {{ true }};
                receive Got via Messages of Sent as (got: M::Plain) {{ true }};
                attempt Tried by {attempter} on M::Node::step contracts []
                    as (attempted: M::Plain) {{ true }};
                choice Decide by {chooser} visible ({visible}) {{
                    case yes when {{ {yes} }} check Accepted using S {{ true }};
                    case no when {{ {no} }} check Rejected using S {{ true }};
                }}
            }}"
        ));
        inputs.with_proofs(
            TypeLimits::default(),
            proofs::ProofLimits::default(),
            |proofs, selected| {
                discharged(proofs);
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
}

fn protocol(run: &str) -> Inputs {
    let body = format!(
        "protocol Decisions using P over (view: M::Node) on origin {{
          role Sender on M::Node;
          role Receiver on M::Node;
          channel Messages from Sender to Receiver carries M::Plain
            ordering unordered delivery [1,1];
          run {run}
          finish Closed as (closed: M::Node) {{ true }};
        }}"
    );
    Inputs::new(&[Unit {
        name: "mixed-decisions",
        body: &body,
        declarations: &["Decisions"],
    }])
}

fn discharged(report: &proofs::ProofReport<'_, '_, '_>) {
    assert!(report.exhaustion().is_none(), "{:?}", report.exhaustion());
    for declaration in report.declarations() {
        let id = declaration.declaration();
        assert_eq!(
            report.types().disposition(id),
            Some(TypeDisposition::Typed),
            "type causes {:?}",
            report.types().declaration(id).map(|typed| typed.causes())
        );
        assert_eq!(
            declaration.disposition(),
            proofs::ProofDisposition::Discharged,
            "{:?}",
            declaration.causes()
        );
    }
}
