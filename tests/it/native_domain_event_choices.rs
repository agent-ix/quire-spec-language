// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-042: source-owned domain-event Boolean choices retain original role,
//! observation and compensation-registration authority through emission.

use crate::support::native_protocol as setup;

use ix_trace_rs::trace;
use quire_spec_language::checking::composed::{proofs, TypeDisposition, TypeLimits};
use quire_spec_language::protocol_artifact::{native, wire as w, Error, Limits, Unsupported};
use quire_spec_language::syntax::composed as c;
use setup::{Inputs, Unit};

const RECEIVED: &str = "sequence Incoming {
    send Sent via Messages as (sent: M::Plain) { true };
    receive Got via Messages of Sent as (got: M::Plain) { true };
}";

fn decisions(run: &str) -> Inputs {
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
        name: "domain-event-decisions",
        body: &body,
        declarations: &["Decisions"],
    }])
}

fn choice(owner: &str, visible: &str, cases: &str) -> String {
    format!("choice Decide by {owner} visible ({visible}) {{ {cases} }}")
}

fn two_cases(record: &str) -> String {
    format!(
        "case yes when {{ {record}.ready }} check Accepted using S {{ true }};
         case no when {{ not {record}.ready }} check Rejected using S {{ true }};"
    )
}

/// The compensation fixture is the already-admitted qualified-event source: one
/// `Full` obligation over `Main::Applied`, its original `Node::step` operation,
/// cross-unit pre/post contracts and the authored `event ... for Full` node.
fn recovery(decision: &str, after_commit: &str) -> (Inputs, w::ExportRef) {
    let body = format!(
        "protocol RecoveryFlow using P over (view: M::Node) on origin {{
          role Service on M::Node;
          compensate Full for Main::Applied as (forwardFull: M::Node)
              by Service on M::Node::step using T clock \"recovery-Full\" {{
            capture targetFull: M::Total = forwardFull.total;
            activate first (triggerFull: M::Plain) when {{ not triggerFull.ready }} {{
              capture activatedFull: Boolean = triggerFull.ready;
            }}
            within [0,30]; attempts 3 of M::Node;
            retry (earlierFull: M::Node, laterFull: M::Node) {{
              earlierFull.tally < 3 and laterFull.tally = earlierFull.tally + 1
              and targetFull >= 0 and not activatedFull
            }};
            commit Main::Committed;
            recover (recoveredFull: M::Node) {{
              sum<M::Total>(refundFull in recoveredFull.amounts: refundFull) = targetFull
              and not activatedFull
            }};
          }}
          requires temporal Due;
          run sequence Main {{
            attempt Tried by Service on M::Node::step contracts [Before,After]
                as (attempted: M::Plain) {{ attempted.ready }};
            effect Applied of Main::Tried as (applied: M::Node) {{ true }};
            event Recovered by Service for Full
                as (recoveryEvent: M::Plain) {{ recoveryEvent.ready }};
            {decision}
            commit Committed by Service as (committed: M::Plain) {{ committed.ready }};
            {after_commit}
          }}
          finish Closed as (closed: M::Node) {{ true }};
        }}"
    );
    let mut inputs = Inputs::new(&[
        Unit {
            name: "recovery-contracts",
            body: "pre Before using S on M::Node::step { delta >= 0 }
                   post After using S on M::Node::step { result }",
            declarations: &["Before", "After"],
        },
        Unit {
            name: "recovery-time",
            body: "temporal Due using T over (sample: M::Node) clock \"workflow-clock\" on origin { always[0,30] holds(true) }",
            declarations: &["Due"],
        },
        Unit {
            name: "recovery-flow",
            body: &body,
            declarations: &["RecoveryFlow"],
        },
    ]);
    let operation = inputs.step_contracts("Before", "After");
    (inputs, operation)
}

fn discharged(report: &proofs::ProofReport<'_, '_, '_>) {
    assert!(report.exhaustion().is_none(), "{:?}", report.exhaustion());
    for declaration in report.declarations() {
        let id = declaration.declaration();
        assert_eq!(
            report.types().disposition(id),
            Some(TypeDisposition::Typed),
            "{id:?}: type causes {:?}; scope issues {:?}",
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
            "{id:?}: {:?}",
            declaration.causes()
        );
    }
}

