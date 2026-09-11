// SPDX-License-Identifier: AGPL-3.0-only
//! Received Boolean decision facts pass through actual compiler stages. These
//! tests establish static admission and retained graphs, not runtime choices.

#[path = "support/native_protocol/mod.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_spec_language::checking::composed::{proofs, TypeDisposition, TypeLimits};
use quire_spec_language::linking::composed::scopes::ScopeIssue;
use quire_spec_language::protocol_artifact::{
    self as artifact, native, wire as w, Error, Invalid, Limits, Unsupported,
};
use quire_spec_language::syntax::{composed as c, ExprKind};
use setup::{Inputs, Unit};

const RECEIVE_A: &str = "send SentA via Messages as (sentA: M::Plain) { true };
    receive GotA via Messages of SentA as (gotA: M::Plain) { true };";
const RECEIVE_B: &str = "send SentB via Messages as (sentB: M::Plain) { true };
    receive GotB via Messages of SentB as (gotB: M::Plain) { true };";

fn inputs(run: &str) -> Inputs {
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
        name: "received-decisions",
        body: &body,
        declarations: &["Decisions"],
    }])
}

fn choice(owner: &str, visible: &str, yes: &str, no: &str) -> String {
    format!(
        "choice Decide by {owner} visible ({visible}) {{
          case yes when {{ {yes} }} check Accepted using S {{ true }};
          case no when {{ {no} }} check Rejected using S {{ true }};
        }}"
    )
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
                .and_then(|s| s.declaration(id))
                .map(|s| &s.issues)
        );
        assert_eq!(
            declaration.disposition(),
            proofs::ProofDisposition::Discharged,
            "{:?}",
            declaration.causes()
        );
    }
}

