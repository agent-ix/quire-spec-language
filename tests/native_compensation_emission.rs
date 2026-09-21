// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native compensation source preserves static recovery obligations through
//! actual proof discharge and emission. No runtime recovery is executed here.

#[path = "support/native_protocol/mod.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_contract_ir as ir;
use quire_spec_language::checking::composed::{
    proofs, CauseKind as TypeCause, TypeDisposition, TypeLimits,
};
use quire_spec_language::linking::composed::{
    definition_source::RegisteredDefinition as R,
    scopes::{Anchor, ScopeIssue, StructuralKind},
    DeclarationId,
};
use quire_spec_language::protocol_artifact::{
    self as artifact, native, v2, wire as w, Error, Invalid, Limits, ProtocolNumber, Unsupported,
};
use quire_spec_language::state::{
    self, BinderInput, EvaluationOutcome, EvaluationRequest, InputSlot, Limits as StateLimits,
    Refusal, StateView, Value, ValueKind,
};
use quire_spec_language::syntax::composed as c;
use quire_spec_language::ByteDigest;
use setup::{definition_bytes, Inputs, TemporalDefinitionExpectation, Unit};

fn obligation(name: &str, recovery: &str, commit: &str) -> String {
    format!(
        "compensate {name} for Main::Applied as (forward{name}: M::Node)
            by Service on M::Node::step using T clock \"recovery-{name}\" {{
          capture target{name}: M::Total = forward{name}.total;
          activate first (trigger{name}: M::Plain) when {{ not trigger{name}.ready }} {{
            capture activated{name}: Boolean = trigger{name}.ready;
          }}
          within [0,30]; attempts 3 of M::Node;
          retry (earlier{name}: M::Node, later{name}: M::Node) {{
            earlier{name}.tally < 3 and later{name}.tally = earlier{name}.tally + 1
            and target{name} >= 0 and not activated{name}
          }};
          commit {commit};
          recover (recovered{name}: M::Node) {{ {recovery} }};
        }}"
    )
}

fn source() -> String {
    let full = obligation(
        "Full",
        "sum<M::Total>(refundFull in recoveredFull.amounts: refundFull) = targetFull and not activatedFull",
        "Main::Committed",
    );
    let partial = obligation(
        "Partial",
        "let restoredPartial = sum<M::Total>(refundPartial in recoveredPartial.amounts: refundPartial) in restoredPartial > 0 and restoredPartial < targetPartial and not activatedPartial",
        "never",
    );
    format!(
        "protocol RecoveryFlow using P over (view: M::Node) on origin {{
          role Service on M::Node;
          {full}
          requires temporal Due;
          {partial}
          run sequence Main {{
            attempt Tried by Service on M::Node::step contracts [Before,After]
                as (attempted: M::Plain) {{ attempted.ready }};
            effect Applied of Main::Tried as (applied: M::Node) {{ true }};
            event Recovered by Service for Full as (recoveryEvent: M::Plain) {{ recoveryEvent.ready }};
            commit Committed by Service as (committed: M::Plain) {{ committed.ready }};
          }}
          finish Closed as (closed: M::Node) {{ true }};
        }}"
    )
}

fn inputs(body: &str) -> (Inputs, w::ExportRef) {
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
            body,
            declarations: &["RecoveryFlow"],
        },
    ]);
    let operation = inputs.step_contracts("Before", "After");
    (inputs, operation)
}

fn changed(from: &str, to: &str) -> String {
    let original = source();
    assert_eq!(
        original.matches(from).count(),
        1,
        "one authored mutation site"
    );
    original.replacen(from, to, 1)
}

fn flow(report: &proofs::ProofReport<'_, '_, '_>) -> DeclarationId {
    let [id] = report.types().binding().namespace().lookup("RecoveryFlow") else {
        panic!("one original protocol declaration")
    };
    *id
}

fn discharged(report: &proofs::ProofReport<'_, '_, '_>) {
    assert!(report.exhaustion().is_none(), "{:?}", report.exhaustion());
    for proof in report.declarations() {
        let id = proof.declaration();
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
            proof.disposition(),
            proofs::ProofDisposition::Discharged,
            "{id:?}: {:?}",
            proof.causes()
        );
        assert!(proof.complete());
    }
}

#[track_caller]
fn failure<T>(report: &artifact::Report<T>, expected: Error) {
    assert_eq!(
        report.result().err(),
        Some(&expected),
        "locus: {:?}",
        report.locus()
    );
}

fn integer(value: &w::Integer) -> i64 {
    let ProtocolNumber::Integer(value) = value.checked().unwrap() else {
        panic!("authored integer bound")
    };
    value.value()
}

fn value<'a>(declaration: &'a w::Declaration, owner: u32, handle: &w::Handle) -> &'a w::Value {
    assert_eq!(handle.declaration, owner);
    &declaration.values[handle.index as usize]
}

fn binder<'a>(declaration: &'a w::Declaration, owner: u32, handle: &w::Handle) -> &'a w::Binder {
    assert_eq!(handle.declaration, owner);
    &declaration.binders[handle.index as usize]
}

fn nominal(package: &w::Package, at: u32, expected: &str) {
    let export = match &package.types[at as usize] {
        w::Type::Scalar { export, .. }
        | w::Type::Object { export }
        | w::Type::Record { export } => export,
        _ => panic!("expected original nominal {expected} type"),
    };
    assert_eq!(
        package.models[export.model as usize].exports[export.export as usize].path,
        [expected]
    );
}

