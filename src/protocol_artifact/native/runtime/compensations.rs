// SPDX-License-Identifier: AGPL-3.0-only
//! FR-042: original compensation templates and distinct future binding subjects.

use super::{
    binder_type, captures, model_type, nominal, operation_export, parameter_handle, qualified_span,
    selected_profile, structural, Requirement, Runtime,
};
use crate::linking::composed::{
    definition_source::RegisteredDefinition as R,
    scopes::{Anchor, BinderKind, StructuralKind},
};
use crate::protocol_artifact::{
    native::{
        context::Declaration,
        controls,
        types::{integer, text, ValueBuilder},
    },
    wire as w,
    work::Work,
    Dimension, Error, Invalid,
};
use crate::syntax::composed as c;

pub(super) fn lower(
    context: &Declaration<'_, '_>,
    roles: &[w::Role],
    controls: &[w::Control],
    runtime: &mut Runtime<'_, '_>,
    builder: &mut ValueBuilder<'_>,
    work: &mut Work,
) -> Result<Vec<w::Compensation>, Error> {
    let mut result = Vec::new();
    for &original in &context.layout.compensations {
        work.visit()?;
        let value = lower_one(context, roles, controls, original, runtime, builder, work)?;
        work.charge(Dimension::Entries, 1)?;
        result.push(value);
    }
    // Controls may name templates declared before or after their own source
    // occurrences. Bind their already-resolved references after both tables exist.
    for control in controls {
        work.locus = Some(control.locus.clone());
        work.visit()?;
        match &control.operation {
            w::ControlOperation::Event {
                event:
                    w::Event::Event {
                        compensation: w::Nullable(Some(target)),
                        instance,
                        ..
                    },
                ..
            } => {
                let value = result
                    .get(target.index as usize)
                    .ok_or(Error::Invalid(Invalid::Reference))?;
                runtime.require(*instance, value.registration_instance, work)?;
            }
            w::ControlOperation::Await {
                after: w::AwaitAnchor::Compensation { compensation },
                clock,
                ..
            } => {
                let original = *context
                    .layout
                    .compensations
                    .get(compensation.index as usize)
                    .ok_or(Error::Invalid(Invalid::Reference))?;
                let activation =
                    runtime.anchor_binding(Anchor::CompensationActivation(original), work)?;
                runtime.require(*clock, activation, work)?;
            }
            _ => {}
        }
    }
    work.locus = Some(context.layout.locus(context.syntax.span)?);
    Ok(result)
}

