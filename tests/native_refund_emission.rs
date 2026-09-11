// SPDX-License-Identifier: AGPL-3.0-only
//! TC-121: the related-refund leg of #39 through the actual parse, link, type,
//! proof, admission, emission and public reader stages.
//!
//! Provider P charges order O1 through the forward attempt A1, whose single
//! business effect is E1. The refunds R1 and R2 are compensations of that
//! established effect: R1 recovers it fully and commits, R2 recovers it partly
//! and never commits. A refund registers on the effect and on nothing else, and
//! the refund attempts and refund effects stay separate identities from the
//! transport deliveries, the forward attempt and that one business effect.
//! Nothing here observes a refund; every record is a static requirement.

#[path = "support/native_protocol/mod.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_spec_language::checking::composed::{proofs, TypeDisposition, TypeLimits};
use quire_spec_language::linking::composed::scopes::{ScopeIssue, StructuralKind};
use quire_spec_language::protocol_artifact::{
    self as artifact, native, wire as w, Error, Limits, Unsupported,
};
use setup::{Inputs, Unit};

/// One refund obligation on the payment effect, with its authored attempt bound,
/// commit boundary and recovery relation.
fn refund(name: &str, attempts: u32, commit: &str, recovery: &str) -> String {
    format!(
        "compensate {name} for O1::Payment::E1 as (forward{name}: M::Node)
            by Provider on M::Node::step using T clock \"refund-{name}\" {{
          capture target{name}: M::Total = forward{name}.total;
          activate first (trigger{name}: M::Plain) when {{ not trigger{name}.ready }} {{
            capture activated{name}: Boolean = trigger{name}.ready;
          }}
          within [0,30]; attempts {attempts} of M::Node;
          retry (earlier{name}: M::Node, later{name}: M::Node) {{
            earlier{name}.tally < 3 and later{name}.tally = earlier{name}.tally + 1
            and target{name} >= 0 and not activated{name}
          }};
          commit {commit};
          recover (recovered{name}: M::Node) {{ {recovery} }};
        }}"
    )
}

/// R1 restores the whole charged total; R2 restores a strict part of it.
fn refunds() -> (String, String) {
    (
        refund(
            "R1",
            3,
            "O1::Committed",
            "sum<M::Total>(amountR1 in recoveredR1.amounts: amountR1) = targetR1 and not activatedR1",
        ),
        refund(
            "R2",
            5,
            "never",
            "let restoredR2 = sum<M::Total>(amountR2 in recoveredR2.amounts: amountR2)
             in restoredR2 > 0 and restoredR2 < targetR2 and not activatedR2",
        ),
    )
}

/// The charged order: one notice split into the two shipment deliveries, the
/// forward payment attempt, its single business effect and the commit.
fn source() -> String {
    let (full, partial) = refunds();
    format!(
        "protocol Refunds using P over (view: M::Node) on origin {{
          role Merchant on M::Node;
          role Provider on M::Node;
          channel Shipments from Merchant to Provider carries M::Plain
            ordering unordered delivery [1,2];
          {full}
          requires temporal Due;
          {partial}
          run sequence O1 {{
            send Notice via Shipments as (notice: M::Plain) {{ true }};
            receive S1 via Shipments of Notice as (shipmentS1: M::Plain) {{ shipmentS1.ready }};
            receive S2 via Shipments of Notice as (shipmentS2: M::Plain) {{ shipmentS2.ready }};
            sequence Payment {{
              attempt A1 by Provider on M::Node::step contracts [Before,After]
                as (attemptA1: M::Plain) {{ attemptA1.ready }};
              effect E1 of Payment::A1 as (effectE1: M::Node) {{ true }};
            }}
            commit Committed by Provider as (committed: M::Plain) {{ committed.ready }};
          }}
          finish Closed as (closed: M::Node) {{ true }};
        }}"
    )
}

