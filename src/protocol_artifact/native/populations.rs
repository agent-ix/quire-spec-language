// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-036/040/042: exact future population inputs over admitted native object roles.
//! Reuses the binding catalog; no observation or membership is supplied here.

use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};

use quire_contract_ir as ir;

use crate::checking::{composed::ObservationOrigin, Catalog, NativeType};
use crate::linking::composed::scopes::{Anchor, BinderKind};
use crate::native_model::{NativeModel, ObjectRole};
use crate::protocol_artifact::{work::Work, Dimension, Error, Invalid};
use qsl_foundation::Span;

use super::context::Declaration;

pub(super) struct Need<'a> {
    pub model: &'a NativeModel,
    pub role: &'a ObjectRole,
    pub anchor: Anchor,
    pub span: Span,
}

// ModelBindings refuses same-owner/different-digest inputs (Owner conflict)
// and duplicate exact selections (AmbiguousSelection), so a selected owner
// identifies one admitted model here. Distinct model owners remain supported.
type Key<'a> = (&'a ir::RequirementRef, &'a ir::SymbolName, u32);

struct Collector<'s, 'm> {
    context: &'s Declaration<'s, 'm>,
    visited: BTreeSet<Key<'m>>,
    needed: BTreeMap<Key<'m>, Need<'m>>,
}

pub(super) fn collect<'m>(
    context: &Declaration<'_, 'm>,
    work: &mut Work,
) -> Result<Vec<Need<'m>>, Error> {
    let mut collector = Collector {
        context,
        visited: BTreeSet::new(),
        needed: BTreeMap::new(),
    };
    for &original in &context.layout.binders {
        work.visit()?;
        let binder = context
            .scope
            .binders
            .get(original)
            .ok_or(Error::Invalid(Invalid::Binding))?;
        // Derived values keep their initializer/domain's observation; their
        // declaration token must not mint a new population at a later anchor.
        if matches!(
            binder.kind,
            BinderKind::Let | BinderKind::Capture | BinderKind::Query
        ) {
            continue;
        }
        collector.ty(
            super::runtime::binder_type(context.typed, original)?,
            binder.anchor,
            binder.span,
            work,
        )?;
    }
    for &original in &context.layout.values {
        work.visit()?;
        let node = context
            .typed
            .node(original)
            .ok_or(Error::Invalid(Invalid::Type))?;
        match node.origin.ok_or(Error::Invalid(Invalid::Owner))? {
            ObservationOrigin::Anchored(anchor) => collector.ty(
                node.ty.as_ref().ok_or(Error::Invalid(Invalid::Type))?,
                anchor,
                node.span,
                work,
            )?,
            // All contributing original values are visited separately. A
            // selected origin must not collapse their distinct observations.
            ObservationOrigin::Selected { unit, .. } if unit != context.typed.unit() => {
                return Err(Error::Invalid(Invalid::Owner))
            }
            ObservationOrigin::Independent | ObservationOrigin::Selected { .. } => {}
        }
    }
    work.charge(Dimension::Entries, collector.needed.len())?;
    Ok(collector.needed.into_values().collect())
}

impl<'s, 'm> Collector<'s, 'm> {
    fn catalog(&self, model: &NativeModel, work: &mut Work) -> Result<&'s Catalog<'m>, Error> {
        for (input, selected) in self.context.models.inputs().iter().enumerate() {
            work.visit()?;
            match (*selected).native_model() {
                Some(candidate) if std::ptr::eq(candidate, model) => {
                    return self
                        .context
                        .models
                        .catalog_at(input)
                        .ok_or(Error::Invalid(Invalid::Model));
                }
                Some(_) | None => {}
            }
        }
        Err(Error::Invalid(Invalid::Model))
    }

    fn ty(
        &mut self,
        mut ty: &NativeType<'m>,
        anchor: Anchor,
        span: Span,
        work: &mut Work,
    ) -> Result<(), Error> {
        work.locus = Some(self.context.layout.locus(span)?);
        let mut depth = 0;
        let (model, record) = loop {
            work.visit()?;
            depth += 1;
            work.charge(Dimension::Depth, depth)?;
            match ty {
                NativeType::Option(value) | NativeType::Sequence { element: value, .. } => {
                    ty = value
                }
                NativeType::Object { model, role } | NativeType::Reference { model, role } => {
                    break (*model, &role.record)
                }
                NativeType::Record { model, declaration } => break (*model, declaration.name()),
                // A domain object type's population input has no export yet;
                // a record value type needs none unless it reaches one.
                NativeType::Domain(domain) => {
                    return crate::protocol_artifact::domain::require_no_population(domain, work)
                }
                NativeType::Boolean
                | NativeType::Scalar { .. }
                | NativeType::Enumeration { .. } => return Ok(()),
            }
        };
        let catalog = self.catalog(model, work)?;
        let anchor_index = self.context.layout.anchor(anchor)?.index;
        work.charge(Dimension::Entries, 1)?;
        let mut pending = vec![record];
        while let Some(name) = pending.pop() {
            work.visit()?;
            work.bytes(name.as_str().len())?;
            let native = catalog
                .record_type(name)
                .ok_or(Error::Invalid(Invalid::Model))?;
            let record = match native {
                NativeType::Object { role, .. } | NativeType::Reference { role, .. } => {
                    let key = (model.environment().owner(), &role.record, anchor_index);
                    if let Entry::Vacant(entry) = self.needed.entry(key) {
                        work.charge(Dimension::Entries, 1)?;
                        entry.insert(Need {
                            model,
                            role,
                            anchor,
                            span,
                        });
                    }
                    &role.record
                }
                NativeType::Record { declaration, .. } => declaration.name(),
                NativeType::Boolean
                | NativeType::Scalar { .. }
                | NativeType::Enumeration { .. }
                | NativeType::Option(_)
                | NativeType::Sequence { .. }
                | NativeType::Domain(_) => return Err(Error::Invalid(Invalid::Model)),
            };
            let key = (model.environment().owner(), record, anchor_index);
            if self.visited.contains(&key) {
                continue;
            }
            work.charge(Dimension::Entries, 1)?;
            self.visited.insert(key);
            work.visit()?;
            let declaration = catalog
                .records
                .get(record)
                .ok_or(Error::Invalid(Invalid::Model))?;
            for field in declaration.fields() {
                let mut ty = field.value_type();
                let mut depth = 0;
                loop {
                    work.visit()?;
                    depth += 1;
                    work.charge(Dimension::Depth, depth)?;
                    match ty {
                        ir::ValueType::Option { value } => ty = value,
                        ir::ValueType::Collection { value } => ty = value.element(),
                        ir::ValueType::Record { name } => {
                            work.charge(Dimension::Entries, 1)?;
                            pending.push(name);
                            break;
                        }
                        ir::ValueType::Boolean
                        | ir::ValueType::Integer { .. }
                        | ir::ValueType::Rational { .. }
                        | ir::ValueType::Text
                        | ir::ValueType::Enum { .. } => break,
                    }
                }
            }
        }
        Ok(())
    }
}