#[test]
#[trace(
    "TC-121",
    "TC-134",
    "FR-042-AC-4",
    "FR-042-AC-6",
    "FR-042-AC-7",
    "FR-048-AC-8"
)]
fn recovery_population_origins_follow_used_capture_operands_not_neighboring_source() {
    for reads_capture in [true, false] {
        let mut body = source();
        for (from, to) in [
            (
                "capture targetFull: M::Total = forwardFull.total;",
                "capture targetFull: M::Total = forwardFull.total;
                 capture recordedFull: Boolean =
                   let discardedSelectionFull = if true then forwardFull else view in true;",
            ),
            (
                "capture targetPartial: M::Total = forwardPartial.total;",
                "capture targetPartial: M::Total = forwardPartial.total;
                 capture neighborPartial: Boolean = let discardedNeighborPartial = view in true;",
            ),
            (
                "restoredPartial < targetPartial and not activatedPartial",
                "restoredPartial < targetPartial and not activatedPartial and neighborPartial",
            ),
        ] {
            assert_eq!(body.matches(from).count(), 1);
            body = body.replacen(from, to, 1);
        }
        let original_recovery = "sum<M::Total>(refundFull in recoveredFull.amounts: refundFull) = targetFull and not activatedFull";
        let recovery = format!(
            "let filteredFull = filter(refundFull in recoveredFull.amounts: refundFull >= 1)
             in let totalFull = sum<M::Total>(totalItemFull in
               map(mappedItemFull in filteredFull: mappedItemFull): totalItemFull)
             in totalFull = targetFull and {}",
            if reads_capture {
                "recordedFull"
            } else {
                "true"
            }
        );
        assert_eq!(body.matches(original_recovery).count(), 1);
        body = body.replacen(original_recovery, &recovery, 1);
        let (inputs, _) = inputs(&body);
        inputs.with_proofs(
            TypeLimits::default(),
            proofs::ProofLimits::default(),
            |proofs, selected| {
                discharged(proofs);
                let admitted = native::admit(proofs, selected, Limits::default());
                assert!(
                    admitted.result().is_ok(),
                    "reads capture {reads_capture}: {:?}; locus {:?}",
                    admitted.result().err(),
                    admitted.locus()
                );
                let admitted = admitted.into_result().unwrap();
                let package = admitted.package();
                let owner = package
                    .declarations
                    .iter()
                    .position(|d| d.name == "RecoveryFlow")
                    .unwrap() as u32;
                let declaration = &package.declarations[owner as usize];
                let w::Body::Protocol { compensations, .. } = &declaration.body else {
                    panic!("protocol")
                };
                let [full, partial] = compensations.as_slice() else {
                    panic!("two original compensations")
                };
                assert_eq!(
                    (full.name.as_str(), partial.name.as_str()),
                    ("Full", "Partial")
                );
                let namespace = proofs.types().binding().namespace();
                let id = flow(proofs);
                let typed = proofs.types().declaration(id).unwrap();
                let unit = namespace.unit(typed.unit()).unwrap();
                let scope = proofs
                    .types()
                    .binding()
                    .scopes()
                    .unwrap()
                    .declaration(id)
                    .unwrap();
                let c::DeclarationKind::Protocol(original) = &namespace.syntax(id).unwrap().kind
                else {
                    panic!("original protocol")
                };
                assert_eq!(
                    package.sources[declaration.locus.source as usize].text,
                    unit.source().text()
                );
                let named = |name: &str| {
                    let found: Vec<_> = declaration
                        .binders
                        .iter()
                        .filter(|b| b.name == name)
                        .collect();
                    assert_eq!(found.len(), 1, "one original binder {name}");
                    found[0]
                };
                let view = named("view");
                assert_eq!(
                    declaration.anchors[view.anchor.index as usize].kind,
                    w::AnchorKind::ProtocolInstant
                );
                for (name, expected) in [
                    ("view", Anchor::ProtocolInstant),
                    ("recordedFull", Anchor::Registration(0)),
                    ("neighborPartial", Anchor::Registration(2)),
                    ("recoveredFull", Anchor::Recovery(0)),
                    ("recoveredPartial", Anchor::Recovery(2)),
                ] {
                    let retained = named(name);
                    let original = scope
                        .binders
                        .iter()
                        .find(|b| b.name.as_deref() == Some(name))
                        .unwrap();
                    assert_eq!(original.anchor, expected);
                    assert_eq!(
                        (
                            retained.locus.span.start as usize,
                            retained.locus.span.end as usize
                        ),
                        (original.span.start, original.span.end)
                    );
                }

                let assert_original = |handle: &w::Handle, expression| {
                    let retained = value(declaration, owner, handle);
                    let original = unit.expression(expression).unwrap();
                    assert_eq!(
                        &unit.expressions()[retained.original_expression as usize],
                        original
                    );
                    assert_eq!(
                        (
                            retained.locus.span.start as usize,
                            retained.locus.span.end as usize
                        ),
                        (original.span.start, original.span.end)
                    );
                };
                for (index, name) in [(0, "recordedFull"), (2, "neighborPartial")] {
                    let c::ProtocolRequirement::Compensation(original) =
                        &original.requirements[index]
                    else {
                        panic!("original compensation")
                    };
                    let capture = original
                        .registration_captures
                        .iter()
                        .find(|c| c.parameter.name.value == name)
                        .unwrap();
                    let initializer = named(name)
                        .initializer
                        .0
                        .as_ref()
                        .expect("original capture initializer");
                    assert_original(initializer, capture.value);
                    assert!(
                        capture.span.end < unit.expression(original.recover).unwrap().span.start,
                        "capture initializer is outside recovery's source region"
                    );
                    assert!(
                        matches!(
                            value(declaration, owner, initializer).origin,
                            w::Origin::Independent {}
                        ),
                        "the discarded object operand is not the captured Boolean's result origin"
                    );
                }

                // Compare original edges, including unused let initializers and
                // both conditional branches. This is not the inventory algorithm.
                let mut forms = [0; 5];
                for (index, retained) in declaration.values.iter().enumerate() {
                    let original = &unit.expressions()[retained.original_expression as usize];
                    match &original.kind {
                        c::ValueKind::Shared(quire_spec_language::syntax::ExprKind::Let {
                            name,
                            value: initial,
                            body,
                        }) => {
                            let w::ValueOperation::Let {
                                binder: target,
                                initializer,
                                body: result,
                            } = &retained.operation
                            else {
                                panic!("authored let")
                            };
                            assert_eq!(binder(declaration, owner, target).name, name.value);
                            assert_eq!(
                                binder(declaration, owner, target).initializer.0.as_ref(),
                                Some(initializer)
                            );
                            assert_original(initializer, *initial);
                            assert_original(result, *body);
                            forms[0] += 1;
                        }
                        c::ValueKind::Shared(quire_spec_language::syntax::ExprKind::If {
                            condition,
                            then_value,
                            else_value,
                        }) => {
                            let w::ValueOperation::If {
                                condition: test,
                                then_value: yes,
                                else_value: no,
                            } = &retained.operation
                            else {
                                panic!("authored selection")
                            };
                            assert_original(test, *condition);
                            assert_original(yes, *then_value);
                            assert_original(no, *else_value);
                            assert_eq!(
                                retained.origin,
                                w::Origin::Selected {
                                    value: w::Handle {
                                        declaration: owner,
                                        index: index as u32
                                    }
                                }
                            );
                            for (handle, name) in [(yes, "forwardFull"), (no, "view")] {
                                let w::ValueOperation::Read { binder: target } =
                                    &value(declaration, owner, handle).operation
                                else {
                                    panic!("original object read")
                                };
                                assert_eq!(binder(declaration, owner, target).name, name);
                            }
                            forms[1] += 1;
                        }
                        c::ValueKind::Query {
                            op,
                            binder: name,
                            domain,
                            body,
                            ..
                        } => {
                            let (expected, counter) = match op.value {
                                c::QueryOp::Filter => (w::Query::Filter, 2),
                                c::QueryOp::Map => (w::Query::Map, 3),
                                c::QueryOp::Sum => (w::Query::Sum, 4),
                                c::QueryOp::Count => panic!("fixture has no count"),
                            };
                            let w::ValueOperation::Query {
                                operator,
                                binder: target,
                                collection,
                                body: result,
                                ..
                            } = &retained.operation
                            else {
                                panic!("original query graph")
                            };
                            assert_eq!(*operator, expected);
                            assert_eq!(binder(declaration, owner, target).name, name.value);
                            assert_original(collection, *domain);
                            assert_original(result, *body);
                            forms[counter] += 1;
                        }
                        _ => {}
                    }
                }
                assert_eq!(forms, [5, 1, 1, 1, 2]);

                let full_recovery = &named("recoveredFull").anchor;
                let partial_recovery = &named("recoveredPartial").anchor;
                let mut full_anchors = vec![full.registration_anchor.index, full_recovery.index];
                if reads_capture {
                    full_anchors.push(view.anchor.index);
                }
                for (compensation, anchors) in [
                    (full, full_anchors),
                    (
                        partial,
                        vec![
                            partial.registration_anchor.index,
                            partial_recovery.index,
                            view.anchor.index,
                        ],
                    ),
                ] {
                    let mut actual = Vec::new();
                    for at in &compensation.recovery_bindings {
                        let requirement = &declaration.bindings[*at as usize];
                        if let w::Subject::Declaration {
                            declaration: original_owner,
                        } = requirement.subject
                        {
                            assert_eq!(original_owner, owner);
                            let closure = match requirement.kind {
                                w::BindingKind::Population => false,
                                w::BindingKind::Closure => true,
                                _ => panic!("expected a nominal population pair"),
                            };
                            let export = requirement
                                .model
                                .0
                                .as_ref()
                                .expect("actual ObjectRole population");
                            assert_eq!(
                                package.models[export.model as usize].exports
                                    [export.export as usize]
                                    .path,
                                ["Node", "nodes"]
                            );
                            nominal(package, requirement.value_type.0.unwrap(), "Node");
                            actual.push((requirement.anchor.index, closure));
                        }
                    }
                    let mut expected: Vec<_> = anchors
                        .into_iter()
                        .flat_map(|anchor| [(anchor, false), (anchor, true)])
                        .collect();
                    actual.sort_unstable();
                    expected.sort_unstable();
                    assert_eq!(
                        actual, expected,
                        "{} reads capture {reads_capture}: exact original anchor membership",
                        compensation.name
                    );
                }
                let emitted = native::emit(&admitted, Limits::default())
                    .into_result()
                    .unwrap();
                let read = inputs.read(proofs, &emitted);
                assert!(
                    read.result().is_ok(),
                    "independent reader: {:?}; locus {:?}",
                    read.result().err(),
                    read.locus()
                );
                assert_eq!(read.into_result().unwrap().package(), package);
            },
        );
    }
}

