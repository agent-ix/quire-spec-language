// SPDX-License-Identifier: AGPL-3.0-only
//! Domain event records reach Boolean choices only through the actual public
//! compiler pipeline, and only for the role that authored the event. These
//! tests establish static admission boundaries and measured budgets, not
//! runtime choices.

#[path = "support/native_protocol/mod.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_spec_language::checking::composed::{proofs, TypeDisposition, TypeLimits};
use quire_spec_language::protocol_artifact::{
    self as artifact, native, wire as w, Error, Limits, Unsupported,
};
use quire_spec_language::syntax::composed as c;
use setup::{Inputs, Unit};

const OBSERVE_A: &str = "event ObservedA by Receiver as (observedA: M::Plain) { true };";
const OBSERVE_B: &str = "event ObservedB by Receiver as (observedB: M::Plain) { true };";

#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-7", "FR-042-AC-9")]
fn domain_event_valuation_exhaustion_keeps_the_original_decide_locus_and_retries() {
    let mut events = String::new();
    let mut visible = Vec::new();
    for index in 0..12 {
        events.push_str(&format!(
            "event Observed{index} by Receiver as (observed{index}: M::Plain) {{ true }};"
        ));
        visible.push(format!("observed{index}.ready"));
    }
    let decision = choice(
        "Receiver",
        &visible.join(","),
        "observed0.ready",
        "not observed0.ready",
    );
    let inputs = inputs(&format!("sequence Main {{ {events} {decision} }}"));
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let namespace = proofs.types().binding().namespace();
            let [id] = namespace.lookup("Decisions") else {
                panic!("one original declaration")
            };
            let unit = namespace
                .unit(proofs.types().declaration(*id).unwrap().unit())
                .unwrap();
            let original = unit
                .controls()
                .iter()
                .find(|node| node.name.value == "Decide")
                .unwrap();
            // Twelve independent owned event atoms have 4096 valuations, and
            // merely visiting the A / not-A formula nodes costs at least 3 per
            // valuation (12288); 10000 necessarily interrupts partition work.
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
            let locus = limited
                .locus()
                .expect("the original choice owns valuation exhaustion");
            assert_eq!(Some(locus), exhaustion.locus.as_ref());
            assert_eq!(locus.source, 0);
            assert_eq!(
                (locus.span.start as usize, locus.span.end as usize),
                (original.span.start, original.span.end)
            );

            // A fresh default admission after exhaustion carries no residue.
            let retry = native::admit(proofs, selected, Limits::default());
            assert!(
                retry.result().is_ok(),
                "fresh default retry: {:?}; locus {:?}",
                retry.result().err(),
                retry.locus()
            );
            let required = retry.usage().references;
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
            assert_eq!(exact.package(), retry.result().unwrap().package());
            assert_eq!(authored_choices(proofs, exact.package()), 1);

            let retry = retry.into_result().unwrap();
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
            assert_eq!(read.into_result().unwrap().package(), exact.package());
        },
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-7", "FR-042-AC-9")]
fn two_domain_event_choices_preserve_measured_entry_budget_boundaries() {
    let first = choice(
        "Receiver",
        "observedA.ready",
        "observedA.ready",
        "not observedA.ready",
    );
    let second = "choice Later by Receiver visible (observedB.ready) {
        case yes when { observedB.ready } check LaterYes using S { true };
        case no when { not observedB.ready } check LaterNo using S { true };
    }";
    let inputs = inputs(&format!(
        "sequence Main {{ {OBSERVE_A} {OBSERVE_B} {first} {second} }}"
    ));
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let baseline = native::admit(proofs, selected, Limits::default());
            assert!(
                baseline.result().is_ok(),
                "{:?}; locus {:?}",
                baseline.result().err(),
                baseline.locus()
            );
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
            assert_eq!(authored_choices(proofs, exact.package()), 2);

            let declaration = &exact.package().declarations[0];
            let w::Body::Protocol {
                controls, roles, ..
            } = &declaration.body
            else {
                panic!("protocol")
            };
            let owners: Vec<_> = ["ObservedA", "ObservedB"]
                .into_iter()
                .map(|name| {
                    let control = controls.iter().find(|node| node.name == name).unwrap();
                    let w::ControlOperation::Event {
                        event: w::Event::Event { owner, .. },
                        binder,
                        ..
                    } = &control.operation
                    else {
                        panic!("actual domain event record")
                    };
                    assert_eq!(roles[owner.index as usize].name, "Receiver");
                    (owner.clone(), &declaration.binders[binder.index as usize])
                })
                .collect();
            assert_eq!(owners[0].1.value_type, owners[1].1.value_type);
            assert_ne!(
                owners[0].1.anchor, owners[1].1.anchor,
                "the same model field at two events keeps two facts"
            );
            for choice_owner in controls.iter().filter_map(|node| match &node.operation {
                w::ControlOperation::Choice { owner, .. } => Some(owner),
                _ => None,
            }) {
                assert_eq!(choice_owner, &owners[0].0);
            }

            let emitted = native::emit(&exact, Limits::default())
                .into_result()
                .unwrap();
            let read = inputs.read(proofs, &emitted);
            assert!(
                read.result().is_ok(),
                "{:?}; locus {:?}",
                read.result().err(),
                read.locus()
            );
            assert_eq!(read.into_result().unwrap().package(), exact.package());
        },
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-5")]
fn foreign_role_cannot_use_a_domain_event_even_with_the_same_model_type() {
    let decision = choice(
        "Sender",
        "observedA.ready",
        "observedA.ready",
        "not observedA.ready",
    );
    let inputs = inputs(&format!("sequence Main {{ {OBSERVE_A} {decision} }}"));
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
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
#[trace("TC-121", "FR-042-AC-5")]
fn omitted_and_composite_visible_domain_event_bases_do_not_grant_admission() {
    for (case, visible, yes, no) in [
        (
            "omitted advertised atom",
            "observedA.ready",
            "observedA.ready and observedB.ready",
            "not (observedA.ready and observedB.ready)",
        ),
        (
            "composite result does not disclose its individual operands",
            "observedA.ready and observedB.ready",
            "observedA.ready",
            "not observedA.ready",
        ),
    ] {
        let decision = choice("Receiver", visible, yes, no);
        let inputs = inputs(&format!(
            "sequence Main {{ {OBSERVE_A} {OBSERVE_B} {decision} }}"
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
        name: "domain-event-decisions",
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

/// Every retained choice keeps its authored name, span, advertised arity and
/// case labels from the original unit; returns how many were checked.
fn authored_choices(proofs: &proofs::ProofReport<'_, '_, '_>, package: &w::Package) -> usize {
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
        assert_eq!(control.locus.source, declaration.locus.source);
        assert_eq!(
            (
                control.locus.span.start as usize,
                control.locus.span.end as usize
            ),
            (original.span.start, original.span.end)
        );
        assert_eq!(visible.len(), original_visible.len());
        for (retained, original) in visible.iter().zip(original_visible) {
            assert_eq!(retained.declaration, 0);
            let expected = unit.expression(*original).unwrap();
            let value = &declaration.values[retained.index as usize];
            assert_eq!(
                &unit.expressions()[value.original_expression as usize],
                expected
            );
            assert_eq!(
                (
                    value.locus.span.start as usize,
                    value.locus.span.end as usize
                ),
                (expected.span.start, expected.span.end)
            );
        }
        assert_eq!(cases.len(), original_cases.len());
        let owner = w::Handle {
            declaration: 0,
            index: index as u32,
        };
        for (retained, original) in cases.iter().zip(original_cases) {
            assert_eq!(retained.label, original.name.value);
            assert_eq!(retained.guard.declaration, 0);
            assert_eq!(
                &unit.expressions()[declaration.values[retained.guard.index as usize]
                    .original_expression as usize],
                unit.expression(original.guard).unwrap()
            );
            let body = &controls[retained.body.index as usize];
            assert_eq!(
                &unit.controls()[body.original_node as usize],
                unit.control(original.control).unwrap()
            );
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
    found
}
