// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-036/040/042: authored channel payloads and distinct communication premises.
//! Delivery bounds count deliveries per send; they never select a clock.

use crate::checking::NativeType;
use crate::linking::composed::{
    definition_source::RegisteredDefinition as Definition,
    scopes::{Anchor, BinderKind, DeclarationScope, StructuralKind, SymbolKind},
};
use crate::protocol_artifact::{wire as w, work::Work, Dimension, Error, Invalid};
use crate::syntax::composed as c;
use crate::{Span, Spanned};

use super::{
    context::Declaration,
    families,
    layout::DeclLayout,
    runtime::{binder_type, model_type, nominal, parameter_handle, Requirement, Runtime},
    types::{index, text, ValueBuilder},
};

pub(super) fn lower(
    context: &Declaration<'_, '_>,
    roles: &[w::Role],
    runtime: &mut Runtime<'_, '_>,
    builder: &mut ValueBuilder<'_>,
    work: &mut Work,
) -> Result<Vec<w::Channel>, Error> {
    let c::DeclarationKind::Protocol(protocol) = &context.syntax.kind else {
        return Err(Error::Invalid(Invalid::Owner));
    };
    let (typed, scope, layout) = (context.typed, context.scope, context.layout);
    let mut result = Vec::new();
    for (at, channel) in protocol.channels.iter().enumerate() {
        work.visit()?;
        work.locus = Some(layout.locus(channel.span)?);
        let from = endpoint(protocol, scope, layout, &channel.from, work)?;
        let to = endpoint(protocol, scope, layout, &channel.to, work)?;
        let sender = roles
            .get(from.index as usize)
            .ok_or(Error::Invalid(Invalid::Owner))?
            .instance;
        let receiver = roles
            .get(to.index as usize)
            .ok_or(Error::Invalid(Invalid::Owner))?
            .instance;
        let payload_span = Span {
            start: channel.carries.model.span.start,
            end: channel.carries.name.span.end,
        };
        work.locus = Some(layout.locus(payload_span)?);
        let ty = model_type(context, payload_span, work)?;
        let message_type = builder.ty(ty, work)?;
        let model = nominal(ty, builder, work)?;
        work.locus = Some(layout.locus(channel.delivery.span)?);
        let delivery = families::interval(&channel.delivery, work)?;
        let ordering = match &channel.ordering {
            c::Ordering::Unordered(_) => w::Ordering::Unordered {},
            c::Ordering::Fifo { parameter, key, .. } => {
                let binder = parameter_handle(layout, scope, parameter, BinderKind::Fifo)?;
                let original = *layout
                    .binders
                    .get(binder.index as usize)
                    .ok_or(Error::Invalid(Invalid::Binding))?;
                work.locus = Some(layout.locus(parameter.span)?);
                let bound_type = builder.ty(binder_type(typed, original)?, work)?;
                if bound_type != message_type {
                    return Err(Error::Invalid(Invalid::Type));
                }
                let node = typed.node(*key).ok_or(Error::Invalid(Invalid::Type))?;
                work.locus = Some(layout.locus(node.span)?);
                work.visit()?;
                stable_equality(node.ty.as_ref().ok_or(Error::Invalid(Invalid::Type))?)?;
                w::Ordering::Fifo {
                    binder,
                    key: layout.value(*key)?,
                    anchor: layout.anchor(Anchor::Fifo(at))?,
                }
            }
        };
        work.locus = Some(layout.locus(channel.span)?);
        let anchor = match ordering {
            w::Ordering::Fifo { .. } => Anchor::Fifo(at),
            w::Ordering::Unordered {} => Anchor::ProtocolInstant,
        };
        let subject = w::Subject::Channel {
            channel: layout.handle(index(at)?),
        };
        let endpoints = dependencies(&[sender, receiver], work)?;
        let message = runtime.add(
            Requirement {
                name: &name(at, "message", work)?,
                kind: w::BindingKind::Message,
                selected: Definition::ObservationBinding,
                value_type: Some(message_type),
                model: Some(model.clone()),
                subject: subject.clone(),
                anchor,
                requires: endpoints,
                span: channel.span,
            },
            work,
        )?;
        let send = runtime.add(
            Requirement {
                name: &name(at, "send", work)?,
                kind: w::BindingKind::Send,
                selected: Definition::ObservationBinding,
                value_type: Some(message_type),
                model: Some(model.clone()),
                subject: subject.clone(),
                anchor,
                requires: dependencies(&[sender, message], work)?,
                span: channel.span,
            },
            work,
        )?;
        let delivery_instance = runtime.add(
            Requirement {
                name: &name(at, "delivery", work)?,
                kind: w::BindingKind::Delivery,
                selected: Definition::ObservationBinding,
                value_type: Some(message_type),
                model: Some(model.clone()),
                subject: subject.clone(),
                anchor,
                requires: dependencies(&[send], work)?,
                span: channel.span,
            },
            work,
        )?;
        let receive = runtime.add(
            Requirement {
                name: &name(at, "receive", work)?,
                kind: w::BindingKind::Receive,
                selected: Definition::ObservationBinding,
                value_type: Some(message_type),
                model: Some(model),
                subject,
                anchor,
                requires: dependencies(&[receiver, delivery_instance], work)?,
                span: channel.span,
            },
            work,
        )?;
        if matches!(ordering, w::Ordering::Fifo { .. }) {
            runtime.bind_anchor(anchor, message)?;
        }
        work.charge(Dimension::Entries, 1)?;
        result.push(w::Channel {
            name: text(&channel.name.value, work)?,
            from,
            to,
            message_type,
            ordering,
            delivery,
            message,
            send,
            receive,
            delivery_instance,
            locus: layout.locus(channel.span)?,
        });
    }
    Ok(result)
}

