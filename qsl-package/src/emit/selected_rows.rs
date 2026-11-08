// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531: accounted selected-monitor row materialization from one emission.

use quire_contract_model::{
    NativeJoinBudget, NativeJoinLocus, NativeJoinLocusSource, SelectedSourceEnumRow,
};

use crate::native_row as charge;
use crate::NativeRowError;

use super::Emission;

impl Emission {
    /// Materialize complete source enum rows in canonical variant order under
    /// the caller's shared native allowance.
    ///
    /// Every owned row and payload is precharged before copying. These are
    /// original producer claims; IR's subsequent join establishes admission.
    ///
    /// # Errors
    ///
    /// A compact [`NativeRowError`] on an unpaid charge or an incomplete case
    /// population. A preceding denial is preserved without resuming over it.
    pub fn selected_source_enum_rows(
        &self,
        budget: &mut NativeJoinBudget,
    ) -> Result<Vec<SelectedSourceEnumRow>, NativeRowError> {
        let locus = charge::start(budget, "selected_source_enum_rows")?;
        charge::items(budget, locus, self.selected_enum_declaration_rows.len())?;
        let mut expected_members = 0_u128;
        for (index, declaration) in self.selected_enum_declaration_rows.iter().enumerate() {
            let locus = NativeJoinLocus {
                source: NativeJoinLocusSource::AdapterRow(index),
                form: "source_enum_declaration_row",
            };
            charge::work(budget, locus, 1)?;
            expected_members =
                expected_members.saturating_add(declaration.shape.variants().count() as u128);
        }
        if expected_members != self.selected_enum_member_rows.len() as u128 {
            return Err(NativeRowError::incomplete_enum(locus));
        }
        for (index, declaration) in self.selected_enum_declaration_rows.iter().enumerate() {
            let locus = NativeJoinLocus {
                source: NativeJoinLocusSource::AdapterRow(index),
                form: "source_enum_declaration_row",
            };
            charge::enum_declaration(budget, locus, declaration)?;
            let variants = declaration.shape.variants().count();
            charge::items(budget, locus, variants)?;
            // Account for the borrowed population scan and the repeated scan
            // during materialization. VariantId comparisons use Exact's core,
            // avoiding CheckedNodeId comparison's temporary task allocation.
            charge::work(
                budget,
                locus,
                (self.selected_enum_member_rows.len() as u128)
                    .saturating_mul((variants as u128).saturating_add(1))
                    .saturating_mul(2),
            )?;
            let population = self
                .selected_enum_member_rows
                .iter()
                .filter(|member| member.variant.owner() == &declaration.node_key)
                .count();
            if population != variants {
                return Err(NativeRowError::incomplete_enum(locus));
            }
            for variant in declaration.shape.variants() {
                let mut matching = self
                    .selected_enum_member_rows
                    .iter()
                    .enumerate()
                    .filter(|(_, member)| &member.variant == variant);
                let Some((member_index, member)) = matching.next() else {
                    return Err(NativeRowError::incomplete_enum(locus));
                };
                if matching.next().is_some() {
                    return Err(NativeRowError::incomplete_enum(locus));
                }
                charge::enum_member(
                    budget,
                    NativeJoinLocus {
                        source: NativeJoinLocusSource::AdapterRow(member_index),
                        form: "source_enum_member_row",
                    },
                    member,
                )?;
            }
        }

        let mut rows = Vec::with_capacity(self.selected_enum_declaration_rows.len());
        for declaration in &self.selected_enum_declaration_rows {
            let mut members = Vec::with_capacity(declaration.shape.variants().count());
            for variant in declaration.shape.variants() {
                let member = self
                    .selected_enum_member_rows
                    .iter()
                    .find(|member| &member.variant == variant)
                    .expect("preflight found exactly one original member per variant");
                members.push(member.clone());
            }
            rows.push(SelectedSourceEnumRow {
                declaration: declaration.clone(),
                members,
            });
        }
        Ok(rows)
    }

    /// Convert model enum shape rows to IR's native selected-monitor row type,
    /// preserving complete original shapes, cases and identities.
    ///
    /// # Errors
    ///
    /// A compact [`NativeRowError`] before an allocation whose charge cannot
    /// be admitted by the caller's shared budget.
    pub fn selected_ir_model_enum_shape_rows(
        &self,
        budget: &mut NativeJoinBudget,
    ) -> Result<Vec<quire_contract_model::SelectedModelEnumShapeRow>, NativeRowError> {
        let locus = charge::start(budget, "selected_model_enum_shape_rows")?;
        charge::items(budget, locus, self.selected_model_enum_shape_rows.len())?;
        for (index, row) in self.selected_model_enum_shape_rows.iter().enumerate() {
            let locus = NativeJoinLocus {
                source: NativeJoinLocusSource::AdapterRow(index),
                form: "model_enum_shape_row",
            };
            charge::node(budget, locus, &row.structural_node)?;
            charge::text(budget, locus, &row.selected_document)?;
            charge::text(budget, locus, &row.original_identity)?;
            charge::enum_shape(budget, locus, &row.shape)?;
            charge::items(budget, locus, row.cases.len())?;
            for case in &row.cases {
                charge::node(budget, locus, &case.structural_node)?;
                charge::text(budget, locus, &case.case)?;
                charge::text(budget, locus, &case.member_original_identity)?;
                charge::text(budget, locus, case.variant.case())?;
            }
        }
        let mut rows = Vec::with_capacity(self.selected_model_enum_shape_rows.len());
        for row in &self.selected_model_enum_shape_rows {
            rows.push(quire_contract_model::SelectedModelEnumShapeRow {
                structural_node: row.structural_node.clone(),
                node_key: row.node_key.clone(),
                selected_document: row.selected_document.clone(),
                original_identity: row.original_identity.clone(),
                shape: row.shape.clone(),
                cases: row
                    .cases
                    .iter()
                    .map(|case| quire_contract_model::SelectedModelEnumShapeCaseRow {
                        case: case.case.clone(),
                        structural_node: case.structural_node.clone(),
                        member_key: case.member_key.clone(),
                        member_original_identity: case.member_original_identity.clone(),
                        variant: case.variant.clone(),
                    })
                    .collect(),
            });
        }
        Ok(rows)
    }
}

impl super::SelectedSourceCollectionRow {
    /// Convert this already-selected collection row to IR's native row type.
    /// Other retained collections in the emission are neither inspected nor
    /// materialized. A map retains its original key before its value node.
    /// `locus` identifies this row in the caller's retained selection.
    ///
    /// # Errors
    ///
    /// A compact [`NativeRowError`] on an unpaid charge.
    pub fn to_ir(
        &self,
        budget: &mut NativeJoinBudget,
        locus: NativeJoinLocus,
    ) -> Result<quire_contract_model::SelectedSourceCollectionRow, NativeRowError> {
        charge::start(budget, locus.form)?;
        charge::work(budget, locus, 1)?;
        charge::node(budget, locus, &self.structural_node)?;
        charge::node(budget, locus, &self.base_node)?;
        if let Some(key) = &self.key_node {
            charge::node(budget, locus, key)?;
        }
        charge::node(budget, locus, &self.element_node)?;
        charge::value_type(budget, locus, &self.value_type)?;
        Ok(quire_contract_model::SelectedSourceCollectionRow {
            structural_node: self.structural_node.clone(),
            base_node: self.base_node.clone(),
            key_node: self.key_node.clone(),
            element_node: self.element_node.clone(),
            value_type: self.value_type.clone(),
        })
    }
}