#[test]
#[trace(
    "TC-121",
    "TC-134",
    "FR-042-AC-4",
    "FR-042-AC-6",
    "FR-042-AC-7",
    "FR-048-AC-6",
    "FR-048-AC-7",
    "FR-048-AC-8"
)]
fn native_compensation_keeps_registration_activation_retry_and_authored_recovery_relations() {
    let (inputs, expected_operation) = inputs(&source());
    inputs.with_proofs(TypeLimits::default(), proofs::ProofLimits::default(), |proofs, selected| {
        discharged(proofs);
        let admitted = native::admit(proofs, selected, Limits::default()).into_result().expect("actual compensation source family");
        let package = admitted.package();
        let owner = package.declarations.iter().position(|declaration| declaration.name == "RecoveryFlow").unwrap() as u32;
        let declaration = &package.declarations[owner as usize];
        let w::Body::Protocol { compensations, controls, roles, temporal_requirements, .. } = &declaration.body else { panic!("protocol family") };
        assert_eq!(compensations.len(), 2, "distinct obligations may share one forward effect");
        assert_eq!(compensations.iter().map(|value| value.name.as_str()).collect::<Vec<_>>(), ["Full", "Partial"]);
        assert_eq!(compensations[0].forward_effect, compensations[1].forward_effect);
        assert_eq!(temporal_requirements.len(), 1);
        assert_eq!(package.declarations[temporal_requirements[0] as usize].name, "Due");
        let namespace = proofs.types().binding().namespace();
        let id = flow(proofs);
        let typed = proofs.types().declaration(id).unwrap();
        let unit = namespace.unit(typed.unit()).unwrap();
        let scope = proofs.types().binding().scopes().unwrap().declaration(id).unwrap();
        let c::DeclarationKind::Protocol(original) = &namespace.syntax(id).unwrap().kind else { panic!("original protocol AST") };
        let authored: Vec<_> = original.requirements.iter().enumerate().filter_map(|(index, requirement)| match requirement {
            c::ProtocolRequirement::Compensation(value) => Some((index, value.as_ref())),
            c::ProtocolRequirement::Temporal { .. } => None,
        }).collect();
        assert_eq!(authored.iter().map(|(index, _)| *index).collect::<Vec<_>>(), [0, 2]);
        let mut instance_indices = Vec::new();
        for (compact, (compensation, (requirement_index, original))) in compensations.iter().zip(authored).enumerate() {
            let subject = w::Subject::Compensation { compensation: w::Handle { declaration: owner, index: compact as u32 } };
            assert_eq!(compensation.operation, expected_operation);
            assert_eq!(package.definitions[compensation.profile as usize].identity, R::EventPosition.identity());
            assert_eq!((integer(&compensation.within.lower), integer(&compensation.within.upper), integer(&compensation.maximum_attempts)), (0, 30, 3));
            assert_eq!(compensation.locus.source, declaration.locus.source);
            assert_eq!((compensation.locus.span.start as usize, compensation.locus.span.end as usize), (original.span.start, original.span.end));
            assert_eq!(controls[compensation.forward_effect.index as usize].name, "Applied");
            let w::ControlOperation::Event { event: w::Event::Effect { instance: forward_instance, .. }, .. } = &controls[compensation.forward_effect.index as usize].operation else { panic!("successful forward effect") };
            let registration = &declaration.bindings[compensation.registration_instance as usize];
            assert_eq!(registration.kind, w::BindingKind::CompensationRegistration);
            assert_eq!(registration.subject, subject);
            let mut prerequisites = vec![*forward_instance, roles[compensation.owner.index as usize].instance];
            prerequisites.sort_unstable();
            assert_eq!(registration.requires, prerequisites);
            let registration_anchor = &declaration.anchors[compensation.registration_anchor.index as usize];
            let activation_anchor = &declaration.anchors[compensation.activation_anchor.index as usize];
            assert_ne!(compensation.registration_anchor, compensation.activation_anchor);
            assert_eq!(registration_anchor.kind, w::AnchorKind::Registration);
            assert_eq!(activation_anchor.kind, w::AnchorKind::CompensationActivation);
            for anchor in [registration_anchor, activation_anchor] {
                assert_eq!(anchor.owner.0, Some(w::Handle { declaration: owner, index: compact as u32 }));
            }
            assert_eq!(registration_anchor.binding.0, Some(compensation.registration_instance));
            let activation_instance = activation_anchor.binding.0.expect("activation has its own observation requirement");
            let activation = &declaration.bindings[activation_instance as usize];
            assert_eq!(activation.kind, w::BindingKind::Observation);
            assert_eq!(activation.subject, subject);
            assert_eq!(activation.requires, [compensation.registration_instance]);
            assert_eq!(declaration.bindings[compensation.clock as usize].kind, w::BindingKind::Clock);
            assert_ne!(compensation.earlier, compensation.later);
            nominal(package, compensation.attempt_type, "Node");
            for (handle, kind) in [(&compensation.earlier, w::BinderKind::EarlierAttempt), (&compensation.later, w::BinderKind::LaterAttempt)] {
                let record = binder(declaration, owner, handle);
                assert_eq!(record.kind, kind);
                assert_eq!(record.value_type, compensation.attempt_type);
                assert_eq!(declaration.anchors[record.anchor.index as usize].kind, w::AnchorKind::Retry);
            }
            for (at, kind) in [(compensation.attempt_instance, w::BindingKind::CompensationAttempt), (compensation.effect_instance, w::BindingKind::CompensationEffect)] {
                assert_eq!(declaration.bindings[at as usize].kind, kind);
                assert_eq!(declaration.bindings[at as usize].subject, subject);
            }
            let attempt = &declaration.bindings[compensation.attempt_instance as usize];
            let effect = &declaration.bindings[compensation.effect_instance as usize];
            assert_eq!(attempt.value_type.0, Some(compensation.attempt_type));
            assert_eq!(attempt.model.0.as_ref(), Some(&expected_operation));
            let mut attempt_dependencies = vec![roles[compensation.owner.index as usize].instance, activation_instance];
            attempt_dependencies.sort_unstable();
            assert_eq!(attempt.requires, attempt_dependencies);
            assert_eq!(declaration.anchors[attempt.anchor.index as usize].kind, w::AnchorKind::Retry);
            assert_eq!(declaration.anchors[attempt.anchor.index as usize].binding.0, Some(compensation.attempt_instance));
            assert!(effect.value_type.0.is_none(), "source declares effect identity, not an inferred payload");
            assert_eq!(effect.model.0.as_ref(), Some(&expected_operation));
            assert_eq!(effect.requires, [compensation.attempt_instance]);
            assert_eq!(effect.anchor, attempt.anchor);
            instance_indices.extend([compensation.registration_instance, activation_instance, compensation.attempt_instance, compensation.effect_instance]);
            assert_eq!(compensation.registration_captures.len(), 1);
            assert_eq!(compensation.activation_captures.len(), 1);
            for (handle, capture, wire_anchor, original_anchor) in [
                (&compensation.registration_captures[0], &original.registration_captures[0], &compensation.registration_anchor, Anchor::Registration(requirement_index)),
                (&compensation.activation_captures[0], &original.activation_captures[0], &compensation.activation_anchor, Anchor::CompensationActivation(requirement_index)),
            ] {
                let retained = binder(declaration, owner, handle);
                assert_eq!(retained.kind, w::BinderKind::Capture);
                assert_eq!(retained.name, capture.parameter.name.value);
                assert_eq!(&retained.anchor, wire_anchor);
                let (original_index, original_binder) = scope.binders.iter().enumerate().find(|(_, binder)| binder.span == capture.parameter.name.span).unwrap();
                assert_eq!(original_binder.anchor, original_anchor);
                assert_eq!(typed.binders().iter().find(|binder| binder.binder == original_index).unwrap().anchor, original_anchor);
                let initializer = value(declaration, owner, retained.initializer.0.as_ref().expect("authored capture initializer"));
                let original_expression = unit.expression(capture.value).unwrap();
                assert_eq!(&unit.expressions()[initializer.original_expression as usize], original_expression);
                assert_eq!(initializer.origin, w::Origin::Anchor { anchor: wire_anchor.clone() });
            }
            nominal(package, binder(declaration, owner, &compensation.registration_captures[0]).value_type, "Total");
            assert!(matches!(package.types[binder(declaration, owner, &compensation.activation_captures[0]).value_type as usize], w::Type::Boolean {}));
            for (handle, expression) in [(&compensation.guard, original.guard), (&compensation.retry, original.retry), (&compensation.recover, original.recover)] {
                let retained = value(declaration, owner, handle);
                assert_eq!(&unit.expressions()[retained.original_expression as usize], unit.expression(expression).unwrap());
                assert!(matches!(package.types[retained.value_type as usize], w::Type::Boolean {}));
            }
            let recovered = binder(declaration, owner, &compensation.recovery);
            assert_eq!(recovered.kind, w::BinderKind::Recovery);
            assert_eq!(declaration.anchors[recovered.anchor.index as usize].kind, w::AnchorKind::Recovery);
            nominal(package, recovered.value_type, "Node");
            assert!(!compensation.recovery_bindings.is_empty(), "recovery view keeps its static completeness authorities");
            for at in &compensation.recovery_bindings {
                let requirement = &declaration.bindings[*at as usize];
                match &requirement.subject {
                    w::Subject::Compensation { .. } => {
                        assert_eq!(requirement.subject, subject);
                        assert_eq!(requirement.anchor, recovered.anchor);
                    }
                    w::Subject::Declaration { declaration: original_owner } => {
                        assert_eq!(*original_owner, owner);
                        assert!(matches!(requirement.kind, w::BindingKind::Population | w::BindingKind::Closure));
                        assert!([&recovered.anchor, &compensation.registration_anchor, &compensation.activation_anchor].contains(&&requirement.anchor), "population authority retains the actual recovery or capture origin");
                        if requirement.kind == w::BindingKind::Closure {
                            let [population] = requirement.requires.as_slice() else { panic!("one selected population closure premise") };
                            assert!(compensation.recovery_bindings.contains(population));
                            let population = &declaration.bindings[*population as usize];
                            assert_eq!(population.kind, w::BindingKind::Population);
                            assert_eq!(population.anchor, requirement.anchor);
                            assert_eq!(population.model, requirement.model);
                            assert_eq!(population.value_type, requirement.value_type);
                        }
                    }
                    _ => panic!("recovery requires its compensation and declaration population authorities"),
                }
            }
            let recovery_requirement = |kind| {
                let matches: Vec<_> = compensation.recovery_bindings.iter().filter(|at| declaration.bindings[**at as usize].kind == kind && declaration.bindings[**at as usize].subject == subject).copied().collect();
                assert_eq!(matches.len(), 1, "one exact {kind:?} recovery authority");
                matches[0]
            };
            let snapshot = recovery_requirement(w::BindingKind::Snapshot);
            assert_eq!(declaration.bindings[snapshot as usize].value_type.0, Some(recovered.value_type));
            assert_eq!(declaration.bindings[snapshot as usize].requires, [activation_instance]);
            let mut completeness_dependencies = vec![compensation.clock, snapshot, compensation.effect_instance];
            completeness_dependencies.sort_unstable();
            for kind in [w::BindingKind::Progress, w::BindingKind::Closure] {
                assert_eq!(declaration.bindings[recovery_requirement(kind) as usize].requires, completeness_dependencies);
            }
            let recovery_population = compensation.recovery_bindings.iter().map(|at| &declaration.bindings[*at as usize]).find(|binding| binding.kind == w::BindingKind::Population && binding.anchor == recovered.anchor).expect("actual recovered Node population is required");
            assert_eq!(recovery_population.value_type.0, Some(recovered.value_type));
            let population = recovery_population.model.0.as_ref().unwrap();
            assert_eq!(package.models[population.model as usize].exports[population.export as usize].path, ["Node", "nodes"]);
        }
        let count = instance_indices.len();
        instance_indices.sort_unstable();
        instance_indices.dedup();
        assert_eq!(instance_indices.len(), count, "two obligations keep distinct registration/activation/attempt/effect identities");
        let committed = compensations[0].commit.0.as_ref().unwrap();
        assert_eq!(controls[committed.index as usize].name, "Committed");
        assert!(compensations[1].commit.0.is_none(), "authored never remains explicit");
        let full = value(declaration, owner, &compensations[0].recover);
        let partial = value(declaration, owner, &compensations[1].recover);
        assert!(matches!(full.operation, w::ValueOperation::Binary { operator: w::Binary::And, .. }));
        assert!(matches!(partial.operation, w::ValueOperation::Let { .. }));
        assert_ne!(compensations[0].recover, compensations[1].recover);
        let sums: Vec<_> = declaration.values.iter().filter_map(|value| match &value.operation { w::ValueOperation::Query { operator: w::Query::Sum, result, .. } => Some(*result), _ => None }).collect();
        assert_eq!(sums.len(), 2);
        for result in sums { nominal(package, result, "Total"); }
        let recovered_event = controls.iter().find(|control| control.name == "Recovered").unwrap();
        assert!(matches!(&recovered_event.operation, w::ControlOperation::Event { event: w::Event::Event { compensation: w::Nullable(Some(target)), .. }, .. } if target.declaration == owner && target.index == 0));
        let emitted = native::emit(&admitted, Limits::default()).into_result().unwrap();
        let read = inputs.read(proofs, &emitted).into_result().expect("independent reader uses original model/source/definition selections");
        assert_eq!(read.package(), package);
        assert_eq!(read.digest(), emitted.digest());
    });
}

