// SPDX-License-Identifier: AGPL-3.0-only
//! FR-042: a composite visible formula discloses its Boolean output while
//! retaining every original operand's authorization and observation identity.

#[path = "support/native_protocol/mod.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_spec_language::checking::composed::{proofs, TypeDisposition, TypeLimits};
use quire_spec_language::protocol_artifact::{
    native, wire as w, Dimension, Error, Limits, Unsupported,
};
use quire_spec_language::syntax::composed as c;
use setup::{Inputs, Unit};

fn inputs(visible: &str, yes: &str, no: &str, count: usize) -> Inputs {
    let events: String = (0..count)
        .map(|index| {
            format!("event Observed{index} by Receiver as (a{index}: M::Plain) {{ true }};")
        })
        .collect();
    let body = format!("protocol Decisions using P over (view: M::Node) on origin {{
        role Sender on M::Node; role Receiver on M::Node;
        run sequence Main {{
            attempt Tried by Receiver on M::Node::step contracts [Ready,Done] as (attempted: M::Plain) {{ true }};
            event Hidden by Sender as (hidden: M::Plain) {{ true }};
            {events}
            choice Decide by Receiver visible ({visible}) {{
                case yes when {{ {yes} }} check Accepted using S {{ true }};
                case no when {{ {no} }} check Rejected using S {{ true }};
            }}
        }}
        finish Closed as (closed: M::Node) {{ true }};
    }}");
    let mut inputs = Inputs::new(&[
        Unit { name: "composite-contracts", body: "pre Ready using S on M::Node::step { delta >= 0 } post Done using S on M::Node::step { result }", declarations: &["Ready", "Done"] },
        Unit { name: "composite-choice", body: &body, declarations: &["Decisions"] },
    ]);
    inputs.step_contracts("Ready", "Done");
    inputs
}

fn discharged(proofs: &proofs::ProofReport<'_, '_, '_>) {
    assert!(proofs.exhaustion().is_none());
    for declaration in proofs.declarations() {
        assert_eq!(
            proofs.types().disposition(declaration.declaration()),
            Some(TypeDisposition::Typed)
        );
        assert_eq!(
            declaration.disposition(),
            proofs::ProofDisposition::Discharged,
            "{:?}",
            declaration.causes()
        );
    }
}

fn original_roots(proofs: &proofs::ProofReport<'_, '_, '_>, package: &w::Package) {
    let owner = package
        .declarations
        .iter()
        .position(|d| d.name == "Decisions")
        .unwrap();
    let declaration = &package.declarations[owner];
    let namespace = proofs.types().binding().namespace();
    let [id] = namespace.lookup("Decisions") else {
        panic!("one original declaration")
    };
    let unit = namespace
        .unit(proofs.types().declaration(*id).unwrap().unit())
        .unwrap();
    assert_eq!(
        package.sources[declaration.locus.source as usize].text,
        unit.source().text()
    );
    let w::Body::Protocol { controls, .. } = &declaration.body else {
        panic!("protocol")
    };
    let control = controls.iter().find(|node| node.name == "Decide").unwrap();
    let original = &unit.controls()[control.original_node as usize];
    let c::ControlKind::Choice {
        visible: original_visible,
        cases: original_cases,
        ..
    } = &original.kind
    else {
        panic!("original choice")
    };
    let w::ControlOperation::Choice { visible, cases, .. } = &control.operation else {
        panic!("retained choice")
    };
    assert_eq!(visible.len(), original_visible.len());
    assert_eq!(cases.len(), original_cases.len());
    for (retained, authored) in visible.iter().zip(original_visible).chain(
        cases
            .iter()
            .zip(original_cases)
            .map(|(retained, authored)| (&retained.guard, &authored.guard)),
    ) {
        assert_eq!(retained.declaration as usize, owner);
        let value = &declaration.values[retained.index as usize];
        let expression = unit.expression(*authored).unwrap();
        assert_eq!(
            &unit.expressions()[value.original_expression as usize],
            expression
        );
        assert_eq!(value.locus.source, declaration.locus.source);
        assert_eq!(
            (
                value.locus.span.start as usize,
                value.locus.span.end as usize
            ),
            (expression.span.start, expression.span.end)
        );
    }
    for (index, node) in controls.iter().enumerate() {
        let w::ControlOperation::Event { binder, .. } = &node.operation else {
            continue;
        };
        let record = &declaration.binders[binder.index as usize];
        assert_eq!(binder.declaration as usize, owner);
        assert_eq!(record.anchor.declaration as usize, owner);
        assert_eq!(
            declaration.anchors[record.anchor.index as usize]
                .owner
                .0
                .as_ref(),
            Some(&w::Handle {
                declaration: owner as u32,
                index: index as u32
            })
        );
    }
}

fn round_trip(
    inputs: &Inputs,
    proofs: &proofs::ProofReport<'_, '_, '_>,
    admission: &native::FamilyAdmission,
) {
    original_roots(proofs, admission.package());
    let emitted = native::emit(admission, Limits::default())
        .into_result()
        .unwrap();
    let read = inputs.read(proofs, &emitted);
    assert!(
        read.result().is_ok(),
        "{:?}; {:?}",
        read.result().err(),
        read.locus()
    );
    assert_eq!(read.into_result().unwrap().package(), admission.package());
}

