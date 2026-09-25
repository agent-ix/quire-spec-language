// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-042: original finite controls and their exact causal-edge expansion.
use super::{
    context::Declaration,
    layout::DeclLayout,
    runtime::{
        binder_type, nominal, operation_export, parameter_handle, selected_profile, structural,
        structural_symbol, Requirement, Runtime,
    },
    types::{index, integer, text, ValueBuilder},
};
use crate::linking::composed::{
    definition_source::RegisteredDefinition as R,
    models::ModelTarget,
    scopes::{self, Anchor, DeclarationScope},
    DependencyKind, DependencySite,
};
use crate::protocol_artifact::{wire as w, work::Work, Dimension, Error, Invalid, Unsupported};
use crate::syntax::composed as c;
use qsl_foundation::Spanned;
pub(super) fn controls(
    context: &Declaration<'_, '_>,
    roles: &[w::Role],
    channels: &[w::Channel],
    runtime: &mut Runtime<'_, '_>,
    builder: &mut ValueBuilder<'_>,
    work: &mut Work,
) -> Result<Vec<w::Control>, Error> {
    let c::DeclarationKind::Protocol(protocol) = &context.syntax.kind else {
        return Err(Error::Invalid(Invalid::Owner));
    };
    let (unit, scope, layout, typed) = (context.unit, context.scope, context.layout, context.typed);
    let mut controls = Vec::new();
    let mut instances = std::collections::BTreeMap::new();
    for &original in &layout.controls {
        work.visit()?;
        let c = unit
            .control(original)
            .ok_or(Error::Invalid(Invalid::Reference))?;
        work.locus = Some(layout.locus(c.span)?);
        let values =
            |ids: &[crate::syntax::ExprId], work: &mut Work| -> Result<Vec<w::Handle>, Error> {
                let mut result = Vec::new();
                for &id in ids {
                    work.visit()?;
                    work.charge(Dimension::Entries, 1)?;
                    result.push(layout.value(id)?);
                }
                Ok(result)
            };
        let operation = match &c.kind {
            c::ControlKind::Sequence(children) => {
                let mut selected = Vec::new();
                for &id in children {
                    work.visit()?;
                    work.charge(Dimension::Entries, 1)?;
                    selected.push(layout.control(id)?);
                }
                w::ControlOperation::Sequence { children: selected }
            }
            c::ControlKind::Parallel { branches, join } => {
                let mut selected = Vec::new();
                let mut joined = Vec::new();
                for branch in branches {
                    work.charge(Dimension::Entries, 1)?;
                    selected.push(w::Branch {
                        label: text(&branch.name.value, work)?,
                        body: layout.control(branch.control)?,
                        locus: layout.locus(branch.span)?,
                    });
                }
                for name in join {
                    work.visit()?;
                    let position = branches
                        .iter()
                        .position(|b| b.name.value == name.value)
                        .ok_or(Error::Invalid(Invalid::Control))?;
                    work.charge(Dimension::Entries, 1)?;
                    joined.push(index(position)?);
                }
                joined.sort_unstable();
                w::ControlOperation::Parallel {
                    branches: selected,
                    join: joined,
                }
            }
            c::ControlKind::Check {
                profile,
                expression,
            } => w::ControlOperation::Check {
                profile: selected_profile(context, profile, work)?,
                value: layout.value(*expression)?,
            },
            c::ControlKind::Choice {
                role: owner,
                visible,
                cases,
            } => {
                let mut selected = Vec::new();
                for case in cases {
                    work.charge(Dimension::Entries, 1)?;
                    selected.push(w::ChoiceCase {
                        label: text(&case.name.value, work)?,
                        guard: layout.value(case.guard)?,
                        body: layout.control(case.control)?,
                        locus: layout.locus(case.span)?,
                    });
                }
                w::ControlOperation::Choice {
                    owner: role_handle(protocol, scope, layout, Some(original), owner, work)?,
                    visible: values(visible, work)?,
                    cases: selected,
                }
            }
            c::ControlKind::Repeat {
                role: owner,
                visible,
                maximum,
                guard,
                body,
                exhausted,
            } => {
                work.bytes(maximum.value.len())?;
                let maximum = maximum
                    .value
                    .parse::<i64>()
                    .map_err(|_| Error::Invalid(Invalid::NumericDomain))?;
                if maximum < 0 {
                    return Err(Error::Invalid(Invalid::NumericDomain));
                }
                w::ControlOperation::Repeat {
                    owner: role_handle(protocol, scope, layout, Some(original), owner, work)?,
                    visible: values(visible, work)?,
                    maximum: integer(maximum, work)?,
                    guard: layout.value(*guard)?,
                    body: layout.control(*body)?,
                    exhausted: layout.control(*exhausted)?,
                }
            }
            c::ControlKind::Await {
                after,
                profile,
                clock,
                within,
                event,
                then,
                timeout,
            } => {
                let profile = selected_profile(context, profile, work)?;
                let symbol = structural_symbol(
                    scope,
                    after.span,
                    scopes::StructuralKind::AwaitAnchor,
                    work,
                )?;
                let after = if symbol.kind == scopes::SymbolKind::Compensation {
                    w::AwaitAnchor::Compensation {
                        compensation: layout.compensation_named(
                            protocol,
                            symbol.name.span,
                            work,
                        )?,
                    }
                } else {
                    w::AwaitAnchor::Event {
                        node: layout
                            .control(symbol.control.ok_or(Error::Invalid(Invalid::Reference))?)?,
                    }
                };
                let selected = *runtime
                    .metadata()
                    .registered
                    .get(profile as usize)
                    .ok_or(Error::Invalid(Invalid::Profile))?;
                let subject = w::Subject::Control {
                    control: layout.control(original)?,
                };
                // Seven prefix/separator bytes plus at most twenty decimal digits
                // for the source index, before the temporary formatted string.
                work.bytes(clock.value.len().saturating_add(27))?;
                let name = format!("await:{}:{}", original.0, clock.value);
                let clock_index = runtime.add(
                    Requirement {
                        name: &name,
                        kind: w::BindingKind::Clock,
                        selected,
                        value_type: None,
                        model: None,
                        subject: subject.clone(),
                        anchor: Anchor::Control(original),
                        requires: Vec::new(),
                        span: clock.span,
                    },
                    work,
                )?;
                for (kind, suffix) in [
                    (w::BindingKind::Progress, "progress"),
                    (w::BindingKind::Closure, "closure"),
                ] {
                    work.charge(Dimension::Entries, 1)?;
                    work.bytes(name.len().saturating_add(suffix.len()).saturating_add(1))?;
                    let name = format!("{name}:{suffix}");
                    runtime.add(
                        Requirement {
                            name: &name,
                            kind,
                            selected: R::Progress,
                            value_type: None,
                            model: None,
                            subject: subject.clone(),
                            anchor: Anchor::Control(original),
                            requires: vec![clock_index],
                            span: clock.span,
                        },
                        work,
                    )?;
                }
                w::ControlOperation::Await {
                    after,
                    profile,
                    clock: clock_index,
                    within: super::families::interval(within, work)?,
                    event: layout.control(*event)?,
                    then_body: layout.control(*then)?,
                    timeout: layout.control(*timeout)?,
                }
            }
            c::ControlKind::Event(event) => {
                let binder = parameter_handle(
                    layout,
                    scope,
                    &event.parameter,
                    scopes::BinderKind::EventRecord,
                )?;
                let original_binder = *layout
                    .binders
                    .get(binder.index as usize)
                    .ok_or(Error::Invalid(Invalid::Binding))?;
                work.visit()?;
                let ty = binder_type(typed, original_binder)?;
                require_record(ty)?;
                let value_type = builder.ty(ty, work)?;
                let model = nominal(ty, builder, work)?;
                let subject = w::Subject::Control {
                    control: layout.control(original)?,
                };
                let constraint = layout.value(event.constraint)?;
                let related = related_occurrences(context, protocol, event, builder, work)?;
                let (kind, requires, lowered) = match &event.kind {
                    c::EventKind::Event {
                        role: owner,
                        compensation,
                    } => {
                        let compensation = compensation
                            .as_ref()
                            .map(|path| {
                                let symbol = structural_symbol(
                                    scope,
                                    path.span,
                                    scopes::StructuralKind::Compensation,
                                    work,
                                )?;
                                if symbol.kind != scopes::SymbolKind::Compensation {
                                    return Err(Error::Invalid(Invalid::Owner));
                                }
                                layout.compensation_named(protocol, symbol.name.span, work)
                            })
                            .transpose()?;
                        let owner =
                            role_handle(protocol, scope, layout, Some(original), owner, work)?;
                        let role_instance = roles
                            .get(owner.index as usize)
                            .ok_or(Error::Invalid(Invalid::Owner))?
                            .instance;
                        work.charge(Dimension::Entries, 1)?;
                        (
                            w::BindingKind::Invocation,
                            vec![role_instance],
                            w::Event::Event {
                                owner,
                                compensation: w::Nullable(compensation),
                                instance: 0,
                            },
                        )
                    }
                    c::EventKind::Attempt {
                        role: owner,
                        operation,
                        contracts,
                    } => {
                        let owner =
                            role_handle(protocol, scope, layout, Some(original), owner, work)?;
                        let selected_role = roles
                            .get(owner.index as usize)
                            .ok_or(Error::Invalid(Invalid::Owner))?;
                        let operation_export =
                            operation_export(context, operation.name.span, builder, work)?;
                        let operation_owner =
                            operation_context(context, operation.name.span, builder, work)?;
                        if selected_role.model != operation_owner {
                            return Err(Error::Invalid(Invalid::Type));
                        }
                        let mut selected = Vec::new();
                        for contract in contracts {
                            let mut target = None;
                            for reference in context.references {
                                work.visit()?;
                                if reference.site == DependencySite::Control(original)
                                    && reference.kind == DependencyKind::OperationContract
                                    && reference.name.span == contract.span
                                {
                                    if target.is_some() {
                                        return Err(Error::Invalid(Invalid::Duplicate));
                                    }
                                    target = reference
                                        .target
                                        .and_then(|target| {
                                            runtime.metadata().declaration_indices.get(&target)
                                        })
                                        .copied();
                                }
                            }
                            work.charge(Dimension::Entries, 1)?;
                            selected.push(target.ok_or(Error::Invalid(Invalid::Dependency))?);
                        }
                        selected.sort_unstable();
                        if selected.windows(2).any(|pair| pair[0] == pair[1]) {
                            return Err(Error::Invalid(Invalid::Duplicate));
                        }
                        work.charge(Dimension::Entries, 1)?;
                        (
                            w::BindingKind::Attempt,
                            vec![selected_role.instance],
                            w::Event::Attempt {
                                owner,
                                operation: operation_export,
                                contracts: selected,
                                instance: 0,
                            },
                        )
                    }
                    c::EventKind::Effect { attempt } => {
                        let target = structural(
                            scope,
                            attempt.span,
                            scopes::StructuralKind::Attempt,
                            layout,
                            work,
                        )?;
                        let attempt_original = *layout
                            .controls
                            .get(target.index as usize)
                            .ok_or(Error::Invalid(Invalid::Reference))?;
                        if !matches!(
                            unit.control(attempt_original).map(|control| &control.kind),
                            Some(c::ControlKind::Event(c::Event {
                                kind: c::EventKind::Attempt { .. },
                                ..
                            }))
                        ) {
                            return Err(Error::Invalid(Invalid::Control));
                        }
                        work.visit()?;
                        let attempt_instance = *instances
                            .get(&target.index)
                            .ok_or(Error::Invalid(Invalid::Binding))?;
                        work.charge(Dimension::Entries, 1)?;
                        (
                            w::BindingKind::Effect,
                            vec![attempt_instance],
                            w::Event::Effect {
                                attempt: target,
                                instance: 0,
                            },
                        )
                    }
                    c::EventKind::Send { channel } => {
                        let channel =
                            channel_handle(protocol, scope, layout, original, channel, work)?;
                        let selected = channels
                            .get(channel.index as usize)
                            .ok_or(Error::Invalid(Invalid::Reference))?;
                        // This direct native adapter has no record-to-payload
                        // projection supplied by a foreign observation producer.
                        if selected.message_type != value_type {
                            return Err(Error::Unsupported(Unsupported::Export));
                        }
                        work.charge(Dimension::Entries, 1)?;
                        (
                            w::BindingKind::Send,
                            vec![selected.send],
                            w::Event::Send { channel },
                        )
                    }
                    c::EventKind::Receive { channel, send } => {
                        let channel =
                            channel_handle(protocol, scope, layout, original, channel, work)?;
                        let selected = channels
                            .get(channel.index as usize)
                            .ok_or(Error::Invalid(Invalid::Reference))?;
                        if selected.message_type != value_type {
                            return Err(Error::Unsupported(Unsupported::Export));
                        }
                        let send = structural(
                            scope,
                            send.span,
                            scopes::StructuralKind::Send,
                            layout,
                            work,
                        )?;
                        let original_send = *layout
                            .controls
                            .get(send.index as usize)
                            .ok_or(Error::Invalid(Invalid::Reference))?;
                        let Some(c::ControlKind::Event(c::Event {
                            kind:
                                c::EventKind::Send {
                                    channel: sent_channel,
                                },
                            ..
                        })) = unit.control(original_send).map(|control| &control.kind)
                        else {
                            return Err(Error::Invalid(Invalid::Control));
                        };
                        if channel_handle(
                            protocol,
                            scope,
                            layout,
                            original_send,
                            sent_channel,
                            work,
                        )? != channel
                        {
                            return Err(Error::Invalid(Invalid::Control));
                        }
                        work.visit()?;
                        let send_instance = *instances
                            .get(&send.index)
                            .ok_or(Error::Invalid(Invalid::Binding))?;
                        work.charge(Dimension::Entries, 2)?;
                        let mut requires = vec![selected.receive, send_instance];
                        requires.sort_unstable();
                        (
                            w::BindingKind::Receive,
                            requires,
                            w::Event::Receive { channel, send },
                        )
                    }
                };
                let name = instance_name(kind.as_str(), original, work)?;
                let instance = runtime.add(
                    Requirement {
                        name: &name,
                        kind,
                        selected: R::ObservationBinding,
                        value_type: Some(value_type),
                        model: Some(model),
                        subject,
                        anchor: Anchor::Control(original),
                        requires,
                        span: event.parameter.span,
                    },
                    work,
                )?;
                work.charge(Dimension::Entries, 1)?;
                instances.insert(layout.control(original)?.index, instance);
                let lowered_event = match lowered {
                    w::Event::Event {
                        owner,
                        compensation,
                        ..
                    } => w::Event::Event {
                        owner,
                        compensation,
                        instance,
                    },
                    w::Event::Attempt {
                        owner,
                        operation,
                        contracts,
                        ..
                    } => w::Event::Attempt {
                        owner,
                        operation,
                        contracts,
                        instance,
                    },
                    w::Event::Effect { attempt, .. } => w::Event::Effect { attempt, instance },
                    w::Event::Send { channel } => w::Event::Send { channel },
                    w::Event::Receive { channel, send } => w::Event::Receive { channel, send },
                };
                w::ControlOperation::Event {
                    event: lowered_event,
                    binder,
                    related,
                    constraint,
                }
            }
            c::ControlKind::Commit {
                role: owner,
                parameter,
                constraint,
            } => {
                let owner = role_handle(protocol, scope, layout, Some(original), owner, work)?;
                let selected_role = roles
                    .get(owner.index as usize)
                    .ok_or(Error::Invalid(Invalid::Owner))?;
                let binder =
                    parameter_handle(layout, scope, parameter, scopes::BinderKind::EventRecord)?;
                let original_binder = *layout
                    .binders
                    .get(binder.index as usize)
                    .ok_or(Error::Invalid(Invalid::Binding))?;
                work.visit()?;
                let ty = binder_type(typed, original_binder)?;
                require_record(ty)?;
                let value_type = builder.ty(ty, work)?;
                let model = nominal(ty, builder, work)?;
                let name = instance_name("commit", original, work)?;
                work.charge(Dimension::Entries, 1)?;
                let instance = runtime.add(
                    Requirement {
                        name: &name,
                        kind: w::BindingKind::Commit,
                        selected: R::ObservationBinding,
                        value_type: Some(value_type),
                        model: Some(model),
                        subject: w::Subject::Control {
                            control: layout.control(original)?,
                        },
                        anchor: Anchor::Control(original),
                        requires: vec![selected_role.instance],
                        span: parameter.span,
                    },
                    work,
                )?;
                w::ControlOperation::Commit {
                    owner,
                    binder,
                    constraint: layout.value(*constraint)?,
                    instance,
                }
            }
        };
        work.charge(Dimension::Entries, 1)?;
        controls.push(w::Control {
            name: text(&c.name.value, work)?,
            original_node: index(original.0)?,
            locus: layout.locus(c.span)?,
            operation,
        });
    }
    Ok(controls)
}

