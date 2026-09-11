// SPDX-License-Identifier: AGPL-3.0-only
//! FR-042: existing lexical/flow and model authority for received Boolean atoms.

use std::collections::BTreeMap;

use quire_contract_ir as ir;

use crate::checking::NativeType;
use crate::linking::composed::scopes::{
    Anchor, BinderId, BinderKind, ScopeDisposition, StructuralKind, SymbolId, SymbolKind,
    ValueOccurrence,
};
use crate::native_model::NativeModel;
use crate::protocol_artifact::{work::Work, Dimension, Error, Invalid};
use crate::syntax::{composed as c, ExprId};
use crate::{Span, Spanned};

use super::Context;

struct Atom<'m> {
    binder: BinderId,
    anchor: Anchor,
    model: &'m NativeModel,
    record: &'m ir::RecordDeclaration,
    field: &'m ir::RecordFieldDeclaration,
}

pub(super) struct Received<'s, 'm> {
    context: &'s Context<'s, 'm>,
    choice: c::ControlId,
    owner: SymbolId,
    reads: BTreeMap<usize, &'s ValueOccurrence>,
    eligible: BTreeMap<BinderId, bool>,
    atoms: Vec<Atom<'m>>,
}

impl<'s, 'm> Received<'s, 'm> {
    pub fn new(
        context: &'s Context<'s, 'm>,
        choice: c::ControlId,
        work: &mut Work,
    ) -> Result<Self, Error> {
        if context.scope.disposition() != ScopeDisposition::Resolved
            || context.scope.unit != context.typed.unit()
            || context.scope.declaration != context.typed.declaration()
        {
            return Err(Error::Invalid(Invalid::Binding));
        }
        work.visit()?;
        let node = context
            .unit
            .control(choice)
            .ok_or(Error::Invalid(Invalid::Reference))?;
        let c::ControlKind::Choice { role, .. } = &node.kind else {
            return Err(Error::Invalid(Invalid::Control));
        };
        let owner = structural(context, Some(choice), role.span, StructuralKind::Role, work)?;
        let mut reads = BTreeMap::new();
        for read in &context.scope.values {
            work.visit()?;
            if read.unit != context.typed.unit() {
                return Err(Error::Invalid(Invalid::Owner));
            }
            work.charge(Dimension::Entries, 1)?;
            if reads.insert(read.expression.0, read).is_some() {
                return Err(Error::Invalid(Invalid::Binding));
            }
        }
        Ok(Self {
            context,
            choice,
            owner,
            reads,
            eligible: BTreeMap::new(),
            atoms: Vec::new(),
        })
    }

    pub fn atom_count(&self) -> usize {
        self.atoms.len()
    }

    pub fn read(&self, id: ExprId, work: &mut Work) -> Result<Option<BinderId>, Error> {
        work.visit()?;
        let Some(read) = self.reads.get(&id.0) else {
            return Err(Error::Invalid(Invalid::Binding));
        };
        // The existing flow pass exposes only necessarily produced event
        // records here: all-branch joins qualify, branch/await leaks do not.
        if read.evaluation_anchor != Anchor::Control(self.choice) {
            return Ok(None);
        }
        work.visit()?;
        if self.context.typed.node(id).and_then(|node| node.binder) != Some(read.target) {
            return Err(Error::Invalid(Invalid::Binding));
        }
        Ok(Some(read.target))
    }

    pub fn eligible(&mut self, binder: BinderId, work: &mut Work) -> Result<bool, Error> {
        work.visit()?;
        if let Some(value) = self.eligible.get(&binder) {
            return Ok(*value);
        }
        let value = self.receive(binder, work)?;
        work.charge(Dimension::Entries, 1)?;
        self.eligible.insert(binder, value);
        Ok(value)
    }

