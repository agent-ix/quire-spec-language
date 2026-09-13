// SPDX-License-Identifier: AGPL-3.0-only
//! TC-121: the payment-retry choreography of #39 through the actual parse,
//! link, type, proof, admission, emission and public reader stages.
//!
//! Orders O1/O2 share payment provider P (`role Provider`). One charge message
//! is observed by the two shipment deliveries S1/S2; A1 is the single authored
//! forward payment attempt inside the retry and E1 is its one business effect,
//! and R1/R2 are the refunds on exhaustion. Bounded iteration mints no further
//! static attempt record, so there is one A1 and no A2 in the artifact.
//! Every record here is a static identity requirement: no payment, delivery or
//! recovery is observed, and no runtime outcome is supplied by these fixtures.

#[path = "support/native_protocol/mod.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_spec_language::checking::composed::{proofs, TypeDisposition, TypeLimits};
use quire_spec_language::protocol_artifact::{
    self as artifact, native, wire as w, Error, Invalid, Limits, Unsupported,
};
use setup::{Inputs, Unit};

/// One charge message with the two shipment delivery observations S1 and S2.
const CHARGE: &str = "send ChargeO1 via Charges as (chargeO1: M::Plain) { true };
    receive S1 via Charges of ChargeO1 as (shipmentS1: M::Plain) { true };
    receive S2 via Charges of ChargeO1 as (shipmentS2: M::Plain) { true };";

/// The forward payment attempt and the single declared business effect of it.
const ATTEMPT: &str = "sequence Charged {
    attempt A1 by Provider on M::Node::step contracts [] as (attemptA1: M::Plain) { true };
    effect E1 of Charged::A1 as (effectE1: M::Plain) { true };
}";

/// Distinct refunds R1 and R2, reached only when the bound is exhausted.
const REFUNDS: &str = "sequence Refunds {
    event R1 by Merchant as (refundR1: M::Plain) { true };
    event R2 by Provider as (refundR2: M::Plain) { true };
}";

/// A second order sharing provider P, with its own distinct binding subject.
const ORDER_O2: &str = "sequence O2 {
    event AckO2 by Provider as (ackO2: M::Plain) { true };
}";

/// The retry as authored: guard, bound and both exits are original source.
fn retry(owner: &str, visible: &str, maximum: i64, guard: &str, body: &str, exits: &str) -> String {
    format!(
        "repeat Retry by {owner} visible ({visible}) max {maximum} while {{ {guard} }}
         {body} exhausted {exits}"
    )
}

/// One order O1 carrying the charge, both deliveries and the authored retry.
fn order(retry: &str) -> String {
    format!("sequence O1 {{ {CHARGE} {retry} }}")
}

fn inputs(run: &str) -> Inputs {
    let body = format!(
        "protocol Payments using P over (view: M::Node) on origin {{
          role Merchant on M::Node;
          role Provider on M::Node;
          channel Charges from Merchant to Provider carries M::Plain
            ordering unordered delivery [1,2];
          run {run}
          finish Closed as (closed: M::Node) {{ true }};
        }}"
    );
    Inputs::new(&[Unit {
        name: "payment-retry",
        body: &body,
        declarations: &["Payments"],
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

#[track_caller]
fn refused(inputs: &Inputs, expected: Error, case: &str) {
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

struct Protocol<'a> {
    controls: &'a [w::Control],
    edges: &'a [w::CausalEdge],
    roles: &'a [w::Role],
    channels: &'a [w::Channel],
}

fn protocol(package: &w::Package) -> Protocol<'_> {
    let w::Body::Protocol {
        controls,
        causal_edges,
        roles,
        channels,
        ..
    } = &package.declarations[0].body
    else {
        panic!("authored protocol declaration")
    };
    Protocol {
        controls,
        edges: causal_edges,
        roles,
        channels,
    }
}

/// The authored control of that name, with the handle other records use.
#[track_caller]
fn control<'a>(controls: &'a [w::Control], name: &str) -> (w::Handle, &'a w::Control) {
    let index = controls
        .iter()
        .position(|control| control.name == name)
        .unwrap_or_else(|| panic!("authored control {name}"));
    (
        w::Handle {
            declaration: 0,
            index: index as u32,
        },
        &controls[index],
    )
}

#[track_caller]
fn repeat<'a>(controls: &'a [w::Control], name: &str) -> (w::Handle, &'a w::ControlOperation) {
    let (handle, control) = control(controls, name);
    assert!(
        matches!(control.operation, w::ControlOperation::Repeat { .. }),
        "{name} is an authored bounded repeat"
    );
    (handle, &control.operation)
}