// No admitted correspondence can authorize a relationship-endpoint export
// without the removed Producer 1.2 adapter (#131); the composed model layer
// already refuses every `c::Relationship` occurrence as an unsupported
// correspondence (`ModelWalk::relationship`), so an event that names one
// here refuses the same way, after confirming the name itself resolves.
fn related_occurrences(
    context: &Declaration<'_, '_>,
    protocol: &c::Protocol,
    event: &c::Event,
    _builder: &ValueBuilder<'_>,
    work: &mut Work,
) -> Result<Vec<w::Related>, Error> {
    if event.related.is_empty() {
        return Ok(Vec::new());
    }
    for related in &event.related {
        work.visit()?;
        let symbol = structural_symbol(
            context.scope,
            related.relationship.span,
            scopes::StructuralKind::Relationship,
            work,
        )?;
        protocol
            .relationships
            .iter()
            .find(|relationship| relationship.name.span == symbol.name.span)
            .ok_or(Error::Invalid(Invalid::Reference))?;
    }
    // Unsupported::Export: matches relationship_authority (runtime.rs) and
    // the wire-read equivalent (protocol_artifact::models::validate_related)
    // for the same unreachable-by-construction condition.
    Err(Error::Unsupported(Unsupported::Export))
}

pub(super) fn require_record(ty: &crate::checking::NativeType<'_>) -> Result<(), Error> {
    match ty {
        crate::checking::NativeType::Record { .. } | crate::checking::NativeType::Object { .. } => {
            Ok(())
        }
        crate::checking::NativeType::Domain(_) => Err(Error::Unsupported(Unsupported::Export)),
        crate::checking::NativeType::Boolean
        | crate::checking::NativeType::Scalar { .. }
        | crate::checking::NativeType::Enumeration { .. }
        | crate::checking::NativeType::Reference { .. }
        | crate::checking::NativeType::Option(_)
        | crate::checking::NativeType::Sequence { .. } => Err(Error::Invalid(Invalid::Type)),
    }
}
pub(super) fn role_handle(
    protocol: &c::Protocol,
    scope: &DeclarationScope,
    layout: &DeclLayout,
    original: Option<c::ControlId>,
    name: &Spanned<String>,
    work: &mut Work,
) -> Result<w::Handle, Error> {
    let mut target = None;
    for reference in &scope.references {
        work.visit()?;
        if reference.span == name.span
            && reference.site == original
            && reference.required == scopes::StructuralKind::Role
        {
            if target.is_some() {
                return Err(Error::Invalid(Invalid::Duplicate));
            }
            target = reference.target;
        }
    }
    let target = scope
        .symbols
        .get(target.ok_or(Error::Invalid(Invalid::Reference))?.index())
        .ok_or(Error::Invalid(Invalid::Reference))?;
    if target.kind != scopes::SymbolKind::Role {
        return Err(Error::Invalid(Invalid::Owner));
    }
    for (at, role) in protocol.roles.iter().enumerate() {
        work.visit()?;
        if role.name.span == target.name.span {
            return Ok(layout.handle(index(at)?));
        }
    }
    Err(Error::Invalid(Invalid::Owner))
}
fn instance_name(kind: &str, original: c::ControlId, work: &mut Work) -> Result<String, Error> {
    let digits = original
        .0
        .checked_ilog10()
        .map_or(1, |value| value as usize + 1);
    work.bytes(kind.len().saturating_add(digits).saturating_add(1))?;
    work.charge(Dimension::Entries, 1)?;
    Ok(format!("{kind}:{}", original.0))
}
fn channel_handle(
    protocol: &c::Protocol,
    scope: &DeclarationScope,
    layout: &DeclLayout,
    original: c::ControlId,
    name: &Spanned<String>,
    work: &mut Work,
) -> Result<w::Handle, Error> {
    let mut target = None;
    for reference in &scope.references {
        work.visit()?;
        if reference.span == name.span
            && reference.site == Some(original)
            && reference.required == scopes::StructuralKind::Channel
        {
            if target.is_some() {
                return Err(Error::Invalid(Invalid::Duplicate));
            }
            target = reference.target;
        }
    }
    let target = scope
        .symbols
        .get(target.ok_or(Error::Invalid(Invalid::Reference))?.index())
        .ok_or(Error::Invalid(Invalid::Reference))?;
    if target.kind != scopes::SymbolKind::Channel {
        return Err(Error::Invalid(Invalid::Owner));
    }
    for (at, channel) in protocol.channels.iter().enumerate() {
        work.visit()?;
        if channel.name.span == target.name.span {
            return Ok(layout.handle(index(at)?));
        }
    }
    Err(Error::Invalid(Invalid::Owner))
}
pub(super) fn operation_context(
    context: &Declaration<'_, '_>,
    span: qsl_foundation::Span,
    builder: &ValueBuilder<'_>,
    work: &mut Work,
) -> Result<w::ExportRef, Error> {
    for occurrence in &context.exports.occurrences {
        work.visit()?;
        if occurrence.span.start <= span.start && span.end <= occurrence.span.end {
            match &occurrence.target {
                ModelTarget::Operation(operation) => {
                    return builder.export(
                        operation.model(),
                        w::ExportKind::Object,
                        operation.role().context.as_str(),
                        None,
                        work,
                    );
                }
                // A domain operation has no compiled-protocol attempt or
                // compensation authority yet.
                ModelTarget::Declaration(_) => {
                    return Err(Error::Unsupported(Unsupported::Export));
                }
                ModelTarget::Type(_) => {}
            }
        }
    }
    Err(Error::Invalid(Invalid::Model))
}
pub(super) fn edges(
    controls: &[w::Control],
    layout: &DeclLayout,
    work: &mut Work,
) -> Result<Vec<w::CausalEdge>, Error> {
    let mut edges = Vec::new();
    for (i, control) in controls.iter().enumerate() {
        let owner = layout.handle(index(i)?);
        let enter = w::Endpoint {
            node: owner.clone(),
            port: w::Port::Enter,
        };
        let exit = w::Endpoint {
            node: owner.clone(),
            port: w::Port::Exit,
        };
        let mut add = |kind,
                       from: w::Endpoint,
                       to: w::Endpoint,
                       maximum: Option<&w::Integer>|
         -> Result<(), Error> {
            work.visit()?;
            let maximum = maximum
                .map(|value| -> Result<w::Integer, Error> {
                    let crate::protocol_artifact::NumberWire::Integer { decimal } = &value.0 else {
                        return Err(Error::Invalid(Invalid::WrongNumericKind));
                    };
                    work.bytes(decimal.len())?;
                    work.charge(Dimension::ContentBytes, decimal.len())?;
                    Ok(value.clone())
                })
                .transpose()?;
            work.charge(Dimension::Entries, 1)?;
            edges.push(w::CausalEdge {
                kind,
                owner: owner.clone(),
                from,
                to,
                maximum: w::Nullable(maximum),
            });
            Ok(())
        };
        let start = |h: &w::Handle| w::Endpoint {
            node: h.clone(),
            port: w::Port::Enter,
        };
        let end = |h: &w::Handle| w::Endpoint {
            node: h.clone(),
            port: w::Port::Exit,
        };
        match &control.operation {
            w::ControlOperation::Sequence { children } => {
                let mut from = enter;
                for child in children {
                    add(w::EdgeKind::Sequence, from, start(child), None)?;
                    from = end(child);
                }
                add(w::EdgeKind::Sequence, from, exit, None)?;
            }
            w::ControlOperation::Choice { cases, .. } => {
                for case in cases {
                    add(w::EdgeKind::Branch, enter.clone(), start(&case.body), None)?;
                    add(w::EdgeKind::Join, end(&case.body), exit.clone(), None)?;
                }
            }
            w::ControlOperation::Parallel { branches, .. } => {
                for branch in branches {
                    add(
                        w::EdgeKind::Branch,
                        enter.clone(),
                        start(&branch.body),
                        None,
                    )?;
                    add(w::EdgeKind::Join, end(&branch.body), exit.clone(), None)?;
                }
            }
            w::ControlOperation::Repeat {
                body,
                exhausted,
                maximum,
                ..
            } => {
                add(w::EdgeKind::Branch, enter.clone(), exit.clone(), None)?;
                add(w::EdgeKind::Branch, enter.clone(), start(body), None)?;
                add(w::EdgeKind::Branch, enter.clone(), start(exhausted), None)?;
                add(w::EdgeKind::RepeatProgress, end(body), enter, Some(maximum))?;
                add(w::EdgeKind::Join, end(exhausted), exit, None)?;
            }
            w::ControlOperation::Await {
                after,
                event,
                then_body,
                timeout,
                ..
            } => {
                if let w::AwaitAnchor::Event { node } = after {
                    add(w::EdgeKind::Sequence, end(node), enter.clone(), None)?;
                }
                add(w::EdgeKind::AwaitSuccess, enter.clone(), start(event), None)?;
                add(
                    w::EdgeKind::AwaitSuccess,
                    end(event),
                    start(then_body),
                    None,
                )?;
                add(w::EdgeKind::AwaitTimeout, enter, start(timeout), None)?;
                add(w::EdgeKind::Join, end(then_body), exit.clone(), None)?;
                add(w::EdgeKind::Join, end(timeout), exit, None)?;
            }
            w::ControlOperation::Event { event, .. } => {
                match event {
                    w::Event::Receive { send, .. } => {
                        add(w::EdgeKind::Sequence, end(send), enter.clone(), None)?
                    }
                    w::Event::Effect { attempt, .. } => {
                        add(w::EdgeKind::Sequence, end(attempt), enter.clone(), None)?
                    }
                    w::Event::Send { .. } | w::Event::Attempt { .. } | w::Event::Event { .. } => {}
                }
                add(w::EdgeKind::Sequence, enter, exit, None)?;
            }
            w::ControlOperation::Check { .. } | w::ControlOperation::Commit { .. } => {
                add(w::EdgeKind::Sequence, enter, exit, None)?
            }
        }
    }
    edges.sort_by_key(|e| {
        (
            e.owner.index,
            e.kind.as_str(),
            e.from.node.index,
            e.from.port.as_str(),
            e.to.node.index,
            e.to.port.as_str(),
        )
    });
    Ok(edges)
}
