// SPDX-License-Identifier: AGPL-3.0-only
//! Observed Boolean decision facts pass through actual compiler stages. These
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

const PROGRESSING_BODY: &str = "event Looped by Receiver as (looped: M::Plain) { true };";

const OWN_ATTEMPT: &str = "attempt Tried by Receiver on M::Node::step contracts []
    as (attempted: M::Plain) { true };";

#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-8")]
fn send_and_effect_records_never_become_choice_observations() {
    for (kind, owner, record, run) in [
        (
            "send",
            "Sender",
            "sent",
            "send Sent via Messages as (sent: M::Plain) { true };",
        ),
        (
            "effect",
            "Receiver",
            "applied",
            "attempt Tried by Receiver on M::Node::step contracts [] as (attempted: M::Plain) { true };
             effect Applied of Main::Tried as (applied: M::Plain) { true };",
        ),
    ] {
        let decision = choice(owner, &format!("{record}.ready"), &format!("{record}.ready"), &format!("not {record}.ready"));
        let inputs = inputs(&format!("sequence Main {{ {run} {decision} }}"));
        inputs.with_proofs(
            TypeLimits::default(),
            proofs::ProofLimits::default(),
            |proofs, selected| {
                discharged(proofs);
                assert_eq!(
                    native::admit(proofs, selected, Limits::default()).result().err(),
                    Some(&Error::Unsupported(Unsupported::FamilyProof)),
                    "{kind} record must not become a Boolean choice observation"
                );
            },
        );
    }
}