fn integer_value(value: &w::Integer) -> i64 {
    let artifact::ProtocolNumber::Integer(value) = value.checked().unwrap() else {
        panic!("authored integer bound")
    };
    value.value()
}

/// A retained observed Boolean atom: an admitted Boolean field of an event
/// binder, carrying the originating event anchor rather than the anchor of the
/// decision that reads it.
struct Atom<'a> {
    binder: &'a w::Binder,
    field: &'a w::ExportRef,
    origin: &'a w::Handle,
}

#[track_caller]
fn atom<'a>(declaration: &'a w::Declaration, handle: &w::Handle) -> Atom<'a> {
    assert_eq!(handle.declaration, 0, "declaration-local value handle");
    let value = &declaration.values[handle.index as usize];
    let w::ValueOperation::Field { base, field } = &value.operation else {
        panic!("observed atom is a field read, found {:?}", value.operation)
    };
    let w::ValueOperation::Read { binder } = &declaration.values[base.index as usize].operation
    else {
        panic!("observed atom reads an original binder")
    };
    let w::Origin::Anchor { anchor } = &value.origin else {
        panic!(
            "observed atom keeps its originating anchor, found {:?}",
            value.origin
        )
    };
    Atom {
        binder: &declaration.binders[binder.index as usize],
        field,
        origin: anchor,
    }
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

/// The declaration-local subject of a control-owned binding requirement.
fn subject(control: &w::Handle) -> w::Subject {
    w::Subject::Control {
        control: control.clone(),
    }
}

fn events(controls: &[w::Control]) -> Vec<&w::Event> {
    controls
        .iter()
        .filter_map(|control| match &control.operation {
            w::ControlOperation::Event { event, .. } => Some(event),
            _ => None,
        })
        .collect()
}