fn inputs(body: &str) -> Inputs {
    let mut inputs = Inputs::new(&[
        Unit {
            name: "refund-contracts",
            body: "pre Before using S on M::Node::step { delta >= 0 }
                   post After using S on M::Node::step { result }",
            declarations: &["Before", "After"],
        },
        Unit {
            name: "refund-time",
            body: "temporal Due using T over (sample: M::Node) clock \"workflow-clock\" on origin { always[0,30] holds(true) }",
            declarations: &["Due"],
        },
        Unit {
            name: "refund-flow",
            body,
            declarations: &["Refunds"],
        },
    ]);
    inputs.step_contracts("Before", "After");
    inputs
}

/// One authored mutation of the original source, checked to apply exactly once.
fn changed(from: &str, to: &str) -> String {
    let original = source();
    assert_eq!(
        original.matches(from).count(),
        1,
        "one authored mutation site"
    );
    original.replacen(from, to, 1)
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

struct Flow<'a> {
    owner: u32,
    declaration: &'a w::Declaration,
    controls: &'a [w::Control],
    compensations: &'a [w::Compensation],
}

fn flow(package: &w::Package) -> Flow<'_> {
    let at = package
        .declarations
        .iter()
        .position(|declaration| declaration.name == "Refunds")
        .expect("authored protocol declaration");
    let declaration = &package.declarations[at];
    let w::Body::Protocol {
        controls,
        compensations,
        ..
    } = &declaration.body
    else {
        panic!("authored protocol declaration")
    };
    Flow {
        owner: at as u32,
        declaration,
        controls,
        compensations,
    }
}

impl Flow<'_> {
    /// A declaration-local handle into this protocol's own tables.
    fn handle(&self, index: u32) -> w::Handle {
        w::Handle {
            declaration: self.owner,
            index,
        }
    }
}

/// The authored control of that name, with the handle other records use.
#[track_caller]
fn control(flow: &Flow<'_>, name: &str) -> w::Handle {
    let index = flow
        .controls
        .iter()
        .position(|control| control.name == name)
        .unwrap_or_else(|| panic!("authored control {name}"));
    flow.handle(index as u32)
}

fn integer_value(value: &w::Integer) -> i64 {
    let artifact::ProtocolNumber::Integer(value) = value.checked().unwrap() else {
        panic!("authored integer bound")
    };
    value.value()
}

/// The one forward business effect requirement, and the control that declares it.
#[track_caller]
fn business_effect(flow: &Flow<'_>) -> (w::Handle, u32) {
    let handle = control(flow, "E1");
    let w::ControlOperation::Event {
        event: w::Event::Effect { instance, .. },
        ..
    } = &flow.controls[handle.index as usize].operation
    else {
        panic!("E1 is the authored business effect")
    };
    (handle, *instance)
}

/// Binding requirement indices of one kind.
fn requirements(declaration: &w::Declaration, kind: w::BindingKind) -> Vec<u32> {
    declaration
        .bindings
        .iter()
        .enumerate()
        .filter(|(_, binding)| binding.kind == kind)
        .map(|(index, _)| index as u32)
        .collect()
}

