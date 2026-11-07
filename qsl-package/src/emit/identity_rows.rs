// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531: account producer inputs and every owned identity-row output.

use quire_contract_model::{
    NativeJoinBudget, NativeJoinLocus, NativeJoinLocusSource, SelectedExactDeclarationForm,
    SelectedExactDeclarationRow, SelectedExactFieldRow, SelectedExactIdentities,
    SelectedExactIdentity, SelectedSourceRecordRow, SelectedSourceUnionRow,
};
use quire_exact::{EffectiveId, MemberId, MemberOwner};

use crate::native_row as charge;
use crate::{IdentityFileRefusal, NativeRowError, ReadIdentityRows};

use super::Emission;

#[derive(Clone, Debug, Default)]
pub(super) struct IdentityRowSelection {
    declarations: Vec<usize>,
    fields: Vec<usize>,
}

impl Emission {
    /// Derive producer membership again after every final carry/rekey. These
    /// flags avoid allocating structural comparisons in the budgeted adapter;
    /// they do not replace the subsequent native validation of the rows.
    #[ix_trace_rs::string_edge]
    pub(super) fn with_identity_row_selection(mut self) -> Self {
        self.identity_row_selection.declarations = self
            .selected_exact_declaration_rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| {
                let expected = match row.form {
                    SelectedExactDeclarationForm::RecordValueType => ("model", "record_value_type"),
                    SelectedExactDeclarationForm::ObjectType => ("model", "object_type"),
                    SelectedExactDeclarationForm::Union => ("composite_type", "union"),
                    SelectedExactDeclarationForm::DeclaredUnit => ("scalar_type", "unit"),
                };
                self.wire_nodes
                    .iter()
                    .any(|node| {
                        node.node_id == row.structural_node
                            && (node.node_tag.as_ref(), node.semantic_form.as_ref()) == expected
                    })
                    .then_some(index)
            })
            .collect();
        self.identity_row_selection.fields = self
            .selected_exact_field_rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| {
                self.wire_nodes
                    .iter()
                    .any(|node| {
                        node.node_id == row.record_node
                            && node.node_tag.as_ref() == "model"
                            && node.semantic_form.as_ref() == "record_value_type"
                    })
                    .then_some(index)
            })
            .collect();
        self
    }

    pub(super) fn materialize_identity_rows(
        &self,
        package: &quire_contract_model::CheckedPackageV2,
        budget: &mut NativeJoinBudget,
    ) -> Result<ReadIdentityRows, IdentityFileRefusal> {
        let (declarations, fields) = self.identity_input_rows(budget)?;
        let joined =
            SelectedExactIdentities::from_stage_rows(package, declarations, fields, budget)
                .map_err(|error| match budget.first_unpaid() {
                    Some(denied) => {
                        IdentityFileRefusal::Rows(NativeRowError::exhausted(denied, budget))
                    }
                    None => IdentityFileRefusal::Native(Box::new(error)),
                })?;
        self.identity_output_rows(package, &joined, budget)
    }

    pub(super) fn identity_input_rows(
        &self,
        budget: &mut NativeJoinBudget,
    ) -> Result<(Vec<SelectedExactDeclarationRow>, Vec<SelectedExactFieldRow>), IdentityFileRefusal>
    {
        let locus = charge::start(budget, "identity_rows")?;
        // The native reader peeks at its owned input iterator. Build that
        // input only after accounting for each producer-owned copy.
        charge::items(
            budget,
            locus,
            self.identity_row_selection.declarations.len(),
        )?;
        let mut declarations = Vec::with_capacity(self.identity_row_selection.declarations.len());
        for &index in &self.identity_row_selection.declarations {
            let row = &self.selected_exact_declaration_rows[index];
            let locus = NativeJoinLocus {
                source: NativeJoinLocusSource::IdentityDeclaration(index),
                form: "identity_declaration_input",
            };
            charge::work(budget, locus, 1)?;
            declaration(budget, locus, row)?;
            declarations.push(row.clone());
        }
        charge::items(budget, locus, self.identity_row_selection.fields.len())?;
        let mut fields = Vec::with_capacity(self.identity_row_selection.fields.len());
        for &index in &self.identity_row_selection.fields {
            let row = &self.selected_exact_field_rows[index];
            let locus = package_locus("identity_field_input");
            charge::work(budget, locus, 1)?;
            field(budget, locus, row)?;
            fields.push(row.clone());
        }
        Ok((declarations, fields))
    }

    pub(super) fn identity_output_rows(
        &self,
        package: &quire_contract_model::CheckedPackageV2,
        joined: &SelectedExactIdentities,
        budget: &mut NativeJoinBudget,
    ) -> Result<ReadIdentityRows, IdentityFileRefusal> {
        let locus = package_locus("identity_declaration_output");
        let count = joined.declaration_rows().count();
        charge::items(budget, locus, count)?;
        let mut declarations = Vec::with_capacity(count);
        for (index, row) in joined.declaration_rows().enumerate() {
            let locus = NativeJoinLocus {
                source: NativeJoinLocusSource::IdentityDeclaration(index),
                form: "identity_declaration_output",
            };
            charge::work(budget, locus, 1)?;
            declaration(budget, locus, row)?;
            declarations.push(row.clone());
        }
        let locus = package_locus("identity_field_output");
        charge::items(budget, locus, joined.field_rows().len())?;
        let mut fields = Vec::with_capacity(joined.field_rows().len());
        for row in joined.field_rows() {
            charge::work(budget, locus, 1)?;
            field(budget, locus, row)?;
            fields.push(row.clone());
        }
        let locus = package_locus("identity_record_output");
        charge::items(budget, locus, self.selected_source_record_rows.len())?;
        let mut records = Vec::with_capacity(self.selected_source_record_rows.len());
        for row in &self.selected_source_record_rows {
            charge::work(budget, locus, 1)?;
            charge::node(budget, locus, &row.structural_node)?;
            charge::text(budget, locus, &row.source_authority)?;
            charge::text(budget, locus, &row.source_identity)?;
            charge::texts(budget, locus, &row.qualified_declaration)?;
            charge::items(budget, locus, row.members.len())?;
            for member in &row.members {
                charge::work(budget, locus, 1)?;
                charge::text(budget, locus, &member.original_member_identity)?;
                charge::text(budget, locus, &member.admitted_name)?;
                charge::node(budget, locus, &member.type_node)?;
                charge::value_type(budget, locus, &member.value_type)?;
            }
            records.push(SelectedSourceRecordRow {
                structural_node: row.structural_node.clone(),
                source_authority: row.source_authority.clone(),
                source_identity: row.source_identity.clone(),
                qualified_declaration: row.qualified_declaration.clone(),
                record_key: row.record_key.clone(),
                members: row
                    .members
                    .iter()
                    .map(
                        |member| quire_contract_model::SelectedSourceRecordMemberRow {
                            original_member_identity: member.original_member_identity.clone(),
                            admitted_name: member.admitted_name.clone(),
                            presence: member.presence,
                            type_node: member.type_node.clone(),
                            value_type: member.value_type.clone(),
                        },
                    )
                    .collect(),
            });
        }
        let locus = package_locus("identity_union_output");
        charge::items(budget, locus, self.selected_source_union_rows.len())?;
        let mut unions = Vec::with_capacity(self.selected_source_union_rows.len());
        for row in &self.selected_source_union_rows {
            charge::work(budget, locus, 1)?;
            union(budget, locus, row)?;
            unions.push(row.clone());
        }
        charge::text(
            budget,
            package_locus("identity_package_output"),
            package.name(),
        )?;
        Ok(ReadIdentityRows::of_rows(
            package,
            declarations,
            fields,
            records,
            unions,
        ))
    }
}