/// Tracing: TC-134; ACs: FR-048-AC-8, FR-050-AC-1.
#[trace("TC-134", "FR-048-AC-8", "FR-050-AC-1")]
#[test]
fn timed_compensation_is_admitted_and_reread_only_as_authenticated_v2() {
    let (inputs, _) = inputs(&source());
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let namespace = proofs.types().binding().namespace();
            let [due] = namespace.lookup("Due") else {
                panic!("one authored recovery temporal declaration")
            };
            let producer_span = namespace.syntax(*due).expect("Due syntax").span;
            let producer_span = w::Span {
                start: producer_span.start as u32,
                end: producer_span.end as u32,
            };
            let definition = R::EventPosition;
            let definition_artifact = selected
                .dependencies
                .iter()
                .find(|dependency| {
                    dependency.artifact.identity == definition.identity()
                        && dependency.bytes == definition_bytes(definition)
                })
                .expect("registered event-position definition")
                .artifact;
            let producer_revision = w::Revision {
                namespace: selected.definition_revision_namespace.into(),
                value: definition.revision().into(),
            };
            let producer_clock = v2::wire::ClockConfiguration::EventPosition {
                sequence_authority: "recovery-events".into(),
            };
            let temporal = [native::TemporalSelection {
                source: &inputs.source_references[1],
                span: &producer_span,
                definition_identity: definition.identity(),
                definition_revision: &producer_revision,
                definition_artifact,
                clock: &producer_clock,
            }];
            let emitted = native::admit_v2(proofs, selected, &temporal, Limits::default())
                .into_result()
                .expect("timed recovery emits only through strict v2");

            // Re-select from the authored source and definition catalog rather
            // than projecting the producer's temporal selection record.
            let expected = [inputs.temporal_expectation(
                proofs,
                1,
                "Due",
                TemporalDefinitionExpectation {
                    identity: R::EventPosition.identity().into(),
                    revision: w::Revision {
                        namespace: selected.definition_revision_namespace.into(),
                        value: R::EventPosition.revision().into(),
                    },
                    artifact: selected
                        .dependencies
                        .iter()
                        .find(|dependency| {
                            dependency.artifact.identity == R::EventPosition.identity()
                                && dependency.bytes == definition_bytes(R::EventPosition)
                        })
                        .expect("independently selected event-position definition")
                        .artifact
                        .clone(),
                    clock: v2::wire::ClockConfiguration::EventPosition {
                        sequence_authority: "recovery-events".into(),
                    },
                },
            )];
            let admitted = inputs
                .read_v2(proofs, &emitted, &expected)
                .into_result()
                .expect("independent strict-v2 recovery read");
            let recovery = admitted
                .package()
                .inherited
                .declarations
                .iter()
                .find(|declaration| declaration.name == "RecoveryFlow")
                .expect("recovery protocol");
            let w::Body::Protocol {
                compensations,
                temporal_requirements,
                ..
            } = &recovery.body
            else {
                panic!("recovery protocol body")
            };
            assert_eq!(compensations.len(), 2);
            assert_eq!(temporal_requirements.len(), 1);
            assert_eq!(admitted.package().temporal_bindings.len(), 1);
            let binding = &admitted.package().temporal_bindings[0];
            assert_eq!(binding.declaration, temporal_requirements[0]);
            assert_eq!(
                binding.definition,
                admitted.package().inherited.declarations[temporal_requirements[0] as usize]
                    .profile
            );
            let owner = admitted
                .inherited()
                .declarations
                .iter()
                .position(|declaration| declaration.name == "RecoveryFlow")
                .and_then(|index| u32::try_from(index).ok())
                .expect("recovery declaration fits the wire handle domain");
            for compensation in compensations {
                let initialized_captures = compensation
                    .registration_captures
                    .iter()
                    .chain(&compensation.activation_captures)
                    .collect::<Vec<_>>();
                for value in [
                    compensation.guard.clone(),
                    compensation.retry.clone(),
                    compensation.recover.clone(),
                ] {
                    let report = state::evaluate_v2(
                        &admitted,
                        EvaluationRequest {
                            declaration: owner,
                            value,
                        },
                        &StateView::default(),
                        StateLimits::default(),
                    );
                    match report.outcome() {
                        EvaluationOutcome::Refused(Refusal::MissingBinding(missing)) => {
                            assert!(
                                !initialized_captures.contains(&missing),
                                "initialized capture {missing:?} must be resolved from its authored initializer"
                            );
                        }
                        other => panic!("compensation expression must request only an external input: {other:?}"),
                    }
                }
                for capture in initialized_captures {
                    let report = state::evaluate_v2(
                        &admitted,
                        EvaluationRequest {
                            declaration: owner,
                            value: compensation.guard.clone(),
                        },
                        &StateView {
                            binders: vec![BinderInput {
                                binder: capture.clone(),
                                requirement: None,
                                authority: None,
                                // Input admission must reject the capture before
                                // this deliberately arbitrary payload can be read.
                                value: InputSlot::Available(Value::new(0, ValueKind::Boolean(true))),
                            }],
                            populations: Vec::new(),
                        },
                        StateLimits::default(),
                    );
                    assert_eq!(
                        report.outcome(),
                        &EvaluationOutcome::Refused(Refusal::SurplusBinding(capture.clone())),
                        "an initialized capture is evaluator-owned and cannot be overridden"
                    );
                }
            }
        },
    );
}