/// Item 1. The whole retry: a bounded repeat over a pre-loop observed charge
/// outcome, with the forward attempt and its single effect inside the body.
#[test]
#[trace(
    "TC-121",
    "TC-134",
    "FR-042-AC-1",
    "FR-042-AC-5",
    "FR-042-AC-6",
    "FR-042-AC-7",
    "FR-048-AC-6"
)]
fn payment_retry_over_a_pre_loop_observed_charge_outcome_emits_its_attempt_and_effect() {
    let run = order(&retry(
        "Provider",
        "shipmentS1.ready",
        2,
        "not shipmentS1.ready",
        ATTEMPT,
        REFUNDS,
    ));
    let inputs = inputs(&run);
    admitted(&inputs, |package| {
        let declaration = &package.declarations[0];
        let protocol = protocol(package);
        let (retry, operation) = repeat(protocol.controls, "Retry");
        let w::ControlOperation::Repeat {
            owner,
            visible,
            maximum,
            guard,
            body,
            exhausted,
        } = operation
        else {
            unreachable!()
        };
        // The guard is owned by the actual channel recipient, provider P.
        assert_eq!(protocol.roles[owner.index as usize].name, "Provider");
        assert_eq!(&protocol.channels[0].to, owner);
        assert_eq!(integer_value(maximum), 2);

        // One listed atom, and the guard negates exactly that same atom: the
        // same binder, the same admitted Boolean export, the same anchor.
        assert_eq!(visible.len(), 1);
        let listed = atom(declaration, &visible[0]);
        let w::ValueOperation::Unary {
            operator: w::Unary::Not,
            value,
        } = &declaration.values[guard.index as usize].operation
        else {
            panic!("authored `not` over the observed outcome")
        };
        let observed = atom(declaration, value);
        assert_eq!(listed.binder.name, "shipmentS1");
        assert_eq!(observed.binder.name, "shipmentS1");
        assert_eq!(listed.field, observed.field);
        assert_eq!(
            package.models[listed.field.model as usize].exports[listed.field.export as usize].path,
            ["Plain", "ready"]
        );

        // The atom's provenance is the pre-loop delivery observation S1, not
        // the repeat's own decision anchor.
        let (shipment, _) = control(protocol.controls, "S1");
        assert_eq!(listed.origin, &listed.binder.anchor);
        assert_eq!(
            declaration.anchors[listed.origin.index as usize].owner.0,
            Some(shipment.clone())
        );
        assert_ne!(
            declaration.anchors[listed.origin.index as usize].owner.0,
            Some(retry.clone()),
            "the atom is anchored at S1, never at the repeat that reads it"
        );

        // The body carries the forward attempt and its single effect.
        let (charged, _) = control(protocol.controls, "Charged");
        assert_eq!(body, &charged);
        let (attempt, attempted) = control(protocol.controls, "A1");
        let w::ControlOperation::Event {
            event: w::Event::Attempt { owner: by, .. },
            ..
        } = &attempted.operation
        else {
            panic!("A1 is the authored forward attempt")
        };
        assert_eq!(by, owner);
        let (_, applied) = control(protocol.controls, "E1");
        let w::ControlOperation::Event {
            event:
                w::Event::Effect {
                    attempt: of_attempt,
                    ..
                },
            ..
        } = &applied.operation
        else {
            panic!("E1 is the authored business effect")
        };
        assert_eq!(of_attempt, &attempt);

        // Exhaustion leaves the retry through the authored refunds R1/R2.
        let (refunds, _) = control(protocol.controls, "Refunds");
        assert_eq!(exhausted, &refunds);

        let progress: Vec<_> = protocol
            .edges
            .iter()
            .filter(|edge| edge.kind == w::EdgeKind::RepeatProgress)
            .collect();
        assert_eq!(progress.len(), 1);
        assert_eq!(progress[0].owner, retry);
        assert_eq!(progress[0].from.node, charged);
        assert_eq!(progress[0].from.port, w::Port::Exit);
        assert_eq!(progress[0].to.node, retry);
        assert_eq!(progress[0].to.port, w::Port::Enter);
        assert_eq!(integer_value(progress[0].maximum.0.as_ref().unwrap()), 2);
    });
}

/// Item 2, guard-independent half. The `exhausted` branch is retained and
/// entered exactly once at the bound, and a zero maximum admits only the
/// normal and exhausted paths: the same non-progressing body that a positive
/// bound refuses is accepted when the body can never be entered.
#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-7")]
fn a_zero_maximum_never_enters_the_retry_body_that_a_positive_bound_requires() {
    let checks = "sequence Charged { check Pending using S { true }; }";
    for maximum in [0, 2] {
        let run = order(&retry("Provider", "true", maximum, "true", checks, REFUNDS));
        let inputs = inputs(&run);
        if maximum == 0 {
            admitted(&inputs, |package| {
                let protocol = protocol(package);
                let (retry, operation) = repeat(protocol.controls, "Retry");
                let w::ControlOperation::Repeat {
                    maximum, exhausted, ..
                } = operation
                else {
                    unreachable!()
                };
                assert_eq!(integer_value(maximum), 0);
                let (refunds, _) = control(protocol.controls, "Refunds");
                assert_eq!(exhausted, &refunds);
                let progress: Vec<_> = protocol
                    .edges
                    .iter()
                    .filter(|edge| edge.kind == w::EdgeKind::RepeatProgress)
                    .collect();
                assert_eq!(progress.len(), 1);
                assert_eq!(integer_value(progress[0].maximum.0.as_ref().unwrap()), 0);

                // The exhausted branch is reached once, and rejoins once.
                let entered: Vec<_> = protocol
                    .edges
                    .iter()
                    .filter(|edge| {
                        edge.owner == retry
                            && edge.kind == w::EdgeKind::Branch
                            && edge.to.node == refunds
                    })
                    .collect();
                assert_eq!(entered.len(), 1);
                assert_eq!(entered[0].from.node, retry);
                assert_eq!(entered[0].from.port, w::Port::Enter);
                assert_eq!(entered[0].to.port, w::Port::Enter);
                let rejoined: Vec<_> = protocol
                    .edges
                    .iter()
                    .filter(|edge| {
                        edge.owner == retry
                            && edge.kind == w::EdgeKind::Join
                            && edge.from.node == refunds
                    })
                    .collect();
                assert_eq!(rejoined.len(), 1);
                assert_eq!(rejoined[0].to.node, retry);
                assert_eq!(rejoined[0].to.port, w::Port::Exit);
            });
        } else {
            refused(
                &inputs,
                Error::Invalid(Invalid::Control),
                "a bounded retry whose body can be entered must progress",
            );
        }
    }
}