#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-8")]
fn own_attempt_non_boolean_field_comparisons_remain_unsupported() {
    let decision = choice(
        "Receiver",
        "attempted.signed >= 0",
        "attempted.signed >= 0",
        "attempted.signed < 0",
    );
    let inputs = inputs(&format!(
        "sequence Main {{
        attempt Tried by Receiver on M::Node::step contracts [] as (attempted: M::Node) {{ true }};
        {decision}
    }}"
    ));
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            assert_eq!(
                native::admit(proofs, selected, Limits::default())
                    .result()
                    .err(),
                Some(&Error::Unsupported(Unsupported::FamilyProof))
            );
        },
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-7", "FR-042-AC-9")]
fn own_attempt_reference_budget_exhaustion_preserves_original_locus_and_retries() {
    let mut attempts = String::new();
    let mut visible = Vec::new();
    for index in 0..12 {
        attempts.push_str(&format!("attempt Tried{index} by Receiver on M::Node::step contracts [] as (attempted{index}: M::Plain) {{ true }};"));
        visible.push(format!("attempted{index}.ready"));
    }
    let decision = choice(
        "Receiver",
        &visible.join(","),
        "attempted0.ready",
        "not attempted0.ready",
    );
    let inputs = inputs(&format!("sequence Main {{ {attempts} {decision} }}"));
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let namespace = proofs.types().binding().namespace();
            let [id] = namespace.lookup("Decisions") else {
                panic!("original declaration")
            };
            let unit = namespace
                .unit(proofs.types().declaration(*id).unwrap().unit())
                .unwrap();
            let original = unit
                .controls()
                .iter()
                .find(|node| node.name.value == "Decide")
                .unwrap();
            // 4096 valuations each visit at least the A/not-A guard nodes, so this
            // bound necessarily interrupts the real decision partition work.
            let limited = native::admit(
                proofs,
                selected,
                Limits {
                    references: 10_000,
                    ..Limits::default()
                },
            );
            let Err(Error::Incomplete(exhaustion)) = limited.result() else {
                panic!("{:?}", limited.result().err())
            };
            assert_eq!(exhaustion.dimension, artifact::Dimension::References);
            assert_eq!(exhaustion.limit, 10_000);
            assert!(exhaustion.used <= exhaustion.limit);
            assert!(exhaustion.requested > exhaustion.limit - exhaustion.used);
            let locus = limited
                .locus()
                .expect("original choice owns valuation exhaustion");
            assert_eq!(Some(locus), exhaustion.locus.as_ref());
            assert_eq!(locus.source, 0);
            assert_eq!(
                (locus.span.start as usize, locus.span.end as usize),
                (original.span.start, original.span.end)
            );

            let baseline = native::admit(proofs, selected, Limits::default());
            assert!(baseline.result().is_ok(), "{:?}", baseline.result().err());
            let required = baseline.usage().references;
            assert!(required > 10_000);
            let one_short = native::admit(
                proofs,
                selected,
                Limits {
                    references: required - 1,
                    ..Limits::default()
                },
            );
            let Err(Error::Incomplete(exhaustion)) = one_short.result() else {
                panic!("one-short reference budget must refuse")
            };
            assert_eq!(exhaustion.dimension, artifact::Dimension::References);
            assert_eq!(exhaustion.limit, required - 1);
            assert!(exhaustion.used <= exhaustion.limit);
            assert!(exhaustion.requested > exhaustion.limit - exhaustion.used);
            let exact = native::admit(
                proofs,
                selected,
                Limits {
                    references: required,
                    ..Limits::default()
                },
            );
            assert!(
                exact.result().is_ok(),
                "{:?}; {:?}",
                exact.result().err(),
                exact.locus()
            );
            assert_eq!(exact.usage().references, required);
            let exact = exact.into_result().unwrap();
            assert_eq!(exact.package(), baseline.result().unwrap().package());
            original_choices(proofs, exact.package());
            let emitted = native::emit(&exact, Limits::default())
                .into_result()
                .unwrap();
            let read = inputs.read(proofs, &emitted);
            assert!(
                read.result().is_ok(),
                "{:?}; {:?}",
                read.result().err(),
                read.locus()
            );
            assert_eq!(read.into_result().unwrap().package(), exact.package());
        },
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-1", "FR-042-AC-4", "FR-042-AC-5", "FR-042-AC-7")]
fn own_attempt_choice_preserves_cross_unit_contract_and_observation_owners() {
    let mut inputs = Inputs::new(&[
        Unit { name: "attempt-contracts", body: "pre Ready using S on M::Node::step { delta >= 0 }\npost Done using S on M::Node::step { result }", declarations: &["Ready", "Done"] },
        Unit { name: "attempt-choice", body: "protocol Decisions using P over (view: M::Node) on origin {
            role Receiver on M::Node;
            run sequence Main {
                attempt Tried by Receiver on M::Node::step contracts [Ready,Done] as (attempted: M::Plain) { true };
                choice Decide by Receiver visible (attempted.ready) {
                    case yes when { attempted.ready } check Accepted using S { true };
                    case no when { not attempted.ready } check Rejected using S { true };
                }
            }
            finish Closed as (closed: M::Node) { true };
        }", declarations: &["Decisions"] },
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
                "{:?}; {:?}",
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
            let ready = find("Ready");
            let done = find("Done");
            let owner = find("Decisions");
            for (id, source_index) in [(ready, 0), (done, 0), (owner, 1)] {
                let declaration = &package.declarations[id as usize];
                let source = &package.sources[declaration.locus.source as usize];
                assert_eq!(
                    source.native.identity,
                    inputs.sources[source_index].identity().identity
                );
                assert_eq!(source.text, inputs.sources[source_index].text());
                let namespace = proofs.types().binding().namespace();
                let [original] = namespace.lookup(&declaration.name) else {
                    panic!("one original owner")
                };
                let syntax = namespace.syntax(*original).unwrap();
                assert_eq!(
                    (
                        declaration.locus.span.start as usize,
                        declaration.locus.span.end as usize
                    ),
                    (syntax.span.start, syntax.span.end)
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
                package.dependencies[model.artifact as usize].artifact,
                inputs.model_reference
            );
            assert_eq!(
                model.exports[expected_operation.export as usize].path,
                ["Node", "step"]
            );
            let declaration = &package.declarations[owner as usize];
            assert!(declaration
                .bindings
                .iter()
                .all(|binding| binding.kind != w::BindingKind::Effect));
            let mut contracts = vec![ready, done];
            contracts.sort_unstable();
            assert_eq!(declaration.requires, contracts);
            let w::Body::Protocol {
                roles, controls, ..
            } = &declaration.body
            else {
                panic!("protocol")
            };
            assert!(controls.iter().all(|control| !matches!(
                control.operation,
                w::ControlOperation::Event {
                    event: w::Event::Effect { .. },
                    ..
                }
            )));
            let handle = |name| w::Handle {
                declaration: owner,
                index: controls.iter().position(|node| node.name == name).unwrap() as u32,
            };
            let tried = handle("Tried");
            let w::ControlOperation::Event {
                event:
                    w::Event::Attempt {
                        owner: attempt_owner,
                        operation,
                        contracts: retained,
                        ..
                    },
                binder,
                ..
            } = &controls[tried.index as usize].operation
            else {
                panic!("attempt")
            };
            assert_eq!(operation, &expected_operation);
            assert_eq!(retained, &contracts);
            let w::ControlOperation::Choice {
                owner: chooser,
                visible,
                cases,
            } = &controls[handle("Decide").index as usize].operation
            else {
                panic!("choice")
            };
            assert_eq!(chooser, attempt_owner);
            assert_eq!(chooser.declaration, owner);
            assert_eq!(roles[chooser.index as usize].name, "Receiver");
            assert_eq!(binder.declaration, owner);
            let observation = &declaration.binders[binder.index as usize];
            assert_eq!(observation.name, "attempted");
            assert_eq!(observation.kind, w::BinderKind::Event);
            assert_eq!(observation.anchor.declaration, owner);
            assert_eq!(
                declaration.anchors[observation.anchor.index as usize]
                    .owner
                    .0
                    .as_ref(),
                Some(&tried)
            );
            for value in [&visible[0], &cases[0].guard] {
                assert_eq!(value.declaration, owner);
                let value = &declaration.values[value.index as usize];
                let w::ValueOperation::Field { base, field } = &value.operation else {
                    panic!("attempt observation field")
                };
                assert_eq!(base.declaration, owner);
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
                assert_eq!(value.locus.source, declaration.locus.source);
                let text = &package.sources[value.locus.source as usize].text;
                assert_eq!(
                    &text[value.locus.span.start as usize..value.locus.span.end as usize],
                    "attempted.ready"
                );
            }
            let emitted = native::emit(&admission, Limits::default())
                .into_result()
                .unwrap();
            let read = inputs.read(proofs, &emitted);
            assert!(
                read.result().is_ok(),
                "{:?}; {:?}",
                read.result().err(),
                read.locus()
            );
            assert_eq!(read.into_result().unwrap().package(), package);
        },
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-8")]
fn foreign_role_cannot_use_an_attempt_even_with_the_same_model_type() {
    let decision = choice(
        "Sender",
        "attempted.ready",
        "attempted.ready",
        "not attempted.ready",
    );
    let inputs = inputs(&format!("sequence Main {{ {OWN_ATTEMPT} {decision} }}"));
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

#[test]
#[trace("TC-121", "FR-042-AC-1", "FR-042-AC-5", "FR-042-AC-7", "FR-042-AC-8")]
fn all_joined_own_attempts_keep_distinct_boolean_atoms() {
    for (yes, no, succeeds) in [
        (
            "left.ready and right.ready",
            "not (left.ready and right.ready)",
            true,
        ),
        ("left.ready", "not right.ready", false),
    ] {
        let decision = choice("Receiver", "left.ready,right.ready", yes, no);
        let inputs = inputs(&format!("sequence Main {{
            parallel Gather {{
                branch left attempt Left by Receiver on M::Node::step contracts [] as (left: M::Plain) {{ true }};
                branch right attempt Right by Receiver on M::Node::step contracts [] as (right: M::Plain) {{ true }};
            }} join all [left,right]; {decision}
        }}"));
        if succeeds {
            admitted(&inputs, |proofs, package| {
                original_choices(proofs, package);
                let declaration = &package.declarations[0];
                let w::Body::Protocol {
                    controls,
                    causal_edges,
                    ..
                } = &declaration.body
                else {
                    panic!("protocol")
                };
                let gather = w::Handle {
                    declaration: 0,
                    index: controls
                        .iter()
                        .position(|node| node.name == "Gather")
                        .unwrap() as u32,
                };
                let w::ControlOperation::Parallel { branches, join } =
                    &controls[gather.index as usize].operation
                else {
                    panic!("all join")
                };
                assert_eq!(join, &[0, 1]);
                let mut observations = Vec::new();
                for branch in branches {
                    let w::ControlOperation::Event {
                        event: w::Event::Attempt { .. },
                        binder,
                        ..
                    } = &controls[branch.body.index as usize].operation
                    else {
                        panic!("attempt")
                    };
                    let observation = &declaration.binders[binder.index as usize];
                    assert_eq!(
                        declaration.anchors[observation.anchor.index as usize]
                            .owner
                            .0
                            .as_ref(),
                        Some(&branch.body)
                    );
                    observations.push(observation);
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
                assert_eq!(observations.len(), 2);
                assert_eq!(observations[0].value_type, observations[1].value_type);
                assert_ne!(observations[0].anchor, observations[1].anchor);
            });
        } else {
            inputs.with_proofs(
                TypeLimits::default(),
                proofs::ProofLimits::default(),
                |proofs, selected| {
                    discharged(proofs);
                    assert_eq!(
                        native::admit(proofs, selected, Limits::default())
                            .result()
                            .err(),
                        Some(&Error::Unsupported(Unsupported::FamilyProof))
                    );
                },
            );
        }
    }
}

#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-8")]
// The linker/type checker owns this pre-existing lexical-flow refusal; attempt
// eligibility is reached only after the record is available at the choice.
fn own_attempt_parallel_sibling_preserves_inherited_scope_refusal() {
    let decision = choice(
        "Receiver",
        "attempted.ready",
        "attempted.ready",
        "not attempted.ready",
    );
    let inputs = inputs(&format!("parallel Both {{ branch performing {OWN_ATTEMPT} branch deciding {decision} }} join all [performing,deciding];"));
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            let namespace = proofs.types().binding().namespace();
            let [id] = namespace.lookup("Decisions") else {
                panic!("one declaration")
            };
            let scope = proofs
                .types()
                .binding()
                .scopes()
                .unwrap()
                .declaration(*id)
                .unwrap();
            let unavailable: Vec<_> = scope
                .issues
                .iter()
                .filter_map(|issue| match issue {
                    ScopeIssue::OutOfScope { name, span, .. } if name == "attempted" => Some(span),
                    _ => None,
                })
                .collect();
            assert_eq!(unavailable.len(), 3, "{:?}", scope.issues);
            for span in unavailable {
                assert_eq!(&inputs.sources[0].text()[span.start..span.end], "attempted");
            }
            assert_eq!(
                proofs.types().disposition(*id),
                Some(TypeDisposition::Refused)
            );
            assert_eq!(
                proofs.disposition(*id),
                Some(proofs::ProofDisposition::Refused)
            );
            assert_eq!(
                native::admit(proofs, selected, Limits::default())
                    .result()
                    .err(),
                Some(&Error::Unsupported(Unsupported::FamilyProof))
            );
        },
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-1", "FR-042-AC-3", "FR-042-AC-5", "FR-042-AC-7")]
fn received_object_boolean_field_uses_its_admitted_record_owner() {
    // Extend the fixture's original model source before admission and before
    // constructing any native source or selected model/dependency inventory.
    let original = Inputs::new(&[]);
    let model_source = original.model.source();
    let mut document: serde_json::Value =
        serde_json::from_str(model_source.source().text()).unwrap();
    let node = document["records"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|record| record["name"] == "Node")
        .unwrap();
    node["fields"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"name":"ready", "type":{"kind":"boolean"}}));
    let bytes = serde_json::to_vec(&document).unwrap();
    let source = quire_spec_language::Source::read(
        model_source.source().identity().clone(),
        model_source.source().path(),
        &bytes,
        quire_spec_language::Limits::default().source_bytes,
    )
    .unwrap();
    let formal = quire_spec_language::formal_source::FormalSource::new(
        source,
        model_source.identity().clone(),
    );
    let model = quire_spec_language::model_source::read(
        formal,
        quire_spec_language::model_source::FORMAT_V2,
        quire_spec_language::model_source::ModelSourceLimits::default(),
    )
    .unwrap()
    .admit(quire_spec_language::native_model::ModelLimits::default())
    .unwrap();
    let inputs = Inputs::with_model(&[Unit {
        name: "received-object", declarations: &["Decisions"],
        body: "protocol Decisions using P over (view: M::Node) on origin {
            role Sender on M::Node; role Receiver on M::Node;
            channel Messages from Sender to Receiver carries M::Node ordering unordered delivery [1,1];
            run sequence Main {
                send Sent via Messages as (sent: M::Node) { true };
                receive Got via Messages of Sent as (got: M::Node) { true };
                choice Decide by Receiver visible (got.ready) {
                    case yes when { got.ready } check Accepted using S { true };
                    case no when { not got.ready } check Rejected using S { true };
                }
            }
            finish Closed as (closed: M::Node) { true };
        }",
    }], model);
    admitted(&inputs, |proofs, package| {
        original_choices(proofs, package);
        let declaration = &package.declarations[0];
        let binder = declaration
            .binders
            .iter()
            .find(|binder| binder.name == "got")
            .unwrap();
        let w::Type::Object { export } = &package.types[binder.value_type as usize] else {
            panic!("received binder must exercise Object, not Record")
        };
        let selected_model = &package.models[export.model as usize];
        assert_eq!(
            selected_model.exports[export.export as usize].path,
            ["Node"]
        );
        assert_eq!(
            package.dependencies[selected_model.artifact as usize].artifact,
            inputs.model_reference
        );
        let fields: Vec<_> = declaration
            .values
            .iter()
            .filter_map(|value| match &value.operation {
                w::ValueOperation::Field { field, .. } => Some(field),
                _ => None,
            })
            .collect();
        assert_eq!(fields.len(), 3);
        for field in fields {
            assert_eq!(field.model, export.model);
            assert_eq!(
                package.models[field.model as usize].exports[field.export as usize].path,
                ["Node", "ready"]
            );
        }
    });
}

#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-7", "FR-042-AC-9")]
fn multiple_received_choices_preserve_entry_budget_boundaries() {
    let first = choice("Receiver", "gotA.ready", "gotA.ready", "not gotA.ready");
    let second = "choice Later by Receiver visible (gotB.ready) {
        case yes when { gotB.ready } check LaterYes using S { true };
        case no when { not gotB.ready } check LaterNo using S { true };
    }";
    let inputs = inputs(&format!(
        "sequence Main {{ {RECEIVE_A} {RECEIVE_B} {first} {second} }}"
    ));
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let baseline = native::admit(proofs, selected, Limits::default());
            assert!(baseline.result().is_ok(), "{:?}", baseline.result().err());
            let entries = baseline.usage().entries;
            assert!(entries > 0);
            let limited = native::admit(
                proofs,
                selected,
                Limits {
                    entries: entries - 1,
                    ..Limits::default()
                },
            );
            let Err(Error::Incomplete(exhaustion)) = limited.result() else {
                panic!("one-short Entries must refuse")
            };
            assert_eq!(exhaustion.dimension, artifact::Dimension::Entries);
            assert_eq!(exhaustion.limit, entries - 1);
            assert!(exhaustion.used <= exhaustion.limit);
            assert!(exhaustion.requested > exhaustion.limit - exhaustion.used);
            let exact = native::admit(
                proofs,
                selected,
                Limits {
                    entries,
                    ..Limits::default()
                },
            );
            assert!(
                exact.result().is_ok(),
                "{:?}; {:?}",
                exact.result().err(),
                exact.locus()
            );
            assert_eq!(exact.usage().entries, entries);
            let exact = exact.into_result().unwrap();
            assert_eq!(exact.package(), baseline.result().unwrap().package());
            original_choices(proofs, exact.package());
            let w::Body::Protocol { controls, .. } = &exact.package().declarations[0].body else {
                panic!("protocol")
            };
            assert_eq!(
                controls
                    .iter()
                    .filter(|node| matches!(node.operation, w::ControlOperation::Choice { .. }))
                    .count(),
                2
            );
            let emitted = native::emit(&exact, Limits::default())
                .into_result()
                .unwrap();
            let read = inputs.read(proofs, &emitted);
            assert!(
                read.result().is_ok(),
                "{:?}; {:?}",
                read.result().err(),
                read.locus()
            );
            assert_eq!(read.into_result().unwrap().package(), exact.package());
        },
    );
}

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

