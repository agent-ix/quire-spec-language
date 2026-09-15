// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exhaustive two-atom output-determinacy cases through the public compiler.
//! A four-row bit table is the independent test oracle, not a formula evaluator.

#[path = "support/native_protocol/mod.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_spec_language::checking::composed::{proofs, TypeDisposition, TypeLimits};
use quire_spec_language::protocol_artifact::{native, wire as w, Error, Limits, Unsupported};
use quire_spec_language::syntax::composed as c;
use setup::{Inputs, Unit};

/// Enumerate each Boolean function on two independent observation bits. Every
/// minterm names both atoms, including the deliberately contradictory zero
/// function, so authorization is identical across the 256 pairs.
fn formula(table: u8) -> String {
    const MINTERMS: [&str; 4] = [
        "(not a.ready and not b.ready)",
        "(a.ready and not b.ready)",
        "(not a.ready and b.ready)",
        "(a.ready and b.ready)",
    ];
    if table == 0 {
        return "(a.ready and not a.ready and b.ready)".into();
    }
    MINTERMS
        .iter()
        .enumerate()
        .filter(|(row, _)| table & (1_u8 << row) != 0)
        .map(|(_, term)| *term)
        .collect::<Vec<_>>()
        .join(" or ")
}

/// Visibility determines the selected case iff each pair of rows with equal
/// advertised output also has equal guard output. No compiler or syntax state
/// participates in this four-row relational check.
fn determines(visible: u8, guard: u8) -> bool {
    (0..4).all(|left| {
        (0..4).all(|right| {
            let advertised_equal = ((visible >> left) & 1) == ((visible >> right) & 1);
            let selected_equal = ((guard >> left) & 1) == ((guard >> right) & 1);
            !advertised_equal || selected_equal
        })
    })
}

fn inputs(visible: &str, guard: &str) -> Inputs {
    let body = format!(
        "protocol Decisions using P over (view: M::Node) on origin {{
            role Receiver on M::Node;
            run sequence Main {{
                event ObservedA by Receiver as (a: M::Plain) {{ true }};
                event ObservedB by Receiver as (b: M::Plain) {{ true }};
                choice Decide by Receiver visible ({visible}) {{
                    case yes when {{ {guard} }} check Accepted using S {{ true }};
                    case no when {{ not ({guard}) }} check Rejected using S {{ true }};
                }}
            }}
            finish Closed as (closed: M::Node) {{ true }};
        }}"
    );
    Inputs::new(&[Unit {
        name: "two-atom-visible-truth-tables",
        body: &body,
        declarations: &["Decisions"],
    }])
}

#[test]
#[trace("TC-121", "FR-042-AC-1", "FR-042-AC-5", "FR-042-AC-7")]
fn all_two_atom_visible_and_guard_functions_match_output_determinacy() {
    let mut accepted = 0;
    let mut refused = 0;
    for visible in 0_u8..16 {
        for guard in 0_u8..16 {
            let inputs = inputs(&formula(visible), &formula(guard));
            inputs.with_proofs(
                TypeLimits::default(),
                proofs::ProofLimits::default(),
                |proofs, selected| {
                    assert!(proofs.exhaustion().is_none());
                    assert_eq!(proofs.declarations().len(), 1);
                    for declaration in proofs.declarations() {
                        assert_eq!(
                            proofs.types().disposition(declaration.declaration()),
                            Some(TypeDisposition::Typed),
                            "visible={visible:04b}, guard={guard:04b}"
                        );
                        assert_eq!(
                            declaration.disposition(),
                            proofs::ProofDisposition::Discharged,
                            "visible={visible:04b}, guard={guard:04b}: {:?}",
                            declaration.causes()
                        );
                    }
                    let report = native::admit(proofs, selected, Limits::default());
                    if !determines(visible, guard) {
                        assert_eq!(
                            report.result().err(),
                            Some(&Error::Unsupported(Unsupported::FamilyProof)),
                            "visible={visible:04b}, guard={guard:04b}"
                        );
                        refused += 1;
                        return;
                    }
                    assert!(
                        report.result().is_ok(),
                        "visible={visible:04b}, guard={guard:04b}: {:?}, {:?}",
                        report.result().err(),
                        report.locus()
                    );
                    let admitted = report.into_result().unwrap();
                    let package = admitted.package();
                    assert_eq!(package.sources.len(), 1);
                    assert_eq!(package.sources[0].text, inputs.sources[0].text());
                    assert_eq!(
                        package.sources[0].native.identity,
                        inputs.sources[0].identity().identity
                    );
                    assert_eq!(
                        package.sources[0].native.revision,
                        inputs.sources[0].identity().revision
                    );
                    original_roots(proofs, package);
                    let emitted = native::emit(&admitted, Limits::default())
                        .into_result()
                        .unwrap();
                    let read = inputs.read(proofs, &emitted);
                    assert!(
                        read.result().is_ok(),
                        "visible={visible:04b}, guard={guard:04b}: {:?}",
                        read.result().err()
                    );
                    assert_eq!(read.into_result().unwrap().package(), package);
                    accepted += 1;
                },
            );
        }
    }
    // Each constant visible function admits the two constant guards. Each of
    // the other 14 functions admits false, true, itself and its complement.
    assert_eq!((accepted, refused), (60, 196));
}

fn original_roots(proofs: &proofs::ProofReport<'_, '_, '_>, package: &w::Package) {
    let declaration = &package.declarations[0];
    let namespace = proofs.types().binding().namespace();
    let [id] = namespace.lookup("Decisions") else {
        panic!("one original declaration")
    };
    let unit = namespace
        .unit(proofs.types().declaration(*id).unwrap().unit())
        .unwrap();
    let w::Body::Protocol { controls, .. } = &declaration.body else {
        panic!("authored protocol")
    };
    let retained = controls.iter().find(|node| node.name == "Decide").unwrap();
    let original = &unit.controls()[retained.original_node as usize];
    assert_eq!(retained.name, original.name.value);
    assert_eq!(retained.locus.source, 0);
    assert_eq!(
        (
            retained.locus.span.start as usize,
            retained.locus.span.end as usize
        ),
        (original.span.start, original.span.end)
    );
    let c::ControlKind::Choice {
        visible: original_visible,
        cases: original_cases,
        ..
    } = &original.kind
    else {
        panic!("original choice")
    };
    let w::ControlOperation::Choice { visible, cases, .. } = &retained.operation else {
        panic!("retained choice")
    };
    assert_eq!(visible.len(), 1);
    assert_eq!(visible.len(), original_visible.len());
    assert_eq!(cases.len(), 2);
    assert_eq!(cases.len(), original_cases.len());
    let roots = visible.iter().zip(original_visible.iter().copied()).chain(
        cases
            .iter()
            .zip(original_cases)
            .map(|(retained, original)| {
                assert_eq!(retained.label, original.name.value);
                (&retained.guard, original.guard)
            }),
    );
    for (handle, expression) in roots {
        assert_eq!(handle.declaration, 0);
        let value = &declaration.values[handle.index as usize];
        let original = unit.expression(expression).unwrap();
        assert_eq!(
            &unit.expressions()[value.original_expression as usize],
            original
        );
        assert_eq!(value.locus.source, 0);
        assert_eq!(
            (
                value.locus.span.start as usize,
                value.locus.span.end as usize
            ),
            (original.span.start, original.span.end)
        );
    }
}