/// Item 1 of the refund leg. Both refunds compensate the one payment effect and
/// keep their own registration, activation, retry and recovery authorities: the
/// authored attempt bound, commit boundary and full versus partial recovery
/// relation belong to one refund each and are never shared or swapped.
#[test]
#[trace("TC-121", "FR-042-AC-1", "FR-042-AC-6", "FR-042-AC-7")]
fn two_refunds_of_one_payment_effect_keep_separate_retry_commit_and_recovery_authorities() {
    let inputs = inputs(&source());
    admitted(&inputs, |package| {
        let flow = flow(package);
        let declaration = flow.declaration;
        let (effect, effect_instance) = business_effect(&flow);
        let [full, partial] = flow.compensations else {
            panic!("two authored refund obligations")
        };
        assert_eq!((full.name.as_str(), partial.name.as_str()), ("R1", "R2"));

        // A refund is compensation of the established effect: both register on
        // exactly the payment effect E1 and its requirement.
        let committed = control(&flow, "Committed");
        for (at, refund) in [full, partial].into_iter().enumerate() {
            assert_eq!(refund.forward_effect, effect, "refund {at} compensates E1");
            let registration = &declaration.bindings[refund.registration_instance as usize];
            assert_eq!(registration.kind, w::BindingKind::CompensationRegistration);
            assert!(
                registration.requires.contains(&effect_instance),
                "refund {at} registers only after the effect requirement"
            );
            assert_eq!(
                registration.subject,
                w::Subject::Compensation {
                    compensation: flow.handle(at as u32)
                }
            );

            // Retry: the refund attempt depends on its own activation, and the
            // refund effect on exactly that attempt.
            let attempt = &declaration.bindings[refund.attempt_instance as usize];
            let refund_effect = &declaration.bindings[refund.effect_instance as usize];
            assert_eq!(attempt.kind, w::BindingKind::CompensationAttempt);
            assert_eq!(refund_effect.kind, w::BindingKind::CompensationEffect);
            assert_eq!(refund_effect.requires, vec![refund.attempt_instance]);
            assert!(
                refund_effect.value_type.0.is_none(),
                "the identity-only refund effect declares no payload type"
            );
            let activation = attempt
                .requires
                .iter()
                .find(|index| {
                    declaration.bindings[**index as usize].anchor == refund.activation_anchor
                })
                .expect("refund attempt depends on its own activation");
            assert!(declaration.bindings[*activation as usize]
                .requires
                .contains(&refund.registration_instance));

            // The refund's own recovery premises lead its inventory: its
            // snapshot, and the progress and closure that depend on that
            // snapshot, its clock and its refund effect.
            let [snapshot, progress, closure] = refund.recovery_bindings[..3] else {
                panic!("refund {at} keeps its three recovery premises")
            };
            for (index, kind) in [
                (snapshot, w::BindingKind::Snapshot),
                (progress, w::BindingKind::Progress),
                (closure, w::BindingKind::Closure),
            ] {
                let premise = &declaration.bindings[index as usize];
                assert_eq!(premise.kind, kind, "refund {at}");
                assert_eq!(premise.subject, registration.subject, "refund {at}");
            }
            for index in [progress, closure] {
                let mut expected = vec![refund.clock, refund.effect_instance, snapshot];
                expected.sort_unstable();
                assert_eq!(
                    declaration.bindings[index as usize].requires, expected,
                    "refund {at} recovery premise depends on its own clock, \
                     refund effect and snapshot"
                );
            }
        }

        // The authored bounds, commit boundaries and recovery relations are not
        // shared: a swap between the two refunds fails every line below.
        assert_eq!(integer_value(&full.maximum_attempts), 3);
        assert_eq!(integer_value(&partial.maximum_attempts), 5);
        assert_eq!(full.commit.0.as_ref(), Some(&committed));
        assert_eq!(
            partial.commit.0, None,
            "the partial refund authored no commit boundary"
        );
        assert_ne!(full.recover, partial.recover);
        assert_ne!(full.recovery, partial.recovery);
        assert_ne!(full.clock, partial.clock);
        assert_ne!(full.registration_anchor, partial.registration_anchor);
        let shared: Vec<_> = full
            .recovery_bindings
            .iter()
            .filter(|index| partial.recovery_bindings.contains(index))
            .collect();
        assert!(
            shared.is_empty(),
            "each refund owns its recovery premises: {shared:?}"
        );

        // The two authored recovery relations keep their own original shapes:
        // the full one is a conjunction, the partial one opens its `let` alias.
        assert!(matches!(
            declaration.values[full.recover.index as usize].operation,
            w::ValueOperation::Binary { .. }
        ));
        assert!(matches!(
            declaration.values[partial.recover.index as usize].operation,
            w::ValueOperation::Let { .. }
        ));
    });
}