/// Item 2 and item 3 in their final observed-guard form. Guard falsification is
/// a normal exit, the bound still governs the body, and a continuing body of
/// checks only is refused as an evaluated control defect rather than accepted.
#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-7", "FR-042-AC-8")]
fn an_observed_guard_retains_the_bound_and_refuses_a_nonprogressing_retry_body() {
    let checks = "sequence Charged { check Pending using S { true }; }";
    for (case, maximum, body, expected) in [
        ("zero bound never enters the body", 0, checks, None),
        ("bounded attempt progresses", 2, ATTEMPT, None),
        (
            "a continuing body of checks only",
            2,
            checks,
            Some(Error::Invalid(Invalid::Control)),
        ),
    ] {
        let run = order(&retry(
            "Provider",
            "shipmentS1.ready",
            maximum,
            "not shipmentS1.ready",
            body,
            REFUNDS,
        ));
        let inputs = inputs(&run);
        match expected {
            Some(expected) => refused(&inputs, expected, case),
            None => admitted(&inputs, |package| {
                let protocol = protocol(package);
                let (_, operation) = repeat(protocol.controls, "Retry");
                let w::ControlOperation::Repeat {
                    maximum: bound,
                    exhausted,
                    ..
                } = operation
                else {
                    unreachable!()
                };
                assert_eq!(integer_value(bound), maximum, "{case}");
                let (refunds, _) = control(protocol.controls, "Refunds");
                assert_eq!(exhausted, &refunds, "{case}");
            }),
        }
    }
}

/// Item 4. A guard atom observed by another role, or not listed in this
/// repeat's own `visible(...)` set, is an unsupported family prerequisite —
/// while the identical retry whose owner does observe the listed atom admits.
#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-8")]
fn foreign_role_and_unlisted_guard_atoms_refuse_where_the_owner_observation_admits() {
    for (case, owner, visible, guard, supported) in [
        (
            "the actual delivery recipient owns the guard",
            "Provider",
            "shipmentS1.ready",
            "not shipmentS1.ready",
            true,
        ),
        (
            "the sending merchant never observes the delivery",
            "Merchant",
            "shipmentS1.ready",
            "not shipmentS1.ready",
            false,
        ),
        (
            "a hidden atom with nothing listed",
            "Provider",
            "true",
            "not shipmentS1.ready",
            false,
        ),
        (
            "a hidden atom while another delivery is listed",
            "Provider",
            "shipmentS2.ready",
            "not shipmentS1.ready",
            false,
        ),
    ] {
        let run = order(&retry(owner, visible, 2, guard, ATTEMPT, REFUNDS));
        let inputs = inputs(&run);
        if supported {
            admitted(&inputs, |package| {
                let protocol = protocol(package);
                let (_, operation) = repeat(protocol.controls, "Retry");
                let w::ControlOperation::Repeat { owner, .. } = operation else {
                    unreachable!()
                };
                assert_eq!(
                    protocol.roles[owner.index as usize].name, "Provider",
                    "{case}"
                );
            });
        } else {
            refused(&inputs, Error::Unsupported(Unsupported::FamilyProof), case);
        }
    }
}