    fn receive(&self, binder: BinderId, work: &mut Work) -> Result<bool, Error> {
        work.visit()?;
        let original = self
            .context
            .scope
            .binders
            .get(binder.index())
            .ok_or(Error::Invalid(Invalid::Binding))?;
        let Anchor::Control(id) = original.anchor else {
            return Ok(false);
        };
        if original.kind != BinderKind::EventRecord {
            return Ok(false);
        }
        work.visit()?;
        let control = self
            .context
            .unit
            .control(id)
            .ok_or(Error::Invalid(Invalid::Reference))?;
        let c::ControlKind::Event(c::Event {
            kind: c::EventKind::Receive { channel, .. },
            parameter,
            ..
        }) = &control.kind
        else {
            return Ok(false);
        };
        if original.span != parameter.name.span {
            return Err(Error::Invalid(Invalid::Binding));
        }
        let target = structural(
            self.context,
            Some(id),
            channel.span,
            StructuralKind::Channel,
            work,
        )?;
        work.visit()?;
        let symbol = self
            .context
            .scope
            .symbols
            .get(target.index())
            .ok_or(Error::Invalid(Invalid::Reference))?;
        if symbol.kind != SymbolKind::Channel {
            return Err(Error::Invalid(Invalid::Binding));
        }
        // The retained selector resolves a SymbolId; its original token selects
        // the AST channel record. Equal display names never appoint a channel.
        for channel in &self.context.protocol.channels {
            work.visit()?;
            if channel.name.span == symbol.name.span {
                return Ok(structural(
                    self.context,
                    None,
                    channel.to.span,
                    StructuralKind::Role,
                    work,
                )? == self.owner);
            }
        }
        Err(Error::Invalid(Invalid::Reference))
    }

    pub fn field(
        &mut self,
        binder: BinderId,
        name: &Spanned<String>,
        work: &mut Work,
    ) -> Result<Option<usize>, Error> {
        if !self.eligible(binder, work)? {
            return Ok(None);
        }
        let Some((model, record)) = self.record(binder, work)? else {
            return Ok(None);
        };
        for field in record.fields() {
            work.visit()?;
            work.bytes(name.value.len().saturating_add(field.name().as_str().len()))?;
            if field.name().as_str() != name.value {
                continue;
            }
            if !matches!(field.value_type(), ir::ValueType::Boolean) {
                return Ok(None);
            }
            work.visit()?;
            let anchor = self
                .context
                .scope
                .binders
                .get(binder.index())
                .ok_or(Error::Invalid(Invalid::Binding))?
                .anchor;
            for (index, atom) in self.atoms.iter().enumerate() {
                work.visit()?;
                if atom.binder == binder
                    && atom.anchor == anchor
                    && std::ptr::eq(atom.model, model)
                    && std::ptr::eq(atom.record, record)
                    && std::ptr::eq(atom.field, field)
                {
                    return Ok(Some(index));
                }
            }
            work.charge(Dimension::Entries, 1)?;
            let index = self.atoms.len();
            self.atoms.push(Atom {
                binder,
                anchor,
                model,
                record,
                field,
            });
            return Ok(Some(index));
        }
        Err(Error::Invalid(Invalid::Type))
    }

    fn record(
        &self,
        binder: BinderId,
        work: &mut Work,
    ) -> Result<Option<(&'m NativeModel, &'m ir::RecordDeclaration)>, Error> {
        work.visit()?;
        let ty = self
            .context
            .typed
            .binders()
            .get(binder.index())
            .filter(|typed| typed.binder == binder.index())
            .and_then(|typed| typed.ty.as_ref())
            .ok_or(Error::Invalid(Invalid::Type))?;
        match ty {
            NativeType::Record { model, declaration } => Ok(Some((*model, *declaration))),
            NativeType::Object { model, role } => {
                for declaration in model.environment().types() {
                    work.visit()?;
                    if let ir::TypeDeclaration::Record { declaration } = declaration {
                        work.bytes(
                            role.record
                                .as_str()
                                .len()
                                .saturating_add(declaration.name().as_str().len()),
                        )?;
                        if declaration.name() == &role.record {
                            return Ok(Some((*model, declaration)));
                        }
                    }
                }
                Err(Error::Invalid(Invalid::Model))
            }
            NativeType::Boolean
            | NativeType::Scalar { .. }
            | NativeType::Enumeration { .. }
            | NativeType::Reference { .. }
            | NativeType::Option(_)
            | NativeType::Sequence { .. } => Ok(None),
        }
    }
}

fn structural(
    context: &Context<'_, '_>,
    site: Option<c::ControlId>,
    span: Span,
    kind: StructuralKind,
    work: &mut Work,
) -> Result<SymbolId, Error> {
    for reference in &context.scope.references {
        work.visit()?;
        if reference.site == site && reference.span == span && reference.required == kind {
            return reference.target.ok_or(Error::Invalid(Invalid::Reference));
        }
    }
    Err(Error::Invalid(Invalid::Reference))
}
