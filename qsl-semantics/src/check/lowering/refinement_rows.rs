// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-375: lowering a checked refinement's authored rows to the node
//! keys and terms its V2 body names.
//!
//! Each row expression is lowered as a state clause's body is (FR-341): one
//! `parameter` node per binder, `self` first at level 0, then a step's
//! operation parameters, each read through a `reference`. A row's nodes are
//! located in its expression; every declaration a row names that no other
//! declaration located takes one `generated` occurrence at the refinement.

use quire_checked::key::DeclarationKey;
use quire_exact::NodeKey;
use quire_semantic_value::location::{Location, Origin};

use super::state::AnchorInput;
use super::{fault, Binder, Binders, Lowering};
use crate::check::claims::BinderSite;
use crate::check::family::OccurrenceRole;
use crate::check::node_key::LeafTerm;
use crate::check::refinement::{
    CheckedRefinement, CheckedRow, KeyedCatalogClass, KeyedFairnessRow, KeyedObjectRow,
    KeyedPopulationRow, KeyedRefinementRows, KeyedStepRow, KeyedStepTarget, StepClass, StepTarget,
};
use crate::check::refusal::{CheckRefusal, KeyFault};
use quire_semantic_value::fairness::FairnessConstraint;

impl Lowering<'_> {
    /// `refinement`'s authored rows, keyed. Its class catalog, when it has
    /// one, is already keyed: a protocol or memory step's node is its
    /// catalog entry's. `location` is the refinement declaration's own.
    pub(crate) fn refinement_rows(
        &mut self,
        refinement: &CheckedRefinement,
        location: &Location,
    ) -> Result<KeyedRefinementRows, CheckRefusal> {
        let catalog = refinement.class_catalog_keys();
        let mut rows = KeyedRefinementRows::default();
        for row in refinement.populations() {
            rows.populations.push(KeyedPopulationRow {
                abstract_population: self.located(location, |this| {
                    this.population_reference_node(&row.abstract_population, location)
                })?,
                concrete_population: self.located(location, |this| {
                    this.population_reference_node(&row.concrete_population, location)
                })?,
            });
        }
        for row in refinement.objects() {
            let abstract_type = self.located(location, |this| {
                this.model_node(&row.abstract_type, location)
            })?;
            let concrete_type = self.located(location, |this| {
                this.model_node(&row.concrete_type, location)
            })?;
            let mut fields = Vec::with_capacity(row.fields.len());
            for field in &row.fields {
                let (_, name) = self.frame_field(&field.field, location)?;
                fields.push((name, self.row_term(&field.expression)?));
            }
            rows.objects.push(KeyedObjectRow {
                abstract_type,
                concrete_type,
                fields,
            });
        }
        for (source, row) in refinement.steps().iter().enumerate() {
            let source_node = match (&row.class, catalog) {
                (_, Some(catalog)) => catalog_node(catalog.concrete(), row.spelling(), location)?,
                (StepClass::Operation(operation), None) => {
                    Some(self.operation_anchor_of(operation, location)?)
                }
                (_, None) => return Err(rows_fault(location, "a-protocol-row-has-a-catalog")),
            };
            let target = match &row.target {
                StepTarget::Abstract {
                    operation,
                    receiver,
                    arguments,
                    ..
                } => KeyedStepTarget::Operation {
                    operation: self.operation_anchor_of(operation, location)?,
                    receiver: self.row_term(receiver)?,
                    arguments: self.row_arguments(arguments)?,
                },
                StepTarget::Node {
                    protocol,
                    node,
                    arguments,
                } => {
                    let class = format!("{protocol}::{node}");
                    let Some(catalog) = catalog else {
                        return Err(rows_fault(location, "a-protocol-target-has-a-catalog"));
                    };
                    KeyedStepTarget::ProtocolStep {
                        node: catalog_node(catalog.abstract_side(), &class, location)?,
                        class,
                        arguments: self.row_arguments(arguments)?,
                    }
                }
                StepTarget::Stutter => KeyedStepTarget::Stutter,
                StepTarget::Any => KeyedStepTarget::Any,
            };
            rows.steps.push(KeyedStepRow {
                source_class: row.spelling().to_owned(),
                source_node,
                target,
                source,
            });
        }
        for (side, fairness) in [
            (
                &mut rows.assume,
                refinement.concrete_operation_fairness().collect::<Vec<_>>(),
            ),
            (
                &mut rows.ensure,
                refinement.abstract_operation_fairness().collect::<Vec<_>>(),
            ),
        ] {
            for constraint in fairness {
                side.push(self.fairness_row(constraint, location)?);
            }
        }
        Ok(rows)
    }

    /// `key`, built by `node`, with a `generated` occurrence at `location`
    /// when no declaration has located it.
    fn located(
        &mut self,
        location: &Location,
        node: impl FnOnce(&mut Self) -> Result<NodeKey, CheckRefusal>,
    ) -> Result<NodeKey, CheckRefusal> {
        let key = node(self)?;
        if !self.occurrences.has(key) {
            self.record(key, OccurrenceRole::Generated, location.clone());
        }
        Ok(key)
    }

    /// The lowered term of one row expression, over one `parameter` node per
    /// binder of its context.
    fn row_term(&mut self, row: &CheckedRow) -> Result<LeafTerm, CheckRefusal> {
        let location = Location::root(Origin::TypeDeclaration {
            name: row.site.clone(),
        });
        let mut scope = Vec::with_capacity(row.parameters.len());
        for (level, (name, value_type)) in row.parameters.iter().enumerate() {
            let type_key = self.binder_type(value_type, None, &location)?;
            let key = self.parameter(name, level, type_key, &location)?;
            self.record(type_key, OccurrenceRole::Type, location.clone());
            self.record_binder(
                BinderSite {
                    binder: location.clone(),
                    slot: level,
                },
                key,
            );
            scope.push(Binder {
                slot: level,
                parameter: key,
                semantic_type: type_key,
            });
        }
        let mut binders = Binders::new(&row.slot_names, &scope);
        self.expression(row.body(), &mut binders)
    }

    /// A step target's arguments, `_` as `None`.
    fn row_arguments(
        &mut self,
        arguments: &[Option<CheckedRow>],
    ) -> Result<Vec<Option<LeafTerm>>, CheckRefusal> {
        arguments
            .iter()
            .map(|argument| argument.as_ref().map(|row| self.row_term(row)).transpose())
            .collect()
    }

    /// The `operation_anchor` node of the model operation `operation`.
    fn operation_anchor_of(
        &mut self,
        operation: &DeclarationKey,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let (_, model) = self.model_owner(operation, location)?;
        let (declaring, name) = model
            .operation_anchor_declaration(operation)
            .ok_or_else(|| fault(location, KeyFault::UnknownDeclaration(operation.clone())))?;
        self.anchor_by_name(declaring, &name, location)
    }

    /// The `operation_anchor` node of the operation `name` that `declaring`
    /// declares.
    fn anchor_by_name(
        &mut self,
        declaring: quire_exact::EffectiveId,
        name: &str,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let scope = self.scope;
        let operation = scope
            .operations()
            .declared_by(declaring)
            .iter()
            .find(|operation| operation.name() == name)
            .ok_or_else(|| rows_fault(location, "a-row-operation-is-declared"))?;
        let anchor = AnchorInput {
            declaring,
            operation,
            location,
            reference: location,
        };
        let binding = self.operation_anchor(&anchor, None, location)?;
        // An `operation_anchor`'s every occurrence is an `anchor` (FR-322).
        if !self.occurrences.has(binding.anchor) {
            self.record(binding.anchor, OccurrenceRole::Anchor, location.clone());
        }
        Ok(binding.anchor)
    }

    fn fairness_row(
        &mut self,
        constraint: &FairnessConstraint,
        location: &Location,
    ) -> Result<KeyedFairnessRow, CheckRefusal> {
        let operation = constraint.operation();
        Ok(KeyedFairnessRow {
            operation: self.anchor_by_name(
                operation.declaring_type(),
                operation.name(),
                location,
            )?,
            kind: constraint.kind(),
            granularity: constraint.granularity(),
        })
    }
}

/// The node of the catalog entry spelled `class` on one side: `None` for a
/// memory class, which names no declaration.
fn catalog_node(
    side: &[KeyedCatalogClass],
    class: &str,
    location: &Location,
) -> Result<Option<NodeKey>, CheckRefusal> {
    match side.iter().find(|entry| entry.class() == class) {
        Some(KeyedCatalogClass::Declared { node, .. }) => Ok(Some(*node)),
        Some(KeyedCatalogClass::Memory(_)) => Ok(None),
        None => Err(rows_fault(location, "a-row-class-is-catalogued")),
    }
}

fn rows_fault(location: &Location, invariant: &'static str) -> CheckRefusal {
    fault(
        location,
        KeyFault::StageFault(quire_semantic_value::catalog::InternalFault::new(
            "check.refinement-rows",
            invariant,
        )),
    )
}