fn package_locus(form: &'static str) -> NativeJoinLocus {
    NativeJoinLocus {
        source: NativeJoinLocusSource::Package,
        form,
    }
}

fn effective(
    budget: &mut NativeJoinBudget,
    locus: NativeJoinLocus,
    value: &EffectiveId,
) -> Result<(), NativeRowError> {
    charge::items(budget, locus, value.owner_chain().len())?;
    for original in value.owner_chain() {
        charge::text(budget, locus, original.package())?;
        charge::text(budget, locus, original.node())?;
    }
    Ok(())
}

fn member(
    budget: &mut NativeJoinBudget,
    locus: NativeJoinLocus,
    value: &MemberId,
) -> Result<(), NativeRowError> {
    charge::text(budget, locus, value.member())?;
    match value.owner() {
        MemberOwner::Node(_) => Ok(()),
        MemberOwner::Effective(value) => effective(budget, locus, value),
    }
}

fn declaration(
    budget: &mut NativeJoinBudget,
    locus: NativeJoinLocus,
    row: &SelectedExactDeclarationRow,
) -> Result<(), NativeRowError> {
    charge::node(budget, locus, &row.structural_node)?;
    charge::text(budget, locus, &row.selected_document)?;
    charge::text(budget, locus, &row.original_identity)?;
    match &row.exact_identity {
        SelectedExactIdentity::Object(value) => effective(budget, locus, value),
        SelectedExactIdentity::DeclaredUnit(value) => charge::unit(budget, locus, value),
        SelectedExactIdentity::Record(_) | SelectedExactIdentity::Union(_) => Ok(()),
    }
}

fn field(
    budget: &mut NativeJoinBudget,
    locus: NativeJoinLocus,
    row: &SelectedExactFieldRow,
) -> Result<(), NativeRowError> {
    charge::node(budget, locus, &row.record_node)?;
    charge::text(budget, locus, &row.selected_document)?;
    charge::text(budget, locus, &row.record_original_identity)?;
    charge::text(budget, locus, &row.field_original_identity)?;
    member(budget, locus, &row.member_id)
}

fn union(
    budget: &mut NativeJoinBudget,
    locus: NativeJoinLocus,
    row: &SelectedSourceUnionRow,
) -> Result<(), NativeRowError> {
    charge::node(budget, locus, &row.structural_node)?;
    charge::text(budget, locus, &row.source_authority)?;
    charge::text(budget, locus, &row.source_identity)?;
    charge::texts(budget, locus, &row.qualified_declaration)?;
    charge::items(budget, locus, row.members.len())?;
    for member in &row.members {
        charge::work(budget, locus, 1)?;
        charge::text(budget, locus, member.member.identifier())?;
        charge::text(budget, locus, member.member.variant().case())?;
        charge::items(budget, locus, member.payload_nodes.len())?;
        for node in &member.payload_nodes {
            charge::node(budget, locus, node)?;
        }
        charge::items(budget, locus, member.payload.len())?;
        for value in &member.payload {
            charge::value_type(budget, locus, value)?;
        }
    }
    Ok(())
}