/// Item 5. An immutable alias is transparent to the guard's identity: the exact
/// atom, binder, admitted Boolean export, owning role and originating anchor
/// survive, and none of them is replaced by the alias's own name or position.
#[test]
#[trace("TC-121", "FR-042-AC-4", "FR-042-AC-5", "FR-042-AC-7")]
fn an_immutable_alias_guard_retains_the_exact_atom_binder_field_role_and_anchor() {
    let run = order(&retry(
        "Provider",
        "shipmentS1.ready",
        2,
        "let settled = shipmentS1.ready in not settled",
        ATTEMPT,
        REFUNDS,
    ));
    let inputs = inputs(&run);
    admitted(&inputs, |package| {
        let declaration = &package.declarations[0];
        let protocol = protocol(package);
        let (_, operation) = repeat(protocol.controls, "Retry");
        let w::ControlOperation::Repeat {
            owner,
            visible,
            guard,
            ..
        } = operation
        else {
            unreachable!()
        };
        let w::ValueOperation::Let {
            binder,
            initializer,
            body,
        } = &declaration.values[guard.index as usize].operation
        else {
            panic!("authored immutable alias retained in the guard")
        };
        let alias = &declaration.binders[binder.index as usize];
        assert_eq!(alias.name, "settled");
        assert_eq!(alias.initializer.0.as_ref(), Some(initializer));

        // The alias initializer is the same atom the repeat lists as visible.
        let aliased = atom(declaration, initializer);
        let listed = atom(declaration, &visible[0]);
        assert_eq!(aliased.binder.name, "shipmentS1");
        assert_eq!(aliased.binder.name, listed.binder.name);
        assert_eq!(aliased.field, listed.field);
        assert_eq!(
            package.models[aliased.field.model as usize].exports[aliased.field.export as usize]
                .path,
            ["Plain", "ready"]
        );

        // The alias is read, not re-derived: the guard body reads the binder.
        let w::ValueOperation::Unary {
            operator: w::Unary::Not,
            value,
        } = &declaration.values[body.index as usize].operation
        else {
            panic!("authored `not` over the alias")
        };
        assert!(matches!(
            &declaration.values[value.index as usize].operation,
            w::ValueOperation::Read { binder: read } if read == binder
        ));

        // Role and anchor identity: the repeat owner is the actual channel
        // recipient, and the atom's anchor is the delivery event S1 that bound
        // it, which is exactly the anchor of S1's own delivery requirement.
        assert_eq!(protocol.roles[owner.index as usize].name, "Provider");
        assert_eq!(&protocol.channels[0].to, owner);
        let (shipment, _) = control(protocol.controls, "S1");
        assert_eq!(aliased.origin, &aliased.binder.anchor);
        assert_eq!(
            declaration.anchors[aliased.origin.index as usize].owner.0,
            Some(shipment.clone())
        );
        let observed: Vec<_> = control_requirements(declaration, w::BindingKind::Receive)
            .into_iter()
            .filter(|(_, binding)| binding.subject == subject(&shipment))
            .collect();
        assert_eq!(observed.len(), 1);
        assert_eq!(&observed[0].1.anchor, aliased.origin);
    });
}