fn endpoint(
    protocol: &c::Protocol,
    scope: &DeclarationScope,
    layout: &DeclLayout,
    name: &Spanned<String>,
    work: &mut Work,
) -> Result<w::Handle, Error> {
    work.locus = Some(layout.locus(name.span)?);
    let mut target = None;
    for reference in &scope.references {
        work.visit()?;
        if reference.site.is_none()
            && reference.required == StructuralKind::Role
            && reference.span == name.span
        {
            if target.is_some() {
                return Err(Error::Invalid(Invalid::Duplicate));
            }
            target = reference.target;
        }
    }
    let symbol = scope
        .symbols
        .get(target.ok_or(Error::Invalid(Invalid::Reference))?.index())
        .ok_or(Error::Invalid(Invalid::Reference))?;
    if symbol.kind != SymbolKind::Role {
        return Err(Error::Invalid(Invalid::Owner));
    }
    for (at, role) in protocol.roles.iter().enumerate() {
        work.visit()?;
        if role.name.span == symbol.name.span {
            return Ok(layout.handle(index(at)?));
        }
    }
    Err(Error::Invalid(Invalid::Reference))
}

fn stable_equality(ty: &NativeType<'_>) -> Result<(), Error> {
    match ty {
        NativeType::Boolean
        | NativeType::Scalar { .. }
        | NativeType::Enumeration { .. }
        | NativeType::Object { .. }
        | NativeType::Reference { .. } => Ok(()),
        NativeType::Record { .. } | NativeType::Option(_) | NativeType::Sequence { .. } => {
            Err(Error::Invalid(Invalid::Type))
        }
    }
}

fn dependencies(values: &[u32], work: &mut Work) -> Result<Vec<u32>, Error> {
    work.charge(Dimension::Entries, values.len())?;
    let mut result = values.to_vec();
    result.sort_unstable();
    result.dedup();
    Ok(result)
}

fn name(channel: usize, kind: &str, work: &mut Work) -> Result<String, Error> {
    let digits = channel
        .checked_ilog10()
        .map_or(1, |value| value as usize + 1);
    work.bytes(9usize.saturating_add(digits).saturating_add(kind.len()))?;
    work.charge(Dimension::Entries, 1)?;
    Ok(format!("channel:{channel}:{kind}"))
}