fn repeat(owner: &str, visible: &str, maximum: u8, guard: &str, body: &str) -> String {
    format!(
        "repeat Loop by {owner} visible ({visible}) max {maximum} while {{ {guard} }} {body}
          exhausted check Limit using S {{ true }};"
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
#[trace("TC-121", "FR-042-AC-1", "FR-042-AC-4", "FR-042-AC-5", "FR-042-AC-7")]
fn received_choice_with_cross_unit_operation_contracts_retains_original_owners() {
    let mut inputs = Inputs::new(&[
        Unit {
            name: "decision-contracts",
            body: "pre Ready using S on M::Node::step { delta >= 0 }\npost Done using S on M::Node::step { result }",
            declarations: &["Ready", "Done"],
        },
        Unit {
            name: "decision-operation",
            body: "protocol Decisions using P over (view: M::Node) on origin {
                role Sender on M::Node;
                role Receiver on M::Node;
                channel Messages from Sender to Receiver carries M::Plain
                    ordering unordered delivery [1,1];
                run sequence Main {
                    send Sent via Messages as (sent: M::Plain) { true };
                    receive Got via Messages of Sent as (got: M::Plain) { true };
                    choice Decide by Receiver visible (got.ready) {
                        case yes when { got.ready }
                            attempt Tried by Receiver on M::Node::step contracts [Ready,Done]
                                as (attempted: M::Plain) { attempted.ready };
                        case no when { not got.ready } check Skipped using S { true };
                    }
                }
                finish Closed as (closed: M::Node) { true };
            }",
            declarations: &["Decisions"],
        },
    ]);
    let expected_operation = inputs.step_contracts("Ready", "Done");
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let admission = native::admit(proofs, selected, Limits::default());
            assert!(
                admission.result().is_ok(),
                "{:?}; {:?}",
                admission.result().err(),
                admission.locus()
            );
            let admission = admission.into_result().unwrap();
            let package = admission.package();
            assert_eq!((package.sources.len(), package.declarations.len()), (2, 3));
            for source in &package.sources {
                let original = inputs
                    .sources
                    .iter()
                    .find(|s| s.identity().identity == source.native.identity)
                    .unwrap();
                assert_eq!(source.native.revision, original.identity().revision);
                assert_eq!(source.text, original.text());
            }
            let find = |name| {
                package
                    .declarations
                    .iter()
                    .position(|d| d.name == name)
                    .unwrap() as u32
            };
            let ready = find("Ready");
            let done = find("Done");
            let decisions = find("Decisions");
            let declaration = &package.declarations[decisions as usize];
            let namespace = proofs.types().binding().namespace();
            for (name, index, source_index) in [
                ("Ready", ready, 0),
                ("Done", done, 0),
                ("Decisions", decisions, 1),
            ] {
                let retained = &package.declarations[index as usize];
                let [id] = namespace.lookup(name) else {
                    panic!("one authored declaration")
                };
                let original = namespace.syntax(*id).unwrap();
                assert_eq!(
                    (
                        retained.locus.span.start as usize,
                        retained.locus.span.end as usize
                    ),
                    (original.span.start, original.span.end)
                );
                assert_eq!(
                    package.sources[retained.locus.source as usize]
                        .native
                        .identity,
                    inputs.sources[source_index].identity().identity
                );
            }
            assert_ne!(
                declaration.locus.source,
                package.declarations[ready as usize].locus.source
            );
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
            assert_eq!(
                package.models[expected_operation.model as usize].exports
                    [expected_operation.export as usize]
                    .path,
                ["Node", "step"]
            );
            assert_eq!(
                package.dependencies
                    [package.models[expected_operation.model as usize].artifact as usize]
                    .artifact,
                inputs.model_reference
            );
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
            let decide = handle("Decide");
            let tried = handle("Tried");
            let got = handle("Got");
            let [original_id] = namespace.lookup("Decisions") else {
                panic!("one original protocol")
            };
            let original_unit = namespace
                .unit(proofs.types().declaration(*original_id).unwrap().unit())
                .unwrap();
            let original_choice =
                &original_unit.controls()[controls[decide.index as usize].original_node as usize];
            assert_eq!(original_choice.name.value, "Decide");
            assert!(matches!(
                original_choice.kind,
                c::ControlKind::Choice { .. }
            ));
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
            let w::ControlOperation::Choice {
                owner,
                visible,
                cases,
            } = &controls[decide.index as usize].operation
            else {
                panic!("received choice")
            };
            assert_eq!(
                *owner,
                w::Handle {
                    declaration: decisions,
                    index: 1
                }
            );
            assert_eq!(roles[owner.index as usize].name, "Receiver");
            assert_eq!(owner, &channels[0].to);
            assert_eq!(
                cases.iter().map(|c| c.label.as_str()).collect::<Vec<_>>(),
                ["yes", "no"]
            );
            assert_eq!(cases[0].body, tried);
            assert_eq!(cases[1].body, handle("Skipped"));
            let w::ControlOperation::Event {
                event: w::Event::Receive { channel, .. },
                binder: received,
                ..
            } = &controls[got.index as usize].operation
            else {
                panic!("actual receive")
            };
            assert_eq!(
                *channel,
                w::Handle {
                    declaration: decisions,
                    index: 0
                }
            );
            assert_eq!(received.declaration, decisions);
            for value in [&visible[0], &cases[0].guard] {
                assert_eq!(value.declaration, decisions);
                let retained = &declaration.values[value.index as usize];
                let w::ValueOperation::Field { base, field } = &retained.operation else {
                    panic!("dynamic received field")
                };
                assert_eq!(base.declaration, decisions);
                assert_eq!(
                    declaration.values[base.index as usize].operation,
                    w::ValueOperation::Read {
                        binder: received.clone()
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
                    "got.ready"
                );
            }
            let w::ControlOperation::Event {
                event:
                    w::Event::Attempt {
                        owner: attempt_owner,
                        operation,
                        contracts: actual_contracts,
                        ..
                    },
                ..
            } = &controls[tried.index as usize].operation
            else {
                panic!("actual operation attempt")
            };
            assert_eq!(attempt_owner, owner);
            assert_eq!(operation, &expected_operation);
            assert_eq!(actual_contracts, &contracts);
            for case in cases {
                assert_eq!(case.guard.declaration, decisions);
                assert!(causal_edges.iter().any(|edge| edge.owner == decide
                    && edge.kind == w::EdgeKind::Branch
                    && edge.from.node == decide
                    && edge.to.node == case.body));
                assert!(causal_edges.iter().any(|edge| edge.owner == decide
                    && edge.kind == w::EdgeKind::Join
                    && edge.from.node == case.body
                    && edge.to.node == decide));
            }
            let emitted = native::emit(&admission, Limits::default())
                .into_result()
                .unwrap();
            let read = inputs.read(proofs, &emitted);
            assert!(
                read.result().is_ok(),
                "{:?}; {:?}",
                read.result().err(),
                read.locus()
            );
            assert_eq!(read.into_result().unwrap().package(), package);
        },
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-8")]
fn closed_choice_classification_requires_every_authored_operand_to_be_closed() {
    for (yes, no, expected) in [
        (
            "false and view.plain.ready",
            "true",
            Error::Unsupported(Unsupported::FamilyProof),
        ),
        (
            "true or view.plain.ready",
            "false",
            Error::Unsupported(Unsupported::FamilyProof),
        ),
        (
            "false implies view.plain.ready",
            "false",
            Error::Unsupported(Unsupported::FamilyProof),
        ),
        (
            "if true then true else view.plain.ready",
            "false",
            Error::Unsupported(Unsupported::FamilyProof),
        ),
        (
            "let decisionConstant = true in decisionConstant",
            "true",
            Error::Invalid(Invalid::Control),
        ),
        (
            "let decisionConstant = false in decisionConstant",
            "false",
            Error::Invalid(Invalid::Control),
        ),
    ] {
        let decision = choice("Receiver", "true", yes, no);
        let inputs = inputs(&decision);
        inputs.with_proofs(
            TypeLimits::default(),
            proofs::ProofLimits::default(),
            |proofs, selected| {
                discharged(proofs);
                let admission = native::admit(proofs, selected, Limits::default());
                assert_eq!(
                    admission.result().err(),
                    Some(&expected),
                    "guard {yes}; {:?}",
                    admission.locus()
                );
            },
        );
    }
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
            "composite result does not disclose its individual operands",
            "Receiver",
            "gotA.ready and gotB.ready",
            "gotA.ready",
            "not gotA.ready",
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
            true,
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
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-8")]
fn an_observed_repeat_guard_requires_owned_listed_and_individual_atoms() {
    for (case, received, owner, visible, guard) in [
        (
            "foreign observing role",
            RECEIVE_A.to_owned(),
            "Sender",
            "gotA.ready",
            "gotA.ready",
        ),
        (
            "unlisted guard atom",
            RECEIVE_A.to_owned(),
            "Receiver",
            "true",
            "gotA.ready",
        ),
        (
            "composite visible entry",
            format!("{RECEIVE_A} {RECEIVE_B}"),
            "Receiver",
            "gotA.ready and gotB.ready",
            "gotA.ready",
        ),
    ] {
        let loop_ = repeat(owner, visible, 2, guard, PROGRESSING_BODY);
        let inputs = inputs(&format!("sequence Main {{ {received} {loop_} }}"));
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

#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-8")]
fn a_repeat_guard_atom_established_inside_the_body_has_no_decision_availability() {
    let loop_ = repeat(
        "Receiver",
        "gotA.ready",
        2,
        "gotA.ready",
        &format!("sequence Iteration {{ {RECEIVE_A} {PROGRESSING_BODY} }}"),
    );
    let inputs = inputs(&format!("sequence Main {{ {loop_} }}"));
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            let namespace = proofs.types().binding().namespace();
            let [id] = namespace.lookup("Decisions") else {
                panic!("one declaration")
            };
            let scope = proofs
                .types()
                .binding()
                .scopes()
                .unwrap()
                .declaration(*id)
                .unwrap();
            let unavailable: Vec<_> = scope
                .issues
                .iter()
                .filter_map(|issue| match issue {
                    ScopeIssue::OutOfScope { name, span, .. } if name == "gotA" => Some(span),
                    _ => None,
                })
                .collect();
            assert_eq!(
                unavailable.len(),
                2,
                "the visible entry and the guard both refuse; issues {:?}",
                scope.issues
            );
            for span in unavailable {
                assert_eq!(&inputs.sources[0].text()[span.start..span.end], "gotA");
            }
            assert_eq!(
                proofs.types().disposition(*id),
                Some(TypeDisposition::Refused)
            );
            let report = native::admit(proofs, selected, Limits::default());
            assert_eq!(
                report.result().err(),
                Some(&Error::Unsupported(Unsupported::FamilyProof)),
                "locus {:?}",
                report.locus()
            );
        },
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-7", "FR-042-AC-8")]
fn an_observed_repeat_guard_carries_the_true_constant_guard_body_obligation() {
    for (case, body, expected) in [
        (
            "continuing body proves no progress",
            "check Stalled using S { true };".to_owned(),
            Error::Invalid(Invalid::Control),
        ),
        (
            "continuing body progress stays unproved",
            format!(
                "sequence Iteration {{
                   choice Decide by Receiver visible (gotA.ready) {{
                     case yes when {{ gotA.ready }} {PROGRESSING_BODY}
                     case no when {{ not gotA.ready }} check Rejected using S {{ true }};
                   }}
                 }}"
            ),
            Error::Unsupported(Unsupported::FamilyProof),
        ),
    ] {
        let loop_ = repeat("Receiver", "gotA.ready", 2, "gotA.ready", &body);
        let inputs = inputs(&format!("sequence Main {{ {RECEIVE_A} {loop_} }}"));
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
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-7", "FR-042-AC-9")]
fn a_zero_maximum_observed_repeat_keeps_its_authored_bound_without_a_body_obligation() {
    let loop_ = repeat(
        "Receiver",
        "gotA.ready",
        0,
        "gotA.ready",
        "check Stalled using S { true };",
    );
    let inputs = inputs(&format!("sequence Main {{ {RECEIVE_A} {loop_} }}"));
    admitted(&inputs, |_proofs, package| {
        let w::Body::Protocol { controls, .. } = &package.declarations[0].body else {
            panic!("protocol")
        };
        let repeat = controls.iter().find(|c| c.name == "Loop").unwrap();
        let w::ControlOperation::Repeat {
            maximum,
            visible,
            guard,
            ..
        } = &repeat.operation
        else {
            panic!("repeat")
        };
        let artifact::ProtocolNumber::Integer(maximum) = maximum.checked().unwrap() else {
            panic!("authored integer")
        };
        assert_eq!(maximum.value(), 0);
        // The observed guard and its visible entry stay retained expressions,
        // never a proof witness or a simplified constant.
        assert_eq!(visible.len(), 1);
        assert!(matches!(
            package.declarations[0].values[guard.index as usize].operation,
            w::ValueOperation::Field { .. }
        ));
    });
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
            let Err(Error::Incomplete(exhaustion)) = limited.result() else {
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