/// Item 6. Nested control flow: an owned choice inside the retry body, and the
/// retry inside a sequence with an independent event after it. The choice's
/// branches stay inside the loop and the later event stays outside it.
#[test]
#[trace("TC-121", "FR-042-AC-1", "FR-042-AC-5", "FR-042-AC-7")]
fn a_choice_inside_the_retry_and_a_following_event_keep_their_nested_edges() {
    let body = format!(
        "sequence Iteration {{
          choice Decide by Provider visible (shipmentS1.ready) {{
            case yes when {{ shipmentS1.ready }} {ATTEMPT}
            case no when {{ not shipmentS1.ready }}
              event Deferred by Provider as (deferred: M::Plain) {{ true }};
          }}
        }}"
    );
    let run = format!(
        "sequence Orders {{ {} event Settled by Merchant as (settled: M::Plain) {{ true }}; }}",
        order(&retry("Provider", "true", 2, "true", &body, REFUNDS))
    );
    let inputs = inputs(&run);
    admitted(&inputs, |package| {
        let protocol = protocol(package);
        let (retry, operation) = repeat(protocol.controls, "Retry");
        let w::ControlOperation::Repeat { body, .. } = operation else {
            unreachable!()
        };
        let (iteration, _) = control(protocol.controls, "Iteration");
        assert_eq!(body, &iteration);

        // The choice is a child of the loop body, with both branches joined.
        let (decide, decision) = control(protocol.controls, "Decide");
        let w::ControlOperation::Choice { owner, cases, .. } = &decision.operation else {
            panic!("owned choice inside the retry body")
        };
        assert_eq!(protocol.roles[owner.index as usize].name, "Provider");
        assert_eq!(
            cases
                .iter()
                .map(|case| case.label.as_str())
                .collect::<Vec<_>>(),
            ["yes", "no"]
        );
        let (charged, _) = control(protocol.controls, "Charged");
        let (deferred, _) = control(protocol.controls, "Deferred");
        assert_eq!(cases[0].body, charged);
        assert_eq!(cases[1].body, deferred);
        for branch in [charged, deferred] {
            assert!(protocol.edges.iter().any(|edge| edge.owner == decide
                && edge.kind == w::EdgeKind::Branch
                && edge.from.node == decide
                && edge.to.node == branch));
            assert!(protocol.edges.iter().any(|edge| edge.owner == decide
                && edge.kind == w::EdgeKind::Join
                && edge.from.node == branch
                && edge.to.node == decide));
        }
        assert!(protocol.edges.iter().any(|edge| edge.owner == iteration
            && edge.kind == w::EdgeKind::Sequence
            && edge.from.node == iteration
            && edge.to.node == decide));

        // The retry repeats only its own body, and the later event follows the
        // completed retry instead of joining the iteration.
        let progress: Vec<_> = protocol
            .edges
            .iter()
            .filter(|edge| edge.kind == w::EdgeKind::RepeatProgress)
            .collect();
        assert_eq!(progress.len(), 1);
        assert_eq!(progress[0].from.node, iteration);
        assert_eq!(progress[0].to.node, retry);
        let (orders, _) = control(protocol.controls, "Orders");
        let (order, _) = control(protocol.controls, "O1");
        let (settled, _) = control(protocol.controls, "Settled");
        assert!(protocol.edges.iter().any(|edge| edge.owner == orders
            && edge.kind == w::EdgeKind::Sequence
            && edge.from.node == order
            && edge.from.port == w::Port::Exit
            && edge.to.node == settled
            && edge.to.port == w::Port::Enter));
        assert!(
            !protocol
                .edges
                .iter()
                .any(|edge| edge.owner == retry && edge.to.node == settled),
            "the event after the retry is not a branch of it"
        );
    });
}

