// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-036/040/042: exact future population inputs over admitted native object
//! roles and domain-package object types (FR-153).
//! Reuses the binding catalog; no observation or membership is supplied here.

use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};

use quire_contract_ir as ir;

use crate::checking::{composed::ObservationOrigin, Catalog, DomainType, NativeType};
use crate::linking::composed::scopes::{Anchor, BinderKind};
use crate::native_model::{NativeModel, ObjectRole};
use crate::protocol_artifact::{domain, work::Work, Dimension, Error, Invalid};
use qsl_foundation::Span;
use qsl_semantics::model::admitted::Declaration as DomainDeclaration;

use super::context::Declaration;

/// The population one input binds.
pub(super) enum Population<'a> {
    /// An admitted native object role, under its universe.
    Native {
        model: &'a NativeModel,
        role: &'a ObjectRole,
    },
    /// A domain object type, under the population declaration covering it.
    Domain {
        object: DomainType<'a>,
        population: DomainDeclaration<'a>,
    },
}

pub(super) struct Need<'a> {
    pub population: Population<'a>,
    pub anchor: Anchor,
    pub span: Span,
}

// ModelBindings refuses same-owner/different-digest inputs (Owner conflict)
// and duplicate exact selections (AmbiguousSelection), so a selected owner
// identifies one admitted model here. Distinct model owners remain supported.
type Key<'a> = (&'a ir::RequirementRef, &'a ir::SymbolName, u32);
// A domain object type, by its package identity and FR-154 key node, at an
// anchor. FR-056 admits one version of a package identity per selection.
type DomainKey<'a> = (&'a str, &'a str, u32);

struct Collector<'s, 'm> {
    context: &'s Declaration<'s, 'm>,
    visited: BTreeSet<Key<'m>>,
    needed: BTreeMap<Key<'m>, Need<'m>>,
    domain: BTreeMap<DomainKey<'m>, Need<'m>>,
}

pub(super) fn collect<'m>(
    context: &Declaration<'_, 'm>,
    work: &mut Work,
) -> Result<Vec<Need<'m>>, Error> {
    let mut collector = Collector {
        context,
        visited: BTreeSet::new(),
        needed: BTreeMap::new(),
        domain: BTreeMap::new(),
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
    work.charge(
        Dimension::Entries,
        collector
            .needed
            .len()
            .saturating_add(collector.domain.len()),
    )?;
    Ok(collector
        .needed
        .into_values()
        .chain(collector.domain.into_values())
        .collect())
}

impl<'s, 'm> Collector<'s, 'm> {
    fn domain(
        &mut self,
        ty: &DomainType<'m>,
        anchor: Anchor,
        span: Span,
        work: &mut Work,
    ) -> Result<(), Error> {
        let anchor_index = self.context.layout.anchor(anchor)?.index;
        for object in domain::reached_objects(ty, work)? {
            let identity = object.package.selection().identity.as_str();
            let node = object.declaration.key.node.as_str();
            work.bytes(identity.len().saturating_add(node.len()))?;
            if let Entry::Vacant(entry) = self.domain.entry((identity, node, anchor_index)) {
                let population = domain::population(&object, work)?;
                work.charge(Dimension::Entries, 1)?;
                entry.insert(Need {
                    population: Population::Domain { object, population },
                    anchor,
                    span,
                });
            }
        }
        Ok(())
    }

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
                // Every domain object type the value reaches needs the
                // population input of the declaration covering it.
                NativeType::Domain(ty) => return self.domain(ty, anchor, span, work),
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
                            population: Population::Native { model, role },
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
