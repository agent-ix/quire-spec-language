// SPDX-License-Identifier: AGPL-3.0-only
//! FR-016-AC-9: transitive population needs, including nested and skipped inputs.

use std::collections::{BTreeMap, BTreeSet};

use quire_contract_ir as ir;

use super::types::Catalog;
use super::{failure, Result, UniverseRequirement};
use crate::native_model::{NativeModel, ObjectRole};
use crate::{Code, Source, Span};

type Identity<'a> = (&'a ir::RequirementRef, &'a ir::SymbolName);

pub(super) struct Populations<'u, 'a> {
    catalogs: &'u [Catalog<'a>],
    source: &'u Source,
    needed: BTreeMap<Identity<'a>, (&'a NativeModel, &'a ObjectRole, [bool; 3])>,
    visited: BTreeSet<(Identity<'a>, usize)>,
}

fn record_leaf(mut ty: &ir::ValueType) -> Option<&ir::SymbolName> {
    loop {
        match ty {
            ir::ValueType::Record { name } => return Some(name),
            ir::ValueType::Option { value } => ty = value,
            ir::ValueType::Collection { value } => ty = value.element(),
            _ => return None,
        }
    }
}

impl<'u, 'a> Populations<'u, 'a> {
    pub fn new(catalogs: &'u [Catalog<'a>], source: &'u Source) -> Self {
        Self {
            catalogs,
            source,
            needed: BTreeMap::new(),
            visited: BTreeSet::new(),
        }
    }

    fn invalid(&self) -> Box<crate::Diagnostic> {
        failure(
            self.source,
            Code::InvalidModelBinding,
            Span { start: 0, end: 0 },
            "native input requirement has no admitted declaration",
        )
    }

    pub fn require_value(
        &mut self,
        model: &'a NativeModel,
        name: &ir::SymbolName,
        observation: ir::StateObservation,
    ) -> Result<()> {
        let catalog = self
            .catalogs
            .iter()
            .find(|catalog| catalog.model.environment().owner() == model.environment().owner())
            .ok_or_else(|| self.invalid())?;
        let value = catalog
            .values
            .get(name)
            .copied()
            .ok_or_else(|| self.invalid())?;
        if let Some(record) = record_leaf(value.value_type()) {
            self.require(model, record, observation)?;
        }
        Ok(())
    }

    pub fn require(
        &mut self,
        model: &'a NativeModel,
        record: &'a ir::SymbolName,
        observation: ir::StateObservation,
    ) -> Result<()> {
        let catalog = self
            .catalogs
            .iter()
            .find(|catalog| catalog.model.environment().owner() == model.environment().owner())
            .ok_or_else(|| self.invalid())?;
        let observation = match observation {
            ir::StateObservation::Current => 0,
            ir::StateObservation::Pre => 1,
            ir::StateObservation::Post => 2,
        };
        let mut pending = vec![record];
        while let Some(name) = pending.pop() {
            let ty = catalog.record_type(name).ok_or_else(|| self.invalid())?;
            let (_, record) = ty.population_record().ok_or_else(|| self.invalid())?;
            let identity = (model.environment().owner(), record);
            if !self.visited.insert((identity, observation)) {
                continue;
            }
            if let Some(role) = catalog.objects.get(record) {
                self.needed
                    .entry(identity)
                    .or_insert((model, role, [false; 3]))
                    .2[observation] = true;
            }
            let declaration = catalog.records.get(record).ok_or_else(|| self.invalid())?;
            // Each admitted record is expanded once per observation. References
            // follow their target object, so native cycles do not expand forever.
            pending.extend(
                declaration
                    .fields()
                    .iter()
                    .filter_map(|field| record_leaf(field.value_type())),
            );
        }
        Ok(())
    }

    pub fn into_requirements(self) -> Vec<UniverseRequirement<'a>> {
        self.needed
            .into_values()
            .map(|(model, object, flags)| UniverseRequirement {
                model,
                object,
                observations: [
                    ir::StateObservation::Current,
                    ir::StateObservation::Pre,
                    ir::StateObservation::Post,
                ]
                .into_iter()
                .zip(flags)
                .filter_map(|(value, needed)| needed.then_some(value))
                .collect(),
            })
            .collect()
    }
}