/// Item 7. The accumulating work dimensions each distinguish a zero, a
/// one-short and an exact budget over the same retry, and a fresh retry at the
/// default budget still emits a package the public reader accepts. The retry's
/// own guard is observed, so the repeat's partition work — its lowering,
/// visible basis, both branch roots and their valuations — is inside the
/// measured budget rather than only the inner choice's.
#[test]
#[trace("TC-121", "TC-133", "FR-042-AC-9", "FR-048-AC-5")]
fn accumulating_work_dimensions_separate_zero_exact_and_one_short_retry_budgets() {
    let body = format!(
        "sequence Iteration {{
          choice Decide by Provider visible (shipmentS1.ready) {{
            case yes when {{ shipmentS1.ready }} {ATTEMPT}
            case no when {{ not shipmentS1.ready }}
              event Deferred by Provider as (deferred: M::Plain) {{ true }};
          }}
        }}"
    );
    let run = order(&retry(
        "Provider",
        "shipmentS2.ready",
        2,
        "shipmentS2.ready",
        &body,
        REFUNDS,
    ));
    let inputs = inputs(&run);
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let baseline = native::admit(proofs, selected, Limits::default());
            assert!(baseline.result().is_ok(), "{:?}", baseline.result().err());
            let usage = baseline.usage();
            let expected = baseline.result().unwrap().package();
            // The measured work only covers the repeat's own partition while
            // its guard is observed; a closed guard would charge the inner
            // choice alone and leave these budgets blind to the repeat.
            let declaration = &expected.declarations[0];
            let (_, operation) = repeat(protocol(expected).controls, "Retry");
            let w::ControlOperation::Repeat { visible, guard, .. } = operation else {
                panic!("authored bounded repeat")
            };
            assert_eq!(visible.len(), 1);
            assert_eq!(atom(declaration, &visible[0]).binder.name, "shipmentS2");
            assert_eq!(atom(declaration, guard).binder.name, "shipmentS2");
            for (dimension, required, budget) in [
                (
                    artifact::Dimension::References,
                    usage.references,
                    (|value| Limits {
                        references: value,
                        ..Limits::default()
                    }) as fn(usize) -> Limits,
                ),
                (artifact::Dimension::Entries, usage.entries, |value| {
                    Limits {
                        entries: value,
                        ..Limits::default()
                    }
                }),
                (artifact::Dimension::ByteWork, usage.byte_work, |value| {
                    Limits {
                        byte_work: value,
                        ..Limits::default()
                    }
                }),
            ] {
                assert!(required > 0, "{dimension:?} accumulates over this retry");
                for limit in [0, required - 1] {
                    let report = native::admit(proofs, selected, budget(limit));
                    let Err(Error::Incomplete(exhaustion)) = report.result() else {
                        panic!(
                            "{dimension:?} budget {limit} must refuse: {:?}",
                            report.result()
                        )
                    };
                    assert_eq!(exhaustion.dimension, dimension);
                    assert_eq!(exhaustion.limit, limit);
                    assert!(exhaustion.used <= exhaustion.limit);
                    assert!(exhaustion.requested > exhaustion.limit - exhaustion.used);
                    assert!(
                        report.into_result().is_err(),
                        "{dimension:?} exhaustion admits no partial artifact"
                    );
                }
                let exact = native::admit(proofs, selected, budget(required));
                assert!(
                    exact.result().is_ok(),
                    "{dimension:?} exact budget: {:?}; locus {:?}",
                    exact.result().err(),
                    exact.locus()
                );
                assert_eq!(exact.usage(), usage);
                assert_eq!(exact.result().unwrap().package(), expected);
            }
            // A fresh invocation after exhaustion retains no charged state.
            let fresh = native::admit(proofs, selected, Limits::default());
            assert!(fresh.result().is_ok(), "{:?}", fresh.result().err());
            assert_eq!(fresh.usage(), usage);
            let fresh = fresh.into_result().unwrap();
            assert_eq!(fresh.package(), expected);
            let emitted = native::emit(&fresh, Limits::default())
                .into_result()
                .unwrap();
            let read = inputs.read(proofs, &emitted);
            assert!(
                read.result().is_ok(),
                "{:?}; locus {:?}",
                read.result().err(),
                read.locus()
            );
            assert_eq!(read.into_result().unwrap().package(), fresh.package());
        },
    );
}