fn admitted(inputs: &Inputs, inspect: impl FnOnce(&proofs::ProofReport<'_, '_, '_>, &w::Package)) {
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
            assert_eq!(package.sources.len(), 1);
            assert_eq!(
                package.sources[0].native.identity,
                inputs.sources[0].identity().identity
            );
            assert_eq!(
                package.sources[0].native.revision,
                inputs.sources[0].identity().revision
            );
            assert_eq!(package.sources[0].text, inputs.sources[0].text());
            inspect(proofs, package);
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

fn original_choices(proofs: &proofs::ProofReport<'_, '_, '_>, package: &w::Package) {
    let declaration = &package.declarations[0];
    let namespace = proofs.types().binding().namespace();
    let [id] = namespace.lookup("Decisions") else {
        panic!("one original declaration")
    };
    let unit = namespace
        .unit(proofs.types().declaration(*id).unwrap().unit())
        .unwrap();
    let w::Body::Protocol {
        controls,
        causal_edges,
        ..
    } = &declaration.body
    else {
        panic!("protocol")
    };
    let expression = |handle: &w::Handle, original| {
        assert_eq!(handle.declaration, 0);
        let retained = &declaration.values[handle.index as usize];
        let expected = unit.expression(original).unwrap();
        assert_eq!(
            &unit.expressions()[retained.original_expression as usize],
            expected
        );
        assert_eq!(
            (
                retained.locus.span.start as usize,
                retained.locus.span.end as usize
            ),
            (expected.span.start, expected.span.end)
        );
    };
    for retained in &declaration.values {
        let original = &unit.expressions()[retained.original_expression as usize];
        match (&retained.operation, &original.kind) {
            (
                w::ValueOperation::Group { value },
                c::ValueKind::Shared(ExprKind::Group { inner }),
            ) => expression(value, *inner),
            (
                w::ValueOperation::Boolean { value },
                c::ValueKind::Shared(ExprKind::Boolean(expected)),
            ) => assert_eq!(value, expected),
            (w::ValueOperation::Read { binder }, c::ValueKind::Shared(ExprKind::Name(name))) => {
                assert_eq!(declaration.binders[binder.index as usize].name, name.value)
            }
            (
                w::ValueOperation::Field { base, field },
                c::ValueKind::Shared(ExprKind::Field {
                    base: receiver,
                    name,
                }),
            ) => {
                expression(base, *receiver);
                assert_eq!(
                    package.models[field.model as usize].exports[field.export as usize]
                        .path
                        .last(),
                    Some(&name.value)
                );
            }
            (
                w::ValueOperation::Unary { value, .. },
                c::ValueKind::Shared(ExprKind::Unary { argument, .. }),
            ) => expression(value, *argument),
            (
                w::ValueOperation::Binary { left, right, .. },
                c::ValueKind::Shared(ExprKind::Binary {
                    left: first,
                    right: second,
                    ..
                }),
            ) => {
                expression(left, *first);
                expression(right, *second);
            }
            (
                w::ValueOperation::If {
                    condition,
                    then_value,
                    else_value,
                },
                c::ValueKind::Shared(ExprKind::If {
                    condition: test,
                    then_value: yes,
                    else_value: no,
                }),
            ) => {
                expression(condition, *test);
                expression(then_value, *yes);
                expression(else_value, *no);
            }
            (
                w::ValueOperation::Let {
                    binder,
                    initializer,
                    body,
                },
                c::ValueKind::Shared(ExprKind::Let {
                    name,
                    value,
                    body: result,
                }),
            ) => {
                assert_eq!(declaration.binders[binder.index as usize].name, name.value);
                assert_eq!(
                    declaration.binders[binder.index as usize]
                        .initializer
                        .0
                        .as_ref(),
                    Some(initializer)
                );
                expression(initializer, *value);
                expression(body, *result);
            }
            _ => panic!(
                "fixture Boolean operation did not retain its original shape: {:?}",
                original.kind
            ),
        }
    }
    let mut found = 0;
    for (index, control) in controls.iter().enumerate() {
        let original = &unit.controls()[control.original_node as usize];
        let c::ControlKind::Choice {
            visible: original_visible,
            cases: original_cases,
            ..
        } = &original.kind
        else {
            continue;
        };
        let w::ControlOperation::Choice { visible, cases, .. } = &control.operation else {
            panic!("retained authored choice")
        };
        assert_eq!(control.name, original.name.value);
        assert_eq!(
            (
                control.locus.span.start as usize,
                control.locus.span.end as usize
            ),
            (original.span.start, original.span.end)
        );
        assert_eq!(visible.len(), original_visible.len());
        for (retained, original) in visible.iter().zip(original_visible) {
            expression(retained, *original);
        }
        assert_eq!(cases.len(), original_cases.len());
        for (retained, original) in cases.iter().zip(original_cases) {
            assert_eq!(retained.label, original.name.value);
            expression(&retained.guard, original.guard);
            let body = &controls[retained.body.index as usize];
            assert_eq!(
                &unit.controls()[body.original_node as usize],
                unit.control(original.control).unwrap()
            );
            let owner = w::Handle {
                declaration: 0,
                index: index as u32,
            };
            for (kind, from, to) in [
                (w::EdgeKind::Branch, &owner, &retained.body),
                (w::EdgeKind::Join, &retained.body, &owner),
            ] {
                assert!(causal_edges.iter().any(|edge| edge.owner == owner
                    && edge.kind == kind
                    && edge.from.node == *from
                    && edge.to.node == *to));
            }
        }
        found += 1;
    }
    assert!(found > 0);
}

#[test]
#[trace("TC-121", "FR-042-AC-1", "FR-042-AC-5", "FR-042-AC-7")]
fn received_boolean_formula_preserves_repeated_atoms_aliases_and_original_operators() {
    // Independent Boolean identity: both guards are exactly A and not A;
    // B or not B preserves the same repeated B atom, not two arbitrary facts.
    let decision = choice(
        "Receiver", "(let advertisedA = gotA.ready in advertisedA), (gotB.ready)",
        "let decisionA = gotA.ready in if (decisionA and (gotB.ready or not gotB.ready)) then (decisionA = true) else (decisionA != false)",
        "gotA.ready implies false",
    );
    let inputs = inputs(&format!(
        "sequence Main {{ {RECEIVE_A} {RECEIVE_B} {decision} }}"
    ));
    admitted(&inputs, |proofs, package| {
        original_choices(proofs, package);
        let declaration = &package.declarations[0];
        let w::Body::Protocol {
            controls,
            roles,
            channels,
            ..
        } = &declaration.body
        else {
            panic!("protocol")
        };
        let decide = controls.iter().find(|c| c.name == "Decide").unwrap();
        let w::ControlOperation::Choice { owner, cases, .. } = &decide.operation else {
            panic!("choice")
        };
        assert_eq!(roles[owner.index as usize].name, "Receiver");
        assert_eq!(*owner, channels[0].to);
        assert_eq!(
            cases.iter().map(|c| c.label.as_str()).collect::<Vec<_>>(),
            ["yes", "no"]
        );
        let received: Vec<_> = ["GotA", "GotB"]
            .into_iter()
            .map(|name| {
                let control = controls.iter().find(|c| c.name == name).unwrap();
                let w::ControlOperation::Event {
                    event: w::Event::Receive { channel, .. },
                    binder,
                    ..
                } = &control.operation
                else {
                    panic!("actual received record")
                };
                assert_eq!(channel.index, 0);
                &declaration.binders[binder.index as usize]
            })
            .collect();
        assert_eq!(received[0].value_type, received[1].value_type);
        assert_ne!(
            received[0].anchor, received[1].anchor,
            "same model field at two receives keeps two facts"
        );
        for operator in [
            w::Binary::And,
            w::Binary::Or,
            w::Binary::Implies,
            w::Binary::Equal,
            w::Binary::NotEqual,
        ] {
            assert!(declaration.values.iter().any(|v| matches!(&v.operation, w::ValueOperation::Binary { operator: actual, .. } if *actual == operator)));
        }
        assert!(declaration
            .values
            .iter()
            .any(|v| matches!(v.operation, w::ValueOperation::If { .. })));
        assert!(declaration
            .values
            .iter()
            .any(|v| matches!(v.operation, w::ValueOperation::Let { .. })));
        assert!(declaration
            .values
            .iter()
            .any(|v| matches!(v.operation, w::ValueOperation::Group { .. })));
        assert!(declaration.values.iter().any(|v| matches!(
            v.operation,
            w::ValueOperation::Unary {
                operator: w::Unary::Not,
                ..
            }
        )));
    });
}

#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-7")]
fn parallel_all_join_supplies_two_distinct_received_atoms_to_a_four_way_partition() {
    let inputs = inputs(&format!(
        "sequence Main {{ parallel Gather {{
           branch left sequence Left {{ {RECEIVE_A} }}
           branch right sequence Right {{ {RECEIVE_B} }}
         }} join all [left,right];
         choice Decide by Receiver visible (gotA.ready, gotB.ready) {{
           case both when {{ gotA.ready and gotB.ready }} check Both using S {{ true }};
           case onlyA when {{ gotA.ready and not gotB.ready }} check OnlyA using S {{ true }};
           case onlyB when {{ not gotA.ready and gotB.ready }} check OnlyB using S {{ true }};
           case neither when {{ not gotA.ready and not gotB.ready }} check Neither using S {{ true }};
         }} }}"
    ));
    admitted(&inputs, |proofs, package| {
        original_choices(proofs, package);
        let w::Body::Protocol { controls, .. } = &package.declarations[0].body else {
            panic!("protocol")
        };
        let gather = controls.iter().find(|c| c.name == "Gather").unwrap();
        let w::ControlOperation::Parallel { branches, join } = &gather.operation else {
            panic!("all-branch join")
        };
        assert_eq!(
            branches
                .iter()
                .map(|b| b.label.as_str())
                .collect::<Vec<_>>(),
            ["left", "right"]
        );
        assert_eq!(join, &[0, 1]);
        let decide = controls.iter().find(|c| c.name == "Decide").unwrap();
        let w::ControlOperation::Choice { cases, .. } = &decide.operation else {
            panic!("choice")
        };
        assert_eq!(
            cases.iter().map(|c| c.label.as_str()).collect::<Vec<_>>(),
            ["both", "onlyA", "onlyB", "neither"]
        );
    });
}

