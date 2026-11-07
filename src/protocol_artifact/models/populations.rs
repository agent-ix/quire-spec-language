// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-042: nominal population requirements and admitted finite-graph fields.

use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};

use crate::checking::DomainType;
use crate::linking::definition_source::RegisteredDefinition;

use super::{local_value, target, w, Dimension, DomainTarget, Error, Invalid, View, Work};

/// A population input: model index, object record name (native) or object
/// artifact id (domain), and anchor index.
type RoleKey<'a> = (u32, &'a str, u32);
type Pair = (Option<u32>, Option<u32>);

/// A domain type a binder or anchored value carries, at a model and anchor.
struct DomainSeed<'p, 'a> {
    model: u32,
    ty: DomainType<'a>,
    anchor: u32,
    locus: &'p w::Locus,
}

pub(super) struct Validation<'p, 'v, 'a> {
    pub package: &'p w::Package,
    pub declaration: &'p w::Declaration,
    pub views: &'v [View<'a>],
}

impl<'p, 'a> Validation<'p, '_, 'a> {
    fn ty(&self, index: u32, work: &mut Work) -> Result<&w::Type, Error> {
        work.visit()?;
        self.package
            .types
            .get(index as usize)
            .ok_or(Error::Invalid(Invalid::Reference))
    }

    fn owner(&self, handle: &w::Handle) -> Result<(), Error> {
        let owner = self
            .package
            .declarations
            .get(handle.declaration as usize)
            .ok_or(Error::Invalid(Invalid::Owner))?;
        if !std::ptr::eq(owner, self.declaration) {
            return Err(Error::Invalid(Invalid::Owner));
        }
        Ok(())
    }

    fn anchor(&self, handle: &w::Handle, work: &mut Work) -> Result<&w::Anchor, Error> {
        work.visit()?;
        self.owner(handle)?;
        self.declaration
            .anchors
            .get(handle.index as usize)
            .ok_or(Error::Invalid(Invalid::Reference))
    }

    pub fn value(&self, value: &w::Value, work: &mut Work) -> Result<(), Error> {
        let w::ValueOperation::Reaches {
            start, target: end, ..
        } = &value.operation
        else {
            return Ok(());
        };
        let start = local_value(self.package, self.declaration, start)?;
        let end = local_value(self.package, self.declaration, end)?;
        if start.value_type != end.value_type
            || start.origin != end.origin
            || !matches!(self.ty(value.value_type, work)?, w::Type::Boolean {})
        {
            return Err(Error::Invalid(Invalid::Type));
        }
        // Reachability is defined over native object roles; a domain-package
        // model exports none, so no wire type reaches here validly.
        Err(Error::Invalid(Invalid::Type))
    }

    pub fn bindings(&self, work: &mut Work) -> Result<(), Error> {
        let mut pairs = BTreeMap::<RoleKey<'a>, Pair>::new();
        for (index, binding) in self.declaration.bindings.iter().enumerate() {
            work.visit()?;
            let population = match (binding.kind, &binding.model.0) {
                (w::BindingKind::Population, Some(export)) => Some(export),
                (w::BindingKind::Population, None) => return Err(Error::Invalid(Invalid::Binding)),
                (w::BindingKind::Closure, Some(export)) => matches!(
                    target(self.views, export, work)?,
                    DomainTarget::Population(_)
                )
                .then_some(export),
                _ => None,
            };
            let Some(export) = population else { continue };
            work.locus = Some(binding.locus.clone());
            let object_name = match target(self.views, export, work)? {
                // The model's exports hold a population export only for a
                // declaration covering the object type, so a declared export
                // already names a covering declaration. FR-153: an object
                // type more than one declaration covers binds none of them.
                DomainTarget::Population(object) => {
                    crate::protocol_artifact::domain::population(&object, work)?;
                    object
                }
                _ => return Err(Error::Invalid(Invalid::Binding)),
            };
            let w::Subject::Declaration { declaration } = binding.subject else {
                return Err(Error::Invalid(Invalid::Binding));
            };
            self.owner(&w::Handle {
                declaration,
                index: 0,
            })?;
            self.owner(&binding.scope)?;
            if binding.scope.index != 0 || binding.relation.0.is_some() {
                return Err(Error::Invalid(Invalid::Binding));
            }
            let anchor = self.anchor(&binding.anchor, work)?;
            let ty = binding
                .value_type
                .0
                .ok_or(Error::Invalid(Invalid::Binding))?;
            let w::Type::Object { export: object } = self.ty(ty, work)? else {
                return Err(Error::Invalid(Invalid::Binding));
            };
            let same_object = match target(self.views, object, work)? {
                DomainTarget::Type(actual) => super::same_domain_type(&actual, &object_name, work)?,
                _ => false,
            };
            if object.model != export.model || !same_object {
                return Err(Error::Invalid(Invalid::Binding));
            }
            let population = binding.kind == w::BindingKind::Population;
            self.contract(
                binding,
                if population {
                    RegisteredDefinition::ObservationBinding
                } else {
                    RegisteredDefinition::Progress
                },
                work,
            )?;
            if population {
                work.visit()?;
                if binding.requires.as_slice() != anchor.binding.0.as_slice() {
                    return Err(Error::Invalid(Invalid::Binding));
                }
            }
            let name = object_name.artifact_id();
            work.bytes(name.len())?;
            let key = (export.model, name, binding.anchor.index);
            let pair = match pairs.entry(key) {
                Entry::Vacant(entry) => {
                    work.charge(Dimension::Entries, 1)?;
                    entry.insert(Pair::default())
                }
                Entry::Occupied(entry) => entry.into_mut(),
            };
            let slot = if population { &mut pair.0 } else { &mut pair.1 };
            if slot
                .replace(
                    u32::try_from(index).map_err(|_| Error::Invalid(Invalid::StructuralInteger))?,
                )
                .is_some()
            {
                return Err(Error::Invalid(Invalid::Duplicate));
            }
        }
        for (population, closure) in pairs.values() {
            work.visit()?;
            let (Some(population), Some(closure)) = (population, closure) else {
                return Err(Error::Invalid(Invalid::Binding));
            };
            work.visit()?;
            let pop = self
                .declaration
                .bindings
                .get(*population as usize)
                .ok_or(Error::Invalid(Invalid::Binding))?;
            work.visit()?;
            let close = self
                .declaration
                .bindings
                .get(*closure as usize)
                .ok_or(Error::Invalid(Invalid::Binding))?;
            if close.requires.as_slice() != [*population]
                || close.value_type != pop.value_type
                || close.model != pop.model
                || close.anchor != pop.anchor
                || close.scope != pop.scope
            {
                return Err(Error::Invalid(Invalid::Binding));
            }
        }
        self.required(&pairs, work)
    }

    fn contract(
        &self,
        binding: &w::BindingRequirement,
        selected: RegisteredDefinition,
        work: &mut Work,
    ) -> Result<(), Error> {
        for definition in &self.package.definitions {
            work.visit()?;
            work.bytes(definition.identity.len())?;
            if definition.identity == selected.identity() && definition.artifact == binding.contract
            {
                let dependency = self
                    .package
                    .dependencies
                    .get(binding.contract as usize)
                    .ok_or(Error::Invalid(Invalid::Reference))?;
                return super::same_ref(&binding.authority, &dependency.artifact, work);
            }
        }
        Err(Error::Invalid(Invalid::Binding))
    }

    fn required(&self, pairs: &BTreeMap<RoleKey<'a>, Pair>, work: &mut Work) -> Result<(), Error> {
        let mut domains = Vec::new();
        for binder in &self.declaration.binders {
            work.visit()?;
            if !matches!(
                binder.kind,
                w::BinderKind::Let | w::BinderKind::Capture | w::BinderKind::Query
            ) {
                work.locus = Some(binder.locus.clone());
                self.seed(
                    binder.value_type,
                    &binder.anchor,
                    &binder.locus,
                    &mut domains,
                    work,
                )?;
            }
        }
        for value in &self.declaration.values {
            work.visit()?;
            if let w::Origin::Anchor { anchor } = &value.origin {
                work.locus = Some(value.locus.clone());
                self.seed(value.value_type, anchor, &value.locus, &mut domains, work)?;
            }
        }
        let mut seen = BTreeSet::new();
        // Every domain object type a seeded domain type reaches needs its
        // population pair at that same model and anchor.
        for DomainSeed {
            model,
            ty,
            anchor,
            locus,
        } in domains
        {
            work.locus = Some(locus.clone());
            for object in crate::protocol_artifact::domain::reached_objects(&ty, work)? {
                let key = (model, object.artifact_id(), anchor);
                work.bytes(key.1.len())?;
                if !pairs.contains_key(&key) {
                    return Err(Error::Invalid(Invalid::Binding));
                }
                if seen.insert(key) {
                    work.charge(Dimension::Entries, 1)?;
                }
            }
        }
        // Each offered pair names an admitted object role; the source-derived
        // record closure must reach that same role at that same anchor.
        for (key, (population, _)) in pairs {
            work.visit()?;
            work.bytes(key.1.len())?;
            if !seen.contains(key) {
                let population = population.ok_or(Error::Invalid(Invalid::Binding))?;
                work.visit()?;
                let binding = self
                    .declaration
                    .bindings
                    .get(population as usize)
                    .ok_or(Error::Invalid(Invalid::Binding))?;
                work.locus = Some(binding.locus.clone());
                return Err(Error::Invalid(Invalid::Binding));
            }
        }
        Ok(())
    }

    fn seed(
        &self,
        mut ty: u32,
        anchor: &w::Handle,
        locus: &'p w::Locus,
        domains: &mut Vec<DomainSeed<'p, 'a>>,
        work: &mut Work,
    ) -> Result<(), Error> {
        self.anchor(anchor, work)?;
        let mut depth = 0;
        loop {
            depth += 1;
            work.charge(Dimension::Depth, depth)?;
            match self.ty(ty, work)? {
                w::Type::Option { value } => ty = *value,
                w::Type::Sequence { element, .. } => ty = *element,
                w::Type::Record { export }
                | w::Type::Object { export }
                | w::Type::Reference { export, .. } => {
                    // A domain type needs the population input of every
                    // domain object type it reaches.
                    let DomainTarget::Type(ty) = target(self.views, export, work)? else {
                        return Err(Error::Invalid(Invalid::Type));
                    };
                    work.charge(Dimension::Entries, 1)?;
                    domains.push(DomainSeed {
                        model: export.model,
                        ty,
                        anchor: anchor.index,
                        locus,
                    });
                    return Ok(());
                }
                w::Type::Boolean {} | w::Type::Scalar { .. } | w::Type::Enum { .. } => {
                    return Ok(())
                }
            }
        }
    }
}
