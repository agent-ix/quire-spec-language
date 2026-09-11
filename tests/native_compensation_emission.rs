// SPDX-License-Identifier: AGPL-3.0-only
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
    self as artifact, native, wire as w, Error, Invalid, Limits, ProtocolNumber, Unsupported,
};
use quire_spec_language::syntax::composed as c;
use quire_spec_language::ByteDigest;
use setup::{Inputs, Unit};

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
#[trace("TC-121", "FR-042-AC-4", "FR-042-AC-6", "FR-042-AC-7")]
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

#[test]
#[trace("TC-121", "FR-036-AC-4", "FR-042-AC-6", "FR-042-AC-8")]
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
#[trace("TC-121", "FR-036-AC-4", "FR-042-AC-6")]
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