/// Admit, emit and re-read through the independent reader, then hand the
/// original proof report and admitted package to the case's own assertions.
fn admitted(
    inputs: &Inputs,
    name: &str,
    inspect: impl FnOnce(&proofs::ProofReport<'_, '_, '_>, &w::Package, u32),
) {
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let report = native::admit(proofs, selected, Limits::default());
            assert!(
                report.result().is_ok(),
                "{name}: {:?}; locus {:?}",
                report.result().err(),
                report.locus()
            );
            let admission = report.into_result().unwrap();
            let package = admission.package();
            assert_eq!(package.sources.len(), inputs.sources.len());
            for source in &package.sources {
                let original = inputs
                    .sources
                    .iter()
                    .find(|selected| selected.identity().identity == source.native.identity)
                    .expect("every emitted source keeps an original authored identity");
                assert_eq!(source.native.revision, original.identity().revision);
                assert_eq!(source.text, original.text());
            }
            let owner = package
                .declarations
                .iter()
                .position(|declaration| declaration.name == name)
                .expect("authored protocol declaration") as u32;
            inspect(proofs, package, owner);
            let emitted = native::emit(&admission, Limits::default())
                .into_result()
                .unwrap();
            let read = inputs.read(proofs, &emitted);
            assert!(
                read.result().is_ok(),
                "independent reader: {:?}; locus {:?}",
                read.result().err(),
                read.locus()
            );
            let read = read.into_result().unwrap();
            assert_eq!(read.package(), package);
            assert_eq!(read.digest(), emitted.digest());
        },
    );
}

fn refused(inputs: &Inputs, case: &str) {
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

#[track_caller]
fn event_node<'a>(controls: &'a [w::Control], name: &str) -> (u32, &'a w::Event, &'a w::Handle) {
    let index = controls
        .iter()
        .position(|control| control.name == name)
        .unwrap_or_else(|| panic!("authored {name} node"));
    let w::ControlOperation::Event { event, binder, .. } = &controls[index].operation else {
        panic!("{name} is an authored event node")
    };
    (index as u32, event, binder)
}