#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-8")]
fn role_knowledge_visible_basis_and_abstract_partition_failures_do_not_grant_admission() {
    for (case, role, visible, yes, no, expected) in [
        (
            "wrong recipient",
            "Sender",
            "gotA.ready",
            "gotA.ready",
            "not gotA.ready",
            Error::Unsupported(Unsupported::FamilyProof),
        ),
        (
            "workflow input",
            "Receiver",
            "view.plain.ready",
            "view.plain.ready",
            "not view.plain.ready",
            Error::Unsupported(Unsupported::FamilyProof),
        ),
        (
            "hidden received atom",
            "Receiver",
            "gotA.ready",
            "gotA.ready and gotB.ready",
            "not (gotA.ready and gotB.ready)",
            Error::Unsupported(Unsupported::FamilyProof),
        ),
        (
            "composite advertised value",
            "Receiver",
            "gotA.ready and gotB.ready",
            "gotA.ready and gotB.ready",
            "not (gotA.ready and gotB.ready)",
            Error::Unsupported(Unsupported::FamilyProof),
        ),
        (
            "distinct receive gap and overlap",
            "Receiver",
            "gotA.ready, gotB.ready",
            "gotA.ready",
            "not gotB.ready",
            Error::Unsupported(Unsupported::FamilyProof),
        ),
        (
            "nonconstant overlap",
            "Receiver",
            "gotA.ready",
            "gotA.ready",
            "true",
            Error::Unsupported(Unsupported::FamilyProof),
        ),
        (
            "only both-true is uncovered",
            "Receiver",
            "gotA.ready, gotB.ready",
            "not gotA.ready",
            "gotA.ready and not gotB.ready",
            Error::Unsupported(Unsupported::FamilyProof),
        ),
        (
            "only both-true overlaps",
            "Receiver",
            "gotA.ready, gotB.ready",
            "gotA.ready",
            "not gotA.ready or gotB.ready",
            Error::Unsupported(Unsupported::FamilyProof),
        ),
        (
            "closed overlap",
            "Receiver",
            "true",
            "true",
            "not false",
            Error::Invalid(Invalid::Control),
        ),
        (
            "closed gap",
            "Receiver",
            "true",
            "false",
            "not true",
            Error::Invalid(Invalid::Control),
        ),
    ] {
        let decision = choice(role, visible, yes, no);
        let inputs = inputs(&format!(
            "sequence Main {{ {RECEIVE_A} {RECEIVE_B} {decision} }}"
        ));
        inputs.with_proofs(
            TypeLimits::default(),
            proofs::ProofLimits::default(),
            |proofs, selected| {
                discharged(proofs);
                let report = native::admit(proofs, selected, Limits::default());
                assert_eq!(
                    report.result().err(),
                    Some(&expected),
                    "{case}; locus {:?}",
                    report.locus()
                );
            },
        );
    }
}