#[test]
#[trace(
    "TC-121",
    "TC-134",
    "FR-036-AC-4",
    "FR-042-AC-6",
    "FR-042-AC-8",
    "FR-048-AC-7"
)]
fn compensation_capture_stages_cannot_read_later_or_expired_bindings() {
    for (from, to, name) in [
        (
            "targetFull: M::Total = forwardFull.total",
            "targetFull: M::Total = recoveredFull.total",
            "recoveredFull",
        ),
        (
            "when { not triggerFull.ready }",
            "when { forwardFull.total = targetFull }",
            "forwardFull",
        ),
        (
            "targetFull >= 0 and not activatedFull",
            "targetFull >= 0 and triggerFull.ready",
            "triggerFull",
        ),
        (
            "refundFull) = targetFull and not activatedFull",
            "refundFull) = targetFull and earlierFull.tally >= 0",
            "earlierFull",
        ),
    ] {
        let (inputs, _) = inputs(&changed(from, to));
        inputs.with_proofs(
            TypeLimits::default(),
            proofs::ProofLimits::default(),
            |proofs, selected| {
                let id = flow(proofs);
                let scope = proofs
                    .types()
                    .binding()
                    .scopes()
                    .unwrap()
                    .declaration(id)
                    .unwrap();
                let span = scope
                    .issues
                    .iter()
                    .find_map(|issue| match issue {
                        ScopeIssue::OutOfScope {
                            name: actual, span, ..
                        } if actual == name => Some(*span),
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("{name}: {:?}", scope.issues));
                let unit = proofs
                    .types()
                    .binding()
                    .namespace()
                    .unit(scope.unit)
                    .unwrap();
                assert_eq!(unit.source().slice(span), Some(name));
                assert_eq!(
                    proofs.types().disposition(id),
                    Some(TypeDisposition::Refused)
                );
                assert!(proofs
                    .types()
                    .declaration(id)
                    .unwrap()
                    .causes()
                    .iter()
                    .any(|cause| cause.kind == TypeCause::UpstreamBinding));
                assert_eq!(
                    proofs.declaration(id).unwrap().disposition(),
                    proofs::ProofDisposition::Refused
                );
                failure(
                    &native::admit(proofs, selected, Limits::default()),
                    Error::Unsupported(Unsupported::FamilyProof),
                );
            },
        );
    }
}

#[test]
#[trace("TC-121", "TC-134", "FR-036-AC-4", "FR-042-AC-6", "FR-048-AC-7")]
fn compensation_requires_the_exact_effect_and_commit_control_kinds() {
    for (from, to, kind, selected_name) in [
        (
            "Full for Main::Applied",
            "Full for Main::Tried",
            StructuralKind::Effect,
            "Main::Tried",
        ),
        (
            "commit Main::Committed",
            "commit Main::Applied",
            StructuralKind::Commit,
            "Main::Applied",
        ),
    ] {
        let (inputs, _) = inputs(&changed(from, to));
        inputs.with_proofs(
            TypeLimits::default(),
            proofs::ProofLimits::default(),
            |proofs, selected| {
                let id = flow(proofs);
                let scope = proofs
                    .types()
                    .binding()
                    .scopes()
                    .unwrap()
                    .declaration(id)
                    .unwrap();
                let reference = scope
                    .issues
                    .iter()
                    .find_map(|issue| match issue {
                        ScopeIssue::WrongTargetKind { reference, .. }
                            if scope.references[*reference].required == kind =>
                        {
                            Some(&scope.references[*reference])
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("{:?}", scope.issues));
                let unit = proofs
                    .types()
                    .binding()
                    .namespace()
                    .unit(scope.unit)
                    .unwrap();
                assert_eq!(unit.source().slice(reference.span), Some(selected_name));
                assert_eq!(
                    proofs.types().disposition(id),
                    Some(TypeDisposition::Refused)
                );
                failure(
                    &native::admit(proofs, selected, Limits::default()),
                    Error::Unsupported(Unsupported::FamilyProof),
                );
            },
        );
    }
}

#[test]
#[trace("TC-121", "FR-042-AC-3", "FR-042-AC-6")]
fn compensation_refuses_zero_attempts_reversed_deadlines_and_foreign_attempt_types() {
    for (from, to, expected) in [
        (
            "within [0,30]; attempts 3 of M::Node;\n          retry (earlierFull",
            "within [0,30]; attempts 0 of M::Node;\n          retry (earlierFull",
            Invalid::NumericDomain,
        ),
        (
            "within [0,30]; attempts 3 of M::Node;\n          retry (earlierFull",
            "within [31,30]; attempts 3 of M::Node;\n          retry (earlierFull",
            Invalid::NumericDomain,
        ),
        (
            "within [0,30]; attempts 3 of M::Node;\n          retry (earlierFull",
            "within [0,30]; attempts 3 of M::Plain;\n          retry (earlierFull",
            Invalid::Type,
        ),
    ] {
        let (inputs, _) = inputs(&changed(from, to));
        inputs.with_proofs(
            TypeLimits::default(),
            proofs::ProofLimits::default(),
            |proofs, selected| {
                discharged(proofs);
                failure(
                    &native::admit(proofs, selected, Limits::default()),
                    Error::Invalid(expected),
                );
            },
        );
    }
}

#[test]
#[trace("TC-121", "FR-040-AC-5", "FR-042-AC-6", "FR-042-AC-8")]
fn an_unsafe_retry_relation_cannot_borrow_activation_or_runtime_attempt_evidence() {
    let (inputs, _) = inputs(&changed("earlierFull.tally < 3 and", "true and"));
    inputs.with_proofs(TypeLimits::default(), proofs::ProofLimits::default(), |proofs, selected| {
        let id = flow(proofs);
        assert_eq!(proofs.types().disposition(id), Some(TypeDisposition::Typed));
        let proof = proofs.declaration(id).unwrap();
        assert_eq!(proof.disposition(), proofs::ProofDisposition::Refused);
        let cause = proof.causes().iter().find(|cause| matches!(&cause.kind, proofs::CauseKind::Unproved { diagnostics } if diagnostics.iter().any(|diagnostic| diagnostic.obligation_kind == Some(ir::DefinednessObligationKind::CheckedRange)))).unwrap_or_else(|| panic!("{:?}", proof.causes()));
        let unit = proofs.types().binding().namespace().unit(cause.site.unit).unwrap();
        assert_eq!(unit.source().slice(cause.site.span), Some("earlierFull.tally + 1"));
        assert_eq!(cause.site.declaration, id);
        assert_eq!(unit.expression(cause.site.expression.unwrap()).unwrap().span, cause.site.span);
        failure(&native::admit(proofs, selected, Limits::default()), Error::Unsupported(Unsupported::FamilyProof));
    });
}

#[test]
#[trace("TC-121", "FR-042-AC-6", "FR-042-AC-7")]
fn identity_only_compensation_effects_keep_exact_operation_obligation_and_attempt_authority() {
    let (inputs, _) = inputs(&source());
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let admitted = native::admit(proofs, selected, Limits::default())
                .into_result()
                .expect("actual native compensation baseline");
            let emitted = native::emit(&admitted, Limits::default())
                .into_result()
                .unwrap();
            inputs
                .read(proofs, &emitted)
                .into_result()
                .expect("original independently selected baseline");
            let package = admitted.package();
            let owner = package
                .declarations
                .iter()
                .position(|declaration| declaration.name == "RecoveryFlow")
                .unwrap();
            let declaration = &package.declarations[owner];
            let w::Body::Protocol {
                compensations,
                controls,
                ..
            } = &declaration.body
            else {
                panic!("protocol")
            };
            let full = &compensations[0];
            let partial = &compensations[1];
            let original = &declaration.bindings[full.effect_instance as usize];
            assert!(original.value_type.0.is_none());
            assert_eq!(original.model.0.as_ref(), Some(&full.operation));
            let w::Type::Object {
                export: object_export,
            } = &package.types[full.attempt_type as usize]
            else {
                panic!("actual Node Object authority")
            };
            let mut missing_model = original.clone();
            missing_model.model.0 = None;
            let mut wrong_export = original.clone();
            wrong_export.model.0 = Some(object_export.clone());
            let mut other_obligation = original.clone();
            other_obligation.subject = declaration.bindings[partial.effect_instance as usize]
                .subject
                .clone();
            let mut other_anchor = original.clone();
            other_anchor.anchor = declaration.bindings[partial.effect_instance as usize]
                .anchor
                .clone();
            let mut other_attempt = original.clone();
            other_attempt.requires = vec![partial.attempt_instance];
            for (name, binding) in [
                ("missing operation authority", missing_model),
                ("object export substituted for operation", wrong_export),
                ("other obligation", other_obligation),
                ("other retry anchor", other_anchor),
                ("other attempt", other_attempt),
            ] {
                let mut offered = package.clone();
                offered.declarations[owner].bindings[full.effect_instance as usize] = binding;
                // A malformed offer derived from actual emission, not a substitute
                // positive producer. Original source/model/dependency selectors stay
                // independent; the offered seal lets this reach binding validation.
                let bytes = serde_json::to_vec(&offered).unwrap();
                let report = inputs.read_bytes(proofs, &bytes, ByteDigest::of(&bytes));
                assert_eq!(
                    report.result().err(),
                    Some(&Error::Invalid(Invalid::Binding)),
                    "{name}; locus {:?}",
                    report.locus()
                );
            }
            let mut offered = package.clone();
            offered.declarations[owner].bindings[full.effect_instance as usize].subject =
                w::Subject::Compensation {
                    compensation: w::Handle {
                        declaration: ((owner + 1) % package.declarations.len()) as u32,
                        index: 0,
                    },
                };
            let bytes = serde_json::to_vec(&offered).unwrap();
            failure(
                &inputs.read_bytes(proofs, &bytes, ByteDigest::of(&bytes)),
                Error::Invalid(Invalid::Owner),
            );
            let w::ControlOperation::Event {
                event:
                    w::Event::Effect {
                        instance: ordinary_effect,
                        ..
                    },
                ..
            } = &controls[full.forward_effect.index as usize].operation
            else {
                panic!("ordinary successful forward effect")
            };
            assert!(declaration.bindings[*ordinary_effect as usize]
                .value_type
                .0
                .is_some());
            let activation = declaration.anchors[full.activation_anchor.index as usize]
                .binding
                .0
                .expect("original compensation activation binding");
            for (name, binding, prerequisite) in [
                (
                    "registration without its forward effect",
                    full.registration_instance,
                    *ordinary_effect,
                ),
                (
                    "activation without registration",
                    activation,
                    full.registration_instance,
                ),
                (
                    "attempt without activation",
                    full.attempt_instance,
                    activation,
                ),
            ] {
                assert!(declaration.bindings[binding as usize]
                    .requires
                    .contains(&prerequisite));
                let mut offered = package.clone();
                offered.declarations[owner].bindings[binding as usize]
                    .requires
                    .retain(|required| *required != prerequisite);
                let bytes = serde_json::to_vec(&offered).unwrap();
                let report = inputs.read_bytes(proofs, &bytes, ByteDigest::of(&bytes));
                assert_eq!(
                    report.result().err(),
                    Some(&Error::Invalid(Invalid::Binding)),
                    "{name}; locus {:?}",
                    report.locus()
                );
            }
            let mut offered = package.clone();
            offered.declarations[owner].bindings[*ordinary_effect as usize]
                .value_type
                .0 = None;
            let bytes = serde_json::to_vec(&offered).unwrap();
            failure(
                &inputs.read_bytes(proofs, &bytes, ByteDigest::of(&bytes)),
                Error::Invalid(Invalid::Binding),
            );
        },
    );
}
#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-6", "FR-042-AC-7")]
fn compensation_await_and_associated_event_keep_original_activation_prerequisites() {
    let body = changed(
        "event Recovered by Service for Full as (recoveryEvent: M::Plain) { recoveryEvent.ready };",
        "await RecoveryWindow after Partial using T clock \"await-recovery\" within [0,5]
            match event RecoveryObserved by Service for Partial as (observedRecovery: M::Plain) { true };
            then check RecoverySeen using S { true };
            timeout check RecoveryExpired using S { true };",
    );
    let (inputs, _) = inputs(&body);
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let admitted = native::admit(proofs, selected, Limits::default())
                .into_result()
                .expect("actual compensation-anchored await and associated domain event");
            let package = admitted.package();
            let owner = package
                .declarations
                .iter()
                .position(|declaration| declaration.name == "RecoveryFlow")
                .unwrap() as u32;
            let declaration = &package.declarations[owner as usize];
            let w::Body::Protocol {
                compensations,
                controls,
                roles,
                causal_edges,
                ..
            } = &declaration.body
            else {
                panic!("original protocol")
            };
            let partial = w::Handle {
                declaration: owner,
                index: 1,
            };
            let selected_compensation = &compensations[partial.index as usize];
            assert_eq!(selected_compensation.name, "Partial");
            let wait = w::Handle {
                declaration: owner,
                index: controls
                    .iter()
                    .position(|control| control.name == "RecoveryWindow")
                    .unwrap() as u32,
            };
            let wait_control = &controls[wait.index as usize];
            let w::ControlOperation::Await {
                after,
                clock,
                within,
                event,
                then_body,
                timeout,
                ..
            } = &wait_control.operation
            else {
                panic!("native await")
            };
            assert_eq!(
                after,
                &w::AwaitAnchor::Compensation {
                    compensation: partial.clone()
                }
            );
            assert_eq!((integer(&within.lower), integer(&within.upper)), (0, 5));
            assert_eq!(controls[event.index as usize].name, "RecoveryObserved");
            assert_eq!(controls[then_body.index as usize].name, "RecoverySeen");
            assert_eq!(controls[timeout.index as usize].name, "RecoveryExpired");

            let namespace = proofs.types().binding().namespace();
            let id = flow(proofs);
            let unit = namespace
                .unit(namespace.declaration(id).unwrap().unit())
                .unwrap();
            let c::DeclarationKind::Protocol(original) = &namespace.syntax(id).unwrap().kind else {
                panic!("native protocol AST")
            };
            let c::ProtocolRequirement::Compensation(authored_partial) = &original.requirements[2]
            else {
                panic!("temporal requirement keeps its separate source index")
            };
            assert_eq!(authored_partial.name.value, "Partial");
            let c::ControlKind::Await {
                after: authored_after,
                clock: authored_clock,
                event: authored_event,
                ..
            } = &unit.controls()[wait_control.original_node as usize].kind
            else {
                panic!("original await occurrence")
            };
            assert_eq!(unit.source().slice(authored_after.span), Some("Partial"));
            assert_eq!(authored_clock.value, "await-recovery");
            let observed = &controls[event.index as usize];
            let original_event = unit.control(*authored_event).unwrap();
            assert_eq!(
                &unit.controls()[observed.original_node as usize],
                original_event
            );
            let c::ControlKind::Event(c::Event {
                kind:
                    c::EventKind::Event {
                        compensation: Some(authored_target),
                        ..
                    },
                ..
            }) = &original_event.kind
            else {
                panic!("authored compensation event association")
            };
            assert_eq!(unit.source().slice(authored_target.span), Some("Partial"));
            assert_eq!(
                (
                    wait_control.locus.span.start as usize,
                    wait_control.locus.span.end as usize
                ),
                (
                    unit.controls()[wait_control.original_node as usize]
                        .span
                        .start,
                    unit.controls()[wait_control.original_node as usize]
                        .span
                        .end
                )
            );
            assert_eq!(observed.locus.source, declaration.locus.source);

            let activation_anchor =
                &declaration.anchors[selected_compensation.activation_anchor.index as usize];
            assert_eq!(activation_anchor.owner.0.as_ref(), Some(&partial));
            assert_eq!(
                activation_anchor.kind,
                w::AnchorKind::CompensationActivation
            );
            let activation = activation_anchor.binding.0.unwrap();
            assert_eq!(
                declaration.bindings[activation as usize].requires,
                [selected_compensation.registration_instance]
            );
            let clock_requirement = &declaration.bindings[*clock as usize];
            assert_eq!(clock_requirement.requires, [activation]);
            assert_eq!(
                clock_requirement.subject,
                w::Subject::Control {
                    control: wait.clone()
                }
            );
            assert_eq!(
                (
                    clock_requirement.locus.span.start as usize,
                    clock_requirement.locus.span.end as usize
                ),
                (authored_clock.span.start, authored_clock.span.end)
            );
            for kind in [w::BindingKind::Progress, w::BindingKind::Closure] {
                let requirement = declaration
                    .bindings
                    .iter()
                    .find(|binding| {
                        binding.kind == kind
                            && binding.subject
                                == w::Subject::Control {
                                    control: wait.clone(),
                                }
                    })
                    .unwrap();
                assert_eq!(requirement.requires, [*clock]);
                assert_eq!(requirement.anchor, clock_requirement.anchor);
            }
            let w::ControlOperation::Event {
                event:
                    w::Event::Event {
                        owner: role,
                        compensation,
                        instance,
                    },
                ..
            } = &observed.operation
            else {
                panic!("bound domain event")
            };
            assert_eq!(compensation.0.as_ref(), Some(&partial));
            assert_eq!(event.declaration, owner);
            let event_requirement = &declaration.bindings[*instance as usize];
            assert_eq!(event_requirement.kind, w::BindingKind::Invocation);
            assert_eq!(
                event_requirement.subject,
                w::Subject::Control {
                    control: event.clone()
                }
            );
            let mut event_dependencies = vec![
                roles[role.index as usize].instance,
                selected_compensation.registration_instance,
            ];
            event_dependencies.sort_unstable();
            assert_eq!(event_requirement.requires, event_dependencies);

            // These are branch alternatives and static prerequisites, not claims
            // that an activation, event, timeout, or recovery actually occurred.
            let edges: Vec<_> = causal_edges
                .iter()
                .filter(|edge| edge.owner == wait)
                .collect();
            assert_eq!(edges.len(), 5);
            for (kind, from, from_port, to, to_port) in [
                (
                    w::EdgeKind::AwaitSuccess,
                    &wait,
                    w::Port::Enter,
                    event,
                    w::Port::Enter,
                ),
                (
                    w::EdgeKind::AwaitSuccess,
                    event,
                    w::Port::Exit,
                    then_body,
                    w::Port::Enter,
                ),
                (
                    w::EdgeKind::AwaitTimeout,
                    &wait,
                    w::Port::Enter,
                    timeout,
                    w::Port::Enter,
                ),
                (
                    w::EdgeKind::Join,
                    then_body,
                    w::Port::Exit,
                    &wait,
                    w::Port::Exit,
                ),
                (
                    w::EdgeKind::Join,
                    timeout,
                    w::Port::Exit,
                    &wait,
                    w::Port::Exit,
                ),
            ] {
                assert!(edges.iter().any(|edge| edge.kind == kind
                    && edge.from.node == *from
                    && edge.from.port == from_port
                    && edge.to.node == *to
                    && edge.to.port == to_port
                    && edge.maximum.0.is_none()));
            }
            let emitted = native::emit(&admitted, Limits::default())
                .into_result()
                .unwrap();
            let read = inputs.read(proofs, &emitted).into_result().expect(
                "independently selected reader accepts the original compensation references",
            );
            assert_eq!(read.package(), package);
            assert_eq!(read.digest(), emitted.digest());
        },
    );
}
#[test]
#[trace("TC-121", "FR-042-AC-6", "FR-042-AC-7", "FR-042-AC-8")]
fn adding_a_payload_type_cannot_bypass_compensation_effect_authority() {
    let (inputs, _) = inputs(&source());
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let admitted = native::admit(proofs, selected, Limits::default())
                .into_result()
                .expect("actual identity-only source baseline");
            let emitted = native::emit(&admitted, Limits::default())
                .into_result()
                .unwrap();
            inputs
                .read(proofs, &emitted)
                .into_result()
                .expect("original null-type effect passes independent admission");
            let package = admitted.package();
            let owner = package
                .declarations
                .iter()
                .position(|declaration| declaration.name == "RecoveryFlow")
                .unwrap();
            let declaration = &package.declarations[owner];
            let w::Body::Protocol { compensations, .. } = &declaration.body else {
                panic!("protocol")
            };
            let full = &compensations[0];
            let effect = &declaration.bindings[full.effect_instance as usize];
            assert_eq!(effect.kind, w::BindingKind::CompensationEffect);
            assert!(effect.value_type.0.is_none());
            assert!(
                effect.relation.0.is_none(),
                "this source selects no typed-effect correspondence"
            );
            assert_eq!(effect.model.0.as_ref(), Some(&full.operation));
            assert!(package
                .definitions
                .iter()
                .any(
                    |definition| definition.identity == R::ObservationBinding.identity()
                        && definition.artifact == effect.contract
                ));
            let trigger_type = declaration.binders[full.trigger.index as usize].value_type;
            assert_ne!(
                trigger_type, full.attempt_type,
                "two actual model views remain distinct"
            );
            for (name, value_type) in [
                ("attempt record", full.attempt_type),
                ("trigger record", trigger_type),
            ] {
                let mut offered = package.clone();
                offered.declarations[owner].bindings[full.effect_instance as usize]
                    .value_type
                    .0 = Some(value_type);
                // This negative offer adds only a type. Its unchanged operation,
                // subject and identity chain cannot establish payload correspondence.
                let bytes = serde_json::to_vec(&offered).unwrap();
                let report = inputs.read_bytes(proofs, &bytes, ByteDigest::of(&bytes));
                assert_eq!(
                    report.result().err(),
                    Some(&Error::Unsupported(Unsupported::Export)),
                    "unauthorized {name}; locus {:?}",
                    report.locus()
                );
            }
            let w::Type::Object { export } = &package.types[full.attempt_type as usize] else {
                panic!("actual Object export")
            };
            let mut offered = package.clone();
            let changed = &mut offered.declarations[owner].bindings[full.effect_instance as usize];
            changed.value_type.0 = Some(full.attempt_type);
            changed.model.0 = Some(export.clone());
            // The offered Object genuinely exists, but it is not the compensating
            // Operation. Adding a payload must not bypass that identity check.
            let bytes = serde_json::to_vec(&offered).unwrap();
            failure(
                &inputs.read_bytes(proofs, &bytes, ByteDigest::of(&bytes)),
                Error::Invalid(Invalid::Binding),
            );
        },
    );
}