/// Each advertised and guard operand is the original Boolean field read of the
/// exact event record, with its authored model export and source region.
#[track_caller]
fn boolean_field(
    package: &w::Package,
    declaration: &w::Declaration,
    value: &w::Handle,
    binder: &w::Handle,
    text: &str,
) {
    assert_eq!(value.declaration, binder.declaration);
    let retained = &declaration.values[value.index as usize];
    let w::ValueOperation::Field { base, field } = &retained.operation else {
        panic!("{text} is an authored Boolean record field")
    };
    assert_eq!(base.declaration, binder.declaration);
    assert_eq!(
        declaration.values[base.index as usize].operation,
        w::ValueOperation::Read {
            binder: binder.clone()
        }
    );
    assert_eq!(
        package.models[field.model as usize].exports[field.export as usize].path,
        ["Plain", "ready"]
    );
    assert_eq!(retained.locus.source, declaration.locus.source);
    let source = &package.sources[retained.locus.source as usize].text;
    assert_eq!(
        &source[retained.locus.span.start as usize..retained.locus.span.end as usize],
        text
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-1", "FR-042-AC-5", "FR-042-AC-7")]
fn same_owner_domain_event_supplies_its_own_boolean_decision_fact() {
    let cases = two_cases("observed");
    let decision = choice("Receiver", "observed.ready", &cases);
    let inputs = decisions(&format!(
        "sequence Main {{
          event Observed by Receiver as (observed: M::Plain) {{ observed.ready }};
          {decision}
        }}"
    ));
    admitted(&inputs, "Decisions", |proofs, package, owner| {
        let declaration = &package.declarations[owner as usize];
        let w::Body::Protocol {
            roles, controls, ..
        } = &declaration.body
        else {
            panic!("protocol family")
        };
        let (index, event, binder) = event_node(controls, "Observed");
        let w::Event::Event {
            owner: role,
            compensation,
            instance,
        } = event
        else {
            panic!("authored domain event, not a send/receive/effect")
        };
        assert!(
            compensation.0.is_none(),
            "this source associates no compensation"
        );
        assert_eq!(roles[role.index as usize].name, "Receiver");
        let node = w::Handle {
            declaration: owner,
            index,
        };
        let record = &declaration.binders[binder.index as usize];
        assert_eq!(binder.declaration, owner);
        assert_eq!(role.declaration, owner);
        assert_eq!(record.anchor.declaration, owner);
        assert_eq!(record.name, "observed");
        assert_eq!(record.kind, w::BinderKind::Event);
        let anchor = &declaration.anchors[record.anchor.index as usize];
        assert_eq!(anchor.kind, w::AnchorKind::Control);
        assert_eq!(anchor.owner.0.as_ref(), Some(&node));
        let observation = &declaration.bindings[*instance as usize];
        assert_eq!(observation.kind, w::BindingKind::Invocation);
        assert_eq!(
            observation.subject,
            w::Subject::Control {
                control: node.clone()
            }
        );
        assert_eq!(observation.requires, [roles[role.index as usize].instance]);

        let decide = controls
            .iter()
            .position(|control| control.name == "Decide")
            .expect("authored choice");
        let w::ControlOperation::Choice {
            owner: chooser,
            visible,
            cases,
        } = &controls[decide].operation
        else {
            panic!("authored choice")
        };
        assert_eq!(chooser, role, "the deciding role owns the observed event");
        assert_eq!(
            cases
                .iter()
                .map(|case| case.label.as_str())
                .collect::<Vec<_>>(),
            ["yes", "no"]
        );
        for value in [&visible[0], &cases[0].guard] {
            boolean_field(package, declaration, value, binder, "observed.ready");
        }
        let field_model = match &declaration.values[visible[0].index as usize].operation {
            w::ValueOperation::Field { field, .. } => field.model,
            _ => panic!("advertised Boolean field"),
        };
        assert_eq!(
            package.dependencies[package.models[field_model as usize].artifact as usize].artifact,
            inputs.model_reference
        );

        let namespace = proofs.types().binding().namespace();
        let [id] = namespace.lookup("Decisions") else {
            panic!("one original protocol declaration")
        };
        let unit = namespace
            .unit(proofs.types().declaration(*id).unwrap().unit())
            .unwrap();
        let original = &unit.controls()[controls[index as usize].original_node as usize];
        assert!(matches!(
            &original.kind,
            c::ControlKind::Event(c::Event {
                kind: c::EventKind::Event {
                    compensation: None,
                    ..
                },
                ..
            })
        ));
        assert_eq!(
            (
                controls[index as usize].locus.span.start as usize,
                controls[index as usize].locus.span.end as usize
            ),
            (original.span.start, original.span.end)
        );
        assert_eq!(
            (
                record.locus.span.start as usize,
                record.locus.span.end as usize
            ),
            match &original.kind {
                c::ControlKind::Event(event) => (
                    event.parameter.name.span.start,
                    event.parameter.name.span.end
                ),
                _ => unreachable!("checked above"),
            }
        );
    });
}