#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-8")]
fn sibling_optional_branch_and_await_records_refuse_at_the_original_lexical_use() {
    let decision = choice("Receiver", "gotA.ready", "gotA.ready", "not gotA.ready");
    for (case, run) in [
        ("sibling before join", format!("parallel Both {{ branch incoming sequence Incoming {{ {RECEIVE_A} }} branch deciding {decision} }} join all [incoming,deciding];")),
        ("optional choice branch", format!("sequence Main {{ choice Optional by Receiver visible (true) {{ case selected when {{ true }} sequence Incoming {{ {RECEIVE_A} }} case absent when {{ false }} check Absent using S {{ true }}; }} {decision} }}")),
        ("await match after join", format!("sequence Main {{ send SentA via Messages as (sentA: M::Plain) {{ true }}; await Wait after Main::SentA using T clock \"response\" within [0,2] match receive GotA via Messages of Main::SentA as (gotA: M::Plain) {{ true }}; then check Arrived using S {{ true }}; timeout check Expired using S {{ true }}; {decision} }}")),
    ] {
        let inputs = inputs(&run);
        inputs.with_proofs(TypeLimits::default(), proofs::ProofLimits::default(), |proofs, selected| {
            let namespace = proofs.types().binding().namespace();
            let [id] = namespace.lookup("Decisions") else { panic!("one declaration") };
            let scope = proofs.types().binding().scopes().unwrap().declaration(*id).unwrap();
            let unavailable: Vec<_> = scope.issues.iter().filter_map(|issue| match issue {
                ScopeIssue::OutOfScope { name, span, .. } if name == "gotA" => Some(span),
                _ => None,
            }).collect();
            assert_eq!(unavailable.len(), 3, "{case}: each original visibility/guard use refuses; issues {:?}", scope.issues);
            for span in unavailable { assert_eq!(&inputs.sources[0].text()[span.start..span.end], "gotA"); }
            assert_eq!(proofs.types().disposition(*id), Some(TypeDisposition::Refused));
            assert_eq!(proofs.disposition(*id), Some(proofs::ProofDisposition::Refused));
            let report = native::admit(proofs, selected, Limits::default());
            assert_eq!(report.result().err(), Some(&Error::Unsupported(Unsupported::FamilyProof)), "{case}");
        });
    }
}

