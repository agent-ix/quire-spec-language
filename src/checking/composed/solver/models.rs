// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected model bounds are inspected once, without rebuilding model catalogs.

use super::*;

pub(super) struct ModelBounds {
    invalid: BTreeMap<usize, bool>,
    imports: BTreeMap<UnitId, Vec<usize>>,
}

impl ModelBounds {
    pub(super) fn new(binding: &binding::Report<'_>, work: &mut Work, site: Site) -> Result<Self> {
        let mut result = Self {
            invalid: BTreeMap::new(),
            imports: BTreeMap::new(),
        };
        let Some(models) = binding.models() else {
            return Ok(result);
        };
        // Empty units have no dependent type result and need no invented owner.
        let mut owners = BTreeMap::new();
        for declaration in binding.declarations() {
            work.charge(D::Constraints, 1, site)?;
            let id = declaration.declaration();
            let unit = binding
                .namespace()
                .declaration(id)
                .expect("bound owner")
                .unit();
            if let std::collections::btree_map::Entry::Vacant(entry) = owners.entry(unit) {
                work.charge(D::Records, 1, site)?;
                entry.insert(id);
            }
        }
        for import in models.imports() {
            work.charge(D::Constraints, 1, site)?;
            let Some(&declaration) = owners.get(&import.unit) else {
                continue;
            };
            let Ok(input) = import.selection else {
                continue;
            };
            let Some(model) = models.inputs()[input].native_model() else {
                continue;
            };
            let span = binding
                .namespace()
                .unit(import.unit)
                .expect("import unit")
                .models()[import.import]
                .span;
            let import_site = Site {
                declaration,
                unit: import.unit,
                expression: None,
                span,
            };
            work.charge(D::Records, 1, import_site)?;
            result.imports.entry(import.unit).or_default().push(input);
            if let std::collections::btree_map::Entry::Vacant(entry) = result.invalid.entry(input) {
                work.charge(D::Records, 1, import_site)?;
                let mut invalid = false;
                for declaration in model.environment().types() {
                    work.charge(D::Constraints, 1, import_site)?;
                    match declaration {
                        ir::TypeDeclaration::Record { declaration } => {
                            for field in declaration.fields() {
                                invalid |= type_bounds(field.value_type(), 1, work, import_site)?;
                            }
                        }
                        ir::TypeDeclaration::Enum { .. } => {}
                    }
                }
                for value in model.environment().values() {
                    invalid |= type_bounds(value.value_type(), 1, work, import_site)?;
                }
                entry.insert(invalid);
            }
        }
        Ok(result)
    }

    pub(super) fn imports(&self, unit: UnitId) -> &[usize] {
        self.imports.get(&unit).map_or(&[], Vec::as_slice)
    }

    pub(super) fn invalid(&self, input: usize) -> bool {
        self.invalid[&input]
    }
}

fn type_bounds(ty: &ir::ValueType, depth: usize, work: &mut Work, site: Site) -> Result<bool> {
    work.charge(D::Constraints, 1, site)?;
    work.charge(D::Depth, depth, site)?;
    match ty {
        ir::ValueType::Collection { value } => {
            let nested = type_bounds(value.element(), depth + 1, work, site)?;
            Ok(value.maximum_items() > 10_000 || nested)
        }
        ir::ValueType::Option { value } => type_bounds(value, depth + 1, work, site),
        ir::ValueType::Boolean
        | ir::ValueType::Integer { .. }
        | ir::ValueType::Rational { .. }
        | ir::ValueType::Text
        | ir::ValueType::Enum { .. }
        | ir::ValueType::Record { .. } => Ok(false),
    }
}