#[test]
#[trace("TC-121", "FR-042-AC-5")]
fn a_foreign_role_cannot_decide_on_another_roles_event_of_the_same_model_type() {
    let cases = two_cases("observed");
    let decision = choice("Sender", "observed.ready", &cases);
    let inputs = decisions(&format!(
        "sequence Main {{
          event Observed by Receiver as (observed: M::Plain) {{ observed.ready }};
          {decision}
        }}"
    ));
    refused(
        &inputs,
        "foreign role decides on another role's domain event",
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-1", "FR-042-AC-5", "FR-042-AC-7")]
fn all_joined_event_and_received_records_keep_two_distinct_boolean_atoms() {
    for (case, cases, succeeds) in [
        (
            "four-way partition over both joined facts",
            "case both when { observed.ready and got.ready } check Both using S { true };
             case onlyObserved when { observed.ready and not got.ready }
               check OnlyObserved using S { true };
             case onlyReceived when { not observed.ready and got.ready }
               check OnlyReceived using S { true };
             case neither when { not observed.ready and not got.ready }
               check Neither using S { true };",
            true,
        ),
        (
            "one gap and one overlap across the two distinct facts",
            "case yes when { observed.ready } check Accepted using S { true };
             case no when { not got.ready } check Rejected using S { true };",
            false,
        ),
    ] {
        let decision = choice("Receiver", "observed.ready, got.ready", cases);
        let inputs = decisions(&format!(
            "sequence Main {{
              parallel Gather {{
                branch observing event Observed by Receiver
                  as (observed: M::Plain) {{ observed.ready }};
                branch incoming {RECEIVED}
              }} join all [observing,incoming];
              {decision}
            }}"
        ));
        if !succeeds {
            refused(&inputs, case);
            continue;
        }
        admitted(&inputs, "Decisions", |_, package, owner| {
            let declaration = &package.declarations[owner as usize];
            let w::Body::Protocol {
                controls,
                causal_edges,
                ..
            } = &declaration.body
            else {
                panic!("protocol family")
            };
            let gather = w::Handle {
                declaration: owner,
                index: controls
                    .iter()
                    .position(|control| control.name == "Gather")
                    .unwrap() as u32,
            };
            let w::ControlOperation::Parallel { branches, join } =
                &controls[gather.index as usize].operation
            else {
                panic!("authored all-branch join")
            };
            assert_eq!(join, &[0, 1]);
            assert_eq!(
                branches
                    .iter()
                    .map(|branch| branch.label.as_str())
                    .collect::<Vec<_>>(),
                ["observing", "incoming"]
            );
            for branch in branches {
                for (kind, from, to) in [
                    (w::EdgeKind::Branch, &gather, &branch.body),
                    (w::EdgeKind::Join, &branch.body, &gather),
                ] {
                    assert!(causal_edges.iter().any(|edge| edge.owner == gather
                        && edge.kind == kind
                        && edge.from.node == *from
                        && edge.to.node == *to));
                }
            }
            let (_, event, observed) = event_node(controls, "Observed");
            assert!(matches!(event, w::Event::Event { .. }));
            let (_, received, got) = event_node(controls, "Got");
            assert!(matches!(received, w::Event::Receive { .. }));
            let observed = &declaration.binders[observed.index as usize];
            let got = &declaration.binders[got.index as usize];
            assert_eq!(
                observed.value_type, got.value_type,
                "one model record type supplies both facts"
            );
            assert_ne!(
                observed.anchor, got.anchor,
                "an event and a receive of the same record keep two facts"
            );
            let w::ControlOperation::Choice { cases, visible, .. } = &controls[controls
                .iter()
                .position(|control| control.name == "Decide")
                .expect("authored choice")]
            .operation
            else {
                panic!("authored choice")
            };
            assert_eq!(visible.len(), 2);
            assert_eq!(
                cases
                    .iter()
                    .map(|case| case.label.as_str())
                    .collect::<Vec<_>>(),
                ["both", "onlyObserved", "onlyReceived", "neither"]
            );
        });
    }
}