#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-7", "FR-042-AC-8")]
fn repeat_progress_covers_every_feasible_dynamic_case_and_keeps_dead_cases() {
    for (case, no_body, after, guard, succeeds) in [
        (
            "both feasible cases progress",
            "event Rejected by Receiver as (rejected: M::Plain) { true };",
            "",
            "true",
            true,
        ),
        (
            "one feasible case cannot progress",
            "check Rejected using S { true };",
            "",
            "true",
            false,
        ),
        (
            "later guaranteed event supplies progress",
            "check Rejected using S { true };",
            "event Advanced by Receiver as (advanced: M::Plain) { true };",
            "true",
            true,
        ),
        (
            "dynamic repeat guard",
            "event Rejected by Receiver as (rejected: M::Plain) { true };",
            "",
            "gotA.ready",
            false,
        ),
    ] {
        let visible = if guard == "true" {
            "true"
        } else {
            "gotA.ready"
        };
        let run = format!("sequence Main {{ {RECEIVE_A}
          repeat Loop by Receiver visible ({visible}) max 2 while {{ {guard} }} sequence Iteration {{
            choice Decide by Receiver visible (gotA.ready) {{
              case yes when {{ gotA.ready }} event Accepted by Receiver as (accepted: M::Plain) {{ true }};
              case no when {{ not gotA.ready }} {no_body}
              case dead when {{ false }} check Dead using S {{ true }};
            }} {after}
          }} exhausted check Limit using S {{ true }};
        }}");
        let inputs = inputs(&run);
        if succeeds {
            admitted(&inputs, |proofs, package| {
                original_choices(proofs, package);
                let w::Body::Protocol { controls, .. } = &package.declarations[0].body else {
                    panic!("protocol")
                };
                let choice = controls.iter().find(|c| c.name == "Decide").unwrap();
                let w::ControlOperation::Choice { cases, .. } = &choice.operation else {
                    panic!("choice")
                };
                assert_eq!(
                    cases.iter().map(|c| c.label.as_str()).collect::<Vec<_>>(),
                    ["yes", "no", "dead"]
                );
                assert!(matches!(
                    controls[cases[2].body.index as usize].operation,
                    w::ControlOperation::Check { .. }
                ));
                let repeat = controls.iter().find(|c| c.name == "Loop").unwrap();
                let w::ControlOperation::Repeat { maximum, .. } = &repeat.operation else {
                    panic!("repeat")
                };
                let artifact::ProtocolNumber::Integer(maximum) = maximum.checked().unwrap() else {
                    panic!("authored integer")
                };
                assert_eq!(maximum.value(), 2);
            });
        } else {
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
}

#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-7", "FR-042-AC-8")]
fn an_unused_boolean_let_initializer_still_requires_its_received_atom() {
    for (visible, succeeds) in [("gotB.ready", true), ("true", false)] {
        let decision = choice(
            "Receiver",
            visible,
            "let discardedB = gotB.ready in true",
            "false",
        );
        let inputs = inputs(&format!("sequence Main {{ {RECEIVE_B} {decision} }}"));
        if succeeds {
            admitted(&inputs, |proofs, package| {
                original_choices(proofs, package);
                let declaration = &package.declarations[0];
                let discarded = declaration
                    .binders
                    .iter()
                    .find(|b| b.name == "discardedB")
                    .unwrap();
                let initializer = discarded
                    .initializer
                    .0
                    .as_ref()
                    .expect("authored evaluated initializer");
                assert!(matches!(
                    declaration.values[initializer.index as usize].operation,
                    w::ValueOperation::Field { .. }
                ));
                let initializers: Vec<_> = declaration
                    .values
                    .iter()
                    .filter_map(|v| match &v.operation {
                        w::ValueOperation::Let {
                            initializer, body, ..
                        } => Some((initializer, body)),
                        _ => None,
                    })
                    .collect();
                assert_eq!(initializers.len(), 1);
                assert_eq!(initializers[0].0, initializer);
                assert!(matches!(
                    declaration.values[initializers[0].1.index as usize].operation,
                    w::ValueOperation::Boolean { value: true }
                ));
            });
        } else {
            inputs.with_proofs(
                TypeLimits::default(),
                proofs::ProofLimits::default(),
                |proofs, selected| {
                    discharged(proofs);
                    let report = native::admit(proofs, selected, Limits::default());
                    assert_eq!(
                        report.result().err(),
                        Some(&Error::Unsupported(Unsupported::FamilyProof))
                    );
                },
            );
        }
    }
}

#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-9")]
fn finite_boolean_valuation_work_exhausts_at_choice_and_fresh_retry_emits() {
    let mut events = String::new();
    let mut visible = Vec::new();
    for index in 0..12 {
        events.push_str(&format!(
            "send Sent{index} via Messages as (sent{index}: M::Plain) {{ true }};
             receive Got{index} via Messages of Sent{index} as (got{index}: M::Plain) {{ true }};"
        ));
        visible.push(format!("got{index}.ready"));
    }
    let decision = choice(
        "Receiver",
        &visible.join(","),
        "got0.ready",
        "not got0.ready",
    );
    let inputs = inputs(&format!("sequence Main {{ {events} {decision} }}"));
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let namespace = proofs.types().binding().namespace();
            let [id] = namespace.lookup("Decisions") else {
                panic!("one declaration")
            };
            let unit = namespace
                .unit(proofs.types().declaration(*id).unwrap().unit())
                .unwrap();
            let original = unit
                .controls()
                .iter()
                .find(|c| c.name.value == "Decide")
                .unwrap();
            // Twelve independent selected atoms have 4096 valuations. Merely
            // visiting the A and not-A formula nodes costs at least 3 per valuation
            // (12288), before case/carry work; 10000 cannot complete this check.
            let limited = native::admit(
                proofs,
                selected,
                Limits {
                    references: 10_000,
                    ..Limits::default()
                },
            );
            let Error::Incomplete(exhaustion) =
                limited.result().err().expect("finite valuation ceiling")
            else {
                panic!(
                    "expected resource refusal, found {:?}",
                    limited.result().err()
                )
            };
            assert_eq!(exhaustion.dimension, artifact::Dimension::References);
            assert_eq!(exhaustion.limit, 10_000);
            assert!(exhaustion.used <= exhaustion.limit);
            assert!(exhaustion.requested > exhaustion.limit - exhaustion.used);
            assert_eq!(limited.locus(), exhaustion.locus.as_ref());
            let locus = limited.locus().expect("choice-owned valuation work");
            assert_eq!(locus.source, 0);
            assert_eq!(
                (locus.span.start as usize, locus.span.end as usize),
                (original.span.start, original.span.end)
            );
            let retry = native::admit(proofs, selected, Limits::default());
            assert!(
                retry.result().is_ok(),
                "fresh default retry: {:?}; locus {:?}",
                retry.result().err(),
                retry.locus()
            );
            let retry = retry.into_result().unwrap();
            original_choices(proofs, retry.package());
            let emitted = native::emit(&retry, Limits::default())
                .into_result()
                .unwrap();
            let read = inputs.read(proofs, &emitted);
            assert!(
                read.result().is_ok(),
                "{:?}; locus {:?}",
                read.result().err(),
                read.locus()
            );
            assert_eq!(read.into_result().unwrap().package(), retry.package());
        },
    );
}