#[test]
#[trace("TC-121", "FR-042-AC-4", "FR-042-AC-5", "FR-042-AC-7")]
fn composite_outputs_determine_complementary_cases_without_rewriting_source_roots() {
    for (visible, yes, no) in [
        (
            "a0.ready and a1.ready",
            "a0.ready and a1.ready",
            "not (a0.ready and a1.ready)",
        ),
        ("not a0.ready", "not a0.ready", "a0.ready"),
        (
            "a0.ready and a1.ready, a0.ready and a1.ready",
            "a0.ready and a1.ready",
            "not (a0.ready and a1.ready)",
        ),
        (
            "let saved = a0.ready in if saved then a1.ready else false",
            "a0.ready and a1.ready",
            "not (a0.ready and a1.ready)",
        ),
        (
            "let discarded = a1.ready in a0.ready",
            "a0.ready",
            "not a0.ready",
        ),
        (
            "a0.ready and a1.ready, a0.ready or a1.ready",
            "a0.ready = a1.ready",
            "a0.ready != a1.ready",
        ),
    ] {
        let inputs = inputs(visible, yes, no, 2);
        inputs.with_proofs(
            TypeLimits::default(),
            proofs::ProofLimits::default(),
            |proofs, selected| {
                discharged(proofs);
                let report = native::admit(proofs, selected, Limits::default());
                assert!(
                    report.result().is_ok(),
                    "visible {visible}: {:?}; {:?}",
                    report.result().err(),
                    report.locus()
                );
                round_trip(&inputs, proofs, &report.into_result().unwrap());
            },
        );
    }
}

#[test]
#[trace("TC-121", "FR-042-AC-5")]
fn composite_disclosure_never_exposes_hidden_or_indistinguishable_leaf_values() {
    for (case, visible, yes, no) in [
        (
            "conjunction hides individual operands",
            "a0.ready and a1.ready",
            "a0.ready",
            "not a0.ready",
        ),
        (
            "tautology hides its operand",
            "a0.ready or not a0.ready",
            "a0.ready",
            "not a0.ready",
        ),
        (
            "duplicate roots add no information",
            "a0.ready and a1.ready, a0.ready and a1.ready",
            "a0.ready",
            "not a0.ready",
        ),
        (
            "unused initializer discloses no value",
            "let discarded = a1.ready in a0.ready",
            "a1.ready",
            "not a1.ready",
        ),
        ("missing guard atom", "a0.ready", "a1.ready", "not a1.ready"),
        (
            "foreign leaf",
            "a0.ready and hidden.ready",
            "a0.ready and hidden.ready",
            "not (a0.ready and hidden.ready)",
        ),
        (
            "workflow input leaf",
            "a0.ready and view.plain.ready",
            "a0.ready and view.plain.ready",
            "not (a0.ready and view.plain.ready)",
        ),
        (
            "unused hidden initializer",
            "let discarded = hidden.ready in a0.ready",
            "a0.ready",
            "not a0.ready",
        ),
        (
            "unselected hidden operand",
            "if true then a0.ready else hidden.ready",
            "a0.ready",
            "not a0.ready",
        ),
        (
            "guard unused hidden initializer",
            "a0.ready",
            "let discarded = hidden.ready in a0.ready",
            "not a0.ready",
        ),
    ] {
        let inputs = inputs(visible, yes, no, 2);
        inputs.with_proofs(
            TypeLimits::default(),
            proofs::ProofLimits::default(),
            |proofs, selected| {
                discharged(proofs);
                assert_eq!(
                    native::admit(proofs, selected, Limits::default())
                        .result()
                        .err(),
                    Some(&Error::Unsupported(Unsupported::FamilyProof)),
                    "{case}"
                );
            },
        );
    }
}

#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-7", "FR-042-AC-9")]
fn composite_disclosure_budget_exhaustion_keeps_choice_locus_and_fresh_retry() {
    let formula = (0..12)
        .map(|i| format!("a{i}.ready"))
        .collect::<Vec<_>>()
        .join(" and ");
    let inputs = inputs(&formula, &formula, &format!("not ({formula})"), 12);
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
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
            assert_eq!(exhaustion.dimension, Dimension::References);
            assert_eq!(exhaustion.limit, 10_000);
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
                .find(|node| node.name.value == "Decide")
                .unwrap();
            let locus = limited.locus().unwrap();
            assert_eq!(locus, exhaustion.locus.as_ref().unwrap());
            assert_eq!(
                (locus.span.start as usize, locus.span.end as usize),
                (original.span.start, original.span.end)
            );
            let baseline = native::admit(proofs, selected, Limits::default());
            assert!(baseline.result().is_ok(), "{:?}", baseline.result().err());
            assert_eq!(
                baseline.result().unwrap().package().sources[locus.source as usize]
                    .native
                    .identity,
                unit.source().identity().identity
            );
            for dimension in [Dimension::References, Dimension::Entries] {
                let used = match dimension {
                    Dimension::References => baseline.usage().references,
                    Dimension::Entries => baseline.usage().entries,
                    _ => unreachable!(),
                };
                let limit = |amount| match dimension {
                    Dimension::References => Limits {
                        references: amount,
                        ..Limits::default()
                    },
                    Dimension::Entries => Limits {
                        entries: amount,
                        ..Limits::default()
                    },
                    _ => unreachable!(),
                };
                assert!(used > 0);
                let short = native::admit(proofs, selected, limit(used - 1));
                let Err(Error::Incomplete(exhaustion)) = short.result() else {
                    panic!("one-short {dimension:?}")
                };
                assert_eq!(exhaustion.dimension, dimension);
                assert_eq!(exhaustion.limit, used - 1);
                assert!(exhaustion.used <= exhaustion.limit);
                assert!(exhaustion.requested > exhaustion.limit - exhaustion.used);
                let exact = native::admit(proofs, selected, limit(used));
                assert!(exact.result().is_ok(), "{:?}", exact.result().err());
                assert_eq!(
                    exact.result().unwrap().package(),
                    baseline.result().unwrap().package()
                );
            }
            round_trip(&inputs, proofs, &baseline.into_result().unwrap());
        },
    );
}