#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-6", "FR-042-AC-7")]
fn a_compensation_qualified_event_decides_for_its_owner_and_keeps_the_association() {
    // The no-choice baseline proves this fixture is the already-admitted
    // qualified-event source before the decision is added.
    let (baseline, baseline_operation) = recovery("", "");
    admitted(&baseline, "RecoveryFlow", |_, package, owner| {
        let w::Body::Protocol { controls, .. } = &package.declarations[owner as usize].body else {
            panic!("protocol family")
        };
        let (_, event, _) = event_node(controls, "Recovered");
        assert!(matches!(
            event,
            w::Event::Event {
                compensation: w::Nullable(Some(_)),
                ..
            }
        ));
        assert!(
            !controls
                .iter()
                .any(|control| matches!(control.operation, w::ControlOperation::Choice { .. })),
            "the baseline authors no decision"
        );
    });

    let cases = two_cases("recoveryEvent");
    let (inputs, operation) = recovery(&choice("Service", "recoveryEvent.ready", &cases), "");
    assert_eq!(
        operation, baseline_operation,
        "adding a decision selects the same original operation export"
    );
    admitted(&inputs, "RecoveryFlow", |proofs, package, owner| {
        let declaration = &package.declarations[owner as usize];
        let w::Body::Protocol {
            compensations,
            controls,
            roles,
            ..
        } = &declaration.body
        else {
            panic!("protocol family")
        };
        let [full] = compensations.as_slice() else {
            panic!("one authored obligation")
        };
        assert_eq!(full.name, "Full");
        assert_eq!(full.operation, operation);
        let handle = w::Handle {
            declaration: owner,
            index: 0,
        };
        let subject = w::Subject::Compensation {
            compensation: handle.clone(),
        };
        let (index, event, binder) = event_node(controls, "Recovered");
        let namespace = proofs.types().binding().namespace();
        let [id] = namespace.lookup("RecoveryFlow") else {
            panic!("original recovery protocol")
        };
        let unit = namespace
            .unit(proofs.types().declaration(*id).unwrap().unit())
            .unwrap();
        let original = &unit.controls()[controls[index as usize].original_node as usize];
        let c::ControlKind::Event(c::Event {
            kind:
                c::EventKind::Event {
                    compensation: Some(target),
                    ..
                },
            ..
        }) = &original.kind
        else {
            panic!("original qualified event")
        };
        assert_eq!(unit.source().slice(target.span), Some("Full"));
        assert_eq!(
            controls[index as usize].locus.source,
            declaration.locus.source
        );
        assert_eq!(
            (
                controls[index as usize].locus.span.start as usize,
                controls[index as usize].locus.span.end as usize
            ),
            (original.span.start, original.span.end)
        );
        let w::Event::Event {
            owner: role,
            compensation,
            instance,
        } = event
        else {
            panic!("typed domain event with a retained association")
        };
        assert_eq!(
            compensation.0.as_ref(),
            Some(&handle),
            "the authored `for Full` association is retained exactly"
        );
        assert_eq!(role, &full.owner);
        assert_eq!(roles[role.index as usize].name, "Service");
        assert_eq!(
            controls[full.forward_effect.index as usize].name, "Applied",
            "the original successful forward effect is preserved"
        );
        assert_eq!(
            controls[full.commit.0.as_ref().expect("authored commit").index as usize].name,
            "Committed"
        );

        // The event's own observation and every compensation runtime binding keep
        // their original ownership and prerequisites; a decision adds no evidence.
        let observation = &declaration.bindings[*instance as usize];
        assert_eq!(observation.kind, w::BindingKind::Invocation);
        assert_eq!(
            observation.subject,
            w::Subject::Control {
                control: w::Handle {
                    declaration: owner,
                    index
                }
            }
        );
        let mut expected = vec![
            roles[role.index as usize].instance,
            full.registration_instance,
        ];
        expected.sort_unstable();
        assert_eq!(observation.requires, expected);
        let registration = &declaration.bindings[full.registration_instance as usize];
        assert_eq!(registration.kind, w::BindingKind::CompensationRegistration);
        assert_eq!(registration.subject, subject);
        let activation_anchor = &declaration.anchors[full.activation_anchor.index as usize];
        assert_eq!(
            activation_anchor.kind,
            w::AnchorKind::CompensationActivation
        );
        assert_eq!(activation_anchor.owner.0.as_ref(), Some(&handle));
        let activation = activation_anchor
            .binding
            .0
            .expect("activation keeps its own observation requirement");
        assert_eq!(
            declaration.bindings[activation as usize].kind,
            w::BindingKind::Observation
        );
        assert_eq!(declaration.bindings[activation as usize].subject, subject);
        assert_eq!(
            declaration.bindings[activation as usize].requires,
            [full.registration_instance]
        );
        let attempt = &declaration.bindings[full.attempt_instance as usize];
        assert_eq!(attempt.kind, w::BindingKind::CompensationAttempt);
        assert_eq!(attempt.model.0.as_ref(), Some(&operation));
        let mut attempt_requires = vec![roles[role.index as usize].instance, activation];
        attempt_requires.sort_unstable();
        assert_eq!(attempt.requires, attempt_requires);
        let effect = &declaration.bindings[full.effect_instance as usize];
        assert_eq!(effect.kind, w::BindingKind::CompensationEffect);
        assert_eq!(effect.requires, [full.attempt_instance]);

        let record = &declaration.binders[binder.index as usize];
        assert_eq!(record.name, "recoveryEvent");
        assert_eq!(record.kind, w::BinderKind::Event);
        let w::ControlOperation::Choice {
            owner: chooser,
            visible,
            cases,
        } = &controls[controls
            .iter()
            .position(|control| control.name == "Decide")
            .expect("authored choice")]
        .operation
        else {
            panic!("authored choice")
        };
        assert_eq!(chooser, role);
        for value in [&visible[0], &cases[0].guard] {
            boolean_field(package, declaration, value, binder, "recoveryEvent.ready");
        }
    });
}

#[test]
#[trace("TC-121", "FR-042-AC-5")]
fn a_commit_record_does_not_supply_a_boolean_decision_fact() {
    let cases = two_cases("committed");
    let (inputs, _) = recovery("", &choice("Service", "committed.ready", &cases));
    refused(
        &inputs,
        "commit record is not an eligible event observation",
    );
}