/// Item 8, the #39 acceptance sentence. Transport delivery, payment attempt and
/// business effect are three separate identities: one send, two deliveries and
/// one effect stay 1/2/1 across a bounded retry, the effect depends on the
/// attempt and never the other way round, and orders O1/O2 sharing provider P
/// keep distinct binding subjects.
#[test]
#[trace(
    "TC-121",
    "TC-132",
    "TC-134",
    "FR-042-AC-5",
    "FR-042-AC-6",
    "FR-042-AC-7",
    "FR-048-AC-2",
    "FR-048-AC-6"
)]
fn delivery_attempt_and_business_effect_identities_stay_separate_across_a_retry() {
    let run = format!(
        "sequence Orders {{ {} {ORDER_O2} }}",
        order(&retry("Provider", "true", 2, "true", ATTEMPT, REFUNDS))
    );
    let inputs = inputs(&run);
    admitted(&inputs, |package| {
        let declaration = &package.declarations[0];
        let protocol = protocol(package);
        let events = events(protocol.controls);

        // One authored send, two delivery observations of it, one effect.
        let sends: Vec<_> = events
            .iter()
            .filter(|event| matches!(event, w::Event::Send { .. }))
            .collect();
        let deliveries: Vec<_> = events
            .iter()
            .filter(|event| matches!(event, w::Event::Receive { .. }))
            .collect();
        let effects: Vec<_> = events
            .iter()
            .filter(|event| matches!(event, w::Event::Effect { .. }))
            .collect();
        let attempts: Vec<_> = events
            .iter()
            .filter(|event| matches!(event, w::Event::Attempt { .. }))
            .collect();
        assert_eq!((sends.len(), deliveries.len(), effects.len()), (1, 2, 1));
        assert_eq!(attempts.len(), 1);
        let (charge, _) = control(protocol.controls, "ChargeO1");
        for delivery in &deliveries {
            let w::Event::Receive { send, channel } = delivery else {
                unreachable!()
            };
            assert_eq!(send, &charge, "both deliveries observe the one charge");
            assert_eq!(
                channel,
                &w::Handle {
                    declaration: 0,
                    index: 0
                }
            );
        }

        // The three identities are separate binding requirements, and the
        // transport delivery of the channel is not the attempt or the effect.
        assert_eq!(
            control_requirements(declaration, w::BindingKind::Send).len(),
            1
        );
        assert_eq!(
            control_requirements(declaration, w::BindingKind::Receive).len(),
            2
        );
        assert_eq!(requirements(declaration, w::BindingKind::Delivery).len(), 1);
        let attempt = control_requirements(declaration, w::BindingKind::Attempt);
        let effect = control_requirements(declaration, w::BindingKind::Effect);
        assert_eq!(attempt.len(), 1, "one attempt identity, not one per bound");
        assert_eq!(effect.len(), 1, "a retried attempt mints no further effect");
        let (attempt_index, attempt) = attempt[0];
        let (effect_index, effect) = effect[0];
        let (a1, _) = control(protocol.controls, "A1");
        let (e1, applied) = control(protocol.controls, "E1");
        assert_eq!(attempt.subject, subject(&a1));
        assert_eq!(effect.subject, subject(&e1));

        // The effect requires exactly its own attempt; the attempt requires no
        // effect, so an attempt observation implies no business effect.
        assert_eq!(effect.requires, vec![attempt_index]);
        assert!(!attempt.requires.contains(&effect_index));
        let w::ControlOperation::Event {
            event: w::Event::Effect { attempt: of, .. },
            ..
        } = &applied.operation
        else {
            unreachable!()
        };
        assert_eq!(of, &a1);
        assert!(
            !protocol
                .edges
                .iter()
                .any(|edge| edge.kind == w::EdgeKind::RepeatProgress && edge.from.node == e1),
            "the effect is not itself the repeated occurrence"
        );

        // Orders O1 and O2 share provider P through distinct subjects.
        let provider = protocol
            .roles
            .iter()
            .position(|role| role.name == "Provider")
            .expect("authored provider role") as u32;
        let provider = w::Handle {
            declaration: 0,
            index: provider,
        };
        let instance = requirements(declaration, w::BindingKind::RoleInstance)
            .into_iter()
            .find(|(_, binding)| {
                binding.subject
                    == (w::Subject::Role {
                        role: provider.clone(),
                    })
            })
            .expect("provider role instance")
            .0;
        let (ack, acknowledged) = control(protocol.controls, "AckO2");
        let w::ControlOperation::Event {
            event: w::Event::Event { owner, .. },
            ..
        } = &acknowledged.operation
        else {
            panic!("O2 acknowledgement event")
        };
        assert_eq!(owner, &provider);
        let owned: Vec<_> = declaration
            .bindings
            .iter()
            .filter(|binding| binding.requires.contains(&instance))
            .map(|binding| &binding.subject)
            .collect();
        assert!(owned.contains(&&subject(&a1)));
        assert!(owned.contains(&&subject(&ack)));
        assert_ne!(subject(&a1), subject(&ack));
    });
}

/// Item 8, the negative half: an authored attempt with no authored effect
/// produces no effect record at all. No stage may mint one from the attempt.
#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-6", "FR-042-AC-7")]
fn a_retried_attempt_without_an_authored_effect_emits_no_effect_record() {
    let body = "sequence Charged {
        attempt A1 by Provider on M::Node::step contracts [] as (attemptA1: M::Plain) { true };
    }";
    let run = order(&retry("Provider", "true", 2, "true", body, REFUNDS));
    let inputs = inputs(&run);
    admitted(&inputs, |package| {
        let declaration = &package.declarations[0];
        let protocol = protocol(package);
        let events = events(protocol.controls);
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, w::Event::Attempt { .. }))
                .count(),
            1
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, w::Event::Effect { .. })),
            "no unauthored effect record accompanies the attempt"
        );
        assert!(requirements(declaration, w::BindingKind::Effect).is_empty());
        assert_eq!(
            control_requirements(declaration, w::BindingKind::Attempt).len(),
            1
        );
    });
}