fn lower_one(
    context: &Declaration<'_, '_>,
    roles: &[w::Role],
    controls: &[w::Control],
    original: usize,
    runtime: &mut Runtime<'_, '_>,
    builder: &mut ValueBuilder<'_>,
    work: &mut Work,
) -> Result<w::Compensation, Error> {
    let c::DeclarationKind::Protocol(protocol) = &context.syntax.kind else {
        return Err(Error::Invalid(Invalid::Owner));
    };
    let Some(c::ProtocolRequirement::Compensation(value)) = protocol.requirements.get(original)
    else {
        return Err(Error::Invalid(Invalid::Reference));
    };
    let layout = context.layout;
    work.locus = Some(layout.locus(value.span)?);
    let subject = w::Subject::Compensation {
        compensation: layout.compensation(original)?,
    };
    let owner = controls::role_handle(protocol, context.scope, layout, None, &value.role, work)?;
    work.visit()?;
    let role = roles
        .get(owner.index as usize)
        .ok_or(Error::Invalid(Invalid::Owner))?;
    let operation = operation_export(context, value.operation.name.span, builder, work)?;
    if role.model != controls::operation_context(context, value.operation.name.span, builder, work)?
    {
        return Err(Error::Invalid(Invalid::Type));
    }
    let forward_effect = structural(
        context.scope,
        value.effect.span,
        StructuralKind::Effect,
        layout,
        work,
    )?;
    work.visit()?;
    let effect = controls
        .get(forward_effect.index as usize)
        .ok_or(Error::Invalid(Invalid::Reference))?;
    let w::ControlOperation::Event {
        event: w::Event::Effect {
            instance: forward_instance,
            ..
        },
        ..
    } = &effect.operation
    else {
        return Err(Error::Invalid(Invalid::Control));
    };
    let (forward, forward_type, forward_model) = record(
        context,
        &value.forward,
        BinderKind::ForwardEffect,
        builder,
        work,
    )?;
    work.visit()?;
    if runtime
        .bindings
        .get(*forward_instance as usize)
        .and_then(|binding| binding.value_type.0)
        != Some(forward_type)
    {
        return Err(Error::Invalid(Invalid::Type));
    }
    let registration = Anchor::Registration(original);
    let activation = Anchor::CompensationActivation(original);
    let retry = Anchor::Retry(original);
    let recovery_anchor = Anchor::Recovery(original);
    work.charge(Dimension::Entries, 2)?;
    let mut requires = vec![*forward_instance, role.instance];
    requires.sort_unstable();
    let registration_instance = add(
        runtime,
        value,
        Requirement {
            name: "registration",
            kind: w::BindingKind::CompensationRegistration,
            selected: R::ObservationBinding,
            value_type: Some(forward_type),
            model: Some(forward_model),
            subject: subject.clone(),
            anchor: registration,
            requires,
            span: value.forward.span,
        },
        work,
    )?;
    runtime.bind_anchor(registration, registration_instance)?;
    let (trigger, trigger_type, trigger_model) = record(
        context,
        &value.trigger,
        BinderKind::CompensationTrigger,
        builder,
        work,
    )?;
    work.charge(Dimension::Entries, 1)?;
    let activation_instance = add(
        runtime,
        value,
        Requirement {
            name: "activation",
            kind: w::BindingKind::Observation,
            selected: R::ObservationBinding,
            value_type: Some(trigger_type),
            model: Some(trigger_model),
            subject: subject.clone(),
            anchor: activation,
            requires: vec![registration_instance],
            span: value.trigger.span,
        },
        work,
    )?;
    runtime.bind_anchor(activation, activation_instance)?;
    let profile = selected_profile(context, &value.profile, work)?;
    work.visit()?;
    let temporal = *runtime
        .metadata()
        .registered
        .get(profile as usize)
        .ok_or(Error::Invalid(Invalid::Profile))?;
    work.charge(Dimension::Entries, 1)?;
    work.bytes(value.clock.value.len().saturating_add(6))?;
    let clock_name = format!("clock:{}", value.clock.value);
    let clock = add(
        runtime,
        value,
        Requirement {
            name: &clock_name,
            kind: w::BindingKind::Clock,
            selected: temporal,
            value_type: None,
            model: None,
            subject: subject.clone(),
            anchor: activation,
            requires: vec![activation_instance],
            span: value.clock.span,
        },
        work,
    )?;
    let attempt_native = model_type(context, qualified_span(&value.attempt_type), work)?;
    controls::require_record(attempt_native)?;
    let attempt_type = builder.ty(attempt_native, work)?;
    let (earlier, earlier_type, _) = record(
        context,
        &value.earlier,
        BinderKind::EarlierAttempt,
        builder,
        work,
    )?;
    let (later, later_type, _) = record(
        context,
        &value.later,
        BinderKind::LaterAttempt,
        builder,
        work,
    )?;
    if earlier_type != attempt_type || later_type != attempt_type || earlier == later {
        return Err(Error::Invalid(Invalid::Type));
    }
    work.bytes(value.attempts.value.len())?;
    let maximum = value
        .attempts
        .value
        .parse::<i64>()
        .map_err(|_| Error::Invalid(Invalid::NumericDomain))?;
    if maximum <= 0 {
        return Err(Error::Invalid(Invalid::NumericDomain));
    }
    work.charge(Dimension::Entries, 2)?;
    let mut requires = vec![role.instance, activation_instance];
    requires.sort_unstable();
    let attempt_instance = add(
        runtime,
        value,
        Requirement {
            name: "attempt",
            kind: w::BindingKind::CompensationAttempt,
            selected: R::ObservationBinding,
            value_type: Some(attempt_type),
            model: Some(operation.clone()),
            subject: subject.clone(),
            anchor: retry,
            requires,
            span: value.earlier.span,
        },
        work,
    )?;
    runtime.bind_anchor(retry, attempt_instance)?;
    work.charge(Dimension::Entries, 1)?;
    // The grammar declares no effect payload. This exact operation/attempt
    // identity requires a separate F-bound effect; success is never inferred.
    let effect_instance = add(
        runtime,
        value,
        Requirement {
            name: "effect",
            kind: w::BindingKind::CompensationEffect,
            selected: R::ObservationBinding,
            value_type: None,
            model: Some(operation.clone()),
            subject: subject.clone(),
            anchor: retry,
            requires: vec![attempt_instance],
            span: value.operation.name.span,
        },
        work,
    )?;
    let commit = match &value.commit {
        c::CommitBoundary::Never(_) => None,
        c::CommitBoundary::Node(node) => Some(structural(
            context.scope,
            node.span,
            StructuralKind::Commit,
            layout,
            work,
        )?),
    };
    let (recovery, recovery_type, recovery_model) = record(
        context,
        &value.recovery,
        BinderKind::Recovery,
        builder,
        work,
    )?;
    work.charge(Dimension::Entries, 1)?;
    let snapshot = add(
        runtime,
        value,
        Requirement {
            name: "recovery",
            kind: w::BindingKind::Snapshot,
            selected: R::ObservationBinding,
            value_type: Some(recovery_type),
            model: Some(recovery_model),
            subject: subject.clone(),
            anchor: recovery_anchor,
            requires: vec![activation_instance],
            span: value.recovery.span,
        },
        work,
    )?;
    runtime.bind_anchor(recovery_anchor, snapshot)?;
    work.charge(Dimension::Entries, 1)?;
    let mut recovery_bindings = vec![snapshot];
    for (kind, name) in [
        (w::BindingKind::Progress, "progress"),
        (w::BindingKind::Closure, "closure"),
    ] {
        work.charge(Dimension::Entries, 4)?;
        let mut requires = vec![clock, effect_instance, snapshot];
        requires.sort_unstable();
        recovery_bindings.push(add(
            runtime,
            value,
            Requirement {
                name,
                kind,
                selected: R::Progress,
                value_type: None,
                model: None,
                subject: subject.clone(),
                anchor: recovery_anchor,
                requires,
                span: value.recovery.span,
            },
            work,
        )?);
    }
    Ok(w::Compensation {
        name: text(&value.name.value, work)?,
        forward_effect,
        forward,
        owner,
        operation,
        profile,
        clock,
        registration_anchor: layout.anchor(registration)?,
        registration_instance,
        registration_captures: captures(&value.registration_captures, layout, context.scope, work)?,
        trigger,
        guard: layout.value(value.guard)?,
        activation_anchor: layout.anchor(activation)?,
        activation_captures: captures(&value.activation_captures, layout, context.scope, work)?,
        within: super::super::families::interval(&value.within, work)?,
        maximum_attempts: integer(maximum, work)?,
        attempt_type,
        earlier,
        later,
        retry: layout.value(value.retry)?,
        attempt_instance,
        effect_instance,
        commit: w::Nullable(commit),
        recovery,
        recover: layout.value(value.recover)?,
        recovery_bindings,
        locus: layout.locus(value.span)?,
    })
}

fn record(
    context: &Declaration<'_, '_>,
    parameter: &c::Parameter,
    kind: BinderKind,
    builder: &mut ValueBuilder<'_>,
    work: &mut Work,
) -> Result<(w::Handle, u32, w::ExportRef), Error> {
    work.charge(Dimension::References, context.layout.binders.len())?;
    let binder = parameter_handle(context.layout, context.scope, parameter, kind)?;
    work.visit()?;
    let original = *context
        .layout
        .binders
        .get(binder.index as usize)
        .ok_or(Error::Invalid(Invalid::Binding))?;
    let ty = binder_type(context.typed, original)?;
    controls::require_record(ty)?;
    Ok((binder, builder.ty(ty, work)?, nominal(ty, builder, work)?))
}

fn add(
    runtime: &mut Runtime<'_, '_>,
    value: &c::Compensation,
    requirement: Requirement<'_>,
    work: &mut Work,
) -> Result<u32, Error> {
    work.bytes(
        14_usize
            .saturating_add(value.name.value.len())
            .saturating_add(requirement.name.len()),
    )?;
    let name = format!("compensation:{}:{}", value.name.value, requirement.name);
    runtime.add(
        Requirement {
            name: &name,
            ..requirement
        },
        work,
    )
}