#[test]
#[trace(
    "TC-121",
    "TC-134",
    "FR-042-AC-6",
    "FR-042-AC-7",
    "FR-048-AC-7",
    "FR-048-AC-8"
)]
fn compensation_clocks_and_recovery_premises_cannot_cross_obligations_or_lose_edges() {
    let (inputs, _) = inputs(&source());
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let admitted = native::admit(proofs, selected, Limits::default())
                .into_result()
                .expect("actual two-obligation recovery baseline");
            let emitted = native::emit(&admitted, Limits::default())
                .into_result()
                .unwrap();
            inputs
                .read(proofs, &emitted)
                .into_result()
                .expect("original authority edges pass independent admission");
            let package = admitted.package();
            let owner = package
                .declarations
                .iter()
                .position(|declaration| declaration.name == "RecoveryFlow")
                .unwrap();
            let declaration = &package.declarations[owner];
            let w::Body::Protocol { compensations, .. } = &declaration.body else {
                panic!("protocol")
            };
            let [full, partial] = compensations.as_slice() else {
                panic!("two authored obligations")
            };
            assert_eq!(
                (full.name.as_str(), partial.name.as_str()),
                ("Full", "Partial")
            );
            let requirement = |compensation: &w::Compensation, index: u32, kind| {
                let subject = w::Subject::Compensation {
                    compensation: w::Handle {
                        declaration: owner as u32,
                        index,
                    },
                };
                let matches: Vec<_> = compensation
                    .recovery_bindings
                    .iter()
                    .copied()
                    .filter(|at| {
                        let binding = &declaration.bindings[*at as usize];
                        binding.kind == kind && binding.subject == subject
                    })
                    .collect();
                assert_eq!(matches.len(), 1, "one original {kind:?} per obligation");
                matches[0]
            };
            let snapshot = requirement(full, 0, w::BindingKind::Snapshot);
            let activation = declaration.anchors[full.activation_anchor.index as usize]
                .binding
                .0
                .unwrap();
            assert_eq!(
                declaration.bindings[snapshot as usize].requires,
                [activation]
            );
            assert_ne!(full.clock, partial.clock);
            for compensation in [full, partial] {
                assert_eq!(
                    declaration.bindings[compensation.clock as usize].kind,
                    w::BindingKind::Clock
                );
            }
            for (name, full_clock, partial_clock) in [
                ("Full selects Partial clock", partial.clock, partial.clock),
                ("both clock selections exchanged", partial.clock, full.clock),
            ] {
                let mut offered = package.clone();
                let w::Body::Protocol { compensations, .. } = &mut offered.declarations[owner].body
                else {
                    panic!("protocol")
                };
                compensations[0].clock = full_clock;
                compensations[1].clock = partial_clock;
                let bytes = serde_json::to_vec(&offered).unwrap();
                let report = inputs.read_bytes(proofs, &bytes, ByteDigest::of(&bytes));
                assert_eq!(
                    report.result().err(),
                    Some(&Error::Invalid(Invalid::Binding)),
                    "{name}; locus {:?}",
                    report.locus()
                );
            }
            for kind in [
                w::BindingKind::Snapshot,
                w::BindingKind::Progress,
                w::BindingKind::Closure,
            ] {
                let original = requirement(full, 0, kind);
                let foreign = requirement(partial, 1, kind);
                assert_ne!(original, foreign);
                assert!(!full.recovery_bindings.contains(&foreign));
                let mut offered = package.clone();
                let w::Body::Protocol { compensations, .. } = &mut offered.declarations[owner].body
                else {
                    panic!("protocol")
                };
                let slots = &mut compensations[0].recovery_bindings;
                let positions: Vec<_> = slots
                    .iter()
                    .enumerate()
                    .filter(|(_, at)| **at == original)
                    .map(|(index, _)| index)
                    .collect();
                assert_eq!(positions.len(), 1, "original edge exists exactly once");
                slots[positions[0]] = foreign;
                slots.sort_unstable();
                let bytes = serde_json::to_vec(&offered).unwrap();
                let report = inputs.read_bytes(proofs, &bytes, ByteDigest::of(&bytes));
                assert_eq!(
                    report.result().err(),
                    Some(&Error::Invalid(Invalid::Binding)),
                    "Full selects Partial {kind:?}; locus {:?}",
                    report.locus()
                );
            }
            let population_pair = |compensation: &w::Compensation| {
                let recovered = &declaration.binders[compensation.recovery.index as usize];
                let subject = w::Subject::Declaration {
                    declaration: owner as u32,
                };
                let populations: Vec<_> = compensation
                    .recovery_bindings
                    .iter()
                    .copied()
                    .filter(|at| {
                        let binding = &declaration.bindings[*at as usize];
                        binding.kind == w::BindingKind::Population
                            && binding.subject == subject
                            && binding.anchor == recovered.anchor
                    })
                    .collect();
                assert_eq!(populations.len(), 1, "one actual recovered Node population");
                let population_index = populations[0];
                let population = &declaration.bindings[population_index as usize];
                assert_eq!(population.value_type.0, Some(recovered.value_type));
                let model = population
                    .model
                    .0
                    .as_ref()
                    .expect("actual population export");
                assert_eq!(
                    package.models[model.model as usize].exports[model.export as usize].path,
                    ["Node", "nodes"]
                );
                let closures: Vec<_> = compensation
                    .recovery_bindings
                    .iter()
                    .copied()
                    .filter(|at| {
                        let binding = &declaration.bindings[*at as usize];
                        binding.kind == w::BindingKind::Closure
                            && binding.subject == subject
                            && binding.anchor == recovered.anchor
                            && binding.value_type == population.value_type
                            && binding.model == population.model
                            && binding.requires == [population_index]
                    })
                    .collect();
                assert_eq!(closures.len(), 1, "one exact recovered population closure");
                [population_index, closures[0]]
            };
            let full_pair = population_pair(full);
            let partial_pair = population_pair(partial);
            assert_ne!(
                declaration.binders[full.recovery.index as usize].anchor,
                declaration.binders[partial.recovery.index as usize].anchor,
                "the two actual recovery populations have distinct anchors"
            );
            for member in partial_pair {
                assert!(!full.recovery_bindings.contains(&member));
            }

            for member in full_pair {
                let mut offered = package.clone();
                let w::Body::Protocol { compensations, .. } = &mut offered.declarations[owner].body
                else {
                    panic!("protocol")
                };
                let slots = &mut compensations[0].recovery_bindings;
                assert_eq!(slots.iter().filter(|at| **at == member).count(), 1);
                slots.retain(|at| *at != member);
                let bytes = serde_json::to_vec(&offered).unwrap();
                let report = inputs.read_bytes(proofs, &bytes, ByteDigest::of(&bytes));
                assert_eq!(
                    report.result().err(),
                    Some(&Error::Invalid(Invalid::Binding)),
                    "Full omits required population member {member}; locus {:?}",
                    report.locus()
                );
            }

            let mut offered = package.clone();
            let w::Body::Protocol { compensations, .. } = &mut offered.declarations[owner].body
            else {
                panic!("protocol")
            };
            let slots = &mut compensations[0].recovery_bindings;
            slots.extend(partial_pair);
            slots.sort_unstable();
            assert!(slots.windows(2).all(|pair| pair[0] < pair[1]));
            let bytes = serde_json::to_vec(&offered).unwrap();
            let report = inputs.read_bytes(proofs, &bytes, ByteDigest::of(&bytes));
            assert_eq!(
                report.result().err(),
                Some(&Error::Invalid(Invalid::Binding)),
                "Full adds the genuine Partial-only population pair; locus {:?}",
                report.locus()
            );
            let mut edges = vec![(snapshot, activation)];
            for kind in [w::BindingKind::Progress, w::BindingKind::Closure] {
                let record = requirement(full, 0, kind);
                for prerequisite in [full.clock, full.effect_instance, snapshot] {
                    edges.push((record, prerequisite));
                }
            }
            for (record, prerequisite) in edges {
                assert_eq!(
                    declaration.bindings[record as usize]
                        .requires
                        .iter()
                        .filter(|at| **at == prerequisite)
                        .count(),
                    1,
                    "original prerequisite exists exactly once"
                );
                let mut offered = package.clone();
                offered.declarations[owner].bindings[record as usize]
                    .requires
                    .retain(|at| *at != prerequisite);
                let bytes = serde_json::to_vec(&offered).unwrap();
                let report = inputs.read_bytes(proofs, &bytes, ByteDigest::of(&bytes));
                assert_eq!(
                    report.result().err(),
                    Some(&Error::Invalid(Invalid::Binding)),
                    "missing edge {record}->{prerequisite}; locus {:?}",
                    report.locus()
                );
            }
        },
    );
}
#[test]
#[trace(
    "TC-121",
    "TC-134",
    "FR-042-AC-3",
    "FR-042-AC-6",
    "FR-042-AC-7",
    "FR-048-AC-6"
)]
fn compensation_attempt_bound_preserves_signed64_maximum_and_refuses_one_beyond() {
    let minimum = changed(
        "within [0,30]; attempts 3 of M::Node;\n          retry (earlierFull",
        "within [0,30]; attempts 1 of M::Node;\n          retry (earlierFull",
    );
    let (minimum_inputs, _) = inputs(&minimum);
    minimum_inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let admitted = native::admit(proofs, selected, Limits::default())
                .into_result()
                .expect("one is the smallest admitted compensation-attempt bound");
            let declaration = admitted
                .package()
                .declarations
                .iter()
                .find(|declaration| declaration.name == "RecoveryFlow")
                .unwrap();
            let w::Body::Protocol { compensations, .. } = &declaration.body else {
                panic!("protocol")
            };
            assert_eq!(integer(&compensations[0].maximum_attempts), 1);
        },
    );

    let body = changed(
        "within [0,30]; attempts 3 of M::Node;\n          retry (earlierFull",
        "within [0,30]; attempts 9223372036854775807 of M::Node;\n          retry (earlierFull",
    );
    let (inputs, _) = inputs(&body);
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let admitted = native::admit(proofs, selected, Limits::default())
                .into_result()
                .expect("signed64 maximum is a static authored attempt bound");
            let package = admitted.package();
            let declaration = package
                .declarations
                .iter()
                .find(|declaration| declaration.name == "RecoveryFlow")
                .unwrap();
            let w::Body::Protocol { compensations, .. } = &declaration.body else {
                panic!("protocol")
            };
            let [full, partial] = compensations.as_slice() else {
                panic!("two original obligations")
            };
            assert_eq!(
                (full.name.as_str(), integer(&full.maximum_attempts)),
                ("Full", i64::MAX)
            );
            assert_eq!(
                (partial.name.as_str(), integer(&partial.maximum_attempts)),
                ("Partial", 3)
            );
            let namespace = proofs.types().binding().namespace();
            let id = flow(proofs);
            let c::DeclarationKind::Protocol(original) = &namespace.syntax(id).unwrap().kind else {
                panic!("original protocol")
            };
            let c::ProtocolRequirement::Compensation(authored) = &original.requirements[0] else {
                panic!("original Full obligation")
            };
            let unit = namespace
                .unit(namespace.declaration(id).unwrap().unit())
                .unwrap();
            assert_eq!(authored.attempts.value, "9223372036854775807");
            assert_eq!(
                unit.source().slice(authored.attempts.span),
                Some("9223372036854775807")
            );
            let emitted = native::emit(&admitted, Limits::default())
                .into_result()
                .unwrap();
            let read = inputs
                .read(proofs, &emitted)
                .into_result()
                .expect("independent reader preserves the exact maximum");
            assert_eq!(read.package(), package);
            assert_eq!(read.digest(), emitted.digest());

            // Change exactly this numeric field in the actually emitted bytes. All
            // original source/model/dependency selectors remain independently fixed.
            let text = std::str::from_utf8(emitted.bytes()).unwrap();
            let from = r#""maximum_attempts":{"kind":"integer","decimal":"9223372036854775807"}"#;
            let to = r#""maximum_attempts":{"kind":"integer","decimal":"9223372036854775808"}"#;
            assert_eq!(text.matches(from).count(), 1);
            let offered = text.replacen(from, to, 1);
            failure(
                &inputs.read_bytes(
                    proofs,
                    offered.as_bytes(),
                    ByteDigest::of(offered.as_bytes()),
                ),
                Error::Numeric(artifact::NumberError::ComponentOutOfRange {
                    component: artifact::NumberComponent::Decimal,
                }),
            );
        },
    );
}