/// Item 2 of the refund leg. A refund registers on an established business
/// effect and on nothing weaker: a send, a transport delivery or the forward
/// attempt that lacks that effect is refused before family admission.
#[test]
#[trace("TC-121", "FR-036-AC-4", "FR-042-AC-6", "FR-042-AC-8")]
fn a_refund_cannot_register_on_a_send_delivery_or_attempt_without_the_payment_effect() {
    for (target, case) in [
        ("O1::Notice", "a send is not a business effect"),
        ("O1::S1", "a transport delivery is not a business effect"),
        (
            "O1::Payment::A1",
            "an attempt observation is not a business effect",
        ),
    ] {
        let body = changed(
            "compensate R1 for O1::Payment::E1",
            &format!("compensate R1 for {target}"),
        );
        let inputs = inputs(&body);
        inputs.with_proofs(
            TypeLimits::default(),
            proofs::ProofLimits::default(),
            |proofs, selected| {
                let binding = proofs.types().binding();
                let [id] = binding.namespace().lookup("Refunds") else {
                    panic!("{case}: one original protocol declaration")
                };
                let scope = binding.scopes().unwrap().declaration(*id).unwrap();
                let reference = scope
                    .issues
                    .iter()
                    .find_map(|issue| match issue {
                        ScopeIssue::WrongTargetKind { reference, .. }
                            if scope.references[*reference].required == StructuralKind::Effect =>
                        {
                            Some(&scope.references[*reference])
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("{case}: {:?}", scope.issues));
                let unit = binding.namespace().unit(scope.unit).unwrap();
                assert_eq!(unit.source().slice(reference.span), Some(target));
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
}

/// Item 3, the #39 acceptance sentence over all three legs at once. Transport
/// delivery, payment attempt and business effect stay distinct, and the two
/// refund attempts, two transport receipts and two refund effects are three
/// different cardinalities. Retrying a refund under the one effect identity
/// manufactures no further successful effect.
#[test]
#[trace("TC-121", "FR-042-AC-6", "FR-042-AC-7")]
fn transport_delivery_attempt_and_refund_identities_remain_separate_cardinalities() {
    let inputs = inputs(&source());
    admitted(&inputs, |package| {
        let flow = flow(package);
        let declaration = flow.declaration;
        let (_, effect_instance) = business_effect(&flow);

        // Independent cardinalities of the one choreography.
        let counted = |kind| requirements(declaration, kind);
        let receives: Vec<_> = counted(w::BindingKind::Receive)
            .into_iter()
            .filter(|index| {
                matches!(
                    declaration.bindings[*index as usize].subject,
                    w::Subject::Control { .. }
                )
            })
            .collect();
        let deliveries = counted(w::BindingKind::Delivery);
        let attempts = counted(w::BindingKind::Attempt);
        let effects = counted(w::BindingKind::Effect);
        let refund_attempts = counted(w::BindingKind::CompensationAttempt);
        let refund_effects = counted(w::BindingKind::CompensationEffect);
        assert_eq!(
            (
                receives.len(),
                deliveries.len(),
                attempts.len(),
                effects.len(),
                refund_attempts.len(),
                refund_effects.len()
            ),
            (2, 1, 1, 1, 2, 2),
            "two shipments, one per-send delivery bound, one payment attempt, \
             one business effect, two refund attempts and two refund effects"
        );
        assert_eq!(effects, vec![effect_instance]);

        // No requirement serves two of those roles, and no refund effect is the
        // business effect or depends on it directly.
        let mut every: Vec<_> = [
            receives.as_slice(),
            deliveries.as_slice(),
            attempts.as_slice(),
            effects.as_slice(),
            refund_attempts.as_slice(),
            refund_effects.as_slice(),
        ]
        .concat();
        let total = every.len();
        every.sort_unstable();
        every.dedup();
        assert_eq!(every.len(), total, "each identity is its own requirement");
        for index in &refund_effects {
            let refund_effect = &declaration.bindings[*index as usize];
            assert!(
                !refund_effect.requires.contains(&effect_instance),
                "a refund effect never inherits the forward business effect"
            );
            assert!(matches!(
                refund_effect.subject,
                w::Subject::Compensation { .. }
            ));
        }

        // The payment attempt's own requirement does not depend on the effect:
        // an attempt observation is never evidence that the effect occurred.
        let attempt = &declaration.bindings[attempts[0] as usize];
        assert!(!attempt.requires.contains(&effect_instance));
        assert_eq!(
            declaration.bindings[effect_instance as usize].requires,
            vec![attempts[0]],
            "the one business effect requires exactly its own attempt"
        );
    });
}
