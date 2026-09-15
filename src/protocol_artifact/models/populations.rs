// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-042: nominal population requirements and admitted finite-graph fields.

use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};

use crate::linking::composed::definition_source::RegisteredDefinition;

use super::{
    ir, local_value, reference_target, reserve_formal, same_model, target, w, Dimension, Error,
    Invalid, NativeType, ScalarSite, Target, View, Work,
};

type RoleKey<'a> = (u32, &'a ir::SymbolName, u32);
type Pair = (Option<u32>, Option<u32>);

struct Pending<'p, 'a> {
    model: u32,
    name: &'a ir::SymbolName,
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
            start,
            target: end,
            edge,
            universe,
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
        let (model, role) = match self.ty(start.value_type, work)? {
            w::Type::Object { export } => {
                let (_, Target::Object(role, _)) = target(self.views, export, work)? else {
                    return Err(Error::Invalid(Invalid::Type));
                };
                (export.model, role)
            }
            w::Type::Reference {
                export,
                object,
                universe,
            } => {
                let (_, role) = reference_target(self.views, export, object, universe, work)?;
                (export.model, role)
            }
            _ => return Err(Error::Invalid(Invalid::Type)),
        };
        let (_, Target::Population(population)) = target(self.views, universe, work)? else {
            return Err(Error::Invalid(Invalid::Type));
        };
        let (view, Target::Field(record, field)) = target(self.views, edge, work)? else {
            return Err(Error::Invalid(Invalid::Type));
        };
        work.bytes(record.name().as_str().len())?;
        if model != universe.model
            || model != edge.model
            || !std::ptr::eq(role, population)
            || record.name() != &role.record
        {
            return Err(Error::Invalid(Invalid::Type));
        }
        reserve_formal(field.value_type(), work)?;
        work.bytes(
            record
                .name()
                .as_str()
                .len()
                .saturating_add(field.name().as_str().len()),
        )?;
        work.charge(Dimension::Entries, 1)?;
        let site = ScalarSite::Field {
            record: record.name().clone(),
            field: field.name().clone(),
        };
        let native = view
            .catalog
            .formal(field.value_type(), &site)
            .ok_or(Error::Invalid(Invalid::Type))?;
        let leaf = match &native {
            NativeType::Option(value) => value.as_ref(),
            NativeType::Sequence { element, .. } => element.as_ref(),
            value => value,
        };
        if !matches!(leaf, NativeType::Reference { model, role: target }
            if same_model(view.catalog.model, model) && std::ptr::eq(role, *target))
        {
            return Err(Error::Invalid(Invalid::Type));
        }
        Ok(())
    }

    pub fn bindings(&self, work: &mut Work) -> Result<(), Error> {
        let mut pairs = BTreeMap::<RoleKey<'a>, Pair>::new();
        for (index, binding) in self.declaration.bindings.iter().enumerate() {
            work.visit()?;
            let population = match (binding.kind, &binding.model.0) {
                (w::BindingKind::Population, Some(export)) => Some(export),
                (w::BindingKind::Population, None) => return Err(Error::Invalid(Invalid::Binding)),
                (w::BindingKind::Closure, Some(export)) => {
                    matches!(target(self.views, export, work)?.1, Target::Population(_))
                        .then_some(export)
                }
                _ => None,
            };
            let Some(export) = population else { continue };
            work.locus = Some(binding.locus.clone());
            let (_, Target::Population(role)) = target(self.views, export, work)? else {
                return Err(Error::Invalid(Invalid::Binding));
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
            let (_, Target::Object(actual, _)) = target(self.views, object, work)? else {
                return Err(Error::Invalid(Invalid::Binding));
            };
            if object.model != export.model || !std::ptr::eq(actual, role) {
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
            work.bytes(role.record.as_str().len())?;
            let key = (export.model, &role.record, binding.anchor.index);
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
        let mut records = Vec::new();
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
                    &mut records,
                    work,
                )?;
            }
        }
        for value in &self.declaration.values {
            work.visit()?;
            if let w::Origin::Anchor { anchor } = &value.origin {
                work.locus = Some(value.locus.clone());
                self.seed(value.value_type, anchor, &value.locus, &mut records, work)?;
            }
        }
        let mut seen = BTreeSet::new();
        while let Some(Pending {
            model,
            name,
            anchor,
            locus,
        }) = records.pop()
        {
            work.locus = Some(locus.clone());
            work.visit()?;
            work.bytes(name.as_str().len())?;
            let view = self
                .views
                .get(model as usize)
                .ok_or(Error::Invalid(Invalid::Reference))?;
            let ty = view
                .catalog
                .record_type(name)
                .ok_or(Error::Invalid(Invalid::Type))?;
            let record = match ty {
                NativeType::Object { role, .. } | NativeType::Reference { role, .. } => {
                    work.bytes(role.record.as_str().len())?;
                    if !pairs.contains_key(&(model, &role.record, anchor)) {
                        return Err(Error::Invalid(Invalid::Binding));
                    }
                    &role.record
                }
                NativeType::Record { declaration, .. } => declaration.name(),
                _ => return Err(Error::Invalid(Invalid::Type)),
            };
            work.bytes(record.as_str().len())?;
            if seen.contains(&(model, record, anchor)) {
                continue;
            }
            work.charge(Dimension::Entries, 1)?;
            seen.insert((model, record, anchor));
            let declaration = view
                .catalog
                .records
                .get(record)
                .ok_or(Error::Invalid(Invalid::Type))?;
            for field in declaration.fields() {
                work.visit()?;
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
                            records.push(Pending {
                                model,
                                name,
                                anchor,
                                locus,
                            });
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
        // Each offered pair names an admitted object role; the source-derived
        // record closure must reach that same role at that same anchor.
        for (key, (population, _)) in pairs {
            work.visit()?;
            work.bytes(key.1.as_str().len())?;
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
        records: &mut Vec<Pending<'p, 'a>>,
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
                    let (_, selected) = target(self.views, export, work)?;
                    let name = match selected {
                        Target::Record(record) => record.name(),
                        Target::Object(role, _) | Target::Reference(role) => &role.record,
                        _ => return Err(Error::Invalid(Invalid::Type)),
                    };
                    work.charge(Dimension::Entries, 1)?;
                    records.push(Pending {
                        model: export.model,
                        name,
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
